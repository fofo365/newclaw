// 工具系统模块
pub mod error;
pub mod executor;
pub mod files;
pub mod permissions;
pub mod registry;
pub mod web;
pub mod exec;
pub mod browser;
pub mod canvas;
pub mod memory;
pub mod sessions;
pub mod subagents;
pub mod nodes;
pub mod feishu;
pub mod tts;
pub mod orchestrator;
pub mod permission;
pub mod init;

pub use error::ToolError;
pub use executor::ToolExecutor;
pub use files::{EditTool, ReadTool, WriteTool};
pub use permissions::PermissionManager;
pub use registry::ToolRegistry;
pub use web::{WebFetchTool, WebSearchTool};
pub use exec::ExecTool;
pub use browser::BrowserTool;
pub use canvas::CanvasTool;
pub use memory::MemoryTool;
pub use sessions::{SessionsTool, SessionStore, SessionInfo};
pub use subagents::{SubagentsTool, SubagentStore, SubagentInfo};
pub use nodes::{NodesTool, NodeStore, NodeInfo, NodeType, NodeStatus, NodeCapability};
pub use feishu::{FeishuDocTool, FeishuBitableTool, FeishuDriveTool, FeishuWikiTool, FeishuChatTool};
pub use tts::TtsTool;
pub use orchestrator::{ToolOrchestrator, OrchestrationPlan, ToolStep, ErrorHandling};
pub use permission::{PermissionTool, ChannelConfigTool};
pub use init::init_builtin_tools;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// 工具 trait 定义
#[async_trait]
pub trait Tool: Send + Sync {
    /// 工具元数据
    fn metadata(&self) -> ToolMetadata;

    /// 执行工具
    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue>;
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub parameters: JsonValue,
}

/// 工具执行结果类型（简写）
pub type Value = JsonValue;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        let metadata = ToolMetadata {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string"
                    }
                }
            }),
        };

        assert_eq!(metadata.name, "test_tool");
        assert!(metadata.parameters.is_object());
    }
}
