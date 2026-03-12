# NewClaw v0.6.0 开发路线图

## 🎯 版本目标

**实现高可用、自愈的 AI Agent 基础设施**

核心特性：
- 分离式双主控架构（核心主控 + 智慧主控）
- 三级恢复策略（L1 快速修复 / L2 AI 诊断 / L3 人工介入）
- 租约机制防止脑裂
- 完整审计日志

---

## 📋 前置条件

### v0.5.5 稳定检查清单
- [ ] 所有测试通过（346 个单元测试 + 集成测试）
- [ ] 零编译警告
- [ ] 生产环境稳定运行 7 天
- [ ] 性能指标达标：
  - [ ] 内存 < 50MB
  - [ ] 启动时间 < 100ms
  - [ ] 工具执行延迟 < 50ms
- [ ] 文档完整：
  - [ ] README.md 更新
  - [ ] CHANGELOG.md 更新
  - [ ] 部署指南验证

### 依赖准备
- [ ] gRPC 巏具链（`tonic`, `prost`）
- [ ] Redis（用于租约存储，可选）
- [ ] 监控系统（Prometheus + Grafana，可选）

---

## 🚀 开发计划（3 周）

### Week 1（3.13 - 3.19）：基础架构

#### Day 1-2：gRPC 协议定义
**目标**：建立双主控通信基础

**任务**：
- [ ] 完善 `watchdog.proto`（已完成初稿）
- [ ] 使用 `tonic` 生成 Rust 代码
- [ ] 创建 `src/proto/generated/` 目录
- [ ] 编写协议文档注释

**交付物**：
- `src/proto/generated/*.rs` - 自动生成的 gRPC 代码
- `docs/v0.6.0-grpc-api.md` - API 文档

#### Day 3-4：租约管理器
**目标**：实现租约机制防止脑裂

**任务**：
- [ ] 实现 `LeaseManager`（内存存储）
- [ ] 实现 `LeaseStorage` trait（支持 Redis）
- [ ] 添加租约过期检测
- [ ] 添加续约逻辑
- [ ] 单元测试（10+ 用例）

**交付物**：
- `src/watchdog/lease.rs` - 租约管理器
- `src/watchdog/storage.rs` - 存储抽象
- 测试覆盖率 > 90%

#### Day 5-7：核心主控框架
**目标**：实现核心主控主循环

**任务**：
- [ ] 实现 `CoreController` 主循环
- [ ] 实现 `HeartbeatChecker` 心跳检测
- [ ] 实现 `AuditLogger` 审计日志
- [ ] 集成租约管理器
- [ ] 添加配置加载
- [ ] 集成测试（3+ 场景）

**交付物**：
- `src/watchdog/controller.rs` - 核心控制器
- `src/watchdog/heartbeat.rs` - 心跳检测器
- `src/watchdog/audit.rs` - 审计日志
- `newclaw-watchdog` 二进制（可启动）

---

### Week 2（3.20 - 3.26）：智慧主控集成

#### Day 1-3：心跳上报 + 自检
**目标**：智慧主控主动上报状态

**任务**：
- [ ] 实现 `HeartbeatReporter` 心跳上报器
- [ ] 实现 `SelfChecker` 自检模块：
  - [ ] 内存检查
  - [ ] CPU 检查
  - [ ] 线程/协程检查
  - [ ] 依赖检查（Redis、数据库）
- [ ] 实现 `DegradedMode` 降级模式
- [ ] 集成到 `newclaw gateway` 启动流程
- [ ] 单元测试（15+ 用例）

**交付物**：
- `src/core/heartbeat_reporter.rs` - 心跳上报器
- `src/core/self_check.rs` - 自检模块
- `src/core/degraded_mode.rs` - 降级模式
- 智慧主控启动时申请租约并开始心跳

#### Day 4-5：L1 快速修复
**目标**：实现快速恢复策略

**任务**：
- [ ] 实现 `QuickFixExecutor`：
  - [ ] 重启服务
  - [ ] 清理缓存
  - [ ] 回滚配置
  - [ ] 释放资源
- [ ] 实现 `DiagnosticEngine` 故障诊断
- [ ] 实现恢复动作的幂等性
- [ ] 单元测试（10+ 用例）

**交付物**：
- `src/recovery/quick_fix.rs` - 快速修复执行器
- `src/diagnostic/analyzer.rs` - 故障分析器
- L1 恢复成功率 > 80%

#### Day 6-7：集成测试
**目标**：验证双主控协作

**任务**：
- [ ] 实现集成测试框架
- [ ] 测试场景：
  - [ ] 正常通信
  - [ ] 心跳失败
  - [ ] 租约过期
  - [ ] L1 恢复
- [ ] 故障注入测试
- [ ] 性能基准测试

**交付物**：
- `tests/watchdog_integration.rs` - 集成测试（完善）
- `tests/fault_injection.rs` - 故障注入测试（完善）
- 测试覆盖率 > 85%

---

### Week 3（3.27 - 4.2）：高级恢复

#### Day 1-3：L2 AI 诊断
**目标**：实现 AI 驱动的故障诊断

**任务**：
- [ ] 实现 `AIDiagnosisExecutor`：
  - [ ] 日志收集和预处理
  - [ ] 调用 LLM 分析日志
  - [ ] 生成修复脚本
  - [ ] 脚本安全验证
- [ ] 实现 `RootCauseAnalyzer` 根因分析
- [ ] 添加 LLM 调用限制（避免费用失控）
- [ ] 单元测试（8+ 用例）

**交付物**：
- `src/recovery/ai_diagnosis.rs` - AI 诊断执行器
- `src/diagnostic/root_cause.rs` - 根因分析器
- L2 恢复成功率 > 60%

#### Day 4-5：L3 人工介入
**目标**：实现人工兜底机制

**任务**：
- [ ] 实现 `HumanInterventionExecutor`：
  - [ ] 多通道告警（飞书/邮件/短信）
  - [ ] 进入安全模式
  - [ ] 等待人工确认
  - [ ] 恢复后验证
- [ ] 实现安全模式（暂停所有自动操作）
- [ ] 实现恢复确认流程
- [ ] 单元测试（5+ 用例）

**交付物**：
- `src/recovery/human_intervention.rs` - 人工介入执行器
- 告警通知功能
- 安全模式文档

#### Day 6-7：压力测试 + 文档
**目标**：验证生产就绪

**任务**：
- [ ] 压力测试：
  - [ ] 心跳风暴（1000 次/秒）
  - [ ] 恢复风暴（连续触发）
  - [ ] 7×24 小时稳定性
- [ ] 文档完善：
  - [ ] `docs/v0.6.0-deployment-guide.md` - 部署指南
  - [ ] `docs/v0.6.0-troubleshooting.md` - 故障排查
  - [ ] `README.md` 更新
  - [ ] `CHANGELOG.md` 更新
- [ ] 发布准备：
  - [ ] 版本号更新（0.6.0）
  - [ ] Git tag
  - [ ] GitHub Release Notes

**交付物**：
- 压力测试报告
- 完整文档
- `v0.6.0` Release

---

## 📊 验收标准

### 功能完整性
- [ ] 核心主控独立运行，资源占用 < 32MB
- [ ] 智慧主控心跳上报正常，3s 间隔
- [ ] 租约机制有效，防止脑裂
- [ ] 三级恢复策略全部实现
- [ ] 审计日志完整记录决策链

### 性能指标
- [ ] 心跳检测延迟 < 10ms
- [ ] L1 恢复时间 < 5s
- [ ] L2 恢复时间 < 30s
- [ ] L3 告警延迟 < 1s
- [ ] 核心主控 CPU < 5%，内存 < 32MB

### 可靠性
- [ ] 单元测试覆盖率 > 90%
- [ ] 集成测试覆盖率 > 85%
- [ ] 故障注入测试通过（10+ 场景）
- [ ] 7×24 小时稳定性测试通过

### 文档
- [ ] 架构设计文档
- [ ] 部署指南
- [ ] 故障排查手册
- [ ] API 文档

---

## 🔗 相关资源

- [架构设计](docs/v0.6.0-watchdog-architecture.md)
- [gRPC 协议](src/proto/watchdog.proto)
- [配置示例](config/watchdog.example.toml)
- [测试计划](tests/watchdog_integration.rs)

---

**状态**: 预设完成，等待 v0.5.5 稳定后启动  
**预计完成**: 2026-04-02  
**最后更新**: 2026-03-12
