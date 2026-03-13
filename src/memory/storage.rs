// Memory Storage - 统一记忆存储抽象
//
// v0.7.0 - 实现持久化存储，支持 FTS5 全文索引

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use rusqlite::{Connection, OptionalExtension};
use tokio::sync::Mutex;

use super::shared::{MemoryEntry, MemoryType, UserId};

// ============================================================================
// Hybrid Search - 混合检索配置和结果
// ============================================================================

/// 混合搜索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    /// 返回结果数量
    pub top_k: usize,
    /// BM25 权重
    pub bm25_weight: f32,
    /// 向量权重
    pub vector_weight: f32,
    /// 是否应用时间衰减
    pub apply_time_decay: bool,
    /// 时间衰减系数 (λ)
    pub decay_lambda: f32,
    /// 最小相似度阈值
    pub min_score: f32,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            bm25_weight: 0.7,
            vector_weight: 0.3,
            apply_time_decay: true,
            decay_lambda: 0.1,
            min_score: 0.0,
        }
    }
}

impl HybridSearchConfig {
    /// 创建严格配置（精确匹配优先）
    pub fn strict() -> Self {
        Self {
            top_k: 5,
            bm25_weight: 0.8,
            vector_weight: 0.2,
            apply_time_decay: true,
            decay_lambda: 0.05,
            min_score: 0.3,
        }
    }
    
    /// 创建语义配置（向量优先）
    pub fn semantic() -> Self {
        Self {
            top_k: 20,
            bm25_weight: 0.3,
            vector_weight: 0.7,
            apply_time_decay: true,
            decay_lambda: 0.1,
            min_score: 0.1,
        }
    }
}

/// 混合搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    /// 记忆 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// BM25 分数
    pub bm25_score: f32,
    /// 向量分数
    pub vector_score: f32,
    /// 最终分数（融合后）
    pub final_score: f32,
    /// 重要性
    pub importance: f32,
    /// 创建时间
    pub created_at: String,
}

// ============================================================================
// Memory Storage Trait
// ============================================================================

/// 记忆存储 Trait - 统一抽象
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// 存储记忆条目
    async fn store(&self, entry: &MemoryEntry) -> Result<String>;
    
    /// 获取记忆条目
    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>>;
    
    /// 删除记忆条目
    async fn delete(&self, id: &str) -> Result<()>;
    
    /// 混合搜索（BM25 + 向量）
    async fn search_hybrid(&self, query: &str, config: &HybridSearchConfig) -> Result<Vec<HybridSearchResult>>;
    
    /// BM25 全文搜索
    async fn search_bm25(&self, query: &str, limit: usize) -> Result<Vec<HybridSearchResult>>;
    
    /// 获取用户所有记忆
    async fn get_user_memories(&self, user_id: &UserId) -> Result<Vec<MemoryEntry>>;
    
    /// 获取存储统计
    async fn stats(&self) -> Result<StorageStats>;
}

/// 存储统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_entries: usize,
    pub total_users: usize,
    pub db_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// 存储配置
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub db_path: PathBuf,
    pub enable_fts: bool,
    pub enable_vector: bool,
    pub max_entries: usize,
    pub auto_cleanup_days: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("data/memory.db"),
            enable_fts: true,
            enable_vector: true,
            max_entries: 100000,
            auto_cleanup_days: 30,
        }
    }
}

impl StorageConfig {
    pub fn in_memory() -> Self {
        Self {
            db_path: PathBuf::from(":memory:"),
            enable_fts: true,
            enable_vector: true,
            max_entries: 10000,
            auto_cleanup_days: 7,
        }
    }
}

// ============================================================================
// SQLite Memory Storage
// ============================================================================

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
        
        // 初始化表（同步操作）
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
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
            CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(memory_type);
            CREATE INDEX IF NOT EXISTS idx_memories_created ON memories(created_at);
            
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
}

#[async_trait]
impl MemoryStorage for SQLiteMemoryStorage {
    async fn store(&self, entry: &MemoryEntry) -> Result<String> {
        let conn = self.conn.lock().await;
        
        let memory_type = Self::memory_type_to_string(&entry.memory_type);
        let tags = serde_json::to_string(&entry.tags)?;
        let metadata = serde_json::to_string(&entry.metadata)?;
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO memories 
            (id, user_id, memory_type, content, importance, source_agent, tags, metadata, created_at, last_accessed)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            rusqlite::params![
                entry.id,
                "",
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
    
    async fn retrieve(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.conn.lock().await;
        
        let result = conn.query_row(
            "SELECT id, memory_type, content, importance, source_agent, tags, metadata, created_at, last_accessed 
             FROM memories WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                let created_at_str: String = row.get(7)?;
                let last_accessed_str: String = row.get(8)?;
                
                Ok(MemoryEntry {
                    id: row.get(0)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    last_accessed: chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    memory_type: Self::string_to_memory_type(&row.get::<_, String>(1)?),
                    importance: row.get(3)?,
                    content: row.get(2)?,
                    metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    source_agent: row.get(4)?,
                    tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                })
            },
        ).optional()?;
        
        Ok(result)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![id])?;
        conn.execute("DELETE FROM memories_fts WHERE id = ?1", rusqlite::params![id]).ok();
        Ok(())
    }
    
    async fn search_hybrid(&self, query: &str, config: &HybridSearchConfig) -> Result<Vec<HybridSearchResult>> {
        let bm25_results: Vec<HybridSearchResult> = self.search_bm25(query, config.top_k * 2).await?;
        
        let mut results: Vec<HybridSearchResult> = bm25_results
            .into_iter()
            .take(config.top_k)
            .collect();
        
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
        
        results.sort_by(|a, b| {
            b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(results)
    }
    
    async fn search_bm25(&self, query: &str, limit: usize) -> Result<Vec<HybridSearchResult>> {
        let conn = self.conn.lock().await;
        
        let results = conn.prepare(
            "SELECT m.id, m.content, m.importance, m.created_at, -bm25(memories_fts) as score
             FROM memories_fts fts
             JOIN memories m ON fts.id = m.id
             WHERE memories_fts MATCH ?1
             ORDER BY bm25(memories_fts)
             LIMIT ?2"
        )
        .and_then(|mut stmt| {
            let rows = stmt.query_map(rusqlite::params![query, limit as i32], |row| {
                Ok(HybridSearchResult {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    bm25_score: row.get::<_, f32>(4)?,
                    vector_score: 0.0,
                    final_score: row.get::<_, f32>(4)?,
                    importance: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()
        })?;
        
        Ok(results)
    }
    
    async fn get_user_memories(&self, _user_id: &UserId) -> Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }
    
    async fn stats(&self) -> Result<StorageStats> {
        let conn = self.conn.lock().await;
        
        let total_entries: usize = conn.query_row(
            "SELECT COUNT(*) FROM memories", 
            [], 
            |row| row.get(0)
        ).unwrap_or(0);
        
        let db_size = if self.config.db_path != PathBuf::from(":memory:") {
            std::fs::metadata(&self.config.db_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };
        
        Ok(StorageStats {
            total_entries,
            total_users: 0,
            db_size_bytes: db_size,
            last_updated: Utc::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    
    fn create_test_entry(id: &str, content: &str) -> MemoryEntry {
        MemoryEntry {
            id: id.to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Fact,
            importance: 0.8,
            content: content.to_string(),
            metadata: HashMap::new(),
            source_agent: None,
            tags: vec!["test".to_string()],
        }
    }
    
    #[tokio::test]
    async fn test_sqlite_storage_store_and_retrieve() {
        let storage = SQLiteMemoryStorage::new(StorageConfig::in_memory()).unwrap();
        let entry = create_test_entry("test-1", "This is a test memory about Rust programming");
        
        storage.store(&entry).await.unwrap();
        let retrieved = storage.retrieve("test-1").await.unwrap().unwrap();
        
        assert_eq!(retrieved.content, "This is a test memory about Rust programming");
    }
    
    #[tokio::test]
    async fn test_sqlite_storage_bm25_search() {
        let storage = SQLiteMemoryStorage::new(StorageConfig::in_memory()).unwrap();
        
        storage.store(&create_test_entry("m1", "Rust is a systems programming language")).await.unwrap();
        storage.store(&create_test_entry("m2", "Python is great for data science")).await.unwrap();
        storage.store(&create_test_entry("m3", "Rust emphasizes memory safety")).await.unwrap();
        
        let results = storage.search_bm25("Rust", 10).await.unwrap();
        assert!(!results.is_empty());
    }
    
    #[tokio::test]
    async fn test_hybrid_search_config_default() {
        let config = HybridSearchConfig::default();
        assert_eq!(config.top_k, 10);
        assert_eq!(config.bm25_weight, 0.7);
    }
}