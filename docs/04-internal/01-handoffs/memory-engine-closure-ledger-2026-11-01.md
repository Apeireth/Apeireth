# 记忆引擎收口清单（2026-11-01 实测基线）

[Document-Meta]
Document:        memory-engine-closure-ledger-2026-11-01.md
Version:         v1 (2026-11-01 实测基线)
Last-Modified:   2026-11-01
Status:          🟢 活跃 (记忆引擎这条线的接手人入口)
Class:           STATUS

> **给谁看**：接续记忆引擎这条线的人或 AI。**本文自足** —— 不需要读对话记录。
> **本文是什么**：记忆引擎（`crates/engine/memory`，129 个顶层模块）当前**做到了哪、还差什么**的**实测**清单。
> **口径纪律**：本文的每一条都来自**当场实测**（代码 grep / 工具跑 / 文件级 diff），
> **不引用文档里的状态描述**（设计文档是**输入**，不是**现状**）。
> ⚠️ **本文的方法论警告在 §4**：**「零引用」清单不能当结论用** —— 今天我在这一条上连错 5 次。

---

## 0. 一句话结论

**记忆引擎不是「写了一半」，而是「库已完成、接线大半完成，但有 3 类债没收口」**：

| 类别 | 状态 | 详见 |
|---|---|---|
| **库本体** | ✅ **完成且强**（129 模块 · 902 个集成测试 · 遗忘曲线有 TLA+ 验证） | §2 |
| **深化四期** | ✅ **P1 / P3 / P4 已完成**；⚠️ **P2 缺「运行时触发器」** | §3 |
| **接线** | ◐ 工具报 **51 个记忆模块整块未接线**，但其中**多数是评测装置**；**4 组整族**待收口 | §5 |
| **口径** | ⚠️ **生产路径上的核心能力没有 claim**（检索排序/注入契约） | §6 |
| **门禁** | ⭐ **CI 长期红的根因查清并已修**（不是代码问题） | §7 |

---

## 1. 三条必须先知道的纪律（本文最有价值的部分）

1. ⭐ **「零引用」≠「没接电」**（`unwired-ledger.md` §5.5q 已写，本文再确认一次）：
   - **同名遮蔽**：`ProactiveRecallPolicy` 在生产出现 ⇒ 裸 token 计数会把 `RecallPolicy` 也算成「已用」；
     反之 `TemporalGraphStore::facts_as_of` **真死**却被 `BitemporalGraph::facts_as_of` 遮蔽 ⇒ **看不见**。
   - **语言边界**：`apeireth_sdk_py`（PyO3 `#[pymodule]`）由 **Python import** 调用 ⇒ Rust 侧零引用是**正常态**。
   - ⇒ ⇒ **凡用「grep 库外引用」判「接线/未接线」的结论，都必须再核一次模块自述（`//!` 头注释）**。
2. ⭐ **模块自述是最强的判据**：`persistent_vector.rs` 头注释明写
   「**default-off — nothing in `hybrid_search` auto-wires it**」
   ⇒ ⇒ **零生产消费是「设计如此」，不是缺口**（差点被我当成缺口）。
3. ⭐ **同一能力被写两遍**是本仓最常见的形态：`RecallPolicy`（未接）与
   `MemoryRecallQuery` + `MemoryModule`（在用）**默认值逐字相同**（见 §6.1）。
4. ⭐ **改了 claim 的 `sources` 就要重打 `verified_at`**（否则 CI 的 `docs truth gate` 立刻红）：
   本轮实测 —— `gov.chain_drill_ci` 的 `sources` 就是 `.github/workflows/rust.yml`，
   我为修 CI 磁盘改了那个文件 ⇒ ⇒ 那条 claim **立刻 STALE** ⇒ ⇒ 修法：把戳指向
   **包含该改动的远端 commit**（CI 跑的是真实历史 ⇒ `git diff <戳> -- <sources>` 必为空），
   然后**重生成** `STATUS.md` / `wiring.yaml`。
   ⇒ ⇒ **推论**：`claims.yaml` 是真源，**改 CI 配置也会动它** ⇒ ⇒ 别以为「我只改了 workflow」。

---

## 2. 库本体（✅ 完成）

| 维度 | 读数 | 取证方式 |
|---|---|---|
| 顶层模块 | **129** | `ls crates/engine/memory/src/*.rs` |
| 代码行数 | **78,495** | `cat crates/engine/memory/src/**/*.rs` |
| 集成测试文件 | **58** | `ls crates/engine/memory/tests/*.rs` |
| 集成测试函数 | **902** | `grep -c "#\[test\]\|#\[tokio::test\]"` |
| examples | **29** | `ls crates/engine/memory/examples/*.rs` |
| 遗忘曲线 | **有 TLA+ 验证** | `docs/04-internal/backend-enhancement/02-axes/C-memory-deepening.md` 的现状段 |

---

## 3. 深化四期（**三期已完成，一期缺触发器**）

> 依据：`docs/04-internal/backend-enhancement/02-axes/C-memory-deepening.md` 的分期
> （P1 效果评估集 → P2 夜间整编 → P3 教训回路 → P4 图谱向量生产），
> **但状态以下表为准**（该文档正文写「一步未动」是**过时的**）。

| 期 | 状态 | 证据（可复核） |
|---|---|---|
| **P1 效果评估集** | ✅ **完成** | `scripts/golden-baseline.json` 固化了 2 个语料（redchamber 476 例 / watershed 248 例）的 `recall@k`/`ndcg@k`/`groundedness`/`evidence_precision`/`delivery_rate` + **`case_fingerprint`（防偷换语料）**；`scripts/check_golden_baseline.py` 是闸门；内部卷 **301 条精标**（目标 300）⇒ 基线 `recall@10 = 0.8738`，`coverage` 接入后 **0.9003** |
| **P2 夜间整编** | ⚠️ **库内已完成，缺运行时触发** | `crates/engine/memory/src/consolidation.rs` 有 **4 个 `pub fn` 入口**（`consolidate` / `consolidate_governed_episodes` / `consolidate_governed` / `consolidate_persona`），且 `coordinator.rs:22` **已 import 它** ⇒ ⇒ **算法与入口都在**；但 **core 的 `CognitiveDreamState::Consolidating` 没有任何生产代码推进它**（只有 core 自己的测试推进到 `Dreaming`）⇒ ⇒ **「夜里谁来敲这一下」是缺的** |
| **P3 教训回路** | ✅ **已接线** | `reflexion` 在库外 **16 处**引用（`runtime-assembly/src/canonical/cognitive.rs`、`consolidation_knobs.rs`、`capability_switch_chains.rs` …） |
| **P4 图谱/向量** | ✅ **已完成**（向量持久化是「设计如此」） | 图谱 `amem_graph` · 语义轴 `semantic_axis` **已进 `runtime-assembly` 的 `cognitive.rs`**；向量：`canonical::vector::VectorIndex` **就是 hybrid-search 的语义通道**（生产在用），`persistent_vector::PersistentVectorIndex` 自述 **default-off**（`sqlite-vec` 延后 ⇒ 暴力 KNN 兜底）⇒ **不是缺口** |

---

## 4. ⭐ 判据盲区（**今天连踩 5 次的坑** ⇒ 「零引用清单」不可直接当结论）

| 盲区 | 表现 | 实证 |
|---|---|---|
| **同名遮蔽** | 裸 token 计数把 `ProactiveRecallPolicy` 算成 `RecallPolicy` 的消费方 | `grep -rn RecallPolicy crates/` 命中的全是 `ProactiveRecallPolicy` |
| **真死被遮蔽** | `TemporalGraphStore::facts_as_of` / `::history` **真死**，但同名方法在别处出现 ⇒ 符号级基线看不见 | `unwired-ledger.md` §5.5q「判据盲区（未改判据，报备）」 |
| **语言边界** | PyO3 模块由 Python import 调用 ⇒ Rust 侧零引用正常 | `apeireth_sdk_py` |
| **「与另一公开 API 同签名纯改名」** | 这种是真死（应删），但签名不同的便利包装应保留 | §5.5q 的判据口径（先例：`append_fact` / `as_session_store` / `stats_ref`） |

⇒ ⇒ **⇒ 建议（给守卫维护者）**：把符号级审计从**裸 token 计数**改成**限定名计数**
（`Type::method`），并给 **PyO3** 模块加显式豁免 ⇒ ⇒ 否则「未接线清单」会持续误导判断。
⚠️ **改判据会动基线**（`scripts/unwired-baseline.txt`）⇒ ⇒ **基线只减不增**的流程要一起走 ⇒ ⇒ **需先对表**。

---

## 5. 接线（◐ 大半已完成，**4 组整族待收口**）

**工具读数**：`python3 scripts/audit_unwired_modules.py` ⇒ **整块未接线 95 个模块，其中记忆 crate 占 51 个**。
⚠️ 但按 §4 的纪律，这 51 个**不能直接当缺口**：其中约 **21 个是评测/报告装置**
（`golden_baseline` / `golden_diag` / `locomo_corpus` / `quality_metrics` / `delivery_metrics` /
`robustness_metrics` / `forgetting_metrics` / `perltqa_adapter` / `novel_corpus` / `wiki_corpus_real` /
`offline_embed` / `vector_rebuild` / `signal_sources` / `swap_flow` / `d4_eval` / `cold_archive` …）
⇒ **零生产引用是它们的正常形态**（跑评测时不接生产）。

### 5.1 台账点名的 4 组整族（**限定名级复核结果**）

| 族 | 库外引用（限定名） | 判定 |
|---|---|---|
| `scope::RecallPolicy` | **0** | ⬜ **仅库内** ⇒ 与 `MemoryRecallQuery` + `MemoryModule` **重复实现** ⇒ 见 §6.1 |
| `dailynote::DailyNote` | **0** | ⬜ 仅库内 ⇒ 待判 |
| `online_calibration::AdaptiveBaseline` | **0** | ⬜ 仅库内 ⇒ 待判 |
| `online_calibration::DriftDetector` | **1**（`crates/engine/runtime-assembly/src/canonical/research_autonomy.rs`） | ✅ **已接线 ⇒ 台账这条已过期，需更正** |
| `cluster_store::ClusterStore` | **0** | ⬜ 仅库内 ⇒ 待判 |

⇒ ⇒ **⇒ 台账里这一条需要更正**（`DriftDetector` 已接）。

---

## 6. ⚠️ 两处「欠的口径 / 欠的收口」（**真正的剩余工作**）

### 6.1 ✅ `RecallPolicy` 与生产检索：**同一套策略写了两遍 ⇒ 已归组销账**（2026-11-01 复核后修正）

| | `scope::RecallPolicy`（**未接**） | 生产侧（在用） |
|---|---|---|
| 可见范围 | `visible_scopes` + `with_visible_scope(s)` | `MemoryRecallQuery.visible_scopes` |
| 层 | `layers` | `MemoryRecallQuery.layers` |
| 条数/字符 | `limit` / `max_chars` | `MemoryRecallQuery.limit` / `max_chars` |
| 最低分 | `min_score` | `MemoryRecallQuery.min_score` |
| 时点 | `as_of_ms` | `MemoryRecallQuery.as_of_ms` |
| **主动召回预算** | **`proactive_budget`（默认 2）** | ❌ **无此字段** |
| **token 预算** | **`token_budget`（默认 1024）** | `MemoryModule.max_recall_tokens`（**默认 1024**）· `max_context_chars`（**默认 4000**） |

⇒ ⇒ **两个「预算」字段只在 `scope.rs` 内出现**（定义 + 默认值 + builder + 单测）⇒ ⇒ **零生产消费**；
⇒ ⇒ 而生产侧的默认值与它**逐字相同** ⇒ ⇒ **同一套策略被写了两遍**。

**⚠️ 动手前必须先分清的同名问题（已查清）**：
`ProactiveRecallPolicy`（**已接生产**）= 「**要不要主动召回** + 每次召回多少」
（`enabled` + `budget` + `confidence_threshold` + `min_candidate_score` + `visible_scopes`）；
⇒ ⇒ **「主动召回预算」这个能力并没有缺失**（已由 `ProactiveRecallPolicy.budget` 承担）
⇒ ⇒ **不需要**把 `proactive_budget` 再并进生产查询类型（那会引入**第四份预算真相**）。

**⇒ 处置（已落地）**：按 **B** 归组销账 ——
`docs/status/unwired-ledger.md` §5.1「重复实现」表**新增一行**：
`scope.rs` 的 `RecallPolicy` | 召回策略已分三处承担（`MemoryRecallQuery` 查询面 ·
`ProactiveRecallPolicy` 主动召回决策+预算 · `MemoryModule` 两个预算字段）
⇒ ⇒ **一份真相**，**不动生产路径**，**不删 API**（删除走「真死 API」独立流程）。
⇒ ⇒ 连带更正 §5.5q 的「整族未接线」段：**`RecallPolicy` 销账** · **`DriftDetector` 已接线（原记录过期）**
· 另三族仍待判。

### 6.2 ★ 生产路径上的核心能力**没有 claim**

- `docs/status/claims.yaml` 共 **38 条**（2026-11-01 复核时；⚠️ **这个数会变** —— 它每天都在被人加），
  `level` 分布 **PRODUCTION WIRED 24 / IMPLEMENTED 13 / NOT IMPLEMENTED 1**；
- 与记忆相关的只有 **3 条**：`mem.lesson_object`（PRODUCTION WIRED）· `gap.memory_update`（PRODUCTION WIRED）·
  `mem.registered_namespace`（IMPLEMENTED）；
- ⚠️ **检索排序 / 注入契约没有任何 claim**，**而它们在产品生产路径上**：
  `tool.memory.retrieve` → `MemoryCoordinator::recall`（`coordinator.rs:659`）
  → `HybridRetrievalPipeline`（`coordinator.rs:801` 建 `Bm25LexicalCandidateSource` + 向量源）
  → **`coverage` 以权重 0.5 参与打分**（`retrieval_pipeline.rs:137`，常量在 `:469`）；
- ⇒ ⇒ **「B1 `coverage` 接进生产排序」这条成果，在真源里查不到** ⇒ ⇒ **口径缺位**。
  ⚠️ `docs/status/claims.yaml` 是**真源** ⇒ ⇒ 改动属**他（总指挥）**的决策面
  ⇒ ⇒ **建议由他登记**，本线只提供**证据**（上面的调用链 + 文件:行号）。

---

## 7. ⭐ CI 长期红的根因（**已修**，不是代码问题）

**现象**：CI 上 `Build workspace tests` / `cargo test --workspace` / `cargo nextest` **反复 exit 101**，
本地同命令退出码 0。**逐层实测**：

| 层 | 实测 | 结论 |
|---|---|---|
| 内存 | 构建峰值 RSS **2.96 GB**（`-j 2`） | ❌ 不是 OOM |
| 日志 | Actions 日志 API **403** | ⇒ 读不到 CI 报错 ⇒ 改用模拟 |
| **磁盘** | 本地 target **101 GB** | ✅ **runner 14 GB SSD ⇒ 写满 = exit 101** |
| **模拟复现** | `tmpfs -o size=14G`：默认配置**占满 98%（剩 363 MB）** | ✅ 根因闭合 |

**修法**（5 个 workflow 加顶层 env，**不删测试、不改口径**）：
`CARGO_INCREMENTAL=0` · `CARGO_PROFILE_{DEV,TEST}_DEBUG=0` · `CARGO_PROFILE_{DEV,TEST}_STRIP=symbols`

| 配置 | target | 耗时 | 单个测试二进制 |
|---|---|---|---|
| 默认 | **101 GB** | 10m14s | — |
| `debug=0`+`incremental=0` | 14 GB | 3m21s | 160 MB |
| **+ `strip=symbols`**（采用） | **7.5 GB** | **3m20s** | **76 MB** |

⚠️ **关键**：`strip=debuginfo` **实测无效**（仍 14 GB）⇒ 因为每个测试二进制的
**`STRTAB` 73.6 MiB** + `SYMTAB` 9.5 MiB **不是调试信息** ⇒ 必须 `strip=symbols`。
⚠️ `coverage.yml` **故意不改**（tarpaulin 要调试信息）。
⚠️ **代价**：CI 无调试信息 ⇒ 失败时 backtrace 少符号/行号（**权衡，不是免费**）。

**为什么这事卡的是记忆线**：记忆线相对 main 新增 **87 个测试/评测二进制**
（examples 1→29、集成测试 13→58、记忆 crate 行数 56,286→78,495）⇒ **把磁盘撑爆了**。

---

## 8. 下一步清单（**按可做性排序**，标注归属）

| # | 事项 | 归属 | 阻塞 |
|---|---|---|---|
| 1 | ✅ **已完成**：`RecallPolicy` **归组销账** + `DriftDetector` **已接线更正**（限定名级复核，已落 `unwired-ledger.md` §5.1 / §5.5q） | **记忆 crate = 本线** | — |
| 1b | `DailyNote` / `AdaptiveBaseline` / `ClusterStore` 三族：**逐条判「归组 / 有意不接 / 真缺口」**并落账 | **记忆 crate = 本线** | 需先读各模块自述（`//!` 头注释）再判 |
| 2 | 更正 `unwired-ledger.md` 里 `DriftDetector` 那条（**已接线**） | 他在改该文件 ⇒ 报备 | 无 |
| 3 | **P2 运行时触发器**：谁在夜里推进 `CognitiveDreamState::Consolidating` | `core` / `orchestration` = **他的区** ⇒ 需对表 | 跨区 |
| 4 | **检索排序 / 注入契约**的 claim 登记 | `claims.yaml` = **真源，他的决策面** | 需他拍板 |
| 5 | 符号级守卫改**限定名计数** + PyO3 豁免（让「未接线清单」不再误导） | `scripts/` + 基线 ⇒ **改判据 ⇒ 需对表** | 跨区 + 基线只减不增流程 |
| 6 | 记忆线的 CI 闸门转绿验证（本次磁盘修复的效果） | 本线已推 ⇒ 等 CI | runner 队列饱和 |

---

## 9. 相关文件索引

| 文件 | 作用 |
|---|---|
| `docs/status/unwired-ledger.md` | **未接线台账**（§五 含人工过筛的四类判定 + **判据盲区报备**）⇒ **本文的事实基线** |
| `docs/status/claims.yaml` | 四级口径**真源**（38 条 · 会变） |
| `docs/04-internal/backend-enhancement/02-axes/C-memory-deepening.md` | 记忆轴**设计**（分期与验收线「不许感觉更聪明」）⇒ **输入，不是现状** |
| `scripts/audit_unwired_modules.py` · `scripts/audit_unwired.py` | 未接线审计（符号级 / 模块级）⇒ ⚠️ 判据盲区见 §4 |
| `scripts/golden-baseline.json` · `scripts/check_golden_baseline.py` | **P1 评估集**真源与闸门 |
| `crates/engine/memory/src/consolidation.rs` | **P2 夜间整编**（4 个 `pub fn` 入口，库内已接，缺运行时触发） |
| `crates/engine/memory/src/scope.rs` | `RecallPolicy`（§6.1 的重复实现） |
| `crates/engine/runtime-assembly/src/canonical/memory_module.rs` | 生产记忆模块（7 个操作：`retrieve`/`write`/`update`/`delete`/`search`/`compact`/`organize`） |
| `.github/workflows/golden-baseline.yml` | 记忆线闸门（含**runner 容量体检 artifact**） |
