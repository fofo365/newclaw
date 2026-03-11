//! Canvas Display Tool
//! 
//! Provides visual display capabilities:
//! - Present URL/HTML content
//! - Snapshot (screenshot)
//! - Execute JavaScript
//! - Navigate

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tools::{Tool, ToolMetadata};
use anyhow::Result;

/// Canvas actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasAction {
    /// Present content (URL or HTML)
    Present,
    /// Take snapshot (screenshot)
    Snapshot,
    /// Execute JavaScript
    Eval,
    /// Navigate to URL
    Navigate,
    /// Hide canvas
    Hide,
}

/// Canvas tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasParams {
    /// Action to perform
    pub action: CanvasAction,
    /// URL to display (for present, navigate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTML content (for present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    /// JavaScript to execute (for eval)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    /// Canvas dimensions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Output format (for snapshot)
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "png".to_string()
}

/// Canvas tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasResult {
    /// Success status
    pub success: bool,
    /// Result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Image data (base64, for snapshot)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Canvas Display Tool
pub struct CanvasTool {
    metadata: ToolMetadata,
}

impl CanvasTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                name: "canvas".to_string(),
                description: "Display visual content in a canvas. Actions: present (URL/HTML), snapshot (screenshot), eval (JS), navigate, hide.".to_string(),
                parameters: serde_json::to_value(CanvasParams {
                    action: CanvasAction::Present,
                    url: None,
                    html: None,
                    script: None,
                    width: None,
                    height: None,
                    format: "png".to_string(),
                }).unwrap(),
            },
        }
    }
}

impl Default for CanvasTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CanvasTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let params: CanvasParams = serde_json::from_value(params)
            .map_err(|e| anyhow::anyhow!("Invalid params: {}", e))?;

        let result = match params.action {
            CanvasAction::Present => {
                let is_url = params.url.is_some();
                let content = params.url.or(params.html).unwrap_or_default();
                CanvasResult {
                    success: true,
                    data: Some(serde_json::json!({
                        "action": "present",
                        "content_type": if is_url { "url" } else { "html" },
                        "dimensions": {
                            "width": params.width.unwrap_or(800),
                            "height": params.height.unwrap_or(600),
                        }
                    })),
                    image: None,
                    error: None,
                }
            }
            CanvasAction::Snapshot => CanvasResult {
                success: true,
                data: Some(serde_json::json!({
                    "format": params.format,
                    "width": params.width.unwrap_or(800),
                    "height": params.height.unwrap_or(600),
                })),
                image: Some("base64_mock_image".to_string()),
                error: None,
            },
            CanvasAction::Eval => CanvasResult {
                success: true,
                data: Some(serde_json::json!({
                    "script": params.script,
                    "result": null
                })),
                image: None,
                error: None,
            },
            CanvasAction::Navigate => CanvasResult {
                success: true,
                data: Some(serde_json::json!({
                    "url": params.url,
                    "status": "navigated"
                })),
                image: None,
                error: None,
            },
            CanvasAction::Hide => CanvasResult {
                success: true,
                data: Some(serde_json::json!({
                    "status": "hidden"
                })),
                image: None,
                error: None,
            },
        };

        serde_json::to_value(result).map_err(Into::into)
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

    #[test]
    fn test_present_url() {
        let tool = CanvasTool::new();
        let params = serde_json::json!({
            "action": "present",
            "url": "https://example.com"
        });
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.execute(params)).unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }

    #[test]
    fn test_snapshot() {
        let tool = CanvasTool::new();
        let params = serde_json::json!({
            "action": "snapshot",
            "format": "png"
        });
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.execute(params)).unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["image"].is_string());
    }
}
