// 恢复执行器模块

use std::collections::HashMap;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::audit::{AuditLogger, AuditEvent, EventType};
use super::diagnostic::DiagnosticResult;

/// 恢复级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryLevel {
    /// L1 - 快速修复
    L1QuickFix,
    /// L2 - AI 诊断
    L2AiDiagnosis,
    /// L3 - 人工介入
    L3HumanIntervention,
}

impl Default for RecoveryLevel {
    fn default() -> Self {
        Self::L1QuickFix
    }
}

/// 恢复状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Cancelled,
}

/// 恢复动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    /// 动作名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 状态
    pub state: RecoveryState,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 输出
    pub output: String,
    /// 错误
    pub error: Option<String>,
}

impl RecoveryAction {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            state: RecoveryState::Pending,
            started_at: None,
            completed_at: None,
            output: String::new(),
            error: None,
        }
    }
    
    pub fn start(&mut self) {
        self.state = RecoveryState::InProgress;
        self.started_at = Some(Utc::now());
    }
    
    pub fn complete(&mut self, output: String) {
        self.state = RecoveryState::Succeeded;
        self.completed_at = Some(Utc::now());
        self.output = output;
    }
    
    pub fn fail(&mut self, error: String) {
        self.state = RecoveryState::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error);
    }
    
    /// 检查是否已完成
    pub fn is_completed(&self) -> bool {
        matches!(self.state, RecoveryState::Succeeded | RecoveryState::Failed | RecoveryState::Cancelled)
    }
}

/// 恢复计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    /// 计划 ID
    pub id: String,
    /// 恢复级别
    pub level: RecoveryLevel,
    /// 目标组件
    pub component: String,
    /// 诊断结果
    pub diagnostic_result: Option<DiagnosticResult>,
    /// 动作列表
    pub actions: Vec<RecoveryAction>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 重试次数
    pub retry_count: u32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 退避时间
    pub backoff: Duration,
}

impl RecoveryPlan {
    pub fn new(level: RecoveryLevel, component: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            level,
            component,
            diagnostic_result: None,
            actions: vec![],
            created_at: Utc::now(),
            retry_count: 0,
            max_retries: 3,
            backoff: Duration::from_secs(1),
        }
    }
    
    pub fn with_actions(mut self, actions: Vec<RecoveryAction>) -> Self {
        self.actions = actions;
        self
    }
    
    pub fn with_diagnostic(mut self, result: DiagnosticResult) -> Self {
        self.diagnostic_result = Some(result);
        self
    }
    
    pub fn next_backoff(&mut self) -> Duration {
        let current = self.backoff;
        self.backoff = std::cmp::min(self.backoff * 2, Duration::from_secs(60));
        self.retry_count += 1;
        current
    }
    
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

/// 恢复结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    /// 计划 ID
    pub plan_id: String,
    /// 是否成功
    pub success: bool,
    /// 恢复级别
    pub level: RecoveryLevel,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 完成时间
    pub completed_at: DateTime<Utc>,
    /// 执行的动作
    pub actions: Vec<RecoveryAction>,
    /// 消息
    pub message: String,
}

impl RecoveryResult {
    pub fn success(plan: &RecoveryPlan) -> Self {
        Self {
            plan_id: plan.id.clone(),
            success: true,
            level: plan.level,
            started_at: plan.created_at,
            completed_at: Utc::now(),
            actions: plan.actions.clone(),
            message: "Recovery succeeded".to_string(),
        }
    }
    
    pub fn failure(plan: &RecoveryPlan, message: String) -> Self {
        Self {
            plan_id: plan.id.clone(),
            success: false,
            level: plan.level,
            started_at: plan.created_at,
            completed_at: Utc::now(),
            actions: plan.actions.clone(),
            message,
        }
    }
    
    pub fn duration(&self) -> Duration {
        (self.completed_at - self.started_at).to_std().unwrap_or(Duration::ZERO)
    }
}

/// 恢复执行器
pub struct RecoveryExecutor {
    audit_log: AuditLogger,
}

impl RecoveryExecutor {
    pub fn new(audit_log: AuditLogger) -> Self {
        Self { audit_log }
    }
    
    /// 执行恢复计划
    pub async fn execute(&self, plan: RecoveryPlan) -> anyhow::Result<RecoveryResult> {
        // 记录开始
        self.audit_log.log_recovery(
            EventType::RecoveryStarted,
            &plan.id,
            format!("Starting {:?} recovery for {}", plan.level, plan.component),
        )?;
        
        let mut plan = plan;
        let mut failed = false;
        let mut error_msg = String::new();
        
        // 执行所有动作
        for i in 0..plan.actions.len() {
            plan.actions[i].start();
            
            match self.execute_action(&plan.actions[i]).await {
                Ok(output) => {
                    plan.actions[i].complete(output);
                }
                Err(e) => {
                    plan.actions[i].fail(e.to_string());
                    
                    // 如果动作失败且不能重试，记录失败
                    if !plan.can_retry() {
                        failed = true;
                        error_msg = e.to_string();
                        break;
                    }
                }
            }
        }
        
        if failed {
            let result = RecoveryResult::failure(&plan, error_msg);
            self.audit_log.log_recovery(
                EventType::RecoveryFailed,
                &plan.id,
                format!("Recovery failed"),
            )?;
            return Ok(result);
        }
        
        let result = RecoveryResult::success(&plan);
        self.audit_log.log_recovery(
            EventType::RecoverySucceeded,
            &plan.id,
            "Recovery completed successfully".to_string(),
        )?;
        
        Ok(result)
    }
    
    /// 执行单个动作
    async fn execute_action(&self, action: &RecoveryAction) -> anyhow::Result<String> {
        match action.name.as_str() {
            "restart_service" => self.restart_service().await,
            "clear_cache" => self.clear_cache().await,
            "rollback_config" => self.rollback_config().await,
            "release_resources" => self.release_resources().await,
            "ai_diagnosis" => self.ai_diagnosis().await,
            "notify_human" => self.notify_human().await,
            "enter_safe_mode" => self.enter_safe_mode().await,
            _ => Err(anyhow::anyhow!("Unknown action: {}", action.name)),
        }
    }
    
    /// L1 动作：重启服务
    async fn restart_service(&self) -> anyhow::Result<String> {
        // 模拟重启
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok("Service restarted".to_string())
    }
    
    /// L1 动作：清理缓存
    async fn clear_cache(&self) -> anyhow::Result<String> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok("Cache cleared".to_string())
    }
    
    /// L1 动作：回滚配置
    async fn rollback_config(&self) -> anyhow::Result<String> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok("Config rolled back".to_string())
    }
    
    /// L1 动作：释放资源
    async fn release_resources(&self) -> anyhow::Result<String> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Ok("Resources released".to_string())
    }
    
    /// L2 动作：AI 诊断
    async fn ai_diagnosis(&self) -> anyhow::Result<String> {
        // TODO: 调用 LLM 分析日志，生成修复建议
        // 当前返回模拟结果
        tokio::time::sleep(Duration::from_millis(200)).await;
        self.audit_log.log(AuditEvent::new(
            EventType::AiDiagnosisStarted,
            "watchdog".to_string(),
            "AI diagnosis started".to_string(),
        ))?;
        Ok("AI diagnosis completed: suggested action is restart_service".to_string())
    }
    
    /// L3 动作：通知人工
    async fn notify_human(&self) -> anyhow::Result<String> {
        // TODO: 实际发送通知
        Ok("Human notified".to_string())
    }
    
    /// L3 动作：进入安全模式
    async fn enter_safe_mode(&self) -> anyhow::Result<String> {
        self.audit_log.log(AuditEvent::new(
            EventType::SafeModeEntered,
            "watchdog".to_string(),
            "Entering safe mode".to_string(),
        ))?;
        Ok("Safe mode entered".to_string())
    }
    
    /// 生成 L1 恢复计划
    pub fn generate_l1_plan(component: String, actions: Vec<String>) -> RecoveryPlan {
        let recovery_actions: Vec<RecoveryAction> = actions
            .into_iter()
            .map(|name| RecoveryAction::new(name.clone(), format!("Execute {}", name)))
            .collect();
        
        RecoveryPlan::new(RecoveryLevel::L1QuickFix, component)
            .with_actions(recovery_actions)
    }
    
    /// 生成 L2 恢复计划
    pub fn generate_l2_plan(component: String, diagnostic: DiagnosticResult) -> RecoveryPlan {
        let mut plan = RecoveryPlan::new(RecoveryLevel::L2AiDiagnosis, component);
        plan.diagnostic_result = Some(diagnostic);
        plan.actions = vec![
            RecoveryAction::new("ai_diagnosis".to_string(), "AI analysis of logs".to_string()),
        ];
        plan
    }
    
    /// 生成 L3 恢复计划
    pub fn generate_l3_plan(component: String) -> RecoveryPlan {
        RecoveryPlan::new(RecoveryLevel::L3HumanIntervention, component)
            .with_actions(vec![
                RecoveryAction::new("notify_human".to_string(), "Send alert to human".to_string()),
                RecoveryAction::new("enter_safe_mode".to_string(), "Enter safe mode".to_string()),
            ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watchdog::config::AuditConfig;
    
    #[test]
    fn test_recovery_action_lifecycle() {
        let mut action = RecoveryAction::new("restart".to_string(), "Restart service".to_string());
        assert!(matches!(action.state, RecoveryState::Pending));
        
        action.start();
        assert!(matches!(action.state, RecoveryState::InProgress));
        assert!(action.started_at.is_some());
        
        action.complete("OK".to_string());
        assert!(matches!(action.state, RecoveryState::Succeeded));
        assert!(action.completed_at.is_some());
    }
    
    #[test]
    fn test_recovery_action_fail() {
        let mut action = RecoveryAction::new("test".to_string(), "Test".to_string());
        action.start();
        action.fail("Error".to_string());
        
        assert!(matches!(action.state, RecoveryState::Failed));
        assert_eq!(action.error, Some("Error".to_string()));
    }
    
    #[test]
    fn test_recovery_plan_backoff() {
        let mut plan = RecoveryPlan::new(RecoveryLevel::L1QuickFix, "test".to_string());
        
        let backoff1 = plan.next_backoff();
        assert_eq!(backoff1, Duration::from_secs(1));
        
        let backoff2 = plan.next_backoff();
        assert_eq!(backoff2, Duration::from_secs(2));
        
        let backoff3 = plan.next_backoff();
        assert_eq!(backoff3, Duration::from_secs(4));
    }
    
    #[test]
    fn test_recovery_plan_can_retry() {
        let mut plan = RecoveryPlan::new(RecoveryLevel::L1QuickFix, "test".to_string());
        plan.max_retries = 2;
        
        assert!(plan.can_retry());
        plan.next_backoff();
        assert!(plan.can_retry());
        plan.next_backoff();
        assert!(!plan.can_retry());
    }
    
    #[tokio::test]
    async fn test_recovery_executor_l1() {
        let audit_log = AuditLogger::new(AuditConfig::default());
        let executor = RecoveryExecutor::new(audit_log);
        
        let plan = RecoveryExecutor::generate_l1_plan(
            "test".to_string(),
            vec!["clear_cache".to_string()],
        );
        
        let result = executor.execute(plan).await.unwrap();
        assert!(result.success);
    }
    
    #[test]
    fn test_recovery_result_duration() {
        let plan = RecoveryPlan::new(RecoveryLevel::L1QuickFix, "test".to_string());
        let result = RecoveryResult::success(&plan);
        
        let duration = result.duration();
        assert!(duration < Duration::from_secs(1));
    }
}
