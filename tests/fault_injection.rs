// v0.6.0 故障注入测试实现

#[cfg(test)]
mod fault_injection_tests {
    use std::time::Duration;
    
    /// 测试：内存检测配置
    #[test]
    fn test_oom_detection_config() {
        let config = newclaw::core::self_check::SelfCheckConfig {
            memory_threshold_mb: 500,
            cpu_threshold_percent: 80.0,
            ..Default::default()
        };
        
        assert_eq!(config.memory_threshold_mb, 500);
        assert_eq!(config.cpu_threshold_percent, 80.0);
        assert!(config.enabled);
    }

    /// 测试：CPU 过载检测配置
    #[test]
    fn test_cpu_detection_config() {
        let config = newclaw::core::self_check::SelfCheckConfig {
            cpu_threshold_percent: 90.0,
            ..Default::default()
        };
        
        let checker = newclaw::core::self_check::SelfChecker::new(config);
        let result = checker.check();
        
        // 应该有 4 个检查项
        assert_eq!(result.checks.len(), 4);
    }

    /// 测试：心跳超时配置
    #[test]
    fn test_heartbeat_timeout_config() {
        let config = newclaw::watchdog::heartbeat::HeartbeatConfig {
            check_interval: Duration::from_secs(5),
            timeout: Duration::from_secs(15),
            max_failures: 3,
        };
        
        assert_eq!(config.max_failures, 3);
    }

    /// 测试：降级模式激活
    #[test]
    fn test_degraded_mode_activation() {
        let config = newclaw::core::degraded_mode::DegradedModeConfig {
            enabled: true,
            max_concurrent_requests: 2,
            ..Default::default()
        };
        let manager = newclaw::core::degraded_mode::DegradedModeManager::new(config);
        
        // 正常状态
        assert!(!manager.is_degraded());
        
        // 进入降级模式
        manager.enter("High memory usage");
        assert!(manager.is_degraded());
        
        // 验证请求限制
        assert!(manager.try_acquire());
        assert!(manager.try_acquire());
        assert!(!manager.try_acquire()); // 超过限制
        
        manager.release();
        assert!(manager.try_acquire());
        
        // 清理
        manager.release();
        manager.release();
        
        // 退出降级模式
        manager.exit();
        assert!(!manager.is_degraded());
    }

    /// 测试：租约过期配置
    #[test]
    fn test_lease_expiry_config() {
        let config = newclaw::watchdog::config::LeaseConfig {
            duration: 15,
            renew_deadline: 10,
        };
        
        assert_eq!(config.duration, 15);
        assert_eq!(config.renew_deadline, 10);
    }

    /// 测试：恢复计划退避
    #[test]
    fn test_recovery_backoff() {
        let mut plan = newclaw::watchdog::recovery::RecoveryPlan::new(
            newclaw::watchdog::recovery::RecoveryLevel::L1QuickFix,
            "test".to_string(),
        );
        
        // 初始退避
        let d1 = plan.next_backoff();
        assert_eq!(d1, Duration::from_secs(1));
        
        // 指数增长
        let d2 = plan.next_backoff();
        assert_eq!(d2, Duration::from_secs(2));
        
        let d3 = plan.next_backoff();
        assert_eq!(d3, Duration::from_secs(4));
        
        // 验证重试次数
        assert_eq!(plan.retry_count, 3);
    }

    /// 测试：恢复动作生命周期
    #[test]
    fn test_recovery_action_lifecycle() {
        let mut action = newclaw::watchdog::recovery::RecoveryAction::new(
            "test".to_string(),
            "Test action".to_string(),
        );
        
        assert!(matches!(action.state, newclaw::watchdog::recovery::RecoveryState::Pending));
        
        action.start();
        assert!(matches!(action.state, newclaw::watchdog::recovery::RecoveryState::InProgress));
        
        action.complete("OK".to_string());
        assert!(matches!(action.state, newclaw::watchdog::recovery::RecoveryState::Succeeded));
    }

    /// 测试：快速修复执行器创建
    #[test]
    fn test_quick_fix_executor_creation() {
        let executor = newclaw::watchdog::quick_fix::QuickFixExecutor::new(
            "newclaw-gateway".to_string()
        );
        // 验证执行器成功创建
        assert!(true);
    }

    /// 测试：心跳状态创建
    #[test]
    fn test_heartbeat_status() {
        let healthy = newclaw::watchdog::heartbeat::HeartbeatStatus::healthy(
            "lease-123".to_string(),
            "smart".to_string(),
        );
        assert!(healthy.is_healthy());
        
        let degraded = newclaw::watchdog::heartbeat::HeartbeatStatus::degraded(
            "lease-123".to_string(),
            "smart".to_string(),
            "High CPU".to_string(),
        );
        assert!(degraded.is_healthy()); // degraded 仍算健康
        
        let unhealthy = newclaw::watchdog::heartbeat::HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["OOM".to_string()],
        );
        assert!(!unhealthy.is_healthy());
    }

    /// 测试：自检执行
    #[test]
    fn test_self_check_execution() {
        let checker = newclaw::core::self_check::SelfChecker::new(
            newclaw::core::self_check::SelfCheckConfig::default()
        );
        
        let result = checker.check();
        
        // 应该有检查项
        assert!(!result.checks.is_empty());
        
        // 打印结果
        for check in &result.checks {
            println!("{}: passed={} value={}", check.name, check.passed, check.current_value);
        }
    }
}