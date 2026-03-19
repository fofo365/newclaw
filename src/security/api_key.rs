// API Key Authentication Module
use super::AgentId;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// API Key authentication manager
pub struct ApiKeyAuth {
    keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub key: String,
    pub agent_id: AgentId,
    pub permissions: Vec<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub is_active: bool,
}

impl ApiKeyAuth {
    /// Create a new API Key authentication manager
    pub fn new() -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a new API key for an agent
    pub async fn generate(&self, agent_id: AgentId, permissions: Vec<String>) -> String {
        let key = format!("sk_{}", Uuid::new_v4().to_string().replace("-", ""));
        let info = ApiKeyInfo {
            key: key.clone(),
            agent_id,
            permissions,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
            is_active: true,
        };
        
        self.keys.write().await.insert(key.clone(), info);
        key
    }

    /// Generate a new API key with expiration
    pub async fn generate_with_expiry(
        &self,
        agent_id: AgentId,
        permissions: Vec<String>,
        expires_in_secs: i64,
    ) -> String {
        let key = format!("sk_{}", Uuid::new_v4().to_string().replace("-", ""));
        let info = ApiKeyInfo {
            key: key.clone(),
            agent_id,
            permissions,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: Some(chrono::Utc::now().timestamp() + expires_in_secs),
            is_active: true,
        };
        
        self.keys.write().await.insert(key.clone(), info);
        key
    }

    /// Validate an API key
    pub async fn validate(&self, key: &str) -> Result<ApiKeyInfo> {
        let keys = self.keys.read().await;
        let info = keys
            .get(key)
            .ok_or_else(|| anyhow!("Invalid API key"))?
            .clone();

        // Check if key is active
        if !info.is_active {
            return Err(anyhow!("API key is inactive"));
        }

        // Check expiration
        if let Some(expires_at) = info.expires_at {
            if chrono::Utc::now().timestamp() > expires_at {
                return Err(anyhow!("API key has expired"));
            }
        }

        Ok(info)
    }

    /// Revoke an API key
    pub async fn revoke(&self, key: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        if let Some(info) = keys.get_mut(key) {
            info.is_active = false;
            Ok(())
        } else {
            Err(anyhow!("API key not found"))
        }
    }

    /// Delete an API key
    pub async fn delete(&self, key: &str) -> Result<()> {
        let mut keys = self.keys.write().await;
        if keys.remove(key).is_some() {
            Ok(())
        } else {
            Err(anyhow!("API key not found"))
        }
    }

    /// List all API keys for an agent
    pub async fn list_for_agent(&self, agent_id: &AgentId) -> Vec<ApiKeyInfo> {
        let keys = self.keys.read().await;
        keys.values()
            .filter(|info| info.agent_id == *agent_id)
            .cloned()
            .collect()
    }

    /// Check if a key has a specific permission
    pub async fn has_permission(&self, key: &str, permission: &str) -> bool {
        if let Ok(info) = self.validate(key).await {
            info.permissions.contains(&permission.to_string())
                || info.permissions.contains(&"*".to_string())
        } else {
            false
        }
    }
}

impl Default for ApiKeyAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_and_validate() {
        let auth = ApiKeyAuth::new();
        let key = auth.generate("agent-1".to_string(), vec!["read".to_string()]).await;
        
        let info = auth.validate(&key).await.unwrap();
        assert_eq!(info.agent_id, "agent-1");
        assert!(info.permissions.contains(&"read".to_string()));
    }

    #[tokio::test]
    async fn test_invalid_key() {
        let auth = ApiKeyAuth::new();
        let result = auth.validate("invalid-key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_revoke_key() {
        let auth = ApiKeyAuth::new();
        let key = auth.generate("agent-1".to_string(), vec!["read".to_string()]).await;
        
        auth.revoke(&key).await.unwrap();
        let result = auth.validate(&key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_expiration() {
        let auth = ApiKeyAuth::new();
        let key = auth
            .generate_with_expiry("agent-1".to_string(), vec!["read".to_string()], -1)
            .await;
        
        let result = auth.validate(&key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_has_permission() {
        let auth = ApiKeyAuth::new();
        let key = auth.generate("agent-1".to_string(), vec!["read".to_string(), "write".to_string()]).await;
        
        assert!(auth.has_permission(&key, "read").await);
        assert!(auth.has_permission(&key, "write").await);
        assert!(!auth.has_permission(&key, "admin").await);
    }
}
