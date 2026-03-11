// AGP Message

use serde::{Deserialize, Serialize};

/// 联邦域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDomain(pub String);

/// AGP 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGPMessage {
    /// 发送者 ID
    pub sender: String,
    /// 接收者 ID
    pub recipient: String,
    /// 消息内容
    pub payload: String,
    /// 回复地址
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_addr: Option<String>,
    /// 关联 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// 联邦域
    pub domain: Option<FederationDomain>,
}

impl AGPMessage {
    /// 创建新消息
    pub fn new(
        sender: String,
        recipient: String,
        payload: String,
    ) -> Self {
        Self {
            sender,
            recipient,
            payload,
            reply_addr: None,
            correlation_id: None,
            domain: None,
        }
    }

    /// 创建回复消息
    pub fn reply(&self, payload: String) -> Self {
        Self {
            sender: self.recipient.clone(),
            recipient: self.sender.clone(),
            payload,
            reply_addr: None,
            correlation_id: self.correlation_id.clone(),
            domain: self.domain.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = AGPMessage::new(
            "agent-1".to_string(),
            "agent-2".to_string(),
            "Hello".to_string(),
        );

        assert_eq!(msg.sender, "agent-1");
        assert_eq!(msg.recipient, "agent-2");
        assert_eq!(msg.payload, "Hello");
    }

    #[test]
    fn test_message_reply() {
        let msg = AGPMessage::new(
            "agent-1".to_string(),
            "agent-2".to_string(),
            "Hello".to_string(),
        );

        let reply = msg.reply("Hi there".to_string());
        assert_eq!(reply.sender, "agent-2");
        assert_eq!(reply.recipient, "agent-1");
        assert_eq!(reply.payload, "Hi there");
    }
}
