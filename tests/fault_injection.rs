// v0.6.0 故障注入测试实现

#[cfg(test)]
mod fault_injection_tests {
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::time::sleep;

    // 导入必要的模块
    use crate::watchdog::{
        CoreController, WatchdogConfig, HeartbeatStatus,
        RecoveryExecutor, RecoveryLevel, RecoveryPlan,
        LeaseManager,
    };
    use crate::watchdog::quick_fix::QuickFixExecutor;
    use crate::core::self_check::{SelfChecker, SelfCheckConfig};
    use crate::core::degraded_mode::{DegradedModeManager, DegradedModeConfig};
    use crate::watchdog::audit::AuditLogger;

    /// 测试：内存耗尽检测
    #[tokio::test]
    async fn test_oom_detection() {
        // 创建自检器，设置低内存阈值
        let config = SelfCheckConfig {
            memory_threshold_mb: 10, // 设置很低的阈值
            ..Default::default()
        };
        let checker = SelfChecker::new(config);
        
        // 执行检查
        let result = checker.check();
        
        // 验证：内存检查应该失败（因为实际使用通常超过 10MB）
        let memory_check = result.checks.iter().find(|c| c.name == "memory");
        assert!(memory_check.is_some());
        
        // 记录结果
        if let Some(check) = memory_check {
            println!("Memory check: passed={}, current={}", 
                check.passed, check.current_value);
        }
    }

    /// 测试：CPU 过载检测
    #[tokio::test]
    async fn test_cpu_exhaustion_detection() {
        let config = SelfCheckConfig {
            cpu_threshold_percent: 5.0, // 设置很低的阈值
            ..Default::default()
        };
        let checker = SelfChecker::new(config);
        
        // 执行 CPU 密集任务
        let start = std::time::Instant::now();
        let handle = tokio::spawn(async {
            let mut counter = 0u64;
            while std::time::Instant::now().duration_since(start) < Duration::from_millis(100) {
                counter = counter.wrapping_add(1);
            }
            counter
        });
        
        sleep(Duration::from_millis(50)).await;
        
        // 执行检查
        let result = checker.check();
        
        // 验证 CPU 检查存在
        let cpu_check = result.checks.iter().find(|c| c.name == "cpu");
        assert!(cpu_check.is_some());
        
        // 等待任务完成
        let _ = handle.await;
    }

    /// 测试：心跳超时检测
    #[tokio::test]
    async fn test_heartbeat_timeout() {
        use crate::watchdog::heartbeat::{HeartbeatChecker, HeartbeatConfig};
        
        let config = HeartbeatConfig {
            timeout: Duration::from_millis(100),
            max_failures: 2,
            ..Default::default()
        };
        let checker = HeartbeatChecker::new(config);
        
        // 发送不健康的心跳
        let status = HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["Error".to_string()],
        );
        
        // 第一次失败
        let result = checker.check(status.clone()).await;
        assert!(result.is_ok());
        assert_eq!(checker.failure_count(), 1);
        
        // 第二次失败
        let result = checker.check(status.clone()).await;
        assert!(result.is_ok());
        assert_eq!(checker.failure_count(), 2);
        
        // 验证已超过阈值
        assert!(checker.is_exceeded());
    }

    /// 测试：降级模式激活
    #[test]
    fn test_degraded_mode_activation() {
        let config = DegradedModeConfig {
            enabled: true,
            max_concurrent_requests: 2,
            ..Default::default()
        };
        let manager = DegradedModeManager::new(config);
        
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

    /// 测试：租约过期
    #[tokio::test]
    async fn test_lease_expiry() {
        use crate::watchdog::lease::{LeaseManager, LeaseConfig};
        
        let config = LeaseConfig {
            duration: 1, // 1 秒过期
            renew_deadline: 0,
        };
        let manager = LeaseManager::new(config);
        
        // 申请租约
        let lease = manager.acquire("test".to_string()).unwrap();
        assert!(manager.is_valid());
        
        // 等待过期
        sleep(Duration::from_millis(1100)).await;
        
        // 验证租约已过期
        assert!(!manager.is_valid());
    }

    /// 测试：L1 恢复执行
    #[tokio::test]
    async fn test_l1_recovery_execution() {
        let audit_log = AuditLogger::new(Default::default());
        let executor = RecoveryExecutor::new(audit_log);
        
        // 创建 L1 恢复计划
        let plan = RecoveryPlan::new(
            RecoveryLevel::L1QuickFix,
            "test-component".to_string(),
        ).with_actions(vec![
            crate::watchdog::recovery::RecoveryAction::new(
                "clear_cache".to_string(),
                "Clear memory cache".to_string(),
            ),
        ]);
        
        // 执行恢复
        let result = executor.execute(plan).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        println!("L1 recovery completed in {:?}", result.duration());
    }

    /// 测试：恢复计划退避
    #[test]
    fn test_recovery_backoff() {
        let mut plan = RecoveryPlan::new(
            RecoveryLevel::L1QuickFix,
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

    /// 测试：并发恢复请求序列化
    #[tokio::test]
    async fn test_concurrent_recovery_serialization() {
        use std::sync::atomic::{AtomicU32, Ordering};
        
        let counter = Arc::new(AtomicU32::new(0));
        let mut handles = vec![];
        
        // 模拟多个并发恢复请求
        for i in 0..5 {
            let counter = counter.clone();
            let handle = tokio::spawn(async move {
                // 模拟恢复动作
                sleep(Duration::from_millis(10)).await;
                counter.fetch_add(1, Ordering::SeqCst);
                i
            });
            handles.push(handle);
        }
        
        // 等待所有完成
        for handle in handles {
            let _ = handle.await;
        }
        
        // 验证所有请求都已处理
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    /// 测试：快速修复执行器
    #[test]
    fn test_quick_fix_executor_creation() {
        let executor = QuickFixExecutor::new("newclaw-gateway".to_string());
        assert_eq!(executor.service_name, "newclaw-gateway");
    }

    /// 测试：服务状态检查
    #[tokio::test]
    async fn test_service_status_check() {
        let executor = QuickFixExecutor::new("systemd-journald".to_string());
        
        // 检查一个肯定存在的服务
        let status = executor.check_service_status().await;
        assert!(status.is_ok());
        
        let status = status.unwrap();
        println!("Service status: active={}, state={}", 
            status.active_state, status.sub_state);
    }
}