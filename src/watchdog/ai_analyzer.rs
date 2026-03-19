// AI 日志分析器模块

use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::config::L2Config;
use super::diagnostic::{RootCause, Severity, CauseType, DiagnosticResult};
use crate::core::llm::{LLMProvider, LLMMessage, LLMResponse};

/// AI 分析器
pub struct AIAnalyzer {
    /// LLM Provider
    provider: Arc<RwLock<Box<dyn LLMProvider>>>,
    /// 配置
    config: L2Config,
}

/// LLM 诊断请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisRequest {
    /// 日志内容
    pub logs: Vec<String>,
    /// 系统状态
    pub system_status: SystemStatus,
    /// 历史恢复记录
    pub recovery_history: Vec<String>,
}

/// 系统状态摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub memory_usage_mb: u64,
    pub cpu_percent: f32,
    pub active_sessions: u64,
    pub uptime_seconds: u64,
}

/// LLM 诊断响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisResponse {
    /// 识别的问题
    pub identified_issues: Vec<IdentifiedIssue>,
    /// 根因分析
    pub root_cause: String,
    /// 建议的修复动作
    pub suggested_actions: Vec<SuggestedAction>,
    /// 置信度
    pub confidence: f64,
}

/// 识别的问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedIssue {
    pub issue_type: String,
    pub description: String,
    pub severity: String,
    pub evidence: String,
}

/// 建议的动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action: String,
    pub reason: String,
    pub priority: u8,
    pub estimated_impact: String,
}

impl AIAnalyzer {
    /// 创建新的 AI 分析器
    pub fn new(provider: Box<dyn LLMProvider>, config: L2Config) -> Self {
        Self {
            provider: Arc::new(RwLock::new(provider)),
            config,
        }
    }
    
    /// 使用默认配置创建
    pub fn with_default_config(provider: Box<dyn LLMProvider>) -> Self {
        Self::new(provider, L2Config::default())
    }
    
    /// 分析日志并生成诊断结果
    pub async fn analyze(&self, request: DiagnosisRequest) -> Result<DiagnosisResponse> {
        let prompt = self.build_diagnosis_prompt(&request);
        
        let messages = vec![
            LLMMessage {
                role: "system".to_string(),
                content: DIAGNOSIS_SYSTEM_PROMPT.to_string(),
            },
            LLMMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];
        
        let provider = self.provider.read().await;
        let response = provider.chat(&messages).await?;
        
        self.parse_diagnosis_response(&response)
    }
    
    /// 构建诊断提示词
    fn build_diagnosis_prompt(&self, request: &DiagnosisRequest) -> String {
        let logs_str = request.logs.join("\n");
        let status_str = format!(
            "内存使用: {}MB\nCPU: {}%\n活跃会话: {}\n运行时间: {}s",
            request.system_status.memory_usage_mb,
            request.system_status.cpu_percent,
            request.system_status.active_sessions,
            request.system_status.uptime_seconds
        );
        let history_str = if request.recovery_history.is_empty() {
            "无".to_string()
        } else {
            request.recovery_history.join("\n")
        };
        
        format!(
            r#"请分析以下系统状态和日志，诊断问题根因并给出修复建议。

## 系统状态
{}

## 最近日志
```
{}
```

## 最近恢复记录
{}

请以 JSON 格式返回诊断结果，包含以下字段：
- identified_issues: 识别的问题列表
- root_cause: 根因分析
- suggested_actions: 建议的修复动作
- confidence: 置信度 (0.0-1.0)

返回格式示例：
{{
  "identified_issues": [
    {{
      "issue_type": "memory_exhaustion",
      "description": "内存不足",
      "severity": "high",
      "evidence": "OOM killer triggered"
    }}
  ],
  "root_cause": "内存泄漏导致系统内存耗尽",
  "suggested_actions": [
    {{
      "action": "restart_service",
      "reason": "重启服务释放内存",
      "priority": 1,
      "estimated_impact": "服务短暂不可用"
    }}
  ],
  "confidence": 0.85
}}
"#,
            status_str, logs_str, history_str
        )
    }
    
    /// 解析 LLM 响应
    fn parse_diagnosis_response(&self, response: &LLMResponse) -> Result<DiagnosisResponse> {
        // 尝试从响应中提取 JSON
        let content = response.content.trim();
        
        // 查找 JSON 块
        let json_str = if content.starts_with('{') {
            content.to_string()
        } else if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                content[start..=end].to_string()
            } else {
                content.to_string()
            }
        } else {
            // 如果无法解析 JSON，返回默认响应
            return Ok(DiagnosisResponse {
                identified_issues: vec![IdentifiedIssue {
                    issue_type: "parse_error".to_string(),
                    description: "无法解析 LLM 响应".to_string(),
                    severity: "low".to_string(),
                    evidence: content.to_string(),
                }],
                root_cause: "LLM 响应格式异常".to_string(),
                suggested_actions: vec![SuggestedAction {
                    action: "clear_cache".to_string(),
                    reason: "清理缓存作为安全措施".to_string(),
                    priority: 2,
                    estimated_impact: "低风险".to_string(),
                }],
                confidence: 0.3,
            });
        };
        
        // 解析 JSON
        match serde_json::from_str::<DiagnosisResponse>(&json_str) {
            Ok(diagnosis) => Ok(diagnosis),
            Err(e) => {
                tracing::warn!("Failed to parse diagnosis response: {}", e);
                Ok(DiagnosisResponse {
                    identified_issues: vec![],
                    root_cause: format!("解析错误: {}", e),
                    suggested_actions: vec![],
                    confidence: 0.0,
                })
            }
        }
    }
    
    /// 将诊断响应转换为 DiagnosticResult
    pub fn to_diagnostic_result(&self, response: &DiagnosisResponse, logs: Vec<String>) -> DiagnosticResult {
        let root_causes: Vec<RootCause> = response.identified_issues
            .iter()
            .map(|issue| {
                let severity = match issue.severity.as_str() {
                    "high" => Severity::High,
                    "medium" => Severity::Medium,
                    _ => Severity::Low,
                };
                let cause_type = match issue.issue_type.as_str() {
                    "memory_exhaustion" | "oom" => CauseType::MemoryExhaustion,
                    "cpu_overload" => CauseType::CpuOverload,
                    "network_issue" => CauseType::NetworkIssue,
                    "database_connection" => CauseType::DatabaseConnection,
                    "deadlock" => CauseType::Deadlock,
                    _ => CauseType::Unknown,
                };
                
                RootCause::new(severity, cause_type, issue.description.clone())
                    .with_suggestions(response.suggested_actions.iter().map(|a| a.action.clone()).collect())
            })
            .collect();
        
        DiagnosticResult::new()
            .with_logs(logs)
            .with_root_causes(root_causes)
    }
}

/// 系统诊断提示词
const DIAGNOSIS_SYSTEM_PROMPT: &str = r#"你是一个专业的系统诊断专家，负责分析系统日志和状态，识别问题根因并给出修复建议。

你的职责：
1. 分析日志中的错误模式
2. 识别系统瓶颈和异常
3. 确定问题根因
4. 提供可执行的修复建议

分析原则：
- 从日志中提取关键信息
- 结合系统状态综合判断
- 给出具体的修复步骤
- 评估修复风险和影响

输出要求：
- 必须返回有效的 JSON 格式
- 问题分类要准确
- 修复建议要具体可执行
- 置信度要实事求是"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::llm::MockLLMProvider;
    
    #[tokio::test]
    async fn test_ai_analyzer_create() {
        let provider = Box::new(MockLLMProvider);
        let analyzer = AIAnalyzer::with_default_config(provider);
        
        let request = DiagnosisRequest {
            logs: vec!["Error: Out of memory".to_string()],
            system_status: SystemStatus {
                memory_usage_mb: 1024,
                cpu_percent: 50.0,
                active_sessions: 10,
                uptime_seconds: 3600,
            },
            recovery_history: vec![],
        };
        
        let result = analyzer.analyze(request).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_build_diagnosis_prompt() {
        let provider = Box::new(MockLLMProvider);
        let analyzer = AIAnalyzer::with_default_config(provider);
        
        let request = DiagnosisRequest {
            logs: vec!["Error: test".to_string()],
            system_status: SystemStatus {
                memory_usage_mb: 100,
                cpu_percent: 10.0,
                active_sessions: 1,
                uptime_seconds: 100,
            },
            recovery_history: vec![],
        };
        
        let prompt = analyzer.build_diagnosis_prompt(&request);
        assert!(prompt.contains("内存使用: 100MB"));
        assert!(prompt.contains("Error: test"));
    }
    
    #[test]
    fn test_parse_diagnosis_response() {
        let provider = Box::new(MockLLMProvider);
        let analyzer = AIAnalyzer::with_default_config(provider);
        
        let response = LLMResponse {
            content: r#"{"identified_issues":[],"root_cause":"test","suggested_actions":[],"confidence":0.5}"#.to_string(),
            tokens_used: Some(100),
            model: "test".to_string(),
        };
        
        let result = analyzer.parse_diagnosis_response(&response);
        assert!(result.is_ok());
        let diagnosis = result.unwrap();
        assert_eq!(diagnosis.root_cause, "test");
    }
}