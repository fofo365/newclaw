// Dashboard 对话 API
//
// 提供：
// 1. 多轮对话（集成真实 LLM）
// 2. 消息历史
// 3. 流式输出（SSE）
// 4. Token 计数和费用统计

use axum::{
    extract::{Extension, Path, Json},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use futures::stream::{self, Stream, StreamExt};
use std::convert::Infallible;
use futures::pin_mut;

// ============== 数据结构 ==============

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<ChatMessage>,
    pub metadata: serde_json::Value,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: Option<TokenUsage>,
    pub metadata: serde_json::Value,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub total: u32,
}

/// 创建会话请求
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 发送消息请求
#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub stream: Option<bool>,
}

/// 会话列表响应
#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionSummary>,
    pub total: usize,
}

/// 会话摘要
#[derive(Debug, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub preview: String,
}

// ============== API 端点 ==============

/// 列出所有会话
pub async fn list_sessions(
    Extension(state): Extension<Arc<super::DashboardState>>,
) -> Json<SessionListResponse> {
    let sessions = state.sessions.read().await;
    
    let summaries: Vec<SessionSummary> = sessions
        .iter()
        .map(|s| {
            let preview = s.messages
                .last()
                .map(|m| {
                    if m.content.len() > 100 {
                        format!("{}...", &m.content[..100])
                    } else {
                        m.content.clone()
                    }
                })
                .unwrap_or_else(|| "No messages".to_string());
            
            SessionSummary {
                id: s.id.clone(),
                title: s.title.clone(),
                created_at: s.created_at,
                updated_at: s.updated_at,
                message_count: s.messages.len(),
                preview,
            }
        })
        .collect();
    
    Json(SessionListResponse {
        total: summaries.len(),
        sessions: summaries,
    })
}

/// 创建新会话
pub async fn create_session(
    Extension(state): Extension<Arc<super::DashboardState>>,
    Json(payload): Json<CreateSessionRequest>,
) -> Json<ChatSession> {
    let now = Utc::now();
    let session = ChatSession {
        id: Uuid::new_v4().to_string(),
        title: payload.title.unwrap_or_else(|| "New Chat".to_string()),
        created_at: now,
        updated_at: now,
        messages: Vec::new(),
        metadata: payload.metadata.unwrap_or(serde_json::json!({})),
    };
    
    let mut sessions = state.sessions.write().await;
    sessions.push(session.clone());
    
    tracing::info!("Created chat session: {}", session.id);
    
    Json(session)
}

/// 获取会话详情
pub async fn get_session(
    Extension(state): Extension<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> Result<Json<ChatSession>, (axum::http::StatusCode, String)> {
    let sessions = state.sessions.read().await;
    
    sessions
        .iter()
        .find(|s| s.id == id)
        .map(|s| Json(s.clone()))
        .ok_or_else(|| {
            (axum::http::StatusCode::NOT_FOUND, "Session not found".to_string())
        })
}

/// 发送消息
pub async fn send_message(
    Extension(state): Extension<Arc<super::DashboardState>>,
    Path(id): Path<String>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<ChatMessage>, (axum::http::StatusCode, String)> {
    let mut sessions = state.sessions.write().await;
    
    let session = sessions
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| {
            (axum::http::StatusCode::NOT_FOUND, "Session not found".to_string())
        })?;
    
    // 添加用户消息
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        content: payload.content.clone(),
        timestamp: Utc::now(),
        tokens: None,
        metadata: serde_json::json!({}),
    };
    session.messages.push(user_message.clone());
    
    // 获取 LLM 配置
    let llm_config = state.llm_config.read().await.clone();
    
    // 调用真实 LLM（如果配置）
    let start_time = std::time::Instant::now();
    let (assistant_content, token_usage, model_name) = if let Some(ref provider) = state.llm_provider {
        match call_llm(provider.as_ref(), &session.messages, &llm_config).await {
            Ok(result) => result,
            Err(e) => {
                tracing::error!("LLM call failed: {}", e);
                return Err((
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("LLM error: {}", e)
                ));
            }
        }
    } else {
        // 模拟响应（无 LLM 配置）
        tracing::warn!("No LLM provider configured, returning mock response");
        (
            format!("收到您的消息: {}", payload.content),
            TokenUsage {
                input: payload.content.len() as u32 / 4,
                output: 20,
                total: payload.content.len() as u32 / 4 + 20,
            },
            "mock".to_string(),
        )
    };
    
    let latency_ms = start_time.elapsed().as_millis() as u64;
    
    // 添加助手消息
    let assistant_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: assistant_content,
        timestamp: Utc::now(),
        tokens: Some(token_usage),
        metadata: serde_json::json!({
            "model": model_name,
            "latency_ms": latency_ms,
        }),
    };
    
    session.messages.push(assistant_message.clone());
    session.updated_at = Utc::now();
    
    // 更新标题（如果是第一条消息）
    if session.messages.len() == 2 && session.title == "New Chat" {
        session.title = if payload.content.len() > 30 {
            format!("{}...", &payload.content[..30])
        } else {
            payload.content.clone()
        };
    }
    
    tracing::info!(
        "Added message to session {}: {} messages, latency: {}ms",
        id, session.messages.len(), latency_ms
    );
    
    Ok(Json(assistant_message))
}

/// 调用 LLM Provider
async fn call_llm(
    provider: &dyn crate::llm::LLMProviderV3,
    messages: &[ChatMessage],
    llm_config: &Option<crate::config::LLMConfig>,
) -> Result<(String, TokenUsage, String), Box<dyn std::error::Error + Send + Sync>> {
    use crate::llm::{ChatRequest, Message, MessageRole};
    
    // 转换消息格式
    let llm_messages: Vec<Message> = messages
        .iter()
        .map(|m| Message {
            role: match m.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::System,
            },
            content: m.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        })
        .collect();
    
    // 获取模型和参数
    let (model, temperature, max_tokens) = match llm_config {
        Some(config) => (
            config.model.clone(),
            config.temperature,
            Some(config.max_tokens),
        ),
        None => (
            "glm-4".to_string(),
            0.7,
            None,
        ),
    };
    
    // 创建请求
    let request = ChatRequest {
        messages: llm_messages,
        model,
        temperature,
        max_tokens,
        top_p: None,
        stop: None,
        tools: None,
    };
    
    // 调用 LLM
    let response = provider.chat(request).await
        .map_err(|e| format!("LLM API error: {}", e))?;
    
    // 提取结果
    let content = response.message.content;
    let tokens = TokenUsage {
        input: response.usage.prompt_tokens as u32,
        output: response.usage.completion_tokens as u32,
        total: response.usage.total_tokens as u32,
    };
    
    Ok((content, tokens, response.model))
}

/// 流式响应（SSE）
pub async fn stream_response(
    Extension(state): Extension<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // 获取会话的最后一条用户消息
    let llm_config = state.llm_config.read().await.clone();
    
    let (messages, model, temperature) = {
        let sessions = state.sessions.read().await;
        let session = sessions.iter().find(|s| s.id == id);
        
        match session {
            Some(s) => {
                let msgs = s.messages.clone();
                let (model, temp) = match &llm_config {
                    Some(config) => (config.model.clone(), config.temperature),
                    None => ("glm-4".to_string(), 0.7),
                };
                (msgs, model, temp)
            }
            None => {
                // 会话不存在，返回错误流
                return Sse::new(futures::stream::iter(vec![
                    Ok::<_, Infallible>(Event::default().data("{\"error\": \"Session not found\"}")),
                    Ok(Event::default().data("[DONE]")),
                ])).into_response();
            }
        }
    };
    
    // 检查是否有 LLM Provider
    if let Some(ref provider) = state.llm_provider {
        // 使用真实 LLM 流式响应
        let stream = stream_llm_sse(provider.clone(), messages, model, temperature);
        Sse::new(stream).into_response()
    } else {
        // 模拟流式输出
        let last_content = messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "Hello".to_string());
        
        let stream = mock_stream(last_content);
        Sse::new(stream).into_response()
    }
}

/// 使用真实 LLM 的 SSE 流式响应
fn stream_llm_sse(
    provider: Arc<dyn crate::llm::LLMProviderV3>,
    messages: Vec<ChatMessage>,
    model: String,
    temperature: f32,
) -> impl Stream<Item = Result<Event, Infallible>> {
    use crate::llm::{ChatRequest, Message, MessageRole};
    
    // 创建异步流
    async_stream::stream! {
        // 转换消息格式
        let llm_messages: Vec<Message> = messages
            .iter()
            .map(|m| Message {
                role: match m.role.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    _ => MessageRole::System,
                },
                content: m.content.clone(),
                tool_calls: None,
                tool_call_id: None,
            })
            .collect();
        
        let request = ChatRequest {
            messages: llm_messages,
            model,
            temperature,
            max_tokens: None,
            top_p: None,
            stop: None,
            tools: None,
        };
        
        // 调用流式 LLM
        match provider.chat_stream(request).await {
            Ok(mut stream) => {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            // 发送 SSE 事件
                            let json = serde_json::to_string(&serde_json::json!({
                                "content": chunk,
                                "done": false
                            })).unwrap_or_default();
                            yield Ok(Event::default().data(json));
                        }
                        Err(e) => {
                            let json = serde_json::to_string(&serde_json::json!({
                                "error": e.to_string(),
                                "done": true
                            })).unwrap_or_default();
                            yield Ok(Event::default().data(json));
                            break;
                        }
                    }
                }
                
                // 发送完成事件
                yield Ok(Event::default().data("[DONE]"));
            }
            Err(e) => {
                let json = serde_json::to_string(&serde_json::json!({
                    "error": e.to_string(),
                    "done": true
                })).unwrap_or_default();
                yield Ok(Event::default().data(json));
            }
        }
    }
}

/// 模拟流式响应（无 LLM 配置时）
fn mock_stream(content: String) -> impl Stream<Item = Result<Event, Infallible>> {
    let words: Vec<String> = content.split_whitespace().map(|s| s.to_string()).collect();
    
    futures::stream::iter(
        words
            .into_iter()
            .enumerate()
            .map(|(i, word)| {
                let data = if i == 0 {
                    word
                } else {
                    format!(" {}", word)
                };
                Ok(Event::default().data(data))
            })
            .chain(std::iter::once(Ok(Event::default().data("[DONE]")))),
    )
}

// ============== 调试工具 ==============

/// 请求详情
#[derive(Debug, Serialize)]
pub struct RequestDebug {
    pub session_id: String,
    pub message_id: String,
    pub raw_request: serde_json::Value,
    pub raw_response: serde_json::Value,
    pub timing: TimingInfo,
}

/// 时间信息
#[derive(Debug, Serialize)]
pub struct TimingInfo {
    pub request_start: DateTime<Utc>,
    pub request_end: DateTime<Utc>,
    pub latency_ms: u64,
    pub first_token_ms: Option<u64>,
}
