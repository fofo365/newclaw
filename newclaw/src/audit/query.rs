//! Advanced query capabilities for audit logs
//!
//! This module provides:
//! - Query builder pattern for complex queries
//! - Filters for various criteria
//! - Time range queries
//! - Pagination and sorting

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use rusqlite::{Connection, params};

use super::{AuditEntry, AuditEvent, AuditStore, AuditResult, AuditError};
use crate::audit::store::AuditStoreError;

/// Sort order for query results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Ascending (oldest first)
    Asc,
    /// Descending (newest first)
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Desc
    }
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Page number (1-based)
    pub page: usize,
    /// Items per page
    pub page_size: usize,
}

impl Pagination {
    /// Create new pagination
    pub fn new(page: usize, page_size: usize) -> Self {
        Self { page, page_size }
    }

    /// Get offset for SQL query
    pub fn offset(&self) -> usize {
        (self.page.saturating_sub(1)) * self.page_size
    }

    /// Get limit for SQL query
    pub fn limit(&self) -> usize {
        self.page_size
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 100,
        }
    }
}

/// Audit log filter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    /// Filter by event types
    pub event_types: Vec<String>,
    /// Filter by subject ID (for decision events)
    pub subject_id: Option<String>,
    /// Filter by resource ID (for decision events)
    pub resource_id: Option<String>,
    /// Filter by resource type
    pub resource_type: Option<String>,
    /// Filter by action
    pub action: Option<String>,
    /// Filter by decision result (Permit/Deny/etc.)
    pub decision: Option<String>,
    /// Filter by source
    pub source: Option<String>,
    /// Filter by correlation ID
    pub correlation_id: Option<Uuid>,
    /// Filter by task ID (for task events)
    pub task_id: Option<Uuid>,
    /// Filter by task status
    pub task_status: Option<String>,
    /// Filter by config scope
    pub config_scope: Option<String>,
    /// Filter by config key
    pub config_key: Option<String>,
    /// Filter by system event level
    pub system_level: Option<String>,
    /// Custom filters (field -> value)
    pub custom: Vec<(String, String)>,
}

impl AuditFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Add event type filter
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_types.push(event_type.into());
        self
    }

    /// Set subject ID filter
    pub fn with_subject(mut self, subject_id: impl Into<String>) -> Self {
        self.subject_id = Some(subject_id.into());
        self
    }

    /// Set resource ID filter
    pub fn with_resource(mut self, resource_id: impl Into<String>) -> Self {
        self.resource_id = Some(resource_id.into());
        self
    }

    /// Set resource type filter
    pub fn with_resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Set action filter
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Set decision filter
    pub fn with_decision(mut self, decision: impl Into<String>) -> Self {
        self.decision = Some(decision.into());
        self
    }

    /// Set source filter
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Set correlation ID filter
    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Set task ID filter
    pub fn with_task_id(mut self, id: Uuid) -> Self {
        self.task_id = Some(id);
        self
    }

    /// Set task status filter
    pub fn with_task_status(mut self, status: impl Into<String>) -> Self {
        self.task_status = Some(status.into());
        self
    }

    /// Add custom filter
    pub fn with_custom(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.push((field.into(), value.into()));
        self
    }

    /// Check if any filters are applied
    pub fn is_empty(&self) -> bool {
        self.event_types.is_empty()
            && self.subject_id.is_none()
            && self.resource_id.is_none()
            && self.resource_type.is_none()
            && self.action.is_none()
            && self.decision.is_none()
            && self.source.is_none()
            && self.correlation_id.is_none()
            && self.task_id.is_none()
            && self.task_status.is_none()
            && self.config_scope.is_none()
            && self.config_key.is_none()
            && self.system_level.is_none()
            && self.custom.is_empty()
    }
}

/// Audit query with all parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Time range start
    pub start_time: Option<DateTime<Utc>>,
    /// Time range end
    pub end_time: Option<DateTime<Utc>>,
    /// Filters
    pub filter: AuditFilter,
    /// Sort order
    pub sort_order: SortOrder,
    /// Pagination
    pub pagination: Pagination,
    /// Include total count in result
    pub include_total: bool,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            start_time: None,
            end_time: None,
            filter: AuditFilter::default(),
            sort_order: SortOrder::Desc,
            pagination: Pagination::default(),
            include_total: true,
        }
    }
}

impl AuditQuery {
    /// Create a new query
    pub fn new() -> Self {
        Self::default()
    }

    /// Set time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Set start time only
    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self
    }

    /// Set end time only
    pub fn with_end_time(mut self, end: DateTime<Utc>) -> Self {
        self.end_time = Some(end);
        self
    }

    /// Set filter
    pub fn with_filter(mut self, filter: AuditFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set sort order
    pub fn with_sort_order(mut self, order: SortOrder) -> Self {
        self.sort_order = order;
        self
    }

    /// Set pagination
    pub fn with_pagination(mut self, page: usize, page_size: usize) -> Self {
        self.pagination = Pagination::new(page, page_size);
        self
    }

    /// Set include total
    pub fn with_include_total(mut self, include: bool) -> Self {
        self.include_total = include;
        self
    }
}

/// Query builder for fluent API
#[derive(Debug, Clone)]
pub struct AuditQueryBuilder {
    query: AuditQuery,
}

impl AuditQueryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            query: AuditQuery::new(),
        }
    }

    /// Set time range
    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.query.start_time = Some(start);
        self.query.end_time = Some(end);
        self
    }

    /// Set start time
    pub fn with_start_time(mut self, start: DateTime<Utc>) -> Self {
        self.query.start_time = Some(start);
        self
    }

    /// Set end time
    pub fn with_end_time(mut self, end: DateTime<Utc>) -> Self {
        self.query.end_time = Some(end);
        self
    }

    /// Filter by event type
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.query.filter.event_types.push(event_type.into());
        self
    }

    /// Filter by subject ID
    pub fn with_subject(mut self, subject_id: impl Into<String>) -> Self {
        self.query.filter.subject_id = Some(subject_id.into());
        self
    }

    /// Filter by resource ID
    pub fn with_resource(mut self, resource_id: impl Into<String>) -> Self {
        self.query.filter.resource_id = Some(resource_id.into());
        self
    }

    /// Filter by resource type
    pub fn with_resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.query.filter.resource_type = Some(resource_type.into());
        self
    }

    /// Filter by action
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.query.filter.action = Some(action.into());
        self
    }

    /// Filter by decision
    pub fn with_decision(mut self, decision: impl Into<String>) -> Self {
        self.query.filter.decision = Some(decision.into());
        self
    }

    /// Filter by source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.query.filter.source = Some(source.into());
        self
    }

    /// Filter by correlation ID
    pub fn with_correlation_id(mut self, id: Uuid) -> Self {
        self.query.filter.correlation_id = Some(id);
        self
    }

    /// Filter by task status
    pub fn with_task_status(mut self, status: impl Into<String>) -> Self {
        self.query.filter.task_status = Some(status.into());
        self
    }

    /// Filter by task ID
    pub fn with_task_id(mut self, id: Uuid) -> Self {
        self.query.filter.task_id = Some(id);
        self
    }

    /// Set sort order
    pub fn sort_by(mut self, order: SortOrder) -> Self {
        self.query.sort_order = order;
        self
    }

    /// Set pagination
    pub fn paginate(mut self, page: usize, page_size: usize) -> Self {
        self.query.pagination = Pagination::new(page, page_size);
        self
    }

    /// Build the query
    pub fn build(self) -> AuditQuery {
        self.query
    }
}

impl Default for AuditQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Query result with pagination info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryResult {
    /// Matching entries
    pub entries: Vec<AuditEntry>,
    /// Total count (if requested)
    pub total: Option<usize>,
    /// Current page
    pub page: usize,
    /// Page size
    pub page_size: usize,
    /// Total pages (if total is known)
    pub total_pages: Option<usize>,
}

impl AuditQueryResult {
    /// Check if there are more pages
    pub fn has_more(&self) -> bool {
        match self.total_pages {
            Some(pages) => self.page < pages,
            None => self.entries.len() == self.page_size,
        }
    }
}

/// Query execution extension for AuditStore
impl AuditStore {
    /// Execute a query
    pub fn query(&self, query: &AuditQuery) -> AuditResult<AuditQueryResult> {
        let conn_arc = self.connection();
        let conn = conn_arc.lock();

        // Build SQL query
        let (sql, count_sql, params) = Self::build_query_sql(query)?;

        // Get total count if requested
        let total = if query.include_total {
            Some(Self::execute_count(&conn, &count_sql, &params)?)
        } else {
            None
        };

        // Execute query
        let entries = Self::execute_query(&conn, &sql, &params)?;

        let total_pages = total.map(|t| {
            (t + query.pagination.page_size - 1) / query.pagination.page_size
        });

        Ok(AuditQueryResult {
            entries,
            total,
            page: query.pagination.page,
            page_size: query.pagination.page_size,
            total_pages,
        })
    }

    /// Build SQL query from AuditQuery
    fn build_query_sql(query: &AuditQuery) -> AuditResult<(String, String, Vec<String>)> {
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<String> = Vec::new();

        // Time range
        if let Some(start) = &query.start_time {
            conditions.push("timestamp >= ?".to_string());
            params.push(start.to_rfc3339());
        }
        if let Some(end) = &query.end_time {
            conditions.push("timestamp <= ?".to_string());
            params.push(end.to_rfc3339());
        }

        // Event types
        if !query.filter.event_types.is_empty() {
            let placeholders: Vec<&str> = query.filter.event_types.iter().map(|_| "?").collect();
            conditions.push(format!("event_type IN ({})", placeholders.join(",")));
            for et in &query.filter.event_types {
                params.push(et.clone());
            }
        }

        // Source
        if let Some(ref source) = query.filter.source {
            conditions.push("source = ?".to_string());
            params.push(source.clone());
        }

        // Correlation ID
        if let Some(ref id) = query.filter.correlation_id {
            conditions.push("correlation_id = ?".to_string());
            params.push(id.to_string());
        }

        // JSON field filters
        let json_filters = Self::build_json_filters(query);
        for (condition, value) in json_filters {
            conditions.push(condition);
            params.push(value);
        }

        // Custom filters
        for (field, value) in &query.filter.custom {
            conditions.push(format!(
                "json_extract(event_data, '$.{}') = ?",
                field
            ));
            params.push(value.clone());
        }

        // Build WHERE clause
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        // Build ORDER BY
        let order = match query.sort_order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };

        // Build pagination
        let offset = query.pagination.offset();
        let limit = query.pagination.limit();

        let sql = format!(
            "SELECT id, timestamp, event_type, event_data, source, correlation_id, metadata \
             FROM audit_log {} ORDER BY timestamp {} LIMIT {} OFFSET {}",
            where_clause, order, limit, offset
        );

        let count_sql = format!(
            "SELECT COUNT(*) FROM audit_log {}",
            where_clause
        );

        Ok((sql, count_sql, params))
    }

    /// Build JSON field filters
    fn build_json_filters(query: &AuditQuery) -> Vec<(String, String)> {
        let mut filters = Vec::new();

        // Decision filters
        if let Some(ref subject) = query.filter.subject_id {
            filters.push((
                "event_type = 'decision' AND json_extract(event_data, '$.subject_id') = ?".to_string(),
                subject.clone(),
            ));
        }

        if let Some(ref resource) = query.filter.resource_id {
            filters.push((
                "event_type = 'decision' AND json_extract(event_data, '$.resource_id') = ?".to_string(),
                resource.clone(),
            ));
        }

        if let Some(ref resource_type) = query.filter.resource_type {
            filters.push((
                "event_type = 'decision' AND json_extract(event_data, '$.resource_type') = ?".to_string(),
                resource_type.clone(),
            ));
        }

        if let Some(ref action) = query.filter.action {
            filters.push((
                "event_type = 'decision' AND json_extract(event_data, '$.action') = ?".to_string(),
                action.clone(),
            ));
        }

        if let Some(ref decision) = query.filter.decision {
            filters.push((
                "event_type = 'decision' AND json_extract(event_data, '$.decision') = ?".to_string(),
                decision.clone(),
            ));
        }

        // Task filters
        if let Some(ref task_id) = query.filter.task_id {
            filters.push((
                "event_type = 'task' AND json_extract(event_data, '$.task_id') = ?".to_string(),
                task_id.to_string(),
            ));
        }

        if let Some(ref status) = query.filter.task_status {
            filters.push((
                "event_type = 'task' AND json_extract(event_data, '$.status') = ?".to_string(),
                status.clone(),
            ));
        }

        // Config filters
        if let Some(ref scope) = query.filter.config_scope {
            filters.push((
                "event_type = 'config' AND json_extract(event_data, '$.scope') = ?".to_string(),
                scope.clone(),
            ));
        }

        if let Some(ref key) = query.filter.config_key {
            filters.push((
                "event_type = 'config' AND json_extract(event_data, '$.key') = ?".to_string(),
                key.clone(),
            ));
        }

        // System filters
        if let Some(ref level) = query.filter.system_level {
            filters.push((
                "event_type = 'system' AND json_extract(event_data, '$.level') = ?".to_string(),
                level.clone(),
            ));
        }

        filters
    }

    /// Execute count query
    fn execute_count(conn: &Connection, sql: &str, params: &[String]) -> AuditResult<usize> {
        let count: i64 = if params.is_empty() {
            conn.query_row(sql, [], |row| row.get(0))
                .map_err(|e| AuditError::Database(e.to_string()))?
        } else {
            // Build params dynamically
            match params.len() {
                1 => conn.query_row(sql, params![&params[0]], |row| row.get(0)),
                2 => conn.query_row(sql, params![&params[0], &params[1]], |row| row.get(0)),
                3 => conn.query_row(sql, params![&params[0], &params[1], &params[2]], |row| row.get(0)),
                4 => conn.query_row(sql, params![&params[0], &params[1], &params[2], &params[3]], |row| row.get(0)),
                5 => conn.query_row(sql, params![&params[0], &params[1], &params[2], &params[3], &params[4]], |row| row.get(0)),
                _ => conn.query_row(sql, params![&params[0], &params[1], &params[2], &params[3], &params[4], &params[5]], |row| row.get(0)),
            }.map_err(|e| AuditError::Database(e.to_string()))?
        };

        Ok(count as usize)
    }

    /// Execute query and parse results
    fn execute_query(conn: &Connection, sql: &str, params: &[String]) -> AuditResult<Vec<AuditEntry>> {
        let mut stmt = conn.prepare(sql)
            .map_err(|e| AuditError::Database(e.to_string()))?;

        let entries = if params.is_empty() {
            stmt.query_map([], Self::parse_row)
                .map_err(|e| AuditError::Database(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AuditError::Database(e.to_string()))?
        } else {
            // Build params dynamically and collect
            match params.len() {
                1 => {
                    let rows = stmt.query_map(params![&params[0]], Self::parse_row)
                        .map_err(|e| AuditError::Database(e.to_string()))?;
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| AuditError::Database(e.to_string()))?
                }
                2 => {
                    let rows = stmt.query_map(params![&params[0], &params[1]], Self::parse_row)
                        .map_err(|e| AuditError::Database(e.to_string()))?;
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| AuditError::Database(e.to_string()))?
                }
                3 => {
                    let rows = stmt.query_map(params![&params[0], &params[1], &params[2]], Self::parse_row)
                        .map_err(|e| AuditError::Database(e.to_string()))?;
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| AuditError::Database(e.to_string()))?
                }
                4 => {
                    let rows = stmt.query_map(params![&params[0], &params[1], &params[2], &params[3]], Self::parse_row)
                        .map_err(|e| AuditError::Database(e.to_string()))?;
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| AuditError::Database(e.to_string()))?
                }
                5 => {
                    let rows = stmt.query_map(params![&params[0], &params[1], &params[2], &params[3], &params[4]], Self::parse_row)
                        .map_err(|e| AuditError::Database(e.to_string()))?;
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| AuditError::Database(e.to_string()))?
                }
                _ => {
                    let rows = stmt.query_map(params![&params[0], &params[1], &params[2], &params[3], &params[4], &params[5]], Self::parse_row)
                        .map_err(|e| AuditError::Database(e.to_string()))?;
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|e| AuditError::Database(e.to_string()))?
                }
            }
        };

        Ok(entries)
    }

    /// Parse a row into AuditEntry
    fn parse_row(row: &rusqlite::Row) -> Result<AuditEntry, rusqlite::Error> {
        let id_str: String = row.get(0)?;
        let timestamp_str: String = row.get(1)?;
        let _event_type: String = row.get(2)?;
        let event_data: String = row.get(3)?;
        let source: String = row.get(4)?;
        let correlation_id: Option<String> = row.get(5)?;
        let metadata: Option<String> = row.get(6)?;

        let id = id_str.parse::<Uuid>()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|_| rusqlite::Error::InvalidQuery)?
            .with_timezone(&Utc);

        let event: AuditEvent = serde_json::from_str(&event_data)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let correlation_id = correlation_id
            .map(|s| s.parse::<Uuid>())
            .transpose()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let metadata = metadata
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|_| rusqlite::Error::InvalidQuery)?
            .unwrap_or_default();

        Ok(AuditEntry {
            id,
            timestamp,
            event,
            source,
            correlation_id,
            metadata,
        })
    }

    /// Query by subject ID (convenience method)
    pub fn query_by_subject(&self, subject_id: &str) -> AuditResult<Vec<AuditEntry>> {
        let query = AuditQueryBuilder::new()
            .with_subject(subject_id)
            .build();

        Ok(self.query(&query)?.entries)
    }

    /// Query by resource ID (convenience method)
    pub fn query_by_resource(&self, resource_id: &str) -> AuditResult<Vec<AuditEntry>> {
        let query = AuditQueryBuilder::new()
            .with_resource(resource_id)
            .build();

        Ok(self.query(&query)?.entries)
    }

    /// Query by time range (convenience method)
    pub fn query_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AuditResult<Vec<AuditEntry>> {
        let query = AuditQueryBuilder::new()
            .with_time_range(start, end)
            .build();

        Ok(self.query(&query)?.entries)
    }

    /// Query recent entries (convenience method)
    pub fn query_recent(&self, count: usize) -> AuditResult<Vec<AuditEntry>> {
        let query = AuditQuery {
            pagination: Pagination::new(1, count),
            include_total: false,
            ..Default::default()
        };

        Ok(self.query(&query)?.entries)
    }

    /// Query denied decisions (convenience method)
    pub fn query_denied(&self) -> AuditResult<Vec<AuditEntry>> {
        let query = AuditQueryBuilder::new()
            .with_event_type("decision")
            .with_decision("Deny")
            .build();

        Ok(self.query(&query)?.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_store() -> AuditStore {
        let store = AuditStore::in_memory().unwrap();

        // Insert test entries
        for i in 0..10 {
            let entry = AuditEntry::from_decision(
                Uuid::new_v4(),
                Uuid::new_v4(),
                if i % 2 == 0 { "Permit" } else { "Deny" },
                &format!("user{}", i % 3),
                &format!("doc{}", i),
                "document",
                if i % 3 == 0 { "read" } else if i % 3 == 1 { "write" } else { "delete" },
                "Policy",
                1, 1, vec![], 100,
            );
            store.insert(&entry).unwrap();
        }

        // Insert task entries
        for i in 0..5 {
            let entry = AuditEntry::from_task(
                Uuid::new_v4(),
                &format!("task{}", i),
                "cron",
                if i % 2 == 0 { "completed" } else { "failed" },
                None, None, None, None,
            );
            store.insert(&entry).unwrap();
        }

        store
    }

    #[test]
    fn test_pagination() {
        let pagination = Pagination::new(2, 10);
        assert_eq!(pagination.offset(), 10);
        assert_eq!(pagination.limit(), 10);
    }

    #[test]
    fn test_filter_builder() {
        let filter = AuditFilter::new()
            .with_event_type("decision")
            .with_subject("user1")
            .with_resource("doc1")
            .with_decision("Permit");

        assert_eq!(filter.event_types, vec!["decision"]);
        assert_eq!(filter.subject_id, Some("user1".to_string()));
        assert_eq!(filter.resource_id, Some("doc1".to_string()));
        assert_eq!(filter.decision, Some("Permit".to_string()));
    }

    #[test]
    fn test_query_builder() {
        let start = Utc::now() - chrono::Duration::hours(1);
        let end = Utc::now();

        let query = AuditQueryBuilder::new()
            .with_time_range(start, end)
            .with_event_type("decision")
            .with_subject("user1")
            .sort_by(SortOrder::Asc)
            .paginate(1, 50)
            .build();

        assert_eq!(query.start_time, Some(start));
        assert_eq!(query.end_time, Some(end));
        assert_eq!(query.filter.event_types, vec!["decision"]);
        assert_eq!(query.sort_order, SortOrder::Asc);
        assert_eq!(query.pagination.page, 1);
    }

    #[test]
    fn test_query_all() {
        let store = setup_store();

        let query = AuditQuery::new();
        let result = store.query(&query).unwrap();

        assert_eq!(result.entries.len(), 15);
        assert_eq!(result.total, Some(15));
    }

    #[test]
    fn test_query_by_event_type() {
        let store = setup_store();

        let query = AuditQueryBuilder::new()
            .with_event_type("decision")
            .build();

        let result = store.query(&query).unwrap();
        assert_eq!(result.entries.len(), 10);
    }

    #[test]
    fn test_query_by_subject() {
        let store = setup_store();

        let result = store.query_by_subject("user0").unwrap();
        assert_eq!(result.len(), 4); // 0, 3, 6, 9

        let result = store.query_by_subject("user1").unwrap();
        assert_eq!(result.len(), 3); // 1, 4, 7
    }

    #[test]
    fn test_query_by_decision() {
        let store = setup_store();

        let query = AuditQueryBuilder::new()
            .with_event_type("decision")
            .with_decision("Permit")
            .build();

        let result = store.query(&query).unwrap();
        assert_eq!(result.entries.len(), 5);

        let denied = store.query_denied().unwrap();
        assert_eq!(denied.len(), 5);
    }

    #[test]
    fn test_query_pagination() {
        let store = setup_store();

        let query = AuditQueryBuilder::new()
            .paginate(1, 5)
            .build();

        let result = store.query(&query).unwrap();
        assert_eq!(result.entries.len(), 5);
        assert_eq!(result.page, 1);
        assert!(result.has_more());

        let query = AuditQueryBuilder::new()
            .paginate(3, 5)
            .build();

        let result = store.query(&query).unwrap();
        assert_eq!(result.entries.len(), 5);
        assert_eq!(result.page, 3);
        assert!(!result.has_more());
    }

    #[test]
    fn test_query_recent() {
        let store = setup_store();

        let entries = store.query_recent(5).unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn test_query_by_task_status() {
        let store = setup_store();

        let query = AuditQueryBuilder::new()
            .with_event_type("task")
            .with_task_status("completed")
            .build();

        let result = store.query(&query).unwrap();
        assert_eq!(result.entries.len(), 3); // 0, 2, 4
    }

    #[test]
    fn test_query_sort_order() {
        let store = setup_store();

        // Descending (default)
        let query_desc = AuditQueryBuilder::new()
            .sort_by(SortOrder::Desc)
            .paginate(1, 100)
            .build();

        let result_desc = store.query(&query_desc).unwrap();

        // Ascending
        let query_asc = AuditQueryBuilder::new()
            .sort_by(SortOrder::Asc)
            .paginate(1, 100)
            .build();

        let result_asc = store.query(&query_asc).unwrap();

        // First entry in desc should be last in asc
        assert_eq!(result_desc.entries[0].id, result_asc.entries[14].id);
    }

    #[test]
    fn test_filter_is_empty() {
        let empty = AuditFilter::new();
        assert!(empty.is_empty());

        let not_empty = AuditFilter::new()
            .with_subject("user1");
        assert!(!not_empty.is_empty());
    }

    #[test]
    fn test_query_result_has_more() {
        let result = AuditQueryResult {
            entries: vec![],
            total: Some(100),
            page: 1,
            page_size: 10,
            total_pages: Some(10),
        };
        assert!(result.has_more());

        let result = AuditQueryResult {
            entries: vec![],
            total: Some(100),
            page: 10,
            page_size: 10,
            total_pages: Some(10),
        };
        assert!(!result.has_more());
    }
}