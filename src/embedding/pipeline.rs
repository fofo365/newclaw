// Embedding Pipeline - v0.5.0
//
// 向量化流水线：
// - 智能文本分块
// - 批量嵌入处理
// - 进度追踪
// - 错误处理和重试
// - 缓存集成

use super::{EmbeddingClient, EmbeddingError, EmbeddingResult, BatchEmbeddingResult, EmbeddingOptions};
use super::cache::EmbeddingCache;
use std::time::Instant;
use tokio::sync::mpsc;
use std::sync::Arc;

/// 文本分块器
pub struct TextChunker {
    /// 最大 Token 数
    max_tokens: usize,
    /// 重叠 Token 数
    overlap_tokens: usize,
    /// 句子边界检测
    sentence_boundary: bool,
}

impl TextChunker {
    /// 创建新的分块器
    pub fn new(max_tokens: usize, overlap_tokens: usize) -> Self {
        Self {
            max_tokens,
            overlap_tokens,
            sentence_boundary: true,
        }
    }

    /// 分割文本为块
    pub fn chunk(&self, text: &str) -> Result<Vec<String>, EmbeddingError> {
        if text.is_empty() {
            return Ok(vec![]);
        }

        // 简单实现：按字符分割（TODO: 改进为按句子分割）
        let chars: Vec<char> = text.chars().collect();
        let chunk_size = self.max_tokens * 4; // 粗略估计 1 token ≈ 4 chars
        let overlap_size = self.overlap_tokens * 4;

        // 防止死循环：确保 overlap_size < chunk_size
        if overlap_size >= chunk_size {
            return Err(EmbeddingError::InvalidInput(
                "overlap_tokens must be less than max_tokens".to_string()
            ));
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < chars.len() {
            let end = std::cmp::min(start + chunk_size, chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            // 确保至少前进 1 个字符
            start = std::cmp::max(start + 1, end - overlap_size);
        }

        Ok(chunks)
    }

    /// 估算 Token 数量
    pub fn estimate_tokens(&self, text: &str) -> usize {
        // 粗略估算：1 token ≈ 4 chars (英文) 或 2 chars (中文)
        let char_count = text.chars().count();
        let chinese_count = text.chars().filter(|c| {
            let code = *c as u32;
            code > 0x4E00 && code < 0x9FFF
        }).count();
        let english_count = char_count - chinese_count;

        (chinese_count / 2) + (english_count / 4)
    }
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new(8000, 200)
    }
}

/// 嵌入流水线
pub struct EmbeddingPipeline {
    /// 嵌入客户端
    client: Box<dyn EmbeddingClient>,
    /// 文本分块器
    chunker: TextChunker,
    /// 配置选项
    options: EmbeddingOptions,
    /// 嵌入缓存
    cache: Option<Arc<EmbeddingCache>>,
}

impl EmbeddingPipeline {
    /// 创建新的流水线
    pub fn new(client: Box<dyn EmbeddingClient>, chunker: TextChunker, options: EmbeddingOptions) -> Self {
        Self {
            client,
            chunker,
            options,
            cache: None,
        }
    }

    /// 启用缓存
    pub fn with_cache(mut self, cache: Arc<EmbeddingCache>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// 处理单个文档（自动分块 + 缓存）
    pub async fn process_document(&self, text: &str) -> Result<PipelineResult, EmbeddingError> {
        let start = Instant::now();

        // 分块
        let chunks = self.chunker.chunk(text)?;
        let chunk_count = chunks.len();

        // 检查缓存（如果启用）
        let mut uncached_chunks = Vec::new();
        let mut cached_results = Vec::new();
        let mut cache_hits = 0;
        let mut chunk_to_cache_idx: Vec<Option<usize>> = Vec::new(); // 映射原始块索引到缓存结果索引

        if let Some(cache) = &self.cache {
            for (idx, chunk) in chunks.iter().enumerate() {
                let cache_key = self.compute_cache_key(chunk);
                if let Some(result) = cache.get(&cache_key).await {
                    cached_results.push((idx, result.clone()));
                    cache_hits += 1;
                    chunk_to_cache_idx.push(Some(cached_results.len() - 1));
                } else {
                    uncached_chunks.push(chunk.clone());
                    chunk_to_cache_idx.push(None);
                }
            }
        } else {
            uncached_chunks = chunks.clone();
            for _ in 0..chunks.len() {
                chunk_to_cache_idx.push(None);
            }
        }

        // 批量嵌入未缓存的块
        let mut new_embeddings = Vec::new();
        if !uncached_chunks.is_empty() {
            let batch_result = self.client.embed_batch(uncached_chunks.clone()).await?;

            // 将新结果存入缓存
            if let Some(cache) = &self.cache {
                for (chunk, embedding) in uncached_chunks.iter().zip(batch_result.embeddings.iter()) {
                    let result = EmbeddingResult {
                        embedding: embedding.clone(),
                        model: "pipeline".to_string(),
                        tokens: self.chunker.estimate_tokens(chunk),
                        duration: if chunk_count > 0 {
                            batch_result.total_duration / chunk_count as u32
                        } else {
                            batch_result.total_duration
                        },
                    };

                    let cache_key = self.compute_cache_key(chunk);
                    cache.put(cache_key, result.clone()).await;
                    new_embeddings.push(result);
                }
            } else {
                new_embeddings = uncached_chunks
                    .into_iter()
                    .zip(batch_result.embeddings.into_iter())
                    .map(|(text, embedding)| EmbeddingResult {
                        embedding,
                        model: "pipeline".to_string(),
                        tokens: self.chunker.estimate_tokens(&text),
                        duration: if chunk_count > 0 {
                            batch_result.total_duration / chunk_count as u32
                        } else {
                            batch_result.total_duration
                        },
                    })
                    .collect();
            }
        }

        // 合并缓存和新结果（保持原始顺序）
        let mut all_embeddings: Vec<Option<EmbeddingResult>> = vec![None; chunks.len()];

        // 填充缓存结果
        for (idx, result) in cached_results {
            all_embeddings[idx] = Some(result);
        }

        // 填充新结果
        let mut new_idx = 0;
        for (idx, cache_idx) in chunk_to_cache_idx.iter().enumerate() {
            if cache_idx.is_none() && new_idx < new_embeddings.len() {
                all_embeddings[idx] = Some(new_embeddings[new_idx].clone());
                new_idx += 1;
            }
        }

        // 转换为 Vec，移除 None
        let embeddings: Vec<EmbeddingResult> = all_embeddings.into_iter().filter_map(|x| x).collect();

        // 计算统计信息
        let total_tokens: usize = embeddings.iter().map(|e| e.tokens).sum();
        let cache_hit_rate = if chunk_count > 0 {
            cache_hits as f64 / chunk_count as f64
        } else {
            0.0
        };

        Ok(PipelineResult {
            embeddings,
            total_tokens,
            total_duration: start.elapsed(),
            chunk_count,
            cache_hits,
            cache_hit_rate,
        })
    }

    /// 处理多个文档
    pub async fn process_documents(&self, texts: Vec<String>) -> Result<Vec<PipelineResult>, EmbeddingError> {
        let mut results = Vec::new();

        for text in texts {
            let result = self.process_document(&text).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 流式处理（发送进度更新）
    pub async fn process_stream(
        &self,
        _texts: Vec<String>,
    ) -> mpsc::UnboundedReceiver<PipelineProgress> {
        let (tx, rx) = mpsc::unbounded_channel();

        // TODO: 实现流式处理
        let _ = tx.send(PipelineProgress {
            current: 0,
            total: 0,
            status: "Not implemented".to_string(),
        });

        rx
    }

    /// 计算缓存键（简单哈希）
    fn compute_cache_key(&self, text: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// 流水线结果
#[derive(Debug, Clone)]
pub struct PipelineResult {
    /// 嵌入向量列表
    pub embeddings: Vec<EmbeddingResult>,
    /// 总 Token 使用量
    pub total_tokens: usize,
    /// 总处理时间
    pub total_duration: std::time::Duration,
    /// 分块数量
    pub chunk_count: usize,
    /// 缓存命中次数
    pub cache_hits: usize,
    /// 缓存命中率
    pub cache_hit_rate: f64,
}

/// 流水线进度
#[derive(Debug, Clone)]
pub struct PipelineProgress {
    /// 当前处理数量
    pub current: usize,
    /// 总数量
    pub total: usize,
    /// 状态信息
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_chunker() {
        let chunker = TextChunker::new(100, 20);
        let text = "A".repeat(500);
        let chunks = chunker.chunk(&text).unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_estimate_tokens() {
        let chunker = TextChunker::default();
        let text = "Hello, world!";
        let tokens = chunker.estimate_tokens(text);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let chunker = TextChunker::default();
        let text = "你好，世界！";
        let tokens = chunker.estimate_tokens(text);
        assert!(tokens > 0);
    }
}
