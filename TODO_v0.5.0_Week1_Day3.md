# NewClaw v0.5.0 Week 1 Day 3 任务清单

**日期**: 2026-03-10
**模型**: GLM-5 (高性能推理)
**任务**: 性能基准测试和验证

---

## 当前状态

### Week 1 进度: 50% (Day 2/4 完成)

✅ **已完成**:
- Day 1: 嵌入模型选型 + 客户端实现 + 向量化流水线
- Day 2: 性能优化（BatchOptimizer + 缓存集成）

⏳ **进行中**:
- Day 3-4: 性能基准测试和验证

📊 **代码统计**: 24,357 行（新增 ~300 行）

---

## 今日任务 (Day 3)

### 1. 创建性能基准测试套件

**目标**: 建立性能基准，验证优化效果

#### 1.1 嵌入性能基准

**文件**: `benches/embedding_bench.rs`

**测试项**:
```rust
// 1. 单条嵌入延迟
fn bench_embed_single(c: &mut Criterion) {
    // 目标: < 500ms
}

// 2. 批量嵌入吞吐量
fn bench_embed_batch(c: &mut Criterion) {
    // 批次大小: 10, 50, 100
    // 目标: > 100 req/s
}

// 3. 缓存性能
fn bench_cache_hit(c: &mut Criterion) {
    // 目标: < 1ms (vs ~500ms 未命中)
}

// 4. BatchOptimizer 吞吐量
fn bench_batch_optimizer(c: &mut Criterion) {
    // 目标: +30% vs 无优化
}
```

**验收标准**:
- ✅ 所有基准测试可运行
- ✅ 生成性能报告
- ✅ 建立性能基线

---

### 2. 集成测试：缓存命中率验证

**目标**: 验证真实场景下的缓存效果

#### 2.1 测试场景设计

**场景**: 处理 1000+ 文档，30% 重复查询

```rust
#[tokio::test]
async fn test_cache_hit_rate() {
    // 1. 生成 1000 个测试文档
    let documents = generate_test_documents(1000);

    // 2. 首次嵌入（全部缓存未命中）
    let pipeline = EmbeddingPipeline::new()
        .with_cache()
        .build();

    for doc in &documents {
        pipeline.embed(doc).await.unwrap();
    }

    // 3. 重复查询 30%（验证缓存命中）
    let repeat_indices = select_random_indices(300);
    for i in repeat_indices {
        pipeline.embed(&documents[i]).await.unwrap();
    }

    // 4. 验证缓存命中率
    let stats = pipeline.cache_stats();
    assert!(stats.hit_rate > 0.80, "Cache hit rate should be > 80%");
}
```

**验收标准**:
- ✅ 缓存命中率 > 80%
- ✅ 缓存命中延迟 < 1ms
- ✅ 总体吞吐量提升 > 2x

---

### 3. 压力测试：并发请求验证

**目标**: 验证线程安全和并发性能

#### 3.1 并发嵌入测试

```rust
#[tokio::test]
async fn test_concurrent_embedding() {
    // 1. 创建 100 个并发任务
    let tasks = (0..100)
        .map(|_| {
            let pipeline = pipeline.clone();
            tokio::spawn(async move {
                pipeline.embed("Test message").await
            })
        })
        .collect::<Vec<_>>();

    // 2. 等待所有任务完成
    let results = futures::future::join_all(tasks).await;

    // 3. 验证无错误
    for result in results {
        assert!(result.is_ok());
    }
}
```

#### 3.2 BatchOptimizer 并发测试

```rust
#[tokio::test]
async fn test_batch_optimizer_concurrent() {
    // 1. 创建 1000 个并发请求
    let optimizer = BatchOptimizer::new(config);

    let tasks = (0..1000)
        .map(|i| {
            let optimizer = optimizer.clone();
            tokio::spawn(async move {
                optimizer.process(format!("Message {}", i)).await
            })
        })
        .collect::<Vec<_>>();

    // 2. 等待所有任务完成
    let results = futures::future::join_all(tasks).await;

    // 3. 验证无丢失
    assert_eq!(results.len(), 1000);
}
```

**验收标准**:
- ✅ 100+ 并发请求无错误
- ✅ 无数据竞争
- ✅ 无内存泄漏（Valgrind 检查）

---

### 4. 内存泄漏检测

**目标**: 确保长时间运行无内存泄漏

#### 4.1 长时间运行测试

```rust
#[tokio::test]
async fn test_memory_leak() {
    let pipeline = EmbeddingPipeline::new()
        .with_cache()
        .build();

    // 记录初始内存
    let initial_memory = get_memory_usage();

    // 处理 10,000 条消息
    for i in 0..10_000 {
        pipeline.embed(&format!("Message {}", i)).await.unwrap();

        // 每 1000 条检查一次
        if i % 1000 == 0 {
            let current_memory = get_memory_usage();
            let growth = current_memory - initial_memory;

            // 内存增长不应超过 100MB
            assert!(growth < 100 * 1024 * 1024);
        }
    }
}
```

**验收标准**:
- ✅ 处理 10,000 条消息后内存增长 < 100MB
- ✅ 缓存 LRU 淘汰机制正常工作
- ✅ 无明显内存泄漏

---

## 执行计划

### 优先级 P0 (今日必须完成)

1. **创建基准测试套件**（2 小时）
   - [ ] 实现 `benches/embedding_bench.rs`
   - [ ] 添加 4 个基准测试
   - [ ] 验证可运行

2. **运行基准测试**（1 小时）
   - [ ] 运行 `cargo bench --bench embedding_bench`
   - [ ] 收集性能数据
   - [ ] 生成性能报告

3. **缓存命中率验证**（2 小时）
   - [ ] 实现 `test_cache_hit_rate`
   - [ ] 运行测试
   - [ ] 分析结果

### 优先级 P1 (今日尽量完成)

4. **压力测试**（2 小时）
   - [ ] 实现并发测试
   - [ ] 运行 100+ 并发
   - [ ] 验证线程安全

5. **内存泄漏检测**（1 小时）
   - [ ] 实现长时间运行测试
   - [ ] 检查内存增长
   - [ ] 修复泄漏（如有）

---

## 验收标准

### 性能指标

| 指标 | 目标 | 实际 | 状态 |
|------|------|------|------|
| 单条嵌入延迟 | < 500ms | ___ | ⏳ |
| 批量嵌入吞吐量 | > 100 req/s | ___ | ⏳ |
| 缓存命中延迟 | < 1ms | ___ | ⏳ |
| 缓存命中率 | > 80% | ___ | ⏳ |
| 并发请求 | 100+ 无错误 | ___ | ⏳ |
| 内存增长（10K 条） | < 100MB | ___ | ⏳ |

### 功能完整性

- [ ] 所有基准测试可运行
- [ ] 所有集成测试通过
- [ ] 性能报告生成
- [ ] 无内存泄漏
- [ ] 代码提交到 Git

---

## 预期成果

### 交付物

1. **基准测试套件**
   - `benches/embedding_bench.rs`
   - 4 个基准测试
   - 性能基线数据

2. **集成测试**
   - 缓存命中率测试
   - 并发请求测试
   - 内存泄漏测试

3. **性能报告**
   - 性能指标表格
   - 优化前后对比
   - 瓶颈分析

### Git 提交

```bash
git add benches/embedding_bench.rs
git add tests/embedding_integration_test.rs
git commit -m "feat: v0.5.0 Week 1 Day 3 - 性能基准测试完成"
```

---

## 下一步 (Day 4)

如果 Day 3 顺利完成，Day 4 将进行：

1. **性能调优**
   - 分析瓶颈
   - 优化热点路径
   - 验证优化效果

2. **文档完善**
   - API 文档
   - 使用示例
   - 性能指南

3. **Week 1 总结**
   - 整理成果
   - 评估进度
   - 规划 Week 2

---

**开始时间**: 2026-03-10 14:00 UTC+8
**预计完成**: 2026-03-10 18:00 UTC+8
**状态**: ⏳ 进行中
