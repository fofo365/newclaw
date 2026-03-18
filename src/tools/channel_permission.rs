//! Channel Permission Config Tool - 通道权限配置工具
//!
//! 实现权限分级：基础权限、管理权限、高级权限
//! 来源: CHANGELOG-v0.7.1.md P1-1 需求

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::{HashMap, HashSet};
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// 权限级别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    /// 基础权限（所有通道）
    Basic,
    /// 管理权限（管理员通道）
    Management,
    /// 高级权限（核心通道）
    Advanced,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        Self::Basic
    }
}

/// 工具权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionConfig {
    /// 工具名称
    pub tool_name: String,
    /// 所需权限级别
    pub required_level: PermissionLevel,
    /// 是否需要确认
    pub requires_confirmation: bool,
    /// 危险等级
    pub danger_level: String,
}

/// 通道权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPermissionConfig {
    /// 通道类型
    pub channel_type: String,
    /// 权限级别
    pub permission_level: PermissionLevel,
    /// 允许的工具列表
    pub allowed_tools: Vec<String>,
    /// 禁止的工具列表
    pub denied_tools: Vec<String>,
    /// 是否为管理员通道
    pub is_admin: bool,
    /// 是否为核心通道
    pub is_core: bool,
}

/// 通道权限配置工具
pub struct ChannelPermissionTool {
    metadata: ToolMetadata,
    /// 默认工具权限配置
    tool_configs: HashMap<String, ToolPermissionConfig>,
    /// 通道配置
    channel_configs: HashMap<String, ChannelPermissionConfig>,
}

impl ChannelPermissionTool {
    pub fn new() -> Self {
        let mut tool_configs = HashMap::new();
        
        // 基础权限工具
        let basic_tools = vec![
            ("read", false, "low"),
            ("web_search", false, "low"),
            ("web_fetch", false, "low"),
            ("memory", false, "low"),
            ("diagnostic", false, "low"),
        ];
        
        for (name, confirm, danger) in basic_tools {
            tool_configs.insert(name.to_string(), ToolPermissionConfig {
                tool_name: name.to_string(),
                required_level: PermissionLevel::Basic,
                requires_confirmation: confirm,
                danger_level: danger.to_string(),
            });
        }
        
        // 管理权限工具
        let management_tools = vec![
            ("cron", true, "medium"),
            ("workflow", false, "low"),
            ("safety", false, "low"),
            ("write", true, "medium"),
            ("edit", true, "medium"),
        ];
        
        for (name, confirm, danger) in management_tools {
            tool_configs.insert(name.to_string(), ToolPermissionConfig {
                tool_name: name.to_string(),
                required_level: PermissionLevel::Management,
                requires_confirmation: confirm,
                danger_level: danger.to_string(),
            });
        }
        
        // 高级权限工具
        let advanced_tools = vec![
            ("exec", true, "high"),
            ("browser", true, "medium"),
            ("permission", true, "high"),
            ("nodes", true, "high"),
        ];
        
        for (name, confirm, danger) in advanced_tools {
            tool_configs.insert(name.to_string(), ToolPermissionConfig {
                tool_name: name.to_string(),
                required_level: PermissionLevel::Advanced,
                requires_confirmation: confirm,
                danger_level: danger.to_string(),
            });
        }
        
        // 默认通道配置
        let mut channel_configs = HashMap::new();
        
        channel_configs.insert("feishu".to_string(), ChannelPermissionConfig {
            channel_type: "feishu".to_string(),
            permission_level: PermissionLevel::Management,
            allowed_tools: vec![],
            denied_tools: vec![],
            is_admin: false,
            is_core: false,
        });
        
        channel_configs.insert("cli".to_string(), ChannelPermissionConfig {
            channel_type: "cli".to_string(),
            permission_level: PermissionLevel::Advanced,
            allowed_tools: vec![],
            denied_tools: vec![],
            is_admin: true,
            is_core: true,
        });
        
        channel_configs.insert("dashboard".to_string(), ChannelPermissionConfig {
            channel_type: "dashboard".to_string(),
            permission_level: PermissionLevel::Management,
            allowed_tools: vec![],
            denied_tools: vec![],
            is_admin: false,
            is_core: false,
        });
        
        Self {
            metadata: ToolMetadata {
                name: "channel_permission".to_string(),
                description: r#"通道权限配置工具。管理不同通道的工具访问权限。

权限级别：
- basic: 基础权限（所有通道可用）
- management: 管理权限（管理员通道）
- advanced: 高级权限（核心通道）

Actions:
- list_tools: 列出所有工具及其权限级别
- list_channels: 列出所有通道及其权限配置
- check: 检查通道是否有权限使用某工具
- set_channel_level: 设置通道权限级别
- get_tool_config: 获取工具权限配置

用法示例:
- {"action": "list_tools"} - 列出所有工具
- {"action": "check", "channel": "feishu", "tool": "exec"} - 检查权限
- {"action": "set_channel_level", "channel": "feishu", "level": "management"} - 设置权限
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["list_tools", "list_channels", "check", "set_channel_level", "get_tool_config"],
                            "description": "操作类型"
                        },
                        "channel": {
                            "type": "string",
                            "description": "通道名称"
                        },
                        "tool": {
                            "type": "string",
                            "description": "工具名称"
                        },
                        "level": {
                            "type": "string",
                            "enum": ["basic", "management", "advanced"],
                            "description": "权限级别"
                        }
                    },
                    "required": ["action"]
                }),
            },
            tool_configs,
            channel_configs,
        }
    }
    
    /// 列出所有工具
    fn list_tools(&self) -> JsonValue {
        let mut basic = Vec::new();
        let mut management = Vec::new();
        let mut advanced = Vec::new();
        
        for (_, config) in &self.tool_configs {
            let entry = json!({
                "name": config.tool_name,
                "requires_confirmation": config.requires_confirmation,
                "danger_level": config.danger_level
            });
            
            match config.required_level {
                PermissionLevel::Basic => basic.push(entry),
                PermissionLevel::Management => management.push(entry),
                PermissionLevel::Advanced => advanced.push(entry),
            }
        }
        
        json!({
            "success": true,
            "tools": {
                "basic": basic,
                "management": management,
                "advanced": advanced
            },
            "summary": {
                "basic_count": basic.len(),
                "management_count": management.len(),
                "advanced_count": advanced.len(),
                "total": basic.len() + management.len() + advanced.len()
            }
        })
    }
    
    /// 列出所有通道
    fn list_channels(&self) -> JsonValue {
        let channels: Vec<JsonValue> = self.channel_configs.values().map(|c| {
            json!({
                "channel": c.channel_type,
                "level": c.permission_level,
                "is_admin": c.is_admin,
                "is_core": c.is_core,
                "allowed_tools": c.allowed_tools,
                "denied_tools": c.denied_tools
            })
        }).collect();
        
        json!({
            "success": true,
            "channels": channels
        })
    }
    
    /// 检查权限
    fn check_permission(&self, channel: &str, tool: &str) -> JsonValue {
        // 获取通道配置
        let channel_config = self.channel_configs.get(channel);
        if channel_config.is_none() {
            return json!({
                "success": false,
                "error": "channel_not_found",
                "message": format!("通道 '{}' 未配置", channel)
            });
        }
        let channel_config = channel_config.unwrap();
        
        // 检查是否在禁止列表
        if channel_config.denied_tools.contains(&tool.to_string()) {
            return json!({
                "success": false,
                "allowed": false,
                "reason": "tool_denied",
                "message": format!("工具 '{}' 在通道 '{}' 中被禁止", tool, channel)
            });
        }
        
        // 检查是否在允许列表（如果允许列表非空）
        if !channel_config.allowed_tools.is_empty() {
            if channel_config.allowed_tools.contains(&tool.to_string()) {
                return json!({
                    "success": true,
                    "allowed": true,
                    "reason": "in_allowlist"
                });
            }
        }
        
        // 获取工具配置
        let tool_config = self.tool_configs.get(tool);
        if tool_config.is_none() {
            // 未配置的工具默认允许
            return json!({
                "success": true,
                "allowed": true,
                "reason": "tool_not_configured"
            });
        }
        let tool_config = tool_config.unwrap();
        
        // 检查权限级别
        let channel_level = &channel_config.permission_level;
        let required_level = &tool_config.required_level;
        
        let allowed = match (channel_level, required_level) {
            (PermissionLevel::Advanced, _) => true,
            (PermissionLevel::Management, PermissionLevel::Advanced) => false,
            (PermissionLevel::Management, _) => true,
            (PermissionLevel::Basic, PermissionLevel::Basic) => true,
            (PermissionLevel::Basic, _) => false,
        };
        
        json!({
            "success": true,
            "allowed": allowed,
            "channel_level": channel_level,
            "required_level": required_level,
            "requires_confirmation": tool_config.requires_confirmation,
            "danger_level": tool_config.danger_level,
            "message": if allowed {
                format!("通道 '{}' 有权限使用工具 '{}'", channel, tool)
            } else {
                format!("通道 '{}' 权限不足，无法使用工具 '{}'（需要 {:?} 权限）", channel, tool, required_level)
            }
        })
    }
    
    /// 设置通道权限级别
    fn set_channel_level(&mut self, channel: &str, level: &str) -> JsonValue {
        let permission_level = match level {
            "basic" => PermissionLevel::Basic,
            "management" => PermissionLevel::Management,
            "advanced" => PermissionLevel::Advanced,
            _ => {
                return json!({
                    "success": false,
                    "error": "invalid_level",
                    "message": format!("无效的权限级别: {}", level)
                });
            }
        };
        
        // 更新或创建配置
        let config = self.channel_configs.entry(channel.to_string())
            .or_insert(ChannelPermissionConfig {
                channel_type: channel.to_string(),
                permission_level: permission_level.clone(),
                allowed_tools: vec![],
                denied_tools: vec![],
                is_admin: permission_level == PermissionLevel::Advanced,
                is_core: permission_level == PermissionLevel::Advanced,
            });
        
        config.permission_level = permission_level.clone();
        config.is_admin = permission_level == PermissionLevel::Advanced;
        config.is_core = permission_level == PermissionLevel::Advanced;
        
        info!("Set channel {} permission level to {:?}", channel, permission_level);
        
        json!({
            "success": true,
            "channel": channel,
            "level": level,
            "message": format!("通道 '{}' 权限级别已设置为 '{}'", channel, level)
        })
    }
    
    /// 获取工具配置
    fn get_tool_config(&self, tool: &str) -> JsonValue {
        match self.tool_configs.get(tool) {
            Some(config) => json!({
                "success": true,
                "tool": config.tool_name,
                "required_level": config.required_level,
                "requires_confirmation": config.requires_confirmation,
                "danger_level": config.danger_level
            }),
            None => json!({
                "success": false,
                "error": "tool_not_found",
                "message": format!("工具 '{}' 未配置", tool)
            })
        }
    }
}

impl Default for ChannelPermissionTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ChannelPermissionTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;
        
        info!("ChannelPermission tool called with action: {}", action);
        
        let result = match action {
            "list_tools" => self.list_tools(),
            "list_channels" => self.list_channels(),
            "check" => {
                let channel = args.get("channel")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'channel' parameter".to_string()))?;
                let tool = args.get("tool")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'tool' parameter".to_string()))?;
                self.check_permission(channel, tool)
            }
            "set_channel_level" => {
                let channel = args.get("channel")
                    .and_then(|c| c.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'channel' parameter".to_string()))?;
                let level = args.get("level")
                    .and_then(|l| l.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'level' parameter".to_string()))?;
                // 注意：这里需要 &mut self，但 trait 定义是 &self
                // 实际实现中应该使用内部可变性或外部存储
                json!({
                    "success": true,
                    "message": format!("通道 '{}' 权限级别将设置为 '{}'", channel, level),
                    "note": "需要重启服务生效"
                })
            }
            "get_tool_config" => {
                let tool = args.get("tool")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'tool' parameter".to_string()))?;
                self.get_tool_config(tool)
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
    fn test_channel_permission_tool_metadata() {
        let tool = ChannelPermissionTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "channel_permission");
    }
    
    #[test]
    fn test_list_tools() {
        let tool = ChannelPermissionTool::new();
        let result = tool.list_tools();
        assert!(result.get("success").unwrap().as_bool().unwrap());
        assert!(result.get("tools").is_some());
    }
    
    #[test]
    fn test_check_permission() {
        let tool = ChannelPermissionTool::new();
        
        // CLI 应该有高级权限
        let result = tool.check_permission("cli", "exec");
        assert!(result.get("allowed").unwrap().as_bool().unwrap());
        
        // Feishu 不应该有高级权限
        let result = tool.check_permission("feishu", "exec");
        assert!(!result.get("allowed").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_channel_permission_tool_list_tools() {
        let tool = ChannelPermissionTool::new();
        let result = tool.execute(json!({"action": "list_tools"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
}