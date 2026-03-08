# NewClaw

**Next-gen AI Agent framework - Rust performance + TypeScript plugins**

## 🎯 特性

- 🚀 高性能 Rust 核心（30-40MB 内存）
- 🔌 TypeScript 插件系统（动态加载、热更新）
- 🧠 智能上下文管理（向量化存储、语义检索）
- 🎛️ 策略引擎（可配置的策略仓库）
- 📊 低 Token 消耗（智能截断、动态规划）
- 🌍 Feishu/Lark 通道（基于 simonaries 修复）
- 🔄 Web Dashboard（可视化配置）

## 🚀 快速开始

### 安装

```bash
# 安装
cargo install --path . --features "channel-lark,plugin"

# 运行
$ newclaw agent    # CLI 模式
$ newclaw gateway  # Web Gateway
$ newclaw plugin --list  # 列出插件
```

## 📚 文档

- [架构设计](./ARCHITECTURE.md)
- [开发指南](./DEVELOPMENT.md)
- [API 参考](./API.md)
- [配置指南](./CONFIG.md)

## 🎯 优势

### vs ZeroClaw
- ✅ **更灵活**：插件系统
- ✅ **更智能**：上下文管理
- ✅ **更省成本**：智能截断

### vs OpenClaw
- ✅ **更高性能**：Rust 核心
- ✅ **更好扩展性**：Rust + TypeScript
- ✅ **更高效**：原生向量化、智能检索

## 🤝 社区

- GitHub: https://github.com/yourusername/newclaw
- Discord: https://discord.gg/newclaw
- 飞书: https://openclaw
