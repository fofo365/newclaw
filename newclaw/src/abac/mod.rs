//! ABAC (Attribute-Based Access Control) Module
//!
//! This module implements a complete ABAC authorization system for NewClaw v0.7.0.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Decision Engine                          │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                  DecisionResult                         ││
//! │  │  - Decision (Permit/Deny/NotApplicable/Indeterminate)  ││
//! │  │  - Reason                                               ││
//! │  │  - Obligations & Advice                                 ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │                            ▲                                 │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                  Policy Evaluator                       ││
//! │  │  - PolicyMatcher                                        ││
//! │  │  - EvaluationResult                                     ││
//! │  │  - Combining Algorithms                                 ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │                            ▲                                 │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                  Policy Set                             ││
//! │  │  - Policies with Rules                                  ││
//! │  │  - Conditions & Effects                                 ││
//! │  │  - Priority & Targets                                   ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │                            ▲                                 │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                  Attribute Bag                          ││
//! │  │  - Subject Attributes (user, role, dept, etc.)         ││
//! │  │  - Resource Attributes (type, owner, class, etc.)      ││
//! │  │  - Action Attributes (operation, method, etc.)         ││
//! │  │  - Environment Attributes (time, location, etc.)       ││
//! │  └─────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use newclaw::abac::*;
//!
//! // Create attributes
//! let attrs = AttributeBag::new()
//!     .with_subject("role", AttributeValue::string("admin"))
//!     .with_resource("type", AttributeValue::string("document"));
//!
//! // Define policies
//! let policy = Policy::allow("admin_access")
//!     .with_priority(100)
//!     .with_rule(Rule::single(
//!         "is_admin",
//!         Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
//!     ));
//!
//! // Create policy set
//! let policy_set = PolicySet::new().add(policy);
//!
//! // Create decision engine
//! let evaluator = PolicyEvaluator::new(policy_set);
//! let engine = DecisionEngine::new(evaluator);
//!
//! // Make authorization decision
//! let request = AuthzRequest::new("user123", "doc456", "document", "read")
//!     .with_subject_attr("role", AttributeValue::string("admin"));
//!
//! let result = engine.decide(&request);
//! assert!(result.is_permitted());
//! ```
//!
//! ## Features
//!
//! - **Attribute-based**: Fine-grained access control based on subject, resource, action, and environment attributes
//! - **Flexible conditions**: Support for various operators (equals, in, contains, regex, etc.)
//! - **Policy combining**: Multiple algorithms (deny-overrides, allow-overrides, first-applicable)
//! - **Priority-based**: Higher priority policies take precedence
//! - **Audit logging**: Complete audit trail of all authorization decisions
//! - **Default deny**: Secure by default when no policy matches
//!
//! ## Integration Points
//!
//! - **Configuration System**: Load policies from 6-layer config
//! - **Task System**: Authorization checks for task execution
//! - **Tool System**: Permission checks for tool calls
//! - **Channel System**: Channel-specific authorization rules

pub mod attribute;
pub mod policy;
pub mod evaluator;
pub mod decision;

// Re-exports for convenience
pub use attribute::{
    Attribute, AttributeBag, AttributeCategory, AttributeError, AttributeResolver,
    AttributeValue, AttributeValueType, CompositeAttributeResolver, StaticAttributeResolver,
};

pub use policy::{
    Condition, ConditionOperator, Effect, LogicalOperator, Policy, PolicyCombiningAlgorithm,
    PolicyError, PolicySet, Rule,
};

pub use evaluator::{
    AuthzRequest, EvaluationResult, PolicyEvaluator, PolicyEvaluatorBuilder, PolicyMatch,
    PolicyMatcher,
};

pub use decision::{
    Advice, AuditEntry, AuditLog, AuditStats, Decision, DecisionEngine, DecisionEngineBuilder,
    DecisionReason, DecisionResult, Obligation,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test for complete ABAC flow
    #[test]
    fn test_complete_abac_flow() {
        // Step 1: Create attributes
        let attrs = AttributeBag::new()
            .with_subject("user_id", AttributeValue::string("admin123"))
            .with_subject("role", AttributeValue::string("admin"))
            .with_subject("department", AttributeValue::string("engineering"))
            .with_resource("resource_id", AttributeValue::string("doc456"))
            .with_resource("type", AttributeValue::string("document"))
            .with_resource("classification", AttributeValue::string("confidential"))
            .with_environment("hour", AttributeValue::number(10.0))
            .with_environment("ip", AttributeValue::string("192.168.1.100"));

        // Step 2: Define policies
        let policy_set = PolicySet::new()
            // Deny guests access to everything
            .add(Policy::deny("deny_guests")
                .with_priority(1000)
                .with_rule(Rule::single(
                    "is_guest",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("guest")),
                )))
            // Allow admins full access during business hours
            .add(Policy::allow("admin_business_hours")
                .with_priority(100)
                .with_rule(Rule::and("admin_and_hours", vec![
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                    Condition::new(
                        AttributeCategory::Environment,
                        "hour",
                        ConditionOperator::GreaterThanOrEqual,
                        AttributeValue::number(9.0),
                    ),
                    Condition::new(
                        AttributeCategory::Environment,
                        "hour",
                        ConditionOperator::LessThanOrEqual,
                        AttributeValue::number(17.0),
                    ),
                ])))
            // Allow owners access to their own resources
            .add(Policy::allow("owner_access")
                .with_priority(50)
                .with_rule(Rule::single(
                    "is_owner",
                    Condition::equals(AttributeCategory::Resource, "owner", AttributeValue::string("admin123")),
                )));

        // Step 3: Create evaluator and engine
        let evaluator = PolicyEvaluator::new(policy_set);
        let engine = DecisionEngine::new(evaluator);

        // Step 4: Create authorization request
        let request = AuthzRequest::new("admin123", "doc456", "document", "read")
            .with_attributes(attrs);

        // Step 5: Make decision
        let result = engine.decide(&request);

        // Verify result
        assert!(result.is_permitted(), "Admin should be permitted during business hours");
        assert!(matches!(result.reason, DecisionReason::PolicyPermit { .. }));

        // Check audit log
        let stats = engine.audit_stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.permits, 1);
    }

    /// Test deny override behavior
    #[test]
    fn test_deny_override() {
        let policy_set = PolicySet::new()
            // This deny should take precedence
            .add(Policy::deny("deny_confidential")
                .with_priority(100)
                .with_rule(Rule::single(
                    "is_confidential",
                    Condition::equals(AttributeCategory::Resource, "classification", AttributeValue::string("confidential")),
                )))
            // This allow would match but deny overrides
            .add(Policy::allow("allow_admin")
                .with_priority(50)
                .with_rule(Rule::single(
                    "is_admin",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                )));

        let evaluator = PolicyEvaluator::new(policy_set);
        let engine = DecisionEngine::new(evaluator);

        let request = AuthzRequest::new("admin1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("admin"))
            .with_resource_attr("classification", AttributeValue::string("confidential"));

        let result = engine.decide(&request);

        // Even though admin, access to confidential is denied
        assert!(result.is_denied());
    }

    /// Test multi-condition rules
    #[test]
    fn test_multi_condition_rules() {
        let policy_set = PolicySet::new()
            .add(Policy::allow("engineering_access")
                .with_rule(Rule::and("eng_and_active", vec![
                    // Must be in engineering
                    Condition::equals(AttributeCategory::Subject, "department", AttributeValue::string("engineering")),
                    // Must be active
                    Condition::equals(AttributeCategory::Subject, "status", AttributeValue::string("active")),
                    // Must be during work hours
                    Condition::new(
                        AttributeCategory::Environment,
                        "hour",
                        ConditionOperator::GreaterThanOrEqual,
                        AttributeValue::number(9.0),
                    ),
                ])));

        let evaluator = PolicyEvaluator::new(policy_set);
        let engine = DecisionEngine::new(evaluator);

        // Test with all conditions met
        let request = AuthzRequest::new("eng1", "doc1", "document", "read")
            .with_subject_attr("department", AttributeValue::string("engineering"))
            .with_subject_attr("status", AttributeValue::string("active"))
            .with_env_attr("hour", AttributeValue::number(10.0));

        let result = engine.decide(&request);
        assert!(result.is_permitted());

        // Test with missing status
        let request = AuthzRequest::new("eng2", "doc1", "document", "read")
            .with_subject_attr("department", AttributeValue::string("engineering"))
            .with_env_attr("hour", AttributeValue::number(10.0));

        let result = engine.decide(&request);
        assert!(result.is_denied());
    }

    /// Test role hierarchy simulation
    #[test]
    fn test_role_hierarchy() {
        // Simulate role hierarchy: admin > editor > viewer
        let policy_set = PolicySet::new()
            // Admin can do anything
            .add(Policy::allow("admin_all")
                .with_priority(100)
                .with_target_resource("*")
                .with_target_action("*")
                .with_rule(Rule::single(
                    "is_admin",
                    Condition::in_list(
                        AttributeCategory::Subject,
                        "role",
                        vec![
                            AttributeValue::string("admin"),
                            AttributeValue::string("superuser"),
                        ],
                    ),
                )))
            // Editor can read and write
            .add(Policy::allow("editor_rw")
                .with_priority(50)
                .with_target_resource("document")
                .with_target_action("read")
                .with_target_action("write")
                .with_rule(Rule::single(
                    "is_editor",
                    Condition::in_list(
                        AttributeCategory::Subject,
                        "role",
                        vec![
                            AttributeValue::string("editor"),
                            AttributeValue::string("admin"), // Admin inherits editor permissions
                        ],
                    ),
                )))
            // Viewer can only read
            .add(Policy::allow("viewer_read")
                .with_priority(10)
                .with_target_resource("document")
                .with_target_action("read")
                .with_rule(Rule::or("any_viewer", vec![
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("viewer")),
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("editor")),
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                ])));

        let evaluator = PolicyEvaluator::new(policy_set);
        let engine = DecisionEngine::new(evaluator);

        // Admin can delete (only admin policy applies)
        let request = AuthzRequest::new("admin1", "doc1", "document", "delete")
            .with_subject_attr("role", AttributeValue::string("admin"));
        assert!(engine.is_permitted(&request));

        // Editor can write
        let request = AuthzRequest::new("editor1", "doc1", "document", "write")
            .with_subject_attr("role", AttributeValue::string("editor"));
        assert!(engine.is_permitted(&request));

        // Editor cannot delete
        let request = AuthzRequest::new("editor1", "doc1", "document", "delete")
            .with_subject_attr("role", AttributeValue::string("editor"));
        assert!(engine.is_denied(&request));

        // Viewer can read
        let request = AuthzRequest::new("viewer1", "doc1", "document", "read")
            .with_subject_attr("role", AttributeValue::string("viewer"));
        assert!(engine.is_permitted(&request));

        // Viewer cannot write
        let request = AuthzRequest::new("viewer1", "doc1", "document", "write")
            .with_subject_attr("role", AttributeValue::string("viewer"));
        assert!(engine.is_denied(&request));
    }

    /// Test set-based conditions
    #[test]
    fn test_set_conditions() {
        let policy_set = PolicySet::new()
            .add(Policy::allow("multi_role_access")
                .with_rule(Rule::single(
                    "has_required_role",
                    Condition::new(
                        AttributeCategory::Subject,
                        "roles",
                        ConditionOperator::AnyOf,
                        AttributeValue::list(vec![
                            AttributeValue::string("admin"),
                            AttributeValue::string("editor"),
                        ]),
                    ),
                )));

        let evaluator = PolicyEvaluator::new(policy_set);
        let engine = DecisionEngine::new(evaluator);

        // User with multiple roles including admin
        let request = AuthzRequest::new("user1", "doc1", "document", "read")
            .with_subject_attr("roles", AttributeValue::set(vec!["user", "editor"]));
        assert!(engine.is_permitted(&request));

        // User without required role
        let request = AuthzRequest::new("user2", "doc1", "document", "read")
            .with_subject_attr("roles", AttributeValue::set(vec!["user", "guest"]));
        assert!(engine.is_denied(&request));
    }

    /// Test regex matching
    #[test]
    fn test_regex_matching() {
        let policy_set = PolicySet::new()
            .add(Policy::allow("internal_access")
                .with_rule(Rule::single(
                    "internal_ip",
                    Condition::new(
                        AttributeCategory::Environment,
                        "ip",
                        ConditionOperator::Matches,
                        AttributeValue::string(r"^192\.168\."),
                    ),
                )));

        let evaluator = PolicyEvaluator::new(policy_set);
        let engine = DecisionEngine::new(evaluator);

        // Internal IP
        let request = AuthzRequest::new("user1", "doc1", "document", "read")
            .with_env_attr("ip", AttributeValue::string("192.168.1.100"));
        assert!(engine.is_permitted(&request));

        // External IP
        let request = AuthzRequest::new("user2", "doc1", "document", "read")
            .with_env_attr("ip", AttributeValue::string("8.8.8.8"));
        assert!(engine.is_denied(&request));
    }

    /// Test attribute resolver integration
    #[tokio::test]
    async fn test_attribute_resolver_integration() {
        // Create a static resolver with additional attributes
        let resolver = StaticAttributeResolver::new()
            .with_subject("role", AttributeValue::string("admin"))
            .with_subject("department", AttributeValue::string("engineering"));

        let policy_set = PolicySet::new()
            .add(Policy::allow("admin_access")
                .with_rule(Rule::single(
                    "is_admin",
                    Condition::equals(AttributeCategory::Subject, "role", AttributeValue::string("admin")),
                )));

        let evaluator = PolicyEvaluator::new(policy_set);

        // Request without role attribute
        let request = AuthzRequest::new("admin1", "doc1", "document", "read");

        // Evaluate with resolver
        let result = evaluator.evaluate_with_resolver(&request, &resolver).await.unwrap();
        assert_eq!(result.effect, Effect::Allow);
    }
}