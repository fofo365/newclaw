// NewClaw v0.4.0 - 企业微信（WeCom）Webhook 处理
//
// 处理企业微信 Webhook 回调：
// 1. URL 验证
// 2. 消息接收和解密
// 3. 事件处理

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use super::crypto::WeComCrypto;
use super::types::*;

/// Webhook URL 验证请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlVerifyRequest {
    #[serde(rename = "msg_signature")]
    pub msg_signature: String,
    pub timestamp: String,
    pub nonce: String,
    pub echostr: String,
}

/// Webhook URL 验证响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlVerifyResponse {
    pub echostr: String,
}

/// Webhook 加密消息包装
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedWebhookMessage {
    #[serde(rename = "ToUserName")]
    pub to_user_name: String,
    #[serde(rename = "AgentID")]
    pub agent_id: String,
    #[serde(rename = "Encrypt")]
    pub encrypt: String,
}

/// Webhook 处理器
pub struct WeComWebhook {
    crypto: WeComCrypto,
}

impl WeComWebhook {
    /// 创建新的 Webhook 处理器
    pub fn new(crypto: WeComCrypto) -> Self {
        Self { crypto }
    }
    
    /// 从配置创建
    pub fn from_config(config: &WeComConfig) -> Result<Self> {
        let encoding_aes_key = config.encoding_aes_key.as_ref()
            .ok_or_else(|| anyhow!("encoding_aes_key is required for webhook"))?;
        let crypto = WeComCrypto::new(
            encoding_aes_key.clone(),
            config.token.clone(),
            config.receive_id.clone(),
        )?;
        Ok(Self { crypto })
    }
    
    /// 验证 URL（首次配置时调用）
    pub fn verify_url(&self, req: &UrlVerifyRequest) -> Result<String> {
        // 验证签名
        if !self.crypto.verify(&req.timestamp, &req.nonce, &req.echostr, &req.msg_signature) {
            return Err(anyhow!("Signature verification failed"));
        }
        
        // 解密 echostr
        let decrypted = self.crypto.decrypt(&req.echostr)?;
        Ok(decrypted)
    }
    
    /// 处理消息回调
    pub fn handle_message(
        &self,
        msg_signature: &str,
        timestamp: &str,
        nonce: &str,
        encrypt: &str,
    ) -> Result<WebhookInbound> {
        // 验证签名
        if !self.crypto.verify(timestamp, nonce, encrypt, msg_signature) {
            return Err(anyhow!("Signature verification failed"));
        }
        
        // 解密消息
        let decrypted = self.crypto.decrypt(encrypt)?;
        
        // 解析 JSON
        let json: serde_json::Value = serde_json::from_str(&decrypted)
            .map_err(|e| anyhow!("Failed to parse decrypted JSON: {}", e))?;
        
        // 根据消息类型分发
        let msgtype = json.get("msgtype")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let inbound = match msgtype {
            "text" => {
                let msg: WebhookTextMessage = serde_json::from_value(json)
                    .map_err(|e| anyhow!("Failed to parse text message: {}", e))?;
                WebhookInbound::Text(msg)
            }
            "event" => {
                let msg: WebhookEventMessage = serde_json::from_value(json)
                    .map_err(|e| anyhow!("Failed to parse event message: {}", e))?;
                WebhookInbound::Event(msg)
            }
            _ => WebhookInbound::Unknown(json),
        };
        
        Ok(inbound)
    }
    
    /// 解析并验证 XML 格式的回调
    pub fn parse_xml_callback(&self, xml: &str) -> Result<EncryptedWebhookMessage> {
        // 使用 quick-xml 解析
        let doc: EncryptedWebhookMessage = quick_xml::de::from_str(xml)
            .map_err(|e| anyhow!("Failed to parse XML: {}", e))?;
        Ok(doc)
    }
    
    /// 获取加密客户端
    pub fn crypto(&self) -> &WeComCrypto {
        &self.crypto
    }
}

/// 构建加密的回复消息
pub fn build_encrypted_reply(
    crypto: &WeComCrypto,
    plaintext_json: &serde_json::Value,
) -> Result<EncryptedReply> {
    let nonce = generate_nonce();
    let timestamp = chrono::Utc::now().timestamp().to_string();
    
    let plaintext = serde_json::to_string(plaintext_json)
        .map_err(|e| anyhow!("Failed to serialize JSON: {}", e))?;
    
    let encrypt = crypto.encrypt(&plaintext)?;
    let msg_signature = crypto.compute_signature(&timestamp, &nonce, &encrypt)
        .ok_or_else(|| anyhow!("Token not configured"))?;
    
    Ok(EncryptedReply {
        encrypt,
        msg_signature,
        timestamp,
        nonce,
    })
}

/// 加密的回复
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedReply {
    pub encrypt: String,
    #[serde(rename = "msgsignature")]
    pub msg_signature: String,
    pub timestamp: String,
    pub nonce: String,
}

/// 生成随机 nonce
fn generate_nonce() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_nonce() {
        let nonce = generate_nonce();
        assert_eq!(nonce.len(), 16);
        assert!(nonce.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
