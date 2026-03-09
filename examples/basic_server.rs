// Example: Basic HTTP Server with Security
// Demonstrates how to set up a NewClaw server with authentication

use newclaw::{
    communication::HttpServer,
    security::{ApiKeyAuth, JwtAuth, RbacManager, AuditLogger},
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting NewClaw Basic Server...\n");

    // 1. Setup API Key Authentication
    println!("✓ Configuring API Key authentication...");
    let api_keys = vec![newclaw::security::api_key::ApiKey {
        key: "demo-api-key-12345".to_string(),
        name: "Demo Key".to_string(),
        scopes: vec!["read".to_string(), "write".to_string()],
        expires_at: None,
    }];
    let api_config = newclaw::security::api_key::ApiKeyConfig { keys: api_keys };
    let _api_auth = ApiKeyAuth::new(api_config);

    // 2. Setup JWT Authentication
    println!("✓ Configuring JWT authentication...");
    let jwt_config = newclaw::security::jwt::JwtConfig {
        secret: "demo-secret-key-change-in-production".to_string(),
        issuer: "newclaw-demo".to_string(),
        expiry_hours: 24,
    };
    let _jwt_auth = JwtAuth::new(jwt_config);

    // 3. Setup RBAC
    println!("✓ Configuring RBAC...");
    let _rbac = RbacManager::new();

    // 4. Setup Audit Logging
    println!("✓ Configuring audit logging...");
    let audit_config = newclaw::security::audit::AuditConfig {
        log_path: "./logs/audit.log".to_string(),
        max_entries: 10000,
    };
    let _audit = AuditLogger::new(audit_config);

    // 5. Start HTTP Server
    println!("\n📡 Starting HTTP server on port 8080...");
    println!("   API Endpoints:");
    println!("   - GET  /api/v1/health      - Health check");
    println!("   - GET  /api/v1/agents      - List agents");
    println!("   - POST /api/v1/messages    - Send message");
    println!("\n   Authentication:");
    println!("   - API Key: X-API-Key header");
    println!("   - JWT: Authorization: Bearer <token>");
    println!("\n⚡ Server ready!\n");

    // In a real application, you would start the server here
    // let server = HttpServer::new(8080);
    // server.start().await?;

    // For demo purposes, just keep running
    tokio::signal::ctrl_c().await?;
    println!("\n\n👋 Shutting down gracefully...");
    
    Ok(())
}
