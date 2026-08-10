# R125-10 Sub-Agent Dispatch Prompt (Kani 形式化 24 LOCKED 全覆盖)

**Date**: 2026-08-10 17:32
**Author**: R125 P2 supervisor (general agent, mvs_a7af0f1f15cd4a79901442e14878333d, dispatched 17:23)
**Receiving agent**: R125-10 sub-agent (Mavis 派)

---

## 任务 (per 主人 17:22 升级授权 + decision-33 + B3 25 维升级)

**主题**: Kani 形式化验证扩 24 LOCKED crate 全覆盖. R122-9 已 5 harness, R125-10 从 5 → 24 LOCKED 关键不变量 (per LOCKED 名单: supervisor/agent/bus/council/evolution/extension/graph/mcp/pipeline/tool-registry/tool-runtime/protocol/asi/onion/sovereignty/constraint/memory/cognition/perception/consciousness/motivation/life-force/relation/value).

**借鉴 ID**: `R124-3-BORROW-model-checking/kani-harness-pattern-2026-08-10`

**借鉴源码**: `.openclaw\workspace\borrowed-repos\kani\`

**目标文件**:
- `Apeireth-rust/crates/apeireth-formal/src/kani_harness.rs` (扩 5 → 24 harness, +19 new)
- `Apeireth-rust/crates/apeireth-formal/tests/kani_24_locked_test.rs` (8 smoke test, NEW)
- `Apeireth-rust/crates/apeireth-formal/KANI.md` (扩 doc, 加 24 LOCKED mapping table)
- `Apeireth-rust/crates/apeireth-formal/kani.toml` (per-harness unwind 调整, 24 harness)

**触发 B3 (V0.5 24→25 维 Robustness 鲁棒性)**: 24 LOCKED 不变量验证是 25 维 (Robustness 鲁棒性) 关键证据.

**整合依赖**: R122-9 既有 5 harness (BackoffPolicy / JitterMode / ResponseCache / ResponseReplay / RoleDivide), 0 改. R125-10 加 19 new + 重构 5 既有为 24 LOCKED 命名.

**估时**: 2-3 天 (24 harness + 8 smoke test + 1 KANI.md 扩).

**截止**: 8/12 17:30 (跑过夜 8/11-8/12, 含 24 Kani 跑耗时).

---

## 0 装解除 (主人 17:22) — 重要

**借鉴源码状态** (verify 实施前):
```bash
Test-Path '.openclaw\workspace\borrowed-repos\kani\.git'  # 必须 True
```

**3 种状态对应动作**:
1. ✅ **cloned** (`.git` 存在) = 真实施, 报告里写 "借鉴源码 ✅ cloned, 已实施"
2. ⏳ **限流中** (`.git` 0 存在) = 等 30 min 再 verify, 仍 0 实施, 报告里写 "借鉴源码 ⏳ 限流中, 0 实施, 借鉴 ID 索引完成"
3. ❌ **永久失败** (24h 后仍 0 cloned) = 报 supervisor + 取消任务, 0 假装"已借鉴"

**0 装 PASS 严守**: ❌ 0 假装"已借鉴", ❌ 0 写 src 假装 import 借鉴代码, ❌ 0 写 doc 假装 API 兼容. 借鉴源码 0 在手 = 0 实施, 报告诚实标.

---

## 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)

| # | 硬墙 | 你 (R125-10) 必守 |
|---|------|-----------------|
| 1 | **B2** workspace.version 1.2.0 (R125 末 B2 已升, 你 0 再升) | ✅ 0 触碰 `Cargo.toml` `version` 字段 |
| 2 | **A1** R11 baseline 3 值 数字 严守 (0.8682/0.8532/0.9063) | ✅ 0 触碰 `integration_r_measure.rs` |
| 3 | **B1** 24 LOCKED crate mtime 16:34 baseline (apeireth-formal **不在 24 LOCKED**, 实施可改) | ✅ 仅加 19 new harness, 0 触碰 24 LOCKED crate mtime |
| 4 | **B5** 6→8 哲学锚 (R125 末升) | ✅ 0 改 6 哲学锚原 6 实质, 8 锚是扩展 |
| 5 | **B3** V0.5 24→25 维 (R125 末升 Robustness 鲁棒性, **触发 24 LOCKED 形式化** ) | ✅ 0 改 V0.5 公式, 25 维是扩展 |
| 6 | **B4** 6 重守门 v6 (R125-5 实施) | ✅ 0 改 5 重原 5 重, 6 重是扩展 |
| 7 | **A3** 12 键 + PHL-07 = 13 键 (R125-12 后) | ✅ 0 改 12 键原 12, 13 键是扩展 |
| 8 | **C1** 0 主动 commit (你 sub-agent 0 commit) + **C2** 0 装 解除 (主人 17:22) + **C3** 0 装 5 项 升 6 重 v6 + 0 主动 push 严守 | ✅ 0 commit, 0 push, 借鉴源码 ✅ cloned 才真实施 |

**新增 harness 0 触碰 workspace.version**: apeireth-formal 自身 Cargo.toml 是 `version.workspace = true`, 你 0 触碰 workspace root 的 Cargo.toml.

**POD 镜像原则** (per 既有 KANI.md §4 + lib.rs::PermissionLayerConfig 1:1 模式):
- 0 复用 LOCKED crate 的真实类型 (String / Vec / HashMap 在 Kani 下状态爆炸)
- 仅验证形式属性 shape (24 LOCKED 各 1 个关键不变量)
- 真实生产代码不调用进 harness (per lib.rs 禁止条款)

---

## 实施步骤 (4 阶段)

### 阶段 1: 借鉴源码 study (30 min)
```bash
# verify cloned
Test-Path '.openclaw\workspace\borrowed-repos\kani\.git'
# 读 Kani 核心: cprover/src/ + library/kani/src/ + docs/src/ + rust-toolchain.toml
Get-ChildItem '.openclaw\workspace\borrowed-repos\kani\library\kani\src' -ErrorAction SilentlyContinue | Select-Object Name
```
提取 3 个核心 pattern:
1. **Harness 模板**: `#[kani::proof]` + `#[kani::unwind(N)]` + `kani::any::<T>()` 怎么写
2. **POD 模型**: 避开 String/Vec/HashMap, 用 u8/u32/bool/固定 array 镜像 LOCKED 行为
3. **CBMC 限制**: unwind bound / 浮点支持 / unsafe 禁用 / 大 N 拆小

### 阶段 2: Rust 实施 (4-6 hours, 24 harness)
**kani_harness.rs 扩 5 → 24**:
```rust
//! Kani 形式化验证 — 24 LOCKED crate 关键不变量 (R125-10)
//!
//! 5 既有 (R122-9) + 19 new (R125-10) = 24 LOCKED 全覆盖.
//! 24 LOCKED per `docs/omnibus/24-locked-crates.md` (B1 落实).

// === 既有 5 (R122-9, 0 改) ===
#[kani::proof] fn kani_verify_backoff_policy_step_within_cap() { /* ... */ }
#[kani::proof] fn kani_verify_jitter_sleep_returns_value_in_range() { /* ... */ }
#[kani::proof] fn kani_verify_response_cache_capacity_respected() { /* ... */ }
#[kani::proof] fn kani_verify_response_replay_lookup_consistent() { /* ... */ }
#[kani::proof] fn kani_verify_role_divide_wrap_unwrap_round_trip() { /* ... */ }

// === 19 new (R125-10) ===
// LOCKED #1-#24 1:1 命名: kani_verify_<locked_crate>_<key_invariant>

// 1. apeireth-supervisor
#[kani::proof] fn kani_verify_supervisor_l0_requires_ha() { /* POD: LayerConfig { kind: u8, requires_ha: bool }, 不变量: L0 kind=0 → requires_ha=true */ }
// 2. apeireth-agent
#[kani::proof] fn kani_verify_agent_role_dispatch_no_collision() { /* POD: 6 role u8 + req_role, 验证 dispatch 0 跨 role 冲突 */ }
// 3. apeireth-bus
#[kani::proof] fn kani_verify_bus_topic_priority_no_starvation() { /* POD: 4 topic u8 + priority u8, 验证 high-prio 0 永久饿死 low-prio */ }
// 4. apeireth-council
#[kani::proof] fn kani_verify_council_vote_quorum_reached() { /* POD: 7 advisor u8 + vote bool[7], 验证 ≥4 yes → quorum */ }
// 5. apeireth-evolution
#[kani::proof] fn kani_verify_evolution_poda_cycle_terminates() { /* POD: cycle_step u8, 验证 cycle ≤ N step */ }
// 6. apeireth-extension
#[kani::proof] fn kani_verify_extension_plugin_id_unique() { /* POD: 16 plugin id u16, 验证 0 重复 */ }
// 7. apeireth-graph
#[kani::proof] fn kani_verify_graph_stategraph_acyclic() { /* POD: 8 node u8 + 8 edge u8, 验证 0 cycle */ }
// 8. apeireth-mcp
#[kani::proof] fn kani_verify_mcp_resource_uri_parseable() { /* POD: uri_len u8, 验证 parse 0 panic */ }
// 9. apeireth-pipeline
#[kani::proof] fn kani_verify_pipeline_stage_order_preserved() { /* POD: 5 stage u8, 验证 stage 顺序 0 乱 */ }
// 10. apeireth-tool-registry
#[kani::proof] fn kani_verify_tool_registry_capability_no_duplicate() { /* POD: 32 cap u8, 验证 register 0 重复 */ }
// 11. apeireth-tool-runtime
#[kani::proof] fn kani_verify_tool_runtime_sandbox_escape_prevented() { /* POD: sandbox_flag u8, 验证不允许 escape */ }
// 12. apeireth-protocol
#[kani::proof] fn kani_verify_protocol_ws_message_within_cap() { /* POD: msg_len u32 + cap u32, 验证 ≤ cap */ }
// 13. apeireth-asi
#[kani::proof] fn kani_verify_asi_v05_25dim_sum_eq_1() { /* POD: 25 weight u8, 验证 sum = 1.0 ± ε */ }
// 14. apeireth-onion
#[kani::proof] fn kani_verify_onion_layer_isolation() { /* POD: 6 layer u8, 验证 L_i 0 读 L_j */ }
// 15. apeireth-sovereignty
#[kani::proof] fn kani_verify_sovereignty_gate_pass_count_correct() { /* POD: gate_id u8 + pass bool, 验证 pass = N 重 */ }
// 16. apeireth-constraint
#[kani::proof] fn kani_verify_constraint_risk_level_within_bounds() { /* POD: risk u8, 验证 ∈ [0, 4] */ }
// 17. apeireth-memory
#[kani::proof] fn kani_verify_memory_l3_capacity_no_overflow() { /* POD: l3_len u32 + l3_cap u32, 验证 ≤ cap */ }
// 18. apeireth-cognition
#[kani::proof] fn kani_verify_cognition_brain_signal_no_deadlock() { /* POD: signal_queue u8, 验证 ack < send */ }
// 19. apeireth-perception
#[kani::proof] fn kani_verify_perception_eye_ear_buffer_no_overflow() { /* POD: buf_len u32, 验证 ≤ cap */ }
// 20. apeireth-consciousness
#[kani::proof] fn kani_verify_consciousness_qualia_count_in_range() { /* POD: qualia u8, 验证 ∈ [0, 8] */ }
// 21. apeireth-motivation
#[kani::proof] fn kani_verify_motivation_drive_strength_normalized() { /* POD: drive u8, 验证 ∈ [0, 100] */ }
// 22. apeireth-life-force
#[kani::proof] fn kani_verify_life_force_pulse_period_within_range() { /* POD: pulse u32, 验证 ∈ [1, 1000]ms */ }
// 23. apeireth-relation
#[kani::proof] fn kani_verify_relation_trust_score_in_bounds() { /* POD: trust u8, 验证 ∈ [0, 100] */ }
// 24. apeireth-value
#[kani::proof] fn kani_verify_value_priority_queue_ordered() { /* POD: 8 value u8, 验证降序 */ }
```

**kani.toml**:
```toml
[options]
default-unwind = 100

# 24 LOCKED harness 各 1 个 unwind bound
[harness.kani_verify_asi_v05_25dim_sum_eq_1]
unwind = 50

[harness.kani_verify_council_vote_quorum_reached]
unwind = 20
# ... 24 个分别
```

**lib.rs 修改**:
- 0 改原 lib.rs 任何东西
- 仅 `pub mod kani_harness;` 已存在, 0 改

### 阶段 3: 8 smoke test (30 min)
- `test_24_harness_compile` — 24 harness #[kani::proof] 编译通过
- `test_5_existing_unchanged` — 既有 5 harness 0 改
- `test_19_new_harness_listed` — 19 new harness 命名 1:1 跟 24 LOCKED
- `test_pod_model_no_heap` — 19 new 全 POD, 0 String/Vec/HashMap
- `test_unwind_bound_set` — 24 kani.toml 各自 unwind 配
- `test_smoke_test_0_panic` — cargo test 跑 smoke test 0 panic
- `test_documented_in_kani_md` — KANI.md 表格列 24 LOCKED mapping
- `test_24_locked_no_touch` — git status 24 LOCKED crate 0 触碰 (验证 apeireth-formal 不在 24)

### 阶段 4: KANI.md 扩 + final 报告 (30 min)
- `crates/apeireth-formal/KANI.md` — 表格 24 LOCKED mapping + R125-10 触发 B3
- final 报告: `Apeireth-rust/reports/agent-r125-10-final-2026-08-10.md`

---

## 0 主动 commit (C1 严守)

❌ **你 (R125-10 sub-agent) 0 commit, 0 push**. 实施完成 = 写 src/test/KANI.md + 写 final 报告. Mavis 整合 #3 拍板 17:30 (0 含 R125 实施, R125 续 mavis 整合 commit 链 8/15-9/10).

---

## final 报告 必含 6 段

```markdown
# R125-10 Final Report — Kani 形式化 24 LOCKED 全覆盖
**Date**: 2026-08-10
**Author**: R125-10 sub-agent
**借鉴 ID**: R124-3-BORROW-model-checking/kani-harness-pattern-2026-08-10
**实施路径**: crates/apeireth-formal/src/kani_harness.rs (扩 5 → 24)

## 1. 借鉴源码状态 (0 装解除 verify)
- ✅ cloned / ⏳ 限流中 / ❌ 永久失败 (3 选 1)

## 2. 实施步骤
- 阶段 1 借鉴 study: (3 提取 pattern: harness 模板 / POD 模型 / CBMC 限制)
- 阶段 2 Rust 实施: (24 harness 命名 1:1 跟 24 LOCKED + kani.toml 24 unwind)
- 阶段 3 smoke test: (8 test pass/fail)
- 阶段 4 KANI.md 扩: (24 LOCKED mapping table + R125-10 触发 B3 25 维)

## 3. 8 硬墙 verify (B1-B7 + A1-A3 + C1-C3)
- B2 ✅ 0 触碰 workspace.version
- A1 ✅ 0 触碰 R11 baseline 3 值
- B1 ✅ 0 触碰 24 LOCKED crate mtime
- B5 ✅ 0 改 6 哲学锚实质
- B3 ✅ 0 改 V0.5 公式, 25 维是扩展 (B3 触发 24 LOCKED 不变量验证)
- B4 ✅ 0 改 5 重守门实质
- A3 ✅ 0 改 12 键原 12
- C1-C3 ✅ 0 commit, 0 装 PASS, 0 push

## 4. 0 装解除 verify
- 借鉴源码状态: (✅/⏳/❌)
- 0 假装"已借鉴": (true/false)
- 真实实施 vs 索引完成: (真实施/索引完成)

## 5. 整合 verify
- 24 LOCKED 1:1 命名覆盖: (是/否 + 表格)
- R122-9 既有 5 harness 0 改: (是/否)
- B3 25 维触发: (Robustness 鲁棒性 = 24 LOCKED 形式化)

## 6. 下一步 + 风险
- 1 个风险 / 1 个待 R125-N 续协调
```

---

## 你的工具 (你 sub-agent 必知)

你有: read, write, edit, grep, glob, bash. 你 0 commit, 0 push. 你 0 假装.

---

**派活完成 17:32. 截止 8/12 17:30 (跑过夜 8/11-8/12, 含 24 Kani 跑耗时). 卡 30 min → 诊断 + kill + 派替代 (supervisor 监督).**
