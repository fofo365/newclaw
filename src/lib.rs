// NewClaw - Next-gen AI Agent framework
// Version: 0.2.0

// Core modules
pub mod core;
pub mod channels;
pub mod config;
pub mod cli;
pub mod llm;
pub mod gateway;
pub mod vector;
pub mod plugin;
pub mod openclaw;

// v0.2.0 - Security and Communication modules
pub mod security;
pub mod communication;

// Re-export main types
pub use core::AgentEngine;
pub use core::{ContextManager, ContextConfig, ContextChunk};
pub use core::{StrategyEngine, Strategy, StrategyType};

// v0.2.0 - Security re-exports
pub use security::{
    ApiKeyAuth, JwtAuth, RbacManager, Permission, Role, AuditLogger, AuditEntry, RateLimiter,
    SecurityConfig,
};

// v0.2.0 - Communication re-exports
pub use communication::{
    InterAgentMessage, MessageId, MessagePayload, MessagePriority,
    WebSocketServer, WebSocketClient, HttpApiServer, HttpClient,
    CommProtocol, CommunicationConfig,
};

#[cfg(feature = "redis-support")]
pub use communication::RedisMessageQueue;

// v0.2.0 - Core isolation
pub use core::IsolationLevel;

/// NewClaw version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Create a new agent with default configuration
pub fn create_agent(name: String, model: String) -> anyhow::Result<AgentEngine> {
    AgentEngine::new(name, model)
}

/// Create a new agent with security enabled
pub fn create_secure_agent(
    name: String,
    model: String,
    config: SecurityConfig,
) -> anyhow::Result<(AgentEngine, ApiKeyAuth, JwtAuth, RbacManager)> {
    let agent = AgentEngine::new(name, model)?;
    let api_key_auth = ApiKeyAuth::new();
    let jwt_auth = JwtAuth::new(config.jwt_secret.clone());
    let rbac = RbacManager::new();
    
    Ok((agent, api_key_auth, jwt_auth, rbac))
}
