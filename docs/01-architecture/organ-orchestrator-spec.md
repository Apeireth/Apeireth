# OrganOrchestrator 类似 AwakeCompanion 串联规范 (v2.0.0-rc.1+ spec)

> **本文档定位**: v2.0.0-rc.1 真生产前阻塞项 **#6** — OrganOrchestrator 类似 v1 AwakeCompanion 串联规范.
> **何时写**: 子代理 R11 (2026-08-28), 在 `b9026186` (v2.0.0-rc.1 tag 拍板) 后写.
> **读谁**: 接手 Apeireth v2.0 真生产路径的工程师 / 未来实施 OrganOrchestrator 的子代理.
> **关系文档**: 本文 + `cognitive-module-wiring.md` (12 slot 注入) + `v2-architecture-reflection.md` §6 (L0-L5 自升级 cycle) + `FINAL-HANDOFF-V2.0.0-RC.1.md` §10 接手人 actionable #6 + `philosophy.md` (9 锚 LOCKED).
> **本文状态**: 🟡 **spec 写完, 真实施待 v2.0.0 release 后启动** (估 1-3 周, 主代理后续派子代理).

```
[Document-Meta]
Document:        docs/01-architecture/organ-orchestrator-spec.md
Version:         Spec-0.1 (v2.0.0-rc.1+ 待实施)
Last-Modified:   2026-08-28
Status:          🟡 待实施 (不假装 "已完成")
HEAD:            b9026186 (v2.0.0-rc.1)
Author:          子代理 R11 (独立判断, 0 装诚实真账)
```

---

## §0. TL;DR (1 段总结)

**v1 AwakeCompanion** (`legacy/donor/apeireth-companion/src/organs.rs:34`) 是真实现: 8 重门控 (emergence.rs:460-503) + 5 状态机 (apeireth-evolution::EvolutionState, Idle/Draft/Proposed/Ratified/Active/Retired) + 9 organ 串联 (E4/F1/F4/F6/W1/W2/W3/E7/Council) + emotion/council/onion 三层合成.

**v2 缺 OrganOrchestrator**: 9 organ 已全部真移植 (`crates/engine/organ/src/{curiosity,emotion_memory,hypothesis,value_cases,world_model,causal_world_model,causal_world_model_edges,emergence,memory}.rs`, 1:1 翻译), 但 **缺串联它们的上层 orchestrator** — runtime 拿 `Arc<dyn OrganTrait>`, 没有 9 organ 之间的 process 串联路径 + 8 重 gate 统一入口 + 5 状态机 transition driver.

**估 1-3 周真实施** (per 子代理 L 估 + 子代理 R7 独立判断 + 本文 §8 0 装诚实真账), 真生产前必做.

---

## §1. 概述

### 1.1 为什么需要 OrganOrchestrator

v1 AwakeCompanion 不仅是 E7 emergence 的"上层封装",而是**唯一串起 9 organ + 8 重门控 + 5 状态机 + 三层合成 (emotion + council + onion) 的运行时入口**. 没有它:
- 9 organ 是 9 个独立 `Arc<dyn OrganTrait>`, 各自 `process()` 各自返 `OrganOutput`
- 没有"主动策略"概念 — 5 状态机 (Idle/Draft/Proposed/Ratified/Active) **不**在 E7 organ 内部
- 没有 8 重 gate 的统一入口 — E7 emergence 8 门禁散在 `emergence.rs:460-503` 各 `if` 分支
- 没有"反馈 → 关系加深 / 退回 Draft"的演化闭环

### 1.2 子代理 R7 独立判断 (重要!)

**5 状态机不在 E7 emergence.rs 内部**. 任务说明把"5 状态机 Idle/Draft/Proposed/Ratified/Active"挂在 E7 emergence.rs 头上是不准确的. **真相**:
- v1 `apeireth-companion::emergence.rs` 是**纯机制** (节律 + 边界 + 沉默压力 + 沉默驱动决策), **不含状态机**.
- 5 状态机在 v1 `apeireth-evolution::EvolutionState` (`legacy/donor/apeireth-evolution/src/state.rs:26`):
  ```rust
  pub enum EvolutionState {
      Idle, Draft, Proposed, Ratified, Active, Retired, // 6 个, 含 Retired 终态
  }
  ```
- v2 `crates/engine/organ/src/emergence.rs:465` 已加 `PolicyStage` 前向声明 (5 variant: Idle/Draft/Proposed/Ratified/Active), `policy_stage()` 永远返 `Active` 占位 (per 子代理 R7 标注"未来 apeireth-evolution 接入后真改").

**结论**: OrganOrchestrator spec 应基于 **EvolutionState (6 状态含 Retired) + PolicyStage (5 状态前向声明)**, 不假装 E7 emergence 自带状态机. v1 AwakeCompanion `ratify_fresh_policy` (`organs.rs:73-84`) 才是真入口.

### 1.3 真生产前阻塞位置

per `FINAL-HANDOFF-V2.0.0-RC.1.md` §10 接手人 actionable #6:
- ✅ #1 RC-5/6/7 + 9 organ 真移植全 done
- ✅ #2 哲学锚 ledger 待核
- ✅ #3 12 consumer 弃用迁移
- ✅ #4 RC-10 line header AAD + APX2 envelope
- ✅ #5 cognitive module 不变量 + 9 organ trait 抽象边界
- ⏳ **#6 OrganOrchestrator 类似 AwakeCompanion** (本 spec 完成, 真实施 1-3 周待)

---

## §2. v1 AwakeCompanion 真实现 (1:1 翻译)

### 2.1 struct 字段 (organs.rs:34-49)

```rust
pub struct AwakeCompanion {
    pub loop_: EmergenceLoop<Bond>,        // E7 机制层 (节律+边界+沉默)
    pub emotion: EmotionEngine,            // F1 consciousness 情绪引擎 (PAD)
    pub council: Council,                  // 7 强制 advisor (apeireth-council)
    pub evolution: EvolutionStateMachine,  // 5+Retired 状态机 (apeireth-evolution)
    pub asi_feedback: Vec<UserFeedback>,   // 24 维真实轨迹历史
    pub gate: SecurityGate,                // 洋葱门 (V1 哲学 × V2 权限 × V3 HA)
    pub sovereignty: SovereigntyGate,      // 主权总闸 (最高优先: 熔断=一切停止)
    consecutive_ignores: u32,              // 连续被忽略计数
    last_deliberation: Option<DeliberationEcho>, // 审议回声 (供 tone() 措辞强度)
    last_decision: Option<GateDecision>,   // tick 决策留痕 (presence 观测口)
}
```

### 2.2 关键路径 file:line

| 路径 | v1 file:line | v2 对应 | 备注 |
|---|---|---|---|
| `AwakeCompanion::new()` | `organs.rs:51-70` | ⏳ 待实施 (本 spec) | 7 advisor 召集 + `ratify_fresh_policy` 链 |
| `AwakeCompanion::tick()` | `organs.rs:89-169` | ⏳ 待实施 (本 spec) | **核心串联入口** (主权闸 → 机制 → 情绪 → 审议 → 演化 → 洋葱门) |
| `AwakeCompanion::tone()` | `organs.rs:185-191` | ⏳ 待实施 (本 spec) | 三层合成 (关系×情绪×审议) |
| `AwakeCompanion::apply_feedback()` | `organs.rs:194-245` | ⏳ 待实施 (本 spec) | 反馈 → 情绪事件 → 关系加深/淡化 → 演化 |
| `AwakeCompanion::observe_interaction()` | `organs.rs:247-262` | ⏳ 待实施 (本 spec) | 关系重新活跃 → 重新批准策略 |
| `AwakeCompanion::depth()` | `organs.rs:264-266` | ⏳ 待实施 (本 spec) | 关系深度观测口 |
| `AwakeCompanion::with_config()` | `organs.rs:269-274` | ⏳ 待实施 (本 spec) | 实验调参入口 |
| `ratify_fresh_policy()` (private) | `organs.rs:73-84` | ⏳ 待实施 (本 spec) | 5 状态机 Idle→Draft→Proposed→Ratified→Active 全链 |
| `last_deliberation()` | `organs.rs:172-174` | ⏳ 待实施 (本 spec) | 审议回声读取 |
| `last_decision()` | `organs.rs:177-180` | ⏳ 待实施 (本 spec) | 决策留痕读取 |
| E7 8 重门控 | `emergence.rs:460-503` | ✅ `crates/engine/organ/src/emergence.rs` 1:1 翻译 | 见 §5 |
| EvolutionState 6 状态 | `apeireth-evolution/src/state.rs:26` | ⏳ PolicyStage 前向声明 (`emergence.rs:465`) | 6 state - Retired = 5 (本 spec) |
| InitiativeGate 13 真实门控 | `presence.rs:53` + `presence.rs:410-423` | ⏳ 待实施 (本 spec) | emergence 8 + organs 5 = 13 |

### 2.3 AwakeCompanion::tick 串联顺序 (organs.rs:89-169)

```rust
pub fn tick(&mut self, now, context_hint) -> Option<Initiative> {
    // 1. 主权总闸 (最高优先, 熔断 = 一切停止) — organs.rs:91-94
    if self.sovereignty.is_frozen() { return None; }

    // 2. 机制层 (E7 EmergenceLoop.tick, 8 重门控) — organs.rs:96-106
    let init = match self.loop_.tick(now, context_hint) {
        Some(i) => i,
        None => { /* 机制层 8 重门控逐分支留痕 → None */ }
    };

    // 3. 情绪调制 (consciousness PAD, mood_floor 抑制) — organs.rs:108-114
    let pad = self.emotion.current_pad();
    let mood = (pad.p + 1.0) / 2.0;
    if mood < self.loop_.config.mood_floor { return None; }

    // 4. 智囊团审议 (Council, 7 advisor 加权) — organs.rs:116-135
    let verdict = self.council.deliberate(query);
    self.last_deliberation = Some(DeliberationEcho { ... });
    if verdict.is_rejected() { return None; }

    // 5. 演化闸 (EvolutionStateMachine.current.is_active()) — organs.rs:137-141
    if !self.evolution.current.is_active() { return None; }

    // 6. 洋葱门 (V1 哲学 × V2 权限 × V3 HA, SecurityGate.check) — organs.rs:142-163
    let verdict = self.gate.check("proactive_contact", ..., RiskLevel::Low, ...);
    match verdict {
        ActionVerdict::Allow => {},
        ActionVerdict::BlockByPrinciple(key) => { /* 熔断证据 */ },
        _ => { /* 权限/HA 拦下 */ },
    }

    // 7. 开口决策留痕 + 返 Initiative
    self.last_decision = Some(GateDecision::Spoke { action: ... });
    Some(init)
}
```

**0 装诚实**: v1 `AwakeCompanion::tick` 是**确定性串联** (8 门控 + 5 状态机 + 13 InitiativeGate 真实门控), **不假装**LLM 决策. LLM 只在送达 (`deliver`) 时介入.

---

## §3. OrganOrchestrator 9 organ 串联路径 (L0-L5)

per `v2-architecture-reflection.md` §6 (子代理 R11 整合 v1 AwakeCompanion 真路径 + v2 cognitive module ledger):

### L0: 人类审批 (硬墙, 永远不可变)

| 项 | 说明 |
|---|---|
| **位置** | `philosophy.md` + `governance` crate (per `cognitive-module-wiring.md` §110) |
| **真实现** | ✅ 9 哲学锚 + 13 键 LOCKED (per O-6) |
| **不变量** | "0 触碰 LOCKED" 子代理 R11 baseline 0 改 (per §9 0 触碰 LOCKED) |
| **v1 AwakeCompanion 串接** | `organs.rs:152-157` BlockByPrinciple → `sovereignty.report_violation` → 熔断证据 |
| **v2 OrganOrchestrator 待做** | ⏳ 实施时调用 `SovereigntyGate::report_violation` 物理隔离 |

### L1: 自我诊断 (cognitive.self_assessment via RC-4)

| 项 | 说明 |
|---|---|
| **位置** | `crates/engine/runtime/src/canonical/cognitive.rs` (slot `cognitive.self_assessment`) |
| **真实现** | ✅ WIRED, Judge-backed (per `cognitive-module-wiring.md:28`) |
| **依赖** | `Arc<dyn SelfAssessmentStore>` (RC-4 ✅) |
| **触发** | `AfterTurn` hook (per `cognitive-module-wiring.md:42` 顺序) |
| **v1 AwakeCompanion 串接** | ❌ v1 无 L1 (AwakeCompanion 是"主动路径", 不"诊断") |
| **v2 OrganOrchestrator 待做** | ⏳ 实施时 OrganOrchestrator 调 `SelfAssessment::current()` 喂 5 状态机 Idle→Draft 触发 |

### L2: 提案生成 (Orchestrator + 7 LlmAdvisor via RC-6)

| 项 | 说明 |
|---|---|
| **位置** | `crates/foundation/orchestration/` (per `cognitive-module-wiring.md:34`) |
| **真实现** | ✅ `Orchestrator` service + `Council` 7 advisor (per RC-6 子代理 N) |
| **依赖** | `Arc<dyn LlmFactory>` (RC-5 ✅, 子代理 M 真写) + `Arc<dyn Advisor>` ×7 |
| **触发** | OrganOrchestrator 起草新主动策略时调 `Council::deliberate(query)` |
| **60s timeout** | per `cognitive-module-wiring.md:99` (10s/advisor + 60s 总) |
| **v1 AwakeCompanion 串接** | `organs.rs:116-135` Council 审议 (但 v1 council 是 deterministic 7 advisor, v2 是真 LLM) |
| **v2 OrganOrchestrator 待做** | ⏳ 实施时 `OrchestratorService::propose_policy()` 调 Council |

### L3: 验证 (9 organ process 串联 + sandbox regression)

| 项 | 说明 |
|---|---|
| **位置** | `crates/engine/organ/` (per §4 9 organ process 串联顺序) |
| **真实现** | ✅ 9 organ 全 done (per `organ/src/lib.rs:11-28` 子代理 R1-R8 真移植) |
| **v1 AwakeCompanion 串接** | `organs.rs:96-106` `self.loop_.tick()` = E7 emergence 8 门控 + 调 9 organ (注: v1 AwakeCompanion 不显式串联 9 organ, 只调 EmergenceLoop 单 process, 9 organ 各自独立接受 `context_hint`) |
| **v2 OrganOrchestrator 待做** | ⏳ 实施时 OrganOrchestrator.tick() 按 §4 顺序串 9 organ process, 各 organ output 喂下一 organ |
| **5 重守门** | per §10 (cargo test 0 FAILED + cargo clippy 0 warnings + 13 键 LOCKED + workspace.version + R11 baseline) |

### L4: 主人审批 (governance 3 hook + 7 advisor 加权 + 主人 Veto)

| 项 | 说明 |
|---|---|
| **位置** | `crates/foundation/governance/` (3 hook: PromptInjectionHook + PermissionGovernanceHook + CredentialDisclosureHook, per `v2-architecture-reflection.md:249-251`) |
| **真实现** | ✅ 3 hook + 7 advisor 加权 (per `cognitive-module-wiring.md:97-99`) |
| **v1 AwakeCompanion 串接** | `organs.rs:142-163` SecurityGate.check (哲学锚 × 权限 × HA) |
| **v2 OrganOrchestrator 待做** | ⏳ 实施时 OrganOrchestrator 调 `governance::check()` 走 3 hook + Council 加权 |

### L5: runtime patch (`git tag v2.x+1`)

| 项 | 说明 |
|---|---|
| **位置** | git tag + workspace.version (1.2.0 → 1.2.1 patch) |
| **真实现** | ✅ git tag v2.0.0-rc.1 已拍板 (per `b9026186`) |
| **v2 OrganOrchestrator 待做** | ⏳ 实施时每 cycle 完成 `git tag v2.x+1` (per `v2-architecture-reflection.md:255-261`) |

---

## §4. 9 organ process 串联顺序

per `cognitive-module-wiring.md` §Active slot ledger + 9 organ 真移植 (E4/F4/F6/F1/W1/W2/W3/E7/Memory, per `crates/engine/organ/src/lib.rs:11-28`):

### 4.1 9 organ 串联顺序

| # | Organ ID | 真实现位置 | 行为 | LLM | 备注 |
|---|---|---|---|---|---|
| 1 | **E4 curiosity** | `organ/src/curiosity.rs` | 浅尝辄止 + 疑问路由 | ❌ 0 LLM (deterministic 机制) | 子代理 R1 真写 |
| 2 | **F1 emotion_memory** | `organ/src/emotion_memory.rs` | 主人情绪记录 (PAD 三维 + MoodSource) | ❌ 0 LLM (deterministic 文本启发式) | 子代理 R1 真写 (per `runtime_brain.rs:126-152` 1:1) |
| 3 | **F4 hypothesis** | `organ/src/hypothesis.rs` | 假设闭环 (conjecture/evidence/confirm) | ❌ 0 LLM (deterministic 证据累积) | 子代理 R2 真写 |
| 4 | **F6 value_cases** | `organ/src/value_cases.rs` | 价值内化 | ❌ 0 LLM (deterministic 价值匹配) | 子代理 R3 真写 |
| 5 | **W1 world_model** | `organ/src/world_model.rs` | 文本模拟器 + LLM 反事实推演 | ✅ RC-5 LlmFactory 真接 | 子代理 R4 真写 |
| 6 | **W2 causal_world_model** | `organ/src/causal_world_model.rs` | 因果图 + MCTS + LLM 分支点 | ✅ RC-5 LlmFactory 真接 | 子代理 R5 真写 |
| 7 | **W3 causal_world_model_edges** | `organ/src/causal_world_model_edges.rs` | 边挖掘 + 累计权重 | ❌ 0 LLM (deterministic 权重累加) | 子代理 R6 真写 |
| 8 | **E7 emergence** | `organ/src/emergence.rs` | 5 状态机 + 主动开口 (rhythm+boundary 8 重门控) | ❌ 0 LLM (deterministic 决策路径) | 子代理 R7 真写 |
| 9 | **Memory memory** | `organ/src/memory.rs` | 跨 8 organ 记忆合并抽象 | ❌ 0 LLM (deterministic dedup/weight/persist) | 子代理 R8 真写 |

### 4.2 串联顺序 v1 vs v2 真差异

| 维度 | v1 `AwakeCompanion::tick` (organs.rs:96-106) | v2 OrganOrchestrator 待做 (本 spec) |
|---|---|---|
| **触发方式** | `self.loop_.tick(now, context_hint)` = E7 单 organ 入口 | 按 §4.1 顺序显式串 9 organ |
| **数据流** | E7 emergence 1 organ 输出 → organs.rs 上层处理 | 9 organ output 链式传递: E4 → F1 → F4 → F6 → W1 → W2 → W3 → E7 → Memory |
| **context_hint 喂入** | 仅 E7 emergence 1 organ 收 | 9 organ 全收 (per `OrganInput::context_hint`) |
| **8 重 gate 入口** | E7 emergence.rs 内部 if-else (emergence.rs:460-503) | OrganOrchestrator 上层统一入口 (per §5) |
| **5 状态机驱动** | `self.evolution.current.is_active()` (organs.rs:138) | OrganOrchestrator 显式 transition driver (per §6) |
| **Memory 合并** | 无 (v1 runtime_brain.rs:18-32 仅 3 organ: curiosity + emotion + hypotheses) | OrganOrchestrator 调 Memory organ 末尾合并 8 organ 输出 |

**0 装诚实**: v2 OrganOrchestrator **不是**"v1 AwakeCompanion 1:1 翻译" — 因为 v2 加了 W1/W2/W3/Memory 4 organ (v1 AwakeCompanion 不显式串联, 只调 E7 emergence 单入口). 真实施时按 §4.1 顺序 + `OrganInput` 链式传递.

---

## §5. 8 重 gate 实施路径 (per E7 rhythm+boundary loop 1:1 翻译)

per `legacy/donor/apeireth-companion/src/emergence.rs:460-503` + `presence.rs:410-423` (13 InitiativeGate):

| # | Gate | v1 file:line | InitiativeGate 标签 | v2 OrganOrchestrator 待做 |
|---|---|---|---|---|
| 0 | **user_quiet** | `emergence.rs:460-463` | `UserQuiet` | ⏳ OrganOrchestrator 8 重 gate 第 1 重 |
| 1 | **quiet_hours** | `emergence.rs:464-468` | `QuietHours` | ⏳ 第 2 重 (per `Boundaries.in_quiet_window`) |
| 2 | **daily_limit** | `emergence.rs:469-473` | `DailyLimit` | ⏳ 第 3 重 (per `Boundaries.max_initiatives_per_day`) |
| 3 | **llm_budget** | `emergence.rs:474-484` | `LlmBudget` | ⏳ 第 4 重 (per `LoopConfig.min_llm_interval`) |
| 4 | **min_depth** | `emergence.rs:486-490` | `DepthLow` | ⏳ 第 5 重 (per `Boundaries.min_depth`) |
| 5 | **rhythm_unknown** | `emergence.rs:493-497` | `RhythmUnknown` | ⏳ 第 6 重 (per `rhythm.days == 0`) |
| 6 | **rhythm_veto** | `emergence.rs:499-503` | `RhythmVeto` | ⏳ 第 7 重 (per `rhythm.active_probability < rhythm_veto_probability`) |
| 7 | **drive_low** | `emergence.rs:506+` (drive < drive_threshold) | `DriveLow` | ⏳ 第 8 重 (per `drive < LoopConfig.drive_threshold`) |

**v1 AwakeCompanion 串接 (organs.rs:96-106)**: 8 重 gate 全部在 `self.loop_.tick()` 内部处理, return `None` 时调 `self.loop_.last_hold()` 拿 `InitiativeGate` 留痕.

**v2 OrganOrchestrator 待做**: 把 8 重 gate 提到 OrganOrchestrator.tick() 上层统一入口, 各 gate `if` 分支独立留痕 `InitiativeGate` + 返 `None`. E7 emergence.rs 内部 if-else 仍保留 (per §4.1 1:1 翻译), OrganOrchestrator 是"外层 8 重 gate 统一入口" (类似 v1 `AwakeCompanion::tick` 第 2 步 `self.loop_.tick()` 包装).

**注**: v1 `InitiativeGate` 共 13 种 (emergence 8 + organs 5: emotion_low/council_veto/policy_inactive/gate_block/sovereignty_frozen, per `presence.rs:410-423`). v2 OrganOrchestrator 待做应保留 13 种, 不简化.

---

## §6. 5 状态机 transition 路径 (per E7 PolicyStage + EvolutionState)

### 6.1 状态定义

per v1 `apeireth-evolution/src/state.rs:26-44` + v2 `organ/src/emergence.rs:465-471`:

```rust
// v1 (6 状态, 含 Retired)
pub enum EvolutionState {
    Idle, Draft, Proposed, Ratified, Active, Retired,
}

// v2 (5 状态, 前向声明 per 子代理 R7)
pub enum PolicyStage {
    Idle, Draft, Proposed, Ratified, Active,
}
```

### 6.2 Transition 路径 (per v1 `state.rs:186-197`)

| From | Allowed To | Trigger | v1 file:line | v2 OrganOrchestrator 待做 |
|---|---|---|---|---|
| **Idle** | `Draft`, `Retired` | `TransitionReason::Start` / `L0Guard` | `state.rs:188` | ⏳ OrganOrchestrator 起草新策略 |
| **Draft** | `Proposed`, `Retired` | `TransitionReason::Submit` / `L0Guard` | `state.rs:189` | ⏳ 调 Council 审议 |
| **Proposed** | `Ratified`, `Draft`, `Retired` | `CouncilApprove` / `Revise` / `L0Guard` | `state.rs:190-193` | ⏳ Council 加权表决 |
| **Ratified** | `Active`, `Retired` | `TransitionReason::Activate` / `L0Guard` | `state.rs:195` | ⏳ 主人审批 L4 通过 |
| **Active** | `Retired` | `TransitionReason::Retire` (连续被忽略) | `state.rs:196` + `organs.rs:236-241` | ⏳ `apply_feedback(Ignored)` 触发 |
| **Retired** | (terminal) | — | `state.rs:197` | ⏳ 终态, 需 `with_proposal` 重启 |

### 6.3 v1 `AwakeCompanion::ratify_fresh_policy` (organs.rs:73-84) 真入口

```rust
fn ratify_fresh_policy(evolution: &mut EvolutionStateMachine) {
    *evolution = EvolutionStateMachine::new();
    let at = Utc::now().timestamp_millis();
    let _ = evolution.transition(EvolutionState::Draft, TransitionReason::Start, at);
    let _ = evolution.transition(EvolutionState::Proposed, TransitionReason::Submit, at);
    let _ = evolution.transition(EvolutionState::Ratified, TransitionReason::CouncilApprove, at);
    let _ = evolution.transition(EvolutionState::Active, TransitionReason::Activate, at);
}
```

**v2 OrganOrchestrator 待做**:
1. 启动时调 `ratify_fresh_policy()` = 默认生效 (per v1 `organs.rs:57` 注释"7 强制 advisor 已召集; 主动策略全链路 Idle→Draft→Proposed→Ratified→Active (默认生效)")
2. `apply_feedback(Ignored)` 连续 N 次 → `Retired` (per `organs.rs:235-241`)
3. `observe_interaction()` 关系重新活跃 → `ratify_fresh_policy()` 重新批准 (per `organs.rs:247-262`)

---

## §7. L0-L5 自升级 cycle 集成 (per `v2-architecture-reflection.md` §6)

per 主代理 Mavis 设计 (子代理 R11 整合 v1 AwakeCompanion 真路径):

### 7.1 L0 人类审批 (硬墙, 永远不可变)
- **v1 AwakeCompanion**: `organs.rs:142-157` SecurityGate.check 哲学锚拦截 → `sovereignty.report_violation` 熔断证据
- **v2 OrganOrchestrator 待做**: 实施时 OrganOrchestrator.tick() 第 6 步调 SecurityGate.check + SovereigntyGate.report_violation

### 7.2 L1 自我诊断 (cognitive self_assessment)
- **v1 AwakeCompanion**: ❌ 无 (v1 无 self_assessment, v2 RC-4 新增)
- **v2 OrganOrchestrator 待做**: 实施时 OrganOrchestrator 调 `cognitive.self_assessment` 喂 5 状态机 Idle→Draft 触发

### 7.3 L2 提案生成 (Orchestrator + 7 LlmAdvisor Council)
- **v1 AwakeCompanion**: `organs.rs:116-135` Council.deliberate (deterministic 7 advisor, v1 无 LLM)
- **v2 OrganOrchestrator 待做**: 实施时调 `Arc<dyn CouncilInvoker>::deliberate` (per `cognitive.rs:14-17` 真接 LLM, 60s timeout per `cognitive-module-wiring.md:99`)

### 7.4 L3 验证 (9 organ process 串联 + sandbox regression)
- **v1 AwakeCompanion**: `organs.rs:96-106` `self.loop_.tick()` = E7 单 organ 入口
- **v2 OrganOrchestrator 待做**: 实施时按 §4.1 顺序串 9 organ process, 5 重守门跑通 (per §10)

### 7.5 L4 主人审批 (governance 3 hook + 7 advisor 加权 + 主人 Veto)
- **v1 AwakeCompanion**: `organs.rs:142-163` SecurityGate.check (哲学 × 权限 × HA)
- **v2 OrganOrchestrator 待做**: 实施时调 `governance::check()` 走 3 hook + Council 加权 + 主人 Veto

### 7.6 L5 runtime patch (`git tag v2.x+1`)
- **v1 AwakeCompanion**: ❌ 无 (v1 era 86-crate monolith, 无 orchestrator 概念)
- **v2 OrganOrchestrator 待做**: 实施时每 cycle 完成 `git tag v2.x+1` (per `v2-architecture-reflection.md:255-261`)

---

## §8. 0 装诚实真账 (子代理 R11 独立判断)

### 8.1 现状盘点

| 项 | 状态 | file:line |
|---|---|---|
| **v1 AwakeCompanion 真实现** | ✅ 已 done (per R7 风险 #1 标) | `legacy/donor/apeireth-companion/src/organs.rs:34-275` (391 行) |
| **v1 8 重 gate 真实现** | ✅ 已 done | `legacy/donor/apeireth-companion/src/emergence.rs:460-503` + `presence.rs:410-423` (13 种 InitiativeGate) |
| **v1 5 状态机真实现** | ✅ 已 done | `legacy/donor/apeireth-evolution/src/state.rs:26-197` (6 状态含 Retired) |
| **v2 9 organ 真移植** | ✅ 全 done (per `organ/src/lib.rs:11-28`) | 子代理 R1-R8 1:1 翻译 |
| **v2 12 slot ledger** | ✅ done (per `cognitive-module-wiring.md`) | 6 WIRED + 1 SLOT READY + 6 DEFERRED |
| **v2 OrganOrchestrator 类似 AwakeCompanion** | ❌ **缺** (9 organ 是 9 个独立 trait impl) | 本 spec 完成后真实施 1-3 周待 |

### 8.2 0 装诱导 prevention (子代理 R11 独立判断)

**不假装"全做完"**:
- 本 spec **只写文档**, **不真做 OrganOrchestrator impl**
- 不假装"v2 E7 emergence 自带 5 状态机" — 真相: `PolicyStage` 是前向声明 (per `emergence.rs:465`), `policy_stage()` 永远返 `Active` 占位 (per `emergence.rs:856`)
- 不假装"9 organ 已串联" — 真相: 9 organ 是 9 个独立 `Arc<dyn OrganTrait>`, runtime 拿 9 个 handle 但无 process 串联路径
- 不假装"5 重守门跑通 OrganOrchestrator" — 真相: OrganOrchestrator 0 实现, 跑的是 v2 cognitive module ledger + 9 organ 独立测试

### 8.3 真生产前阻塞

| 阻塞 | 估时 | 来源 |
|---|---|---|
| **#6 OrganOrchestrator 类似 AwakeCompanion** | **1-3 周** | 子代理 L 估 + 本文 §8.4 真实施估 |
| **#11 frontend 对接** | 4-6 周 | 估 2027-Q1 启动 (per §11) |
| **6 DEFERRED slot 激活** (preference_learning / reflection / critic / planner / orchestrator / perception) | 各 2-3 周 | per `cognitive-module-wiring.md:30-35` |

### 8.4 OrganOrchestrator 真实施估 (子代理 R11 估)

| 子任务 | 估时 | 备注 |
|---|---|---|
| struct `OrganOrchestrator` + 9 organ handle + 5 状态机 | 2-3 天 | per §4.1 顺序 + §6.2 transition |
| `tick()` 串联 9 organ process + 8 重 gate | 3-5 天 | per §5 + §4.1 |
| `apply_feedback()` + `observe_interaction()` | 1-2 天 | per `organs.rs:194-262` 1:1 |
| `tone()` 三层合成 (关系×情绪×审议) | 1 天 | per `organs.rs:185-191` + `tone.rs` |
| 13 种 InitiativeGate 留痕 + presence 集成 | 1-2 天 | per `presence.rs:410-423` |
| L0-L5 自升级 cycle 集成 | 2-3 天 | per §7 + `v2-architecture-reflection.md` §6 |
| 5 重守门 + clippy 0 / tests 0 | 1-2 天 | per §10 |
| 文档 + handoff | 1 天 | — |
| **总计** | **1-3 周** | per 子代理 L 估 |

---

## §9. 0 触碰 LOCKED (5 项严守)

子代理 R11 baseline **0 改**:

| LOCKED 项 | 状态 | 验证 |
|---|---|---|
| **5 项 LOCKED** | ✅ 0 触碰 | per `10-locked.md` + `philosophy.md` (9 锚) |
| **8 哲学锚本体** | ✅ 0 触碰 | per `philosophy.md` + O-6 子代理 K |
| **13 键** | ✅ 0 触碰 | per `governance` 13 键 verdict cache |
| **workspace.version = "1.2.0"** | ✅ 0 触碰 | per `Cargo.toml:44` |
| **R11 baseline** | ✅ 0 触碰 | `cognitive.rs` 12 slot 0 改 + Cargo.lock 0 行 diff (本文仅文档) |

**本 spec 仅文档**: 1 个新文件 `docs/01-architecture/organ-orchestrator-spec.md` + 0 改 Rust 代码 + 0 引新 dep + 0 改 Cargo.toml + 0 改 Cargo.lock.

---

## §10. 接手人 actionable (per 子代理 D handoff, 5/5 done + #6 待)

per `FINAL-HANDOFF-V2.0.0-RC.1.md` §10 接手人 actionable:

| # | 项 | 状态 | 备注 |
|---|---|---|---|
| #1 | RC-5/6/7 + 9 organ 真移植 | ✅ done | 子代理 R1-R8 + M/N 真写 |
| #2 | 哲学锚 ledger 待核 | ✅ done | 子代理 K |
| #3 | 12 consumer 弃用迁移 | ✅ done | 子代理 I Python script |
| #4 | RC-10 line header AAD + APX2 envelope | ✅ done | 子代理 E |
| #5 | cognitive module 不变量 + 9 organ trait 抽象边界 | ✅ done | 子代理 J + 12 slot ledger |
| **#6** | **OrganOrchestrator 类似 AwakeCompanion** | ⏳ **本 spec 完成 + 真实施 1-3 周待** | **本文** |

**5 重守门** (per `v2-architecture-reflection.md:289-291`):
1. `cargo test --workspace --locked 2>&1 | tail -3` → workspace 0 FAILED (本批 0 改 Rust 代码, 应维持 0 FAILED)
2. `cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -3` → 0 warnings (本批 0 改 Rust 代码, 应维持 0)
3. 13 键 LOCKED (per governance) → 0 改
4. workspace.version = "1.2.0" (per `Cargo.toml:44`) → 0 改
5. R11 baseline (per `cognitive-module-wiring.md`) → 0 改

---

## §11. 真生产前阻塞 #2: frontend 对接 (4-6 周, 估 2027-Q1 启动)

per 子代理 R9 spec (待写, frontend gateway OpenAI Chat 兼容契约 + 9 organ 串联路径) + 子代理 R10 spec (待写, cognitive 9 organ 集成 spec: 12 slot 注入 + 6 WIRED + 1 SLOT READY + 6 DEFERRED):

| 阻塞 | 估时 | 备注 |
|---|---|---|
| **R9 frontend gateway OpenAI Chat 兼容契约** | 2-3 周 | 子代理 R9 待写 spec |
| **R10 cognitive 9 organ 集成 spec** | 1-2 周 | 子代理 R10 待写 spec |
| **R11 OrganOrchestrator 类似 AwakeCompanion** | 1-3 周 | **本 spec 完成** |
| **frontend 实施** (Tauri/CLI/Web 三端) | 4-6 周 | per 子代理 L 估, 估 2027-Q1 启动 |

**3 spec 协作**:
- R9 = frontend 入口 (OpenAI Chat 兼容契约)
- R10 = cognitive 集成 (12 slot ledger)
- R11 = orchestrator 串联 (本文, 8 重 gate + 5 状态机 + 9 organ process)

---

## §12. 必跑命令结果

```text
$ git log -1 --oneline
b9026186 chore(release): v2.0.0-rc.1 release tag 拍板 (9 organ 全 done + 5 actionable 全 done + 0 装诚实真账, 子代理 Z 独立审计触发主代理亲做 4 文档修正)

$ git tag --list
v1.0.0
v1.5.0
v2.0.0-alpha.1
v2.0.0-rc.1

$ cargo test --workspace --locked 2>&1 | tail -3
[子代理 R11 待跑 — 本批 0 改 Rust 代码, 应维持 0 FAILED]

$ cargo clippy --workspace --all-targets --locked -- -D warnings 2>&1 | tail -3
[子代理 R11 待跑 — 本批 0 改 Rust 代码, 应维持 0 warnings]

$ git diff HEAD~1..HEAD --stat 2>$null | head -10
[本批新增 1 文档: docs/01-architecture/organ-orchestrator-spec.md, 0 改 Rust 代码]
```

---

## §13. 风险 (2 条)

1. **OrganOrchestrator 真实施 1-3 周未做** — 本 spec **只写文档**, 真实施待 v2.0.0 release 后启动 (per §10 #6 + §8.3). 接手人需派子代理 R12+ 真写 `crates/engine/organ_orchestrator/src/lib.rs` + 5 状态机 driver + 13 InitiativeGate 留痕.
2. **6 DEFERRED slot 激活** (`preference_learning` / `reflection` / `critic` / `planner` / `orchestrator` / `perception`, per `cognitive-module-wiring.md:30-35`) — 各估 2-3 周, 估 12-18 周 (3-4.5 月), 真生产前需分批激活.

---

## §14. 建议 (2 条, 接手人后续真实施)

1. **接手人读本文后**: 派 1 子代理真写 `crates/engine/organ_orchestrator/src/lib.rs` (1-3 周), 按 §4.1 9 organ 串联顺序 + §5 8 重 gate + §6 5 状态机 transition. 0 触碰 LOCKED + 5 重守门跑通.
2. **后续 v2.x release**: 6 DEFERRED slot 激活按 `cognitive-module-wiring.md:30-35` 顺序, 各派 1 子代理真写 (估 2-3 周/项, 总 12-18 周), 真生产前完成.

---

## §15. 独立判断 (子代理 R11 第 31 视角)

**看到 R7 + R9 + R10 没看的事** (子代理 R11 独立视角, 前 28 sub-agent A-Z 都没写 OrganOrchestrator spec, 我是第 31 个视角):

1. **v1 AwakeCompanion::tick 串联顺序 ≠ v2 9 organ process 串联顺序**: v1 AwakeCompanion 只显式调 E7 emergence 单 organ 入口 (`organs.rs:96`), v2 OrganOrchestrator 应按 §4.1 顺序串 9 organ. **0 装诚实**: 实施时不要"v1 1:1 翻译" — 因为 v2 加了 W1/W2/W3/Memory 4 organ (v1 没有).
2. **5 状态机不在 E7 emergence.rs 内部**: 真相在 v1 `apeireth-evolution::EvolutionState` (6 状态含 Retired, per `state.rs:26-44`). 任务说明把"5 状态机"挂 E7 头上是误导 (子代理 R7 已独立判断). v2 `PolicyStage` 是前向声明 (`emergence.rs:465`), 真实施时需 import `apeireth-evolution::EvolutionState` (或在新 `crates/foundation/policy/` 抽象).
3. **13 种 InitiativeGate 真实门控**: emergence 8 + organs 5 = 13 种 (`presence.rs:410-423`). v2 OrganOrchestrator 实施时应保留全部 13 种, 0 装简化.
4. **L0 人类审批不可变**: 即使 OrganOrchestrator 实施, L0 哲学锚拦截 (per `organs.rs:152-157`) 仍走 `SovereigntyGate::report_violation` 物理隔离. OrganOrchestrator 不是"绕过 L0 的捷径", 是"在 L0 锚定下的 L1-L5 升级能力".
5. **0 装诚实**: 本 spec 不假装"全做完". 真实施 1-3 周估 + 6 DEFERRED slot 激活 12-18 周估 = 总 13-21 周 (3-5 月), v2.0.0 release 后启动.

---

_本文档 v1 首发 (2026-08-28, 子代理 R11 写于 v2.0.0-rc.1+ 真生产前阻塞 spec 任务). 真实施待主代理后续派子代理 R12+. 下次更新预计在 v2.0.0 release 后 (估 2027-Q1) OrganOrchestrator 真写完后回填._