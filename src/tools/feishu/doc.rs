// 飞书文档操作工具
// 增强版本：支持 fetch、update 等高级功能
use crate::tools::{Tool, ToolMetadata, Value};
use crate::tools::feishu::{FeishuClient, FeishuConfig};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use std::sync::Arc;

/// 飞书文档工具（增强版）
pub struct FeishuDocTool {
    client: Arc<FeishuClient>,
}

impl Default for FeishuDocTool {
    fn default() -> Self {
        Self::new()
    }
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

    /// 从 URL 提取文档 token
    fn extract_token_from_url(url: &str) -> Result<String> {
        // 支持完整 URL: https://xxx.feishu.cn/docx/TOKEN 或 https://xxx.feishu.cn/wiki/TOKEN
        if url.contains("/docx/") {
            if let Some(pos) = url.rfind("/docx/") {
                let start = pos + 6;
                let token = &url[start..];
                // 去除查询参数
                if let Some(q) = token.find('?') {
                    return Ok(token[..q].to_string());
                }
                return Ok(token.to_string());
            }
        } else if url.contains("/wiki/") {
            if let Some(pos) = url.rfind("/wiki/") {
                let start = pos + 6;
                let token = &url[start..];
                if let Some(q) = token.find('?') {
                    return Ok(token[..q].to_string());
                }
                return Ok(token.to_string());
            }
        }
        // 直接返回（假设已经是 token）
        Ok(url.to_string())
    }

    /// Fetch: 获取文档内容（支持分页）
    async fn fetch(&self, doc_id: &str, offset: Option<usize>, limit: Option<usize>) -> Result<Value> {
        let token = Self::extract_token_from_url(doc_id)?;
        
        match self.client.read_doc(&token).await {
            Ok(content) => {
                let total_length = content.len();
                let offset_val = offset.unwrap_or(0);
                let limit_val = limit.unwrap_or(total_length);
                
                let content_part = if offset_val < total_length {
                    let end = std::cmp::min(offset_val + limit_val, total_length);
                    content.chars().skip(offset_val).take(end - offset_val).collect::<String>()
                } else {
                    String::new()
                };
                
                let title = self.client.get_doc_title(&token).await.unwrap_or_else(|_| "Unknown".to_string());
                
                Ok(json!({
                    "doc_id": token,
                    "title": title,
                    "markdown": content_part,
                    "offset": offset_val,
                    "limit": limit_val,
                    "total_length": total_length,
                    "message": "Document fetched successfully"
                }))
            },
            Err(e) => Ok(json!({
                "doc_id": token,
                "error": e.to_string(),
                "message": "Failed to fetch document",
                "status": "error"
            }))
        }
    }

    /// Update: 更新文档（支持多种模式）
    async fn update(&self, args: &serde_json::Value) -> Result<Value> {
        let doc_id = args.get("doc_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_id"))?;
        
        let token = Self::extract_token_from_url(doc_id)?;
        let mode = args.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("overwrite");
        
        let markdown = args.get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let new_title = args.get("new_title")
            .and_then(|v| v.as_str());
        
        let task_id = args.get("task_id")
            .and_then(|v| v.as_str());
        
        // 如果有 task_id，查询任务状态
        if let Some(tid) = task_id {
            return self.check_task_status(tid).await;
        }
        
        // 根据模式执行更新
        match mode {
            "overwrite" => {
                self.client.update_doc(&token, markdown, None).await
            },
            "append" => {
                self.client.update_doc(&token, markdown, Some("append".to_string())).await
            },
            "replace_range" => {
                let selection = args.get("selection_with_ellipsis")
                    .and_then(|v| v.as_str())
                    .or_else(|| args.get("selection_by_title").and_then(|v| v.as_str()))
                    .ok_or_else(|| anyhow::anyhow!("Missing selection for replace_range"))?;
                self.client.update_doc_with_selection(&token, markdown, mode, selection).await
            },
            "replace_all" => {
                // 先读取全文，替换所有匹配项
                let old_content = self.client.read_doc(&token).await?;
                let new_content = markdown.replace(&old_content, "");
                self.client.update_doc(&token, &new_content, None).await
            },
            "insert_before" => {
                let selection = args.get("selection_with_ellipsis")
                    .and_then(|v| v.as_str())
                    .or_else(|| args.get("selection_by_title").and_then(|v| v.as_str()))
                    .ok_or_else(|| anyhow::anyhow!("Missing selection for insert_before"))?;
                self.client.update_doc_with_selection(&token, markdown, mode, selection).await
            },
            "insert_after" => {
                let selection = args.get("selection_with_ellipsis")
                    .and_then(|v| v.as_str())
                    .or_else(|| args.get("selection_by_title").and_then(|v| v.as_str()))
                    .ok_or_else(|| anyhow::anyhow!("Missing selection for insert_after"))?;
                self.client.update_doc_with_selection(&token, markdown, mode, selection).await
            },
            "delete_range" => {
                let selection = args.get("selection_with_ellipsis")
                    .and_then(|v| v.as_str())
                    .or_else(|| args.get("selection_by_title").and_then(|v| v.as_str()))
                    .ok_or_else(|| anyhow::anyhow!("Missing selection for delete_range"))?;
                self.client.update_doc_with_selection(&token, markdown, mode, selection).await
            },
            _ => Ok(json!({
                "status": "error",
                "message": format!("Unknown update mode: {}", mode)
            }))
        }
    }

    /// Create: 创建文档（从 Markdown）
    async fn create(&self, args: &serde_json::Value) -> Result<Value> {
        let title = args.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: title"))?;
        
        let markdown = args.get("markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        let folder_token = args.get("folder_token")
            .or_else(|| args.get("wiki_node"))
            .and_then(|v| v.as_str());
        
        let wiki_space = args.get("wiki_space")
            .and_then(|v| v.as_str());
        
        match self.client.create_doc_with_content(title, markdown, folder_token, wiki_space).await {
            Ok(doc_id) => Ok(json!({
                "status": "success",
                "action": "create",
                "title": title,
                "folder_token": folder_token,
                "wiki_space": wiki_space,
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

    /// 检查任务状态
    async fn check_task_status(&self, task_id: &str) -> Result<Value> {
        self.client.check_task_status(task_id).await
    }

    /// 读取文档（保留旧接口兼容）
    async fn read(&self, doc_token: &str) -> Result<Value> {
        let token = Self::extract_token_from_url(doc_token)?;
        
        match self.client.read_doc(&token).await {
            Ok(content) => {
                let title = self.client.get_doc_title(&token).await.unwrap_or_else(|_| "Unknown".to_string());
                Ok(json!({
                    "status": "success",
                    "action": "read",
                    "doc_token": token,
                    "title": title,
                    "content": content,
                    "message": "Document read successfully"
                }))
            },
            Err(e) => Ok(json!({
                "status": "error",
                "action": "read",
                "doc_token": token,
                "error": e.to_string(),
                "message": "Failed to read document"
            }))
        }
    }

    /// 写入文档（保留旧接口兼容）
    async fn write(&self, doc_token: &str, content: &str) -> Result<Value> {
        let token = Self::extract_token_from_url(doc_token)?;
        self.client.update_doc(&token, content, None).await
    }

    /// 追加内容（保留旧接口兼容）
    async fn append(&self, doc_token: &str, content: &str) -> Result<Value> {
        let token = Self::extract_token_from_url(doc_token)?;
        self.client.update_doc(&token, content, Some("append".to_string())).await
    }
}

#[async_trait]
impl Tool for FeishuDocTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "feishu_doc".to_string(),
            description: "Feishu document operations. Actions: read, write, append, create, fetch, update.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Document action to perform"
                    },
                    "doc_id": {
                        "type": "string",
                        "description": "Document ID or URL (required for read, write, append, fetch, update)"
                    },
                    "doc_token": {
                        "type": "string",
                        "description": "Document token (legacy, use doc_id instead)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write or append (required for write, append)"
                    },
                    "markdown": {
                        "type": "string",
                        "description": "Markdown content (for create, update)"
                    },
                    "title": {
                        "type": "string",
                        "description": "Document title (required for create)"
                    },
                    "folder_token": {
                        "type": "string",
                        "description": "Folder token (optional)"
                    },
                    "wiki_node": {
                        "type": "string",
                        "description": "Wiki node token (optional, for create)"
                    },
                    "wiki_space": {
                        "type": "string",
                        "description": "Wiki space ID (optional, for create)"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Character offset (for fetch, optional)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max characters to return (for fetch, optional)"
                    },
                    "mode": {
                        "type": "string",
                        "description": "Update mode: overwrite, append, replace_range, replace_all, insert_before, insert_after, delete_range"
                    },
                    "selection_with_ellipsis": {
                        "type": "string",
                        "description": "Position with ellipsis (for update)"
                    },
                    "selection_by_title": {
                        "type": "string",
                        "description": "Position by title (for update)"
                    },
                    "new_title": {
                        "type": "string",
                        "description": "New document title (optional, for update)"
                    },
                    "task_id": {
                        "type": "string",
                        "description": "Task ID for async operation (for update)"
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
            "fetch" => {
                let doc_id = args.get("doc_id")
                    .or_else(|| args.get("doc_token"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_id"))?;
                let offset = args.get("offset").and_then(|v| v.as_i64()).map(|v| v as usize);
                let limit = args.get("limit").and_then(|v| v.as_i64()).map(|v| v as usize);
                self.fetch(doc_id, offset, limit).await
            }

            "update" => {
                self.update(&args).await
            }

            "read" => {
                let doc_token = args.get("doc_token")
                    .or_else(|| args.get("doc_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_id"))?;
                self.read(doc_token).await
            }

            "write" => {
                let doc_token = args.get("doc_token")
                    .or_else(|| args.get("doc_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_id"))?;
                let content = args.get("content")
                    .or_else(|| args.get("markdown"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
                self.write(doc_token, content).await
            }

            "append" => {
                let doc_token = args.get("doc_token")
                    .or_else(|| args.get("doc_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: doc_id"))?;
                let content = args.get("content")
                    .or_else(|| args.get("markdown"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
                self.append(doc_token, content).await
            }

            "create" => {
                self.create(&args).await
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_token_from_url() {
        // 测试从完整 URL 提取 token
        let url1 = "https://xxx.feishu.cn/docx/WHsldVn1OozVGuxXy1zccPl5nr3";
        let token1 = FeishuDocTool::extract_token_from_url(url1).unwrap();
        assert_eq!(token1, "WHsldVn1OozVGuxXy1zccPl5nr3");
        
        let url2 = "https://xxx.feishu.cn/wiki/ABC123";
        let token2 = FeishuDocTool::extract_token_from_url(url2).unwrap();
        assert_eq!(token2, "ABC123");
        
        let url3 = "WHsldVn1OozVGuxXy1zccPl5nr3";
        let token3 = FeishuDocTool::extract_token_from_url(url3).unwrap();
        assert_eq!(token3, "WHsldVn1OozVGuxXy1zccPl5nr3");
    }

    #[test]
    fn test_feishu_doc_metadata() {
        let tool = FeishuDocTool::new();
        assert_eq!(tool.metadata().name, "feishu_doc");
        assert!(tool.metadata().description.contains("fetch"));
        assert!(tool.metadata().description.contains("update"));
    }

    #[tokio::test]
    async fn test_read_document() {
        let tool = FeishuDocTool::new();
        let result = tool.read("doccn123456").await.unwrap();
        assert_eq!(result["action"], "read");
    }

    #[tokio::test]
    async fn test_fetch_with_url() {
        let tool = FeishuDocTool::new();
        let result = tool.fetch("https://xxx.feishu.cn/docx/WHsldVn1OozVGuxXy1zccPl5nr3", None, None).await.unwrap();
        assert_eq!(result["doc_id"], "WHsldVn1OozVGuxXy1zccPl5nr3");
    }
}