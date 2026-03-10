// MCP (Model Context Protocol) - Week 2
//
// Model Context Protocol 是一个开放协议，
// 用于连接 AI 模型与外部工具、资源和提示词。
//
// 核心能力：
// 1. 工具 (Tools): 可执行的函数
// 2. 资源 (Resources): 数据访问接口
// 3. 提示词 (Prompts): 模板化提示词

pub mod client;
pub mod protocol;
pub mod transport;
pub mod tools;
pub mod resources;
pub mod prompts;
pub mod error;

// 重新导出核心类型
pub use client::McpClient;
pub use protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcId, JsonRpcNotification};
pub use error::{McpError, McpResult, JsonRpcError};
pub use tools::{ToolRegistry, ToolMetadata, ToolCall};
pub use resources::{ResourceRegistry, ResourceMetadata, ResourceContent};
pub use prompts::{PromptRegistry, PromptMetadata, PromptTemplate};

/// MCP 协议版本
pub const MCP_VERSION: &str = "2024-11-05";

/// JSON-RPC 版本
pub const JSONRPC_VERSION: &str = "2.0";
