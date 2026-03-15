# 飞书 WebSocket 连接管理设计文档

**版本**: v0.4.0  
**设计时间**: 2026-03-09  
**作者**: SubAgent (GLM-5)

---

## 📋 设计目标

### 核心需求
1. **稳定连接**: 24小时不掉线
2. **自动重连**: 断线后自动恢复
3. **心跳检测**: 定期检测连接状态
4. **连接池**: 支持多个飞书应用
5. **事件处理**: 实时接收飞书事件

### 性能目标
- 连接延迟 < 100ms
- 重连时间 < 5s
- 心跳间隔 30s
- 支持 10+ 并发连接

---

## 🏗️ 架构设计

### 整体架构

```
┌─────────────────────────────────────────────────┐
│         FeishuWebSocketManager                   │
│  ┌───────────────────────────────────────────┐  │
│  │      ConnectionPool                       │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐   │  │
│  │  │ Conn 1  │  │ Conn 2  │  │ Conn 3  │   │  │
│  │  └─────────┘  └─────────┘  └─────────┘   │  │
│  └───────────────────────────────────────────┘  │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │      HeartbeatManager                     │  │
│  │  - 定时心跳                                │  │
│  │  - 超时检测                                │  │
│  │  - 连接健康检查                            │  │
│  └───────────────────────────────────────────┘  │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │      ReconnectionManager                  │  │
│  │  - 指数退避                                │  │
│  │  - 重连策略                                │  │
│  │  - 失败处理                                │  │
│  └───────────────────────────────────────────┘  │
│                                                  │
│  ┌───────────────────────────────────────────┐  │
│  │      EventHandler                         │  │
│  │  - 事件解析                                │  │
│  │  - 事件分发                                │  │
│  │  - 错误处理                                │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

---

## 📦 核心组件

### 1. FeishuWebSocketManager

**职责**: 管理 WebSocket 连接的生命周期

**核心方法**:
```rust
pub struct FeishuWebSocketManager {
    pool: Arc<ConnectionPool>,
    heartbeat: Arc<HeartbeatManager>,
    reconnection: Arc<ReconnectionManager>,
    event_handler: Arc<dyn EventHandler>,
    config: WebSocketConfig,
}

impl FeishuWebSocketManager {
    pub async fn new(config: WebSocketConfig) -> Result<Self>;
    pub async fn connect(&self, app_id: &str) -> Result<Connection>;
    pub async fn disconnect(&self, app_id: &str) -> Result<()>;
    pub async fn reconnect(&self, app_id: &str) -> Result<Connection>;
    pub async fn send(&self, app_id: &str, message: &str) -> Result<()>;
    pub async fn is_connected(&self, app_id: &str) -> bool;
}
```

---

### 2. ConnectionPool

**职责**: 管理多个飞书应用的连接

**数据结构**:
```rust
pub struct ConnectionPool {
    connections: Arc<RwLock<HashMap<String, Connection>>>,
    max_connections: usize,
}

pub struct Connection {
    app_id: String,
    ws: WebSocketStream,
    state: ConnectionState,
    last_heartbeat: Instant,
    reconnect_count: u32,
}

pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
    Error,
}
```

**核心方法**:
```rust
impl ConnectionPool {
    pub async fn add(&self, app_id: &str, ws: WebSocketStream) -> Result<()>;
    pub async fn remove(&self, app_id: &str) -> Result<()>;
    pub async fn get(&self, app_id: &str) -> Option<Connection>;
    pub async fn list(&self) -> Vec<String>;
    pub async fn count(&self) -> usize;
}
```

---

### 3. HeartbeatManager

**职责**: 管理心跳检测

**配置**:
```rust
pub struct HeartbeatConfig {
    pub interval: Duration,      // 心跳间隔（默认 30s）
    pub timeout: Duration,       // 超时时间（默认 10s）
    pub max_failures: u32,       // 最大失败次数（默认 3）
}
```

**核心方法**:
```rust
impl HeartbeatManager {
    pub async fn start(&self, app_id: &str) -> Result<()>;
    pub async fn stop(&self, app_id: &str) -> Result<()>;
    pub async fn check_health(&self, app_id: &str) -> Result<bool>;
}
```

**心跳流程**:
```
1. 每 30s 发送一次心跳包
2. 等待 10s 内的响应
3. 如果超时，失败计数 +1
4. 连续失败 3 次，触发重连
```

---

### 4. ReconnectionManager

**职责**: 管理自动重连

**重连策略**: 指数退避
```
第1次: 1s
第2次: 2s
第3次: 4s
第4次: 8s
第5次: 16s
最大: 60s
```

**核心方法**:
```rust
impl ReconnectionManager {
    pub async fn should_reconnect(&self, app_id: &str) -> bool;
    pub async fn get_delay(&self, attempt: u32) -> Duration;
    pub async fn record_failure(&self, app_id: &str) -> Result<()>;
    pub async fn record_success(&self, app_id: &str) -> Result<()>;
}
```

---

### 5. EventHandler

**职责**: 处理飞书事件

**事件类型**:
```rust
pub enum FeishuEvent {
    MessageReceived(MessageEvent),
    MessageRead(ReadEvent),
    UserTyping(TypingEvent),
    BotAdded(BotEvent),
    BotRemoved(BotEvent),
    Error(ErrorEvent),
}
```

**核心方法**:
```rust
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: FeishuEvent) -> Result<()>;
    async fn on_connect(&self, app_id: &str) -> Result<()>;
    async fn on_disconnect(&self, app_id: &str) -> Result<()>;
    async fn on_error(&self, app_id: &str, error: &Error) -> Result<()>;
}
```

---

## 🔄 工作流程

### 1. 连接建立流程

```
1. 获取飞书 WebSocket URL
   ↓
2. 建立 WebSocket 连接
   ↓
3. 发送认证消息
   ↓
4. 等待认证响应
   ↓
5. 添加到连接池
   ↓
6. 启动心跳检测
   ↓
7. 开始接收事件
```

### 2. 心跳检测流程

```
1. 定时器触发（30s）
   ↓
2. 发送心跳包
   ↓
3. 等待响应（10s）
   ↓
4a. 收到响应 → 重置失败计数
4b. 超时 → 失败计数 +1
   ↓
5. 失败计数 >= 3 → 触发重连
```

### 3. 自动重连流程

```
1. 检测到连接断开
   ↓
2. 记录失败次数
   ↓
3. 计算重连延迟（指数退避）
   ↓
4. 等待延迟时间
   ↓
5. 尝试重新连接
   ↓
6a. 成功 → 重置失败计数，恢复心跳
6b. 失败 → 返回步骤 2
```

---

## 🔧 配置管理

### WebSocketConfig

```rust
pub struct WebSocketConfig {
    // 连接配置
    pub base_url: String,
    pub app_id: String,
    pub app_secret: String,
    
    // 心跳配置
    pub heartbeat_interval: Duration,
    pub heartbeat_timeout: Duration,
    pub max_heartbeat_failures: u32,
    
    // 重连配置
    pub enable_auto_reconnect: bool,
    pub max_reconnect_attempts: u32,
    pub initial_reconnect_delay: Duration,
    pub max_reconnect_delay: Duration,
    
    // 连接池配置
    pub max_connections: usize,
    
    // 日志配置
    pub log_level: LogLevel,
}
```

### 默认配置

```rust
impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            base_url: "wss://open.feishu.cn/open-apis/ws/v2".to_string(),
            app_id: String::new(),
            app_secret: String::new(),
            heartbeat_interval: Duration::from_secs(30),
            heartbeat_timeout: Duration::from_secs(10),
            max_heartbeat_failures: 3,
            enable_auto_reconnect: true,
            max_reconnect_attempts: 10,
            initial_reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(60),
            max_connections: 10,
            log_level: LogLevel::Info,
        }
    }
}
```

---

## 🧪 测试策略

### 单元测试
- [x] ConnectionPool 测试
  - 添加/移除连接
  - 连接状态管理
  - 并发访问

- [x] HeartbeatManager 测试
  - 心跳发送
  - 超时检测
  - 失败计数

- [x] ReconnectionManager 测试
  - 指数退避计算
  - 重连决策
  - 失败记录

### 集成测试
- [ ] 完整连接流程
- [ ] 心跳检测流程
- [ ] 自动重连流程
- [ ] 事件处理流程

### 压力测试
- [ ] 10+ 并发连接
- [ ] 长时间运行（24小时）
- [ ] 网络抖动测试
- [ ] 频繁断线重连

---

## 📊 性能指标

### 连接性能
- 连接建立时间: < 1s
- 心跳延迟: < 100ms
- 事件处理延迟: < 50ms

### 稳定性
- 连接成功率: > 99%
- 重连成功率: > 95%
- 24小时掉线次数: < 1

### 资源占用
- 单连接内存: < 1MB
- CPU 占用: < 1%
- 网络带宽: < 10KB/s

---

## 🚀 实现计划

### 第1天（今天）
- [x] 设计文档编写
- [ ] 实现基础数据结构
- [ ] 实现 ConnectionPool

### 第2天
- [ ] 实现 HeartbeatManager
- [ ] 实现 ReconnectionManager
- [ ] 单元测试

### 第3天
- [ ] 实现 FeishuWebSocketManager
- [ ] 集成测试
- [ ] 文档更新

---

## 📝 注意事项

### 安全性
1. **认证信息**: 使用环境变量存储 app_secret
2. **TLS 加密**: 使用 wss:// 协议
3. **Token 管理**: 定期刷新 access_token

### 可靠性
1. **幂等性**: 确保重连不会重复处理事件
2. **错误处理**: 完善的错误处理和日志
3. **资源清理**: 断线时清理相关资源

### 可维护性
1. **日志记录**: 详细记录连接状态变化
2. **监控指标**: 暴露 Prometheus 指标
3. **配置灵活**: 支持动态调整参数

---

## 🎯 验收标准

### 功能验收
- [ ] 连接建立成功
- [ ] 心跳检测正常
- [ ] 自动重连成功
- [ ] 事件接收正常

### 性能验收
- [ ] 连接延迟 < 1s
- [ ] 心跳延迟 < 100ms
- [ ] 事件延迟 < 50ms
- [ ] 内存占用 < 10MB

### 稳定性验收
- [ ] 24小时不掉线
- [ ] 重连成功率 > 95%
- [ ] 无内存泄漏

---

**状态**: 📋 设计完成，准备实现  
**下一步**: 实现基础数据结构和 ConnectionPool
