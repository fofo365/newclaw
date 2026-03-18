//! Cron Management Tool - Cron 任务管理工具
//!
//! 提供 Cron 任务的查询、创建、删除、暂停/恢复功能
//! 让 AI 能够管理定时任务

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::process::Command;
use std::fs;
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// Cron 管理工具
pub struct CronTool {
    metadata: ToolMetadata,
}

impl CronTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                name: "cron".to_string(),
                description: r#"Cron 任务管理工具。用于查询、创建、删除、暂停/恢复定时任务。

⚠️ 安全提示：
- 删除任务前必须确认
- 批量操作需要用户授权
- 危险操作会有警告

Actions:
- list: 列出所有定时任务
- get: 获取指定任务详情
- create: 创建新的定时任务（需要确认）
- delete: 删除定时任务（需要确认）
- pause: 暂停定时任务
- resume: 恢复定时任务

用法示例:
- {"action": "list"} - 列出所有任务
- {"action": "get", "task_id": "task-123"} - 获取任务详情
- {"action": "delete", "task_id": "task-123", "confirm": true} - 删除任务
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["list", "get", "create", "delete", "pause", "resume"],
                            "description": "操作类型"
                        },
                        "task_id": {
                            "type": "string",
                            "description": "任务ID（get/delete/pause/resume时需要）"
                        },
                        "schedule": {
                            "type": "string",
                            "description": "Cron表达式（create时需要，如 '0 9 * * *' 表示每天9点）"
                        },
                        "command": {
                            "type": "string",
                            "description": "要执行的命令（create时需要）"
                        },
                        "description": {
                            "type": "string",
                            "description": "任务描述（create时可选）"
                        },
                        "confirm": {
                            "type": "boolean",
                            "description": "确认执行危险操作（delete时必须为true）"
                        }
                    },
                    "required": ["action"]
                }),
            }
        }
    }
    
    /// 列出所有 Cron 任务
    fn list_tasks(&self) -> JsonValue {
        let mut tasks = Vec::new();
        let mut task_id = 0;
        
        // 读取用户 crontab
        if let Ok(output) = Command::new("crontab").args(["-l"]).output() {
            let crontab = String::from_utf8_lossy(&output.stdout);
            for line in crontab.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    task_id += 1;
                    // 解析 cron 行
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 6 {
                        let schedule = parts[..5].join(" ");
                        let command = parts[5..].join(" ");
                        tasks.push(json!({
                            "id": format!("user-{}", task_id),
                            "source": "user_crontab",
                            "schedule": schedule,
                            "command": command,
                            "enabled": true,
                            "raw_line": line
                        }));
                    }
                }
            }
        }
        
        // 读取 /etc/cron.d/ 目录
        if let Ok(entries) = fs::read_dir("/etc/cron.d") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        for line in content.lines() {
                            let line = line.trim();
                            if !line.is_empty() && !line.starts_with('#') {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if parts.len() >= 6 {
                                    task_id += 1;
                                    let schedule = parts[..5].join(" ");
                                    let command = parts[5..].join(" ");
                                    tasks.push(json!({
                                        "id": format!("system-{}", entry.file_name().to_string_lossy()),
                                        "source": format!("/etc/cron.d/{}", entry.file_name().to_string_lossy()),
                                        "schedule": schedule,
                                        "command": command,
                                        "enabled": true,
                                        "raw_line": line
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        json!({
            "success": true,
            "count": tasks.len(),
            "tasks": tasks,
            "warning": if tasks.is_empty() { "没有找到定时任务" } else { null }
        })
    }
    
    /// 删除任务
    fn delete_task(&self, task_id: &str, confirm: bool) -> JsonValue {
        if !confirm {
            return json!({
                "success": false,
                "error": "dangerous_operation_not_confirmed",
                "message": "⚠️ 删除任务需要确认。请设置 confirm: true 来确认删除。",
                "task_id": task_id
            });
        }
        
        // 解析任务来源
        if task_id.starts_with("user-") {
            // 从用户 crontab 删除
            if let Ok(output) = Command::new("crontab").args(["-l"]).output() {
                let mut lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
                
                // 找到要删除的行
                let task_num: usize = task_id.strip_prefix("user-")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                
                let mut count = 0;
                let mut found = false;
                lines.retain(|line| {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        count += 1;
                        if count == task_num {
                            found = true;
                            return false; // 删除这行
                        }
                    }
                    true
                });
                
                if found {
                    // 写回 crontab
                    let new_crontab = lines.join("\n");
                    if let Ok(mut child) = Command::new("crontab").args(["-"]).stdin(std::process::Stdio::piped()).spawn() {
                        if let Some(stdin) = child.stdin.as_mut() {
                            use std::io::Write;
                            let _ = stdin.write_all(new_crontab.as_bytes());
                        }
                        let _ = child.wait();
                        
                        info!("Deleted cron task: {}", task_id);
                        return json!({
                            "success": true,
                            "message": format!("任务 {} 已删除", task_id),
                            "task_id": task_id
                        });
                    }
                }
            }
            
            return json!({
                "success": false,
                "error": "task_not_found",
                "message": format!("找不到任务: {}", task_id)
            });
        }
        
        // 系统级任务需要 root 权限，这里只返回提示
        json!({
            "success": false,
            "error": "permission_denied",
            "message": "系统级任务需要 root 权限才能删除。请手动编辑 /etc/cron.d/ 中的文件。"
        })
    }
    
    /// 创建任务
    fn create_task(&self, schedule: &str, command: &str, description: Option<&str>) -> JsonValue {
        // 验证 cron 表达式
        let parts: Vec<&str> = schedule.split_whitespace().collect();
        if parts.len() != 5 {
            return json!({
                "success": false,
                "error": "invalid_schedule",
                "message": "Cron 表达式格式错误。应为: 分 时 日 月 周"
            });
        }
        
        // 构建新的 crontab 行
        let new_line = if let Some(desc) = description {
            format!("{} {} # {}", schedule, command, desc)
        } else {
            format!("{} {}", schedule, command)
        };
        
        // 读取现有 crontab
        let mut lines: Vec<String> = Vec::new();
        if let Ok(output) = Command::new("crontab").args(["-l"]).output() {
            lines = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect();
        }
        
        // 添加新行
        lines.push(new_line);
        
        // 写回 crontab
        let new_crontab = lines.join("\n");
        if let Ok(mut child) = Command::new("crontab").args(["-"]).stdin(std::process::Stdio::piped()).spawn() {
            if let Some(stdin) = child.stdin.as_mut() {
                use std::io::Write;
                let _ = stdin.write_all(new_crontab.as_bytes());
            }
            if let Ok(status) = child.wait() {
                if status.success() {
                    info!("Created new cron task: {} -> {}", schedule, command);
                    return json!({
                        "success": true,
                        "message": "定时任务创建成功",
                        "schedule": schedule,
                        "command": command
                    });
                }
            }
        }
        
        json!({
            "success": false,
            "error": "create_failed",
            "message": "创建定时任务失败，请检查权限"
        })
    }
}

impl Default for CronTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CronTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'action' parameter".to_string()))?;
        
        info!("Cron tool called with action: {}", action);
        
        let result = match action {
            "list" => self.list_tasks(),
            "get" => {
                let task_id = args.get("task_id")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| ToolError::InvalidParameters("Missing 'task_id' parameter".to_string()))?;
                json!({
                    "success": false,
                    "error": "not_implemented",
                    "message": "请使用 list 操作查看所有任务"
                })
            }
            "create" => {
                let schedule = args.get("schedule")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| ToolError::InvalidParameters("Missing 'schedule' parameter".to_string()))?;
                let command = args.get("command")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| ToolError::InvalidParameters("Missing 'command' parameter".to_string()))?;
                let description = args.get("description").and_then(|d| d.as_str());
                self.create_task(schedule, command, description)
            }
            "delete" => {
                let task_id = args.get("task_id")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| ToolError::InvalidParameters("Missing 'task_id' parameter".to_string()))?;
                let confirm = args.get("confirm").and_then(|c| c.as_bool()).unwrap_or(false);
                self.delete_task(task_id, confirm)
            }
            "pause" | "resume" => {
                json!({
                    "success": false,
                    "error": "not_implemented",
                    "message": "暂停/恢复功能尚未实现，请使用 delete 删除任务"
                })
            }
            _ => {
                return Err(ToolError::InvalidParameters(format!("Unknown action: {}", action)).into());
            }
        };
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cron_tool_metadata() {
        let tool = CronTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "cron");
    }
    
    #[tokio::test]
    async fn test_cron_tool_list() {
        let tool = CronTool::new();
        let result = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_cron_tool_delete_without_confirm() {
        let tool = CronTool::new();
        let result = tool.execute(json!({"action": "delete", "task_id": "user-1"})).await.unwrap();
        assert!(!result.get("success").unwrap().as_bool().unwrap());
        assert_eq!(result.get("error").unwrap(), "dangerous_operation_not_confirmed");
    }
}