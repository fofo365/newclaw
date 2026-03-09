// CLI module for NewClaw - v0.4.0
//
// 支持：
// 1. 多 LLM Provider (OpenAI, Claude, GLM 多区域)
// 2. 命令行参数配置
// 3. 配置文件支持
// 4. 工具执行

use std::io::{self, Write};
use clap::Parser;
use crate::config::Config;
use crate::llm::{
    LLMProviderV3, OpenAIProvider, ClaudeProvider, GlmProvider, GlmConfig, GlmRegion, GlmProviderType,
    ChatRequest, Message, MessageRole, TokenUsage, is_glm_alias, create_glm_provider
};
use crate::tools::ToolRegistry;

/// NewClaw CLI
#[derive(Parser, Debug)]
#[command(name = "newclaw")]
#[command(about = "Next-gen AI Agent framework", long_about = None)]
pub struct CliArgs {
    /// LLM Provider: openai, claude, glm, glm-cn, glm-global, z.ai, zai-cn
    #[arg(short, long)]
    pub provider: Option<String>,
    
    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,
    
    /// GLM Region: china, international (for GLM providers)
    #[arg(long)]
    pub glm_region: Option<String>,
    
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
    
    /// List supported providers
    #[arg(long)]
    pub list_providers: bool,
}

pub async fn run_cli() -> anyhow::Result<()> {
    let args = CliArgs::parse();
    
    // 生成示例配置
    if args.generate_config {
        println!("{}", crate::config::generate_example_config());
        return Ok(());
    }
    
    // 列出支持的 Provider
    if args.list_providers {
        print_providers();
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
    if let Some(region) = &args.glm_region {
        config.llm.glm.region = region.clone();
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
    
    // 显示 GLM 区域信息
    if is_glm_alias(&config.llm.provider) {
        let glm_config = config.get_glm_config();
        println!("   Region:   {}", glm_config.region);
        println!("   Type:     {}", glm_config.provider_type);
    }
    
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
            if is_glm_alias(&config.llm.provider) {
                println!("   export GLM_API_KEY=your-id.your-secret");
            } else {
                println!("   export {}_API_KEY=your-key-here", config.llm.provider.to_uppercase());
            }
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
            "providers" => {
                print_providers();
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
    let provider_lower = config.llm.provider.to_lowercase();
    
    // 检查是否为 GLM 系列
    if is_glm_alias(&provider_lower) {
        let glm_config = config.get_glm_config();
        
        let region = match glm_config.region.to_lowercase().as_str() {
            "china" | "cn" | "中国" => GlmRegion::China,
            _ => GlmRegion::International,
        };
        
        let provider_type = match glm_config.provider_type.to_lowercase().as_str() {
            "glmcode" | "coding" => GlmProviderType::GlmCode,
            _ => GlmProviderType::Glm,
        };
        
        let provider = if let Some(ref base_url) = glm_config.base_url {
            GlmProvider::with_config(api_key, GlmConfig {
                region,
                provider_type,
                model: config.get_model(),
                temperature: config.llm.temperature,
                max_tokens: config.llm.max_tokens,
            }).set_base_url(base_url.clone())
        } else {
            GlmProvider::with_config(api_key, GlmConfig {
                region,
                provider_type,
                model: config.get_model(),
                temperature: config.llm.temperature,
                max_tokens: config.llm.max_tokens,
            })
        };
        
        return Ok(Box::new(provider));
    }
    
    match provider_lower.as_str() {
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
        other => {
            Err(anyhow::anyhow!(
                "Unknown provider: {}. Use --list-providers to see supported providers.",
                other
            ))
        }
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
        let env_hint = if is_glm_alias(&config.llm.provider) {
            "GLM_API_KEY"
        } else {
            &format!("{}_API_KEY", config.llm.provider.to_uppercase())
        };
        
        Ok(format!(
            "[Mock Mode] Processed: {}\n\nSet {} to enable real responses.",
            input, env_hint
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
    
    if is_glm_alias(&config.llm.provider) {
        let glm_config = config.get_glm_config();
        println!("\n  GLM Configuration:");
        println!("    Region: {}", glm_config.region);
        println!("    Type:   {}", glm_config.provider_type);
        if let Some(ref url) = glm_config.base_url {
            println!("    URL:    {}", url);
        }
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 打印支持的 Provider
fn print_providers() {
    println!("\n🔌 Supported Providers:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\n  OpenAI Compatible:");
    println!("    openai       - OpenAI GPT models");
    println!("    claude       - Anthropic Claude models");
    println!("\n  GLM / Zhipu (Multi-Region):");
    println!("    glm          - GLM International (api.z.ai)");
    println!("    glm-global   - GLM International (alias)");
    println!("    glm-cn       - GLM China (open.bigmodel.cn)");
    println!("    bigmodel     - GLM China (alias)");
    println!("\n  GLMCode / z.ai (Coding Models):");
    println!("    z.ai         - z.ai International (api.z.ai/coding)");
    println!("    zai          - z.ai International (alias)");
    println!("    zai-cn       - z.ai China (open.bigmodel.cn/coding)");
    println!("    glmcode      - GLMCode International (alias)");
    println!("    glmcode-cn   - GLMCode China (alias)");
    println!("\n  📝 Environment Variables:");
    println!("    LLM_PROVIDER    - Set provider");
    println!("    LLM_MODEL       - Set model");
    println!("    GLM_API_KEY     - GLM API key (format: id.secret)");
    println!("    GLM_REGION      - GLM region (china/international)");
    println!("    GLM_TYPE        - GLM type (glm/glmcode)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 打印帮助
fn print_help() {
    println!("\n📖 Commands:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  help      - Show this help message");
    println!("  tools     - List available tools");
    println!("  config    - Show current configuration");
    println!("  providers - List supported providers");
    println!("  clear     - Clear the screen");
    println!("  exit      - Exit the program");
    println!("  quit      - Exit the program");
    println!("\n📝 Environment Variables:");
    println!("  LLM_PROVIDER    - Provider (openai, claude, glm, glm-cn, z.ai, etc.)");
    println!("  LLM_MODEL       - Model name");
    println!("  GLM_API_KEY     - GLM API key (format: id.secret)");
    println!("  GLM_REGION      - GLM region (china/international)");
    println!("  OPENAI_API_KEY  - OpenAI API key");
    println!("  ANTHROPIC_API_KEY - Claude API key");
    println!("\n🔧 CLI Options:");
    println!("  --provider NAME   - Set provider");
    println!("  --model NAME      - Set model");
    println!("  --glm-region REGION - Set GLM region (china/international)");
    println!("  --gateway         - Run in gateway mode");
    println!("  --port PORT       - Gateway port (default: 3000)");
    println!("  --generate-config - Generate example config.toml");
    println!("  --list-providers  - List supported providers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
