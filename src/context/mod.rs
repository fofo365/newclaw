// Context Manager Module - v0.7.0
//
// 智能上下文管理系统：
// - Layer 0: Ephemeral Context (瞬时层) - v0.7.0 新增
// - Token 实时估算
// - 智能截断优化
// - 策略引擎
// - RAG 检索
//
// 注意：ContextManager 的主要实现在 src/core/context.rs
// 这个模块专注于 Token 计数、截断策略和策略引擎

// Layer 0: Ephemeral Context (瞬时层) - v0.7.0
pub mod ephemeral;

pub mod token_counter;
pub mod truncation;
pub mod strategy;

// v0.5.2 - RAG 检索
pub mod retrieval;

// v0.5.2 - 上下文压缩
pub mod compression;

// v0.5.3 - 策略管理
pub mod policy;

// v0.5.3 - 透明管理
pub mod transparency;

// v0.5.3 - 配置管理
pub mod config;

// Layer 0: Ephemeral Context 导出 - v0.7.0
pub use ephemeral::{EphemeralContext, TokenBudget, AdaptiveAllocator, EphemeralStats};

pub use token_counter::{TokenCounter, TokenUsageStats};
pub use truncation::{TruncationStrategy, TruncationConfig};
pub use strategy::{StrategyEngine, StrategyType};

// v0.5.2 - RAG 导出
pub use retrieval::{RetrievalConfig, RetrievalResult, RAGContextBuilder, HybridRetriever, Citation};

// v0.5.2 - 压缩导出
pub use compression::{CompressionConfig, CompressionResult, ContextCompressor, Summarizer, BasicSummarizer};

// 重新导出核心的 ContextManager
pub use crate::core::context::{ContextManager, ContextConfig, ContextChunk};

use serde::{Deserialize, Serialize};

/// 上下文统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextStats {
    /// 总消息数
    pub total_messages: usize,
    /// 总 token 数
    pub total_tokens: usize,
    /// 缓存命中率
    pub cache_hit_rate: f64,
    /// 平均嵌入延迟
    pub avg_embedding_latency: u64,
    /// 最后更新时间
    pub last_updated: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter() {
        let mut counter = TokenCounter::new().unwrap();
        let text = "Hello, world!";
        let tokens = counter.count_tokens(text, "gpt-4").unwrap();
        assert!(tokens > 0);
    }

    #[test]
    fn test_truncation_strategy() {
        let strategy = TruncationStrategy::default();
        // Test that we can create a strategy
        // Just check it exists
        assert!(true);
    }

    #[test]
    fn test_strategy_engine() {
        let engine = StrategyEngine::new().unwrap();
        // Test that we can create an engine
        let strategies = engine.list_strategies();
        assert_eq!(strategies.len(), 6);
    }
}
