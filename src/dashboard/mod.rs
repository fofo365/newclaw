// NewClaw v0.4.0 - Dashboard Module
//
// Dashboard 提供 Web UI 用于：
// 1. 配置管理（LLM、工具、飞书）
// 2. 监控面板（日志、指标、告警）
// 3. 对话界面（聊天、调试）
// 4. 管理功能（用户、权限、API Key）

pub mod config_api;
pub mod monitor;
pub mod chat;
pub mod admin;
pub mod metrics;
pub mod session;

use axum::{
    Router,
    routing::{get, post, put, delete},
    Extension,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// Re-exports
pub use monitor::{LogEntry, LogFilter};
pub use chat::{ChatSession, ChatMessage};
pub use admin::{UserInfo, ApiKeyInfo};
pub use metrics::{MetricsCollector, SystemMetrics};

/// Dashboard 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// 是否启用 Dashboard
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Dashboard 端口（独立于 Gateway）
    #[serde(default = "default_dashboard_port")]
    pub port: u16,
    
    /// 是否启用认证
    #[serde(default)]
    pub auth_enabled: bool,
    
    /// JWT Secret（用于 Dashboard 认证）
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    
    /// 会话超时（秒）
    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: u64,
    
    /// 日志保留数量
    #[serde(default = "default_log_retention")]
    pub log_retention: usize,
    
    /// 指标保留时间（秒）
    #[serde(default = "default_metrics_retention")]
    pub metrics_retention_secs: u64,
}

fn default_enabled() -> bool { true }
fn default_dashboard_port() -> u16 { 8080 }
fn default_jwt_secret() -> String { "newclaw-dashboard-secret".to_string() }
fn default_session_timeout() -> u64 { 3600 }
fn default_log_retention() -> usize { 1000 }
fn default_metrics_retention() -> u64 { 3600 }

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            port: default_dashboard_port(),
            auth_enabled: false,
            jwt_secret: default_jwt_secret(),
            session_timeout_secs: default_session_timeout(),
            log_retention: default_log_retention(),
            metrics_retention_secs: default_metrics_retention(),
        }
    }
}

/// Dashboard 状态
pub struct DashboardState {
    pub config: DashboardConfig,
    pub metrics: Arc<MetricsCollector>,
    pub sessions: Arc<RwLock<Vec<ChatSession>>>,
    pub logs: Arc<RwLock<Vec<LogEntry>>>,
}

impl DashboardState {
    pub fn new(config: DashboardConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
            sessions: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

/// 创建 Dashboard Router
pub fn create_dashboard_router(state: Arc<DashboardState>) -> Router {
    use tower_http::services::ServeDir;
    
    Router::new()
        // 配置 API
        .route("/api/config/llm", get(config_api::get_llm_config))
        .route("/api/config/llm", put(config_api::update_llm_config))
        .route("/api/config/tools", get(config_api::get_tools_config))
        .route("/api/config/tools", put(config_api::update_tools_config))
        .route("/api/config/feishu", get(config_api::get_feishu_config))
        .route("/api/config/feishu", put(config_api::update_feishu_config))
        
        // 监控 API
        .route("/api/monitor/logs", get(monitor::get_logs))
        .route("/api/monitor/logs/stream", get(monitor::stream_logs))
        .route("/api/monitor/metrics", get(monitor::get_metrics))
        .route("/api/monitor/health", get(monitor::health_check))
        
        // 对话 API
        .route("/api/chat/sessions", get(chat::list_sessions))
        .route("/api/chat/sessions", post(chat::create_session))
        .route("/api/chat/sessions/:id", get(chat::get_session))
        .route("/api/chat/sessions/:id/messages", post(chat::send_message))
        .route("/api/chat/sessions/:id/stream", get(chat::stream_response))
        
        // 管理 API
        .route("/api/admin/users", get(admin::list_users))
        .route("/api/admin/users", post(admin::create_user))
        .route("/api/admin/users/:id", delete(admin::delete_user))
        .route("/api/admin/apikeys", get(admin::list_api_keys))
        .route("/api/admin/apikeys", post(admin::create_api_key))
        .route("/api/admin/apikeys/:id", delete(admin::revoke_api_key))
        
        // 静态文件服务（前端）- 从 static/ 目录
        .fallback_service(ServeDir::new("static").not_found_service(ServeDir::new("static/index.html")))
        
        .layer(Extension(state))
}

/// 启动 Dashboard 服务器
pub async fn start_dashboard(config: DashboardConfig) -> anyhow::Result<()> {
    use axum::serve;
    use tokio::net::TcpListener;
    
    let state = Arc::new(DashboardState::new(config.clone()));
    let app = create_dashboard_router(state);
    
    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("🚀 Dashboard starting on http://{}", addr);
    
    let listener = TcpListener::bind(&addr).await?;
    serve(listener, app).await?;
    
    Ok(())
}
