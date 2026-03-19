#!/usr/bin/env python3
"""
快速修复 CLI 模块的脚本
"""
import re

with open('/root/newclaw/src/cli/mod.rs', 'r') as f:
    content = f.read()

# 找到 register_tools 函数并注释掉
pattern = r'(async fn register_tools\(registry: &ToolRegistry\) \{.*?^\})'
match = re.search(pattern, content, re.DOTALL)
if match:
    func_start = match.start()
    # 找到对应的缩进
    indent_match = re.search(r'^(\s*)', content[:func_start], re.MULTILINE)
    indent = indent_match.group(1) if indent_match else '    '
    
    # 注释整个函数
    commented_func = re.sub(r'^', '// ', match.group(0), flags=re.MULTILINE)
    content = content[:match.start()] + '// TODO: 工具注册暂时禁用\n' + commented_func + content[match.end():]
    
    with open('/root/newclaw/src/cli/mod.rs', 'w') as f:
        f.write(content)
    
    print("✅ 已注释掉 register_tools 函数")
else:
    print("⚠️ 未找到 register_tools 函数")

# 同样处理 list_tools 函数
pattern = r'(async fn list_tools\(registry: &ToolRegistry\) \{.*?^\})'
match = re.search(pattern, content, re.DOTALL)
if match:
    func_start = match.start()
    indent_match = re.search(r'^(\s*)', content[:func_start], re.MULTILINE)
    indent = indent_match.group(1) if indent_match else '    '
    
    commented_func = re.sub(r'^', '// ', match.group(0), flags=re.MULTILINE)
    content = content[:match.start()] + '// TODO: 工具列表暂时禁用\n' + commented_func + content[match.end():]
    
    with open('/root/newclaw/src/cli/mod.rs', 'w') as f:
        f.write(content)
    
    print("✅ 已注释掉 list_tools 函数")
else:
    print("⚠️ 未找到 list_tools 函数")

print("\n修复完成！")
