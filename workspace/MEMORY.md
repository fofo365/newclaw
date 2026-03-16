# MEMORY.md - 长期记忆

_这是 NewClaw 的长期记忆存储，保存重要决策、项目信息和用户偏好。_

---

## 项目信息

### NewClaw v0.7.0

**项目路径**: `/root/newclaw/`  
**主分支**: `main`  
**远程仓库**: `https://github.com/fofo365/newclaw.git`  
**Dashboard UI**: `/root/newclaw/dashboard-ui/`

**架构**:
- 后端: Rust (Cargo)
- 前端: React + Vite + TypeScript + Ant Design
- 记忆系统: SQLite + FTS5 + 多层隔离
- 策略系统: StrategyEngine
- 通道: CLI / Dashboard / Feishu

---

## 已完成功能

### v0.7.0 (2026-03-16)

1. **ChannelProcessor** - 统一消息处理器
   - 文件: `src/channel/processor.rs`
   - 集成: 记忆、策略、权限、工具、LLM

2. **多层隔离机制 (MemoryScope)**
   - 文件: `src/memory/storage_impl.rs`
   - 隔离维度: 用户/通道/Agent/命名空间

3. **Token 自动刷新**
   - 文件: `src/feishu_websocket/token_manager.rs`
   - 支持: TenantAccessToken / UserAccessToken

4. **三通道集成**
   - Feishu: `src/bin/feishu-connect.rs`
   - CLI: `src/cli/mod.rs`
   - Dashboard: `src/dashboard/mod.rs`

### v0.5.0 - v0.6.0

- 飞书 WebSocket 集成
- LLM 对话功能
- Dashboard 登录版本显示
- JWT 认证修复

---

## 关键文件

| 文件 | 用途 |
|------|------|
| `src/channel/processor.rs` | 统一消息处理器 |
| `src/memory/storage_impl.rs` | SQLite 存储（多层隔离） |
| `src/feishu_websocket/token_manager.rs` | Token 自动刷新 |
| `src/bin/feishu-connect.rs` | 飞书连接服务 |
| `src/cli/mod.rs` | CLI 交互 |
| `src/dashboard/mod.rs` | Dashboard 状态 |

---

## Git 提交记录

| Commit | 日期 | 说明 |
|--------|------|------|
| `6867192e` | 2026-03-16 | ChannelProcessor + 多层隔离 |
| `2faea177` | 2026-03-16 | CLI/Dashboard集成 + Token刷新 |
| `0b40dbaf` | 2026-03-16 | 开发工作总结文档 |

---

## 技术决策

### 1. 多层隔离设计

**原因**: 不同用户、通道、Agent 的记忆需要完全隔离

**实现**: `MemoryScope` 四层隔离
```rust
MemoryScope {
    user_id: "ou_xxx",      // 用户隔离
    channel: "feishu",      // 通道隔离
    agent_id: "feishu-bot", // Agent 隔离
    namespace: "default",   // 命名空间隔离
}
```

### 2. Token 自动刷新

**原因**: 飞书 Token 有效期 2 小时，需要自动续期

**实现**: UAT 机制
- 定时检查（每 60 秒）
- 提前刷新（过期前 5 分钟）
- 失败重试（最多 5 次）

### 3. ChannelProcessor 统一处理

**原因**: 三通道（CLI/Dashboard/Feishu）需要统一的消息处理流程

**实现**: 统一处理器
```
消息 → 权限 → 隔离 → 记忆 → 策略 → LLM → 存储
```

---

## 环境配置

### 系统信息
- OS: Ubuntu 22.04 (Linux 6.8.0-71-generic)
- Node: v22.22.1
- Rust: cargo 1.94.0
- 模型: qwencode/glm-5

### 服务端口
- Dashboard: http://localhost:3000
- Feishu WebSocket: wss://open.feishu.cn/open-apis/ws/v2

### 数据库
- CLI 记忆: `data/cli_memory.db`
- Feishu 记忆: `data/feishu_memory.db`
- Dashboard 记忆: `data/dashboard_memory.db`

---

## 用户偏好

- **姓名**: 王老吉 / 老吉
- **时区**: Asia/Shanghai (GMT+8)
- **习惯**:
  - 收到消息后先回复"已收到，等待我的处理。"
  - 喜欢快速迭代
  - 使用 git main 分支开发

---

_最后更新: 2026-03-16_