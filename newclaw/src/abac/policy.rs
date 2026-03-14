//! Policy definitions for ABAC (Attribute-Based Access Control)
//!
//! This module defines:
//! - Policy structure
//! - Policy rules with conditions
//! - Policy effect (Allow/Deny)
//! - Policy priority

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::attribute::{AttributeCategory, AttributeValue, AttributeBag};

/// Policy effect (allow or deny)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    #[default]
    Allow,
    Deny,
}

impl Effect {
    /// Check if this is an allow effect
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow)
    }
    
    /// Check if this is a deny effect
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny)
    }
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::Deny => write!(f, "deny"),
        }
    }
}

/// Condition operator for policy rules
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Equals (exact match)
    Equals,
    
    /// Not equals
    NotEquals,
    
    /// Greater than
    GreaterThan,
    
    /// Greater than or equal
    GreaterThanOrEqual,
    
    /// Less than
    LessThan,
    
    /// Less than or equal
    LessThanOrEqual,
    
    /// String contains
    Contains,
    
    /// String starts with
    StartsWith,
    
    /// String ends with
    EndsWith,
    
    /// Value is in a set
    In,
    
    /// Value is not in a set
    NotIn,
    
    /// Matches regex pattern
    Matches,
    
    /// Check if attribute exists
    Exists,
    
    /// Check if attribute does not exist
    NotExists,
    
    /// Check if set contains any of the values
    AnyOf,
    
    /// Check if set contains all of the values
    AllOf,
}

/// A condition in a policy rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Attribute category
    pub category: AttributeCategory,
    
    /// Attribute name
    pub name: String,
    
    /// Condition operator
    pub operator: ConditionOperator,
    
    /// Value to compare against
    pub value: AttributeValue,
}

impl Condition {
    /// Create a new condition
    pub fn new(
        category: AttributeCategory,
        name: impl Into<String>,
        operator: ConditionOperator,
        value: AttributeValue,
    ) -> Self {
        Self {
            category,
            name: name.into(),
            operator,
            value,
        }
    }
    
    /// Create an equals condition
    pub fn equals(category: AttributeCategory, name: impl Into<String>, value: AttributeValue) -> Self {
        Self::new(category, name, ConditionOperator::Equals, value)
    }
    
    /// Create a not equals condition
    pub fn not_equals(category: AttributeCategory, name: impl Into<String>, value: AttributeValue) -> Self {
        Self::new(category, name, ConditionOperator::NotEquals, value)
    }
    
    /// Create an in condition
    pub fn in_list(category: AttributeCategory, name: impl Into<String>, values: Vec<AttributeValue>) -> Self {
        Self::new(category, name, ConditionOperator::In, AttributeValue::list(values))
    }
    
    /// Create a contains condition
    pub fn contains(category: AttributeCategory, name: impl Into<String>, value: AttributeValue) -> Self {
        Self::new(category, name, ConditionOperator::Contains, value)
    }
    
    /// Create an exists condition
    pub fn exists(category: AttributeCategory, name: impl Into<String>) -> Self {
        Self::new(category, name, ConditionOperator::Exists, AttributeValue::null())
    }
    
    /// Evaluate this condition against an attribute bag
    pub fn evaluate(&self, attributes: &AttributeBag) -> bool {
        let attr_value = attributes.get(&self.category, &self.name);
        
        match &self.operator {
            ConditionOperator::Exists => attr_value.is_some(),
            ConditionOperator::NotExists => attr_value.is_none(),
            _ => {
                if let Some(value) = attr_value {
                    self.compare(value)
                } else {
                    false
                }
            }
        }
    }
    
    /// Compare attribute value with condition value
    fn compare(&self, attr_value: &AttributeValue) -> bool {
        match &self.operator {
            ConditionOperator::Equals => attr_value.equals(&self.value),
            ConditionOperator::NotEquals => !attr_value.equals(&self.value),
            
            ConditionOperator::GreaterThan => {
                match (attr_value.as_number(), self.value.as_number()) {
                    (Some(a), Some(b)) => a > b,
                    _ => false,
                }
            }
            ConditionOperator::GreaterThanOrEqual => {
                match (attr_value.as_number(), self.value.as_number()) {
                    (Some(a), Some(b)) => a >= b,
                    _ => false,
                }
            }
            ConditionOperator::LessThan => {
                match (attr_value.as_number(), self.value.as_number()) {
                    (Some(a), Some(b)) => a < b,
                    _ => false,
                }
            }
            ConditionOperator::LessThanOrEqual => {
                match (attr_value.as_number(), self.value.as_number()) {
                    (Some(a), Some(b)) => a <= b,
                    _ => false,
                }
            }
            
            ConditionOperator::Contains => {
                match (attr_value.as_string(), self.value.as_string()) {
                    (Some(a), Some(b)) => a.contains(b),
                    _ => false,
                }
            }
            ConditionOperator::StartsWith => {
                match (attr_value.as_string(), self.value.as_string()) {
                    (Some(a), Some(b)) => a.starts_with(b),
                    _ => false,
                }
            }
            ConditionOperator::EndsWith => {
                match (attr_value.as_string(), self.value.as_string()) {
                    (Some(a), Some(b)) => a.ends_with(b),
                    _ => false,
                }
            }
            
            ConditionOperator::In => {
                if let Some(list) = self.value.as_list() {
                    list.iter().any(|v| attr_value.equals(v))
                } else {
                    false
                }
            }
            ConditionOperator::NotIn => {
                if let Some(list) = self.value.as_list() {
                    !list.iter().any(|v| attr_value.equals(v))
                } else {
                    true
                }
            }
            
            ConditionOperator::Matches => {
                match (attr_value.as_string(), self.value.as_string()) {
                    (Some(a), Some(pattern)) => {
                        regex::Regex::new(pattern).ok()
                            .map(|re| re.is_match(a))
                            .unwrap_or(false)
                    }
                    _ => false,
                }
            }
            
            ConditionOperator::AnyOf => {
                match (attr_value.as_set(), self.value.as_list()) {
                    (Some(set), Some(list)) => {
                        list.iter().any(|v| {
                            if let Some(s) = v.as_string() {
                                set.contains(s)
                            } else {
                                false
                            }
                        })
                    }
                    _ => false,
                }
            }
            ConditionOperator::AllOf => {
                match (attr_value.as_set(), self.value.as_list()) {
                    (Some(set), Some(list)) => {
                        list.iter().all(|v| {
                            if let Some(s) = v.as_string() {
                                set.contains(s)
                            } else {
                                false
                            }
                        })
                    }
                    _ => false,
                }
            }
            
            ConditionOperator::Exists | ConditionOperator::NotExists => {
                // Handled above
                false
            }
        }
    }
}

/// Logical operator for combining conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogicalOperator {
    #[default]
    And,
    Or,
}

/// A rule combining multiple conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule ID
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    
    /// Rule name
    pub name: String,
    
    /// Logical operator for combining conditions
    #[serde(default)]
    pub operator: LogicalOperator,
    
    /// Conditions to evaluate
    pub conditions: Vec<Condition>,
    
    /// Whether this rule is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Rule {
    /// Create a new rule with AND logic
    pub fn and(name: impl Into<String>, conditions: Vec<Condition>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            operator: LogicalOperator::And,
            conditions,
            enabled: true,
        }
    }
    
    /// Create a new rule with OR logic
    pub fn or(name: impl Into<String>, conditions: Vec<Condition>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            operator: LogicalOperator::Or,
            conditions,
            enabled: true,
        }
    }
    
    /// Create a single condition rule
    pub fn single(name: impl Into<String>, condition: Condition) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            operator: LogicalOperator::And,
            conditions: vec![condition],
            enabled: true,
        }
    }
    
    /// Enable/disable the rule
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    
    /// Evaluate this rule against attributes
    pub fn evaluate(&self, attributes: &AttributeBag) -> bool {
        if !self.enabled || self.conditions.is_empty() {
            return false;
        }
        
        match self.operator {
            LogicalOperator::And => self.conditions.iter().all(|c| c.evaluate(attributes)),
            LogicalOperator::Or => self.conditions.iter().any(|c| c.evaluate(attributes)),
        }
    }
}

/// Policy combining algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicyCombiningAlgorithm {
    /// Deny overrides allow
    #[default]
    DenyOverrides,
    
    /// Allow overrides deny
    AllowOverrides,
    
    /// First applicable rule wins
    FirstApplicable,
    
    /// Only allow if all allow
    OnlyOneApplicable,
}

/// A complete ABAC policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    
    /// Policy name
    pub name: String,
    
    /// Policy description
    #[serde(default)]
    pub description: String,
    
    /// Policy effect (allow or deny)
    pub effect: Effect,
    
    /// Policy priority (higher = more important)
    #[serde(default)]
    pub priority: u32,
    
    /// Target resource types this policy applies to
    #[serde(default)]
    pub target_resources: Vec<String>,
    
    /// Target actions this policy applies to
    #[serde(default)]
    pub target_actions: Vec<String>,
    
    /// Rules to evaluate (combined with AND logic by default)
    pub rules: Vec<Rule>,
    
    /// Policy combining algorithm
    #[serde(default)]
    pub combining_algorithm: PolicyCombiningAlgorithm,
    
    /// Whether this policy is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Creation timestamp
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    
    /// Last update timestamp
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    
    /// Policy version
    #[serde(default)]
    pub version: u32,
    
    /// Policy metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Policy {
    /// Create a new policy
    pub fn new(name: impl Into<String>, effect: Effect) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            effect,
            priority: 0,
            target_resources: Vec::new(),
            target_actions: Vec::new(),
            rules: Vec::new(),
            combining_algorithm: PolicyCombiningAlgorithm::default(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
            metadata: HashMap::new(),
        }
    }
    
    /// Create an allow policy
    pub fn allow(name: impl Into<String>) -> Self {
        Self::new(name, Effect::Allow)
    }
    
    /// Create a deny policy
    pub fn deny(name: impl Into<String>) -> Self {
        Self::new(name, Effect::Deny)
    }
    
    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self.updated_at = Utc::now();
        self
    }
    
    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self.updated_at = Utc::now();
        self
    }
    
    /// Add target resource
    pub fn with_target_resource(mut self, resource: impl Into<String>) -> Self {
        self.target_resources.push(resource.into());
        self.updated_at = Utc::now();
        self
    }
    
    /// Add target action
    pub fn with_target_action(mut self, action: impl Into<String>) -> Self {
        self.target_actions.push(action.into());
        self.updated_at = Utc::now();
        self
    }
    
    /// Add a rule
    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self.updated_at = Utc::now();
        self
    }
    
    /// Set combining algorithm
    pub fn with_combining_algorithm(mut self, algorithm: PolicyCombiningAlgorithm) -> Self {
        self.combining_algorithm = algorithm;
        self
    }
    
    /// Set enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self.updated_at = Utc::now();
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self.updated_at = Utc::now();
        self
    }
    
    /// Check if this policy applies to a resource type
    pub fn applies_to_resource(&self, resource_type: &str) -> bool {
        self.target_resources.is_empty() || self.target_resources.iter().any(|r| {
            r == "*" || r == resource_type || (r.ends_with('*') && resource_type.starts_with(&r[..r.len()-1]))
        })
    }
    
    /// Check if this policy applies to an action
    pub fn applies_to_action(&self, action: &str) -> bool {
        self.target_actions.is_empty() || self.target_actions.iter().any(|a| {
            a == "*" || a == action || (a.ends_with('*') && action.starts_with(&a[..a.len()-1]))
        })
    }
    
    /// Evaluate all rules against attributes
    pub fn evaluate(&self, attributes: &AttributeBag) -> bool {
        if !self.enabled {
            return false;
        }
        
        if self.rules.is_empty() {
            return true; // No rules means policy always matches
        }
        
        match self.combining_algorithm {
            PolicyCombiningAlgorithm::DenyOverrides => {
                // All rules must pass
                self.rules.iter().all(|r| r.evaluate(attributes))
            }
            PolicyCombiningAlgorithm::AllowOverrides => {
                // Any rule must pass
                self.rules.iter().any(|r| r.evaluate(attributes))
            }
            PolicyCombiningAlgorithm::FirstApplicable => {
                // First enabled rule determines result
                self.rules.iter().find(|r| r.enabled).map(|r| r.evaluate(attributes)).unwrap_or(false)
            }
            PolicyCombiningAlgorithm::OnlyOneApplicable => {
                // Only one rule should be applicable
                let applicable: Vec<_> = self.rules.iter().filter(|r| r.evaluate(attributes)).collect();
                applicable.len() == 1
            }
        }
    }
}

/// Policy set containing multiple policies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicySet {
    /// Policies in this set
    pub policies: Vec<Policy>,
    
    /// Default effect when no policy matches
    #[serde(default)]
    pub default_effect: Effect,
}

impl PolicySet {
    /// Create an empty policy set with deny-by-default
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            default_effect: Effect::Deny,
        }
    }
    
    /// Create with allow-by-default
    pub fn allow_by_default() -> Self {
        Self {
            policies: Vec::new(),
            default_effect: Effect::Allow,
        }
    }
    
    /// Add a policy
    pub fn add(mut self, policy: Policy) -> Self {
        self.policies.push(policy);
        self
    }
    
    /// Get policies sorted by priority (highest first)
    pub fn sorted_by_priority(&self) -> Vec<&Policy> {
        let mut policies: Vec<_> = self.policies.iter().filter(|p| p.enabled).collect();
        policies.sort_by(|a, b| b.priority.cmp(&a.priority));
        policies
    }
    
    /// Find applicable policies for a resource and action
    pub fn find_applicable(&self, resource_type: &str, action: &str) -> Vec<&Policy> {
        self.sorted_by_priority()
            .into_iter()
            .filter(|p| p.applies_to_resource(resource_type) && p.applies_to_action(action))
            .collect()
    }
}

/// Policy error type
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    #[error("Policy not found: {0}")]
    NotFound(String),
    
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),
    
    #[error("Policy evaluation error: {0}")]
    EvaluationError(String),
    
    #[error("Policy conflict: {0}")]
    Conflict(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_effect() {
        assert!(Effect::Allow.is_allow());
        assert!(!Effect::Allow.is_deny());
        assert!(Effect::Deny.is_deny());
        assert!(!Effect::Deny.is_allow());
    }
    
    #[test]
    fn test_condition_equals() {
        let condition = Condition::equals(
            AttributeCategory::Subject,
            "role",
            AttributeValue::string("admin"),
        );
        
        let mut bag = AttributeBag::new();
        
        // Missing attribute
        assert!(!condition.evaluate(&bag));
        
        // Wrong value
        bag.set(AttributeCategory::Subject, "role", AttributeValue::string("user"));
        assert!(!condition.evaluate(&bag));
        
        // Correct value
        bag.set(AttributeCategory::Subject, "role", AttributeValue::string("admin"));
        assert!(condition.evaluate(&bag));
    }
    
    #[test]
    fn test_condition_in_list() {
        let condition = Condition::in_list(
            AttributeCategory::Subject,
            "role",
            vec![
                AttributeValue::string("admin"),
                AttributeValue::string("editor"),
            ],
        );
        
        let mut bag = AttributeBag::new();
        
        bag.set(AttributeCategory::Subject, "role", AttributeValue::string("admin"));
        assert!(condition.evaluate(&bag));
        
        bag.set(AttributeCategory::Subject, "role", AttributeValue::string("editor"));
        assert!(condition.evaluate(&bag));
        
        bag.set(AttributeCategory::Subject, "role", AttributeValue::string("viewer"));
        assert!(!condition.evaluate(&bag));
    }
    
    #[test]
    fn test_condition_numeric() {
        let condition = Condition::new(
            AttributeCategory::Environment,
            "hour",
            ConditionOperator::GreaterThan,
            AttributeValue::number(9.0),
        );
        
        let mut bag = AttributeBag::new();
        
        bag.set(AttributeCategory::Environment, "hour", AttributeValue::number(10.0));
        assert!(condition.evaluate(&bag));
        
        bag.set(AttributeCategory::Environment, "hour", AttributeValue::number(8.0));
        assert!(!condition.evaluate(&bag));
    }
    
    #[test]
    fn test_rule_and() {
        let rule = Rule::and("admin_during_hours", vec![
            Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
            Condition::new(
                AttributeCategory::Environment,
                "hour",
                ConditionOperator::GreaterThanOrEqual,
                AttributeValue::number(9.0),
            ),
        ]);
        
        let mut bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("admin"))
            .with_environment("hour", AttributeValue::number(10.0));
        
        assert!(rule.evaluate(&bag));
        
        // Wrong role
        bag.set(AttributeCategory::Subject, "role", AttributeValue::string("user"));
        assert!(!rule.evaluate(&bag));
    }
    
    #[test]
    fn test_rule_or() {
        let rule = Rule::or("admin_or_owner", vec![
            Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
            Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("owner")),
        ]);
        
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("admin"));
        
        assert!(rule.evaluate(&bag));
        
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("owner"));
        
        assert!(rule.evaluate(&bag));
        
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("user"));
        
        assert!(!rule.evaluate(&bag));
    }
    
    #[test]
    fn test_policy() {
        let policy = Policy::allow("admin_full_access")
            .with_priority(100)
            .with_target_resource("*")
            .with_target_action("*")
            .with_rule(Rule::single(
                "is_admin",
                Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
            ));
        
        assert!(policy.applies_to_resource("document"));
        assert!(policy.applies_to_action("read"));
        
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("admin"));
        
        assert!(policy.evaluate(&bag));
        
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("user"));
        
        assert!(!policy.evaluate(&bag));
    }
    
    #[test]
    fn test_policy_with_multiple_rules() {
        let policy = Policy::allow("document_access")
            .with_rule(Rule::or("role_check", vec![
                Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("editor")),
            ]))
            .with_rule(Rule::single(
                "active_check",
                Condition::equals(AttributeCategory::Subject, "status", AttributeValue::string("active")),
            ));
        
        // Admin and active
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("admin"))
            .with_subject("status", AttributeValue::string("active"));
        assert!(policy.evaluate(&bag));
        
        // Admin but inactive
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("admin"))
            .with_subject("status", AttributeValue::string("inactive"));
        assert!(!policy.evaluate(&bag));
        
        // Editor and active
        let bag = AttributeBag::new()
            .with_subject("role", AttributeValue::string("editor"))
            .with_subject("status", AttributeValue::string("active"));
        assert!(policy.evaluate(&bag));
    }
    
    #[test]
    fn test_policy_set() {
        let policy_set = PolicySet::new()
            .add(Policy::deny("deny_guest")
                .with_priority(1000)
                .with_rule(Rule::single(
                    "is_guest",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("guest")),
                )))
            .add(Policy::allow("allow_admin")
                .with_priority(100)
                .with_rule(Rule::single(
                    "is_admin",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                )));
        
        // Guest should be denied (higher priority)
        let policies = policy_set.find_applicable("document", "read");
        assert_eq!(policies.len(), 2);
        
        // Verify priority order
        assert_eq!(policies[0].name, "deny_guest");
        assert_eq!(policies[1].name, "allow_admin");
    }
    
    #[test]
    fn test_policy_target_matching() {
        let policy = Policy::allow("document_policy")
            .with_target_resource("document")
            .with_target_action("read")
            .with_target_action("write");
        
        assert!(policy.applies_to_resource("document"));
        assert!(!policy.applies_to_resource("user"));
        assert!(policy.applies_to_action("read"));
        assert!(policy.applies_to_action("write"));
        assert!(!policy.applies_to_action("delete"));
    }
    
    #[test]
    fn test_policy_wildcard_matching() {
        let policy = Policy::allow("resource_policy")
            .with_target_resource("doc*")
            .with_target_action("*");
        
        assert!(policy.applies_to_resource("document"));
        assert!(policy.applies_to_resource("doc"));
        assert!(!policy.applies_to_resource("user"));
        assert!(policy.applies_to_action("read"));
        assert!(policy.applies_to_action("write"));
    }
}