//! B1: `apeireth-telemetry::cache` module — 1:1 搬自 `apeireth-cache` (R35+1.1)
//! 0 改源: 完整保留 apeireth-cache 公开 API + module path. 兼容 shim 见 `crates/apeireth-cache/src/lib.rs`.

mod _root;
pub use _root::*;

pub mod backend;
pub mod config;
pub mod error;
pub mod lru;
pub mod memory_provider;
pub mod policy;
pub mod shard;
pub mod stats;
pub mod ttl;
