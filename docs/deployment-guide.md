# NewClaw 生产环境部署指南

## 系统要求

### 最低配置
- CPU: 2 核
- 内存: 1GB
- 存储: 500MB
- OS: Linux (Ubuntu 20.04+, CentOS 7+)

### 推荐配置
- CPU: 4 核
- 内存: 2GB
- 存储: 1GB
- OS: Ubuntu 22.04 LTS

### 软件依赖
- Rust 1.75+ (编译)
- Redis 7.0+ (必需)
- Nginx (可选，反向代理)

## 快速部署

### 方式一：使用预编译二进制

```bash
# 下载最新版本
wget https://github.com/fofo365/newclaw/releases/download/v0.5.0/newclaw-linux-x86_64.tar.gz

# 解压
tar -xzf newclaw-linux-x86_64.tar.gz
cd newclaw

# 复制配置文件
cp config/newclaw.example.toml newclaw.toml

# 编辑配置（修改 API keys 等）
vim newclaw.toml

# 启动服务
./newclaw gateway
```

### 方式二：从源码编译

```bash
# 克隆仓库
git clone https://github.com/fofo365/newclaw.git
cd newclaw

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 编译
cargo build --release

# 二进制文件位于 target/release/newclaw
```

### 方式三：Docker 部署

```bash
# 构建镜像
docker build -t newclaw:0.5.0 .

# 运行容器
docker run -d \
  --name newclaw \
  -p 3000:3000 \
  -v $(pwd)/newclaw.toml:/app/newclaw.toml \
  --link redis:redis \
  newclaw:0.5.0
```

## 系统服务配置

### 创建 systemd 服务

```bash
# 创建服务文件
sudo cat > /etc/systemd/system/newclaw.service << 'EOF'
[Unit]
Description=NewClaw AI Agent Gateway
After=network.target redis.service
Requires=redis.service

[Service]
Type=simple
User=newclaw
Group=newclaw
WorkingDirectory=/opt/newclaw
ExecStart=/opt/newclaw/newclaw gateway
Restart=on-failure
RestartSec=10

# 安全配置
NoNewPrivileges=true
PrivateTmp=true

# 环境变量
Environment="RUST_LOG=info"
Environment="NEWCLAW_CONFIG=/opt/newclaw/newclaw.toml"

[Install]
WantedBy=multi-user.target
EOF

# 创建用户
sudo useradd -r -s /bin/false newclaw

# 创建目录
sudo mkdir -p /opt/newclaw
sudo chown newclaw:newclaw /opt/newclaw

# 复制文件
sudo cp target/release/newclaw /opt/newclaw/
sudo cp config/newclaw.example.toml /opt/newclaw/newclaw.toml
sudo chown -R newclaw:newclaw /opt/newclaw

# 启用服务
sudo systemctl daemon-reload
sudo systemctl enable newclaw
sudo systemctl start newclaw

# 查看状态
sudo systemctl status newclaw
```

## Nginx 反向代理

```nginx
upstream newclaw_backend {
    server 127.0.0.1:3000;
    keepalive 32;
}

server {
    listen 80;
    server_name your-domain.com;

    # 重定向到 HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://newclaw_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        
        # 超时配置
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # 健康检查
    location /health {
        proxy_pass http://newclaw_backend/health;
        access_log off;
    }
}
```

## Redis 配置

```bash
# 安装 Redis
sudo apt install redis-server

# 配置 Redis
sudo vim /etc/redis/redis.conf

# 推荐配置：
# bind 127.0.0.1
# protected-mode yes
# maxmemory 256mb
# maxmemory-policy allkeys-lru

# 启动 Redis
sudo systemctl enable redis-server
sudo systemctl start redis-server
```

## 配置验证

```bash
# 检查配置文件
./newclaw config check

# 测试连接
./newclaw config test-redis
./newclaw config test-llm
```

## 健康检查

```bash
# 检查服务状态
curl http://localhost:3000/health

# 预期响应
# {"status":"ok","version":"0.5.0","uptime":123}
```

## 日志管理

### 日志级别

```bash
# 设置日志级别
export RUST_LOG=debug  # trace, debug, info, warn, error
```

### 日志轮转

```bash
# /etc/logrotate.d/newclaw
/var/log/newclaw/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 newclaw newclaw
}
```

## 性能优化

### 系统参数

```bash
# /etc/sysctl.d/99-newclaw.conf
# 增加文件描述符限制
fs.file-max = 65535

# TCP 优化
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 30

# 应用
sudo sysctl -p /etc/sysctl.d/99-newclaw.conf
```

### 资源限制

```bash
# /etc/security/limits.d/newclaw.conf
newclaw soft nofile 65535
newclaw hard nofile 65535
```

## 监控和告警

### Prometheus 指标

访问 `http://localhost:3000/metrics` 获取 Prometheus 格式指标：

- `newclaw_requests_total` - 总请求数
- `newclaw_request_duration_seconds` - 请求延迟
- `newclaw_active_connections` - 活跃连接数
- `newclaw_tool_executions_total` - 工具执行次数

### 告警规则示例

```yaml
groups:
  - name: newclaw
    rules:
      - alert: NewClawDown
        expr: up{job="newclaw"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "NewClaw 服务不可用"
```

## 故障排查

### 常见问题

1. **端口被占用**
   ```bash
   lsof -i :3000
   kill -9 <PID>
   ```

2. **Redis 连接失败**
   ```bash
   redis-cli ping
   # 应返回 PONG
   ```

3. **内存不足**
   ```bash
   free -h
   # 检查可用内存
   ```

### 查看日志

```bash
# systemd 日志
sudo journalctl -u newclaw -f

# 文件日志
tail -f /var/log/newclaw/gateway.log
```

## 升级指南

```bash
# 备份配置
cp /opt/newclaw/newclaw.toml /opt/newclaw/newclaw.toml.bak

# 停止服务
sudo systemctl stop newclaw

# 替换二进制
sudo cp target/release/newclaw /opt/newclaw/

# 启动服务
sudo systemctl start newclaw

# 验证
sudo systemctl status newclaw
curl http://localhost:3000/health
```

## 安全建议

1. **API Key 保护**
   - 使用环境变量存储敏感信息
   - 定期轮换 API Key
   - 限制 API Key 权限

2. **网络安全**
   - 启用 HTTPS
   - 配置防火墙规则
   - 限制 Redis 仅本地访问

3. **访问控制**
   - 启用 JWT 认证
   - 配置速率限制
   - 记录审计日志

## 备份策略

```bash
# 备份配置和 Redis 数据
tar -czf newclaw-backup-$(date +%Y%m%d).tar.gz \
  /opt/newclaw/newclaw.toml \
  /var/lib/redis/dump.rdb
```
