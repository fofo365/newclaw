// 记忆系统模块

use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub path: String,
    pub content: String,
    pub source: String,  // "long_term" 或 "daily"
    pub score: Option<f32>,
}

/// 记忆工具
pub struct MemoryTool {
    memory_dir: PathBuf,
    daily_dir: PathBuf,
    openclaw_workspace: PathBuf,
}

impl MemoryTool {
    /// 创建新的记忆工具实例
    pub fn new(memory_dir: PathBuf, openclaw_workspace: PathBuf) -> Self {
        let daily_dir = memory_dir.join("daily");
        Self {
            memory_dir,
            daily_dir,
            openclaw_workspace,
        }
    }

    /// 自动迁移 OpenClaw 记忆数据
    pub async fn auto_migrate(&self) -> Result<()> {
        // 检查是否已迁移
        if self.memory_dir.join("MEMORY.md").exists() {
            info!("记忆系统已初始化，跳过迁移");
            return Ok(());
        }

        info!("检测到 OpenClaw 记忆系统，开始迁移...");

        // 创建目录
        fs::create_dir_all(&self.memory_dir).await?;
        fs::create_dir_all(&self.daily_dir).await?;

        // 复制长期记忆
        let src = self.openclaw_workspace.join("MEMORY.md");
        let dst = self.memory_dir.join("MEMORY.md");
        if src.exists() {
            fs::copy(&src, &dst).await?;
            info!("✅ 复制长期记忆: MEMORY.md");
        }

        // 复制每日日志
        let src_dir = self.openclaw_workspace.join("memory");
        if src_dir.exists() {
            let mut count = 0;
            let mut entries = fs::read_dir(&src_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Some(file_name) = path.file_name() {
                        let dst_path = self.daily_dir.join(file_name);
                        fs::copy(&path, &dst_path).await?;
                        count += 1;
                    }
                }
            }
            info!("✅ 复制每日日志: {} 个文件", count);
        }

        info!("✅ 记忆迁移完成");
        Ok(())
    }

    /// 关键词搜索
    pub async fn keyword_search(&self, query: &str, max_results: usize) -> Result<Vec<MemoryEntry>> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        // 搜索长期记忆
        let memory_path = self.memory_dir.join("MEMORY.md");
        if memory_path.exists() {
            let content = fs::read_to_string(&memory_path).await?;
            if content.to_lowercase().contains(&query_lower) {
                results.push(MemoryEntry {
                    id: "MEMORY.md".to_string(),
                    path: "MEMORY.md".to_string(),
                    content: self.extract_relevant_lines(&content, &query_lower, 5),
                    source: "long_term".to_string(),
                    score: Some(1.0),
                });
            }
        }

        // 搜索每日日志
        if self.daily_dir.exists() {
            let mut entries = fs::read_dir(&self.daily_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    let content = fs::read_to_string(&path).await?;
                    if content.to_lowercase().contains(&query_lower) {
                        let file_name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        results.push(MemoryEntry {
                            id: format!("daily/{}", file_name),
                            path: format!("daily/{}", file_name),
                            content: self.extract_relevant_lines(&content, &query_lower, 5),
                            source: "daily".to_string(),
                            score: Some(0.9),
                        });
                    }
                }

                if results.len() >= max_results {
                    break;
                }
            }
        }

        // 按相关性排序
        results.sort_by(|a, b| {
            b.score.unwrap_or(0.0).partial_cmp(&a.score.unwrap_or(0.0)).unwrap()
        });

        results.truncate(max_results);
        Ok(results)
    }

    /// 提取相关行（包含关键词的上下文）
    fn extract_relevant_lines(&self, content: &str, query: &str, context_lines: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut relevant_blocks = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains(query) {
                // 提取上下文
                let start = i.saturating_sub(context_lines);
                let end = (i + context_lines + 1).min(lines.len());

                let block: Vec<&str> = lines[start..end].to_vec();
                relevant_blocks.push(block.join("\n"));
            }
        }

        // 合并重叠的块
        if relevant_blocks.is_empty() {
            // 如果没有找到精确匹配，返回前 10 行
            lines.iter().take(10).cloned().collect::<Vec<_>>().join("\n")
        } else {
            relevant_blocks.join("\n...\n")
        }
    }

    /// 获取记忆片段
    pub async fn get(&self, path: &str, from: Option<usize>, lines: Option<usize>) -> Result<String> {
        let file_path = if path.starts_with("daily/") {
            self.memory_dir.join(path)
        } else {
            self.memory_dir.join(path)
        };

        let content = fs::read_to_string(&file_path).await?;

        // 支持分页读取
        if let (Some(from), Some(lines)) = (from, lines) {
            let lines_vec: Vec<&str> = content.lines().collect();
            let selected: Vec<&str> = lines_vec.iter()
                .skip(from)
                .take(lines)
                .copied()
                .collect();
            Ok(selected.join("\n"))
        } else {
            Ok(content)
        }
    }

    /// 获取统计信息
    pub async fn stats(&self) -> Result<MemoryStats> {
        let mut total_files = 0;
        let mut total_size = 0;

        // 统计长期记忆
        let memory_path = self.memory_dir.join("MEMORY.md");
        if memory_path.exists() {
            total_files += 1;
            let metadata = fs::metadata(&memory_path).await?;
            total_size += metadata.len();
        }

        // 统计每日日志
        if self.daily_dir.exists() {
            let mut entries = fs::read_dir(&self.daily_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    total_files += 1;
                    let metadata = fs::metadata(&path).await?;
                    total_size += metadata.len();
                }
            }
        }

        Ok(MemoryStats {
            total_files,
            total_size,
            memory_dir: self.memory_dir.to_str().unwrap_or("unknown").to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_files: usize,
    pub total_size: u64,
    pub memory_dir: String,
}

#[async_trait]
impl Tool for MemoryTool {
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "memory".to_string(),
            description: "Mandatory recall step: semantically search MEMORY.md + memory/*.md (and optional session transcripts) before answering questions about prior work, decisions, dates, people, preferences, or todos; returns top snippets with path + lines. If response has disabled=true, memory retrieval is unavailable and should be surfaced to the user.".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["search", "get", "stats", "migrate"],
                        "description": "Action to run: search | get | stats | migrate"
                    },
                    "query": {
                        "type": "string",
                        "description": "Search query (required for search action)"
                    },
                    "path": {
                        "type": "string",
                        "description": "Memory file path (required for get action, e.g., MEMORY.md or daily/2026-03-11.md)"
                    },
                    "from": {
                        "type": "number",
                        "description": "Start line number for get action (0-indexed)"
                    },
                    "lines": {
                        "type": "number",
                        "description": "Number of lines to read for get action"
                    },
                    "max_results": {
                        "type": "number",
                        "description": "Maximum number of results for search (default: 5)",
                        "default": 5
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<Value> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: action"))?;

        match action {
            "search" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;

                let max_results = args.get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;

                let results = self.keyword_search(query, max_results).await?;
                Ok(serde_json::to_value(results)?)
            }

            "get" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: path"))?;

                let from = args.get("from").and_then(|v| v.as_u64()).map(|n| n as usize);
                let lines = args.get("lines").and_then(|v| v.as_u64()).map(|n| n as usize);

                let content = self.get(path, from, lines).await?;
                Ok(serde_json::json!({
                    "path": path,
                    "content": content,
                    "from": from,
                    "lines": lines
                }))
            }

            "stats" => {
                let stats = self.stats().await?;
                Ok(serde_json::to_value(stats)?)
            }

            "migrate" => {
                self.auto_migrate().await?;
                let stats = self.stats().await?;
                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Memory migration completed",
                    "stats": stats
                }))
            }

            _ => Err(anyhow::anyhow!("Unknown action: {}", action))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_tool_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let tool = MemoryTool::new(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf()
        );

        assert_eq!(tool.metadata().name, "memory");
    }

    #[tokio::test]
    async fn test_keyword_search() {
        let temp_dir = TempDir::new().unwrap();
        // memory_dir 直接就是 temp_dir
        let tool = MemoryTool::new(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf()
        );

        // 创建测试记忆文件（直接在 memory_dir 根目录）
        let test_content = "# Test Memory\n\nThis is a test about NewClaw development.\nNewClaw v0.5.0 is coming.";
        fs::write(temp_dir.path().join("MEMORY.md"), test_content).await.unwrap();

        // 测试搜索
        let results = tool.keyword_search("NewClaw", 5).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "MEMORY.md");
    }

    #[tokio::test]
    async fn test_get_memory() {
        let temp_dir = TempDir::new().unwrap();
        let tool = MemoryTool::new(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf()
        );

        // 创建测试记忆文件
        let test_content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        fs::write(temp_dir.path().join("MEMORY.md"), test_content).await.unwrap();

        // 测试读取
        let content = tool.get("MEMORY.md", Some(1), Some(2)).await.unwrap();
        assert_eq!(content, "Line 2\nLine 3");
    }

    #[tokio::test]
    async fn test_stats() {
        let temp_dir = TempDir::new().unwrap();
        let tool = MemoryTool::new(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf()
        );

        // 创建测试记忆文件
        let daily_dir = temp_dir.path().join("daily");
        fs::create_dir_all(&daily_dir).await.unwrap();

        fs::write(temp_dir.path().join("MEMORY.md"), "test content").await.unwrap();
        fs::write(daily_dir.join("2026-03-11.md"), "daily log").await.unwrap();

        let stats = tool.stats().await.unwrap();
        assert_eq!(stats.total_files, 2);
        assert!(stats.total_size > 0);
    }

    #[tokio::test]
    async fn test_extract_relevant_lines() {
        let temp_dir = TempDir::new().unwrap();
        let tool = MemoryTool::new(
            temp_dir.path().to_path_buf(),
            temp_dir.path().to_path_buf()
        );

        let content = "Line 1\nLine 2\nTarget Line\nLine 4\nLine 5";
        let extracted = tool.extract_relevant_lines(content, "target", 1);

        assert!(extracted.contains("Target Line"));
        assert!(extracted.contains("Line 2"));
        assert!(extracted.contains("Line 4"));
    }
}
