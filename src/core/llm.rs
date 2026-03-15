// LLM Integration Module

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// LLM Provider trait
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: &[LLMMessage]) -> Result<LLMResponse>;
    fn name(&self) -> &str;
}

/// LLM Message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: String,
    pub content: String,
}

/// LLM Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub tokens_used: Option<usize>,
    pub model: String,
}

/// Mock LLM Provider for testing
pub struct MockLLMProvider;

#[async_trait]
impl LLMProvider for MockLLMProvider {
    async fn chat(&self, messages: &[LLMMessage]) -> Result<LLMResponse> {
        let last_message = messages.last()
            .map(|m| m.content.as_str())
            .unwrap_or("Hello");
        
        Ok(LLMResponse {
            content: format!("Mock response to: {}", last_message),
            tokens_used: Some(100),
            model: "mock".to_string(),
        })
    }
    
    fn name(&self) -> &str {
        "mock"
    }
}

/// GLM Provider (TODO: implement)
pub struct GLMProvider {
    api_key: String,
    model: String,
}

impl GLMProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
    
    pub fn get_api_key(&self) -> &str {
        &self.api_key
    }
}

#[async_trait]
impl LLMProvider for GLMProvider {
    async fn chat(&self, _messages: &[LLMMessage]) -> Result<LLMResponse> {
        // TODO: Implement GLM API call
        Ok(LLMResponse {
            content: "GLM response placeholder".to_string(),
            tokens_used: Some(150),
            model: self.model.clone(),
        })
    }
    
    fn name(&self) -> &str {
        "glm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_llm() {
        let provider = MockLLMProvider;
        let messages = vec![
            LLMMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }
        ];
        
        let result = provider.chat(&messages).await.unwrap();
        assert!(result.content.contains("Hello"));
    }
}
