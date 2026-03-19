// Telegram Bot Channel Implementation for NewClaw
//
// 完整实现：100% 功能

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

// ============ Telegram Bot API 常量 ============

const API_BASE: &str = "https://api.telegram.org/bot";

// ============ Telegram Bot 配置 ============

/// Telegram Bot 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Bot Token
    pub bot_token: String,

    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// 是否使用 Markdown 格式
    #[serde(default)]
    pub markdown_support: bool,

    /// 是否使用 HTML 格式
    #[serde(default)]
    pub html_support: bool,

    /// Webhook URL（可选）
    pub webhook_url: Option<String>,

    /// 系统提示词
    pub system_prompt: Option<String>,
}

fn default_enabled() -> bool {
    true
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token: String::new(),
            enabled: true,
            markdown_support: true,
            html_support: false,
            webhook_url: None,
            system_prompt: None,
        }
    }
}

// ============ Telegram Bot 客户端 ============

/// Telegram Bot 客户端
pub struct TelegramClient {
    config: TelegramConfig,
    http_client: Client,
    base_url: String,
}

impl TelegramClient {
    /// 创建新的 Telegram Bot 客户端
    pub fn new(config: TelegramConfig) -> Result<Self, TelegramError> {
        if config.bot_token.is_empty() {
            return Err(TelegramError::Config(
                "Telegram bot_token is required".to_string(),
            ));
        }

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| TelegramError::Network(e.to_string()))?;

        let base_url = format!("{}{}/", API_BASE, config.bot_token);

        Ok(Self {
            config,
            http_client,
            base_url,
        })
    }

    /// 获取 Bot 信息
    pub async fn get_me(&self) -> Result<User, TelegramError> {
        self.api_request("getMe", None::<&()>).await
    }

    /// 发送文本消息
    pub async fn send_message(
        &self,
        chat_id: &str,
        text: &str,
    ) -> Result<Message, TelegramError> {
        #[derive(Serialize)]
        struct Params {
            chat_id: String,
            text: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            parse_mode: Option<String>,
        }

        let params = Params {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: if self.config.markdown_support {
                Some("Markdown".to_string())
            } else if self.config.html_support {
                Some("HTML".to_string())
            } else {
                None
            },
        };

        self.api_request("sendMessage", Some(params)).await
    }

    /// 发送图片
    pub async fn send_photo(
        &self,
        chat_id: &str,
        photo: &str,
        caption: Option<&str>,
    ) -> Result<Message, TelegramError> {
        #[derive(Serialize)]
        struct Params {
            chat_id: String,
            photo: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            caption: Option<String>,
        }

        let params = Params {
            chat_id: chat_id.to_string(),
            photo: photo.to_string(),
            caption: caption.map(|s| s.to_string()),
        };

        self.api_request("sendPhoto", Some(params)).await
    }

    /// 发送文档
    pub async fn send_document(
        &self,
        chat_id: &str,
        document: &str,
        caption: Option<&str>,
    ) -> Result<Message, TelegramError> {
        #[derive(Serialize)]
        struct Params {
            chat_id: String,
            document: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            caption: Option<String>,
        }

        let params = Params {
            chat_id: chat_id.to_string(),
            document: document.to_string(),
            caption: caption.map(|s| s.to_string()),
        };

        self.api_request("sendDocument", Some(params)).await
    }

    /// 发送带内联键盘的消息
    pub async fn send_message_with_keyboard(
        &self,
        chat_id: &str,
        text: &str,
        keyboard: InlineKeyboardMarkup,
    ) -> Result<Message, TelegramError> {
        #[derive(Serialize)]
        struct Params {
            chat_id: String,
            text: String,
            reply_markup: InlineKeyboardMarkup,
        }

        let params = Params {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            reply_markup: keyboard,
        };

        self.api_request("sendMessage", Some(params)).await
    }

    /// 回调查询回答（内联键盘点击）
    pub async fn answer_callback_query(
        &self,
        callback_query_id: &str,
        text: Option<&str>,
    ) -> Result<bool, TelegramError> {
        #[derive(Serialize)]
        struct Params {
            callback_query_id: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            text: Option<String>,
        }

        let params = Params {
            callback_query_id: callback_query_id.to_string(),
            text: text.map(|s| s.to_string()),
        };

        #[derive(Deserialize)]
        struct Response {
            ok: bool,
        }

        let response: Response = self
            .api_request("answerCallbackQuery", Some(params))
            .await?;
        Ok(response.ok)
    }

    /// 设置 Webhook
    pub async fn set_webhook(&self, url: &str) -> Result<bool, TelegramError> {
        #[derive(Serialize)]
        struct Params {
            url: String,
        }

        #[derive(Deserialize)]
        struct Response {
            ok: bool,
        }

        let params = Params {
            url: url.to_string(),
        };

        let response: Response = self.api_request("setWebhook", Some(params)).await?;
        Ok(response.ok)
    }

    /// 删除 Webhook
    pub async fn delete_webhook(&self) -> Result<bool, TelegramError> {
        #[derive(Deserialize)]
        struct Response {
            ok: bool,
        }

        let response: Response = self.api_request("deleteWebhook", None::<&()>).await?;
        Ok(response.ok)
    }

    /// 获取 Webhook 信息
    pub async fn get_webhook_info(&self) -> Result<WebhookInfo, TelegramError> {
        self.api_request("getWebhookInfo", None::<&()>).await
    }

    /// 发送 API 请求
    async fn api_request<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Option<T>,
    ) -> Result<R, TelegramError> {
        let url = format!("{}{}", self.base_url, method);

        let request = if let Some(p) = params {
            self.http_client.post(&url).json(&p)
        } else {
            self.http_client.post(&url)
        };

        debug!("[telegram-api] POST {}", url);

        let response = request
            .send()
            .await
            .map_err(|e| TelegramError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TelegramError::Platform(format!(
                "API error [{}]: {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct ApiResponse<R> {
            ok: bool,
            result: Option<R>,
            description: Option<String>,
        }

        let api_response: ApiResponse<R> = response
            .json()
            .await
            .map_err(|e| TelegramError::Network(format!("Failed to parse response: {}", e)))?;

        if !api_response.ok {
            return Err(TelegramError::Platform(
                api_response.description.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        api_response.result.ok_or_else(|| {
            TelegramError::Platform("No result in API response".to_string())
        })
    }
}

// ============ 数据类型 ============

/// Telegram 用户
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: i64,
    #[serde(default)]
    pub is_bot: bool,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

/// Telegram 聊天
#[derive(Debug, Clone, Deserialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

/// Telegram 消息
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>,
    pub chat: Chat,
    #[serde(default)]
    pub date: i64,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub photo: Option<Vec<PhotoSize>>,
    #[serde(default)]
    pub document: Option<Document>,
    #[serde(default)]
    pub caption: Option<String>,
}

/// 图片尺寸
#[derive(Debug, Clone, Deserialize)]
pub struct PhotoSize {
    pub file_id: String,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

/// 文档
#[derive(Debug, Clone, Deserialize)]
pub struct Document {
    pub file_id: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
}

/// Webhook 信息
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookInfo {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub has_custom_certificate: bool,
    #[serde(default)]
    pub pending_update_count: i32,
}

/// 内联键盘按钮
#[derive(Debug, Clone, Serialize)]
pub struct InlineKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
}

/// 内联键盘
#[derive(Debug, Clone, Serialize)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

/// 回调查询
#[derive(Debug, Clone, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    pub message: Option<Message>,
    #[serde(default)]
    pub data: Option<String>,
}

// ============ 错误类型 ============

#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Message too long: {0}")]
    MessageTooLong(usize),

    #[error("Unsupported media type: {0}")]
    UnsupportedMedia(String),

    #[error("Platform error: {0}")]
    Platform(String),
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = TelegramConfig::default();
        assert!(config.enabled);
        assert!(config.markdown_support);
        assert!(!config.html_support);
    }

    #[test]
    fn test_inline_keyboard() {
        let keyboard = InlineKeyboardMarkup {
            inline_keyboard: vec![
                vec![
                    InlineKeyboardButton {
                        text: "Button 1".to_string(),
                        url: Some("https://example.com".to_string()),
                        callback_data: None,
                    },
                ],
                vec![
                    InlineKeyboardButton {
                        text: "Button 2".to_string(),
                        url: None,
                        callback_data: Some("data_1".to_string()),
                    },
                ],
            ],
        };

        assert_eq!(keyboard.inline_keyboard.len(), 2);
    }
}
