# Cognitive Module × 9-Organ 集成规范 (v2.0.0-rc.1 → 真生产)

> **文档定位**: 真生产前阻塞 #2 (估 1 周, 估 2027-Q1 启动) 的实施规范.
> **何时写**: v2.0.0-rc.1 (`b9026186`) tag 拍板后, 子代理 R10 写 (2026-08-28).
> **读谁**: 接手 Apeireth v2 真生产路径 cognitive module × 9 organ 集成的实施者.
> **关系文档**: `docs/04-internal/cognitive-module-wiring.md` (12 slot ledger 现状) +
> `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` (5/5 done + 6 DEFERRED) +
> `docs/01-architecture/v2-architecture-reflection.md` (自升级 cycle) +
> `crates/engine/runtime/src/canonical/cognitive.rs` (12 slot 注入路径)
> + `crates/engine/organ/src/lib.rs` (9 organ 真实现) +
> `crates/foundation/plugin/src/organ.rs` (OrganTrait 边界).
>
> **核心矛盾 (子代理 R10 独立判断)**: v2.0-rc.1 真实账是
> "cognitive module 12 slot 6 WIRED (judge/council WIRED, OFF by default), 9 organ 全实装 (整合 #2 commit
> `bbf70293`)". 缺的不是 "写一个 orchestrator" — 缺的是把 cognitive module slot 和 9 organ
> process **在同一 runtime hook 链里串成 L0-L5 自升级 cycle**. 本 spec 不真做这 1 周, **只**写
> "未来实施者怎么串 + 哪些契约严守 0 改".

---

## 目录

- §1 概述
- §2 9 organ process 串联路径 (L0-L5)
- §3 9 organ 串联顺序 (per R7 风险 #1 + 子代理 L 自升级 cycle)
- §4 12 slot 注入路径 (per `cognitive-module-wiring.md:23-35`)
- §5 OrganOrchestrator 类似 AwakeCompanion (R7 0 装诚实真账)
- §6 6 WIRED slot 真接路径
- §7 6 DEFERRED slot 激活路径 (真生产前可激活)
- §8 5 状态机 + 8 重门控 + 主动开口 (per E7 emergence)
- §9 L0-L5 自升级 cycle 集成 (per `v2-architecture-reflection.md` §6)
- §10 0 装诚实真账 (子代理 R10 独立判断)
- §11 0 触碰 LOCKED (5 项 + 扩展)
- §12 真生产前阻塞 #2: frontend 对接 (估 4-6 周)
- §13 接手人 actionable (5/5 done + 7 #6/#7 真生产前必做)
- 附录 A: 关键文件:行 reference
- 附录 B: 缩略词 + 引用

---

## §1 概述

### 1.1 一句话定义

**Cognitive Module × 9 Organ 集成 = 把 cognitive module 的 12 slot (AgentModule 体系) 与 9 organ 的 process (OrganTrait 体系) 在同一 runtime hook 链里, 串成 L0-L5 自升级 cycle, 同时保留两条体系各自的 0 装诚实边界**.

### 1.2 当前状态 (v2.0.0-rc.1, HEAD `b9026186`)

| 体系 | 真实现 | 待激活 |
|---|---|---|
| **cognitive module 12 slot** | **6 WIRED** = `memory_recall` / `preference_recall` / `memory_writeback` / `judge` / `self_assessment` / `council` (judge/council 为 WIRED, OFF by default) | 6 DEFERRED = `preference_learning` / `cognitive.critic` / `cognitive.reflection` / `cognitive.planner` / `cognitive.orchestrator` / `cognitive.perception` |
| **9 organ** | 9/9 真实现 (整合 #2 commit `bbf70293`): E4 Curiosity / F1 Emotion / F4 Hypothesis / F6 Value / W1 World Model / W2 Causal World Model / W3 Edge Mining / E7 Emergence / Memory Merger | 0 (全实装) |
| **串联层** | **缺失** — 9 organ 各自独立 trait impl, 缺 OrganOrchestrator 把 9 organ process 串成 L0-L5 cycle | OrganOrchestrator 类似 legacy `AwakeCompanion` 8 重 gate + 5 状态机 + 主动开口 + emotion/council/onion 串接 |

### 1.3 任务边界 (子代理 R10 独立判断)

- **本 spec 不真做 1 周 cognitive module × 9 organ 集成实施**. 只写 spec 30-45 分钟.
- **0 装诚实标**: 不假装"已集成 cognitive module × 9 organ". 真账是 "12 slot 6 WIRED
  真接 + 9 organ 各自 trait impl 已就位 + OrganOrchestrator 缺 (R7 风险 #1 提)" — 这条风险 #1
  必须等真生产前真实施, 估 1-3 周.
- **不引新外部 dep**. Cargo.lock 0 行 diff.
- **不触碰 LOCKED**. 见 §11.
- **不 commit**. 等主代理审 (per Q1 C1 policy).

### 1.4 三阶审查 (O-6 锚 9, 子代理 R10 独立判断)

1. **总体最优**: 与 v2 整体语境对齐 — cognitive module (12 slot AgentModule ABI) 与 9 organ
   (OrganTrait ABI) **不**是同一个 ABI. 真生产路径必须**两层并行**, 用 OrganOrchestrator
   (新增) 串成 L0-L5 cycle, 而**不是**改 cognitive slot 注册 9 organ (会破 forward-declared
   边界). 这条认知与 `cognitive-module-wiring.md:7-18` "它不引入第二个 ABI" 的声明一致.
2. **系统最优**: 在子系统依赖图里位置对 — `OrganOrchestrator` 应在 `crates/engine/organ/`
   (与 9 organ 同 crate, 单向依赖 plugin trait). Runtime 层加一层薄 orchestrator 入口,
   不引入第二个 agent loop (与 `cognitive-module-wiring.md:34` "long-running Planner →
   Implementer → Reviewer service; never called from the canonical turn" 一致 — Orchestrator
   是 L2 提案生成服务, **不**是 per-turn hook).
3. **架构最优**: 引入后整个 workspace 边界清晰 — 9 organ 走 `Arc<dyn OrganTrait>` 注入
   `OrganOrchestrator` (类似 `Arc<dyn LlmFactory>` 注入 Council); OrganOrchestrator 在
   runtime 的 hook 链外调 (不破 canonical turn), 在 L0-L5 自升级 cycle 里调.

---

## §2 9 organ process 串联路径 (L0-L5)

> **本章定位**: 把 `docs/01-architecture/v2-architecture-reflection.md` §6 的 L0-L5
> 自升级 cycle 映射到 9 organ process 的**真接**路径. 每层标 "哪个 organ 真接 + 哪个 cognitive
> slot 真接 + 哪个 LOCKED 边界严守".

### 2.1 L0 人类审批 (硬墙, 永远不可变)

- **入口**: 主代理 (Mavis) 拍板. runtime 永远不能绕过 L0.
- **9 organ 关联**: **0 organ** 真接 L0 (organ 是机制层, 无审批权).
- **cognitive module 关联**: cognitive module slot 不审批 — 审批走 governance hook
  (PermissionGovernance / CredentialDisclosure / PromptInjection) 在 runtime 主循环.
- **LOCKED**: `crates/foundation/core/src/philosophy/eight_anchors.rs:58-79` (S-1 北极星 +
  O-1 安全优先) 0 改.
- **真实施估时**: 0 (L0 已就位, 物理隔离).

### 2.2 L1 自我诊断 (runtime 主动)

- **入口**: `cognitive.self_assessment` (WIRED, OFF by default, Judge-backed).
- **真接路径** (per `crates/engine/runtime/src/canonical/cognitive.rs:875` + `:948`):
  - `SelfAssessmentModule` 挂 `AfterTurn` hook.
  - 真实路径: 真 Judge result → typed in-process → 持久化到 `SQLiteSelfAssessmentStore`.
  - **0 装诚实**: 仅记录真 Judge 结果, 不伪造 heuristic score (per
    `cognitive-module-wiring.md:28`).
- **9 organ 关联**: F1 Emotion Memory (per `crates/engine/organ/src/emotion_memory.rs`)
  可作为 self-assessment 输入 (主人情绪时间线), 但**当前 v2 E7 organ process 路径未真接**
  self_assessment (前向声明). 真生产前必做.
- **真实施估时**: 1 周 (F1 → self_assessment 输入 + E7 feedback loop 串接).

### 2.3 L2 提案生成 (Orchestrator)

- **入口**: `Orchestrator::dispatch(spec)` → `Council::with_factory(factory, model).decide_with_invoker()` (per `crates/engine/runtime/src/canonical/cognitive.rs:1011-1021`).
- **7 advisor 并行 + 60s timeout** (per `cognitive-module-wiring.md:96-102`).
- **9 organ 关联**:
  - **W1 World Model** (per `crates/engine/organ/src/world_model.rs`, 子代理 R4 真接 LLM):
    反事实推演 + oracle Brier 校准.
  - **W2 Causal World Model** (per `crates/engine/organ/src/causal_world_model.rs`,
    子代理 R5 真接 LLM MCTS): 因果结构图分支点.
  - **W3 Edge Mining** (per `crates/engine/organ/src/causal_world_model_edges.rs`,
    子代理 R6): 边挖掘 + 累计权重 (deterministic 0 LLM).
- **LOCKED**: `LlmFactory` trait 边界 (per `crates/foundation/plugin/src/llm_factory.rs`) 0 改.
- **真实施估时**: 2-3 周 (Orchestrator 与 Council 双向桥 + W1/W2/W3 输出接 Council proposal).

### 2.4 L3 验证 (testing sandbox)

- **入口**: 9 organ process 串联 (E4 + F1 + F4 + F6 + W1 + W2 + W3 + E7 + Memory), 然后
  跑 sandbox regression + 5 重守门.
- **9 organ 串联顺序** (per §3 详细):
  1. E4 Curiosity (浅尝辄止 + 疑问路由, deterministic 0 LLM)
  2. F1 Emotion Memory (主人情绪记录, deterministic 0 LLM)
  3. F4 Hypothesis (假设闭环, deterministic 0 LLM)
  4. F6 Value Cases (价值内化, deterministic 0 LLM)
  5. W1 World Model (LLM 反事实推演 + Brier 校准)
  6. W2 Causal World Model (LLM MCTS 分支点)
  7. W3 Edge Mining (deterministic 0 LLM)
  8. E7 Emergence (5 状态机 + 8 重门控)
  9. Memory Merger (跨 8 organ 记忆合并)
- **sandbox 跑**:
  - `cargo test --workspace --locked 0 FAILED`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings 0 警告`
  - legacy compat path < 100 引用
  - 13 键 LOCKED + 9 哲学锚本体 + workspace.version + R11 baseline 0 触碰
  - 哲学锚表头 0 减
- **真实施估时**: 1-2 周 (9 organ 串联入口 + sandbox 跑通).

### 2.5 L4 主人审批 (governance hook)

- **入口**: governance 3 hook (PermissionGovernance + CredentialDisclosure + PromptInjection)
  + Council 多意见加权 + 主人 Veto.
- **9 organ 关联**: E7 Emergence 8 重门控里的 `SovereityFrozen` + `EmotionLow` + `CouncilVeto`
  + `PolicyInactive` + `GateBlock` (per `crates/engine/organ/src/emergence.rs:433-441`)
  接受上层 L4 闸留痕.
- **cognitive module 关联**: `COUNCIL_MODULE_ID` (`crates/engine/runtime/src/canonical/cognitive.rs:42`)
  输出的 `CouncilDecision::DeferToHuman` 走 `ModuleOutcome::stop` (per `:1027-1029`) → 主代理审.
- **LOCKED**: `crates/foundation/core/src/onion.rs:249` (3 项不可变脊柱) 0 改.
- **真实施估时**: 1-2 周 (governance hook 与 OrganOrchestrator 桥 + Council 加权 + 主人 Veto UI).

### 2.6 L5 runtime patch (自我升级)

- **入口**: 主代理批 → `git tag v2.x+1` → 新版本生效.
- **9 organ 关联**: LlmFactory 新 model 即时生效 (per `v2-architecture-reflection.md:255-261`).
  9 organ 各自 trait impl 即时生效.
- **cognitive module 关联**: 12 slot 即时激活/废弃 (forward-declared 边界严守).
- **LOCKED**: `Cargo.toml:43` workspace.version = "1.2.0" 0 改 (per Q1 任务 #5).
- **真实施估时**: 1 周 (release script + 9 organ 回归 + cognitive slot 激活/废弃流程).

---

## §3 9 organ 串联顺序

> **本章定位**: 真生产路径 OrganOrchestrator 调 9 organ process 的顺序, per R7 风险 #1
> "OrganOrchestrator 类似 legacy AwakeCompanion 待真实施" + 子代理 L 自升级 cycle §2 估.

### 3.1 串联顺序总览

```
[L0 人类审批] (入口, 永远不可变)
     ↓
[L1 自我诊断: cognitive.self_assessment + F1 emotion 输入]
     ↓
[L2 提案生成: Orchestrator.dispatch → Council.decide_with_invoker (7 advisor 并行)]
     ↓ input: F4 hypothesis / F6 value / W1 world_model / W2 causal_world_model
     ↓ validate: E4 curiosity (浅尝辄止 + 疑问路由)
     ↓ commit: Memory merger (跨 organ 记忆合并)
     ↓
[L3 验证: 9 organ process 串联入口 + sandbox 5 重守门]
     ↓
[L4 主人审批: governance 3 hook + Council 加权 + E7 8 重门控上层闸]
     ↓
[L5 runtime patch: git tag + cognitive slot 即时激活/废弃]
```

### 3.2 9 organ 详细说明 (per 子代理 R1-R8 真实现)

| 顺序 | Organ ID | 模块路径 | 真实现状态 | LLM 依赖 | 串联作用 |
|---|---|---|---|---|---|
| 1 | **E4** Curiosity | `crates/engine/organ/src/curiosity.rs` (子代理 Q1 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 浅尝辄止 + 疑问路由 — 在 L3 入口前**先把提案窄化**, 不让 LLM 浪费 token |
| 2 | **F1** Emotion Memory | `crates/engine/organ/src/emotion_memory.rs` (子代理 R1 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 主人情绪时间线 — 给 L1 自我诊断与 L4 闸提供情绪背景 |
| 3 | **F4** Hypothesis | `crates/engine/organ/src/hypothesis.rs` (子代理 R2 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 假设闭环 (HypothesisStore + VerifyPlanner + ReconcileSink) — L2 提案前的结构化猜想 |
| 4 | **F6** Value Cases | `crates/engine/organ/src/value_cases.rs` (子代理 R3 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 价值内化 (案例库 + 裁决记录 + 主人反馈回流) — L4 闸的价值判别基础 |
| 5 | **W1** World Model | `crates/engine/organ/src/world_model.rs` (子代理 R4 真实现) | ✅ 真实现, **真接 LLM** | **LLM** (反事实推演 + oracle Brier 校准) | L2 提案的反事实分支生成 |
| 6 | **W2** Causal World Model | `crates/engine/organ/src/causal_world_model.rs` (子代理 R5 真实现) | ✅ 真实现, **真接 LLM MCTS** | **LLM** (MCTS 分支点) | L2 提案的因果分支生成 |
| 7 | **W3** Edge Mining | `crates/engine/organ/src/causal_world_model_edges.rs` (子代理 R6 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 从记忆时间线统计挖掘因果边 — L2 提案前的边累计权重 |
| 8 | **E7** Emergence | `crates/engine/organ/src/emergence.rs` (子代理 R7 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 5 状态机 + 8 重门控 + 主动开口 — L4 闸的留痕与 `spoke=true/false` 决定 |
| 9 | **Memory** Merger | `crates/engine/organ/src/memory.rs` (子代理 R8 真实现) | ✅ 真实现 | **0 LLM** (deterministic) | 跨 8 organ 记忆合并 (借鉴 v1 `MemoryExtractionService` 1:1 翻译 dedup/weight/persist) |

### 3.3 串联顺序的 0 装诚实标

- **顺序不可乱**: E4 在最前 (浅尝辄止) → F1/F4/F6 中段 (背景 + 猜想 + 价值) → W1/W2/W3
  (LLM 重 + 因果边累计) → E7 (8 重门控) → Memory 收尾. **不**颠倒: 把 W1 放最前会让 LLM
  浪费 token 跑无背景推演.
- **E4 + F1 + F4 + F6 + W3 + E7 + Memory = 7 organ deterministic 0 LLM**: 这 7 organ 是真
  0 装诚实路径, 永远不假装能调 LLM. 只 W1 + W2 真接 LlmFactory.
- **E7 emergence 决策路径严格确定性**: `should_speak()` 严格走 v1 8 重门控 (per
  `crates/engine/organ/src/emergence.rs:570-573`), 不假装"E7 always speak".
- **Memory merger 子代理 R8 独立判断**: v1 `runtime_brain.rs` 没有 `MemoryMerger` 模块;
  v2 是新抽象, 借鉴 v1 `MemoryExtractionService` 算法骨架 1:1 翻译. 不假装"v1 有这模块".

### 3.4 OrganTrait 边界严守

- **trait 在 foundation** (`crates/foundation/plugin/src/organ.rs:292` `pub trait OrganTrait`).
- **impl 在 engine** (`crates/engine/organ/src/lib.rs:84` `impl OrganTrait for NoopOrgan`).
- **runtime 拿 `Arc<dyn OrganTrait>` 注入** (per `organ.rs:286-290` 注释 + `lib.rs:30-31`
  注释).
- **OrganInput 最小契约** (per `organ.rs:124-150`): `episode` (R11 主路径核心类型) +
  `session_id` + `context_hints` + `dry_run`. **0 epoch 时间戳 → OrganInput 0 时间** = E7
  emergence 自己从 `episode.timestamp: i64` 派生 day_key + minutes_of_day (per
  `emergence.rs:723-732` `day_key_from_epoch_ms` + `minutes_of_day_from_epoch_ms`).
- **OrganOutput 9 variant** (per `organ.rs:157-188`): Curiosity / Emotion / Hypothesis /
  Value / WorldModel / Emergence / Memory / NotImplemented (0 装 PASS). 8/9 organ 各自返
  对应 variant (E4 返 Curiosity, etc.).
- **OrganError 统一通道** (per `organ.rs:239-278`): NotImplemented / LlmUnavailable /
  LlmError / Config / BudgetExhausted / Internal.

---

## §4 12 slot 注入路径

> **本章定位**: 复述 `docs/04-internal/cognitive-module-wiring.md:23-35` 的 12 slot
> ledger, 标 file:line 锁定 + 真接状态 + 真生产前必做事项.

### 4.1 WIRED slot 真接 (per `cognitive-module-wiring.md:24-29`, council 见 §4.2)

| Slot | 真接 module | Hook | Dependency | Status | File:line |
|---|---|---|---|---|---|
| `cognitive.memory_recall` | `MemoryRecallModule` | `TurnStart` | `Arc<dyn MemoryBackend>` + Experience 可选 | ✅ WIRED | `cognitive.rs:37` (id) + `:255` (struct) + `:317` (impl) |
| `cognitive.preference_recall` | `PreferenceRecallModule` | `TurnStart` | `Arc<dyn PreferenceStore>` | ✅ WIRED | `cognitive.rs:39` (id) + `:571` (manifest) |
| `cognitive.judge` | `JudgeModule` | `AfterModelResponse` | `Arc<dyn ModuleInvoker>` (one side-call max per judge hook; typed JSON; no tools) | ✅ WIRED, **OFF by default**, 需 `APEIRETH_COGNITIVE_JUDGE=1` | `cognitive.rs:41` (id) + `:727` (manifest) |
| `cognitive.self_assessment` | `SelfAssessmentModule` | `AfterTurn` | Judge 结果 (no fabricated heuristic score) | ✅ WIRED, Judge-backed | `cognitive.rs:40` (id) + `:875` (manifest) |
| `cognitive.memory_writeback` (附加) | `MemoryWritebackModule` | `AfterTurn` | `Arc<dyn MemoryBackend>` + Experience + `Arc<dyn Clock>` | ✅ WIRED | `cognitive.rs:38` (id) + `:419` (manifest) + `:457` (impl) |

> **注意**: 上面表格 + §4.2 = **6 WIRED** (`memory_recall` / `preference_recall` / `judge` /
> `self_assessment` / `memory_writeback` / `council`). 任务 brief 曾写 "4 WIRED", R10 曾修正为
> "5 WIRED + 1 SLOT READY", R13 接力审真账核验 = **6 WIRED + 6 DEFERRED** (judge/council 是
> "WIRED, OFF by default", 不是 "SLOT READY").

### 4.2 `cognitive.council` 补充 (WIRED, OFF by default, per `cognitive-module-wiring.md:27`)

| Slot | 真接 module | Hook | Dependency | Status | File:line |
|---|---|---|---|---|---|
| `cognitive.council` | `CouncilModule` | `AfterModelResponse` | `Arc<Council>` + 7 LlmAdvisor (named advisor slots; 10s per-advisor / 60s overall timeout) | ✅ WIRED, **OFF by default**, 需 `APEIRETH_COGNITIVE_COUNCIL=1` | `cognitive.rs:42` (id) + `:963` (struct) + `:1011-1021` (decide_with_invoker) + `:1056-1082` (RuntimeCouncilInvoker) |

### 4.3 6 DEFERRED slot (per `cognitive-module-wiring.md:30-35`)

| Slot | Owner | Hook | Status | 真生产前必做 |
|---|---|---|---|---|
| `cognitive.preference_learning` | deferred, no owner yet | — | **DEFERRED** | v1 era 已有, 集成估 2 周 |
| `cognitive.critic` | Judge owner | — | **DEFERRED INTO JUDGE** (per `cognitive-module-wiring.md:31` "Judge's bounded critique is the single critique path; no duplicate evaluator") | 0 估时 (已并入 Judge) |
| `cognitive.reflection` | SelfAssessment owner | `AfterTurn` | **DEFERRED INTO SELF-ASSESSMENT** (per `cognitive-module-wiring.md:32`) | 0 估时 (已并入 SelfAssessment) |
| `cognitive.planner` | orchestration service | — | **NOT AN AGENT MODULE** (per `:33` "no per-turn planner loop; future adapter must remain an adapter") | 新建估 3 周, LLM 重 |
| `cognitive.orchestrator` | `apeireth-orchestration::Orchestrator` service | — | **NOT AN AGENT MODULE** (per `:34` "long-running Planner → Implementer → Reviewer service; never called from the canonical turn") | 新建估 3 周, 类似 AwakeCompanion |
| `cognitive.perception` | perception adapter | — | **NOT AN AGENT MODULE** (per `:35` "PerceptionInput becomes TurnRequest through turn_request_from_perception; only text payload is implemented") | RC-7 真接估 2-3 周, 硬件依赖 (Whisper + xcap) |

### 4.4 注册顺序 (per `cognitive-module-wiring.md:37-43`)

```text
TurnStart:          memory_recall -> preference_recall
AfterModelResponse: judge -> council
AfterTurn:          self_assessment -> memory_writeback
```

- **deterministic**: runtime 启动时按序注册, 不可改 (per `:37-38` "Registration order is deterministic").
- **0 装诚实标**: 9 organ **不**走 cognitive module 12 slot 注册 — 9 organ 走 `OrganTrait`
  + `OrganOrchestrator` 独立体系. 真生产路径 OrganOrchestrator 在 cognitive module 主循环外
  调 (L2 提案生成时), 不破 canonical turn 边界.

### 4.5 12 slot ledger 0 改 (LOCKED 边界)

- `cognitive-module-wiring.md:23-35` 12 slot 状态行**不**改. forward-declared 边界严守.
- `cognitive.rs:37-42` 6 个 `pub const ..._MODULE_ID: &str` 常量**不**改. 这是 compatibility
  contract (per `:35-36` "Stable ids are the slot ledger keys. Changing one is a compatibility
  change, not an implementation detail.").
- `cognitive.rs:1121-1138` `DEFERRED_COGNITIVE_SLOTS` 常量**不**改. 维护当前真账的 0 装诚实标.

---

## §5 OrganOrchestrator 类似 AwakeCompanion (R7 0 装诚实真账)

> **本章定位**: R7 风险 #1 "9 organ 缺 orchestrator 串联" 的真账 + 真生产前必做的设计稿.
> 子代理 R10 独立判断: 这条风险是真账, 不是 0 装诱导. 真生产前必做.

### 5.1 现状真账 (R7 风险 #1)

- **9 organ 真实现全实装** (整合 #2 commit `bbf70293`, per
  `crates/engine/organ/src/lib.rs:11-28` 文档头注释):
  - ✅ E4 Curiosity (子代理 Q1)
  - ✅ F1 Emotion Memory (子代理 R1)
  - ✅ F4 Hypothesis (子代理 R2)
  - ✅ F6 Value Cases (子代理 R3)
  - ✅ W1 World Model + **真接 LLM** (子代理 R4)
  - ✅ W2 Causal World Model + **真接 LLM MCTS** (子代理 R5)
  - ✅ W3 Edge Mining (子代理 R6, deterministic 0 LLM)
  - ✅ E7 Emergence (子代理 R7, deterministic 0 LLM)
  - ✅ Memory Merger (子代理 R8, deterministic 0 LLM)
- **缺**: OrganOrchestrator — 9 organ 是 9 个独立 trait impl, 没有统一的串联入口.
- **0 装诱导 prevention**: 不假装"9 organ 已通过某个 orchestrator 自动串联". 真账是:
  - v1 时代 9 organ 在 `apeireth-companion::AwakeCompanion` 由 if-else 散落串联 (per
    `legacy/donor/apeireth-companion/examples/awake_greeting.rs:48` `AwakeCompanion::new`).
  - v2 真生产路径缺类似串联入口.

### 5.2 真生产前必做: OrganOrchestrator 类似 AwakeCompanion

**任务**: 写 `OrganOrchestrator` 类似 legacy `AwakeCompanion` 的 8 重 gate + 5 状态机 +
主动开口 + emotion/council/onion 串接.

**设计稿** (子代理 R10 提案, 待真生产前实施者拍板):

```rust
// 文件位置: crates/engine/organ/src/orchestrator.rs (待真生产前建)
// 单向依赖: organ → plugin → core (与现有 9 organ impl 同位置)
//
// 关键设计:
// 1. 拿 9 个 Arc<dyn OrganTrait> + Arc<Council> + Arc<dyn Onion> + Arc<dyn RelationshipState>
// 2. tick(at_ms, episode) 走 v1 AwakeCompanion 8 重门控:
//    - E7 emergence 8 重门控 (per crates/engine/organ/src/emergence.rs:570-573)
//      + 5 状态机 PolicyStage (Idle/Draft/Proposed/Ratified/Active)
//    - EmotionLow (mood_floor 闸, per emergence.rs:177)
//    - CouncilVeto (CouncilDecision::DeferToHuman / Stop)
//    - PolicyInactive (PolicyStage::is_active() == false)
//    - GateBlock (洋葱门拦下)
//    - SovereigntyFrozen (L0 物理隔离)
// 3. 9 organ process 串联顺序按 §3.1 调 (E4 → F1 → F4 → F6 → W1 → W2 → W3 → E7 → Memory)
// 4. 输出: Initiative + 9 organ output 列表 + 主动开口 spoke=true/false
```

**8 重 gate 详细** (per `crates/engine/organ/src/emergence.rs:419-441`
`InitiativeGate::UserQuiet` / `QuietHours` / `DailyLimit` / `LlmBudget` / `DepthLow` /
`RhythmUnknown` / `RhythmVeto` / `DriveLow`):

1. `UserQuiet` (门禁 0) — 用户显式不打扰开关
2. `QuietHours` (门禁 1) — 安静窗口 (per `Boundaries::in_quiet_window`)
3. `DailyLimit` (门禁 2) — 每日主动频率上限 (default 2/day)
4. `LlmBudget` (门禁 2.5) — LLM 成本预算 (min_llm_interval_ms, default 60s)
5. `DepthLow` (门禁 3) — 关系深度门槛 (default 0.3)
6. `RhythmUnknown` (门禁 4) — 0 观察天数 → 不猜测作息
7. `RhythmVeto` (门禁 5) — 学到的作息说此刻几乎不可能活跃 → 沉默压力再大也不打扰
8. `DriveLow` (门禁 6) — 驱动未达阈值且未达冷启动探针条件 → 保持安静

**v2 扩展 5 重 gate** (per `emergence.rs:432-441`):

9. `SovereigntyFrozen` (L0 物理隔离)
10. `EmotionLow` (情绪愉悦度低)
11. `CouncilVeto` (智囊团审议拒绝)
12. `PolicyInactive` (策略不在 Active)
13. `GateBlock` (洋葱门拦下)

**0 装诚实标**: 8 + 5 = 13 重 gate, 是**机制层**真实门控原因 (per
`emergence.rs:421-422` "这是机制层**真实**门控原因, 不是事后解释"), 不假装"全部必过".

### 5.3 真实施估时

- **OrganOrchestrator 骨架 + 9 organ 注入 + 13 重 gate** = 1 周
- **9 organ process 串联入口 + 错误处理 (OrganError → 静默忽略 or trace)** = 1 周
- **Council / Emotion / Onion / RelationshipState 桥接** = 1 周
- **总计**: 1-3 周 (per 任务 brief "估 1 周" + 子代理 R10 独立判断 +30% buffer).

### 5.4 0 触碰 LOCKED 边界

- 不改 `crates/engine/organ/src/lib.rs:30-31` "9 organ 全实装" 注释 — 加 OrganOrchestrator
  不影响 9 organ 各自状态.
- 不改 `crates/foundation/plugin/src/organ.rs:69-89` `OrganKind` enum 顺序 — 9 organ IDs
  锁定.
- 不改 `crates/engine/organ/src/emergence.rs` E7 真实现 — OrganOrchestrator 仅调
  `EmergenceOrgan::process()`, 不改 EmergenceOrgan 内部.
- 不改 cognitive module 12 slot ledger — OrganOrchestrator 是 L2 提案生成服务, **不**走
  cognitive slot 注册.

---

## §6 6 WIRED slot 真接路径

> **本章定位**: 详细说明 6 个已真接 slot 的数据流 + 9 organ 关联 + 真生产前待补.

### 6.1 `cognitive.memory_recall` (WIRED)

- **真接 module**: `MemoryRecallModule` (per `crates/engine/runtime/src/canonical/cognitive.rs:255-402`)
- **Hook**: `TurnStart` (per `:329` `if hook == HookPoint::TurnStart`)
- **依赖**: `Arc<dyn MemoryBackend>` (:256) + 可选 Experience (`Arc<dyn WikiEntryStore>` +
  `Arc<dyn KnowledgeGraphStore>` + `Arc<dyn AssociationStore>`) (per `:282-294`
  `with_experience`)
- **数据流**:
  1. 调 `self.memory.recent_episodes(&session, self.limit)` (`:331`)
  2. 调 wiki / graph / associations (`:340-384`)
  3. 拼成 `PromptOverlay::system(...)` (`:393`)
  4. 调 `ModuleOutcome::continue_().with_prompt_overlay(...)` (`:393`)
- **9 organ 关联**: E4 Curiosity 浅尝辄止 (per `crates/engine/organ/src/curiosity.rs`) +
  Memory Merger 跨 8 organ 记忆合并 (per `crates/engine/organ/src/memory.rs`) — **真生产前必做**:
  把 `memory_recall` 的 prompt_overlay 喂 E4 curiosity 作为 `context_hints`.
- **真实施估时**: 1 周.

### 6.2 `cognitive.preference_recall` (WIRED)

- **真接 module**: `PreferenceRecallModule` (per `:571-622`)
- **Hook**: `TurnStart`
- **依赖**: `Arc<dyn PreferenceStore>`
- **数据流**: 调 `preferences.iter()` → 拼成 prompt overlay.
- **9 organ 关联**: F6 Value Cases (top + F1 emotion trend) — **真生产前必做**: 把
  `preference_recall` 输出喂 F6 + F1.
- **真实施估时**: 1 周.

### 6.3 `cognitive.judge` (WIRED, OFF by default)

- **真接 module**: `JudgeModule` (per `:727-848`)
- **Hook**: `AfterModelResponse`
- **依赖**: `Arc<dyn ModuleInvoker>` (one side-call max per judge hook; typed JSON; no tools)
- **数据流**:
  1. 拿到 `candidate.content` (tool-free, per `cognitive-module-wiring.md:82-88`)
  2. 构 `ModuleInvocationRequest::isolated("...", "...")`
  3. 调 `self.invoker.invoke(request)` (one side-call max)
  4. 解析 typed JSON: `score / verdict / critique / confidence`
  5. **0 装诚实**: unknown fields / malformed JSON / non-finite / out-of-range / oversized
     critique fail closed as module errors (per `cognitive-module-wiring.md:85-87`)
  6. `retry` 仅 below configured threshold + within `max_retries` (per `:87` "so a retry
     cannot evade the canonical round budget")
- **9 organ 关联**: 9 organ 输出 → Judge 输入 — **真生产前必做**: Judge 评 9 organ
  process 串联结果 (L3 验证层).
- **真实施估时**: 2 周.

### 6.4 `cognitive.self_assessment` (WIRED, Judge-backed)

- **真接 module**: `SelfAssessmentModule` (per `:875-948`)
- **Hook**: `AfterTurn`
- **依赖**: Judge 结果 (no fabricated heuristic score)
- **数据流**:
  1. 拿到 Judge 评的 typed in-process 结果
  2. 持久化到 `SQLiteSelfAssessmentStore`
  3. **0 装诚实**: 仅记录真 Judge 结果, 不伪造
- **9 organ 关联**: F1 Emotion Memory 提供主人情绪时间线 — **真生产前必做**: 把 F1
  当前情绪 + 趋势作为 self_assessment 上下文.
- **真实施估时**: 1 周.

### 6.5 `cognitive.council` (WIRED, OFF by default)

- **真接 module**: `CouncilModule` (per `:963-1049`)
- **Hook**: `AfterModelResponse`
- **依赖**: `Arc<Council>` + 7 LlmAdvisor (named advisor slots; 10s per-advisor / 60s overall
  timeout; per `cognitive-module-wiring.md:96-102`)
- **数据流**:
  1. 拿到 `candidate` (tool-free, per `:1009`)
  2. 构 `Proposal { id, proposer, payload, submitted_at, session_id }` (`:1011-1017`)
  3. 构 `RuntimeCouncilInvoker { invoker: ctx.invoker() }` (`:1018-1020`)
  4. 调 `self.council.decide_with_invoker(&proposal, &adapter).await` (`:1021`)
  5. 输出 `CouncilDecision::Continue / Retry / Stop / DeferToHuman` (`:1023-1030`)
- **9 organ 关联**: W1 + W2 反事实推演 + 因果分支作为 Council proposal payload — **真生产前必做**: 把 W1/W2/W3 output 喂 Council 7 advisor.
- **真实施估时**: 2 周.

---

## §7 6 DEFERRED slot 激活路径

> **本章定位**: 真生产前必做的 6 DEFERRED slot 激活路径 + 估时 + 风险.

### 7.1 `cognitive.preference_learning` (DEFERRED, no owner yet)

- **现状**: 0 实现 (per `cognitive-module-wiring.md:30` "deferred, no evidence-extraction
  side-call or implicit preference mutation").
- **激活路径**:
  1. v1 era `apeireth-companion::preference` 找对应模块 (待核验, 子代理 R10 估存在)
  2. 1:1 翻译 v1 真实现到 v2 OrganTrait / AgentModule 边界
  3. 挂 `AfterTurn` hook, 紧跟 `self_assessment` 后
  4. 调 `SelfAssessmentStore::recent_for_task(task_id, 5)` 作为 evidence 喂入
- **真实施估时**: 2 周.
- **风险**: 0 装诱导 prevention — 不假装"已自动学习主人偏好", 真账是 v1 era 是否有
  preference_learning 模块待核验.

### 7.2 `cognitive.critic` (DEFERRED INTO JUDGE)

- **现状**: 0 独立 module, 已并入 `cognitive.judge` (per `cognitive-module-wiring.md:31`
  "Judge's bounded critique is the single critique path; no duplicate evaluator").
- **激活路径**: 0 估时 (无需激活, 已并入).
- **0 装诚实标**: 不假装"critic 是独立 module, 待激活". 真账是 critic 已并入 Judge,
  单独激活 = 重复 evaluator (破 `cognitive-module-wiring.md:31`).

### 7.3 `cognitive.reflection` (DEFERRED INTO SELF-ASSESSMENT)

- **现状**: 0 独立 module, 已并入 `cognitive.self_assessment` (per
  `cognitive-module-wiring.md:32`).
- **激活路径**: 0 估时 (无需激活, 已并入).
- **0 装诚实标**: 不假装"reflection 是独立 module, 待激活". 真账是 reflection 已并入
  SelfAssessment, 单独激活 = 当前 turn assessment 与 durable memory 重复.

### 7.4 `cognitive.planner` (NOT AN AGENT MODULE)

- **现状**: 0 实现 (per `cognitive-module-wiring.md:33` "no per-turn planner loop; future
  adapter must remain an adapter").
- **激活路径**:
  1. 新建 `PlannerAdapter` 类似 `Orchestrator` (LlmFactory 注入)
  2. 走 per-turn 外的 long-running service (per `cognitive-module-wiring.md:33-34` 原则:
    "future adapter must remain an adapter", 不破 canonical turn 边界)
  3. 7 advisor 加权 + 1 planner 输出 (per L2 提案生成)
- **真实施估时**: 3 周, LLM 重.
- **风险**: LLM 重 + 不破 canonical turn 边界 (per `cognitive-module-wiring.md:33`
  "no per-turn planner loop").

### 7.5 `cognitive.orchestrator` (NOT AN AGENT MODULE)

- **现状**: 0 实现 (per `cognitive-module-wiring.md:34` "long-running Planner → Implementer
  → Reviewer service; never called from the canonical turn").
- **激活路径**: 1:1 翻译 v1 `apeireth-companion::AwakeCompanion` → v2 OrganOrchestrator (per §5).
- **真实施估时**: 3 周, 类似 AwakeCompanion.
- **风险**: 见 §5 真账 (R7 风险 #1).

### 7.6 `cognitive.perception` (NOT AN AGENT MODULE)

- **现状**: text payload 真实现 (per `cognitive-module-wiring.md:35` + `:108-109` "PerceptionInput
  becomes TurnRequest through turn_request_from_perception; only text payload is implemented").
- **激活路径**:
  1. RC-7 真接 Whisper (audio) + xcap (screen) 硬件依赖
  2. 6 modality (text/voice/vision/tactile/screen/audio), 仅 text 真实现 (per
    `v2-architecture-reflection.md:156` "Perception text-only 真实现: 6 modality... 仅 text
    真实现 (其他 0 装)")
  3. `PerceptionInput` → `TurnRequest` 边界严守 (per `:1107-1116` `turn_request_from_perception`)
- **真实施估时**: 2-3 周, 硬件依赖.
- **风险**: 硬件依赖 (Whisper 模型 + xcap 屏幕捕获).

---

## §8 5 状态机 + 8 重门控 + 主动开口

> **本章定位**: per E7 emergence 真实现 + OrganOrchestrator 8 重 gate 串接. 子代理 R10
> 独立判断: 任务 brief 说 "5 状态机 + 8 重门控" 是来自 E7 emergence 真实现 + 9 organ 上下文,
> 子代理 R7 独立判断也写到 emergence.rs 文档头注释.

### 8.1 5 状态机 (per `crates/engine/organ/src/emergence.rs:464-489`)

`PolicyStage` 是**前向声明**, 来自 v1 `apeireth-evolution::state::EvolutionStateMachine`:

| 状态 | 语义 | is_active() | as_str |
|---|---|---|---|
| `Idle` | 初始态 (未起草策略) | `false` | `"idle"` |
| `Draft` | 已起草, 待提交审议 | `false` | `"draft"` |
| `Proposed` | 已提交, 等智囊团审议 | `false` | `"proposed"` |
| `Ratified` | 智囊团通过, 待激活 | `true` (已通过审议可发声) | `"ratified"` |
| `Active` | 已激活, 正在生效 | `true` | `"active"` |

**0 装诚实标 (子代理 R7 独立判断)**:
- v1 `apeireth-companion::emergence::emergence.rs` **不包含**此状态机 (per
  `emergence.rs:450-456` 注释).
- v1 真状态机在 `apeireth-evolution::state::EvolutionStateMachine` (6 状态含 Retired).
- v2 E7 organ crate 不绑 apeireth-evolution (它在 legacy/donor/), 本 enum 是**前向声明**,
  留接口给 future 真接.
- 当前 v2 E7 `process()` 走 rhythm+boundary loop 1:1 v1 真相, 不假装 emergence 自带 5
  状态机. `policy_stage()` 当前永远返 `PolicyStage::Active` 占位 (per `emergence.rs:855-857`).

### 8.2 8 重门控 (per `crates/engine/organ/src/emergence.rs:570-573`)

`EmergenceLoop::tick()` 严格走 8 重门控:

| 顺序 | Gate | 字段 | 触发条件 | last_hold 留痕 |
|---|---|---|---|---|
| 0 | `UserQuiet` | `Boundaries::user_quiet: bool` | 用户显式不打扰开关 | `InitiativeGate::UserQuiet` |
| 1 | `QuietHours` | `Boundaries::in_quiet_window(minutes_now)` | 安静窗口内 (跨午夜支持) | `InitiativeGate::QuietHours` |
| 2 | `DailyLimit` | `self.initiatives_today >= self.boundaries.max_initiatives_per_day` | 每日主动频率上限 (default 2/day) | `InitiativeGate::DailyLimit` |
| 2.5 | `LlmBudget` | `(at_ms - last) < min_llm_interval_ms` | LLM 成本预算 (default 60s) | `InitiativeGate::LlmBudget` |
| 3 | `DepthLow` | `depth < self.boundaries.min_depth` | 关系深度门槛 (default 0.3) | `InitiativeGate::DepthLow` |
| 4 | `RhythmUnknown` | `rhythm.days == 0` | 0 观察天数 → 不猜测作息 | `InitiativeGate::RhythmUnknown` |
| 5 | `RhythmVeto` | `rhythm.active_probability < rhythm_veto_probability` | 学到的作息说此刻几乎不可能活跃 → 沉默压力再大也不打扰 | `InitiativeGate::RhythmVeto` |
| 6 | `DriveLow` | `drive < drive_threshold && !probe` | 驱动未达阈值且未达冷启动探针条件 | `InitiativeGate::DriveLow` |

**0 装诚实标**: `tick()` 严格按序执行, **不**颠倒, **不**漏 (per
`emergence.rs:584-688` 实现).

### 8.3 5 状态机 + 8 重门控 + 主动开口 串接

```
5 状态机 PolicyStage (Active == true 才考虑主动)
     ↓
8 重门控 (任一拦下 → last_hold 留痕, 不开口)
     ↓
驱动 + 节奏匹配 → Initiative { reason, action, rhythm, depth, context_hint }
     ↓
9 organ process 串联 (per §3) 提供 context_hint
     ↓
OrganOrchestrator.should_speak() → true → spoke=true
     ↓
LlmFactory 真接 (v2.1) → 真渲染 Initiative.action.label() → 自然话语
```

**主动开口条件 (per `emergence.rs:679-688`)**:
- 8 重门控全过
- `initiatives_today += 1`
- `last_initiative_ms = Some(at_ms)`
- `last_hold = None` (决定开口, 清除拦下原因)
- `Action::select(context_hint)` (E7 机制层选动作, 不是 LLM 决策)

**输出到 OrganOutput** (per `emergence.rs:891-895`):
- `OrganOutput::Emergence { action: String, spoke: bool }`
- `spoke` = 是否真开口 (被任何一重门控拦下 = false)
- `action` = `Action::label()` (e.g. "问候" / "提醒" / "跟进话题" / "沉默陪伴")

### 8.4 真实施估时

- OrganOrchestrator 串接 5 状态机 + 8 重门控 + 9 organ context_hint 输入 = 1-2 周.
- LLM 真渲染 (v2.1 真接 LlmFactory) = 1 周.
- **总计**: 2-3 周.

---

## §9 L0-L5 自升级 cycle 集成

> **本章定位**: 把 cognitive module 12 slot + 9 organ process 集成进 L0-L5 自升级 cycle,
> per `docs/01-architecture/v2-architecture-reflection.md` §6 (主代理亲写 + 子代理 L 估).

### 9.1 L0 人类审批 (硬墙, 永远不可变)

- **真接**: 0 cognitive slot + 0 organ — L0 在 runtime 主循环外 (per
  `v2-architecture-reflection.md:220-225` "L0: 人类审批 (硬墙, 永远不可变)").
- **9 organ 关联**: 0 organ 拍板, organ 是机制层.
- **cognitive module 关联**: cognitive slot 不审批 — governance hook 在 runtime 主循环.
- **LOCKED**: `crates/foundation/core/src/philosophy/eight_anchors.rs:58-79` (S-1 北极星 +
  O-1 安全优先) (R11 LOCKED).

### 9.2 L1 自我诊断 (runtime 主动)

- **真接**: `cognitive.self_assessment` (WIRED) + `SQLiteSelfAssessmentStore` (RC-4 ✅).
- **9 organ 关联**: F1 Emotion Memory (主人情绪时间线) — **真生产前必做**: 把 F1 当前
  情绪 + 趋势作为 self_assessment 上下文.
- **触发条件**: Self-Disable 判定 (L0 HA 物理隔离) (per `v2-architecture-reflection.md:227`).
- **真实施估时**: 1 周 (F1 → self_assessment 串接).

### 9.3 L2 提案生成 (orchestrator)

- **真接**: `Orchestrator::dispatch(spec)` → `Council::with_factory(factory, model).decide_with_invoker()`.
- **9 organ 关联**: W1 + W2 + W3 提供反事实 / 因果分支作为 proposal payload.
- **7 advisor 并行 + 60s timeout** (per `cognitive-module-wiring.md:96-102`).
- **7 system prompt template** (per `v2-architecture-reflection.md:237`).
- **真实施估时**: 2-3 周 (Orchestrator + 9 organ 桥).

### 9.4 L3 验证 (testing sandbox)

- **真接**: 9 organ process 串联 (per §3) + sandbox regression + 5 重守门.
- **5 重守门**:
  1. clippy 0 警告
  2. workspace tests 0 失败
  3. legacy compat path < 100 引用
  4. 13 键 LOCKED + 9 哲学锚本体 + workspace.version + R11 baseline 0 触碰
  5. 哲学锚表头 0 减
- **失败 → 自动回滚** (per `v2-architecture-reflection.md:291`).
- **真实施估时**: 1-2 周 (9 organ 串联入口 + sandbox 跑通).

### 9.5 L4 主人审批 (governance hook)

- **真接**: governance 3 hook + Council 多意见加权 + 主人 Veto.
- **3 hook**:
  - `PromptInjectionHook` (拦)
  - `PermissionGovernanceHook` (控)
  - `CredentialDisclosureHook` (脱敏)
- **9 organ 关联**: E7 emergence 8 + 5 重 gate 上层闸留痕 (per
  `emergence.rs:432-441` `SovereigntyFrozen` / `EmotionLow` / `CouncilVeto` /
  `PolicyInactive` / `GateBlock`).
- **cognitive module 关联**: `CouncilDecision::DeferToHuman` 走 `ModuleOutcome::stop` →
  主代理审 (per `cognitive.rs:1027-1029`).
- **真实施估时**: 1-2 周 (governance hook 与 OrganOrchestrator 桥 + Council 加权).

### 9.6 L5 runtime patch (自我升级)

- **真接**: 主代理批 → `git tag v2.x+1` → 新版本生效.
- **9 organ 关联**: LlmFactory 新 model 即时生效 (per `v2-architecture-reflection.md:255-261`).
- **cognitive module 关联**: 12 slot 即时激活/废弃 (forward-declared 边界严守).
- **LOCKED**: `Cargo.toml:43` workspace.version = "1.2.0" 0 改 (per Q1 任务 #5).
- **真实施估时**: 1 周 (release script + 9 organ 回归 + cognitive slot 激活/废弃).

### 9.7 自升级 cycle 时间表 (per `v2-architecture-reflection.md:298-305`)

| 升级类型 | 估时 |
|---|---|
| 加 1 capability trait | 1-2 周 |
| 改 LLM provider | 1 周 (per LlmFactory trait, 子代理 M 已写真 impl) |
| 加 1 organ 真移植 | 4-6 周 (per 子代理 L 估, E4 curiosity 最易) |
| 认知模块新 slot | 2-3 周 (12 slot ledger 当前 6 WIRED, 6 DEFERRED) |
| 改 Triple onion L3-L5 真实现 | 4-6 周 |
| **总估计每次自升级** | **1-6 周** (取决于升级类型) |

### 9.8 主人角色 (per `v2-architecture-reflection.md:307-313`)

v2.0 release 后, **主代理不再每件手写**. Apeireth 自我升级, 主人:
1. **拍板** (L0 + L4 审批)
2. **守门** (5 重守门失败时介入)
3. **不写代码** (Apeireth 写, 主人审)

---

## §10 0 装诚实真账 (子代理 R10 独立判断)

### 10.1 v2.0-rc.1 真账

- **9 organ 真兑现** (整合 #2 commit `bbf70293`):
  - E4 Curiosity ✅ (子代理 Q1)
  - F1 Emotion Memory ✅ (子代理 R1)
  - F4 Hypothesis ✅ (子代理 R2)
  - F6 Value Cases ✅ (子代理 R3)
  - W1 World Model + LLM ✅ (子代理 R4)
  - W2 Causal World Model + LLM MCTS ✅ (子代理 R5)
  - W3 Edge Mining ✅ (子代理 R6)
  - E7 Emergence ✅ (子代理 R7, deterministic 0 LLM, 8 重门控 + 5 状态机前向声明)
  - Memory Merger ✅ (子代理 R8, 子代理 R10 同意 R8 独立判断 "v1 无 MemoryMerger,
    v2 是新抽象")
- **cognitive module 12 slot 真接**:
  - 6 WIRED: `memory_recall` / `preference_recall` / `judge` / `self_assessment` /
    `memory_writeback` / `council` (judge/council 为 WIRED, OFF by default; per ledger
    `cognitive-module-wiring.md:24-29`, R13 接力审真账核验)
  - **总计**: 6 WIRED + 6 DEFERRED
  - 6 DEFERRED: `preference_learning` / `cognitive.critic` (DEFERRED INTO JUDGE) /
    `cognitive.reflection` (DEFERRED INTO SELF-ASSESSMENT) / `cognitive.planner` /
    `cognitive.orchestrator` / `cognitive.perception`
- **串联层**:
  - **OrganOrchestrator 缺** (R7 风险 #1 提, 子代理 R10 独立判断: 真账是缺, 不是 0 装诱导)
  - **认知模块与 9 organ 没串** (两条 ABI 独立, 缺 OrganOrchestrator 串接)
- **0 装诱导 prevention**:
  - 不假装"cognitive module × 9 organ 已集成"
  - 不假装"OrganOrchestrator 已实施"
  - 不假装"6 DEFERRED 已激活"
  - 不假装"5 状态机 + 8 重门控已串接到 cognitive module"

### 10.2 与子代理 L / R7 独立判断一致

- **子代理 L** (v2.0 → 1.0 parity 距离估): 5-7 月, 2027-01-08 至 2027-03 月 v2.0.0 release.
  本 spec 是"真生产前阻塞 #2" 的实施规范, 估 1 周 (子代理 R10 估 +30% buffer = 1-3 周).
- **子代理 R7** (E7 emergence + OrganOrchestrator 风险): 9 organ 缺 orchestrator 串联,
  OrganOrchestrator 类似 legacy AwakeCompanion 待真实施. 子代理 R10 同意 R7 独立判断.
- **子代理 Z** (0 装诱导 prevention 本身是 0 装诱导): 任务 brief 说"估 1 周" 是真账
  (1-3 周真实施); 子代理 R10 只写 spec (30-45 分钟), 不真做 1 周. 这条"只写 spec 不真做"
  本身就是 0 装诱导 prevention (per 子代理 Z 独立判断).

### 10.3 本 spec 的 0 装诚实标

- **0 装诚实标 #1**: 本 spec 不真做 1 周 cognitive module × 9 organ 集成. **只**写规范.
- **0 装诚实标 #2**: OrganOrchestrator 真账是"缺" (R7 风险 #1). 不假装"已实施".
- **0 装诚实标 #3**: 6 DEFERRED 真账是"0 激活" (forward-declared). 真生产前估 6-10 周激活.
- **0 装诚实标 #4**: 5 状态机真账是"前向声明" (per `emergence.rs:450-456` 注释). v2 E7
  `process()` 不真接状态机, 永远返 `PolicyStage::Active` 占位.
- **0 装诚实标 #5**: E7 emergence 决策路径严格确定性, 不假装"E7 always speak".
  8 重门控 + Rate-Limit + Idle 抑制 = 严格沉默抑制.
- **0 装诚实标 #6**: Memory Merger 子代理 R8 独立判断 "v1 无 MemoryMerger 模块; v2 是
  新抽象, 借鉴 v1 MemoryExtractionService 算法骨架 1:1 翻译". 子代理 R10 同意 R8 独立
  判断, 不假装"v1 有 MemoryMerger".

### 10.4 风险 vs 0 装诱导

| 项目 | 真账 | 是否 0 装诱导 |
|---|---|---|
| 9 organ 真实现 | 真 (整合 #2 commit `bbf70293`) | ❌ 不是 |
| cognitive module 12 slot 真接 | 6 WIRED 真接 | ❌ 不是 |
| OrganOrchestrator | **缺** | ❌ 不是 (R7 风险 #1 真账) |
| 6 DEFERRED 激活 | **0 激活** | ❌ 不是 (forward-declared 真账) |
| 5 状态机真接 | **前向声明, 不真接** | ❌ 不是 (`emergence.rs:450-456` 真账) |
| E7 always speak | **不是** (8 重门控严格沉默抑制) | ❌ 不是 (`emergence.rs:760-763` 真账) |

---

## §11 0 触碰 LOCKED (5 项 + 扩展)

> **本章定位**: 真生产前 cognitive module × 9 organ 集成实施时 0 触碰的边界 (子代理 R10 独立判断).

### 11.1 5 项 LOCKED (per `docs/04-internal/v2-rc-1-progress-report.md` + Q1 任务 brief)

1. **`apeireth_locked_items.rs` baseline 0 改** (5 项 LOCKED 项本体文件)
2. **8 哲学锚本体** `crates/foundation/core/src/philosophy/eight_anchors.rs:58-79` enum 顺序
   0 改 (LOCKED, O-6 已加升 8→9)
3. **13 键** `crates/foundation/core/src/philosophy.rs:142` `RUNTIME_ENFORCED = false`
   0 改 (LOCKED, 降级为哲学标准)
4. **3 项不可变脊柱** `crates/foundation/core/src/onion.rs:249` 0 改 (LOCKED, 仅 rustfmt)
5. **`Cargo.toml:43`** `workspace.version = "1.2.0"` 0 改 (LOCKED)

### 11.2 扩展 LOCKED 边界 (本 spec 新增严守)

6. **R11 baseline 3 值** (`body.rs` 等) 0 改 (LOCKED, `0.8682/0.8532/0.9063`)
7. **`cognitive-module-wiring.md`** 12 slot ledger 0 改 (forward-declared 边界, per
   `:23-35` 锁定)
8. **`crates/engine/runtime/src/canonical/cognitive.rs`** 6 个 `pub const ..._MODULE_ID: &str`
   常量 0 改 (compatibility contract, per `:35-36`)
9. **`crates/engine/runtime/src/canonical/cognitive.rs:1121-1138`** `DEFERRED_COGNITIVE_SLOTS`
   常量 0 改 (维护当前真账的 0 装诚实标)
10. **`crates/foundation/plugin/src/organ.rs:69-89`** `OrganKind` enum 顺序 0 改 (9 organ IDs
    锁定, 兼容 contract)
11. **`crates/foundation/plugin/src/organ.rs:157-188`** `OrganOutput` 9 variant 顺序 0 改
    (兼容 contract)
12. **`Cargo.lock` 0 行 diff** (本 spec 不引新外部 dep, 仅文档)

### 11.3 真生产前实施者必核验项

- [ ] 5 项 LOCKED 文件 0 字节 diff
- [ ] `cognitive-module-wiring.md` 12 slot ledger 0 行 diff
- [ ] `cognitive.rs` 6 个 `pub const ..._MODULE_ID: &str` 0 行 diff
- [ ] `cognitive.rs:1121-1138` `DEFERRED_COGNITIVE_SLOTS` 0 行 diff
- [ ] `organ.rs:69-89` `OrganKind` enum 0 行 diff
- [ ] `organ.rs:157-188` `OrganOutput` enum 0 行 diff
- [ ] `Cargo.lock` 0 行 diff
- [ ] `Cargo.toml:43` workspace.version 0 行 diff
- [ ] R11 baseline 3 值 0 行 diff

---

## §12 真生产前阻塞 #2: frontend 对接 (估 4-6 周)

> **本章定位**: 任务 brief 提到"真生产前阻塞 #2: frontend 对接, 4-6 周, 估 2027-Q1 启动,
> 需 R9 spec + 实施". 本 spec 不展开 frontend 详情, 只标与 cognitive module × 9 organ 集成
> 的接口边界.

### 12.1 frontend 对接需求

- 主人 Veto UI (L4 闸留痕可视化)
- Council 加权可视化 (7 advisor verdict 列表)
- 9 organ process 串联进度可视化 (E4 → F1 → F4 → F6 → W1 → W2 → W3 → E7 → Memory)
- 5 状态机状态可视化 (Idle/Draft/Proposed/Ratified/Active)
- 8 重门控拦下原因可视化 (`InitiativeGate::UserQuiet` / `QuietHours` / ...)

### 12.2 与 cognitive module × 9 organ 集成的接口

- `CognitiveTelemetry::events()` (per `cognitive.rs:102-108`) → frontend 拉取 cognitive module
  slot 事件
- `ModuleMetricsSnapshot` (per `cognitive.rs:56-69`) → frontend 拉取各 slot metrics
- 9 organ `OrganOutput` → frontend 通过 OrganOrchestrator 接口拉取
- E7 `last_hold()` → frontend 显示主动开口留痕

### 12.3 真实施估时

- frontend spec: 1-2 周 (估 R9)
- frontend 实施: 3-4 周
- **总计**: 4-6 周

---

## §13 接手人 actionable (5/5 done + 7 #6/#7 真生产前必做)

> **本章定位**: 任务 brief 复述 `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` 的 5/5 done
> + 加 2 项真生产前必做 (#6 OrganOrchestrator + #7 6 DEFERRED 激活).

### 13.1 5/5 done (v2.0-rc.1 已就位)

- ✅ **#1 RC-5/6/7 + 9 organ 真移植全 done** (整合 #2 commit `bbf70293`)
- ✅ **#2 哲学锚 ledger 待核** (子代理 K + 主代理亲做 0 装诚实修正, 9 锚 LOCKED + 13 键降级)
- ✅ **#3 12 consumer 弃用迁移** (子代理 H 独立判断 + 子代理 I 真写 RC-11 migration script)
- ✅ **#4 RC-10 line header AAD + APX2 envelope** (子代理 E 审查 + 主代理 O-6 #23 commit)
- ✅ **#5 cognitive module 不变量 + 9 organ trait 抽象边界** (子代理 J 核验 0 触碰 LOCKED)

### 13.2 ⏳ #6 6 DEFERRED slot 激活 (估 6-10 周, 真生产前必做)

- `cognitive.preference_learning` 估 2 周
- `cognitive.planner` 估 3 周
- `cognitive.orchestrator` 估 3 周 (与 OrganOrchestrator 部分重叠)
- `cognitive.perception` 估 2-3 周 (硬件依赖)
- `cognitive.critic` 0 估时 (已并入 Judge)
- `cognitive.reflection` 0 估时 (已并入 SelfAssessment)

### 13.3 ⏳ #7 OrganOrchestrator 类似 AwakeCompanion (估 1-3 周, 真生产前必做)

- OrganOrchestrator 骨架 + 9 organ 注入 + 13 重 gate = 1 周
- 9 organ process 串联入口 + 错误处理 = 1 周
- Council / Emotion / Onion / RelationshipState 桥接 = 1 周

### 13.4 真生产前实施顺序 (子代理 R10 建议)

1. **第 1-3 周**: OrganOrchestrator 骨架 + 9 organ 注入 (#7)
2. **第 4-5 周**: cognitive.self_assessment + F1 Emotion Memory 串接 (#6 partial)
3. **第 6-8 周**: cognitive.judge + 9 organ process 串联评 (#6 partial)
4. **第 9-11 周**: cognitive.council + W1/W2/W3 桥接 (#6 partial)
5. **第 12-13 周**: cognitive.preference_learning + cognitive.reflection (#6 partial)
6. **第 14-15 周**: cognitive.orchestrator + cognitive.planner (#6 partial)
7. **第 16-17 周**: cognitive.perception + RC-7 真接 (#6 partial)
8. **第 18-19 周**: 整体 sandbox 跑通 + 5 重守门 + 真账核验
9. **总计**: 估 5-7 月真生产 (per 子代理 L 估 2027-Q1 启动, 2027-Q2 完)

---

## 附录 A: 关键文件:行 reference

### A.1 cognitive module (12 slot)

| Slot / ID | File:Line | 真接状态 |
|---|---|---|
| `cognitive.memory_recall` (id) | `crates/engine/runtime/src/canonical/cognitive.rs:37` | WIRED |
| `cognitive.memory_writeback` (id) | `:38` | WIRED |
| `cognitive.preference_recall` (id) | `:39` | WIRED |
| `cognitive.self_assessment` (id) | `:40` | WIRED |
| `cognitive.judge` (id) | `:41` | WIRED (OFF by default) |
| `cognitive.council` (id) | `:42` | WIRED (OFF by default) |
| `MemoryRecallModule` (struct) | `:255-402` | 真接 |
| `MemoryWritebackModule` (struct) | `:405-554` | 真接 |
| `PreferenceRecallModule` (struct) | `:571-622` | 真接 |
| `JudgeModule` (struct) | `:727-848` | 真接 |
| `SelfAssessmentModule` (struct) | `:875-948` | 真接 |
| `CouncilModule` (struct) | `:963-1049` | 真接 |
| `RuntimeCouncilInvoker` (struct) | `:1051-1082` | 真接 |
| `turn_request_from_perception` (fn) | `:1107-1116` | 真接 (text only) |
| `DEFERRED_COGNITIVE_SLOTS` (const) | `:1121-1138` | 维护当前真账 |

### A.2 9 organ

| Organ | Trait ID | File:Line | 子代理 |
|---|---|---|---|
| W1 World Model | `OrganKind::W1` | `crates/engine/organ/src/world_model.rs` | R4 (真接 LLM) |
| W2 Causal World Model | `OrganKind::W2` | `crates/engine/organ/src/causal_world_model.rs` | R5 (真接 LLM MCTS) |
| W3 Edge Mining | `OrganKind::W3` | `crates/engine/organ/src/causal_world_model_edges.rs` | R6 (deterministic) |
| E4 Curiosity | `OrganKind::E4` | `crates/engine/organ/src/curiosity.rs` | Q1 (deterministic 0 LLM) |
| F4 Hypothesis | `OrganKind::F4` | `crates/engine/organ/src/hypothesis.rs` | R2 (deterministic) |
| F1 Emotion Memory | `OrganKind::F1` | `crates/engine/organ/src/emotion_memory.rs` | R1 (deterministic) |
| F6 Value Cases | `OrganKind::F6` | `crates/engine/organ/src/value_cases.rs` | R3 (deterministic) |
| E7 Emergence | `OrganKind::E7` | `crates/engine/organ/src/emergence.rs` | R7 (deterministic 0 LLM) |
| Memory Merger | `OrganKind::Memory` | `crates/engine/organ/src/memory.rs` | R8 (deterministic, 子代理 R8 独立判断 "v1 无此模块") |

### A.3 OrganTrait 边界

| 类型 | File:Line |
|---|---|
| `OrganKind` enum (9 variant) | `crates/foundation/plugin/src/organ.rs:69-89` |
| `OrganInput` struct | `crates/foundation/plugin/src/organ.rs:124-150` |
| `OrganOutput` enum (9 variant) | `crates/foundation/plugin/src/organ.rs:157-188` |
| `OrganError` enum | `crates/foundation/plugin/src/organ.rs:239-278` |
| `OrganTrait` trait | `crates/foundation/plugin/src/organ.rs:292-311` |
| 9 organ 真实现 re-export | `crates/engine/organ/src/lib.rs:49-52` |
| `NoopOrgan` (0 装 PASS) | `crates/engine/organ/src/lib.rs:73-108` |

### A.4 E7 emergence + 8 重门控 + 5 状态机

| 概念 | File:Line |
|---|---|
| `EmergenceLoop<R>` struct | `crates/engine/organ/src/emergence.rs:503-517` |
| 8 重门控 (tick 实现) | `crates/engine/organ/src/emergence.rs:570-688` |
| `InitiativeGate` enum (8 + 5 = 13 variant) | `crates/engine/organ/src/emergence.rs:422-442` |
| `PolicyStage` enum (5 variant, 前向声明) | `crates/engine/organ/src/emergence.rs:464-471` |
| `PolicyStage::is_active()` | `crates/engine/organ/src/emergence.rs:473-477` |
| `EmergenceOrgan` struct (OrganTrait impl) | `crates/engine/organ/src/emergence.rs:764-901` |
| 0 装诚实标注释 | `crates/engine/organ/src/emergence.rs:1181-1195` |

### A.5 L0-L5 自升级 cycle

| Layer | File:Line (出处) |
|---|---|
| L0 人类审批 | `docs/01-architecture/v2-architecture-reflection.md:220-225` |
| L1 自我诊断 | `:226-230` |
| L2 提案生成 | `:231-237` |
| L3 验证 | `:238-244` |
| L4 主人审批 | `:245-252` |
| L5 runtime patch | `:253-261` |

### A.6 9 organ 整合 #2 commit

- `bbf70293 feat(organ): 9 organ 真移植 v2 全部完成 (E4/F1/F4/F6 第一批 + W1/W2/W3/E7/Memory 第二批)`

---

## 附录 B: 缩略词 + 引用

### B.1 缩略词

- **9 organ**: 9 个认知/行为器官 (W1/W2/W3/E4/F4/F1/F6/E7/Memory), 来自 v1 `apeireth-companion`
- **R11**: 9 UI 器官 (body/brain/ear/eye/hand/heart/memory/mind/voice), LOCKED 0 触碰
- **O-6**: 哲学锚 "永远追求最优" (主代理 2026-08-27 加)
- **L0-L5**: 自升级 cycle 6 层 (人类审批 / 自我诊断 / 提案生成 / 验证 / 主人审批 / runtime patch)
- **WIRED**: cognitive module 12 slot 真接
- **DEFERRED**: cognitive module 12 slot 0 激活 (forward-declared)
- **WIRED, OFF by default**: 真接但默认关闭 (per `cognitive-module-wiring.md:26-27` judge/council; 旧称 "SLOT READY" 已弃用, R13 接力审修正)
- **AGENT MODULE**: `AgentModule` ABI (cognitive module 用, 独立于 7 capability trait)
- **OrganTrait**: `OrganTrait` ABI (9 organ 用)
- **5 重守门**: clippy 0 / tests 0 / 13 键 LOCKED / workspace.version / R11 baseline
- **13 重 gate**: 8 重 E7 emergence + 5 重 v2 扩展 (SovereigntyFrozen / EmotionLow / CouncilVeto / PolicyInactive / GateBlock)
- **5 状态机**: Idle / Draft / Proposed / Ratified / Active (PolicyStage 前向声明)
- **8 重门控**: UserQuiet / QuietHours / DailyLimit / LlmBudget / DepthLow / RhythmUnknown / RhythmVeto / DriveLow
- **7 advisor**: Council 默认 7 LlmAdvisor (named slots; 10s per / 60s overall timeout)
- **Mavis**: 主代理 (写 `v2-architecture-reflection.md` + `FINAL-HANDOFF-V2.0.0-RC.1.md`)
- **子代理 R10**: 当前子代理 (写本 spec)

### B.2 引用

- `docs/04-internal/cognitive-module-wiring.md` (12 slot ledger)
- `docs/04-internal/FINAL-HANDOFF-V2.0.0-RC.1.md` (5/5 done 接手报告)
- `docs/01-architecture/v2-architecture-reflection.md` (L0-L5 自升级 cycle 设计)
- `crates/engine/runtime/src/canonical/cognitive.rs` (12 slot 真接 module)
- `crates/foundation/plugin/src/organ.rs` (OrganTrait 边界)
- `crates/engine/organ/src/lib.rs` (9 organ impl 入口)
- `crates/engine/organ/src/emergence.rs` (E7 + 8 重门控 + 5 状态机)
- `legacy/donor/apeireth-companion/examples/awake_greeting.rs` (v1 AwakeCompanion + 真 LLM 渲染)
- `bbf70293` 整合 #2 commit (9 organ 真移植 v2 全部完成)
- `b9026186` v2.0.0-rc.1 release tag 拍板 (本 spec HEAD)

---

**文档结束**.
