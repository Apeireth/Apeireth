//! Vision perception backend implementations (engine layer).
//!
//! **架构**: trait 在 `apeireth-plugin::perception_backend` (foundation),
//! impl 在本模块 (engine). 单向依赖: perception → plugin.
//!
//! **当前实现**:
//! - `NoopVisionBackend`: 0 装显式占位实现 (返回 `BackendUnavailable`).
//! - `XcapVisionBackend`: 屏幕截屏感知实现.

pub mod noop;
pub mod xcap_backend;

pub use noop::NoopVisionBackend;
pub use xcap_backend::{XcapVisionBackend, XcapVisionConfig};
