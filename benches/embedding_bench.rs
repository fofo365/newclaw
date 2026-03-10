// NewClaw v0.5.0 - 嵌入性能基准测试
//
// 测试目标：
// 1. 单条嵌入延迟 < 500ms
// 2. 批量嵌入吞吐量 > 100 req/s
// 3. 缓存命中延迟 < 1ms
// 4. BatchOptimizer 吞吐量提升 > 30%

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use newclaw::embedding::{EmbeddingClient, EmbeddingPipeline, OpenAIEmbedding};
use newclaw::embedding::cache::EmbeddingCache;
use newclaw::embedding::config::EmbeddingConfig;
use tokio::runtime::Runtime;

/// 基准测试配置
fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(50)  // 减少样本数，加快测试速度
        .warm_up_time(std::time::Duration::from_secs(3))
        .measurement_time(std::time::Duration::from_secs(10))
}

/// 1. 单条嵌入延迟基准测试
fn bench_embed_single(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let config = EmbeddingConfig {
        provider: "openai".to_string(),
        model: "text-embedding-3-small".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test".to_string()),
        batch_size: 1,
        timeout_ms: 5000,
    };

    let client = OpenAIEmbedding::new(config);
    let text = "This is a test message for embedding benchmarking.";

    c.bench_function("embed_single", |b| {
        b.to_async(&rt).iter(|| {
            let client = &client;
            let text = black_box(text);
            async move {
                // 注意：这里需要实际的 API key 才能运行
                // 测试时会跳过，仅用于建立基线
                let _ = client.embed(text).await;
            }
        });
    });
}

/// 2. 批量嵌入吞吐量基准测试
fn bench_embed_batch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("embed_batch");
    
    for batch_size in [10, 50, 100].iter() {
        let config = EmbeddingConfig {
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "test".to_string()),
            batch_size: *batch_size,
            timeout_ms: 10000,
        };

        let client = OpenAIEmbedding::new(config);
        let texts: Vec<String> = (0..*batch_size)
            .map(|i| format!("Test message number {}", i))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(batch_size), batch_size, |b, _| {
            b.to_async(&rt).iter(|| {
                let client = &client;
                let texts = black_box(&texts);
                async move {
                    let _ = client.embed_batch(texts).await;
                }
            });
        });
    }
    
    group.finish();
}

/// 3. 缓存性能基准测试
fn bench_cache_hit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = EmbeddingCache::new(1000);
    
    // 预填充缓存
    let text = "Cached test message";
    let embedding = vec![0.1f32; 1536];
    rt.block_on(async {
        cache.insert(text.to_string(), embedding.clone()).await.unwrap();
    });

    c.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| {
            let cache = &cache;
            async move {
                let _ = cache.get(black_box(text)).await;
            }
        });
    });
}

/// 4. 缓存未命中性能基准测试
fn bench_cache_miss(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = EmbeddingCache::new(1000);
    
    c.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| {
            let cache = &cache;
            async move {
                let _ = cache.get(black_box("nonexistent message")).await;
            }
        });
    });
}

/// 5. TextChunker 性能基准测试
fn bench_text_chunker(c: &mut Criterion) {
    use newclaw::embedding::chunker::TextChunker;
    
    let mut group = c.benchmark_group("text_chunker");
    
    for text_size in [100, 1000, 10000].iter() {
        let text = "a".repeat(*text_size);
        
        group.bench_with_input(BenchmarkId::from_parameter(text_size), text_size, |b, _| {
            b.iter(|| {
                let chunker = TextChunker::new(500, 50);
                let _ = chunker.chunk(black_box(&text));
            })
        });
    }
    
    group.finish();
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = bench_embed_single, bench_embed_batch, bench_cache_hit, bench_cache_miss, bench_text_chunker
}

criterion_main!(benches);
