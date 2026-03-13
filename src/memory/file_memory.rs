// File Memory - 文件级记忆持久化
//
// v0.7.0 - 实现 MEMORY.md / SOUL.md / decisions/ 持久化

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};

/// 文件记忆管理器
pub struct FileMemoryManager {
    /// 工作区路径
    workspace_path: PathBuf,
    /// 缓存的记忆文件
    cache: Arc<RwLock<HashMap<String, MemoryFile>>>,
}

/// 记忆文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFile {
    /// 文件路径
    pub path: PathBuf,
    /// 文件内容
    pub content: String,
    /// 最后修改时间
    pub last_modified: DateTime<Utc>,
    /// 解析的节
    pub sections: Vec<MemorySection>,
}

/// 记忆节
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    /// 节标题
    pub title: String,
    /// 节级别
    pub level: u8,
    /// 节内容
    pub content: String,
    /// 子节
    pub children: Vec<MemorySection>,
}

/// 决策记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    /// 决策 ID
    pub id: String,
    /// 决策时间
    pub timestamp: DateTime<Utc>,
    /// 决策标题
    pub title: String,
    /// 决策内容
    pub content: String,
    /// 相关上下文
    pub context: Option<String>,
    /// 影响
    pub impact: Option<String>,
}

impl FileMemoryManager {
    /// 创建新的文件记忆管理器
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            workspace_path,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// 读取 MEMORY.md
    pub async fn read_memory(&self) -> Result<MemoryFile> {
        self.read_file("MEMORY.md").await
    }
    
    /// 读取 SOUL.md
    pub async fn read_soul(&self) -> Result<MemoryFile> {
        self.read_file("SOUL.md").await
    }
    
    /// 读取任意记忆文件
    pub async fn read_file(&self, name: &str) -> Result<MemoryFile> {
        // 检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(name) {
                // 检查文件是否更新
                let path = self.workspace_path.join(name);
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    if let Ok(modified) = metadata.modified() {
                        let modified: DateTime<Utc> = modified.into();
                        if modified <= cached.last_modified {
                            return Ok(cached.clone());
                        }
                    }
                }
            }
        }
        
        // 读取文件
        let path = self.workspace_path.join(name);
        let content = tokio::fs::read_to_string(&path).await
            .with_context(|| format!("Failed to read {}", name))?;
        
        let metadata = tokio::fs::metadata(&path).await
            .with_context(|| format!("Failed to get metadata for {}", name))?;
        
        let last_modified: DateTime<Utc> = metadata.modified()
            .map(|t| t.into())
            .unwrap_or_else(|_| Utc::now());
        
        // 解析 Markdown 节
        let sections = Self::parse_sections(&content);
        
        let memory_file = MemoryFile {
            path,
            content,
            last_modified,
            sections,
        };
        
        // 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(name.to_string(), memory_file.clone());
        }
        
        Ok(memory_file)
    }
    
    /// 写入 MEMORY.md
    pub async fn write_memory(&self, content: &str) -> Result<()> {
        self.write_file("MEMORY.md", content).await
    }
    
    /// 写入任意记忆文件
    pub async fn write_file(&self, name: &str, content: &str) -> Result<()> {
        let path = self.workspace_path.join(name);
        
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .with_context(|| "Failed to create directory")?;
        }
        
        tokio::fs::write(&path, content).await
            .with_context(|| format!("Failed to write {}", name))?;
        
        // 更新缓存
        let sections = Self::parse_sections(content);
        let memory_file = MemoryFile {
            path,
            content: content.to_string(),
            last_modified: Utc::now(),
            sections,
        };
        
        let mut cache = self.cache.write().await;
        cache.insert(name.to_string(), memory_file);
        
        Ok(())
    }
    
    /// 追加内容到文件
    pub async fn append_to_file(&self, name: &str, content: &str) -> Result<()> {
        let existing = self.read_file(name).await.ok();
        let new_content = match existing {
            Some(mut file) => {
                file.content.push_str("\n\n");
                file.content.push_str(content);
                file.content
            }
            None => content.to_string(),
        };
        
        self.write_file(name, &new_content).await
    }
    
    /// 写入决策记录
    pub async fn write_decision(&self, decision: &Decision) -> Result<()> {
        let date = decision.timestamp.format("%Y-%m-%d");
        let decisions_dir = self.workspace_path.join("decisions");
        
        tokio::fs::create_dir_all(&decisions_dir).await
            .with_context(|| "Failed to create decisions directory")?;
        
        let path = decisions_dir.join(format!("{}.md", date));
        
        let entry = format!(
            r#"## 决策: {}

**时间**: {}  
**ID**: {}

{}

---
"#,
            decision.title,
            decision.timestamp.format("%Y-%m-%d %H:%M:%S"),
            decision.id,
            decision.content,
        );
        
        // 检查文件是否存在
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            // 追加到现有文件
            let mut existing = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            existing.push_str("\n\n");
            existing.push_str(&entry);
            tokio::fs::write(&path, existing).await?;
        } else {
            // 创建新文件
            let header = format!("# 决策记录 - {}\n\n", date);
            tokio::fs::write(&path, format!("{}{}", header, entry)).await?;
        }
        
        Ok(())
    }
    
    /// 获取最近的决策
    pub async fn get_recent_decisions(&self, limit: usize) -> Result<Vec<Decision>> {
        let decisions_dir = self.workspace_path.join("decisions");
        
        if !tokio::fs::try_exists(&decisions_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }
        
        let mut decisions = Vec::new();
        let mut entries = tokio::fs::read_dir(&decisions_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                // 简单解析决策
                for section in content.split("## 决策:") {
                    if section.trim().is_empty() {
                        continue;
                    }
                    
                    let lines: Vec<&str> = section.lines().collect();
                    if lines.is_empty() {
                        continue;
                    }
                    
                    let title = lines[0].trim().to_string();
                    let content = lines[1..].join("\n").trim().to_string();
                    
                    decisions.push(Decision {
                        id: uuid::Uuid::new_v4().to_string(),
                        timestamp: Utc::now(),
                        title,
                        content,
                        context: None,
                        impact: None,
                    });
                }
            }
        }
        
        // 按时间排序（最新的在前）
        decisions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        decisions.truncate(limit);
        
        Ok(decisions)
    }
    
    /// 将记忆同步到上下文
    pub async fn sync_to_context(&self, context: &mut Vec<String>) -> Result<()> {
        // 读取 MEMORY.md
        if let Ok(memory) = self.read_memory().await {
            context.push(format!("[MEMORY.md]\n{}", memory.content));
        }
        
        // 读取 SOUL.md
        if let Ok(soul) = self.read_soul().await {
            context.push(format!("[SOUL.md]\n{}", soul.content));
        }
        
        // 读取最近的决策
        if let Ok(decisions) = self.get_recent_decisions(5).await {
            if !decisions.is_empty() {
                let decisions_text = decisions.iter()
                    .map(|d| format!("- {}: {}", d.title, d.content))
                    .collect::<Vec<_>>()
                    .join("\n");
                context.push(format!("[最近决策]\n{}", decisions_text));
            }
        }
        
        Ok(())
    }
    
    /// 解析 Markdown 节
    fn parse_sections(content: &str) -> Vec<MemorySection> {
        let mut sections = Vec::new();
        let mut current_section: Option<MemorySection> = None;
        let mut content_buffer = String::new();
        
        for line in content.lines() {
            // 检测标题行
            if line.starts_with('#') {
                // 保存上一个节
                if let Some(mut section) = current_section.take() {
                    section.content = content_buffer.trim().to_string();
                    sections.push(section);
                    content_buffer.clear();
                }
                
                // 解析新节
                let level = line.chars().take_while(|&c| c == '#').count() as u8;
                let title = line.trim_start_matches('#').trim().to_string();
                
                current_section = Some(MemorySection {
                    title,
                    level,
                    content: String::new(),
                    children: Vec::new(),
                });
            } else if let Some(ref _section) = current_section {
                content_buffer.push_str(line);
                content_buffer.push('\n');
            }
        }
        
        // 保存最后一个节
        if let Some(mut section) = current_section {
            section.content = content_buffer.trim().to_string();
            sections.push(section);
        }
        
        sections
    }
    
    /// 获取工作区路径
    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_write_and_read_memory() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileMemoryManager::new(temp_dir.path().to_path_buf());
        
        let content = "# Test Memory\n\nThis is a test.";
        manager.write_memory(content).await.unwrap();
        
        let memory = manager.read_memory().await.unwrap();
        assert_eq!(memory.content, content);
        assert_eq!(memory.sections.len(), 1);
        assert_eq!(memory.sections[0].title, "Test Memory");
    }
    
    #[tokio::test]
    async fn test_write_decision() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileMemoryManager::new(temp_dir.path().to_path_buf());
        
        let decision = Decision {
            id: "test-1".to_string(),
            timestamp: Utc::now(),
            title: "测试决策".to_string(),
            content: "这是一个测试决策".to_string(),
            context: None,
            impact: None,
        };
        
        manager.write_decision(&decision).await.unwrap();
        
        let decisions = manager.get_recent_decisions(10).await.unwrap();
        assert!(!decisions.is_empty());
    }
    
    #[tokio::test]
    async fn test_sync_to_context() {
        let temp_dir = TempDir::new().unwrap();
        let manager = FileMemoryManager::new(temp_dir.path().to_path_buf());
        
        manager.write_memory("# Memory\n\nTest content").await.unwrap();
        
        let mut context = Vec::new();
        manager.sync_to_context(&mut context).await.unwrap();
        
        assert!(!context.is_empty());
        assert!(context[0].contains("[MEMORY.md]"));
    }
    
    #[test]
    fn test_parse_sections() {
        let content = r#"# Title 1

Content 1

## Title 2

Content 2

# Title 3

Content 3
"#;
        
        let sections = FileMemoryManager::parse_sections(content);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].title, "Title 1");
        assert_eq!(sections[0].level, 1);
        assert_eq!(sections[1].title, "Title 2");
        assert_eq!(sections[1].level, 2);
    }
}