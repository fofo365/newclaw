// JWT Authentication Module
use super::AgentId;
use anyhow::{anyhow, Result};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (agent ID)
    pub sub: AgentId,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
    /// Issued at (Unix timestamp)
    pub iat: usize,
    /// Issuer
    pub iss: Option<String>,
    /// Custom claims
    pub permissions: Option<Vec<String>>,
    pub role: Option<String>,
}

/// JWT Authentication manager
pub struct JwtAuth {
    secret: Arc<String>,
    issuer: Option<String>,
    default_expiry_secs: i64,
}

impl JwtAuth {
    /// Create a new JWT authentication manager
    pub fn new(secret: String) -> Self {
        Self {
            secret: Arc::new(secret),
            issuer: None,
            default_expiry_secs: 3600, // 1 hour
        }
    }

    /// Create with custom issuer
    pub fn with_issuer(secret: String, issuer: String) -> Self {
        Self {
            secret: Arc::new(secret),
            issuer: Some(issuer),
            default_expiry_secs: 3600,
        }
    }

    /// Set default expiration time
    pub fn with_expiry(mut self, expiry_secs: i64) -> Self {
        self.default_expiry_secs = expiry_secs;
        self
    }

    /// Generate a JWT token for an agent
    pub fn generate(&self, agent_id: &AgentId) -> Result<String> {
        self.generate_with_claims(agent_id, None, None)
    }

    /// Generate a JWT token with custom claims
    pub fn generate_with_claims(
        &self,
        agent_id: &AgentId,
        permissions: Option<Vec<String>>,
        role: Option<String>,
    ) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let exp = (now + self.default_expiry_secs) as usize;

        let claims = Claims {
            sub: agent_id.clone(),
            exp,
            iat: now as usize,
            iss: self.issuer.clone(),
            permissions,
            role,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Generate a JWT token with custom expiration
    pub fn generate_with_expiry(
        &self,
        agent_id: &AgentId,
        expiry_secs: i64,
    ) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let exp = (now + expiry_secs) as usize;

        let claims = Claims {
            sub: agent_id.clone(),
            exp,
            iat: now as usize,
            iss: self.issuer.clone(),
            permissions: None,
            role: None,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Validate a JWT token and extract claims
    pub fn validate(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )?;

        // Check expiration
        let now = chrono::Utc::now().timestamp() as usize;
        if token_data.claims.exp < now {
            return Err(anyhow!("Token has expired"));
        }

        // Check issuer if configured
        if let Some(ref expected_issuer) = self.issuer {
            if token_data.claims.iss.as_ref() != Some(expected_issuer) {
                return Err(anyhow!("Invalid token issuer"));
            }
        }

        Ok(token_data.claims)
    }

    /// Validate and check permission
    pub fn validate_with_permission(&self, token: &str, permission: &str) -> Result<Claims> {
        let claims = self.validate(token)?;

        if let Some(ref permissions) = claims.permissions {
            if !permissions.contains(&permission.to_string())
                && !permissions.contains(&"*".to_string())
            {
                return Err(anyhow!("Permission denied: {}", permission));
            }
        } else {
            return Err(anyhow!("No permissions in token"));
        }

        Ok(claims)
    }

    /// Refresh a token (generate new token with same claims)
    pub fn refresh(&self, token: &str) -> Result<String> {
        let claims = self.validate(token)?;
        
        self.generate_with_claims(&claims.sub, claims.permissions, claims.role)
    }

    /// Extract agent ID from token without full validation
    pub fn extract_agent_id(&self, token: &str) -> Result<AgentId> {
        let claims = self.validate(token)?;
        Ok(claims.sub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate() {
        let auth = JwtAuth::new("test-secret".to_string());
        let token = auth.generate(&"agent-1".to_string()).unwrap();
        
        let claims = auth.validate(&token).unwrap();
        assert_eq!(claims.sub, "agent-1");
    }

    #[test]
    fn test_invalid_token() {
        let auth = JwtAuth::new("test-secret".to_string());
        let result = auth.validate("invalid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        let auth = JwtAuth::new("test-secret".to_string());
        let token = auth.generate_with_expiry(&"agent-1".to_string(), -1).unwrap();
        
        let result = auth.validate(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_permissions() {
        let auth = JwtAuth::new("test-secret".to_string());
        let token = auth
            .generate_with_claims(
                &"agent-1".to_string(),
                Some(vec!["read".to_string(), "write".to_string()]),
                Some("user".to_string()),
            )
            .unwrap();

        let claims = auth.validate(&token).unwrap();
        assert!(claims.permissions.unwrap().contains(&"read".to_string()));
        assert_eq!(claims.role, Some("user".to_string()));
    }

    #[test]
    fn test_permission_check() {
        let auth = JwtAuth::new("test-secret".to_string());
        let token = auth
            .generate_with_claims(
                &"agent-1".to_string(),
                Some(vec!["read".to_string()]),
                None,
            )
            .unwrap();

        assert!(auth.validate_with_permission(&token, "read").is_ok());
        assert!(auth.validate_with_permission(&token, "write").is_err());
    }

    #[test]
    fn test_refresh() {
        let auth = JwtAuth::new("test-secret".to_string());
        let token = auth.generate(&"agent-1".to_string()).unwrap();
        
        let new_token = auth.refresh(&token).unwrap();
        let claims = auth.validate(&new_token).unwrap();
        assert_eq!(claims.sub, "agent-1");
    }
}
