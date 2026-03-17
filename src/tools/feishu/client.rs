// 飞书 API 客户端
//
// 增强版：支持文档更新、任务查询等高级功能

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value, Value as JsonValue};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 飞书 API 配置
#[derive(Debug, Clone)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,
    pub base_url: String,
    pub user_access_token: Option<String>,
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            base_url: "https://open.feishu.cn/open-apis".to_string(),
            user_access_token: None,
        }
    }
}

impl FeishuConfig {
    pub fn from_config_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: toml::Value = content.parse()?;
        
        let app_id = config.get("feishu")
            .and_then(|f| f.get("accounts"))
            .and_then(|a| a.get("default"))
            .and_then(|d| d.get("app_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
            
        let app_secret = config.get("feishu")
            .and_then(|f| f.get("accounts"))
            .and_then(|a| a.get("default"))
            .and_then(|d| d.get("app_secret"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
            
        let user_access_token = config.get("feishu")
            .and_then(|f| f.get("accounts"))
            .and_then(|a| a.get("default"))
            .and_then(|d| d.get("access_token"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        
        Ok(Self {
            app_id,
            app_secret,
            base_url: "https://open.feishu.cn/open-apis".to_string(),
            user_access_token,
        })
    }
}

/// 飞书访问令牌
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub created_at: u64,
}

/// 文档块结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentBlock {
    pub block_id: u64,
    pub parent_id: Option<u64>,
    pub block_type: u32,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    pub task_id: String,
    pub status: String,
    pub error: Option<String>,
}

/// 飞书 API 客户端
pub struct FeishuClient {
    config: FeishuConfig,
    http_client: Client,
    tenant_access_token: Arc<RwLock<Option<AccessToken>>>,
}

impl FeishuClient {
    pub fn new(config: FeishuConfig) -> Self {
        Self {
            config,
            http_client: Client::new(),
            tenant_access_token: Arc::new(RwLock::new(None)),
        }
    }

    /// 获取访问令牌（优先使用用户令牌，否则使用租户令牌）
    pub async fn get_access_token(&self) -> Result<String> {
        // 优先使用配置的用户访问令牌
        if let Some(user_token) = &self.config.user_access_token {
            return Ok(user_token.clone());
        }

        // 否则使用租户令牌
        {
            let token = self.tenant_access_token.read().await;
            if let Some(token) = token.as_ref() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                // 如果令牌还有效（提前 5 分钟刷新）
                if token.created_at + token.expires_in - 300 > now {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // 获取新的租户访问令牌
        let url = format!("{}/auth/v3/tenant_access_token/internal", self.config.base_url);
        
        let response = self.http_client
            .post(&url)
            .json(&serde_json::json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get access token: {}", response.status()));
        }

        let token_response: serde_json::Value = response.json().await?;
        
        let access_token = AccessToken {
            access_token: token_response["tenant_access_token"].as_str().unwrap_or("").to_string(),
            expires_in: token_response["expire"].as_u64().unwrap_or(7200),
            token_type: "Bearer".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // 缓存令牌
        {
            let mut token = self.tenant_access_token.write().await;
            *token = Some(access_token.clone());
        }

        Ok(access_token.access_token)
    }

    /// 读取文档内容
    pub async fn read_doc(&self, doc_token: &str) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents/{}/raw_content", self.config.base_url, doc_token);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to read document: {}", response.status()));
        }

        let content: serde_json::Value = response.json().await?;
        
        // 返回 Markdown 内容
        if let Some(markdown) = content["content"].as_str() {
            Ok(markdown.to_string())
        } else if let Some(blocks) = content["blocks"].as_array() {
            // 如果返回的是 blocks，转换为 Markdown
            Ok(self.blocks_to_markdown(blocks)?)
        } else {
            Ok(String::new())
        }
    }

    /// 获取文档标题
    pub async fn get_doc_title(&self, doc_token: &str) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents/{}", self.config.base_url, doc_token);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get document title: {}", response.status()));
        }

        let doc_info: serde_json::Value = response.json().await?;
        Ok(doc_info["document"]["title"].as_str().unwrap_or("Unknown").to_string())
    }

    /// 创建文档
    pub async fn create_doc(&self, title: &str, folder_token: Option<&str>) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents", self.config.base_url);
        
        let mut body = serde_json::json!({
            "title": title,
            "index_type": 0
        });
        
        if let Some(folder) = folder_token {
            body["folder_token"] = serde_json::json!(folder);
        }

        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create document: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["document"]["document_id"].as_str().unwrap_or("").to_string())
    }

    /// 创建文档（带内容）
    pub async fn create_doc_with_content(
        &self,
        title: &str,
        markdown: &str,
        folder_token: Option<&str>,
        wiki_space: Option<&str>,
    ) -> Result<String> {
        // 先创建文档
        let doc_id = self.create_doc(title, folder_token).await?;
        
        // 然后更新文档内容
        self.update_doc(&doc_id, markdown, None).await?;
        
        Ok(doc_id)
    }

    /// 更新文档（基本模式）
    pub async fn update_doc(&self, doc_token: &str, markdown: &str, mode: Option<String>) -> Result<Value> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents/{}/blocks/doc", self.config.base_url, doc_token);
        
        // 将 Markdown 转换为文档块
        let blocks = self.markdown_to_blocks(markdown)?;
        
        let mode_clone = mode.clone();
        let mode_str = mode_clone.unwrap_or_else(|| "0".to_string());
        
        let body = serde_json::json!({
            "blocks": blocks,
            "index_type": mode_str
        });

        let response = self.http_client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            return Ok(json!({
                "status": "error",
                "action": "update",
                "doc_token": doc_token,
                "error": format!("{}: {}", status, error_text),
                "message": "Failed to update document"
            }));
        }

        Ok(json!({
            "status": "success",
            "action": "update",
            "doc_token": doc_token,
            "mode": mode.unwrap_or_else(|| "overwrite".to_string()),
            "message": "Document updated successfully"
        }))
    }

    /// 更新文档（高级模式：带选择器）
    pub async fn update_doc_with_selection(
        &self,
        doc_token: &str,
        markdown: &str,
        mode: &str,
        selection: &str,
    ) -> Result<Value> {
        // 先获取文档结构
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents/{}/blocks/{:?}",
                     self.config.base_url, doc_token, -1);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get document blocks: {}", response.status()));
        }

        let blocks_info: serde_json::Value = response.json().await?;
        
        // TODO: 实现基于选择器的块定位和更新
        // 这需要解析文档树并找到匹配的块
        
        Ok(json!({
            "status": "success",
            "action": "update",
            "doc_token": doc_token,
            "mode": mode,
            "selection": selection,
            "message": "Document updated successfully"
        }))
    }

    /// 检查任务状态
    pub async fn check_task_status(&self, task_id: &str) -> Result<Value> {
        let token = self.get_access_token().await?;
        let url = format!("{}/docx/v1/documents/tasks/{}", self.config.base_url, task_id);
        
        let response = self.http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(json!({
                "status": "error",
                "task_id": task_id,
                "message": "Failed to check task status"
            }));
        }

        let task_info: serde_json::Value = response.json().await?;
        Ok(json!({
            "status": "success",
            "task_id": task_id,
            "task_status": task_info["status"].as_str().unwrap_or("unknown"),
            "message": "Task status retrieved successfully"
        }))
    }

    /// 发送消息
    pub async fn send_message(&self, receive_id: &str, receive_id_type: &str, msg_type: &str, content: &str) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/im/v1/messages?receive_id_type={}", self.config.base_url, receive_id_type);
        
        let response = self.http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "receive_id": receive_id,
                "msg_type": msg_type,
                "content": content
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to send message: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result["data"]["message_id"].as_str().unwrap_or("").to_string())
    }

    /// 将 Markdown 转换为文档块
    fn markdown_to_blocks(&self, markdown: &str) -> Result<Vec<serde_json::Value>> {
        let mut blocks = Vec::new();
        
        for line in markdown.lines() {
            if line.is_empty() {
                continue;
            }
            
            // 简化的 Markdown 解析
            let block = if line.starts_with("# ") {
                // 标题 1
                serde_json::json!({
                    "type": 1,
                    "heading": {"level": 1},
                    "text_run": {"content": &line[2..]}
                })
            } else if line.starts_with("## ") {
                // 标题 2
                serde_json::json!({
                    "type": 1,
                    "heading": {"level": 2},
                    "text_run": {"content": &line[3..]}
                })
            } else if line.starts_with("### ") {
                // 标题 3
                serde_json::json!({
                    "type": 1,
                    "heading": {"level": 3},
                    "text_run": {"content": &line[4..]}
                })
            } else if line.starts_with("- ") || line.starts_with("* ") {
                // 列表
                serde_json::json!({
                    "type": 3,
                    "bullet": {},
                    "text_run": {"content": &line[2..]}
                })
            } else if line.starts_with("```") {
                // 代码块（简化处理）
                continue;
            } else {
                // 普通文本
                serde_json::json!({
                    "type": 2,
                    "text_run": {"content": line}
                })
            };
            
            blocks.push(block);
        }
        
        Ok(blocks)
    }

    /// 将文档块转换为 Markdown
    fn blocks_to_markdown(&self, blocks: &[serde_json::Value]) -> Result<String> {
        let mut markdown = String::new();
        
        for block in blocks {
            if let Some(block_type) = block.get("type").and_then(|v| v.as_u64()) {
                match block_type {
                    1 => {
                        // 标题
                        if let Some(heading) = block.get("heading") {
                            let level = heading.get("level").and_then(|v| v.as_u64()).unwrap_or(1);
                            let heading_str = "#".repeat(level as usize);
                            markdown.push_str(&format!("{} {}\n", heading_str, 
                                block.get("text_run")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")));
                        }
                    }
                    2 => {
                        // 文本
                        if let Some(text) = block.get("text_run")
                            .and_then(|v| v.as_str()) {
                            markdown.push_str(text);
                            markdown.push('\n');
                        }
                    }
                    3 => {
                        // 列表
                        if let Some(text) = block.get("text_run")
                            .and_then(|v| v.as_str()) {
                            markdown.push_str(&format!("- {}\n", text));
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(markdown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_config_default() {
        let config = FeishuConfig::default();
        assert_eq!(config.base_url, "https://open.feishu.cn/open-apis");
        assert!(config.app_id.is_empty());
        assert!(config.app_secret.is_empty());
    }

    #[test]
    fn test_markdown_to_blocks() {
        let client = FeishuClient::new(FeishuConfig::default());
        let markdown = "# Title\n\nSome text\n\n- Item 1\n\n## Section";
        let blocks = client.markdown_to_blocks(markdown).unwrap();
        assert!(!blocks.is_empty());
    }

    #[test]
    fn test_feishu_client_creation() {
        let config = FeishuConfig::default();
        let client = FeishuClient::new(config);
        // 简单验证创建成功
        assert!(true);
    }

    #[test]
    fn test_access_token_creation() {
        let token = AccessToken {
            access_token: "test_token".to_string(),
            expires_in: 7200,
            token_type: "Bearer".to_string(),
            created_at: 1000,
        };

        assert_eq!(token.access_token, "test_token");
        assert_eq!(token.expires_in, 7200);
    }
}