//! Federated Memory Cache - 联邦记忆本地缓存
//!
//! 提供本地缓存机制，减少跨节点查询
//! 支持 LRU 淘汰、TTL 过期、缓存预热
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::num::NonZeroUsize;

use super::protocol::NodeId;
use super::storage::DistributedMemoryEntry;
use crate::memory::{MemoryEntry, HybridSearchResult};

// ============================================================================
// 缓存配置
// ============================================================================

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 最大条目数
    pub max_entries: usize,
    /// 默认 TTL（秒）
    pub default_ttl_secs: u64,
    /// 是否启用预热
    pub enable_warmup: bool,
    /// 预热批量大小
    pub warmup_batch_size: usize,
    /// 是否启用后台清理
    pub enable_background_cleanup: bool,
    /// 清理间隔（秒）
    pub cleanup_interval_secs: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            default_ttl_secs: 300, // 5 分钟
            enable_warmup: true,
            warmup_batch_size: 100,
            enable_background_cleanup: true,
            cleanup_interval_secs: 60,
        }
    }
}

// ============================================================================
// 缓存条目
// ============================================================================

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// 数据
    pub data: T,
    /// 缓存时间
    pub cached_at: Instant,
    /// 过期时间
    pub expires_at: Option<Instant>,
    /// 来源节点
    pub source_node: Option<NodeId>,
    /// 访问计数
    pub access_count: u64,
    /// 最后访问时间
    pub last_accessed: Instant,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl<T: Clone> CacheEntry<T> {
    pub fn new(data: T, ttl_secs: Option<u64>) -> Self {
        let now = Instant::now();
        let expires_at = ttl_secs.map(|ttl| now + Duration::from_secs(ttl));
        
        Self {
            data,
            cached_at: now,
            expires_at,
            source_node: None,
            access_count: 0,
            last_accessed: now,
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_source(mut self, source: NodeId) -> Self {
        self.source_node = Some(source);
        self
    }
    
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }
    
    /// 记录访问
    pub fn touch(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }
    
    /// 获取存活时间
    pub fn age(&self) -> Duration {
        Instant::now().duration_since(self.cached_at)
    }
}

// ============================================================================
// 缓存统计
// ============================================================================

/// 缓存统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// 总条目数
    pub total_entries: usize,
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 驱逐次数
    pub evictions: u64,
    /// 过期次数
    pub expirations: u64,
    /// 更新次数
    pub updates: u64,
    /// 预热次数
    pub warmups: u64,
}

impl CacheStats {
    /// 获取命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ============================================================================
// 记忆缓存
// ============================================================================

/// 记忆缓存
pub struct MemoryCache {
    /// 条目缓存（ID -> 缓存条目）
    entries: RwLock<HashMap<String, CacheEntry<DistributedMemoryEntry>>>,
    /// 查询结果缓存
    query_cache: RwLock<HashMap<String, CacheEntry<Vec<HybridSearchResult>>>>,
    /// 配置
    config: CacheConfig,
    /// 统计
    stats: RwLock<CacheStats>,
}

impl MemoryCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            query_cache: RwLock::new(HashMap::new()),
            config,
            stats: RwLock::new(CacheStats::default()),
        }
    }
    
    /// 获取条目
    pub async fn get(&self, id: &str) -> Option<DistributedMemoryEntry> {
        let mut entries = self.entries.write().await;
        
        if let Some(entry) = entries.get_mut(id) {
            if entry.is_expired() {
                entries.remove(id);
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                stats.expirations += 1;
                return None;
            }
            
            entry.touch();
            let mut stats = self.stats.write().await;
            stats.hits += 1;
            
            return Some(entry.data.clone());
        }
        
        let mut stats = self.stats.write().await;
        stats.misses += 1;
        None
    }
    
    /// 存储条目
    pub async fn put(&self, id: String, entry: DistributedMemoryEntry) {
        let ttl = Some(self.config.default_ttl_secs);
        let cache_entry = CacheEntry::new(entry, ttl);
        
        let mut entries = self.entries.write().await;
        
        if entries.contains_key(&id) {
            let mut stats = self.stats.write().await;
            stats.updates += 1;
        }
        
        // LRU 淘汰：如果超过最大条目数，移除最久未访问的
        if entries.len() >= self.config.max_entries {
            let oldest = entries.iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = oldest {
                entries.remove(&key);
            }
        }
        
        entries.insert(id, cache_entry);
        
        let mut stats = self.stats.write().await;
        stats.total_entries = entries.len();
    }
    
    /// 存储条目（带来源节点）
    pub async fn put_with_source(&self, id: String, entry: DistributedMemoryEntry, source: NodeId) {
        let ttl = Some(self.config.default_ttl_secs);
        let cache_entry = CacheEntry::new(entry, ttl).with_source(source);
        
        let mut entries = self.entries.write().await;
        
        // LRU 淘汰
        if entries.len() >= self.config.max_entries {
            let oldest = entries.iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = oldest {
                entries.remove(&key);
            }
        }
        
        entries.insert(id, cache_entry);
        
        let mut stats = self.stats.write().await;
        stats.total_entries = entries.len();
    }
    
    /// 移除条目
    pub async fn remove(&self, id: &str) -> bool {
        let mut entries = self.entries.write().await;
        let removed = entries.remove(id).is_some();
        
        if removed {
            let mut stats = self.stats.write().await;
            stats.evictions += 1;
            stats.total_entries = entries.len();
        }
        
        removed
    }
    
    /// 获取查询结果缓存
    pub async fn get_query(&self, query_hash: &str) -> Option<Vec<HybridSearchResult>> {
        let mut query_cache = self.query_cache.write().await;
        
        if let Some(entry) = query_cache.get_mut(query_hash) {
            if entry.is_expired() {
                query_cache.remove(query_hash);
                return None;
            }
            
            entry.touch();
            return Some(entry.data.clone());
        }
        
        None
    }
    
    /// 存储查询结果缓存
    pub async fn put_query(&self, query_hash: String, results: Vec<HybridSearchResult>) {
        let ttl = Some(self.config.default_ttl_secs);
        let cache_entry = CacheEntry::new(results, ttl);
        
        let mut query_cache = self.query_cache.write().await;
        
        // LRU 淘汰
        if query_cache.len() >= self.config.max_entries {
            let oldest = query_cache.iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = oldest {
                query_cache.remove(&key);
            }
        }
        
        query_cache.insert(query_hash, cache_entry);
    }
    
    /// 清空缓存
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        
        let mut query_cache = self.query_cache.write().await;
        query_cache.clear();
        
        let mut stats = self.stats.write().await;
        stats.total_entries = 0;
    }
    
    /// 清理过期条目
    pub async fn cleanup_expired(&self) -> usize {
        let mut count = 0;
        
        {
            let mut entries = self.entries.write().await;
            let expired_ids: Vec<String> = entries.iter()
                .filter(|(_, e)| e.is_expired())
                .map(|(id, _)| id.clone())
                .collect();
            
            for id in expired_ids {
                entries.remove(&id);
                count += 1;
            }
        }
        
        {
            let mut query_cache = self.query_cache.write().await;
            let expired_keys: Vec<String> = query_cache.iter()
                .filter(|(_, e)| e.is_expired())
                .map(|(k, _)| k.clone())
                .collect();
            
            for k in expired_keys {
                query_cache.remove(&k);
                count += 1;
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.expirations += count as u64;
        
        count
    }
    
    /// 获取统计
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
    
    /// 获取缓存大小
    pub async fn size(&self) -> usize {
        self.entries.read().await.len()
    }
    
    /// 检查是否包含
    pub async fn contains(&self, id: &str) -> bool {
        self.entries.read().await.contains_key(id)
    }
    
    /// 批量获取
    pub async fn get_batch(&self, ids: &[String]) -> HashMap<String, DistributedMemoryEntry> {
        let mut result = HashMap::new();
        let mut entries = self.entries.write().await;
        
        for id in ids {
            if let Some(entry) = entries.get_mut(id) {
                if !entry.is_expired() {
                    entry.touch();
                    result.insert(id.clone(), entry.data.clone());
                }
            }
        }
        
        let mut stats = self.stats.write().await;
        stats.hits += result.len() as u64;
        stats.misses += (ids.len() - result.len()) as u64;
        
        result
    }
    
    /// 批量存储
    pub async fn put_batch(&self, items: Vec<(String, DistributedMemoryEntry)>) {
        let ttl = Some(self.config.default_ttl_secs);
        
        let mut entries = self.entries.write().await;
        
        for (id, entry) in items {
            let cache_entry = CacheEntry::new(entry, ttl);
            
            // LRU 淘汰
            if entries.len() >= self.config.max_entries {
                let oldest = entries.iter()
                    .min_by_key(|(_, e)| e.last_accessed)
                    .map(|(k, _)| k.clone());
                
                if let Some(key) = oldest {
                    entries.remove(&key);
                }
            }
            
            entries.insert(id, cache_entry);
        }
        
        let mut stats = self.stats.write().await;
        stats.total_entries = entries.len();
    }
    
    /// 预热缓存
    pub async fn warmup(&self, entries: Vec<(String, DistributedMemoryEntry)>) -> usize {
        let batch_size = self.config.warmup_batch_size.min(entries.len());
        let items: Vec<_> = entries.into_iter().take(batch_size).collect();
        let count = items.len();
        
        self.put_batch(items).await;
        
        let mut stats = self.stats.write().await;
        stats.warmups += count as u64;
        
        count
    }
}

// ============================================================================
// 查询哈希
// ============================================================================

/// 计算查询哈希
pub fn query_hash(query: &str, limit: usize, offset: usize) -> String {
    use sha1::{Digest, Sha1};
    
    let mut hasher = Sha1::new();
    hasher.update(query.as_bytes());
    hasher.update(limit.to_le_bytes());
    hasher.update(offset.to_le_bytes());
    
    format!("{:x}", hasher.finalize())
}

// ============================================================================
// 缓存键生成
// ============================================================================

/// 缓存键生成器
pub struct CacheKeyBuilder {
    prefix: String,
}

impl CacheKeyBuilder {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
    
    /// 生成条目键
    pub fn entry_key(&self, id: &str) -> String {
        format!("{}:entry:{}", self.prefix, id)
    }
    
    /// 生成查询键
    pub fn query_key(&self, query: &str, limit: usize, offset: usize) -> String {
        format!("{}:query:{}", self.prefix, query_hash(query, limit, offset))
    }
    
    /// 生成节点键
    pub fn node_key(&self, node_id: &NodeId) -> String {
        format!("{}:node:{}", self.prefix, node_id)
    }
    
    /// 生成用户键
    pub fn user_key(&self, user_id: &str) -> String {
        format!("{}:user:{}", self.prefix, user_id)
    }
}

// ============================================================================
// 缓存策略
// ============================================================================

/// 缓存策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStrategy {
    /// LRU 淘汰
    LRU,
    /// LFU 淘汰
    LFU,
    /// FIFO 淘汰
    FIFO,
    /// TTL 过期
    TTL,
    /// 混合策略
    Hybrid,
}

impl Default for CacheStrategy {
    fn default() -> Self {
        Self::LRU
    }
}

// ============================================================================
// 两级缓存
// ============================================================================

/// 两级缓存（内存 + 持久化）
pub struct TwoLevelCache {
    /// 一级缓存（内存）
    l1: MemoryCache,
    /// 二级缓存大小
    l2_size: usize,
    /// 配置
    config: CacheConfig,
}

impl TwoLevelCache {
    pub fn new(config: CacheConfig) -> Self {
        let l1_config = CacheConfig {
            max_entries: config.max_entries / 2,
            ..config.clone()
        };
        
        Self {
            l1: MemoryCache::new(l1_config),
            l2_size: config.max_entries / 2,
            config,
        }
    }
    
    /// 获取条目（先查 L1，再查 L2）
    pub async fn get(&self, id: &str) -> Option<DistributedMemoryEntry> {
        // 先查 L1
        if let Some(entry) = self.l1.get(id).await {
            return Some(entry);
        }
        
        // 实际实现中，这里应该查询 L2（如 SQLite）
        None
    }
    
    /// 存储条目（存入 L1，异步写回 L2）
    pub async fn put(&self, id: String, entry: DistributedMemoryEntry) {
        // 写入 L1
        self.l1.put(id.clone(), entry.clone()).await;
        
        // 异步写入 L2（实际实现中）
    }
    
    /// 清空缓存
    pub async fn clear(&self) {
        self.l1.clear().await;
    }
    
    /// 获取 L1 统计
    pub async fn l1_stats(&self) -> CacheStats {
        self.l1.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryType, MemoryEntry};
    use std::collections::HashMap;
    
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
    
    #[tokio::test]
    async fn test_cache_put_get() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let entry = create_test_entry("test-1");
        cache.put("test-1".to_string(), entry.clone()).await;
        
        let retrieved = cache.get("test-1").await.unwrap();
        assert_eq!(retrieved.entry.id, "test-1");
    }
    
    #[tokio::test]
    async fn test_cache_miss() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let result = cache.get("nonexistent").await;
        assert!(result.is_none());
        
        let stats = cache.stats().await;
        assert_eq!(stats.misses, 1);
    }
    
    #[tokio::test]
    async fn test_cache_remove() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let entry = create_test_entry("test-1");
        cache.put("test-1".to_string(), entry).await;
        
        assert!(cache.remove("test-1").await);
        assert!(!cache.contains("test-1").await);
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let entry = create_test_entry("test-1");
        cache.put("test-1".to_string(), entry).await;
        
        // 命中
        cache.get("test-1").await;
        
        // 未命中
        cache.get("nonexistent").await;
        
        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 0.01);
    }
    
    #[tokio::test]
    async fn test_cache_batch() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let items = vec![
            ("test-1".to_string(), create_test_entry("test-1")),
            ("test-2".to_string(), create_test_entry("test-2")),
        ];
        
        cache.put_batch(items).await;
        
        let ids = vec!["test-1".to_string(), "test-2".to_string()];
        let result = cache.get_batch(&ids).await;
        
        assert_eq!(result.len(), 2);
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        cache.put("test-1".to_string(), create_test_entry("test-1")).await;
        cache.put("test-2".to_string(), create_test_entry("test-2")).await;
        
        cache.clear().await;
        
        assert_eq!(cache.size().await, 0);
    }
    
    #[test]
    fn test_cache_entry_expiry() {
        let entry: CacheEntry<String> = CacheEntry::new("test".to_string(), Some(0));
        
        // TTL 为 0，应该立即过期
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(entry.is_expired());
    }
    
    #[test]
    fn test_cache_entry_touch() {
        let mut entry: CacheEntry<String> = CacheEntry::new("test".to_string(), None);
        
        assert_eq!(entry.access_count, 0);
        
        entry.touch();
        assert_eq!(entry.access_count, 1);
        
        entry.touch();
        assert_eq!(entry.access_count, 2);
    }
    
    #[test]
    fn test_query_hash() {
        let hash1 = query_hash("test", 10, 0);
        let hash2 = query_hash("test", 10, 0);
        let hash3 = query_hash("test", 20, 0);
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
    
    #[test]
    fn test_cache_key_builder() {
        let builder = CacheKeyBuilder::new("federated");
        
        let entry_key = builder.entry_key("123");
        assert!(entry_key.starts_with("federated:entry:"));
        
        let query_key = builder.query_key("test", 10, 0);
        assert!(query_key.starts_with("federated:query:"));
    }
    
    #[tokio::test]
    async fn test_cache_warmup() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let items = vec![
            ("test-1".to_string(), create_test_entry("test-1")),
            ("test-2".to_string(), create_test_entry("test-2")),
        ];
        
        let count = cache.warmup(items).await;
        assert_eq!(count, 2);
        
        let stats = cache.stats().await;
        assert_eq!(stats.warmups, 2);
    }
    
    #[tokio::test]
    async fn test_query_cache() {
        let config = CacheConfig::default();
        let cache = MemoryCache::new(config);
        
        let hash = query_hash("test query", 10, 0);
        let results = vec![
            HybridSearchResult {
                id: "1".to_string(),
                content: "Result 1".to_string(),
                bm25_score: 1.0,
                vector_score: 0.0,
                final_score: 1.0,
                importance: 0.8,
                created_at: "2026-03-14".to_string(),
            },
        ];
        
        cache.put_query(hash.clone(), results.clone()).await;
        
        let cached = cache.get_query(&hash).await.unwrap();
        assert_eq!(cached.len(), 1);
    }
    
    #[tokio::test]
    async fn test_two_level_cache() {
        let config = CacheConfig {
            max_entries: 100,
            ..Default::default()
        };
        let cache = TwoLevelCache::new(config);
        
        let entry = create_test_entry("test-1");
        cache.put("test-1".to_string(), entry).await;
        
        let retrieved = cache.get("test-1").await.unwrap();
        assert_eq!(retrieved.entry.id, "test-1");
    }
}