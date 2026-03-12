// 诊断引擎模块

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::heartbeat::HeartbeatStatus;
use super::recovery::RecoveryLevel;

/// 根因
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {
    /// 根因 ID
    pub id: String,
    /// 严重程度
    pub severity: Severity,
    /// 根因类型
    pub cause_type: CauseType,
    /// 描述
    pub description: String,
    /// 建议
    pub suggestions: Vec<String>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl RootCause {
    pub fn new(severity: Severity, cause_type: CauseType, description: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            severity,
            cause_type,
            description,
            suggestions: vec![],
            timestamp: Utc::now(),
        }
    }
    
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
    
    /// 根据严重程度确定恢复级别
    pub fn recovery_level(&self) -> RecoveryLevel {
        match self.severity {
            Severity::Low => RecoveryLevel::L1QuickFix,
            Severity::Medium => RecoveryLevel::L2AiDiagnosis,
            Severity::High => RecoveryLevel::L3HumanIntervention,
        }
    }
}

/// 严重程度
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

/// 根因类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CauseType {
    /// 内存不足
    MemoryExhaustion,
    /// CPU 过载
    CpuOverload,
    /// 网络问题
    NetworkIssue,
    /// 数据库连接
    DatabaseConnection,
    /// 依赖服务
    DependencyFailure,
    /// 配置错误
    ConfigurationError,
    /// 代码错误
    CodeError,
    /// 资源泄漏
    ResourceLeak,
    /// 死锁
    Deadlock,
    /// 未知
    Unknown,
}

/// 诊断结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    /// 结果 ID
    pub id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 分析的日志
    pub analyzed_logs: Vec<String>,
    /// 匹配的模式
    pub matched_patterns: Vec<PatternMatch>,
    /// 识别的根因
    pub root_causes: Vec<RootCause>,
    /// 建议的恢复级别
    pub suggested_level: RecoveryLevel,
}

impl DiagnosticResult {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            analyzed_logs: vec![],
            matched_patterns: vec![],
            root_causes: vec![],
            suggested_level: RecoveryLevel::L1QuickFix,
        }
    }
    
    pub fn with_logs(mut self, logs: Vec<String>) -> Self {
        self.analyzed_logs = logs;
        self
    }
    
    pub fn with_patterns(mut self, patterns: Vec<PatternMatch>) -> Self {
        self.matched_patterns = patterns;
        self
    }
    
    pub fn with_root_causes(mut self, causes: Vec<RootCause>) -> Self {
        if let Some(most_severe) = causes.iter().max_by_key(|c| c.severity.clone() as i32) {
            self.suggested_level = most_severe.recovery_level();
        }
        self.root_causes = causes;
        self
    }
}

/// 模式匹配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_name: String,
    pub pattern_type: PatternType,
    pub confidence: f64,
    pub matched_text: String,
}

/// 模式类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    Error,
    Warning,
    Anomaly,
    Threshold,
}

/// 已知问题模式
struct KnownPattern {
    name: String,
    pattern: regex::Regex,
    cause_type: CauseType,
    severity: Severity,
    suggestions: Vec<String>,
}

/// 诊断引擎
pub struct DiagnosticEngine {
    known_patterns: Vec<KnownPattern>,
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        Self {
            known_patterns: Self::build_known_patterns(),
        }
    }
    
    /// 构建已知问题模式
    fn build_known_patterns() -> Vec<KnownPattern> {
        use regex::Regex;
        
        vec![
            KnownPattern {
                name: "OOM".to_string(),
                pattern: Regex::new(r"(?i)(out of memory|oom|memory.*exhausted)").unwrap(),
                cause_type: CauseType::MemoryExhaustion,
                severity: Severity::High,
                suggestions: vec![
                    "重启服务".to_string(),
                    "清理缓存".to_string(),
                    "增加内存限制".to_string(),
                ],
            },
            KnownPattern {
                name: "Deadlock".to_string(),
                pattern: Regex::new(r"(?i)(deadlock|lock.*timeout|blocked)").unwrap(),
                cause_type: CauseType::Deadlock,
                severity: Severity::High,
                suggestions: vec![
                    "重启服务".to_string(),
                    "检查锁使用".to_string(),
                ],
            },
            KnownPattern {
                name: "NetworkError".to_string(),
                pattern: Regex::new(r"(?i)(connection.*refused|timeout|network.*unreachable)").unwrap(),
                cause_type: CauseType::NetworkIssue,
                severity: Severity::Medium,
                suggestions: vec![
                    "检查网络连接".to_string(),
                    "重试请求".to_string(),
                ],
            },
            KnownPattern {
                name: "DatabaseError".to_string(),
                pattern: Regex::new(r"(?i)(database.*error|connection.*pool|sql.*error)").unwrap(),
                cause_type: CauseType::DatabaseConnection,
                severity: Severity::Medium,
                suggestions: vec![
                    "检查数据库连接".to_string(),
                    "重置连接池".to_string(),
                ],
            },
            KnownPattern {
                name: "ConfigError".to_string(),
                pattern: Regex::new(r"(?i)(config.*error|invalid.*setting|missing.*config)").unwrap(),
                cause_type: CauseType::ConfigurationError,
                severity: Severity::Medium,
                suggestions: vec![
                    "检查配置文件".to_string(),
                    "回滚配置".to_string(),
                ],
            },
        ]
    }
    
    /// 分析心跳状态
    pub async fn analyze(&self, status: &HeartbeatStatus) -> anyhow::Result<DiagnosticResult> {
        let mut result = DiagnosticResult::new();
        let mut matched_patterns = Vec::new();
        let mut root_causes = Vec::new();
        
        // 分析错误日志
        for error in &status.recent_errors {
            for pattern in &self.known_patterns {
                if pattern.pattern.is_match(error) {
                    matched_patterns.push(PatternMatch {
                        pattern_name: pattern.name.clone(),
                        pattern_type: PatternType::Error,
                        confidence: 0.8,
                        matched_text: error.clone(),
                    });
                    
                    root_causes.push(
                        RootCause::new(
                            pattern.severity.clone(),
                            pattern.cause_type.clone(),
                            format!("Pattern '{}' matched", pattern.name),
                        ).with_suggestions(pattern.suggestions.clone())
                    );
                }
            }
        }
        
        // 检查系统指标
        if status.metrics.memory_mb > 500 {
            root_causes.push(
                RootCause::new(
                    Severity::Medium,
                    CauseType::MemoryExhaustion,
                    format!("High memory usage: {} MB", status.metrics.memory_mb),
                ).with_suggestions(vec!["清理缓存".to_string(), "重启服务".to_string()])
            );
        }
        
        if status.metrics.cpu_percent > 80.0 {
            root_causes.push(
                RootCause::new(
                    Severity::Low,
                    CauseType::CpuOverload,
                    format!("High CPU usage: {}%", status.metrics.cpu_percent),
                ).with_suggestions(vec!["限制请求速率".to_string()])
            );
        }
        
        result = result
            .with_logs(status.recent_errors.clone())
            .with_patterns(matched_patterns)
            .with_root_causes(root_causes);
        
        Ok(result)
    }
    
    /// 分析日志
    pub fn analyze_logs(&self, logs: &[String]) -> Vec<PatternMatch> {
        let mut matches = Vec::new();
        
        for log in logs {
            for pattern in &self.known_patterns {
                if pattern.pattern.is_match(log) {
                    matches.push(PatternMatch {
                        pattern_name: pattern.name.clone(),
                        pattern_type: PatternType::Error,
                        confidence: 0.8,
                        matched_text: log.clone(),
                    });
                }
            }
        }
        
        matches
    }
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_root_cause_recovery_level() {
        let low = RootCause::new(Severity::Low, CauseType::Unknown, "test".to_string());
        assert_eq!(low.recovery_level(), RecoveryLevel::L1QuickFix);
        
        let medium = RootCause::new(Severity::Medium, CauseType::Unknown, "test".to_string());
        assert_eq!(medium.recovery_level(), RecoveryLevel::L2AiDiagnosis);
        
        let high = RootCause::new(Severity::High, CauseType::Unknown, "test".to_string());
        assert_eq!(high.recovery_level(), RecoveryLevel::L3HumanIntervention);
    }
    
    #[test]
    fn test_diagnostic_engine_oom() {
        let engine = DiagnosticEngine::new();
        let logs = vec!["Out of memory error".to_string()];
        
        let matches = engine.analyze_logs(&logs);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].pattern_name, "OOM");
    }
    
    #[test]
    fn test_diagnostic_engine_deadlock() {
        let engine = DiagnosticEngine::new();
        let logs = vec!["Thread deadlock detected".to_string()];
        
        let matches = engine.analyze_logs(&logs);
        assert!(!matches.is_empty());
        assert_eq!(matches[0].pattern_name, "Deadlock");
    }
    
    #[tokio::test]
    async fn test_diagnostic_engine_analyze() {
        let engine = DiagnosticEngine::new();
        let status = HeartbeatStatus::unhealthy(
            "lease-123".to_string(),
            "smart".to_string(),
            vec!["Database error: connection refused".to_string()],
        );
        
        let result = engine.analyze(&status).await.unwrap();
        assert!(!result.matched_patterns.is_empty());
        assert!(!result.root_causes.is_empty());
    }
}
