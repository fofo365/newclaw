// AGP Session

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

use super::message::AGPMessage;
use super::config::FederationDomain;

/// AGP 会话
#[derive(Clone)]
pub struct AGPSession {
    identity: String,
    peers: Vec<String>,
    domain: Option<FederationDomain>,
    connected: Arc<RwLock<bool>>,
}

impl AGPSession {
    pub async fn new(
        identity: String,
        initial_peers: Vec<String>,
        domain: Option<FederationDomain>,
    ) -> Result<Self> {
        Ok(Self {
            identity,
            peers: initial_peers,
            domain,
            connected: Arc::new(RwLock::new(true)),
        })
    }

    /// 接收消息
    pub async fn receive(&self) -> Option<AGPMessage> {
        // TODO: 实现消息接收
        None
    }

    /// 发送消息
    pub async fn send(&self, _message: AGPMessage) -> Result<()> {
        // TODO: 实现消息发送
        Ok(())
    }

    /// 离开会话
    pub async fn leave(&self) -> Result<()> {
        let mut connected = self.connected.write().await;
        *connected = false;
        Ok(())
    }

    /// 检查连接状态
    pub fn is_connected(&self) -> bool {
        // 简化实现
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let session = AGPSession::new(
            "test-agent".to_string(),
            vec!["peer-1".to_string()],
            Some(FederationDomain("test-mesh".to_string())),
        ).await.unwrap();

        assert_eq!(session.identity, "test-agent");
        assert_eq!(session.peers.len(), 1);
        assert!(session.is_connected());
    }
}
