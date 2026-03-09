// Web Gateway for NewClaw - v0.3.1
//
// 支持：
// 1. 多 LLM Provider (OpenAI, Claude, GLM)
// 2. 工具执行引擎集成
// 3. 配置文件支持
// 4. 向后兼容环境变量

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
    Extension,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::llm::{LLMProviderV3, OpenAIProvider, ClaudeProvider, GLMProvider, ChatRequest, Message, MessageRole, TokenUsage};
use crate::tools::{ToolRegistry, ReadTool, WriteTool, EditTool, ExecTool, SearchTool};
use std::collections::HashMap;

/// Gateway 状态
pub struct GatewayState {
    pub config: Config,
    pub llm_provider: Arc<Box<dyn LLMProviderV3>>,
    pub tool_registry: Arc<ToolRegistry>,
}

impl GatewayState {
    /// 从配置创建 Gateway 状态
    pub async fn from_config(config: Config) -> anyhow::Result<Self> {
        // 创建 LLM Provider
        let llm_provider = create_llm_provider(&config)?;
        
        // 创建工具注册表
        let tool_registry = Arc::new(ToolRegistry::new());
        
        // 注册默认工具
        register_default_tools(&tool_registry, &config).await;
        
        Ok(Self {
            config,
            llm_provider: Arc::new(llm_provider),
            tool_registry,
        })
    }
}

/// 创建 LLM Provider
fn create_llm_provider(config: &Config) -> anyhow::Result<Box<dyn LLMProviderV3>> {
    let provider = config.llm.provider.as_str();
    let api_key = config.get_api_key()?;
    
    match provider {
        "openai" => {
            let mut p = OpenAIProvider::new(api_key);
            if let Some(base_url) = &config.llm.openai.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            tracing::info!("Using OpenAI provider with model: {}", config.get_model());
            Ok(Box::new(p))
        }
        "claude" => {
            let mut p = ClaudeProvider::new(api_key);
            if let Some(base_url) = &config.llm.claude.base_url {
                p = p.with_base_url(base_url.clone());
            }
            p = p.with_default_model(config.get_model());
            tracing::info!("Using Claude provider with model: {}", config.get_model());
            Ok(Box::new(p))
        }
        "glm" => {
            // GLM 使用旧的 LegacyLLMProvider，需要包装
            tracing::info!("Using GLM provider with model: {}", config.get_model());
            let p = GLMProviderWrapper::new(api_key, config.get_model());
            Ok(Box::new(p))
        }
        other => {
            Err(anyhow::anyhow!("Unknown LLM provider: {}. Supported: openai, claude, glm", other))
        }
    }
}

/// GLM Provider 包装器（适配 LLMProviderV3 trait）
struct GLMProviderWrapper {
    inner: GLMProvider,
    model: String,
}

impl GLMProviderWrapper {
    fn new(api_key: String, model: String) -> Self {
        Self {
            inner: GLMProvider::new(api_key),
            model,
        }
    }
}

#[async_trait::async_trait]
impl LLMProviderV3 for GLMProviderWrapper {
    fn name(&self) -> &str {
        "glm"
    }
    
    async fn chat(&self, req: ChatRequest) -> Result<crate::llm::ChatResponse, crate::llm::LLMError> {
        // 转换请求格式
        let glm_messages: Vec<crate::llm::LLMMessage> = req.messages.into_iter()
            .map(|m| crate::llm::LLMMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::Tool => "tool".to_string(),
                },
                content: m.content,
            })
            .collect();
        
        let glm_req = crate::llm::LLMRequest {
            model: req.model.clone(),
            messages: glm_messages,
            temperature: req.temperature,
            max_tokens: req.max_tokens,
        };
        
        // 调用 GLM
        use crate::llm::LegacyLLMProvider;
        let resp = self.inner.chat(&glm_req).await
            .map_err(|e| crate::llm::LLMError::ApiError(e.to_string()))?;
        
        // 转换响应格式
        Ok(crate::llm::ChatResponse {
            message: Message {
                role: MessageRole::Assistant,
                content: resp.content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens: resp.tokens_used / 2, // 估算
                completion_tokens: resp.tokens_used / 2,
                total_tokens: resp.tokens_used,
            },
            finish_reason: Some("stop".to_string()),
            model: resp.model,
        })
    }
    
    async fn chat_stream(
        &self,
        _req: ChatRequest,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, crate::llm::LLMError>> + Send>>, crate::llm::LLMError> {
        Err(crate::llm::LLMError::ApiError("GLM streaming not implemented".to_string()))
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        // 简单估算
        let chinese_chars = text.chars().filter(|c| {
            let cp = *c as u32;
            (0x4E00..=0x9FFF).contains(&cp)
        }).count();
        let total = text.chars().count();
        (chinese_chars / 2) + ((total - chinese_chars) / 4)
    }
    
    async fn validate(&self) -> Result<bool, crate::llm::LLMError> {
        Ok(true) // 简化实现
    }
}

/// 注册默认工具
async fn register_default_tools(registry: &ToolRegistry, config: &Config) {
    let enabled = &config.tools.enabled;
    
    if enabled.contains(&"read".to_string()) || enabled.is_empty() {
        registry.register(Arc::new(ReadTool::default())).await;
    }
    if enabled.contains(&"write".to_string()) || enabled.is_empty() {
        registry.register(Arc::new(WriteTool::default())).await;
    }
    if enabled.contains(&"edit".to_string()) || enabled.is_empty() {
        registry.register(Arc::new(EditTool::default())).await;
    }
    if enabled.contains(&"exec".to_string()) || enabled.is_empty() {
        registry.register(Arc::new(ExecTool::default())).await;
    }
    if enabled.contains(&"search".to_string()) || enabled.is_empty() {
        registry.register(Arc::new(SearchTool::default())).await;
    }
    
    tracing::info!("Registered {} tools", registry.list().await.len());
}

/// 创建 Gateway Router
pub fn create_router(state: Arc<GatewayState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
        .route("/tools", get(list_tools))
        .route("/tools/execute", post(execute_tool))
        .layer(Extension(state))
}

/// 健康检查
async fn health_check() -> &'static str {
    "OK"
}

/// 聊天处理
async fn chat_handler(
    Extension(state): Extension<Arc<GatewayState>>,
    Json(request): Json<ChatRequestJson>,
) -> Result<Json<ChatResponseJson>, ErrorResponse> {
    let provider = &state.llm_provider;
    let model = state.config.get_model();
    
    // 构建请求
    let chat_request = ChatRequest {
        messages: vec![Message {
            role: MessageRole::User,
            content: request.message.clone(),
            tool_calls: None,
            tool_call_id: None,
        }],
        model,
        temperature: state.config.llm.temperature,
        max_tokens: Some(state.config.llm.max_tokens),
        top_p: None,
        stop: None,
        tools: None,
    };
    
    // 调用 LLM
    match provider.chat(chat_request).await {
        Ok(response) => Ok(Json(ChatResponseJson {
            response: response.message.content,
            session_id: request.session_id.unwrap_or_else(|| "default".to_string()),
            model: response.model,
            tokens_used: Some(response.usage.total_tokens),
        })),
        Err(e) => Err(ErrorResponse {
            error: format!("LLM error: {}", e),
        }),
    }
}

/// 列出可用工具
async fn list_tools(
    Extension(state): Extension<Arc<GatewayState>>,
) -> Json<Vec<crate::tools::ToolDescription>> {
    Json(state.tool_registry.list().await)
}

/// 执行工具
async fn execute_tool(
    Extension(state): Extension<Arc<GatewayState>>,
    Json(request): Json<ToolExecuteRequest>,
) -> Result<Json<crate::tools::ToolOutput>, ErrorResponse> {
    let result = state.tool_registry.execute(&request.name, request.params).await;
    
    match result {
        Ok(output) => Ok(Json(output)),
        Err(e) => Err(ErrorResponse {
            error: format!("Tool execution error: {}", e),
        }),
    }
}

// 请求/响应类型

#[derive(Debug, Deserialize)]
pub struct ChatRequestJson {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponseJson {
    pub response: String,
    pub session_id: String,
    pub model: String,
    pub tokens_used: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ToolExecuteRequest {
    pub name: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let body = serde_json::to_string(&self).unwrap();
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

/// 启动 Gateway 服务器
pub async fn run_server(config: Config) -> anyhow::Result<()> {
    let host = config.gateway.host.clone();
    let port = config.gateway.port;
    
    let state = Arc::new(GatewayState::from_config(config).await?);
    let app = create_router(state);
    
    let addr = format!("{}:{}", host, port);
    tracing::info!("🚀 Gateway server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

// 保留向后兼容的类型别名
// Re-export for backward compatibility (different names to avoid conflict)
pub type LegacyChatRequest = ChatRequestJson;
pub type LegacyChatResponse = ChatResponseJson;
