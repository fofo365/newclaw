use std::sync::Arc;
use axum::http::StatusCode;
use axum::extract::State;
use axum::{middleware::Next, response::Response};

/// 认证检查中间件 - 不需要 State 参数
pub async fn auth_check_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode>
{
    // 检查路径是否需要认证
    let path = req.uri().path();
    let exempt_paths = [
        "/api/auth/paircode",
        "/api/auth/login",
        "/api/auth/verify",
        "/metrics",
        "/health",
    ];

    if exempt_paths.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(req).await);
    }

    // 提取认证头
    let auth_header = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = &auth_header[7..];

    // 这里无法验证 token，因为中间件无法访问 state
    // 暂时只检查格式，实际验证在 handler 中进行
    Ok(next.run(req).await)
}
