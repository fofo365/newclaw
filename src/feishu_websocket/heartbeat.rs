// NewClaw v0.4.0 - 心跳检测管理
//
// 功能：
// 1. 定时发送心跳包
// 2. 检测心跳超时
// 3. 失败计数管理

use super::{WebSocketError, WebSocketResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

/// 心跳配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    /// 心跳间隔（默认 30s）
    pub interval: Duration,
    
    /// 超时时间（默认 10s）
    pub timeout: Duration,
    
    /// 最大失败次数（默认 3）
    pub max_failures: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            max_failures: 3,
        }
    }
}

/// 心跳状态
#[derive(Debug, Clone)]
struct HeartbeatState {
    /// 最后一次心跳时间
    last_sent: Instant,
    
    /// 最后一次响应时间
    last_received: Option<Instant>,
    
    /// 连续失败次数
    failure_count: u32,
    
    /// 是否正在等待响应
    waiting_for_response: bool,
}

/// 心跳管理器
pub struct HeartbeatManager {
    /// 配置
    config: HeartbeatConfig,
    
    /// 心跳状态映射
    states: Arc<RwLock<HashMap<String, HeartbeatState>>>,
    
    /// 是否运行中
    running: Arc<RwLock<bool>>,
}

impl HeartbeatManager {
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            states: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// 注册应用（开始心跳）
    pub async fn register(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        states.insert(app_id.to_string(), HeartbeatState {
            last_sent: Instant::now(),
            last_received: None,
            failure_count: 0,
            waiting_for_response: false,
        });
        
        info!("Registered heartbeat for app: {}", app_id);
        Ok(())
    }
    
    /// 注销应用（停止心跳）
    pub async fn unregister(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if states.remove(app_id).is_some() {
            info!("Unregistered heartbeat for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 记录心跳发送
    pub async fn record_send(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.last_sent = Instant::now();
            state.waiting_for_response = true;
            debug!("Heartbeat sent for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 记录心跳响应
    pub async fn record_receive(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.last_received = Some(Instant::now());
            state.waiting_for_response = false;
            state.failure_count = 0; // 重置失败计数
            debug!("Heartbeat received for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 检查是否需要发送心跳
    pub async fn should_send(&self, app_id: &str) -> WebSocketResult<bool> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            let elapsed = state.last_sent.elapsed();
            Ok(elapsed >= self.config.interval)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 检查心跳是否超时
    pub async fn is_timeout(&self, app_id: &str) -> WebSocketResult<bool> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            if state.waiting_for_response {
                if let Some(last_received) = state.last_received {
                    // 检查是否超时
                    let elapsed = last_received.elapsed();
                    Ok(elapsed > self.config.timeout)
                } else {
                    // 还没收到过响应，检查从发送开始的时间
                    let elapsed = state.last_sent.elapsed();
                    Ok(elapsed > self.config.timeout)
                }
            } else {
                Ok(false)
            }
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 增加失败计数
    pub async fn increment_failure(&self, app_id: &str) -> WebSocketResult<u32> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.failure_count += 1;
            state.waiting_for_response = false;
            
            warn!(
                "Heartbeat failure for app: {}, count: {}",
                app_id, state.failure_count
            );
            
            Ok(state.failure_count)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 检查是否达到最大失败次数
    pub async fn should_reconnect(&self, app_id: &str) -> WebSocketResult<bool> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(state.failure_count >= self.config.max_failures)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 获取失败计数
    pub async fn get_failure_count(&self, app_id: &str) -> WebSocketResult<u32> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(state.failure_count)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 重置失败计数
    pub async fn reset_failure_count(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.failure_count = 0;
            debug!("Reset heartbeat failure count for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 获取所有需要心跳的应用
    pub async fn get_registered_apps(&self) -> Vec<String> {
        let states = self.states.read().await;
        states.keys().cloned().collect()
    }
    
    /// 生成心跳消息
    pub fn generate_heartbeat_message() -> String {
        // 飞书心跳消息格式
        serde_json::json!({
            "type": "ping"
        }).to_string()
    }
    
    /// 检查是否为心跳响应
    pub fn is_heartbeat_response(message: &str) -> bool {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(message) {
            json.get("type").and_then(|v| v.as_str()) == Some("pong")
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_failures, 3);
    }
    
    #[tokio::test]
    async fn test_heartbeat_manager_create() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        let apps = manager.get_registered_apps().await;
        assert!(apps.is_empty());
    }
    
    #[tokio::test]
    async fn test_heartbeat_register() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 检查是否注册成功
        let apps = manager.get_registered_apps().await;
        assert_eq!(apps.len(), 1);
        assert!(apps.contains(&"test_app".to_string()));
    }
    
    #[tokio::test]
    async fn test_heartbeat_unregister() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 注销应用
        manager.unregister("test_app").await.unwrap();
        
        // 检查是否注销成功
        let apps = manager.get_registered_apps().await;
        assert!(apps.is_empty());
    }
    
    #[tokio::test]
    async fn test_heartbeat_send_receive() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 记录发送
        manager.record_send("test_app").await.unwrap();
        
        // 检查是否需要发送（刚发送过，应该不需要）
        let should_send = manager.should_send("test_app").await.unwrap();
        assert!(!should_send);
        
        // 记录接收
        manager.record_receive("test_app").await.unwrap();
        
        // 检查失败计数（应该为 0）
        let count = manager.get_failure_count("test_app").await.unwrap();
        assert_eq!(count, 0);
    }
    
    #[tokio::test]
    async fn test_heartbeat_failure() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 增加失败计数
        let count = manager.increment_failure("test_app").await.unwrap();
        assert_eq!(count, 1);
        
        // 检查是否需要重连（失败次数 < 3，不需要）
        let should_reconnect = manager.should_reconnect("test_app").await.unwrap();
        assert!(!should_reconnect);
        
        // 再增加两次失败
        manager.increment_failure("test_app").await.unwrap();
        manager.increment_failure("test_app").await.unwrap();
        
        // 检查是否需要重连（失败次数 >= 3，需要）
        let should_reconnect = manager.should_reconnect("test_app").await.unwrap();
        assert!(should_reconnect);
    }
    
    #[test]
    fn test_heartbeat_message() {
        let msg = HeartbeatManager::generate_heartbeat_message();
        assert!(msg.contains("ping"));
        
        // 测试心跳响应检测
        let pong_msg = r#"{"type":"pong"}"#;
        assert!(HeartbeatManager::is_heartbeat_response(pong_msg));
        
        let other_msg = r#"{"type":"message"}"#;
        assert!(!HeartbeatManager::is_heartbeat_response(other_msg));
    }
}
