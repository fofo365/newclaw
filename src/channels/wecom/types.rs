// NewClaw v0.4.0 - 企业微信（WeCom）类型定义
//
// 包含所有 WeCom 相关的结构体和枚举定义

use serde::{Deserialize, Serialize};

/// WeCom 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    /// 企业 ID
    pub corp_id: String,
    /// 应用 Secret
    pub corp_secret: String,
    /// 应用 ID
    pub agent_id: String,
    /// Token（用于消息签名验证）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// EncodingAESKey（用于消息加密/解密）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_aes_key: Option<String>,
    /// 接收消息的 ID（企业 ID 或应用 ID）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_id: Option<String>,
}

/// AccessToken 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    /// AccessToken
    pub access_token: String,
    /// 过期时间（秒）
    pub expires_in: i64,
}

/// API 错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 错误码
    pub errcode: i32,
    /// 错误消息
    pub errmsg: String,
}

/// 发送消息的目标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MessageTarget {
    /// 接收用户 ID（多个用 | 分隔）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touser: Option<String>,
    /// 接收部门 ID（多个用 | 分隔）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toparty: Option<String>,
    /// 接收标签 ID（多个用 | 分隔）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totag: Option<String>,
}

/// 文本消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    /// 消息内容（最长 2048 字节）
    pub content: String,
}

/// 图片消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMessage {
    /// 媒体 ID
    pub media_id: String,
}

/// 文件消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMessage {
    /// 媒体 ID
    pub media_id: String,
}

/// 视频消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMessage {
    /// 媒体 ID
    pub media_id: String,
    /// 标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// 语音消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceMessage {
    /// 媒体 ID
    pub media_id: String,
}

/// 消息类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "msgtype", rename_all = "lowercase")]
pub enum MessageType {
    Text { text: TextMessage },
    Image { image: ImageMessage },
    File { file: FileMessage },
    Video { video: VideoMessage },
    Voice { voice: VoiceMessage },
}

/// 发送消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// 接收用户
    #[serde(skip_serializing_if = "Option::is_none")]
    pub touser: Option<String>,
    /// 接收部门
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toparty: Option<String>,
    /// 接收标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub totag: Option<String>,
    /// 应用 ID
    pub agentid: String,
    /// 消息类型
    #[serde(flatten)]
    pub msg_type: MessageType,
    /// 是否是保密消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe: Option<i32>,
    /// 启用 ID 转译
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_id_trans: Option<i32>,
    /// 启用重复检查
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_duplicate_check: Option<i32>,
    /// 重复检查间隔
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_check_interval: Option<i32>,
}

/// 发送消息响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    /// 错误码（0 表示成功）
    pub errcode: i32,
    /// 错误消息
    pub errmsg: String,
    /// 无效用户
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invaliduser: Option<String>,
    /// 无效部门
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidparty: Option<String>,
    /// 无效标签
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalidtag: Option<String>,
    /// 消息 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msgid: Option<String>,
}

/// 媒体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Voice,
    Video,
    File,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Voice => "voice",
            MediaType::Video => "video",
            MediaType::File => "file",
        }
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// 上传媒体响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadMediaResponse {
    /// 媒体 ID
    pub media_id: String,
    /// 媒体类型
    #[serde(rename = "type")]
    pub media_type: String,
    /// 创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// 错误码
    #[serde(default)]
    pub errcode: i32,
    /// 错误消息
    #[serde(default)]
    pub errmsg: String,
}

/// 下载媒体响应
#[derive(Debug, Clone)]
pub struct DownloadMediaResponse {
    /// 文件内容
    pub buffer: Vec<u8>,
    /// Content-Type
    pub content_type: String,
    /// 文件名
    pub filename: Option<String>,
}

/// Webhook 消息基类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMessageBase {
    /// 消息 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msgid: Option<String>,
    /// 机器人 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aibotid: Option<String>,
    /// 会话类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chattype: Option<String>,
    /// 会话 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatid: Option<String>,
    /// 回复 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_url: Option<String>,
    /// 发送者信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<WebhookFrom>,
    /// 消息类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msgtype: Option<String>,
}

/// 发送者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookFrom {
    /// 用户 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userid: Option<String>,
    /// 企业 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpid: Option<String>,
}

/// Webhook 文本消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTextMessage {
    #[serde(flatten)]
    pub base: WebhookMessageBase,
    /// 文本内容
    pub text: Option<WebhookText>,
    /// 引用消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<WebhookQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookText {
    pub content: Option<String>,
}

/// Webhook 引用消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookQuote {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msgtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<WebhookText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<WebhookImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<WebhookFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookImage {
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookFile {
    pub url: Option<String>,
}

/// Webhook 事件消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEventMessage {
    #[serde(flatten)]
    pub base: WebhookMessageBase,
    /// 创建时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_time: Option<i64>,
    /// 事件内容
    pub event: Option<WebhookEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// 事件类型
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eventtype: Option<String>,
    /// 事件数据
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// Webhook 事件枚举
#[derive(Debug, Clone)]
pub enum WebhookInbound {
    Text(WebhookTextMessage),
    Event(WebhookEventMessage),
    Unknown(serde_json::Value),
}

/// API 端点常量
pub mod api {
    pub const GET_TOKEN: &str = "https://qyapi.weixin.qq.com/cgi-bin/gettoken";
    pub const SEND_MESSAGE: &str = "https://qyapi.weixin.qq.com/cgi-bin/message/send";
    pub const SEND_APPCHAT: &str = "https://qyapi.weixin.qq.com/cgi-bin/appchat/send";
    pub const UPLOAD_MEDIA: &str = "https://qyapi.weixin.qq.com/cgi-bin/media/upload";
    pub const DOWNLOAD_MEDIA: &str = "https://qyapi.weixin.qq.com/cgi-bin/media/get";
}

/// 限制常量
pub mod limits {
    /// 文本消息最大字节数
    pub const TEXT_MAX_BYTES: usize = 2048;
    /// Token 刷新缓冲时间（毫秒）
    pub const TOKEN_REFRESH_BUFFER_MS: i64 = 60_000;
    /// HTTP 请求超时（毫秒）
    pub const REQUEST_TIMEOUT_MS: u64 = 15_000;
    /// 最大请求体大小
    pub const MAX_REQUEST_BODY_SIZE: usize = 1024 * 1024;
}

/// 加密常量
pub mod crypto {
    /// PKCS#7 块大小
    pub const PKCS7_BLOCK_SIZE: usize = 32;
    /// AES Key 长度
    pub const AES_KEY_LENGTH: usize = 32;
    /// IV 长度
    pub const IV_LENGTH: usize = 16;
}
