# TOOLS.md - 工具配置

_NewClaw 使用的工具和 API 配置_

---

## LLM Provider

### GLM (智谱AI)

- **Provider Type**: glm / glm-cn / glm-global
- **API Endpoint**: https://open.bigmodel.cn/api/paas/v4
- **Models**: glm-4, glm-4-flash, glm-4-plus
- **Auth**: GLM_API_KEY (format: id.secret)

### Claude (Anthropic)

- **Provider Type**: claude
- **API Endpoint**: https://api.anthropic.com
- **Models**: claude-3-opus, claude-3-sonnet, claude-3-haiku
- **Auth**: ANTHROPIC_API_KEY

### OpenAI

- **Provider Type**: openai
- **API Endpoint**: https://api.openai.com/v1
- **Models**: gpt-4, gpt-4-turbo, gpt-3.5-turbo
- **Auth**: OPENAI_API_KEY

---

## 飞书集成

### 应用信息

- **App ID**: cli_a928559df7b8dbcc
- **App Type**: 企业自建应用
- **WebSocket**: wss://open.feishu.cn/open-apis/ws/v2

### API Endpoints

| 功能 | Endpoint |
|------|----------|
| 获取 Token | POST https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal |
| 发送消息 | POST https://open.feishu.cn/open-apis/im/v1/messages |
| 获取 WebSocket URL | POST https://open.feishu.cn/callback/ws/endpoint |

### 权限要求

- `im:message` - 发送消息
- `im:message:receive_as_bot` - 接收消息
- `docx:document` - 创建/编辑文档 (未开通)
- `drive:drive` - 访问云空间 (未开通)

---

## 数据库

### SQLite

- **位置**: `/root/newclaw/data/`
- **文件**:
  - `cli_memory.db` - CLI 通道记忆
  - `feishu_memory.db` - 飞书通道记忆
  - `dashboard_memory.db` - Dashboard 通道记忆
- **特性**: FTS5 全文索引

---

## 服务端口

| 服务 | 端口 | 说明 |
|------|------|------|
| Dashboard | 3000 | Web UI |
| Gateway | 3001 | API Gateway |

---

## Systemd 服务

### newclaw-feishu

```bash
# 启动
systemctl start newclaw-feishu

# 停止
systemctl stop newclaw-feishu

# 状态
systemctl status newclaw-feishu

# 日志
journalctl -u newclaw-feishu -f
```

### 服务文件位置

```
/etc/systemd/system/newclaw-feishu.service
```

---

## 环境变量

### 必需

- `GLM_API_KEY` - GLM API 密钥

### 可选

- `NEWCLAW_CONFIG` - 配置文件路径 (默认: `/etc/newclaw/config.toml`)
- `RUST_LOG` - 日志级别 (默认: `info`)
- `FEISHU_APP_ID` - 飞书应用 ID
- `FEISHU_APP_SECRET` - 飞书应用密钥

---

## CLI 命令

### newclaw

```bash
# 启动交互模式
newclaw

# 启动 Gateway 模式
newclaw --gateway --port 3001

# 指定 Provider
newclaw --provider glm --model glm-4

# 生成配置文件
newclaw --generate-config > config.toml

# 列出支持的 Provider
newclaw --list-providers
```

### feishu-connect

```bash
# 启动飞书连接服务
cargo run --bin feishu-connect
```

---

## 开发工具

### Cargo

```bash
# 构建
cargo build --release

# 测试
cargo test

# 检查
cargo check

# 文档
cargo doc --open
```

### Dashboard UI

```bash
cd dashboard-ui

# 开发
pnpm dev

# 构建
pnpm build

# 预览
pnpm preview
```

---

_这个文件记录了 NewClaw 使用的工具和配置。_

_最后更新: 2026-03-16_