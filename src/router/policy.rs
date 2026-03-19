// Policy Engine - v0.5.1
//
// 权限策略引擎

use super::RouterId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 策略决策
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    /// 是否允许
    pub allowed: bool,
    /// 决策原因
    pub reason: String,
}

impl PolicyDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: "Allowed by policy".to_string(),
        }
    }
    
    pub fn deny(reason: &str) -> Self {
        Self {
            allowed: false,
            reason: reason.to_string(),
        }
    }
}

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Policy {
    /// 允许所有
    AllowAll,
    /// 拒绝所有
    DenyAll,
    /// 白名单
    Whitelist(Vec<String>),
    /// 黑名单
    Blacklist(Vec<String>),
    /// 速率限制（每分钟请求数）
    RateLimit(u32),
    /// 时间窗口（格式："09:00-18:00"）
    TimeWindow(String),
}

impl Policy {
    /// 评估动作
    pub fn evaluate(&self, action: &str) -> PolicyDecision {
        match self {
            Policy::AllowAll => PolicyDecision::allow(),
            Policy::DenyAll => PolicyDecision::deny("All actions denied"),
            Policy::Whitelist(actions) => {
                if actions.contains(&action.to_string()) {
                    PolicyDecision::allow()
                } else {
                    PolicyDecision::deny(&format!("Action '{}' not in whitelist", action))
                }
            }
            Policy::Blacklist(actions) => {
                if actions.contains(&action.to_string()) {
                    PolicyDecision::deny(&format!("Action '{}' is blacklisted", action))
                } else {
                    PolicyDecision::allow()
                }
            }
            Policy::RateLimit(_) => {
                // 速率限制需要状态，这里简化处理
                PolicyDecision::allow()
            }
            Policy::TimeWindow(window) => {
                if Self::is_in_time_window(window) {
                    PolicyDecision::allow()
                } else {
                    PolicyDecision::deny(&format!("Outside time window: {}", window))
                }
            }
        }
    }
    
    /// 检查是否在时间窗口内
    fn is_in_time_window(window: &str) -> bool {
        // 解析时间窗口格式 "HH:MM-HH:MM"
        let parts: Vec<&str> = window.split('-').collect();
        if parts.len() != 2 {
            return true; // 格式错误时默认允许
        }
        
        let now = chrono::Local::now();
        let current_time = now.format("%H:%M").to_string();
        
        // 简单字符串比较
        current_time.as_str() >= parts[0] && current_time.as_str() <= parts[1]
    }
}

/// 策略引擎
pub struct PolicyEngine {
    /// 路由策略映射
    policies: HashMap<RouterId, Vec<Policy>>,
    /// 默认策略
    default_policy: Policy,
}

impl PolicyEngine {
    /// 创建新的策略引擎
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            default_policy: Policy::AllowAll,
        }
    }
    
    /// 设置路由策略
    pub fn set_policy(&mut self, router_id: RouterId, policy: Policy) {
        self.policies.insert(router_id, vec![policy]);
    }
    
    /// 添加路由策略
    pub fn add_policy(&mut self, router_id: RouterId, policy: Policy) {
        self.policies.entry(router_id).or_default().push(policy);
    }
    
    /// 设置默认策略
    pub fn set_default_policy(&mut self, policy: Policy) {
        self.default_policy = policy;
    }
    
    /// 评估动作
    pub fn evaluate(&self, router_id: &RouterId, action: &str) -> PolicyDecision {
        if let Some(policies) = self.policies.get(router_id) {
            for policy in policies {
                let decision = policy.evaluate(action);
                if !decision.allowed {
                    return decision;
                }
            }
            PolicyDecision::allow()
        } else {
            self.default_policy.evaluate(action)
        }
    }
    
    /// 组合策略
    pub fn combine_policies(&self, policies: &[Policy]) -> Policy {
        // 如果有任何 DenyAll，返回 DenyAll
        if policies.iter().any(|p| matches!(p, Policy::DenyAll)) {
            return Policy::DenyAll;
        }
        
        // 合并所有白名单
        let mut whitelist = Vec::new();
        for policy in policies {
            if let Policy::Whitelist(actions) = policy {
                whitelist.extend(actions.clone());
            }
        }
        
        if !whitelist.is_empty() {
            Policy::Whitelist(whitelist)
        } else {
            Policy::AllowAll
        }
    }
    
    /// 移除路由策略
    pub fn remove_policy(&mut self, router_id: &RouterId) {
        self.policies.remove(router_id);
    }
    
    /// 获取路由策略
    pub fn get_policies(&self, router_id: &RouterId) -> Option<&Vec<Policy>> {
        self.policies.get(router_id)
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_allow_all() {
        let policy = Policy::AllowAll;
        let decision = policy.evaluate("any_action");
        assert!(decision.allowed);
    }

    #[test]
    fn test_policy_deny_all() {
        let policy = Policy::DenyAll;
        let decision = policy.evaluate("any_action");
        assert!(!decision.allowed);
    }

    #[test]
    fn test_policy_whitelist() {
        let policy = Policy::Whitelist(vec!["read".to_string(), "write".to_string()]);
        
        assert!(policy.evaluate("read").allowed);
        assert!(policy.evaluate("write").allowed);
        assert!(!policy.evaluate("delete").allowed);
    }

    #[test]
    fn test_policy_blacklist() {
        let policy = Policy::Blacklist(vec!["delete".to_string()]);
        
        assert!(policy.evaluate("read").allowed);
        assert!(policy.evaluate("write").allowed);
        assert!(!policy.evaluate("delete").allowed);
    }

    #[test]
    fn test_policy_engine() {
        let mut engine = PolicyEngine::new();
        let router_id = RouterId::new();
        
        engine.set_policy(router_id.clone(), Policy::Whitelist(vec!["read".to_string()]));
        
        assert!(engine.evaluate(&router_id, "read").allowed);
        assert!(!engine.evaluate(&router_id, "write").allowed);
    }

    #[test]
    fn test_policy_engine_default() {
        let engine = PolicyEngine::new();
        let router_id = RouterId::new();
        
        // 默认策略是 AllowAll
        assert!(engine.evaluate(&router_id, "any_action").allowed);
    }
}
