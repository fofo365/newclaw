// Strategy Engine Module - v0.5.0
//
// 策略引擎：
// - 策略注册表
// - 策略配置系统
// - 策略效果评估

use crate::context::truncation::TruncationStrategyType;
use crate::llm::Message;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 策略类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum StrategyType {
    /// 智能截断
    Smart,
    /// 时间衰减
    TimeDecay,
    /// 语义聚类
    SemanticCluster,
    /// 最大化信息
    MaximizeInfo,
    /// 最小化 Token
    MinimizeTokens,
    /// 平衡模式
    Balanced,
}

/// 策略定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDefinition {
    /// 策略名称
    pub name: String,
    /// 策略类型
    pub strategy_type: StrategyType,
    /// 描述
    pub description: String,
    /// 参数
    pub parameters: HashMap<String, serde_json::Value>,
}

/// 策略执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyResult {
    /// 策略类型
    pub strategy_type: StrategyType,
    /// 输入 token 数量
    pub input_tokens: usize,
    /// 输出 token 数量
    pub output_tokens: usize,
    /// 压缩率（0-1）
    pub compression_ratio: f32,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
}

/// 策略引擎
pub struct StrategyEngine {
    /// 注册的策略
    strategies: HashMap<StrategyType, StrategyDefinition>,
    /// 策略执行历史
    execution_history: Vec<StrategyResult>,
}

impl StrategyEngine {
    /// 创建新的策略引擎
    pub fn new() -> anyhow::Result<Self> {
        let mut engine = Self {
            strategies: HashMap::new(),
            execution_history: Vec::new(),
        };

        // 注册内置策略
        engine.register_builtin_strategies()?;

        Ok(engine)
    }

    /// 注册内置策略
    fn register_builtin_strategies(&mut self) -> anyhow::Result<()> {
        // 智能截断
        self.register_strategy(StrategyDefinition {
            name: "智能截断".to_string(),
            strategy_type: StrategyType::Smart,
            description: "基于相关性排序，保留最相关的消息".to_string(),
            parameters: HashMap::new(),
        })?;

        // 时间衰减
        self.register_strategy(StrategyDefinition {
            name: "时间衰减".to_string(),
            strategy_type: StrategyType::TimeDecay,
            description: "优先近期信息".to_string(),
            parameters: HashMap::new(),
        })?;

        // 语义聚类
        self.register_strategy(StrategyDefinition {
            name: "语义聚类".to_string(),
            strategy_type: StrategyType::SemanticCluster,
            description: "去重相似信息".to_string(),
            parameters: HashMap::new(),
        })?;

        // 最大化信息
        self.register_strategy(StrategyDefinition {
            name: "最大化信息".to_string(),
            strategy_type: StrategyType::MaximizeInfo,
            description: "保留最多信息".to_string(),
            parameters: HashMap::new(),
        })?;

        // 最小化 Token
        self.register_strategy(StrategyDefinition {
            name: "最小化 Token".to_string(),
            strategy_type: StrategyType::MinimizeTokens,
            description: "最小化成本".to_string(),
            parameters: HashMap::new(),
        })?;

        // 平衡模式
        self.register_strategy(StrategyDefinition {
            name: "平衡模式".to_string(),
            strategy_type: StrategyType::Balanced,
            description: "平衡信息和成本".to_string(),
            parameters: HashMap::new(),
        })?;

        Ok(())
    }

    /// 注册策略
    pub fn register_strategy(&mut self, strategy: StrategyDefinition) -> anyhow::Result<()> {
        self.strategies.insert(strategy.strategy_type.clone(), strategy);
        Ok(())
    }

    /// 获取策略
    pub fn get_strategy(&self, strategy_type: &StrategyType) -> Option<&StrategyDefinition> {
        self.strategies.get(strategy_type)
    }

    /// 列出所有策略
    pub fn list_strategies(&self) -> Vec<&StrategyDefinition> {
        self.strategies.values().collect()
    }

    /// 应用策略
    pub async fn apply(
        &mut self,
        messages: &Vec<Message>,
        strategy_type: StrategyType,
        max_tokens: usize,
        model: &str,
    ) -> anyhow::Result<Vec<Message>> {
        let start_time = std::time::Instant::now();

        // 计算输入 token 数量
        let input_tokens = self.estimate_tokens(messages, model)?;

        // 应用策略（简化实现，实际需要调用 TruncationStrategy）
        let result = self.apply_strategy_internal(messages, strategy_type.clone(), max_tokens)?;

        // 计算输出 token 数量
        let output_tokens = self.estimate_tokens(&result, model)?;

        // 计算压缩率
        let compression_ratio = if input_tokens > 0 {
            output_tokens as f32 / input_tokens as f32
        } else {
            1.0
        };

        // 记录执行结果
        let execution_time = start_time.elapsed().as_millis() as u64;
        self.execution_history.push(StrategyResult {
            strategy_type,
            input_tokens,
            output_tokens,
            compression_ratio,
            execution_time_ms: execution_time,
        });

        Ok(result)
    }

    /// 内部策略应用（简化实现）
    fn apply_strategy_internal(
        &self,
        messages: &Vec<Message>,
        strategy_type: StrategyType,
        max_tokens: usize,
    ) -> anyhow::Result<Vec<Message>> {
        match strategy_type {
            StrategyType::TimeDecay => {
                // 保留最近的消息
                let mut result = Vec::new();
                let mut current_length = 0;

                for msg in messages.iter().rev() {
                    if current_length + msg.content.len() > max_tokens * 4 {
                        break;
                    }
                    current_length += msg.content.len();
                    result.insert(0, msg.clone());
                }

                Ok(result)
            }
            StrategyType::MinimizeTokens => {
                // 保留短消息
                let mut sorted = messages.clone();
                sorted.sort_by_key(|msg| msg.content.len());

                let mut result = Vec::new();
                let mut current_length = 0;

                for msg in &sorted {
                    if current_length + msg.content.len() > max_tokens * 4 {
                        break;
                    }
                    current_length += msg.content.len();
                    result.push(msg.clone());
                }

                Ok(result)
            }
            _ => {
                // 默认：平衡策略
                let mut result = messages.clone();
                if result.len() > 10 {
                    result = result.split_off(result.len() - 10);
                }
                Ok(result)
            }
        }
    }

    /// 估算 token 数量（简化实现）
    fn estimate_tokens(&self, messages: &Vec<Message>, _model: &str) -> anyhow::Result<usize> {
        // 简化：假设每 4 个字符 = 1 个 token
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
        Ok(total_chars / 4)
    }

    /// 获取策略执行历史
    pub fn get_execution_history(&self) -> &[StrategyResult] {
        &self.execution_history
    }

    /// 获取策略效果统计
    pub fn get_strategy_stats(&self, strategy_type: &StrategyType) -> Option<StrategyStats> {
        let results: Vec<_> = self
            .execution_history
            .iter()
            .filter(|r| &r.strategy_type == strategy_type)
            .collect();

        if results.is_empty() {
            return None;
        }

        let avg_compression = results.iter().map(|r| r.compression_ratio).sum::<f32>() / results.len() as f32;
        let avg_time = results.iter().map(|r| r.execution_time_ms).sum::<u64>() / results.len() as u64;

        Some(StrategyStats {
            strategy_type: strategy_type.clone(),
            total_executions: results.len(),
            avg_compression_ratio: avg_compression,
            avg_execution_time_ms: avg_time,
        })
    }
}

impl Default for StrategyEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create StrategyEngine")
    }
}

/// 策略统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStats {
    /// 策略类型
    pub strategy_type: StrategyType,
    /// 总执行次数
    pub total_executions: usize,
    /// 平均压缩率
    pub avg_compression_ratio: f32,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;

    #[test]
    fn test_strategy_engine() {
        let engine = StrategyEngine::new().unwrap();

        // 测试策略注册
        let strategies = engine.list_strategies();
        assert_eq!(strategies.len(), 6);

        // 测试策略获取
        let strategy = engine.get_strategy(&StrategyType::Balanced);
        assert!(strategy.is_some());
        assert_eq!(strategy.unwrap().name, "平衡模式");
    }

    #[tokio::test]
    async fn test_apply_strategy() {
        let mut engine = StrategyEngine::new().unwrap();

        let messages = vec![
            Message {
                role: MessageRole::User,
                content: "First message".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: MessageRole::Assistant,
                content: "Response".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: MessageRole::User,
                content: "Second message".to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let result = engine.apply(&messages, StrategyType::Balanced, 100, "gpt-4").await.unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_strategy_stats() {
        let mut engine = StrategyEngine::new().unwrap();

        // 执行一些策略
        let messages = vec![Message {
            role: MessageRole::User,
            content: "Test message".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        // 手动添加一些执行历史
        engine.execution_history.push(StrategyResult {
            strategy_type: StrategyType::Balanced,
            input_tokens: 100,
            output_tokens: 80,
            compression_ratio: 0.8,
            execution_time_ms: 10,
        });

        let stats = engine.get_strategy_stats(&StrategyType::Balanced);
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().total_executions, 1);
    }
}
