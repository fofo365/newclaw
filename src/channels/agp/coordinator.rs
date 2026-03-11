// 轻量级协调平面

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub capabilities: Vec<String>,
    pub endpoint: String,
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

/// 注册结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub identity: String,
    pub initial_peers: Vec<String>,
}

/// 协调器后端
pub enum CoordinatorBackend {
    Embedded(EmbeddedCoordinator),
}

/// 协调器客户端
pub struct CoordinatorClient {
    bootstrap: String,
    backend: CoordinatorBackend,
}

impl CoordinatorClient {
    /// 连接到协调平面
    pub async fn connect(bootstrap: &str) -> Result<Self> {
        // 如果 bootstrap 是本地地址，使用嵌入式协调器
        let backend = if bootstrap.contains("localhost") || bootstrap.contains("127.0.0.1") {
            CoordinatorBackend::Embedded(EmbeddedCoordinator::new())
        } else {
            // TODO: 实现远程协调器
            CoordinatorBackend::Embedded(EmbeddedCoordinator::new())
        };

        Ok(Self {
            bootstrap: bootstrap.to_string(),
            backend,
        })
    }

    /// 注册 Agent
    pub async fn register(
        &self,
        agent_id: &str,
        capabilities: &[String],
        endpoint: String,
    ) -> Result<Registration> {
        match &self.backend {
            CoordinatorBackend::Embedded(coord) => {
                coord.register(agent_id.to_string(), capabilities.to_vec(), endpoint).await
            }
        }
    }

    /// 注销 Agent
    pub async fn unregister(&self, agent_id: &str) -> Result<()> {
        match &self.backend {
            CoordinatorBackend::Embedded(coord) => {
                coord.unregister(agent_id).await
            }
        }
    }

    /// 发现 Agent
    pub async fn discover(&self, capability: &str) -> Result<Vec<AgentInfo>> {
        match &self.backend {
            CoordinatorBackend::Embedded(coord) => {
                Ok(coord.discover(capability).await)
            }
        }
    }
}

/// 嵌入式协调器（基于 gossip）
pub struct EmbeddedCoordinator {
    peers: Arc<RwLock<HashMap<String, AgentInfo>>>,
}

impl EmbeddedCoordinator {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册 Agent
    pub async fn register(
        &self,
        agent_id: String,
        capabilities: Vec<String>,
        endpoint: String,
    ) -> Result<Registration> {
        let info = AgentInfo {
            id: agent_id.clone(),
            capabilities,
            endpoint,
            registered_at: chrono::Utc::now(),
        };

        let mut peers = self.peers.write().await;
        peers.insert(agent_id.clone(), info);

        // 返回当前已知的部分节点（用于建立连接）
        let initial_peers = peers.keys()
            .filter(|id| *id != &agent_id)
            .take(3) // 随机选择 3 个邻居
            .cloned()
            .collect();

        Ok(Registration {
            identity: agent_id,
            initial_peers,
        })
    }

    /// 注销 Agent
    pub async fn unregister(&self, agent_id: &str) -> Result<()> {
        let mut peers = self.peers.write().await;
        peers.remove(agent_id);
        Ok(())
    }

    /// 发现 Agent
    pub async fn discover(&self, capability: &str) -> Vec<AgentInfo> {
        let peers = self.peers.read().await;
        peers.values()
            .filter(|info| info.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }
}

impl Default for EmbeddedCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_coordinator() {
        let coord = EmbeddedCoordinator::new();

        // 注册两个 Agent
        let reg1 = coord.register(
            "agent-1".to_string(),
            vec!["math".to_string()],
            "agp://localhost:7777/agent-1".to_string(),
        ).await.unwrap();

        let reg2 = coord.register(
            "agent-2".to_string(),
            vec!["math".to_string(), "physics".to_string()],
            "agp://localhost:7777/agent-2".to_string(),
        ).await.unwrap();

        assert_eq!(reg1.identity, "agent-1");
        assert!(reg1.initial_peers.is_empty()); // 第一个注册，没有邻居

        assert_eq!(reg2.identity, "agent-2");
        assert_eq!(reg2.initial_peers.len(), 1); // 有一个邻居

        // 发现
        let math_agents = coord.discover("math").await;
        assert_eq!(math_agents.len(), 2);

        let physics_agents = coord.discover("physics").await;
        assert_eq!(physics_agents.len(), 1);

        // 注销
        coord.unregister("agent-1").await.unwrap();
        let math_agents = coord.discover("math").await;
        assert_eq!(math_agents.len(), 1);
    }

    #[tokio::test]
    async fn test_coordinator_client() {
        let client = CoordinatorClient::connect("agp://localhost:8000").await.unwrap();

        let reg = client.register(
            "test-agent",
            &["test".to_string()],
            "agp://localhost:7777/test-agent".to_string(),
        ).await.unwrap();

        assert_eq!(reg.identity, "test-agent");
    }
}
