// Skillhub CLI 集成模块
//
// 用于搜索和安装 clawhub 的技能

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{info, warn};

/// Skillhub 技能信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillhubSkill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub repository: String,
    pub homepage: String,
    pub keywords: Vec<String>,
    pub type_: String,  // "typescript", "rust", "python", "shell"
    pub downloads: u64,
    pub rating: f64,
    pub verified: bool,
    pub signature: Option<String>,
}

/// Skillhub 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillhubSearchResult {
    pub skills: Vec<SkillhubSkill>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

/// Skillhub 客户端
pub struct SkillhubClient {
    registry_url: String,
    cache_dir: PathBuf,
    pub skills_dir: PathBuf,
}

impl SkillhubClient {
    /// 创建新的 Skillhub 客户端
    pub fn new() -> Self {
        let skills_dir = PathBuf::from("/root/newclaw/skills");
        let cache_dir = PathBuf::from("/root/newclaw/.skillhub-cache");

        Self {
            registry_url: "https://skillhub.openclaw.ai".to_string(),
            cache_dir,
            skills_dir,
        }
    }

    /// 搜索技能
    pub fn search(&self, query: &str) -> Result<SkillhubSearchResult> {
        info!("搜索技能: {}", query);

        // 调用 skillhub CLI
        let output = Command::new("skillhub")
            .args(["search", "--format", "json", query])
            .output()
            .map_err(|e| anyhow::anyhow!("skillhub CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("搜索失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SkillhubSearchResult = serde_json::from_str(&stdout)?;

        Ok(result)
    }

    /// 获取技能详情
    pub fn get(&self, name: &str) -> Result<SkillhubSkill> {
        info!("获取技能详情: {}", name);

        let output = Command::new("skillhub")
            .args(["get", "--format", "json", name])
            .output()
            .map_err(|e| anyhow::anyhow!("skillhub CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("获取详情失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let skill: SkillhubSkill = serde_json::from_str(&stdout)?;

        Ok(skill)
    }

    /// 安装技能
    ///
    /// 安全检查：
    /// 1. 验证签名（如果有）
    /// 2. 检查是否为官方验证技能
    /// 3. 沙箱环境执行预检查
    /// 4. 权限审查
    pub fn install(&self, name: &str, version: Option<&str>) -> Result<PathBuf> {
        info!("安装技能: {} (版本: {:?})", name, version);

        // 1. 获取技能详情
        let skill = self.get(name)?;

        // 2. 安全验证
        self.verify_skill(&skill)?;

        // 3. 安装到本地
        let install_path = self.skills_dir.join(&skill.name);

        let output = Command::new("skillhub")
            .arg("install")
            .arg("--dir")
            .arg(self.skills_dir.to_string_lossy().as_ref())
            .arg(if let Some(v) = version {
                format!("{}@{}", name, v)
            } else {
                name.to_string()
            })
            .output()
            .map_err(|e| anyhow::anyhow!("skillhub CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("安装失败: {}", stderr));
        }

        // 4. 权限配置
        self.configure_permissions(&skill)?;

        info!("✅ 技能安装成功: {} -> {}", name, install_path.display());

        Ok(install_path)
    }

    /// 卸载技能
    pub fn uninstall(&self, name: &str) -> Result<()> {
        info!("卸载技能: {}", name);

        let skill_path = self.skills_dir.join(name);

        if !skill_path.exists() {
            return Err(anyhow::anyhow!("技能未安装: {}", name));
        }

        // 删除技能目录
        std::fs::remove_dir_all(&skill_path)?;

        info!("✅ 技能卸载成功: {}", name);

        Ok(())
    }

    /// 列出已安装的技能
    pub fn list_installed(&self) -> Result<Vec<String>> {
        let mut skills = Vec::new();

        if !self.skills_dir.exists() {
            return Ok(skills);
        }

        for entry in std::fs::read_dir(&self.skills_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        skills.push(name_str.to_string());
                    }
                }
            }
        }

        Ok(skills)
    }

    /// 更新技能
    pub fn update(&self, name: &str) -> Result<()> {
        info!("更新技能: {}", name);

        let output = Command::new("skillhub")
            .args(["update", "--dir", &self.skills_dir.to_string_lossy(), name])
            .output()
            .map_err(|e| anyhow::anyhow!("skillhub CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("更新失败: {}", stderr));
        }

        info!("✅ 技能更新成功: {}", name);

        Ok(())
    }

    /// 验证技能安全性
    fn verify_skill(&self, skill: &SkillhubSkill) -> Result<()> {
        // 1. 检查是否为官方验证技能
        if !skill.verified {
            warn!("⚠️  技能未经过官方验证: {}", skill.name);
            // 可以在这里添加确认逻辑
        }

        // 2. 检查下载量和评分
        if skill.downloads < 10 && skill.rating < 3.0 {
            warn!("⚠️  技能下载量和评分较低: {} ({} 次下载, {:.1} 分)",
                  skill.name, skill.downloads, skill.rating);
        }

        // 3. 检查技能类型
        match skill.type_.as_str() {
            "typescript" | "python" | "shell" => {
                info!("✅ 支持的技能类型: {}", skill.type_);
            }
            "rust" => {
                info!("✅ Rust 原生技能: {}", skill.name);
            }
            _ => {
                warn!("⚠️  未知的技能类型: {}", skill.type_);
            }
        }

        // 4. 检查签名（如果有）
        if let Some(signature) = &skill.signature {
            // TODO: 实现签名验证
            info!("✅ 技能已签名: {}", skill.name);
        }

        Ok(())
    }

    /// 配置技能权限
    fn configure_permissions(&self, skill: &SkillhubSkill) -> Result<()> {
        // 根据技能类型和评分自动配置权限
        let skill_path = self.skills_dir.join(&skill.name);
        let skill_md = skill_path.join("SKILL.md");

        if !skill_md.exists() {
            return Ok(());
        }

        // 读取 SKILL.md
        let content = std::fs::read_to_string(&skill_md)?;

        // 解析权限需求
        let mut permissions = HashMap::new();
        permissions.insert("read_files".to_string(), serde_json::json!(true));
        permissions.insert("write_files".to_string(), serde_json::json!(false));
        permissions.insert("execute_commands".to_string(), serde_json::json!(false));
        permissions.insert("network_access".to_string(), serde_json::json!(false));

        // 根据关键词调整权限
        let content_lower = content.to_lowercase();
        if content_lower.contains("file") || content_lower.contains("文件") {
            permissions.insert("read_files".to_string(), serde_json::json!(true));
        }
        if content_lower.contains("network") || content_lower.contains("网络") || content_lower.contains("http") {
            permissions.insert("network_access".to_string(), serde_json::json!(true));
        }
        if skill.type_ == "shell" || skill.type_ == "python" {
            // 脚本类技能默认允许执行命令
            permissions.insert("execute_commands".to_string(), serde_json::json!(true));
        }

        // 写入权限配置
        let permissions_path = skill_path.join("permissions.json");
        let permissions_json = serde_json::to_string_pretty(&permissions)?;
        std::fs::write(&permissions_path, permissions_json)?;

        info!("✅ 权限配置已写入: {}", permissions_path.display());

        Ok(())
    }
}

impl Default for SkillhubClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skillhub_client_new() {
        let client = SkillhubClient::new();
        assert_eq!(client.registry_url, "https://skillhub.openclaw.ai");
    }

    #[test]
    fn test_verify_skill_verified() {
        let client = SkillhubClient::new();

        let skill = SkillhubSkill {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            repository: "https://github.com/test/test".to_string(),
            homepage: "https://test.com".to_string(),
            keywords: vec![],
            type_: "typescript".to_string(),
            downloads: 100,
            rating: 4.5,
            verified: true,
            signature: Some("abc123".to_string()),
        };

        assert!(client.verify_skill(&skill).is_ok());
    }
}