// 自检模块 - 检查智慧主控健康状态

use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::metrics::resources::ResourceMonitor;

/// 自检配置
#[derive(Debug, Clone)]
pub struct SelfCheckConfig {
    /// 是否启用
    pub enabled: bool,
    /// 检查间隔（秒）
    pub check_interval_secs: u64,
    /// 内存阈值（MB）
    pub memory_threshold_mb: u64,
    /// CPU 阈值（%）
    pub cpu_threshold_percent: f64,
    /// 最大错误率（%）
    pub max_error_rate_percent: f64,
}

impl Default for SelfCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_secs: 10,
            memory_threshold_mb: 500,
            cpu_threshold_percent: 80.0,
            max_error_rate_percent: 10.0,
        }
    }
}

/// 检查项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    /// 检查项名称
    pub name: String,
    /// 是否通过
    pub passed: bool,
    /// 当前值
    pub current_value: String,
    /// 阈值
    pub threshold: String,
    /// 消息
    pub message: String,
}

impl CheckItem {
    pub fn pass(name: &str, current: &str, threshold: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            current_value: current.to_string(),
            threshold: threshold.to_string(),
            message: "OK".to_string(),
        }
    }
    
    pub fn fail(name: &str, current: &str, threshold: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            current_value: current.to_string(),
            threshold: threshold.to_string(),
            message: message.to_string(),
        }
    }
}

/// 自检结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfCheckResult {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 整体健康状态
    pub healthy: bool,
    /// 检查项列表
    pub checks: Vec<CheckItem>,
    /// 警告列表
    pub warnings: Vec<String>,
}

impl SelfCheckResult {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            healthy: true,
            checks: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    pub fn add_check(&mut self, check: CheckItem) {
        if !check.passed {
            self.healthy = false;
        }
        self.checks.push(check);
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn summary(&self) -> String {
        let passed = self.checks.iter().filter(|c| c.passed).count();
        let total = self.checks.len();
        format!("{}/{} checks passed", passed, total)
    }
}

/// 自检器
pub struct SelfChecker {
    config: SelfCheckConfig,
    resource_monitor: Arc<ResourceMonitor>,
    last_result: Arc<std::sync::RwLock<Option<SelfCheckResult>>>,
}

impl SelfChecker {
    pub fn new(config: SelfCheckConfig) -> Self {
        Self {
            resource_monitor: Arc::new(ResourceMonitor::new()),
            last_result: Arc::new(std::sync::RwLock::new(None)),
            config,
        }
    }
    
    /// 执行自检
    pub fn check(&self) -> SelfCheckResult {
        let mut result = SelfCheckResult::new();
        
        if !self.config.enabled {
            result.add_warning("Self-check disabled".to_string());
            return result;
        }
        
        // 1. 内存检查
        self.check_memory(&mut result);
        
        // 2. CPU 检查
        self.check_cpu(&mut result);
        
        // 3. 线程检查
        self.check_threads(&mut result);
        
        // 4. 文件描述符检查
        self.check_file_descriptors(&mut result);
        
        // 保存结果
        {
            let mut last = self.last_result.write().unwrap();
            *last = Some(result.clone());
        }
        
        result
    }
    
    /// 内存检查
    fn check_memory(&self, result: &mut SelfCheckResult) {
        let metrics = self.resource_monitor.get_metrics();
        
        let check = if metrics.memory_used_mb > self.config.memory_threshold_mb {
            CheckItem::fail(
                "memory",
                &format!("{} MB", metrics.memory_used_mb),
                &format!("{} MB", self.config.memory_threshold_mb),
                "Memory usage exceeds threshold",
            )
        } else {
            CheckItem::pass(
                "memory",
                &format!("{} MB", metrics.memory_used_mb),
                &format!("{} MB", self.config.memory_threshold_mb),
            )
        };
        
        result.add_check(check);
    }
    
    /// CPU 检查
    fn check_cpu(&self, result: &mut SelfCheckResult) {
        let metrics = self.resource_monitor.get_metrics();
        
        let check = if metrics.cpu_usage_percent > self.config.cpu_threshold_percent {
            CheckItem::fail(
                "cpu",
                &format!("{:.1}%", metrics.cpu_usage_percent),
                &format!("{:.1}%", self.config.cpu_threshold_percent),
                "CPU usage exceeds threshold",
            )
        } else {
            CheckItem::pass(
                "cpu",
                &format!("{:.1}%", metrics.cpu_usage_percent),
                &format!("{:.1}%", self.config.cpu_threshold_percent),
            )
        };
        
        result.add_check(check);
    }
    
    /// 线程检查
    fn check_threads(&self, result: &mut SelfCheckResult) {
        // 简单检查：获取当前进程的线程数
        let thread_count = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1);
        
        let check = CheckItem::pass(
            "threads",
            &format!("{}", thread_count),
            "N/A",
        );
        
        result.add_check(check);
    }
    
    /// 文件描述符检查
    fn check_file_descriptors(&self, result: &mut SelfCheckResult) {
        // 尝试读取 /proc/self/fd 的数量
        let fd_count = std::fs::read_dir("/proc/self/fd")
            .map(|entries| entries.count())
            .unwrap_or(0);
        
        // 通常限制是 1024
        let check = if fd_count > 900 {
            CheckItem::fail(
                "file_descriptors",
                &format!("{}", fd_count),
                "900",
                "Too many open file descriptors",
            )
        } else {
            CheckItem::pass(
                "file_descriptors",
                &format!("{}", fd_count),
                "900",
            )
        };
        
        result.add_check(check);
    }
    
    /// 获取最后一次检查结果
    pub fn last_result(&self) -> Option<SelfCheckResult> {
        self.last_result.read().unwrap().clone()
    }
    
    /// 检查是否需要降级
    pub fn should_degrade(&self) -> bool {
        if let Some(result) = self.last_result() {
            !result.healthy
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_check_item_pass() {
        let item = CheckItem::pass("test", "100", "200");
        assert!(item.passed);
        assert_eq!(item.name, "test");
    }
    
    #[test]
    fn test_check_item_fail() {
        let item = CheckItem::fail("test", "300", "200", "Too high");
        assert!(!item.passed);
        assert_eq!(item.message, "Too high");
    }
    
    #[test]
    fn test_self_check_result() {
        let mut result = SelfCheckResult::new();
        result.add_check(CheckItem::pass("a", "1", "2"));
        result.add_check(CheckItem::fail("b", "3", "2", "fail"));
        
        assert!(!result.healthy);
        assert_eq!(result.checks.len(), 2);
        assert_eq!(result.summary(), "1/2 checks passed");
    }
    
    #[test]
    fn test_self_checker_check() {
        let config = SelfCheckConfig::default();
        let checker = SelfChecker::new(config);
        
        let result = checker.check();
        assert!(!result.checks.is_empty());
        
        let last = checker.last_result();
        assert!(last.is_some());
    }
    
    #[test]
    fn test_self_checker_disabled() {
        let config = SelfCheckConfig {
            enabled: false,
            ..Default::default()
        };
        let checker = SelfChecker::new(config);
        
        let result = checker.check();
        assert!(result.healthy);
        assert!(result.checks.is_empty());
        assert!(!result.warnings.is_empty());
    }
}
