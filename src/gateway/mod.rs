// Web Gateway for NewClaw - Simplified version

use axum::{
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
}

async fn health_check() -> &'static str {
    "OK"
}

async fn chat_handler(
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, ErrorResponse> {
    // Create a new agent for each request (simple stateless approach)
    let mut agent = crate::core::AgentEngine::new(
        "NewClaw".to_string(),
        "glm-4".to_string(),
    ).map_err(|e| ErrorResponse {
        error: format!("Failed to create agent: {}", e),
    })?;
    
    match agent.process(&request.message).await {
        Ok(response) => Ok(Json(ChatResponse {
            response,
            session_id: request.session_id.unwrap_or_else(|| "default".to_string()),
        })),
        Err(e) => Err(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub response: String,
    pub session_id: String,
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
