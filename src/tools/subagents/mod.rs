// 子代理管理工具
use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 子代理信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentInfo {
    pub id: String,
    pub label: String,
    pub task: String,
    pub status: String,
    pub created_at: u64,
    pub last_active: u64,
}

/// 子代理存储
#[derive(Debug, Default)]
pub struct SubagentStore {
    subagents: Arc<RwLock<HashMap<String, SubagentInfo>>>,
}

impl SubagentStore {
    pub fn new() -> Self {
        Self {
            subagents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建子代理
    pub async fn create(&self, label: &str, task: &str) -> Result<SubagentInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp() as u64;

        let subagent = SubagentInfo {
            id: id.clone(),
            label: label.to_string(),
            task: task.to_string(),
            status: "running".to_string(),
            created_at: now,
            last_active: now,
        };

        let mut subagents = self.subagents.write().await;
        subagents.insert(id, subagent.clone());

        Ok(subagent)
    }

    /// 列出子代理
    pub async fn list(&self, recent_minutes: Option<u64>) -> Vec<SubagentInfo> {
        let subagents = self.subagents.read().await;
        let now = chrono::Utc::now().timestamp() as u64;
        let threshold = recent_minutes.map(|m| now - m * 60);

        subagents.values()
            .filter(|s| {
                threshold.is_none_or(|t| s.last_active >= t)
            })
            .cloned()
            .collect()
    }

    /// 获取子代理
    pub async fn get(&self, id: &str) -> Option<SubagentInfo> {
        let subagents = self.subagents.read().await;
        subagents.get(id).cloned()
    }

    /// 更新子代理状态
    pub async fn update_status(&self, id: &str, status: &str) -> Result<()> {
        let mut subagents = self.subagents.write().await;
        if let Some(subagent) = subagents.get_mut(id) {
            subagent.status = status.to_string();
            subagent.last_active = chrono::Utc::now().timestamp() as u64;
        }
        Ok(())
    }

    /// 删除子代理
    pub async fn delete(&self, id: &str) -> Result<()> {
        let mut subagents = self.subagents.write().await;
        subagents.remove(id);
        Ok(())
    }
}

/// 子代理管理工具
pub struct SubagentsTool {
    store: Arc<SubagentStore>,
}

impl Default for SubagentsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SubagentsTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(SubagentStore::new()),
        }
    }

    /// 列出子代理
    async fn list(&self, recent_minutes: Option<u64>) -> Result<Value> {
        let subagents = self.store.list(recent_minutes).await;
        Ok(json!({
            "status": "success",
            "action": "list",
            "subagents": subagents,
            "count": subagents.len()
        }))
    }

    /// 引导子代理
    async fn steer(&self, id: &str, message: &str) -> Result<Value> {
        if let Some(subagent) = self.store.get(id).await {
            // TODO: 实现实际的引导逻辑
            Ok(json!({
                "status": "success",
                "action": "steer",
                "subagent_id": id,
                "message": message,
                "subagent": subagent
            }))
        } else {
            Err(anyhow::anyhow!("Subagent not found: {}", id))
        }
    }

    /// 终止子代理
    async fn kill(&self, id: &str) -> Result<Value> {
        if let Some(subagent) = self.store.get(id).await {
            self.store.update_status(id, "terminated").await?;
            Ok(json!({
                "status": "success",
                "action": "kill",
                "subagent_id": id,
                "subagent": subagent
            }))
        } else {
            Err(anyhow::anyhow!("Subagent not found: {}", id))
        }
    }
}

#[async_trait]
impl Tool for SubagentsTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "subagents".to_string(),
            description: "Manage spawned sub-agents: list, steer, kill.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "steer", "kill"],
                        "description": "Subagent action to perform"
                    },
                    "subagent_id": {
                        "type": "string",
                        "description": "Subagent ID (for steer and kill actions)"
                    },
                    "message": {
                        "type": "string",
                        "description": "Steering message (for steer action)"
                    },
                    "recent_minutes": {
                        "type": "number",
                        "description": "Filter by recent activity in minutes (for list action)"
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
            "list" => {
                let recent_minutes = args.get("recent_minutes").and_then(|v| v.as_u64());
                self.list(recent_minutes).await
            }

            "steer" => {
                let id = args.get("subagent_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: subagent_id"))?;
                let message = args.get("message")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: message"))?;
                self.steer(id, message).await
            }

            "kill" => {
                let id = args.get("subagent_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: subagent_id"))?;
                self.kill(id).await
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagents_tool_metadata() {
        let tool = SubagentsTool::new();
        assert_eq!(tool.metadata().name, "subagents");
    }

    #[tokio::test]
    async fn test_list_subagents() {
        let tool = SubagentsTool::new();
        let result = tool.list(None).await.unwrap();
        assert_eq!(result["action"], "list");
        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_steer_subagent() {
        let store = Arc::new(SubagentStore::new());
        let subagent = store.create("Test", "Do something").await.unwrap();

        let mut tool = SubagentsTool::new();
        tool.store = store;

        let result = tool.steer(&subagent.id, "New instruction").await.unwrap();
        assert_eq!(result["action"], "steer");
        assert_eq!(result["message"], "New instruction");
    }

    #[tokio::test]
    async fn test_kill_subagent() {
        let store = Arc::new(SubagentStore::new());
        let subagent = store.create("Test", "Do something").await.unwrap();

        let mut tool = SubagentsTool::new();
        tool.store = store;

        let result = tool.kill(&subagent.id).await.unwrap();
        assert_eq!(result["action"], "kill");
    }
}
