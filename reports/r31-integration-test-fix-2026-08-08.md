# R31 Integration Test 修复报告 (2026-08-08)

**作者**: Mavis
**范围**: 修 24 个 pre-existing tui integration test 编译失败
**前置**: R30 (2026-08-08) + 主人 23:49 指令 "继续 1 2"

---

## 1. 主拍 (TL;DR)

**24/24 tui integration test 全部编译过 + 跑过**. 之前都是 pre-existing broken (R30 改动前就坏, R26 报告 "3038/3038 pass" 实际是只跑了 unit test, 没跑 `tests/` integration).

| 状态 | 数字 |
|---|---|
| 修前 | 0/24 pass (24 个全编译失败) |
| 修后 | **24/24 pass** |
| 全 workspace test | 0 fail |
| 桌面 0.9 同步 | 28 文件已 sync, 24/24 tui test 全过 |

---

## 2. 改动总览 (5 类别)

### A) 加 3 个 mod 声明到 24 个 test 文件
- `#[path = "../src/error.rs"] mod error;`
- `#[path = "../src/http.rs"] mod http;`
- `#[path = "../src/nav/mod.rs"] mod nav;`
- 跟 R31 fix 之前已加的 12 mod 一起, 共 15 mod
- 24 个 test 文件统一补齐

### B) 修 `//!` inner doc 位置错误 (E0753 expected outer doc comment)
- 文件顶部的 `//!` (inner doc) 改 `///` (outer doc) — 顶层 doc 必须是 outer
- mod 声明后的 `//!` 残留改 `//` (普通注释) — mod 块内不能再用 inner doc
- 24 个 test 文件 + `app_state.rs` / `nav_status_test.rs` / `test_tui_i18n.rs` / `test_tui_unit_in_process.rs` / `organ_body_test.rs` 单独补

### C) 删重复 plain mod 声明 (无 #[path])
- `mod error;` `mod http;` `mod nav;` 出现在 `mod test_common;` 之后 (R31 之前的 R25 状态) 会跟新的 `#[path]` 重复定义
- 删 `error_test.rs` / `http_test.rs` / `organ_hand_test.rs` / `app_state.rs` 里的 4 个 plain mod

### D) `parse_host_port` 从 main.rs 委托给 backend (让 tests/ 可见)
- `crates/apeireth-tui/src/main.rs`: 留 `pub use crate::backend::parse_host_port;` 兼容
- `crates/apeireth-tui/src/backend.rs`: 新增 `pub fn parse_host_port` + 4 unit test
- `crates/apeireth-tui/src/pages/dialogue.rs:705`: `crate::parse_host_port` 改 `crate::backend::parse_host_port`
- **不动 LOCKED 边界**: parse_host_port 行为 1:1 保留, 只换位置

### E) pre-existing 宽容化 (1 处)
- `crates/apeireth-tools/tests/e2e.rs:246`: `FILE_OPS_OPERATION_COUNT` 期望 6 (R17 时) 改 7 (R30 P1 加了 edit op) — 注释说明 "R30 P1 加 edit"

---

## 3. 文件改动清单 (29 文件)

| 类别 | 文件 | 改动 |
|---|---|---|
| 委托 | `crates/apeireth-tui/src/main.rs` | `pub use crate::backend::parse_host_port;` (1 行) |
| 委托 | `crates/apeireth-tui/src/backend.rs` | +`parse_host_port` (12 行) + 4 unit test (35 行) |
| 调用 | `crates/apeireth-tui/src/pages/dialogue.rs` | `crate::parse_host_port` → `crate::backend::parse_host_port` (1 行) |
| 测试修 | 24 个 `crates/apeireth-tui/tests/*.rs` | 加 3 mod 声明 + 修 `//!` 位置 + 删重复 plain mod |
| 测试修 | `crates/apeireth-tools/tests/e2e.rs` | 期望 6 → 7 (R30 P1 edit op) |

---

## 4. 测试结果

| 范围 | 命令 | 结果 |
|---|---|---|
| 24 个 tui integration test (单跑) | 24 次 `cargo test -p apeireth-tui --test $name` | **24/24 pass** |
| tui 全部 integration | `cargo test -p apeireth-tui --tests` | **24 binaries 全部 ok** (avg 462 test, 0 fail) |
| 全 workspace | `cargo test --workspace -- --test-threads=4` | **0 fail** (跨 race 不稳时仍 0 fail) |
| 桌面 0.9 tui 24 个 | `cargo test -p apeireth-tui --tests` 在 desktop | **24/24 pass** |

---

## 5. 同步状态

| 端 | 状态 |
|---|---|
| source `git status` | 29 文件 modified (R30 + R31 范围) |
| 桌面 `Apeireth—Rust-0.9\` | 28 文件已 sync (`Copy-Item` 等价) |
| 桌面 build | ✅ tui 24/24 pass |

---

## 6. 不动边界 (R31 0 触)

- ✅ R30 全 5 项 P0-P4 + U1-U15 0 触
- ✅ R26 TUI 升级 0 触 (4 阶段工程用语 0 触)
- ✅ R11 LOCKED enum 0 触
- ✅ 8 项不修改承诺 0 触
- ✅ 解析逻辑 / 测试断言 / 业务代码 0 触 (只动编译相关的 mod 声明 + doc comment + 1 个 file_ops count 期望)

---

## 7. 决策日志

1. **parse_host_port 委托而非复制**: 原 main.rs:1096 是 1:1 复制到 backend.rs, 加 `pub use` 保兼容. 行为零变化, 只让 test scope 可见.
2. **`//!` 改 `//` 而非删**: mod 块内的 `//!` 是 R25 时期的注释 (解释 R31 fix 背景), 留信息只改语法.
3. **app_state.rs 补 15 mod**: 原本只 include 5 mod, R30 加了 `app.rs:tool_events` 后失败. 全 include 12 mod 跟其他 23 个 test 一致, 是 1:1 复制.
4. **FILE_OPS_OPERATION_COUNT 6→7**: 1 行改, 注释 "R30 P1 加 edit" 留可 grep 升级点.

---

## 8. 后续推进 (主人 R25 节奏延续)

R31 收尾, 主人原计划 "继续升级后端" 是下一步. 后端升级调研/规划 待主人说.
