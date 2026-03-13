// Memory Module - v0.7.0
//
// 多 Agent 记忆共享系统
// + 约束保护系统 (v0.7.0)

pub mod shared;

// v0.7.0 - 约束保护系统
pub mod constraint;

pub use shared::{
    UserId, MemoryEntry, MemoryType, UserMemory, 
    SharedMemoryManager, SharedMemoryConfig, GlobalMemoryStats,
};

// v0.7.0 - 约束导出
pub use constraint::{
    Constraint, ConstraintType, ConstraintScope, ConstraintSource,
    ConstraintManager, ConstraintConflict, ConflictResolution,
    ConstraintInjector, ConstraintYaml, ConstraintMetadata,
};
