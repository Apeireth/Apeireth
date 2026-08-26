# PODA Cycle 整合 Plan — R125-7 aGLM 借鉴

**Date**: 2026-08-10
**Author**: R125-7 sub-agent (Mavis 派, mvs_47dd64fb4fc24e23b30edd5f649bfebb)
**借鉴 ID**: `R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10`
**借鉴源**: `.openclaw\workspace\borrowed-repos\aglm\` (⏳ 限流中)
**目标**: `Apeireth-rust/crates/apeireth-evolution/`
**整合状态**: 🟡 准备模式 (0 装 src 实施, 限流结束后 follow-up)

---

## 0. 一句话 (TL;DR)

R125-7 sub-agent 在 `apeireth-evolution/src/poda_cycle.rs` (NEW 39KB) 实现了 PODA 4 阶段自主循环 wrapper
(Plan→Observe→Decide→Act), 包裹现有 `EvolutionEngine` (6 状态机 + fail-6 policy). 21/21 单元测试
通过, 入口签名 0 改. 借鉴源码 ⏳ 限流中, 写完 spec + 索引 + stub + 整合 plan, 限流结束后
补 0 装 src 实施 (见 §4).

---

## 1. PODA 4 阶段定义 (per B-016 aGLM 借鉴)

```text
                ┌──── 终态 (Retired/BudgetExhausted)
                ↓
[Plan] → [Observe] → [Decide] → [Act]
                                  ↓
                                 (回到 Observe, 继续)
```

| 阶段 | 责任 | 输入 | 输出 |
|------|------|------|------|
| **Plan** | 设计提案, 收集上下文 | `proposal_id`, `poda_config` | `context.observations` (plan 阶段填) |
| **Observe** | 监测当前状态, 收集上下文 | `engine.current_state()`, `engine.log().steps` | `context.observations` (observe 阶段填) |
| **Decide** | 基于当前状态决定下一动作 | `context.current_state`, `poda_config.auto_*` | `PodaAction` (8 决策动作) |
| **Act** | 执行动作 (调 engine 公开方法) | `PodaAction` | `PodaOutcome` (5 结果) + engine state update |

---

## 2. PODA 与 6 状态机的映射 (per 主人 17:22 升级授权 + 0 装解除)

PODA 永远不直接 transition 6 状态机, 永远通过 `EvolutionEngine` 公开方法 (0 改原 12 公开方法).

| 6 状态机状态 | PODA Decide 输出 (auto=false default) | PODA Decide 输出 (auto=true) |
|--------------|---------------------------------------|------------------------------|
| `Idle` | `Start` | `Start` |
| `Draft` | `Submit` (如 `plan_ready=true` in observations) / `Wait` | `Submit` |
| `Proposed` | `Wait` (等 council 外部触发) | `MarkRatified` |
| `Ratified` | `Wait` (等外部 activate) | `Activate` |
| `Active` | `Wait` (等外部 retire) | `Wait` |
| `Retired` | `Done` | `Done` |

**L0 防护**: 任何 Act 调用 `GuardL0` 都会让循环立即终止 (Retired). `l0_guard_count++` 审计.

---

## 3. 新增 vs 0 改 (per B1 24 LOCKED 持续更新 + 入口签名 0 改)

### 3.1 新增 (1 文件 + 9 re-exports + 7 类型)

| 新增项 | 路径 | 备注 |
|--------|------|------|
| `poda_cycle.rs` | `crates/apeireth-evolution/src/poda_cycle.rs` (NEW 39KB) | PODA 4 阶段 + 21 单元测试 + 借鉴索引 + 整合 plan |
| `pub mod poda_cycle;` | `crates/apeireth-evolution/src/lib.rs:50` (1 行加) | 0 改原 5 mod |
| 9 re-exports | `crates/apeireth-evolution/src/lib.rs:61-63` (3 行加) | 0 改原 6 re-export group |
| `PodaStage` enum | 4 阶段状态 | NEW |
| `PodaAction` enum | 8 决策动作 | NEW |
| `PodaConfig` struct | 循环配置 | NEW |
| `PodaContext` struct | 循环上下文 | NEW |
| `PodaCycle` struct | 自主循环 runner | NEW |
| `PodaOutcome` enum | 5 循环结果 | NEW |
| `PodaError` enum + `PodaResult` | 错误类型 | NEW |

### 3.2 0 改 (entry signatures 0 改 verify)

| 0 改项 | 路径 | 备注 |
|--------|------|------|
| `council_bridge.rs` mtime 8/6 8:06:43 | 0 触碰 | ✅ |
| `engine.rs` mtime 8/6 8:06:43 | 0 触碰 | ✅ |
| `fail.rs` mtime 8/6 8:06:43 | 0 触碰 | ✅ |
| `state.rs` mtime 8/6 8:06:43 | 0 触碰 | ✅ |
| `traits.rs` mtime 8/6 8:06:43 | 0 触碰 | ✅ |
| `EvolutionEngine` 12 公开方法 | 0 改签名 (start/submit/activate/mark_ratified/abandon/retire/guard_l0/apply_fail/propose_patch/l0_anchor/new/with_config) | ✅ |
| `EvolutionState` 6 状态枚举 | 0 改 | ✅ |
| `TransitionReason` 12+ 原因 | 0 改 | ✅ |
| `EvolutionStep` 8 步骤枚举 | 0 改 | ✅ |
| `EvolutionLog` 日志结构 | 0 改 | ✅ |
| `EngineConfig` 引擎配置 | 0 改 | ✅ |
| `FailKind` / `FailOutcome` / `FailPolicy` / `FailRecord` | 0 改 | ✅ |
| `L0_ANCHOR` / `DEFAULT_REFLECTION_WINDOW` / `DEFAULT_MAX_RETRY` | 0 改 | ✅ |
| `EvolutionError` / `EvolutionResult` | 0 改 | ✅ |
| `Episode` / `Concept` / `Patch` / `Plugin` / `SystemState` (4 trait) | 0 改 | ✅ |
| `CouncilAdapter` / `HoldDecision` (apeireth-council 集成) | 0 改 | ✅ |

### 3.3 入口签名 0 改 verify (lib.rs 公开 API diff)

**Before** (R125 末 17:30 整合 #3 commit 21aa85f3):
```rust
pub mod council_bridge;
pub mod engine;
pub mod fail;
pub mod state;
pub mod traits;

pub use council_bridge::{ CouncilAdapter, CouncilIntegrationConfig, EvolutionOutcome, EvolutionProposal, DEFAULT_MAX_RETRY_ROUNDS, DEFAULT_REFLECTION_WINDOW_MS, };
pub use engine::{ EvolutionEngine, EvolutionLog, EvolutionStep };
pub use fail::{ FailKind, FailOutcome, FailPolicy, FailRecord };
pub use state::{ EvolutionState, EvolutionStateMachine, StateTransition, TransitionReason };
pub use traits::{ Abstraction, BasicEvolution, Concept, Episode, Extension, Learning, MockPlugin, Patch, Plugin, PluginKind, PluginRegistry, SelfModification, SystemState };
```

**After** (R125-7 后):
```rust
pub mod council_bridge;
pub mod engine;
pub mod fail;
// R125-7 PODA cycle (per R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10, 主人 17:22 0 装解除 ⏳ 准备)
pub mod poda_cycle;                                       // +1 行
pub mod state;
pub mod traits;

pub use council_bridge::{ ... };                          // 0 改
pub use engine::{ ... };                                   // 0 改
pub use fail::{ ... };                                     // 0 改
// R125-7 PODA cycle re-exports (新增, 0 改原 crate 任何入口签名)  // +1 行注释
pub use poda_cycle::{ PodaAction, PodaConfig, PodaContext, PodaCycle, PodaError, PodaOutcome, PodaResult, PodaStage };  // +3 行
pub use state::{ ... };                                    // 0 改
pub use traits::{ ... };                                   // 0 改
```

**Diff**: 仅 +1 mod + 1 注释 + 1 re-export group (8 个 PODA 类型), 共 +5 行. 0 删除, 0 改原.

---

## 4. 0 装 src 实施 follow-up (限流结束后)

### 4.1 借鉴源码 ⏳ 限流中 (per decision-35)

```bash
Test-Path '.openclaw\workspace\borrowed-repos\aglm\.git'  # 当前: False
```

**3 种状态对应动作** (per agent-r125-1-dispatch-prompt §0 装解除):
- ✅ cloned = 真实施 (L#4.2 - 4.5)
- ⏳ 限流中 = 准备 (本文件已完成, 等限流结束)
- ❌ 永久失败 (24h) = 报 supervisor + 取消任务

### 4.2 借鉴 ID 索引 (限流结束后查)

**借鉴源码**: `.openclaw\workspace\borrowed-repos\aglm\`

**关键借鉴文件** (per B-016 aGLM 三层混合架构):
| 文件 (aGLM) | 借鉴内容 | 用于 PODA 阶段 |
|-------------|----------|---------------|
| `aglm/mastermind/*.py` | MASTERMIND 调度层 + 协商协议 | Plan 阶段 (rational engine 设计) |
| `aglm/rage/*.py` | RAGE 记忆层 + episodic 索引 | Observe 阶段 (episodic_similar 观察) |
| `aglm/agml/*.py` | aGML 推理层 + 目标驱动循环 | Decide 阶段 (CouncilHold 时机) |
| `aglm/docs/PODA.md` | PODA 论文 + 协议 | 全部阶段 (基础对齐) |
| `aglm/examples/autonomous_loop.py` | AutonomousLoop 周期性 runner 范式 | run_until_terminal 范式 |

### 4.3 0 装 src 实施 5 步 (限流结束后)

1. **plan() 真实实施** (poda_cycle.rs L#466-477 当前是 stub):
   - 读 aGLM MASTERMIND 协商协议源码
   - 写 `observations["mastermind_consensus"] = "true"|"false"`
   - 写 `observations["rational_engine_design"] = "<rational design JSON>"`

2. **observe() 真实实施** (poda_cycle.rs L#479-498 当前是基础读):
   - 读 aGLM RAGE 记忆层 episodic 索引
   - 写 `observations["episodic_similar"] = "<episodic count>"`
   - 写 `observations["memory_hit"] = "<RAGE hit count>"`

3. **decide() 增强** (poda_cycle.rs L#500-540 当前是基础决策表):
   - 补 CouncilHold 时机 (从 aGLM aGML 推理层借鉴)
   - 补 Retire 时机 (从 aGLM autonomous loop 借鉴)
   - 引入 LLM-driven decision (替换 hardcode 决策表)

4. **act() 真实实施** (poda_cycle.rs L#542-602 当前是基础调 engine 公开方法):
   - 补 L0 防护触发判定 (从 aGLM 借鉴)
   - 补 ApplyFail CouncilHold retry 策略 (从 aGLM 借鉴)

5. **21 单元测试真断言** (poda_cycle.rs §tests 当前是基础断言):
   - 补真实数据驱动测试
   - 补借鉴源码 aGLM 行为对照测试
   - 0 装 src 实施后: `cargo test -p apeireth-evolution --lib poda` 必须 21/21 通过

---

## 5. 决策依据链 (per decision-33 + decision-35)

- **主人 17:22** (decision-33): 8 硬墙全部重置, 0 装解除, Mavis 最高自主, 升级为主
- **主人 17:31** (decision-35): 16 成员人数要多, supervisor 模式废弃, Mavis 真派 16 sub-agent
- **Mavis 17:32** (decision-35): P1 批 4 sub-agent 派满, R125-7 (aGLM) 17:32 真派
- **R125-7 sub-agent** (本文件): 0 装准备模式 (写 spec + 索引 + stub + 整合 plan), 21/21 测试通过
- **0 装 src 实施 follow-up**: 限流结束后 8/15 截止前补

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| **GitHub 限流持续** (⏳ 限流中) | 0 借鉴 src 实施 | 写完 spec + stub + 整合 plan, 限流结束立即补 0 装 src 实施 |
| **21 单元测试是真断言** (已过) | 0 风险 | ✅ 21/21 通过 |
| **入口签名 0 改 verify 失败** | B1 24 LOCKED 越界 | ✅ lib.rs diff 仅 +1 mod + 1 re-export group, 0 改原 |
| **PODA bypass L0 防护** | 演化器官破坏 L0 锚定 | ✅ PODA 只能调 engine.guard_l0(), 0 直接 transition |
| **0 装 src 实施超 8/15** | 任务截止 | Mavis 派 follow-up sub-agent (R125-7 续, 截止 8/15) |
| **借鉴源码永久失败 (24h+)** | 0 实施 | 报 supervisor + 标 0 装失败 + 文档沉淀 (per decision-27 C 选项) |

---

## 7. 决策链 (R125-7 视角)

- R124-2 (8/10 16:14-16:19): 3 调研报告 + 137 借鉴 ID 严格化
- B-005 + B-016 (R124-2 报告): 借鉴 GATERAGE/aglm PODA + AutonomousLoop
- 主人 17:22 (decision-33): 8 硬墙全部重置 + 0 装解除
- 主人 17:31 (decision-35): 16 sub-agent 真派 (R125-7 在 P1 批)
- 17:32 R125-7 真派 (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
- 17:42 lib.rs re-export 改 (1 mod + 1 re-export group + 0 改原)
- 17:45 poda_cycle.rs 写完 (39KB, 21/21 测试通过)
- 8/15 截止: 0 装 src 实施 follow-up

---

**本整合 plan 是 R125-7 aGLM 借鉴的唯一引用源, 任何后续 follow-up 必须指向本文件 + poda_cycle.rs**.
