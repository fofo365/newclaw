// Ollama 本地模型集成 (v0.5.5)
//
// 支持本地 Ollama 模型

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use futures::Stream;
use std::pin::Pin;

use crate::llm::{LLMProviderV3, ChatRequest, ChatResponse, Message, MessageRole, TokenUsage, LLMError};

/// Ollama 配置
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub default_model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            default_model: "llama3".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            timeout_secs: 120,
        }
    }
}

/// Ollama Provider
pub struct OllamaProvider {
    config: OllamaConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap_or_default();
        
        Self { config, client }
    }
    
    pub fn with_base_url(mut self, url: &str) -> Self {
        self.config.base_url = url.to_string();
        self
    }
    
    pub fn with_model(mut self, model: &str) -> Self {
        self.config.default_model = model.to_string();
        self
    }
    
    /// 获取可用模型列表
    pub async fn list_models(&self) -> anyhow::Result<Vec<OllamaModel>> {
        let url = format!("{}/api/tags", self.config.base_url);
        let response = self.client.get(&url).send().await?;
        let result: OllamaTagsResponse = response.json().await?;
        Ok(result.models)
    }
    
    /// 检查服务是否可用
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/api/tags", self.config.base_url);
        self.client.get(&url).send().await.is_ok()
    }
    
    fn convert_messages(&self, messages: Vec<Message>) -> Vec<OllamaMessage> {
        messages.into_iter().map(|m| OllamaMessage {
            role: match m.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "tool".to_string(),
            },
            content: m.content,
        }).collect()
    }
}

#[async_trait]
impl LLMProviderV3 for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }
    
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let url = format!("{}/api/chat", self.config.base_url);
        
        let ollama_request = OllamaChatRequest {
            model: request.model.clone(),
            messages: self.convert_messages(request.messages),
            stream: false,
            options: Some(OllamaOptions {
                temperature: Some(request.temperature),
                num_predict: request.max_tokens.map(|t| t as i32),
            }),
        };
        
        let response = self.client.post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!("Ollama API error: {}", error)));
        }
        
        let result: OllamaChatResponse = response.json().await
            .map_err(|e| LLMError::SerializationError(e.to_string()))?;
        
        Ok(ChatResponse {
            message: Message {
                role: MessageRole::Assistant,
                content: result.message.content,
                tool_calls: None,
                tool_call_id: None,
            },
            model: result.model,
            usage: TokenUsage {
                prompt_tokens: result.prompt_eval_count.unwrap_or(0) as usize,
                completion_tokens: result.eval_count.unwrap_or(0) as usize,
                total_tokens: (result.prompt_eval_count.unwrap_or(0) + result.eval_count.unwrap_or(0)) as usize,
            },
            finish_reason: Some("stop".to_string()),
        })
    }
    
    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError> {
        // TODO: 实现流式响应
        Err(LLMError::UnsupportedProvider("Streaming not implemented for Ollama".to_string()))
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        // 简单估算：平均每 4 个字符 = 1 token
        text.len() / 4
    }
    
    async fn validate(&self) -> Result<bool, LLMError> {
        Ok(self.health_check().await)
    }
}

/// Ollama 模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
}

/// Ollama 标签响应
#[derive(Debug, Serialize, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

/// Ollama 聊天请求
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: Option<OllamaOptions>,
}

/// Ollama 消息
#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Ollama 选项
#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: Option<f32>,
    num_predict: Option<i32>,
}

/// Ollama 聊天响应
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaMessage,
    prompt_eval_count: Option<u64>,
    eval_count: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_config_default() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
    }

    #[test]
    fn test_ollama_provider_new() {
        let provider = OllamaProvider::new(OllamaConfig::default());
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_count_tokens() {
        let provider = OllamaProvider::new(OllamaConfig::default());
        let tokens = provider.count_tokens("Hello world");
        assert!(tokens > 0);
    }
}
