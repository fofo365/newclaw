// gRPC 服务端实现

use tonic::{transport::Server, Request, Response, Status};
use std::sync::Arc;

use crate::watchdog::controller::CoreController;
use crate::watchdog::heartbeat::HeartbeatStatus;

// 引入生成的 gRPC 代码
pub mod proto {
    tonic::include_proto!("newclaw.watchdog.v1");
}

use proto::{
    heartbeat_service_server::{HeartbeatService, HeartbeatServiceServer},
    lease_service_server::{LeaseService, LeaseServiceServer},
    health_check_service_server::{HealthCheckService, HealthCheckServiceServer},
    recovery_service_server::{RecoveryService, RecoveryServiceServer},
    *,
};

/// Watchdog gRPC 服务器
pub struct WatchdogGrpcServer {
    controller: Arc<tokio::sync::RwLock<CoreController>>,
}

impl WatchdogGrpcServer {
    pub fn new(controller: CoreController) -> Self {
        Self {
            controller: Arc::new(tokio::sync::RwLock::new(controller)),
        }
    }
    
    /// 启动 gRPC 服务器
    pub async fn serve(self, addr: &str) -> anyhow::Result<()> {
        let addr = addr.parse()?;
        
        println!("Watchdog gRPC server listening on {}", addr);
        
        Server::builder()
            .add_service(HeartbeatServiceServer::new(self.clone()))
            .add_service(LeaseServiceServer::new(self.clone()))
            .add_service(RecoveryServiceServer::new(self.clone()))
            .add_service(HealthCheckServiceServer::new(self))
            .serve(addr)
            .await?;
        
        Ok(())
    }
}

impl Clone for WatchdogGrpcServer {
    fn clone(&self) -> Self {
        Self {
            controller: self.controller.clone(),
        }
    }
}

/// 心跳服务实现
#[tonic::async_trait]
impl HeartbeatService for WatchdogGrpcServer {
    async fn report_heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        
        let status = HeartbeatStatus {
            lease_id: req.lease_id.clone(),
            timestamp: chrono::DateTime::from_timestamp_millis(req.timestamp)
                .unwrap_or_else(chrono::Utc::now),
            health: convert_health_status(req.health),
            metrics: convert_metrics(req.metrics),
            recent_errors: req.recent_errors,
            component: req.component,
        };
        
        let mut controller = self.controller.write().await;
        let acknowledged = controller.handle_heartbeat(status).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(HeartbeatResponse {
            acknowledged,
            server_time: chrono::Utc::now().timestamp_millis(),
            lease_valid: controller.lease_manager().is_valid(),
            message: if acknowledged { "OK".to_string() } else { "Rejected".to_string() },
        }))
    }
    
    async fn get_heartbeat_status(
        &self,
        _request: Request<GetHeartbeatRequest>,
    ) -> Result<Response<HeartbeatStatusResponse>, Status> {
        // TODO: 实现状态查询
        Ok(Response::new(HeartbeatStatusResponse {
            available: true,
            health: Some(HealthStatus {
                status: health_status::Status::Healthy as i32,
                message: "OK".to_string(),
                checks: vec![],
            }),
            last_heartbeat: chrono::Utc::now().timestamp_millis(),
            consecutive_failures: 0,
        }))
    }
}

/// 租约服务实现
#[tonic::async_trait]
impl LeaseService for WatchdogGrpcServer {
    async fn acquire_lease(
        &self,
        request: Request<AcquireLeaseRequest>,
    ) -> Result<Response<LeaseResponse>, Status> {
        let req = request.into_inner();
        
        let controller = self.controller.read().await;
        let lease = controller.lease_manager()
            .acquire(req.holder)
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(LeaseResponse {
            success: true,
            lease_id: lease.id,
            holder: lease.holder,
            created_at: lease.created_at.timestamp_millis(),
            expires_at: lease.expires_at.timestamp_millis(),
            last_renewed: lease.last_renewed.timestamp_millis(),
            error: String::new(),
        }))
    }
    
    async fn renew_lease(
        &self,
        request: Request<RenewLeaseRequest>,
    ) -> Result<Response<LeaseResponse>, Status> {
        let req = request.into_inner();
        
        let controller = self.controller.read().await;
        let lease = controller.lease_manager()
            .renew(&req.lease_id)
            .map_err(|e| Status::not_found(e.to_string()))?;
        
        Ok(Response::new(LeaseResponse {
            success: true,
            lease_id: lease.id,
            holder: lease.holder,
            created_at: lease.created_at.timestamp_millis(),
            expires_at: lease.expires_at.timestamp_millis(),
            last_renewed: lease.last_renewed.timestamp_millis(),
            error: String::new(),
        }))
    }
    
    async fn release_lease(
        &self,
        request: Request<ReleaseLeaseRequest>,
    ) -> Result<Response<ReleaseLeaseResponse>, Status> {
        let req = request.into_inner();
        
        let controller = self.controller.read().await;
        controller.lease_manager()
            .release(&req.lease_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(ReleaseLeaseResponse {
            success: true,
            message: "Lease released".to_string(),
        }))
    }
    
    async fn get_lease_status(
        &self,
        request: Request<GetLeaseRequest>,
    ) -> Result<Response<LeaseResponse>, Status> {
        let req = request.into_inner();
        
        let controller = self.controller.read().await;
        let lease = controller.lease_manager()
            .get(&req.lease_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        
        match lease {
            Some(lease) => Ok(Response::new(LeaseResponse {
                success: true,
                lease_id: lease.id,
                holder: lease.holder,
                created_at: lease.created_at.timestamp_millis(),
                expires_at: lease.expires_at.timestamp_millis(),
                last_renewed: lease.last_renewed.timestamp_millis(),
                error: String::new(),
            })),
            None => Ok(Response::new(LeaseResponse {
                success: false,
                lease_id: String::new(),
                holder: String::new(),
                created_at: 0,
                expires_at: 0,
                last_renewed: 0,
                error: "Lease not found".to_string(),
            })),
        }
    }
}

/// 健康检查服务实现
#[tonic::async_trait]
impl HealthCheckService for WatchdogGrpcServer {
    async fn check_health(
        &self,
        _request: Request<CheckHealthRequest>,
    ) -> Result<Response<CheckHealthResponse>, Status> {
        let controller = self.controller.read().await;
        let lease_valid = controller.lease_manager().is_valid();
        
        Ok(Response::new(CheckHealthResponse {
            status: Some(HealthStatus {
                status: if lease_valid {
                    health_status::Status::Healthy as i32
                } else {
                    health_status::Status::Unhealthy as i32
                },
                message: if lease_valid { "OK" } else { "No valid lease" }.to_string(),
                checks: vec![],
            }),
            components: std::collections::HashMap::new(),
            checked_at: chrono::Utc::now().timestamp_millis(),
        }))
    }
    
    async fn check_ready(
        &self,
        _request: Request<CheckReadyRequest>,
    ) -> Result<Response<CheckReadyResponse>, Status> {
        let controller = self.controller.read().await;
        let ready = controller.lease_manager().is_valid();
        
        let mut checks = std::collections::HashMap::new();
        checks.insert("lease".to_string(), ready);
        
        Ok(Response::new(CheckReadyResponse {
            ready,
            checks,
            message: if ready { "Ready" } else { "Not ready" }.to_string(),
        }))
    }
}

/// 恢复服务实现
#[tonic::async_trait]
impl RecoveryService for WatchdogGrpcServer {
    async fn trigger_recovery(
        &self,
        request: Request<TriggerRecoveryRequest>,
    ) -> Result<Response<RecoveryResponse>, Status> {
        let req = request.into_inner();
        
        // 解析恢复级别
        let level = match RecoveryLevel::try_from(req.level) {
            Ok(RecoveryLevel::L1QuickFix) => {
                crate::watchdog::recovery::RecoveryLevel::L1QuickFix
            }
            Ok(RecoveryLevel::L2AiDiagnosis) => {
                crate::watchdog::recovery::RecoveryLevel::L2AiDiagnosis
            }
            Ok(RecoveryLevel::L3HumanIntervention) => {
                crate::watchdog::recovery::RecoveryLevel::L3HumanIntervention
            }
            _ => crate::watchdog::recovery::RecoveryLevel::L1QuickFix,
        };
        
        // 生成恢复计划
        let plan = crate::watchdog::recovery::RecoveryExecutor::generate_l1_plan(
            req.component.clone(),
            req.actions,
        );
        
        let recovery_id = plan.id.clone();
        let recovery_id_for_spawn = recovery_id.clone();
        let estimated_duration = match level {
            crate::watchdog::recovery::RecoveryLevel::L1QuickFix => 1,
            crate::watchdog::recovery::RecoveryLevel::L2AiDiagnosis => 30,
            crate::watchdog::recovery::RecoveryLevel::L3HumanIntervention => 300,
        };
        
        // 触发异步恢复
        let controller = self.controller.clone();
        let plan_clone = plan;
        tokio::spawn(async move {
            let audit_log = crate::watchdog::audit::AuditLogger::new(
                crate::watchdog::config::AuditConfig::default()
            );
            let executor = crate::watchdog::recovery::RecoveryExecutor::new(audit_log);
            
            match executor.execute(plan_clone).await {
                Ok(result) => {
                    tracing::info!("Recovery {} completed: success={}", recovery_id_for_spawn, result.success);
                }
                Err(e) => {
                    tracing::error!("Recovery {} failed: {}", recovery_id_for_spawn, e);
                }
            }
        });
        
        Ok(Response::new(RecoveryResponse {
            accepted: true,
            recovery_id,
            level: req.level,
            estimated_duration,
        }))
    }
    
    async fn get_recovery_status(
        &self,
        request: Request<GetRecoveryRequest>,
    ) -> Result<Response<RecoveryStatus>, Status> {
        let req = request.into_inner();
        
        // TODO: 实现持久化的恢复状态查询
        // 目前返回模拟数据
        Ok(Response::new(RecoveryStatus {
            recovery_id: req.recovery_id,
            level: RecoveryLevel::L1QuickFix.into(),
            state: RecoveryState::Succeeded.into(),
            started_at: chrono::Utc::now().timestamp_millis() - 1000,
            completed_at: chrono::Utc::now().timestamp_millis(),
            actions: vec![RecoveryAction {
                name: "restart_service".to_string(),
                description: "Restart the service".to_string(),
                state: RecoveryState::Succeeded.into(),
                started_at: chrono::Utc::now().timestamp_millis() - 1000,
                completed_at: chrono::Utc::now().timestamp_millis(),
                output: "Service restarted successfully".to_string(),
                error: String::new(),
            }],
            result: "Recovery completed successfully".to_string(),
        }))
    }
    
    async fn acknowledge_recovery(
        &self,
        request: Request<AcknowledgeRecoveryRequest>,
    ) -> Result<Response<AcknowledgeRecoveryResponse>, Status> {
        let req = request.into_inner();
        
        tracing::info!(
            "Recovery {} acknowledged by {}: {}",
            req.recovery_id,
            req.acknowledged_by,
            req.notes
        );
        
        Ok(Response::new(AcknowledgeRecoveryResponse {
            success: true,
            message: "Recovery acknowledged".to_string(),
        }))
    }
}

/// 转换健康状态
fn convert_health_status(health: Option<HealthStatus>) -> crate::watchdog::heartbeat::HealthStatus {
    match health {
        Some(h) => match health_status::Status::try_from(h.status) {
            Ok(health_status::Status::Healthy) => crate::watchdog::heartbeat::HealthStatus::Healthy,
            Ok(health_status::Status::Degraded) => crate::watchdog::heartbeat::HealthStatus::Degraded(h.message),
            Ok(health_status::Status::Unhealthy) => crate::watchdog::heartbeat::HealthStatus::Unhealthy(h.message),
            _ => crate::watchdog::heartbeat::HealthStatus::Healthy,
        },
        None => crate::watchdog::heartbeat::HealthStatus::Healthy,
    }
}

/// 转换系统指标
fn convert_metrics(metrics: Option<SystemMetrics>) -> crate::watchdog::heartbeat::SystemMetrics {
    match metrics {
        Some(m) => crate::watchdog::heartbeat::SystemMetrics {
            memory_mb: m.memory_mb,
            cpu_percent: m.cpu_percent as f64,
            active_sessions: m.active_sessions,
            request_rate: m.request_rate,
            error_rate: m.error_rate,
            uptime_secs: m.uptime_secs,
        },
        None => crate::watchdog::heartbeat::SystemMetrics::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_convert_health_status() {
        let healthy = Some(HealthStatus {
            status: health_status::Status::Healthy as i32,
            message: "OK".to_string(),
            checks: vec![],
        });
        
        let result = convert_health_status(healthy);
        assert!(result.is_healthy());
    }
    
    #[test]
    fn test_convert_metrics() {
        let metrics = Some(SystemMetrics {
            memory_mb: 100,
            cpu_percent: 50.0,
            active_sessions: 10,
            request_rate: 100,
            error_rate: 1,
            uptime_secs: 3600,
            goroutines: 0,
            open_fds: 0,
        });
        
        let result = convert_metrics(metrics);
        assert_eq!(result.memory_mb, 100);
        assert_eq!(result.cpu_percent, 50.0);
    }
}
