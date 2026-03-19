// AuditLog - 审计日志系统
//
// DESIGN.md 定义的核心组件，用于记录系统操作和决策过程

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use tracing::info;

/// 审计事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// 用户消息接收
    MessageReceived,
    /// 消息处理完成
    MessageProcessed,
    /// 工具调用
    ToolInvoked,
    /// 工具调用结果
    ToolResult,
    /// LLM 调用
    LLMCalled,
    /// 记忆存储
    MemoryStored,
    /// 记忆检索
    MemoryRetrieved,
    /// 策略应用
    StrategyApplied,
    /// 权限检查
    PermissionChecked,
    /// 配置变更
    ConfigChanged,
    /// 系统启动
    SystemStarted,
    /// 系统关闭
    SystemShutdown,
    /// 错误发生
    ErrorOccurred,
}

/// 审计事件严重级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 审计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    pub event_type: AuditEventType,
    /// 严重级别
    pub severity: AuditSeverity,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 来源（模块名）
    pub source: String,
    /// 用户 ID（如果有）
    pub user_id: Option<String>,
    /// 会话 ID（如果有）
    pub session_id: Option<String>,
    /// 事件描述
    pub description: String,
    /// 附加数据（JSON）
    pub metadata: Option<serde_json::Value>,
}

impl AuditEvent {
    /// 创建新的审计事件
    pub fn new(
        event_type: AuditEventType,
        source: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            severity: AuditSeverity::Info,
            timestamp: Utc::now(),
            source: source.into(),
            user_id: None,
            session_id: None,
            description: description.into(),
            metadata: None,
        }
    }
    
    /// 设置严重级别
    pub fn with_severity(mut self, severity: AuditSeverity) -> Self {
        self.severity = severity;
        self
    }
    
    /// 设置用户 ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    
    /// 设置会话 ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
    
    /// 设置附加数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// 审计日志存储
pub struct AuditLogStorage {
    db_path: PathBuf,
    events: Arc<RwLock<Vec<AuditEvent>>>,
    max_events: usize,
}

impl AuditLogStorage {
    /// 创建新的审计日志存储
    pub fn new(db_path: PathBuf) -> Result<Self> {
        // 确保目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Self {
            db_path,
            events: Arc::new(RwLock::new(Vec::new())),
            max_events: 10000,
        })
    }
    
    /// 记录审计事件
    pub async fn log(&self, event: AuditEvent) -> Result<()> {
        let mut events = self.events.write().await;
        
        // 格式化输出
        let severity = match &event.severity {
            AuditSeverity::Info => "INFO",
            AuditSeverity::Warning => "WARN",
            AuditSeverity::Error => "ERROR",
            AuditSeverity::Critical => "CRIT",
        };
        
        info!(
            "[AUDIT] {} {} {} - {}",
            severity,
            event.source,
            match &event.event_type {
                AuditEventType::MessageReceived => "MSG_RECV",
                AuditEventType::MessageProcessed => "MSG_PROC",
                AuditEventType::ToolInvoked => "TOOL_CALL",
                AuditEventType::ToolResult => "TOOL_RES",
                AuditEventType::LLMCalled => "LLM_CALL",
                AuditEventType::MemoryStored => "MEM_STORE",
                AuditEventType::MemoryRetrieved => "MEM_GET",
                AuditEventType::StrategyApplied => "STRATEGY",
                AuditEventType::PermissionChecked => "PERM",
                AuditEventType::ConfigChanged => "CONFIG",
                AuditEventType::SystemStarted => "START",
                AuditEventType::SystemShutdown => "STOP",
                AuditEventType::ErrorOccurred => "ERROR",
            },
            event.description
        );
        
        events.push(event);
        
        // 限制内存中的事件数量
        if events.len() > self.max_events {
            let start = events.len() - self.max_events;
            *events = events.split_off(start);
        }
        
        Ok(())
    }
    
    /// 查询审计事件
    pub async fn query(
        &self,
        event_type: Option<AuditEventType>,
        user_id: Option<&str>,
        limit: usize,
    ) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        
        events
            .iter()
            .filter(|e| {
                if let Some(ref et) = event_type {
                    if std::mem::discriminant(&e.event_type) != std::mem::discriminant(et) {
                        return false;
                    }
                }
                if let Some(uid) = user_id {
                    if e.user_id.as_deref() != Some(uid) {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// 持久化到文件
    pub async fn flush(&self) -> Result<()> {
        let events = self.events.read().await;
        let json = serde_json::to_string_pretty(&*events)?;
        tokio::fs::write(&self.db_path, json).await?;
        Ok(())
    }
}

/// 审计日志管理器
pub struct AuditLog {
    storage: Arc<AuditLogStorage>,
}

impl AuditLog {
    /// 创建新的审计日志管理器
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        let db_path = data_dir.join("audit_log.json");
        let storage = AuditLogStorage::new(db_path)?;
        
        Ok(Self {
            storage: Arc::new(storage),
        })
    }
    
    /// 记录事件
    pub async fn log(&self, event: AuditEvent) -> Result<()> {
        self.storage.log(event).await
    }
    
    /// 快捷方法：记录消息接收
    pub async fn log_message_received(
        &self,
        user_id: &str,
        session_id: &str,
        message_preview: &str,
    ) -> Result<()> {
        self.log(
            AuditEvent::new(
                AuditEventType::MessageReceived,
                "channel",
                format!("Received message: {}", message_preview.chars().take(50).collect::<String>()),
            )
            .with_user(user_id)
            .with_session(session_id)
        ).await
    }
    
    /// 快捷方法：记录工具调用
    pub async fn log_tool_invoked(
        &self,
        user_id: &str,
        tool_name: &str,
        args_summary: &str,
    ) -> Result<()> {
        self.log(
            AuditEvent::new(
                AuditEventType::ToolInvoked,
                "tools",
                format!("Tool '{}' called with: {}", tool_name, args_summary),
            )
            .with_user(user_id)
            .with_metadata(serde_json::json!({
                "tool": tool_name,
                "args": args_summary
            }))
        ).await
    }
    
    /// 快捷方法：记录错误
    pub async fn log_error(
        &self,
        source: &str,
        error: &str,
        user_id: Option<&str>,
    ) -> Result<()> {
        let mut event = AuditEvent::new(
            AuditEventType::ErrorOccurred,
            source,
            error,
        )
        .with_severity(AuditSeverity::Error);
        
        if let Some(uid) = user_id {
            event = event.with_user(uid);
        }
        
        self.log(event).await
    }
    
    /// 查询事件
    pub async fn query(
        &self,
        event_type: Option<AuditEventType>,
        user_id: Option<&str>,
        limit: usize,
    ) -> Vec<AuditEvent> {
        self.storage.query(event_type, user_id, limit).await
    }
    
    /// 持久化
    pub async fn flush(&self) -> Result<()> {
        self.storage.flush().await
    }
}

// 实现全局审计日志实例
use std::sync::OnceLock;

static GLOBAL_AUDIT_LOG: OnceLock<Arc<AuditLog>> = OnceLock::new();

/// 获取全局审计日志实例
pub fn global_audit() -> Option<Arc<AuditLog>> {
    GLOBAL_AUDIT_LOG.get().cloned()
}

/// 初始化全局审计日志
pub fn init_global_audit(data_dir: PathBuf) -> Result<()> {
    let audit = AuditLog::new(data_dir)?;
    GLOBAL_AUDIT_LOG.set(Arc::new(audit)).map_err(|_| anyhow::anyhow!("AuditLog already initialized"))?;
    Ok(())
}