//! Diagnostic Tool - 系统诊断工具
//!
//! 提供系统状态检查、问题诊断、修复建议等功能
//! 让 AI 能够诊断和解决系统问题

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::process::Command;
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// 诊断工具
pub struct DiagnosticTool {
    metadata: ToolMetadata,
}

impl DiagnosticTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                name: "diagnostic".to_string(),
                description: r#"系统诊断工具。用于检查系统状态、诊断问题、提供修复建议。

Actions:
- status: 检查系统整体状态（内存、CPU、磁盘、服务）
- check_cron: 检查定时任务列表
- check_logs: 检查最近的错误日志
- check_services: 检查服务运行状态
- full_diagnosis: 执行完整诊断，返回详细报告

用法示例:
- {"action": "status"} - 检查系统状态
- {"action": "check_cron"} - 查看定时任务
- {"action": "check_logs", "lines": 100} - 查看最近100行日志
- {"action": "full_diagnosis"} - 完整诊断报告
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["status", "check_cron", "check_logs", "check_services", "full_diagnosis"],
                            "description": "诊断操作类型"
                        },
                        "lines": {
                            "type": "integer",
                            "description": "日志行数（check_logs时使用，默认100）"
                        },
                        "service": {
                            "type": "string",
                            "description": "服务名称（可选，用于检查特定服务）"
                        }
                    },
                    "required": ["action"]
                }),
            }
        }
    }
    
    /// 检查系统状态
    fn check_system_status(&self) -> JsonValue {
        let mut result = serde_json::Map::new();
        
        // 内存状态
        if let Ok(output) = Command::new("free").args(["-m"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            result.insert("memory".to_string(), json!({
                "raw": output_str.lines().take(2).collect::<Vec<_>>().join("\n"),
                "status": "ok"
            }));
        }
        
        // CPU 负载
        if let Ok(output) = Command::new("cat").args(["/proc/loadavg"]).output() {
            let loadavg = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = loadavg.split_whitespace().take(3).collect();
            result.insert("cpu".to_string(), json!({
                "load_1m": parts.get(0).unwrap_or(&"0"),
                "load_5m": parts.get(1).unwrap_or(&"0"),
                "load_15m": parts.get(2).unwrap_or(&"0"),
                "status": "ok"
            }));
        }
        
        // 磁盘使用
        if let Ok(output) = Command::new("df").args(["-h", "/"]).output() {
            let df_output = String::from_utf8_lossy(&output.stdout);
            result.insert("disk".to_string(), json!({
                "raw": df_output.lines().skip(1).next().unwrap_or("unknown"),
                "status": "ok"
            }));
        }
        
        json!({
            "success": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "system": result
        })
    }
    
    /// 检查 Cron 任务
    fn check_cron_tasks(&self) -> JsonValue {
        let mut tasks = Vec::new();
        
        // 检查用户 crontab
        if let Ok(output) = Command::new("crontab").args(["-l"]).output() {
            let crontab = String::from_utf8_lossy(&output.stdout);
            for line in crontab.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    tasks.push(json!({
                        "source": "user_crontab",
                        "schedule": line
                    }));
                }
            }
        }
        
        // 检查 /etc/cron.d/
        if let Ok(entries) = std::fs::read_dir("/etc/cron.d") {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            tasks.push(json!({
                                "source": format!("/etc/cron.d/{}", entry.file_name().to_string_lossy()),
                                "schedule": line
                            }));
                        }
                    }
                }
            }
        }
        
        json!({
            "success": true,
            "count": tasks.len(),
            "tasks": tasks
        })
    }
    
    /// 检查日志
    fn check_logs(&self, lines: usize) -> JsonValue {
        let mut logs: Vec<String> = Vec::new();
        
        // 检查 systemd journal
        if let Ok(output) = Command::new("journalctl")
            .args(["-n", &lines.to_string(), "--no-pager", "-p", "err"])
            .output()
        {
            let journal = String::from_utf8_lossy(&output.stdout);
            for line in journal.lines().take(lines) {
                logs.push(line.to_string());
            }
        }
        
        // 检查 OpenClaw 日志
        let openclaw_log = "/var/log/openclaw/openclaw.log";
        if std::path::Path::new(openclaw_log).exists() {
            if let Ok(output) = Command::new("tail")
                .args(["-n", &lines.to_string(), openclaw_log])
                .output()
            {
                let tail = String::from_utf8_lossy(&output.stdout);
                for line in tail.lines() {
                    if line.contains("error") || line.contains("Error") || line.contains("ERROR") {
                        logs.push(format!("[openclaw] {}", line));
                    }
                }
            }
        }
        
        json!({
            "success": true,
            "count": logs.len(),
            "logs": logs
        })
    }
    
    /// 检查服务状态
    fn check_services(&self, service: Option<&str>) -> JsonValue {
        let mut services = Vec::new();
        
        let services_to_check = if let Some(s) = service {
            vec![s.to_string()]
        } else {
            vec![
                "openclaw-gateway".to_string(),
                "newclaw-gateway".to_string(),
                "newclaw".to_string(),
            ]
        };
        
        for svc in services_to_check {
            if let Ok(output) = Command::new("systemctl")
                .args(["status", &svc, "--no-pager"])
                .output()
            {
                let status = String::from_utf8_lossy(&output.stdout);
                let is_active = status.contains("active (running)");
                let is_failed = status.contains("failed");
                
                services.push(json!({
                    "name": svc,
                    "active": is_active,
                    "failed": is_failed,
                    "status": if is_active { "running" } else if is_failed { "failed" } else { "stopped" }
                }));
            } else {
                services.push(json!({
                    "name": svc,
                    "active": false,
                    "status": "not_found"
                }));
            }
        }
        
        json!({
            "success": true,
            "services": services
        })
    }
    
    /// 完整诊断
    fn full_diagnosis(&self) -> JsonValue {
        let status = self.check_system_status();
        let cron = self.check_cron_tasks();
        let logs = self.check_logs(50);
        let services = self.check_services(None);
        
        // 分析问题
        let mut issues: Vec<String> = Vec::new();
        let mut recommendations: Vec<String> = Vec::new();
        
        // 检查内存
        if let Some(mem) = status.get("system").and_then(|s| s.get("memory")) {
            if let Some(raw) = mem.get("raw").and_then(|r| r.as_str()) {
                if raw.contains("90%") || raw.contains("95%") {
                    issues.push("内存使用率过高".to_string());
                    recommendations.push("考虑清理缓存或增加内存".to_string());
                }
            }
        }
        
        // 检查服务
        if let Some(svc_list) = services.get("services").and_then(|s| s.as_array()) {
            for svc in svc_list {
                if svc.get("failed").and_then(|f| f.as_bool()).unwrap_or(false) {
                    let svc_name = svc.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
                    issues.push(format!("服务 {} 处于失败状态", svc_name));
                    recommendations.push(format!("检查服务 {} 的日志并尝试重启", svc_name));
                }
            }
        }
        
        // 检查日志错误
        if let Some(log_list) = logs.get("logs").and_then(|l| l.as_array()) {
            if log_list.len() > 10 {
                issues.push("发现大量错误日志".to_string());
                recommendations.push("检查日志详情，定位具体问题".to_string());
            }
        }
        
        json!({
            "success": true,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "summary": {
                "issues_found": issues.len(),
                "issues": issues,
                "recommendations": recommendations
            },
            "details": {
                "system": status.get("system"),
                "cron": cron.get("tasks"),
                "logs": logs.get("logs"),
                "services": services.get("services")
            }
        })
    }
}

impl Default for DiagnosticTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DiagnosticTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;
        
        info!("Diagnostic tool called with action: {}", action);
        
        let result = match action {
            "status" => self.check_system_status(),
            "check_cron" => self.check_cron_tasks(),
            "check_logs" => {
                let lines = args.get("lines")
                    .and_then(|l| l.as_u64())
                    .unwrap_or(100) as usize;
                self.check_logs(lines)
            }
            "check_services" => {
                let service = args.get("service").and_then(|s| s.as_str());
                self.check_services(service)
            }
            "full_diagnosis" => self.full_diagnosis(),
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
    fn test_diagnostic_tool_metadata() {
        let tool = DiagnosticTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "diagnostic");
    }
    
    #[tokio::test]
    async fn test_diagnostic_tool_status() {
        let tool = DiagnosticTool::new();
        let result = tool.execute(json!({"action": "status"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_diagnostic_tool_full_diagnosis() {
        let tool = DiagnosticTool::new();
        let result = tool.execute(json!({"action": "full_diagnosis"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
        assert!(result.get("summary").is_some());
    }
}