// Truncation Strategy Module - v0.5.0
//
// 智能截断策略：
// - 智能截断：最小化 token 使用
// - 时间衰减：优先近期信息
// - 语义聚类：去重相似信息
// - 最大化信息：保留最多信息
// - 最小化 Token：最小化成本
// - 平衡模式：平衡信息和成本

use crate::context::{ContextManager, TokenCounter};
use crate::llm::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 截断策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TruncationStrategyType {
    /// 智能截断：基于相关性排序
    Smart,
    /// 时间衰减：优先近期信息
    TimeDecay,
    /// 语义聚类：去重相似信息
    SemanticCluster,
    /// 最大化信息：保留最多信息
    MaximizeInfo,
    /// 最小化 Token：最小化成本
    MinimizeTokens,
    /// 平衡模式：平衡信息和成本
    Balanced,
}

/// 截断配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncationConfig {
    /// 策略类型
    pub strategy: TruncationStrategyType,
    /// 时间衰减权重（0-1）
    pub time_decay_weight: f32,
    /// 语义相似度阈值（0-1）
    pub semantic_threshold: f32,
    /// 保留最近 N 条消息
    pub keep_recent: usize,
}

impl Default for TruncationConfig {
    fn default() -> Self {
        Self {
            strategy: TruncationStrategyType::Balanced,
            time_decay_weight: 0.7,
            semantic_threshold: 0.85,
            keep_recent: 10,
        }
    }
}

/// 截断策略实现
#[derive(Debug)]
pub struct TruncationStrategy {
    config: TruncationConfig,
    token_counter: TokenCounter,
}

impl TruncationStrategy {
    /// 创建新的截断策略
    pub fn new(config: TruncationConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            token_counter: TokenCounter::new()?,
        })
    }

    /// 截断消息
    pub async fn truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        match self.config.strategy {
            TruncationStrategyType::Smart => self.smart_truncate(messages, max_tokens, model).await,
            TruncationStrategyType::TimeDecay => self.time_decay_truncate(messages, max_tokens, model).await,
            TruncationStrategyType::SemanticCluster => self.semantic_cluster_truncate(messages, max_tokens, model).await,
            TruncationStrategyType::MaximizeInfo => self.maximize_info_truncate(messages, max_tokens, model).await,
            TruncationStrategyType::MinimizeTokens => self.minimize_tokens_truncate(messages, max_tokens, model).await,
            TruncationStrategyType::Balanced => self.balanced_truncate(messages, max_tokens, model).await,
        }
    }

    /// 智能截断：基于相关性排序
    async fn smart_truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        // 简化实现：优先保留最近和较长的消息
        let mut scored = Vec::new();

        for (idx, msg) in messages.iter().enumerate() {
            // 计算分数：最近性 + 长度
            let recency_score = idx as f32 / messages.len() as f32;
            let length_score = (msg.content.len() as f32 / 1000.0).min(1.0);
            let score = recency_score * 0.6 + length_score * 0.4;

            scored.push((score, msg.clone()));
        }

        // 按分数排序
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // 选择消息直到达到 max_tokens
        let mut result = Vec::new();
        let mut current_tokens = 0;

        for (_, msg) in scored {
            let tokens = self.token_counter.count_tokens(&msg.content, model)?;
            if current_tokens + tokens > max_tokens {
                break;
            }
            current_tokens += tokens;
            result.push(msg);
        }

        // 按原始顺序排序
        result.sort_by(|a, b| {
            messages
                .iter()
                .position(|x| x.content == a.content)
                .unwrap()
                .partial_cmp(&messages.iter().position(|x| x.content == b.content).unwrap())
                .unwrap()
        });

        Ok(result)
    }

    /// 时间衰减：优先近期信息
    async fn time_decay_truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        let mut result = Vec::new();
        let mut current_tokens = 0;

        // 从最新消息开始倒序添加
        for msg in messages.iter().rev() {
            let tokens = self.token_counter.count_tokens(&msg.content, model)?;
            if current_tokens + tokens > max_tokens {
                break;
            }
            current_tokens += tokens;
            result.insert(0, msg.clone());
        }

        Ok(result)
    }

    /// 语义聚类：去重相似信息（简化实现）
    async fn semantic_cluster_truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        // 简化实现：去除内容相似的消息
        let mut result = Vec::new();
        let mut seen = HashMap::new();

        for msg in messages {
            // 使用内容的前 50 个字符作为键
            let key = msg.content.chars().take(50).collect::<String>();

            if !seen.contains_key(&key) {
                seen.insert(key.clone(), true);
                result.push(msg.clone());
            }
        }

        // 如果还是超过，保留最近的
        let current_tokens = self.count_total_tokens(&result, model)?;
        if current_tokens > max_tokens {
            return self.time_decay_truncate(&result, max_tokens, model).await;
        }

        Ok(result)
    }

    /// 最大化信息：保留最多信息
    async fn maximize_info_truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        // 优先保留长消息（包含更多信息）
        let mut sorted = messages.clone();
        sorted.sort_by_key(|msg| msg.content.len());

        let mut result = Vec::new();
        let mut current_tokens = 0;

        for msg in sorted.iter().rev() {
            let tokens = self.token_counter.count_tokens(&msg.content, model)?;
            if current_tokens + tokens > max_tokens {
                break;
            }
            current_tokens += tokens;
            result.push(msg.clone());
        }

        Ok(result)
    }

    /// 最小化 Token：最小化成本
    async fn minimize_tokens_truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        // 优先保留短消息
        let mut sorted = messages.clone();
        sorted.sort_by_key(|msg| msg.content.len());

        let mut result = Vec::new();
        let mut current_tokens = 0;

        for msg in &sorted {
            let tokens = self.token_counter.count_tokens(&msg.content, model)?;
            if current_tokens + tokens > max_tokens {
                break;
            }
            current_tokens += tokens;
            result.push(msg.clone());
        }

        Ok(result)
    }

    /// 平衡模式：平衡信息和成本
    async fn balanced_truncate(
        &mut self,
        messages: &Vec<Message>,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        // 混合策略：60% 时间衰减 + 40% 信息量
        let mut scored = Vec::new();

        for (idx, msg) in messages.iter().enumerate() {
            let recency_score = idx as f32 / messages.len() as f32;
            let length_score = (msg.content.len() as f32 / 500.0).min(1.0);
            let score = recency_score * 0.6 + length_score * 0.4;

            scored.push((score, msg.clone()));
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let mut result = Vec::new();
        let mut current_tokens = 0;

        for (_, msg) in scored {
            let tokens = self.token_counter.count_tokens(&msg.content, model)?;
            if current_tokens + tokens > max_tokens {
                break;
            }
            current_tokens += tokens;
            result.push(msg);
        }

        // 按原始顺序排序
        result.sort_by(|a, b| {
            messages
                .iter()
                .position(|x| x.content == a.content)
                .unwrap()
                .partial_cmp(&messages.iter().position(|x| x.content == b.content).unwrap())
                .unwrap()
        });

        Ok(result)
    }

    /// 计算消息列表的总 token 数量
    fn count_total_tokens(&mut self, messages: &Vec<Message>, model: &str) -> anyhow::Result<usize> {
        let mut total = 0;
        for msg in messages {
            total += self.token_counter.count_tokens(&msg.content, model)?;
        }
        Ok(total)
    }
}

impl Default for TruncationStrategy {
    fn default() -> Self {
        Self::new(TruncationConfig::default()).expect("Failed to create TruncationStrategy")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;

    #[tokio::test]
    async fn test_truncation_strategy() {
        let config = TruncationConfig::default();
        let mut strategy = TruncationStrategy::new(config).unwrap();

        let messages = vec![
            Message {
                role: MessageRole::User,
                content: "First message".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: MessageRole::Assistant,
                content: "Response to first".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: MessageRole::User,
                content: "Second message".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let result = strategy.truncate(&messages, 50, "gpt-4").await.unwrap();
        assert!(!result.is_empty());
        assert!(result.len() <= messages.len());
    }
}
