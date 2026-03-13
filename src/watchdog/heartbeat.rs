// 心跳检测模块

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::config::WatchdogConfig;

/// 心跳状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatStatus {
    /// 租约 ID
    pub lease_id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 健康状态
    pub health: HealthStatus,
    /// 系统指标
    pub metrics: SystemMetrics,
    /// 最近错误
    pub recent_errors: Vec<String>,
    /// 组件名称
    pub component: String,
}

impl HeartbeatStatus {
    pub fn healthy(lease_id: String, component: String) -> Self {
        Self {
            lease_id,
            timestamp: Utc::now(),
            health: HealthStatus::Healthy,
            metrics: SystemMetrics::default(),
            recent_errors: vec![],
            component,
        }
    }
    
    pub fn degraded(lease_id: String, component: String, message: String) -> Self {
        Self {
            lease_id,
            timestamp: Utc::now(),
            health: HealthStatus::Degraded(message.clone()),
            metrics: SystemMetrics::default(),
            recent_errors: vec![message],
            component,
        }
    }
    
    pub fn unhealthy(lease_id: String, component: String, errors: Vec<String>) -> Self {
        Self {
            lease_id,
            timestamp: Utc::now(),
            health: HealthStatus::Unhealthy(errors.join("; ")),
            metrics: SystemMetrics::default(),
            recent_errors: errors,
            component,
        }
    }
    
    pub fn is_healthy(&self) -> bool {
        matches!(self.health, HealthStatus::Healthy | HealthStatus::Degraded(_))
    }
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }
    
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::Degraded(_))
    }
    
    pub fn is_unhealthy(&self) -> bool {
        matches!(self, Self::Unhealthy(_))
    }
}

/// 系统指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// 内存使用（MB）
    pub memory_mb: u64,
    /// CPU 使用率（%）
    pub cpu_percent: f64,
    /// 活跃会话数
    pub active_sessions: u64,
    /// 请求速率
    pub request_rate: u64,
    /// 错误率（0-100）
    pub error_rate: u64,
    /// 运行时间（秒）
    pub uptime_secs: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            memory_mb: 0,
            cpu_percent: 0.0,
            active_sessions: 0,
            request_rate: 0,
            error_rate: 0,
            uptime_secs: 0,
        }
    }
}

/// 心跳检测配置
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// 检测间隔
    pub check_interval: Duration,
    /// 超时时间
    pub timeout: Duration,
    /// 最大失败次数
    pub max_failures: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(5),
            timeout: Duration::from_secs(15),
            max_failures: 3,
        }
    }
}

impl From<&WatchdogConfig> for HeartbeatConfig {
    fn from(config: &WatchdogConfig) -> Self {
        Self {
            check_interval: Duration::from_secs(config.check_interval),
            timeout: Duration::from_secs(config.heartbeat_timeout),
            max_failures: config.max_heartbeat_failures,
        }
    }
}

/// 心跳检测器
pub struct HeartbeatChecker {
    config: HeartbeatConfig,
    failure_count: Arc<AtomicU32>,
    last_status: Arc<std::sync::RwLock<Option<HeartbeatStatus>>>,
}

impl HeartbeatChecker {
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            failure_count: Arc::new(AtomicU32::new(0)),
            last_status: Arc::new(std::sync::RwLock::new(None)),
        }
    }
    
    /// 检查心跳（从 gRPC 获取状态）
    pub async fn check(&self, status: HeartbeatStatus) -> anyhow::Result<HeartbeatStatus> {
        // 更新最后状态
        {
            match self.last_status.write() {
                Ok(mut last) => *last = Some(status.clone()),
                Err(e) => tracing::error!("Failed to acquire write lock: {}", e),
            }
        }
        
        if status.is_healthy() {
            self.failure_count.store(0, Ordering::SeqCst);
            Ok(status)
        } else {
            let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
            if failures >= self.config.max_failures {
                Err(anyhow::anyhow!("Heartbeat failed {} times", failures))
            } else {
                Ok(status)
            }
        }
    }
    
    /// 记录失败
    pub fn record_failure(&self) -> u32 {
        self.failure_count.fetch_add(1, Ordering::SeqCst) + 1
    }
    
    /// 重置失败计数
    pub fn reset_failures(&self) {
        self.failure_count.store(0, Ordering::SeqCst);
    }
    
    /// 获取失败次数
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }
    
    /// 是否超过最大失败次数
    pub fn is_exceeded(&self) -> bool {
        self.failure_count.load(Ordering::SeqCst) >= self.config.max_failures
    }
    
    /// 获取最后状态
    pub fn last_status(&self) -> Option<HeartbeatStatus> {
        match self.last_status.read() {
            Ok(status) => status.clone(),
            Err(e) => {
                tracing::error!("Failed to acquire read lock: {}", e);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_heartbeat_status_healthy() {
        let status = HeartbeatStatus::healthy("lease-123".to_string(), "smart".to_string());
        assert!(status.is_healthy());
        assert!(status.health.is_healthy());
    }
    
    #[test]
    fn test_heartbeat_status_degraded() {
        let status = HeartbeatStatus::degraded(
            "lease-123".to_string(),
            "smart".to_string(),
            "High memory".to_string(),
        );
        assert!(status.is_healthy()); // degraded still counts as healthy
        assert!(status.health.is_degraded());
    }
    
    #[test]
    fn test_heartbeat_status_unhealthy() {
        let status = HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["OOM".to_string(), "Crash".to_string()],
        );
        assert!(!status.is_healthy());
        assert!(status.health.is_unhealthy());
    }
    
    #[tokio::test]
    async fn test_heartbeat_checker_success() {
        let config = HeartbeatConfig::default();
        let checker = HeartbeatChecker::new(config);
        
        let status = HeartbeatStatus::healthy("lease-123".to_string(), "smart".to_string());
        let result = checker.check(status).await;
        
        assert!(result.is_ok());
        assert_eq!(checker.failure_count(), 0);
    }
    
    #[tokio::test]
    async fn test_heartbeat_checker_failures() {
        let config = HeartbeatConfig {
            max_failures: 2,
            ..Default::default()
        };
        let checker = HeartbeatChecker::new(config);
        
        // 第一次失败
        let status = HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["Error".to_string()],
        );
        let result = checker.check(status.clone()).await;
        assert!(result.is_ok()); // 还没超过阈值
        
        // 第二次失败
        let result = checker.check(status).await;
        assert!(result.is_err()); // 超过阈值
    }
}
