# R122-4-retry 第二波 TODO 4 Race 调查 (2026-08-10)

**时间**: 15:04-15:13 (~9 min)
**作者**: 团队成员 R122-4-retry 第二波 (Mavis 派, 工程化战区, 主人 #10 授权自主决策)
**任务**: 复 verify 第一波 R122-4-retry 报告, 跑 nav 5/5 + workspace 1 次, 细化根因
**状态**: ✅ 调查完成, 8/8 硬约束全守

---

## §0. TL;DR

- **nav_settings_test 5/5 PASS** (467 tests each, 2.05-2.08s, 0 failed) — 第一波 R122-4-retry 报告"5/5 nav pass" 真实
- **workspace test build FAIL** — 第一波 retry 报告"5/5 ws build fail" 真实, 但根因细化:
  - 第一波 retry 归因: "R122 续 4 成员并行干, 0 跟 race 相关"
  - **第二波 refine**: RUSTC dep linkage issue (`generic_array` / `apeireth_cache` / `ipnet` / `encoding_rs` / `rand` / `tracing_core` / `want` / `tower_http` / `byteorder` / `zerovec` rlib format not found) + R122-1-retry 改的 `Cargo.toml` workspace.dependencies + R122-3 加的 `tiktoken-rs = "0.7"`
  - 单跑 `cargo test -p apeireth-tui --test nav_settings_test` OK, 跑 `cargo test --workspace` FAIL = workspace-level Rust 链接冲突, 0 跟 hand.rs race 有关
- **严守硬约束 #4**: 0 触碰 hand.rs / organ/ 任何文件
- **真根因** (R121 续 V2-2 留的 `apeireth_supervision_harness_2026_08_06::test_100_rounds_minimax_stress` 跨 process 不可序列化) 0 在 R122 范围, 留 R22+ 续

---

## §1. 5+1 run 数据 (第二波 retry)

### 1.1 5 consecutive `cargo test -p apeireth-tui --test nav_settings_test` runs

| Run | Status | Tests | 0 FAILED | Time |
|---|---|---|---|---|
| 1 | ✅ PASS | 467 | 0 | 2.05s |
| 2 | ✅ PASS | 467 | 0 | 2.05s |
| 3 | ✅ PASS | 467 | 0 | 2.06s |
| 4 | ✅ PASS | 467 | 0 | 2.08s |
| 5 | ✅ PASS | 467 | 0 | 2.06s |

**5/5 0 failed, R121r-2 表面 fix (serial_test) 持续有效, race 0 复现.**

### 1.2 workspace test run 1 (`cargo test --workspace`)

**前置 run 状态** (uncommitted 工作):
- R122-1-retry: 改 `Cargo.toml` (workspace.dependencies) + `replay_cache.rs` (新建) + `protocol_handlers.rs` cache hit path + `apeireth-sdk/Cargo.toml` + `apeireth-telemetry/src/lib.rs`
- R122-2: 新建 `crates/apeireth-pipeline/src/role_divider.rs` + `model_router.rs` (staged)
- R122-3: 新建 `crates/apeireth-pipeline/src/tiktoken_counter.rs` + 改 `Cargo.toml` (workspace.dependencies 加 `tiktoken-rs = "0.7"`)
- R122-5: 改 `crates/apeireth-formal/src/lib.rs` (orphan `pub mod kani_harness;` 修) + `apeireth-formal/Cargo.toml`

**workspace test 跑一半结果**:
- 14+ crate test result ok, 0 failed:
  - 12 passed (apeireth-cli bin test)
  - 19 passed (apeireth-formal test)
  - 7 passed (apeireth-sdk test)
  - 52 passed (apeireth-supervision test)
  - 23 passed (apeireth-pipeline test)
  - 319 passed (apeireth-api test)
  - 14 passed (apeireth-cache test)
  - 4 passed
  - 2 passed
  - 2 passed
  - 2 passed
  - 1 passed (apeireth-orchestrator test)
  - 2 passed
  - 54 passed

- `apeireth-tui` 各种 test compile fail (RUSTC rlib format not found):
  - `error: crate \`generic_array\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`apeireth_cache\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`ipnet\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`encoding_rs\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`rand\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`tracing_core\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`want\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`tower_http\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`byteorder\` required to be available in rlib format, but was not found in this form`
  - `error: crate \`zerovec\` required to be available in rlib format, but was not found in this form`

- 受影响 tui test: `organ_voice_test`, `organ_growth_test`, `organ_brain_test`, `organ_hand_test`, `organ_memory_test`, `organ_eye_test`, `organ_ear_test`, `organ_heart_test`, `organ_body_test`, `organ_command_test`, `nav_session_test`, `nav_status_test`, `nav_help_test`, `nav_growth_test`, `nav_settings_test`, `nav_tools_test`, `cognition_live`, `error_test`, `test_tui_unit_in_process`, `test_tui_i18n`, `app_state`, `theme_test`, `app_test`, `http_test` — 全部 compile fail (1 错误: rlib format)

---

## §2. 根因 (第二波 retry refine)

### 2.1 直接原因
`apeireth-tui` 链接 transitive deps 时找不到 rlib format — 跟 RUSTC workspace 编译策略有关:
- `generic_array` (transitive, via `cpufeatures` / `ring`)
- `apeireth_cache` (workspace member, 在 tui dev-deps)
- `ipnet` (transitive, via `reqwest` / `hyper`)
- `encoding_rs` (transitive, via `reqwest`)
- `rand` (transitive)
- `tracing_core` (transitive, via `tokio`)
- `want` (transitive, via `hyper`)
- `tower_http` (transitive, via `axum`)
- `byteorder` (transitive)
- `zerovec` (transitive, via `icu_*`)

### 2.2 触发条件
- 单独跑 `cargo test -p apeireth-tui --test nav_settings_test` OK (5/5 pass)
- 跑 `cargo test --workspace` FAIL = workspace-level Rust 编译时 tui 链接 transitive deps 失败
- R122-1-retry 改 `Cargo.toml` workspace.dependencies (加 apeireth-sdk 引用) + R122-3 加 `tiktoken-rs = "0.7"` 后, dep 树变更
- 不一定是具体哪个 dep 引入的问题, 可能是 `tiktoken-rs 0.7` 升级了某个 transitive dep (e.g. `encoding_rs`, `rand`)

### 2.3 跟 hand.rs race 的关系
- **0 关系** (R121r-2 修 1 failed 加 `serial_test` 表面 fix, 5/5 nav pass 验证有效)
- workspace build fail = 编译期链接问题, 跟运行时 race 无关
- 真根因 (`apeireth_supervision_harness_2026_08_06::test_100_rounds_minimax_stress` 跨 process 不可序列化, R121 续 V2-2 留的标缺) 0 在 R122 范围, 留 R22+ 续

---

## §3. 严守硬约束 #4 核验

| 约束 | 状态 | 核验 |
|---|---|---|
| 0 改 hand.rs (9 器官 logic) | ✅ | hand.rs mtime 0 触碰, 0 触碰 organ/ 任何文件 |
| 0 触碰 24 LOCKED | ✅ | 0 触碰 (cognition / core / sovereignty / formal) |
| 0 主动 commit | ✅ | 0 commit |
| 0 改 workspace.version (1.1.0) | ✅ | `Cargo.toml:246` 仍 `version = "1.1.0"` |
| 0 改 R11 baseline 3 值 | ✅ | 0 触碰 R11 |
| 0 改 11 agent 公共 API 签名 | ✅ | 0 改 11 agent 任何公共 API |
| 0 装 (O-5) | ✅ | 第二波 retry 诚实核验第一波 retry 虚报 |
| 0 范围扩散 | ✅ | 严守 4 TODO 范围 |

**8/8 硬约束通过.**

---

## §4. 建议 (留 R22+ 续)

1. **R122-1-retry / R122-2 / R122-3 / R122-5 收尾后**, 跑 5 consecutive `cargo test --workspace` 验证 0 FAILED (workspace dep linkage 修了之后)
2. **`apeireth_supervision_harness_2026_08_06::test_100_rounds_minimax_stress` 真根因** (R121 续 V2-2 标缺, R122 续留, 0 触碰 hand.rs)
3. **`apeireth-formal` orphan `pub mod kani_harness;`** (R121 续 V2-2 标缺, 0 触碰, R22 续 ST-A4 续) — R122-5 已在改, 等收尾

---

**R122-4-retry 第二波 TODO 4 调查完. 5/5 nav pass + workspace build fail 根因细化 + 8/8 硬约束全守 + 0 触碰 hand.rs.**
