// 通知模块 - L3 人工介入通知

use std::sync::Arc;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::RwLock;

use super::config::L3Config;
use super::audit::{AuditLogger, AuditEvent, EventType};
use super::recovery::RecoveryPlan;

/// 通知器
pub struct Notifier {
    /// 配置
    config: L3Config,
    /// 审计日志
    audit_log: AuditLogger,
    /// 飞书 webhook（可选）
    feishu_webhook: Option<String>,
}

/// 告警消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertMessage {
    /// 告警级别
    pub level: AlertLevel,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 组件
    pub component: String,
    /// 建议操作
    pub suggested_actions: Vec<String>,
    /// 时间戳
    pub timestamp: String,
}

/// 告警级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
    Emergency,
}

impl AlertMessage {
    pub fn new(level: AlertLevel, title: String, component: String) -> Self {
        Self {
            level,
            title,
            content: String::new(),
            component,
            suggested_actions: vec![],
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
    
    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
    }
    
    pub fn with_actions(mut self, actions: Vec<String>) -> Self {
        self.suggested_actions = actions;
        self
    }
    
    /// 转换为飞书卡片消息
    pub fn to_feishu_card(&self) -> serde_json::Value {
        let level_color = match self.level {
            AlertLevel::Info => "blue",
            AlertLevel::Warning => "yellow",
            AlertLevel::Critical => "red",
            AlertLevel::Emergency => "red",
        };
        
        let level_text = match self.level {
            AlertLevel::Info => "ℹ️ 信息",
            AlertLevel::Warning => "⚠️ 警告",
            AlertLevel::Critical => "🔴 严重",
            AlertLevel::Emergency => "🚨 紧急",
        };
        
        json!({
            "msg_type": "interactive",
            "card": {
                "config": {
                    "wide_screen_mode": true
                },
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": self.title
                    },
                    "template": level_color
                },
                "elements": [
                    {
                        "tag": "div",
                        "fields": [
                            {
                                "is_short": true,
                                "text": {
                                    "tag": "lark_md",
                                    "content": format!("**级别**\n{}", level_text)
                                }
                            },
                            {
                                "is_short": true,
                                "text": {
                                    "tag": "lark_md",
                                    "content": format!("**组件**\n{}", self.component)
                                }
                            }
                        ]
                    },
                    {
                        "tag": "div",
                        "text": {
                            "tag": "lark_md",
                            "content": self.content
                        }
                    },
                    {
                        "tag": "note",
                        "elements": [
                            {
                                "tag": "plain_text",
                                "content": format!("时间: {}", self.timestamp)
                            }
                        ]
                    }
                ]
            }
        })
    }
    
    /// 转换为简单文本消息
    pub fn to_text(&self) -> String {
        let level_str = match self.level {
            AlertLevel::Info => "INFO",
            AlertLevel::Warning => "WARNING",
            AlertLevel::Critical => "CRITICAL",
            AlertLevel::Emergency => "EMERGENCY",
        };
        
        let actions_str = if self.suggested_actions.is_empty() {
            "无".to_string()
        } else {
            self.suggested_actions.join(", ")
        };
        
        format!(
            "[{}] {}\n组件: {}\n内容: {}\n建议操作: {}\n时间: {}",
            level_str,
            self.title,
            self.component,
            self.content,
            actions_str,
            self.timestamp
        )
    }
}

impl Notifier {
    /// 创建新的通知器
    pub fn new(config: L3Config, audit_log: AuditLogger) -> Self {
        Self {
            config,
            audit_log,
            feishu_webhook: None,
        }
    }
    
    /// 设置飞书 webhook
    pub fn with_feishu_webhook(mut self, webhook: String) -> Self {
        self.feishu_webhook = Some(webhook);
        self
    }
    
    /// 发送告警
    pub async fn send_alert(&self, message: AlertMessage) -> Result<()> {
        // 记录审计日志
        self.audit_log.log(AuditEvent::new(
            EventType::HumanIntervention,
            message.component.clone(),
            format!("Sending {} alert: {}", 
                match message.level {
                    AlertLevel::Info => "info",
                    AlertLevel::Warning => "warning",
                    AlertLevel::Critical => "critical",
                    AlertLevel::Emergency => "emergency",
                },
                message.title
            ),
        ))?;
        
        let mut sent = false;
        
        // 通过配置的渠道发送
        for channel in &self.config.alert_channels {
            match channel.as_str() {
                "feishu" => {
                    if let Some(ref webhook) = self.feishu_webhook {
                        if let Err(e) = self.send_to_feishu(webhook, &message).await {
                            tracing::error!("Failed to send alert to Feishu: {}", e);
                        } else {
                            sent = true;
                        }
                    }
                }
                "log" => {
                    // 记录到日志
                    tracing::warn!("ALERT: {}", message.to_text());
                    sent = true;
                }
                _ => {
                    tracing::warn!("Unknown alert channel: {}", channel);
                }
            }
        }
        
        if !sent {
            // 至少记录到日志
            tracing::warn!("ALERT (no channel): {}", message.to_text());
        }
        
        Ok(())
    }
    
    /// 发送到飞书
    async fn send_to_feishu(&self, webhook: &str, message: &AlertMessage) -> Result<()> {
        let card = message.to_feishu_card();
        
        let client = reqwest::Client::new();
        let response = client
            .post(webhook)
            .json(&card)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Feishu webhook failed: {} - {}", status, body));
        }
        
        tracing::info!("Alert sent to Feishu: {}", message.title);
        Ok(())
    }
    
    /// 从恢复计划创建告警消息
    pub fn alert_from_plan(&self, plan: &RecoveryPlan, reason: &str) -> AlertMessage {
        let level = match plan.level {
            super::recovery::RecoveryLevel::L1QuickFix => AlertLevel::Warning,
            super::recovery::RecoveryLevel::L2AiDiagnosis => AlertLevel::Critical,
            super::recovery::RecoveryLevel::L3HumanIntervention => AlertLevel::Emergency,
        };
        
        let title = format!("NewClaw Watchdog - {} 恢复需要人工介入", plan.component);
        
        let content = format!(
            "**恢复计划**: {}\n**恢复级别**: {:?}\n**原因**: {}\n**重试次数**: {}",
            plan.id,
            plan.level,
            reason,
            plan.retry_count
        );
        
        let actions: Vec<String> = plan.actions
            .iter()
            .filter(|a| !a.is_completed())
            .map(|a| a.name.clone())
            .collect();
        
        AlertMessage::new(level, title, plan.component.clone())
            .with_content(content)
            .with_actions(actions)
    }
    
    /// 发送恢复失败告警
    pub async fn notify_recovery_failed(&self, plan: &RecoveryPlan, error: &str) -> Result<()> {
        let message = self.alert_from_plan(plan, &format!("恢复失败: {}", error));
        self.send_alert(message).await
    }
    
    /// 发送进入安全模式告警
    pub async fn notify_safe_mode(&self, component: &str, reason: &str) -> Result<()> {
        let message = AlertMessage::new(
            AlertLevel::Emergency,
            "NewClaw 进入安全模式".to_string(),
            component.to_string(),
        )
        .with_content(format!("系统已进入安全模式。\n原因: {}", reason))
        .with_actions(vec!["检查系统状态".to_string(), "手动恢复".to_string()]);
        
        self.send_alert(message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watchdog::config::AuditConfig;
    use crate::watchdog::recovery::RecoveryLevel;
    
    #[test]
    fn test_alert_message_creation() {
        let alert = AlertMessage::new(
            AlertLevel::Warning,
            "Test Alert".to_string(),
            "test-component".to_string(),
        );
        
        assert_eq!(alert.level, AlertLevel::Warning);
        assert_eq!(alert.title, "Test Alert");
    }
    
    #[test]
    fn test_alert_to_text() {
        let alert = AlertMessage::new(
            AlertLevel::Critical,
            "Test".to_string(),
            "comp".to_string(),
        )
        .with_content("Something went wrong".to_string())
        .with_actions(vec!["restart".to_string()]);
        
        let text = alert.to_text();
        assert!(text.contains("CRITICAL"));
        assert!(text.contains("Something went wrong"));
    }
    
    #[test]
    fn test_alert_to_feishu_card() {
        let alert = AlertMessage::new(
            AlertLevel::Critical,
            "Test Alert".to_string(),
            "component".to_string(),
        );
        
        let card = alert.to_feishu_card();
        assert_eq!(card["msg_type"], "interactive");
    }
    
    #[tokio::test]
    async fn test_notifier_create() {
        let config = L3Config::default();
        let audit_log = AuditLogger::new(AuditConfig::default());
        let notifier = Notifier::new(config, audit_log);
        
        let alert = AlertMessage::new(
            AlertLevel::Info,
            "Test".to_string(),
            "test".to_string(),
        );
        
        // 不配置 webhook，应该仍然能创建
        assert!(notifier.feishu_webhook.is_none());
    }
    
    #[test]
    fn test_alert_from_plan() {
        let config = L3Config::default();
        let audit_log = AuditLogger::new(AuditConfig::default());
        let notifier = Notifier::new(config, audit_log);
        
        let plan = RecoveryPlan::new(RecoveryLevel::L3HumanIntervention, "test".to_string());
        let alert = notifier.alert_from_plan(&plan, "test reason");
        
        assert_eq!(alert.level, AlertLevel::Emergency);
        assert!(alert.title.contains("test"));
    }
}