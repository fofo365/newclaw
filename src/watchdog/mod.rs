// Watchdog Module - v0.6.0
//
// 核心主控（轻量级看门狗）

pub mod grpc;
pub mod controller;
pub mod lease;
pub mod heartbeat;
pub mod diagnostic;
pub mod recovery;
pub mod audit;
pub mod config;
pub mod ai_analyzer;
pub mod notifier;
pub mod quick_fix;

pub use controller::CoreController;
pub use lease::{LeaseManager, Lease, LeaseStorage};
pub use heartbeat::{HeartbeatChecker, HeartbeatConfig, HeartbeatStatus};
pub use diagnostic::{DiagnosticEngine, DiagnosticResult, RootCause};
pub use recovery::{RecoveryExecutor, RecoveryPlan, RecoveryLevel, RecoveryResult};
pub use audit::{AuditLogger, AuditEvent};
pub use config::WatchdogConfig;
pub use notifier::{Notifier, AlertMessage, AlertLevel};
pub use grpc::WatchdogGrpcServer;
pub use quick_fix::{QuickFixExecutor, ServiceStatus};
