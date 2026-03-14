//! Configuration hot reload manager
//!
//! This module provides hot reload capabilities for configuration, including:
//! - Configuration version management
//! - Configuration diff calculation
//! - Rollback mechanism
//! - Configuration history tracking

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

use super::types::{Config, ConfigError, ConfigResult};
use super::layers::{ConfigScope, ConfigLayer, LayeredConfig};
use super::merge::{ConfigMerger, ConfigDiff};
use super::watcher::WatchEvent;

/// Maximum number of versions to keep in history
const MAX_HISTORY_SIZE: usize = 100;

/// Configuration version entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigVersion {
    /// Unique version identifier
    pub id: Uuid,
    
    /// Scope of this version
    pub scope: ConfigScope,
    
    /// Configuration content
    pub config: Config,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Creator identifier
    pub created_by: String,
    
    /// Commit message
    pub message: String,
    
    /// Source file path (if loaded from file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<PathBuf>,
    
    /// Checksum of the configuration
    pub checksum: String,
}

impl ConfigVersion {
    /// Create a new version
    pub fn new(scope: ConfigScope, config: Config, created_by: impl Into<String>, message: impl Into<String>) -> Self {
        let id = Uuid::new_v4();
        let checksum = Self::calculate_checksum(&config);
        
        Self {
            id,
            scope,
            config,
            created_at: Utc::now(),
            created_by: created_by.into(),
            message: message.into(),
            source_path: None,
            checksum,
        }
    }
    
    /// Calculate a checksum for the configuration
    fn calculate_checksum(config: &Config) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        
        // Hash the config fields
        config.model.hash(&mut hasher);
        config.temperature.to_bits().hash(&mut hasher);
        config.max_tokens.hash(&mut hasher);
        config.system_prompt.hash(&mut hasher);
        
        for tool in &config.tools {
            tool.hash(&mut hasher);
        }
        
        let mut metadata_keys: Vec<_> = config.metadata.keys().collect();
        metadata_keys.sort();
        for key in metadata_keys {
            key.hash(&mut hasher);
            config.metadata.get(key).hash(&mut hasher);
        }
        
        format!("{:x}", hasher.finish())
    }
}

/// Configuration history for a scope
#[derive(Debug, Clone)]
struct ScopeHistory {
    /// Scope identifier
    scope: ConfigScope,
    
    /// Version history (most recent first)
    versions: VecDeque<ConfigVersion>,
    
    /// Maximum history size
    max_size: usize,
}

impl ScopeHistory {
    fn new(scope: ConfigScope) -> Self {
        Self {
            scope,
            versions: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            max_size: MAX_HISTORY_SIZE,
        }
    }
    
    /// Add a version to history
    fn push(&mut self, version: ConfigVersion) {
        // Remove oldest if at capacity
        if self.versions.len() >= self.max_size {
            self.versions.pop_back();
        }
        self.versions.push_front(version);
    }
    
    /// Get the current (latest) version
    fn current(&self) -> Option<&ConfigVersion> {
        self.versions.front()
    }
    
    /// Get a version by ID
    fn get(&self, id: &Uuid) -> Option<&ConfigVersion> {
        self.versions.iter().find(|v| &v.id == id)
    }
    
    /// Get versions within a time range
    fn get_by_time_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&ConfigVersion> {
        self.versions.iter()
            .filter(|v| v.created_at >= start && v.created_at <= end)
            .collect()
    }
    
    /// List all versions
    fn list(&self) -> &VecDeque<ConfigVersion> {
        &self.versions
    }
    
    /// Clear history
    fn clear(&mut self) {
        self.versions.clear();
    }
}

/// Hot reload event
#[derive(Debug, Clone)]
pub enum HotReloadEvent {
    /// Configuration was reloaded
    Reloaded {
        scope: ConfigScope,
        old_version: Option<Uuid>,
        new_version: Uuid,
        diff: ConfigDiff,
    },
    
    /// Configuration was rolled back
    RolledBack {
        scope: ConfigScope,
        from_version: Uuid,
        to_version: Uuid,
    },
    
    /// A version was saved
    VersionSaved {
        scope: ConfigScope,
        version: Uuid,
    },
    
    /// An error occurred
    Error {
        scope: Option<ConfigScope>,
        error: String,
    },
}

/// Configuration hot reload manager
pub struct ConfigHotReloadManager {
    /// Layered configuration
    layered: Arc<RwLock<LayeredConfig>>,
    
    /// History per scope
    history: Arc<RwLock<Vec<ScopeHistory>>>,
    
    /// Event broadcaster
    event_tx: broadcast::Sender<HotReloadEvent>,
    
    /// Watch event receiver
    watch_rx: Option<broadcast::Receiver<WatchEvent>>,
    
    /// Auto-save enabled
    auto_save: bool,
    
    /// Save directory for versions
    save_dir: Option<PathBuf>,
}

impl ConfigHotReloadManager {
    /// Create a new hot reload manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        
        Self {
            layered: Arc::new(RwLock::new(LayeredConfig::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            watch_rx: None,
            auto_save: false,
            save_dir: None,
        }
    }
    
    /// Create with existing layered config
    pub fn with_layered(layered: LayeredConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        
        Self {
            layered: Arc::new(RwLock::new(layered)),
            history: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            watch_rx: None,
            auto_save: false,
            save_dir: None,
        }
    }
    
    /// Connect to a config watcher
    pub fn connect_watcher(&mut self, watcher: &super::watcher::ConfigWatcher) {
        self.watch_rx = Some(watcher.subscribe());
    }
    
    /// Enable auto-save
    pub fn with_auto_save(mut self, dir: impl Into<PathBuf>) -> Self {
        self.auto_save = true;
        self.save_dir = Some(dir.into());
        self
    }
    
    /// Subscribe to hot reload events
    pub fn subscribe(&self) -> broadcast::Receiver<HotReloadEvent> {
        self.event_tx.subscribe()
    }
    
    /// Save a configuration version
    pub async fn save_version(
        &self,
        scope: ConfigScope,
        config: Config,
        created_by: impl Into<String>,
        message: impl Into<String>,
    ) -> ConfigResult<Uuid> {
        let version = ConfigVersion::new(
            scope.clone(),
            config.clone(),
            created_by,
            message,
        );
        
        let version_id = version.id;
        
        // Update layered config
        {
            let mut layered = self.layered.write().await;
            layered.set_layer(ConfigLayer::new(scope.clone(), config));
        }
        
        // Add to history
        {
            let mut history = self.history.write().await;
            
            // Find or create scope history
            let scope_history = history.iter_mut()
                .find(|h| h.scope == scope);
            
            if let Some(sh) = scope_history {
                sh.push(version);
            } else {
                let mut sh = ScopeHistory::new(scope.clone());
                sh.push(version);
                history.push(sh);
            }
        }
        
        // Send event
        let _ = self.event_tx.send(HotReloadEvent::VersionSaved {
            scope,
            version: version_id,
        });
        
        Ok(version_id)
    }
    
    /// Get the current configuration for a scope
    pub async fn get_current(&self, scope: &ConfigScope) -> Option<Config> {
        let layered = self.layered.read().await;
        layered.get_layer(scope).map(|l| l.config.clone())
    }
    
    /// Get a specific version
    pub async fn get_version(&self, scope: &ConfigScope, version_id: &Uuid) -> Option<ConfigVersion> {
        let history = self.history.read().await;
        
        history.iter()
            .find(|h| &h.scope == scope)
            .and_then(|sh| sh.get(version_id).cloned())
    }
    
    /// Get version history for a scope
    pub async fn get_history(&self, scope: &ConfigScope) -> Vec<ConfigVersion> {
        let history = self.history.read().await;
        
        history.iter()
            .find(|h| &h.scope == scope)
            .map(|sh| sh.list().iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get the current version ID
    pub async fn get_current_version_id(&self, scope: &ConfigScope) -> Option<Uuid> {
        let history = self.history.read().await;
        
        history.iter()
            .find(|h| &h.scope == scope)
            .and_then(|sh| sh.current().map(|v| v.id))
    }
    
    /// Rollback to a specific version
    pub async fn rollback(&self, scope: &ConfigScope, version_id: &Uuid) -> ConfigResult<Config> {
        // Get the version
        let version = self.get_version(scope, version_id).await
            .ok_or_else(|| ConfigError::LayerNotFound(format!("Version not found: {}", version_id)))?;
        
        let from_version_id = self.get_current_version_id(scope).await;
        
        // Update the layered config
        {
            let mut layered = self.layered.write().await;
            layered.set_layer(ConfigLayer::new(scope.clone(), version.config.clone()));
        }
        
        // Add the rollback as a new version
        let rollback_version = ConfigVersion::new(
            scope.clone(),
            version.config.clone(),
            "system",
            format!("Rollback to version {}", version_id),
        );
        
        let new_version_id = rollback_version.id;
        
        {
            let mut history = self.history.write().await;
            if let Some(sh) = history.iter_mut().find(|h| &h.scope == scope) {
                sh.push(rollback_version);
            }
        }
        
        // Send event
        let _ = self.event_tx.send(HotReloadEvent::RolledBack {
            scope: scope.clone(),
            from_version: from_version_id.unwrap_or_default(),
            to_version: new_version_id,
        });
        
        Ok(version.config)
    }
    
    /// Reload configuration from a watch event
    pub async fn reload_from_event(&self, event: WatchEvent) -> ConfigResult<()> {
        match event {
            WatchEvent::Created { path, scope, config } => {
                self.save_version(scope, config, "file-watcher", format!("Created from {:?}", path)).await?;
            }
            
            WatchEvent::Modified { path, scope, old_config, new_config } => {
                let old_version_id = self.get_current_version_id(&scope).await;
                
                let diff = ConfigMerger::diff(&old_config, &new_config);
                
                let new_version_id = self.save_version(
                    scope.clone(),
                    new_config,
                    "file-watcher",
                    format!("Modified from {:?}", path),
                ).await?;
                
                let _ = self.event_tx.send(HotReloadEvent::Reloaded {
                    scope,
                    old_version: old_version_id,
                    new_version: new_version_id,
                    diff,
                });
            }
            
            WatchEvent::Deleted { path, scope } => {
                // Remove from layered config
                let mut layered = self.layered.write().await;
                layered.remove_layer(&scope);
                
                let _ = self.event_tx.send(HotReloadEvent::Error {
                    scope: Some(scope),
                    error: format!("Configuration file deleted: {:?}", path),
                });
            }
            
            WatchEvent::Error { path, error } => {
                let _ = self.event_tx.send(HotReloadEvent::Error {
                    scope: None,
                    error: format!("Watch error for {:?}: {}", path, error),
                });
            }
        }
        
        Ok(())
    }
    
    /// Resolve configuration for a context
    pub async fn resolve(&self, scopes: &[ConfigScope]) -> Config {
        let layered = self.layered.read().await;
        layered.resolve(scopes)
    }
    
    /// Resolve all configurations
    pub async fn resolve_all(&self) -> Config {
        let layered = self.layered.read().await;
        layered.resolve_all()
    }
    
    /// Get the layered configuration
    pub async fn get_layered(&self) -> LayeredConfig {
        self.layered.read().await.clone()
    }
    
    /// Clear all history
    pub async fn clear_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }
    
    /// Start processing watch events
    pub async fn start(&mut self) -> ConfigResult<()> {
        if let Some(mut rx) = self.watch_rx.take() {
            let event_tx = self.event_tx.clone();
            let layered = self.layered.clone();
            let history = self.history.clone();
            
            tokio::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    // Process the event
                    match &event {
                        WatchEvent::Created { scope, config, .. } => {
                            let mut layered = layered.write().await;
                            layered.set_layer(ConfigLayer::new(scope.clone(), config.clone()));
                        }
                        
                        WatchEvent::Modified { scope, new_config, .. } => {
                            let mut layered = layered.write().await;
                            layered.set_layer(ConfigLayer::new(scope.clone(), new_config.clone()));
                        }
                        
                        WatchEvent::Deleted { scope, .. } => {
                            let mut layered = layered.write().await;
                            layered.remove_layer(scope);
                        }
                        
                        _ => {}
                    }
                }
            });
        }
        
        Ok(())
    }
    
    /// Stop the manager
    pub async fn stop(&self) -> ConfigResult<()> {
        // Clear watch receiver
        Ok(())
    }
    
    /// Get statistics
    pub async fn stats(&self) -> HotReloadStats {
        let history = self.history.read().await;
        
        let total_versions: usize = history.iter().map(|h| h.versions.len()).sum();
        let scope_count = history.len();
        
        HotReloadStats {
            scope_count,
            total_versions,
            max_history_size: MAX_HISTORY_SIZE,
        }
    }
}

impl Default for ConfigHotReloadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Hot reload statistics
#[derive(Debug, Clone)]
pub struct HotReloadStats {
    /// Number of scopes with history
    pub scope_count: usize,
    
    /// Total versions across all scopes
    pub total_versions: usize,
    
    /// Maximum history size per scope
    pub max_history_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_save_version() {
        let manager = ConfigHotReloadManager::new();
        
        let config = Config::new("glm-5");
        let version_id = manager.save_version(
            ConfigScope::Global,
            config,
            "test",
            "Initial config",
        ).await.unwrap();
        
        assert!(!version_id.is_nil());
        
        let stats = manager.stats().await;
        assert_eq!(stats.scope_count, 1);
        assert_eq!(stats.total_versions, 1);
    }
    
    #[tokio::test]
    async fn test_get_current() {
        let manager = ConfigHotReloadManager::new();
        
        let config = Config::new("glm-5").with_temperature(0.5);
        manager.save_version(
            ConfigScope::Global,
            config.clone(),
            "test",
            "Initial",
        ).await.unwrap();
        
        let current = manager.get_current(&ConfigScope::Global).await.unwrap();
        assert_eq!(current.model, "glm-5");
        assert_eq!(current.temperature, 0.5);
    }
    
    #[tokio::test]
    async fn test_get_history() {
        let manager = ConfigHotReloadManager::new();
        
        manager.save_version(
            ConfigScope::Global,
            Config::new("glm-4"),
            "test",
            "Version 1",
        ).await.unwrap();
        
        manager.save_version(
            ConfigScope::Global,
            Config::new("glm-5"),
            "test",
            "Version 2",
        ).await.unwrap();
        
        let history = manager.get_history(&ConfigScope::Global).await;
        assert_eq!(history.len(), 2);
        
        // Most recent should be first
        assert_eq!(history[0].config.model, "glm-5");
        assert_eq!(history[1].config.model, "glm-4");
    }
    
    #[tokio::test]
    async fn test_rollback() {
        let manager = ConfigHotReloadManager::new();
        
        let v1 = manager.save_version(
            ConfigScope::Global,
            Config::new("glm-4"),
            "test",
            "Version 1",
        ).await.unwrap();
        
        manager.save_version(
            ConfigScope::Global,
            Config::new("glm-5"),
            "test",
            "Version 2",
        ).await.unwrap();
        
        // Rollback to v1
        let config = manager.rollback(&ConfigScope::Global, &v1).await.unwrap();
        assert_eq!(config.model, "glm-4");
        
        // Current should now be glm-4
        let current = manager.get_current(&ConfigScope::Global).await.unwrap();
        assert_eq!(current.model, "glm-4");
    }
    
    #[tokio::test]
    async fn test_resolve() {
        let manager = ConfigHotReloadManager::new();
        
        manager.save_version(
            ConfigScope::Global,
            Config::new("glm-4").with_temperature(0.7),
            "test",
            "Global",
        ).await.unwrap();
        
        manager.save_version(
            ConfigScope::Agent("default".into()),
            Config::new("glm-5"),
            "test",
            "Agent",
        ).await.unwrap();
        
        let resolved = manager.resolve(&[
            ConfigScope::Global,
            ConfigScope::Agent("default".into()),
        ]).await;
        
        assert_eq!(resolved.model, "glm-5"); // Agent overrides
        assert_eq!(resolved.temperature, 0.7); // From global
    }
    
    #[tokio::test]
    async fn test_receive_events() {
        let manager = ConfigHotReloadManager::new();
        let mut rx = manager.subscribe();
        
        manager.save_version(
            ConfigScope::Global,
            Config::new("glm-4"),
            "test",
            "Initial",
        ).await.unwrap();
        
        let event = rx.try_recv().unwrap();
        match event {
            HotReloadEvent::VersionSaved { scope, .. } => {
                assert_eq!(scope, ConfigScope::Global);
            }
            _ => panic!("Expected VersionSaved event"),
        }
    }
    
    #[test]
    fn test_config_version_checksum() {
        let config1 = Config::new("glm-4");
        let config2 = Config::new("glm-4");
        let config3 = Config::new("glm-5");
        
        let v1 = ConfigVersion::new(ConfigScope::Global, config1, "test", "");
        let v2 = ConfigVersion::new(ConfigScope::Global, config2, "test", "");
        let v3 = ConfigVersion::new(ConfigScope::Global, config3, "test", "");
        
        // Same config should have same checksum
        assert_eq!(v1.checksum, v2.checksum);
        
        // Different config should have different checksum
        assert_ne!(v1.checksum, v3.checksum);
    }
    
    #[tokio::test]
    async fn test_history_size_limit() {
        let manager = ConfigHotReloadManager::new();
        
        // Add more than max versions
        for i in 0..MAX_HISTORY_SIZE + 10 {
            manager.save_version(
                ConfigScope::Global,
                Config::new(format!("model-{}", i)),
                "test",
                format!("Version {}", i),
            ).await.unwrap();
        }
        
        let history = manager.get_history(&ConfigScope::Global).await;
        
        // Should be capped at MAX_HISTORY_SIZE
        assert!(history.len() <= MAX_HISTORY_SIZE);
    }
}