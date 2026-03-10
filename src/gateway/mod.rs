// Web Gateway for NewClaw - v0.5.0
//
// 临时占位符 - 工具集成功能将在修复后重新启用

use axum::{
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::Config;
use crate::llm::{LLMProviderV3, create_glm_provider};

/// Gateway 状态
pub struct GatewayState {
    pub config: Config,
    pub llm_provider: Arc<Box<dyn LLMProviderV3>>,
}

impl GatewayState {
    pub async fn from_config(config: Config) -> anyhow::Result<Self> {
        // TODO: 支持多种 Provider 类型
        let api_key = config.get_api_key()?;  // 使用 get_api_key() 方法
        let model_name = config.get_model();
        let glm_provider = crate::llm::create_glm_provider(api_key, &model_name);
        
        // 包装成 Box<dyn LLMProviderV3>
        let llm_provider: Box<dyn LLMProviderV3> = Box::new(glm_provider);
        
        Ok(Self {
            config,
            llm_provider: Arc::new(llm_provider),
        })
    }
}

/// 启动 Gateway 服务器
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    let host = config.gateway.host.clone();
    let port = config.gateway.port;
    
    tracing::info!("🦀 NewClaw v0.5.0 Gateway starting...");
    tracing::info!("   Provider: {}", config.llm.provider);
    tracing::info!("   Model: {}", config.get_model());
    
    let state = Arc::new(GatewayState::from_config(config).await?);
    let app = create_router(state);
    
    let addr = format!("{}:{}", host, port);
    tracing::info!("🚀 Gateway server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn create_router(state: Arc<GatewayState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat))
}

async fn health_check() -> &'static str {
    "OK"
}

async fn chat() -> &'static str {
    "Chat endpoint will be implemented soon"
}
