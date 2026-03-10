// Embedding Integration Tests - v0.5.0
//
// 集成测试：
// - OpenAI 嵌入 API 测试
// - 批量嵌入测试
// - 缓存机制测试
// - 端到端流水线测试

use newclaw::embedding::{
    EmbeddingClient, OpenAIEmbeddingClient, EmbeddingConfig, EmbeddingCache,
    EmbeddingPipeline, TextChunker, EmbeddingModel, EmbeddingOptions,
};
use std::time::Duration;

#[tokio::test]
#[ignore] // 需要真实的 API Key
async fn test_openai_embedding_single() {
    // 跳过测试如果没有 API Key
    let api_key = std::env::var("OPENAI_API_KEY");
    if api_key.is_err() {
        return;
    }

    let config = EmbeddingConfig {
        api_key: api_key.unwrap(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);

    let result = client.embed("Hello, world!").await.unwrap();

    assert_eq!(result.embedding.len(), 1536);
    assert!(result.tokens > 0);
    assert!(result.duration.as_millis() > 0);
}

#[tokio::test]
#[ignore]
async fn test_openai_embedding_batch() {
    let api_key = std::env::var("OPENAI_API_KEY");
    if api_key.is_err() {
        return;
    }

    let config = EmbeddingConfig {
        api_key: api_key.unwrap(),
        base_url: "https://api.openai.com/v1".to_string(),
        model: EmbeddingModel::OpenAI3Small,
        options: EmbeddingOptions {
            batch_size: 5,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            enable_cache: false,
            cache_ttl: Duration::from_secs(3600),
        },
    };

    let client = OpenAIEmbeddingClient::new(config);

    let texts = vec![
        "The quick brown fox jumps over the lazy dog.".to_string(),
        "AI is transforming the world.".to_string(),
        "Rust is a systems programming language.".to_string(),
    ];

    let result = client.embed_batch(texts).await.unwrap();

    assert_eq!(result.embeddings.len(), 3);
    assert!(result.total_tokens > 0);
    assert!(result.total_duration.as_millis() > 0);
}

#[tokio::test]
#[ignore]
async fn test_embedding_cache() {
    let api_key = std::env::var("OPENAI_API_KEY");
    if api_key.is_err() {
        return;
    }

    let config = EmbeddingConfig {
        api_key: api_key.unwrap(),
        base_url: "https://api.openai.com/v1".to_string(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let cache = EmbeddingCache::new(Default::default());

    // 第一次请求（未缓存）
    let text1 = "Cache test text";
    let result1 = client.embed(text1).await.unwrap();

    // 缓存结果
    cache.put(text1.to_string(), result1.clone()).await;

    // 第二次请求（从缓存）
    let cached = cache.get(text1).await;
    assert!(cached.is_some());
    let cached = cached.unwrap();

    assert_eq!(cached.embedding, result1.embedding);
    assert_eq!(cached.model, result1.model);

    // 检查统计
    let stats = cache.stats().await;
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
}

#[tokio::test]
async fn test_text_chunker() {
    let chunker = TextChunker::new(100, 20);

    // 短文本（不分块）
    let short = "Short text";
    let chunks = chunker.chunk(short).unwrap();
    assert_eq!(chunks.len(), 1);

    // 长文本（分块）
    let long = "A".repeat(500);
    let chunks = chunker.chunk(&long).unwrap();
    assert!(chunks.len() > 1);
}

#[tokio::test]
async fn test_estimate_tokens() {
    let chunker = TextChunker::default();

    // 英文
    let english = "The quick brown fox jumps over the lazy dog.";
    let tokens_en = chunker.estimate_tokens(english);
    assert!(tokens_en > 0 && tokens_en < 20);

    // 中文
    let chinese = "快速的棕色狐狸跳过懒狗。";
    let tokens_zh = chunker.estimate_tokens(chinese);
    assert!(tokens_zh > 0 && tokens_zh < 20);

    // 混合
    let mixed = "Hello 你好 World 世界";
    let tokens_mixed = chunker.estimate_tokens(mixed);
    assert!(tokens_mixed > 0);
}

#[tokio::test]
#[ignore]
async fn test_embedding_pipeline() {
    let api_key = std::env::var("OPENAI_API_KEY");
    if api_key.is_err() {
        return;
    }

    let config = EmbeddingConfig {
        api_key: api_key.unwrap(),
        base_url: "https://api.openai.com/v1".to_string(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let chunker = TextChunker::new(500, 100);
    let options = EmbeddingOptions::default();

    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        chunker,
        options,
    );

    // 测试短文档
    let short_text = "This is a short document for testing.";
    let result = pipeline.process_document(short_text).await.unwrap();

    assert!(!result.embeddings.is_empty());
    assert!(result.total_tokens > 0);
    assert!(result.total_duration.as_millis() > 0);

    // 测试长文档
    let long_text = "A".repeat(2000);
    let result = pipeline.process_document(&long_text).await.unwrap();

    assert!(result.chunk_count > 1);
    assert!(!result.embeddings.is_empty());
}

#[tokio::test]
async fn test_cache_ttl() {
    let cache = EmbeddingCache::new(newclaw::embedding::CacheConfig {
        max_entries: 100,
        ttl: Duration::from_millis(100), // 短 TTL
        enable_stats: true,
    });

    let result = newclaw::embedding::EmbeddingResult {
        embedding: vec![0.0; 1536],
        model: "test".to_string(),
        tokens: 10,
        duration: Duration::from_millis(100),
    };

    // 添加缓存
    cache.put("test".to_string(), result.clone()).await;

    // 立即获取（应该成功）
    let cached = cache.get("test").await;
    assert!(cached.is_some());

    // 等待 TTL 过期
    tokio::time::sleep(Duration::from_millis(150)).await;

    // 再次获取（应该失败）
    let cached = cache.get("test").await;
    assert!(cached.is_none());
}
