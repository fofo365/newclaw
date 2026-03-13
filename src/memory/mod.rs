// Memory Module - v0.7.0
//
// 多 Agent 记忆共享系统
// + 约束保护系统 (v0.7.0)
// + 统一存储抽象 (v0.7.0)
// + FTS5 全文索引 (v0.7.0)
// + MMR 去重 (v0.7.0)
// + 文件级记忆持久化 (v0.7.0)
// + 分层摘要机制 (v0.7.0)

pub mod shared;
pub mod constraint;
pub mod storage;
pub mod file_memory;
pub mod summary;

pub use shared::{
    UserId, MemoryEntry, MemoryType, UserMemory, 
    SharedMemoryManager, SharedMemoryConfig, GlobalMemoryStats,
};

pub use constraint::{
    Constraint, ConstraintType, ConstraintScope, ConstraintSource,
    ConstraintManager, ConstraintConflict, ConflictResolution,
    ConstraintInjector, ConstraintYaml, ConstraintMetadata,
};

pub use storage::{
    MemoryStorage, StorageStats, StorageConfig,
    SQLiteMemoryStorage, HybridSearchConfig, HybridSearchResult,
    MMRConfig, mmr_diversify,
};

pub use file_memory::{
    FileMemoryManager, MemoryFile, MemorySection, Decision,
};

pub use summary::{
    SummaryTree, SummaryNode, SummaryConfig, SummaryStats,
    HierarchicalSummaryManager, SummaryMessage, SummaryAction,
};