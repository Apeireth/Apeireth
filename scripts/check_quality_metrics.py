#!/usr/bin/env python3
"""度量体系的**覆盖率台账**（机器可检）。

# 为什么要有这个脚本

设计稿 `docs/01-architecture/memory-knowledge-depth-system-spec.md` §6.1 定了
**五层 20 项**质量指标 ⇒ 它们是「**好用**」的裁判。

⚠️ **实测发现**：这 20 项里，**大量只存在于文档、代码里没有**：
* L1 的「时间正确率」「矛盾检出率」⇒ 缺；
* L2 的「Token 效率」「Citation coverage」⇒ 缺；
* L3 四项 ⇒ 缺；
* L4 的「影子胜率」「回退时间」⇒ 缺；
* L5 的「handover 演练」⇒ 缺。

⇒ 「指标体系」这件事**很容易悄悄烂掉**：文档写了 20 项 ⇒ 看起来很全
⇒ 实际只有 5 项能算 ⇒ **没人发现**。

⇒ 本脚本把「**哪些指标有实现**」变成**可机检的台账**：
* **有实现**（代码里有对应模块）⇒ 标 `IMPLEMENTED`；
* **没实现** ⇒ 标 `MISSING` ⇒ **打印出来**（**不假装有**）。

# 口径

* 「有实现」= 仓里有**对应的实现模块或类型**（用**代码锚点**判定，不是靠人填表）；
* **不判定**「有没有接进生产」⇒ 那是**另一层**（PRODUCTION WIRED），
  本脚本只管「**能不能算**」；
* 台账文件：`scripts/quality-metrics-ledger.json`
  ⇒ **可手改**（人工裁决结果），但**每个条目都必须有代码锚点或明确的 `MISSING` 标记**。

# 用法

```sh
python3 scripts/check_quality_metrics.py            # 查台账与代码是否一致
python3 scripts/check_quality_metrics.py --report    # 只打印覆盖率报告
python3 scripts/check_quality_metrics.py --update    # 重扫代码锚点，更新台账
```
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LEDGER_PATH = REPO_ROOT / "scripts" / "quality-metrics-ledger.json"

# 设计稿 §6.1 的五层 20 项（**逐字照抄设计稿的指标名** ⇒ 不自创）。
LAYERS: dict[str, list[str]] = {
    "L1 定位": ["Recall@k", "nDCG@k", "时间正确率", "矛盾检出率"],
    "L2 注入": ["Evidence Precision", "Token 效率", "Groundedness", "Citation coverage"],
    "L3 鲁棒/成长": ["规模延展斜率", "噪声鲁棒", "确定性（逐字节）", "越权命中率（恒 0）"],
    "L4 可塑性": ["Swap cost", "影子胜率", "回退时间"],
    "L5 可继承": ["脱机可读", "schema 自说明覆盖", "handover 演练通过"],
}

# 每个指标 ⇒ **代码锚点**（正则）。命中文件里出现 ⇒ 「能算」。
# ⚠️ 锚点要**具体**（函数名/类型名），不要用泛词（否则到处都命中 = 假绿）。
ANCHORS: dict[str, list[tuple[str, str]]] = {
    "Recall@k": [("crates/engine/memory/src/golden_set.rs", r"pub recall_at_k: f64")],
    "nDCG@k": [("crates/engine/memory/src/golden_set.rs", r"pub ndcg_at_k: f64")],
    "时间正确率": [("crates/engine/memory/src/quality_metrics.rs", r"pub fn measure_temporal")],
    "矛盾检出率": [("crates/engine/memory/src/quality_metrics.rs", r"pub fn measure_contradiction")],
    "Evidence Precision": [("crates/engine/memory/src/golden_set.rs", r"evidence_precision")],
    "Token 效率": [("crates/engine/memory/src/quality_metrics.rs", r"pub fn measure_token_efficiency")],
    "Groundedness": [("crates/engine/memory/src/golden_set.rs", r"groundedness")],
    "Citation coverage": [("crates/engine/memory/src/quality_metrics.rs", r"pub fn measure_citation_coverage")],
    "规模延展斜率": [("crates/engine/memory/src/robustness_metrics.rs", r"pub fn measure_scale_slope")],
    "噪声鲁棒": [("crates/engine/memory/src/robustness_metrics.rs", r"pub fn measure_noise_robustness")],
    "确定性（逐字节）": [("crates/engine/memory/src/robustness_metrics.rs", r"pub fn measure_determinism")],
    "越权命中率（恒 0）": [("crates/engine/memory/src/robustness_metrics.rs", r"pub fn measure_overreach")],
    "Swap cost": [("crates/engine/memory/src/swap_flow.rs", r"pub struct SwapCost")],
    "影子胜率": [("crates/engine/memory/src/quality_metrics.rs", r"pub fn measure_shadow_win")],
    "回退时间": [("crates/engine/memory/src/quality_metrics.rs", r"pub fn measure_rollback_time")],
    "脱机可读": [("crates/engine/memory/src/offline_archive.rs", r"pub fn to_jsonl")],
    "schema 自说明覆盖": [("crates/engine/memory/src/offline_archive.rs", r"pub fn default_field_docs")],
    "handover 演练通过": [("crates/engine/memory/src/handover.rs", r"pub fn drill_handover")],
}


def scan() -> dict[str, list[str]]:
    """扫代码 ⇒ 每个指标命中了哪些锚点。"""
    hits: dict[str, list[str]] = {}
    for metric, anchors in ANCHORS.items():
        found: list[str] = []
        for relative, pattern in anchors:
            path = REPO_ROOT / relative
            if not path.exists():
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            if re.search(pattern, text):
                found.append(f"{relative}:{pattern}")
        hits[metric] = found
    return hits


def load_ledger() -> dict:
    if not LEDGER_PATH.exists():
        return {}
    try:
        return json.loads(LEDGER_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        print(f"台账 JSON 坏了：{error}", file=sys.stderr)
        return {}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", action="store_true", help="只打印覆盖率报告")
    parser.add_argument("--update", action="store_true", help="重扫锚点并更新台账")
    args = parser.parse_args()

    hits = scan()
    ledger = load_ledger()

    implemented = [m for m, found in hits.items() if found]
    missing = [m for m, found in hits.items() if not found]

    print("=" * 72)
    print("质量度量体系覆盖率（设计稿 §6.1 五层 20 项）")
    print("=" * 72)
    for layer, metrics in LAYERS.items():
        marks = []
        for metric in metrics:
            marks.append(f"{metric} ✅" if metric in implemented else f"{metric} ❌")
        print(f"  {layer}: " + " · ".join(marks))
    print("-" * 72)
    print(f"  可算：{len(implemented)} / {len(LAYERS) and sum(len(v) for v in LAYERS.values())} 项")
    if missing:
        print("  **未实现**（缺口**如实列出**，不许假装有）：")
        for metric in missing:
            print(f"    - {metric}")

    if args.update:
        payload = {
            "version": "quality-metrics-ledger/v1",
            "source": "docs/01-architecture/memory-knowledge-depth-system-spec.md §6.1",
            "implemented": implemented,
            "missing": missing,
            "anchors": {m: hits[m] for m in sorted(hits)},
        }
        LEDGER_PATH.write_text(
            json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        print(f"  ✓ 台账已更新：{LEDGER_PATH.relative_to(REPO_ROOT)}")
        return 0

    if args.report:
        return 0

    # 校验：台账里标了「已实现」的指标，**代码里必须真有**（⇒ 台账不许撒谎）。
    problems: list[str] = []
    for metric in ledger.get("implemented", []):
        if metric not in implemented:
            problems.append(
                f"台账说「{metric}」已实现，但代码锚点没命中 ⇒ **台账撒谎了**"
                f"（要么代码被删了，要么台账该改）"
            )
    for metric in ledger.get("missing", []):
        if metric in implemented:
            problems.append(
                f"台账说「{metric}」未实现，但代码里已实现 ⇒ **台账过期了**"
                f"（跑 --update 更新）"
            )
    if problems:
        print("\n[check_quality_metrics] 发现问题：", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print("\nOK: 台账与代码一致")
    return 0


if __name__ == "__main__":
    sys.exit(main())
