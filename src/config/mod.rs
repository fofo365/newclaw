// NewClaw v0.4.0 - 配置管理
//
// 支持：
// 1. TOML 配置文件
// 2. 环境变量覆盖
// 3. 默认值
// 4. GLM 多区域支持
// 5. v0.7.0: 6 层配置架构

// v0.7.0: 6 层配置系统
pub mod layers;

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, Context};

/// 主配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Config {
    #[serde(default)]
    pub llm: LLMConfig,
    
    #[serde(default)]
    pub gateway: GatewayConfig,
    
    #[serde(default)]
    pub tools: ToolsConfig,
    
    #[serde(default)]
    pub feishu: FeishuConfig,
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// 提供商: openai, claude, glm, glm-cn, glm-global, zai, zai-cn, qwencode
    #[serde(default = "default_provider")]
    pub provider: String,
    
    /// 默认模型
    #[serde(default = "default_model")]
    pub model: String,
    
    /// 温度
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// 最大 token
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    
    /// OpenAI 配置
    #[serde(default)]
    pub openai: ProviderCredentials,
    
    /// Claude 配置
    #[serde(default)]
    pub claude: ProviderCredentials,
    
    /// QwenCode 配置
    #[serde(default)]
    pub qwencode: ProviderCredentials,
    
    /// GLM 配置 (支持多区域)
    #[serde(default)]
    pub glm: GlmProviderConfig,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            openai: ProviderCredentials::default(),
            claude: ProviderCredentials::default(),
            qwencode: ProviderCredentials::default(),
            glm: GlmProviderConfig::default(),
        }
    }
}

fn default_provider() -> String {
    std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "glm".to_string())
}

fn default_model() -> String {
    // 根据提供商返回默认模型
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "glm".to_string());
    match provider.to_lowercase().as_str() {
        "openai" => std::env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        "claude" => std::env::var("LLM_MODEL").unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
        "qwencode" => std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwencode/glm-4.7".to_string()),
        // GLM 和 z.ai (GLMCode) 的默认模型
        "glm" | "glm-global" | "zhipu" | "zhipu-global" => 
            std::env::var("LLM_MODEL").unwrap_or_else(|_| "glm-4".to_string()),
        "glm-cn" | "zhipu-cn" | "bigmodel" => 
            std::env::var("LLM_MODEL").unwrap_or_else(|_| "glm-4".to_string()),
        "zai" | "z.ai" | "zai-global" | "z.ai-global" | "glmcode" | "glmcode-global" => 
            std::env::var("LLM_MODEL").unwrap_or_else(|_| "glm-4.7".to_string()),
        "zai-cn" | "z.ai-cn" | "glmcode-cn" => 
            std::env::var("LLM_MODEL").unwrap_or_else(|_| "glm-4.7".to_string()),
        _ => std::env::var("LLM_MODEL").unwrap_or_else(|_| "glm-4".to_string()),
    }
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

/// 通用提供商凭证
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCredentials {
    /// API Key（优先从环境变量读取）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    
    /// 自定义 Base URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// GLM Provider 配置 (支持多区域)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmProviderConfig {
    /// API Key（格式: id.secret）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    
    /// 区域: china, international (默认 international)
    #[serde(default = "default_glm_region")]
    pub region: String,
    
    /// Provider 类型: glm, glmcode (默认 glm)
    #[serde(default = "default_glm_type")]
    pub provider_type: String,
    
    /// 自定义 Base URL (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn default_glm_region() -> String {
    "international".to_string()
}

fn default_glm_type() -> String {
    "glm".to_string()
}

impl Default for GlmProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            region: default_glm_region(),
            provider_type: default_glm_type(),
            base_url: None,
        }
    }
}

impl GlmProviderConfig {
    /// 获取 GLM Base URL
    pub fn get_base_url(&self) -> &'static str {
        // 先检查自定义 URL
        if let Some(ref url) = self.base_url {
            // 返回静态字符串不太现实，但这个方法主要用于获取默认 URL
            // 自定义 URL 会在创建 Provider 时直接使用
        }
        
        // 根据区域和类型返回默认 URL
        match (self.provider_type.as_str(), self.region.as_str()) {
            ("glmcode", "china") | ("glmcode-cn", _) => 
                "https://open.bigmodel.cn/api/coding/paas/v4",
            ("glmcode", _) | ("glmcode-global", _) | ("glmcode-intl", _) => 
                "https://api.z.ai/api/coding/paas/v4",
            ("glm", "china") | ("glm-cn", _) | ("bigmodel", _) => 
                "https://open.bigmodel.cn/api/paas/v4",
            ("glm", _) | ("glm-global", _) | ("glm-intl", _) => 
                "https://api.z.ai/api/paas/v4",
            _ => "https://api.z.ai/api/paas/v4", // 默认国际区域
        }
    }
}

/// Gateway 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// 监听地址
    #[serde(default = "default_host")]
    pub host: String,
    
    /// 监听端口
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// 是否启用 Watchdog 集成
    #[serde(default)]
    pub enable_watchdog: bool,
    
    /// Watchdog gRPC 地址
    #[serde(default = "default_watchdog_addr")]
    pub watchdog_addr: String,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            enable_watchdog: false,
            watchdog_addr: default_watchdog_addr(),
        }
    }
}

fn default_watchdog_addr() -> String {
    "http://127.0.0.1:50051".to_string()
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    /// 启用的工具列表
    #[serde(default = "default_enabled_tools")]
    pub enabled: Vec<String>,
    
    /// 工具执行超时（秒）
    #[serde(default = "default_tool_timeout")]
    pub timeout_secs: u64,
}

fn default_enabled_tools() -> Vec<String> {
    vec![
        "read".to_string(),
        "write".to_string(),
        "edit".to_string(),
        "exec".to_string(),
        "search".to_string(),
    ]
}

fn default_tool_timeout() -> u64 {
    60
}

impl Config {
    /// 从文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| "Failed to read config file")?;
        
        let mut config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse config file")?;
        
        // 环境变量覆盖
        config.apply_env_overrides();
        
        Ok(config)
    }
    
    /// 加载默认配置（查找 config.toml）
    pub fn load() -> Result<Self> {
        // 尝试从多个位置加载
        let config_paths = [
            "config.toml",
            ".newclaw/config.toml",
            "~/.config/newclaw/config.toml",
        ];
        
        for path in &config_paths {
            let expanded = shellexpand::tilde(path);
            if Path::new(expanded.as_ref()).exists() {
                return Self::from_file(expanded.as_ref());
            }
        }
        
        // 没有配置文件，使用默认值 + 环境变量
        let mut config = Config::default();
        config.apply_env_overrides();
        Ok(config)
    }
    
    /// 应用环境变量覆盖
    fn apply_env_overrides(&mut self) {
        // LLM Provider
        if let Ok(provider) = std::env::var("LLM_PROVIDER") {
            self.llm.provider = provider;
        }
        
        // LLM Model
        if let Ok(model) = std::env::var("LLM_MODEL") {
            self.llm.model = model;
        }
        
        // API Keys（环境变量优先）
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.llm.openai.api_key = Some(key);
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            self.llm.claude.api_key = Some(key);
        }
        if let Ok(key) = std::env::var("GLM_API_KEY") {
            self.llm.glm.api_key = Some(key);
        }
        
        // GLM 区域配置
        if let Ok(region) = std::env::var("GLM_REGION") {
            self.llm.glm.region = region;
        }
        if let Ok(provider_type) = std::env::var("GLM_TYPE") {
            self.llm.glm.provider_type = provider_type;
        }
        
        // Gateway
        if let Ok(host) = std::env::var("GATEWAY_HOST") {
            self.gateway.host = host;
        }
        if let Ok(port) = std::env::var("GATEWAY_PORT") {
            if let Ok(p) = port.parse() {
                self.gateway.port = p;
            }
        }
    }
    
    /// 获取当前提供商的 API Key
    pub fn get_api_key(&self) -> Result<String> {
        let provider_lower = self.llm.provider.to_lowercase();
        
        // 检查是否为 GLM 系列 Provider
        if is_glm_provider(&provider_lower) {
            return self.llm.glm.api_key.clone().ok_or_else(|| anyhow::anyhow!(
                "API key not found for GLM provider '{}'. Set environment variable: GLM_API_KEY",
                self.llm.provider
            ));
        }
        
        let key = match self.llm.provider.as_str() {
            "openai" => self.llm.openai.api_key.clone(),
            "claude" => self.llm.claude.api_key.clone(),
            "qwencode" => {
                // 从环境变量或配置文件读取
                std::env::var("QWENCODE_API_KEY")
                    .or_else(|_| {
                        match self.llm.qwencode.api_key.clone() {
                            Some(key) => Ok(key),
                            None => Err(std::env::VarError::NotPresent),
                        }
                    })
                    .ok()
            },
            other => return Err(anyhow::anyhow!("Unknown provider: {}", other)),
        };
        
        key.ok_or_else(|| anyhow::anyhow!(
            "API key not found for provider '{}'. Set environment variable: {}_API_KEY",
            self.llm.provider,
            self.llm.provider.to_uppercase()
        ))
    }
    
    /// 获取默认模型
    pub fn get_model(&self) -> String {
        if !self.llm.model.is_empty() {
            return self.llm.model.clone();
        }
        
        // 根据提供商返回默认模型
        let provider_lower = self.llm.provider.to_lowercase();
        match provider_lower.as_str() {
            "openai" => "gpt-4o-mini".to_string(),
            "claude" => "claude-3-5-sonnet-20241022".to_string(),
            // GLM 系列
            "glm" | "glm-global" | "zhipu" | "zhipu-global" => "glm-4".to_string(),
            "glm-cn" | "zhipu-cn" | "bigmodel" => "glm-4".to_string(),
            // z.ai / GLMCode 系列
            "zai" | "z.ai" | "zai-global" | "z.ai-global" | "glmcode" | "glmcode-global" => "glm-4.7".to_string(),
            "zai-cn" | "z.ai-cn" | "glmcode-cn" => "glm-4.7".to_string(),
            _ => "glm-4".to_string(),
        }
    }
    
    /// 获取 GLM 配置（解析 provider 名称）
    pub fn get_glm_config(&self) -> GlmProviderConfig {
        let provider_lower = self.llm.provider.to_lowercase();
        
        // 从 provider 名称推断区域和类型
        let (region, provider_type) = parse_glm_provider_name(&provider_lower);
        
        GlmProviderConfig {
            api_key: self.llm.glm.api_key.clone(),
            region: if self.llm.glm.region.clone().is_empty() { region.to_string() } else { self.llm.glm.region.clone() },
            provider_type: if self.llm.glm.provider_type.clone().is_empty() { provider_type.to_string() } else { self.llm.glm.provider_type.clone() },
            base_url: self.llm.glm.base_url.clone(),
        }
    }
}

/// 检查是否为 GLM 系列 Provider
fn is_glm_provider(name: &str) -> bool {
    matches!(
        name,
        "glm" | "glm-global" | "glm-cn" | "glm-intl" |
        "zhipu" | "zhipu-global" | "zhipu-cn" |
        "bigmodel" |
        "zai" | "z.ai" | "zai-global" | "zai-cn" | "z.ai-global" | "z.ai-cn" |
        "glmcode" | "glmcode-global" | "glmcode-cn" | "glmcode-intl"
    )
}

/// 从 provider 名称解析区域和类型
fn parse_glm_provider_name(name: &str) -> (&'static str, &'static str) {
    match name {
        // GLM 中国
        "glm-cn" | "zhipu-cn" | "bigmodel" => ("china", "glm"),
        // GLM 国际
        "glm" | "glm-global" | "glm-intl" | "zhipu" | "zhipu-global" => ("international", "glm"),
        // GLMCode / z.ai 中国
        "zai-cn" | "z.ai-cn" | "glmcode-cn" | "glmcode-china" => ("china", "glmcode"),
        // GLMCode / z.ai 国际
        "zai" | "z.ai" | "zai-global" | "z.ai-global" | "glmcode" | "glmcode-global" | "glmcode-intl" => ("international", "glmcode"),
        // 默认
        _ => ("international", "glm"),
    }
}


/// 飞书配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeishuConfig {
    /// 飞书账号配置
    #[serde(default)]
    pub accounts: std::collections::HashMap<String, FeishuAccount>,
}

/// 飞书账号
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuAccount {
    /// 应用 ID
    pub app_id: String,
    
    /// 应用密钥
    pub app_secret: String,
    
    /// 加密密钥
    #[serde(default)]
    pub encrypt_key: String,
    
    /// 验证令牌
    #[serde(default)]
    pub verification_token: String,
    
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// 连接模式：websocket, longpoll
    #[serde(default = "default_connection_mode")]
    pub connection_mode: String,
    
    /// 域名
    #[serde(default)]
    pub domain: String,
    
    /// 飞书访问令牌（从飞书 API 获取，自动管理）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    
    /// 令牌过期时间（Unix 时间戳）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_expires_at: Option<i64>,
    
    /// WebSocket URL（从飞书 API 获取，自动管理）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub websocket_url: Option<String>,
}

fn default_enabled() -> bool {
    true
}

fn default_connection_mode() -> String {
    "websocket".to_string()
}

impl Default for FeishuAccount {
    fn default() -> Self {
        Self {
            app_id: String::new(),
            app_secret: String::new(),
            encrypt_key: String::new(),
            verification_token: String::new(),
            enabled: true,
            connection_mode: "websocket".to_string(),
            domain: "feishu".to_string(),
            access_token: None,
            token_expires_at: None,
            websocket_url: None,
        }
    }
}

/// 生成示例配置文件
pub fn generate_example_config() -> String {
    let config = Config {
        llm: LLMConfig {
            provider: "glm".to_string(),
            model: "glm-4".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            openai: ProviderCredentials {
                api_key: Some("sk-...".to_string()),
                base_url: None,
            },
            claude: ProviderCredentials {
                api_key: Some("sk-ant-...".to_string()),
                base_url: None,
            },
            glm: GlmProviderConfig {
                api_key: Some("your-id.your-secret".to_string()),
                region: "international".to_string(),
                provider_type: "glm".to_string(),
                base_url: None,
            },
            qwencode: ProviderCredentials {
                api_key: Some("sk-...".to_string()),
                base_url: None,
            },
        },
        gateway: GatewayConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            enable_watchdog: false,
            watchdog_addr: "http://127.0.0.1:50051".to_string(),
        },
        tools: ToolsConfig {
            enabled: vec![
                "read".to_string(),
                "write".to_string(),
                "edit".to_string(),
                "exec".to_string(),
                "search".to_string(),
            ],
            timeout_secs: 60,
        },
        feishu: FeishuConfig::default(),
    };
    
    toml::to_string_pretty(&config).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.llm.temperature, 0.7);
        assert_eq!(config.llm.max_tokens, 4096);
    }
    
    #[test]
    fn test_glm_provider_detection() {
        assert!(is_glm_provider("glm"));
        assert!(is_glm_provider("glm-cn"));
        assert!(is_glm_provider("z.ai"));
        assert!(is_glm_provider("glmcode"));
        assert!(!is_glm_provider("openai"));
    }
    
    #[test]
    fn test_parse_glm_provider_name() {
        assert_eq!(parse_glm_provider_name("glm"), ("international", "glm"));
        assert_eq!(parse_glm_provider_name("glm-cn"), ("china", "glm"));
        assert_eq!(parse_glm_provider_name("z.ai"), ("international", "glmcode"));
        assert_eq!(parse_glm_provider_name("zai-cn"), ("china", "glmcode"));
    }
    
    #[test]
    fn test_env_override() {
        std::env::set_var("LLM_PROVIDER", "openai");
        std::env::set_var("OPENAI_API_KEY", "test-key");
        
        let config = Config::load().unwrap();
        assert_eq!(config.llm.provider, "openai");
        assert_eq!(config.llm.openai.api_key, Some("test-key".to_string()));
        
        std::env::remove_var("LLM_PROVIDER");
        std::env::remove_var("OPENAI_API_KEY");
    }
}
