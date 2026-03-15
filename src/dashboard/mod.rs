// NewClaw v0.7.0 - Dashboard Module
//
// Dashboard 提供 Web UI 用于：
// 1. 配置管理（LLM、工具、飞书）
// 2. 监控面板（日志、指标、告警）
// 3. 对话界面（聊天、调试）
// 4. 管理功能（用户、权限、API Key）
// 5. 任务管理（任务、DAG、调度）
// 6. 记忆管理（存储、搜索、联邦）
// 7. 审计日志（查询、统计）

pub mod config_api;
pub mod monitor;
pub mod chat;
pub mod admin;
pub mod metrics;
pub mod session;
pub mod auth;
pub mod tasks;
pub mod memory;
pub mod audit;

use axum::{
    Router,
    routing::{get, post, put, delete},
    Extension,
    response::Redirect,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use crate::llm::{GlmRegion, GlmProviderType, GlmConfig as LlmGlmConfig};
use crate::tools::feishu::client::{FeishuClient, FeishuConfig as FeishuClientConfig};

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
    /// 飞书配置（内存存储）
    pub feishu_config: Arc<RwLock<FeishuConfig>>,
    /// 配置文件路径
    pub config_path: Option<std::path::PathBuf>,
    /// 工具注册表（用于工具调用）
    pub tool_registry: Arc<crate::tools::ToolRegistry>,
}

/// 飞书配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub encrypt_key: Option<String>,
    pub verification_token: Option<String>,
    pub connection_mode: Option<String>,
    pub configured: bool,
}

impl Default for FeishuConfig {
    fn default() -> Self {
        Self {
            app_id: std::env::var("FEISHU_APP_ID").ok(),
            app_secret: None,
            encrypt_key: std::env::var("FEISHU_ENCRYPT_KEY").ok(),
            verification_token: std::env::var("FEISHU_VERIFICATION_TOKEN").ok(),
            connection_mode: Some("http_callback".to_string()),
            configured: std::env::var("FEISHU_APP_ID").is_ok(),
        }
    }
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
            feishu_config: Arc::new(RwLock::new(FeishuConfig::default())),
            config_path: None,
            tool_registry: Arc::new(crate::tools::ToolRegistry::new()),
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
            feishu_config: Arc::new(RwLock::new(FeishuConfig::default())),
            config_path: None,
            tool_registry: Arc::new(crate::tools::ToolRegistry::new()),
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
            feishu_config: Arc::new(RwLock::new(FeishuConfig::default())),
            config_path: Some(config_path),
            tool_registry: Arc::new(crate::tools::ToolRegistry::new()),
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

        let config = llm_config.get_or_insert_with(crate::config::LLMConfig::default);

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

    /// 更新飞书配置
    pub async fn update_feishu_config(&self, payload: crate::dashboard::config_api::UpdateFeishuConfigRequest) -> anyhow::Result<()> {
        // 更新内存中的飞书配置
        let mut feishu_config = self.feishu_config.write().await;

        if let Some(app_id) = &payload.app_id {
            feishu_config.app_id = Some(app_id.clone());
            feishu_config.configured = !app_id.is_empty();
        }

        if let Some(app_secret) = &payload.app_secret {
            feishu_config.app_secret = Some(app_secret.clone());
        }

        if let Some(encrypt_key) = &payload.encrypt_key {
            feishu_config.encrypt_key = Some(encrypt_key.clone());
        }

        if let Some(verification_token) = &payload.verification_token {
            feishu_config.verification_token = Some(verification_token.clone());
        }

        if let Some(connection_mode) = &payload.connection_mode {
            feishu_config.connection_mode = Some(connection_mode.clone());
        }

        tracing::info!("Updated Feishu config: app_id={:?}", payload.app_id);

        // 释放锁
        drop(feishu_config);

        // 保存到文件
        if let Some(config_path) = &self.config_path {
            self.save_feishu_config_to_file(config_path).await?;
        } else {
            tracing::warn!("No config file path set, Feishu config not persisted");
        }

        Ok(())
    }

    /// 保存飞书配置到文件
    async fn save_feishu_config_to_file(&self, config_path: &std::path::PathBuf) -> anyhow::Result<()> {
        let feishu_config = self.feishu_config.read().await;

        // 创建配置文件目录（如果不存在）
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| anyhow::anyhow!("Failed to create config directory: {}", e))?;
        }

        // 读取现有配置（如果存在）
        let mut config = if config_path.exists() {
            crate::config::Config::from_file(config_path)?
        } else {
            crate::config::Config::default()
        };

        // 更新飞书配置（使用第一个账号）
        let app_id = feishu_config.app_id.as_ref().and_then(|s| if s.is_empty() { None } else { Some(s.clone()) });
        let app_secret = feishu_config.app_secret.as_ref().and_then(|s| if s.is_empty() { None } else { Some(s.clone()) });
        let encrypt_key = feishu_config.encrypt_key.as_ref().and_then(|s| if s.is_empty() { None } else { Some(s.clone()) });
        let verification_token = feishu_config.verification_token.as_ref().and_then(|s| if s.is_empty() { None } else { Some(s.clone()) });
        let connection_mode = feishu_config.connection_mode.as_ref().and_then(|s| if s.is_empty() { None } else { Some(s.clone()) });

        if let (Some(app_id), Some(app_secret)) = (app_id, app_secret) {
            // 更新或添加第一个账号
            config.feishu.accounts.entry("default".to_string()).or_insert_with(crate::config::FeishuAccount::default);
            if let Some(account) = config.feishu.accounts.get_mut("default") {
                account.app_id = app_id;
                account.app_secret = app_secret;
                if let Some(key) = encrypt_key {
                    account.encrypt_key = key;
                }
                if let Some(token) = verification_token {
                    account.verification_token = token;
                }
                if let Some(mode) = connection_mode {
                    account.connection_mode = mode;
                }
            }
        }

        // 序列化为 TOML
        let toml_content = toml::to_string_pretty(&config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config: {}", e))?;

        // 写入文件
        tokio::fs::write(config_path, toml_content).await
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

        tracing::info!("Saved Feishu config to {}", config_path.display());

        Ok(())
    }

    /// 测试飞书连接
    pub async fn test_feishu_connection(&self) -> anyhow::Result<bool> {
        let feishu_config = self.feishu_config.read().await;

        let app_id = feishu_config.app_id.as_ref().ok_or_else(|| anyhow::anyhow!("Feishu app_id not configured"))?;
        let app_secret = feishu_config.app_secret.as_ref().ok_or_else(|| anyhow::anyhow!("Feishu app_secret not configured"))?;

        if app_id.is_empty() || app_secret.is_empty() {
            return Ok(false);
        }

        // 创建飞书客户端
        let client_config = FeishuClientConfig {
            app_id: app_id.clone(),
            app_secret: app_secret.clone(),
            base_url: "https://open.feishu.cn/open-apis".to_string(),
        };

        let client = FeishuClient::new(client_config);

        // 尝试获取访问令牌来验证配置
        match client.get_access_token().await {
            Ok(token) => {
                tracing::info!("Feishu connection test successful, got token: {}...", &token[..20.min(token.len())]);
                Ok(true)
            }
            Err(e) => {
                tracing::warn!("Feishu connection test failed: {}", e);
                Ok(false)
            }
        }
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

        // 任务管理 API（v0.7.0）
        .route("/api/tasks", get(tasks::list_tasks))
        .route("/api/tasks", post(tasks::create_task))
        .route("/api/tasks/{id}", get(tasks::get_task))
        .route("/api/tasks/{id}/cancel", post(tasks::cancel_task))

        // DAG 工作流 API（v0.7.0）
        .route("/api/dags", get(tasks::list_dags))
        .route("/api/dags", post(tasks::create_dag))
        .route("/api/dags/{id}", get(tasks::get_dag_status))
        .route("/api/dags/{id}/run", post(tasks::run_dag))

        // 调度任务 API（v0.7.0）
        .route("/api/schedules", get(tasks::list_schedules))
        .route("/api/schedules", post(tasks::create_schedule))
        .route("/api/schedules/{id}", delete(tasks::delete_schedule))

        // 记忆管理 API（v0.7.0）
        .route("/api/memories", get(memory::list_memories))
        .route("/api/memories", post(memory::store_memory))
        .route("/api/memories/search", post(memory::search_memory))
        .route("/api/memories/{id}", get(memory::get_memory))
        .route("/api/memories/{id}", delete(memory::delete_memory))

        // 联邦管理 API（v0.7.0）
        .route("/api/federation/status", get(memory::get_federation_status))
        .route("/api/federation/sync", post(memory::sync_memories))

        // 审计日志 API（v0.7.0）
        .route("/api/audit/logs", get(audit::query_audit_logs))
        .route("/api/audit/logs/{id}", get(audit::get_audit_log))
        .route("/api/audit/stats", get(audit::get_audit_stats))
        .route("/api/audit/export", get(audit::export_audit_logs))

        // Prometheus 指标端点 (v0.5.5) - 无需认证
        .route("/metrics", get(prometheus_metrics))

        // 静态文件服务（前端）
        // 默认重定向到登录页
        .route("/", get(redirect_to_login))
        .fallback_service(ServeDir::new("static"))

        .with_state(state)
}

/// Prometheus /metrics 端点
pub async fn prometheus_metrics() -> impl axum::response::IntoResponse {
    use crate::metrics::prometheus::{export_metrics, init_metrics};
    
    // 确保已初始化（幂等操作）
    init_metrics();
    
    match export_metrics() {
        Ok(body) => (
            [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
            body,
        ),
        Err(e) => (
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            format!("Error exporting metrics: {}", e).into_bytes(),
        ),
    }
}

/// 启动 Dashboard 服务器
pub async fn start_dashboard(config: DashboardConfig) -> anyhow::Result<()> {
    use axum::serve;
    use tokio::net::TcpListener;

    // 初始化 Prometheus 指标
    crate::metrics::prometheus::init_metrics();

    // 尝试从配置文件初始化
    let state = if let Ok(config_path) = std::env::var("NEWCLAW_CONFIG") {
        let path = std::path::PathBuf::from(config_path);
        match DashboardState::from_config_file(config.clone(), &path) {
            Ok(state) => {
                tracing::info!("Loaded config from {}", path.display());
                Arc::new(state)
            }
            Err(e) => {
                tracing::warn!("Failed to load config from {}: {}, creating default state", path.display(), e);
                Arc::new(DashboardState::new(config.clone()))
            }
        }
    } else {
        // 尝试从环境变量初始化 LLM Provider
        if let Ok(api_key) = std::env::var("GLM_API_KEY") {
            tracing::info!("Found GLM_API_KEY, initializing LLM provider");
            let llm_config = crate::config::LLMConfig {
                provider: "glm".to_string(),
                model: std::env::var("GLM_MODEL").unwrap_or_else(|_| "glm-4".to_string()),
                temperature: std::env::var("GLM_TEMPERATURE")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.7),
                max_tokens: std::env::var("GLM_MAX_TOKENS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(4096),
                glm: crate::config::GlmProviderConfig {
                    api_key: Some(api_key),
                    base_url: std::env::var("GLM_BASE_URL").ok(),
                    region: "international".to_string(),
                    provider_type: "glm".to_string(),
                },
                openai: Default::default(),
                claude: Default::default(),
            };

            Arc::new(DashboardState::with_llm(config.clone(), llm_config)?)
        } else {
            tracing::warn!("No GLM_API_KEY found, starting Dashboard without LLM support");
            Arc::new(DashboardState::new(config.clone()))
        }
    };

    // 初始化内置工具
    let data_dir = std::path::PathBuf::from("./data");
    let workspace_dir = std::path::PathBuf::from(".");
    if let Err(e) = crate::tools::init_builtin_tools(&state.tool_registry, data_dir, workspace_dir).await {
        tracing::warn!("Failed to initialize some tools: {}", e);
    } else {
        tracing::info!("✅ Built-in tools initialized for Dashboard");
    }

    let app = create_dashboard_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("🚀 Dashboard starting on http://{}", addr);

    let listener = TcpListener::bind(&addr).await?;
    serve(listener, app).await?;

    Ok(())
}
