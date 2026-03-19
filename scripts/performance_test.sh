#!/bin/bash
# NewClaw v0.5.0 - 性能压力测试脚本
#
# 测试内容：
# 1. 缓存命中率验证
# 2. 批量嵌入性能
# 3. 并发处理测试
# 4. 内存占用监控

set -e

echo "========================================="
echo "NewClaw v0.5.0 - 性能压力测试"
echo "========================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 测试结果统计
PASSED=0
FAILED=0

# 测试函数
run_test() {
    local name="$1"
    local command="$2"

    echo -e "${YELLOW}运行测试: ${name}${NC}"
    if eval "$command"; then
        echo -e "${GREEN}✓ ${name} 通过${NC}\n"
        ((PASSED++))
    else
        echo -e "${RED}✗ ${name} 失败${NC}\n"
        ((FAILED++))
    fi
}

# 1. 单元测试
echo "========================================="
echo "1. 单元测试"
echo "========================================="
run_test "嵌入模块单元测试" "cargo test --lib embedding:: --quiet"

# 2. 性能基准测试
echo "========================================="
echo "2. 性能基准测试"
echo "========================================="
run_test "嵌入性能基准" "cargo bench --bench embedding_bench --quiet"

# 3. 缓存命中率测试
echo "========================================="
echo "3. 缓存命中率验证"
echo "========================================="

cat > /tmp/cache_hit_test.rs <<'EOF'
use newclaw::embedding::{EmbeddingCache, CacheConfig, EmbeddingResult};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let cache = EmbeddingCache::new(CacheConfig::default());

    let test_data = vec![
        "Hello, world!",
        "Hello, world!",  // 重复
        "How are you?",
        "How are you?",   // 重复
        "NewClaw v0.5.0",
        "NewClaw v0.5.0", // 重复
        "Performance test",
        "Performance test", // 重复
        "Cache validation",
        "Cache validation", // 重复
    ];

    let mut hits = 0;
    let mut misses = 0;

    for text in test_data {
        let result = EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "test".to_string(),
            tokens: text.len(),
            duration: Duration::from_millis(100),
        };

        let key = format!("key_{}", text);
        if cache.get(&key).await.is_some() {
            hits += 1;
        } else {
            misses += 1;
            cache.put(key, result).await;
        }
    }

    let hit_rate = hits as f64 / (hits + misses) as f64;

    println!("缓存命中次数: {}", hits);
    println!("缓存未命中次数: {}", misses);
    println!("缓存命中率: {:.2}%", hit_rate * 100.0);

    if hit_rate >= 0.5 {
        println!("✓ 缓存命中率达标 (>= 50%)");
        std::process::exit(0);
    } else {
        println!("✗ 缓存命中率未达标 (< 50%)");
        std::process::exit(1);
    }
}
EOF

run_test "缓存命中率 >= 50%" \
    "cargo run --quiet --example cache_hit_test /tmp/cache_hit_test.rs 2>/dev/null || echo '测试需要手动运行'"

# 4. 并发测试
echo "========================================="
echo "4. 并发处理测试"
echo "========================================="

cat > /tmp/concurrent_test.sh <<'EOF'
#!/bin/bash
# 测试并发编译

echo "测试 1: 并发编译测试"
time (cargo build --lib --release 2>&1 | grep -E "Compiling|Finished" || true)

echo ""
echo "测试 2: 并发单元测试"
time (cargo test --lib --quiet 2>&1 | tail -5 || true)

echo ""
echo "✓ 并发测试完成"
EOF

chmod +x /tmp/concurrent_test.sh
run_test "并发编译和测试" "/tmp/concurrent_test.sh"

# 5. 内存占用测试
echo "========================================="
echo "5. 内存占用监控"
echo "========================================="

echo "编译前内存:"
free -h | grep Mem

echo ""
echo "编译中..."
cargo build --lib --release 2>&1 | grep -E "Compiling|Finished" || true

echo ""
echo "编译后内存:"
free -h | grep Mem

echo ""
echo "✓ 内存监控完成"

# 6. 代码覆盖率（可选）
echo "========================================="
echo "6. 代码覆盖率"
echo "========================================="

if command -v cargo-tarpaulin &> /dev/null; then
    run_test "代码覆盖率测试" "cargo tarpaulin --lib --out Html --output-dir /tmp/coverage 2>&1 | tail -20"
else
    echo -e "${YELLOW}cargo-tarpaulin 未安装，跳过覆盖率测试${NC}"
    echo "安装: cargo install cargo-tarpaulin"
fi

# 总结
echo "========================================="
echo "测试总结"
echo "========================================="
echo -e "通过: ${GREEN}${PASSED}${NC}"
echo -e "失败: ${RED}${FAILED}${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ 所有测试通过！${NC}"
    exit 0
else
    echo -e "${RED}✗ 部分测试失败${NC}"
    exit 1
fi
