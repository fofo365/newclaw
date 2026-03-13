// NewClaw - Next-gen AI Agent framework
// Version: 0.6.1

// Allow some warnings for now (WIP code)
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

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

// v0.5.0 - Vector Embedding (向量嵌入)
pub mod embedding;

// v0.5.0 - MCP (Model Context Protocol)
pub mod mcp;

// v0.5.1 - Router (多层路由架构)
pub mod router;

// v0.5.2 - Skill (OpenClaw Skill 兼容)
pub mod skill;

// v0.5.4 - 健康检查
pub mod health;

// v0.5.4 - 指标导出
pub mod metrics;

// v0.5.4 - 缓存层
pub mod cache;

// v0.5.4 - 向量数据库
pub mod vector_db;

// v0.5.5 - 多 Agent 记忆共享
pub mod memory;

// v0.5.5 - 心跳机制
pub mod heartbeat;

// v0.5.5 - 多模型调度
pub mod dispatcher;

// v0.6.0 - Watchdog 核心主控
pub mod watchdog;

// v0.6.0 - Smart Controller 智慧主控集成
pub mod smart_controller;

// v0.7.0 - Task System (任务流原生架构)
pub mod task;

// Re-export main types
pub use core::AgentEngine;
pub use core::{ContextManager, ContextConfig, ContextChunk};
pub use core::{StrategyEngine, Strategy, StrategyType};

// v0.5.0 - Embedding re-exports
pub use embedding::{
    EmbeddingClient, OpenAIEmbeddingClient, EmbeddingConfig,
    EmbeddingCache, CacheConfig,
    EmbeddingPipeline, TextChunker,
    EmbeddingModel, EmbeddingOptions,
    EmbeddingResult, BatchEmbeddingResult,
};

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
// 注释掉旧的导出，使用新的工具系统
// pub use tools::{Tool, ToolOutput, ToolError, ToolResult, ToolDescription, Media, MediaType, ToolRegistry};
// pub use tools::{ReadTool, WriteTool, EditTool, ExecTool, SearchTool};

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
