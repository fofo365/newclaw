//! Alert system for audit events
//!
//! This module provides:
//! - Alert rule definition
//! - Alert trigger evaluation
//! - Alert notification integration

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use regex::Regex;

use super::{AuditEntry, AuditEvent, AuditStore, AuditResult, AuditError};

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

impl Default for AlertSeverity {
    fn default() -> Self {
        Self::Warning
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Alert condition operator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertCondition {
    /// Count exceeds threshold in time window
    CountExceeds {
        /// Event type to count
        event_type: String,
        /// Time window in minutes
        window_minutes: u32,
        /// Threshold count
        threshold: u32,
    },
    /// Deny rate exceeds percentage
    DenyRateExceeds {
        /// Time window in minutes
        window_minutes: u32,
        /// Threshold percentage (0-100)
        threshold_percent: f64,
    },
    /// Task failure rate exceeds percentage
    TaskFailureRateExceeds {
        /// Task type filter (optional)
        task_type: Option<String>,
        /// Time window in minutes
        window_minutes: u32,
        /// Threshold percentage (0-100)
        threshold_percent: f64,
    },
    /// Specific event occurs
    EventMatches {
        /// Event type
        event_type: String,
        /// Field to match
        field: String,
        /// Match value (supports regex with * wildcard)
        value: String,
    },
    /// No activity for period
    NoActivity {
        /// Event type (optional, any if None)
        event_type: Option<String>,
        /// Inactivity period in minutes
        period_minutes: u32,
    },
    /// Pattern detection
    PatternDetected {
        /// Pattern name
        name: String,
        /// Pattern description
        description: String,
        /// Event sequence to match
        sequence: Vec<PatternStep>,
    },
    /// Composite condition (AND/OR)
    And(Vec<AlertCondition>),
    Or(Vec<AlertCondition>),
}

/// Pattern step for sequence detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStep {
    /// Event type
    pub event_type: String,
    /// Field filters
    pub filters: HashMap<String, String>,
    /// Maximum time to next step (minutes, optional)
    pub max_delay_minutes: Option<u32>,
}

/// Alert action to take when triggered
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertAction {
    /// Log to console
    Log,
    /// Send webhook
    Webhook {
        /// Webhook URL
        url: String,
        /// HTTP method
        method: String,
        /// Custom headers
        headers: HashMap<String, String>,
        /// Request body template (supports {{placeholders}})
        body_template: Option<String>,
    },
    /// Send email
    Email {
        /// Recipient addresses
        to: Vec<String>,
        /// Subject template
        subject: String,
        /// Body template
        body: String,
    },
    /// Send to channel (QQ/Feishu/etc.)
    Channel {
        /// Channel type
        channel_type: String,
        /// Channel ID
        channel_id: String,
        /// Message template
        message: String,
    },
    /// Execute command
    Command {
        /// Command to execute
        command: String,
        /// Arguments
        args: Vec<String>,
    },
    /// Custom action
    Custom {
        /// Action name
        name: String,
        /// Action parameters
        params: HashMap<String, String>,
    },
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule ID
    pub id: Uuid,
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Severity level
    pub severity: AlertSeverity,
    /// Whether rule is enabled
    pub enabled: bool,
    /// Alert condition
    pub condition: AlertCondition,
    /// Actions to take
    pub actions: Vec<AlertAction>,
    /// Cooldown period in minutes (prevent duplicate alerts)
    pub cooldown_minutes: u32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
}

impl AlertRule {
    /// Create a new alert rule
    pub fn new(name: impl Into<String>, condition: AlertCondition) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            severity: AlertSeverity::default(),
            enabled: true,
            condition,
            actions: Vec::new(),
            cooldown_minutes: 5,
            tags: Vec::new(),
            created_at: now,
            modified_at: now,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set severity
    pub fn with_severity(mut self, severity: AlertSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Add action
    pub fn with_action(mut self, action: AlertAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Set cooldown
    pub fn with_cooldown(mut self, minutes: u32) -> Self {
        self.cooldown_minutes = minutes;
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set enabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.modified_at = Utc::now();
    }
}

/// Triggered alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredAlert {
    /// Alert instance ID
    pub id: Uuid,
    /// Rule that triggered
    pub rule_id: Uuid,
    /// Rule name
    pub rule_name: String,
    /// Severity
    pub severity: AlertSeverity,
    /// Trigger timestamp
    pub triggered_at: DateTime<Utc>,
    /// Context data
    pub context: HashMap<String, serde_json::Value>,
    /// Related entries (sample)
    pub related_entries: Vec<Uuid>,
    /// Message
    pub message: String,
}

/// Alert manager
pub struct AlertManager {
    /// Alert rules
    rules: Arc<RwLock<Vec<AlertRule>>>,
    /// Triggered alerts history
    triggered: Arc<RwLock<Vec<TriggeredAlert>>>,
    /// Last trigger time per rule (for cooldown)
    last_trigger: Arc<RwLock<HashMap<Uuid, DateTime<Utc>>>>,
    /// Maximum triggered alerts to keep
    max_triggered: usize,
    /// Notification handler
    notification_handler: Option<Box<dyn NotificationHandler + Send + Sync>>,
}

/// Notification handler trait
pub trait NotificationHandler {
    /// Handle a triggered alert
    fn handle(&self, alert: &TriggeredAlert, actions: &[AlertAction]) -> AuditResult<()>;
}

/// Default notification handler (logs to console)
struct DefaultNotificationHandler;

impl NotificationHandler for DefaultNotificationHandler {
    fn handle(&self, alert: &TriggeredAlert, _actions: &[AlertAction]) -> AuditResult<()> {
        log::warn!(
            "[ALERT {}] {}: {}",
            alert.severity,
            alert.rule_name,
            alert.message
        );
        Ok(())
    }
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            triggered: Arc::new(RwLock::new(Vec::new())),
            last_trigger: Arc::new(RwLock::new(HashMap::new())),
            max_triggered: 1000,
            notification_handler: Some(Box::new(DefaultNotificationHandler)),
        }
    }

    /// Create with custom notification handler
    pub fn with_handler(handler: Box<dyn NotificationHandler + Send + Sync>) -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            triggered: Arc::new(RwLock::new(Vec::new())),
            last_trigger: Arc::new(RwLock::new(HashMap::new())),
            max_triggered: 1000,
            notification_handler: Some(handler),
        }
    }

    /// Add a rule
    pub fn add_rule(&self, rule: AlertRule) {
        self.rules.write().push(rule);
    }

    /// Remove a rule
    pub fn remove_rule(&self, rule_id: &Uuid) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == *rule_id) {
            rules.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get all rules
    pub fn rules(&self) -> Vec<AlertRule> {
        self.rules.read().clone()
    }

    /// Get rule by ID
    pub fn get_rule(&self, rule_id: &Uuid) -> Option<AlertRule> {
        self.rules.read().iter().find(|r| r.id == *rule_id).cloned()
    }

    /// Update a rule
    pub fn update_rule(&self, rule: AlertRule) -> bool {
        let mut rules = self.rules.write();
        if let Some(pos) = rules.iter().position(|r| r.id == rule.id) {
            rules[pos] = rule;
            true
        } else {
            false
        }
    }

    /// Evaluate all rules against the store
    pub fn evaluate(&self, store: &AuditStore) -> AuditResult<Vec<TriggeredAlert>> {
        let rules = self.rules.read();
        let mut triggered_alerts = Vec::new();

        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }

            // Check cooldown
            if self.in_cooldown(&rule) {
                continue;
            }

            // Evaluate condition
            if let Some(alert) = self.evaluate_rule(rule, store)? {
                // Record trigger
                self.last_trigger.write().insert(rule.id, Utc::now());

                // Add to history
                self.add_triggered(alert.clone());

                // Execute actions
                if let Some(ref handler) = self.notification_handler {
                    handler.handle(&alert, &rule.actions)?;
                }

                triggered_alerts.push(alert);
            }
        }

        Ok(triggered_alerts)
    }

    /// Check if rule is in cooldown
    fn in_cooldown(&self, rule: &AlertRule) -> bool {
        let last = self.last_trigger.read();
        if let Some(&last_time) = last.get(&rule.id) {
            let cooldown = Duration::minutes(rule.cooldown_minutes as i64);
            Utc::now() - last_time < cooldown
        } else {
            false
        }
    }

    /// Evaluate a single rule
    fn evaluate_rule(&self, rule: &AlertRule, store: &AuditStore) -> AuditResult<Option<TriggeredAlert>> {
        let result = self.evaluate_condition(&rule.condition, store)?;

        if result.triggered {
            Ok(Some(TriggeredAlert {
                id: Uuid::new_v4(),
                rule_id: rule.id,
                rule_name: rule.name.clone(),
                severity: rule.severity,
                triggered_at: Utc::now(),
                context: result.context,
                related_entries: result.related_entries,
                message: result.message.unwrap_or_else(|| format!("Alert '{}' triggered", rule.name)),
            }))
        } else {
            Ok(None)
        }
    }

    /// Evaluate a condition
    fn evaluate_condition(&self, condition: &AlertCondition, store: &AuditStore) -> AuditResult<ConditionResult> {
        match condition {
            AlertCondition::CountExceeds { event_type, window_minutes, threshold } => {
                self.eval_count_exceeds(store, event_type, *window_minutes, *threshold)
            }
            AlertCondition::DenyRateExceeds { window_minutes, threshold_percent } => {
                self.eval_deny_rate(store, *window_minutes, *threshold_percent)
            }
            AlertCondition::TaskFailureRateExceeds { task_type, window_minutes, threshold_percent } => {
                self.eval_task_failure_rate(store, task_type.as_deref(), *window_minutes, *threshold_percent)
            }
            AlertCondition::EventMatches { event_type, field, value } => {
                self.eval_event_matches(store, event_type, field, value)
            }
            AlertCondition::NoActivity { event_type, period_minutes } => {
                self.eval_no_activity(store, event_type.as_deref(), *period_minutes)
            }
            AlertCondition::PatternDetected { name, sequence, .. } => {
                self.eval_pattern(store, name, sequence)
            }
            AlertCondition::And(conditions) => {
                let mut combined = ConditionResult::default();
                for c in conditions {
                    let result = self.evaluate_condition(c, store)?;
                    if !result.triggered {
                        return Ok(ConditionResult::default());
                    }
                    combined.context.extend(result.context);
                    combined.related_entries.extend(result.related_entries);
                }
                combined.triggered = true;
                combined.message = Some("All conditions matched".to_string());
                Ok(combined)
            }
            AlertCondition::Or(conditions) => {
                for c in conditions {
                    let result = self.evaluate_condition(c, store)?;
                    if result.triggered {
                        return Ok(result);
                    }
                }
                Ok(ConditionResult::default())
            }
        }
    }

    /// Evaluate count exceeds condition
    fn eval_count_exceeds(
        &self,
        store: &AuditStore,
        event_type: &str,
        window_minutes: u32,
        threshold: u32,
    ) -> AuditResult<ConditionResult> {
        let start = Utc::now() - Duration::minutes(window_minutes as i64);
        let entries = store.query_by_time_range(start, Utc::now())?;

        let count = entries.iter()
            .filter(|e| e.event_type() == event_type)
            .count();

        if count > threshold as usize {
            Ok(ConditionResult {
                triggered: true,
                message: Some(format!(
                    "Count of '{}' events ({}) exceeds threshold ({}) in last {} minutes",
                    event_type, count, threshold, window_minutes
                )),
                context: vec![
                    ("event_type".to_string(), serde_json::json!(event_type)),
                    ("count".to_string(), serde_json::json!(count)),
                    ("threshold".to_string(), serde_json::json!(threshold)),
                    ("window_minutes".to_string(), serde_json::json!(window_minutes)),
                ].into_iter().collect(),
                related_entries: entries.iter().take(10).map(|e| e.id).collect(),
            })
        } else {
            Ok(ConditionResult::default())
        }
    }

    /// Evaluate deny rate condition
    fn eval_deny_rate(
        &self,
        store: &AuditStore,
        window_minutes: u32,
        threshold_percent: f64,
    ) -> AuditResult<ConditionResult> {
        let start = Utc::now() - Duration::minutes(window_minutes as i64);
        let entries = store.query_by_time_range(start, Utc::now())?;

        let decisions: Vec<_> = entries.iter()
            .filter_map(|e| match &e.event {
                AuditEvent::Decision(d) => Some(&d.decision),
                _ => None,
            })
            .collect();

        if decisions.is_empty() {
            return Ok(ConditionResult::default());
        }

        let deny_count = decisions.iter().filter(|d| **d == "Deny").count();
        let deny_rate = (deny_count as f64 / decisions.len() as f64) * 100.0;

        if deny_rate > threshold_percent {
            Ok(ConditionResult {
                triggered: true,
                message: Some(format!(
                    "Deny rate ({:.1}%) exceeds threshold ({:.1}%) in last {} minutes",
                    deny_rate, threshold_percent, window_minutes
                )),
                context: vec![
                    ("deny_rate".to_string(), serde_json::json!(deny_rate)),
                    ("threshold_percent".to_string(), serde_json::json!(threshold_percent)),
                    ("deny_count".to_string(), serde_json::json!(deny_count)),
                    ("total_decisions".to_string(), serde_json::json!(decisions.len())),
                ].into_iter().collect(),
                related_entries: entries.iter().take(10).map(|e| e.id).collect(),
            })
        } else {
            Ok(ConditionResult::default())
        }
    }

    /// Evaluate task failure rate condition
    fn eval_task_failure_rate(
        &self,
        store: &AuditStore,
        task_type: Option<&str>,
        window_minutes: u32,
        threshold_percent: f64,
    ) -> AuditResult<ConditionResult> {
        let start = Utc::now() - Duration::minutes(window_minutes as i64);
        let entries = store.query_by_time_range(start, Utc::now())?;

        let tasks: Vec<_> = entries.iter()
            .filter_map(|e| match &e.event {
                AuditEvent::TaskExecution(t) => {
                    if let Some(tt) = task_type {
                        if t.task_type != tt {
                            return None;
                        }
                    }
                    Some((&t.status, &t.task_name))
                }
                _ => None,
            })
            .collect();

        if tasks.is_empty() {
            return Ok(ConditionResult::default());
        }

        let failed_count = tasks.iter().filter(|(s, _)| *s == "failed").count();
        let failure_rate = (failed_count as f64 / tasks.len() as f64) * 100.0;

        if failure_rate > threshold_percent {
            Ok(ConditionResult {
                triggered: true,
                message: Some(format!(
                    "Task failure rate ({:.1}%) exceeds threshold ({:.1}%) in last {} minutes",
                    failure_rate, threshold_percent, window_minutes
                )),
                context: vec![
                    ("failure_rate".to_string(), serde_json::json!(failure_rate)),
                    ("threshold_percent".to_string(), serde_json::json!(threshold_percent)),
                    ("failed_count".to_string(), serde_json::json!(failed_count)),
                    ("total_tasks".to_string(), serde_json::json!(tasks.len())),
                ].into_iter().collect(),
                related_entries: entries.iter().take(10).map(|e| e.id).collect(),
            })
        } else {
            Ok(ConditionResult::default())
        }
    }

    /// Evaluate event matches condition
    fn eval_event_matches(
        &self,
        store: &AuditStore,
        event_type: &str,
        field: &str,
        value: &str,
    ) -> AuditResult<ConditionResult> {
        let entries = store.query_recent(100)?;
        let pattern = wildcard_to_regex(value);

        let matches: Vec<_> = entries.iter()
            .filter(|e| {
                if e.event_type() != event_type {
                    return false;
                }
                match &e.event {
                    AuditEvent::Decision(d) => {
                        let field_value = match field {
                            "subject_id" => &d.subject_id,
                            "resource_id" => &d.resource_id,
                            "action" => &d.action,
                            "decision" => &d.decision,
                            _ => return false,
                        };
                        pattern.is_match(field_value)
                    }
                    AuditEvent::TaskExecution(t) => {
                        let field_value = match field {
                            "task_name" => &t.task_name,
                            "task_type" => &t.task_type,
                            "status" => &t.status,
                            _ => return false,
                        };
                        pattern.is_match(field_value)
                    }
                    AuditEvent::System(s) => {
                        let field_value = match field {
                            "event" => &s.event,
                            "level" => &s.level,
                            _ => return false,
                        };
                        pattern.is_match(field_value)
                    }
                    _ => false,
                }
            })
            .collect();

        if !matches.is_empty() {
            Ok(ConditionResult {
                triggered: true,
                message: Some(format!(
                    "Event '{}' matched condition: {}={}",
                    event_type, field, value
                )),
                context: vec![
                    ("event_type".to_string(), serde_json::json!(event_type)),
                    ("field".to_string(), serde_json::json!(field)),
                    ("value".to_string(), serde_json::json!(value)),
                    ("match_count".to_string(), serde_json::json!(matches.len())),
                ].into_iter().collect(),
                related_entries: matches.iter().take(10).map(|e| e.id).collect(),
            })
        } else {
            Ok(ConditionResult::default())
        }
    }

    /// Evaluate no activity condition
    fn eval_no_activity(
        &self,
        store: &AuditStore,
        event_type: Option<&str>,
        period_minutes: u32,
    ) -> AuditResult<ConditionResult> {
        let start = Utc::now() - Duration::minutes(period_minutes as i64);
        let entries = store.query_by_time_range(start, Utc::now())?;

        let has_activity = if let Some(et) = event_type {
            entries.iter().any(|e| e.event_type() == et)
        } else {
            !entries.is_empty()
        };

        if !has_activity {
            Ok(ConditionResult {
                triggered: true,
                message: Some(format!(
                    "No activity detected for {} in last {} minutes",
                    event_type.unwrap_or("any event"),
                    period_minutes
                )),
                context: vec![
                    ("event_type".to_string(), serde_json::json!(event_type)),
                    ("period_minutes".to_string(), serde_json::json!(period_minutes)),
                ].into_iter().collect(),
                related_entries: vec![],
            })
        } else {
            Ok(ConditionResult::default())
        }
    }

    /// Evaluate pattern detection
    fn eval_pattern(
        &self,
        store: &AuditStore,
        name: &str,
        sequence: &[PatternStep],
    ) -> AuditResult<ConditionResult> {
        if sequence.is_empty() {
            return Ok(ConditionResult::default());
        }

        let entries = store.query_recent(100)?;

        // Simple pattern matching: check if events appear in sequence
        let mut step_idx = 0;
        let mut matched_entries: Vec<Uuid> = Vec::new();

        for entry in &entries {
            if step_idx >= sequence.len() {
                break;
            }

            let step = &sequence[step_idx];
            if entry.event_type() != step.event_type {
                continue;
            }

            // Check filters
            let matches_filters = match &entry.event {
                AuditEvent::Decision(d) => Self::check_decision_filters(d, &step.filters),
                AuditEvent::TaskExecution(t) => Self::check_task_filters(t, &step.filters),
                _ => true,
            };

            if matches_filters {
                matched_entries.push(entry.id);
                step_idx += 1;
            }
        }

        if step_idx == sequence.len() {
            Ok(ConditionResult {
                triggered: true,
                message: Some(format!("Pattern '{}' detected", name)),
                context: vec![
                    ("pattern_name".to_string(), serde_json::json!(name)),
                    ("matched_steps".to_string(), serde_json::json!(step_idx)),
                ].into_iter().collect(),
                related_entries: matched_entries,
            })
        } else {
            Ok(ConditionResult::default())
        }
    }

    fn check_decision_filters(d: &super::DecisionAudit, filters: &HashMap<String, String>) -> bool {
        for (key, value) in filters {
            let actual = match key.as_str() {
                "subject_id" => &d.subject_id,
                "resource_id" => &d.resource_id,
                "action" => &d.action,
                "decision" => &d.decision,
                _ => continue,
            };
            if actual != value {
                return false;
            }
        }
        true
    }

    fn check_task_filters(t: &super::TaskAudit, filters: &HashMap<String, String>) -> bool {
        for (key, value) in filters {
            let actual = match key.as_str() {
                "task_name" => &t.task_name,
                "task_type" => &t.task_type,
                "status" => &t.status,
                _ => continue,
            };
            if actual != value {
                return false;
            }
        }
        true
    }

    /// Add triggered alert to history
    fn add_triggered(&self, alert: TriggeredAlert) {
        let mut triggered = self.triggered.write();

        // Trim if needed
        if triggered.len() >= self.max_triggered {
            let remove = triggered.len() - self.max_triggered + 1;
            triggered.drain(0..remove);
        }

        triggered.push(alert);
    }

    /// Get triggered alerts
    pub fn triggered_alerts(&self) -> Vec<TriggeredAlert> {
        self.triggered.read().clone()
    }

    /// Get triggered alerts by severity
    pub fn triggered_by_severity(&self, severity: AlertSeverity) -> Vec<TriggeredAlert> {
        self.triggered.read()
            .iter()
            .filter(|a| a.severity == severity)
            .cloned()
            .collect()
    }

    /// Clear triggered alerts history
    pub fn clear_triggered(&self) {
        self.triggered.write().clear();
    }

    /// Clear cooldown for a rule
    pub fn clear_cooldown(&self, rule_id: &Uuid) {
        self.last_trigger.write().remove(rule_id);
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Condition evaluation result
#[derive(Debug, Clone, Default)]
struct ConditionResult {
    triggered: bool,
    message: Option<String>,
    context: HashMap<String, serde_json::Value>,
    related_entries: Vec<Uuid>,
}

/// Convert wildcard pattern to regex
fn wildcard_to_regex(pattern: &str) -> Regex {
    let mut regex_str = String::from("^");
    for c in pattern.chars() {
        match c {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push_str("."),
            '.' | '^' | '$' | '+' | '[' | ']' | '(' | ')' | '{' | '}' | '\\' | '|' => {
                regex_str.push('\\');
                regex_str.push(c);
            }
            _ => regex_str.push(c),
        }
    }
    regex_str.push('$');
    Regex::new(&regex_str).unwrap_or_else(|_| Regex::new("^.*$").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_store_with_data() -> AuditStore {
        let store = AuditStore::in_memory().unwrap();

        // Insert decision entries
        for i in 0..20 {
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                if i < 15 { "Permit" } else { "Deny" },
                &format!("user{}", i % 5),
                &format!("doc{}", i),
                "document",
                "read",
                "Policy",
                1, 1, vec![], 100,
            );
            store.insert(&entry).unwrap();
        }

        // Insert task entries
        for i in 0..10 {
            let entry = AuditEntry::from_task(
                Uuid::new_v4(),
                &format!("task{}", i),
                if i < 5 { "cron" } else { "dag" },
                if i % 3 == 0 { "failed" } else { "completed" },
                None, None, None, None,
            );
            store.insert(&entry).unwrap();
        }

        // Insert system entries
        for i in 0..5 {
            let entry = AuditEntry::system(
                "test",
                if i == 0 { "error" } else { "info" },
                &format!("Message {}", i),
                None,
            );
            store.insert(&entry).unwrap();
        }

        store
    }

    #[test]
    fn test_alert_rule_builder() {
        let rule = AlertRule::new(
            "High Deny Rate",
            AlertCondition::DenyRateExceeds {
                window_minutes: 60,
                threshold_percent: 50.0,
            },
        )
        .with_description("Alert when deny rate exceeds 50%")
        .with_severity(AlertSeverity::Warning)
        .with_action(AlertAction::Log)
        .with_cooldown(10)
        .with_tag("security");

        assert_eq!(rule.name, "High Deny Rate");
        assert_eq!(rule.severity, AlertSeverity::Warning);
        assert_eq!(rule.actions.len(), 1);
        assert_eq!(rule.cooldown_minutes, 10);
        assert!(rule.tags.contains(&"security".to_string()));
    }

    #[test]
    fn test_count_exceeds_condition() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "Too Many Decisions",
            AlertCondition::CountExceeds {
                event_type: "decision".to_string(),
                window_minutes: 60,
                threshold: 10,
            },
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].message.contains("exceeds threshold"));
    }

    #[test]
    fn test_deny_rate_condition() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        // 5 denies out of 20 decisions = 25% deny rate
        let rule = AlertRule::new(
            "High Deny Rate",
            AlertCondition::DenyRateExceeds {
                window_minutes: 60,
                threshold_percent: 20.0, // 25% > 20%
            },
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].message.contains("Deny rate"));
    }

    #[test]
    fn test_task_failure_rate_condition() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        // 4 failures out of 10 tasks = 40% failure rate
        let rule = AlertRule::new(
            "High Task Failure Rate",
            AlertCondition::TaskFailureRateExceeds {
                task_type: None,
                window_minutes: 60,
                threshold_percent: 30.0, // 40% > 30%
            },
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].message.contains("failure rate"));
    }

    #[test]
    fn test_event_matches_condition() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "Deny Events",
            AlertCondition::EventMatches {
                event_type: "decision".to_string(),
                field: "decision".to_string(),
                value: "Deny".to_string(),
            },
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].message.contains("matched condition"));
    }

    #[test]
    fn test_wildcard_match() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "User Pattern",
            AlertCondition::EventMatches {
                event_type: "decision".to_string(),
                field: "subject_id".to_string(),
                value: "user*".to_string(),
            },
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_no_activity_condition() {
        let store = AuditStore::in_memory().unwrap(); // Empty store
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "No Activity",
            AlertCondition::NoActivity {
                event_type: None,
                period_minutes: 1,
            },
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].message.contains("No activity"));
    }

    #[test]
    fn test_and_condition() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "Combined Alert",
            AlertCondition::And(vec![
                AlertCondition::CountExceeds {
                    event_type: "decision".to_string(),
                    window_minutes: 60,
                    threshold: 10,
                },
                AlertCondition::CountExceeds {
                    event_type: "task".to_string(),
                    window_minutes: 60,
                    threshold: 5,
                },
            ]),
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_or_condition() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "Either Condition",
            AlertCondition::Or(vec![
                AlertCondition::CountExceeds {
                    event_type: "nonexistent".to_string(),
                    window_minutes: 60,
                    threshold: 1,
                },
                AlertCondition::CountExceeds {
                    event_type: "decision".to_string(),
                    window_minutes: 60,
                    threshold: 10,
                },
            ]),
        );

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_cooldown() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "Test Cooldown",
            AlertCondition::CountExceeds {
                event_type: "decision".to_string(),
                window_minutes: 60,
                threshold: 10,
            },
        )
        .with_cooldown(60); // 60 minute cooldown

        manager.add_rule(rule);

        // First evaluation should trigger
        let alerts1 = manager.evaluate(&store).unwrap();
        assert_eq!(alerts1.len(), 1);

        // Second evaluation should not trigger due to cooldown
        let alerts2 = manager.evaluate(&store).unwrap();
        assert_eq!(alerts2.len(), 0);
    }

    #[test]
    fn test_rule_management() {
        let manager = AlertManager::new();

        let rule1 = AlertRule::new("Rule 1", AlertCondition::CountExceeds {
            event_type: "decision".to_string(),
            window_minutes: 60,
            threshold: 10,
        });

        let rule2 = AlertRule::new("Rule 2", AlertCondition::CountExceeds {
            event_type: "task".to_string(),
            window_minutes: 60,
            threshold: 5,
        });

        manager.add_rule(rule1.clone());
        manager.add_rule(rule2.clone());

        assert_eq!(manager.rules().len(), 2);

        // Get rule
        let retrieved = manager.get_rule(&rule1.id).unwrap();
        assert_eq!(retrieved.name, "Rule 1");

        // Remove rule
        assert!(manager.remove_rule(&rule1.id));
        assert_eq!(manager.rules().len(), 1);

        // Update rule
        let mut updated = rule2.clone();
        updated.name = "Updated Rule 2".to_string();
        assert!(manager.update_rule(updated));
        assert_eq!(manager.get_rule(&rule2.id).unwrap().name, "Updated Rule 2");
    }

    #[test]
    fn test_triggered_alerts_history() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let rule = AlertRule::new(
            "Test",
            AlertCondition::CountExceeds {
                event_type: "decision".to_string(),
                window_minutes: 60,
                threshold: 10,
            },
        )
        .with_cooldown(0); // No cooldown

        manager.add_rule(rule);
        manager.evaluate(&store).unwrap();

        let triggered = manager.triggered_alerts();
        assert_eq!(triggered.len(), 1);

        manager.clear_triggered();
        assert!(manager.triggered_alerts().is_empty());
    }

    #[test]
    fn test_disabled_rule() {
        let store = setup_store_with_data();
        let manager = AlertManager::new();

        let mut rule = AlertRule::new(
            "Disabled Rule",
            AlertCondition::CountExceeds {
                event_type: "decision".to_string(),
                window_minutes: 60,
                threshold: 10,
            },
        );
        rule.enabled = false;

        manager.add_rule(rule);
        let alerts = manager.evaluate(&store).unwrap();

        assert!(alerts.is_empty());
    }

    #[test]
    fn test_wildcard_to_regex() {
        let re = wildcard_to_regex("user*");
        assert!(re.is_match("user123"));
        assert!(re.is_match("user"));
        assert!(!re.is_match("admin"));

        let re = wildcard_to_regex("doc?");
        assert!(re.is_match("doc1"));
        assert!(!re.is_match("doc12"));

        let re = wildcard_to_regex("*test*");
        assert!(re.is_match("my_test_file"));
        assert!(!re.is_match("example"));
    }
}