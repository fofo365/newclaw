// NewClaw v0.4.0 - 飞书 WebSocket 使用示例
//
// 演示如何使用新的 WebSocket 管理器连接飞书

use newclaw::{
    FeishuWebSocketManager, WebSocketConfig, EventHandler, FeishuEvent,
    WebSocketResult, LogLevel,
};
use async_trait::async_trait;
use std::sync::Arc;

/// 自定义事件处理器
struct MyEventHandler;

#[async_trait]
impl EventHandler for MyEventHandler {
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()> {
        match event {
            FeishuEvent::MessageReceived { open_id, content, .. } => {
                println!("收到来自 {} 的消息: {}", open_id, content);
            }
            FeishuEvent::UserTyping { open_id, .. } => {
                println!("用户 {} 正在输入...", open_id);
            }
            _ => {
                println!("收到事件: {:?}", event);
            }
        }
        Ok(())
    }
    
    async fn on_connect(&self, app_id: &str) -> WebSocketResult<()> {
        println!("✅ 应用 {} 已连接", app_id);
        Ok(())
    }
    
    async fn on_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        println!("❌ 应用 {} 已断开", app_id);
        Ok(())
    }
    
    async fn on_error(&self, app_id: &str, error: &newclaw::WebSocketError) -> WebSocketResult<()> {
        println!("⚠️  应用 {} 发生错误: {:?}", app_id, error);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("🚀 NewClaw v0.4.0 - 飞书 WebSocket 示例\n");
    
    // 创建配置
    let config = WebSocketConfig {
        base_url: "wss://open.feishu.cn/open-apis/ws/v2".to_string(),
        app_id: "cli_test_app_id".to_string(),
        app_secret: "test_app_secret".to_string(),
        heartbeat_interval: std::time::Duration::from_secs(30),
        heartbeat_timeout: std::time::Duration::from_secs(10),
        max_heartbeat_failures: 3,
        enable_auto_reconnect: true,
        max_reconnect_attempts: 10,
        initial_reconnect_delay: std::time::Duration::from_secs(1),
        max_reconnect_delay: std::time::Duration::from_secs(60),
        max_connections: 10,
        log_level: LogLevel::Info,
    };
    
    // 创建事件处理器
    let handler = Arc::new(MyEventHandler);
    
    // 创建 WebSocket 管理器
    let manager = FeishuWebSocketManager::new(config, handler);
    
    println!("✅ WebSocket 管理器已创建");
    
    // 启动管理器
    manager.start().await?;
    println!("✅ WebSocket 管理器已启动");
    
    // 连接到飞书（示例）
    // 注意：这里需要真实的 app_id 和 app_secret
    // manager.connect("your_app_id", "your_app_secret").await?;
    
    println!("\n📋 当前功能:");
    println!("  - ✅ 连接池管理（支持多应用）");
    println!("  - ✅ 自动重连（指数退避）");
    println!("  - ✅ 心跳检测（30s 间隔）");
    println!("  - ✅ 事件处理（消息、输入、错误）");
    println!("  - ✅ 线程安全（Arc<RwLock>）");
    println!("  - ✅ 异步支持（Tokio）");
    
    println!("\n📊 统计信息:");
    println!("  - 连接数: {}", manager.connection_count().await);
    println!("  - 活跃连接: {:?}", manager.list_connections().await);
    
    // 停止管理器
    manager.stop().await?;
    println!("\n✅ WebSocket 管理器已停止");
    
    println!("\n🎉 示例运行完成！");
    
    Ok(())
}
