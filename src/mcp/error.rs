// MCP 错误类型

use serde::{Deserialize, Serialize};
use std::fmt;

/// MCP 错误类型
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// JSON-RPC 错误
    #[error("JSON-RPC error: {0}")]
    JsonRpcError(JsonRpcError),

    /// 传输层错误
    #[error("Transport error: {0}")]
    TransportError(String),

    /// 工具未找到
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// 资源未找到
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// 提示词未找到
    #[error("Prompt not found: {0}")]
    PromptNotFound(String),

    /// 无效的参数
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// 执行错误
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// 序列化错误
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// IO 错误
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// 其他错误
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// MCP 结果类型
pub type McpResult<T> = Result<T, McpError>;

/// JSON-RPC 错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// 错误代码
    pub code: i32,
    /// 错误消息
    pub message: String,
    /// 错误数据（可选）
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// 解析错误 (-32700)
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    /// 无效请求 (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// 方法未找到 (-32601)
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    /// 无效参数 (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// 内部错误 (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
