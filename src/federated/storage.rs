//! Federated Memory Storage - 联邦记忆分布式存储
//!
//! 提供跨节点的分布式记忆存储
//! 支持最终一致性、冲突解决、数据复制
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use super::protocol::{NodeId, FederatedError, FederatedResult};
use crate::memory::{
    MemoryEntry, MemoryType, UserId,
    HybridSearchConfig, HybridSearchResult, MMRConfig, mmr_diversify,
};

// ============================================================================
// 存储错误
// ============================================================================

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("存储错误: {0}")]
    StorageError(String),
    
    #[error("条目未找到: {0}")]
    NotFound(String),
    
    #[error("冲突错误: {0}")]
    ConflictError(String),
    
    #[error("复制错误: {0}")]
    ReplicationError(String),
    
    #[error("同步错误: {0}")]
    SyncError(String),
    
    #[error("版本冲突: 本地 {local}, 远程 {remote}")]
    VersionConflict { local: u64, remote: u64 },
    
    #[error("节点不可用: {0}")]
    NodeUnavailable(String),
    
    #[error("超时: {0}")]
    Timeout(String),
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

// ============================================================================
// 分布式记忆条目
// ============================================================================

/// 分布式记忆条目（带版本控制）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedMemoryEntry {
    /// 基础记忆条目
    pub entry: MemoryEntry,
    /// 版本号（用于冲突检测）
    pub version: u64,
    /// 来源节点
    pub source_node: NodeId,
    /// 最后更新节点
    pub last_updated_by: NodeId,
    /// 向量时钟（用于冲突解决）
    pub vector_clock: VectorClock,
    /// 是否已同步
    pub synced: bool,
    /// 同步时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<DateTime<Utc>>,
    /// Tombstone 标记（软删除）
    pub tombstone: bool,
}

impl DistributedMemoryEntry {
    pub fn new(entry: MemoryEntry, source_node: NodeId) -> Self {
        Self {
            entry,
            version: 1,
            source_node: source_node.clone(),
            last_updated_by: source_node,
            vector_clock: VectorClock::new(),
            synced: false,
            synced_at: None,
            tombstone: false,
        }
    }
    
    /// 更新条目
    pub fn update(&mut self, updated_entry: MemoryEntry, updated_by: NodeId) {
        self.entry = updated_entry;
        self.version += 1;
        self.last_updated_by = updated_by;
        self.vector_clock.increment(&self.source_node);
        self.synced = false;
        self.synced_at = None;
    }
    
    /// 标记删除
    pub fn mark_deleted(&mut self, deleted_by: NodeId) {
        self.tombstone = true;
        self.version += 1;
        self.last_updated_by = deleted_by;
        self.synced = false;
    }
    
    /// 标记已同步
    pub fn mark_synced(&mut self) {
        self.synced = true;
        self.synced_at = Some(Utc::now());
    }
}

// ============================================================================
// 向量时钟
// ============================================================================

/// 向量时钟（用于分布式系统中的因果一致性）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorClock {
    /// 节点 -> 版本映射
    pub clocks: HashMap<String, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }
    
    /// 增加节点版本
    pub fn increment(&mut self, node_id: &NodeId) {
        let counter = self.clocks.entry(node_id.to_string()).or_insert(0);
        *counter += 1;
    }
    
    /// 合并另一个向量时钟
    pub fn merge(&mut self, other: &VectorClock) {
        for (node, version) in &other.clocks {
            let entry = self.clocks.entry(node.clone()).or_insert(0);
            *entry = (*entry).max(*version);
        }
    }
    
    /// 检查是否发生之前（happened-before）
    pub fn happens_before(&self, other: &VectorClock) -> bool {
        let mut all_less_or_equal = true;
        let mut at_least_one_less = false;
        
        for (node, version) in &self.clocks {
            let other_version = other.clocks.get(node).unwrap_or(&0);
            if version > other_version {
                all_less_or_equal = false;
                break;
            }
            if version < other_version {
                at_least_one_less = true;
            }
        }
        
        // 检查 other 中是否有 self 没有的节点
        for (node, version) in &other.clocks {
            if !self.clocks.contains_key(node) && *version > 0 {
                at_least_one_less = true;
            }
        }
        
        all_less_or_equal && at_least_one_less
    }
    
    /// 检查是否并发
    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        !self.happens_before(other) && !other.happens_before(self) && self != other
    }
    
    /// 获取版本
    pub fn get(&self, node_id: &NodeId) -> u64 {
        *self.clocks.get(&node_id.to_string()).unwrap_or(&0)
    }
}

impl Default for VectorClock {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for VectorClock {
    fn eq(&self, other: &Self) -> bool {
        self.clocks == other.clocks
    }
}

// ============================================================================
// 冲突解决策略
// ============================================================================

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// 最后写入获胜
    LastWriteWins,
    /// 最高版本获胜
    HighestVersionWins,
    /// 向量时钟优先
    VectorClockWins,
    /// 来源节点优先
    SourceNodePriority,
    /// 保留两者
    KeepBoth,
    /// 自定义合并
    CustomMerge,
}

impl Default for ConflictResolutionStrategy {
    fn default() -> Self {
        Self::LastWriteWins
    }
}

/// 冲突记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    /// 冲突 ID
    pub id: String,
    /// 冲突条目 ID
    pub entry_id: String,
    /// 本地版本
    pub local_version: DistributedMemoryEntry,
    /// 远程版本
    pub remote_version: DistributedMemoryEntry,
    /// 冲突时间
    pub detected_at: DateTime<Utc>,
    /// 解决状态
    pub status: ConflictStatus,
    /// 解决策略
    pub resolution: Option<ConflictResolution>,
}

/// 冲突状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStatus {
    Pending,
    Resolving,
    Resolved,
    ManualIntervention,
}

/// 冲突解决结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    /// 解决策略
    pub strategy: ConflictResolutionStrategy,
    /// 获胜条目
    pub winner: DistributedMemoryEntry,
    /// 解决时间
    pub resolved_at: DateTime<Utc>,
}

/// 冲突解决器
pub struct ConflictResolver {
    /// 默认策略
    default_strategy: ConflictResolutionStrategy,
    /// 优先节点列表
    priority_nodes: HashSet<NodeId>,
}

impl ConflictResolver {
    pub fn new(default_strategy: ConflictResolutionStrategy) -> Self {
        Self {
            default_strategy,
            priority_nodes: HashSet::new(),
        }
    }
    
    /// 添加优先节点
    pub fn add_priority_node(&mut self, node_id: NodeId) {
        self.priority_nodes.insert(node_id);
    }
    
    /// 检测冲突
    pub fn detect_conflict(
        &self,
        local: &DistributedMemoryEntry,
        remote: &DistributedMemoryEntry,
    ) -> bool {
        // 同一 ID 但不同版本
        local.entry.id == remote.entry.id 
            && local.version != remote.version
            && !local.vector_clock.happens_before(&remote.vector_clock)
            && !remote.vector_clock.happens_before(&local.vector_clock)
    }
    
    /// 解决冲突
    pub fn resolve(
        &self,
        local: DistributedMemoryEntry,
        remote: DistributedMemoryEntry,
        strategy: Option<ConflictResolutionStrategy>,
    ) -> DistributedMemoryEntry {
        let strategy = strategy.unwrap_or(self.default_strategy);
        
        match strategy {
            ConflictResolutionStrategy::LastWriteWins => {
                if local.entry.last_accessed > remote.entry.last_accessed {
                    local
                } else {
                    remote
                }
            }
            
            ConflictResolutionStrategy::HighestVersionWins => {
                if local.version >= remote.version {
                    local
                } else {
                    remote
                }
            }
            
            ConflictResolutionStrategy::VectorClockWins => {
                if local.vector_clock.happens_before(&remote.vector_clock) {
                    remote
                } else if remote.vector_clock.happens_before(&local.vector_clock) {
                    local
                } else {
                    // 并发，使用 LWW
                    if local.entry.last_accessed > remote.entry.last_accessed {
                        local
                    } else {
                        remote
                    }
                }
            }
            
            ConflictResolutionStrategy::SourceNodePriority => {
                if self.priority_nodes.contains(&local.source_node) {
                    local
                } else if self.priority_nodes.contains(&remote.source_node) {
                    remote
                } else {
                    // 使用 LWW
                    if local.entry.last_accessed > remote.entry.last_accessed {
                        local
                    } else {
                        remote
                    }
                }
            }
            
            ConflictResolutionStrategy::KeepBoth => {
                // 为远程条目创建新 ID
                let mut winner = remote.clone();
                winner.entry.id = format!("{}-{}", remote.entry.id, Uuid::new_v4());
                winner
            }
            
            ConflictResolutionStrategy::CustomMerge => {
                // 自定义合并逻辑
                let mut merged = local.clone();
                merged.version = local.version.max(remote.version) + 1;
                merged.vector_clock.merge(&remote.vector_clock);
                merged.entry.importance = (local.entry.importance + remote.entry.importance) / 2.0;
                merged
            }
        }
    }
}

// ============================================================================
// 分布式存储配置
// ============================================================================

/// 分布式存储配置
#[derive(Debug, Clone)]
pub struct DistributedStorageConfig {
    /// 本地节点 ID
    pub local_node_id: NodeId,
    /// 冲突解决策略
    pub conflict_strategy: ConflictResolutionStrategy,
    /// 同步间隔（秒）
    pub sync_interval_secs: u64,
    /// 最大本地缓存条目数
    pub max_cache_entries: usize,
    /// 是否启用自动同步
    pub auto_sync: bool,
    /// 副本数量
    pub replication_factor: usize,
    /// 写入仲裁数
    pub write_quorum: usize,
    /// 读取仲裁数
    pub read_quorum: usize,
}

impl Default for DistributedStorageConfig {
    fn default() -> Self {
        Self {
            local_node_id: NodeId::new(),
            conflict_strategy: ConflictResolutionStrategy::LastWriteWins,
            sync_interval_secs: 60,
            max_cache_entries: 10000,
            auto_sync: true,
            replication_factor: 3,
            write_quorum: 2,
            read_quorum: 2,
        }
    }
}

// ============================================================================
// 分布式存储 Trait
// ============================================================================

/// 分布式存储 Trait
#[async_trait]
pub trait DistributedStorage: Send + Sync {
    /// 存储条目
    async fn store(&self, entry: &MemoryEntry) -> StorageResult<String>;
    
    /// 批量存储
    async fn store_batch(&self, entries: &[MemoryEntry]) -> StorageResult<Vec<String>>;
    
    /// 获取条目
    async fn retrieve(&self, id: &str) -> StorageResult<Option<DistributedMemoryEntry>>;
    
    /// 删除条目（软删除）
    async fn delete(&self, id: &str) -> StorageResult<()>;
    
    /// 搜索
    async fn search(&self, query: &str, config: &HybridSearchConfig) -> StorageResult<Vec<HybridSearchResult>>;
    
    /// 跨节点搜索
    async fn federated_search(
        &self,
        query: &str,
        config: &HybridSearchConfig,
        nodes: &[NodeId],
    ) -> StorageResult<Vec<HybridSearchResult>>;
    
    /// 同步条目到其他节点
    async fn sync(&self, entry_id: &str, target_nodes: &[NodeId]) -> StorageResult<()>;
    
    /// 接收来自其他节点的条目
    async fn receive(&self, entry: DistributedMemoryEntry, from: NodeId) -> StorageResult<()>;
    
    /// 获取未同步条目
    async fn get_unsynced(&self) -> StorageResult<Vec<DistributedMemoryEntry>>;
    
    /// 获取存储统计
    async fn stats(&self) -> StorageResult<DistributedStorageStats>;
}

/// 分布式存储统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedStorageStats {
    /// 本地条目数
    pub local_entries: usize,
    /// 已同步条目数
    pub synced_entries: usize,
    /// 未同步条目数
    pub unsynced_entries: usize,
    /// 冲突条目数
    pub conflict_entries: usize,
    /// 已删除条目数（tombstone）
    pub tombstone_entries: usize,
    /// 存储大小（字节）
    pub storage_size: u64,
    /// 最后同步时间
    pub last_sync: Option<DateTime<Utc>>,
    /// 节点数
    pub node_count: usize,
}

// ============================================================================
// 本地存储实现
// ============================================================================

/// 本地分布式存储实现
pub struct LocalDistributedStorage {
    /// 本地条目存储
    entries: RwLock<HashMap<String, DistributedMemoryEntry>>,
    /// 配置
    config: DistributedStorageConfig,
    /// 冲突解决器
    conflict_resolver: ConflictResolver,
    /// 事件发送器
    event_tx: broadcast::Sender<StorageEvent>,
    /// 存储统计
    stats: RwLock<DistributedStorageStats>,
}

/// 存储事件
#[derive(Debug, Clone)]
pub enum StorageEvent {
    /// 条目存储
    EntryStored(String),
    /// 条目更新
    EntryUpdated(String),
    /// 条目删除
    EntryDeleted(String),
    /// 条目同步
    EntrySynced { id: String, to_node: NodeId },
    /// 冲突检测
    ConflictDetected(ConflictRecord),
    /// 冲突解决
    ConflictResolved { id: String, strategy: ConflictResolutionStrategy },
}

impl LocalDistributedStorage {
    pub fn new(config: DistributedStorageConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        let stats = DistributedStorageStats {
            local_entries: 0,
            synced_entries: 0,
            unsynced_entries: 0,
            conflict_entries: 0,
            tombstone_entries: 0,
            storage_size: 0,
            last_sync: None,
            node_count: 0,
        };
        
        Self {
            entries: RwLock::new(HashMap::new()),
            config,
            conflict_resolver: ConflictResolver::new(ConflictResolutionStrategy::LastWriteWins),
            event_tx,
            stats: RwLock::new(stats),
        }
    }
    
    /// 订阅存储事件
    pub fn subscribe(&self) -> broadcast::Receiver<StorageEvent> {
        self.event_tx.subscribe()
    }
    
    /// 更新统计
    async fn update_stats(&self) {
        let entries = self.entries.read().await;
        let mut stats = self.stats.write().await;
        
        stats.local_entries = entries.len();
        stats.synced_entries = entries.values().filter(|e| e.synced).count();
        stats.unsynced_entries = entries.values().filter(|e| !e.synced).count();
        stats.tombstone_entries = entries.values().filter(|e| e.tombstone).count();
    }
}

#[async_trait]
impl DistributedStorage for LocalDistributedStorage {
    async fn store(&self, entry: &MemoryEntry) -> StorageResult<String> {
        let mut entries = self.entries.write().await;
        
        let id = entry.id.clone();
        let dist_entry = DistributedMemoryEntry::new(
            entry.clone(),
            self.config.local_node_id.clone(),
        );
        
        entries.insert(id.clone(), dist_entry);
        
        let _ = self.event_tx.send(StorageEvent::EntryStored(id.clone()));
        
        drop(entries);
        self.update_stats().await;
        
        Ok(id)
    }
    
    async fn store_batch(&self, entries: &[MemoryEntry]) -> StorageResult<Vec<String>> {
        let mut ids = Vec::with_capacity(entries.len());
        
        for entry in entries {
            let id = self.store(entry).await?;
            ids.push(id);
        }
        
        Ok(ids)
    }
    
    async fn retrieve(&self, id: &str) -> StorageResult<Option<DistributedMemoryEntry>> {
        let entries = self.entries.read().await;
        
        let entry = entries.get(id).cloned();
        
        if let Some(ref e) = entry {
            if e.tombstone {
                return Ok(None);
            }
        }
        
        Ok(entry)
    }
    
    async fn delete(&self, id: &str) -> StorageResult<()> {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.get_mut(id) {
            entry.mark_deleted(self.config.local_node_id.clone());
            
            let _ = self.event_tx.send(StorageEvent::EntryDeleted(id.to_string()));
        }
        
        drop(entries);
        self.update_stats().await;
        
        Ok(())
    }
    
    async fn search(&self, query: &str, config: &HybridSearchConfig) -> StorageResult<Vec<HybridSearchResult>> {
        let entries = self.entries.read().await;
        
        // 简化的搜索实现
        let mut results: Vec<HybridSearchResult> = entries.values()
            .filter(|e| !e.tombstone)
            .filter(|e| {
                e.entry.content.to_lowercase().contains(&query.to_lowercase())
            })
            .map(|e| HybridSearchResult {
                id: e.entry.id.clone(),
                content: e.entry.content.clone(),
                bm25_score: 1.0,
                vector_score: 0.0,
                final_score: e.entry.importance,
                importance: e.entry.importance,
                created_at: e.entry.created_at.to_rfc3339(),
            })
            .take(config.top_k)
            .collect();
        
        // 排序
        results.sort_by(|a, b| {
            b.final_score.partial_cmp(&a.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(results)
    }
    
    async fn federated_search(
        &self,
        query: &str,
        config: &HybridSearchConfig,
        _nodes: &[NodeId],
    ) -> StorageResult<Vec<HybridSearchResult>> {
        // 本地实现只搜索本地数据
        self.search(query, config).await
    }
    
    async fn sync(&self, entry_id: &str, _target_nodes: &[NodeId]) -> StorageResult<()> {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.get_mut(entry_id) {
            entry.mark_synced();
            
            let _ = self.event_tx.send(StorageEvent::EntrySynced {
                id: entry_id.to_string(),
                to_node: NodeId::new(), // 实际应该是目标节点
            });
        }
        
        drop(entries);
        self.update_stats().await;
        
        Ok(())
    }
    
    async fn receive(&self, entry: DistributedMemoryEntry, _from: NodeId) -> StorageResult<()> {
        let mut entries = self.entries.write().await;
        
        let id = entry.entry.id.clone();
        
        // 检查是否存在冲突
        if let Some(local) = entries.get(&id) {
            if self.conflict_resolver.detect_conflict(local, &entry) {
                // 解决冲突
                let resolved = self.conflict_resolver.resolve(
                    local.clone(),
                    entry,
                    Some(self.config.conflict_strategy),
                );
                
                entries.insert(id, resolved);
            } else {
                // 无冲突，直接更新
                entries.insert(id, entry);
            }
        } else {
            // 新条目，直接插入
            entries.insert(id, entry);
        }
        
        drop(entries);
        self.update_stats().await;
        
        Ok(())
    }
    
    async fn get_unsynced(&self) -> StorageResult<Vec<DistributedMemoryEntry>> {
        let entries = self.entries.read().await;
        
        let unsynced: Vec<DistributedMemoryEntry> = entries.values()
            .filter(|e| !e.synced && !e.tombstone)
            .cloned()
            .collect();
        
        Ok(unsynced)
    }
    
    async fn stats(&self) -> StorageResult<DistributedStorageStats> {
        let stats = self.stats.read().await.clone();
        Ok(stats)
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
    
    #[test]
    fn test_vector_clock() {
        let mut vc1 = VectorClock::new();
        let node1 = NodeId::new();
        
        vc1.increment(&node1);
        assert_eq!(vc1.get(&node1), 1);
        
        vc1.increment(&node1);
        assert_eq!(vc1.get(&node1), 2);
    }
    
    #[test]
    fn test_vector_clock_merge() {
        let mut vc1 = VectorClock::new();
        let mut vc2 = VectorClock::new();
        
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        
        vc1.increment(&node1);
        vc2.increment(&node2);
        
        vc1.merge(&vc2);
        
        assert_eq!(vc1.get(&node1), 1);
        assert_eq!(vc1.get(&node2), 1);
    }
    
    #[test]
    fn test_vector_clock_happens_before() {
        let mut vc1 = VectorClock::new();
        let mut vc2 = VectorClock::new();
        
        let node1 = NodeId::new();
        
        vc1.increment(&node1);
        vc2.increment(&node1);
        vc2.increment(&node1);
        
        assert!(vc1.happens_before(&vc2));
        assert!(!vc2.happens_before(&vc1));
    }
    
    #[test]
    fn test_distributed_memory_entry() {
        let entry = create_test_entry("test-1", "Test content");
        let node = NodeId::new();
        
        let dist_entry = DistributedMemoryEntry::new(entry, node.clone());
        
        assert_eq!(dist_entry.version, 1);
        assert_eq!(dist_entry.source_node, node);
        assert!(!dist_entry.synced);
    }
    
    #[test]
    fn test_distributed_memory_entry_update() {
        let entry = create_test_entry("test-1", "Test content");
        let node = NodeId::new();
        
        let mut dist_entry = DistributedMemoryEntry::new(entry, node.clone());
        
        let updated_entry = create_test_entry("test-1", "Updated content");
        dist_entry.update(updated_entry, node.clone());
        
        assert_eq!(dist_entry.version, 2);
        assert_eq!(dist_entry.entry.content, "Updated content");
    }
    
    #[test]
    fn test_conflict_resolver() {
        let resolver = ConflictResolver::new(ConflictResolutionStrategy::LastWriteWins);
        
        let entry1 = create_test_entry("test-1", "Content 1");
        let entry2 = create_test_entry("test-1", "Content 2");
        
        let mut dist1 = DistributedMemoryEntry::new(entry1, NodeId::new());
        let dist2 = DistributedMemoryEntry::new(entry2, NodeId::new());
        
        // 设置 dist1 更新时间更晚
        dist1.entry.last_accessed = Utc::now() + chrono::Duration::seconds(1);
        
        let resolved = resolver.resolve(dist1, dist2, None);
        
        assert_eq!(resolved.entry.content, "Content 1");
    }
    
    #[tokio::test]
    async fn test_local_distributed_storage() {
        let config = DistributedStorageConfig::default();
        let storage = LocalDistributedStorage::new(config);
        
        let entry = create_test_entry("test-1", "Test content");
        
        // 存储
        let id = storage.store(&entry).await.unwrap();
        assert_eq!(id, "test-1");
        
        // 获取
        let retrieved = storage.retrieve(&id).await.unwrap().unwrap();
        assert_eq!(retrieved.entry.content, "Test content");
        
        // 搜索
        let results = storage.search("Test", &HybridSearchConfig::default()).await.unwrap();
        assert!(!results.is_empty());
        
        // 删除
        storage.delete(&id).await.unwrap();
        let deleted = storage.retrieve(&id).await.unwrap();
        assert!(deleted.is_none());
    }
    
    #[tokio::test]
    async fn test_storage_batch() {
        let config = DistributedStorageConfig::default();
        let storage = LocalDistributedStorage::new(config);
        
        let entries = vec![
            create_test_entry("test-1", "Content 1"),
            create_test_entry("test-2", "Content 2"),
        ];
        
        let ids = storage.store_batch(&entries).await.unwrap();
        assert_eq!(ids.len(), 2);
    }
    
    #[tokio::test]
    async fn test_storage_stats() {
        let config = DistributedStorageConfig::default();
        let storage = LocalDistributedStorage::new(config);
        
        let entry = create_test_entry("test-1", "Test content");
        storage.store(&entry).await.unwrap();
        
        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.local_entries, 1);
        assert_eq!(stats.unsynced_entries, 1);
    }
    
    #[tokio::test]
    async fn test_storage_sync() {
        let config = DistributedStorageConfig::default();
        let storage = LocalDistributedStorage::new(config);
        
        let entry = create_test_entry("test-1", "Test content");
        storage.store(&entry).await.unwrap();
        
        // 获取未同步条目
        let unsynced = storage.get_unsynced().await.unwrap();
        assert_eq!(unsynced.len(), 1);
        
        // 同步
        storage.sync("test-1", &[]).await.unwrap();
        
        // 再次获取
        let unsynced = storage.get_unsynced().await.unwrap();
        assert!(unsynced.is_empty());
    }
    
    #[tokio::test]
    async fn test_storage_receive() {
        let config = DistributedStorageConfig::default();
        let storage = LocalDistributedStorage::new(config);
        
        let entry = create_test_entry("test-1", "Remote content");
        let remote_node = NodeId::new();
        let dist_entry = DistributedMemoryEntry::new(entry, remote_node.clone());
        
        storage.receive(dist_entry, remote_node).await.unwrap();
        
        let retrieved = storage.retrieve("test-1").await.unwrap().unwrap();
        assert_eq!(retrieved.entry.content, "Remote content");
    }
}