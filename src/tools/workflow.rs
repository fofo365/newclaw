//! Diagnostic Workflow Tool - 诊断工作流工具
//!
//! 提供标准化的诊断流程，避免AI走极端
//! 当遇到问题时，先诊断，再行动

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::process::Command;
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// 诊断工作流工具
pub struct WorkflowTool {
    metadata: ToolMetadata,
}

/// 诊断步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStep {
    /// 检查系统状态
    CheckSystemStatus,
    /// 查询定时任务
    ListCronJobs,
    /// 检查配置文件
    CheckConfig,
    /// 检查日志
    CheckLogs,
    /// 检查服务状态
    CheckServices,
    /// 检查记忆状态
    CheckMemory,
}

/// 诊断结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticWorkflowResult {
    /// 是否成功
    pub success: bool,
    /// 时间戳
    pub timestamp: String,
    /// 问题描述
    pub problem: Option<String>,
    /// 诊断步骤结果
    pub steps: Vec<StepResult>,
    /// 发现的问题
    pub issues: Vec<String>,
    /// 建议的解决方案
    pub solutions: Vec<Solution>,
    /// 需要用户确认的操作
    pub requires_confirmation: Vec<String>,
}

/// 单步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step: String,
    pub success: bool,
    pub summary: String,
    pub details: Option<JsonValue>,
}

/// 解决方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    /// 方案描述
    pub description: String,
    /// 风险等级
    pub risk_level: String,
    /// 执行命令（如果有）
    pub command: Option<String>,
    /// 需要确认
    pub requires_confirmation: bool,
}

impl WorkflowTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                name: "workflow".to_string(),
                description: r#"诊断工作流工具。提供标准化的诊断流程。

当遇到问题时，按照以下步骤处理：
1. 问题识别
2. 初步诊断
3. 问题定位
4. 提供解决方案
5. 验证和记录

Actions:
- diagnose: 执行完整诊断流程
- quick_check: 快速检查系统状态
- find_cron_issue: 查找定时任务相关问题
- find_service_issue: 查找服务相关问题

用法示例:
- {"action": "diagnose", "problem": "飞书一直在发消息"} - 诊断问题
- {"action": "quick_check"} - 快速检查
- {"action": "find_cron_issue"} - 查找定时任务问题
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["diagnose", "quick_check", "find_cron_issue", "find_service_issue"],
                            "description": "工作流操作类型"
                        },
                        "problem": {
                            "type": "string",
                            "description": "问题描述（diagnose时使用）"
                        }
                    },
                    "required": ["action"]
                }),
            }
        }
    }
    
    /// 执行诊断工作流
    fn run_diagnosis(&self, problem: Option<&str>) -> DiagnosticWorkflowResult {
        let mut steps = Vec::new();
        let mut issues = Vec::new();
        let mut solutions = Vec::new();
        let mut requires_confirmation = Vec::new();
        
        // 步骤1: 检查系统状态
        let system_result = self.check_system_status();
        steps.push(system_result.clone());
        if let Some(details) = &system_result.details {
            if let Some(mem) = details.get("memory_high").and_then(|m| m.as_bool()) {
                if mem {
                    issues.push("内存使用率过高".to_string());
                    solutions.push(Solution {
                        description: "清理缓存或重启服务".to_string(),
                        risk_level: "low".to_string(),
                        command: Some("sync && echo 3 > /proc/sys/vm/drop_caches".to_string()),
                        requires_confirmation: true,
                    });
                }
            }
        }
        
        // 步骤2: 检查定时任务
        let cron_result = self.check_cron_jobs();
        steps.push(cron_result.clone());
        if let Some(details) = &cron_result.details {
            if let Some(count) = details.get("count").and_then(|c| c.as_u64()) {
                if count > 10 {
                    issues.push(format!("发现 {} 个定时任务，可能存在重复或无用任务", count));
                    solutions.push(Solution {
                        description: "检查并清理不需要的定时任务".to_string(),
                        risk_level: "medium".to_string(),
                        command: Some("crontab -l".to_string()),
                        requires_confirmation: true,
                    });
                }
            }
        }
        
        // 步骤3: 检查服务状态
        let service_result = self.check_services();
        steps.push(service_result.clone());
        if let Some(details) = &service_result.details {
            if let Some(failed) = details.get("failed_services").and_then(|f| f.as_array()) {
                if !failed.is_empty() {
                    for svc in failed {
                        issues.push(format!("服务 {} 处于失败状态", svc));
                        solutions.push(Solution {
                            description: format!("重启服务 {}", svc),
                            risk_level: "medium".to_string(),
                            command: Some(format!("systemctl restart {}", svc)),
                            requires_confirmation: true,
                        });
                    }
                }
            }
        }
        
        // 步骤4: 检查日志
        let log_result = self.check_logs();
        steps.push(log_result);
        
        // 如果有问题描述，针对性分析
        if let Some(prob) = problem {
            if prob.contains("消息") || prob.contains("发送") || prob.contains("通知") {
                requires_confirmation.push("是否需要检查定时任务？".to_string());
            }
            if prob.contains("崩溃") || prob.contains("重启") {
                requires_confirmation.push("是否需要检查服务状态？".to_string());
            }
        }
        
        DiagnosticWorkflowResult {
            success: true,
            timestamp: chrono::Utc::now().to_rfc3339(),
            problem: problem.map(|s| s.to_string()),
            steps,
            issues,
            solutions,
            requires_confirmation,
        }
    }
    
    /// 快速检查
    fn quick_check(&self) -> JsonValue {
        let mut warnings = Vec::new();
        
        // 检查内存
        if let Ok(output) = Command::new("free").args(["-m"]).output() {
            let mem_info = String::from_utf8_lossy(&output.stdout);
            if mem_info.contains("90%") || mem_info.contains("95%") {
                warnings.push("内存使用率过高");
            }
        }
        
        // 检查磁盘
        if let Ok(output) = Command::new("df").args(["-h", "/"]).output() {
            let disk_info = String::from_utf8_lossy(&output.stdout);
            if disk_info.contains("90%") || disk_info.contains("95%") {
                warnings.push("磁盘使用率过高");
            }
        }
        
        // 检查失败服务
        if let Ok(output) = Command::new("systemctl")
            .args(["list-units", "--state=failed", "--no-pager"])
            .output()
        {
            let failed = String::from_utf8_lossy(&output.stdout);
            if failed.contains("failed") {
                warnings.push("存在失败的服务");
            }
        }
        
        json!({
            "success": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": if warnings.is_empty() { "healthy" } else { "warning" },
            "warnings": warnings
        })
    }
    
    /// 查找定时任务问题
    fn find_cron_issues(&self) -> JsonValue {
        let mut issues = Vec::new();
        let mut tasks: Vec<String> = Vec::new();
        
        // 检查用户 crontab
        if let Ok(output) = Command::new("crontab").args(["-l"]).output() {
            let crontab = String::from_utf8_lossy(&output.stdout);
            for line in crontab.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    let line_owned = line.to_string();
                    
                    // 检测高频任务
                    if line.contains("*/1") || line.contains("*/2") || line.contains("*/5") {
                        issues.push(json!({
                            "type": "high_frequency",
                            "task": line_owned.clone(),
                            "suggestion": "高频定时任务，请确认是否需要"
                        }));
                    }
                    
                    tasks.push(line_owned);
                }
            }
        }
        
        json!({
            "success": true,
            "task_count": tasks.len(),
            "tasks": tasks,
            "issues": issues,
            "recommendation": if !issues.is_empty() {
                "发现可疑的定时任务，请检查是否需要删除或调整"
            } else if tasks.is_empty() {
                "没有发现定时任务"
            } else {
                "定时任务状态正常"
            }
        })
    }
    
    /// 查找服务问题
    fn find_service_issues(&self) -> JsonValue {
        let mut failed_services = Vec::new();
        let services = vec!["openclaw-gateway", "newclaw-gateway", "newclaw"];
        
        for svc in services {
            if let Ok(output) = Command::new("systemctl")
                .args(["status", svc, "--no-pager"])
                .output()
            {
                let status = String::from_utf8_lossy(&output.stdout);
                if status.contains("failed") {
                    failed_services.push(svc);
                }
            }
        }
        
        json!({
            "success": true,
            "failed_count": failed_services.len(),
            "failed_services": failed_services,
            "recommendation": if !failed_services.is_empty() {
                format!("发现 {} 个失败服务，建议检查日志并尝试重启", failed_services.len())
            } else {
                "所有服务运行正常".to_string()
            }
        })
    }
    
    /// 检查系统状态（内部方法）
    fn check_system_status(&self) -> StepResult {
        let mut details = serde_json::Map::new();
        let mut memory_high = false;
        
        if let Ok(output) = Command::new("free").args(["-m"]).output() {
            let mem = String::from_utf8_lossy(&output.stdout);
            memory_high = mem.contains("90%") || mem.contains("95%");
            details.insert("memory".to_string(), json!(mem.lines().take(2).collect::<Vec<_>>()));
        }
        
        details.insert("memory_high".to_string(), json!(memory_high));
        
        StepResult {
            step: "check_system_status".to_string(),
            success: true,
            summary: if memory_high { "内存使用率偏高" } else { "系统状态正常" }.to_string(),
            details: Some(json!(details)),
        }
    }
    
    /// 检查定时任务（内部方法）
    fn check_cron_jobs(&self) -> StepResult {
        let mut count = 0;
        
        if let Ok(output) = Command::new("crontab").args(["-l"]).output() {
            let crontab = String::from_utf8_lossy(&output.stdout);
            for line in crontab.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    count += 1;
                }
            }
        }
        
        StepResult {
            step: "list_cron_jobs".to_string(),
            success: true,
            summary: format!("发现 {} 个定时任务", count),
            details: Some(json!({"count": count})),
        }
    }
    
    /// 检查服务（内部方法）
    fn check_services(&self) -> StepResult {
        let mut failed = Vec::new();
        let services = vec!["openclaw-gateway", "newclaw-gateway"];
        
        for svc in services {
            if let Ok(output) = Command::new("systemctl")
                .args(["is-active", svc])
                .output()
            {
                let status = String::from_utf8_lossy(&output.stdout);
                if status.trim() == "failed" {
                    failed.push(svc);
                }
            }
        }
        
        StepResult {
            step: "check_services".to_string(),
            success: failed.is_empty(),
            summary: if failed.is_empty() { "所有服务正常".to_string() } else { format!("{} 个服务失败", failed.len()) },
            details: Some(json!({"failed_services": failed})),
        }
    }
    
    /// 检查日志（内部方法）
    fn check_logs(&self) -> StepResult {
        let mut error_count = 0;
        
        if let Ok(output) = Command::new("journalctl")
            .args(["-n", "50", "--no-pager", "-p", "err"])
            .output()
        {
            let logs = String::from_utf8_lossy(&output.stdout);
            error_count = logs.lines().count();
        }
        
        StepResult {
            step: "check_logs".to_string(),
            success: error_count < 10,
            summary: format!("最近发现 {} 条错误日志", error_count),
            details: Some(json!({"error_count": error_count})),
        }
    }
}

impl Default for WorkflowTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WorkflowTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;
        
        info!("Workflow tool called with action: {}", action);
        
        let result = match action {
            "diagnose" => {
                let problem = args.get("problem").and_then(|p| p.as_str());
                serde_json::to_value(self.run_diagnosis(problem)).unwrap_or_default()
            }
            "quick_check" => self.quick_check(),
            "find_cron_issue" => self.find_cron_issues(),
            "find_service_issue" => self.find_service_issues(),
            _ => {
                return Err(ToolError::InvalidArguments(format!("Unknown action: {}", action)).into());
            }
        };
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_workflow_tool_metadata() {
        let tool = WorkflowTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "workflow");
    }
    
    #[tokio::test]
    async fn test_workflow_tool_quick_check() {
        let tool = WorkflowTool::new();
        let result = tool.execute(json!({"action": "quick_check"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_workflow_tool_diagnose() {
        let tool = WorkflowTool::new();
        let result = tool.execute(json!({"action": "diagnose", "problem": "测试问题"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
        assert!(result.get("steps").is_some());
    }
}