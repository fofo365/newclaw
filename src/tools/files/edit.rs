// 文件编辑工具（精确替换）

use std::path::Path;
use async_trait::async_trait;

use crate::tools::{Tool, ToolError, ToolResult, ToolMetadata};

/// 文件编辑工具
pub struct EditTool {
    /// 允许的基础目录（沙箱）
    allowed_dirs: Vec<std::path::PathBuf>,
}

impl EditTool {
    /// 创建新的编辑工具
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
    fn validate_path(&self, path: &Path) -> ToolResult<()> {
        let canonical = path.canonicalize()
            .map_err(|e| ToolError::ExecutionFailed(format!("路径不存在: {}", e)))?;
        
        for dir in &self.allowed_dirs {
            if canonical.starts_with(dir) {
                return Ok(());
            }
        }
        
        Err(ToolError::PermissionDenied(format!(
            "路径 {:?} 不在允许的目录范围内",
            path
        )))
    }
}

#[async_trait::async_trait]
impl Tool for EditTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "edit".to_string(),
            description: "编辑文件（精确文本替换）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径"
                    },
                    "oldText": {
                        "type": "string",
                        "description": "要替换的文本（必须完全匹配）"
                    },
                    "newText": {
                        "type": "string",
                        "description": "替换后的文本"
                    }
                },
                "required": ["path", "oldText", "newText"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        // 解析参数
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 path 参数".to_string()))?;
        
        let old_text = args["oldText"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 oldText 参数".to_string()))?;
        
        let new_text = args["newText"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 newText 参数".to_string()))?;
        
        let path = Path::new(path_str);
        
        // 验证路径
        self.validate_path(path)?;
        
        // 读取文件
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("读取文件失败: {}", e)))?;
        
        // 检查 oldText 是否存在
        if !content.contains(old_text) {
            return Err(ToolError::ExecutionFailed(format!(
                "未找到要替换的文本: {}",
                if old_text.len() > 50 {
                    &old_text[..50]
                } else {
                    old_text
                }
            )));
        }
        
        // 执行替换（只替换第一次出现）
        let new_content = content.replacen(old_text, new_text, 1);
        
        // 写回文件
        tokio::fs::write(path, &new_content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("写入文件失败: {}", e)))?;
        
        Ok(serde_json::json!({
            "success": true,
            "path": path_str,
            "replacements": 1
        }))
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    fn create_test_tool() -> EditTool {
        EditTool::new().with_allowed_dirs(vec![
            std::path::PathBuf::from("/tmp"),
            std::env::current_dir().unwrap_or_default(),
        ])
    }
    
    #[tokio::test]
    async fn test_edit_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap(),
            "oldText": "world",
            "newText": "Rust"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        
        let content = tokio::fs::read_to_string(file.path()).await.unwrap();
        assert!(content.contains("Hello, Rust!"));
    }
    
    #[tokio::test]
    async fn test_edit_multiline() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1\nLine 2\nLine 3").unwrap();
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap(),
            "oldText": "Line 2",
            "newText": "Modified Line 2"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_edit_not_found() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap(),
            "oldText": "nonexistent",
            "newText": "replacement"
        })).await;
        
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_edit_exact_match() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "foo bar foo").unwrap();
        
        let tool = create_test_tool();
        tool.execute(serde_json::json!({
            "path": file.path().to_str().unwrap(),
            "oldText": "foo",
            "newText": "baz"
        })).await.unwrap();
        
        let content = tokio::fs::read_to_string(file.path()).await.unwrap();
        // 只替换第一个
        assert_eq!(content.trim(), "baz bar foo");
    }
}
