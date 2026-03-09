// NewClaw v0.4.0 - 事件处理
//
// 功能：
// 1. 定义飞书事件类型
// 2. 事件处理器接口
// 3. 事件分发机制

use super::{WebSocketError, WebSocketResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 飞书事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FeishuEvent {
    /// 消息接收事件
    #[serde(rename = "message")]
    MessageReceived {
        #[serde(rename = "appId")]
        app_id: String,
        #[serde(rename = "openId")]
        open_id: String,
        #[serde(rename = "chatId")]
        chat_id: String,
        #[serde(rename = "userId")]
        user_id: String,
        content: String,
        #[serde(rename = "messageId")]
        message_id: String,
        #[serde(rename = "createTime")]
        create_time: i64,
    },
    
    /// 消息已读事件
    #[serde(rename = "read")]
    MessageRead {
        #[serde(rename = "chatId")]
        chat_id: String,
        #[serde(rename = "readTime")]
        read_time: i64,
    },
    
    /// 用户输入事件
    #[serde(rename = "typing")]
    UserTyping {
        #[serde(rename = "chatId")]
        chat_id: String,
        #[serde(rename = "openId")]
        open_id: String,
    },
    
    /// 机器人添加事件
    #[serde(rename = "bot_added")]
    BotAdded {
        #[serde(rename = "appId")]
        app_id: String,
        #[serde(rename = "chatId")]
        chat_id: String,
    },
    
    /// 机器人移除事件
    #[serde(rename = "bot_removed")]
    BotRemoved {
        #[serde(rename = "appId")]
        app_id: String,
        #[serde(rename = "chatId")]
        chat_id: String,
    },
    
    /// 错误事件
    #[serde(rename = "error")]
    Error {
        code: i32,
        message: String,
    },
}

impl FeishuEvent {
    /// 从 JSON 字符串解析事件
    pub fn from_json(json: &str) -> WebSocketResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
    
    /// 转换为 JSON 字符串
    pub fn to_json(&self) -> WebSocketResult<String> {
        serde_json::to_string(self)
            .map_err(|e| WebSocketError::Serialization(e.to_string()))
    }
    
    /// 获取事件类型名称
    pub fn event_type(&self) -> &str {
        match self {
            FeishuEvent::MessageReceived { .. } => "message",
            FeishuEvent::MessageRead { .. } => "read",
            FeishuEvent::UserTyping { .. } => "typing",
            FeishuEvent::BotAdded { .. } => "bot_added",
            FeishuEvent::BotRemoved { .. } => "bot_removed",
            FeishuEvent::Error { .. } => "error",
        }
    }
}

/// 事件处理器接口
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// 处理事件
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()>;
    
    /// 连接建立回调
    async fn on_connect(&self, app_id: &str) -> WebSocketResult<()> {
        Ok(())
    }
    
    /// 连接断开回调
    async fn on_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        Ok(())
    }
    
    /// 错误回调
    async fn on_error(&self, app_id: &str, error: &WebSocketError) -> WebSocketResult<()> {
        Ok(())
    }
}

/// 默认事件处理器（打印日志）
pub struct DefaultEventHandler;

#[async_trait]
impl EventHandler for DefaultEventHandler {
    async fn handle(&self, event: FeishuEvent) -> WebSocketResult<()> {
        tracing::info!("Received event: {:?}", event);
        Ok(())
    }
    
    async fn on_connect(&self, app_id: &str) -> WebSocketResult<()> {
        tracing::info!("Connected: {}", app_id);
        Ok(())
    }
    
    async fn on_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        tracing::info!("Disconnected: {}", app_id);
        Ok(())
    }
    
    async fn on_error(&self, app_id: &str, error: &WebSocketError) -> WebSocketResult<()> {
        tracing::error!("Error for {}: {:?}", app_id, error);
        Ok(())
    }
}

/// 事件分发器
pub struct EventDispatcher {
    handlers: HashMap<String, Box<dyn EventHandler>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }
    
    /// 注册事件处理器
    pub fn register(&mut self, app_id: String, handler: Box<dyn EventHandler>) {
        self.handlers.insert(app_id, handler);
    }
    
    /// 注销事件处理器
    pub fn unregister(&mut self, app_id: &str) {
        self.handlers.remove(app_id);
    }
    
    /// 分发事件
    pub async fn dispatch(&self, app_id: &str, event: FeishuEvent) -> WebSocketResult<()> {
        if let Some(handler) = self.handlers.get(app_id) {
            handler.handle(event).await?;
        }
        Ok(())
    }
    
    /// 通知连接建立
    pub async fn notify_connect(&self, app_id: &str) -> WebSocketResult<()> {
        if let Some(handler) = self.handlers.get(app_id) {
            handler.on_connect(app_id).await?;
        }
        Ok(())
    }
    
    /// 通知连接断开
    pub async fn notify_disconnect(&self, app_id: &str) -> WebSocketResult<()> {
        if let Some(handler) = self.handlers.get(app_id) {
            handler.on_disconnect(app_id).await?;
        }
        Ok(())
    }
    
    /// 通知错误
    pub async fn notify_error(&self, app_id: &str, error: &WebSocketError) -> WebSocketResult<()> {
        if let Some(handler) = self.handlers.get(app_id) {
            handler.on_error(app_id, error).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_feishu_event_from_json() {
        let json = r#"{
            "type": "message",
            "appId": "cli_test",
            "openId": "ou_test",
            "chatId": "oc_test",
            "userId": "user_test",
            "content": "Hello",
            "messageId": "msg_test",
            "createTime": 1234567890
        }"#;
        
        let event = FeishuEvent::from_json(json).unwrap();
        assert_eq!(event.event_type(), "message");
        
        if let FeishuEvent::MessageReceived { app_id, content, .. } = event {
            assert_eq!(app_id, "cli_test");
            assert_eq!(content, "Hello");
        } else {
            panic!("Wrong event type");
        }
    }
    
    #[test]
    fn test_feishu_event_to_json() {
        let event = FeishuEvent::MessageReceived {
            app_id: "cli_test".to_string(),
            open_id: "ou_test".to_string(),
            chat_id: "oc_test".to_string(),
            user_id: "user_test".to_string(),
            content: "Hello".to_string(),
            message_id: "msg_test".to_string(),
            create_time: 1234567890,
        };
        
        let json = event.to_json().unwrap();
        assert!(json.contains("message"));
        assert!(json.contains("Hello"));
    }
    
    #[tokio::test]
    async fn test_default_event_handler() {
        let handler = DefaultEventHandler;
        
        let event = FeishuEvent::MessageReceived {
            app_id: "test".to_string(),
            open_id: "ou_test".to_string(),
            chat_id: "oc_test".to_string(),
            user_id: "user_test".to_string(),
            content: "Test".to_string(),
            message_id: "msg_test".to_string(),
            create_time: 1234567890,
        };
        
        handler.handle(event).await.unwrap();
        handler.on_connect("test").await.unwrap();
        handler.on_disconnect("test").await.unwrap();
    }
    
    #[test]
    fn test_event_dispatcher() {
        let mut dispatcher = EventDispatcher::new();
        
        // 注册处理器
        dispatcher.register("test_app".to_string(), Box::new(DefaultEventHandler));
        
        // 检查是否注册成功
        assert!(dispatcher.handlers.contains_key("test_app"));
        
        // 注销处理器
        dispatcher.unregister("test_app");
        
        // 检查是否注销成功
        assert!(!dispatcher.handlers.contains_key("test_app"));
    }
}
