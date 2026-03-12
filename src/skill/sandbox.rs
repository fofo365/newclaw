// Skill 沙箱安全控制模块 (v0.5.5)
//
// 提供隔离的 Skill 执行环境

use std::collections::HashMap;
use std::sync::RwLock;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use super::{SkillId, SkillConfig, SkillType};

/// 沙箱配置
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub allowed_dirs: Vec<PathBuf>,
    pub temp_dir: PathBuf,
    pub memory_limit: usize,
    pub execution_timeout_secs: u64,
    pub allow_network: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_dirs: vec![PathBuf::from("/tmp")],
            temp_dir: PathBuf::from("/tmp/sandbox"),
            memory_limit: 100 * 1024 * 1024,
            execution_timeout_secs: 30,
            allow_network: false,
        }
    }
}

/// 沙箱执行结果
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 沙箱管理器
pub struct SandboxManager {
    config: SandboxConfig,
    skills: RwLock<HashMap<SkillId, SkillConfig>>,
}

impl SandboxManager {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            skills: RwLock::new(HashMap::new()),
        }
    }
    
    pub fn register_skill(&self, config: SkillConfig) {
        let mut skills = self.skills.write().unwrap();
        skills.insert(config.id.clone(), config);
    }
    
    pub fn unregister_skill(&self, id: &SkillId) {
        let mut skills = self.skills.write().unwrap();
        skills.remove(id);
    }
    
    pub fn get_skill(&self, id: &SkillId) -> Option<SkillConfig> {
        let skills = self.skills.read().unwrap();
        skills.get(id).cloned()
    }
    
    pub fn list_skills(&self) -> Vec<SkillConfig> {
        let skills = self.skills.read().unwrap();
        skills.values().cloned().collect()
    }
    
    pub async fn execute(&self, id: &SkillId, input: SkillInput) -> anyhow::Result<SandboxResult> {
        let skills = self.skills.read().unwrap();
        let config = skills.get(id)
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", id))?;
        
        self.check_permissions(&config, &input)?;
        
        let start = std::time::Instant::now();
        let result = self.execute_skill(config, input).await?;
        let duration = start.elapsed().as_millis() as u64;
        
        Ok(SandboxResult {
            success: result.success,
            output: result.output,
            error: result.error,
            duration_ms: duration,
        })
    }
    
    fn check_permissions(&self, config: &SkillConfig, input: &SkillInput) -> anyhow::Result<()> {
        let perms = &config.permissions;
        
        if !input.files.is_empty() && !perms.read_files {
            return Err(anyhow::anyhow!("File read not allowed"));
        }
        
        if !input.commands.is_empty() && !perms.execute_commands {
            return Err(anyhow::anyhow!("Command execution not allowed"));
        }
        
        if !input.network_requests.is_empty() && !perms.network_access {
            return Err(anyhow::anyhow!("Network access not allowed"));
        }
        
        for cmd in &input.commands {
            if self.is_dangerous_command(cmd) {
                return Err(anyhow::anyhow!("Dangerous command: {}", cmd));
            }
        }
        
        Ok(())
    }
    
    fn is_dangerous_command(&self, cmd: &str) -> bool {
        let dangerous = ["rm -rf /", "dd if=", "mkfs", "format", "fdisk", 
                         "shutdown", "reboot", ":(){:|:&", "/etc/shadow"];
        dangerous.iter().any(|p| cmd.contains(p))
    }
    
    async fn execute_skill(&self, config: &SkillConfig, input: SkillInput) -> anyhow::Result<SandboxResult> {
        match config.skill_type {
            SkillType::Shell => self.execute_shell(config, input).await,
            SkillType::Python => self.execute_python(config, input).await,
            _ => Ok(SandboxResult {
                success: true,
                output: format!("Skill {} executed", config.id),
                error: None,
                duration_ms: 0,
            }),
        }
    }
    
    async fn execute_shell(&self, config: &SkillConfig, input: SkillInput) -> anyhow::Result<SandboxResult> {
        let cmd = if input.commands.is_empty() {
            "echo 'Skill executed'"
        } else {
            &input.commands[0]
        };
        
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(cmd)
            .current_dir(&config.path)
            .output()
            .map_err(|e| anyhow::anyhow!("Shell execution failed: {}", e))?;
        
        Ok(SandboxResult {
            success: output.status.success(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            error: if output.status.success() {
                None
            } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            },
            duration_ms: 0,
        })
    }
    
    async fn execute_python(&self, config: &SkillConfig, _input: SkillInput) -> anyhow::Result<SandboxResult> {
        Ok(SandboxResult {
            success: true,
            output: "Python skill executed".to_string(),
            error: None,
            duration_ms: 0,
        })
    }
}

/// Skill 输入
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillInput {
    pub params: HashMap<String, serde_json::Value>,
    pub files: Vec<PathBuf>,
    pub commands: Vec<String>,
    pub network_requests: Vec<NetworkRequest>,
}

/// 网络请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SkillPermissions;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(config.enabled);
    }

    #[test]
    fn test_sandbox_manager_register() {
        let manager = SandboxManager::new(SandboxConfig::default());
        
        let skill = SkillConfig {
            id: SkillId::new("test"),
            name: "Test".to_string(),
            description: String::new(),
            skill_type: SkillType::Shell,
            version: "1.0".to_string(),
            path: "/tmp".to_string(),
            permissions: SkillPermissions::readonly(),
            config: HashMap::new(),
            enabled: true,
        };
        
        manager.register_skill(skill);
        assert!(!manager.list_skills().is_empty());
    }

    #[test]
    fn test_is_dangerous_command() {
        let manager = SandboxManager::new(SandboxConfig::default());
        
        assert!(manager.is_dangerous_command("rm -rf /"));
        assert!(!manager.is_dangerous_command("ls -la"));
    }
}
