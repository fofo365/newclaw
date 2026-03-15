// 飞书云存储工具
use crate::tools::{Tool, ToolMetadata, Value};
use crate::tools::feishu::{FeishuClient, FeishuConfig};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct FeishuDriveTool {
    client: Arc<FeishuClient>,
}

impl Default for FeishuDriveTool {
    fn default() -> Self {
        Self::new()
    }
}

impl FeishuDriveTool {
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

    /// 列出文件/文件夹
    async fn list(&self, folder_token: Option<&str>) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "list",
            "folder_token": folder_token,
            "files": [],
            "message": "Files listed successfully"
        }))
    }

    /// 创建文件夹
    async fn create_folder(&self, name: &str, parent_token: Option<&str>) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "create_folder",
            "name": name,
            "parent_token": parent_token,
            "folder_token": "fldcnXXXXXXXXXXXX",
            "message": "Folder created successfully"
        }))
    }

    /// 移动文件/文件夹
    async fn move_item(&self, file_token: &str, target_folder: &str) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "move",
            "file_token": file_token,
            "target_folder": target_folder,
            "message": "Item moved successfully"
        }))
    }

    /// 删除文件/文件夹
    async fn delete(&self, file_token: &str) -> Result<Value> {
        // TODO: 实现真实的 API 调用
        Ok(json!({
            "status": "success",
            "action": "delete",
            "file_token": file_token,
            "message": "Item deleted successfully"
        }))
    }
}

#[async_trait]
impl Tool for FeishuDriveTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "feishu_drive".to_string(),
            description: "Feishu cloud storage operations.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Drive action (list, create_folder, move, delete)"
                    },
                    "folder_token": {
                        "type": "string",
                        "description": "Folder token (for list)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Folder name (for create_folder)"
                    },
                    "parent_token": {
                        "type": "string",
                        "description": "Parent folder token (for create_folder)"
                    },
                    "file_token": {
                        "type": "string",
                        "description": "File/folder token (for move/delete)"
                    },
                    "target_folder": {
                        "type": "string",
                        "description": "Target folder token (for move)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args["action"].as_str().unwrap_or("");

        match action {
            "list" => {
                let folder_token = args["folder_token"].as_str();
                self.list(folder_token).await
            }
            "create_folder" => {
                let name = args["name"].as_str().unwrap_or("");
                let parent_token = args["parent_token"].as_str();
                self.create_folder(name, parent_token).await
            }
            "move" => {
                let file_token = args["file_token"].as_str().unwrap_or("");
                let target_folder = args["target_folder"].as_str().unwrap_or("");
                self.move_item(file_token, target_folder).await
            }
            "delete" => {
                let file_token = args["file_token"].as_str().unwrap_or("");
                self.delete(file_token).await
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
    fn test_drive_metadata() {
        let tool = FeishuDriveTool::new();
        assert_eq!(tool.metadata().name, "feishu_drive");
    }

    #[tokio::test]
    async fn test_list() {
        let tool = FeishuDriveTool::new();
        let result = tool.list(Some("folder123")).await.unwrap();
        assert_eq!(result["action"], "list");
    }

    #[tokio::test]
    async fn test_create_folder() {
        let tool = FeishuDriveTool::new();
        let result = tool.create_folder("New Folder", Some("parent123")).await.unwrap();
        assert_eq!(result["action"], "create_folder");
    }

    #[tokio::test]
    async fn test_move() {
        let tool = FeishuDriveTool::new();
        let result = tool.move_item("file123", "folder456").await.unwrap();
        assert_eq!(result["action"], "move");
    }

    #[tokio::test]
    async fn test_delete() {
        let tool = FeishuDriveTool::new();
        let result = tool.delete("file123").await.unwrap();
        assert_eq!(result["action"], "delete");
    }
}
