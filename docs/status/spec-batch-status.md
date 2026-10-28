# 设计稿批次看板（B0–B4 / D0–D5）—— **当前实施到哪儿**

[Document-Meta]
Document: docs/status/spec-batch-status.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-10-28
Author: lead

> 依据：`docs/01-architecture/memory-knowledge-depth-system-spec.md`（321 行 / 11 章）
> 本页是**唯一权威进度页** ⇒ 每批/每层状态变了就改这里，**别只留在对话里**。
> 最后核对：**2026-10-28**（按代码实测，不是按记忆）
>
> 📍 **开工前先认领工作线**：[`workline-registry.md`](workline-registry.md)
> （工作线 ↔ 设计稿 ↔ 进度页 ↔ 守卫 的登记表；`AGENTS.md` §0 的规则落地在这张表上）

---

<!-- BEGIN:auto-facts -->
<!-- 本块由 `python3 scripts/check_batch_status.py --gen` 生成；**别手改**（改了 --check 会失败） -->

| 事实 | 值 |
|---|---|
| `d0_hash_chain` · D0 保真：哈希链 | **在写入路径** |
| `d4_default` · D4 姿态（默认关？） | **生产已接 · **默认关**** |
| `d4_modules` · B0-c 候选池模块数 | **50** |
| `d4_candidates` · B0-c 可测候选数 | **22** |
| `d4_measured` · B0-c 已测行数 | **3** |
| `d4_not_measurable` · B0-c 测法无效行数 | **1** |
| `b1_memory_verify` · B1 `memory verify` CLI | **存在** |
| `b4_offline_archive` · B4 脱机归档模块 | **存在** |
| `b4_handover` · B4 handover 模块 | **存在** |
| `b4_migrate_rollback` · B4 迁移回滚模块 | **存在** |
| `cli_export` · `memory export` 分派 | **已分派** |
| `cli_handover` · `memory handover` 分派 | **已分派** |
| `b2_contracts` · B2 三接口契约 | **3/3** |
| `b3_shadow_run` · B3 影子双跑模块 | **存在** |
| `b0a_adapter` · B0-a PerLTQA 适配器 | **存在** |
| `b0a_reproduce` · B0-a `reproduce.sh` | ****无**** |
| `claims` · 状态声明 | **29 条（延后 1 · 豁免 3）** |

<!-- END:auto-facts -->

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
| **B0-a** | 外部卷：核许可/结构 → 写适配器 → 跑**现状基线** | 出第一份成绩单；`reproduce.sh` 可重跑 | 🟡 **适配器有，数据下不来** | `perltqa_adapter.rs`（260 行 + 189 行测试）已写 ⇒ **数据从没落地 · 无 `reproduce.sh` · 无基线数字** · ⚠️ **2026-10-28 网络已通**（设计稿 R4「TLS 被阻断」前提**已失效**） |
| **B0-b** | 内部卷：挖 2000 → 精标 300 | 卷子定稿 + 基线数字 | ✅ **达标（301 条 ≥ 300 目标 + 基线已出）** | ✅ 记录抽取器 `scripts/internal_golden.py`（**5,201 条**章节级记录，**确定性 id**）<br>✅ 校验器（片段必须是正文**逐字**子串）<br>✅ 考卷 `docs/03-reference/internal-golden-v1.jsonl` **301 条**（owner 2026-10-28 裁定：**精标由 AI 自己认真读，不必找人**）<br>✅ **基线数字（301 题）**：`recall@10 = 0.8738` · nDCG 0.7572 · groundedness 0.2724 · **送达率 0.3118**<br>⚠️ **自造卷红利**：题是读着文档写的 ⇒ recall **偏高** ⇒ 干净对照看 LoCoMo<br>✅ **301/300 达标**（可继续加题但已满足验收） |
| **B0-c** | **D4 离线评估器**：不接线也能量「某算法开启的增量价值」 | **36 个模块进候选池，各有一行数字** | 🟡 **候选池已建，数字才 2 行** | ✅ **候选池台账** `scripts/d4-ledger.json`（**40 模块 / 20 个可测候选**）+ 守卫 `scripts/d4_ledger.py --check`（台账与代码不一致即失败）<br>⚠️ **2 行测完但「测法无效」**（见 §2.1）⇒ **有效数字仍是 0 行** |

### 2.1 B0-c 的两个实测发现（2026-10-28，**都是架构级**）

用 `cargo run -p apeireth-memory --example locomo_d4_probe` 把「能在检索卷子上测」的两个跑了一遍
（单证据题 1,122 题 · k=10 · 基线 `recall@10 = 0.6221`）：

| 算法 | 实测 | **为什么不作数** |
|---|---|---|
| `memory_rank` | Δ = **+0.0000**（精确 0） | **重排索引对 BM25 完全无效** ⇒ `Bm25LexicalCandidateSource` 会按自己的分数**重新排序** ⇒ ⇒ 换索引顺序等于没做 |
| `dedup` | Δ = **−0.2558** | `overlap_ratio` 是**字符集 Jaccard** ⇒ 短英文句子之间字符集高度重叠 ⇒ 把大量轮次判成重复 ⇒ **「被误用」不是「它有害」**；它的定位是**写入门禁**，不是检索侧重排 |

⇒ ⇒ ⇒ **核心结论（这条比数字重要）**：

> **当前检索路径没有「分数融合钩子」** ⇒ 绝大多数 D4 算法**根本无法被"开一下试试"**——
> 它们要么需要融合进 BM25 分数，要么需要另一份卷子（标注质量）。
> ⇒ ⇒ **B0-c 的真前置条件是 B2/B3**：把 `search_rrf` / `search_weighted`（已写好、未接）
> 接进检索路径 ⇒ 才有地方挂 D4 的分数。

⚠️ 这也是**两次假结果的教训**：两个数字初版都错
（`memory_rank` 写成 `as i64` 把 0~2 的小数截成 0；`dedup` 未做「答案重映射」⇒ 被并掉的答案记成"检索不到"）
⇒ ⇒ **已修**，并把「测法无效」作为**台账里的一等状态**（`not-measurable-as-designed`）。

---

## 2.2 防线加固（2026-10-28，owner 指令「漏洞全补上」）

> 主题：把「防漂移」从 **AI 自觉** 搬到 **仓库机器**。
> 详见审计结论：`docs/status/anti-drift-audit-2026-10-28.md`。

| 编号 | 漏洞 | 状态 | 实测效果 |
|---|---|---|---|
| **H1** | 未接线判据把 `examples/`/`benches/` 当生产引用 ⇒ **写个 example 就算接线** | ✅ **已修** | 整块未接线模块 **81 → 93**（+12 个此前被 example 藏住）；记忆 crate 侧 41 → 51 |
| **H2** | `claims.yaml` 的 `verify_cmd` **从不被执行** ⇒ 装饰品 | ✅ **已修** | 新增 `check_claims_verify.py` + 独立 CI；**首次真跑抓到 6 条问题** ⇒ 修完后现 **26 跑 / 2 豁免 / 1 延后**（3 条 Tauri 声明已**真验通过 11 passed** 并恢复 `PRODUCTION WIRED`）|
| **H3** | `STALE` 只查登记过的 source ⇒ **漏列即静默漂** | ✅ **已修** | 加**目录级兜底**（窄目录 ≤8 文件）⇒ **抓到 2 处真漏列**（`process/mod.rs` · `rubric_advice.rs`）⇒ 已补源并重验（19 / 3 passed）后 bump |
| **H4** | 尺寸棘轮可被「搬家」满足 | ⏳ **未做** | 计划：加 per-crate 总量兜底 |
| **H5** | `check_doc_caliber` 是**形式守卫**（写标记就过） | ⏳ **未做** | 计划：标 `PRODUCTION WIRED` 的 claim 必须 `verify_cmd` 跑通（H2 已提供底座） |
| **H6** | **慢性红**（`cargo +nightly doc` 三次全红）⇒ 红失去信号价值 | ⏳ **未做** | 计划：清红或显式隔离并带到期日 |
| **H7** | 批次进度页**零守卫** ⇒ 漂一整天没人喊 | ⏳ **未做** | 计划：`check_batch_status.py`（本页可判定断言） |
| **H8** | 文档**无分类** ⇒ 约束靠自觉 | ✅ **已修** | `check_doc_class.py`（九类 + 每类强制结构 + 存量棘轮 491 份）；已示范分类 8 份 |

### 2.2.1 ⚠️ 自己的守卫也差点「形式可满足」（值得记）

`check_doc_class.py` 第一版用「关键词出现在正文里」判定 ⇒
**我自己造的探针文档写「本文没有批次/验收/不做章节」竟然通过了**
⇒ ⇒ 这正是我刚批评的那类漏洞 ⇒ **改为标题级 / 列表表格行级锚定**，并加两条反证：
反着写的 `SPEC` 失败 · 只在散文里提「已修」的 `AUDIT` 失败。

### 2.2.2 已知未修（**如实**）

* ⚠️ **纠正**：曾写「`src-tauri/Cargo.lock` 与 `Cargo.toml` 不同步 = 真漂移」——**说错了**。
  复测 `cargo metadata --locked` **exit=0**、lock **一行未变**；当时失败是**本机缺 crate 缓存**。
* **3 条 Tauri 声明已真验**（`cargo test --locked`：**154 / 39 / 9 passed**）
  ⇒ 已从 `IMPLEMENTED` **恢复 `PRODUCTION WIRED`**；
  `claims-verify.yml` 也已同步补装 Tauri 系统库 + sidecar 占位（否则 CI 会假红）；
* **只剩 1 条延后**：`sec.terminal_permission_surfaces`（`pnpm test`）——
  本机 **Node v24** 未编译 TS 支持（`ERR_NO_TYPESCRIPT`），项目 CI 固定 **node 22**
  ⇒ **不代表该 claim 成立**；
* ⚠️ **B0-a 被网络卡住（不是许可、不是代码）**：PerLTQA 数据 `perltqa.json` 8.63MB +
  `perltmem.json` 8.99MB；`github.com`（git 协议）**超时不通**，只有 `raw.githubusercontent.com` 通，
  实测速率约 **3.5 KB/s**，随后**断崖到 0**（手动 64KB 块 90 秒零字节）
  ⇒ ⇒ 分块续传脚本已跑到 **917KB / 8.6MB** 后**反复失败** ⇒ **已中止**（不空转）。
  ⇒ 结论：**等网络恢复再跑**；适配器与许可处置都已就绪，数据一到即可出中文基线。
* H1 残余：**只有 `build.rs`** —— **已修**（加进 `is_test_file`）。
  ⚠️ 「`#[cfg(test)]` 内联模块」那条我原先也说错了，它早已被 `split_code_test()` 排除。
  彻底解决「**引用 ≠ 可达**」仍需**从装配根做可达性分析**（未做）。

## 3. 下一步（按设计稿顺序，优先级从高到低）

1. **B2/B3 的「分数融合钩子」**（**B0-c 的真前置**）
   ⇒ 把 `search_rrf` / `search_weighted`（已写好、未接）接进检索路径
   ⇒ ⇒ 否则 D4 的分数**没有地方挂**（已实测：重排索引对 BM25 无效）；
2. **B0-c 续**：钩子有了之后，把 20 个候选**逐个**跑出有效数字
   （当前 2 行 = `not-measurable-as-designed`，**有效数字 0 行**）；
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
