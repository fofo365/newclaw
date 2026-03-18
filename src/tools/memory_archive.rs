//! Memory Archive Tool - 记忆归档和恢复工具
//!
//! 实现记忆删除前自动归档，支持恢复
//! 来源: CHANGELOG-v0.7.1.md P1-2 需求

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::path::{Path, PathBuf};
use std::fs;
use chrono::{DateTime, Utc, Duration};
use tracing::{info, warn};

use crate::tools::{Tool, ToolMetadata, ToolError};

/// 归档元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// 归档 ID
    pub id: String,
    /// 原始路径
    pub original_path: String,
    /// 归档时间
    pub archived_at: DateTime<Utc>,
    /// 归档原因
    pub reason: String,
    /// 过期时间
    pub expires_at: Option<DateTime<Utc>>,
    /// 文件大小
    pub size_bytes: u64,
    /// 是否重要
    pub is_important: bool,
}

/// 记忆归档工具
pub struct MemoryArchiveTool {
    metadata: ToolMetadata,
    /// 归档目录
    archive_dir: PathBuf,
    /// 归档保留天数
    retention_days: i64,
}

impl MemoryArchiveTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                name: "memory_archive".to_string(),
                description: r#"记忆归档和恢复工具。防止记忆误删，支持恢复。

功能：
- 删除前自动归档
- 支持恢复已归档的记忆
- 归档保留30天（可配置）
- 重要记忆标记，防止误删

Actions:
- archive: 归档记忆文件
- list: 列出所有归档
- restore: 恢复归档
- delete: 永久删除归档
- cleanup: 清理过期归档
- mark_important: 标记重要记忆

用法示例:
- {"action": "archive", "path": "memory/2026-03-18.md", "reason": "用户删除"} - 归档
- {"action": "list"} - 列出归档
- {"action": "restore", "archive_id": "xxx"} - 恢复
"#.to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["archive", "list", "restore", "delete", "cleanup", "mark_important"],
                            "description": "操作类型"
                        },
                        "path": {
                            "type": "string",
                            "description": "要归档的文件路径"
                        },
                        "archive_id": {
                            "type": "string",
                            "description": "归档ID"
                        },
                        "reason": {
                            "type": "string",
                            "description": "归档原因"
                        },
                        "retention_days": {
                            "type": "integer",
                            "description": "保留天数（默认30）"
                        }
                    },
                    "required": ["action"]
                }),
            },
            archive_dir: PathBuf::from("/var/lib/newclaw/memory.archive"),
            retention_days: 30,
        }
    }
    
    /// 创建归档目录
    fn ensure_archive_dir(&self) -> anyhow::Result<()> {
        if !self.archive_dir.exists() {
            fs::create_dir_all(&self.archive_dir)?;
            info!("Created archive directory: {:?}", self.archive_dir);
        }
        Ok(())
    }
    
    /// 归档文件
    fn archive_file(&self, path: &str, reason: &str) -> JsonValue {
        let source_path = PathBuf::from(path);
        
        // 检查文件是否存在
        if !source_path.exists() {
            return json!({
                "success": false,
                "error": "file_not_found",
                "message": format!("文件不存在: {}", path)
            });
        }
        
        // 确保归档目录存在
        if let Err(e) = self.ensure_archive_dir() {
            return json!({
                "success": false,
                "error": "archive_dir_error",
                "message": format!("无法创建归档目录: {}", e)
            });
        }
        
        // 生成归档 ID
        let archive_id = format!("archive-{}", uuid::Uuid::new_v4());
        let archive_path = self.archive_dir.join(format!("{}.archive", archive_id));
        
        // 读取原文件
        let content = match fs::read(&source_path) {
            Ok(c) => c,
            Err(e) => {
                return json!({
                    "success": false,
                    "error": "read_error",
                    "message": format!("读取文件失败: {}", e)
                });
            }
        };
        
        // 写入归档
        if let Err(e) = fs::write(&archive_path, &content) {
            return json!({
                "success": false,
                "error": "write_error",
                "message": format!("写入归档失败: {}", e)
            });
        }
        
        // 创建元数据
        let metadata = ArchiveMetadata {
            id: archive_id.clone(),
            original_path: path.to_string(),
            archived_at: Utc::now(),
            reason: reason.to_string(),
            expires_at: Some(Utc::now() + Duration::days(self.retention_days)),
            size_bytes: content.len() as u64,
            is_important: false,
        };
        
        // 保存元数据
        let metadata_path = self.archive_dir.join(format!("{}.meta.json", archive_id));
        if let Ok(json) = serde_json::to_string_pretty(&metadata) {
            let _ = fs::write(&metadata_path, json);
        }
        
        info!("Archived file: {} -> {}", path, archive_id);
        
        json!({
            "success": true,
            "archive_id": archive_id,
            "original_path": path,
            "archived_at": metadata.archived_at.to_rfc3339(),
            "expires_at": metadata.expires_at.map(|d| d.to_rfc3339()),
            "size_bytes": metadata.size_bytes,
            "message": format!("文件已归档，归档ID: {}", archive_id)
        })
    }
    
    /// 列出所有归档
    fn list_archives(&self) -> JsonValue {
        if !self.archive_dir.exists() {
            return json!({
                "success": true,
                "archives": [],
                "count": 0
            });
        }
        
        let mut archives = Vec::new();
        
        if let Ok(entries) = fs::read_dir(&self.archive_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<ArchiveMetadata>(&content) {
                            archives.push(json!({
                                "id": meta.id,
                                "original_path": meta.original_path,
                                "archived_at": meta.archived_at.to_rfc3339(),
                                "expires_at": meta.expires_at.map(|d| d.to_rfc3339()),
                                "size_bytes": meta.size_bytes,
                                "is_important": meta.is_important,
                                "reason": meta.reason
                            }));
                        }
                    }
                }
            }
        }
        
        json!({
            "success": true,
            "archives": archives,
            "count": archives.len(),
            "archive_dir": self.archive_dir.to_string_lossy()
        })
    }
    
    /// 恢复归档
    fn restore_archive(&self, archive_id: &str) -> JsonValue {
        let archive_path = self.archive_dir.join(format!("{}.archive", archive_id));
        let metadata_path = self.archive_dir.join(format!("{}.meta.json", archive_id));
        
        // 检查归档是否存在
        if !archive_path.exists() {
            return json!({
                "success": false,
                "error": "archive_not_found",
                "message": format!("归档不存在: {}", archive_id)
            });
        }
        
        // 读取元数据
        let metadata: Option<ArchiveMetadata> = if metadata_path.exists() {
            fs::read_to_string(&metadata_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        };
        
        // 确定恢复路径
        let restore_path = metadata
            .as_ref()
            .map(|m| PathBuf::from(&m.original_path))
            .unwrap_or_else(|| PathBuf::from(format!("restored-{}.md", archive_id)));
        
        // 读取归档内容
        let content = match fs::read(&archive_path) {
            Ok(c) => c,
            Err(e) => {
                return json!({
                    "success": false,
                    "error": "read_error",
                    "message": format!("读取归档失败: {}", e)
                });
            }
        };
        
        // 确保目标目录存在
        if let Some(parent) = restore_path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        
        // 写入恢复文件
        if let Err(e) = fs::write(&restore_path, &content) {
            return json!({
                "success": false,
                "error": "write_error",
                "message": format!("写入恢复文件失败: {}", e)
            });
        }
        
        info!("Restored archive: {} -> {:?}", archive_id, restore_path);
        
        json!({
            "success": true,
            "archive_id": archive_id,
            "restored_to": restore_path.to_string_lossy(),
            "message": format!("归档已恢复到: {}", restore_path.to_string_lossy())
        })
    }
    
    /// 永久删除归档
    fn delete_archive(&self, archive_id: &str) -> JsonValue {
        let archive_path = self.archive_dir.join(format!("{}.archive", archive_id));
        let metadata_path = self.archive_dir.join(format!("{}.meta.json", archive_id));
        
        // 检查是否重要
        if metadata_path.exists() {
            if let Ok(content) = fs::read_to_string(&metadata_path) {
                if let Ok(meta) = serde_json::from_str::<ArchiveMetadata>(&content) {
                    if meta.is_important {
                        return json!({
                            "success": false,
                            "error": "archive_is_important",
                            "message": "此归档已标记为重要，无法删除。请先取消重要标记。"
                        });
                    }
                }
            }
        }
        
        // 删除归档文件
        let mut deleted = false;
        if archive_path.exists() {
            if let Err(e) = fs::remove_file(&archive_path) {
                warn!("Failed to delete archive file: {}", e);
            } else {
                deleted = true;
            }
        }
        
        // 删除元数据
        if metadata_path.exists() {
            let _ = fs::remove_file(&metadata_path);
        }
        
        if deleted {
            info!("Permanently deleted archive: {}", archive_id);
        }
        
        json!({
            "success": deleted,
            "archive_id": archive_id,
            "message": if deleted { "归档已永久删除" } else { "归档不存在或已删除" }
        })
    }
    
    /// 清理过期归档
    fn cleanup_expired(&self) -> JsonValue {
        if !self.archive_dir.exists() {
            return json!({
                "success": true,
                "cleaned": 0,
                "message": "归档目录不存在"
            });
        }
        
        let mut cleaned = 0;
        let now = Utc::now();
        
        if let Ok(entries) = fs::read_dir(&self.archive_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(meta) = serde_json::from_str::<ArchiveMetadata>(&content) {
                            // 检查是否过期且非重要
                            if let Some(expires_at) = meta.expires_at {
                                if expires_at < now && !meta.is_important {
                                    // 删除归档
                                    let archive_path = self.archive_dir.join(format!("{}.archive", meta.id));
                                    let _ = fs::remove_file(&archive_path);
                                    let _ = fs::remove_file(&path);
                                    cleaned += 1;
                                    info!("Cleaned expired archive: {}", meta.id);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        json!({
            "success": true,
            "cleaned": cleaned,
            "message": format!("已清理 {} 个过期归档", cleaned)
        })
    }
    
    /// 标记重要
    fn mark_important(&self, archive_id: &str, important: bool) -> JsonValue {
        let metadata_path = self.archive_dir.join(format!("{}.meta.json", archive_id));
        
        if !metadata_path.exists() {
            return json!({
                "success": false,
                "error": "archive_not_found",
                "message": format!("归档不存在: {}", archive_id)
            });
        }
        
        // 读取并更新元数据
        if let Ok(content) = fs::read_to_string(&metadata_path) {
            if let Ok(mut meta) = serde_json::from_str::<ArchiveMetadata>(&content) {
                meta.is_important = important;
                if let Ok(json) = serde_json::to_string_pretty(&meta) {
                    let _ = fs::write(&metadata_path, json);
                }
            }
        }
        
        json!({
            "success": true,
            "archive_id": archive_id,
            "is_important": important,
            "message": if important { "已标记为重要" } else { "已取消重要标记" }
        })
    }
}

impl Default for MemoryArchiveTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryArchiveTool {
    fn metadata(&self) -> ToolMetadata {
        self.metadata.clone()
    }

    async fn execute(&self, args: JsonValue) -> anyhow::Result<JsonValue> {
        let action = args.get("action")
            .and_then(|a| a.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'action' parameter".to_string()))?;
        
        info!("MemoryArchive tool called with action: {}", action);
        
        let result = match action {
            "archive" => {
                let path = args.get("path")
                    .and_then(|p| p.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'path' parameter".to_string()))?;
                let reason = args.get("reason").and_then(|r| r.as_str()).unwrap_or("用户操作");
                self.archive_file(path, reason)
            }
            "list" => self.list_archives(),
            "restore" => {
                let archive_id = args.get("archive_id")
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'archive_id' parameter".to_string()))?;
                self.restore_archive(archive_id)
            }
            "delete" => {
                let archive_id = args.get("archive_id")
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'archive_id' parameter".to_string()))?;
                self.delete_archive(archive_id)
            }
            "cleanup" => self.cleanup_expired(),
            "mark_important" => {
                let archive_id = args.get("archive_id")
                    .and_then(|a| a.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments("Missing 'archive_id' parameter".to_string()))?;
                let important = args.get("important").and_then(|i| i.as_bool()).unwrap_or(true);
                self.mark_important(archive_id, important)
            }
            _ => {
                return Err(ToolError::InvalidArguments(format!("Unknown action: {}", action)).into());
            }
        };
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_archive_tool_metadata() {
        let tool = MemoryArchiveTool::new();
        let meta = tool.metadata();
        assert_eq!(meta.name, "memory_archive");
    }
    
    #[tokio::test]
    async fn test_memory_archive_tool_list() {
        let tool = MemoryArchiveTool::new();
        let result = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
    
    #[tokio::test]
    async fn test_memory_archive_tool_cleanup() {
        let tool = MemoryArchiveTool::new();
        let result = tool.execute(json!({"action": "cleanup"})).await.unwrap();
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }
}