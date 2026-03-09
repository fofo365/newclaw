#!/usr/bin/env rust-script
//! NewClaw Feishu WebSocket Daemon
//!
//! 维持与飞书服务器的长连接，接收事件推送

use newclaw::config::Config;
use newclaw::feishu_websocket::{
    EventHandler, FeishuEvent, FeishuWebSocketManager, WebSocketConfig,
};
use std::time::Duration;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::fmt;

struct SimpleEventHandler;

impl EventHandler for SimpleEventHandler {
    async fn handle_event(&self, event: FeishuEvent) {
        info!("📬 Received event: {:?}", event);
        
        // 这里可以转发给 Gateway 处理
        // 或者直接处理事件
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("🚀 NewClaw Feishu WebSocket Daemon starting...");

    // 加载配置
    let config_path = "/etc/newclaw/config.toml";
    let config = Config::from_file(config_path)?;

    // 获取飞书配置
    let feishu_accounts = &config.feishu.accounts;
    let account = feishu_accounts
        .values()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No Feishu account configured"))?;

    if !account.enabled {
        error!("❌ Feishu is disabled in config");
        return Ok(());
    }

    // 创建 WebSocket 配置
    let ws_config = WebSocketConfig {
        base_url: "wss://open.feishu.cn/open-apis/ws/v2".to_string(),
        app_id: account.app_id.clone(),
        app_secret: account.app_secret.clone().unwrap_or_default(),
        heartbeat_interval: Duration::from_secs(30),
        heartbeat_timeout: Duration::from_secs(10),
        max_heartbeat_failures: 3,
        enable_auto_reconnect: true,
        max_reconnect_attempts: 10,
        initial_reconnect_delay: Duration::from_secs(1),
        max_reconnect_delay: Duration::from_secs(60),
        max_connections: 10,
        log_level: newclaw::feishu_websocket::LogLevel::Info,
    };

    // 创建事件处理器
    let event_handler = SimpleEventHandler;

    // 创建管理器
    let manager = FeishuWebSocketManager::new(ws_config, std::sync::Arc::new(event_handler));

    // 启动管理器
    manager.start().await?;

    info!("✅ Feishu WebSocket Manager started");
    info!("   App ID: {}", account.app_id);
    info!("   Listening for events...");

    // 等待终止信号
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received CTRL+C, shutting down...");
        }
        _ = terminate => {
            info!("Received terminate signal, shutting down...");
        }
    }

    // 停止管理器
    manager.stop().await?;

    info!("👋 Feishu WebSocket Daemon stopped");

    Ok(())
}
