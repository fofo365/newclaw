// Memory Storage - 统一记忆存储抽象
//
// v0.7.0 - 实现持久化存储，支持 FTS5 全文索引
// v0.7.0 - 多层隔离机制：用户、通道、Agent、命名空间
//
// 注意：SQLiteMemoryStorage 的实现在 storage_impl.rs 中

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use super::shared::{MemoryEntry, MemoryType, UserId};

// ============================================================================
// 多层隔离标识 - v0.7.0
// ============================================================================

/// 记忆隔离维度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MemoryScope {
    /// 用户 ID
    pub user_id: String,
    /// 通道类型 (cli/dashboard/feishu/telegram...)
    pub channel: String,
    /// Agent ID (可选)
    pub agent_id: Option<String>,
    /// 命名空间 (可选，用于业务逻辑分组)
    pub namespace: Option<String>,
}

impl MemoryScope {
    /// 创建用户+通道隔离
    pub fn for_channel(user_id: &str, channel: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            channel: channel.to_string(),
            agent_id: None,
            namespace: None,
        }
    }

    /// 创建完整隔离
    pub fn full(user_id: &str, channel: &str, agent_id: &str, namespace: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            channel: channel.to_string(),
            agent_id: Some(agent_id.to_string()),
            namespace: Some(namespace.to_string()),
        }
    }

    /// 设置 Agent
    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }

    /// 设置命名空间
    pub fn with_namespace(mut self, namespace: &str) -> Self {
        self.namespace = Some(namespace.to_string());
        self
    }

    /// 生成 SQL WHERE 条件
    pub fn to_where_clause(&self) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
        let mut conditions = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        conditions.push("user_id = ?");
        params.push(Box::new(self.user_id.clone()));

        conditions.push("channel = ?");
        params.push(Box::new(self.channel.clone()));

        if let Some(ref agent_id) = self.agent_id {
            conditions.push("agent_id = ?");
            params.push(Box::new(agent_id.clone()));
        }

        if let Some(ref namespace) = self.namespace {
            conditions.push("namespace = ?");
            params.push(Box::new(namespace.clone()));
        }

        (conditions.join(" AND "), params)
    }
}

impl Default for MemoryScope {
    fn default() -> Self {
        Self {
            user_id: "default".to_string(),
            channel: "global".to_string(),
            agent_id: None,
            namespace: None,
        }
    }
}

// ============================================================================
// Hybrid Search - 混合检索配置和结果
// ============================================================================

/// 混合搜索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchConfig {
    /// 返回结果数量
    pub top_k: usize,
    /// BM25 权重
    pub bm25_weight: f32,
    /// 向量权重
    pub vector_weight: f32,
    /// 是否应用时间衰减
    pub apply_time_decay: bool,
    /// 时间衰减系数 (λ)
    pub decay_lambda: f32,
    /// 最小相似度阈值
    pub min_score: f32,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            top_k: 10,
            bm25_weight: 0.7,
            vector_weight: 0.3,
            apply_time_decay: true,
            decay_lambda: 0.1,
            min_score: 0.0,
        }
    }
}

impl HybridSearchConfig {
    /// 创建严格配置（精确匹配优先）
    pub fn strict() -> Self {
        Self {
            top_k: 5,
            bm25_weight: 0.8,
            vector_weight: 0.2,
            apply_time_decay: true,
            decay_lambda: 0.05,
            min_score: 0.3,
        }
    }
    
    /// 创建语义配置（向量优先）
    pub fn semantic() -> Self {
        Self {
            top_k: 20,
            bm25_weight: 0.3,
            vector_weight: 0.7,
            apply_time_decay: true,
            decay_lambda: 0.1,
            min_score: 0.1,
        }
    }
}

/// 混合搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    /// 记忆 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// BM25 分数
    pub bm25_score: f32,
    /// 向量分数
    pub vector_score: f32,
    /// 最终分数（融合后）
    pub final_score: f32,
    /// 重要性
    pub importance: f32,
    /// 创建时间
    pub created_at: String,
}

// ============================================================================
// Memory Storage Trait
// ============================================================================

/// 记忆存储 Trait - 统一抽象
#[async_trait::async_trait]
pub trait MemoryStorage: Send + Sync {
    /// 存储记忆条目（带隔离维度）
    async fn store_with_scope(&self, entry: &MemoryEntry, scope: &MemoryScope) -> anyhow::Result<String>;
    
    /// 存储记忆条目（默认隔离）
    async fn store(&self, entry: &MemoryEntry) -> anyhow::Result<String> {
        self.store_with_scope(entry, &MemoryScope::default()).await
    }
    
    /// 获取记忆条目
    async fn retrieve(&self, id: &str) -> anyhow::Result<Option<MemoryEntry>>;
    
    /// 删除记忆条目
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
    
    /// 混合搜索（BM25 + 向量）- 带隔离
    async fn search_hybrid_with_scope(
        &self, 
        query: &str, 
        config: &HybridSearchConfig,
        scope: &MemoryScope
    ) -> anyhow::Result<Vec<HybridSearchResult>>;
    
    /// 混合搜索（默认隔离）
    async fn search_hybrid(&self, query: &str, config: &HybridSearchConfig) -> anyhow::Result<Vec<HybridSearchResult>> {
        self.search_hybrid_with_scope(query, config, &MemoryScope::default()).await
    }
    
    /// BM25 全文搜索 - 带隔离
    async fn search_bm25_with_scope(
        &self, 
        query: &str, 
        limit: usize,
        scope: &MemoryScope
    ) -> anyhow::Result<Vec<HybridSearchResult>>;
    
    /// BM25 全文搜索（默认隔离）
    async fn search_bm25(&self, query: &str, limit: usize) -> anyhow::Result<Vec<HybridSearchResult>> {
        self.search_bm25_with_scope(query, limit, &MemoryScope::default()).await
    }
    
    /// 获取用户所有记忆
    async fn get_user_memories(&self, user_id: &UserId) -> anyhow::Result<Vec<MemoryEntry>>;
    
    /// 按隔离维度获取记忆
    async fn get_memories_by_scope(&self, scope: &MemoryScope) -> anyhow::Result<Vec<MemoryEntry>>;
    
    /// 获取存储统计
    async fn stats(&self) -> anyhow::Result<StorageStats>;
    
    /// 获取隔离维度统计
    async fn stats_by_scope(&self, scope: &MemoryScope) -> anyhow::Result<StorageStats>;
}

/// 存储统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_entries: usize,
    pub total_users: usize,
    pub db_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}

/// 存储配置
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub db_path: std::path::PathBuf,
    pub enable_fts: bool,
    pub enable_vector: bool,
    pub max_entries: usize,
    pub auto_cleanup_days: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: std::path::PathBuf::from("data/memory.db"),
            enable_fts: true,
            enable_vector: true,
            max_entries: 100000,
            auto_cleanup_days: 30,
        }
    }
}

impl StorageConfig {
    pub fn in_memory() -> Self {
        Self {
            db_path: std::path::PathBuf::from(":memory:"),
            enable_fts: true,
            enable_vector: true,
            max_entries: 10000,
            auto_cleanup_days: 7,
        }
    }
}

// ============================================================================
// MMR 去重
// ============================================================================

/// MMR 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MMRConfig {
    /// λ 参数 (0-1)，越大越强调相关性，越小越强调多样性
    pub lambda: f32,
    /// 返回结果数量
    pub top_k: usize,
}

impl Default for MMRConfig {
    fn default() -> Self {
        Self {
            lambda: 0.7,
            top_k: 10,
        }
    }
}

/// MMR 去重算法
pub fn mmr_diversify(
    results: Vec<HybridSearchResult>,
    config: &MMRConfig,
) -> Vec<HybridSearchResult> {
    if results.is_empty() || config.top_k == 0 {
        return results;
    }

    let mut selected: Vec<HybridSearchResult> = Vec::new();
    let mut remaining: Vec<HybridSearchResult> = results;

    // 选择第一个（最高分）
    if let Some(first) = remaining.first() {
        selected.push(first.clone());
        remaining.remove(0);
    }

    // 迭代选择剩余
    while selected.len() < config.top_k && !remaining.is_empty() {
        let best_idx = find_best_mmr(&selected, &remaining, config.lambda);
        selected.push(remaining.remove(best_idx));
    }

    selected
}

fn find_best_mmr(
    selected: &[HybridSearchResult],
    remaining: &[HybridSearchResult],
    lambda: f32,
) -> usize {
    let mut best_idx = 0;
    let mut best_score = f32::MIN;

    for (idx, candidate) in remaining.iter().enumerate() {
        // 相关性分数
        let relevance = candidate.final_score;

        // 计算与已选择的最大相似度（简化：使用内容长度差异作为相似度代理）
        let max_similarity = selected
            .iter()
            .map(|s| {
                let len_diff = (s.content.len() as f32 - candidate.content.len() as f32).abs();
                1.0 / (1.0 + len_diff / 100.0)
            })
            .fold(0.0, f32::max);

        // MMR 分数
        let mmr_score = lambda * relevance - (1.0 - lambda) * max_similarity;

        if mmr_score > best_score {
            best_score = mmr_score;
            best_idx = idx;
        }
    }

    best_idx
}