# USER.md - 用户信息

_用户是谁？他们的偏好是什么？_

---

## 基本信息

- **姓名**: 王老吉
- **称呼**: 王老吉 / 老吉
- **时区**: Asia/Shanghai (GMT+8)
- **语言**: 中文

---

## 项目背景

### NewClaw 项目

**角色**: 主要开发者

**项目描述**: AI Agent 框架

**当前版本**: v0.7.0

**主要模块**:
- Dashboard UI (React + Vite + Ant Design)
- 飞书集成 (Feishu WebSocket)
- 联邦记忆 (Federated Memory)
- 任务调度 (Cron + 延迟队列)
- CLI 工具
- 多渠道支持 (Telegram, Discord, WhatsApp)

---

## 工作偏好

### 沟通习惯

1. **消息确认**: 收到消息后，先回复"已收到，等待我的处理。"再执行任务
2. **快速迭代**: 喜欢快速看到结果
3. **版本追踪**: 重视版本信息和构建时间显示

### 开发习惯

- **分支策略**: 使用 `main` 分支进行开发
- **提交频率**: 频繁提交，保持小步快跑
- **测试重视**: 所有功能都要有测试

---

## 技术背景

### 熟悉的技术栈

- **后端**: Rust, Cargo
- **前端**: React, TypeScript, Vite, Ant Design
- **数据库**: SQLite
- **消息**: WebSocket, HTTP
- **云服务**: 飞书开放平台

### 当前关注点

- 记忆系统的多层隔离
- Token 自动刷新机制
- 多通道统一处理
- Workspace 打包转移

---

## 环境信息

### 服务器

- **主机**: VM-0-13-ubuntu
- **系统**: Ubuntu 22.04 (Linux 6.8.0-71-generic)
- **架构**: x86_64

### 开发环境

- **Node.js**: v22.22.1
- **Rust**: cargo 1.94.0
- **默认模型**: qwencode/glm-5

---

## 联系方式

- **飞书**: ou_1fd6d40ae1fa693340b85a97428973be
- **飞书应用**: cli_a928559df7b8dbcc

---

## 历史交互

### 2026-03-16

- ✅ 完成 ChannelProcessor 统一消息处理器
- ✅ 实现多层隔离机制 (MemoryScope)
- ✅ 实现 Token 自动刷新
- ✅ 集成到 CLI/Dashboard/Feishu
- ✅ 创建 workspace 目录结构

---

_这个文件记录了用户的个人信息和偏好，确保 AI 助手能够提供个性化的服务。_

_最后更新: 2026-03-16_