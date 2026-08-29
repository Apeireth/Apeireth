//! Canonical builtin tool capabilities.
//!
//! This crate provides the M2A low-risk builtin tools as canonical
//! [`ToolCapability`] implementations wrapped in a single plugin.
//!
//! # Ownership
//!
//! The tools own their identity, input schema, execution implementation, and
//! structured result. They do **not** own the runtime, the gateway, sessions,
//! providers, governance, or the dispatch loop. The runtime reaches them only
//! through the canonical plugin/capability registry path.

#![deny(unsafe_code)]

pub mod education;
pub mod egress;
pub mod fetch;
pub mod filesystem;
pub mod plugin;
pub mod process;
pub mod repo;
pub mod search;
mod sensitive_path;
pub mod shell;
pub mod spill;
// P-arch (2026-08-27): B5 process supervisor trait 骨架. 详见 ROADMAP §4 P5.
// v2.0.0-rc.1 RC-8: 加 std_sub_supervisor 模块 (真 impl, std::process::Command 同步启进程).
pub mod std_sub_supervisor;
pub mod supervisor;

pub use education::{DxCheckTool, DxReport, REPLACED_DIFFS};
pub use egress::{ControlledEgress, EgressAllowList, EgressError, EgressPolicy};
pub use fetch::{FetchConfig, FetchTool};
pub use filesystem::{FilesystemError, FilesystemTool};
pub use plugin::{BuiltinToolsOptions, BuiltinToolsPlugin};
pub use repo::{RepoError, RepoTool};
pub use search::{SearchError, SearchTool};
pub use shell::{ShellTool, TrustedShellConfig};
pub use spill::{safe_segment, SpillStore, SPILL_THRESHOLD_CHARS};
