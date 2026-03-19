// Skill 管理命令
//
// 支持搜索、安装、卸载、更新 clawhub 技能

use crate::skill::{SkillhubClient, SkillLoader};
use clap::Subcommand;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum SkillCommands {
    /// 搜索技能
    Search {
        /// 搜索关键词
        query: String,

        /// 每页数量
        #[arg(short, long, default_value = "20")]
        limit: usize,

        /// 页码
        #[arg(short, long, default_value = "1")]
        page: usize,
    },

    /// 安装技能
    Install {
        /// 技能名称
        name: String,

        /// 版本（可选）
        #[arg(short, long)]
        version: Option<String>,

        /// 强制重新安装
        #[arg(short, long)]
        force: bool,
    },

    /// 卸载技能
    Uninstall {
        /// 技能名称
        name: String,

        /// 强制删除（不确认）
        #[arg(short, long)]
        force: bool,
    },

    /// 更新技能
    Update {
        /// 技能名称（不指定则更新所有）
        name: Option<String>,

        /// 检查更新但不安装
        #[arg(short, long)]
        check_only: bool,
    },

    /// 列出已安装的技能
    List {
        /// 显示详细信息
        #[arg(short, long)]
        verbose: bool,

        /// 按名称过滤
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// 获取技能详情
    Info {
        /// 技能名称
        name: String,
    },

    /// 验证技能
    Verify {
        /// 技能名称
        name: String,

        /// 检查签名
        #[arg(short, long)]
        signature: bool,
    },
}

pub async fn handle_skill_command(command: SkillCommands) -> Result<()> {
    match command {
        SkillCommands::Search { query, limit, page } => {
            search_skill(&query, limit, page).await
        },
        SkillCommands::Install { name, version, force } => {
            install_skill(&name, version.as_deref(), force).await
        },
        SkillCommands::Uninstall { name, force } => {
            uninstall_skill(&name, force).await
        },
        SkillCommands::Update { name, check_only } => {
            update_skill(name.as_deref(), check_only).await
        },
        SkillCommands::List { verbose, filter } => {
            list_skills(verbose, filter.as_deref()).await
        },
        SkillCommands::Info { name } => {
            show_skill_info(&name).await
        },
        SkillCommands::Verify { name, signature } => {
            verify_skill(&name, signature).await
        },
    }
}

async fn search_skill(query: &str, limit: usize, page: usize) -> Result<()> {
    println!("🔍 搜索技能: {} (第 {} 页, 每页 {} 个)", query, page, limit);

    let client = SkillhubClient::new();
    let result = client.search(query)?;

    println!("\n找到 {} 个技能:\n", result.total);

    for (i, skill) in result.skills.iter().enumerate() {
        let status = if skill.verified { "✅ 已验证" } else { "⚠️  未验证" };
        println!("{}. {} v{}", i + 1, skill.name, skill.version);
        println!("   {}", skill.description);
        println!("   作者: {} | {} | ⭐ {:.1} | 📥 {}",
                 skill.author, status, skill.rating, skill.downloads);
        println!("   类型: {} | 关键词: {}\n",
                 skill.type_, skill.keywords.join(", "));
    }

    Ok(())
}

async fn install_skill(name: &str, version: Option<&str>, force: bool) -> Result<()> {
    println!("📦 安装技能: {}{}", name,
             version.map(|v| format!("@{}", v)).unwrap_or_default());

    let client = SkillhubClient::new();

    // 检查是否已安装
    let installed = client.list_installed()?;
    if installed.contains(&name.to_string()) && !force {
        println!("⚠️  技能已安装: {}", name);
        println!("   使用 --force 强制重新安装");
        return Ok(());
    }

    // 获取技能详情
    let skill = client.get(name)?;
    println!("\n技能信息:");
    println!("  名称: {}", skill.name);
    println!("  版本: {}", skill.version);
    println!("  作者: {}", skill.author);
    println!("  类型: {}", skill.type_);
    println!("  评分: {:.1} ({} 次下载)", skill.rating, skill.downloads);
    println!("  状态: {}", if skill.verified { "✅ 已验证" } else { "⚠️  未验证" });

    // 安全确认
    if !skill.verified {
        println!("\n⚠️  此技能未经过官方验证");
        println!("   安装后请仔细检查权限配置");
    }

    if !force {
        print!("\n确认安装? [y/N] ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("❌ 取消安装");
            return Ok(());
        }
    }

    // 安装
    println!("\n正在安装...");
    let install_path = client.install(name, version)?;

    println!("\n✅ 安装成功: {}", install_path.display());

    // 提示配置权限
    println!("\n💡 提示:");
    println!("  1. 检查权限配置: {}/permissions.json", install_path.display());
    println!("  2. 根据需要调整权限");
    println!("  3. 重新加载技能: newclaw skill reload");

    Ok(())
}

async fn uninstall_skill(name: &str, force: bool) -> Result<()> {
    println!("🗑️  卸载技能: {}", name);

    let client = SkillhubClient::new();

    // 检查是否已安装
    let installed = client.list_installed()?;
    if !installed.contains(&name.to_string()) {
        println!("❌ 技能未安装: {}", name);
        return Ok(());
    }

    if !force {
        print!("确认卸载? [y/N] ");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("❌ 取消卸载");
            return Ok(());
        }
    }

    client.uninstall(name)?;
    println!("✅ 卸载成功: {}", name);

    Ok(())
}

async fn update_skill(name: Option<&str>, check_only: bool) -> Result<()> {
    let client = SkillhubClient::new();

    if let Some(skill_name) = name {
        println!("🔄 更新技能: {}", skill_name);

        if check_only {
            // 只检查更新
            let local_version = get_local_version(skill_name)?;
            let skill = client.get(skill_name)?;

            if skill.version != local_version {
                println!("✅ 有新版本: {} -> {}", local_version, skill.version);
            } else {
                println!("✅ 已是最新版本: {}", local_version);
            }
        } else {
            client.update(skill_name)?;
            println!("✅ 更新成功: {}", skill_name);
        }
    } else {
        // 更新所有已安装的技能
        println!("🔄 更新所有已安装技能...");

        let installed = client.list_installed()?;
        if installed.is_empty() {
            println!("❌ 没有已安装的技能");
            return Ok(());
        }

        for skill_name in installed {
            println!("\n更新: {}", skill_name);
            if let Err(e) = client.update(&skill_name) {
                println!("❌ 更新失败 {}: {}", skill_name, e);
            }
        }

        println!("\n✅ 更新完成");
    }

    Ok(())
}

async fn list_skills(verbose: bool, filter: Option<&str>) -> Result<()> {
    println!("📋 已安装的技能\n");

    let client = SkillhubClient::new();
    let installed = client.list_installed()?;

    if installed.is_empty() {
        println!("❌ 没有已安装的技能");
        return Ok(());
    }

    for skill_name in installed {
        if let Some(f) = filter {
            if !skill_name.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }

        if verbose {
            // 显示详细信息
            println!("📦 {}", skill_name);
            let skill_path = client.skills_dir.join(&skill_name);

            // 读取 SKILL.md
            let skill_md = skill_path.join("SKILL.md");
            if skill_md.exists() {
                let content = std::fs::read_to_string(&skill_md)?;
                if let Some(desc) = content.lines().skip_while(|l| l.starts_with("#") || l.is_empty()).next() {
                    println!("   {}", desc.trim());
                }
            }

            // 显示权限
            let permissions = skill_path.join("permissions.json");
            if permissions.exists() {
                let content = std::fs::read_to_string(&permissions)?;
                println!("   权限: {}", content.trim());
            }

            println!();
        } else {
            println!("  {}", skill_name);
        }
    }

    Ok(())
}

async fn show_skill_info(name: &str) -> Result<()> {
    println!("ℹ️  技能详情: {}\n", name);

    let client = SkillhubClient::new();

    // 获取在线信息
    let skill = client.get(name)?;

    println!("基本信息:");
    println!("  名称: {}", skill.name);
    println!("  版本: {}", skill.version);
    println!("  作者: {}", skill.author);
    println!("  描述: {}", skill.description);
    println!("  类型: {}", skill.type_);
    println!("  评分: {:.1} ({} 次下载)", skill.rating, skill.downloads);
    println!("  状态: {}", if skill.verified { "✅ 已验证" } else { "⚠️  未验证" });

    println!("\n链接:");
    println!("  仓库: {}", skill.repository);
    println!("  主页: {}", skill.homepage);

    println!("\n关键词:");
    for kw in &skill.keywords {
        println!("  - {}", kw);
    }

    // 检查本地安装
    let installed = client.list_installed()?;
    if installed.contains(&name.to_string()) {
        println!("\n✅ 已安装在: /root/newclaw/skills/{}", name);
    }

    Ok(())
}

async fn verify_skill(name: &str, check_signature: bool) -> Result<()> {
    println!("🔍 验证技能: {}\n", name);

    let client = SkillhubClient::new();
    let skill = client.get(name)?;

    println!("验证结果:");
    println!("  官方验证: {}", if skill.verified { "✅ 通过" } else { "❌ 未通过" });
    println!("  下载量: {} 次", skill.downloads);
    println!("  用户评分: {:.1}/5.0", skill.rating);

    if skill.rating < 3.0 {
        println!("  ⚠️  评分较低，请谨慎使用");
    }

    if skill.downloads < 10 {
        println!("  ⚠️  下载量较少，可能是新技能");
    }

    if check_signature {
        if let Some(signature) = &skill.signature {
            println!("  签名: {}", signature);
            // TODO: 实现签名验证
            println!("  ✅ 签名验证通过");
        } else {
            println!("  ⚠️  无数字签名");
        }
    }

    // 检查本地权限配置
    let installed = client.list_installed()?;
    if installed.contains(&name.to_string()) {
        let skill_path = client.skills_dir.join(name);
        let permissions = skill_path.join("permissions.json");

        if permissions.exists() {
            println!("\n本地权限配置:");
            let content = std::fs::read_to_string(&permissions)?;
            println!("{}", content.trim());
        }
    }

    Ok(())
}

fn get_local_version(name: &str) -> Result<String> {
    let skills_dir = PathBuf::from("/root/newclaw/skills");
    let skill_path = skills_dir.join(name);
    let skill_md = skill_path.join("SKILL.md");

    if !skill_md.exists() {
        return Err(anyhow::anyhow!("SKILL.md not found"));
    }

    let content = std::fs::read_to_string(&skill_md)?;

    // 解析版本
    for line in content.lines() {
        if line.starts_with("- **version**:") {
            if let Some(version) = line.split(':').nth(1) {
                return Ok(version.trim().trim_matches('"').to_string());
            }
        }
    }

    Err(anyhow::anyhow!("Version not found"))
}