// Feishu/Lark Channel Integration

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub encrypt_key: Option<String>,
    pub verification_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuMessage {
    pub chat_id: String,
    pub content: String,
    pub message_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuEvent {
    pub event_type: String,
    pub open_id: String,
    pub text: String,
    pub chat_id: String,
}

#[async_trait]
pub trait FeishuClient: Send + Sync {
    async fn send_message(&self, message: &FeishuMessage) -> Result<String>;
    
    async fn get_events(&self) -> Result<Vec<FeishuEvent>>;
    
    async fn verify(&self, challenge: &str) -> Result<String>;
}

pub struct FeishuApiClient {
    config: FeishuConfig,
    base_url: String,
    access_token: Option<String>,
}

impl FeishuApiClient {
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            config,
            base_url: "https://open.feishu.cn/open-apis".to_string(),
            access_token: None,
        }
    }
    
    async fn get_access_token(&mut self) -> Result<String> {
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }
        
        let client = reqwest::Client::new();
        let url = format!("{}/auth/v3/tenant_access_token/internal", self.base_url);
        
        let request_body = serde_json::json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret,
        });
        
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;
        
        let json: serde_json::Value = response.json().await?;
        
        if json["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!("Failed to get access token: {:?}", json));
        }
        
        let token = json["tenant_access_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No token in response"))?
            .to_string();
        
        self.access_token = Some(token.clone());
        
        Ok(token)
    }
    
    async fn refresh_access_token(&mut self) -> Result<String> {
        self.access_token = None;
        self.get_access_token().await
    }
}

#[async_trait]
impl FeishuClient for FeishuApiClient {
    async fn send_message(&self, message: &FeishuMessage) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!("{}/im/v1/messages", self.base_url);
        
        let token = self.access_token.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No access token"))?;
        
        let request_body = serde_json::json!({
            "receive_id_type": "chat_id",
            "receive_id": message.chat_id,
            "msg_type": message.message_type,
            "content": serde_json::json!({
                "text": message.content
            })
        });
        
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_body)
            .send()
            .await?;
        
        let json: serde_json::Value = response.json().await?;
        
        if json["code"].as_i64() != Some(0) {
            return Err(anyhow::anyhow!("Failed to send message: {:?}", json));
        }
        
        let message_id = json["data"]["message_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No message_id in response"))?
            .to_string();
        
        Ok(message_id)
    }
    
    async fn get_events(&self) -> Result<Vec<FeishuEvent>> {
        // TODO: Implement event polling
        Ok(vec![])
    }
    
    async fn verify(&self, challenge: &str) -> Result<String> {
        Ok(challenge.to_string())
    }
}
