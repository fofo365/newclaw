// Embedding Client - v0.5.0
//
// 嵌入客户端实现：
// - OpenAI 嵌入 API 客户端
// - 异步批量嵌入
// - 错误重试机制
// - 速率限制处理

use super::{EmbeddingError, EmbeddingResult, BatchEmbeddingResult, EmbeddingModel, EmbeddingOptions};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// OpenAI 嵌入 API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// API Key
    pub api_key: String,
    /// API 基础 URL
    pub base_url: String,
    /// 嵌入模型
    pub model: EmbeddingModel,
    /// 请求选项
    pub options: EmbeddingOptions,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: EmbeddingModel::default(),
            options: EmbeddingOptions::default(),
        }
    }
}

/// 嵌入客户端 Trait
#[async_trait]
pub trait EmbeddingClient: Send + Sync {
    /// 嵌入单个文本
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError>;

    /// 批量嵌入
    async fn embed_batch(&self, texts: Vec<String>) -> Result<BatchEmbeddingResult, EmbeddingError>;
}

/// OpenAI 嵌入客户端
pub struct OpenAIEmbeddingClient {
    config: Arc<EmbeddingConfig>,
    client: Client,
    semaphore: Arc<Semaphore>,
}

impl OpenAIEmbeddingClient {
    /// 创建新的客户端
    pub fn new(config: EmbeddingConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.options.batch_size));
        Self {
            config: Arc::new(config),
            client: Client::new(),
            semaphore,
        }
    }

    /// 从环境变量创建客户端
    pub fn from_env() -> Result<Self, EmbeddingError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| EmbeddingError::InvalidInput("OPENAI_API_KEY not set".to_string()))?;

        let config = EmbeddingConfig {
            api_key,
            ..Default::default()
        };

        Ok(Self::new(config))
    }

    /// 发送 API 请求
    async fn send_request(&self, texts: Vec<String>) -> Result<OpenAIEmbeddingResponse, EmbeddingError> {
        let url = format!("{}/embeddings", self.config.base_url);

        let request = OpenAIEmbeddingRequest {
            model: self.config.model.as_str().to_string(),
            input: texts,
            encoding_format: Some("float".to_string()),
            dimensions: Some(self.config.model.dimension() as u32),
        };

        let mut retries = 0;
        let max_retries = self.config.options.max_retries;

        loop {
            let permit = self.semaphore.acquire().await
                .map_err(|e| EmbeddingError::Unknown(e.to_string()))?;

            let response = self.client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .timeout(self.config.options.timeout)
                .send()
                .await;

            drop(permit);

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    if retries < max_retries {
                        retries += 1;
                        tokio::time::sleep(Duration::from_millis(1000 * retries as u64)).await;
                        continue;
                    }
                    return Err(EmbeddingError::NetworkError(e.to_string()));
                }
            };

            let status = response.status();

            if status.is_success() {
                let text = response.text().await
                    .map_err(|e| EmbeddingError::NetworkError(e.to_string()))?;
                return serde_json::from_str(&text)
                    .map_err(|e| EmbeddingError::ApiError(e.to_string()));
            } else {
                let error_text = response.text().await.unwrap_or_default();
                let error = if status.as_u16() == 429 {
                    EmbeddingError::RateLimit
                } else {
                    EmbeddingError::ApiError(format!("{}: {}", status, error_text))
                };

                if retries < max_retries && status.as_u16() >= 500 {
                    retries += 1;
                    tokio::time::sleep(Duration::from_millis(1000 * retries as u64)).await;
                    continue;
                }

                return Err(error);
            }
        }
    }
}

#[async_trait]
impl EmbeddingClient for OpenAIEmbeddingClient {
    /// 嵌入单个文本
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, EmbeddingError> {
        let start = Instant::now();
        let response = self.send_request(vec![text.to_string()]).await?;
        let duration = start.elapsed();

        if response.data.is_empty() {
            return Err(EmbeddingError::ApiError("No embedding returned".to_string()));
        }

        let result = response.data.into_iter().next().unwrap();
        let tokens = response.usage.total_tokens;

        Ok(EmbeddingResult {
            embedding: result.embedding,
            model: response.model,
            tokens,
            duration,
        })
    }

    /// 批量嵌入
    async fn embed_batch(&self, texts: Vec<String>) -> Result<BatchEmbeddingResult, EmbeddingError> {
        let start = Instant::now();
        let response = self.send_request(texts).await?;
        let total_duration = start.elapsed();

        let embeddings: Vec<Vec<f32>> = response.data
            .into_iter()
            .map(|d| d.embedding)
            .collect();

        Ok(BatchEmbeddingResult {
            embeddings,
            total_tokens: response.usage.total_tokens,
            total_duration,
        })
    }
}

/// OpenAI 嵌入请求
#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<u32>,
}

/// OpenAI 嵌入响应
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    object: String,
    data: Vec<OpenAIEmbeddingData>,
    model: String,
    usage: OpenAIUsage,
}

/// OpenAI 嵌入数据
#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: usize,
}

/// OpenAI 使用情况
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.model, EmbeddingModel::OpenAI3Small);
    }

    #[tokio::test]
    #[ignore] // 需要 API Key
    async fn test_openai_client_embed() {
        let client = OpenAIEmbeddingClient::from_env().unwrap();
        let result = client.embed("Hello, world!").await.unwrap();
        assert_eq!(result.embedding.len(), 1536);
        assert!(result.tokens > 0);
    }
}
