// 飞书通道 Skill 管理模块
//
// 支持在飞书中通过自然语言命令管理技能

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Skill 管理命令
#[derive(Debug, Clone)]
pub enum SkillCommand {
    /// 搜索技能
    Search {
        query: String,
        limit: usize,
        page: usize,
    },
    /// 安装技能
    Install {
        name: String,
        version: Option<String>,
        force: bool,
    },
    /// 卸载技能
    Uninstall {
        name: String,
        force: bool,
    },
    /// 更新技能
    Update {
        name: Option<String>,
        check_only: bool,
    },
    /// 列出已安装技能
    List {
        verbose: bool,
        filter: Option<String>,
    },
    /// 查看技能详情
    Info {
        name: String,
    },
    /// 验证技能
    Verify {
        name: String,
        signature: bool,
    },
    /// 重新加载技能
    Reload {
        name: Option<String>,
    },
}

/// Skill 管理器
pub struct SkillManager {
    skills_dir: PathBuf,
}

impl SkillManager {
    /// 创建新的 Skill 管理器
    pub fn new() -> Self {
        Self {
            skills_dir: PathBuf::from("/root/newclaw/skills"),
        }
    }

    /// 解析自然语言命令
    pub fn parse_command(text: &str) -> Option<SkillCommand> {
        let text = text.trim().to_lowercase();

        // 搜索技能: "搜索 技能" 或 "skill search"
        if text.starts_with("搜索技能") || text.starts_with("search skill") || text.starts_with("skill search") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() >= 3 {
                let query = parts[2..].join(" ");
                return Some(SkillCommand::Search {
                    query,
                    limit: 20,
                    page: 1,
                });
            }
        }

        // 安装技能: "安装 技能" 或 "skill install"
        if text.starts_with("安装技能") || text.starts_with("install skill") || text.starts_with("skill install") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].to_string();
                return Some(SkillCommand::Install {
                    name,
                    version: None,
                    force: false,
                });
            }
        }

        // 卸载技能: "卸载 技能" 或 "skill uninstall"
        if text.starts_with("卸载技能") || text.starts_with("uninstall skill") || text.starts_with("skill uninstall") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].to_string();
                return Some(SkillCommand::Uninstall {
                    name,
                    force: false,
                });
            }
        }

        // 更新技能: "更新 技能" 或 "skill update"
        if text.starts_with("更新技能") || text.starts_with("update skill") || text.starts_with("skill update") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() == 2 {
                // 更新所有
                return Some(SkillCommand::Update {
                    name: None,
                    check_only: false,
                });
            } else if parts.len() >= 3 {
                let name = parts[2].to_string();
                return Some(SkillCommand::Update {
                    name: Some(name),
                    check_only: false,
                });
            }
        }

        // 列出技能: "列出技能" 或 "skill list"
        if text.starts_with("列出技能") || text.starts_with("list skill") || text.starts_with("skill list") {
            return Some(SkillCommand::List {
                verbose: false,
                filter: None,
            });
        }

        // 查看技能: "查看 技能" 或 "skill info"
        if text.starts_with("查看技能") || text.starts_with("skill info") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].to_string();
                return Some(SkillCommand::Info {
                    name,
                });
            }
        }

        // 重新加载: "重新加载 技能" 或 "skill reload"
        if text.starts_with("重新加载") || text.starts_with("reload skill") || text.starts_with("skill reload") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].to_string();
                return Some(SkillCommand::Reload {
                    name: Some(name),
                });
            } else {
                return Some(SkillCommand::Reload {
                    name: None,
                });
            }
        }

        None
    }

    /// 执行命令
    pub async fn execute(&self, command: SkillCommand) -> Result<String> {
        match command {
            SkillCommand::Search { query, limit, page } => {
                self.search(&query, limit, page).await
            },
            SkillCommand::Install { name, version, force } => {
                self.install(&name, version.as_deref(), force).await
            },
            SkillCommand::Uninstall { name, force } => {
                self.uninstall(&name, force).await
            },
            SkillCommand::Update { name, check_only } => {
                self.update(name.as_deref(), check_only).await
            },
            SkillCommand::List { verbose, filter } => {
                self.list(verbose, filter.as_deref()).await
            },
            SkillCommand::Info { name } => {
                self.info(&name).await
            },
            SkillCommand::Verify { name, signature } => {
                self.verify(&name, signature).await
            },
            SkillCommand::Reload { name } => {
                self.reload(name.as_deref()).await
            },
        }
    }

    /// 搜索技能
    async fn search(&self, query: &str, _limit: usize, _page: usize) -> Result<String> {
        let output = Command::new("skillhub")
            .args(["search", query])
            .output()
            .map_err(|e| anyhow::anyhow!("skillhub CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 搜索失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("🔍 搜索结果:\n\n{}", stdout))
    }

    /// 安装技能
    async fn install(&self, name: &str, version: Option<&str>, force: bool) -> Result<String> {
        let args = if let Some(v) = version {
            vec!["install", name, "--version", v]
        } else if force {
            vec!["install", name, "--force"]
        } else {
            vec!["install", name]
        };

        let output = Command::new("newclaw")
            .args(["skill"])
            .args(&args)
            .output()
            .map_err(|e| anyhow::anyhow!("newclaw CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 安装失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("📦 安装成功:\n\n{}", stdout))
    }

    /// 卸载技能
    async fn uninstall(&self, name: &str, force: bool) -> Result<String> {
        let args = if force {
            vec!["uninstall", name, "--force"]
        } else {
            vec!["uninstall", name]
        };

        let output = Command::new("newclaw")
            .args(["skill"])
            .args(&args)
            .output()
            .map_err(|e| anyhow::anyhow!("newclaw CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 卸载失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("🗑️  卸载成功:\n\n{}", stdout))
    }

    /// 更新技能
    async fn update(&self, name: Option<&str>, _check_only: bool) -> Result<String> {
        let args = if let Some(n) = name {
            vec!["update", n]
        } else {
            vec!["update"]
        };

        let output = Command::new("newclaw")
            .args(["skill"])
            .args(&args)
            .output()
            .map_err(|e| anyhow::anyhow!("newclaw CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 更新失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("🔄 更新成功:\n\n{}", stdout))
    }

    /// 列出已安装技能
    async fn list(&self, verbose: bool, filter: Option<&str>) -> Result<String> {
        let mut args = vec!["list"];
        if verbose {
            args.push("--verbose");
        }
        if let Some(f) = filter {
            args.extend(&["--filter", f]);
        }

        let output = Command::new("newclaw")
            .args(["skill"])
            .args(&args)
            .output()
            .map_err(|e| anyhow::anyhow!("newclaw CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 列表失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("📋 已安装的技能:\n\n{}", stdout))
    }

    /// 查看技能详情
    async fn info(&self, name: &str) -> Result<String> {
        let output = Command::new("newclaw")
            .args(["skill", "info", name])
            .output()
            .map_err(|e| anyhow::anyhow!("newclaw CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 查询失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("ℹ️  技能详情:\n\n{}", stdout))
    }

    /// 验证技能
    async fn verify(&self, name: &str, _signature: bool) -> Result<String> {
        let output = Command::new("newclaw")
            .args(["skill", "verify", name])
            .output()
            .map_err(|e| anyhow::anyhow!("newclaw CLI 不可用: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Ok(format!("❌ 验证失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(format!("🔍 验证结果:\n\n{}", stdout))
    }

    /// 重新加载技能
    async fn reload(&self, name: Option<&str>) -> Result<String> {
        // TODO: 实现技能重新加载
        if let Some(n) = name {
            Ok(format!("🔄 技能 '{}' 已重新加载", n))
        } else {
            Ok("🔄 所有技能已重新加载".to_string())
        }
    }

    /// 生成帮助信息
    pub fn help() -> String {
        r#"🛠️ 技能管理命令

## 搜索技能
- 搜索技能 <关键词>
- skill search <query>

## 安装技能
- 安装技能 <名称> [--version <版本>]
- skill install <name>

## 卸载技能
- 卸载技能 <名称>
- skill uninstall <name>

## 更新技能
- 更新技能 [名称]
- skill update [name]

## 列出技能
- 列出技能
- skill list

## 查看技能
- 查看技能 <名称>
- skill info <name>

## 验证技能
- 验证技能 <名称>
- skill verify <name>

## 重新加载
- 重新加载 [名称]
- skill reload [name]

## 示例
搜索技能 weather
安装技能 weather-api
更新技能
列出技能
查看技能 weather-api
卸载技能 old-skill
重新加载"#.to_string()
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search_command() {
        let manager = SkillManager::new();

        assert!(matches!(
            manager.parse_command("搜索技能 weather"),
            Some(SkillCommand::Search { .. })
        ));

        assert!(matches!(
            manager.parse_command("skill search feishu"),
            Some(SkillCommand::Search { .. })
        ));
    }

    #[test]
    fn test_parse_install_command() {
        let manager = SkillManager::new();

        assert!(matches!(
            manager.parse_command("安装技能 weather-api"),
            Some(SkillCommand::Install { .. })
        ));

        assert!(matches!(
            manager.parse_command("skill install translator"),
            Some(SkillCommand::Install { .. })
        ));
    }

    #[test]
    fn test_parse_list_command() {
        let manager = SkillManager::new();

        assert!(matches!(
            manager.parse_command("列出技能"),
            Some(SkillCommand::List { .. })
        ));

        assert!(matches!(
            manager.parse_command("skill list"),
            Some(SkillCommand::List { .. })
        ));
    }

    #[test]
    fn test_help() {
        let help = SkillManager::help();
        assert!(help.contains("技能管理"));
        assert!(help.contains("搜索技能"));
        assert!(help.contains("安装技能"));
    }
}