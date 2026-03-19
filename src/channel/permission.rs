// 通道层权限控制
//
// 权限模型：
// 1. 缺省规则：任意通道成员可以拥有任意工具权限 (宽松模式)
// 2. 可配置规则：限制特定通道/成员/工具的访问
// 3. 动态配置：通过权限工具动态修改

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{ChannelType, ChannelMember, ChannelRole};

/// 权限规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则类型
    pub rule_type: PermissionRuleType,
    /// 目标通道类型 (None = 所有通道)
    pub channel_types: Option<Vec<ChannelType>>,
    /// 目标成员 ID (None = 所有成员)
    pub member_ids: Option<Vec<String>>,
    /// 目标角色 (None = 所有角色)
    pub roles: Option<Vec<ChannelRole>>,
    /// 目标工具 (None = 所有工具)
    pub tool_names: Option<Vec<String>>,
    /// 是否允许
    pub allow: bool,
    /// 优先级 (数字越大优先级越高)
    pub priority: u8,
    /// 描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 权限规则类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionRuleType {
    /// 允许规则
    Allow,
    /// 拒绝规则
    Deny,
}

/// 权限配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// 默认策略: true = 默认允许, false = 默认拒绝
    pub default_allow: bool,
    /// 是否启用权限检查
    pub enabled: bool,
    /// 超级管理员列表 (跳过权限检查)
    pub super_admins: HashSet<String>,
    /// 规则列表
    pub rules: Vec<PermissionRule>,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            // 缺省：任意成员可以拥有任意工具权限
            default_allow: true,
            enabled: true,
            super_admins: HashSet::new(),
            rules: Vec::new(),
        }
    }
}

impl PermissionConfig {
    /// 从文件加载
    pub fn load(path: &str) -> anyhow::Result<Self> {
        if let Ok(content) = std::fs::read_to_string(path) {
            let config: PermissionConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// 保存到文件
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// 通道权限管理器
#[derive(Debug)]
pub struct ChannelPermission {
    /// 配置
    config: Arc<RwLock<PermissionConfig>>,
    /// 配置文件路径
    config_path: String,
    /// 权限缓存
    cache: Arc<RwLock<HashMap<String, bool>>>,
}

impl ChannelPermission {
    /// 创建新的权限管理器
    pub fn new(config_path: &str) -> Self {
        let config = PermissionConfig::load(config_path).unwrap_or_default();
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: config_path.to_string(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取配置
    pub async fn config(&self) -> PermissionConfig {
        self.config.read().await.clone()
    }

    /// 检查权限
    ///
    /// 检查流程：
    /// 1. 检查是否为超级管理员 -> 直接允许
    /// 2. 检查是否有匹配的规则 -> 按规则决定
    /// 3. 返回默认策略
    pub async fn check(&self, member: &ChannelMember, tool_name: &str) -> bool {
        let config = self.config.read().await;

        // 未启用权限检查 -> 使用默认策略
        if !config.enabled {
            return config.default_allow;
        }

        // 检查超级管理员
        if config.super_admins.contains(&member.member_id) {
            return true;
        }

        // 检查角色
        if matches!(member.role, ChannelRole::SuperAdmin) {
            return true;
        }

        // 检查规则 (按优先级排序)
        let mut matched_rules: Vec<&PermissionRule> = config.rules
            .iter()
            .filter(|rule| self.rule_matches(rule, member, tool_name))
            .collect();
        
        matched_rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        if let Some(rule) = matched_rules.first() {
            return rule.allow;
        }

        // 返回默认策略
        config.default_allow
    }

    /// 检查规则是否匹配
    fn rule_matches(&self, rule: &PermissionRule, member: &ChannelMember, tool_name: &str) -> bool {
        // 检查通道类型
        if let Some(channel_types) = &rule.channel_types {
            if !channel_types.contains(&member.channel_type) {
                return false;
            }
        }

        // 检查成员 ID
        if let Some(member_ids) = &rule.member_ids {
            if !member_ids.contains(&member.member_id) {
                return false;
            }
        }

        // 检查角色
        if let Some(roles) = &rule.roles {
            if !roles.contains(&member.role) {
                return false;
            }
        }

        // 检查工具
        if let Some(tool_names) = &rule.tool_names {
            if !tool_names.contains(&tool_name.to_string()) {
                return false;
            }
        }

        true
    }

    /// 添加规则
    pub async fn add_rule(&self, rule: PermissionRule) -> anyhow::Result<()> {
        let rule_name = rule.name.clone();
        let rule_id = rule.id.clone();
        let mut config = self.config.write().await;
        config.rules.push(rule);
        config.save(&self.config_path)?;
        
        // 清除缓存
        self.cache.write().await.clear();
        
        info!("添加权限规则: {} ({})", rule_name, rule_id);
        Ok(())
    }

    /// 删除规则
    pub async fn remove_rule(&self, rule_id: &str) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        config.rules.retain(|r| r.id != rule_id);
        config.save(&self.config_path)?;
        
        // 清除缓存
        self.cache.write().await.clear();
        
        info!("删除权限规则: {}", rule_id);
        Ok(())
    }

    /// 更新规则
    pub async fn update_rule(&self, rule: PermissionRule) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        if let Some(existing) = config.rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule;
        }
        config.save(&self.config_path)?;
        
        // 清除缓存
        self.cache.write().await.clear();
        
        Ok(())
    }

    /// 列出所有规则
    pub async fn list_rules(&self) -> Vec<PermissionRule> {
        self.config.read().await.rules.clone()
    }

    /// 获取规则
    pub async fn get_rule(&self, rule_id: &str) -> Option<PermissionRule> {
        self.config.read().await.rules.iter()
            .find(|r| r.id == rule_id)
            .cloned()
    }

    /// 添加超级管理员
    pub async fn add_super_admin(&self, member_id: &str) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        config.super_admins.insert(member_id.to_string());
        config.save(&self.config_path)?;
        
        info!("添加超级管理员: {}", member_id);
        Ok(())
    }

    /// 移除超级管理员
    pub async fn remove_super_admin(&self, member_id: &str) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        config.super_admins.remove(member_id);
        config.save(&self.config_path)?;
        
        info!("移除超级管理员: {}", member_id);
        Ok(())
    }

    /// 设置默认策略
    pub async fn set_default_policy(&self, allow: bool) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        config.default_allow = allow;
        config.save(&self.config_path)?;
        
        info!("设置默认权限策略: {}", if allow { "允许" } else { "拒绝" });
        Ok(())
    }

    /// 启用/禁用权限检查
    pub async fn set_enabled(&self, enabled: bool) -> anyhow::Result<()> {
        let mut config = self.config.write().await;
        config.enabled = enabled;
        config.save(&self.config_path)?;
        
        info!("权限检查: {}", if enabled { "启用" } else { "禁用" });
        Ok(())
    }

    /// 导出配置
    pub async fn export_config(&self) -> PermissionConfig {
        self.config.read().await.clone()
    }

    /// 导入配置
    pub async fn import_config(&self, config: PermissionConfig) -> anyhow::Result<()> {
        config.save(&self.config_path)?;
        *self.config.write().await = config;
        
        // 清除缓存
        self.cache.write().await.clear();
        
        info!("导入权限配置");
        Ok(())
    }
}

impl Default for ChannelPermission {
    fn default() -> Self {
        Self::new("/var/lib/newclaw/permissions.json")
    }
}

/// 权限规则构建器
pub struct PermissionRuleBuilder {
    rule: PermissionRule,
}

impl PermissionRuleBuilder {
    pub fn new(name: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            rule: PermissionRule {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                rule_type: PermissionRuleType::Allow,
                channel_types: None,
                member_ids: None,
                roles: None,
                tool_names: None,
                allow: true,
                priority: 50,
                description: None,
                created_at: now,
                updated_at: now,
            },
        }
    }

    pub fn deny(mut self) -> Self {
        self.rule.rule_type = PermissionRuleType::Deny;
        self.rule.allow = false;
        self
    }

    pub fn allow(mut self) -> Self {
        self.rule.rule_type = PermissionRuleType::Allow;
        self.rule.allow = true;
        self
    }

    pub fn for_channels(mut self, channel_types: Vec<ChannelType>) -> Self {
        self.rule.channel_types = Some(channel_types);
        self
    }

    pub fn for_members(mut self, member_ids: Vec<String>) -> Self {
        self.rule.member_ids = Some(member_ids);
        self
    }

    pub fn for_roles(mut self, roles: Vec<ChannelRole>) -> Self {
        self.rule.roles = Some(roles);
        self
    }

    pub fn for_tools(mut self, tool_names: Vec<String>) -> Self {
        self.rule.tool_names = Some(tool_names);
        self
    }

    pub fn priority(mut self, priority: u8) -> Self {
        self.rule.priority = priority;
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.rule.description = Some(description.to_string());
        self
    }

    pub fn build(self) -> PermissionRule {
        self.rule
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_permission() {
        let perm = ChannelPermission::default();
        let member = ChannelMember {
            channel_type: ChannelType::Cli,
            member_id: "test_user".to_string(),
            display_name: None,
            role: ChannelRole::User,
        };

        // 默认允许所有
        assert!(perm.check(&member, "any_tool").await);
    }

    #[tokio::test]
    async fn test_add_rule() {
        let perm = ChannelPermission::default();
        
        let rule = PermissionRuleBuilder::new("deny_exec")
            .deny()
            .for_tools(vec!["exec".to_string()])
            .build();

        perm.add_rule(rule).await.unwrap();
        
        let member = ChannelMember {
            channel_type: ChannelType::Cli,
            member_id: "test_user".to_string(),
            display_name: None,
            role: ChannelRole::User,
        };

        // exec 应该被拒绝
        assert!(!perm.check(&member, "exec").await);
        // 其他工具应该允许
        assert!(perm.check(&member, "read").await);
    }

    #[tokio::test]
    async fn test_super_admin() {
        let perm = ChannelPermission::default();
        perm.set_default_policy(false).await.unwrap();
        
        let member = ChannelMember {
            channel_type: ChannelType::Cli,
            member_id: "admin".to_string(),
            display_name: None,
            role: ChannelRole::SuperAdmin,
        };

        // 超级管理员角色应该允许
        assert!(perm.check(&member, "any_tool").await);
    }

    #[test]
    fn test_rule_builder() {
        let rule = PermissionRuleBuilder::new("test_rule")
            .deny()
            .for_channels(vec![ChannelType::Feishu])
            .for_tools(vec!["exec".to_string()])
            .priority(100)
            .build();

        assert_eq!(rule.name, "test_rule");
        assert!(!rule.allow);
        assert_eq!(rule.priority, 100);
    }
}