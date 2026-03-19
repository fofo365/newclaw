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
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::Config;
use crate::llm::{LLMProviderV3, create_glm_provider, OpenAIProvider, QwenCodeProvider, TokenUsage};
use crate::smart_controller::{SmartController, SmartControllerConfig};

/// 安全创建 LLM Provider（不抛出错误）
fn create_llm_provider(config: &Config) -> anyhow::Result<Box<dyn LLMProviderV3>> {
    let provider_type = config.llm.provider.to_lowercase();
    let model = config.get_model();
    
    match provider_type.as_str() {
        "openai" => {
            let api_key = config.llm.openai.api_key.clone()
                .ok_or_else(|| anyhow::anyhow!("OpenAI API key not found"))?;
            let base_url = config.llm.openai.base_url.clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            
            // 如果base_url是coding.dashscope，使用QwenCode Provider
            if base_url.contains("coding.dashscope") {
                let provider = QwenCodeProvider::new(api_key)
                    .with_base_url(base_url)
                    .with_default_model(model);
                Ok(Box::new(provider))
            } else {
                // 标准 OpenAI Provider
                let provider = OpenAIProvider::new(api_key)
                    .with_base_url(base_url)
                    .with_default_model(model);
                Ok(Box::new(provider))
            }
        }
        "qwencode" => {
            let api_key = config.llm.qwencode.api_key.clone()
                .or_else(|| config.llm.openai.api_key.clone())
                .ok_or_else(|| anyhow::anyhow!("QwenCode API key not found"))?;
            
            let provider = QwenCodeProvider::new(api_key)
                .with_default_model(model);
            Ok(Box::new(provider))
        }
        _ => {
            // 默认使用 GLM Provider
            let api_key = config.get_api_key()?;
            let provider = create_glm_provider(api_key, &model);
            Ok(Box::new(provider))
        }
    }
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
        let llm_provider = match create_llm_provider(&config) {
            Ok(provider) => {
                tracing::info!(
                    "✅ LLM Provider initialized: {} (model: {})",
                    config.llm.provider,
                    config.get_model()
                );
                Some(Arc::new(provider))
            }
            Err(e) => {
                tracing::warn!("⚠️  Failed to initialize LLM Provider: {}", e);
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
    // Dashboard 静态文件路径
    let dashboard_path = std::env::var("NEWCLAW_DASHBOARD_PATH")
        .unwrap_or_else(|_| "/opt/newclaw/dashboard".to_string());
    
    Router::new()
        .route("/health", get(health_check))
        .route("/ready", get(readiness_check))
        .route("/chat", post(chat))
        // Dashboard 静态文件服务
        .fallback_service(ServeDir::new(&dashboard_path))
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
    Json(req): Json<ChatApiRequest>,
) -> Result<Json<ChatApiResponse>, (StatusCode, Json<ErrorResponse>)> {
    // 检查 LLM Provider 是否配置
    let provider = match &state.llm_provider {
        Some(p) => p,
        None => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: "LLM Provider not configured".to_string(),
                    message: "Please configure API key in newclaw.toml".to_string(),
                }),
            ))
        }
    };
    
    // 转换消息格式
    let messages = req.messages.into_iter().map(|m| crate::llm::Message {
        role: match m.role.as_str() {
            "system" => crate::llm::MessageRole::System,
            "user" => crate::llm::MessageRole::User,
            "assistant" => crate::llm::MessageRole::Assistant,
            "tool" => crate::llm::MessageRole::Tool,
            _ => crate::llm::MessageRole::User,
        },
        content: m.content,
        tool_calls: None,
        tool_call_id: None,
    }).collect();
    
    let chat_req = crate::llm::ChatRequest {
        messages,
        model: req.model,
        temperature: req.temperature.unwrap_or(0.7),
        max_tokens: req.max_tokens,
        top_p: req.top_p,
        stop: req.stop,
        tools: None,
    };
    
    // 调用 LLM Provider
    match provider.chat(chat_req).await {
        Ok(resp) => {
            Ok(Json(ChatApiResponse {
                message: ChatApiMessage {
                    role: "assistant".to_string(),
                    content: resp.message.content,
                    tool_calls: None,
                },
                usage: TokenUsage {
                    prompt_tokens: resp.usage.prompt_tokens,
                    completion_tokens: resp.usage.completion_tokens,
                    total_tokens: resp.usage.total_tokens,
                },
                finish_reason: resp.finish_reason,
                model: resp.model,
            }))
        }
        Err(e) => {
            tracing::error!("LLM API error: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "LLM API error".to_string(),
                    message: e.to_string(),
                }),
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatApiRequest {
    model: String,
    messages: Vec<ChatApiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatApiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct ChatApiResponse {
    message: ChatApiMessage,
    usage: TokenUsage,
    finish_reason: Option<String>,
    model: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    message: String,
}

#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: std::collections::HashMap<String, bool>,
}