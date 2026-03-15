// 资源监控模块 (v0.5.5)
//
// CPU/内存/磁盘监控

use sysinfo::System;
use std::sync::{Arc, RwLock};

/// 资源指标
#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub cpu_usage_percent: f64,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub memory_usage_percent: f64,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_used_mb: 0,
            memory_total_mb: 0,
            memory_usage_percent: 0.0,
        }
    }
}

/// 资源监控器
pub struct ResourceMonitor {
    sys: RwLock<System>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { sys: RwLock::new(sys) }
    }
    
    pub fn get_metrics(&self) -> ResourceMetrics {
        let mut sys = self.sys.write().unwrap();
        sys.refresh_all();
        
        let cpu_usage = sys.global_cpu_usage();
        let memory_total = sys.total_memory() / 1024 / 1024;
        let memory_used = sys.used_memory() / 1024 / 1024;
        let memory_usage = if memory_total > 0 {
            (memory_used as f64 / memory_total as f64) * 100.0
        } else { 0.0 };
        
        ResourceMetrics {
            cpu_usage_percent: cpu_usage as f64,
            memory_used_mb: memory_used,
            memory_total_mb: memory_total,
            memory_usage_percent: memory_usage,
        }
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_monitor() {
        let monitor = ResourceMonitor::new();
        let metrics = monitor.get_metrics();
        assert!(metrics.memory_total_mb > 0);
    }
}
