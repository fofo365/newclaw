// Context Transparency - v0.5.3
//
// 上下文透明管理：操作可追溯、可回滚

use crate::core::context::{ContextChunk, ContextMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// 变更操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChangeOperation {
    /// 添加
    Add,
    /// 更新
    Update,
    /// 删除
    Delete,
    /// 截断
    Truncate,
    /// 压缩
    Compress,
    /// 恢复
    Restore,
}

/// 上下文变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChange {
    /// 变更 ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 操作类型
    pub operation: ChangeOperation,
    /// 变更前的值
    pub before: Option<String>,
    /// 变更后的值
    pub after: Option<String>,
    /// 受影响的 Chunk ID
    pub chunk_id: Option<String>,
    /// 操作者
    pub actor: String,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ContextChange {
    /// 创建添加变更
    pub fn add(chunk_id: &str, content: &str, actor: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation: ChangeOperation::Add,
            before: None,
            after: Some(content.to_string()),
            chunk_id: Some(chunk_id.to_string()),
            actor: actor.to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// 创建更新变更
    pub fn update(chunk_id: &str, before: &str, after: &str, actor: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation: ChangeOperation::Update,
            before: Some(before.to_string()),
            after: Some(after.to_string()),
            chunk_id: Some(chunk_id.to_string()),
            actor: actor.to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// 创建删除变更
    pub fn delete(chunk_id: &str, content: &str, actor: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            operation: ChangeOperation::Delete,
            before: Some(content.to_string()),
            after: None,
            chunk_id: Some(chunk_id.to_string()),
            actor: actor.to_string(),
            metadata: HashMap::new(),
        }
    }
}

/// 上下文快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// 快照 ID
    pub id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 快照名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 所有 Chunk
    pub chunks: Vec<ContextChunk>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ContextSnapshot {
    /// 创建新快照
    pub fn new(name: &str, chunks: Vec<ContextChunk>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            name: name.to_string(),
            description: String::new(),
            chunks,
            metadata: HashMap::new(),
        }
    }
    
    /// 添加描述
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }
}

/// 透明管理器
pub struct TransparencyManager {
    /// 变更历史
    changes: Vec<ContextChange>,
    /// 快照存储
    snapshots: HashMap<String, ContextSnapshot>,
    /// 最大历史长度
    max_history: usize,
}

impl TransparencyManager {
    /// 创建新的透明管理器
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
            snapshots: HashMap::new(),
            max_history: 1000,
        }
    }
    
    /// 设置最大历史长度
    pub fn with_max_history(mut self, max: usize) -> Self {
        self.max_history = max;
        self
    }
    
    /// 记录变更
    pub fn record(&mut self, change: ContextChange) {
        self.changes.push(change);
        
        // 清理过期历史
        if self.changes.len() > self.max_history {
            let remove_count = self.changes.len() - self.max_history;
            self.changes.drain(0..remove_count);
        }
    }
    
    /// 创建快照
    pub fn create_snapshot(&mut self, name: &str, chunks: Vec<ContextChunk>) -> String {
        let snapshot = ContextSnapshot::new(name, chunks);
        let id = snapshot.id.clone();
        self.snapshots.insert(id.clone(), snapshot);
        id
    }
    
    /// 获取快照
    pub fn get_snapshot(&self, id: &str) -> Option<&ContextSnapshot> {
        self.snapshots.get(id)
    }
    
    /// 列出所有快照
    pub fn list_snapshots(&self) -> Vec<&ContextSnapshot> {
        self.snapshots.values().collect()
    }
    
    /// 删除快照
    pub fn delete_snapshot(&mut self, id: &str) -> Result<()> {
        self.snapshots.remove(id)
            .map(|_| ())
            .ok_or_else(|| anyhow::anyhow!("Snapshot not found: {}", id))
    }
    
    /// 获取变更历史
    pub fn get_history(&self, limit: usize) -> Vec<&ContextChange> {
        self.changes.iter().rev().take(limit).collect()
    }
    
    /// 按 Chunk ID 获取变更历史
    pub fn get_history_for_chunk(&self, chunk_id: &str) -> Vec<&ContextChange> {
        self.changes.iter()
            .filter(|c| c.chunk_id.as_deref() == Some(chunk_id))
            .collect()
    }
    
    /// 获取最近的变更
    pub fn get_recent_changes(&self, count: usize) -> Vec<&ContextChange> {
        self.changes.iter().rev().take(count).collect()
    }
    
    /// 按时间范围获取变更
    pub fn get_changes_in_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<&ContextChange> {
        self.changes.iter()
            .filter(|c| c.timestamp >= start && c.timestamp <= end)
            .collect()
    }
    
    /// 获取变更统计
    pub fn get_statistics(&self) -> ChangeStatistics {
        let mut stats = ChangeStatistics::default();
        
        for change in &self.changes {
            match change.operation {
                ChangeOperation::Add => stats.adds += 1,
                ChangeOperation::Update => stats.updates += 1,
                ChangeOperation::Delete => stats.deletes += 1,
                ChangeOperation::Truncate => stats.truncates += 1,
                ChangeOperation::Compress => stats.compresses += 1,
                ChangeOperation::Restore => stats.restores += 1,
            }
        }
        
        stats.total = self.changes.len();
        stats.snapshot_count = self.snapshots.len();
        
        stats
    }
    
    /// 导出变更历史
    pub fn export_history(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.changes)
            .map_err(|e| anyhow::anyhow!("Failed to export history: {}", e))
    }
    
    /// 清空历史
    pub fn clear_history(&mut self) {
        self.changes.clear();
    }
}

impl Default for TransparencyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 变更统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChangeStatistics {
    pub total: usize,
    pub adds: usize,
    pub updates: usize,
    pub deletes: usize,
    pub truncates: usize,
    pub compresses: usize,
    pub restores: usize,
    pub snapshot_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_change_add() {
        let change = ContextChange::add("chunk-1", "test content", "user");
        assert_eq!(change.operation, ChangeOperation::Add);
        assert!(change.before.is_none());
        assert!(change.after.is_some());
    }

    #[test]
    fn test_context_change_update() {
        let change = ContextChange::update("chunk-1", "old", "new", "user");
        assert_eq!(change.operation, ChangeOperation::Update);
        assert!(change.before.is_some());
        assert!(change.after.is_some());
    }

    #[test]
    fn test_context_snapshot() {
        let snapshot = ContextSnapshot::new("test", vec![]);
        assert!(!snapshot.id.is_empty());
        assert_eq!(snapshot.name, "test");
    }

    #[test]
    fn test_transparency_manager_record() {
        let mut manager = TransparencyManager::new();
        let change = ContextChange::add("chunk-1", "test", "user");
        
        manager.record(change);
        assert_eq!(manager.get_history(10).len(), 1);
    }

    #[test]
    fn test_transparency_manager_snapshot() {
        let mut manager = TransparencyManager::new();
        let chunks = vec![
            ContextChunk {
                id: "1".to_string(),
                text: "test".to_string(),
                tokens: 5,
                created_at: 0,
                metadata: ContextMetadata {
                    source: "test".to_string(),
                    timestamp: 0,
                    message_type: "user".to_string(),
                },
            }
        ];
        
        let id = manager.create_snapshot("test-snapshot", chunks);
        assert!(manager.get_snapshot(&id).is_some());
    }

    #[test]
    fn test_transparency_manager_statistics() {
        let mut manager = TransparencyManager::new();
        
        manager.record(ContextChange::add("1", "test", "user"));
        manager.record(ContextChange::add("2", "test", "user"));
        manager.record(ContextChange::update("1", "old", "new", "user"));
        
        let stats = manager.get_statistics();
        assert_eq!(stats.adds, 2);
        assert_eq!(stats.updates, 1);
        assert_eq!(stats.total, 3);
    }
}
