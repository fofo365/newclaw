// QQ Bot Channel Implementation for NewClaw
//
// 基于 OpenClaw qqbot 插件的完整 Rust 实现
// 支持 QQ 官方 Bot API（QQ 开放平台）

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, debug, error};

// ============ QQ Bot API 常量 ============

const API_BASE: &str = "https://api.sgroup.qq.com";
const TOKEN_URL: &str = "https://bots.qq.com/app/getAppAccessToken";
const GATEWAY_URL: &str = "wss://api.sgroup.qq.com/websocket";

// ============ QQ Bot 配置 ============

/// QQ Bot 账户配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QQConfig {
    /// 账户 ID
    #[serde(default = "default_account_id")]
    pub account_id: String,

    /// 应用 ID
    pub app_id: String,

    /// 应用密钥
    pub client_secret: String,

    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// 是否支持 Markdown 消息（需要机器人具备该权限）
    #[serde(default)]
    pub markdown_support: bool,

    /// 系统提示词
    pub system_prompt: Option<String>,

    /// 图床服务器公网地址
    pub image_server_base_url: Option<String>,
}

fn default_account_id() -> String {
    "default".to_string()
}

fn default_enabled() -> bool {
    true
}

impl Default for QQConfig {
    fn default() -> Self {
        Self {
            account_id: default_account_id(),
            app_id: String::new(),
            client_secret: String::new(),
            enabled: true,
            markdown_support: false,
            system_prompt: None,
            image_server_base_url: None,
        }
    }
}

// ============ AccessToken 管理 ============

/// AccessToken 缓存
#[derive(Debug, Clone)]
struct TokenCache {
    token: String,
    expires_at: i64, // Unix timestamp (seconds)
}

/// QQ Bot 客户端
pub struct QQClient {
    config: QQConfig,
    http_client: Client,
    token_cache: Arc<RwLock<Option<TokenCache>>>,
}

impl QQClient {
    /// 创建新的 QQ Bot 客户端
    pub fn new(config: QQConfig) -> Result<Self, QQError> {
        if config.app_id.is_empty() || config.client_secret.is_empty() {
            return Err(QQError::Config(
                "QQBot requires app_id and client_secret".to_string(),
            ));
        }

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| QQError::Network(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
            token_cache: Arc::new(RwLock::new(None)),
        })
    }

    /// 获取 AccessToken（带缓存）
    pub async fn get_access_token(&self) -> Result<String, QQError> {
        // 检查缓存
        {
            let cache = self.token_cache.read().await;
            if let Some(ref token_cache) = *cache {
                let now = chrono::Utc::now().timestamp();
                // 提前 5 分钟刷新
                if now < token_cache.expires_at - 300 {
                    return Ok(token_cache.token.clone());
                }
            }
        }

        // 获取新 Token
        self.fetch_access_token().await
    }

    /// 从 API 获取 AccessToken
    async fn fetch_access_token(&self) -> Result<String, QQError> {
        #[derive(Serialize)]
        struct TokenRequest {
            app_id: String,
            client_secret: String,
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: i64,
        }

        let request_body = TokenRequest {
            app_id: self.config.app_id.clone(),
            client_secret: self.config.client_secret.clone(),
        };

        let response = self
            .http_client
            .post(TOKEN_URL)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| QQError::Network(format!("Failed to fetch token: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(QQError::Auth(format!(
                "Token fetch failed: {} - {}",
                status, body
            )));
        }

        let token_data: TokenResponse = response
            .json()
            .await
            .map_err(|e| QQError::Network(format!("Failed to parse token response: {}", e)))?;

        // 更新缓存
        let now = chrono::Utc::now().timestamp();
        let cache = TokenCache {
            token: token_data.access_token.clone(),
            expires_at: now + token_data.expires_in,
        };

        {
            let mut token_cache = self.token_cache.write().await;
            *token_cache = Some(cache);
        }

        info!(
            "[qqbot] Token refreshed, expires in {} seconds",
            token_data.expires_in
        );

        Ok(token_data.access_token)
    }

    /// 清除 Token 缓存
    pub async fn clear_token_cache(&self) {
        let mut cache = self.token_cache.write().await;
        *cache = None;
    }

    /// 解析目标地址
    pub fn parse_target(target: &str) -> Result<TargetInfo, QQError> {
        let target = target.trim_start_matches("qqbot:");

        let (target_type, id) = if target.starts_with("c2c:") {
            (TargetType::Direct, target[4..].to_string())
        } else if target.starts_with("group:") {
            (TargetType::Group, target[6..].to_string())
        } else if target.starts_with("channel:") {
            (TargetType::Channel, target[8..].to_string())
        } else if target.len() == 32 && target.chars().all(|c| c.is_ascii_hexdigit()) {
            // 纯 openid（32位十六进制）
            (TargetType::Direct, target.to_string())
        } else {
            (TargetType::Direct, target.to_string())
        };

        Ok(TargetInfo {
            target_type,
            id,
            raw: target.to_string(),
        })
    }

    /// 发送文本消息
    pub async fn send_text(
        &self,
        target: &str,
        text: &str,
        msg_id: Option<String>,
    ) -> Result<MessageResponse, QQError> {
        let target_info = Self::parse_target(target)?;
        let token = self.get_access_token().await?;

        #[derive(Serialize)]
        struct MessageBody {
            #[serde(skip_serializing_if = "Option::is_none")]
            content: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            markdown: Option<MarkdownContent>,
            msg_type: u8,
            #[serde(skip_serializing_if = "Option::is_none")]
            msg_id: Option<String>,
        }

        #[derive(Serialize)]
        struct MarkdownContent {
            content: String,
        }

        let body = if self.config.markdown_support {
            MessageBody {
                content: None,
                markdown: Some(MarkdownContent {
                    content: text.to_string(),
                }),
                msg_type: 2,
                msg_id,
            }
        } else {
            MessageBody {
                content: Some(text.to_string()),
                markdown: None,
                msg_type: 0,
                msg_id,
            }
        };

        let path = match target_info.target_type {
            TargetType::Direct => format!("/v2/users/{}/messages", target_info.id),
            TargetType::Group => format!("/v2/groups/{}/messages", target_info.id),
            TargetType::Channel => format!("/channels/{}/messages", target_info.id),
        };

        let response: MessageResponse = self
            .api_request("POST", &path, Some(&body))
            .await?;

        info!(
            "[qqbot] Message sent to {}: msg_id={}",
            target,
            response.id
        );

        Ok(response)
    }

    /// 发送 API 请求
    async fn api_request<T: Serialize + Send, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        path: &str,
        body: Option<&T>,
    ) -> Result<R, QQError> {
        let token = self.get_access_token().await?;
        let url = format!("{}{}", API_BASE, path);

        let request = match method {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            _ => return Err(QQError::Config(format!("Unsupported HTTP method: {}", method))),
        }
        .bearer_auth(&token)
        .header("Content-Type", "application/json");

        let request = if let Some(b) = body {
            request.json(b)
        } else {
            request
        };

        debug!("[qqbot-api] >>> {} {}", method, url);

        let response = request
            .send()
            .await
            .map_err(|e| QQError::Network(e.to_string()))?;

        let status = response.status();
        debug!("[qqbot-api] <<< Status: {}", status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(QQError::Platform(format!(
                "API error [{}]: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| QQError::Network(format!("Failed to parse response: {}", e)))
    }

    /// 主动发送 C2C 消息
    pub async fn send_proactive_c2c(
        &self,
        openid: &str,
        content: &str,
    ) -> Result<MessageResponse, QQError> {
        if content.trim().is_empty() {
            return Err(QQError::Config(
                "Proactive message content cannot be empty".to_string(),
            ));
        }

        self.send_text(openid, content, None).await
    }

    /// 主动发送群聊消息
    pub async fn send_proactive_group(
        &self,
        group_openid: &str,
        content: &str,
    ) -> Result<MessageResponse, QQError> {
        if content.trim().is_empty() {
            return Err(QQError::Config(
                "Proactive message content cannot be empty".to_string(),
            ));
        }

        self.send_text(group_openid, content, None).await
    }
}

// ============ 数据类型 ============

/// 目标类型
#[derive(Debug, Clone, PartialEq)]
pub enum TargetType {
    Direct,    // 私聊
    Group,     // 群聊
    Channel,   // 频道
}

/// 目标地址信息
#[derive(Debug, Clone)]
pub struct TargetInfo {
    pub target_type: TargetType,
    pub id: String,
    pub raw: String,
}

/// 消息响应
#[derive(Debug, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(default)]
    pub timestamp: i64,
}

// ============ 错误类型 ============

#[derive(Debug, thiserror::Error)]
pub enum QQError {
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
    fn test_parse_target() {
        // C2C
        let target = QQClient::parse_target("c2c:ABC123DEF456").unwrap();
        assert!(matches!(target.target_type, TargetType::Direct));
        assert_eq!(target.id, "ABC123DEF456");

        // Group
        let target = QQClient::parse_target("group:GROUP123").unwrap();
        assert!(matches!(target.target_type, TargetType::Group));
        assert_eq!(target.id, "GROUP123");

        // Channel
        let target = QQClient::parse_target("channel:CH123").unwrap();
        assert!(matches!(target.target_type, TargetType::Channel));
        assert_eq!(target.id, "CH123");
    }

    #[test]
    fn test_config_default() {
        let config = QQConfig::default();
        assert_eq!(config.account_id, "default");
        assert!(config.enabled);
        assert!(!config.markdown_support);
    }
}
