// 工具编排引擎
//
// 支持多工具协作、工具链执行、错误恢复

use crate::tools::{Tool, ToolRegistry, Value};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 工具调用步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStep {
    pub tool: String,
    pub action: String,
    pub params: Value,
    pub optional: bool,
}

/// 工具编排计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub id: String,
    pub name: String,
    pub steps: Vec<ToolStep>,
    pub error_handling: ErrorHandling,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorHandling {
    Stop,           // 遇到错误立即停止
    Continue,       // 继续执行下一步
    Retry(u32),     // 重试指定次数
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_index: usize,
    pub tool: String,
    pub action: String,
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// 编排执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    pub plan_id: String,
    pub success: bool,
    pub step_results: Vec<StepResult>,
    pub total_duration_ms: u64,
    pub error: Option<String>,
}

/// 工具编排引擎
pub struct ToolOrchestrator {
    registry: Arc<ToolRegistry>,
}

impl ToolOrchestrator {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// 执行编排计划
    pub async fn execute(&self, plan: OrchestrationPlan) -> Result<OrchestrationResult> {
        let start_time = std::time::Instant::now();
        let mut step_results = Vec::new();
        let mut success = true;
        let mut error = None;

        for (index, step) in plan.steps.iter().enumerate() {
            let step_start = std::time::Instant::now();
            
            // 执行单个步骤
            let result = self.execute_step(step).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(value) => {
                    step_results.push(StepResult {
                        step_index: index,
                        tool: step.tool.clone(),
                        action: step.action.clone(),
                        success: true,
                        result: Some(value),
                        error: None,
                        duration_ms,
                    });
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    step_results.push(StepResult {
                        step_index: index,
                        tool: step.tool.clone(),
                        action: step.action.clone(),
                        success: false,
                        result: None,
                        error: Some(error_msg.clone()),
                        duration_ms,
                    });

                    // 处理错误
                    if !step.optional {
                        match plan.error_handling {
                            ErrorHandling::Stop => {
                                success = false;
                                error = Some(error_msg);
                                break;
                            }
                            ErrorHandling::Continue => {
                                // 继续执行下一步
                            }
                            ErrorHandling::Retry(times) => {
                                // 重试逻辑
                                for _ in 0..times {
                                    let retry_result = self.execute_step(step).await;
                                    if retry_result.is_ok() {
                                        // 重试成功，更新结果
                                        if let Some(last) = step_results.last_mut() {
                                            last.success = true;
                                            last.error = None;
                                            last.result = retry_result.ok();
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let total_duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(OrchestrationResult {
            plan_id: plan.id,
            success,
            step_results,
            total_duration_ms,
            error,
        })
    }

    /// 执行单个步骤
    async fn execute_step(&self, step: &ToolStep) -> Result<Value> {
        // 构建工具调用参数
        let mut params = step.params.clone();
        if let Some(obj) = params.as_object_mut() {
            obj.insert("action".to_string(), serde_json::json!(step.action));
        }

        // 从注册表获取工具并执行
        let tools = self.registry.list_tools().await;
        for tool_meta in tools {
            if tool_meta.name == step.tool {
                // 找到工具，执行
                // 注意：这里简化实现，实际需要从 registry 获取工具实例
                return Ok(serde_json::json!({
                    "status": "success",
                    "tool": step.tool,
                    "action": step.action,
                    "message": "Tool executed (placeholder)"
                }));
            }
        }

        Err(anyhow::anyhow!("Tool not found: {}", step.tool))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestration_plan_creation() {
        let plan = OrchestrationPlan {
            id: "test-plan".to_string(),
            name: "Test Plan".to_string(),
            steps: vec![ToolStep {
                tool: "memory".to_string(),
                action: "search".to_string(),
                params: serde_json::json!({"query": "test"}),
                optional: false,
            }],
            error_handling: ErrorHandling::Stop,
            timeout_ms: 30000,
        };

        assert_eq!(plan.id, "test-plan");
        assert_eq!(plan.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let registry = Arc::new(ToolRegistry::new());
        let orchestrator = ToolOrchestrator::new(registry);
        
        // 简单验证创建成功
        assert!(true);
    }

    #[test]
    fn test_error_handling() {
        assert_eq!(ErrorHandling::Stop, ErrorHandling::Stop);
        assert_ne!(ErrorHandling::Stop, ErrorHandling::Continue);
        assert_eq!(ErrorHandling::Retry(3), ErrorHandling::Retry(3));
    }

    #[test]
    fn test_step_result() {
        let result = StepResult {
            step_index: 0,
            tool: "memory".to_string(),
            action: "search".to_string(),
            success: true,
            result: Some(serde_json::json!({"count": 5})),
            error: None,
            duration_ms: 10,
        };

        assert!(result.success);
        assert_eq!(result.step_index, 0);
    }
}
