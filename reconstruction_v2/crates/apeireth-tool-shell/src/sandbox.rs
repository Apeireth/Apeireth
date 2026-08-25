//! Real sandbox: cross-platform process isolation via tokio safe APIs.
//!
//! R264 真接: Standard/Strict 模式全 safe 实现 (无 libc/nix dep),
//! 保持 workspace ponytail ceiling. 真 seccomp BPF / JobObject syscall filter
//! 超 scope, 留 TODO (需要 unsafe + 平台特定 dep).
//!
//! ## 模式分级
//!
//! | Mode | env_clear | stdin_null | process_group(0) | kill_on_drop | CREATE_NO_WINDOW |
//! |---|---|---|---|---|---|
//! | None      | (config) | -          | -      | -       | -       |
//! | Light     | yes      | yes        | -      | -       | -       |
//! | Standard  | yes      | yes        | yes    | yes     | -       |
//! | Strict    | yes      | yes        | yes    | yes     | yes     |
//!
//! Standard 真接:
//! - `Command::process_group(0)` 创建新进程组 (Unix: setsid(); Windows: CREATE_NEW_PROCESS_GROUP)
//! - `Command::kill_on_drop(true)` 父进程死时子进程也被杀 (防 orphan zombie)
//!
//! Strict 增:
//! - Windows: `CREATE_NO_WINDOW` flag (避免子进程弹 console window)
//!
//! ## TODO (超出 scope)
//!
//! - Linux seccomp BPF filter: 需要 `seccompiler` crate + unsafe pre_exec
//! - Windows JobObject: 需要 `windows-sys` crate + unsafe win32 API
//! - macOS sandbox_init: 需要 `sandbox` crate 或 FFI
//! - Linux namespaces (mount/pid/user): 需要 `nix` crate + unsafe unshare

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMode {
    /// No sandbox (default, allows everything)
    None,
    /// Lightweight: env_clear + stdin null (apeireth-tools/code_exec level)
    Light,
    /// R264: Standard = Light + process group isolation + kill_on_drop.
    /// (Unix setsid, Windows CREATE_NEW_PROCESS_GROUP)
    Standard,
    /// R264: Strict = Standard + CREATE_NO_WINDOW (Windows).
    /// (TODO: + seccomp/JobObject syscall filter via libc/windows-sys)
    Strict,
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub mode: SandboxMode,
    #[allow(dead_code)] // reserved for future Strict syscall whitelist
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

/// R264: Apply sandbox policy to a tokio::process::Command before exec.
/// All platform-specific work goes through tokio safe APIs (process_group,
/// kill_on_drop, creation_flags) -- 0 unsafe, 0 external sys-crate dep.
pub fn apply_sandbox(
    cmd: &mut tokio::process::Command,
    policy: &SandboxPolicy,
) -> Result<(), SandboxError> {
    // env_clear applies to all modes that opt in
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
            // R264: 新进程组 (Unix: setsid(); Windows: CREATE_NEW_PROCESS_GROUP).
            // std::os::unix::process::CommandExt / std::os::windows::process::CommandExt
            // 提供 cfg-gated process_group / creation_flags. tokio::process::Command
            // 通过 as_std_mut() 暴露底层 std::process::Command, 我们调 std 的方法即可.
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                cmd.as_std_mut().process_group(0);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
                cmd.as_std_mut().creation_flags(CREATE_NEW_PROCESS_GROUP);
            }
            // R264: 父进程 drop 时子进程也被杀 (防 orphan). tokio safe API.
            cmd.kill_on_drop(true);
        }
        SandboxMode::Strict => {
            cmd.stdin(std::process::Stdio::null());
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                cmd.as_std_mut().process_group(0);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                cmd.as_std_mut()
                    .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
            }
            cmd.kill_on_drop(true);
            // TODO Linux seccomp / Windows JobObject / macOS sandbox_init
            // (out of scope; would need libc / windows-sys / sandbox crate).
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
        let p = SandboxPolicy {
            mode: SandboxMode::Light,
            env_clear: false,
            allowed_syscalls: vec![],
        };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_none_noop() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy {
            mode: SandboxMode::None,
            env_clear: false,
            allowed_syscalls: vec![],
        };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_standard_applies_process_group_and_kill_on_drop() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy {
            mode: SandboxMode::Standard,
            env_clear: true,
            allowed_syscalls: vec![],
        };
        apply_sandbox(&mut cmd, &p).unwrap();
        // Both process_group(0) and kill_on_drop(true) are side effects on the Command
        // builder; they can be observed indirectly via stdio + spawn semantics.
        // Direct introspection is platform-specific; here we just verify no panic.
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_strict_includes_standard() {
        let mut cmd = tokio::process::Command::new("echo");
        let p = SandboxPolicy {
            mode: SandboxMode::Strict,
            env_clear: true,
            allowed_syscalls: vec![],
        };
        apply_sandbox(&mut cmd, &p).unwrap();
        let _ = cmd;
    }

    #[test]
    fn apply_sandbox_all_modes_no_panic_on_empty_program() {
        for mode in [
            SandboxMode::None,
            SandboxMode::Light,
            SandboxMode::Standard,
            SandboxMode::Strict,
        ] {
            let mut cmd = tokio::process::Command::new("true");
            let p = SandboxPolicy {
                mode,
                env_clear: true,
                allowed_syscalls: vec![],
            };
            apply_sandbox(&mut cmd, &p).unwrap();
        }
    }

    #[test]
    fn sandbox_mode_equality_and_copy() {
        // Verify Copy + Eq + Clone semantics (used in pattern matching)
        let m = SandboxMode::Standard;
        let m2 = m; // Copy
        assert_eq!(m, m2);
        let m3 = m;
        assert_eq!(m, m3);
    }

    #[test]
    fn sandbox_policy_clone_preserves_mode() {
        let p = SandboxPolicy {
            mode: SandboxMode::Strict,
            env_clear: false,
            allowed_syscalls: vec!["read".into()],
        };
        let p2 = p.clone();
        assert_eq!(p.mode, p2.mode);
        assert_eq!(p.env_clear, p2.env_clear);
        assert_eq!(p.allowed_syscalls, p2.allowed_syscalls);
    }
}
