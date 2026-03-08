// Vector Database Module for Semantic Search

use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub source: String,
    pub timestamp: i64,
    pub message_type: String,
    pub tokens: usize,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub document: VectorDocument,
    pub score: f32,
}

pub trait VectorStore: Send + Sync {
    fn add_document(&mut self, document: VectorDocument) -> Result<()>;
    
    fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>>;
    
    fn delete(&mut self, id: &str) -> Result<bool>;
    
    fn len(&self) -> usize;
}

// Simple in-memory vector store with cosine similarity
#[derive(Debug)]
pub struct MemoryVectorStore {
    documents: Vec<VectorDocument>,
}

impl MemoryVectorStore {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }
    
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            documents: Vec::with_capacity(capacity),
        }
    }
}

impl Default for MemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorStore for MemoryVectorStore {
    fn add_document(&mut self, document: VectorDocument) -> Result<()> {
        self.documents.push(document);
        Ok(())
    }
    
    fn search(&self, query: &[f32], limit: usize) -> Result<Vec<SearchResult>> {
        let mut results: Vec<SearchResult> = self.documents
            .iter()
            .map(|doc| {
                let score = cosine_similarity(query, &doc.embedding);
                SearchResult {
                    document: doc.clone(),
                    score,
                }
            })
            .filter(|r| r.score > 0.0)
            .collect();
        
        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // Take top results
        results.truncate(limit);
        
        Ok(results)
    }
    
    fn delete(&mut self, id: &str) -> Result<bool> {
        let original_len = self.documents.len();
        self.documents.retain(|doc| doc.id != id);
        Ok(self.documents.len() < original_len)
    }
    
    fn len(&self) -> usize {
        self.documents.len()
    }
}

// Cosine similarity calculation
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

// Mock embedding function (in real use, replace with actual embedding API)
pub fn mock_embedding(text: &str) -> Vec<f32> {
    // Create a pseudo-embedding based on text hash
    let hash = text.chars().map(|c| c as u32).sum::<u32>() as f32;
    let size = 384; // Common embedding size
    (0..size)
        .map(|i| {
            let angle = (hash + i as f32) * 0.1;
            angle.sin() * 0.5 + 0.5
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        
        let c = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }
    
    #[test]
    fn test_vector_store() {
        let mut store = MemoryVectorStore::new();
        
        let doc1 = VectorDocument {
            id: "1".to_string(),
            text: "test".to_string(),
            embedding: vec![1.0, 0.0],
            metadata: DocumentMetadata {
                source: "test".to_string(),
                timestamp: 0,
                message_type: "test".to_string(),
                tokens: 1,
            },
        };
        
        store.add_document(doc1).unwrap();
        assert_eq!(store.len(), 1);
        
        let results = store.search(&vec![1.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!((results[0].score - 1.0).abs() < 0.001);
    }
    
    #[test]
    fn test_mock_embedding() {
        let emb = mock_embedding("test");
        assert_eq!(emb.len(), 384);
        for &val in &emb {
            assert!(val >= 0.0 && val <= 1.0);
        }
    }
}
