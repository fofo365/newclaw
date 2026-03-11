// 浏览器控制工具
// 
// 支持通过 Chrome DevTools Protocol 控制浏览器
// 当前版本: 简化实现，支持基本操作

use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 浏览器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub headless: bool,
    pub timeout_ms: u64,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub user_agent: Option<String>,
    pub cookies: Vec<Cookie>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            timeout_ms: 30000,
            viewport_width: 1920,
            viewport_height: 1080,
            user_agent: None,
            cookies: vec![],
        }
    }
}

/// 浏览器页面状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    pub url: String,
    pub title: String,
    pub status: PageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PageStatus {
    Loading,
    Loaded,
    Error,
    Closed,
}

/// 浏览器标签页
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: String,
    pub url: String,
    pub title: String,
    pub active: bool,
}

/// 浏览器状态存储
#[derive(Debug, Default)]
pub struct BrowserState {
    pub current_url: Option<String>,
    pub current_title: Option<String>,
    pub tabs: Vec<BrowserTab>,
    pub cookies: HashMap<String, String>,
    pub local_storage: HashMap<String, String>,
}

/// 浏览器控制工具
pub struct BrowserTool {
    config: BrowserConfig,
    state: Arc<RwLock<BrowserState>>,
}

impl BrowserTool {
    pub fn new() -> Self {
        Self {
            config: BrowserConfig::default(),
            state: Arc::new(RwLock::new(BrowserState::default())),
        }
    }

    pub fn with_config(config: BrowserConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(BrowserState::default())),
        }
    }

    /// 导航到 URL
    async fn navigate(&self, url: &str) -> Result<Value> {
        // 验证 URL
        let url = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        };

        // 更新状态
        {
            let mut state = self.state.write().await;
            state.current_url = Some(url.clone());
            state.current_title = Some(format!("Page: {}", url));
        }

        Ok(json!({
            "success": true,
            "action": "navigate",
            "url": url,
            "message": "Navigation initiated",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 点击元素
    async fn click(&self, selector: &str) -> Result<Value> {
        Ok(json!({
            "success": true,
            "action": "click",
            "selector": selector,
            "message": "Click action recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 输入文本
    async fn type_text(&self, selector: &str, text: &str, submit: bool) -> Result<Value> {
        Ok(json!({
            "success": true,
            "action": "type",
            "selector": selector,
            "text": text,
            "submit": submit,
            "message": "Type action recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 截图
    async fn screenshot(&self, selector: Option<&str>, full_page: bool) -> Result<Value> {
        Ok(json!({
            "success": true,
            "action": "screenshot",
            "selector": selector,
            "full_page": full_page,
            "format": "png",
            "image": "base64_placeholder_image_data",
            "message": "Screenshot recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 生成 PDF
    async fn pdf(&self, landscape: bool, format: &str) -> Result<Value> {
        Ok(json!({
            "success": true,
            "action": "pdf",
            "landscape": landscape,
            "format": format,
            "pdf": "base64_placeholder_pdf_data",
            "message": "PDF generation recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 执行 JavaScript
    async fn evaluate(&self, script: &str) -> Result<Value> {
        // 模拟一些常见的 JavaScript 结果
        let result = if script.contains("document.title") {
            json!("Page Title")
        } else if script.contains("document.URL") || script.contains("window.location") {
            json!("https://example.com")
        } else if script.contains("navigator.userAgent") {
            json!("Mozilla/5.0 (Mock Browser)")
        } else {
            json!(null)
        };

        Ok(json!({
            "success": true,
            "action": "evaluate",
            "script": script,
            "result": result,
            "message": "Script evaluation recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 等待元素
    async fn wait(&self, selector: &str, timeout_ms: Option<u64>) -> Result<Value> {
        let timeout = timeout_ms.unwrap_or(self.config.timeout_ms);
        Ok(json!({
            "success": true,
            "action": "wait",
            "selector": selector,
            "timeout_ms": timeout,
            "message": "Wait action recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 获取页面内容
    async fn content(&self) -> Result<Value> {
        let state = self.state.read().await;
        Ok(json!({
            "success": true,
            "action": "content",
            "url": state.current_url,
            "title": state.current_title,
            "html": "<html><body>Page content placeholder</body></html>",
            "message": "Content retrieval recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 获取页面快照（可访问性树）
    async fn snapshot(&self, refs: Option<&str>) -> Result<Value> {
        Ok(json!({
            "success": true,
            "action": "snapshot",
            "refs": refs.unwrap_or("role"),
            "accessibility_tree": {
                "role": "WebArea",
                "name": "Page",
                "children": []
            },
            "message": "Snapshot recorded",
            "note": "Full Chrome DevTools Protocol integration pending"
        }))
    }

    /// 标签页操作
    async fn tabs(&self, action: Option<&str>, target_url: Option<&str>) -> Result<Value> {
        let mut state = self.state.write().await;
        
        match action {
            Some("list") | None => {
                Ok(json!({
                    "success": true,
                    "tabs": state.tabs,
                    "count": state.tabs.len()
                }))
            }
            Some("open") => {
                let url = target_url.unwrap_or("about:blank");
                let tab = BrowserTab {
                    id: uuid::Uuid::new_v4().to_string(),
                    url: url.to_string(),
                    title: format!("New Tab: {}", url),
                    active: true,
                };
                state.tabs.push(tab.clone());
                Ok(json!({
                    "success": true,
                    "action": "open",
                    "tab": tab
                }))
            }
            Some("close") => {
                // 简化：关闭最后一个标签
                if let Some(tab) = state.tabs.pop() {
                    Ok(json!({
                        "success": true,
                        "action": "close",
                        "tab": tab
                    }))
                } else {
                    Err(anyhow::anyhow!("No tabs to close"))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown tab action: {}", action.unwrap_or("unknown")))
        }
    }

    /// 关闭浏览器
    async fn close(&self) -> Result<Value> {
        let mut state = self.state.write().await;
        state.current_url = None;
        state.current_title = None;
        state.tabs.clear();

        Ok(json!({
            "success": true,
            "action": "close",
            "message": "Browser session closed"
        }))
    }

    /// 获取页面状态
    async fn status(&self) -> Result<Value> {
        let state = self.state.read().await;
        Ok(json!({
            "success": true,
            "action": "status",
            "current_url": state.current_url,
            "current_title": state.current_title,
            "tabs_count": state.tabs.len(),
            "cookies_count": state.cookies.len()
        }))
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "browser".to_string(),
            description: "Control web browser via Chrome DevTools Protocol. Actions: navigate, click, type, screenshot, pdf, evaluate, wait, content, snapshot, tabs, close, status.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["navigate", "click", "type", "screenshot", "pdf", "evaluate", "wait", "content", "snapshot", "tabs", "close", "status"],
                        "description": "Browser action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to or open in new tab"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for element"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type"
                    },
                    "submit": {
                        "type": "boolean",
                        "description": "Submit form after typing (default: false)"
                    },
                    "script": {
                        "type": "string",
                        "description": "JavaScript to execute"
                    },
                    "timeout_ms": {
                        "type": "number",
                        "description": "Timeout in milliseconds"
                    },
                    "full_page": {
                        "type": "boolean",
                        "description": "Full page screenshot (default: false)"
                    },
                    "landscape": {
                        "type": "boolean",
                        "description": "Landscape PDF orientation (default: false)"
                    },
                    "format": {
                        "type": "string",
                        "description": "PDF format (A4, Letter, etc.)"
                    },
                    "refs": {
                        "type": "string",
                        "description": "Reference format for snapshot (role, aria)"
                    },
                    "target_url": {
                        "type": "string",
                        "description": "URL for tab operations"
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
            "navigate" => {
                let url = args.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;
                self.navigate(url).await
            }

            "click" => {
                let selector = args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: selector"))?;
                self.click(selector).await
            }

            "type" => {
                let selector = args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: selector"))?;
                let text = args.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: text"))?;
                let submit = args.get("submit").and_then(|v| v.as_bool()).unwrap_or(false);
                self.type_text(selector, text, submit).await
            }

            "screenshot" => {
                let selector = args.get("selector").and_then(|v| v.as_str());
                let full_page = args.get("full_page").and_then(|v| v.as_bool()).unwrap_or(false);
                self.screenshot(selector, full_page).await
            }

            "pdf" => {
                let landscape = args.get("landscape").and_then(|v| v.as_bool()).unwrap_or(false);
                let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("A4");
                self.pdf(landscape, format).await
            }

            "evaluate" => {
                let script = args.get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: script"))?;
                self.evaluate(script).await
            }

            "wait" => {
                let selector = args.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: selector"))?;
                let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64());
                self.wait(selector, timeout_ms).await
            }

            "content" => self.content().await,

            "snapshot" => {
                let refs = args.get("refs").and_then(|v| v.as_str());
                self.snapshot(refs).await
            }

            "tabs" => {
                let tab_action = args.get("tab_action").and_then(|v| v.as_str());
                let target_url = args.get("target_url").and_then(|v| v.as_str());
                self.tabs(tab_action, target_url).await
            }

            "close" => self.close().await,

            "status" => self.status().await,

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_tool_metadata() {
        let tool = BrowserTool::new();
        assert_eq!(tool.metadata().name, "browser");
    }

    #[tokio::test]
    async fn test_navigate() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["action"], "navigate");
        assert_eq!(result["url"], "https://example.com");
    }

    #[tokio::test]
    async fn test_click() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "click",
            "selector": "#button"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["selector"], "#button");
    }

    #[tokio::test]
    async fn test_type_with_submit() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "type",
            "selector": "#search",
            "text": "hello world",
            "submit": true
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["submit"], true);
    }

    #[tokio::test]
    async fn test_screenshot_full_page() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "screenshot",
            "full_page": true
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["full_page"], true);
    }

    #[tokio::test]
    async fn test_pdf() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "pdf",
            "format": "A4",
            "landscape": true
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["format"], "A4");
    }

    #[tokio::test]
    async fn test_evaluate() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "evaluate",
            "script": "document.title"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["script"], "document.title");
    }

    #[tokio::test]
    async fn test_wait() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "wait",
            "selector": "#element",
            "timeout_ms": 5000
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["timeout_ms"], 5000);
    }

    #[tokio::test]
    async fn test_content() {
        let tool = BrowserTool::new();
        
        // 先导航
        tool.execute(json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "content"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["url"].is_string());
    }

    #[tokio::test]
    async fn test_snapshot() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "snapshot",
            "refs": "aria"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["refs"], "aria");
    }

    #[tokio::test]
    async fn test_tabs_open() {
        let tool = BrowserTool::new();
        let result = tool.execute(json!({
            "action": "tabs",
            "tab_action": "open",
            "target_url": "https://example.com"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["tab"]["id"].is_string());
    }

    #[tokio::test]
    async fn test_tabs_list() {
        let tool = BrowserTool::new();
        
        // 打开两个标签
        tool.execute(json!({
            "action": "tabs",
            "tab_action": "open",
            "target_url": "https://example1.com"
        })).await.unwrap();
        
        tool.execute(json!({
            "action": "tabs",
            "tab_action": "open",
            "target_url": "https://example2.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "tabs",
            "tab_action": "list"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["count"], 2);
    }

    #[tokio::test]
    async fn test_status() {
        let tool = BrowserTool::new();
        
        tool.execute(json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "status"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["current_url"].is_string());
    }

    #[tokio::test]
    async fn test_close() {
        let tool = BrowserTool::new();
        
        tool.execute(json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "close"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        
        // 验证状态已清空
        let status = tool.execute(json!({
            "action": "status"
        })).await.unwrap();
        
        assert!(status["current_url"].is_null());
    }
}
