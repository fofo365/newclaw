// Vector Embedding Module - v0.5.0
//
// 向量嵌入模块：
// - 嵌入客户端抽象
// - OpenAI 嵌入实现
// - 本地模型支持 (可选)
// - 嵌入缓存机制

pub mod client;
pub mod pipeline;
pub mod cache;

pub use client::{EmbeddingClient, OpenAIEmbeddingClient, EmbeddingConfig};
pub use pipeline::{EmbeddingPipeline, TextChunker};
pub use cache::{EmbeddingCache, CacheConfig};

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 嵌入向量维度
pub const EMBEDDING_DIM: usize = 1536; // OpenAI text-embedding-3-small

/// 嵌入错误类型
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// 嵌入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// 嵌入向量
    pub embedding: Vec<f32>,
    /// 使用的模型
    pub model: String,
    /// Token 使用量
    pub tokens: usize,
    /// 延迟时间
    pub duration: Duration,
}

/// 批量嵌入结果
#[derive(Debug, Clone)]
pub struct BatchEmbeddingResult {
    /// 嵌入向量列表
    pub embeddings: Vec<Vec<f32>>,
    /// 总 Token 使用量
    pub total_tokens: usize,
    /// 总延迟时间
    pub total_duration: Duration,
}

/// 嵌入模型类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmbeddingModel {
    /// OpenAI text-embedding-3-small (1536 维, 性价比高)
    OpenAI3Small,
    /// OpenAI text-embedding-3-large (3072 维, 质量高)
    OpenAI3Large,
    /// 本地模型 (BGE, MTEB 等)
    Local(String),
}

impl EmbeddingModel {
    /// 获取模型维度
    pub fn dimension(&self) -> usize {
        match self {
            EmbeddingModel::OpenAI3Small => 1536,
            EmbeddingModel::OpenAI3Large => 3072,
            EmbeddingModel::Local(_) => 768, // 默认本地模型维度
        }
    }

    /// 获取模型名称
    pub fn as_str(&self) -> &str {
        match self {
            EmbeddingModel::OpenAI3Small => "text-embedding-3-small",
            EmbeddingModel::OpenAI3Large => "text-embedding-3-large",
            EmbeddingModel::Local(name) => name,
        }
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text-embedding-3-small" => Some(EmbeddingModel::OpenAI3Small),
            "text-embedding-3-large" => Some(EmbeddingModel::OpenAI3Large),
            _ => Some(EmbeddingModel::Local(s.to_string())),
        }
    }

    /// 获取最大 Token 数
    pub fn max_tokens(&self) -> usize {
        match self {
            EmbeddingModel::OpenAI3Small => 8191,
            EmbeddingModel::OpenAI3Large => 8191,
            EmbeddingModel::Local(_) => 512,
        }
    }
}

impl Default for EmbeddingModel {
    fn default() -> Self {
        EmbeddingModel::OpenAI3Small
    }
}

/// 嵌入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingOptions {
    /// 批量大小
    pub batch_size: usize,
    /// 超时时间
    pub timeout: Duration,
    /// 最大重试次数
    pub max_retries: usize,
    /// 是否启用缓存
    pub enable_cache: bool,
    /// 缓存 TTL
    pub cache_ttl: Duration,
}

impl Default for EmbeddingOptions {
    fn default() -> Self {
        Self {
            batch_size: 10,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_cache: true,
            cache_ttl: Duration::from_secs(3600 * 24), // 24 小时
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_model() {
        let model = EmbeddingModel::OpenAI3Small;
        assert_eq!(model.dimension(), 1536);
        assert_eq!(model.as_str(), "text-embedding-3-small");
        assert_eq!(model.max_tokens(), 8191);
    }

    #[test]
    fn test_embedding_model_from_str() {
        assert_eq!(
            EmbeddingModel::from_str("text-embedding-3-small"),
            Some(EmbeddingModel::OpenAI3Small)
        );
        assert_eq!(
            EmbeddingModel::from_str("text-embedding-3-large"),
            Some(EmbeddingModel::OpenAI3Large)
        );
        assert!(EmbeddingModel::from_str("unknown").is_some());
    }

    #[test]
    fn test_embedding_options_default() {
        let options = EmbeddingOptions::default();
        assert_eq!(options.batch_size, 10);
        assert_eq!(options.timeout, Duration::from_secs(30));
        assert_eq!(options.max_retries, 3);
        assert!(options.enable_cache);
    }
}
