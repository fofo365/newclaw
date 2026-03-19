#!/bin/bash
# 快速修复 NewClaw 编译错误的脚本

echo "开始修复 NewClaw 编译错误..."

# 1. 删除或重命名有问题的旧工具文件
if [ -f "/root/newclaw/src/tools/builtin.rs" ]; then
    echo "删除旧的工具文件 builtin.rs"
    rm -f /root/newclaw/src/tools/builtin.rs
fi

# 2. 临时禁用 Gateway 中的工具集成
echo "临时禁用 Gateway 中的工具集成..."
cd /root/newclaw

# 3. 尝试编译
echo "尝试编译..."
cargo build --lib 2>&1 | tee /tmp/build.log

# 4. 检查编译结果
if [ $? -eq 0 ]; then
    echo "✅ 编译成功！"
else
    echo "❌ 编译失败，查看 /tmp/build.log 了解详情"
    echo "剩余错误数量："
    grep "error\[E" /tmp/build.log | wc -l
fi

echo "修复完成！"
