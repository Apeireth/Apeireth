# apeireth-tool-shell

## R264: sandbox Standard/Strict 真接 (cross-platform)

| Mode | env_clear | stdin_null | process_group(0) | kill_on_drop | CREATE_NO_WINDOW |
|------|-----------|------------|-------------------|--------------|-------------------|
| None      | (config) | -          | -      | -       | -       |
| Light     | yes      | yes        | -      | -       | -       |
| Standard  | yes      | yes        | yes    | yes     | -       |
| Strict    | yes      | yes        | yes    | yes     | yes     |

**全 safe 实现** (0 引 libc/nix), 复用 tokio safe API:
- `cmd.as_std_mut().process_group(0)` (Unix=setsid, Windows=CREATE_NEW_PROCESS_GROUP)
- `cmd.kill_on_drop(true)` (父 drop 时子进程被杀)
- Windows: `creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)` (避免弹窗)

TODO (超 scope): Linux seccomp BPF / Windows JobObject / macOS sandbox_init
需 libc / windows-sys / sandbox crate + unsafe pre_exec, 留给 R139+ 或后续 R.

用法:
```rust
use apeireth_tool_shell::sandbox::{apply_sandbox, SandboxMode, SandboxPolicy};

let mut cmd = tokio::process::Command::new("echo");
let policy = SandboxPolicy { mode: SandboxMode::Standard, env_clear: true, allowed_syscalls: vec![] };
apply_sandbox(&mut cmd, &policy).unwrap();
cmd.arg("hello").spawn().unwrap();
```


**R138** — Shell 执行工具

## 职责

真实 shell / SSH / 计算器统一 trait: 替代 VCP LinuxShellExecutor (113KB 字符串黑名单伪安全).

## 核心模块

- local.rs (本地 shell)
- ssh.rs (russh 纯 Rust SSH)
- sandbox.rs (seccomp / Job Object)
- persist.rs (task 持久化)
- stream.rs (实时 stdout/stderr)

## 借鉴 vs 超越

VCP 六层"安全" = 字符串黑名单 → 我们真沙箱 (process::Command + seccomp filter / Windows Job Object).

## 0 假装

✅ 19 单元测试 | ✅ 真 sandbox 路径 | ⚠️ 真 seccomp 需 Linux

## R162 lint cleanup

61 -> 0 warnings. Test-only SandboxMode import moved into tests mod.
## R166 public API deep cleanup

`VCP_SHELL_COMMAND_COUNT` -> `LEGACY_SHELL_COMMAND_COUNT`. 19 tests pass.
