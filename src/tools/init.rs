// 工具初始化模块
use crate::tools::{MemoryTool, ToolRegistry, BrowserTool, CanvasTool, SessionsTool, SubagentsTool, NodesTool};
use std::path::PathBuf;
use tracing::info;

/// 初始化内置工具
pub async fn init_builtin_tools(registry: &ToolRegistry, data_dir: PathBuf, openclaw_workspace: PathBuf) -> anyhow::Result<()> {
    // 注册记忆工具
    let memory_tool = MemoryTool::new(
        data_dir.join("memory"),
        openclaw_workspace,
    );
    registry.register(memory_tool).await?;

    // 注册浏览器工具
    let browser_tool = BrowserTool::new();
    registry.register(browser_tool).await?;

    // 注册 Canvas 工具
    let canvas_tool = CanvasTool::new();
    registry.register(canvas_tool).await?;

    // 注册会话管理工具
    let sessions_tool = SessionsTool::new();
    registry.register(sessions_tool).await?;

    // 注册子代理管理工具
    let subagents_tool = SubagentsTool::new();
    registry.register(subagents_tool).await?;

    // 注册节点管理工具
    let nodes_tool = NodesTool::new();
    registry.register(nodes_tool).await?;

    info!("✅ 内置工具初始化完成 (memory + browser + canvas + sessions + subagents + nodes)");
    Ok(())
}
