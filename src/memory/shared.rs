// 多 Agent 记忆共享模块 (v0.5.5)
//
// 基于用户名的记忆存储和共享机制

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 用户 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(String);

impl UserId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 条目 ID
    pub id: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后访问时间
    pub last_accessed: DateTime<Utc>,
    /// 记忆类型
    pub memory_type: MemoryType,
    /// 重要性 (0-1)
    pub importance: f32,
    /// 内容
    pub content: String,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 来源 Agent
    pub source_agent: Option<String>,
    /// 标签
    pub tags: Vec<String>,
}

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    /// 对话记忆
    Conversation,
    /// 任务记忆
    Task,
    /// 偏好记忆
    Preference,
    /// 事实记忆
    Fact,
    /// 技能记忆
    Skill,
    /// 上下文记忆
    Context,
}

/// 用户记忆存储
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMemory {
    /// 用户 ID
    pub user_id: UserId,
    /// 记忆条目
    pub entries: HashMap<String, MemoryEntry>,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
    /// 记忆统计
    pub stats: MemoryStats,
}

/// 记忆统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_conversations: usize,
    pub total_tasks: usize,
    pub total_preferences: usize,
}

impl UserMemory {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            entries: HashMap::new(),
            last_updated: Utc::now(),
            stats: MemoryStats::default(),
        }
    }
    
    /// 添加记忆
    pub fn add(&mut self, entry: MemoryEntry) {
        self.stats.total_entries += 1;
        match entry.memory_type {
            MemoryType::Conversation => self.stats.total_conversations += 1,
            MemoryType::Task => self.stats.total_tasks += 1,
            MemoryType::Preference => self.stats.total_preferences += 1,
            _ => {}
        }
        self.entries.insert(entry.id.clone(), entry);
        self.last_updated = Utc::now();
    }
    
    /// 获取记忆
    pub fn get(&self, id: &str) -> Option<&MemoryEntry> {
        self.entries.get(id)
    }
    
    /// 搜索记忆
    pub fn search(&self, query: &str, limit: usize) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<&MemoryEntry> = self.entries.values()
            .filter(|e| {
                e.content.to_lowercase().contains(&query_lower) ||
                e.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect();
        
        // 按重要性和时间排序
        results.sort_by(|a, b| {
            b.importance.partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        
        results.into_iter().take(limit).collect()
    }
    
    /// 按类型获取记忆
    pub fn get_by_type(&self, memory_type: &MemoryType) -> Vec<&MemoryEntry> {
        self.entries.values()
            .filter(|e| std::mem::discriminant(&e.memory_type) == std::mem::discriminant(memory_type))
            .collect()
    }
    
    /// 清理过期记忆
    pub fn cleanup(&mut self, max_age_days: u64) {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        self.entries.retain(|_, e| e.last_accessed > cutoff);
        self.stats.total_entries = self.entries.len();
    }
}

/// 共享记忆管理器
pub struct SharedMemoryManager {
    /// 用户记忆存储
    users: RwLock<HashMap<UserId, UserMemory>>,
    /// 缓存
    cache: RwLock<HashMap<String, CachedMemory>>,
    /// 配置
    config: SharedMemoryConfig,
}

/// 共享记忆配置
#[derive(Debug, Clone)]
pub struct SharedMemoryConfig {
    /// 最大记忆条目数
    pub max_entries_per_user: usize,
    /// 记忆过期天数
    pub max_age_days: u64,
    /// 缓存 TTL（秒）
    pub cache_ttl_secs: u64,
}

impl Default for SharedMemoryConfig {
    fn default() -> Self {
        Self {
            max_entries_per_user: 1000,
            max_age_days: 30,
            cache_ttl_secs: 300,
        }
    }
}

/// 缓存的记忆
#[derive(Debug, Clone)]
struct CachedMemory {
    entry: MemoryEntry,
    cached_at: Instant,
}

impl SharedMemoryManager {
    pub fn new(config: SharedMemoryConfig) -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            config,
        }
    }
    
    /// 获取用户记忆
    pub fn get_user_memory(&self, user_id: &UserId) -> UserMemory {
        let users = self.users.read().unwrap();
        users.get(user_id).cloned().unwrap_or_else(|| UserMemory::new(user_id.clone()))
    }
    
    /// 保存用户记忆
    pub fn save_user_memory(&self, memory: UserMemory) {
        let mut users = self.users.write().unwrap();
        users.insert(memory.user_id.clone(), memory);
    }
    
    /// 添加记忆
    pub fn add_memory(&self, user_id: &UserId, entry: MemoryEntry) {
        let mut users = self.users.write().unwrap();
        let memory = users.entry(user_id.clone()).or_insert_with(|| UserMemory::new(user_id.clone()));
        memory.add(entry);
    }
    
    /// 搜索记忆
    pub fn search(&self, user_id: &UserId, query: &str, limit: usize) -> Vec<MemoryEntry> {
        let users = self.users.read().unwrap();
        if let Some(memory) = users.get(user_id) {
            memory.search(query, limit).into_iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    /// 转移任务到另一个 Agent
    pub fn transfer_task(&self, from_user: &UserId, to_user: &UserId, task_id: &str) -> anyhow::Result<()> {
        let mut users = self.users.write().unwrap();
        
        let task = {
            let from_memory = users.get_mut(from_user)
                .ok_or_else(|| anyhow::anyhow!("Source user not found"))?;
            from_memory.entries.remove(task_id)
                .ok_or_else(|| anyhow::anyhow!("Task not found"))?
        };
        
        let to_memory = users.entry(to_user.clone())
            .or_insert_with(|| UserMemory::new(to_user.clone()));
        to_memory.add(task);
        
        Ok(())
    }
    
    /// 清理过期记忆
    pub fn cleanup(&self) {
        let mut users = self.users.write().unwrap();
        for memory in users.values_mut() {
            memory.cleanup(self.config.max_age_days);
        }
    }
    
    /// 获取统计信息
    pub fn get_stats(&self) -> GlobalMemoryStats {
        let users = self.users.read().unwrap();
        let total_users = users.len();
        let total_entries: usize = users.values().map(|m| m.entries.len()).sum();
        
        GlobalMemoryStats {
            total_users,
            total_entries,
        }
    }
}

/// 全局记忆统计
#[derive(Debug, Clone, Serialize)]
pub struct GlobalMemoryStats {
    pub total_users: usize,
    pub total_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id() {
        let id = UserId::new("user123");
        assert_eq!(id.as_str(), "user123");
    }

    #[test]
    fn test_user_memory() {
        let mut memory = UserMemory::new(UserId::new("user1"));
        
        let entry = MemoryEntry {
            id: "mem1".to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Fact,
            importance: 0.8,
            content: "Test memory".to_string(),
            metadata: HashMap::new(),
            source_agent: None,
            tags: vec!["test".to_string()],
        };
        
        memory.add(entry);
        assert_eq!(memory.entries.len(), 1);
    }

    #[test]
    fn test_shared_memory_manager() {
        let manager = SharedMemoryManager::new(SharedMemoryConfig::default());
        
        let entry = MemoryEntry {
            id: "mem1".to_string(),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            memory_type: MemoryType::Fact,
            importance: 0.8,
            content: "Test content".to_string(),
            metadata: HashMap::new(),
            source_agent: None,
            tags: vec![],
        };
        
        manager.add_memory(&UserId::new("user1"), entry);
        
        let results = manager.search(&UserId::new("user1"), "Test", 10);
        assert_eq!(results.len(), 1);
    }
}
