// CLI module for NewClaw - v0.7.0
//
// 支持：
// 1. 多 LLM Provider (OpenAI, Claude, GLM 多区域)
// 2. 命令行参数配置
// 3. 配置文件支持
// 4. 工具执行
// 5. 通道层抽象 (v0.7.0)
// 6. 权限控制 (v0.7.0)
// 7. 记忆管理 (v0.7.0) - 多层隔离
// 8. 策略管理 (v0.7.0) - 动态调整

use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use clap::Parser;
use crate::config::Config;
use crate::llm::{
    LLMProviderV3, OpenAIProvider, ClaudeProvider, GlmProvider, GlmConfig, GlmRegion, GlmProviderType,
    Message, MessageRole, is_glm_alias, QwenCodeProvider
};
use crate::tools::ToolRegistry;
use crate::channel::{ChannelPermission, ChannelType, ChannelMember, ChannelRole, ChannelProcessor, ProcessorConfig};
use crate::channel::{ChannelMessage, MessageContent};
use crate::memory::{SQLiteMemoryStorage, StorageConfig};
use crate::context::StrategyEngine;

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
    
    // 创建工具注册表
    let tool_registry = Arc::new(ToolRegistry::new());
    
    // 创建权限管理器
    let permissions = Arc::new(ChannelPermission::new("./data/cli_permissions.json"));
    
    // 初始化内置工具
    if let Err(e) = crate::tools::init_builtin_tools_with_permissions(
        &tool_registry,
        std::path::PathBuf::from("./data"),
        std::path::PathBuf::from("."),
        Some(Arc::clone(&permissions)),
    ).await {
        eprintln!("Warning: Failed to initialize some tools: {}", e);
    }
    
    // 创建 ChannelProcessor - v0.7.0
    let processor_config = ProcessorConfig {
        enable_memory: true,
        enable_strategy: true,
        default_strategy: crate::context::StrategyType::Balanced,
        max_context_tokens: 8000,
        memory_search_limit: 5,
        default_agent_id: "cli".to_string(),
        default_namespace: "default".to_string(),
    };
    
    // 创建记忆存储
    let memory_storage = Arc::new(SQLiteMemoryStorage::new(StorageConfig {
        db_path: std::path::PathBuf::from("data/cli_memory.db"),
        ..Default::default()
    })?);
    
    // 创建策略引擎
    let strategy_engine = Arc::new(RwLock::new(StrategyEngine::new()?));
    
    // 创建处理器
    let mut processor = ChannelProcessor::new(
        Arc::clone(&tool_registry),
        Arc::clone(&permissions),
        processor_config,
    )
    .with_memory(memory_storage)
    .with_strategy(strategy_engine);
    
    // 设置 LLM Provider
    if let Ok(ref api_key) = api_key {
        let llm_provider = create_llm_provider(config, api_key)?;
        processor = processor.with_llm(llm_provider, config.get_model());
    }
    
    let processor = Arc::new(RwLock::new(processor));
    
    // 对话历史
    let conversation_history: Arc<RwLock<Vec<Message>>> = Arc::new(RwLock::new(Vec::new()));
    
    // CLI 通道成员
    let cli_member = ChannelMember {
        channel_type: ChannelType::Cli,
        member_id: "cli_user".to_string(),
        display_name: Some("CLI User".to_string()),
        role: ChannelRole::Admin,
    };
    
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
            "strategy" => {
                print_strategy_help(&processor).await;
                continue;
            }
            "memory" => {
                print_memory_stats(&processor).await;
                continue;
            }
            _ => {}
        }
        
        // 处理聊天请求 - 使用 ChannelProcessor
        match process_chat_with_processor(
            &processor,
            input,
            &cli_member,
            &conversation_history,
        ).await {
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
fn create_llm_provider(config: &Config, api_key: &str) -> anyhow::Result<Arc<dyn LLMProviderV3>> {
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
            GlmProvider::with_config(api_key.to_string(), GlmConfig {
                region,
                provider_type,
                model: config.get_model(),
                temperature: config.llm.temperature,
                max_tokens: config.llm.max_tokens,
            }).set_base_url(base_url.clone())
        } else {
            GlmProvider::with_config(api_key.to_string(), GlmConfig {
                region,
                provider_type,
                model: config.get_model(),
                temperature: config.llm.temperature,
                max_tokens: config.llm.max_tokens,
            })
        };
        
        return Ok(Arc::new(provider));
    }
    
    match provider_lower.as_str() {
        "openai" => {
            let mut p = OpenAIProvider::new(api_key.to_string());
            if let Some(base_url) = &config.llm.openai.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            Ok(Arc::new(p))
        }
        "qwencode" => {
            let mut p = crate::llm::QwenCodeProvider::new(api_key.to_string());
            if let Some(base_url) = &config.llm.qwencode.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            Ok(Arc::new(p))
        }
        "claude" => {
            let mut p = ClaudeProvider::new(api_key.to_string());
            if let Some(base_url) = &config.llm.claude.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            Ok(Arc::new(p))
        }
        other => {
            Err(anyhow::anyhow!(
                "Unknown provider: {}. Use --list-providers to see supported providers.",
                other
            ))
        }
    }
}

/// 处理聊天请求 - 使用 ChannelProcessor
async fn process_chat_with_processor(
    processor: &Arc<RwLock<ChannelProcessor>>,
    input: &str,
    cli_member: &ChannelMember,
    conversation_history: &Arc<RwLock<Vec<Message>>>,
) -> anyhow::Result<String> {
    // 构建通道消息
    let message = ChannelMessage {
        message_id: format!("cli_{}", chrono::Utc::now().timestamp_millis()),
        channel_type: ChannelType::Cli,
        sender: cli_member.clone(),
        chat_id: "cli_session".to_string(),
        content: MessageContent::Text(input.to_string()),
        timestamp: chrono::Utc::now().timestamp(),
        reply_to: None,
        metadata: serde_json::Map::new(),
    };
    
    // 获取历史消息
    let history = conversation_history.read().await.clone();
    
    // 使用处理器处理消息
    let proc = processor.read().await;
    let result = proc.process(&message, &history).await?;
    
    // 更新对话历史
    {
        let mut history = conversation_history.write().await;
        history.push(Message {
            role: MessageRole::User,
            content: input.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });
        history.push(Message {
            role: MessageRole::Assistant,
            content: result.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });
        
        // 限制历史长度
        if history.len() > 20 {
            let start = history.len() - 20;
            *history = history.split_off(start);
        }
    }
    
    // 显示处理信息
    if result.memory_count > 0 || result.tool_calls > 0 {
        println!("📊 [记忆: {}, 工具: {}, 策略: {:?}, 延迟: {}ms]",
            result.memory_count,
            result.tool_calls,
            result.strategy,
            result.latency_ms
        );
    }
    
    Ok(result.content)
}

/// 打印策略帮助
async fn print_strategy_help(processor: &Arc<RwLock<ChannelProcessor>>) {
    let proc = processor.read().await;
    let stats = proc.stats();
    
    println!("\n📊 策略管理:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  记忆启用: {}", stats.memory_enabled);
    println!("  策略启用: {}", stats.strategy_enabled);
    println!("  Agent ID: {}", stats.agent_id);
    println!("  命名空间: {}", stats.namespace);
    println!("\n  可用策略:");
    println!("    balanced    - 平衡模式 (默认)");
    println!("    smart       - 智能截断");
    println!("    time_decay  - 时间衰减");
    println!("    minimize    - 最小化 Token");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 打印记忆统计
async fn print_memory_stats(processor: &Arc<RwLock<ChannelProcessor>>) {
    let proc = processor.read().await;
    let stats = proc.stats();
    
    println!("\n🧠 记忆系统:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  启用状态: {}", stats.memory_enabled);
    println!("  隔离维度: user={}, channel=cli, agent={}, ns={}",
        "cli_user", stats.agent_id, stats.namespace);
    println!("  数据库: data/cli_memory.db");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 列出工具
async fn list_tools(registry: &Arc<ToolRegistry>) {
    let tools = registry.list_tools().await;
    
    if tools.is_empty() {
        println!("\n📦 No tools registered.");
        println!("   Run 'tools' again after tools are initialized.\n");
        return;
    }
    
    println!("\n📦 Available Tools ({}):", tools.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
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
    println!("    qwencode     - QwenCode (coding.dashscope.aliyuncs.com)");
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
    println!("    OPENAI_API_KEY  - OpenAI API key");
    println!("    QWENCODE_API_KEY - QwenCode API key");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}

/// 打印帮助
fn print_help() {
    println!("\n📖 Interactive Mode Commands (v0.7.0):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  help      - Show this help message");
    println!("  tools     - List available tools");
    println!("  config    - Show current configuration");
    println!("  providers - List supported providers");
    println!("  strategy  - Show strategy status");
    println!("  memory    - Show memory system status");
    println!("  clear     - Clear the screen");
    println!("  exit      - Exit the program");
    println!("  quit      - Exit the program");
    println!();
    println!("📝 Environment Variables:");
    println!("  LLM_PROVIDER    - Provider (openai, claude, glm, glm-cn, z.ai, etc.)");
    println!("  LLM_MODEL       - Model name");
    println!("  GLM_API_KEY     - GLM API key (format: id.secret)");
    println!("  GLM_REGION      - GLM region (china/international)");
    println!("  OPENAI_API_KEY  - OpenAI API key");
    println!("  ANTHROPIC_API_KEY - Claude API key");
    println!();
    println!("🔧 CLI Options:");
    println!("  --provider NAME   - Set provider");
    println!("  --model NAME      - Set model");
    println!("  --glm-region REGION - Set GLM region (china/international)");
    println!("  --gateway         - Run in gateway mode");
    println!("  --port PORT       - Gateway port (default: 3000)");
    println!("  --generate-config - Generate example config.toml");
    println!("  --list-providers  - List supported providers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}