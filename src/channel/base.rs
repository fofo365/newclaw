// 通道层基础抽象

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{ChannelType, ChannelMember, ChannelCapabilities, ChannelPermission};
use crate::tools::ToolRegistry;

/// 通道消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// 消息 ID
    pub message_id: String,
    /// 通道类型
    pub channel_type: ChannelType,
    /// 发送者
    pub sender: ChannelMember,
    /// 聊天/会话 ID
    pub chat_id: String,
    /// 消息内容
    pub content: MessageContent,
    /// 时间戳
    pub timestamp: i64,
    /// 引用的消息 ID (可选)
    pub reply_to: Option<String>,
    /// 元数据
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// 纯文本
    Text(String),
    /// 富文本 (Markdown)
    RichText(String),
    /// 图片
    Image {
        url: String,
        caption: Option<String>,
    },
    /// 文件
    File {
        name: String,
        url: String,
        size: u64,
        mime_type: String,
    },
    /// 语音
    Voice {
        url: String,
        duration: u32,
    },
    /// 位置
    Location {
        latitude: f64,
        longitude: f64,
        name: Option<String>,
    },
    /// 混合内容
    Mixed(Vec<MessageContent>),
}

impl MessageContent {
    /// 获取文本内容
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(t) => Some(t),
            MessageContent::RichText(t) => Some(t),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn to_string_content(&self) -> String {
        match self {
            MessageContent::Text(t) => t.clone(),
            MessageContent::RichText(t) => t.clone(),
            MessageContent::Image { url, caption } => {
                format!("[Image: {}] {}", url, caption.as_deref().unwrap_or(""))
            }
            MessageContent::File { name, .. } => format!("[File: {}]", name),
            MessageContent::Voice { .. } => "[Voice message]".to_string(),
            MessageContent::Location { name, .. } => {
                format!("[Location: {}]", name.as_deref().unwrap_or("Unknown"))
            }
            MessageContent::Mixed(items) => {
                items.iter().map(|i| i.to_string_content()).collect::<Vec<_>>().join("\n")
            }
        }
    }
}

/// 通道响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelResponse {
    /// 消息 ID (如果发送了消息)
    pub message_id: Option<String>,
    /// 响应内容
    pub content: MessageContent,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
}

/// 通道上下文
#[derive(Clone)]
pub struct ChannelContext {
    /// 通道类型
    pub channel_type: ChannelType,
    /// 工具注册表 (共享)
    pub tools: Arc<ToolRegistry>,
    /// 权限管理 (共享)
    pub permissions: Arc<ChannelPermission>,
    /// 通道能力
    pub capabilities: ChannelCapabilities,
    /// 配置
    pub config: serde_json::Map<String, serde_json::Value>,
}

impl ChannelContext {
    /// 创建新的通道上下文
    pub fn new(
        channel_type: ChannelType,
        tools: Arc<ToolRegistry>,
        permissions: Arc<ChannelPermission>,
    ) -> Self {
        Self {
            channel_type,
            tools,
            permissions,
            capabilities: ChannelCapabilities::default(),
            config: serde_json::Map::new(),
        }
    }

    /// 设置能力
    pub fn with_capabilities(mut self, capabilities: ChannelCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// 设置配置
    pub fn with_config(mut self, key: &str, value: serde_json::Value) -> Self {
        self.config.insert(key.to_string(), value);
        self
    }
}

/// 通道 trait - 所有通道必须实现
#[async_trait]
pub trait Channel: Send + Sync {
    /// 获取通道类型
    fn channel_type(&self) -> ChannelType;

    /// 获取通道名称
    fn name(&self) -> &str;

    /// 获取通道上下文
    fn context(&self) -> &ChannelContext;

    /// 获取工具注册表
    fn tools(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.context().tools)
    }

    /// 获取权限管理
    fn permissions(&self) -> Arc<ChannelPermission> {
        Arc::clone(&self.context().permissions)
    }

    /// 检查权限
    async fn check_permission(
        &self,
        member: &ChannelMember,
        tool_name: &str,
    ) -> bool {
        self.permissions().check(member, tool_name).await
    }

    /// 处理消息 (核心方法)
    async fn handle_message(&self, message: ChannelMessage) -> anyhow::Result<ChannelResponse>;

    /// 发送消息
    async fn send_message(
        &self,
        chat_id: &str,
        content: MessageContent,
    ) -> anyhow::Result<ChannelResponse>;

    /// 启动通道
    async fn start(&self) -> anyhow::Result<()>;

    /// 停止通道
    async fn stop(&self) -> anyhow::Result<()>;

    /// 获取通道状态
    fn status(&self) -> ChannelStatus;
}

/// 通道状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChannelStatus {
    /// 已停止
    Stopped,
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 错误
    Error(String),
}

impl Default for ChannelStatus {
    fn default() -> Self {
        ChannelStatus::Stopped
    }
}

/// 基础通道实现
pub struct BaseChannel {
    /// 通道名称
    name: String,
    /// 通道类型
    channel_type: ChannelType,
    /// 上下文
    context: ChannelContext,
    /// 状态
    status: std::sync::RwLock<ChannelStatus>,
}

impl BaseChannel {
    /// 创建基础通道
    pub fn new(
        name: impl Into<String>,
        channel_type: ChannelType,
        tools: Arc<ToolRegistry>,
        permissions: Arc<ChannelPermission>,
    ) -> Self {
        Self {
            name: name.into(),
            channel_type: channel_type.clone(),
            context: ChannelContext::new(channel_type, tools, permissions),
            status: std::sync::RwLock::new(ChannelStatus::Stopped),
        }
    }

    /// 设置状态
    pub fn set_status(&self, status: ChannelStatus) {
        *self.status.write().unwrap() = status;
    }
}

#[async_trait]
impl Channel for BaseChannel {
    fn channel_type(&self) -> ChannelType {
        self.channel_type.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn context(&self) -> &ChannelContext {
        &self.context
    }

    async fn handle_message(&self, _message: ChannelMessage) -> anyhow::Result<ChannelResponse> {
        // 基础实现，子类应该覆盖
        Ok(ChannelResponse {
            message_id: None,
            content: MessageContent::Text("Not implemented".to_string()),
            success: false,
            error: Some("Base channel does not implement handle_message".to_string()),
        })
    }

    async fn send_message(
        &self,
        _chat_id: &str,
        _content: MessageContent,
    ) -> anyhow::Result<ChannelResponse> {
        Ok(ChannelResponse {
            message_id: None,
            content: MessageContent::Text(String::new()),
            success: false,
            error: Some("Base channel does not implement send_message".to_string()),
        })
    }

    async fn start(&self) -> anyhow::Result<()> {
        self.set_status(ChannelStatus::Running);
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.set_status(ChannelStatus::Stopped);
        Ok(())
    }

    fn status(&self) -> ChannelStatus {
        self.status.read().unwrap().clone()
    }
}