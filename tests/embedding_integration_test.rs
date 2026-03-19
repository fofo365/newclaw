// NewClaw v0.5.0 - 嵌入集成测试
//
// 测试场景：
// 1. 端到端文档处理
// 2. 缓存效果验证
// 3. 批量处理性能
// 4. 错误处理和重试

use newclaw::embedding::{
    EmbeddingClient, OpenAIEmbeddingClient, EmbeddingConfig,
    EmbeddingPipeline, TextChunker, EmbeddingCache, CacheConfig,
    EmbeddingModel, EmbeddingOptions,
};
use std::time::Duration;
use std::sync::Arc;

#[tokio::test]
#[ignore] // 需要 OPENAI_API_KEY
async fn test_end_to_end_embedding() {
    // 创建客户端
    let config = EmbeddingConfig {
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        TextChunker::default(),
        EmbeddingOptions::default(),
    );

    // 测试文档
    let document = "NewClaw is a next-generation AI agent framework written in Rust. \
        It provides intelligent context management, multi-LLM support, and enterprise-grade security. \
        The v0.5.0 release focuses on vector embeddings and semantic search capabilities.";

    // 处理文档
    let result = pipeline.process_document(document).await.unwrap();

    // 验证结果
    assert!(!result.embeddings.is_empty(), "应该生成嵌入向量");
    assert!(result.total_tokens > 0, "应该消耗 tokens");
    assert!(result.total_duration.as_millis() > 0, "应该有处理时间");

    println!("嵌入向量数量: {}", result.embeddings.len());
    println!("总 tokens: {}", result.total_tokens);
    println!("处理时间: {:?}", result.total_duration);
    println!("分块数量: {}", result.chunk_count);

    // 验证嵌入维度
    for embedding in &result.embeddings {
        assert_eq!(embedding.embedding.len(), 1536, "嵌入维度应该是 1536");
    }
}

#[tokio::test]
#[ignore] // 需要 OPENAI_API_KEY
async fn test_cache_effectiveness() {
    // 创建带缓存的 pipeline
    let config = EmbeddingConfig {
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let cache = Arc::new(EmbeddingCache::new(CacheConfig::default()));

    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        TextChunker::default(),
        EmbeddingOptions::default(),
    ).with_cache(cache.clone());

    // 测试文档（相同内容）
    let document = "This is a test document for cache validation. \
        NewClaw v0.5.0 implements intelligent caching to improve performance.";

    // 第一次处理（缓存未命中）
    let start = std::time::Instant::now();
    let result1 = pipeline.process_document(document).await.unwrap();
    let duration1 = start.elapsed();

    println!("第一次处理: {:?}", duration1);
    println!("缓存命中: {}", result1.cache_hits);
    println!("缓存命中率: {:.2}%", result1.cache_hit_rate * 100.0);

    // 第二次处理（缓存命中）
    let start = std::time::Instant::now();
    let result2 = pipeline.process_document(document).await.unwrap();
    let duration2 = start.elapsed();

    println!("第二次处理: {:?}", duration2);
    println!("缓存命中: {}", result2.cache_hits);
    println!("缓存命中率: {:.2}%", result2.cache_hit_rate * 100.0);

    // 验证缓存效果
    assert!(result2.cache_hits > 0, "第二次应该有缓存命中");
    assert!(result2.cache_hit_rate > 0.5, "缓存命中率应该 > 50%");
    assert!(duration2 < duration1, "第二次应该更快（缓存）");

    println!("性能提升: {:.2}x", duration1.as_millis() as f64 / duration2.as_millis() as f64);
}

#[tokio::test]
#[ignore] // 需要 OPENAI_API_KEY
async fn test_batch_processing() {
    let config = EmbeddingConfig {
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        TextChunker::default(),
        EmbeddingOptions::default(),
    );

    // 多个文档
    let documents = vec![
        "Document 1: Introduction to NewClaw framework.",
        "Document 2: Architecture and design principles.",
        "Document 3: Performance optimization techniques.",
        "Document 4: Security best practices.",
        "Document 5: Deployment and operations guide.",
    ];

    // 批量处理
    let start = std::time::Instant::now();
    let results = pipeline.process_documents(documents.iter().map(|s| s.to_string()).collect()).await.unwrap();
    let total_duration = start.elapsed();

    println!("处理文档数: {}", results.len());
    println!("总时间: {:?}", total_duration);
    println!("平均时间: {:?}", total_duration / results.len() as u32);

    // 验证结果
    assert_eq!(results.len(), 5, "应该处理 5 个文档");

    for (i, result) in results.iter().enumerate() {
        println!("文档 {}: {} 个向量, {} tokens",
            i + 1, result.embeddings.len(), result.total_tokens);
        assert!(!result.embeddings.is_empty(), "每个文档都应该有嵌入");
    }
}

#[tokio::test]
#[ignore] // 需要 OPENAI_API_KEY
async fn test_large_document() {
    let config = EmbeddingConfig {
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        TextChunker::default(),
        EmbeddingOptions::default(),
    );

    // 大文档（~5000 字符）
    let large_document = "NewClaw v0.5.0 represents a significant leap forward in AI agent capabilities. \
        The framework now includes intelligent context management, vector embeddings, and semantic search. \
        ".repeat(20);

    println!("文档大小: {} 字符", large_document.len());

    // 处理大文档
    let start = std::time::Instant::now();
    let result = pipeline.process_document(&large_document).await.unwrap();
    let duration = start.elapsed();

    println!("处理时间: {:?}", duration);
    println!("分块数量: {}", result.chunk_count);
    println!("总 tokens: {}", result.total_tokens);

    // 验证分块
    assert!(result.chunk_count > 1, "大文档应该被分块");
    assert!(result.total_tokens > 0, "应该消耗 tokens");

    // 验证性能
    let tokens_per_second = result.total_tokens as f64 / duration.as_secs_f64();
    println!("吞吐量: {:.2} tokens/秒", tokens_per_second);
    assert!(tokens_per_second > 100.0, "吞吐量应该 > 100 tokens/秒");
}

#[tokio::test]
#[ignore] // 需要 OPENAI_API_KEY
async fn test_cache_hit_rate() {
    let config = EmbeddingConfig {
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let cache = Arc::new(EmbeddingCache::new(CacheConfig::default()));

    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        TextChunker::default(),
        EmbeddingOptions::default(),
    ).with_cache(cache);

    // 重复文档（测试缓存）
    let documents = vec![
        "Document A: Unique content",
        "Document B: Unique content",
        "Document A: Unique content",  // 重复
        "Document C: Unique content",
        "Document B: Unique content",  // 重复
    ];

    let mut total_hits = 0;
    let mut total_requests = 0;

    for doc in &documents {
        let result = pipeline.process_document(doc).await.unwrap();
        total_hits += result.cache_hits;
        total_requests += result.chunk_count;
        println!("文档: '{}', 缓存命中: {}, 命中率: {:.2}%",
            &doc[..20], result.cache_hits, result.cache_hit_rate * 100.0);
    }

    let overall_hit_rate = total_hits as f64 / total_requests as f64;
    println!("总体缓存命中率: {:.2}%", overall_hit_rate * 100.0);

    // 验证缓存效果
    assert!(overall_hit_rate > 0.3, "总体缓存命中率应该 > 30%");
}

#[tokio::test]
async fn test_error_handling() {
    // 测试错误处理（不需要 API Key）
    let config = EmbeddingConfig {
        api_key: "invalid_key".to_string(),
        model: EmbeddingModel::OpenAI3Small,
        ..Default::default()
    };

    let client = OpenAIEmbeddingClient::new(config);
    let pipeline = EmbeddingPipeline::new(
        Box::new(client),
        TextChunker::default(),
        EmbeddingOptions::default(),
    );

    // 应该返回错误
    let result = pipeline.process_document("Test document").await;

    assert!(result.is_err(), "无效 API Key 应该返回错误");

    if let Err(e) = result {
        println!("正确捕获错误: {:?}", e);
    }
}
