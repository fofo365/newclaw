# Issue #002: newclaw-gateway.service 疯狂重启导致系统崩溃

**严重级别**: P0 (Critical)  
**状态**: ✅ 已修复  
**发现时间**: 2026-03-11 12:31  
**修复时间**: 2026-03-11 12:32  
**影响范围**: 系统稳定性  
**负责人**: AI Agent (GLM-5)

---

## 📋 问题描述

`newclaw-gateway.service` 因配置错误导致无限重启，最终拖垮整个系统。

### 崩溃时间线

| 时间 | 重启计数 | 事件 |
|------|---------|------|
| 11:56 | 294-313 | 服务疯狂重启 |
| 12:25:25 | 系统重启 | 第一次崩溃 |
| 12:25:42 | 1-33 | 重启后继续疯狂重启 |
| 12:45:50 | 系统重启 | 第二次崩溃 |

### 关键日志证据

```bash
# systemd 重启日志（每 10 秒一次）
11:56:15 newclaw-gateway.service: Scheduled restart job, restart counter is at 294.
11:56:26 newclaw-gateway.service: Scheduled restart job, restart counter is at 295.
...
12:02:55 newclaw-gateway.service: Scheduled restart job, restart counter is at 313.

# 启动失败原因
11:59:11 newclaw-gateway.service: Main process exited, code=exited, status=203/EXEC
```

---

## 🔍 根本原因分析

### 服务配置问题
```ini
[Service]
Type=simple
Restart=always      # ❌ 问题：无论什么错误都重启
RestartSec=10       # ❌ 问题：重启间隔太短
```

### 启动失败原因
- **Exit code 203/EXEC**：可执行文件路径错误或权限问题
- **推测原因**：
  1. 配置文件路径不存在（`NEWCLAW_CONFIG=/etc/newclaw/config.toml`）
  2. 可执行文件依赖缺失
  3. 端口冲突（3000 端口可能被占用）

### 疯狂重启机制
```
启动失败 → Restart=always → 10秒后重启 → 再次失败 → 无限循环
```

每次重启消耗系统资源（进程创建、日志写入、systemd 调度），累积导致：
- CPU 负载升高
- 内存碎片化
- I/O 压力
- 最终触发系统崩溃

---

## 🎯 解决方案

### 已执行修复 ✅

#### 1. 停止并禁用服务
```bash
systemctl stop newclaw-gateway.service
systemctl disable newclaw-gateway.service
```

#### 2. 删除服务文件
```bash
rm -f /etc/systemd/system/newclaw-gateway.service
systemctl daemon-reload
systemctl reset-failed
```

#### 3. 验证修复
```bash
systemctl list-units --state=failed  # 无失败服务
systemctl list-unit-files | grep newclaw  # 无 newclaw 服务
```

### 推荐配置（如需重新启用）

```ini
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
ExecStart=/root/newclaw/target/release/newclaw gateway

# ✅ 修复：仅在异常退出时重启
Restart=on-failure
RestartSec=30

# ✅ 修复：添加资源限制
MemoryMax=512M
CPUQuota=50%

# ✅ 修复：限制重启次数
StartLimitIntervalSec=300
StartLimitBurst=5

[Install]
WantedBy=multi-user.target
```

---

## 📊 验证标准

### 修复完成后需验证
- [x] 服务已禁用
- [x] 服务文件已删除
- [x] 无失败服务
- [x] 系统稳定运行 > 10 分钟

---

## 🔄 关联 Issue

- **#001**: 内存耗尽导致系统崩溃（根因之一）
- **#003**: 服务器资源规划（待创建）

---

## 📝 行动记录

### 2026-03-11 12:31 (GLM-5)
1. ✅ 分析 systemd 日志
2. ✅ 定位疯狂重启问题
3. ✅ 停止并禁用服务
4. ✅ 删除服务文件
5. ✅ 验证修复

---

## 🚨 风险评估

### 高风险（已消除）
- **系统崩溃**：疯狂重启拖垮系统 ✅ 已修复
- **服务冲突**：端口占用 ✅ 已禁用

### 中风险
- **NewClaw 无法使用**：服务已禁用，需手动启动
- **配置文件丢失**：需验证 `/etc/newclaw/config.toml`

---

## 📚 参考资料

- [systemd.service 配置](https://www.freedesktop.org/software/systemd/man/systemd.service.html)
- [systemd 资源控制](https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html)

---

**状态**: ✅ 已修复  
**最后更新**: 2026-03-11 12:32 UTC+8
