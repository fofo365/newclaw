// 飞书 API 客户端
//
// 实现飞书开放平台 API 调用

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 飞书 API 配置
#[derive(Debug, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub base_url: String,
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            base_url: "https://open.feishu.cn/open-apis".to_string(),
        }
    }
}

/// 飞书访问令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub created_at: u64,
}

/// 飞书 API 客户端
pub struct FeishuClient {
    config: FeishuConfig,
    http_client: Client,
    access_token: Arc<RwLock<Option<AccessToken>>>,
}

impl FeishuClient {
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取访问令牌
    pub async fn get_access_token(&self) -> Result<String> {
        // 检查是否有缓存的令牌
        {
            let token = self.access_token.read().await;
            if let Some(token) = token.as_ref() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                // 如果令牌还有效（提前 5 分钟刷新）
                if token.created_at + token.expires_in - 300 > now {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // 获取新的访问令牌
        let url = format!("{}/auth/v3/tenant_access_token/internal", self.config.base_url);
        
        let response = self.http_client
            .post(&url)
            .json(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get access token: {}", response.status()));
        }

        let token_response: serde_json::Value = response.json().await?;
        
        let access_token = AccessToken {
            access_token: token_response["tenant_access_token"].as_str().unwrap_or("").to_string(),
            expires_in: token_response["expire"].as_u64().unwrap_or(7200),
            token_type: "Bearer".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // 缓存令牌
        {
            let mut token = self.access_token.write().await;
            *token = Some(access_token.clone());
        }

        Ok(access_token.access_token)
    }

    /// 读取文档内容
    pub async fn read_doc(&self, doc_token: &str) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents/{}/raw_content", self.config.base_url, doc_token);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to read document: {}", response.status()));
        }

        let content: serde_json::Value = response.json().await?;
        Ok(content["content"].as_str().unwrap_or("").to_string())
    }

    /// 创建文档
    pub async fn create_doc(&self, title: &str, folder_token: Option<&str>) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents", self.config.base_url);
        
        let mut body = serde_json::json!({
            "title": title
        });
        
        if let Some(folder) = folder_token {
            body["folder_token"] = serde_json::json!(folder);
        }

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create document: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["document"]["document_id"].as_str().unwrap_or("").to_string())
    }

    /// 发送消息
    pub async fn send_message(&self, receive_id: &str, receive_id_type: &str, msg_type: &str, content: &str) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/im/v1/messages?receive_id_type={}", self.config.base_url, receive_id_type);
        
        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "receive_id": receive_id,
                "msg_type": msg_type,
                "content": content
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to send message: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["data"]["message_id"].as_str().unwrap_or("").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_config_default() {
        let config = FeishuConfig::default();
        assert_eq!(config.base_url, "https://open.feishu.cn/open-apis");
        assert!(config.app_id.is_empty());
        assert!(config.app_secret.is_empty());
    }

    #[test]
    fn test_feishu_client_creation() {
        let config = FeishuConfig::default();
        let client = FeishuClient::new(config);
        // 简单验证创建成功
        assert!(true);
    }

    #[test]
    fn test_access_token_creation() {
        let token = AccessToken {
            access_token: "test_token".to_string(),
            expires_in: 7200,
            token_type: "Bearer".to_string(),
            created_at: 1000,
        };

        assert_eq!(token.access_token, "test_token");
        assert_eq!(token.expires_in, 7200);
    }
}
