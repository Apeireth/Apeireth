# apeireth-tool-shell

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
