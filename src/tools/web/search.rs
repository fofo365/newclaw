// 网络搜索工具（Brave Search API）

use async_trait::async_trait;
use reqwest::Client;

use crate::tools::{Tool, ToolError, ToolResult, ToolMetadata};

/// 网络搜索工具
pub struct WebSearchTool {
    client: Client,
    api_key: Option<String>,
}

impl WebSearchTool {
    /// 创建新的搜索工具
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: None,
        }
    }
    
    /// 设置 API Key
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = Some(key);
        self
    }
    
    /// 执行搜索（使用 DuckDuckGo HTML 版本，无需 API Key）
    async fn search_duckduckgo(&self, query: &str, count: usize) -> ToolResult<serde_json::Value> {
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "Mozilla/5.0 (compatible; NewClaw/0.5.0)")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("请求失败: {}", e)))?;
        
        let html = response.text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("读取响应失败: {}", e)))?;
        
        // 简单解析结果（提取标题和 URL）
        let mut results = Vec::new();
        let re = regex::Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#).unwrap();
        
        for cap in re.captures_iter(&html).take(count) {
            let url = cap[1].to_string();
            let title = cap[2].to_string();
            
            // DuckDuckGo 使用重定向 URL，需要解码
            let url = if url.starts_with("/l/?uddg=") {
                urlencoding_decode(&url[9..])
            } else {
                url
            };
            
            results.push(serde_json::json!({
                "title": title,
                "url": url,
                "snippet": "" // DuckDuckGo HTML 版本不提供 snippet
            }));
        }
        
        Ok(serde_json::json!({
            "results": results,
            "count": results.len(),
            "query": query
        }))
    }
}

fn urlencoding_decode(s: &str) -> String {
    urlencoding::decode(s)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| s.to_string())
}

#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "web_search".to_string(),
            description: "搜索网络内容".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "搜索关键词"
                    },
                    "count": {
                        "type": "integer",
                        "description": "返回结果数量（1-10）",
                        "minimum": 1,
                        "maximum": 10,
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 query 参数".to_string()))?;
        
        let count = args["count"].as_u64().unwrap_or(5) as usize;
        let count = count.clamp(1, 10);
        
        // 使用 DuckDuckGo（无需 API Key）
        self.search_duckduckgo(query, count).await
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_search_metadata() {
        let tool = WebSearchTool::new();
        let metadata = tool.metadata();
        
        assert_eq!(metadata.name, "web_search");
        assert!(metadata.description.contains("搜索"));
    }
    
    #[tokio::test]
    async fn test_search_validation() {
        let tool = WebSearchTool::new();
        
        // 缺少 query 参数
        let result = tool.execute(serde_json::json!({
            "count": 5
        })).await;
        
        assert!(result.is_err());
    }
}
