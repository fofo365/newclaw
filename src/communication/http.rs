// HTTP API Communication Module
use super::message::{AgentId, InterAgentMessage, MessagePayload};
use crate::security::{ApiKeyAuth, JwtAuth, RbacManager, AuditLogger};
use anyhow::{anyhow, Result};
use axum::{
    extract::{Extension, Json, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// HTTP API Server
pub struct HttpApiServer {
    addr: SocketAddr,
    state: Arc<ServerState>,
}

/// Server state
pub struct ServerState {
    pub agents: RwLock<HashMap<AgentId, AgentInfo>>,
    pub message_tx: mpsc::UnboundedSender<InterAgentMessage>,
    pub api_key_auth: Option<ApiKeyAuth>,
    pub jwt_auth: Option<JwtAuth>,
    pub rbac: Option<RbacManager>,
    pub audit: Option<AuditLogger>,
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub status: AgentStatus,
    pub connected_at: i64,
    pub metadata: HashMap<String, String>,
}

/// Agent status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentStatus {
    Online,
    Offline,
    Busy,
}

impl HttpApiServer {
    /// Create a new HTTP API server
    pub fn new(addr: SocketAddr) -> Self {
        let (message_tx, _) = mpsc::unbounded_channel();
        Self {
            addr,
            state: Arc::new(ServerState {
                agents: RwLock::new(HashMap::new()),
                message_tx,
                api_key_auth: None,
                jwt_auth: None,
                rbac: None,
                audit: None,
            }),
        }
    }

    /// Create with security components
    pub fn with_security(
        addr: SocketAddr,
        api_key_auth: ApiKeyAuth,
        jwt_auth: JwtAuth,
        rbac: RbacManager,
        audit: AuditLogger,
    ) -> Self {
        let (message_tx, _) = mpsc::unbounded_channel();
        Self {
            addr,
            state: Arc::new(ServerState {
                agents: RwLock::new(HashMap::new()),
                message_tx,
                api_key_auth: Some(api_key_auth),
                jwt_auth: Some(jwt_auth),
                rbac: Some(rbac),
                audit: Some(audit),
            }),
        }
    }

    /// Get message receiver
    pub fn message_receiver(&self) -> mpsc::UnboundedReceiver<InterAgentMessage> {
        let (_tx, rx) = mpsc::unbounded_channel();
        // Note: In production, you'd share the actual channel
        rx
    }

    /// Build the router
    pub fn build_router(&self) -> Router {
        Router::new()
            .route("/api/v1/send", post(send_message))
            .route("/api/v1/agents", get(list_agents).post(register_agent))
            .route("/api/v1/agents/:id", get(get_agent).delete(delete_agent))
            .route("/api/v1/broadcast", post(broadcast_message))
            .route("/api/v1/health", get(health_check))
            .layer(Extension(self.state.clone()))
    }

    /// Start the server
    pub async fn start(&self) -> Result<()> {
        let app = self.build_router();
        let addr = self.addr;

        tracing::info!("HTTP API server listening on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("HTTP server error: {}", e);
            }
        });

        Ok(())
    }
}

/// API error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "UNAUTHORIZED" => StatusCode::UNAUTHORIZED,
            "FORBIDDEN" => StatusCode::FORBIDDEN,
            "NOT_FOUND" => StatusCode::NOT_FOUND,
            "RATE_LIMITED" => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::BAD_REQUEST,
        };
        (status, Json(self)).into_response()
    }
}

/// Send message request
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub from: AgentId,
    pub to: AgentId,
    pub payload: serde_json::Value,
    pub priority: Option<String>,
}

/// Send message response
#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub status: String,
}

/// Send a message
async fn send_message(
    Extension(state): Extension<Arc<ServerState>>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, ApiError> {
    // Parse payload
    let payload: MessagePayload = serde_json::from_value(req.payload)
        .map_err(|e| ApiError {
            code: "INVALID_PAYLOAD".to_string(),
            message: format!("Invalid payload: {}", e),
        })?;

    // Create message
    let msg = InterAgentMessage::new(req.from, req.to, payload);

    // Send message
    state
        .message_tx
        .send(msg.clone())
        .map_err(|e| ApiError {
            code: "SEND_ERROR".to_string(),
            message: format!("Failed to send message: {}", e),
        })?;

    // Audit log
    if let Some(ref audit) = state.audit {
        let _ = audit
            .log_success(
                msg.from.clone(),
                "send_message".to_string(),
                format!("message:{}", msg.id),
                Some(serde_json::json!({
                    "to": msg.to,
                    "payload_type": match &msg.payload {
                        MessagePayload::Request(_) => "request",
                        MessagePayload::Response(_) => "response",
                        MessagePayload::Event(_) => "event",
                        MessagePayload::Command(_) => "command",
                    }
                })),
            )
            .await;
    }

    Ok(Json(SendMessageResponse {
        message_id: msg.id,
        status: "sent".to_string(),
    }))
}

/// List agents response
#[derive(Debug, Serialize, Deserialize)]
pub struct ListAgentsResponse {
    pub agents: Vec<AgentInfo>,
    pub total: usize,
}

/// List all agents
async fn list_agents(
    Extension(state): Extension<Arc<ServerState>>,
) -> Result<Json<ListAgentsResponse>, ApiError> {
    let agents = state.agents.read().await;
    let agent_list: Vec<AgentInfo> = agents.values().cloned().collect();
    let total = agent_list.len();

    Ok(Json(ListAgentsResponse {
        agents: agent_list,
        total,
    }))
}

/// Register agent request
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterAgentRequest {
    pub id: AgentId,
    pub name: String,
    pub metadata: Option<HashMap<String, String>>,
}

/// Register a new agent
async fn register_agent(
    Extension(state): Extension<Arc<ServerState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<AgentInfo>, ApiError> {
    let agent = AgentInfo {
        id: req.id.clone(),
        name: req.name,
        status: AgentStatus::Online,
        connected_at: chrono::Utc::now().timestamp(),
        metadata: req.metadata.unwrap_or_default(),
    };

    state.agents.write().await.insert(req.id, agent.clone());

    // Audit log
    if let Some(ref audit) = state.audit {
        let _ = audit
            .log_success(
                agent.id.clone(),
                "register_agent".to_string(),
                format!("agent:{}", agent.id),
                None,
            )
            .await;
    }

    Ok(Json(agent))
}

/// Get agent by ID
async fn get_agent(
    Path(id): Path<String>,
    Extension(state): Extension<Arc<ServerState>>,
) -> Result<Json<AgentInfo>, ApiError> {
    let agents = state.agents.read().await;
    let agent = agents
        .get(&id)
        .ok_or_else(|| ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Agent {} not found", id),
        })?
        .clone();

    Ok(Json(agent))
}

/// Delete agent by ID
async fn delete_agent(
    Path(id): Path<String>,
    Extension(state): Extension<Arc<ServerState>>,
) -> Result<StatusCode, ApiError> {
    let removed = state.agents.write().await.remove(&id).is_some();

    if removed {
        // Audit log
        if let Some(ref audit) = state.audit {
            let _ = audit
                .log_success(
                    id.clone(),
                    "delete_agent".to_string(),
                    format!("agent:{}", id),
                    None,
                )
                .await;
        }

        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError {
            code: "NOT_FOUND".to_string(),
            message: format!("Agent {} not found", id),
        })
    }
}

/// Broadcast message request
#[derive(Debug, Serialize, Deserialize)]
pub struct BroadcastRequest {
    pub from: AgentId,
    pub payload: serde_json::Value,
}

/// Broadcast a message to all agents
async fn broadcast_message(
    Extension(state): Extension<Arc<ServerState>>,
    Json(req): Json<BroadcastRequest>,
) -> Result<Json<SendMessageResponse>, ApiError> {
    let payload: MessagePayload = serde_json::from_value(req.payload)
        .map_err(|e| ApiError {
            code: "INVALID_PAYLOAD".to_string(),
            message: format!("Invalid payload: {}", e),
        })?;

    let msg = InterAgentMessage::new(req.from, "broadcast".to_string(), payload);

    state
        .message_tx
        .send(msg.clone())
        .map_err(|e| ApiError {
            code: "SEND_ERROR".to_string(),
            message: format!("Failed to broadcast message: {}", e),
        })?;

    Ok(Json(SendMessageResponse {
        message_id: msg.id,
        status: "broadcast".to_string(),
    }))
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub status: String,
    pub version: String,
    pub timestamp: i64,
}

/// Health check endpoint
async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// HTTP Client
pub struct HttpClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            api_key: None,
            client: reqwest::Client::new(),
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    /// Send a message
    pub async fn send(&self, msg: InterAgentMessage) -> Result<SendMessageResponse> {
        let url = format!("{}/api/v1/send", self.base_url);

        let req = SendMessageRequest {
            from: msg.from,
            to: msg.to,
            payload: serde_json::to_value(&msg.payload)?,
            priority: Some(match msg.priority {
                super::message::MessagePriority::Low => "low".to_string(),
                super::message::MessagePriority::Normal => "normal".to_string(),
                super::message::MessagePriority::High => "high".to_string(),
                super::message::MessagePriority::Urgent => "urgent".to_string(),
            }),
        };

        let mut builder = self.client.post(&url).json(&req);

        if let Some(ref api_key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = builder.send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ApiError = response.json().await?;
            Err(anyhow!("API error: {} - {}", error.code, error.message))
        }
    }

    /// Register an agent
    pub async fn register_agent(&self, id: &str, name: &str) -> Result<AgentInfo> {
        let url = format!("{}/api/v1/agents", self.base_url);

        let req = RegisterAgentRequest {
            id: id.to_string(),
            name: name.to_string(),
            metadata: None,
        };

        let response = self.client.post(&url).json(&req).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ApiError = response.json().await?;
            Err(anyhow!("API error: {} - {}", error.code, error.message))
        }
    }

    /// Get agent info
    pub async fn get_agent(&self, id: &str) -> Result<AgentInfo> {
        let url = format!("{}/api/v1/agents/{}", self.base_url, id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ApiError = response.json().await?;
            Err(anyhow!("API error: {} - {}", error.code, error.message))
        }
    }

    /// List all agents
    pub async fn list_agents(&self) -> Result<ListAgentsResponse> {
        let url = format!("{}/api/v1/agents", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            let error: ApiError = response.json().await?;
            Err(anyhow!("API error: {} - {}", error.code, error.message))
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = format!("{}/api/v1/health", self.base_url);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(anyhow!("Health check failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_response() {
        let response = HealthCheckResponse {
            status: "ok".to_string(),
            version: "0.2.0".to_string(),
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
    }

    #[test]
    fn test_send_message_request() {
        let req = SendMessageRequest {
            from: "agent-1".to_string(),
            to: "agent-2".to_string(),
            payload: serde_json::json!({"type": "test"}),
            priority: Some("normal".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("agent-1"));
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let client = HttpClient::new("http://localhost:3000".to_string());
        assert_eq!(client.base_url, "http://localhost:3000");
    }
}
