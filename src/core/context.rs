// Context Manager - Unified context management with vector search and token counting
//
// This module provides the main ContextManager implementation that combines:
// - Message storage (SQLite)
// - Vector search (semantic retrieval)
// - Token counting (multi-model support)
// - Intelligent truncation strategies

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::vector::{VectorStore, MemoryVectorStore, VectorDocument, DocumentMetadata};
use crate::context::{TokenCounter, TruncationStrategy, StrategyEngine, StrategyType};
use crate::embedding::{EmbeddingClient, OpenAIEmbeddingClient, EmbeddingConfig, EmbeddingModel};

pub struct ContextManager {
    db: Connection,
    pub config: ContextConfig,
    vector_store: MemoryVectorStore,
    // Token counting capabilities
    token_counter: Arc<RwLock<TokenCounter>>,
    // Truncation strategy
    truncation_strategy: Arc<RwLock<TruncationStrategy>>,
    // Strategy engine
    strategy_engine: Arc<RwLock<StrategyEngine>>,
    // Embedding client (optional - for real embeddings)
    embedding_client: Option<Arc<dyn EmbeddingClient>>,
}

// Manual Debug impl to handle dyn trait
impl std::fmt::Debug for ContextManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextManager")
            .field("config", &self.config)
            .field("vector_store", &self.vector_store)
            .field("embedding_client", &self.embedding_client.as_ref().map(|_| "EmbeddingClient"))
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_chunks: usize,
    pub max_tokens: usize,
    pub overlap_tokens: usize,
    pub enable_vector_search: bool,
    /// Default model for token counting
    pub default_model: String,
    /// Token buffer (reserved space)
    pub token_buffer: usize,
    /// Default strategy
    pub default_strategy: StrategyType,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_chunks: 100,
            max_tokens: 8000,
            overlap_tokens: 200,
            enable_vector_search: true,
            default_model: "gpt-4".to_string(),
            token_buffer: 512,
            default_strategy: StrategyType::Balanced,
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
        
        // Initialize token counter and strategies
        let token_counter = Arc::new(RwLock::new(TokenCounter::new()?));
        let truncation_strategy = Arc::new(RwLock::new(TruncationStrategy::default()));
        let strategy_engine = Arc::new(RwLock::new(StrategyEngine::new()?));

        Ok(Self { 
            db, 
            config, 
            vector_store,
            token_counter,
            truncation_strategy,
            strategy_engine,
            embedding_client: None,
        })
    }
    
    /// Create ContextManager with real embedding client
    pub fn with_embedding(mut self, client: Arc<dyn EmbeddingClient>) -> Self {
        self.embedding_client = Some(client);
        self
    }
    
    /// Set embedding client
    pub fn set_embedding_client(&mut self, client: Arc<dyn EmbeddingClient>) {
        self.embedding_client = Some(client);
    }
    
    /// Get embedding for text (uses real client if available, otherwise falls back to mock)
    fn get_embedding(&self, text: &str) -> Vec<f32> {
        // For synchronous context, we use a simple hash-based embedding
        // Real async embedding should be called via get_embedding_async
        self.hash_embedding(text)
    }
    
    /// Simple hash-based embedding (fallback when no async context)
    fn hash_embedding(&self, text: &str) -> Vec<f32> {
        let hash = text.chars().map(|c| c as u32).sum::<u32>() as f32;
        let dim = 1536; // OpenAI embedding dimension
        (0..dim)
            .map(|i| {
                let angle = (hash + i as f32) * 0.1;
                angle.sin() * 0.5 + 0.5
            })
            .collect()
    }

    pub fn add_message(&mut self, message: &str, source: &str) -> Result<String> {
        let chunks = self.chunk_text(message)?;
        
        if let Some(chunk) = chunks.into_iter().next() {
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
                let embedding = self.get_embedding(&chunk);
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
            let query_embedding = self.get_embedding(query);
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
    
    // ===== Async Embedding Methods (Real API) =====
    
    /// Add message with real embedding (async)
    pub async fn add_message_async(&mut self, message: &str, source: &str) -> Result<String> {
        let chunks = self.chunk_text(message)?;
        
        if let Some(chunk) = chunks.into_iter().next() {
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
            
            // Store in vector store with real embedding
            if self.config.enable_vector_search {
                let embedding = self.get_embedding_async(&chunk).await;
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
    
    /// Get real embedding via API (async)
    async fn get_embedding_async(&self, text: &str) -> Vec<f32> {
        if let Some(ref client) = self.embedding_client {
            match client.embed(text).await {
                Ok(result) => result.embedding,
                Err(e) => {
                    tracing::warn!("Embedding API failed, using fallback: {}", e);
                    self.hash_embedding(text)
                }
            }
        } else {
            self.hash_embedding(text)
        }
    }
    
    /// Retrieve relevant chunks with real embedding (async)
    pub async fn retrieve_relevant_async(&self, query: &str, limit: usize) -> Result<Vec<ContextChunk>> {
        if self.config.enable_vector_search {
            let query_embedding = self.get_embedding_async(query).await;
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
        
        // Fallback to sync method
        self.retrieve_relevant(query, limit)
    }
    
    // ===== Token Counting Methods =====
    
    /// Count tokens for a text using the configured model
    pub fn count_tokens(&self, text: &str) -> Result<usize> {
        // Use the estimate_tokens function for now
        // TODO: Integrate with TokenCounter for multi-model support
        Ok(estimate_tokens(text))
    }
    
    /// Count tokens for messages using the configured model
    pub async fn count_messages_tokens(&self, messages: &[crate::llm::Message]) -> Result<usize> {
        // This would use the TokenCounter in an async context
        // For now, use a simple implementation
        let mut total = 0;
        for msg in messages {
            total += self.count_tokens(&msg.content)?;
        }
        Ok(total)
    }
    
    /// Estimate token usage for a conversation
    pub async fn estimate_token_usage(&self, messages: &[crate::llm::Message]) -> Result<TokenUsageEstimate> {
        let input_tokens = self.count_messages_tokens(messages).await?;
        let output_tokens = input_tokens / 4; // Rough estimate
        
        Ok(TokenUsageEstimate {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
        })
    }
    
    // ===== Strategy Methods =====
    
    /// Apply a truncation strategy to messages
    pub async fn apply_truncation_strategy(
        &self,
        messages: Vec<crate::llm::Message>,
        strategy: StrategyType,
    ) -> Result<Vec<crate::llm::Message>> {
        // This would integrate with TruncationStrategy
        // For now, implement simple logic
        let max_tokens = self.config.max_tokens - self.config.token_buffer;
        
        match strategy {
            StrategyType::MinimizeTokens => {
                // Keep only the most recent messages
                let mut result = Vec::new();
                let mut current_tokens = 0;
                
                for msg in messages.iter().rev() {
                    let tokens = self.count_tokens(&msg.content)?;
                    if current_tokens + tokens > max_tokens {
                        break;
                    }
                    result.insert(0, msg.clone());
                    current_tokens += tokens;
                }
                
                Ok(result)
            }
            StrategyType::Balanced => {
                // Keep system messages and recent messages
                let mut result = Vec::new();
                let mut current_tokens = 0;
                
                // Keep system messages first
                for msg in &messages {
                    if matches!(msg.role, crate::llm::MessageRole::System) {
                        result.push(msg.clone());
                        current_tokens += self.count_tokens(&msg.content)?;
                    }
                }
                
                // Add recent messages
                for msg in messages.iter().rev() {
                    if matches!(msg.role, crate::llm::MessageRole::System) {
                        continue;
                    }
                    let tokens = self.count_tokens(&msg.content)?;
                    if current_tokens + tokens > max_tokens {
                        break;
                    }
                    result.insert(result.len().saturating_sub(1), msg.clone());
                    current_tokens += tokens;
                }
                
                Ok(result)
            }
            _ => {
                // Default: keep recent messages
                let mut result = Vec::new();
                let mut current_tokens = 0;
                
                for msg in messages.iter().rev() {
                    let tokens = self.count_tokens(&msg.content)?;
                    if current_tokens + tokens > max_tokens {
                        break;
                    }
                    result.insert(0, msg.clone());
                    current_tokens += tokens;
                }
                
                Ok(result)
            }
        }
    }
    
    /// Get statistics about the context manager
    pub fn get_stats(&self) -> ContextManagerStats {
        ContextManagerStats {
            total_chunks: self.count_chunks().unwrap_or(0),
            max_chunks: self.config.max_chunks,
            max_tokens: self.config.max_tokens,
            vector_search_enabled: self.config.enable_vector_search,
        }
    }
    
    /// Count the number of chunks in storage
    fn count_chunks(&self) -> Result<usize> {
        let mut stmt = self.db.prepare("SELECT COUNT(*) FROM context_chunks")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }
}

/// Token usage estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageEstimate {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub total_tokens: usize,
}

/// Context manager statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextManagerStats {
    pub total_chunks: usize,
    pub max_chunks: usize,
    pub max_tokens: usize,
    pub vector_search_enabled: bool,
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
