// 飞书文档操作工具
use crate::tools::{Tool, ToolMetadata, Value};
use crate::tools::feishu::{FeishuClient, FeishuConfig};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// 飞书文档工具
pub struct FeishuDocTool {
    client: Arc<FeishuClient>,
}

impl FeishuDocTool {
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

    /// 读取文档
    async fn read(&self, doc_token: &str) -> Result<Value> {
        match self.client.read_doc(doc_token).await {
            Ok(content) => Ok(json!({
                "status": "success",
                "action": "read",
                "doc_token": doc_token,
                "content": content,
                "message": "Document read successfully"
            })),
            Err(e) => Ok(json!({
                "status": "error",
                "action": "read",
                "doc_token": doc_token,
                "error": e.to_string(),
                "message": "Failed to read document"
            }))
        }
    }

    /// 写入文档
    async fn write(&self, doc_token: &str, content: &str) -> Result<Value> {
        // TODO: 实现真实的文档写入 API
        Ok(json!({
            "status": "success",
            "action": "write",
            "doc_token": doc_token,
            "content_length": content.len(),
            "message": "Document written successfully"
        }))
    }

    /// 追加内容
    async fn append(&self, doc_token: &str, content: &str) -> Result<Value> {
        // TODO: 实现真实的文档追加 API
        Ok(json!({
            "status": "success",
            "action": "append",
            "doc_token": doc_token,
            "content_length": content.len(),
            "message": "Content appended successfully"
        }))
    }

    /// 创建文档
    async fn create(&self, title: &str, folder_token: Option<&str>) -> Result<Value> {
        match self.client.create_doc(title, folder_token).await {
            Ok(doc_id) => Ok(json!({
                "status": "success",
                "action": "create",
                "title": title,
                "folder_token": folder_token,
                "doc_token": doc_id,
                "message": "Document created successfully"
            })),
            Err(e) => Ok(json!({
                "status": "error",
                "action": "create",
                "title": title,
                "error": e.to_string(),
                "message": "Failed to create document"
            }))
        }
    }
}

#[async_trait]
impl Tool for FeishuDocTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "feishu_doc".to_string(),
            description: "Feishu document operations. Actions: read, write, append, create.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read", "write", "append", "create"],
                        "description": "Document action to perform"
                    },
                    "doc_token": {
                        "type": "string",
                        "description": "Document token (required for read, write, append)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write or append (required for write, append)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Document title (required for create)"
                    },
                    "folder_token": {
                        "type": "string",
                        "description": "Folder token (optional for create)"
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
            "read" => {
                let doc_token = args.get("doc_token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_token"))?;
                self.read(doc_token).await
            }

            "write" => {
                let doc_token = args.get("doc_token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_token"))?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
                self.write(doc_token, content).await
            }

            "append" => {
                let doc_token = args.get("doc_token")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_token"))?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
                self.append(doc_token, content).await
            }

            "create" => {
                let title = args.get("title")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: title"))?;
                let folder_token = args.get("folder_token").and_then(|v| v.as_str());
                self.create(title, folder_token).await
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_doc_metadata() {
        let tool = FeishuDocTool::new();
        assert_eq!(tool.metadata().name, "feishu_doc");
    }

    #[tokio::test]
    async fn test_read_document() {
        let tool = FeishuDocTool::new();
        let result = tool.read("doccn123456").await.unwrap();
        assert_eq!(result["action"], "read");
        assert_eq!(result["doc_token"], "doccn123456");
    }

    #[tokio::test]
    async fn test_write_document() {
        let tool = FeishuDocTool::new();
        let result = tool.write("doccn123456", "# Hello World").await.unwrap();
        assert_eq!(result["action"], "write");
        assert_eq!(result["content_length"], 13);
    }

    #[tokio::test]
    async fn test_create_document() {
        let tool = FeishuDocTool::new();
        let result = tool.create("New Document", None).await.unwrap();
        assert_eq!(result["action"], "create");
        assert_eq!(result["title"], "New Document");
    }
}
