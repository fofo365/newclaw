# NewClaw v0.3.0 部署文档

**版本**: v0.3.0  
**更新时间**: 2026-03-09

---

## 📋 系统要求

### 最低配置
- **OS**: Linux (Ubuntu 20.04+, CentOS 8+, OpenCloudOS)
- **CPU**: 2 核
- **内存**: 2 GB
- **磁盘**: 500 MB

### 推荐配置
- **OS**: Linux (Ubuntu 22.04+)
- **CPU**: 4 核
- **内存**: 4 GB
- **磁盘**: 1 GB

---

## 🔧 安装步骤

### 1. 安装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. 克隆仓库

```bash
git clone https://github.com/fofo365/newclaw.git
cd newclaw
```

### 3. 编译项目

```bash
# 开发版本
cargo build

# 生产版本（推荐）
cargo build --release
```

### 4. 验证安装

```bash
./target/release/newclaw --version
```

---

## ⚙️ 配置文件

### 创建配置文件

```bash
mkdir -p ~/.config/newclaw
cat > ~/.config/newclaw/config.yaml << 'EOF'
agent:
  name: "newclaw-agent"
  model: "gpt-4o-mini"

llm:
  provider: "openai"
  model: "gpt-4o-mini"
  api_key: "${OPENAI_API_KEY}"
  base_url: "https://api.openai.com/v1"
  strategy:
    type: "static"
    model: "gpt-4o-mini"

tools:
  enabled: true
  retry:
    max_attempts: 3
    delay_ms: 100

communication:
  websocket:
    enabled: true
    host: "0.0.0.0"
    port: 8080
  http:
    enabled: true
    host: "0.0.0.0"
    port: 3000

logging:
  level: "info"
  format: "json"
EOF
```

---

## 🔑 环境变量

### OpenAI 配置
```bash
export OPENAI_API_KEY="sk-..."
```

### Claude 配置
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### GLM 配置
```bash
export GLM_API_KEY="..."
```

### Feishu 配置
```bash
export FEISHU_APP_ID="..."
export FEISHU_APP_SECRET="..."
```

---

## 🚀 启动服务

### 开发模式

```bash
cargo run
```

### 生产模式

```bash
./target/release/newclaw --config ~/.config/newclaw/config.yaml
```

### 后台运行

```bash
nohup ./target/release/newclaw --config ~/.config/newclaw/config.yaml > logs/newclaw.log 2>&1 &
```

### 使用 systemd（推荐）

创建服务文件 `/etc/systemd/system/newclaw.service`:

```ini
[Unit]
Description=NewClaw AI Agent
After=network.target

[Service]
Type=simple
User=newclaw
WorkingDirectory=/opt/newclaw
ExecStart=/opt/newclaw/target/release/newclaw --config /etc/newclaw/config.yaml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable newclaw
sudo systemctl start newclaw
sudo systemctl status newclaw
```

---

## 🔍 健康检查

### HTTP 端点

```bash
curl http://localhost:3000/health
```

预期响应：

```json
{
  "status": "ok",
  "version": "0.3.0",
  "uptime": 3600
}
```

### WebSocket 端点

```bash
wscat -c ws://localhost:8080
```

---

## 📊 监控与日志

### 日志位置

- **标准输出**: stdout/stderr
- **日志文件**: `/var/log/newclaw/`（如果配置）

### 日志级别

- `debug`: 详细调试信息
- `info`: 一般信息（默认）
- `warn`: 警告信息
- `error`: 错误信息

---

## 🛠️ 故障排查

### 问题 1：编译失败

**症状**: `cargo build` 失败

**解决方案**:
```bash
# 更新 Rust
rustup update

# 清理缓存
cargo clean

# 重新编译
cargo build
```

### 问题 2：LLM API 调用失败

**症状**: API 返回 401/403 错误

**解决方案**:
1. 检查 API Key 是否正确
2. 检查网络连接
3. 查看日志文件

### 问题 3：WebSocket 连接失败

**症状**: 无法连接到 WebSocket

**解决方案**:
```bash
# 检查端口是否被占用
netstat -tuln | grep 8080

# 检查防火墙
sudo firewall-cmd --list-all

# 开放端口
sudo firewall-cmd --add-port=8080/tcp --permanent
sudo firewall-cmd --reload
```

### 问题 4：内存占用过高

**症状**: 进程内存占用 > 1GB

**解决方案**:
1. 减少上下文大小
2. 启用上下文清理
3. 增加物理内存

---

## 🔄 更新与升级

### 更新到最新版本

```bash
cd /path/to/newclaw
git pull origin main
cargo build --release
sudo systemctl restart newclaw
```

### 查看版本

```bash
./target/release/newclaw --version
```

---

## 🔐 安全建议

### 1. 使用专用用户运行

```bash
sudo useradd -r -s /bin/false newclaw
sudo chown -R newclaw:newclaw /opt/newclaw
```

### 2. 限制文件权限

```bash
chmod 600 ~/.config/newclaw/config.yaml
chmod 600 /etc/newclaw/config.yaml
```

### 3. 使用防火墙

```bash
# 只允许本地访问
sudo iptables -A INPUT -p tcp --dport 3000 -s 127.0.0.1 -j ACCEPT
sudo iptables -A INPUT -p tcp --dport 3000 -j DROP
```

### 4. 启用 TLS（生产环境）

使用 Nginx 反向代理：

```nginx
server {
    listen 443 ssl;
    server_name your-domain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

---

## 📚 更多资源

- **GitHub**: https://github.com/fofo365/newclaw
- **文档**: https://docs.newclaw.ai
- **社区**: https://discord.com/xxx

---

**最后更新**: 2026-03-09  
**维护者**: NewClaw Team
