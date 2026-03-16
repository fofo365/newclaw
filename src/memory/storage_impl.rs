// Memory Storage Implementation - v0.7.0
//
// SQLite 实现支持多层隔离：
// - 用户 (user_id)
// - 通道 (channel)
// - Agent (agent_id)
// - 命名空间 (namespace)

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use rusqlite::{Connection, OptionalExtension};
use tokio::sync::Mutex;

use super::storage::{MemoryStorage, MemoryScope, HybridSearchConfig, HybridSearchResult, StorageStats, StorageConfig};
use super::shared::{MemoryEntry, MemoryType, UserId};

/// SQLite 记忆存储实现
pub struct SQLiteMemoryStorage {
    conn: Arc<Mutex<Connection>>,
    config: StorageConfig,
}

impl SQLiteMemoryStorage {
    /// 创建新的 SQLite 存储
    pub fn new(config: StorageConfig) -> Result<Self> {
        if config.db_path != PathBuf::from(":memory:") {
            if let Some(parent) = config.db_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| "Failed to create database directory")?;
            }
        }
        
        let conn = Connection::open(&config.db_path)
            .with_context(|| "Failed to open database")?;
        
        // 初始化表（同步操作）- v0.7.0 支持多层隔离
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                channel TEXT NOT NULL DEFAULT 'global',
                agent_id TEXT,
                namespace TEXT,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                importance REAL DEFAULT 0.5,
                source_agent TEXT,
                tags TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_memories_user ON memories(user_id);
            CREATE INDEX IF NOT EXISTS idx_memories_channel ON memories(channel);
            CREATE INDEX IF NOT EXISTS idx_memories_agent ON memories(agent_id);
            CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
            CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type);
            CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
            
            -- 多层隔离复合索引
            CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(user_id, channel, agent_id, namespace);
            
            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                id,
                content,
                tags,
                tokenize='porter unicode61'
            );
            "#,
        ).with_context(|| "Failed to initialize tables")?;
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            config,
        })
    }
    
    fn memory_type_to_string(mt: &MemoryType) -> &'static str {
        match mt {
            MemoryType::Conversation => "conversation",
            MemoryType::Task => "task",
            MemoryType::Preference => "preference",
            MemoryType::Fact => "fact",
            MemoryType::Skill => "skill",
            MemoryType::Context => "context",
        }
    }
    
    fn string_to_memory_type(s: &str) -> MemoryType {
        match s {
            "conversation" => MemoryType::Conversation,
            "task" => MemoryType::Task,
            "preference" => MemoryType::Preference,
            "fact" => MemoryType::Fact,
            "skill" => MemoryType::Skill,
            "context" => MemoryType::Context,
            _ => MemoryType::Fact,
        }
    }
    
    /// 从数据库行解析 MemoryEntry
    fn parse_memory_entry(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let created_at_str: String = row.get(11)?;
        let last_accessed_str: String = row.get(12)?;
        
        Ok(MemoryEntry {
            id: row.get(0)?,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_accessed: chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            memory_type: Self::string_to_memory_type(&row.get::<_, String>(5)?),
            importance: row.get(7)?,
            content: row.get(6)?,
            metadata: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
            source_agent: row.get(8)?,
            tags: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
        })
    }
}

#[async_trait]
impl MemoryStorage for SQLiteMemoryStorage {
    /// 存储记忆条目（带隔离维度）
    async fn store_with_scope(&self, entry: &MemoryEntry, scope: &MemoryScope) -> Result<String> {
        let conn = self.conn.lock().await;
        
        let memory_type = Self::memory_type_to_string(&entry.memory_type);
        let tags = serde_json::to_string(&entry.tags)?;
        let metadata = serde_json::to_string(&entry.metadata)?;
        let agent_id = scope.agent_id.as_deref().unwrap_or("");
        let namespace = scope.namespace.as_deref().unwrap_or("");
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO memories 
            (id, user_id, channel, agent_id, namespace, memory_type, content, importance, source_agent, tags, metadata, created_at, last_accessed)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            rusqlite::params![
                entry.id,
                scope.user_id,
                scope.channel,
                agent_id,
                namespace,
                memory_type,
                entry.content,
                entry.importance,
                entry.source_agent,
                tags,
                metadata,
                entry.created_at.to_rfc3339(),
                entry.last_accessed.to_rfc3339(),
            ],
        ).with_context(|| "Failed to insert memory")?;
        
        conn.execute(
            "INSERT INTO memories_fts (id, content, tags) VALUES (?1, ?2, ?3)",
            rusqlite::params![entry.id, entry.content, tags],
        ).ok();
        
        Ok(entry.id.clone())
    }
    
    /// 获取记忆条目
    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().await;
        
        let result = conn.query_row(
            "SELECT id, user_id, channel, agent_id, namespace, memory_type, content, importance, source_agent, tags, metadata, created_at, last_accessed 
             FROM memories WHERE id = ?1",
            rusqlite::params![id],
            |row| Self::parse_memory_entry(row),
        ).optional()?;
        
        Ok(result)
    }
    
    /// 删除记忆条目
    async fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![id])?;
        conn.execute("DELETE FROM memories_fts WHERE id = ?1", rusqlite::params![id]).ok();
        Ok(())
    }
    
    /// 混合搜索（带隔离）
    async fn search_hybrid_with_scope(
        &self, 
        query: &str, 
        config: &HybridSearchConfig,
        scope: &MemoryScope
    ) -> Result<Vec<HybridSearchResult>> {
        // 先用 BM25 搜索
        let bm25_results = self.search_bm25_with_scope(query, config.top_k * 2, scope).await?;
        
        let mut results: Vec<HybridSearchResult> = bm25_results
            .into_iter()
            .take(config.top_k)
            .collect();
        
        // 应用时间衰减
        if config.apply_time_decay {
            let now = Utc::now();
            for result in &mut results {
                if let Ok(Some(entry)) = self.retrieve(&result.id).await {
                    let age_days = (now - entry.last_accessed).num_seconds() as f32 / 86400.0;
                    let decay = (-config.decay_lambda * age_days).exp();
                    result.final_score = result.bm25_score * decay;
                }
            }
        }
        
        // 按分数排序
        results.sort_by(|a, b| {
            b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(results)
    }
    
    /// BM25 全文搜索（带隔离）
    async fn search_bm25_with_scope(
        &self, 
        query: &str, 
        limit: usize,
        scope: &MemoryScope
    ) -> Result<Vec<HybridSearchResult>> {
        let conn = self.conn.lock().await;
        
        // 构建隔离条件
        let (where_clause, params) = scope.to_where_clause();
        
        // 使用 FTS5 搜索
        let sql = format!(
            r#"
            SELECT m.id, m.content, m.importance, m.created_at, fts.rank as bm25_score
            FROM memories m
            JOIN memories_fts fts ON m.id = fts.id
            WHERE {} AND memories_fts MATCH ?
            ORDER BY fts.rank DESC
            LIMIT ?
            "#,
            where_clause
        );
        
        // 构建参数
        let mut sql_params: Vec<Box<dyn rusqlite::ToSql>> = params;
        sql_params.push(Box::new(query.to_string()));
        sql_params.push(Box::new(limit as i32));
        
        // 转换参数引用
        let params_refs: Vec<&dyn rusqlite::ToSql> = sql_params.iter().map(|p| p.as_ref()).collect();
        
        let mut stmt = conn.prepare(&sql)?;
        let results = stmt.query_map(&params_refs[..], |row| {
            Ok(HybridSearchResult {
                id: row.get(0)?,
                content: row.get(1)?,
                bm25_score: row.get::<_, f64>(4)? as f32,
                vector_score: 0.0,
                final_score: row.get::<_, f64>(4)? as f32,
                importance: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(results)
    }
    
    /// 获取用户所有记忆
    async fn get_user_memories(&self, user_id: &UserId) -> Result<Vec<MemoryEntry>> {
        let scope = MemoryScope {
            user_id: user_id.as_str().to_string(),
            channel: "global".to_string(),
            agent_id: None,
            namespace: None,
        };
        self.get_memories_by_scope(&scope).await
    }
    
    /// 按隔离维度获取记忆
    async fn get_memories_by_scope(&self, scope: &MemoryScope) -> Result<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;
        
        let (where_clause, params) = scope.to_where_clause();
        let sql = format!(
            "SELECT id, user_id, channel, agent_id, namespace, memory_type, content, importance, source_agent, tags, metadata, created_at, last_accessed 
             FROM memories WHERE {} ORDER BY created_at DESC",
            where_clause
        );
        
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map(&params_refs[..], |row| Self::parse_memory_entry(row))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(entries)
    }
    
    /// 获取存储统计
    async fn stats(&self) -> Result<StorageStats> {
        let conn = self.conn.lock().await;
        
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
        let users: i64 = conn.query_row("SELECT COUNT(DISTINCT user_id) FROM memories", [], |row| row.get(0))?;
        
        Ok(StorageStats {
            total_entries: total as usize,
            total_users: users as usize,
            db_size_bytes: 0, // TODO: 计算数据库大小
            last_updated: Utc::now(),
        })
    }
    
    /// 获取隔离维度统计
    async fn stats_by_scope(&self, scope: &MemoryScope) -> Result<StorageStats> {
        let conn = self.conn.lock().await;
        
        let (where_clause, params) = scope.to_where_clause();
        let sql = format!("SELECT COUNT(*) FROM memories WHERE {}", where_clause);
        
        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        let total: i64 = conn.query_row(&sql, &params_refs[..], |row| row.get(0))?;
        
        Ok(StorageStats {
            total_entries: total as usize,
            total_users: 1,
            db_size_bytes: 0,
            last_updated: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_scope() {
        let scope = MemoryScope::for_channel("user1", "feishu");
        assert_eq!(scope.user_id, "user1");
        assert_eq!(scope.channel, "feishu");
        assert!(scope.agent_id.is_none());
        assert!(scope.namespace.is_none());
        
        let scope = scope.with_agent("agent1").with_namespace("ns1");
        assert_eq!(scope.agent_id, Some("agent1".to_string()));
        assert_eq!(scope.namespace, Some("ns1".to_string()));
    }
    
    #[tokio::test]
    async fn test_sqlite_storage() {
        let storage = SQLiteMemoryStorage::new(StorageConfig::in_memory()).unwrap();
        
        let entry = MemoryEntry {
            id: "test1".to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Fact,
            importance: 0.8,
            content: "这是一个测试记忆".to_string(),
            metadata: std::collections::HashMap::new(),
            source_agent: Some("test".to_string()),
            tags: vec!["test".to_string()],
        };
        
        let scope = MemoryScope::for_channel("user1", "cli");
        storage.store_with_scope(&entry, &scope).await.unwrap();
        
        let retrieved = storage.retrieve("test1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "这是一个测试记忆");
    }
    
    #[tokio::test]
    async fn test_search_with_scope() {
        let storage = SQLiteMemoryStorage::new(StorageConfig::in_memory()).unwrap();
        
        // 存储不同隔离维度的记忆
        let scope1 = MemoryScope::for_channel("user1", "cli");
        let scope2 = MemoryScope::for_channel("user1", "feishu");
        
        let entry1 = MemoryEntry {
            id: "msg1".to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Conversation,
            importance: 0.5,
            content: "CLI channel message".to_string(),
            metadata: std::collections::HashMap::new(),
            source_agent: None,
            tags: vec![],
        };
        
        let entry2 = MemoryEntry {
            id: "msg2".to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Conversation,
            importance: 0.5,
            content: "Feishu channel message".to_string(),
            metadata: std::collections::HashMap::new(),
            source_agent: None,
            tags: vec![],
        };
        
        storage.store_with_scope(&entry1, &scope1).await.unwrap();
        storage.store_with_scope(&entry2, &scope2).await.unwrap();
        
        // 验证按隔离维度获取记忆
        let mem1 = storage.get_memories_by_scope(&scope1).await.unwrap();
        let mem2 = storage.get_memories_by_scope(&scope2).await.unwrap();
        
        // 验证隔离正确
        assert_eq!(mem1.len(), 1);
        assert_eq!(mem2.len(), 1);
        assert_eq!(mem1[0].id, "msg1");
        assert_eq!(mem2[0].id, "msg2");
    }
}