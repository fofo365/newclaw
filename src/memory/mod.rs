// Memory Module - v0.7.0
//
// 多 Agent 记忆共享系统
// + 约束保护系统 (v0.7.0)
// + 统一存储抽象 (v0.7.0)
// + FTS5 全文索引 (v0.7.0)
// + MMR 去重 (v0.7.0)
// + 文件级记忆持久化 (v0.7.0)
// + 分层摘要机制 (v0.7.0)
// + 向量存储 (v0.7.0 P2)
// + 代码库嵌入索引 (v0.7.0 P2)
// + 多层隔离机制 (v0.7.0) - 用户/通道/Agent/命名空间

pub mod shared;
pub mod constraint;
pub mod storage;
pub mod storage_impl;
pub mod file_memory;
pub mod summary;
pub mod vector_store;
pub mod code_embedding;

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
    HybridSearchConfig, HybridSearchResult,
    MMRConfig, mmr_diversify,
    MemoryScope,
};

pub use storage_impl::SQLiteMemoryStorage;

pub use file_memory::{
    FileMemoryManager, MemoryFile, MemorySection, Decision,
};

pub use summary::{
    SummaryTree, SummaryNode, SummaryConfig, SummaryStats,
    HierarchicalSummaryManager, SummaryMessage, SummaryAction,
};

pub use vector_store::{
    VectorStore, VectorStoreConfig, VectorSearchResult, VectorStoreStats,
    InMemoryVectorStore, SQLiteVectorStore,
    cosine_similarity, euclidean_distance, normalize_vector,
};

pub use code_embedding::{
    CodeIndex, CodeChunk, ChunkType, CodeIndexConfig, CodeIndexStats,
    CodeSearchResult,
};