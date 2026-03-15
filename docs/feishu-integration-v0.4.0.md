# NewClaw v0.4.0 - 飞书集成完善

## 📋 概述

NewClaw v0.4.0 专注于完善飞书集成功能，提供企业级的可靠性和易用性。本版本新增三个核心模块：

1. **事件轮询系统** (`polling.rs`) - 长轮询机制与 WebSocket 协同工作
2. **消息类型支持** (`messages.rs`) - 完整的飞书消息类型支持
3. **错误重试机制** (`retry.rs`) - 指数退避、错误分类、降级策略

## 🎯 核心目标（P0）完成情况

### ✅ 已完成功能

#### 1. WebSocket 连接管理（v0.3.x 已完成）
- 连接池管理（线程安全）
- 心跳检测（30s 间隔）
- 自动重连（指数退避）
- 事件处理（6 种事件类型）

#### 2. 事件轮询系统（✅ 本次完成）
**文件**: `src/feishu_websocket/polling.rs`

**核心组件**:
- `PollingConfig` - 轮询配置
- `EventQueue` - 事件队列（支持最大长度限制）
- `EventPoller` - 事件轮询器（长轮询机制）
- `PollingManager` - 轮询管理器（多应用支持）
- `HybridEventManager` - WebSocket + 轮询混合模式

**特性**:
- 长轮询机制（30s 超时）
- 事件队列管理（1000 条上限）
- 并发事件处理（10 个并发）
- 与 WebSocket 模式协同工作
- 自动降级到轮询模式

**使用示例**:
```rust
use newclaw::feishu_websocket::{EventPoller, PollingConfig};

let config = PollingConfig {
    polling_interval: Duration::from_secs(5),
    long_polling_timeout: Duration::from_secs(30),
    max_queue_size: 1000,
    max_concurrent_handlers: 10,
    ..Default::default()
};

let mut poller = EventPoller::new(config, ws_config, handler);
poller.start().await?;
// ... 处理事件
poller.stop().await?;
```

#### 3. 消息类型支持（✅ 本次完成）
**文件**: `src/feishu_websocket/messages.rs`

**支持的消息类型**:
- ✅ 文本消息 (`TextMessage`)
- ✅ 富文本消息 (`RichTextMessage`)
- ✅ 卡片消息 (`CardMessage`)
- ✅ 图片消息 (`ImageMessage`)
- ✅ 文件消息 (`FileMessage`)
- ✅ 音频消息
- ✅ 媒体消息
- ✅ 贴纸消息

**核心组件**:
- `MessageSender` - 消息发送器
- `BaseMessage` - 消息基础结构
- `TextContent` - 文本内容
- `RichTextContent` - 富文本内容
- `CardContent` - 卡片内容

**使用示例**:

1. **发送文本消息**:
```rust
let msg = TextMessage::new("Hello, World!");
sender.send_text("chat_id", ReceiveIdType::ChatId, msg).await?;
```

2. **发送富文本消息**:
```rust
let rich_text = RichTextContent::new()
    .with_title("任务报告")
    .add_paragraph(vec![
        RichTextParagraph::Text {
            text: "状态: ".to_string(),
            style: None,
        },
        RichTextParagraph::Text {
            text: "已完成 ✅".to_string(),
            style: Some(vec![TextStyle::Bold]),
        },
    ]);

let msg = RichTextMessage::new(rich_text);
sender.send_rich_text("chat_id", ReceiveIdType::ChatId, msg).await?;
```

3. **发送卡片消息**:
```rust
let card = CardContent::new()
    .with_header("任务完成")
    .with_header_template("blue")
    .add_element(CardElement::Div {
        text: Some(CardText::lark_md("**任务已成功完成！**")),
        fields: None,
        extra: None,
    })
    .add_element(CardElement::Divider);

let msg = CardMessage::new(card);
sender.send_card("chat_id", ReceiveIdType::ChatId, msg).await?;
```

#### 4. 错误重试机制（✅ 本次完成）
**文件**: `src/feishu_websocket/retry.rs`

**核心功能**:
- ✅ 指数退避算法
- ✅ 错误分类处理（8 种错误类型）
- ✅ 降级策略（缓存、默认值）
- ✅ 监控和告警

**核心组件**:
- `RetryStrategy` - 重试策略配置
- `RetryExecutor` - 重试执行器
- `RetryManager` - 重试管理器（带监控）
- `ErrorCategory` - 错误分类
- `FallbackStrategy` - 降级策略接口
- `AlertRule` - 告警规则
- `RetryMetrics` - 监控指标

**错误分类**:
| 类别 | 可重试 | 默认延迟 |
|------|--------|----------|
| Network | ✅ | 5s |
| RateLimit | ✅ | 60s |
| ServiceUnavailable | ✅ | 30s |
| Timeout | ✅ | 10s |
| Authentication | ❌ | - |
| Permission | ❌ | - |
| Data | ❌ | - |

**使用示例**:

1. **基本重试**:
```rust
let strategy = RetryStrategy {
    max_attempts: 3,
    initial_delay: Duration::from_secs(1),
    max_delay: Duration::from_secs(60),
    multiplier: 2.0,
    jitter: true,
    ..Default::default()
};

let executor = RetryExecutor::new(strategy);
let result = executor.execute(|| async {
    // 可能失败的操作
    Ok("success".to_string())
}).await?;
```

2. **带降级的重试**:
```rust
let fallback = Arc::new(DefaultValueFallback::new("默认响应"));
let executor = RetryExecutor::new(strategy)
    .with_fallback(fallback);

let result = executor.execute(|| async {
    // 可能失败的操作
}).await?;
```

3. **带监控的重试**:
```rust
let manager = RetryManager::new(strategy)
    .add_alert_rule(AlertRule::new(
        "高错误率",
        ErrorSeverity::High,
        5,  // 5 次失败
        60, // 60 秒窗口
    ))
    .with_alert_callback(|rule, metrics| {
        println!("告警: {}", rule.name);
    });

let result = manager.execute_with_metrics(|| async {
    // 操作
}).await?;

// 查看指标
let metrics = manager.get_metrics().await;
println!("成功率: {}/{}", 
    metrics.successful_retries, 
    metrics.successful_retries + metrics.failed_retries
);
```

## 📊 测试覆盖

所有新增模块都有完整的单元测试：

```bash
# 运行飞书模块测试
cargo test --lib feishu_websocket

# 测试结果
test result: ok. 56 passed; 0 failed
```

**测试覆盖的功能**:
- 事件队列管理（添加、取出、最大长度）
- 消息序列化/反序列化
- 重试策略计算
- 错误分类判断
- 降级策略执行
- 监控指标统计
- 告警规则触发

## 📁 文件结构

```
src/feishu_websocket/
├── mod.rs              # 模块定义和 re-exports
├── pool.rs             # 连接池管理
├── heartbeat.rs        # 心跳检测
├── reconnect.rs        # 重连机制
├── event.rs            # 事件处理
├── manager.rs          # WebSocket 管理器
├── polling.rs          # 🆕 事件轮询系统
├── messages.rs         # 🆕 消息类型支持
└── retry.rs            # 🆕 错误重试机制

examples/
└── feishu_integration_example.rs  # 完整示例
```

## 🚀 快速开始

### 1. 添加依赖

```toml
[dependencies]
newclaw = { version = "0.4.0" }
```

### 2. 基本使用

```rust
use newclaw::feishu_websocket::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建配置
    let config = WebSocketConfig {
        app_id: "your_app_id".to_string(),
        app_secret: "your_secret".to_string(),
        ..Default::default()
    };
    
    // 2. 创建事件处理器
    let handler = Arc::new(MyEventHandler::new());
    
    // 3. 创建管理器
    let manager = FeishuWebSocketManager::new(config, handler);
    
    // 4. 启动连接
    manager.start().await?;
    manager.connect("your_app_id", "your_secret").await?;
    
    // 5. 运行...
    tokio::signal::ctrl_c().await?;
    
    // 6. 清理
    manager.stop().await?;
    
    Ok(())
}
```

### 3. 运行示例

```bash
# 编译项目
cargo build --release

# 运行示例
cargo run --example feishu_integration_example
```

## 📈 性能特性

- **并发处理**: 支持 10 个并发事件处理
- **队列容量**: 1000 条事件队列
- **重试优化**: 指数退避 + 抖动，避免雪崩
- **内存效率**: 使用 Arc 和 RwLock 进行共享
- **零拷贝**: 尽可能避免不必要的数据克隆

## 🔧 配置选项

### WebSocketConfig
```rust
pub struct WebSocketConfig {
    pub base_url: String,              // WebSocket URL
    pub app_id: String,                // 应用 ID
    pub app_secret: String,            // 应用密钥
    pub heartbeat_interval: Duration,  // 心跳间隔（默认 30s）
    pub heartbeat_timeout: Duration,   // 心跳超时（默认 10s）
    pub max_heartbeat_failures: u32,   // 最大失败次数（默认 3）
    pub enable_auto_reconnect: bool,   // 自动重连（默认 true）
    pub max_reconnect_attempts: u32,   // 最大重连次数（默认 10）
    pub max_connections: usize,        // 最大连接数（默认 10）
}
```

### PollingConfig
```rust
pub struct PollingConfig {
    pub polling_interval: Duration,           // 轮询间隔（默认 5s）
    pub long_polling_timeout: Duration,       // 长轮询超时（默认 30s）
    pub max_queue_size: usize,                // 队列大小（默认 1000）
    pub max_concurrent_handlers: usize,       // 并发数（默认 10）
    pub event_processing_timeout: Duration,   // 处理超时（默认 60s）
    pub enable_long_polling: bool,            // 长轮询（默认 true）
    pub max_retries: u32,                     // 重试次数（默认 3）
}
```

### RetryStrategy
```rust
pub struct RetryStrategy {
    pub max_attempts: u32,           // 最大尝试（默认 3）
    pub initial_delay: Duration,     // 初始延迟（默认 1s）
    pub max_delay: Duration,         // 最大延迟（默认 60s）
    pub multiplier: f64,             // 退避倍数（默认 2.0）
    pub jitter: bool,                // 抖动（默认 true）
    pub jitter_range: f64,           // 抖动范围（默认 0.1）
}
```

## 🛠️ 下一步计划（P1）

### Dashboard 开发（3-4 周）
- [ ] 配置界面（LLM、工具、飞书）
- [ ] 监控面板（日志、性能、告警）
- [ ] 对话界面（聊天、历史、流式）
- [ ] 管理功能（用户、权限、API Key）

### 单体间通信（P2）
- [ ] 标准 API 接口
- [ ] Socket 通信
- [ ] 触发机制

## 📝 更新日志

### v0.4.0 (2026-03-09)

**新增功能**:
- ✨ 事件轮询系统（`polling.rs`）
- ✨ 完整消息类型支持（`messages.rs`）
- ✨ 企业级错误重试机制（`retry.rs`）
- ✨ 完整示例代码（`feishu_integration_example.rs`）

**改进**:
- 🎨 WebSocketError 实现 Clone trait
- 📝 完善文档和注释
- ✅ 增加 56 个单元测试

**技术细节**:
- 使用 `Arc<RwLock>` 实现线程安全
- 使用 `tokio::sync::Semaphore` 控制并发
- 使用指数退避算法避免重试风暴
- 支持多种降级策略

## 🤝 贡献

欢迎贡献代码！请确保：
1. 所有测试通过
2. 代码格式化（`cargo fmt`）
3. 没有编译警告
4. 添加必要的文档和注释

## 📄 许可证

MIT License

---

**NewClaw v0.4.0** - Next-gen AI Agent framework
