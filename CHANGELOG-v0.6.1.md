# Changelog - v0.6.1

## [0.6.1] - 2026-03-13

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

### Added - Phase 2 功能完善

#### L1 快速修复执行器
- **`QuickFixExecutor`**: 真实系统调用实现
  - `restart_service()`: 通过 systemd 重启服务
  - `clear_cache()`: Redis FLUSHDB 或内存缓存清理
  - `rollback_config()`: Git 配置回滚
  - `release_resources()`: 清理临时文件、触发内存回收
  - `check_service_status()`: 检查服务运行状态

#### 故障注入测试
- `test_oom_detection_config` - 内存检测配置测试
- `test_cpu_detection_config` - CPU 检测配置测试
- `test_heartbeat_timeout_config` - 心跳超时配置测试
- `test_degraded_mode_activation` - 降级模式激活测试
- `test_lease_expiry_config` - 租约过期配置测试
- `test_recovery_backoff` - 恢复退避策略测试
- `test_recovery_action_lifecycle` - 恢复动作生命周期测试
- `test_quick_fix_executor_creation` - 快速修复执行器创建测试
- `test_heartbeat_status` - 心跳状态测试
- `test_self_check_execution` - 自检执行测试

### Changed
- 版本号升级至 0.6.1
- `src/watchdog/mod.rs` 导出 `WatchdogGrpcServer` 和 `QuickFixExecutor`
- `src/lib.rs` 添加 `smart_controller` 模块
- `tests/fault_injection.rs` 重写为实际测试用例

### Testing
- **551 个单元测试全部通过** ✅
- 新增 10 个故障注入测试

### Next Steps (Phase 3)
- [ ] L3 通知完善（邮件/短信）
- [ ] 压力测试和性能基准
- [ ] 部署文档和故障排查手册

---

## 完成进度

| 阶段 | 任务 | 状态 |
|------|------|------|
| Phase 1 | 核心集成 | ✅ 100% |
| Phase 2 | 功能完善 | ✅ 80% |
| Phase 3 | 生产就绪 | ⏳ 20% |

**总体完成度**: **85%**