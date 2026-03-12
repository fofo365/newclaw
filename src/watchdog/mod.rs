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

pub use controller::CoreController;
pub use lease::{LeaseManager, Lease, LeaseStorage};
pub use heartbeat::{HeartbeatChecker, HeartbeatConfig, HeartbeatStatus};
pub use diagnostic::{DiagnosticEngine, DiagnosticResult, RootCause};
pub use recovery::{RecoveryExecutor, RecoveryPlan, RecoveryLevel, RecoveryResult};
pub use audit::{AuditLogger, AuditEvent};
pub use config::WatchdogConfig;
