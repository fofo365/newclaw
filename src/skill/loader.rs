// Skill Loader - v0.5.2
//
// 从文件系统加载 Skill

use super::{SkillConfig, SkillId, SkillType, SkillPermissions};
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Skill 清单（从 SKILL.md 解析）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Skill 名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 版本
    pub version: String,
    /// 作者
    pub author: Option<String>,
    /// 触发关键词
    pub triggers: Vec<String>,
    /// 依赖
    pub dependencies: Vec<String>,
    /// 配置模板
    pub config_template: HashMap<String, String>,
}

/// Skill 加载器
pub struct SkillLoader {
    /// 搜索路径
    search_paths: Vec<PathBuf>,
}

impl SkillLoader {
    /// 创建新的加载器
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
        }
    }
    
    /// 添加搜索路径
    pub fn add_search_path(mut self, path: &Path) -> Self {
        self.search_paths.push(path.to_path_buf());
        self
    }
    
    /// 使用默认路径
    pub fn with_default_paths() -> Self {
        let mut loader = Self::new();

        // 添加 NewClaw 默认路径
        loader.search_paths.push(PathBuf::from("/root/newclaw/skills"));
        loader.search_paths.push(PathBuf::from("/root/newclaw/extensions"));

        loader
    }
    
    /// 发现所有 Skill
    pub fn discover(&self) -> Result<Vec<SkillConfig>> {
        let mut skills = Vec::new();
        
        for search_path in &self.search_paths {
            if !search_path.exists() {
                continue;
            }
            
            for entry in std::fs::read_dir(search_path)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    if let Ok(Some(skill)) = self.load_from_dir(&path) {
                        skills.push(skill);
                    }
                }
            }
        }
        
        Ok(skills)
    }
    
    /// 从目录加载 Skill
    pub fn load_from_dir(&self, dir: &Path) -> Result<Option<SkillConfig>> {
        let skill_md = dir.join("SKILL.md");
        
        if !skill_md.exists() {
            return Ok(None);
        }
        
        let manifest = self.parse_skill_md(&skill_md)?;
        
        let skill_name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        // 检测 Skill 类型
        let skill_type = self.detect_skill_type(dir);
        
        Ok(Some(SkillConfig {
            id: SkillId::new(&skill_name),
            name: manifest.name.clone(),
            description: manifest.description,
            skill_type,
            version: manifest.version,
            path: dir.to_string_lossy().to_string(),
            permissions: SkillPermissions::default(),
            config: HashMap::new(),
            enabled: true,
        }))
    }
    
    /// 解析 SKILL.md 文件
    pub fn parse_skill_md(&self, path: &Path) -> Result<SkillManifest> {
        let content = std::fs::read_to_string(path)?;
        
        let mut manifest = SkillManifest {
            name: String::new(),
            description: String::new(),
            version: "0.0.1".to_string(),
            author: None,
            triggers: Vec::new(),
            dependencies: Vec::new(),
            config_template: HashMap::new(),
        };
        
        // 简单的 Markdown 解析
        let lines: Vec<&str> = content.lines().collect();
        let mut in_section: Option<&str> = None;
        
        for line in &lines {
            let line = line.trim();
            
            // 解析标题作为名称
            if line.starts_with("# ") {
                manifest.name = line[2..].to_string();
                continue;
            }
            
            // 解析元数据
            if line.starts_with("- **") {
                if let Some((key, value)) = self.parse_metadata_line(line) {
                    match key.as_str() {
                        "version" => manifest.version = value,
                        "author" => manifest.author = Some(value),
                        "triggers" => manifest.triggers = value.split(',').map(|s| s.trim().to_string()).collect(),
                        "dependencies" => manifest.dependencies = value.split(',').map(|s| s.trim().to_string()).collect(),
                        _ => {}
                    }
                }
                continue;
            }
            
            // 解析描述（第一个段落）
            if !line.is_empty() && !line.starts_with('#') && manifest.description.is_empty() {
                manifest.description = line.to_string();
            }
        }
        
        if manifest.name.is_empty() {
            return Err(anyhow!("Skill name not found in SKILL.md"));
        }
        
        Ok(manifest)
    }
    
    /// 解析元数据行
    fn parse_metadata_line(&self, line: &str) -> Option<(String, String)> {
        // 格式: - **key**: value
        let re = Regex::new(r"- \*\*([^*]+)\*\*:\s*(.+)").ok()?;
        let caps = re.captures(line)?;
        
        Some((
            caps[1].trim().to_lowercase(),
            caps[2].trim().to_string(),
        ))
    }
    
    /// 检测 Skill 类型
    fn detect_skill_type(&self, dir: &Path) -> SkillType {
        if dir.join("package.json").exists() {
            return SkillType::TypeScript;
        }
        if dir.join("main.py").exists() || dir.join("__init__.py").exists() {
            return SkillType::Python;
        }
        if dir.join("main.sh").exists() || dir.join("run.sh").exists() {
            return SkillType::Shell;
        }
        if dir.join("Cargo.toml").exists() {
            return SkillType::NewClaw;
        }
        SkillType::OpenClaw
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_loader_new() {
        let loader = SkillLoader::new();
        assert!(loader.search_paths.is_empty());
    }

    #[test]
    fn test_skill_loader_with_paths() {
        let loader = SkillLoader::with_default_paths();
        assert!(!loader.search_paths.is_empty());
    }

    #[test]
    fn test_parse_metadata_line() {
        let loader = SkillLoader::new();
        let result = loader.parse_metadata_line("- **version**: 1.0.0");
        assert!(result.is_some());
        let (key, value) = result.unwrap();
        assert_eq!(key, "version");
        assert_eq!(value, "1.0.0");
    }

    #[test]
    fn test_detect_skill_type() {
        let loader = SkillLoader::new();
        // 默认类型
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(matches!(loader.detect_skill_type(temp_dir.path()), SkillType::OpenClaw));
    }
}
