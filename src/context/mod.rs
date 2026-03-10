// Context Manager Module - v0.5.0
//
// 智能上下文管理系统：
// - Token 实时估算
// - 智能截断优化
// - 向量化存储（v0.5.0）
// - 智能检索 RAG（v0.5.0）

pub mod token_counter;
pub mod truncation;
pub mod strategy;

pub use token_counter::{TokenCounter, TokenUsageStats};
pub use truncation::{TruncationStrategy, TruncationConfig};
pub use strategy::{StrategyEngine, StrategyType};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 上下文管理器
pub struct ContextManager {
    /// Token 计数器
    token_counter: Arc<RwLock<TokenCounter>>,
    /// 截断策略
    truncation_strategy: Arc<RwLock<TruncationStrategy>>,
    /// 策略引擎
    strategy_engine: Arc<RwLock<StrategyEngine>>,
}

impl ContextManager {
    /// 创建新的上下文管理器
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            token_counter: Arc::new(RwLock::new(TokenCounter::new()?)),
            truncation_strategy: Arc::new(RwLock::new(TruncationStrategy::default())),
            strategy_engine: Arc::new(RwLock::new(StrategyEngine::new()?)),
        })
    }

    /// 计算消息的 token 数量
    pub async fn count_tokens(
        &self,
        messages: &Vec<crate::llm::Message>,
        model: &str,
    ) -> anyhow::Result<usize> {
        let mut counter = self.token_counter.write().await;
        counter.count_messages_tokens(messages, model)
    }

    /// 智能截断消息
    ///
    /// 根据策略自动截断消息，保留最重要的内容
    pub async fn truncate_messages(
        &self,
        messages: &Vec<crate::llm::Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<crate::llm::Message>> {
        let mut strategy = self.truncation_strategy.write().await;
        strategy.truncate(messages, max_tokens, model).await
    }

    /// 估算 token 使用
    pub async fn estimate_usage(
        &self,
        messages: &Vec<crate::llm::Message>,
        model: &str,
    ) -> anyhow::Result<TokenUsageStats> {
        let mut counter = self.token_counter.write().await;
        let input_tokens = counter.count_messages_tokens(messages, model)?;
        let output_tokens = counter.estimate_output_tokens(input_tokens, model)?;

        TokenUsageStats::new(input_tokens, output_tokens, model, &mut counter)
    }

    /// 应用策略
    pub async fn apply_strategy(
        &self,
        messages: &Vec<crate::llm::Message>,
        strategy: StrategyType,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<crate::llm::Message>> {
        let mut engine = self.strategy_engine.write().await;
        engine.apply(messages, strategy, max_tokens, model).await
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ContextManager")
    }
}

/// 上下文配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// 最大 token 数量
    pub max_tokens: usize,
    /// 默认策略
    pub default_strategy: StrategyType,
    /// 是否启用智能截断
    pub enable_smart_truncation: bool,
    /// Token 缓冲（保留的空间）
    pub token_buffer: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            default_strategy: StrategyType::Balanced,
            enable_smart_truncation: true,
            token_buffer: 512,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_manager() {
        let manager = ContextManager::new().unwrap();

        let messages = vec![
            crate::llm::Message {
                role: crate::llm::MessageRole::User,
                content: "Hello, world!".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let count = manager.count_tokens(&messages, "gpt-4").await.unwrap();
        assert!(count > 0);
    }
}
