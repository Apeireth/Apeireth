#!/usr/bin/env python3
"""核对「**速查表**里的当前状态数字」与**实测基线**是否**逐字一致**。

# 为什么要有这个脚本

`docs/status/memory-quality-route-map-2026-10-27.md` 是「**活文档**」
（给下一个动手的人的速查表）⇒ 它里面的「当前状态」数字**必须是真的**。

⚠️ **实测踩过的坑**：改了检索代码 ⇒ 基线里的数变了 ⇒
但**文档里的数字没改** ⇒ ⇒ 人会拿**过期数字**做决策
⇒ ⇒ **文档里的数字过期，比没有数字更糟**（它看起来可信）。

⇒ 本脚本把那 5 个「**当前状态**」数字与 `scripts/golden-baseline.json`
（= **实测基线**）**逐字对照** ⇒ **不一致就失败**。

# 口径（**刻意很窄**）

* **只**核对速查表「当前状态」表里的 5 个数字
  （`recall_at_k` × 2 · `groundedness` × 2 · `delivery_rate` × 2）
  ⇒ **不**核对历史/实验数字（那些是**当时的**实测值 ⇒ **不该**随基线变）
* 数字按**四位小数**比对（与速查表的写法一致）
* 基线里**没有**的字段 ⇒ **跳过**（不假装有）

# 用法

```sh
python3 scripts/check_doc_numbers.py
```
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE_PATH = REPO_ROOT / "scripts" / "golden-baseline.json"
ROUTE_MAP_PATH = REPO_ROOT / "docs" / "status" / "memory-quality-route-map-2026-10-27.md"


def load_baseline() -> dict:
    if not BASELINE_PATH.exists():
        print(f"基线不在：{BASELINE_PATH}", file=sys.stderr)
        return {}
    try:
        return json.loads(BASELINE_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        print(f"基线 JSON 坏了：{error}", file=sys.stderr)
        return {}


def baseline_numbers(baseline: dict) -> dict[str, dict[str, float]]:
    out: dict[str, dict[str, float]] = {}
    for item in baseline.get("corpora", []):
        name = item.get("name")
        if not name:
            continue
        entry: dict[str, float] = {}
        for label in ("recall_at_k", "groundedness", "delivery_rate"):
            value = item.get(label)
            if isinstance(value, (int, float)):
                entry[label] = float(value)
        out[name] = entry
    return out


def route_map_numbers() -> dict[str, dict[str, float]]:
    """从速查表的「当前状态」表里抽出数字。

    ⚠️ **刻意只取第一张表**（「当前状态」）⇒ 后面的表是**历史/实验**数字
    （那些是**当时的**实测值 ⇒ **不该**随基线变）。
    """
    if not ROUTE_MAP_PATH.exists():
        return {}
    text = ROUTE_MAP_PATH.read_text(encoding="utf-8")
    start = text.find("## 一、当前状态")
    if start < 0:
        return {}
    end = text.find("## 二、", start)
    section = text[start:end if end > 0 else len(text)]
    numbers: dict[str, dict[str, float]] = {}
    for line in section.splitlines():
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 3:
            continue
        label = cells[0]
        if label.startswith("---") or "量" in label and "L1" in label:
            continue
        values: list[float] = []
        for cell in cells[1:3]:
            match = re.search(r"(\d+\.\d{4})", cell.replace("*", ""))
            if match:
                values.append(float(match.group(1)))
        if len(values) == 2:
            if "recall" in label:
                numbers.setdefault("recall_at_k", {})["redchamber"] = values[0]
                numbers["recall_at_k"]["watershed"] = values[1]
            elif "groundedness" in label:
                numbers.setdefault("groundedness", {})["redchamber"] = values[0]
                numbers["groundedness"]["watershed"] = values[1]
            elif "送达率" in label:
                numbers.setdefault("delivery_rate", {})["redchamber"] = values[0]
                numbers["delivery_rate"]["watershed"] = values[1]
    return numbers


def main() -> int:
    baseline = load_baseline()
    if not baseline:
        print("⇒ **没法比**（不许当通过）", file=sys.stderr)
        return 1
    expected = baseline_numbers(baseline)
    actual = route_map_numbers()
    if not actual:
        print("速查表不在或「当前状态」表没解析出来 ⇒ **没法比**", file=sys.stderr)
        return 1

    problems: list[str] = []
    checked = 0
    for label, per_corpus in sorted(actual.items()):
        for corpus, value in sorted(per_corpus.items()):
            base_value = expected.get(corpus, {}).get(label)
            if base_value is None:
                continue
            checked += 1
            if abs(base_value - value) > 5e-5:
                problems.append(
                    f"速查表的「{corpus} · {label}」= {value:.4f}，"
                    f"但实测基线 = {base_value:.4f}"
                    f" ⇒ **文档里的数字过期了**"
                    f"（过期数字比没有数字更糟：它看起来可信）"
                )

    print("=" * 72)
    print("速查表数字 vs 实测基线")
    print("=" * 72)
    for label, per_corpus in sorted(actual.items()):
        for corpus, value in sorted(per_corpus.items()):
            base_value = expected.get(corpus, {}).get(label)
            mark = "—" if base_value is None else ("✓" if abs(base_value - value) <= 5e-5 else "✗")
            print(f"  {mark} {corpus:<12} {label:<16} 文档 {value:.4f}"
                  + ("" if base_value is None else f" · 基线 {base_value:.4f}"))

    if problems:
        print("\n[check_doc_numbers] 发现问题：", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print(f"\nOK: {checked} 个数字与实测基线逐字一致")
    return 0


if __name__ == "__main__":
    sys.exit(main())
