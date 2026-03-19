//! Federated Memory Protocol - AGP 联邦记忆协议
//!
//! 定义 Agent 间联邦记忆通信协议
//! 支持节点发现、注册、消息传递和加密传输
//!
//! v0.7.0 P1 - 联邦记忆

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

// ============================================================================
// 错误定义
// ============================================================================

/// 联邦协议错误
#[derive(Debug, Error)]
pub enum FederatedError {
    #[error("节点未找到: {0}")]
    NodeNotFound(String),
    
    #[error("节点已存在: {0}")]
    NodeAlreadyExists(String),
    
    #[error("连接失败: {0}")]
    ConnectionFailed(String),
    
    #[error("认证失败: {0}")]
    AuthenticationFailed(String),
    
    #[error("加密错误: {0}")]
    EncryptionError(String),
    
    #[error("消息超时: {0}")]
    Timeout(String),
    
    #[error("无效消息: {0}")]
    InvalidMessage(String),
    
    #[error("版本不兼容: 本地 {local}, 远程 {remote}")]
    VersionMismatch { local: String, remote: String },
    
    #[error("网络错误: {0}")]
    NetworkError(String),
    
    #[error("内部错误: {0}")]
    Internal(String),
}

pub type FederatedResult<T> = std::result::Result<T, FederatedError>;

// ============================================================================
// 节点定义
// ============================================================================

/// 节点 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NodeId(String);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    /// 已发现，待认证
    Discovered,
    /// 认证中
    Authenticating,
    /// 活跃
    Active,
    /// 空闲
    Idle,
    /// 离线
    Offline,
    /// 禁用
    Disabled,
}

impl Default for NodeState {
    fn default() -> Self {
        Self::Discovered
    }
}

/// 节点能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// 支持的记忆类型
    pub memory_types: Vec<String>,
    /// 最大存储条目数
    pub max_entries: usize,
    /// 最大存储大小（字节）
    pub max_storage_bytes: u64,
    /// 支持的查询类型
    pub query_types: Vec<String>,
    /// 支持的加密算法
    pub encryption_algorithms: Vec<String>,
    /// 是否支持向量搜索
    pub supports_vector_search: bool,
    /// 是否支持全文搜索
    pub supports_fulltext_search: bool,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            memory_types: vec![
                "conversation".to_string(),
                "task".to_string(),
                "preference".to_string(),
                "fact".to_string(),
            ],
            max_entries: 100000,
            max_storage_bytes: 1024 * 1024 * 1024, // 1GB
            query_types: vec!["bm25".to_string(), "hybrid".to_string()],
            encryption_algorithms: vec!["aes-256-gcm".to_string()],
            supports_vector_search: true,
            supports_fulltext_search: true,
        }
    }
}

/// 节点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// 节点 ID
    pub id: NodeId,
    /// 节点名称
    pub name: String,
    /// 节点地址
    pub address: SocketAddr,
    /// 节点状态
    pub state: NodeState,
    /// 节点能力
    pub capabilities: NodeCapabilities,
    /// 协议版本
    pub protocol_version: String,
    /// 最后心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// 注册时间
    pub registered_at: DateTime<Utc>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// 公钥（用于加密）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
}

impl NodeInfo {
    pub fn new(name: String, address: SocketAddr) -> Self {
        Self {
            id: NodeId::new(),
            name,
            address,
            state: NodeState::Discovered,
            capabilities: NodeCapabilities::default(),
            protocol_version: crate::VERSION.to_string(),
            last_heartbeat: Utc::now(),
            registered_at: Utc::now(),
            metadata: HashMap::new(),
            public_key: None,
        }
    }
    
    /// 检查节点是否在线
    pub fn is_online(&self) -> bool {
        matches!(self.state, NodeState::Active | NodeState::Idle)
    }
    
    /// 更新心跳
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Utc::now();
        if self.state == NodeState::Idle {
            self.state = NodeState::Active;
        }
    }
    
    /// 检查心跳超时
    pub fn is_heartbeat_timeout(&self, timeout_secs: i64) -> bool {
        let elapsed = (Utc::now() - self.last_heartbeat).num_seconds();
        elapsed > timeout_secs
    }
}

// ============================================================================
// 协议配置
// ============================================================================

/// 联邦协议配置
#[derive(Debug, Clone)]
pub struct FederatedConfig {
    /// 本地节点 ID
    pub local_node_id: NodeId,
    /// 本地节点名称
    pub local_node_name: String,
    /// 监听地址
    pub listen_address: SocketAddr,
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    /// 心跳超时（秒）
    pub heartbeat_timeout_secs: i64,
    /// 最大连接数
    pub max_connections: usize,
    /// 消息超时（毫秒）
    pub message_timeout_ms: u64,
    /// 是否启用加密
    pub encryption_enabled: bool,
    /// 是否启用认证
    pub authentication_enabled: bool,
    /// 节点发现广播间隔（秒）
    pub discovery_interval_secs: u64,
}

impl Default for FederatedConfig {
    fn default() -> Self {
        Self {
            local_node_id: NodeId::new(),
            local_node_name: "newclaw-node".to_string(),
            listen_address: "0.0.0.0:7654".parse().unwrap(),
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 90,
            max_connections: 100,
            message_timeout_ms: 30000,
            encryption_enabled: true,
            authentication_enabled: true,
            discovery_interval_secs: 60,
        }
    }
}

// ============================================================================
// 节点发现和注册
// ============================================================================

/// 发现消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryMessage {
    /// 消息 ID
    pub id: String,
    /// 发送节点 ID
    pub from: NodeId,
    /// 发送节点信息
    pub node_info: NodeInfo,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 发现类型
    pub discovery_type: DiscoveryType,
}

/// 发现类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryType {
    /// 广播发现
    Broadcast,
    /// 响应发现
    Response,
    /// 主动注册
    Register,
    /// 注销
    Unregister,
}

impl DiscoveryMessage {
    pub fn new(from: NodeId, node_info: NodeInfo, discovery_type: DiscoveryType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            node_info,
            timestamp: Utc::now(),
            discovery_type,
        }
    }
}

/// 注册请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    /// 请求 ID
    pub id: String,
    /// 节点信息
    pub node_info: NodeInfo,
    /// 认证令牌
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl RegisterRequest {
    pub fn new(node_info: NodeInfo) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            node_info,
            auth_token: None,
            timestamp: Utc::now(),
        }
    }
    
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }
}

/// 注册响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    /// 请求 ID
    pub request_id: String,
    /// 是否成功
    pub success: bool,
    /// 分配的节点 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_id: Option<NodeId>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// 服务端节点信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<NodeInfo>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

impl RegisterResponse {
    pub fn success(request_id: String, assigned_id: NodeId, server_info: NodeInfo) -> Self {
        Self {
            request_id,
            success: true,
            assigned_id: Some(assigned_id),
            error: None,
            server_info: Some(server_info),
            timestamp: Utc::now(),
        }
    }
    
    pub fn failure(request_id: String, error: String) -> Self {
        Self {
            request_id,
            success: false,
            assigned_id: None,
            error: Some(error),
            server_info: None,
            timestamp: Utc::now(),
        }
    }
}

// ============================================================================
// 节点注册表
// ============================================================================

/// 节点注册表
pub struct NodeRegistry {
    /// 本地节点信息
    local_node: RwLock<NodeInfo>,
    /// 远程节点映射
    nodes: RwLock<HashMap<NodeId, NodeInfo>>,
    /// 配置
    config: FederatedConfig,
    /// 节点事件发送器
    event_tx: broadcast::Sender<NodeEvent>,
}

/// 节点事件
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// 节点上线
    NodeJoined(NodeInfo),
    /// 节点下线
    NodeLeft(NodeId),
    /// 节点状态变化
    NodeStateChanged { id: NodeId, old_state: NodeState, new_state: NodeState },
    /// 节点能力更新
    NodeCapabilitiesUpdated { id: NodeId, capabilities: NodeCapabilities },
}

impl NodeRegistry {
    pub fn new(config: FederatedConfig) -> Self {
        let local_node = NodeInfo::new(
            config.local_node_name.clone(),
            config.listen_address,
        );
        
        let (event_tx, _) = broadcast::channel(100);
        
        Self {
            local_node: RwLock::new(local_node),
            nodes: RwLock::new(HashMap::new()),
            config,
            event_tx,
        }
    }
    
    /// 获取本地节点信息
    pub async fn local_node(&self) -> NodeInfo {
        self.local_node.read().await.clone()
    }
    
    /// 获取所有节点
    pub async fn all_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }
    
    /// 获取在线节点
    pub async fn online_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.values()
            .filter(|n| n.is_online())
            .cloned()
            .collect()
    }
    
    /// 注册节点
    pub async fn register(&self, node: NodeInfo) -> FederatedResult<()> {
        let mut nodes = self.nodes.write().await;
        
        if nodes.contains_key(&node.id) {
            return Err(FederatedError::NodeAlreadyExists(node.id.to_string()));
        }
        
        let id = node.id.clone();
        nodes.insert(id.clone(), node.clone());
        
        // 发送事件
        let _ = self.event_tx.send(NodeEvent::NodeJoined(node));
        
        Ok(())
    }
    
    /// 注销节点
    pub async fn unregister(&self, id: &NodeId) -> FederatedResult<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.remove(id) {
            let _ = self.event_tx.send(NodeEvent::NodeLeft(node.id));
            Ok(())
        } else {
            Err(FederatedError::NodeNotFound(id.to_string()))
        }
    }
    
    /// 获取节点
    pub async fn get(&self, id: &NodeId) -> Option<NodeInfo> {
        let nodes = self.nodes.read().await;
        nodes.get(id).cloned()
    }
    
    /// 更新节点状态
    pub async fn update_state(&self, id: &NodeId, new_state: NodeState) -> FederatedResult<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(id) {
            let old_state = node.state;
            node.state = new_state;
            
            let _ = self.event_tx.send(NodeEvent::NodeStateChanged {
                id: id.clone(),
                old_state,
                new_state,
            });
            
            Ok(())
        } else {
            Err(FederatedError::NodeNotFound(id.to_string()))
        }
    }
    
    /// 更新心跳
    pub async fn update_heartbeat(&self, id: &NodeId) -> FederatedResult<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(id) {
            node.update_heartbeat();
            Ok(())
        } else {
            Err(FederatedError::NodeNotFound(id.to_string()))
        }
    }
    
    /// 检查超时节点
    pub async fn check_timeouts(&self) -> Vec<NodeId> {
        let mut nodes = self.nodes.write().await;
        let mut timed_out = Vec::new();
        
        let timeout_secs = self.config.heartbeat_timeout_secs;
        
        for (id, node) in nodes.iter_mut() {
            if node.is_heartbeat_timeout(timeout_secs) && node.state == NodeState::Active {
                node.state = NodeState::Offline;
                timed_out.push(id.clone());
                
                let _ = self.event_tx.send(NodeEvent::NodeStateChanged {
                    id: id.clone(),
                    old_state: NodeState::Active,
                    new_state: NodeState::Offline,
                });
            }
        }
        
        timed_out
    }
    
    /// 订阅节点事件
    pub fn subscribe(&self) -> broadcast::Receiver<NodeEvent> {
        self.event_tx.subscribe()
    }
    
    /// 获取节点数量
    pub async fn node_count(&self) -> usize {
        self.nodes.read().await.len()
    }
}

// ============================================================================
// 协议 Trait
// ============================================================================

/// 联邦协议 Trait
#[async_trait]
pub trait FederatedProtocol: Send + Sync {
    /// 启动协议
    async fn start(&mut self) -> FederatedResult<()>;
    
    /// 停止协议
    async fn stop(&mut self) -> FederatedResult<()>;
    
    /// 注册节点
    async fn register_node(&self, node: NodeInfo) -> FederatedResult<()>;
    
    /// 注销节点
    async fn unregister_node(&self, id: &NodeId) -> FederatedResult<()>;
    
    /// 发现节点
    async fn discover_nodes(&self) -> FederatedResult<Vec<NodeInfo>>;
    
    /// 发送消息
    async fn send_message(&self, to: &NodeId, message: FederatedMessage) -> FederatedResult<()>;
    
    /// 广播消息
    async fn broadcast(&self, message: FederatedMessage) -> FederatedResult<()>;
    
    /// 接收消息
    async fn receive_message(&mut self) -> FederatedResult<Option<FederatedMessage>>;
    
    /// 获取节点注册表
    fn registry(&self) -> &NodeRegistry;
}

/// 联邦消息（包装器）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedMessage {
    /// 消息 ID
    pub id: String,
    /// 来源节点
    pub from: NodeId,
    /// 目标节点
    pub to: NodeId,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 消息类型
    pub message_type: FederatedMessageType,
    /// 相关 ID（用于请求-响应）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    /// 加密数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_payload: Option<Vec<u8>>,
}

/// 联邦消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FederatedMessageType {
    /// 发现消息
    Discovery(DiscoveryMessage),
    /// 注册请求
    RegisterRequest(RegisterRequest),
    /// 注册响应
    RegisterResponse(RegisterResponse),
    /// 心跳
    Heartbeat(HeartbeatMessage),
    /// 记忆查询
    MemoryQuery(MemoryQueryRequest),
    /// 记忆响应
    MemoryResponse(MemoryQueryResponse),
    /// 记忆同步
    MemorySync(MemorySyncMessage),
    /// 确认
    Ack(AckMessage),
    /// 错误
    Error(ErrorMessage),
}

/// 心跳消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatMessage {
    /// 节点 ID
    pub node_id: NodeId,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 节点状态
    pub state: NodeState,
    /// 额外信息
    #[serde(default)]
    pub stats: HeartbeatStats,
}

/// 心跳统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeartbeatStats {
    /// 记忆条目数
    pub memory_entries: usize,
    /// 活跃连接数
    pub active_connections: usize,
    /// CPU 使用率
    pub cpu_usage: f32,
    /// 内存使用率
    pub memory_usage: f32,
}

/// 确认消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMessage {
    /// 确认的消息 ID
    pub message_id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 错误消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// 错误代码
    pub code: String,
    /// 错误信息
    pub message: String,
    /// 相关消息 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_message_id: Option<String>,
}

// ============================================================================
// 记忆查询协议
// ============================================================================

/// 记忆查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryRequest {
    /// 查询 ID
    pub query_id: String,
    /// 查询内容
    pub query: String,
    /// 查询类型
    pub query_type: QueryType,
    /// 返回数量限制
    pub limit: usize,
    /// 偏移量
    #[serde(default)]
    pub offset: usize,
    /// 过滤条件
    #[serde(default)]
    pub filters: QueryFilters,
    /// 是否包含元数据
    #[serde(default = "default_true")]
    pub include_metadata: bool,
}

/// 查询类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    /// 关键词搜索
    Keyword,
    /// 向量搜索
    Vector,
    /// 混合搜索
    Hybrid,
    /// 全量搜索
    Full,
}

impl Default for QueryType {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// 查询过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    /// 记忆类型过滤
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_type: Option<String>,
    /// 时间范围起始
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_start: Option<DateTime<Utc>>,
    /// 时间范围结束
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_end: Option<DateTime<Utc>>,
    /// 最小重要性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_importance: Option<f32>,
    /// 标签过滤
    #[serde(default)]
    pub tags: Vec<String>,
    /// 来源 Agent 过滤
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_agent: Option<String>,
}

fn default_true() -> bool { true }

/// 记忆查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResponse {
    /// 查询 ID
    pub query_id: String,
    /// 查询结果
    pub results: Vec<RemoteMemoryEntry>,
    /// 总数（用于分页）
    pub total: usize,
    /// 查询耗时（毫秒）
    pub elapsed_ms: u64,
    /// 是否有更多结果
    pub has_more: bool,
    /// 来源节点 ID
    pub from_node: NodeId,
}

/// 远程记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteMemoryEntry {
    /// 条目 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// 记忆类型
    pub memory_type: String,
    /// 重要性
    pub importance: f32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 来源节点
    pub source_node: NodeId,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

// ============================================================================
// 记忆同步协议
// ============================================================================

/// 记忆同步消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySyncMessage {
    /// 同步 ID
    pub sync_id: String,
    /// 同步类型
    pub sync_type: SyncType,
    /// 记忆条目
    pub entries: Vec<SyncEntry>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 同步类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncType {
    /// 全量同步
    Full,
    /// 增量同步
    Incremental,
    /// 单条同步
    Single,
}

/// 同步条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    /// 条目 ID
    pub id: String,
    /// 操作类型
    pub operation: SyncOperation,
    /// 条目数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<RemoteMemoryEntry>,
    /// 向量数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<Vec<f32>>,
}

/// 同步操作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOperation {
    /// 创建
    Create,
    /// 更新
    Update,
    /// 删除
    Delete,
}

// ============================================================================
// 协议版本
// ============================================================================

/// 协议版本
pub const PROTOCOL_VERSION: &str = "1.0.0";

/// 检查版本兼容性
pub fn check_version_compatibility(local: &str, remote: &str) -> FederatedResult<()> {
    let local_parts: Vec<&str> = local.split('.').collect();
    let remote_parts: Vec<&str> = remote.split('.').collect();
    
    if local_parts.len() >= 1 && remote_parts.len() >= 1 {
        if local_parts[0] != remote_parts[0] {
            return Err(FederatedError::VersionMismatch {
                local: local.to_string(),
                remote: remote.to_string(),
            });
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_id_new() {
        let id = NodeId::new();
        assert!(!id.as_str().is_empty());
    }
    
    #[test]
    fn test_node_info_new() {
        let addr: SocketAddr = "127.0.0.1:7654".parse().unwrap();
        let info = NodeInfo::new("test-node".to_string(), addr);
        
        assert_eq!(info.name, "test-node");
        assert_eq!(info.state, NodeState::Discovered);
        assert!(info.is_online() == false);
    }
    
    #[test]
    fn test_node_state_transitions() {
        let addr: SocketAddr = "127.0.0.1:7654".parse().unwrap();
        let mut info = NodeInfo::new("test-node".to_string(), addr);
        
        info.state = NodeState::Active;
        assert!(info.is_online());
        
        info.state = NodeState::Idle;
        assert!(info.is_online());
        
        info.update_heartbeat();
        assert_eq!(info.state, NodeState::Active);
    }
    
    #[test]
    fn test_discovery_message() {
        let node_id = NodeId::new();
        let addr: SocketAddr = "127.0.0.1:7654".parse().unwrap();
        let node_info = NodeInfo::new("test-node".to_string(), addr);
        
        let msg = DiscoveryMessage::new(node_id, node_info, DiscoveryType::Broadcast);
        
        assert!(!msg.id.is_empty());
        assert!(matches!(msg.discovery_type, DiscoveryType::Broadcast));
    }
    
    #[test]
    fn test_register_request_response() {
        let addr: SocketAddr = "127.0.0.1:7654".parse().unwrap();
        let node_info = NodeInfo::new("test-node".to_string(), addr);
        
        let request = RegisterRequest::new(node_info);
        assert!(request.auth_token.is_none());
        
        let request = request.with_auth_token("secret-token".to_string());
        assert!(request.auth_token.is_some());
        
        let response = RegisterResponse::success(
            request.id.clone(),
            NodeId::new(),
            NodeInfo::new("server".to_string(), addr),
        );
        assert!(response.success);
        
        let failure = RegisterResponse::failure(request.id, "Error".to_string());
        assert!(!failure.success);
    }
    
    #[test]
    fn test_check_version_compatibility() {
        // 兼容版本
        assert!(check_version_compatibility("1.0.0", "1.2.3").is_ok());
        assert!(check_version_compatibility("1.0.0", "1.0.0").is_ok());
        
        // 不兼容版本
        assert!(check_version_compatibility("1.0.0", "2.0.0").is_err());
    }
    
    #[test]
    fn test_federated_message_serialization() {
        let msg = FederatedMessage {
            id: Uuid::new_v4().to_string(),
            from: NodeId::new(),
            to: NodeId::new(),
            timestamp: Utc::now(),
            message_type: FederatedMessageType::Heartbeat(HeartbeatMessage {
                node_id: NodeId::new(),
                timestamp: Utc::now(),
                state: NodeState::Active,
                stats: HeartbeatStats::default(),
            }),
            correlation_id: None,
            encrypted_payload: None,
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        let restored: FederatedMessage = serde_json::from_str(&json).unwrap();
        
        assert_eq!(msg.id, restored.id);
    }
    
    #[test]
    fn test_memory_query_request() {
        let request = MemoryQueryRequest {
            query_id: Uuid::new_v4().to_string(),
            query: "test query".to_string(),
            query_type: QueryType::Hybrid,
            limit: 10,
            offset: 0,
            filters: QueryFilters::default(),
            include_metadata: true,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let restored: MemoryQueryRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request.query, restored.query);
        assert_eq!(request.limit, restored.limit);
    }
    
    #[tokio::test]
    async fn test_node_registry() {
        let config = FederatedConfig::default();
        let registry = NodeRegistry::new(config);
        
        let addr: SocketAddr = "127.0.0.1:7654".parse().unwrap();
        let node = NodeInfo::new("remote-node".to_string(), addr);
        let node_id = node.id.clone();
        
        // 注册节点
        registry.register(node.clone()).await.unwrap();
        
        // 获取节点
        let retrieved = registry.get(&node_id).await;
        assert!(retrieved.is_some());
        
        // 更新状态
        registry.update_state(&node_id, NodeState::Active).await.unwrap();
        let active_node = registry.get(&node_id).await.unwrap();
        assert_eq!(active_node.state, NodeState::Active);
        
        // 注销节点
        registry.unregister(&node_id).await.unwrap();
        let removed = registry.get(&node_id).await;
        assert!(removed.is_none());
    }
    
    #[tokio::test]
    async fn test_node_registry_events() {
        let config = FederatedConfig::default();
        let registry = NodeRegistry::new(config);
        let mut event_rx = registry.subscribe();
        
        let addr: SocketAddr = "127.0.0.1:7654".parse().unwrap();
        let node = NodeInfo::new("test-node".to_string(), addr);
        
        registry.register(node.clone()).await.unwrap();
        
        // 接收事件
        if let Ok(NodeEvent::NodeJoined(joined)) = event_rx.try_recv() {
            assert_eq!(joined.id, node.id);
        }
    }
}