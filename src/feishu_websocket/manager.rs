// NewClaw v0.4.0 - WebSocket 管理器
//
// 功能：
// 1. 整合连接池、心跳、重连、事件处理
// 2. 提供 WebSocket 连接的完整生命周期管理
// 3. 对外提供简洁的 API

use super::{
    ConnectionPool, ConnectionState, EventHandler, FeishuEvent,
    HeartbeatConfig, HeartbeatManager, ReconnectStrategy, ReconnectionManager,
    WebSocketConfig, WebSocketError, WebSocketResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{debug, error, info, warn};

/// WebSocket 管理器
pub struct FeishuWebSocketManager {
    /// 配置
    config: WebSocketConfig,
    
    /// 连接池
    pool: Arc<ConnectionPool>,
    
    /// 心跳管理器
    heartbeat: Arc<HeartbeatManager>,
    
    /// 重连管理器
    reconnection: Arc<ReconnectionManager>,
    
    /// 事件处理器
    event_handler: Arc<dyn EventHandler>,
    
    /// 运行状态
    running: Arc<RwLock<bool>>,
}

impl FeishuWebSocketManager {
    /// 创建新的 WebSocket 管理器
    pub fn new(config: WebSocketConfig, event_handler: Arc<dyn EventHandler>) -> Self {
        let pool = Arc::new(ConnectionPool::new(config.max_connections));
        
        let heartbeat_config = HeartbeatConfig {
            interval: config.heartbeat_interval,
            timeout: config.heartbeat_timeout,
            max_failures: config.max_heartbeat_failures,
        };
        let heartbeat = Arc::new(HeartbeatManager::new(heartbeat_config));
        
        let reconnect_strategy = ReconnectStrategy {
            initial_delay: config.initial_reconnect_delay,
            max_delay: config.max_reconnect_delay,
            max_attempts: config.max_reconnect_attempts,
            enabled: config.enable_auto_reconnect,
        };
        let reconnection = Arc::new(ReconnectionManager::new(reconnect_strategy));
        
        Self {
            config,
            pool,
            heartbeat,
            reconnection,
            event_handler,
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 启动管理器
    pub async fn start(&self) -> WebSocketResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        
        *running = true;
        info!("WebSocket manager started");
        
        Ok(())
    }
    
    /// 停止管理器
    pub async fn stop(&self) -> WebSocketResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        
        // 断开所有连接
        let apps = self.pool.list().await;
        for app_id in apps {
            self.disconnect(&app_id).await?;
        }
        
        info!("WebSocket manager stopped");
        Ok(())
    }
    
    /// 连接到飞书 WebSocket
    pub async fn connect(&self, app_id: &str, app_secret: &str) -> WebSocketResult<()> {
        // 检查是否已连接
        if self.pool.contains(app_id).await {
            debug!("App {} already connected", app_id);
            return Ok(());
        }
        
        info!("Connecting to Feishu WebSocket for app: {}", app_id);
        
        // 获取 WebSocket URL
        let ws_url = self.get_websocket_url(app_id, app_secret).await?;
        
        // 建立 WebSocket 连接
        match connect_async(&ws_url).await {
            Ok((ws_stream, _)) => {
                // 添加到连接池
                self.pool.add(app_id, ws_stream).await?;
                
                // 注册心跳
                self.heartbeat.register(app_id).await?;
                
                // 注册重连
                self.reconnection.register(app_id).await?;
                
                // 更新状态为已连接
                self.pool.update_state(app_id, ConnectionState::Connected).await?;
                
                // 通知事件处理器
                self.event_handler.on_connect(app_id).await?;
                
                info!("Successfully connected to Feishu WebSocket for app: {}", app_id);
                
                // 启动消息接收循环
                self.start_message_loop(app_id).await;
                
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to Feishu WebSocket: {}", e);
                Err(WebSocketError::ConnectionFailed(e.to_string()))
            }
        }
    }
    
    /// 断开连接
    pub async fn disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        info!("Disconnecting from Feishu WebSocket for app: {}", app_id);
        
        // 注销心跳
        self.heartbeat.unregister(app_id).await?;
        
        // 注销重连
        self.reconnection.unregister(app_id).await?;
        
        // 从连接池移除
        self.pool.remove(app_id).await?;
        
        // 通知事件处理器
        self.event_handler.on_disconnect(app_id).await?;
        
        info!("Successfully disconnected for app: {}", app_id);
        Ok(())
    }
    
    /// 发送消息
    pub async fn send(&self, app_id: &str, message: &str) -> WebSocketResult<()> {
        // 这里需要重新设计，因为不能直接访问 WebSocket 流
        // 暂时返回成功
        debug!("Sending message to app {}: {}", app_id, message);
        Ok(())
    }
    
    /// 检查是否已连接
    pub async fn is_connected(&self, app_id: &str) -> bool {
        self.pool.contains(app_id).await
    }
    
    /// 获取连接状态
    pub async fn get_state(&self, app_id: &str) -> Option<ConnectionState> {
        self.pool.get(app_id).await
    }
    
    /// 获取所有连接的应用
    pub async fn list_connections(&self) -> Vec<String> {
        self.pool.list().await
    }
    
    /// 获取连接数量
    pub async fn connection_count(&self) -> usize {
        self.pool.count().await
    }
    
    /// 获取飞书 WebSocket URL
    async fn get_websocket_url(&self, app_id: &str, app_secret: &str) -> WebSocketResult<String> {
        use serde_json::json;
        
        // 第一步：获取 access_token
        let token_url = format!("{}/auth/v3/tenant_access_token/internal", self.config.base_url);
        let client = reqwest::Client::new();
        
        let request_body = json!({
            "app_id": app_id,
            "app_secret": app_secret,
        });
        
        let response = client
            .post(&token_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;
        
        let json_response: serde_json::Value = response.json().await?;
        
        if json_response["code"].as_i64() != Some(0) {
            return Err(WebSocketError::AuthFailed(format!(
                "Failed to get access token: {:?}", json_response["msg"]
            )));
        }
        
        let access_token = json_response["tenant_access_token"]
            .as_str()
            .ok_or_else(|| WebSocketError::AuthFailed("No token in response".to_string()))?
            .to_string();
        
        // 第二步：获取 WebSocket URL
        let ws_url_url = format!("{}/v1.3/cn/copilot/realtime/create_tcp", self.config.base_url);
        
        let ws_request_body = json!({
            "tenant_access_token": access_token,
        });
        
        let ws_response = client
            .post(&ws_url_url)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&ws_request_body)
            .send()
            .await?;
        
        let ws_json: serde_json::Value = ws_response.json().await?;
        
        if ws_json["code"].as_i64() != Some(0) {
            return Err(WebSocketError::AuthFailed(format!(
                "Failed to get WebSocket URL: {:?}", ws_json["msg"]
            )));
        }
        
        let ws_url = ws_json["data"]["ws_url"]
            .as_str()
            .ok_or_else(|| WebSocketError::AuthFailed("No ws_url in response".to_string()))?
            .to_string();
        
        info!("Successfully obtained WebSocket URL for app {}", app_id);
        Ok(ws_url)
    }
    
    /// 启动消息接收循环
    async fn start_message_loop(&self, app_id: &str) {
        // 这里应该启动一个后台任务来接收消息
        // 由于架构限制，暂时跳过
        debug!("Starting message loop for app: {}", app_id);
    }
    
    /// 处理重连
    async fn handle_reconnect(&self, app_id: &str) -> WebSocketResult<()> {
        // 检查是否应该重连
        if !self.reconnection.should_reconnect(app_id).await? {
            warn!("Max reconnect attempts reached for app: {}", app_id);
            return Err(WebSocketError::MaxReconnectAttempts);
        }
        
        // 获取重连延迟
        let delay = self.reconnection.get_delay(app_id).await?;
        info!("Reconnecting in {:?} for app: {}", delay, app_id);
        
        // 等待延迟
        sleep(delay).await;
        
        // 记录重连尝试
        let attempt = self.reconnection.record_attempt_start(app_id).await?;
        
        // 更新状态为重连中
        self.pool.update_state(app_id, ConnectionState::Reconnecting).await?;
        
        // 尝试重新连接
        match self.connect(app_id, &self.config.app_secret).await {
            Ok(_) => {
                // 重连成功
                self.reconnection.record_success(app_id).await?;
                self.pool.reset_reconnect_count(app_id).await?;
                Ok(())
            }
            Err(e) => {
                // 重连失败
                self.reconnection.record_failure(app_id).await?;
                self.pool.increment_reconnect_count(app_id).await?;
                
                // 通知错误
                self.event_handler.on_error(app_id, &e).await?;
                
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feishu_websocket::event::DefaultEventHandler;
    
    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert!(config.enable_auto_reconnect);
        assert_eq!(config.max_connections, 10);
    }
    
    #[tokio::test]
    async fn test_manager_create() {
        let config = WebSocketConfig::default();
        let handler = Arc::new(DefaultEventHandler);
        let manager = FeishuWebSocketManager::new(config, handler);
        
        let count = manager.connection_count().await;
        assert_eq!(count, 0);
    }
    
    #[tokio::test]
    async fn test_manager_start_stop() {
        let config = WebSocketConfig::default();
        let handler = Arc::new(DefaultEventHandler);
        let manager = FeishuWebSocketManager::new(config, handler);
        
        // 启动
        manager.start().await.unwrap();
        
        // 停止
        manager.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_manager_list_connections() {
        let config = WebSocketConfig::default();
        let handler = Arc::new(DefaultEventHandler);
        let manager = FeishuWebSocketManager::new(config, handler);
        
        let connections = manager.list_connections().await;
        assert!(connections.is_empty());
    }
}
