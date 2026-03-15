// Dashboard 管理 API
//
// 提供：
// 1. 用户管理
// 2. 权限控制（RBAC）
// 3. API Key 管理

use axum::{
    http::StatusCode,
    response::IntoResponse,
};
use axum::extract::{State, Path, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============== 用户管理 ==============

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub quota: QuotaInfo,
}

/// 配额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub max_requests_per_day: Option<u32>,
    pub max_tokens_per_day: Option<u64>,
    pub used_requests: u32,
    pub used_tokens: u64,
}

/// 创建用户请求
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
    pub role: Option<String>,
}

/// 用户列表响应
#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserInfo>,
    pub total: usize,
}

/// 列出用户
pub async fn list_users(
    State(_state): State<Arc<super::DashboardState>>,
) -> Json<UserListResponse> {
    // TODO: 从数据库读取用户
    // 目前返回模拟数据
    let users = vec![
        UserInfo {
            id: "admin".to_string(),
            username: "admin".to_string(),
            email: Some("admin@example.com".to_string()),
            role: "admin".to_string(),
            created_at: Utc::now() - chrono::Duration::days(30),
            last_login: Some(Utc::now() - chrono::Duration::hours(1)),
            is_active: true,
            quota: QuotaInfo {
                max_requests_per_day: None,
                max_tokens_per_day: None,
                used_requests: 150,
                used_tokens: 50000,
            },
        },
    ];
    
    Json(UserListResponse {
        total: users.len(),
        users,
    })
}

/// 创建用户
pub async fn create_user(
    State(_state): State<Arc<super::DashboardState>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserInfo>, AppError> {
    // 验证输入
    if payload.username.len() < 3 {
        return Err(AppError(anyhow::anyhow!("Username must be at least 3 characters")));
    }
    
    if payload.password.len() < 8 {
        return Err(AppError(anyhow::anyhow!("Password must be at least 8 characters")));
    }
    
    // TODO: 保存到数据库
    let user = UserInfo {
        id: Uuid::new_v4().to_string(),
        username: payload.username,
        email: payload.email,
        role: payload.role.unwrap_or_else(|| "user".to_string()),
        created_at: Utc::now(),
        last_login: None,
        is_active: true,
        quota: QuotaInfo {
            max_requests_per_day: Some(1000),
            max_tokens_per_day: Some(100000),
            used_requests: 0,
            used_tokens: 0,
        },
    };
    
    tracing::info!("Created user: {} (role: {})", user.username, user.role);
    
    Ok(Json(user))
}

/// 删除用户
pub async fn delete_user(
    State(_state): State<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    // 防止删除 admin
    if id == "admin" {
        return Err(AppError(anyhow::anyhow!("Cannot delete admin user")));
    }
    
    // TODO: 从数据库删除
    tracing::info!("Deleted user: {}", id);
    
    Ok(StatusCode::NO_CONTENT)
}

// ============== API Key 管理 ==============

/// API Key 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub permissions: Vec<String>,
    pub usage: KeyUsage,
}

/// Key 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyUsage {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub last_24h_requests: u64,
    pub last_24h_tokens: u64,
}

/// 创建 API Key 请求
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_in_days: Option<u32>,
    pub permissions: Option<Vec<String>>,
}

/// API Key 响应（包含完整 key，只返回一次）
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub info: ApiKeyInfo,
    pub key: String,  // 完整 key，只在创建时显示一次
}

/// 列出 API Keys
pub async fn list_api_keys(
    State(_state): State<Arc<super::DashboardState>>,
) -> Json<Vec<ApiKeyInfo>> {
    // TODO: 从数据库读取
    Json(vec![])
}

/// 创建 API Key
pub async fn create_api_key(
    State(_state): State<Arc<super::DashboardState>>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, AppError> {
    // 生成 API Key
    let key_id = Uuid::new_v4().to_string();
    let key_secret = Uuid::new_v4().to_string();
    let full_key = format!("nc_{}_{}", &key_id[..8], &key_secret.replace("-", ""));
    
    let info = ApiKeyInfo {
        id: key_id,
        name: payload.name,
        prefix: full_key[..12].to_string(),
        created_at: Utc::now(),
        expires_at: payload.expires_in_days.map(|days| {
            Utc::now() + chrono::Duration::days(days as i64)
        }),
        last_used: None,
        is_active: true,
        permissions: payload.permissions.unwrap_or_else(|| {
            vec!["chat".to_string(), "tools".to_string()]
        }),
        usage: KeyUsage {
            total_requests: 0,
            total_tokens: 0,
            last_24h_requests: 0,
            last_24h_tokens: 0,
        },
    };
    
    // TODO: 保存到数据库
    
    tracing::info!("Created API key: {} ({})", info.name, info.prefix);
    
    Ok(Json(ApiKeyResponse {
        info,
        key: full_key,
    }))
}

/// 撤销 API Key
pub async fn revoke_api_key(
    State(_state): State<Arc<super::DashboardState>>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    // TODO: 从数据库标记为已撤销
    tracing::info!("Revoked API key: {}", id);
    
    Ok(StatusCode::NO_CONTENT)
}

// ============== 权限管理 ==============

/// 角色信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

/// 获取所有角色
pub async fn list_roles() -> Json<Vec<RoleInfo>> {
    Json(vec![
        RoleInfo {
            name: "admin".to_string(),
            description: "Administrator with full access".to_string(),
            permissions: vec![
                "admin:read".to_string(),
                "admin:write".to_string(),
                "config:read".to_string(),
                "config:write".to_string(),
                "chat:read".to_string(),
                "chat:write".to_string(),
                "tools:execute".to_string(),
            ],
        },
        RoleInfo {
            name: "user".to_string(),
            description: "Standard user with chat access".to_string(),
            permissions: vec![
                "chat:read".to_string(),
                "chat:write".to_string(),
                "tools:execute".to_string(),
            ],
        },
        RoleInfo {
            name: "readonly".to_string(),
            description: "Read-only access".to_string(),
            permissions: vec![
                "chat:read".to_string(),
            ],
        },
    ])
}

// ============== 错误处理 ==============

#[derive(Debug)]
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Admin API error: {:?}", self.0);
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": self.0.to_string()
            })),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
