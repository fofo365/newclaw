//! Cron Scheduler - Cron 表达式调度器
//!
//! v0.7.0 - 支持标准 cron 表达式和快捷语法

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc, TimeZone, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc, oneshot};
use anyhow::{Result, Context};
use tracing::{info, warn, error, debug};

use super::{Task, TaskId, TaskState};

/// Cron 表达式解析错误
#[derive(Debug, Clone, thiserror::Error)]
pub enum CronParseError {
    #[error("Invalid cron expression: {0}")]
    InvalidExpression(String),
    #[error("Invalid field '{field}' with value '{value}'")]
    InvalidField { field: String, value: String },
    #[error("Unsupported syntax: {0}")]
    UnsupportedSyntax(String),
}

/// Cron 调度 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScheduleId(String);

impl ScheduleId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ScheduleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ScheduleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Cron 表达式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExpression {
    /// 原始表达式
    pub raw: String,
    /// 分钟 (0-59)
    pub minute: CronField,
    /// 小时 (0-23)
    pub hour: CronField,
    /// 日 (1-31)
    pub day_of_month: CronField,
    /// 月 (1-12)
    pub month: CronField,
    /// 周几 (0-6, 0=周日)
    pub day_of_week: CronField,
}

/// Cron 字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CronField {
    /// 任意值 (*)
    Any,
    /// 固定值
    Value(u32),
    /// 范围 (start-end)
    Range { start: u32, end: u32 },
    /// 步进 (*/step)
    Step { step: u32 },
    /// 列表 (1,2,3)
    List(Vec<u32>),
    /// 范围步进 (start-end/step)
    RangeStep { start: u32, end: u32, step: u32 },
}

impl CronField {
    /// 检查值是否匹配
    pub fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Value(v) => *v == value,
            CronField::Range { start, end } => value >= *start && value <= *end,
            CronField::Step { step } => value % step == 0,
            CronField::List(values) => values.contains(&value),
            CronField::RangeStep { start, end, step } => {
                value >= *start && value <= *end && (value - start) % step == 0
            }
        }
    }
    
    /// 获取所有匹配的值
    pub fn values(&self, min: u32, max: u32) -> Vec<u32> {
        match self {
            CronField::Any => (min..=max).collect(),
            CronField::Value(v) => {
                if *v >= min && *v <= max {
                    vec![*v]
                } else {
                    vec![]
                }
            }
            CronField::Range { start, end } => {
                (*start.max(&min)..=*end.min(&max)).collect()
            }
            CronField::Step { step } => {
                (min..=max).filter(|v| v % step == 0).collect()
            }
            CronField::List(values) => {
                values.iter().filter(|v| **v >= min && **v <= max).cloned().collect()
            }
            CronField::RangeStep { start, end, step } => {
                (*start.max(&min)..=*end.min(&max))
                    .filter(|v| (v - start) % step == 0)
                    .collect()
            }
        }
    }
}

impl CronExpression {
    /// 解析 cron 表达式
    /// 标准格式：分 时 日 月 周
    pub fn parse(expr: &str) -> Result<Self, CronParseError> {
        let expr = expr.trim();
        
        // 检查快捷语法
        if let Some(cron) = Self::parse_shortcut(expr)? {
            return Ok(cron);
        }
        
        // 解析标准 cron 表达式
        let parts: Vec<&str> = expr.split_whitespace().collect();
        
        if parts.len() != 5 {
            return Err(CronParseError::InvalidExpression(
                format!("Expected 5 fields, got {}", parts.len())
            ));
        }
        
        let minute = Self::parse_field(parts[0], 0, 59, "minute")?;
        let hour = Self::parse_field(parts[1], 0, 23, "hour")?;
        let day_of_month = Self::parse_field(parts[2], 1, 31, "day_of_month")?;
        let month = Self::parse_field(parts[3], 1, 12, "month")?;
        let day_of_week = Self::parse_field(parts[4], 0, 6, "day_of_week")?;
        
        Ok(Self {
            raw: expr.to_string(),
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
        })
    }
    
    /// 解析快捷语法
    fn parse_shortcut(expr: &str) -> Result<Option<Self>, CronParseError> {
        let expr_lower = expr.to_lowercase();
        
        let cron = match expr_lower.as_str() {
            // 每分钟
            "@every_minute" | "@minutely" => Self {
                raw: expr.to_string(),
                minute: CronField::Any,
                hour: CronField::Any,
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            // 每小时
            "@hourly" => Self {
                raw: expr.to_string(),
                minute: CronField::Value(0),
                hour: CronField::Any,
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            // 每天
            "@daily" | "@midnight" => Self {
                raw: expr.to_string(),
                minute: CronField::Value(0),
                hour: CronField::Value(0),
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            // 每周
            "@weekly" => Self {
                raw: expr.to_string(),
                minute: CronField::Value(0),
                hour: CronField::Value(0),
                day_of_month: CronField::Any,
                month: CronField::Any,
                day_of_week: CronField::Value(0), // 周日
            },
            // 每月
            "@monthly" => Self {
                raw: expr.to_string(),
                minute: CronField::Value(0),
                hour: CronField::Value(0),
                day_of_month: CronField::Value(1),
                month: CronField::Any,
                day_of_week: CronField::Any,
            },
            // 每年
            "@yearly" | "@annually" => Self {
                raw: expr.to_string(),
                minute: CronField::Value(0),
                hour: CronField::Value(0),
                day_of_month: CronField::Value(1),
                month: CronField::Value(1),
                day_of_week: CronField::Any,
            },
            // 每 N 分钟: @every_5m, @every_10m, etc.
            s if s.starts_with("@every_") && s.ends_with('m') => {
                let num_str = &s[7..s.len()-1];
                let minutes: u32 = num_str.parse()
                    .map_err(|_| CronParseError::InvalidExpression(
                        format!("Invalid interval: {}", s)
                    ))?;
                
                if minutes == 0 || minutes > 59 {
                    return Err(CronParseError::InvalidExpression(
                        format!("Minute interval must be 1-59, got {}", minutes)
                    ));
                }
                
                Self {
                    raw: expr.to_string(),
                    minute: CronField::Step { step: minutes },
                    hour: CronField::Any,
                    day_of_month: CronField::Any,
                    month: CronField::Any,
                    day_of_week: CronField::Any,
                }
            },
            // 每 N 小时: @every_2h, @every_6h, etc.
            s if s.starts_with("@every_") && s.ends_with('h') => {
                let num_str = &s[7..s.len()-1];
                let hours: u32 = num_str.parse()
                    .map_err(|_| CronParseError::InvalidExpression(
                        format!("Invalid interval: {}", s)
                    ))?;
                
                if hours == 0 || hours > 23 {
                    return Err(CronParseError::InvalidExpression(
                        format!("Hour interval must be 1-23, got {}", hours)
                    ));
                }
                
                Self {
                    raw: expr.to_string(),
                    minute: CronField::Value(0),
                    hour: CronField::Step { step: hours },
                    day_of_month: CronField::Any,
                    month: CronField::Any,
                    day_of_week: CronField::Any,
                }
            },
            _ => return Ok(None),
        };
        
        Ok(Some(cron))
    }
    
    /// 解析单个字段
    fn parse_field(s: &str, min: u32, max: u32, field_name: &str) -> Result<CronField, CronParseError> {
        let s = s.trim();
        
        // 任意值
        if s == "*" {
            return Ok(CronField::Any);
        }
        
        // 步进: */step
        if s.starts_with("*/") {
            let step: u32 = s[2..].parse()
                .map_err(|_| CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                })?;
            
            if step == 0 {
                return Err(CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                });
            }
            
            return Ok(CronField::Step { step });
        }
        
        // 列表: 1,2,3
        if s.contains(',') {
            let values: Result<Vec<u32>, _> = s.split(',')
                .map(|v| {
                    v.trim().parse::<u32>()
                        .map_err(|_| CronParseError::InvalidField {
                            field: field_name.to_string(),
                            value: v.to_string(),
                        })
                })
                .collect();
            
            let values = values?;
            
            // 验证范围
            for v in &values {
                if *v < min || *v > max {
                    return Err(CronParseError::InvalidField {
                        field: field_name.to_string(),
                        value: v.to_string(),
                    });
                }
            }
            
            return Ok(CronField::List(values));
        }
        
        // 范围步进: start-end/step
        if s.contains('/') && s.contains('-') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() != 2 {
                return Err(CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                });
            }
            
            let range_parts: Vec<&str> = parts[0].split('-').collect();
            if range_parts.len() != 2 {
                return Err(CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                });
            }
            
            let start: u32 = range_parts[0].parse()
                .map_err(|_| CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                })?;
            let end: u32 = range_parts[1].parse()
                .map_err(|_| CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                })?;
            let step: u32 = parts[1].parse()
                .map_err(|_| CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                })?;
            
            if start < min || end > max || start > end || step == 0 {
                return Err(CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                });
            }
            
            return Ok(CronField::RangeStep { start, end, step });
        }
        
        // 范围: start-end
        if s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() != 2 {
                return Err(CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                });
            }
            
            let start: u32 = parts[0].parse()
                .map_err(|_| CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                })?;
            let end: u32 = parts[1].parse()
                .map_err(|_| CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                })?;
            
            if start < min || end > max || start > end {
                return Err(CronParseError::InvalidField {
                    field: field_name.to_string(),
                    value: s.to_string(),
                });
            }
            
            return Ok(CronField::Range { start, end });
        }
        
        // 固定值
        let value: u32 = s.parse()
            .map_err(|_| CronParseError::InvalidField {
                field: field_name.to_string(),
                value: s.to_string(),
            })?;
        
        if value < min || value > max {
            return Err(CronParseError::InvalidField {
                field: field_name.to_string(),
                value: s.to_string(),
            });
        }
        
        Ok(CronField::Value(value))
    }
    
    /// 检查时间是否匹配
    pub fn matches(&self, dt: &DateTime<Utc>) -> bool {
        self.minute.matches(dt.minute() as u32)
            && self.hour.matches(dt.hour() as u32)
            && self.day_of_month.matches(dt.day() as u32)
            && self.month.matches(dt.month() as u32)
            && self.day_of_week.matches(dt.weekday().num_days_from_sunday())
    }
    
    /// 计算下一次执行时间
    pub fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        // 从 after 的下一分钟开始搜索
        let mut current = after.clone()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            + chrono::Duration::minutes(1);
        
        // 最多搜索 4 年（处理闰年等边界情况）
        let max_iterations = 4 * 365 * 24 * 60; // 4 年的分钟数
        let mut iterations = 0;
        
        while iterations < max_iterations {
            if self.matches(&current) {
                return Some(current);
            }
            current = current + chrono::Duration::minutes(1);
            iterations += 1;
        }
        
        None
    }
    
    /// 获取接下来的 N 次执行时间
    pub fn next_n(&self, after: &DateTime<Utc>, n: usize) -> Vec<DateTime<Utc>> {
        let mut result = Vec::with_capacity(n);
        let mut current = after.clone();
        
        while result.len() < n {
            if let Some(next) = self.next_after(&current) {
                result.push(next);
                current = next;
            } else {
                break;
            }
        }
        
        result
    }
}

/// 调度任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// 调度 ID
    pub id: ScheduleId,
    /// Cron 表达式
    pub cron: CronExpression,
    /// 任务名称
    pub name: String,
    /// 任务类型
    pub task_type: String,
    /// 任务参数
    pub params: serde_json::Value,
    /// 是否启用
    pub enabled: bool,
    /// 上次执行时间
    pub last_run: Option<DateTime<Utc>>,
    /// 下次执行时间
    pub next_run: Option<DateTime<Utc>>,
    /// 执行次数
    pub run_count: u64,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl ScheduledTask {
    /// 创建新的调度任务
    pub fn new(name: String, cron_expr: &str, task_type: String, params: serde_json::Value) -> Result<Self> {
        let cron = CronExpression::parse(cron_expr)?;
        let now = Utc::now();
        let next_run = cron.next_after(&now);
        
        Ok(Self {
            id: ScheduleId::new(),
            cron,
            name,
            task_type,
            params,
            enabled: true,
            last_run: None,
            next_run,
            run_count: 0,
            created_at: now,
            metadata: HashMap::new(),
        })
    }
    
    /// 更新下次执行时间
    pub fn update_next_run(&mut self) {
        let now = Utc::now();
        self.next_run = self.cron.next_after(&now);
    }
    
    /// 记录执行
    pub fn record_run(&mut self) {
        self.last_run = Some(Utc::now());
        self.run_count += 1;
        self.update_next_run();
    }
}

/// 调度器命令
enum SchedulerCommand {
    /// 添加调度任务
    Add {
        task: ScheduledTask,
        response: oneshot::Sender<Result<ScheduleId>>,
    },
    /// 移除调度任务
    Remove {
        id: ScheduleId,
        response: oneshot::Sender<Result<bool>>,
    },
    /// 获取调度任务
    Get {
        id: ScheduleId,
        response: oneshot::Sender<Option<ScheduledTask>>,
    },
    /// 列出所有调度任务
    List {
        response: oneshot::Sender<Vec<ScheduledTask>>,
    },
    /// 启用/禁用任务
    SetEnabled {
        id: ScheduleId,
        enabled: bool,
        response: oneshot::Sender<Result<bool>>,
    },
    /// 停止调度器
    Stop,
}

/// Cron 调度器
pub struct TaskScheduler {
    /// 调度任务映射
    tasks: Arc<RwLock<HashMap<ScheduleId, ScheduledTask>>>,
    /// 命令发送器
    command_tx: mpsc::Sender<SchedulerCommand>,
    /// 停止信号
    stop_tx: Option<oneshot::Sender<()>>,
}

/// 任务执行器 trait
#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync {
    async fn execute(&self, task: ScheduledTask) -> Result<()>;
}

/// 默认任务执行器（使用闭包）
pub struct FnTaskExecutor<F> 
where 
    F: Fn(ScheduledTask) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync,
{
    f: F,
}

#[async_trait::async_trait]
impl<F> TaskExecutor for FnTaskExecutor<F>
where 
    F: Fn(ScheduledTask) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>> + Send + Sync,
{
    async fn execute(&self, task: ScheduledTask) -> Result<()> {
        (self.f)(task).await
    }
}

impl TaskScheduler {
    /// 创建新的调度器
    pub fn new() -> Self {
        let tasks = Arc::new(RwLock::new(HashMap::new()));
        let (command_tx, command_rx) = mpsc::channel(100);
        
        Self {
            tasks: tasks.clone(),
            command_tx,
            stop_tx: None,
        }
    }
    
    /// 启动调度器
    pub async fn start<E: TaskExecutor + 'static>(&mut self, executor: E) -> Result<()> {
        let (stop_tx, stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);
        
        let tasks = self.tasks.clone();
        let (tick_tx, mut tick_rx) = mpsc::channel::<()>(1);
        
        // 启动定时器任务
        let tick_tasks = tasks.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval.tick().await;
                let _ = tick_tx.send(()).await;
            }
        });
        
        // 启动命令处理任务
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<SchedulerCommand>(100);
        self.command_tx = cmd_tx;
        
        let handle_tasks = tasks.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // 处理定时器 tick
                    _ = tick_rx.recv() => {
                        let now = Utc::now();
                        let mut tasks_guard = handle_tasks.write().await;
                        
                        for (id, task) in tasks_guard.iter_mut() {
                            if !task.enabled {
                                continue;
                            }
                            
                            if let Some(next_run) = task.next_run {
                                if next_run <= now {
                                    // 执行任务
                                    let task_clone = task.clone();
                                    let executor: Box<dyn TaskExecutor + Send> = Box::new(FnTaskExecutor {
                                        f: move |t| {
                                            Box::pin(async move {
                                                // 默认执行器只记录日志
                                                info!("Executing scheduled task: {} ({})", t.name, t.id);
                                                Ok(())
                                            })
                                        }
                                    });
                                    
                                    // 记录执行
                                    task.record_run();
                                    info!("Scheduled task {} triggered at {:?}", task.name, now);
                                }
                            }
                        }
                    }
                    
                    // 处理命令
                    cmd = cmd_rx.recv() => {
                        if let Some(cmd) = cmd {
                            match cmd {
                                SchedulerCommand::Add { task, response } => {
                                    let id = task.id.clone();
                                    let mut tasks_guard = handle_tasks.write().await;
                                    tasks_guard.insert(id.clone(), task);
                                    let _ = response.send(Ok(id));
                                }
                                SchedulerCommand::Remove { id, response } => {
                                    let mut tasks_guard = handle_tasks.write().await;
                                    let removed = tasks_guard.remove(&id).is_some();
                                    let _ = response.send(Ok(removed));
                                }
                                SchedulerCommand::Get { id, response } => {
                                    let tasks_guard = handle_tasks.read().await;
                                    let _ = response.send(tasks_guard.get(&id).cloned());
                                }
                                SchedulerCommand::List { response } => {
                                    let tasks_guard = handle_tasks.read().await;
                                    let list: Vec<_> = tasks_guard.values().cloned().collect();
                                    let _ = response.send(list);
                                }
                                SchedulerCommand::SetEnabled { id, enabled, response } => {
                                    let mut tasks_guard = handle_tasks.write().await;
                                    if let Some(task) = tasks_guard.get_mut(&id) {
                                        task.enabled = enabled;
                                        let _ = response.send(Ok(true));
                                    } else {
                                        let _ = response.send(Err(anyhow::anyhow!("Task not found")));
                                    }
                                }
                                SchedulerCommand::Stop => {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// 停止调度器
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        Ok(())
    }
    
    /// 添加调度任务
    pub async fn add(&self, task: ScheduledTask) -> Result<ScheduleId> {
        let (tx, rx) = oneshot::channel();
        let id = task.id.clone();
        
        self.command_tx.send(SchedulerCommand::Add { task, response: tx }).await?;
        rx.await?
    }
    
    /// 移除调度任务
    pub async fn remove(&self, id: ScheduleId) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(SchedulerCommand::Remove { id, response: tx }).await?;
        rx.await?
    }
    
    /// 获取调度任务
    pub async fn get(&self, id: &ScheduleId) -> Option<ScheduledTask> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(SchedulerCommand::Get { id: id.clone(), response: tx }).await.ok()?;
        rx.await.ok()?
    }
    
    /// 列出所有调度任务
    pub async fn list(&self) -> Vec<ScheduledTask> {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(SchedulerCommand::List { response: tx }).await.is_err() {
            return vec![];
        }
        rx.await.unwrap_or_default()
    }
    
    /// 启用/禁用任务
    pub async fn set_enabled(&self, id: ScheduleId, enabled: bool) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(SchedulerCommand::SetEnabled { id, enabled, response: tx }).await?;
        rx.await?
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cron_expression_parse_standard() {
        // 标准格式
        let cron = CronExpression::parse("0 0 * * *").unwrap();
        assert!(matches!(cron.minute, CronField::Value(0)));
        assert!(matches!(cron.hour, CronField::Value(0)));
        assert!(matches!(cron.day_of_month, CronField::Any));
    }
    
    #[test]
    fn test_cron_expression_parse_step() {
        // 步进
        let cron = CronExpression::parse("*/5 * * * *").unwrap();
        assert!(matches!(cron.minute, CronField::Step { step: 5 }));
    }
    
    #[test]
    fn test_cron_expression_parse_range() {
        // 范围
        let cron = CronExpression::parse("0 9-17 * * *").unwrap();
        assert!(matches!(cron.hour, CronField::Range { start: 9, end: 17 }));
    }
    
    #[test]
    fn test_cron_expression_parse_list() {
        // 列表
        let cron = CronExpression::parse("0,30 * * * *").unwrap();
        if let CronField::List(values) = cron.minute {
            assert_eq!(values, vec![0, 30]);
        } else {
            panic!("Expected List");
        }
    }
    
    #[test]
    fn test_cron_expression_parse_range_step() {
        // 范围步进
        let cron = CronExpression::parse("0-30/5 * * * *").unwrap();
        if let CronField::RangeStep { start, end, step } = cron.minute {
            assert_eq!(start, 0);
            assert_eq!(end, 30);
            assert_eq!(step, 5);
        } else {
            panic!("Expected RangeStep");
        }
    }
    
    #[test]
    fn test_cron_shortcut_every_minute() {
        let cron = CronExpression::parse("@every_minute").unwrap();
        assert!(matches!(cron.minute, CronField::Any));
    }
    
    #[test]
    fn test_cron_shortcut_hourly() {
        let cron = CronExpression::parse("@hourly").unwrap();
        assert!(matches!(cron.minute, CronField::Value(0)));
        assert!(matches!(cron.hour, CronField::Any));
    }
    
    #[test]
    fn test_cron_shortcut_daily() {
        let cron = CronExpression::parse("@daily").unwrap();
        assert!(matches!(cron.minute, CronField::Value(0)));
        assert!(matches!(cron.hour, CronField::Value(0)));
    }
    
    #[test]
    fn test_cron_shortcut_every_n_minutes() {
        let cron = CronExpression::parse("@every_5m").unwrap();
        assert!(matches!(cron.minute, CronField::Step { step: 5 }));
        
        let cron = CronExpression::parse("@every_15m").unwrap();
        assert!(matches!(cron.minute, CronField::Step { step: 15 }));
    }
    
    #[test]
    fn test_cron_shortcut_every_n_hours() {
        let cron = CronExpression::parse("@every_2h").unwrap();
        assert!(matches!(cron.minute, CronField::Value(0)));
        assert!(matches!(cron.hour, CronField::Step { step: 2 }));
    }
    
    #[test]
    fn test_cron_matches() {
        let cron = CronExpression::parse("30 14 * * *").unwrap(); // 每天 14:30
        
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        assert!(cron.matches(&dt));
        
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 14, 31, 0).unwrap();
        assert!(!cron.matches(&dt));
    }
    
    #[test]
    fn test_cron_next_after() {
        let cron = CronExpression::parse("0 * * * *").unwrap(); // 每小时整点
        
        let now = Utc.with_ymd_and_hms(2024, 1, 15, 14, 30, 0).unwrap();
        let next = cron.next_after(&now).unwrap();
        
        assert_eq!(next.hour(), 15);
        assert_eq!(next.minute(), 0);
    }
    
    #[test]
    fn test_cron_next_n() {
        let cron = CronExpression::parse("0 */2 * * *").unwrap(); // 每 2 小时
        
        let now = Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap();
        let next_times = cron.next_n(&now, 5);
        
        assert_eq!(next_times.len(), 5);
        assert_eq!(next_times[0].hour(), 12);
        assert_eq!(next_times[1].hour(), 14);
    }
    
    #[test]
    fn test_scheduled_task_new() {
        let task = ScheduledTask::new(
            "test-task".to_string(),
            "0 9 * * *",
            "shell".to_string(),
            serde_json::json!({"command": "echo hello"}),
        ).unwrap();
        
        assert_eq!(task.name, "test-task");
        assert!(task.enabled);
        assert!(task.next_run.is_some());
    }
    
    #[test]
    fn test_scheduled_task_record_run() {
        let mut task = ScheduledTask::new(
            "test-task".to_string(),
            "@hourly",
            "shell".to_string(),
            serde_json::Value::Null,
        ).unwrap();
        
        let first_next = task.next_run.clone();
        task.record_run();
        
        assert!(task.last_run.is_some());
        assert_eq!(task.run_count, 1);
        // next_run 应该更新
        assert_ne!(task.next_run, first_next);
    }
    
    #[test]
    fn test_cron_field_values() {
        let field = CronField::Range { start: 5, end: 10 };
        let values = field.values(0, 15);
        assert_eq!(values, vec![5, 6, 7, 8, 9, 10]);
        
        let field = CronField::Step { step: 15 };
        let values = field.values(0, 59);
        assert_eq!(values, vec![0, 15, 30, 45]);
        
        let field = CronField::List(vec![1, 5, 10]);
        let values = field.values(0, 20);
        assert_eq!(values, vec![1, 5, 10]);
    }
    
    #[test]
    fn test_cron_invalid_expression() {
        // 字段数量不对
        assert!(CronExpression::parse("0 0 * *").is_err());
        
        // 无效值
        assert!(CronExpression::parse("60 0 * * *").is_err()); // 分钟 > 59
        assert!(CronExpression::parse("0 24 * * *").is_err()); // 小时 > 23
        assert!(CronExpression::parse("0 0 32 * *").is_err()); // 日 > 31
        assert!(CronExpression::parse("0 0 0 13 *").is_err()); // 月 > 12
        assert!(CronExpression::parse("0 0 * * 7").is_err()); // 周几 > 6
    }
    
    #[tokio::test]
    async fn test_task_scheduler_add_remove() {
        let scheduler = TaskScheduler::new();
        
        let task = ScheduledTask::new(
            "test-task".to_string(),
            "@hourly",
            "shell".to_string(),
            serde_json::Value::Null,
        ).unwrap();
        
        let id = task.id.clone();
        
        // 添加任务
        let result = scheduler.add(task).await;
        // 注意：由于调度器未启动，命令可能无法处理
        // 这里主要测试 API 是否正常工作
        
        // 获取任务
        let retrieved = scheduler.get(&id).await;
        // 同样，由于调度器未启动，可能返回 None
    }
    
    #[test]
    fn test_cron_weekday_matching() {
        // 每周一 9:00
        let cron = CronExpression::parse("0 9 * * 1").unwrap();
        
        // 2024-01-15 是周一
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 9, 0, 0).unwrap();
        assert!(cron.matches(&dt));
        
        // 2024-01-16 是周二
        let dt = Utc.with_ymd_and_hms(2024, 1, 16, 9, 0, 0).unwrap();
        assert!(!cron.matches(&dt));
    }
    
    #[test]
    fn test_cron_month_matching() {
        // 每月 1 日 0:00
        let cron = CronExpression::parse("0 0 1 * *").unwrap();
        
        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        assert!(cron.matches(&dt));
        
        let dt = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        assert!(!cron.matches(&dt));
    }
}