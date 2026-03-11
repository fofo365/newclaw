// 工具初始化模块
use crate::tools::{MemoryTool, ToolRegistry};
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

    info!("✅ 内置工具初始化完成");
    Ok(())
}
