// NewClaw v0.3.0 - 多 LLM 支持
//
// 核心设计：
// 1. 统一的 LLMProvider trait 抽象
// 2. 支持多个提供商（GLM、OpenAI、Claude）
// 3. 模型切换策略
// 4. 配置文件支持

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::collections::HashMap;

/// 新版 LLM 提供商 trait (v0.3.0)
#[async_trait]
pub trait LLMProviderV3: Send + Sync {
    /// 提供商名称
    fn name(&self) -> &str;
    
    /// 发送聊天请求
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, LLMError>;
    
    /// 发送流式聊天请求
    async fn chat_stream(
        &self,
        req: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<String, LLMError>> + Send>>, LLMError>;
    
    /// 计算 token 数量
    fn count_tokens(&self, text: &str) -> usize;
    
    /// 验证 API Key 是否有效
    async fn validate(&self) -> Result<bool, LLMError>;
}

/// 聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 消息列表
    pub messages: Vec<Message>,
    
    /// 模型名称
    pub model: String,
    
    /// 温度（0-2）
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// 最大 token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    
    /// Top-p 采样
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    
    /// 停止序列
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    
    /// 工具定义（用于 Function Calling）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
}

fn default_temperature() -> f32 {
    0.7
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 角色（system/user/assistant）
    pub role: MessageRole,
    
    /// 内容
    pub content: String,
    
    /// 工具调用（assistant 消息可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    
    /// 工具调用 ID（tool 消息可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    pub name: String,
    
    /// 工具描述
    pub description: String,
    
    /// 参数 schema (JSON Schema)
    pub parameters: serde_json::Value,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用 ID
    pub id: String,
    
    /// 工具名称
    pub name: String,
    
    /// 参数（JSON 字符串）
    pub arguments: String,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 消息内容
    pub message: Message,
    
    /// 使用的 token 数
    pub usage: TokenUsage,
    
    /// 原因（如果被拒绝）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    
    /// 模型名称
    pub model: String,
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入 token 数
    pub prompt_tokens: usize,
    
    /// 输出 token 数
    pub completion_tokens: usize,
    
    /// 总 token 数
    pub total_tokens: usize,
}

/// LLM 错误
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("API error: {0}")]
    ApiError(String),
    
    #[error("Authentication failed: {0}")]
    AuthError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitError,
    
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Provider not supported: {0}")]
    UnsupportedProvider(String),
}

/// 模型切换策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStrategy {
    /// 静态模型
    Static { model: String },
    
    /// 轮询
    RoundRobin { models: Vec<String> },
    
    /// 故障降级
    Fallback { primary: String, fallback: String },
    
    /// 成本优化
    CostOptimized { cheap: String, premium: String },
    
    /// 基于任务复杂度
    Adaptive { simple: String, complex: String, threshold: usize },
}

impl ModelStrategy {
    /// 选择模型
    pub fn select(&self, task_complexity: usize) -> String {
        match self {
            ModelStrategy::Static { model } => model.clone(),
            
            ModelStrategy::RoundRobin { models } => {
                use std::sync::atomic::{AtomicUsize, Ordering};
                static INDEX: AtomicUsize = AtomicUsize::new(0);
                let idx = INDEX.fetch_add(1, Ordering::Relaxed) % models.len();
                models[idx].clone()
            }
            
            ModelStrategy::Fallback { primary, .. } => primary.clone(),
            
            ModelStrategy::CostOptimized { cheap, .. } => cheap.clone(),
            
            ModelStrategy::Adaptive { simple, complex, threshold } => {
                if task_complexity < *threshold {
                    simple.clone()
                } else {
                    complex.clone()
                }
            }
        }
    }
}

/// LLM 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    /// 提供商类型
    pub provider: ProviderType,
    
    /// API Key
    pub api_key: String,
    
    /// Base URL（可选，用于自定义端点）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    
    /// 模型配置
    pub models: ModelConfig,
    
    /// 切换策略
    #[serde(default = "default_strategy")]
    pub strategy: ModelStrategy,
}

fn default_strategy() -> ModelStrategy {
    ModelStrategy::Static { model: "gpt-4o-mini".to_string() }
}

/// 提供商类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    GLM,
    OpenAI,
    Claude,
    DeepSeek,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 默认模型
    pub default: String,
    
    /// 可用模型列表
    #[serde(default)]
    pub available: Vec<String>,
    
    /// 最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

fn default_max_tokens() -> usize {
    4096
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_model_strategy_static() {
        let strategy = ModelStrategy::Static {
            model: "gpt-4o-mini".to_string(),
        };
        
        let model = strategy.select(100);
        assert_eq!(model, "gpt-4o-mini");
    }
    
    #[test]
    fn test_model_strategy_round_robin() {
        let strategy = ModelStrategy::RoundRobin {
            models: vec!["model-1".to_string(), "model-2".to_string()],
        };
        
        let model1 = strategy.select(0);
        let model2 = strategy.select(0);
        let model3 = strategy.select(0);
        
        assert_eq!(model1, "model-1");
        assert_eq!(model2, "model-2");
        assert_eq!(model3, "model-1");
    }
    
    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            messages: vec![Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }],
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            max_tokens: Some(1000),
            top_p: None,
            stop: None,
            tools: None,
        };
        
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("Hello"));
    }
}
