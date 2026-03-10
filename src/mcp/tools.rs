// MCP 工具系统

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{McpError, McpResult};

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 输入参数 schema (JSON Schema)
    pub input_schema: serde_json::Value,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具名称
    pub name: String,
    /// 调用参数
    pub arguments: serde_json::Value,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 结果内容
    pub content: Vec<ToolContent>,
    /// 是否错误
    pub is_error: bool,
}

/// 工具内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// 文本内容
    #[serde(rename = "text")]
    Text { text: String },
    /// 图片内容
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    /// 资源内容
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        mime_type: Option<String>,
        text: Option<String>,
        blob: Option<String>,
    },
}

/// 工具注册表
pub struct ToolRegistry {
    /// 工具列表
    tools: Arc<RwLock<HashMap<String, ToolMetadata>>>,
}

impl ToolRegistry {
    /// 创建新的工具注册表
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册工具
    pub async fn register(&self, metadata: ToolMetadata) -> McpResult<()> {
        let mut tools = self.tools.write().await;
        tools.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    /// 注销工具
    pub async fn unregister(&self, name: &str) -> McpResult<()> {
        let mut tools = self.tools.write().await;
        tools.remove(name)
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))?;
        Ok(())
    }

    /// 列出所有工具
    pub async fn list_tools(&self) -> Vec<ToolMetadata> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// 获取工具元数据
    pub async fn get_tool(&self, name: &str) -> McpResult<ToolMetadata> {
        let tools = self.tools.read().await;
        tools.get(name)
            .cloned()
            .ok_or_else(|| McpError::ToolNotFound(name.to_string()))
    }

    /// 调用工具
    pub async fn call_tool(&self, call: ToolCall) -> McpResult<ToolResult> {
        // 验证工具是否存在
        let metadata = self.get_tool(&call.name).await?;

        // 验证参数是否符合 schema
        Self::validate_arguments(&call.arguments, &metadata.input_schema)?;

        // 执行工具
        match self.execute_tool(&call.name, &call.arguments).await {
            Ok(result) => Ok(ToolResult {
                content: vec![ToolContent::Text { text: result }],
                is_error: false,
            }),
            Err(e) => Ok(ToolResult {
                content: vec![ToolContent::Text {
                    text: format!("Error executing tool '{}': {}", call.name, e),
                }],
                is_error: true,
            }),
        }
    }

    /// 执行工具（实际逻辑）
    async fn execute_tool(&self, name: &str, arguments: &serde_json::Value) -> Result<String, McpError> {
        match name {
            // 示例工具：获取当前时间
            "get_current_time" => {
                let now = chrono::Local::now();
                Ok(format!("Current time: {}", now.format("%Y-%m-%d %H:%M:%S")))
            }

            // 示例工具：echo
            "echo" => {
                if let Some(text) = arguments.get("text").and_then(|v| v.as_str()) {
                    Ok(text.to_string())
                } else {
                    Err(McpError::InvalidArguments("Missing 'text' argument".to_string()))
                }
            }

            // 示例工具：计算表达式
            "calculate" => {
                if let Some(expr) = arguments.get("expression").and_then(|v| v.as_str()) {
                    // 简单的计算（仅支持加法）
                    // 实际应用中应该使用更安全的表达式求值器
                    if let Some(numbers) = arguments.get("numbers").and_then(|v| v.as_array()) {
                        let sum: i64 = numbers.iter()
                            .filter_map(|v| v.as_i64())
                            .sum();
                        Ok(format!("Result: {}", sum))
                    } else {
                        Err(McpError::InvalidArguments("Invalid 'numbers' argument".to_string()))
                    }
                } else {
                    Err(McpError::InvalidArguments("Missing 'expression' argument".to_string()))
                }
            }

            // 未实现的工具
            _ => Err(McpError::ExecutionError(format!(
                "Tool '{}' is not implemented yet",
                name
            ))),
        }
    }

    /// 验证参数是否符合 JSON Schema
    fn validate_arguments(arguments: &serde_json::Value, schema: &serde_json::Value) -> McpResult<()> {
        // 基本验证：检查是否为对象
        if !arguments.is_object() {
            return Err(McpError::InvalidArguments(
                "Arguments must be a JSON object".to_string()
            ));
        }

        // 检查 required 字段
        if let Some(required) = schema.get("required").and_then(|v| v.as_array()) {
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if !arguments.get(field_name).is_some() {
                        return Err(McpError::InvalidArguments(format!(
                            "Missing required field: '{}'",
                            field_name
                        )));
                    }
                }
            }
        }

        // 检查属性类型
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            for (prop_name, prop_schema) in properties {
                if let Some(arg_value) = arguments.get(prop_name) {
                    if let Some(expected_type) = prop_schema.get("type").and_then(|v| v.as_str()) {
                        Self::validate_type(prop_name, arg_value, expected_type)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 验证值类型
    fn validate_type(name: &str, value: &serde_json::Value, expected_type: &str) -> McpResult<()> {
        let is_valid = match expected_type {
            "string" => value.is_string(),
            "number" | "integer" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => true, // 未知类型，跳过验证
        };

        if !is_valid {
            return Err(McpError::InvalidArguments(format!(
                "Field '{}' must be of type '{}', got {:?}",
                name,
                expected_type,
                value
            )));
        }

        Ok(())
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

    #[tokio::test]
    async fn test_tool_registry() {
        let registry = ToolRegistry::new();

        // 注册工具
        let metadata = ToolMetadata {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": {"type": "string"}
                }
            }),
        };

        registry.register(metadata).await.unwrap();

        // 列出工具
        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");

        // 获取工具
        let tool = registry.get_tool("test_tool").await.unwrap();
        assert_eq!(tool.name, "test_tool");

        // 注销工具
        registry.unregister("test_tool").await.unwrap();
        let tools = registry.list_tools().await;
        assert_eq!(tools.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_not_found() {
        let registry = ToolRegistry::new();
        let result = registry.get_tool("non_existent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_call_get_current_time() {
        let registry = ToolRegistry::new();

        // 注册 get_current_time 工具
        let metadata = ToolMetadata {
            name: "get_current_time".to_string(),
            description: "Get current time".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
            }),
        };
        registry.register(metadata).await.unwrap();

        // 调用工具
        let call = ToolCall {
            name: "get_current_time".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = registry.call_tool(call).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolContent::Text { text } => assert!(text.contains("Current time")),
            _ => panic!("Expected Text content"),
        }
    }

    #[tokio::test]
    async fn test_tool_call_echo() {
        let registry = ToolRegistry::new();

        // 注册 echo 工具
        let metadata = ToolMetadata {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                },
                "required": ["text"],
            }),
        };
        registry.register(metadata).await.unwrap();

        // 调用工具
        let call = ToolCall {
            name: "echo".to_string(),
            arguments: serde_json::json!({"text": "Hello, MCP!"}),
        };
        let result = registry.call_tool(call).await.unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Text { text } => assert_eq!(text, "Hello, MCP!"),
            _ => panic!("Expected Text content"),
        }
    }

    #[tokio::test]
    async fn test_tool_call_missing_required_argument() {
        let registry = ToolRegistry::new();

        // 注册需要参数的工具
        let metadata = ToolMetadata {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                },
                "required": ["text"],
            }),
        };
        registry.register(metadata).await.unwrap();

        // 调用工具时缺少必需参数
        let call = ToolCall {
            name: "echo".to_string(),
            arguments: serde_json::json!({}),  // 缺少 text
        };
        let result = registry.call_tool(call).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_call_invalid_type() {
        let registry = ToolRegistry::new();

        // 注册需要字符串参数的工具
        let metadata = ToolMetadata {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                },
                "required": ["text"],
            }),
        };
        registry.register(metadata).await.unwrap();

        // 调用工具时传递错误类型
        let call = ToolCall {
            name: "echo".to_string(),
            arguments: serde_json::json!({"text": 123}),  // 应该是字符串
        };
        let result = registry.call_tool(call).await;
        assert!(result.is_err());
    }
}
