//! B1: `apeireth-telemetry::metric` module — 1:1 搬自 `apeireth-metrics` (R35+1.1)
//! 0 改源: 完整保留 apeireth-metrics 公开 API + module path. 兼容 shim 见 `crates/apeireth-metrics/src/lib.rs`.

mod _root;
pub use _root::*;

pub mod config;
pub mod counter;
pub mod encoder;
pub mod error;
pub mod exporter;
pub mod gauge;
pub mod histogram;
pub mod label;
pub mod metric;
pub mod registry;
pub mod summary;
