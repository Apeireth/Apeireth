"""Cross-small-model CI: report (R9-DevOps / R9-DEV-001).

主 00:56 任何人都能接手: 报告可读, 一目了然.
主 00:44 质量工程化: Markdown + JSON 双格式, 报告自动产出.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence

from .harness import HarnessResult
from .runner import summarize


def render_json(results: Sequence[HarnessResult]) -> str:
    """CI 报告 JSON 化."""
    return json.dumps(
        {
            "summary": summarize(results),
            "results": [r.to_dict() for r in results],
        },
        ensure_ascii=False,
        indent=2,
    )


def render_markdown(results: Sequence[HarnessResult],
                    pass_threshold: float = 0.50,
                    title: str = "Cross-Small-Model CI Report") -> str:
    """CI 报告 Markdown (主 00:56 一目了然)."""
    lines: List[str] = []
    s = summarize(results)
    verdict = "✅ ALL PASS" if s["all_pass"] else f"⚠️ {s['n_passed']}/{s['n_models']} PASS"
    lines.append(f"# {title} ({verdict})")
    lines.append("")
    lines.append("## Summary")
    lines.append(f"- 模型数: {s['n_models']}")
    lines.append(f"- 通过数: {s['n_passed']}")
    lines.append(f"- available 数: {s['n_available']}")
    lines.append(f"- 平均 subscore: {s['avg_subscore']:.4f}")
    lines.append(f"- PASS 阈值: subscore >= {pass_threshold}")
    lines.append("")
    lines.append("## HQB 4 维结果 (主 18:52 HARNESS §2.3)")
    lines.append("")
    lines.append("| 模型 | family | available | SC | NR | EV | CDT | subscore | PASS | 推理次数 | 耗时 (s) |")
    lines.append("|------|--------|-----------|-----|-----|-----|------|----------|------|----------|----------|")
    for r in results:
        mark = "✅" if r.passed else "❌"
        avail = "✅" if r.available else "—"
        err = f" — err: {r.error}" if r.error else ""
        lines.append(
            f"| {r.model_name} | {r.family} | {avail} | {r.sc:.4f} | {r.nr:.4f} | {r.ev:.4f} | {r.cdt:.4f} "
            f"| {r.subscore:.4f} | {mark} | {r.n_inferences} | {r.elapsed_sec:.2f}{err} |"
        )
    lines.append("")
    # 跨域明细
    lines.append("## CDT 跨域迁移 (主 18:52)")
    lines.append("")
    lines.append("| 模型 | code | math | reasoning | creative |")
    lines.append("|------|------|------|-----------|----------|")
    for r in results:
        d = r.cdt_per_domain
        lines.append(
            f"| {r.model_name} | {d.get('code', 0):.4f} | {d.get('math', 0):.4f} "
            f"| {d.get('reasoning', 0):.4f} | {d.get('creative', 0):.4f} |"
        )
    lines.append("")
    lines.append("## 哲学守门")
    lines.append("")
    lines.append("- 主 17:58+20:46 不假装: adapter 加载失败 → is_available=False, 不混入 PASS")
    lines.append("- 主 17:43 实事求是: subscore 来自 4 维真测, 不 hardcode")
    lines.append("- 主 19:33 走在前人经验上: 借鉴 V36 HQB + V160 HQB 4 dims + V1085 HQB core")
    lines.append("")
    return "\n".join(lines) + "\n"


def write_report(results: Sequence[HarnessResult],
                 path: str | Path = "reports/cross-small-model-ci.md",
                 pass_threshold: float = 0.50) -> Path:
    """写 Markdown 报告 (主 00:56 任何人都能接手: 报告自动产出)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(render_markdown(results, pass_threshold=pass_threshold), encoding="utf-8")
    return p
