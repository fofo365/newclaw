// NewClaw - Next-gen AI Agent framework
// Version: 0.5.0

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

// v0.3.0 - Tool execution engine
pub mod tools;

// v0.4.0 - Feishu WebSocket connection management
pub mod feishu_websocket;

// v0.4.0 - Dashboard Web UI
pub mod dashboard;

// v0.5.0 - Context Manager (智能上下文管理)
pub mod context;

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

// v0.3.0 - Tool re-exports
pub use tools::{Tool, ToolOutput, ToolError, ToolResult, ToolDescription, Media, MediaType, ToolRegistry};
pub use tools::{ReadTool, WriteTool, EditTool, ExecTool, SearchTool};

// v0.4.0 - Feishu WebSocket re-exports
pub use feishu_websocket::{
    FeishuWebSocketManager, WebSocketConfig, WebSocketError, WebSocketResult,
    ConnectionPool, Connection, ConnectionState,
    HeartbeatManager, HeartbeatConfig,
    ReconnectionManager, ReconnectStrategy,
    EventHandler, FeishuEvent, LogLevel,
};

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
