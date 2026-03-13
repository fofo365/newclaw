// NewClaw Smart Controller - 智慧主控独立二进制
//
// 业务逻辑处理进程，向核心主控上报心跳
// 资源占用: 50-200MB 内存
// 功能: Agent 引擎、工具执行、消息处理

use clap::Parser;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "newclaw-smart")]
#[command(version = "0.6.1")]
#[command(about = "NewClaw Smart Controller - Business Logic Processor", long_about = None)]
struct Cli {
    /// Watchdog gRPC address
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    watchdog_addr: String,
    
    /// Gateway port
    #[arg(short, long, default_value = "3000")]
    port: u16,
    
    /// Gateway host
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    
    /// Heartbeat interval in seconds
    #[arg(long, default_value = "3")]
    heartbeat_interval: u64,
    
    /// Memory threshold in MB for degraded mode
    #[arg(long, default_value = "500")]
    memory_threshold: u64,
    
    /// CPU threshold in percent for degraded mode
    #[arg(long, default_value = "80")]
    cpu_threshold: u64,
    
    /// Config file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
    
    /// Disable watchdog integration (standalone mode)
    #[arg(long)]
    standalone: bool,
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
    
    tracing::info!("🧠 NewClaw Smart Controller v0.6.1 starting...");
    
    // 构建配置
    let mut config = if let Some(config_path) = &cli.config {
        newclaw::config::Config::from_file(config_path)?
    } else {
        newclaw::config::Config::default()
    };
    
    // 覆盖命令行参数
    config.gateway.host = cli.host.clone();
    config.gateway.port = cli.port;
    config.gateway.enable_watchdog = !cli.standalone;
    config.gateway.watchdog_addr = cli.watchdog_addr.clone();
    
    // 启动 Gateway 服务（内置智慧主控集成）
    tracing::info!("🌐 Starting Gateway on {}:{}", config.gateway.host, config.gateway.port);
    newclaw::gateway::run_server(config).await?;
    
    Ok(())
}