// 心跳机制模块 (v0.5.5)
//
// 自检、自维护、任务持续

use std::sync::{Arc, RwLock};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::time::interval;

use crate::metrics::resources::ResourceMonitor;
use crate::metrics::prometheus::export_metrics;

/// 心跳配置
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// 心跳间隔（秒）
    pub interval_secs: u64,
    /// 是否启用自检
    pub enable_self_check: bool,
    /// 是否启用自维护
    pub enable_self_maintenance: bool,
    /// 内存阈值（百分比）
    pub memory_threshold_percent: f64,
    /// CPU 阈值（百分比）
    pub cpu_threshold_percent: f64,
    /// 最大历史记录
    pub max_history: usize,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60,
            enable_self_check: true,
            enable_self_maintenance: true,
            memory_threshold_percent: 80.0,
            cpu_threshold_percent: 80.0,
            max_history: 100,
        }
    }
}

/// 心跳状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatStatus {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 是否健康
    pub healthy: bool,
    /// CPU 使用率
    pub cpu_usage: f64,
    /// 内存使用率
    pub memory_usage: f64,
    /// 活跃会话数
    pub active_sessions: usize,
    /// 待处理任务数
    pub pending_tasks: usize,
    /// 警告列表
    pub warnings: Vec<String>,
    /// 执行的自维护操作
    pub maintenance_actions: Vec<String>,
}

/// 心跳管理器
pub struct HeartbeatManager {
    config: HeartbeatConfig,
    resource_monitor: Arc<ResourceMonitor>,
    status_history: RwLock<VecDeque<HeartbeatStatus>>,
    pending_tasks: RwLock<Vec<PendingTask>>,
    last_heartbeat: RwLock<Option<Instant>>,
}

/// 待处理任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTask {
    pub id: String,
    pub description: String,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    pub retry_count: usize,
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl HeartbeatManager {
    pub fn new(config: HeartbeatConfig) -> Self {
        Self {
            config,
            resource_monitor: Arc::new(ResourceMonitor::new()),
            status_history: RwLock::new(VecDeque::with_capacity(100)),
            pending_tasks: RwLock::new(Vec::new()),
            last_heartbeat: RwLock::new(None),
        }
    }
    
    /// 执行心跳检查
    pub async fn check(&self) -> HeartbeatStatus {
        let metrics = self.resource_monitor.get_metrics();
        let mut warnings = Vec::new();
        let mut maintenance_actions = Vec::new();
        
        // 自检
        if self.config.enable_self_check {
            // 检查内存
            if metrics.memory_usage_percent > self.config.memory_threshold_percent {
                warnings.push(format!(
                    "内存使用率过高: {:.1}% > {:.1}%",
                    metrics.memory_usage_percent, self.config.memory_threshold_percent
                ));
            }
            
            // 检查 CPU
            if metrics.cpu_usage_percent > self.config.cpu_threshold_percent {
                warnings.push(format!(
                    "CPU 使用率过高: {:.1}% > {:.1}%",
                    metrics.cpu_usage_percent, self.config.cpu_threshold_percent
                ));
            }
        }
        
        // 自维护
        if self.config.enable_self_maintenance {
            // 清理过期状态历史
            if self.status_history.read().unwrap().len() >= self.config.max_history {
                self.status_history.write().unwrap().pop_front();
                maintenance_actions.push("清理过期状态历史".to_string());
            }
            
            // 重试失败任务
            let retry_count = self.retry_failed_tasks();
            if retry_count > 0 {
                maintenance_actions.push(format!("重试 {} 个失败任务", retry_count));
            }
        }
        
        // 构建状态
        let status = HeartbeatStatus {
            timestamp: Utc::now(),
            healthy: warnings.is_empty(),
            cpu_usage: metrics.cpu_usage_percent,
            memory_usage: metrics.memory_usage_percent,
            active_sessions: 0, // TODO: 从会话管理器获取
            pending_tasks: self.pending_tasks.read().unwrap().len(),
            warnings,
            maintenance_actions,
        };
        
        // 保存状态
        self.status_history.write().unwrap().push_back(status.clone());
        *self.last_heartbeat.write().unwrap() = Some(Instant::now());
        
        status
    }
    
    /// 添加待处理任务
    pub fn add_task(&self, task: PendingTask) {
        self.pending_tasks.write().unwrap().push(task);
    }
    
    /// 完成任务
    pub fn complete_task(&self, task_id: &str) {
        self.pending_tasks.write().unwrap().retain(|t| t.id != task_id);
    }
    
    /// 重试失败任务
    fn retry_failed_tasks(&self) -> usize {
        // TODO: 实现任务重试逻辑
        0
    }
    
    /// 获取最近状态
    pub fn get_recent_status(&self, count: usize) -> Vec<HeartbeatStatus> {
        self.status_history.read().unwrap()
            .iter().rev().take(count).cloned().collect()
    }
    
    /// 启动后台心跳任务
    pub fn start_background(
        manager: Arc<Self>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(manager.config.interval_secs));
            
            loop {
                interval.tick().await;
                let status = manager.check().await;
                
                if !status.healthy {
                    tracing::warn!("心跳检查发现问题: {:?}", status.warnings);
                } else {
                    tracing::debug!("心跳检查正常");
                }
            }
        })
    }
}

/// 自检报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfCheckReport {
    pub timestamp: DateTime<Utc>,
    pub overall_health: HealthLevel,
    pub components: Vec<ComponentHealth>,
    pub recommendations: Vec<String>,
}

/// 健康级别
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
}

/// 组件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: HealthLevel,
    pub message: String,
    pub metrics: serde_json::Value,
}

impl HeartbeatManager {
    /// 生成自检报告
    pub fn generate_report(&self) -> SelfCheckReport {
        let metrics = self.resource_monitor.get_metrics();
        let mut components = Vec::new();
        let mut recommendations = Vec::new();
        
        // 检查内存
        let memory_health = if metrics.memory_usage_percent > 90.0 {
            HealthLevel::Critical
        } else if metrics.memory_usage_percent > 80.0 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        components.push(ComponentHealth {
            name: "memory".to_string(),
            status: memory_health,
            message: format!("Memory usage: {:.1}%", metrics.memory_usage_percent),
            metrics: serde_json::json!({
                "used_mb": metrics.memory_used_mb,
                "total_mb": metrics.memory_total_mb,
                "usage_percent": metrics.memory_usage_percent,
            }),
        });
        
        if matches!(memory_health, HealthLevel::Critical | HealthLevel::Warning) {
            recommendations.push("考虑清理内存或增加系统内存".to_string());
        }
        
        // 检查 CPU
        let cpu_health = if metrics.cpu_usage_percent > 90.0 {
            HealthLevel::Critical
        } else if metrics.cpu_usage_percent > 80.0 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        components.push(ComponentHealth {
            name: "cpu".to_string(),
            status: cpu_health,
            message: format!("CPU usage: {:.1}%", metrics.cpu_usage_percent),
            metrics: serde_json::json!({
                "usage_percent": metrics.cpu_usage_percent,
            }),
        });
        
        // 总体健康状态
        let overall_health = if components.iter().any(|c| matches!(c.status, HealthLevel::Critical)) {
            HealthLevel::Critical
        } else if components.iter().any(|c| matches!(c.status, HealthLevel::Warning)) {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        SelfCheckReport {
            timestamp: Utc::now(),
            overall_health,
            components,
            recommendations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_default() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval_secs, 60);
    }

    #[test]
    fn test_heartbeat_manager_new() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        assert!(manager.get_recent_status(10).is_empty());
    }

    #[tokio::test]
    async fn test_heartbeat_check() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        let status = manager.check().await;
        
        assert!(status.healthy || !status.warnings.is_empty());
        assert!(!manager.get_recent_status(1).is_empty());
    }

    #[test]
    fn test_add_task() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        
        let task = PendingTask {
            id: "task1".to_string(),
            description: "Test task".to_string(),
            priority: TaskPriority::Normal,
            created_at: Utc::now(),
            retry_count: 0,
        };
        
        manager.add_task(task);
        assert_eq!(manager.pending_tasks.read().unwrap().len(), 1);
    }

    #[test]
    fn test_generate_report() {
        let manager = HeartbeatManager::new(HeartbeatConfig::default());
        let report = manager.generate_report();
        
        assert!(!report.components.is_empty());
    }
}
