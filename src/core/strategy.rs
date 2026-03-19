// Strategy Engine - Context selection strategies

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::core::context::ContextChunk;

pub struct StrategyEngine {
    pub active_strategy: Option<Box<dyn Strategy>>,
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StrategyEngine {
    pub fn new() -> Self {
        Self {
            active_strategy: Some(Box::new(SmartTruncationStrategy::default())),
        }
    }

    pub async fn select_context(
        &self,
        context: &[ContextChunk],
        max_tokens: usize,
    ) -> Result<Vec<ContextChunk>> {
        if let Some(strategy) = &self.active_strategy {
            strategy.select_context(context, max_tokens).await
        } else {
            Ok(context.to_vec())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    SmartTruncation,
    TimeDecay,
    SemanticCluster,
    MaximalInformation,
    MinimalTokens,
    Balanced,
}

#[async_trait]
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    
    async fn select_context(
        &self,
        context: &[ContextChunk],
        max_tokens: usize,
    ) -> Result<Vec<ContextChunk>>;
}

#[derive(Debug, Clone)]
pub struct SmartTruncationStrategy {
    pub importance_threshold: f32,
}

impl Default for SmartTruncationStrategy {
    fn default() -> Self {
        Self {
            importance_threshold: 0.7,
        }
    }
}

#[async_trait]
impl Strategy for SmartTruncationStrategy {
    fn name(&self) -> &str {
        "smart_truncation"
    }
    
    async fn select_context(
        &self,
        context: &[ContextChunk],
        max_tokens: usize,
    ) -> Result<Vec<ContextChunk>> {
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        for chunk in context {
            let importance = self.estimate_importance(chunk)?;
            
            if importance < self.importance_threshold {
                continue;
            }
            
            if current_tokens + chunk.tokens > max_tokens {
                break;
            }
            
            selected.push(chunk.clone());
            current_tokens += chunk.tokens;
        }
        
        Ok(selected)
    }
}

impl SmartTruncationStrategy {
    fn estimate_importance(&self, chunk: &ContextChunk) -> Result<f32> {
        let mut importance: f32 = 0.5;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        let age_seconds = now - chunk.created_at;
        let recency_factor = if age_seconds < 7200 {
            1.5
        } else {
            0.5
        };
        importance *= recency_factor;
        
        let content_lower = chunk.text.to_lowercase();
        for keyword in &["问题", "错误", "bug", "优化", "重要", "紧急", "error", "issue"] {
            if content_lower.contains(keyword) {
                importance += 0.3;
            }
        }
        
        Ok(importance.min(1.0))
    }
}
