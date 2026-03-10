// NewClaw CLI - v0.4.0-beta.1
//
// 支持：
// 1. 多 LLM Provider
// 2. 配置文件
// 3. Gateway 模式
// 4. 插件系统

use clap::{Parser, Subcommand};
use std::net::SocketAddr;

#[derive(Parser)]
#[command(name = "newclaw")]
#[command(version = "0.4.1")]
#[command(about = "Next-gen AI Agent framework - Rust performance + TypeScript plugins", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// LLM Provider: openai, claude, glm
    #[arg(short, long, global = true)]
    provider: Option<String>,
    
    /// Model to use
    #[arg(short, long, global = true)]
    model: Option<String>,
    
    /// Config file path
    #[arg(short, long, global = true)]
    config: Option<String>,
    
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in interactive chat mode
    Chat,
    
    /// Start web gateway server
    Gateway {
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
        
        /// Host to bind to
        #[arg(short, long, default_value = "0.0.0.0")]
        host: String,
    },
    
    /// Start dashboard server
    Dashboard {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
        
        /// Host to bind to
        #[arg(short, long, default_value = "0.0.0.0")]
        host: String,
    },
    
    /// Get dashboard pair code for authentication
    #[command(name = "paircode")]
    PairCode,
    
    /// Generate example configuration file
    Config {
        /// Output file path (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// List and manage plugins
    Plugin {
        #[command(subcommand)]
        action: Option<PluginCommands>,
    },
    
    /// Manage tools
    Tools {
        #[command(subcommand)]
        action: Option<ToolCommands>,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// List installed plugins
    List,
    
    /// Install a plugin
    Install {
        /// Plugin name or path
        name: String,
    },
    
    /// Remove a plugin
    Remove {
        /// Plugin name
        name: String,
    },
}

#[derive(Subcommand)]
enum ToolCommands {
    /// List available tools
    List,
    
    /// Execute a tool
    Exec {
        /// Tool name
        name: String,
        
        /// JSON parameters
        #[arg(short, long)]
        params: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    // 初始化日志
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }
    
    // 加载配置
    let mut config = if let Some(config_path) = &cli.config {
        newclaw::config::Config::from_file(config_path)?
    } else {
        newclaw::config::Config::load()?
    };
    
    // 应用命令行覆盖
    if let Some(provider) = &cli.provider {
        config.llm.provider = provider.clone();
    }
    if let Some(model) = &cli.model {
        config.llm.model = model.clone();
    }
    
    // 执行命令
    match cli.command {
        None | Some(Commands::Chat) => {
            // 默认：交互模式
            newclaw::cli::run_cli().await?;
        }
        
        Some(Commands::Gateway { port, host }) => {
            config.gateway.port = port;
            config.gateway.host = host;
            
            println!("🌐 Starting NewClaw Gateway...");
            println!("   Provider: {}", config.llm.provider);
            println!("   Model:    {}", config.get_model());
            println!("   Address:  {}:{}", config.gateway.host, config.gateway.port);
            println!();
            
            newclaw::gateway::run_server(config).await?;
        }
        
        Some(Commands::Dashboard { port, host }) => {
            let dashboard_config = newclaw::dashboard::DashboardConfig {
                enabled: true,
                port,
                auth_enabled: true,
                ..Default::default()
            };

            println!("🚀 Starting NewClaw Dashboard...");
            println!("   Address:  {}:{}", host, dashboard_config.port);
            println!("   Auth:     Enabled (6-digit pair code)");
            println!();

            newclaw::dashboard::start_dashboard(dashboard_config).await?;
        }
        
        Some(Commands::PairCode) => {
            // 调用 Dashboard API 获取配对码
            // 注意：CLI 命令是 `pair-code`（kebab-case）
            match get_pair_code_from_api().await {
                Ok(info) => {
                    println!("🔐 Dashboard Pair Code:");
                    println!();
                    println!("   ⚡  {}", info.code);
                    println!();
                    println!("   Session ID:  {}", info.session_id);
                    println!("   Expires at:  {}", info.expires_at.format("%Y-%m-%d %H:%M:%S UTC"));
                    println!("   Dashboard:   {}", info.dashboard_url);
                    println!();
                    println!("💡 Use this code in the Dashboard login page");
                }
                Err(e) => {
                    eprintln!("❌ Failed to get pair code: {}", e);
                    eprintln!();
                    eprintln!("💡 Make sure Dashboard is running:");
                    eprintln!("   newclaw dashboard");
                }
            }
        }
        
        Some(Commands::Config { output }) => {
            let content = newclaw::config::generate_example_config();
            
            if let Some(path) = output {
                std::fs::write(&path, &content)?;
                println!("✅ Configuration written to: {}", path);
            } else {
                println!("{}", content);
            }
        }
        
        Some(Commands::Plugin { action }) => {
            match action {
                None | Some(PluginCommands::List) => {
                    println!("📦 Installed Plugins:");
                    println!("   (No plugins installed yet)");
                    println!();
                    println!("💡 To install a plugin:");
                    println!("   newclaw plugin install <name>");
                }
                Some(PluginCommands::Install { name }) => {
                    println!("📦 Installing plugin: {}", name);
                    println!("⚠️  Plugin system coming soon!");
                }
                Some(PluginCommands::Remove { name }) => {
                    println!("🗑️  Removing plugin: {}", name);
                    println!("⚠️  Plugin system coming soon!");
                }
            }
        }
        
        Some(Commands::Tools { action }) => {
            match action {
                None | Some(ToolCommands::List) => {
                    println!("🔧 Available Tools:");
                    println!("   read   - Read file contents");
                    println!("   write  - Write to file");
                    println!("   edit   - Edit file (replace text)");
                    println!("   exec   - Execute shell command");
                    println!("   search - Web search");
                    println!();
                    println!("💡 To execute a tool:");
                    println!("   newclaw tools exec read --params '{{\"path\": \"file.txt\"}}'");
                }
                Some(ToolCommands::Exec { name, params }) => {
                    let params_json = params
                        .map(|p| serde_json::from_str(&p))
                        .transpose()?
                        .unwrap_or(serde_json::json!({}));

                    println!("🔧 Executing tool: {}", name);
                    println!("   Params: {}", serde_json::to_string_pretty(&params_json)?);
                    println!();

                    // 创建工具注册表并执行
                    let registry = newclaw::tools::ToolRegistry::new();
                    // register_default_tools(&registry).await;  // TODO: 工具系统待实现

                    // TODO: 工具执行功能待实现
                    println!("❌ Tool system not yet implemented");
                    println!("   Tool: {}", name);
                    println!("   Params: {}", params_json);
                    /* match registry.execute(&name, params_json).await {
                        Ok(output) => {
                            if output.is_success() {
                                println!("✅ Result:");
                                println!("{}", output.content);
                            } else {
                                println!("❌ Error: {:?}", output.error);
                            }
                        }
                        Err(e) => {
                            println!("❌ Tool execution failed: {}", e);
                        }
                    } */
                }
            }
        }
    }

    Ok(())
}

/// 注册默认工具
async fn register_default_tools(_registry: &newclaw::tools::ToolRegistry) {
    // TODO: 工具系统待实现
    /* use std::sync::Arc;
    use newclaw::tools::{ReadTool, WriteTool, EditTool, ExecTool, SearchTool};

    registry.register(Arc::new(ReadTool)).await;
    registry.register(Arc::new(WriteTool)).await;
    registry.register(Arc::new(EditTool)).await;
    registry.register(Arc::new(ExecTool)).await;
    registry.register(Arc::new(SearchTool)).await; */
}

/// 从 Dashboard API 获取配对码
async fn get_pair_code_from_api() -> anyhow::Result<newclaw::dashboard::auth::PairCodeInfo> {
    use reqwest::Client;

    let client = Client::new();
    let resp = client
        .get("http://localhost:8080/api/auth/paircode")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("API returned error: {}", resp.status());
    }

    let data: serde_json::Value = resp.json().await?;

    Ok(newclaw::dashboard::auth::PairCodeInfo {
        code: data["code"].as_str().unwrap().to_string(),
        session_id: data["session_id"].as_str().unwrap().to_string(),
        created_at: chrono::Utc::now(), // API 不返回，使用当前时间
        expires_at: chrono::DateTime::parse_from_rfc3339(data["expires_at"].as_str().unwrap())
            .unwrap()
            .with_timezone(&chrono::Utc),
        dashboard_url: "http://localhost:8080".to_string(),
    })
}
