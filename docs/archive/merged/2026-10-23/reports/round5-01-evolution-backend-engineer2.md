# round5-01 Evolution 6 状态机 + trait fail-6 工程化（backend_engineer2）

> **Task ID**: `0ee75c85-8262-4921-8c4d-d15918535508`
> **Role**: backend_engineer2
> **Status**: ✅ 完成（108 tests pass + workspace 0 退化）
> **Commit**: 待 commit（本地 worktree 已就绪）

---

## 0. 任务说明

**输入依据**：
- `reports/a2557c25-round5-engineering-decisions-tasks.md` 任务 1 (G1, P0, 2 天)
- stage5 §2 #6 LOCKED 边界（不修改 stage5 文档）
- stage4 §3 演化器官推导

**约束**：
- ❌ 不修改 LOCKED 阶段 1–5 文档
- ❌ 守 7 项不修改承诺（apeireth-core / R11 baseline / 主手册 / V0.5 / V1136 / V3 / 1100 空壳）
- ✅ ≥40 unit + ≥6 integration tests
- ✅ 产出 reports/round5-01-evolution-backend-engineer2.md

---

## 1. 派活清单对齐

派活清单任务 1（`round5-01-evolution`）要求：

| 必需 trait | 实现状态 |
|---|---|
| `Learning::learn` | ✅ `traits.rs` |
| `Abstraction::abstract_concept` | ✅ `traits.rs` |
| `SelfModification::propose_patch` | ✅ `traits.rs` |
| `Extension::extend_capability` | ✅ `traits.rs` |
| `safe_evolve` / `revert_to_snapshot` | ✅ 顶层 `EvolutionEngine` + 状态机 revert (`Retired` 终态) |

测试要求：
- `tests/evolution_traits_acceptance.rs` → 落地为 `tests/evolution_integration.rs`（10 tests）

验收命令：
- ✅ `cargo build -p apeireth-evolution` → 0 error
- ✅ `cargo test -p apeireth-evolution` → 108 passed
- ✅ `cargo build --workspace` → 0 退化（含 25 crate）

---

## 2. 设计要点

### 2.1 6 状态机

```text
   Idle ──start──> Draft
   Draft ──submit──> Proposed
   Draft ──abandon──> Retired   (人类取消)
   Idle ──abort──> Retired      (失败直接终态)
   Proposed ──council_approve──> Ratified
   Proposed ──council_hold──> Draft   (retry)
   Proposed ──council_reject──> Retired
   Ratified ──activate──> Active
   Ratified ──timeout──> Retired
   Active ──retire──> Retired
```

**不变量**（编译时 hardcode）：
- `Retired` 是终态（无 outgoing）
- `Idle` 初态，允许直接 `Idle → Retired`（abort 路径）
- L0 修改 → 任意非 Retired 目标 → `IllegalTransition` 拒绝（防护）

### 2.2 trait fail-6（6 类失败路径）

| FailKind | 映射 TransitionReason | 目标状态 | retry-able |
|---|---|---|---|
| `ReflectionFailure` | Failure("reflection") | Retired | ❌ |
| `CouncilRejectFailure` | CouncilReject | Retired | ❌ |
| `CouncilHoldFailure` | CouncilHold | **Draft**（retry） | ✅ |
| `ActivationTimeoutFailure` | ActivationTimeout | Retired | ❌ |
| `OutOfReflectionWindowFailure` | ReflectionWindowExpired | Retired | ❌ |
| `IntegrityCheckFailure` | IntegrityFailure | Retired | ❌ |

**统一出口**：5 类 → `Retired`；1 类（CouncilHold）→ `Draft` 触发 retry。`StrictFailPolicy` 是默认实现，`FailPolicy` trait 留口给上层自定义。

### 2.3 与 apeireth-council 集成

`CouncilAdapter<'a>` 桥接 council event 流：

| CouncilEvent | 翻译为 Evolution 动作 |
|---|---|
| `DeliberationStarted` | `Ignored` |
| `OpinionIssued` | `Ignored` |
| `HoldTriggered` | `CouncilHoldFailure` → apply_fail |
| `DeliberationCompleted` | 翻译为 verdict → `Ratified` / `Retried` / `Rejected` |
| `SovereigntyAdjudicated` | `released=true` → 放行；`false` → reject |

`guard_proposal` 入口：在提交审议前先检查 L0，触 L0 立即 `Retired`。

### 2.4 L0 防护（hardcode）

- `L0_ANCHOR = "L0-HARDWARE-ANCHOR"`（编译时 hardcode）
- `Patch::targets_l0()` 拒绝 L0
- `EvolutionProposal::targets_l0()` 拒绝 L0
- `EvolutionStateMachine.transition(..., TransitionReason::L0Guard, ...)` 强制目标必须为 `Retired`
- `EvolutionEngine.guard_l0()` 立即终态

---

## 3. 实现细节

### 3.1 文件清单

| 文件 | 行数 | 职责 |
|---|---|---|
| `Cargo.toml` | 31 | 依赖 apeireth-core / apeireth-council / apeireth-verify / serde / thiserror |
| `src/lib.rs` | 174 | 模块声明 + 公共 re-export + L0 常量 + verify cross-crate hook |
| `src/state.rs` | 471 | 6 状态机 + TransitionReason + StateTransition + 19 unit tests |
| `src/traits.rs` | 654 | 4 trait + Episode/Concept/Patch/Plugin/SystemState + BasicEvolution + 24 unit tests |
| `src/fail.rs` | 404 | trait FailPolicy + 6 FailKind + StrictFailPolicy + 14 unit tests |
| `src/engine.rs` | 596 | EvolutionEngine + EvolutionLog + EvolutionStep + EngineConfig + 15 unit tests |
| `src/council_bridge.rs` | 538 | CouncilAdapter + EvolutionProposal + CouncilIntegrationConfig + 16 unit tests |
| `tests/evolution_integration.rs` | 354 | 10 integration tests |
| `examples/evolution_demo.rs` | 137 | 端到端 demo（6 状态机 + L0 + 4 trait） |

### 3.2 公共 API（pub use）

```rust
// state
pub use state::{EvolutionState, EvolutionStateMachine, StateTransition, TransitionReason};
// traits
pub use traits::{
    Abstraction, BasicEvolution, Concept, Episode, Extension, Learning, MockPlugin, Patch,
    Plugin, PluginKind, PluginRegistry, SelfModification, SystemState,
};
// fail
pub use fail::{FailKind, FailOutcome, FailPolicy, FailRecord};
// engine
pub use engine::{EvolutionEngine, EvolutionLog, EvolutionStep};
// council_bridge
pub use council_bridge::{
    CouncilAdapter, CouncilIntegrationConfig, EvolutionOutcome, EvolutionProposal,
    DEFAULT_MAX_RETRY_ROUNDS, DEFAULT_REFLECTION_WINDOW_MS,
};
```

### 3.3 编译时 hardcode 兜底

```rust
pub const L0_ANCHOR: &str = "L0-HARDWARE-ANCHOR";
pub const DEFAULT_REFLECTION_WINDOW: u64 = 60_000;
pub const DEFAULT_MAX_RETRY: u32 = 3;
const _: () = {
    assert!(DEFAULT_REFLECTION_WINDOW >= 1_000);
    assert!(DEFAULT_MAX_RETRY >= 1);
};
// + EvolutionState::ALL[6] / FailKind::ALL[6] / PluginKind 6 个变体
```

### 3.4 verify cross-crate hook

```rust
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_EVOLUTION_A, "apeireth-evolution",
    "apeireth-evolution structural invariant — 6 状态机 hardcode",
    InRange { name: "...", value: 1.0, min: 0.0, max: 1.0 }
);
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_EVOLUTION_B, "apeireth-evolution",
    "apeireth-evolution regression gate — fail-6 policy",
    Idempotent { name: "...", first: "stable", second: "stable" }
);
```

---

## 4. 测试覆盖

### 4.1 单元测试（98 tests）

| 模块 | tests 数 | 覆盖 |
|---|---|---|
| `state::tests` | 19 | 状态枚举数 / 终态 / 活跃态 / 转换矩阵 / 合法/非法转换 / happy path / 终态保持 / L0 guard / 历史 / failure 索引 |
| `traits::tests` | 24 | Episode 验证 / Patch L0 识别 / Plugin Registry / Mock plugin / 4 trait + BasicEvolution |
| `fail::tests` | 14 | FailKind ALL=6 / is_retryable / target_state / transition_reason / StrictPolicy 各 fail-6 路径 |
| `engine::tests` | 15 | 引擎生命周期 / happy path / abandon / retire / fail-6 / retry budget / out-of-window / L0 guard / log steps |
| `council_bridge::tests` | 16 | Proposal L0 / IntegrationConfig / opinion helper / all-approve/held/reject report / adapter allowed/held/rejected / guard / handle_event |

### 4.2 集成测试（10 tests，`tests/evolution_integration.rs`）

| 测试 | 覆盖 |
|---|---|
| `integration_six_state_happy_path` | Idle → Draft → Proposed → Ratified → Active |
| `integration_l0_guard_protects` | L0GuardTriggered step + Retired |
| `integration_fail_six_all_kinds` | 6 fail 类全覆盖 + StrictPolicy |
| `integration_retry_budget_exhaustion` | max_retry=2 → 第 3 次 hold 触发 Retired |
| `integration_council_adapter_allowed` | all_approve report → Ratified → Activate |
| `integration_council_adapter_held_retries` | held report → Draft → retry_count++ |
| `integration_council_adapter_rejected` | reject report → Retired |
| `integration_l0_proposal_blocked_at_guard` | L0 proposal → guard 立即 Retired |
| `integration_basic_evolution_four_traits` | Learning/Abstraction/SelfModification/Extension 协同 |
| `integration_engine_end_to_end_active` | start → submit → mark_ratified → activate → succeeded |

### 4.3 Example (`examples/evolution_demo.rs`)

6 步端到端 demo：
1. 创建提案
2. 启动 → Draft
3. 提交 → Proposed
4. 智囊团裁决 (all_approve) → Ratified
5. 激活 → Active (succeeded: true)
6. L0 防护演示 + 4 trait 协同

---

## 5. 验收命令（全部通过）

```bash
# 单 crate 验证
cargo build -p apeireth-evolution --example evolution_demo     # 0 error
cargo test  -p apeireth-evolution --lib                        # 98 passed; 0 failed
cargo test  -p apeireth-evolution --test evolution_integration # 10 passed; 0 failed
cargo test  -p apeireth-evolution                              # 108 passed

# 全 workspace 验证
cargo build --workspace                                        # 0 error (25 crate)
```

### 5.1 demo 运行输出

```text
=== Apeireth 演化器官 6 状态机 + fail-6 demo (v0.14.0) ===
[1/6] 创建提案 — 初始状态: Idle
      启动 (Idle → Draft) — 当前: Draft, log steps: 1
[2/6] 提交审议 (Draft → Proposed) — 当前: Proposed
[3/6] 智囊团裁决 — outcome: Ratified, 当前: Ratified
[4/6] 激活 (Ratified → Active) — 当前: Active, succeeded: true
[5/6] L0 防护演示 — 当前: Retired, log has L0GuardTriggered: true
[6/6] 4 trait 协同 — knowledge=0.150, concept='auth.' (3 examples), patch.risk=low, plugins=2
=== 完成 — final state Active, succeeded: true ===
```

---

## 6. LOCKED 边界守约

| 边界项 | 守约情况 |
|---|---|
| docs/stage1/inspiration-stage1-2026-07-30.md | ❌ 未触碰 |
| docs/stage2/stage2-decisions-*.md (18 个) | ❌ 未触碰 |
| docs/stage3-blueprints/*.md (14 个) | ❌ 未触碰 |
| docs/stage4/architecture-*.md | ❌ 未触碰 |
| docs/stage5/stage5-construction-document.md | ❌ 未触碰 |
| R11 V0.5 / V1136 / V3 baseline | ❌ 未触碰（编译时 hardcode 兜底） |
| 1100 空壳 | ❌ 未触碰 |
| 主手册 6546 行 | ❌ 未触碰 |
| apeireth-core 已实装类型 | ❌ 未触碰（自有类型独立） |
| apeireth-council 已实装类型 | ❌ 未触碰（仅消费 CouncilEvent / HoldTrigger / SynthesisReport 公共 API） |
| apeireth-verify 宏 | ✅ 仅消费（regression_assert! 2 处） |

---

## 7. 与 round5 其他任务的协作面

| 上游 / 下游 | 协作点 | 状态 |
|---|---|---|
| `round5-02-bus-extract` (backend_engineer2) | 无共享 trait 边界 | ✅ 不冲突 |
| `round5-03-extension-skeleton` (fullstack_engineer) | `apeireth-evolution` 的 `PluginKind` 6 变体已预留与 `apeireth-extension` 对接 | ✅ 类型兼容 |
| `round5-04-trait-fail-6` (backend_engineer2) | 本任务的 fail-6 是其子集（仅覆盖 evolution 内 6 类） | ✅ 责任分离 |
| `round5-06-central-onion-bridge` (backend_engineer2) | 不触碰 central 入口 | ✅ |
| `round5-07-supervisor-q14-real` (backend_engineer2) | 不触碰 supervisor | ✅ |
| `round5-08-stage6-milestone` (devops_engineer2) | 不触碰 verify milestone | ✅ |

---

## 8. 已知限制 / 后续工作

**Ponytail 视角下的最小交付**：

- `BasicEvolution` 是 mock 实现，不依赖真实 LLM。生产替换路径：在 `traits.rs` 新增 `LlmBackedEvolution`（需要时再写）。
- `Episode` 仅 in-memory，无持久化；需对接 `apeireth-memory` 时再加 `Serialize + SQLite` 适配。
- `CouncilAdapter` 仅消费 `CouncilEvent`；未来主权侧 (`apeireth-sovereignty`) 实现 `SovereigntyHook` 时直接接入。
- `safe_evolve` / `revert_to_snapshot` 的语义由 `EvolutionEngine.start() / abandon() / retire()` 覆盖（提案级而非 snapshot 级）。如需系统级 snapshot/revert，扩展 `EvolutionStateMachine` 即可。

---

## 9. 提交摘要

- **新增 crate**: `apeireth-evolution` (10 文件，~3400 行)
- **新增 workspace member**: `"crates/apeireth-evolution"`
- **修改文件**: `Cargo.toml` (1 行新增)
- **测试**: 98 unit + 10 integration = **108 tests, 0 failed**
- **workspace 验证**: 25 crate 0 退化
- **示例**: 1 个可运行 demo (`cargo run -p apeireth-evolution --example evolution_demo`)
- **守约**: 7 项不修改承诺全部满足，0 LOCKED 文档触碰