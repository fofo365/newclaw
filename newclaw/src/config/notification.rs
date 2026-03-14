//! Configuration change notification system
//!
//! This module provides a publish/subscribe pattern for configuration changes:
//! - Subscribe to configuration changes
//! - Receive notifications when configurations are updated
//! - Filter notifications by scope or type

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use super::types::Config;
use super::layers::ConfigScope;
use super::merge::ConfigDiff;
use super::hot_reload::HotReloadEvent;

/// Configuration change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeEvent {
    /// Unique event identifier
    pub id: Uuid,
    
    /// Scope that changed
    pub scope: ConfigScope,
    
    /// Type of change
    pub change_type: ChangeType,
    
    /// Old configuration (if applicable)
    pub old_config: Option<Config>,
    
    /// New configuration (if applicable)
    pub new_config: Option<Config>,
    
    /// Configuration diff
    pub diff: Option<ConfigDiff>,
    
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    
    /// Source of the change
    pub source: String,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ConfigChangeEvent {
    /// Create a new change event
    pub fn new(scope: ConfigScope, change_type: ChangeType) -> Self {
        Self {
            id: Uuid::new_v4(),
            scope,
            change_type,
            old_config: None,
            new_config: None,
            diff: None,
            timestamp: Utc::now(),
            source: "unknown".to_string(),
            metadata: HashMap::new(),
        }
    }
    
    /// Set the old configuration
    pub fn with_old_config(mut self, config: Config) -> Self {
        self.old_config = Some(config);
        self
    }
    
    /// Set the new configuration
    pub fn with_new_config(mut self, config: Config) -> Self {
        self.new_config = Some(config);
        self
    }
    
    /// Set the diff
    pub fn with_diff(mut self, diff: ConfigDiff) -> Self {
        self.diff = Some(diff);
        self
    }
    
    /// Set the source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Type of configuration change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Configuration was created
    Created,
    
    /// Configuration was updated
    Updated,
    
    /// Configuration was deleted
    Deleted,
    
    /// Configuration was rolled back
    RolledBack,
    
    /// Configuration was imported
    Imported,
    
    /// Configuration was exported
    Exported,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Updated => write!(f, "updated"),
            Self::Deleted => write!(f, "deleted"),
            Self::RolledBack => write!(f, "rolled_back"),
            Self::Imported => write!(f, "imported"),
            Self::Exported => write!(f, "exported"),
        }
    }
}

/// Subscriber filter for configuration changes
#[derive(Debug, Clone, Default)]
pub struct SubscriberFilter {
    /// Filter by scope (None = all scopes)
    pub scope: Option<ConfigScope>,
    
    /// Filter by change type (None = all types)
    pub change_types: Option<Vec<ChangeType>>,
    
    /// Filter by source (None = all sources)
    pub source: Option<String>,
}

impl SubscriberFilter {
    /// Create a new filter
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Filter by scope
    pub fn with_scope(mut self, scope: ConfigScope) -> Self {
        self.scope = Some(scope);
        self
    }
    
    /// Filter by change types
    pub fn with_change_types(mut self, types: Vec<ChangeType>) -> Self {
        self.change_types = Some(types);
        self
    }
    
    /// Filter by source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    
    /// Check if an event matches this filter
    pub fn matches(&self, event: &ConfigChangeEvent) -> bool {
        // Check scope
        if let Some(ref scope) = self.scope {
            if &event.scope != scope {
                return false;
            }
        }
        
        // Check change types
        if let Some(ref types) = self.change_types {
            if !types.contains(&event.change_type) {
                return false;
            }
        }
        
        // Check source
        if let Some(ref source) = self.source {
            if &event.source != source {
                return false;
            }
        }
        
        true
    }
}

/// Subscriber information
#[derive(Debug)]
pub struct ConfigSubscriber {
    /// Subscriber identifier
    pub id: Uuid,
    
    /// Subscriber name
    pub name: String,
    
    /// Event receiver
    receiver: broadcast::Receiver<ConfigChangeEvent>,
    
    /// Filter
    pub filter: SubscriberFilter,
    
    /// Created at
    pub created_at: DateTime<Utc>,
    
    /// Last activity
    pub last_activity: DateTime<Utc>,
    
    /// Event count
    pub event_count: u64,
}

impl Clone for ConfigSubscriber {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            receiver: self.receiver.resubscribe(),
            filter: self.filter.clone(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            event_count: self.event_count,
        }
    }
}

impl ConfigSubscriber {
    /// Receive the next event (non-blocking)
    pub fn try_recv(&mut self) -> Option<ConfigChangeEvent> {
        match self.receiver.try_recv() {
            Ok(event) => {
                self.last_activity = Utc::now();
                self.event_count += 1;
                Some(event)
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => None,
            Err(broadcast::error::TryRecvError::Lagged(_)) => {
                // Continue receiving
                None
            }
        }
    }
    
    /// Receive the next event (blocking with timeout)
    pub async fn recv_timeout(&mut self, timeout: Duration) -> Option<ConfigChangeEvent> {
        let start = std::time::Instant::now();
        loop {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return None;
            }
            
            match tokio::time::timeout(remaining, self.receiver.recv()).await {
                Ok(Ok(event)) => {
                    self.last_activity = Utc::now();
                    self.event_count += 1;
                    
                    // Apply filter
                    if self.filter.matches(&event) {
                        return Some(event);
                    }
                    // Continue waiting for matching event
                }
                _ => return None,
            }
        }
    }
    
    /// Receive the next event (blocking)
    pub fn recv(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<ConfigChangeEvent>> + Send + '_>> {
        Box::pin(async move {
            loop {
                match self.receiver.recv().await {
                    Ok(event) => {
                        self.last_activity = Utc::now();
                        self.event_count += 1;
                        
                        // Apply filter
                        if self.filter.matches(&event) {
                            return Some(event);
                        }
                        // Continue waiting for matching event
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Continue trying
                        continue;
                    }
                }
            }
        })
    }
}

/// Configuration notification manager
pub struct ConfigNotificationManager {
    /// Event broadcaster
    event_tx: broadcast::Sender<ConfigChangeEvent>,
    
    /// Subscribers
    subscribers: Arc<RwLock<HashMap<Uuid, ConfigSubscriber>>>,
    
    /// Event history
    history: Arc<RwLock<Vec<ConfigChangeEvent>>>,
    
    /// Maximum history size
    max_history_size: usize,
    
    /// Broadcast capacity
    capacity: usize,
}

impl ConfigNotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }
    
    /// Create with custom broadcast capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (event_tx, _) = broadcast::channel(capacity);
        
        Self {
            event_tx,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 100,
            capacity,
        }
    }
    
    /// Subscribe to all configuration changes
    pub async fn subscribe(&self, name: impl Into<String>) -> ConfigSubscriber {
        self.subscribe_with_filter(name, SubscriberFilter::new()).await
    }
    
    /// Subscribe with a filter
    pub async fn subscribe_with_filter(&self, name: impl Into<String>, filter: SubscriberFilter) -> ConfigSubscriber {
        let id = Uuid::new_v4();
        let receiver = self.event_tx.subscribe();
        
        let subscriber = ConfigSubscriber {
            id,
            name: name.into(),
            receiver,
            filter,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            event_count: 0,
        };
        
        {
            let mut subscribers = self.subscribers.write().await;
            subscribers.insert(id, subscriber.clone());
        }
        
        subscriber
    }
    
    /// Unsubscribe
    pub async fn unsubscribe(&self, id: &Uuid) -> bool {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(id).is_some()
    }
    
    /// Publish a configuration change event
    pub async fn publish(&self, event: ConfigChangeEvent) {
        // Add to history
        {
            let mut history = self.history.write().await;
            if history.len() >= self.max_history_size {
                history.remove(0);
            }
            history.push(event.clone());
        }
        
        // Broadcast to all subscribers
        // Note: broadcast::Sender::send doesn't return an error for no receivers
        let _ = self.event_tx.send(event);
    }
    
    /// Publish from a hot reload event
    pub async fn publish_from_hot_reload(&self, event: &HotReloadEvent) {
        match event {
            HotReloadEvent::Reloaded { scope, diff, new_version, .. } => {
                let event = ConfigChangeEvent::new(
                    scope.clone(),
                    ChangeType::Updated,
                )
                .with_source("hot_reload")
                .with_metadata("version_id", new_version.to_string())
                .with_diff(diff.clone());
                
                self.publish(event).await;
            }
            
            HotReloadEvent::RolledBack { scope, to_version, .. } => {
                let event = ConfigChangeEvent::new(
                    scope.clone(),
                    ChangeType::RolledBack,
                )
                .with_source("hot_reload")
                .with_metadata("version_id", to_version.to_string());
                
                self.publish(event).await;
            }
            
            HotReloadEvent::VersionSaved { scope, version } => {
                let event = ConfigChangeEvent::new(
                    scope.clone(),
                    ChangeType::Created,
                )
                .with_source("hot_reload")
                .with_metadata("version_id", version.to_string());
                
                self.publish(event).await;
            }
            
            HotReloadEvent::Error { scope, error } => {
                let scope = scope.clone().unwrap_or(ConfigScope::Global);
                let event = ConfigChangeEvent::new(
                    scope,
                    ChangeType::Updated,
                )
                .with_source("hot_reload")
                .with_metadata("error", error.clone());
                
                self.publish(event).await;
            }
        }
    }
    
    /// Get event history
    pub async fn get_history(&self) -> Vec<ConfigChangeEvent> {
        let history = self.history.read().await;
        history.clone()
    }
    
    /// Get events matching a filter
    pub async fn get_history_filtered(&self, filter: &SubscriberFilter) -> Vec<ConfigChangeEvent> {
        let history = self.history.read().await;
        history.iter()
            .filter(|e| filter.matches(e))
            .cloned()
            .collect()
    }
    
    /// Get subscriber count
    pub async fn subscriber_count(&self) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers.len()
    }
    
    /// Get subscriber info
    pub async fn get_subscriber(&self, id: &Uuid) -> Option<ConfigSubscriber> {
        let subscribers = self.subscribers.read().await;
        subscribers.get(id).cloned()
    }
    
    /// List all subscribers
    pub async fn list_subscribers(&self) -> Vec<(Uuid, String)> {
        let subscribers = self.subscribers.read().await;
        subscribers.iter()
            .map(|(id, s)| (*id, s.name.clone()))
            .collect()
    }
    
    /// Clear event history
    pub async fn clear_history(&self) {
        let mut history = self.history.write().await;
        history.clear();
    }
    
    /// Get statistics
    pub async fn stats(&self) -> NotificationStats {
        let subscribers = self.subscribers.read().await;
        let history = self.history.read().await;
        
        NotificationStats {
            subscriber_count: subscribers.len(),
            total_events_sent: subscribers.values().map(|s| s.event_count).sum(),
            history_size: history.len(),
            max_history_size: self.max_history_size,
        }
    }
}

impl Default for ConfigNotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification statistics
#[derive(Debug, Clone)]
pub struct NotificationStats {
    /// Number of active subscribers
    pub subscriber_count: usize,
    
    /// Total events sent to all subscribers
    pub total_events_sent: u64,
    
    /// Current history size
    pub history_size: usize,
    
    /// Maximum history size
    pub max_history_size: usize,
}

/// Async notification handler trait
#[async_trait::async_trait]
pub trait NotificationHandler: Send + Sync {
    /// Handle a configuration change event
    async fn handle(&self, event: &ConfigChangeEvent);
    
    /// Get the handler name
    fn name(&self) -> &str;
    
    /// Get the filter for this handler
    fn filter(&self) -> SubscriberFilter {
        SubscriberFilter::new()
    }
}

/// Notification worker that processes events in the background
pub struct NotificationWorker {
    /// Notification manager
    manager: Arc<ConfigNotificationManager>,
    
    /// Handlers
    handlers: Vec<Arc<dyn NotificationHandler>>,
    
    /// Shutdown sender
    shutdown_tx: mpsc::Sender<()>,
    
    /// Shutdown receiver
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl NotificationWorker {
    /// Create a new notification worker
    pub fn new(manager: Arc<ConfigNotificationManager>) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        Self {
            manager,
            handlers: Vec::new(),
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
        }
    }
    
    /// Add a handler
    pub fn add_handler(&mut self, handler: Arc<dyn NotificationHandler>) {
        self.handlers.push(handler);
    }
    
    /// Start the worker
    pub async fn start(&mut self) {
        let mut subscriber = self.manager.subscribe("worker").await;
        let mut shutdown_rx = self.shutdown_rx.take().unwrap();
        let handlers = self.handlers.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Check for shutdown
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    
                    // Receive events
                    event = subscriber.recv() => {
                        if let Some(event) = event {
                            // Process with each handler
                            for handler in &handlers {
                                if handler.filter().matches(&event) {
                                    handler.handle(&event).await;
                                }
                            }
                        }
                    }
                }
            }
        });
    }
    
    /// Stop the worker
    pub async fn stop(&self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_subscribe() {
        let manager = ConfigNotificationManager::new();
        let subscriber = manager.subscribe("test").await;
        
        assert!(!subscriber.id.is_nil());
        assert_eq!(subscriber.name, "test");
    }
    
    #[tokio::test]
    async fn test_publish_and_receive() {
        let manager = ConfigNotificationManager::new();
        let mut subscriber = manager.subscribe("test").await;
        
        let event = ConfigChangeEvent::new(
            ConfigScope::Global,
            ChangeType::Updated,
        )
        .with_source("test");
        
        manager.publish(event.clone()).await;
        
        let received = subscriber.recv_timeout(Duration::from_millis(100)).await;
        
        assert!(received.is_some());
        let received = received.unwrap();
        assert_eq!(received.change_type, ChangeType::Updated);
        assert_eq!(received.source, "test");
    }
    
    #[tokio::test]
    async fn test_filter_by_scope() {
        let manager = ConfigNotificationManager::new();
        
        let filter = SubscriberFilter::new()
            .with_scope(ConfigScope::Agent("default".into()));
        
        let mut subscriber = manager.subscribe_with_filter("test", filter).await;
        
        // Publish event for different scope
        let event1 = ConfigChangeEvent::new(
            ConfigScope::Global,
            ChangeType::Updated,
        );
        manager.publish(event1).await;
        
        // Publish event for matching scope
        let event2 = ConfigChangeEvent::new(
            ConfigScope::Agent("default".into()),
            ChangeType::Updated,
        );
        manager.publish(event2.clone()).await;
        
        // Should only receive event2
        let received = subscriber.recv_timeout(Duration::from_millis(100)).await;
        assert!(received.is_some());
        assert_eq!(received.unwrap().scope, ConfigScope::Agent("default".into()));
    }
    
    #[tokio::test]
    async fn test_filter_by_change_type() {
        let manager = ConfigNotificationManager::new();
        
        let filter = SubscriberFilter::new()
            .with_change_types(vec![ChangeType::Created, ChangeType::Deleted]);
        
        let mut subscriber = manager.subscribe_with_filter("test", filter).await;
        
        // Publish updated event
        let event1 = ConfigChangeEvent::new(ConfigScope::Global, ChangeType::Updated);
        manager.publish(event1).await;
        
        // Publish created event
        let event2 = ConfigChangeEvent::new(ConfigScope::Global, ChangeType::Created);
        manager.publish(event2.clone()).await;
        
        // Should only receive created event
        let received = subscriber.recv_timeout(Duration::from_millis(100)).await;
        assert!(received.is_some());
        assert_eq!(received.unwrap().change_type, ChangeType::Created);
    }
    
    #[tokio::test]
    async fn test_unsubscribe() {
        let manager = ConfigNotificationManager::new();
        
        let subscriber = manager.subscribe("test").await;
        let id = subscriber.id;
        
        assert_eq!(manager.subscriber_count().await, 1);
        
        manager.unsubscribe(&id).await;
        
        assert_eq!(manager.subscriber_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_history() {
        let manager = ConfigNotificationManager::new();
        
        // Publish some events
        for i in 0..5 {
            let event = ConfigChangeEvent::new(
                ConfigScope::Global,
                ChangeType::Updated,
            )
            .with_metadata("index", i.to_string());
            manager.publish(event).await;
        }
        
        let history = manager.get_history().await;
        assert_eq!(history.len(), 5);
    }
    
    #[tokio::test]
    async fn test_stats() {
        let manager = ConfigNotificationManager::new();
        
        let mut subscriber = manager.subscribe("test").await;
        
        for _ in 0..3 {
            let event = ConfigChangeEvent::new(ConfigScope::Global, ChangeType::Updated);
            manager.publish(event).await;
        }
        
        // Let the subscriber receive events
        for _ in 0..3 {
            let _ = subscriber.recv_timeout(Duration::from_millis(100)).await;
        }
        
        let stats = manager.stats().await;
        assert_eq!(stats.subscriber_count, 1);
        assert_eq!(stats.history_size, 3);
    }
    
    #[test]
    fn test_change_type_display() {
        assert_eq!(ChangeType::Created.to_string(), "created");
        assert_eq!(ChangeType::Updated.to_string(), "updated");
        assert_eq!(ChangeType::Deleted.to_string(), "deleted");
        assert_eq!(ChangeType::RolledBack.to_string(), "rolled_back");
    }
    
    #[test]
    fn test_subscriber_filter_matches() {
        let filter = SubscriberFilter::new()
            .with_scope(ConfigScope::Global)
            .with_change_types(vec![ChangeType::Updated]);
        
        let event1 = ConfigChangeEvent::new(ConfigScope::Global, ChangeType::Updated);
        let event2 = ConfigChangeEvent::new(ConfigScope::Global, ChangeType::Created);
        let event3 = ConfigChangeEvent::new(
            ConfigScope::Agent("default".into()),
            ChangeType::Updated,
        );
        
        assert!(filter.matches(&event1));
        assert!(!filter.matches(&event2)); // Wrong change type
        assert!(!filter.matches(&event3)); // Wrong scope
    }
}