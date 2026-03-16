// 通道管理器
//
// 管理所有通道实例，提供统一的入口点

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::{Channel, ChannelType, ChannelStatus, ChannelPermission, base::BaseChannel};
use crate::tools::ToolRegistry;

/// 通道注册表
pub struct ChannelRegistry {
    /// 通道实例
    channels: Arc<RwLock<HashMap<String, Arc<dyn Channel>>>>,
    /// 工具注册表 (共享)
    tools: Arc<ToolRegistry>,
    /// 权限管理 (共享)
    permissions: Arc<ChannelPermission>,
}

impl ChannelRegistry {
    /// 创建新的通道注册表
    pub fn new(tools: Arc<ToolRegistry>, permissions: Arc<ChannelPermission>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            tools,
            permissions,
        }
    }

    /// 注册通道
    pub async fn register(&self, name: &str, channel: Arc<dyn Channel>) {
        let mut channels = self.channels.write().await;
        channels.insert(name.to_string(), channel);
        info!("注册通道: {}", name);
    }

    /// 注销通道
    pub async fn unregister(&self, name: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(name);
        info!("注销通道: {}", name);
    }

    /// 获取通道
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        let channels = self.channels.read().await;
        channels.get(name).cloned()
    }

    /// 列出所有通道
    pub async fn list(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }

    /// 列出所有通道状态
    pub async fn list_status(&self) -> HashMap<String, (ChannelType, ChannelStatus)> {
        let channels = self.channels.read().await;
        channels.iter()
            .map(|(name, channel)| {
                (name.clone(), (channel.channel_type(), channel.status()))
            })
            .collect()
    }

    /// 获取工具注册表
    pub fn tools(&self) -> Arc<ToolRegistry> {
        Arc::clone(&self.tools)
    }

    /// 获取权限管理
    pub fn permissions(&self) -> Arc<ChannelPermission> {
        Arc::clone(&self.permissions)
    }
}

/// 通道管理器
pub struct ChannelManager {
    /// 通道注册表
    registry: ChannelRegistry,
}

impl ChannelManager {
    /// 创建新的通道管理器
    pub fn new(tools: Arc<ToolRegistry>, permissions: Arc<ChannelPermission>) -> Self {
        Self {
            registry: ChannelRegistry::new(tools, permissions),
        }
    }

    /// 获取注册表
    pub fn registry(&self) -> &ChannelRegistry {
        &self.registry
    }

    /// 启动所有通道
    pub async fn start_all(&self) -> anyhow::Result<()> {
        let channels = self.registry.channels.read().await;
        for (name, channel) in channels.iter() {
            if let Err(e) = channel.start().await {
                warn!("启动通道 {} 失败: {}", name, e);
            } else {
                info!("启动通道: {}", name);
            }
        }
        Ok(())
    }

    /// 停止所有通道
    pub async fn stop_all(&self) -> anyhow::Result<()> {
        let channels = self.registry.channels.read().await;
        for (name, channel) in channels.iter() {
            if let Err(e) = channel.stop().await {
                warn!("停止通道 {} 失败: {}", name, e);
            } else {
                info!("停止通道: {}", name);
            }
        }
        Ok(())
    }

    /// 获取通道状态
    pub async fn status(&self) -> HashMap<String, (ChannelType, ChannelStatus)> {
        self.registry.list_status().await
    }
}