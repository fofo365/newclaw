// Context Config - v0.5.3
//
// 声明式策略配置，支持热重载

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow};

/// 上下文配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ContextSystemConfig {
    /// 嵌入配置
    pub embedding: EmbeddingSection,
    /// 检索配置
    pub retrieval: RetrievalSection,
    /// 压缩配置
    pub compression: CompressionSection,
    /// 策略配置
    pub policy: PolicySection,
    /// 存储配置
    pub storage: StorageSection,
}


/// 嵌入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSection {
    /// 是否启用
    pub enabled: bool,
    /// 嵌入模型
    pub model: String,
    /// 向量维度
    pub dimensions: usize,
    /// 批量大小
    pub batch_size: usize,
    /// 是否启用缓存
    pub cache_enabled: bool,
    /// 缓存 TTL (秒)
    pub cache_ttl_secs: u64,
}

impl Default for EmbeddingSection {
    fn default() -> Self {
        Self {
            enabled: true,
            model: "text-embedding-3-small".to_string(),
            dimensions: 1536,
            batch_size: 10,
            cache_enabled: true,
            cache_ttl_secs: 86400,
        }
    }
}

/// 检索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalSection {
    /// 是否启用
    pub enabled: bool,
    /// Top-K 结果数
    pub top_k: usize,
    /// 相似度阈值
    pub similarity_threshold: f32,
    /// 是否启用混合检索
    pub hybrid_enabled: bool,
    /// 语义检索权重
    pub semantic_weight: f32,
    /// 关键词检索权重
    pub keyword_weight: f32,
}

impl Default for RetrievalSection {
    fn default() -> Self {
        Self {
            enabled: true,
            top_k: 5,
            similarity_threshold: 0.5,
            hybrid_enabled: true,
            semantic_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
}

/// 压缩配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionSection {
    /// 是否启用
    pub enabled: bool,
    /// 目标压缩率
    pub target_ratio: f32,
    /// 最大 Token 数
    pub max_tokens: usize,
    /// 保留最近消息数
    pub keep_recent_count: usize,
    /// 是否保留系统消息
    pub keep_system_messages: bool,
}

impl Default for CompressionSection {
    fn default() -> Self {
        Self {
            enabled: true,
            target_ratio: 0.5,
            max_tokens: 4000,
            keep_recent_count: 3,
            keep_system_messages: true,
        }
    }
}

/// 策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySection {
    /// 是否启用
    pub enabled: bool,
    /// 默认策略
    pub default_policy: String,
    /// 策略列表
    pub policies: Vec<PolicyConfig>,
}

impl Default for PolicySection {
    fn default() -> Self {
        Self {
            enabled: true,
            default_policy: "balanced".to_string(),
            policies: vec![],
        }
    }
}

/// 单个策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// 策略名称
    pub name: String,
    /// 策略类型
    pub policy_type: String,
    /// 参数
    pub params: HashMap<String, serde_json::Value>,
}

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    /// 数据库路径
    pub db_path: String,
    /// 向量存储类型
    pub vector_store_type: String,
    /// 最大存储大小 (MB)
    pub max_storage_mb: usize,
    /// 是否自动清理
    pub auto_cleanup: bool,
    /// 清理周期 (秒)
    pub cleanup_interval_secs: u64,
}

impl Default for StorageSection {
    fn default() -> Self {
        Self {
            db_path: "/var/lib/newclaw/context.db".to_string(),
            vector_store_type: "memory".to_string(),
            max_storage_mb: 500,
            auto_cleanup: true,
            cleanup_interval_secs: 3600,
        }
    }
}

/// 配置加载器
pub struct ConfigLoader {
    /// 配置路径
    config_path: Option<PathBuf>,
    /// 当前配置
    config: ContextSystemConfig,
}

impl ConfigLoader {
    /// 创建新的配置加载器
    pub fn new() -> Self {
        Self {
            config_path: None,
            config: ContextSystemConfig::default(),
        }
    }
    
    /// 从文件加载配置
    pub fn load_from_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        
        let config: ContextSystemConfig = match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => {
                toml::from_str(&content)
                    .map_err(|e| anyhow!("Failed to parse TOML: {}", e))?
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::from_str(&content)
                    .map_err(|e| anyhow!("Failed to parse YAML: {}", e))?
            }
            Some("json") => {
                serde_json::from_str(&content)
                    .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?
            }
            _ => return Err(anyhow!("Unsupported config format")),
        };
        
        self.config_path = Some(path.to_path_buf());
        self.config = config;
        
        Ok(())
    }
    
    /// 获取当前配置
    pub fn get_config(&self) -> &ContextSystemConfig {
        &self.config
    }
    
    /// 保存配置到文件
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => {
                toml::to_string_pretty(&self.config)
                    .map_err(|e| anyhow!("Failed to serialize TOML: {}", e))?
            }
            Some("yaml") | Some("yml") => {
                serde_yaml::to_string(&self.config)
                    .map_err(|e| anyhow!("Failed to serialize YAML: {}", e))?
            }
            Some("json") => {
                serde_json::to_string_pretty(&self.config)
                    .map_err(|e| anyhow!("Failed to serialize JSON: {}", e))?
            }
            _ => return Err(anyhow!("Unsupported config format")),
        };
        
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// 更新配置
    pub fn update<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut ContextSystemConfig),
    {
        f(&mut self.config);
        self.validate()?;
        Ok(())
    }
    
    /// 验证配置
    pub fn validate(&self) -> Result<()> {
        // 验证嵌入配置
        if self.config.embedding.enabled {
            if self.config.embedding.dimensions == 0 {
                return Err(anyhow!("Embedding dimensions must be > 0"));
            }
            if self.config.embedding.batch_size == 0 {
                return Err(anyhow!("Embedding batch_size must be > 0"));
            }
        }
        
        // 验证检索配置
        if self.config.retrieval.enabled {
            if self.config.retrieval.top_k == 0 {
                return Err(anyhow!("Retrieval top_k must be > 0"));
            }
            if self.config.retrieval.similarity_threshold < 0.0 
                || self.config.retrieval.similarity_threshold > 1.0 {
                return Err(anyhow!("Similarity threshold must be between 0 and 1"));
            }
        }
        
        // 验证压缩配置
        if self.config.compression.enabled
            && (self.config.compression.target_ratio <= 0.0 
                || self.config.compression.target_ratio > 1.0) {
                return Err(anyhow!("Target ratio must be between 0 and 1"));
            }
        
        Ok(())
    }
    
    /// 生成默认配置文件
    pub fn generate_default_config(path: &Path) -> Result<()> {
        let config = ContextSystemConfig::default();
        let loader = Self {
            config_path: Some(path.to_path_buf()),
            config,
        };
        loader.save_to_file(path)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_system_config_default() {
        let config = ContextSystemConfig::default();
        assert!(config.embedding.enabled);
        assert!(config.retrieval.enabled);
    }

    #[test]
    fn test_embedding_section_default() {
        let section = EmbeddingSection::default();
        assert_eq!(section.model, "text-embedding-3-small");
        assert_eq!(section.dimensions, 1536);
    }

    #[test]
    fn test_retrieval_section_default() {
        let section = RetrievalSection::default();
        assert_eq!(section.top_k, 5);
        assert!((section.similarity_threshold - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_config_loader_new() {
        let loader = ConfigLoader::new();
        assert!(loader.config_path.is_none());
    }

    #[test]
    fn test_config_loader_validate() {
        let loader = ConfigLoader::new();
        assert!(loader.validate().is_ok());
    }

    #[test]
    fn test_config_loader_update() {
        let mut loader = ConfigLoader::new();
        
        loader.update(|c| {
            c.embedding.batch_size = 20;
        }).unwrap();
        
        assert_eq!(loader.get_config().embedding.batch_size, 20);
    }

    #[test]
    fn test_config_loader_validate_invalid() {
        let mut loader = ConfigLoader::new();
        
        // 直接修改配置，不经过 update（它会调用 validate）
        loader.config.retrieval.similarity_threshold = 2.0;
        
        assert!(loader.validate().is_err());
    }
}
