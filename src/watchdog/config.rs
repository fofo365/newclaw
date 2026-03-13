// Watchdog 配置模块

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 核心主控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    /// 检查间隔（秒）
    pub check_interval: u64,
    
    /// 心跳超时（秒）
    pub heartbeat_timeout: u64,
    
    /// 最大失败次数
    pub max_heartbeat_failures: u32,
    
    /// 租约配置
    pub lease: LeaseConfig,
    
    /// 恢复配置
    pub recovery: RecoveryConfig,
    
    /// 审计配置
    pub audit: AuditConfig,
    
    /// gRPC 配置
    pub grpc: GrpcConfig,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            check_interval: 5,
            heartbeat_timeout: 15,
            max_heartbeat_failures: 3,
            lease: LeaseConfig::default(),
            recovery: RecoveryConfig::default(),
            audit: AuditConfig::default(),
            grpc: GrpcConfig::default(),
        }
    }
}

/// 租约存储类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseStorageType {
    /// 内存存储（开发环境，不推荐生产）
    Memory,
    /// Redis 存储（生产环境推荐）
    Redis,
}

impl Default for LeaseStorageType {
    fn default() -> Self {
        // 生产环境默认使用 Redis
        Self::Redis
    }
}

/// 租约配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseConfig {
    /// 租约期限（秒）
    pub duration: u64,
    
    /// 续约截止时间（秒）
    pub renew_deadline: u64,
    
    /// 存储类型
    pub storage: LeaseStorageType,
    
    /// Redis 连接地址（当 storage 为 Redis 时使用）
    pub redis_url: String,
}

impl Default for LeaseConfig {
    fn default() -> Self {
        Self {
            duration: 15,
            renew_deadline: 10,
            storage: LeaseStorageType::Redis,
            redis_url: "redis://127.0.0.1:6379".to_string(),
        }
    }
}

impl LeaseConfig {
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.duration)
    }
    
    pub fn renew_deadline(&self) -> Duration {
        Duration::from_secs(self.renew_deadline)
    }
}

/// 恢复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// 恢复策略
    pub strategy: RecoveryStrategy,
    
    /// L1 快速修复
    pub l1: L1Config,
    
    /// L2 AI 诊断
    pub l2: L2Config,
    
    /// L3 人工介入
    pub l3: L3Config,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            strategy: RecoveryStrategy::Graduated,
            l1: L1Config::default(),
            l2: L2Config::default(),
            l3: L3Config::default(),
        }
    }
}

/// 恢复策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStrategy {
    /// 分级恢复（L1 → L2 → L3）
    Graduated,
    /// 立即恢复
    Immediate,
    /// 保守恢复
    Conservative,
}

/// L1 快速修复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1Config {
    pub enabled: bool,
    pub max_retries: u32,
    pub backoff_base: u64,
}

impl Default for L1Config {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: 3,
            backoff_base: 1,
        }
    }
}

/// L2 AI 诊断配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Config {
    pub enabled: bool,
    pub llm_provider: String,
    pub llm_model: String,
    pub max_tokens: usize,
}

impl Default for L2Config {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_provider: "glm".to_string(),
            llm_model: "glm-4".to_string(),
            max_tokens: 2000,
        }
    }
}

/// L3 人工介入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L3Config {
    pub enabled: bool,
    pub alert_channels: Vec<String>,
    pub safe_mode_timeout: u64,
}

impl Default for L3Config {
    fn default() -> Self {
        Self {
            enabled: true,
            alert_channels: vec!["feishu".to_string()],
            safe_mode_timeout: 300,
        }
    }
}

/// 审计配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_path: String,
    pub max_size_mb: u64,
    pub retention_days: u64,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: "/var/log/newclaw/watchdog-audit.json".to_string(),
            max_size_mb: 100,
            retention_days: 30,
        }
    }
}

/// gRPC 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    pub bind: String,
    pub tls_enabled: bool,
    pub tls_cert: Option<String>,
    pub tls_key: Option<String>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:50051".to_string(),
            tls_enabled: false,
            tls_cert: None,
            tls_key: None,
        }
    }
}
