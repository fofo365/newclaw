// Context Manager - Handles message storage and retrieval with vector search

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::vector::{VectorStore, MemoryVectorStore, VectorDocument, DocumentMetadata, mock_embedding};

#[derive(Debug)]
pub struct ContextManager {
    db: Connection,
    pub config: ContextConfig,
    vector_store: MemoryVectorStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_chunks: usize,
    pub max_tokens: usize,
    pub overlap_tokens: usize,
    pub enable_vector_search: bool,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_chunks: 100,
            max_tokens: 8000,
            overlap_tokens: 200,
            enable_vector_search: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    pub id: String,
    pub text: String,
    pub tokens: usize,
    pub created_at: i64,
    pub metadata: ContextMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub source: String,
    pub timestamp: i64,
    pub message_type: String,
}

impl ContextManager {
    pub fn new(config: ContextConfig) -> Result<Self> {
        let db_path = "/var/lib/newclaw/context.db";
        
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let db = Connection::open(db_path)?;
        
        db.execute(
            "CREATE TABLE IF NOT EXISTS context_chunks (
                id TEXT PRIMARY KEY,
                text TEXT NOT NULL,
                tokens INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                metadata TEXT
            )",
            [],
        )?;

        let vector_store = MemoryVectorStore::new();

        Ok(Self { db, config, vector_store })
    }

    pub fn add_message(&mut self, message: &str, source: &str) -> Result<String> {
        let chunks = self.chunk_text(message)?;
        
        for chunk in chunks {
            let id = uuid::Uuid::new_v4().to_string();
            let tokens = estimate_tokens(&chunk);
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs() as i64;
            
            let metadata = ContextMetadata {
                source: source.to_string(),
                timestamp,
                message_type: "user".to_string(),
            };
            
            // Store in SQLite
            self.db.execute(
                "INSERT INTO context_chunks (id, text, tokens, created_at, metadata) \
                VALUES (?1, ?2, ?3, ?4, ?5)",
                [&id, &chunk, &(tokens as i32).to_string(), &timestamp.to_string(), &serde_json::to_string(&metadata)?],
            )?;
            
            // Store in vector store
            if self.config.enable_vector_search {
                let embedding = mock_embedding(&chunk);
                let doc = VectorDocument {
                    id: id.clone(),
                    text: chunk.clone(),
                    embedding,
                    metadata: DocumentMetadata {
                        source: source.to_string(),
                        timestamp,
                        message_type: "user".to_string(),
                        tokens,
                    },
                };
                self.vector_store.add_document(doc)?;
            }
            
            return Ok(id);
        }
        
        Ok(uuid::Uuid::new_v4().to_string())
    }

    pub fn retrieve_relevant(&self, query: &str, limit: usize) -> Result<Vec<ContextChunk>> {
        // Try vector search first
        if self.config.enable_vector_search {
            let query_embedding = mock_embedding(query);
            if let Ok(results) = self.vector_store.search(&query_embedding, limit) {
                if !results.is_empty() {
                    return Ok(results
                        .into_iter()
                        .map(|r| ContextChunk {
                            id: r.document.id,
                            text: r.document.text,
                            tokens: r.document.metadata.tokens,
                            created_at: r.document.metadata.timestamp,
                            metadata: ContextMetadata {
                                source: r.document.metadata.source,
                                timestamp: r.document.metadata.timestamp,
                                message_type: r.document.metadata.message_type,
                            },
                        })
                        .collect());
                }
            }
        }
        
        // Fallback to time-based retrieval
        let mut stmt = self.db.prepare(
            "SELECT id, text, tokens, created_at, metadata FROM context_chunks 
            ORDER BY created_at DESC LIMIT ?1"
        )?;
        
        let chunks = stmt.query_map([limit as i32], |row| {
            let metadata_str: String = row.get(4)?;
            let metadata = serde_json::from_str(&metadata_str).unwrap_or_else(|_| ContextMetadata {
                source: "unknown".to_string(),
                timestamp: 0,
                message_type: "unknown".to_string(),
            });
            
            Ok(ContextChunk {
                id: row.get(0)?,
                text: row.get(1)?,
                tokens: row.get(2)?,
                created_at: row.get(3)?,
                metadata,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        
        Ok(chunks)
    }

    pub fn select_optimal_context(&self, recent_chunks: &[ContextChunk], max_tokens: usize) -> Vec<ContextChunk> {
        let mut selected = Vec::new();
        let mut current_tokens = 0;
        
        for chunk in recent_chunks.iter().take(10) {
            if current_tokens + chunk.tokens > max_tokens {
                break;
            }
            selected.push(chunk.clone());
            current_tokens += chunk.tokens;
        }
        
        selected
    }

    fn chunk_text(&self, text: &str) -> Result<Vec<String>> {
        let chunk_size = 1000;
        let mut chunks = Vec::new();
        
        for _chunk in text.as_bytes().chunks(chunk_size) {
            chunks.push(String::from_utf8_lossy(_chunk).to_string());
        }
        
        Ok(chunks)
    }
}

pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    let chars = text.chars().count();
    
    let chinese_chars = text.chars().filter(|c| *c as u32 > 255).count();
    let chinese_words = chinese_chars / 2;
    let english_words = words.saturating_sub(chinese_words);
    
    let chinese_tokens = (chinese_words as f64 * 1.5) as usize;
    let english_tokens = (english_words as f64 * 0.75) as usize;
    let punctuation = chars.saturating_sub(words);
    let symbols = punctuation / 3;
    
    chinese_tokens + english_tokens + symbols
}
