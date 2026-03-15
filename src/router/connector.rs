// Router Connector - v0.5.1
//
// 路由间通信连接器

use super::RouterId;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use anyhow::{Result, anyhow};

/// 路由消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterMessage {
    /// 消息 ID
    pub id: String,
    /// 发送者
    pub from: RouterId,
    /// 接收者
    pub to: RouterId,
    /// 动作类型
    pub action: Action,
    /// 消息内容
    pub payload: serde_json::Value,
    /// 时间戳
    pub timestamp: i64,
    /// 关联 ID（用于请求-响应匹配）
    pub correlation_id: Option<String>,
}

impl RouterMessage {
    /// 创建新消息
    pub fn new(from: RouterId, to: RouterId, action: Action, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            action,
            payload,
            timestamp: chrono::Utc::now().timestamp(),
            correlation_id: None,
        }
    }
    
    /// 创建请求消息
    pub fn request(from: RouterId, to: RouterId, payload: serde_json::Value) -> Self {
        let mut msg = Self::new(from, to, Action::Request, payload);
        msg.correlation_id = Some(uuid::Uuid::new_v4().to_string());
        msg
    }
    
    /// 创建响应消息
    pub fn response(from: RouterId, to: RouterId, correlation_id: String, payload: serde_json::Value) -> Self {
        let mut msg = Self::new(from, to, Action::Response, payload);
        msg.correlation_id = Some(correlation_id);
        msg
    }
    
    /// 创建命令消息
    pub fn command(from: RouterId, to: RouterId, payload: serde_json::Value) -> Self {
        Self::new(from, to, Action::Command, payload)
    }
    
    /// 创建通知消息
    pub fn notify(from: RouterId, to: RouterId, payload: serde_json::Value) -> Self {
        Self::new(from, to, Action::Notify, payload)
    }
}

/// 动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// 请求（下级 → 上级）
    Request,
    /// 响应（上级 → 下级）
    Response,
    /// 命令（上级 → 下级）
    Command,
    /// 共享（同级 ↔ 同级）
    Share,
    /// 通知（广播）
    Notify,
}

/// 路由连接器
pub struct RouterConnector {
    /// 消息发送通道
    sender: mpsc::Sender<RouterMessage>,
    /// 消息接收通道
    receiver: mpsc::Receiver<RouterMessage>,
    /// 路由 ID
    router_id: RouterId,
}

impl RouterConnector {
    /// 创建新的连接器
    pub fn new(router_id: RouterId, buffer_size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(buffer_size);
        Self {
            sender,
            receiver,
            router_id,
        }
    }
    
    /// 发送消息
    pub async fn send(&self, msg: RouterMessage) -> Result<()> {
        self.sender.send(msg).await
            .map_err(|e| anyhow!("Failed to send message: {}", e))
    }
    
    /// 接收消息
    pub async fn receive(&mut self) -> Option<RouterMessage> {
        self.receiver.recv().await
    }
    
    /// 尝试接收消息（非阻塞）
    pub fn try_receive(&mut self) -> Option<RouterMessage> {
        self.receiver.try_recv().ok()
    }
    
    /// 获取发送器克隆
    pub fn sender(&self) -> mpsc::Sender<RouterMessage> {
        self.sender.clone()
    }
    
    /// 获取路由 ID
    pub fn router_id(&self) -> &RouterId {
        &self.router_id
    }
}

/// 连接两个路由
pub fn connect(
    connector_a: &mut RouterConnector,
    connector_b: &mut RouterConnector,
) -> Result<()> {
    // 在实际实现中，这里会建立双向通信
    // 目前只是占位符
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_message() {
        let from = RouterId::new();
        let to = RouterId::new();
        let msg = RouterMessage::new(from.clone(), to.clone(), Action::Request, serde_json::json!({"test": "data"}));
        
        assert_eq!(msg.from, from);
        assert_eq!(msg.to, to);
        assert_eq!(msg.action, Action::Request);
    }

    #[test]
    fn test_router_message_request_response() {
        let from = RouterId::new();
        let to = RouterId::new();
        
        let request = RouterMessage::request(from.clone(), to.clone(), serde_json::json!({"query": "test"}));
        assert!(request.correlation_id.is_some());
        
        let response = RouterMessage::response(
            to.clone(),
            from.clone(),
            request.correlation_id.unwrap(),
            serde_json::json!({"result": "ok"})
        );
        assert!(response.correlation_id.is_some());
    }

    #[tokio::test]
    async fn test_router_connector() {
        let router_id = RouterId::new();
        let connector = RouterConnector::new(router_id.clone(), 10);
        
        let msg = RouterMessage::new(
            RouterId::new(),
            router_id.clone(),
            Action::Notify,
            serde_json::json!({"event": "test"})
        );
        
        connector.send(msg).await.unwrap();
    }
}
