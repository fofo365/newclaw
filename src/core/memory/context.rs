// Context Manager

use crate::core::{agent::AgentState, agent::AgentEngine};

pub struct ContextManager {
    db: rusqlite::Connection,
    max_context_chunks: usize,
    max_tokens: usize,
}

pub struct ContextMessage {
    content: String,
    tokens: usize,
    timestamp: i64,
    metadata: String,
}

pub struct ContextChunk {
    id: String,
    text: String,
    tokens: usize,
    created_at: i64,
    source: String,
}

impl ContextManager {
    /// 创建新的 Context Manager
    pub fn new(max_context_chunks: usize, max_tokens: usize) -> Result<Self> {
        let db = rusqlite::Connection::connect(
            "file:/var/lib/newclaw/context.db"
        )?;

        // 初始化数据库
        db.execute(
            "CREATE TABLE IF NOT EXISTS context_messages (
                id INTEGER PRIMARY KEY,
                content TEXT NOT NULL,
                tokens INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                metadata TEXT
            )"
        )?;
        
        // 创建索引
        db.execute(
            "CREATE INDEX IF NOT EXISTS context_messages_by_timestamp \
             ON context_messages(timestamp DESC"
        )?;

        Ok(Self {
            db,
            max_context_chunks,
            max_tokens,
        })
    }

    /// 添加消息到上下文
    pub async fn add_message(&mut self, content: &str) -> Result<()> {
        let tokens = crate::core::estimate_tokens(content);
        let timestamp = chrono::Utc::now().timestamp();
        let metadata = serde_json::json!(to_string).unwrap_or_default);

        self.db.execute(
            "INSERT INTO context_messages (content, tokens, timestamp, metadata) \
             VALUES (?1, ?2, ?3, ?4)",
            [content, tokens, timestamp, metadata],
        )?;
        
        Ok(())
    }

    /// 获取最近的上下文
    pub async fn get_recent_context(&self) -> Result<Vec<ContextMessage>> {
        let mut messages = Vec::new();
        
        let mut stmt = self.db.prepare(
            "SELECT content, tokens, timestamp, metadata \
             FROM context_messages \
             ORDER BY timestamp DESC \
             LIMIT ?1"
        )?;

        let mut rows = stmt.query(rs, |row| {
            let content: String = row.get(0)?;
            let tokens: i32 = row.get(1)?;
            let timestamp: i64 = row.get(2)?;
            let metadata: String = row.get(3)?;
            
            messages.push(ContextMessage {
                content,
                tokens,
                timestamp,
                metadata,
            });
        })?;

        Ok(messages)
    }
}
