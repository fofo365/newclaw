//! Federated Memory Aggregation - 联邦记忆结果聚合
//!
//! 提供查询结果的聚合、去重、排序功能
//! 支持分数融合、多样性优化
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::protocol::NodeId;
use super::query::{AggregatedResult, FederatedQueryResponse, NodeQueryResponse};
use crate::memory::{HybridSearchResult, MMRConfig, mmr_diversify};

// ============================================================================
// 聚合配置
// ============================================================================

/// 聚合配置
#[derive(Debug, Clone)]
pub struct AggregationConfig {
    /// 分数融合策略
    pub fusion_strategy: FusionStrategy,
    /// 去重策略
    pub dedup_strategy: DeduplicationStrategy,
    /// 排序策略
    pub sort_strategy: SortStrategy,
    /// 是否应用多样性
    pub apply_diversity: bool,
    /// MMR 配置
    pub mmr_config: MMRConfig,
    /// 是否加权聚合
    pub weighted_aggregation: bool,
    /// 节点权重（节点 ID -> 权重）
    pub node_weights: HashMap<String, f32>,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            fusion_strategy: FusionStrategy::Average,
            dedup_strategy: DeduplicationStrategy::ContentSimilarity,
            sort_strategy: SortStrategy::ScoreDescending,
            apply_diversity: true,
            mmr_config: MMRConfig::default(),
            weighted_aggregation: false,
            node_weights: HashMap::new(),
        }
    }
}

// ============================================================================
// 策略枚举
// ============================================================================

/// 分数融合策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FusionStrategy {
    /// 平均分数
    #[default]
    Average,
    /// 最大分数
    Max,
    /// 最小分数
    Min,
    /// 中位数分数
    Median,
    /// 加权平均
    WeightedAverage,
    /// Reciprocal Rank Fusion (RRF)
    ReciprocalRankFusion,
}

/// 去重策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeduplicationStrategy {
    /// 不去重
    None,
    /// 按 ID 去重
    #[default]
    ById,
    /// 按内容相似度去重
    ContentSimilarity,
    /// 按内容哈希去重
    ContentHash,
    /// 混合去重（ID + 相似度）
    Hybrid,
}

/// 排序策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortStrategy {
    /// 按分数降序
    #[default]
    ScoreDescending,
    /// 按分数升序
    ScoreAscending,
    /// 按时间降序
    TimeDescending,
    /// 按时间升序
    TimeAscending,
    /// 按重要性降序
    ImportanceDescending,
    /// 混合排序（分数 + 时间）
    Mixed,
}

// ============================================================================
// 聚合器
// ============================================================================

/// 结果聚合器
pub struct ResultAggregator {
    config: AggregationConfig,
}

impl ResultAggregator {
    pub fn new(config: AggregationConfig) -> Self {
        Self { config }
    }
    
    /// 聚合多节点结果
    pub fn aggregate(
        &self,
        node_results: HashMap<NodeId, Vec<HybridSearchResult>>,
    ) -> AggregationOutput {
        // 1. 收集所有结果
        let all_results = self.collect_results(&node_results);
        
        // 2. 融合分数
        let fused = self.fuse_scores(all_results);
        
        // 3. 去重
        let deduped = self.deduplicate(fused);
        
        // 4. 排序
        let sorted = self.sort_results(deduped);
        
        // 5. 应用多样性
        let diversified = if self.config.apply_diversity {
            self.apply_diversity(sorted)
        } else {
            sorted
        };
        
        AggregationOutput {
            results: diversified,
            total_nodes: node_results.len(),
            total_raw_results: node_results.values().map(|r| r.len()).sum(),
        }
    }
    
    /// 收集结果
    fn collect_results(
        &self,
        node_results: &HashMap<NodeId, Vec<HybridSearchResult>>,
    ) -> Vec<(NodeId, HybridSearchResult)> {
        let mut results = Vec::new();
        
        for (node, entries) in node_results {
            for entry in entries {
                results.push((node.clone(), entry.clone()));
            }
        }
        
        results
    }
    
    /// 融合分数
    fn fuse_scores(
        &self,
        results: Vec<(NodeId, HybridSearchResult)>,
    ) -> Vec<FusedResult> {
        // 按 ID 分组
        let mut grouped: HashMap<String, Vec<(NodeId, HybridSearchResult)>> = HashMap::new();
        
        for (node, result) in results {
            grouped.entry(result.id.clone())
                .or_default()
                .push((node, result));
        }
        
        // 对每个组融合分数
        grouped.into_iter()
            .map(|(id, entries)| {
                let fused_score = self.compute_fused_score(&entries);
                let (_, first) = &entries[0];
                
                FusedResult {
                    id: id.clone(),
                    content: first.content.clone(),
                    score: fused_score,
                    importance: first.importance,
                    created_at: first.created_at.clone(),
                    source_nodes: entries.iter().map(|(n, _)| n.clone()).collect(),
                    bm25_scores: entries.iter().map(|(_, r)| r.bm25_score).collect(),
                    vector_scores: entries.iter().map(|(_, r)| r.vector_score).collect(),
                }
            })
            .collect()
    }
    
    /// 计算融合分数
    fn compute_fused_score(&self, entries: &[(NodeId, HybridSearchResult)]) -> f32 {
        if entries.is_empty() {
            return 0.0;
        }
        
        match self.config.fusion_strategy {
            FusionStrategy::Average => {
                let sum: f32 = entries.iter().map(|(_, r)| r.final_score).sum();
                sum / entries.len() as f32
            }
            
            FusionStrategy::Max => {
                entries.iter().map(|(_, r)| r.final_score).fold(0.0, f32::max)
            }
            
            FusionStrategy::Min => {
                entries.iter().map(|(_, r)| r.final_score).fold(f32::MAX, f32::min)
            }
            
            FusionStrategy::Median => {
                let mut scores: Vec<f32> = entries.iter().map(|(_, r)| r.final_score).collect();
                scores.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                
                let mid = scores.len() / 2;
                if scores.len().is_multiple_of(2) {
                    (scores[mid - 1] + scores[mid]) / 2.0
                } else {
                    scores[mid]
                }
            }
            
            FusionStrategy::WeightedAverage => {
                let mut total_weight = 0.0;
                let mut weighted_sum = 0.0;
                
                for (node, result) in entries {
                    let weight = self.config.node_weights
                        .get(&node.to_string())
                        .copied()
                        .unwrap_or(1.0);
                    
                    weighted_sum += result.final_score * weight;
                    total_weight += weight;
                }
                
                if total_weight > 0.0 {
                    weighted_sum / total_weight
                } else {
                    0.0
                }
            }
            
            FusionStrategy::ReciprocalRankFusion => {
                let k = 60.0; // RRF 常数
                
                // 计算每个结果的排名
                let mut scores: Vec<(usize, f32)> = entries.iter()
                    .enumerate()
                    .map(|(i, (_, r))| (i, r.final_score))
                    .collect();
                
                scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                
                // RRF 分数
                let rrf_score: f32 = scores.iter()
                    .enumerate()
                    .map(|(rank, _)| 1.0 / (k + rank as f32 + 1.0))
                    .sum();
                
                rrf_score
            }
        }
    }
    
    /// 去重
    fn deduplicate(&self, results: Vec<FusedResult>) -> Vec<FusedResult> {
        match self.config.dedup_strategy {
            DeduplicationStrategy::None => results,
            
            DeduplicationStrategy::ById => {
                let mut seen = HashSet::new();
                results.into_iter()
                    .filter(|r| seen.insert(r.id.clone()))
                    .collect()
            }
            
            DeduplicationStrategy::ContentHash => {
                let mut seen = HashSet::new();
                results.into_iter()
                    .filter(|r| {
                        let hash = self.content_hash(&r.content);
                        seen.insert(hash)
                    })
                    .collect()
            }
            
            DeduplicationStrategy::ContentSimilarity => {
                // 使用相似度阈值去重
                let mut deduped: Vec<FusedResult> = Vec::new();
                
                for result in results {
                    let mut is_duplicate = false;
                    
                    for existing in &deduped {
                        let similarity = self.content_similarity(&result.content, &existing.content);
                        if similarity > 0.9 {
                            is_duplicate = true;
                            break;
                        }
                    }
                    
                    if !is_duplicate {
                        deduped.push(result);
                    }
                }
                
                deduped
            }
            
            DeduplicationStrategy::Hybrid => {
                // 先按 ID 去重，再按相似度去重
                let mut by_id: HashMap<String, FusedResult> = HashMap::new();
                
                for result in results {
                    by_id.entry(result.id.clone())
                        .and_modify(|existing| {
                            if result.score > existing.score {
                                *existing = result.clone();
                            }
                        })
                        .or_insert(result);
                }
                
                let results: Vec<FusedResult> = by_id.into_values().collect();
                
                // 再按相似度去重
                let mut deduped: Vec<FusedResult> = Vec::new();
                
                for result in results {
                    let mut is_duplicate = false;
                    
                    for existing in &deduped {
                        let similarity = self.content_similarity(&result.content, &existing.content);
                        if similarity > 0.9 && result.id != existing.id {
                            is_duplicate = true;
                            break;
                        }
                    }
                    
                    if !is_duplicate {
                        deduped.push(result);
                    }
                }
                
                deduped
            }
        }
    }
    
    /// 排序
    fn sort_results(&self, mut results: Vec<FusedResult>) -> Vec<FusedResult> {
        match self.config.sort_strategy {
            SortStrategy::ScoreDescending => {
                results.sort_by(|a, b| {
                    b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            
            SortStrategy::ScoreAscending => {
                results.sort_by(|a, b| {
                    a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            
            SortStrategy::TimeDescending => {
                results.sort_by(|a, b| {
                    b.created_at.cmp(&a.created_at)
                });
            }
            
            SortStrategy::TimeAscending => {
                results.sort_by(|a, b| {
                    a.created_at.cmp(&b.created_at)
                });
            }
            
            SortStrategy::ImportanceDescending => {
                results.sort_by(|a, b| {
                    b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
            
            SortStrategy::Mixed => {
                // 综合分数 = 0.7 * 标准化分数 + 0.2 * 标准化重要性 + 0.1 * 时间衰减
                results.sort_by(|a, b| {
                    let a_combined = a.score * 0.7 + a.importance * 0.2;
                    let b_combined = b.score * 0.7 + b.importance * 0.2;
                    b_combined.partial_cmp(&a_combined).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
        
        results
    }
    
    /// 应用多样性
    fn apply_diversity(&self, results: Vec<FusedResult>) -> Vec<FusedResult> {
        if results.len() <= 1 {
            return results;
        }
        
        let mmr_config = &self.config.mmr_config;
        
        // 转换为 HybridSearchResult 进行 MMR
        let hybrid_results: Vec<HybridSearchResult> = results.iter()
            .map(|r| HybridSearchResult {
                id: r.id.clone(),
                content: r.content.clone(),
                bm25_score: r.score,
                vector_score: 0.0,
                final_score: r.score,
                importance: r.importance,
                created_at: r.created_at.clone(),
            })
            .collect();
        
        // 应用 MMR
        let diversified = mmr_diversify(hybrid_results, mmr_config);
        
        // 转换回来
        let mut result_map: HashMap<String, FusedResult> = results.into_iter()
            .map(|r| (r.id.clone(), r))
            .collect();
        
        diversified.into_iter()
            .filter_map(|hr| result_map.remove(&hr.id))
            .collect()
    }
    
    /// 计算内容哈希
    fn content_hash(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// 计算内容相似度
    fn content_similarity(&self, a: &str, b: &str) -> f32 {
        use std::collections::HashSet;
        
        let words_a: HashSet<String> = a.split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        
        let words_b: HashSet<String> = b.split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();
        
        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }
        
        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
}

/// 融合后的结果
#[derive(Debug, Clone)]
pub struct FusedResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub importance: f32,
    pub created_at: String,
    pub source_nodes: Vec<NodeId>,
    pub bm25_scores: Vec<f32>,
    pub vector_scores: Vec<f32>,
}

/// 聚合输出
#[derive(Debug, Clone)]
pub struct AggregationOutput {
    /// 聚合后的结果
    pub results: Vec<FusedResult>,
    /// 涉及的节点数
    pub total_nodes: usize,
    /// 原始结果总数
    pub total_raw_results: usize,
}

// ============================================================================
// 分数归一化
// ============================================================================

/// 分数归一化器
pub struct ScoreNormalizer;

impl ScoreNormalizer {
    /// Min-Max 归一化
    pub fn min_max(scores: &[f32]) -> Vec<f32> {
        if scores.is_empty() {
            return Vec::new();
        }
        
        let min = scores.iter().fold(f32::MAX, |a, &b| a.min(b));
        let max = scores.iter().fold(f32::MIN, |a, &b| a.max(b));
        
        if (max - min).abs() < f32::EPSILON {
            return vec![0.5; scores.len()];
        }
        
        scores.iter().map(|s| (s - min) / (max - min)).collect()
    }
    
    /// Z-Score 归一化
    pub fn z_score(scores: &[f32]) -> Vec<f32> {
        if scores.is_empty() {
            return Vec::new();
        }
        
        let n = scores.len() as f32;
        let mean: f32 = scores.iter().sum::<f32>() / n;
        
        let variance: f32 = scores.iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f32>() / n;
        
        let std_dev = variance.sqrt();
        
        if std_dev.abs() < f32::EPSILON {
            return vec![0.0; scores.len()];
        }
        
        scores.iter().map(|s| (s - mean) / std_dev).collect()
    }
    
    /// Softmax 归一化
    pub fn softmax(scores: &[f32]) -> Vec<f32> {
        if scores.is_empty() {
            return Vec::new();
        }
        
        // 数值稳定性
        let max = scores.iter().fold(f32::MIN, |a, &b| a.max(b));
        let exp_sum: f32 = scores.iter().map(|s| (s - max).exp()).sum();
        
        scores.iter().map(|s| (s - max).exp() / exp_sum).collect()
    }
}

// ============================================================================
// 结果合并器
// ============================================================================

/// 结果合并器
pub struct ResultMerger {
    config: AggregationConfig,
}

impl ResultMerger {
    pub fn new(config: AggregationConfig) -> Self {
        Self { config }
    }
    
    /// 合并两个结果集
    pub fn merge(
        &self,
        left: Vec<AggregatedResult>,
        right: Vec<AggregatedResult>,
    ) -> Vec<AggregatedResult> {
        let mut merged: HashMap<String, AggregatedResult> = HashMap::new();
        
        // 添加左侧结果
        for result in left {
            merged.insert(result.id.clone(), result);
        }
        
        // 合并右侧结果
        for result in right {
            merged.entry(result.id.clone())
                .and_modify(|existing| {
                    // 合并来源节点
                    existing.source_nodes.extend(result.source_nodes.clone());
                    existing.source_nodes.sort();
                    existing.source_nodes.dedup();
                    
                    // 更新分数（取最大）
                    if result.score > existing.score {
                        existing.score = result.score;
                    }
                })
                .or_insert(result);
        }
        
        // 转换并排序
        let mut results: Vec<AggregatedResult> = merged.into_values().collect();
        
        if self.config.sort_strategy == SortStrategy::ScoreDescending {
            results.sort_by(|a, b| {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        
        results
    }
    
    /// 交集
    pub fn intersect(
        &self,
        left: Vec<AggregatedResult>,
        right: Vec<AggregatedResult>,
    ) -> Vec<AggregatedResult> {
        let right_ids: HashSet<String> = right.iter().map(|r| r.id.clone()).collect();
        
        left.into_iter()
            .filter(|r| right_ids.contains(&r.id))
            .collect()
    }
    
    /// 差集
    pub fn difference(
        &self,
        left: Vec<AggregatedResult>,
        right: Vec<AggregatedResult>,
    ) -> Vec<AggregatedResult> {
        let right_ids: HashSet<String> = right.iter().map(|r| r.id.clone()).collect();
        
        left.into_iter()
            .filter(|r| !right_ids.contains(&r.id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_result(id: &str, score: f32, content: &str) -> HybridSearchResult {
        HybridSearchResult {
            id: id.to_string(),
            content: content.to_string(),
            bm25_score: score,
            vector_score: 0.0,
            final_score: score,
            importance: 0.5,
            created_at: "2026-03-14".to_string(),
        }
    }
    
    #[test]
    fn test_fusion_strategy_average() {
        let config = AggregationConfig {
            fusion_strategy: FusionStrategy::Average,
            ..Default::default()
        };
        let aggregator = ResultAggregator::new(config);
        
        let mut node_results: HashMap<NodeId, Vec<HybridSearchResult>> = HashMap::new();
        let node = NodeId::new();
        
        node_results.insert(node, vec![
            create_test_result("1", 0.8, "Content 1"),
            create_test_result("1", 0.6, "Content 1"),
        ]);
        
        let output = aggregator.aggregate(node_results);
        
        assert_eq!(output.results.len(), 1);
        assert!((output.results[0].score - 0.7).abs() < 0.01);
    }
    
    #[test]
    fn test_fusion_strategy_max() {
        let config = AggregationConfig {
            fusion_strategy: FusionStrategy::Max,
            ..Default::default()
        };
        let aggregator = ResultAggregator::new(config);
        
        let mut node_results: HashMap<NodeId, Vec<HybridSearchResult>> = HashMap::new();
        let node = NodeId::new();
        
        node_results.insert(node, vec![
            create_test_result("1", 0.8, "Content 1"),
            create_test_result("1", 0.6, "Content 1"),
        ]);
        
        let output = aggregator.aggregate(node_results);
        
        assert!((output.results[0].score - 0.8).abs() < 0.01);
    }
    
    #[test]
    fn test_deduplication_by_id() {
        let config = AggregationConfig {
            dedup_strategy: DeduplicationStrategy::ById,
            ..Default::default()
        };
        let aggregator = ResultAggregator::new(config);
        
        let mut node_results: HashMap<NodeId, Vec<HybridSearchResult>> = HashMap::new();
        let node1 = NodeId::new();
        let node2 = NodeId::new();
        
        node_results.insert(node1, vec![create_test_result("1", 0.8, "Content 1")]);
        node_results.insert(node2, vec![create_test_result("1", 0.6, "Content 1")]);
        
        let output = aggregator.aggregate(node_results);
        
        assert_eq!(output.results.len(), 1);
    }
    
    #[test]
    fn test_sort_score_descending() {
        let config = AggregationConfig {
            sort_strategy: SortStrategy::ScoreDescending,
            apply_diversity: false, // 禁用多样性以测试排序
            ..Default::default()
        };
        let aggregator = ResultAggregator::new(config);
        
        let mut node_results: HashMap<NodeId, Vec<HybridSearchResult>> = HashMap::new();
        let node = NodeId::new();
        
        node_results.insert(node, vec![
            create_test_result("1", 0.5, "Content 1"),
            create_test_result("2", 0.9, "Content 2"),
            create_test_result("3", 0.7, "Content 3"),
        ]);
        
        let output = aggregator.aggregate(node_results);
        
        assert_eq!(output.results.len(), 3);
        assert_eq!(output.results[0].id, "2");
        assert_eq!(output.results[1].id, "3");
        assert_eq!(output.results[2].id, "1");
    }
    
    #[test]
    fn test_score_normalizer_min_max() {
        let scores = vec![0.0, 0.5, 1.0];
        let normalized = ScoreNormalizer::min_max(&scores);
        
        assert!((normalized[0] - 0.0).abs() < 0.01);
        assert!((normalized[1] - 0.5).abs() < 0.01);
        assert!((normalized[2] - 1.0).abs() < 0.01);
    }
    
    #[test]
    fn test_result_merger() {
        let config = AggregationConfig::default();
        let merger = ResultMerger::new(config);
        
        let left = vec![
            AggregatedResult {
                id: "1".to_string(),
                content: "Content 1".to_string(),
                score: 0.8,
                importance: 0.5,
                source_nodes: vec![NodeId::new()],
                created_at: Utc::now(),
                memory_type: "fact".to_string(),
                tags: vec![],
            },
        ];
        
        let right = vec![
            AggregatedResult {
                id: "1".to_string(),
                content: "Content 1".to_string(),
                score: 0.9,
                importance: 0.5,
                source_nodes: vec![NodeId::new()],
                created_at: Utc::now(),
                memory_type: "fact".to_string(),
                tags: vec![],
            },
        ];
        
        let merged = merger.merge(left, right);
        
        assert_eq!(merged.len(), 1);
        assert!((merged[0].score - 0.9).abs() < 0.01);
    }
    
    #[test]
    fn test_content_similarity() {
        let config = AggregationConfig::default();
        let aggregator = ResultAggregator::new(config);
        
        let sim = aggregator.content_similarity(
            "the quick brown fox",
            "the quick brown fox"
        );
        assert!((sim - 1.0).abs() < 0.01);
        
        let sim2 = aggregator.content_similarity(
            "the quick brown fox",
            "hello world"
        );
        assert!(sim2 < 0.5);
    }
}