// 节点管理工具
use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 节点类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Desktop,
    Mobile,
    Iot,
    Server,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Online,
    Offline,
    Busy,
}

/// 节点能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeCapability {
    Camera,
    Microphone,
    Screen,
    Location,
    Notifications,
    FileAccess,
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub status: NodeStatus,
    pub capabilities: Vec<NodeCapability>,
    pub last_seen: u64,
}

/// 节点存储
#[derive(Debug, Default)]
pub struct NodeStore {
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
}

impl NodeStore {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册节点
    pub async fn register(&self, name: &str, node_type: NodeType, capabilities: Vec<NodeCapability>) -> Result<NodeInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp() as u64;

        let node = NodeInfo {
            id: id.clone(),
            name: name.to_string(),
            node_type,
            status: NodeStatus::Online,
            capabilities,
            last_seen: now,
        };

        let mut nodes = self.nodes.write().await;
        nodes.insert(id, node.clone());

        Ok(node)
    }

    /// 列出节点
    pub async fn list(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// 获取节点
    pub async fn get(&self, id: &str) -> Option<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.get(id).cloned()
    }
}

/// 节点管理工具
pub struct NodesTool {
    store: Arc<NodeStore>,
}

impl NodesTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(NodeStore::new()),
        }
    }

    /// 获取节点状态
    async fn status(&self) -> Result<Value> {
        let nodes = self.store.list().await;
        Ok(json!({
            "status": "success",
            "action": "status",
            "nodes": nodes,
            "count": nodes.len()
        }))
    }

    /// 获取节点描述
    async fn describe(&self, id: &str) -> Result<Value> {
        if let Some(node) = self.store.get(id).await {
            Ok(json!({
                "status": "success",
                "action": "describe",
                "node": node
            }))
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }

    /// 发送通知
    async fn notify(&self, id: &str, title: &str, body: &str) -> Result<Value> {
        if let Some(node) = self.store.get(id).await {
            // TODO: 实现实际的通知发送
            Ok(json!({
                "status": "success",
                "action": "notify",
                "node_id": id,
                "title": title,
                "body": body,
                "node": node
            }))
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }

    /// 相机截图
    async fn camera_snap(&self, id: &str) -> Result<Value> {
        if let Some(node) = self.store.get(id).await {
            // TODO: 实现实际的相机截图
            Ok(json!({
                "status": "success",
                "action": "camera_snap",
                "node_id": id,
                "image": "base64_encoded_image_placeholder",
                "node": node
            }))
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }

    /// 屏幕录制
    async fn screen_record(&self, id: &str, duration_ms: u64) -> Result<Value> {
        if let Some(node) = self.store.get(id).await {
            // TODO: 实现实际的屏幕录制
            Ok(json!({
                "status": "success",
                "action": "screen_record",
                "node_id": id,
                "duration_ms": duration_ms,
                "video": "base64_encoded_video_placeholder",
                "node": node
            }))
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }

    /// 获取位置
    async fn location_get(&self, id: &str) -> Result<Value> {
        if let Some(node) = self.store.get(id).await {
            // TODO: 实现实际的位置获取
            Ok(json!({
                "status": "success",
                "action": "location_get",
                "node_id": id,
                "latitude": 0.0,
                "longitude": 0.0,
                "accuracy": 10.0,
                "node": node
            }))
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }

    /// 执行命令
    async fn run(&self, id: &str, command: &str) -> Result<Value> {
        if let Some(node) = self.store.get(id).await {
            // TODO: 实现实际的命令执行
            Ok(json!({
                "status": "success",
                "action": "run",
                "node_id": id,
                "command": command,
                "output": "Command executed (placeholder)",
                "node": node
            }))
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }
}

#[async_trait]
impl Tool for NodesTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "nodes".to_string(),
            description: "Discover and control paired nodes (status/describe/notify/camera/screen/location/notifications/run).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "describe", "notify", "camera_snap", "screen_record", "location_get", "run"],
                        "description": "Node action to perform"
                    },
                    "node_id": {
                        "type": "string",
                        "description": "Node ID (for all actions except status)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Notification title (for notify action)"
                    },
                    "body": {
                        "type": "string",
                        "description": "Notification body (for notify action)"
                    },
                    "duration_ms": {
                        "type": "number",
                        "description": "Recording duration in milliseconds (for screen_record action)"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to execute (for run action)"
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
            "status" => self.status().await,

            "describe" => {
                let id = args.get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node_id"))?;
                self.describe(id).await
            }

            "notify" => {
                let id = args.get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node_id"))?;
                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: title"))?;
                let body = args.get("body")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: body"))?;
                self.notify(id, title, body).await
            }

            "camera_snap" => {
                let id = args.get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node_id"))?;
                self.camera_snap(id).await
            }

            "screen_record" => {
                let id = args.get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node_id"))?;
                let duration_ms = args.get("duration_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);
                self.screen_record(id, duration_ms).await
            }

            "location_get" => {
                let id = args.get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node_id"))?;
                self.location_get(id).await
            }

            "run" => {
                let id = args.get("node_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node_id"))?;
                let command = args.get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: command"))?;
                self.run(id, command).await
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nodes_tool_metadata() {
        let tool = NodesTool::new();
        assert_eq!(tool.metadata().name, "nodes");
    }

    #[tokio::test]
    async fn test_status() {
        let tool = NodesTool::new();
        let result = tool.status().await.unwrap();
        assert_eq!(result["action"], "status");
        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_describe() {
        let store = Arc::new(NodeStore::new());
        let node = store.register("Test Node", NodeType::Desktop, vec![NodeCapability::Camera]).await.unwrap();

        let mut tool = NodesTool::new();
        tool.store = store;

        let result = tool.describe(&node.id).await.unwrap();
        assert_eq!(result["action"], "describe");
        assert_eq!(result["node"]["name"], "Test Node");
    }

    #[tokio::test]
    async fn test_notify() {
        let store = Arc::new(NodeStore::new());
        let node = store.register("Test Node", NodeType::Mobile, vec![NodeCapability::Notifications]).await.unwrap();

        let mut tool = NodesTool::new();
        tool.store = store;

        let result = tool.notify(&node.id, "Test", "Hello").await.unwrap();
        assert_eq!(result["action"], "notify");
        assert_eq!(result["title"], "Test");
    }

    #[tokio::test]
    async fn test_camera_snap() {
        let store = Arc::new(NodeStore::new());
        let node = store.register("Test Node", NodeType::Mobile, vec![NodeCapability::Camera]).await.unwrap();

        let mut tool = NodesTool::new();
        tool.store = store;

        let result = tool.camera_snap(&node.id).await.unwrap();
        assert_eq!(result["action"], "camera_snap");
    }

    #[tokio::test]
    async fn test_screen_record() {
        let store = Arc::new(NodeStore::new());
        let node = store.register("Test Node", NodeType::Desktop, vec![NodeCapability::Screen]).await.unwrap();

        let mut tool = NodesTool::new();
        tool.store = store;

        let result = tool.screen_record(&node.id, 5000).await.unwrap();
        assert_eq!(result["action"], "screen_record");
        assert_eq!(result["duration_ms"], 5000);
    }

    #[tokio::test]
    async fn test_location_get() {
        let store = Arc::new(NodeStore::new());
        let node = store.register("Test Node", NodeType::Mobile, vec![NodeCapability::Location]).await.unwrap();

        let mut tool = NodesTool::new();
        tool.store = store;

        let result = tool.location_get(&node.id).await.unwrap();
        assert_eq!(result["action"], "location_get");
    }

    #[tokio::test]
    async fn test_run() {
        let store = Arc::new(NodeStore::new());
        let node = store.register("Test Node", NodeType::Server, vec![NodeCapability::FileAccess]).await.unwrap();

        let mut tool = NodesTool::new();
        tool.store = store;

        let result = tool.run(&node.id, "ls -la").await.unwrap();
        assert_eq!(result["action"], "run");
        assert_eq!(result["command"], "ls -la");
    }
}
