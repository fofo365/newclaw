//! Federated Memory Query - 联邦记忆查询
//!
//! 提供跨节点的联邦查询功能
//! 支持查询路由、结果聚合、去重排序
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use super::protocol::{NodeId, FederatedError, FederatedResult};
use super::message::{MemoryQueryPayload, QueryFiltersPayload};
use crate::memory::{HybridSearchConfig, HybridSearchResult, MMRConfig, mmr_diversify};

// ============================================================================
// 查询错误
// ============================================================================

#[derive(Debug, Error)]
pub enum QueryError {
    #[error("查询错误: {0}")]
    QueryError(String),
    
    #[error("超时: {0}")]
    Timeout(String),
    
    #[error("节点不可用: {0}")]
    NodeUnavailable(String),
    
    #[error("结果聚合失败: {0}")]
    AggregationError(String),
    
    #[error("路由错误: {0}")]
    RoutingError(String),
}

pub type QueryResult<T> = std::result::Result<T, QueryError>;

// ============================================================================
// 查询定义
// ============================================================================

/// 联邦查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedQuery {
    /// 查询 ID
    pub id: String,
    /// 查询内容
    pub query: String,
    /// 查询类型
    pub query_type: QueryType,
    /// 目标节点
    pub target_nodes: Vec<NodeId>,
    /// 返回数量
    pub limit: usize,
    /// 偏移量
    pub offset: usize,
    /// 过滤条件
    pub filters: QueryFilters,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 最小节点数（必须成功的节点数）
    pub min_nodes: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

impl FederatedQuery {
    pub fn new(query: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            query,
            query_type: QueryType::Hybrid,
            target_nodes: Vec::new(),
            limit: 10,
            offset: 0,
            filters: QueryFilters::default(),
            timeout_ms: 30000,
            min_nodes: 1,
            created_at: Utc::now(),
        }
    }
    
    pub fn with_query_type(mut self, query_type: QueryType) -> Self {
        self.query_type = query_type;
        self
    }
    
    pub fn with_target_nodes(mut self, nodes: Vec<NodeId>) -> Self {
        self.target_nodes = nodes;
        self
    }
    
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
    
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
    
    pub fn with_filters(mut self, filters: QueryFilters) -> Self {
        self.filters = filters;
        self
    }
    
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
    
    pub fn all_nodes(mut self, nodes: Vec<NodeId>) -> Self {
        self.target_nodes = nodes;
        self.min_nodes = 1;
        self
    }
    
    pub fn quorum(mut self, min_nodes: usize) -> Self {
        self.min_nodes = min_nodes;
        self
    }
}

/// 查询类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// 关键词搜索
    Keyword,
    /// 向量搜索
    Vector,
    /// 混合搜索
    Hybrid,
    /// 全量搜索
    Full,
}

impl Default for QueryType {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// 查询过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    /// 记忆类型
    pub memory_type: Option<String>,
    /// 时间范围
    pub time_range: Option<TimeRange>,
    /// 最小重要性
    pub min_importance: Option<f32>,
    /// 标签过滤
    pub tags: Vec<String>,
    /// 来源节点过滤
    pub source_nodes: Vec<NodeId>,
}

/// 时间范围
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

// ============================================================================
// 查询响应
// ============================================================================

/// 联邦查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedQueryResponse {
    /// 查询 ID
    pub query_id: String,
    /// 聚合结果
    pub results: Vec<AggregatedResult>,
    /// 总数
    pub total: usize,
    /// 成功节点数
    pub successful_nodes: usize,
    /// 失败节点数
    pub failed_nodes: usize,
    /// 查询耗时（毫秒）
    pub elapsed_ms: u64,
    /// 节点响应详情
    pub node_responses: Vec<NodeQueryResponse>,
}

/// 节点查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeQueryResponse {
    /// 节点 ID
    pub node_id: NodeId,
    /// 是否成功
    pub success: bool,
    /// 结果数量
    pub result_count: usize,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 错误信息
    pub error: Option<String>,
}

/// 聚合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResult {
    /// 条目 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// 最终分数
    pub score: f32,
    /// 重要性
    pub importance: f32,
    /// 来源节点
    pub source_nodes: Vec<NodeId>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 记忆类型
    pub memory_type: String,
    /// 标签
    pub tags: Vec<String>,
}

// ============================================================================
// 查询路由
// ============================================================================

/// 查询路由器
pub struct QueryRouter {
    /// 已知节点
    known_nodes: RwLock<HashSet<NodeId>>,
    /// 节点权重
    node_weights: RwLock<HashMap<NodeId, f32>>,
    /// 节点延迟
    node_latencies: RwLock<HashMap<NodeId, Duration>>,
}

impl QueryRouter {
    pub fn new() -> Self {
        Self {
            known_nodes: RwLock::new(HashSet::new()),
            node_weights: RwLock::new(HashMap::new()),
            node_latencies: RwLock::new(HashMap::new()),
        }
    }
    
    /// 添加节点
    pub async fn add_node(&self, node: NodeId) {
        let mut nodes = self.known_nodes.write().await;
        nodes.insert(node);
    }
    
    /// 移除节点
    pub async fn remove_node(&self, node: &NodeId) {
        let mut nodes = self.known_nodes.write().await;
        nodes.remove(node);
        
        let mut weights = self.node_weights.write().await;
        weights.remove(node);
        
        let mut latencies = self.node_latencies.write().await;
        latencies.remove(node);
    }
    
    /// 更新节点延迟
    pub async fn update_latency(&self, node: &NodeId, latency: Duration) {
        let mut latencies = self.node_latencies.write().await;
        latencies.insert(node.clone(), latency);
        
        // 更新权重（延迟越低权重越高）
        let mut weights = self.node_weights.write().await;
        let weight = 1.0 / (1.0 + latency.as_secs_f32());
        weights.insert(node.clone(), weight);
    }
    
    /// 获取查询目标节点
    pub async fn get_query_targets(&self, query: &FederatedQuery) -> Vec<NodeId> {
        // 如果指定了目标节点，使用指定的
        if !query.target_nodes.is_empty() {
            return query.target_nodes.clone();
        }
        
        // 否则选择最优节点
        let nodes = self.known_nodes.read().await;
        let weights = self.node_weights.read().await;
        
        let mut ranked: Vec<(NodeId, f32)> = nodes.iter()
            .map(|n| {
                let weight = weights.get(n).copied().unwrap_or(0.5);
                (n.clone(), weight)
            })
            .collect();
        
        // 按权重排序
        ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        ranked.into_iter()
            .take(query.limit.min(10))
            .map(|(n, _)| n)
            .collect()
    }
    
    /// 获取所有节点
    pub async fn all_nodes(&self) -> Vec<NodeId> {
        self.known_nodes.read().await.iter().cloned().collect()
    }
}

// ============================================================================
// 查询执行器
// ============================================================================

/// 查询执行器 Trait
#[async_trait]
pub trait QueryExecutor: Send + Sync {
    /// 执行本地查询
    async fn execute_local(&self, query: &FederatedQuery) -> QueryResult<Vec<HybridSearchResult>>;
    
    /// 执行远程查询
    async fn execute_remote(
        &self,
        query: &FederatedQuery,
        target: &NodeId,
    ) -> QueryResult<Vec<HybridSearchResult>>;
}

/// 查询执行器实现
pub struct DefaultQueryExecutor {
    router: Arc<QueryRouter>,
    local_node_id: NodeId,
    event_tx: broadcast::Sender<QueryEvent>,
}

/// 查询事件
#[derive(Debug, Clone)]
pub enum QueryEvent {
    QueryStarted { query_id: String },
    QueryCompleted { query_id: String, elapsed_ms: u64 },
    NodeQueryStarted { query_id: String, node: NodeId },
    NodeQueryCompleted { query_id: String, node: NodeId, result_count: usize },
    NodeQueryFailed { query_id: String, node: NodeId, error: String },
}

impl DefaultQueryExecutor {
    pub fn new(router: Arc<QueryRouter>, local_node_id: NodeId) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self {
            router,
            local_node_id,
            event_tx,
        }
    }
    
    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<QueryEvent> {
        self.event_tx.subscribe()
    }
}

#[async_trait]
impl QueryExecutor for DefaultQueryExecutor {
    async fn execute_local(&self, query: &FederatedQuery) -> QueryResult<Vec<HybridSearchResult>> {
        // 实际实现中应该调用本地存储
        Ok(Vec::new())
    }
    
    async fn execute_remote(
        &self,
        query: &FederatedQuery,
        _target: &NodeId,
    ) -> QueryResult<Vec<HybridSearchResult>> {
        // 实际实现中应该通过网络发送查询请求
        Ok(Vec::new())
    }
}

// ============================================================================
// 联邦查询引擎
// ============================================================================

/// 联邦查询引擎
pub struct FederatedQueryEngine {
    /// 查询路由器
    router: Arc<QueryRouter>,
    /// 查询执行器
    executor: Arc<dyn QueryExecutor>,
    /// 本地节点 ID
    local_node_id: NodeId,
    /// 查询超时
    default_timeout: Duration,
}

impl FederatedQueryEngine {
    pub fn new(
        router: Arc<QueryRouter>,
        executor: Arc<dyn QueryExecutor>,
        local_node_id: NodeId,
    ) -> Self {
        Self {
            router,
            executor,
            local_node_id,
            default_timeout: Duration::from_secs(30),
        }
    }
    
    /// 执行联邦查询
    pub async fn execute(&self, query: FederatedQuery) -> QueryResult<FederatedQueryResponse> {
        let start = Instant::now();
        let query_id = query.id.clone();
        
        // 获取目标节点
        let targets = self.router.get_query_targets(&query).await;
        
        if targets.is_empty() {
            return Err(QueryError::NodeUnavailable("No available nodes".to_string()));
        }
        
        let mut node_responses = Vec::new();
        let mut all_results: Vec<(NodeId, HybridSearchResult)> = Vec::new();
        let mut successful_nodes = 0;
        let mut failed_nodes = 0;
        
        // 执行查询（并发）
        for target in targets {
            let node_start = Instant::now();
            
            let result = if target == self.local_node_id {
                self.executor.execute_local(&query).await
            } else {
                self.executor.execute_remote(&query, &target).await
            };
            
            match result {
                Ok(results) => {
                    let response_time = node_start.elapsed();
                    
                    // 更新节点延迟
                    self.router.update_latency(&target, response_time).await;
                    
                    node_responses.push(NodeQueryResponse {
                        node_id: target.clone(),
                        success: true,
                        result_count: results.len(),
                        response_time_ms: response_time.as_millis() as u64,
                        error: None,
                    });
                    
                    for result in results {
                        all_results.push((target.clone(), result));
                    }
                    
                    successful_nodes += 1;
                }
                Err(e) => {
                    node_responses.push(NodeQueryResponse {
                        node_id: target,
                        success: false,
                        result_count: 0,
                        response_time_ms: node_start.elapsed().as_millis() as u64,
                        error: Some(e.to_string()),
                    });
                    
                    failed_nodes += 1;
                }
            }
        }
        
        // 检查最小节点数
        if successful_nodes < query.min_nodes {
            return Err(QueryError::AggregationError(
                format!("Insufficient nodes: {} < {}", successful_nodes, query.min_nodes)
            ));
        }
        
        // 聚合结果
        let aggregated = self.aggregate_results(all_results, &query).await?;
        
        let elapsed = start.elapsed();
        
        Ok(FederatedQueryResponse {
            query_id,
            results: aggregated.results,
            total: aggregated.total,
            successful_nodes,
            failed_nodes,
            elapsed_ms: elapsed.as_millis() as u64,
            node_responses,
        })
    }
    
    /// 聚合结果
    async fn aggregate_results(
        &self,
        results: Vec<(NodeId, HybridSearchResult)>,
        query: &FederatedQuery,
    ) -> QueryResult<AggregationResult> {
        // 按条目 ID 分组
        let mut grouped: HashMap<String, Vec<(NodeId, HybridSearchResult)>> = HashMap::new();
        
        for (node, result) in results {
            grouped.entry(result.id.clone())
                .or_insert_with(Vec::new)
                .push((node, result));
        }
        
        // 合并分数
        let mut aggregated: Vec<AggregatedResult> = grouped.into_iter()
            .map(|(id, entries)| {
                // 计算平均分数
                let avg_score = entries.iter()
                    .map(|(_, r)| r.final_score)
                    .sum::<f32>() / entries.len() as f32;
                
                // 收集来源节点
                let source_nodes: Vec<NodeId> = entries.iter()
                    .map(|(n, _)| n.clone())
                    .collect();
                
                // 使用第一个条目的详细信息
                let (_, first) = &entries[0];
                
                AggregatedResult {
                    id,
                    content: first.content.clone(),
                    score: avg_score,
                    importance: first.importance,
                    source_nodes,
                    created_at: chrono::DateTime::parse_from_rfc3339(&first.created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    memory_type: "fact".to_string(),
                    tags: vec![],
                }
            })
            .collect();
        
        // 按分数排序
        aggregated.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        let total = aggregated.len();
        
        // 限制结果数量
        aggregated.truncate(query.limit);
        
        Ok(AggregationResult {
            results: aggregated,
            total,
        })
    }
    
    /// 快速搜索（本地优先）
    pub async fn quick_search(&self, query: &str, limit: usize) -> QueryResult<Vec<AggregatedResult>> {
        let query = FederatedQuery::new(query.to_string())
            .with_limit(limit)
            .with_target_nodes(vec![self.local_node_id.clone()]);
        
        let response = self.execute(query).await?;
        Ok(response.results)
    }
    
    /// 广播搜索（所有节点）
    pub async fn broadcast_search(&self, query: &str, limit: usize) -> QueryResult<Vec<AggregatedResult>> {
        let nodes = self.router.all_nodes().await;
        
        let query = FederatedQuery::new(query.to_string())
            .with_limit(limit)
            .with_target_nodes(nodes)
            .quorum(1);
        
        let response = self.execute(query).await?;
        Ok(response.results)
    }
}

/// 聚合结果
#[derive(Debug, Clone)]
struct AggregationResult {
    results: Vec<AggregatedResult>,
    total: usize,
}

// ============================================================================
// 查询缓存
// ============================================================================

/// 查询缓存
pub struct QueryCache {
    cache: RwLock<HashMap<String, CachedQuery>>,
    max_size: usize,
    ttl: Duration,
}

/// 缓存的查询
#[derive(Debug, Clone)]
struct CachedQuery {
    response: FederatedQueryResponse,
    cached_at: Instant,
}

impl QueryCache {
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            max_size,
            ttl,
        }
    }
    
    /// 计算查询哈希
    pub fn hash(query: &FederatedQuery) -> String {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(query.query.as_bytes());
        hasher.update(query.limit.to_le_bytes());
        hasher.update(query.offset.to_le_bytes());
        hasher.update(&format!("{:?}", query.filters));
        
        format!("{:x}", hasher.finalize())
    }
    
    /// 获取缓存
    pub async fn get(&self, query: &FederatedQuery) -> Option<FederatedQueryResponse> {
        let hash = Self::hash(query);
        let cache = self.cache.read().await;
        
        if let Some(cached) = cache.get(&hash) {
            if cached.cached_at.elapsed() < self.ttl {
                return Some(cached.response.clone());
            }
        }
        
        None
    }
    
    /// 存储缓存
    pub async fn put(&self, query: &FederatedQuery, response: &FederatedQueryResponse) {
        let hash = Self::hash(query);
        let mut cache = self.cache.write().await;
        
        // LRU 淘汰
        if cache.len() >= self.max_size {
            // 移除最旧的
            if let Some(oldest_key) = cache.iter()
                .min_by_key(|(_, v)| v.cached_at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }
        
        cache.insert(hash, CachedQuery {
            response: response.clone(),
            cached_at: Instant::now(),
        });
    }
    
    /// 清空缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_federated_query() {
        let query = FederatedQuery::new("test query".to_string())
            .with_limit(10)
            .with_offset(0)
            .with_timeout(5000);
        
        assert_eq!(query.query, "test query");
        assert_eq!(query.limit, 10);
        assert_eq!(query.timeout_ms, 5000);
    }
    
    #[test]
    fn test_query_filters() {
        let filters = QueryFilters {
            memory_type: Some("fact".to_string()),
            min_importance: Some(0.5),
            tags: vec!["important".to_string()],
            ..Default::default()
        };
        
        assert_eq!(filters.memory_type, Some("fact".to_string()));
        assert_eq!(filters.min_importance, Some(0.5));
    }
    
    #[tokio::test]
    async fn test_query_router() {
        let router = QueryRouter::new();
        
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        
        router.add_node(node1.clone()).await;
        router.add_node(node2.clone()).await;
        
        let nodes = router.all_nodes().await;
        assert_eq!(nodes.len(), 2);
        
        router.update_latency(&node1, Duration::from_millis(10)).await;
        router.update_latency(&node2, Duration::from_millis(100)).await;
        
        let query = FederatedQuery::new("test".to_string()).with_limit(10);
        let targets = router.get_query_targets(&query).await;
        
        // node1 延迟更低，应该排在前面
        assert_eq!(targets[0], node1);
    }
    
    #[tokio::test]
    async fn test_query_cache() {
        let cache = QueryCache::new(100, Duration::from_secs(60));
        
        let query = FederatedQuery::new("test".to_string());
        let response = FederatedQueryResponse {
            query_id: query.id.clone(),
            results: vec![],
            total: 0,
            successful_nodes: 1,
            failed_nodes: 0,
            elapsed_ms: 10,
            node_responses: vec![],
        };
        
        cache.put(&query, &response).await;
        
        let cached = cache.get(&query).await;
        assert!(cached.is_some());
    }
    
    #[test]
    fn test_aggregated_result() {
        let result = AggregatedResult {
            id: "test-1".to_string(),
            content: "Test content".to_string(),
            score: 0.95,
            importance: 0.8,
            source_nodes: vec![NodeId::new()],
            created_at: Utc::now(),
            memory_type: "fact".to_string(),
            tags: vec![],
        };
        
        assert_eq!(result.id, "test-1");
        assert!((result.score - 0.95).abs() < 0.01);
    }
    
    #[test]
    fn test_time_range() {
        let start = Utc::now() - chrono::Duration::days(7);
        let end = Utc::now();
        
        let range = TimeRange { start, end };
        
        assert!(range.start < range.end);
    }
}