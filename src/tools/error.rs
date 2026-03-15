// 工具错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("工具不存在: {0}")]
    NotFound(String),
    
    #[error("参数验证失败: {0}")]
    InvalidArguments(String),
    
    #[error("权限不足: {0}")]
    PermissionDenied(String),
    
    #[error("执行失败: {0}")]
    ExecutionFailed(String),
    
    #[error("超时: {0}")]
    Timeout(String),
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("JSON 错误: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("未知错误: {0}")]
    Unknown(String),
}

pub type ToolResult<T> = Result<T, ToolError>;
