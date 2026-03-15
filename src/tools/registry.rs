// 工具注册表

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value as JsonValue;

use super::{Tool, ToolMetadata};

/// 工具注册表
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册工具
    pub async fn register<T: Tool + 'static>(&self, tool: T) -> anyhow::Result<()> {
        let metadata = tool.metadata();
        let name = metadata.name.clone();
        
        let mut tools = self.tools.write().await;
        tools.insert(name, Arc::new(tool));
        
        Ok(())
    }
    
    /// 注销工具
    pub async fn unregister(&self, name: &str) -> anyhow::Result<()> {
        let mut tools = self.tools.write().await;
        let exists = tools.contains_key(name);
        tools.remove(name);

        if exists {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }
    
    /// 列出所有工具元数据
    pub async fn list_tools(&self) -> Vec<ToolMetadata> {
        let tools = self.tools.read().await;
        tools.values().map(|t| t.metadata()).collect()
    }
    
    /// 获取工具元数据
    pub async fn get_tool(&self, name: &str) -> Option<ToolMetadata> {
        let tools = self.tools.read().await;
        tools.get(name).map(|t| t.metadata())
    }
    
    /// 调用工具（内部方法）
    pub async fn call(&self, name: &str, args: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let tools = self.tools.read().await;
        let tool = tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        tool.execute(args).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestTool;
    
    #[async_trait::async_trait]
    impl Tool for TestTool {
        fn metadata(&self) -> ToolMetadata {
            ToolMetadata {
                name: "test".to_string(),
                description: "测试工具".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            }
        }
        
        async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<JsonValue> {
            Ok(serde_json::json!({"result": "ok"}))
        }
    }
    
    #[tokio::test]
    async fn test_register_tool() {
        let registry = ToolRegistry::new();
        registry.register(TestTool).await.unwrap();
        
        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test");
    }
    
    #[tokio::test]
    async fn test_call_tool() {
        let registry = ToolRegistry::new();
        registry.register(TestTool).await.unwrap();
        
        let result = registry.call("test", serde_json::json!({})).await.unwrap();
        assert_eq!(result["result"], "ok");
    }
    
    #[tokio::test]
    async fn test_unregister_tool() {
        let registry = ToolRegistry::new();
        registry.register(TestTool).await.unwrap();
        
        registry.unregister("test").await.unwrap();
        
        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 0);
    }
    
    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        
        let result = registry.call("nonexistent", serde_json::json!({})).await;
        assert!(result.is_err());
    }
}
