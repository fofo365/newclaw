// Embedding Cache - v0.5.0
//
// 嵌入缓存机制：
// - 内存缓存
// - 可选持久化 (Redis)
// - LRU 淘汰策略
// - 缓存统计

use super::{EmbeddingResult, EmbeddingError};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 缓存配置
#[derive(Debug, Clone, Copy)]
pub struct CacheConfig {
    /// 最大条目数
    pub max_entries: usize,
    /// TTL
    pub ttl: Duration,
    /// 是否启用统计
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10_000,
            ttl: Duration::from_secs(3600 * 24), // 24 小时
            enable_stats: true,
        }
    }
}

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 嵌入结果
    result: EmbeddingResult,
    /// 创建时间
    created_at: Instant,
    /// 访问次数
    access_count: usize,
    /// 最后访问时间
    last_access: Instant,
}

/// 缓存统计
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// 命中次数
    pub hits: usize,
    /// 未命中次数
    pub misses: usize,
    /// 总请求数
    pub total_requests: usize,
}

impl CacheStats {
    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.hits as f64 / self.total_requests as f64
    }
}

/// 嵌入缓存
pub struct EmbeddingCache {
    /// 缓存存储
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// 配置
    config: CacheConfig,
    /// 统计信息
    stats: Arc<RwLock<CacheStats>>,
}

impl EmbeddingCache {
    /// 创建新的缓存
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// 获取缓存条目
    pub async fn get(&self, key: &str) -> Option<EmbeddingResult> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(key) {
            // 检查是否过期
            if entry.created_at.elapsed() > self.config.ttl {
                cache.remove(key);
                self.update_stats(false).await;
                return None;
            }

            // 更新访问信息
            entry.access_count += 1;
            entry.last_access = Instant::now();

            self.update_stats(true).await;
            Some(entry.result.clone())
        } else {
            self.update_stats(false).await;
            None
        }
    }

    /// 插入缓存条目
    pub async fn put(&self, key: String, result: EmbeddingResult) {
        let mut cache = self.cache.write().await;

        // 检查是否需要淘汰
        if cache.len() >= self.config.max_entries && !cache.contains_key(&key) {
            self.evict_lru(&mut cache).await;
        }

        let entry = CacheEntry {
            result,
            created_at: Instant::now(),
            access_count: 0,
            last_access: Instant::now(),
        };

        cache.insert(key, entry);
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 获取缓存大小
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// 获取统计信息
    pub async fn stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// 淘汰 LRU 条目
    async fn evict_lru(&self, cache: &mut HashMap<String, CacheEntry>) {
        if let Some(lru_key) = cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| key.clone())
        {
            cache.remove(&lru_key);
        }
    }

    /// 更新统计信息
    async fn update_stats(&self, hit: bool) {
        if !self.config.enable_stats {
            return;
        }

        let mut stats = self.stats.write().await;
        if hit {
            stats.hits += 1;
        } else {
            stats.misses += 1;
        }
        stats.total_requests += 1;
    }

    /// 清理过期条目
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();

        cache.retain(|_, entry| now.duration_since(entry.created_at) < self.config.ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_put_get() {
        let cache = EmbeddingCache::new(CacheConfig::default());

        let result = EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "test".to_string(),
            tokens: 10,
            duration: Duration::from_millis(100),
        };

        cache.put("key1".to_string(), result.clone()).await;
        let retrieved = cache.get("key1").await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().tokens, 10);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = EmbeddingCache::new(CacheConfig::default());
        let result = cache.get("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache = EmbeddingCache::new(CacheConfig::default());

        let result = EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "test".to_string(),
            tokens: 10,
            duration: Duration::from_millis(100),
        };

        cache.put("key1".to_string(), result).await;
        cache.get("key1").await;
        cache.get("key2").await;

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_requests, 2);
        assert!((stats.hit_rate() - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = EmbeddingCache::new(CacheConfig::default());

        let result = EmbeddingResult {
            embedding: vec![0.0; 1536],
            model: "test".to_string(),
            tokens: 10,
            duration: Duration::from_millis(100),
        };

        cache.put("key1".to_string(), result).await;
        assert_eq!(cache.size().await, 1);

        cache.clear().await;
        assert_eq!(cache.size().await, 0);
    }
}
