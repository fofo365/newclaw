// Router - v0.5.1
//
// 路由实体定义

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 路由 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouterId(String);

impl RouterId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn parse(s: &str) -> Self {
        Self(s.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RouterId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RouterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 路由层级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouterLevel {
    /// 顶级路由（独立）
    Top,
    /// 上级路由（有下级）
    Upper,
    /// 下级路由（有上级）
    Lower,
    /// 特殊路由（Channel/Skill）
    Special,
}

/// 路由能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterCapabilities {
    /// 是否可以管理下级
    pub can_manage_children: bool,
    /// 是否可以请求上级
    pub can_request_parent: bool,
    /// 是否可以共享给同级
    pub can_share_with_peers: bool,
    /// 是否可以派生下级
    pub can_spawn_children: bool,
}

impl Default for RouterCapabilities {
    fn default() -> Self {
        Self {
            can_manage_children: true,
            can_request_parent: true,
            can_share_with_peers: true,
            can_spawn_children: true,
        }
    }
}

impl RouterCapabilities {
    /// 顶级路由能力
    pub fn top_level() -> Self {
        Self {
            can_manage_children: true,
            can_request_parent: false,
            can_share_with_peers: true,
            can_spawn_children: true,
        }
    }
    
    /// 上级路由能力
    pub fn upper_level() -> Self {
        Self {
            can_manage_children: true,
            can_request_parent: true,
            can_share_with_peers: true,
            can_spawn_children: true,
        }
    }
    
    /// 下级路由能力
    pub fn lower_level() -> Self {
        Self {
            can_manage_children: false,
            can_request_parent: true,
            can_share_with_peers: true,
            can_spawn_children: false,
        }
    }
    
    /// 特殊路由能力（Channel/Skill）
    pub fn special() -> Self {
        Self {
            can_manage_children: false,
            can_request_parent: true,
            can_share_with_peers: false,
            can_spawn_children: false,
        }
    }
}

/// 路由实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Router {
    /// 路由 ID
    pub id: RouterId,
    /// 路由名称
    pub name: String,
    /// 路由层级
    pub level: RouterLevel,
    /// 父路由 ID
    pub parent: Option<RouterId>,
    /// 子路由 ID 列表
    pub children: Vec<RouterId>,
    /// 路由能力
    pub capabilities: RouterCapabilities,
    /// 路由元数据
    pub metadata: serde_json::Value,
}

impl Router {
    /// 创建新的顶级路由
    pub fn new_top(name: &str) -> Self {
        Self {
            id: RouterId::new(),
            name: name.to_string(),
            level: RouterLevel::Top,
            parent: None,
            children: Vec::new(),
            capabilities: RouterCapabilities::top_level(),
            metadata: serde_json::json!({}),
        }
    }
    
    /// 创建子路由
    pub fn new_child(name: &str, parent: RouterId, level: RouterLevel) -> Self {
        let capabilities = match level {
            RouterLevel::Upper => RouterCapabilities::upper_level(),
            RouterLevel::Lower => RouterCapabilities::lower_level(),
            RouterLevel::Special => RouterCapabilities::special(),
            RouterLevel::Top => RouterCapabilities::top_level(),
        };
        
        Self {
            id: RouterId::new(),
            name: name.to_string(),
            level,
            parent: Some(parent),
            children: Vec::new(),
            capabilities,
            metadata: serde_json::json!({}),
        }
    }
    
    /// 添加子路由
    pub fn add_child(&mut self, child_id: RouterId) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }
    
    /// 移除子路由
    pub fn remove_child(&mut self, child_id: &RouterId) {
        self.children.retain(|id| id != child_id);
    }
    
    /// 检查是否为顶级路由
    pub fn is_top_level(&self) -> bool {
        self.level == RouterLevel::Top
    }
    
    /// 检查是否有子路由
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
    
    /// 检查是否有父路由
    pub fn has_parent(&self) -> bool {
        self.parent.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_id() {
        let id1 = RouterId::new();
        let id2 = RouterId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_router_new_top() {
        let router = Router::new_top("main");
        assert_eq!(router.name, "main");
        assert_eq!(router.level, RouterLevel::Top);
        assert!(router.parent.is_none());
        assert!(router.children.is_empty());
    }

    #[test]
    fn test_router_add_child() {
        let mut parent = Router::new_top("parent");
        let child = Router::new_child("child", parent.id.clone(), RouterLevel::Lower);
        
        parent.add_child(child.id.clone());
        assert_eq!(parent.children.len(), 1);
        assert!(parent.children.contains(&child.id));
    }

    #[test]
    fn test_router_capabilities() {
        let top_caps = RouterCapabilities::top_level();
        assert!(top_caps.can_manage_children);
        assert!(!top_caps.can_request_parent);
        
        let lower_caps = RouterCapabilities::lower_level();
        assert!(!lower_caps.can_manage_children);
        assert!(lower_caps.can_request_parent);
    }
}
