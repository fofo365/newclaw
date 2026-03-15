// Vector Database Abstraction - v0.5.4
//
// 向量存储抽象层

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::collections::HashMap;

/// 向量文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    /// 文档 ID
    pub id: String,
    /// 文本内容
    pub text: String,
    /// 嵌入向量
    pub embedding: Vec<f32>,
    /// 元数据
    pub metadata: DocumentMetadata,
}

/// 文档元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// 来源
    pub source: String,
    /// 时间戳
    pub timestamp: i64,
    /// 文档类型
    pub doc_type: String,
    /// 自定义字段
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            source: "unknown".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            doc_type: "text".to_string(),
            custom: HashMap::new(),
        }
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 文档
    pub document: VectorDocument,
    /// 相似度分数
    pub score: f32,
}

/// 向量存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    /// 存储类型
    pub store_type: VectorStoreType,
    /// 连接 URL
    pub url: Option<String>,
    /// 集合名称
    pub collection_name: String,
    /// 向量维度
    pub dimension: usize,
    /// 距离度量
    pub distance_metric: DistanceMetric,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            store_type: VectorStoreType::Memory,
            url: None,
            collection_name: "default".to_string(),
            dimension: 1536,
            distance_metric: DistanceMetric::Cosine,
        }
    }
}

/// 向量存储类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VectorStoreType {
    /// 内存存储
    Memory,
    /// Qdrant
    Qdrant,
    /// Milvus
    Milvus,
    /// PostgreSQL pgvector
    PgVector,
}

/// 距离度量
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DistanceMetric {
    /// 余弦相似度
    Cosine,
    /// 欧几里得距离
    Euclidean,
    /// 点积
    DotProduct,
}

/// 向量存储 Trait
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 添加文档
    async fn add(&mut self, document: VectorDocument) -> Result<()>;
    
    /// 批量添加文档
    async fn add_batch(&mut self, documents: Vec<VectorDocument>) -> Result<()>;
    
    /// 搜索相似文档
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
    
    /// 搜索相似文档（带过滤）
    async fn search_with_filter(
        &self,
        query: &[f32],
        limit: usize,
        filter: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<SearchResult>>;
    
    /// 删除文档
    async fn delete(&mut self, id: &str) -> Result<bool>;
    
    /// 获取文档
    async fn get(&self, id: &str) -> Result<Option<VectorDocument>>;
    
    /// 更新文档
    async fn update(&mut self, document: VectorDocument) -> Result<()>;
    
    /// 获取文档数量
    async fn count(&self) -> Result<usize>;
    
    /// 清空集合
    async fn clear(&mut self) -> Result<()>;
    
    /// 创建索引
    async fn create_index(&mut self) -> Result<()>;
    
    /// 删除索引
    async fn drop_index(&mut self) -> Result<()>;
}

/// 内存向量存储
pub struct MemoryVectorStore {
    documents: HashMap<String, VectorDocument>,
    dimension: usize,
}

impl MemoryVectorStore {
    /// 创建新的内存存储
    pub fn new(dimension: usize) -> Self {
        Self {
            documents: HashMap::new(),
            dimension,
        }
    }
    
    /// 计算余弦相似度
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        
        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }
        
        dot_product / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorStore for MemoryVectorStore {
    async fn add(&mut self, document: VectorDocument) -> Result<()> {
        self.documents.insert(document.id.clone(), document);
        Ok(())
    }
    
    async fn add_batch(&mut self, documents: Vec<VectorDocument>) -> Result<()> {
        for doc in documents {
            self.documents.insert(doc.id.clone(), doc);
        }
        Ok(())
    }
    
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let mut results: Vec<SearchResult> = self.documents
            .values()
            .map(|doc| {
                let score = Self::cosine_similarity(query, &doc.embedding);
                SearchResult {
                    document: doc.clone(),
                    score,
                }
            })
            .filter(|r| r.score > 0.0)
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);
        
        Ok(results)
    }
    
    async fn search_with_filter(
        &self,
        query: &[f32],
        limit: usize,
        _filter: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<SearchResult>> {
        // 简化实现：忽略过滤
        self.search(query, limit).await
    }
    
    async fn delete(&mut self, id: &str) -> Result<bool> {
        Ok(self.documents.remove(id).is_some())
    }
    
    async fn get(&self, id: &str) -> Result<Option<VectorDocument>> {
        Ok(self.documents.get(id).cloned())
    }
    
    async fn update(&mut self, document: VectorDocument) -> Result<()> {
        self.documents.insert(document.id.clone(), document);
        Ok(())
    }
    
    async fn count(&self) -> Result<usize> {
        Ok(self.documents.len())
    }
    
    async fn clear(&mut self) -> Result<()> {
        self.documents.clear();
        Ok(())
    }
    
    async fn create_index(&mut self) -> Result<()> {
        // 内存存储不需要索引
        Ok(())
    }
    
    async fn drop_index(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Qdrant 向量存储（模拟实现）
pub struct QdrantVectorStore {
    config: VectorStoreConfig,
    inner: MemoryVectorStore,
}

impl QdrantVectorStore {
    /// 创建新的 Qdrant 存储
    pub fn new(config: VectorStoreConfig) -> Self {
        let dimension = config.dimension;
        Self {
            config,
            inner: MemoryVectorStore::new(dimension),
        }
    }
    
    /// 连接到 Qdrant
    pub async fn connect(&mut self) -> Result<()> {
        // 实际实现：使用 qdrant-client 连接
        if let Some(ref url) = self.config.url {
            tracing::info!("Connecting to Qdrant at {}", url);
        }
        Ok(())
    }
}

#[async_trait]
impl VectorStore for QdrantVectorStore {
    async fn add(&mut self, document: VectorDocument) -> Result<()> {
        // 实际实现：调用 Qdrant API
        self.inner.add(document).await
    }
    
    async fn add_batch(&mut self, documents: Vec<VectorDocument>) -> Result<()> {
        self.inner.add_batch(documents).await
    }
    
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        self.inner.search(query, limit).await
    }
    
    async fn search_with_filter(
        &self,
        query: &[f32],
        limit: usize,
        filter: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<SearchResult>> {
        self.inner.search_with_filter(query, limit, filter).await
    }
    
    async fn delete(&mut self, id: &str) -> Result<bool> {
        self.inner.delete(id).await
    }
    
    async fn get(&self, id: &str) -> Result<Option<VectorDocument>> {
        self.inner.get(id).await
    }
    
    async fn update(&mut self, document: VectorDocument) -> Result<()> {
        self.inner.update(document).await
    }
    
    async fn count(&self) -> Result<usize> {
        self.inner.count().await
    }
    
    async fn clear(&mut self) -> Result<()> {
        self.inner.clear().await
    }
    
    async fn create_index(&mut self) -> Result<()> {
        self.inner.create_index().await
    }
    
    async fn drop_index(&mut self) -> Result<()> {
        self.inner.drop_index().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_store_config_default() {
        let config = VectorStoreConfig::default();
        assert_eq!(config.store_type, VectorStoreType::Memory);
        assert_eq!(config.dimension, 1536);
    }

    #[test]
    fn test_document_metadata_default() {
        let metadata = DocumentMetadata::default();
        assert_eq!(metadata.source, "unknown");
    }

    #[tokio::test]
    async fn test_memory_vector_store_add() {
        let mut store = MemoryVectorStore::new(1536);
        let doc = VectorDocument {
            id: "1".to_string(),
            text: "test".to_string(),
            embedding: vec![0.1; 1536],
            metadata: DocumentMetadata::default(),
        };
        
        store.add(doc).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_memory_vector_store_search() {
        let mut store = MemoryVectorStore::new(3);
        
        store.add_batch(vec![
            VectorDocument {
                id: "1".to_string(),
                text: "doc1".to_string(),
                embedding: vec![1.0, 0.0, 0.0],
                metadata: DocumentMetadata::default(),
            },
            VectorDocument {
                id: "2".to_string(),
                text: "doc2".to_string(),
                embedding: vec![0.0, 1.0, 0.0],
                metadata: DocumentMetadata::default(),
            },
        ]).await.unwrap();
        
        let query = vec![1.0, 0.0, 0.0];
        let results = store.search(&query, 10).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "1");
    }

    #[tokio::test]
    async fn test_memory_vector_store_delete() {
        let mut store = MemoryVectorStore::new(3);
        
        store.add(VectorDocument {
            id: "1".to_string(),
            text: "test".to_string(),
            embedding: vec![0.0; 3],
            metadata: DocumentMetadata::default(),
        }).await.unwrap();
        
        let deleted = store.delete("1").await.unwrap();
        assert!(deleted);
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        
        let sim = MemoryVectorStore::cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0, 0.0];
        let sim2 = MemoryVectorStore::cosine_similarity(&a, &c);
        assert!((sim2 - 0.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_qdrant_vector_store_new() {
        let config = VectorStoreConfig {
            store_type: VectorStoreType::Qdrant,
            url: Some("http://localhost:6333".to_string()),
            ..Default::default()
        };
        
        let store = QdrantVectorStore::new(config);
        assert_eq!(store.count().await.unwrap(), 0);
    }
}
