// Context Compression - v0.5.2
//
// 智能摘要和信息密度优化

use crate::llm::{Message, MessageRole};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;
use async_trait::async_trait;

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// 目标压缩率 (0.0 - 1.0)
    pub target_ratio: f32,
    /// 最大 Token 数
    pub max_tokens: usize,
    /// 保留系统消息
    pub keep_system_messages: bool,
    /// 保留最近 N 条消息
    pub keep_recent_count: usize,
    /// 是否启用重要性评分
    pub enable_importance_scoring: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            target_ratio: 0.5,
            max_tokens: 4000,
            keep_system_messages: true,
            keep_recent_count: 3,
            enable_importance_scoring: true,
        }
    }
}

/// 压缩结果
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// 压缩后的消息
    pub messages: Vec<Message>,
    /// 原始 Token 数
    pub original_tokens: usize,
    /// 压缩后 Token 数
    pub compressed_tokens: usize,
    /// 压缩率
    pub ratio: f32,
    /// 摘要文本
    pub summary: Option<String>,
}

/// 摘要器 Trait
#[async_trait]
pub trait Summarizer: Send + Sync {
    /// 生成摘要
    async fn summarize(&self, text: &str, max_length: usize) -> Result<String>;
    
    /// 摘要多轮对话
    async fn summarize_messages(&self, messages: &[Message]) -> Result<String>;
}

/// 基础摘要器（使用简单截断）
pub struct BasicSummarizer;

impl BasicSummarizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BasicSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Summarizer for BasicSummarizer {
    async fn summarize(&self, text: &str, max_length: usize) -> Result<String> {
        if text.len() <= max_length {
            return Ok(text.to_string());
        }
        
        // 简单截断，保留开头和结尾
        let keep_start = (max_length / 2).min(text.len());
        let keep_end = max_length.saturating_sub(keep_start).saturating_sub(20);
        
        let start = &text[..keep_start];
        let end_start = text.len().saturating_sub(keep_end);
        let end = if end_start > keep_start {
            &text[end_start..]
        } else {
            ""
        };
        
        Ok(format!("{}...[truncated]...{}", start, end))
    }
    
    async fn summarize_messages(&self, messages: &[Message]) -> Result<String> {
        let mut summary = String::new();
        
        for msg in messages {
            let role = match msg.role {
                MessageRole::System => "System",
                MessageRole::User => "User",
                MessageRole::Assistant => "Assistant",
                MessageRole::Tool => "Tool",
            };
            summary.push_str(&format!("{}: {}\n", role, msg.content));
        }
        
        Ok(summary)
    }
}

/// 上下文压缩器
pub struct ContextCompressor {
    config: CompressionConfig,
    summarizer: Option<Box<dyn Summarizer>>,
}

impl ContextCompressor {
    /// 创建新的压缩器
    pub fn new(config: CompressionConfig) -> Self {
        Self {
            config,
            summarizer: None,
        }
    }
    
    /// 设置摘要器
    pub fn with_summarizer(mut self, summarizer: Box<dyn Summarizer>) -> Self {
        self.summarizer = Some(summarizer);
        self
    }
    
    /// 压缩消息
    pub async fn compress(&self, messages: Vec<Message>) -> Result<CompressionResult> {
        let original_tokens = self.estimate_tokens(&messages);
        
        // 如果已经足够小，直接返回
        if original_tokens <= self.config.max_tokens {
            return Ok(CompressionResult {
                messages,
                original_tokens,
                compressed_tokens: original_tokens,
                ratio: 1.0,
                summary: None,
            });
        }
        
        let mut compressed = Vec::new();
        let mut middle_messages = Vec::new();
        
        // 分离消息
        for (i, msg) in messages.into_iter().enumerate() {
            if matches!(msg.role, MessageRole::System) && self.config.keep_system_messages {
                compressed.push(msg);
            } else {
                middle_messages.push((i, msg));
            }
        }
        
        // 保留最近 N 条
        let recent_count = self.config.keep_recent_count.min(middle_messages.len());
        let recent_start = middle_messages.len().saturating_sub(recent_count);
        
        let to_compress: Vec<Message> = middle_messages[..recent_start]
            .iter()
            .map(|(_, msg)| msg.clone())
            .collect();
        
        let recent: Vec<Message> = middle_messages[recent_start..]
            .iter()
            .map(|(_, msg)| msg.clone())
            .collect();
        
        // 生成摘要
        let summary = if !to_compress.is_empty() {
            if let Some(ref summarizer) = self.summarizer {
                Some(summarizer.summarize_messages(&to_compress).await?)
            } else {
                // 基础摘要
                let summarizer = BasicSummarizer::new();
                Some(summarizer.summarize_messages(&to_compress).await?)
            }
        } else {
            None
        };
        
        // 添加摘要作为系统消息
        if let Some(ref summary_text) = summary {
            compressed.push(Message {
                role: MessageRole::System,
                content: format!("[Previous conversation summary]\n{}", summary_text),
                tool_calls: None,
                tool_call_id: None,
            });
        }
        
        // 添加最近消息
        compressed.extend(recent);
        
        let compressed_tokens = self.estimate_tokens(&compressed);
        let ratio = compressed_tokens as f32 / original_tokens as f32;
        
        Ok(CompressionResult {
            messages: compressed,
            original_tokens,
            compressed_tokens,
            ratio,
            summary,
        })
    }
    
    /// 估算 Token 数
    fn estimate_tokens(&self, messages: &[Message]) -> usize {
        let mut total = 0;
        for msg in messages {
            total += self.count_tokens(&msg.content);
        }
        total
    }
    
    /// 简单 Token 计数
    fn count_tokens(&self, text: &str) -> usize {
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        
        let chinese_chars = text.chars().filter(|c| *c as u32 > 255).count();
        let english_words = words.saturating_sub(chinese_chars / 2);
        
        (chinese_chars as f64 * 1.5) as usize + (english_words as f64 * 0.75) as usize
    }
    
    /// 计算消息重要性分数
    pub fn calculate_importance(&self, msg: &Message, position: usize, total: usize) -> f32 {
        let mut score = 0.5;
        
        // 系统消息更重要
        if matches!(msg.role, MessageRole::System) {
            score += 0.3;
        }
        
        // 最近的消息更重要
        let recency_weight = position as f32 / total as f32;
        score += recency_weight * 0.2;
        
        // 长度因素（包含更多信息）
        let length_weight = (msg.content.len() as f32 / 1000.0).min(0.2);
        score += length_weight;
        
        score.min(1.0)
    }
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self::new(CompressionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        assert!((config.target_ratio - 0.5).abs() < 0.001);
        assert!(config.keep_system_messages);
    }

    #[tokio::test]
    async fn test_basic_summarizer() {
        let summarizer = BasicSummarizer::new();
        let text = "This is a long text that needs to be summarized.";
        let summary = summarizer.summarize(text, 10).await.unwrap();
        assert!(summary.len() <= 35); // 10 + ...[truncated]...
    }

    #[tokio::test]
    async fn test_context_compressor_no_compression_needed() {
        let compressor = ContextCompressor::default();
        let messages = vec![
            Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }
        ];
        
        let result = compressor.compress(messages).await.unwrap();
        assert!((result.ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_calculate_importance_system() {
        let compressor = ContextCompressor::default();
        let msg = Message {
            role: MessageRole::System,
            content: "System message".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        
        let score = compressor.calculate_importance(&msg, 0, 10);
        assert!(score > 0.5);
    }

    #[test]
    fn test_calculate_importance_recent() {
        let compressor = ContextCompressor::default();
        let msg = Message {
            role: MessageRole::User,
            content: "Test".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        
        let score_recent = compressor.calculate_importance(&msg, 9, 10);
        let score_old = compressor.calculate_importance(&msg, 0, 10);
        
        assert!(score_recent > score_old);
    }
}
