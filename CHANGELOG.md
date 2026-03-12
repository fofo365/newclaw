# Changelog

All notable changes to NewClaw will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
