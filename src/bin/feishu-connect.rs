// NewClaw 飞书 WebSocket 长连接服务
//
// 独立服务，用于接收飞书消息并通过内部通道转发给 Gateway

use anyhow::Result;
use tracing::{info, error, warn};
use std::sync::Arc;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "newclaw=info".to_string())
        )
        .init();

    info!("🚀 NewClaw Feishu WebSocket 长连接服务启动...");

    // 加载配置
    let config = load_config()?;

    // 检查飞书配置
    if config.feishu.accounts.is_empty() {
        warn!("未配置飞书账号，服务将退出");
        warn!("请在 /etc/newclaw/config.toml 中配置 [feishu.accounts.*]");
        return Ok(());
    }

    info!("找到 {} 个飞书账号配置", config.feishu.accounts.len());

    // 检查并刷新过期的token
    for (account_name, account_config) in &config.feishu.accounts {
        if !account_config.enabled {
            info!("账号 {} 已禁用，跳过", account_name);
            continue;
        }

        // 检查token是否过期或即将过期（提前5分钟刷新）
        let need_refresh = account_config.access_token.is_none() 
            || account_config.token_expires_at.map_or(true, |exp| {
                let now = Utc::now().timestamp();
                exp - now < 300 // 5分钟内过期
            });

        if need_refresh {
            info!("账号 {} 的token已过期或即将过期，尝试刷新...", account_name);
            
            // 获取新token
            match fetch_access_token(&account_config.app_id, &account_config.app_secret).await {
                Ok((token, expires_in)) => {
                    info!("✅ 成功刷新账号 {} 的access_token，有效期 {} 秒", account_name, expires_in);
                    // TODO: 更新配置文件
                }
                Err(e) => {
                    error!("❌ 刷新账号 {} 的token失败: {}", account_name, e);
                }
            }
        } else {
            info!("账号 {} 的token仍然有效", account_name);
        }
    }

    // 创建 WebSocket 管理器（使用默认事件处理器）
    let ws_config = newclaw::feishu_websocket::WebSocketConfig {
        base_url: "https://open.feishu.cn/open-apis".to_string(),
        app_id: String::new(),
        app_secret: String::new(),
        heartbeat_interval: std::time::Duration::from_secs(30),
        heartbeat_timeout: std::time::Duration::from_secs(10),
        max_heartbeat_failures: 3,
        enable_auto_reconnect: true,
        max_reconnect_attempts: 10,
        initial_reconnect_delay: std::time::Duration::from_secs(1),
        max_reconnect_delay: std::time::Duration::from_secs(60),
        max_connections: 10,
        log_level: newclaw::feishu_websocket::LogLevel::Info,
    };

    let event_handler = Arc::new(newclaw::feishu_websocket::event::DefaultEventHandler);
    let manager = Arc::new(newclaw::feishu_websocket::FeishuWebSocketManager::new(ws_config, event_handler));

    // 启动管理器
    manager.start().await?;

    // 为每个账号启动连接
    for (account_name, account_config) in &config.feishu.accounts {
        if !account_config.enabled {
            info!("账号 {} 已禁用，跳过", account_name);
            continue;
        }

        info!("启动账号 {} 的飞书连接...", account_name);

        if let Err(e) = manager.connect(&account_config.app_id, &account_config.app_secret).await {
            error!("启动账号 {} 连接失败: {}", account_name, e);
        } else {
            info!("账号 {} 连接成功", account_name);
        }
    }

    info!("✅ 所有飞书连接已启动，等待消息...");

    // 等待终止信号
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())?;
        let mut sigint = signal(SignalKind::interrupt())?;

        tokio::select! {
            _ = sigterm.recv() => {
                info!("收到 SIGTERM 信号，正在关闭服务...");
            }
            _ = sigint.recv() => {
                info!("收到 SIGINT 信号，正在关闭服务...");
            }
        }
    }

    info!("👋 NewClaw Feishu WebSocket 长连接服务已停止");
    Ok(())
}

/// 加载配置
fn load_config() -> Result<newclaw::config::Config> {
    let config_path = std::env::var("NEWCLAW_CONFIG")
        .unwrap_or_else(|_| "/etc/newclaw/config.toml".to_string());

    let config = newclaw::config::Config::from_file(&config_path)?;
    info!("已加载配置: {}", config_path);
    Ok(config)
}

/// 获取飞书 access_token
async fn fetch_access_token(app_id: &str, app_secret: &str) -> Result<(String, u32)> {
    use serde_json::json;
    
    let client = reqwest::Client::new();
    let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
    
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&json!({
            "app_id": app_id,
            "app_secret": app_secret,
        }))
        .send()
        .await?;
    
    let json: serde_json::Value = response.json().await?;
    
    if json["code"].as_i64() != Some(0) {
        return Err(anyhow::anyhow!("Feishu API error: {:?}", json["msg"]));
    }
    
    let token = json["tenant_access_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No token in response"))?
        .to_string();
    
    let expires_in = json["expire"].as_u64().unwrap_or(7200) as u32;
    
    Ok((token, expires_in))
}