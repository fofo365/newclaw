// 权限管理工具
//
// 提供权限配置和管理能力

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};

use crate::tools::{Tool, ToolMetadata};
use crate::channel::{ChannelPermission, PermissionRule, PermissionRuleBuilder, ChannelType, ChannelRole};

/// 权限管理工具
pub struct PermissionTool {
    permissions: std::sync::Arc<ChannelPermission>,
}

impl PermissionTool {
    pub fn new(permissions: std::sync::Arc<ChannelPermission>) -> Self {
        Self { permissions }
    }
}

#[async_trait]
impl Tool for PermissionTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "permission".to_string(),
            description: "权限管理工具：配置通道层和工具层的权限控制".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "list", "get", "add_rule", "remove_rule", "update_rule",
                            "add_super_admin", "remove_super_admin", "list_super_admins",
                            "set_default_policy", "get_config", "enable", "disable"
                        ],
                        "description": "操作类型"
                    },
                    "rule_id": {
                        "type": "string",
                        "description": "规则 ID (用于 get/remove/update)"
                    },
                    "rule": {
                        "type": "object",
                        "description": "规则配置 (用于 add_rule/update_rule)"
                    },
                    "member_id": {
                        "type": "string",
                        "description": "成员 ID (用于 add_super_admin/remove_super_admin)"
                    },
                    "allow": {
                        "type": "boolean",
                        "description": "默认策略 (用于 set_default_policy)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 action 参数"))?;

        match action {
            "list" => self.list_rules().await,
            "get" => {
                let rule_id = args.get("rule_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 rule_id 参数"))?;
                self.get_rule(rule_id).await
            }
            "add_rule" => {
                let rule_json = args.get("rule")
                    .ok_or_else(|| anyhow::anyhow!("缺少 rule 参数"))?;
                let rule: PermissionRule = serde_json::from_value(rule_json.clone())?;
                self.add_rule(rule).await
            }
            "remove_rule" => {
                let rule_id = args.get("rule_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 rule_id 参数"))?;
                self.remove_rule(rule_id).await
            }
            "update_rule" => {
                let rule_json = args.get("rule")
                    .ok_or_else(|| anyhow::anyhow!("缺少 rule 参数"))?;
                let rule: PermissionRule = serde_json::from_value(rule_json.clone())?;
                self.update_rule(rule).await
            }
            "add_super_admin" => {
                let member_id = args.get("member_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 member_id 参数"))?;
                self.add_super_admin(member_id).await
            }
            "remove_super_admin" => {
                let member_id = args.get("member_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 member_id 参数"))?;
                self.remove_super_admin(member_id).await
            }
            "list_super_admins" => self.list_super_admins().await,
            "set_default_policy" => {
                let allow = args.get("allow")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| anyhow::anyhow!("缺少 allow 参数"))?;
                self.set_default_policy(allow).await
            }
            "get_config" => self.get_config().await,
            "enable" => self.set_enabled(true).await,
            "disable" => self.set_enabled(false).await,
            _ => Err(anyhow::anyhow!("未知操作: {}", action)),
        }
    }
}

impl PermissionTool {
    async fn list_rules(&self) -> anyhow::Result<JsonValue> {
        let rules = self.permissions.list_rules().await;
        Ok(json!({
            "success": true,
            "rules": rules,
            "count": rules.len()
        }))
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<JsonValue> {
        match self.permissions.get_rule(rule_id).await {
            Some(rule) => Ok(json!({
                "success": true,
                "rule": rule
            })),
            None => Ok(json!({
                "success": false,
                "error": format!("规则不存在: {}", rule_id)
            })),
        }
    }

    async fn add_rule(&self, rule: PermissionRule) -> anyhow::Result<JsonValue> {
        self.permissions.add_rule(rule.clone()).await?;
        Ok(json!({
            "success": true,
            "message": format!("规则已添加: {}", rule.name),
            "rule_id": rule.id
        }))
    }

    async fn remove_rule(&self, rule_id: &str) -> anyhow::Result<JsonValue> {
        self.permissions.remove_rule(rule_id).await?;
        Ok(json!({
            "success": true,
            "message": format!("规则已删除: {}", rule_id)
        }))
    }

    async fn update_rule(&self, rule: PermissionRule) -> anyhow::Result<JsonValue> {
        self.permissions.update_rule(rule.clone()).await?;
        Ok(json!({
            "success": true,
            "message": format!("规则已更新: {}", rule.name)
        }))
    }

    async fn add_super_admin(&self, member_id: &str) -> anyhow::Result<JsonValue> {
        self.permissions.add_super_admin(member_id).await?;
        Ok(json!({
            "success": true,
            "message": format!("已添加超级管理员: {}", member_id)
        }))
    }

    async fn remove_super_admin(&self, member_id: &str) -> anyhow::Result<JsonValue> {
        self.permissions.remove_super_admin(member_id).await?;
        Ok(json!({
            "success": true,
            "message": format!("已移除超级管理员: {}", member_id)
        }))
    }

    async fn list_super_admins(&self) -> anyhow::Result<JsonValue> {
        let config = self.permissions.config().await;
        let admins: Vec<String> = config.super_admins.into_iter().collect();
        Ok(json!({
            "success": true,
            "super_admins": admins
        }))
    }

    async fn set_default_policy(&self, allow: bool) -> anyhow::Result<JsonValue> {
        self.permissions.set_default_policy(allow).await?;
        Ok(json!({
            "success": true,
            "message": format!("默认策略已设置为: {}", if allow { "允许" } else { "拒绝" })
        }))
    }

    async fn get_config(&self) -> anyhow::Result<JsonValue> {
        let config = self.permissions.export_config().await;
        Ok(json!({
            "success": true,
            "config": config
        }))
    }

    async fn set_enabled(&self, enabled: bool) -> anyhow::Result<JsonValue> {
        self.permissions.set_enabled(enabled).await?;
        Ok(json!({
            "success": true,
            "message": format!("权限检查已{}", if enabled { "启用" } else { "禁用" })
        }))
    }
}

/// 通道配置工具
pub struct ChannelConfigTool {
    permissions: std::sync::Arc<ChannelPermission>,
}

impl ChannelConfigTool {
    pub fn new(permissions: std::sync::Arc<ChannelPermission>) -> Self {
        Self { permissions }
    }
}

#[async_trait]
impl Tool for ChannelConfigTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "channel_config".to_string(),
            description: "通道配置工具：查询和配置通道层状态".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "status", "check_permission", "export_config", "import_config"
                        ],
                        "description": "操作类型"
                    },
                    "member": {
                        "type": "object",
                        "description": "成员信息 (用于 check_permission)"
                    },
                    "tool_name": {
                        "type": "string",
                        "description": "工具名称 (用于 check_permission)"
                    },
                    "config": {
                        "type": "object",
                        "description": "配置内容 (用于 import_config)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 action 参数"))?;

        match action {
            "status" => self.get_status().await,
            "check_permission" => {
                let member_json = args.get("member")
                    .ok_or_else(|| anyhow::anyhow!("缺少 member 参数"))?;
                let member: crate::channel::ChannelMember = serde_json::from_value(member_json.clone())?;
                let tool_name = args.get("tool_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("缺少 tool_name 参数"))?;
                self.check_permission(&member, tool_name).await
            }
            "export_config" => self.export_config().await,
            "import_config" => {
                let config_json = args.get("config")
                    .ok_or_else(|| anyhow::anyhow!("缺少 config 参数"))?;
                let config: crate::channel::PermissionConfig = serde_json::from_value(config_json.clone())?;
                self.import_config(config).await
            }
            _ => Err(anyhow::anyhow!("未知操作: {}", action)),
        }
    }
}

impl ChannelConfigTool {
    async fn get_status(&self) -> anyhow::Result<JsonValue> {
        let config = self.permissions.config().await;
        Ok(json!({
            "success": true,
            "enabled": config.enabled,
            "default_allow": config.default_allow,
            "rule_count": config.rules.len(),
            "super_admin_count": config.super_admins.len()
        }))
    }

    async fn check_permission(
        &self,
        member: &crate::channel::ChannelMember,
        tool_name: &str,
    ) -> anyhow::Result<JsonValue> {
        let allowed = self.permissions.check(member, tool_name).await;
        Ok(json!({
            "success": true,
            "allowed": allowed,
            "member_id": member.member_id,
            "tool_name": tool_name
        }))
    }

    async fn export_config(&self) -> anyhow::Result<JsonValue> {
        let config = self.permissions.export_config().await;
        Ok(json!({
            "success": true,
            "config": config
        }))
    }

    async fn import_config(
        &self,
        config: crate::channel::PermissionConfig,
    ) -> anyhow::Result<JsonValue> {
        self.permissions.import_config(config).await?;
        Ok(json!({
            "success": true,
            "message": "配置已导入"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_permission_tool_list() {
        let permissions = Arc::new(ChannelPermission::default());
        let tool = PermissionTool::new(permissions);
        
        let result = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(result["success"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_permission_tool_add_rule() {
        let permissions = Arc::new(ChannelPermission::default());
        let tool = PermissionTool::new(permissions);
        
        let result = tool.execute(json!({
            "action": "add_rule",
            "rule": {
                "id": "test-1",
                "name": "Test Rule",
                "rule_type": "Deny",
                "allow": false,
                "priority": 50,
                "tool_names": ["exec"],
                "created_at": 0,
                "updated_at": 0
            }
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }
}