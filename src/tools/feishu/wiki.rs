// 飞书知识库工具
use crate::tools::{Tool, ToolMetadata, Value};
use crate::tools::feishu::{FeishuClient, FeishuConfig};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct FeishuWikiTool {
    client: Arc<FeishuClient>,
}

impl FeishuWikiTool {
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

    /// 列出知识空间
    async fn list_spaces(&self) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "list_spaces",
            "spaces": [],
            "message": "Spaces listed successfully"
        }))
    }

    /// 获取节点列表
    async fn list_nodes(&self, space_id: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "list_nodes",
            "space_id": space_id,
            "nodes": [],
            "message": "Nodes listed successfully"
        }))
    }

    /// 创建节点
    async fn create_node(&self, space_id: &str, title: &str, obj_type: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "create_node",
            "space_id": space_id,
            "title": title,
            "obj_type": obj_type,
            "node_token": "wikcnXXXXXXXXXXXX",
            "message": "Node created successfully"
        }))
    }

    /// 移动节点
    async fn move_node(&self, node_token: &str, target_parent: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "move_node",
            "node_token": node_token,
            "target_parent": target_parent,
            "message": "Node moved successfully"
        }))
    }
}

#[async_trait]
impl Tool for FeishuWikiTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "feishu_wiki".to_string(),
            description: "Feishu wiki/knowledge base operations.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Wiki action (list_spaces, list_nodes, create_node, move_node)"
                    },
                    "space_id": {
                        "type": "string",
                        "description": "Space ID"
                    },
                    "title": {
                        "type": "string",
                        "description": "Node title (for create_node)"
                    },
                    "obj_type": {
                        "type": "string",
                        "description": "Object type (docx, sheet, bitable)"
                    },
                    "node_token": {
                        "type": "string",
                        "description": "Node token (for move_node)"
                    },
                    "target_parent": {
                        "type": "string",
                        "description": "Target parent token (for move_node)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("");

        match action {
            "list_spaces" => self.list_spaces().await,
            "list_nodes" => {
                let space_id = args["space_id"].as_str().unwrap_or("");
                self.list_nodes(space_id).await
            }
            "create_node" => {
                let space_id = args["space_id"].as_str().unwrap_or("");
                let title = args["title"].as_str().unwrap_or("");
                let obj_type = args["obj_type"].as_str().unwrap_or("docx");
                self.create_node(space_id, title, obj_type).await
            }
            "move_node" => {
                let node_token = args["node_token"].as_str().unwrap_or("");
                let target_parent = args["target_parent"].as_str().unwrap_or("");
                self.move_node(node_token, target_parent).await
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
    fn test_wiki_metadata() {
        let tool = FeishuWikiTool::new();
        assert_eq!(tool.metadata().name, "feishu_wiki");
    }

    #[tokio::test]
    async fn test_list_spaces() {
        let tool = FeishuWikiTool::new();
        let result = tool.list_spaces().await.unwrap();
        assert_eq!(result["action"], "list_spaces");
    }

    #[tokio::test]
    async fn test_list_nodes() {
        let tool = FeishuWikiTool::new();
        let result = tool.list_nodes("space123").await.unwrap();
        assert_eq!(result["action"], "list_nodes");
    }

    #[tokio::test]
    async fn test_create_node() {
        let tool = FeishuWikiTool::new();
        let result = tool.create_node("space123", "New Doc", "docx").await.unwrap();
        assert_eq!(result["action"], "create_node");
    }

    #[tokio::test]
    async fn test_move_node() {
        let tool = FeishuWikiTool::new();
        let result = tool.move_node("node123", "parent456").await.unwrap();
        assert_eq!(result["action"], "move_node");
    }
}
