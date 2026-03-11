// 记忆系统模块

mod vector_index;

use crate::tools::{Tool, ToolMetadata, Value};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{info, warn};
use vector_index::VectorIndex;

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
    vector_index: Arc<RwLock<Option<VectorIndex>>>,
}

impl MemoryTool {
    /// 创建新的记忆工具实例
    pub fn new(memory_dir: PathBuf, openclaw_workspace: PathBuf) -> Self {
        let daily_dir = memory_dir.join("daily");
        Self {
            memory_dir,
            daily_dir,
            openclaw_workspace,
            vector_index: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化向量索引
    pub async fn init_vector_index(&self, dimension: usize) -> Result<()> {
        let mut vector_index = self.vector_index.write().await;
        *vector_index = Some(VectorIndex::new(dimension));
        info!("✅ 向量索引初始化完成 (维度: {})", dimension);
        Ok(())
    }

    /// 语义搜索（基于向量索引）
    pub async fn semantic_search(&self, query: &str, max_results: usize) -> Result<Vec<MemoryEntry>> {
        let vector_index = self.vector_index.read().await;
        
        if let Some(index) = vector_index.as_ref() {
            // TODO: 实现真实的文本嵌入生成
            // 目前使用简单的 TF-IDF 或词袋模型作为占位符
            let query_vector = self.simple_embedding(query);
            
            let results = index.search(&query_vector, max_results).await?;
            
            Ok(results.into_iter().map(|(id, score, content)| {
                MemoryEntry {
                    id: id.clone(),
                    path: id.clone(),
                    content,
                    source: "vector_index".to_string(),
                    score: Some(score),
                }
            }).collect())
        } else {
            warn!("向量索引未初始化，回退到关键词搜索");
            self.keyword_search(query, max_results).await
        }
    }

    /// 简单的文本嵌入（占位符实现）
    fn simple_embedding(&self, text: &str) -> Vec<f32> {
        // TODO: 使用真实的嵌入模型（如 OpenAI Embeddings）
        // 目前使用简单的哈希向量作为占位符
        let dimension = 128;
        let mut vector = vec![0.0; dimension];
        
        for (i, c) in text.chars().enumerate() {
            let idx = (c as usize) % dimension;
            vector[idx] += 1.0;
        }
        
        // 归一化
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }
        
        vector
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

    /// 添加新记忆
    pub async fn add(&self, content: &str, source: &str, tags: Option<Vec<String>>) -> Result<MemoryEntry> {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().format("%Y-%m-%d").to_string();
        
        // 根据来源决定存储位置
        let file_name = if source == "long_term" {
            "MEMORY.md".to_string()
        } else {
            format!("daily/{}.md", timestamp)
        };
        
        let file_path = self.memory_dir.join(&file_name);
        
        // 准备记忆内容
        let memory_content = if let Some(tags) = tags {
            format!("\n## {} (Added: {})\n\nTags: {}\n\n{}\n\n---\n", 
                id, timestamp, tags.join(", "), content)
        } else {
            format!("\n## {} (Added: {})\n\n{}\n\n---\n", 
                id, timestamp, content)
        };
        
        // 追加到文件
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;
        file.write_all(memory_content.as_bytes()).await?;
        
        Ok(MemoryEntry {
            id: id.clone(),
            path: file_name,
            content: content.to_string(),
            source: source.to_string(),
            score: Some(1.0),
        })
    }
    
    /// 更新记忆
    pub async fn update(&self, id: &str, new_content: &str) -> Result<bool> {
        // 搜索包含该 ID 的记忆
        let files = vec![
            self.memory_dir.join("MEMORY.md"),
            self.daily_dir.clone(),
        ];
        
        for base_path in files {
            if base_path.is_dir() {
                let mut entries = fs::read_dir(&base_path).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        if self.update_in_file(&path, id, new_content).await? {
                            return Ok(true);
                        }
                    }
                }
            } else if base_path.exists() {
                if self.update_in_file(&base_path, id, new_content).await? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// 在文件中更新记忆
    async fn update_in_file(&self, file_path: &PathBuf, id: &str, new_content: &str) -> Result<bool> {
        let content = fs::read_to_string(file_path).await?;
        
        // 查找包含该 ID 的部分
        if content.contains(id) {
            // 简单实现：找到 ## {id} 部分，替换内容
            let start_marker = format!("## {}", id);
            if let Some(start) = content.find(&start_marker) {
                let end = content[start..].find("\n---\n")
                    .map(|i| start + i + 5)
                    .unwrap_or(content.len());
                
                let new_section = format!("## {} (Updated: {})\n\n{}\n\n---\n",
                    id, chrono::Utc::now().format("%Y-%m-%d %H:%M"), new_content);
                
                let updated_content = format!("{}{}{}",
                    &content[..start],
                    new_section,
                    &content[end..]);
                
                fs::write(file_path, updated_content).await?;
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// 删除记忆
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let files = vec![
            self.memory_dir.join("MEMORY.md"),
            self.daily_dir.clone(),
        ];
        
        for base_path in files {
            if base_path.is_dir() {
                let mut entries = fs::read_dir(&base_path).await?;
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        if self.delete_from_file(&path, id).await? {
                            return Ok(true);
                        }
                    }
                }
            } else if base_path.exists() {
                if self.delete_from_file(&base_path, id).await? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// 从文件中删除记忆
    async fn delete_from_file(&self, file_path: &PathBuf, id: &str) -> Result<bool> {
        let content = fs::read_to_string(file_path).await?;
        
        if content.contains(id) {
            let start_marker = format!("## {}", id);
            if let Some(start) = content.find(&start_marker) {
                let end = content[start..].find("\n---\n")
                    .map(|i| start + i + 5)
                    .unwrap_or(content.len());
                
                let updated_content = format!("{}{}",
                    &content[..start],
                    &content[end..]);
                
                fs::write(file_path, updated_content).await?;
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// 列出所有记忆
    pub async fn list(&self, source: Option<&str>, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let mut entries = Vec::new();
        let limit = limit.unwrap_or(100);
        
        // 列出长期记忆
        if source.is_none() || source == Some("long_term") {
            let memory_path = self.memory_dir.join("MEMORY.md");
            if memory_path.exists() {
                let content = fs::read_to_string(&memory_path).await?;
                // 简单实现：按 ## 分割
                for section in content.split("\n## ") {
                    if section.trim().is_empty() {
                        continue;
                    }
                    
                    let lines: Vec<&str> = section.lines().collect();
                    let title = lines.first().unwrap_or(&"").to_string();
                    
                    entries.push(MemoryEntry {
                        id: title.split_whitespace().next().unwrap_or("").to_string(),
                        path: "MEMORY.md".to_string(),
                        content: section.to_string(),
                        source: "long_term".to_string(),
                        score: None,
                    });
                    
                    if entries.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        // 列出每日日志
        if (source.is_none() || source == Some("daily")) && entries.len() < limit {
            if self.daily_dir.exists() {
                let mut dir_entries = fs::read_dir(&self.daily_dir).await?;
                while let Some(entry) = dir_entries.next_entry().await? {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        let file_name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        
                        entries.push(MemoryEntry {
                            id: file_name.clone(),
                            path: format!("daily/{}", file_name),
                            content: "".to_string(), // 不加载内容，节省内存
                            source: "daily".to_string(),
                            score: None,
                        });
                        
                        if entries.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(entries)
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
                        "enum": ["search", "get", "stats", "migrate", "add", "update", "delete", "list", "semantic_search"],
                        "description": "Action to run: search | get | stats | migrate | add | update | delete | list | semantic_search"
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
                    },
                    "content": {
                        "type": "string",
                        "description": "Content for add/update actions"
                    },
                    "id": {
                        "type": "string",
                        "description": "Memory ID for update/delete actions"
                    },
                    "source": {
                        "type": "string",
                        "description": "Source type: long_term or daily (for add/list actions)"
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags for add action (optional)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of entries for list action"
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

            "add" => {
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
                let source = args.get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("long_term");
                let tags = args.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());
                
                let entry = self.add(content, source, tags).await?;
                Ok(serde_json::json!({
                    "status": "success",
                    "message": "Memory added successfully",
                    "entry": entry
                }))
            }

            "update" => {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;
                let content = args.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: content"))?;
                
                let updated = self.update(id, content).await?;
                Ok(serde_json::json!({
                    "status": if updated { "success" } else { "not_found" },
                    "message": if updated { "Memory updated successfully" } else { "Memory not found" },
                    "id": id
                }))
            }

            "delete" => {
                let id = args.get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: id"))?;
                
                let deleted = self.delete(id).await?;
                Ok(serde_json::json!({
                    "status": if deleted { "success" } else { "not_found" },
                    "message": if deleted { "Memory deleted successfully" } else { "Memory not found" },
                    "id": id
                }))
            }

            "list" => {
                let source = args.get("source").and_then(|v| v.as_str());
                let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
                
                let entries = self.list(source, limit).await?;
                Ok(serde_json::json!({
                    "status": "success",
                    "entries": entries,
                    "count": entries.len()
                }))
            }

            "semantic_search" => {
                let query = args.get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: query"))?;

                let max_results = args.get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as usize;

                // 确保向量索引已初始化
                {
                    let vector_index = self.vector_index.read().await;
                    if vector_index.is_none() {
                        drop(vector_index);
                        self.init_vector_index(128).await?;
                    }
                }

                let results = self.semantic_search(query, max_results).await?;
                Ok(serde_json::to_value(results)?)
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
