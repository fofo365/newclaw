# NewClaw 故障排查指南

## 快速诊断

### 1. 检查服务状态

```bash
# 检查进程是否运行
ps aux | grep newclaw

# 检查端口是否监听
netstat -tlnp | grep 3000
# 或
ss -tlnp | grep 3000

# 检查 systemd 状态
sudo systemctl status newclaw
```

### 2. 检查日志

```bash
# 实时查看日志
sudo journalctl -u newclaw -f

# 查看最近 100 行
sudo journalctl -u newclaw -n 100

# 查看错误日志
sudo journalctl -u newclaw -p err
```

### 3. 检查依赖

```bash
# 检查 Redis
redis-cli ping
# 应返回: PONG

# 检查 Redis 连接
redis-cli info server
```

## 常见问题

### 问题 1: 服务启动失败

**症状**
```
Error: Address already in use (os error 98)
```

**原因**: 端口 3000 被其他进程占用

**解决方案**
```bash
# 查找占用进程
lsof -i :3000

# 杀死进程
kill -9 <PID>

# 或修改配置文件端口
# newclaw.toml
[server]
port = 3001
```

---

### 问题 2: Redis 连接失败

**症状**
```
Error: Cannot connect to Redis at redis://127.0.0.1:6379
```

**原因**: Redis 服务未启动或配置错误

**解决方案**
```bash
# 检查 Redis 状态
sudo systemctl status redis-server

# 启动 Redis
sudo systemctl start redis-server

# 测试连接
redis-cli ping

# 检查配置
# newclaw.toml
[redis]
url = "redis://127.0.0.1:6379"
```

---

### 问题 3: API Key 无效

**症状**
```
Error: Invalid API key
```

**原因**: API Key 配置错误或过期

**解决方案**
```bash
# 检查配置文件
cat newclaw.toml | grep api_key

# 确保格式正确
[llm.glm]
api_key = "your-actual-api-key"  # 不要有引号错误

# 测试 API Key
curl -H "Authorization: Bearer YOUR_API_KEY" \
  https://open.bigmodel.cn/api/paas/v4/models
```

---

### 问题 4: 内存不足

**症状**
```
Error: Out of memory
```
或系统频繁重启

**原因**: 可用内存不足

**解决方案**
```bash
# 检查内存
free -h

# 检查内存占用
ps aux --sort=-%mem | head -10

# 释放缓存
sync && echo 3 > /proc/sys/vm/drop_caches

# 增加交换空间
sudo fallocate -l 2G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

---

### 问题 5: 请求超时

**症状**
```
Error: Request timeout after 30s
```

**原因**: LLM API 响应慢或网络问题

**解决方案**
```bash
# 增加超时时间
# newclaw.toml
[tools]
timeout = 60  # 秒

# 检查网络
ping open.bigmodel.cn

# 使用代理（如需要）
export HTTPS_PROXY=http://your-proxy:8080
```

---

### 问题 6: 工具执行失败

**症状**
```
Error: Tool execution failed: permission denied
```

**原因**: 权限不足

**解决方案**
```bash
# 检查文件权限
ls -la /path/to/file

# 修改权限
chmod 644 /path/to/file

# 检查用户权限
whoami
# 确保 newclaw 用户有执行权限
```

---

### 问题 7: 配置文件错误

**症状**
```
Error: Failed to parse config: missing field `api_key`
```

**原因**: 配置文件格式错误或缺少必需字段

**解决方案**
```bash
# 验证配置文件
./newclaw config check

# 使用示例配置
cp config/newclaw.example.toml newclaw.toml

# 检查 TOML 语法
# 常见错误:
# 1. 字符串未加引号
# 2. 数组格式错误
# 3. 缩进问题
```

---

### 问题 8: WebSocket 连接断开

**症状**
```
WebSocket connection closed unexpectedly
```

**原因**: 网络不稳定或超时

**解决方案**
```bash
# 检查 Nginx 配置（如使用）
# 增加 WebSocket 超时
proxy_read_timeout 3600s;

# 检查防火墙
sudo ufw status

# 检查心跳配置
# newclaw.toml
[server]
websocket_heartbeat = 30  # 秒
```

## 性能问题

### CPU 占用过高

```bash
# 查看进程 CPU 使用
top -p $(pgrep newclaw)

# 生成火焰图（需要 perf）
perf record -g -p $(pgrep newclaw)
perf script | stackcollapse-perf.pl | flamegraph.pl > flame.svg
```

### 内存泄漏

```bash
# 监控内存
watch -n 1 'ps aux | grep newclaw'

# 使用 valgrind 检测
valgrind --leak-check=full ./newclaw gateway
```

### 请求慢

```bash
# 启用 debug 日志
export RUST_LOG=debug

# 查看耗时
./newclaw gateway 2>&1 | grep "duration"
```

## 网络问题

### DNS 解析失败

```bash
# 测试 DNS
nslookup open.bigmodel.cn

# 修改 DNS
echo "nameserver 8.8.8.8" | sudo tee /etc/resolv.conf
```

### SSL 证书错误

```bash
# 更新证书
sudo update-ca-certificates

# 或禁用 SSL 验证（仅测试）
export SSL_CERT_FILE=/dev/null
```

## 日志分析

### 启用详细日志

```bash
# 方式 1: 环境变量
export RUST_LOG=trace

# 方式 2: 配置文件
[server]
log_level = "trace"
```

### 日志级别说明

- `trace`: 最详细，包含所有信息
- `debug`: 调试信息
- `info`: 一般信息（默认）
- `warn`: 警告信息
- `error`: 仅错误

### 日志搜索

```bash
# 搜索错误
grep "ERROR" /var/log/newclaw/*.log

# 搜索特定用户
grep "user_id=123" /var/log/newclaw/*.log

# 统计错误类型
grep "ERROR" /var/log/newclaw/*.log | awk '{print $5}' | sort | uniq -c
```

## 重置和恢复

### 完全重置

```bash
# 停止服务
sudo systemctl stop newclaw

# 清除 Redis 数据
redis-cli FLUSHDB

# 清除日志
sudo rm -rf /var/log/newclaw/*

# 重新启动
sudo systemctl start newclaw
```

### 回滚版本

```bash
# 下载旧版本
wget https://github.com/fofo365/newclaw/releases/download/v0.4.0/newclaw

# 替换
sudo cp newclaw /opt/newclaw/newclaw
sudo systemctl restart newclaw
```

## 获取帮助

### 收集诊断信息

```bash
#!/bin/bash
# diagnose.sh

echo "=== System Info ==="
uname -a
cat /etc/os-release

echo "=== NewClaw Version ==="
./newclaw --version

echo "=== Service Status ==="
sudo systemctl status newclaw

echo "=== Recent Logs ==="
sudo journalctl -u newclaw -n 50

echo "=== Redis Status ==="
redis-cli info

echo "=== Network ==="
netstat -tlnp | grep 3000

echo "=== Memory ==="
free -h
```

### 联系支持

1. GitHub Issues: https://github.com/fofo365/newclaw/issues
2. 提供信息：
   - 错误信息
   - 配置文件（隐藏敏感信息）
   - 系统信息
   - 日志片段
