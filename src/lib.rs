// NewClaw - Next-gen AI Agent framework

pub mod core;
pub mod channels;
pub mod config;
pub mod cli;
pub mod llm;
pub mod gateway;

// Re-export main types
pub use core::AgentEngine;
pub use core::{ContextManager, ContextConfig, ContextChunk};
pub use core::{StrategyEngine, Strategy, StrategyType};

/// NewClaw version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO: Add LLM module when ready
// pub mod llm;
