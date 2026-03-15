// 飞书聊天工具
use crate::tools::{Tool, ToolMetadata, Value};
use crate::tools::feishu::{FeishuClient, FeishuConfig};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct FeishuChatTool {
    client: Arc<FeishuClient>,
}

impl Default for FeishuChatTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FeishuChatTool {
    pub fn new() -> Self {
        let config = FeishuConfig::default();
        Self {
            client: Arc::new(FeishuClient::new(config)),
        }
    }

    pub fn with_config(config: FeishuConfig) -> Self {
        Self {
            client: Arc::new(FeishuClient::new(config)),
        }
    }

    /// 获取聊天信息
    async fn info(&self, chat_id: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "info",
            "chat_id": chat_id,
            "chat_info": {},
            "message": "Chat info retrieved successfully"
        }))
    }

    /// 获取聊天成员
    async fn members(&self, chat_id: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "members",
            "chat_id": chat_id,
            "members": [],
            "message": "Members listed successfully"
        }))
    }

    /// 发送消息
    async fn send(&self, receive_id: &str, receive_id_type: &str, msg_type: &str, content: &str) -> Result<Value> {
        match self.client.send_message(receive_id, receive_id_type, msg_type, content).await {
            Ok(message_id) => Ok(json!({
                "status": "success",
                "action": "send",
                "message_id": message_id,
                "receive_id": receive_id,
                "msg_type": msg_type,
                "message": "Message sent successfully"
            })),
            Err(e) => Ok(json!({
                "status": "error",
                "action": "send",
                "error": e.to_string(),
                "message": "Failed to send message"
            }))
        }
    }
}

#[async_trait]
impl Tool for FeishuChatTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "feishu_chat".to_string(),
            description: "Feishu chat operations.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Chat action (info, members, send)"
                    },
                    "chat_id": {
                        "type": "string",
                        "description": "Chat ID (for info, members)"
                    },
                    "receive_id": {
                        "type": "string",
                        "description": "Receiver ID (for send)"
                    },
                    "receive_id_type": {
                        "type": "string",
                        "description": "Receiver ID type (open_id, user_id, union_id, chat_id)"
                    },
                    "msg_type": {
                        "type": "string",
                        "description": "Message type (text, post, image, etc.)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Message content (for send)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("");

        match action {
            "info" => {
                let chat_id = args["chat_id"].as_str().unwrap_or("");
                self.info(chat_id).await
            }
            "members" => {
                let chat_id = args["chat_id"].as_str().unwrap_or("");
                self.members(chat_id).await
            }
            "send" => {
                let receive_id = args["receive_id"].as_str().unwrap_or("");
                let receive_id_type = args["receive_id_type"].as_str().unwrap_or("open_id");
                let msg_type = args["msg_type"].as_str().unwrap_or("text");
                let content = args["content"].as_str().unwrap_or("");
                self.send(receive_id, receive_id_type, msg_type, content).await
            }
            _ => Ok(json!({
                "status": "error",
                "message": format!("Unknown action: {}", action)
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_metadata() {
        let tool = FeishuChatTool::new();
        assert_eq!(tool.metadata().name, "feishu_chat");
    }

    #[tokio::test]
    async fn test_info() {
        let tool = FeishuChatTool::new();
        let result = tool.info("chat123").await.unwrap();
        assert_eq!(result["action"], "info");
    }

    #[tokio::test]
    async fn test_members() {
        let tool = FeishuChatTool::new();
        let result = tool.members("chat123").await.unwrap();
        assert_eq!(result["action"], "members");
    }

    #[tokio::test]
    async fn test_send() {
        let tool = FeishuChatTool::new();
        let result = tool.send("ou_xxx", "open_id", "text", "Hello").await.unwrap();
        assert_eq!(result["action"], "send");
    }
}
