// 租约管理模块

use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::config::LeaseConfig;

/// 租约
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lease {
    /// 租约 ID
    pub id: String,
    /// 持有者
    pub holder: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 最后续约时间
    pub last_renewed: DateTime<Utc>,
}

impl Lease {
    /// 创建新租约
    pub fn new(holder: String, duration: Duration) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            holder,
            created_at: now,
            expires_at: now + chrono::Duration::from_std(duration).unwrap(),
            last_renewed: now,
        }
    }
    
    /// 检查是否有效
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
    
    /// 续约
    pub fn renew(&mut self, duration: Duration) {
        let now = Utc::now();
        self.expires_at = now + chrono::Duration::from_std(duration).unwrap();
        self.last_renewed = now;
    }
    
    /// 剩余时间
    pub fn remaining(&self) -> Duration {
        let now = Utc::now();
        if now >= self.expires_at {
            Duration::ZERO
        } else {
            (self.expires_at - now).to_std().unwrap_or(Duration::ZERO)
        }
    }
}

/// 租约存储 trait
pub trait LeaseStorage: Send + Sync {
    /// 保存租约
    fn save(&self, lease: &Lease) -> anyhow::Result<()>;
    
    /// 获取租约
    fn get(&self, id: &str) -> anyhow::Result<Option<Lease>>;
    
    /// 删除租约
    fn delete(&self, id: &str) -> anyhow::Result<()>;
    
    /// 获取所有租约
    fn list(&self) -> anyhow::Result<Vec<Lease>>;
}

/// 内存租约存储
pub struct MemoryLeaseStorage {
    leases: RwLock<HashMap<String, Lease>>,
}

impl MemoryLeaseStorage {
    pub fn new() -> Self {
        Self {
            leases: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryLeaseStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl LeaseStorage for MemoryLeaseStorage {
    fn save(&self, lease: &Lease) -> anyhow::Result<()> {
        let mut leases = self.leases.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        leases.insert(lease.id.clone(), lease.clone());
        Ok(())
    }
    
    fn get(&self, id: &str) -> anyhow::Result<Option<Lease>> {
        let leases = self.leases.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(leases.get(id).cloned())
    }
    
    fn delete(&self, id: &str) -> anyhow::Result<()> {
        let mut leases = self.leases.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        leases.remove(id);
        Ok(())
    }
    
    fn list(&self) -> anyhow::Result<Vec<Lease>> {
        let leases = self.leases.read().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(leases.values().cloned().collect())
    }
}

/// 租约管理器
pub struct LeaseManager {
    storage: Box<dyn LeaseStorage>,
    config: LeaseConfig,
    current_lease: Arc<RwLock<Option<Lease>>>,
}

impl LeaseManager {
    pub fn new(config: LeaseConfig) -> Self {
        Self {
            storage: Box::new(MemoryLeaseStorage::new()),
            config,
            current_lease: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn with_storage(config: LeaseConfig, storage: Box<dyn LeaseStorage>) -> Self {
        Self {
            storage,
            config,
            current_lease: Arc::new(RwLock::new(None)),
        }
    }
    
    /// 申请租约
    pub fn acquire(&self, holder: String) -> anyhow::Result<Lease> {
        let lease = Lease::new(holder, self.config.duration());
        self.storage.save(&lease)?;
        
        let mut current = self.current_lease.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        *current = Some(lease.clone());
        
        Ok(lease)
    }
    
    /// 续约
    pub fn renew(&self, lease_id: &str) -> anyhow::Result<Lease> {
        let mut lease = self.storage.get(lease_id)?
            .ok_or_else(|| anyhow::anyhow!("Lease not found: {}", lease_id))?;
        
        lease.renew(self.config.duration());
        self.storage.save(&lease)?;
        
        let mut current = self.current_lease.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        *current = Some(lease.clone());
        
        Ok(lease)
    }
    
    /// 释放租约
    pub fn release(&self, lease_id: &str) -> anyhow::Result<()> {
        self.storage.delete(lease_id)?;
        
        let mut current = self.current_lease.write().map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        *current = None;
        
        Ok(())
    }
    
    /// 检查当前租约有效性
    pub fn is_valid(&self) -> bool {
        let current = self.current_lease.read().unwrap();
        match current.as_ref() {
            Some(lease) => lease.is_valid(),
            None => false,
        }
    }
    
    /// 获取当前租约
    pub fn current(&self) -> Option<Lease> {
        self.current_lease.read().unwrap().clone()
    }
    
    /// 获取租约
    pub fn get(&self, id: &str) -> anyhow::Result<Option<Lease>> {
        self.storage.get(id)
    }
    
    /// 清理过期租约
    pub fn cleanup_expired(&self) -> anyhow::Result<usize> {
        let leases = self.storage.list()?;
        let mut cleaned = 0;
        
        for lease in leases {
            if !lease.is_valid() {
                self.storage.delete(&lease.id)?;
                cleaned += 1;
            }
        }
        
        Ok(cleaned)
    }
}

/// Redis 租约存储
/// 
/// 生产环境推荐使用 Redis 存储，支持跨进程租约共享
#[cfg(feature = "redis-support")]
pub struct RedisLeaseStorage {
    client: redis::Client,
    key_prefix: String,
}

#[cfg(feature = "redis-support")]
impl RedisLeaseStorage {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| anyhow::anyhow!("Failed to connect to Redis: {}", e))?;
        
        Ok(Self {
            client,
            key_prefix: "newclaw:lease:".to_string(),
        })
    }
    
    pub fn with_prefix(redis_url: &str, key_prefix: String) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| anyhow::anyhow!("Failed to connect to Redis: {}", e))?;
        
        Ok(Self {
            client,
            key_prefix,
        })
    }
    
    fn lease_key(&self, id: &str) -> String {
        format!("{}{}", self.key_prefix, id)
    }
    
    fn list_key(&self) -> String {
        format!("{}__all__", self.key_prefix)
    }
}

#[cfg(feature = "redis-support")]
impl LeaseStorage for RedisLeaseStorage {
    fn save(&self, lease: &Lease) -> anyhow::Result<()> {
        let mut conn = self.client.get_connection()
            .map_err(|e| anyhow::anyhow!("Redis connection error: {}", e))?;
        
        let key = self.lease_key(&lease.id);
        let value = serde_json::to_string(lease)?;
        
        // 使用 SETEX 设置带过期时间的键
        let ttl = lease.remaining().as_secs() as i64 + 60; // 额外 60s 缓冲
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .arg("EX")
            .arg(ttl)
            .query(&mut conn)
            .map_err(|e| anyhow::anyhow!("Redis SET error: {}", e))?;
        
        // 添加到列表
        let _: () = redis::cmd("SADD")
            .arg(self.list_key())
            .arg(&lease.id)
            .query(&mut conn)
            .map_err(|e| anyhow::anyhow!("Redis SADD error: {}", e))?;
        
        Ok(())
    }
    
    fn get(&self, id: &str) -> anyhow::Result<Option<Lease>> {
        let mut conn = self.client.get_connection()
            .map_err(|e| anyhow::anyhow!("Redis connection error: {}", e))?;
        
        let key = self.lease_key(id);
        
        let value: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query(&mut conn)
            .map_err(|e| anyhow::anyhow!("Redis GET error: {}", e))?;
        
        match value {
            Some(json) => {
                let lease: Lease = serde_json::from_str(&json)?;
                Ok(Some(lease))
            }
            None => Ok(None),
        }
    }
    
    fn delete(&self, id: &str) -> anyhow::Result<()> {
        let mut conn = self.client.get_connection()
            .map_err(|e| anyhow::anyhow!("Redis connection error: {}", e))?;
        
        let key = self.lease_key(id);
        
        let _: () = redis::cmd("DEL")
            .arg(&key)
            .query(&mut conn)
            .map_err(|e| anyhow::anyhow!("Redis DEL error: {}", e))?;
        
        // 从列表中移除
        let _: () = redis::cmd("SREM")
            .arg(self.list_key())
            .arg(id)
            .query(&mut conn)
            .map_err(|e| anyhow::anyhow!("Redis SREM error: {}", e))?;
        
        Ok(())
    }
    
    fn list(&self) -> anyhow::Result<Vec<Lease>> {
        let mut conn = self.client.get_connection()
            .map_err(|e| anyhow::anyhow!("Redis connection error: {}", e))?;
        
        // 获取所有租约 ID
        let ids: Vec<String> = redis::cmd("SMEMBERS")
            .arg(self.list_key())
            .query(&mut conn)
            .map_err(|e| anyhow::anyhow!("Redis SMEMBERS error: {}", e))?;
        
        let mut leases = Vec::new();
        for id in ids {
            if let Some(lease) = self.get(&id)? {
                // 只返回有效的租约
                if lease.is_valid() {
                    leases.push(lease);
                } else {
                    // 清理过期租约
                    self.delete(&id)?;
                }
            }
        }
        
        Ok(leases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lease_creation() {
        let lease = Lease::new("smart_controller".to_string(), Duration::from_secs(15));
        assert!(lease.is_valid());
        assert_eq!(lease.holder, "smart_controller");
    }
    
    #[test]
    fn test_lease_expiry() {
        let mut lease = Lease::new("test".to_string(), Duration::from_secs(1));
        assert!(lease.is_valid());
        
        // 手动设置过期
        lease.expires_at = Utc::now() - chrono::Duration::seconds(1);
        assert!(!lease.is_valid());
    }
    
    #[test]
    fn test_lease_renewal() {
        let mut lease = Lease::new("test".to_string(), Duration::from_secs(10));
        let original_expires = lease.expires_at;
        
        std::thread::sleep(Duration::from_millis(100));
        lease.renew(Duration::from_secs(10));
        
        assert!(lease.expires_at > original_expires);
        assert!(lease.is_valid());
    }
    
    #[test]
    fn test_lease_manager_acquire() {
        let config = LeaseConfig::default();
        let manager = LeaseManager::new(config);
        
        let lease = manager.acquire("smart_controller".to_string()).unwrap();
        assert!(manager.is_valid());
        assert_eq!(lease.holder, "smart_controller");
    }
    
    #[test]
    fn test_lease_manager_renew() {
        let config = LeaseConfig::default();
        let manager = LeaseManager::new(config);
        
        let lease = manager.acquire("test".to_string()).unwrap();
        let renewed = manager.renew(&lease.id).unwrap();
        
        assert!(renewed.last_renewed >= lease.last_renewed);
    }
    
    #[test]
    fn test_lease_manager_release() {
        let config = LeaseConfig::default();
        let manager = LeaseManager::new(config);
        
        let lease = manager.acquire("test".to_string()).unwrap();
        manager.release(&lease.id).unwrap();
        
        assert!(!manager.is_valid());
    }
    
    #[test]
    fn test_memory_storage() {
        let storage = MemoryLeaseStorage::new();
        let lease = Lease::new("test".to_string(), Duration::from_secs(15));
        
        storage.save(&lease).unwrap();
        let retrieved = storage.get(&lease.id).unwrap();
        assert!(retrieved.is_some());
        
        storage.delete(&lease.id).unwrap();
        let retrieved = storage.get(&lease.id).unwrap();
        assert!(retrieved.is_none());
    }
}
