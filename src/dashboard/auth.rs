// NewClaw v0.4.0 - Dashboard Pairing Code Authentication
//
// 使用 6 位数字配对码进行身份认证
// 配对码通过 CLI 命令获取

use axum::{
    Json,
    extract::State,
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

use super::DashboardState;

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,        // session_id
    pub exp: usize,         // 过期时间
    pub iat: usize,         // 签发时间
}

/// 配对码信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairCode {
    pub code: String,           // 6 位数字
    pub session_id: String,     // 会话 ID
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

/// 认证状态
pub struct AuthState {
    /// 活跃的配对码
    pair_codes: Arc<RwLock<Vec<PairCode>>>,
    /// JWT Secret
    jwt_secret: String,
    /// 会话超时（秒）
    session_timeout_secs: u64,
}

impl AuthState {
    pub fn new(jwt_secret: String, session_timeout_secs: u64) -> Self {
        Self {
            pair_codes: Arc::new(RwLock::new(Vec::new())),
            jwt_secret,
            session_timeout_secs,
        }
    }
    
    /// 生成 6 位数字配对码
    pub async fn generate_pair_code(&self) -> PairCode {
        let code = Self::random_6digit();
        let session_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.session_timeout_secs as i64);
        
        let pair_code = PairCode {
            code: code.clone(),
            session_id: session_id.clone(),
            created_at: now,
            expires_at,
            used: false,
        };
        
        // 保存到活跃列表
        let mut codes = self.pair_codes.write().await;
        codes.push(pair_code.clone());
        
        // 清理过期配对码
        codes.retain(|c| c.expires_at > now);
        
        tracing::info!("Generated pair code: {} (session: {})", code, session_id);
        
        pair_code
    }
    
    /// 验证配对码
    pub async fn verify_pair_code(&self, code: &str) -> Option<String> {
        let now = Utc::now();
        let mut codes = self.pair_codes.write().await;
        
        // 查找匹配的未使用配对码
        if let Some(pair_code) = codes.iter_mut()
            .find(|c| c.code == code && !c.used && c.expires_at > now)
        {
            pair_code.used = true;
            let session_id = pair_code.session_id.clone();
            
            tracing::info!("Pair code {} verified for session {}", code, session_id);
            
            Some(session_id)
        } else {
            tracing::warn!("Invalid or expired pair code: {}", code);
            None
        }
    }
    
    /// 生成 JWT Token
    pub fn generate_token(&self, session_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::seconds(self.session_timeout_secs as i64);
        
        let claims = Claims {
            sub: session_id.to_string(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };
        
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
    }
    
    /// 验证 JWT Token
    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        ).map(|data| data.claims)
    }
    
    /// 生成随机 6 位数字
    fn random_6digit() -> String {
        use rand::Rng;
        let code: u32 = rand::thread_rng().gen_range(100_000..=999_999);
        code.to_string()
    }
}

/// 获取配对码请求
#[derive(Debug, Deserialize)]
pub struct GetPairCodeRequest {
    /// 安全码（可选，用于防止滥用）
    #[serde(default)]
    security_token: Option<String>,
}

/// 获取配对码响应
#[derive(Debug, Serialize)]
pub struct GetPairCodeResponse {
    pub code: String,
    pub session_id: String,
    pub expires_at: DateTime<Utc>,
}

/// 获取配对码 CLI 命令响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PairCodeInfo {
    pub code: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub dashboard_url: String,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub pair_code: String,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub session_id: String,
    pub expires_at: DateTime<Utc>,
}

/// API: 获取配对码（仅限本地访问）
pub async fn get_pair_code(
    State(state): State<Arc<DashboardState>>,
) -> Result<Json<GetPairCodeResponse>, StatusCode> {
    // 使用共享的认证状态
    let pair_code = state.auth_state.generate_pair_code().await;

    Ok(Json(GetPairCodeResponse {
        code: pair_code.code.clone(),
        session_id: pair_code.session_id,
        expires_at: pair_code.expires_at,
    }))
}

/// API: 登录（配对码换取 token）
pub async fn login(
    State(state): State<Arc<DashboardState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // 使用共享的认证状态
    // 验证配对码
    let session_id = match state.auth_state.verify_pair_code(&req.pair_code).await {
        Some(id) => id,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 生成 JWT Token
    let token = state.auth_state.generate_token(&session_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let expires_at = Utc::now() + chrono::Duration::seconds(state.config.session_timeout_secs as i64);

    tracing::info!("User logged in with session {}", session_id);

    Ok(Json(LoginResponse {
        token,
        session_id,
        expires_at,
    }))
}

/// API: 验证 Token
pub async fn verify_token(
    State(state): State<Arc<DashboardState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 从 Authorization Header 提取 token
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // 使用共享的认证状态验证 token
    let claims = state.auth_state.verify_token(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(serde_json::json!({
        "valid": true,
        "session_id": claims.sub,
    })))
}

/// JWT Token 提取器（用于其他 API 的认证）
pub struct AuthenticatedUser {
    pub session_id: String,
}

// 全局 JWT Secret（用于 FromRequestParts，因为无法访问 State）
static GLOBAL_JWT_SECRET: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// 设置全局 JWT Secret（在 Dashboard 启动时调用）
pub fn set_global_jwt_secret(secret: String) {
    let _ = GLOBAL_JWT_SECRET.set(secret);
}

/// 获取全局 JWT Secret
fn get_global_jwt_secret() -> &'static str {
    GLOBAL_JWT_SECRET.get().map(|s| s.as_str()).unwrap_or("newclaw-dashboard-secret")
}

impl<S> axum::extract::FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;
    
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // 从 Authorization Header 提取 token
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        
        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        
        let token = &auth_header[7..];
        
        // 使用全局 JWT Secret 验证 token
        let secret = get_global_jwt_secret();
        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        ).map_err(|e| {
            tracing::warn!("JWT validation failed: {:?}", e);
            StatusCode::UNAUTHORIZED
        })?;
        
        Ok(Self {
            session_id: claims.claims.sub,
        })
    }
}
