// gRPC 客户端 - 智慧主控连接核心主控

use tonic::transport::Channel;
use std::time::Duration;

use super::proto::{
    lease_service_client::LeaseServiceClient,
    heartbeat_service_client::HeartbeatServiceClient,
    recovery_service_client::RecoveryServiceClient,
    AcquireLeaseRequest,
    RenewLeaseRequest,
    ReleaseLeaseRequest,
    HeartbeatRequest,
    HeartbeatResponse,
    LeaseResponse,
    TriggerRecoveryRequest,
    RecoveryResponse,
    ReleaseLeaseResponse,
};

/// 心跳数据
#[derive(Debug, Clone)]
pub struct HeartbeatData {
    pub lease_id: String,
    pub health_status: i32,
    pub memory_mb: u64,
    pub cpu_percent: f32,
    pub active_sessions: u64,
    pub errors: Vec<String>,
    pub component: String,
}

/// Watchdog gRPC 客户端
pub struct WatchdogClient {
    /// 租约服务客户端
    lease_client: Option<LeaseServiceClient<Channel>>,
    /// 心跳服务客户端
    heartbeat_client: Option<HeartbeatServiceClient<Channel>>,
    /// 恢复服务客户端
    recovery_client: Option<RecoveryServiceClient<Channel>>,
    /// 连接地址
    addr: String,
}

impl WatchdogClient {
    /// 创建新客户端
    pub fn new(addr: String) -> Self {
        Self {
            lease_client: None,
            heartbeat_client: None,
            recovery_client: None,
            addr,
        }
    }
    
    /// 连接到 Watchdog
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        let channel = Channel::from_shared(self.addr.clone())?
            .timeout(Duration::from_secs(5))
            .connect()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Watchdog: {}", e))?;
        
        self.lease_client = Some(LeaseServiceClient::new(channel.clone()));
        self.heartbeat_client = Some(HeartbeatServiceClient::new(channel.clone()));
        self.recovery_client = Some(RecoveryServiceClient::new(channel));
        
        tracing::info!("✅ gRPC client connected to Watchdog at {}", self.addr);
        Ok(())
    }
    
    /// 申请租约
    pub async fn acquire_lease(&mut self, holder: String, duration_secs: u64) -> anyhow::Result<String> {
        let client = self.lease_client.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Watchdog"))?;
        
        let request = tonic::Request::new(AcquireLeaseRequest {
            holder,
            duration_secs: duration_secs as i64,
        });
        
        let response: tonic::Response<LeaseResponse> = client.acquire_lease(request).await?;
        let lease = response.into_inner();
        
        if lease.success {
            tracing::debug!("Lease acquired: {}", lease.lease_id);
            Ok(lease.lease_id)
        } else {
            anyhow::bail!("Lease acquisition failed: {}", lease.error)
        }
    }
    
    /// 续约
    pub async fn renew_lease(&mut self, lease_id: String, duration_secs: u64) -> anyhow::Result<()> {
        let client = self.lease_client.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Watchdog"))?;
        
        let request = tonic::Request::new(RenewLeaseRequest {
            lease_id,
            duration_secs: duration_secs as i64,
        });
        
        let response: tonic::Response<LeaseResponse> = client.renew_lease(request).await?;
        let lease = response.into_inner();
        
        if lease.success {
            tracing::debug!("Lease renewed");
            Ok(())
        } else {
            anyhow::bail!("Lease renewal failed: {}", lease.error)
        }
    }
    
    /// 释放租约
    pub async fn release_lease(&mut self, lease_id: String, holder: String) -> anyhow::Result<()> {
        let client = self.lease_client.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Watchdog"))?;
        
        let request = tonic::Request::new(ReleaseLeaseRequest {
            lease_id,
            holder,
        });
        
        let response: tonic::Response<super::proto::ReleaseLeaseResponse> = client.release_lease(request).await?;
        let result = response.into_inner();
        
        if result.success {
            tracing::debug!("Lease released");
            Ok(())
        } else {
            anyhow::bail!("Lease release failed: {}", result.message)
        }
    }
    
    /// 发送心跳
    pub async fn send_heartbeat(
        &mut self,
        data: &HeartbeatData,
    ) -> anyhow::Result<bool> {
        let client = self.heartbeat_client.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Watchdog"))?;
        
        let request = tonic::Request::new(HeartbeatRequest {
            lease_id: data.lease_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            health: Some(super::proto::HealthStatus {
                status: data.health_status,
                message: String::new(),
                checks: vec![],
            }),
            metrics: Some(super::proto::SystemMetrics {
                memory_mb: data.memory_mb,
                cpu_percent: data.cpu_percent,
                active_sessions: data.active_sessions,
                request_rate: 0,
                error_rate: 0,
                uptime_secs: 0,
                goroutines: 0,
                open_fds: 0,
            }),
            recent_errors: data.errors.clone(),
            component: data.component.clone(),
        });
        
        let response: tonic::Response<HeartbeatResponse> = client.report_heartbeat(request).await?;
        let result = response.into_inner();
        
        tracing::debug!("Heartbeat acknowledged: {}", result.acknowledged);
        Ok(result.acknowledged)
    }
    
    /// 触发恢复
    pub async fn trigger_recovery(
        &mut self,
        component: String,
        level: i32,
        actions: Vec<String>,
    ) -> anyhow::Result<String> {
        let client = self.recovery_client.as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Watchdog"))?;
        
        let request = tonic::Request::new(TriggerRecoveryRequest {
            component,
            health_status: None,
            level,
            actions,
        });
        
        let response: tonic::Response<RecoveryResponse> = client.trigger_recovery(request).await?;
        let result = response.into_inner();
        
        if result.accepted {
            tracing::info!("Recovery triggered: {}", result.recovery_id);
            Ok(result.recovery_id)
        } else {
            anyhow::bail!("Recovery not accepted")
        }
    }
    
    /// 检查是否已连接
    pub fn is_connected(&self) -> bool {
        self.lease_client.is_some() && self.heartbeat_client.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_creation() {
        let client = WatchdogClient::new("http://127.0.0.1:50051".to_string());
        assert_eq!(client.addr, "http://127.0.0.1:50051");
        assert!(!client.is_connected());
    }
}