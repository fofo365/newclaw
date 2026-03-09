// Dashboard 配置 API
//
// 提供 LLM、工具、飞书配置的 CRUD 接口

use axum::{
    extract::Extension,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::{Config, LLMConfig, ToolsConfig};

/// 配置状态
pub struct ConfigState {
    pub config: Arc<tokio::sync::RwLock<Config>>,
}

// ============== LLM 配置 ==============

/// LLM 配置响应
#[derive(Debug, Serialize, Deserialize)]
pub struct LLMConfigResponse {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub providers: Vec<ProviderInfo>,
}

/// Provider 信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub display_name: String,
    pub configured: bool,
    pub models: Vec<String>,
}

/// LLM 配置更新请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLLMConfigRequest {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub region: Option<String>,
}

/// 获取 LLM 配置
pub async fn get_llm_config(
    Extension(state): Extension<Arc<super::DashboardState>>,
) -> Result<Json<LLMConfigResponse>, AppError> {
    // 这里暂时返回默认配置
    // 实际应该从 ConfigState 读取
    let response = LLMConfigResponse {
        provider: "glm".to_string(),
        model: "glm-4".to_string(),
        temperature: 0.7,
        max_tokens: 4096,
        providers: vec![
            ProviderInfo {
                name: "openai".to_string(),
                display_name: "OpenAI".to_string(),
                configured: std::env::var("OPENAI_API_KEY").is_ok(),
                models: vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string(), "gpt-4-turbo".to_string()],
            },
            ProviderInfo {
                name: "claude".to_string(),
                display_name: "Claude (Anthropic)".to_string(),
                configured: std::env::var("ANTHROPIC_API_KEY").is_ok(),
                models: vec!["claude-3-5-sonnet".to_string(), "claude-3-opus".to_string()],
            },
            ProviderInfo {
                name: "glm".to_string(),
                display_name: "GLM (智谱)".to_string(),
                configured: std::env::var("GLM_API_KEY").is_ok(),
                models: vec!["glm-4".to_string(), "glm-4-flash".to_string()],
            },
            ProviderInfo {
                name: "glmcode".to_string(),
                display_name: "GLM Code (z.ai)".to_string(),
                configured: std::env::var("GLM_API_KEY").is_ok(),
                models: vec!["glm-4.7".to_string(), "glm-5".to_string()],
            },
        ],
    };
    
    Ok(Json(response))
}

/// 更新 LLM 配置
pub async fn update_llm_config(
    Extension(state): Extension<Arc<super::DashboardState>>,
    Json(payload): Json<UpdateLLMConfigRequest>,
) -> Result<Json<LLMConfigResponse>, AppError> {
    // TODO: 实际保存配置到文件
    // 目前只返回更新后的配置
    
    let provider = payload.provider.unwrap_or_else(|| "glm".to_string());
    let model = payload.model.unwrap_or_else(|| "glm-4".to_string());
    
    tracing::info!("Updating LLM config: provider={}, model={}", provider, model);
    
    // 返回更新后的配置
    get_llm_config(Extension(state)).await
}

// ============== 工具配置 ==============

/// 工具配置响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolsConfigResponse {
    pub tools: Vec<ToolInfo>,
}

/// 工具信息
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub enabled: bool,
    pub category: String,
    pub parameters: Vec<ToolParameter>,
}

/// 工具参数
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub type_: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
}

/// 工具配置更新请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateToolsConfigRequest {
    pub enabled_tools: Vec<String>,
    pub tool_configs: std::collections::HashMap<String, serde_json::Value>,
}

/// 获取工具配置
pub async fn get_tools_config(
    Extension(_state): Extension<Arc<super::DashboardState>>,
) -> Result<Json<ToolsConfigResponse>, AppError> {
    let response = ToolsConfigResponse {
        tools: vec![
            ToolInfo {
                name: "read".to_string(),
                display_name: "Read File".to_string(),
                description: "Read file contents".to_string(),
                enabled: true,
                category: "file".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        type_: "string".to_string(),
                        description: "File path to read".to_string(),
                        required: true,
                        default: None,
                    },
                ],
            },
            ToolInfo {
                name: "write".to_string(),
                display_name: "Write File".to_string(),
                description: "Write content to file".to_string(),
                enabled: true,
                category: "file".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        type_: "string".to_string(),
                        description: "File path to write".to_string(),
                        required: true,
                        default: None,
                    },
                    ToolParameter {
                        name: "content".to_string(),
                        type_: "string".to_string(),
                        description: "Content to write".to_string(),
                        required: true,
                        default: None,
                    },
                ],
            },
            ToolInfo {
                name: "edit".to_string(),
                display_name: "Edit File".to_string(),
                description: "Edit file with find/replace".to_string(),
                enabled: true,
                category: "file".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "path".to_string(),
                        type_: "string".to_string(),
                        description: "File path to edit".to_string(),
                        required: true,
                        default: None,
                    },
                ],
            },
            ToolInfo {
                name: "exec".to_string(),
                display_name: "Execute Command".to_string(),
                description: "Execute shell commands".to_string(),
                enabled: true,
                category: "system".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "command".to_string(),
                        type_: "string".to_string(),
                        description: "Command to execute".to_string(),
                        required: true,
                        default: None,
                    },
                ],
            },
            ToolInfo {
                name: "web_search".to_string(),
                display_name: "Web Search".to_string(),
                description: "Search the web using Brave Search API".to_string(),
                enabled: true,
                category: "web".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "query".to_string(),
                        type_: "string".to_string(),
                        description: "Search query".to_string(),
                        required: true,
                        default: None,
                    },
                ],
            },
        ],
    };
    
    Ok(Json(response))
}

/// 更新工具配置
pub async fn update_tools_config(
    Extension(_state): Extension<Arc<super::DashboardState>>,
    Json(payload): Json<UpdateToolsConfigRequest>,
) -> Result<Json<ToolsConfigResponse>, AppError> {
    tracing::info!("Updating tools config: enabled={:?}", payload.enabled_tools);
    // TODO: 保存配置
    get_tools_config(Extension(_state)).await
}

// ============== 飞书配置 ==============

/// 飞书配置响应
#[derive(Debug, Serialize, Deserialize)]
pub struct FeishuConfigResponse {
    pub configured: bool,
    pub app_id: Option<String>,
    pub connection_mode: String,
    pub webhook_url: Option<String>,
    pub events_enabled: bool,
}

/// 飞书配置更新请求
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateFeishuConfigRequest {
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub encrypt_key: Option<String>,
    pub verification_token: Option<String>,
    pub connection_mode: Option<String>,
}

/// 获取飞书配置
pub async fn get_feishu_config(
    Extension(_state): Extension<Arc<super::DashboardState>>,
) -> Result<Json<FeishuConfigResponse>, AppError> {
    let app_id = std::env::var("FEISHU_APP_ID").ok();
    let configured = app_id.is_some();
    
    // 隐藏敏感信息
    let masked_app_id = app_id.map(|id| {
        if id.len() > 4 {
            format!("{}****", &id[..4])
        } else {
            "****".to_string()
        }
    });
    
    let response = FeishuConfigResponse {
        configured,
        app_id: masked_app_id,
        connection_mode: "websocket".to_string(),
        webhook_url: None,
        events_enabled: true,
    };
    
    Ok(Json(response))
}

/// 更新飞书配置
pub async fn update_feishu_config(
    Extension(_state): Extension<Arc<super::DashboardState>>,
    Json(payload): Json<UpdateFeishuConfigRequest>,
) -> Result<Json<FeishuConfigResponse>, AppError> {
    tracing::info!("Updating Feishu config: app_id={:?}", payload.app_id);
    // TODO: 保存配置到文件和环境变量
    get_feishu_config(Extension(_state)).await
}

// ============== 静态文件服务 ==============

/// 静态文件服务（前端）
pub async fn serve_static(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> impl IntoResponse {
    // 如果是 API 请求但没有匹配到路由，返回 404
    if path.starts_with("api/") {
        return Err((StatusCode::NOT_FOUND, "API endpoint not found"));
    }
    
    // 尝试提供静态文件
    let static_path = if path.is_empty() || path == "index.html" {
        "index.html"
    } else {
        &path
    };
    
    // TODO: 从 build 目录读取文件
    // 目前返回一个简单的 HTML 页面
    let html = include_str!("dashboard.html");
    
    Ok((
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html.to_string(),
    ))
}

// ============== 错误处理 ==============

/// 应用错误
#[derive(Debug)]
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("API error: {:?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
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
