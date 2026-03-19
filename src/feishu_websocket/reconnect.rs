// NewClaw v0.4.0 - 重连管理
//
// 功能：
// 1. 指数退避重连策略
// 2. 重连次数限制
// 3. 重连状态跟踪

use super::{WebSocketError, WebSocketResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 重连策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconnectStrategy {
    /// 初始重连延迟（默认 1s）
    pub initial_delay: Duration,
    
    /// 最大重连延迟（默认 60s）
    pub max_delay: Duration,
    
    /// 最大重连次数（默认 10）
    pub max_attempts: u32,
    
    /// 是否启用自动重连
    pub enabled: bool,
}

impl Default for ReconnectStrategy {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_attempts: 10,
            enabled: true,
        }
    }
}

/// 重连状态
#[derive(Debug, Clone)]
struct ReconnectState {
    /// 重连次数
    attempts: u32,
    
    /// 最后一次重连时间
    last_attempt: Option<Instant>,
    
    /// 下一次重连延迟
    next_delay: Duration,
    
    /// 是否正在重连
    is_reconnecting: bool,
}

/// 重连管理器
pub struct ReconnectionManager {
    /// 重连策略
    strategy: ReconnectStrategy,
    
    /// 重连状态映射
    states: Arc<RwLock<HashMap<String, ReconnectState>>>,
}

impl ReconnectionManager {
    pub fn new(strategy: ReconnectStrategy) -> Self {
        Self {
            strategy,
            states: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 注册应用
    pub async fn register(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        states.insert(app_id.to_string(), ReconnectState {
            attempts: 0,
            last_attempt: None,
            next_delay: self.strategy.initial_delay,
            is_reconnecting: false,
        });
        
        debug!("Registered reconnect for app: {}", app_id);
        Ok(())
    }
    
    /// 注销应用
    pub async fn unregister(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if states.remove(app_id).is_some() {
            debug!("Unregistered reconnect for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 检查是否应该重连
    pub async fn should_reconnect(&self, app_id: &str) -> WebSocketResult<bool> {
        if !self.strategy.enabled {
            return Ok(false);
        }
        
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(state.attempts < self.strategy.max_attempts)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 获取下一次重连延迟（指数退避）
    pub async fn get_delay(&self, app_id: &str) -> WebSocketResult<Duration> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(state.next_delay)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 记录重连开始
    pub async fn record_attempt_start(&self, app_id: &str) -> WebSocketResult<u32> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.attempts += 1;
            state.last_attempt = Some(Instant::now());
            state.is_reconnecting = true;
            
            info!(
                "Reconnect attempt {} for app: {}",
                state.attempts, app_id
            );
            
            Ok(state.attempts)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 记录重连成功
    pub async fn record_success(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.attempts = 0; // 重置计数
            state.next_delay = self.strategy.initial_delay; // 重置延迟
            state.is_reconnecting = false;
            
            info!("Reconnect successful for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 记录重连失败
    pub async fn record_failure(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.is_reconnecting = false;
            
            // 计算下一次延迟（指数退避）
            state.next_delay = std::cmp::min(
                state.next_delay * 2,
                self.strategy.max_delay,
            );
            
            warn!(
                "Reconnect failed for app: {}, next delay: {:?}",
                app_id, state.next_delay
            );
        }
        
        Ok(())
    }
    
    /// 获取重连次数
    pub async fn get_attempts(&self, app_id: &str) -> WebSocketResult<u32> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(state.attempts)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 检查是否达到最大重连次数
    pub async fn is_max_attempts_reached(&self, app_id: &str) -> WebSocketResult<bool> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(state.attempts >= self.strategy.max_attempts)
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 获取剩余重连次数
    pub async fn get_remaining_attempts(&self, app_id: &str) -> WebSocketResult<u32> {
        let states = self.states.read().await;
        
        if let Some(state) = states.get(app_id) {
            Ok(self.strategy.max_attempts.saturating_sub(state.attempts))
        } else {
            Err(WebSocketError::ConnectionNotFound(app_id.to_string()))
        }
    }
    
    /// 重置重连状态
    pub async fn reset(&self, app_id: &str) -> WebSocketResult<()> {
        let mut states = self.states.write().await;
        
        if let Some(state) = states.get_mut(app_id) {
            state.attempts = 0;
            state.next_delay = self.strategy.initial_delay;
            state.is_reconnecting = false;
            
            debug!("Reset reconnect state for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 获取所有注册的应用
    pub async fn get_registered_apps(&self) -> Vec<String> {
        let states = self.states.read().await;
        states.keys().cloned().collect()
    }
    
    /// 计算指数退避延迟
    pub fn calculate_exponential_backoff(attempt: u32, config: &ReconnectStrategy) -> Duration {
        let delay = config.initial_delay * 2u32.pow(attempt);
        std::cmp::min(delay, config.max_delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reconnect_strategy_default() {
        let strategy = ReconnectStrategy::default();
        assert_eq!(strategy.initial_delay, Duration::from_secs(1));
        assert_eq!(strategy.max_delay, Duration::from_secs(60));
        assert_eq!(strategy.max_attempts, 10);
        assert!(strategy.enabled);
    }
    
    #[tokio::test]
    async fn test_reconnect_manager_create() {
        let manager = ReconnectionManager::new(ReconnectStrategy::default());
        let apps = manager.get_registered_apps().await;
        assert!(apps.is_empty());
    }
    
    #[tokio::test]
    async fn test_reconnect_register() {
        let manager = ReconnectionManager::new(ReconnectStrategy::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 检查是否注册成功
        let apps = manager.get_registered_apps().await;
        assert_eq!(apps.len(), 1);
        assert!(apps.contains(&"test_app".to_string()));
    }
    
    #[tokio::test]
    async fn test_reconnect_attempts() {
        let manager = ReconnectionManager::new(ReconnectStrategy::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 检查是否应该重连
        let should = manager.should_reconnect("test_app").await.unwrap();
        assert!(should);
        
        // 记录重连尝试
        let attempt = manager.record_attempt_start("test_app").await.unwrap();
        assert_eq!(attempt, 1);
        
        // 检查重连次数
        let attempts = manager.get_attempts("test_app").await.unwrap();
        assert_eq!(attempts, 1);
        
        // 检查剩余次数
        let remaining = manager.get_remaining_attempts("test_app").await.unwrap();
        assert_eq!(remaining, 9);
    }
    
    #[tokio::test]
    async fn test_reconnect_success() {
        let manager = ReconnectionManager::new(ReconnectStrategy::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 记录重连尝试
        manager.record_attempt_start("test_app").await.unwrap();
        
        // 记录成功
        manager.record_success("test_app").await.unwrap();
        
        // 检查重连次数（应该重置为 0）
        let attempts = manager.get_attempts("test_app").await.unwrap();
        assert_eq!(attempts, 0);
        
        // 检查延迟（应该重置为初始值）
        let delay = manager.get_delay("test_app").await.unwrap();
        assert_eq!(delay, Duration::from_secs(1));
    }
    
    #[tokio::test]
    async fn test_reconnect_failure() {
        let manager = ReconnectionManager::new(ReconnectStrategy::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 记录重连尝试
        manager.record_attempt_start("test_app").await.unwrap();
        
        // 记录失败
        manager.record_failure("test_app").await.unwrap();
        
        // 检查延迟（应该增加）
        let delay = manager.get_delay("test_app").await.unwrap();
        assert_eq!(delay, Duration::from_secs(2)); // 1 * 2 = 2
    }
    
    #[tokio::test]
    async fn test_max_attempts() {
        let manager = ReconnectionManager::new(ReconnectStrategy::default());
        
        // 注册应用
        manager.register("test_app").await.unwrap();
        
        // 达到最大重连次数
        for _ in 0..10 {
            manager.record_attempt_start("test_app").await.unwrap();
        }
        
        // 检查是否达到最大次数
        let is_max = manager.is_max_attempts_reached("test_app").await.unwrap();
        assert!(is_max);
        
        // 检查是否应该重连（不应该）
        let should = manager.should_reconnect("test_app").await.unwrap();
        assert!(!should);
    }
    
    #[test]
    fn test_exponential_backoff() {
        let config = ReconnectStrategy::default();
        
        // 测试指数退避
        assert_eq!(
            ReconnectionManager::calculate_exponential_backoff(0, &config),
            Duration::from_secs(1)
        );
        
        assert_eq!(
            ReconnectionManager::calculate_exponential_backoff(1, &config),
            Duration::from_secs(2)
        );
        
        assert_eq!(
            ReconnectionManager::calculate_exponential_backoff(2, &config),
            Duration::from_secs(4)
        );
        
        // 测试最大延迟限制
        assert_eq!(
            ReconnectionManager::calculate_exponential_backoff(10, &config),
            Duration::from_secs(60) // 限制为最大延迟
        );
    }
}
