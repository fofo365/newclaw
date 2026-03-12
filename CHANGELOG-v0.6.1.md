# Changelog - v0.6.1

## [0.6.1] - 2026-03-12

### Added - Phase 1 核心集成

#### 独立二进制
- **`newclaw-watchdog`**: 核心主控独立可执行文件
  - 轻量级看门狗进程（< 32MB 内存）
  - gRPC 服务器（默认端口 50051）
  - CLI 参数支持：port, host, check-interval, heartbeat-timeout, max-failures, lease-duration

#### 智慧主控集成
- **`SmartController`**: 智慧主控管理器
  - 心跳上报器集成（HeartbeatReporter）
  - 自检模块集成（SelfChecker）
  - 降级模式集成（DegradedModeManager）
  - 后台任务自动启动

#### Gateway 增强
- **Watchdog 集成**: 
  - 新增 `enable_watchdog` 配置项
  - 新增 `watchdog_addr` 配置项
  - 启动时自动初始化智慧主控
- **新增端点**:
  - `/ready` - 就绪检查（包含 LLM Provider、Smart Controller、Lease 状态）

#### 配置系统
- **GatewayConfig 扩展**:
  - `enable_watchdog: bool` - 是否启用 Watchdog 集成
  - `watchdog_addr: String` - Watchdog gRPC 地址

### Changed
- 版本号升级至 0.6.1
- `src/watchdog/mod.rs` 导出 `WatchdogGrpcServer`
- `src/lib.rs` 添加 `smart_controller` 模块

### Testing
- **545 个单元测试全部通过**
- 新增 `smart_controller` 模块测试

### Next Steps
- [ ] Phase 2: L1 动作实际实现
- [ ] Phase 2: L3 通知完善（邮件/短信）
- [ ] Phase 2: 故障注入测试实现
- [ ] Phase 3: 压力测试和文档

---

## 完成进度

| 阶段 | 任务 | 状态 |
|------|------|------|
| Phase 1 | 核心集成 | ✅ 100% |
| Phase 2 | 功能完善 | ⏳ 30% |
| Phase 3 | 生产就绪 | ⏳ 10% |

**总体完成度**: **75% → 80%**