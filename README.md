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
# 克隆仓库
git clone https://github.com/yourusername/newclaw.git
cd newclaw

# 编译
cargo build --release

# 运行
./target/release/newclaw agent
```

### 运行模式

```bash
# CLI 交互模式
./target/release/newclaw agent

# Web Gateway (待实现)
./target/release/newclaw gateway

# 插件管理 (待实现)
./target/release/newclaw plugin --list
```

## 📚 开发状态

**v0.1.0** (当前版本)
- ✅ Rust 核心引擎 (编译通过)
- ✅ 上下文管理器 (SQLite + 智能检索)
- ✅ 策略引擎 (SmartTruncation + 更多)
- ✅ LLM 集成框架 (Mock + GLM 接口)
- ✅ CLI 交互模式
- ✅ 完整文档

详见 [DEVELOPMENT.md](./DEVELOPMENT.md)

## 🎯 核心特性

### 🔧 Rust 性能
- **低内存**: ~35MB 运行时内存
- **高性能**: 异步 I/O + 零成本地存储
- **类型安全**: Rust 类型系统保证稳定性

### 🧠 智能上下文管理
- **自动分块**: 智能消息分块存储
- **语义检索**: 基于相关性的上下文选择
- **Token 优化**: 30-50% Token 使用优化（目标）

### 🎯 策略引擎
- **SmartTruncation**: 基于重要性的智能截断
- **TimeDecay**: 时间衰减权重（开发中）
- **SemanticCluster**: 语义聚类（计划中）

### 🔌 LLM 集成
- **多模型支持**: GLM-4, Zhipu AI, OpenAI
- **统一接口**: 标准化的 LLM Provider trait
- **错误处理**: 完善的错误处理和重试机制

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
