// MCP 客户端

use std::sync::Arc;
use async_trait::async_trait;

use super::{
    error::{McpError, McpResult},
    protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcId},
    transport::Transport,
    tools::{ToolRegistry, ToolMetadata, ToolCall, ToolResult},
    resources::{ResourceRegistry, ResourceMetadata, ResourceContent},
    prompts::{PromptRegistry, PromptMetadata, PromptTemplate},
};
use std::collections::HashMap;

/// MCP 客户端
pub struct McpClient {
    /// 传输层
    transport: Box<dyn Transport>,
    /// 工具注册表
    tools: Arc<ToolRegistry>,
    /// 资源注册表
    resources: Arc<ResourceRegistry>,
    /// 提示词注册表
    prompts: Arc<PromptRegistry>,
}

impl McpClient {
    /// 创建新的 MCP 客户端
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            transport,
            tools: Arc::new(ToolRegistry::new()),
            resources: Arc::new(ResourceRegistry::new()),
            prompts: Arc::new(PromptRegistry::new()),
        }
    }

    /// 初始化客户端
    pub async fn initialize(&self) -> McpResult<InitializeResult> {
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(1),
            "initialize"
        ).with_params(serde_json::json!({
            "protocolVersion": super::MCP_VERSION,
            "capabilities": {
                "tools": {},
                "resources": {},
                "prompts": {}
            }
        }));

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        let result = response.result.ok_or_else(|| {
            McpError::TransportError("No result in response".to_string())
        })?;

        let init_result: InitializeResult = serde_json::from_value(result)?;
        Ok(init_result)
    }

    /// 列出所有工具
    pub async fn list_tools(&self) -> McpResult<Vec<ToolMetadata>> {
        let request = JsonRpcRequest::new(JsonRpcId::Number(2), "tools/list");

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        // TODO: 解析工具列表
        Ok(self.tools.list_tools().await)
    }

    /// 调用工具
    pub async fn call_tool(&self, call: ToolCall) -> McpResult<ToolResult> {
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(3),
            "tools/call"
        ).with_params(serde_json::json!({
            "name": call.name,
            "arguments": call.arguments
        }));

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        // TODO: 解析工具结果
        Ok(ToolResult {
            content: vec![],
            is_error: false,
        })
    }

    /// 列出所有资源
    pub async fn list_resources(&self) -> McpResult<Vec<ResourceMetadata>> {
        let request = JsonRpcRequest::new(JsonRpcId::Number(4), "resources/list");

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        // TODO: 解析资源列表
        Ok(self.resources.list_resources().await)
    }

    /// 读取资源
    pub async fn read_resource(&self, uri: &str) -> McpResult<ResourceContent> {
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(5),
            "resources/read"
        ).with_params(serde_json::json!({
            "uri": uri
        }));

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        // TODO: 解析资源内容
        Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: None,
            text: None,
            blob: None,
        })
    }

    /// 列出所有提示词
    pub async fn list_prompts(&self) -> McpResult<Vec<PromptMetadata>> {
        let request = JsonRpcRequest::new(JsonRpcId::Number(6), "prompts/list");

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        // TODO: 解析提示词列表
        Ok(self.prompts.list_prompts().await)
    }

    /// 获取提示词模板
    pub async fn get_prompt(&self, name: &str, args: HashMap<String, String>) -> McpResult<PromptTemplate> {
        let request = JsonRpcRequest::new(
            JsonRpcId::Number(7),
            "prompts/get"
        ).with_params(serde_json::json!({
            "name": name,
            "arguments": args
        }));

        let response = self.transport.send_request(request).await?;

        if let Some(error) = response.error {
            return Err(McpError::JsonRpcError(error));
        }

        // TODO: 解析提示词模板
        Ok(PromptTemplate {
            name: name.to_string(),
            messages: vec![],
        })
    }

    /// 关闭客户端
    pub async fn close(&self) -> McpResult<()> {
        self.transport.close().await
    }
}

/// 初始化结果
#[derive(Debug, serde::Deserialize)]
pub struct InitializeResult {
    /// 协议版本
    pub protocolVersion: String,
    /// 服务器能力
    pub capabilities: ServerCapabilities,
    /// 服务器信息（可选）
    #[serde(default)]
    pub serverInfo: Option<ServerInfo>,
}

/// 服务器能力
#[derive(Debug, serde::Deserialize)]
pub struct ServerCapabilities {
    /// 工具支持
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
    /// 资源支持
    #[serde(default)]
    pub resources: Option<serde_json::Value>,
    /// 提示词支持
    #[serde(default)]
    pub prompts: Option<serde_json::Value>,
}

/// 服务器信息
#[derive(Debug, serde::Deserialize)]
pub struct ServerInfo {
    /// 服务器名称
    pub name: String,
    /// 服务器版本
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_client_create() {
        // TODO: 实现 Mock 传输层用于测试
    }
}
