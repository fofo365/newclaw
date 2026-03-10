// 文件写入工具

use std::path::Path;
use async_trait::async_trait;

use crate::tools::{Tool, ToolError, ToolResult, ToolMetadata};

/// 文件写入工具
pub struct WriteTool {
    /// 允许的基础目录（沙箱）
    allowed_dirs: Vec<std::path::PathBuf>,
}

impl WriteTool {
    /// 创建新的写入工具
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
        // 对于不存在的文件，检查父目录
        let check_path = if path.exists() {
            path.canonicalize()
                .map_err(|e| ToolError::ExecutionFailed(format!("路径验证失败: {}", e)))?
        } else {
            let parent = path.parent()
                .ok_or_else(|| ToolError::InvalidArguments("无效路径".to_string()))?;
            
            if parent.exists() {
                parent.canonicalize()
                    .map_err(|e| ToolError::ExecutionFailed(format!("路径验证失败: {}", e)))?
            } else {
                return Err(ToolError::PermissionDenied(
                    "父目录不存在".to_string()
                ));
            }
        };
        
        for dir in &self.allowed_dirs {
            if check_path.starts_with(dir) {
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
impl Tool for WriteTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "write".to_string(),
            description: "写入文件内容（创建或覆盖）".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件路径（相对或绝对）"
                    },
                    "content": {
                        "type": "string",
                        "description": "文件内容"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }
    
    async fn execute(&self, args: serde_json::Value) -> ToolResult<serde_json::Value> {
        // 解析参数
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 path 参数".to_string()))?;
        
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("缺少 content 参数".to_string()))?;
        
        let path = Path::new(path_str);
        
        // 验证路径
        self.validate_path(path)?;
        
        // 创建父目录（如果需要）
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("创建目录失败: {}", e)))?;
            }
        }
        
        // 写入文件
        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("写入文件失败: {}", e)))?;
        
        // 获取文件信息
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("获取文件信息失败: {}", e)))?;
        
        Ok(serde_json::json!({
            "success": true,
            "path": path_str,
            "bytes_written": metadata.len()
        }))
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    fn create_test_tool() -> WriteTool {
        WriteTool::new().with_allowed_dirs(vec![
            std::path::PathBuf::from("/tmp"),
            std::env::current_dir().unwrap_or_default(),
        ])
    }
    
    #[tokio::test]
    async fn test_write_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Hello, world!"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["bytes_written"], 13);
        
        // 验证文件内容
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "Hello, world!");
    }
    
    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let subdir = dir.path().join("subdir");
        tokio::fs::create_dir_all(&subdir).await.unwrap();
        let path = subdir.join("test.txt");
        
        let tool = create_test_tool();
        let result = tool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Nested content"
        })).await.unwrap();
        
        assert!(result["success"].as_bool().unwrap());
        assert!(path.exists());
    }
    
    #[tokio::test]
    async fn test_write_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        
        let tool = create_test_tool();
        
        // 第一次写入
        tool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Original"
        })).await.unwrap();
        
        // 第二次写入（覆盖）
        tool.execute(serde_json::json!({
            "path": path.to_str().unwrap(),
            "content": "Updated"
        })).await.unwrap();
        
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(content, "Updated");
    }
}
