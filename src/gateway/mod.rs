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
use crate::llm::{LLMProviderV3, create_glm_provider};
use crate::smart_controller::{SmartController, SmartControllerConfig};

/// 安全创建 GLM Provider（不抛出错误）
fn create_glm_provider_safe(api_key: String, model: &str) -> anyhow::Result<Box<dyn LLMProviderV3>> {
    let provider = create_glm_provider(api_key, model);
    Ok(Box::new(provider))
}

/// Gateway 状态
pub struct GatewayState {
    pub config: Config,
    pub llm_provider: Option<Arc<Box<dyn LLMProviderV3>>>,  // 可选，不影响启动
    pub smart_controller: Option<Arc<SmartController>>,
}

impl GatewayState {
    pub async fn from_config(config: Config) -> anyhow::Result<Self> {
        // 尝试创建 LLM Provider，失败也不影响服务启动
        let llm_provider = match config.get_api_key() {
            Ok(api_key) => {
                let model_name = config.get_model();
                match create_glm_provider_safe(api_key, &model_name) {
                    Ok(provider) => {
                        tracing::info!(
                            "✅ LLM Provider initialized: {} (model: {})",
                            config.llm.provider,
                            model_name
                        );
                        Some(Arc::new(provider))
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  Failed to initialize LLM Provider: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("⚠️  LLM Provider not configured: {}", e);
                tracing::warn!("   Chat API will return error until API Key is configured.");
                None
            }
        };
        
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
            llm_provider,
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
    
    // 检查 LLM Provider（服务可以启动，但 LLM 可能未配置）
    checks.insert("llm_provider".to_string(), state.llm_provider.is_some());
    
    // 检查智慧主控
    if let Some(ref sc) = state.smart_controller {
        let health = sc.check_health();
        let is_healthy = matches!(health, crate::core::heartbeat_reporter::HealthState::Healthy);
        checks.insert("smart_controller".to_string(), is_healthy);
        checks.insert("lease".to_string(), sc.lease_id().await.is_some());
    }
    
    // 注意：即使 LLM 未配置，服务也是"就绪"的（可以启动和响应其他端点）
    let ready = true;
    
    Ok(Json(ReadinessResponse {
        ready,
        checks,
    }))
}

async fn chat(
    State(state): State<Arc<GatewayState>>,
) -> Result<&'static str, (StatusCode, &'static str)> {
    // 检查 LLM Provider 是否配置
    if state.llm_provider.is_none() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "LLM Provider not configured. Please set GLM_API_KEY environment variable or configure in newclaw.toml"
        ));
    }
    
    Ok("Chat endpoint will be implemented soon")
}

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: std::collections::HashMap<String, bool>,
}