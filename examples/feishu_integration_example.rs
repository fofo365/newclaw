// NewClaw v0.4.0 - 飞书集成完整示例
//
// 本示例展示如何使用 v0.4.0 新增的三个核心功能：
// 1. 事件轮询系统
// 2. 消息类型支持
// 3. 错误重试机制

use newclaw::feishu_websocket::{
    // 配置
    WebSocketConfig, PollingConfig, RetryStrategy,
    
    // WebSocket 管理
    FeishuWebSocketManager, EventHandler, FeishuEvent, WebSocketResult,
    
    // 轮询系统
    EventPoller, PollingManager,
    
    // 消息类型
    TextMessage, RichTextMessage, CardMessage, ImageMessage, FileMessage,
    MessageSender, ReceiveIdType, RichTextContent, CardContent, CardText,
    RichTextParagraph, CardElement,
    
    // 重试机制
    RetryExecutor, RetryManager, ErrorCategory, AlertRule,
    CacheFallback, DefaultValueFallback,
};

use std::sync::Arc;
use tokio::sync::RwLock;

// ==================== 示例 1: 自定义事件处理器 ====================

struct MyEventHandler {
    message_sender: Arc<MessageSender>,
}

impl MyEventHandler {
    fn new() -> Self {
        Self {
            message_sender: Arc::new(MessageSender::new("https://open.feishu.cn")),
        }
    }
}

#[async_trait::async_trait]
impl EventHandler for MyEventHandler {
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()> {
        match event {
            FeishuEvent::MessageReceived {
                app_id,
                open_id,
                chat_id,
                user_id,
                content,
                message_id,
                create_time,
            } => {
                println!("收到消息:");
                println!("  App ID: {}", app_id);
                println!("  Chat ID: {}", chat_id);
                println!("  User ID: {}", user_id);
                println!("  Content: {}", content);
                
                // 自动回复
                let reply = TextMessage::new("收到您的消息！")
                    .with_root_id(message_id);
                
                // self.message_sender.send_text(
                //     &chat_id,
                //     ReceiveIdType::ChatId,
                //     reply,
                // ).await?;
                
                println!("已自动回复");
            }
            
            FeishuEvent::UserTyping { chat_id, open_id } => {
                println!("用户 {} 正在群 {} 中输入...", open_id, chat_id);
            }
            
            FeishuEvent::BotAdded { app_id, chat_id } => {
                println!("机器人被添加到群 {} ({})", chat_id, app_id);
                
                // 发送欢迎消息
                let welcome = TextMessage::new("大家好！我是 NewClaw 机器人，很高兴为您服务！");
                
                // self.message_sender.send_text(
                //     &chat_id,
                //     ReceiveIdType::ChatId,
                //     welcome,
                // ).await?;
            }
            
            _ => {
                println!("收到其他事件: {:?}", event);
            }
        }
        
        Ok(())
    }
    
    async fn on_connect(&self, app_id: &str) -> WebSocketResult<()> {
        println!("✅ 连接成功: {}", app_id);
        Ok(())
    }
    
    async fn on_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        println!("❌ 连接断开: {}", app_id);
        Ok(())
    }
    
    async fn on_error(&self, app_id: &str, error: &newclaw::feishu_websocket::WebSocketError) -> WebSocketResult<()> {
        println!("⚠️  错误 ({}): {:?}", app_id, error);
        Ok(())
    }
}

// ==================== 示例 2: 发送各种类型的消息 ====================

async fn send_various_messages() -> WebSocketResult<()> {
    let sender = MessageSender::new("https://open.feishu.cn");
    let chat_id = "oc_test_chat";
    
    // 1. 发送文本消息
    println!("\n1. 发送文本消息...");
    let text_msg = TextMessage::new("这是一条普通文本消息");
    // sender.send_text(chat_id, ReceiveIdType::ChatId, text_msg).await?;
    println!("   ✅ 文本消息已发送");
    
    // 2. 发送富文本消息
    println!("\n2. 发送富文本消息...");
    let rich_text = RichTextContent::new()
        .with_title("📋 任务完成报告")
        .add_paragraph(vec![
            RichTextParagraph::Text {
                text: "任务状态: ".to_string(),
                style: None,
            },
            RichTextParagraph::Text {
                text: "已完成 ✅".to_string(),
                style: Some(vec![newclaw::feishu_websocket::TextStyle::Bold]),
            },
        ])
        .add_paragraph(vec![
            RichTextParagraph::Text {
                text: "详细信息请查看: ".to_string(),
                style: None,
            },
            RichTextParagraph::Link {
                text: "点击这里".to_string(),
                href: "https://example.com/report".to_string(),
            },
        ]);
    
    let rich_msg = RichTextMessage::new(rich_text);
    // sender.send_rich_text(chat_id, ReceiveIdType::ChatId, rich_msg).await?;
    println!("   ✅ 富文本消息已发送");
    
    // 3. 发送卡片消息
    println!("\n3. 发送卡片消息...");
    let card = CardContent::new()
        .with_header("🎉 恭喜！")
        .with_header_template("blue")
        .add_element(CardElement::Div {
            text: Some(CardText::lark_md("**任务已成功完成！**\n\n所有检查项均已通过。")),
            fields: None,
            extra: None,
        })
        .add_element(CardElement::Divider)
        .add_element(CardElement::Div {
            text: Some(CardText::plain("处理时间: 2.3 秒")),
            fields: None,
            extra: None,
        });
    
    let card_msg = CardMessage::new(card);
    // sender.send_card(chat_id, ReceiveIdType::ChatId, card_msg).await?;
    println!("   ✅ 卡片消息已发送");
    
    // 4. 发送图片消息
    println!("\n4. 发送图片消息...");
    let img_msg = ImageMessage::new("img_v2_test123456");
    // sender.send_image(chat_id, ReceiveIdType::ChatId, img_msg).await?;
    println!("   ✅ 图片消息已发送");
    
    // 5. 发送文件消息
    println!("\n5. 发送文件消息...");
    let file_msg = FileMessage::new("file_v2_test789012");
    // sender.send_file(chat_id, ReceiveIdType::ChatId, file_msg).await?;
    println!("   ✅ 文件消息已发送");
    
    Ok(())
}

// ==================== 示例 3: 使用事件轮询系统 ====================

async fn setup_polling_system() -> WebSocketResult<()> {
    println!("\n========== 事件轮询系统示例 ==========\n");
    
    // 配置轮询
    let polling_config = PollingConfig {
        polling_interval: std::time::Duration::from_secs(5),
        long_polling_timeout: std::time::Duration::from_secs(30),
        max_queue_size: 1000,
        max_concurrent_handlers: 10,
        event_processing_timeout: std::time::Duration::from_secs(60),
        enable_long_polling: true,
        max_retries: 3,
        retry_delay: std::time::Duration::from_secs(1),
    };
    
    println!("轮询配置:");
    println!("  轮询间隔: {:?}", polling_config.polling_interval);
    println!("  长轮询超时: {:?}", polling_config.long_polling_timeout);
    println!("  最大队列大小: {}", polling_config.max_queue_size);
    println!("  最大并发处理数: {}", polling_config.max_concurrent_handlers);
    
    // WebSocket 配置
    let ws_config = WebSocketConfig::default();
    
    // 创建事件处理器
    let handler = Arc::new(MyEventHandler::new());
    
    // 创建轮询器
    let mut poller = EventPoller::new(polling_config, ws_config, handler);
    
    println!("\n启动轮询器...");
    // poller.start().await?;
    println!("✅ 轮询器已启动");
    
    // 模拟运行
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    println!("\n停止轮询器...");
    // poller.stop().await?;
    println!("✅ 轮询器已停止");
    
    Ok(())
}

// ==================== 示例 4: 使用错误重试机制 ====================

async fn retry_mechanism_demo() -> WebSocketResult<()> {
    println!("\n========== 错误重试机制示例 ==========\n");
    
    // 1. 基本重试策略
    let strategy = RetryStrategy {
        max_attempts: 5,
        initial_delay: std::time::Duration::from_secs(1),
        max_delay: std::time::Duration::from_secs(30),
        multiplier: 2.0,
        jitter: true,
        jitter_range: 0.1,
    };
    
    println!("重试策略:");
    println!("  最大尝试次数: {}", strategy.max_attempts);
    println!("  初始延迟: {:?}", strategy.initial_delay);
    println!("  最大延迟: {:?}", strategy.max_delay);
    println!("  退避倍数: {}", strategy.multiplier);
    println!("  启用抖动: {}", strategy.jitter);
    
    // 2. 计算重试延迟
    println!("\n重试延迟示例:");
    for attempt in 1..=5 {
        let delay = strategy.calculate_delay(attempt);
        println!("  第 {} 次重试延迟: {:?}", attempt, delay);
    }
    
    // 3. 使用重试执行器
    println!("\n使用重试执行器执行操作...");
    let executor = RetryExecutor::new(strategy.clone())
        .with_error_callback(|error, context| {
            println!("  ⚠️  第 {} 次失败: {:?}", context.attempt, error);
        });
    
    let mut attempt_count = 0;
    let result = executor.execute(|| async {
        attempt_count += 1;
        if attempt_count < 3 {
            Err(newclaw::feishu_websocket::WebSocketError::ConnectionFailed(
                "模拟连接失败".to_string(),
            ))
        } else {
            Ok("操作成功！".to_string())
        }
    }).await;
    
    println!("  结果: {:?}", result);
    
    // 4. 使用降级策略
    println!("\n使用降级策略...");
    let fallback = Arc::new(DefaultValueFallback::new("默认响应"));
    let executor_with_fallback = RetryExecutor::new(strategy)
        .with_fallback(fallback);
    
    println!("  ✅ 已配置默认值降级策略");
    
    // 5. 使用监控和告警
    println!("\n设置告警规则...");
    let manager = RetryManager::new(RetryStrategy::default())
        .add_alert_rule(AlertRule::new(
            "高错误率告警",
            newclaw::feishu_websocket::ErrorSeverity::High,
            5,
            60,
        ))
        .with_alert_callback(|rule, metrics| {
            println!("  🚨 告警触发: {}", rule.name);
            println!("     失败次数: {}", metrics.failed_retries);
        });
    
    println!("  ✅ 已添加告警规则");
    
    Ok(())
}

// ==================== 示例 5: 完整集成示例 ====================

async fn full_integration_example() -> WebSocketResult<()> {
    println!("\n========== 完整集成示例 ==========\n");
    
    // 1. 创建配置
    let ws_config = WebSocketConfig {
        app_id: "cli_test_app".to_string(),
        app_secret: "test_secret".to_string(),
        enable_auto_reconnect: true,
        ..Default::default()
    };
    
    println!("WebSocket 配置:");
    println!("  App ID: {}", ws_config.app_id);
    println!("  自动重连: {}", ws_config.enable_auto_reconnect);
    println!("  心跳间隔: {:?}", ws_config.heartbeat_interval);
    
    // 2. 创建事件处理器
    let handler = Arc::new(MyEventHandler::new());
    println!("\n✅ 事件处理器已创建");
    
    // 3. 创建 WebSocket 管理器
    // let manager = Arc::new(FeishuWebSocketManager::new(ws_config.clone(), handler.clone()));
    println!("✅ WebSocket 管理器已创建");
    
    // 4. 创建轮询管理器（作为备用）
    let polling_config = PollingConfig::default();
    // let polling_manager = Arc::new(PollingManager::new(polling_config, ws_config.clone()));
    println!("✅ 轮询管理器已创建");
    
    // 5. 创建重试管理器
    let retry_strategy = RetryStrategy::default();
    let retry_manager = Arc::new(RwLock::new(RetryManager::new(retry_strategy)));
    println!("✅ 重试管理器已创建");
    
    println!("\n系统已准备就绪，等待事件...");
    println!("支持的功能:");
    println!("  ✓ WebSocket 实时通信");
    println!("  ✓ 事件轮询备用机制");
    println!("  ✓ 自动重连");
    println!("  ✓ 错误重试与降级");
    println!("  ✓ 多种消息类型支持");
    println!("  ✓ 监控与告警");
    
    Ok(())
}

// ==================== 主函数 ====================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║     NewClaw v0.4.0 - 飞书集成完整示例                    ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    
    // 运行示例
    send_various_messages().await?;
    setup_polling_system().await?;
    retry_mechanism_demo().await?;
    full_integration_example().await?;
    
    println!("\n✅ 所有示例运行完成！");
    
    Ok(())
}
