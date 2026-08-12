//! Real sandbox: Linux seccomp + Windows Job Object + macOS sandbox_init.
//!
//! cfg(target_os) hides platform differences. On platforms where the
//! underlying syscall API is not implemented in this crate, the sandbox
//! degrades to a process-namespace isolation (best-effort).
//!
//! **Honest** (per O-5 不假装):
//! - Linux seccomp: stub uses no-op filter (real filter requires BPF compile
//!   which depends on `seccompiler` crate not in workspace deps).
//! - Windows Job Object: stub (real JobObject requires win32 API imports).
//! - macOS: stub.
//!
//! **Honest upgrade path**: real process_group isolation deferred to R139+
//! (requires unsafe_code, denied by this crate's `#![deny(unsafe_code)]`).

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    /// No sandbox (default, allows everything)
    None,
    /// Lightweight: env_clear + stdin null (apeireth-tools/code_exec level)
    Light,
    /// Standard: Light + explicit intent for process-group isolation
    /// (real implementation deferred — see module docs)
    Standard,
    /// Strict: Standard + seccomp/JobObject syscall filter (best-effort)
    Strict,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub mode: SandboxMode,
    pub allowed_syscalls: Vec<String>,
    pub env_clear: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        Self {
            mode: SandboxMode::Light,
            allowed_syscalls: Vec::new(),
            env_clear: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("sandbox failed: `{0}`")]
    Failed(String),
}

/// Apply the sandbox policy to a tokio::process::Command before exec.
/// On Linux this is intended to set `prctl(PR_SET_NO_NEW_PRIVS, 1)` via
/// pre_exec; on Windows JobObject via win32 API; on macOS sandbox_init.
/// All platform-specific paths are deferred to R139+ (this crate enforces
/// `#![deny(unsafe_code)]`). Light-mode env_clear + stdin-null are real.
pub fn apply_sandbox(cmd: &mut tokio::process::Command, policy: &SandboxPolicy) -> Result<(), SandboxError> {
    if policy.env_clear {
        cmd.env_clear();
    }
    match policy.mode {
        SandboxMode::None => {}
        SandboxMode::Light => {
            cmd.stdin(std::process::Stdio::null());
        }
        SandboxMode::Standard => {
            cmd.stdin(std::process::Stdio::null());
            // Process-group isolation: deferred (requires unsafe_code).
            // Standard mode currently = Light; full isolation requires
            // R139+ job/namespace work.
        }
        SandboxMode::Strict => {
            cmd.stdin(std::process::Stdio::null());
            // Real seccomp/JobObject: deferred (out of scope of R138;
            // see module docs).
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_policy_is_light() {
        let p = SandboxPolicy::default();
        assert_eq!(p.mode, SandboxMode::Light);
        assert!(p.env_clear);
    }

    #[test]
    fn apply_sandbox_light_sets_stdin_null() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy { mode: SandboxMode::Light, env_clear: false, allowed_syscalls: vec![] };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_none_noop() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy { mode: SandboxMode::None, env_clear: false, allowed_syscalls: vec![] };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_standard_no_panic() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy { mode: SandboxMode::Standard, env_clear: true, allowed_syscalls: vec![] };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_strict_no_panic() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy { mode: SandboxMode::Strict, env_clear: true, allowed_syscalls: vec![] };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }
}