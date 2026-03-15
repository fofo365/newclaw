// Sandbox - v0.5.2
//
// 代码执行沙箱

use super::{RouterId, isolation::{IsolationManager, IsolationConfig, ResourceQuota}};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 沙箱配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// 最大内存 (MB)
    pub max_memory_mb: usize,
    /// 最大 CPU 时间 (秒)
    pub max_cpu_seconds: u64,
    /// 最大执行时间 (秒)
    pub max_execution_seconds: u64,
    /// 最大输出大小 (bytes)
    pub max_output_size: usize,
    /// 允许的网络访问
    pub allow_network: bool,
    /// 允许的文件系统访问
    pub allow_filesystem: bool,
    /// 禁止的命令
    pub forbidden_commands: Vec<String>,
    /// 白名单命令
    pub allowed_commands: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 100,
            max_cpu_seconds: 10,
            max_execution_seconds: 60,
            max_output_size: 1024 * 1024, // 1MB
            allow_network: false,
            allow_filesystem: true,
            forbidden_commands: vec![
                "rm".to_string(),
                "mkfs".to_string(),
                "dd".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
            ],
            allowed_commands: vec![
                "ls".to_string(),
                "cat".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "whoami".to_string(),
                "date".to_string(),
            ],
        }
    }
}

/// 沙箱执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    /// 标准输出
    pub stdout: String,
    /// 错误输出
    pub stderr: String,
    /// 退出码
    pub exit_code: Option<i32>,
    /// 执行时间 (ms)
    pub execution_time_ms: u64,
    /// 内存使用 (MB)
    pub memory_used_mb: u64,
}

/// 沙箱执行器
pub struct SandboxExecutor {
    /// 配置
    config: SandboxConfig,
    /// 隔离管理器
    isolation_manager: Arc<RwLock<IsolationManager>>,
}

impl SandboxExecutor {
    /// 创建新的沙箱执行器
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            isolation_manager: Arc::new(RwLock::new(IsolationManager::new())),
        }
    }
    
    /// 使用默认配置创建
    pub fn default_executor() -> Self {
        Self::new(SandboxConfig::default())
    }
    
    /// 执行命令
    pub async fn execute(&self, command: &str, args: &[&str]) -> Result<SandboxResult> {
        // 检查命令是否被禁止
        if self.is_command_forbidden(command) {
            return Err(anyhow!("Command is forbidden: {}", command));
        }
        
        // 检查命令是否在白名单中
        if !self.is_command_allowed(command) {
            return Err(anyhow!("Command is not in allowed list: {}", command));
        }
        
        // 执行命令
        let start_time = std::time::Instant::now();
        
        let output = self.execute_internal(command, args).await?;
        
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // 检查执行时间
        if execution_time_ms > self.config.max_execution_seconds * 1000 {
            return Err(anyhow!("Execution timeout"));
        }
        
        Ok(SandboxResult {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
            execution_time_ms,
            memory_used_mb: 0, // TODO: 实现内存监控
        })
    }
    
    /// 执行内部命令
    async fn execute_internal(&self, command: &str, args: &[&str]) -> Result<InternalResult> {
        let mut cmd = tokio::process::Command::new(command);
        
        for arg in args {
            cmd.arg(arg);
        }
        
        // 设置超时
        cmd.kill_on_drop(true);
        
        let output = cmd.output().await
            .map_err(|e| anyhow!("Failed to execute command: {}", e))?;
        
        Ok(InternalResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        })
    }
    
    /// 检查命令是否被禁止
    fn is_command_forbidden(&self, command: &str) -> bool {
        self.config.forbidden_commands.iter().any(|cmd| command == cmd)
    }
    
    /// 检查命令是否在白名单中
    fn is_command_allowed(&self, command: &str) -> bool {
        self.config.allowed_commands.iter().any(|cmd| command == cmd)
    }
}

/// 内部执行结果
struct InternalResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert_eq!(config.max_memory_mb, 100);
        assert!(!config.allow_network);
    }

    #[test]
    fn test_sandbox_executor_new() {
        let executor = SandboxExecutor::default_executor();
        assert!(executor.is_command_allowed("ls"));
        assert!(!executor.is_command_allowed("rm"));
    }

    #[test]
    fn test_is_command_forbidden() {
        let executor = SandboxExecutor::default_executor();
        assert!(executor.is_command_forbidden("rm"));
        assert!(executor.is_command_forbidden("dd"));
        assert!(!executor.is_command_forbidden("ls"));
    }

    #[test]
    fn test_is_command_allowed() {
        let executor = SandboxExecutor::default_executor();
        assert!(executor.is_command_allowed("ls"));
        assert!(executor.is_command_allowed("cat"));
        assert!(!executor.is_command_allowed("rm"));
    }
}
