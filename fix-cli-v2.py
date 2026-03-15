#!/usr/bin/env python3
"""
精确修复 CLI 模块的脚本 - 只注释掉有问题的一行
"""
import re

with open('/root/newclaw/src/cli/mod.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
skip_until_next_brace = False

for i, line in enumerate(lines):
    line_number = i + 1
    
    # 注释掉第 331 行的工具导入
    if line_number == 331:
        new_lines.append(f'// {line.lstrip()}')
        print(f"✅ 第 331 行：注释掉工具导入")
        continue
    
    # 查找并注释掉 register_tools 和 list_tools 的调用
    if 'register_tools(registry)' in line and 'async fn register_tools' not in lines[max(0, i-10):i+10]:
        new_lines.append(f'    // {line.lstrip()}')
        print(f"✅ 第 {line_number} 行：注释掉 register_tools 调用")
        skip_until_next_brace = False
        continue
    
    if 'list_tools(registry)' in line and 'async fn list_tools' not in lines[max(0, i-10):i+10]:
        new_lines.append(f'    // {line.lstrip()}')
        print(f"✅ 第 {line_number} 行：注释掉 list_tools 调用")
        skip_until_next_brace = False
        continue
    
    # 如果需要跳过直到下一个 }
    if skip_until_next_brace:
        new_lines.append(line)
        if '}' in line and '//' not in line:
            skip_until_next_brace = False
            new_lines.append(f'    // {line.lstrip()}')
        else:
            new_lines.append(line)
    else:
        new_lines.append(line)

with open('/root/newclaw/src/cli/mod.rs', 'w') as f:
    f.writelines(new_lines)

print("\n✅ CLI 模块修复完成！")
