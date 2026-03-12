// Metrics - v0.5.4
//
// Prometheus 指标导出

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::collections::HashMap;

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

/// 单个指标
#[derive(Debug, Clone)]
pub struct Metric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 帮助文本
    pub help: String,
    /// 标签
    pub labels: HashMap<String, String>,
    /// 值
    pub value: f64,
}

impl Metric {
    /// 创建计数器
    pub fn counter(name: &str, help: &str) -> Self {
        Self {
            name: name.to_string(),
            metric_type: MetricType::Counter,
            help: help.to_string(),
            labels: HashMap::new(),
            value: 0.0,
        }
    }
    
    /// 创建仪表盘
    pub fn gauge(name: &str, help: &str) -> Self {
        Self {
            name: name.to_string(),
            metric_type: MetricType::Gauge,
            help: help.to_string(),
            labels: HashMap::new(),
            value: 0.0,
        }
    }
    
    /// 添加标签
    pub fn with_label(mut self, key: &str, value: &str) -> Self {
        self.labels.insert(key.to_string(), value.to_string());
        self
    }
    
    /// 设置值
    pub fn set(mut self, value: f64) -> Self {
        self.value = value;
        self
    }
    
    /// 转换为 Prometheus 格式
    pub fn to_prometheus(&self) -> String {
        let mut output = String::new();
        
        // 帮助文本
        output.push_str(&format!("# HELP {} {}\n", self.name, self.help));
        
        // 类型
        let type_str = match self.metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
            MetricType::Histogram => "histogram",
        };
        output.push_str(&format!("# TYPE {} {}\n", self.name, type_str));
        
        // 值
        if self.labels.is_empty() {
            output.push_str(&format!("{} {}\n", self.name, self.value));
        } else {
            let labels_str = self.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join(",");
            output.push_str(&format!("{}{{{}}} {}\n", self.name, labels_str, self.value));
        }
        
        output
    }
}

/// 指标注册表
pub struct MetricsRegistry {
    /// 计数器
    counters: HashMap<String, Arc<AtomicU64>>,
    /// 仪表盘
    gauges: HashMap<String, Arc<AtomicU64>>,
    /// 指标元数据
    metadata: HashMap<String, (MetricType, String)>,
}

impl MetricsRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// 注册计数器
    pub fn register_counter(&mut self, name: &str, help: &str) {
        self.counters.insert(name.to_string(), Arc::new(AtomicU64::new(0)));
        self.metadata.insert(name.to_string(), (MetricType::Counter, help.to_string()));
    }
    
    /// 注册仪表盘
    pub fn register_gauge(&mut self, name: &str, help: &str) {
        self.gauges.insert(name.to_string(), Arc::new(AtomicU64::new(0)));
        self.metadata.insert(name.to_string(), (MetricType::Gauge, help.to_string()));
    }
    
    /// 增加计数器
    pub fn inc_counter(&self, name: &str) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    /// 增加计数器（指定值）
    pub fn inc_counter_by(&self, name: &str, value: u64) {
        if let Some(counter) = self.counters.get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }
    
    /// 设置仪表盘值
    pub fn set_gauge(&self, name: &str, value: u64) {
        if let Some(gauge) = self.gauges.get(name) {
            gauge.store(value, Ordering::Relaxed);
        }
    }
    
    /// 导出 Prometheus 格式
    pub fn export(&self) -> String {
        let mut output = String::new();
        
        // 导出计数器
        for (name, counter) in &self.counters {
            if let Some((_, help)) = self.metadata.get(name) {
                let metric = Metric::counter(name, help)
                    .set(counter.load(Ordering::Relaxed) as f64);
                output.push_str(&metric.to_prometheus());
            }
        }
        
        // 导出仪表盘
        for (name, gauge) in &self.gauges {
            if let Some((_, help)) = self.metadata.get(name) {
                let metric = Metric::gauge(name, help)
                    .set(gauge.load(Ordering::Relaxed) as f64);
                output.push_str(&metric.to_prometheus());
            }
        }
        
        output
    }
    
    /// 获取所有指标名称
    pub fn list_metrics(&self) -> Vec<String> {
        let mut names: Vec<String> = self.counters.keys()
            .chain(self.gauges.keys())
            .cloned()
            .collect();
        names.sort();
        names
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 预定义指标
pub struct DefaultMetrics;

impl DefaultMetrics {
    /// 注册默认指标
    pub fn register(registry: &mut MetricsRegistry) {
        // 请求相关
        registry.register_counter("newclaw_requests_total", "Total number of requests");
        registry.register_counter("newclaw_requests_errors_total", "Total number of request errors");
        
        // Token 相关
        registry.register_counter("newclaw_tokens_input_total", "Total input tokens");
        registry.register_counter("newclaw_tokens_output_total", "Total output tokens");
        registry.register_gauge("newclaw_context_size", "Current context size in tokens");
        
        // 工具相关
        registry.register_counter("newclaw_tool_calls_total", "Total tool calls");
        registry.register_counter("newclaw_tool_errors_total", "Total tool call errors");
        
        // 性能相关
        registry.register_gauge("newclaw_latency_ms", "Request latency in milliseconds");
        registry.register_gauge("newclaw_memory_mb", "Memory usage in MB");
        
        // 连接相关
        registry.register_gauge("newclaw_active_connections", "Number of active connections");
        registry.register_counter("newclaw_messages_sent_total", "Total messages sent");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_counter() {
        let metric = Metric::counter("test_counter", "A test counter");
        assert_eq!(metric.name, "test_counter");
        assert_eq!(metric.metric_type, MetricType::Counter);
    }

    #[test]
    fn test_metric_gauge() {
        let metric = Metric::gauge("test_gauge", "A test gauge");
        assert_eq!(metric.name, "test_gauge");
        assert_eq!(metric.metric_type, MetricType::Gauge);
    }

    #[test]
    fn test_metric_with_label() {
        let metric = Metric::counter("test", "help")
            .with_label("method", "GET");
        
        assert!(metric.labels.contains_key("method"));
    }

    #[test]
    fn test_metric_to_prometheus() {
        let metric = Metric::counter("test", "A test metric")
            .set(42.0);
        
        let output = metric.to_prometheus();
        assert!(output.contains("# HELP test A test metric"));
        assert!(output.contains("# TYPE test counter"));
        assert!(output.contains("test 42"));
    }

    #[test]
    fn test_metric_to_prometheus_with_labels() {
        let metric = Metric::counter("test", "help")
            .with_label("method", "GET")
            .set(10.0);
        
        let output = metric.to_prometheus();
        assert!(output.contains("method=\"GET\""));
    }

    #[test]
    fn test_metrics_registry_new() {
        let registry = MetricsRegistry::new();
        assert!(registry.counters.is_empty());
        assert!(registry.gauges.is_empty());
    }

    #[test]
    fn test_metrics_registry_register_counter() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test", "help");
        
        assert!(registry.counters.contains_key("test"));
    }

    #[test]
    fn test_metrics_registry_inc_counter() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test", "help");
        
        registry.inc_counter("test");
        registry.inc_counter("test");
        
        let value = registry.counters.get("test").unwrap().load(Ordering::Relaxed);
        assert_eq!(value, 2);
    }

    #[test]
    fn test_metrics_registry_set_gauge() {
        let mut registry = MetricsRegistry::new();
        registry.register_gauge("test", "help");
        
        registry.set_gauge("test", 100);
        
        let value = registry.gauges.get("test").unwrap().load(Ordering::Relaxed);
        assert_eq!(value, 100);
    }

    #[test]
    fn test_metrics_registry_export() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test_counter", "A counter");
        registry.register_gauge("test_gauge", "A gauge");
        
        registry.inc_counter("test_counter");
        registry.set_gauge("test_gauge", 50);
        
        let output = registry.export();
        assert!(output.contains("test_counter"));
        assert!(output.contains("test_gauge"));
    }

    #[test]
    fn test_default_metrics_register() {
        let mut registry = MetricsRegistry::new();
        DefaultMetrics::register(&mut registry);
        
        let metrics = registry.list_metrics();
        assert!(!metrics.is_empty());
        assert!(metrics.contains(&"newclaw_requests_total".to_string()));
    }
}
