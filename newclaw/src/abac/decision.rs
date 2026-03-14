//! Decision engine for ABAC
//!
//! This module provides:
//! - Decision result types
//! - DecisionEngine for making authorization decisions
//! - Audit logging for decisions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use parking_lot::RwLock;
use super::attribute::AttributeValue;
use super::policy::Effect;
use super::evaluator::{AuthzRequest, EvaluationResult, PolicyEvaluator};

/// Authorization decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    /// Access permitted
    Permit,
    
    /// Access denied
    Deny,
    
    /// No applicable policy found
    NotApplicable,
    
    /// Indeterminate (error during evaluation)
    Indeterminate,
}

impl Decision {
    /// Check if this is a permit
    pub fn is_permit(&self) -> bool {
        matches!(self, Self::Permit)
    }
    
    /// Check if this is a deny
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny)
    }
    
    /// Check if this is not applicable
    pub fn is_not_applicable(&self) -> bool {
        matches!(self, Self::NotApplicable)
    }
    
    /// Check if this is indeterminate
    pub fn is_indeterminate(&self) -> bool {
        matches!(self, Self::Indeterminate)
    }
    
    /// Convert from effect
    pub fn from_effect(effect: Effect) -> Self {
        match effect {
            Effect::Allow => Self::Permit,
            Effect::Deny => Self::Deny,
        }
    }
    
    /// Convert to effect (None for NotApplicable/Indeterminate)
    pub fn to_effect(&self) -> Option<Effect> {
        match self {
            Self::Permit => Some(Effect::Allow),
            Self::Deny => Some(Effect::Deny),
            Self::NotApplicable | Self::Indeterminate => None,
        }
    }
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Permit => write!(f, "Permit"),
            Self::Deny => write!(f, "Deny"),
            Self::NotApplicable => write!(f, "NotApplicable"),
            Self::Indeterminate => write!(f, "Indeterminate"),
        }
    }
}

impl Default for Decision {
    fn default() -> Self {
        Self::Deny // Default deny
    }
}

/// Authorization decision result with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    /// Unique decision ID
    pub id: Uuid,
    
    /// The decision
    pub decision: Decision,
    
    /// The request that was evaluated
    pub request_id: Uuid,
    
    /// Subject ID
    pub subject_id: String,
    
    /// Resource ID
    pub resource_id: String,
    
    /// Resource type
    pub resource_type: String,
    
    /// Action
    pub action: String,
    
    /// Evaluation result details
    pub evaluation: EvaluationResult,
    
    /// Decision timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Decision reason
    pub reason: DecisionReason,
    
    /// Obligations (actions to be performed)
    #[serde(default)]
    pub obligations: Vec<Obligation>,
    
    /// Advice (recommendations)
    #[serde(default)]
    pub advice: Vec<Advice>,
}

impl DecisionResult {
    /// Check if access is permitted
    pub fn is_permitted(&self) -> bool {
        self.decision.is_permit()
    }
    
    /// Check if access is denied
    pub fn is_denied(&self) -> bool {
        self.decision.is_deny()
    }
    
    /// Get the matched policies that contributed to this decision
    pub fn contributing_policies(&self) -> Vec<&Uuid> {
        self.evaluation.matched_policies()
            .iter()
            .map(|m| &m.policy_id)
            .collect()
    }
}

/// Reason for the decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    /// Policy explicitly permitted
    PolicyPermit {
        policy_id: Uuid,
        policy_name: String,
    },
    
    /// Policy explicitly denied
    PolicyDeny {
        policy_id: Uuid,
        policy_name: String,
    },
    
    /// No applicable policy found
    NoApplicablePolicy,
    
    /// Deny by default
    DenyByDefault,
    
    /// Evaluation error
    EvaluationError(String),
    
    /// Multiple conflicting policies
    ConflictingPolicies {
        allow_policies: Vec<String>,
        deny_policies: Vec<String>,
    },
}

/// Obligation to be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    /// Obligation ID
    pub id: String,
    
    /// Obligation type
    pub obligation_type: String,
    
    /// Parameters
    #[serde(default)]
    pub params: HashMap<String, AttributeValue>,
}

impl Obligation {
    /// Create a new obligation
    pub fn new(id: impl Into<String>, obligation_type: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            obligation_type: obligation_type.into(),
            params: HashMap::new(),
        }
    }
    
    /// Add a parameter
    pub fn with_param(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.params.insert(key.into(), value);
        self
    }
}

/// Advice/recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advice {
    /// Advice ID
    pub id: String,
    
    /// Advice type
    pub advice_type: String,
    
    /// Message
    pub message: String,
    
    /// Additional data
    #[serde(default)]
    pub data: HashMap<String, AttributeValue>,
}

impl Advice {
    /// Create new advice
    pub fn new(id: impl Into<String>, advice_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            advice_type: advice_type.into(),
            message: message.into(),
            data: HashMap::new(),
        }
    }
    
    /// Add data
    pub fn with_data(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: Uuid,
    
    /// Decision ID
    pub decision_id: Uuid,
    
    /// Decision
    pub decision: Decision,
    
    /// Subject ID
    pub subject_id: String,
    
    /// Resource ID
    pub resource_id: String,
    
    /// Resource type
    pub resource_type: String,
    
    /// Action
    pub action: String,
    
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Duration in microseconds
    pub duration_us: u64,
    
    /// Number of policies evaluated
    pub policies_evaluated: usize,
    
    /// Number of policies matched
    pub policies_matched: usize,
    
    /// Matched policy names
    pub matched_policy_names: Vec<String>,
    
    /// Reason
    pub reason: DecisionReason,
    
    /// Additional context
    #[serde(default)]
    pub context: HashMap<String, String>,
}

impl AuditEntry {
    /// Create from decision result
    pub fn from_result(result: &DecisionResult) -> Self {
        let matched: Vec<_> = result.evaluation.matched_policies();
        
        Self {
            id: Uuid::new_v4(),
            decision_id: result.id,
            decision: result.decision,
            subject_id: result.subject_id.clone(),
            resource_id: result.resource_id.clone(),
            resource_type: result.resource_type.clone(),
            action: result.action.clone(),
            timestamp: result.timestamp,
            duration_us: result.evaluation.duration_us,
            policies_evaluated: result.evaluation.matches.len(),
            policies_matched: matched.len(),
            matched_policy_names: matched.iter().map(|m| m.policy_name.clone()).collect(),
            reason: result.reason.clone(),
            context: HashMap::new(),
        }
    }
}

/// Audit log storage
pub struct AuditLog {
    /// In-memory storage (in production, would be persisted)
    entries: RwLock<Vec<AuditEntry>>,
    
    /// Maximum entries to keep
    max_entries: usize,
}

impl AuditLog {
    /// Create a new audit log
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            max_entries,
        }
    }
    
    /// Create with default size
    pub fn default_size() -> Self {
        Self::new(10000)
    }
    
    /// Add an entry
    pub fn add(&self, entry: AuditEntry) {
        let mut entries = self.entries.write();
        
        // Trim if needed
        if entries.len() >= self.max_entries {
            let remove_count = entries.len() - self.max_entries + 1;
            entries.drain(0..remove_count);
        }
        
        entries.push(entry);
    }
    
    /// Get all entries
    pub fn all(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }
    
    /// Get recent entries
    pub fn recent(&self, count: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        let start = entries.len().saturating_sub(count);
        entries[start..].to_vec()
    }
    
    /// Find entries by subject
    pub fn by_subject(&self, subject_id: &str) -> Vec<AuditEntry> {
        self.entries.read()
            .iter()
            .filter(|e| e.subject_id == subject_id)
            .cloned()
            .collect()
    }
    
    /// Find entries by resource
    pub fn by_resource(&self, resource_id: &str) -> Vec<AuditEntry> {
        self.entries.read()
            .iter()
            .filter(|e| e.resource_id == resource_id)
            .cloned()
            .collect()
    }
    
    /// Find entries by decision
    pub fn by_decision(&self, decision: Decision) -> Vec<AuditEntry> {
        self.entries.read()
            .iter()
            .filter(|e| e.decision == decision)
            .cloned()
            .collect()
    }
    
    /// Get statistics
    pub fn stats(&self) -> AuditStats {
        let entries = self.entries.read();
        let total = entries.len();
        
        let mut permits = 0;
        let mut denies = 0;
        let mut not_applicable = 0;
        let mut indeterminate = 0;
        let mut total_duration_us = 0u64;
        
        for entry in entries.iter() {
            match entry.decision {
                Decision::Permit => permits += 1,
                Decision::Deny => denies += 1,
                Decision::NotApplicable => not_applicable += 1,
                Decision::Indeterminate => indeterminate += 1,
            }
            total_duration_us += entry.duration_us;
        }
        
        AuditStats {
            total,
            permits,
            denies,
            not_applicable,
            indeterminate,
            avg_duration_us: if total > 0 { total_duration_us / total as u64 } else { 0 },
        }
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        self.entries.write().clear();
    }
    
    /// Get count
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::default_size()
    }
}

/// Audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total: usize,
    pub permits: usize,
    pub denies: usize,
    pub not_applicable: usize,
    pub indeterminate: usize,
    pub avg_duration_us: u64,
}

/// Decision engine for ABAC
pub struct DecisionEngine {
    /// Policy evaluator
    evaluator: PolicyEvaluator,
    
    /// Audit log
    audit_log: AuditLog,
    
    /// Whether to log all decisions
    audit_enabled: bool,
    
    /// Default decision when no policy matches
    default_decision: Decision,
}

impl DecisionEngine {
    /// Create a new decision engine
    pub fn new(evaluator: PolicyEvaluator) -> Self {
        Self {
            evaluator,
            audit_log: AuditLog::default(),
            audit_enabled: true,
            default_decision: Decision::Deny,
        }
    }
    
    /// Create with custom audit log size
    pub fn with_audit_size(evaluator: PolicyEvaluator, max_entries: usize) -> Self {
        Self {
            evaluator,
            audit_log: AuditLog::new(max_entries),
            audit_enabled: true,
            default_decision: Decision::Deny,
        }
    }
    
    /// Set audit enabled
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit_enabled = enabled;
        self
    }
    
    /// Set default decision
    pub fn with_default_decision(mut self, decision: Decision) -> Self {
        self.default_decision = decision;
        self
    }
    
    /// Get the evaluator
    pub fn evaluator(&self) -> &PolicyEvaluator {
        &self.evaluator
    }
    
    /// Get mutable evaluator
    pub fn evaluator_mut(&mut self) -> &mut PolicyEvaluator {
        &mut self.evaluator
    }
    
    /// Get the audit log
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit_log
    }
    
    /// Make an authorization decision
    pub fn decide(&self, request: &AuthzRequest) -> DecisionResult {
        let evaluation = self.evaluator.evaluate(request);
        
        let decision = if !evaluation.any_matched {
            self.default_decision
        } else {
            Decision::from_effect(evaluation.effect)
        };
        
        // Determine reason
        let reason = self.determine_reason(&evaluation, decision);
        
        let result = DecisionResult {
            id: Uuid::new_v4(),
            decision,
            request_id: request.id,
            subject_id: request.subject_id.clone(),
            resource_id: request.resource_id.clone(),
            resource_type: request.resource_type.clone(),
            action: request.action.clone(),
            evaluation,
            timestamp: Utc::now(),
            reason,
            obligations: Vec::new(),
            advice: Vec::new(),
        };
        
        // Log to audit
        if self.audit_enabled {
            let entry = AuditEntry::from_result(&result);
            self.audit_log.add(entry);
        }
        
        result
    }
    
    /// Determine the reason for a decision
    fn determine_reason(&self, evaluation: &EvaluationResult, decision: Decision) -> DecisionReason {
        if !evaluation.any_matched {
            return DecisionReason::NoApplicablePolicy;
        }
        
        let matched = evaluation.matched_policies();
        
        match decision {
            Decision::Permit => {
                // Find the highest priority permit
                matched.iter()
                    .find(|m| m.effect == Effect::Allow)
                    .map(|m| DecisionReason::PolicyPermit {
                        policy_id: m.policy_id,
                        policy_name: m.policy_name.clone(),
                    })
                    .unwrap_or(DecisionReason::DenyByDefault)
            }
            Decision::Deny => {
                // Check if there was an explicit deny
                if let Some(deny_match) = matched.iter().find(|m| m.effect == Effect::Deny) {
                    DecisionReason::PolicyDeny {
                        policy_id: deny_match.policy_id,
                        policy_name: deny_match.policy_name.clone(),
                    }
                } else if evaluation.any_matched {
                    // Policies matched but none permitted
                    DecisionReason::DenyByDefault
                } else {
                    DecisionReason::NoApplicablePolicy
                }
            }
            Decision::NotApplicable => DecisionReason::NoApplicablePolicy,
            Decision::Indeterminate => DecisionReason::EvaluationError("Indeterminate result".to_string()),
        }
    }
    
    /// Check if access is permitted
    pub fn is_permitted(&self, request: &AuthzRequest) -> bool {
        self.decide(request).is_permitted()
    }
    
    /// Check if access is denied
    pub fn is_denied(&self, request: &AuthzRequest) -> bool {
        self.decide(request).is_denied()
    }
    
    /// Get audit statistics
    pub fn audit_stats(&self) -> AuditStats {
        self.audit_log.stats()
    }
}

/// Decision engine builder
pub struct DecisionEngineBuilder {
    evaluator: PolicyEvaluator,
    audit_size: usize,
    audit_enabled: bool,
    default_decision: Decision,
}

impl DecisionEngineBuilder {
    /// Create a new builder
    pub fn new(evaluator: PolicyEvaluator) -> Self {
        Self {
            evaluator,
            audit_size: 10000,
            audit_enabled: true,
            default_decision: Decision::Deny,
        }
    }
    
    /// Set audit log size
    pub fn with_audit_size(mut self, size: usize) -> Self {
        self.audit_size = size;
        self
    }
    
    /// Set audit enabled
    pub fn with_audit(mut self, enabled: bool) -> Self {
        self.audit_enabled = enabled;
        self
    }
    
    /// Set default decision
    pub fn with_default_decision(mut self, decision: Decision) -> Self {
        self.default_decision = decision;
        self
    }
    
    /// Build the engine
    pub fn build(self) -> DecisionEngine {
        DecisionEngine {
            evaluator: self.evaluator,
            audit_log: AuditLog::new(self.audit_size),
            audit_enabled: self.audit_enabled,
            default_decision: self.default_decision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abac::policy::{PolicySet, Policy, Rule, Condition};
    use crate::abac::attribute::{AttributeCategory, AttributeValue};
    
    fn create_test_engine() -> DecisionEngine {
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
        
        let evaluator = PolicyEvaluator::new(policy_set);
        DecisionEngine::new(evaluator)
    }
    
    #[test]
    fn test_decision() {
        assert!(Decision::Permit.is_permit());
        assert!(!Decision::Permit.is_deny());
        assert!(Decision::Deny.is_deny());
        assert!(Decision::NotApplicable.is_not_applicable());
        assert!(Decision::Indeterminate.is_indeterminate());
    }
    
    #[test]
    fn test_decision_from_effect() {
        assert_eq!(Decision::from_effect(Effect::Allow), Decision::Permit);
        assert_eq!(Decision::from_effect(Effect::Deny), Decision::Deny);
    }
    
    #[test]
    fn test_decision_engine_permit() {
        let engine = create_test_engine();
        
        let request = AuthzRequest::new("admin1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"));
        
        let result = engine.decide(&request);
        
        assert!(result.is_permitted());
        assert_eq!(result.decision, Decision::Permit);
    }
    
    #[test]
    fn test_decision_engine_deny() {
        let engine = create_test_engine();
        
        let request = AuthzRequest::new("guest1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("guest"));
        
        let result = engine.decide(&request);
        
        assert!(result.is_denied());
        assert_eq!(result.decision, Decision::Deny);
    }
    
    #[test]
    fn test_decision_engine_no_match() {
        let engine = create_test_engine();
        
        let request = AuthzRequest::new("user1", "doc456", "document", "read")
            .with_subject_attr("role", AttributeValue::string("user"));
        
        let result = engine.decide(&request);
        
        assert!(result.is_denied());
        assert!(matches!(result.reason, DecisionReason::NoApplicablePolicy));
    }
    
    #[test]
    fn test_audit_log() {
        let engine = create_test_engine();
        
        // Make some decisions
        let r1 = AuthzRequest::new("admin1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"));
        let r2 = AuthzRequest::new("guest1", "doc2", "document", "write")
            .with_subject_attr("role", AttributeValue::string("guest"));
        
        engine.decide(&r1);
        engine.decide(&r2);
        
        let stats = engine.audit_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.permits, 1);
        assert_eq!(stats.denies, 1);
    }
    
    #[test]
    fn test_audit_log_query() {
        let engine = create_test_engine();
        
        // Make decisions for different subjects
        let r1 = AuthzRequest::new("admin1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"));
        let r2 = AuthzRequest::new("admin1", "doc2", "document", "write")
            .with_subject_attr("role", AttributeValue::string("admin"));
        let r3 = AuthzRequest::new("guest1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("guest"));
        
        engine.decide(&r1);
        engine.decide(&r2);
        engine.decide(&r3);
        
        // Query by subject
        let admin_entries = engine.audit_log().by_subject("admin1");
        assert_eq!(admin_entries.len(), 2);
        
        let guest_entries = engine.audit_log().by_subject("guest1");
        assert_eq!(guest_entries.len(), 1);
        
        // Query by resource
        let doc1_entries = engine.audit_log().by_resource("doc1");
        assert_eq!(doc1_entries.len(), 2);
    }
    
    #[test]
    fn test_obligation() {
        let ob = Obligation::new("ob1", "notify")
            .with_param("email", AttributeValue::string("admin@example.com"));
        
        assert_eq!(ob.id, "ob1");
        assert_eq!(ob.obligation_type, "notify");
        assert!(ob.params.contains_key("email"));
    }
    
    #[test]
    fn test_advice() {
        let adv = Advice::new("adv1", "warning", "Access logged")
            .with_data("sensitivity", AttributeValue::string("high"));
        
        assert_eq!(adv.id, "adv1");
        assert_eq!(adv.advice_type, "warning");
        assert_eq!(adv.message, "Access logged");
    }
    
    #[test]
    fn test_decision_engine_builder() {
        let policy_set = PolicySet::new()
            .add(Policy::allow("test")
                .with_rule(Rule::single(
                    "always",
                    Condition::exists(AttributeCategory::Subject, "id"),
                )));
        
        let evaluator = PolicyEvaluator::new(policy_set);
        
        let engine = DecisionEngineBuilder::new(evaluator)
            .with_audit_size(100)
            .with_audit(true)
            .with_default_decision(Decision::Permit)
            .build();
        
        let request = AuthzRequest::new("user1", "doc1", "document", "read");
        let result = engine.decide(&request);
        
        assert!(result.is_permitted());
    }
    
    #[test]
    fn test_decision_reason() {
        let engine = create_test_engine();
        
        // Test permit reason
        let r = AuthzRequest::new("admin1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"));
        let result = engine.decide(&r);
        
        match result.reason {
            DecisionReason::PolicyPermit { policy_name, .. } => {
                assert_eq!(policy_name, "allow_admin");
            }
            _ => panic!("Expected PolicyPermit reason"),
        }
        
        // Test deny reason
        let r = AuthzRequest::new("guest1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("guest"));
        let result = engine.decide(&r);
        
        match result.reason {
            DecisionReason::PolicyDeny { policy_name, .. } => {
                assert_eq!(policy_name, "deny_guest");
            }
            _ => panic!("Expected PolicyDeny reason"),
        }
    }
}