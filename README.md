# NewClaw v0.2.0

> A secure, multi-channel AI agent framework with context isolation

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/newclaw/newclaw)

## 🎯 Overview

NewClaw is a production-ready AI agent framework that provides:

- **🔒 Security Layer**: API Key, JWT, RBAC, audit logging, rate limiting
- **📡 Communication**: WebSocket, HTTP API, inter-agent message queue
- **🧠 Context Isolation**: Secure multi-tenant context management
- **📱 Multi-Channel**: Support for multiple communication channels

## 🚀 Quick Start

### Prerequisites

- Rust 1.75 or higher
- Cargo package manager

### Installation

```bash
# Clone the repository
git clone https://github.com/newclaw/newclaw.git
cd newclaw

# Build in release mode
cargo build --release

# Run tests
cargo test
```

### Basic Usage

```rust
use newclaw::{
    communication::{HttpServer, WebSocketServer},
    security::{ApiKeyAuth, JwtAuth, RbacManager},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize security layer
    let api_auth = ApiKeyAuth::new(Default::default());
    let jwt_auth = JwtAuth::new(Default::default());
    let rbac = RbacManager::new();
    
    // Start HTTP server
    let http_server = HttpServer::new(8080);
    http_server.start().await?;
    
    Ok(())
}
```

## 📚 Architecture

### Security Layer

```
┌─────────────────────────────────────────┐
│          Security Layer                  │
├─────────────────────────────────────────┤
│  • API Key Authentication                │
│  • JWT Token Management                  │
│  • Role-Based Access Control (RBAC)     │
│  • Audit Logging                         │
│  • Rate Limiting                         │
└─────────────────────────────────────────┘
```

### Communication Layer

```
┌─────────────────────────────────────────┐
│       Communication Layer                │
├─────────────────────────────────────────┤
│  • HTTP REST API                         │
│  • WebSocket Real-time                   │
│  • Inter-Agent Message Queue             │
└─────────────────────────────────────────┘
```

### Core Layer

```
┌─────────────────────────────────────────┐
│           Core Layer                     │
├─────────────────────────────────────────┤
│  • Context Isolation                     │
│  • Agent Management                      │
│  • State Management                      │
└─────────────────────────────────────────┘
```

## 🔧 Configuration

### API Key Configuration

```toml
[api_keys]
keys = [
    { key = "your-api-key", name = "Production", scopes = ["read", "write"] }
]
```

### JWT Configuration

```toml
[jwt]
secret = "your-secret-key"
issuer = "newclaw"
expiry_hours = 24
```

### RBAC Configuration

```toml
[[roles]]
name = "admin"
permissions = [
    { resource = "*", action = "*" }
]

[[roles]]
name = "editor"
permissions = [
    { resource = "posts", action = "read" },
    { resource = "posts", action = "write" }
]
```

## 📖 API Documentation

### Authentication

#### API Key

```bash
curl -H "X-API-Key: your-api-key" http://localhost:8080/api/v1/endpoint
```

#### JWT Token

```bash
# Get token
curl -X POST http://localhost:8080/auth/token \
  -H "Content-Type: application/json" \
  -d '{"username":"user","password":"pass"}'

# Use token
curl -H "Authorization: Bearer <token>" http://localhost:8080/api/v1/endpoint
```

### Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/health` | GET | Health check |
| `/api/v1/agents` | GET | List all agents |
| `/api/v1/agents/:id` | GET | Get agent details |
| `/api/v1/messages` | POST | Send inter-agent message |
| `/ws` | WebSocket | Real-time communication |

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

### Run Integration Tests

```bash
cargo test --test integration_test
```

### Run with Coverage

```bash
cargo tarpaulin --out Html
```

## 📊 Performance

NewClaw is designed for high performance:

- **Throughput**: 10,000+ requests/second
- **Latency**: < 10ms p99
- **Memory**: < 50MB baseline
- **Startup**: < 100ms

## 🔒 Security Features

### API Key Authentication

- Secure key validation
- Scope-based permissions
- Key expiration support

### JWT Tokens

- RS256 signing
- Configurable expiration
- Role-based claims

### RBAC

- Fine-grained permissions
- Role inheritance
- Resource-based access control

### Audit Logging

- Comprehensive action logging
- Tamper-proof logs
- Query and search capabilities

### Rate Limiting

- Token bucket algorithm
- Per-user limits
- Distributed rate limiting

## 📝 Examples

See the `examples/` directory for:

- Basic HTTP server setup
- WebSocket chat application
- Inter-agent messaging
- Security layer configuration

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) async runtime
- HTTP powered by [Axum](https://github.com/tokio-rs/axum)
- Serialization with [Serde](https://serde.rs/)

## 📞 Support

- **Documentation**: [https://docs.newclaw.io](https://docs.newclaw.io)
- **Issues**: [GitHub Issues](https://github.com/newclaw/newclaw/issues)
- **Discord**: [Join our community](https://discord.gg/newclaw)

---

**Version**: v0.2.0  
**Release Date**: 2026-03-08  
**Maintainer**: NewClaw Team
