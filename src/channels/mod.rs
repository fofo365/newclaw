// Channel module for messaging platforms

use async_trait::async_trait;

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    
    async fn send_message(&self, target: &str, message: &str) -> Result<(), Box<dyn std::error::Error>>;
    
    async fn receive_messages(&self) -> Result<Vec<ChannelMessage>, Box<dyn std::error::Error>>;
}

#[derive(Debug, Clone)]
pub struct ChannelMessage {
    pub id: String,
    pub source: String,
    pub content: String,
    pub timestamp: i64,
    pub author: Option<String>,
}

// Channel implementations will be added later
// pub mod feishu;
// pub mod wecom;
