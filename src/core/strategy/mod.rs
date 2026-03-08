// Strategy Engine

use crate::core::context::ContextManager;
use serde::{Deserialize, Serialize};

pub struct StrategyEngine {
    strategies: std::collections::HashMap<String, Box<dyn Strategy>>,
    active_strategy: Option<String>,
    config: StrategyConfig,
}

pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    
    async fn select_context(
        &self,
        context: &[crate::core::context::ContextMessage],
        max_tokens: usize,
    ) -> Result<Vec<crate::core::context::ContextMessage>>;
}

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    SmartTruncation,
    TimeDecay,
    SemanticCluster,
    MaximalInformation,
    MinimalTokens,
    Balanced,
}

/// 策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub max_tokens: Option<usize>,
    pub importance_threshold: Option<f32>,
    pub decay_rate: Option<f64>,
    pub similarity_threshold: Option<f32>,
    pub max_per_cluster: Option<usize>,
    pub prioritize_recent: Option<bool>,
    pub recent_weight: Option<f64>,
}

/// 策略元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
}

/// 智能截断策略
pub struct SmartTruncationStrategy {
    config: StrategyConfig,
}

#[async_trait::async_trait]
impl Strategy for SmartTruncation {
    fn name(&self) -> &str {
        "smart_truncation"
    }
    
    async fn select_context(
        &self,
        context: &[crate::core::context::ContextMessage],
        max_tokens: usize,
    ) -> Result<Vec<crate::core::context::ContextMessage>> {
        let importance_threshold = self.config
            .importance_threshold
            .unwrap_or(0.7);
        
        // 根据重要性选择
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        for msg in context {
            let importance = self.estimate_importance(msg)?;
            
            if importance < importance_threshold {
                continue;  // 跳过低重要性的消息
            }
            
            if current_tokens + msg.tokens > max_tokens {
                break;  // 达到 token 限制
            }
            
            selected.push(msg.clone());
            current_tokens += msg.tokens;
        }
        
        Ok(selected)
    }
    
    fn estimate_importance(&self, msg: &crate::core::context::ContextMessage) -> Result<f32> {
        // 基于多种因素估算重要性
        let mut importance = 0.5;
        
        // 新鲜度权重
        let age_s = 60 * 60; // 1 小时 = 3600 秒
        let age = msg.timestamp - age_s;
        let recency_factor = if age < 7200 {
            1.5  // 2 小时内权重 1.5 倍
        } else {
            0.5  // 更老的权重降低
        };
        importance *= recency;
        
        // 关键词检测
        let content_lower = msg.content.to_lowercase();
        for keyword in &["问题", "错误", "bug", "优化", "重要", "紧急"] {
            if content_lower.contains(keyword) {
                importance += 0.3;
            }
        }
        
        Ok(importance.min(1.0))
    }
}

/// 时间衰减策略
pub struct TimeDecayStrategy {
    decay_rate: f64,
    recent_minutes: i32,
}

#[async_trait::async_trait]
impl Strategy for TimeDecayStrategy {
    fn name(&str) -> &str {
        "time_decay"
    }
    
    async fn select_context(
        &self,
        context: &[crate::core::context::ContextMessage],
        max_tokens: usize,
    ) -> Result<Vec<crate::core::context::ContextMessage>> {
        let mut weighted: Vec<(ContextMessage, f64)> = context
            .iter()
            .map(|msg| {
                let age_s = 3600 - (msg.timestamp / 1000);  // 转换为秒
                let weight = (self.recent_minutes * 60) as f64;
                
                let decay = f64::exp(-age_s as f64 / self.decay_rate);
                
                (msg.clone(), decay)
            })
            .collect();
        
        // 按权重排序
        weighted.sort_by(|a, b| a.1.partial_cmp(&b.1));
        
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        for (msg, weight) in weighted {
            if selected.len() >= 10 {
                break;
            }
            
            if current_tokens + msg.tokens > max_tokens {
                break;
            }
            
            selected.push(msg.clone());
            current_tokens += msg.tokens;
        }
        
        Ok(selected)
    }
}

/// 语义聚类策略
pub struct SemanticClusterStrategy {
    similarity_threshold: f64,
    max_cluster_size: usize,
    cluster_by: String,  // 可选字段用于指定聚类依据（如：author, topic）
}

#[async_trait::async_trait]
impl Strategy for SemanticClusterStrategy {
    fn name(&str) -> &str {
        "semantic_cluster"
    }
    
    async fn select_context(
        &self,
        context: &[crate::core::context::ContextMessage],
        max_tokens: usize,
    ) -> Result<Vec<crate::core::context::ContextMessage>> {
        // 按相似度聚类
        let clustered = self.cluster_by_similarity(context, self.similarity_threshold)?;
        
        // 从每个聚类中选择代表
        let selected = self.select_from_clusters(&clustered, max_tokens)?;
        
        Ok(selected)
    }
}
