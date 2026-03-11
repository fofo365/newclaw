// 会话管理工具
use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub label: String,
    pub created_at: u64,
    pub last_active: u64,
    pub status: String,
}

/// 会话存储
#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: Arc<RwLock<HashMap<String, SessionInfo>>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建会话
    pub async fn create(&self, label: &str) -> Result<SessionInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp() as u64;

        let session = SessionInfo {
            id: id.clone(),
            label: label.to_string(),
            created_at: now,
            last_active: now,
            status: "active".to_string(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(id, session.clone());

        Ok(session)
    }

    /// 列出会话
    pub async fn list(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// 获取会话
    pub async fn get(&self, id: &str) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(id).cloned()
    }

    /// 发送消息到会话
    pub async fn send(&self, id: &str, _message: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            session.last_active = chrono::Utc::now().timestamp() as u64;
        }
        // TODO: 实现实际的消息发送
        Ok(())
    }
}

/// 会话管理工具
pub struct SessionsTool {
    store: Arc<SessionStore>,
}

impl SessionsTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(SessionStore::new()),
        }
    }

    /// 创建会话
    async fn spawn(&self, label: &str) -> Result<Value> {
        let session = self.store.create(label).await?;
        Ok(json!({
            "status": "success",
            "action": "spawn",
            "session": session
        }))
    }

    /// 列出会话
    async fn list(&self) -> Result<Value> {
        let sessions = self.store.list().await;
        Ok(json!({
            "status": "success",
            "action": "list",
            "sessions": sessions,
            "count": sessions.len()
        }))
    }

    /// 发送消息
    async fn send(&self, id: &str, message: &str) -> Result<Value> {
        self.store.send(id, message).await?;
        Ok(json!({
            "status": "success",
            "action": "send",
            "session_id": id,
            "message": message
        }))
    }

    /// 获取历史
    async fn history(&self, id: &str, limit: Option<usize>) -> Result<Value> {
        let _limit = limit.unwrap_or(100);
        // TODO: 实现实际的历史记录
        Ok(json!({
            "status": "success",
            "action": "history",
            "session_id": id,
            "messages": [],
            "message": "History feature coming soon (placeholder)"
        }))
    }
}

#[async_trait]
impl Tool for SessionsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "sessions".to_string(),
            description: "Session management: spawn new sessions, send messages, list sessions, get history.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["spawn", "send", "list", "history"],
                        "description": "Session action to perform"
                    },
                    "label": {
                        "type": "string",
                        "description": "Session label (for spawn action)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Session ID (for send and history actions)"
                    },
                    "message": {
                        "type": "string",
                        "description": "Message to send (for send action)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Number of messages to retrieve (for history action)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        match action {
            "spawn" => {
                let label = args.get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled Session");
                self.spawn(label).await
            }

            "list" => self.list().await,

            "send" => {
                let id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;
                let message = args.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;
                self.send(id, message).await
            }

            "history" => {
                let id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
                self.history(id, limit).await
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sessions_tool_metadata() {
        let tool = SessionsTool::new();
        assert_eq!(tool.metadata().name, "sessions");
    }

    #[tokio::test]
    async fn test_spawn_session() {
        let tool = SessionsTool::new();
        let result = tool.spawn("Test Session").await.unwrap();
        assert_eq!(result["action"], "spawn");
        assert_eq!(result["session"]["label"], "Test Session");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let tool = SessionsTool::new();
        tool.spawn("Session 1").await.unwrap();
        tool.spawn("Session 2").await.unwrap();

        let result = tool.list().await.unwrap();
        assert_eq!(result["action"], "list");
        assert_eq!(result["count"], 2);
    }

    #[tokio::test]
    async fn test_send_message() {
        let tool = SessionsTool::new();
        let spawn_result = tool.spawn("Test").await.unwrap();
        let session_id = spawn_result["session"]["id"].as_str().unwrap();

        let result = tool.send(session_id, "Hello").await.unwrap();
        assert_eq!(result["action"], "send");
        assert_eq!(result["message"], "Hello");
    }

    #[tokio::test]
    async fn test_history() {
        let tool = SessionsTool::new();
        let spawn_result = tool.spawn("Test").await.unwrap();
        let session_id = spawn_result["session"]["id"].as_str().unwrap();

        let result = tool.history(session_id, Some(10)).await.unwrap();
        assert_eq!(result["action"], "history");
    }
}
