// NewClaw v0.3.0 - 工具注册表
//
// 核心功能：
// 1. 工具注册和发现
// 2. 工具执行调度
// 3. 参数验证
// 4. 重试机制

use super::{ToolOutput, ToolError, ToolDescription};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 工具执行结果类型别名
pub type ToolResult<T> = Result<T, super::ToolError>;

/// 工具注册表
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册工具
    pub async fn register(&self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        let mut tools = self.tools.write().await;
        tools.insert(name, tool);
    }
    
    /// 批量注册
    pub async fn register_all(&self, tools: Vec<Arc<dyn Tool>>) {
        for tool in tools {
            self.register(tool).await;
        }
    }
    
    /// 列出所有工具
    pub async fn list(&self) -> Vec<ToolDescription> {
        let tools = self.tools.read().await;
        tools.values().map(|tool| ToolDescription {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: tool.parameters(),
        }).collect()
    }
    
    /// 执行工具（带重试）
    pub async fn execute(&self, name: &str, params: serde_json::Value) -> ToolResult<ToolOutput> {
        let tool = {
            let tools = self.tools.read().await;
            tools.get(name)
                .ok_or_else(|| ToolError::NotFound(name.to_string()))?
                .clone()
        };
        
        // 尝试执行，最多重试 2 次
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            attempts += 1;
            
            match tool.execute(params.clone()).await {
                Ok(output) => return Ok(output),
                Err(e) if attempts < max_attempts => {
                    tracing::warn!("Tool {} failed (attempt {}/{}): {}", name, attempts, max_attempts, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100 * attempts as u64)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    /// 检查工具是否存在
    pub async fn exists(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 工具 trait
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;
    
    /// 工具描述
    fn description(&self) -> &str;
    
    /// 参数 schema (JSON Schema)
    fn parameters(&self) -> serde_json::Value;
    
    /// 执行工具
    async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockTool {
        name: String,
    }
    
    #[async_trait::async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn description(&self) -> &str {
            "Mock tool for testing"
        }
        
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {"type": "string"}
                }
            })
        }
        
        async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success(format!("Executed with: {:?}", params)))
        }
    }
    
    #[tokio::test]
    async fn test_register_and_execute() {
        let registry = ToolRegistry::new();
        let tool = Arc::new(MockTool { name: "mock".to_string() });
        
        registry.register(tool).await;
        
        assert!(registry.exists("mock").await);
        
        let output = registry.execute("mock", serde_json::json!({"input": "test"})).await.unwrap();
        assert!(output.is_success());
    }
    
    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        
        let result = registry.execute("nonexistent", serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::NotFound(_))));
    }
    
    #[tokio::test]
    async fn test_list_tools() {
        let registry = ToolRegistry::new();
        
        let tool1 = Arc::new(MockTool { name: "tool1".to_string() });
        let tool2 = Arc::new(MockTool { name: "tool2".to_string() });
        
        registry.register_all(vec![tool1, tool2]).await;
        
        let tools = registry.list().await;
        assert_eq!(tools.len(), 2);
    }
}
