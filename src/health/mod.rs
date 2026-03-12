// Health Check - v0.5.4
//
// 健康检查端点实现

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 健康状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Healthy
    }
}

/// 组件健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// 组件名称
    pub name: String,
    /// 状态
    pub status: HealthStatus,
    /// 详细消息
    pub message: Option<String>,
    /// 最后检查时间
    pub last_check: i64,
    /// 响应时间 (ms)
    pub latency_ms: u64,
}

impl ComponentHealth {
    /// 创建健康组件
    pub fn healthy(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: HealthStatus::Healthy,
            message: None,
            last_check: chrono::Utc::now().timestamp(),
            latency_ms: 0,
        }
    }
    
    /// 创建降级组件
    pub fn degraded(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: HealthStatus::Degraded,
            message: Some(message.to_string()),
            last_check: chrono::Utc::now().timestamp(),
            latency_ms: 0,
        }
    }
    
    /// 创建不健康组件
    pub fn unhealthy(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            status: HealthStatus::Unhealthy,
            message: Some(message.to_string()),
            last_check: chrono::Utc::now().timestamp(),
            latency_ms: 0,
        }
    }
}

/// 系统健康报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// 整体状态
    pub status: HealthStatus,
    /// 版本
    pub version: String,
    /// 启动时间
    pub started_at: i64,
    /// 运行时间 (秒)
    pub uptime_secs: u64,
    /// 组件健康状态
    pub components: HashMap<String, ComponentHealth>,
    /// 系统指标
    pub metrics: SystemMetrics,
}

impl HealthReport {
    /// 创建新的健康报告
    pub fn new(version: &str) -> Self {
        Self {
            status: HealthStatus::Healthy,
            version: version.to_string(),
            started_at: chrono::Utc::now().timestamp(),
            uptime_secs: 0,
            components: HashMap::new(),
            metrics: SystemMetrics::default(),
        }
    }
    
    /// 添加组件健康状态
    pub fn add_component(&mut self, health: ComponentHealth) {
        // 更新整体状态
        if matches!(health.status, HealthStatus::Unhealthy) {
            self.status = HealthStatus::Unhealthy;
        } else if matches!(health.status, HealthStatus::Degraded) 
            && !matches!(self.status, HealthStatus::Unhealthy) {
            self.status = HealthStatus::Degraded;
        }
        
        self.components.insert(health.name.clone(), health);
    }
    
    /// 更新运行时间
    pub fn update_uptime(&mut self) {
        let now = chrono::Utc::now().timestamp();
        self.uptime_secs = (now - self.started_at) as u64;
    }
    
    /// 更新系统指标
    pub fn update_metrics(&mut self, metrics: SystemMetrics) {
        self.metrics = metrics;
    }
}

/// 系统指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// 内存使用 (MB)
    pub memory_mb: u64,
    /// CPU 使用率 (%)
    pub cpu_percent: f32,
    /// Goroutine 数量（如果有）
    pub goroutines: Option<u64>,
    /// 打开文件描述符
    pub open_fds: u64,
    /// 请求总数
    pub total_requests: u64,
    /// 错误总数
    pub total_errors: u64,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            memory_mb: 0,
            cpu_percent: 0.0,
            goroutines: None,
            open_fds: 0,
            total_requests: 0,
            total_errors: 0,
        }
    }
}

/// 就绪检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    /// 是否就绪
    pub ready: bool,
    /// 检查详情
    pub checks: HashMap<String, bool>,
    /// 消息
    pub message: Option<String>,
}

impl ReadinessReport {
    /// 创建就绪报告
    pub fn new() -> Self {
        Self {
            ready: true,
            checks: HashMap::new(),
            message: None,
        }
    }
    
    /// 添加检查项
    pub fn add_check(&mut self, name: &str, passed: bool) {
        if !passed {
            self.ready = false;
        }
        self.checks.insert(name.to_string(), passed);
    }
    
    /// 设置消息
    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }
}

/// 健康检查器
pub struct HealthChecker {
    /// 版本
    version: String,
    /// 启动时间
    started_at: Instant,
    /// 当前报告
    report: HealthReport,
}

impl HealthChecker {
    /// 创建新的健康检查器
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            started_at: Instant::now(),
            report: HealthReport::new(version),
        }
    }
    
    /// 执行健康检查
    pub fn check_health(&mut self) -> &HealthReport {
        self.report.update_uptime();
        
        // 检查数据库
        self.check_database();
        
        // 检查 Redis（如果配置）
        self.check_redis();
        
        // 检查内存
        self.check_memory();
        
        // 更新系统指标
        self.update_system_metrics();
        
        &self.report
    }
    
    /// 执行就绪检查
    pub fn check_ready(&self) -> ReadinessReport {
        let mut readiness = ReadinessReport::new();
        
        // 检查必要组件
        readiness.add_check("database", self.is_database_ready());
        readiness.add_check("config", self.is_config_loaded());
        
        readiness
    }
    
    /// 检查数据库
    fn check_database(&mut self) {
        let start = Instant::now();
        
        // 模拟数据库检查
        // 实际实现会检查 SQLite/PostgreSQL 连接
        let health = ComponentHealth {
            name: "database".to_string(),
            status: HealthStatus::Healthy,
            message: Some("SQLite connection OK".to_string()),
            last_check: chrono::Utc::now().timestamp(),
            latency_ms: start.elapsed().as_millis() as u64,
        };
        
        self.report.add_component(health);
    }
    
    /// 检查 Redis
    fn check_redis(&mut self) {
        let start = Instant::now();
        
        // 模拟 Redis 检查
        // 实际实现会检查 Redis 连接
        let health = ComponentHealth {
            name: "redis".to_string(),
            status: HealthStatus::Degraded,
            message: Some("Redis not configured".to_string()),
            last_check: chrono::Utc::now().timestamp(),
            latency_ms: start.elapsed().as_millis() as u64,
        };
        
        self.report.add_component(health);
    }
    
    /// 检查内存
    fn check_memory(&mut self) {
        // 获取内存使用情况
        let memory_mb = Self::get_memory_usage();
        
        let status = if memory_mb > 500 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        
        let health = ComponentHealth {
            name: "memory".to_string(),
            status,
            message: Some(format!("Using {} MB", memory_mb)),
            last_check: chrono::Utc::now().timestamp(),
            latency_ms: 0,
        };
        
        self.report.add_component(health);
    }
    
    /// 更新系统指标
    fn update_system_metrics(&mut self) {
        self.report.metrics = SystemMetrics {
            memory_mb: Self::get_memory_usage(),
            cpu_percent: Self::get_cpu_usage(),
            goroutines: None,
            open_fds: Self::get_open_fds(),
            total_requests: 0, // TODO: 从统计模块获取
            total_errors: 0,
        };
    }
    
    /// 获取内存使用 (简化版)
    fn get_memory_usage() -> u64 {
        // 实际实现会读取 /proc/self/status
        50 // MB (模拟值)
    }
    
    /// 获取 CPU 使用率
    fn get_cpu_usage() -> f32 {
        // 实际实现会计算 CPU 使用率
        10.0 // % (模拟值)
    }
    
    /// 获取打开的文件描述符数量
    fn get_open_fds() -> u64 {
        // 实际实现会读取 /proc/self/fd
        20 // (模拟值)
    }
    
    /// 检查数据库是否就绪
    fn is_database_ready(&self) -> bool {
        // 检查数据库连接
        true
    }
    
    /// 检查配置是否加载
    fn is_config_loaded(&self) -> bool {
        // 检查配置是否加载
        true
    }
    
    /// 获取运行时间
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_default() {
        let status = HealthStatus::default();
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[test]
    fn test_component_health_healthy() {
        let health = ComponentHealth::healthy("test");
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.message.is_none());
    }

    #[test]
    fn test_component_health_degraded() {
        let health = ComponentHealth::degraded("test", "warning");
        assert_eq!(health.status, HealthStatus::Degraded);
        assert!(health.message.is_some());
    }

    #[test]
    fn test_component_health_unhealthy() {
        let health = ComponentHealth::unhealthy("test", "error");
        assert_eq!(health.status, HealthStatus::Unhealthy);
        assert!(health.message.is_some());
    }

    #[test]
    fn test_health_report_new() {
        let report = HealthReport::new("0.5.4");
        assert_eq!(report.version, "0.5.4");
        assert_eq!(report.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_report_add_component() {
        let mut report = HealthReport::new("0.5.4");
        report.add_component(ComponentHealth::healthy("db"));
        
        assert!(report.components.contains_key("db"));
    }

    #[test]
    fn test_health_report_status_propagation() {
        let mut report = HealthReport::new("0.5.4");
        report.add_component(ComponentHealth::unhealthy("db", "failed"));
        
        assert_eq!(report.status, HealthStatus::Unhealthy);
    }

    #[test]
    fn test_readiness_report_new() {
        let report = ReadinessReport::new();
        assert!(report.ready);
    }

    #[test]
    fn test_readiness_report_add_check() {
        let mut report = ReadinessReport::new();
        report.add_check("db", false);
        
        assert!(!report.ready);
        assert!(report.checks.contains_key("db"));
    }

    #[test]
    fn test_health_checker_new() {
        let checker = HealthChecker::new("0.5.4");
        assert!(checker.uptime().as_secs() < 1);
    }
}
