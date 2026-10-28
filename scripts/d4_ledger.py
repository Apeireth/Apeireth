#!/usr/bin/env python3
"""生成 **D4 候选池台账**（`scripts/d4-ledger.json`）—— B0-c 的结构部分。

# 为什么有这个脚本（设计稿 §8 B0-c）

设计稿的 B0-c 验收标准是：

> **36 个模块进候选池，并各有一行数字**

⇒ ⇒ 那需要一个**机器可检**的台账：每个「整块未接线」的模块都要有一行，
且每行必须说清 **它测什么、由哪份卷子测、测出来多少**。

# 三个字段的口径（**别自创**）

* `layer`：按设计稿 §3 的 D0–D5（不是按目录）；
* `consumes` / `produces`：对齐 `d4_eval::AlgoInput` / `AnnotationKind`
  （`Text` / `TextAndRanking` / `FullPipeline` ⇒ `Gap` / `Association` / `Pattern` / `Score`）；
* `measurable_by`：**这份卷子能不能测它**
  * `retrieval`：能用「检索卷子」量（召回/送达的增减）；
  * `annotation`：只能量「标注质量」（需要另一份卷子）；
  * `not-applicable`：它不是算法（存储/facade/适配器）⇒ **不该进候选池**。

# ⚠️ 这个脚本**不产生数字**

它只把**候选池结构**建起来（谁在里面、各自怎么测）。
`delta_recall` 等数字由**测量脚本**回填 ⇒ 本次先留 `null` 并标 `status`。

# 用法

    python3 scripts/d4_ledger.py --gen       # 重写台账
    python3 scripts/d4_ledger.py --check     # 只校验（CI 用）
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / "scripts" / "d4-ledger.json"
AUDIT = ROOT / "scripts" / "audit_unwired_modules.py"
MEMORY_SRC = ROOT / "crates" / "engine" / "memory" / "src"

# ---------------------------------------------------------------------------
# **人工分类表**（必须覆盖审计报出的每一个模块 ⇒ 守卫会核对）
#
# 口径：只给「它是什么」和「怎么测」；**不给分数**（分数由测量回填）。
# ---------------------------------------------------------------------------
CLASSIFICATION: dict[str, dict[str, str]] = {
    # ---- 能上「检索卷子」的：产出可参与排序的分数/结构 ----
    "memory_rank": {
        "layer": "D5",
        "kind": "score",
        "consumes": "Text",
        "produces": "Score",
        "measurable_by": "retrieval",
        "note": "`parse_importance` / `group_bonus` / `recency_score` ⇒ 直接是排序分项",
    },
    "hallways": {
        "layer": "D4",
        "kind": "association",
        "consumes": "TextAndRanking",
        "produces": "Association",
        "measurable_by": "retrieval",
        "note": "实体共现「走廊」⇒ 命中走廊可加权",
    },
    "graph_algo": {
        "layer": "D1",
        "kind": "traversal",
        "consumes": "TextAndRanking",
        "produces": "Association",
        "measurable_by": "retrieval",
        "note": "加权 Dijkstra / 有向 BFS ⇒ 图检索扩展",
    },
    "dedup": {
        "layer": "D0",
        "kind": "filter",
        "consumes": "Text",
        "produces": "Score",
        "measurable_by": "retrieval",
        "note": "指纹归一 ⇒ 去掉近似重复后**腾出** top-k 名额",
    },
    "vector_distance": {
        "layer": "D2",
        "kind": "distance",
        "consumes": "Text",
        "produces": "Score",
        "measurable_by": "retrieval",
        "note": "距离函数（配合向量源才有意义）",
    },
    # ---- 只能量「标注质量」：产出的是标注，不是分数 ----
    "five_dimensional": {
        "layer": "D4",
        "kind": "topology",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "五维时空拓扑 ⇒ 产标注",
    },
    "ensemble": {
        "layer": "D5",
        "kind": "aggregation",
        "consumes": "FullPipeline",
        "produces": "Score",
        "measurable_by": "annotation",
        "note": "预测市场聚合 ⇒ 需要多来源预测",
    },
    "intent_brier": {
        "layer": "D5",
        "kind": "calibration",
        "consumes": "FullPipeline",
        "produces": "Score",
        "measurable_by": "annotation",
        "note": "Brier 校准 ⇒ 需要预测 vs 结果",
    },
    "online_calibration": {
        "layer": "D5",
        "kind": "calibration",
        "consumes": "FullPipeline",
        "produces": "Score",
        "measurable_by": "annotation",
        "note": "在线校准器",
    },
    "calibration_critic": {
        "layer": "D5",
        "kind": "calibration",
        "consumes": "FullPipeline",
        "produces": "Score",
        "measurable_by": "annotation",
        "note": "校准批评者",
    },
    "chronicle_crystallizer": {
        "layer": "D3",
        "kind": "refine",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "编年史结晶 ⇒ 提炼层",
    },
    "principles": {
        "layer": "D3",
        "kind": "refine",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "原则提炼",
    },
    "daily_summary": {
        "layer": "D3",
        "kind": "refine",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "日报摘要",
    },
    "sleep_cycle": {
        "layer": "D4",
        "kind": "consolidation",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "睡眠周期巩固",
    },
    "dream_consolidation": {
        "layer": "D4",
        "kind": "consolidation",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "做梦巩固",
    },
    "research_roaming_memory": {
        "layer": "D4",
        "kind": "explore",
        "consumes": "Text",
        "produces": "Association",
        "measurable_by": "annotation",
        "note": "漫游记忆",
    },
    "research_non_interference": {
        "layer": "D4",
        "kind": "explore",
        "consumes": "TextAndRanking",
        "produces": "Gap",
        "measurable_by": "annotation",
        "note": "研究不干扰保证",
    },
    "three_layer": {
        "layer": "D4",
        "kind": "facade",
        "consumes": "Text",
        "produces": "Pattern",
        "measurable_by": "annotation",
        "note": "三层记忆 facade",
    },
    "streams": {
        "layer": "D1",
        "kind": "structure",
        "consumes": "Text",
        "produces": "Association",
        "measurable_by": "annotation",
        "note": "流式结构",
    },
    "history_streams": {
        "layer": "D1",
        "kind": "structure",
        "consumes": "Text",
        "produces": "Association",
        "measurable_by": "annotation",
        "note": "历史流",
    },
    # ---- 2026-10-28 H1 修正后**新暴露**的（此前被 examples/ 引用藏住）----
    "signal_sources": {
        "layer": "D4",
        "kind": "expansion",
        "consumes": "Text",
        "produces": "Score",
        "measurable_by": "retrieval",
        "note": "5 种非答案侧查询扩展（共现/同主题/时间锚/段落首句/原文首句）",
    },
    "offline_embed": {
        "layer": "D2",
        "kind": "pseudo-semantic",
        "consumes": "Text",
        "produces": "Score",
        "measurable_by": "retrieval",
        "note": "字符 n-gram 哈希投影（**不是语义模型**，模块文档自述）",
    },
    "locomo_corpus": {
        "layer": "—",
        "kind": "corpus",
        "consumes": "Text",
        "produces": "Gap",
        "measurable_by": "not-applicable",
        "note": "LoCoMo 语料加载器（对照卷；设计稿 §7.1 明确不作主卷）",
    },
    "cold_archive": {
        "layer": "D0",
        "kind": "infra",
        "consumes": "Text",
        "produces": "Gap",
        "measurable_by": "not-applicable",
        "note": "冷层归档（B4 基建）",
    },
    "delivery_metrics": {
        "layer": "—",
        "kind": "eval",
        "consumes": "TextAndRanking",
        "produces": "Score",
        "measurable_by": "not-applicable",
        "note": "送达率诊断量（§6.1.3；**观察项**不参与闸门）",
    },
    "forgetting_metrics": {
        "layer": "D5",
        "kind": "eval",
        "consumes": "Text",
        "produces": "Score",
        "measurable_by": "not-applicable",
        "note": "§6.2 四个遗忘指标（评估基建）",
    },
    "golden_diag": {
        "layer": "—",
        "kind": "eval",
        "consumes": "TextAndRanking",
        "produces": "Gap",
        "measurable_by": "not-applicable",
        "note": "召回缺口诊断 Hit/RankedOut/NotInPool（评估基建）",
    },
    "quality_metrics": {
        "layer": "—",
        "kind": "eval",
        "consumes": "TextAndRanking",
        "produces": "Score",
        "measurable_by": "not-applicable",
        "note": "§6.1 十八项指标（评估基建）",
    },
    "robustness_metrics": {
        "layer": "—",
        "kind": "eval",
        "consumes": "TextAndRanking",
        "produces": "Score",
        "measurable_by": "not-applicable",
        "note": "L3 鲁棒/成长指标（评估基建）",
    },
    "swap_flow": {
        "layer": "—",
        "kind": "infra",
        "consumes": "Text",
        "produces": "Gap",
        "measurable_by": "not-applicable",
        "note": "算法切换流程（B3 基建）",
    },
    # ---- 基础设施 / 适配器 ⇒ 不该进「算法候选池」 ----
    "admission_gate": {"layer": "—", "kind": "infra", "consumes": "TextAndRanking",
                       "produces": "Gap", "measurable_by": "not-applicable", "note": "准入闸（策略，不是算法）"},
    "agent_trace": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                    "measurable_by": "not-applicable", "note": "轨迹记录"},
    "arbitration": {"layer": "—", "kind": "infra", "consumes": "TextAndRanking", "produces": "Gap",
                    "measurable_by": "not-applicable", "note": "仲裁"},
    "in_memory": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                  "measurable_by": "not-applicable", "note": "内存后端"},
    "enhanced": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                 "measurable_by": "not-applicable", "note": "增强包装"},
    "facade": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
               "measurable_by": "not-applicable", "note": "门面"},
    "pipe": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
             "measurable_by": "not-applicable", "note": "管道"},
    "retrieval": {"layer": "D2", "kind": "infra", "consumes": "Text", "produces": "Score",
                  "measurable_by": "not-applicable", "note": "检索门面"},
    "vector_rebuild": {"layer": "D2", "kind": "infra", "consumes": "Text", "produces": "Gap",
                       "measurable_by": "not-applicable", "note": "派生索引重建（B2 基建）"},
    "forget_coordinator": {"layer": "D5", "kind": "infra", "consumes": "Text", "produces": "Gap",
                           "measurable_by": "not-applicable", "note": "遗忘协调（D-9 基建）"},
    "progression": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                    "measurable_by": "not-applicable", "note": "进度记录"},
    "librarian": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                  "measurable_by": "not-applicable", "note": "图书管理员（路由）"},
    "preference_store": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                         "measurable_by": "not-applicable", "note": "偏好存储"},
    "dual_track": {"layer": "—", "kind": "infra", "consumes": "Text", "produces": "Gap",
                   "measurable_by": "not-applicable", "note": "双轨"},
    "git_corpus": {"layer": "—", "kind": "corpus", "consumes": "Text", "produces": "Gap",
                   "measurable_by": "not-applicable", "note": "语料加载器"},
    "wiki_corpus_real": {"layer": "—", "kind": "corpus", "consumes": "Text", "produces": "Gap",
                         "measurable_by": "not-applicable", "note": "语料加载器"},
    "wiki_fs": {"layer": "—", "kind": "corpus", "consumes": "Text", "produces": "Gap",
                "measurable_by": "not-applicable", "note": "语料加载器"},
    "golden_baseline": {"layer": "—", "kind": "eval", "consumes": "Text", "produces": "Score",
                        "measurable_by": "not-applicable", "note": "考卷基线（评估基建）"},
    "golden_injection2": {"layer": "—", "kind": "eval", "consumes": "Text", "produces": "Score",
                          "measurable_by": "not-applicable", "note": "注入对照实验台"},
    "perltqa_adapter": {"layer": "—", "kind": "eval", "consumes": "Text", "produces": "Gap",
                        "measurable_by": "not-applicable", "note": "外部卷适配器（B0-a）"},
}


# ---------------------------------------------------------------------------
# **已测结果**（由 `cargo run -p apeireth-memory --example locomo_d4_probe` 产出）
#
# ⚠️ 纪律：**只记真结果**。若「测法本身无效」⇒ 记 `not-measurable-as-designed`
# **并写清原因**，绝不记一个假数字冒充结果（2026-10-28 两次假结果教训）。
# ---------------------------------------------------------------------------
MEASUREMENTS: dict[str, dict] = {
    "signal_sources": {
        "status": "measured",
        "delta_recall": None,
        "delta_grounded": None,
        "measure_note": (
            "**5 种非答案侧信号全灭**（此前的严格 A/B，见 "
            "`docs/status/memory-quality-route-map-2026-10-27.md` §三 否证 #5）："
            "紅樓夢 同主题 −0.0231 / 共现 −0.0189 / 段落首句 −0.0441 / 原文首句 −0.0515；"
            "水滸傳 同主题 −0.1734 / 共现 −0.2802 / 段落首句 −0.0504 / 原文首句 −0.0383。"
            "⇒ 结论：**单一弱信号打不过 BM25**（这是「逐个试」的结论，不是「特征无用」的结论）。"
            "⚠️ 数字取自上述路线图（当时的严格 A/B）⇒ 本次未重跑。"
        ),
    },
    "offline_embed": {
        "status": "measured",
        "delta_recall": None,
        "delta_grounded": None,
        "measure_note": (
            "**字符 n-gram 哈希投影实测更差**：−0.1534 / −0.3165"
            "（见路线图 §三 否证 #2）⇒ 它**不是语义模型**（模块文档自述），"
            "原理上也难赢（桶空间被词面饱和）。⚠️ 数字取自路线图 ⇒ 本次未重跑。"
        ),
    },
    "memory_rank": {
        "status": "not-measurable-as-designed",
        "delta_recall": None,
        "delta_grounded": None,
        "measure_note": (
            "**重排索引对 BM25 完全无效** ⇒ 实测 Δ = 0.0000（精确 0）"
            "⇒ 因为 `Bm25LexicalCandidateSource` 会按自己的 BM25 分数**重新排序** "
            "⇒ ⇒ 换索引顺序等于没做。"
            "要测它必须**融合分数** ⇒ 而检索路径**当前没有分数融合钩子**"
            "⇒ ⇒ 这是**架构缺口**（B2/B3 的 `search_rrf` / `search_weighted` 存在但未接）。"
        ),
    },
    "dedup": {
        "status": "not-measurable-as-designed",
        "delta_recall": None,
        "delta_grounded": None,
        "measure_note": (
            "`overlap_ratio` 是**字符集 Jaccard** ⇒ 短英文句子之间字符集高度重叠 "
            "⇒ 它把大量对话轮次判成近似重复 ⇒ 实测 −0.2558 **是「被误用」不是「它有害」**。"
            "该模块的定位是**写入门禁**（去近似重复的 episode），不是检索侧重排"
            "⇒ ⇒ 拿它当重排器测，测法本身无效。"
        ),
    },
}


def unwired_memory_modules() -> list[str]:
    """从审计脚本拿**权威**的未接线模块名（同一次运行的输出）。"""
    proc = subprocess.run(
        [sys.executable, str(AUDIT)],
        capture_output=True,
        text=True,
        cwd=str(ROOT),
        timeout=600,
    )
    text = proc.stdout
    names: list[str] = []
    for line in text.splitlines():
        match = re.match(r"^  crates/engine/memory/src/(.+)\.rs  \(", line)
        if match:
            names.append(Path(match.group(1)).name)
    return names


def build() -> dict:
    audited = unwired_memory_modules()
    if not audited:
        raise SystemExit("审计脚本没报出 engine/memory 的模块 ⇒ 输出格式变了？")
    # ⚠️ **口径**：台账 = **分类表全集 ∪ 审计结果**。
    #
    # 为什么不能只用审计结果：**审计是「当前未接线」的瞬时快照**
    # ⇒ 一旦某模块被 example / 测试引用（例如 B0-c 自己的探针），它就从审计里消失
    # ⇒ ⇒ **恰好丢掉最该跟踪的那些**（2026-10-28 实测：`memory_rank`/`dedup` 掉了）。
    # ⇒ ⇒ 所以「进过候选池」这件事由**分类表**记录（永久），审计只用来发现**新**的。
    audited_set = set(audited)
    modules = list(CLASSIFICATION.keys()) + [
        name for name in audited if name not in CLASSIFICATION
    ]
    rows = []
    missing = []
    for module in modules:
        meta = CLASSIFICATION.get(module)
        if meta is None:
            missing.append(module)
            meta = {
                "layer": "—",
                "kind": "unclassified",
                "consumes": "Text",
                "produces": "Gap",
                "measurable_by": "not-applicable",
                "note": "⚠️ **未分类**（补 CLASSIFICATION 表）",
            }
        files = list(MEMORY_SRC.glob(f"{module}.rs")) + list(MEMORY_SRC.glob(f"{module}/*.rs"))
        loc = 0
        for path in files:
            try:
                loc += sum(1 for _ in path.open(encoding="utf-8", errors="ignore"))
            except OSError:
                pass
        measurement = MEASUREMENTS.get(module, {})
        if module not in audited_set:
            default_status = "left-pool"
        elif meta["measurable_by"] != "not-applicable":
            default_status = "pending"
        else:
            default_status = "out-of-scope"
        rows.append(
            {
                "module": module,
                "loc": loc,
                **meta,
                # ⚠️ 数字**只**来自真实测量；测法无效 ⇒ 记 `not-measurable-as-designed`
                # 并写清原因，**不许**记假数字。
                "delta_recall": measurement.get("delta_recall"),
                "delta_grounded": measurement.get("delta_grounded"),
                "measure_note": measurement.get("measure_note"),
                "status": measurement.get("status", default_status),
            }
        )
    # ⚠️ **台账是粘性的**：一旦某模块进过候选池，就**永久留档**
    # ⇒ ⇒ 否则「被引用一次」就会让它从审计里消失 ⇒ **恰好丢掉最该跟踪的那些**
    # （2026-10-28 实测：探针引用 `memory_rank`/`dedup` 后，两行直接从台账里掉了）。
    seen = {row["module"] for row in rows}
    if LEDGER.exists():
        try:
            previous = json.loads(LEDGER.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            previous = {}
        for old_row in previous.get("rows", []):
            if old_row.get("module") in seen:
                continue
            kept = dict(old_row)
            # 保留原分类与已测结果；状态改成「已离池」，并说明原因。
            kept["status"] = "left-pool"
            kept["measure_note"] = (
                (kept.get("measure_note") or "")
                + " \n⇒ **已离池**：它是本次台账生成时**不再被审计判为未接线**"
                "（例如被 example / 测试引用）⇒ 但**留档保留**，因为进过候选池。"
            ).strip()
            rows.append(kept)

    rows.sort(key=lambda row: (-row["loc"], row["module"]))
    by_layer: dict[str, int] = {}
    for row in rows:
        by_layer[row["layer"]] = by_layer.get(row["layer"], 0) + 1
    return {
        "version": "d4-candidates/v1",
        "spec": "docs/01-architecture/memory-knowledge-depth-system-spec.md §8 B0-c",
        "board": "docs/status/spec-batch-status.md",
        "source": "scripts/audit_unwired_modules.py",
        "module_count": len(rows),
        "candidate_count": sum(1 for row in rows if row["measurable_by"] != "not-applicable"),
        "measured_count": sum(1 for row in rows if row["status"] == "measured"),
        "not_measurable_count": sum(
            1 for row in rows if row["status"] == "not-measurable-as-designed"
        ),
        "left_pool_count": sum(1 for row in rows if row["status"] == "left-pool"),
        "by_layer": by_layer,
        "unclassified": missing,
        "rows": rows,
    }


def render(data: dict) -> str:
    return json.dumps(data, ensure_ascii=False, indent=2, sort_keys=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--gen", action="store_true", help="重写台账")
    group.add_argument("--check", action="store_true", help="校验台账与代码一致")
    args = parser.parse_args()

    data = build()
    if data["unclassified"]:
        print(
            f"⚠️ 未分类模块 {len(data['unclassified'])} 个：{data['unclassified']}",
            file=sys.stderr,
        )

    if args.gen:
        LEDGER.write_text(render(data), encoding="utf-8")
        print(
            f"✓ 已写 {LEDGER.relative_to(ROOT)}："
            f"{data['module_count']} 个模块 · "
            f"其中**候选**（可测）{data['candidate_count']} 个 · "
            f"已测 {data['measured_count']} 个 · "
            f"试过但**测法无效** {data['not_measurable_count']} 个"
        )
        print(f"  分层：{data['by_layer']}")
        return 0

    # --check
    if not LEDGER.exists():
        print(f"FAIL: 台账不存在（{LEDGER.relative_to(ROOT)}）⇒ 跑 --gen", file=sys.stderr)
        return 1
    on_disk = json.loads(LEDGER.read_text(encoding="utf-8"))
    expected = render(data)
    actual = LEDGER.read_text(encoding="utf-8")
    if actual == expected:
        print(
            f"OK: D4 候选池台账与代码一致（{data['module_count']} 模块 · "
            f"候选 {data['candidate_count']} · 已测 {data['measured_count']} · "
            f"测法无效 {data['not_measurable_count']}）"
        )
        return 0
    # 只报「哪些行不一致」⇒ 便于修，而不是让人 diff 整个 JSON
    on_disk_rows = {row["module"]: row for row in on_disk.get("rows", [])}
    new_rows = {row["module"]: row for row in data["rows"]}
    added = sorted(set(new_rows) - set(on_disk_rows))
    removed = sorted(set(on_disk_rows) - set(new_rows))
    changed = sorted(
        name
        for name in set(new_rows) & set(on_disk_rows)
        if {k: v for k, v in new_rows[name].items() if k not in ("delta_recall", "delta_grounded", "status", "measure_note")}
        != {k: v for k, v in on_disk_rows[name].items() if k not in ("delta_recall", "delta_grounded", "status", "measure_note")}
    )
    print("FAIL: D4 候选池台账与代码不一致 ⇒ 跑 `python3 scripts/d4_ledger.py --gen`", file=sys.stderr)
    if added:
        print(f"  新增模块（台账里没有）：{added}", file=sys.stderr)
    if removed:
        print(f"  台账里有、代码已不再是未接线：{removed}", file=sys.stderr)
    if changed:
        print(f"  分类变了：{changed}", file=sys.stderr)
    if data["unclassified"]:
        print(f"  未分类（必须先补 CLASSIFICATION）：{data['unclassified']}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
