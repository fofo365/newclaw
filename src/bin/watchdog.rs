// NewClaw Watchdog - 核心主控独立二进制
//
// 轻量级看门狗进程，独立于智慧主控运行
// 资源占用: < 32MB 内存
// 功能: 心跳检测、租约管理、故障恢复

use clap::Parser;

#[derive(Parser)]
#[command(name = "newclaw-watchdog")]
#[command(version = "0.6.0")]
#[command(about = "NewClaw Watchdog - Core Controller for HA", long_about = None)]
struct Cli {
    /// gRPC server port
    #[arg(short, long, default_value = "50051")]
    port: u16,
    
    /// gRPC server host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    /// Check interval in seconds
    #[arg(short, long, default_value = "5")]
    check_interval: u64,
    
    /// Heartbeat timeout in seconds
    #[arg(long, default_value = "15")]
    heartbeat_timeout: u64,
    
    /// Max heartbeat failures before recovery
    #[arg(long, default_value = "3")]
    max_failures: u32,
    
    /// Lease duration in seconds
    #[arg(long, default_value = "15")]
    lease_duration: u64,
    
    /// Config file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
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
    
    tracing::info!("🐕 NewClaw Watchdog v0.6.0 starting...");
    
    // 构建配置
    let config = newclaw::watchdog::config::WatchdogConfig {
        check_interval: cli.check_interval,
        heartbeat_timeout: cli.heartbeat_timeout,
        max_heartbeat_failures: cli.max_failures,
        lease: newclaw::watchdog::config::LeaseConfig {
            duration: cli.lease_duration,
            renew_deadline: 10,
            storage: newclaw::watchdog::config::LeaseStorageType::Redis,
            redis_url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
        },
        ..Default::default()
    };
    
    tracing::info!("   Check interval: {}s", config.check_interval);
    tracing::info!("   Heartbeat timeout: {}s", config.heartbeat_timeout);
    tracing::info!("   Max failures: {}", config.max_heartbeat_failures);
    tracing::info!("   Lease duration: {}s", config.lease.duration);
    
    // 创建核心控制器
    let controller = newclaw::watchdog::CoreController::new(config.clone());
    
    tracing::info!("🚀 Watchdog ready on {}:{}", cli.host, cli.port);
    
    // 启动 gRPC 服务器
    let grpc_server = newclaw::watchdog::grpc::WatchdogGrpcServer::new(controller);
    let addr = format!("{}:{}", cli.host, cli.port);
    
    grpc_server.serve(&addr).await?;
    
    Ok(())
}