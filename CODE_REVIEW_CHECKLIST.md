# Code Review Checklist - 代码评审检查清单

## 🔴 关键检查项（必须通过）

### 1. 硬编码路径检查

**检查内容**：
- [ ] 没有硬编码的绝对路径（如 `/root/.openclaw/`, `/root/newclaw/`）
- [ ] 路径应从配置文件读取
- [ ] 临时路径应使用系统临时目录

**示例违规**：
```rust
// ❌ 错误
let workspace = PathBuf::from("/root/.openclaw/workspace-dev");

// ✅ 正确
let workspace = config.get("workspace.path").unwrap_or("/var/lib/newclaw/workspace");
```

### 2. 系统独立性检查

**检查内容**：
- [ ] NewClaw 不依赖 OpenClaw 的运行时状态
- [ ] 两个系统可以独立部署和运行
- [ ] 修改一个系统不会影响另一个

### 3. 配置化检查

**检查内容**：
- [ ] 所有可配置项都有配置文件支持
- [ ] 没有魔法数字或硬编码值
- [ ] 默认值合理且安全

---

## 🟡 重要检查项

### 4. 错误处理

- [ ] 所有 Result 都被正确处理
- [ ] 错误信息包含足够的上下文
- [ ] 危险操作有确认机制

### 5. 安全检查

- [ ] 没有明文的敏感信息
- [ ] 权限检查到位
- [ ] 输入验证完整

### 6. 性能检查

- [ ] 没有明显的性能问题
- [ ] 资源正确释放
- [ ] 避免不必要的克隆

---

## 📝 检查流程

1. **提交前自检**：开发者自行检查上述项目
2. **代码评审**：至少一人审核，重点关注关键检查项
3. **CI 检查**：自动化检查（添加静态分析规则）
4. **合并前确认**：确认所有检查项通过

---

## 🚨 历史教训

### Issue #003: 运行时硬编码依赖 OpenClaw 目录

**发现时间**：2026-03-18  
**严重性**：P0（架构缺陷）

**问题**：
- `src/skill/loader.rs:56-57` 硬编码 `/root/.openclaw/workspace/skills`
- `src/feishu_websocket/tools.rs:97` 硬编码 `/root/.openclaw/workspace-dev`

**后果**：
- NewClaw 和 OpenClaw 系统耦合
- 修改 OpenClaw 会影响 NewClaw
- 违背独立性原则

**预防措施**：
- 所有路径必须从配置读取
- 代码评审必须检查硬编码路径
- 添加静态分析检查

---

**最后更新**：2026-03-18