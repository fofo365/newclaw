//! Audit Query Engine for NewClaw v0.7.0
//!
//! This module provides comprehensive audit logging, querying, and alerting capabilities.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Audit Query Engine                         │
//! │                                                                 │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
//! │  │  AuditLog    │  │ AuditQuery   │  │ AuditReport  │         │
//! │  │  (Core)      │  │ (Query)      │  │ (Stats)      │         │
//! │  └──────────────┘  └──────────────┘  └──────────────┘         │
//! │         │                 │                 │                  │
//! │         ▼                 ▼                 ▼                  │
//! │  ┌──────────────────────────────────────────────────────┐     │
//! │  │                    AuditStore (SQLite)               │     │
//! │  │  - Persistent storage                                 │     │
//! │  │  - Log rotation                                       │     │
//! │  │  - Archiving                                          │     │
//! │  └──────────────────────────────────────────────────────┘     │
//! │         │                                                       │
//! │         ▼                                                       │
//! │  ┌──────────────────────────────────────────────────────┐     │
//! │  │                   Alert System                        │     │
//! │  │  - Rule definitions                                   │     │
//! │  │  - Trigger evaluation                                 │     │
//! │  │  - Notification integration                           │     │
//! │  └──────────────────────────────────────────────────────┘     │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **Persistent Storage**: SQLite-based audit log persistence
//! - **Log Rotation**: Automatic rotation and archiving of old logs
//! - **Advanced Querying**: Complex filters, time ranges, pagination
//! - **Statistics**: Aggregation and reporting capabilities
//! - **Export**: CSV and JSON export formats
//! - **Alerting**: Configurable alert rules and notifications
//!
//! ## Usage
//!
//! ```rust,ignore
//! use newclaw::audit::*;
//!
//! // Create audit store
//! let store = AuditStore::open("audit.db")?;
//!
//! // Log an audit event
//! let entry = AuditEntry::new(AuditEvent::Decision(decision_result));
//! store.insert(&entry)?;
//!
//! // Query with filters
//! let query = AuditQueryBuilder::new()
//!     .with_subject("user123")
//!     .with_time_range(start, end)
//!     .with_decision(Decision::Deny)
//!     .build();
//!
//! let results = store.query(&query)?;
//!
//! // Generate report
//! let report = AuditReporter::new(&store)
//!     .with_time_range(start, end)
//!     .generate()?;
//!
//! // Export to CSV
//! report.export_csv("audit_report.csv")?;
//! ```

pub mod store;
pub mod query;
pub mod report;
pub mod alert;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashMap;

// Re-exports
pub use store::{AuditStore, AuditStoreError, RotationConfig};
pub use query::{AuditQueryBuilder, AuditQuery, AuditFilter, SortOrder, Pagination};
pub use report::{AuditReporter, AuditReport, AuditStatistics, ExportFormat};
pub use alert::{AlertRule, AlertManager, AlertCondition, AlertAction, AlertSeverity};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuditEvent {
    /// Authorization decision
    Decision(DecisionAudit),
    /// Task execution
    TaskExecution(TaskAudit),
    /// Configuration change
    ConfigChange(ConfigAudit),
    /// System event
    System(SystemAudit),
    /// Custom event
    Custom(CustomAudit),
}

/// Decision audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionAudit {
    /// Decision ID
    pub decision_id: Uuid,
    /// Request ID
    pub request_id: Uuid,
    /// Decision (Permit/Deny/etc.)
    pub decision: String,
    /// Subject ID
    pub subject_id: String,
    /// Resource ID
    pub resource_id: String,
    /// Resource type
    pub resource_type: String,
    /// Action
    pub action: String,
    /// Reason
    pub reason: String,
    /// Policies evaluated
    pub policies_evaluated: usize,
    /// Policies matched
    pub policies_matched: usize,
    /// Matched policy names
    pub matched_policy_names: Vec<String>,
    /// Duration in microseconds
    pub duration_us: u64,
}

/// Task execution audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAudit {
    /// Task ID
    pub task_id: Uuid,
    /// Task name
    pub task_name: String,
    /// Task type (dag, cron, delayed, event)
    pub task_type: String,
    /// Status (started, completed, failed, cancelled)
    pub status: String,
    /// Input parameters (JSON)
    pub input: Option<String>,
    /// Output (JSON)
    pub output: Option<String>,
    /// Error message
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
}

/// Configuration change audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigAudit {
    /// Config scope
    pub scope: String,
    /// Config key
    pub key: String,
    /// Change type (created, updated, deleted)
    pub change_type: String,
    /// Old value (JSON)
    pub old_value: Option<String>,
    /// New value (JSON)
    pub new_value: Option<String>,
    /// Source (file, api, hot_reload)
    pub source: String,
}

/// System audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemAudit {
    /// Event name
    pub event: String,
    /// Event level (info, warning, error)
    pub level: String,
    /// Message
    pub message: String,
    /// Additional data (JSON)
    pub data: Option<String>,
}

/// Custom audit record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAudit {
    /// Event category
    pub category: String,
    /// Event name
    pub name: String,
    /// Event data (JSON)
    pub data: HashMap<String, serde_json::Value>,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry ID
    pub id: Uuid,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type and data
    pub event: AuditEvent,
    /// Source agent/service
    pub source: String,
    /// Correlation ID for tracing
    pub correlation_id: Option<Uuid>,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(event: AuditEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event,
            source: "newclaw".to_string(),
            correlation_id: None,
            metadata: HashMap::new(),
        }
    }

    /// Set source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Create from decision result
    pub fn from_decision(
        decision_id: Uuid,
        request_id: Uuid,
        decision: &str,
        subject_id: &str,
        resource_id: &str,
        resource_type: &str,
        action: &str,
        reason: &str,
        policies_evaluated: usize,
        policies_matched: usize,
        matched_policy_names: Vec<String>,
        duration_us: u64,
    ) -> Self {
        Self::new(AuditEvent::Decision(DecisionAudit {
            decision_id,
            request_id,
            decision: decision.to_string(),
            subject_id: subject_id.to_string(),
            resource_id: resource_id.to_string(),
            resource_type: resource_type.to_string(),
            action: action.to_string(),
            reason: reason.to_string(),
            policies_evaluated,
            policies_matched,
            matched_policy_names,
            duration_us,
        }))
    }

    /// Create from task execution
    pub fn from_task(
        task_id: Uuid,
        task_name: &str,
        task_type: &str,
        status: &str,
        input: Option<&str>,
        output: Option<&str>,
        error: Option<&str>,
        duration_ms: Option<u64>,
    ) -> Self {
        Self::new(AuditEvent::TaskExecution(TaskAudit {
            task_id,
            task_name: task_name.to_string(),
            task_type: task_type.to_string(),
            status: status.to_string(),
            input: input.map(|s| s.to_string()),
            output: output.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
            duration_ms,
        }))
    }

    /// Create from config change
    pub fn from_config(
        scope: &str,
        key: &str,
        change_type: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
        source: &str,
    ) -> Self {
        Self::new(AuditEvent::ConfigChange(ConfigAudit {
            scope: scope.to_string(),
            key: key.to_string(),
            change_type: change_type.to_string(),
            old_value: old_value.map(|s| s.to_string()),
            new_value: new_value.map(|s| s.to_string()),
            source: source.to_string(),
        }))
    }

    /// Create system event
    pub fn system(event: &str, level: &str, message: &str, data: Option<&str>) -> Self {
        Self::new(AuditEvent::System(SystemAudit {
            event: event.to_string(),
            level: level.to_string(),
            message: message.to_string(),
            data: data.map(|s| s.to_string()),
        }))
    }

    /// Get event type name
    pub fn event_type(&self) -> &'static str {
        match &self.event {
            AuditEvent::Decision(_) => "decision",
            AuditEvent::TaskExecution(_) => "task",
            AuditEvent::ConfigChange(_) => "config",
            AuditEvent::System(_) => "system",
            AuditEvent::Custom(_) => "custom",
        }
    }

    /// Get subject ID if applicable
    pub fn subject_id(&self) -> Option<&str> {
        match &self.event {
            AuditEvent::Decision(d) => Some(&d.subject_id),
            _ => None,
        }
    }

    /// Get resource ID if applicable
    pub fn resource_id(&self) -> Option<&str> {
        match &self.event {
            AuditEvent::Decision(d) => Some(&d.resource_id),
            _ => None,
        }
    }
}

/// Audit error type
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Store error: {0}")]
    Store(#[from] AuditStoreError),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Report error: {0}")]
    Report(String),

    #[error("Alert error: {0}")]
    Alert(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("CSV error: {0}")]
    Csv(String),
}

pub type AuditResult<T> = Result<T, AuditError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_decision() {
        let entry = AuditEntry::from_decision(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Permit",
            "user123",
            "doc456",
            "document",
            "read",
            "PolicyPermit",
            3,
            1,
            vec!["allow_admin".to_string()],
            150,
        );

        assert_eq!(entry.event_type(), "decision");
        assert_eq!(entry.subject_id(), Some("user123"));
        assert_eq!(entry.resource_id(), Some("doc456"));
    }

    #[test]
    fn test_audit_entry_task() {
        let entry = AuditEntry::from_task(
            Uuid::new_v4(),
            "daily_report",
            "cron",
            "completed",
            Some("{\"report\": \"daily\"}"),
            Some("{\"status\": \"ok\"}"),
            None,
            Some(1234),
        );

        assert_eq!(entry.event_type(), "task");
        assert!(entry.subject_id().is_none());
    }

    #[test]
    fn test_audit_entry_config() {
        let entry = AuditEntry::from_config(
            "agent.main",
            "model",
            "updated",
            Some("gpt-3.5"),
            Some("gpt-4"),
            "hot_reload",
        );

        assert_eq!(entry.event_type(), "config");
    }

    #[test]
    fn test_audit_entry_system() {
        let entry = AuditEntry::system(
            "startup",
            "info",
            "NewClaw v0.7.0 started",
            None,
        );

        assert_eq!(entry.event_type(), "system");
    }

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditEntry::new(AuditEvent::System(SystemAudit {
            event: "test".to_string(),
            level: "info".to_string(),
            message: "Test event".to_string(),
            data: None,
        }))
        .with_source("test_service")
        .with_correlation_id(Uuid::new_v4())
        .with_metadata("key1", "value1");

        assert_eq!(entry.source, "test_service");
        assert!(entry.correlation_id.is_some());
        assert_eq!(entry.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent::Decision(DecisionAudit {
            decision_id: Uuid::new_v4(),
            request_id: Uuid::new_v4(),
            decision: "Permit".to_string(),
            subject_id: "user1".to_string(),
            resource_id: "doc1".to_string(),
            resource_type: "document".to_string(),
            action: "read".to_string(),
            reason: "PolicyPermit".to_string(),
            policies_evaluated: 3,
            policies_matched: 1,
            matched_policy_names: vec!["policy1".to_string()],
            duration_us: 100,
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"decision\""));

        let parsed: AuditEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, AuditEvent::Decision(_)));
    }
}