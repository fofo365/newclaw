// NewClaw v0.4.0 - 飞书 WebSocket 连接管理
//
// 核心功能：
// 1. WebSocket 连接池管理
// 2. 自动重连机制
// 3. 心跳检测
// 4. 事件处理
// 5. 事件轮询系统
// 6. 消息类型支持
// 7. 错误重试机制

pub mod pool;
pub mod heartbeat;
pub mod reconnect;
pub mod event;
pub mod manager;
pub mod polling;
pub mod messages;
pub mod retry;

// Re-exports
pub use pool::{ConnectionPool, Connection, ConnectionState};
pub use heartbeat::{HeartbeatManager, HeartbeatConfig};
pub use reconnect::{ReconnectionManager, ReconnectStrategy};
pub use event::{EventHandler, FeishuEvent};
pub use manager::FeishuWebSocketManager;
pub use polling::{EventPoller, EventQueue, PollingConfig, PollingEvent, PollingManager};
pub use messages::{
    MessageType, BaseMessage, TextMessage, RichTextMessage, CardMessage,
    ImageMessage, FileMessage, MessageSender, ReceiveIdType,
    TextContent, RichTextContent, CardContent, ImageContent, FileContent,
    CardText, CardElement, RichTextParagraph, TextStyle,
};
pub use retry::{
    RetryExecutor, RetryStrategy, RetryContext, RetryManager, RetryMetrics,
    ErrorCategory, ErrorSeverity, AlertRule, FallbackStrategy,
    CacheFallback, DefaultValueFallback,
};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// WebSocket 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// 基础 URL
    pub base_url: String,
    
    /// 应用 ID
    pub app_id: String,
    
    /// 应用密钥
    pub app_secret: String,
    
    /// 心跳间隔（默认 30s）
    pub heartbeat_interval: Duration,
    
    /// 心跳超时（默认 10s）
    pub heartbeat_timeout: Duration,
    
    /// 最大心跳失败次数（默认 3）
    pub max_heartbeat_failures: u32,
    
    /// 是否启用自动重连
    pub enable_auto_reconnect: bool,
    
    /// 最大重连次数（默认 10）
    pub max_reconnect_attempts: u32,
    
    /// 初始重连延迟（默认 1s）
    pub initial_reconnect_delay: Duration,
    
    /// 最大重连延迟（默认 60s）
    pub max_reconnect_delay: Duration,
    
    /// 最大连接数
    pub max_connections: usize,
    
    /// 日志级别
    pub log_level: LogLevel,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            base_url: "wss://open.feishu.cn/open-apis/ws/v2".to_string(),
            app_id: String::new(),
            app_secret: String::new(),
            heartbeat_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(10),
            max_heartbeat_failures: 3,
            enable_auto_reconnect: true,
            max_reconnect_attempts: 10,
            initial_reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(60),
            max_connections: 10,
            log_level: LogLevel::Info,
        }
    }
}

/// WebSocket 错误类型
#[derive(Debug, Clone, thiserror::Error)]
pub enum WebSocketError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    
    #[error("Heartbeat timeout")]
    HeartbeatTimeout,
    
    #[error("Max reconnection attempts reached")]
    MaxReconnectAttempts,
    
    #[error("Pool is full")]
    PoolFull,
    
    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<std::io::Error> for WebSocketError {
    fn from(err: std::io::Error) -> Self {
        WebSocketError::Io(err.to_string())
    }
}

impl From<reqwest::Error> for WebSocketError {
    fn from(err: reqwest::Error) -> Self {
        WebSocketError::WebSocket(err.to_string())
    }
}

pub type WebSocketResult<T> = Result<T, WebSocketError>;

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_websocket_error() {
        let err = WebSocketError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("test"));
    }
    
    #[test]
    fn test_log_level() {
        let level = LogLevel::Info;
        let json = serde_json::to_string(&level).unwrap();
        assert!(json.contains("Info"));
    }
}
