// 工具执行器

use std::sync::Arc;
use std::time::Duration;

use super::{ToolRegistry, ToolError, ToolResult, ToolCall, ToolContent};
use crate::mcp::tools::ToolResult as McpToolResult;

/// 工具执行器
pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    timeout: Duration,
}

impl ToolExecutor {
    /// 创建新的工具执行器
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self {
            registry,
            timeout: Duration::from_secs(30),
        }
    }
    
    /// 设置超时
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    
    /// 执行工具调用
    pub async fn execute(&self, call: ToolCall) -> ToolResult<McpToolResult> {
        // 执行工具（带超时）
        let result = tokio::time::timeout(
            self.timeout,
            self.registry.call(&call.name, call.arguments)
        )
        .await
        .map_err(|_| ToolError::Timeout(format!("工具 {} 执行超时", call.name)))?
        .map_err(|e| e)?;
        
        // 转换为 MCP 工具结果
        Ok(McpToolResult {
            content: vec![ToolContent::Text {
                text: serde_json::to_string(&result).unwrap_or_else(|_| "{}".to_string()),
            }],
            is_error: false,
        })
    }
    
    /// 批量执行工具调用
    pub async fn execute_batch(&self, calls: Vec<ToolCall>) -> Vec<ToolResult<McpToolResult>> {
        let mut results = Vec::with_capacity(calls.len());
        
        for call in calls {
            results.push(self.execute(call).await);
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_executor_creation() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(registry);
        
        assert_eq!(executor.timeout, Duration::from_secs(30));
    }
    
    #[tokio::test]
    async fn test_executor_with_timeout() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(registry)
            .with_timeout(Duration::from_secs(60));
        
        assert_eq!(executor.timeout, Duration::from_secs(60));
    }
}
