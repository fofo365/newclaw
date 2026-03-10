# NewClaw v0.5.0 - 声明式联邦架构设计

## 核心理念

**"联邦成员资格是一种重身份，需要通过声明式配置来表达，而非运行时热拔插。"**

基于 OpenClaw 的配置驱动哲学，NewClaw 将 AGP（Agent Gateway Protocol）网络提升为一等公民输入源（Channel），而非后天打补丁的 Skill。

---

## 一、架构简化：回归 Channel 本质

### 1.1 核心设计

将 AGP 网络视为与 CLI、HTTP 并列的一等公民 Channel，通过声明式配置加入联邦：

```yaml
# newclaw_agent.yaml（标准 NewClaw 配置）
agent:
  id: "math-solver-1"
  name: "数学专家"
  version: "0.5.0"

# 输入源配置（Channels）
channels:
  - type: cli              # 本地终端
    enabled: true

  - type: http             # REST API
    enabled: true
    config:
      host: "0.0.0.0"
      port: 3000

  - type: agp              # 🆕 联邦网络通道
    enabled: true
    config:
      bootstrap: "agp://registry.local:8000"  # 轻量注册点
      advertise:                                   # 声明能力
        - "math-solver"
        - "latex-parser"
      domain: "academic-mesh"                     # 联邦域标识
      endpoint: "agp://auto"                      # 自动检测本地 endpoint

# 工具配置
tools:
  enabled:
    - read
    - write
    - exec
    - web_search

# LLM 配置
llm:
  provider: "glm"
  model: "glm-4"
  api_key_env: "GLM_API_KEY"
```

### 1.2 关键洞察

**一旦 Agent 决定加入联邦，它就在网络拓扑中占据固定位置**：
- 有身份（agent-id）
- 有 endpoint（agp://ip:port/agent-id）
- 有能力声明（advertise capabilities）
- 有联邦域归属（domain membership）

**热拔插的弊端**：
- 破坏成员资格的契约严肃性
- 增加运行时复杂度（线程安全、内存泄漏风险）
- 难以调试和监控

**声明式的优势**：
- 配置即文档（configuration as documentation）
- 重启一次即可加入联邦（简单、可靠、可预测）
- 符合 OpenClaw 的配置驱动哲学

---

## 二、AGP Channel 实现（NewClaw 扩展）

### 2.1 Channel 接口实现

```rust
// src/channels/agp/mod.rs

use async_trait::async_trait;
use crate::channels::{Channel, Message, MessageHandler};
use crate::channels::agp::{
    AGPConfig, AGPSession, CoordinatorClient,
    AGPMessage, FederationDomain
};

/// AGP Channel - 符合 NewClaw Channel 契约的联邦网络适配器
///
/// 生命周期与 Agent 绑定：
/// - Agent 启动时：连接协调平面 → 注册身份 → 启动监听
/// - Agent 运行时：接收联邦消息 → 转换为 NewClaw Message → 触发 Agent 主循环
/// - Agent 关闭时：注销身份 → 关闭连接 → 清理资源
pub struct AGPChannel {
    config: AGPConfig,
    session: Option<AGPSession>,
    coordinator: Option<CoordinatorClient>,
    message_handler: Option<Arc<dyn MessageHandler>>,
}

#[async_trait]
impl Channel for AGPChannel {
    /// Channel 类型标识
    fn channel_type(&self) -> &str {
        "agp"
    }

    /// 启动 Channel（Agent 初始化时调用）
    async fn start(&mut self, handler: Arc<dyn MessageHandler>) -> Result<(), ChannelError> {
        self.message_handler = Some(handler.clone());

        // 1. 连接轻量协调平面（获取网络身份）
        self.coordinator = Some(CoordinatorClient::connect(&self.config.bootstrap).await?);

        let assignment = self.coordinator.as_ref().unwrap()
            .register(
                &self.config.agent_id,
                &self.config.advertise,
                self.config.endpoint.clone()
                    .unwrap_or_else(|| self.detect_local_endpoint())
            )
            .await?;

        tracing::info!(
            "AGP Channel: Registered as '{}' with {} initial peers",
            assignment.identity,
            assignment.initial_peers.len()
        );

        // 2. 启动 AGP 监听（长期运行）
        self.session = Some(AGPSession::new(
            assignment.identity,
            assignment.initial_peers,
            self.config.domain.clone(),
        ).await?);

        // 3. 启动消息接收循环
        let session = self.session.as_ref().unwrap().clone();
        let handler_clone = handler.clone();
        tokio::spawn(async move {
            while let Some(agp_msg) = session.receive().await {
                // 将 AGP 消息转换为 NewClaw Message
                let message = Message {
                    content: agp_msg.payload,
                    role: MessageRole::User,
                    channel: "agp".to_string(),
                    metadata: serde_json::json!({
                        "remote_id": agp_msg.sender,
                        "reply_addr": agp_msg.reply_addr,
                        "federation_domain": agp_msg.domain,
                        "correlation_id": agp_msg.correlation_id,
                    }),
                    timestamp: chrono::Utc::now(),
                };

                // 触发 Agent 主循环
                if let Err(e) = handler_clone.handle_message(message).await {
                    tracing::error!("AGP message handler error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// 发送消息到联邦网络
    async fn send(&self, message: Message, target: Option<String>) -> Result<(), ChannelError> {
        let session = self.session.as_ref()
            .ok_or_else(|| ChannelError::NotConnected)?;

        let target_id = target.ok_or_else(||
            ChannelError::InvalidInput("AGP Channel requires explicit target (remote Agent ID)".to_string())
        )?;

        session.send(AGPMessage {
            sender: self.config.agent_id.clone(),
            recipient: target_id,
            payload: message.content,
            reply_addr: None,
            correlation_id: message.metadata.get("correlation_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            domain: self.config.domain.clone(),
        }).await?;

        Ok(())
    }

    /// 关闭 Channel（Agent 关闭时调用）
    async fn close(&mut self) -> Result<(), ChannelError> {
        // 1. 注销身份
        if let Some(coordinator) = &self.coordinator {
            coordinator.unregister(&self.config.agent_id).await?;
        }

        // 2. 关闭会话
        if let Some(session) = &self.session {
            session.leave().await?;
        }

        tracing::info!("AGP Channel closed");
        Ok(())
    }

    /// 健康检查
    async fn health_check(&self) -> ChannelHealth {
        match &self.session {
            Some(session) if session.is_connected() => ChannelHealth::Healthy,
            _ => ChannelHealth::Unhealthy,
        }
    }
}

impl AGPChannel {
    /// 自动检测本地 endpoint
    fn detect_local_endpoint(&self) -> String {
        // TODO: 实现自动检测逻辑
        // 1. 检查环境变量 NEWCLAW_AGP_ENDPOINT
        // 2. 检查配置文件
        // 3. 使用默认值 agp://localhost:7777/agent-id
        format!("agp://localhost:7777/{}", self.config.agent_id)
    }
}
```

### 2.2 配置结构

```rust
// src/channels/agp/config.rs

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// AGP Channel 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGPConfig {
    /// Agent ID（联邦网络中的唯一标识）
    pub agent_id: String,

    /// 协调平面 bootstrap 地址
    pub bootstrap: String,

    /// 能力声明（向联邦网络宣告的能力）
    pub advertise: Vec<String>,

    /// 联邦域标识
    pub domain: Option<String>,

    /// 本地 endpoint（可选，自动检测）
    pub endpoint: Option<String>,

    /// 连接超时（秒）
    pub timeout_secs: Option<u64>,

    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: Option<u64>,
}
```

---

## 三、轻量级协调平面（Coordination Plane）

### 3.1 设计原则

**避免重量级**：不要 Kubernetes，不要 Raft，只要轻量注册与发现。

### 3.2 选项 A：嵌入式协调（最小化）

对于单域小规模联邦（<100 Agents），协调平面直接嵌入 AGP Channel：

```rust
// src/channels/agp/coordinator/embedded.rs

/// 基于 gossip 的嵌入式协调
///
/// 特点：
/// - 无独立进程
/// - 零外部依赖
/// - 适合边缘部署
pub struct EmbeddedCoordinator {
    peers: Arc<RwLock<HashMap<String, AgentInfo>>>,
    gossip: GossipProtocol,
}

impl EmbeddedCoordinator {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            gossip: GossipProtocol::new(),
        }
    }

    /// 注册 Agent
    pub async fn register(
        &self,
        agent_id: String,
        capabilities: Vec<String>,
        endpoint: String,
    ) -> Result<Registration, CoordinatorError> {
        let info = AgentInfo {
            id: agent_id.clone(),
            capabilities,
            endpoint,
            registered_at: chrono::Utc::now(),
        };

        // 通过 gossip 广播存在
        self.gossip.broadcast(GossipMessage::Join(info.clone())).await;

        // 返回当前已知的部分节点（用于建立连接）
        let peers = self.peers.read().await;
        let initial_peers = peers.keys()
            .filter(|id| *id != &agent_id)
            .take(3) // 随机选择 3 个邻居
            .cloned()
            .collect();

        Ok(Registration {
            identity: agent_id,
            initial_peers,
        })
    }

    /// 发现 Agent
    pub async fn discover(&self, capability: &str) -> Vec<AgentInfo> {
        let peers = self.peers.read().await;
        peers.values()
            .filter(|info| info.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }
}
```

### 3.3 选项 B：独立协调服务（轻量级）

对于多域或需要持久化的场景，单独的协调进程：

```rust
// src/coordinator/server.rs

/// 独立协调服务
///
/// 特点：
/// - 单文件二进制（<10MB）
/// - 支持 SQLite/Redis 后端
/// - Docker 一键部署
/// - 仅负责身份分配、能力目录、健康检查
/// - 不负责消息路由（Agent 直连）
pub struct CoordinatorServer {
    backend: CoordinatorBackend,
    config: CoordinatorConfig,
}

impl CoordinatorServer {
    pub async fn run(config: CoordinatorConfig) -> anyhow::Result<()> {
        let backend = match config.backend.as_str() {
            "sqlite" => CoordinatorBackend::Sqlite(SqliteBackend::new(&config.db_path).await?),
            "redis" => CoordinatorBackend::Redis(RedisBackend::new(&config.redis_url).await?),
            _ => return Err(anyhow::anyhow!("Unknown backend: {}", config.backend)),
        };

        let server = Self { backend, config };

        // 启动 HTTP API
        let app = server.create_router();
        let addr = format!("{}:{}", config.host, config.port);
        tracing::info!("Coordinator listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    fn create_router(&self) -> Router {
        Router::new()
            // 注册（Join）
            .route("/v1/agents", post(Self::register_agent))
            // 发现（Discover）
            .route("/v1/agents", get(Self::discover_agents))
            // 健康检查
            .route("/health", get(Self::health_check))
    }

    /// 注册 Agent
    async fn register_agent(
        State(server): State<Arc<CoordinatorServer>>,
        Json(req): Json<RegisterRequest>,
    ) -> Result<Json<RegistrationResponse>, ErrorResponse> {
        let registration = server.backend.register(
            req.id,
            req.capabilities,
            req.endpoint,
            req.domain,
        ).await.map_err(|e| ErrorResponse {
            error: e.to_string(),
        })?;

        Ok(Json(registration))
    }

    /// 发现 Agent
    async fn discover_agents(
        State(server): State<Arc<CoordinatorServer>>,
        Query(params): Query<DiscoverQuery>,
    ) -> Result<Json<Vec<AgentInfo>>, ErrorResponse> {
        let agents = server.backend.discover(
            params.capability,
            params.domain,
        ).await.map_err(|e| ErrorResponse {
            error: e.to_string(),
        })?;

        Ok(Json(agents))
    }
}
```

---

## 四、与之前方案的对比

| 维度 | Daemon Skill（热拔插） | AGP Channel（声明式） |
|------|------------------------|----------------------|
| **复杂性** | 高（运行时编织、线程安全） | 低（遵循 Channel 契约） |
| **稳定性** | 有隐患（内存泄漏、竞态） | 高（生命周期与 Agent 绑定） |
| **启动成本** | 零（随时挂载） | 低（重启/重载配置） |
| **灵活性** | 高（动态进出联邦） | 中（需重启变更身份） |
| **架构一致性** | 破坏 Skill/Channel 边界 | 完美契合 NewClaw 设计 |
| **联邦语义** | 模糊（临时参与） | 清晰（正式成员） |
| **运维友好** | 难调试（后台任务） | 易监控（标准 Channel 日志） |

**关键洞察**：
> 联邦成员资格（Membership）本身就是一种重身份（有持久化 endpoint、有声誉、有历史），重启一次来加入是合理且健康的仪式感。

---

## 五、NewClaw 与 OpenClaw 的统一路径

### 5.1 OpenClaw（现有用户）

```bash
# 只需安装扩展并修改配置
pip install newclaw-agp-channel

# config.yaml 添加：
channels:
  - type: agp
    config:
      bootstrap: "agp://bootstrap.example.com"

# 重启 Agent
# → 现在它是一个联邦节点
```

### 5.2 NewClaw（下一代）

NewClaw 就是默认启用 AGP Channel 的 OpenClaw，加上增强的联邦感知：

```rust
// src/agent/federated.rs

/// 联邦感知的 Agent
///
/// 继承 OpenClaw 所有能力，但联邦是一等公民
pub struct FederatedAgent {
    inner: OpenClawAgent,
    agp_channel: Option<Arc<AGPChannel>>,
}

impl FederatedAgent {
    pub async fn new(config: AgentConfig) -> anyhow::Result<Self> {
        // 1. 创建底层 OpenClaw Agent
        let inner = OpenClawAgent::new(config.clone()).await?;

        // 2. 默认加载 AGP Channel（无需显式配置）
        let agp_channel = if config.federation.enabled {
            Some(Arc::new(AGPChannel::from_config(config.federation.agp)?))
        } else {
            None
        };

        Ok(Self { inner, agp_channel })
    }

    /// 发现联邦中的对等节点
    pub async fn discover_peers(&self, capability: &str) -> Vec<PeerInfo> {
        if let Some(agp) = &this.agp_channel {
            agp.discover(capability).await
        } else {
            vec![]
        }
    }

    /// 调用远程 Agent 的能力
    pub async fn call_remote(
        &self,
        target: &str,
        capability: &str,
        input: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        if let Some(agp) = &this.agp_channel {
            agp.call(target, capability, input).await
        } else {
            Err(anyhow::anyhow!("AGP Channel not enabled"))
        }
    }
}
```

---

## 六、实施路线图

### Phase 1：AGP Channel 扩展（1-2 周）
- [ ] 实现 `AGPChannel` 类，符合 NewClaw Channel 接口
- [ ] 提供 `cargo install newclaw-agp` 扩展包
- [ ] 支持嵌入式 gossip 协调（零配置模式）

### Phase 2：轻量协调服务（1 周）
- [ ] 独立 `newclaw-coordinator` 二进制（<10MB，单文件）
- [ ] 支持 SQLite/Redis 后端
- [ ] 提供 Docker 一键部署

### Phase 3：联邦感知工具（2 周）
- [ ] 基于 AGP Channel 的 `FederationSkill`（用于主动调用远程）
- [ ] 提供 `newclaw join` CLI 工具（自动生成配置 + 重启）
- [ ] 实现联邦能力发现和路由

### Phase 4：NewClaw 原生（长期）
- [ ] 将 AGP Channel 提升为核心组件（默认启用）
- [ ] 实现跨域路由（基于 AGP 的分层路由扩展）
- [ ] 添加联邦安全和认证机制

---

## 七、总结

**回归本质，放弃热拔插，拥抱声明式联邦：**

> NewClaw 不是一个需要运行时伪装的复杂适配层，而是一个为 OpenClaw 提供 AGP Channel 扩展的轻量级网络层。

Agent 通过简单的配置声明（`channels: [agp]`）和一次重启，即可从单机智能体转变为联邦网络的正式成员。

轻量级协调平面仅解决"发现"与"身份"问题，真正的通信通过 AGP Channel 直连，保持 OpenClaw 的简洁与自治。

**这是最现实的方案**：
- ✅ 尊重现有架构
- ✅ 最小化惊喜
- ✅ 最大化可维护性
- ✅ 配置驱动哲学
- ✅ 生产级可靠性
