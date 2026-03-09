// Configuration Module - v0.2.0
pub mod security;
pub mod communication;

pub use security::SecurityConfig;
pub use communication::CommunicationConfig;

use serde::{Deserialize, Serialize};

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub agent: AgentConfig,
    pub security: SecurityConfig,
    pub communication: CommunicationConfig,
    pub context: ContextConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            security: SecurityConfig::default(),
            communication: CommunicationConfig::default(),
            context: ContextConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub version: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "newclaw-agent".to_string(),
            model: "glm-4".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub isolation: IsolationLevel,
    pub max_tokens: usize,
    pub db_path: String,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            isolation: IsolationLevel::None,
            max_tokens: 8000,
            db_path: "newclaw.db".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    None,
    User,
    Session,
}

impl std::fmt::Display for IsolationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsolationLevel::None => write!(f, "none"),
            IsolationLevel::User => write!(f, "user"),
            IsolationLevel::Session => write!(f, "session"),
        }
    }
}

impl std::str::FromStr for IsolationLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(IsolationLevel::None),
            "user" => Ok(IsolationLevel::User),
            "session" => Ok(IsolationLevel::Session),
            _ => Err(format!("Invalid isolation level: {}", s)),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load from environment variables
    pub fn from_env() -> Self {
        let mut config = Config::default();

        if let Ok(name) = std::env::var("NEWCLAW_AGENT_NAME") {
            config.agent.name = name;
        }

        if let Ok(model) = std::env::var("NEWCLAW_MODEL") {
            config.agent.model = model;
        }

        if let Ok(secret) = std::env::var("NEWCLAW_JWT_SECRET") {
            config.security.jwt_secret = secret;
        }

        if let Ok(port) = std::env::var("NEWCLAW_WS_PORT") {
            if let Ok(port) = port.parse() {
                config.communication.websocket_port = port;
            }
        }

        if let Ok(port) = std::env::var("NEWCLAW_HTTP_PORT") {
            if let Ok(port) = port.parse() {
                config.communication.http_port = port;
            }
        }

        config
    }
}
