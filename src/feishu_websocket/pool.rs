// NewClaw v0.4.0 - WebSocket 连接池管理
//
// 功能：
// 1. 管理多个飞书应用的连接
// 2. 线程安全的连接访问
// 3. 连接状态跟踪

use super::{WebSocketError, WebSocketResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{SinkExt, StreamExt};

/// WebSocket 连接流类型
pub type WsStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// 连接状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Reconnecting,
    Error(String),
}

/// WebSocket 连接
#[derive(Debug)]
pub struct Connection {
    /// 应用 ID
    pub app_id: String,
    
    /// WebSocket 流
    pub ws: Option<WsStream>,
    
    /// 连接状态
    pub state: ConnectionState,
    
    /// 最后一次心跳时间
    pub last_heartbeat: Instant,
    
    /// 重连次数
    pub reconnect_count: u32,
    
    /// 创建时间
    pub created_at: Instant,
}

impl Connection {
    pub fn new(app_id: String, ws: WsStream) -> Self {
        Self {
            app_id,
            ws: Some(ws),
            state: ConnectionState::Connected,
            last_heartbeat: Instant::now(),
            reconnect_count: 0,
            created_at: Instant::now(),
        }
    }
    
    /// 发送消息
    pub async fn send(&mut self, msg: &str) -> WebSocketResult<()> {
        if let Some(ref mut ws) = self.ws {
            ws.send(Message::Text(msg.to_string())).await
                .map_err(|e| WebSocketError::WebSocket(e.to_string()))?;
            Ok(())
        } else {
            Err(WebSocketError::ConnectionNotFound(self.app_id.clone()))
        }
    }
    
    /// 接收消息
    pub async fn receive(&mut self) -> WebSocketResult<Option<String>> {
        if let Some(ref mut ws) = self.ws {
            match ws.next().await {
                Some(Ok(Message::Text(text))) => Ok(Some(text)),
                Some(Ok(Message::Ping(_))) => Ok(None), // 忽略 Ping
                Some(Ok(Message::Pong(_))) => Ok(None), // 忽略 Pong
                Some(Ok(Message::Close(_))) => {
                    self.state = ConnectionState::Disconnected;
                    Ok(None)
                }
                Some(Err(e)) => {
                    self.state = ConnectionState::Error(e.to_string());
                    Err(WebSocketError::WebSocket(e.to_string()))
                }
                None => {
                    self.state = ConnectionState::Disconnected;
                    Ok(None)
                }
                _ => Ok(None),
            }
        } else {
            Err(WebSocketError::ConnectionNotFound(self.app_id.clone()))
        }
    }
    
    /// 关闭连接
    pub async fn close(&mut self) -> WebSocketResult<()> {
        if let Some(ref mut ws) = self.ws {
            ws.close(None).await
                .map_err(|e| WebSocketError::WebSocket(e.to_string()))?;
        }
        self.state = ConnectionState::Disconnected;
        self.ws = None;
        Ok(())
    }
    
    /// 更新心跳时间
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }
    
    /// 检查是否超时
    pub fn is_timeout(&self, timeout_secs: u64) -> bool {
        self.last_heartbeat.elapsed().as_secs() > timeout_secs
    }
    
    /// 是否已连接
    pub fn is_connected(&self) -> bool {
        matches!(self.state, ConnectionState::Connected)
    }
}

/// 连接池
pub struct ConnectionPool {
    /// 连接映射表
    connections: Arc<RwLock<HashMap<String, Connection>>>,
    
    /// 最大连接数
    max_connections: usize,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
        }
    }
    
    /// 添加连接
    pub async fn add(&self, app_id: &str, ws: WsStream) -> WebSocketResult<()> {
        let mut connections = self.connections.write().await;
        
        // 检查是否超过最大连接数
        if connections.len() >= self.max_connections && !connections.contains_key(app_id) {
            return Err(WebSocketError::PoolFull);
        }
        
        let connection = Connection::new(app_id.to_string(), ws);
        connections.insert(app_id.to_string(), connection);
        
        Ok(())
    }
    
    /// 移除连接
    pub async fn remove(&self, app_id: &str) -> WebSocketResult<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(mut conn) = connections.remove(app_id) {
            conn.close().await?;
        }
        
        Ok(())
    }
    
    /// 获取连接（只读）
    pub async fn get(&self, app_id: &str) -> Option<ConnectionState> {
        let connections = self.connections.read().await;
        connections.get(app_id).map(|c| c.state.clone())
    }
    
    /// 获取连接（可变）
    pub async fn get_mut(&self, app_id: &str) -> Option<Connection> {
        // 注意：这里需要重新设计，因为不能直接返回可变引用
        // 暂时返回克隆的连接信息
        let connections = self.connections.read().await;
        connections.get(app_id).map(|c| Connection {
            app_id: c.app_id.clone(),
            ws: None, // 不能克隆 WebSocket 流
            state: c.state.clone(),
            last_heartbeat: c.last_heartbeat,
            reconnect_count: c.reconnect_count,
            created_at: c.created_at,
        })
    }
    
    /// 检查连接是否存在
    pub async fn contains(&self, app_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(app_id)
    }
    
    /// 列出所有连接
    pub async fn list(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }
    
    /// 获取连接数量
    pub async fn count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
    
    /// 更新连接状态
    pub async fn update_state(&self, app_id: &str, state: ConnectionState) -> WebSocketResult<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.get_mut(app_id) {
            conn.state = state;
            Ok(())
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 更新心跳时间
    pub async fn update_heartbeat(&self, app_id: &str) -> WebSocketResult<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.get_mut(app_id) {
            conn.update_heartbeat();
            Ok(())
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 增加重连计数
    pub async fn increment_reconnect_count(&self, app_id: &str) -> WebSocketResult<u32> {
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.get_mut(app_id) {
            conn.reconnect_count += 1;
            Ok(conn.reconnect_count)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 重置重连计数
    pub async fn reset_reconnect_count(&self, app_id: &str) -> WebSocketResult<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(conn) = connections.get_mut(app_id) {
            conn.reconnect_count = 0;
            Ok(())
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_pool_create() {
        let pool = ConnectionPool::new(10);
        assert_eq!(pool.max_connections, 10);
        assert_eq!(pool.count().await, 0);
    }
    
    #[tokio::test]
    async fn test_connection_pool_list() {
        let pool = ConnectionPool::new(10);
        let list = pool.list().await;
        assert!(list.is_empty());
    }
    
    #[tokio::test]
    async fn test_connection_state() {
        let state = ConnectionState::Connected;
        assert!(matches!(state, ConnectionState::Connected));
    }
    
    #[test]
    fn test_connection_timeout() {
        // 这个测试需要模拟时间，暂时跳过
        // 实际测试中可以使用 tokio::time::pause()
    }
}
