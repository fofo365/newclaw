// Communication Configuration
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationConfig {
    pub websocket_enabled: bool,
    pub websocket_port: u16,
    pub websocket_host: String,
    pub http_enabled: bool,
    pub http_port: u16,
    pub http_host: String,
    #[cfg(feature = "redis-support")]
    pub redis_enabled: bool,
    #[cfg(feature = "redis-support")]
    pub redis_url: String,
}

impl Default for CommunicationConfig {
    fn default() -> Self {
        Self {
            websocket_enabled: true,
            websocket_port: 8080,
            websocket_host: "0.0.0.0".to_string(),
            http_enabled: true,
            http_port: 3000,
            http_host: "0.0.0.0".to_string(),
            #[cfg(feature = "redis-support")]
            redis_enabled: false,
            #[cfg(feature = "redis-support")]
            redis_url: "redis://localhost".to_string(),
        }
    }
}
