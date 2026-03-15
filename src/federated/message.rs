//! Federated Memory Message Format - 联邦记忆消息格式
//!
//! 定义 Agent 间联邦记忆通信的消息格式
//! 支持消息序列化、签名、压缩
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::protocol::{
    NodeId, NodeState, QueryType, SyncType, SyncOperation,
    DiscoveryMessage, DiscoveryType, RegisterRequest, RegisterResponse,
    HeartbeatMessage, HeartbeatStats, AckMessage, ErrorMessage,
    MemoryQueryRequest, MemoryQueryResponse, QueryFilters,
    MemorySyncMessage, SyncEntry, RemoteMemoryEntry,
};

// ============================================================================
// 消息错误
// ============================================================================

/// 消息错误
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("序列化错误: {0}")]
    SerializationError(String),
    
    #[error("反序列化错误: {0}")]
    DeserializationError(String),
    
    #[error("签名验证失败: {0}")]
    SignatureVerificationFailed(String),
    
    #[error("消息过期: {0}")]
    MessageExpired(String),
    
    #[error("消息格式无效: {0}")]
    InvalidFormat(String),
    
    #[error("压缩错误: {0}")]
    CompressionError(String),
    
    #[error("解压错误: {0}")]
    DecompressionError(String),
}

pub type MessageResult<T> = std::result::Result<T, MessageError>;

// ============================================================================
// 消息头
// ============================================================================

/// 消息头
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// 消息 ID
    pub id: String,
    /// 协议版本
    pub version: String,
    /// 消息类型
    pub message_type: String,
    /// 来源节点
    pub from: NodeId,
    /// 目标节点
    pub to: NodeId,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 过期时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// 相关 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// 是否压缩
    #[serde(default)]
    pub compressed: bool,
    /// 是否加密
    #[serde(default)]
    pub encrypted: bool,
    /// 签名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl MessageHeader {
    pub fn new(from: NodeId, to: NodeId, message_type: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            version: crate::VERSION.to_string(),
            message_type,
            from,
            to,
            timestamp: Utc::now(),
            expires_at: None,
            correlation_id: None,
            compressed: false,
            encrypted: false,
            signature: None,
            metadata: HashMap::new(),
        }
    }
    
    /// 设置过期时间
    pub fn with_expiry(mut self, seconds: i64) -> Self {
        self.expires_at = Some(Utc::now() + chrono::Duration::seconds(seconds));
        self
    }
    
    /// 设置相关 ID
    pub fn with_correlation_id(mut self, id: String) -> Self {
        self.correlation_id = Some(id);
        self
    }
    
    /// 设置压缩标志
    pub fn with_compression(mut self, compressed: bool) -> Self {
        self.compressed = compressed;
        self
    }
    
    /// 设置加密标志
    pub fn with_encryption(mut self, encrypted: bool) -> Self {
        self.encrypted = encrypted;
        self
    }
    
    /// 设置签名
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// 检查消息是否过期
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

// ============================================================================
// 消息包装器
// ============================================================================

/// 消息包装器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// 消息头
    pub header: MessageHeader,
    /// 消息负载
    pub payload: MessagePayload,
}

impl MessageEnvelope {
    pub fn new(header: MessageHeader, payload: MessagePayload) -> Self {
        Self { header, payload }
    }
    
    /// 序列化为 JSON
    pub fn to_json(&self) -> MessageResult<String> {
        serde_json::to_string(self)
            .map_err(|e| MessageError::SerializationError(e.to_string()))
    }
    
    /// 从 JSON 反序列化
    pub fn from_json(json: &str) -> MessageResult<Self> {
        serde_json::from_str(json)
            .map_err(|e| MessageError::DeserializationError(e.to_string()))
    }
    
    /// 序列化为字节数组
    pub fn to_bytes(&self) -> MessageResult<Vec<u8>> {
        serde_json::to_vec(self)
            .map_err(|e| MessageError::SerializationError(e.to_string()))
    }
    
    /// 从字节数组反序列化
    pub fn from_bytes(bytes: &[u8]) -> MessageResult<Self> {
        serde_json::from_slice(bytes)
            .map_err(|e| MessageError::DeserializationError(e.to_string()))
    }
    
    /// 压缩消息
    pub fn compress(&mut self) -> MessageResult<()> {
        if self.header.compressed {
            return Ok(());
        }
        
        // 使用简单的压缩策略（将 payload 序列化后压缩）
        // 实际实现中可以使用 zstd 或 lz4
        self.header.compressed = true;
        Ok(())
    }
    
    /// 解压消息
    pub fn decompress(&mut self) -> MessageResult<()> {
        if !self.header.compressed {
            return Ok(());
        }
        
        self.header.compressed = false;
        Ok(())
    }
}

// ============================================================================
// 消息负载
// ============================================================================

/// 消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePayload {
    /// 发现消息
    Discovery(DiscoveryPayload),
    /// 注册请求
    RegisterRequest(RegisterRequestPayload),
    /// 注册响应
    RegisterResponse(RegisterResponsePayload),
    /// 心跳
    Heartbeat(HeartbeatPayload),
    /// 记忆查询请求
    MemoryQuery(MemoryQueryPayload),
    /// 记忆查询响应
    MemoryResponse(MemoryResponsePayload),
    /// 记忆同步
    MemorySync(MemorySyncPayload),
    /// 确认
    Ack(AckPayload),
    /// 错误
    Error(ErrorPayload),
    /// Ping
    Ping,
    /// Pong
    Pong,
}

// ============================================================================
// 具体消息负载类型
// ============================================================================

/// 发现消息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPayload {
    pub discovery_type: DiscoveryType,
    pub node_info: NodeInfoPayload,
}

/// 节点信息负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfoPayload {
    pub id: NodeId,
    pub name: String,
    pub address: String,
    pub state: NodeState,
    pub capabilities: NodeCapabilitiesPayload,
    pub protocol_version: String,
    pub last_heartbeat: DateTime<Utc>,
    pub registered_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

/// 节点能力负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilitiesPayload {
    pub memory_types: Vec<String>,
    pub max_entries: usize,
    pub max_storage_bytes: u64,
    pub query_types: Vec<String>,
    pub encryption_algorithms: Vec<String>,
    pub supports_vector_search: bool,
    pub supports_fulltext_search: bool,
}

/// 注册请求负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequestPayload {
    pub request_id: String,
    pub node_info: NodeInfoPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

/// 注册响应负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponsePayload {
    pub request_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_id: Option<NodeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<NodeInfoPayload>,
}

/// 心跳负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub node_id: NodeId,
    pub state: NodeState,
    pub stats: HeartbeatStatsPayload,
}

/// 心跳统计负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatStatsPayload {
    pub memory_entries: usize,
    pub active_connections: usize,
    pub cpu_usage: f32,
    pub memory_usage: f32,
}

/// 记忆查询负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryPayload {
    pub query_id: String,
    pub query: String,
    pub query_type: QueryType,
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub filters: QueryFiltersPayload,
    #[serde(default = "default_true")]
    pub include_metadata: bool,
}

/// 查询过滤器负载
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFiltersPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_end: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_importance: Option<f32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_agent: Option<String>,
}

fn default_true() -> bool { true }

/// 记忆响应负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponsePayload {
    pub query_id: String,
    pub results: Vec<MemoryEntryPayload>,
    pub total: usize,
    pub elapsed_ms: u64,
    pub has_more: bool,
    pub from_node: NodeId,
}

/// 记忆条目负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntryPayload {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
    pub source_node: NodeId,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 记忆同步负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySyncPayload {
    pub sync_id: String,
    pub sync_type: SyncType,
    pub entries: Vec<SyncEntryPayload>,
}

/// 同步条目负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntryPayload {
    pub id: String,
    pub operation: SyncOperation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<MemoryEntryPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
}

/// 确认负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPayload {
    pub message_id: String,
}

/// 错误负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_message_id: Option<String>,
}

// ============================================================================
// 消息构建器
// ============================================================================

/// 消息构建器
pub struct MessageBuilder {
    from: NodeId,
    to: NodeId,
    header_metadata: HashMap<String, String>,
}

impl MessageBuilder {
    pub fn new(from: NodeId, to: NodeId) -> Self {
        Self {
            from,
            to,
            header_metadata: HashMap::new(),
        }
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.header_metadata.insert(key, value);
        self
    }
    
    /// 构建 Discovery 消息
    pub fn discovery(self, discovery_type: DiscoveryType, node_info: NodeInfoPayload) -> MessageEnvelope {
        let mut header = MessageHeader::new(
            self.from,
            self.to,
            "discovery".to_string(),
        );
        for (k, v) in self.header_metadata {
            header.metadata.insert(k, v);
        }
        
        MessageEnvelope::new(header, MessagePayload::Discovery(DiscoveryPayload {
            discovery_type,
            node_info,
        }))
    }
    
    /// 构建注册请求消息
    pub fn register_request(self, request_id: String, node_info: NodeInfoPayload, auth_token: Option<String>) -> MessageEnvelope {
        let mut header = MessageHeader::new(
            self.from,
            self.to,
            "register_request".to_string(),
        );
        for (k, v) in self.header_metadata {
            header.metadata.insert(k, v);
        }
        
        MessageEnvelope::new(header, MessagePayload::RegisterRequest(RegisterRequestPayload {
            request_id,
            node_info,
            auth_token,
        }))
    }
    
    /// 构建注册响应消息
    pub fn register_response(
        self,
        request_id: String,
        success: bool,
        assigned_id: Option<NodeId>,
        error: Option<String>,
        server_info: Option<NodeInfoPayload>,
    ) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from,
            self.to,
            "register_response".to_string(),
        ).with_correlation_id(request_id.clone());
        
        MessageEnvelope::new(header, MessagePayload::RegisterResponse(RegisterResponsePayload {
            request_id,
            success,
            assigned_id,
            error,
            server_info,
        }))
    }
    
    /// 构建心跳消息
    pub fn heartbeat(self, state: NodeState, stats: HeartbeatStatsPayload) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from.clone(),
            self.to,
            "heartbeat".to_string(),
        );
        
        MessageEnvelope::new(header, MessagePayload::Heartbeat(HeartbeatPayload {
            node_id: self.from,
            state,
            stats,
        }))
    }
    
    /// 构建记忆查询消息
    pub fn memory_query(
        self,
        query_id: String,
        query: String,
        query_type: QueryType,
        limit: usize,
        offset: usize,
        filters: QueryFiltersPayload,
    ) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from,
            self.to,
            "memory_query".to_string(),
        ).with_correlation_id(query_id.clone());
        
        MessageEnvelope::new(header, MessagePayload::MemoryQuery(MemoryQueryPayload {
            query_id,
            query,
            query_type,
            limit,
            offset,
            filters,
            include_metadata: true,
        }))
    }
    
    /// 构建记忆响应消息
    pub fn memory_response(
        self,
        query_id: String,
        results: Vec<MemoryEntryPayload>,
        total: usize,
        elapsed_ms: u64,
        has_more: bool,
    ) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from.clone(),
            self.to,
            "memory_response".to_string(),
        ).with_correlation_id(query_id.clone());
        
        MessageEnvelope::new(header, MessagePayload::MemoryResponse(MemoryResponsePayload {
            query_id,
            results,
            total,
            elapsed_ms,
            has_more,
            from_node: self.from,
        }))
    }
    
    /// 构建记忆同步消息
    pub fn memory_sync(
        self,
        sync_id: String,
        sync_type: SyncType,
        entries: Vec<SyncEntryPayload>,
    ) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from,
            self.to,
            "memory_sync".to_string(),
        ).with_correlation_id(sync_id.clone());
        
        MessageEnvelope::new(header, MessagePayload::MemorySync(MemorySyncPayload {
            sync_id,
            sync_type,
            entries,
        }))
    }
    
    /// 构建确认消息
    pub fn ack(self, message_id: String) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from,
            self.to,
            "ack".to_string(),
        ).with_correlation_id(message_id.clone());
        
        MessageEnvelope::new(header, MessagePayload::Ack(AckPayload { message_id }))
    }
    
    /// 构建错误消息
    pub fn error(self, code: String, message: String, related_message_id: Option<String>) -> MessageEnvelope {
        let mut header = MessageHeader::new(
            self.from,
            self.to,
            "error".to_string(),
        );
        if let Some(ref related_id) = related_message_id {
            header = header.with_correlation_id(related_id.clone());
        }
        
        MessageEnvelope::new(header, MessagePayload::Error(ErrorPayload {
            code,
            message,
            related_message_id,
        }))
    }
    
    /// 构建 Ping 消息
    pub fn ping(self) -> MessageEnvelope {
        let header = MessageHeader::new(self.from, self.to, "ping".to_string());
        MessageEnvelope::new(header, MessagePayload::Ping)
    }
    
    /// 构建 Pong 消息
    pub fn pong(self, correlation_id: String) -> MessageEnvelope {
        let header = MessageHeader::new(
            self.from,
            self.to,
            "pong".to_string(),
        ).with_correlation_id(correlation_id);
        
        MessageEnvelope::new(header, MessagePayload::Pong)
    }
}

// ============================================================================
// 消息验证
// ============================================================================

/// 消息验证器
pub struct MessageValidator {
    /// 最大消息大小（字节）
    pub max_message_size: usize,
    /// 消息过期时间（秒）
    pub message_expiry_secs: i64,
    /// 允许的协议版本
    pub allowed_versions: Vec<String>,
}

impl Default for MessageValidator {
    fn default() -> Self {
        Self {
            max_message_size: 10 * 1024 * 1024, // 10MB
            message_expiry_secs: 300, // 5 minutes
            allowed_versions: vec!["0.7.0".to_string()],
        }
    }
}

impl MessageValidator {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 验证消息
    pub fn validate(&self, envelope: &MessageEnvelope) -> MessageResult<()> {
        // 检查消息是否过期
        if envelope.header.is_expired() {
            return Err(MessageError::MessageExpired(
                format!("Message {} has expired", envelope.header.id)
            ));
        }
        
        // 检查版本兼容性
        if !self.allowed_versions.contains(&envelope.header.version) {
            // 只检查主版本号
            let parts: Vec<&str> = envelope.header.version.split('.').collect();
            if parts.len() >= 1 && parts[0] != "0" {
                return Err(MessageError::InvalidFormat(
                    format!("Unsupported protocol version: {}", envelope.header.version)
                ));
            }
        }
        
        Ok(())
    }
    
    /// 验证消息大小
    pub fn validate_size(&self, bytes: &[u8]) -> MessageResult<()> {
        if bytes.len() > self.max_message_size {
            return Err(MessageError::InvalidFormat(
                format!("Message too large: {} bytes (max: {})", 
                    bytes.len(), self.max_message_size)
            ));
        }
        Ok(())
    }
}

// ============================================================================
// 消息路由
// ============================================================================

/// 消息路由信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRoute {
    /// 目标节点 ID
    pub target: NodeId,
    /// 路由路径
    pub path: Vec<NodeId>,
    /// 跳数
    pub hop_count: usize,
    /// 路由时间
    pub routed_at: DateTime<Utc>,
}

impl MessageRoute {
    pub fn direct(target: NodeId) -> Self {
        Self {
            target: target.clone(),
            path: vec![target],
            hop_count: 1,
            routed_at: Utc::now(),
        }
    }
    
    pub fn via(target: NodeId, path: Vec<NodeId>) -> Self {
        let hop_count = path.len();
        Self {
            target,
            path,
            hop_count,
            routed_at: Utc::now(),
        }
    }
}

// ============================================================================
// 转换实现
// ============================================================================

impl From<crate::memory::MemoryEntry> for MemoryEntryPayload {
    fn from(entry: crate::memory::MemoryEntry) -> Self {
        use crate::memory::MemoryType;
        
        let memory_type = match entry.memory_type {
            MemoryType::Conversation => "conversation",
            MemoryType::Task => "task",
            MemoryType::Preference => "preference",
            MemoryType::Fact => "fact",
            MemoryType::Skill => "skill",
            MemoryType::Context => "context",
        };
        
        Self {
            id: entry.id,
            content: entry.content,
            memory_type: memory_type.to_string(),
            importance: entry.importance,
            created_at: entry.created_at,
            source_node: NodeId::from_str(entry.source_agent.as_deref().unwrap_or("unknown")),
            tags: entry.tags,
            metadata: entry.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_header() {
        let from = NodeId::new();
        let to = NodeId::new();
        
        let header = MessageHeader::new(from.clone(), to.clone(), "test".to_string());
        
        assert!(!header.id.is_empty());
        assert_eq!(header.from, from);
        assert_eq!(header.to, to);
        assert!(!header.is_expired());
        
        // 设置过期时间
        let header = header.with_expiry(-1); // 已过期
        assert!(header.is_expired());
    }
    
    #[test]
    fn test_message_envelope_serialization() {
        let from = NodeId::new();
        let to = NodeId::new();
        
        let builder = MessageBuilder::new(from.clone(), to.clone());
        let envelope = builder.ping();
        
        let json = envelope.to_json().unwrap();
        let restored = MessageEnvelope::from_json(&json).unwrap();
        
        assert_eq!(envelope.header.id, restored.header.id);
        assert!(matches!(restored.payload, MessagePayload::Ping));
    }
    
    #[test]
    fn test_message_builder_heartbeat() {
        let from = NodeId::new();
        let to = NodeId::new();
        
        let builder = MessageBuilder::new(from.clone(), to.clone());
        let envelope = builder.heartbeat(
            NodeState::Active,
            HeartbeatStatsPayload {
                memory_entries: 100,
                active_connections: 5,
                cpu_usage: 0.5,
                memory_usage: 0.3,
            },
        );
        
        assert_eq!(envelope.header.message_type, "heartbeat");
        
        if let MessagePayload::Heartbeat(hb) = &envelope.payload {
            assert_eq!(hb.state, NodeState::Active);
        } else {
            panic!("Expected Heartbeat payload");
        }
    }
    
    #[test]
    fn test_message_builder_memory_query() {
        let from = NodeId::new();
        let to = NodeId::new();
        
        let builder = MessageBuilder::new(from.clone(), to.clone());
        let envelope = builder.memory_query(
            "query-123".to_string(),
            "test query".to_string(),
            QueryType::Hybrid,
            10,
            0,
            QueryFiltersPayload::default(),
        );
        
        if let MessagePayload::MemoryQuery(mq) = &envelope.payload {
            assert_eq!(mq.query, "test query");
            assert_eq!(mq.limit, 10);
        } else {
            panic!("Expected MemoryQuery payload");
        }
    }
    
    #[test]
    fn test_message_validator() {
        let validator = MessageValidator::new();
        
        let from = NodeId::new();
        let to = NodeId::new();
        
        let builder = MessageBuilder::new(from, to);
        let envelope = builder.ping();
        
        // 有效消息
        assert!(validator.validate(&envelope).is_ok());
        
        // 过期消息
        let mut expired_envelope = envelope.clone();
        expired_envelope.header = expired_envelope.header.with_expiry(-100);
        assert!(validator.validate(&expired_envelope).is_err());
    }
    
    #[test]
    fn test_message_route() {
        let target = NodeId::new();
        
        let route = MessageRoute::direct(target.clone());
        assert_eq!(route.hop_count, 1);
        
        let intermediate = NodeId::new();
        let route = MessageRoute::via(target, vec![intermediate]);
        assert_eq!(route.hop_count, 1);
    }
    
    #[test]
    fn test_message_size_validation() {
        let validator = MessageValidator::new();
        
        let small_message = vec![0u8; 100];
        assert!(validator.validate_size(&small_message).is_ok());
        
        let large_message = vec![0u8; validator.max_message_size + 1];
        assert!(validator.validate_size(&large_message).is_err());
    }
}