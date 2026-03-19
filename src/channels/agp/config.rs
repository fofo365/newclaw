// AGP Channel 配置

use serde::{Deserialize, Serialize};

/// 联邦域
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationDomain(pub String);

/// AGP Channel 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AGPConfig {
    /// Agent ID（联邦网络中的唯一标识）
    pub agent_id: String,

    /// 协调平面 bootstrap 地址
    pub bootstrap: String,

    /// 能力声明（向联邦网络宣告的能力）
    #[serde(default)]
    pub advertise: Vec<String>,

    /// 联邦域标识
    pub domain: Option<String>,

    /// 本地 endpoint（可选，自动检测）
    pub endpoint: Option<String>,

    /// 连接超时（秒）
    #[serde(default = "default_timeout")]
    pub timeout_secs: Option<u64>,

    /// 心跳间隔（秒）
    #[serde(default = "default_heartbeat")]
    pub heartbeat_interval_secs: Option<u64>,
}

fn default_timeout() -> Option<u64> { Some(30) }
fn default_heartbeat() -> Option<u64> { Some(60) }

impl Default for AGPConfig {
    fn default() -> Self {
        Self {
            agent_id: "unknown".to_string(),
            bootstrap: "agp://localhost:8000".to_string(),
            advertise: vec![],
            domain: None,
            endpoint: None,
            timeout_secs: default_timeout(),
            heartbeat_interval_secs: default_heartbeat(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AGPConfig::default();
        assert_eq!(config.agent_id, "unknown");
        assert_eq!(config.bootstrap, "agp://localhost:8000");
        assert!(config.advertise.is_empty());
        assert!(config.domain.is_none());
        assert_eq!(config.timeout_secs, Some(30));
        assert_eq!(config.heartbeat_interval_secs, Some(60));
    }
}
