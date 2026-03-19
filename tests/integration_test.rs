// NewClaw v0.3.0 - 集成测试
//
// 测试场景：
// 1. 工具 + LLM 协作
// 2. 多模型切换
// 3. 流式响应

use newclaw::*;
use newclaw::tools::{ToolRegistry, ReadTool, WriteTool};
use newclaw::llm::LLMProviderV3;
use std::path::PathBuf;

#[tokio::test]
async fn test_tool_llm_integration() {
    // 创建工具注册表
    let registry = ToolRegistry::new();

    // 注册工具（允许 /tmp 目录用于测试）
    let read_tool = ReadTool::new().with_allowed_dirs(vec![PathBuf::from("/tmp")]);
    let write_tool = WriteTool::new().with_allowed_dirs(vec![PathBuf::from("/tmp")]);
    registry.register(read_tool).await.unwrap();
    registry.register(write_tool).await.unwrap();
    
    // 模拟 Agent 思考过程
    let _user_input = "请创建一个测试文件并写入 Hello, World!";
    
    // Agent 决定使用 write 工具
    let output = registry.call(
        "write",
        serde_json::json!({
            "path": "/tmp/test_integration.txt",
            "content": "Hello, World!"
        })
    ).await.unwrap();
    
    assert!(output["success"].as_bool().unwrap_or(false));
    
    // 验证文件创建
    let read_output = registry.call(
        "read",
        serde_json::json!({
            "path": "/tmp/test_integration.txt"
        })
    ).await.unwrap();
    
    assert!(read_output["content"].as_str().unwrap().contains("Hello, World!"));
    
    // 清理
    std::fs::remove_file("/tmp/test_integration.txt").ok();
}

#[tokio::test]
async fn test_model_strategy_static() {
    let strategy = llm::ModelStrategy::Static {
        model: "gpt-4o-mini".to_string(),
    };
    
    let model = strategy.select(100);
    assert_eq!(model, "gpt-4o-mini");
}

#[tokio::test]
async fn test_model_strategy_round_robin() {
    let strategy = llm::ModelStrategy::RoundRobin {
        models: vec![
            "gpt-4o-mini".to_string(),
            "claude-3-5-sonnet".to_string(),
        ],
    };
    
    let model1 = strategy.select(0);
    let model2 = strategy.select(0);
    let model3 = strategy.select(0);
    
    assert_eq!(model1, "gpt-4o-mini");
    assert_eq!(model2, "claude-3-5-sonnet");
    assert_eq!(model3, "gpt-4o-mini");
}

#[tokio::test]
async fn test_model_strategy_cost_optimized() {
    let strategy = llm::ModelStrategy::CostOptimized {
        cheap: "gpt-4o-mini".to_string(),
        premium: "gpt-4o".to_string(),
    };
    
    let model = strategy.select(0);
    assert_eq!(model, "gpt-4o-mini");
}

#[tokio::test]
async fn test_model_strategy_adaptive() {
    let strategy = llm::ModelStrategy::Adaptive {
        simple: "gpt-4o-mini".to_string(),
        complex: "gpt-4o".to_string(),
        threshold: 1000,
    };
    
    // 简单任务
    let model1 = strategy.select(500);
    assert_eq!(model1, "gpt-4o-mini");
    
    // 复杂任务
    let model2 = strategy.select(1500);
    assert_eq!(model2, "gpt-4o");
}

#[tokio::test]
async fn test_sse_streaming() {
    use llm::streaming::*;
    
    let event = SSEEvent::new("Hello, World!".to_string())
        .with_id("123".to_string())
        .with_event("message".to_string());
    
    let formatted = event.format();
    
    assert!(formatted.contains("id: 123"));
    assert!(formatted.contains("event: message"));
    assert!(formatted.contains("data: Hello, World!"));
    assert!(formatted.ends_with("\n\n"));
}

#[tokio::test]
async fn test_feishu_streaming_adapter() {
    use llm::streaming::*;
    
    let mut adapter = FeishuStreamAdapter::new();
    
    adapter.add_chunk("Hello".to_string());
    adapter.add_chunk(" World".to_string());
    adapter.add_chunk("!".to_string());
    
    assert_eq!(adapter.chunks.len(), 3);
    assert_eq!(adapter.next_chunk(0), Some("Hello".to_string()));
    assert_eq!(adapter.next_chunk(1), Some(" World".to_string()));
    assert_eq!(adapter.next_chunk(2), Some("!".to_string()));
    assert_eq!(adapter.next_chunk(3), None);
}

#[tokio::test]
async fn test_openai_provider_creation() {
    use llm::OpenAIProvider;
    
    let provider = OpenAIProvider::new("test-key".to_string());
    assert_eq!(provider.name(), "openai");
}

#[tokio::test]
async fn test_claude_provider_creation() {
    use llm::ClaudeProvider;
    
    let provider = ClaudeProvider::new("test-key".to_string());
    assert_eq!(provider.name(), "claude");
}

#[tokio::test]
async fn test_tool_registry() {
    let registry = ToolRegistry::new();

    let read_tool = ReadTool::new().with_allowed_dirs(vec![PathBuf::from("/tmp")]);
    registry.register(read_tool).await.unwrap();
    
    let tools = registry.list_tools().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "read");
    
    let metadata = registry.get_tool("read").await;
    assert!(metadata.is_some());
    
    let not_exists = registry.get_tool("nonexistent").await;
    assert!(not_exists.is_none());
}

#[tokio::test]
async fn end_to_end_workflow() {
    // 完整的工作流测试

    // 1. 创建工具（允许 /tmp 目录用于测试）
    let registry = ToolRegistry::new();
    let read_tool = ReadTool::new().with_allowed_dirs(vec![PathBuf::from("/tmp")]);
    let write_tool = WriteTool::new().with_allowed_dirs(vec![PathBuf::from("/tmp")]);
    registry.register(write_tool).await.unwrap();
    registry.register(read_tool).await.unwrap();
    
    // 2. 创建 LLM Provider
    let openai = llm::OpenAIProvider::new("dummy-key".to_string());
    assert_eq!(openai.name(), "openai");
    
    // 3. 模拟工作流
    // 用户: "创建一个文件并写入测试内容"
    // Agent: 使用 write 工具
    let write_output = registry.call(
        "write",
        serde_json::json!({
            "path": "/tmp/e2e_test.txt",
            "content": "测试内容"
        })
    ).await.unwrap();
    
    assert!(write_output["success"].as_bool().unwrap_or(false));
    
    // Agent: 使用 read 工具验证
    let read_output = registry.call(
        "read",
        serde_json::json!({
            "path": "/tmp/e2e_test.txt"
        })
    ).await.unwrap();
    
    assert!(read_output["content"].as_str().unwrap().contains("测试内容"));
    
    // 清理
    std::fs::remove_file("/tmp/e2e_test.txt").ok();
}
