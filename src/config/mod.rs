// NewClaw v0.3.1 - 配置管理
//
// 支持：
// 1. TOML 配置文件
// 2. 环境变量覆盖
// 3. 默认值

use serde::{Deserialize, Serialize};
use std::path::Path;
use anyhow::{Result, Context};

/// 主配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub llm: LLMConfig,
    
    #[serde(default)]
    pub gateway: GatewayConfig,
    
    #[serde(default)]
    pub tools: ToolsConfig,
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// 提供商: openai, claude, glm
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
    
    /// GLM 配置
    #[serde(default)]
    pub glm: ProviderCredentials,
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
            glm: ProviderCredentials::default(),
        }
    }
}

fn default_provider() -> String {
    std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "glm".to_string())
}

fn default_model() -> String {
    std::env::var("LLM_MODEL").unwrap_or_else(|_| "glm-4".to_string())
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

/// 提供商凭证
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCredentials {
    /// API Key（优先从环境变量读取）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    
    /// 自定义 Base URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

/// Gateway 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayConfig {
    /// 监听地址
    #[serde(default = "default_host")]
    pub host: String,
    
    /// 监听端口
    #[serde(default = "default_port")]
    pub port: u16,
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
        let key = match self.llm.provider.as_str() {
            "openai" => self.llm.openai.api_key.clone(),
            "claude" => self.llm.claude.api_key.clone(),
            "glm" => self.llm.glm.api_key.clone(),
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
        if self.llm.model.is_empty() {
            // 根据提供商返回默认模型
            match self.llm.provider.as_str() {
                "openai" => "gpt-4o-mini".to_string(),
                "claude" => "claude-3-5-sonnet-20241022".to_string(),
                "glm" => "glm-4".to_string(),
                _ => "glm-4".to_string(),
            }
        } else {
            self.llm.model.clone()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            llm: LLMConfig::default(),
            gateway: GatewayConfig::default(),
            tools: ToolsConfig::default(),
        }
    }
}

/// 生成示例配置文件
pub fn generate_example_config() -> String {
    let config = Config {
        llm: LLMConfig {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
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
            glm: ProviderCredentials {
                api_key: Some("...".to_string()),
                base_url: None,
            },
        },
        gateway: GatewayConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
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
    };
    
    toml::to_string_pretty(&config).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        // Check that LLMConfig has proper defaults
        assert_eq!(config.llm.temperature, 0.7); // default_temperature
        assert_eq!(config.llm.max_tokens, 4096); // default_max_tokens
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
