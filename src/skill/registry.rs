// Skill Registry - v0.5.2
//
// Skill 注册表

use super::{SkillConfig, SkillId, SkillLoader, SkillExecutor, SkillInput, SkillOutput};
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Skill 注册表
pub struct SkillRegistry {
    /// 已注册的 Skill
    skills: Arc<RwLock<HashMap<SkillId, SkillConfig>>>,
    /// 加载器
    loader: SkillLoader,
    /// 执行器
    executor: SkillExecutor,
}

impl SkillRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            loader: SkillLoader::with_default_paths(),
            executor: SkillExecutor::new(),
        }
    }
    
    /// 注册 Skill
    pub async fn register(&self, skill: SkillConfig) -> Result<()> {
        let mut skills = self.skills.write().await;
        skills.insert(skill.id.clone(), skill);
        Ok(())
    }
    
    /// 注销 Skill
    pub async fn unregister(&self, id: &SkillId) -> Result<()> {
        let mut skills = self.skills.write().await;
        skills.remove(id)
            .map(|_| ())
            .ok_or_else(|| anyhow!("Skill not found: {}", id))
    }
    
    /// 获取 Skill
    pub async fn get(&self, id: &SkillId) -> Option<SkillConfig> {
        let skills = self.skills.read().await;
        skills.get(id).cloned()
    }
    
    /// 列出所有 Skill
    pub async fn list(&self) -> Vec<SkillConfig> {
        let skills = self.skills.read().await;
        skills.values().cloned().collect()
    }
    
    /// 发现并注册 Skill
    pub async fn discover(&self) -> Result<usize> {
        let discovered = self.loader.discover()?;
        let count = discovered.len();
        
        let mut skills = self.skills.write().await;
        for skill in discovered {
            skills.insert(skill.id.clone(), skill);
        }
        
        Ok(count)
    }
    
    /// 执行 Skill
    pub async fn execute(&self, id: &SkillId, input: SkillInput) -> Result<SkillOutput> {
        let skill = self.get(id).await
            .ok_or_else(|| anyhow!("Skill not found: {}", id))?;
        
        self.executor.execute(&skill, input).await
    }
    
    /// 按名称查找 Skill
    pub async fn find_by_name(&self, name: &str) -> Option<SkillConfig> {
        let skills = self.skills.read().await;
        skills.values()
            .find(|s| s.name == name)
            .cloned()
    }
    
    /// 获取 Skill 数量
    pub async fn len(&self) -> usize {
        self.skills.read().await.len()
    }
    
    /// 检查是否为空
    pub async fn is_empty(&self) -> bool {
        self.skills.read().await.is_empty()
    }
    
    /// 清空注册表
    pub async fn clear(&self) {
        self.skills.write().await.clear();
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_new() {
        let registry = SkillRegistry::new();
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_registry_register() {
        let registry = SkillRegistry::new();
        let skill = SkillConfig {
            id: SkillId::new("test"),
            name: "Test Skill".to_string(),
            ..Default::default()
        };
        
        registry.register(skill).await.unwrap();
        assert_eq!(registry.len().await, 1);
    }

    #[tokio::test]
    async fn test_registry_get() {
        let registry = SkillRegistry::new();
        let skill = SkillConfig {
            id: SkillId::new("test"),
            name: "Test Skill".to_string(),
            ..Default::default()
        };
        
        registry.register(skill.clone()).await.unwrap();
        
        let retrieved = registry.get(&skill.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Skill");
    }

    #[tokio::test]
    async fn test_registry_unregister() {
        let registry = SkillRegistry::new();
        let skill = SkillConfig {
            id: SkillId::new("test"),
            name: "Test Skill".to_string(),
            ..Default::default()
        };
        
        registry.register(skill.clone()).await.unwrap();
        registry.unregister(&skill.id).await.unwrap();
        
        assert!(registry.is_empty().await);
    }

    #[tokio::test]
    async fn test_registry_list() {
        let registry = SkillRegistry::new();
        
        registry.register(SkillConfig {
            id: SkillId::new("skill1"),
            name: "Skill 1".to_string(),
            ..Default::default()
        }).await.unwrap();
        
        registry.register(SkillConfig {
            id: SkillId::new("skill2"),
            name: "Skill 2".to_string(),
            ..Default::default()
        }).await.unwrap();
        
        let list = registry.list().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_find_by_name() {
        let registry = SkillRegistry::new();
        
        registry.register(SkillConfig {
            id: SkillId::new("test-id"),
            name: "Test Skill".to_string(),
            ..Default::default()
        }).await.unwrap();
        
        let found = registry.find_by_name("Test Skill").await;
        assert!(found.is_some());
    }
}
