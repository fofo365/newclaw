# NewClaw v0.6.0 - 声明式联邦架构设计

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
            ChannelError::InvalidInput("AGP Channel requires explicit target".to_string())
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
        if let Some(coordinator) = &self.coordinator {
            coordinator.unregister(&self.config.agent_id).await?;
        }
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
        format!("agp://localhost:7777/{}", self.config.agent_id)
    }
}
```

### 2.2 配置结构

```rust
// src/channels/agp/config.rs

use serde::{Deserialize, Serialize};

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

## 三、协调平面（外部服务）

> **注意**：协调平面是独立项目，不在 NewClaw 代码库中开发。
> NewClaw 仅提供 Coordinator trait 接口，具体实现由外部服务提供。

### 3.1 接口定义

```rust
/// 协调平面接口（对接外部服务）
#[async_trait]
pub trait Coordinator: Send + Sync {
    /// 注册 Agent
    async fn register(&self, agent_id: &str, capabilities: &[String], endpoint: &str) -> Result<Registration>;
    
    /// 注销 Agent
    async fn unregister(&self, agent_id: &str) -> Result<()>;
    
    /// 发现 Agent
    async fn discover(&self, capability: &str) -> Result<Vec<AgentInfo>>;
}
```

### 3.2 默认实现

NewClaw 内置最小化实现，用于开发和测试：

```rust
/// 内存协调器（仅用于开发/测试）
pub struct InMemoryCoordinator {
    peers: Arc<RwLock<HashMap<String, AgentInfo>>>,
}
```

生产环境应替换为外部协调服务（独立项目）。

---

## 四、与热拔插方案的对比

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

## 五、实施路线图

### Phase 4.1: AGP Channel 扩展（1-2 周）
- [ ] 实现 `AGPChannel` 类，符合 Channel 接口
- [ ] 点对点直连通信（无协调器）
- [ ] 10 个单元测试

### Phase 4.2: 联邦感知工具（1 周）
- [ ] `FederationTool`（主动调用远程 Agent）
- [ ] `newclaw join` CLI 工具
- [ ] 能力发现和路由
- [ ] 5 个单元测试

### Phase 4.3: 原生集成（1 周）
- [ ] AGP Channel 提升为核心组件
- [ ] 联邦安全和认证机制
- [ ] 5 个单元测试

**注意**：协调平面仅保留概念和接口，不开发实际服务。

---

## 六、总结

**回归本质，放弃热拔插，拥抱声明式联邦：**

> NewClaw 不是一个需要运行时伪装的复杂适配层，而是一个为 OpenClaw 提供 AGP Channel 扩展的轻量级网络层。

Agent 通过简单的配置声明（`channels: [agp]`）和一次重启，即可从单机智能体转变为联邦网络的正式成员。

**这是最现实的方案**：
- ✅ 尊重现有架构
- ✅ 最小化惊喜
- ✅ 最大化可维护性
- ✅ 配置驱动哲学
- ✅ 生产级可靠性

---

**状态**: 📝 设计完成，准备实施
**版本**: v0.6.0
**最后更新**: 2026-03-11
