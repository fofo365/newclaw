// LLM Integration Module

// v0.3.0 - 新的多 LLM 架构
pub mod provider;
pub mod openai;
pub mod claude;
pub mod streaming;

// Re-exports
pub use provider::{LLMProviderV3, ChatRequest, ChatResponse, Message, MessageRole, LLMError, ModelStrategy, LLMConfig, ProviderType};
pub use openai::OpenAIProvider;
pub use claude::ClaudeProvider;
pub use streaming::{StreamChunk, StreamingResponse, SSEEvent, stream_llm_response, WebSocketStream, FeishuStreamAdapter};

// 向后兼容：旧的 LLMProvider 导出
pub use GLMProvider as LLMProvider;

// Legacy v0.2.0 LLM 接口（保留向后兼容）
use async_trait::async_trait;
use anyhow::Result;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<LLMMessage>,
    pub temperature: f32,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub tokens_used: usize,
    pub model: String,
}

/// Legacy LLM Provider trait (v0.2.0)
#[async_trait]
pub trait LegacyLLMProvider: Send + Sync {
    fn name(&self) -> &str;
    
    async fn chat(&self, request: &LLMRequest) -> Result<LLMResponse>;
    
    async fn stream_chat(&self, _request: &LLMRequest) -> Result<String> {
        Err(anyhow::anyhow!("Streaming not implemented for this provider"))
    }
}

/// GLM Provider (v0.2.0 实现，保留向后兼容)
pub struct GLMProvider {
    #[allow(dead_code)]
    api_key: String,
    base_url: String,
}

impl GLMProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string(),
        }
    }
    
    pub fn with_url(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }
}

#[async_trait]
impl LegacyLLMProvider for GLMProvider {
    fn name(&self) -> &str {
        "glm"
    }
    
    async fn chat(&self, request: &LLMRequest) -> Result<LLMResponse> {
        let client = reqwest::Client::new();
        
        let request_body = serde_json::json!({
            "model": request.model,
            "messages": request.messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });
        
        let response = client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request_body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("GLM API error: {}", error_text));
        }
        
        let json: serde_json::Value = response.json().await?;
        
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let tokens_used = json["usage"]["total_tokens"]
            .as_u64()
            .unwrap_or(0) as usize;
        
        Ok(LLMResponse {
            content,
            tokens_used,
            model: request.model.clone(),
        })
    }
}
