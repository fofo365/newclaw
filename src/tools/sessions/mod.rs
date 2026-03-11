// 会话管理工具
// 
// 提供多会话管理能力：
// - 列出活跃会话
// - 发送消息到其他会话
// - 创建/销毁子代理会话
// - 查看会话历史

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tools::{Tool, ToolMetadata};
use anyhow::Result;

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Idle,
    Closed,
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub label: Option<String>,
    pub status: SessionStatus,
    pub created_at: u64,
    pub last_active: u64,
    pub message_count: usize,
    pub metadata: HashMap<String, String>,
}

/// 会话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

/// 会话存储（简化版，实际应使用数据库）
#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: HashMap<String, SessionInfo>,
    messages: HashMap<String, Vec<SessionMessage>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建新会话
    pub fn create_session(&mut self, id: String, label: Option<String>) -> SessionInfo {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let info = SessionInfo {
            id: id.clone(),
            label,
            status: SessionStatus::Active,
            created_at: now,
            last_active: now,
            message_count: 0,
            metadata: HashMap::new(),
        };

        self.sessions.insert(id.clone(), info.clone());
        self.messages.insert(id, Vec::new());
        info
    }

    /// 获取会话信息
    pub fn get_session(&self, id: &str) -> Option<&SessionInfo> {
        self.sessions.get(id)
    }

    /// 列出所有会话
    pub fn list_sessions(&self) -> Vec<&SessionInfo> {
        self.sessions.values().collect()
    }

    /// 添加消息到会话
    pub fn add_message(&mut self, session_id: &str, role: String, content: String) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let message = SessionMessage {
            role,
            content,
            timestamp: now,
        };

        // 添加消息
        if let Some(messages) = self.messages.get_mut(session_id) {
            messages.push(message);
        }

        // 更新会话信息
        if let Some(info) = self.sessions.get_mut(session_id) {
            info.last_active = now;
            info.message_count += 1;
        }

        Ok(())
    }

    /// 获取会话历史
    pub fn get_history(&self, session_id: &str, limit: Option<usize>) -> Option<Vec<&SessionMessage>> {
        self.messages.get(session_id).map(|msgs| {
            let limit = limit.unwrap_or(50);
            msgs.iter().rev().take(limit).collect::<Vec<_>>().into_iter().rev().collect()
        })
    }

    /// 关闭会话
    pub fn close_session(&mut self, id: &str) -> Result<()> {
        if let Some(info) = self.sessions.get_mut(id) {
            info.status = SessionStatus::Closed;
        }
        Ok(())
    }

    /// 删除会话
    pub fn delete_session(&mut self, id: &str) -> Result<()> {
        self.sessions.remove(id);
        self.messages.remove(id);
        Ok(())
    }
}

/// 会话管理工具
pub struct SessionsTool {
    store: Arc<RwLock<SessionStore>>,
    metadata: ToolMetadata,
}

impl SessionsTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(SessionStore::new())),
            metadata: ToolMetadata {
                name: "sessions".to_string(),
                description: "Manage multiple conversation sessions. Actions: list, get, create, send, history, close, delete.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["list", "get", "create", "send", "history", "close", "delete"],
                            "description": "Action to perform"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Session ID (for get, send, history, close, delete)"
                        },
                        "label": {
                            "type": "string",
                            "description": "Session label (for create)"
                        },
                        "message": {
                            "type": "string",
                            "description": "Message content (for send)"
                        },
                        "role": {
                            "type": "string",
                            "description": "Message role (for send, default: assistant)"
                        },
                        "limit": {
                            "type": "number",
                            "description": "Max messages to return (for history, default: 50)"
                        }
                    },
                    "required": ["action"]
                }),
            },
        }
    }

    /// 使用现有存储创建工具
    pub fn with_store(store: Arc<RwLock<SessionStore>>) -> Self {
        Self {
            store,
            metadata: ToolMetadata {
                name: "sessions".to_string(),
                description: "Manage multiple conversation sessions.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        }
    }
}

impl Default for SessionsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SessionsTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> Result<JsonValue> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        let mut store = self.store.write().await;

        match action {
            "list" => {
                let sessions = store.list_sessions();
                Ok(serde_json::json!({
                    "success": true,
                    "sessions": sessions,
                    "count": sessions.len()
                }))
            }

            "get" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;

                match store.get_session(session_id) {
                    Some(info) => Ok(serde_json::json!({
                        "success": true,
                        "session": info
                    })),
                    None => Err(anyhow::anyhow!("Session not found: {}", session_id))
                }
            }

            "create" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

                let label = args.get("label").and_then(|v| v.as_str()).map(|s| s.to_string());

                let info = store.create_session(session_id.clone(), label);
                
                Ok(serde_json::json!({
                    "success": true,
                    "session": info
                }))
            }

            "send" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;

                let message = args.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;

                let role = args.get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("assistant")
                    .to_string();

                store.add_message(session_id, role, message.to_string())?;

                Ok(serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "message": "Message sent"
                }))
            }

            "history" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;

                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);

                match store.get_history(session_id, limit) {
                    Some(messages) => Ok(serde_json::json!({
                        "success": true,
                        "session_id": session_id,
                        "messages": messages,
                        "count": messages.len()
                    })),
                    None => Err(anyhow::anyhow!("Session not found: {}", session_id))
                }
            }

            "close" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;

                store.close_session(session_id)?;

                Ok(serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "message": "Session closed"
                }))
            }

            "delete" => {
                let session_id = args.get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: session_id"))?;

                store.delete_session(session_id)?;

                Ok(serde_json::json!({
                    "success": true,
                    "session_id": session_id,
                    "message": "Session deleted"
                }))
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sessions_tool_metadata() {
        let tool = SessionsTool::new();
        assert_eq!(tool.metadata().name, "sessions");
    }

    #[tokio::test]
    async fn test_create_session() {
        let tool = SessionsTool::new();
        
        let result = tool.execute(serde_json::json!({
            "action": "create",
            "label": "Test Session"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["session"]["id"].is_string());
        assert_eq!(result["session"]["label"], "Test Session");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let tool = SessionsTool::new();
        
        // 创建两个会话
        tool.execute(serde_json::json!({"action": "create", "label": "Session 1"})).await.unwrap();
        tool.execute(serde_json::json!({"action": "create", "label": "Session 2"})).await.unwrap();

        let result = tool.execute(serde_json::json!({"action": "list"})).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["count"], 2);
    }

    #[tokio::test]
    async fn test_send_and_history() {
        let tool = SessionsTool::new();
        
        // 创建会话
        let create_result = tool.execute(serde_json::json!({
            "action": "create"
        })).await.unwrap();
        let session_id = create_result["session"]["id"].as_str().unwrap();

        // 发送消息
        tool.execute(serde_json::json!({
            "action": "send",
            "session_id": session_id,
            "message": "Hello, world!"
        })).await.unwrap();

        // 获取历史
        let history = tool.execute(serde_json::json!({
            "action": "history",
            "session_id": session_id
        })).await.unwrap();

        assert!(history["success"].as_bool().unwrap());
        assert_eq!(history["count"], 1);
    }

    #[tokio::test]
    async fn test_close_session() {
        let tool = SessionsTool::new();
        
        let create_result = tool.execute(serde_json::json!({
            "action": "create"
        })).await.unwrap();
        let session_id = create_result["session"]["id"].as_str().unwrap();

        let result = tool.execute(serde_json::json!({
            "action": "close",
            "session_id": session_id
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());

        // 验证状态
        let get_result = tool.execute(serde_json::json!({
            "action": "get",
            "session_id": session_id
        })).await.unwrap();

        assert_eq!(get_result["session"]["status"], "Closed");
    }

    #[tokio::test]
    async fn test_delete_session() {
        let tool = SessionsTool::new();
        
        let create_result = tool.execute(serde_json::json!({
            "action": "create"
        })).await.unwrap();
        let session_id = create_result["session"]["id"].as_str().unwrap();

        let result = tool.execute(serde_json::json!({
            "action": "delete",
            "session_id": session_id
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());

        // 验证已删除
        let get_result = tool.execute(serde_json::json!({
            "action": "get",
            "session_id": session_id
        })).await;

        assert!(get_result.is_err());
    }
}
