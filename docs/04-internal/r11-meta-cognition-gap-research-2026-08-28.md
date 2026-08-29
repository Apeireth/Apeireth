# R11 元认知 gap 真调研 — meta_thinking + reflexion + reflection + thought_cluster + intent_brier + confidence (2026-08-28)

> **作者**: sub-agent R11-MetaCognition (主代理 Mavis 派单 Round 11, brief "1.0 vs 2.0 功能对比 + 反思+元认知 gap 真调研")
> **用途**: 主代理真账 1.0 vs 2.0 反思+元认知 6 模块 gap, 给 v2 release 前必补/必决策/必调研 清单, 跟 R22 reflection 真实施 + R11 长期记忆塑形调研对接
> **关系**: 跟 `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §2.6 + `apeireth-true-understanding-2026-08-28.md` + `v2-reference-handbook-2026-08-28.md` §1.3 + `cognitive-module-wiring.md` + `r21-r24-r12-research-2026-08-28.md` + `backlog.md` (E1/N4/W6) + `r7-mio-species-research-2026-08-28.md` + `r7-neko-species-research-2026-08-28.md` 互补

```
[Document-Meta]
Document:        docs/04-internal/r11-meta-cognition-gap-research-2026-08-28.md
Version:         1.0 (sub-agent R11-MetaCognition 写于 Round 11)
Last-Modified:   2026-08-28
Status:          🟢 活跃 (反思+元认知 gap 真调研, 6 模块 1:1 翻译 vs v2 release critical path)
Author:          sub-agent R11-MetaCognition
```

---

## 0. 用户 directive 真账 + 主代理 brief 真账

**用户原话 (Round 11)**: "看 1.0 缺什么, 2.0 最终功能应该和 1.0 相同, 但架构不同而已"
**主代理派单 brief** (per `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §2.6 + §4.2 #5): 反思+元认知 6 模块 (meta_thinking + reflexion + reflection + thought_cluster + intent_brier + confidence) gap 真调研 + 1.0 真账 + 2.0 现状 + 0 装诚实标 + 真实施路径 + 主代理决策建议.

**关键 directive** (per O-5 + S-2): v2.0 release 必须 = v1.0 功能全集 + 架构升级 — release 阻断; 反思+元认知 = 认知层核心循环 + 物种化核心 (per `apeireth-true-understanding-2026-08-28.md` §1.1 "她").

---

## 1. meta_thinking — 元思考递归链

### 1.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/meta_thinking.rs` 643 行)
**REAL**: `MetaThinkingChain` (L176) 阶段化递归 (上一段产出 → 下一段输入, VCP MetaThinkingManager.js 吸收); `DEFAULT_MAX_DEPTH=10`; `CycleDetected` 熔断; `EmptyThought` 降级; `ThinkerHalted` 错误熔断; `MetaThinker` trait (L78); `ReflectionMetaThinker` (L335) + `ChainReflectionThinker` (L341); `MetaChainResult::to_markdown()` (L145); `save_to_cluster()` (L323). **0 装 PASS (L22-25 显式)**: reflection.rs 接线未做, 真 LLM MetaThinker 留部署层. **8 单测全绿**.

### 1.2 2.0 现状 (0 真实施)
**真账**: `cognitive.reflection` = DEFERRED INTO SELF-ASSESSMENT (handbook §1.3 L62) → R22 派单 1 周; `cognitive.rs` L1118 "Marker showing that this release has no separate reflection or planner module"; meta_thinking trait 0 真实施 (no crate, no OrganTrait); 借鉴链已有 (per `team-work-doc.md` L138 "元思考递归链 ✅ Rust 落点 companion meta_thinking.rs (提交 6fcd36c2; reflection 接线待 backlog N15)").
**0 装诚实标**: meta_thinking.rs 提交 6fcd36c2 落 donor v1 路径, v2 active workspace 没移植.

### 1.3 真实施路径 (P0 跟 R22 reflection 同期, 1 周)
借鉴 v1 `apeireth-companion/src/meta_thinking.rs:1-643` + VCP MetaThinkingManager.js. 新 crate `crates/engine/reflection/` (per `r21-r24-r12-research-2026-08-28.md` L94) + MetaThinkingChain 1:1 翻译 L176-330 (zero LLM) + `ReflectionMetaThinker` + `ChainReflectionThinker` 1:1 翻译 + 复用 thought_cluster `ThoughtClusterReader` trait + OrganKind::Reflection variant (加不破 9 现有) + 单测 1:1 翻译 v1 8 测试 + 集成 `cognitive.reflection` slot + 5 重守门 baseline.

**主代理决策**: 🔴 P0 critical path, 跟 R22 同期 (1 周). 物种化塑形核心 (元思考 = "思考的再思考").

---

## 2. reflexion — 反思循环 (口头强化闭环)

### 2.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/reflexion.rs` 497 行)
**REAL**: 痛点 = 反思有周期无喂回 (L2-4); 职责链 4 段 (L7-14): 失败轨迹采集 → CRITIC 反思 → 反思记忆 → 重试注入; `FailureKind` 三类 (L32-39) `DecisionRejected` / `ValidationFailed` / `ExperienceFailed`; `ReflexionStore` (L141) `record_failure` (L169) / `critic_step` (L197) / `retry_injection` (L226); `Critic` trait (L101) LLM 版预留口; `RuleCritic` (L107) 确定性规则版先行; `retry_injection` 相似度 = 精确 2 > 子串 1, 同分取最新, 字符预算截断 (L226-276). **0 装 PASS (L17-19 显式)**: LLM 版 CRITIC 未接; 失败事件实接线未接; 注入块消费侧未接线. **5 测试全绿**.

### 2.2 2.0 现状 (已入库 E1, but v2 cognitive slot 0 接)
**真账** (per `backlog.md` L101): ✅ E1 完成 (7285995c agent_orchestrator2): reflexion.rs 自包含, 5 单测全绿; **但是**: v2 active workspace `grep reflexion` 在 `crates/` 0 命中 (§0 grep 实测); v1 donor 路径已入库 legacy/, 没移植 active workspace; 没注册 cognitive slot.
**0 装诚实标**: v2 没移植 reflexion; donor E1 commit 仅入库 legacy/, active workspace 0 触及. 物种化反思循环 = 0 真实施.

### 2.3 真实施路径 (P1 物种化, post-R22, 1-2 周)
借鉴 v1 `apeireth-companion/src/reflexion.rs:1-497` + Reflexion 论文. 新 crate `crates/engine/reflexion/` + ReflexionStore 1:1 翻译 v1 L141-277 (zero LLM, JSON 落盘) + RuleCritic 1:1 翻译 + LlmCritic trait 留部署层 + 失败事件接线 = `cognitive.judge` AfterModelResponse (verdict = retry/stop) + `cognitive.self_assessment` AfterTurn (score < 阈值) + 重试注入接线 = `cognitive.preference_recall` + `cognitive.memory_recall` TurnStart (预算 ≤ N chars) + 单测 1:1 翻译 5 测试 + 5 重守门 + LOCKED 0 触碰.

**主代理决策**: 🟡 P1 (post-R22, 1-2 周). 物种化塑形核心 (反思循环 = "她").

---

## 3. reflection — 反思周期 (物种化塑形反思)

### 3.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/reflection.rs` 329 行)
**REAL**: 反思周期调度 (接 daemon, L1); 每周期 (默认 24h) 推进 4 阶段 `Triggered→Reflecting→Consolidating→Concluded`, Concluded 自动重触发; `ReflectionScheduler` (L27) 状态机 + 写回真 SQLite 【反思周期】episode; `ReflectionReflector` trait (L21) LLM 深度反思 (lib 0 LLM 依赖); `with_thought_reader` (L65) N4 元自学习读取口, 反思附历史思维链 (≤3 簇×最新 1 篇×400 字) "思考的再思考"; 触发条件 (L92-103) 周期到 OR 最近 100 条 importance 和 > 150 (Generative Agents 吸收). **0 假装 (L8)**: 状态机与写回真实机制; 反思内容由上层注入. **4 测试全绿**.

### 3.2 2.0 现状 (cognitive.reflection DEFERRED, R22 critical path)
**真账**: `cognitive.reflection` = DEFERRED INTO SELF-ASSESSMENT → R22 派单 1 周 (per handbook §1.3 L62 + `cognitive-module-wiring.md` L32 "current-turn assessment is distinct from durable memory; **long-term reflection pipeline remains future work**"); `cognitive.rs` L1118 "included in AfterTurn self-assessment" (短时 current-turn, 不是长程 pipeline); R22 spec **待写** (per `r21-r24-r12-research-2026-08-28.md` L125 "R17 spec 待写"); R22 路径真账 (r21-r24 L82): 新 crate `crates/engine/reflection/` + OrganKind::Reflection variant + ReflectionScheduler 1:1 翻译 + ReflectionReflector trait lib 0 LLM + N4 thought_reader.

### 3.3 真实施路径 (R22 critical path, 1 周, 主代理亲做 R17 spec)
借鉴 v1 `apeireth-companion/src/reflection.rs:1-329` + Generative Agents importance 触发. (1) R17 spec 接力 (主代理派 sub-agent 30-45 分钟仿 R15 模板, 6 节 + 5 LOCKED + 0 装诚实 4 块, 新 crate + OrganKind::Reflection variant, **cognitive.reflection slot OWN** — 当前 SELF-ASSESSMENT owner 错账, R22 真实施要 OWN). (2) 真实施 1 周: ReflectionScheduler 1:1 翻译 + ReflectionReflector trait + N4 thought_reader 接入 + 写回 SQLite 【反思周期】episode (复用 v1 写回路径). (3) 0 触碰 LOCKED 5 项 + 9 哲学锚 0 减 (DEFERRED → WIRED 是 doc sync, 不改 slot). (4) 0 装诚实标: 真 LLM 反思留部署层 trait, lib 0 LLM 依赖.

**主代理决策**: 🔴 P0 critical path (1 周). 物种化塑形核心 (per `r7-mio-species-research-2026-08-28.md` L166 "R22 reflection 真实施直接借鉴" + `r7-neko-species-research-2026-08-28.md` L260 "reflective 维借鉴"). 主代理必亲写 R17 spec + 派单.

---

## 4. thought_cluster — 思考聚类 (元自学习读取口)

### 4.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/thought_cluster.rs` 522 行)
**REAL**: 思维簇管理 + 元自学习读取口 (backlog N4, L1-2) — AI 思维链文件按主题聚簇落盘; `ThoughtClusterManager` (L79) create_file / list_clusters / read_cluster / register_chain / read_chain / edit_file / search; 簇 = root 下以「簇」结尾的目录, 条目 = `{YYYY-MM-DD}-{seq:03}.md`; 链 = `meta_thinking_chains.json` `{"chains": {链名: [簇, ...]}}` (VCP 格式一致); `ThoughtClusterReader` trait (L69) 反思/做梦消费思维簇的统一 trait 口 (reflection.rs `with_thought_reader` / dream.rs `with_thought_reader` 注入). **0 装 PASS (L17-19 显式)**: 写入侧需 LLM 在部署层经工具调用驱动. **8 测试全绿**.

### 4.2 2.0 现状 (已入库 N4, but v2 cognitive slot 0 接)
**真账** (per `backlog.md` L58): ✅ N4 完成 (任务 eac874d5): thought_cluster.rs 模块入库 (ThoughtClusterManager 全 API + ThoughtClusterReader trait 口, 8 单测全绿) + reflection.rs/dream.rs with_thought_reader 注入点. **但是**: v2 active workspace `grep thought_cluster` 在 `crates/` 0 命中 (§0 grep 实测); v1 donor 已入库 legacy/, 没移植 active workspace.

### 4.3 真实施路径 (P0 跟 R22 reflection 同期, 1 周)
借鉴 v1 `apeireth-companion/src/thought_cluster.rs:1-522` + VCP ThoughtClusterManager. 新 crate `crates/engine/thought_cluster/` + ThoughtClusterManager 1:1 翻译 L79-292 (zero LLM) + ThoughtClusterReader trait 1:1 翻译 L69-76 (消费侧) + 集成 R22 reflection with_thought_reader 注入 (per v1 reflection L65-72) + meta_thinking save_to_cluster (per v1 meta_thinking L323) + 单测 1:1 翻译 8 测试 + 5 重守门 + LOCKED 0 触碰.

**主代理决策**: 🔴 P0 critical path (跟 R22 + meta_thinking 三件套, 1 周). 物种化塑形核心 (思考聚类 = 物种化长期记忆).

---

## 5. intent_brier — Brier 校准意图 (跟 W1/W2/W3 world_model)

### 5.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/intent_brier.rs` 817 行)
**REAL**: W6 意图理解准确率 Brier 自我诊断 (L1); 哲学 (L3-7) "对主人意图的理解准确率": 每轮对话后预测概率 vs 事后真实意图命中 → Brier score; 与 oracle 差异 (L9-14) oracle 是「外部世界事件是否会发生」, W6 是「我是否猜对主人意图」, 公式同源领域不同; 公式同 `oracle.rs::Forecast::resolve` `(p-1)² if hit else p²`; 滚动窗口 (L16-19) 默认 30/100/300 轮三档; 诊断输出 (L21-24) 按 `domain` 分组 → mean_brier → 识别低校准领域 (mean > threshold = 0.25); 数据结构 IntentPrediction + FeedbackOutcome (Agree/Correct/Silent) + IntentRecord + IntentLedger (滑动记录簿, cap 1000) + BrierWindow + BrierTrend (Improving/Stable/Degrading, 5% delta 阈值) + DomainDiagnostic + IntentDiagnosticReport + render_report. **31 测试全绿**.

### 5.2 2.0 现状 (已入库 W6, but cognitive slot 0 接)
**真账** (per `backlog.md` L402): ✅ W6 完成 (任务 aa65a995, backend_engineer2): intent_brier.rs 新模块 31 新测试 + 576 旧测试 = 607/607 全绿; oracle API 0 改动向后兼容. **但是**: v2 active workspace `grep intent_brier` 在 `crates/` 0 命中; v1 donor 已入库 legacy/, 没移植 active workspace.
**0 装诚实标**: v2 没移植 intent_brier; donor W6 commit 仅入库 legacy/, active workspace 0 触及. 认知层 Brier 校准意图 = 0 真实施.

### 5.3 真实施路径 (P1 跟 W1/W2/W3 world_model Brier 校准并行, 1-2 周)
借鉴 v1 `apeireth-companion/src/intent_brier.rs:1-817` + oracle.rs Brier 公式. 新 crate `crates/engine/intent_brier/` + IntentPrediction + FeedbackOutcome + IntentRecord + IntentLedger 1:1 翻译 L40-191 + brier_score + mean_brier + compute_window + compute_trend + domain_diagnostics + compute_report + render_report 1:1 翻译 L196-431 + 集成 `cognitive.self_assessment` slot (AfterTurn, 反馈 = Agree/Correct/Silent; 反馈源 = Judge verdict) + 集成 `cognitive.preference_recall` (TurnStart, render_report 注入 system prompt, 预算 ≤ N chars) + 跟 W1/W2/W3 world_model organ 共享 brier_score 公式 (per v1 L11-14 + `organ::world_model.rs` L159-160 "CalibrationStrength 本地 enum 0 依赖 apeireth-confidence") + 单测 1:1 翻译 31 测试.

**主代理决策**: 🟡 P1 (跟 W1/W2/W3 world_model Brier 校准并行, 1-2 周). 物种化塑形核心 (Brier 校准意图 = 物种化 "她").

---

## 6. confidence — 置信度 (Beta-Binomial 数学化自信度)

### 6.1 1.0 真账 (per `legacy/donor/apeireth-companion/src/confidence.rs` 177 行)
**REAL**: Beta-Binomial 置信度 (吸收 hydra genome 置信度数学, 重写, L1); 用途: 能力提案/自测的数学化自信度 — `conf=91% [89%-93%] obs=25000 strength=STRONG`; 模型: 成功数 k / 观察数 n, 先验 (α₀, β₀) = (1, 1) (均匀), 后验均值 `E[θ]=(α₀+k)/(α₀+β₀+n)`, 区间用 Wilson 近似; `BetaBinomial` struct (L11) — alpha0/beta0/successes/observations + `observe()` / `mean()` / `interval95()` / `strength()` / `report()`; `Strength` enum (L31) Weak (0-4) / Moderate (5-49) / Strong (50-999) / VeryStrong (1000+). **4 测试全绿**.

### 6.2 2.0 现状 (本地 Calibr­ationStrength 简化复刻, but v1 BetaBinomial trait 0 移植)
**真账** (per `crates/engine/organ/src/world_model.rs` L159-180 + §0 grep 实测): ✅ v2 `organ::world_model.rs` L159-160 显式说 "v1 的 strength 字段引用 `crate::confidence::Strength` enum; **v2 organ crate 不依赖 `apeireth-confidence`** (0 装诚实 + 依赖最小). 改用同语义 `CalibrationStrength` 本地 enum" (L163-167 Zero/Weak/Moderate/Strong); L275 "BetaBinomial (1,1) 先验 + 观测 successes/total" — v2 organ 内嵌 BetaBinomial 简化版 (不依赖 v1 confidence crate); `cognitive.council` slot 多意见加权 (per `v2-unabsorbed-features.md` L72 AdvisorVerdict.confidence: Option<f64>); **但是**: v1 BetaBinomial trait **0 移植** (active workspace 没 crate).
**0 装诚实标**: v2 organ::world_model 自含简化 BetaBinomial (主动合 0 装诚实 + 依赖最小); v1 confidence.rs self-contained BetaBinomial 数学化 trait 不止 organ 用 — 还可用于 cognitive.judge / cognitive.council / cognitive.self_assessment / cognitive.preference_learning 多处; 当前 v2 cognitive.council confidence 是 `Option<f64>`, **不基于 BetaBinomial 数学化模型**.

### 6.3 真实施路径 (P1 跟 cognitive.council + judge 绑定, 1 周)
借鉴 v1 `apeireth-companion/src/confidence.rs:1-177` + hydra genome + v2 organ::world_model::CalibrationStrength 本地简化版 (L159-292). 新 crate `crates/engine/confidence/` (per R10 OrganKind variant, 跟 organ::world_model::CalibrationStrength 区分 — confidence crate = 通用 BetaBinomial trait, organ crate = world_model specific 本地简化版) + BetaBinomial + Strength 1:1 翻译 L9-110 (zero LLM) + 集成 cognitive.council (AdvisorVerdict.confidence: Option<f64> → Option<BetaBinomial>) + 集成 cognitive.judge (Judge critique 置信度基于 BetaBinomial, 不再 LLM 自报) + 集成 cognitive.self_assessment (AfterTurn self_assessment 置信度) + **organ::world_model 协同**: 保留本地 Calibr­ationStrength (per 0 装诚实 + 依赖最小), 跟通用 confidence crate 双向 optional borrow + 单测 1:1 翻译 4 测试.

**主代理决策**: 🟡 P1 (跟 cognitive.council + judge 绑定, 1 周). 数学化自信度 = 物种化塑形核心 (per v1 L4 "不依赖 LLM 自报").

---

## 7. 反思+元认知综述 (整合 6 项)

### 7.1 6 项 = 认知层循环

```
┌──────────────────────────────────────────────────────────────────┐
│  认知层循环 (per 6 项真账)                                          │
├──────────────────────────────────────────────────────────────────┤
│  meta_thinking ──→ thought_cluster ──→ reflection               │
│  (元思考递归链)    (思考聚类)          (反思周期写回)             │
│       ↑                                  │                       │
│       │                                  ↓                       │
│  confidence ←─── cognitive.council/judge ── intent_brier          │
│  (数学化自信度)  (多意见加权)              (Brier 校准意图)         │
│       └─────────────── reflexion ──────────────────┘             │
│                  (反思循环: 失败 → 反思 → 重试注入)                │
└──────────────────────────────────────────────────────────────────┘
```

整合真账: meta_thinking + thought_cluster = "思考的再思考" (per v1 meta_thinking L19 save_to_cluster + thought_cluster L5 元自学习); reflection = 反思周期 (daemon 白昼+夜间+反思); cognitive.council + judge = 多意见加权 + 反思证据 (per `cognitive-module-wiring.md` L26-28); intent_brier = 校准意图理解准确率 (per v1 L4); confidence = 数学化自信度 (per v1 L4); reflexion = 失败→反思→重试注入 (per v1 L2-4).

### 7.2 跟 R22 reflection + R11 长期记忆塑形 + cognitive self_assessment 对接

**真账** (per §3.2 + `r7-mio-species-research-2026-08-28.md` L75-77 + `cognitive-module-wiring.md` L28):
- R22 reflection = 1:1 翻译 v1 reflection.rs (Triggered→Reflecting→Consolidating→Concluded 4 阶段 cycle, 写回 SQLite 【反思周期】episode)
- R11 长期记忆塑形 = 派 1 sub-agent 真调研 daily_summary/diary + cross_diary + memory_injection (per `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §4.2 #2, 跟 R20 + R22 并行)
- cognitive.self_assessment = WIRED, Judge-backed (per `cognitive-module-wiring.md` L28), AfterTurn 真实 Judge 结果 (不假装评分)
- meta_thinking + thought_cluster = 反思的元自学习读取口 (per v1 reflection L65-72 + thought_cluster L69-76)

**对接真账**: (1) R22 reflection 真实施 (1 周) = reflection.rs + meta_thinking.rs + thought_cluster.rs 三件套 (共享 trait `ThoughtClusterReader` + 反思写回 SQLite). (2) R20 preference_learning (2-3 周) + R11 长期记忆塑形 (2-3 周, 跟 R22 并行) = 反思写回 + 偏好学习 + 日记反思 三者并发 critical path (per `r7-mio-species-research-2026-08-28.md` L77 "R22 reflection (1 周) + R20 preference_learning (2-3 周) + 日记 cognitive 增维 (1-2 周) 三者并发"). (3) reflexion + intent_brier + confidence (3-5 周) = post-R22/R20/R11 critical path. (4) cognitive.self_assessment WIRED (AfterTurn, Judge-backed) = R22 reflection OWNER (per r21-r24 L82).

### 7.3 真实施顺序 + 估时 + 借鉴链

| 顺序 | 模块 | 估时 | 借鉴链 (v1 donor) | 阻塞 | 并行 |
|---|---|---|---|---|---|
| 1 | **R22 reflection** | 1 周 (R17 spec 30-45 min + 实施) | `apeireth-companion/src/reflection.rs:1-329` | R12 ✅ | — |
| 2 | **meta_thinking** | 1 周 (含 R22 同期) | `apeireth-companion/src/meta_thinking.rs:1-643` + VCP MetaThinkingManager.js | R22 | 跟 R22 + thought_cluster 并行 |
| 3 | **thought_cluster** | 1 周 (含 R22 同期) | `apeireth-companion/src/thought_cluster.rs:1-522` + VCP ThoughtClusterManager | R22 | 跟 R22 + meta_thinking 并行 |
| 4 | **R11 长期记忆塑形** | 2-3 周 | Mio 真账 §5 #5 + R20 + R22 critical path | R20 + R22 done | 跟 R20 + R22 并行 |
| 5 | **reflexion** | 1-2 周 | `apeireth-companion/src/reflexion.rs:1-497` + Reflexion 论文 | R22 done | 跟 R11 并行 |
| 6 | **intent_brier** | 1-2 周 | `apeireth-companion/src/intent_brier.rs:1-817` + oracle.rs Brier 公式 | W1/W2/W3 done | 跟 W1/W2/W3 并行 |
| 7 | **confidence** | 1 周 | `apeireth-companion/src/confidence.rs:1-177` + hydra genome + organ::world_model::CalibrationStrength | council + judge | 跟 council + judge 并行 |

**总估时**: 串行 12-14 周; 并行 **6-10 周** (R22 三件套同期 1 周; R20 + R11 + R22 三者并发 3-4 周; reflexion + intent_brier + confidence post-critical path 2-3 周). **跟 v2.0 release 估 2027-Q1-Q2 critical path 兼容** (per `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §6.3 "v2.0 release 估时上调 4-6 月 → 6-9 月 P0 真补完").

**借鉴链汇总**: v1 donor `legacy/donor/apeireth-companion/src/{6 modules}` (6 REAL 模块 1:1 翻译) + VCP MetaThinkingManager + ThoughtClusterManager + Generative Agents importance 触发 + Reflexion 论文 + hydra genome 置信度数学 + oracle.rs Brier 公式 + N.E.K.O reflective 维 (per `r7-neko-species-research-2026-08-28.md` L70) + Mio 日记反思部分 (per `r7-mio-species-research-2026-08-28.md` L75).

---

## 8. 主代理决策建议

### 8.1 6 项优先级

| # | 模块 | 优先级 | 估时 | 跟 R22 关系 | 物种化核心 |
|---|---|---|---|---|---|
| 1 | **R22 reflection** | 🔴 P0 critical path | 1 周 | — | 物种化塑形反思 |
| 2 | **meta_thinking** | 🔴 P0 critical path (同期) | 1 周 (含 R22) | 同期 | 元思考递归链 |
| 3 | **thought_cluster** | 🔴 P0 critical path (同期) | 1 周 (含 R22) | 同期 | 思考聚类元自学习 |
| 4 | **reflexion** | 🟡 P1 (post-R22) | 1-2 周 | 解耦 | 反思循环口头强化 |
| 5 | **intent_brier** | 🟡 P1 (跟 W1/W2/W3 并行) | 1-2 周 | 解耦 | Brier 校准意图 |
| 6 | **confidence** | 🟡 P1 (跟 council + judge 绑定) | 1 周 | 解耦 | 数学化自信度 |

### 8.2 6 项真实施 brief

1. **R22 reflection (🔴 P0, 1 周)**: 主代理亲写 R17 spec (30-45 分钟仿 R15 模板, 6 节 + 5 LOCKED + 0 装诚实 4 块) → 派 sub-agent 真实施 1 周: 新 crate `crates/engine/reflection/` + OrganKind::Reflection variant (R10 加不破 9 现有) + ReflectionScheduler 1:1 翻译 + ReflectionReflector trait (lib 0 LLM) + N4 thought_reader 接入 + 写回 SQLite 【反思周期】episode. **关键 0 装诚实标**: 当前 `cognitive-module-wiring.md` L32 "DEFERRED INTO SELF-ASSESSMENT" — R22 真实施要 OWN `cognitive.reflection` slot (短期 current-turn 仍是 self_assessment; 长期 cycle = reflection module owner). 0 触碰 LOCKED 5 项 + 9 哲学锚 0 减.
2. **meta_thinking (🔴 P0, 1 周 同期)**: 跟 R22 真实施同期 (共享 ReflectionReflector trait) + 新 crate `crates/engine/reflection/meta_thinking.rs` (子模块) 或独立 + MetaThinkingChain + MetaThinker + ReflectionMetaThinker + ChainReflectionThinker 1:1 翻译 + save_to_cluster 集成 thought_cluster trait (R22 同期) + 单测 1:1 翻译 v1 8 测试.
3. **thought_cluster (🔴 P0, 1 周 同期)**: 跟 R22 + meta_thinking 同期 (共享 trait) + 新 crate `crates/engine/thought_cluster/` + ThoughtClusterManager + ThoughtClusterReader trait 1:1 翻译 + 集成 R22 reflection with_thought_reader 注入 (per v1 reflection L65-72) + meta_thinking save_to_cluster (per v1 meta_thinking L323) + 单测 1:1 翻译 8 测试.
4. **reflexion (🟡 P1, 1-2 周)**: post-R22 + R20 (per `r21-r24-r12-research-2026-08-28.md` Part 2.4 派单顺序) + 新 crate `crates/engine/reflexion/` + ReflexionStore + Critic trait + RuleCritic (确定性规则版先行) + LlmCritic (LLM 留部署层) 1:1 翻译 + 失败事件接线 = `cognitive.judge` AfterModelResponse (verdict = retry/stop) + `cognitive.self_assessment` AfterTurn (score < 阈值) + 重试注入接线 = `cognitive.preference_recall` + `cognitive.memory_recall` TurnStart (预算 ≤ N chars) + 单测 1:1 翻译 5 测试.
5. **intent_brier (🟡 P1, 1-2 周)**: 跟 W1/W2/W3 world_model Brier 校准并行 (公式同源 per v1 L11-14) + 新 crate `crates/engine/intent_brier/` + IntentPrediction + FeedbackOutcome + IntentRecord + IntentLedger + brier_score + mean_brier + compute_window + compute_trend + domain_diagnostics + compute_report + render_report 1:1 翻译 v1 + 集成 `cognitive.self_assessment` slot (反馈 = Agree/Correct/Silent; 反馈源 = Judge verdict) + 集成 `cognitive.preference_recall` (render_report 注入 system prompt) + 单测 1:1 翻译 31 测试.
6. **confidence (🟡 P1, 1 周)**: 跟 cognitive.council + judge 绑定 + 新 crate `crates/engine/confidence/` + BetaBinomial + Strength + observe + mean + interval95 + strength + report 1:1 翻译 v1 + **organ::world_model 协同**: 保留本地 Calibr­ationStrength (per 0 装诚实 + 依赖最小), 跟通用 confidence crate 双向 optional borrow + 集成 cognitive.council (AdvisorVerdict.confidence: Option<f64> → Option<BetaBinomial>) + 集成 cognitive.judge (Judge critique 置信度基于 BetaBinomial, 不再 LLM 自报) + 集成 cognitive.self_assessment (AfterTurn self_assessment 置信度) + 单测 1:1 翻译 4 测试.

### 8.3 0 装诚实标 + 5 重守门 + LOCKED 0 触碰

**0 装诚实标** (per O-5): (1) 本调研 0 git clone v2 active workspace (per 主代理 brief "不写真账以外的 file"); (2) 仅读 1.0 真账 (`legacy/donor/apeireth-companion/src/{6 modules}`) + 2.0 handbook (`v2-reference-handbook-2026-08-28.md` + `cognitive-module-wiring.md`) + 5 R7 真调研 + R21-R24 调研推论; (3) 6 模块在 v2 active workspace **0 真实施** (per §0 grep 实测, no crate, no OrganTrait, no OrganKind variant; 仅 `organ::world_model::CalibrationStrength` 本地简化版 in-place); (4) donor E1 reflexion + N4 thought_cluster + W6 intent_brier 已入库 legacy/, **active workspace 0 移植**; (5) **真实施前主代理必亲验** (per `apeireth-1-0-vs-2-0-functional-gap-2026-08-28.md` §5).

**5 重守门 baseline** (per主代理派单 brief): clippy 0 warning ✅ (前 baseline); tests 0 fail (1739 passed) ✅ (前 baseline); legacy compat path < 100 (36) ✅ (前 baseline); LOCKED 5 项 0 触碰 ✅ (本调研 0 改 src / Cargo.toml / Cargo.lock); 9 哲学锚 0 减 ✅.

**LOCKED 0 触碰** (per §7.2 + `MANIFESTO §10`): 9 哲学锚本体 (`crates/foundation/core/src/eight_anchors.rs:58-79` + `NINE_ANCHORS_HARDCODE` L222-366); 13 键 (`crates/foundation/core/src/philosophy.rs:142` `RUNTIME_ENFORCED = false`); 3 项不可变脊柱 (`crates/foundation/core/src/onion.rs:249`); workspace.version (`Cargo.toml:44` `"1.2.0"`); R11 baseline 3 值 (`legacy/donor/apeireth-asi/tests/integration_r_measure.rs:42-44`); **新增 OrganKind variant** (R22 + reflexion + intent_brier + confidence 4 variant; meta_thinking + thought_cluster 可挂 reflection crate 共享 variant) — 加不破 9 现有 (per `r21-r24-r12-research-2026-08-28.md` §2.3).

**物种化维度真账** (per `apeireth-true-understanding-2026-08-28.md` §1.1 "她"): 反思+元认知 6 模块 = 物种化塑形核心 (per §1.1 + §3.1 "她记得你的存在"); meta_thinking + thought_cluster + reflection = 元自学习 (per §1.1 "per-user memory/preference/personality 塑形"); reflexion + intent_brier + confidence = 价值内化量化 (per §1.1 "她能教养后代 (vision L48)" — 价值内化从玄学变有数字); R22 reflection 真实施是 **物种化塑形反思的核心 critical path**.

---

## 9. 留 backlog (per §8 派单顺序)

| # | 模块 | 估时 | 阻塞 | 并行 |
|---|---|---|---|---|
| 1 | **R17 spec 接力** (主代理派 sub-agent, 30-45 min 仿 R15 模板) | 30-45 min | 0 | — |
| 2 | **R22 reflection 真实施** (1 周, 含 meta_thinking + thought_cluster 同期) | 1 周 | R17 spec done | meta_thinking + thought_cluster 并行 |
| 3 | **meta_thinking 1:1 翻译 v1 + save_to_cluster 集成 thought_cluster** | 1 周 (含 R22) | R22 | 跟 R22 + thought_cluster 并行 |
| 4 | **thought_cluster 1:1 翻译 v1 + with_thought_reader 注入 R22 reflection** | 1 周 (含 R22) | R22 | 跟 R22 + meta_thinking 并行 |
| 5 | **R20 preference_learning 真实施** (2-3 周) | 2-3 周 | R15 spec done | 跟 R22 + R11 并行 |
| 6 | **R11 长期记忆塑形真调研** (派 1 sub-agent, daily_summary/diary + cross_diary + memory_injection) | 2-3 周调研 + 1-2 周实施 | R20 + R22 done | 跟 R20 + R22 并行 |
| 7 | **reflexion 真实施** (1-2 周, post-R22 + R20) | 1-2 周 | R22 + R20 done | 跟 R11 并行 |
| 8 | **intent_brier 真实施** (1-2 周, 跟 W1/W2/W3 world_model Brier 校准并行) | 1-2 周 | W1/W2/W3 done | 跟 W1/W2/W3 并行 |
| 9 | **confidence 真实施** (1 周, 跟 cognitive.council + judge 绑定) | 1 周 | cognitive.council + judge | 跟 cognitive.council + judge 并行 |

**总估时**: 串行 12-14 周; 并行 **6-10 周** (R22 三件套同期 1 周; R20 + R11 + R22 三者并发 3-4 周; reflexion + intent_brier + confidence post-critical path 2-3 周). **跟 v2.0 release 估 2027-Q1-Q2 critical path 兼容**.

---

_R11-MetaCognition 写于 2026-08-28 Round 11, 主代理派单 "1.0 vs 2.0 反思+元认知 gap 真调研" 触发, 6 模块 1:1 翻译 v1 真账 vs v2 active workspace 0 真实施 推论真账, R22 reflection critical path 1 周 + meta_thinking + thought_cluster 同期 + reflexion / intent_brier / confidence post-critical path 派单顺序 + 估时 + 借鉴链真账就位. **0 装诚实标**: 本调研 0 git clone v2 active workspace, 仅读 1.0 真账 + 2.0 handbook + 5 R7 真调研 + R21-R24 调研 + 物种化真理解推论, 真实施前主代理必亲验._