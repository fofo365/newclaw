// Skill Module - v0.5.2
//
// OpenClaw Skill 系统兼容层

pub mod loader;
pub mod executor;
pub mod registry;

pub use loader::{SkillLoader, SkillManifest};
pub use executor::{SkillExecutor, SkillInput, SkillOutput};
pub use registry::SkillRegistry;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Skill ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(String);

impl SkillId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SkillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Skill 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillType {
    /// OpenClaw 原生 Skill
    OpenClaw,
    /// NewClaw 原生插件
    NewClaw,
    /// TypeScript 插件
    TypeScript,
    /// Shell 脚本
    Shell,
    /// Python 脚本
    Python,
}

impl Default for SkillType {
    fn default() -> Self {
        Self::OpenClaw
    }
}

/// Skill 权限
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPermissions {
    /// 是否允许文件读取
    pub read_files: bool,
    /// 是否允许文件写入
    pub write_files: bool,
    /// 是否允许执行命令
    pub execute_commands: bool,
    /// 是否允许网络访问
    pub network_access: bool,
    /// 允许的环境变量
    pub allowed_env_vars: Vec<String>,
}

impl Default for SkillPermissions {
    fn default() -> Self {
        Self {
            read_files: true,
            write_files: false,
            execute_commands: false,
            network_access: false,
            allowed_env_vars: Vec::new(),
        }
    }
}

impl SkillPermissions {
    /// 完全权限
    pub fn full() -> Self {
        Self {
            read_files: true,
            write_files: true,
            execute_commands: true,
            network_access: true,
            allowed_env_vars: vec!["*".to_string()],
        }
    }
    
    /// 只读权限
    pub fn readonly() -> Self {
        Self {
            read_files: true,
            write_files: false,
            execute_commands: false,
            network_access: false,
            allowed_env_vars: Vec::new(),
        }
    }
    
    /// 无权限
    pub fn none() -> Self {
        Self {
            read_files: false,
            write_files: false,
            execute_commands: false,
            network_access: false,
            allowed_env_vars: Vec::new(),
        }
    }
}

/// Skill 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    /// Skill ID
    pub id: SkillId,
    /// Skill 名称
    pub name: String,
    /// Skill 描述
    pub description: String,
    /// Skill 类型
    pub skill_type: SkillType,
    /// Skill 版本
    pub version: String,
    /// Skill 路径
    pub path: String,
    /// 权限配置
    pub permissions: SkillPermissions,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
    /// 是否启用
    pub enabled: bool,
}

impl Default for SkillConfig {
    fn default() -> Self {
        Self {
            id: SkillId::new("unknown"),
            name: "Unknown".to_string(),
            description: String::new(),
            skill_type: SkillType::default(),
            version: "0.0.1".to_string(),
            path: String::new(),
            permissions: SkillPermissions::default(),
            config: HashMap::new(),
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_id() {
        let id = SkillId::new("test-skill");
        assert_eq!(id.as_str(), "test-skill");
    }

    #[test]
    fn test_skill_permissions_default() {
        let perms = SkillPermissions::default();
        assert!(perms.read_files);
        assert!(!perms.write_files);
    }

    #[test]
    fn test_skill_permissions_full() {
        let perms = SkillPermissions::full();
        assert!(perms.read_files);
        assert!(perms.write_files);
        assert!(perms.execute_commands);
    }

    #[test]
    fn test_skill_config_default() {
        let config = SkillConfig::default();
        assert!(config.enabled);
        assert!(config.config.is_empty());
    }
}
