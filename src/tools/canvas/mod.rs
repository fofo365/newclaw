// Canvas 展示工具 (简化版，占位符实现)
use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// Canvas 配置
#[derive(Debug, Clone)]
pub struct CanvasConfig {
    pub headless: bool,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1920,
            viewport_height: 1080,
        }
    }
}

/// Canvas 展示工具
pub struct CanvasTool {
    config: CanvasConfig,
}

impl CanvasTool {
    pub fn new() -> Self {
        Self {
            config: CanvasConfig::default(),
        }
    }

    pub fn with_config(config: CanvasConfig) -> Self {
        Self { config }
    }

    /// 展示内容（URL 或 HTML）
    async fn present(&self, url: Option<&str>, html: Option<&str>) -> Result<Value> {
        if url.is_none() && html.is_none() {
            return Err(anyhow::anyhow!("Either url or html must be provided"));
        }

        Ok(json!({
            "status": "success",
            "action": "present",
            "url": url,
            "html_provided": html.is_some(),
            "message": "Canvas presented (placeholder - Chrome integration pending)"
        }))
    }

    /// 隐藏 Canvas
    async fn hide(&self) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "hide",
            "message": "Canvas hidden (placeholder - Chrome integration pending)"
        }))
    }

    /// 导航到 URL
    async fn navigate(&self, url: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "navigate",
            "url": url,
            "message": "Navigated (placeholder - Chrome integration pending)"
        }))
    }

    /// 执行 JavaScript
    async fn eval(&self, script: &str) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "eval",
            "script": script,
            "result": "Script executed (placeholder - Chrome integration pending)"
        }))
    }

    /// 截图
    async fn snapshot(&self, selector: Option<&str>) -> Result<Value> {
        Ok(json!({
            "status": "success",
            "action": "snapshot",
            "selector": selector,
            "image": "base64_encoded_image_placeholder",
            "message": "Snapshot taken (placeholder - Chrome integration pending)"
        }))
    }
}

#[async_trait]
impl Tool for CanvasTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "canvas".to_string(),
            description: "Present and control canvas for displaying web content. Actions: present, hide, navigate, eval, snapshot.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["present", "hide", "navigate", "eval", "snapshot"],
                        "description": "Canvas action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to present or navigate to"
                    },
                    "html": {
                        "type": "string",
                        "description": "HTML content to present"
                    },
                    "script": {
                        "type": "string",
                        "description": "JavaScript to execute"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for snapshot (optional)"
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
            "present" => {
                let url = args.get("url").and_then(|v| v.as_str());
                let html = args.get("html").and_then(|v| v.as_str());
                self.present(url, html).await
            }

            "hide" => self.hide().await,

            "navigate" => {
                let url = args.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: url"))?;
                self.navigate(url).await
            }

            "eval" => {
                let script = args.get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: script"))?;
                self.eval(script).await
            }

            "snapshot" => {
                let selector = args.get("selector").and_then(|v| v.as_str());
                self.snapshot(selector).await
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canvas_tool_metadata() {
        let tool = CanvasTool::new();
        assert_eq!(tool.metadata().name, "canvas");
    }

    #[tokio::test]
    async fn test_present_url() {
        let tool = CanvasTool::new();
        let result = tool.present(Some("https://example.com"), None).await.unwrap();
        assert_eq!(result["action"], "present");
        assert_eq!(result["url"], "https://example.com");
    }

    #[tokio::test]
    async fn test_present_html() {
        let tool = CanvasTool::new();
        let result = tool.present(None, Some("<h1>Hello</h1>")).await.unwrap();
        assert_eq!(result["action"], "present");
        assert_eq!(result["html_provided"], true);
    }

    #[tokio::test]
    async fn test_hide() {
        let tool = CanvasTool::new();
        let result = tool.hide().await.unwrap();
        assert_eq!(result["action"], "hide");
    }

    #[tokio::test]
    async fn test_navigate() {
        let tool = CanvasTool::new();
        let result = tool.navigate("https://example.com").await.unwrap();
        assert_eq!(result["action"], "navigate");
    }

    #[tokio::test]
    async fn test_eval() {
        let tool = CanvasTool::new();
        let result = tool.eval("console.log('test')").await.unwrap();
        assert_eq!(result["action"], "eval");
    }

    #[tokio::test]
    async fn test_snapshot() {
        let tool = CanvasTool::new();
        let result = tool.snapshot(None).await.unwrap();
        assert_eq!(result["action"], "snapshot");
    }
}
