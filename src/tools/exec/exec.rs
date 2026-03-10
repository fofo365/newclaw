// Shell 命令执行工具

use std::process::Stdio;
use async_trait::async_trait;
use tokio::process::{Command, Child};
use tokio::io::{AsyncReadExt, AsyncBufReadExt, BufReader};

use crate::tools::{Tool, ToolError, ToolResult, ToolMetadata};

/// Shell 执行工具
pub struct ExecTool {
    /// 允许执行的命令白名单（空 = 允许所有）
    allowed_commands: Vec<String>,
    /// 默认超时（秒）
    default_timeout: u64,
}

impl ExecTool {
    /// 创建新的执行工具
    pub fn new() -> Self {
        Self {
            allowed_commands: vec![],
            default_timeout: 30,
        }
    }
    
    /// 设置允许的命令
    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands;
        self
    }
    
    /// 设置默认超时
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.default_timeout = timeout;
        self
    }
    
    /// 验证命令是否允许
    fn validate_command(&self, command: &str) -> ToolResult<()> {
        if self.allowed_commands.is_empty() {
            return Ok(());
        }
        
        // 提取命令名
        let cmd_name = command.split_whitespace().next().unwrap_or("");
        
        if self.allowed_commands.iter().any(|c| c == cmd_name) {
            Ok(())
        } else {
            Err(ToolError::PermissionDenied(format!(
                "命令 '{}' 不在白名单中",
                cmd_name
            )))
        }
    }
}

#[async_trait::async_trait]
impl Tool for ExecTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "exec".to_string(),
            description: "执行 shell 命令".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "要执行的命令"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "超时时间（秒）",
                        "minimum": 1,
                        "maximum": 300
                    },
                    "cwd": {
                        "type": "string",
                        "description": "工作目录"
                    }
                },
                "required": ["command"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        // 解析参数
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 command 参数".to_string()))?;
        
        let timeout = args["timeout"].as_u64().unwrap_or(self.default_timeout);
        let cwd = args["cwd"].as_str();
        
        // 验证命令
        self.validate_command(command)?;
        
        // 执行命令
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            cmd.output()
        )
        .await
        .map_err(|_| ToolError::Timeout(format!("命令执行超时（{} 秒）", timeout)))?
        .map_err(|e| ToolError::ExecutionFailed(format!("执行失败: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok(serde_json::json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
            "success": output.status.success()
        }))
    }
}

impl Default for ExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_exec_echo() {
        let tool = ExecTool::new();
        let result = tool.execute(serde_json::json!({
            "command": "echo 'Hello, world!'"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(result["stdout"].as_str().unwrap().contains("Hello, world!"));
    }
    
    #[tokio::test]
    async fn test_exec_with_timeout() {
        let tool = ExecTool::new();
        let result = tool.execute(serde_json::json!({
            "command": "sleep 0.1 && echo 'done'",
            "timeout": 2
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_exec_timeout_error() {
        let tool = ExecTool::new();
        let result = tool.execute(serde_json::json!({
            "command": "sleep 10",
            "timeout": 1
        })).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_exec_with_whitelist() {
        let tool = ExecTool::new().with_allowed_commands(vec!["echo".to_string()]);
        
        // 允许的命令
        let result = tool.execute(serde_json::json!({
            "command": "echo 'allowed'"
        })).await.unwrap();
        assert!(result["success"].as_bool().unwrap());
        
        // 不允许的命令
        let result = tool.execute(serde_json::json!({
            "command": "ls"
        })).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_exec_exit_code() {
        let tool = ExecTool::new();
        let result = tool.execute(serde_json::json!({
            "command": "exit 42"
        })).await.unwrap();
        
        assert_eq!(result["exit_code"], 42);
        assert!(!result["success"].as_bool().unwrap());
    }
}
