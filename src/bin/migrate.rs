// Migration CLI tool

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "newclaw-migrate")]
#[command(about = "OpenClaw to NewClaw migration tool", long_about = None)]
struct MigrateCli {
    #[command(subcommand)]
    command: MigrateCommands,
}

#[derive(Subcommand)]
enum MigrateCommands {
    /// Migrate all data from OpenClaw
    All {
        #[arg(short, long, default_value = "/root/.openclaw")]
        openclaw_path: String,
        
        #[arg(short, long, default_value = "/root/newclaw")]
        newclaw_path: String,
        
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Migrate only memory (MEMORY.md and memory/)
    Memory {
        #[arg(short, long, default_value = "/root/.openclaw")]
        openclaw_path: String,
        
        #[arg(short, long, default_value = "/root/newclaw")]
        newclaw_path: String,
    },
    /// List discovered skills
    ListSkills {
        #[arg(short, long, default_value = "/root/.openclaw")]
        openclaw_path: String,
    },
}

fn main() -> Result<()> {
    let cli = MigrateCli::parse();
    
    match cli.command {
        MigrateCommands::All { openclaw_path, newclaw_path, dry_run } => {
            println!("🔄 Starting migration from OpenClaw to NewClaw...");
            println!("   Source: {}", openclaw_path);
            println!("   Target: {}", newclaw_path);
            
            if dry_run {
                println!("   ⚠️  DRY RUN MODE - No files will be modified");
            }
            
            let migrator = newclaw::openclaw::OpenClawMigrator::new(
                openclaw_path.into(),
                newclaw_path.into(),
            );
            
            let report = migrator.migrate_all()?;
            
            print_migration_report(&report);
        }
        MigrateCommands::Memory { openclaw_path, newclaw_path } => {
            println!("📝 Migrating memory...");
            
            let migrator = newclaw::openclaw::OpenClawMigrator::new(
                openclaw_path.into(),
                newclaw_path.into(),
            );
            
            let result = migrator.migrate_memory()?;
            
            println!("   ✅ Migrated {} memory files", result.files_migrated);
            for file in &result.files {
                println!("      - {:?}", file);
            }
        }
        MigrateCommands::ListSkills { openclaw_path } => {
            println!("🔍 Discovering skills in {}...", openclaw_path);
            
            let migrator = newclaw::openclaw::OpenClawMigrator::new(
                openclaw_path.into(),
                "/tmp/newclaw".into(),
            );
            
            let result = migrator.migrate_skills()?;
            
            println!("   Found {} skills:", result.skills_found);
            for skill in &result.skills {
                println!("      - {}: {}", skill.name, skill.description);
            }
        }
    }
    
    Ok(())
}

fn print_migration_report(report: &newclaw::openclaw::MigrationReport) {
    println!("\n📊 Migration Report:");
    println!("   Memory files: {}", report.memory.files_migrated);
    println!("   Skills found: {}", report.skills.skills_found);
    println!("   Workspace items: {}", report.workspace_files.directories_migrated);
    
    if !report.skills.skills.is_empty() {
        println!("\n📚 Skills:");
        for skill in &report.skills.skills {
            println!("   - {}: {}", skill.name, skill.description);
        }
    }
}
