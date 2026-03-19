# CHANGELOG v0.7.1

## [0.7.1] - 规划中

### 🚨 严重问题：运行时硬编码依赖 OpenClaw 目录

**Issue #003** - P0 架构缺陷（2026-03-18 发现）

**问题位置**：
- `src/skill/loader.rs:56-57` - 硬编码 `/root/.openclaw/workspace/skills`
- `src/feishu_websocket/tools.rs:97` - 硬编码 `/root/.openclaw/workspace-dev`

**严重性**：
- NewClaw 运行时依赖 OpenClaw 目录
- 两个系统耦合，违背独立性原则
- 修改 OpenClaw 会影响 NewClaw

**状态**：🔴 待修复（另一智能体处理中）

**教训**：
- 所有路径必须从配置文件读取
- 代码评审必须检查硬编码路径
- 新增 `CODE_REVIEW_CHECKLIST.md` 防止复发

---

### 核心问题（来自飞书通道事件）

飞书通道AI在遇到"持续发送开发进度报告"问题时，采取了极端措施（删除所有记忆），而不是正确诊断和解决问题。

**事件总结**：
- 问题：飞书通道每20分钟收到"v0.7.0进度汇报"的持续任务消息
- 飞书AI的处理：删除所有记忆（极端操作）
- 正确处理：`openclaw cron list` 查询任务 → `openclaw cron delete <ID>` 删除任务

### 需求

#### 1. 🔴 系统诊断工具集成（P0）
**目标**：所有通道AI都应该能够诊断和解决系统问题

**功能需求**：
- [ ] **Cron任务管理**
  - 查询持续任务列表
  - 删除指定任务
  - 暂停/恢复任务
  - 修改任务配置

- [ ] **系统状态查询**
  - 查询活跃会话
  - 查询服务状态
  - 查询错误日志
  - 查询资源使用情况

- [ ] **记忆管理**
  - 查询记忆内容
  - 归档旧记忆（而非删除）
  - 搜索历史记录
  - 恢复归档的记忆

#### 2. 🔴 系统诊断工作流（P0）
**目标**：建立标准化的诊断流程，避免AI走极端

**诊断流程**：
1. **问题识别**
   - 用户报告问题
   - 系统自动检测异常
   - AI主动发现问题

2. **初步诊断**
   - 运行系统状态检查
   - 查询持续任务
   - 检查配置文件
   - 查看错误日志

3. **问题定位**
   - 分析诊断结果
   - 确定问题根源
   - 评估影响范围

4. **解决方案**
   - 提供多个解决选项
   - 风险评估
   - 执行最小改动

5. **验证和记录**
   - 验证问题已解决
   - 记录处理过程
   - 更新记忆系统

**工作流示例**：
```
用户: "飞书一直在发进度报告，停不下来"
  ↓
AI: 运行诊断工作流
  ↓
1. 查询持续任务: `openclaw cron list`
2. 发现问题任务: "v0.7.0进度汇报" (每20分钟)
3. 确认操作: "发现持续任务 'v0.7.0进度汇报'，是否删除？"
4. 执行操作: `openclaw cron delete <ID>`
5. 验证: 确认任务已删除，消息不再发送
6. 记录: 将处理过程写入记忆
```

#### 3. 🔴 AI行为约束（P0）
**目标**：防止AI采取极端措施

**约束规则**：
- [ ] **禁止删除所有记忆**
  - 只有在用户明确要求时才能执行
  - 必须提供备份/归档选项
  - 执行前必须二次确认

- [ ] **危险操作保护**
  - 删除操作必须询问用户
  - 批量操作必须有数量限制
  - 不可逆操作必须有警告

- [ ] **先诊断，再行动**
  - 遇到问题时先运行诊断
  - 提供多个解决方案
  - 选择最小改动方案

#### 4. 🟡 通道工具权限优化（P1）
**目标**：合理分配不同通道的工具权限

**权限分级**：
- [ ] **基础权限**（所有通道）
  - 消息收发
  - 基本对话
  - 记忆查询

- [ ] **管理权限**（管理员通道）
  - Cron任务管理
  - 系统状态查询
  - 记忆归档/恢复

- [ ] **高级权限**（核心通道）
  - 配置修改
  - 服务重启
  - 文件系统操作

**配置示例**：
```toml
[channels.feishu]
permissions = ["basic", "management"]

[channels.qqbot]
permissions = ["basic", "management", "advanced"]
```

#### 5. 🟡 记忆恢复机制（P1）
**目标**：记忆被误删后能够恢复

**功能**：
- [ ] 记忆删除前自动归档到 `memory.archive/`
- [ ] 提供 `memory restore <date>` 命令
- [ ] 归档保留30天（可配置）
- [ ] 重要记忆标记，防止误删

#### 6. 🟢 诊断报告生成（P2）
**目标**：生成详细的诊断报告，便于问题追踪

**报告内容**：
- 系统状态快照
- 问题描述
- 诊断步骤
- 解决方案
- 执行结果
- 后续建议

**输出格式**：
- 控制台输出（简洁版）
- 文件输出（详细版）
- 发送到指定通道

---

## 技术实现

### Cron工具集成

```rust
// 新增工具: CronManager
pub struct CronManager {
    cron_client: CronClient,
}

impl CronManager {
    pub async fn list_jobs(&self) -> Result<Vec<CronJob>>;
    pub async fn delete_job(&self, id: &str) -> Result<()>;
    pub async fn pause_job(&self, id: &str) -> Result<()>;
    pub async fn resume_job(&self, id: &str) -> Result<()>;
}
```

### 诊断工作流

```rust
// 新增模块: diagnostics
pub mod diagnostics {
    pub struct DiagnosticWorkflow {
        steps: Vec<DiagnosticStep>,
    }

    pub async fn run_diagnosis(problem: &Problem) -> Result<DiagnosticReport>;
}

pub enum DiagnosticStep {
    CheckSystemStatus,
    ListCronJobs,
    CheckConfig,
    CheckLogs,
}
```

### 行为约束

```rust
// 新增模块: constraints
pub mod constraints {
    pub struct BehaviorConstraints {
        rules: Vec<Constraint>,
    }

    pub enum Constraint {
        NoFullMemoryDelete,
        RequireConfirmationForDangerousActions,
        DiagnoseBeforeAction,
    }
}
```

---

## 测试计划

### 单元测试
- [ ] CronManager 工具测试
- [ ] 诊断工作流测试
- [ ] 行为约束测试

### 集成测试
- [ ] 飞书通道诊断流程测试
- [ ] 记忆恢复机制测试
- [ ] 权限控制测试

### 场景测试
- [ ] 重复消息问题处理
- [ ] 系统异常诊断
- [ ] 记忆误删恢复

---

## 发布检查清单

- [ ] 所有P0功能完成
- [ ] 单元测试通过率 > 90%
- [ ] 集成测试全部通过
- [ ] 文档更新完成
- [ ] CHANGELOG更新完成
- [ ] 版本号更新
- [ ] 发布说明准备

---

**预计发布时间**: 2026-03-16
**预计工作量**: 1-2天
**优先级**: P0（基于飞书通道事件）