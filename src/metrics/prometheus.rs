// 简化的 Prometheus 指标导出模块 (v0.5.5)
//
// 不依赖外部 crate，手动实现 Prometheus 格式

use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

/// 全局指标注册表
static REGISTRY: Lazy<MetricsRegistry> = Lazy::new(|| {
    let mut registry = MetricsRegistry::new();
    register_default_metrics(&mut registry);
    registry
});

/// 注册默认指标
fn register_default_metrics(registry: &mut MetricsRegistry) {
    // 请求指标
    registry.register_counter("newclaw_requests_total", "Total number of requests");
    registry.register_counter("newclaw_requests_success_total", "Number of successful requests");
    registry.register_counter("newclaw_requests_failed_total", "Number of failed requests");
    registry.register_counter("newclaw_http_request_duration_seconds_total", "HTTP request duration in seconds");
    
    // Token 指标
    registry.register_counter("newclaw_tokens_input_total", "Total input tokens");
    registry.register_counter("newclaw_tokens_output_total", "Total output tokens");
    
    // 连接指标
    registry.register_gauge("newclaw_active_connections", "Number of active connections");
    registry.register_gauge("newclaw_active_sessions", "Number of active sessions");
    
    // 资源指标
    registry.register_gauge("newclaw_cpu_usage_percent", "CPU usage percentage");
    registry.register_gauge("newclaw_memory_usage_bytes", "Memory usage in bytes");
    registry.register_gauge("newclaw_memory_total_bytes", "Total memory in bytes");
    
    // LLM 指标
    registry.register_counter("newclaw_llm_requests_total", "Total LLM API requests");
    registry.register_counter("newclaw_llm_errors_total", "Total LLM API errors");
    registry.register_counter("newclaw_llm_request_duration_seconds_total", "LLM request duration in seconds");
    
    // 工具指标
    registry.register_counter("newclaw_tool_executions_total", "Total tool executions");
    registry.register_counter("newclaw_tool_errors_total", "Total tool errors");
    
    // 飞书指标
    registry.register_counter("newclaw_feishu_messages_total", "Total Feishu messages");
    registry.register_counter("newclaw_feishu_errors_total", "Total Feishu errors");
    registry.register_gauge("newclaw_feishu_connected", "Whether Feishu is connected (1 or 0)");
    
    tracing::info!("Prometheus metrics initialized");
}

/// 指标类型
#[derive(Debug, Clone, Copy)]
pub enum MetricType {
    Counter,
    Gauge,
}

/// 指标注册表
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, Arc<AtomicU64>>>,
    gauges: RwLock<HashMap<String, Arc<AtomicU64>>>,
    metadata: RwLock<HashMap<String, (MetricType, String)>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        }
    }
    
    /// 注册计数器
    pub fn register_counter(&mut self, name: &str, help: &str) {
        let name = name.to_string();
        self.counters.write().unwrap().insert(name.clone(), Arc::new(AtomicU64::new(0)));
        self.metadata.write().unwrap().insert(name, (MetricType::Counter, help.to_string()));
    }
    
    /// 注册仪表盘
    pub fn register_gauge(&mut self, name: &str, help: &str) {
        let name = name.to_string();
        self.gauges.write().unwrap().insert(name.clone(), Arc::new(AtomicU64::new(0)));
        self.metadata.write().unwrap().insert(name, (MetricType::Gauge, help.to_string()));
    }
    
    /// 增加计数器
    pub fn inc_counter(&self, name: &str) {
        if let Some(counter) = self.counters.read().unwrap().get(name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// 增加计数器（指定值）
    pub fn inc_counter_by(&self, name: &str, value: u64) {
        if let Some(counter) = self.counters.read().unwrap().get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }
    
    /// 设置仪表盘值
    pub fn set_gauge(&self, name: &str, value: u64) {
        if let Some(gauge) = self.gauges.read().unwrap().get(name) {
            gauge.store(value, Ordering::Relaxed);
        }
    }
    
    /// 导出 Prometheus 格式
    pub fn export(&self) -> String {
        let mut output = String::new();
        
        // 导出计数器
        for (name, counter) in self.counters.read().unwrap().iter() {
            if let Some((_, help)) = self.metadata.read().unwrap().get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, help));
                output.push_str(&format!("# TYPE {} counter\n", name));
                output.push_str(&format!("{} {}\n", name, counter.load(Ordering::Relaxed)));
            }
        }
        
        // 导出仪表盘
        for (name, gauge) in self.gauges.read().unwrap().iter() {
            if let Some((_, help)) = self.metadata.read().unwrap().get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, help));
                output.push_str(&format!("# TYPE {} gauge\n", name));
                output.push_str(&format!("{} {}\n", name, gauge.load(Ordering::Relaxed)));
            }
        }
        
        output
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 初始化指标（幂等）
pub fn init_metrics() {
    // Lazy 会自动初始化
    let _ = REGISTRY.counters.read().unwrap().len();
}

/// 导出 Prometheus 格式指标
pub fn export_metrics() -> Result<Vec<u8>, String> {
    Ok(REGISTRY.export().into_bytes())
}

/// 记录 HTTP 请求
pub fn record_http_request(endpoint: &str, success: bool, latency_secs: f64) {
    REGISTRY.inc_counter("newclaw_requests_total");
    if success {
        REGISTRY.inc_counter("newclaw_requests_success_total");
    } else {
        REGISTRY.inc_counter("newclaw_requests_failed_total");
    }
    // 累加延迟（秒转毫秒存储避免精度问题）
    REGISTRY.inc_counter_by("newclaw_http_request_duration_seconds_total", (latency_secs * 1000.0) as u64);
    
    // 按端点记录（简化实现）
    let _ = endpoint;
}

/// 记录 LLM 请求
pub fn record_llm_request(success: bool, latency_secs: f64) {
    REGISTRY.inc_counter("newclaw_llm_requests_total");
    if !success {
        REGISTRY.inc_counter("newclaw_llm_errors_total");
    }
    REGISTRY.inc_counter_by("newclaw_llm_request_duration_seconds_total", (latency_secs * 1000.0) as u64);
}

/// 记录 Token 使用
pub fn record_tokens(input: u64, output: u64) {
    REGISTRY.inc_counter_by("newclaw_tokens_input_total", input);
    REGISTRY.inc_counter_by("newclaw_tokens_output_total", output);
}

/// 更新资源指标
pub fn update_resource_metrics(cpu_percent: f64, memory_used: u64, memory_total: u64) {
    REGISTRY.set_gauge("newclaw_cpu_usage_percent", cpu_percent as u64);
    REGISTRY.set_gauge("newclaw_memory_usage_bytes", memory_used);
    REGISTRY.set_gauge("newclaw_memory_total_bytes", memory_total);
}

/// 更新连接指标
pub fn update_connection_metrics(active_connections: u64, active_sessions: u64) {
    REGISTRY.set_gauge("newclaw_active_connections", active_connections);
    REGISTRY.set_gauge("newclaw_active_sessions", active_sessions);
}

/// 更新飞书状态
pub fn update_feishu_status(connected: bool) {
    REGISTRY.set_gauge("newclaw_feishu_connected", if connected { 1 } else { 0 });
}

/// 记录飞书消息
pub fn record_feishu_message(success: bool) {
    REGISTRY.inc_counter("newclaw_feishu_messages_total");
    if !success {
        REGISTRY.inc_counter("newclaw_feishu_errors_total");
    }
}

/// 记录工具执行
pub fn record_tool_execution(tool: &str, success: bool) {
    REGISTRY.inc_counter("newclaw_tool_executions_total");
    if !success {
        REGISTRY.inc_counter("newclaw_tool_errors_total");
    }
    let _ = tool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = MetricsRegistry::new();
        assert!(registry.counters.read().unwrap().is_empty());
    }

    #[test]
    fn test_registry_counter() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test", "A test counter");
        
        registry.inc_counter("test");
        registry.inc_counter("test");
        
        let value = registry.counters.read().unwrap().get("test").unwrap().load(Ordering::Relaxed);
        assert_eq!(value, 2);
    }

    #[test]
    fn test_registry_export() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test_counter", "A counter");
        registry.register_gauge("test_gauge", "A gauge");
        
        registry.inc_counter("test_counter");
        registry.set_gauge("test_gauge", 42);
        
        let output = registry.export();
        assert!(output.contains("test_counter"));
        assert!(output.contains("test_gauge"));
        assert!(output.contains("42"));
    }

    #[test]
    fn test_global_functions() {
        init_metrics();
        record_http_request("/test", true, 0.5);
        record_tokens(100, 50);
        update_resource_metrics(50.0, 1024, 2048);
        
        let result = export_metrics();
        assert!(result.is_ok());
    }
}
