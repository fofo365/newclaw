// Embedding Pipeline - v0.5.0
//
// 向量化流水线：
// - 智能文本分块
// - 批量嵌入处理
// - 进度追踪
// - 错误处理和重试

use super::{EmbeddingClient, EmbeddingError, EmbeddingResult, BatchEmbeddingResult, EmbeddingOptions};
use std::time::Instant;
use tokio::sync::mpsc;

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
}

impl EmbeddingPipeline {
    /// 创建新的流水线
    pub fn new(client: Box<dyn EmbeddingClient>, chunker: TextChunker, options: EmbeddingOptions) -> Self {
        Self {
            client,
            chunker,
            options,
        }
    }

    /// 处理单个文档（自动分块）
    pub async fn process_document(&self, text: &str) -> Result<PipelineResult, EmbeddingError> {
        let start = Instant::now();

        // 分块
        let chunks = self.chunker.chunk(text)?;
        let chunk_count = chunks.len();

        // 批量嵌入
        let batch_result = self.client.embed_batch(chunks.clone()).await?;

        // 组装结果
        let embeddings: Vec<EmbeddingResult> = chunks
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

        Ok(PipelineResult {
            embeddings,
            total_tokens: batch_result.total_tokens,
            total_duration: start.elapsed(),
            chunk_count,
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
        texts: Vec<String>,
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
