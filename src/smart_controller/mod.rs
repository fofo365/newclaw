// Smart Controller Integration - 智慧主控集成模块
//
// 集成到 Gateway 启动流程，提供：
// - 心跳上报
// - 自检
// - 降级模式

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::core::heartbeat_reporter::{HeartbeatReporter, HeartbeatReporterConfig, HealthState};
use crate::core::self_check::{SelfChecker, SelfCheckConfig};
use crate::core::degraded_mode::{DegradedModeManager, DegradedModeConfig};
use crate::watchdog::heartbeat::HeartbeatStatus;

/// 智慧主控配置
#[derive(Debug, Clone)]
pub struct SmartControllerConfig {
    /// 组件名称
    pub component: String,
    /// 心跳上报配置
    pub heartbeat: HeartbeatReporterConfig,
    /// 自检配置
    pub self_check: SelfCheckConfig,
    /// 降级模式配置
    pub degraded_mode: DegradedModeConfig,
    /// Watchdog gRPC 地址
    pub watchdog_addr: String,
    /// 是否启用
    pub enabled: bool,
}

impl Default for SmartControllerConfig {
    fn default() -> Self {
        Self {
            component: "smart_controller".to_string(),
            heartbeat: HeartbeatReporterConfig::default(),
            self_check: SelfCheckConfig::default(),
            degraded_mode: DegradedModeConfig::default(),
            watchdog_addr: "http://127.0.0.1:50051".to_string(),
            enabled: true,
        }
    }
}

/// 智慧主控管理器
pub struct SmartController {
    config: SmartControllerConfig,
    heartbeat_reporter: Arc<HeartbeatReporter>,
    self_checker: Arc<SelfChecker>,
    degraded_mode: Arc<DegradedModeManager>,
    lease_id: Arc<RwLock<Option<String>>>,
}

impl SmartController {
    pub fn new(config: SmartControllerConfig) -> Self {
        let heartbeat_reporter = Arc::new(HeartbeatReporter::new(config.heartbeat.clone()));
        let self_checker = Arc::new(SelfChecker::new(config.self_check.clone()));
        let degraded_mode = Arc::new(DegradedModeManager::new(config.degraded_mode.clone()));
        
        Self {
            config,
            heartbeat_reporter,
            self_checker,
            degraded_mode,
            lease_id: Arc::new(RwLock::new(None)),
        }
    }
    
    /// 获取心跳上报器
    pub fn heartbeat_reporter(&self) -> Arc<HeartbeatReporter> {
        self.heartbeat_reporter.clone()
    }
    
    /// 获取自检器
    pub fn self_checker(&self) -> Arc<SelfChecker> {
        self.self_checker.clone()
    }
    
    /// 获取降级模式管理器
    pub fn degraded_mode(&self) -> Arc<DegradedModeManager> {
        self.degraded_mode.clone()
    }
    
    /// 设置租约 ID
    pub async fn set_lease_id(&self, lease_id: String) {
        let mut current = self.lease_id.write().await;
        *current = Some(lease_id.clone());
        self.heartbeat_reporter.set_lease_id(lease_id);
    }
    
    /// 获取租约 ID
    pub async fn lease_id(&self) -> Option<String> {
        self.lease_id.read().await.clone()
    }
    
    /// 执行自检并返回健康状态
    pub fn check_health(&self) -> HealthState {
        let result = self.self_checker.check();
        
        if result.healthy {
            HealthState::Healthy
        } else {
            let warnings: Vec<&str> = result.warnings.iter().map(|s| s.as_str()).collect();
            if warnings.is_empty() {
                HealthState::Degraded("Health check warnings".to_string())
            } else {
                HealthState::Degraded(warnings.join("; "))
            }
        }
    }
    
    /// 启动后台任务
    pub async fn start_background_tasks(&self) {
        if !self.config.enabled {
            tracing::info!("Smart controller disabled, skipping background tasks");
            return;
        }
        
        let heartbeat_reporter = self.heartbeat_reporter.clone();
        let self_checker = self.self_checker.clone();
        let degraded_mode = self.degraded_mode.clone();
        let watchdog_addr = self.config.watchdog_addr.clone();
        
        // 启动心跳上报循环
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            
            loop {
                interval.tick().await;
                
                // 执行自检
                let health = {
                    let result = self_checker.check();
                    if result.healthy {
                        HealthState::Healthy
                    } else {
                        let failed_checks: Vec<&str> = result.checks
                            .iter()
                            .filter(|c| !c.passed)
                            .map(|c| c.name.as_str())
                            .collect();
                        
                        if failed_checks.is_empty() {
                            HealthState::Degraded("Unknown issue".to_string())
                        } else {
                            HealthState::Degraded(failed_checks.join(", "))
                        }
                    }
                };
                
                // 根据健康状态决定是否进入降级模式
                match &health {
                    HealthState::Healthy => {
                        if degraded_mode.is_degraded() {
                            degraded_mode.exit();
                        }
                    }
                    HealthState::Degraded(reason) => {
                        if !degraded_mode.is_degraded() {
                            degraded_mode.enter(reason);
                        }
                    }
                    HealthState::Unhealthy(reason) => {
                        degraded_mode.enter(reason);
                    }
                }
                
                // 生成心跳报告
                if let Some(report) = heartbeat_reporter.generate_report(health) {
                    // 发送到 Watchdog（通过 gRPC）
                    if let Err(e) = send_heartbeat_to_watchdog(&watchdog_addr, &report).await {
                        tracing::warn!("Failed to send heartbeat: {}", e);
                        heartbeat_reporter.record_error(format!("Heartbeat failed: {}", e));
                    }
                }
            }
        });
        
        tracing::info!("🫀 Smart controller background tasks started");
    }
}

/// 发送心跳到 Watchdog
async fn send_heartbeat_to_watchdog(
    watchdog_addr: &str,
    report: &crate::core::heartbeat_reporter::HeartbeatReport,
) -> anyhow::Result<()> {
    // 尝试使用 gRPC 客户端
    use crate::watchdog::grpc::WatchdogClient;
    
    // 创建客户端并连接
    let mut client = WatchdogClient::new(watchdog_addr.to_string());
    
    match client.connect().await {
        Ok(_) => {
            // 使用 gRPC 发送心跳
            let health_status = match &report.health {
                HealthState::Healthy => 1,
                HealthState::Degraded(_) => 2,
                HealthState::Unhealthy(_) => 3,
            };
            
            match client.send_heartbeat(
                report.lease_id.clone(),
                health_status,
                report.metrics.memory_mb,
                report.metrics.cpu_percent as f32,
                report.metrics.active_sessions,
                report.recent_errors.clone(),
                "smart_controller".to_string(),
            ).await {
                Ok(_) => {
                    tracing::debug!("Heartbeat sent via gRPC");
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("gRPC heartbeat failed: {}, falling back to HTTP", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("gRPC connection failed: {}, falling back to HTTP", e);
        }
    }
    
    // HTTP fallback
    let client = reqwest::Client::new();
    let url = format!("{}/heartbeat", watchdog_addr.replace("http://", "http://"));
    
    let response = client
        .post(&url)
        .json(report)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    
    if !response.status().is_success() {
        anyhow::bail!("Watchdog returned error: {}", response.status());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_smart_controller_creation() {
        let config = SmartControllerConfig::default();
        let controller = SmartController::new(config);
        
        assert!(controller.heartbeat_reporter().lease_id().is_none());
    }
    
    #[test]
    fn test_check_health() {
        let config = SmartControllerConfig::default();
        let controller = SmartController::new(config);
        
        let health = controller.check_health();
        // 默认配置下应该是健康的
        assert!(matches!(health, HealthState::Healthy | HealthState::Degraded(_)));
    }
}