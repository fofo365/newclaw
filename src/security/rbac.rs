// RBAC (Role-Based Access Control) Module
use super::AgentId;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Permission definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    // Messaging
    SendMessage,
    ReceiveMessage,
    BroadcastMessage,
    
    // Agent management
    RegisterAgent,
    ListAgents,
    GetAgentInfo,
    DeleteAgent,
    
    // System
    Admin,
    All,
    
    // Custom permission
    Custom(String),
}

impl Permission {
    pub fn as_str(&self) -> &str {
        match self {
            Permission::SendMessage => "send_message",
            Permission::ReceiveMessage => "receive_message",
            Permission::BroadcastMessage => "broadcast_message",
            Permission::RegisterAgent => "register_agent",
            Permission::ListAgents => "list_agents",
            Permission::GetAgentInfo => "get_agent_info",
            Permission::DeleteAgent => "delete_agent",
            Permission::Admin => "admin",
            Permission::All => "*",
            Permission::Custom(s) => s,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "send_message" => Permission::SendMessage,
            "receive_message" => Permission::ReceiveMessage,
            "broadcast_message" => Permission::BroadcastMessage,
            "register_agent" => Permission::RegisterAgent,
            "list_agents" => Permission::ListAgents,
            "get_agent_info" => Permission::GetAgentInfo,
            "delete_agent" => Permission::DeleteAgent,
            "admin" => Permission::Admin,
            "*" => Permission::All,
            _ => Permission::Custom(s.to_string()),
        }
    }
}

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub inherits: Option<Vec<String>>,
}

impl Role {
    pub fn new(name: String, permissions: Vec<Permission>) -> Self {
        Self {
            name,
            description: None,
            permissions,
            inherits: None,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn inherits(mut self, roles: Vec<String>) -> Self {
        self.inherits = Some(roles);
        self
    }

    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(&Permission::All) || self.permissions.contains(permission)
    }
}

/// RBAC Manager
pub struct RbacManager {
    roles: Arc<RwLock<HashMap<String, Role>>>,
    user_roles: Arc<RwLock<HashMap<AgentId, Vec<String>>>>,
}

impl RbacManager {
    /// Create a new RBAC manager with default roles
    pub fn new() -> Self {
        let mut manager = Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Add default roles
        manager.add_default_roles();
        manager
    }

    /// Add default roles
    fn add_default_roles(&mut self) {
        // Admin role - full access
        let admin_role = Role::new("admin".to_string(), vec![Permission::All])
            .with_description("Full administrative access".to_string());

        // User role - basic messaging
        let user_role = Role::new(
            "user".to_string(),
            vec![
                Permission::SendMessage,
                Permission::ReceiveMessage,
                Permission::ListAgents,
                Permission::GetAgentInfo,
            ],
        )
        .with_description("Basic user access".to_string());

        // Guest role - read-only
        let guest_role = Role::new(
            "guest".to_string(),
            vec![Permission::ReceiveMessage, Permission::ListAgents],
        )
        .with_description("Read-only guest access".to_string());

        // Add roles using blocking write
        // Always initialize roles for each new instance
        if let Ok(mut roles) = self.roles.try_write() {
            roles.insert("admin".to_string(), admin_role);
            roles.insert("user".to_string(), user_role);
            roles.insert("guest".to_string(), guest_role);
        }
    }

    /// Add a custom role
    pub async fn add_role(&self, role: Role) -> Result<()> {
        let mut roles = self.roles.write().await;
        if roles.contains_key(&role.name) {
            return Err(anyhow!("Role {} already exists", role.name));
        }
        roles.insert(role.name.clone(), role);
        Ok(())
    }

    /// Remove a role
    pub async fn remove_role(&self, role_name: &str) -> Result<()> {
        let mut roles = self.roles.write().await;
        if roles.remove(role_name).is_some() {
            Ok(())
        } else {
            Err(anyhow!("Role {} not found", role_name))
        }
    }

    /// Get a role by name
    pub async fn get_role(&self, role_name: &str) -> Option<Role> {
        let roles = self.roles.read().await;
        roles.get(role_name).cloned()
    }

    /// List all roles
    pub async fn list_roles(&self) -> Vec<Role> {
        let roles = self.roles.read().await;
        roles.values().cloned().collect()
    }

    /// Assign a role to an agent
    pub async fn assign_role(&self, agent_id: AgentId, role_name: String) -> Result<()> {
        // Verify role exists
        {
            let roles = self.roles.read().await;
            if !roles.contains_key(&role_name) {
                return Err(anyhow!("Role {} not found", role_name));
            }
        }

        let mut user_roles = self.user_roles.write().await;
        user_roles
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(role_name);
        
        Ok(())
    }

    /// Remove a role from an agent
    pub async fn revoke_role(&self, agent_id: &AgentId, role_name: &str) -> Result<()> {
        let mut user_roles = self.user_roles.write().await;
        if let Some(roles) = user_roles.get_mut(agent_id) {
            if let Some(pos) = roles.iter().position(|r| r == role_name) {
                roles.remove(pos);
                Ok(())
            } else {
                Err(anyhow!("Agent {} does not have role {}", agent_id, role_name))
            }
        } else {
            Err(anyhow!("Agent {} has no roles", agent_id))
        }
    }

    /// Get all roles for an agent
    pub async fn get_agent_roles(&self, agent_id: &AgentId) -> Vec<String> {
        let user_roles = self.user_roles.read().await;
        user_roles.get(agent_id).cloned().unwrap_or_default()
    }

    /// Check if an agent has a specific permission
    pub async fn check_permission(&self, agent_id: &AgentId, permission: Permission) -> bool {
        let user_roles = self.user_roles.read().await;
        let roles = self.roles.read().await;

        if let Some(agent_roles) = user_roles.get(agent_id) {
            for role_name in agent_roles {
                if let Some(role) = roles.get(role_name) {
                    // Check direct permission
                    if role.has_permission(&permission) {
                        return true;
                    }

                    // Check inherited permissions
                    if let Some(inherited) = &role.inherits {
                        for inherited_role_name in inherited {
                            if let Some(inherited_role) = roles.get(inherited_role_name) {
                                if inherited_role.has_permission(&permission) {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Get all permissions for an agent
    pub async fn get_agent_permissions(&self, agent_id: &AgentId) -> Vec<Permission> {
        let user_roles = self.user_roles.read().await;
        let roles = self.roles.read().await;
        let mut permissions = std::collections::HashSet::new();

        if let Some(agent_roles) = user_roles.get(agent_id) {
            for role_name in agent_roles {
                if let Some(role) = roles.get(role_name) {
                    // Add direct permissions
                    for perm in &role.permissions {
                        permissions.insert(perm.clone());
                    }

                    // Add inherited permissions
                    if let Some(inherited) = &role.inherits {
                        for inherited_role_name in inherited {
                            if let Some(inherited_role) = roles.get(inherited_role_name) {
                                for perm in &inherited_role.permissions {
                                    permissions.insert(perm.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        permissions.into_iter().collect()
    }
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_roles() {
        let rbac = RbacManager::new();
        
        let admin_role = rbac.get_role("admin").await.unwrap();
        assert!(admin_role.has_permission(&Permission::All));
        
        let user_role = rbac.get_role("user").await.unwrap();
        assert!(user_role.has_permission(&Permission::SendMessage));
        assert!(!user_role.has_permission(&Permission::Admin));
    }

    #[tokio::test]
    async fn test_assign_and_check() {
        let rbac = RbacManager::new();
        
        rbac.assign_role("agent-1".to_string(), "user".to_string()).await.unwrap();
        
        assert!(rbac.check_permission(&"agent-1".to_string(), Permission::SendMessage).await);
        assert!(!rbac.check_permission(&"agent-1".to_string(), Permission::Admin).await);
    }

    #[tokio::test]
    async fn test_custom_role() {
        let rbac = RbacManager::new();
        
        let custom_role = Role::new(
            "custom".to_string(),
            vec![Permission::SendMessage, Permission::ReceiveMessage],
        );
        
        rbac.add_role(custom_role).await.unwrap();
        rbac.assign_role("agent-2".to_string(), "custom".to_string()).await.unwrap();
        
        assert!(rbac.check_permission(&"agent-2".to_string(), Permission::SendMessage).await);
    }

    #[tokio::test]
    async fn test_revoke_role() {
        let rbac = RbacManager::new();
        
        rbac.assign_role("agent-1".to_string(), "admin".to_string()).await.unwrap();
        rbac.revoke_role(&"agent-1".to_string(), "admin").await.unwrap();
        
        assert!(!rbac.check_permission(&"agent-1".to_string(), Permission::Admin).await);
    }

    #[tokio::test]
    async fn test_get_permissions() {
        let rbac = RbacManager::new();
        
        rbac.assign_role("agent-1".to_string(), "user".to_string()).await.unwrap();
        
        let permissions = rbac.get_agent_permissions(&"agent-1".to_string()).await;
        assert!(permissions.contains(&Permission::SendMessage));
    }
}
