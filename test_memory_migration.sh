#!/bin/bash
# 记忆迁移测试脚本

set -e

echo "🧪 测试记忆迁移功能..."
echo ""

# 设置 PATH
export PATH="$HOME/.cargo/bin:$PATH"

# 运行测试
echo "1️⃣ 运行单元测试..."
cargo test --lib memory --no-fail-fast -- --nocapture

echo ""
echo "✅ 所有测试通过！"
echo ""

# 测试自动迁移
echo "2️⃣ 测试自动迁移功能..."
echo "   创建测试目录..."
TEST_DIR=$(mktemp -d)
OPENCLAW_WORKSPACE="/root/.openclaw/workspace"

echo "   运行迁移..."
# 这里需要在 Rust 代码中调用 auto_migrate
# 暂时手动测试

echo ""
echo "3️⃣ 验证迁移结果..."
if [ -f "$TEST_DIR/data/memory/MEMORY.md" ]; then
    echo "   ✅ 长期记忆已迁移"
else
    echo "   ❌ 长期记忆未迁移"
fi

if [ -d "$TEST_DIR/data/memory/daily" ]; then
    FILE_COUNT=$(ls -1 "$TEST_DIR/data/memory/daily"/*.md 2>/dev/null | wc -l)
    echo "   ✅ 每日日志已迁移: $FILE_COUNT 个文件"
else
    echo "   ❌ 每日日志未迁移"
fi

# 清理
rm -rf "$TEST_DIR"

echo ""
echo "✅ 记忆迁移测试完成！"
