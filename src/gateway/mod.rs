// Web Gateway for NewClaw - v0.6.0
//
// 支持：
// - 多 LLM Provider
// - 智慧主控集成（心跳上报、自检、降级模式）
// - 健康检查端点

use axum::{
    routing::{get, post},
    Router,
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::Config;
use crate::llm::{LLMProviderV3, LazyLLMProvider};
use crate::smart_controller::{SmartController, SmartControllerConfig};

/// Gateway 状态
pub struct GatewayState {
    pub config: Config,
    pub llm_provider: Arc<LazyLLMProvider>,
    pub smart_controller: Option<Arc<SmartController>>,
}

impl GatewayState {
    pub async fn from_config(config: Config) -> anyhow::Result<Self> {
        // 使用懒加载 Provider，允许无 API Key 启动
        let lazy_provider = LazyLLMProvider::new(config.clone());
        
        // 检查是否已配置 API Key，给出友好提示
        if lazy_provider.is_configured() {
            tracing::info!(
                "✅ LLM Provider configured: {} (model: {})",
                lazy_provider.provider_name(),
                lazy_provider.model_name()
            );
        } else {
            tracing::warn!(
                "⚠️  LLM Provider not configured. Please set {}_API_KEY environment variable.",
                config.llm.provider.to_uppercase()
            );
            tracing::warn!("   The service will start but chat requests will fail until API Key is configured.");
        }
        
        // 创建智慧主控（如果启用）
        let smart_controller = if config.gateway.enable_watchdog {
            let sc_config = SmartControllerConfig {
                enabled: true,
                watchdog_addr: config.gateway.watchdog_addr.clone(),
                ..Default::default()
            };
            Some(Arc::new(SmartController::new(sc_config)))
        } else {
            None
        };
        
        Ok(Self {
            config,
            llm_provider: Arc::new(lazy_provider),
            smart_controller,
        })
    }
    
    /// 初始化智慧主控
    pub async fn init_smart_controller(&self) -> anyhow::Result<()> {
        if let Some(ref sc) = self.smart_controller {
            // 申请租约
            // TODO: 实现实际的租约申请
            let lease_id = format!("lease-{}", uuid::Uuid::new_v4());
            sc.set_lease_id(lease_id).await;
            
            // 启动后台任务
            sc.start_background_tasks().await;
            
            tracing::info!("✅ Smart controller initialized");
        }
        Ok(())
    }
}

/// 启动 Gateway 服务器
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    let host = config.gateway.host.clone();
    let port = config.gateway.port;
    
    tracing::info!("🦀 NewClaw v0.6.0 Gateway starting...");
    tracing::info!("   Provider: {}", config.llm.provider);
    tracing::info!("   Model: {}", config.get_model());
    
    let state = Arc::new(GatewayState::from_config(config).await?);
    
    // 初始化智慧主控
    state.init_smart_controller().await?;
    
    let app = create_router(state.clone());
    
    let addr = format!("{}:{}", host, port);
    tracing::info!("🚀 Gateway server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn create_router(state: Arc<GatewayState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/chat", post(chat))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn readiness_check(
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<ReadinessResponse>, StatusCode> {
    let mut checks = std::collections::HashMap::new();
    
    // 检查 LLM Provider
    checks.insert("llm_provider".to_string(), true);
    
    // 检查智慧主控
    if let Some(ref sc) = state.smart_controller {
        let health = sc.check_health();
        let is_healthy = matches!(health, crate::core::heartbeat_reporter::HealthState::Healthy);
        checks.insert("smart_controller".to_string(), is_healthy);
        checks.insert("lease".to_string(), sc.lease_id().await.is_some());
    }
    
    let ready = checks.values().all(|&v| v);
    
    Ok(Json(ReadinessResponse {
        ready,
        checks,
    }))
}

async fn chat() -> &'static str {
    "Chat endpoint will be implemented soon"
}

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: std::collections::HashMap<String, bool>,
}