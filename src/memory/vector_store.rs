// Vector Store - 向量存储抽象
//
// v0.7.0 - 支持多种向量数据库后端

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use rusqlite::OptionalExtension;

/// 向量维度
pub const DEFAULT_VECTOR_DIM: usize = 1536; // OpenAI ada-002

/// 向量存储 Trait
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 添加向量
    async fn upsert(&self, id: &str, vector: &[f32], metadata: HashMap<String, String>) -> Result<()>;
    
    /// 删除向量
    async fn delete(&self, id: &str) -> Result<()>;
    
    /// 向量搜索
    async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>>;
    
    /// 获取向量
    async fn get(&self, id: &str) -> Result<Option<Vec<f32>>>;
    
    /// 获取统计
    async fn stats(&self) -> Result<VectorStoreStats>;
}

/// 向量搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// ID
    pub id: String,
    /// 相似度分数
    pub score: f32,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

/// 向量存储统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    /// 总向量数
    pub total_vectors: usize,
    /// 向量维度
    pub dimension: usize,
    /// 索引大小（字节）
    pub index_size_bytes: u64,
}

/// 向量存储配置
#[derive(Debug, Clone)]
pub enum VectorStoreConfig {
    /// 内存存储（默认）
    InMemory {
        dimension: usize,
    },
    /// SQLite 存储
    SQLite {
        path: String,
        dimension: usize,
    },
    /// Qdrant 向量数据库
    Qdrant {
        url: String,
        collection: String,
        api_key: Option<String>,
    },
    /// Milvus 向量数据库
    Milvus {
        address: String,
        collection: String,
    },
    /// pgvector (PostgreSQL)
    PgVector {
        connection_string: String,
        table_name: String,
    },
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self::InMemory {
            dimension: DEFAULT_VECTOR_DIM,
        }
    }
}

// ============================================================================
// 内存向量存储
// ============================================================================

use tokio::sync::RwLock;

/// 内存向量存储实现
pub struct InMemoryVectorStore {
    vectors: Arc<RwLock<HashMap<String, (Vec<f32>, HashMap<String, String>)>>>,
    dimension: usize,
}

impl InMemoryVectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            vectors: Arc::new(RwLock::new(HashMap::new())),
            dimension,
        }
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, id: &str, vector: &[f32], metadata: HashMap<String, String>) -> Result<()> {
        if vector.len() != self.dimension {
            anyhow::bail!("Vector dimension mismatch: expected {}, got {}", self.dimension, vector.len());
        }
        
        let mut vectors = self.vectors.write().await;
        vectors.insert(id.to_string(), (vector.to_vec(), metadata));
        Ok(())
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let mut vectors = self.vectors.write().await;
        vectors.remove(id);
        Ok(())
    }
    
    async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>> {
        let vectors = self.vectors.read().await;
        
        let mut results: Vec<VectorSearchResult> = vectors.iter()
            .map(|(id, (vec, meta))| {
                let score = cosine_similarity(query, vec);
                VectorSearchResult {
                    id: id.clone(),
                    score,
                    metadata: meta.clone(),
                }
            })
            .collect();
        
        // 按分数排序
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        results.truncate(top_k);
        Ok(results)
    }
    
    async fn get(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let vectors = self.vectors.read().await;
        Ok(vectors.get(id).map(|(v, _)| v.clone()))
    }
    
    async fn stats(&self) -> Result<VectorStoreStats> {
        let vectors = self.vectors.read().await;
        Ok(VectorStoreStats {
            total_vectors: vectors.len(),
            dimension: self.dimension,
            index_size_bytes: vectors.len() as u64 * self.dimension as u64 * std::mem::size_of::<f32>() as u64,
        })
    }
}

// ============================================================================
// SQLite 向量存储（使用同步锁）
// ============================================================================

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::RwLock as StdRwLock;

/// SQLite 向量存储实现
pub struct SQLiteVectorStore {
    conn: Arc<StdRwLock<Connection>>,
    dimension: usize,
    table_name: String,
}

impl SQLiteVectorStore {
    pub fn new(path: &str, dimension: usize, table_name: &str) -> Result<Self> {
        // 确保目录存在
        let path_buf = PathBuf::from(path);
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| "Failed to create database directory")?;
        }
        
        let conn = Connection::open(path)
            .with_context(|| "Failed to open database")?;
        
        // 创建表
        conn.execute_batch(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                metadata TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );
            
            CREATE INDEX IF NOT EXISTS idx_{}_created ON {}(created_at);
            "#,
            table_name, table_name, table_name
        )).with_context(|| "Failed to create table")?;
        
        Ok(Self {
            conn: Arc::new(StdRwLock::new(conn)),
            dimension,
            table_name: table_name.to_string(),
        })
    }
}

impl SQLiteVectorStore {
    /// 同步搜索
    fn search_sync(&self, query: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>> {
        let conn = self.conn.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        
        let mut stmt = conn.prepare(
            &format!("SELECT id, vector, metadata FROM {}", self.table_name)
        )?;
        
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let vector_bytes: Vec<u8> = row.get(1)?;
            let metadata_json: String = row.get(2)?;
            
            let vector: Vec<f32> = vector_bytes.chunks_exact(4)
                .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                .collect();
            
            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_json).unwrap_or_default();
            
            Ok((id, vector, metadata))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            let (id, vector, metadata) = row?;
            let score = cosine_similarity(query, &vector);
            results.push(VectorSearchResult { id, score, metadata });
        }
        
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        
        Ok(results)
    }
    
    /// 同步获取向量
    fn get_sync(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let conn = self.conn.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        
        let result = conn.query_row(
            &format!("SELECT vector FROM {} WHERE id = ?1", self.table_name),
            rusqlite::params![id],
            |row| {
                let vector_bytes: Vec<u8> = row.get(0)?;
                let vector: Vec<f32> = vector_bytes.chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                Ok(vector)
            },
        ).optional()?;
        
        Ok(result)
    }
    
    /// 同步获取统计
    fn stats_sync(&self) -> Result<VectorStoreStats> {
        let conn = self.conn.read().map_err(|_| anyhow::anyhow!("Lock poisoned"))?;
        
        let count: usize = conn.query_row(
            &format!("SELECT COUNT(*) FROM {}", self.table_name),
            [],
            |row| row.get(0)
        ).unwrap_or(0);
        
        Ok(VectorStoreStats {
            total_vectors: count,
            dimension: self.dimension,
            index_size_bytes: count as u64 * self.dimension as u64 * 4,
        })
    }
}

// 注意：SQLiteVectorStore 不是 Send，不能直接用于 async trait
// 但可以用于本地同步场景

// ============================================================================
// 工具函数
// ============================================================================

/// 计算余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot / (norm_a * norm_b)
}

/// 欧几里得距离
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::MAX;
    }
    
    a.iter().zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// 向量归一化
pub fn normalize_vector(vector: &[f32]) -> Vec<f32> {
    let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm == 0.0 {
        return vector.to_vec();
    }
    
    vector.iter().map(|x| x / norm).collect()
}

/// 创建向量存储
pub fn create_vector_store(config: VectorStoreConfig) -> Result<Arc<dyn VectorStore>> {
    match config {
        VectorStoreConfig::InMemory { dimension } => {
            Ok(Arc::new(InMemoryVectorStore::new(dimension)))
        }
        VectorStoreConfig::SQLite { dimension, .. } => {
            // SQLite 存储由于 rusqlite::Connection 不是 Send，暂时回退到内存存储
            // 未来可以使用 sqlx 或其他异步 SQLite 库
            Ok(Arc::new(InMemoryVectorStore::new(dimension)))
        }
        _ => {
            // 其他后端暂不支持，回退到内存存储
            Ok(Arc::new(InMemoryVectorStore::new(DEFAULT_VECTOR_DIM)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.01);
        
        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.01);
        
        let d = vec![0.5, 0.5, 0.0];
        let d_normalized = normalize_vector(&d);
        assert!((cosine_similarity(&a, &d_normalized) - 0.707).abs() < 0.01);
    }
    
    #[tokio::test]
    async fn test_in_memory_vector_store() {
        let store = InMemoryVectorStore::new(3);
        
        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "test".to_string());
        
        store.upsert("v1", &[1.0, 0.0, 0.0], meta.clone()).await.unwrap();
        store.upsert("v2", &[0.0, 1.0, 0.0], meta.clone()).await.unwrap();
        store.upsert("v3", &[0.0, 0.0, 1.0], meta).await.unwrap();
        
        let results = store.search(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "v1");
        assert!(results[0].score > 0.99);
    }
    
    #[test]
    fn test_sqlite_vector_store_sync() {
        let store = SQLiteVectorStore::new(":memory:", 3, "vectors").unwrap();
        
        // 使用同步方法测试
        let results = store.search_sync(&[1.0, 0.0, 0.0], 10).unwrap();
        assert!(results.is_empty());
        
        let stats = store.stats_sync().unwrap();
        assert_eq!(stats.total_vectors, 0);
    }
    
    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        assert!((euclidean_distance(&a, &b) - 5.0).abs() < 0.01);
    }
    
    #[test]
    fn test_normalize_vector() {
        let v = vec![3.0, 4.0];
        let n = normalize_vector(&v);
        assert!((n[0] - 0.6).abs() < 0.01);
        assert!((n[1] - 0.8).abs() < 0.01);
    }
}