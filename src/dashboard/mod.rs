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
pub mod auth;

use axum::{
    Router,
    routing::{get, post, put, delete},
    Extension,
    response::Redirect,
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
    /// 认证状态（用于配对码管理）
    pub auth_state: Arc<auth::AuthState>,
    /// LLM Provider（可选，用于实际 AI 对话）
    pub llm_provider: Option<Arc<dyn crate::llm::LLMProviderV3>>,
    /// LLM 配置
    pub llm_config: Arc<RwLock<Option<crate::config::LLMConfig>>>,
    /// 配置文件路径
    pub config_path: Option<std::path::PathBuf>,
}

impl DashboardState {
    pub fn new(config: DashboardConfig) -> Self {
        let auth_state = Arc::new(auth::AuthState::new(
            config.jwt_secret.clone(),
            config.session_timeout_secs,
        ));

        Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
            sessions: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
            auth_state,
            llm_provider: None,
            llm_config: Arc::new(RwLock::new(None)),
            config_path: None,
        }
    }

    /// 创建带 LLM Provider 的 DashboardState
    pub fn with_llm(
        config: DashboardConfig,
        llm_config: crate::config::LLMConfig,
    ) -> anyhow::Result<Self> {
        // 创建 GLM Provider
        let provider = Self::create_llm_provider(&llm_config)?;

        let auth_state = Arc::new(auth::AuthState::new(
            config.jwt_secret.clone(),
            config.session_timeout_secs,
        ));

        Ok(Self {
            config,
            metrics: Arc::new(MetricsCollector::new()),
            sessions: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
            auth_state,
            llm_provider: Some(Arc::new(provider)),
            llm_config: Arc::new(RwLock::new(Some(llm_config))),
            config_path: None,
        })
    }

    /// 从配置文件创建 DashboardState
    pub fn from_config_file(
        dashboard_config: DashboardConfig,
        config_path: impl Into<std::path::PathBuf>,
    ) -> anyhow::Result<Self> {
        let config_path = config_path.into();

        // 尝试加载配置
        let llm_config = if config_path.exists() {
            match crate::config::Config::from_file(&config_path) {
                Ok(config) => Some(config.llm),
                Err(e) => {
                    tracing::warn!("Failed to load config from {}: {}", config_path.display(), e);
                    None
                }
            }
        } else {
            None
        };

        // 如果有 LLM 配置，创建 Provider
        let llm_provider: Option<Arc<dyn crate::llm::LLMProviderV3>> = if let Some(ref llm_cfg) = llm_config {
            match Self::create_llm_provider(llm_cfg) {
                Ok(provider) => Some(Arc::new(provider) as Arc<dyn crate::llm::LLMProviderV3>),
                Err(e) => {
                    tracing::warn!("Failed to create LLM provider: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // 创建认证状态
        let auth_state = Arc::new(auth::AuthState::new(
            dashboard_config.jwt_secret.clone(),
            dashboard_config.session_timeout_secs,
        ));

        Ok(Self {
            config: dashboard_config,
            metrics: Arc::new(MetricsCollector::new()),
            sessions: Arc::new(RwLock::new(Vec::new())),
            logs: Arc::new(RwLock::new(Vec::new())),
            auth_state,
            llm_provider,
            llm_config: Arc::new(RwLock::new(llm_config)),
            config_path: Some(config_path),
        })
    }

    /// 保存配置到文件
    pub async fn save_config(&self) -> anyhow::Result<()> {
        let config_path = self.config_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config file path set"))?;

        let llm_config = self.llm_config.read().await;

        // 构建完整配置
        let config = crate::config::Config {
            llm: llm_config.clone().unwrap_or_default(),
            gateway: crate::config::GatewayConfig::default(),
            tools: crate::config::ToolsConfig::default(),
            feishu: crate::config::FeishuConfig::default(),
        };

        // 序列化为 TOML
        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        // 写入文件
        tokio::fs::write(&config_path, toml_content).await
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

        tracing::info!("Saved config to {}", config_path.display());

        Ok(())
    }

    /// 重新加载配置
    pub async fn reload_config(&self) -> anyhow::Result<()> {
        let config_path = self.config_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No config file path set"))?;

        // 加载新配置
        let new_config = crate::config::Config::from_file(config_path)?;

        // 更新 LLM 配置
        let mut llm_config = self.llm_config.write().await;
        *llm_config = Some(new_config.llm.clone());

        tracing::info!("Reloaded config from {}", config_path.display());

        Ok(())
    }

    /// 更新 LLM 配置
    pub async fn update_llm_config(&self, updates: crate::dashboard::config_api::UpdateLLMConfigRequest) -> anyhow::Result<()> {
        let mut llm_config = self.llm_config.write().await;

        let config = llm_config.get_or_insert_with(|| crate::config::LLMConfig::default());

        // 应用更新
        if let Some(provider) = updates.provider {
            config.provider = provider;
        }
        if let Some(model) = updates.model {
            config.model = model;
        }
        if let Some(temperature) = updates.temperature {
            config.temperature = temperature;
        }
        if let Some(max_tokens) = updates.max_tokens {
            config.max_tokens = max_tokens;
        }
        if let Some(api_key) = updates.api_key {
            config.glm.api_key = Some(api_key);
        }
        if let Some(base_url) = updates.base_url {
            config.glm.base_url = Some(base_url);
        }
        if let Some(region) = updates.region {
            config.glm.region = region;
        }

        tracing::info!("Updated LLM config: provider={}, model={}", config.provider, config.model);

        // 保存到文件
        drop(llm_config); // 释放锁
        self.save_config().await?;

        Ok(())
    }

    /// 创建 LLM Provider
    fn create_llm_provider(
        llm_config: &crate::config::LLMConfig,
    ) -> anyhow::Result<crate::llm::GlmProvider> {
        use crate::llm::{GlmProvider, GlmConfig, GlmRegion, GlmProviderType};

        let provider_lower = llm_config.provider.to_lowercase();

        // 检查是否为 GLM 系列
        if !Self::is_glm_provider(&provider_lower) {
            anyhow::bail!("Only GLM providers are supported in Dashboard. Got: {}", llm_config.provider);
        }

        // 获取 API Key
        let api_key = llm_config.glm.api_key.clone()
            .or_else(|| std::env::var("GLM_API_KEY").ok())
            .ok_or_else(|| anyhow::anyhow!("GLM API key not found. Set GLM_API_KEY env var"))?;

        // 解析区域和类型
        let (region, provider_type) = Self::parse_glm_provider(&provider_lower);

        // 创建 GLM 配置
        let glm_config = GlmConfig {
            region,
            provider_type,
            model: llm_config.model.clone(),
            temperature: llm_config.temperature,
            max_tokens: llm_config.max_tokens,
        };

        Ok(GlmProvider::with_config(api_key, glm_config))
    }

    /// 检查是否为 GLM Provider
    fn is_glm_provider(name: &str) -> bool {
        matches!(
            name,
            "glm" | "glm-global" | "glm-cn" | "glm-intl" |
            "zhipu" | "zhipu-global" | "zhipu-cn" |
            "bigmodel" |
            "zai" | "z.ai" | "zai-global" | "zai-cn" | "z.ai-global" | "z.ai-cn" |
            "glmcode" | "glmcode-global" | "glmcode-cn" | "glmcode-intl"
        )
    }

    /// 解析 GLM Provider 名称
    fn parse_glm_provider(name: &str) -> (crate::llm::GlmRegion, crate::llm::GlmProviderType) {
        use crate::llm::{GlmRegion, GlmProviderType};

        match name {
            "glm-cn" | "zhipu-cn" | "bigmodel" => (GlmRegion::China, GlmProviderType::Glm),
            "glm" | "glm-global" | "glm-intl" | "zhipu" | "zhipu-global" => (GlmRegion::International, GlmProviderType::Glm),
            "zai-cn" | "z.ai-cn" | "glmcode-cn" => (GlmRegion::China, GlmProviderType::GlmCode),
            "zai" | "z.ai" | "zai-global" | "z.ai-global" | "glmcode" | "glmcode-global" => (GlmRegion::International, GlmProviderType::GlmCode),
            _ => (GlmRegion::International, GlmProviderType::Glm),
        }
    }
}

async fn redirect_to_login() -> impl axum::response::IntoResponse {
    axum::response::Redirect::permanent("/login.html")
}

/// 创建 Dashboard Router
pub fn create_dashboard_router(state: Arc<DashboardState>) -> Router {
    use tower_http::services::ServeDir;

    Router::new()
        // 认证 API（无需认证）
        .route("/api/auth/paircode", get(auth::get_pair_code))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/verify", get(auth::verify_token))

        // 配置 API（需要认证）
        .route("/api/config/llm", get(config_api::get_llm_config))
        .route("/api/config/llm", put(config_api::update_llm_config))
        .route("/api/config/tools", get(config_api::get_tools_config))
        .route("/api/config/tools", put(config_api::update_tools_config))
        .route("/api/config/feishu", get(config_api::get_feishu_config))
        .route("/api/config/feishu", put(config_api::update_feishu_config))

        // 监控 API（需要认证）
        .route("/api/monitor/logs", get(monitor::get_logs))
        .route("/api/monitor/logs/stream", get(monitor::stream_logs))
        .route("/api/monitor/metrics", get(monitor::get_metrics))
        .route("/api/monitor/health", get(monitor::health_check))

        // 对话 API（需要认证）
        .route("/api/chat/sessions", get(chat::list_sessions))
        .route("/api/chat/sessions", post(chat::create_session))
        .route("/api/chat/sessions/{id}", get(chat::get_session))
        .route("/api/chat/sessions/{id}/messages", post(chat::send_message))
        .route("/api/chat/sessions/{id}/stream", get(chat::stream_response))

        // 管理 API（需要认证）
        .route("/api/admin/users", get(admin::list_users))
        .route("/api/admin/users", post(admin::create_user))
        .route("/api/admin/users/{id}", delete(admin::delete_user))
        .route("/api/admin/apikeys", get(admin::list_api_keys))
        .route("/api/admin/apikeys", post(admin::create_api_key))
        .route("/api/admin/apikeys/{id}", delete(admin::revoke_api_key))

        // 静态文件服务（前端）
        // 默认重定向到登录页
        .route("/", get(redirect_to_login))
        .fallback_service(ServeDir::new("static"))

        .with_state(state)
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
