# R12-R13-R14 真实施派单 brief 模板 (2026-08-28)

> **作者**: 主代理 Mavis (写于 Round 15, 用户原话 "做点小的, 边做边更新文档, 注意文档工程规范, 哲学锚, 追求整体系统架构最优" 触发)
> **用途**: Round 12-14 真实施派单 brief 模板 (11 项派单 + 真实施流程 + 真账 brief 必含 7 段), 给主代理 + 接手工程师 + 派 sub-agent 真实施统一模板
> **关系**: 跟 `v2-reference-handbook-2026-08-28.md` §3.1 brief 模板 (Round 9 已就位) + `round-14-v2-completion-plan-2026-08-28.md` §3.1 真实施顺序 + `round-13-1-0-maturity-audit-2026-08-28.md` §3.3 派单顺序 11 项 互补

```
[Document-Meta]
Document:        docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md
Version:         1.0 (主代理 Mavis 写于 Round 15, 立即派单 brief 模板)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (R12-R13-R14 真实施派单 brief 模板, 主代理亲做 + 派 sub-agent 真实施统一模板)
Author:          主代理 Mavis
```

---

## 0. 真账 brief 模板设计原则 (per 9 哲学锚 + 文档工程规范)

**设计原则** (per O-6 永远追求最优 + S-2 实事求是 + 整体系统架构最优):

1. **统一模板**: 11 项派单 brief 用同一模板 (R12-CoordinationContext-1/2/3 + R12-SpeciesCore-1/2 + R12-LongTermMemory + R12-Storage + R13-SpeciesForm + R13-MetaCognition + R13-ToolsSecurity)
2. **必含 7 段**: 任务 + 必读 + 必输出 + 0 装诚实标 + 5 重守门 + LOCKED 0 触碰 + 真实施流程
3. **不依赖网络**: 本地 working tree 已就位 (per Round 15 用户 catch 修订), 0 git clone 必要
4. **主代理亲做 + 派 sub-agent 真实施分工**: 主代理亲做 spec (~2 周, 7 项 spec 决策冻结), 派 sub-agent 真实施 (~12-14 周 critical path, 11 项派单)

---

## 1. 真账 brief 模板 (主代理 + 接手工程师 + sub-agent 必读)

### 1.1 任务 (Brief)

```
任务: [具体任务名 + 1 句话定位]

背景:
- v2.0 真账缺 ~35 项 1.0 真实施 (per Round 11-12 1.0 vs 2.0 gap 真账)
- 本次派单是真实施 critical path 12-14 周的一部分
- 物种化借签边界 (per Round 10 5 R7 真调研 + Round 13 1.0 maturity 补查)

承接:
- 主代理亲做 spec 已决策冻结 (~7 项, per Round 14 真实施完成计划 §2.1)
- 真实施派单 brief 模板 (本文件, per O-6 整体系统架构最优)
- 真账 brief 模板必含 7 段 (任务 + 必读 + 必输出 + 0 装诚实 + 5 重守门 + LOCKED 0 触碰 + 真实施流程)

物种化维度 (per vision.md L29-49 + apeireth-true-understanding-2026-08-28.md):
- 三面一体: 基地 (LLM 操作系统) + Agent 平台 (16 crates workspace) + 她 (物种实现, per-user 塑形)
- 五原型: 世界模型 + 自我改进 + 自主好奇心 + 连续感知 + 价值内化
- 物种化: "发布后每个用户养的她, 机制/哲学/安全同源, 记忆/偏好/好奇形状被各自的共同生活塑形"
```

### 1.2 必读 (Brief)

```
Apeireth v2.0 真实施必读 (主代理 + 接手工程师 + sub-agent 必读 7 份):

1. 文档元数据 (必含):
   - docs/04-internal/round-14-v2-completion-plan-2026-08-28.md (v2 完成计划, 9 哲学锚 + 401 行)
   - docs/04-internal/apeireth-true-understanding-2026-08-28.md (物种化真理解, 229 行)
   - docs/04-internal/apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md (1.0 vs 2.0 真账, 361 行)

2. 工程规范 (必含):
   - docs/04-internal/v2-reference-handbook-2026-08-28.md (§3.1 brief 模板 + §4 改前必跑 + §5 commit msg 模板 + §7 工程规范 + §8.5 hook, 613 行)
   - docs/04-internal/ENGINEER-MANIFESTO.md (§13 12 真实陷阱 + §10 LOCKED 5 项)

3. 物种化借签 (Round 10 5 真调研):
   - docs/01-architecture/r7-neko-species-research-2026-08-28.md (五维记忆)
   - docs/01-architecture/r7-open-llm-vtuber-species-research-2026-08-28.md (4 段 pipeline + ASR/TTS)
   - docs/01-architecture/r7-firefly-species-research-2026-08-28.md (GPT-SoVITS 原声 TTS)
   - docs/01-architecture/r7-mio-species-research-2026-08-28.md (Windows 本地优先 + 日记)
   - docs/01-architecture/r7-airi-species-research-2026-08-28.md (永远不下播)

4. 1.0 真账 maturity 补查 (Round 13):
   - docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md (8 个核心 .rs 实测 + 修订真实施估时)

5. v2 真账 (本地 working tree):
   - crates/ (16 crates workspace, 0 git clone 必要)
   - legacy/donor/apeireth-companion/src/ (~100 modules, 1.0 真账)
   - _research_mem/apeireth-rust-fork/ (~86-crate v1 era)

6. 子代理 brief (本派单项):
   - legacy/donor/apeireth-companion/src/[具体 1.0 模块].rs (~100-1500 行 1.0 真账)
   - crates/[具体 2.0 真账 path] (物种化借签边界真账)

7. 真账 brief 模板 (本文件):
   - docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md (本文件, 派单统一模板)
```

### 1.3 必输出 (Brief)

```
写真账 to: docs/[具体 path]/[具体 name]-implementation-2026-08-28.md (≤ 300 行, 0 装诚实标必含)

写真账必含 7 段 (跟主代理 brief 模板对齐):

### 1. 真实施摘要 (≤ 50 行)
- 1.0 真账实测 (legacy/donor/apeireth-companion/src/[具体].rs 行数 + maturity + 0 装 PASS 标注)
- 2.0 真账实测 (crates/[具体] 真账 + LOCKED 0 触碰 verify)
- 真实施 7 段 (1.0 → 2.0 真账对接 + 真实施代码 + 真账对接 + 物种化借签 + 集成测试 + 0 装诚实 + 下一步)

### 2. 5 重守门 baseline 实测 (≤ 30 行)
- cargo test --workspace --locked (期望 1739+N passed / 0 failed)
- cargo clippy --workspace --all-targets --locked -- -D warnings (期望 0 warning)
- cargo check --workspace --locked (期望 0 副作用)
- git diff HEAD -- crates/foundation/core/src/{eight_anchors,philosophy,onion}.rs Cargo.toml:44 (期望 0 行, LOCKED 0 触碰)
- grep -r "legacy/" crates/ | wc -l (期望 < 100)

### 3. LOCKED 5 项 0 触碰 (≤ 30 行)
- 9 哲学锚本体 (eight_anchors.rs:58-79): 0 行
- 13 键 (philosophy.rs:142): 0 行
- 3 项不可变脊柱 (onion.rs:249): 0 行
- workspace.version (Cargo.toml:44): 0 改
- R11 baseline 3 值 (legacy reference): 0 触碰
- 9 哲学锚表头 (eight_anchors.rs enum): 0 减

### 4. 真账对接 + 物种化借签 (≤ 50 行)
- 1:1 翻译 v1 真账 (per maturity REAL 1:1 可移植)
- 1:1 + trait 口主代理亲做 spec (per maturity REAL with 注, 4 项)
- 物种化借签边界 (per-user 塑形 + 声音形状 + 时间 + 语言 + 形态, per vision.md L29-49)
- 0 装诚实: 0 装诱导 prevention (不假装 OK, 0 LLM 真接, 0 装 PASS 标注)
- 0 引新外部 dep (per 真账 brief 约束, 1:1 翻译优先借签 1.0 真账)

### 5. 真账对接 + 集成测试 (≤ 30 行)
- 真实施代码 (per 1.0 真账 1:1 翻译 + 2.0 真账对接)
- 集成测试 (cargo test + 真账对接 + species 塑形边界)
- 物种化借签 (per R7 5 真调研: per-user memory / preference / personality 塑形)

### 6. 主代理决策建议 (≤ 30 行)
- 1.0 真账可移植度 (REAL / PARTIAL / 0 装 PASS)
- 2.0 真账对接路径 (走扩展 trait 接口)
- 真实施 critical path 估时 (per 真账 brief brief)
- 下一步 (跟其他派单对接 / 真账 brief / 真实施主代理亲测)

### 7. 0 装诚实标 (≤ 30 行, 必含)
- 真实施时主代理亲测 (1.0 .rs 0 实测部分补查 + 2.0 真账实测)
- 真账 brief 模板必含 (本文件 §1.1-1.7)
- LOCKED 5 项 0 触碰 verify (本节 §3)
- 5 重守门 baseline verify (本节 §2)
- 物种化借签边界 (本节 §4)
- 真账 brief 模板必含 (§1.3 + §2 + §3 + §4)
```

### 1.4 0 装诚实标 (Brief)

```
真实施时主代理必亲测 (0 装诚实 doctrine):
- 1.0 .rs 0 实测部分补查 (35 项中 Round 13 亲测 8 项, 余 27 项需真实施时主代理亲测)
- 2.0 真账实测 (16 crates workspace 真账, 本地 working tree 已就位)
- 真实施时主代理必亲测 (~2-3 天本地实测, 0 git clone 必要, per Round 15 用户 catch 修订)

真账 brief 必含 (per O-6 永远追求最优):
- 物种化借签边界 (per vision.md + apeireth-true-understanding-2026-08-28.md)
- 0 装 PASS 标注 (1.0 真账 self-flag, ~10 项 trait 口待主代理亲做 spec)
- 真账 brief 模板 (本文件 §1.3 + §2 真实施流程 + §3 5 重守门 + §4 LOCKED 0 触碰)

真实施时主代理必亲验:
- 真账 brief 模板 (本文件)
- 真账 brief 必含 §3 5 重守门 baseline 实测
- 真账 brief 必含 §4 LOCKED 5 项 0 触碰 verify
- 真账 brief 必含 §5 真账对接 + 集成测试
- 真账 brief 必含 §7 0 装诚实标
```

### 1.5 5 重守门 + LOCKED 0 触碰 (Brief)

```
真实施时主代理亲测 (必含在写真账 §2 + §3):

5 重守门 baseline 实测:
1. clippy 0 warning:
   - 命令: cargo clippy --workspace --all-targets --locked -- -D warnings
   - 期望: 0 warning, 0 error
   - 当前实测: 0 warning
2. tests 0 fail:
   - 命令: cargo test --workspace --locked
   - 期望: 1739+N passed / 0 failed / 12 ignored
   - 当前实测: 1739 passed
3. legacy compat path < 100:
   - 命令: grep -r "legacy/" crates/ | wc -l
   - 期望: < 100
   - 当前实测: 36
4. LOCKED 5 项 0 触碰:
   - 命令: git diff HEAD -- crates/foundation/core/src/{eight_anchors,philosophy,onion}.rs Cargo.toml:44 crates/foundation/core/src/cognitive.rs
   - 期望: 0 行
   - 当前实测: 0 行
5. 9 哲学锚表头 0 减:
   - 命令: grep "S-[1-3]\|O-[1-6]" crates/foundation/core/src/eight_anchors.rs | wc -l
   - 期望: 9 (S-1, S-2, S-3, O-1, O-2, O-3, O-4, O-5, O-6)
   - 当前实测: 9

LOCKED 5 项 0 触碰 verify:
- 9 哲学锚本体: crates/foundation/core/src/eight_anchors.rs:58-79 (enum)
- 13 键: crates/foundation/core/src/philosophy.rs:142 (RUNTIME_ENFORCED = false)
- 3 项不可变脊柱: crates/foundation/core/src/onion.rs:249
- workspace.version: Cargo.toml:44 ("1.2.0")
- R11 baseline 3 值: legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44 (R11_V1141/1131/1136_BASELINE = 0.8682/0.8532/0.9063)

真实施必含 §3 + §4 5 重守门 baseline + LOCKED 0 触碰 verify (走扩展 trait 接口, 不破现有 9 organ trait + 12 cognitive slot wiring + LOCKED 5 项)
```

### 1.6 真实施流程 (Brief)

```
真实施流程 (主代理亲做 + 派 sub-agent 真实施分工):

Phase 1: 主代理亲做 spec (~2 周, 立即可做, 不依赖网络, 本地 working tree 已就位)
  - #1 v1 context.rs + context_rot.rs rot_score 融合 (1-2 天)
  - #2 hello.rs 主题确认 (1 小时, 已知是 Windows Hello NGC, 不是"启动/装配", per Round 14 真实施完成计划 §2.1)
  - #3 cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec (1-2 天)
  - #4 6 真实施派单 brief 模板 (1-2 天, 本文件)
  - #5 education 真 CAS spec (1-2 周, 物种化核心)
  - #6 confidence BetaBinomial trait spec (1 周, 物种化核心)
  - #7 reflexion 3 trait 口实接线 spec (1 周, 物种化核心)
  - 主代理实测 27 项 1.0 .rs maturity 补查 + 2.0 真账实测 (本地 working tree, 0 git clone 必要, ~2-3 天)

Phase 2: 派 sub-agent 真实施 (12-14 周 critical path, 不依赖网络, 本地 working tree 已就位真账)
  - R12-CoordinationContext-1/2/3 (协调+上下文, 11-13 周最重)
  - R12-SpeciesCore-1/2 (物种化核心, 4 周)
  - R12-LongTermMemory (长期记忆塑形, 5-7 周)
  - R12-Storage (修订后 1-2 周, BM25 hybrid + causal engine 补)
  - R13-SpeciesForm + MetaCognition + ToolsSecurity (后续, 5-8 周 + 5-7 周 + 6-10 周)
  - R20 preference_learning (in-progress, 2-3 周)

Phase 3: release 流程 (Week 18-20, 1-2 周)
  - 5 重守门 baseline 实测 + ROADMAP §7 + MANIFESTO §14 + ROADMAP §12 check
  - git tag v2.0.0 (per 真账 §6 修订)
  - push v2.0.0 tag + release notes + release announcement

真实施 + release 总估时: ~4 月 (~15-18 周 critical path, 主代理亲做 + 真实施 + release 流程)
```

---

## 2. 真实施流程 (per Round 14 真账)

### 2.1 主代理亲做 10 项 spec (~2 周, 立即可做)

| # | 项 | 估时 | 阻塞 | 优先级 |
|---|---|---|---|---|
| 1 | v1 context.rs + context_rot.rs rot_score 融合 | 1-2 天 | 0 | 🟢 P0 |
| 2 | hello.rs 主题确认 (已知 Windows Hello NGC, 不是"启动/装配") | 1 小时 | 0 | 🟢 P0 |
| 3 | cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec | 1-2 天 | 0 | 🟢 P0 |
| 4 | 6 真实施派单 brief 模板 (本文件) | 1-2 天 | 0 | 🟢 P0 |
| 5 | education 真 CAS spec (1.0 字符串规则 → 2.0 真 CAS sympy) | 1-2 周 | 0 | 🟢 P0 (物种化核心) |
| 6 | confidence BetaBinomial trait spec (1.0 完整, 2.0 organ::world_model::CalibrationStrength 本地简化版需补) | 1 周 | 0 | 🟢 P0 (物种化核心) |
| 7 | reflexion 3 trait 口实接线 spec (LLM CRITIC + 失败事件实接线 + 注入块消费侧) | 1 周 | 0 | 🟢 P0 (物种化核心) |
| 8 | 主代理实测 27 项 1.0 .rs maturity 补查 + 2.0 真账实测 | 2-3 天 | 0 (本地 working tree) | 🟡 P1 (不依赖网络) |
| 9 | ROADMAP §7 + MANIFESTO §14 release timeline 修订 ✅ | (Round 14 commit `3ea454f1` 已 done) | 0 | ✅ done |
| 10 | §3.1 估时修订 ✅ | (Round 13 真账已 done) | 0 | ✅ done |

### 2.2 派 sub-agent 真实施 11 项 (~12-14 周 critical path)

| # | 派单 | 估时 | 物种化维度 | 阻塞 |
|---|---|---|---|---|
| 1 | R12-CoordinationContext-1 (onering + oracle / context+context_rot 融合 + 部分 hello) | 3-4 周 | 协调+上下文 | 主代理 #1 rot_score 融合 + #2 hello 主题确认 |
| 2 | R12-CoordinationContext-2 (continuation + continuity + spill + milestone + experiment_field) | 3-4 周 | 协调+上下文 | 0 |
| 3 | R12-CoordinationContext-3 (proactive + progressive + pentest + Kani bridge) | 2-3 周 | 协调+上下文 | 0 |
| 4 | R12-SpeciesCore-1 (principles + partner) | 2 周 | 物种化核心 | 主代理 #6 confidence BetaBinomial trait spec |
| 5 | R12-SpeciesCore-2 (community + education) | 2 周 | 物种化核心 | 主代理 #5 education 真 CAS spec |
| 6 | R12-LongTermMemory (daily_summary + diary + cross_diary + memory_injection + reflexion + reflection) | 5-7 周 | 长期记忆塑形 | 主代理 #3 cognitive module spec + #7 reflexion 3 trait 口 spec |
| 7 | R12-Storage (VectorIndex BM25 hybrid + Graph causal engine) | 1-2 周 | 存储抽象层 | 0 |
| 8 | R13-SpeciesForm (timeline + tone + morphology) | 5-8 周 | 物种化塑形维度 | 0 |
| 9 | R13-MetaCognition (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter) | 5-7 周 | 反思+元认知 | 主代理 #6 confidence spec |
| 10 | R13-ToolsSecurity (ToolSynthesizer + Invest + Browser + Vision Windows + Voice whisper) | 6-10 周 | 工具+安全 | D 块硬件 |
| 11 | R20 preference_learning (in-progress, 跟 R12-LongTermMemory 并行) | 2-3 周 | 长期记忆塑形 | R10 OrganKind 决策 + 主代理 #3 spec |

### 2.3 release 流程 (Week 18-20, 1-2 周)

```
- 5 重守门 baseline 实测 (test 1739 / clippy 0 / LOCKED 0 / legacy 36 / 9 哲学锚 0 减)
- ROADMAP §7 + MANIFESTO §14 + ROADMAP §12 check
- git tag v2.0.0 (per 真账 §6 修订)
- push v2.0.0 tag + release notes + release announcement
```

---

## 3. 11 派单 brief 具体模板 (per 真账 §2.2)

### 3.1 R12-CoordinationContext-1 (协调+上下文, 3-4 周, critical path 最重)

```
任务: onering + oracle / context+context_rot rot_score 融合 + 部分 hello 主题确认

必读:
- docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md (本文件)
- docs/04-internal/apeireth-true-understanding-2026-08-28.md (物种化真理解, §1.1 协调+上下文)
- docs/04-internal/r11-coordination-context-gap-research-2026-08-28.md (9 项调研真账, 283 行)
- docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md (1.0 真账 maturity 补查, 8 .rs 实测)
- legacy/donor/apeireth-companion/src/{onering.rs,oracle.rs,oracle_adapters.rs,context.rs,context_rot.rs,continuation.rs,continuity.rs,spill.rs,assemble.rs,hello.rs,milestone.rs,experiment_field.rs,proactive.rs,progressive.rs,pentest.rs,bridge_kani_proofs.rs,organ_kani_proofs.rs} (~1500+ 行 1.0 真账, REAL/PARTIAL)
- crates/engine/memory/src/canonical/{vector.rs,graph.rs} (v2 Storage 抽象层, VectorIndex + MemoryGraph 已 1:1 翻译)
- crates/engine/runtime/src/canonical/{orchestrator.rs,organ_kani_proofs.rs} (A 块 Stage 5 L0-L5 UpgradeCycle + organ_kani_proofs 已 1:1 翻译)

必输出:
- 写真账 to: docs/04-internal/r12-coordination-context-1-implementation-2026-08-28.md (≤ 300 行, 必含 §1.3 7 段)
- 7 段: 真实施摘要 + 5 重守门 baseline + LOCKED 0 触碰 + 真账对接 + 真账对接 + 集成测试 + 主代理决策建议 + 0 装诚实标
- 真实施代码: 1.0 真账 1:1 翻译 (onering + oracle + context+context_rot 融合 + assemble+hello 部分)
- 集成测试: cargo test + 真账对接 + species 塑形边界

0 装诚实标:
- 1.0 真账 ~1500+ 行 28 个 .rs maturity (REAL / PARTIAL / 0 装 PASS)
- 2.0 真账实测 (本地 working tree, 0 git clone 必要)
- 真实施时主代理亲测 ~3-4 天本地实测 (不依赖网络)
- v1 context.rs + context_rot.rs 重复实现 rot_score 主代理亲做融合先 (R11 catch)
- hello.rs 已知是 Windows Hello NGC, 不是"启动/装配" (R11 catch)
- 0 引新外部 dep (per 真账 brief 约束)

5 重守门 baseline + LOCKED 0 触碰:
- cargo test --workspace --locked (期望 1739+N passed / 0 failed)
- cargo clippy --workspace --all-targets --locked -- -D warnings (期望 0 warning)
- cargo check --workspace --locked (期望 0 副作用)
- git diff HEAD -- crates/foundation/core/src/{eight_anchors,philosophy,onion}.rs Cargo.toml:44 crates/foundation/core/src/cognitive.rs (期望 0 行, LOCKED 0 触碰)
- grep -r "legacy/" crates/ | wc -l (期望 < 100)

真实施流程:
- 主代理亲做 #1 v1 rot_score 融合 + #2 hello 主题确认 (~2-3 天)
- 派 sub-agent 真实施 onering + oracle + context+context_rot 融合 + assemble+hello 部分 (~3-4 周)
- 集成测试 + 5 重守门 baseline + LOCKED 0 触碰 verify
- 主代理亲验 commit + push
```

### 3.2 R12-CoordinationContext-2 (continuation + continuity + spill + milestone + experiment_field, 3-4 周)

```
任务: continuation + continuity + spill + milestone + experiment_field 真账对接 + 真实施

必读:
- docs/04-internal/r12-r13-r14-implementation-brief-template-2026-08-28.md (本文件)
- legacy/donor/apeireth-companion/src/{continuation.rs,continuity.rs,spill.rs,milestone.rs,experiment_field.rs} (~300+ 行 1.0 真账)
- crates/engine/runtime/src/canonical/orchestrator.rs (A 块 Stage 5 L0-L5 UpgradeCycle 已 1:1 翻译)
- crates/foundation/core/src/onion.rs (3 项不可变脊柱 LOCKED, 真实施走扩展 trait 接口)

必输出:
- 写真账 to: docs/04-internal/r12-coordination-context-2-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 + 2.0 L0-L5 UpgradeCycle 对接
- 0 引新外部 dep

5 重守门 baseline + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- git diff HEAD -- crates/foundation/core/src/{eight_anchors,philosophy,onion}.rs (期望 0 行)

0 装诚实标:
- 1.0 真账 ~300+ 行 5 .rs maturity (continuation/spill 部分缺)
- 真实施时主代理亲测 ~3-4 天本地实测
```

### 3.3 R12-CoordinationContext-3 (proactive + progressive + pentest + Kani bridge, 2-3 周)

```
任务: proactive + progressive + pentest + Kani bridge 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/{proactive.rs,progressive.rs,pentest.rs,bridge_kani_proofs.rs,organ_kani_proofs.rs} (~600+ 行 1.0 真账)
- crates/engine/organ/src/emergence.rs (E7 emergence organ ✅ 1:1 翻译, organ_kani_proofs 6 crate 已装 per R177)

必输出:
- 写真账 to: docs/04-internal/r12-coordination-context-3-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 + 2.0 E7 emergence 对接
- Kani bridge 0 → 1 (R177 organ_kani_proofs 6 crate 已装, bridge_kani_proofs 仍 0)

0 装诚实标:
- 1.0 真账 proactive 部分 (LarkDelivery 缺), progressive + pentest 完整
- Kani bridge_kani_proofs 0 → 1 是真实施任务 (organ_kani_proofs 6 crate R177 已装)
- 0 引新外部 dep (Kani 已是 dev-dependency, 真实施用现有 Kani 工具链)
```

### 3.4 R12-SpeciesCore-1 (principles + partner, 2 周)

```
任务: principles (F6 价值内化) + partner (跨用户协作) 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/{principles.rs (478 行), partner.rs (141 行), value_cases.rs, bond.rs} (1.0 真账)
- crates/engine/organ/src/value_cases.rs (F6 value_cases organ ✅ 1:1 翻译)
- docs/01-architecture/apeireth-true-understanding-2026-08-28.md (物种化真理解, §2 vision.md L49 跨墙的信任)

必输出:
- 写真账 to: docs/04-internal/r12-species-core-1-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 (principles 478 行 + partner 141 行 + bond 借签)
- 2.0 F6 value_cases organ 对接
- 物种化借签 (per vision.md L47 "记忆/偏好/好奇形状被共同生活塑形")

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项

0 装诚实标:
- principles 是 F6 价值内化层基础 (跟 F6 organ 借签边界)
- partner 是物种化核心 (vision L49 跨墙的信任)
- 主代理亲做 #6 confidence BetaBinomial trait spec (~1 周, 跟 partner 借签)
```

### 3.5 R12-SpeciesCore-2 (community + education, 2 周)

```
任务: community (物种化社区) + education (教育升级, 真 CAS) 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/{community.rs (360 行), education.rs (402 行)}
- docs/01-architecture/apeireth-true-understanding-2026-08-28.md (物种化真理解, vision L47 物种化社区 + L48 教育升级)
- 1.0 education 0 装诚实 L7-10 "v1 是字符串级规则表, 不是真实符号计算 (无 CAS 引擎)"

必输出:
- 写真账 to: docs/04-internal/r12-species-core-2-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 (community 360 行 + education 402 行)
- 真 CAS 借签 (主代理亲做 #5 education 真 CAS spec, 1.0 字符串规则 → 2.0 sympy 真 CAS)
- 物种化借签 (per vision.md L47 物种化社区 + L48 教育升级)

0 装诚实标:
- education 字符串规则不是真 CAS, 主代理亲做真 CAS spec (sympy 借签)
- community 物种化社区 (借签 vision L47)
- 1.0 真账 ~760 行 2 .rs maturity (REAL)
```

### 3.6 R12-LongTermMemory (daily_summary + diary + cross_diary + memory_injection + reflexion + reflection, 5-7 周)

```
任务: 长期记忆塑形 pipeline (daily_summary → cross_diary → memory_injection → reflection → memory_writeback → consolidation) 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/{daily_summary.rs (99 行 REAL), diary.rs (442 行 REAL + ⚠️ trait 口), cross_diary.rs (301 行 REAL + ⚠️ trait 口), memory_injection.rs (66 行 REAL), reflexion.rs (497 行 REAL + ⚠️ 3 trait 口待主代理亲做), reflection.rs (329 行 REAL), memory_extractor.rs, memory_graph.rs} (1.0 真账, ~2000 行 6 .rs)
- crates/engine/memory/src/{lightmemo/search.rs, dailynote/search.rs} (v2 BM25-lite 子模块, 不是 storage 主线)
- docs/04-internal/r11-longterm-memory-gap-research-2026-08-28.md (6 项调研真账, 311 行)
- docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md (8 个核心 .rs maturity 实测, reflexion 3 trait 口主代理亲做 spec)

必输出:
- 写真账 to: docs/04-internal/r12-longterm-memory-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 (6 .rs ~2000 行) + 2.0 cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline trait 对接
- reflexion 3 trait 口实接线 (主代理 #7 spec)
- cognitive module spec (主代理 #3 spec)

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项

0 装诚实标:
- 1.0 真账 ~2000 行 6 .rs maturity (REAL, 1:1 可移植)
- reflexion 3 trait 口 主代理亲做 spec (LLM CRITIC + 失败事件实接线 + 注入块消费侧)
- cognitive module consolidation_writeback_pipeline + reflection_writeback_pipeline 主代理亲做 spec
- 物种化借签 (per vision L47 + LongTermMemory 真账 §3.2)
- 0 引新外部 dep
```

### 3.7 R12-Storage (VectorIndex BM25 hybrid + Graph causal engine, 1-2 周)

```
任务: VectorIndex BM25 hybrid + Graph causal engine 真账对接 + 真实施

必读:
- crates/engine/memory/src/canonical/{vector.rs,graph.rs} (v2 VectorIndex + MemoryGraph 已 1:1 翻译 cosine)
- _research_mem/apeireth-rust-fork/crates/apeireth-vector/ (1.0 真账 sqlite-vec + Qdrant + traits, ~400+ 行)
- _research_mem/apeireth-rust-fork/crates/apeireth-graph-primitive/ (1.0 真账 BFS + predicate query, ~500+ 行)
- docs/04-internal/r11-storage-gap-research-2026-08-28.md (303 行, R11-Storage 真账)
- crates/engine/memory/src/{lightmemo,search.rs,dailynote/search.rs} (v2 BM25-lite 子模块, 不是 storage 主线)

必输出:
- 写真账 to: docs/04-internal/r12-storage-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: VectorIndex BM25 hybrid 补 + Graph causal engine 补 (跟 v2 canonical 已 1:1 翻译的 cosine VectorIndex + MemoryGraph BFS/shortest_path 借签)
- 1.0 真账 1:1 翻译 (BM25 hybrid + causal engine)

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项

0 装诚实标:
- 1.0 真账 BM25 hybrid 在 SqliteVecBackend + QdrantClient (~400+ 行)
- 1.0 真账 causal engine 在 graph_primitive (~500+ 行, BFS + predicate query)
- v2 canonical/vector.rs 已 1:1 翻译 cosine VectorIndex (差 BM25 hybrid)
- v2 canonical/graph.rs 已 1:1 翻译 MemoryGraph BFS + shortest_path (差 causal engine)
- 0 引新外部 dep (SqliteVecBackend 是 sqlite-vec 真接, QdrantClient 是 REST API 真接, 都在 1.0 真账已有)
```

### 3.8 R13-SpeciesForm (timeline + tone + morphology, 5-8 周)

```
任务: 物种化塑形维度 (时间 + 语言 + 形态) 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/{timeline.rs (79 行 REAL), tone.rs (374 行 REAL), morphology.rs (284 行 REAL)}
- docs/01-architecture/r7-mio-species-research-2026-08-28.md (物种化塑形维度借签边界, Mio §2 timeline 真账)
- docs/01-architecture/r7-firefly-species-research-2026-08-28.md (物种化塑形语言维度, Firefly §2 tone 真账)

必输出:
- 写真账 to: docs/04-internal/r13-species-form-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 (timeline 79 + tone 374 + morphology 284 = 737 行)
- 物种化借签 (per vision.md L47 + R7 真账 species 塑形边界)

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项

0 装诚实标:
- 1.0 真账 ~737 行 3 .rs maturity (REAL, 1:1 可移植)
- timeline + tone + morphology 物种化塑形维度 (时间 + 语言 + 形态)
- 0 引新外部 dep
```

### 3.9 R13-MetaCognition (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter, 5-7 周)

```
任务: 反思+元认知 (meta_thinking + thought_cluster + intent_brier + confidence + HybridCognitiveRouter) 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/{meta_thinking.rs (643 行 REAL), thought_cluster.rs (522 行 REAL), intent_brier.rs (817 行 REAL, 31 单测全绿), confidence.rs (177 行 REAL, BetaBinomial trait), hybrid.rs (master PARTIAL rule-based fast path)}
- crates/engine/organ/src/world_model.rs (v2 organ::world_model::CalibrationStrength 本地简化版 in-place, 差 BetaBinomial trait)
- docs/04-internal/round-13-1-0-maturity-audit-2026-08-28.md (1.0 maturity 补查, 8 .rs 实测)

必输出:
- 写真账 to: docs/04-internal/r13-meta-cognition-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 (meta_thinking 643 + thought_cluster 522 + intent_brier 817 + confidence 177 + HybridCognitiveRouter 真实施)
- v2 organ::world_model::CalibrationStrength 补 BetaBinomial trait (主代理 #6 confidence spec)
- cognitive.council + judge 对接 (confidence trait)

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项

0 装诚实标:
- 1.0 真账 ~2200 行 5 .rs maturity (REAL, 1:1 可移植)
- confidence BetaBinomial trait 主代理亲做 spec (~1 周)
- HybridCognitiveRouter 1.0 rule-based fast path with hardcoded templates (master PARTIAL, 不推荐 1:1 翻译)
- 物种化借签 (per R7 真账 species + R11 真账 meta-cognition)
- 0 引新外部 dep
```

### 3.10 R13-ToolsSecurity (ToolSynthesizer + Invest + Browser + Vision Windows + Voice whisper, 6-10 周, 需硬件)

```
任务: ToolSynthesizer sandbox fix + Invest + Browser 真接 + Vision Windows (ScreenCapture + OmniParser + DesktopAction) + Voice whisper 真接 (R14 真 modality backend)

必读:
- legacy/donor/apeireth-companion/src/{synthesis.rs (PARTIAL, sandbox unused security risk), builtin/{shell.rs, filesystem.rs, fetch.rs, browser.rs (PARTIAL, HTML stripping only), search.rs, repo_tools.rs, invest.rs (PARTIAL), learning.rs (PARTIAL), system_monitor.rs (PARTIAL)}, vision/{screen.rs (REAL Windows), omni_parser.rs (REAL Windows), desktop_action.rs (PARTIAL)}, voice/{vad.rs (REAL), tts.rs (STUB), lipsync.rs (PARTIAL)}, mcp/* (REAL)}
- crates/apeireth-tools/src/{lib.rs,builtin/{shell,filesystem,fetch,browser,search,repo_tools,invest,learning,system_monitor}, vision/{screen,omni_parser,desktop_action}, sandbox.rs, worktree.rs, synthesis.rs, mcp/*} (v2 真账)
- crates/engine/perception/src/{perception_backend.rs} (R6 trait + 5 modality 抽象, 真 backend 待 R14)
- docs/04-internal/rc7-perception-research-2026-08-28.md (RC-7 真账, 228 行)

必输出:
- 写真账 to: docs/04-internal/r13-tools-security-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: ToolSynthesizer sandbox 修复 (security critical) + Invest + Browser 真接 (Playwright MCP) + Vision Windows 真接 (GDI capture + 窗口枚举 + SendInput) + Voice whisper 真接 (R14 真 modality backend)
- 1.0 真账 1:1 翻译 (~2000+ 行 30+ .rs)

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项
- 0 unsafe code (ToolSynthesizer sandbox 修复优先 0 unsafe)

0 装诚实标:
- 1.0 真账 ~30 个 .rs maturity (REAL/PARTIAL/STUB)
- Vision Windows 真接需硬件 (ScreenCapture + OmniParser Windows-only)
- Voice whisper 真接 (R14 真 modality backend, OpenAI /v1/audio/transcriptions 或 MiniMax 兼容)
- ToolSynthesizer sandbox unused 是 security risk, 修复优先
- 0 引新外部 dep (sqlite-vec + Qdrant REST + Playwright MCP + Windows API 都在 1.0 真账已有)
```

### 3.11 R20 preference_learning (2-3 周, in-progress, 跟 R12-LongTermMemory 并行)

```
任务: preference_learning (R20 真实施, 跟 R12-LongTermMemory 并行) 真账对接 + 真实施

必读:
- legacy/donor/apeireth-companion/src/proactive_memory.rs (919 行 REAL, TopicPredictor + PreloadChannel, 0 LLM)
- docs/01-architecture/deferred-slot-activation-preference_learning-spec.md (R15 spec, 617 行, 1:1 翻译 v1)
- docs/01-architecture/c-block-preference_learning-readiness-2026-08-28.md (R20 真账 ready, 318 行)
- docs/01-architecture/r20-preference_learning-research-2026-08-28.md (R20 真调研 257 行)
- cognitive-module-wiring.md L30 DEFERRED → WIRED 状态标 (主代理亲做 spec, 真实施时改 1 行)

必输出:
- 写真账 to: docs/04-internal/r20-preference-learning-implementation-2026-08-28.md (≤ 300 行)
- 真实施代码: 1.0 真账 1:1 翻译 (TopicPredictor + PreloadChannel + PreloadChannel trait + 4 impl + CompositeChannel default_composite_channel, ~700 行)
- cognitive module 加 consolidation_writeback_pipeline + reflection_writeback_pipeline trait spec (主代理 #3 spec)

5 重守门 + LOCKED 0 触碰:
- cargo test + clippy + check (期望 0 副作用)
- 0 触碰 LOCKED 5 项
- 0 引新外部 dep (per 真账 brief 约束, 0 LLM 1:1 翻译)

0 装诚实标:
- 1.0 真账 919 行 1.0 真账 proactive_memory.rs (TopicPredictor + PreloadChannel 1:1 可移植)
- 0 LLM 1:1 翻译 (heuristic-based, 跟 Round 10 真账 brief 0 LLM 一致)
- cognitive-module-wiring.md L30 1 行 doc sync (R20 真实施时改 DEFERRED → WIRED)
- R10 OrganKind 决策 1.0 真账 919 行 1:1 翻译 (借签现有 9 organ trait)
```

---

## 4. 0 装诚实标 + 哲学锚 (per 真账 §1.2)

### 4.1 真账 brief 必含 (per O-5 + S-2 实事求是)

- ✅ 真账 brief 必含 5 重守门 baseline 实测 (per 真账 §1.5)
- ✅ 真账 brief 必含 LOCKED 5 项 0 触碰 verify (per 真账 §1.5)
- ✅ 真账 brief 必含 0 装诚实标 (per 真账 §1.4)
- ✅ 真账 brief 必含 真实施流程 (per 真账 §2 + §1.6)
- ✅ 真账 brief 必含 物种化借签边界 (per 真账 §1.1 vision.md + Round 10 5 R7)
- ✅ 0 引新外部 dep (per 真账 brief 约束, 1:1 翻译优先借签 1.0 真账)

### 4.2 真实施时主代理必亲测 (per Round 15 用户 catch 修订)

- ✅ 真实施时主代理必亲测 (~2-3 天本地实测, 不依赖网络, 本地 working tree 已就位, 0 git clone 必要)
- ✅ 1.0 真账 maturity 补查 (Round 13 主代理亲测 8 .rs, 余 27 项需真实施时主代理亲测)
- ✅ 2.0 真账实测 (本地 working tree, 16 crates workspace)

---

## 5. 留 backlog

### 5.1 主代理亲做 (10 项 spec, ~2 周, 立即可做)

per Round 14 真实施完成计划 §2.1 表 1-10 (主代理亲做 10 项 spec, 不依赖网络).

### 5.2 派 sub-agent 真实施 (11 项, ~12-14 周 critical path)

per Round 14 真实施完成计划 §2.2 表 1-11 (派 sub-agent 真实施 11 项, 12-14 周 critical path, 不依赖网络).

### 5.3 release 流程 (Week 18-20, 1-2 周)

per Round 14 真实施完成计划 §2.3 (release 流程 5 重守门 + ROADMAP §7 + MANIFESTO §14 + git tag v2.0.0 + release announcement).

---

## 6. 真账 brief 模板结束语

per 9 哲学锚 + O-6 永远追求最优 + S-2 实事求是 + 文档工程规范 + 整体系统架构最优:

**主代理亲做 + 派 sub-agent 真实施分工 + release 流程 = v2.0 release 2027-Q3 (修订估时, 真实施 critical path 12-14 周 + release 1-2 周, 总估时 15-18 周 ~4 月, per Round 14 真账).**

**派单 brief 模板 (本文件) 是真账 brief 必含 7 段 (任务 + 必读 + 必输出 + 0 装诚实 + 5 重守门 + LOCKED 0 触碰 + 真实施流程), 派单 11 项统一模板, 主代理亲做 + 派 sub-agent 真实施 + release 流程, 走扩展 trait 接口, 0 引新外部 dep, 真实施时主代理必亲测 (本地 working tree 已就位, 0 git clone 必要).**

---

_Mavis 写于 2026-08-28 Round 15, 用户原话 "做点小的, 边做边更新文档, 注意文档工程规范, 哲学锚, 追求整体系统架构最优" 触发, 写真账 R12-R13-R14 真实施派单 brief 模板 (7 段 + 11 派单具体模板), 按 9 哲学锚 + S-2 实事求是 + 文档工程规范 + 整体系统架构最优. 0 装诚实标: 真实施时主代理必亲测 + 真账 brief 模板必含 7 段 + 5 重守门 baseline 实测 + LOCKED 0 触碰 verify. 本地 working tree 已就位 (per Round 15 用户 catch 修订, 0 git clone 必要)._
