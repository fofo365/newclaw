# Changelog - v0.6.0 Preview

All notable changes for NewClaw v0.6.0 will be documented in this file.

## [0.6.0] - 2026-03-XX (Planned)

### Added - 分离式双主控架构

#### 核心主控（Watchdog）
- **独立二进制**: `newclaw-watchdog`，极轻量（< 32MB 内存）
- **租约管理器**: 防止脑裂，15s 过期，10s 续约
- **心跳检测器**: 5s 间隔检测，3 次失败触发恢复
- **诊断引擎**: 日志分析 + 模式匹配 + 根因分析
- **恢复执行器**: 三级恢复策略（L1/L2/L3）
- **审计日志**: 完整决策链记录，JSON 格式

#### 智慧主控增强（Smart Controller）
- **心跳上报器**: 3s 间隔向核心主控上报状态
- **自检模块**: 内存/CPU/线程/依赖检查
- **降级模式**: 故障时自动降级，限制功能

#### 恢复策略
- **L1 - 快速修复**: 重启服务、清理缓存、回滚配置（< 1s）
- **L2 - AI 诊断**: 调用 LLM 分析日志，生成修复脚本（5-30s）
- **L3 - 人工介入**: 告警通知，进入安全模式（立即）

#### gRPC 协议
- `HeartbeatService`: 心跳上报和查询
- `LeaseService`: 租约申请、续约、释放
- `RecoveryService`: 恢复触发和状态查询
- `HealthCheckService`: 主动健康探测

#### 配置和部署
- `watchdog.toml`: 核心主控配置
- `newclaw.toml`: 智慧主控配置（新增 watchdog_client 部分）
- `newclaw-watchdog.service`: 核心主控 systemd 服务
- `newclaw-smart.service`: 智慧主控 systemd 服务（更新依赖关系）

### Changed
- 智慧主控启动时先申请租约，失败则等待
- 智慧主控增加自检逻辑，异常时主动降级
- 审计日志格式统一为 JSON

### Security
- gRPC 通信支持 mTLS（可选）
- 核心主控独立用户和权限
- 资源限制（MemoryMax=64M, CPUQuota=10%）

### Documentation
- `docs/v0.6.0-watchdog-architecture.md`: 架构设计文档
- `docs/v0.6.0-deployment-guide.md`: 双主控部署指南（待完成）
- `docs/v0.6.0-troubleshooting.md`: 故障排查（待完成）

---

## 开发计划

### Week 1（3.13 - 3.19）：基础架构
- [ ] gRPC 协议定义 + 代码生成
- [ ] 租约管理器实现
- [ ] 核心主控框架 + 心跳检测

### Week 2（3.20 - 3.26）：智慧主控集成
- [ ] 心跳上报器 + 自检模块
- [ ] L1 快速修复策略
- [ ] 集成测试

### Week 3（3.27 - 4.2）：高级恢复
- [ ] L2 AI 诊断
- [ ] L3 人工介入
- [ ] 压力测试 + 文档

---

**注意**: v0.6.0 将在 v0.5.5 稳定后启动开发。
