# Issue #001: 内存耗尽导致系统崩溃

**严重级别**: P0 (Critical)  
**状态**: Open  
**发现时间**: 2026-03-11 12:50  
**影响范围**: 服务器稳定性  
**负责人**: AI Agent (GLM-5)

---

## 📋 问题描述

服务器在 **2026-03-11 12:25** 和 **12:45** 两次崩溃，均因内存耗尽触发系统重启。

### 崩溃时间线

| 时间 | 事件 | 内存使用率 |
|------|------|-----------|
| 11:56-12:02 | cargo build 执行（推测） | 飙升至 94.5% |
| 12:02:59 | 内存压力临界 | 94.52% (kbcommit=104.87%) |
| 12:25:25 | 第一次系统重启 | - |
| 12:41:29 | 内存再次耗尽 | 94.72% (kbcommit=101.14%) |
| 12:45:50 | 第二次系统重启 | - |

### 关键日志证据

```bash
# 内存压力警告
12:42:21 systemd-journald: Under memory pressure, flushing caches.
12:42:54 systemd-journald: Under memory pressure, flushing caches.

# sar 内存历史
12:02:59 PM  kbmemfree=106084   kbmemused=3603704   %memused=94.52%
12:41:29 PM  kbmemfree=106372   kbmemused=3611544   %memused=94.72%
```

---

## 🔍 根本原因分析

### 直接原因
1. **Rust 编译占用大量内存**：`cargo build --release` 在 3.6GB 内存的服务器上可能消耗 2-3GB 内存
2. **编译产物未清理**：`/root/newclaw/target/` 目录占用 3.6GB 磁盘空间，编译过程中内存占用更高
3. **缺少资源限制**：没有对编译进程设置内存限制

### 系统配置问题
- **panic_on_oom=0**：系统不会因 OOM panic，但内存压力可能导致其他崩溃机制
- **无 swap 使用限制**：虽然配置了 9.9GB swap，但内存压力过大时仍会触发系统保护机制

### 间接原因
1. **newclaw-gateway.service 疯狂重启**（已在 #002 修复）：
   - 11:56-12:25 期间重启 313 次
   - 每次重启消耗系统资源
   - 加剧内存压力

2. **服务器内存不足**：
   - 总内存：3.6GB
   - OpenClaw Gateway 正常运行需要：~600MB
   - 剩余可用：~3GB
   - cargo build 可能需要：2-4GB

---

## 🎯 解决方案

### 立即执行（P0）

#### 1. 清理编译产物 ✅
```bash
cd /root/newclaw
cargo clean  # 清理 target/ 目录，释放 3.6GB 空间
```

#### 2. 禁止在服务器上执行 cargo build ✅
- **原因**：服务器内存不足（3.6GB），无法承受 cargo build 的内存消耗
- **替代方案**：
  - 在本地开发机器编译
  - 使用 CI/CD 构建并上传二进制文件
  - 使用容器化构建（限制内存）

#### 3. 设置进程内存限制 ✅
```bash
# 为 OpenClaw Gateway 设置内存限制
systemctl edit openclaw-gateway
```
添加：
```ini
[Service]
MemoryMax=1G
MemoryHigh=800M
```

### 中期优化（P1）

#### 4. 增加 swap 使用策略
```bash
# 调整 swappiness（当前可能过低）
sysctl vm.swappiness=60
echo "vm.swappiness=60" >> /etc/sysctl.conf
```

#### 5. 配置 OOM Killer 优先级
```bash
# 降低 OpenClaw Gateway 的 OOM 评分（优先级降低，更不容易被 kill）
echo -500 > /proc/$(pidof openclaw-gateway)/oom_score_adj
```

#### 6. 添加内存监控
```bash
# 安装并配置内存监控
apt install -y earlyoom
systemctl enable earlyoom
```

### 长期方案（P2）

#### 7. 升级服务器内存
- **当前**：3.6GB
- **建议**：8GB+（支持并发编译和运行）

#### 8. 容器化隔离
- 将 NewClaw 放入 Docker 容器
- 设置严格的内存限制（如 `memory: 512M`）
- 防止影响宿主机

---

## 📊 验证标准

### 修复完成后需验证
- [ ] `/root/newclaw/target/` 目录已清理
- [ ] 内存使用率 < 50%（正常运行）
- [ ] 系统稳定运行 > 24 小时无崩溃
- [ ] OpenClaw Gateway 正常响应
- [ ] 无内存压力日志

---

## 🔄 关联 Issue

- **#002**: newclaw-gateway.service 疯狂重启（已修复）
- **#003**: 服务器资源规划（待创建）

---

## 📝 行动记录

### 2026-03-11 12:50 (GLM-5)
1. ✅ 分析崩溃日志
2. ✅ 定位根本原因：内存耗尽
3. ✅ 创建 Issue 文档
4. ⏳ 执行 cargo clean（待执行）
5. ⏳ 配置资源限制（待执行）

---

## 🚨 风险评估

### 高风险
- **重复崩溃**：如果不清理 target/ 目录，再次编译会导致崩溃
- **服务中断**：系统重启期间 OpenClaw 无法服务

### 中风险
- **OOM Killer**：可能误杀重要进程
- **数据丢失**：未保存的数据在崩溃时可能丢失

---

## 📚 参考资料

- [Rust 编译内存优化](https://github.com/rust-lang/rust/issues/71257)
- [Linux OOM Killer 机制](https://www.kernel.org/doc/Documentation/filesystems/proc.txt)
- [systemd 资源限制](https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html)

---

**状态**: 📝 Issue 已创建，待执行修复  
**最后更新**: 2026-03-11 12:50 UTC+8
