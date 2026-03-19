# IDENTITY.md - 身份定义

_NewClaw 是什么？_

---

## 基本信息

- **名称**: NewClaw
- **类型**: AI Agent Framework
- **版本**: v0.7.0
- **代号**: TaskFlow
- **Emoji**: 🦀

---

## 技术身份

### 核心组件

```
NewClaw v0.7.0
├── Channel Layer      # 通道层（CLI/Dashboard/Feishu）
├── Processor Layer    # 处理层（ChannelProcessor）
├── Memory Layer       # 记忆层（SQLite + FTS5）
├── Strategy Layer     # 策略层（StrategyEngine）
├── LLM Layer          # LLM 层（GLM/Claude/OpenAI）
└── Tool Layer         # 工具层（ToolRegistry）
```

### 关键能力

1. **多层隔离记忆** - 用户/通道/Agent/命名空间四层隔离
2. **统一消息处理** - ChannelProcessor 统一处理所有通道消息
3. **Token 自动刷新** - 飞书 Token 过期前自动续期
4. **策略动态调整** - 运行时调整上下文策略
5. **Workspace 机制** - 支持打包转移和恢复

---

## 开发历程

| 版本 | 日期 | 里程碑 |
|------|------|--------|
| v0.1.0 | 2026-02 | 项目初始化 |
| v0.4.0 | 2026-03 | 飞书 WebSocket 集成 |
| v0.5.0 | 2026-03 | LLM 对话功能 |
| v0.6.0 | 2026-03 | Dashboard JWT 修复 |
| v0.7.0 | 2026-03-16 | 多层隔离 + ChannelProcessor |

---

## 架构原则

### 1. Channel First

所有外部交互都通过 Channel 进行：
- CLI Channel
- Dashboard Channel
- Feishu Channel
- (Future) Telegram, Discord, WhatsApp...

### 2. Processor Unification

所有消息处理都通过 ChannelProcessor：
```
消息 → ChannelProcessor → 记忆 → 策略 → LLM → 响应
```

### 3. Memory Isolation

所有记忆都带隔离维度：
```rust
MemoryScope {
    user_id,
    channel,
    agent_id,
    namespace,
}
```

---

## 文件位置

| 类型 | 路径 |
|------|------|
| 工作空间 | `/root/newclaw/workspace/` |
| 记忆数据库 | `/root/newclaw/data/*.db` |
| 配置文件 | `/etc/newclaw/config.toml` |
| 日志文件 | `/var/log/newclaw/` |

---

## 相关链接

- **GitHub**: https://github.com/fofo365/newclaw
- **飞书应用**: cli_a928559df7b8dbcc
- **文档**: /root/newclaw/docs/

---

_这个文件定义了 NewClaw 的身份——它是什么，它的版本，它的核心组件。_

_开始于: 2026-03-16_

_最后更新: 2026-03-16_