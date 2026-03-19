// 核心控制器模块

use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use super::audit::{AuditEvent, AuditLogger, EventType};
use super::config::WatchdogConfig;
use super::diagnostic::DiagnosticEngine;
use super::heartbeat::{HeartbeatChecker, HeartbeatStatus};
use super::lease::LeaseManager;
use super::recovery::{RecoveryExecutor, RecoveryLevel, RecoveryPlan};

/// 核心控制器
pub struct CoreController {
    config: WatchdogConfig,
    lease_manager: Arc<LeaseManager>,
    heartbeat_checker: HeartbeatChecker,
    diagnostic_engine: DiagnosticEngine,
    recovery_executor: RecoveryExecutor,
    audit_log: AuditLogger,
}

impl Clone for CoreController {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            lease_manager: self.lease_manager.clone(),
            heartbeat_checker: HeartbeatChecker::new((&self.config).into()),
            diagnostic_engine: DiagnosticEngine::new(),
            recovery_executor: RecoveryExecutor::new(AuditLogger::new(self.config.audit.clone())),
            audit_log: AuditLogger::new(self.config.audit.clone()),
        }
    }
}

impl CoreController {
    pub fn new(config: WatchdogConfig) -> Self {
        let lease_manager = Arc::new(LeaseManager::new(config.lease.clone()));
        let audit_log = AuditLogger::new(config.audit.clone());
        let recovery_executor = RecoveryExecutor::new(AuditLogger::new(config.audit.clone()));
        
        Self {
            heartbeat_checker: HeartbeatChecker::new((&config).into()),
            diagnostic_engine: DiagnosticEngine::new(),
            recovery_executor,
            lease_manager,
            audit_log,
            config,
        }
    }
    
    /// 主循环
    pub async fn run(&mut self) -> anyhow::Result<()> {
        let mut check_interval = interval(Duration::from_secs(self.config.check_interval));
        
        self.audit_log.log(AuditEvent::new(
            EventType::SafeModeExited,
            "watchdog".to_string(),
            "Watchdog started".to_string(),
        ))?;
        
        loop {
            check_interval.tick().await;
            
            // 1. 检查租约有效性
            if !self.lease_manager.is_valid() {
                tracing::warn!("Lease invalid, waiting...");
                continue;
            }
            
            // 2. 检查心跳
            if let Some(status) = self.heartbeat_checker.last_status() {
                if self.heartbeat_checker.is_exceeded() {
                    // 心跳失败超过阈值，触发恢复
                    self.trigger_recovery(status).await?;
                }
            }
            
            // 3. 清理过期租约
            if let Ok(cleaned) = self.lease_manager.cleanup_expired() {
                if cleaned > 0 {
                    tracing::info!("Cleaned {} expired leases", cleaned);
                }
            }
        }
    }
    
    /// 处理心跳
    pub async fn handle_heartbeat(&mut self, status: HeartbeatStatus) -> anyhow::Result<bool> {
        let lease_id = status.lease_id.clone();
        let component = status.component.clone();
        
        // 验证租约
        if !self.lease_manager.is_valid() {
            self.audit_log.log_heartbeat(&component, false, "Invalid lease".to_string())?;
            return Ok(false);
        }
        
        // 检查心跳
        match self.heartbeat_checker.check(status.clone()).await {
            Ok(status) => {
                if !status.is_healthy() {
                    self.audit_log.log_heartbeat(&component, false, "Unhealthy status".to_string())?;
                    
                    // 如果超过阈值，触发恢复
                    if self.heartbeat_checker.is_exceeded() {
                        self.trigger_recovery(status).await?;
                    }
                } else {
                    self.audit_log.log_heartbeat(&component, true, "OK".to_string())?;
                }
                Ok(true)
            }
            Err(e) => {
                self.audit_log.log_heartbeat(&component, false, e.to_string())?;
                Err(e)
            }
        }
    }
    
    /// 触发恢复
    async fn trigger_recovery(&mut self, status: HeartbeatStatus) -> anyhow::Result<()> {
        tracing::warn!("Triggering recovery for {}", status.component);
        
        // 记录恢复触发
        self.audit_log.log(AuditEvent::new(
            EventType::RecoveryTriggered,
            status.component.clone(),
            format!("Status: {:?}", status.health),
        ).with_lease(status.lease_id.clone()))?;
        
        // 分析故障
        let diagnostic = self.diagnostic_engine.analyze(&status).await?;
        
        // 确定恢复级别
        let level = diagnostic.suggested_level;
        
        // 生成恢复计划
        let plan = self.generate_recovery_plan(&status, level, diagnostic);
        
        // 执行恢复
        let result = self.recovery_executor.execute(plan).await?;
        
        if result.success {
            // 重置心跳计数
            self.heartbeat_checker.reset_failures();
            tracing::info!("Recovery succeeded in {:?}", result.duration());
        } else {
            tracing::error!("Recovery failed: {}", result.message);
            
            // 尝试升级恢复级别
            if level != RecoveryLevel::L3HumanIntervention {
                self.escalate_recovery(&status, level).await?;
            }
        }
        
        Ok(())
    }
    
    /// 生成恢复计划
    fn generate_recovery_plan(
        &self,
        status: &HeartbeatStatus,
        level: RecoveryLevel,
        diagnostic: super::diagnostic::DiagnosticResult,
    ) -> RecoveryPlan {
        match level {
            RecoveryLevel::L1QuickFix => {
                let actions = self.get_l1_actions(&diagnostic);
                RecoveryExecutor::generate_l1_plan(status.component.clone(), actions)
                    .with_diagnostic(diagnostic)
            }
            RecoveryLevel::L2AiDiagnosis => {
                RecoveryExecutor::generate_l2_plan(status.component.clone(), diagnostic)
            }
            RecoveryLevel::L3HumanIntervention => {
                RecoveryExecutor::generate_l3_plan(status.component.clone())
            }
        }
    }
    
    /// 获取 L1 动作
    fn get_l1_actions(&self, diagnostic: &super::diagnostic::DiagnosticResult) -> Vec<String> {
        let mut actions = Vec::new();
        
        for cause in &diagnostic.root_causes {
            for suggestion in &cause.suggestions {
                match suggestion.as_str() {
                    "重启服务" => {
                        if !actions.contains(&"restart_service".to_string()) {
                            actions.push("restart_service".to_string());
                        }
                    }
                    "清理缓存" => {
                        if !actions.contains(&"clear_cache".to_string()) {
                            actions.push("clear_cache".to_string());
                        }
                    }
                    "回滚配置" => {
                        if !actions.contains(&"rollback_config".to_string()) {
                            actions.push("rollback_config".to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // 默认动作
        if actions.is_empty() {
            actions.push("clear_cache".to_string());
        }
        
        actions
    }
    
    /// 升级恢复级别
    async fn escalate_recovery(
        &mut self,
        status: &HeartbeatStatus,
        current_level: RecoveryLevel,
    ) -> anyhow::Result<()> {
        let next_level = match current_level {
            RecoveryLevel::L1QuickFix => RecoveryLevel::L2AiDiagnosis,
            RecoveryLevel::L2AiDiagnosis => RecoveryLevel::L3HumanIntervention,
            RecoveryLevel::L3HumanIntervention => return Ok(()),
        };
        
        tracing::warn!("Escalating recovery from {:?} to {:?}", current_level, next_level);
        
        let diagnostic = self.diagnostic_engine.analyze(status).await?;
        let plan = self.generate_recovery_plan(status, next_level, diagnostic);
        
        self.recovery_executor.execute(plan).await?;
        
        Ok(())
    }
    
    /// 获取租约管理器（用于 gRPC 服务）
    pub fn lease_manager(&self) -> Arc<LeaseManager> {
        self.lease_manager.clone()
    }
    
    /// 获取审计日志（用于 gRPC 服务）
    pub fn audit_log(&self) -> &AuditLogger {
        &self.audit_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watchdog::config::LeaseConfig;
    
    #[test]
    fn test_core_controller_creation() {
        let config = WatchdogConfig::default();
        let controller = CoreController::new(config);
        
        assert!(!controller.lease_manager.is_valid());
    }
    
    #[tokio::test]
    async fn test_core_controller_handle_heartbeat() {
        let config = WatchdogConfig::default();
        let mut controller = CoreController::new(config);
        
        // 先获取租约
        controller.lease_manager.acquire("test".to_string()).unwrap();
        
        let status = HeartbeatStatus::healthy("lease-123".to_string(), "smart".to_string());
        let result = controller.handle_heartbeat(status).await;
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_get_l1_actions() {
        let config = WatchdogConfig::default();
        let controller = CoreController::new(config);
        
        let diagnostic = super::super::diagnostic::DiagnosticResult::new();
        let actions = controller.get_l1_actions(&diagnostic);
        
        assert!(!actions.is_empty());
    }
}
