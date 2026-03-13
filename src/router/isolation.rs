// Agent Isolation - v0.5.3
//
// Agent 间动态隔离机制

use super::{RouterId, RouterLevel};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use anyhow::{Result, anyhow};

/// 隔离级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum IsolationLevel {
    /// 无隔离（共享所有资源）
    None,
    /// 上下文隔离（独立的上下文空间）
    #[default]
    Context,
    /// 工具隔离（独立的工具权限）
    Tools,
    /// 进程隔离（独立的进程空间）
    Process,
    /// 完全隔离（独立的资源和权限）
    Full,
}


/// 资源配额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// 最大 Token 数
    pub max_tokens: usize,
    /// 最大消息数
    pub max_messages: usize,
    /// 最大存储空间 (bytes)
    pub max_storage: usize,
    /// CPU 时间限制 (ms)
    pub cpu_time_ms: u64,
    /// 内存限制 (bytes)
    pub memory_bytes: usize,
}

impl Default for ResourceQuota {
    fn default() -> Self {
        Self {
            max_tokens: 100000,
            max_messages: 1000,
            max_storage: 100 * 1024 * 1024, // 100MB
            cpu_time_ms: 30000,             // 30s
            memory_bytes: 512 * 1024 * 1024, // 512MB
        }
    }
}

impl ResourceQuota {
    /// 创建宽松配额
    pub fn relaxed() -> Self {
        Self {
            max_tokens: 500000,
            max_messages: 5000,
            max_storage: 1024 * 1024 * 1024, // 1GB
            cpu_time_ms: 60000,
            memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
        }
    }
    
    /// 创建严格配额
    pub fn strict() -> Self {
        Self {
            max_tokens: 10000,
            max_messages: 100,
            max_storage: 10 * 1024 * 1024, // 10MB
            cpu_time_ms: 5000,
            memory_bytes: 128 * 1024 * 1024, // 128MB
        }
    }
}

/// 隔离配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct IsolationConfig {
    /// 隔离级别
    pub level: IsolationLevel,
    /// 资源配额
    pub quota: ResourceQuota,
    /// 允许的共享资源
    pub shared_resources: HashSet<String>,
    /// 允许的通信目标
    pub allowed_communications: HashSet<RouterId>,
}


/// 隔离边界
#[derive(Debug)]
pub struct IsolationBoundary {
    /// Agent ID
    pub agent_id: RouterId,
    /// 隔离配置
    pub config: IsolationConfig,
    /// 当前资源使用
    pub usage: ResourceUsage,
}

/// 资源使用情况
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// 已用 Token
    pub tokens_used: usize,
    /// 消息数
    pub messages_count: usize,
    /// 存储使用
    pub storage_used: usize,
}

impl IsolationBoundary {
    /// 创建新的隔离边界
    pub fn new(agent_id: RouterId, config: IsolationConfig) -> Self {
        Self {
            agent_id,
            config,
            usage: ResourceUsage::default(),
        }
    }
    
    /// 检查是否可以分配资源
    pub fn can_allocate(&self, tokens: usize) -> bool {
        self.usage.tokens_used + tokens <= self.config.quota.max_tokens
    }
    
    /// 分配资源
    pub fn allocate(&mut self, tokens: usize) -> Result<()> {
        if !self.can_allocate(tokens) {
            return Err(anyhow!("Resource quota exceeded"));
        }
        self.usage.tokens_used += tokens;
        Ok(())
    }
    
    /// 释放资源
    pub fn release(&mut self, tokens: usize) {
        self.usage.tokens_used = self.usage.tokens_used.saturating_sub(tokens);
    }
    
    /// 检查是否可以与目标通信
    pub fn can_communicate_with(&self, target: &RouterId) -> bool {
        // 完全隔离不允许任何通信
        if self.config.level == IsolationLevel::Full {
            return false;
        }
        
        // 检查白名单
        self.config.allowed_communications.contains(target)
    }
    
    /// 添加允许的通信目标
    pub fn allow_communication(&mut self, target: RouterId) {
        self.config.allowed_communications.insert(target);
    }
    
    /// 移除通信权限
    pub fn revoke_communication(&mut self, target: &RouterId) {
        self.config.allowed_communications.remove(target);
    }
    
    /// 获取资源使用率
    pub fn usage_ratio(&self) -> f32 {
        if self.config.quota.max_tokens == 0 {
            return 0.0;
        }
        self.usage.tokens_used as f32 / self.config.quota.max_tokens as f32
    }
    
    /// 检查资源是否紧张
    pub fn is_stressed(&self) -> bool {
        self.usage_ratio() > 0.8
    }
}

/// 隔离管理器
pub struct IsolationManager {
    /// 所有隔离边界
    boundaries: HashMap<RouterId, IsolationBoundary>,
    /// 默认配置
    default_config: IsolationConfig,
}

impl IsolationManager {
    /// 创建新的隔离管理器
    pub fn new() -> Self {
        Self {
            boundaries: HashMap::new(),
            default_config: IsolationConfig::default(),
        }
    }
    
    /// 创建 Agent 隔离
    pub fn create_isolation(&mut self, agent_id: RouterId) -> &mut IsolationBoundary {
        self.boundaries.entry(agent_id.clone())
            .or_insert_with(|| {
                IsolationBoundary::new(agent_id, self.default_config.clone())
            })
    }
    
    /// 创建带配置的隔离
    pub fn create_isolation_with_config(
        &mut self,
        agent_id: RouterId,
        config: IsolationConfig,
    ) -> &mut IsolationBoundary {
        let id = agent_id.clone();
        self.boundaries.insert(
            id.clone(),
            IsolationBoundary::new(id.clone(), config),
        );
        self.boundaries.get_mut(&id).unwrap()
    }
    
    /// 移除隔离
    pub fn remove_isolation(&mut self, agent_id: &RouterId) -> Result<()> {
        self.boundaries.remove(agent_id)
            .map(|_| ())
            .ok_or_else(|| anyhow!("Isolation not found: {}", agent_id))
    }
    
    /// 获取隔离边界
    pub fn get(&self, agent_id: &RouterId) -> Option<&IsolationBoundary> {
        self.boundaries.get(agent_id)
    }
    
    /// 获取可变隔离边界
    pub fn get_mut(&mut self, agent_id: &RouterId) -> Option<&mut IsolationBoundary> {
        self.boundaries.get_mut(agent_id)
    }
    
    /// 设置默认配置
    pub fn set_default_config(&mut self, config: IsolationConfig) {
        self.default_config = config;
    }
    
    /// 检查两个 Agent 是否可以通信
    pub fn can_communicate(&self, from: &RouterId, to: &RouterId) -> bool {
        match (self.get(from), self.get(to)) {
            (Some(from_boundary), Some(to_boundary)) => {
                from_boundary.can_communicate_with(to) &&
                to_boundary.can_communicate_with(from)
            }
            _ => true, // 没有隔离边界的默认允许
        }
    }
    
    /// 获取所有 Agent 的资源使用情况
    pub fn get_all_usage(&self) -> HashMap<RouterId, ResourceUsage> {
        self.boundaries
            .iter()
            .map(|(id, boundary)| (id.clone(), boundary.usage.clone()))
            .collect()
    }
    
    /// 获取资源紧张的 Agent
    pub fn get_stressed_agents(&self) -> Vec<&RouterId> {
        self.boundaries
            .iter()
            .filter(|(_, b)| b.is_stressed())
            .map(|(id, _)| id)
            .collect()
    }
    
    /// 动态调整隔离级别
    pub fn adjust_isolation_level(
        &mut self,
        agent_id: &RouterId,
        level: IsolationLevel,
    ) -> Result<()> {
        let boundary = self.boundaries.get_mut(agent_id)
            .ok_or_else(|| anyhow!("Isolation not found: {}", agent_id))?;
        
        boundary.config.level = level;
        Ok(())
    }
    
    /// 获取隔离数量
    pub fn len(&self) -> usize {
        self.boundaries.len()
    }
    
    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.boundaries.is_empty()
    }
}

impl Default for IsolationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_default() {
        let level = IsolationLevel::default();
        assert_eq!(level, IsolationLevel::Context);
    }

    #[test]
    fn test_resource_quota_default() {
        let quota = ResourceQuota::default();
        assert_eq!(quota.max_tokens, 100000);
    }

    #[test]
    fn test_isolation_boundary_can_allocate() {
        let boundary = IsolationBoundary::new(
            RouterId::new(),
            IsolationConfig::default(),
        );
        
        assert!(boundary.can_allocate(1000));
    }

    #[test]
    fn test_isolation_boundary_allocate() {
        let mut boundary = IsolationBoundary::new(
            RouterId::new(),
            IsolationConfig {
                quota: ResourceQuota { max_tokens: 100, ..Default::default() },
                ..Default::default()
            },
        );
        
        assert!(boundary.allocate(50).is_ok());
        assert!(boundary.allocate(60).is_err()); // 超过配额
    }

    #[test]
    fn test_isolation_manager_create() {
        let mut manager = IsolationManager::new();
        let agent_id = RouterId::new();
        
        manager.create_isolation(agent_id.clone());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_isolation_manager_can_communicate() {
        let mut manager = IsolationManager::new();
        let agent1 = RouterId::new();
        let agent2 = RouterId::new();
        
        // 创建隔离（默认级别允许通信）
        manager.create_isolation(agent1.clone());
        manager.create_isolation(agent2.clone());
        
        // 添加通信权限
        let boundary = manager.get_mut(&agent1).unwrap();
        boundary.allow_communication(agent2.clone());
        
        // 测试通信（需要双方都授权）
        // 由于 agent2 没有授权 agent1，所以不能通信
        assert!(!manager.can_communicate(&agent1, &agent2));
    }

    #[test]
    fn test_isolation_boundary_usage_ratio() {
        let mut boundary = IsolationBoundary::new(
            RouterId::new(),
            IsolationConfig {
                quota: ResourceQuota { max_tokens: 100, ..Default::default() },
                ..Default::default()
            },
        );
        
        boundary.allocate(50).unwrap();
        let ratio = boundary.usage_ratio();
        assert!((ratio - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_isolation_boundary_is_stressed() {
        let mut boundary = IsolationBoundary::new(
            RouterId::new(),
            IsolationConfig {
                quota: ResourceQuota { max_tokens: 100, ..Default::default() },
                ..Default::default()
            },
        );
        
        boundary.allocate(90).unwrap();
        assert!(boundary.is_stressed());
    }
}
