// Metrics - v0.5.5
//
// Prometheus 指标导出 + 资源监控

pub mod registry;
pub mod prometheus;
pub mod resources;

// Re-exports
pub use registry::{MetricsRegistry, Metric, MetricType, DefaultMetrics};
pub use prometheus::*;
pub use resources::ResourceMonitor;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 系统指标（Dashboard 使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub uptime_secs: u64,
    pub start_time: DateTime<Utc>,
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub active_sessions: u64,
    pub cpu_usage_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            uptime_secs: 0,
            start_time: Utc::now(),
            requests_total: 0,
            requests_successful: 0,
            requests_failed: 0,
            tokens_input: 0,
            tokens_output: 0,
            active_sessions: 0,
            cpu_usage_percent: 0.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
        }
    }
}

/// 指标收集器（简化版）
pub struct MetricsCollector {
    inner: std::sync::RwLock<SystemMetrics>,
    start_time: std::time::Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: std::sync::RwLock::new(SystemMetrics::default()),
            start_time: std::time::Instant::now(),
        }
    }
    
    pub async fn get_metrics(&self) -> SystemMetrics {
        let mut inner = self.inner.write().unwrap();
        inner.uptime_secs = self.start_time.elapsed().as_secs();
        inner.clone()
    }
    
    pub async fn record_request(&self, success: bool, _latency_ms: f64) {
        let mut inner = self.inner.write().unwrap();
        inner.requests_total += 1;
        if success {
            inner.requests_successful += 1;
        } else {
            inner.requests_failed += 1;
        }
    }
    
    pub async fn record_tokens(&self, input: u64, output: u64) {
        let mut inner = self.inner.write().unwrap();
        inner.tokens_input += input;
        inner.tokens_output += output;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
