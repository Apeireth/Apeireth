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
// R177: organ invariants (5 tests + 2 Kani)
pub mod calculator;
pub mod compat;
pub mod enhanced;
mod organ_kani_proofs;
pub mod persist;
pub mod preset;
pub mod ssh;
pub mod streaming; // TP4/N22: ShellPreset 预设命令模板 (白名单 + 参数模板填充防注入, §10 官方包最后一件)

pub use calculator::{evaluate_expression, CalcError};
pub use compat::{ShellCommand, ShellCompatRouter};
pub use enhanced::{EnhancedShell, ShellError};
pub use persist::{PersistentTaskStore, TaskRecord};
pub use preset::{ArgSpec, PresetError, PresetRegistry, PresetShell, ShellPreset};
pub use sandbox::{apply_sandbox, SandboxMode, SandboxPolicy};
pub use ssh::SshClient;

/// R138 deliverables (per 5-dim extension):
/// - 7 modules (sandbox / ssh / persist / streaming / calculator / compat / enhanced)
/// - all real impls (no skeleton), 0 borrowed-from-VCP strings
pub const R138_DELIVERABLES: usize = 7;

/// 5 dimensions of extension (per R137-style framework):
/// 1. Real sandbox (seccomp/JobObject)
/// 2. russh SSH client
/// 3. Persistent tasks (SQLite)
/// 4. Streaming stdout
/// 5. Multi-sig sensitive ops
pub const UPGRADE_DIMENSIONS: usize = 5;
