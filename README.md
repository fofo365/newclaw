# NewClaw v0.7.2

> 生产级 AI Agent 框架 - Rust 性能 + TypeScript 插件生态

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/badge/release-v0.7.2-blue.svg)](https://github.com/fofo365/newclaw/releases/tag/v0.7.2)

## 🎯 项目概述

NewClaw 是新一代 AI Agent 框架，提供：

- **🔧 14 类核心工具** - 文件、Shell、网络、浏览器、记忆等
- **🤖 多 LLM 支持** - OpenAI、Claude、GLM 统一接口
- **📡 7 个消息通道** - 飞书、企微、钉钉、QQ、Telegram、Discord、AGP
- **🧠 飞书集成 100%** - 文档、多维表格、云存储、知识库、聊天
- **🔒 安全特性** - JWT 认证、RBAC 权限、审计日志
- **🚀 高性能** - 内存 < 50MB，启动时间 < 100ms

## 📊 v0.7.2 最新特性

### 🔧 核心改进
- **飞书令牌修复** - 修复飞书文档创建的令牌类型混淆问题
- **系统诊断工具** - Cron 任务管理、系统诊断工作流、AI 行为约束
- **记忆归档恢复** - 自动归档、支持恢复、重要数据保护
- **移除硬编码依赖** - 所有路径从配置文件读取

### v0.7.1 增强功能
- **消息去重机制** - 防止重复处理，重启后仍有效
- **会话历史持久化** - 重启不丢失上下文
- **Token 自动刷新** - 避免连接断开
- **记忆系统修复** - 统一数据路径，FTS5 搜索优化

### v0.7.0 重大更新
- **DAG 工作流引擎** - 6 层配置架构
- **联邦记忆系统** - SQLite 存储 + FTS5 索引
- **审计查询引擎** - 高级查询 + 统计报表
- **ABAC 权限引擎** - 属性定义 + 策略评估
- **配置热更新** - 文件监听 + 热重载

## 🚀 快速开始

### 方式一：下载预编译二进制

```bash
# 下载
wget https://github.com/fofo365/newclaw/releases/download/v0.7.2/newclaw-linux-x86_64.tar.gz
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
```

## 📊 性能指标

| 指标 | 数值 |
|------|------|
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
docker build -t newclaw:0.7.2 .
docker run -d -p 3000:3000 newclaw:0.7.2
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

Apache-2.0 License - 详见 [LICENSE](LICENSE)

## 📞 支持

- **Issues**: [GitHub Issues](https://github.com/fofo365/newclaw/issues)
- **文档**: [docs/](docs/)
- **Releases**: [GitHub Releases](https://github.com/fofo365/newclaw/releases)

---

**版本**: v0.7.2  
**发布日期**: 2026-03-18  
**维护者**: NewClaw Team  
**许可证**: Apache-2.0
