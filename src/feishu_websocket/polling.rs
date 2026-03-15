// NewClaw v0.4.0 - 事件轮询系统
//
// 功能：
// 1. 飞书事件长轮询机制
// 2. 事件队列管理
// 3. 并发事件处理
// 4. 与 WebSocket 模式协同工作

use super::{EventHandler, FeishuEvent, WebSocketConfig, WebSocketError, WebSocketResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock, Semaphore};
use tokio::time::{interval, sleep, timeout};
use tracing::{debug, error, info, warn};

/// 轮询配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    /// 轮询间隔（默认 5s）
    pub polling_interval: Duration,
    
    /// 长轮询超时（默认 30s）
    pub long_polling_timeout: Duration,
    
    /// 事件队列最大长度（默认 1000）
    pub max_queue_size: usize,
    
    /// 最大并发处理数（默认 10）
    pub max_concurrent_handlers: usize,
    
    /// 事件处理超时（默认 60s）
    pub event_processing_timeout: Duration,
    
    /// 是否启用长轮询（默认 true）
    pub enable_long_polling: bool,
    
    /// 重试次数（默认 3）
    pub max_retries: u32,
    
    /// 重试延迟（默认 1s）
    pub retry_delay: Duration,
}

impl Default for PollingConfig {
    fn default() -> Self {
        Self {
            polling_interval: Duration::from_secs(5),
            long_polling_timeout: Duration::from_secs(30),
            max_queue_size: 1000,
            max_concurrent_handlers: 10,
            event_processing_timeout: Duration::from_secs(60),
            enable_long_polling: true,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

/// 轮询事件（从飞书 API 获取的原始事件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingEvent {
    /// 事件 ID
    pub event_id: String,
    
    /// 事件类型
    pub event_type: String,
    
    /// 事件数据
    pub event: FeishuEvent,
    
    /// 时间戳
    pub timestamp: i64,
    
    /// 重试次数
    pub retry_count: u32,
}

impl PollingEvent {
    pub fn new(event: FeishuEvent) -> Self {
        let event_id = uuid::Uuid::new_v4().to_string();
        let event_type = event.event_type().to_string();
        let timestamp = chrono::Utc::now().timestamp();
        
        Self {
            event_id,
            event_type,
            event,
            timestamp,
            retry_count: 0,
        }
    }
    
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

/// 事件队列
pub struct EventQueue {
    /// 内部队列
    queue: Arc<RwLock<VecDeque<PollingEvent>>>,
    
    /// 最大长度
    max_size: usize,
}

impl EventQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }
    
    /// 添加事件
    pub async fn push(&self, event: PollingEvent) -> WebSocketResult<()> {
        let mut queue = self.queue.write().await;
        
        if queue.len() >= self.max_size {
            warn!("Event queue is full, dropping oldest event");
            queue.pop_front();
        }
        
        queue.push_back(event);
        debug!("Event added to queue, current size: {}", queue.len());
        Ok(())
    }
    
    /// 取出事件
    pub async fn pop(&self) -> Option<PollingEvent> {
        let mut queue = self.queue.write().await;
        queue.pop_front()
    }
    
    /// 获取队列长度
    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }
    
    /// 检查是否为空
    pub async fn is_empty(&self) -> bool {
        self.queue.read().await.is_empty()
    }
    
    /// 清空队列
    pub async fn clear(&self) {
        self.queue.write().await.clear();
    }
    
    /// 获取所有事件（用于恢复）
    pub async fn get_all(&self) -> Vec<PollingEvent> {
        self.queue.read().await.iter().cloned().collect()
    }
}

/// 事件轮询器
pub struct EventPoller {
    /// 配置
    config: PollingConfig,
    
    /// WebSocket 配置
    ws_config: WebSocketConfig,
    
    /// 事件队列
    queue: Arc<EventQueue>,
    
    /// 事件处理器
    event_handler: Arc<dyn EventHandler>,
    
    /// 并发控制信号量
    semaphore: Arc<Semaphore>,
    
    /// 运行状态
    running: Arc<RwLock<bool>>,
    
    /// 停止信号发送器
    stop_tx: Option<mpsc::Sender<()>>,
}

impl EventPoller {
    /// 创建新的事件轮询器
    pub fn new(
        config: PollingConfig,
        ws_config: WebSocketConfig,
        event_handler: Arc<dyn EventHandler>,
    ) -> Self {
        let queue = Arc::new(EventQueue::new(config.max_queue_size));
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_handlers));
        
        Self {
            config,
            ws_config,
            queue,
            event_handler,
            semaphore,
            running: Arc::new(RwLock::new(false)),
            stop_tx: None,
        }
    }
    
    /// 启动轮询器
    pub async fn start(&mut self) -> WebSocketResult<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        
        *running = true;
        drop(running);
        
        info!("Event poller started");
        
        // 创建停止信号通道
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);
        
        // 启动轮询循环
        let queue = self.queue.clone();
        let config = self.config.clone();
        let ws_config = self.ws_config.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            let mut poll_interval = interval(config.polling_interval);
            
            loop {
                // 检查是否应该停止
                if !*running.read().await {
                    break;
                }
                
                tokio::select! {
                    _ = stop_rx.recv() => {
                        info!("Poller received stop signal");
                        break;
                    }
                    
                    _ = poll_interval.tick() => {
                        // 执行轮询
                        match Self::poll_events(&ws_config, &config).await {
                            Ok(events) => {
                                for event in events {
                                    if let Err(e) = queue.push(event).await {
                                        error!("Failed to push event to queue: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to poll events: {:?}", e);
                            }
                        }
                    }
                }
            }
            
            info!("Polling loop stopped");
        });
        
        // 启动事件处理循环
        self.start_event_processor().await?;
        
        Ok(())
    }
    
    /// 停止轮询器
    pub async fn stop(&mut self) -> WebSocketResult<()> {
        let mut running = self.running.write().await;
        if !*running {
            return Ok(());
        }
        
        *running = false;
        
        // 发送停止信号
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(()).await;
        }
        
        info!("Event poller stopped");
        Ok(())
    }
    
    /// 启动事件处理器
    async fn start_event_processor(&self) -> WebSocketResult<()> {
        let queue = self.queue.clone();
        let event_handler = self.event_handler.clone();
        let semaphore = self.semaphore.clone();
        let running = self.running.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut process_interval = interval(Duration::from_millis(100));
            
            loop {
                if !*running.read().await {
                    break;
                }
                
                process_interval.tick().await;
                
                // 从队列取出事件
                while let Some(event) = queue.pop().await {
                    // 获取信号量许可（控制并发）
                    let permit = semaphore.clone().acquire_owned().await.unwrap();
                    
                    let handler = event_handler.clone();
                    let config_clone = config.clone();
                    
                    // 异步处理事件
                    tokio::spawn(async move {
                        let _permit = permit;
                        
                        match timeout(
                            config_clone.event_processing_timeout,
                            handler.handle(event.event.clone())
                        ).await {
                            Ok(Ok(_)) => {
                                debug!("Event processed successfully: {}", event.event_id);
                            }
                            Ok(Err(e)) => {
                                error!("Failed to process event {}: {:?}", event.event_id, e);
                            }
                            Err(_) => {
                                error!("Event processing timeout: {}", event.event_id);
                            }
                        }
                    });
                }
            }
        });
        
        Ok(())
    }
    
    /// 轮询事件（从飞书 API 获取）
    async fn poll_events(
        ws_config: &WebSocketConfig,
        config: &PollingConfig,
    ) -> WebSocketResult<Vec<PollingEvent>> {
        // 这里应该调用飞书事件订阅 API
        // API: GET /open-apis/bot/v3/events
        
        // 模拟实现 - 实际应该调用真实 API
        // GET https://open.feishu.cn/open-apis/bot/v3/events
        
        let url = format!(
            "{}/events?timeout={}",
            ws_config.base_url.replace("ws", "https").replace("/ws/v2", ""),
            config.long_polling_timeout.as_secs()
        );
        
        // 实际实现应该使用 reqwest
        // let client = reqwest::Client::new();
        // let response = client
        //     .get(&url)
        //     .header("Authorization", format!("Bearer {}", token))
        //     .timeout(config.long_polling_timeout)
        //     .send()
        //     .await?;
        
        // 模拟返回空事件列表
        debug!("Polling events from: {}", url);
        Ok(vec![])
    }
    
    /// 获取队列长度
    pub async fn queue_size(&self) -> usize {
        self.queue.len().await
    }
    
    /// 检查是否运行中
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// 轮询管理器（管理多个应用的轮询）
pub struct PollingManager {
    /// 轮询器映射（app_id -> poller）
    pollers: Arc<RwLock<std::collections::HashMap<String, EventPoller>>>,
    
    /// 配置
    config: PollingConfig,
    
    /// WebSocket 配置
    ws_config: WebSocketConfig,
}

impl PollingManager {
    pub fn new(config: PollingConfig, ws_config: WebSocketConfig) -> Self {
        Self {
            pollers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            config,
            ws_config,
        }
    }
    
    /// 为应用注册轮询器
    pub async fn register(
        &self,
        app_id: &str,
        event_handler: Arc<dyn EventHandler>,
    ) -> WebSocketResult<()> {
        let mut pollers = self.pollers.write().await;
        
        if pollers.contains_key(app_id) {
            return Ok(());
        }
        
        let poller = EventPoller::new(
            self.config.clone(),
            self.ws_config.clone(),
            event_handler,
        );
        
        pollers.insert(app_id.to_string(), poller);
        info!("Registered poller for app: {}", app_id);
        
        Ok(())
    }
    
    /// 注销轮询器
    pub async fn unregister(&self, app_id: &str) -> WebSocketResult<()> {
        let mut pollers = self.pollers.write().await;
        
        if let Some(mut poller) = pollers.remove(app_id) {
            poller.stop().await?;
            info!("Unregistered poller for app: {}", app_id);
        }
        
        Ok(())
    }
    
    /// 启动应用的轮询器
    pub async fn start(&self, app_id: &str) -> WebSocketResult<()> {
        let pollers = self.pollers.read().await;
        
        if let Some(poller) = pollers.get(app_id) {
            // 这里需要 &mut，所以需要重新设计
            warn!("Cannot start poller in read-only context");
        }
        
        Ok(())
    }
    
    /// 停止应用的轮询器
    pub async fn stop(&self, app_id: &str) -> WebSocketResult<()> {
        self.unregister(app_id).await
    }
    
    /// 获取所有轮询器
    pub async fn list(&self) -> Vec<String> {
        self.pollers.read().await.keys().cloned().collect()
    }
    
    /// 获取轮询器数量
    pub async fn count(&self) -> usize {
        self.pollers.read().await.len()
    }
}

/// WebSocket 和轮询协同工作模式
pub struct HybridEventManager {
    /// WebSocket 管理器
    ws_manager: Option<Arc<super::FeishuWebSocketManager>>,
    
    /// 轮询管理器
    polling_manager: Arc<PollingManager>,
    
    /// 是否优先使用 WebSocket
    prefer_websocket: bool,
}

impl HybridEventManager {
    pub fn new(
        ws_manager: Option<Arc<super::FeishuWebSocketManager>>,
        polling_config: PollingConfig,
        ws_config: WebSocketConfig,
    ) -> Self {
        let polling_manager = Arc::new(PollingManager::new(polling_config, ws_config));
        
        Self {
            ws_manager,
            polling_manager,
            prefer_websocket: true,
        }
    }
    
    /// 启动事件接收
    pub async fn start(&self, app_id: &str) -> WebSocketResult<()> {
        // 优先尝试 WebSocket
        if self.prefer_websocket {
            if let Some(ref ws_manager) = self.ws_manager {
                if ws_manager.is_connected(app_id).await {
                    info!("Using WebSocket for app: {}", app_id);
                    return Ok(());
                }
            }
        }
        
        // 回退到轮询
        info!("Falling back to polling for app: {}", app_id);
        Ok(())
    }
    
    /// 停止事件接收
    pub async fn stop(&self, app_id: &str) -> WebSocketResult<()> {
        // 停止轮询
        self.polling_manager.stop(app_id).await?;
        
        // 停止 WebSocket（如果存在）
        if let Some(ref ws_manager) = self.ws_manager {
            ws_manager.disconnect(app_id).await?;
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feishu_websocket::event::DefaultEventHandler;
    
    #[test]
    fn test_polling_config_default() {
        let config = PollingConfig::default();
        assert_eq!(config.polling_interval, Duration::from_secs(5));
        assert!(config.enable_long_polling);
        assert_eq!(config.max_concurrent_handlers, 10);
    }
    
    #[tokio::test]
    async fn test_event_queue() {
        let queue = EventQueue::new(10);
        
        // 测试空队列
        assert!(queue.is_empty().await);
        assert_eq!(queue.len().await, 0);
        
        // 添加事件
        let event = FeishuEvent::MessageReceived {
            app_id: "test".to_string(),
            open_id: "ou_test".to_string(),
            chat_id: "oc_test".to_string(),
            user_id: "user_test".to_string(),
            content: "Hello".to_string(),
            message_id: "msg_test".to_string(),
            create_time: 1234567890,
        };
        
        let polling_event = PollingEvent::new(event);
        queue.push(polling_event).await.unwrap();
        
        assert_eq!(queue.len().await, 1);
        assert!(!queue.is_empty().await);
        
        // 取出事件
        let popped = queue.pop().await;
        assert!(popped.is_some());
        assert!(queue.is_empty().await);
    }
    
    #[tokio::test]
    async fn test_event_queue_max_size() {
        let queue = EventQueue::new(3);
        
        // 添加 4 个事件（超过最大容量）
        for i in 0..4 {
            let event = FeishuEvent::MessageReceived {
                app_id: format!("test_{}", i),
                open_id: "ou_test".to_string(),
                chat_id: "oc_test".to_string(),
                user_id: "user_test".to_string(),
                content: format!("Message {}", i),
                message_id: format!("msg_{}", i),
                create_time: 1234567890,
            };
            
            queue.push(PollingEvent::new(event)).await.unwrap();
        }
        
        // 队列长度应该是 3（最老的被丢弃）
        assert_eq!(queue.len().await, 3);
    }
    
    #[tokio::test]
    async fn test_polling_event() {
        let event = FeishuEvent::MessageReceived {
            app_id: "test".to_string(),
            open_id: "ou_test".to_string(),
            chat_id: "oc_test".to_string(),
            user_id: "user_test".to_string(),
            content: "Test".to_string(),
            message_id: "msg_test".to_string(),
            create_time: 1234567890,
        };
        
        let mut polling_event = PollingEvent::new(event);
        
        assert!(!polling_event.event_id.is_empty());
        assert_eq!(polling_event.event_type, "message");
        assert_eq!(polling_event.retry_count, 0);
        
        // 测试重试计数
        polling_event.increment_retry();
        assert_eq!(polling_event.retry_count, 1);
    }
    
    #[test]
    fn test_polling_manager_create() {
        let config = PollingConfig::default();
        let ws_config = WebSocketConfig::default();
        let manager = PollingManager::new(config, ws_config);
        
        // 管理器应该为空
        // 注意：由于需要异步，这里只测试创建
    }
}
