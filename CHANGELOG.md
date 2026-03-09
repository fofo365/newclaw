# Changelog

All notable changes to NewClaw will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-03-09

### Fixed
- **Gateway multi-provider support**: Gateway now correctly reads LLM provider configuration
  - Previously hardcoded to GLM-4
  - Now supports OpenAI, Claude, and GLM based on configuration
  - Environment variables properly override config file values
  
- **CLI multi-provider support**: CLI now supports all three providers
  - Added `--provider` flag (openai/claude/glm)
  - Added `--model` flag for model selection
  - Proper API key detection with helpful error messages

- **Tool execution integration**: Tools are now properly integrated into both CLI and Gateway
  - ToolRegistry initialized with all built-in tools
  - `/tools` endpoint lists available tools
  - `/tools/execute` endpoint runs tools

### Added
- **Configuration system** (`config.toml`)
  - TOML-based configuration file support
  - Environment variable overrides
  - Provider-specific credential sections
  - Gateway host/port configuration
  - Tool enable/disable configuration

- **CLI commands**
  - `newclaw config` - Generate example configuration file
  - `newclaw tools list` - List available tools
  - `newclaw tools exec <name>` - Execute a tool directly
  - `newclaw gateway` - Start the HTTP gateway server
  - `newclaw chat` - Interactive chat mode (default)

- **Config module** (`src/config/mod.rs`)
  - `Config::load()` - Auto-load from default locations
  - `Config::from_file()` - Load from specific path
  - Environment variable override support
  - Provider credential management

### Changed
- Default behavior: `newclaw` without arguments starts interactive chat
- Gateway uses shared state with LLM provider
- Tool registration is now async with proper Arc wrapping

### Technical Details
- Added `shellexpand` dependency for path expansion
- Added `toml` dependency for config parsing
- All built-in tools now implement `Default` trait
- Proper error messages for missing API keys

## [0.3.0] - 2026-03-08

### Added
- **Tool Execution Engine**
  - Type-safe `Tool` trait
  - Built-in tools: read, write, edit, exec, search
  - Automatic retry mechanism (up to 3 attempts)
  - ToolRegistry for tool management

- **Multi-LLM Architecture**
  - `LLMProviderV3` trait for unified interface
  - OpenAI provider implementation
  - Claude provider implementation
  - GLM provider (legacy compatibility)
  - Model switching strategies

- **Streaming Support**
  - SSE streaming
  - WebSocket streaming
  - Feishu streaming adapter

### Changed
- Refactored LLM module for multi-provider support
- Added comprehensive test coverage

## [0.2.0] - 2026-03-07

### Added
- **Security Layer**
  - API Key Authentication
  - JWT Token Management
  - Role-Based Access Control (RBAC)
  - Audit Logging
  - Rate Limiting

- **Communication Layer**
  - WebSocket server/client
  - HTTP REST API
  - Redis message queue (optional)

- **Context Isolation**
  - Multi-tenant support
  - Secure context management

## [0.1.0] - 2026-03-01

### Added
- Initial release
- Basic agent engine
- Simple LLM integration
- Context management
- Strategy engine

---

## Upgrade Guide

### From 0.3.0 to 0.3.1

1. **No breaking changes** - All existing code should work

2. **Optional: Create config file**
   ```bash
   newclaw config --output config.toml
   # Edit with your API keys
   ```

3. **Environment variables still work**
   - `LLM_PROVIDER`
   - `OPENAI_API_KEY`
   - `ANTHROPIC_API_KEY`
   - `GLM_API_KEY`

4. **Gateway mode now respects configuration**
   - Previously: Always used GLM-4
   - Now: Uses configured provider

### From 0.2.0 to 0.3.0

1. **LLM imports changed**
   ```rust
   // Old
   use newclaw::llm::LLMProvider;
   
   // New
   use newclaw::llm::{LLMProviderV3, OpenAIProvider, ClaudeProvider};
   ```

2. **Tool system added**
   ```rust
   use newclaw::tools::{ToolRegistry, ReadTool, WriteTool};
   
   let registry = ToolRegistry::new();
   registry.register(Arc::new(ReadTool::default())).await;
   ```

---

## Roadmap

### v0.3.2 (Planned)
- [ ] Streaming support for Gateway
- [ ] Plugin system for custom tools
- [ ] Conversation memory persistence
- [ ] More model strategies

### v0.4.0 (Planned)
- [ ] Multi-agent orchestration
- [ ] Agent-to-agent communication
- [ ] Distributed deployment support
- [ ] Vector database integration
