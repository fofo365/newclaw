// 心跳上报器 - 智慧主控向核心主控上报状态

use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::interval;

use crate::metrics::resources::ResourceMonitor;

/// 心跳上报配置
#[derive(Debug, Clone)]
pub struct HeartbeatReporterConfig {
    /// 上报间隔（秒）
    pub interval_secs: u64,
    /// 组件名称
    pub component: String,
    /// 是否启用
    pub enabled: bool,
}

impl Default for HeartbeatReporterConfig {
    fn default() -> Self {
        Self {
            interval_secs: 3,
            component: "smart_controller".to_string(),
            enabled: true,
        }
    }
}

/// 心跳上报消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatReport {
    /// 租约 ID
    pub lease_id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 健康状态
    pub health: HealthState,
    /// 系统指标
    pub metrics: MetricsReport,
    /// 最近错误
    pub recent_errors: Vec<String>,
    /// 组件名称
    pub component: String,
}

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthState {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

impl HealthState {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy)
    }
}

/// 指标报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReport {
    pub memory_mb: u64,
    pub cpu_percent: f64,
    pub active_sessions: u64,
    pub request_rate: u64,
    pub error_rate: u64,
    pub uptime_secs: u64,
}

impl Default for MetricsReport {
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

/// 心跳上报器
pub struct HeartbeatReporter {
    config: HeartbeatReporterConfig,
    resource_monitor: Arc<ResourceMonitor>,
    lease_id: Arc<std::sync::RwLock<Option<String>>>,
    started_at: DateTime<Utc>,
    recent_errors: Arc<std::sync::RwLock<Vec<String>>>,
    active_sessions: Arc<std::sync::atomic::AtomicU64>,
    request_count: Arc<std::sync::atomic::AtomicU64>,
    error_count: Arc<std::sync::atomic::AtomicU64>,
}

impl HeartbeatReporter {
    pub fn new(config: HeartbeatReporterConfig) -> Self {
        Self {
            resource_monitor: Arc::new(ResourceMonitor::new()),
            lease_id: Arc::new(std::sync::RwLock::new(None)),
            started_at: Utc::now(),
            recent_errors: Arc::new(std::sync::RwLock::new(Vec::new())),
            active_sessions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            error_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            config,
        }
    }
    
    /// 设置租约 ID
    pub fn set_lease_id(&self, lease_id: String) {
        let mut current = self.lease_id.write().unwrap();
        *current = Some(lease_id);
    }
    
    /// 获取租约 ID
    pub fn lease_id(&self) -> Option<String> {
        self.lease_id.read().unwrap().clone()
    }
    
    /// 记录错误
    pub fn record_error(&self, error: String) {
        let mut errors = self.recent_errors.write().unwrap();
        errors.push(error);
        // 保留最近 10 个错误
        if errors.len() > 10 {
            errors.remove(0);
        }
        self.error_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// 增加活跃会话
    pub fn increment_sessions(&self) {
        self.active_sessions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// 减少活跃会话
    pub fn decrement_sessions(&self) {
        self.active_sessions.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// 增加请求计数
    pub fn increment_requests(&self) {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    
    /// 收集指标
    fn collect_metrics(&self) -> MetricsReport {
        let metrics = self.resource_monitor.get_metrics();
        
        MetricsReport {
            memory_mb: metrics.memory_used_mb,
            cpu_percent: metrics.cpu_usage_percent,
            active_sessions: self.active_sessions.load(std::sync::atomic::Ordering::SeqCst),
            request_rate: self.request_count.load(std::sync::atomic::Ordering::SeqCst),
            error_rate: self.error_count.load(std::sync::atomic::Ordering::SeqCst),
            uptime_secs: (Utc::now() - self.started_at).num_seconds() as u64,
        }
    }
    
    /// 生成心跳报告
    pub fn generate_report(&self, health: HealthState) -> Option<HeartbeatReport> {
        let lease_id = self.lease_id.read().unwrap().clone()?;
        
        let errors = self.recent_errors.read().unwrap().clone();
        
        Some(HeartbeatReport {
            lease_id,
            timestamp: Utc::now(),
            health,
            metrics: self.collect_metrics(),
            recent_errors: errors,
            component: self.config.component.clone(),
        })
    }
    
    /// 启动心跳上报循环（需要 gRPC 客户端）
    pub async fn start<F, Fut>(&self, mut send_fn: F)
    where
        F: FnMut(HeartbeatReport) -> Fut + Send,
        Fut: std::future::Future<Output = anyhow::Result<()>> + Send,
    {
        if !self.config.enabled {
            return;
        }
        
        let mut interval = interval(Duration::from_secs(self.config.interval_secs));
        
        loop {
            interval.tick().await;
            
            let report = self.generate_report(HealthState::Healthy);
            
            if let Some(report) = report {
                if let Err(e) = send_fn(report).await {
                    tracing::error!("Heartbeat report failed: {}", e);
                    self.record_error(format!("Heartbeat failed: {}", e));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_heartbeat_reporter_creation() {
        let config = HeartbeatReporterConfig::default();
        let reporter = HeartbeatReporter::new(config);
        
        assert!(reporter.lease_id().is_none());
    }
    
    #[test]
    fn test_set_lease_id() {
        let reporter = HeartbeatReporter::new(HeartbeatReporterConfig::default());
        reporter.set_lease_id("lease-123".to_string());
        
        assert_eq!(reporter.lease_id(), Some("lease-123".to_string()));
    }
    
    #[test]
    fn test_record_error() {
        let reporter = HeartbeatReporter::new(HeartbeatReporterConfig::default());
        reporter.record_error("Error 1".to_string());
        reporter.record_error("Error 2".to_string());
        
        let errors = reporter.recent_errors.read().unwrap();
        assert_eq!(errors.len(), 2);
    }
    
    #[test]
    fn test_generate_report() {
        let reporter = HeartbeatReporter::new(HeartbeatReporterConfig::default());
        reporter.set_lease_id("lease-123".to_string());
        
        let report = reporter.generate_report(HealthState::Healthy);
        assert!(report.is_some());
        
        let report = report.unwrap();
        assert_eq!(report.lease_id, "lease-123");
        assert!(report.health.is_healthy());
    }
    
    #[test]
    fn test_generate_report_no_lease() {
        let reporter = HeartbeatReporter::new(HeartbeatReporterConfig::default());
        
        let report = reporter.generate_report(HealthState::Healthy);
        assert!(report.is_none());
    }
    
    #[test]
    fn test_session_tracking() {
        let reporter = HeartbeatReporter::new(HeartbeatReporterConfig::default());
        
        reporter.increment_sessions();
        reporter.increment_sessions();
        assert_eq!(
            reporter.active_sessions.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
        
        reporter.decrement_sessions();
        assert_eq!(
            reporter.active_sessions.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }
}
