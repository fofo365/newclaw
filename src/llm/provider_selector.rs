// Provider 选择器
//
// 统一的 LLM Provider 创建逻辑，避免 CLI 和 feishu-connect 代码重复

use std::sync::Arc;
use anyhow::Result;
use tracing::info;

use crate::config::Config;
use crate::llm::{
    LLMProviderV3, OpenAIProvider, QwenCodeProvider, ClaudeProvider,
    GlmProvider, GlmConfig, GlmRegion, GlmProviderType,
};

/// 创建 LLM Provider
///
/// 根据配置选择合适的 Provider，支持：
/// - openai: OpenAI GPT 模型
/// - qwencode: QwenCode (coding.dashscope.aliyuncs.com)
/// - claude: Anthropic Claude 模型
/// - glm/zhipu: 智谱 GLM 模型
pub fn create_llm_provider(config: &Config) -> Result<Arc<dyn LLMProviderV3>> {
    let provider = config.llm.provider.to_lowercase();
    let model = config.llm.model.clone();
    
    let provider: Arc<dyn LLMProviderV3> = match provider.as_str() {
        "openai" => {
            let api_key = config.llm.openai.api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .unwrap_or_default();
            
            let mut p = OpenAIProvider::new(api_key);
            if let Some(base_url) = &config.llm.openai.base_url {
                p = p.with_base_url(base_url.clone());
                info!("✅ 使用 OpenAI Provider (base_url: {})", base_url);
            } else {
                info!("✅ 使用 OpenAI Provider");
            }
            p = p.with_default_model(model);
            Arc::new(p)
        }
        
        "qwencode" => {
            let api_key = config.llm.qwencode.api_key.clone()
                .or_else(|| std::env::var("QWENCODE_API_KEY").ok())
                .unwrap_or_default();
            
            let mut p = QwenCodeProvider::new(api_key);
            if let Some(base_url) = &config.llm.qwencode.base_url {
                p = p.with_base_url(base_url.clone());
                info!("✅ 使用 QwenCode Provider (base_url: {})", base_url);
            } else {
                info!("✅ 使用 QwenCode Provider");
            }
            p = p.with_default_model(model);
            Arc::new(p)
        }
        
        "claude" => {
            let api_key = config.llm.claude.api_key.clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                .unwrap_or_default();
            
            let mut p = ClaudeProvider::new(api_key);
            if let Some(base_url) = &config.llm.claude.base_url {
                p = p.with_base_url(base_url.clone());
                info!("✅ 使用 Claude Provider (base_url: {})", base_url);
            } else {
                info!("✅ 使用 Claude Provider");
            }
            p = p.with_default_model(model);
            Arc::new(p)
        }
        
        "glm" | "zhipu" | "z.ai" | "glmcode" => {
            let api_key = config.llm.glm.api_key.clone()
                .or_else(|| std::env::var("GLM_API_KEY").ok())
                .unwrap_or_default();
            
            // 根据模型名称判断 Provider 类型
            let provider_type = if model.contains("code") || provider.contains("code") {
                GlmProviderType::GlmCode
            } else {
                GlmProviderType::Glm
            };
            
            let glm_config = GlmConfig {
                region: GlmRegion::International,
                provider_type,
                model: model.clone(),
                temperature: 0.7,
                max_tokens: 4096,
            };
            
            info!("✅ 使用 GLM Provider (type: {:?}, model: {})", provider_type, model);
            Arc::new(GlmProvider::with_config(api_key, glm_config))
        }
        
        _ => {
            // 默认使用 GLM
            let api_key = config.llm.glm.api_key.clone()
                .or_else(|| std::env::var("GLM_API_KEY").ok())
                .unwrap_or_default();
            
            info!("⚠️ 未知 Provider: {}, 使用 GLM", provider);
            Arc::new(GlmProvider::new(api_key))
        }
    };
    
    Ok(provider)
}