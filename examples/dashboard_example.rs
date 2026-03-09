//! NewClaw Dashboard 示例
//!
//! 演示如何启动 Dashboard 服务器

use newclaw::dashboard::{DashboardConfig, start_dashboard};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();
    
    println!("🦀 NewClaw Dashboard v{}", env!("CARGO_PKG_VERSION"));
    println!();
    
    // 配置 Dashboard
    let config = DashboardConfig {
        enabled: true,
        port: 8080,
        auth_enabled: false,
        jwt_secret: "your-secret-key".to_string(),
        session_timeout_secs: 3600,
        log_retention: 1000,
        metrics_retention_secs: 3600,
    };
    
    println!("📊 Dashboard 配置:");
    println!("  - 端口: {}", config.port);
    println!("  - 认证: {}", if config.auth_enabled { "已启用" } else { "已禁用" });
    println!("  - 会话超时: {}秒", config.session_timeout_secs);
    println!();
    
    // 启动 Dashboard
    println!("🚀 启动 Dashboard 服务器...");
    println!("   访问 http://localhost:{}", config.port);
    println!();
    
    start_dashboard(config).await?;
    
    Ok(())
}
