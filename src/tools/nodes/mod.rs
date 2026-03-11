// 节点管理工具
//
// 提供节点（设备）管理能力：
// - 发现和列出配对节点
// - 获取节点状态和描述
// - 发送通知
// - 相机/屏幕/位置等功能

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tools::{Tool, ToolMetadata};
use anyhow::Result;

/// 节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Desktop,
    Mobile,
    Server,
    IoT,
    Unknown,
}

/// 节点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Online,
    Offline,
    Busy,
    Unknown,
}

/// 节点能力
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum NodeCapability {
    Camera,
    Screen,
    Location,
    Notifications,
    Photos,
    RunCommands,
    Invoke,
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
    pub metadata: HashMap<String, String>,
}

/// 节点存储
#[derive(Debug, Default)]
pub struct NodeStore {
    nodes: HashMap<String, NodeInfo>,
}

impl NodeStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册节点
    pub fn register(&mut self, id: String, name: String, node_type: NodeType, capabilities: Vec<NodeCapability>) -> NodeInfo {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let info = NodeInfo {
            id: id.clone(),
            name,
            node_type,
            status: NodeStatus::Online,
            capabilities,
            last_seen: now,
            metadata: HashMap::new(),
        };

        self.nodes.insert(id, info.clone());
        info
    }

    /// 获取节点
    pub fn get(&self, id: &str) -> Option<&NodeInfo> {
        self.nodes.get(id)
    }

    /// 列出所有节点
    pub fn list(&self) -> Vec<&NodeInfo> {
        self.nodes.values().collect()
    }

    /// 列出在线节点
    pub fn list_online(&self) -> Vec<&NodeInfo> {
        self.nodes.values()
            .filter(|n| n.status == NodeStatus::Online)
            .collect()
    }

    /// 更新节点状态
    pub fn update_status(&mut self, id: &str, status: NodeStatus) -> Result<()> {
        if let Some(node) = self.nodes.get_mut(id) {
            node.status = status;
            node.last_seen = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }

    /// 注销节点
    pub fn unregister(&mut self, id: &str) -> Result<()> {
        if self.nodes.remove(id).is_some() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Node not found: {}", id))
        }
    }
}

/// 节点管理工具
pub struct NodesTool {
    store: Arc<RwLock<NodeStore>>,
    metadata: ToolMetadata,
}

impl NodesTool {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(NodeStore::new())),
            metadata: ToolMetadata {
                name: "nodes".to_string(),
                description: "Discover and control paired nodes (devices). Actions: status, describe, notify, camera, screen, location, photos, run, invoke.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["status", "describe", "list", "notify", "camera_snap", "screen_record", "location_get", "run", "invoke"],
                            "description": "Action to perform"
                        },
                        "node": {
                            "type": "string",
                            "description": "Node ID or name"
                        },
                        "title": {
                            "type": "string",
                            "description": "Notification title (for notify)"
                        },
                        "body": {
                            "type": "string",
                            "description": "Notification body (for notify)"
                        },
                        "facing": {
                            "type": "string",
                            "enum": ["front", "back", "both"],
                            "description": "Camera facing (for camera_snap)"
                        },
                        "command": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Command to run (for run)"
                        },
                        "invoke_command": {
                            "type": "string",
                            "description": "Invoke command name (for invoke)"
                        },
                        "invoke_params": {
                            "type": "object",
                            "description": "Invoke parameters (for invoke)"
                        }
                    },
                    "required": ["action"]
                }),
            },
        }
    }

    /// 使用现有存储创建工具
    pub fn with_store(store: Arc<RwLock<NodeStore>>) -> Self {
        Self {
            store,
            metadata: ToolMetadata {
                name: "nodes".to_string(),
                description: "Manage paired nodes.".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        }
    }
}

impl Default for NodesTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for NodesTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> Result<JsonValue> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        let mut store = self.store.write().await;

        match action {
            "status" | "list" => {
                let nodes = store.list();
                let online = store.list_online();
                
                Ok(serde_json::json!({
                    "success": true,
                    "total_nodes": nodes.len(),
                    "online_nodes": online.len(),
                    "nodes": nodes
                }))
            }

            "describe" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                match store.get(node_id) {
                    Some(info) => Ok(serde_json::json!({
                        "success": true,
                        "node": info
                    })),
                    None => Err(anyhow::anyhow!("Node not found: {}", node_id))
                }
            }

            "notify" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Notification");

                let body = args.get("body")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: body"))?;

                // 检查节点是否存在
                if store.get(node_id).is_none() {
                    return Err(anyhow::anyhow!("Node not found: {}", node_id));
                }

                // 模拟发送通知
                Ok(serde_json::json!({
                    "success": true,
                    "node": node_id,
                    "title": title,
                    "body": body,
                    "message": "Notification sent"
                }))
            }

            "camera_snap" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                let facing = args.get("facing")
                    .and_then(|v| v.as_str())
                    .unwrap_or("back");

                // 检查节点和能力
                match store.get(node_id) {
                    Some(info) => {
                        if !info.capabilities.contains(&NodeCapability::Camera) {
                            return Err(anyhow::anyhow!("Node does not have camera capability"));
                        }

                        // 模拟拍照
                        Ok(serde_json::json!({
                            "success": true,
                            "node": node_id,
                            "facing": facing,
                            "image": "base64_mock_image_data",
                            "message": "Photo captured"
                        }))
                    }
                    None => Err(anyhow::anyhow!("Node not found: {}", node_id))
                }
            }

            "screen_record" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                // 检查节点和能力
                match store.get(node_id) {
                    Some(info) => {
                        if !info.capabilities.contains(&NodeCapability::Screen) {
                            return Err(anyhow::anyhow!("Node does not have screen recording capability"));
                        }

                        // 模拟屏幕录制
                        Ok(serde_json::json!({
                            "success": true,
                            "node": node_id,
                            "video": "base64_mock_video_data",
                            "message": "Screen recorded"
                        }))
                    }
                    None => Err(anyhow::anyhow!("Node not found: {}", node_id))
                }
            }

            "location_get" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                // 检查节点和能力
                match store.get(node_id) {
                    Some(info) => {
                        if !info.capabilities.contains(&NodeCapability::Location) {
                            return Err(anyhow::anyhow!("Node does not have location capability"));
                        }

                        // 模拟位置获取
                        Ok(serde_json::json!({
                            "success": true,
                            "node": node_id,
                            "latitude": 39.9042,
                            "longitude": 116.4074,
                            "accuracy": 10.0,
                            "message": "Location retrieved"
                        }))
                    }
                    None => Err(anyhow::anyhow!("Node not found: {}", node_id))
                }
            }

            "run" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                let command = args.get("command")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: command"))?;

                // 检查节点和能力
                match store.get(node_id) {
                    Some(info) => {
                        if !info.capabilities.contains(&NodeCapability::RunCommands) {
                            return Err(anyhow::anyhow!("Node does not have run command capability"));
                        }

                        // 模拟命令执行
                        Ok(serde_json::json!({
                            "success": true,
                            "node": node_id,
                            "command": command,
                            "output": "Mock command output",
                            "exit_code": 0
                        }))
                    }
                    None => Err(anyhow::anyhow!("Node not found: {}", node_id))
                }
            }

            "invoke" => {
                let node_id = args.get("node")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: node"))?;

                let invoke_command = args.get("invoke_command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: invoke_command"))?;

                let invoke_params = args.get("invoke_params").cloned().unwrap_or(JsonValue::Null);

                // 检查节点和能力
                match store.get(node_id) {
                    Some(info) => {
                        if !info.capabilities.contains(&NodeCapability::Invoke) {
                            return Err(anyhow::anyhow!("Node does not have invoke capability"));
                        }

                        // 模拟调用
                        Ok(serde_json::json!({
                            "success": true,
                            "node": node_id,
                            "command": invoke_command,
                            "params": invoke_params,
                            "result": "Mock invoke result"
                        }))
                    }
                    None => Err(anyhow::anyhow!("Node not found: {}", node_id))
                }
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nodes_tool_metadata() {
        let tool = NodesTool::new();
        assert_eq!(tool.metadata().name, "nodes");
    }

    #[tokio::test]
    async fn test_list_nodes() {
        let tool = NodesTool::new();
        
        // 先注册一个节点
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-1".to_string(),
                "Test Node".to_string(),
                NodeType::Desktop,
                vec![NodeCapability::Camera, NodeCapability::Screen],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "list"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["total_nodes"], 1);
    }

    #[tokio::test]
    async fn test_describe_node() {
        let tool = NodesTool::new();
        
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-2".to_string(),
                "Test Node 2".to_string(),
                NodeType::Mobile,
                vec![NodeCapability::Camera, NodeCapability::Location],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "describe",
            "node": "test-node-2"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["node"]["name"], "Test Node 2");
        assert_eq!(result["node"]["node_type"], "Mobile");
    }

    #[tokio::test]
    async fn test_notify() {
        let tool = NodesTool::new();
        
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-3".to_string(),
                "Test Node 3".to_string(),
                NodeType::Mobile,
                vec![NodeCapability::Notifications],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "notify",
            "node": "test-node-3",
            "title": "Test",
            "body": "Hello from NewClaw!"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["title"], "Test");
    }

    #[tokio::test]
    async fn test_camera_snap() {
        let tool = NodesTool::new();
        
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-4".to_string(),
                "Test Node 4".to_string(),
                NodeType::Mobile,
                vec![NodeCapability::Camera],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "camera_snap",
            "node": "test-node-4",
            "facing": "back"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["image"].is_string());
    }

    #[tokio::test]
    async fn test_camera_no_capability() {
        let tool = NodesTool::new();
        
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-5".to_string(),
                "Server Node".to_string(),
                NodeType::Server,
                vec![NodeCapability::RunCommands],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "camera_snap",
            "node": "test-node-5"
        })).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_location_get() {
        let tool = NodesTool::new();
        
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-6".to_string(),
                "Mobile Node".to_string(),
                NodeType::Mobile,
                vec![NodeCapability::Location],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "location_get",
            "node": "test-node-6"
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["latitude"].is_number());
        assert!(result["longitude"].is_number());
    }

    #[tokio::test]
    async fn test_run_command() {
        let tool = NodesTool::new();
        
        {
            let mut store = tool.store.write().await;
            store.register(
                "test-node-7".to_string(),
                "Desktop Node".to_string(),
                NodeType::Desktop,
                vec![NodeCapability::RunCommands],
            );
        }

        let result = tool.execute(serde_json::json!({
            "action": "run",
            "node": "test-node-7",
            "command": ["ls", "-la"]
        })).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["exit_code"], 0);
    }

    #[tokio::test]
    async fn test_node_not_found() {
        let tool = NodesTool::new();

        let result = tool.execute(serde_json::json!({
            "action": "describe",
            "node": "nonexistent"
        })).await;

        assert!(result.is_err());
    }
}
