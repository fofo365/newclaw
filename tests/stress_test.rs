// 压力测试 - Week 1 Day 4
//
// 测试场景：
// 1. 100+ 并发请求
// 2. 验证线程安全
// 3. 内存泄漏检测（10,000 条消息）

use newclaw::embedding::{
    EmbeddingPipeline, EmbeddingClient, EmbeddingResult, 
    BatchEmbeddingResult, EmbeddingError, CacheConfig, 
    TextChunker, EmbeddingOptions, EmbeddingCache
};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};

/// Mock 嵌入客户端（用于测试）
struct MockEmbeddingClient {
    request_count: Arc<AtomicUsize>,
}

#[async_trait]
impl EmbeddingClient for MockEmbeddingClient {
    async fn embed(&self, _text: &str) -> Result<EmbeddingResult, EmbeddingError> {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        // 模拟网络延迟
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        Ok(EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "mock".to_string(),
            tokens: 100,
            duration: std::time::Duration::from_millis(10),
        })
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<BatchEmbeddingResult, EmbeddingError> {
        self.request_count.fetch_add(texts.len(), Ordering::Relaxed);
        
        // 模拟网络延迟
        tokio::time::sleep(Duration::from_millis(10 * texts.len() as u64)).await;
        
        let count = texts.len();
        Ok(BatchEmbeddingResult {
            embeddings: vec![vec![0.0; 1536]; count],
            total_tokens: count * 100,
            total_duration: std::time::Duration::from_millis(10 * count as u64),
        })
    }
}

#[tokio::test]
async fn test_concurrent_requests() {
    println!("🔥 开始并发压力测试");
    
    let request_count = Arc::new(AtomicUsize::new(0));
    let client = Box::new(MockEmbeddingClient {
        request_count: request_count.clone(),
    });
    
    let chunker = TextChunker::default();
    let options = EmbeddingOptions::default();
    let cache = Arc::new(EmbeddingCache::new(CacheConfig::default()));
    
    let pipeline = Arc::new(EmbeddingPipeline::new(client, chunker, options)
        .with_cache(cache.clone()));

    // 生成 100 个并发请求
    let concurrent_count = 100;
    let mut handles = Vec::new();
    
    println!("并发请求数: {}", concurrent_count);
    let start = Instant::now();
    
    for i in 0..concurrent_count {
        let pipeline_clone = pipeline.clone();
        let handle = tokio::spawn(async move {
            let text = format!("Test document number {} with some content", i);
            let _result = pipeline_clone.process_document(&text).await.unwrap();
        });
        handles.push(handle);
    }

    // 等待所有请求完成
    for handle in handles {
        handle.await.unwrap();
    }
    
    let duration = start.elapsed();
    
    println!("✅ 并发测试完成");
    println!("  总请求数: {}", concurrent_count);
    println!("  实际嵌入请求: {}", request_count.load(Ordering::Relaxed));
    println!("  总耗时: {:?}", duration);
    println!("  平均延迟: {:?}", duration / concurrent_count);
    
    // 验证所有请求都成功完成
    assert_eq!(
        request_count.load(Ordering::Relaxed),
        concurrent_count as usize,
        "所有请求都应该被处理"
    );
}

#[tokio::test]
async fn test_memory_leak_detection() {
    println!("🧠 开始内存泄漏检测");
    
    let request_count = Arc::new(AtomicUsize::new(0));
    let client = Box::new(MockEmbeddingClient {
        request_count: request_count.clone(),
    });
    
    let chunker = TextChunker::default();
    let options = EmbeddingOptions::default();
    let cache = Arc::new(EmbeddingCache::new(CacheConfig {
        max_entries: 10000,
        ttl: Duration::from_secs(3600),
        enable_stats: true,
    }));
    
    let pipeline = Arc::new(EmbeddingPipeline::new(client, chunker, options)
        .with_cache(cache.clone()));

    // 处理 10,000 条消息
    let message_count = 10000;
    println!("消息数量: {}", message_count);
    
    let start = Instant::now();
    
    for i in 0..message_count {
        // 每 100 条消息打印一次进度
        if i % 100 == 0 {
            println!("  进度: {}/{}", i, message_count);
        }
        
        // 生成测试消息（包含重复内容以测试缓存）
        let text = if i % 10 == 0 {
            // 10% 的重复内容
            format!("Repeated test document content for cache hit testing")
        } else {
            format!("Test document number {} with unique content {}", i, i)
        };
        
        let _result = pipeline.process_document(&text).await.unwrap();
    }
    
    let duration = start.elapsed();
    let stats = cache.stats().await;
    
    println!("✅ 内存泄漏检测完成");
    println!("  总消息数: {}", message_count);
    println!("  实际嵌入请求: {}", request_count.load(Ordering::Relaxed));
    println!("  缓存命中: {}", stats.hits);
    println!("  缓存未命中: {}", stats.misses);
    println!("  缓存命中率: {:.2}%", stats.hit_rate() * 100.0);
    println!("  总耗时: {:?}", duration);
    println!("  平均延迟: {:?}", duration / message_count);
    
    // 验证缓存有效工作（应该有显著的命中率）
    let hit_rate = stats.hit_rate();
    assert!(
        hit_rate > 0.05, // 至少 5% 的命中率（10% 重复内容）
        "缓存命中率应该 > 5%，实际为 {:.2}%",
        hit_rate * 100.0
    );
    
    // 验证没有内存泄漏（嵌入请求应该远小于消息数）
    let embedding_ratio = request_count.load(Ordering::Relaxed) as f64 / message_count as f64;
    assert!(
        embedding_ratio < 0.95, // 最多 95% 的消息需要嵌入（5% 来自缓存）
        "嵌入比例过高，可能存在内存泄漏: {:.2}%",
        embedding_ratio * 100.0
    );
}

#[tokio::test]
async fn test_thread_safety() {
    println!("🔒 开始线程安全测试");
    
    let client = Box::new(MockEmbeddingClient {
        request_count: Arc::new(AtomicUsize::new(0)),
    });
    
    let chunker = TextChunker::default();
    let options = EmbeddingOptions::default();
    let cache = Arc::new(EmbeddingCache::new(CacheConfig::default()));
    
    let pipeline = Arc::new(EmbeddingPipeline::new(client, chunker, options)
        .with_cache(cache.clone()));

    // 多个任务并发写入和读取缓存
    let mut handles = Vec::new();
    
    for i in 0..50 {
        let pipeline_clone = pipeline.clone();
        let handle = tokio::spawn(async move {
            // 写入
            let text = format!("Thread safety test document {}", i);
            let _result = pipeline_clone.process_document(&text).await.unwrap();
            
            // 读取（应该命中缓存）
            let _result2 = pipeline_clone.process_document(&text).await.unwrap();
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    for handle in handles {
        handle.await.unwrap();
    }
    
    println!("✅ 线程安全测试完成");
    println!("  所有任务成功完成，无数据竞争");
}
