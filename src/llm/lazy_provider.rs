// LLM Provider 懒加载包装器
// 解决启动时强制要求 API Key 的问题

use std::sync::Arc;
use tokio::sync::OnceCell;
use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::config::Config;
use crate::llm::{LLMProviderV3, ChatRequest, ChatResponse, LLMError, create_glm_provider};

/// 懒加载 LLM Provider
/// 
/// 允许 NewClaw 在没有配置 API Key 的情况下启动
/// 实际调用 LLM 时才会检查和加载 Provider
pub struct LazyLLMProvider {
    config: Config,
    inner: OnceCell<Arc<Box<dyn LLMProviderV3>>>,
}

impl LazyLLMProvider {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            inner: OnceCell::new(),
        }
    }
    
    /// 获取或初始化 Provider
    pub async fn get_or_init(&self) -> Result<Arc<Box<dyn LLMProviderV3>>, LLMError> {
        self.inner.get_or_try_init(|| async {
            // 延迟到第一次调用时才检查 API Key
            match self.config.get_api_key() {
                Ok(api_key) => {
                    let model_name = self.config.get_model();
                    let provider = create_glm_provider(api_key, &model_name);
                    tracing::info!(
                        "✅ LLM Provider initialized: {} (model: {})", 
                        self.config.llm.provider, 
                        model_name
                    );
                    // 显式转换为 trait object
                    let boxed: Box<dyn LLMProviderV3> = Box::new(provider);
                    Ok(Arc::new(boxed))
                }
                Err(e) => {
                    tracing::error!("❌ Failed to initialize LLM Provider: {}", e);
                    Err(LLMError::ApiError(format!(
                        "API Key not configured. Please set {}_API_KEY environment variable or configure in newclaw.toml",
                        self.config.llm.provider.to_uppercase()
                    )))
                }
            }
        }).await.cloned()
    }
    
    /// 检查是否已配置 API Key（用于健康检查）
    pub fn is_configured(&self) -> bool {
        self.config.get_api_key().is_ok()
    }
    
    /// 获取配置的 Provider 名称
    pub fn provider_name(&self) -> &str {
        &self.config.llm.provider
    }
    
    /// 获取配置的模型名称
    pub fn model_name(&self) -> String {
        self.config.get_model()
    }
}

#[async_trait]
impl LLMProviderV3 for LazyLLMProvider {
    fn name(&self) -> &str {
        &self.config.llm.provider
    }
    
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LLMError> {
        let provider = self.get_or_init().await?;
        provider.chat(req).await
    }
    
    async fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError> {
        let provider = self.get_or_init().await?;
        provider.chat_stream(req).await
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        // 使用简单估算（~4 字符/token）
        text.len() / 4
    }
    
    async fn validate(&self) -> Result<bool, LLMError> {
        match self.get_or_init().await {
            Ok(provider) => provider.validate().await,
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lazy_provider_creation() {
        let config = Config::default();
        let _lazy = LazyLLMProvider::new(config);
    }
    
    #[test]
    fn test_lazy_provider_not_configured() {
        let config = Config::default();
        let lazy = LazyLLMProvider::new(config);
        
        // 未配置 API Key 时，is_configured 应该返回 false
        assert!(!lazy.is_configured());
    }
    
    #[test]
    fn test_lazy_provider_name() {
        let config = Config::default();
        let lazy = LazyLLMProvider::new(config);
        
        // 默认 Provider 应该是 glm
        assert_eq!(lazy.provider_name(), "glm");
    }
}