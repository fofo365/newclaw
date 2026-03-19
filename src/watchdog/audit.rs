// 审计日志模块

use std::sync::RwLock;
use std::collections::VecDeque;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::config::AuditConfig;

/// 审计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件 ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 事件类型
    pub event_type: EventType,
    /// 组件
    pub component: String,
    /// 详情
    pub details: String,
    /// 相关租约 ID
    pub lease_id: Option<String>,
    /// 相关恢复 ID
    pub recovery_id: Option<String>,
}

impl AuditEvent {
    pub fn new(event_type: EventType, component: String, details: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            component,
            details,
            lease_id: None,
            recovery_id: None,
        }
    }
    
    pub fn with_lease(mut self, lease_id: String) -> Self {
        self.lease_id = Some(lease_id);
        self
    }
    
    pub fn with_recovery(mut self, recovery_id: String) -> Self {
        self.recovery_id = Some(recovery_id);
        self
    }
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// 心跳正常
    HeartbeatOk,
    /// 心跳失败
    HeartbeatFailed,
    /// 租约获取
    LeaseAcquired,
    /// 租约续约
    LeaseRenewed,
    /// 租约释放
    LeaseReleased,
    /// 租约过期
    LeaseExpired,
    /// 恢复触发
    RecoveryTriggered,
    /// 恢复开始
    RecoveryStarted,
    /// 恢复成功
    RecoverySucceeded,
    /// 恢复失败
    RecoveryFailed,
    /// 进入安全模式
    SafeModeEntered,
    /// 退出安全模式
    SafeModeExited,
    /// 降级模式
    DegradedMode,
    /// 人工介入
    HumanIntervention,
    /// AI 诊断开始
    AiDiagnosisStarted,
    /// AI 诊断完成
    AiDiagnosisCompleted,
    /// 配置变更
    ConfigChanged,
}

/// 审计日志器
pub struct AuditLogger {
    config: AuditConfig,
    events: RwLock<VecDeque<AuditEvent>>,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config,
            events: RwLock::new(VecDeque::with_capacity(1000)),
        }
    }
    
    /// 记录事件
    pub fn log(&self, event: AuditEvent) -> anyhow::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }
        
        let mut events = self.events.write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        // 如果超过容量，移除最旧的事件
        if events.len() >= 1000 {
            events.pop_front();
        }
        
        events.push_back(event);
        Ok(())
    }
    
    /// 快捷方法：记录心跳事件
    pub fn log_heartbeat(&self, component: &str, success: bool, details: String) -> anyhow::Result<()> {
        let event_type = if success {
            EventType::HeartbeatOk
        } else {
            EventType::HeartbeatFailed
        };
        
        self.log(AuditEvent::new(event_type, component.to_string(), details))
    }
    
    /// 快捷方法：记录租约事件
    pub fn log_lease(&self, event_type: EventType, lease_id: &str, details: String) -> anyhow::Result<()> {
        self.log(AuditEvent::new(event_type, "lease_manager".to_string(), details)
            .with_lease(lease_id.to_string()))
    }
    
    /// 快捷方法：记录恢复事件
    pub fn log_recovery(&self, event_type: EventType, recovery_id: &str, details: String) -> anyhow::Result<()> {
        self.log(AuditEvent::new(event_type, "recovery_executor".to_string(), details)
            .with_recovery(recovery_id.to_string()))
    }
    
    /// 获取所有事件
    pub fn get_events(&self) -> anyhow::Result<Vec<AuditEvent>> {
        let events = self.events.read()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        Ok(events.iter().cloned().collect())
    }
    
    /// 获取最近 N 个事件
    pub fn get_recent(&self, n: usize) -> anyhow::Result<Vec<AuditEvent>> {
        let events = self.events.read()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        let start = if events.len() > n {
            events.len() - n
        } else {
            0
        };
        
        Ok(events.iter().skip(start).cloned().collect())
    }
    
    /// 按类型筛选事件
    pub fn filter_by_type(&self, event_type: EventType) -> anyhow::Result<Vec<AuditEvent>> {
        let events = self.events.read()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        
        Ok(events.iter()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(&event_type))
            .cloned()
            .collect())
    }
    
    /// 清空事件
    pub fn clear(&self) -> anyhow::Result<()> {
        let mut events = self.events.write()
            .map_err(|e| anyhow::anyhow!("Lock error: {}", e))?;
        events.clear();
        Ok(())
    }
    
    /// 导出为 JSON
    pub fn export_json(&self) -> anyhow::Result<String> {
        let events = self.get_events()?;
        serde_json::to_string_pretty(&events)
            .map_err(|e| anyhow::anyhow!("JSON error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(
            EventType::HeartbeatOk,
            "smart_controller".to_string(),
            "Heartbeat OK".to_string(),
        );
        
        assert!(!event.id.is_empty());
        assert!(event.lease_id.is_none());
    }
    
    #[test]
    fn test_audit_event_with_lease() {
        let event = AuditEvent::new(
            EventType::LeaseAcquired,
            "lease_manager".to_string(),
            "Lease acquired".to_string(),
        ).with_lease("lease-123".to_string());
        
        assert_eq!(event.lease_id, Some("lease-123".to_string()));
    }
    
    #[test]
    fn test_audit_logger_log() {
        let config = AuditConfig::default();
        let logger = AuditLogger::new(config);
        
        let event = AuditEvent::new(
            EventType::HeartbeatOk,
            "test".to_string(),
            "OK".to_string(),
        );
        
        logger.log(event).unwrap();
        let events = logger.get_events().unwrap();
        assert_eq!(events.len(), 1);
    }
    
    #[test]
    fn test_audit_logger_log_heartbeat() {
        let config = AuditConfig::default();
        let logger = AuditLogger::new(config);
        
        logger.log_heartbeat("smart", true, "OK".to_string()).unwrap();
        logger.log_heartbeat("smart", false, "Failed".to_string()).unwrap();
        
        let events = logger.get_events().unwrap();
        assert_eq!(events.len(), 2);
    }
    
    #[test]
    fn test_audit_logger_disabled() {
        let config = AuditConfig {
            enabled: false,
            ..Default::default()
        };
        let logger = AuditLogger::new(config);
        
        let event = AuditEvent::new(
            EventType::HeartbeatOk,
            "test".to_string(),
            "OK".to_string(),
        );
        
        logger.log(event).unwrap();
        let events = logger.get_events().unwrap();
        assert_eq!(events.len(), 0);
    }
    
    #[test]
    fn test_audit_logger_get_recent() {
        let config = AuditConfig::default();
        let logger = AuditLogger::new(config);
        
        for i in 0..10 {
            logger.log(AuditEvent::new(
                EventType::HeartbeatOk,
                "test".to_string(),
                format!("Event {}", i),
            )).unwrap();
        }
        
        let recent = logger.get_recent(3).unwrap();
        assert_eq!(recent.len(), 3);
    }
}
