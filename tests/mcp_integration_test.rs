// MCP 集成测试

use newclaw::mcp::{
    tools::ToolCall,
    tools::ToolMetadata,
    tools::ToolRegistry,
    resources::ResourceMetadata,
    resources::ResourceRegistry,
    prompts::PromptMetadata,
    prompts::PromptRegistry,
};

// 工具发现测试
#[tokio::test]
async fn test_tool_discovery() {
    let registry = ToolRegistry::new();

    // 注册示例工具
    registry.register(ToolMetadata {
        name: "test_tool".to_string(),
        description: "测试工具".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            },
            "required": ["input"]
        }),
    }).await.unwrap();

    // 列出工具
    let tools = registry.list_tools().await;
    assert_eq!(tools.len(), 1, "应返回 1 个工具");
    assert_eq!(tools[0].name, "test_tool");
}

// 工具调用测试
#[tokio::test]
async fn test_tool_call() {
    let registry = ToolRegistry::new();

    // 注册示例工具
    registry.register(ToolMetadata {
        name: "get_current_time".to_string(),
        description: "获取当前时间".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {},
        }),
    }).await.unwrap();

    // 调用工具
    let call = ToolCall {
        name: "get_current_time".to_string(),
        arguments: serde_json::json!({}),
    };

    let result = registry.call_tool(call).await.unwrap();
    assert!(!result.is_error, "工具调用不应失败");
    assert!(!result.content.is_empty(), "结果内容不应为空");
}

// 工具调用参数验证测试
#[tokio::test]
async fn test_tool_call_argument_validation() {
    let registry = ToolRegistry::new();

    // 注册需要参数的工具
    registry.register(ToolMetadata {
        name: "echo".to_string(),
        description: "回显文本".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "text": {"type": "string"}
            },
            "required": ["text"]
        }),
    }).await.unwrap();

    // 测试缺少必需参数
    let call = ToolCall {
        name: "echo".to_string(),
        arguments: serde_json::json!({}),
    };

    let result = registry.call_tool(call).await;
    assert!(result.is_err(), "应返回参数验证错误");

    // 测试正确参数
    let call = ToolCall {
        name: "echo".to_string(),
        arguments: serde_json::json!({"text": "Hello"}),
    };

    let result = registry.call_tool(call).await.unwrap();
    assert!(!result.is_error, "工具调用不应失败");
}

// 资源发现测试
#[tokio::test]
async fn test_resource_discovery() {
    let registry = ResourceRegistry::new();

    // 注册示例资源
    registry.register(ResourceMetadata {
        uri: "data://test".to_string(),
        name: "测试资源".to_string(),
        description: "测试资源描述".to_string(),
        mime_type: Some("text/plain".to_string()),
    }).await.unwrap();

    // 列出资源
    let resources = registry.list_resources().await;
    assert_eq!(resources.len(), 1, "应返回 1 个资源");
    assert_eq!(resources[0].uri, "data://test");
}

// 资源读取测试
#[tokio::test]
async fn test_resource_reading() {
    let registry = ResourceRegistry::new();

    // 注册示例资源
    registry.register(ResourceMetadata {
        uri: "data://example".to_string(),
        name: "示例资源".to_string(),
        description: "示例".to_string(),
        mime_type: Some("text/plain".to_string()),
    }).await.unwrap();

    // 验证资源已注册
    let resources = registry.list_resources().await;
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "data://example");
}

// 提示词发现测试
#[tokio::test]
async fn test_prompt_discovery() {
    let registry = PromptRegistry::new();

    // 注册示例提示词
    registry.register(PromptMetadata {
        name: "test_prompt".to_string(),
        description: "测试提示词".to_string(),
        arguments: vec![],
    }).await.unwrap();

    // 列出提示词
    let prompts = registry.list_prompts().await;
    assert_eq!(prompts.len(), 1, "应返回 1 个提示词");
    assert_eq!(prompts[0].name, "test_prompt");
}

// 提示词模板获取测试
#[tokio::test]
async fn test_prompt_template_generation() {
    let registry = PromptRegistry::new();

    // 注册预定义提示词
    registry.register(PromptMetadata {
        name: "summarize".to_string(),
        description: "文本摘要".to_string(),
        arguments: vec![],
    }).await.unwrap();

    // 验证注册成功
    let prompts = registry.list_prompts().await;
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].name, "summarize");
}

// 提示词参数替换测试
#[tokio::test]
async fn test_prompt_argument_substitution() {
    let registry = PromptRegistry::new();

    // 注册带参数的提示词
    registry.register(PromptMetadata {
        name: "translate".to_string(),
        description: "文本翻译".to_string(),
        arguments: vec![
            newclaw::mcp::prompts::PromptArgument {
                name: "text".to_string(),
                description: "要翻译的文本".to_string(),
                required: true,
            },
            newclaw::mcp::prompts::PromptArgument {
                name: "target_language".to_string(),
                description: "目标语言".to_string(),
                required: true,
            },
        ],
    }).await.unwrap();

    // 获取提示词模板（带参数）
    let mut args = std::collections::HashMap::new();
    args.insert("text".to_string(), "Hello, world!".to_string());
    args.insert("target_language".to_string(), "中文".to_string());

    let template = registry.get_prompt_template("translate", args).await.unwrap();
    assert!(!template.messages.is_empty(), "生成的消息不应为空");
}

// 工具注销测试
#[tokio::test]
async fn test_tool_unregistration() {
    let registry = ToolRegistry::new();

    // 注册工具
    registry.register(ToolMetadata {
        name: "temp_tool".to_string(),
        description: "临时工具".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
    }).await.unwrap();

    // 验证工具存在
    let tools = registry.list_tools().await;
    assert_eq!(tools.len(), 1);

    // 注销工具
    registry.unregister("temp_tool").await.unwrap();

    // 验证工具已删除
    let tools = registry.list_tools().await;
    assert_eq!(tools.len(), 0);
}

// 资源注销测试
#[tokio::test]
async fn test_resource_unregistration() {
    let registry = ResourceRegistry::new();

    // 注册资源
    registry.register(ResourceMetadata {
        uri: "data://temp".to_string(),
        name: "临时资源".to_string(),
        description: "临时".to_string(),
        mime_type: Some("text/plain".to_string()),
    }).await.unwrap();

    // 验证资源存在
    let resources = registry.list_resources().await;
    assert_eq!(resources.len(), 1);

    // 注销资源
    registry.unregister("data://temp").await.unwrap();

    // 验证资源已删除
    let resources = registry.list_resources().await;
    assert_eq!(resources.len(), 0);
}

// 错误处理测试 - 工具不存在
#[tokio::test]
async fn test_tool_not_found() {
    let registry = ToolRegistry::new();

    let call = ToolCall {
        name: "nonexistent_tool".to_string(),
        arguments: serde_json::json!({}),
    };

    let result = registry.call_tool(call).await;
    assert!(result.is_err(), "应返回工具不存在的错误");
}

// 错误处理测试 - 资源不存在
#[tokio::test]
async fn test_resource_not_found() {
    let registry = ResourceRegistry::new();

    let result = registry.read_resource("data://nonexistent").await;
    assert!(result.is_err(), "应返回资源不存在的错误");
}

// 错误处理测试 - 提示词不存在
#[tokio::test]
async fn test_prompt_not_found() {
    let registry = PromptRegistry::new();

    let args = std::collections::HashMap::new();
    let result = registry.get_prompt_template("nonexistent", args).await;
    assert!(result.is_err(), "应返回提示词不存在的错误");
}

// 并发测试 - 工具注册
#[tokio::test]
async fn test_concurrent_tool_registration() {
    let registry = std::sync::Arc::new(ToolRegistry::new());
    let mut handles = vec![];

    // 10 个并发注册任务
    for i in 0..10 {
        let registry_clone = registry.clone();
        let handle = tokio::spawn(async move {
            registry_clone.register(ToolMetadata {
                name: format!("tool_{}", i),
                description: format!("工具 {}", i),
                input_schema: serde_json::json!({"type": "object"}),
            }).await
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    let results = futures::future::join_all(handles).await;
    assert_eq!(results.len(), 10);

    // 验证所有注册都成功
    for result in results {
        assert!(result.is_ok(), "并发注册失败");
    }

    // 验证工具数量
    let tools = registry.list_tools().await;
    assert_eq!(tools.len(), 10);
}

// 性能测试 - 批量工具调用
#[tokio::test]
async fn test_batch_tool_call_performance() {
    let registry = ToolRegistry::new();

    // 注册工具
    registry.register(ToolMetadata {
        name: "get_current_time".to_string(),
        description: "获取当前时间".to_string(),
        input_schema: serde_json::json!({"type": "object"}),
    }).await.unwrap();

    let start = std::time::Instant::now();

    // 执行 100 次工具调用
    for _ in 0..100 {
        let call = ToolCall {
            name: "get_current_time".to_string(),
            arguments: serde_json::json!({}),
        };
        let _ = registry.call_tool(call).await.unwrap();
    }

    let duration = start.elapsed();

    // 平均延迟应 < 10ms
    let avg_latency = duration.as_millis() as f64 / 100.0;
    assert!(avg_latency < 10.0, "平均延迟过高: {} ms", avg_latency);
}
