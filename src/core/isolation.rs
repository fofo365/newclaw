// Context Isolation Module
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Isolation level
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(Default)]
pub enum IsolationLevel {
    /// No isolation - global context
    #[default]
    None,
    /// User-level isolation
    User(String),
    /// Session-level isolation
    Session(String),
}


impl IsolationLevel {
    /// Get namespace for this isolation level
    pub fn namespace(&self) -> String {
        match self {
            IsolationLevel::None => "global".to_string(),
            IsolationLevel::User(id) => format!("user:{}", id),
            IsolationLevel::Session(id) => format!("session:{}", id),
        }
    }

    /// Check if this is global context
    pub fn is_global(&self) -> bool {
        matches!(self, IsolationLevel::None)
    }

    /// Create user isolation
    pub fn user(id: impl Into<String>) -> Self {
        Self::User(id.into())
    }

    /// Create session isolation
    pub fn session(id: impl Into<String>) -> Self {
        Self::Session(id.into())
    }
}

/// Context isolation manager
pub struct ContextIsolation {
    isolation: IsolationLevel,
    namespaces: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ContextIsolation {
    /// Create a new context isolation manager
    pub fn new(isolation: IsolationLevel) -> Self {
        Self {
            isolation,
            namespaces: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get current isolation level
    pub fn level(&self) -> &IsolationLevel {
        &self.isolation
    }

    /// Add a message to the current namespace
    pub async fn add_message(&self, message_id: &str) -> Result<()> {
        let namespace = self.isolation.namespace();
        let mut namespaces = self.namespaces.write().await;
        namespaces
            .entry(namespace)
            .or_insert_with(Vec::new)
            .push(message_id.to_string());
        Ok(())
    }

    /// Get all message IDs in the current namespace
    pub async fn get_messages(&self) -> Vec<String> {
        let namespace = self.isolation.namespace();
        let namespaces = self.namespaces.read().await;
        namespaces.get(&namespace).cloned().unwrap_or_default()
    }

    /// Clear all messages in the current namespace
    pub async fn clear(&self) -> Result<()> {
        let namespace = self.isolation.namespace();
        let mut namespaces = self.namespaces.write().await;
        namespaces.remove(&namespace);
        Ok(())
    }

    /// Get message count in the current namespace
    pub async fn message_count(&self) -> usize {
        let namespace = self.isolation.namespace();
        let namespaces = self.namespaces.read().await;
        namespaces.get(&namespace).map(|v| v.len()).unwrap_or(0)
    }

    /// Check if a message exists in the current namespace
    pub async fn has_message(&self, message_id: &str) -> bool {
        let namespace = self.isolation.namespace();
        let namespaces = self.namespaces.read().await;
        if let Some(messages) = namespaces.get(&namespace) {
            messages.contains(&message_id.to_string())
        } else {
            false
        }
    }

    /// Get statistics for all namespaces
    pub async fn stats(&self) -> HashMap<String, usize> {
        let namespaces = self.namespaces.read().await;
        namespaces
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_namespace() {
        assert_eq!(IsolationLevel::None.namespace(), "global");
        assert_eq!(IsolationLevel::user("user1").namespace(), "user:user1");
        assert_eq!(IsolationLevel::session("sess1").namespace(), "session:sess1");
    }

    #[tokio::test]
    async fn test_context_isolation() {
        let isolation = ContextIsolation::new(IsolationLevel::user("user1"));
        
        isolation.add_message("msg1").await.unwrap();
        isolation.add_message("msg2").await.unwrap();
        
        assert_eq!(isolation.message_count().await, 2);
        assert!(isolation.has_message("msg1").await);
        
        isolation.clear().await.unwrap();
        assert_eq!(isolation.message_count().await, 0);
    }

    #[tokio::test]
    async fn test_different_namespaces() {
        let isolation1 = ContextIsolation::new(IsolationLevel::user("user1"));
        let mut isolation2 = ContextIsolation::new(IsolationLevel::user("user2"));
        
        // Share the same namespaces map
        isolation2.namespaces = isolation1.namespaces.clone();
        
        isolation1.add_message("msg1").await.unwrap();
        isolation2.add_message("msg2").await.unwrap();
        
        assert_eq!(isolation1.message_count().await, 1);
        assert_eq!(isolation2.message_count().await, 1);
        assert!(isolation1.has_message("msg1").await);
        assert!(!isolation1.has_message("msg2").await);
    }
}
