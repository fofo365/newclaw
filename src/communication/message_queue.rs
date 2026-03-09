// Redis Message Queue Module (Optional)
// Requires "redis-support" feature

use super::message::{AgentId, InterAgentMessage};
use anyhow::{anyhow, Result};
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Redis message queue
pub struct RedisMessageQueue {
    client: Arc<redis::Client>,
    agent_id: AgentId,
}

impl RedisMessageQueue {
    /// Create a new Redis message queue
    pub async fn new(url: &str, agent_id: AgentId) -> Result<Self> {
        let client = redis::Client::open(url)?;
        
        // Test connection
        let mut conn = client.get_async_connection().await?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await?;
        
        Ok(Self {
            client: Arc::new(client),
            agent_id,
        })
    }

    /// Publish a message
    pub async fn publish(&self, msg: InterAgentMessage) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let channel = format!("agent:{}", msg.to);
        let payload = serde_json::to_string(&msg)?;
        
        conn.publish(&channel, payload).await?;
        
        Ok(())
    }

    /// Subscribe to messages for this agent
    pub async fn subscribe(&self) -> Result<mpsc::UnboundedReceiver<InterAgentMessage>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let client = self.client.clone();
        let agent_id = self.agent_id.clone();
        
        tokio::spawn(async move {
            let mut pubsub = match client.get_async_pubsub().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to create pubsub: {}", e);
                    return;
                }
            };
            
            let channel = format!("agent:{}", agent_id);
            if let Err(e) = pubsub.subscribe(&channel).await {
                tracing::error!("Failed to subscribe: {}", e);
                return;
            }
            
            let mut stream = pubsub.on_message();
            
            while let Some(msg) = stream.next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    if let Ok(message) = serde_json::from_str::<InterAgentMessage>(&payload) {
                        if tx.send(message).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        
        Ok(rx)
    }

    /// Subscribe to a topic
    pub async fn subscribe_topic(&self, topic: &str) -> Result<mpsc::UnboundedReceiver<InterAgentMessage>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let client = self.client.clone();
        let topic = topic.to_string();
        
        tokio::spawn(async move {
            let mut pubsub = match client.get_async_pubsub().await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to create pubsub: {}", e);
                    return;
                }
            };
            
            if let Err(e) = pubsub.subscribe(&topic).await {
                tracing::error!("Failed to subscribe to topic: {}", e);
                return;
            }
            
            let mut stream = pubsub.on_message();
            
            while let Some(msg) = stream.next().await {
                if let Ok(payload) = msg.get_payload::<String>() {
                    if let Ok(message) = serde_json::from_str::<InterAgentMessage>(&payload) {
                        if tx.send(message).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        
        Ok(rx)
    }

    /// Publish to a topic
    pub async fn publish_topic(&self, topic: &str, msg: InterAgentMessage) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let payload = serde_json::to_string(&msg)?;
        
        conn.publish(topic, payload).await?;
        
        Ok(())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        let mut conn = self.client.get_async_connection().await?;
        let result: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(result == "PONG")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Redis server
    // Run with: cargo test --features redis-support -- --ignored
    
    #[tokio::test]
    #[ignore]
    async fn test_redis_connection() {
        let queue = RedisMessageQueue::new("redis://localhost", "test-agent".to_string()).await;
        assert!(queue.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_health_check() {
        let queue = RedisMessageQueue::new("redis://localhost", "test-agent".to_string())
            .await
            .unwrap();
        
        let healthy = queue.health_check().await.unwrap();
        assert!(healthy);
    }
}
