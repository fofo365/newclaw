// 工具初始化

use std::sync::Arc;
use super::ToolRegistry;
use super::files::{ReadTool, WriteTool, EditTool};
use super::web::{WebFetchTool, WebSearchTool};
use super::exec::ExecTool;
use super::browser::BrowserTool;
use super::canvas::CanvasTool;

/// 初始化所有内置工具
pub async fn init_builtin_tools(registry: &Arc<ToolRegistry>) -> anyhow::Result<()> {
    // 文件工具
    registry.register(ReadTool::new()).await?;
    registry.register(WriteTool::new()).await?;
    registry.register(EditTool::new()).await?;
    
    // 网络工具
    registry.register(WebFetchTool).await?;
    registry.register(WebSearchTool::new()).await?;
    
    // 执行工具
    registry.register(ExecTool::new()).await?;
    
    // 浏览器工具
    registry.register(BrowserTool::new()).await?;
    
    // Canvas 工具
    registry.register(CanvasTool::new()).await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_builtin_tools() {
        let registry = Arc::new(ToolRegistry::new());
        init_builtin_tools(&registry).await.unwrap();
        
        let tools = registry.list_tools().await;
        assert!(tools.len() >= 8, "Should have at least 8 tools");
        
        // 检查关键工具
        assert!(registry.get_tool("read").await.is_some());
        assert!(registry.get_tool("write").await.is_some());
        assert!(registry.get_tool("web_fetch").await.is_some());
        assert!(registry.get_tool("browser").await.is_some());
        assert!(registry.get_tool("canvas").await.is_some());
    }
}
