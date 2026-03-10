// NewClaw 工具执行引擎
// 提供与 OpenClaw 相当的工具能力

pub mod registry;
pub mod executor;
pub mod permissions;
pub mod error;
pub mod files;
pub mod exec;

// 重新导出主要类型
pub use registry::ToolRegistry;
pub use executor::ToolExecutor;
pub use permissions::PermissionManager;
pub use error::{ToolError, ToolResult};

// 文件操作工具
pub use files::{ReadTool, WriteTool, EditTool};

// Shell 执行工具
pub use exec::{ExecTool, ProcessTool};
pub use crate::tools::exec::ProcessManager;

// MCP 工具类型（与 MCP 层兼容）
pub use crate::mcp::tools::{ToolMetadata, ToolCall, ToolResult as McpToolResult, ToolContent};

/// 工具 trait - 所有工具必须实现
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// 工具元数据
    fn metadata(&self) -> ToolMetadata;
    
    /// 执行工具
    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}
