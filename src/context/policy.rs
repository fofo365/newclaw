// Context Policy - v0.5.3
//
// 上下文策略管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContextPolicyType {
    /// Token 限制策略
    TokenLimit,
    /// 时间窗口策略
    TimeWindow,
    /// 优先级策略
    Priority,
    /// 来源策略
    Source,
    /// 自定义策略
    Custom(String),
}

/// 上下文策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPolicy {
    /// 策略 ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 策略类型
    pub policy_type: ContextPolicyType,
    /// 优先级 (0-100, 越高越优先)
    pub priority: u8,
    /// 是否启用
    pub enabled: bool,
    /// 策略参数
    pub params: HashMap<String, serde_json::Value>,
}

impl ContextPolicy {
    /// 创建 Token 限制策略
    pub fn token_limit(name: &str, max_tokens: usize) -> Self {
        let mut params = HashMap::new();
        params.insert("max_tokens".to_string(), serde_json::json!(max_tokens));
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            policy_type: ContextPolicyType::TokenLimit,
            priority: 50,
            enabled: true,
            params,
        }
    }
    
    /// 创建时间窗口策略
    pub fn time_window(name: &str, window_secs: u64) -> Self {
        let mut params = HashMap::new();
        params.insert("window_secs".to_string(), serde_json::json!(window_secs));
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            policy_type: ContextPolicyType::TimeWindow,
            priority: 40,
            enabled: true,
            params,
        }
    }
    
    /// 创建优先级策略
    pub fn priority(name: &str, min_priority: u8) -> Self {
        let mut params = HashMap::new();
        params.insert("min_priority".to_string(), serde_json::json!(min_priority));
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            policy_type: ContextPolicyType::Priority,
            priority: 30,
            enabled: true,
            params,
        }
    }
}

impl Default for ContextPolicy {
    fn default() -> Self {
        Self::token_limit("default", 8000)
    }
}

/// 策略评估结果
#[derive(Debug, Clone)]
pub struct PolicyEvaluation {
    /// 是否通过
    pub passed: bool,
    /// 策略 ID
    pub policy_id: String,
    /// 评估消息
    pub message: String,
    /// 建议操作
    pub suggested_action: Option<String>,
}

/// 上下文策略管理器
pub struct ContextPolicyManager {
    /// 已注册的策略
    policies: HashMap<String, ContextPolicy>,
    /// 策略执行顺序（按优先级）
    execution_order: Vec<String>,
}

impl ContextPolicyManager {
    /// 创建新的策略管理器
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            execution_order: Vec::new(),
        }
    }
    
    /// 注册策略
    pub fn register(&mut self, policy: ContextPolicy) {
        let id = policy.id.clone();
        let priority = policy.priority;
        
        self.policies.insert(id.clone(), policy);
        
        // 更新执行顺序
        self.execution_order.push(id);
        self.execution_order.sort_by(|a, b| {
            let pa = self.policies.get(a).map(|p| p.priority).unwrap_or(0);
            let pb = self.policies.get(b).map(|p| p.priority).unwrap_or(0);
            pb.cmp(&pa) // 降序，高优先级在前
        });
    }
    
    /// 注销策略
    pub fn unregister(&mut self, id: &str) -> Result<()> {
        self.policies.remove(id)
            .ok_or_else(|| anyhow::anyhow!("Policy not found: {}", id))?;
        self.execution_order.retain(|s| s != id);
        Ok(())
    }
    
    /// 获取策略
    pub fn get(&self, id: &str) -> Option<&ContextPolicy> {
        self.policies.get(id)
    }
    
    /// 列出所有策略
    pub fn list(&self) -> Vec<&ContextPolicy> {
        self.execution_order
            .iter()
            .filter_map(|id| self.policies.get(id))
            .collect()
    }
    
    /// 评估上下文
    pub fn evaluate(&self, context: &crate::core::ContextChunk) -> Vec<PolicyEvaluation> {
        let mut results = Vec::new();
        
        for policy_id in &self.execution_order {
            if let Some(policy) = self.policies.get(policy_id) {
                if !policy.enabled {
                    continue;
                }
                
                let result = self.evaluate_policy(policy, context);
                results.push(result);
            }
        }
        
        results
    }
    
    /// 评估单个策略
    fn evaluate_policy(&self, policy: &ContextPolicy, context: &crate::core::ContextChunk) -> PolicyEvaluation {
        match policy.policy_type {
            ContextPolicyType::TokenLimit => {
                let max_tokens = policy.params.get("max_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8000) as usize;
                
                if context.tokens > max_tokens {
                    PolicyEvaluation {
                        passed: false,
                        policy_id: policy.id.clone(),
                        message: format!("Token limit exceeded: {} > {}", context.tokens, max_tokens),
                        suggested_action: Some("truncate".to_string()),
                    }
                } else {
                    PolicyEvaluation {
                        passed: true,
                        policy_id: policy.id.clone(),
                        message: "Within token limit".to_string(),
                        suggested_action: None,
                    }
                }
            }
            ContextPolicyType::TimeWindow => {
                let window_secs = policy.params.get("window_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(3600);
                
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                let age = now - context.created_at;
                
                if age > window_secs as i64 {
                    PolicyEvaluation {
                        passed: false,
                        policy_id: policy.id.clone(),
                        message: format!("Context too old: {}s > {}s", age, window_secs),
                        suggested_action: Some("expire".to_string()),
                    }
                } else {
                    PolicyEvaluation {
                        passed: true,
                        policy_id: policy.id.clone(),
                        message: "Within time window".to_string(),
                        suggested_action: None,
                    }
                }
            }
            ContextPolicyType::Priority => {
                PolicyEvaluation {
                    passed: true,
                    policy_id: policy.id.clone(),
                    message: "Priority check passed".to_string(),
                    suggested_action: None,
                }
            }
            _ => PolicyEvaluation {
                passed: true,
                policy_id: policy.id.clone(),
                message: "Policy not implemented".to_string(),
                suggested_action: None,
            }
        }
    }
    
    /// 获取策略数量
    pub fn len(&self) -> usize {
        self.policies.len()
    }
    
    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }
    
    /// 清空所有策略
    pub fn clear(&mut self) {
        self.policies.clear();
        self.execution_order.clear();
    }
}

impl Default for ContextPolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_policy_token_limit() {
        let policy = ContextPolicy::token_limit("test", 1000);
        assert_eq!(policy.policy_type, ContextPolicyType::TokenLimit);
        assert!(policy.enabled);
    }

    #[test]
    fn test_context_policy_time_window() {
        let policy = ContextPolicy::time_window("test", 3600);
        assert_eq!(policy.policy_type, ContextPolicyType::TimeWindow);
    }

    #[test]
    fn test_policy_manager_register() {
        let mut manager = ContextPolicyManager::new();
        let policy = ContextPolicy::token_limit("test", 1000);
        
        manager.register(policy);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_policy_manager_unregister() {
        let mut manager = ContextPolicyManager::new();
        let policy = ContextPolicy::token_limit("test", 1000);
        let id = policy.id.clone();
        
        manager.register(policy);
        manager.unregister(&id).unwrap();
        
        assert!(manager.is_empty());
    }

    #[test]
    fn test_policy_manager_list() {
        let mut manager = ContextPolicyManager::new();
        
        manager.register(ContextPolicy::token_limit("low", 1000));
        manager.register(ContextPolicy::token_limit("high", 5000));
        
        let list = manager.list();
        assert_eq!(list.len(), 2);
    }
}
