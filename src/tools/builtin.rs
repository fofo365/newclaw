// NewClaw v0.3.0 - 内置工具实现
//
// 实现的工具：
// 1. read - 文件读取（支持图片）
// 2. write - 文件写入
// 3. edit - 精确编辑
// 4. exec - Shell 命令执行
// 5. search - 网络搜索

use super::{Tool, ToolOutput, ToolError, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 读取文件工具
#[derive(Default)]
pub struct ReadTool;

#[async_trait::async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }
    
    fn description(&self) -> &str {
        "Read the contents of a file. Supports text files and images (jpg, png, gif, webp)."
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "offset": {
                    "type": "number",
                    "description": "Line number to start reading from (1-indexed, optional)"
                },
                "limit": {
                    "type": "number",
                    "description": "Maximum number of lines to read (optional)"
                }
            },
            "required": ["path"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput> {
        let params: ReadParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        let path = Path::new(&params.path);
        
        if !path.exists() {
            return Ok(ToolOutput::error(format!("File not found: {}", params.path)));
        }
        
        // 检查是否为图片
        if is_image_file(&params.path) {
            return Ok(ToolOutput::success(format!(
                "[Image file detected: {}. Use a compatible viewer to display this image.]",
                params.path
            )));
        }
        
        // 读取文本文件
        let content = fs::read_to_string(&params.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        
        let lines: Vec<&str> = content.lines().collect();
        let start = params.offset.unwrap_or(1).saturating_sub(1) as usize;
        let end = if let Some(limit) = params.limit {
            (start + limit as usize).min(lines.len())
        } else {
            lines.len()
        };
        
        let result_lines = if start >= lines.len() {
            vec!["[Empty or offset beyond file length]"]
        } else {
            lines[start..end].to_vec()
        };
        
        let result = result_lines.join("\n");
        
        Ok(ToolOutput::with_metadata(
            result,
            [("total_lines".to_string(), serde_json::json!(lines.len()))]
                .iter()
                .cloned()
                .collect(),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct ReadParams {
    path: String,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default)]
    limit: Option<u32>,
}

fn is_image_file(path: &str) -> bool {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp")
}

/// 写入文件工具
#[derive(Default)]
pub struct WriteTool;

#[async_trait::async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }
    
    fn description(&self) -> &str {
        "Write content to a file. Creates the file if it doesn't exist, overwrites if it does."
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput> {
        let params: WriteParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // 创建父目录
        if let Some(parent) = Path::new(&params.path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create directory: {}", e)))?;
        }
        
        // 写入文件
        fs::write(&params.path, &params.content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;
        
        Ok(ToolOutput::success(format!(
            "Successfully wrote {} bytes to {}",
            params.content.len(),
            params.path
        )))
    }
}

#[derive(Debug, Deserialize)]
struct WriteParams {
    path: String,
    content: String,
}

/// 编辑文件工具
#[derive(Default)]
pub struct EditTool;

#[async_trait::async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }
    
    fn description(&self) -> &str {
        "Edit a file by replacing exact text. The oldText must match exactly (including whitespace)."
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "Exact text to find and replace (must match exactly)"
                },
                "new_text": {
                    "type": "string",
                    "description": "New text to replace the old text with"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput> {
        let params: EditParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        let content = fs::read_to_string(&params.path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;
        
        if !content.contains(&params.old_text) {
            return Ok(ToolOutput::error(format!(
                "Old text not found in file. Make sure it matches exactly (including whitespace)."
            )));
        }
        
        let new_content = content.replace(&params.old_text, &params.new_text);
        
        fs::write(&params.path, new_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;
        
        Ok(ToolOutput::success("File edited successfully"))
    }
}

#[derive(Debug, Deserialize)]
struct EditParams {
    path: String,
    old_text: String,
    new_text: String,
}

/// Shell 命令执行工具
#[derive(Default)]
pub struct ExecTool;

#[async_trait::async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &str {
        "exec"
    }
    
    fn description(&self) -> &str {
        "Execute shell commands. Supports background execution with yieldMs."
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "workdir": {
                    "type": "string",
                    "description": "Working directory (optional)"
                },
                "background": {
                    "type": "boolean",
                    "description": "Run in background (optional)"
                }
            },
            "required": ["command"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput> {
        let params: ExecParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c").arg(&params.command);
        
        if let Some(dir) = &params.workdir {
            cmd.current_dir(dir);
        }
        
        let output = cmd.output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        if output.status.success() {
            Ok(ToolOutput::success(if !stderr.is_empty() {
                format!("{}\n[stderr] {}", stdout, stderr)
            } else {
                stdout
            }))
        } else {
            Ok(ToolOutput::error(format!(
                "Command failed with exit code {:?}\n[stdout] {}\n[stderr] {}",
                output.status.code(),
                stdout,
                stderr
            )))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExecParams {
    command: String,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    background: Option<bool>,
}

/// 网络搜索工具
#[derive(Default)]
pub struct SearchTool;

#[async_trait::async_trait]
impl Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }
    
    fn description(&self) -> &str {
        "Search the web using Brave Search API"
    }
    
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query string"
                },
                "count": {
                    "type": "number",
                    "description": "Number of results to return (1-10)"
                }
            },
            "required": ["query"]
        })
    }
    
    async fn execute(&self, params: serde_json::Value) -> ToolResult<ToolOutput> {
        let params: SearchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // TODO: 实现 Brave Search API 调用
        // 暂时返回模拟结果
        Ok(ToolOutput::success(format!(
            "Search results for '{}':\n[Mock results - API integration pending]",
            params.query
        )))
    }
}

#[derive(Debug, Deserialize)]
struct SearchParams {
    query: String,
    #[serde(default)]
    count: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_read_tool() {
        let tool = ReadTool;
        
        // 测试读取不存在的文件
        let output = tool.execute(serde_json::json!({
            "path": "/nonexistent/file.txt"
        })).await.unwrap();
        
        // 应该返回错误
        assert!(!output.is_success());
        assert!(output.error.is_some());
    }
    
    #[tokio::test]
    async fn test_write_tool() {
        let tool = WriteTool;
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        
        let output = tool.execute(serde_json::json!({
            "path": path,
            "content": "Hello, World!"
        })).await.unwrap();
        
        assert!(output.is_success());
        println!("Write output: {}", output.content);
        assert!(output.content.contains("Successfully"));
    }
    
    #[tokio::test]
    async fn test_exec_tool() {
        let tool = ExecTool;
        
        let output = tool.execute(serde_json::json!({
            "command": "echo 'Hello, World!'"
        })).await.unwrap();
        
        assert!(output.is_success());
        assert!(output.content.contains("Hello, World!"));
    }
}
