// v0.6.0 集成测试 - Watchdog 双主控架构

#[cfg(test)]
mod watchdog_integration_tests {
    use std::sync::Arc;
    use std::time::Duration;
    
    // 从 newclaw 导入所需类型
    use newclaw::watchdog::{
        CoreController, WatchdogConfig, LeaseManager, HeartbeatChecker,
        HeartbeatStatus, DiagnosticEngine, RecoveryExecutor, AuditLogger,
    };
    use newclaw::core::{
        HeartbeatReporter, HeartbeatReporterConfig, SelfChecker, SelfCheckConfig,
        DegradedModeManager, DegradedModeConfig, HealthState,
    };
    
    /// 测试：租约管理基本功能
    #[test]
    fn test_lease_basic_flow() {
        let config = WatchdogConfig::default();
        let controller = CoreController::new(config);
        
        // 1. 获取租约
        let lease = controller.lease_manager()
            .acquire("smart_controller".to_string())
            .expect("Failed to acquire lease");
        
        assert!(controller.lease_manager().is_valid());
        assert_eq!(lease.holder, "smart_controller");
        
        // 2. 续约
        let renewed = controller.lease_manager()
            .renew(&lease.id)
            .expect("Failed to renew lease");
        
        assert!(renewed.last_renewed >= lease.last_renewed);
        
        // 3. 释放租约
        controller.lease_manager()
            .release(&lease.id)
            .expect("Failed to release lease");
        
        assert!(!controller.lease_manager().is_valid());
    }
    
    /// 测试：心跳上报流程
    #[test]
    fn test_heartbeat_reporter() {
        let config = HeartbeatReporterConfig {
            interval_secs: 1,
            component: "smart_controller".to_string(),
            enabled: true,
        };
        let reporter = HeartbeatReporter::new(config);
        
        // 1. 设置租约
        reporter.set_lease_id("lease-123".to_string());
        assert_eq!(reporter.lease_id(), Some("lease-123".to_string()));
        
        // 2. 记录错误
        reporter.record_error("Test error".to_string());
        
        // 3. 生成报告
        let report = reporter.generate_report(HealthState::Healthy);
        assert!(report.is_some());
        
        let report = report.unwrap();
        assert_eq!(report.lease_id, "lease-123");
        assert!(report.health.is_healthy());
    }
    
    /// 测试：自检模块
    #[test]
    fn test_self_checker() {
        let config = SelfCheckConfig::default();
        let checker = SelfChecker::new(config);
        
        // 执行自检
        let result = checker.check();
        
        // 验证检查项
        assert!(!result.checks.is_empty());
        
        // 获取摘要
        let summary = result.summary();
        assert!(summary.contains("checks passed"));
    }
    
    /// 测试：降级模式
    #[test]
    fn test_degraded_mode() {
        let config = DegradedModeConfig {
            max_concurrent_requests: 3,
            disabled_features: vec!["web_search".to_string()],
            ..Default::default()
        };
        let manager = DegradedModeManager::new(config);
        
        // 正常模式，功能可用
        assert!(manager.is_feature_available("web_search"));
        
        // 进入降级模式
        manager.enter("High memory");
        assert!(manager.is_degraded());
        
        // 降级模式，功能受限
        assert!(!manager.is_feature_available("web_search"));
        assert!(manager.is_feature_available("chat"));
        
        // 请求限制
        assert!(manager.try_acquire());
        assert!(manager.try_acquire());
        assert!(manager.try_acquire());
        assert!(!manager.try_acquire()); // 超过限制
        
        // 退出降级模式
        manager.exit();
        assert!(!manager.is_degraded());
    }
    
    /// 测试：诊断引擎模式匹配
    #[test]
    fn test_diagnostic_patterns() {
        let engine = DiagnosticEngine::new();
        
        // 测试 OOM 模式
        let status = HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["Out of memory error".to_string()],
        );
        
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        let result = tokio_runtime.block_on(engine.analyze(&status)).unwrap();
        
        assert!(!result.matched_patterns.is_empty());
        assert!(!result.root_causes.is_empty());
    }
    
    /// 测试：恢复计划生成
    #[test]
    fn test_recovery_plan_generation() {
        use newclaw::watchdog::recovery::RecoveryLevel;
        
        // L1 快速修复计划
        let l1_plan = RecoveryExecutor::generate_l1_plan(
            "smart_controller".to_string(),
            vec!["clear_cache".to_string(), "restart_service".to_string()],
        );
        
        assert_eq!(l1_plan.level, RecoveryLevel::L1QuickFix);
        assert_eq!(l1_plan.actions.len(), 2);
        
        // L3 人工介入计划
        let l3_plan = RecoveryExecutor::generate_l3_plan("smart_controller".to_string());
        
        assert_eq!(l3_plan.level, RecoveryLevel::L3HumanIntervention);
    }
    
    /// 测试：审计日志
    #[test]
    fn test_audit_logging() {
        use newclaw::watchdog::audit::{AuditEvent, EventType};
        
        let config = newclaw::watchdog::config::AuditConfig::default();
        let logger = AuditLogger::new(config);
        
        // 记录事件
        let event = AuditEvent::new(
            EventType::HeartbeatOk,
            "smart_controller".to_string(),
            "Heartbeat OK".to_string(),
        );
        
        logger.log(event).unwrap();
        
        // 获取事件
        let events = logger.get_events().unwrap();
        assert_eq!(events.len(), 1);
        
        // 按类型筛选
        let filtered = logger.filter_by_type(EventType::HeartbeatOk).unwrap();
        assert_eq!(filtered.len(), 1);
    }
    
    /// 测试：完整恢复流程
    #[test]
    fn test_full_recovery_flow() {
        use newclaw::watchdog::recovery::RecoveryLevel;
        
        // 创建审计日志
        let audit_config = newclaw::watchdog::config::AuditConfig::default();
        let audit_log = AuditLogger::new(audit_config);
        
        // 创建恢复执行器
        let executor = RecoveryExecutor::new(audit_log);
        
        // 创建 L1 恢复计划
        let plan = RecoveryExecutor::generate_l1_plan(
            "smart_controller".to_string(),
            vec!["clear_cache".to_string()],
        );
        
        // 执行恢复
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        let result = tokio_runtime.block_on(executor.execute(plan)).unwrap();
        
        assert!(result.success);
        assert_eq!(result.level, RecoveryLevel::L1QuickFix);
    }
    
    /// 测试：心跳失败累积
    #[test]
    fn test_heartbeat_failure_accumulation() {
        use newclaw::watchdog::heartbeat::HeartbeatConfig;
        
        let config = HeartbeatConfig {
            max_failures: 3,
            ..Default::default()
        };
        let checker = HeartbeatChecker::new(config);
        
        // 正常心跳
        let healthy_status = HeartbeatStatus::healthy(
            "lease-123".to_string(),
            "smart".to_string(),
        );
        
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        let result = tokio_runtime.block_on(checker.check(healthy_status));
        assert!(result.is_ok());
        assert_eq!(checker.failure_count(), 0);
        
        // 失败心跳
        let unhealthy_status = HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["Error".to_string()],
        );
        
        // 连续失败
        for i in 1..=3 {
            let result = tokio_runtime.block_on(checker.check(unhealthy_status.clone()));
            if i < 3 {
                assert!(result.is_ok()); // 还没超过阈值
            }
        }
        
        // 超过阈值
        assert!(checker.is_exceeded());
    }
}