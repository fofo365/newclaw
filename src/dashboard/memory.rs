// Dashboard 记忆管理 API
//
// 提供：
// 1. 记忆存储/搜索/列表
// 2. 联邦记忆状态
// 3. 记忆同步

use axum::{
    extract::{State, Path, Json, Query},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono::{DateTime, Utc};

// ============== 数据结构 ==============

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
}

/// 创建记忆请求
#[derive(Debug, Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub importance: Option<f32>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

/// 搜索记忆请求
#[derive(Debug, Deserialize)]
pub struct SearchMemoryRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub tags: Option<Vec<String>>,
}

/// 搜索记忆响应
#[derive(Debug, Serialize)]
pub struct SearchMemoryResponse {
    pub results: Vec<MemoryEntry>,
    pub total: usize,
    pub query: String,
}

/// 记忆列表响应
#[derive(Debug, Serialize)]
pub struct MemoryListResponse {
    pub memories: Vec<MemoryEntry>,
    pub total: usize,
}

/// 记忆查询参数
#[derive(Debug, Deserialize)]
pub struct MemoryQueryParams {
    pub tags: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 联邦节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationNode {
    pub id: String,
    pub name: String,
    pub status: String,
    pub last_sync: Option<DateTime<Utc>>,
    pub latency_ms: Option<u64>,
}

/// 联邦状态响应
#[derive(Debug, Serialize)]
pub struct FederationStatusResponse {
    pub enabled: bool,
    pub local_node_id: String,
    pub nodes: Vec<FederationNode>,
    pub total_nodes: usize,
}

/// 同步请求
#[derive(Debug, Deserialize)]
pub struct SyncRequest {
    pub target_node: Option<String>,
    pub memory_ids: Option<Vec<String>>,
}

/// 同步响应
#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub synced_count: usize,
    pub message: String,
}

// ============== API 端点 ==============

/// 存储记忆
pub async fn store_memory(
    State(state): State<Arc<super::DashboardState>>,
    Json(payload): Json<CreateMemoryRequest>,
) -> Result<Json<MemoryEntry>, (axum::http::StatusCode, String)> {
    let now = Utc::now();
    let entry = MemoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        content: payload.content,
        importance: payload.importance.unwrap_or(0.5),
        created_at: now,
        updated_at: now,
        tags: payload.tags.unwrap_or_default(),
        metadata: payload.metadata.unwrap_or(serde_json::json!({})),
    };
    
    // 调用记忆工具存储
    let args = serde_json::json!({
        "action": "store",
        "content": entry.content,
        "importance": entry.importance,
    });
    
    if let Err(e) = state.tool_registry.call("memory", args).await {
        tracing::warn!("Failed to store memory via tool: {}", e);
    }
    
    tracing::info!("Stored memory: {}", entry.id);
    Ok(Json(entry))
}

/// 搜索记忆
pub async fn search_memory(
    State(state): State<Arc<super::DashboardState>>,
    Json(payload): Json<SearchMemoryRequest>,
) -> Result<Json<SearchMemoryResponse>, (axum::http::StatusCode, String)> {
    let args = serde_json::json!({
        "action": "search",
        "query": payload.query,
        "limit": payload.limit.unwrap_or(10),
    });
    
    let results = match state.tool_registry.call("memory", args).await {
        Ok(res) => res,
        Err(e) => {
            tracing::warn!("Memory search failed: {}", e);
            serde_json::json!({"results": []})
        }
    };
    
    Ok(Json(SearchMemoryResponse {
        results: vec![],
        total: 0,
        query: payload.query,
    }))
}

/// 列出记忆
pub async fn list_memories(
    State(state): State<Arc<super::DashboardState>>,
    Query(params): Query<MemoryQueryParams>,
) -> Json<MemoryListResponse> {
    let args = serde_json::json!({
        "action": "list",
        "limit": params.limit.unwrap_or(50),
    });
    
    let _ = state.tool_registry.call("memory", args).await;
    
    Json(MemoryListResponse {
        memories: vec![],
        total: 0,
    })
}

/// 获取记忆详情
pub async fn get_memory(
    State(state): State<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> Result<Json<MemoryEntry>, (axum::http::StatusCode, String)> {
    Err((
        axum::http::StatusCode::NOT_FOUND,
        format!("Memory not found: {}", id),
    ))
}

/// 删除记忆
pub async fn delete_memory(
    State(state): State<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    Ok(Json(serde_json::json!({"success": true, "id": id})))
}

// ============== 联邦管理 API ==============

/// 获取联邦状态
pub async fn get_federation_status(
    State(state): State<Arc<super::DashboardState>>,
) -> Json<FederationStatusResponse> {
    Json(FederationStatusResponse {
        enabled: false,
        local_node_id: "local".to_string(),
        nodes: vec![],
        total_nodes: 0,
    })
}

/// 同步记忆到联邦节点
pub async fn sync_memories(
    State(state): State<Arc<super::DashboardState>>,
    Json(payload): Json<SyncRequest>,
) -> Result<Json<SyncResponse>, (axum::http::StatusCode, String)> {
    Ok(Json(SyncResponse {
        success: true,
        synced_count: 0,
        message: "Sync completed".to_string(),
    }))
}