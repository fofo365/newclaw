// CLI module for NewClaw - v0.3.1
//
// 支持：
// 1. 多 LLM Provider (OpenAI, Claude, GLM)
// 2. 命令行参数配置
// 3. 配置文件支持
// 4. 工具执行

use std::io::{self, Write};
use clap::Parser;
use crate::config::Config;
use crate::llm::{LLMProviderV3, OpenAIProvider, ClaudeProvider, ChatRequest, Message, MessageRole, TokenUsage};
use crate::tools::ToolRegistry;

/// NewClaw CLI
#[derive(Parser, Debug)]
#[command(name = "newclaw")]
#[command(about = "Next-gen AI Agent framework", long_about = None)]
pub struct CliArgs {
    /// LLM Provider: openai, claude, glm
    #[arg(short, long, value_parser = ["openai", "claude", "glm"])]
    pub provider: Option<String>,
    
    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,
    
    /// Path to config file
    #[arg(short, long)]
    pub config: Option<String>,
    
    /// Run in gateway mode
    #[arg(short, long)]
    pub gateway: bool,
    
    /// Gateway port
    #[arg(long, default_value = "3000")]
    pub port: u16,
    
    /// Enable verbose logging
    #[arg(short, long)]
    pub verbose: bool,
    
    /// Generate example config
    #[arg(long)]
    pub generate_config: bool,
}

pub async fn run_cli() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    
    // 生成示例配置
    if args.generate_config {
        println!("{}", crate::config::generate_example_config());
        return Ok(());
    }
    
    // 初始化日志
    if args.verbose {
        tracing_subscriber::fmt::init();
    }
    
    // 加载配置
    let mut config = if let Some(config_path) = &args.config {
        Config::from_file(config_path)?
    } else {
        Config::load()?
    };
    
    // 命令行参数覆盖
    if let Some(provider) = &args.provider {
        config.llm.provider = provider.clone();
    }
    if let Some(model) = &args.model {
        config.llm.model = model.clone();
    }
    if args.gateway || args.port != 3000 {
        config.gateway.port = args.port;
    }
    
    // Gateway 模式
    if args.gateway {
        println!("🌐 Starting Gateway mode on port {}...", config.gateway.port);
        return crate::gateway::run_server(config).await;
    }
    
    // CLI 交互模式
    run_interactive_mode(&config).await
}

async fn run_interactive_mode(config: &Config) -> anyhow::Result<()> {
    println!("🦀 NewClaw v{} - Interactive Mode", env!("CARGO_PKG_VERSION"));
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // 显示当前配置
    println!("📋 Configuration:");
    println!("   Provider: {}", config.llm.provider);
    println!("   Model:    {}", config.get_model());
    
    // 检查 API Key
    let api_key = config.get_api_key();
    match &api_key {
        Ok(key) => {
            let masked = if key.len() > 8 {
                format!("{}...{}", &key[..4], &key[key.len()-4..])
            } else {
                "***".to_string()
            };
            println!("   API Key:  {} ✅", masked);
        }
        Err(e) => {
            println!("   API Key:  ⚠️  {}", e);
            println!("\n💡 Set your API key:");
            println!("   export {}_API_KEY=your-key-here", config.llm.provider.to_uppercase());
            println!("\n   Or create a config.toml file:");
            println!("   newclaw --generate-config > config.toml");
            println!("\nRunning in mock mode...\n");
        }
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Type 'exit' or 'quit' to exit");
    println!("Type 'help' for more commands\n");
    
    // 创建 Provider
    let provider: Option<Box<dyn LLMProviderV3>> = if api_key.is_ok() {
        Some(create_provider(config)?)
    } else {
        None
    };
    
    // 创建工具注册表
    let tool_registry = ToolRegistry::new();
    register_tools(&tool_registry).await;
    
    loop {
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // 处理特殊命令
        match input {
            "exit" | "quit" => {
                println!("👋 Goodbye!");
                break;
            }
            "help" => {
                print_help();
                continue;
            }
            "tools" => {
                list_tools(&tool_registry).await;
                continue;
            }
            "config" => {
                print_config(config);
                continue;
            }
            "clear" => {
                print!("\x1B[2J\x1B[1;1H"); // ANSI clear screen
                continue;
            }
            _ => {}
        }
        
        // 处理聊天请求
        match process_chat(&provider, input, config).await {
            Ok(response) => {
                println!("🤖 {}\n", response);
            }
            Err(e) => {
                eprintln!("❌ Error: {}\n", e);
            }
        }
    }
    
    Ok(())
}

/// 创建 LLM Provider
fn create_provider(config: &Config) -> anyhow::Result<Box<dyn LLMProviderV3>> {
    let api_key = config.get_api_key()?;
    
    match config.llm.provider.as_str() {
        "openai" => {
            let mut p = OpenAIProvider::new(api_key);
            if let Some(base_url) = &config.llm.openai.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            Ok(Box::new(p))
        }
        "claude" => {
            let mut p = ClaudeProvider::new(api_key);
            if let Some(base_url) = &config.llm.claude.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            Ok(Box::new(p))
        }
        "glm" => {
            // GLM 暂时使用 mock 实现
            println!("⚠️  GLM provider in CLI uses simplified implementation");
            Ok(Box::new(GLMProvider::new(api_key)))
        }
        other => {
            Err(anyhow::anyhow!("Unknown provider: {}", other))
        }
    }
}

/// GLM 简化实现
struct GLMProvider {
    api_key: String,
}

impl GLMProvider {
    fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait::async_trait]
impl LLMProviderV3 for GLMProvider {
    fn name(&self) -> &str {
        "glm"
    }
    
    async fn chat(&self, req: ChatRequest) -> Result<crate::llm::ChatResponse, crate::llm::LLMError> {
        // 转换并调用 GLM
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                },
                "content": m.content
            })).collect::<Vec<_>>(),
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });
        
        let resp = client
            .post("https://open.bigmodel.cn/api/paas/v4/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::llm::LLMError::NetworkError(e.to_string()))?;
        
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        
        if !status.is_success() {
            return Err(crate::llm::LLMError::ApiError(text));
        }
        
        let json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| crate::llm::LLMError::SerializationError(e.to_string()))?;
        
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        Ok(crate::llm::ChatResponse {
            message: Message {
                role: MessageRole::Assistant,
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: json["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize,
            },
            finish_reason: json["choices"][0]["finish_reason"].as_str().map(|s| s.to_string()),
            model: req.model,
        })
    }
    
    async fn chat_stream(
        &self,
        _req: ChatRequest,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, crate::llm::LLMError>> + Send>>, crate::llm::LLMError> {
        Err(crate::llm::LLMError::ApiError("Streaming not implemented".to_string()))
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        text.len() / 4
    }
    
    async fn validate(&self) -> Result<bool, crate::llm::LLMError> {
        Ok(true)
    }
}

/// 处理聊天请求
async fn process_chat(
    provider: &Option<Box<dyn LLMProviderV3>>,
    input: &str,
    config: &Config,
) -> anyhow::Result<String> {
    if let Some(p) = provider {
        let request = ChatRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: input.to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            model: config.get_model(),
            temperature: config.llm.temperature,
            max_tokens: Some(config.llm.max_tokens),
            top_p: None,
            stop: None,
            tools: None,
        };
        
        let response = p.chat(request).await?;
        Ok(response.message.content)
    } else {
        // Mock 模式
        Ok(format!(
            "[Mock Mode] Processed: {}\n\nSet {}_API_KEY to enable real responses.",
            input,
            config.llm.provider.to_uppercase()
        ))
    }
}

/// 注册工具
async fn register_tools(registry: &ToolRegistry) {
    use std::sync::Arc;
    use crate::tools::{ReadTool, WriteTool, EditTool, ExecTool, SearchTool};
    
    registry.register(Arc::new(ReadTool::default())).await;
    registry.register(Arc::new(WriteTool::default())).await;
    registry.register(Arc::new(EditTool::default())).await;
    registry.register(Arc::new(ExecTool::default())).await;
    registry.register(Arc::new(SearchTool::default())).await;
}

/// 列出工具
async fn list_tools(registry: &ToolRegistry) {
    println!("\n📦 Available Tools:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let tools = registry.list().await;
    for tool in tools {
        println!("  • {} - {}", tool.name, tool.description);
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 打印配置
fn print_config(config: &Config) {
    println!("\n⚙️  Current Configuration:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Provider:    {}", config.llm.provider);
    println!("  Model:       {}", config.get_model());
    println!("  Temperature: {}", config.llm.temperature);
    println!("  Max Tokens:  {}", config.llm.max_tokens);
    println!("  Gateway:     {}:{}", config.gateway.host, config.gateway.port);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 打印帮助
fn print_help() {
    println!("\n📖 Commands:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  help     - Show this help message");
    println!("  tools    - List available tools");
    println!("  config   - Show current configuration");
    println!("  clear    - Clear the screen");
    println!("  exit     - Exit the program");
    println!("  quit     - Exit the program");
    println!("\n📝 Environment Variables:");
    println!("  LLM_PROVIDER    - Provider: openai, claude, glm");
    println!("  LLM_MODEL       - Model name");
    println!("  OPENAI_API_KEY  - OpenAI API key");
    println!("  ANTHROPIC_API_KEY - Claude API key");
    println!("  GLM_API_KEY     - GLM API key");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
