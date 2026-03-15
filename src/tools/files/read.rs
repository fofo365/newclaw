// 文件读取工具

use std::path::Path;
use async_trait::async_trait;

use crate::tools::{Tool, ToolMetadata};

/// 文件读取工具
pub struct ReadTool {
    /// 允许的基础目录（沙箱）
    allowed_dirs: Vec<std::path::PathBuf>,
}

impl ReadTool {
    /// 创建新的读取工具
    pub fn new() -> Self {
        Self {
            allowed_dirs: vec![std::env::current_dir().unwrap_or_default()],
        }
    }

    /// 设置允许的目录
    pub fn with_allowed_dirs(mut self, dirs: Vec<std::path::PathBuf>) -> Self {
        self.allowed_dirs = dirs;
        self
    }

    /// 验证路径是否在允许范围内
    fn validate_path(&self, path: &Path) -> anyhow::Result<()> {
        let canonical = path.canonicalize()
            .map_err(|e| anyhow::anyhow!("路径不存在: {}", e))?;

        for dir in &self.allowed_dirs {
            if canonical.starts_with(dir) {
                return Ok(());
            }
        }

        Err(anyhow::anyhow!(
            "路径 {:?} 不在允许的目录范围内",
            path
        ))
    }
}

#[async_trait::async_trait]
impl Tool for ReadTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "read".to_string(),
            description: "读取文件内容".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径（相对或绝对）"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "起始行号（1-indexed）",
                        "minimum": 1
                    },
                    "limit": {
                        "type": "integer",
                        "description": "读取行数限制",
                        "minimum": 1
                    }
                },
                "required": ["path"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        // 解析参数
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("缺少 path 参数"))?;

        let offset = args["offset"].as_u64().unwrap_or(1) as usize;
        let limit = args["limit"].as_u64().unwrap_or(2000) as usize;

        let path = Path::new(path_str);

        // 验证路径
        self.validate_path(path)?;

        // 读取文件
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("读取文件失败: {}", e))?;

        // 分行处理
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // 应用 offset 和 limit
        let start = (offset.saturating_sub(1)).min(total_lines);
        let end = (start + limit).min(total_lines);
        let selected_lines = &lines[start..end];

        // 构建结果
        let result = selected_lines.join("\n");

        Ok(serde_json::json!({
            "content": result,
            "total_lines": total_lines,
            "start_line": start + 1,
            "end_line": end,
            "truncated": end < total_lines
        }))
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    fn create_test_tool() -> ReadTool {
        ReadTool::new().with_allowed_dirs(vec![
            std::path::PathBuf::from("/tmp"),
            std::env::current_dir().unwrap_or_default(),
        ])
    }
    
    #[tokio::test]
    async fn test_read_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        writeln!(file, "Line 3").unwrap();
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap()
        })).await.unwrap();
        
        assert!(result["content"].as_str().unwrap().contains("Line 1"));
        assert_eq!(result["total_lines"], 3);
    }
    
    #[tokio::test]
    async fn test_read_with_offset() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        writeln!(file, "Line 3").unwrap();
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap(),
            "offset": 2
        })).await.unwrap();
        
        assert!(!result["content"].as_str().unwrap().contains("Line 1"));
        assert!(result["content"].as_str().unwrap().contains("Line 2"));
    }
    
    #[tokio::test]
    async fn test_read_with_limit() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        writeln!(file, "Line 3").unwrap();
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap(),
            "limit": 1
        })).await.unwrap();
        
        assert!(result["truncated"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": "/nonexistent/file.txt"
        })).await;
        
        assert!(result.is_err());
    }
}
