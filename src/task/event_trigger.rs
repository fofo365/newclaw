//! Event Trigger - 事件触发器
//!
//! v0.7.0 - 事件驱动任务执行

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::sync::{RwLock, mpsc, oneshot, broadcast};
use anyhow::{Result, Context};
use tracing::{info, warn, debug, error};
use uuid::Uuid;
use regex::Regex;

/// 事件 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 触发器 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(String);

impl TriggerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TriggerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TriggerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EventType(pub String);

impl EventType {
    pub fn new(s: &str) -> Self {
        Self(s.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// 事件 ID
    pub id: EventId,
    /// 事件类型
    pub event_type: EventType,
    /// 事件源
    pub source: String,
    /// 事件数据
    pub data: serde_json::Value,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Event {
    /// 创建新事件
    pub fn new(event_type: EventType, source: String, data: serde_json::Value) -> Self {
        Self {
            id: EventId::new(),
            event_type,
            source,
            data,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// 事件模式匹配规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventPattern {
    /// 精确匹配事件类型
    Exact(String),
    /// 前缀匹配
    Prefix(String),
    /// 后缀匹配
    Suffix(String),
    /// 通配符匹配（支持 * 和 ?）
    Wildcard(String),
    /// 正则表达式匹配
    Regex(String),
    /// 组合匹配（所有条件都满足）
    All(Vec<EventPattern>),
    /// 任一匹配（任一条件满足）
    Any(Vec<EventPattern>),
}

impl EventPattern {
    /// 检查事件是否匹配
    pub fn matches(&self, event_type: &str) -> bool {
        match self {
            EventPattern::Exact(s) => event_type == s,
            EventPattern::Prefix(s) => event_type.starts_with(s),
            EventPattern::Suffix(s) => event_type.ends_with(s),
            EventPattern::Wildcard(pattern) => {
                Self::wildcard_match(pattern, event_type)
            }
            EventPattern::Regex(pattern) => {
                match Regex::new(pattern) {
                    Ok(re) => re.is_match(event_type),
                    Err(_) => false,
                }
            }
            EventPattern::All(patterns) => {
                patterns.iter().all(|p| p.matches(event_type))
            }
            EventPattern::Any(patterns) => {
                patterns.iter().any(|p| p.matches(event_type))
            }
        }
    }
    
    /// 通配符匹配
    fn wildcard_match(pattern: &str, text: &str) -> bool {
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();
        
        Self::wildcard_match_recursive(&pattern_chars, &text_chars, 0, 0)
    }
    
    fn wildcard_match_recursive(pattern: &[char], text: &[char], p_idx: usize, t_idx: usize) -> bool {
        if p_idx == pattern.len() && t_idx == text.len() {
            return true;
        }
        
        if p_idx == pattern.len() {
            return false;
        }
        
        match pattern[p_idx] {
            '*' => {
                // * 匹配任意数量的字符
                for i in t_idx..=text.len() {
                    if Self::wildcard_match_recursive(pattern, text, p_idx + 1, i) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                // ? 匹配单个字符
                if t_idx < text.len() {
                    Self::wildcard_match_recursive(pattern, text, p_idx + 1, t_idx + 1)
                } else {
                    false
                }
            }
            c => {
                if t_idx < text.len() && text[t_idx] == c {
                    Self::wildcard_match_recursive(pattern, text, p_idx + 1, t_idx + 1)
                } else {
                    false
                }
            }
        }
    }
}

/// 事件触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTrigger {
    /// 触发器 ID
    pub id: TriggerId,
    /// 触发器名称
    pub name: String,
    /// 事件模式
    pub pattern: EventPattern,
    /// 触发条件（可选的 JavaScript 表达式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// 任务类型
    pub task_type: String,
    /// 任务参数模板
    pub params_template: serde_json::Value,
    /// 是否启用
    pub enabled: bool,
    /// 触发次数
    pub trigger_count: u64,
    /// 上次触发时间
    pub last_triggered: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 传播规则
    pub propagation: PropagationRule,
    /// 优先级
    pub priority: i32,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl EventTrigger {
    /// 创建新触发器
    pub fn new(name: String, pattern: EventPattern, task_type: String, params_template: serde_json::Value) -> Self {
        Self {
            id: TriggerId::new(),
            name,
            pattern,
            condition: None,
            task_type,
            params_template,
            enabled: true,
            trigger_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
            propagation: PropagationRule::default(),
            priority: 0,
            metadata: HashMap::new(),
        }
    }
    
    /// 设置触发条件
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = Some(condition);
        self
    }
    
    /// 设置传播规则
    pub fn with_propagation(mut self, propagation: PropagationRule) -> Self {
        self.propagation = propagation;
        self
    }
    
    /// 设置优先级
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 检查事件是否匹配
    pub fn matches(&self, event: &Event) -> bool {
        if !self.enabled {
            return false;
        }
        
        if !self.pattern.matches(&event.event_type.0) {
            return false;
        }
        
        // 检查条件
        if let Some(condition) = &self.condition {
            // 简单的条件检查：检查事件数据中是否有指定字段
            // 实际实现可以使用表达式引擎
            self.evaluate_condition(condition, &event.data)
        } else {
            true
        }
    }
    
    /// 评估条件
    fn evaluate_condition(&self, condition: &str, data: &serde_json::Value) -> bool {
        // 简单的条件解析
        // 格式: field == value, field != value, field > value, etc.
        let condition = condition.trim();
        
        // 支持简单的 JSON 路径检查
        if condition.starts_with("data.") {
            let path = &condition[5..];
            if let Some(value) = path.strip_prefix("has(") {
                let field = value.trim_end_matches(')').trim();
                return data.get(field).is_some();
            }
        }
        
        // 默认返回 true
        true
    }
    
    /// 生成任务参数
    pub fn generate_params(&self, event: &Event) -> serde_json::Value {
        // 替换模板中的变量
        let mut params = self.params_template.clone();
        Self::substitute_variables(&mut params, event);
        params
    }
    
    /// 替换变量
    fn substitute_variables(value: &mut serde_json::Value, event: &Event) {
        match value {
            serde_json::Value::String(s) => {
                // 替换 ${event.type}, ${event.source}, ${event.data.field} 等
                let mut result = s.clone();
                result = result.replace("${event.type}", &event.event_type.0);
                result = result.replace("${event.source}", &event.source);
                result = result.replace("${event.id}", event.id.as_str());
                
                // 替换 ${event.data.xxx}
                if let serde_json::Value::Object(ref data) = event.data {
                    for (k, v) in data {
                        if let serde_json::Value::String(v_str) = v {
                            result = result.replace(&format!("${{event.data.{}}}", k), v_str);
                        } else {
                            result = result.replace(&format!("${{event.data.{}}}", k), &v.to_string());
                        }
                    }
                }
                
                *s = result;
            }
            serde_json::Value::Object(map) => {
                for v in map.values_mut() {
                    Self::substitute_variables(v, event);
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    Self::substitute_variables(v, event);
                }
            }
            _ => {}
        }
    }
    
    /// 记录触发
    pub fn record_trigger(&mut self) {
        self.trigger_count += 1;
        self.last_triggered = Some(Utc::now());
    }
}

/// 事件传播规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropagationRule {
    /// 继续传播给其他触发器
    Continue,
    /// 停止传播
    Stop,
    /// 仅传播给同级触发器
    SiblingOnly,
}

impl Default for PropagationRule {
    fn default() -> Self {
        Self::Continue
    }
}

/// 触发器命令
enum TriggerCommand {
    /// 注册触发器
    Register {
        trigger: EventTrigger,
        response: oneshot::Sender<Result<TriggerId>>,
    },
    /// 注销触发器
    Unregister {
        id: TriggerId,
        response: oneshot::Sender<Result<bool>>,
    },
    /// 获取触发器
    Get {
        id: TriggerId,
        response: oneshot::Sender<Option<EventTrigger>>,
    },
    /// 列出所有触发器
    List {
        response: oneshot::Sender<Vec<EventTrigger>>,
    },
    /// 启用/禁用触发器
    SetEnabled {
        id: TriggerId,
        enabled: bool,
        response: oneshot::Sender<Result<bool>>,
    },
    /// 停止
    Stop,
}

/// 事件触发器统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTriggerStats {
    /// 总事件数
    pub total_events: u64,
    /// 匹配事件数
    pub matched_events: u64,
    /// 触发次数
    pub total_triggers: u64,
    /// 触发器数量
    pub trigger_count: usize,
}

impl Default for EventTriggerStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            matched_events: 0,
            total_triggers: 0,
            trigger_count: 0,
        }
    }
}

/// 事件触发器管理器
pub struct EventTriggerManager {
    /// 触发器映射
    triggers: Arc<RwLock<HashMap<TriggerId, EventTrigger>>>,
    /// 命令发送器
    command_tx: mpsc::Sender<TriggerCommand>,
    /// 事件广播发送器
    event_tx: broadcast::Sender<Event>,
    /// 统计信息
    stats: Arc<RwLock<EventTriggerStats>>,
}

impl EventTriggerManager {
    /// 创建新的触发器管理器
    pub fn new() -> Self {
        let triggers = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(EventTriggerStats::default()));
        let (event_tx, _) = broadcast::channel(1000);
        let (command_tx, _command_rx) = mpsc::channel(100);
        
        Self {
            triggers,
            command_tx,
            event_tx,
            stats,
        }
    }
    
    /// 启动触发器管理器
    pub async fn start<F, Fut>(&mut self, executor: F) -> Result<()>
    where
        F: Fn(String, serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let triggers = self.triggers.clone();
        let stats = self.stats.clone();
        let (cmd_tx, mut cmd_rx) = mpsc::channel(100);
        self.command_tx = cmd_tx;
        
        let executor = Arc::new(executor);
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    cmd = cmd_rx.recv() => {
                        if let Some(cmd) = cmd {
                            match cmd {
                                TriggerCommand::Register { trigger, response } => {
                                    let id = trigger.id.clone();
                                    let mut triggers_guard = triggers.write().await;
                                    triggers_guard.insert(id.clone(), trigger);
                                    stats.write().await.trigger_count = triggers_guard.len();
                                    let _ = response.send(Ok(id));
                                }
                                TriggerCommand::Unregister { id, response } => {
                                    let mut triggers_guard = triggers.write().await;
                                    let removed = triggers_guard.remove(&id).is_some();
                                    stats.write().await.trigger_count = triggers_guard.len();
                                    let _ = response.send(Ok(removed));
                                }
                                TriggerCommand::Get { id, response } => {
                                    let triggers_guard = triggers.read().await;
                                    let _ = response.send(triggers_guard.get(&id).cloned());
                                }
                                TriggerCommand::List { response } => {
                                    let triggers_guard = triggers.read().await;
                                    let list: Vec<_> = triggers_guard.values().cloned().collect();
                                    let _ = response.send(list);
                                }
                                TriggerCommand::SetEnabled { id, enabled, response } => {
                                    let mut triggers_guard = triggers.write().await;
                                    if let Some(trigger) = triggers_guard.get_mut(&id) {
                                        trigger.enabled = enabled;
                                        let _ = response.send(Ok(true));
                                    } else {
                                        let _ = response.send(Err(anyhow::anyhow!("Trigger not found")));
                                    }
                                }
                                TriggerCommand::Stop => {
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
    
    /// 停止管理器
    pub async fn stop(&mut self) -> Result<()> {
        let _ = self.command_tx.send(TriggerCommand::Stop).await;
        Ok(())
    }
    
    /// 发送事件
    pub async fn emit(&self, event: Event) -> Result<Vec<TriggerId>> {
        let mut stats_guard = self.stats.write().await;
        stats_guard.total_events += 1;
        drop(stats_guard);
        
        // 广播事件
        let _ = self.event_tx.send(event.clone());
        
        // 匹配触发器
        let triggers_guard = self.triggers.read().await;
        let mut matched: Vec<(i32, &EventTrigger)> = triggers_guard
            .values()
            .filter(|t| t.matches(&event))
            .map(|t| (t.priority, t))
            .collect();
        
        // 按优先级排序（优先级高的先执行）
        matched.sort_by(|a, b| b.0.cmp(&a.0));
        
        let matched_ids: Vec<TriggerId> = matched.iter().map(|(_, t)| t.id.clone()).collect();
        
        if !matched.is_empty() {
            let mut stats_guard = self.stats.write().await;
            stats_guard.matched_events += 1;
            stats_guard.total_triggers += matched.len() as u64;
        }
        
        Ok(matched_ids)
    }
    
    /// 注册触发器
    pub async fn register(&self, trigger: EventTrigger) -> Result<TriggerId> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(TriggerCommand::Register { trigger, response: tx }).await?;
        rx.await?
    }
    
    /// 注销触发器
    pub async fn unregister(&self, id: TriggerId) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(TriggerCommand::Unregister { id, response: tx }).await?;
        rx.await?
    }
    
    /// 获取触发器
    pub async fn get(&self, id: &TriggerId) -> Option<EventTrigger> {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(TriggerCommand::Get { id: id.clone(), response: tx }).await.is_err() {
            return None;
        }
        rx.await.ok()?
    }
    
    /// 列出所有触发器
    pub async fn list(&self) -> Vec<EventTrigger> {
        let (tx, rx) = oneshot::channel();
        
        if self.command_tx.send(TriggerCommand::List { response: tx }).await.is_err() {
            return vec![];
        }
        rx.await.unwrap_or_default()
    }
    
    /// 启用/禁用触发器
    pub async fn set_enabled(&self, id: TriggerId, enabled: bool) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        
        self.command_tx.send(TriggerCommand::SetEnabled { id, enabled, response: tx }).await?;
        rx.await?
    }
    
    /// 获取统计信息
    pub async fn stats(&self) -> EventTriggerStats {
        self.stats.read().await.clone()
    }
    
    /// 订阅事件流
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.event_tx.subscribe()
    }
}

impl Default for EventTriggerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_event_new() {
        let event = Event::new(
            EventType::new("user.created"),
            "auth-service".to_string(),
            serde_json::json!({"user_id": "123"}),
        );
        
        assert_eq!(event.event_type.as_str(), "user.created");
        assert_eq!(event.source, "auth-service");
    }
    
    #[test]
    fn test_event_pattern_exact() {
        let pattern = EventPattern::Exact("user.created".to_string());
        
        assert!(pattern.matches("user.created"));
        assert!(!pattern.matches("user.updated"));
        assert!(!pattern.matches("user.created.v2"));
    }
    
    #[test]
    fn test_event_pattern_prefix() {
        let pattern = EventPattern::Prefix("user.".to_string());
        
        assert!(pattern.matches("user.created"));
        assert!(pattern.matches("user.updated"));
        assert!(pattern.matches("user.deleted"));
        assert!(!pattern.matches("order.created"));
    }
    
    #[test]
    fn test_event_pattern_suffix() {
        let pattern = EventPattern::Suffix(".created".to_string());
        
        assert!(pattern.matches("user.created"));
        assert!(pattern.matches("order.created"));
        assert!(!pattern.matches("user.updated"));
    }
    
    #[test]
    fn test_event_pattern_wildcard() {
        let pattern = EventPattern::Wildcard("user.*".to_string());
        
        assert!(pattern.matches("user.created"));
        assert!(pattern.matches("user.updated"));
        assert!(!pattern.matches("order.created"));
        
        let pattern = EventPattern::Wildcard("*.created".to_string());
        
        assert!(pattern.matches("user.created"));
        assert!(pattern.matches("order.created"));
        assert!(!pattern.matches("user.updated"));
        
        let pattern = EventPattern::Wildcard("user.???".to_string());
        
        assert!(pattern.matches("user.new"));
        assert!(!pattern.matches("user.created")); // 太长
    }
    
    #[test]
    fn test_event_pattern_regex() {
        let pattern = EventPattern::Regex(r"user\.(created|updated)".to_string());
        
        assert!(pattern.matches("user.created"));
        assert!(pattern.matches("user.updated"));
        assert!(!pattern.matches("user.deleted"));
        assert!(!pattern.matches("order.created"));
    }
    
    #[test]
    fn test_event_pattern_all() {
        let pattern = EventPattern::All(vec![
            EventPattern::Prefix("user.".to_string()),
            EventPattern::Suffix(".created".to_string()),
        ]);
        
        assert!(pattern.matches("user.created"));
        assert!(!pattern.matches("user.updated")); // 不满足后缀
        assert!(!pattern.matches("order.created")); // 不满足前缀
    }
    
    #[test]
    fn test_event_pattern_any() {
        let pattern = EventPattern::Any(vec![
            EventPattern::Exact("user.created".to_string()),
            EventPattern::Exact("user.deleted".to_string()),
        ]);
        
        assert!(pattern.matches("user.created"));
        assert!(pattern.matches("user.deleted"));
        assert!(!pattern.matches("user.updated"));
    }
    
    #[test]
    fn test_event_trigger_new() {
        let trigger = EventTrigger::new(
            "user-created-trigger".to_string(),
            EventPattern::Exact("user.created".to_string()),
            "send-welcome-email".to_string(),
            serde_json::json!({"user_id": "${event.data.user_id}"}),
        );
        
        assert_eq!(trigger.name, "user-created-trigger");
        assert!(trigger.enabled);
    }
    
    #[test]
    fn test_event_trigger_matches() {
        let trigger = EventTrigger::new(
            "test".to_string(),
            EventPattern::Prefix("user.".to_string()),
            "handler".to_string(),
            serde_json::Value::Null,
        );
        
        let event = Event::new(
            EventType::new("user.created"),
            "test".to_string(),
            serde_json::Value::Null,
        );
        
        assert!(trigger.matches(&event));
        
        let event = Event::new(
            EventType::new("order.created"),
            "test".to_string(),
            serde_json::Value::Null,
        );
        
        assert!(!trigger.matches(&event));
    }
    
    #[test]
    fn test_event_trigger_disabled() {
        let mut trigger = EventTrigger::new(
            "test".to_string(),
            EventPattern::Any(vec![]), // 匹配所有
            "handler".to_string(),
            serde_json::Value::Null,
        );
        
        trigger.enabled = false;
        
        let event = Event::new(
            EventType::new("any.event"),
            "test".to_string(),
            serde_json::Value::Null,
        );
        
        assert!(!trigger.matches(&event));
    }
    
    #[test]
    fn test_event_trigger_generate_params() {
        let trigger = EventTrigger::new(
            "test".to_string(),
            EventPattern::Any(vec![]),
            "handler".to_string(),
            serde_json::json!({
                "event_type": "${event.type}",
                "source": "${event.source}",
                "user_id": "${event.data.user_id}"
            }),
        );
        
        let event = Event::new(
            EventType::new("user.created"),
            "auth".to_string(),
            serde_json::json!({"user_id": "123", "name": "John"}),
        );
        
        let params = trigger.generate_params(&event);
        
        assert_eq!(params["event_type"], "user.created");
        assert_eq!(params["source"], "auth");
        assert_eq!(params["user_id"], "123");
    }
    
    #[test]
    fn test_propagation_rule_default() {
        let rule = PropagationRule::default();
        assert!(matches!(rule, PropagationRule::Continue));
    }
    
    #[tokio::test]
    async fn test_event_trigger_manager_register() {
        let mut manager = EventTriggerManager::new();
        manager.start(|task_type, params| async move {
            Ok(())
        }).await.unwrap();
        
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        let trigger = EventTrigger::new(
            "test-trigger".to_string(),
            EventPattern::Prefix("test.".to_string()),
            "handler".to_string(),
            serde_json::Value::Null,
        );
        
        let id = manager.register(trigger).await.unwrap();
        
        let retrieved = manager.get(&id).await;
        assert!(retrieved.is_some());
        
        manager.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_event_trigger_manager_emit() {
        let mut manager = EventTriggerManager::new();
        manager.start(|task_type, params| async move {
            Ok(())
        }).await.unwrap();
        
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        // 注册触发器
        let trigger = EventTrigger::new(
            "test-trigger".to_string(),
            EventPattern::Exact("test.event".to_string()),
            "handler".to_string(),
            serde_json::Value::Null,
        );
        
        manager.register(trigger).await.unwrap();
        
        // 发送匹配事件
        let event = Event::new(
            EventType::new("test.event"),
            "test".to_string(),
            serde_json::Value::Null,
        );
        
        let matched = manager.emit(event).await.unwrap();
        assert_eq!(matched.len(), 1);
        
        // 发送不匹配事件
        let event = Event::new(
            EventType::new("other.event"),
            "test".to_string(),
            serde_json::Value::Null,
        );
        
        let matched = manager.emit(event).await.unwrap();
        assert!(matched.is_empty());
        
        manager.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_event_trigger_manager_stats() {
        let mut manager = EventTriggerManager::new();
        manager.start(|task_type, params| async move {
            Ok(())
        }).await.unwrap();
        
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        
        // 注册多个触发器
        for i in 0..3 {
            let trigger = EventTrigger::new(
                format!("trigger-{}", i),
                EventPattern::Prefix("test.".to_string()),
                "handler".to_string(),
                serde_json::Value::Null,
            );
            manager.register(trigger).await.unwrap();
        }
        
        // 发送事件
        for i in 0..5 {
            let event = Event::new(
                EventType::new(&format!("test.event-{}", i)),
                "test".to_string(),
                serde_json::Value::Null,
            );
            manager.emit(event).await.unwrap();
        }
        
        let stats = manager.stats().await;
        assert_eq!(stats.total_events, 5);
        assert_eq!(stats.matched_events, 5);
        assert_eq!(stats.total_triggers, 15); // 5 events * 3 triggers
        
        manager.stop().await.unwrap();
    }
    
    #[test]
    fn test_wildcard_match_edge_cases() {
        // 空 pattern
        assert!(EventPattern::Wildcard("".to_string()).matches(""));
        assert!(!EventPattern::Wildcard("".to_string()).matches("a"));
        
        // 单个 *
        assert!(EventPattern::Wildcard("*".to_string()).matches(""));
        assert!(EventPattern::Wildcard("*".to_string()).matches("anything"));
        
        // 多个 *
        assert!(EventPattern::Wildcard("a*b*c".to_string()).matches("a123b456c"));
        assert!(!EventPattern::Wildcard("a*b*c".to_string()).matches("abc"));
    }
}