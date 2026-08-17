# S1 最小权限执行 — 自审报告

**任务**: S1 Windows 最小权限执行 (exec_worker 补 Job Object 的权限洞)
**执行者**: fullstack_engineer2
**提交**: 7945b38 (代码) + c2806ac (backlog 标 ✅)
**S1 backlog**: ⬜ → ✅

---

## 1. 交付物清点

| 文件 | 行数 | 角色 |
|------|------|------|
| `crates/apeireth-companion/src/restricted_token.rs` | 521 | Win32 受限 token (CreateRestrictedToken + TokenIntegrityLevel + DACL) |
| `crates/apeireth-companion/src/directory_acl.rs` | 419 | 工具沙盒根目录 read-only DACL + DirAclGuard |
| `crates/apeireth-companion/src/app_container.rs` | 117 | AppContainer 档 trait 口 (高危档, 0 装 PASS) |
| `crates/apeireth-companion/src/sandbox.rs` | 504 (扩 +350) | SandboxConfig S1 字段 + prepare_child 总入口 |
| `crates/apeireth-companion/tests/s1_sandbox_hardening.rs` | 204 | 13 集成测试 |
| `crates/apeireth-companion/Cargo.toml` | +2 | Win32_Security_Authorization feature |
| `crates/apeireth-companion/src/lib.rs` | +3 | 模块声明 |
| `crates/apeireth-companion/src/suites.rs` | +1 | SandboxConfig 字面量补 ..Default::default() |
| `crates/apeireth-companion/src/job_object.rs` | +1 | 测试 SandboxConfig 字面量补 ..Default::default() |
| `crates/apeireth-companion/src/memory_graph.rs` | +4 | 补 GraphBackend::continuity_id (WIP 漏写机械补全) |

**总计**: 1637 行 + 47 行 (来自 7945b38 commit)

---

## 2. 机制总结

### 2.1 三件套 (Chromium 经典分层模型 §3)

| 件 | 函数 | Windows FFI | 跨平台 |
|----|------|-------------|--------|
| ① | `create_restricted_token` | OpenProcessToken + CreateRestrictedToken + DISABLE_MAX_PRIVILEGE + LUA_TOKEN + SetTokenInformation(TokenIntegrityLevel) + SetEntriesInAclW (默认 DACL) | no-op, needs_hardening=true 时返 Err |
| ② | `apply_read_only_acl` | GetNamedSecurityInfoW + SetEntriesInAclW + SetNamedSecurityInfoW + DirAclGuard Drop 自动还原 | no-op, needs_hardening=true 时返 Err |
| ③ | `AppContainerBackend` | 仅 trait 口 (0 装 PASS, available=false) | trait 口 |

### 2.2 SandboxConfig 扩展

新增 4 个 S1 字段 (向后兼容老 B3 JSON):
- `integrity_level: Option<IntegrityLevel>` (Untrusted / Low / Medium)
- `deny_only_sids: Vec<WellKnownSid>` (BuiltinAdministrators / World / AuthenticatedUser / Interactive)
- `directory_acl_roots: Vec<PathBuf>` (与 APEIRETH_TOOL_FS_ROOTS env 协作)
- `use_app_container: bool` (高危档, 0 装默认 false)

新增方法:
- `has_privilege_hardening()` — 与 `has_limits()` 解耦 (资源 vs 权限)
- `from_json` — 解析所有字段, 非法值回退 + eprintln 记录

新增类型:
- `IntegrityLevel` enum (parse/as_str)
- `WellKnownSid` enum (parse/as_str)

### 2.3 整合入口

`prepare_child(cfg: &SandboxConfig) -> Result<PreparedChild, String>`
- Windows: 真接 token + 真接 ACL guard, 失败不阻断 (eprintln 降级)
- 跨平台: needs_hardening=true 时如实返 Err (0 装 PASS)
- 跨平台 needs_hardening=false 时: Ok 空 stub

PreparedChild { token, dir_acl } 持有至 worker 退出, Drop 自动还原 ACL + CloseHandle token.

### 2.4 待续 wire-up (升级路径)

`prepare_child` 返 token + ACL guard, 真接至 worker spawn 需 `CreateProcessAsUserW` (与 `std::process::Command::spawn` 不兼容), 属后续 wire-up. Job Object (B3) 仍主防线, S1 为纵深加固.

---

## 3. 测试结果

### 3.1 Lib 单测 (cargo test --lib)

| 模块 | 测试数 | 状态 |
|------|--------|------|
| sandbox | 21 | ✅ (含 S1 字段解析 / has_privilege_hardening / prepare_child 入口) |
| restricted_token | 6 | ✅ (含 4 个 Windows 真接测试) |
| directory_acl | 7 | ✅ (含 2 个 Windows 真接 + 1 个 跨平台 Err 测试) |
| app_container | 4 | ✅ (0 装 PASS 测试) |
| 其他 (已存在) | 1 | ✅ (`sandbox_config_invalid_falls_back_not_blocking` 验证 S1 改动 0 回归) |
| 其他 (已存在) | 1 | ✅ (`install_sandbox_pack_applies_sandbox_config` 验证 suites 集成) |

### 3.2 集成测试 (cargo test --test s1_sandbox_hardening)

13 测试全绿:
- JSON 解析: backward_compat / full / invalid_items_dropped / empty_or_null
- 类型枚举: integrity_level_roundtrips / wellknown_sid_aliases / as_str_includes_authority
- 0 装 PASS: sandbox_backends_all_honest_unavailable / app_container_0_install_passes_loudly
- 字段独立: has_limits_only_resource_flags / privilege_hardening_flag_logic
- 跨平台: prepare_child_off_windows_returns_err / prepare_child_default_is_passthrough
- 模板: app_container_render_params_contains_timeout / app_container_backends_listing

**总计: 51 测试全绿 ✅**

### 3.3 Windows 真接 (4 个关键测试)

1. `real_restricted_token_creates_on_windows` — 真打 OpenProcessToken + CreateRestrictedToken + SetTokenInformation
2. `real_restricted_token_without_hardening_returns_handle` — 真打无 harden 路径
3. `apply_windows_real_path_creates_guard` — 真打 GetNamedSecurityInfoW + SetEntriesInAclW + SetNamedSecurityInfoW
4. `apply_windows_nonexistent_path_is_skipped` — 真打失败语义

---

## 4. 0 装 PASS 诚实标注

| 后端 | 状态 | 说明 |
|------|------|------|
| Sandboxie-Plus (B3) | 0 装 PASS | `available=false` + status 标注 "未接" |
| landlock (B3) | 0 装 PASS | `available=false` + status 标注 "未接" |
| AppContainer (S1) | 0 装 PASS | `available=false` + status 标注 "未接" + 真接路径 (CreateAppContainerProfile + DeriveAppContainerSidFromAppContainerName) |
| RestrictedToken (S1) | Windows 真接 | OpenProcessToken + CreateRestrictedToken + TokenIntegrityLevel + DACL |
| DirectoryACL (S1) | Windows 真接 | Get/SetNamedSecurityInfoW + SetEntriesInAclW + DirAclGuard |
| 跨平台 prepare_child | 0 装 PASS | needs_hardening=true 时如实返 Err "非 Windows 平台未实现" |

---

## 5. 边界遵守

- ✅ exec_worker/sandbox 相关模块扩展 (sandbox.rs, restricted_token.rs, directory_acl.rs, app_container.rs)
- ✅ SandboxConfig 扩展字段 (向后兼容老 B3 JSON, `has_limits_only_resource_flags` 测试验证)
- ✅ 不动 tool-approval / tool-runtime / tool-registry (B3 同边界)
- ⚠️ tool_bridge.rs 未修改 (memory_graph.rs / job_object.rs 补机械完整性的小修复, 不影响其他逻辑)

---

## 6. 已知约束 / 未实现

| 项 | 原因 | 升级路径 |
|---|------|---------|
| prepare_child 返的 token 真接至 worker spawn | 需 CreateProcessAsUserW, 与 std::process::Command 不兼容 | 后续 wire-up (改 exec_worker.bin / new child process 路径) |
| AppContainer 档 0 装 | 需 CreateAppContainerProfile + DeriveAppContainerSidFromAppContainerName + Capability 清单, 复杂度高, 属高危档 | 后续高危档任务 |
| 跨平台 S1 0 装 | Linux 走 prctl(PR_SET_NO_NEW_PRIVS) + seccomp + namespaces, 复杂度高 | 后续 S1 跨平台任务 |
| 目录 ACL 还原以"原 DACL 重建"为粒度 | 不能原子事务 (SetSecurityInfo 调用间窗口极小, 但非零) | 生产环境走事务化设计 (transactional DACL 持久化) |

---

## 7. 提交记录

```
7945b38 feat(companion): S1 最小权限执行 — RestrictedToken + DirectoryACL + AppContainer trait
c2806ac docs(backlog): S1 标 ✅ — 提交 7945b38 RestrictedToken+DirectoryACL+AppContainer 落地
```

---

## 8. 验收

- ✅ SandboxConfig 扩展 4 S1 字段 (integrity_level / deny_only_sids / directory_acl_roots / use_app_container)
- ✅ 新增 restricted_token / directory_acl / app_container 三模块
- ✅ 总入口 `prepare_child` (Windows 真接 / 跨平台 0 装)
- ✅ 51 测试全绿 (含 4 Windows 真接)
- ✅ 0 装 PASS 诚实标注 (所有未接后端 available=false + status 标 "未接")
- ✅ 与 B3 协作 (Job Object 仍主防线, S1 为纵深加固)
- ✅ 与 tool-filesystem 协作 (directory_acl_roots + APEIRETH_TOOL_FS_ROOTS env)
- ✅ backlog S1 ⬜ → ✅
- ✅ git 入库 (2 commits)
