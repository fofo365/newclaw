// Dashboard 配置 API
//
// 提供 LLM、工具、飞书配置的 CRUD 接口

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Json,
};
use axum::extract::State;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashSet;
use crate::config::{Config, LLMConfig, ToolsConfig};
use crate::llm::models;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    State(state): State<Arc<super::DashboardState>>,
) -> Result<Json<LLMConfigResponse>, AppError> {
    // 从 DashboardState 读取配置
    let llm_config = state.llm_config.read().await;
    
    let (provider, model, temperature, max_tokens) = match llm_config.as_ref() {
        Some(cfg) => (
            cfg.provider.clone(),
            cfg.model.clone(),
            cfg.temperature,
            cfg.max_tokens,
        ),
        None => (
            "glm".to_string(),
            "glm-4".to_string(),
            0.7,
            4096,
        ),
    };
    
    // 从 models.rs 获取所有提供商和模型
    let all_models = models::get_all_models();
    
    // 按提供商分组
    let mut provider_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut provider_display_names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    for model_info in all_models.iter() {
        let provider_name = model_info.provider.to_lowercase();
        
        provider_map.entry(provider_name.clone())
            .or_insert_with(Vec::new)
            .push(model_info.id.clone());
        
        if !provider_display_names.contains_key(&provider_name) {
            provider_display_names.insert(
                provider_name.clone(),
                get_provider_display_name(&provider_name),
            );
        }
    }
    
    // 检查哪些提供商已配置 API Key
    let mut configured_providers: HashSet<String> = HashSet::new();
    
    if std::env::var("GLM_API_KEY").is_ok() {
        configured_providers.insert("glm".to_string());
    }
    if std::env::var("ZAI_API_KEY").is_ok() {
        configured_providers.insert("z.ai".to_string());
    }
    if std::env::var("OPENAI_API_KEY").is_ok() {
        configured_providers.insert("openai".to_string());
    }
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        configured_providers.insert("claude".to_string());
    }
    
    // 构建 ProviderInfo 列表
    let providers: Vec<ProviderInfo> = provider_map
        .into_iter()
        .map(|(provider_name, models)| {
            ProviderInfo {
                name: provider_name.clone(),
                display_name: provider_display_names.get(&provider_name)
                    .unwrap_or(&provider_name)
                    .clone(),
                configured: configured_providers.contains(&provider_name),
                models,
            }
        })
        .collect();
    
    let response = LLMConfigResponse {
        provider,
        model,
        temperature,
        max_tokens,
        providers,
    };
    
    Ok(Json(response))
}

/// 获取提供商显示名称
fn get_provider_display_name(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "glm" => "GLM (智谱)".to_string(),
        "z.ai" | "zai" | "glmcode" => "GLM Code (z.ai)".to_string(),
        "openai" => "OpenAI".to_string(),
        "claude" | "anthropic" => "Claude (Anthropic)".to_string(),
        _ => {
            let mut name = provider.chars().next().unwrap().to_uppercase().collect::<String>();
            name.push_str(&provider.chars().skip(1).collect::<String>());
            name
        }
    }
}

/// 更新 LLM 配置
pub async fn update_llm_config(
    State(state): State<Arc<super::DashboardState>>,
    Json(payload): Json<UpdateLLMConfigRequest>,
) -> Result<Json<LLMConfigResponse>, AppError> {
    // 更新配置
    state.update_llm_config(payload).await
        .map_err(AppError)?;

    // 返回更新后的配置
    get_llm_config(State(state)).await
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
    State(_state): State<Arc<super::DashboardState>>,
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
    State(_state): State<Arc<super::DashboardState>>,
    Json(payload): Json<UpdateToolsConfigRequest>,
) -> Result<Json<ToolsConfigResponse>, AppError> {
    tracing::info!("Updating tools config: enabled={:?}", payload.enabled_tools);
    // TODO: 保存配置
    get_tools_config(State(_state)).await
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
    /// 是否已获取 access_token
    pub has_token: bool,
    /// 是否已获取 WebSocket URL
    pub has_websocket_url: bool,
}

/// 飞书配置更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFeishuConfigRequest {
    pub app_id: Option<String>,
    pub app_secret: Option<String>,
    pub encrypt_key: Option<String>,
    pub verification_token: Option<String>,
    pub connection_mode: Option<String>,
}

/// 获取飞书配置
pub async fn get_feishu_config(
    State(state): State<Arc<super::DashboardState>>,
) -> Result<Json<FeishuConfigResponse>, AppError> {
    let feishu_config = state.feishu_config.read().await;

    // 隐藏敏感信息 - 只返回掩码后的 app_id 用于显示
    let masked_app_id = feishu_config.app_id.as_ref().map(|id| {
        if id.len() > 4 {
            format!("{}****", &id[..4])
        } else {
            "****".to_string()
        }
    });

    // 检查是否有 token 和 WebSocket URL
    let has_token = feishu_config.access_token.is_some();
    let has_websocket_url = feishu_config.websocket_url.is_some();

    let response = FeishuConfigResponse {
        configured: feishu_config.configured,
        app_id: masked_app_id,
        connection_mode: feishu_config.connection_mode.clone().unwrap_or_else(|| "http_callback".to_string()),
        webhook_url: Some(format!("{}/api/feishu/webhook", "http://localhost:3001")),
        events_enabled: true,
        has_token,
        has_websocket_url,
    };

    Ok(Json(response))
}

/// 更新飞书配置
pub async fn update_feishu_config(
    State(state): State<Arc<super::DashboardState>>,
    Json(payload): Json<UpdateFeishuConfigRequest>,
) -> Result<Json<FeishuConfigResponse>, AppError> {
    tracing::info!("Updating Feishu config: app_id={:?}", payload.app_id);

    // 更新配置并保存到文件
    state.update_feishu_config(payload.clone()).await
        .map_err(AppError)?;

    // 测试飞书连接（如果配置了 app_id 和 app_secret）
    if payload.app_id.is_some() && payload.app_secret.is_some() {
        if let Ok(connected) = state.test_feishu_connection().await {
            tracing::info!("Feishu connection test result: {}", connected);
            if connected {
                // 更新连接状态
                state.metrics.update_connection_status(connected, true).await;
            }
        }
    }

    // 返回更新后的配置
    get_feishu_config(State(state)).await
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
