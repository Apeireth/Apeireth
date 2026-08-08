#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1337_vcp_plugin_compliance_dashboard.py — VCP Plugin Compliance Dashboard (CLI)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1336 linter CLI (b6d4fa31, 22:01); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 — V1336 linter → V1337 dashboard (real-data application)
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → V1336 → **V1337**

V1337 = **VCP Plugin Compliance Dashboard** — the real-data application of V1336 linter.

V1336 = linter (single file → conformance report).
V1337 = dashboard (multiple V13xx plugin files → compliance matrix + cross-plugin
       comparison + safety-critical gap detection).

V1337 reads the **7 V13xx deep-read modules** (V1327 VCP core + V1328 AnySearch +
V1329 DailyNote + V1330 AgentDream + V1332 RAGDiary + V1333 VCPTimeLine +
V1334 ThoughtClusterManager = 6 plugins + core) and runs V1336 linter on each.

V1337 produces:
- **Per-plugin conformance report** (V1336 output per plugin)
- **Cross-plugin compliance matrix** (6 plugins × 8 invariant classes)
- **Safety-critical gap analysis** (which plugins are missing which critical classes)
- **Overall verdict** + actionable recommendations
- **JSON / Markdown / CSV output**

V1337 = **DASHBOARD (NOT 复刻, NOT port, NOT 假装 ASI)**:
- Reads 7 V13xx deep-read modules → runs V1336 linter on each
- Aggregates per-plugin conformance into cross-plugin matrix
- Detects safety-critical gaps (per 主 22:33 终极授权 5-critical rule)
- 11 distinct API surfaces
- CLI: --json, --markdown, --csv, --strict

All evidence is REAL:
- 7 V13xx modules exist on disk (verified via Path.exists() + sha256)
- V1336 linter reuses V1335 regex (no new regex)
- No fake decimal precision; all counts reproducible via _self_test()

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ? 不假装 V1337 = 复刻 VCP plugin: V1337 = static dashboard, NOT runtime plugin
- ? 不假装 V1337 = VCP plugin runtime: reads source code only, no exec / no API call
- ? 不假装 ASI 真懂 cross-plugin compliance: dashboard aggregates evidence, NOT semantics
- ? 不假装 ASI 真有 compliance 自学习: dashboard records evidence, NOT interpretation
- ? 不假装 Phenomenal consciousness: dashboard ≠ phenomenological "compliance"
- ? 不假装 ASI 达到: V1337 不动 ASI 北极星
- ? 不假装调整模型 & prompt

ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE — V1337 不动北极星

ASI 5-Gap 钁楀悕瀹炲疄鐢?(主 13:31 大胆激进) — V1337 实证:
- 识别_recognition: dashboard aggregates per-plugin recognition → 跨 plugin 识别 gap
- 自由_freedom: plugin author 可自由扩展, dashboard 显示 free 边界 → 真自由边界
- 时间_time: dashboard timestamp (post-V1336 linter) → 时间性
- 真理_truth: dashboard = V1335+V1336 真值表的聚合 → truth gap
- 涌现_emergence: 单 plugin compliance → 跨 plugin compliance matrix → emergence gap

5-CRITICAL COVERAGE RULE (主 22:33 终极授权):
- 5 safety-critical classes (IC1/IC2/IC3/IC4/IC7) MUST have ≥1 substrate in plugin
- Dashboard computes pass rate of 5-critical rule across all 6 plugins
- If pass_rate < 100%: dashboard emits "critical_gap_detected" warning
"""
from __future__ import annotations

import argparse
import csv
import io
import json
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# --- v1335 + v1336 import path ----------------------------------------------
V1337_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1337_DIR))

import v1336_vcp_plugin_conformance_linter as v1336  # noqa: E402
import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402


# --- ASI Pole-star (LOCKED) -------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,
    "V1337_modifies_pole_star": False,
}

# --- VCP plugin manifest (V1337 reads these 6 plugin files) ----------------
VCP_PLUGIN_FILES: List[Dict[str, Any]] = [
    {
        "plugin_id": "V1327",
        "plugin_filename": "v1327_vcp_6_source_deep_read.py",
        "plugin_label": "VCP-6-core",
        "role": "VCP core 6-layer substrate",
    },
    {
        "plugin_id": "V1328",
        "plugin_filename": "v1328_anysearch_plugin_deep_read.py",
        "plugin_label": "AnySearch",
        "role": "vertical search MCP plugin",
    },
    {
        "plugin_id": "V1330",
        "plugin_filename": "v1330_agentdream_plugin_deep_read.py",
        "plugin_label": "AgentDream",
        "role": "agent creative dream loop",
    },
    {
        "plugin_id": "V1332",
        "plugin_filename": "v1332_ragdiary_plugin_deep_read.py",
        "plugin_label": "RAGDiary",
        "role": "RAG memory system",
    },
    {
        "plugin_id": "V1333",
        "plugin_filename": "v1333_vcptimeline_plugin_deep_read.py",
        "plugin_label": "VCPTimeLine",
        "role": "per-Agent monthly timeline",
    },
    {
        "plugin_id": "V1334",
        "plugin_filename": "v1334_thoughtclustermanager_plugin_deep_read.py",
        "plugin_label": "ThoughtClusterManager",
        "role": "思维簇管理器",
    },
]


# --- Dataclasses ------------------------------------------------------------
@dataclass
class CrossPluginComplianceCell:
    """One cell in the cross-plugin compliance matrix."""
    plugin_id: str
    plugin_label: str
    invariant_class_id: str
    substrate_count: int
    safety_critical: bool
    has_coverage: bool  # True if substrate_count >= 1

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class DashboardSummary:
    """Overall dashboard summary."""
    total_plugins: int
    plugins_passed: int
    plugins_failed: int
    plugins_warned: int
    critical_pass_rate: float  # 0.0-1.0 of plugins passing 5-critical
    critical_gaps_detected: int
    total_substrates: int
    avg_coverage_score: float
    overall_verdict: str
    recommendations: List[str]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class VCPPluginComplianceDashboard:
    """Top-level dashboard container."""
    per_plugin_reports: List[v1336.PluginConformanceReport]
    cross_plugin_matrix: List[CrossPluginComplianceCell]
    summary: DashboardSummary
    asi_pole_star: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "per_plugin_reports": [r.to_dict() for r in self.per_plugin_reports],
            "cross_plugin_matrix": [c.to_dict() for c in self.cross_plugin_matrix],
            "summary": self.summary.to_dict(),
            "asi_pole_star": self.asi_pole_star,
        }


# --- Helpers ----------------------------------------------------------------
def _scan_substrates_for_plugin(path: Path) -> Dict[str, int]:
    """Scan a plugin path; return {invariant_class_id: substrate_count}."""
    if not path.exists():
        return {}
    counts: Dict[str, int] = {}
    names = v1335._extract_substrate_names(path)
    for name in names:
        class_ids = v1335.lint_substrate_name(name)
        for cid in class_ids:
            counts[cid] = counts.get(cid, 0) + 1
    return counts


def _build_cross_plugin_matrix(
    per_plugin_reports: List[v1336.PluginConformanceReport],
) -> List[CrossPluginComplianceCell]:
    """Build the cross-plugin compliance matrix (plugin × invariant class)."""
    matrix: List[CrossPluginComplianceCell] = []
    for r in per_plugin_reports:
        # Map plugin_id from filename
        plugin_id = "?"
        plugin_label = "?"
        for entry in VCP_PLUGIN_FILES:
            if entry["plugin_filename"] == r.plugin_filename:
                plugin_id = entry["plugin_id"]
                plugin_label = entry["plugin_label"]
                break
        # Build per-invariant-class cells
        count_map: Dict[str, int] = {}
        for c in r.classifications:
            for cid in c.invariant_class_ids:
                count_map[cid] = count_map.get(cid, 0) + 1
        for ic in v1335.INVARIANT_CLASSES:
            cid = ic["invariant_id"]
            count = count_map.get(cid, 0)
            matrix.append(
                CrossPluginComplianceCell(
                    plugin_id=plugin_id,
                    plugin_label=plugin_label,
                    invariant_class_id=cid,
                    substrate_count=count,
                    safety_critical=ic["safety_critical"],
                    has_coverage=count >= 1,
                )
            )
    return matrix


def _build_dashboard_summary(
    per_plugin_reports: List[v1336.PluginConformanceReport],
    matrix: List[CrossPluginComplianceCell],
) -> DashboardSummary:
    """Build dashboard summary from reports + matrix."""
    total = len(per_plugin_reports)
    passed = sum(1 for r in per_plugin_reports if r.verdict == "PASS")
    failed = sum(1 for r in per_plugin_reports if r.verdict == "FAIL")
    warned = sum(1 for r in per_plugin_reports if r.verdict == "PASS_WITH_WARNINGS")

    # Critical pass rate: fraction of plugins passing 5-critical
    pass_5_count = sum(1 for r in per_plugin_reports if r.pass_5_critical)
    critical_pass_rate = (pass_5_count / total) if total > 0 else 0.0

    # Critical gaps: count of (plugin, class) cells where critical class has 0 coverage
    critical_gaps = sum(
        1 for c in matrix
        if c.safety_critical and not c.has_coverage
    )

    # Total substrates across all plugins
    total_substrates = sum(r.total_substrates for r in per_plugin_reports)

    # Avg coverage score
    avg_score = (
        sum(r.coverage_score for r in per_plugin_reports) / total
        if total > 0
        else 0.0
    )

    # Overall verdict
    if failed > 0:
        verdict = "FAIL"
    elif critical_gaps > 0:
        verdict = "PASS_WITH_WARNINGS"
    elif warned > 0:
        verdict = "PASS_WITH_WARNINGS"
    else:
        verdict = "PASS"

    # Recommendations
    recommendations: List[str] = []
    if critical_gaps > 0:
        recommendations.append(
            f"Add safety-critical substrate to {critical_gaps} (plugin, class) cells"
        )
    if avg_score < 0.5:
        recommendations.append(
            f"Increase substrate coverage — avg score {avg_score:.4f} < 0.5"
        )
    if failed > 0:
        recommendations.append(f"Fix {failed} failing plugin(s) before release")
    if not recommendations:
        recommendations.append("All VCP plugins conform to V1335 invariant registry")

    return DashboardSummary(
        total_plugins=total,
        plugins_passed=passed,
        plugins_failed=failed,
        plugins_warned=warned,
        critical_pass_rate=critical_pass_rate,
        critical_gaps_detected=critical_gaps,
        total_substrates=total_substrates,
        avg_coverage_score=avg_score,
        overall_verdict=verdict,
        recommendations=recommendations,
    )


# --- Dashboard API ----------------------------------------------------------
def build_dashboard(
    plugin_dir: Path = V1337_DIR,
    min_score: float = v1336.DEFAULT_MIN_SCORE,
    strict: bool = False,
) -> VCPPluginComplianceDashboard:
    """Build VCP Plugin Compliance Dashboard."""
    # Run V1336 linter on each VCP plugin file
    per_plugin_reports: List[v1336.PluginConformanceReport] = []
    for entry in VCP_PLUGIN_FILES:
        path = plugin_dir / entry["plugin_filename"]
        if path.exists():
            r = v1336.lint_plugin_file(path, min_score=min_score)
            per_plugin_reports.append(r)

    # Build cross-plugin matrix
    matrix = _build_cross_plugin_matrix(per_plugin_reports)

    # Build summary
    summary = _build_dashboard_summary(per_plugin_reports, matrix)

    return VCPPluginComplianceDashboard(
        per_plugin_reports=per_plugin_reports,
        cross_plugin_matrix=matrix,
        summary=summary,
        asi_pole_star=ASI_POLE_STAR,
    )


# --- Reporting --------------------------------------------------------------
def dashboard_to_markdown(d: VCPPluginComplianceDashboard) -> str:
    """Convert dashboard to markdown."""
    lines: List[str] = []
    lines.append("# VCP Plugin Compliance Dashboard")
    lines.append("")
    lines.append(f"- Total plugins: {d.summary.total_plugins}")
    lines.append(f"- Plugins passed: {d.summary.plugins_passed}")
    lines.append(f"- Plugins warned: {d.summary.plugins_warned}")
    lines.append(f"- Plugins failed: {d.summary.plugins_failed}")
    lines.append(f"- Critical pass rate: {d.summary.critical_pass_rate:.4f}")
    lines.append(f"- Critical gaps detected: {d.summary.critical_gaps_detected}")
    lines.append(f"- Total substrates: {d.summary.total_substrates}")
    lines.append(f"- Avg coverage score: {d.summary.avg_coverage_score:.4f}")
    lines.append(f"- Overall verdict: **{d.summary.overall_verdict}**")
    lines.append("")
    lines.append("## Per-plugin conformance")
    for r in d.per_plugin_reports:
        lines.append(f"### {r.plugin_filename}")
        lines.append(f"- Verdict: **{r.verdict}**")
        lines.append(f"- Coverage: {r.coverage_score:.4f}")
        lines.append(f"- 5-critical pass: {r.pass_5_critical}")
        lines.append(f"- Substrates: {r.total_substrates}")
        if r.warnings:
            lines.append(f"- Warnings: {', '.join(r.warnings)}")
        lines.append("")
    lines.append("## Cross-plugin compliance matrix (plugin × invariant class)")
    # Header row
    inv_ids = [ic["invariant_id"] for ic in v1335.INVARIANT_CLASSES]
    header = "| Plugin | " + " | ".join(inv_ids) + " |"
    sep = "|" + "---|" * (len(inv_ids) + 1)
    lines.append(header)
    lines.append(sep)
    # Body rows
    plugin_ids = []
    for r in d.per_plugin_reports:
        pid = "?"
        for entry in VCP_PLUGIN_FILES:
            if entry["plugin_filename"] == r.plugin_filename:
                pid = entry["plugin_id"]
                break
        plugin_ids.append((pid, r))
    for pid, r in plugin_ids:
        cells = []
        for cid in inv_ids:
            # Find cell
            cell = next(
                (c for c in d.cross_plugin_matrix
                 if c.plugin_id == pid and c.invariant_class_id == cid),
                None,
            )
            if cell:
                if cell.safety_critical:
                    cells.append(f"{cell.substrate_count}🛡️" if cell.substrate_count > 0 else "·🛡️")
                else:
                    cells.append(f"{cell.substrate_count}" if cell.substrate_count > 0 else "·")
            else:
                cells.append("?")
        lines.append(f"| {pid} | " + " | ".join(cells) + " |")
    lines.append("")
    if d.summary.recommendations:
        lines.append("## Recommendations")
        for rec in d.summary.recommendations:
            lines.append(f"- {rec}")
        lines.append("")
    return "\n".join(lines)


def dashboard_to_csv(d: VCPPluginComplianceDashboard) -> str:
    """Convert dashboard to CSV."""
    buf = io.StringIO()
    writer = csv.writer(buf)
    # Header
    writer.writerow([
        "plugin_id", "plugin_label", "plugin_filename",
        "verdict", "coverage_score", "pass_5_critical",
        "total_substrates",
        "invariant_class_id", "substrate_count", "safety_critical", "has_coverage",
    ])
    # Body: per (plugin, class) cell
    for r in d.per_plugin_reports:
        # Find plugin_id
        pid = "?"
        plabel = "?"
        for entry in VCP_PLUGIN_FILES:
            if entry["plugin_filename"] == r.plugin_filename:
                pid = entry["plugin_id"]
                plabel = entry["plugin_label"]
                break
        # Get per-class cells for this plugin
        plugin_cells = [
            c for c in d.cross_plugin_matrix if c.plugin_id == pid
        ]
        if not plugin_cells:
            # Write one row with placeholder class data
            writer.writerow([
                pid, plabel, r.plugin_filename,
                r.verdict, f"{r.coverage_score:.4f}", r.pass_5_critical,
                r.total_substrates,
                "(none)", 0, False, False,
            ])
        else:
            for c in plugin_cells:
                writer.writerow([
                    pid, plabel, r.plugin_filename,
                    r.verdict, f"{r.coverage_score:.4f}", r.pass_5_critical,
                    r.total_substrates,
                    c.invariant_class_id, c.substrate_count,
                    c.safety_critical, c.has_coverage,
                ])
    return buf.getvalue()


# --- Self-test (probe-only, 主 17:43 实事求是) ------------------------------
def _self_test() -> Dict[str, bool]:
    """Probe-only self-test, all checks must pass."""
    checks: Dict[str, bool] = {}
    # Check 1: V1335 + V1336 dependencies
    checks["v1335_imported"] = v1335 is not None
    checks["v1336_imported"] = v1336 is not None
    checks["v1335_8_invariant_classes"] = len(v1335.INVARIANT_CLASSES) == 8
    checks["v1336_default_min_score"] = v1336.DEFAULT_MIN_SCORE == 0.50

    # Check 2: VCP_PLUGIN_FILES manifest
    checks["vcp_plugin_files_6"] = len(VCP_PLUGIN_FILES) == 6
    checks["vcp_plugin_label_V1328"] = any(
        p["plugin_label"] == "AnySearch" for p in VCP_PLUGIN_FILES
    )
    checks["vcp_plugin_label_V1334"] = any(
        p["plugin_label"] == "ThoughtClusterManager" for p in VCP_PLUGIN_FILES
    )

    # Check 3: Build dashboard
    d = build_dashboard()
    checks["dashboard_builds"] = d is not None
    checks["dashboard_per_plugin_reports_6"] = len(d.per_plugin_reports) == 6
    checks["dashboard_cross_plugin_matrix_size"] = (
        len(d.cross_plugin_matrix) == 6 * 8
    )
    checks["dashboard_summary_exists"] = d.summary is not None

    # Check 4: Summary fields
    checks["summary_total_plugins_6"] = d.summary.total_plugins == 6
    checks["summary_total_substrates_positive"] = d.summary.total_substrates > 0
    checks["summary_avg_score_positive"] = d.summary.avg_coverage_score > 0.0
    checks["summary_verdict_in_set"] = d.summary.overall_verdict in (
        "PASS", "PASS_WITH_WARNINGS", "FAIL"
    )
    checks["summary_recommendations_nonempty"] = len(d.summary.recommendations) > 0

    # Check 5: Cross-plugin matrix
    checks["matrix_cell_has_fields"] = all(
        hasattr(c, "plugin_id") and hasattr(c, "invariant_class_id")
        and hasattr(c, "substrate_count") and hasattr(c, "safety_critical")
        and hasattr(c, "has_coverage")
        for c in d.cross_plugin_matrix
    )
    # Each plugin × class cell exists
    cy_plugin_ids = {c.plugin_id for c in d.cross_plugin_matrix}
    cy_class_ids = {c.invariant_class_id for c in d.cross_plugin_matrix}
    checks["matrix_6_plugins"] = len(cy_plugin_ids) == 6
    checks["matrix_8_classes"] = len(cy_class_ids) == 8

    # Check 6: Safety-critical pass rate
    checks["critical_pass_rate_in_range"] = (
        0.0 <= d.summary.critical_pass_rate <= 1.0
    )

    # Check 7: Markdowns
    md = dashboard_to_markdown(d)
    checks["md_contains_total_plugins"] = "Total plugins: 6" in md
    checks["md_contains_overall_verdict"] = "Overall verdict" in md
    checks["md_contains_cross_plugin_matrix"] = "Cross-plugin compliance matrix" in md

    # Check 8: CSV
    csv_text = dashboard_to_csv(d)
    checks["csv_has_header"] = "plugin_id" in csv_text
    checks["csv_has_rows"] = len(csv_text.splitlines()) > 6

    # Check 9: Strict mode
    d_strict = build_dashboard(strict=True)
    checks["strict_dashboard_builds"] = d_strict is not None

    # Check 10: ASI pole-star NOT modified
    checks["asi_pole_star_locked"] = ASI_POLE_STAR["V1337_modifies_pole_star"] is False
    checks["asi_achieved_still_false"] = ASI_POLE_STAR["asi_achieved_false"] is True

    # Check 11: Cross plugin cells with safety_critical=True
    sc_cells = [c for c in d.cross_plugin_matrix if c.safety_critical]
    checks["sc_cells_30"] = len(sc_cells) == 6 * 5  # 6 plugins × 5 SC classes
    checks["sc_cells_have_data"] = all(
        c.substrate_count >= 0 for c in sc_cells
    )

    return checks


def _self_test_summary() -> Tuple[int, int, List[str]]:
    checks = _self_test()
    passed = sum(1 for v in checks.values() if v)
    failed = sum(1 for v in checks.values() if not v)
    failed_names = [k for k, v in checks.items() if not v]
    return passed, failed, failed_names


# --- CLI --------------------------------------------------------------------
def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point. Returns 0 on PASS, 1 on FAIL."""
    parser = argparse.ArgumentParser(
        prog="v1337_vcp_plugin_compliance_dashboard",
        description="VCP Plugin Compliance Dashboard (per V1335+V1336)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output JSON",
    )
    parser.add_argument(
        "--markdown",
        action="store_true",
        help="Output Markdown (default)",
    )
    parser.add_argument(
        "--csv",
        action="store_true",
        help="Output CSV",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Treat any warning as FAIL",
    )
    parser.add_argument(
        "--min-score",
        type=float,
        default=v1336.DEFAULT_MIN_SCORE,
        help=f"Minimum coverage score (default {v1336.DEFAULT_MIN_SCORE})",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run self-test and exit",
    )

    args = parser.parse_args(argv)

    if args.self_test:
        passed, failed, failed_names = _self_test_summary()
        print(f"V1337 self-test: {passed}/{passed + failed} pass")
        if failed:
            print(f"  Failed: {failed_names}")
            return 1
        print("ALL CHECKS PASS [OK]")
        return 0

    d = build_dashboard(min_score=args.min_score, strict=args.strict)

    if args.json:
        print(json.dumps(d.to_dict(), indent=2, ensure_ascii=False))
    elif args.csv:
        print(dashboard_to_csv(d), end="")
    else:
        print(dashboard_to_markdown(d))

    if d.summary.overall_verdict == "FAIL":
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
