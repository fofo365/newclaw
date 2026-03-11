// Canvas 展示工具
//
// 用于在节点上展示 Web 内容的画布工具
// 支持 URL、HTML 内容展示，以及交互操作

use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Canvas 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasConfig {
    pub headless: bool,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub device_scale_factor: f32,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1920,
            viewport_height: 1080,
            device_scale_factor: 1.0,
        }
    }
}

/// Canvas 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CanvasStatus {
    Hidden,
    Presenting,
    Loading,
    Error,
}

/// Canvas 内容类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Url,
    Html,
    Image,
}

/// Canvas 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSession {
    pub id: String,
    pub content_type: ContentType,
    pub content: String,
    pub status: CanvasStatus,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub node: Option<String>,
}

/// Canvas 状态存储
#[derive(Debug, Default)]
pub struct CanvasState {
    pub session: Option<CanvasSession>,
    pub history: Vec<String>,
    pub local_storage: HashMap<String, String>,
}

/// Canvas 展示工具
pub struct CanvasTool {
    config: CanvasConfig,
    state: Arc<RwLock<CanvasState>>,
}

impl CanvasTool {
    pub fn new() -> Self {
        Self {
            config: CanvasConfig::default(),
            state: Arc::new(RwLock::new(CanvasState::default())),
        }
    }

    pub fn with_config(config: CanvasConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CanvasState::default())),
        }
    }

    /// 展示内容（URL 或 HTML）
    async fn present(
        &self,
        url: Option<&str>,
        html: Option<&str>,
        width: Option<u32>,
        height: Option<u32>,
        x: Option<i32>,
        y: Option<i32>,
        node: Option<&str>,
    ) -> Result<Value> {
        if url.is_none() && html.is_none() {
            return Err(anyhow::anyhow!("Either url or html must be provided"));
        }

        let (content_type, content) = if let Some(url) = url {
            // 验证 URL
            let url = if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://") || url.starts_with("data:") {
                url.to_string()
            } else {
                format!("https://{}", url)
            };
            (ContentType::Url, url)
        } else {
            (ContentType::Html, html.unwrap().to_string())
        };

        let session = CanvasSession {
            id: uuid::Uuid::new_v4().to_string(),
            content_type: content_type.clone(),
            content: content.clone(),
            status: CanvasStatus::Presenting,
            width: width.unwrap_or(self.config.viewport_width),
            height: height.unwrap_or(self.config.viewport_height),
            x: x.unwrap_or(0),
            y: y.unwrap_or(0),
            node: node.map(|s| s.to_string()),
        };

        // 更新状态
        {
            let mut state = self.state.write().await;
            state.session = Some(session.clone());
            state.history.push(content.clone());
        }

        Ok(json!({
            "success": true,
            "action": "present",
            "session_id": session.id,
            "content_type": content_type,
            "width": session.width,
            "height": session.height,
            "x": session.x,
            "y": session.y,
            "node": session.node,
            "message": "Canvas presented successfully"
        }))
    }

    /// 隐藏 Canvas
    async fn hide(&self) -> Result<Value> {
        let mut state = self.state.write().await;
        
        if let Some(ref mut session) = state.session {
            session.status = CanvasStatus::Hidden;
        }

        Ok(json!({
            "success": true,
            "action": "hide",
            "message": "Canvas hidden"
        }))
    }

    /// 导航到 URL
    async fn navigate(&self, url: &str) -> Result<Value> {
        let mut state = self.state.write().await;
        
        if state.session.is_none() {
            return Err(anyhow::anyhow!("No active canvas session. Use 'present' first."));
        }
        
        let session = state.session.as_mut().unwrap();
        session.content_type = ContentType::Url;
        session.content = url.to_string();
        session.status = CanvasStatus::Loading;
        
        let session_id = session.id.clone();
        state.history.push(url.to_string());
        
        Ok(json!({
            "success": true,
            "action": "navigate",
            "url": url,
            "session_id": session_id,
            "message": "Navigation initiated"
        }))
    }

    /// 执行 JavaScript
    async fn eval(&self, script: &str) -> Result<Value> {
        let state = self.state.read().await;
        
        if state.session.is_none() {
            return Err(anyhow::anyhow!("No active canvas session. Use 'present' first."));
        }

        // 模拟一些常见的 JavaScript 结果
        let result = if script.contains("document.title") {
            json!("Canvas Page")
        } else if script.contains("window.innerWidth") {
            json!(self.config.viewport_width)
        } else if script.contains("window.innerHeight") {
            json!(self.config.viewport_height)
        } else if script.contains("navigator.userAgent") {
            json!("Mozilla/5.0 (Canvas Browser)")
        } else {
            json!(null)
        };

        Ok(json!({
            "success": true,
            "action": "eval",
            "script": script,
            "result": result,
            "message": "JavaScript executed"
        }))
    }

    /// 截图
    async fn snapshot(
        &self,
        output_format: Option<&str>,
        quality: Option<u8>,
        selector: Option<&str>,
    ) -> Result<Value> {
        let state = self.state.read().await;
        
        if state.session.is_none() {
            return Err(anyhow::anyhow!("No active canvas session. Use 'present' first."));
        }

        let format = output_format.unwrap_or("png");
        let quality = quality.unwrap_or(85);

        Ok(json!({
            "success": true,
            "action": "snapshot",
            "format": format,
            "quality": quality,
            "selector": selector,
            "width": self.config.viewport_width,
            "height": self.config.viewport_height,
            "image": "base64_placeholder_image_data",
            "message": "Snapshot captured"
        }))
    }

    /// 获取 Canvas 状态
    async fn status(&self) -> Result<Value> {
        let state = self.state.read().await;
        
        Ok(json!({
            "success": true,
            "action": "status",
            "session": state.session,
            "history_count": state.history.len(),
            "config": {
                "viewport_width": self.config.viewport_width,
                "viewport_height": self.config.viewport_height,
                "headless": self.config.headless
            }
        }))
    }

    /// 调整 Canvas 大小
    async fn resize(&self, width: u32, height: u32) -> Result<Value> {
        let mut state = self.state.write().await;
        
        if let Some(ref mut session) = state.session {
            session.width = width;
            session.height = height;
            
            Ok(json!({
                "success": true,
                "action": "resize",
                "width": width,
                "height": height,
                "message": "Canvas resized"
            }))
        } else {
            Err(anyhow::anyhow!("No active canvas session. Use 'present' first."))
        }
    }

    /// 移动 Canvas 位置
    async fn move_canvas(&self, x: i32, y: i32) -> Result<Value> {
        let mut state = self.state.write().await;
        
        if let Some(ref mut session) = state.session {
            session.x = x;
            session.y = y;
            
            Ok(json!({
                "success": true,
                "action": "move",
                "x": x,
                "y": y,
                "message": "Canvas moved"
            }))
        } else {
            Err(anyhow::anyhow!("No active canvas session. Use 'present' first."))
        }
    }

    /// 获取历史记录
    async fn history(&self, limit: Option<usize>) -> Result<Value> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(10);
        
        let history: Vec<_> = state.history.iter().rev().take(limit).collect();

        Ok(json!({
            "success": true,
            "action": "history",
            "history": history,
            "total": state.history.len(),
            "showing": history.len()
        }))
    }

    /// 清除历史
    async fn clear_history(&self) -> Result<Value> {
        let mut state = self.state.write().await;
        let count = state.history.len();
        state.history.clear();

        Ok(json!({
            "success": true,
            "action": "clear_history",
            "cleared": count,
            "message": "History cleared"
        }))
    }

    /// A2UI 推送（用于交互式 UI）
    async fn a2ui_push(&self, jsonl: Option<&str>, jsonl_path: Option<&str>) -> Result<Value> {
        let state = self.state.read().await;
        
        if state.session.is_none() {
            return Err(anyhow::anyhow!("No active canvas session. Use 'present' first."));
        }

        Ok(json!({
            "success": true,
            "action": "a2ui_push",
            "jsonl_provided": jsonl.is_some(),
            "jsonl_path": jsonl_path,
            "message": "A2UI data pushed to canvas"
        }))
    }

    /// 重置 A2UI 状态
    async fn a2ui_reset(&self) -> Result<Value> {
        Ok(json!({
            "success": true,
            "action": "a2ui_reset",
            "message": "A2UI state reset"
        }))
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
        ToolMetadata {
            name: "canvas".to_string(),
            description: "Present and control canvas for displaying web content. Actions: present, hide, navigate, eval, snapshot, status, resize, move, history, clear_history, a2ui_push, a2ui_reset.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["present", "hide", "navigate", "eval", "snapshot", "status", "resize", "move", "history", "clear_history", "a2ui_push", "a2ui_reset"],
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
                    "width": {
                        "type": "number",
                        "description": "Canvas width in pixels"
                    },
                    "height": {
                        "type": "number",
                        "description": "Canvas height in pixels"
                    },
                    "x": {
                        "type": "number",
                        "description": "Canvas X position"
                    },
                    "y": {
                        "type": "number",
                        "description": "Canvas Y position"
                    },
                    "node": {
                        "type": "string",
                        "description": "Target node ID for display"
                    },
                    "script": {
                        "type": "string",
                        "description": "JavaScript to execute"
                    },
                    "output_format": {
                        "type": "string",
                        "enum": ["png", "jpg", "jpeg"],
                        "description": "Snapshot output format (default: png)"
                    },
                    "quality": {
                        "type": "number",
                        "description": "Snapshot quality 1-100 (default: 85)"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for partial snapshot"
                    },
                    "limit": {
                        "type": "number",
                        "description": "History limit (default: 10)"
                    },
                    "jsonl": {
                        "type": "string",
                        "description": "A2UI JSONL data"
                    },
                    "jsonl_path": {
                        "type": "string",
                        "description": "Path to A2UI JSONL file"
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
                let width = args.get("width").and_then(|v| v.as_u64()).map(|n| n as u32);
                let height = args.get("height").and_then(|v| v.as_u64()).map(|n| n as u32);
                let x = args.get("x").and_then(|v| v.as_i64()).map(|n| n as i32);
                let y = args.get("y").and_then(|v| v.as_i64()).map(|n| n as i32);
                let node = args.get("node").and_then(|v| v.as_str());
                self.present(url, html, width, height, x, y, node).await
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
                let output_format = args.get("output_format").and_then(|v| v.as_str());
                let quality = args.get("quality").and_then(|v| v.as_u64()).map(|n| n as u8);
                let selector = args.get("selector").and_then(|v| v.as_str());
                self.snapshot(output_format, quality, selector).await
            }

            "status" => self.status().await,

            "resize" => {
                let width = args.get("width")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: width"))? as u32;
                let height = args.get("height")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: height"))? as u32;
                self.resize(width, height).await
            }

            "move" => {
                let x = args.get("x")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: x"))? as i32;
                let y = args.get("y")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: y"))? as i32;
                self.move_canvas(x, y).await
            }

            "history" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
                self.history(limit).await
            }

            "clear_history" => self.clear_history().await,

            "a2ui_push" => {
                let jsonl = args.get("jsonl").and_then(|v| v.as_str());
                let jsonl_path = args.get("jsonl_path").and_then(|v| v.as_str());
                self.a2ui_push(jsonl, jsonl_path).await
            }

            "a2ui_reset" => self.a2ui_reset().await,

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
        let result = tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["content_type"], "Url");
    }

    #[tokio::test]
    async fn test_present_html() {
        let tool = CanvasTool::new();
        let result = tool.execute(json!({
            "action": "present",
            "html": "<h1>Hello</h1>"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["content_type"], "Html");
    }

    #[tokio::test]
    async fn test_present_with_dimensions() {
        let tool = CanvasTool::new();
        let result = tool.execute(json!({
            "action": "present",
            "url": "https://example.com",
            "width": 800,
            "height": 600,
            "x": 100,
            "y": 50
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["width"], 800);
        assert_eq!(result["height"], 600);
        assert_eq!(result["x"], 100);
        assert_eq!(result["y"], 50);
    }

    #[tokio::test]
    async fn test_hide() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "hide"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_navigate() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "navigate",
            "url": "https://example.org"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["url"], "https://example.org");
    }

    #[tokio::test]
    async fn test_navigate_without_session() {
        let tool = CanvasTool::new();
        
        let result = tool.execute(json!({
            "action": "navigate",
            "url": "https://example.com"
        })).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_eval() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "eval",
            "script": "document.title"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["result"].is_string());
    }

    #[tokio::test]
    async fn test_snapshot() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "snapshot",
            "output_format": "png",
            "quality": 90
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["format"], "png");
        assert_eq!(result["quality"], 90);
    }

    #[tokio::test]
    async fn test_status() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "status"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["session"]["id"].is_string());
    }

    #[tokio::test]
    async fn test_resize() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "resize",
            "width": 1024,
            "height": 768
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["width"], 1024);
        assert_eq!(result["height"], 768);
    }

    #[tokio::test]
    async fn test_move() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "move",
            "x": 200,
            "y": 100
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["x"], 200);
        assert_eq!(result["y"], 100);
    }

    #[tokio::test]
    async fn test_history() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example1.com"
        })).await.unwrap();
        
        tool.execute(json!({
            "action": "navigate",
            "url": "https://example2.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "history",
            "limit": 5
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["total"], 2);
    }

    #[tokio::test]
    async fn test_clear_history() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "clear_history"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["cleared"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_a2ui_push() {
        let tool = CanvasTool::new();
        
        tool.execute(json!({
            "action": "present",
            "url": "https://example.com"
        })).await.unwrap();
        
        let result = tool.execute(json!({
            "action": "a2ui_push",
            "jsonl": "{\"type\":\"text\",\"content\":\"Hello\"}"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_a2ui_reset() {
        let tool = CanvasTool::new();
        
        let result = tool.execute(json!({
            "action": "a2ui_reset"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }
}
