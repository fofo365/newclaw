// Security Configuration
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub api_key_enabled: bool,
    pub jwt_enabled: bool,
    pub jwt_secret: String,
    pub jwt_expiry_secs: i64,
    pub rbac_enabled: bool,
    pub default_role: String,
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
            jwt_expiry_secs: 3600,
            rbac_enabled: true,
            default_role: "user".to_string(),
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
