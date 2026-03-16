// Workspace Manager - v0.7.0
//
// 管理 NewClaw workspace 目录：
// - 打包转移 (export)
// - 跨设备恢复 (import)
// - 校验和验证 (verify)
// - 版本控制

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use sha2::{Sha256, Digest};
use tracing::{info, warn};

/// Workspace 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    /// 工作空间名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 创建时间
    pub created_at: String,
    /// 所有者
    pub owner: String,
    /// 描述
    pub description: String,
    /// 最后更新时间
    pub updated_at: Option<String>,
    /// 文件校验和
    pub checksums: HashMap<String, String>,
}

impl Default for WorkspaceMetadata {
    fn default() -> Self {
        Self {
            name: "newclaw-workspace".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            owner: "unknown".to_string(),
            description: "NewClaw workspace".to_string(),
            updated_at: None,
            checksums: HashMap::new(),
        }
    }
}

/// Workspace Manager
pub struct WorkspaceManager {
    /// workspace 目录路径
    workspace_dir: PathBuf,
    /// 元数据
    metadata: WorkspaceMetadata,
}

impl WorkspaceManager {
    /// 创建新的 WorkspaceManager
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Result<Self> {
        let workspace_dir = workspace_dir.into();
        
        // 确保目录存在
        std::fs::create_dir_all(&workspace_dir)
            .with_context(|| format!("Failed to create workspace dir: {:?}", workspace_dir))?;
        
        // 确保子目录存在
        std::fs::create_dir_all(workspace_dir.join("memory"))?;
        
        // 加载或创建元数据
        let metadata = Self::load_or_create_metadata(&workspace_dir)?;
        
        Ok(Self {
            workspace_dir,
            metadata,
        })
    }

    /// 加载或创建元数据
    fn load_or_create_metadata(workspace_dir: &Path) -> Result<WorkspaceMetadata> {
        let metadata_path = workspace_dir.join("workspace.yaml");
        
        if metadata_path.exists() {
            let content = std::fs::read_to_string(&metadata_path)
                .with_context(|| "Failed to read workspace.yaml")?;
            let metadata: WorkspaceMetadata = serde_yaml::from_str(&content)
                .with_context(|| "Failed to parse workspace.yaml")?;
            Ok(metadata)
        } else {
            let metadata = WorkspaceMetadata::default();
            Self::save_metadata(workspace_dir, &metadata)?;
            Ok(metadata)
        }
    }

    /// 保存元数据
    fn save_metadata(workspace_dir: &Path, metadata: &WorkspaceMetadata) -> Result<()> {
        let metadata_path = workspace_dir.join("workspace.yaml");
        let content = serde_yaml::to_string(metadata)
            .with_context(|| "Failed to serialize metadata")?;
        std::fs::write(&metadata_path, content)
            .with_context(|| "Failed to write workspace.yaml")?;
        Ok(())
    }

    /// 计算文件 SHA256 校验和
    pub fn calculate_checksum(path: &Path) -> Result<String> {
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;
        
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        
        Ok(format!("{:x}", hash))
    }

    /// 更新所有文件的校验和
    pub fn update_checksums(&mut self) -> Result<()> {
        let files = self.list_workspace_files()?;
        
        self.metadata.checksums.clear();
        
        for file in files {
            let relative = file.strip_prefix(&self.workspace_dir)
                .with_context(|| "Failed to strip prefix")?
                .to_string_lossy()
                .to_string();
            
            let checksum = Self::calculate_checksum(&file)?;
            self.metadata.checksums.insert(relative, checksum);
        }
        
        self.metadata.updated_at = Some(chrono::Utc::now().to_rfc3339());
        Self::save_metadata(&self.workspace_dir, &self.metadata)?;
        
        info!("Updated checksums for {} files", self.metadata.checksums.len());
        Ok(())
    }

    /// 列出所有 workspace 文件
    pub fn list_workspace_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.collect_files(&self.workspace_dir, &mut files)?;
        Ok(files)
    }

    /// 递归收集文件
    fn collect_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // 跳过隐藏目录
                if !path.file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    self.collect_files(&path, files)?;
                }
            } else {
                files.push(path);
            }
        }
        Ok(())
    }

    /// 验证文件完整性
    pub fn verify(&self) -> Result<VerifyResult> {
        let mut result = VerifyResult {
            valid: true,
            checked: 0,
            passed: 0,
            failed: Vec::new(),
        };
        
        for (relative, expected_checksum) in &self.metadata.checksums {
            let file_path = self.workspace_dir.join(relative);
            result.checked += 1;
            
            if !file_path.exists() {
                result.valid = false;
                result.failed.push(format!("Missing: {}", relative));
                continue;
            }
            
            match Self::calculate_checksum(&file_path) {
                Ok(actual_checksum) => {
                    if actual_checksum == *expected_checksum {
                        result.passed += 1;
                    } else {
                        result.valid = false;
                        result.failed.push(format!("Checksum mismatch: {}", relative));
                    }
                }
                Err(e) => {
                    result.valid = false;
                    result.failed.push(format!("Error reading {}: {}", relative, e));
                }
            }
        }
        
        Ok(result)
    }

    /// 导出 workspace 到 tar.gz
    pub fn export(&self, output_path: &Path) -> Result<()> {
        // 先更新校验和
        let mut metadata = self.metadata.clone();
        let files = self.list_workspace_files()?;
        
        metadata.checksums.clear();
        for file in &files {
            let relative = file.strip_prefix(&self.workspace_dir)?
                .to_string_lossy()
                .to_string();
            let checksum = Self::calculate_checksum(file)?;
            metadata.checksums.insert(relative, checksum);
        }
        metadata.updated_at = Some(chrono::Utc::now().to_rfc3339());
        
        // 保存更新后的元数据
        Self::save_metadata(&self.workspace_dir, &metadata)?;
        
        // 创建 tar.gz
        let file = std::fs::File::create(output_path)
            .with_context(|| format!("Failed to create output file: {:?}", output_path))?;
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        
        // 添加所有文件
        for file in &files {
            let relative = file.strip_prefix(&self.workspace_dir)?;
            tar.append_path_with_name(file, relative)
                .with_context(|| format!("Failed to add file to archive: {:?}", file))?;
        }
        
        tar.finish()?;
        
        info!("Exported workspace to {:?}", output_path);
        Ok(())
    }

    /// 从 tar.gz 导入 workspace
    pub fn import(archive_path: &Path, workspace_dir: &Path) -> Result<Self> {
        // 创建目录
        std::fs::create_dir_all(workspace_dir)?;
        
        // 解压 tar.gz
        let file = std::fs::File::open(archive_path)
            .with_context(|| format!("Failed to open archive: {:?}", archive_path))?;
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        
        tar.unpack(workspace_dir)
            .with_context(|| "Failed to unpack archive")?;
        
        // 验证
        let manager = Self::new(workspace_dir)?;
        let verify_result = manager.verify()?;
        
        if !verify_result.valid {
            warn!("Import verification failed: {:?}", verify_result.failed);
        }
        
        info!("Imported workspace from {:?}", archive_path);
        Ok(manager)
    }

    /// 获取 workspace 路径
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }

    /// 获取元数据
    pub fn metadata(&self) -> &WorkspaceMetadata {
        &self.metadata
    }

    /// 获取 MEMORY.md 路径
    pub fn memory_path(&self) -> PathBuf {
        self.workspace_dir.join("MEMORY.md")
    }

    /// 获取每日记录路径
    pub fn daily_memory_path(&self, date: &str) -> PathBuf {
        self.workspace_dir.join("memory").join(format!("{}.md", date))
    }

    /// 获取 SOUL.md 路径
    pub fn soul_path(&self) -> PathBuf {
        self.workspace_dir.join("SOUL.md")
    }

    /// 获取 USER.md 路径
    pub fn user_path(&self) -> PathBuf {
        self.workspace_dir.join("USER.md")
    }
}

/// 验证结果
#[derive(Debug, Clone)]
pub struct VerifyResult {
    /// 是否全部通过
    pub valid: bool,
    /// 检查的文件数
    pub checked: usize,
    /// 通过的文件数
    pub passed: usize,
    /// 失败的文件列表
    pub failed: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_workspace_manager() {
        let dir = tempdir().unwrap();
        let manager = WorkspaceManager::new(dir.path()).unwrap();
        
        assert!(manager.workspace_dir().exists());
        assert!(manager.memory_path().parent().unwrap().exists());
    }

    #[test]
    fn test_checksum() {
        let dir = tempdir().unwrap();
        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "hello world").unwrap();
        
        let checksum = WorkspaceManager::calculate_checksum(&test_file).unwrap();
        assert_eq!(checksum.len(), 64); // SHA256 hex string length
    }
}