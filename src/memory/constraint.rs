//! Constraint System - 约束保护系统
//!
//! 实现约束的持久化、作用域管理、冲突检测
//! 来源：ClawNext 设计

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio::sync::RwLock;

/// 约束类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    /// 硬约束，不可违反
    HardConstraint,
    /// 软约束，尽量遵守
    SoftConstraint,
    /// 偏好，参考性质
    Preference,
}

impl Default for ConstraintType {
    fn default() -> Self {
        Self::SoftConstraint
    }
}

/// 约束作用域
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintScope {
    /// 全局生效
    Global,
    /// 当前会话
    Session,
    /// 当前项目
    Project,
    /// 当前任务
    Task,
}

impl Default for ConstraintScope {
    fn default() -> Self {
        Self::Session
    }
}

/// 约束来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintSource {
    /// 用户明确定义
    UserDefined,
    /// 从记忆中提取
    MemoryExtraction,
    /// 系统默认
    System,
}

impl Default for ConstraintSource {
    fn default() -> Self {
        Self::UserDefined
    }
}

/// 约束定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// 唯一标识
    pub id: String,
    /// 约束类型
    #[serde(rename = "type")]
    pub constraint_type: ConstraintType,
    /// 作用域
    pub scope: ConstraintScope,
    /// 约束内容
    pub content: String,
    /// 来源
    pub source: ConstraintSource,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后验证时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_validated: Option<DateTime<Utc>>,
    /// 验证次数
    #[serde(default)]
    pub validation_count: u32,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool { true }

impl Constraint {
    /// 创建新的约束
    pub fn new(content: String, constraint_type: ConstraintType, scope: ConstraintScope) -> Self {
        Self {
            id: format!("constraint-{}", uuid::Uuid::new_v4()),
            constraint_type,
            scope,
            content,
            source: ConstraintSource::UserDefined,
            created_at: Utc::now(),
            last_validated: None,
            validation_count: 0,
            enabled: true,
        }
    }
    
    /// 创建硬约束
    pub fn hard(content: String, scope: ConstraintScope) -> Self {
        Self::new(content, ConstraintType::HardConstraint, scope)
    }
    
    /// 创建软约束
    pub fn soft(content: String, scope: ConstraintScope) -> Self {
        Self::new(content, ConstraintType::SoftConstraint, scope)
    }
    
    /// 创建偏好
    pub fn preference(content: String, scope: ConstraintScope) -> Self {
        Self::new(content, ConstraintType::Preference, scope)
    }
    
    /// 标记已验证
    pub fn mark_validated(&mut self) {
        self.last_validated = Some(Utc::now());
        self.validation_count += 1;
    }
    
    /// 检查是否过期（需要重新验证）
    pub fn needs_validation(&self, max_age_hours: u64) -> bool {
        match self.last_validated {
            None => true,
            Some(last) => {
                let age = Utc::now() - last;
                age.num_hours() as u64 > max_age_hours
            }
        }
    }
}

/// YAML 文件中的约束集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintYaml {
    /// 约束列表
    pub constraints: Vec<Constraint>,
    /// 元数据
    #[serde(default)]
    pub metadata: ConstraintMetadata,
}

/// 约束元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConstraintMetadata {
    /// 版本
    pub version: String,
    /// 最后更新时间
    pub updated_at: Option<DateTime<Utc>>,
    /// 描述
    pub description: Option<String>,
}

/// 约束冲突
#[derive(Debug, Clone)]
pub struct ConstraintConflict {
    /// 已存在的约束
    pub existing: Constraint,
    /// 新约束
    pub new: Constraint,
    /// 解决方案
    pub resolution: ConflictResolution,
}

/// 冲突解决方案
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    /// 保留现有
    KeepExisting,
    /// 使用新约束
    UseNew,
    /// 需要用户输入
    RequireUserInput,
    /// 合并两者
    Merge,
}

/// 约束管理器
pub struct ConstraintManager {
    /// 约束存储
    constraints: HashMap<String, Constraint>,
    /// YAML 文件路径
    yaml_path: PathBuf,
    /// 是否已修改
    dirty: bool,
}

impl ConstraintManager {
    /// 创建新的约束管理器
    pub fn new(yaml_path: PathBuf) -> Self {
        Self {
            constraints: HashMap::new(),
            yaml_path,
            dirty: false,
        }
    }
    
    /// 从目录创建管理器
    pub fn from_dir(dir: &Path, scope: ConstraintScope) -> Self {
        let filename = match scope {
            ConstraintScope::Global => "global.yaml",
            ConstraintScope::Session => "session.yaml",
            ConstraintScope::Project => "project.yaml",
            ConstraintScope::Task => "task.yaml",
        };
        
        Self::new(dir.join(filename))
    }
    
    /// 从 YAML 文件加载
    pub fn load_from_yaml(&mut self) -> Result<()> {
        if !self.yaml_path.exists() {
            return Ok(());
        }
        
        let content = std::fs::read_to_string(&self.yaml_path)?;
        
        if content.trim().is_empty() {
            return Ok(());
        }
        
        let yaml: ConstraintYaml = serde_yaml::from_str(&content)?;
        
        self.constraints.clear();
        for c in yaml.constraints {
            self.constraints.insert(c.id.clone(), c);
        }
        
        self.dirty = false;
        Ok(())
    }
    
    /// 保存到 YAML 文件
    pub fn save_to_yaml(&self) -> Result<()> {
        // 确保目录存在
        if let Some(parent) = self.yaml_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let yaml = ConstraintYaml {
            constraints: self.constraints.values().cloned().collect(),
            metadata: ConstraintMetadata {
                version: "1.0".to_string(),
                updated_at: Some(Utc::now()),
                description: None,
            },
        };
        
        let content = serde_yaml::to_string(&yaml)?;
        std::fs::write(&self.yaml_path, content)?;
        
        Ok(())
    }
    
    /// 添加约束
    pub fn add(&mut self, constraint: Constraint) -> Result<Vec<ConstraintConflict>> {
        // 检测冲突
        let conflicts = self.detect_conflicts(&constraint);
        
        self.constraints.insert(constraint.id.clone(), constraint);
        self.dirty = true;
        
        Ok(conflicts)
    }
    
    /// 移除约束
    pub fn remove(&mut self, id: &str) -> Option<Constraint> {
        let removed = self.constraints.remove(id);
        if removed.is_some() {
            self.dirty = true;
        }
        removed
    }
    
    /// 获取约束
    pub fn get(&self, id: &str) -> Option<&Constraint> {
        self.constraints.get(id)
    }
    
    /// 获取所有约束
    pub fn all(&self) -> Vec<&Constraint> {
        self.constraints.values().collect()
    }
    
    /// 按类型获取约束
    pub fn by_type(&self, constraint_type: &ConstraintType) -> Vec<&Constraint> {
        self.constraints.values()
            .filter(|c| &c.constraint_type == constraint_type && c.enabled)
            .collect()
    }
    
    /// 按作用域获取约束
    pub fn by_scope(&self, scope: &ConstraintScope) -> Vec<&Constraint> {
        self.constraints.values()
            .filter(|c| &c.scope == scope && c.enabled)
            .collect()
    }
    
    /// 获取所有硬约束
    pub fn hard_constraints(&self) -> Vec<&Constraint> {
        self.by_type(&ConstraintType::HardConstraint)
    }
    
    /// 自动提取约束
    pub fn extract_from_message(&mut self, content: &str) -> Option<Constraint> {
        // 使用关键词识别约束语句
        let patterns = [
            ("必须", ConstraintType::HardConstraint),
            ("不要", ConstraintType::HardConstraint),
            ("禁止", ConstraintType::HardConstraint),
            ("严禁", ConstraintType::HardConstraint),
            ("绝不能", ConstraintType::HardConstraint),
            ("尽量", ConstraintType::SoftConstraint),
            ("最好", ConstraintType::SoftConstraint),
            ("应该", ConstraintType::SoftConstraint),
            ("偏好", ConstraintType::Preference),
            ("喜欢", ConstraintType::Preference),
        ];
        
        for (keyword, ctype) in patterns {
            if content.contains(keyword) {
                let constraint = Constraint {
                    id: format!("auto-{}", uuid::Uuid::new_v4()),
                    constraint_type: ctype,
                    scope: ConstraintScope::Session,
                    content: content.to_string(),
                    source: ConstraintSource::MemoryExtraction,
                    created_at: Utc::now(),
                    last_validated: None,
                    validation_count: 0,
                    enabled: true,
                };
                
                self.constraints.insert(constraint.id.clone(), constraint.clone());
                self.dirty = true;
                
                return Some(constraint);
            }
        }
        
        None
    }
    
    /// 检测冲突
    pub fn detect_conflicts(&self, new_constraint: &Constraint) -> Vec<ConstraintConflict> {
        let mut conflicts = Vec::new();
        
        for existing in self.constraints.values() {
            if !existing.enabled {
                continue;
            }
            
            // 简单的冲突检测：检查内容是否相反
            if self.are_conflicting(existing, new_constraint) {
                conflicts.push(ConstraintConflict {
                    existing: existing.clone(),
                    new: new_constraint.clone(),
                    resolution: ConflictResolution::RequireUserInput,
                });
            }
        }
        
        conflicts
    }
    
    /// 判断两个约束是否冲突
    fn are_conflicting(&self, a: &Constraint, b: &Constraint) -> bool {
        // 相同作用域的硬约束
        if a.scope == b.scope 
            && a.constraint_type == ConstraintType::HardConstraint
            && b.constraint_type == ConstraintType::HardConstraint 
        {
            // 检查关键词冲突
            let opposite_pairs = [
                ("必须", "不要"),
                ("必须", "禁止"),
                ("要", "不要"),
                ("可以", "禁止"),
            ];
            
            for (word_a, word_b) in opposite_pairs {
                if (a.content.contains(word_a) && b.content.contains(word_b))
                    || (a.content.contains(word_b) && b.content.contains(word_a))
                {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// 获取约束数量
    pub fn len(&self) -> usize {
        self.constraints.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }
    
    /// 是否已修改
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// 标记已保存
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }
}

/// 约束注入器
pub struct ConstraintInjector;

impl ConstraintInjector {
    /// 将约束注入到消息列表
    pub fn inject(constraints: &[&Constraint], messages: &mut Vec<crate::llm::provider::Message>) {
        if constraints.is_empty() {
            return;
        }
        
        let mut content = String::from("[CRITICAL CONSTRAINTS - MUST FOLLOW]\n\n");
        
        for c in constraints {
            let prefix = match c.constraint_type {
                ConstraintType::HardConstraint => "🔴 REQUIRED",
                ConstraintType::SoftConstraint => "🟡 PREFERRED",
                ConstraintType::Preference => "🟢 PREFERENCE",
            };
            
            content.push_str(&format!(
                "{}: {} (scope: {:?})\n",
                prefix, c.content, c.scope
            ));
        }
        
        content.push_str("\n[END CONSTRAINTS]\n");
        
        // 插入到第一条消息之后
        messages.insert(
            if messages.is_empty() { 0 } else { 1 },
            crate::llm::provider::Message {
                role: crate::llm::provider::MessageRole::System,
                content,
                tool_calls: None,
                tool_call_id: None,
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_constraint_new() {
        let c = Constraint::hard("Do not delete files".to_string(), ConstraintScope::Global);
        assert_eq!(c.constraint_type, ConstraintType::HardConstraint);
        assert_eq!(c.scope, ConstraintScope::Global);
        assert!(c.enabled);
    }
    
    #[test]
    fn test_constraint_soft() {
        let c = Constraint::soft("Prefer concise responses".to_string(), ConstraintScope::Session);
        assert_eq!(c.constraint_type, ConstraintType::SoftConstraint);
    }
    
    #[test]
    fn test_constraint_mark_validated() {
        let mut c = Constraint::default_soft();
        assert_eq!(c.validation_count, 0);
        
        c.mark_validated();
        assert_eq!(c.validation_count, 1);
        assert!(c.last_validated.is_some());
    }
    
    #[test]
    fn test_constraint_needs_validation() {
        let mut c = Constraint::default_soft();
        assert!(c.needs_validation(24)); // 从未验证
        
        c.mark_validated();
        assert!(!c.needs_validation(24)); // 刚验证
    }
    
    #[test]
    fn test_constraint_manager_new() {
        let dir = tempdir().unwrap();
        let manager = ConstraintManager::from_dir(dir.path(), ConstraintScope::Global);
        assert!(manager.is_empty());
    }
    
    #[test]
    fn test_constraint_manager_add() {
        let dir = tempdir().unwrap();
        let mut manager = ConstraintManager::from_dir(dir.path(), ConstraintScope::Global);
        
        let c = Constraint::hard("Test constraint".to_string(), ConstraintScope::Global);
        manager.add(c).unwrap();
        
        assert_eq!(manager.len(), 1);
        assert!(manager.is_dirty());
    }
    
    #[test]
    fn test_constraint_manager_save_load() {
        let dir = tempdir().unwrap();
        let mut manager = ConstraintManager::from_dir(dir.path(), ConstraintScope::Global);
        
        let c = Constraint::hard("Test constraint".to_string(), ConstraintScope::Global);
        manager.add(c).unwrap();
        manager.save_to_yaml().unwrap();
        
        // 重新加载
        let mut manager2 = ConstraintManager::from_dir(dir.path(), ConstraintScope::Global);
        manager2.load_from_yaml().unwrap();
        
        assert_eq!(manager2.len(), 1);
    }
    
    #[test]
    fn test_constraint_manager_extract() {
        let dir = tempdir().unwrap();
        let mut manager = ConstraintManager::from_dir(dir.path(), ConstraintScope::Session);
        
        let extracted = manager.extract_from_message("你必须遵守这个规则");
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().constraint_type, ConstraintType::HardConstraint);
    }
    
    #[test]
    fn test_constraint_manager_conflict_detection() {
        let dir = tempdir().unwrap();
        let mut manager = ConstraintManager::from_dir(dir.path(), ConstraintScope::Global);
        
        let c1 = Constraint::hard("必须删除临时文件".to_string(), ConstraintScope::Global);
        manager.add(c1).unwrap();
        
        let c2 = Constraint::hard("不要删除任何文件".to_string(), ConstraintScope::Global);
        let conflicts = manager.add(c2).unwrap();
        
        assert!(!conflicts.is_empty());
    }
    
    #[test]
    fn test_constraint_by_type() {
        let dir = tempdir().unwrap();
        let mut manager = ConstraintManager::from_dir(dir.path(), ConstraintScope::Global);
        
        manager.add(Constraint::hard("Hard 1".to_string(), ConstraintScope::Global)).unwrap();
        manager.add(Constraint::hard("Hard 2".to_string(), ConstraintScope::Global)).unwrap();
        manager.add(Constraint::soft("Soft 1".to_string(), ConstraintScope::Global)).unwrap();
        
        let hard = manager.by_type(&ConstraintType::HardConstraint);
        assert_eq!(hard.len(), 2);
        
        let soft = manager.by_type(&ConstraintType::SoftConstraint);
        assert_eq!(soft.len(), 1);
    }
}

impl Constraint {
    fn default_soft() -> Self {
        Self::soft("Default constraint".to_string(), ConstraintScope::Session)
    }
}