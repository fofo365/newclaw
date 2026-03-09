// Dashboard 对话 API
//
// 提供：
// 1. 多轮对话
// 2. 消息历史
// 3. 流式输出（SSE）

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
use futures::stream::{self, Stream};
use std::convert::Infallible;

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
    
    // TODO: 调用 LLM 生成回复
    // 目前返回一个模拟回复
    let assistant_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        content: format!("收到您的消息: {}", payload.content),
        timestamp: Utc::now(),
        tokens: Some(TokenUsage {
            input: payload.content.len() as u32 / 4,
            output: 20,
            total: payload.content.len() as u32 / 4 + 20,
        }),
        metadata: serde_json::json!({
            "model": "glm-4",
            "latency_ms": 150,
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
    
    tracing::info!("Added message to session {}: {} messages", id, session.messages.len());
    
    Ok(Json(assistant_message))
}

/// 流式响应（SSE）
pub async fn stream_response(
    Extension(state): Extension<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // 获取会话的最后一条用户消息
    let sessions = state.sessions.read().await;
    let last_content = sessions
        .iter()
        .find(|s| s.id == id)
        .and_then(|s| s.messages.last())
        .map(|m| m.content.clone())
        .unwrap_or_else(|| "Hello".to_string());
    drop(sessions);
    
    // 模拟流式输出
    let words: Vec<String> = last_content.split_whitespace().map(|s| s.to_string()).collect();
    let stream = futures::stream::iter(
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
    );
    
    Sse::new(stream)
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
