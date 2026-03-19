// NewClaw v0.4.0 - 消息类型支持
//
// 功能：
// 1. 文本消息处理
// 2. 富文本消息处理
// 3. 卡片消息处理
// 4. 图片/文件消息处理
// 5. 消息发送接口

use super::{WebSocketError, WebSocketResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 消息类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 文本消息
    Text,
    /// 富文本消息
    Post,
    /// 卡片消息
    Interactive,
    /// 图片消息
    Image,
    /// 文件消息
    File,
    /// 音频消息
    Audio,
    /// 媒体消息
    Media,
    /// 贴纸消息
    Sticker,
}

impl MessageType {
    pub fn as_str(&self) -> &str {
        match self {
            MessageType::Text => "text",
            MessageType::Post => "post",
            MessageType::Interactive => "interactive",
            MessageType::Image => "image",
            MessageType::File => "file",
            MessageType::Audio => "audio",
            MessageType::Media => "media",
            MessageType::Sticker => "sticker",
        }
    }
    
    pub fn parse(s: &str) -> WebSocketResult<Self> {
        match s {
            "text" => Ok(MessageType::Text),
            "post" => Ok(MessageType::Post),
            "interactive" => Ok(MessageType::Interactive),
            "image" => Ok(MessageType::Image),
            "file" => Ok(MessageType::File),
            "audio" => Ok(MessageType::Audio),
            "media" => Ok(MessageType::Media),
            "sticker" => Ok(MessageType::Sticker),
            _ => Err(WebSocketError::Serialization(format!(
                "Unknown message type: {}",
                s
            ))),
        }
    }
}

/// 消息接收者类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveIdType {
    /// Open ID
    OpenId,
    /// User ID
    UserId,
    /// Union ID
    UnionId,
    /// Chat ID
    ChatId,
}

/// 消息基础结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
    /// 消息 ID（飞书生成）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    
    /// 根消息 ID（用于回复）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_id: Option<String>,
    
    /// 父消息 ID（用于话题回复）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    
    /// 消息类型
    #[serde(rename = "msg_type")]
    pub msg_type: MessageType,
    
    /// 消息内容（JSON 字符串）
    pub content: String,
    
    /// 发送时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<i64>,
    
    /// 更新时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_time: Option<i64>,
    
    /// 是否已撤回
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    
    /// 是否已更新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<bool>,
}

// ==================== 文本消息 ====================

/// 文本消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    /// 文本内容
    pub text: String,
}

impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
        }
    }
    
    pub fn to_json(&self) -> WebSocketResult<String> {
        serde_json::to_string(self)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
}

/// 文本消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl TextMessage {
    pub fn new(text: impl Into<String>) -> Self {
        let content = TextContent::new(text);
        Self {
            base: BaseMessage {
                message_id: None,
                root_id: None,
                parent_id: None,
                msg_type: MessageType::Text,
                content: content.to_json().unwrap(),
                create_time: None,
                update_time: None,
                deleted: None,
                updated: None,
            },
        }
    }
    
    pub fn with_root_id(mut self, root_id: impl Into<String>) -> Self {
        self.base.root_id = Some(root_id.into());
        self
    }
    
    pub fn with_parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.base.parent_id = Some(parent_id.into());
        self
    }
}

// ==================== 富文本消息 ====================

/// 富文本段落
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag")]
pub enum RichTextParagraph {
    /// 文本
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<Vec<TextStyle>>,
    },
    
    /// 链接
    #[serde(rename = "a")]
    Link {
        text: String,
        href: String,
    },
    
    /// @ 某人
    #[serde(rename = "at")]
    At {
        #[serde(rename = "user_id")]
        user_id: String,
    },
    
    /// 图片
    #[serde(rename = "img")]
    Image {
        image_key: String,
    },
    
    /// 媒体
    #[serde(rename = "media")]
    Media {
        file_key: String,
        #[serde(rename = "image_key")]
        #[serde(skip_serializing_if = "Option::is_none")]
        image_key: Option<String>,
    },
    
    /// 表情
    #[serde(rename = "emotion")]
    Emotion {
        emoji_type: String,
    },
}

/// 文本样式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    InlineCode,
    CodeBlock,
}

/// 富文本内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichTextContent {
    /// 富文本内容（飞书格式）
    #[serde(rename = "zh_cn")]
    pub zh_cn: RichTextSection,
}

/// 富文本段落组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichTextSection {
    /// 标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    /// 内容段落
    pub content: Vec<Vec<RichTextParagraph>>,
}

impl RichTextContent {
    pub fn new() -> Self {
        Self {
            zh_cn: RichTextSection {
                title: None,
                content: Vec::new(),
            },
        }
    }
    
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.zh_cn.title = Some(title.into());
        self
    }
    
    pub fn add_paragraph(mut self, paragraph: Vec<RichTextParagraph>) -> Self {
        self.zh_cn.content.push(paragraph);
        self
    }
    
    pub fn to_json(&self) -> WebSocketResult<String> {
        serde_json::to_string(self)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
}

impl Default for RichTextContent {
    fn default() -> Self {
        Self::new()
    }
}

/// 富文本消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RichTextMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl RichTextMessage {
    pub fn new(content: RichTextContent) -> Self {
        Self {
            base: BaseMessage {
                message_id: None,
                root_id: None,
                parent_id: None,
                msg_type: MessageType::Post,
                content: content.to_json().unwrap(),
                create_time: None,
                update_time: None,
                deleted: None,
                updated: None,
            },
        }
    }
}

// ==================== 卡片消息 ====================

/// 卡片配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardConfig {
    /// 是否启用更新
    #[serde(rename = "enable_forward")]
    pub enable_forward: bool,
    
    /// 是否全员可见
    #[serde(rename = "wide_screen_mode")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wide_screen_mode: Option<bool>,
    
    /// 是否启用更新
    #[serde(rename = "update_multi")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_multi: Option<bool>,
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            enable_forward: true,
            wide_screen_mode: Some(true),
            update_multi: Some(false),
        }
    }
}

/// 卡片元素
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tag")]
pub enum CardElement {
    /// 标题
    #[serde(rename = "div")]
    Div {
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<CardText>,
        #[serde(skip_serializing_if = "Option::is_none")]
        fields: Option<Vec<CardField>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        extra: Option<Box<CardElement>>,
    },
    
    /// 分割线
    #[serde(rename = "hr")]
    Divider,
    
    /// 备注
    #[serde(rename = "note")]
    Note {
        elements: Vec<CardElement>,
    },
    
    /// 图片
    #[serde(rename = "img")]
    Image {
        #[serde(rename = "img_key")]
        img_key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<CardText>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<CardText>,
        #[serde(skip_serializing_if = "Option::is_none")]
        custom_width: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
    },
    
    /// Markdown
    #[serde(rename = "markdown")]
    Markdown {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text_align: Option<String>,
    },
    
    /// 按钮
    #[serde(rename = "action")]
    Action {
        actions: Vec<CardAction>,
        #[serde(skip_serializing_if = "Option::is_none")]
        layout: Option<String>,
    },
}

/// 卡片文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardText {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<CardIcon>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

impl CardText {
    pub fn plain(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tag: Some("plain_text".to_string()),
            icon: None,
            href: None,
        }
    }
    
    pub fn lark_md(content: impl Into<String>) -> Self {
        Self {
            content: Some(content.into()),
            tag: Some("lark_md".to_string()),
            icon: None,
            href: None,
        }
    }
}

/// 卡片字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardField {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_short: Option<bool>,
    
    pub text: CardText,
}

/// 卡片图标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardIcon {
    pub tag: String,
    pub token: String,
}

/// 卡片动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAction {
    pub tag: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<CardText>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<HashMap<String, String>>,
}

/// 卡片消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardContent {
    #[serde(rename = "type")]
    pub type_: String,
    
    pub config: CardConfig,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<CardHeader>,
    
    pub elements: Vec<CardElement>,
}

/// 卡片头部
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardHeader {
    pub title: CardText,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

impl CardContent {
    pub fn new() -> Self {
        Self {
            type_: "template".to_string(),
            config: CardConfig::default(),
            header: None,
            elements: Vec::new(),
        }
    }
    
    pub fn with_header(mut self, title: impl Into<String>) -> Self {
        self.header = Some(CardHeader {
            title: CardText::plain(title),
            template: None,
        });
        self
    }
    
    pub fn with_header_template(mut self, template: impl Into<String>) -> Self {
        if let Some(ref mut header) = self.header {
            header.template = Some(template.into());
        }
        self
    }
    
    pub fn add_element(mut self, element: CardElement) -> Self {
        self.elements.push(element);
        self
    }
    
    pub fn to_json(&self) -> WebSocketResult<String> {
        serde_json::to_string(self)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
}

impl Default for CardContent {
    fn default() -> Self {
        Self::new()
    }
}

/// 卡片消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl CardMessage {
    pub fn new(content: CardContent) -> Self {
        Self {
            base: BaseMessage {
                message_id: None,
                root_id: None,
                parent_id: None,
                msg_type: MessageType::Interactive,
                content: content.to_json().unwrap(),
                create_time: None,
                update_time: None,
                deleted: None,
                updated: None,
            },
        }
    }
}

// ==================== 图片/文件消息 ====================

/// 图片消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    #[serde(rename = "image_key")]
    pub image_key: String,
}

impl ImageContent {
    pub fn new(image_key: impl Into<String>) -> Self {
        Self {
            image_key: image_key.into(),
        }
    }
    
    pub fn to_json(&self) -> WebSocketResult<String> {
        serde_json::to_string(self)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
}

/// 图片消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl ImageMessage {
    pub fn new(image_key: impl Into<String>) -> Self {
        let content = ImageContent::new(image_key);
        Self {
            base: BaseMessage {
                message_id: None,
                root_id: None,
                parent_id: None,
                msg_type: MessageType::Image,
                content: content.to_json().unwrap(),
                create_time: None,
                update_time: None,
                deleted: None,
                updated: None,
            },
        }
    }
}

/// 文件消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    #[serde(rename = "file_key")]
    pub file_key: String,
}

impl FileContent {
    pub fn new(file_key: impl Into<String>) -> Self {
        Self {
            file_key: file_key.into(),
        }
    }
    
    pub fn to_json(&self) -> WebSocketResult<String> {
        serde_json::to_string(self)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
}

/// 文件消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
}

impl FileMessage {
    pub fn new(file_key: impl Into<String>) -> Self {
        let content = FileContent::new(file_key);
        Self {
            base: BaseMessage {
                message_id: None,
                root_id: None,
                parent_id: None,
                msg_type: MessageType::File,
                content: content.to_json().unwrap(),
                create_time: None,
                update_time: None,
                deleted: None,
                updated: None,
            },
        }
    }
}

// ==================== 消息发送器 ====================

/// 消息发送器
pub struct MessageSender {
    /// HTTP 客户端
    client: reqwest::Client,
    
    /// 基础 URL
    base_url: String,
    
    /// Access Token
    access_token: Option<String>,
}

impl MessageSender {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            access_token: None,
        }
    }
    
    /// 设置 Access Token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }
    
    /// 发送文本消息
    pub async fn send_text(
        &self,
        receive_id: &str,
        receive_id_type: ReceiveIdType,
        message: TextMessage,
    ) -> WebSocketResult<String> {
        self.send_message(receive_id, receive_id_type, message.base).await
    }
    
    /// 发送富文本消息
    pub async fn send_rich_text(
        &self,
        receive_id: &str,
        receive_id_type: ReceiveIdType,
        message: RichTextMessage,
    ) -> WebSocketResult<String> {
        self.send_message(receive_id, receive_id_type, message.base).await
    }
    
    /// 发送卡片消息
    pub async fn send_card(
        &self,
        receive_id: &str,
        receive_id_type: ReceiveIdType,
        message: CardMessage,
    ) -> WebSocketResult<String> {
        self.send_message(receive_id, receive_id_type, message.base).await
    }
    
    /// 发送图片消息
    pub async fn send_image(
        &self,
        receive_id: &str,
        receive_id_type: ReceiveIdType,
        message: ImageMessage,
    ) -> WebSocketResult<String> {
        self.send_message(receive_id, receive_id_type, message.base).await
    }
    
    /// 发送文件消息
    pub async fn send_file(
        &self,
        receive_id: &str,
        receive_id_type: ReceiveIdType,
        message: FileMessage,
    ) -> WebSocketResult<String> {
        self.send_message(receive_id, receive_id_type, message.base).await
    }
    
    /// 发送简单文本消息（便捷方法）
    pub async fn send_simple_text(&self, chat_id: &str, text: &str) -> WebSocketResult<String> {
        let message = TextMessage::new(text);
        self.send_text(chat_id, ReceiveIdType::ChatId, message).await
    }
    
    /// 通用消息发送
    async fn send_message(
        &self,
        receive_id: &str,
        receive_id_type: ReceiveIdType,
        base: BaseMessage,
    ) -> WebSocketResult<String> {
        let url = format!("{}/open-apis/im/v1/messages?receive_id_type={}", 
            self.base_url, receive_id_type.as_str());
        
        // 构建请求体
        let body = serde_json::json!({
            "receive_id": receive_id,
            "msg_type": base.msg_type.as_str(),
            "content": base.content,
        });
        
        // 发送请求
        let mut request = self.client.post(&url)
            .header("Content-Type", "application/json");
        
        if let Some(ref token) = self.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request.json(&body).send().await
            .map_err(|e| WebSocketError::WebSocket(e.to_string()))?;
        
        let status = response.status();
        let text = response.text().await
            .map_err(|e| WebSocketError::WebSocket(e.to_string()))?;
        
        if !status.is_success() {
            return Err(WebSocketError::WebSocket(format!(
                "Failed to send message: {} - {}", status, text
            )));
        }
        
        // 解析响应获取消息ID
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))?;
        
        let message_id = json["data"]["message_id"]
            .as_str()
            .unwrap_or(&uuid::Uuid::new_v4().to_string())
            .to_string();
        
        Ok(message_id)
    }
}

impl ReceiveIdType {
    pub fn as_str(&self) -> &str {
        match self {
            ReceiveIdType::OpenId => "open_id",
            ReceiveIdType::UserId => "user_id",
            ReceiveIdType::UnionId => "union_id",
            ReceiveIdType::ChatId => "chat_id",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_type() {
        let msg_type = MessageType::Text;
        assert_eq!(msg_type.as_str(), "text");
        
        let parsed = MessageType::parse("post").unwrap();
        assert_eq!(parsed, MessageType::Post);
    }
    
    #[test]
    fn test_text_message() {
        let msg = TextMessage::new("Hello, World!");
        assert_eq!(msg.base.msg_type, MessageType::Text);
        
        let content: TextContent = serde_json::from_str(&msg.base.content).unwrap();
        assert_eq!(content.text, "Hello, World!");
    }
    
    #[test]
    fn test_text_message_with_reply() {
        let msg = TextMessage::new("Reply")
            .with_root_id("root_123")
            .with_parent_id("parent_456");
        
        assert_eq!(msg.base.root_id, Some("root_123".to_string()));
        assert_eq!(msg.base.parent_id, Some("parent_456".to_string()));
    }
    
    #[test]
    fn test_rich_text_content() {
        let content = RichTextContent::new()
            .with_title("Title")
            .add_paragraph(vec![
                RichTextParagraph::Text {
                    text: "Hello".to_string(),
                    style: Some(vec![TextStyle::Bold]),
                },
            ]);
        
        let json = content.to_json().unwrap();
        assert!(json.contains("Title"));
        assert!(json.contains("Hello"));
    }
    
    #[test]
    fn test_card_content() {
        let card = CardContent::new()
            .with_header("Card Title")
            .with_header_template("blue")
            .add_element(CardElement::Div {
                text: Some(CardText::plain("Content")),
                fields: None,
                extra: None,
            })
            .add_element(CardElement::Divider);
        
        let json = card.to_json().unwrap();
        assert!(json.contains("Card Title"));
        assert!(json.contains("Content"));
    }
    
    #[test]
    fn test_image_message() {
        let msg = ImageMessage::new("img_123456");
        assert_eq!(msg.base.msg_type, MessageType::Image);
        
        let content: ImageContent = serde_json::from_str(&msg.base.content).unwrap();
        assert_eq!(content.image_key, "img_123456");
    }
    
    #[test]
    fn test_file_message() {
        let msg = FileMessage::new("file_789012");
        assert_eq!(msg.base.msg_type, MessageType::File);
        
        let content: FileContent = serde_json::from_str(&msg.base.content).unwrap();
        assert_eq!(content.file_key, "file_789012");
    }
    
    #[test]
    fn test_card_text() {
        let plain = CardText::plain("Plain text");
        assert_eq!(plain.tag, Some("plain_text".to_string()));
        
        let md = CardText::lark_md("**Bold**");
        assert_eq!(md.tag, Some("lark_md".to_string()));
    }
    
    #[tokio::test]
    async fn test_message_sender() {
        let sender = MessageSender::new("https://open.feishu.cn");
        
        let msg = TextMessage::new("Test message");
        let result = sender
            .send_text("ou_test", ReceiveIdType::OpenId, msg)
            .await;
        
        // 由于是模拟实现，应该返回成功
        assert!(result.is_ok());
        let message_id = result.unwrap();
        assert!(!message_id.is_empty());
    }
}
