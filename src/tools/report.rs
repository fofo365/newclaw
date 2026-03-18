//! Diagnostic Report Tool - 诊断报告生成工具
//!
//! 生成详细的诊断报告，便于问题追踪
//! 来源: CHANGELOG-v0.7.1.md P2 需求

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::process::Command;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// 诊断报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// 报告 ID
    pub id: String,
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 系统信息
    pub system_info: SystemInfo,
    /// 问题描述
    pub problem: Option<String>,
    /// 诊断步骤
    pub diagnostic_steps: Vec<DiagnosticStep>,
    /// 发现的问题
    pub issues: Vec<Issue>,
    /// 解决方案
    pub solutions: Vec<Solution>,
    /// 执行结果
    pub execution_results: Vec<ExecutionResult>,
    /// 后续建议
    pub recommendations: Vec<String>,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub kernel: String,
    pub cpu_cores: usize,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
    pub disk_total_gb: u64,
    pub disk_available_gb: u64,
    pub uptime: String,
}

/// 诊断步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticStep {
    pub step_number: u32,
    pub name: String,
    pub description: String,
    pub status: String,
    pub output: Option<String>,
    pub duration_ms: Option<u64>,
}

/// 问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub details: Option<String>,
}

/// 解决方案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Solution {
    pub priority: u32,
    pub description: String,
    pub command: Option<String>,
    pub risk: String,
    pub requires_confirmation: bool,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub step: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// 诊断报告工具
pub struct ReportTool {
    metadata: ToolMetadata,
    /// 报告输出目录
    output_dir: PathBuf,
}

impl ReportTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                name: "report".to_string(),
                description: r#"诊断报告生成工具。生成详细的诊断报告，便于问题追踪。

报告内容：
- 系统状态快照
- 问题描述
- 诊断步骤
- 解决方案
- 执行结果
- 后续建议

输出格式：
- json: JSON 格式
- markdown: Markdown 格式
- text: 纯文本格式

Actions:
- generate: 生成诊断报告
- quick: 快速诊断报告
- save: 保存报告到文件

用法示例:
- {"action": "generate", "problem": "服务无法启动"} - 生成完整报告
- {"action": "quick"} - 快速诊断
- {"action": "save", "format": "markdown"} - 保存报告
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["generate", "quick", "save"],
                            "description": "操作类型"
                        },
                        "problem": {
                            "type": "string",
                            "description": "问题描述"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["json", "markdown", "text"],
                            "description": "输出格式（默认 json）"
                        },
                        "output_path": {
                            "type": "string",
                            "description": "输出文件路径（save 时使用）"
                        }
                    },
                    "required": ["action"]
                }),
            },
            output_dir: PathBuf::from("/var/lib/newclaw/reports"),
        }
    }
    
    /// 收集系统信息
    fn collect_system_info(&self) -> SystemInfo {
        let mut info = SystemInfo {
            hostname: "unknown".to_string(),
            os: "unknown".to_string(),
            kernel: "unknown".to_string(),
            cpu_cores: 1,
            total_memory_mb: 0,
            available_memory_mb: 0,
            disk_total_gb: 0,
            disk_available_gb: 0,
            uptime: "unknown".to_string(),
        };
        
        // CPU 核心数
        if let Ok(output) = Command::new("nproc").output() {
            if let Ok(n) = String::from_utf8_lossy(&output.stdout).trim().parse::<usize>() {
                info.cpu_cores = n;
            }
        }
        
        // 主机名
        if let Ok(output) = Command::new("hostname").output() {
            info.hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        
        // 操作系统
        if let Ok(output) = Command::new("uname").args(["-s"]).output() {
            info.os = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        
        // 内核版本
        if let Ok(output) = Command::new("uname").args(["-r"]).output() {
            info.kernel = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        
        // 内存信息
        if let Ok(output) = Command::new("free").args(["-m"]).output() {
            let mem_info = String::from_utf8_lossy(&output.stdout);
            for line in mem_info.lines() {
                if line.starts_with("Mem:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        info.total_memory_mb = parts[1].parse().unwrap_or(0);
                        info.available_memory_mb = parts[6].parse().unwrap_or(0);
                    }
                }
            }
        }
        
        // 磁盘信息
        if let Ok(output) = Command::new("df").args(["-BG", "/"]).output() {
            let df_info = String::from_utf8_lossy(&output.stdout);
            for line in df_info.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    info.disk_total_gb = parts[1].trim_end_matches('G').parse().unwrap_or(0);
                    info.disk_available_gb = parts[3].trim_end_matches('G').parse().unwrap_or(0);
                }
            }
        }
        
        // 运行时间
        if let Ok(output) = Command::new("uptime").args(["-p"]).output() {
            info.uptime = String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        
        info
    }
    
    /// 运行诊断步骤
    fn run_diagnostics(&self) -> Vec<DiagnosticStep> {
        let mut steps = Vec::new();
        let mut step_number = 0;
        
        // 步骤 1: 检查内存
        step_number += 1;
        let start = std::time::Instant::now();
        let memory_ok = self.check_memory();
        steps.push(DiagnosticStep {
            step_number,
            name: "内存检查".to_string(),
            description: "检查系统内存使用情况".to_string(),
            status: if memory_ok { "pass".to_string() } else { "warning".to_string() },
            output: Some(format!("内存使用率: 需检查")),
            duration_ms: Some(start.elapsed().as_millis() as u64),
        });
        
        // 步骤 2: 检查磁盘
        step_number += 1;
        let start = std::time::Instant::now();
        let disk_ok = self.check_disk();
        steps.push(DiagnosticStep {
            step_number,
            name: "磁盘检查".to_string(),
            description: "检查磁盘空间使用情况".to_string(),
            status: if disk_ok { "pass".to_string() } else { "warning".to_string() },
            output: Some(format!("磁盘使用率: 需检查")),
            duration_ms: Some(start.elapsed().as_millis() as u64),
        });
        
        // 步骤 3: 检查服务
        step_number += 1;
        let start = std::time::Instant::now();
        let services = self.check_services();
        steps.push(DiagnosticStep {
            step_number,
            name: "服务检查".to_string(),
            description: "检查关键服务运行状态".to_string(),
            status: if services.is_empty() { "pass".to_string() } else { "warning".to_string() },
            output: if services.is_empty() {
                Some("所有服务正常运行".to_string())
            } else {
                Some(format!("失败服务: {}", services.join(", ")))
            },
            duration_ms: Some(start.elapsed().as_millis() as u64),
        });
        
        // 步骤 4: 检查日志
        step_number += 1;
        let start = std::time::Instant::now();
        let error_count = self.check_logs();
        steps.push(DiagnosticStep {
            step_number,
            name: "日志检查".to_string(),
            description: "检查最近的错误日志".to_string(),
            status: if error_count < 10 { "pass".to_string() } else { "warning".to_string() },
            output: Some(format!("发现 {} 条错误日志", error_count)),
            duration_ms: Some(start.elapsed().as_millis() as u64),
        });
        
        steps
    }
    
    fn check_memory(&self) -> bool {
        if let Ok(output) = Command::new("free").args(["-m"]).output() {
            let mem_info = String::from_utf8_lossy(&output.stdout);
            // 简单检查：不包含 90% 或 95%
            !mem_info.contains("90%") && !mem_info.contains("95%")
        } else {
            true
        }
    }
    
    fn check_disk(&self) -> bool {
        if let Ok(output) = Command::new("df").args(["-h", "/"]).output() {
            let df_info = String::from_utf8_lossy(&output.stdout);
            !df_info.contains("90%") && !df_info.contains("95%")
        } else {
            true
        }
    }
    
    fn check_services(&self) -> Vec<String> {
        let mut failed = Vec::new();
        let services = vec!["openclaw-gateway", "newclaw-gateway"];
        
        for svc in services {
            if let Ok(output) = Command::new("systemctl")
                .args(["is-active", svc])
                .output()
            {
                let status = String::from_utf8_lossy(&output.stdout);
                if status.trim() == "failed" {
                    failed.push(svc.to_string());
                }
            }
        }
        
        failed
    }
    
    fn check_logs(&self) -> usize {
        if let Ok(output) = Command::new("journalctl")
            .args(["-n", "100", "--no-pager", "-p", "err"])
            .output()
        {
            String::from_utf8_lossy(&output.stdout).lines().count()
        } else {
            0
        }
    }
    
    /// 生成问题列表
    fn identify_issues(&self, steps: &[DiagnosticStep]) -> Vec<Issue> {
        let mut issues = Vec::new();
        
        for step in steps {
            if step.status != "pass" {
                issues.push(Issue {
                    severity: if step.status == "fail" { "high".to_string() } else { "medium".to_string() },
                    category: step.name.clone(),
                    description: format!("{} 检测到问题", step.name),
                    details: step.output.clone(),
                });
            }
        }
        
        issues
    }
    
    /// 生成解决方案
    fn generate_solutions(&self, issues: &[Issue]) -> Vec<Solution> {
        let mut solutions = Vec::new();
        
        for issue in issues {
            match issue.category.as_str() {
                "内存检查" => {
                    solutions.push(Solution {
                        priority: 1,
                        description: "清理缓存或增加内存".to_string(),
                        command: Some("sync && echo 3 > /proc/sys/vm/drop_caches".to_string()),
                        risk: "low".to_string(),
                        requires_confirmation: true,
                    });
                }
                "磁盘检查" => {
                    solutions.push(Solution {
                        priority: 1,
                        description: "清理不必要的文件或扩展磁盘".to_string(),
                        command: Some("du -sh /* 2>/dev/null | sort -h".to_string()),
                        risk: "low".to_string(),
                        requires_confirmation: false,
                    });
                }
                "服务检查" => {
                    solutions.push(Solution {
                        priority: 2,
                        description: "重启失败的服务".to_string(),
                        command: Some("systemctl restart <service>".to_string()),
                        risk: "medium".to_string(),
                        requires_confirmation: true,
                    });
                }
                "日志检查" => {
                    solutions.push(Solution {
                        priority: 3,
                        description: "检查日志详情定位问题".to_string(),
                        command: Some("journalctl -xe".to_string()),
                        risk: "low".to_string(),
                        requires_confirmation: false,
                    });
                }
                _ => {}
            }
        }
        
        solutions
    }
    
    /// 生成后续建议
    fn generate_recommendations(&self, issues: &[Issue]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if issues.is_empty() {
            recommendations.push("系统状态良好，建议定期监控".to_string());
        } else {
            recommendations.push("建议尽快处理发现的问题".to_string());
            recommendations.push("处理前建议备份重要数据".to_string());
            recommendations.push("如问题持续，建议查看详细日志".to_string());
        }
        
        recommendations.push("定期运行诊断，预防问题".to_string());
        
        recommendations
    }
    
    /// 生成完整报告
    fn generate_report(&self, problem: Option<&str>) -> DiagnosticReport {
        let system_info = self.collect_system_info();
        let diagnostic_steps = self.run_diagnostics();
        let issues = self.identify_issues(&diagnostic_steps);
        let solutions = self.generate_solutions(&issues);
        let recommendations = self.generate_recommendations(&issues);
        
        DiagnosticReport {
            id: format!("report-{}", uuid::Uuid::new_v4()),
            generated_at: Utc::now(),
            system_info,
            problem: problem.map(|s| s.to_string()),
            diagnostic_steps,
            issues,
            solutions,
            execution_results: Vec::new(),
            recommendations,
        }
    }
    
    /// 转换为 Markdown 格式
    fn to_markdown(&self, report: &DiagnosticReport) -> String {
        let mut md = String::new();
        
        md.push_str(&format!("# 诊断报告 {}\n\n", report.id));
        md.push_str(&format!("**生成时间**: {}\n\n", report.generated_at.to_rfc3339()));
        
        // 系统信息
        md.push_str("## 系统信息\n\n");
        md.push_str(&format!("- **主机名**: {}\n", report.system_info.hostname));
        md.push_str(&format!("- **操作系统**: {} {}\n", report.system_info.os, report.system_info.kernel));
        md.push_str(&format!("- **CPU 核心数**: {}\n", report.system_info.cpu_cores));
        md.push_str(&format!("- **内存**: {} MB / {} MB\n", 
            report.system_info.available_memory_mb, report.system_info.total_memory_mb));
        md.push_str(&format!("- **磁盘**: {} GB / {} GB\n",
            report.system_info.disk_available_gb, report.system_info.disk_total_gb));
        md.push_str(&format!("- **运行时间**: {}\n\n", report.system_info.uptime));
        
        // 问题描述
        if let Some(problem) = &report.problem {
            md.push_str(&format!("## 问题描述\n\n{}\n\n", problem));
        }
        
        // 诊断步骤
        md.push_str("## 诊断步骤\n\n");
        for step in &report.diagnostic_steps {
            let status = match step.status.as_str() {
                "pass" => "✅",
                "warning" => "⚠️",
                "fail" => "❌",
                _ => "❓",
            };
            md.push_str(&format!("{}. {} **{}** {}\n", 
                step.step_number, status, step.name, step.status));
            if let Some(output) = &step.output {
                md.push_str(&format!("   - {}\n", output));
            }
        }
        md.push_str("\n");
        
        // 发现的问题
        if !report.issues.is_empty() {
            md.push_str("## 发现的问题\n\n");
            for issue in &report.issues {
                md.push_str(&format!("- **[{}]** {}: {}\n",
                    issue.severity.to_uppercase(), issue.category, issue.description));
            }
            md.push_str("\n");
        }
        
        // 解决方案
        if !report.solutions.is_empty() {
            md.push_str("## 建议解决方案\n\n");
            for solution in &report.solutions {
                md.push_str(&format!("{}. {} (风险: {})\n",
                    solution.priority, solution.description, solution.risk));
                if let Some(cmd) = &solution.command {
                    md.push_str(&format!("   ```bash\n   {}\n   ```\n", cmd));
                }
            }
            md.push_str("\n");
        }
        
        // 后续建议
        md.push_str("## 后续建议\n\n");
        for rec in &report.recommendations {
            md.push_str(&format!("- {}\n", rec));
        }
        
        md
    }
}

impl Default for ReportTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReportTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;
        
        info!("Report tool called with action: {}", action);
        
        let result = match action {
            "generate" => {
                let problem = args.get("problem").and_then(|p| p.as_str());
                let format = args.get("format").and_then(|f| f.as_str()).unwrap_or("json");
                
                let report = self.generate_report(problem);
                
                match format {
                    "markdown" => json!({
                        "success": true,
                        "format": "markdown",
                        "report": self.to_markdown(&report),
                        "id": report.id
                    }),
                    "text" => json!({
                        "success": true,
                        "format": "text",
                        "report": format!("{:?}", report),
                        "id": report.id
                    }),
                    _ => serde_json::to_value(report).unwrap_or_default()
                }
            }
            "quick" => {
                let system_info = self.collect_system_info();
                json!({
                    "success": true,
                    "system": system_info,
                    "status": if system_info.available_memory_mb > 500 { "healthy" } else { "warning" },
                    "timestamp": Utc::now().to_rfc3339()
                })
            }
            "save" => {
                let format = args.get("format").and_then(|f| f.as_str()).unwrap_or("markdown");
                let output_path = args.get("output_path")
                    .and_then(|p| p.as_str())
                    .map(|s| PathBuf::from(s))
                    .unwrap_or_else(|| {
                        self.output_dir.join(format!("report-{}.md", Utc::now().format("%Y%m%d-%H%M%S")))
                    });
                
                let report = self.generate_report(None);
                let content = match format {
                    "json" => serde_json::to_string_pretty(&report).unwrap_or_default(),
                    "text" => format!("{:?}", report),
                    _ => self.to_markdown(&report),
                };
                
                // 确保目录存在
                if let Some(parent) = output_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                
                match std::fs::write(&output_path, &content) {
                    Ok(_) => json!({
                        "success": true,
                        "path": output_path.to_string_lossy(),
                        "format": format,
                        "report_id": report.id
                    }),
                    Err(e) => json!({
                        "success": false,
                        "error": format!("写入文件失败: {}", e)
                    })
                }
            }
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
    fn test_report_tool_metadata() {
        let tool = ReportTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "report");
    }
    
    #[tokio::test]
    async fn test_report_tool_quick() {
        let tool = ReportTool::new();
        let result = tool.execute(json!({"action": "quick"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_report_tool_generate() {
        let tool = ReportTool::new();
        let result = tool.execute(json!({"action": "generate", "problem": "测试"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
}