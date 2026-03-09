// NewClaw v0.4.0 - GLM Provider with Multi-Region Support
//
// 支持的区域:
// 1. GLM China (中国区域) - https://open.bigmodel.cn/api/paas/v4
// 2. GLM International (国际区域) - https://api.z.ai/api/paas/v4
// 3. GLMCode China (中国区域 Coding) - https://open.bigmodel.cn/api/coding/paas/v4
// 4. GLMCode International (国际区域 Coding) - https://api.z.ai/api/coding/paas/v4
//
// 参考: zeroclaw/src/providers/glm.rs 和 mod.rs

use async_trait::async_trait;
use reqwest::Client;
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::provider::{ChatRequest, ChatResponse, LLMError, LLMProviderV3, Message, MessageRole, TokenUsage};

/// GLM 区域枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GlmRegion {
    /// 中国区域 - open.bigmodel.cn
    China,
    /// 国际区域 - api.z.ai
    International,
}

impl Default for GlmRegion {
    fn default() -> Self {
        // 默认使用国际区域（更稳定）
        GlmRegion::International
    }
}

impl GlmRegion {
    /// 从字符串解析区域
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "china" | "cn" | "中国" | "glm-cn" | "bigmodel" => Some(GlmRegion::China),
            "international" | "intl" | "global" | "国际" | "glm" | "zhipu" | "z.ai" => Some(GlmRegion::International),
            _ => None,
        }
    }
    
    /// 获取 GLM API Base URL
    pub fn glm_base_url(&self) -> &'static str {
        match self {
            GlmRegion::China => GLM_CN_BASE_URL,
            GlmRegion::International => GLM_GLOBAL_BASE_URL,
        }
    }
    
    /// 获取 GLMCode (Coding) API Base URL
    pub fn glmcode_base_url(&self) -> &'static str {
        match self {
            GlmRegion::China => GLMCODE_CN_BASE_URL,
            GlmRegion::International => GLMCODE_GLOBAL_BASE_URL,
        }
    }
}

// GLM API 端点常量
const GLM_GLOBAL_BASE_URL: &str = "https://api.z.ai/api/paas/v4";
const GLM_CN_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
const GLMCODE_GLOBAL_BASE_URL: &str = "https://api.z.ai/api/coding/paas/v4";
const GLMCODE_CN_BASE_URL: &str = "https://open.bigmodel.cn/api/coding/paas/v4";

/// GLM Provider 类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum GlmProviderType {
    /// 标准 GLM API
    Glm,
    /// GLMCode (Coding 专用)
    GlmCode,
}

impl Default for GlmProviderType {
    fn default() -> Self {
        GlmProviderType::Glm
    }
}

/// GLM Provider 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmConfig {
    /// 区域设置
    #[serde(default)]
    pub region: GlmRegion,
    
    /// Provider 类型 (GLM 或 GLMCode)
    #[serde(default)]
    pub provider_type: GlmProviderType,
    
    /// 默认模型
    #[serde(default = "default_glm_model")]
    pub model: String,
    
    /// 温度
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_glm_model() -> String {
    "glm-4".to_string()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

impl Default for GlmConfig {
    fn default() -> Self {
        Self {
            region: GlmRegion::default(),
            provider_type: GlmProviderType::default(),
            model: default_glm_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

/// GLM Provider - 支持多区域
pub struct GlmProvider {
    /// API Key ID (从 id.secret 格式中解析)
    api_key_id: String,
    /// API Key Secret (从 id.secret 格式中解析)
    api_key_secret: String,
    /// Base URL
    base_url: String,
    /// 默认模型
    default_model: String,
    /// 温度
    temperature: f32,
    /// 最大 token
    max_tokens: usize,
    /// 缓存的 JWT token + 过期时间戳 (ms)
    token_cache: Mutex<Option<(String, u64)>>,
}

impl GlmProvider {
    /// 创建新的 GLM Provider
    pub fn new(api_key: String) -> Self {
        let (id, secret) = parse_glm_api_key(&api_key);
        Self {
            api_key_id: id,
            api_key_secret: secret,
            base_url: GLM_GLOBAL_BASE_URL.to_string(),
            default_model: default_glm_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            token_cache: Mutex::new(None),
        }
    }
    
    /// 使用指定区域创建 Provider
    pub fn with_region(api_key: String, region: GlmRegion) -> Self {
        let (id, secret) = parse_glm_api_key(&api_key);
        Self {
            api_key_id: id,
            api_key_secret: secret,
            base_url: region.glm_base_url().to_string(),
            default_model: default_glm_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            token_cache: Mutex::new(None),
        }
    }
    
    /// 使用指定配置创建 Provider
    pub fn with_config(api_key: String, config: GlmConfig) -> Self {
        let (id, secret) = parse_glm_api_key(&api_key);
        let base_url = match config.provider_type {
            GlmProviderType::Glm => config.region.glm_base_url().to_string(),
            GlmProviderType::GlmCode => config.region.glmcode_base_url().to_string(),
        };
        Self {
            api_key_id: id,
            api_key_secret: secret,
            base_url,
            default_model: config.model,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            token_cache: Mutex::new(None),
        }
    }
    
    /// 设置 Base URL (用于自定义端点)
    pub fn set_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
    
    /// 设置默认模型
    pub fn set_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }
    
    /// 生成 JWT Token (GLM 使用特殊的 JWT 认证)
    fn generate_token(&self) -> Result<String, LLMError> {
        if self.api_key_id.is_empty() || self.api_key_secret.is_empty() {
            return Err(LLMError::AuthError(
                "GLM API key not set or invalid format. Expected 'id.secret'. \
                 Set GLM_API_KEY environment variable or configure in config.toml.".to_string()
            ));
        }
        
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| LLMError::ApiError(format!("Time error: {}", e)))?
            .as_millis() as u64;
        
        // 检查缓存 (3 分钟有效，token 3.5 分钟后过期)
        if let Ok(cache) = self.token_cache.lock() {
            if let Some((ref token, expiry)) = *cache {
                if now_ms < expiry {
                    return Ok(token.clone());
                }
            }
        }
        
        let exp_ms = now_ms + 210_000; // 3.5 分钟
        
        // 构建 JWT (包含自定义 sign_type header)
        // Header: {"alg":"HS256","typ":"JWT","sign_type":"SIGN"}
        let header_json = r#"{"alg":"HS256","typ":"JWT","sign_type":"SIGN"}"#;
        let header_b64 = base64url_encode_str(header_json);
        
        // Payload: {"api_key":"...","exp":...,"timestamp":...}
        let payload_json = format!(
            r#"{{"api_key":"{}","exp":{},"timestamp":{}}}"#,
            self.api_key_id, exp_ms, now_ms
        );
        let payload_b64 = base64url_encode_str(&payload_json);
        
        // Sign: HMAC-SHA256(header.payload, secret)
        let signing_input = format!("{header_b64}.{payload_b64}");
        let key = hmac::Key::new(hmac::HMAC_SHA256, self.api_key_secret.as_bytes());
        let signature = hmac::sign(&key, signing_input.as_bytes());
        let sig_b64 = base64url_encode_bytes(signature.as_ref());
        
        let token = format!("{signing_input}.{sig_b64}");
        
        // 缓存 3 分钟
        if let Ok(mut cache) = self.token_cache.lock() {
            *cache = Some((token.clone(), now_ms + 180_000));
        }
        
        Ok(token)
    }
    
    /// 获取 HTTP 客户端
    fn http_client() -> Client {
        Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| Client::new())
    }
}

#[async_trait]
impl LLMProviderV3 for GlmProvider {
    fn name(&self) -> &str {
        "glm"
    }
    
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LLMError> {
        let token = self.generate_token()?;
        
        // 转换消息格式
        let messages: Vec<GlmMessage> = req.messages.into_iter()
            .map(|m| GlmMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::Tool => "tool".to_string(),
                },
                content: m.content,
            })
            .collect();
        
        let request = GlmChatRequest {
            model: req.model,
            messages,
            temperature: req.temperature as f64,
            max_tokens: req.max_tokens,
        };
        
        let url = format!("{}/chat/completions", self.base_url);
        
        let response = Self::http_client()
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;
        
        let status = response.status();
        if !status.is_success() {
            let error = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::ApiError(format!("GLM API error ({}): {}", status, error)));
        }
        
        let chat_response: GlmChatResponse = response
            .json()
            .await
            .map_err(|e| LLMError::SerializationError(e.to_string()))?;
        
        let content = chat_response.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();
        
        Ok(ChatResponse {
            message: Message {
                role: MessageRole::Assistant,
                content,
                tool_calls: None,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens: chat_response.usage.prompt_tokens,
                completion_tokens: chat_response.usage.completion_tokens,
                total_tokens: chat_response.usage.total_tokens,
            },
            finish_reason: Some("stop".to_string()),
            model: request.model,
        })
    }
    
    async fn chat_stream(
        &self,
        _req: ChatRequest,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<String, LLMError>> + Send>>, LLMError> {
        Err(LLMError::ApiError("GLM streaming not implemented yet".to_string()))
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        // 简单估算：中文字符算 0.5 token，英文单词算 0.25 token
        let chinese_chars = text.chars().filter(|c| {
            let cp = *c as u32;
            (0x4E00..=0x9FFF).contains(&cp)
        }).count();
        let total = text.chars().count();
        (chinese_chars / 2) + ((total - chinese_chars) / 4)
    }
    
    async fn validate(&self) -> Result<bool, LLMError> {
        if self.api_key_id.is_empty() || self.api_key_secret.is_empty() {
            return Ok(false);
        }
        
        // 尝试生成 token 来验证
        self.generate_token()?;
        Ok(true)
    }
}

// GLM API 数据结构

#[derive(Debug, Serialize)]
struct GlmChatRequest {
    model: String,
    messages: Vec<GlmMessage>,
    temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

#[derive(Debug, Serialize)]
struct GlmMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GlmChatResponse {
    choices: Vec<GlmChoice>,
    usage: GlmUsage,
}

#[derive(Debug, Deserialize)]
struct GlmChoice {
    message: GlmResponseMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GlmResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GlmUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

// 辅助函数

/// 解析 GLM API Key (格式: id.secret)
fn parse_glm_api_key(api_key: &str) -> (String, String) {
    api_key
        .split_once('.')
        .map(|(id, secret)| (id.to_string(), secret.to_string()))
        .unwrap_or_default()
}

/// Base64url 编码 (无填充，符合 JWT 规范)
fn base64url_encode_bytes(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;
    
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if i + 1 < data.len() {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        }
        if i + 2 < data.len() {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        }

        i += 3;
    }

    // 转换为 base64url: 替换 + 为 -, / 为 _, 移除 =
    result.replace('+', "-").replace('/', "_")
}

fn base64url_encode_str(s: &str) -> String {
    base64url_encode_bytes(s.as_bytes())
}

/// 从 provider 名称判断是否为 GLM Provider
pub fn is_glm_alias(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "glm" | "zhipu" | "glm-global" | "zhipu-global" | "glm-intl" |
        "glm-cn" | "zhipu-cn" | "bigmodel" | "glmcode" | "z.ai" |
        "zai" | "zai-global" | "zai-intl" | "zai-cn" | "z.ai-global" | "z.ai-cn"
    )
}

/// 从 provider 名称判断是否为 GLM 国际区域
pub fn is_glm_global_alias(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "glm" | "zhipu" | "glm-global" | "zhipu-global" | "glm-intl"
    )
}

/// 从 provider 名称判断是否为 GLM 中国区域
pub fn is_glm_cn_alias(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "glm-cn" | "zhipu-cn" | "bigmodel"
    )
}

/// 从 provider 名称判断是否为 z.ai (GLMCode) 国际区域
pub fn is_zai_global_alias(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "zai" | "z.ai" | "zai-global" | "z.ai-global" | "zai-intl" | "glmcode" | "glmcode-global" | "glmcode-intl"
    )
}

/// 从 provider 名称判断是否为 z.ai (GLMCode) 中国区域
pub fn is_zai_cn_alias(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "zai-cn" | "z.ai-cn" | "glmcode-cn" | "glmcode-china"
    )
}

/// 根据 provider 名称获取 GLM Base URL
pub fn glm_base_url(name: &str) -> Option<&'static str> {
    let name_lower = name.to_lowercase();
    
    if is_glm_cn_alias(&name_lower) {
        Some(GLM_CN_BASE_URL)
    } else if is_glm_global_alias(&name_lower) {
        Some(GLM_GLOBAL_BASE_URL)
    } else if is_zai_cn_alias(&name_lower) {
        Some(GLMCODE_CN_BASE_URL)
    } else if is_zai_global_alias(&name_lower) {
        Some(GLMCODE_GLOBAL_BASE_URL)
    } else {
        None
    }
}

/// 根据 provider 名称创建 GLM Provider
pub fn create_glm_provider(api_key: String, name: &str) -> GlmProvider {
    let name_lower = name.to_lowercase();
    
    let region = if is_glm_cn_alias(&name_lower) || is_zai_cn_alias(&name_lower) {
        GlmRegion::China
    } else {
        GlmRegion::International
    };
    
    let provider_type = if is_zai_global_alias(&name_lower) || is_zai_cn_alias(&name_lower) {
        GlmProviderType::GlmCode
    } else {
        GlmProviderType::Glm
    };
    
    GlmProvider::with_config(api_key, GlmConfig {
        region,
        provider_type,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_api_key() {
        let (id, secret) = parse_glm_api_key("abc123.secretXYZ");
        assert_eq!(id, "abc123");
        assert_eq!(secret, "secretXYZ");
    }
    
    #[test]
    fn test_parse_invalid_key() {
        let (id, secret) = parse_glm_api_key("no-dot-here");
        assert!(id.is_empty());
        assert!(secret.is_empty());
    }
    
    #[test]
    fn test_generate_jwt_token() {
        let provider = GlmProvider::new("testid.testsecret".to_string());
        let token = provider.generate_token().unwrap();
        assert!(!token.is_empty());
        
        // JWT 有 3 个点分隔的部分
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }
    
    #[test]
    fn test_token_caching() {
        let provider = GlmProvider::new("testid.testsecret".to_string());
        let token1 = provider.generate_token().unwrap();
        let token2 = provider.generate_token().unwrap();
        assert_eq!(token1, token2);
    }
    
    #[test]
    fn test_token_fails_without_key() {
        let provider = GlmProvider::new("".to_string());
        let result = provider.generate_token();
        assert!(result.is_err());
    }
    
    #[test]
    fn test_region_aliases() {
        assert!(is_glm_global_alias("glm"));
        assert!(is_glm_global_alias("zhipu-global"));
        assert!(is_glm_cn_alias("glm-cn"));
        assert!(is_glm_cn_alias("bigmodel"));
        assert!(is_zai_global_alias("z.ai"));
        assert!(is_zai_global_alias("glmcode"));
        assert!(is_zai_cn_alias("zai-cn"));
    }
    
    #[test]
    fn test_base_url_resolution() {
        assert_eq!(glm_base_url("glm"), Some(GLM_GLOBAL_BASE_URL));
        assert_eq!(glm_base_url("glm-cn"), Some(GLM_CN_BASE_URL));
        assert_eq!(glm_base_url("z.ai"), Some(GLMCODE_GLOBAL_BASE_URL));
        assert_eq!(glm_base_url("zai-cn"), Some(GLMCODE_CN_BASE_URL));
    }
    
    #[test]
    fn test_base64url_encoding() {
        let encoded = base64url_encode_bytes(b"hello");
        assert!(!encoded.contains('='));
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
    }
    
    #[test]
    fn test_count_tokens() {
        let provider = GlmProvider::new("test.test".to_string());
        
        // 英文
        let english = "Hello world this is a test";
        assert!(provider.count_tokens(english) > 0);
        
        // 中文
        let chinese = "你好世界这是一个测试";
        assert!(provider.count_tokens(chinese) > 0);
    }
}
