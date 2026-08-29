# Apeireth v2.0 完成计划 — 9 哲学锚 + S-2 实事求是真账 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 14, 用户原话 "把文档先更新了, 注意实事求是, 注意哲学锚, 然后给我一个你下一步的想法, 要怎么完成 2.0" 触发)
> **用途**: v2.0 release 完成计划真账, 按 9 哲学锚 + 真账修订 (12-14 周 critical path + ~35 项 1.0 缺口 + 0 装诚实标), 给主代理 + 接手工程师执行计划
> **关系**: 修订 ROADMAP §7 + MANIFESTO §14 (Round 14 已 push `3ea454f1`) + `v2-reference-handbook-2026-08-28.md` + `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` + `round-13-1-0-maturity-audit-2026-08-28.md` + 22 真账 doc (Round 9-14 累计)

```
[Document-Meta]
Document:        docs/04-internal/round-14-v2-completion-plan-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 14, 用户原话 '怎么完成 2.0' 触发)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (v2.0 release 完成计划真账)
Author:          主代理 Mavis
```

---

## 0. 用户原话触发 + 主代理自省

**用户原话**: "OK 把文档先更新了, 注意实事求是, 注意哲学锚, 然后给我一个你下一步的想法, 要怎么完成 2.0"

**主代理自省 (per O-5 + S-2 实事求是 + 9 哲学锚)**:
- ✅ 文档已更新 (Round 14 commit `3ea454f1` push 成功, ROADMAP §7 + MANIFESTO §14 + 主代理真账 §6.3 §3.1 修订)
- ✅ 实事求是: 修订 release 估时 4-6 月 → **6-9 月** (因 ~35 项 1.0 缺口 + Round 13 1.0 maturity 补查), 不假装 4-6 月能完成
- ✅ 哲学锚: 9 哲学锚 LOCKED (S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装 / O-6 永远追求最优), 0 触碰
- ⏳ 下一步: 真实施 critical path 12-14 周 (per 真账), release 估 2027-Q3

---

## 1. v2.0 真实施完成计划 — 按 9 哲学锚

### 1.1 S-1 北极星 (Everything serves the ASI north star 五原型)

**真账**: v2.0 release = 1.0 功能全集 + 架构升级, 不是 "框架" 是 **AI 物种实现** (per `apeireth-true-understanding-2026-08-28.md`):
- **基地** (LLM 操作系统, 16 crates) ✅ done
- **Agent 平台** (OrganOrchestrator + 12 cognitive slot) ✅ A 块 done
- **她** (物种实现, per-user 塑形) — 真实施 (28 项 1:1 可移植 + 4 项 trait 口 + 5 项 PARTIAL, ~35 项 1.0 缺口)

**北极星兑现路径**:
1. 完成 28 项 1:1 可移植真实施 (per R11 真账)
2. 主代理亲做 4 项 trait 口实接线 spec
3. 真实施 5 项 PARTIAL (education 真 CAS + confidence BetaBinomial + HybridCognitiveRouter 真接 + proactive 真接 + Kani bridge)
4. R20/R22 真实施 (跟认知模块 critical path 并行)

### 1.2 S-2 实事求是 (Verify before writing; truth over narrative)

**真账 (修订估时)**: 
- 原估 4-6 月 release ❌ 偏乐观 (per Round 13 catch)
- 修订估 **6-9 月 release (2027-Q3)**, 真实施 critical path **12-14 周**
- 修订原因: 1.0 vs 2.0 功能全集对比发现 ~35 项缺口 (主代理真账 §2.4), Round 13 1.0 maturity 补查 8 个核心 .rs 确认 maturity
- 修订不留 O-5 失守: 主代理真账 §3.1 估 3-4 周 ❌ → 实际 12-14 周 critical path (修订 +2 周主代理亲做 spec)

**实事求是路径**:
1. **不假装** 1.0 全部完整 (Round 13 maturity 补查发现 5 项 PARTIAL + 4 项 trait 口 + 28 项 REAL)
2. **不假装** 调研 100% 完成 (R11 sub-agent + 主代理都 0 实测 1.0 .rs, **本地 working tree 已就位 86-crate v1 + 16 crates v2 真账** 0 git clone 必要, 仅真账 + 推断)
3. **不假装** release 时间 (4-6 月 → 6-9 月修订, 因 ~35 项 1.0 缺口 + 1.0 maturity 补查)

### 1.3 S-3 质量工程化 (Engineering rigor = 1739 tests / 0 clippy / clippy 0 / LOCKED 0)

**真账 (5 重守门 baseline 维持)**:
| 守门 | 当前实测 | 真实施时要求 |
|---|---|---|
| clippy 0 warning | ✅ | 1 真实施 1 测 (per §13) |
| tests 0 fail (1739 passed) | ✅ | 1 真实施 1 测 (per §13) |
| legacy compat path < 100 (36) | ✅ | 1 真实施 1 测 (per §13) |
| LOCKED 5 项 0 触碰 | ✅ | 1 真实施 1 测 (per §13, 走扩展 trait 接口) |
| 9 哲学锚表头 0 减 | ✅ | 1 真实施 1 测 (per §13) |

**质量工程化路径**:
1. 5 重守门 baseline 维持 (前 baseline 已 OK, 真实施时必保 5 重 0 失误)
2. o6-anchor.yml workflow (`.github/workflows/o6-anchor.yml` 166 行) 自动跑 5 重 (CI 必含)
3. §8.5 pre-commit + commit-msg hook 强制 commit msg 含 O-6 三段审查关键词
4. 每真实施 1 commit message 必带 4 项标 + O-6 三阶审查 + 拒 alternatives + 拒理由

### 1.4 O-1 安全优先 (Safety > function > performance, 9 重 v9 守门 + 13 键 verdict cache + 3 项不可变脊柱)

**真账 (LOCKED 5 项 0 触碰)**:
| LOCKED 项 | 位置 | 当前状态 | 真实施时要求 |
|---|---|---|---|
| 9 哲学锚本体 | `crates/foundation/core/src/eight_anchors.rs:58-79` | ✅ 0 触碰 | 走扩展 trait 接口 |
| 13 键 verdict cache | `crates/foundation/core/src/philosophy.rs:142` `RUNTIME_ENFORCED = false` | ✅ 0 触碰 | 走扩展 trait 接口 |
| 3 项不可变脊柱 | `crates/foundation/core/src/onion.rs:249` | ✅ 0 触碰 | 走扩展 trait 接口 |
| workspace.version | `Cargo.toml:44` "1.2.0" | ✅ 0 改 | 走 `version.workspace = true` |
| R11 baseline 3 值 (0.8682/0.8532/0.9063) | legacy reference | ✅ 0 触碰 | 走扩展 trait 接口 |

**安全优先路径**:
1. 真实施 11 项派单, **走扩展 trait 接口** (不破现有 9 organ trait + 12 cognitive slot wiring + LOCKED 5 项)
2. P0 governance 3 hook 已装 (`PermissionGovernanceHook` + `CredentialDisclosureHook` + `PromptInjectionHook`)
3. v1 真实施 unsafe code (per master audit, 仅 1 fn 需 unsafe) — 真实施时 0 unsafe code 优先

### 1.5 O-2 前人肩上 (Borrow, attribute, adapt)

**真账 (借鉴链完整)**:
- ✅ **5 R7 物种化借鉴真账** (N.E.K.O / Open-LLM-VTuber / Firefly / Mio / AIRI) push 成功
- ✅ **R11 6 gap 真调研真账** (Storage / LongTermMemory / SpeciesCore / SpeciesForm / MetaCognition / CoordinationContext) push 成功
- ✅ **research/source ~36 真开源借鉴** (tokio / wasmtime / qdrant / sled / hermes-agent-rs / MetaGPT / openclaw / LangGraph / CrewAI / Claude Code 等)
- ✅ **legacy/donor/~100 modules 1.0 真账** (12 slot + 9 organ 1:1 翻译源)
- ✅ **_research_mem/** (apeireth-rust-fork + AgentFlow + sub_agent_reports + wave2-wave7 真账)

**前人肩上路径**:
1. 真实施时 1.0 1:1 翻译优先 (28 项 1:1 可移植)
2. 1.0 PARTIAL 真账借鉴 (5 项: education 字符串规则 / confidence BetaBinomial / HybridCognitiveRouter / proactive / Kani bridge)
3. R7 物种化借鉴 (5 项目对接: N.E.K.O 五维记忆 / Open-LLM-VTuber 4 段 pipeline / Firefly GPT-SoVITS 原声 TTS / Mio Windows 本地优先 / AIRI 永远不下播)
4. research/source 真开源借鉴 (~36 项目, 真实施时按需 clone)
5. 0 新外部 dep 引入 (per 真账 brief 约束)

### 1.6 O-3 干到底 (Finish what we start; no half-measures)

**真账 (真实施 critical path 不"等以后做")**:
- ✅ A 块 OrganOrchestrator 完整化 5 stage 真实施 (c003e078 / 087ab2ac / 50ba2e57 / 29e5ce66 / 0afa733f) + O-6 三阶审查 amend 复盘 (bbbfb75b) + A 块同步真账 (1d885299)
- ✅ ENGINEER-MANIFESTO.md 14 章 + §13 12 真实陷阱
- ✅ 22 真账 doc (~5700 行调研)
- ✅ Round 11 真调研 5/6 R11 + Round 12 终极审计 + Round 13 1.0 maturity 补查 + Round 14 release 修订

**干到底路径**:
1. 派 R12 真实施 11 项 (Round 13 §3.3), 不"等以后做" (O-6 锚 #6 doctrine: "工作量与麻烦不是拒绝重做的理由")
2. 真实施 critical path 12-14 周不延期 (per 真账)
3. release 2027-Q3 不延期 (修订估时)

### 1.7 O-4 任何人都能接手 (Any newcomer can onboard from docs alone)

**真账 (22 真账 doc + 1 handbook + 1 handoff log)**:
- ✅ `v2-reference-handbook-2026-08-28.md` (613 行, 一站式 reference 13 节)
- ✅ `handoff-log-2026-08-28-mavis.md` (124 行)
- ✅ `TO-NEW-TEAM.md` (Round 9 已就位, 接手入口)
- ✅ `sub-agent-audit-round-4-2026-08-28.md` (201 行)
- ✅ `round-8-verifications-2026-08-28.md` (256 行)
- ✅ `apeireth-true-understanding-2026-08-28.md` (229 行, 三面一体 + 五原型)
- ✅ `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` (361 行)
- ✅ `round-13-1-0-maturity-audit-2026-08-28.md` (323 行)
- ✅ 5 R7 真账 + 6 R11 真账 = 11 真调研
- ✅ 22 真账 doc (~5700 行)

**任何人都能接手路径**:
1. 接手工程师读 5 份必读 doc (handbook + handoff log + audit + verifications + true-understanding) = 1-2 小时
2. 真账完整索引 (Round 9-14 累计)
3. 派单 brief 模板 (Round 13 §3.3 派单顺序 11 项)
4. 主代理亲做 10 项 spec 决策冻结 (~2 周)

### 1.8 O-5 不假装 (Never fake it)

**真账 (0 装诚实标全 flag, per Round 13 1.0 maturity 补查)**:
- ✅ R11 6 sub-agent 调研 0 实测 1.0 .rs 真代码 (仅真账 + 推断, 跟 0 实测 2.0 master branch 同失守)
- ✅ 主代理亲测 8 个核心 1.0 .rs maturity (daily_summary / diary / cross_diary / memory_injection / reflexion / education / partner + storage 修订)
- ✅ 0 装诚实标 ~10 项 trait 口已知 (1.0 真账 self-flag "trait 口留" / "0 LLM" / "无持久化" / "无 CAS 引擎")
- ✅ 主代理真账 §3.1 估 3-4 周 ❌ 修订 → 12-14 周 (不假装估对)
- ✅ R11 真账 ~25 项 → ~23 项 1.0 缺口 (修订 2 项 OK/partial)

**不假装路径**:
1. 真实施时主代理必亲测 (~35 项 1.0 .rs + 2.0 master branch) — 不假装已实测
2. 真实施时主代理必亲做 spec (10 项, ~2 周) — 不假装"调研就够"
3. 真实施时主代理必跑 5 重守门 baseline + LOCKED 0 触碰 — 不假装"代码就过"
4. release 时主代理必跑 ROADMAP §7 + MANIFESTO §14 修订 check — 不假装"估对就过"

### 1.9 O-6 永远追求最优 (总体 / 系统 / 架构 三阶审查)

**真账 (三阶审查, per 真账 brief §5 模板)**:
- **总体最优**: 真实施按真账 §3.3 派单顺序, critical path 11-13 周协调+上下文最重, 总 12-14 周
- **系统最优**: 真实施走扩展 trait 接口 (不破现有 9 organ trait + 12 cognitive slot wiring + LOCKED 5 项), 1 真实施 1 测 + 5 重守门 baseline + LOCKED 0 触碰
- **架构最优**: 真实施借签 28 项 1:1 可移植 + 4 项 trait 口主代理亲做 + 5 项 PARTIAL 0 装诚实标, 0 引新外部 dep, 物种化借签边界 (per-user 塑形 + 声音形状 + 时间 + 语言 + 形态)

**O-6 永远追求最优路径**:
1. 每 commit message 必带三阶审查 (per §5 模板) + 拒 alternatives + 拒理由
2. 派单 brief 必含 "0 装诚实" + "5 重守门 baseline" + "LOCKED 0 触碰"
3. 真实施路径按 critical path 排序 (R12-CoordinationContext 11-13 周最重)

---

## 2. v2.0 真实施完成计划真账 (per 9 哲学锚 + 12-14 周 critical path)

### 2.1 主代理亲做 (10 项 spec, ~2 周)

按真账 `round-13-1-0-maturity-audit-2026-08-28.md` §5.1:

| # | 项 | 估时 | 阻塞 | 优先级 |
|---|---|---|---|---|
| 1 | v1 context.rs + context_rot.rs rot_score 融合 (per R11 catch) | 1-2 天 | 0 | 🟢 P0 (Round 13 派单同步) |
| 2 | cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec (per R11 真账) | 1-2 天 | 0 | 🟢 P0 |
| 3 | hello.rs 主题确认 (Windows Hello NGC vs 启动/装配, per R11 catch) | 1 小时 | 0 | 🟢 P0 |
| 4 | education 真 CAS spec (1.0 是字符串规则, 2.0 真 CAS sympy, per Round 13 maturity) | 1-2 周 | 0 | 🟢 P0 (物种化核心) |
| 5 | confidence BetaBinomial trait spec (1.0 完整, 2.0 organ::world_model::CalibrationStrength 本地简化版需补 BetaBinomial trait, per Round 13 maturity) | 1 周 | 0 | 🟢 P0 (物种化核心) |
| 6 | reflexion 3 trait 口实接线 spec (LLM CRITIC + 失败事件实接线 + 注入块消费侧, per Round 13 maturity) | 1 周 | 0 | 🟢 P0 (物种化核心) |
| 7 | 6 真实施派单 brief 模板 (Round 13 §3.3 派单顺序 11 项) | 1-2 天 | 0 | 🟢 P0 |
| 8 | **主代理实测 27 项 1.0 .rs maturity 补查 + 2.0 crates 真账实测** (本地 working tree 已就位, 0 git clone 必要, per Round 12 catch + Round 13 修订) | 2-3 天 | 0 (本地 working tree 已就位, 只需主代理亲自读) | 🟡 P1 (主代理亲做, 不依赖网络) |
| 9 | ROADMAP §7 + MANIFESTO §14 release timeline 修订 ✅ | (Round 14 commit `3ea454f1` 已 done) | 0 | ✅ done |
| 10 | §3.1 估时修订 ✅ | (Round 13 真账已 done) | 0 | ✅ done |
| **主代理亲做总估时** | **~2 周** (7 项 spec 立即可做, 不依赖网络, +#8 本地实测 2-3 天) | | | |

### 2.2 派 sub-agent 真实施 (11 项, 12-14 周 critical path)

按真账 `round-13-1-0-maturity-audit-2026-08-28.md` §3.3 派单顺序, 真账 brief 模板 per `v2-reference-handbook-2026-08-28.md` §3.1:

| # | 派单 | 估时 | 优先级 | 物种化维度 | 阻塞 |
|---|---|---|---|---|---|
| 1 | **R12-CoordinationContext-1** (onering + oracle / context+context_rot 融合 + 部分 hello) | 3-4 周 | 🟢 P0 | 协调+上下文 | 主代理 #1 rot_score 融合 + #3 hello 主题 |
| 2 | **R12-CoordinationContext-2** (continuation + continuity + spill + milestone + experiment_field) | 3-4 周 | 🟢 P0 | 协调+上下文 | 0 |
| 3 | **R12-CoordinationContext-3** (proactive + progressive + pentest + Kani bridge) | 2-3 周 | 🟢 P0 | 协调+上下文 | 0 |
| 4 | **R12-SpeciesCore-1** (principles + partner) | 2 周 | 🟢 P0 | 物种化核心 | 主代理 #5 confidence BetaBinomial trait spec |
| 5 | **R12-SpeciesCore-2** (community + education) | 2 周 | 🟢 P0 | 物种化核心 | 主代理 #4 education 真 CAS spec |
| 6 | **R12-LongTermMemory** (daily_summary + diary + cross_diary + memory_injection + reflexion + reflection) | 5-7 周 | 🟢 P0 | 长期记忆塑形 | 主代理 #2 cognitive module spec + #6 reflexion 3 trait 口 spec |
| 7 | **R12-Storage** (VectorIndex BM25 hybrid + Graph causal engine) | 1-2 周 | 🟢 P0 | 存储抽象层 | 0 |
| 8 | **R13-SpeciesForm** (timeline + tone + morphology) | 5-8 周 | 🟡 P1 | 物种化塑形维度 | 0 |
| 9 | **R13-MetaCognition** (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter) | 5-7 周 | 🟡 P1 | 反思+元认知 | 主代理 #5 confidence spec |
| 10 | **R13-ToolsSecurity** (ToolSynthesizer sandbox fix + Invest + Browser 真接 + Vision Windows 真接 + Voice whisper 真接) | 6-10 周 | 🟡 P1 | 工具+安全 | D 块硬件 |
| 11 | **R20 preference_learning** (跟 R12-LongTermMemory 并行, cognitive memory 增维) | 2-3 周 | 🟢 P0 (in-progress) | 长期记忆塑形 | R10 OrganKind 决策 + 主代理 #2 cognitive module spec |
| **派 sub-agent 真实施总 critical path** | **12-14 周** | | | |
| **真实施总估时 (含主代理亲做)** | **14-16 周 (~4 月)** | | | |

### 2.3 真实施顺序 + critical path

按真账 `round-13-1-0-maturity-audit-2026-08-28.md` §3.3, 真实施顺序:

**Phase 1: 主代理亲做 spec (2 周) + 派 R12-CoordinationContext-1 准备 (1 周)**:
- Week 1-2: 主代理亲做 #1-#7 spec
- Week 2-3: 派 R12-CoordinationContext-1 + R12-Storage

**Phase 2: R12-CoordinationContext 3 sub-agent (8-11 周 critical path 最重)**:
- Week 3-6: R12-CoordinationContext-1 (onering + oracle + context+context_rot 融合)
- Week 4-8: R12-CoordinationContext-2 (continuation + continuity + spill + milestone + experiment_field)
- Week 5-8: R12-CoordinationContext-3 (proactive + progressive + pentest + Kani bridge)

**Phase 3: R12-SpeciesCore 2 sub-agent (4 周)**:
- Week 6-8: R12-SpeciesCore-1 (principles + partner)
- Week 8-10: R12-SpeciesCore-2 (community + education)

**Phase 4: R12-LongTermMemory + R20 (7-10 周 critical path 跟 R22 reflection 并行)**:
- Week 7-14: R12-LongTermMemory (daily_summary + diary + cross_diary + memory_injection + reflexion + reflection)
- Week 7-10: R20 preference_learning (跟 R22 reflection 并行)

**Phase 5: R13 后续 (8-12 周)**:
- Week 10-17: R13-SpeciesForm (timeline + tone + morphology)
- Week 10-16: R13-MetaCognition (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter)
- Week 12-20: R13-ToolsSecurity (ToolSynthesizer + Invest + Browser + Vision Windows + Voice whisper, 需硬件)

**Phase 6: R14 release 流程 (1-2 周)**:
- Week 18-20: ROADMAP §7 + MANIFESTO §14 修订 check + 5 重守门 baseline 实测 + 0 触碰 LOCKED verify + ROADMAP §12 release path check + tag v2.0.0

**总 critical path**: Week 1-20 (约 5 月, 真实施 14-16 周 + release 流程 2 周)

### 2.4 release 估时修订 (per 真账 §3 + Round 14 commit `3ea454f1`)

| 项 | 原估 (MANIFESTO §14) | 修订估 (Round 12-13) | 修订原因 |
|---|---|---|---|
| Release 时间 | 2027-Q1-Q2 (4-6 月) | **2027-Q3 (6-9 月)** | ~35 项 1.0 缺口 + 1.0 maturity 补查 |
| 总进度 | 80% | **70-75%** | 因 1.0 vs 2.0 功能全集对比发现缺口, 重新估 |
| 真实施 critical path | 3-4 周 (主代理真账 §3.1 估) | **12-14 周** (修订) | 主代理 §3.1 估 3-4 周 ❌ 偏乐观 |
| 总 critical path (含主代理亲做) | n/a | **14-16 周 (~4 月)** | 主代理亲做 10 项 ~2 周 + 派 sub-agent 12-14 周 |

---

## 3. 真实施 release 流程 (per 9 哲学锚 + 真账 brief)

### 3.1 真实施前主代理亲做 10 项 spec (估时 ~2 周)

按真账 §2.1 表 1-10:

```
Week 1:
- Day 1-2: v1 context.rs + context_rot.rs rot_score 融合 (per R11 catch)
- Day 2-3: hello.rs 主题确认 (Windows Hello NGC vs 启动/装配)
- Day 3-5: cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec (per R11 真账)
- Day 5-7: 6 真实施派单 brief 模板 (per Round 13 §3.3 派单顺序 11 项)
- Day 7: §3.1 估时修订 ✅ (已 done)

Week 2:
- Day 8-10: education 真 CAS spec (1.0 是字符串规则, 2.0 真 CAS sympy, per Round 13 maturity)
- Day 11-14: confidence BetaBinomial trait spec (1.0 完整, 2.0 organ::world_model::CalibrationStrength 本地简化版需补 BetaBinomial trait, per Round 13 maturity)
- Day 14: reflexion 3 trait 口实接线 spec (LLM CRITIC + 失败事件实接线 + 注入块消费侧, per Round 13 maturity)
```

### 3.2 派 sub-agent 真实施 (估时 12-14 周 critical path)

按真账 §2.2 派单顺序 1-11 + 真账 brief 模板 (per `v2-reference-handbook-2026-08-28.md` §3.1):

```
Week 3-6: R12-CoordinationContext-1 (onering + oracle + context+context_rot 融合)
Week 4-8: R12-CoordinationContext-2 (continuation + continuity + spill + milestone + experiment_field)
Week 5-8: R12-CoordinationContext-3 (proactive + progressive + pentest + Kani bridge)
Week 6-8: R12-SpeciesCore-1 (principles + partner)
Week 8-10: R12-SpeciesCore-2 (community + education)
Week 7-14: R12-LongTermMemory (daily_summary + diary + cross_diary + memory_injection + reflexion + reflection)
Week 3-5: R12-Storage (VectorIndex BM25 hybrid + Graph causal engine)
Week 10-17: R13-SpeciesForm (timeline + tone + morphology)
Week 10-16: R13-MetaCognition (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter)
Week 12-20: R13-ToolsSecurity (ToolSynthesizer + Invest + Browser + Vision Windows + Voice whisper)
Week 7-10: R20 preference_learning (in-progress)
```

### 3.3 release 流程 (Week 18-20, 估时 1-2 周)

```
- 5 重守门 baseline 实测 (test 1739 / clippy 0 / LOCKED 0 触碰 / legacy 36 / 9 哲学锚 0 减)
- o6-anchor.yml workflow 自动跑 5 重守门
- ROADMAP §7 总进度 check + MANIFESTO §14 release timeline check
- ROADMAP §12 release path check
- git tag v2.0.0 (per 真账 §6 修订)
- push v2.0.0 tag + release notes
- release announcement
```

---

## 4. 主代理真账下一步想法 (答用户 "要怎么完成 2.0")

### 4.1 立即可做 (主代理亲做 spec, ~2 周, **不依赖网络**)

按真账 §2.1 表 1-10, 主代理亲做 10 项 spec 决策冻结, **不依赖网络**:

1. **v1 context.rs + context_rot.rs rot_score 融合** (1-2 天) — **立即做**
2. **hello.rs 主题确认** (1 小时) — **立即做**
3. **cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec** (1-2 天) — **立即做**
4. **6 真实施派单 brief 模板** (1-2 天) — **立即做**
5. **education 真 CAS spec** (1-2 周) — **立即做** (物种化核心)
6. **confidence BetaBinomial trait spec** (1 周) — **立即做** (物种化核心)
7. **reflexion 3 trait 口实接线 spec** (1 周) — **立即做** (物种化核心)

### 4.2 派 sub-agent 真实施 (主代理亲做派单 brief, ~12-14 周 critical path, **不依赖网络 — 本地 working tree 已就位真账**)
- per O-6 doctrine "工作量不是拒绝重做的理由"
- 主代理亲做 #4 6 真实施派单 brief 模板 (Round 13 §3.3 派单顺序 11 项)
- 真账 brief 模板 per `v2-reference-handbook-2026-08-28.md` §3.1
- 0 装诚实: sub-agent brief 必含 "0 装诚实" + "5 重守门 baseline" + "LOCKED 0 触碰"
- **不依赖网络** — sub-agent 写真账到本地 working tree, 主代理亲验 + commit + push
- 真实施时主代理必亲测 (per Round 13 catch 0 实测 1.0 .rs 部分 + 2.0 真账实测, 本地 working tree 已就位)

按真账 §2.2 派单顺序 1-11, 派 sub-agent 真实施:

| # | 派单 | 估时 | 物种化维度 |
|---|---|---|---|
| 1 | R12-CoordinationContext-1 (onering + oracle / context+context_rot 融合 + 部分 hello) | 3-4 周 | 协调+上下文 |
| 2 | R12-CoordinationContext-2 (continuation + continuity + spill + milestone + experiment_field) | 3-4 周 | 协调+上下文 |
| 3 | R12-CoordinationContext-3 (proactive + progressive + pentest + Kani bridge) | 2-3 周 | 协调+上下文 |
| 4 | R12-SpeciesCore-1 (principles + partner) | 2 周 | 物种化核心 |
| 5 | R12-SpeciesCore-2 (community + education) | 2 周 | 物种化核心 |
| 6 | R12-LongTermMemory (daily_summary + diary + cross_diary + memory_injection + reflexion + reflection) | 5-7 周 | 长期记忆塑形 |
| 7 | R12-Storage (VectorIndex BM25 hybrid + Graph causal engine) | 1-2 周 | 存储抽象层 |
| 8 | R13-SpeciesForm (timeline + tone + morphology) | 5-8 周 | 物种化塑形维度 |
| 9 | R13-MetaCognition (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter) | 5-7 周 | 反思+元认知 |
| 10 | R13-ToolsSecurity (ToolSynthesizer + Invest + Browser + Vision Windows + Voice whisper) | 6-10 周 | 工具+安全 |
| 11 | R20 preference_learning (跟 R12-LongTermMemory 并行) | 2-3 周 | 长期记忆塑形 |

### 4.3 release 流程 (Week 18-20, 1-2 周)

按真账 §3.3:

1. **5 重守门 baseline 实测** (test / clippy / LOCKED / legacy / 9 哲学锚)
2. **ROADMAP §7 + MANIFESTO §14** check (修订 release timeline 2027-Q3)
3. **ROADMAP §12 release path** check
4. **git tag v2.0.0** (per 真账 §6 修订)
5. **push v2.0.0 tag + release notes + release announcement**

### 4.4 主代理 + 接手工程师分工建议

按真账 brief 模板 (per `v2-reference-handbook-2026-08-28.md` §3.1):

**主代理 (Mavis)**:
- 10 项 spec 决策冻结 (~2 周)
- git clone ~~v2 master branch~~ + 真对照 1.0 vs 2.0 (**本地 working tree 已就位, 0 git clone 必要**, ~35 项 .rs 实测, 主代理亲做 ~2-3 天)
- release 流程 check + ROADMAP §7 + MANIFESTO §14 修订 (~1 周)
- release announcement + tag v2.0.0 (~1 周)

**接手工程师**:
- 读 5 份必读 doc (handbook + handoff log + audit + verifications + true-understanding) ~1-2 小时
- 真账完整索引 (Round 9-14 累计 22 真账)
- 派单 brief 模板 (Round 13 §3.3)
- 接手 sub-agent 真实施 (per 真账 §2.2)
- 5 重守门 baseline + LOCKED 0 触碰 实测 (1 真实施 1 测)

---

## 5. 0 装诚实标 (per O-5)

| 失守 | 详情 | 修法 |
|---|---|---|
| **Round 11 6 sub-agent 调研 0 实测 1.0 .rs** | 仅凭真账 + 推断, 跟 0 实测 2.0 master branch 同失守 | Round 13 主代理亲测 8 个核心 1.0 .rs maturity, 修订主代理真账 §2.4 maturity 区分 |
| **主代理 §3.1 估 3-4 周 ❌ 偏乐观** | 修订 → 12-14 周 critical path (Round 14 release 修订 commit `3ea454f1` 已 push) | 主代理亲做 spec ~2 周 + 派 sub-agent 12-14 周 = 真实施 14-16 周 critical path |
| **release timeline 修订** | 4-6 月 → 6-9 月 (因 ~35 项 1.0 缺口 + 1.0 maturity 补查) | Round 14 commit `3ea454f1` 已 push 修订 |
| **0 实测 2.0 master branch** | 本地 working tree 已就位 ~86-crate v1 + 16 crates v2 真账, 仅真账 + 推断 + R7/R11 真调研推论 | 真实施时主代理必亲验 (~2-3 天本地实测 27 项 1.0 .rs + 2.0 真账, 0 git clone 必要) |
| **1.0 maturity 35 项中 8 项实测 (~23%)** | Round 13 主代理亲测 8 个核心 1.0 .rs, 余 27 项仅凭推断 | 真实施时主代理必亲测 (~35 项 1.0 .rs 实测 + 物种化扩展 + 0 触碰 LOCKED) |
| **0 引新外部 dep** | per 真账 brief 约束, 1:1 翻译优先借签 1.0 真账 | 物种化借签边界: 借签 1.0 真账 + R7 真调研 + research/source 真开源, 0 新外部 dep |

---

## 6. 留 backlog (per Round 14 真实施计划)

### 6.1 主代理亲做 (10 项 spec 决策冻结, 估时 ~2 周)

per 真账 §2.1 表 1-10, 按 critical path + 物种化核心 + LOCKED 0 触碰 派单.

### 6.2 派 sub-agent 真实施 (11 项, 估时 12-14 周 critical path)

per 真账 §2.2 派单顺序 1-11, 按真账 brief 模板 (per `v2-reference-handbook-2026-08-28.md` §3.1).

### 6.3 release 流程 (Week 18-20, 估时 1-2 周)

per 真账 §3.3, 5 重守门 + ROADMAP §7 + MANIFESTO §14 + ROADMAP §12 check + git tag v2.0.0 + release announcement.

### 6.4 真实施 + release 总估时

- 主代理亲做 spec ~2 周
- 派 sub-agent 真实施 ~12-14 周 critical path
- release 流程 ~1-2 周
- **真实施 + release 总估时: ~14-16 周 + 1-2 周 = 15-18 周 (~4 月)**
- **真账修订: 2027-Q3 release (per ROADMAP §7 + MANIFESTO §14 修订, Round 14 commit `3ea454f1` 已 push)**

---

_Mavis 写于 2026-08-28 Round 14, 用户原话 '把文档先更新了, 注意实事求是, 注意哲学锚, 然后给我一个你下一步的想法, 要怎么完成 2.0' 触发, 真账写真账 v2.0 release 完成计划 (per 9 哲学锚 + S-2 实事求是 + 真账修订 12-14 周 critical path + ~35 项 1.0 缺口 + 真实施 14-16 周 + release 2027-Q3). 0 装诚实标: 0 实测 2.0 master + 1.0 .rs 部分实测 (~23%), 真实施时主代理必亲测 (~35 项 1.0 .rs + 2.0 master branch)._
