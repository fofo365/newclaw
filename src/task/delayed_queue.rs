//! Delayed Queue - 延迟任务队列
//!
//! v0.7.0 - 基于时间优先队列的延迟任务执行

use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc, oneshot};
use anyhow::Result;
use tracing::{info, warn, debug};
use uuid::Uuid;

/// 延迟任务 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DelayedTaskId(String);

impl DelayedTaskId {
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

impl Default for DelayedTaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DelayedTaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 延迟任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DelayedTaskState {
    /// 等待中
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 已失败
    Failed,
}

/// 延迟任务
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelayedTask {
    /// 任务 ID
    pub id: DelayedTaskId,
    /// 任务名称
    pub name: String,
    /// 任务类型
    pub task_type: String,
    /// 任务参数
    pub params: serde_json::Value,
    /// 延迟时间（秒）
    pub delay_secs: u64,
    /// 执行时间
    pub execute_at: DateTime<Utc>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 状态
    pub state: DelayedTaskState,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 优先级（数值越大优先级越高）
    pub priority: i32,
    /// 元数据
    pub metadata: std::collections::HashMap<String, String>,
    /// 结果
    pub result: Option<String>,
    /// 错误信息
    pub error: Option<String>,
}

impl DelayedTask {
    /// 创建新的延迟任务
    pub fn new(name: String, task_type: String, params: serde_json::Value, delay_secs: u64) -> Self {
        let now = Utc::now();
        let execute_at = now + chrono::Duration::seconds(delay_secs as i64);
        
        Self {
            id: DelayedTaskId::new(),
            name,
            task_type,
            params,
            delay_secs,
            execute_at,
            created_at: now,
            state: DelayedTaskState::Pending,
            retry_count: 0,
            max_retries: 3,
            priority: 0,
            metadata: std::collections::HashMap::new(),
            result: None,
            error: None,
        }
    }
    
    /// 设置执行时间
    pub fn at(name: String, task_type: String, params: serde_json::Value, execute_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        let delay_secs = (execute_at - now).num_seconds().max(0) as u64;
        
        Self {
            id: DelayedTaskId::new(),
            name,
            task_type,
            params,
            delay_secs,
            execute_at,
            created_at: now,
            state: DelayedTaskState::Pending,
            retry_count: 0,
            max_retries: 3,
            priority: 0,
            metadata: std::collections::HashMap::new(),
            result: None,
            error: None,
        }
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 设置最大重试次数
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// 是否到期
    pub fn is_due(&self) -> bool {
        Utc::now() >= self.execute_at
    }
    
    /// 剩余时间（秒）
    pub fn remaining_secs(&self) -> i64 {
        (self.execute_at - Utc::now()).num_seconds().max(0)
    }
    
    /// 标记为执行中
    pub fn start(&mut self) {
        self.state = DelayedTaskState::Running;
    }
    
    /// 标记为完成
    pub fn complete(&mut self, result: String) {
        self.state = DelayedTaskState::Completed;
        self.result = Some(result);
    }
    
    /// 标记为失败
    pub fn fail(&mut self, error: String) {
        self.retry_count += 1;
        
        if self.retry_count >= self.max_retries {
            self.state = DelayedTaskState::Failed;
            self.error = Some(error);
        } else {
            // 可以重试
            self.state = DelayedTaskState::Pending;
            self.error = Some(error);
        }
    }
    
    /// 取消任务
    pub fn cancel(&mut self) {
        self.state = DelayedTaskState::Cancelled;
    }
    
    /// 是否可以执行
    pub fn is_executable(&self) -> bool {
        matches!(self.state, DelayedTaskState::Pending) && self.is_due()
    }
    
    /// 是否已结束
    pub fn is_finished(&self) -> bool {
        matches!(
            self.state,
            DelayedTaskState::Completed | DelayedTaskState::Failed | DelayedTaskState::Cancelled
        )
    }
}

/// 用于优先队列的包装
#[derive(Debug, Clone)]
struct PrioritizedTask {
    task: DelayedTask,
    execute_at: DateTime<Utc>,
    priority: i32,
    id: DelayedTaskId,
}

impl Eq for PrioritizedTask {}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 按执行时间和优先级排序
        // BinaryHeap 是最大堆，我们想要最早执行的任务在堆顶
        // 所以使用反向比较
        other.execute_at.cmp(&self.execute_at)
            .then_with(|| other.priority.cmp(&self.priority))
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// 队列命令
enum QueueCommand {
    /// 提交延迟任务
    Submit {
        task: DelayedTask,
        response: oneshot::Sender<Result<DelayedTaskId>>,
    },
    /// 取消任务
    Cancel {
        id: DelayedTaskId,
        response: oneshot::Sender<Result<bool>>,
    },
    /// 获取任务
    Get {
        id: DelayedTaskId,
        response: oneshot::Sender<Option<DelayedTask>>,
    },
    /// 列出所有任务
    List {
        response: oneshot::Sender<Vec<DelayedTask>>,
    },
    /// 获取队列长度
    Len {
        response: oneshot::Sender<usize>,
    },
    /// 清空队列
    Clear {
        response: oneshot::Sender<()>,
    },
    /// 停止队列
    Stop,
}

/// 延迟任务队列统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelayedQueueStats {
    /// 总任务数
    pub total_tasks: usize,
    /// 待执行任务数
    pub pending_tasks: usize,
    /// 已完成任务数
    pub completed_tasks: usize,
    /// 已失败任务数
    pub failed_tasks: usize,
    /// 已取消任务数
    pub cancelled_tasks: usize,
    /// 平均延迟时间（秒）
    pub avg_delay_secs: f64,
}

/// 延迟任务队列
pub struct DelayedQueue {
    /// 任务映射
    tasks: Arc<RwLock<std::collections::HashMap<DelayedTaskId, DelayedTask>>>,
    /// 命令发送器
    command_tx: mpsc::Sender<QueueCommand>,
    /// 停止信号
    stop_tx: Option<oneshot::Sender<()>>,
    /// 统计信息
    stats: Arc<RwLock<DelayedQueueStats>>,
}

/// 任务执行回调类型
pub type TaskCallback = Box<dyn Fn(DelayedTask) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> + Send + Sync>;

impl DelayedQueue {
    /// 创建新的延迟队列
    pub fn new() -> Self {
        let tasks = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let stats = Arc::new(RwLock::new(DelayedQueueStats {
            total_tasks: 0,
            pending_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            cancelled_tasks: 0,
            avg_delay_secs: 0.0,
        }));
        let (command_tx, _command_rx) = mpsc::channel(100);
        
        Self {
            tasks: tasks.clone(),
            command_tx,
            stop_tx: None,
            stats,
        }
    }
    
    /// 启动队列处理
    pub async fn start<F, Fut>(&mut self, executor: F) -> Result<()>
    where
        F: Fn(DelayedTask) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<String>> + Send + 'static,
    {
        let (stop_tx, mut stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);
        
        let tasks = self.tasks.clone();
        let stats = self.stats.clone();
        let (cmd_tx, mut cmd_rx) = mpsc::channel(100);
        self.command_tx = cmd_tx;
        
        // 创建执行器包装
        let executor = Arc::new(executor);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // 检查到期任务
                        let mut tasks_guard = tasks.write().await;
                        let mut due_tasks: Vec<DelayedTask> = Vec::new();
                        
                        for (id, task) in tasks_guard.iter_mut() {
                            if task.is_executable() {
                                let mut task_clone = task.clone();
                                task_clone.start();
                                *task = task_clone.clone();
                                due_tasks.push(task_clone);
                            }
                        }
                        
                        // 释放锁后执行任务
                        drop(tasks_guard);
                        
                        for task in due_tasks {
                            let id = task.id.clone();
                            let result = executor(task.clone()).await;
                            
                            let mut tasks_guard = tasks.write().await;
                            if let Some(t) = tasks_guard.get_mut(&id) {
                                match result {
                                    Ok(res) => {
                                        t.complete(res);
                                        let mut stats_guard = stats.write().await;
                                        stats_guard.completed_tasks += 1;
                                        stats_guard.pending_tasks = stats_guard.pending_tasks.saturating_sub(1);
                                    }
                                    Err(e) => {
                                        t.fail(e.to_string());
                                        if t.is_finished() {
                                            let mut stats_guard = stats.write().await;
                                            stats_guard.failed_tasks += 1;
                                            stats_guard.pending_tasks = stats_guard.pending_tasks.saturating_sub(1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    cmd = cmd_rx.recv() => {
                        if let Some(cmd) = cmd {
                            match cmd {
                                QueueCommand::Submit { task, response } => {
                                    let id = task.id.clone();
                                    let mut tasks_guard = tasks.write().await;
                                    tasks_guard.insert(id.clone(), task);
                                    
                                    let mut stats_guard = stats.write().await;
                                    stats_guard.total_tasks += 1;
                                    stats_guard.pending_tasks += 1;
                                    
                                    let _ = response.send(Ok(id));
                                }
                                QueueCommand::Cancel { id, response } => {
                                    let mut tasks_guard = tasks.write().await;
                                    if let Some(task) = tasks_guard.get_mut(&id) {
                                        if matches!(task.state, DelayedTaskState::Pending) {
                                            task.cancel();
                                            let mut stats_guard = stats.write().await;
                                            stats_guard.cancelled_tasks += 1;
                                            stats_guard.pending_tasks = stats_guard.pending_tasks.saturating_sub(1);
                                            let _ = response.send(Ok(true));
                                        } else {
                                            let _ = response.send(Ok(false));
                                        }
                                    } else {
                                        let _ = response.send(Err(anyhow::anyhow!("Task not found")));
                                    }
                                }
                                QueueCommand::Get { id, response } => {
                                    let tasks_guard = tasks.read().await;
                                    let _ = response.send(tasks_guard.get(&id).cloned());
                                }
                                QueueCommand::List { response } => {
                                    let tasks_guard = tasks.read().await;
                                    let list: Vec<_> = tasks_guard.values().cloned().collect();
                                    let _ = response.send(list);
                                }
                                QueueCommand::Len { response } => {
                                    let tasks_guard = tasks.read().await;
                                    let _ = response.send(tasks_guard.len());
                                }
                                QueueCommand::Clear { response } => {
                                    let mut tasks_guard = tasks.write().await;
                                    tasks_guard.clear();
                                    let mut stats_guard = stats.write().await;
                                    *stats_guard = DelayedQueueStats {
                                        total_tasks: 0,
                                        pending_tasks: 0,
                                        completed_tasks: 0,
                                        failed_tasks: 0,
                                        cancelled_tasks: 0,
                                        avg_delay_secs: 0.0,
                                    };
                                    let _ = response.send(());
                                }
                                QueueCommand::Stop => {
                                    break;
                                }
                            }
                        }
                    }
                    
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 停止队列
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        Ok(())
    }
    
    /// 提交延迟任务
    pub async fn submit(&self, task: DelayedTask) -> Result<DelayedTaskId> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(QueueCommand::Submit { task, response: tx }).await?;
        rx.await?
    }
    
    /// 提交延迟任务（便捷方法）
    pub async fn submit_delayed(
        &self,
        name: String,
        task_type: String,
        params: serde_json::Value,
        delay_secs: u64,
    ) -> Result<DelayedTaskId> {
        let task = DelayedTask::new(name, task_type, params, delay_secs);
        self.submit(task).await
    }
    
    /// 提交定时任务
    pub async fn submit_at(
        &self,
        name: String,
        task_type: String,
        params: serde_json::Value,
        execute_at: DateTime<Utc>,
    ) -> Result<DelayedTaskId> {
        let task = DelayedTask::at(name, task_type, params, execute_at);
        self.submit(task).await
    }
    
    /// 取消任务
    pub async fn cancel(&self, id: DelayedTaskId) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(QueueCommand::Cancel { id, response: tx }).await?;
        rx.await?
    }
    
    /// 获取任务
    pub async fn get(&self, id: &DelayedTaskId) -> Option<DelayedTask> {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(QueueCommand::Get { id: id.clone(), response: tx }).await.is_err() {
            return None;
        }
        rx.await.ok()?
    }
    
    /// 列出所有任务
    pub async fn list(&self) -> Vec<DelayedTask> {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(QueueCommand::List { response: tx }).await.is_err() {
            return vec![];
        }
        rx.await.unwrap_or_default()
    }
    
    /// 获取队列长度
    pub async fn len(&self) -> usize {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(QueueCommand::Len { response: tx }).await.is_err() {
            return 0;
        }
        rx.await.unwrap_or(0)
    }
    
    /// 清空队列
    pub async fn clear(&self) {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(QueueCommand::Clear { response: tx }).await.is_err() {
            return;
        }
        let _ = rx.await;
    }
    
    /// 获取统计信息
    pub async fn stats(&self) -> DelayedQueueStats {
        self.stats.read().await.clone()
    }
}

impl Default for DelayedQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_delayed_task_new() {
        let task = DelayedTask::new(
            "test-task".to_string(),
            "shell".to_string(),
            serde_json::json!({"command": "echo hello"}),
            60,
        );
        
        assert_eq!(task.name, "test-task");
        assert_eq!(task.delay_secs, 60);
        assert!(task.remaining_secs() > 0);
        assert!(!task.is_due());
    }
    
    #[test]
    fn test_delayed_task_at() {
        let execute_at = Utc::now() + chrono::Duration::seconds(120);
        let task = DelayedTask::at(
            "scheduled-task".to_string(),
            "shell".to_string(),
            serde_json::Value::Null,
            execute_at,
        );
        
        assert!(task.remaining_secs() > 100 && task.remaining_secs() <= 120);
    }
    
    #[test]
    fn test_delayed_task_with_options() {
        let task = DelayedTask::new("test".to_string(), "shell".to_string(), serde_json::Value::Null, 10)
            .with_priority(5)
            .with_max_retries(5)
            .with_metadata("key".to_string(), "value".to_string());
        
        assert_eq!(task.priority, 5);
        assert_eq!(task.max_retries, 5);
        assert_eq!(task.metadata.get("key"), Some(&"value".to_string()));
    }
    
    #[test]
    fn test_delayed_task_state_transitions() {
        let mut task = DelayedTask::new("test".to_string(), "shell".to_string(), serde_json::Value::Null, 0);
        
        // 等待一小段时间让任务到期
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        assert!(task.is_executable());
        
        task.start();
        assert_eq!(task.state, DelayedTaskState::Running);
        assert!(!task.is_executable());
        
        task.complete("success".to_string());
        assert_eq!(task.state, DelayedTaskState::Completed);
        assert!(task.is_finished());
        assert_eq!(task.result, Some("success".to_string()));
    }
    
    #[test]
    fn test_delayed_task_retry() {
        let mut task = DelayedTask::new("test".to_string(), "shell".to_string(), serde_json::Value::Null, 0)
            .with_max_retries(2);
        
        task.start();
        task.fail("error 1".to_string());
        assert_eq!(task.state, DelayedTaskState::Pending); // 可以重试
        assert_eq!(task.retry_count, 1);
        
        task.start();
        task.fail("error 2".to_string());
        assert_eq!(task.state, DelayedTaskState::Failed); // 达到最大重试次数
        assert_eq!(task.retry_count, 2);
        assert!(task.is_finished());
    }
    
    #[test]
    fn test_delayed_task_cancel() {
        let mut task = DelayedTask::new("test".to_string(), "shell".to_string(), serde_json::Value::Null, 10);
        
        task.cancel();
        assert_eq!(task.state, DelayedTaskState::Cancelled);
        assert!(task.is_finished());
    }
    
    #[tokio::test]
    async fn test_delayed_queue_basic() {
        let mut queue = DelayedQueue::new();
        
        // 启动队列
        queue.start(|task| async move {
            Ok(format!("Executed: {}", task.name))
        }).await.unwrap();
        
        // 等待启动
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // 提交任务
        let id = queue.submit_delayed(
            "test-task".to_string(),
            "shell".to_string(),
            serde_json::Value::Null,
            0, // 立即执行
        ).await.unwrap();
        
        // 等待执行
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // 检查结果
        let task = queue.get(&id).await;
        assert!(task.is_some());
        let task = task.unwrap();
        assert_eq!(task.state, DelayedTaskState::Completed);
        
        // 停止队列
        queue.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_delayed_queue_cancel() {
        let mut queue = DelayedQueue::new();
        
        queue.start(|task| async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok("done".to_string())
        }).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // 提交延迟任务
        let id = queue.submit_delayed(
            "cancel-test".to_string(),
            "shell".to_string(),
            serde_json::Value::Null,
            10, // 10秒后执行
        ).await.unwrap();
        
        // 取消任务
        let cancelled = queue.cancel(id.clone()).await.unwrap();
        assert!(cancelled);
        
        // 检查状态
        let task = queue.get(&id).await.unwrap();
        assert_eq!(task.state, DelayedTaskState::Cancelled);
        
        queue.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_delayed_queue_stats() {
        let mut queue = DelayedQueue::new();
        
        queue.start(|task| async move {
            Ok("done".to_string())
        }).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        // 提交多个任务
        for i in 0..5 {
            queue.submit_delayed(
                format!("task-{}", i),
                "shell".to_string(),
                serde_json::Value::Null,
                0,
            ).await.unwrap();
        }
        
        // 等待执行
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        let stats = queue.stats().await;
        assert_eq!(stats.total_tasks, 5);
        assert_eq!(stats.completed_tasks, 5);
        
        queue.stop().await.unwrap();
    }
    
    #[test]
    fn test_prioritized_task_ordering() {
        let now = Utc::now();
        
        let task1 = DelayedTask::new("t1".to_string(), "shell".to_string(), serde_json::Value::Null, 10);
        let task2 = DelayedTask::new("t2".to_string(), "shell".to_string(), serde_json::Value::Null, 5);
        
        let t1 = PrioritizedTask {
            task: task1.clone(),
            execute_at: task1.execute_at,
            priority: task1.priority,
            id: task1.id,
        };
        
        let t2 = PrioritizedTask {
            task: task2.clone(),
            execute_at: task2.execute_at,
            priority: task2.priority,
            id: task2.id,
        };
        
        // t2 应该先执行（延迟更短）
        assert!(t2 < t1);
    }
}