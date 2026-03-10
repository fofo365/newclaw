// Embedding Performance Benchmarks - v0.5.0
//
// 性能基准测试：
// - Token 计数性能
// - 嵌入延迟
// - 缓存性能
// - 分块性能

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use newclaw::embedding::{TextChunker, EmbeddingCache, EmbeddingResult};
use newclaw::context::TokenCounter;
use std::time::Duration;

/// Token 计数基准测试
fn bench_token_counting(c: &mut Criterion) {
    let counter = TokenCounter::default();

    let mut group = c.benchmark_group("token_counting");

    // 短文本（英文）
    group.bench_function("short_english", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut counter = TokenCounter::new().unwrap();
        let text = "The quick brown fox jumps over the lazy dog.";

        b.iter(|| {
            black_box(counter.count_tokens(black_box(text), "gpt-4").unwrap())
        })
    });

    // 短文本（中文）
    group.bench_function("short_chinese", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut counter = TokenCounter::new().unwrap();
        let text = "快速的棕色狐狸跳过懒狗。";

        b.iter(|| {
            black_box(counter.count_tokens(black_box(text), "glm-4").unwrap())
        })
    });

    // 长文本
    group.bench_function("long_text", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut counter = TokenCounter::new().unwrap();
        let text = "A".repeat(10000);

        b.iter(|| {
            black_box(counter.count_tokens(black_box(&text), "gpt-4").unwrap())
        })
    });

    // 多条消息
    group.bench_function("multiple_messages", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut counter = TokenCounter::new().unwrap();
        let messages = vec![
            newclaw::llm::Message {
                role: newclaw::llm::MessageRole::User,
                content: "Hello, how are you?".to_string(),
                tool_call_id: None,
                tool_calls: None,
            },
            newclaw::llm::Message {
                role: newclaw::llm::MessageRole::Assistant,
                content: "I'm doing well, thank you!".to_string(),
                tool_call_id: None,
                tool_calls: None,
            },
        ];

        b.iter(|| {
            black_box(counter.count_messages_tokens(black_box(&messages), "gpt-4").unwrap())
        })
    });

    group.finish();
}

/// 文本分块基准测试
fn bench_text_chunking(c: &mut Criterion) {
    let mut group = c.benchmark_group("text_chunking");

    // 小文档
    group.bench_function("small_document", |b| {
        let chunker = TextChunker::new(1000, 200);
        let text = "A".repeat(500);
        b.iter(|| {
            black_box(chunker.chunk(black_box(&text)))
        })
    });

    // 大文档
    group.bench_function("large_document", |b| {
        let chunker = TextChunker::new(5000, 500);
        let text = "A".repeat(50000);
        b.iter(|| {
            black_box(chunker.chunk(black_box(&text)))
        })
    });

    // 不同文档大小
    group.bench_function("variable_sizes", |b| {
        let chunker = TextChunker::default();
        b.iter(|| {
            let size = (black_box(100) % 10000 + 100) * 10;
            let text = "B".repeat(size);
            black_box(chunker.chunk(&text))
        })
    });

    group.finish();
}

/// Token 估算基准测试
fn bench_token_estimation(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_estimation");

    let chunker = TextChunker::default();

    group.bench_function("estimate_english", |b| {
        let text = "The quick brown fox jumps over the lazy dog. ";
        b.iter(|| {
            black_box(chunker.estimate_tokens(black_box(text)))
        })
    });

    group.bench_function("estimate_chinese", |b| {
        let text = "快速的棕色狐狸跳过懒狗。";
        b.iter(|| {
            black_box(chunker.estimate_tokens(black_box(text)))
        })
    });

    group.bench_function("estimate_mixed", |b| {
        let text = "Hello 你好 World 世界 AI 人工智能";
        b.iter(|| {
            black_box(chunker.estimate_tokens(black_box(text)))
        })
    });

    group.finish();
}

/// 缓存性能基准测试
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    // 缓存写入
    group.bench_function("cache_write", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = EmbeddingCache::new(Default::default());
        let result = EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "test".to_string(),
            tokens: 100,
            duration: Duration::from_millis(100),
        };

        b.iter(|| {
            let key = format!("key_{}", black_box(100));
            rt.block_on(async {
                black_box(cache.put(key, result.clone()).await)
            })
        })
    });

    // 缓存读取（命中）
    group.bench_function("cache_read_hit", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = EmbeddingCache::new(Default::default());
        let result = EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "test".to_string(),
            tokens: 100,
            duration: Duration::from_millis(100),
        };
        rt.block_on(async {
            cache.put("test_key".to_string(), result).await;
        });

        b.iter(|| {
            rt.block_on(async {
                black_box(cache.get(black_box("test_key")).await)
            })
        })
    });

    // 缓存读取（未命中）
    group.bench_function("cache_read_miss", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = EmbeddingCache::new(Default::default());

        b.iter(|| {
            rt.block_on(async {
                black_box(cache.get(black_box("nonexistent_key")).await)
            })
        })
    });

    // 缓存统计
    group.bench_function("cache_stats", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let cache = EmbeddingCache::new(Default::default());

        b.iter(|| {
            rt.block_on(async {
                black_box(cache.stats().await)
            })
        })
    });

    group.finish();
}

/// 缓存不同大小性能
fn bench_cache_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_scaling");

    for size in [100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let cache = EmbeddingCache::new(newclaw::embedding::CacheConfig {
                max_entries: size,
                ..Default::default()
            });

            // 预填充缓存
            rt.block_on(async {
                for i in 0..size {
                    let result = EmbeddingResult {
                        embedding: vec![0.0; 1536],
                        model: "test".to_string(),
                        tokens: 100,
                        duration: Duration::from_millis(100),
                    };
                    cache.put(format!("key_{}", i), result).await;
                }
            });

            b.iter(|| {
                let key = format!("key_{}", black_box(size / 2));
                rt.block_on(async {
                    black_box(cache.get(&key).await)
                })
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_token_counting,
    bench_text_chunking,
    bench_token_estimation,
    bench_cache_performance,
    bench_cache_scaling
);
criterion_main!(benches);
