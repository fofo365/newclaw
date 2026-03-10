// MCP 集成测试

use newclaw::mcp::{
    client::McpClient,
    protocol::{JsonRpcRequest, JsonRpcResponse, JsonRpcId},
    tools::{ToolCall, ToolMetadata, ToolRegistry},
    resources::{ResourceContent, ResourceMetadata, ResourceRegistry},
    prompts::{PromptArgument, PromptMetadata, PromptRegistry},
    transport::{StdioTransport, Transport},
    McpError,
};
use serde_json::json;
use std::time::Duration;

/// 测试完整的 MCP 工具调用流程
#[tokio::test]
async fn test_tool_call_flow() {
    // 创建工具注册表
    let registry = ToolRegistry::new();

    // 注册测试工具
    let metadata = ToolMetadata {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Test message"
                }
            },
            "required": ["message"]
        }),
    };

    registry.register(metadata).await.unwrap();

    // 列出工具
    let tools = registry.list_tools().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "test_tool");

    // 调用工具
    let call = ToolCall {
        name: "test_tool".to_string(),
        arguments: json!({"message": "Hello, MCP!"}),
    };

    let result = registry.call_tool(call).await.unwrap();
    assert!(!result.is_error);
    assert_eq!(result.content.len(), 1);
}

/// 测试完整的 MCP 资源访问流程
#[tokio::test]
async fn test_resource_flow() {
    // 创建资源注册表
    let registry = ResourceRegistry::new();

    // 注册测试资源
    let metadata = ResourceMetadata {
        uri: "file:///test.txt".to_string(),
        name: "test_file".to_string(),
        description: "A test file".to_string(),
        mime_type: Some("text/plain".to_string()),
    };

    registry.register(metadata).await.unwrap();

    // 列出资源
    let resources = registry.list_resources().await;
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "file:///test.txt");

    // 读取资源
    let content = registry.read_resource("file:///test.txt").await.unwrap();
    assert_eq!(content.uri, "file:///test.txt");
    assert_eq!(content.mime_type, Some("text/plain".to_string()));
}

/// 测试完整的 MCP 提示词模板流程
#[tokio::test]
async fn test_prompt_flow() {
    // 创建提示词注册表
    let registry = PromptRegistry::new();

    // 注册测试提示词
    let metadata = PromptMetadata {
        name: "test_prompt".to_string(),
        description: "A test prompt".to_string(),
        arguments: vec![
            PromptArgument {
                name: "topic".to_string(),
                description: "The topic to write about".to_string(),
                required: true,
            },
            PromptArgument {
                name: "style".to_string(),
                description: "Writing style".to_string(),
                required: false,
            },
        ],
    };

    registry.register(metadata).await.unwrap();

    // 列出提示词
    let prompts = registry.list_prompts().await;
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].name, "test_prompt");

    // 获取提示词模板（带参数）
    let mut args = std::collections::HashMap::new();
    args.insert("topic".to_string(), "AI development".to_string());
    args.insert("style".to_string(), "technical".to_string());

    let template = registry.get_prompt_template("test_prompt", args).await.unwrap();
    assert_eq!(template.name, "test_prompt");
    assert!(!template.messages.is_empty());
}

/// 测试参数验证
#[tokio::test]
async fn test_argument_validation() {
    let registry = ToolRegistry::new();

    let metadata = ToolMetadata {
        name: "validated_tool".to_string(),
        description: "A tool with argument validation".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "required_param": {"type": "string"}
            },
            "required": ["required_param"]
        }),
    };

    registry.register(metadata).await.unwrap();

    // 测试有效参数
    let valid_call = ToolCall {
        name: "validated_tool".to_string(),
        arguments: json!({"required_param": "test"}),
    };
    assert!(registry.call_tool(valid_call).await.is_ok());

    // 测试无效参数（非对象）
    let invalid_call = ToolCall {
        name: "validated_tool".to_string(),
        arguments: json!("not an object"),
    };
    assert!(registry.call_tool(invalid_call).await.is_err());
}

/// 测试提示词参数验证
#[tokio::test]
async fn test_prompt_argument_validation() {
    let registry = PromptRegistry::new();

    let metadata = PromptMetadata {
        name: "validated_prompt".to_string(),
        description: "A prompt with required arguments".to_string(),
        arguments: vec![
            PromptArgument {
                name: "required_arg".to_string(),
                description: "Required argument".to_string(),
                required: true,
            },
        ],
    };

    registry.register(metadata).await.unwrap();

    // 测试缺少必需参数
    let incomplete_args = std::collections::HashMap::new();
    assert!(registry.get_prompt_template("validated_prompt", incomplete_args).await.is_err());

    // 测试完整参数
    let mut complete_args = std::collections::HashMap::new();
    complete_args.insert("required_arg".to_string(), "value".to_string());
    assert!(registry.get_prompt_template("validated_prompt", complete_args).await.is_ok());
}

/// 测试 Stdio 传输层
#[tokio::test]
async fn test_stdio_transport() {
    // 注意：这个测试需要一个真正的 MCP 服务器进程
    // 这里我们测试传输层的基本功能

    let transport = StdioTransport::new();

    // 测试传输层创建
    assert!(transport.is_connected().await);

    // 测试发送请求（会失败，因为没有真正的服务器）
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: JsonRpcId::Number(1),
        method: "test".to_string(),
        params: None,
    };

    // 在实际测试中，这里会启动一个 mock MCP 服务器
    // 然后验证请求和响应

    println!("Stdio transport test completed");
}

/// 测试 JSON-RPC 协议
#[test]
fn test_jsonrpc_protocol() {
    // 测试请求序列化
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: JsonRpcId::Number(1),
        method: "tools/list".to_string(),
        params: None,
    };

    let serialized = serde_json::to_string(&request).unwrap();
    assert!(serialized.contains("\"jsonrpc\":\"2.0\""));
    assert!(serialized.contains("\"method\":\"tools/list\""));

    // 测试响应反序列化
    let response_json = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
    let response: JsonRpcResponse = serde_json::from_str(response_json).unwrap();

    assert_eq!(response.id, JsonRpcId::Number(1));
    assert!(response.result.is_some());
}

/// 测试错误处理
#[tokio::test]
async fn test_error_handling() {
    let registry = ToolRegistry::new();

    // 测试工具不存在错误
    let call = ToolCall {
        name: "nonexistent_tool".to_string(),
        arguments: json!({}),
    };

    let result = registry.call_tool(call).await;
    assert!(result.is_err());

    if let Err(McpError::ToolNotFound(name)) = result {
        assert_eq!(name, "nonexistent_tool");
    } else {
        panic!("Expected ToolNotFound error");
    }

    // 测试资源不存在错误
    let resource_registry = ResourceRegistry::new();
    let result = resource_registry.read_resource("file:///nonexistent.txt").await;
    assert!(result.is_err());

    // 测试提示词不存在错误
    let prompt_registry = PromptRegistry::new();
    let result = prompt_registry.get_prompt("nonexistent_prompt").await;
    assert!(result.is_err());
}

/// 测试并发访问
#[tokio::test]
async fn test_concurrent_access() {
    let registry = std::sync::Arc::new(ToolRegistry::new());

    // 注册多个工具
    for i in 0..10 {
        let metadata = ToolMetadata {
            name: format!("tool_{}", i),
            description: format!("Tool number {}", i),
            input_schema: json!({"type": "object"}),
        };
        registry.register(metadata).await.unwrap();
    }

    // 并发调用工具
    let mut handles = Vec::new();
    for i in 0..10 {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            let call = ToolCall {
                name: format!("tool_{}", i),
                arguments: json!({}),
            };
            registry_clone.call_tool(call).await
        });
        handles.push(handle);
    }

    // 等待所有调用完成
    for handle in handles {
        assert!(handle.await.unwrap().is_ok());
    }
}

/// 测试性能基准
#[tokio::test]
async fn test_performance_benchmark() {
    let registry = ToolRegistry::new();

    let metadata = ToolMetadata {
        name: "benchmark_tool".to_string(),
        description: "Tool for performance testing".to_string(),
        input_schema: json!({"type": "object"}),
    };

    registry.register(metadata).await.unwrap();

    let start = std::time::Instant::now();

    // 执行 1000 次工具调用
    for _ in 0..1000 {
        let call = ToolCall {
            name: "benchmark_tool".to_string(),
            arguments: json!({}),
        };
        registry.call_tool(call).await.unwrap();
    }

    let duration = start.elapsed();

    println!("1000 tool calls took {:?}", duration);
    assert!(duration.as_millis() < 1000, "Performance test should complete in less than 1 second");
}
