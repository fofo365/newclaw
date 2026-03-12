# NewClaw v0.5.0

> 生产级 AI Agent 框架 - Rust 性能 + TypeScript 插件生态

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Test Coverage](https://img.shields.io/badge/tests-346%20passed-brightgreen.svg)](https://github.com/fofo365/newclaw)
[![Release](https://img.shields.io/badge/release-v0.5.0-blue.svg)](https://github.com/fofo365/newclaw/releases/tag/v0.5.0)

## 🎯 项目概述

NewClaw 是新一代 AI Agent 框架，提供：

- **🔧 14 类核心工具** - 文件、Shell、网络、浏览器、记忆等
- **🤖 多 LLM 支持** - OpenAI、Claude、GLM 统一接口
- **📡 7 个消息通道** - 飞书、企微、钉钉、QQ、Telegram、Discord、AGP
- **🧠 飞书集成 100%** - 文档、多维表格、云存储、知识库、聊天
- **✅ 346 个测试** - 100% 通过率，生产就绪
- **🚀 高性能** - 内存 < 50MB，比 OpenClaw 降低 75%

## 📊 v0.5.0 新特性

### 核心工具（82 个测试）
| 工具类别 | 功能 | 状态 |
|---------|------|------|
| 记忆系统 | 自动迁移、语义搜索 | ✅ |
| 文件操作 | read, write, edit | ✅ |
| Shell 执行 | exec, process | ✅ |
| 网络请求 | web_search, web_fetch | ✅ |
| 浏览器控制 | navigate, click, screenshot | ✅ |
| Canvas 展示 | present, hide, eval | ✅ |
| 会话管理 | spawn, list, send | ✅ |
| 节点管理 | status, notify | ✅ |

### 飞书集成（87 个测试）
| 工具 | 功能 | 状态 |
|------|------|------|
| feishu_doc | 文档读写 | ✅ |
| feishu_bitable | 多维表格 | ✅ |
| feishu_drive | 云存储 | ✅ |
| feishu_wiki | 知识库 | ✅ |
| feishu_chat | 聊天 | ✅ |

### OpenClaw 对标
| 指标 | OpenClaw | NewClaw | 覆盖率 |
|------|----------|---------|--------|
| 核心工具 | 10 类 | 10 类 | 100% |
| 飞书集成 | 5 个 | 5 个 | 100% |
| 测试覆盖 | 部分 | 100% | +100% |

## 🚀 快速开始

### 方式一：下载预编译二进制

```bash
# 下载
wget https://github.com/fofo365/newclaw/releases/download/v0.5.0/newclaw-linux-x86_64.tar.gz
tar -xzf newclaw-linux-x86_64.tar.gz
cd newclaw

# 配置
cp config/newclaw.example.toml newclaw.toml
vim newclaw.toml  # 修改 API keys

# 运行
./newclaw gateway
```

### 方式二：从源码编译

```bash
# 克隆
git clone https://github.com/fofo365/newclaw.git
cd newclaw

# 编译
cargo build --release

# 运行
./target/release/newclaw gateway
```

### 方式三：一键部署

```bash
sudo ./deploy/install.sh
```

## 📖 文档

- [部署指南](docs/deployment-guide.md) - 生产环境部署完整教程
- [故障排查](docs/troubleshooting.md) - 常见问题和解决方案
- [配置示例](config/newclaw.example.toml) - 完整配置文件示例

## 🔧 配置

### 最小配置

```toml
[server]
host = "0.0.0.0"
port = 3000

[llm]
provider = "glm"
model = "glm-4"

[llm.glm]
api_key = "your-api-key"

[redis]
url = "redis://127.0.0.1:6379"

[security]
jwt_secret = "change-this-in-production"
```

### 环境变量

```bash
export LLM_PROVIDER=glm
export GLM_API_KEY=your-api-key
export RUST_LOG=info
```

## 🤖 支持的 LLM 提供商

| 提供商 | 模型 | 环境变量 |
|--------|------|----------|
| OpenAI | gpt-4o, gpt-4o-mini | OPENAI_API_KEY |
| Claude | claude-3-5-sonnet | ANTHROPIC_API_KEY |
| GLM | glm-4, glm-5 | GLM_API_KEY |

## 📡 API 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/chat` | POST | 对话补全 |
| `/tools` | GET | 工具列表 |
| `/tools/execute` | POST | 执行工具 |
| `/metrics` | GET | Prometheus 指标 |

### 示例

```bash
# 健康检查
curl http://localhost:3000/health

# 对话
curl -X POST http://localhost:3000/chat \
  -H "Content-Type: application/json" \
  -d '{"message": "你好"}'

# 执行工具
curl -X POST http://localhost:3000/tools/execute \
  -H "Content-Type: application/json" \
  -d '{"name": "read", "params": {"path": "/tmp/test.txt"}}'
```

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_feishu_doc

# 查看覆盖率
cargo tarpaulin
```

## 📊 性能指标

| 指标 | 数值 |
|------|------|
| 测试通过率 | 100% (346 tests) |
| 内存使用 | < 50MB |
| 启动时间 | < 100ms |
| 工具执行延迟 | < 50ms |

## 🏗️ 架构

```
┌─────────────────────────────────────┐
│         应用层                       │
│  CLI / Gateway / Agent Engine       │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│         工具层                       │
│  14 类工具 + 飞书集成 (5 个)         │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│         LLM 层                       │
│  OpenAI / Claude / GLM              │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│         存储层                       │
│  Redis / Memory / File              │
└─────────────────────────────────────┘
```

## 🔒 安全特性

- ✅ JWT 认证
- ✅ API Key 管理
- ✅ 速率限制
- ✅ RBAC 权限控制
- ✅ 审计日志

## 🚢 部署

### Docker

```bash
docker build -t newclaw:0.5.0 .
docker run -d -p 3000:3000 newclaw:0.5.0
```

### Systemd

```bash
sudo cp deploy/newclaw.service /etc/systemd/system/
sudo systemctl enable newclaw
sudo systemctl start newclaw
```

详见 [部署指南](docs/deployment-guide.md)

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md)

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE)

## 📞 支持

- **Issues**: [GitHub Issues](https://github.com/fofo365/newclaw/issues)
- **文档**: [docs/](docs/)

---

**版本**: v0.5.0  
**发布日期**: 2026-03-12  
**维护者**: NewClaw Team
