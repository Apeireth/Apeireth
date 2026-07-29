"""Cross-small-model CI: report + diff + badge (R9-DevOps / R9-DEV-002).

主 00:56 任何人都能接手: 报告可读, 一目了然.
主 00:44 质量工程化: Markdown + JSON 双格式, 报告自动产出.
主 13:31 大胆激进: 跨模型差异可视化 + CI badge 自动生成 (W3 增强).
主 17:58+20:46 不假装: 真模型加载失败 → 显式记录, 不混入 PASS 数据.
主 17:43 实事求是: diff / badge 数据全部从 HarnessResult 真测来, 不 hardcode.
主 19:33 走在前人经验上: 借鉴 shields.io 2014 endpoint badge + GitHub Actions badge 2020
+ d3-compare 2017 cross-table 模式 + W3C EARL 1998 报告对比.
"""
from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Dict, List, Optional, Sequence, Tuple

from .harness import HarnessResult
from .runner import summarize


# ---------------------------------------------------------------------------
# 跨模型差异 (主 13:31 大胆激进: 三向对比 fixture vs 真模型, 含 HQB 4 维 lift delta)
# ---------------------------------------------------------------------------
def compute_diff(results: Sequence[HarnessResult],
                 baseline_name: str = "fixture-7b-v1") -> Dict[str, Any]:
    """跨模型差异 (主 17:43 实事求是: 真测真算, 不假装).

    选定 baseline (默认 fixture-7b-v1), 与其他模型做三向对比:
      - SC / NR / EV / CDT / subscore 各维 delta (target - baseline)
      - 真模型加载失败 → delta 标注为 null + error 字段
      - 汇总 lift_summary: mean_delta, max_delta, min_delta, n_loaded, n_failed

    主 17:58 不假装: 真模型 unavailable 时, 显式记录 "unavailable", 不假装有 delta.
    """
    baseline: Optional[HarnessResult] = None
    others: List[HarnessResult] = []
    for r in results:
        if r.model_name == baseline_name:
            baseline = r
        else:
            others.append(r)

    def _delta_row(target: HarnessResult) -> Dict[str, Any]:
        if baseline is None:
            return {
                "target": target.model_name,
                "family": target.family,
                "available": target.available,
                "delta_sc": None, "delta_nr": None, "delta_ev": None,
                "delta_cdt": None, "delta_subscore": None,
                "error": "no baseline found",
            }
        # 主 17:58: unavailable → delta 显式 null
        if not target.available:
            return {
                "target": target.model_name,
                "family": target.family,
                "available": False,
                "delta_sc": None, "delta_nr": None, "delta_ev": None,
                "delta_cdt": None, "delta_subscore": None,
                "error": target.error or "unavailable",
                "target_subscore": target.subscore,
            }
        return {
            "target": target.model_name,
            "family": target.family,
            "available": True,
            "delta_sc": round(target.sc - baseline.sc, 4),
            "delta_nr": round(target.nr - baseline.nr, 4),
            "delta_ev": round(target.ev - baseline.ev, 4),
            "delta_cdt": round(target.cdt - baseline.cdt, 4),
            "delta_subscore": round(target.subscore - baseline.subscore, 4),
            "error": None,
            "target_subscore": round(target.subscore, 4),
            "baseline_subscore": round(baseline.subscore, 4),
        }

    rows = [_delta_row(r) for r in others]
    # 主 17:43 实事求是: lift_summary 仅汇总 available 的真实 delta
    avail_deltas = [r["delta_subscore"] for r in rows
                    if r.get("delta_subscore") is not None]
    n_loaded = sum(1 for r in rows if r.get("available"))
    n_failed = sum(1 for r in rows if not r.get("available"))
    lift_summary = {
        "n_targets": len(rows),
        "n_loaded": n_loaded,
        "n_failed": n_failed,
        "mean_delta": round(sum(avail_deltas) / len(avail_deltas), 4) if avail_deltas else None,
        "max_delta": round(max(avail_deltas), 4) if avail_deltas else None,
        "min_delta": round(min(avail_deltas), 4) if avail_deltas else None,
        "baseline_name": baseline_name,
        "baseline_subscore": round(baseline.subscore, 4) if baseline else None,
    }
    return {
        "computed_at": time.time(),
        "computed_at_iso": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime()),
        "baseline": baseline.to_dict() if baseline else None,
        "rows": rows,
        "lift_summary": lift_summary,
    }


def render_diff_table(diff: Dict[str, Any]) -> str:
    """跨模型差异 Markdown 表格 (主 13:31 大胆激进 + 主 00:56 一目了然)."""
    lines: List[str] = []
    baseline = diff.get("baseline") or {}
    lines.append("## 跨模型差异 (baseline = {})".format(diff["lift_summary"]["baseline_name"]))
    lines.append("")
    if not baseline:
        lines.append("⚠️ no baseline found")
        return "\n".join(lines) + "\n"
    lines.append(
        f"baseline subscore = **{baseline['subscore']:.4f}** "
        f"(SC={baseline['sc']:.4f} NR={baseline['nr']:.4f} "
        f"EV={baseline['ev']:.4f} CDT={baseline['cdt']:.4f})"
    )
    lines.append("")
    lines.append("| target | family | available | ΔSC | ΔNR | ΔEV | ΔCDT | Δsubscore | 备注 |")
    lines.append("|--------|--------|-----------|-----|-----|-----|------|-----------|------|")
    for r in diff["rows"]:
        avail = "✅" if r.get("available") else "❌"
        d_sc = r.get("delta_sc")
        d_nr = r.get("delta_nr")
        d_ev = r.get("delta_ev")
        d_cdt = r.get("delta_cdt")
        d_sub = r.get("delta_subscore")
        def _fmt(v: Optional[float]) -> str:
            return f"{v:+.4f}" if isinstance(v, (int, float)) else "—"
        note = r.get("error") or ""
        lines.append(
            f"| {r['target']} | {r['family']} | {avail} "
            f"| {_fmt(d_sc)} | {_fmt(d_nr)} | {_fmt(d_ev)} | {_fmt(d_cdt)} "
            f"| {_fmt(d_sub)} | {note} |"
        )
    lines.append("")
    ls = diff["lift_summary"]
    lines.append("### lift_summary")
    lines.append(f"- n_targets: {ls['n_targets']}")
    lines.append(f"- n_loaded: {ls['n_loaded']}")
    lines.append(f"- n_failed: {ls['n_failed']}")
    mean = ls.get("mean_delta")
    mx = ls.get("max_delta")
    mn = ls.get("min_delta")
    lines.append(f"- mean_delta: {f'{mean:+.4f}' if mean is not None else '—'}")
    lines.append(f"- max_delta:  {f'{mx:+.4f}' if mx is not None else '—'}")
    lines.append(f"- min_delta:  {f'{mn:+.4f}' if mn is not None else '—'}")
    lines.append("")
    return "\n".join(lines) + "\n"


def write_diff(diff: Dict[str, Any], path: str | Path = "reports/cross-model-diff.json") -> Path:
    """写 cross_model_diff.json (主 13:31: 跨模型差异可视化数据)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(diff, ensure_ascii=False, indent=2), encoding="utf-8")
    return p


# ---------------------------------------------------------------------------
# CI Badge (主 13:31 大胆激进: 自动生成, 借鉴 shields.io 2014 endpoint)
# ---------------------------------------------------------------------------
def render_badge(results: Sequence[HarnessResult],
                 diff: Optional[Dict[str, Any]] = None,
                 pass_threshold: float = 0.50) -> Dict[str, Any]:
    """CI badge 数据 (主 13:31 + 主 00:44: status: pass/fail/lift_summary).

    借鉴 shields.io 2014 endpoint schema:
      {schemaVersion, label, message, color}

    主 17:43 实事求是: status 来自真测, 不 hardcode.
      - all_pass=True → "pass" + green
      - any failed → "fail" + red
      - 全部 unavailable → "unknown" + lightgrey
      - 兜底 → "mixed" + yellow
    """
    s = summarize(results)
    n = s["n_models"]
    n_pass = s["n_passed"]
    n_avail = s["n_available"]
    avg = s["avg_subscore"]

    if n == 0:
        status = "unknown"
        color = "lightgrey"
    elif s["all_pass"]:
        status = "pass"
        color = "green"
    elif n_avail == 0:
        status = "unknown"
        color = "lightgrey"
    elif n_pass == 0:
        status = "fail"
        color = "red"
    else:
        status = "mixed"
        color = "yellow"

    badge = {
        "schemaVersion": 1,
        "label": "cross-small-model-ci",
        "message": f"{n_pass}/{n} pass · avg {avg:.4f}",
        "color": color,
        "status": status,
        "pass_threshold": pass_threshold,
    }

    # 主 13:31 lift_summary: 从 diff 拿
    lift_summary: Dict[str, Any] = {}
    if diff and "lift_summary" in diff:
        ls = diff["lift_summary"]
        lift_summary = {
            "n_loaded": ls.get("n_loaded"),
            "n_failed": ls.get("n_failed"),
            "mean_delta": ls.get("mean_delta"),
            "max_delta": ls.get("max_delta"),
            "min_delta": ls.get("min_delta"),
            "baseline_name": ls.get("baseline_name"),
        }
        # 若有 lift, 在 message 附加一段
        if ls.get("mean_delta") is not None:
            badge["message"] = f"{n_pass}/{n} pass · lift {ls['mean_delta']:+.4f}"
    elif diff is not None:
        # diff 已计算但无 lift_summary 字段 → 兜底
        lift_summary = {"n_loaded": 0, "n_failed": 0, "mean_delta": None}

    return {
        "schemaVersion": 1,
        "badge": badge,
        "lift_summary": lift_summary,
        "computed_at": time.time(),
        "computed_at_iso": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime()),
        "n_models": n,
        "n_passed": n_pass,
        "n_available": n_avail,
        "avg_subscore": round(avg, 4),
        "pass_threshold": pass_threshold,
    }


def render_badge_markdown(badge: Dict[str, Any]) -> str:
    """Badge Markdown 字符串 (主 00:56 任何人都能接手: README 可直接粘)."""
    b = badge.get("badge", badge)
    label = b.get("label", "ci")
    msg = b.get("message", "n/a")
    color = b.get("color", "lightgrey")
    # 借鉴 shields.io URL: https://img.shields.io/badge/{label}-{message}-{color}
    return f"https://img.shields.io/badge/{label}-{msg.replace(' ', '_')}-{color}.svg\n"


def write_badge(results: Sequence[HarnessResult],
                path: str | Path = "reports/ci-badge.json",
                diff: Optional[Dict[str, Any]] = None,
                pass_threshold: float = 0.50) -> Path:
    """写 badge.json (主 13:31 + 主 00:44 质量工程化: 自动产出)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    badge = render_badge(results, diff=diff, pass_threshold=pass_threshold)
    p.write_text(json.dumps(badge, ensure_ascii=False, indent=2), encoding="utf-8")
    return p


# ---------------------------------------------------------------------------
# 既有 CI 报告 (R9-DEV-001 保留)
# ---------------------------------------------------------------------------
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
                    title: str = "Cross-Small-Model CI Report",
                    diff: Optional[Dict[str, Any]] = None,
                    badge: Optional[Dict[str, Any]] = None) -> str:
    """CI 报告 Markdown (主 00:56 一目了然 + W3 增强: 含 diff + badge)."""
    lines: List[str] = []
    s = summarize(results)
    verdict = "✅ ALL PASS" if s["all_pass"] else f"⚠️ {s['n_passed']}/{s['n_models']} PASS"
    lines.append(f"# {title} ({verdict})")
    lines.append("")
    if badge is not None:
        # 主 13:31: badge 自动生成, README 可粘
        b = badge.get("badge", badge)
        lines.append(f"![ci]({render_badge_markdown(badge).strip()})")
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
    # W3 增强: 跨模型差异
    if diff is not None:
        lines.append(render_diff_table(diff).rstrip("\n"))
        lines.append("")
    lines.append("## 哲学守门")
    lines.append("")
    lines.append("- 主 17:58+20:46 不假装: adapter 加载失败 → is_available=False, 不混入 PASS")
    lines.append("- 主 17:43 实事求是: subscore 来自 4 维真测, 不 hardcode")
    lines.append("- 主 19:33 走在前人经验上: 借鉴 V36 HQB + V160 HQB 4 dims + V1085 HQB core + shields.io badge")
    lines.append("- 主 13:31 大胆激进: 跨模型差异 + badge 自动生成 (W3 增强)")
    lines.append("")
    return "\n".join(lines) + "\n"


def write_report(results: Sequence[HarnessResult],
                 path: str | Path = "reports/cross-small-model-ci.md",
                 pass_threshold: float = 0.50,
                 diff: Optional[Dict[str, Any]] = None,
                 badge: Optional[Dict[str, Any]] = None) -> Path:
    """写 Markdown 报告 (主 00:56 任何人都能接手: 报告自动产出 + W3 增强)."""
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(
        render_markdown(results, pass_threshold=pass_threshold, diff=diff, badge=badge),
        encoding="utf-8",
    )
    return p
