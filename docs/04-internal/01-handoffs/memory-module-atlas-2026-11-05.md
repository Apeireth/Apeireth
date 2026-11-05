[Document-Meta]
Document: docs/04-internal/01-handoffs/memory-module-atlas-2026-11-05.md
Class: EVIDENCE
Status: 🟢 实测生成
Last-Modified: 2026-11-05
Author: deputy

# 记忆引擎 · 模块图谱（**132 个顶层模块逐个说清是干嘛的**）

> **性质**：**实测生成的事实清单**（不是方案）⇒ ⇒ 每个模块的「职责 / 能力域 / 生产可达性」都带判据。
> **四级口径**：本文件描述的是**「库侧 IMPLEMENTED」到「PRODUCTION WIRED」的可达性判定**
> ⇒ ⇒ **不是「全部 PRODUCTION WIRED」** ⇒ ⇒ 逐模块的口径见下表的「生产可达性」列
> （✅ = 该模块能力在生产链路上可达；❌ = 库级 IMPLEMENTED 但生产零消费；
> 🅿 = 有意默认关；📊 = 评估/报告工具）。
> **代码版本**：`debt-round8` @ `b844fa30bd58` · 记忆 crate **132 顶层模块 + 7 子目录 / 81108 行**。
> **生成方式**：`doc` 首句取自模块自身的 `//!` 文档（**不是我编的**）· 可达性按**生产链路传递闭包**判定。

## 一、判据（**先说清楚「可达」怎么定义的**）

**可达** = 该模块的公开能力**在生产链路上被真正用得到**，判定方式：
1. 生产代码用到的 `apeireth_memory::X` **根级导出** ⇒ ⇒ 映射回源模块 ⇒ ⇒ **入口集合**；
2. 在记忆 crate 内算**模块级传递闭包**（`crate::` / `use` 引用边）⇒ ⇒ 闭包内 = 可达。

⚠️ **两次判据纠错**（都记在这里，避免下一个人重踩）：
- ⛔ **限定名直查会严重低估**：第一版用 `apeireth_memory::<模块名>` 直查 ⇒ ⇒ `coordinator` 被判「不可达」
  ⇒ ⇒ **错**：它经**根级重导出**被生产使用 ⇒ ⇒ 必须走**根导出 → 源模块**映射；
- ⛔ **正则只匹配小写也会漏**：第二版 `apeireth_memory::[a-z_0-9]+` ⇒ ⇒ `apeireth_memory::MemoryCoordinator`
  ⇒ ⇒ **不被捕获** ⇒ ⇒ 根级导出映射缺一半 ⇒ ⇒ **可达率被低估到 10%**（真值 42%）。

## 二、总览

| 指标 | 数字 |
|---|---|
| 顶层模块 | **132** |
| 子目录 | 7（`backend` / `canonical` / `coordinator` / `dailynote` / `freshness` / `layered_memo` / `migrations`）|
| ✅ **生产可达** | **55**（42%）|
| ❌ 不可达 | **78**（59%）|
| 其中 🅿 有意默认关（研究原型/胶水）| 5 |
| 其中 📊 评估/报告工具（该在 `tests/`）| 7 |
| 其中 **❌ 真正的未接线能力** | **66**（30541 行）|

## 三、能力域可达率（**「超级引擎」缺哪块 ⇒ 这里**）

| 能力域 | 可达/总数 | 可达率 | 不可达行数 |
|---|---|---|---|
| 其他 | 9/44 | 20% | 14543 |
| 存储与持久 | 1/9 | 11% | 4234 |
| 评估与度量 | 6/13 | 46% | 3062 |
| 检索与排序 | 8/15 | 53% | 3120 |
| 组织与结构 | 5/10 | 50% | 3057 |
| 会话与身份 | 8/12 | 67% | 2282 |
| 整编与反思 | 2/6 | 33% | 1662 |
| 注入与呈现 | 5/8 | 62% | 1496 |
| 写入与抽取 | 5/7 | 71% | 572 |
| 安全与准入 | 0/1 | 0% | 291 |
| 预测与决策 | 0/1 | 0% | 756 |
| 治理与遗忘 | 5/6 | 83% | 447 |

## 四、逐模块清单（**按能力域分组**）

### 其他（44 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `coordinator` | 1944 | 38 | ✅ 可达 | Unified Memory Coordinator (MemoryCoordinator). |
| `access_history` | 786 | 19 | ✅ 可达 | Append-only memory access telemetry and ACT-R-style activation. |
| `hallways` | 734 | 17 | ❌ 未接线 | R179 P1-10: Hallway — wing 内 entity-pair 跨位置走廊 (对齐同类记忆工程公开设计). |
| `intent_brier` | 733 | 33 | ❌ 未接线 | apeireth-memory::intent_brier — 意图理解准确率 Brier 自我诊断系统 (W6 / 元认知自校准). |
| `memory_view` | 732 | 30 | ❌ 未接线 | apeireth-memory::memory_view — 记忆纵深体系的**读侧契约**（设计稿 §5 ①②）。 |
| `lesson` | 729 | 17 | ❌ 未接线 | 教训对象（Lesson）—— 让「成长」有可验证的载体（L1：表 + 四拍状态机 + 五接口）。 |
| `agent_trace` | 637 | 14 | ❌ 未接线 | Core Capability Expansion Phase 5 — Agent 执行轨迹存储 (structured trace). |
| `swap_flow` | 624 | 8 | ❌ 未接线 | apeireth-memory::swap_flow — 算法切换流程（**零风险上线 / 可回退**）。 |
| `betti_hole_detector` | 617 | 8 | ❌ 未接线 | Algebraic Topology Persistent Homology & Betti Cognitive Void Detector |
| `novel_corpus` | 616 | 7 | ❌ 未接线 | apeireth-memory::novel_corpus — 小说（Gutenberg 纯文本）→ 记忆挖题器的语料接线。 |
| `streams` | 587 | 12 | ❌ 未接线 | 6 历史流 Append-only Log (思想/提案/行动/关系/演化/反思期). |
| `dedup` | 586 | 15 | ❌ 未接线 | Windowed fingerprint dedup (salvage of companion observer_capture LR |
| `derived_repair` | 568 | 7 | ✅ 可达 | B2 · RA-15 P0-B: 屏障优先级联修复执行器（Research 前缀，默认关闭）。 |
| `wiki_corpus` | 565 | 7 | ✅ 可达 | apeireth-memory::wiki_corpus — 用户知识库（Markdown 语料）的目录树检索面。 |
| `river_topology` | 550 | 17 | ❌ 未接线 | river_topology: 浪潮流体拓扑动力学与双标度连续场求解器 (DualScaledField) |
| `continuity_link` | 535 | 12 | ✅ 可达 | R22 ST-A2.5 — 主体连续性全链路。 |
| `handover` | 513 | 9 | ❌ 未接线 | apeireth-memory::handover — **handover 演练**（「换人接手」的可验证版本）。 |
| `meta_thinking` | 485 | 19 | ❌ 未接线 | apeireth-memory::meta_thinking — 元思考递归链 (元自学习 / 认知多阶段推演). |
| `wiki_corpus_real` | 460 | 11 | ❌ 未接线 | apeireth-memory::wiki_corpus_real — 真实语料 → 挖题器的接线。 |
| `milestone` | 449 | 13 | ❌ 未接线 | apeireth-memory::milestone — 关系里程碑机制 (R12-CoordinationContext-2 闭环). |
| `shadow_run` | 432 | 8 | ❌ 未接线 | apeireth-memory::shadow_run — **影子双跑**（换算法可零风险上线/回退）。 |
| `five_dimensional` | 412 | 18 | ❌ 未接线 | 五维时空记忆系统 (Five-Dimensional Memory Topology). |
| `provenance` | 392 | 11 | ✅ 可达 | TP24 (M5 + N25): 记忆来源链 + 时间元数据. |
| `procedural` | 390 | 9 | ❌ 未接线 | apeireth-memory::procedural — 程序性记忆与习惯固化引擎 (N.E.K.O 5 维记忆第五维 / 习惯规则链) |
| `git_corpus` | 371 | 5 | ❌ 未接线 | apeireth-memory::git_corpus — git 提交历史 → 记忆挖题器的语料接线。 |
| `arbitration` | 363 | 11 | ❌ 未接线 | apeireth-memory::arbitration — HASH-SQL 唯一事实时间线与不可篡改审计仲裁机 (Arbitratio |
| `community` | 351 | 8 | ❌ 未接线 | apeireth-memory::community — 图社区分层聚合 + 双级检索分诊 (W3 实现批)。 |
| `semantic_axis` | 348 | 5 | ❌ 未接线 | semantic_axis: 加权中心化 PCA 语义主轴 (Semantic Axis)、逻辑深度与跨域共振桥 |
| `proactive_recall` | 333 | 13 | ✅ 可达 | Deterministic, candidate-only proactive memory recall. |
| `reconciler` | 332 | 7 | ❌ 未接线 | Pure, deterministic reconciliation for extracted memory candidates. |
| `locomo_corpus` | 290 | 5 | ❌ 未接线 | **LoCoMo 真实对话记忆基准**的加载与考卷构造。 |
| `kuramoto_resonance` | 281 | 6 | ❌ 未接线 | Kuramoto Non-linear Oscillator Phase-Locking & Epiphany Avalanche Engi |
| `gen_cache` | 267 | 12 | ✅ 可达 | N8: generation 绑定观测缓存 (通用 Arc 快照 + 观测缓存模式). |
| `perltqa_adapter` | 260 | 11 | ❌ 未接线 | apeireth-memory::perltqa_adapter — 外部公开卷（PerLTQA）→ 本仓考卷格式的转换。 |
| `residual_pyramid` | 257 | 7 | ❌ 未接线 | residual_pyramid: 修正 Gram-Schmidt 多层正交残差金字塔 |
| `history_streams` | 244 | 6 | ❌ 未接线 | R22 ST-A2.4 — 6 历史流深度公共 API。 |
| `confidence` | 239 | 10 | ❌ 未接线 | Beta-Binomial confidence with a Wilson 95% interval. |
| `chronicle_crystallizer` | 232 | 7 | ❌ 未接线 | Autobiographical Chronicle Crystallization & Fractal Decay Engine. |
| `wiki_fs` | 215 | 9 | ❌ 未接线 | Karpathy LLM-Wiki 知识编译与反熵治理引擎 (WikiFsEngine). |
| `facade` | 162 | 7 | ❌ 未接线 | Minimal production-callable facade for durable memory mutations. |
| `organ_kani_proofs` | 152 | 0 | ❌ 未接线 | R177 memory organ Kani proofs (W2) |
| `metadata_filter` | 139 | 11 | ❌ 未接线 | JSON property-bag filters for retrieval (salvage of |
| `continuity_state` | 138 | 4 | ✅ 可达 | Bounded Continuity State and incremental compression. |
| `experience` | 21 | 0 | ❌ 未接线 | P-arch (2026-08-28): B1 Experience trait and conservative extraction |

### 存储与持久（9 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `offline_archive` | 909 | 12 | ❌ 未接线 | apeireth-memory::offline_archive — 记忆数据的**脱机归档**（保原文 + 可读 + 自说明）。 |
| `migrate_rollback` | 901 | 9 | ❌ 未接线 | apeireth-memory::migrate_rollback — 迁移的**回滚**（可演练、机器可证）。 |
| `migrations` | 836 | 3 | ✅ 可达 | Schema migrations for apeireth-memory. |
| `experience_store_sqlite` | 742 | 4 | ❌ 未接线 | P-arch (2026-08-27) + v2.0.0-rc.1 RC-2: Experience trait 真 SQLite impl |
| `preference_store_sqlite` | 458 | 5 | ❌ 未接线 | P-arch (2026-08-27): v2.0.0-rc.1 RC-3 SQLitePreferenceStore impl (场景 D |
| `milestone_store_sqlite` | 378 | 5 | ❌ 未接线 | 关系里程碑的 **SQLite 持久实现**（2026-10-24 接电）。 |
| `cold_archive` | 366 | 13 | ❌ 未接线 | apeireth-memory::cold_archive — **单条记忆的冷层归档**（归档 ≠ 删除）。 |
| `self_assessment_store_sqlite` | 337 | 5 | ❌ 未接线 | P-arch (2026-08-27): v2.0.0-rc.1 RC-4 SelfAssessmentStore SQLite impl  |
| `preference_store` | 143 | 1 | ❌ 未接线 | P-arch (2026-08-27): 场景 D 例 1 PreferenceStore 0 装 impl (RC-3). |

### 评估与度量（13 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `golden_mining` | 1323 | 6 | ✅ 可达 | apeireth-memory::golden_mining — 从记忆记录挖考卷候选题目。 |
| `quality_metrics` | 704 | 18 | 📊 评估 | apeireth-memory::quality_metrics — 设计稿 §6.1 的四个剩余指标。 |
| `golden_set` | 604 | 15 | ✅ 可达 | apeireth-memory::golden_set — 记忆质量考卷（golden set）与检索/注入指标。 |
| `golden_diag` | 557 | 12 | 📊 评估 | apeireth-memory::golden_diag — 召回缺口的**诊断**（回答「为什么 recall 这么低」）。 |
| `online_calibration` | 511 | 29 | ✅ 可达 | Generalized online calibration: EMA baseline, linear scale+offset fit, |
| `calibration` | 498 | 21 | ❌ 未接线 | Murphy (1973) Brier 三分解 + 等宽 CalibrationBin + Expected Calibration Err |
| `robustness_metrics` | 436 | 11 | 📊 评估 | apeireth-memory::robustness_metrics — 设计稿 §6.1 的 **L3 鲁棒/成长**四项。 |
| `d4_eval` | 405 | 9 | 🅿 默认关 | apeireth-memory::d4_eval — D4 模式层的**离线评估器**。 |
| `golden_run` | 353 | 7 | ✅ 可达 | apeireth-memory::golden_run — 把考卷接到**真实检索**上，跑出真分数。 |
| `calibration_critic` | 351 | 9 | ❌ 未接线 | Calibration-aware critic: Brier + ECE → Continue / Revise / Reject. |
| `golden_baseline` | 336 | 8 | ✅ 可达 | apeireth-memory::golden_baseline — 内部卷的**检索基线**（供「不许掉」闸用）。 |
| `quality_drift` | 323 | 12 | ✅ 可达 | 在线质量漂移监控：**EMA 基线 + 2σ 连击告警**（记忆线的「不许悄悄劣化」）。 |
| `delivery_metrics` | 111 | 3 | 📊 评估 | apeireth-memory::delivery_metrics — **送达率**（检索到了 ⇒ 送到了吗？）。 |

### 检索与排序（15 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `research_derived_memory` | 1211 | 29 | ✅ 可达 | B2 · Phase 1: 派生记忆图与遗忘传播审计（Research 前缀，默认关闭）。 |
| `retrieval_tuning` | 1187 | 42 | ✅ 可达 | 检索维度自进化 (V4-c) —— 检索排序权重随使用证据小幅改写。 |
| `retrieval_pipeline` | 1172 | 23 | ✅ 可达 | Governed hybrid retrieval pipeline. |
| `signal_sources` | 1108 | 13 | ❌ 未接线 | apeireth-memory::signal_sources — **非答案侧**的题面信号来源（端到端的那一半）。 |
| `query_signals` | 1065 | 11 | ✅ 可达 | apeireth-memory::query_signals — 题面**信号增强**的量化（查询判别力的体检）。 |
| `hybrid_search` | 721 | 17 | ✅ 可达 | apeireth-memory::hybrid_search — BM25 词法与向量混合检索召回引擎 (R12-Storage 实施). |
| `persistent_vector` | 584 | 14 | ❌ 未接线 | Persistent brute-force cosine KNN (salvage of |
| `vector_rebuild` | 460 | 8 | ❌ 未接线 | apeireth-memory::vector_rebuild — 派生索引的**重建**与**换代检测**。 |
| `topic_predictor` | 410 | 14 | ✅ 可达 | apeireth-memory::topic_predictor — 话题预测器与主动记忆预载通道. |
| `research_roaming_memory` | 295 | 9 | 🅿 默认关 | B7 · Phase 6 原型一: 漫游记忆 CRDT (Research 前缀, 不进默认路径)。 |
| `offline_embed` | 253 | 5 | ❌ 未接线 | apeireth-memory::offline_embed — **纯本地**的 embedder（无 API key、无网络）。 |
| `research_non_interference` | 220 | 7 | 🅿 默认关 | B8 · Phase 6 原型二: 模块非干扰性 (Research 前缀, 不进默认路径)。 |
| `vector_distance` | 200 | 12 | ❌ 未接线 | Vector distance utilities (salvage of apeireth-vector::distance). |
| `memory_rank` | 198 | 8 | ✅ 可达 | Generative-Agents-style memory ranking + tombstone filter. |
| `query_expand` | 162 | 2 | ✅ 可达 | Deterministic query expansion for hybrid retrieval. |

### 组织与结构（10 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `persona_layer` | 1680 | 43 | ✅ 可达 | 人格记忆层 (Persona Memory Layer) —— 「我是谁」的独立沉淀面. |
| `temporal_graph_store` | 1022 | 31 | ✅ 可达 | Durable append-only temporal graph storage backed by the V11 schema. |
| `bitemporal_graph` | 781 | 20 | ❌ 未接线 | 双时态事实图谱 (Bitemporal Knowledge Graph) 与 Intrinsic Residual 特异性打分. |
| `graph_algo` | 639 | 12 | ❌ 未接线 | Graph traversal helpers over [MemoryGraph] (salvage of |
| `scope` | 628 | 34 | ✅ 可达 | Canonical Memory Plane scope, provenance, ranking, and provider contra |
| `amem_graph` | 606 | 24 | ❌ 未接线 | A-MEM graph algorithms. |
| `three_layer` | 529 | 10 | ❌ 未接线 | apeireth-memory::three_layer — **R30 U9 3 层记忆 facade** |
| `cluster_store` | 502 | 16 | ❌ 未接线 | apeireth-memory::cluster_store — 思维簇管理与元自学习读取口 (N4 / 认知长程聚类). |
| `three_tier_vault` | 317 | 14 | ✅ 可达 | SwarmVault & OpenKB inspired Three-Tier Knowledge Vault & Vectorless T |
| `layers` | 210 | 13 | ✅ 可达 | Logical 4-layer memory representations for Apeireth Unified Memory 2.0 |

### 会话与身份（12 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `commitments` | 992 | 22 | ✅ 可达 | Durable commitment tracking backed by the V11 memory schema. |
| `session_note` | 955 | 21 | ✅ 可达 | Session + Note 存储. |
| `principles` | 871 | 11 | ❌ 未接线 | apeireth-memory::principles — 动态原则层与原则洋葱晋级候选 (Level 2/3 自成长信条机制). |
| `persona_store_sqlite` | 822 | 7 | ✅ 可达 | SQLite-backed persistence for persona profiles. |
| `session_lifecycle` | 796 | 9 | ❌ 未接线 | Core Capability Expansion Phase 2 — 后端会话生命周期 (真实 source of truth). |
| `identity` | 499 | 5 | ❌ 未接线 | IdentityCard 跨载体存储 (D2 §4 主体连续性). |
| `diary` | 427 | 13 | ✅ 可达 | apeireth-memory::diary — 日记本归档与叙事记忆中心 (R12-LongTermMemory 实施). |
| `partner` | 363 | 20 | ✅ 可达 | apeireth-memory::partner — 伙伴与双向羁绊模型 (R12-SpeciesCore-1 实施). |
| `cross_diary_wiring` | 289 | 7 | ✅ 可达 | apeireth-memory::cross_diary_wiring — 跨日记检索接线面 (Cross-Diary Retrieval |
| `partner_store_sqlite` | 225 | 5 | ✅ 可达 | SQLite-backed persistence for partner records. |
| `cross_diary` | 220 | 8 | ✅ 可达 | apeireth-memory::cross_diary — 跨日记与记忆图关联索引 (Cross-Diary Index). |
| `principal_sessions` | 116 | 2 | ❌ 未接线 | 「**主体 → 会话**」归属映射的**写侧**（2026-11-01 安全审计 P1 收口）。 |

### 整编与反思（6 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `reflexion` | 977 | 14 | ❌ 未接线 | apeireth-memory::reflexion — 口头强化反思闭环 (Reflexion 式失败轨迹与重试注入). |
| `consolidation_trigger` | 766 | 22 | ✅ 可达 | 夜间整编的**驱动层**（P2 的最后一格：把「整编作业」变成「每夜只跑一次、只跑新料」的一次决策）。 |
| `dreaming` | 305 | 10 | ❌ 未接线 | apeireth-memory::dreaming — 离线做梦与长程认知重组引擎 (Cognitive-Dream 6 状态机). |
| `dream_wiring` | 262 | 2 | ❌ 未接线 | **W2 §4.1 dreaming 接线** — 做梦引擎的生产胶水与确定性思考器 (2026-10-10)。 |
| `consolidation` | 155 | 8 | ✅ 可达 | Memory consolidation background job. |
| `dream_consolidation` | 118 | 4 | ❌ 未接线 | Incremental dream-candidate selection (baseline DreamScheduler::tick |

### 注入与呈现（8 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `context_compiler` | 837 | 11 | ✅ 可达 | Closed-World Prompt Context Compiler. |
| `golden_injection2` | 784 | 8 | 📊 评估 | apeireth-memory::golden_injection2 — 注入形态对照的**第二轮**（消偏置版）。 |
| `injection_renderer` | 644 | 17 | ✅ 可达 | apeireth-memory::injection_renderer — 注入契约（设计稿 §5 ③）。 |
| `anchor_injection` | 500 | 14 | ✅ 可达 | apeireth-memory::anchor_injection — **结构化锚点** + **主体段定位**。 |
| `golden_injection` | 469 | 8 | 📊 评估 | apeireth-memory::golden_injection — 注入策略 × 评测的**实验台**。 |
| `context_ledger` | 340 | 11 | ✅ 可达 | Rolling cross-frontend context ledger (salvage of the companion ledger |
| `memory_injection` | 266 | 4 | ✅ 可达 | Closed-world memory injection (apeireth-companion::memory_injection) |
| `context_window` | 243 | 7 | ❌ 未接线 | Context-window projection and bounded compaction. |

### 写入与抽取（7 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `memory_chain` | 837 | 12 | ✅ 可达 | apeireth_memory::memory_chain — 记忆层权威存储（D0）的哈希链集成与体检。 |
| `memory_materializer` | 675 | 22 | ✅ 可达 | Narrow orchestration boundary for turning bounded conversation input i |
| `episode` | 631 | 11 | ✅ 可达 | Episode 存储 (D2 §3 episode/episode query). |
| `extraction` | 552 | 9 | ✅ 可达 | Deferred Memory Plane extraction contracts. |
| `chunking` | 463 | 9 | ❌ 未接线 | apeireth-memory::chunking — 语料的**段落级切分**（召回缺口「切分粒度」假设的**验证器**）。 |
| `append_only` | 425 | 5 | ✅ 可达 | Append-only Log 公共类型 + 6 历史流 trait 共享设施. |
| `daily_summary` | 109 | 3 | ❌ 未接线 | apeireth-memory::daily_summary — 每日记忆摘要与结构化统计. |

### 安全与准入（1 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `admission_gate` | 291 | 11 | 🅿 默认关 | B2 · RA-15 P1-A: 记忆准入控制（Research 前缀，默认关闭）。 |

### 预测与决策（1 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `ensemble` | 756 | 20 | ❌ 未接线 | Ensemble forecast aggregation + Hanson LMSR prediction market. |

### 治理与遗忘（6 个模块）

| 模块 | 行 | API | 生产可达性 | 职责（模块自述）|
|---|---|---|---|---|
| `memory_governance` | 1063 | 7 | ✅ 可达 | Core Capability Expansion Phase 3 — 记忆治理 (Memory Mutation / Forget / P |
| `universal_forget` | 840 | 15 | ✅ 可达 | 生产 forgets 路径 ×「**先记访问史，再忘记**」守卫接线 (G1 收口). |
| `forget_coordinator` | 613 | 8 | ✅ 可达 | B2 · RA-15 P0-A 生产接线（opt-in，默认关闭）：三段式协调遗忘。 |
| `forgetting_metrics` | 447 | 11 | 📊 评估 | apeireth-memory::forgetting_metrics — 设计稿 §6.2 的四个「遗忘」指标。 |
| `retention` | 446 | 13 | ✅ 可达 | Retention policy + sweep (salvage of context-ledger prune, layered-mem |
| `forget_access_guard` | 197 | 10 | ✅ 可达 | apeireth-memory::forget_access_guard — 「**先记访问史，再忘记**」的顺序纪律。 |

## 五、⚠️ **边界：这次没测什么**（**不许把这份清单当全称结论引用**）

| 没测 | 后果 |
|---|---|
| **没测「运行时是否真被调用」** | 本清单判的是**静态可达**（编译期链路）⇒ ⇒ **动态不可达**（有入口但运行时没走到）**测不出** |
| **没测「可达的能力效果好不好」** | ✅ 只说明**够得到**，**不说明**接了就有用 ⇒ ⇒ 效果要看评估集（`golden_set`） |
| **没测 7 个子目录内部** | `backend` / `canonical` / `coordinator` / `dailynote` / `freshness` / `layered_memo` / `migrations` **未逐模块列** ⇒ ⇒ 它们算进了「其他」域 |
| **没测 crate 外的「等效实现」** | 若某能力在**别的 crate** 有等效实现 ⇒ ⇒ 本清单仍判「不可达」⇒ ⇒ **可能低估** |
| **依赖判定是启发式的** | 模块依赖用 `crate::X` / `use X::` 文本匹配 ⇒ ⇒ **动态分派/宏生成的引用会漏** ⇒ ⇒ 42% 是**下界** |
| **没测「模块被引用的强度」** | 「被引 1 次」与「被引 100 次」在本清单里**同等** ⇒ ⇒ 实际价值不同 |

## 六、复现命令

```sh
cd /home/bai/Projects/apeireth-debt94-wt
find crates/engine/memory/src -maxdepth 1 -name '*.rs' | wc -l   # 132
find crates/engine/memory/src -name '*.rs' -exec cat {} + | wc -l  # 81108
python3 scripts/audit_unwired.py --check                          # 孤儿基线 119 条
```

## 七、相关文件

- 收敛方案（**自我更正**版）：[`memory-convergence-plan-2026-11-05.md`](./memory-convergence-plan-2026-11-05.md)
- 记忆线状态：[`memory-engine-status-2026-11-03.md`](./memory-engine-status-2026-11-03.md)