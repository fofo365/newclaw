// Router Manager - v0.5.1
//
// 路由生命周期管理

use super::{Router, RouterId, RouterLevel, RouterConfig};
use crate::router::policy::PolicyEngine;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 路由管理器
pub struct RouterManager {
    /// 所有路由
    routers: HashMap<RouterId, Router>,
    /// 策略引擎
    policy_engine: PolicyEngine,
    /// 审计日志
    audit_log: Vec<AuditEntry>,
}

/// 审计条目
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: i64,
    pub from: Option<RouterId>,
    pub to: Option<RouterId>,
    pub action: String,
    pub result: String,
}

impl RouterManager {
    /// 创建新的路由管理器
    pub fn new() -> Self {
        Self {
            routers: HashMap::new(),
            policy_engine: PolicyEngine::new(),
            audit_log: Vec::new(),
        }
    }
    
    /// 创建路由
    pub fn spawn_router(&mut self, config: RouterConfig) -> Result<RouterId> {
        let router = Router {
            id: config.id.clone(),
            name: config.name,
            level: config.level,
            parent: config.parent.clone(),
            children: config.children.clone(),
            capabilities: config.capabilities,
            metadata: serde_json::json!({}),
        };
        
        let id = router.id.clone();
        
        // 如果有父路由，添加到父路由的子列表
        if let Some(parent_id) = &router.parent {
            if let Some(parent) = self.routers.get_mut(parent_id) {
                parent.add_child(id.clone());
            }
        }
        
        self.routers.insert(id.clone(), router);
        
        self.log_audit(None, Some(id.clone()), "spawn", "success");
        
        Ok(id)
    }
    
    /// 关闭路由
    pub fn shutdown_router(&mut self, id: &RouterId) -> Result<()> {
        let router = self.routers.remove(id)
            .ok_or_else(|| anyhow!("Router not found: {}", id))?;
        
        // 从父路由中移除
        if let Some(parent_id) = &router.parent {
            if let Some(parent) = self.routers.get_mut(parent_id) {
                parent.remove_child(id);
            }
        }
        
        // 移除所有子路由的父引用
        for child_id in &router.children {
            if let Some(child) = self.routers.get_mut(child_id) {
                child.parent = None;
            }
        }
        
        self.log_audit(None, Some(id.clone()), "shutdown", "success");
        
        Ok(())
    }
    
    /// 添加子路由关系
    pub fn add_child(&mut self, parent_id: &RouterId, child_id: &RouterId) -> Result<()> {
        let parent = self.routers.get_mut(parent_id)
            .ok_or_else(|| anyhow!("Parent router not found: {}", parent_id))?;
        
        if !parent.capabilities.can_manage_children {
            return Err(anyhow!("Parent router cannot manage children"));
        }
        
        parent.add_child(child_id.clone());
        
        let child = self.routers.get_mut(child_id)
            .ok_or_else(|| anyhow!("Child router not found: {}", child_id))?;
        
        child.parent = Some(parent_id.clone());
        
        self.log_audit(Some(parent_id.clone()), Some(child_id.clone()), "add_child", "success");
        
        Ok(())
    }
    
    /// 移除子路由关系
    pub fn remove_child(&mut self, parent_id: &RouterId, child_id: &RouterId) -> Result<()> {
        let parent = self.routers.get_mut(parent_id)
            .ok_or_else(|| anyhow!("Parent router not found: {}", parent_id))?;
        
        parent.remove_child(child_id);
        
        let child = self.routers.get_mut(child_id)
            .ok_or_else(|| anyhow!("Child router not found: {}", child_id))?;
        
        child.parent = None;
        
        self.log_audit(Some(parent_id.clone()), Some(child_id.clone()), "remove_child", "success");
        
        Ok(())
    }
    
    /// 查找路由
    pub fn find_router(&self, name: &str) -> Option<&Router> {
        self.routers.values().find(|r| r.name == name)
    }
    
    /// 获取路由
    pub fn get_router(&self, id: &RouterId) -> Option<&Router> {
        self.routers.get(id)
    }
    
    /// 获取路由（可变）
    pub fn get_router_mut(&mut self, id: &RouterId) -> Option<&mut Router> {
        self.routers.get_mut(id)
    }
    
    /// 获取所有顶级路由
    pub fn get_top_level_routers(&self) -> Vec<&Router> {
        self.routers.values()
            .filter(|r| r.level == RouterLevel::Top)
            .collect()
    }
    
    /// 获取子路由
    pub fn get_children(&self, parent_id: &RouterId) -> Vec<&Router> {
        self.routers.get(parent_id)
            .map(|parent| {
                parent.children.iter()
                    .filter_map(|id| self.routers.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// 检查权限
    pub fn check_permission(&self, from: &RouterId, to: &RouterId, action: &str) -> bool {
        self.policy_engine.evaluate(from, action).allowed
    }
    
    /// 获取路由数量
    pub fn len(&self) -> usize {
        self.routers.len()
    }
    
    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.routers.is_empty()
    }
    
    /// 记录审计日志
    fn log_audit(&mut self, from: Option<RouterId>, to: Option<RouterId>, action: &str, result: &str) {
        self.audit_log.push(AuditEntry {
            timestamp: chrono::Utc::now().timestamp(),
            from,
            to,
            action: action.to_string(),
            result: result.to_string(),
        });
    }
    
    /// 获取审计日志
    pub fn get_audit_log(&self) -> &[AuditEntry] {
        &self.audit_log
    }
}

impl Default for RouterManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_manager_spawn() {
        let mut manager = RouterManager::new();
        let config = RouterConfig::default();
        let id = manager.spawn_router(config).unwrap();
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_router_manager_shutdown() {
        let mut manager = RouterManager::new();
        let config = RouterConfig::default();
        let id = manager.spawn_router(config).unwrap();
        
        manager.shutdown_router(&id).unwrap();
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_router_manager_add_child() {
        let mut manager = RouterManager::new();
        
        let parent_config = RouterConfig {
            name: "parent".to_string(),
            ..Default::default()
        };
        let parent_id = manager.spawn_router(parent_config).unwrap();
        
        let child_config = RouterConfig {
            name: "child".to_string(),
            level: RouterLevel::Lower,
            parent: Some(parent_id.clone()),
            ..Default::default()
        };
        let child_id = manager.spawn_router(child_config).unwrap();
        
        let parent = manager.get_router(&parent_id).unwrap();
        assert!(parent.children.contains(&child_id));
    }

    #[test]
    fn test_router_manager_find() {
        let mut manager = RouterManager::new();
        let config = RouterConfig {
            name: "test-router".to_string(),
            ..Default::default()
        };
        manager.spawn_router(config).unwrap();
        
        let router = manager.find_router("test-router");
        assert!(router.is_some());
    }
}
