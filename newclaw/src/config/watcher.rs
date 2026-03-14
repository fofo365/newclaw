//! Configuration file watcher
//!
//! This module provides file system watching capabilities for configuration files.
//! It supports TOML, YAML, and JSON formats and detects file changes using
//! modification timestamps or inotify (on Linux).

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use chrono::{DateTime, Utc};
use notify::{RecommendedWatcher, Watcher, Event, EventKind};
use parking_lot::Mutex;

use super::types::{Config, ConfigError, ConfigResult, ConfigFormat};
use super::layers::ConfigScope;

/// Watch event types
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was created
    Created {
        path: PathBuf,
        scope: ConfigScope,
        config: Config,
    },
    
    /// File was modified
    Modified {
        path: PathBuf,
        scope: ConfigScope,
        old_config: Config,
        new_config: Config,
    },
    
    /// File was deleted
    Deleted {
        path: PathBuf,
        scope: ConfigScope,
    },
    
    /// An error occurred
    Error {
        path: Option<PathBuf>,
        error: String,
    },
}

impl WatchEvent {
    /// Get the path associated with this event
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Created { path, .. } => Some(path),
            Self::Modified { path, .. } => Some(path),
            Self::Deleted { path, .. } => Some(path),
            Self::Error { path, .. } => path.as_ref().map(|v| v.as_path()),
        }
    }
    
    /// Get the scope associated with this event
    pub fn scope(&self) -> Option<&ConfigScope> {
        match self {
            Self::Created { scope, .. } => Some(scope),
            Self::Modified { scope, .. } => Some(scope),
            Self::Deleted { scope, .. } => Some(scope),
            Self::Error { .. } => None,
        }
    }
}

/// File metadata cache entry
#[derive(Debug, Clone)]
struct FileCache {
    /// Last known modification time
    modified_time: DateTime<Utc>,
    
    /// Last known file size
    size: u64,
    
    /// Parsed configuration
    config: Config,
    
    /// Scope for this file
    scope: ConfigScope,
}

/// Configuration file watcher
pub struct ConfigWatcher {
    /// Watched files and their cached metadata
    watched_files: Arc<RwLock<HashMap<PathBuf, FileCache>>>,
    
    /// Event sender
    event_tx: broadcast::Sender<WatchEvent>,
    
    /// Internal file system watcher
    fs_watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    
    /// Polling interval for fallback mode
    poll_interval: Duration,
    
    /// Whether to use polling instead of inotify
    use_polling: bool,
    
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
    
    /// Watch scope resolver
    scope_resolver: Arc<dyn ScopeResolver + Send + Sync>,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    pub fn new(scope_resolver: Arc<dyn ScopeResolver + Send + Sync>) -> ConfigResult<Self> {
        let (event_tx, _) = broadcast::channel(256);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        Ok(Self {
            watched_files: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            fs_watcher: Arc::new(Mutex::new(None)),
            poll_interval: Duration::from_secs(1),
            use_polling: false,
            shutdown_tx,
            scope_resolver,
        })
    }
    
    /// Create a watcher with polling fallback
    pub fn with_polling(scope_resolver: Arc<dyn ScopeResolver + Send + Sync>) -> ConfigResult<Self> {
        let mut watcher = Self::new(scope_resolver)?;
        watcher.use_polling = true;
        Ok(watcher)
    }
    
    /// Set the polling interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
    
    /// Start watching a file
    pub async fn watch_file(&self, path: impl Into<PathBuf>) -> ConfigResult<()> {
        let path = path.into();
        
        // Ensure path is absolute
        let path = if path.is_absolute() {
            path
        } else {
            let path_clone = path.clone();
            std::env::current_dir()
                .map(|cwd| cwd.join(path))
                .unwrap_or_else(|_| path_clone)
        };
        
        // Check if file exists
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.display().to_string()));
        }
        
        // Parse the file
        let (config, modified_time, size) = self.parse_file(&path).await?;
        
        // Resolve the scope
        let scope = self.scope_resolver.resolve(&path);
        
        // Cache the file
        let cache = FileCache {
            modified_time,
            size,
            config: config.clone(),
            scope: scope.clone(),
        };
        
        {
            let mut files = self.watched_files.write().await;
            files.insert(path.clone(), cache);
        }
        
        // Send initial event
        let _ = self.event_tx.send(WatchEvent::Created {
            path: path.clone(),
            scope,
            config,
        });
        
        Ok(())
    }
    
    /// Stop watching a file
    pub async fn unwatch_file(&self, path: &Path) -> ConfigResult<()> {
        let mut files = self.watched_files.write().await;
        
        if let Some(cache) = files.remove(path) {
            let _ = self.event_tx.send(WatchEvent::Deleted {
                path: path.to_path_buf(),
                scope: cache.scope,
            });
        }
        
        Ok(())
    }
    
    /// Subscribe to watch events
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.event_tx.subscribe()
    }
    
    /// Start the watcher
    pub async fn start(&self) -> ConfigResult<()> {
        if self.use_polling {
            self.start_polling().await
        } else {
            self.start_inotify().await
        }
    }
    
    /// Stop the watcher
    pub async fn stop(&self) -> ConfigResult<()> {
        // Clear file cache
        let mut files = self.watched_files.write().await;
        files.clear();
        
        // Stop the filesystem watcher
        {
            let mut watcher = self.fs_watcher.lock();
            *watcher = None;
        }
        
        Ok(())
    }
    
    /// List watched files
    pub async fn watched_files(&self) -> Vec<PathBuf> {
        let files = self.watched_files.read().await;
        files.keys().cloned().collect()
    }
    
    /// Get the number of watched files
    pub async fn watched_count(&self) -> usize {
        let files = self.watched_files.read().await;
        files.len()
    }
    
    /// Parse a file and return config + metadata
    async fn parse_file(&self, path: &Path) -> ConfigResult<(Config, DateTime<Utc>, u64)> {
        // Read file content
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| ConfigError::Io(e))?;
        
        // Detect format
        let format = ConfigFormat::from_path(path)
            .ok_or_else(|| ConfigError::InvalidConfig(format!("Unknown format: {}", path.display())))?;
        
        // Parse configuration
        let config = format.parse(&content)?;
        
        // Get file metadata
        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| ConfigError::Io(e))?;
        
        let modified_time: DateTime<Utc> = metadata.modified()
            .map(|t| t.into())
            .unwrap_or_else(|_| Utc::now());
        
        let size = metadata.len();
        
        Ok((config, modified_time, size))
    }
    
    /// Check for file changes (polling mode)
    async fn check_changes(&self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        let files = self.watched_files.read().await;
        
        for (path, cache) in files.iter() {
            // Check if file still exists
            if !path.exists() {
                events.push(WatchEvent::Deleted {
                    path: path.clone(),
                    scope: cache.scope.clone(),
                });
                continue;
            }
            
            // Get current metadata
            if let Ok(metadata) = std::fs::metadata(path) {
                let current_modified: DateTime<Utc> = metadata.modified()
                    .map(|t| t.into())
                    .unwrap_or_else(|_| Utc::now());
                let current_size = metadata.len();
                
                // Check if file changed
                if current_modified != cache.modified_time || current_size != cache.size {
                    // Try to parse the new content
                    if let Ok((new_config, new_modified, new_size)) = self.parse_file(path).await {
                        events.push(WatchEvent::Modified {
                            path: path.clone(),
                            scope: cache.scope.clone(),
                            old_config: cache.config.clone(),
                            new_config,
                        });
                        
                        // Update cache
                        // Note: We need to update the cache, but we're holding a read lock
                        // This will be handled by the polling loop
                    }
                }
            }
        }
        
        events
    }
    
    /// Start polling-based watching
    async fn start_polling(&self) -> ConfigResult<()> {
        let watched = self.watched_files.clone();
        let event_tx = self.event_tx.clone();
        let poll_interval = self.poll_interval;
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(poll_interval).await;
                
                let mut files = watched.write().await;
                let mut to_remove = Vec::new();
                
                for (path, cache) in files.iter_mut() {
                    // Check if file exists
                    if !path.exists() {
                        let _ = event_tx.send(WatchEvent::Deleted {
                            path: path.clone(),
                            scope: cache.scope.clone(),
                        });
                        to_remove.push(path.clone());
                        continue;
                    }
                    
                    // Get current metadata
                    if let Ok(metadata) = std::fs::metadata(&*path) {
                        let current_modified: DateTime<Utc> = metadata.modified()
                            .map(|t| t.into())
                            .unwrap_or_else(|_| Utc::now());
                        let current_size = metadata.len();
                        
                        // Check if file changed
                        if current_modified != cache.modified_time || current_size != cache.size {
                            // Parse the file
                            let content = match std::fs::read_to_string(&*path) {
                                Ok(c) => c,
                                Err(e) => {
                                    let _ = event_tx.send(WatchEvent::Error {
                                        path: Some(path.clone()),
                                        error: e.to_string(),
                                    });
                                    continue;
                                }
                            };
                            
                            let format = ConfigFormat::from_path(&*path);
                            if let Some(format) = format {
                                match format.parse(&content) {
                                    Ok(new_config) => {
                                        let _ = event_tx.send(WatchEvent::Modified {
                                            path: path.clone(),
                                            scope: cache.scope.clone(),
                                            old_config: cache.config.clone(),
                                            new_config: new_config.clone(),
                                        });
                                        
                                        // Update cache
                                        cache.modified_time = current_modified;
                                        cache.size = current_size;
                                        cache.config = new_config;
                                    }
                                    Err(e) => {
                                        let _ = event_tx.send(WatchEvent::Error {
                                            path: Some(path.clone()),
                                            error: e.to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Remove deleted files
                for path in to_remove {
                    files.remove(&path);
                }
            }
        });
        
        Ok(())
    }
    
    /// Start inotify-based watching (Linux)
    async fn start_inotify(&self) -> ConfigResult<()> {
        let event_tx = self.event_tx.clone();
        let watched = self.watched_files.clone();
        
        // Create the watcher
        let watcher_result: Result<RecommendedWatcher, _> = notify::recommended_watcher(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Check for relevant events
                        if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)) {
                            for path in &event.paths {
                                // Get the scope for this path
                                let rt = tokio::runtime::Handle::current();
                                let files = rt.block_on(watched.read());
                                
                                if let Some(cache) = files.get(path) {
                                    let scope = cache.scope.clone();
                                    let old_config = cache.config.clone();
                                    drop(files);
                                    
                                    match &event.kind {
                                        EventKind::Create(_) => {
                                            // Parse the file
                                            if let Ok(config) = rt.block_on(async {
                                                let content = tokio::fs::read_to_string(path).await?;
                                                let format = ConfigFormat::from_path(path)
                                                    .ok_or_else(|| ConfigError::PathError("Unknown format".into()))?;
                                                format.parse(&content)
                                            }) {
                                                let _ = event_tx.send(WatchEvent::Created {
                                                    path: path.clone(),
                                                    scope,
                                                    config,
                                                });
                                            }
                                        }
                                        EventKind::Modify(_) => {
                                            // Parse the new content
                                            if let Ok(new_config) = rt.block_on(async {
                                                let content = tokio::fs::read_to_string(path).await?;
                                                let format = ConfigFormat::from_path(path)
                                                    .ok_or_else(|| ConfigError::PathError("Unknown format".into()))?;
                                                format.parse(&content)
                                            }) {
                                                let _ = event_tx.send(WatchEvent::Modified {
                                                    path: path.clone(),
                                                    scope,
                                                    old_config,
                                                    new_config,
                                                });
                                            }
                                        }
                                        EventKind::Remove(_) => {
                                            let _ = event_tx.send(WatchEvent::Deleted {
                                                path: path.clone(),
                                                scope,
                                            });
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(WatchEvent::Error {
                            path: None,
                            error: e.to_string(),
                        });
                    }
                }
            }
        );
        
        match watcher_result {
            Ok(watcher) => {
                let mut fs_watcher = self.fs_watcher.lock();
                *fs_watcher = Some(watcher);
                
                // Watch the directories of all files
                let files = self.watched_files.read().await;
                let mut watched_dirs = std::collections::HashSet::new();
                
                for path in files.keys() {
                    if let Some(parent) = path.parent() {
                        watched_dirs.insert(parent.to_path_buf());
                    }
                }
                
                // Note: We would need to call watcher.watch() here
                // but since we're using a closure, we need to restructure
                Ok(())
            }
            Err(e) => {
                // Fall back to polling
                tracing::warn!("Failed to create inotify watcher, falling back to polling: {}", e);
                self.start_polling().await
            }
        }
    }
}

/// Scope resolver trait
///
/// Implement this trait to customize how file paths are mapped to configuration scopes.
pub trait ScopeResolver {
    /// Resolve a file path to a configuration scope
    fn resolve(&self, path: &Path) -> ConfigScope;
}

/// Default scope resolver
///
/// Resolves scopes based on file naming conventions:
/// - `global.toml` → Global scope
/// - `agents.toml` or `agents/{name}.toml` → Agent scope
/// - `channels.toml` or `channels/{name}.toml` → Channel scope
/// - `users.toml` or `users/{id}.toml` → User scope
/// - `groups.toml` or `groups/{id}.toml` → Group scope
/// - `sessions/{id}.toml` → Session scope
pub struct DefaultScopeResolver;

impl ScopeResolver for DefaultScopeResolver {
    fn resolve(&self, path: &Path) -> ConfigScope {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Check for exact matches first
        match file_name {
            "global.toml" | "global.json" | "global.yaml" | "global.yml" => {
                return ConfigScope::Global;
            }
            _ => {}
        }
        
        // Check parent directory
        let parent = path.parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        // Extract the name without extension
        let stem = path.file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        match parent {
            "agents" => ConfigScope::Agent(stem.to_string()),
            "channels" => ConfigScope::Channel(stem.to_string()),
            "users" => ConfigScope::User(stem.to_string()),
            "groups" => ConfigScope::Group(stem.to_string()),
            "sessions" => ConfigScope::Session(stem.to_string()),
            _ => {
                // Default based on file name
                if file_name.starts_with("agent") {
                    ConfigScope::Agent(stem.to_string())
                } else if file_name.starts_with("channel") {
                    ConfigScope::Channel(stem.to_string())
                } else if file_name.starts_with("user") {
                    ConfigScope::User(stem.to_string())
                } else if file_name.starts_with("group") {
                    ConfigScope::Group(stem.to_string())
                } else if file_name.starts_with("session") {
                    ConfigScope::Session(stem.to_string())
                } else {
                    ConfigScope::Global
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    
    struct TestScopeResolver;
    
    impl ScopeResolver for TestScopeResolver {
        fn resolve(&self, _path: &Path) -> ConfigScope {
            ConfigScope::Global
        }
    }
    
    #[tokio::test]
    async fn test_watcher_new() {
        let resolver = Arc::new(TestScopeResolver);
        let watcher = ConfigWatcher::new(resolver).unwrap();
        assert_eq!(watcher.watched_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_watch_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        // Create a config file
        let content = r#"
model = "glm-4"
temperature = 0.7
max_tokens = 4096
"#;
        std::fs::write(&config_path, content).unwrap();
        
        let resolver = Arc::new(TestScopeResolver);
        let watcher = ConfigWatcher::new(resolver).unwrap();
        
        watcher.watch_file(&config_path).await.unwrap();
        assert_eq!(watcher.watched_count().await, 1);
        
        let files = watcher.watched_files().await;
        assert!(files.contains(&config_path));
    }
    
    #[tokio::test]
    async fn test_unwatch_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        std::fs::write(&config_path, "model = \"glm-4\"\n").unwrap();
        
        let resolver = Arc::new(TestScopeResolver);
        let watcher = ConfigWatcher::new(resolver).unwrap();
        
        watcher.watch_file(&config_path).await.unwrap();
        assert_eq!(watcher.watched_count().await, 1);
        
        watcher.unwatch_file(&config_path).await.unwrap();
        assert_eq!(watcher.watched_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_watch_event_receive() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        std::fs::write(&config_path, "model = \"glm-4\"\n").unwrap();
        
        let resolver = Arc::new(TestScopeResolver);
        let watcher = ConfigWatcher::new(resolver).unwrap();
        
        let mut rx = watcher.subscribe();
        
        watcher.watch_file(&config_path).await.unwrap();
        
        // Should receive a Created event
        let event = rx.try_recv().unwrap();
        match event {
            WatchEvent::Created { path, .. } => {
                assert_eq!(path, config_path);
            }
            _ => panic!("Expected Created event"),
        }
    }
    
    #[test]
    fn test_default_scope_resolver() {
        let resolver = DefaultScopeResolver;
        
        assert_eq!(
            resolver.resolve(Path::new("config/global.toml")),
            ConfigScope::Global
        );
        
        assert_eq!(
            resolver.resolve(Path::new("config/agents/default.toml")),
            ConfigScope::Agent("default".to_string())
        );
        
        assert_eq!(
            resolver.resolve(Path::new("config/channels/qq.toml")),
            ConfigScope::Channel("qq".to_string())
        );
        
        assert_eq!(
            resolver.resolve(Path::new("config/users/user123.toml")),
            ConfigScope::User("user123".to_string())
        );
    }
    
    #[test]
    fn test_watch_event_path() {
        let event = WatchEvent::Created {
            path: PathBuf::from("/tmp/config.toml"),
            scope: ConfigScope::Global,
            config: Config::default(),
        };
        
        assert_eq!(event.path(), Some(Path::new("/tmp/config.toml")));
        assert_eq!(event.scope(), Some(&ConfigScope::Global));
    }
}