# 经验沉淀线 · 进度页（做到哪了）

```text
[Document-Meta]
Document: docs/status/experience-distillation-progress.md
Class: STATUS
Status: 🟢 活跃
Last-Modified: 2026-11-08
Author: lead（经验沉淀线）
```

> **本页是「做到哪了」的唯一权威页**（`AGENTS.md` §0.2：状态只引文件，不引对话记忆）。
> **体系入口** ⇒ [engineering-experience/README.md](../04-internal/engineering-experience/README.md)（提炼层 L3 的 8 篇 + 模板库）；
> **本批产物** ⇒ [06-truth-and-guard.md](../04-internal/engineering-experience/06-truth-and-guard.md)（真源矿）·
> [07-workline-and-merge.md](../04-internal/engineering-experience/07-workline-and-merge.md)（工作线矿）。
> **工作线登记** ⇒ [workline-registry.md](workline-registry.md)。
> **宪章回写** ⇒ [CONSTRUCTION-DISCIPLINE.md](../04-internal/backend-enhancement/CONSTRUCTION-DISCIPLINE.md) §七（机器化纪律，8 条）。

---

## 0. 一句话

> **批 L4（L 轴第二战役：2026-10-20 → 2026-11-08）已沉淀**：新增 2 矿（12 条 + 15 条 = **27 条**可复用经验）·
> 1 个模板 · 1 条新守卫（经验族自己的守卫，此前该体系**一条守卫都没有**）· 机器层失败模式**双向互引**接通。
> **本批同时补上体系的最后一个缺口**：`docs/README.md` 流水线写的「门禁检查同类坑二次出现时是否被引用」⇒ **已实现**（口径：`IMPLEMENTED`，守卫已在门禁登记表里）。

## 1. 波次看板

| 波 | 范围 | 状态 | 落地证据 | 还差什么 |
|---|---|---|---|---|
| **L4-W0** | 读体系（8 篇 + 宪章 + 模板 + 机器层知识库） | ✅ **已落地** | 本页 + [族内 README](../04-internal/engineering-experience/README.md)（8 篇表） | — |
| **L4-W1** | 盘点在途/不在途台账（凭据绊线 / 能力创造线 / 跨线合并收口 / 声明真源） | ✅ **已落地** | 06 矿 12 条 + 07 矿 15 条 | — |
| **L4-W2** | 产出新矿 + 模板 + 新守卫 | ✅ **已落地** | `docs/04-internal/engineering-experience/06-truth-and-guard.md` · `07-workline-and-merge.md` · `templates/experience-entry-template.md` · `scripts/check_experience_anchors.py` | — |
| **L4-W3** | 纳入体系（宪章回写 / 登记表 / 门禁清单 / 机器层互引） | ✅ **已落地** | 宪章 §七（8 条）· 登记表新增行 · `contract.toml` 新增 gate · `.harness/knowledge/failure-patterns.yaml` 增补 | — |
| **L4-W4** | 回流与复核（下一批从哪挖） | 🟡 **部分落地** | [L4-mining-map.md](../04-internal/engineering-experience/L4-mining-map.md)（**采矿图已补**：挖了什么 / 没挖什么 / 下一批候选） | ❌ 「按需求/日期做全量复习」机制未建（待 owner 裁决，见 §2 第 5 条） |

## 2. 本批「还差什么」（逐条带判据）

| # | 差什么 | 为什么没做 | 判据（做完怎么验） |
|---|---|---|---|
| 1 | **经验族此前零守卫** ⇒ 条目退化成散文、出处指到已删文件、机器层与人读层各说各话 | 本批补 `scripts/check_experience_anchors.py`（段型 + 锚点可达 + 索引覆盖 + 双向互引）⇒ **已消** | `python3 scripts/check_experience_anchors.py --check` ⇒ exit 0 |
| 2 | **06/07 两矿的条目未被既有守卫覆盖**（既有守卫只管文档链接/分类，不管经验结构） | 本批新增段型表（`REQUIRED_SEGMENTS`）⇒ **已消** | 同上（缺段即红） |
| 3 | **机器层 10 条失败模式未进经验矿**（`failure-patterns.yaml` 与矿族互不知情） | 本批由 06 矿条目 10/11 收编 ⇒ **已消** | 同上（yaml 每个 id 必须被引用） |
| 4 | **L 轴采矿图（2026-10-15）自记「本轴封账」** ⇒ 新战役无采矿图可指 | 本批已补 [L4-mining-map.md](../04-internal/engineering-experience/L4-mining-map.md)（**已证** = 两矿 27 条 + 取材口径；**已否** = 6 条明确不挖 + 理由）⇒ **已消** | ✅ 已闭环：采矿图存在且能被族内 README 索引到；`python3 scripts/check_doc_class.py --check` ⇒ 该页 `Class: ROUTE` 合规 |
| 5 | **「按周期复习」机制未建**（矿只增不复习 ⇒ 旧条目会随代码漂移） | 属机制设计，需 owner 拍板（谁在什么时点复习、复习失败怎么算） | ❌ 待裁决：默认处置 = **不复习**（条目靠锚点可达性被动兜底）|
| 6 | **能力可达率闸门未接 CI**（`check_memory_reachability.py` 在仓内 grep 零命中；提交点名「要总指挥加那一步」至今未做） | 属**别线**写域（workflow 归 CI 线）⇒ 本线不越权改 | ❌ 待归口：判据 = `grep -r check_memory_reachability .github/workflows/` 命中 |
| 7 | **`memory-reachability-baseline.txt` 的分母/比率过期**（仍写 132 / 52%，实测 133 / 51%） | 属该守卫的基线维护，须由该守卫归口线重生成 | ❌ 待归口：判据 = `python3 scripts/check_memory_reachability.py` 输出的分母与基线一致 |

## 3. 本批的守卫（退出码可判）

```bash
python3 scripts/check_experience_anchors.py --check   # 经验族：段型 + 锚点可达 + 索引覆盖 + 机器层互引 + 【守卫】段含可跑命令
python3 scripts/check_failure_knowledge.py             # 机器层：知识引用的守卫必须真实存在
python3 scripts/check_doc_class.py --check            # 06/07 两矿与新模板已分类
python3 scripts/docs_links.py --check                 # 断链 0
python3 scripts/docs_index.py --check                 # 索引覆盖（INDEX.csv 含新篇）
python3 scripts/docs_truth.py --verify --check        # 声明真源（本次 0 触碰 claims.yaml）
```

> ⚠️ **他线在途提示**（2026-11-08 实测）：工作树有别线的未提交改动 ⇒ `docs_truth --check` / `wiring_report --verify` /
> `audit_unwired --check` / `arch_guard --strict` 四道门禁**当时为红** ⇒ **归属他线，本线不代改、不代盖**。
> 本页只登记**本线**产物的状态。
