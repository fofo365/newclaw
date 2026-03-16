# NewClaw 飞书机器人工具调用功能

## 功能概述

飞书机器人现在支持完整的工具调用能力，允许 LLM 查询服务器状态并基于实际数据回复用户。

## 可用工具

### 1. systemctl_status
查看 systemd 服务状态。

**参数：**
- `service` (可选): 服务名称或模式，默认 `newclaw-*`

**示例：**
```json
{
  "name": "systemctl_status",
  "arguments": {"service": "newclaw-*"}
}
```

### 2. ps_list
查看运行中的进程列表。

**参数：**
- `filter` (可选): 进程过滤条件，默认 `newclaw`

**示例：**
```json
{
  "name": "ps_list",
  "arguments": {"filter": "newclaw"}
}
```

### 3. tail_log
查看日志文件尾部内容。

**参数：**
- `file` (可选): 日志文件路径或模式，默认 `/var/log/newclaw/*.log`
- `lines` (可选): 显示行数，默认 `20`

**示例：**
```json
{
  "name": "tail_log",
  "arguments": {
    "file": "/var/log/newclaw/*.log",
    "lines": "20"
  }
}
```

### 4. disk_usage
查看磁盘使用情况。

**参数：**
无

**示例：**
```json
{
  "name": "disk_usage",
  "arguments": {}
}
```

### 5. memory_usage
查看内存使用情况。

**参数：**
无

**示例：**
```json
{
  "name": "memory_usage",
  "arguments": {}
}
```

## 使用场景

### 场景 1：查询服务器状态
**用户：** 服务器状态怎么样？
**LLM 自动调用：**
- `systemctl_status` - 检查服务状态
- `disk_usage` - 检查磁盘使用
- `memory_usage` - 检查内存使用

### 场景 2：查看日志
**用户：** 看下最近的错误日志
**LLM 自动调用：**
- `tail_log` - 读取日志尾部

### 场景 3：检查进程
**用户：** newclaw 进程都在运行吗？
**LLM 自动调用：**
- `ps_list` - 查看进程列表

## 安全特性

1. **白名单机制**：只允许执行预定义的安全命令
2. **命令过滤**：禁止执行任意 shell 命令
3. **错误处理**：工具执行失败时优雅降级
4. **日志记录**：记录所有工具调用和执行结果

## 技术实现

### 核心组件

1. **工具管理器** (`ToolManager`)
   - 定义和管理所有可用工具
   - 执行工具调用
   - 返回格式化的结果

2. **LLM 集成**
   - 自动检测工具调用请求
   - 执行工具并获取结果
   - 将结果返回给 LLM 生成最终回复

3. **多轮对话支持**
   - 最多支持 5 轮工具调用
   - 上下文保持
   - 迭代式问题解决

### 文件结构

```
/root/newclaw/
├── src/
│   ├── bin/
│   │   ├── feishu-connect.rs    # 飞书机器人主程序（支持工具调用）
│   │   └── test-tools.rs        # 工具调用测试程序
│   └── feishu_websocket/
│       ├── mod.rs               # 模块定义（导出工具模块）
│       └── tools.rs             # 工具实现（13846 字节）
```

## 测试

运行测试程序验证工具调用功能：

```bash
/root/newclaw/target/release/test-tools
```

测试覆盖：
- ✅ systemctl_status
- ✅ ps_list
- ✅ tail_log
- ✅ disk_usage
- ✅ memory_usage

## 部署

服务已自动部署并重启：

```bash
systemctl status newclaw-feishu
```

## 日志查看

查看服务日志：

```bash
journalctl -u newclaw-feishu -f
```

查看应用日志：

```bash
tail -f /var/log/newclaw/feishu.stdout.log
```

## 扩展

添加新工具非常简单：

1. 在 `src/feishu_websocket/tools.rs` 中定义工具
2. 在 `ToolManager::new()` 中注册工具
3. 添加执行方法
4. 重新编译并部署

示例：

```rust
tools.insert(
    "custom_tool".to_string(),
    Tool {
        name: "custom_tool".to_string(),
        description: "工具描述".to_string(),
        parameters: ToolParameters {
            // 参数定义
        },
    },
);
```

## 性能特点

- **快速响应**：工具执行延迟 < 1s
- **低内存占用**：服务内存使用 ~5M
- **高可用性**：自动重连机制
- **可扩展**：支持添加更多工具

## 版本信息

- NewClaw 版本：v0.7.0
- 实现时间：2026-03-16
- 状态：✅ 生产就绪

---

**注意：** 所有工具调用都会记录日志，便于审计和问题排查。