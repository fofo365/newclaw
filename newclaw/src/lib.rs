// NewClaw v0.7.0 - Production-ready AI Agent Framework
// 
// Core philosophy: Stability + Security + Enterprise
// 
// Key features:
// - 6-Layer Configuration Architecture with Hot Reload
// - DAG Workflow Engine
// - Task Scheduling (Cron, Delayed Queue, Event Triggers)
// - NamingEngine Matching Engine
// - ABAC Permission Engine (Attribute-Based Access Control)
// - Tool Execution Layer (reliable tool calls)
// - Local Model Support (Ollama integration)
// - Feishu Integration (enterprise IM)
// - Security Layer (API Key + JWT + RBAC)
// - Multi-channel Messaging (QQ/Telegram/Discord)

pub mod config;
pub mod tool;
pub mod provider;
pub mod model;
pub mod integration;
pub mod channel;
pub mod abac;

// Re-exports from config module
pub use config::{
    Config, ConfigError, ConfigResult,
    ConfigWatcher, WatchEvent,
    ConfigHotReloadManager, ConfigVersion, ConfigDiff,
    ConfigNotificationManager, ConfigChangeEvent, ConfigSubscriber,
    ConfigLayer, ConfigScope, LayeredConfig,
    ConfigMerger, ConfigContext,
};

// Re-exports from abac module
pub use abac::{
    // Attribute types
    Attribute, AttributeBag, AttributeCategory, AttributeError, AttributeResolver,
    AttributeValue, AttributeValueType, CompositeAttributeResolver, StaticAttributeResolver,
    // Policy types
    Condition, ConditionOperator, Effect, LogicalOperator, Policy, PolicyCombiningAlgorithm,
    PolicyError, PolicySet, Rule,
    // Evaluator types
    AuthzRequest, EvaluationResult, PolicyEvaluator, PolicyEvaluatorBuilder, PolicyMatch,
    PolicyMatcher,
    // Decision types
    Advice, AuditEntry, AuditLog, AuditStats, Decision, DecisionEngine, DecisionEngineBuilder,
    DecisionReason, DecisionResult, Obligation,
};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Tool error: {0}")]
    Tool(#[from] tool::ToolError),
    
    #[error("Provider error: {0}")]
    Provider(#[from] provider::AdapterError),
    
    #[error("Model error: {0}")]
    Model(#[from] model::ModelError),
    
    #[error("Integration error: {0}")]
    Integration(#[from] integration::IntegrationError),
    
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// NewClaw version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// NewClaw name
pub const NAME: &str = env!("CARGO_PKG_NAME");