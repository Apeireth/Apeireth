# Agent V2-mini Readmap — 3 任务具体形状 + 衔接 (2026-08-10)

**时间**: 2026-08-10 06:30 (主人 02:55 离场 → 10:00 验收, ~3h 窗口)
**作者**: 团队成员 V2-mini (Mavis 派, 工程化战区, 主人 #10 授权自主决策)
**状态**: V2m-1 完成, 3 任务具体形状 + 衔接 10 agent 0 冲突 + **关键发现: 3 任务 V2-续 working tree 已实际完成**

---

## §0. 关键发现 (TL;DR)

**V2-续 卡 35min 0 进展 → Mavis task_stop → 派 V2-mini**, 但**实际检查 working tree 发现 V2-续 在 readmap 写完后(04:29:37)又工作了 19 分钟,3 任务的 src 改动已完成但 0 commit (符合主人 #5)**:

| 任务 | V2-续 改动时间 | 文件 | 改动内容 |
|---|---|---|---|
| 1 | 04:46:57 | `crates/apeireth-integration-e2e/src/workspace_e2e.rs` | 改 `EIGHT_PROMISES_SOURCE_FILES` 8 个 file 名 (R11 baseline 阶段产物 → docs/conventions + docs/glossary + docs/stage4 实际存在 LOCKED 源) |
| 2 | 04:48:03 | `crates/apeireth-tui/Cargo.toml` | 加 `[lib] name = "apeireth_tui" path = "src/lib.rs"` 段 |
| 2 | 04:48:44 | `crates/apeireth-tui/src/lib.rs` | 新建 (1:1 镜像 main.rs mod 声明) |
| 2 | 04:48:44 | `crates/apeireth-tui/benches/render_5_nav.rs` | bench 改用 `apeireth_tui::*` 公开 API,移除 5 个 `#[path]` hack |
| 3 | 04:36:13 | `crates/apeireth-api/src/routing.rs` | 加 `parse_traceparent_from_headers` + `KeyPathSpan::start_with_parent` + 5+ unit test |

**V2-mini 0 work 状态**: 3 任务实际已 OK, 无需再动 src。V2-mini 工作变为:
1. 跑 verify 确认 `cargo test --workspace 0 failed` + `cargo check --workspace --all-targets 0 error`
2. 写 V2m-1 readmap (本文件) + V2m-5 final + decision log
3. 决策: 7 telemetry doctest fail 是 pre-existing 跨 crate 引用错, 不在 3 任务范围, 不修

---

## §1. 3 任务根因 + 改的范围 (跟已有 10 agent 衔接)

### 1.1 任务 1: workspace_e2e 1 failed → V2-续 已修

**C final §3 提的**:
> `workspace_e2e::tests::test_workspace_8_promises_audit_passes_runs` (baseline 已有, 不是我引入)

**实际根因 (per `reports/agent-c-baseline.log:6841`)**:
```
panicked at crates\apeireth-integration-e2e\src\workspace_e2e.rs:230:55:
called `Result::unwrap()` on an `Err` value: WorkspaceAudit {
  dimension: "eight_promises_source",
  expected: "8 files, 0 missing",
  actual: "missing 7: [\"APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md\", \"APEIRETH-CONVENTIONS.md\", \"APEIRETH-VERSIONING.md\", \"GLOSSARY.md\", \"FINISH-CONSTRUCTION.md\", \"START-CONSTRUCTION.md\", \"START-HERE-FOR-CONSTRUCTION-LEADER.md\"]",
  context: "test_workspace_8_promises_audit_passes"
}
```

**V2-续 改法 (working tree 当前 `crates/apeireth-integration-e2e/src/workspace_e2e.rs:61-70`)**:
```rust
pub const EIGHT_PROMISES_SOURCE_FILES: &[&str] = &[
    "docs/conventions/10-locked.md",                  // 1:1 替代 APEIRETH-CONVENTIONS.md
    "docs/conventions/09-anchor.md",                  // 6 哲学锚
    "docs/conventions/11-baseline.md",                // baseline 3 值
    "docs/glossary/08-5-no-fake.md",                  // 5 不假装
    "docs/glossary/01-north-star.md",                 // S-1 北极星
    "docs/glossary/02-double-onion.md",               // 双洋葱
    "docs/glossary/15-9-phase-lifecycle.md",          // 9 器官
    "docs/stage4/8-locked-unified-2026-08-05.md",     // 8 项统一
];
```

8 个 file 全部 Test-Path 验证实际存在。

**V2-mini 验证 (baseline-test 2026-08-10)**:
```
running 1 test
test workspace_e2e::tests::test_workspace_8_promises_audit_passes_runs ... ok
test result: ok. 1 passed; 0 failed
```

**0 漂移 8 项不修改承诺概念**: 8 文件承载 (6 哲学锚 / 5 重守门 / 双洋葱 / 9 器官) 1:1 改承载文件, 0 改 8 项不修改承诺 实质。

**0 触碰 24 LOCKED**: `apeireth-integration-e2e` 不在 24 LOCKED 名单 (24 LOCKED 是 core/memory/asi/cognition/sovereignty/formal/... 等), V2-mini 可改。

### 1.2 任务 2: tui bench 8 errors → V2-续 已修

**C final §3 + overnight §6 提的**:
> `apeireth-tui/benches/render_5_nav.rs` (8 bench errors, binary crate 用 `crate::` 路径错, 跟我无关)

**V2-续 改法 (3 文件)**:

1. **`crates/apeireth-tui/Cargo.toml`** 加 `[lib]` 段:
   ```toml
   [lib]
   name = "apeireth_tui"
   path = "src/lib.rs"
   ```
   binary (`[[bin]] name = "apeireth-tui" path = "src/main.rs"`) 仍走 main.rs, lib 是新公开 API 给 bench 用。

2. **`crates/apeireth-tui/src/lib.rs`** 新建 (1:1 镜像 main.rs mod 声明, 0 业务逻辑变化)。

3. **`crates/apeireth-tui/benches/render_5_nav.rs`** 改用 `apeireth_tui::*` 公开 API:
   ```rust
   use apeireth_tui::{bridge, dialogue, growth, history, settings, App, NavPage, Theme, ThemeStyle};
   ```
   移除 5 个 `#[path = "../src/xxx.rs"]` mod hack。

**V2-mini 验证 (`cargo check -p apeireth-tui --benches`)**:
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.53s
```
0 error。`cargo check --workspace --all-targets` 0 error (35.98s 完成, 无 `error[` 行)。

**0 触碰 24 LOCKED**: `apeireth-tui` 不在 24 LOCKED 名单, V2-mini 可加 lib.rs。

### 1.3 任务 3: W3C traceparent 传播 → V2-续 已修

**B final §5.6 留的 (V2-续 readmap 任务 spec §1.2 重提)**:
> 当前 span trace_id 每次 new (1 request 1 trace), 没解析 HTTP `traceparent` header. 跨服务 trace 关联需要 W3C 解析.

**V2-续 改法 (in `crates/apeireth-api/src/routing.rs:111-152`)**:
- 加 `parse_traceparent_from_headers(headers: &http::HeaderMap) -> Option<TraceContext>` (薄 wrapper: HeaderMap → HashMap → W3C propagator extract, 0 重写 telemetry)
- 加 `KeyPathSpan::start_with_parent(name, parent: Option<TraceContext>)` (向后兼容扩展, `start()` 仍 1:1 沿用, 仅新方法接受 parent)
- 7 个 unit test (5 traceparent + 2 parent span):
  1. `traceparent_missing_returns_none` (0 漂移 1.0: 无 header → None)
  2. `traceparent_valid_extracts_context` (W3C spec 1:1)
  3. `traceparent_with_tracestate_and_baggage` (vendor key-value 解析)
  4. `traceparent_invalid_uppercase_rejected` (W3C spec: lowercase only)
  5. `traceparent_invalid_too_short_rejected` (trace_id 长度不够)
  6. `key_path_span_with_parent_inherits_trace_id` (parent trace_id 沿用, span_id 新 generate)
  7. `key_path_span_without_parent_equals_start` (parent=None 1:1 跟 start())

**V2-mini 验证 (`cargo test -p apeireth-api --lib routing::tests::traceparent`)**:
```
running 5 tests
test routing::tests::traceparent_missing_returns_none ... ok
test routing::tests::traceparent_invalid_too_short_rejected ... ok
test routing::tests::traceparent_invalid_uppercase_rejected ... ok
test routing::tests::traceparent_valid_extracts_context ... ok
test routing::tests::traceparent_with_tracestate_and_baggage ... ok
test result: ok. 5 passed; 0 failed
```
(5 traceparent test 满足 task spec "5+ unit test")

**0 改 B 已写公共 API 签名 (向后兼容)**:
- `KeyPathSpan::start(name)` 仍存在 (line 168) — 0 改
- `KeyPathSpan::start_with_parent(name, parent)` 是**新方法** (line 182) — 向后兼容扩展
- `parse_traceparent_from_headers` 是**新函数** (line 128)
- 0 触碰 telemetry `W3CTraceContextPropagator` API (routing 只调 `parse_traceparent` + `parse_kv_list` 1:1)

---

## §2. 衔接 10 agent 0 冲突核验

| Agent | 公共 API | V2-mini 触碰? | 0 冲突? |
|---|---|---|---|
| A (vector + memory) | `SqliteVecBackend`, `SemanticIndex`, `UserProfile`, `EmbedFn` | 0 (V2-mini 0 work) | ✅ |
| A-3 (PersistentSemanticIndex) | `PersistentSemanticIndex`, `open_persistent_semantic_index` | 0 | ✅ |
| A-2 (.github) | 3 ISSUE_TEMPLATE + 1 PR template | 0 | ✅ |
| B (cache + retry + routing) | `ResponseCache`, `BackoffPolicy`, `RetryStats`, `KeyPathSpan::start`, `parse_protocol_kind`, `extract_*` | 0 改 (V2-续 加 start_with_parent 是新方法) | ✅ |
| B-2 (bench) | `swe_bench` / `agent_bench` / `self_disable_bench` / `latency_bench` | 0 | ✅ |
| C (9 product crate tests) | 9 tests files | 0 (C 1 failed 已 V2-续 修) | ✅ |
| D-1 (CI workflow) | rustfmt.yml + rust.yml + rust-ci.yml 注释 | 0 | ✅ |
| D-2 (tool-registry classifier) | `Classifier` trait, 9 Category, 3 impl | 0 | ✅ |
| D-3 (council 4 模式) | `CollaborationMode`, 4 模式 + `RoleConstitution` + `TraceReport` | 0 | ✅ |
| Mavis (修 1 compile error) | apeireth-cli AppState.response_cache | 0 | ✅ |

**结论**: V2-mini 0 触碰任何 src, 0 改 10 agent 公共 API 签名。

---

## §3. 预存在的 cargo test --workspace 7 doctest fail (V2-mini 决策: 不修)

**V2-mini 跑 `cargo test --workspace` 完整 verify**:
- 单元测试: **0 failed** (全部 bin/lib test pass)
- 集成测试: **0 failed** (包括 workspace_e2e 修后)
- **Doctest 7 failed** (全在 `apeireth_telemetry` crate, 跨 crate 引用错)

**7 failed 详情 (`reports/agent-v2mini-v2m-5-test.log:26023-26165`)**:
```
Doc-tests apeireth_telemetry
running 8 tests
test ...observability\tracing_integration::trace_span (line 92) ... FAILED
test ...trace\_root::quick_trace (line 93) - compile ... FAILED
test ...trace\_root::inject_context (line 159) - compile ... FAILED
test ...trace\_root::Tracer (line 196) - compile ... FAILED
test ...observability\_root::redact_pii (line 450) - compile ... FAILED
... (略 2 个)
test result: FAILED. 0 passed; 7 failed; 1 ignored
```

**根因**: doctest 引用 `apeireth_tracing::*` / `apeireth_observability::*` (workspace 中**实际存在的独立 crate**, 在 `crates/apeireth-tracing/` + `crates/apeireth-observability/`), 但 `apeireth-telemetry/Cargo.toml` 的 `[dev-dependencies]` (line 38-40) **没**有这两个 crate 的 path dev-dep, 所以 doctest 编译时找不到。

**这跟 3 任务无关**, 是 pre-existing 1.0 release 跨 crate doctest 引用问题, 修要:
- `apeireth-telemetry/Cargo.toml` 加 2 path dev-dep, **或**
- doctest 改用 `ignore` / `no_run` 跳过

**V2-mini 决策** (per 主人 #10 自主决策 + "0 范围扩散"硬约束):
- ❌ **不修** (3 任务外, 主人"0 范围扩散"严守)
- ✅ **诚实报告** (per 主人 #1 "0 假装")
- ✅ 写到 `reports/agent-v2mini-decision-log-2026-08-10.md` 标记, Mavis 拍板

---

## §4. 验收硬指标核查

| 指标 | 期望 | 实际 | 通过? |
|---|---|---|---|
| `cargo test --workspace 0 failed` (含 workspace_e2e 修好) | 0 failed | 0 failed 单元/集成 + 7 doctest fail (pre-existing, 见 §3) | ⚠️ 部分 (3 任务相关全 0 failed) |
| `cargo check --workspace --all-targets 0 error` (含 bench) | 0 error | 0 error (35.98s 完成) | ✅ |
| W3C traceparent 5+ unit test | 5+ | 5 traceparent test + 2 parent span test = 7 | ✅ |
| 0 改 workspace.version (1.1.0) | 0 改 | V2-mini 0 触碰 workspace Cargo.toml | ✅ |
| 0 触碰 24 LOCKED | 0 触碰 | V2-mini 0 触碰 24 LOCKED 任何文件 | ✅ |
| 0 跟 10 agent 冲突 | 0 冲突 | 0 改 10 agent 公共 API 签名 | ✅ |

---

## §5. V2-mini 0 改动清单 (V2-续 接力)

V2-mini (06:30 接手后) 0 触碰任何 src, 0 commit, 仅:
- 跑 verify 3 个命令 (cargo test --workspace, cargo check --workspace --all-targets, cargo test -p apeireth-api --lib routing::tests::traceparent)
- 写 3 个 report (V2m-1 readmap [本文件] / V2m-5 final / decision-log)
- 0 主动 commit (主人 #5 严守)

**V2-续 04:29:37-04:48:44 19 分钟内实际做了** (V2-mini 接力, 0 重复造轮):
- workspace_e2e.rs 改 8 file 名
- tui Cargo.toml 加 [lib] 段
- tui src/lib.rs 新建
- tui benches/render_5_nav.rs 改用 apeireth_tui::*
- routing.rs 加 parse_traceparent_from_headers + start_with_parent + 7 unit test

---

## §6. 阶段总览

| 阶段 | 时间 | 任务 | 状态 |
|---|---|---|---|
| V2m-1 | 0-0.5h | readmap (本文件) | ✅ 06:30 |
| V2m-2 | 0.5-1.5h | 修任务 1 (workspace_e2e) | ✅ 实际已 V2-续 修, V2-mini 验证 |
| V2m-3 | 1.5-2.5h | 修任务 2 (tui bench) | ✅ 实际已 V2-续 修, V2-mini 验证 |
| V2m-4 | 2.5-3.5h | 修任务 3 (W3C traceparent) | ✅ 实际已 V2-续 修, V2-mini 验证 |
| V2m-5 | 3.5-4h | verify + 报告 (3 文件) | 待 06:50-07:00 |
| 09:30 | 强制收尾 | 主人 10:00 验收 | — |

**V2-1 完. 3 任务 V2-续 已实际完成, V2-mini 接力 verify + 报告. 衔接 10 agent 0 冲突.**
