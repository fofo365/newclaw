//! Session - 会话管理（Task 的视图）
//!
//! Session 是 Task 的视图，管理任务链和上下文

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use anyhow::Result;
use uuid::Uuid;

use super::{Task, TaskId, TaskSummary, TaskState, TaskPriority};

/// 会话 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// 活跃
    Active,
    /// 空闲
    Idle,
    /// 已关闭
    Closed,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Active
    }
}

/// Focus Chain - 任务焦点链
#[derive(Debug, Clone)]
pub struct FocusChain {
    /// 待执行任务队列
    pending: VecDeque<Task>,
    /// 当前焦点任务
    current: Option<Task>,
    /// 已完成任务
    completed: Vec<TaskSummary>,
    /// 最大队列长度
    max_length: usize,
}

impl FocusChain {
    /// 创建新的 Focus Chain
    pub fn new() -> Self {
        Self {
            pending: VecDeque::with_capacity(10),
            current: None,
            completed: Vec::new(),
            max_length: 100,
        }
    }
    
    /// 添加任务到队列
    pub fn push(&mut self, task: Task) -> Result<()> {
        if self.pending.len() >= self.max_length {
            anyhow::bail!("Focus chain is full (max {})", self.max_length);
        }
        
        self.pending.push_back(task);
        Ok(())
    }
    
    /// 获取下一个任务（成为焦点）
    pub fn pop(&mut self) -> Option<Task> {
        self.current = self.pending.pop_front();
        self.current.clone()
    }
    
    /// 获取当前焦点任务
    pub fn current(&self) -> Option<&Task> {
        self.current.as_ref()
    }
    
    /// 获取当前焦点任务（可变）
    pub fn current_mut(&mut self) -> Option<&mut Task> {
        self.current.as_mut()
    }
    
    /// 完成当前任务
    pub fn complete_current(&mut self) -> Option<TaskSummary> {
        if let Some(mut task) = self.current.take() {
            let _ = task.complete();
            let summary = task.to_summary();
            self.completed.push(summary.clone());
            Some(summary)
        } else {
            None
        }
    }
    
    /// 切换焦点（暂停当前，开始下一个）
    pub fn switch(&mut self) -> Result<Option<Task>> {
        // 保存当前任务状态
        if let Some(ref mut current) = self.current {
            current.suspend()?;
        }
        
        // 如果有下一个任务，将其插入到队列前面
        if let Some(next) = self.pending.pop_front() {
            if let Some(current) = self.current.take() {
                self.pending.push_front(current);
            }
            self.current = Some(next);
        }
        
        Ok(self.current.clone())
    }
    
    /// 获取待执行任务数
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
    
    /// 获取已完成任务数
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.current.is_none()
    }
    
    /// 清空队列
    pub fn clear(&mut self) {
        self.pending.clear();
        self.current = None;
    }
    
    /// 获取所有待执行任务 ID
    pub fn pending_ids(&self) -> Vec<TaskId> {
        self.pending.iter()
            .chain(self.current.iter())
            .map(|t| t.id.clone())
            .collect()
    }
    
    /// 按优先级排序
    pub fn sort_by_priority(&mut self) {
        let mut tasks: Vec<_> = self.pending.drain(..).collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.pending = tasks.into_iter().collect();
    }
}

impl Default for FocusChain {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// 最大任务数
    pub max_tasks: usize,
    /// 会话超时（秒）
    pub timeout_secs: u64,
    /// 自动保存间隔（秒）
    pub auto_save_secs: u64,
    /// 保留历史任务数
    pub keep_history: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_tasks: 100,
            timeout_secs: 3600,
            auto_save_secs: 60,
            keep_history: 50,
        }
    }
}

/// 会话定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话 ID
    pub id: SessionId,
    /// 会话名称
    pub name: String,
    /// 会话状态
    pub state: SessionState,
    /// Focus Chain
    #[serde(skip)]
    focus_chain: FocusChain,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 最后活动时间
    pub last_activity: DateTime<Utc>,
    /// 配置
    #[serde(default)]
    pub config: SessionConfig,
    /// 元数据
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl Session {
    /// 创建新会话
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            name,
            state: SessionState::Active,
            focus_chain: FocusChain::new(),
            created_at: now,
            updated_at: now,
            last_activity: now,
            config: SessionConfig::default(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    /// 获取当前任务
    pub fn current_task(&self) -> Option<&Task> {
        self.focus_chain.current()
    }
    
    /// 获取当前任务（可变）
    pub fn current_task_mut(&mut self) -> Option<&mut Task> {
        self.focus_chain.current_mut()
    }
    
    /// 添加任务
    pub fn add_task(&mut self, task: Task) -> Result<()> {
        self.touch();
        self.focus_chain.push(task)
    }
    
    /// 开始下一个任务
    pub fn next_task(&mut self) -> Option<Task> {
        self.touch();
        self.focus_chain.pop()
    }
    
    /// 完成当前任务
    pub fn complete_current_task(&mut self) -> Option<TaskSummary> {
        self.touch();
        self.focus_chain.complete_current()
    }
    
    /// 切换任务焦点
    pub fn switch_focus(&mut self) -> Result<Option<Task>> {
        self.touch();
        self.focus_chain.switch()
    }
    
    /// 获取任务历史
    pub fn task_history(&self) -> &[TaskSummary] {
        &self.focus_chain.completed
    }
    
    /// 获取待执行任务
    pub fn pending_tasks(&self) -> usize {
        self.focus_chain.pending_count()
    }
    
    /// 关闭会话
    pub fn close(&mut self) {
        self.state = SessionState::Closed;
        self.updated_at = Utc::now();
    }
    
    /// 激活会话
    pub fn activate(&mut self) {
        self.state = SessionState::Active;
        self.touch();
    }
    
    /// 设置空闲
    pub fn set_idle(&mut self) {
        self.state = SessionState::Idle;
        self.updated_at = Utc::now();
    }
    
    /// 更新活动时间
    fn touch(&mut self) {
        self.last_activity = Utc::now();
        self.updated_at = Utc::now();
    }
    
    /// 检查是否超时
    pub fn is_timeout(&self) -> bool {
        let elapsed = (Utc::now() - self.last_activity).num_seconds() as u64;
        elapsed > self.config.timeout_secs
    }
    
    /// 序列化
    pub fn serialize(&self) -> Result<String> {
        Ok(serde_json::to_string(self)?)
    }
    
    /// 反序列化
    pub fn deserialize(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
    
    /// 是否活跃
    pub fn is_active(&self) -> bool {
        matches!(self.state, SessionState::Active)
    }
    
    /// 获取统计
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            total_tasks: self.focus_chain.completed_count() + self.focus_chain.pending_count(),
            completed_tasks: self.focus_chain.completed_count(),
            pending_tasks: self.focus_chain.pending_count(),
            session_duration_secs: (Utc::now() - self.created_at).num_seconds() as u64,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new("Untitled Session".to_string())
    }
}

/// 会话统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub pending_tasks: usize,
    pub session_duration_secs: u64,
}

/// 会话管理器
pub struct SessionManager {
    /// 当前活跃会话
    current: Option<Session>,
    /// 历史会话
    history: Vec<Session>,
    /// 最大历史数
    max_history: usize,
}

impl SessionManager {
    /// 创建会话管理器
    pub fn new() -> Self {
        Self {
            current: None,
            history: Vec::new(),
            max_history: 10,
        }
    }
    
    /// 创建新会话
    pub fn create_session(&mut self, name: String) -> &Session {
        // 保存当前会话
        if let Some(current) = self.current.take() {
            self.history.push(current);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
        
        self.current = Some(Session::new(name));
        self.current.as_ref().unwrap()
    }
    
    /// 获取当前会话
    pub fn current(&self) -> Option<&Session> {
        self.current.as_ref()
    }
    
    /// 获取当前会话（可变）
    pub fn current_mut(&mut self) -> Option<&mut Session> {
        self.current.as_mut()
    }
    
    /// 关闭当前会话
    pub fn close_current(&mut self) -> Option<Session> {
        if let Some(mut session) = self.current.take() {
            session.close();
            self.history.push(session.clone());
            Some(session)
        } else {
            None
        }
    }
    
    /// 恢复会话
    pub fn restore_session(&mut self, mut session: Session) {
        session.activate();
        self.current = Some(session);
    }
    
    /// 获取历史会话
    pub fn history(&self) -> &[Session] {
        &self.history
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_session_id() {
        let id = SessionId::new();
        assert!(!id.as_str().is_empty());
    }
    
    #[test]
    fn test_focus_chain_new() {
        let chain = FocusChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.pending_count(), 0);
    }
    
    #[test]
    fn test_focus_chain_push_pop() {
        let mut chain = FocusChain::new();
        let task = Task::atomic("Test".to_string());
        
        chain.push(task).unwrap();
        assert_eq!(chain.pending_count(), 1);
        
        let popped = chain.pop();
        assert!(popped.is_some());
        assert!(chain.current().is_some());
    }
    
    #[test]
    fn test_focus_chain_complete() {
        let mut chain = FocusChain::new();
        let task = Task::atomic("Test".to_string());
        
        chain.push(task).unwrap();
        chain.pop();
        
        let summary = chain.complete_current();
        assert!(summary.is_some());
        assert_eq!(chain.completed_count(), 1);
    }
    
    #[test]
    fn test_session_new() {
        let session = Session::new("Test Session".to_string());
        assert_eq!(session.name, "Test Session");
        assert!(session.is_active());
    }
    
    #[test]
    fn test_session_add_task() {
        let mut session = Session::new("Test".to_string());
        let task = Task::atomic("Test Task".to_string());
        
        session.add_task(task).unwrap();
        assert_eq!(session.pending_tasks(), 1);
    }
    
    #[test]
    fn test_session_complete_task() {
        let mut session = Session::new("Test".to_string());
        let task = Task::atomic("Test Task".to_string());
        
        session.add_task(task).unwrap();
        session.next_task();
        
        let summary = session.complete_current_task();
        assert!(summary.is_some());
        assert_eq!(session.task_history().len(), 1);
    }
    
    #[test]
    fn test_session_close() {
        let mut session = Session::new("Test".to_string());
        session.close();
        
        assert_eq!(session.state, SessionState::Closed);
    }
    
    #[test]
    fn test_session_stats() {
        let mut session = Session::new("Test".to_string());
        session.add_task(Task::atomic("T1".to_string())).unwrap();
        session.add_task(Task::atomic("T2".to_string())).unwrap();
        
        let stats = session.stats();
        assert_eq!(stats.pending_tasks, 2);
    }
    
    #[test]
    fn test_session_manager() {
        let mut manager = SessionManager::new();
        
        manager.create_session("Session 1".to_string());
        assert!(manager.current().is_some());
        
        manager.close_current();
        assert!(manager.current().is_none());
        assert_eq!(manager.history().len(), 1);
    }
    
    #[test]
    fn test_session_serialize() {
        let session = Session::new("Test".to_string());
        let json = session.serialize().unwrap();
        let restored = Session::deserialize(&json).unwrap();
        
        assert_eq!(restored.name, "Test");
    }
}