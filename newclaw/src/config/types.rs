//! Configuration types and error definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Model to use for LLM calls
    pub model: String,
    
    /// Temperature for response generation (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// Maximum tokens in response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    
    /// System prompt override
    pub system_prompt: Option<String>,
    
    /// Available tools for this configuration
    #[serde(default)]
    pub tools: Vec<String>,
    
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    
    /// Configuration version
    #[serde(default = "default_version")]
    pub version: u64,
    
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    
    /// Last modified timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

fn default_version() -> u64 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "glm-4".to_string(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            system_prompt: None,
            tools: Vec::new(),
            metadata: HashMap::new(),
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Config {
    /// Create a new configuration with the specified model
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }
    
    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }
    
    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }
    
    /// Set system prompt
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
    
    /// Add a tool
    pub fn with_tool(mut self, tool: impl Into<String>) -> Self {
        self.tools.push(tool.into());
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// Merge another configuration into this one
    pub fn merge(&mut self, other: &Config) {
        if !other.model.is_empty() {
            self.model = other.model.clone();
        }
        if other.temperature > 0.0 {
            self.temperature = other.temperature;
        }
        if other.max_tokens > 0 {
            self.max_tokens = other.max_tokens;
        }
        if other.system_prompt.is_some() {
            self.system_prompt = other.system_prompt.clone();
        }
        for tool in &other.tools {
            if !self.tools.contains(tool) {
                self.tools.push(tool.clone());
            }
        }
        for (k, v) in &other.metadata {
            self.metadata.insert(k.clone(), v.clone());
        }
        self.version = self.version.max(other.version) + 1;
        self.updated_at = Utc::now();
    }
}

/// Configuration error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),
    
    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("Config file not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Config version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },
    
    #[error("Watch error: {0}")]
    WatchError(String),
    
    #[error("Notification error: {0}")]
    NotificationError(String),
    
    #[error("Layer not found: {0}")]
    LayerNotFound(String),
    
    #[error("Path error: {0}")]
    PathError(String),
}

/// Configuration result type
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Supported configuration file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigFormat {
    Toml,
    Json,
    Yaml,
}

impl ConfigFormat {
    /// Detect format from file extension
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext.to_lowercase().as_str() {
                "toml" => Some(Self::Toml),
                "json" => Some(Self::Json),
                "yaml" | "yml" => Some(Self::Yaml),
                _ => None,
            })
    }
    
    /// Parse configuration from string
    pub fn parse(&self, content: &str) -> ConfigResult<Config> {
        match self {
            Self::Toml => toml::from_str(content).map_err(ConfigError::from),
            Self::Json => serde_json::from_str(content).map_err(ConfigError::from),
            Self::Yaml => serde_yaml::from_str(content).map_err(ConfigError::from),
        }
    }
    
    /// Serialize configuration to string
    pub fn serialize(&self, config: &Config) -> ConfigResult<String> {
        match self {
            Self::Toml => toml::to_string_pretty(config).map_err(ConfigError::from),
            Self::Json => serde_json::to_string_pretty(config).map_err(ConfigError::from),
            Self::Yaml => serde_yaml::to_string(config).map_err(ConfigError::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.model, "glm-4");
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, 4096);
    }
    
    #[test]
    fn test_config_builder() {
        let config = Config::new("glm-5")
            .with_temperature(0.5)
            .with_max_tokens(2048)
            .with_system_prompt("You are a helpful assistant.")
            .with_tool("read")
            .with_metadata("key", "value");
        
        assert_eq!(config.model, "glm-5");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 2048);
        assert!(config.system_prompt.is_some());
        assert!(config.tools.contains(&"read".to_string()));
        assert_eq!(config.metadata.get("key"), Some(&"value".to_string()));
    }
    
    #[test]
    fn test_config_merge() {
        let mut config1 = Config::new("glm-4")
            .with_temperature(0.7)
            .with_tool("read");
        
        let config2 = Config::new("glm-5")
            .with_temperature(0.5)
            .with_tool("write");
        
        config1.merge(&config2);
        
        assert_eq!(config1.model, "glm-5");
        assert_eq!(config1.temperature, 0.5);
        assert!(config1.tools.contains(&"read".to_string()));
        assert!(config1.tools.contains(&"write".to_string()));
    }
    
    #[test]
    fn test_config_format_detection() {
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("config.toml")),
            Some(ConfigFormat::Toml)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("config.json")),
            Some(ConfigFormat::Json)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("config.yaml")),
            Some(ConfigFormat::Yaml)
        );
        assert_eq!(
            ConfigFormat::from_path(std::path::Path::new("config.yml")),
            Some(ConfigFormat::Yaml)
        );
    }
    
    #[test]
    fn test_config_parse_toml() {
        let content = r#"
model = "glm-5"
temperature = 0.5
max_tokens = 2048
"#;
        let config: Config = toml::from_str(content).unwrap();
        assert_eq!(config.model, "glm-5");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 2048);
    }
    
    #[test]
    fn test_config_parse_json() {
        let content = r#"{"model": "glm-5", "temperature": 0.5, "max_tokens": 2048}"#;
        let config: Config = serde_json::from_str(content).unwrap();
        assert_eq!(config.model, "glm-5");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 2048);
    }
}