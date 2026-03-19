// 工具初始化模块
//
// 注册所有内置工具到统一的 ToolRegistry
// 所有通道层共享此工具系统

use crate::tools::{
    // 文件工具
    ReadTool, WriteTool, EditTool,
    // Web 工具
    WebSearchTool, WebFetchTool,
    // 执行工具
    ExecTool,
    // 其他工具
    ToolRegistry, BrowserTool, CanvasTool,
    SessionsTool, SubagentsTool, NodesTool,
    FeishuDocTool, FeishuBitableTool, FeishuDriveTool, FeishuWikiTool, FeishuChatTool,
    TtsTool, PermissionTool, ChannelConfigTool,
    // 新增工具
    diagnostic::DiagnosticTool,
    cron::CronTool,
    workflow::WorkflowTool,
    safety::SafetyTool,
    channel_permission::ChannelPermissionTool,
    memory_archive::MemoryArchiveTool,
    report::ReportTool,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use crate::channel::ChannelPermission;

/// 初始化所有内置工具
///
/// 此函数注册所有可用工具到 ToolRegistry，确保所有通道层（CLI、飞书、Dashboard等）
/// 都能访问相同的工具集
pub async fn init_builtin_tools(
    registry: &ToolRegistry,
    data_dir: PathBuf,
    openclaw_workspace: PathBuf,
) -> anyhow::Result<()> {
    init_builtin_tools_with_permissions(
        registry,
        data_dir,
        openclaw_workspace,
        None,
    ).await
}

/// 初始化所有内置工具（带权限管理）
pub async fn init_builtin_tools_with_permissions(
    registry: &ToolRegistry,
    data_dir: PathBuf,
    openclaw_workspace: PathBuf,
    permissions: Option<Arc<ChannelPermission>>,
) -> anyhow::Result<()> {
    let mut tool_count = 0;

    // ==================== 文件工具 ====================
    
    // 文件读取
    registry.register(ReadTool::new()).await?;
    tool_count += 1;
    
    // 文件写入
    registry.register(WriteTool::new()).await?;
    tool_count += 1;
    
    // 文件编辑
    registry.register(EditTool::new()).await?;
    tool_count += 1;

    // ==================== Web 工具 ====================
    
    // 网络搜索
    registry.register(WebSearchTool::new()).await?;
    tool_count += 1;
    
    // 网页抓取
    registry.register(WebFetchTool).await?;
    tool_count += 1;

    // ==================== 执行工具 ====================
    
    // 命令执行
    registry.register(ExecTool::new()).await?;
    tool_count += 1;

    // ==================== 浏览器工具 ====================
    
    registry.register(BrowserTool::new()).await?;
    tool_count += 1;

    // ==================== Canvas 工具 ====================
    
    registry.register(CanvasTool::new()).await?;
    tool_count += 1;

    // ==================== 会话/代理工具 ====================
    
    registry.register(SessionsTool::new()).await?;
    tool_count += 1;
    
    registry.register(SubagentsTool::new()).await?;
    tool_count += 1;

    // ==================== 节点工具 ====================
    
    registry.register(NodesTool::new()).await?;
    tool_count += 1;

    // ==================== 飞书工具 ====================
    
    registry.register(FeishuDocTool::new()).await?;
    tool_count += 1;
    
    registry.register(FeishuBitableTool::new()).await?;
    tool_count += 1;
    
    registry.register(FeishuDriveTool::new()).await?;
    tool_count += 1;
    
    registry.register(FeishuWikiTool::new()).await?;
    tool_count += 1;
    
    registry.register(FeishuChatTool::new()).await?;
    tool_count += 1;

    // ==================== TTS 工具 ====================
    
    registry.register(TtsTool::new()).await?;
    tool_count += 1;

    // ==================== 诊断和系统管理工具 ====================
    
    // 系统诊断工具
    registry.register(DiagnosticTool::new()).await?;
    tool_count += 1;
    
    // Cron 任务管理工具
    registry.register(CronTool::new()).await?;
    tool_count += 1;
    
    // 诊断工作流工具
    registry.register(WorkflowTool::new()).await?;
    tool_count += 1;
    
    // AI 行为约束保护工具
    registry.register(SafetyTool::new()).await?;
    tool_count += 1;
    
    // 通道权限配置工具
    registry.register(ChannelPermissionTool::new()).await?;
    tool_count += 1;
    
    // 记忆归档恢复工具
    registry.register(MemoryArchiveTool::new()).await?;
    tool_count += 1;
    
    // 诊断报告生成工具
    registry.register(ReportTool::new()).await?;
    tool_count += 1;

    // ==================== 权限和配置工具 ====================
    
    if let Some(perm) = permissions {
        // 权限管理工具
        registry.register(PermissionTool::new(Arc::clone(&perm))).await?;
        tool_count += 1;
        
        // 通道配置工具
        registry.register(ChannelConfigTool::new(perm)).await?;
        tool_count += 1;
    }

    info!(
        "✅ 内置工具初始化完成: {} 个工具 (files + web + exec + browser + canvas + sessions + nodes + feishu + tts + permission)",
        tool_count
    );
    
    Ok(())
}