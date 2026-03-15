// Context Retrieval - v0.5.2
//
// RAG (Retrieval-Augmented Generation) 实现

use serde::{Deserialize, Serialize};
use anyhow::Result;

/// 检索配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// 返回的最大结果数
    pub top_k: usize,
    /// 相似度阈值 (0.0 - 1.0)
    pub similarity_threshold: f32,
    /// 是否重排序
    pub rerank: bool,
    /// 是否多样化结果
    pub diversify: bool,
    /// 是否包含元数据
    pub include_metadata: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            similarity_threshold: 0.5,
            rerank: false,
            diversify: false,
            include_metadata: true,
        }
    }
}

impl RetrievalConfig {
    /// 创建严格配置（高阈值，少结果）
    pub fn strict() -> Self {
        Self {
            top_k: 3,
            similarity_threshold: 0.8,
            rerank: true,
            diversify: false,
            include_metadata: true,
        }
    }
    
    /// 创建宽松配置（低阈值，多结果）
    pub fn permissive() -> Self {
        Self {
            top_k: 10,
            similarity_threshold: 0.3,
            rerank: false,
            diversify: true,
            include_metadata: true,
        }
    }
}

/// 检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// 文本内容
    pub text: String,
    /// 相似度分数
    pub score: f32,
    /// 来源标识
    pub source: String,
    /// 时间戳
    pub timestamp: i64,
    /// 元数据
    pub metadata: serde_json::Value,
    /// Token 数量
    pub tokens: usize,
}

/// RAG 上下文构建器
#[derive(Debug, Default)]
pub struct RAGContextBuilder {
    results: Vec<RetrievalResult>,
    max_tokens: usize,
    template: String,
}

impl RAGContextBuilder {
    /// 创建新的上下文构建器
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            max_tokens: 4000,
            template: "Context information:\n{context}\n\nQuestion: {query}".to_string(),
        }
    }
    
    /// 设置最大 Token 数
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = tokens;
        self
    }
    
    /// 设置模板
    pub fn template(mut self, template: &str) -> Self {
        self.template = template.to_string();
        self
    }
    
    /// 添加检索结果
    pub fn add_result(mut self, result: RetrievalResult) -> Self {
        self.results.push(result);
        self
    }
    
    /// 添加多个检索结果
    pub fn add_results(mut self, results: Vec<RetrievalResult>) -> Self {
        self.results.extend(results);
        self
    }
    
    /// 构建上下文字符串
    pub fn build(&self) -> String {
        let mut context = String::new();
        let mut total_tokens = 0;
        
        // 按分数排序
        let mut sorted_results = self.results.clone();
        sorted_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        for result in sorted_results {
            if total_tokens + result.tokens > self.max_tokens {
                break;
            }
            
            if !context.is_empty() {
                context.push_str("\n\n");
            }
            
            context.push_str(&format!("[Source: {}]\n{}", result.source, result.text));
            total_tokens += result.tokens;
        }
        
        context
    }
    
    /// 构建完整提示
    pub fn build_prompt(&self, query: &str) -> String {
        let context = self.build();
        self.template
            .replace("{context}", &context)
            .replace("{query}", query)
    }
    
    /// 获取引用列表
    pub fn get_citations(&self) -> Vec<Citation> {
        self.results
            .iter()
            .map(|r| Citation {
                source: r.source.clone(),
                timestamp: r.timestamp,
                score: r.score,
            })
            .collect()
    }
}

/// 引用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub source: String,
    pub timestamp: i64,
    pub score: f32,
}

/// 混合检索器
pub struct HybridRetriever {
    /// 语义搜索权重
    semantic_weight: f32,
    /// 关键词搜索权重
    keyword_weight: f32,
}

impl HybridRetriever {
    /// 创建新的混合检索器
    pub fn new() -> Self {
        Self {
            semantic_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
    
    /// 设置权重
    pub fn with_weights(mut self, semantic: f32, keyword: f32) -> Self {
        self.semantic_weight = semantic;
        self.keyword_weight = keyword;
        self
    }
    
    /// 合并分数
    pub fn combine_scores(&self, semantic_score: f32, keyword_score: f32) -> f32 {
        semantic_score * self.semantic_weight + keyword_score * self.keyword_weight
    }
    
    /// 计算关键词匹配分数
    pub fn keyword_score(query: &str, text: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        
        if query_terms.is_empty() {
            return 0.0;
        }
        
        let matches = query_terms
            .iter()
            .filter(|term| text_lower.contains(**term))
            .count();
        
        matches as f32 / query_terms.len() as f32
    }
}

impl Default for HybridRetriever {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieval_config_default() {
        let config = RetrievalConfig::default();
        assert_eq!(config.top_k, 5);
        assert!((config.similarity_threshold - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_retrieval_config_strict() {
        let config = RetrievalConfig::strict();
        assert_eq!(config.top_k, 3);
        assert!((config.similarity_threshold - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_rag_context_builder() {
        let builder = RAGContextBuilder::new()
            .add_result(RetrievalResult {
                text: "Test content".to_string(),
                score: 0.9,
                source: "test.md".to_string(),
                timestamp: 0,
                metadata: serde_json::json!({}),
                tokens: 10,
            });
        
        let context = builder.build();
        assert!(context.contains("Test content"));
    }

    #[test]
    fn test_rag_context_builder_prompt() {
        let builder = RAGContextBuilder::new()
            .add_result(RetrievalResult {
                text: "Context info".to_string(),
                score: 0.9,
                source: "doc.md".to_string(),
                timestamp: 0,
                metadata: serde_json::json!({}),
                tokens: 10,
            });
        
        let prompt = builder.build_prompt("What is this?");
        assert!(prompt.contains("Context info"));
        assert!(prompt.contains("What is this?"));
    }

    #[test]
    fn test_hybrid_retriever() {
        let retriever = HybridRetriever::new();
        let score = retriever.combine_scores(0.8, 0.6);
        
        // 0.8 * 0.7 + 0.6 * 0.3 = 0.56 + 0.18 = 0.74
        assert!((score - 0.74).abs() < 0.001);
    }

    #[test]
    fn test_keyword_score() {
        let score = HybridRetriever::keyword_score("hello world", "hello there");
        // "hello" matches, "world" doesn't
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_citations() {
        let builder = RAGContextBuilder::new()
            .add_result(RetrievalResult {
                text: "Test".to_string(),
                score: 0.9,
                source: "doc1.md".to_string(),
                timestamp: 123,
                metadata: serde_json::json!({}),
                tokens: 5,
            })
            .add_result(RetrievalResult {
                text: "Test2".to_string(),
                score: 0.8,
                source: "doc2.md".to_string(),
                timestamp: 456,
                metadata: serde_json::json!({}),
                tokens: 5,
            });
        
        let citations = builder.get_citations();
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].source, "doc1.md");
    }
}
