// Configuration management

use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewClawConfig {
    pub agent: AgentConfig,
    pub llm: LLMConfig,
    pub channels: ChannelsConfig,
    pub context: ContextConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelsConfig {
    pub feishu: Option<FeishuChannelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuChannelConfig {
    pub enabled: bool,
    pub app_id: String,
    pub app_secret: String,
    pub encrypt_key: Option<String>,
    pub verification_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_chunks: usize,
    pub max_tokens: usize,
    pub overlap_tokens: usize,
}

impl Default for NewClawConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig {
                name: "NewClaw".to_string(),
                model: "glm-4".to_string(),
            },
            llm: LLMConfig {
                provider: "glm".to_string(),
                api_key: None,
                base_url: None,
                temperature: 0.7,
                max_tokens: Some(2000),
            },
            channels: ChannelsConfig {
                feishu: None,
            },
            context: ContextConfig {
                max_chunks: 100,
                max_tokens: 8000,
                overlap_tokens: 200,
            },
        }
    }
}

impl NewClawConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: NewClawConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save(&self, path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(self,)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    pub fn from_env() -> Self {
        let mut config = Self::default();
        
        if let Ok(api_key) = std::env::var("GLM_API_KEY") {
            config.llm.api_key = Some(api_key);
        }
        
        if let Ok(app_id) = std::env::var("FEISHU_APP_ID") {
            if let Ok(app_secret) = std::env::var("FEISHU_APP_SECRET") {
                config.channels.feishu = Some(FeishuChannelConfig {
                    enabled: true,
                    app_id,
                    app_secret,
                    encrypt_key: std::env::var("FEISHU_ENCRYPT_KEY").ok(),
                    verification_token: std::env::var("FEISHU_VERIFICATION_TOKEN").ok(),
                });
            }
        }
        
        config
    }
}
