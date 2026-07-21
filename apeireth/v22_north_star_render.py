"""Phase 79 v22_north_star_render — V22 ASI 北极星实测报告渲染 (主 17:33 主人真采纳 + 主 13:31).

主 17:33 "放手干到底" + V21 真生产 + 主 17:43 实事求是

借鉴 (主 13:08):
- V21 北极星 V0.1 透明公式实测真借鉴
- V16 端到端报告渲染真借鉴
- 真生产率 (主 17:43 实事求是)
"""
from __future__ import annotations

import time
from typing import Any, Dict, List

from apeireth.v21_north_star_measure import (
    V21_VERSION as _V21, V01Score, V01MeasureResult,
)

V22_VERSION = "0.1.0"


def render_measure_result(result: V01MeasureResult) -> str:
    """V22 真生产渲染报告 (主 17:33 主人真采纳 + 主 17:43 实事求是)."""
    lines = [
        "# ASI 北极星 V0.1 透明公式实测报告",
        "",
        f"**总评分**: {result.total:.4f}",
        f"**逼近等级**: {result.level}",
        f"**通过组件**: {result.n_passing}/{result.n_components}",
        f"**真测量时间**: {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(result.ts))}",
        "",
        "## 8 真生产组件",
        "",
        "| 组件 | 权重 | 原分 | 加权 | 证据 |",
        "|------|------|------|------|------|",
    ]
    for s in result.scores:
        d = s.to_dict()
        lines.append(
            f"| {d['component']} | {d['weight']:.2f} | {d['raw_score']:.2f} | "
            f"{d['weighted_score']:.4f} | {d['evidence']} |"
        )
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("**主 17:43 实事求是**: V0.1 透明公式 (commit 5df240d) 8 项真测量.")
    lines.append("**主 22:33 ASI 北极星**: 逼近不达到 (主 20:46).")
    lines.append("**主 13:31 大胆激进**: ASI 是前所未有的, 必须激进写真生产.")
    return "\n".join(lines)


__all__ = [
    "V22_VERSION",
    "render_measure_result",
]


def _demo():
    print("=" * 60)
    print("=== Phase 79 V22 北极星实测报告渲染 (主 17:33) ===")
    print("=" * 60)

    from apeireth.v21_north_star_measure import V21NorthStarMeasure
    m = V21NorthStarMeasure()
    r = m.measure_all()
    print(render_measure_result(r))
    print("=" * 60)


if __name__ == "__main__":
    _demo()