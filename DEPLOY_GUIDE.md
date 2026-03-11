# NewClaw 部署指南

## 📦 使用 CI 构建产物

### 方法一：下载脚本（推荐）

```bash
# 下载指定版本
cd /root/newclaw
./scripts/download-newclaw.sh v0.4.1

# 或下载最新版本
./scripts/download-newclaw.sh latest
```

### 方法二：手动下载

1. 访问 https://github.com/fofo365/newclaw/releases
2. 下载对应版本的 `newclaw-linux-x64.tar.gz`
3. 解压并安装：
   ```bash
   tar -xzf newclaw-linux-x64.tar.gz
   chmod +x newclaw
   mv newclaw /root/newclaw/
   ```

---

## 🚀 配置 systemd 服务

### 创建服务文件

```bash
sudo tee /etc/systemd/system/newclaw.service > /dev/null <<'EOF'
[Unit]
Description=NewClaw Gateway Service
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/newclaw
Environment="RUST_LOG=info"
Environment="NEWCLAW_CONFIG=/etc/newclaw/config.toml"
ExecStart=/root/newclaw/newclaw gateway

# 仅在异常退出时重启
Restart=on-failure
RestartSec=30

# 资源限制（防止内存耗尽）
MemoryMax=512M
CPUQuota=50%

# 限制重启次数
StartLimitIntervalSec=300
StartLimitBurst=5

[Install]
WantedBy=multi-user.target
EOF
```

### 启用服务

```bash
sudo systemctl daemon-reload
sudo systemctl enable newclaw.service
sudo systemctl start newclaw.service

# 检查状态
sudo systemctl status newclaw.service
```

---

## 🔄 更新流程

### 使用脚本更新

```bash
# 1. 下载新版本
cd /root/newclaw
./scripts/download-newclaw.sh v0.5.0

# 2. 重启服务
sudo systemctl restart newclaw.service

# 3. 验证
sudo systemctl status newclaw.service
/root/newclaw/newclaw --version
```

### 回滚到旧版本

```bash
# 查看备份
ls -la /root/newclaw/newclaw.backup.*

# 恢复
cp /root/newclaw/newclaw.backup.YYYYMMDD_HHMMSS /root/newclaw/newclaw
sudo systemctl restart newclaw.service
```

---

## 🛠️ 故障排查

### 服务无法启动

```bash
# 查看日志
sudo journalctl -u newclaw.service -n 50

# 检查配置文件
cat /etc/newclaw/config.toml

# 手动测试
/root/newclaw/newclaw gateway
```

### 内存不足

```bash
# 检查内存使用
free -h
ps aux --sort=-%mem | head -10

# 调整服务限制
sudo systemctl edit newclaw.service
# 添加:
# [Service]
# MemoryMax=256M
```

---

## 📊 监控

### 日志查看

```bash
# 实时日志
sudo journalctl -u newclaw.service -f

# 最近日志
sudo journalctl -u newclaw.service --since "1 hour ago"
```

### 性能监控

```bash
# CPU 和内存
top -p $(pgrep newclaw)

# 网络连接
ss -tlnp | grep newclaw
```

---

## 🚨 安全建议

1. **不要在服务器上编译**：使用 CI 构建产物
2. **定期更新**：及时修复安全漏洞
3. **限制资源**：配置 MemoryMax 和 CPUQuota
4. **备份数据**：定期备份配置和数据目录

---

## 📝 相关文档

- [Issue #001: 内存耗尽崩溃](../issues/001-memory-exhaustion-crash.md)
- [Issue #002: systemd 疯狂重启](../issues/002-systemd-restart-loop.md)
- [GitHub Actions 配置](../.github/workflows/build.yml)
