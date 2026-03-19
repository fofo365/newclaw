// Dashboard 指标收集器
//
// 收集和存储系统性能指标

use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::time::Instant;

/// 系统指标
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub uptime_secs: u64,
    pub start_time: DateTime<Utc>,
    
    // 请求指标
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    
    // Token 指标
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub tokens_per_minute: f64,
    
    // 连接指标
    pub feishu_connected: bool,
    pub llm_available: bool,
    pub active_sessions: u64,
    pub active_websockets: u64,
    
    // 错误指标
    pub total_errors: u64,
    pub error_rate: f64,
    pub last_error: Option<String>,
    pub last_error_time: Option<DateTime<Utc>>,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            uptime_secs: 0,
            start_time: Utc::now(),
            requests_total: 0,
            requests_successful: 0,
            requests_failed: 0,
            avg_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            tokens_input: 0,
            tokens_output: 0,
            tokens_per_minute: 0.0,
            feishu_connected: false,
            llm_available: true,
            active_sessions: 0,
            active_websockets: 0,
            total_errors: 0,
            error_rate: 0.0,
            last_error: None,
            last_error_time: None,
        }
    }
}

/// 延迟记录
#[derive(Debug, Clone)]
struct LatencyRecord {
    timestamp: DateTime<Utc>,
    latency_ms: f64,
}

/// 指标收集器
pub struct MetricsCollector {
    inner: RwLock<MetricsInner>,
    start_time: Instant,
}

struct MetricsInner {
    metrics: SystemMetrics,
    latencies: Vec<LatencyRecord>,
    hourly_tokens: Vec<(DateTime<Utc>, u64)>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(MetricsInner {
                metrics: SystemMetrics::default(),
                latencies: Vec::with_capacity(10000),
                hourly_tokens: Vec::new(),
            }),
            start_time: Instant::now(),
        }
    }
    
    /// 获取当前指标
    pub async fn get_metrics(&self) -> SystemMetrics {
        let mut inner = self.inner.write().await;
        
        // 更新 uptime
        inner.metrics.uptime_secs = self.start_time.elapsed().as_secs();
        
        // 计算百分位延迟
        if !inner.latencies.is_empty() {
            let mut latencies: Vec<f64> = inner.latencies.iter().map(|l| l.latency_ms).collect();
            latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            let len = latencies.len();
            inner.metrics.avg_latency_ms = latencies.iter().sum::<f64>() / len as f64;
            inner.metrics.p50_latency_ms = latencies[len / 2];
            inner.metrics.p95_latency_ms = latencies[(len as f64 * 0.95) as usize];
            inner.metrics.p99_latency_ms = latencies[(len as f64 * 0.99) as usize];
        }
        
        // 计算 tokens per minute
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let recent_tokens: u64 = inner.hourly_tokens
            .iter()
            .filter(|(t, _)| *t > one_hour_ago)
            .map(|(_, tokens)| *tokens)
            .sum();
        inner.metrics.tokens_per_minute = recent_tokens as f64 / 60.0;
        
        // 计算错误率
        if inner.metrics.requests_total > 0 {
            inner.metrics.error_rate = inner.metrics.requests_failed as f64 / inner.metrics.requests_total as f64;
        }
        
        inner.metrics.clone()
    }
    
    /// 记录请求
    pub async fn record_request(&self, success: bool, latency_ms: f64) {
        let mut inner = self.inner.write().await;
        
        inner.metrics.requests_total += 1;
        if success {
            inner.metrics.requests_successful += 1;
        } else {
            inner.metrics.requests_failed += 1;
        }
        
        // 记录延迟
        inner.latencies.push(LatencyRecord {
            timestamp: Utc::now(),
            latency_ms,
        });
        
        // 保留最近 10000 条延迟记录
        if inner.latencies.len() > 10000 {
            inner.latencies.remove(0);
        }
    }
    
    /// 记录 Token 使用
    pub async fn record_tokens(&self, input: u64, output: u64) {
        let mut inner = self.inner.write().await;
        
        inner.metrics.tokens_input += input;
        inner.metrics.tokens_output += output;
        
        // 记录每小时使用量
        let now = Utc::now();
        let hour_key = now.format("%Y-%m-%d %H:00").to_string();
        inner.hourly_tokens.push((now, input + output));
        
        // 保留最近 24 小时的记录
        let one_day_ago = now - chrono::Duration::hours(24);
        inner.hourly_tokens.retain(|(t, _)| *t > one_day_ago);
    }
    
    /// 记录错误
    pub async fn record_error(&self, error: String) {
        let mut inner = self.inner.write().await;
        
        inner.metrics.total_errors += 1;
        inner.metrics.last_error = Some(error);
        inner.metrics.last_error_time = Some(Utc::now());
    }
    
    /// 更新连接状态
    pub async fn update_connection_status(&self, feishu_connected: bool, llm_available: bool) {
        let mut inner = self.inner.write().await;
        inner.metrics.feishu_connected = feishu_connected;
        inner.metrics.llm_available = llm_available;
    }
    
    /// 更新活跃会话数
    pub async fn update_active_sessions(&self, count: u64) {
        let mut inner = self.inner.write().await;
        inner.metrics.active_sessions = count;
    }
    
    /// 更新活跃 WebSocket 数
    pub async fn update_active_websockets(&self, count: u64) {
        let mut inner = self.inner.write().await;
        inner.metrics.active_websockets = count;
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
