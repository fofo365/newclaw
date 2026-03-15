//! Federated Memory Module - 联邦记忆模块
//!
//! 提供跨 Agent 的联邦记忆功能
//! 
//! # 功能
//! 
//! - **联邦协议**: Agent 间通信协议定义
//! - **分布式存储**: 跨节点记忆存储
//! - **缓存机制**: 本地缓存优化查询性能
//! - **复制同步**: 数据复制和一致性保证
//! - **联邦查询**: 跨节点查询和结果聚合
//!
//! # 架构
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Federated Memory System                      │
//! │                                                                 │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
//! │  │   Protocol  │  │   Message   │  │  Encryption │            │
//! │  │   协议层    │  │   消息层    │  │   加密层    │            │
//! │  └─────────────┘  └─────────────┘  └─────────────┘            │
//! │           │              │              │                      │
//! │           └──────────────┼──────────────┘                      │
//! │                          │                                      │
//! │  ┌───────────────────────────────────────────────────────┐    │
//! │  │                 Distributed Storage                    │    │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │    │
//! │  │  │ Storage │ │  Cache  │ │  Repl.  │ │ Conflict│     │    │
//! │  │  │  存储   │ │  缓存   │ │  复制   │ │  解决   │     │    │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘     │    │
//! │  └───────────────────────────────────────────────────────┘    │
//! │                          │                                      │
//! │  ┌───────────────────────────────────────────────────────┐    │
//! │  │                 Query & Aggregation                    │    │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐     │    │
//! │  │  │  Query  │ │ Router  │ │  Aggr.  │ │  Dedup  │     │    │
//! │  │  │  查询   │ │  路由   │ │  聚合   │ │  去重   │     │    │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘     │    │
//! │  └───────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use newclaw::federated::{
//!     FederatedConfig, NodeRegistry, FederatedQueryEngine,
//!     DistributedStorage, LocalDistributedStorage,
//! };
//!
//! // 创建联邦配置
//! let config = FederatedConfig::default();
//!
//! // 创建节点注册表
//! let registry = NodeRegistry::new(config.clone());
//!
//! // 创建分布式存储
//! let storage_config = DistributedStorageConfig::default();
//! let storage = LocalDistributedStorage::new(storage_config);
//!
//! // 创建查询引擎
//! let router = Arc::new(QueryRouter::new());
//! let engine = FederatedQueryEngine::new(router, /* ... */);
//!
//! // 执行联邦查询
//! let query = FederatedQuery::new("search query".to_string());
//! let response = engine.execute(query).await?;
//! ```
//!
//! v0.7.0 P1 - 联邦记忆

pub mod protocol;
pub mod message;
pub mod encryption;
pub mod storage;
pub mod cache;
pub mod replication;
pub mod query;
pub mod aggregation;

// ============================================================================
// 协议层重导出
// ============================================================================

pub use protocol::{
    // 错误
    FederatedError, FederatedResult,
    
    // 节点
    NodeId, NodeState, NodeInfo, NodeCapabilities,
    NodeRegistry, NodeEvent, FederatedConfig,
    
    // 协议
    FederatedProtocol, FederatedMessage, FederatedMessageType,
    PROTOCOL_VERSION, check_version_compatibility,
    
    // 发现和注册
    DiscoveryMessage, DiscoveryType,
    RegisterRequest, RegisterResponse,
    
    // 心跳
    HeartbeatMessage, HeartbeatStats,
    
    // 查询和同步
    MemoryQueryRequest, MemoryQueryResponse, QueryType, QueryFilters,
    MemorySyncMessage, SyncType, SyncEntry, SyncOperation,
    
    // 远程条目
    RemoteMemoryEntry,
    
    // 消息
    AckMessage, ErrorMessage,
};

// ============================================================================
// 消息层重导出
// ============================================================================

pub use message::{
    // 错误
    MessageError, MessageResult,
    
    // 消息结构
    MessageHeader, MessageEnvelope, MessagePayload,
    MessageBuilder, MessageValidator,
    
    // 负载
    DiscoveryPayload, NodeInfoPayload, NodeCapabilitiesPayload,
    RegisterRequestPayload, RegisterResponsePayload,
    HeartbeatPayload, HeartbeatStatsPayload,
    MemoryQueryPayload, MemoryResponsePayload, MemoryEntryPayload,
    MemorySyncPayload, SyncEntryPayload,
    AckPayload, ErrorPayload,
    QueryFiltersPayload,
    
    // 路由
    MessageRoute,
};

// ============================================================================
// 加密层重导出
// ============================================================================

pub use encryption::{
    // 错误
    EncryptionError, EncryptionResult,
    
    // 密钥
    KeyId, KeyType, KeyUsage, KeyStatus,
    SymmetricKey, KeyPair,
    
    // 加密
    EncryptionConfig, EncryptionAlgorithm, EncryptedData,
    Encryptor, Signer, MessageSignature, SignatureAlgorithm,
    
    // 密钥管理
    KeyManager,
    
    // 安全通道
    EncryptionSession, SecureChannel, ChannelState,
};

// ============================================================================
// 存储层重导出
// ============================================================================

pub use storage::{
    // 错误
    StorageError, StorageResult,
    
    // 条目
    DistributedMemoryEntry, VectorClock,
    
    // 冲突解决
    ConflictResolutionStrategy, ConflictRecord, ConflictStatus,
    ConflictResolution, ConflictResolver,
    
    // 存储
    DistributedStorage, DistributedStorageConfig, DistributedStorageStats,
    LocalDistributedStorage, StorageEvent,
};

// ============================================================================
// 缓存层重导出
// ============================================================================

pub use cache::{
    // 配置
    CacheConfig, CacheStrategy,
    
    // 条目
    CacheEntry, CacheStats,
    
    // 缓存
    MemoryCache, TwoLevelCache,
    
    // 工具
    CacheKeyBuilder, query_hash,
};

// ============================================================================
// 复制层重导出
// ============================================================================

pub use replication::{
    // 配置
    ReplicationConfig,
    
    // 复制
    ReplicationState, ReplicationEntry, ReplicationManager,
    ReplicationStats, ReplicationEvent,
    
    // 提示移交
    Hint, HintStore,
    
    // 同步
    SyncCoordinator, SyncResult,
    
    // 策略
    ReplicationStrategy, QuorumReplicationStrategy,
};

// ============================================================================
// 查询层重导出
// ============================================================================

pub use query::{
    // 错误
    QueryError, QueryResult,
    
    // 查询
    FederatedQuery, QueryType as QueryTypeAlias, QueryFilters as QueryFiltersAlias,
    TimeRange, FederatedQueryResponse, NodeQueryResponse,
    
    // 结果
    AggregatedResult,
    
    // 路由
    QueryRouter,
    
    // 执行
    QueryExecutor, DefaultQueryExecutor, QueryEvent,
    
    // 引擎
    FederatedQueryEngine,
    
    // 缓存
    QueryCache,
};

// ============================================================================
// 聚合层重导出
// ============================================================================

pub use aggregation::{
    // 配置
    AggregationConfig,
    
    // 策略
    FusionStrategy, DeduplicationStrategy, SortStrategy,
    
    // 聚合
    ResultAggregator, AggregationOutput,
    
    // 归一化
    ScoreNormalizer,
    
    // 合并
    ResultMerger,
};

// ============================================================================
// 便捷函数
// ============================================================================

/// 创建默认联邦配置
pub fn default_federated_config() -> FederatedConfig {
    FederatedConfig::default()
}

/// 创建默认分布式存储配置
pub fn default_storage_config() -> storage::DistributedStorageConfig {
    storage::DistributedStorageConfig::default()
}

/// 创建默认缓存配置
pub fn default_cache_config() -> CacheConfig {
    CacheConfig::default()
}

/// 创建默认复制配置
pub fn default_replication_config() -> ReplicationConfig {
    ReplicationConfig::default()
}

// ============================================================================
// 模块统计
// ============================================================================

/// 获取模块版本
pub const MODULE_VERSION: &str = "0.7.0";

/// 获取模块代码行数统计（约）
pub const MODULE_LINES: usize = 10_000;

/// 获取模块功能列表
pub const fn features() -> &'static [&'static str] {
    &[
        "federated_protocol",
        "distributed_storage",
        "local_cache",
        "replication_sync",
        "federated_query",
        "result_aggregation",
        "encryption",
        "conflict_resolution",
        "hinted_handoff",
        "read_repair",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_module_version() {
        assert_eq!(MODULE_VERSION, "0.7.0");
    }
    
    #[test]
    fn test_features_count() {
        assert_eq!(features().len(), 10);
    }
    
    #[test]
    fn test_default_configs() {
        let _ = default_federated_config();
        let _ = default_storage_config();
        let _ = default_cache_config();
        let _ = default_replication_config();
    }
}