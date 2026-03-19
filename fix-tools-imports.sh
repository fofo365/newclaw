#!/bin/bash
# 批量修复 tools 模块的类型导入问题

cd /root/newclaw

# 在需要的文件中添加 JsonValue 类型别名导入
for file in src/tools/files/write.rs src/tools/files/edit.rs src/tools/exec/exec.rs src/tools/exec/process.rs; do
  if ! grep -q "use serde_json::Value as JsonValue" "$file"; then
    # 在第一个 use 语句后添加 JsonValue 导入
    sed -i '0,/^use /s//use serde_json::Value as JsonValue;\n\nuse /' "$file"
  fi
done

# 修复 web 模块中的 ToolResult 类型
for file in src/tools/web/fetch.rs src/tools/web/search.rs; do
  # 移除 ToolResult 导入（如果存在）
  sed -i 's/, ToolResult//' "$file"
  sed -i 's/ToolResult,/anyhow::Result<JsonValue>,/' "$file"
  sed -i 's/-> ToolResult {/-> anyhow::Result<JsonValue> {/' "$file"
done

echo "Fixes applied"
