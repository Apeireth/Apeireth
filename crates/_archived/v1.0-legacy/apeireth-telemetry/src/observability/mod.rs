//! B1: `apeireth-telemetry::observability` module — 1:1 搬自 `apeireth-observability` (R35+1.1)
//! 0 改源: 完整保留 apeireth-observability 公开 API + module path. 兼容 shim 见 `crates/apeireth-observability/src/lib.rs`.

mod _root;
pub use _root::*;

pub mod health;
pub mod logging;
pub mod metrics;
pub mod tracing_integration;
pub mod tui_dashboard;
