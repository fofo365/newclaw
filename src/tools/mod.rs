// NewClaw v0.3.0 - 工具执行引擎
//
// 核心设计：
// 1. 统一的 Tool trait 抽象
// 2. 100% 测试覆盖率
// 3. 完善的错误处理
// 4. 重试机制

pub mod registry;
pub mod builtin;

// Re-exports from submodules
pub use registry::{Tool, ToolRegistry};
pub use builtin::{ReadTool, WriteTool, EditTool, ExecTool, SearchTool};

// Core types
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// 文本内容
    pub content: String,
    
    /// 媒体附件（图片、文件等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<Vec<Media>>,
    
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// 元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl ToolOutput {
    /// 成功结果
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            media: None,
            error: None,
            metadata: None,
        }
    }
    
    /// 带元数据的结果
    pub fn with_metadata(content: impl Into<String>, metadata: HashMap<String, serde_json::Value>) -> Self {
        Self {
            content: content.into(),
            media: None,
            error: None,
            metadata: Some(metadata),
        }
    }
    
    /// 错误结果
    pub fn error(error: impl Into<String>) -> Self {
        Self {
            content: String::new(),
            media: None,
            error: Some(error.into()),
            metadata: None,
        }
    }
    
    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// 媒体附件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    pub media_type: MediaType,
    pub url: String,
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Image,
    Video,
    Audio,
    File,
}

/// 工具描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 工具执行错误
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type ToolResult<T> = Result<T, ToolError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_output_success() {
        let output = ToolOutput::success("Hello, World!");
        assert!(output.is_success());
        assert_eq!(output.content, "Hello, World!");
        assert!(output.error.is_none());
    }
    
    #[test]
    fn test_tool_output_error() {
        let output = ToolOutput::error("Something went wrong");
        assert!(!output.is_success());
        assert_eq!(output.error, Some("Something went wrong".to_string()));
    }
}
