//! Federated Memory Replication - 联邦记忆复制模块
//!
//! 提供数据复制和同步机制
//! 支持主动复制、被动复制、最终一致性
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use super::protocol::{NodeId, NodeState, FederatedError, FederatedResult};
use super::storage::{
    DistributedMemoryEntry, DistributedStorage, StorageError, StorageResult,
    VectorClock, ConflictResolutionStrategy,
};
use crate::memory::MemoryEntry;

// ============================================================================
// 复制配置
// ============================================================================

/// 复制配置
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// 本地节点 ID
    pub local_node_id: NodeId,
    /// 复制因子
    pub replication_factor: usize,
    /// 写入仲裁数
    pub write_quorum: usize,
    /// 读取仲裁数
    pub read_quorum: usize,
    /// 同步间隔（秒）
    pub sync_interval_secs: u64,
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    /// 重试次数
    pub max_retries: usize,
    /// 重试延迟（毫秒）
    pub retry_delay_ms: u64,
    /// 批量同步大小
    pub batch_sync_size: usize,
    /// 是否启用提示移交
    pub enable_hinted_handoff: bool,
    /// 提示移交过期时间（小时）
    pub hint_expiry_hours: u64,
    /// 是否启用读修复
    pub enable_read_repair: bool,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            local_node_id: NodeId::new(),
            replication_factor: 3,
            write_quorum: 2,
            read_quorum: 2,
            sync_interval_secs: 60,
            heartbeat_interval_secs: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            batch_sync_size: 100,
            enable_hinted_handoff: true,
            hint_expiry_hours: 24,
            enable_read_repair: true,
        }
    }
}

// ============================================================================
// 复制状态
// ============================================================================

/// 复制状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationState {
    /// 未同步
    NotSynced,
    /// 同步中
    Syncing,
    /// 已同步
    Synced,
    /// 同步失败
    Failed,
}

// ============================================================================
// 复制条目
// ============================================================================

/// 复制条目追踪
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationEntry {
    /// 条目 ID
    pub entry_id: String,
    /// 版本
    pub version: u64,
    /// 目标节点
    pub target_node: NodeId,
    /// 复制状态
    pub state: ReplicationState,
    /// 尝试次数
    pub attempts: usize,
    /// 最后尝试时间
    pub last_attempt: Option<DateTime<Utc>>,
    /// 成功时间
    pub synced_at: Option<DateTime<Utc>>,
    /// 错误信息
    pub error: Option<String>,
}

impl ReplicationEntry {
    pub fn new(entry_id: String, version: u64, target_node: NodeId) -> Self {
        Self {
            entry_id,
            version,
            target_node,
            state: ReplicationState::NotSynced,
            attempts: 0,
            last_attempt: None,
            synced_at: None,
            error: None,
        }
    }
    
    /// 标记同步中
    pub fn mark_syncing(&mut self) {
        self.state = ReplicationState::Syncing;
        self.attempts += 1;
        self.last_attempt = Some(Utc::now());
    }
    
    /// 标记成功
    pub fn mark_success(&mut self) {
        self.state = ReplicationState::Synced;
        self.synced_at = Some(Utc::now());
        self.error = None;
    }
    
    /// 标记失败
    pub fn mark_failed(&mut self, error: String) {
        self.state = ReplicationState::Failed;
        self.error = Some(error);
    }
    
    /// 检查是否可重试
    pub fn can_retry(&self, max_retries: usize) -> bool {
        self.attempts < max_retries && self.state != ReplicationState::Synced
    }
}

// ============================================================================
// 提示移交
// ============================================================================

/// 提示（Hint）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hint {
    /// 提示 ID
    pub id: String,
    /// 目标节点
    pub target_node: NodeId,
    /// 条目数据
    pub entry: DistributedMemoryEntry,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 重试次数
    pub retry_count: usize,
}

impl Hint {
    pub fn new(target_node: NodeId, entry: DistributedMemoryEntry, expiry_hours: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            target_node,
            entry,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(expiry_hours as i64),
            retry_count: 0,
        }
    }
    
    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// 增加重试计数
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// 提示存储
pub struct HintStore {
    hints: RwLock<HashMap<String, Hint>>,
    max_hints: usize,
}

impl HintStore {
    pub fn new(max_hints: usize) -> Self {
        Self {
            hints: RwLock::new(HashMap::new()),
            max_hints,
        }
    }
    
    /// 添加提示
    pub async fn add(&self, hint: Hint) -> bool {
        let mut hints = self.hints.write().await;
        
        if hints.len() >= self.max_hints {
            // 移除最旧的提示
            if let Some(oldest_key) = hints.values()
                .min_by_key(|h| h.created_at)
                .map(|h| h.id.clone())
            {
                hints.remove(&oldest_key);
            }
        }
        
        hints.insert(hint.id.clone(), hint);
        true
    }
    
    /// 获取节点的提示
    pub async fn get_for_node(&self, node_id: &NodeId) -> Vec<Hint> {
        let hints = self.hints.read().await;
        hints.values()
            .filter(|h| &h.target_node == node_id)
            .cloned()
            .collect()
    }
    
    /// 移除提示
    pub async fn remove(&self, hint_id: &str) -> Option<Hint> {
        let mut hints = self.hints.write().await;
        hints.remove(hint_id)
    }
    
    /// 清理过期提示
    pub async fn cleanup_expired(&self) -> usize {
        let mut hints = self.hints.write().await;
        let before = hints.len();
        hints.retain(|_, h| !h.is_expired());
        before - hints.len()
    }
    
    /// 获取提示数量
    pub async fn count(&self) -> usize {
        self.hints.read().await.len()
    }
}

// ============================================================================
// 复制事件
// ============================================================================

/// 复制事件
#[derive(Debug, Clone)]
pub enum ReplicationEvent {
    /// 同步开始
    SyncStarted { entry_id: String, target_node: NodeId },
    /// 同步成功
    SyncCompleted { entry_id: String, target_node: NodeId },
    /// 同步失败
    SyncFailed { entry_id: String, target_node: NodeId, error: String },
    /// 提示创建
    HintCreated { hint_id: String, target_node: NodeId },
    /// 提示处理
    HintProcessed { hint_id: String, target_node: NodeId },
    /// 读修复触发
    ReadRepairTriggered { entry_id: String },
}

// ============================================================================
// 复制管理器
// ============================================================================

/// 复制管理器
pub struct ReplicationManager {
    /// 配置
    config: ReplicationConfig,
    /// 存储后端
    storage: Arc<dyn DistributedStorage>,
    /// 复制追踪
    replication_tracker: RwLock<HashMap<String, Vec<ReplicationEntry>>>,
    /// 提示存储
    hint_store: HintStore,
    /// 事件发送器
    event_tx: broadcast::Sender<ReplicationEvent>,
    /// 已知节点
    known_nodes: RwLock<HashSet<NodeId>>,
}

impl ReplicationManager {
    pub fn new(config: ReplicationConfig, storage: Arc<dyn DistributedStorage>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        Self {
            config,
            storage,
            replication_tracker: RwLock::new(HashMap::new()),
            hint_store: HintStore::new(10000),
            event_tx,
            known_nodes: RwLock::new(HashSet::new()),
        }
    }
    
    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<ReplicationEvent> {
        self.event_tx.subscribe()
    }
    
    /// 添加已知节点
    pub async fn add_node(&self, node: NodeId) {
        let mut nodes = self.known_nodes.write().await;
        nodes.insert(node);
    }
    
    /// 移除节点
    pub async fn remove_node(&self, node: &NodeId) {
        let mut nodes = self.known_nodes.write().await;
        nodes.remove(node);
    }
    
    /// 获取复制目标节点
    pub async fn get_replica_nodes(&self, entry_id: &str) -> Vec<NodeId> {
        let nodes = self.known_nodes.read().await;
        
        // 使用一致性哈希确定复制目标
        // 简化实现：选择前 N 个节点
        nodes.iter()
            .filter(|n| **n != self.config.local_node_id)
            .take(self.config.replication_factor)
            .cloned()
            .collect()
    }
    
    /// 复制条目到目标节点
    pub async fn replicate_to(
        &self,
        entry: &DistributedMemoryEntry,
        target_nodes: &[NodeId],
    ) -> StorageResult<()> {
        let entry_id = entry.entry.id.clone();
        
        {
            let mut tracker = self.replication_tracker.write().await;
            
            for target_node in target_nodes {
                let repl_entry = ReplicationEntry::new(
                    entry_id.clone(),
                    entry.version,
                    target_node.clone(),
                );
                
                tracker.entry(entry_id.clone())
                    .or_insert_with(Vec::new)
                    .push(repl_entry);
            }
        }
        
        // 发送事件
        let _ = self.event_tx.send(ReplicationEvent::SyncStarted {
            entry_id: entry_id.clone(),
            target_node: target_nodes[0].clone(),
        });
        
        Ok(())
    }
    
    /// 处理复制（尝试同步到目标节点）
    pub async fn process_replication(
        &self,
        entry: DistributedMemoryEntry,
        target_node: NodeId,
    ) -> StorageResult<()> {
        // 检查节点是否可用
        let nodes = self.known_nodes.read().await;
        let node_available = nodes.contains(&target_node);
        drop(nodes);
        
        if !node_available {
            // 节点不可用，创建提示
            if self.config.enable_hinted_handoff {
                let hint = Hint::new(
                    target_node.clone(),
                    entry,
                    self.config.hint_expiry_hours,
                );
                
                let hint_id = hint.id.clone();
                self.hint_store.add(hint).await;
                
                let _ = self.event_tx.send(ReplicationEvent::HintCreated {
                    hint_id,
                    target_node: target_node.clone(),
                });
            }
            
            return Err(StorageError::NodeUnavailable(target_node.to_string()));
        }
        
        // 同步到目标节点
        let result = self.storage.sync(&entry.entry.id, &[target_node.clone()]).await;
        
        match result {
            Ok(()) => {
                let _ = self.event_tx.send(ReplicationEvent::SyncCompleted {
                    entry_id: entry.entry.id,
                    target_node,
                });
                Ok(())
            }
            Err(e) => {
                let _ = self.event_tx.send(ReplicationEvent::SyncFailed {
                    entry_id: entry.entry.id,
                    target_node: target_node.clone(),
                    error: e.to_string(),
                });
                Err(e)
            }
        }
    }
    
    /// 处理节点的提示
    pub async fn process_hints_for_node(&self, node_id: &NodeId) -> usize {
        let hints = self.hint_store.get_for_node(node_id).await;
        let mut processed = 0;
        
        for hint in hints {
            // 尝试同步
            let result = self.storage.sync(&hint.entry.entry.id, &[node_id.clone()]).await;
            
            if result.is_ok() {
                self.hint_store.remove(&hint.id).await;
                processed += 1;
                
                let _ = self.event_tx.send(ReplicationEvent::HintProcessed {
                    hint_id: hint.id,
                    target_node: node_id.clone(),
                });
            }
        }
        
        processed
    }
    
    /// 读取修复
    pub async fn read_repair(&self, entry_id: &str) -> StorageResult<()> {
        if !self.config.enable_read_repair {
            return Ok(());
        }
        
        let _ = self.event_tx.send(ReplicationEvent::ReadRepairTriggered {
            entry_id: entry_id.to_string(),
        });
        
        // 获取条目
        let entry = self.storage.retrieve(entry_id).await?;
        
        if let Some(entry) = entry {
            // 获取副本节点
            let replica_nodes = self.get_replica_nodes(entry_id).await;
            
            // 同步到所有副本
            for node in replica_nodes {
                self.storage.sync(entry_id, &[node]).await?;
            }
        }
        
        Ok(())
    }
    
    /// 获取复制统计
    pub async fn stats(&self) -> ReplicationStats {
        let tracker = self.replication_tracker.read().await;
        let hints = self.hint_store.count().await;
        
        let mut synced = 0;
        let mut pending = 0;
        let mut failed = 0;
        
        for entries in tracker.values() {
            for entry in entries {
                match entry.state {
                    ReplicationState::Synced => synced += 1,
                    ReplicationState::NotSynced | ReplicationState::Syncing => pending += 1,
                    ReplicationState::Failed => failed += 1,
                }
            }
        }
        
        ReplicationStats {
            synced_entries: synced,
            pending_entries: pending,
            failed_entries: failed,
            pending_hints: hints,
        }
    }
    
    /// 清理过期提示
    pub async fn cleanup(&self) -> usize {
        self.hint_store.cleanup_expired().await
    }
}

/// 复制统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStats {
    /// 已同步条目数
    pub synced_entries: usize,
    /// 待同步条目数
    pub pending_entries: usize,
    /// 失败条目数
    pub failed_entries: usize,
    /// 待处理提示数
    pub pending_hints: usize,
}

// ============================================================================
// 同步协调器
// ============================================================================

/// 同步协调器
pub struct SyncCoordinator {
    /// 配置
    config: ReplicationConfig,
    /// 复制管理器
    replication: Arc<ReplicationManager>,
    /// 存储
    storage: Arc<dyn DistributedStorage>,
    /// 运行标志
    running: RwLock<bool>,
}

impl SyncCoordinator {
    pub fn new(
        config: ReplicationConfig,
        replication: Arc<ReplicationManager>,
        storage: Arc<dyn DistributedStorage>,
    ) -> Self {
        Self {
            config,
            replication,
            storage,
            running: RwLock::new(false),
        }
    }
    
    /// 启动同步
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        *running = true;
    }
    
    /// 停止同步
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
    
    /// 执行同步周期
    pub async fn sync_cycle(&self) -> StorageResult<SyncResult> {
        let running = self.running.read().await;
        if !*running {
            return Ok(SyncResult::default());
        }
        drop(running);
        
        let mut result = SyncResult::default();
        
        // 获取未同步条目
        let unsynced = self.storage.get_unsynced().await?;
        result.total_entries = unsynced.len();
        
        // 批量同步
        for entry in unsynced.iter().take(self.config.batch_sync_size) {
            let replica_nodes = self.replication.get_replica_nodes(&entry.entry.id).await;
            
            match self.replication.process_replication(entry.clone(), replica_nodes[0].clone()).await {
                Ok(()) => {
                    result.synced_entries += 1;
                }
                Err(_) => {
                    result.failed_entries += 1;
                }
            }
        }
        
        // 处理提示
        let known_nodes = self.replication.known_nodes.read().await.clone();
        for node in known_nodes {
            result.hints_processed += self.replication.process_hints_for_node(&node).await;
        }
        
        // 清理过期提示
        result.hints_expired = self.replication.cleanup().await;
        
        Ok(result)
    }
}

/// 同步结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncResult {
    /// 总条目数
    pub total_entries: usize,
    /// 成功同步数
    pub synced_entries: usize,
    /// 失败数
    pub failed_entries: usize,
    /// 处理的提示数
    pub hints_processed: usize,
    /// 过期的提示数
    pub hints_expired: usize,
}

// ============================================================================
// 复制策略 Trait
// ============================================================================

/// 复制策略
#[async_trait]
pub trait ReplicationStrategy: Send + Sync {
    /// 确定复制目标
    async fn get_targets(&self, entry_id: &str) -> Vec<NodeId>;
    
    /// 写入复制
    async fn write(&self, entry: &DistributedMemoryEntry) -> StorageResult<()>;
    
    /// 读取复制
    async fn read(&self, entry_id: &str) -> StorageResult<Option<DistributedMemoryEntry>>;
    
    /// 检查写入仲裁
    fn check_write_quorum(&self, successful: usize) -> bool;
    
    /// 检查读取仲裁
    fn check_read_quorum(&self, successful: usize) -> bool;
}

/// Quorum 复制策略
pub struct QuorumReplicationStrategy {
    config: ReplicationConfig,
    replication: Arc<ReplicationManager>,
    storage: Arc<dyn DistributedStorage>,
}

impl QuorumReplicationStrategy {
    pub fn new(
        config: ReplicationConfig,
        replication: Arc<ReplicationManager>,
        storage: Arc<dyn DistributedStorage>,
    ) -> Self {
        Self {
            config,
            replication,
            storage,
        }
    }
}

#[async_trait]
impl ReplicationStrategy for QuorumReplicationStrategy {
    async fn get_targets(&self, entry_id: &str) -> Vec<NodeId> {
        self.replication.get_replica_nodes(entry_id).await
    }
    
    async fn write(&self, entry: &DistributedMemoryEntry) -> StorageResult<()> {
        let targets = self.get_targets(&entry.entry.id).await;
        
        // 写入本地
        self.storage.receive(entry.clone(), self.config.local_node_id.clone()).await?;
        
        // 复制到其他节点
        let mut successful = 1; // 本地写入成功
        
        for target in targets {
            if target != self.config.local_node_id {
                match self.replication.process_replication(entry.clone(), target.clone()).await {
                    Ok(()) => successful += 1,
                    Err(_) => continue,
                }
            }
            
            if self.check_write_quorum(successful) {
                break;
            }
        }
        
        if self.check_write_quorum(successful) {
            Ok(())
        } else {
            Err(StorageError::ReplicationError("Write quorum not reached".to_string()))
        }
    }
    
    async fn read(&self, entry_id: &str) -> StorageResult<Option<DistributedMemoryEntry>> {
        // 先读本地
        let local_result = self.storage.retrieve(entry_id).await?;
        
        // 如果需要读修复，触发读修复
        if local_result.is_some() {
            self.replication.read_repair(entry_id).await?;
        }
        
        Ok(local_result)
    }
    
    fn check_write_quorum(&self, successful: usize) -> bool {
        successful >= self.config.write_quorum
    }
    
    fn check_read_quorum(&self, successful: usize) -> bool {
        successful >= self.config.read_quorum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryType, MemoryEntry};
    
    fn create_test_entry(id: &str) -> DistributedMemoryEntry {
        let entry = MemoryEntry {
            id: id.to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Fact,
            importance: 0.8,
            content: format!("Content for {}", id),
            metadata: HashMap::new(),
            source_agent: None,
            tags: vec![],
        };
        
        DistributedMemoryEntry::new(entry, NodeId::new())
    }
    
    #[test]
    fn test_replication_entry() {
        let entry = ReplicationEntry::new("test-1".to_string(), 1, NodeId::new());
        
        assert_eq!(entry.state, ReplicationState::NotSynced);
        assert_eq!(entry.attempts, 0);
    }
    
    #[test]
    fn test_replication_entry_transitions() {
        let mut entry = ReplicationEntry::new("test-1".to_string(), 1, NodeId::new());
        
        entry.mark_syncing();
        assert_eq!(entry.state, ReplicationState::Syncing);
        assert_eq!(entry.attempts, 1);
        
        entry.mark_success();
        assert_eq!(entry.state, ReplicationState::Synced);
        
        let mut failed_entry = ReplicationEntry::new("test-2".to_string(), 1, NodeId::new());
        failed_entry.mark_syncing();
        failed_entry.mark_failed("Network error".to_string());
        assert_eq!(failed_entry.state, ReplicationState::Failed);
    }
    
    #[test]
    fn test_hint() {
        let node = NodeId::new();
        let entry = create_test_entry("test-1");
        
        let mut hint = Hint::new(node.clone(), entry, 24);
        
        assert!(!hint.is_expired());
        assert_eq!(hint.retry_count, 0);
        
        hint.increment_retry();
        assert_eq!(hint.retry_count, 1);
    }
    
    #[tokio::test]
    async fn test_hint_store() {
        let store = HintStore::new(100);
        
        let node = NodeId::new();
        let entry = create_test_entry("test-1");
        let hint = Hint::new(node.clone(), entry, 24);
        
        store.add(hint).await;
        
        let hints = store.get_for_node(&node).await;
        assert_eq!(hints.len(), 1);
        
        let count = store.count().await;
        assert_eq!(count, 1);
    }
    
    #[test]
    fn test_replication_config_default() {
        let config = ReplicationConfig::default();
        
        assert_eq!(config.replication_factor, 3);
        assert_eq!(config.write_quorum, 2);
        assert_eq!(config.read_quorum, 2);
    }
    
    #[test]
    fn test_sync_result() {
        let result = SyncResult {
            total_entries: 100,
            synced_entries: 95,
            failed_entries: 5,
            hints_processed: 10,
            hints_expired: 2,
        };
        
        assert_eq!(result.total_entries, 100);
    }
}