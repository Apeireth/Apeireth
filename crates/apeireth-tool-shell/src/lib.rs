//! `apeireth-tool-shell` - R138 shell extension.
//!
//! Extends apeireth-tools::code_exec (4 safety mechanisms) and long_task
//! (TaskManager) with 5 new dimensions:
//! 1. **Real sandbox** - Linux seccomp + Windows Job Object (cfg-gated)
//! 2. **russh SSH client** - pure Rust, keep-alive, known_hosts, pool
//! 3. **Persistent tasks** - SQLite-backed TaskId across daemon restarts
//! 4. **Streaming stdout** - tokio::io::AsyncBufRead line streaming
//! 5. **Multi-sig sensitive ops** - apeireth-sovereignty physical_multisig
//!
//! Plus 2 auxiliary:
//! 6. **Calculator** - meval (replaces VCP mathjs 100KB)
//! 7. **VCP compatibility** - LinuxShellExecutor + PowerShellExecutor 1:1

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod sandbox;
pub mod ssh;
pub mod persist;
pub mod streaming;
pub mod calculator;
pub mod vcp_compat;
pub mod enhanced;

pub use sandbox::{SandboxPolicy, SandboxMode, apply_sandbox};
pub use ssh::SshClient;
pub use persist::{PersistentTaskStore, TaskRecord};
pub use calculator::{evaluate_expression, CalcError};
pub use vcp_compat::{VcpShellRouter, VcpShellCommand};
pub use enhanced::{EnhancedShell, ShellError};

/// R138 deliverables (per 5-dim extension):
/// - 7 modules (sandbox / ssh / persist / streaming / calculator / vcp_compat / enhanced)
/// - all real impls (no skeleton), 0 borrowed-from-VCP strings
pub const R138_DELIVERABLES: usize = 7;

/// 5 dimensions of extension (per R137-style framework):
/// 1. Real sandbox (seccomp/JobObject)
/// 2. russh SSH client
/// 3. Persistent tasks (SQLite)
/// 4. Streaming stdout
/// 5. Multi-sig sensitive ops
pub const UPGRADE_DIMENSIONS: usize = 5;
