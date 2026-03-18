//! Safety Guard Tool - AI 行为约束保护工具
//!
//! 防止 AI 采取极端措施，确保危险操作需要用户确认
//! 来源: CHANGELOG-v0.7.1.md P0-3 需求

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::HashSet;
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// 危险操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DangerousAction {
    /// 删除所有记忆
    DeleteAllMemory,
    /// 删除文件
    DeleteFile,
    /// 批量删除
    BatchDelete,
    /// 修改系统配置
    ModifySystemConfig,
    /// 重启服务
    RestartService,
    /// 执行 shell 命令
    ExecuteShell,
    /// 发送外部消息
    SendExternalMessage,
    /// 修改权限
    ModifyPermissions,
}

/// 安全检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    /// 是否安全
    pub is_safe: bool,
    /// 风险等级
    pub risk_level: String,
    /// 警告消息
    pub warnings: Vec<String>,
    /// 需要确认的操作
    pub requires_confirmation: bool,
    /// 确认提示
    pub confirmation_prompt: Option<String>,
    /// 替代方案
    pub alternatives: Vec<String>,
}

/// 安全保护工具
pub struct SafetyTool {
    metadata: ToolMetadata,
    /// 危险关键词
    dangerous_keywords: HashSet<String>,
}

impl SafetyTool {
    pub fn new() -> Self {
        let mut keywords = HashSet::new();
        keywords.insert("删除所有".to_string());
        keywords.insert("删除全部".to_string());
        keywords.insert("清空记忆".to_string());
        keywords.insert("清空所有".to_string());
        keywords.insert("rm -rf".to_string());
        keywords.insert("DROP TABLE".to_string());
        keywords.insert("DELETE FROM".to_string());
        keywords.insert("truncate".to_string());
        keywords.insert("format".to_string());
        keywords.insert("初始化".to_string());
        keywords.insert("重置".to_string());
        
        Self {
            metadata: ToolMetadata {
                name: "safety".to_string(),
                description: r#"AI 行为约束保护工具。检查危险操作，防止极端措施。

⚠️ 安全规则：
- 禁止删除所有记忆（必须用户明确要求+二次确认）
- 删除操作必须询问用户
- 批量操作必须有数量限制
- 不可逆操作必须有警告

Actions:
- check: 检查操作是否安全
- confirm: 确认执行危险操作
- get_alternatives: 获取安全替代方案

用法示例:
- {"action": "check", "operation": "删除所有记忆"} - 检查操作安全性
- {"action": "confirm", "operation_id": "xxx", "user_confirmed": true} - 确认执行
- {"action": "get_alternatives", "operation": "删除所有记忆"} - 获取替代方案
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["check", "confirm", "get_alternatives"],
                            "description": "操作类型"
                        },
                        "operation": {
                            "type": "string",
                            "description": "要检查的操作描述"
                        },
                        "operation_id": {
                            "type": "string",
                            "description": "操作ID（confirm时使用）"
                        },
                        "user_confirmed": {
                            "type": "boolean",
                            "description": "用户是否确认"
                        }
                    },
                    "required": ["action"]
                }),
            },
            dangerous_keywords: keywords,
        }
    }
    
    /// 检查操作安全性
    fn check_operation(&self, operation: &str) -> SafetyCheckResult {
        let mut warnings = Vec::new();
        let mut risk_level = "low".to_string();
        let mut requires_confirmation = false;
        let mut confirmation_prompt = None;
        let mut alternatives = Vec::new();
        
        // 检查危险关键词
        let operation_lower = operation.to_lowercase();
        for keyword in &self.dangerous_keywords {
            if operation_lower.contains(&keyword.to_lowercase()) {
                warnings.push(format!("检测到危险关键词: {}", keyword));
                risk_level = "high".to_string();
                requires_confirmation = true;
            }
        }
        
        // 特殊检查：删除所有记忆
        if operation.contains("所有") && (operation.contains("删除") || operation.contains("清空")) {
            warnings.push("⚠️ 警告：此操作将删除所有数据，不可恢复！".to_string());
            risk_level = "critical".to_string();
            requires_confirmation = true;
            confirmation_prompt = Some(
                "⚠️ **危险操作警告**\n\n\
                您即将执行的操作将永久删除所有数据，无法恢复！\n\n\
                **建议替代方案：**\n\
                1. 归档旧数据而非删除\n\
                2. 仅删除特定时间范围内的数据\n\
                3. 导出备份后再删除\n\n\
                如果您确定要继续，请明确回复\"确认删除所有数据\"。".to_string()
            );
            alternatives.push("归档旧数据而非删除".to_string());
            alternatives.push("仅删除特定内容".to_string());
            alternatives.push("导出备份后再操作".to_string());
        }
        
        // 检查批量操作
        if operation.contains("批量") || operation.contains("全部") {
            warnings.push("批量操作可能影响大量数据".to_string());
            if risk_level == "low" {
                risk_level = "medium".to_string();
            }
            requires_confirmation = true;
        }
        
        // 检查系统级操作
        if operation.contains("系统") || operation.contains("配置") || operation.contains("服务") {
            if operation.contains("重启") || operation.contains("停止") || operation.contains("修改") {
                warnings.push("系统级操作可能影响服务稳定性".to_string());
                if risk_level == "low" {
                    risk_level = "medium".to_string();
                }
                requires_confirmation = true;
            }
        }
        
        SafetyCheckResult {
            is_safe: risk_level == "low",
            risk_level,
            warnings,
            requires_confirmation,
            confirmation_prompt,
            alternatives,
        }
    }
    
    /// 获取替代方案
    fn get_alternatives(&self, operation: &str) -> JsonValue {
        let mut alternatives = Vec::new();
        
        if operation.contains("记忆") {
            if operation.contains("删除") || operation.contains("清空") {
                alternatives.push(json!({
                    "action": "archive",
                    "description": "归档旧记忆到历史文件",
                    "command": "memory archive --older-than 30d",
                    "risk": "low"
                }));
                alternatives.push(json!({
                    "action": "search_and_delete",
                    "description": "搜索并删除特定内容",
                    "command": "memory search <关键词> --delete",
                    "risk": "medium"
                }));
                alternatives.push(json!({
                    "action": "export_and_backup",
                    "description": "导出备份后再操作",
                    "command": "memory export backup.json",
                    "risk": "low"
                }));
            }
        }
        
        if operation.contains("定时") || operation.contains("cron") {
            alternatives.push(json!({
                "action": "list_first",
                "description": "先列出所有任务，再决定删除哪个",
                "command": "cron list",
                "risk": "low"
            }));
            alternatives.push(json!({
                "action": "pause",
                "description": "暂停任务而非删除",
                "command": "cron pause <task_id>",
                "risk": "low"
            }));
        }
        
        if operation.contains("服务") {
            alternatives.push(json!({
                "action": "check_status",
                "description": "检查服务状态",
                "command": "systemctl status <service>",
                "risk": "low"
            }));
            alternatives.push(json!({
                "action": "check_logs",
                "description": "查看服务日志定位问题",
                "command": "journalctl -u <service>",
                "risk": "low"
            }));
        }
        
        json!({
            "success": true,
            "operation": operation,
            "alternatives": alternatives,
            "recommendation": if alternatives.is_empty() {
                "请描述更具体的操作，以便提供替代方案"
            } else {
                "建议先尝试低风险的替代方案"
            }
        })
    }
    
    /// 确认危险操作
    fn confirm_operation(&self, operation_id: &str, user_confirmed: bool) -> JsonValue {
        if !user_confirmed {
            return json!({
                "success": false,
                "error": "operation_not_confirmed",
                "message": "用户未确认，操作已取消"
            });
        }
        
        // 记录确认日志
        info!("Dangerous operation confirmed: {}", operation_id);
        
        json!({
            "success": true,
            "operation_id": operation_id,
            "message": "操作已确认，可以执行",
            "warning": "请确保您了解此操作的后果"
        })
    }
}

impl Default for SafetyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SafetyTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;
        
        info!("Safety tool called with action: {}", action);
        
        let result = match action {
            "check" => {
                let operation = args.get("operation")
                    .and_then(|o| o.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'operation' parameter".to_string()))?;
                serde_json::to_value(self.check_operation(operation)).unwrap_or_default()
            }
            "confirm" => {
                let operation_id = args.get("operation_id")
                    .and_then(|o| o.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'operation_id' parameter".to_string()))?;
                let user_confirmed = args.get("user_confirmed").and_then(|c| c.as_bool()).unwrap_or(false);
                self.confirm_operation(operation_id, user_confirmed)
            }
            "get_alternatives" => {
                let operation = args.get("operation")
                    .and_then(|o| o.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'operation' parameter".to_string()))?;
                self.get_alternatives(operation)
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
    fn test_safety_tool_metadata() {
        let tool = SafetyTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "safety");
    }
    
    #[test]
    fn test_check_dangerous_operation() {
        let tool = SafetyTool::new();
        let result = tool.check_operation("删除所有记忆");
        assert!(!result.is_safe);
        assert_eq!(result.risk_level, "critical");
        assert!(result.requires_confirmation);
    }
    
    #[test]
    fn test_check_safe_operation() {
        let tool = SafetyTool::new();
        let result = tool.check_operation("查看系统状态");
        assert!(result.is_safe);
        assert_eq!(result.risk_level, "low");
    }
    
    #[tokio::test]
    async fn test_safety_tool_check() {
        let tool = SafetyTool::new();
        let result = tool.execute(json!({"action": "check", "operation": "删除所有记忆"})).await.unwrap();
        assert!(!result.get("is_safe").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_safety_tool_alternatives() {
        let tool = SafetyTool::new();
        let result = tool.execute(json!({"action": "get_alternatives", "operation": "删除所有记忆"})).await.unwrap();
        assert!(result.get("alternatives").unwrap().as_array().unwrap().len() > 0);
    }
}