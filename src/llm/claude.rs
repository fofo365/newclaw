// NewClaw v0.3.0 - Claude (Anthropic) Provider 实现
//
// 支持的模型：
// - Claude 3.5 Sonnet
// - Claude 3 Opus
// - Claude 3 Haiku

use super::provider::{LLMProviderV3 as LLMProvider, ChatRequest, ChatResponse, Message, MessageRole, TokenUsage, LLMError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

/// Claude Provider
pub struct ClaudeProvider {
    api_key: String,
    base_url: String,
    client: Client,
    default_model: String,
}

impl ClaudeProvider {
    /// 创建新的 Claude Provider
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap(),
            default_model: "claude-3-5-sonnet-20241022".to_string(),
        }
    }
    
    /// 使用自定义 base URL
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
    
    /// 设置默认模型
    pub fn with_default_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }
    
    /// 转换消息格式
    fn convert_request(&self, req: ChatRequest) -> ClaudeRequest {
        // Claude 需要分离 system 消息
        let (system_messages, user_messages): (Vec<_>, Vec<_>) = req.messages
            .into_iter()
            .partition(|m| m.role == MessageRole::System);
        
        let system = system_messages
            .into_iter()
            .map(|m| m.content)
            .collect::<Vec<_>>()
            .join("\n\n");
        
        ClaudeRequest {
            model: req.model,
            messages: user_messages.into_iter().map(|m| ClaudeMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: m.content,
            }).collect(),
            system: if system.is_empty() { None } else { Some(system) },
            max_tokens: req.max_tokens.unwrap_or(4096),
            temperature: Some(req.temperature),
            top_p: req.top_p,
            stop_sequences: req.stop,
        }
    }
    
    /// 转换响应格式
    fn convert_response(&self, resp: ClaudeResponse, model: String) -> ChatResponse {
        ChatResponse {
            message: Message {
                role: MessageRole::Assistant,
                content: resp.content.first().map(|c| match c {
                    ClaudeContentType::Text { text } => text.clone(),
                    _ => String::new(),
                }).unwrap_or_default(),
                tool_calls: None,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            },
            finish_reason: Some(resp.stop_reason.clone()),
            model,
        }
    }
}

#[async_trait]
impl LLMProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }
    
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LLMError> {
        let claude_req = self.convert_request(req);
        
        let resp = self.client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&claude_req)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;
        
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(LLMError::AuthError(body));
            } else if status.as_u16() == 429 {
                return Err(LLMError::RateLimitError);
            }
            return Err(LLMError::ApiError(body));
        }
        
        let claude_resp: ClaudeResponse = serde_json::from_str(&body)
            .map_err(|e| LLMError::SerializationError(e.to_string()))?;
        
        Ok(self.convert_response(claude_resp, claude_req.model))
    }
    
    async fn chat_stream(
        &self,
        _req: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<String, LLMError>> + Send>>, LLMError> {
        // TODO: 实现 SSE 流式响应
        Err(LLMError::ApiError("Streaming not implemented yet".to_string()))
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        // Claude 使用类似的 token 计算方式
        let chinese_chars = text.chars().filter(|c| {
            let cp = *c as u32;
            (0x4E00..=0x9FFF).contains(&cp) ||
            (0x3400..=0x4DBF).contains(&cp) ||
            (0x20000..=0x2A6DF).contains(&cp)
        }).count();
        
        let total_chars = text.chars().count();
        let other_chars = total_chars - chinese_chars;
        
        let tokens = (chinese_chars / 2) + (other_chars / 4);
        std::cmp::max(1, tokens)
    }
    
    async fn validate(&self) -> Result<bool, LLMError> {
        let test_req = ChatRequest {
            messages: vec![
                Message {
                    role: MessageRole::System,
                    content: "You are a helpful assistant.".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                },
                Message {
                    role: MessageRole::User,
                    content: "test".to_string(),
                    tool_calls: None,
                    tool_call_id: None,
                }
            ],
            model: self.default_model.clone(),
            temperature: 0.0,
            max_tokens: Some(10),
            top_p: None,
            stop: None,
            tools: None,
        };
        
        match self.chat(test_req).await {
            Ok(_) => Ok(true),
            Err(LLMError::AuthError(_)) => Ok(false),
            Err(_) => Ok(true),
        }
    }
}

// Claude API 类型定义

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    r#type: String,
    role: String,
    content: Vec<ClaudeContentType>,
    model: String,
    stop_reason: String,
    usage: ClaudeUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClaudeContentType {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String },
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: usize,
    output_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_claude_provider_creation() {
        let provider = ClaudeProvider::new("test-key".to_string());
        assert_eq!(provider.name(), "claude");
        assert_eq!(provider.default_model, "claude-3-5-sonnet-20241022");
    }
    
    #[test]
    fn test_token_count() {
        let provider = ClaudeProvider::new("test-key".to_string());
        
        let chinese = "你好世界";
        let english = "Hello World";
        
        assert_eq!(provider.count_tokens(chinese), 2);
        assert_eq!(provider.count_tokens(english), 2);
    }
}
