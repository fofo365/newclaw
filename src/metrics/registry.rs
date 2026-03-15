// Metrics Registry - v0.5.5
//
// 简化的指标注册表，不依赖外部 crate

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricType {
    Counter,
    Gauge,
}

/// 单个指标
#[derive(Debug, Clone)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub help: String,
    pub value: f64,
}

impl Metric {
    pub fn counter(name: &str, help: &str) -> Self {
        Self {
            name: name.to_string(),
            metric_type: MetricType::Counter,
            help: help.to_string(),
            value: 0.0,
        }
    }
    
    pub fn gauge(name: &str, help: &str) -> Self {
        Self {
            name: name.to_string(),
            metric_type: MetricType::Gauge,
            help: help.to_string(),
            value: 0.0,
        }
    }
    
    pub fn set(mut self, value: f64) -> Self {
        self.value = value;
        self
    }
    
    pub fn to_prometheus(&self) -> String {
        let type_str = match self.metric_type {
            MetricType::Counter => "counter",
            MetricType::Gauge => "gauge",
        };
        format!("# HELP {} {}\n# TYPE {} {}\n{} {}\n", 
            self.name, self.help, self.name, type_str, self.name, self.value)
    }
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
    
    pub fn register_counter(&mut self, name: &str, help: &str) {
        let name = name.to_string();
        self.counters.write().unwrap().insert(name.clone(), Arc::new(AtomicU64::new(0)));
        self.metadata.write().unwrap().insert(name, (MetricType::Counter, help.to_string()));
    }
    
    pub fn register_gauge(&mut self, name: &str, help: &str) {
        let name = name.to_string();
        self.gauges.write().unwrap().insert(name.clone(), Arc::new(AtomicU64::new(0)));
        self.metadata.write().unwrap().insert(name, (MetricType::Gauge, help.to_string()));
    }
    
    pub fn inc_counter(&self, name: &str) {
        if let Some(counter) = self.counters.read().unwrap().get(name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn inc_counter_by(&self, name: &str, value: u64) {
        if let Some(counter) = self.counters.read().unwrap().get(name) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }
    
    pub fn set_gauge(&self, name: &str, value: u64) {
        if let Some(gauge) = self.gauges.read().unwrap().get(name) {
            gauge.store(value, Ordering::Relaxed);
        }
    }
    
    pub fn export(&self) -> String {
        let mut output = String::new();
        
        for (name, counter) in self.counters.read().unwrap().iter() {
            if let Some((_, help)) = self.metadata.read().unwrap().get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, help));
                output.push_str(&format!("# TYPE {} counter\n", name));
                output.push_str(&format!("{} {}\n", name, counter.load(Ordering::Relaxed)));
            }
        }
        
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

/// 默认指标
pub struct DefaultMetrics;

impl DefaultMetrics {
    pub fn register(registry: &mut MetricsRegistry) {
        registry.register_counter("newclaw_requests_total", "Total requests");
        registry.register_counter("newclaw_requests_success", "Successful requests");
        registry.register_counter("newclaw_requests_failed", "Failed requests");
        registry.register_counter("newclaw_tokens_input", "Input tokens");
        registry.register_counter("newclaw_tokens_output", "Output tokens");
        registry.register_gauge("newclaw_active_sessions", "Active sessions");
        registry.register_gauge("newclaw_cpu_usage", "CPU usage percent");
        registry.register_gauge("newclaw_memory_mb", "Memory usage MB");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let mut registry = MetricsRegistry::new();
        registry.register_counter("test", "A test");
        registry.inc_counter("test");
        assert!(registry.export().contains("test 1"));
    }
}
