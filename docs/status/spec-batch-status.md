# 设计稿批次看板（B0–B4 / D0–D5）—— **当前实施到哪儿**

> 依据：`docs/01-architecture/memory-knowledge-depth-system-spec.md`（321 行 / 11 章）
> 本页是**唯一权威进度页** ⇒ 每批/每层状态变了就改这里，**别只留在对话里**。
> 最后核对：**2026-10-28**（按代码实测，不是按记忆）

---

## 0. 一句话

> **纵向**：D0 做到底（哈希链已进生产写入路径）· D1/D3/D5 挖了一半 ·
> **D2 只有词面那半边**（语义只在"联网 + 有 key"时存在）· **D4 挖了坑但一个数字没量**。
> **横向**：**B1 / B4 完成** · B2/B3 库级完成但**生产未接** ·
> **B0 整批没完成** —— 而设计稿 §8 明确写「B0 是所有后续的裁判，必须最先做」。

---

## 1. D0–D5 六层（设计稿 §3）

| 层 | 契约（承诺什么） | 状态 | 证据 |
|---|---|---|---|
| **D0 原始证据** | 原文 + 结构 + 双时间 + 出处 + 关系 + 上下文 + **访问史**（**唯一"绝不"可丢**） | ✅ **最扎实** | `episode`/`append_only`/`provenance`/`access_history` 生产接线；**哈希链进写入路径** `backend/sqlite.rs:120` → `memory_chain::put_chained_episode`；`memory verify` CLI 可定位坏记录；`docs/status/d0-fidelity-audit.md`（11 KB 缺口清单） |
| **D1 结构化** | 目录树 / 段落切分 / **实体** / 图谱边 | 🟡 **一半** | ✅ `wiki_corpus`(TOC) · `amem_graph` · `bitemporal_graph`（双时间，crate 内生产用）· ❌ **`extraction`（实体抽取）生产 0 调用** |
| **D2 语义索引** | 向量 / 关键词 / 主题；**可重建且可换代** | 🟡 **关键词全开，语义靠外部 API** | ✅ `hybrid_search`/`retrieval_pipeline`（BM25 恒开）· `persistent_vector`（真实现，设计稿"只有 re-export"已过期）· `vector_rebuild` 换代检测（19 测试）· ⚠️ **向量走 `OpenAiCompatibleEmbeddingProvider`**（env 配 URL+key+**联网**）⇒ **离线 = 没有向量召回** |
| **D3 提炼** | 摘要 / 里程碑 / 教训 / 原则；每条带 provenance 指回 D0 | 🟡 **一部分** | ✅ `milestone`（3 处）· `lesson`（1 处）· ❌ `daily_summary` / `principles` / `chronicle_crystallizer` **生产 0 调用** |
| **D4 模式** | **只产标注，不产记忆**；默认关；**不接线也能离线量增量价值** | 🟡 **挖了坑没量** | ✅ `betti_hole_detector` / `kuramoto_resonance` / `semantic_axis` = **生产已接但 DEFAULT OFF**（`APEIRETH_ENABLE_ABSORPTION_INSIGHT=1` 才开）· ❌ 五维 / 程序性 / 做梦 / 社区分层 / `hallways` / `layered_memo` / `ensemble` 未接 · ❌ **`d4_eval` 评估器已写好（405 行），但「各一行数字」= 0 行** |
| **D5 判断** | 置信度 / 校准 / **降权权重** | 🟡 **骨架** | ✅ `confidence.rs` · 双时间衰减（`bitemporal_graph`）· recency 衰减（`coordinator`，含 2026-10-27 接的相对时间锚点）· ❌ `calibration*` / `intent_brier` / `online_calibration` / `calibration_critic` **生产 0 调用**；**D-9 降权权重模型未接生产** |

⚠️ **判断口径**：本页**不**用「crate 外调用数」当接线判据（**该指标会误判**：
`bitemporal_graph` 就是 crate 内生产用、crate 外 0 调用）⇒ 判据是「**是否在 `MemoryCoordinator` 生产路径上**」。

---

## 2. B0–B4 五批（设计稿 §8）

| 批次 | 内容 | 验收标准 | 状态 | 还差什么 |
|---|---|---|---|---|
| **B1** | D0 保真审计 + 哈希链 + `memory verify` | 完整性可检出；保真缺口清单 | ✅ **完成** | — |
| **B4** | 脱机归档（**保原文**）+ 迁移回滚 + handover 演练 | 脱机可读；回滚可演练 | ✅ **完成** | 2026-10-27 补了 **CLI 出口**（`memory export` / `memory handover`，含自校验与演练） |
| **B2** | 三接口契约（`MemoryView` / `MemoryAnnotation` / `InjectionRenderer`）+ `rebuild` | 删派生索引 → 重建 → 检索结果一致 | 🟡 **库级完成，生产未接** | 三个 trait 都在、`vector_rebuild` 有 19 测试；**契约只被 example 调用** ⇒ 要接进生产读路径 |
| **B3** | 影子双跑 + `Swap cost` + 算法切换流程（含报告归档） | 换算法可零风险上线/回退 | 🟡 **库级完成，生产未接** | `shadow_run` / `swap_flow`（三道闸 + 报告归档 + 回退时间）齐；**只有 example 在用** |
| **B0-a** | 外部卷：核许可/结构 → 写适配器 → 跑**现状基线** | 出第一份成绩单；`reproduce.sh` 可重跑 | 🟡 **适配器有，成绩单没有** | `perltqa_adapter.rs`（260 行 + 189 行测试）已写 ⇒ **数据从没落地 · 无 `reproduce.sh` · 无基线数字** · ⚠️ **2026-10-28 网络已通**（设计稿 R4「TLS 被阻断」前提**已失效**） |
| **B0-b** | 内部卷：挖 2000（先看 20 条样本）→ 精标 300 | 卷子定稿 + 基线数字 | 🟡 **自动挖题有，精标没有** | 已挖 476（紅樓夢）/ 248（水滸傳）题；**人工精标 300 条 = 没做（需要人）** |
| **B0-c** | **D4 离线评估器**：不接线也能量「某算法开启的增量价值」 | **36 个模块进候选池，各有一行数字** | ❌ **完全没开始** | `d4_eval`（`AlgoProfile`/`AlgoVerdict`/`evaluate_all`）已写好 ⇒ **数字 0 行** |

---

## 3. 下一步（按设计稿顺序，优先级从高到低）

1. **B0-c**（**最该做**）：用 `d4_eval` + `golden_diag` + 现有卷子，把 D4 模块**逐个跑一行数字**
   ⇒ 这就是「把实验算法学起来」的**官方入口**；
2. **B0-a**：核 PerLTQA LICENSE → 落数据（**网络已通**）→ 出**中文主卷**成绩单 → 补 `reproduce.sh`；
3. **B0-b**：精标 300 条**需要人**（我只能先给 20 条样本）；
4. 之后 **B2 / B3** 接生产（契约与影子双跑已经齐了，只差接线）。

---

## 4. 怎么复核本页（**机器可检**，不靠人自觉）

```sh
# ① D0 哈希链是否真在写入路径
grep -n "put_chained_episode" crates/engine/memory/src/backend/sqlite.rs

# ② D4 默认关的开关
grep -n "APEIRETH_ENABLE_ABSORPTION_INSIGHT" crates/adapters/cli/src/lib.rs

# ③ D2 向量是否依赖外部 API
grep -n "OpenAiCompatibleEmbeddingProvider" crates/adapters/cli/src/lib.rs

# ④ B0-a 适配器在不在
ls crates/engine/memory/src/perltqa_adapter.rs crates/engine/memory/tests/perltqa_adapter.rs

# ⑤ B0-c 到底有没有数字（当前应为「没有台账文件」）
ls scripts/d4-ledger.json 2>/dev/null || echo "B0-c 未开始"
```

---

## 5. 变更纪律

* **状态变了必须改本页**（并在 `docs/INDEX.csv` 重新生成）；
* 引用状态时**只引本页**，不引对话记忆（2026-10-28 教训：上下文压缩会丢掉「我做过什么」，
  我曾因此误判 `golden_diag` 不存在、误判 `betti/kuramoto` 未接线）；
* 每批完成 ⇒ 把**验收证据路径**写进第 2 节的「还差什么」列（清空即完成）。
