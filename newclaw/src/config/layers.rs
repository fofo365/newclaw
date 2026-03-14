//! Configuration layers and scope definitions
//!
//! This module implements the 6-layer configuration architecture:
//! Global → Agent → Channel → User → Group → Session

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use super::types::{Config, ConfigError, ConfigResult, ConfigFormat};

/// Configuration scope identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigScope {
    /// Global configuration (lowest priority)
    Global,
    
    /// Agent-specific configuration
    Agent(String),
    
    /// Channel-specific configuration (e.g., "qq", "feishu")
    Channel(String),
    
    /// User-specific configuration
    User(String),
    
    /// Group-specific configuration
    Group(String),
    
    /// Session-specific configuration (highest priority)
    Session(String),
}

impl ConfigScope {
    /// Get the priority level of this scope (higher = more priority)
    pub fn priority(&self) -> u8 {
        match self {
            Self::Global => 0,
            Self::Agent(_) => 1,
            Self::Channel(_) => 2,
            Self::User(_) => 3,
            Self::Group(_) => 4,
            Self::Session(_) => 5,
        }
    }
    
    /// Get the name of this scope
    pub fn name(&self) -> String {
        match self {
            Self::Global => "global".to_string(),
            Self::Agent(id) => format!("agent:{}", id),
            Self::Channel(id) => format!("channel:{}", id),
            Self::User(id) => format!("user:{}", id),
            Self::Group(id) => format!("group:{}", id),
            Self::Session(id) => format!("session:{}", id),
        }
    }
    
    /// Check if this is a global scope
    pub fn is_global(&self) -> bool {
        matches!(self, Self::Global)
    }
    
    /// Get the layer name
    pub fn layer_name(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Agent(_) => "agent",
            Self::Channel(_) => "channel",
            Self::User(_) => "user",
            Self::Group(_) => "group",
            Self::Session(_) => "session",
        }
    }
}

impl std::fmt::Display for ConfigScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A single configuration layer
#[derive(Debug, Clone)]
pub struct ConfigLayer {
    /// Scope of this layer
    pub scope: ConfigScope,
    
    /// Configuration data
    pub config: Config,
    
    /// Source file path (if loaded from file)
    pub source: Option<PathBuf>,
    
    /// Last modification time
    pub modified_at: DateTime<Utc>,
    
    /// Whether this layer is active
    pub active: bool,
}

impl ConfigLayer {
    /// Create a new configuration layer
    pub fn new(scope: ConfigScope, config: Config) -> Self {
        Self {
            scope,
            config,
            source: None,
            modified_at: Utc::now(),
            active: true,
        }
    }
    
    /// Create a layer from a file
    pub fn from_file(scope: ConfigScope, path: PathBuf) -> ConfigResult<Self> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ConfigError::FileNotFound(format!("{}: {}", path.display(), e)))?;
        
        let format = ConfigFormat::from_path(&path)
            .ok_or_else(|| ConfigError::InvalidConfig(format!("Unknown format: {}", path.display())))?;
        
        let config = format.parse(&content)?;
        
        let metadata = std::fs::metadata(&path)
            .map_err(|e| ConfigError::Io(e))?;
        
        let modified_at: DateTime<Utc> = metadata.modified()
            .map(|t| t.into())
            .unwrap_or_else(|_| Utc::now());
        
        Ok(Self {
            scope,
            config,
            source: Some(path),
            modified_at,
            active: true,
        })
    }
    
    /// Save the layer to a file
    pub fn save_to_file(&self, path: &PathBuf) -> ConfigResult<()> {
        let format = ConfigFormat::from_path(path)
            .ok_or_else(|| ConfigError::InvalidConfig(format!("Unknown format: {}", path.display())))?;
        
        let content = format.serialize(&self.config)?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Update the configuration
    pub fn update(&mut self, config: Config) {
        self.config = config;
        self.modified_at = Utc::now();
    }
}

/// Layered configuration manager
#[derive(Debug, Clone)]
pub struct LayeredConfig {
    /// Configuration layers indexed by scope
    layers: HashMap<String, ConfigLayer>,
    
    /// Ordered list of scopes for resolution
    scope_order: Vec<ConfigScope>,
}

impl Default for LayeredConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl LayeredConfig {
    /// Create a new layered configuration
    pub fn new() -> Self {
        Self {
            layers: HashMap::new(),
            scope_order: Vec::new(),
        }
    }
    
    /// Add or update a layer
    pub fn set_layer(&mut self, layer: ConfigLayer) {
        let key = layer.scope.name();
        
        // Update scope order if this is a new scope
        if !self.layers.contains_key(&key) {
            self.scope_order.push(layer.scope.clone());
            self.sort_scopes();
        }
        
        self.layers.insert(key, layer);
    }
    
    /// Remove a layer by scope
    pub fn remove_layer(&mut self, scope: &ConfigScope) -> Option<ConfigLayer> {
        let key = scope.name();
        self.scope_order.retain(|s| s.name() != key);
        self.layers.remove(&key)
    }
    
    /// Get a layer by scope
    pub fn get_layer(&self, scope: &ConfigScope) -> Option<&ConfigLayer> {
        self.layers.get(&scope.name())
    }
    
    /// Get a mutable layer by scope
    pub fn get_layer_mut(&mut self, scope: &ConfigScope) -> Option<&mut ConfigLayer> {
        self.layers.get_mut(&scope.name())
    }
    
    /// List all scopes
    pub fn list_scopes(&self) -> Vec<&ConfigScope> {
        self.scope_order.iter().collect()
    }
    
    /// Check if a scope exists
    pub fn has_scope(&self, scope: &ConfigScope) -> bool {
        self.layers.contains_key(&scope.name())
    }
    
    /// Resolve the effective configuration for a given context
    pub fn resolve(&self, scopes: &[ConfigScope]) -> Config {
        let mut result = Config::default();
        
        // Sort scopes by priority
        let mut sorted_scopes: Vec<_> = scopes.to_vec();
        sorted_scopes.sort_by_key(|s| s.priority());
        
        // Merge configurations in priority order
        for scope in sorted_scopes {
            if let Some(layer) = self.get_layer(&scope) {
                if layer.active {
                    result.merge(&layer.config);
                }
            }
        }
        
        result
    }
    
    /// Resolve configuration with all layers
    pub fn resolve_all(&self) -> Config {
        self.resolve(&self.scope_order.clone())
    }
    
    /// Sort scopes by priority
    fn sort_scopes(&mut self) {
        self.scope_order.sort_by_key(|s| s.priority());
    }
    
    /// Get the number of layers
    pub fn len(&self) -> usize {
        self.layers.len()
    }
    
    /// Check if there are no layers
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }
    
    /// Clear all layers
    pub fn clear(&mut self) {
        self.layers.clear();
        self.scope_order.clear();
    }
}

/// Configuration context for resolving layered configs
#[derive(Debug, Clone, Default)]
pub struct ConfigContext {
    /// Agent ID
    pub agent_id: Option<String>,
    
    /// Channel ID
    pub channel_id: Option<String>,
    
    /// User ID
    pub user_id: Option<String>,
    
    /// Group ID
    pub group_id: Option<String>,
    
    /// Session ID
    pub session_id: Option<String>,
}

impl ConfigContext {
    /// Create a new configuration context
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set agent ID
    pub fn with_agent(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }
    
    /// Set channel ID
    pub fn with_channel(mut self, id: impl Into<String>) -> Self {
        self.channel_id = Some(id.into());
        self
    }
    
    /// Set user ID
    pub fn with_user(mut self, id: impl Into<String>) -> Self {
        self.user_id = Some(id.into());
        self
    }
    
    /// Set group ID
    pub fn with_group(mut self, id: impl Into<String>) -> Self {
        self.group_id = Some(id.into());
        self
    }
    
    /// Set session ID
    pub fn with_session(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }
    
    /// Convert to a list of scopes
    pub fn to_scopes(&self) -> Vec<ConfigScope> {
        let mut scopes = vec![ConfigScope::Global];
        
        if let Some(ref id) = self.agent_id {
            scopes.push(ConfigScope::Agent(id.clone()));
        }
        if let Some(ref id) = self.channel_id {
            scopes.push(ConfigScope::Channel(id.clone()));
        }
        if let Some(ref id) = self.user_id {
            scopes.push(ConfigScope::User(id.clone()));
        }
        if let Some(ref id) = self.group_id {
            scopes.push(ConfigScope::Group(id.clone()));
        }
        if let Some(ref id) = self.session_id {
            scopes.push(ConfigScope::Session(id.clone()));
        }
        
        scopes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_priority() {
        assert!(ConfigScope::Global.priority() < ConfigScope::Agent("test".into()).priority());
        assert!(ConfigScope::Agent("test".into()).priority() < ConfigScope::Channel("test".into()).priority());
        assert!(ConfigScope::Channel("test".into()).priority() < ConfigScope::User("test".into()).priority());
        assert!(ConfigScope::User("test".into()).priority() < ConfigScope::Group("test".into()).priority());
        assert!(ConfigScope::Group("test".into()).priority() < ConfigScope::Session("test".into()).priority());
    }
    
    #[test]
    fn test_scope_name() {
        assert_eq!(ConfigScope::Global.name(), "global");
        assert_eq!(ConfigScope::Agent("default".into()).name(), "agent:default");
        assert_eq!(ConfigScope::User("123".into()).name(), "user:123");
    }
    
    #[test]
    fn test_config_layer() {
        let config = Config::new("glm-5");
        let layer = ConfigLayer::new(ConfigScope::Agent("default".into()), config);
        
        assert_eq!(layer.scope, ConfigScope::Agent("default".into()));
        assert!(layer.source.is_none());
        assert!(layer.active);
    }
    
    #[test]
    fn test_layered_config() {
        let mut layered = LayeredConfig::new();
        
        // Add global layer
        let global = ConfigLayer::new(
            ConfigScope::Global,
            Config::new("glm-4").with_temperature(0.7),
        );
        layered.set_layer(global);
        
        // Add agent layer
        let agent = ConfigLayer::new(
            ConfigScope::Agent("default".into()),
            Config::new("glm-5").with_temperature(0.5),
        );
        layered.set_layer(agent);
        
        // Resolve configuration
        let resolved = layered.resolve_all();
        
        // Agent layer should override global
        assert_eq!(resolved.model, "glm-5");
        assert_eq!(resolved.temperature, 0.5);
    }
    
    #[test]
    fn test_config_context() {
        let ctx = ConfigContext::new()
            .with_agent("default")
            .with_channel("qq")
            .with_user("user123");
        
        let scopes = ctx.to_scopes();
        
        assert_eq!(scopes.len(), 4);
        assert!(scopes.contains(&ConfigScope::Global));
        assert!(scopes.contains(&ConfigScope::Agent("default".into())));
        assert!(scopes.contains(&ConfigScope::Channel("qq".into())));
        assert!(scopes.contains(&ConfigScope::User("user123".into())));
    }
    
    #[test]
    fn test_layered_config_resolution_order() {
        let mut layered = LayeredConfig::new();
        
        // Add layers in reverse order
        let session = ConfigLayer::new(
            ConfigScope::Session("s1".into()),
            Config::new("model-session"),
        );
        layered.set_layer(session);
        
        let global = ConfigLayer::new(
            ConfigScope::Global,
            Config::new("model-global"),
        );
        layered.set_layer(global);
        
        let user = ConfigLayer::new(
            ConfigScope::User("u1".into()),
            Config::new("model-user"),
        );
        layered.set_layer(user);
        
        // Resolve with specific scopes
        let scopes = vec![
            ConfigScope::Global,
            ConfigScope::User("u1".into()),
            ConfigScope::Session("s1".into()),
        ];
        
        let resolved = layered.resolve(&scopes);
        
        // Session should have highest priority
        assert_eq!(resolved.model, "model-session");
    }
}