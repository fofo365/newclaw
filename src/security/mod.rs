// Security Layer - v0.2.0
// Provides authentication, authorization, audit logging, and rate limiting

pub mod api_key;
pub mod jwt;
pub mod rbac;
pub mod audit;
pub mod rate_limit;

// v0.5.1 - Security enhancements
pub mod ssrf;
pub mod injection;

pub use api_key::ApiKeyAuth;
pub use jwt::JwtAuth;
pub use rbac::{Permission, RbacManager, Role};
pub use audit::{AuditEntry, AuditLogger, AuditStorage, AuditResult};
pub use rate_limit::RateLimiter;

// v0.5.1 - SSRF and Injection protection
pub use ssrf::{SsrfGuard, SsrfConfig};
pub use injection::{PromptInjectionDetector, Threat, ThreatType};

use serde::{Deserialize, Serialize};

/// Agent identifier type
pub type AgentId = String;

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub api_key_enabled: bool,
    pub jwt_enabled: bool,
    pub jwt_secret: String,
    pub rbac_enabled: bool,
    pub audit_enabled: bool,
    pub audit_storage: AuditStorageConfig,
    pub rate_limit_enabled: bool,
    pub rate_limit_max_requests: u32,
    pub rate_limit_window_secs: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            api_key_enabled: true,
            jwt_enabled: true,
            jwt_secret: "change-me-in-production".to_string(),
            rbac_enabled: true,
            audit_enabled: true,
            audit_storage: AuditStorageConfig::File("audit.log".to_string()),
            rate_limit_enabled: true,
            rate_limit_max_requests: 100,
            rate_limit_window_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditStorageConfig {
    File(String),
    Database(String),
}
