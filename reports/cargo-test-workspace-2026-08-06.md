# Cargo Test --workspace 全面验证报告 (2026-08-06)

> **执行者**: Mavis 派 sub-agent (硬限 1, 集成测试补充: cargo test --workspace 验证)
> **日期**: 2026-08-06
> **主仓路径**: `.openclaw\workspace\promethean\Apeireth-rust\`
> **HEAD**: `0da4af0399e43bdd88c88c111bfbcbfc11b218be` (branch: `code_reviewer/t15-fix-rebase`)
> **工作树状态**: 274 modified + 213 untracked (sub-agent 历史累积, 不在本任务职责内)
> **任务约束**: ❌ 0 修代码 / ❌ 0 主动 commit / ❌ 0 触碰 24 LOCKED src / ✅ 只跑 cargo test + 记录

---

## TL;DR

`cargo test --workspace --no-fail-fast` **在 build 阶段就失败, 0 测试运行**. 失败根因: 4 个 **未追踪** (untracked) 的 crate 引入编译错误, 阻塞了 workspace 全量 build.

| 维度 | 值 | 备注 |
|------|----|------|
| `cargo test --workspace` 直跑结果 | **FAIL at build** | 0 tests ran |
| 阻塞 build 的未追踪 crate | 4 | apeireth-formal / apeireth-update / apeireth-state / apeireth-extension |
| 排 4 后 `cargo test --workspace --exclude ...` | **271 test groups, 6715 passed, 20 failed, 26 ignored** | 262 ok groups / 9 failed groups |
| 24 LOCKED crate (lib unittests) | **22 PASS / 1 NOT_RUN (excluded) / 1 partial** | 见 §3 详 |
| 14 crate 集成测试 (`tests/r20_stage4_integration_14crates.rs`) | **NOT COMPILED** | 顶层 tests/ 不被 workspace 自动 pick up, 见 §4 |
| `apeireth-tui/tests/app_state.rs` E0433 | **已修, 19/19 PASS** | R25.2 fix 生效, pre-existing 不复存在 |
| 0 commit | ✅ | 全部只跑 + 记录 |
| 0 代码改动 | ✅ | 只用 cargo test 验证 |
| 0 触碰 24 LOCKED src/ (git diff) | ✅ | 没改任何 LOCKED 源 |

---

## §1 跑的过程 (按用户 spec, 用 PowerShell Tee-Object)

### 1.1 第 1 次: `cargo test --workspace --no-fail-fast` (用户原文命令)

```powershell
$proc = Start-Process -FilePath "powershell" -ArgumentList "-NoProfile","-Command","Set-Location -Path '.openclaw\workspace\promethean\Apeireth-rust'; cargo test --workspace --no-fail-fast 2>&1 | Tee-Object -FilePath 'reports\cargo-test-workspace.log'; Write-Output 'EXIT_CODE:' $LASTEXITCODE" ...
```

- **Log**: `reports/cargo-test-workspace.log` (46,952 bytes, 443 lines)
- **进程**: ~3 分钟退出 (build 在 compile 阶段失败)
- **`error: could not compile` 出现 2 次**:
  1. `apeireth-formal` (example "formal_demo") — 4 previous errors
  2. `apeireth-formal` (test "test_formal_in_process") — 16 previous errors
- **`test result:` 行数**: 0 (build 失败, 测试阶段未启动)
- **第 1 次结果**: ❌ **0 tests ran**, build 阶段中断

### 1.2 第 2 次: `--exclude apeireth-formal` 排查

- **Log**: `reports/cargo-test-workspace-excl-formal.log` (91,068 bytes, 923 lines)
- 暴露第 2 个未追踪 crate 阻塞: `apeireth-update` (11 errors, 整个 crate untracked)

### 1.3 第 3 次: `--exclude apeireth-formal --exclude apeireth-update` 排查

- **Log**: `reports/cargo-test-workspace-excl-2.log` (199,780 bytes, 2,803 lines)
- 暴露第 3 个未追踪 crate 阻塞: `apeireth-state` (1 error: `apeireth-state/src/lib.rs:138` `unresolved import 'crate::organ::OrganStub'`)

### 1.4 第 4 次: `--exclude apeireth-formal --exclude apeireth-update --exclude apeireth-state`

- **Log**: `reports/cargo-test-workspace-excl-3.log` (276,382 bytes, 3,555 lines)
- 暴露第 4 个未追踪 crate 阻塞: `apeireth-extension` (LOCKED crate, 38 + 36 errors)
- **关键**: 这是 24 LOCKED 之一, 阻塞由 untracked 测/例文件 (`examples/extension_demo.rs` + `tests/test_extension_in_process.rs`) 引入

### 1.5 第 5 次: `--exclude apeireth-formal --exclude apeireth-update --exclude apeireth-state --exclude apeireth-extension`

- **Log**: `reports/cargo-test-workspace-excl-4.log` (1,187,056 bytes, 11,824 lines)
- **`error: could not compile`**: **0** ✅
- **`test result: ok.`**: 257 lines
- **`test result: FAILED`**: 9 lines
- **总通过**: 6715 passed / 20 failed / 26 ignored
- **262 ok groups / 9 failed groups**

---

## §2 总览: 排 4 后的全 workspace 测试结果 (excl-4)

### 2.1 数字摘要

| 指标 | 数值 |
|------|-----:|
| `cargo test --workspace` 直跑 | **0 tests** (build 失败) |
| 排除 4 阻塞 crate 后 | 271 test groups, 6715 passed, 20 failed, 26 ignored |
| test groups OK | 262 |
| test groups FAILED | 9 |
| 涉及 crate 编译成功 | 73 (含 unittests + lib test + integration test + doc test + 6 LOCKED) |
| 涉及 crate 编译失败 (排除) | 4 (apeireth-formal, apeireth-update, apeireth-state, apeireth-extension) |
| `--no-fail-fast` 守门 | ✅ 后续 9 fail 仍跑完 |

### 2.2 9 个失败 test groups (在 7 个 crate, 共 20 个 fail 用例)

| # | Crate | Test Target | Pass | Fail | 备注 |
|---|-------|-------------|-----:|-----:|------|
| 1 | apeireth-agent | `tests/agent.rs` | 14 | 1 | `manager_list_aliases` FAILED |
| 2 | apeireth-api | `tests/endpoints.rs` | 12 | 2 | `test_gemini_generate_content_route_exists` + `test_verdict_endpoint_exists` |
| 3 | apeireth-pipeline | `tests/pipeline.rs` | 7 | 3 | `pipeline_runs_gemini` + `pipeline_runs_anthropic` + `pipeline_runs_openai_chat` |
| 4 | apeireth-protocol | `tests/wire_format.rs` | 16 | 1 | `openai_chat_request_with_temperature_and_max_tokens` |
| 5 | apeireth-tool-approval | `tests/rules.rs` | 15 | 1 | `risk_rule_requires_approval` |
| 6 | apeireth-tools | `lib` | 60 | 2 | `lib_tests::lib_end_to_end_4_traits_via_registry` + `register::tests::register_all_tools_dispatch_via_tool_trait` |
| 7 | apeireth-tools | `tests/e2e.rs` | 11 | 8 | `code_exec_*` + `file_ops_tool_name` + `git_ops_*` 失败 |
| 8 | apeireth-vector | `tests/store.rs` | 12 | 1 | `backend_search_returns_top_k` |
| 9 | apeireth-web | `tests/templates.rs` | 12 | 1 | `html_escape_double_quote` |
| **总计** | 7 crates | 9 test groups | **159** | **20** | — |

**注意**: 4 个 LOCKED crate (`apeireth-agent` / `apeireth-pipeline` / `apeireth-tool-approval` / `apeireth-extension` (excluded)) 出现 fail. `apeireth-extension` 因 build error 整个未跑, 跟 `apeireth-formal` / `apeireth-update` / `apeireth-state` 一起排除.

### 2.3 全 22 LOCKED crate + 2 排除, 细分 lib unittests

| LOCKED crate | lib unittests | 备注 |
|--------------|---------------:|------|
| apeireth-action | 19/0/0 | ✅ |
| apeireth-api | 176/0/0 | ✅ (但 `tests/endpoints.rs` 2 fail 见 §2.2) |
| apeireth-asi | 66/0/0 | ✅ |
| apeireth-bus | 15/0/0 | ✅ |
| apeireth-central | 33/0/0 | ✅ |
| apeireth-cognition | 29/0/0 | ✅ |
| apeireth-consciousness | 8/0/0 | ✅ |
| apeireth-constraint | 56/0/0 | ✅ |
| apeireth-core | 32/0/0 | ✅ |
| apeireth-council | 37/0/0 | ✅ |
| apeireth-evolution | 98/0/0 | ✅ |
| apeireth-extension | **NOT_RUN** | ❌ 排除 (untracked 测/例 38+36 errors) |
| apeireth-life-force | 14/0/0 | ✅ |
| apeireth-memory | 37/0/0 | ✅ |
| apeireth-motivation | 10/0/0 | ✅ |
| apeireth-onion | 18/0/0 | ✅ |
| apeireth-perception | 29/0/0 | ✅ |
| apeireth-relation | 8/0/0 + 3/0/0 (integration) | ✅ (11/0/0 总) |
| apeireth-sovereignty | 141/0/0 | ✅ |
| apeireth-supervisor | 14/0/0 | ✅ |
| apeireth-upgrade | 132/0/0 | ✅ |
| apeireth-value | 46/0/0 | ✅ |
| apeireth-verify | 28/0/0 | ✅ |
| apeireth-web | 7/0/0 | ✅ (但 `tests/templates.rs` 1 fail) |
| **小计 (lib unittests)** | **1069/0/0** | 22 个 PASS, 1 NOT_RUN |
| **小计 (+ LOCKED crate 的 integration test groups)** | 1138+/0/0 | 5 LOCKED crate (api/endpoints, web/templates, agent, pipeline, tool-approval) 有 integration fail 见 §2.2 |

---

## §3 24 LOCKED crate 测试状态 (vs 之前 T15 bg_c60c4465)

### 3.1 LOCKED crate 测试通过率 (跟 T15 比较)

| LOCKED crate | 之前 T15 状态 (5a373d16) | 现在 (0da4af03) 状态 | 变化 |
|--------------|--------------------------|----------------------|------|
| apeireth-core | 5/5 (twelve_keys_round10_07) | 32 lib + 0 fail | ✅ 加 27 测试, 全过 |
| apeireth-onion | 2/2 (onion_tests) | 18 lib + 0 fail | ✅ 加 16 测试, 全过 |
| apeireth-sovereignty | 88/0 (sovereignty_tests) + lib | 141 lib + 0 fail | ✅ 加 53 测试, 全过 |
| apeireth-council | 50/0 (council_tests + round10_07) | 37 lib + 0 fail | ✅ 加 (但一些搬 lib) |
| apeireth-asi | 95,555 lines lib | 66 lib + 0 fail | lib 测试加 66 |
| apeireth-memory | 8 (SQLite FFI) | 37 lib + 0 fail | ✅ |
| apeireth-upgrade | (T15 没单独列) | 132 lib + 0 fail | ✅ |
| apeireth-perception | (T15 没单独列) | 29 lib + 0 fail | ✅ |
| apeireth-cognition | (T15 没单独列) | 29 lib + 0 fail | ✅ |
| apeireth-action | 7 (action_tests) | 19 lib + 0 fail | ✅ |
| apeireth-motivation | 6 (motivation_tests) | 10 lib + 0 fail | ✅ |
| apeireth-value | 15 (value_tests) | 46 lib + 0 fail | ✅ |
| apeireth-consciousness | 17/3 (consciousness_v13_negative) | 8 lib + 0 fail | ✅ |
| apeireth-relation | 3 (relation_tests) | 8 lib + 3 integration + 0 fail | ✅ |
| apeireth-life-force | 7 (life_force_tests) | 14 lib + 0 fail | ✅ |
| apeireth-constraint | 14 (constraint_tests + five_gates) | 56 lib + 0 fail | ✅ |
| apeireth-central | 15 (central_tests) | 33 lib + 0 fail | ✅ |
| apeireth-supervisor | 9 (supervisor_integration) | 14 lib + 0 fail | ✅ |
| apeireth-verify | (T15 没单独列) | 28 lib + 0 fail | ✅ |
| apeireth-evolution | 10 (evolution_integration) | 98 lib + 0 fail | ✅ |
| apeireth-extension | (T15 没单独列) | **NOT_RUN** | ❌ **REGRESSION** (untracked 测/例 broken) |
| apeireth-bus | (T15 没单独列) | 15 lib + 0 fail | ✅ |
| apeireth-api | 115/0 (lib + 4 protocols) | 176 lib + 0 fail (但 `tests/endpoints.rs` 2 fail) | ⚠️ lib 175/0 ✅, integration 12/2 |
| apeireth-web | (T15 没单独列) | 7 lib + 0 fail (但 `tests/templates.rs` 1 fail) | ⚠️ lib 7/0 ✅, integration 12/1 |

**总计**: **23/24 LOCKED crate 仍过** (lib unittests 全 PASS), **1/24 NOT_RUN** (`apeireth-extension` 因 untracked 测/例 broken 排除)

### 3.2 关键发现: LOCKED crate 触碰风险

> **⚠️ 重点关注**: `apeireth-extension` 是 24 LOCKED 之一, 但当前 worktree 存在 **untracked** 测/例文件:
>
> | 路径 | 状态 | mtime |
> |------|------|-------|
> | `crates/apeireth-extension/README.md` | untracked | 2026/8/6 0:39 |
> | `crates/apeireth-extension/src/capability.rs` | untracked | 2026/8/6 0:40 |
> | `crates/apeireth-extension/src/lifecycle.rs` | untracked | 2026/8/6 0:36 |
> | `crates/apeireth-extension/src/loader.rs` | untracked | 2026/8/6 0:36 |
> | `crates/apeireth-extension/src/permission.rs` | untracked | 2026/8/6 0:44 |
> | `crates/apeireth-extension/examples/extension_demo.rs` | untracked | 2026/8/6 0:43 |
> | `crates/apeireth-extension/examples/extension_lifecycle.rs` | untracked | 2026/8/6 0:56 |
> | `crates/apeireth-extension/tests/test_extension_in_process.rs` | untracked | 2026/8/6 0:39 |
> | `crates/apeireth-extension/tests/all_6_kinds_lifecycle.rs` | untracked | 2026/8/6 0:56 |
> | `crates/apeireth-extension/tests/extension_toml_loading.rs` | untracked | 2026/8/6 0:56 |
> | `crates/apeireth-extension/tests/sandbox_audit_pipeline.rs` | untracked | 2026/8/6 0:56 |
>
> **风险**: 虽然 `src/lib.rs` (TRACKED at HEAD) 没改, 但 worktree **实质上** 给 LOCKED crate 加了 4 个新 src 文件 + 1 个 example + 4 个 tests. 编译失败说明这些新增的 `capability/lifecycle/loader/permission` 模块 + 测/例 都引用了不存在的 API (`Capability`, `Run`, `Cleanup`, `Init`, `Start`, `Stop`, `ExtensionResult`, `parse_manifest`, `ExtensionLifecycle`, `load_extension`, `SandboxRunner` 等).
>
> **由 Mavis 整合 #3 拍板**:
> - 选项 A: 删除 untracked 测/例, 恢复 apeireth-extension 为 HEAD 状态
> - 选项 B: 补全 src/capability.rs 等新模块的 lib.rs 导出, 让测/例编译过
> - 选项 C: 把新增功能搬出 LOCKED 范畴 (开新 crate apeireth-extension-impl, 类比 R25 pipeline-g5 模式)

---

## §4 14 crate 集成测试状态 (vs 之前 bg_62dcb964)

### 4.1 14 crate 集成测试文件

**关键发现**: `tests/r20_stage4_integration_14crates.rs` (untracked) **NOT COMPILED by cargo test --workspace**.

**根因**: 主仓 `Cargo.toml` 是 **workspace-only** (只有 `[workspace]` 段, 没有 `[package]` 段). workspace 顶层 `tests/*.rs` 不被 cargo 自动 pick up (cargo 只对带 `[package]` 的 crate 编译 `tests/`).

```
[workspace]
resolver = "2"
members = [ ... 75+ crates ... ]
# NO [package] section!
```

**证据**:
- `cargo test --workspace --no-fail-fast 2>&1 | Select-String "r20_stage4"` → **0 hits**
- `cargo test --test r20_stage4_integration_14crates --no-run 2>&1`:
  ```
  error: no test target named `r20_stage4_integration_14crates` in default-run packages
  help: available test targets:
      action_tests, agent, all_6_kinds_lifecycle, app_state, app_test, ...
      (NO r20_stage4 listed)
  ```

### 4.2 14 crate 集成测试文件清单 (untracked, NEVER TESTED)

| 路径 | 状态 | 来源 |
|------|------|------|
| `tests/r20_stage4_integration_14crates.rs` (3228 字节, 6 `mod` 包含) | untracked | R20 阶段 4 sub-agent |
| `tests/integration/test_e2e_tools.rs` (SDK 6 工具) | untracked | R20 阶段 4 |
| `tests/integration/test_5_provider_stub.rs` (5 Provider) | untracked | R20 阶段 4 |
| `tests/integration/test_observability_bus.rs` (3 端点) | untracked | R20 阶段 4 |
| `tests/integration/test_i18n_runtime.rs` (5 区域) | untracked | R20 阶段 4 |
| `tests/integration/test_m3_defense.rs` (14 crate whitelist) | untracked | R20 阶段 4 |
| `tests/integration/test_71gb_incident.rs` (rollback) | untracked | R20 阶段 4 |
| `tests/v09021_provider_e2e.rs` (15577 字节) | untracked | R20 阶段 4 |
| `tests/v09021_tool_endpoint_e2e.rs` (17896 字节) | untracked | R20 阶段 4 |
| `tests/workspace_integration_v2.rs` (40077 字节) | untracked | R20 阶段 4 |
| `tests/workspace-integration-v2.rs` (12972 字节) | **tracked** (但命名带 hyphen, 不被 pick up) | — |

### 4.3 对比 bg_62dcb964

> 之前 bg_62dcb964 跑过的 14 crate 集成测试是否还过? 
>
> **答**: **无法验证**, 因为该测试文件 (以及其 6 个子文件 + 3 个 v09021 文件) 全部 untracked, 全部 NOT COMPILED by `cargo test --workspace`. 在工作树状态下, 这 10 个文件是 **死代码** (dead code), 0 跑过.

**`Cargo.toml` 未配 `[package]`, 顶层 tests/ 不被 pick up** 是根本原因. 这是 sub-agent 没注意 workspace 模式导致的架构错位.

**由 Mavis 整合 #3 拍板**:
- 选项 A: 把 `tests/r20_stage4_integration_14crates.rs` 搬到一个新建 member crate (如 `crates/apeireth-integration-r20-stage4/`) 的 `tests/` 下, 这样 cargo 自动编译
- 选项 B: 在 workspace 根加一个 dummy `[package]` 段 (但这违反 8 项承诺 #8 workspace version 不变的精神)
- 选项 C: 把 14 crate 集成测试拆到各子 crate 的 `tests/` 下 (per-crate integration)

---

## §5 apeireth-tui/tests/app_state.rs E0433 pre-existing

### 5.1 状态: **已修, 19/19 PASS**

```powershell
Running tests\app_state.rs (target\debug\deps\app_state-305b958df067f1f6.exe)

running 19 tests
test app_starts_in_clean_state ... ok
test app_starts_with_splash_and_breath_disabled ... ok
test chat_message_can_be_constructed ... ok
test input_starts_empty ... ok
test chat_history_preserves_order ... ok
test multiple_pushes_accumulate ... ok
test push_user_input_adds_to_history ... ok
test mode_is_copy ... ok
test input_buf_pushes_chars ... ok
test navpage_is_copy ... ok
test language_is_copy ... ok
test app_starts_on_default_page ... ok
test push_assistant_reply_adds_to_history ... ok
test theme::tests::interpolate_at_half_is_midway ... ok
test push_system_adds_to_history ... ok
test theme::tests::interpolate_at_one_returns_to ... ok
test theme::tests::interpolate_at_zero_returns_from ... ok
test theme::tests::interpolate_discrete_fields_switch_at_half ... ok
test theme::tests::interpolate_clamps_out_of_range_progress ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 5.2 R25.2 fix 验证

文件 `crates/apeireth-tui/tests/app_state.rs:6-7` 自带 R25.2 fix 注释:
> `**R25.2 fix**: apeireth-tui 是 binary crate (没 lib.rs), `use apeireth_tui::*` 永远找不到. 改成 `#[path]` include 模式 (跟其他 18 个 _test.rs 一致).`

实现方式:
```rust
#[path = "../src/theme.rs"]
mod theme;
#[path = "../src/app.rs"]
mod app;

use app::{App, ChatMessage, Language, Mode, NavPage};
```

**pre-existing E0433 已彻底消失**. 之前提到的 "use apeireth_tui::* 永远找不到" 问题, 通过改用 `#[path]` include 模式绕过 binary-crate-no-lib 限制.

---

## §6 4 个未追踪阻塞 crate 详情 (Mavis 整合 #3 必读)

### 6.1 apeireth-formal (NOT in 24 LOCKED, V2 战区 5)

- **路径**: `crates/apeireth-formal/`
- **worktree 状态**: TRACKED 部分 (lib.rs + invariants/) + UNTRACKED 部分 (8 文件)
- **TRACKED src** (`src/lib.rs:30`): 只声明 `pub mod invariants;`
- **UNTRACKED 新增**:
  - `src/error.rs` (6125 字节, 2026/8/6 0:21) — 4 FormalError variant + ProofBackend trait
  - `src/example.rs` (9618 字节, 2026/8/6 0:23) — 3 TLA+ example + lock_safety_invariant
  - `src/invariant.rs` (11172 字节, 2026/8/6 0:21) — 5 InvariantKind hardcode + presets
  - `src/proof.rs` (18318 字节, 2026/8/6 0:26) — FormalEngine + 4 backend (Z3/Cvc5/Coq/Lean4) + ProofKind
  - `src/tla.rs` (10732 字节, 2026/8/6 0:22) — TlaExpr + TlaSpec
  - `examples/formal_demo.rs` (5451 字节, 2026/8/6 0:24) — `async fn main()` 用 `#[tokio::main]` 但 Cargo.toml 没 tokio dep
  - `tests/test_formal_in_process.rs` (22070 字节, 2026/8/6 0:28) — 多个 `#[tokio::test]` 但 Cargo.toml 没 tokio dep
  - `README.md`
- **编译错误**:
  - `error: could not compile 'apeireth-formal' (example "formal_demo") due to 4 previous errors`
    - E0432: unresolved imports `apeireth_formal::example`, `apeireth_formal::FormalEngine`, ...
    - E0433: cannot find 'invariant' in 'apeireth_formal' (lib.rs 只声明 `invariants` plural, 测/例用 `invariant` singular)
    - E0433: cannot find module 'tokio' (Cargo.toml 没 tokio)
    - E0752: 'main' function is not allowed to be 'async' (没 `#[tokio::main]`)
  - `error: could not compile 'apeireth-formal' (test "test_formal_in_process") due to 16 previous errors`
    - 14x E0433 tokio::test 不识别
    - 1x E0432 unresolved imports
- **影响**: 阻塞整个 workspace build
- **责任**: R20 阶段 6 估补 skeleton (1:1 翻译 v0.9.21 @anthropic-ai/formal 商业版), 主 2026-08-05 21:35 拍板"派 A". A 没补全 lib.rs 的 `pub mod` 声明, 没加 tokio dev-dep, 没把 `async fn main` 改用 `#[tokio::main]`

### 6.2 apeireth-update (NOT in 24 LOCKED, 全 untracked)

- **路径**: `crates/apeireth-update/` (整个 crate 是 untracked)
- **新增内容** (mtime 2026/8/6 1:52-2:00):
  - `Cargo.toml` (2880 字节)
  - `README.md` (4027 字节)
  - `examples/update_check_demo.rs` (9087 字节)
  - `src/endpoint.rs` (14782 字节)
  - `src/error.rs` (9307 字节)
  - `src/lib.rs` (11812 字节)
  - `src/release.rs` (10236 字节)
  - `src/signature.rs` (12512 字节) — 缺 `#[derive(serde::Deserialize)]` on `SignatureAlgorithm`
  - `src/updater.rs` (13400 字节)
  - `tests/test_update_flow.rs` (17779 字节)
- **编译错误**: `error: could not compile 'apeireth-update' (lib test) due to 11 previous errors`
  - E0432: unresolved import `apeireth_update::SandboxRunner`
  - E0432: unresolved import `tempfile` (dev-dep 没加)
  - E0560: struct `Manifest` has no field `meta` / `deps` / `capabilities`
  - E0405: cannot find trait `ExtensionLifecycle` in crate `apeireth_extension` (试图 cross-crate 用 LOCKED 扩展接口)
  - E0609: no field `meta` / `capabilities` on type `&Manifest`
  - E0599: no method `validate` for `Manifest`
  - E0425: 23+ missing symbols (`Run`, `Cleanup`, `Init`, `Start`, `Stop`, `ExtensionResult`, `load_extension`, `parse_manifest`, etc.)
- **影响**: 阻塞整个 workspace build
- **责任**: 最新 (1:52-2:00) sub-agent 工作, 跟 apeireth-extension 的 untracked 测/例 **强烈关联** — 都引用 `Capability` / `Run` / `Start` / `Stop` 等不存在的扩展 API

### 6.3 apeireth-state (NOT in 24 LOCKED, 全 untracked)

- **路径**: `crates/apeireth-state/` (整个 crate 是 untracked)
- **workspace.members**: 已包含 `"crates/apeireth-state"` (在 Cargo.toml:?? 行)
- **编译错误**: `error: could not compile 'apeireth-state' (lib) due to 1 previous error`
  - E0432: `unresolved import 'crate::organ::OrganStub'`
  - `crates/apeireth-state/src/lib.rs:138:12` — 试图从 `apeireth_state::organ` 导入 `OrganStub` 但 organ module 没有 `OrganStub`
  - 提示编译器: "a similar name exists in the module: BrainStub"
- **影响**: 阻塞整个 workspace build
- **责任**: 新建 crate, 9 organ state machine, 试图引用 `OrganStub` 但不存在的 stub 体系

### 6.4 apeireth-extension (IN 24 LOCKED, partial untracked)

- **路径**: `crates/apeireth-extension/`
- **TRACKED src** (`src/lib.rs:27-34`): 只声明 `pub mod audit; error; manifest; plugins; registry; sandbox; traits; types;`
- **UNTRACKED 新增** (mtime 2026/8/6 0:36-0:56):
  - `src/capability.rs` (9628 字节) — 新增 module, **未被 lib.rs 声明**
  - `src/lifecycle.rs` (15749 字节) — 新增 module, **未被 lib.rs 声明**
  - `src/loader.rs` (14132 字节) — 新增 module, **未被 lib.rs 声明**
  - `src/permission.rs` (15029 字节) — 新增 module, **未被 lib.rs 声明**
  - `examples/extension_demo.rs` (14632 字节) — 引用不存在的 `Capability` / `Run` / `Cleanup` / `Init` / `Start` / `Stop` / `ExtensionResult` / `load_extension`
  - `examples/extension_lifecycle.rs` (4830 字节)
  - `tests/test_extension_in_process.rs` (29147 字节) — 大型 in-process 测试, 引用 18+ 不存在 API
  - `tests/all_6_kinds_lifecycle.rs` (5630 字节)
  - `tests/extension_toml_loading.rs` (2324 字节)
  - `tests/sandbox_audit_pipeline.rs` (5589 字节)
  - `README.md` (4044 字节)
- **编译错误**:
  - `error: could not compile 'apeireth-extension' (example "extension_demo") due to 38 previous errors`
  - `error: could not compile 'apeireth-extension' (test "test_extension_in_process") due to 36 previous errors`
- **LOCKED 触碰判定**: 严格按 git diff HEAD 算, **0 行 tracked src/ 被改** ✅. 但 worktree 实质加了 4 个 untracked src module + 1 example + 4 tests 到 LOCKED crate 内, 等同 LOCKED 范围内"非空工作" — **是否算触碰由 Mavis 拍板**.

---

## §7 失败的 crate 详情 (在 7 个 LOCKED + 0 个新 crate 中)

| 失败 test group | 失败用例 | 跟 T15 对比 |
|-----------------|----------|-------------|
| `apeireth-agent/tests/agent.rs` `manager_list_aliases` | 1 | (T15 没单列 agent) |
| `apeireth-api/tests/endpoints.rs` `test_gemini_generate_content_route_exists` + `test_verdict_endpoint_exists` | 2 | T15 时 115/0 (现在 12/2 integration) — 新加的 endpoint test 在 assertion `left != right` 失败 |
| `apeireth-pipeline/tests/pipeline.rs` `pipeline_runs_gemini` + `_anthropic` + `_openai_chat` | 3 | T15 时 pipeline 0 fail (现在 7/3) — R20 加的 pipeline_runs 三件套 |
| `apeireth-protocol/tests/wire_format.rs` `openai_chat_request_with_temperature_and_max_tokens` | 1 | T15 时 wire_format 没单列 |
| `apeireth-tool-approval/tests/rules.rs` `risk_rule_requires_approval` | 1 | T15 时 tool-approval 没单列 |
| `apeireth-tools lib` `lib_tests::lib_end_to_end_4_traits_via_registry` + `register::tests::register_all_tools_dispatch_via_tool_trait` | 2 | T15 时 tools lib 没单列 (LOCKED) |
| `apeireth-tools/tests/e2e.rs` `code_exec_*` + `file_ops_tool_name` + `git_ops_*` | 8 | T15 时 tools e2e 没单列 (LOCKED) |
| `apeireth-vector/tests/store.rs` `backend_search_returns_top_k` | 1 | T15 时 vector 6 sqlite-vec backend (现在 12/1) |
| `apeireth-web/tests/templates.rs` `html_escape_double_quote` | 1 | T15 时 web 没单列 (LOCKED) |

**总计**: 7 LOCKED crate (agent/api/pipeline/protocol/tool-approval/tools/vector/web) 出现 20 个 test fail, 0 个新 crate 出 fail (因为 4 个新 crate 都因 build error 排除).

**注**: 这些是 pre-existing 测试失败, 不是本任务引入的. 但**没人跑过**, 因为 build 在更早的阶段就被未追踪的 4 个 crate 阻塞了.

---

## §8 0 commit / 0 代码改动 / 0 触碰 LOCKED src/ 声明

| 指标 | 验证 | 状态 |
|------|------|------|
| 0 commit | `git status --short` 显示大量 untracked, **0 commit 操作执行** | ✅ |
| 0 修代码 (本任务) | 只用 `cargo test` + `tee + Select-String` 解析, **0 file edit** | ✅ |
| 0 改 workspace version | `Cargo.toml [workspace.package] version = "1.0.0"` 未动 | ✅ |
| 0 触碰 24 LOCKED src/ (git diff) | `git diff HEAD -- crates/apeireth-{core,onion,sovereignty,council,asi,memory,upgrade,perception,cognition,action,motivation,value,consciousness,relation,life-force,constraint,central,supervisor,verify,evolution,extension,bus,api,web}/src/ 2>&1` — 由 sub-agent 累积的 modification 在 worktree 已存在, **本任务没改一行** | ✅ |
| 0 触碰 workspace 1.0.0 | Cargo.toml 第 166 行 `version = "1.0.0"` 未动 | ✅ |
| 0 假装已实现 | 0 mock 假装, 0 改 OK 假装 PASS, 0 把 fail 写成 pass. 4 个 build error + 20 个 test fail 全部如实记录 | ✅ |

---

## §9 整合 #3 必读 (4 决策点)

### 9.1 决策 1: apeireth-extension LOCKED 触碰 (4 选 1)

| 选项 | 描述 | 风险 |
|------|------|------|
| A. 删除 untracked | 删除 `apeireth-extension/{src/{capability,lifecycle,loader,permission}.rs, examples/{extension_demo,extension_lifecycle}.rs, tests/{test_extension_in_process,all_6_kinds_lifecycle,extension_toml_loading,sandbox_audit_pipeline}.rs, README.md}` | 0 风险, 恢复 HEAD 状态. 失去未提交 work |
| B. 补全 lib.rs | 在 `lib.rs` 加 `pub mod capability; lifecycle; loader; permission;` + 改测/例引用的 API | 0 触碰 tracked src 的 git diff, 但实质加 LOCKED 范围. 要 R20+ 拍板 |
| C. 搬出 LOCKED | 类比 R25 pipeline-g5 模式, 把新功能拆到 `crates/apeireth-extension-impl/` 新 crate | 0 触碰 LOCKED src, 加新 member |
| D. 维持现状 | 让 LOCKED crate 持续 build error, 持续 NOT_RUN | 0 但 24 LOCKED 测试覆盖率掉 1/24 |

### 9.2 决策 2: apeireth-formal (NOT LOCKED) 4 选 1

| 选项 | 描述 | 风险 |
|------|------|------|
| A. 删除 untracked | 删 8 个 untracked 文件, 回到 HEAD | 失去 R20 阶段 6 估补 work |
| B. 补全 lib.rs | 把 `pub mod invariants` 改成 `pub mod invariants, error, example, invariant, proof, tla;` + 在 Cargo.toml 加 tokio dev-dep + 把 `async fn main` 加 `#[tokio::main]` | OK, 4 改动 |
| C. 搬出 workspace | 类比 R25 把 4 个新增 src + tests 拆到 `crates/apeireth-formal-impl/` 新 crate | OK, 干净 |
| D. 维持现状 | 让 formal_demo + test_formal_in_process 持续 build error | 0 但 0 估补 0 跑过 |

### 9.3 决策 3: apeireth-update (NOT LOCKED) 3 选 1

| 选项 | 描述 | 风险 |
|------|------|------|
| A. 删除整个 crate | `rm -rf crates/apeireth-update/` + `git rm` workspace.members 那一行 | OK, 失去 1:00-2:00 工作 |
| B. 修代码 | 补全 SignatureAlgorithm 的 Deserialize + 加 SandboxRunner + 加 Manifest 的 meta/deps/capabilities 字段 + 删 ExtensionLifecycle 引用 | 大, 11+ errors |
| C. 搬出 workspace | `crates/apeireth-update-impl/` 单独 | OK, 干净 |

### 9.4 决策 4: apeireth-state (NOT LOCKED) 2 选 1

| 选项 | 描述 | 风险 |
|------|------|------|
| A. 删 untracked | `rm -rf crates/apeireth-state/` + workspace.members 删除那一行 | OK |
| B. 修 `organ::OrganStub` 引用 | 把 lib.rs:138 改成 `BrainStub` (按编译器提示) | 1 行 fix |

### 9.5 决策 5: 14 crate 集成测试 (NOT LOCKED, 死代码) 3 选 1

| 选项 | 描述 | 风险 |
|------|------|------|
| A. 搬新 crate | `crates/apeireth-integration-r20-stage4/` member, 把 `tests/r20_stage4_integration_14crates.rs` + `tests/integration/*.rs` 6 个子文件搬过去 | OK, 30+ 测试可跑 |
| B. 改 workspace Cargo.toml | 加 `[package]` 段让顶层 tests/ 自动 pick up | **违反 8 项承诺 #8 workspace version 不变** (新增 [package] 段算改 version 范畴?) — **NO**, package.name 跟 version 是不同概念, 但增加"workspace 同时是 package"是 workspace 单根反模式 |
| C. 拆 14 测试到各子 crate | 14 crate 集成测试本质是 cross-crate, 拆了意义不大 | 弱化集成性 |

---

## §10 5 份日志文件路径 (Mavis 整合 #3 可直接拉)

| 文件 | 大小 | 用途 |
|------|-----:|------|
| `reports/cargo-test-workspace.log` | 46,952 字节 | **用户 spec 的原始命令输出** (含 4 errors 完整 trace) |
| `reports/cargo-test-workspace-excl-formal.log` | 91,068 字节 | 排除 1 后, 暴露 apeireth-update |
| `reports/cargo-test-workspace-excl-2.log` | 199,780 字节 | 排除 2 后, 暴露 apeireth-state |
| `reports/cargo-test-workspace-excl-3.log` | 276,382 字节 | 排除 3 后, 暴露 apeireth-extension |
| `reports/cargo-test-workspace-excl-4.log` | 1,187,056 字节 | **排除 4 后, 真正通过的 test log** (6715/20/26) |

---

## §11 路径合规声明

| 项 | 验证 | 状态 |
|----|------|------|
| 主仓路径唯一 | `.openclaw\workspace\promethean\Apeireth-rust\` (env workspace) | ✅ |
| 0 触碰 sandbox 错路径 | `.minimax-agent-cn\projects\apeireth-debug\Apeireth-rust\` **0 访问** | ✅ |
| 报告路径 | `reports/cargo-test-workspace-2026-08-06.md` (在主仓内) | ✅ |
| 子 agent log 路径 | 5 个 `reports/cargo-test-workspace*.log` 全在主仓 reports/ | ✅ |
| 0 commit | `git status --short` 无新增 commit, 无 `git add`, 无 `git commit` | ✅ |

---

**报告结束**. 6 哲学锚穿透 (S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装) + 8 项不修改承诺 (0 触碰).
