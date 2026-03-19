// AGP Channel - 联邦网络通道
//
// 将 AGP（Agent Gateway Protocol）网络作为一等公民 Channel
// 生命周期与 Agent 绑定，通过声明式配置加入联邦

mod config;
pub mod coordinator;
pub mod session;
pub mod message;

pub use config::{AGPConfig, FederationDomain};
pub use coordinator::{CoordinatorClient, EmbeddedCoordinator, AgentInfo, Registration};
pub use session::AGPSession;
pub use message::AGPMessage;

use async_trait::async_trait;
use std::sync::Arc;

/// 简化的 Channel 错误类型
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Not connected: {0}")]
    NotConnected(String),
    #[error("Invalid config: {0}")]
    InvalidConfig(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for ChannelError {
    fn from(err: anyhow::Error) -> Self {
        ChannelError::Other(err.to_string())
    }
}

/// Channel 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelHealth {
    Healthy,
    Unhealthy,
}

/// 消息角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// 简化的消息结构
#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
    pub role: MessageRole,
    pub channel: String,
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 消息处理器 trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle_message(&self, message: Message) -> Result<(), ChannelError>;
}

/// Channel trait
#[async_trait]
pub trait Channel: Send + Sync {
    fn channel_type(&self) -> &str;
    async fn start(&mut self, handler: Arc<dyn MessageHandler>) -> Result<(), ChannelError>;
    async fn send(&self, message: Message, target: Option<String>) -> Result<(), ChannelError>;
    async fn close(&mut self) -> Result<(), ChannelError>;
    async fn health_check(&self) -> ChannelHealth;
}

/// AGP Channel - 符合 NewClaw Channel 契约的联邦网络适配器
///
/// 生命周期：
/// - Agent 启动时：连接协调平面 → 注册身份 → 启动监听
/// - Agent 运行时：接收联邦消息 → 转换为 NewClaw Message → 触发 Agent 主循环
/// - Agent 关闭时：注销身份 → 关闭连接 → 清理资源
pub struct AGPChannel {
    config: AGPConfig,
    session: Option<AGPSession>,
    coordinator: Option<CoordinatorClient>,
    message_handler: Option<Arc<dyn MessageHandler>>,
}

impl AGPChannel {
    /// 创建新的 AGP Channel
    pub fn new(config: AGPConfig) -> Self {
        Self {
            config,
            session: None,
            coordinator: None,
            message_handler: None,
        }
    }

    /// 从配置文件创建
    pub fn from_config(config: serde_yaml::Value) -> Result<Self, ChannelError> {
        let config: AGPConfig = serde_yaml::from_value(config)
            .map_err(|e| ChannelError::InvalidConfig(e.to_string()))?;
        Ok(Self::new(config))
    }

    /// 自动检测本地 endpoint
    fn detect_local_endpoint(&self) -> String {
        // TODO: 实现自动检测逻辑
        // 1. 检查环境变量 NEWCLAW_AGP_ENDPOINT
        // 2. 检查配置文件
        // 3. 使用默认值
        if let Ok(endpoint) = std::env::var("NEWCLAW_AGP_ENDPOINT") {
            return endpoint;
        }
        format!("agp://localhost:7777/{}", self.config.agent_id)
    }
}

#[async_trait]
impl Channel for AGPChannel {
    /// Channel 类型标识
    fn channel_type(&self) -> &str {
        "agp"
    }

    /// 启动 Channel（Agent 初始化时调用）
    async fn start(&mut self, handler: Arc<dyn MessageHandler>) -> Result<(), ChannelError> {
        self.message_handler = Some(handler.clone());

        // 1. 连接轻量协调平面（获取网络身份）
        self.coordinator = Some(
            CoordinatorClient::connect(&self.config.bootstrap).await?
        );

        let assignment = self.coordinator.as_ref().unwrap()
            .register(
                &self.config.agent_id,
                &self.config.advertise,
                self.config.endpoint.clone()
                    .unwrap_or_else(|| self.detect_local_endpoint())
            )
            .await?;

        tracing::info!(
            "AGP Channel: Registered as '{}' with {} initial peers",
            assignment.identity,
            assignment.initial_peers.len()
        );

        // 2. 启动 AGP 监听（长期运行）
        self.session = Some(
            AGPSession::new(
                assignment.identity,
                assignment.initial_peers,
                self.config.domain.as_ref().map(|d| FederationDomain(d.clone())),
            ).await?
        );

        // 3. 启动消息接收循环
        let session = self.session.as_ref().unwrap().clone();
        let handler_clone = handler.clone();
        tokio::spawn(async move {
            while let Some(agp_msg) = session.receive().await {
                // 将 AGP 消息转换为 NewClaw Message
                let message = Message {
                    content: agp_msg.payload,
                    role: crate::channels::agp::MessageRole::User,
                    channel: "agp".to_string(),
                    metadata: serde_json::json!({
                        "remote_id": agp_msg.sender,
                        "reply_addr": agp_msg.reply_addr,
                        "federation_domain": agp_msg.domain,
                        "correlation_id": agp_msg.correlation_id,
                    }),
                    timestamp: chrono::Utc::now(),
                };

                // 触发 Agent 主循环
                if let Err(e) = handler_clone.handle_message(message).await {
                    tracing::error!("AGP message handler error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// 发送消息到联邦网络
    async fn send(&self, message: Message, target: Option<String>) -> Result<(), ChannelError> {
        let session = self.session.as_ref()
            .ok_or_else(|| ChannelError::NotConnected("AGP session not initialized".to_string()))?;

        let target_id = target.ok_or_else(|| {
            ChannelError::InvalidInput("AGP Channel requires explicit target (remote Agent ID)".to_string())
        })?;

        session.send(AGPMessage {
            sender: self.config.agent_id.clone(),
            recipient: target_id,
            payload: message.content,
            reply_addr: None,
            correlation_id: message.metadata.get("correlation_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            domain: self.config.domain.as_ref().map(|d| message::FederationDomain(d.clone())),
        }).await?;

        Ok(())
    }

    /// 关闭 Channel（Agent 关闭时调用）
    async fn close(&mut self) -> Result<(), ChannelError> {
        // 1. 注销身份
        if let Some(coordinator) = &self.coordinator {
            coordinator.unregister(&self.config.agent_id).await?;
        }

        // 2. 关闭会话
        if let Some(session) = &self.session {
            session.leave().await?;
        }

        tracing::info!("AGP Channel closed");
        Ok(())
    }

    /// 健康检查
    async fn health_check(&self) -> ChannelHealth {
        match &self.session {
            Some(session) if session.is_connected() => ChannelHealth::Healthy,
            _ => ChannelHealth::Unhealthy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agp_config_parsing() {
        let yaml = r#"
agent_id: "test-agent"
bootstrap: "agp://localhost:8000"
advertise:
  - "math-solver"
domain: "test-mesh"
"#;
        let config: AGPConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.agent_id, "test-agent");
        assert_eq!(config.bootstrap, "agp://localhost:8000");
        assert_eq!(config.advertise, vec!["math-solver"]);
        assert_eq!(config.domain, Some("test-mesh".to_string()));
    }

    #[test]
    fn test_detect_local_endpoint() {
        let config = AGPConfig {
            agent_id: "my-agent".to_string(),
            bootstrap: "agp://localhost:8000".to_string(),
            advertise: vec![],
            domain: None,
            endpoint: None,
            timeout_secs: None,
            heartbeat_interval_secs: None,
        };
        let channel = AGPChannel::new(config);
        let endpoint = channel.detect_local_endpoint();
        assert!(endpoint.contains("my-agent"));
    }
}
