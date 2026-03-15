// Communication Layer - v0.2.0
// Provides inter-agent communication via WebSocket, HTTP, and Message Queue

pub mod message;
pub mod websocket;
pub mod http;
#[cfg(feature = "redis-support")]
pub mod message_queue;

pub use message::{InterAgentMessage, MessageId, MessagePayload, MessagePriority};
pub use websocket::{WebSocketServer, WebSocketClient};
pub use http::{HttpApiServer, HttpClient};

#[cfg(feature = "redis-support")]
pub use message_queue::RedisMessageQueue;

use async_trait::async_trait;
use anyhow::Result;

pub type AgentId = String;

/// Communication protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommProtocol {
    WebSocket,
    HTTP,
    MessageQueue,
}

/// Agent communicator trait
#[async_trait]
pub trait AgentCommunicator: Send + Sync {
    /// Send a message
    async fn send(&mut self, msg: InterAgentMessage) -> Result<()>;
    
    /// Receive a message
    async fn receive(&mut self) -> Result<InterAgentMessage>;
    
    /// Broadcast a message to all connected agents
    async fn broadcast(&mut self, msg: InterAgentMessage) -> Result<()>;
    
    /// Subscribe to a topic
    async fn subscribe(&mut self, topic: &str) -> Result<()>;
    
    /// Send heartbeat
    async fn heartbeat(&mut self) -> Result<bool>;
    
    /// Close connection
    async fn close(&mut self) -> Result<()>;
}

/// Communication configuration
#[derive(Debug, Clone)]
pub struct CommunicationConfig {
    pub websocket_enabled: bool,
    pub websocket_port: u16,
    pub http_enabled: bool,
    pub http_port: u16,
    #[cfg(feature = "redis-support")]
    pub redis_enabled: bool,
    #[cfg(feature = "redis-support")]
    pub redis_url: String,
}

impl Default for CommunicationConfig {
    fn default() -> Self {
        Self {
            websocket_enabled: true,
            websocket_port: 8080,
            http_enabled: true,
            http_port: 3000,
            #[cfg(feature = "redis-support")]
            redis_enabled: false,
            #[cfg(feature = "redis-support")]
            redis_url: "redis://localhost".to_string(),
        }
    }
}
