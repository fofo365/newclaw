//! Audit log storage with SQLite persistence
//!
//! This module provides:
//! - SQLite-based persistent storage
//! - Log rotation and archiving
//! - Efficient querying with indexes

use rusqlite::{Connection, params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use super::{AuditEntry, AuditEvent, AuditResult, AuditError};

/// Audit store error
#[derive(Debug, thiserror::Error)]
pub enum AuditStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Rotation error: {0}")]
    Rotation(String),
}

impl From<serde_json::Error> for AuditStoreError {
    fn from(e: serde_json::Error) -> Self {
        AuditStoreError::Serialization(e.to_string())
    }
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationConfig {
    /// Maximum log entries before rotation
    pub max_entries: usize,
    /// Maximum age in days before rotation
    pub max_age_days: u32,
    /// Whether to archive rotated logs
    pub archive: bool,
    /// Archive directory (relative to db path)
    pub archive_dir: Option<PathBuf>,
    /// Maximum number of archived files to keep
    pub max_archives: usize,
}

impl Default for RotationConfig {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_age_days: 90,
            archive: true,
            archive_dir: Some(PathBuf::from("audit_archive")),
            max_archives: 12,
        }
    }
}

impl RotationConfig {
    /// Create a new rotation config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max entries
    pub fn with_max_entries(mut self, count: usize) -> Self {
        self.max_entries = count;
        self
    }

    /// Set max age in days
    pub fn with_max_age_days(mut self, days: u32) -> Self {
        self.max_age_days = days;
        self
    }

    /// Enable/disable archiving
    pub fn with_archive(mut self, archive: bool) -> Self {
        self.archive = archive;
        self
    }
}

/// SQLite-based audit log store
pub struct AuditStore {
    /// Database connection
    conn: Arc<Mutex<Connection>>,
    /// Database path
    path: PathBuf,
    /// Rotation config
    rotation_config: RotationConfig,
}

impl AuditStore {
    /// Open or create an audit store
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, AuditStoreError> {
        Self::open_with_config(path, RotationConfig::default())
    }

    /// Open with custom rotation config
    pub fn open_with_config<P: AsRef<Path>>(
        path: P,
        rotation_config: RotationConfig,
    ) -> Result<Self, AuditStoreError> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;

        // Create tables
        Self::create_tables(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
            rotation_config,
        })
    }

    /// Create an in-memory audit store
    pub fn in_memory() -> Result<Self, AuditStoreError> {
        let conn = Connection::open_in_memory()?;
        Self::create_tables(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
            rotation_config: RotationConfig::default(),
        })
    }

    /// Create database tables
    fn create_tables(conn: &Connection) -> Result<(), AuditStoreError> {
        conn.execute_batch(
            r#"
            -- Main audit log table
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'newclaw',
                correlation_id TEXT,
                metadata TEXT
            );

            -- Index for timestamp queries
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);

            -- Index for event type queries
            CREATE INDEX IF NOT EXISTS idx_audit_event_type ON audit_log(event_type);

            -- Index for source queries
            CREATE INDEX IF NOT EXISTS idx_audit_source ON audit_log(source);

            -- Decision-specific indexes (extracted from event_data)
            CREATE INDEX IF NOT EXISTS idx_audit_decision_subject ON audit_log(
                json_extract(event_data, '$.subject_id')
            ) WHERE event_type = 'decision';

            CREATE INDEX IF NOT EXISTS idx_audit_decision_resource ON audit_log(
                json_extract(event_data, '$.resource_id')
            ) WHERE event_type = 'decision';

            -- Archive metadata table
            CREATE TABLE IF NOT EXISTS audit_archive_meta (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                filename TEXT NOT NULL,
                created_at TEXT NOT NULL,
                entries_count INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL
            );

            -- Rotation stats table
            CREATE TABLE IF NOT EXISTS audit_rotation_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_rotation TEXT,
                total_rotations INTEGER DEFAULT 0
            );

            INSERT OR IGNORE INTO audit_rotation_stats (id, last_rotation, total_rotations)
            VALUES (1, NULL, 0);
            "#,
        )?;

        Ok(())
    }

    /// Insert an audit entry
    pub fn insert(&self, entry: &AuditEntry) -> Result<(), AuditStoreError> {
        let conn = self.conn.lock();

        let event_data = serde_json::to_string(&entry.event)?;
        let metadata = if entry.metadata.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&entry.metadata)?)
        };

        conn.execute(
            r#"
            INSERT INTO audit_log (id, timestamp, event_type, event_data, source, correlation_id, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                entry.id.to_string(),
                entry.timestamp.to_rfc3339(),
                entry.event_type(),
                event_data,
                entry.source,
                entry.correlation_id.map(|id| id.to_string()),
                metadata,
            ],
        )?;

        Ok(())
    }

    /// Insert multiple entries in a batch
    pub fn insert_batch(&self, entries: &[AuditEntry]) -> Result<usize, AuditStoreError> {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;

        let mut count = 0;
        for entry in entries {
            let event_data = serde_json::to_string(&entry.event)?;
            let metadata = if entry.metadata.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&entry.metadata)?)
            };

            tx.execute(
                r#"
                INSERT INTO audit_log (id, timestamp, event_type, event_data, source, correlation_id, metadata)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                params![
                    entry.id.to_string(),
                    entry.timestamp.to_rfc3339(),
                    entry.event_type(),
                    event_data,
                    entry.source,
                    entry.correlation_id.map(|id| id.to_string()),
                    metadata,
                ],
            )?;
            count += 1;
        }

        tx.commit()?;
        Ok(count)
    }

    /// Get an entry by ID
    pub fn get(&self, id: &Uuid) -> Result<Option<AuditEntry>, AuditStoreError> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT id, timestamp, event_type, event_data, source, correlation_id, metadata FROM audit_log WHERE id = ?1",
            params![id.to_string()],
            |row| {
                let id_str: String = row.get(0)?;
                let timestamp_str: String = row.get(1)?;
                let event_type: String = row.get(2)?;
                let event_data: String = row.get(3)?;
                let source: String = row.get(4)?;
                let correlation_id: Option<String> = row.get(5)?;
                let metadata: Option<String> = row.get(6)?;

                Ok((id_str, timestamp_str, event_type, event_data, source, correlation_id, metadata))
            },
        ).optional()?;

        match result {
            Some((id_str, timestamp_str, _event_type, event_data, source, correlation_id, metadata)) => {
                let entry = AuditEntry {
                    id: id_str.parse().map_err(|_| AuditStoreError::Serialization(
                        "Invalid UUID".to_string()
                    ))?,
                    timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                        .map_err(|e| AuditStoreError::Serialization(
                            format!("Invalid timestamp: {}", e)
                        ))?
                        .with_timezone(&Utc),
                    event: serde_json::from_str(&event_data)?,
                    source,
                    correlation_id: correlation_id
                        .map(|s| s.parse())
                        .transpose()
                        .map_err(|_| AuditStoreError::Serialization(
                            "Invalid correlation ID".to_string()
                        ))?,
                    metadata: metadata
                        .map(|s| serde_json::from_str(&s))
                        .transpose()?
                        .unwrap_or_default(),
                };
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// Get total entry count
    pub fn count(&self) -> Result<usize, AuditStoreError> {
        let conn = self.conn.lock();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log",
            [],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Get count by event type
    pub fn count_by_type(&self, event_type: &str) -> Result<usize, AuditStoreError> {
        let conn = self.conn.lock();

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE event_type = ?1",
            params![event_type],
            |row| row.get(0),
        )?;

        Ok(count as usize)
    }

    /// Get oldest entry timestamp
    pub fn oldest_timestamp(&self) -> Result<Option<DateTime<Utc>>, AuditStoreError> {
        let conn = self.conn.lock();

        let result: Option<String> = conn.query_row(
            "SELECT MIN(timestamp) FROM audit_log",
            [],
            |row| row.get(0),
        ).optional()?;

        match result {
            Some(s) if !s.is_empty() => {
                let dt = DateTime::parse_from_rfc3339(&s)
                    .map_err(|e| AuditStoreError::Serialization(
                        format!("Invalid timestamp: {}", e)
                    ))?
                    .with_timezone(&Utc);
                Ok(Some(dt))
            }
            _ => Ok(None),
        }
    }

    /// Delete entries older than a date
    pub fn delete_older_than(&self, before: DateTime<Utc>) -> Result<usize, AuditStoreError> {
        let conn = self.conn.lock();

        let count = conn.execute(
            "DELETE FROM audit_log WHERE timestamp < ?1",
            params![before.to_rfc3339()],
        )?;

        Ok(count)
    }

    /// Delete entries by IDs
    pub fn delete_by_ids(&self, ids: &[Uuid]) -> Result<usize, AuditStoreError> {
        if ids.is_empty() {
            return Ok(0);
        }

        let conn = self.conn.lock();
        let mut total_deleted = 0;

        for id in ids {
            let deleted = conn.execute(
                "DELETE FROM audit_log WHERE id = ?1",
                params![id.to_string()],
            )?;
            total_deleted += deleted;
        }

        Ok(total_deleted)
    }

    /// Clear all entries
    pub fn clear(&self) -> Result<(), AuditStoreError> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM audit_log", [])?;
        Ok(())
    }

    /// Check if rotation is needed
    pub fn needs_rotation(&self) -> Result<bool, AuditStoreError> {
        let count = self.count()?;
        if count >= self.rotation_config.max_entries {
            return Ok(true);
        }

        if let Some(oldest) = self.oldest_timestamp()? {
            let max_age = Duration::days(self.rotation_config.max_age_days as i64);
            if (Utc::now() - oldest) > max_age {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Perform log rotation
    pub fn rotate(&self) -> Result<String, AuditStoreError> {
        if !self.rotation_config.archive {
            // Just clear if not archiving
            self.clear()?;
            return Ok("Cleared without archive".to_string());
        }

        let conn = self.conn.lock();

        // Get entries to archive
        let entries: Vec<(String, String)> = conn
            .prepare("SELECT id, event_data FROM audit_log ORDER BY timestamp ASC")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        if entries.is_empty() {
            return Ok("No entries to rotate".to_string());
        }

        // Create archive filename
        let archive_name = format!(
            "audit_{}.jsonl",
            Utc::now().format("%Y%m%d_%H%M%S")
        );

        let archive_path = if let Some(ref archive_dir) = self.rotation_config.archive_dir {
            self.path.parent()
                .ok_or_else(|| AuditStoreError::Rotation("Invalid path".to_string()))?
                .join(archive_dir)
                .join(&archive_name)
        } else {
            self.path.parent()
                .ok_or_else(|| AuditStoreError::Rotation("Invalid path".to_string()))?
                .join(&archive_name)
        };

        // Create archive directory
        if let Some(parent) = archive_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Write archive file (JSONL format)
        let mut file = std::fs::File::create(&archive_path)?;
        for (_, event_data) in &entries {
            use std::io::Write;
            writeln!(file, "{}", event_data)?;
        }

        let archive_size = std::fs::metadata(&archive_path)?.len();
        let entries_count = entries.len();

        // Clear archived entries
        conn.execute("DELETE FROM audit_log", [])?;

        // Record archive metadata
        conn.execute(
            "INSERT INTO audit_archive_meta (filename, created_at, entries_count, size_bytes)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                archive_name,
                Utc::now().to_rfc3339(),
                entries_count as i64,
                archive_size as i64,
            ],
        )?;

        // Update rotation stats
        conn.execute(
            "UPDATE audit_rotation_stats SET last_rotation = ?1, total_rotations = total_rotations + 1 WHERE id = 1",
            params![Utc::now().to_rfc3339()],
        )?;

        // Cleanup old archives if needed
        self.cleanup_old_archives(&conn)?;

        Ok(archive_path.display().to_string())
    }

    /// Clean up old archive files
    fn cleanup_old_archives(&self, conn: &Connection) -> Result<(), AuditStoreError> {
        let max_archives = self.rotation_config.max_archives;

        // Get count of archives
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_archive_meta",
            [],
            |row| row.get(0),
        )?;

        if count as usize <= max_archives {
            return Ok(());
        }

        // Delete oldest archives
        let to_delete = count as usize - max_archives;
        let delete_items: Vec<(i64, String)> = conn
            .prepare("SELECT id, filename FROM audit_archive_meta ORDER BY created_at ASC LIMIT ?1")?
            .query_map(params![to_delete as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for (id, filename) in delete_items {
            // Delete file
            if let Some(archive_dir) = &self.rotation_config.archive_dir {
                let file_path = self.path.parent()
                    .ok_or_else(|| AuditStoreError::Rotation("Invalid path".to_string()))?
                    .join(archive_dir)
                    .join(&filename);
                let _ = std::fs::remove_file(file_path);
            }

            // Delete from metadata
            conn.execute("DELETE FROM audit_archive_meta WHERE id = ?1", params![id])?;
        }

        Ok(())
    }

    /// Get archive list
    pub fn list_archives(&self) -> Result<Vec<ArchiveInfo>, AuditStoreError> {
        let conn = self.conn.lock();

        let archives = conn
            .prepare("SELECT filename, created_at, entries_count, size_bytes FROM audit_archive_meta ORDER BY created_at DESC")?
            .query_map([], |row| {
                Ok(ArchiveInfo {
                    filename: row.get(0)?,
                    created_at: row.get(1)?,
                    entries_count: row.get(2)?,
                    size_bytes: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(archives)
    }

    /// Get rotation statistics
    pub fn rotation_stats(&self) -> Result<RotationStats, AuditStoreError> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT last_rotation, total_rotations FROM audit_rotation_stats WHERE id = 1",
            [],
            |row| {
                Ok(RotationStats {
                    last_rotation: row.get(0)?,
                    total_rotations: row.get(1)?,
                })
            },
        ).optional()?;

        Ok(result.unwrap_or(RotationStats {
            last_rotation: None,
            total_rotations: 0,
        }))
    }

    /// Get the underlying connection for custom queries
    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Get database path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Archive information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveInfo {
    /// Archive filename
    pub filename: String,
    /// Creation timestamp
    pub created_at: String,
    /// Number of entries
    pub entries_count: i64,
    /// File size in bytes
    pub size_bytes: i64,
}

/// Rotation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationStats {
    /// Last rotation timestamp
    pub last_rotation: Option<String>,
    /// Total rotations performed
    pub total_rotations: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_audit_store_in_memory() {
        let store = AuditStore::in_memory().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_insert_and_get() {
        let store = AuditStore::in_memory().unwrap();

        let entry = AuditEntry::from_decision(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Permit",
            "user1",
            "doc1",
            "document",
            "read",
            "PolicyPermit",
            3,
            1,
            vec!["policy1".to_string()],
            100,
        );

        store.insert(&entry).unwrap();
        assert_eq!(store.count().unwrap(), 1);

        let retrieved = store.get(&entry.id).unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, entry.id);
        assert_eq!(retrieved.event_type(), "decision");
    }

    #[test]
    fn test_insert_batch() {
        let store = AuditStore::in_memory().unwrap();

        let entries: Vec<AuditEntry> = (0..10)
            .map(|i| {
                AuditEntry::from_decision(
                    Uuid::new_v4(),
                    Uuid::new_v4(),
                    "Permit",
                    &format!("user{}", i),
                    "doc1",
                    "document",
                    "read",
                    "PolicyPermit",
                    1,
                    1,
                    vec!["policy1".to_string()],
                    100,
                )
            })
            .collect();

        let count = store.insert_batch(&entries).unwrap();
        assert_eq!(count, 10);
        assert_eq!(store.count().unwrap(), 10);
    }

    #[test]
    fn test_count_by_type() {
        let store = AuditStore::in_memory().unwrap();

        // Insert decision entries
        for _ in 0..5 {
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Permit",
                "user1",
                "doc1",
                "document",
                "read",
                "PolicyPermit",
                1, 1, vec![], 100,
            );
            store.insert(&entry).unwrap();
        }

        // Insert task entries
        for _ in 0..3 {
            let entry = AuditEntry::from_task(
                Uuid::new_v4(),
                "task1",
                "cron",
                "completed",
                None, None, None, None,
            );
            store.insert(&entry).unwrap();
        }

        assert_eq!(store.count_by_type("decision").unwrap(), 5);
        assert_eq!(store.count_by_type("task").unwrap(), 3);
    }

    #[test]
    fn test_delete_older_than() {
        let store = AuditStore::in_memory().unwrap();

        let entry = AuditEntry::from_decision(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Permit",
            "user1",
            "doc1",
            "document",
            "read",
            "PolicyPermit",
            1, 1, vec![], 100,
        );

        store.insert(&entry).unwrap();
        assert_eq!(store.count().unwrap(), 1);

        // Delete entries older than 1 second from now
        let future = Utc::now() + Duration::seconds(1);
        let deleted = store.delete_older_than(future).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_clear() {
        let store = AuditStore::in_memory().unwrap();

        for i in 0..5 {
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Permit",
                &format!("user{}", i),
                "doc1",
                "document",
                "read",
                "PolicyPermit",
                1, 1, vec![], 100,
            );
            store.insert(&entry).unwrap();
        }

        assert_eq!(store.count().unwrap(), 5);
        store.clear().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_file_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_audit.db");

        // Create and insert
        {
            let store = AuditStore::open(&db_path).unwrap();
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "Permit",
                "user1",
                "doc1",
                "document",
                "read",
                "PolicyPermit",
                1, 1, vec![], 100,
            );
            store.insert(&entry).unwrap();
        }

        // Reopen and verify
        {
            let store = AuditStore::open(&db_path).unwrap();
            assert_eq!(store.count().unwrap(), 1);
        }
    }

    #[test]
    fn test_rotation_config() {
        let config = RotationConfig::new()
            .with_max_entries(100)
            .with_max_age_days(30)
            .with_archive(false);

        assert_eq!(config.max_entries, 100);
        assert_eq!(config.max_age_days, 30);
        assert!(!config.archive);
    }

    #[test]
    fn test_metadata_and_correlation() {
        let store = AuditStore::in_memory().unwrap();

        let correlation_id = Uuid::new_v4();
        let entry = AuditEntry::system("test", "info", "Test event", None)
            .with_correlation_id(correlation_id)
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");

        store.insert(&entry).unwrap();

        let retrieved = store.get(&entry.id).unwrap().unwrap();
        assert_eq!(retrieved.correlation_id, Some(correlation_id));
        assert_eq!(retrieved.metadata.get("key1"), Some(&"value1".to_string()));
    }
}