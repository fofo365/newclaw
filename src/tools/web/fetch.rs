// 网页内容获取工具

use async_trait::async_trait;
use reqwest::Client;

use crate::tools::{Tool, ToolError, ToolResult, ToolMetadata};

/// 网页内容获取工具
pub struct WebFetchTool {
    client: Client,
    timeout: std::time::Duration,
}

impl WebFetchTool {
    /// 创建新的获取工具
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            timeout: std::time::Duration::from_secs(10),
        }
    }
    
    /// 设置超时
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// 提取网页文本内容（简单版）
    fn extract_text(&self, html: &str) -> String {
        // 移除 script 和 style 标签
        let re_script = regex::Regex::new(r#"<script[^>]*>.*?</script>"#).unwrap();
        let re_style = regex::Regex::new(r#"<style[^>]*>.*?</style>"#).unwrap();
        
        let html = re_script.replace_all(html, "");
        let html = re_style.replace_all(&html, "");
        
        // 移除 HTML 标签
        let re_tags = regex::Regex::new(r#"<[^>]+>"#).unwrap();
        let text = re_tags.replace_all(&html, "");
        
        // 解码 HTML 实体
        let text = html_escape(&text);
        
        // 清理多余空白
        let text = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        
        text
    }
}

fn html_escape(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[async_trait::async_trait]
impl Tool for WebFetchTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "web_fetch".to_string(),
            description: "获取网页内容".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "网页 URL"
                    },
                    "max_chars": {
                        "type": "integer",
                        "description": "最大字符数",
                        "minimum": 100,
                        "default": 5000
                    }
                },
                "required": ["url"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 url 参数".to_string()))?;
        
        let max_chars = args["max_chars"].as_u64().unwrap_or(5000) as usize;
        
        // 验证 URL
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::InvalidArguments("URL 必须以 http:// 或 https:// 开头".to_string()));
        }
        
        // 发送请求
        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; NewClaw/0.5.0)")
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("请求失败: {}", e)))?;
        
        let status = response.status();
        if !status.is_success() {
            return Err(ToolError::ExecutionFailed(format!("HTTP 错误: {}", status)));
        }
        
        // 读取内容
        let html = response.text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("读取响应失败: {}", e)))?;
        
        // 提取文本
        let text = self.extract_text(&html);
        
        // 截断
        let truncated = text.len() > max_chars;
        let text = if truncated {
            text[..max_chars].to_string()
        } else {
            text
        };
        
        Ok(serde_json::json!({
            "content": text,
            "url": url,
            "truncated": truncated,
            "length": text.len()
        }))
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_fetch_metadata() {
        let tool = WebFetchTool::new();
        let metadata = tool.metadata();
        
        assert_eq!(metadata.name, "web_fetch");
        assert!(metadata.description.contains("获取"));
    }
    
    #[tokio::test]
    async fn test_fetch_validation() {
        let tool = WebFetchTool::new();
        
        // 缺少 url 参数
        let result = tool.execute(serde_json::json!({
            "max_chars": 1000
        })).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_fetch_invalid_url() {
        let tool = WebFetchTool::new();
        
        // 无效 URL
        let result = tool.execute(serde_json::json!({
            "url": "ftp://example.com"
        })).await;
        
        assert!(result.is_err());
    }
}
