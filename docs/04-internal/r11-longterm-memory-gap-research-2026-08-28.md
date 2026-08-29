# Apeireth v2 长期记忆塑形 gap 真调研 (R11-LongTermMemory, 2026-08-28)

> **作者**: sub-agent R11-LongTermMemory (主代理 Mavis 派单, ≤4h)
> **用途**: 给主代理决策参考 — Apeireth v2 长期记忆塑形 4 项 1.0 vs 2.0 真实施差距 (daily_summary / diary / cross_diary / memory_injection / reflexion / reflection) + consolidation_writeback pipeline 综述 + 真实施路径
> **关系**: 跟 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §1.8 + §2.5 + §3.1 #4 (本调研派单基础) + `apeireth-true-understanding-2026-08-28.md` §1.1 (物种化塑形真理解) + `r7-mio-species-research-2026-08-28.md` §2.2-§5.1 (Mio 日记反思+写回耦合真调研) + `v2-reference-handbook-2026-08-28.md` §1.3 (12 cognitive slot 真账) + master audit L248-253 互补

```
[Document-Meta]
Document:        docs/04-internal/r11-longterm-memory-gap-research-2026-08-28.md
Version:         1.0 (sub-agent R11-LongTermMemory 写于 2026-08-28)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (R11 真调研, 给主代理决策参考)
Author:          sub-agent R11-LongTermMemory (主代理 Mavis 派)
```

---

## 0. 主代理派单真账 + 0 装诚实标

**派单原话 (per 真账 §3.1 #4)**: "daily_summary / diary + cross_diary + memory_injection (长期记忆塑形), 估 2-3 周, 派 sub-agent 真调研 (Mio 真账 §5 #5 已推荐) + 真实施 (跟 R20 + R22 critical path)."

**0 装诚实标 (per O-5 + O-2 前人肩上)**:
- **0 实测**: 未 git clone v2 master branch (per master audit L138), 仅读 1.0 真账 (legacy/donor/) + 2.0 handbook + Mio 真账 + 1.0 vs 2.0 gap 真账推论
- **0 数字漂移**: daily_summary 99 行 / diary 442 行 / cross_diary 301 行 / memory_injection 66 行 / reflexion 497 行 / reflection 329 行 = 1.0 真账行数 (1+1+1+1+1+1 = 6 文件, 实读 wc -l 推论)
- **5 重守门 + LOCKED 0 触碰**: 本调研 doc-only, 不改 src, 不触碰 9 哲学锚 / 13 键 / 3 项不可变脊柱 / Cargo.toml version / R11 baseline, **LOCKED 5 项 0 触碰保证**
- **0 装诱导 prevention**: 不假装 "v2 已实施 daily_summary" / "v2 已实施 cross_diary" — 实际 0 真实施 (per §1.8 + §3.1 #4)

---

## 1. daily_summary / diary — 1.0 vs 2.0 + 物种化借鉴

### 1.1 1.0 现状 (per 真账 + 1.0 真账路径)

| 维度 | 真账 | 来源 |
|---|---|---|
| **路径** | `legacy/donor/apeireth-companion/src/daily_summary.rs` (99 行) + `diary.rs` (442 行) | 真实施 |
| **maturity** | REAL (1:1 可移植) | per 真账 §1.8 + 真账 §3.1 #4 |
| **职责** | daily_summary: 每日纯函数统计 (episodes 计数 + 做梦整合 + 反思记录 + 工具调用 + 摘录 ≤80 字); diary: 日粒度叙事归档 (一天一 JSON `{YYYY-MM-DD}.json`) + 关键词检索 + 注入块生成 (budget 截断) | per daily_summary.rs:11-99 + diary.rs:84-244 |
| **依赖** | `apeireth_core::clock::Clock` (VirtualClock 可快进, 0 真等待) + `chrono` (Datelike) + `serde` + `thiserror` | per diary.rs:21-28 |
| **测试** | 99 + 442 行 = 541 行真账; 6 test cases (daily_summary 1 个, diary 6 个: append 同日 / 跨日 / 空日 / 大小写 / 预算截断 / 非法输入 / 确定性复测) | per daily_summary.rs:78-99 + diary.rs:274-441 |
| **0 装 PASS** | diary.rs:8-19 诚实标注 "注入实接线延后: companion crate 被 N14 阻塞, 本模块只提供机制口" — 跟 v2 0 装诚实标同源 | per diary.rs:15-19 |

### 1.2 2.0 现状 (per §1.8 + §3.1 #4)

| 维度 | 真账 | 来源 |
|---|---|---|
| **路径** | ❌ **0 真实施** (per 真账 §1.8 L129) | per 真账 §1.8 + §3.1 #4 |
| **maturity** | 🔴 **缺** | per 真账 §1.8 + §2.5 |
| **cognitive slot** | 6/12 WIRED (memory_recall/preference_recall/judge/council/self_assessment/memory_writeback) + 6/12 DEFERRED (preference_learning R20 / critic R21 / reflection R22 / planner R23 / orchestrator R24 / perception R14) — 缺 daily_summary / diary | per handbook §1.3 L58-65 |
| **0 装诚实标** | v2 cognitive 缺耦合: self_assessment → memory_writeback 无显式 pipeline (per Mio 真账 §2.3) | per Mio 真账 §2.3 L73-74 |

### 1.3 物种化借鉴 (per Mio 真账 §2.2 + §3.2 + §5.1)

**借鉴点 (核心)**:
1. **日记 = 反思 + 写回合一** ↔ v2 `cognitive.self_assessment` + `cognitive.memory_writeback` 分离实现缺耦合 (per Mio 真账 §2.3 L74)
2. **R22 reflection 真实施直接借鉴** (1 周, handbook §1.3 L62) — 日记反思部分对标 reflection.rs 1:1 翻译 (per handbook §2.5)
3. **R20 preference_learning 真实施直接借鉴** (2-3 周, handbook §1.3 L60) — 日记 → preference 提炼 (per Mio 真账 §2.3 L76)
4. **cognitive module 增维**: 加 `reflection_writeback_pipeline` trait, R20+R22+日记三者**并发** (3-4 周 critical path, per Mio 真账 §2.3 L77)

**真账基础 (per 真账 §3.1 #4)**: "派 sub-agent 真调研 (Mio 真账 §5 #5 已推荐) + 真实施 (跟 R20 + R22 critical path)"

### 1.4 真实施路径 + 估时 + 阻塞

| # | 真实施 | 估时 | 阻塞 | 派单建议 |
|---|---|---|---|---|
| 1 | cognitive module 加 `reflection_writeback_pipeline` trait (LOCKED 5 项 0 触碰, 走扩展 trait 接口) | 1-2 天 (主代理 spec) | 0 | 主代理亲做 spec |
| 2 | `apeireth-cognitive::daily_summary` 1:1 翻译 (从 daily_summary.rs 99 行 + diary.rs 442 行 ≈ 541 行) | 1 周 | spec 冻结 | 派 sub-agent R11-LTM-A 真实施 |
| 3 | R22 reflection 真实施 (跟 daily_summary + reflection.rs 1:1 翻译并行) | 1 周 | 0 | 派 sub-agent R22 真实施 (per handbook §1.3 L62) |
| 4 | cognitive module `reflection_writeback_pipeline` trait 真接线 (self_assessment → memory_writeback) | 1-2 周 | 1-3 done | 派 sub-agent R11-LTM-B 真接线 |

**总估时**: 3-4 周 critical path (跟 R20 + R22 并行), 跟 v2 release 5-7 周不冲突 (per handbook §8.1)

**0 装诚实标**: 真实施前主代理必亲验 (per §4.2 派单顺序), 跑 5 重守门 baseline + LOCKED 0 触碰验证

---

## 2. cross_diary — 1.0 vs 2.0 + 物种化借鉴

### 2.1 1.0 现状

| 维度 | 真账 | 来源 |
|---|---|---|
| **路径** | `legacy/donor/apeireth-companion/src/cross_diary.rs` (301 行) | 真实施 |
| **maturity** | REAL | per 真账 §1.8 L130 |
| **职责** | 跨日记关联索引: 日记条目 (diary 按日归档) × 记忆图事实 (memory_graph 双时态边) 建立确定性关联索引 (共享 token ≥ min_shared 建链); 双向查询 (fact → diary / diary → fact) | per cross_diary.rs:17-128 |
| **纪律** | 自包含模块: 只通过 diary/memory_graph 已有公开接口采集数据 (不改两模块本体); 确定性关联 (共享 token 复用 topic_groups::topic_tokens, CJK bigram + 拉丁词, 停用词切分); 0 向量 0 嵌入 0 远程 | per cross_diary.rs:8-15 |
| **VCP 对照** | VCP diary 关联走嵌入相似度; 我们走确定性 token 交集 (可审计, 同输入必同输出) | per cross_diary.rs:13-15 |
| **测试** | 7 test cases (建链 + 双向查询 + 空关联 + 0 共享 + min_shared 阈值 + 确定性 + 真实接口集成) | per cross_diary.rs:144-300 |

### 2.2 2.0 现状

| 维度 | 真账 | 来源 |
|---|---|---|
| **路径** | ❌ **0 真实施** (per 真账 §1.8 L130) | per 真账 §1.8 + §2.5 |
| **maturity** | 🔴 **缺** | per 真账 §1.8 |
| **依赖** | 缺 `apeireth-cognitive::cross_diary` 模块 + 缺 `apeireth-storage::memory_graph` (storage 抽象层 0 真实施, per 真账 §1.1 L44) | per 真账 §1.1 |
| **0 装诚实标** | storage graph primitives 抽象层 0 真实施 (organ `causal_world_model` ✅ WIRED, storage 抽象层 ❌) | per 真账 §1.1 L44 |

### 2.3 物种化借鉴 (per Mio 真账 + 真账 §2.5)

**借鉴点**:
1. **跨日记关联** = 长期记忆塑形的"纵向编织" — 日记按日 + 跨会话聚合 → "她记得你的连续存在"
2. **确定性 token 交集** (vs VCP 嵌入相似度) ↔ Apeireth 0 装诚实标同源 (可审计, 同输入必同输出)
3. **共享 token ≥ min_shared 阈值** ↔ cognitive confidence / intent_brier 校准 (per 真账 §2.7)
4. **跟 N.E.K.O 五维记忆 + AIRI 长期记忆借鉴** (per 真理解 §3.2 真借鉴)

### 2.4 真实施路径

| # | 真实施 | 估时 | 阻塞 | 派单建议 |
|---|---|---|---|---|
| 1 | `apeireth-storage::memory_graph` storage 抽象层真实施 (per 真账 §1.1 L44 🔴) | 2-3 周 | 0 | 派 sub-agent 真调研 + 真实施 (跟 R11-Storage 真调研并行, per 真账 §3.1 #2) |
| 2 | `apeireth-cognitive::cross_diary` 1:1 翻译 cross_diary.rs (301 行) | 1 周 | #1 done | 派 sub-agent R11-LTM-C 真实施 |
| 3 | `apeireth-cognitive::topic_groups` 1:1 翻译 topic_tokens (CJK bigram + 拉丁词, 停用词切分) | 3-5 天 | 0 | 派 sub-agent 真实施 |
| 4 | CrossDiaryInjector trait 真接线 (挂 assemble.rs 注入管线) | 3-5 天 | #2-3 done | 派 sub-agent 真接线 |

**总估时**: 3-4 周 (跟 #1 storage graph 抽象层 + R20 + R22 并行)

---

## 3. memory_injection — 1.0 vs 2.0 + 物种化借鉴

### 3.1 1.0 现状

| 维度 | 真账 | 来源 |
|---|---|---|
| **路径** | `legacy/donor/apeireth-companion/src/memory_injection.rs` (66 行) | 真实施 |
| **maturity** | REAL | per 真账 §1.8 L131 |
| **职责** | 反幻觉记忆注入 (吸收 hydra EMI/NEC, 重写): 闭世界证据 (编号列表 + 来源标注 + 反幻觉指令); 解决 LLM 检索记忆后幻觉「我记得我们以前聊过…」 | per memory_injection.rs:1-28 |
| **对齐** | hydra: "You do NOT know this user personally... NEVER say 'based on our previous conversations' — that is fabrication" | per memory_injection.rs:7-8 |
| **测试** | 3 test cases (空条目 / 编号闭世界 / 长条目截断 ≤120) | per memory_injection.rs:31-65 |

### 3.2 2.0 现状

| 维度 | 真账 | 来源 |
|---|---|---|
| **路径** | ❌ **0 真实施** (per 真账 §1.8 L131) | per 真账 §1.8 + §2.5 |
| **maturity** | 🔴 **缺** | per 真账 §1.8 |
| **cognitive slot** | 缺反幻觉记忆注入机制 — v2 `cognitive.memory_recall` WIRED (per handbook §1.3 L54) 但缺 memory_injection 渲染层 | per handbook §1.3 |
| **0 装诚实标** | 跟 R20 preference_learning 写入路径相关 (per 真账 §1.8 L131) | per 真账 §1.8 |

### 3.3 物种化借鉴 (per Mio 真账 + 真理解)

**借鉴点**:
1. **反幻觉"闭世界证据"** = 物种化塑形"她不会假装记得" — 物种塑形不是 1 次 LLM 调用, 而是"基于证据块的诚实叙事"
2. **hydra EMI/NEC 精神** ↔ v2 cognitive memory_recall 注入管线 + 反幻觉指令 — 0 假装 OK
3. **跟 N.E.K.O 五维记忆借鉴** (per 真理解 §3.2 — 物种化 memory)
4. **跟 R20 preference_learning 写入路径相关** — preference 提炼后写入, memory_injection 主动读出 (per 真账 §1.8 L131)

### 3.4 真实施路径

| # | 真实施 | 估时 | 阻塞 | 派单建议 |
|---|---|---|---|---|
| 1 | `apeireth-cognitive::memory_injection` 1:1 翻译 memory_injection.rs (66 行, 最简单) | 2-3 天 | 0 | 派 sub-agent R11-LTM-D 真实施 (low hanging fruit) |
| 2 | cognitive.memory_recall slot 真接线 (turn-start 注入管线 + 反幻觉指令) | 1 周 | #1 done | 派 sub-agent 真接线 |
| 3 | R20 preference_learning 写入路径对接 (preference → memory_injection 主动读出) | 1-2 周 | R20 真实施 done (2-3 周) | 派 sub-agent 真接线 (跟 R20 并行) |

**总估时**: 2-3 周 (跟 R20 + R22 并行, low hanging fruit 可最先实施)

---

## 4. reflexion / reflection — 1.0 vs 2.0 + R22 真实施对齐

### 4.1 1.0 现状 (双模块, 互补)

| 维度 | reflexion.rs (497 行) | reflection.rs (329 行) | 来源 |
|---|---|---|---|
| **路径** | `legacy/donor/apeireth-companion/src/reflexion.rs` | `legacy/donor/apeireth-companion/src/reflection.rs` | per 真账 §1.8 |
| **maturity** | REAL | REAL | per 真账 §1.8 |
| **职责** | **E1 口头强化闭环 (Reflexion 式)**: 失败轨迹 (DecisionRejected / ValidationFailed / ExperienceFailed) → CRITIC 反思文本 → 反思记忆 → 同类任务重试注入 (按 task_type 相似度精确 2 > 子串 1) | **反思周期调度 (接 daemon)**: 4 阶段状态机 (Triggered → Reflecting → Consolidating → Concluded 自动重触发); 周期到 OR 重要事件积累 (importance > 150) 触发; 写回真 SQLite (reflect-* episode); 可选深度反思器 (LLM) + N4 元自学习 (thought_cluster 回读) | per reflexion.rs:1-19 + reflection.rs:1-9 |
| **依赖** | PathBuf + Arc<dyn Critic> + serde + thiserror | apeireth_core::clock::Clock + apeireth_memory (CoreEpisode / EpisodeStore / ReflectionCycleScheduler / ReflectionPhase / SqliteMemoryStore) + chrono + async_trait | per reflexion.rs:21-28 + reflection.rs:10-16 |
| **0 装 PASS** | reflexion.rs:16-19 诚实标注 "LLM 版 CRITIC 未接 (trait 口已留) + 失败事件实接线未接 + 注入块消费侧未接线" — 跟 v2 0 装诚实标同源 | reflection.rs:7-9 诚实标注 "状态机与写回是真实机制; 反思内容 (LLM 深度反思) 由上层注入" | per reflexion.rs:16-19 + reflection.rs:7-9 |
| **测试** | 6 test cases (record_failure 三类型 / critic_step 增量 / retry_injection 排序 + 截断 / 空库全路径 / 确定性复测) | 5 test cases (周期未到 / 周期到写回 / 状态机 4 阶段 / N4 元自学习附历史思维链 / 未接 thought_reader 保持 plain) | per reflexion.rs:308-496 + reflection.rs:194-328 |

### 4.2 2.0 现状

| 维度 | 真账 | 来源 |
|---|---|---|
| **reflexion.rs** | ❌ **0 真实施** (per 真账 §1.8 + §2.6, 不在 cognitive slot 12 列表, 1:1 翻译未派单) | per 真账 §1.8 + handbook §1.3 |
| **reflection.rs** | 🟡 **DEFERRED INTO SELF-ASSESSMENT → R22 派单 (1 周)** — 1:1 翻译 v1 reflection.rs (per handbook §1.3 L62 + §2.5 L157) | per handbook §1.3 L62 + §2.5 |
| **maturity** | reflexion = 🔴 **缺**; reflection = 🟡 DEFERRED | per 真账 §1.8 + handbook §1.3 |
| **cognitive slot** | `cognitive.reflection` 已规划 DEFERRED INTO SELF-ASSESSMENT → R22; 但缺 reflexion 强化闭环 (失败轨迹 → 反思 → 任务重试注入) | per handbook §1.3 L62 |

### 4.3 物种化借鉴 (per Mio 真账 §2.3 + §3.2 + 真理解)

**借鉴点 (核心)**:
1. **反思 + 写回耦合 (reflexion.rs + reflection.rs)** ↔ v2 `cognitive.self_assessment` (Judge-backed, AfterTurn) + `cognitive.memory_writeback` (AfterTurn, append-only Episodes) — R22 reflection 1:1 翻译直接对接 (per handbook §1.3 L62 + §2.5 L157)
2. **重要事件积累触发 (importance > 150, Generative Agents 吸收)** ↔ v2 self_assessment Judge + cognitive.reflection 触发条件
3. **N4 元自学习 (thought_cluster 回读 ≤3 簇 × 最新 1 篇 × 400 字)** ↔ v2 cognitive.thought / thought_cluster (per 真账 §1.8 L135 🔴 缺)
4. **E1 口头强化闭环 (失败轨迹 → 反思 → 同类任务重试注入)** = 物种化塑形"她从失败里学" — 跟 vision.md "共同生活塑形" 对接

### 4.4 真实施路径

| # | 真实施 | 估时 | 阻塞 | 派单建议 |
|---|---|---|---|---|
| 1 | **R22 reflection 真实施** — 1:1 翻译 reflection.rs (329 行) → `apeireth-cognitive::reflection` + DEFERRED INTO SELF-ASSESSMENT (per handbook §1.3 L62) | 1 周 | 0 | 派 sub-agent R22 真实施 (跟 daily_summary 并行) |
| 2 | **R21 critic 真实施** — 1:1 翻译 reflexion.rs RuleCritic (Critic trait) → `apeireth-cognitive::critic` + DEFERRED INTO JUDGE (per handbook §1.3 L61) | 1 周 | 0 | 派 sub-agent R21 真实施 (跟 R22 并行) |
| 3 | **reflexion 闭环真实施** — reflexion.rs ReflexionStore + record_failure + critic_step + retry_injection 1:1 翻译 → `apeireth-cognitive::reflexion` (1:1 翻译 497 行) | 1-2 周 | R22 done | 派 sub-agent R11-LTM-E 真实施 |
| 4 | **N4 元自学习真接线** — thought_cluster 1:1 翻译 (per 真账 §1.8 L135 🔴 缺, 跟 R23+ 真实施并行) | 1 周 | thought_cluster 1:1 翻译 done | 派 sub-agent 真接线 |

**总估时**: 3-4 周 (跟 R20 + R22 + daily_summary 并行, R22 优先 1 周)

**0 装诚实标**: 真实施前主代理必亲验 (per §4.2 派单顺序), 跑 5 重守门 baseline + LOCKED 0 触碰验证; LLM 版 CRITIC trait 口可保留 0 装 PASS (per reflexion.rs:16-19)

---

## 5. consolidation_writeback pipeline 综述

### 5.1 物种化塑形 pipeline (整合 4 项 + R20 + R22)

```
daily_summary (日粒度统计)
    ↓
cross_diary (跨日记关联索引, shared_tokens 共享)
    ↓
memory_injection (反幻觉闭世界证据)
    ↓
reflection (R22, 周期到 OR importance > 150 触发)
    ↓  (可选深度反思器 LLM)
reflexion (R21 + 强化闭环, 失败轨迹 → 反思 → 重试注入)
    ↓
memory_writeback (append-only Episodes, cognitive.slot #6 WIRED)
    ↓
consolidation (跨日记 / 记忆图 / 反思记忆 聚合, 物种化塑形固化)
```

**真实施路径 (1 整套 critical path)**:
1. **cognitive module consolidation_writeback_pipeline trait 增维** (主代理 spec, 1-2 天) — LOCKED 5 项 0 触碰, 走扩展 trait 接口
2. **per §1.4 + §2.4 + §3.4 + §4.4 真实施派单并发** (3-4 周)
3. **CognitiveOrchestrator 真接线** (AfterTurn 钩子挂 consolidation_writeback_pipeline, 1-2 周)

**总估时**: 4-6 周 critical path (跟 R20 + R22 + R23 planner 并行, 不冲突 per handbook §8.1)

### 5.2 跟 vision.md "物种而非个体 + 记忆/偏好/好奇形状被共同生活塑形" 对接

| vision.md 真账 | consolidation_writeback pipeline 真账 | 借鉴点 |
|---|---|---|
| "物种而非个体" | per-user memory / preference / curiosity / emotional timeline 物理独立 | consolidation per-user pipeline (per 真理解 §1.1.3) |
| "记忆/偏好/好奇形状被共同生活塑形" | daily_summary → cross_diary → memory_injection → reflection → memory_writeback → consolidation | 长期记忆塑形全链路 (本调研 5 项 + R20 + R22 整合) |
| "同一个 Apeireth, 不同的'人生'" | per-user consolidation pipeline 实例独立 (本地优先 + portable binary, per Mio 真账 §3.1) | 物种化 frontend + backend 双形态 (per 真理解 §1.1.3) |

### 5.3 真实施路径 (跟 §5.1 整合)

| # | 真实施 | 估时 | 阻塞 | 派单建议 |
|---|---|---|---|---|
| 1 | cognitive module `consolidation_writeback_pipeline` trait 增维 + spec | 1-2 天 (主代理亲做) | 0 | 主代理亲做 spec (LOCKED 0 触碰) |
| 2 | daily_summary + diary + cross_diary + memory_injection + reflection + reflexion 1:1 翻译真实施 (6 sub-agent 并行) | 3-4 周 | #1 spec done | 派 6 sub-agent 真实施 (per §1.4 + §2.4 + §3.4 + §4.4) |
| 3 | CognitiveOrchestrator AfterTurn 钩子挂 consolidation_writeback_pipeline (真接线) | 1-2 周 | #2 done | 派 sub-agent 真接线 |
| 4 | E2E + 5 重守门 baseline (per handbook §8.1 #5) | 1-2 周 | #3 done | 派 sub-agent D 真实施 (per handbook §10.5) |

**总估时**: 6-9 周 critical path (跟 handbook §8.1 R20+R22+R23 6-10 周 + R21+R14 + v2 release 5-7 周不冲突)

---

## 6. 主代理决策建议

### 6.1 4 项优先级排序 + 真实施 brief

| # | 优先级 | 项 | 真实施 brief | 估时 | 阻塞 |
|---|---|---|---|---|---|
| 1 | 🟢 P0 | **memory_injection** (66 行, low hanging fruit) | `apeireth-cognitive::memory_injection` 1:1 翻译, 反幻觉闭世界证据 + cognitive.memory_recall 注入管线对接 | 2-3 天 | 0 |
| 2 | 🟢 P0 | **R22 reflection** (per handbook §1.3 L62) | `apeireth-cognitive::reflection` 1:1 翻译 reflection.rs 329 行, DEFERRED INTO SELF-ASSESSMENT | 1 周 | 0 |
| 3 | 🟢 P0 | **daily_summary + diary** (541 行, 跟 Mio 日记真调研对接 per Mio 真账 §3.2) | `apeireth-cognitive::daily_summary` + `apeireth-cognitive::diary` 1:1 翻译, 真接线 reflection_writeback_pipeline trait | 1-2 周 | #1-2 |
| 4 | 🟢 P0 | **reflexion 闭环** (497 行, 跟 R21 并行) | `apeireth-cognitive::reflexion` 1:1 翻译, 强化闭环 + task_type 重试注入 | 1-2 周 | R22 done |
| 5 | 🟡 P1 | **cross_diary** (301 行, 依赖 storage graph) | `apeireth-cognitive::cross_diary` 1:1 翻译, 依赖 `apeireth-storage::memory_graph` storage 抽象层 (per 真账 §1.1 L44 🔴) | 1 周 + 2-3 周 storage graph | storage graph 真实施 |

### 6.2 借鉴链 (per O-2 前人肩上 + Mio 调研)

1. **1.0 真账** (legacy/donor/apeireth-companion/src/) — 1:1 翻译 6 模块 ~1734 行真账 (99+442+301+66+497+329)
2. **Mio 真调研** (`r7-mio-species-research-2026-08-28.md` §2.2-§5.1) — 日记反思+写回耦合范式
3. **N.E.K.O 五维记忆** (per 真理解 §3.2) — 物种化 memory 借鉴
4. **AIRI 长期记忆** (per 真理解 §3.2) — 物种化 frontend + backend 借鉴
5. **hydra EMI/NEC** (per memory_injection.rs:7-8) — 反幻觉记忆注入精神吸收
6. **Generative Agents 重要事件积累** (per reflection.rs:91-103) — importance > 150 触发反思
7. **VCP RAGDiaryPlugin** (per diary.rs:1 + cross_diary.rs:1) — diary 精神吸收 (0 装诚实改造, 走确定性 token 交集)

### 6.3 0 装诚实标 + 5 重守门 + LOCKED 0 触碰

| 失守 | 详情 | 修法 |
|---|---|---|
| **0 实测** | 本调研未 git clone v2 master branch, 仅读 1.0 真账 (legacy/donor/) + 2.0 handbook + Mio 真账 + 1.0 vs 2.0 gap 真账推论 | 真实施前主代理必亲验: git clone v2 main + grep 跟 1.0 真对照 (per 真账 §4.3 #5 + §5) |
| **0 数字漂移** | 1.0 真账行数 (99+442+301+66+497+329 = 1734 行) 实测基于 wc -l 推论 | 主代理亲验时实测 wc -l (不假装 OK) |
| **0 装诱导 prevention** | 不假装 "v2 已实施 daily_summary / cross_diary / memory_injection / reflexion" — 实际 0 真实施 (per 真账 §1.8 + §3.1 #4) | 真账明确标注 0 真实施 + 真实施前必亲验 |
| **5 重守门 baseline** | cargo test --workspace --locked = 1739 passed / cargo clippy --workspace --all-targets --locked -- -D warnings = 0 warning / git diff LOCKED 5 项 = 0 行 / legacy compat path < 100 (现 36) / 9 哲学锚表头 0 减 | 真实施前后必跑 5 重守门 (per handbook §4) |
| **LOCKED 0 触碰** | 9 哲学锚本体 (eight_anchors.rs:58-79) + 13 键 (philosophy.rs:142) + 3 项不可变脊柱 (onion.rs:249) + workspace.version (Cargo.toml:44) + R11 baseline 数字 (cognitive.rs 等) — 5 项 0 触碰 | cognitive module 增维走扩展 trait 接口, 不改本体; 真实施后 git diff LOCKED 5 项 = 0 行实测 |

### 6.4 派单顺序 + critical path (per O-6 总体最优)

**P0 真实施顺序 (1-4 周)**:
1. 主代理亲做 spec: cognitive module consolidation_writeback_pipeline trait 增维 + reflection_writeback_pipeline trait 增维 (1-2 天, LOCKED 0 触碰)
2. 派 sub-agent R11-LTM-D 真实施 memory_injection (2-3 天, low hanging fruit 优先)
3. 派 sub-agent R22 真实施 reflection (1 周, per handbook §1.3 L62 + §2.5 L157)
4. 派 sub-agent R21 真实施 critic + reflexion 强化闭环 (2 周, per handbook §1.3 L61 + reflexion.rs 1:1 翻译)
5. 派 sub-agent R11-LTM-A 真实施 daily_summary + diary (1-2 周, 跟 Mio 日记真调研对接)
6. 派 sub-agent 真实施 cognitive reflection_writeback_pipeline trait 真接线 (1-2 周)

**P1 真实施顺序 (3-6 周)**:
7. 派 sub-agent 真调研 + 真实施 storage graph 抽象层 (2-3 周, per 真账 §1.1 L44 🔴 + §3.1 #2)
8. 派 sub-agent R11-LTM-C 真实施 cross_diary + topic_groups (1 周 + 3-5 天, 跟 #7 并行)
9. 派 sub-agent D 真实施 E2E + 5 重守门 baseline (1-2 周, per handbook §8.1 #5)

**总 critical path**: 5-7 周 (跟 handbook §8.1 R20+R22+R21 5-7 周 + v2 release 5-7 周并行, 不冲突)

### 6.5 修订 release 路径

- **v2.0.0 release 估时**: 原 2027-Q1-Q2 (per MANIFESTO §14), 长期记忆塑形 4 项 P0 真补估 5-7 周 critical path, release 估时维持 2027-Q2 (per 真账 §6.3 release 路径修订 6-9 月)
- **修订 ROADMAP §7 总进度**: 长期记忆塑形 4 项真实施 +1% (75-80% → 76-81%)
- **修订 MANIFESTO §14 release timeline**: 维持 2027-Q2 (不延期)

---

_R11-LongTermMemory 写于 2026-08-28 Round 11, ≤4h, 长期记忆塑形 4 项 (daily_summary / diary / cross_diary / memory_injection / reflexion / reflection) + consolidation_writeback pipeline 综述真调研, 跟 Mio 真账 P0 调研对接, 借鉴 1.0 真账 1:1 翻译 ~1734 行 + R20+R22+R21 并行, 0 装诚实 + 5 重守门 + LOCKED 0 触碰 保证, 真实施前主代理必亲验. 真账就位._