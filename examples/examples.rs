// NewClaw v0.3.0 - 示例代码集合
//
// 包含：
// 1. 基础 Agent 使用
// 2. 多 LLM 切换
// 3. 工具使用
// 4. 流式响应
// 5. 飞书集成

use newclaw::*;
use std::sync::Arc;

// ============================================================================
// 示例 1: 基础 Agent 使用
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Agent
    let agent = AgentEngine::new(
        "my-agent".to_string(),
        "gpt-4o-mini".to_string()
    )?;
    
    // 处理用户输入
    let response = agent.process("Hello, NewClaw!").await?;
    println!("Response: {}", response);
    
    Ok(())
}

// ============================================================================
// 示例 2: 多 LLM 切换
// ============================================================================

async fn example_multi_llm() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::llm::{OpenAIProvider, ClaudeProvider, ModelStrategy};
    
    // 创建 Providers
    let openai = OpenAIProvider::new("your-openai-key".to_string());
    let claude = ClaudeProvider::new("your-claude-key".to_string());
    
    // 定义策略：成本优化
    let strategy = ModelStrategy::CostOptimized {
        cheap: "gpt-4o-mini".to_string(),
        premium: "gpt-4o".to_string(),
    };
    
    // 选择模型
    let model = strategy.select(100); // 简单任务
    println!("Selected model: {}", model);
    
    let model = strategy.select(2000); // 复杂任务
    println!("Selected model: {}", model);
    
    Ok(())
}

// ============================================================================
// 示例 3: 工具使用
// ============================================================================

async fn example_tools() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::{ToolRegistry, ReadTool, WriteTool};
    use std::sync::Arc;
    
    // 创建工具注册表
    let registry = ToolRegistry::new();
    
    // 注册工具
    registry.register(Arc::new(WriteTool)).await;
    registry.register(Arc::new(ReadTool)).await;
    
    // 写入文件
    let write_output = registry.execute(
        "write",
        serde_json::json!({
            "path": "/tmp/example.txt",
            "content": "Hello, NewClaw!"
        })
    ).await?;
    
    if write_output.is_success() {
        println!("Write successful: {}", write_output.content);
    }
    
    // 读取文件
    let read_output = registry.execute(
        "read",
        serde_json::json!({
            "path": "/tmp/example.txt"
        })
    ).await?;
    
    if read_output.is_success() {
        println!("File content: {}", read_output.content);
    }
    
    Ok(())
}

// ============================================================================
// 示例 4: 流式响应
// ============================================================================

async fn example_streaming() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::llm::{OpenAIProvider, ChatRequest, Message, MessageRole};
    use newclaw::llm::streaming::{stream_llm_response, StreamChunk};
    
    // 创建 Provider
    let provider = OpenAIProvider::new("your-openai-key".to_string());
    
    // 创建请求
    let request = ChatRequest {
        messages: vec![Message {
            role: MessageRole::User,
            content: "Write a haiku about AI.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        model: "gpt-4o-mini".to_string(),
        temperature: 0.7,
        max_tokens: Some(1000),
        top_p: None,
        stop: None,
        tools: None,
    };
    
    // 流式调用
    stream_llm_response(&provider, request, |chunk| {
        match chunk {
            StreamChunk::Data(data) => {
                print!("{}", data);
                std::io::stdout().flush().ok();
            }
            StreamChunk::Done => {
                println!("\n[Stream complete]");
            }
            StreamChunk::Error(e) => {
                eprintln!("[Error: {}]", e);
            }
        }
    }).await?;
    
    Ok(())
}

// ============================================================================
// 示例 5: 飞书集成
// ============================================================================

async fn example_feishu() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::channels::{
        FeishuConfig,
        FeishuStreamClient,
        RichTextContent,
        TextElement,
    };
    
    // 创建配置
    let config = FeishuConfig {
        app_id: "your-app-id".to_string(),
        app_secret: "your-app-secret".to_string(),
        encrypt_key: None,
        verification_token: None,
    };
    
    // 创建流式客户端
    let mut client = FeishuStreamClient::new(&config);
    
    // 发送流式消息
    let chunks = vec![
        "Hello, ".to_string(),
        "Feishu! ".to_string(),
        "This is a streaming message.".to_string(),
    ];
    
    let message_ids = client.send_streaming("chat_id_here", chunks, 100).await?;
    println!("Sent {} messages", message_ids.len());
    
    // 发送富文本
    let rich_text = RichTextContent {
        elements: vec![
            TextElement {
                tag: "text".to_string(),
                text: "Hello, ".to_string(),
                style: None,
            },
            TextElement {
                tag: "text".to_string(),
                text: "Feishu!".to_string(),
                style: Some(newclaw::channels::TextStyle {
                    bold: Some(true),
                    italic: None,
                    color: Some("red".to_string()),
                }),
            },
        ],
    };
    
    let msg_id = client.send_rich_text("chat_id_here", &rich_text).await?;
    println!("Sent rich text message: {}", msg_id);
    
    Ok(())
}

// ============================================================================
// 示例 6: 工具 + LLM 协作
// ============================================================================

async fn example_tool_llm_collaboration() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::*;
    use std::sync::Arc;
    
    // 创建工具注册表
    let registry = Arc::new(ToolRegistry::new());
    registry.register(Arc::new(WriteTool)).await;
    registry.register(Arc::new(ReadTool)).await;
    registry.register(Arc::new(ExecTool)).await;
    
    // 创建 Agent
    let mut agent = AgentEngine::new(
        "assistant".to_string(),
        "gpt-4o-mini".to_string()
    )?;
    
    // 用户输入
    let user_input = "Create a file named test.txt with content 'Hello, World!'";
    
    // Agent 思考并决定使用工具
    let tool_output = registry.execute(
        "write",
        serde_json::json!({
            "path": "/tmp/test.txt",
            "content": "Hello, World!"
        })
    ).await?;
    
    if tool_output.is_success() {
        println!("Tool executed successfully: {}", tool_output.content);
        
        // Agent 生成响应
        let response = agent.process(&format!(
            "I've created the file. {}",
            tool_output.content
        )).await?;
        
        println!("Agent response: {}", response);
    }
    
    Ok(())
}

// ============================================================================
// 示例 7: 模型策略 - 轮询负载均衡
// ============================================================================

async fn example_round_robin() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::llm::ModelStrategy;
    
    let strategy = ModelStrategy::RoundRobin {
        models: vec![
            "gpt-4o-mini".to_string(),
            "claude-3-5-sonnet".to_string(),
            "gpt-4o".to_string(),
        ],
    };
    
    // 模拟多次请求
    for i in 0..6 {
        let model = strategy.select(i * 100);
        println!("Request {}: Using model {}", i + 1, model);
    }
    
    Ok(())
}

// ============================================================================
// 示例 8: 自定义工具
// ============================================================================

use newclaw::tools::{Tool, ToolOutput, ToolError};
use async_trait::async_trait;

struct CustomTool {
    name: String,
}

#[async_trait]
impl Tool for CustomTool {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "A custom tool example"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let input = params["input"].as_str().unwrap_or("");
        
        Ok(ToolOutput::success(format!(
            "Custom tool received: {}",
            input
        )))
    }
}

async fn example_custom_tool() -> Result<(), Box<dyn std::error::Error>> {
    use newclaw::ToolRegistry;
    use std::sync::Arc;
    
    let registry = ToolRegistry::new();
    let custom_tool = Arc::new(CustomTool {
        name: "custom".to_string(),
    });
    
    registry.register(custom_tool).await;
    
    let output = registry.execute(
        "custom",
        serde_json::json!({"input": "Hello from custom tool!"})
    ).await?;
    
    println!("{}", output.content);
    
    Ok(())
}

// ============================================================================
// 运行所有示例
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== NewClaw v0.3.0 Examples ===\n");
    
    println!("Example 1: Basic Agent");
    // main() 已包含基础示例
    
    println!("\nExample 2: Multi-LLM Switching");
    example_multi_llm().await?;
    
    println!("\nExample 3: Tools");
    example_tools().await?;
    
    println!("\nExample 5: Feishu Integration");
    // 飞书示例需要真实凭证，这里只是展示代码结构
    println!("(Skipped - requires Feishu credentials)");
    
    println!("\nExample 6: Tool + LLM Collaboration");
    example_tool_llm_collaboration().await?;
    
    println!("\nExample 7: Round Robin Strategy");
    example_round_robin().await?;
    
    println!("\nExample 8: Custom Tool");
    example_custom_tool().await?;
    
    Ok(())
}
