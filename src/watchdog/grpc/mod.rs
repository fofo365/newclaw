// gRPC 服务模块

pub mod server;
pub mod client;

pub use server::WatchdogGrpcServer;
pub use client::WatchdogClient;

// 导出 gRPC 客户端类型（由 tonic 自动生成）
pub mod proto {
    tonic::include_proto!("newclaw.watchdog.v1");
}

// 重新导出客户端类型，便于智慧主控使用
pub use proto::{
    AcquireLeaseRequest,
    RenewLeaseRequest,
    ReleaseLeaseRequest,
    GetLeaseRequest,
    LeaseResponse,
    HeartbeatRequest,
    HeartbeatResponse,
    TriggerRecoveryRequest,
    RecoveryResponse,
    HealthStatus,
    SystemMetrics,
    lease_service_client::LeaseServiceClient,
    heartbeat_service_client::HeartbeatServiceClient,
    recovery_service_client::RecoveryServiceClient,
};
