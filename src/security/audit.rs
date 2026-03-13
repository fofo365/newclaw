// Audit Logging Module
use super::AgentId;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: i64,
    pub timestamp_iso: String,
    pub agent_id: AgentId,
    pub action: String,
    pub resource: String,
    pub result: AuditResult,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failure,
    Denied,
}

/// Audit storage configuration
#[derive(Debug, Clone)]
pub enum AuditStorage {
    File(PathBuf),
    Database(String), // Connection string
    Memory(Arc<RwLock<Vec<AuditEntry>>>),
}

/// Audit logger
pub struct AuditLogger {
    storage: AuditStorage,
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger with file storage
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            storage: AuditStorage::File(path.into()),
            enabled: true,
        }
    }

    /// Create a new audit logger with database storage
    pub fn database(connection_string: String) -> Self {
        Self {
            storage: AuditStorage::Database(connection_string),
            enabled: true,
        }
    }

    /// Create an in-memory audit logger (for testing)
    pub fn memory() -> Self {
        Self {
            storage: AuditStorage::Memory(Arc::new(RwLock::new(Vec::new()))),
            enabled: true,
        }
    }

    /// Enable or disable logging
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Log an audit entry
    pub async fn log(
        &self,
        agent_id: AgentId,
        action: String,
        resource: String,
        result: AuditResult,
        details: Option<serde_json::Value>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<String> {
        if !self.enabled {
            return Ok("audit-disabled".to_string());
        }

        let now = Utc::now();
        let entry = AuditEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: now.timestamp(),
            timestamp_iso: now.to_rfc3339(),
            agent_id,
            action,
            resource,
            result,
            details,
            ip_address,
            user_agent,
        };

        let id = entry.id.clone();

        match &self.storage {
            AuditStorage::File(path) => {
                self.log_to_file(path, &entry).await?;
            }
            AuditStorage::Database(_conn_str) => {
                // TODO: Implement database storage
                self.log_to_file(&PathBuf::from("audit.log"), &entry).await?;
            }
            AuditStorage::Memory(entries) => {
                entries.write().await.push(entry);
            }
        }

        Ok(id)
    }

    /// Log success
    pub async fn log_success(
        &self,
        agent_id: AgentId,
        action: String,
        resource: String,
        details: Option<serde_json::Value>,
    ) -> Result<String> {
        self.log(
            agent_id,
            action,
            resource,
            AuditResult::Success,
            details,
            None,
            None,
        )
        .await
    }

    /// Log failure
    pub async fn log_failure(
        &self,
        agent_id: AgentId,
        action: String,
        resource: String,
        details: Option<serde_json::Value>,
    ) -> Result<String> {
        self.log(
            agent_id,
            action,
            resource,
            AuditResult::Failure,
            details,
            None,
            None,
        )
        .await
    }

    /// Log denied access
    pub async fn log_denied(
        &self,
        agent_id: AgentId,
        action: String,
        resource: String,
        details: Option<serde_json::Value>,
    ) -> Result<String> {
        self.log(
            agent_id,
            action,
            resource,
            AuditResult::Denied,
            details,
            None,
            None,
        )
        .await
    }

    /// Log to file
    async fn log_to_file(&self, path: &PathBuf, entry: &AuditEntry) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        let json = serde_json::to_string(entry)?;
        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    /// Query audit entries (for in-memory storage only)
    pub async fn query(&self, filter: AuditFilter) -> Result<Vec<AuditEntry>> {
        match &self.storage {
            AuditStorage::Memory(entries) => {
                let entries = entries.read().await;
                let filtered: Vec<AuditEntry> = entries
                    .iter()
                    .filter(|entry| {
                        if let Some(ref agent_id) = filter.agent_id {
                            if entry.agent_id != *agent_id {
                                return false;
                            }
                        }
                        if let Some(ref action) = filter.action {
                            if entry.action != *action {
                                return false;
                            }
                        }
                        if let Some(start_time) = filter.start_time {
                            if entry.timestamp < start_time {
                                return false;
                            }
                        }
                        if let Some(end_time) = filter.end_time {
                            if entry.timestamp > end_time {
                                return false;
                            }
                        }
                        if let Some(ref result) = filter.result {
                            if std::mem::discriminant(&entry.result) != std::mem::discriminant(result) {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();
                Ok(filtered)
            }
            _ => Err(anyhow!("Query only supported for in-memory storage")),
        }
    }

    /// Read audit log from file
    pub async fn read_from_file(&self, path: &PathBuf) -> Result<Vec<AuditEntry>> {
        let content = tokio::fs::read_to_string(path).await?;
        let entries: Vec<AuditEntry> = content
            .lines()
            .filter(|line| !line.is_empty())
            .map(serde_json::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }
}

/// Audit query filter
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub agent_id: Option<AgentId>,
    pub action: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub result: Option<AuditResult>,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn action(mut self, action: String) -> Self {
        self.action = Some(action);
        self
    }

    pub fn time_range(mut self, start: i64, end: i64) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn result(mut self, result: AuditResult) -> Self {
        self.result = Some(result);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let logger = AuditLogger::memory();
        
        let id = logger
            .log_success(
                "agent-1".to_string(),
                "send_message".to_string(),
                "message:123".to_string(),
                None,
            )
            .await
            .unwrap();

        assert!(!id.is_empty());

        let filter = AuditFilter::new().agent_id("agent-1".to_string());
        let entries = logger.query(filter).await.unwrap();
        
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].agent_id, "agent-1");
        assert_eq!(entries[0].action, "send_message");
    }

    #[tokio::test]
    async fn test_filter() {
        let logger = AuditLogger::memory();
        
        logger
            .log_success("agent-1".to_string(), "action1".to_string(), "res1".to_string(), None)
            .await
            .unwrap();
        logger
            .log_success("agent-2".to_string(), "action2".to_string(), "res2".to_string(), None)
            .await
            .unwrap();
        logger
            .log_failure("agent-1".to_string(), "action3".to_string(), "res3".to_string(), None)
            .await
            .unwrap();

        let filter = AuditFilter::new().agent_id("agent-1".to_string());
        let entries = logger.query(filter).await.unwrap();
        assert_eq!(entries.len(), 2);

        let filter = AuditFilter::new().result(AuditResult::Failure);
        let entries = logger.query(filter).await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_disabled() {
        let mut logger = AuditLogger::memory();
        logger.set_enabled(false);
        
        logger
            .log_success("agent-1".to_string(), "action".to_string(), "res".to_string(), None)
            .await
            .unwrap();

        let entries = logger.query(AuditFilter::new()).await.unwrap();
        assert_eq!(entries.len(), 0);
    }
}
