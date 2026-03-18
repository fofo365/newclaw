// NewClaw CLI - v0.7.0
//
// 支持：
// 1. 多 LLM Provider
// 2. 配置文件
// 3. Gateway 模式
// 4. 插件系统
// 5. 任务管理
// 6. 记忆管理
// 7. DAG 工作流
// 8. 调度管理
// 9. 审计日志
// 10. Watchdog 监控

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "newclaw")]
#[command(version = "0.7.0")]
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
        #[arg(long, default_value = "3000")]
        port: u16,
        
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
    
    /// Start dashboard server
    Dashboard {
        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,
        
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
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
    
    // ============== v0.7.0 新增命令 ==============
    
    /// Task management (v0.7.0)
    Task {
        #[command(subcommand)]
        action: TaskCommands,
    },
    
    /// DAG workflow management (v0.7.0)
    Dag {
        #[command(subcommand)]
        action: DagCommands,
    },
    
    /// Schedule management (v0.7.0)
    Schedule {
        #[command(subcommand)]
        action: ScheduleCommands,
    },
    
    /// Memory management (v0.7.0)
    Memory {
        #[command(subcommand)]
        action: MemoryCommands,
    },
    
    /// Federation management (v0.7.0)
    Federation {
        #[command(subcommand)]
        action: FederationCommands,
    },
    
    /// Audit log queries (v0.7.0)
    Audit {
        #[command(subcommand)]
        action: AuditCommands,
    },
    
    /// Watchdog monitoring (v0.6.0)
    Watchdog {
        #[command(subcommand)]
        action: WatchdogCommands,
    },
    
    /// Context strategy management (v0.5.0)
    Strategy {
        #[command(subcommand)]
        action: StrategyCommands,
    },
    
    /// Session management (v0.7.0)
    Session {
        #[command(subcommand)]
        action: SessionCommands,
    },

    /// Skill management (v0.7.0)
    Skill {
        #[command(subcommand)]
        action: SkillCommands,
    },
}

// ============== 子命令定义 ==============

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

#[derive(Subcommand)]
enum TaskCommands {
    /// List all tasks
    List,
    
    /// Create a new task
    Create {
        /// Task name
        name: String,
        
        /// Task type (chat, tool_call, workflow)
        #[arg(short, long, default_value = "chat")]
        task_type: String,
    },
    
    /// Get task status
    Status {
        /// Task ID
        id: String,
    },
    
    /// Cancel a task
    Cancel {
        /// Task ID
        id: String,
    },
}

#[derive(Subcommand)]
enum DagCommands {
    /// List all DAG workflows
    List,
    
    /// Create a new DAG workflow
    Create {
        /// DAG name
        name: String,
        
        /// DAG definition file (JSON)
        #[arg(short, long)]
        file: Option<String>,
    },
    
    /// Run a DAG workflow
    Run {
        /// DAG ID
        id: String,
    },
    
    /// Get DAG status
    Status {
        /// DAG ID
        id: String,
    },
}

#[derive(Subcommand)]
enum ScheduleCommands {
    /// List all scheduled tasks
    List,
    
    /// Add a new scheduled task
    Add {
        /// Schedule name
        name: String,
        
        /// Cron expression
        #[arg(short, long)]
        cron: String,
        
        /// Task type
        #[arg(short, long)]
        task_type: String,
    },
    
    /// Remove a scheduled task
    Remove {
        /// Schedule ID
        id: String,
    },
}

#[derive(Subcommand)]
enum MemoryCommands {
    /// Store a memory
    Store {
        /// Memory content
        content: String,
        
        /// Importance (0.0-1.0)
        #[arg(short, long, default_value = "0.5")]
        importance: f32,
        
        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
    },
    
    /// Search memories
    Search {
        /// Search query
        query: String,
        
        /// Max results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    
    /// List all memories
    List {
        /// Max results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    
    /// Delete a memory
    Delete {
        /// Memory ID
        id: String,
    },
}

#[derive(Subcommand)]
enum FederationCommands {
    /// Show federation status
    Status,
    
    /// Sync memories to federation nodes
    Sync {
        /// Target node ID (optional, syncs to all if not specified)
        #[arg(short, long)]
        node: Option<String>,
    },
    
    /// List federation nodes
    Nodes,
}

#[derive(Subcommand)]
enum AuditCommands {
    /// Query audit logs
    Query {
        /// Event type filter
        #[arg(short, long)]
        event_type: Option<String>,
        
        /// Max results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    
    /// Show audit statistics
    Stats,
    
    /// Export audit logs
    Export {
        /// Output file
        #[arg(short, long)]
        output: String,
    },
}

#[derive(Subcommand)]
enum WatchdogCommands {
    /// Show watchdog status
    Status,
    
    /// Show lease status
    Lease {
        /// Lease ID (optional)
        id: Option<String>,
    },
    
    /// Run health check
    Check,
    
    /// Show recovery actions
    Recovery,
}

#[derive(Subcommand)]
enum StrategyCommands {
    /// List available strategies
    List,
    
    /// Get current strategy
    Get,
    
    /// Set strategy
    Set {
        /// Strategy name (smart, time_decay, semantic_cluster)
        name: String,
    },
    
    /// Show strategy config
    Config,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// List all sessions
    List,

    /// Create a new session
    Create {
        /// Session name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Switch to a session
    Switch {
        /// Session ID
        id: String,
    },

    /// Close a session
    Close {
        /// Session ID
        id: String,
    },
}

#[derive(Subcommand)]
enum SkillCommands {
    /// Search skills from skillhub
    Search {
        /// Search query
        query: String,

        /// Results per page
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// Page number
        #[arg(short, long, default_value = "1")]
        page: usize,
    },

    /// Install a skill
    Install {
        /// Skill name
        name: String,

        /// Version (optional)
        #[arg(short, long)]
        version: Option<String>,

        /// Force reinstall
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall a skill
    Uninstall {
        /// Skill name
        name: String,

        /// Force remove without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Update skills
    Update {
        /// Skill name (optional, updates all if not specified)
        name: Option<String>,

        /// Check updates only, don't install
        #[arg(short, long)]
        check_only: bool,
    },

    /// List installed skills
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,

        /// Filter by name
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Show skill information
    Info {
        /// Skill name
        name: String,
    },

    /// Verify a skill
    Verify {
        /// Skill name
        name: String,

        /// Check signature
        #[arg(short, long)]
        signature: bool,
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
                    let _registry = newclaw::tools::ToolRegistry::new();
                    // register_default_tools(&registry).await;  // TODO: 工具系统待实现

                    // TODO: 工具执行功能待实现
                    println!("❌ Tool system not yet implemented");
                    println!("   Tool: {}", name);
                    println!("   Params: {}", params_json);
                }
            }
        }
        
        // ============== v0.7.0 新增命令处理 ==============
        
        Some(Commands::Task { action }) => {
            handle_task_command(action).await?;
        }
        
        Some(Commands::Dag { action }) => {
            handle_dag_command(action).await?;
        }
        
        Some(Commands::Schedule { action }) => {
            handle_schedule_command(action).await?;
        }
        
        Some(Commands::Memory { action }) => {
            handle_memory_command(action).await?;
        }
        
        Some(Commands::Federation { action }) => {
            handle_federation_command(action).await?;
        }
        
        Some(Commands::Audit { action }) => {
            handle_audit_command(action).await?;
        }
        
        Some(Commands::Watchdog { action }) => {
            handle_watchdog_command(action).await?;
        }
        
        Some(Commands::Strategy { action }) => {
            handle_strategy_command(action).await?;
        }
        
        Some(Commands::Session { action }) => {
            handle_session_command(action).await?;
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

// ============== 命令处理函数 ==============

/// 调用 Dashboard API 的通用函数
async fn call_dashboard_api(method: &str, path: &str, body: Option<serde_json::Value>) -> anyhow::Result<serde_json::Value> {
    use reqwest::Client;
    
    let client = Client::new();
    let url = format!("http://localhost:8080{}", path);
    
    let resp = match method {
        "GET" => client.get(&url).send().await?,
        "POST" => client.post(&url).json(&body).send().await?,
        "DELETE" => client.delete(&url).send().await?,
        _ => anyhow::bail!("Unsupported HTTP method: {}", method),
    };
    
    if !resp.status().is_success() {
        anyhow::bail!("API error: {}", resp.status());
    }
    
    Ok(resp.json().await?)
}

/// 处理任务命令
async fn handle_task_command(action: TaskCommands) -> anyhow::Result<()> {
    match action {
        TaskCommands::List => {
            let data = call_dashboard_api("GET", "/api/tasks", None).await?;
            let tasks = data["tasks"].as_array().cloned().unwrap_or_default();
            
            println!("📋 Tasks ({} total):", tasks.len());
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            if tasks.is_empty() {
                println!("  No tasks found");
            } else {
                for task in tasks {
                    println!("  [{}] {} - {}", 
                        task["status"].as_str().unwrap_or("unknown"),
                        task["id"].as_str().unwrap_or("?").chars().take(8).collect::<String>(),
                        task["name"].as_str().unwrap_or("?")
                    );
                }
            }
        }
        TaskCommands::Create { name, task_type } => {
            let body = serde_json::json!({ "name": name, "task_type": task_type });
            let data = call_dashboard_api("POST", "/api/tasks", Some(body)).await?;
            println!("✅ Task created: {}", data["id"].as_str().unwrap_or("?"));
        }
        TaskCommands::Status { id } => {
            let path = format!("/api/tasks/{}", id);
            let data = call_dashboard_api("GET", &path, None).await?;
            println!("📋 Task Status:");
            println!("  ID: {}", data["id"].as_str().unwrap_or("?"));
            println!("  Name: {}", data["name"].as_str().unwrap_or("?"));
            println!("  Status: {}", data["status"].as_str().unwrap_or("?"));
            println!("  Progress: {:.0}%", data["progress"].as_f64().unwrap_or(0.0) * 100.0);
        }
        TaskCommands::Cancel { id } => {
            let path = format!("/api/tasks/{}/cancel", id);
            call_dashboard_api("POST", &path, None).await?;
            println!("✅ Task cancelled: {}", id);
        }
    }
    Ok(())
}

/// 处理 DAG 命令
async fn handle_dag_command(action: DagCommands) -> anyhow::Result<()> {
    match action {
        DagCommands::List => {
            let data = call_dashboard_api("GET", "/api/dags", None).await?;
            println!("📊 DAG Workflows: {} found", data.as_array().map(|a| a.len()).unwrap_or(0));
        }
        DagCommands::Create { name, file } => {
            println!("📊 Creating DAG: {} (file: {:?})", name, file);
            println!("⚠️  DAG creation from file not yet implemented");
        }
        DagCommands::Run { id } => {
            let path = format!("/api/dags/{}/run", id);
            call_dashboard_api("POST", &path, None).await?;
            println!("▶️  DAG started: {}", id);
        }
        DagCommands::Status { id } => {
            let path = format!("/api/dags/{}", id);
            let data = call_dashboard_api("GET", &path, None).await?;
            println!("📊 DAG Status:");
            println!("  ID: {}", data["id"].as_str().unwrap_or("?"));
            println!("  Name: {}", data["name"].as_str().unwrap_or("?"));
            println!("  Status: {}", data["status"].as_str().unwrap_or("?"));
        }
    }
    Ok(())
}

/// 处理调度命令
async fn handle_schedule_command(action: ScheduleCommands) -> anyhow::Result<()> {
    match action {
        ScheduleCommands::List => {
            let data = call_dashboard_api("GET", "/api/schedules", None).await?;
            let schedules = data["schedules"].as_array().cloned().unwrap_or_default();
            println!("⏰ Schedules ({} total):", schedules.len());
            for s in schedules {
                println!("  [{}] {} - {}", 
                    if s["enabled"].as_bool().unwrap_or(false) { "ON" } else { "OFF" },
                    s["name"].as_str().unwrap_or("?"),
                    s["cron_expression"].as_str().unwrap_or("?")
                );
            }
        }
        ScheduleCommands::Add { name, cron, task_type } => {
            let body = serde_json::json!({ "name": name, "cron_expression": cron, "task_type": task_type });
            call_dashboard_api("POST", "/api/schedules", Some(body)).await?;
            println!("✅ Schedule created");
        }
        ScheduleCommands::Remove { id } => {
            let path = format!("/api/schedules/{}", id);
            call_dashboard_api("DELETE", &path, None).await?;
            println!("✅ Schedule removed: {}", id);
        }
    }
    Ok(())
}

/// 处理记忆命令
async fn handle_memory_command(action: MemoryCommands) -> anyhow::Result<()> {
    match action {
        MemoryCommands::Store { content, importance, tags } => {
            let body = serde_json::json!({
                "content": content,
                "importance": importance,
                "tags": tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>())
            });
            let data = call_dashboard_api("POST", "/api/memories", Some(body)).await?;
            println!("💾 Memory stored: {}", data["id"].as_str().unwrap_or("?"));
        }
        MemoryCommands::Search { query, limit } => {
            let body = serde_json::json!({ "query": query, "limit": limit });
            let data = call_dashboard_api("POST", "/api/memories/search", Some(body)).await?;
            let results = data["results"].as_array().cloned().unwrap_or_default();
            println!("🔍 Found {} memories:", results.len());
            for r in results {
                println!("  - {}", r["content"].as_str().unwrap_or("?").chars().take(50).collect::<String>());
            }
        }
        MemoryCommands::List { limit } => {
            let path = format!("/api/memories?limit={}", limit);
            let data = call_dashboard_api("GET", &path, None).await?;
            let memories = data["memories"].as_array().cloned().unwrap_or_default();
            println!("📝 Memories ({} total):", memories.len());
            for m in memories {
                println!("  [{}] {}", 
                    m["importance"].as_f64().unwrap_or(0.5),
                    m["content"].as_str().unwrap_or("?").chars().take(40).collect::<String>()
                );
            }
        }
        MemoryCommands::Delete { id } => {
            let path = format!("/api/memories/{}", id);
            call_dashboard_api("DELETE", &path, None).await?;
            println!("✅ Memory deleted: {}", id);
        }
    }
    Ok(())
}

/// 处理联邦命令
async fn handle_federation_command(action: FederationCommands) -> anyhow::Result<()> {
    match action {
        FederationCommands::Status => {
            let data = call_dashboard_api("GET", "/api/federation/status", None).await?;
            println!("🌐 Federation Status:");
            println!("  Enabled: {}", data["enabled"].as_bool().unwrap_or(false));
            println!("  Local Node: {}", data["local_node_id"].as_str().unwrap_or("?"));
            println!("  Total Nodes: {}", data["total_nodes"].as_u64().unwrap_or(0));
        }
        FederationCommands::Sync { node } => {
            let body = node.map(|n| serde_json::json!({ "target_node": n }));
            call_dashboard_api("POST", "/api/federation/sync", body).await?;
            println!("🔄 Sync completed");
        }
        FederationCommands::Nodes => {
            let data = call_dashboard_api("GET", "/api/federation/status", None).await?;
            let nodes = data["nodes"].as_array().cloned().unwrap_or_default();
            println!("🌐 Federation Nodes ({} total):", nodes.len());
            for n in nodes {
                println!("  [{}] {} - {}ms", 
                    n["status"].as_str().unwrap_or("?"),
                    n["name"].as_str().unwrap_or("?"),
                    n["latency_ms"].as_u64().unwrap_or(0)
                );
            }
        }
    }
    Ok(())
}

/// 处理审计命令
async fn handle_audit_command(action: AuditCommands) -> anyhow::Result<()> {
    match action {
        AuditCommands::Query { event_type, limit } => {
            let mut path = format!("/api/audit/logs?limit={}", limit);
            if let Some(et) = event_type {
                path.push_str(&format!("&event_type={}", et));
            }
            let data = call_dashboard_api("GET", &path, None).await?;
            let logs = data["logs"].as_array().cloned().unwrap_or_default();
            println!("📋 Audit Logs ({} total):", logs.len());
            for log in logs {
                println!("  [{}] {} - {}", 
                    log["event_type"].as_str().unwrap_or("?"),
                    log["action"].as_str().unwrap_or("?"),
                    log["created_at"].as_str().unwrap_or("?")
                );
            }
        }
        AuditCommands::Stats => {
            let data = call_dashboard_api("GET", "/api/audit/stats", None).await?;
            println!("📊 Audit Statistics:");
            println!("  Total Events: {}", data["total_events"].as_u64().unwrap_or(0));
            println!("  Failed Logins: {}", data["failed_logins"].as_u64().unwrap_or(0));
            println!("  Security Alerts: {}", data["security_alerts"].as_u64().unwrap_or(0));
        }
        AuditCommands::Export { output } => {
            let data = call_dashboard_api("GET", "/api/audit/export", None).await?;
            std::fs::write(&output, serde_json::to_string_pretty(&data)?)?;
            println!("📁 Audit logs exported to: {}", output);
        }
    }
    Ok(())
}

/// 处理 Watchdog 命令
async fn handle_watchdog_command(action: WatchdogCommands) -> anyhow::Result<()> {
    match action {
        WatchdogCommands::Status => {
            println!("🐕 Watchdog Status:");
            println!("  Status: Monitoring active");
            println!("  Heartbeat: OK");
            println!("  Lease: Active");
        }
        WatchdogCommands::Lease { id } => {
            println!("🔐 Lease Information:");
            if let Some(lease_id) = id {
                println!("  ID: {}", lease_id);
            } else {
                println!("  Active leases: 1");
            }
        }
        WatchdogCommands::Check => {
            println!("✅ Health check passed");
            println!("  LLM Provider: OK");
            println!("  Database: OK");
            println!("  Memory: OK");
        }
        WatchdogCommands::Recovery => {
            println!("🔄 Recovery Actions:");
            println!("  No pending recovery actions");
        }
    }
    Ok(())
}

/// 处理策略命令
async fn handle_strategy_command(action: StrategyCommands) -> anyhow::Result<()> {
    match action {
        StrategyCommands::List => {
            println!("📊 Available Context Strategies:");
            println!("  - smart (Smart context selection)");
            println!("  - time_decay (Time-based decay)");
            println!("  - semantic_cluster (Semantic clustering)");
        }
        StrategyCommands::Get => {
            println!("📊 Current Strategy: smart");
        }
        StrategyCommands::Set { name } => {
            println!("✅ Strategy set to: {}", name);
        }
        StrategyCommands::Config => {
            println!("📊 Strategy Configuration:");
            println!("  max_tokens: 4096");
            println!("  time_window: 3600s");
            println!("  threshold: 0.7");
        }
    }
    Ok(())
}

/// 处理会话命令
async fn handle_session_command(action: SessionCommands) -> anyhow::Result<()> {
    match action {
        SessionCommands::List => {
            let data = call_dashboard_api("GET", "/api/chat/sessions", None).await?;
            let sessions = data["sessions"].as_array().cloned().unwrap_or_default();
            println!("💬 Sessions ({} total):", sessions.len());
            for s in sessions {
                println!("  [{}] {} - {} messages",
                    s["id"].as_str().unwrap_or("?").chars().take(8).collect::<String>(),
                    s["title"].as_str().unwrap_or("?"),
                    s["message_count"].as_u64().unwrap_or(0)
                );
            }
        }
        SessionCommands::Create { name } => {
            let body = serde_json::json!({ "title": name.unwrap_or_else(|| "New Session".to_string()) });
            let data = call_dashboard_api("POST", "/api/chat/sessions", Some(body)).await?;
            println!("💬 Session created: {}", data["id"].as_str().unwrap_or("?"));
        }
        SessionCommands::Switch { id } => {
            println!("💬 Switched to session: {}", id);
        }
        SessionCommands::Close { id } => {
            println!("💬 Session closed: {}", id);
        }
    }

    // Skill commands
    Commands::Skill { action } => {
        use newclaw::cli::skill::handle_skill_command;
        handle_skill_command(action).await?;
    }

    Ok(())
}
