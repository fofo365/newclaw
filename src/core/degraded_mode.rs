// 降级模式 - 故障时限制功能

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::HashSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 降级模式配置
#[derive(Debug, Clone)]
pub struct DegradedModeConfig {
    /// 是否启用
    pub enabled: bool,
    /// 最大并发请求数
    pub max_concurrent_requests: u64,
    /// 禁用的功能列表
    pub disabled_features: Vec<String>,
    /// 自动恢复检查间隔（秒）
    pub auto_recovery_interval_secs: u64,
}

impl Default for DegradedModeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_requests: 5,
            disabled_features: vec![
                "web_search".to_string(),
                "browser".to_string(),
                "file_operations".to_string(),
            ],
            auto_recovery_interval_secs: 60,
        }
    }
}

/// 降级状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradedState {
    /// 是否处于降级模式
    pub is_degraded: bool,
    /// 进入降级时间
    pub degraded_since: Option<DateTime<Utc>>,
    /// 降级原因
    pub reason: String,
    /// 当前并发请求数
    pub current_requests: u64,
    /// 被拒绝的请求数
    pub rejected_requests: u64,
}

impl Default for DegradedState {
    fn default() -> Self {
        Self {
            is_degraded: false,
            degraded_since: None,
            reason: String::new(),
            current_requests: 0,
            rejected_requests: 0,
        }
    }
}

/// 降级模式管理器
pub struct DegradedModeManager {
    config: DegradedModeConfig,
    is_degraded: Arc<AtomicBool>,
    degraded_since: Arc<std::sync::RwLock<Option<DateTime<Utc>>>>,
    reason: Arc<std::sync::RwLock<String>>,
    current_requests: Arc<AtomicU64>,
    rejected_requests: Arc<AtomicU64>,
    disabled_features: Arc<std::sync::RwLock<HashSet<String>>>,
}

impl DegradedModeManager {
    pub fn new(config: DegradedModeConfig) -> Self {
        let disabled_features: HashSet<String> = config.disabled_features.iter().cloned().collect();
        
        Self {
            is_degraded: Arc::new(AtomicBool::new(false)),
            degraded_since: Arc::new(std::sync::RwLock::new(None)),
            reason: Arc::new(std::sync::RwLock::new(String::new())),
            current_requests: Arc::new(AtomicU64::new(0)),
            rejected_requests: Arc::new(AtomicU64::new(0)),
            disabled_features: Arc::new(std::sync::RwLock::new(disabled_features)),
            config,
        }
    }
    
    /// 进入降级模式
    pub fn enter(&self, reason: &str) {
        if !self.config.enabled {
            return;
        }
        
        self.is_degraded.store(true, Ordering::SeqCst);
        *self.degraded_since.write().unwrap() = Some(Utc::now());
        *self.reason.write().unwrap() = reason.to_string();
        
        tracing::warn!("Entered degraded mode: {}", reason);
    }
    
    /// 退出降级模式
    pub fn exit(&self) {
        self.is_degraded.store(false, Ordering::SeqCst);
        *self.degraded_since.write().unwrap() = None;
        *self.reason.write().unwrap() = String::new();
        
        tracing::info!("Exited degraded mode");
    }
    
    /// 检查是否处于降级模式
    pub fn is_degraded(&self) -> bool {
        self.is_degraded.load(Ordering::SeqCst)
    }
    
    /// 尝试获取请求槽位
    /// 返回 true 表示可以处理，false 表示应拒绝
    pub fn try_acquire(&self) -> bool {
        if !self.is_degraded() {
            return true;
        }
        
        let current = self.current_requests.load(Ordering::SeqCst);
        if current >= self.config.max_concurrent_requests {
            self.rejected_requests.fetch_add(1, Ordering::SeqCst);
            return false;
        }
        
        self.current_requests.fetch_add(1, Ordering::SeqCst);
        true
    }
    
    /// 释放请求槽位
    pub fn release(&self) {
        self.current_requests.fetch_sub(1, Ordering::SeqCst);
    }
    
    /// 检查功能是否可用
    pub fn is_feature_available(&self, feature: &str) -> bool {
        if !self.is_degraded() {
            return true;
        }
        
        let disabled = self.disabled_features.read().unwrap();
        !disabled.contains(feature)
    }
    
    /// 添加禁用的功能
    pub fn disable_feature(&self, feature: &str) {
        let mut disabled = self.disabled_features.write().unwrap();
        disabled.insert(feature.to_string());
    }
    
    /// 启用功能
    pub fn enable_feature(&self, feature: &str) {
        let mut disabled = self.disabled_features.write().unwrap();
        disabled.remove(feature);
    }
    
    /// 获取当前状态
    pub fn get_state(&self) -> DegradedState {
        DegradedState {
            is_degraded: self.is_degraded.load(Ordering::SeqCst),
            degraded_since: *self.degraded_since.read().unwrap(),
            reason: self.reason.read().unwrap().clone(),
            current_requests: self.current_requests.load(Ordering::SeqCst),
            rejected_requests: self.rejected_requests.load(Ordering::SeqCst),
        }
    }
    
    /// 获取降级持续时间
    pub fn degraded_duration(&self) -> Option<std::time::Duration> {
        let since = self.degraded_since.read().unwrap();
        since.map(|s| {
            (Utc::now() - s).to_std().unwrap_or(std::time::Duration::ZERO)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_degraded_mode_manager_creation() {
        let config = DegradedModeConfig::default();
        let manager = DegradedModeManager::new(config);
        
        assert!(!manager.is_degraded());
    }
    
    #[test]
    fn test_enter_exit_degraded() {
        let manager = DegradedModeManager::new(DegradedModeConfig::default());
        
        manager.enter("High memory");
        assert!(manager.is_degraded());
        
        let state = manager.get_state();
        assert!(state.is_degraded);
        assert_eq!(state.reason, "High memory");
        
        manager.exit();
        assert!(!manager.is_degraded());
    }
    
    #[test]
    fn test_acquire_release() {
        let config = DegradedModeConfig {
            max_concurrent_requests: 2,
            ..Default::default()
        };
        let manager = DegradedModeManager::new(config);
        
        // 正常模式，总是可以获取
        assert!(manager.try_acquire());
        manager.release();
        
        // 进入降级模式
        manager.enter("Test");
        
        assert!(manager.try_acquire());
        assert!(manager.try_acquire());
        assert!(!manager.try_acquire()); // 超过限制
        
        manager.release();
        assert!(manager.try_acquire()); // 释放后可以再获取
        
        manager.release();
        manager.release();
    }
    
    #[test]
    fn test_feature_availability() {
        let manager = DegradedModeManager::new(DegradedModeConfig::default());
        
        // 正常模式，所有功能可用
        assert!(manager.is_feature_available("web_search"));
        
        // 降级模式
        manager.enter("Test");
        assert!(!manager.is_feature_available("web_search"));
        assert!(manager.is_feature_available("basic_chat"));
    }
    
    #[test]
    fn test_degraded_duration() {
        let manager = DegradedModeManager::new(DegradedModeConfig::default());
        
        assert!(manager.degraded_duration().is_none());
        
        manager.enter("Test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let duration = manager.degraded_duration();
        assert!(duration.is_some());
        assert!(duration.unwrap() >= std::time::Duration::from_millis(10));
    }
    
    #[test]
    fn test_disabled_mode() {
        let config = DegradedModeConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = DegradedModeManager::new(config);
        
        manager.enter("Test");
        assert!(!manager.is_degraded()); // 禁用时不会进入降级
    }
}
