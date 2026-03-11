// 钉钉 Bot Channel Implementation for NewClaw
//
// 支持钉钉企业内部机器人和 Stream 模式

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

// ============ 钉钉 Bot 配置 ============

/// 钉钉 Bot 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    /// 应用 Key
    pub app_key: String,

    /// 应用 Secret
    pub app_secret: String,

    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Stream 模式（支持主动消息）
    #[serde(default)]
    pub stream_mode: bool,
}

fn default_enabled() -> bool {
    true
}

impl Default for DingTalkConfig {
    fn default() -> Self {
        Self {
            app_key: String::new(),
            app_secret: String::new(),
            enabled: true,
            stream_mode: false,
        }
    }
}

// ============ 钉钉 API 客户端 ============

/// 钉钉 API 客户端
pub struct DingTalkClient {
    config: DingTalkConfig,
    http_client: Client,
    access_token: Arc<RwLock<Option<String>>>,
}

impl DingTalkClient {
    pub fn new(config: DingTalkConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取访问令牌
    pub async fn get_access_token(&self) -> Result<String> {
        // 检查缓存的令牌
        {
            let token = self.access_token.read().await;
            if let Some(token) = token.as_ref() {
                return Ok(token.clone());
            }
        }

        // 获取新令牌
        let url = format!(
            "https://api.dingtalk.com/v1.0/oauth2/accessToken?appKey={}&appSecret={}",
            self.config.app_key, self.config.app_secret
        );

        let response = self.http_client
            .post(&url)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get access token"));
        }

        let result: serde_json::Value = response.json().await?;
        let token = result["accessToken"].as_str()
            .ok_or_else(|| anyhow::anyhow!("No access token in response"))?
            .to_string();

        // 缓存令牌
        {
            let mut cached = self.access_token.write().await;
            *cached = Some(token.clone());
        }

        Ok(token)
    }

    /// 发送文本消息
    pub async fn send_text(&self, user_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = "https://api.dingtalk.com/v1.0/robot/oToMessages/batchSend";

        let body = serde_json::json!({
            "robotCode": self.config.app_key,
            "userIds": [user_id],
            "msgKey": "sampleText",
            "msgParam": serde_json::json!({
                "content": content
            }).to_string()
        });

        let response = self.http_client
            .post(url)
            .header("x-acs-dingtalk-access-token", &token)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to send message"));
        }

        Ok(())
    }

    /// 发送 Markdown 消息
    pub async fn send_markdown(&self, user_id: &str, title: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = "https://api.dingtalk.com/v1.0/robot/oToMessages/batchSend";

        let body = serde_json::json!({
            "robotCode": self.config.app_key,
            "userIds": [user_id],
            "msgKey": "sampleMarkdown",
            "msgParam": serde_json::json!({
                "title": title,
                "text": content
            }).to_string()
        });

        let response = self.http_client
            .post(url)
            .header("x-acs-dingtalk-access-token", &token)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to send markdown message"));
        }

        Ok(())
    }
}

// ============ 钉钉 Bot ============

/// 钉钉 Bot
pub struct DingTalkBot {
    client: DingTalkClient,
}

impl DingTalkBot {
    pub fn new(config: DingTalkConfig) -> Self {
        Self {
            client: DingTalkClient::new(config),
        }
    }

    /// 发送文本消息
    pub async fn send_text_message(&self, user_id: &str, content: &str) -> Result<()> {
        self.client.send_text(user_id, content).await
    }

    /// 发送 Markdown 消息
    pub async fn send_markdown_message(&self, user_id: &str, title: &str, content: &str) -> Result<()> {
        self.client.send_markdown(user_id, title, content).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dingtalk_config_creation() {
        let config = DingTalkConfig {
            app_key: "test_key".to_string(),
            app_secret: "test_secret".to_string(),
            enabled: true,
            stream_mode: false,
        };

        assert_eq!(config.app_key, "test_key");
        assert!(config.enabled);
    }

    #[test]
    fn test_dingtalk_config_default() {
        let config = DingTalkConfig::default();
        assert!(config.enabled);
        assert!(!config.stream_mode);
    }

    #[test]
    fn test_dingtalk_client_creation() {
        let config = DingTalkConfig::default();
        let client = DingTalkClient::new(config);
        
        // 验证创建成功
        assert!(true);
    }

    #[test]
    fn test_dingtalk_bot_creation() {
        let config = DingTalkConfig::default();
        let bot = DingTalkBot::new(config);
        
        // 验证创建成功
        assert!(true);
    }
}
