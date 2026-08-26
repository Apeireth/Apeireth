//! B1: `apeireth-telemetry::trace` module — 1:1 搬自 `apeireth-tracing` (R35+1.1)
//! 0 改源: 完整保留 apeireth-tracing 公开 API + module path. 兼容 shim 见 `crates/apeireth-tracing/src/lib.rs`.

mod _root;
pub use _root::*;

pub mod config;
pub mod context;
pub mod error;
pub mod exporter;
pub mod propagation;
pub mod sampler;
pub mod span;
pub mod trace;
