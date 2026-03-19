// Dashboard 监控 API
//
// 提供：
// 1. 日志查看（实时流、过滤、搜索）
// 2. 性能指标（请求时间、Token 使用）
// 3. 健康检查

use axum::{
    response::{IntoResponse, Response},
    Json,
};
use axum::extract::{State, Query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

// ============== 日志 ==============

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
    pub metadata: serde_json::Value,
}

/// 日志级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<&tracing::Level> for LogLevel {
    fn from(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        }
    }
}

/// 日志过滤参数
#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub level: Option<String>,
    pub source: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// 日志列表响应
#[derive(Debug, Serialize)]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
    pub total: usize,
    pub has_more: bool,
}

/// 获取日志列表
pub async fn get_logs(
    State(state): State<Arc<super::DashboardState>>,
    Query(filter): Query<LogFilter>,
) -> Json<LogsResponse> {
    let logs = state.logs.read().await;
    let limit = filter.limit.unwrap_or(100).min(1000);
    let offset = filter.offset.unwrap_or(0);
    
    let mut filtered: Vec<LogEntry> = logs
        .iter()
        .filter(|log| {
            // 级别过滤
            if let Some(ref level) = filter.level {
                let level_matches = match level.to_lowercase().as_str() {
                    "trace" => log.level == LogLevel::Trace,
                    "debug" => log.level == LogLevel::Debug,
                    "info" => log.level == LogLevel::Info,
                    "warn" => log.level == LogLevel::Warn,
                    "error" => log.level == LogLevel::Error,
                    _ => true,
                };
                if !level_matches {
                    return false;
                }
            }
            
            // 来源过滤
            if let Some(ref source) = filter.source {
                if !log.source.to_lowercase().contains(&source.to_lowercase()) {
                    return false;
                }
            }
            
            // 搜索过滤
            if let Some(ref search) = filter.search {
                if !log.message.to_lowercase().contains(&search.to_lowercase()) {
                    return false;
                }
            }
            
            // 时间范围过滤
            if let Some(start) = filter.start_time {
                if log.timestamp < start {
                    return false;
                }
            }
            if let Some(end) = filter.end_time {
                if log.timestamp > end {
                    return false;
                }
            }
            
            true
        })
        .cloned()
        .collect();
    
    // 按时间倒序排序
    filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    let total = filtered.len();
    let has_more = offset + limit < total;
    
    let logs: Vec<LogEntry> = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();
    
    Json(LogsResponse { logs, total, has_more })
}

/// WebSocket 日志流（占位符，需要启用 axum ws feature）
pub async fn stream_logs() -> &'static str {
    // TODO: 需要在 Cargo.toml 中启用 axum 的 ws feature
    // 目前返回占位符
    "WebSocket streaming requires ws feature to be enabled"
}

// ============== 性能指标 ==============

/// 指标响应
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub uptime_secs: u64,
    pub requests: RequestMetrics,
    pub tokens: TokenMetrics,
    pub connections: ConnectionMetrics,
    pub errors: ErrorMetrics,
}

/// 请求指标
#[derive(Debug, Serialize)]
pub struct RequestMetrics {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// Token 指标
#[derive(Debug, Serialize)]
pub struct TokenMetrics {
    pub total_input: u64,
    pub total_output: u64,
    pub total: u64,
    pub rate_per_minute: f64,
}

/// 连接指标
#[derive(Debug, Serialize)]
pub struct ConnectionMetrics {
    pub feishu_websocket: bool,
    pub active_sessions: u64,
    pub active_websockets: u64,
}

/// 错误指标
#[derive(Debug, Serialize)]
pub struct ErrorMetrics {
    pub total_errors: u64,
    pub error_rate: f64,
    pub last_error: Option<String>,
    pub last_error_time: Option<DateTime<Utc>>,
}

/// 获取性能指标
pub async fn get_metrics(
    State(state): State<Arc<super::DashboardState>>,
) -> Json<MetricsResponse> {
    let metrics = state.metrics.get_metrics().await;
    
    Json(MetricsResponse {
        uptime_secs: metrics.uptime_secs,
        requests: RequestMetrics {
            total: metrics.requests_total,
            successful: metrics.requests_successful,
            failed: metrics.requests_failed,
            avg_latency_ms: metrics.avg_latency_ms,
            p50_latency_ms: metrics.p50_latency_ms,
            p95_latency_ms: metrics.p95_latency_ms,
            p99_latency_ms: metrics.p99_latency_ms,
        },
        tokens: TokenMetrics {
            total_input: metrics.tokens_input,
            total_output: metrics.tokens_output,
            total: metrics.tokens_input + metrics.tokens_output,
            rate_per_minute: metrics.tokens_per_minute,
        },
        connections: ConnectionMetrics {
            feishu_websocket: metrics.feishu_connected,
            active_sessions: metrics.active_sessions,
            active_websockets: metrics.active_websockets,
        },
        errors: ErrorMetrics {
            total_errors: metrics.total_errors,
            error_rate: metrics.error_rate,
            last_error: metrics.last_error,
            last_error_time: metrics.last_error_time,
        },
    })
}

// ============== 健康检查 ==============

/// 健康检查响应
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub components: ComponentHealth,
}

/// 组件健康状态
#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub llm: ComponentStatus,
    pub feishu: ComponentStatus,
    pub database: ComponentStatus,
}

/// 组件状态
#[derive(Debug, Serialize)]
pub struct ComponentStatus {
    pub status: String,
    pub message: Option<String>,
}

/// 健康检查
pub async fn health_check(
    State(state): State<Arc<super::DashboardState>>,
) -> Json<HealthResponse> {
    let metrics = state.metrics.get_metrics().await;
    
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: metrics.uptime_secs,
        components: ComponentHealth {
            llm: ComponentStatus {
                status: if metrics.llm_available { "ok" } else { "warning" }.to_string(),
                message: if metrics.llm_available { 
                    None 
                } else { 
                    Some("LLM provider not configured".to_string()) 
                },
            },
            feishu: ComponentStatus {
                status: if metrics.feishu_connected { "ok" } else { "warning" }.to_string(),
                message: if metrics.feishu_connected { 
                    None 
                } else { 
                    Some("Feishu WebSocket not connected".to_string()) 
                },
            },
            database: ComponentStatus {
                status: "ok".to_string(),
                message: None,
            },
        },
    })
}
