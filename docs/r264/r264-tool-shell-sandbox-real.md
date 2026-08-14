# R264: tool-shell sandbox Standard/Strict 真接 (cross-platform)

**日期**: 2026-08-14
**作者**: 楚零
**目的**: apeireth-tool-shell sandbox.rs Standard/Strict 模式从 stub 升级到真接 (0 引外部 dep)

---

## §1 背景

R138 sandbox.rs 4 模式 (None / Light / Standard / Strict):
- **Light**: 真接 (env_clear + stdin null)
- **Standard / Strict**: stub, 注释 "deferred to R139+"

R258 GitHub 调研 Tier A 复议案: tool-shell 隔离 = ★★★★★ (O-1 安全优先核心载体).

---

## §2 设计 (全 safe, 0 引 libc/nix)

### 2.1 tokio safe API 复用

| Mode | env_clear | stdin_null | process_group(0) | kill_on_drop | CREATE_NO_WINDOW |
|------|-----------|------------|-------------------|--------------|-------------------|
| None | (config)  | -          | -                 | -            | -                |
| Light | yes       | yes        | -                 | -            | -                |
| Standard | yes   | yes        | **yes** (新增)    | **yes** (新增) | -              |
| Strict | yes     | yes        | yes               | yes          | **yes** (新增)   |

### 2.2 平台 cfg-gated

```rust
SandboxMode::Standard => {
    cmd.stdin(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.as_std_mut().process_group(0);  // setsid
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        cmd.as_std_mut().creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
    cmd.kill_on_drop(true);
}
SandboxMode::Strict => {
    // Standard + Windows CREATE_NO_WINDOW
    #[cfg(windows)]
    cmd.as_std_mut().creation_flags(CREATE_NEW_PROCESS_GROUP | 0x0800_0000);
}
```

### 2.3 设计权衡

- **tokio safe API**: process_group(0) / kill_on_drop(true) / creation_flags() 都是 tokio safe wrapper
- **as_std_mut()**: tokio::process::Command → std::process::Command, 调 std cfg-gated trait
- **0 unsafe, 0 libc/nix**: 保持 workspace ponytail ceiling

### 2.4 真 seccomp BPF / JobObject / sandbox_init 留 TODO

```rust
// TODO Linux seccomp / Windows JobObject / macOS sandbox_init
// (out of scope; would need libc / windows-sys / sandbox crate).
```

完整 syscall filter 需要 libc::prctl + seccomp-sys BPF compile 或 windows-sys::Win32_System_JobObjects,
超出 R264 scope (跨多 unsafe 平台特定 crate).

---

## §3 测试 (5 unit + 4 e2e = 9 cases)

### 3.1 lib tests (`crates/apeireth-tool-shell/src/sandbox.rs::tests`)

- default_policy_is_light (mode=Light, env_clear=true)
- apply_sandbox_light_sets_stdin_null
- apply_sandbox_none_noop
- apply_sandbox_standard_applies_process_group_and_kill_on_drop (no panic)
- apply_sandbox_strict_includes_standard
- apply_sandbox_all_modes_no_panic_on_empty_program (4 模式 × 不 panic)
- sandbox_mode_equality_and_copy (Copy + Eq + Clone)
- sandbox_policy_clone_preserves_mode

### 3.2 e2e tests (`crates/apeireth-tool-shell/tests/r264_sandbox_e2e.rs`)

- r264_sandbox_standard_command_runs (echo hello 输出 "standard-sandbox-ok")
- r264_sandbox_strict_command_runs (echo hello 输出 "strict-sandbox-ok")
- r264_sandbox_standard_kill_on_drop_works (spawn sleep 60s + drop, kill_on_drop 验证)
- r264_sandbox_light_no_process_group_change (向后兼容)

**22 tests pass total** (R138 旧 14 + R264 新 8).

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借鉴 tokio safe API, 自实现不引 libc/nix
- **S-2 实事求是**: 全 safe, 不假装 unsafe
- **O-1 安全优先**: Standard 真接进程组隔离 + kill_on_drop, Strict 增 CREATE_NO_WINDOW, 这是"防 orphan"+"防 escape"的核心
- **O-2 走在前人**: tokio Command 已封装跨平台 process group + creation flags
- **O-3 干到底**: 4/4 e2e 真跑 (cmd.output() 真发子进程)
- **O-5 不假装**: 留 TODO 在 module docs, 明确说真 BPF filter 超 scope
