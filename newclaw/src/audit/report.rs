//! Statistics and report generation for audit logs
//!
//! This module provides:
//! - Statistical aggregation
//! - Report generation
//! - CSV and JSON export

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;
use std::path::Path;
use std::io::Write;
use csv::Writer;

use super::{AuditEntry, AuditStore, AuditResult, AuditError};
use super::query::{AuditQueryBuilder, AuditQuery, Pagination};

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// JSON Lines (one JSON object per line)
    Jsonl,
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    /// Total entries
    pub total: usize,
    /// Time range
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    /// Count by event type
    pub by_event_type: HashMap<String, usize>,
    /// Count by source
    pub by_source: HashMap<String, usize>,
    /// Decision statistics (if applicable)
    pub decisions: Option<DecisionStatistics>,
    /// Task statistics (if applicable)
    pub tasks: Option<TaskStatistics>,
    /// Config statistics (if applicable)
    pub configs: Option<ConfigStatistics>,
    /// System statistics (if applicable)
    pub system: Option<SystemStatistics>,
    /// Average entries per hour
    pub entries_per_hour: f64,
    /// Report generated at
    pub generated_at: DateTime<Utc>,
}

/// Decision statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionStatistics {
    /// Total decisions
    pub total: usize,
    /// By decision type (Permit/Deny/NotApplicable/Indeterminate)
    pub by_decision: HashMap<String, usize>,
    /// By action
    pub by_action: HashMap<String, usize>,
    /// By resource type
    pub by_resource_type: HashMap<String, usize>,
    /// Top subjects by access count
    pub top_subjects: Vec<(String, usize)>,
    /// Top resources by access count
    pub top_resources: Vec<(String, usize)>,
    /// Average evaluation time (microseconds)
    pub avg_duration_us: f64,
    /// Deny rate (percentage)
    pub deny_rate: f64,
}

/// Task statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatistics {
    /// Total tasks
    pub total: usize,
    /// By task type
    pub by_type: HashMap<String, usize>,
    /// By status
    pub by_status: HashMap<String, usize>,
    /// Success rate (percentage)
    pub success_rate: f64,
    /// Average duration (milliseconds)
    pub avg_duration_ms: Option<f64>,
}

/// Config statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigStatistics {
    /// Total changes
    pub total: usize,
    /// By change type
    pub by_change_type: HashMap<String, usize>,
    /// By scope
    pub by_scope: HashMap<String, usize>,
    /// Top changed keys
    pub top_keys: Vec<(String, usize)>,
}

/// System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatistics {
    /// Total events
    pub total: usize,
    /// By level
    pub by_level: HashMap<String, usize>,
    /// Error count
    pub error_count: usize,
    /// Warning count
    pub warning_count: usize,
}

/// Audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Report title
    pub title: String,
    /// Time range
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    /// Statistics
    pub statistics: AuditStatistics,
    /// Recent entries (sample)
    pub recent_entries: Vec<AuditEntry>,
    /// Report metadata
    pub metadata: HashMap<String, String>,
}

/// Report builder
pub struct AuditReporter<'a> {
    store: &'a AuditStore,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    title: Option<String>,
    sample_size: usize,
    metadata: HashMap<String, String>,
}

impl<'a> AuditReporter<'a> {
    /// Create a new reporter
    pub fn new(store: &'a AuditStore) -> Self {
        Self {
            store,
            start_time: None,
            end_time: None,
            title: None,
            sample_size: 10,
            metadata: HashMap::new(),
        }
    }

    /// Set time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Set start time
    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self
    }

    /// Set end time
    pub fn with_end_time(mut self, end: DateTime<Utc>) -> Self {
        self.end_time = Some(end);
        self
    }

    /// Set report title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set sample size for recent entries
    pub fn with_sample_size(mut self, size: usize) -> Self {
        self.sample_size = size;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Generate the report
    pub fn generate(self) -> AuditResult<AuditReport> {
        let title = self.title.clone().unwrap_or_else(|| {
            format!(
                "Audit Report - {}",
                Utc::now().format("%Y-%m-%d %H:%M")
            )
        });

        // Get entries for the time range
        let mut query_builder = AuditQueryBuilder::new()
            .paginate(1, 10000); // Large page size for stats

        if let Some(start) = self.start_time {
            query_builder = query_builder.with_start_time(start);
        }
        if let Some(end) = self.end_time {
            query_builder = query_builder.with_end_time(end);
        }

        let query = query_builder.build();
        let result = self.store.query(&query)?;
        let entries = result.entries;

        // Calculate statistics
        let statistics = self.calculate_statistics(&entries)?;

        // Get recent entries sample
        let recent_query = AuditQueryBuilder::new()
            .paginate(1, self.sample_size)
            .build();
        let recent_entries = self.store.query(&recent_query)?.entries;

        Ok(AuditReport {
            title,
            start_time: self.start_time,
            end_time: self.end_time,
            statistics,
            recent_entries,
            metadata: self.metadata,
        })
    }

    /// Calculate statistics from entries
    fn calculate_statistics(&self, entries: &[AuditEntry]) -> AuditResult<AuditStatistics> {
        let total = entries.len();

        if total == 0 {
            return Ok(AuditStatistics {
                total: 0,
                start_time: None,
                end_time: None,
                by_event_type: HashMap::new(),
                by_source: HashMap::new(),
                decisions: None,
                tasks: None,
                configs: None,
                system: None,
                entries_per_hour: 0.0,
                generated_at: Utc::now(),
            });
        }

        // Time range
        let start_time = entries.iter().map(|e| e.timestamp).min();
        let end_time = entries.iter().map(|e| e.timestamp).max();

        // Calculate entries per hour
        let entries_per_hour = if let (Some(start), Some(end)) = (start_time, end_time) {
            let hours = (end - start).num_seconds() as f64 / 3600.0;
            if hours > 0.0 {
                total as f64 / hours
            } else {
                total as f64
            }
        } else {
            0.0
        };

        // Count by event type
        let mut by_event_type: HashMap<String, usize> = HashMap::new();
        let mut by_source: HashMap<String, usize> = HashMap::new();

        for entry in entries {
            *by_event_type.entry(entry.event_type().to_string()).or_insert(0) += 1;
            *by_source.entry(entry.source.clone()).or_insert(0) += 1;
        }

        // Calculate type-specific statistics
        let decisions = self.calculate_decision_stats(entries);
        let tasks = self.calculate_task_stats(entries);
        let configs = self.calculate_config_stats(entries);
        let system = self.calculate_system_stats(entries);

        Ok(AuditStatistics {
            total,
            start_time,
            end_time,
            by_event_type,
            by_source,
            decisions,
            tasks,
            configs,
            system,
            entries_per_hour,
            generated_at: Utc::now(),
        })
    }

    /// Calculate decision statistics
    fn calculate_decision_stats(&self, entries: &[AuditEntry]) -> Option<DecisionStatistics> {
        let decisions: Vec<_> = entries.iter()
            .filter_map(|e| match &e.event {
                super::AuditEvent::Decision(d) => Some(d),
                _ => None,
            })
            .collect();

        if decisions.is_empty() {
            return None;
        }

        let total = decisions.len();

        // By decision
        let mut by_decision: HashMap<String, usize> = HashMap::new();
        let mut by_action: HashMap<String, usize> = HashMap::new();
        let mut by_resource_type: HashMap<String, usize> = HashMap::new();
        let mut subject_counts: HashMap<String, usize> = HashMap::new();
        let mut resource_counts: HashMap<String, usize> = HashMap::new();
        let mut total_duration_us = 0u64;

        for d in &decisions {
            *by_decision.entry(d.decision.clone()).or_insert(0) += 1;
            *by_action.entry(d.action.clone()).or_insert(0) += 1;
            *by_resource_type.entry(d.resource_type.clone()).or_insert(0) += 1;
            *subject_counts.entry(d.subject_id.clone()).or_insert(0) += 1;
            *resource_counts.entry(d.resource_id.clone()).or_insert(0) += 1;
            total_duration_us += d.duration_us;
        }

        // Top subjects and resources
        let mut top_subjects: Vec<_> = subject_counts.into_iter().collect();
        top_subjects.sort_by(|a, b| b.1.cmp(&a.1));
        top_subjects.truncate(10);

        let mut top_resources: Vec<_> = resource_counts.into_iter().collect();
        top_resources.sort_by(|a, b| b.1.cmp(&a.1));
        top_resources.truncate(10);

        // Deny rate
        let deny_count = *by_decision.get("Deny").unwrap_or(&0);
        let deny_rate = (deny_count as f64 / total as f64) * 100.0;

        // Average duration
        let avg_duration_us = total_duration_us as f64 / total as f64;

        Some(DecisionStatistics {
            total,
            by_decision,
            by_action,
            by_resource_type,
            top_subjects,
            top_resources,
            avg_duration_us,
            deny_rate,
        })
    }

    /// Calculate task statistics
    fn calculate_task_stats(&self, entries: &[AuditEntry]) -> Option<TaskStatistics> {
        let tasks: Vec<_> = entries.iter()
            .filter_map(|e| match &e.event {
                super::AuditEvent::TaskExecution(t) => Some(t),
                _ => None,
            })
            .collect();

        if tasks.is_empty() {
            return None;
        }

        let total = tasks.len();

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut total_duration_ms = 0u64;
        let mut duration_count = 0usize;

        for t in &tasks {
            *by_type.entry(t.task_type.clone()).or_insert(0) += 1;
            *by_status.entry(t.status.clone()).or_insert(0) += 1;
            if let Some(d) = t.duration_ms {
                total_duration_ms += d;
                duration_count += 1;
            }
        }

        let completed_count = *by_status.get("completed").unwrap_or(&0);
        let success_rate = (completed_count as f64 / total as f64) * 100.0;

        let avg_duration_ms = if duration_count > 0 {
            Some(total_duration_ms as f64 / duration_count as f64)
        } else {
            None
        };

        Some(TaskStatistics {
            total,
            by_type,
            by_status,
            success_rate,
            avg_duration_ms,
        })
    }

    /// Calculate config statistics
    fn calculate_config_stats(&self, entries: &[AuditEntry]) -> Option<ConfigStatistics> {
        let configs: Vec<_> = entries.iter()
            .filter_map(|e| match &e.event {
                super::AuditEvent::ConfigChange(c) => Some(c),
                _ => None,
            })
            .collect();

        if configs.is_empty() {
            return None;
        }

        let total = configs.len();

        let mut by_change_type: HashMap<String, usize> = HashMap::new();
        let mut by_scope: HashMap<String, usize> = HashMap::new();
        let mut key_counts: HashMap<String, usize> = HashMap::new();

        for c in &configs {
            *by_change_type.entry(c.change_type.clone()).or_insert(0) += 1;
            *by_scope.entry(c.scope.clone()).or_insert(0) += 1;
            *key_counts.entry(format!("{}.{}", c.scope, c.key)).or_insert(0) += 1;
        }

        let mut top_keys: Vec<_> = key_counts.into_iter().collect();
        top_keys.sort_by(|a, b| b.1.cmp(&a.1));
        top_keys.truncate(10);

        Some(ConfigStatistics {
            total,
            by_change_type,
            by_scope,
            top_keys,
        })
    }

    /// Calculate system statistics
    fn calculate_system_stats(&self, entries: &[AuditEntry]) -> Option<SystemStatistics> {
        let systems: Vec<_> = entries.iter()
            .filter_map(|e| match &e.event {
                super::AuditEvent::System(s) => Some(s),
                _ => None,
            })
            .collect();

        if systems.is_empty() {
            return None;
        }

        let total = systems.len();

        let mut by_level: HashMap<String, usize> = HashMap::new();

        for s in &systems {
            *by_level.entry(s.level.clone()).or_insert(0) += 1;
        }

        let error_count = *by_level.get("error").unwrap_or(&0);
        let warning_count = *by_level.get("warning").unwrap_or(&0);

        Some(SystemStatistics {
            total,
            by_level,
            error_count,
            warning_count,
        })
    }
}

impl AuditReport {
    /// Export to file
    pub fn export<P: AsRef<Path>>(&self, path: P, format: ExportFormat) -> AuditResult<()> {
        let path = path.as_ref();

        match format {
            ExportFormat::Json => self.export_json(path),
            ExportFormat::Csv => self.export_csv(path),
            ExportFormat::Jsonl => self.export_jsonl(path),
        }
    }

    /// Export to JSON
    pub fn export_json<P: AsRef<Path>>(&self, path: P) -> AuditResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Export to CSV
    pub fn export_csv<P: AsRef<Path>>(&self, path: P) -> AuditResult<()> {
        let mut writer = Writer::from_path(path)
            .map_err(|e| AuditError::Csv(e.to_string()))?;

        // Write header
        writer.write_record(&[
            "id",
            "timestamp",
            "event_type",
            "source",
            "subject_id",
            "resource_id",
            "action",
            "decision",
            "details",
        ]).map_err(|e| AuditError::Csv(e.to_string()))?;

        // Write entries
        for entry in &self.recent_entries {
            let (subject_id, resource_id, action, decision, details) = match &entry.event {
                super::AuditEvent::Decision(d) => (
                    d.subject_id.clone(),
                    d.resource_id.clone(),
                    d.action.clone(),
                    d.decision.clone(),
                    format!("reason={}", d.reason),
                ),
                super::AuditEvent::TaskExecution(t) => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("task={}, status={}", t.task_name, t.status),
                ),
                super::AuditEvent::ConfigChange(c) => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("key={}.{}, change={}", c.scope, c.key, c.change_type),
                ),
                super::AuditEvent::System(s) => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("event={}, level={}", s.event, s.level),
                ),
                super::AuditEvent::Custom(c) => (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    format!("category={}, name={}", c.category, c.name),
                ),
            };

            writer.write_record(&[
                entry.id.to_string(),
                entry.timestamp.to_rfc3339(),
                entry.event_type().to_string(),
                entry.source.clone(),
                subject_id,
                resource_id,
                action,
                decision,
                details,
            ]).map_err(|e| AuditError::Csv(e.to_string()))?;
        }

        writer.flush().map_err(|e| AuditError::Csv(e.to_string()))?;
        Ok(())
    }

    /// Export to JSONL
    pub fn export_jsonl<P: AsRef<Path>>(&self, path: P) -> AuditResult<()> {
        let mut file = std::fs::File::create(path)?;

        for entry in &self.recent_entries {
            let line = serde_json::to_string(entry)?;
            writeln!(file, "{}", line)?;
        }

        Ok(())
    }

    /// Export entries only (without statistics)
    pub fn export_entries_csv<P: AsRef<Path>>(entries: &[AuditEntry], path: P) -> AuditResult<()> {
        let mut writer = Writer::from_path(path)
            .map_err(|e| AuditError::Csv(e.to_string()))?;

        // Write header
        writer.write_record(&[
            "id",
            "timestamp",
            "event_type",
            "source",
            "correlation_id",
            "metadata",
            "event_data",
        ]).map_err(|e| AuditError::Csv(e.to_string()))?;

        // Write entries
        for entry in entries {
            writer.write_record(&[
                entry.id.to_string(),
                entry.timestamp.to_rfc3339(),
                entry.event_type().to_string(),
                entry.source.clone(),
                entry.correlation_id.map(|id| id.to_string()).unwrap_or_default(),
                serde_json::to_string(&entry.metadata).unwrap_or_default(),
                serde_json::to_string(&entry.event).unwrap_or_default(),
            ]).map_err(|e| AuditError::Csv(e.to_string()))?;
        }

        writer.flush().map_err(|e| AuditError::Csv(e.to_string()))?;
        Ok(())
    }
}

impl AuditStore {
    /// Create a reporter for this store
    pub fn reporter(&self) -> AuditReporter {
        AuditReporter::new(self)
    }

    /// Get quick statistics
    pub fn quick_stats(&self) -> AuditResult<AuditStatistics> {
        AuditReporter::new(self)
            .generate()
            .map(|r| r.statistics)
    }

    /// Export all entries to a file
    pub fn export_all<P: AsRef<Path>>(&self, path: P, format: ExportFormat) -> AuditResult<usize> {
        let query = AuditQueryBuilder::new()
            .paginate(1, 1_000_000)
            .build();

        let result = self.query(&query)?;
        let entries = result.entries;

        let count = entries.len();

        match format {
            ExportFormat::Json => {
                let content = serde_json::to_string_pretty(&entries)?;
                std::fs::write(path, content)?;
            }
            ExportFormat::Csv => {
                AuditReport::export_entries_csv(&entries, path)?;
            }
            ExportFormat::Jsonl => {
                let mut file = std::fs::File::create(path)?;
                for entry in &entries {
                    writeln!(file, "{}", serde_json::to_string(entry)?)?;
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use uuid::Uuid;

    fn setup_store() -> AuditStore {
        let store = AuditStore::in_memory().unwrap();

        // Insert decision entries
        for i in 0..10 {
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                if i % 2 == 0 { "Permit" } else { "Deny" },
                &format!("user{}", i % 3),
                &format!("doc{}", i),
                "document",
                if i % 2 == 0 { "read" } else { "write" },
                "Policy",
                i + 1,
                1,
                vec!["policy1".to_string()],
                100 + i as u64,
            );
            store.insert(&entry).unwrap();
        }

        // Insert task entries
        for i in 0..5 {
            let entry = AuditEntry::from_task(
                Uuid::new_v4(),
                &format!("task{}", i),
                if i % 2 == 0 { "cron" } else { "dag" },
                if i % 3 == 0 { "failed" } else { "completed" },
                None,
                None,
                None,
                Some(1000 + i as u64 * 100),
            );
            store.insert(&entry).unwrap();
        }

        // Insert config entries
        for i in 0..3 {
            let entry = AuditEntry::from_config(
                &format!("agent.{}", i),
                "model",
                if i == 0 { "created" } else { "updated" },
                Some("old"),
                Some("new"),
                "hot_reload",
            );
            store.insert(&entry).unwrap();
        }

        // Insert system entries
        for i in 0..4 {
            let entry = AuditEntry::system(
                "startup",
                if i == 0 { "error" } else if i == 1 { "warning" } else { "info" },
                &format!("Message {}", i),
                None,
            );
            store.insert(&entry).unwrap();
        }

        store
    }

    #[test]
    fn test_audit_reporter_generate() {
        let store = setup_store();

        let report = AuditReporter::new(&store)
            .with_title("Test Report")
            .with_sample_size(5)
            .generate()
            .unwrap();

        assert_eq!(report.title, "Test Report");
        assert_eq!(report.statistics.total, 22);
        assert_eq!(report.recent_entries.len(), 5);
    }

    #[test]
    fn test_statistics_event_types() {
        let store = setup_store();

        let stats = store.quick_stats().unwrap();

        assert_eq!(stats.total, 22);
        assert_eq!(*stats.by_event_type.get("decision").unwrap(), 10);
        assert_eq!(*stats.by_event_type.get("task").unwrap(), 5);
        assert_eq!(*stats.by_event_type.get("config").unwrap(), 3);
        assert_eq!(*stats.by_event_type.get("system").unwrap(), 4);
    }

    #[test]
    fn test_decision_statistics() {
        let store = setup_store();

        let report = AuditReporter::new(&store).generate().unwrap();
        let decisions = report.statistics.decisions.unwrap();

        assert_eq!(decisions.total, 10);
        assert_eq!(*decisions.by_decision.get("Permit").unwrap(), 5);
        assert_eq!(*decisions.by_decision.get("Deny").unwrap(), 5);
        assert_eq!(decisions.deny_rate, 50.0);
        assert!(decisions.avg_duration_us > 0.0);

        // Check top subjects
        assert!(!decisions.top_subjects.is_empty());
        assert!(decisions.top_subjects[0].1 >= 3); // At least 3 entries for top subject
    }

    #[test]
    fn test_task_statistics() {
        let store = setup_store();

        let report = AuditReporter::new(&store).generate().unwrap();
        let tasks = report.statistics.tasks.unwrap();

        assert_eq!(tasks.total, 5);
        assert_eq!(*tasks.by_type.get("cron").unwrap(), 3);
        assert_eq!(*tasks.by_type.get("dag").unwrap(), 2);
        assert!(tasks.success_rate > 0.0);
        assert!(tasks.avg_duration_ms.is_some());
    }

    #[test]
    fn test_config_statistics() {
        let store = setup_store();

        let report = AuditReporter::new(&store).generate().unwrap();
        let configs = report.statistics.configs.unwrap();

        assert_eq!(configs.total, 3);
        assert_eq!(*configs.by_change_type.get("updated").unwrap(), 2);
        assert_eq!(*configs.by_change_type.get("created").unwrap(), 1);
    }

    #[test]
    fn test_system_statistics() {
        let store = setup_store();

        let report = AuditReporter::new(&store).generate().unwrap();
        let system = report.statistics.system.unwrap();

        assert_eq!(system.total, 4);
        assert_eq!(system.error_count, 1);
        assert_eq!(system.warning_count, 1);
    }

    #[test]
    fn test_export_json() {
        let store = setup_store();
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.json");

        let report = AuditReporter::new(&store)
            .with_title("Export Test")
            .generate()
            .unwrap();

        report.export_json(&path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: AuditReport = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.title, "Export Test");
    }

    #[test]
    fn test_export_csv() {
        let store = setup_store();
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.csv");

        let report = AuditReporter::new(&store).generate().unwrap();
        report.export_csv(&path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("id,timestamp,event_type"));
        assert!(content.contains("decision"));
    }

    #[test]
    fn test_export_jsonl() {
        let store = setup_store();
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.jsonl");

        let report = AuditReporter::new(&store)
            .with_sample_size(5)
            .generate()
            .unwrap();

        report.export_jsonl(&path).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn test_export_all() {
        let store = setup_store();
        let dir = tempdir().unwrap();
        let path = dir.path().join("all.json");

        let count = store.export_all(&path, ExportFormat::Json).unwrap();
        assert_eq!(count, 22);
        assert!(path.exists());
    }

    #[test]
    fn test_reporter_with_metadata() {
        let store = setup_store();

        let report = AuditReporter::new(&store)
            .with_metadata("version", "1.0")
            .with_metadata("author", "test")
            .generate()
            .unwrap();

        assert_eq!(report.metadata.get("version"), Some(&"1.0".to_string()));
        assert_eq!(report.metadata.get("author"), Some(&"test".to_string()));
    }

    #[test]
    fn test_entries_per_hour() {
        let store = AuditStore::in_memory().unwrap();

        // Insert entries spread over time (simulated)
        for _ in 0..5 {
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Permit",
                "user1",
                "doc1",
                "document",
                "read",
                "Policy",
                1, 1, vec![], 100,
            );
            store.insert(&entry).unwrap();
        }

        let stats = store.quick_stats().unwrap();
        // Since entries have similar timestamps, entries_per_hour should be high
        assert!(stats.entries_per_hour > 0.0);
    }

    #[test]
    fn test_empty_store_statistics() {
        let store = AuditStore::in_memory().unwrap();
        let stats = store.quick_stats().unwrap();

        assert_eq!(stats.total, 0);
        assert!(stats.decisions.is_none());
        assert!(stats.tasks.is_none());
    }
}