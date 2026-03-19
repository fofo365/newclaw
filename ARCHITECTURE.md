# Enhanced Monolith Architecture - v0.2.0

## 架构概览

```
┌─────────────────────────────────────────────────────┐
│                  Security Layer                     │
│  API Key | JWT | RBAC | Audit | Rate Limit        │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│              Inter-Agent Communication              │
│         (Socket + API + Message Bus)                │
└────────────────────┬────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────┐
│               Core Agent (Single)                   │
│  - AgentEngine                                       │
│  - ContextManager (with optional isolation)          │
│  - PluginSystem                                      │
└────────────────────┬────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
    Channels     Skills      Storage
```

## 1. 单体间通信接口

### 1.1 通信协议

支持三种通信方式：

```rust
pub enum CommProtocol {
    WebSocket,      // 实时双向通信
    HTTP,           // RESTful API
    MessageQueue,   // 异步消息队列
}
```

### 1.2 消息格式

标准化的消息格式：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterAgentMessage {
    pub id: MessageId,
    pub from: AgentId,
    pub to: AgentId,
    pub timestamp: i64,
    pub payload: MessagePayload,
    pub priority: MessagePriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    Request(Request),
    Response(Response),
    Event(Event),
    Command(Command),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}
```

### 1.3 通信接口

```rust
#[async_trait]
pub trait AgentCommunicator: Send + Sync {
    // 发送消息
    async fn send(&mut self, msg: InterAgentMessage) -> Result<()>;
    
    // 接收消息
    async fn receive(&mut self) -> Result<InterAgentMessage>;
    
    // 广播消息
    async fn broadcast(&mut self, msg: InterAgentMessage) -> Result<()>;
    
    // 订阅消息
    async fn subscribe(&mut self, topic: &str) -> Result<()>;
    
    // 心跳检测
    async fn heartbeat(&mut self) -> Result<bool>;
}
```

## 2. WebSocket 通信实现

### 2.1 WebSocket Server

```rust
pub use tokio_tungstenite::tungstenite::Message;

pub struct WebSocketServer {
    addr: SocketAddr,
    clients: Arc<RwLock<HashMap<AgentId, WebSocketSink>>>,
}

impl WebSocketServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("WebSocket server listening on {}", self.addr);
        
        while let Ok((stream, addr)) = listener.accept().await {
            let ws_stream = accept_async(stream).await?;
            let clients = self.clients.clone();
            
            tokio::spawn(async move {
                let mut agent_id = None;
                
                // 握手
                if let Ok(msg) = ws_stream.into_stream().next().await {
                    if let Some(Ok(Message::Text(text))) = msg {
                        if let Ok(handshake) = serde_json::from_str::<Handshake>(&text) {
                            agent_id = Some(handshake.agent_id);
                            
                            // 注册客户端
                            clients.write().await.insert(
                                handshake.agent_id.clone(),
                                WebSocketSink::new(ws_stream)
                            );
                        }
                    }
                }
                
                // 处理消息
                // ...
            });
        }
        
        Ok(())
    }
}
```

### 2.2 WebSocket Client

```rust
pub struct WebSocketClient {
    url: String,
    agent_id: AgentId,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebSocketClient {
    pub async fn connect(url: String, agent_id: AgentId) -> Result<Self> {
        let (stream, _) = connect_async(&url).await?;
        
        // 发送握手
        let handshake = Handshake {
            agent_id: agent_id.clone(),
            protocol_version: "1.0".to_string(),
        };
        
        let mut client = Self {
            url,
            agent_id,
            stream: Some(stream),
        };
        
        client.send(Message::Text(serde_json::to_string(&handshake)?)).await?;
        
        Ok(client)
    }
    
    pub async fn send(&mut self, msg: InterAgentMessage) -> Result<()> {
        if let Some(stream) = &mut self.stream {
            let text = serde_json::to_string(&msg)?;
            stream.send(Message::Text(text)).await?;
        }
        Ok(())
    }
    
    pub async fn receive(&mut self) -> Result<InterAgentMessage> {
        if let Some(stream) = &mut self.stream {
            if let Some(msg) = stream.next().await {
                match msg? {
                    Message::Text(text) => {
                        return Ok(serde_json::from_str(&text)?);
                    }
                    _ => {}
                }
            }
        }
        Err(anyhow::anyhow!("No message"))
    }
}

#[derive(Serialize, Deserialize)]
struct Handshake {
    agent_id: AgentId,
    protocol_version: String,
}
```

## 3. HTTP API 通信

### 3.1 HTTP Server

```rust
pub struct HttpApiServer {
    addr: SocketAddr,
}

impl HttpApiServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
    
    pub fn start(&self) -> Router {
        let app = Router::new()
            .route("/send", post(send_message))
            .route("/receive", get(receive_messages))
            .route("/agents", post(register_agent))
            .route("/agents/:id", get(get_agent_info))
            .route("/broadcast", post(broadcast_message));
        
        let addr = self.addr;
        tokio::spawn(async move {
            let listener = TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        });
        
        app
    }
}

async fn send_message(
    Json(msg): Json<InterAgentMessage>,
) -> Result<Json<InterAgentMessage>, AppError> {
    // 验证权限
    validate_api_key()?;
    
    // 路由消息
    route_message(msg).await
}
```

### 3.2 HTTP Client

```rust
pub struct HttpClient {
    base_url: String,
    api_key: String,
}

impl HttpClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self { base_url, api_key }
    }
    
    pub async fn send(&self, msg: InterAgentMessage) -> Result<InterAgentMessage> {
        let client = reqwest::Client::new();
        let url = format!("{}/send", self.base_url);
        
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&msg)
            .send()
            .await?;
        
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!("HTTP error: {}", response.status()))
        }
    }
}
```

## 4. 消息队列（可选）

### 4.1 Redis 实现

```rust
pub struct RedisMessageQueue {
    client: redis::Client,
    agent_id: AgentId,
}

impl RedisMessageQueue {
    pub async fn new(url: &str, agent_id: AgentId) -> Result<Self> {
        let client = redis::Client::open(url)?;
        Ok(Self { client, agent_id })
    }
    
    pub async fn publish(&self, msg: InterAgentMessage) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let channel = format!("agent:{}", msg.to);
        let payload = serde_json::to_string(&msg)?;
        
        redis::cmd::PUBLISH")
            .arg(&channel)
            .arg(&payload)
            .query_async(&mut conn)
            .await?;
        
        Ok(())
    }
    
    pub async fn subscribe(&self) -> Result<tokio::sync::mpsc::Receiver<InterAgentMessage>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut pubsub = self.client.get_async_pubsub().await?;
        
        pubsub.subscribe(format!("agent:{}", self.agent_id)).await?;
        
        tokio::spawn(async move {
            while let Some(msg) = pubsub.on_message().next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    if let Ok(message) = serde_json::from_str::<InterAgentMessage>(&payload) {
                        let _ = tx.send(message).await;
                    }
                }
            }
        });
        
        Ok(rx)
    }
}
```

## 5. 安全层

### 5.1 API Key 认证

```rust
pub struct ApiKeyAuth {
    keys: HashMap<String, ApiKeyInfo>,
}

#[derive(Clone)]
struct ApiKeyInfo {
    agent_id: AgentId,
    permissions: Vec<Permission>,
    created_at: i64,
}

impl ApiKeyAuth {
    pub fn validate(&self, key: &str) -> Result<&ApiKeyInfo> {
        self.keys.get(key)
            .ok_or_else(|| anyhow::anyhow!("Invalid API key"))
    }
    
    pub fn generate(&mut self, agent_id: AgentId) -> String {
        let key = format!("sk_{}", uuid::Uuid::new_v4());
        self.keys.insert(key.clone(), ApiKeyInfo {
            agent_id,
            permissions: vec![Permission::All],
            created_at: now(),
        });
        key
    }
}
```

### 5.2 JWT 认证

```rust
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

pub struct JwtAuth {
    secret: String,
}

impl JwtAuth {
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
    
    pub fn generate(&self, agent_id: &AgentId) -> Result<String> {
        let claims = Claims {
            sub: agent_id.clone(),
            exp: (now() + 3600) as usize,
        };
        
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_ref()),
        )?;
        
        Ok(token)
    }
    
    pub fn validate(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_ref()),
            &Validation::default()
        )?;
        
        Ok(token_data.claims)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: AgentId,
    exp: usize,
}
```

### 5.3 RBAC 权限控制

```rust
pub struct RbacManager {
    roles: HashMap<String, Role>,
    user_roles: HashMap<AgentId, Vec<String>>,
}

#[derive(Clone)]
pub struct Role {
    name: String,
    permissions: Vec<Permission>,
}

#[derive(Clone, PartialEq)]
pub enum Permission {
    SendMessage,
    ReceiveMessage,
    Admin,
    All,
}

impl RbacManager {
    pub fn check_permission(&self, agent_id: &AgentId, perm: Permission) -> bool {
        if let Some(roles) = self.user_roles.get(agent_id) {
            for role_name in roles {
                if let Some(role) = self.roles.get(role_name) {
                    if role.permissions.contains(&Permission::All) ||
                       role.permissions.contains(&perm) {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    pub fn assign_role(&mut self, agent_id: AgentId, role_name: String) {
        self.user_roles.entry(agent_id).or_default().push(role_name);
    }
}
```

### 5.4 审计日志

```rust
pub struct AuditLogger {
    storage: AuditStorage,
}

pub enum AuditStorage {
    File(PathBuf),
    Database(DbConnection),
}

impl AuditLogger {
    pub async fn log(&self, entry: AuditEntry) {
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            agent_id: entry.agent_id,
            action: entry.action,
            resource: entry.resource,
            result: entry.result,
        };
        
        match &self.storage {
            AuditStorage::File(path) => {
                // 写入文件
            }
            AuditStorage::Database(db) => {
                // 写入数据库
            }
        }
    }
    
    pub async fn query(&self, filter: AuditFilter) -> Vec<AuditEntry> {
        // 查询日志
        vec![]
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: i64,
    pub agent_id: AgentId,
    pub action: String,
    pub resource: String,
    pub result: String,
}
```

### 5.5 速率限制

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    limits: HashMap<AgentId, RateLimit>,
}

pub struct RateLimit {
    count: u32,
    window_start: Instant,
    max_requests: u32,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn check(&mut self, agent_id: &AgentId) -> Result<()> {
        let now = Instant::now();
        let limit = self.limits.entry(agent_id.clone()).or_insert(RateLimit {
            count: 0,
            window_start: now,
            max_requests: 100,
            window_duration: Duration::from_secs(60),
        });
        
        // 重置窗口
        if now.duration_since(limit.window_start) >= limit.window_duration {
            limit.count = 0;
            limit.window_start = now;
        }
        
        // 检查限制
        if limit.count >= limit.max_requests {
            return Err(anyhow::anyhow!("Rate limit exceeded"));
        }
        
        limit.count += 1;
        Ok(())
    }
}
```

## 6. 可选的上下文隔离

```rust
pub enum IsolationLevel {
    None,              // 全局上下文（默认）
    User(AgentId),     // 用户级隔离
    Session(String),   // 会话级隔离
}

pub struct ContextManager {
    db: Connection,
    isolation: IsolationLevel,
}

impl ContextManager {
    pub fn with_isolation(config: ContextConfig, isolation: IsolationLevel) -> Result<Self> {
        // 根据隔离级别初始化
        Ok(Self {
            db: Connection::open(config.db_path)?,
            isolation,
        })
    }
    
    pub fn add_message(&mut self, message: &str, source: &str) -> Result<String> {
        // 根据 isolation 添加前缀或命名空间
        let namespace = match &self.isolation {
            IsolationLevel::None => "global".to_string(),
            IsolationLevel::User(id) => format!("user:{}", id),
            IsolationLevel::Session(id) => format!("session:{}", id),
        };
        
        // 存储时使用 namespace
        // ...
        
        Ok(uuid::Uuid::new_v4().to_string())
    }
}
```

## 7. 使用示例

### 7.1 启动 Agent（带通信）

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // 创建 Agent
    let agent = AgentEngine::new("my-agent".to_string(), "glm-4".to_string())?;
    
    // 启动 WebSocket 服务器
    let ws_server = WebSocketServer::new("127.0.0.1:8080".parse()?);
    tokio::spawn(async move {
        ws_server.start().await.unwrap();
    });
    
    // 启动 HTTP API
    let http_server = HttpApiServer::new("127.0.0.1:3000".parse()?);
    http_server.start();
    
    // 连接到其他 Agent
    let ws_client = WebSocketClient::connect(
        "ws://localhost:8081".to_string(),
        "other-agent".to_string()
    ).await?;
    
    // 发送消息
    let msg = InterAgentMessage {
        from: "my-agent".to_string(),
        to: "other-agent".to_string(),
        payload: MessagePayload::Request(Request::Query("hello".to_string())),
        ..Default::default()
    };
    
    ws_client.send(msg).await?;
    
    Ok(())
}
```

### 7.2 配置文件

```yaml
agent:
  name: "my-agent"
  model: "glm-4"

security:
  api_key: "sk_xxx"
  jwt_secret: "secret"
  rbac:
    enabled: true
    default_role: "user"

communication:
  websocket:
    enabled: true
    port: 8080
  http:
    enabled: true
    port: 3000
  redis:
    enabled: false
    url: "redis://localhost"

context:
  isolation: "user"  # none | user | session
  max_tokens: 8000
```

## 8. 开发计划

### Week 1-2: 安全层
- [ ] API Key 认证
- [ ] JWT 认证
- [ ] RBAC 权限

### Week 3-4: 通信接口
- [ ] WebSocket 服务器/客户端
- [ ] HTTP API
- [ ] 消息格式标准化

### Week 5-6: 审计与限制
- [ ] 审计日志
- [ ] 速率限制
- [ ] 可选隔离

---

**总开发时间**: 4-6 周
**新增代码**: ~1800 行
**维护成本**: 低
