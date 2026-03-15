// 向量索引模块（内存实现）
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 向量条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub content: String,
}

/// 向量索引（内存实现）
pub struct VectorIndex {
    entries: Arc<RwLock<HashMap<String, VectorEntry>>>,
    dimension: usize,
}

impl VectorIndex {
    /// 创建新的向量索引
    pub fn new(dimension: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            dimension,
        }
    }

    /// 添加向量
    pub async fn add(&self, id: &str, vector: Vec<f32>, content: &str, metadata: HashMap<String, String>) -> Result<()> {
        if vector.len() != self.dimension {
            return Err(anyhow::anyhow!(
                "Vector dimension mismatch: expected {}, got {}",
                self.dimension,
                vector.len()
            ));
        }

        let entry = VectorEntry {
            id: id.to_string(),
            vector,
            metadata,
            content: content.to_string(),
        };

        let mut entries = self.entries.write().await;
        entries.insert(id.to_string(), entry);

        Ok(())
    }

    /// 删除向量
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut entries = self.entries.write().await;
        Ok(entries.remove(id).is_some())
    }

    /// 搜索最相似的向量
    pub async fn search(&self, query_vector: &[f32], top_k: usize) -> Result<Vec<(String, f32, String)>> {
        if query_vector.len() != self.dimension {
            return Err(anyhow::anyhow!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.dimension,
                query_vector.len()
            ));
        }

        let entries = self.entries.read().await;
        let mut results: Vec<(String, f32, String)> = entries
            .values()
            .map(|entry| {
                let similarity = cosine_similarity(query_vector, &entry.vector);
                (entry.id.clone(), similarity, entry.content.clone())
            })
            .collect();

        // 按相似度排序
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(top_k);

        Ok(results)
    }

    /// 获取所有条目数量
    pub async fn count(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }
}

/// 计算余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_index_add() {
        let index = VectorIndex::new(3);
        let vector = vec![1.0, 0.0, 0.0];
        let metadata = HashMap::new();

        index.add("test1", vector, "test content", metadata).await.unwrap();
        assert_eq!(index.count().await, 1);
    }

    #[tokio::test]
    async fn test_vector_index_search() {
        let index = VectorIndex::new(3);
        
        // 添加一些向量
        index.add("vec1", vec![1.0, 0.0, 0.0], "content 1", HashMap::new()).await.unwrap();
        index.add("vec2", vec![0.0, 1.0, 0.0], "content 2", HashMap::new()).await.unwrap();
        index.add("vec3", vec![1.0, 1.0, 0.0], "content 3", HashMap::new()).await.unwrap();

        // 搜索最相似的向量
        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "vec1"); // 最相似的是 vec1
        assert!(results[0].1 > 0.99); // 相似度应该接近 1.0
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_eq!(cosine_similarity(&a, &b), 1.0);

        let c = vec![0.0, 1.0, 0.0];
        assert_eq!(cosine_similarity(&a, &c), 0.0);
    }

    #[tokio::test]
    async fn test_vector_index_delete() {
        let index = VectorIndex::new(3);
        index.add("test1", vec![1.0, 0.0, 0.0], "content", HashMap::new()).await.unwrap();
        
        let deleted = index.delete("test1").await.unwrap();
        assert!(deleted);
        assert_eq!(index.count().await, 0);
    }
}
