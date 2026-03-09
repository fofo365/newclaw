// Integration tests for NewClaw v0.2.0
// Tests end-to-end functionality across all layers

use newclaw::{
    communication::message::{InterAgentMessage, MessagePayload, Request, Response, Event},
    security::{
        ApiKeyAuth, JwtAuth, RbacManager, AuditLogger,
        rbac::Permission,
    },
    core::context::ContextManager,
    ContextConfig,
};
use std::collections::HashMap;

/// Test API Key authentication flow
#[tokio::test]
async fn test_api_key_authentication() {
    let auth = ApiKeyAuth::new();
    
    // Generate a new API key
    let agent_id = "test-agent".to_string();
    let permissions = vec!["read".to_string(), "write".to_string()];
    let key = auth.generate(agent_id.clone(), permissions).await;
    
    // Validate the generated key
    let result = auth.validate(&key).await;
    assert!(result.is_ok());
    
    let key_info = result.unwrap();
    assert_eq!(key_info.agent_id, agent_id);
    assert!(key_info.permissions.contains(&"read".to_string()));
    
    // Test invalid key
    let result = auth.validate("invalid-key").await;
    assert!(result.is_err());
}

/// Test JWT token generation and validation
#[tokio::test]
async fn test_jwt_workflow() {
    let secret = "test-secret-key-123".to_string();
    let jwt_auth = JwtAuth::new(secret);
    
    // Generate token
    let agent_id = "user-123".to_string();
    let token = jwt_auth.generate(&agent_id).unwrap();
    assert!(!token.is_empty());
    
    // Validate token
    let validated = jwt_auth.validate(&token).unwrap();
    assert_eq!(validated.sub, agent_id);
}

/// Test JWT with custom claims
#[tokio::test]
async fn test_jwt_with_claims() {
    let secret = "test-secret".to_string();
    let jwt_auth = JwtAuth::with_issuer(secret, "newclaw-test".to_string())
        .with_expiry(7200);
    
    let agent_id = "user-456".to_string();
    let permissions = Some(vec!["admin".to_string(), "write".to_string()]);
    let role = Some("admin".to_string());
    
    let token = jwt_auth.generate_with_claims(&agent_id, permissions.clone(), role.clone()).unwrap();
    let validated = jwt_auth.validate(&token).unwrap();
    
    assert_eq!(validated.sub, agent_id);
    assert_eq!(validated.permissions, permissions);
    assert_eq!(validated.role, role);
}

/// Test RBAC permission checking
#[tokio::test]
async fn test_rbac_permissions() {
    let rbac = RbacManager::new();
    
    let agent_id = "agent-1".to_string();
    let permission = Permission::Read;
    
    // Check permission (default: no roles assigned, should be false)
    let has_permission = rbac.check_permission(&agent_id, permission.clone()).await;
    assert!(!has_permission);
    
    // Assign role and check again
    rbac.assign_role(&agent_id, "viewer").await;
    let has_permission = rbac.check_permission(&agent_id, Permission::Read).await;
    assert!(has_permission);
}

/// Test audit logging with memory backend
#[tokio::test]
async fn test_audit_logging() {
    let audit = AuditLogger::memory();
    
    let entry = newclaw::security::audit::AuditEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp_iso: chrono::Utc::now().to_rfc3339(),
        agent_id: "user-123".to_string(),
        result: newclaw::security::audit::AuditResult::Success,
    };
    
    // Log entry should succeed
    let result = audit.log(entry).await;
    assert!(result.is_ok());
}

/// Test context isolation
#[tokio::test]
async fn test_context_isolation() {
    let config = ContextConfig::default();
    let mut ctx_manager = ContextManager::new(config).unwrap();
    
    // Add messages to context
    let id1 = ctx_manager.add_message("Message from agent-1", "agent-1").unwrap();
    let id2 = ctx_manager.add_message("Message from agent-2", "agent-2").unwrap();
    
    // Verify messages were stored
    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    
    // Retrieve relevant context
    let chunks = ctx_manager.retrieve_relevant("agent-1", 10).unwrap();
    assert!(!chunks.is_empty());
}

/// Test inter-agent messaging
#[tokio::test]
async fn test_inter_agent_messaging() {
    let from_agent = "source".to_string();
    let to_agent = "target".to_string();
    
    let msg = InterAgentMessage::request(
        from_agent.clone(),
        to_agent.clone(),
        Request::Query {
            query: "Hello".to_string(),
            context: None,
        },
    );
    
    // Verify message structure
    assert_eq!(msg.from, from_agent);
    assert_eq!(msg.to, to_agent);
    
    // Verify payload
    if let MessagePayload::Request(Request::Query { query, .. }) = msg.payload {
        assert_eq!(query, "Hello");
    } else {
        panic!("Expected Request::Query payload");
    }
}

/// Test security layer integration
#[tokio::test]
async fn test_security_integration() {
    // Setup API Key auth
    let api_auth = ApiKeyAuth::new();
    
    // Setup JWT auth
    let jwt_auth = JwtAuth::new("integration-secret".to_string());
    
    // Setup RBAC
    let rbac = RbacManager::new();
    
    // Test authentication flow
    let agent_id = "integration-agent".to_string();
    let api_key = api_auth.generate(agent_id.clone(), vec!["read".to_string()]).await;
    let key_result = api_auth.validate(&api_key).await;
    assert!(key_result.is_ok());
    
    // Test JWT generation
    let token = jwt_auth.generate(&agent_id).unwrap();
    let validated = jwt_auth.validate(&token).unwrap();
    assert_eq!(validated.sub, agent_id);
    
    // Test RBAC
    let permission = Permission::Read;
    let has_perm = rbac.check_permission(&agent_id, permission).await;
    assert!(!has_perm); // No role assigned yet
    
    rbac.assign_role(&agent_id, "viewer").await;
    let has_perm = rbac.check_permission(&agent_id, Permission::Read).await;
    assert!(has_perm); // Now has permission
}

/// Test API key expiration
#[tokio::test]
async fn test_api_key_expiration() {
    let auth = ApiKeyAuth::new();
    
    let agent_id = "test-agent".to_string();
    
    // Generate key that expires in 1 second
    let key = auth.generate_with_expiry(agent_id.clone(), vec!["read".to_string()], 1).await;
    
    // Should be valid immediately
    let result = auth.validate(&key).await;
    assert!(result.is_ok());
    
    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Should be expired now
    let result = auth.validate(&key).await;
    assert!(result.is_err());
}

/// Test JWT token expiration
#[tokio::test]
async fn test_jwt_expiration() {
    let jwt_auth = JwtAuth::new("test-secret".to_string())
        .with_expiry(1); // 1 second expiry
    
    let agent_id = "user-123".to_string();
    let token = jwt_auth.generate(&agent_id).unwrap();
    
    // Should be valid immediately
    let result = jwt_auth.validate(&token);
    assert!(result.is_ok());
    
    // Wait for expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Should be expired now
    let result = jwt_auth.validate(&token);
    assert!(result.is_err());
}

/// Test RBAC role management
#[tokio::test]
async fn test_rbac_role_management() {
    let rbac = RbacManager::new();
    
    let agent_id = "test-agent".to_string();
    
    // Assign multiple roles
    rbac.assign_role(&agent_id, "viewer").await;
    rbac.assign_role(&agent_id, "editor").await;
    
    // Check permissions from both roles
    let can_read = rbac.check_permission(&agent_id, Permission::Read).await;
    let can_write = rbac.check_permission(&agent_id, Permission::Write).await;
    
    assert!(can_read); // From viewer role
    assert!(can_write); // From editor role
    
    // Revoke role
    rbac.revoke_role(&agent_id, "editor").await;
    
    let can_write_after_revoke = rbac.check_permission(&agent_id, Permission::Write).await;
    assert!(!can_write_after_revoke);
}

/// Test audit log filtering
#[tokio::test]
async fn test_audit_filtering() {
    let audit = AuditLogger::memory();
    
    // Log multiple entries
    for i in 0..5 {
        let entry = newclaw::security::audit::AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp_iso: chrono::Utc::now().to_rfc3339(),
            agent_id: format!("agent-{}", i % 2),
            result: if i % 2 == 0 {
                newclaw::security::audit::AuditResult::Success
            } else {
                newclaw::security::audit::AuditResult::Failure
            },
        };
        audit.log(entry).await.unwrap();
    }
    
    // Query by agent_id
    let entries = audit.query_by_agent("agent-0").await.unwrap();
    assert_eq!(entries.len(), 3); // 0, 2, 4
}

/// Test message metadata
#[tokio::test]
async fn test_message_metadata() {
    let from = "sender".to_string();
    let to = "receiver".to_string();
    
    let msg = InterAgentMessage::request(
        from.clone(),
        to.clone(),
        Request::Query {
            query: "Test".to_string(),
            context: None,
        },
    ).with_metadata("priority".to_string(), "high".to_string())
     .with_metadata("trace_id".to_string(), "abc-123".to_string());
    
    assert_eq!(msg.metadata.get("priority"), Some(&"high".to_string()));
    assert_eq!(msg.metadata.get("trace_id"), Some(&"abc-123".to_string()));
}

/// Test message types
#[tokio::test]
async fn test_message_types() {
    let from = "agent-1".to_string();
    let to = "agent-2".to_string();
    
    // Test request message
    let request_msg = InterAgentMessage::request(
        from.clone(),
        to.clone(),
        Request::Query {
            query: "test query".to_string(),
            context: None,
        },
    );
    assert!(matches!(request_msg.payload, MessagePayload::Request(_)));
    
    // Test response message
    let response_msg = InterAgentMessage::response(
        from.clone(),
        to.clone(),
        Response::success(serde_json::json!({"result": "ok"})),
        "correlation-123".to_string(),
    );
    assert!(matches!(response_msg.payload, MessagePayload::Response(_)));
    assert_eq!(response_msg.correlation_id, Some("correlation-123".to_string()));
    
    // Test event message
    let event_msg = InterAgentMessage::event(
        from.clone(),
        to.clone(),
        Event::AgentStatus {
            agent_id: from.clone(),
            status: "online".to_string(),
        },
    );
    assert!(matches!(event_msg.payload, MessagePayload::Event(_)));
}

/// Test rate limiting (if available)
#[tokio::test]
#[ignore = "Rate limiting requires running server"]
async fn test_rate_limiting() {
    // This test would require a running HTTP server
    // Marked as ignored for CI/CD
}

/// Test WebSocket connection (if available)
#[tokio::test]
#[ignore = "WebSocket test requires running server"]
async fn test_websocket_connection() {
    // This test would require a running WebSocket server
    // Marked as ignored for CI/CD
}
