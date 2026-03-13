//! Task System - 任务流原生架构
//!
//! Task 是核心抽象，Session 是 Task 的视图
//! 支持序列化、断点续传、跨设备迁移

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use uuid::Uuid;

use crate::memory::{Constraint, ConstraintScope};
use crate::context::TokenBudget;

/// 任务 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(String);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// 单步操作
    Atomic,
    /// 组合任务（包含子任务）
    Composite,
    /// 条件分支任务
    Conditional,
}

impl Default for TaskType {
    fn default() -> Self {
        Self::Atomic
    }
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// 等待执行
    Pending,
    /// 执行中
    Running,
    /// 已暂停
    Suspended,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
}

impl Default for TaskState {
    fn default() -> Self {
        Self::Pending
    }
}

/// 任务作用域
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskScope {
    /// 全局任务
    Global,
    /// 会话任务
    Session,
    /// 项目任务
    Project,
    /// 用户任务
    User,
}

impl Default for TaskScope {
    fn default() -> Self {
        Self::Session
    }
}

/// 内存访问权限
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPermission {
    /// 只读
    ReadOnly,
    /// 读写
    ReadWrite,
    /// 完全访问
    Full,
}

impl Default for MemoryPermission {
    fn default() -> Self {
        Self::ReadOnly
    }
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    /// 低优先级
    Low,
    /// 普通优先级
    Normal,
    /// 高优先级
    High,
    /// 紧急
    Urgent,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// 任务检查点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCheckpoint {
    /// 检查点 ID
    pub id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 已完成的步骤
    pub completed_steps: Vec<String>,
    /// 当前状态
    pub state: TaskState,
    /// 上下文快照（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_snapshot: Option<String>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl TaskCheckpoint {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            completed_steps: Vec::new(),
            state: TaskState::Running,
            context_snapshot: None,
            metadata: HashMap::new(),
        }
    }
}

impl Default for TaskCheckpoint {
    fn default() -> Self {
        Self::new()
    }
}

/// 任务摘要（用于历史记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    /// 任务 ID
    pub id: TaskId,
    /// 任务名称
    pub name: String,
    /// 任务类型
    pub task_type: TaskType,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// 最终状态
    pub final_state: TaskState,
    /// 结果摘要
    pub result_summary: Option<String>,
    /// Token 消耗
    pub tokens_used: usize,
}

/// 任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// 任务 ID
    pub id: TaskId,
    /// 任务名称
    pub name: String,
    /// 任务描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 任务类型
    #[serde(rename = "type")]
    pub task_type: TaskType,
    /// 任务作用域
    pub scope: TaskScope,
    /// 任务状态
    pub state: TaskState,
    /// 任务优先级
    pub priority: TaskPriority,
    /// 约束列表
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    /// Token 预算
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<TokenBudget>,
    /// 内存访问权限
    pub memory_access: MemoryPermission,
    /// 父任务 ID（用于组合任务）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<TaskId>,
    /// 子任务 ID 列表
    #[serde(default)]
    pub subtasks: Vec<TaskId>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 开始时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// 结束时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Task {
    /// 创建新的原子任务
    pub fn atomic(name: String) -> Self {
        Self {
            id: TaskId::new(),
            name,
            description: None,
            task_type: TaskType::Atomic,
            scope: TaskScope::Session,
            state: TaskState::Pending,
            priority: TaskPriority::Normal,
            constraints: Vec::new(),
            token_budget: None,
            memory_access: MemoryPermission::ReadOnly,
            parent_id: None,
            subtasks: Vec::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            started_at: None,
            ended_at: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            error: None,
        }
    }
    
    /// 创建组合任务
    pub fn composite(name: String) -> Self {
        Self {
            task_type: TaskType::Composite,
            ..Self::atomic(name)
        }
    }
    
    /// 创建条件任务
    pub fn conditional(name: String) -> Self {
        Self {
            task_type: TaskType::Conditional,
            ..Self::atomic(name)
        }
    }
    
    /// 设置描述
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self.updated_at = Utc::now();
        self
    }
    
    /// 设置作用域
    pub fn with_scope(mut self, scope: TaskScope) -> Self {
        self.scope = scope;
        self.updated_at = Utc::now();
        self
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self.updated_at = Utc::now();
        self
    }
    
    /// 添加约束
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
        self.updated_at = Utc::now();
    }
    
    /// 设置 Token 预算
    pub fn with_token_budget(mut self, budget: TokenBudget) -> Self {
        self.token_budget = Some(budget);
        self.updated_at = Utc::now();
        self
    }
    
    /// 添加子任务
    pub fn add_subtask(&mut self, subtask_id: TaskId) {
        if !self.subtasks.contains(&subtask_id) {
            self.subtasks.push(subtask_id);
            self.updated_at = Utc::now();
        }
    }
    
    /// 开始任务
    pub fn start(&mut self) -> Result<()> {
        if self.state != TaskState::Pending {
            anyhow::bail!("Cannot start task in state {:?}", self.state);
        }
        
        self.state = TaskState::Running;
        self.started_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// 完成任务
    pub fn complete(&mut self) -> Result<()> {
        if self.state != TaskState::Running {
            anyhow::bail!("Cannot complete task in state {:?}", self.state);
        }
        
        self.state = TaskState::Completed;
        self.ended_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// 暂停任务
    pub fn suspend(&mut self) -> Result<()> {
        if self.state != TaskState::Running {
            anyhow::bail!("Cannot suspend task in state {:?}", self.state);
        }
        
        self.state = TaskState::Suspended;
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// 恢复任务
    pub fn resume(&mut self) -> Result<()> {
        if self.state != TaskState::Suspended {
            anyhow::bail!("Cannot resume task in state {:?}", self.state);
        }
        
        self.state = TaskState::Running;
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// 失败任务
    pub fn fail(&mut self, error: String) -> Result<()> {
        self.state = TaskState::Failed;
        self.error = Some(error);
        self.ended_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// 取消任务
    pub fn cancel(&mut self) -> Result<()> {
        match self.state {
            TaskState::Pending | TaskState::Running | TaskState::Suspended => {
                self.state = TaskState::Cancelled;
                self.ended_at = Some(Utc::now());
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => anyhow::bail!("Cannot cancel task in state {:?}", self.state),
        }
    }
    
    /// 创建检查点
    pub fn create_checkpoint(&self) -> TaskCheckpoint {
        TaskCheckpoint {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            completed_steps: Vec::new(), // 由外部填充
            state: self.state.clone(),
            context_snapshot: None,
            metadata: self.metadata.clone(),
        }
    }
    
    /// 恢复检查点
    pub fn restore_checkpoint(&mut self, checkpoint: &TaskCheckpoint) -> Result<()> {
        self.state = checkpoint.state.clone();
        self.metadata = checkpoint.metadata.clone();
        self.updated_at = Utc::now();
        Ok(())
    }
    
    /// 序列化
    pub fn serialize(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
    
    /// 反序列化
    pub fn deserialize(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
    
    /// 是否完成
    pub fn is_completed(&self) -> bool {
        matches!(self.state, TaskState::Completed)
    }
    
    /// 是否失败
    pub fn is_failed(&self) -> bool {
        matches!(self.state, TaskState::Failed)
    }
    
    /// 是否可执行
    pub fn is_executable(&self) -> bool {
        matches!(self.state, TaskState::Pending | TaskState::Suspended)
    }
    
    /// 获取执行时长（秒）
    pub fn duration_secs(&self) -> Option<i64> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some((end - start).num_seconds()),
            (Some(start), None) => Some((Utc::now() - start).num_seconds()),
            _ => None,
        }
    }
    
    /// 转换为摘要
    pub fn to_summary(&self) -> TaskSummary {
        TaskSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            task_type: self.task_type.clone(),
            started_at: self.started_at.unwrap_or(self.created_at),
            ended_at: self.ended_at,
            final_state: self.state.clone(),
            result_summary: None,
            tokens_used: 0, // 由外部填充
        }
    }
}

impl Default for Task {
    fn default() -> Self {
        Self::atomic("Untitled Task".to_string())
    }
}

/// 可移植任务包（支持迁移、断点续传）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableTaskPackage {
    /// 任务定义
    pub task: Task,
    /// 检查点
    pub checkpoint: TaskCheckpoint,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 版本
    pub version: String,
}

impl PortableTaskPackage {
    /// 创建可移植包
    pub fn new(task: Task) -> Self {
        let checkpoint = task.create_checkpoint();
        Self {
            task,
            checkpoint,
            timestamp: Utc::now(),
            version: "1.0".to_string(),
        }
    }
    
    /// 序列化为 JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
    
    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_id_new() {
        let id = TaskId::new();
        assert!(!id.as_str().is_empty());
    }
    
    #[test]
    fn test_task_atomic() {
        let task = Task::atomic("Test Task".to_string());
        assert_eq!(task.task_type, TaskType::Atomic);
        assert_eq!(task.state, TaskState::Pending);
    }
    
    #[test]
    fn test_task_composite() {
        let task = Task::composite("Composite Task".to_string());
        assert_eq!(task.task_type, TaskType::Composite);
    }
    
    #[test]
    fn test_task_start() {
        let mut task = Task::atomic("Test".to_string());
        task.start().unwrap();
        assert_eq!(task.state, TaskState::Running);
        assert!(task.started_at.is_some());
    }
    
    #[test]
    fn test_task_complete() {
        let mut task = Task::atomic("Test".to_string());
        task.start().unwrap();
        task.complete().unwrap();
        assert_eq!(task.state, TaskState::Completed);
        assert!(task.ended_at.is_some());
    }
    
    #[test]
    fn test_task_suspend_resume() {
        let mut task = Task::atomic("Test".to_string());
        task.start().unwrap();
        task.suspend().unwrap();
        assert_eq!(task.state, TaskState::Suspended);
        
        task.resume().unwrap();
        assert_eq!(task.state, TaskState::Running);
    }
    
    #[test]
    fn test_task_fail() {
        let mut task = Task::atomic("Test".to_string());
        task.start().unwrap();
        task.fail("Something went wrong".to_string()).unwrap();
        assert_eq!(task.state, TaskState::Failed);
        assert!(task.error.is_some());
    }
    
    #[test]
    fn test_task_cancel() {
        let mut task = Task::atomic("Test".to_string());
        task.cancel().unwrap();
        assert_eq!(task.state, TaskState::Cancelled);
    }
    
    #[test]
    fn test_task_add_constraint() {
        let mut task = Task::atomic("Test".to_string());
        let constraint = Constraint::hard("Do not delete files".to_string(), ConstraintScope::Task);
        task.add_constraint(constraint);
        
        assert_eq!(task.constraints.len(), 1);
    }
    
    #[test]
    fn test_task_add_subtask() {
        let mut task = Task::composite("Parent".to_string());
        let subtask_id = TaskId::new();
        task.add_subtask(subtask_id.clone());
        
        assert_eq!(task.subtasks.len(), 1);
    }
    
    #[test]
    fn test_task_checkpoint() {
        let task = Task::atomic("Test".to_string());
        let checkpoint = task.create_checkpoint();
        
        assert_eq!(checkpoint.state, TaskState::Pending);
    }
    
    #[test]
    fn test_task_serialize_deserialize() {
        let task = Task::atomic("Test Task".to_string())
            .with_description("A test task".to_string())
            .with_priority(TaskPriority::High);
        
        let json = task.serialize().unwrap();
        let restored = Task::deserialize(&json).unwrap();
        
        assert_eq!(restored.id, task.id);
        assert_eq!(restored.name, task.name);
        assert_eq!(restored.priority, TaskPriority::High);
    }
    
    #[test]
    fn test_portable_task_package() {
        let task = Task::atomic("Test".to_string());
        let package = PortableTaskPackage::new(task);
        
        let json = package.to_json().unwrap();
        let restored = PortableTaskPackage::from_json(&json).unwrap();
        
        assert_eq!(restored.version, "1.0");
    }
    
    #[test]
    fn test_task_duration() {
        let mut task = Task::atomic("Test".to_string());
        assert!(task.duration_secs().is_none());
        
        task.start().unwrap();
        // 等待一小段时间
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let duration = task.duration_secs().unwrap();
        assert!(duration >= 0);
    }
    
    #[test]
    fn test_task_is_methods() {
        let mut task = Task::atomic("Test".to_string());
        assert!(task.is_executable());
        assert!(!task.is_completed());
        assert!(!task.is_failed());
        
        task.start().unwrap();
        assert!(!task.is_executable());
        
        task.complete().unwrap();
        assert!(task.is_completed());
    }
    
    #[test]
    fn test_task_to_summary() {
        let mut task = Task::atomic("Test".to_string());
        task.start().unwrap();
        task.complete().unwrap();
        
        let summary = task.to_summary();
        assert_eq!(summary.name, "Test");
        assert_eq!(summary.final_state, TaskState::Completed);
    }
}