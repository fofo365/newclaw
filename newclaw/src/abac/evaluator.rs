//! Policy evaluation engine for ABAC
//!
//! This module provides:
//! - PolicyMatcher for matching policies to requests
//! - PolicyEvaluator for evaluating policy rules
//! - Request context for authorization requests

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use super::attribute::{AttributeBag, AttributeValue, AttributeCategory, AttributeResolver};
use super::policy::{Policy, PolicySet, Effect, PolicyError, PolicyCombiningAlgorithm};

/// Authorization request context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzRequest {
    /// Unique request ID
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    
    /// Subject identifier (who is making the request)
    pub subject_id: String,
    
    /// Resource identifier (what is being accessed)
    pub resource_id: String,
    
    /// Resource type
    pub resource_type: String,
    
    /// Action being performed
    pub action: String,
    
    /// Request attributes
    #[serde(default)]
    pub attributes: AttributeBag,
    
    /// Request timestamp
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    
    /// Request metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl AuthzRequest {
    /// Create a new authorization request
    pub fn new(
        subject_id: impl Into<String>,
        resource_id: impl Into<String>,
        resource_type: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject_id: subject_id.into(),
            resource_id: resource_id.into(),
            resource_type: resource_type.into(),
            action: action.into(),
            attributes: AttributeBag::new(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add subject attribute
    pub fn with_subject_attr(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.set(AttributeCategory::Subject, name, value);
        self
    }
    
    /// Add resource attribute
    pub fn with_resource_attr(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.set(AttributeCategory::Resource, name, value);
        self
    }
    
    /// Add action attribute
    pub fn with_action_attr(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.set(AttributeCategory::Action, name, value);
        self
    }
    
    /// Add environment attribute
    pub fn with_env_attr(mut self, name: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.set(AttributeCategory::Environment, name, value);
        self
    }
    
    /// Set attributes from attribute bag
    pub fn with_attributes(mut self, attributes: AttributeBag) -> Self {
        self.attributes = attributes;
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// Get the attribute bag with default attributes
    pub fn enriched_attributes(&self) -> AttributeBag {
        let mut attrs = self.attributes.clone();
        
        // Add default subject attributes
        attrs.set(
            AttributeCategory::Subject,
            "id",
            AttributeValue::string(&self.subject_id),
        );
        
        // Add default resource attributes
        attrs.set(
            AttributeCategory::Resource,
            "id",
            AttributeValue::string(&self.resource_id),
        );
        attrs.set(
            AttributeCategory::Resource,
            "type",
            AttributeValue::string(&self.resource_type),
        );
        
        // Add default action attributes
        attrs.set(
            AttributeCategory::Action,
            "name",
            AttributeValue::string(&self.action),
        );
        
        // Add default environment attributes
        attrs.set(
            AttributeCategory::Environment,
            "timestamp",
            AttributeValue::string(self.timestamp.to_rfc3339()),
        );
        
        attrs
    }
}

/// Policy match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMatch {
    /// The matched policy
    pub policy_id: Uuid,
    pub policy_name: String,
    
    /// Whether the policy evaluated to true
    pub matched: bool,
    
    /// Policy effect
    pub effect: Effect,
    
    /// Policy priority
    pub priority: u32,
}

/// Policy matcher for finding applicable policies
pub struct PolicyMatcher {
    /// Policy set to match against
    policy_set: PolicySet,
}

impl PolicyMatcher {
    /// Create a new policy matcher
    pub fn new(policy_set: PolicySet) -> Self {
        Self { policy_set }
    }
    
    /// Create with empty policy set
    pub fn empty() -> Self {
        Self::new(PolicySet::new())
    }
    
    /// Get the policy set
    pub fn policy_set(&self) -> &PolicySet {
        &self.policy_set
    }
    
    /// Get mutable policy set
    pub fn policy_set_mut(&mut self) -> &mut PolicySet {
        &mut self.policy_set
    }
    
    /// Find all applicable policies for a request
    pub fn find_applicable(&self, request: &AuthzRequest) -> Vec<&Policy> {
        self.policy_set.find_applicable(&request.resource_type, &request.action)
    }
    
    /// Match policies and return match results
    pub fn match_policies(&self, request: &AuthzRequest) -> Vec<PolicyMatch> {
        let attrs = request.enriched_attributes();
        
        self.find_applicable(request)
            .into_iter()
            .map(|policy| {
                let matched = policy.evaluate(&attrs);
                PolicyMatch {
                    policy_id: policy.id,
                    policy_name: policy.name.clone(),
                    matched,
                    effect: policy.effect,
                    priority: policy.priority,
                }
            })
            .collect()
    }
}

/// Policy evaluator result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Request ID
    pub request_id: Uuid,
    
    /// Final decision effect
    pub effect: Effect,
    
    /// Whether any policy matched
    pub any_matched: bool,
    
    /// All policy match results
    pub matches: Vec<PolicyMatch>,
    
    /// Evaluation timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Evaluation duration in microseconds
    pub duration_us: u64,
    
    /// Additional notes
    #[serde(default)]
    pub notes: Vec<String>,
}

impl EvaluationResult {
    /// Check if the result is allow
    pub fn is_allowed(&self) -> bool {
        self.effect == Effect::Allow
    }
    
    /// Check if the result is deny
    pub fn is_denied(&self) -> bool {
        self.effect == Effect::Deny
    }
    
    /// Get all matched (evaluated to true) policies
    pub fn matched_policies(&self) -> Vec<&PolicyMatch> {
        self.matches.iter().filter(|m| m.matched).collect()
    }
    
    /// Get all policies that evaluated to deny
    pub fn deny_policies(&self) -> Vec<&PolicyMatch> {
        self.matches.iter()
            .filter(|m| m.matched && m.effect == Effect::Deny)
            .collect()
    }
    
    /// Get all policies that evaluated to allow
    pub fn allow_policies(&self) -> Vec<&PolicyMatch> {
        self.matches.iter()
            .filter(|m| m.matched && m.effect == Effect::Allow)
            .collect()
    }
}

/// Policy evaluator for evaluating authorization requests
pub struct PolicyEvaluator {
    /// Policy matcher
    matcher: PolicyMatcher,
    
    /// Default combining algorithm for evaluation
    default_algorithm: PolicyCombiningAlgorithm,
}

impl PolicyEvaluator {
    /// Create a new policy evaluator
    pub fn new(policy_set: PolicySet) -> Self {
        Self {
            matcher: PolicyMatcher::new(policy_set),
            default_algorithm: PolicyCombiningAlgorithm::DenyOverrides,
        }
    }
    
    /// Create with empty policy set
    pub fn empty() -> Self {
        Self::new(PolicySet::new())
    }
    
    /// Set default combining algorithm
    pub fn with_algorithm(mut self, algorithm: PolicyCombiningAlgorithm) -> Self {
        self.default_algorithm = algorithm;
        self
    }
    
    /// Get the policy matcher
    pub fn matcher(&self) -> &PolicyMatcher {
        &self.matcher
    }
    
    /// Get mutable policy matcher
    pub fn matcher_mut(&mut self) -> &mut PolicyMatcher {
        &mut self.matcher
    }
    
    /// Evaluate an authorization request
    pub fn evaluate(&self, request: &AuthzRequest) -> EvaluationResult {
        let start = std::time::Instant::now();
        
        let matches = self.matcher.match_policies(request);
        let any_matched = matches.iter().any(|m| m.matched);
        
        // Determine final effect
        let effect = self.determine_effect(&matches, any_matched);
        
        let duration_us = start.elapsed().as_micros() as u64;
        
        EvaluationResult {
            request_id: request.id,
            effect,
            any_matched,
            matches,
            timestamp: Utc::now(),
            duration_us,
            notes: Vec::new(),
        }
    }
    
    /// Determine the final effect from matches
    fn determine_effect(&self, matches: &[PolicyMatch], any_matched: bool) -> Effect {
        if !any_matched {
            // No policies matched - use default deny
            return Effect::Deny;
        }
        
        // Get matched policies (sorted by priority already)
        let matched: Vec<_> = matches.iter().filter(|m| m.matched).collect();
        
        match self.default_algorithm {
            PolicyCombiningAlgorithm::DenyOverrides => {
                // Check for any deny first
                if matched.iter().any(|m| m.effect == Effect::Deny) {
                    return Effect::Deny;
                }
                // Then check for allow
                if matched.iter().any(|m| m.effect == Effect::Allow) {
                    return Effect::Allow;
                }
                Effect::Deny
            }
            PolicyCombiningAlgorithm::AllowOverrides => {
                // Check for any allow first
                if matched.iter().any(|m| m.effect == Effect::Allow) {
                    return Effect::Allow;
                }
                // Then check for deny
                if matched.iter().any(|m| m.effect == Effect::Deny) {
                    return Effect::Deny;
                }
                Effect::Deny
            }
            PolicyCombiningAlgorithm::FirstApplicable => {
                // First matched policy determines result
                matched.first().map(|m| m.effect).unwrap_or(Effect::Deny)
            }
            PolicyCombiningAlgorithm::OnlyOneApplicable => {
                // Only one policy should match
                if matched.len() == 1 {
                    matched[0].effect
                } else {
                    // Multiple policies matched - conflict
                    Effect::Deny
                }
            }
        }
    }
    
    /// Evaluate with attribute resolution
    pub async fn evaluate_with_resolver(
        &self,
        request: &AuthzRequest,
        resolver: &dyn AttributeResolver,
    ) -> Result<EvaluationResult, PolicyError> {
        // Resolve additional attributes
        let mut enriched = request.enriched_attributes();
        
        // Resolve subject attributes
        let subject_attrs = resolver.resolve_category(&AttributeCategory::Subject).await
            .map_err(|e| PolicyError::EvaluationError(e.to_string()))?;
        for (k, v) in subject_attrs {
            enriched.subject.entry(k).or_insert(v);
        }
        
        // Resolve resource attributes
        let resource_attrs = resolver.resolve_category(&AttributeCategory::Resource).await
            .map_err(|e| PolicyError::EvaluationError(e.to_string()))?;
        for (k, v) in resource_attrs {
            enriched.resource.entry(k).or_insert(v);
        }
        
        // Create enriched request
        let enriched_request = AuthzRequest::new(
            &request.subject_id,
            &request.resource_id,
            &request.resource_type,
            &request.action,
        )
        .with_attributes(enriched);
        
        Ok(self.evaluate(&enriched_request))
    }
}

/// Policy evaluator builder
pub struct PolicyEvaluatorBuilder {
    policy_set: PolicySet,
    algorithm: PolicyCombiningAlgorithm,
}

impl PolicyEvaluatorBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            policy_set: PolicySet::new(),
            algorithm: PolicyCombiningAlgorithm::DenyOverrides,
        }
    }
    
    /// Add a policy
    pub fn add_policy(mut self, policy: Policy) -> Self {
        self.policy_set.policies.push(policy);
        self
    }
    
    /// Set the policy set
    pub fn with_policy_set(mut self, policy_set: PolicySet) -> Self {
        self.policy_set = policy_set;
        self
    }
    
    /// Set the combining algorithm
    pub fn with_algorithm(mut self, algorithm: PolicyCombiningAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }
    
    /// Build the evaluator
    pub fn build(self) -> PolicyEvaluator {
        PolicyEvaluator::new(self.policy_set)
            .with_algorithm(self.algorithm)
    }
}

impl Default for PolicyEvaluatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abac::policy::{Rule, Condition};
    
    fn create_test_policy_set() -> PolicySet {
        PolicySet::new()
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
                )))
            .add(Policy::allow("allow_owner")
                .with_priority(50)
                .with_rule(Rule::single(
                    "is_owner",
                    Condition::equals(AttributeCategory::Subject, "id", AttributeValue::string("owner123")),
                )))
    }
    
    #[test]
    fn test_authz_request() {
        let request = AuthzRequest::new("user123", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"))
            .with_env_attr("ip", AttributeValue::string("192.168.1.1"));
        
        assert_eq!(request.subject_id, "user123");
        assert_eq!(request.resource_id, "doc456");
        assert_eq!(request.resource_type, "document");
        assert_eq!(request.action, "read");
    }
    
    #[test]
    fn test_enriched_attributes() {
        let request = AuthzRequest::new("user123", "doc456", "document", "read");
        let attrs = request.enriched_attributes();
        
        // Check default attributes are added
        assert_eq!(
            attrs.get(&AttributeCategory::Subject, "id"),
            Some(&AttributeValue::string("user123"))
        );
        assert_eq!(
            attrs.get(&AttributeCategory::Resource, "id"),
            Some(&AttributeValue::string("doc456"))
        );
        assert_eq!(
            attrs.get(&AttributeCategory::Resource, "type"),
            Some(&AttributeValue::string("document"))
        );
        assert_eq!(
            attrs.get(&AttributeCategory::Action, "name"),
            Some(&AttributeValue::string("read"))
        );
    }
    
    #[test]
    fn test_policy_matcher() {
        let matcher = PolicyMatcher::new(create_test_policy_set());
        
        let request = AuthzRequest::new("user123", "doc456", "document", "read");
        
        let applicable = matcher.find_applicable(&request);
        assert!(!applicable.is_empty());
        
        let matches = matcher.match_policies(&request);
        assert_eq!(matches.len(), 3); // All 3 policies are applicable
    }
    
    #[test]
    fn test_evaluator_deny_guest() {
        let evaluator = PolicyEvaluator::new(create_test_policy_set());
        
        let request = AuthzRequest::new("guest1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("guest"));
        
        let result = evaluator.evaluate(&request);
        
        assert!(result.is_denied());
        assert!(result.any_matched);
    }
    
    #[test]
    fn test_evaluator_allow_admin() {
        let evaluator = PolicyEvaluator::new(create_test_policy_set());
        
        let request = AuthzRequest::new("admin1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"));
        
        let result = evaluator.evaluate(&request);
        
        assert!(result.is_allowed());
        assert!(result.any_matched);
    }
    
    #[test]
    fn test_evaluator_deny_overrides() {
        // Create policy set where both deny and allow could match
        let policy_set = PolicySet::new()
            .add(Policy::deny("deny_night")
                .with_priority(1000)
                .with_rule(Rule::single(
                    "night_time",
                    Condition::new(
                        AttributeCategory::Environment,
                        "hour",
                        crate::abac::policy::ConditionOperator::LessThan,
                        AttributeValue::number(6.0),
                    ),
                )))
            .add(Policy::allow("allow_admin")
                .with_priority(100)
                .with_rule(Rule::single(
                    "is_admin",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                )));
        
        let evaluator = PolicyEvaluator::new(policy_set);
        
        // Admin at night - deny should override
        let request = AuthzRequest::new("admin1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"))
            .with_env_attr("hour", AttributeValue::number(3.0));
        
        let result = evaluator.evaluate(&request);
        assert!(result.is_denied()); // Deny overrides
    }
    
    #[test]
    fn test_evaluator_no_match() {
        let evaluator = PolicyEvaluator::new(create_test_policy_set());
        
        // User with no matching role
        let request = AuthzRequest::new("user1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("user"));
        
        let result = evaluator.evaluate(&request);
        
        assert!(result.is_denied()); // Default deny
        assert!(!result.any_matched);
    }
    
    #[test]
    fn test_evaluator_first_applicable() {
        let policy_set = PolicySet::new()
            .add(Policy::allow("allow_first")
                .with_priority(100)
                .with_rule(Rule::single(
                    "always_match",
                    Condition::exists(AttributeCategory::Subject, "id"),
                )))
            .add(Policy::deny("deny_second")
                .with_priority(50)
                .with_rule(Rule::single(
                    "always_match",
                    Condition::exists(AttributeCategory::Subject, "id"),
                )));
        
        let evaluator = PolicyEvaluatorBuilder::new()
            .with_policy_set(policy_set)
            .with_algorithm(PolicyCombiningAlgorithm::FirstApplicable)
            .build();
        
        let request = AuthzRequest::new("user1", "doc456", "document", "read");
        let result = evaluator.evaluate(&request);
        
        assert!(result.is_allowed()); // First applicable is allow
    }
    
    #[test]
    fn test_evaluator_builder() {
        let evaluator = PolicyEvaluatorBuilder::new()
            .add_policy(Policy::allow("test_policy")
                .with_rule(Rule::single(
                    "is_admin",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                )))
            .with_algorithm(PolicyCombiningAlgorithm::AllowOverrides)
            .build();
        
        let request = AuthzRequest::new("admin1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"));
        
        let result = evaluator.evaluate(&request);
        assert!(result.is_allowed());
    }
    
    #[tokio::test]
    async fn test_evaluate_with_resolver() {
        use crate::abac::attribute::StaticAttributeResolver;
        
        let evaluator = PolicyEvaluator::new(create_test_policy_set());
        
        let resolver = StaticAttributeResolver::new()
            .with_subject("role", AttributeValue::string("admin"));
        
        let request = AuthzRequest::new("admin1", "doc456", "document", "read");
        
        let result = evaluator.evaluate_with_resolver(&request, &resolver).await.unwrap();
        
        assert!(result.is_allowed());
    }
    
    #[test]
    fn test_evaluation_result() {
        let result = EvaluationResult {
            request_id: Uuid::new_v4(),
            effect: Effect::Allow,
            any_matched: true,
            matches: vec![
                PolicyMatch {
                    policy_id: Uuid::new_v4(),
                    policy_name: "policy1".to_string(),
                    matched: true,
                    effect: Effect::Allow,
                    priority: 100,
                },
                PolicyMatch {
                    policy_id: Uuid::new_v4(),
                    policy_name: "policy2".to_string(),
                    matched: false,
                    effect: Effect::Deny,
                    priority: 50,
                },
            ],
            timestamp: Utc::now(),
            duration_us: 100,
            notes: vec![],
        };
        
        assert!(result.is_allowed());
        assert_eq!(result.matched_policies().len(), 1);
        assert_eq!(result.allow_policies().len(), 1);
        assert_eq!(result.deny_policies().len(), 0);
    }
}