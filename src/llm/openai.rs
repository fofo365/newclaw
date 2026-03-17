// NewClaw v0.3.0 - OpenAI Provider 实现
//
// 支持的模型：
// - GPT-4o
// - GPT-4o-mini
// - GPT-3.5-turbo
// - o1-preview (limited)

use super::provider::{LLMProviderV3 as LLMProvider, ChatRequest, ChatResponse, Message, MessageRole, TokenUsage, LLMError, ToolDefinition, ToolCall};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::Stream;
use std::time::Duration;

/// OpenAI Provider
pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    client: Client,
    default_model: String,
}

impl OpenAIProvider {
    /// 创建新的 OpenAI Provider
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap(),
            default_model: "gpt-4o-mini".to_string(),
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
    fn convert_request(&self, req: ChatRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = req.messages.into_iter().map(|m| {
            serde_json::json!({
                "role": match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                },
                "content": m.content,
            })
        }).collect();
        
        // 基本请求
        serde_json::json!({
            "model": req.model,
            "messages": messages,
            "temperature": req.temperature,
        })
    }
    
    /// 转换响应格式
    fn convert_response(&self, resp: OpenAIResponse) -> ChatResponse {
        let tool_calls = if let Some(choice) = resp.choices.first() {
            if !choice.message.tool_calls.is_empty() {
                Some(choice.message.tool_calls.iter().map(|t| ToolCall {
                    id: t.id.clone(),
                    name: t.function.name.clone(),
                    arguments: t.function.arguments.clone(),
                }).collect())
            } else {
                None
            }
        } else {
            None
        };
        
        ChatResponse {
            message: Message {
                role: MessageRole::Assistant,
                content: resp.choices.first().map(|c| c.message.content.clone()).unwrap_or_default(),
                tool_calls,
                tool_call_id: None,
            },
            usage: TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
            finish_reason: resp.choices.first().map(|c| c.finish_reason.clone()),
            model: resp.model,
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }
    
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LLMError> {
        let openai_req_json = self.convert_request(req);
        tracing::info!("Sending request to {}: {}", self.base_url, openai_req_json);
        
        let resp = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("User-Agent", "newclaw/0.7.0")
            .json(&openai_req_json)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(e.to_string()))?;
        
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::info!("Response status: {}, body: {}", status, body);
        
        if !status.is_success() {
            if status.as_u16() == 401 {
                return Err(LLMError::AuthError(body));
            } else if status.as_u16() == 429 {
                return Err(LLMError::RateLimitError);
            }
            return Err(LLMError::ApiError(body));
        }
        
        let openai_resp: OpenAIResponse = serde_json::from_str(&body)
            .map_err(|e| LLMError::SerializationError(e.to_string()))?;
        
        Ok(self.convert_response(openai_resp))
    }
    
    async fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>, LLMError> {
        // TODO: 实现 SSE 流式响应
        Err(LLMError::ApiError("Streaming not implemented yet".to_string()))
    }
    
    fn count_tokens(&self, text: &str) -> usize {
        // 简单估算：英文约 4 字符/token，中文约 2 字符/token
        // 但最小返回 1
        let chinese_chars = text.chars().filter(|c| {
            let cp = *c as u32;
            (0x4E00..=0x9FFF).contains(&cp) || // CJK Unified Ideographs
            (0x3400..=0x4DBF).contains(&cp) || // CJK Extension A
            (0x20000..=0x2A6DF).contains(&cp) // CJK Extension B
        }).count();
        
        let total_chars = text.chars().count();
        let other_chars = total_chars - chinese_chars;
        
        let tokens = (chinese_chars / 2) + (other_chars / 4);
        std::cmp::max(1, tokens)
    }
    
    async fn validate(&self) -> Result<bool, LLMError> {
        let test_req = ChatRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "test".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
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
            Err(_) => Ok(true), // 其他错误也算 key 有效
        }
    }
}

// OpenAI API 类型定义

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    tools: Option<Vec<OpenAITool>>,
}

// coding.dashscope 专用请求（带coding字段）
#[derive(Debug, Serialize)]
struct CodingDashScopeRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: Option<f32>,
    max_tokens: Option<usize>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    tools: Option<Vec<OpenAITool>>,
    coding: bool,
}

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    r#type: String,
    function: OpenAIFunctionCall,
}

impl From<ToolCall> for OpenAIToolCall {
    fn from(tc: ToolCall) -> Self {
        Self {
            id: tc.id,
            r#type: "function".to_string(),
            function: OpenAIFunctionCall {
                name: tc.name,
                arguments: tc.arguments,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OpenAITool {
    r#type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: usize,
    message: OpenAIResponseMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    content: String,
    #[serde(default)]
    tool_calls: Vec<OpenAIToolCall>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAIProvider::new("test-key".to_string());
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.default_model, "gpt-4o-mini");
    }
    
    #[test]
    fn test_token_count() {
        let provider = OpenAIProvider::new("test-key".to_string());
        
        let chinese = "你好世界";
        let english = "Hello World";
        let mixed = "你好World";
        
        // 中文：4 字符 / 2 = 2 tokens
        assert_eq!(provider.count_tokens(chinese), 2);
        // 英文：11 字符 / 4 = 2 tokens (round down + max(1, ...))
        assert_eq!(provider.count_tokens(english), 2);
        // 混合：2 中文 / 2 = 1 + 5 英文 / 4 = 1 → 2 tokens
        assert_eq!(provider.count_tokens(mixed), 2);
    }
}
