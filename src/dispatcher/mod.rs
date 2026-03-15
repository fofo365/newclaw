// 多模型调度模块 (v0.5.5)
//
// 基于策略、Agent、命名的动态模型调度

use std::collections::HashMap;
use std::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// 模型调度策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDispatchPolicy {
    /// 策略 ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 主控模型（用于任务评估）
    pub controller_model: String,
    /// 任务类型映射
    pub task_type_mapping: HashMap<TaskType, ModelConfig>,
    /// 降级策略
    pub fallback_models: Vec<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// 代码生成
    CodeGeneration,
    /// 代码审查
    CodeReview,
    /// 文本生成
    TextGeneration,
    /// 翻译
    Translation,
    /// 问答
    QnA,
    /// 推理
    Reasoning,
    /// 创意写作
    CreativeWriting,
    /// 数据分析
    DataAnalysis,
    /// 工具调用
    ToolCalling,
    /// 嵌入
    Embedding,
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型名称
    pub model: String,
    /// Provider
    pub provider: String,
    /// 温度
    pub temperature: f32,
    /// 最大 Token
    pub max_tokens: usize,
    /// 优先级（越高越优先）
    pub priority: u8,
}

/// Agent 模型绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentModelBinding {
    /// Agent ID
    pub agent_id: String,
    /// 默认模型
    pub default_model: String,
    /// 任务专用模型
    pub task_models: HashMap<TaskType, String>,
    /// 用户专用模型
    pub user_models: HashMap<String, String>,
}

/// 模型调度器
pub struct ModelDispatcher {
    /// 调度策略
    policies: RwLock<HashMap<String, ModelDispatchPolicy>>,
    /// Agent 绑定
    agent_bindings: RwLock<HashMap<String, AgentModelBinding>>,
    /// 用户偏好
    user_preferences: RwLock<HashMap<String, String>>,
    /// 默认策略
    default_policy: RwLock<Option<String>>,
}

impl ModelDispatcher {
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(HashMap::new()),
            agent_bindings: RwLock::new(HashMap::new()),
            user_preferences: RwLock::new(HashMap::new()),
            default_policy: RwLock::new(None),
        }
    }
    
    /// 注册调度策略
    pub fn register_policy(&self, policy: ModelDispatchPolicy) {
        let mut policies = self.policies.write().unwrap();
        if policies.is_empty() {
            *self.default_policy.write().unwrap() = Some(policy.id.clone());
        }
        policies.insert(policy.id.clone(), policy);
    }
    
    /// 绑定 Agent 模型
    pub fn bind_agent(&self, binding: AgentModelBinding) {
        let mut bindings = self.agent_bindings.write().unwrap();
        bindings.insert(binding.agent_id.clone(), binding);
    }
    
    /// 设置用户模型偏好
    pub fn set_user_preference(&self, user_id: &str, model: &str) {
        self.user_preferences.write().unwrap()
            .insert(user_id.to_string(), model.to_string());
    }
    
    /// 调度模型
    pub fn dispatch(&self, request: DispatchRequest) -> DispatchResult {
        // 1. 检查用户偏好
        if let Some(user_model) = self.get_user_preference(&request.user_id) {
            return DispatchResult {
                model: user_model,
                reason: DispatchReason::UserPreference,
                estimated_latency_ms: 100,
            };
        }
        
        // 2. 检查 Agent 绑定
        if let Some(agent_model) = self.get_agent_model(&request.agent_id, &request.task_type) {
            return DispatchResult {
                model: agent_model,
                reason: DispatchReason::AgentBinding,
                estimated_latency_ms: 100,
            };
        }
        
        // 3. 使用调度策略
        if let Some(policy_id) = self.default_policy.read().unwrap().as_ref() {
            if let Some(model) = self.dispatch_by_policy(policy_id, &request.task_type) {
                return DispatchResult {
                    model,
                    reason: DispatchReason::Policy,
                    estimated_latency_ms: 100,
                };
            }
        }
        
        // 4. 默认模型
        DispatchResult {
            model: "glm-4".to_string(),
            reason: DispatchReason::Default,
            estimated_latency_ms: 100,
        }
    }
    
    fn get_user_preference(&self, user_id: &str) -> Option<String> {
        self.user_preferences.read().unwrap().get(user_id).cloned()
    }
    
    fn get_agent_model(&self, agent_id: &str, task_type: &TaskType) -> Option<String> {
        let bindings = self.agent_bindings.read().unwrap();
        bindings.get(agent_id).and_then(|b| {
            b.task_models.get(task_type).cloned()
                .or_else(|| Some(b.default_model.clone()))
        })
    }
    
    fn dispatch_by_policy(&self, policy_id: &str, task_type: &TaskType) -> Option<String> {
        let policies = self.policies.read().unwrap();
        policies.get(policy_id).and_then(|p| {
            p.task_type_mapping.get(task_type)
                .map(|m| m.model.clone())
        })
    }
    
    /// 主控模型评估任务类型
    pub async fn evaluate_task_type(&self, prompt: &str) -> TaskType {
        let prompt_lower = prompt.to_lowercase();
        
        // 简单的关键词匹配
        if prompt_lower.contains("代码") || prompt_lower.contains("code") || prompt_lower.contains("function") {
            TaskType::CodeGeneration
        } else if prompt_lower.contains("翻译") || prompt_lower.contains("translate") {
            TaskType::Translation
        } else if prompt_lower.contains("审查") || prompt_lower.contains("review") {
            TaskType::CodeReview
        } else if prompt_lower.contains("分析") || prompt_lower.contains("analyze") {
            TaskType::DataAnalysis
        } else if prompt_lower.contains("创意") || prompt_lower.contains("故事") || prompt_lower.contains("story") {
            TaskType::CreativeWriting
        } else if prompt_lower.contains("推理") || prompt_lower.contains("reasoning") {
            TaskType::Reasoning
        } else {
            TaskType::QnA
        }
    }
}

impl Default for ModelDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// 调度请求
#[derive(Debug, Clone)]
pub struct DispatchRequest {
    pub user_id: String,
    pub agent_id: String,
    pub task_type: TaskType,
    pub prompt: String,
    pub context: Option<String>,
}

/// 调度结果
#[derive(Debug, Clone)]
pub struct DispatchResult {
    pub model: String,
    pub reason: DispatchReason,
    pub estimated_latency_ms: u64,
}

/// 调度原因
#[derive(Debug, Clone)]
pub enum DispatchReason {
    UserPreference,
    AgentBinding,
    Policy,
    Default,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_dispatcher_new() {
        let dispatcher = ModelDispatcher::new();
        let request = DispatchRequest {
            user_id: "user1".to_string(),
            agent_id: "agent1".to_string(),
            task_type: TaskType::QnA,
            prompt: "Hello".to_string(),
            context: None,
        };
        
        let result = dispatcher.dispatch(request);
        assert_eq!(result.model, "glm-4");
    }

    #[test]
    fn test_register_policy() {
        let dispatcher = ModelDispatcher::new();
        
        let policy = ModelDispatchPolicy {
            id: "policy1".to_string(),
            name: "Default Policy".to_string(),
            controller_model: "glm-4".to_string(),
            task_type_mapping: vec![
                (TaskType::CodeGeneration, ModelConfig {
                    model: "glmcode/glm-5".to_string(),
                    provider: "glmcode".to_string(),
                    temperature: 0.3,
                    max_tokens: 4096,
                    priority: 10,
                }),
            ].into_iter().collect(),
            fallback_models: vec!["glm-4".to_string()],
            enabled: true,
        };
        
        dispatcher.register_policy(policy);
    }

    #[tokio::test]
    async fn test_evaluate_task_type() {
        let dispatcher = ModelDispatcher::new();
        
        let task = dispatcher.evaluate_task_type("写一个代码").await;
        assert_eq!(task, TaskType::CodeGeneration);
        
        let task = dispatcher.evaluate_task_type("翻译这段文字").await;
        assert_eq!(task, TaskType::Translation);
    }
}
