# Issue #003: 运行时硬编码依赖 OpenClaw 目录

**严重级别**: P0 (Critical - 架构缺陷)  
**状态**: 🔴 Open  
**发现时间**: 2026-03-18 13:26  
**影响范围**: 系统独立性、稳定性、安全性  
**负责人**: AI Agent (GLM-5) / 另一智能体修复中

---

## 📋 问题描述

NewClaw 在运行时**硬编码依赖** OpenClaw 目录，违背了系统独立性原则。

### 问题代码位置

| 文件 | 行号 | 问题 |
|------|------|------|
| `src/skill/loader.rs` | 56-57 | 技能加载路径硬编码为 `/root/.openclaw/` |
| `src/feishu_websocket/tools.rs` | 97 | workspace 路径硬编码为 `/root/.openclaw/workspace-dev` |

### 问题代码

```rust
// src/skill/loader.rs:56-57
loader.search_paths.push(PathBuf::from("/root/.openclaw/workspace/skills"));
loader.search_paths.push(PathBuf::from("/root/.openclaw/extensions"));

// src/feishu_websocket/tools.rs:97
let workspace = PathBuf::from("/root/.openclaw/workspace-dev");
```

---

## 🚨 严重性分析

### 根本性错误

1. **系统耦合**: NewClaw 运行时读取 OpenClaw 的插件
2. **相互影响**: 修改 OpenClaw 插件会影响 NewClaw
3. **违背原则**: 两个系统应该是独立的，不应有运行时依赖

### 潜在风险

| 风险 | 影响 |
|------|------|
| 数据混乱 | OpenClaw 和 NewClaw 共享状态，可能导致数据冲突 |
| 安全隐患 | 修改一个系统可能破坏另一个系统 |
| 部署问题 | 无法独立部署 NewClaw |
| 维护困难 | 无法独立升级和测试 |

### 影响范围

- **v0.7.2 及之前所有版本**（从 commit 6aee05f2 开始引入）
- 所有使用技能加载功能的场景
- 所有 feishu-websocket 通道

---

## 🎯 修复方案

### 需要修改的文件

| 文件 | 修改内容 |
|------|----------|
| `src/skill/loader.rs` | 改为 NewClaw 自己的技能路径 |
| `src/feishu_websocket/tools.rs` | 改为 NewClaw 自己的 workspace |
| `src/bin/migrate.rs` | 保留（迁移工具，不影响运行时） |
| `src/openclaw/mod.rs` | 保留（兼容层，不影响运行时） |

### 推荐路径

```rust
// src/skill/loader.rs - 改为
loader.search_paths.push(PathBuf::from("/root/newclaw/skills"));
loader.search_paths.push(PathBuf::from("/root/newclaw/extensions"));

// src/feishu_websocket/tools.rs - 改为
let workspace = PathBuf::from("/var/lib/newclaw/workspace");
// 或使用配置文件指定路径
```

---

## 📝 教训总结

### 根本原因

1. **复制代码未审查**: 从 OpenClaw 复制代码时未检查硬编码路径
2. **缺少代码评审**: 关键代码变更没有经过严格审查
3. **缺少单元测试**: 没有测试验证路径是否正确

### 防止复发

- [ ] **代码评审检查项**: 检查所有硬编码路径
- [ ] **单元测试**: 添加路径验证测试
- [ ] **配置化**: 路径应从配置文件读取，而非硬编码
- [ ] **CI 检查**: 添加静态分析检查硬编码路径

---

## 🔄 关联 Issue

- **#001**: 内存耗尽导致系统崩溃
- **#002**: systemd 疯狂重启

---

## 📚 参考资料

- Commit: 6aee05f2 - 引入 OpenClaw 兼容层
- OpenClaw 架构文档
- NewClaw 设计文档

---

**状态**: 🔴 待修复（另一智能体处理中）  
**最后更新**: 2026-03-18 13:26 UTC+8