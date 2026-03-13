// Hierarchical Summary - 分层摘要系统
//
// v0.7.0 - 实现分层摘要树，支持长对话的高效压缩

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::{Result, Context};
use tokio::sync::RwLock;

/// 分层摘要树
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryTree {
    /// 根节点
    pub root: Option<SummaryNode>,
    /// 所有节点索引（ID -> Node）
    pub nodes: HashMap<String, SummaryNode>,
    /// 配置
    pub config: SummaryConfig,
    /// 统计
    pub stats: SummaryStats,
}

/// 摘要节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryNode {
    /// 节点 ID
    pub id: String,
    /// 摘要内容
    pub summary: String,
    /// 关键点
    pub key_points: Vec<String>,
    /// 决策记录
    pub decisions: Vec<String>,
    /// 子节点 ID
    pub children: Vec<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// Token 数量
    pub token_count: usize,
    /// 层级
    pub level: u8,
    /// 原始消息范围
    pub message_range: Option<(usize, usize)>,
    /// 重要性评分
    pub importance: f32,
}

/// 摘要配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryConfig {
    /// 一级摘要触发消息数
    pub level1_threshold: usize,
    /// 二级摘要触发一级摘要数
    pub level2_threshold: usize,
    /// 三级摘要触发二级摘要数
    pub level3_threshold: usize,
    /// 最大层级
    pub max_levels: u8,
    /// 最大 Token 数
    pub max_tokens: usize,
    /// 是否保留关键决策
    pub preserve_decisions: bool,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            level1_threshold: 10,  // 每 10 条消息生成一级摘要
            level2_threshold: 10,  // 每 10 个一级摘要生成二级摘要
            level3_threshold: 10,  // 每 10 个二级摘要生成三级摘要
            max_levels: 3,
            max_tokens: 4000,
            preserve_decisions: true,
        }
    }
}

/// 摘要统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummaryStats {
    /// 总消息数
    pub total_messages: usize,
    /// 一级摘要数
    pub level1_count: usize,
    /// 二级摘要数
    pub level2_count: usize,
    /// 三级摘要数
    pub level3_count: usize,
    /// 最后更新时间
    pub last_updated: Option<DateTime<Utc>>,
}

/// 摘要动作
#[derive(Debug, Clone)]
pub enum SummaryAction {
    /// 无需操作
    None,
    /// 创建一级摘要
    CreateLevel1(SummaryNode),
    /// 创建二级摘要
    CreateLevel2(SummaryNode),
    /// 创建三级摘要
    CreateLevel3(SummaryNode),
}

/// 分层摘要管理器
pub struct HierarchicalSummaryManager {
    /// 摘要树
    tree: Arc<RwLock<SummaryTree>>,
    /// 待摘要的消息缓冲
    message_buffer: Arc<RwLock<Vec<SummaryMessage>>>,
}

/// 摘要消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryMessage {
    /// 消息 ID
    pub id: String,
    /// 角色
    pub role: String,
    /// 内容
    pub content: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 是否包含决策
    pub has_decision: bool,
    /// 关键点
    pub key_points: Vec<String>,
}

impl SummaryTree {
    /// 创建新的摘要树
    pub fn new(config: SummaryConfig) -> Self {
        Self {
            root: None,
            nodes: HashMap::new(),
            config,
            stats: SummaryStats::default(),
        }
    }
    
    /// 添加节点
    pub fn add_node(&mut self, node: SummaryNode) {
        if self.root.is_none() && node.level == 3 {
            self.root = Some(node.clone());
        }
        self.nodes.insert(node.id.clone(), node);
        self.stats.last_updated = Some(Utc::now());
    }
    
    /// 获取节点
    pub fn get_node(&self, id: &str) -> Option<&SummaryNode> {
        self.nodes.get(id)
    }
    
    /// 检索相关摘要
    pub fn retrieve(&self, query: &str, max_tokens: usize) -> Vec<&SummaryNode> {
        let mut results = Vec::new();
        let mut total_tokens = 0;
        
        // 从高层到低层检索
        for level in (1..=self.config.max_levels).rev() {
            let level_nodes: Vec<&SummaryNode> = self.nodes.values()
                .filter(|n| n.level == level)
                .filter(|n| self.is_relevant(n, query))
                .collect();
            
            for node in level_nodes {
                if total_tokens + node.token_count > max_tokens {
                    return results;
                }
                results.push(node);
                total_tokens += node.token_count;
            }
        }
        
        results
    }
    
    /// 判断节点是否与查询相关
    fn is_relevant(&self, node: &SummaryNode, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        
        // 检查摘要内容
        if node.summary.to_lowercase().contains(&query_lower) {
            return true;
        }
        
        // 检查关键点
        for point in &node.key_points {
            if point.to_lowercase().contains(&query_lower) {
                return true;
            }
        }
        
        // 检查决策
        for decision in &node.decisions {
            if decision.to_lowercase().contains(&query_lower) {
                return true;
            }
        }
        
        false
    }
    
    /// 获取完整的上下文摘要
    pub fn get_full_summary(&self, max_tokens: usize) -> String {
        let mut result = String::new();
        let mut total_tokens = 0;
        
        // 按层级组织
        for level in (1..=self.config.max_levels).rev() {
            let level_nodes: Vec<&SummaryNode> = self.nodes.values()
                .filter(|n| n.level == level)
                .collect();
            
            if level_nodes.is_empty() {
                continue;
            }
            
            let level_prefix = match level {
                1 => "### 近期对话\n",
                2 => "## 中期摘要\n",
                3 => "# 历史摘要\n",
                _ => "",
            };
            
            result.push_str(level_prefix);
            
            for node in level_nodes {
                let node_text = format!("- {}\n", node.summary);
                let node_tokens = node_text.len() / 4; // 粗略估计
                
                if total_tokens + node_tokens > max_tokens {
                    return result;
                }
                
                result.push_str(&node_text);
                total_tokens += node_tokens;
            }
            
            result.push('\n');
        }
        
        result
    }
    
    /// 获取所有决策
    pub fn get_all_decisions(&self) -> Vec<String> {
        let mut decisions = Vec::new();
        
        for node in self.nodes.values() {
            decisions.extend(node.decisions.clone());
        }
        
        decisions
    }
}

impl SummaryNode {
    /// 创建新的摘要节点
    pub fn new(
        summary: String,
        key_points: Vec<String>,
        decisions: Vec<String>,
        level: u8,
    ) -> Self {
        let token_count = summary.len() / 4; // 粗略估计
        let importance = Self::calculate_importance(&key_points, &decisions);
        
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            summary,
            key_points,
            decisions,
            children: Vec::new(),
            created_at: Utc::now(),
            token_count,
            level,
            message_range: None,
            importance,
        }
    }
    
    /// 计算重要性评分
    fn calculate_importance(key_points: &[String], decisions: &[String]) -> f32 {
        let mut score = 0.5;
        
        // 有关键点加分
        score += (key_points.len() as f32) * 0.05;
        
        // 有决策加分
        score += (decisions.len() as f32) * 0.1;
        
        score.min(1.0)
    }
    
    /// 添加子节点
    pub fn add_child(&mut self, child_id: &str) {
        if !self.children.contains(&child_id.to_string()) {
            self.children.push(child_id.to_string());
        }
    }
}

impl HierarchicalSummaryManager {
    /// 创建新的管理器
    pub fn new(config: SummaryConfig) -> Self {
        Self {
            tree: Arc::new(RwLock::new(SummaryTree::new(config))),
            message_buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// 添加消息
    pub async fn add_message(&self, message: SummaryMessage) -> Result<SummaryAction> {
        let mut buffer = self.message_buffer.write().await;
        buffer.push(message);
        
        let mut tree = self.tree.write().await;
        tree.stats.total_messages += 1;
        
        // 检查是否需要创建摘要
        let level1_count = buffer.len() / tree.config.level1_threshold;
        
        if level1_count > tree.stats.level1_count {
            // 需要创建新的一级摘要
            let start = tree.stats.level1_count * tree.config.level1_threshold;
            let end = start + tree.config.level1_threshold;
            
            if end <= buffer.len() {
                let messages: Vec<&SummaryMessage> = buffer[start..end].iter().collect();
                let node = self.create_summary_node(messages, 1).await?;
                
                tree.stats.level1_count += 1;
                tree.add_node(node.clone());
                
                return Ok(SummaryAction::CreateLevel1(node));
            }
        }
        
        Ok(SummaryAction::None)
    }
    
    /// 创建摘要节点
    async fn create_summary_node(&self, messages: Vec<&SummaryMessage>, level: u8) -> Result<SummaryNode> {
        // 提取关键点
        let key_points: Vec<String> = messages.iter()
            .flat_map(|m| m.key_points.clone())
            .take(5)
            .collect();
        
        // 提取决策
        let decisions: Vec<String> = messages.iter()
            .filter(|m| m.has_decision)
            .map(|m| m.content.clone())
            .take(3)
            .collect();
        
        // 生成摘要（简化版本：取最后几条消息的关键内容）
        let summary = messages.iter()
            .rev()
            .take(3)
            .map(|m| m.content.clone())
            .collect::<Vec<_>>()
            .join(" -> ");
        
        let summary = if summary.len() > 200 {
            format!("{}...", &summary[..200])
        } else {
            summary
        };
        
        Ok(SummaryNode::new(summary, key_points, decisions, level))
    }
    
    /// 获取摘要树
    pub async fn get_tree(&self) -> SummaryTree {
        self.tree.read().await.clone()
    }
    
    /// 检索相关摘要
    pub async fn retrieve(&self, query: &str, max_tokens: usize) -> Vec<SummaryNode> {
        let tree = self.tree.read().await;
        tree.retrieve(query, max_tokens)
            .into_iter()
            .cloned()
            .collect()
    }
    
    /// 获取完整摘要
    pub async fn get_full_summary(&self, max_tokens: usize) -> String {
        let tree = self.tree.read().await;
        tree.get_full_summary(max_tokens)
    }
    
    /// 压缩上下文
    pub async fn compress_context(
        &self,
        messages: Vec<String>,
        max_tokens: usize,
    ) -> Result<Vec<String>> {
        let tree = self.tree.read().await;
        let mut result = Vec::new();
        let mut total_tokens = 0;
        
        // 先添加高层摘要
        let summary = tree.get_full_summary(max_tokens / 3);
        let summary_tokens = summary.len() / 4;
        
        if summary_tokens < max_tokens {
            result.push(format!("[对话摘要]\n{}", summary));
            total_tokens += summary_tokens;
        }
        
        // 再添加最近的消息
        for msg in messages.iter().rev() {
            let msg_tokens = msg.len() / 4;
            if total_tokens + msg_tokens > max_tokens {
                break;
            }
            result.push(msg.clone());
            total_tokens += msg_tokens;
        }
        
        // 反转顺序
        result.reverse();
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_summary_node_new() {
        let node = SummaryNode::new(
            "Test summary".to_string(),
            vec!["Point 1".to_string(), "Point 2".to_string()],
            vec!["Decision 1".to_string()],
            1,
        );
        
        assert!(!node.id.is_empty());
        assert_eq!(node.level, 1);
        assert_eq!(node.key_points.len(), 2);
        assert_eq!(node.decisions.len(), 1);
        assert!(node.importance > 0.5);
    }
    
    #[test]
    fn test_summary_tree_new() {
        let tree = SummaryTree::new(SummaryConfig::default());
        assert!(tree.root.is_none());
        assert!(tree.nodes.is_empty());
    }
    
    #[test]
    fn test_summary_tree_add_node() {
        let mut tree = SummaryTree::new(SummaryConfig::default());
        let node = SummaryNode::new("Test".to_string(), vec![], vec![], 1);
        
        tree.add_node(node.clone());
        
        assert!(tree.get_node(&node.id).is_some());
        assert!(tree.stats.last_updated.is_some());
    }
    
    #[test]
    fn test_summary_tree_retrieve() {
        let mut tree = SummaryTree::new(SummaryConfig::default());
        
        let node1 = SummaryNode::new(
            "Rust programming language".to_string(),
            vec!["systems programming".to_string()],
            vec![],
            1,
        );
        let node2 = SummaryNode::new(
            "Python data science".to_string(),
            vec!["machine learning".to_string()],
            vec![],
            1,
        );
        
        tree.add_node(node1);
        tree.add_node(node2);
        
        let results = tree.retrieve("Rust", 1000);
        assert!(!results.is_empty());
        assert!(results.iter().any(|n| n.summary.contains("Rust")));
    }
    
    #[test]
    fn test_summary_config_default() {
        let config = SummaryConfig::default();
        assert_eq!(config.level1_threshold, 10);
        assert_eq!(config.max_levels, 3);
        assert!(config.preserve_decisions);
    }
    
    #[tokio::test]
    async fn test_hierarchical_summary_manager() {
        let manager = HierarchicalSummaryManager::new(SummaryConfig {
            level1_threshold: 3,
            ..Default::default()
        });
        
        // 添加消息
        for i in 0..5 {
            let msg = SummaryMessage {
                id: format!("msg-{}", i),
                role: "user".to_string(),
                content: format!("Message {}", i),
                timestamp: Utc::now(),
                has_decision: i == 2,
                key_points: vec![format!("Key point {}", i)],
            };
            manager.add_message(msg).await.unwrap();
        }
        
        let tree = manager.get_tree().await;
        assert_eq!(tree.stats.total_messages, 5);
        assert!(tree.stats.level1_count > 0);
    }
    
    #[tokio::test]
    async fn test_get_full_summary() {
        let mut tree = SummaryTree::new(SummaryConfig::default());
        
        tree.add_node(SummaryNode::new("Recent chat 1".to_string(), vec![], vec![], 1));
        tree.add_node(SummaryNode::new("Summary 2".to_string(), vec![], vec![], 2));
        tree.add_node(SummaryNode::new("History 3".to_string(), vec![], vec![], 3));
        
        let summary = tree.get_full_summary(1000);
        assert!(summary.contains("近期对话"));
        assert!(summary.contains("中期摘要"));
        assert!(summary.contains("历史摘要"));
    }
}