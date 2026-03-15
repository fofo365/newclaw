// NewClaw v0.4.0 - 企业微信（WeCom）通道集成
//
// 核心功能：
// 1. AccessToken 管理（自动缓存、刷新）
// 2. 消息发送（文本、图片、文件、视频）
// 3. Webhook 消息接收和处理
// 4. 消息加密/解密（AES-256-CBC）
// 5. 媒体文件上传/下载

pub mod types;
pub mod crypto;
pub mod client;
pub mod message;
pub mod webhook;

// Re-exports
pub use types::*;
pub use crypto::{WeComCrypto, decrypt_message, encrypt_message};
pub use client::WeComClient;
pub use message::{WeComMessageClient, chunk_text};
pub use webhook::{WeComWebhook, EncryptedReply, UrlVerifyRequest};

/// 创建 WeCom 客户端
pub fn create_client(config: WeComConfig) -> WeComClient {
    WeComClient::new(config)
}

/// 创建消息客户端
pub fn create_message_client(config: WeComConfig) -> WeComMessageClient {
    WeComMessageClient::from_config(config)
}

/// 创建 Webhook 处理器
pub fn create_webhook(config: &WeComConfig) -> anyhow::Result<WeComWebhook> {
    WeComWebhook::from_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_client() {
        let config = WeComConfig {
            corp_id: "test_corp".to_string(),
            corp_secret: "test_secret".to_string(),
            agent_id: "1000001".to_string(),
            token: None,
            encoding_aes_key: None,
            receive_id: None,
        };
        
        let client = create_client(config);
        assert_eq!(client.config().corp_id, "test_corp");
    }
}
