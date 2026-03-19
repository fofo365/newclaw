// Core Module - v0.2.0 + v0.6.0

pub mod agent;
pub mod context;
pub mod strategy;
pub mod llm;
pub mod isolation;

// v0.6.0 - 智慧主控增强
pub mod heartbeat_reporter;
pub mod self_check;
pub mod degraded_mode;

// Re-export main types
pub use agent::AgentEngine;
pub use context::{ContextManager, ContextConfig, ContextChunk};
pub use strategy::{StrategyEngine, Strategy, StrategyType};
pub use isolation::{ContextIsolation, IsolationLevel};

// v0.6.0 re-exports
pub use heartbeat_reporter::{HeartbeatReporter, HeartbeatReporterConfig, HeartbeatReport, HealthState};
pub use self_check::{SelfChecker, SelfCheckConfig, SelfCheckResult, CheckItem};
pub use degraded_mode::{DegradedModeManager, DegradedModeConfig, DegradedState};
