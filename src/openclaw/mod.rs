// OpenClaw Compatibility Layer

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    pub location: SkillLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillLocation {
    OpenClaw(PathBuf),
    NewClaw(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawMemory {
    pub user_info: UserInfo,
    pub projects: Vec<Project>,
    pub preferences: UserPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: Option<String>,
    pub timezone: Option<String>,
    pub primary_channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub status: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub model: Option<String>,
    pub writing_style: Option<String>,
    pub communication_preferences: Vec<String>,
}

pub struct OpenClawMigrator {
    openclaw_workspace: PathBuf,
    newclaw_workspace: PathBuf,
}

impl OpenClawMigrator {
    pub fn new(openclaw_workspace: PathBuf, newclaw_workspace: PathBuf) -> Self {
        Self {
            openclaw_workspace,
            newclaw_workspace,
        }
    }
    
    /// Migrate all OpenClaw data to NewClaw
    pub fn migrate_all(&self) -> Result<MigrationReport> {
        Ok(MigrationReport {
            memory: self.migrate_memory()?,
            skills: self.migrate_skills()?,
            workspace_files: self.migrate_workspace_files()?,
        })
    }
    
    /// Migrate MEMORY.md and memory/ directory
    pub fn migrate_memory(&self) -> Result<MemoryMigrationResult> {
        let memory_md = self.openclaw_workspace.join("MEMORY.md");
        let memory_dir = self.openclaw_workspace.join("memory");
        let target_memory_dir = self.newclaw_workspace.join("memory");
        
        let mut migrated_files = Vec::new();
        
        // Copy MEMORY.md if exists
        if memory_md.exists() {
            let target = self.newclaw_workspace.join("MEMORY.md");
            std::fs::copy(&memory_md, &target)?;
            migrated_files.push(target);
        }
        
        // Copy memory/ directory if exists
        if memory_dir.exists() {
            std::fs::create_dir_all(&target_memory_dir)?;
            
            for entry in std::fs::read_dir(&memory_dir)? {
                let entry = entry?;
                let src = entry.path();
                let dst = target_memory_dir.join(entry.file_name());
                
                if src.is_file() {
                    std::fs::copy(&src, &dst)?;
                    migrated_files.push(dst);
                }
            }
        }
        
        Ok(MemoryMigrationResult {
            files_migrated: migrated_files.len(),
            files: migrated_files,
        })
    }
    
    /// Migrate skills from workspace/skills/ and extensions/
    pub fn migrate_skills(&self) -> Result<SkillMigrationResult> {
        let mut skills = Vec::new();
        
        // Migrate workspace skills
        let workspace_skills = self.openclaw_workspace.join("workspace/skills");
        if workspace_skills.exists() {
            for entry in std::fs::read_dir(&workspace_skills)? {
                let entry = entry?;
                let skill_path = entry.path();
                
                if let Ok(manifest) = self.load_skill_manifest(&skill_path) {
                    skills.push(manifest);
                }
            }
        }
        
        // Migrate extension skills
        let extensions = self.openclaw_workspace.join("extensions");
        if extensions.exists() {
            for entry in std::fs::read_dir(&extensions)? {
                let entry = entry?;
                let ext_path = entry.path();
                
                if ext_path.is_dir() {
                    let skill_md = ext_path.join("skills").join("SKILL.md");
                    if skill_md.exists() {
                        if let Ok(manifest) = self.load_skill_manifest(&ext_path.join("skills")) {
                            skills.push(manifest);
                        }
                    }
                }
            }
        }
        
        Ok(SkillMigrationResult {
            skills_found: skills.len(),
            skills,
        })
    }
    
    fn load_skill_manifest(&self, skill_path: &Path) -> Result<SkillManifest> {
        let skill_md = skill_path.join("SKILL.md");
        
        if !skill_md.exists() {
            return Err(anyhow::anyhow!("SKILL.md not found in {:?}", skill_path));
        }
        
        let content = std::fs::read_to_string(&skill_md)?;
        
        // Parse frontmatter
        let mut name = skill_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let mut description = String::new();
        
        for line in content.lines() {
            if line.starts_with("name:") {
                name = line.trim_start_matches("name:").trim().to_string();
            } else if line.starts_with("description:") {
                description = line.trim_start_matches("description:").trim().to_string();
            }
        }
        
        Ok(SkillManifest {
            name,
            description,
            location: SkillLocation::OpenClaw(skill_path.to_path_buf()),
        })
    }
    
    /// Migrate workspace files (novels, reports, etc.)
    pub fn migrate_workspace_files(&self) -> Result<WorkspaceMigrationResult> {
        let workspace_src = self.openclaw_workspace.join("workspace");
        let workspace_dst = self.newclaw_workspace.join("workspace");
        
        let mut migrated = Vec::new();
        
        if workspace_src.exists() {
            // Create target directory
            std::fs::create_dir_all(&workspace_dst)?;
            
            // Copy specific directories
            for dir_name in &["novels", "files", "creative-thoughts"] {
                let src = workspace_src.join(dir_name);
                let dst = workspace_dst.join(dir_name);
                
                if src.exists() {
                    self.copy_dir_recursive(&src, &dst)?;
                    migrated.push(dst.clone());
                }
            }
            
            // Copy individual files
            for file_name in &["MEMORY.md", "USER.md", "IDENTITY.md", "SOUL.md"] {
                let src = workspace_src.join(file_name);
                let dst = self.newclaw_workspace.join(file_name);
                
                if src.exists() {
                    std::fs::copy(&src, &dst)?;
                    migrated.push(dst);
                }
            }
        }
        
        Ok(WorkspaceMigrationResult {
            directories_migrated: migrated.len(),
            paths: migrated,
        })
    }
    
    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)?;
        
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            
            if ty.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MigrationReport {
    pub memory: MemoryMigrationResult,
    pub skills: SkillMigrationResult,
    pub workspace_files: WorkspaceMigrationResult,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MemoryMigrationResult {
    pub files_migrated: usize,
    #[serde(default)]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SkillMigrationResult {
    pub skills_found: usize,
    #[serde(default)]
    pub skills: Vec<SkillManifest>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceMigrationResult {
    pub directories_migrated: usize,
    #[serde(default)]
    pub paths: Vec<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_migrator_creation() {
        let migrator = OpenClawMigrator::new(
            PathBuf::from("/root/.openclaw"),
            PathBuf::from("/root/newclaw"),
        );
        
        assert_eq!(migrator.openclaw_workspace, PathBuf::from("/root/.openclaw"));
        assert_eq!(migrator.newclaw_workspace, PathBuf::from("/root/newclaw"));
    }
}
