# 多层路由架构设计（Multi-Layer Router Architecture）

> **状态 (2026-03-17 v0.7.1)**：本文档描述的是规划中的架构。部分功能已实现，部分仍在开发中。
> 
> **已实现**：Router 基础结构、Isolation、Sandbox、Policy（部分）
> 
> **规划中**：RouterManager、RouterConnector、完整 PolicyEngine、AuditLog

## 1. 核心概念

### 1.1 路由（Router）

路由是 NewClaw 的核心抽象，代表一个独立的智能体边界。

```rust
pub struct Router {
    id: RouterId,
    name: String,
    level: RouterLevel,      // 顶级 | 上级 | 下级
    parent: Option<RouterId>,
    children: Vec<RouterId>,
    capabilities: RouterCapabilities,
    policy: RouterPolicy,
}

pub enum RouterLevel {
    Top,                     // 顶级路由（独立）
    Upper,                   // 上级路由（有下级）
    Lower,                   // 下级路由（有上级）
    Special,                 // 特殊路由（Channel/Skill）
}

pub struct RouterCapabilities {
    can_manage_children: bool,    // 是否可以管理下级
    can_request_parent: bool,     // 是否可以请求上级
    can_share_with_peers: bool,   // 是否可以共享给同级
    can_spawn_children: bool,     // 是否可以派生下级
}
```

### 1.2 路由层级

```
Top-Level Router A                Top-Level Router B
    ├── Upper Router 1                ├── Upper Router 1
    │   ├── Lower Router 1.1          │   ├── Lower Router 1.1
    │   ├── Lower Router 1.2          │   └── Lower Router 1.2
    │   └── Channel A (Special)       └── Skill B (Special)
    └── Upper Router 2
        ├── Lower Router 2.1
        └── Skill A (Special)
```

**规则**:
1. 顶级路由可以有多个
2. 上级路由可以有多个下级
3. 下级路由只能有一个上级
4. 特殊路由（Channel/Skill）不能再派生

### 1.3 权限流

```
┌─────────────────────────────────────────┐
│           Top-Level Router              │
│     (can_manage: true, can_spawn: true) │
└──────────────┬──────────────────────────┘
               │ manage/share
       ┌───────┴────────┐
       ▼                ▼
┌──────────┐      ┌──────────┐
│  Upper   │      │  Upper   │
│ Router 1 │      │ Router 2 │
└─────┬────┘      └─────┬────┘
      │ manage          │ manage
  ┌───┴────┐       ┌───┴────┐
  ▼        ▼       ▼        ▼
Lower    Lower   Lower    Channel
1.1      1.2     2.1       A
  │        │       │
  └──request_parent───┘
```

**权限规则**:
1. 上级可以：管理、共享、监控下级
2. 下级可以：请求上级、共享同级（需授权）
3. 特殊路由可以：执行功能、请求父级
4. 跨级通信：需要上级路由转发

## 2. 核心组件

### 2.1 RouterManager

管理所有路由的生命周期和层级关系。

```rust
pub struct RouterManager {
    routers: HashMap<RouterId, Router>,
    topology: RouterTopology,
    policy_engine: PolicyEngine,
    audit_log: AuditLog,
}

impl RouterManager {
    // 路由生命周期
    pub fn spawn_router(&mut self, config: RouterConfig) -> Result<RouterId>;
    pub fn shutdown_router(&mut self, id: RouterId) -> Result<()>;
    
    // 层级管理
    pub fn add_child(&mut self, parent: RouterId, child: RouterId) -> Result<()>;
    pub fn remove_child(&mut self, parent: RouterId, child: RouterId) -> Result<()>;
    
    // 路由发现
    pub fn find_router(&self, name: &str) -> Option<&Router>;
    pub fn get_topology(&self) -> &RouterTopology;
    
    // 权限检查
    pub fn check_permission(&self, 
        from: RouterId, 
        to: RouterId, 
        action: Action
    ) -> Result<bool>;
}
```

### 2.2 RouterConnector

路由间的标准连接器。

```rust
pub struct RouterConnector {
    sender: mpsc::Sender<RouterMessage>,
    receiver: mpsc::Receiver<RouterMessage>,
}

pub struct RouterMessage {
    id: MessageId,
    from: RouterId,
    to: RouterId,
    action: Action,
    payload: MessagePayload,
    timestamp: i64,
}

pub enum Action {
    Request,      // 下级 → 上级
    Command,      // 上级 → 下级
    Share,        // 同级 ↔ 同级
    Notify,       // 广播
}

impl RouterConnector {
    pub async fn send(&mut self, msg: RouterMessage) -> Result<()>;
    pub async fn receive(&mut self) -> Result<RouterMessage>;
    pub fn connect(&mut self, other: &mut RouterConnector) -> Result<()>;
}
```

### 2.3 PolicyEngine

处理上级响应下级请求的策略。

```rust
pub struct PolicyEngine {
    policies: HashMap<RouterId, Vec<Policy>>,
}

pub enum Policy {
    AllowAll,                    // 允许所有请求
    DenyAll,                     // 拒绝所有请求
    Whitelist(Vec<Action>),      // 白名单
    Blacklist(Vec<Action>),      // 黑名单
    Conditional(Box<dyn Fn(&Request) -> bool>),  // 条件策略
    RateLimit(u32),              // 速率限制
    TimeWindow(TimeRange),       // 时间窗口
}

impl PolicyEngine {
    pub fn set_policy(&mut self, router: RouterId, policy: Policy);
    pub fn evaluate(&self, router: RouterId, request: &Request) -> PolicyDecision;
    pub fn combine_policies(&self, policies: &[Policy]) -> Policy;
}
```

### 2.4 AuditLog

审计日志系统。

```rust
pub struct AuditLog {
    entries: Vec<AuditEntry>,
    storage: AuditStorage,
}

pub struct AuditEntry {
    id: EntryId,
    timestamp: i64,
    from: RouterId,
    to: RouterId,
    action: Action,
    result: ActionResult,
    metadata: HashMap<String, String>,
}

pub enum AuditStorage {
    Memory(Vec<AuditEntry>),
    File(PathBuf),
    Database(DbConnection),
    Remote(RemoteLogger),
}

impl AuditLog {
    pub fn log(&mut self, entry: AuditEntry);
    pub fn query(&self, filter: AuditFilter) -> Vec<AuditEntry>;
    pub fn export(&self, format: ExportFormat) -> Result<Vec<u8>>;
}
```

## 3. 工作流程

### 3.1 下级请求上级

```
1. Lower Router 创建请求
   ↓
2. 检查权限（can_request_parent）
   ↓
3. RouterConnector 发送到上级
   ↓
4. Upper Router 接收请求
   ↓
5. PolicyEngine 评估策略
   ↓
6. 处理请求或拒绝
   ↓
7. 返回响应
   ↓
8. AuditLog 记录
```

**代码示例**:
```rust
// 下级路由请求
async fn request_parent(&self, request: Request) -> Result<Response> {
    // 1. 检查权限
    if !self.capabilities.can_request_parent {
        return Err(Error::PermissionDenied);
    }
    
    // 2. 获取上级路由
    let parent = self.parent.ok_or(Error::NoParent)?;
    
    // 3. 发送请求
    let msg = RouterMessage {
        from: self.id,
        to: parent,
        action: Action::Request,
        payload: request.into(),
        ..Default::default()
    };
    
    self.connector.send(msg).await?;
    
    // 4. 等待响应
    let response = self.connector.receive().await?;
    
    Ok(response.into())
}
```

### 3.2 上级管理下级

```
1. Upper Router 发送命令
   ↓
2. 检查权限（can_manage_children）
   ↓
3. RouterConnector 广播到下级
   ↓
4. Lower Router 接收命令
   ↓
5. 执行命令
   ↓
6. 返回结果
   ↓
7. AuditLog 记录
```

**代码示例**:
```rust
// 上级路由管理下级
async fn manage_child(&self, child_id: RouterId, command: Command) -> Result<Response> {
    // 1. 检查权限
    if !self.capabilities.can_manage_children {
        return Err(Error::PermissionDenied);
    }
    
    // 2. 验证父子关系
    if !self.children.contains(&child_id) {
        return Err(Error::NotAChild);
    }
    
    // 3. 发送命令
    let msg = RouterMessage {
        from: self.id,
        to: child_id,
        action: Action::Command,
        payload: command.into(),
        ..Default::default()
    };
    
    self.connector.send(msg).await?;
    
    // 4. 等待响应
    let response = self.connector.receive().await?;
    
    // 5. 记录审计
    self.audit_log.log(AuditEntry {
        from: self.id,
        to: child_id,
        action: Action::Command,
        result: response.clone().into(),
        ..Default::default()
    });
    
    Ok(response)
}
```

### 3.3 同级共享（需授权）

```
1. Router A 请求共享给 Router B
   ↓
2. 检查权限（can_share_with_peers）
   ↓
3. 上级路由授权
   ↓
4. RouterConnector 点对点连接
   ↓
5. 交换数据
   ↓
6. AuditLog 记录
```

## 4. 特殊路由

### 4.1 Channel Router

通道是特殊路由，负责外部通信。

```rust
pub struct ChannelRouter {
    router: Router,
    channel_type: ChannelType,
    connection: ChannelConnection,
}

pub enum ChannelType {
    Feishu,
    WeCom,
    Telegram,
    Slack,
    // ...
}

impl ChannelRouter {
    pub async fn send_message(&self, msg: Message) -> Result<()>;
    pub async fn receive_message(&mut self) -> Result<Message>;
    pub fn channel_type(&self) -> ChannelType;
}

// 特殊路由不能派生下级
impl ChannelRouter {
    pub fn can_spawn_children(&self) -> bool {
        false  // 特殊路由限制
    }
}
```

### 4.2 Skill Router

技能是特殊路由，负责特定功能。

```rust
pub struct SkillRouter {
    router: Router,
    skill_type: SkillType,
    manifest: SkillManifest,
}

pub enum SkillType {
    OpenClawSkill,      // 兼容 OpenClaw
    NewClawPlugin,      // NewClaw 原生插件
    TypeScriptPlugin,   // TypeScript 插件
}

impl SkillRouter {
    pub async fn execute(&self, input: SkillInput) -> Result<SkillOutput>;
    pub fn skill_type(&self) -> SkillType;
    pub fn manifest(&self) -> &SkillManifest;
}

// 特殊路由不能派生下级
impl SkillRouter {
    pub fn can_spawn_children(&self) -> bool {
        false  // 特殊路由限制
    }
}
```

## 5. 分布式拓扑

### 5.1 多顶级路由

```rust
pub struct DistributedTopology {
    top_level_routers: Vec<RouterId>,
    connections: HashMap<RouterId, Vec<RouterId>>,
}

impl DistributedTopology {
    pub fn add_top_level(&mut self, router: RouterId) {
        self.top_level_routers.push(router);
    }
    
    pub fn connect_routers(&mut self, a: RouterId, b: RouterId) {
        self.connections.entry(a).or_default().push(b);
        self.connections.entry(b).or_default().push(a);
    }
    
    pub fn find_path(&self, from: RouterId, to: RouterId) -> Option<Vec<RouterId>> {
        // BFS 寻找最短路径
        // ...
    }
}
```

### 5.2 路由发现

```rust
pub struct RouterDiscovery {
    registry: RouterRegistry,
    broadcaster: EventBroadcaster,
}

impl RouterDiscovery {
    pub async fn announce(&self, router: &Router) {
        self.broadcaster.broadcast(Event::RouterAnnounced {
            id: router.id,
            name: router.name.clone(),
            capabilities: router.capabilities.clone(),
        }).await;
    }
    
    pub async fn discover(&self, name: &str) -> Option<Router> {
        self.registry.find_by_name(name).await
    }
    
    pub async fn subscribe(&mut self, callback: Callback) {
        self.broadcaster.subscribe(callback).await;
    }
}
```

## 6. 配置示例

### 6.1 简单配置（单顶级路由）

```yaml
routers:
  - id: main
    name: "Main Agent"
    level: Top
    capabilities:
      can_manage_children: true
      can_request_parent: false
      can_share_with_peers: false
      can_spawn_children: true
    policy: AllowAll
    children:
      - id: feishu-channel
        type: Channel
        config:
          channel_type: Feishu
          app_id: "xxx"
          app_secret: "xxx"
      - id: search-skill
        type: Skill
        config:
          skill_type: OpenClawSkill
          path: "/root/.openclaw/workspace/skills/Search"
```

### 6.2 复杂配置（多顶级路由）

```yaml
routers:
  # 顶级路由 A（工作）
  - id: work-agent
    name: "Work Agent"
    level: Top
    policy:
      type: TimeWindow
      window: "09:00-18:00"
    children:
      - id: work-feishu
        type: Channel
      - id: calendar-skill
        type: Skill

  # 顶级路由 B（个人）
  - id: personal-agent
    name: "Personal Agent"
    level: Top
    policy:
      type: TimeWindow
      window: "18:00-09:00"
    children:
      - id: personal-feishu
        type: Channel
      - id: health-skill
        type: Skill

connections:
  - from: work-agent
    to: personal-agent
    type: Peering
    policy: Whitelist(["share_calendar"])
```

## 7. 迁移策略

### 7.1 从当前架构迁移

```
当前（v0.1.0）:
- 单一 AgentEngine
- 全局 ContextManager
- 无隔离

目标（v0.3.0）:
- 多 Router 实例
- 路由级 ContextManager
- 分层隔离
```

**迁移步骤**:

1. **Phase 1: 抽象 Router**
   ```rust
   // 当前
   let agent = AgentEngine::new(...);
   
   // 目标
   let router = Router::new_top_level(...);
   let agent = router.agent();
   ```

2. **Phase 2: 添加层级**
   ```rust
   // 创建下级路由
   let child = router.spawn_child(...)?;
   ```

3. **Phase 3: 添加权限**
   ```rust
   // 配置权限
   router.set_policy(Policy::AllowAll);
   child.set_policy(Policy::Whitelist(...));
   ```

### 7.2 从 OpenClaw 迁移

```yaml
# OpenClaw 配置
channels:
  - feishu
  - wecom

skills:
  - search
  - browser

# NewClaw 配置（等效）
routers:
  - id: main
    level: Top
    children:
      - id: feishu
        type: Channel
      - id: wecom
        type: Channel
      - id: search
        type: Skill
      - id: browser
        type: Skill
```

## 8. 优势总结

### vs OpenClaw
- ✅ 生产就绪（权限、审计）
- ✅ 灵活隔离（可配置）
- ✅ 分布式（多顶级）
- ⚠️ 复杂度增加

### vs ZeroClaw
- ✅ 保留智能聚合
- ✅ 可配置权衡
- ✅ 渐进式迁移
- ⚠️ 学习曲线

### 独特价值
- **灵活性**: 从弱隔离到强隔离
- **兼容性**: 兼容 OpenClaw 概念
- **扩展性**: 分布式拓扑
- **安全性**: 生产级审计

## 9. 实现优先级

### P0（核心）
- [ ] Router 抽象
- [ ] RouterManager
- [ ] RouterConnector
- [ ] 权限检查

### P1（重要）
- [ ] PolicyEngine
- [ ] AuditLog
- [ ] ChannelRouter
- [ ] SkillRouter

### P2（增强）
- [ ] 分布式拓扑
- [ ] 路由发现
- [ ] Web Dashboard
- [ ] CLI 工具

---

**总结**: 这个多层路由架构是一个**有价值的创新**，平衡了安全性与智能聚合，技术上可行，架构清晰。建议作为 v0.3.0 的核心特性。
