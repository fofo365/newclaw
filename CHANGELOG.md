# Changelog

All notable changes to NewClaw will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.7.0] - 2026-03-15

### Added
- **DAG 工作流引擎** - 支持复杂任务编排
- **6 层配置架构** - 灵活的配置管理
- **联邦记忆系统** - 跨节点记忆共享
- **任务调度增强** - Cron 调度器 + 延迟队列 + 事件触发器
- **向量存储和代码库嵌入索引**
- **分层摘要机制** - MMR 去重 + 文件级记忆持久化
- **Dashboard UI 完整支持** - 17 个页面覆盖所有功能
- **CLI 完整支持** - 16 个命令覆盖所有功能

### Dashboard 新增页面
- Tasks.tsx - 任务/DAG/调度管理
- Memory.tsx - 记忆存储/搜索/联邦
- Audit.tsx - 审计日志查询/统计
- Watchdog.tsx - 系统健康监控
- Strategy.tsx - 上下文策略配置
- Sessions.tsx - 会话管理

### CLI 新增命令
- task list/create/status/cancel - 任务管理
- dag list/create/run/status - DAG 工作流
- schedule list/add/remove - 调度管理
- memory store/search/list/delete - 记忆管理
- federation status/sync/nodes - 联邦管理
- audit query/stats/export - 审计日志
- watchdog status/lease/check/recovery - 监控
- strategy list/get/set/config - 策略管理
- session list/create/switch/close - 会话管理

### Fixed
- **问题 #1**: CLI tools 命令显示"工具系统待实现" → 修复 `register_tools` 和 `list_tools`
- **问题 #2**: Dashboard 对话无工具调用能力 → 添加工具调用循环支持
- **问题 #3**: CLI 缺少工具调用能力 → 添加工具调用循环支持
- **问题 #4**: `query.rs` 编译错误 → 修复 `Default` trait impl 块分离
- **问题 #5**: `FusedResult` 可见性警告 → 改为 `pub`
- **问题 #6**: 飞书 WebSocket 连接 404 → 修复 API URL 和请求格式
- **问题 #7**: CLI `-p` 选项冲突 → `--port` 改为长选项
- 导出 `ToolDefinition` 和 `ToolCall` 类型到 `crate::llm`
- Dashboard 启动时自动初始化内置工具

### Changed
- DashboardState 添加 `tool_registry` 字段
- `call_llm` 函数支持工具调用循环（最多 5 轮）
- CLI `process_chat` 支持工具调用循环
- Dashboard 版本号更新到 v0.7.0
- 前端构建并部署到 static/

## [Unreleased]

### Added
- Vector embedding module (v0.5.0 development)
  - `EmbeddingClient` trait for embedding abstraction
  - `OpenAIEmbeddingClient` for OpenAI embeddings API
  - `EmbeddingPipeline` for batch processing
  - `TextChunker` for intelligent text splitting
  - `EmbeddingCache` with LRU eviction and TTL
- Integration tests for embedding module
- Performance benchmarks for embedding operations

### Changed
- Updated `src/lib.rs` to export embedding types
- Restructured context management modules

### Fixed
- (No fixes yet in v0.5.0)

## [0.5.0] - 2026-03-09

## [0.5.5] - 2026-03-12

### Added
- Dashboard/CLI P0 修复
- 多 Agent 记忆共享
- 心跳机制
- 多模型调度
- Ollama 本地模型支持

### Fixed
- 修复 Ollama TokenUsage 类型转换错误（u64 → usize）
- 清理所有编译警告

### Changed
- 更新 `src/lib.rs` 导出 embedding 类型
- 重构上下文管理模块

## [0.5.0] - 2026-03-09

### Added
- Vector embedding module (v0.5.0 development)
  - `EmbeddingClient` trait for embedding abstraction
  - `OpenAIEmbeddingClient` for OpenAI embeddings API
  - `EmbeddingPipeline` for batch processing
  - `TextChunker` for intelligent text splitting
  - `EmbeddingCache` with LRU eviction and TTL
- Integration tests for embedding module
- Performance benchmarks for embedding operations

### Fixed
- (No fixes yet in v0.5.0)

### Added
- Feishu WebSocket connection management
- Dashboard web UI (React + TypeScript)
- Enterprise WeChat integration
- Complete message channel support (QQ, Telegram, Discord)

### Changed
- Improved error handling
- Enhanced configuration system

### Fixed
- System crash due to service conflicts
- Compilation warnings (reduced from 12 to 0)

## [0.4.0] - 2026-03-09

### Added
- Feishu integration (WebSocket + event polling)
- Dashboard web UI
- Security layer (API Key, JWT, RBAC, audit logging, rate limiting)
- Communication interfaces (WebSocket, HTTP API, Redis)
- Context isolation (None/User/Session)

### Changed
- Refactored gateway architecture
- Improved multi-LLM provider support

## [0.3.1] - 2026-03-09

### Fixed
- Gateway multi-LLM provider support
- CLI multi-provider and model selection
- Configuration file support (TOML)
- Tool execution engine integration

## [0.3.0] - 2026-03-08

### Added
- Tool execution engine
- Multi-LLM support (OpenAI, Claude, GLM)
- Configuration system
- Streaming responses

## [0.2.0] - 2026-03-08

### Added
- Security layer (API Key, JWT, RBAC, audit logging, rate limiting)
- Communication interfaces (WebSocket, HTTP API, Redis)
- Context isolation
- Initial agent engine

## [0.1.0] - 2026-03-07

### Added
- Initial release
- Basic agent engine
- Context manager
- GLM provider integration
- Feishu client
- REST API
- Vector store (in-memory)
- Plugin system (Rust traits only)
- OpenClaw compatibility layer

---

**Note**: Versions prior to 0.1.0 were development prototypes and not officially released.
