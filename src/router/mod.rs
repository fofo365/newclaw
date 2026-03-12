// Router Module - v0.5.1
//
// 多层路由架构：
// - Router: 路由实体（顶级/上级/下级/特殊）
// - RouterManager: 路由生命周期管理
// - RouterConnector: 路由间通信
// - PolicyEngine: 权限策略引擎

pub mod router;
pub mod manager;
pub mod connector;
pub mod policy;

pub use router::{Router, RouterId, RouterLevel, RouterCapabilities};
pub use manager::RouterManager;
pub use connector::{RouterConnector, RouterMessage, Action};
pub use policy::{PolicyEngine, Policy, PolicyDecision};

use serde::{Deserialize, Serialize};

/// 路由配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
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
    /// 能力配置
    pub capabilities: RouterCapabilities,
    /// 策略配置
    pub policy: Option<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            id: RouterId::new(),
            name: "default".to_string(),
            level: RouterLevel::Top,
            parent: None,
            children: Vec::new(),
            capabilities: RouterCapabilities::default(),
            policy: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_config_default() {
        let config = RouterConfig::default();
        assert_eq!(config.level, RouterLevel::Top);
        assert!(config.parent.is_none());
        assert!(config.children.is_empty());
    }
}
