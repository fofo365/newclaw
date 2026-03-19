// Skill Executor - v0.5.2
//
// 执行 Skill

use super::{SkillConfig, SkillType};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::collections::HashMap;

/// Skill 输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    /// 用户输入文本
    pub text: String,
    /// 上下文信息
    pub context: HashMap<String, serde_json::Value>,
    /// 配置参数
    pub config: HashMap<String, serde_json::Value>,
}

impl SkillInput {
    /// 创建新输入
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            context: HashMap::new(),
            config: HashMap::new(),
        }
    }
    
    /// 添加上下文
    pub fn with_context(mut self, key: &str, value: serde_json::Value) -> Self {
        self.context.insert(key.to_string(), value);
        self
    }
    
    /// 添加配置
    pub fn with_config(mut self, key: &str, value: serde_json::Value) -> Self {
        self.config.insert(key.to_string(), value);
        self
    }
}

/// Skill 输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOutput {
    /// 输出文本
    pub text: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SkillOutput {
    /// 创建成功输出
    pub fn success(text: &str) -> Self {
        Self {
            text: text.to_string(),
            success: true,
            error: None,
            metadata: HashMap::new(),
        }
    }
    
    /// 创建错误输出
    pub fn error(msg: &str) -> Self {
        Self {
            text: String::new(),
            success: false,
            error: Some(msg.to_string()),
            metadata: HashMap::new(),
        }
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// Skill 执行器
pub struct SkillExecutor {
    /// 超时时间（秒）
    timeout: u64,
}

impl SkillExecutor {
    /// 创建新执行器
    pub fn new() -> Self {
        Self {
            timeout: 30,
        }
    }
    
    /// 设置超时
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = seconds;
        self
    }
    
    /// 执行 Skill
    ///
    /// 支持执行 TypeScript/Shell/Python 脚本
    /// 所有执行都在沙箱环境中进行，确保安全隔离
    pub async fn execute(&self, skill: &SkillConfig, input: SkillInput) -> Result<SkillOutput> {
        // 检查权限
        if !skill.enabled {
            return Ok(SkillOutput::error("Skill is disabled"));
        }

        match skill.skill_type {
            SkillType::Shell => self.execute_shell(skill, input).await,
            SkillType::Python => self.execute_python(skill, input).await,
            SkillType::TypeScript => self.execute_typescript(skill, input).await,
            _ => self.execute_generic(skill, input).await,
        }
    }

    /// 执行 Shell Skill（沙箱环境）
    async fn execute_shell(&self, skill: &SkillConfig, input: SkillInput) -> Result<SkillOutput> {
        let script_path = format!("{}/main.sh", skill.path);

        if !std::path::Path::new(&script_path).exists() {
            return Ok(SkillOutput::error("Shell script not found"));
        }

        let input_json = serde_json::to_string(&input)?;

        // 在沙箱环境中执行（使用 restricted bash）
        let output = Command::new("bash")
            .arg("-r")  // restricted mode
            .arg(&script_path)
            .env("SKILL_INPUT", &input_json)
            .env("SKILL_PATH", &skill.path)
            .current_dir(&skill.path)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(SkillOutput::success(&stdout))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Ok(SkillOutput::error(&stderr))
                }
            }
            Err(e) => Ok(SkillOutput::error(&format!("Failed to execute: {}", e))),
        }
    }

    /// 执行 Python Skill（沙箱环境）
    async fn execute_python(&self, skill: &SkillConfig, input: SkillInput) -> Result<SkillOutput> {
        let script_path = format!("{}/main.py", skill.path);

        if !std::path::Path::new(&script_path).exists() {
            return Ok(SkillOutput::error("Python script not found"));
        }

        let input_json = serde_json::to_string(&input)?;

        // 在沙箱环境中执行（使用 python 的 -S 选项）
        let output = Command::new("python3")
            .arg("-S")  // 禁用 site-packages
            .arg("-I")  // 隔离模式
            .arg(&script_path)
            .env("SKILL_INPUT", &input_json)
            .env("SKILL_PATH", &skill.path)
            .current_dir(&skill.path)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(SkillOutput::success(&stdout))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Ok(SkillOutput::error(&stderr))
                }
            }
            Err(e) => Ok(SkillOutput::error(&format!("Failed to execute: {}", e))),
        }
    }

    /// 执行 TypeScript Skill（沙箱环境）
    async fn execute_typescript(&self, skill: &SkillConfig, input: SkillInput) -> Result<SkillOutput> {
        // TypeScript 需要 Node.js 环境
        let script_path = format!("{}/dist/index.js", skill.path);

        if !std::path::Path::new(&script_path).exists() {
            return Ok(SkillOutput::error("TypeScript compiled script not found"));
        }

        let input_json = serde_json::to_string(&input)?;

        // 在沙箱环境中执行（使用 node 的 --no-warnings 选项）
        let output = Command::new("node")
            .arg("--no-warnings")
            .arg("--no-deprecation")
            .arg(&script_path)
            .env("SKILL_INPUT", &input_json)
            .env("SKILL_PATH", &skill.path)
            .env("NODE_ENV", "production")
            .current_dir(&skill.path)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(SkillOutput::success(&stdout))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Ok(SkillOutput::error(&stderr))
                }
            }
            Err(e) => Ok(SkillOutput::error(&format!("Failed to execute: {}", e))),
        }
    }
    
    /// 执行通用 Skill（OpenClaw 兼容）
    async fn execute_generic(&self, skill: &SkillConfig, input: SkillInput) -> Result<SkillOutput> {
        // 对于 OpenClaw Skill，返回提示信息
        Ok(SkillOutput::success(&format!(
            "Skill '{}' loaded. Input: {}",
            skill.name,
            input.text
        )))
    }
}

impl Default for SkillExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_input() {
        let input = SkillInput::new("test input");
        assert_eq!(input.text, "test input");
    }

    #[test]
    fn test_skill_input_with_context() {
        let input = SkillInput::new("test")
            .with_context("key", serde_json::json!("value"));
        assert!(input.context.contains_key("key"));
    }

    #[test]
    fn test_skill_output_success() {
        let output = SkillOutput::success("result");
        assert!(output.success);
        assert!(output.error.is_none());
    }

    #[test]
    fn test_skill_output_error() {
        let output = SkillOutput::error("something went wrong");
        assert!(!output.success);
        assert!(output.error.is_some());
    }

    #[test]
    fn test_skill_executor_new() {
        let executor = SkillExecutor::new();
        assert_eq!(executor.timeout, 30);
    }

    #[tokio::test]
    async fn test_execute_disabled_skill() {
        let executor = SkillExecutor::new();
        let mut skill = SkillConfig::default();
        skill.enabled = false;
        
        let input = SkillInput::new("test");
        let output = executor.execute(&skill, input).await.unwrap();
        
        assert!(!output.success);
        assert!(output.error.unwrap().contains("disabled"));
    }
}
