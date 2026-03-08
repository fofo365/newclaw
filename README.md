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

### 当前状态

✅ **已完成 (v0.1.0)**
- Rust 核心引擎 (编译通过)
- 上下文管理器 (SQLite 存储)
- 智能截断策略
- Token 估算器
- CLI 交互模式
- 完整的项目结构

⏳ **待开发**
- LLM 集成
- 更多策略实现
- 向量数据库集成
- TypeScript 插件系统
- Feishu/Lark 通道
- Web Dashboard

### 安装与运行

```bash
# 编译
cd /root/newclaw
cargo build --release

# 运行 CLI 模式
./target/release/newclaw agent

# 或使用 cargo 直接运行
cargo run -- agent
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
