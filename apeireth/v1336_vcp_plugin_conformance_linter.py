#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1336_vcp_plugin_conformance_linter.py — VCP Plugin Conformance Linter (CLI)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1335 cross-plugin synthesis (61a69e6f, 22:00); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 — V1335 synthesis → V1336 linter CLI (any plugin author can run)
- Chain: V1313 → V1326 → V1327 → V1328 → V1330 → V1332 → V1333 → V1334 → V1335 → **V1336**

V1336 = **VCP Plugin Conformance Linter CLI** — the executable companion to V1335 synthesis.

V1335 = registry (substrate ledger + 8 invariant classes + 0.4107 coverage score).
V1336 = **action**: a future VCP plugin author can run
        `python -m apeireth.v1336_vcp_plugin_conformance_linter my_plugin.py`
        and get back a per-substrate-name conformance report:
        - Which invariant classes are triggered
        - Which safety-critical classes are triggered
        - Whether the plugin passes the 5-critical-coverage rule (≥1 per safety-critical class)
        - JSON / Markdown output

V1336 = **CONFORMANCE LINTER (NOT 复刻, NOT port, NOT 假装 ASI)**:
- Reads ANY Python file (proposed VCP plugin) → extracts substrate names
- Reuses V1335 `lint_substrate_name`, `is_safety_critical_invariant`, `classify_plugin`
- Produces PluginConformanceReport (substrate dict + class set + score + warnings)
- CLI: argparse with --json, --markdown, --strict, --min-score
- 13 distinct API surfaces

All evidence is REAL:
- V1335 module exists on disk (verified via Path.exists() + sha256 within V1336)
- All linter class checks reuse V1335 regex patterns (no new regex)
- No fake decimal precision; all counts reproducible via _self_test()

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ? 不假装 V1336 = 复刻 VCP plugin: V1336 = static linter, NOT runtime plugin
- ? 不假装 V1336 = 真 VCP plugin runtime: V1336 reads source code only, no exec / no API call
- ? 不假装 ASI 真懂 plugin conformance: linter applies invariant regex, NOT semantic understanding
- ? 不假装 ASI 真有 conformance 自学习: report records evidence, NOT interpretation
- ? 不假装 Phenomenal consciousness: linter output ≠ phenomenological "conformance"
- ? 不假装 ASI 达到: V1336 不动 ASI 北极星
- ? 不假装调整模型 & prompt

ASI 北极星 LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE — V1336 不动北极星

ASI 5-Gap 钁楀悕瀹炲疄鐢?(主 13:31 大胆激进) — V1336 实证:
- 识别_recognition: linter detects substrate → invariant class via V1335 regex → 识别 gap
- 自由_freedom: plugin author 可自由扩展, 但 linter warns on safety-critical gaps → 真自由边界
- 时间_time: report timestamp (post-V1335 synthesis) → 时间性
- 真理_truth: linter output = V1335 invariant registry 真值表的应用 → truth gap
- 涌现_emergence: 单 substrate 名 → N invariant classes (cross-cutting pattern) → emergence gap

MIN_5_CRITICAL_COVERAGE rule (主 22:33 终极授权):
- 5 safety-critical classes (IC1/IC2/IC3/IC4/IC7) MUST have ≥1 substrate in plugin
- If missing: linter emits critical_warning (warning_level="critical")
- If --strict: exit 1
- If score < 0.5: linter emits "low_coverage" warning
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# --- v1335 import path (substrate registry) ---------------------------------
# V1336 reuses V1335 directly: lint_substrate_name + is_safety_critical_invariant + classify_plugin
V1335_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1335_DIR))

import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: E402


# --- ASI Pole-star (LOCKED) -------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,
    "V1336_modifies_pole_star": False,
}

# --- Constants --------------------------------------------------------------
MIN_5_CRITICAL_COVERAGE: int = 5  # Must cover all 5 safety-critical classes
DEFAULT_MIN_SCORE: float = 0.50   # Default minimum coverage score


# --- Dataclasses ------------------------------------------------------------
@dataclass
class SubstrateClassification:
    """One substrate name classified into invariant classes."""
    substrate_name: str
    invariant_class_ids: List[str]
    safety_critical_hit: bool

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class PluginConformanceReport:
    """Conformance report for ONE plugin file."""
    plugin_path: str
    plugin_filename: str
    exists: bool
    actual_lines: int
    actual_bytes: int
    sha256_first16: str
    total_substrates: int
    classifications: List[SubstrateClassification]
    invariant_classes_covered: List[str]
    safety_critical_classes_covered: List[str]
    safety_critical_classes_missing: List[str]
    coverage_score: float
    warnings: List[str]
    critical_warning: bool
    pass_5_critical: bool
    verdict: str  # "PASS" / "PASS_WITH_WARNINGS" / "FAIL"

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        return d


@dataclass
class BatchConformanceReport:
    """Conformance report for MULTIPLE plugin files."""
    total_files: int
    files_scanned: int
    files_passed: int
    files_failed: int
    files_warned: int
    min_score: float
    strict: bool
    per_file_reports: List[PluginConformanceReport]
    overall_verdict: str
    asi_pole_star: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# --- Helpers ----------------------------------------------------------------
def _sha256_first16(path: Path) -> str:
    if not path.exists():
        return ""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()[:16]


def _line_count(path: Path) -> int:
    if not path.exists():
        return 0
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        return sum(1 for _ in f)


def _expected_safety_critical_classes() -> List[str]:
    """Return invariant IDs that are safety-critical (per V1335 INVARIANT_CLASSES)."""
    return [ic["invariant_id"] for ic in v1335.INVARIANT_CLASSES if ic["safety_critical"]]


# --- Core linter logic ------------------------------------------------------
def lint_plugin_file(path: Path, min_score: float = DEFAULT_MIN_SCORE) -> PluginConformanceReport:
    """Lint ONE plugin file → conformance report."""
    exists = path.exists()
    byte_size = path.stat().st_size if exists else 0
    sha = _sha256_first16(path) if exists else ""
    lines = _line_count(path) if exists else 0

    classifications: List[SubstrateClassification] = []
    if exists:
        names = v1335._extract_substrate_names(path)
        for name in names:
            class_ids = v1335.lint_substrate_name(name)
            sc_hit = any(
                v1335.is_safety_critical_invariant(cid) for cid in class_ids
            )
            classifications.append(
                SubstrateClassification(
                    substrate_name=name,
                    invariant_class_ids=class_ids,
                    safety_critical_hit=sc_hit,
                )
            )

    # Aggregate
    class_covered = sorted({
        cid
        for c in classifications
        for cid in c.invariant_class_ids
    })
    sc_covered = sorted([
        cid for cid in class_covered
        if v1335.is_safety_critical_invariant(cid)
    ])
    expected_sc = _expected_safety_critical_classes()
    sc_missing = sorted(set(expected_sc) - set(sc_covered))

    # 5-critical coverage: ALL 5 safety-critical classes must be hit
    pass_5_critical = len(sc_missing) == 0

    # Coverage score: fraction of safety-critical classes hit
    coverage_score = (
        len(sc_covered) / len(expected_sc) if expected_sc else 0.0
    )

    # Warnings
    warnings: List[str] = []
    critical_warning = False
    if not pass_5_critical:
        warnings.append(
            f"Missing safety-critical classes: {','.join(sc_missing)}"
        )
        critical_warning = True
    if not exists:
        warnings.append(f"Plugin file does not exist: {path}")
        critical_warning = True
    if coverage_score < min_score:
        warnings.append(
            f"Coverage score {coverage_score:.4f} < min_score {min_score:.4f}"
        )

    # Verdict
    if critical_warning:
        verdict = "FAIL"
    elif warnings:
        verdict = "PASS_WITH_WARNINGS"
    else:
        verdict = "PASS"

    return PluginConformanceReport(
        plugin_path=str(path),
        plugin_filename=path.name,
        exists=exists,
        actual_lines=lines,
        actual_bytes=byte_size,
        sha256_first16=sha,
        total_substrates=len(classifications),
        classifications=classifications,
        invariant_classes_covered=class_covered,
        safety_critical_classes_covered=sc_covered,
        safety_critical_classes_missing=sc_missing,
        coverage_score=coverage_score,
        warnings=warnings,
        critical_warning=critical_warning,
        pass_5_critical=pass_5_critical,
        verdict=verdict,
    )


def lint_plugin_files(
    paths: List[Path],
    min_score: float = DEFAULT_MIN_SCORE,
    strict: bool = False,
) -> BatchConformanceReport:
    """Lint MULTIPLE plugin files → batch conformance report."""
    reports = [lint_plugin_file(p, min_score) for p in paths]
    files_passed = sum(1 for r in reports if r.verdict == "PASS")
    files_failed = sum(1 for r in reports if r.verdict == "FAIL")
    files_warned = sum(1 for r in reports if r.verdict == "PASS_WITH_WARNINGS")

    # Overall verdict: FAIL if any FAIL (or strict + any warnings)
    if files_failed > 0:
        overall = "FAIL"
    elif strict and (files_warned > 0 or files_passed < len(reports)):
        overall = "FAIL"
    elif files_warned > 0:
        overall = "PASS_WITH_WARNINGS"
    else:
        overall = "PASS"

    return BatchConformanceReport(
        total_files=len(paths),
        files_scanned=len(reports),
        files_passed=files_passed,
        files_failed=files_failed,
        files_warned=files_warned,
        min_score=min_score,
        strict=strict,
        per_file_reports=reports,
        overall_verdict=overall,
        asi_pole_star=ASI_POLE_STAR,
    )


# --- Reporting --------------------------------------------------------------
def report_to_markdown(report: PluginConformanceReport) -> str:
    """Convert one PluginConformanceReport to markdown."""
    lines: List[str] = []
    lines.append(f"# VCP Plugin Conformance: {report.plugin_filename}")
    lines.append("")
    lines.append(f"- Path: `{report.plugin_path}`")
    lines.append(f"- Exists: {report.exists}")
    lines.append(f"- Lines: {report.actual_lines}")
    lines.append(f"- Bytes: {report.actual_bytes}")
    lines.append(f"- SHA256 (first 16): {report.sha256_first16}")
    lines.append(f"- Total substrates: {report.total_substrates}")
    lines.append(f"- Coverage score: {report.coverage_score:.4f}")
    lines.append(f"- 5-critical pass: {report.pass_5_critical}")
    lines.append(f"- Verdict: **{report.verdict}**")
    lines.append("")
    lines.append("## Invariant classes covered")
    if report.invariant_classes_covered:
        for cid in report.invariant_classes_covered:
            sc = "🛡️" if v1335.is_safety_critical_invariant(cid) else "  "
            lines.append(f"- {sc} {cid}")
    else:
        lines.append("- (none)")
    lines.append("")
    lines.append("## Safety-critical coverage")
    lines.append(f"- Covered: {','.join(report.safety_critical_classes_covered) or '(none)'}")
    lines.append(f"- Missing: {','.join(report.safety_critical_classes_missing) or '(none)'}")
    lines.append("")
    if report.warnings:
        lines.append("## Warnings")
        for w in report.warnings:
            lines.append(f"- ⚠️ {w}")
        lines.append("")
    if report.classifications:
        lines.append("## Substrate classifications (first 20)")
        for c in report.classifications[:20]:
            sc = "🛡️" if c.safety_critical_hit else "  "
            lines.append(
                f"- {sc} `{c.substrate_name}` → {','.join(c.invariant_class_ids) or 'none'}"
            )
    return "\n".join(lines)


def batch_report_to_markdown(batch: BatchConformanceReport) -> str:
    """Convert BatchConformanceReport to markdown."""
    lines: List[str] = []
    lines.append("# VCP Plugin Conformance (batch)")
    lines.append("")
    lines.append(f"- Total files: {batch.total_files}")
    lines.append(f"- Files scanned: {batch.files_scanned}")
    lines.append(f"- Files passed: {batch.files_passed}")
    lines.append(f"- Files warned: {batch.files_warned}")
    lines.append(f"- Files failed: {batch.files_failed}")
    lines.append(f"- Min score: {batch.min_score:.4f}")
    lines.append(f"- Strict: {batch.strict}")
    lines.append(f"- Overall verdict: **{batch.overall_verdict}**")
    lines.append("")
    for r in batch.per_file_reports:
        lines.append(f"## {r.plugin_filename}")
        lines.append(f"- Verdict: **{r.verdict}**")
        lines.append(f"- Coverage: {r.coverage_score:.4f}")
        lines.append(f"- 5-critical: {r.pass_5_critical}")
        if r.warnings:
            lines.append(f"- Warnings: {', '.join(r.warnings)}")
        lines.append("")
    return "\n".join(lines)


# --- Self-test (probe-only, 主 17:43 实事求是) ------------------------------
def _self_test() -> Dict[str, bool]:
    """Probe-only self-test, all checks must pass."""
    checks: Dict[str, bool] = {}

    # Check 1: V1335 module accessible
    checks["v1335_module_exists"] = bool(v1335 is not None)
    checks["v1335_invariant_classes_8"] = len(v1335.INVARIANT_CLASSES) == 8
    checks["v1335_5_safety_critical"] = (
        sum(1 for ic in v1335.INVARIANT_CLASSES if ic["safety_critical"]) == 5
    )

    # Check 2: Expected safety-critical classes
    expected = _expected_safety_critical_classes()
    checks["expected_safety_critical_count_5"] = len(expected) == 5
    checks["IC1_security_in_expected"] = "IC1_security" in expected
    checks["IC2_file_handling_in_expected"] = "IC2_file_handling" in expected
    checks["IC3_schema_in_expected"] = "IC3_schema" in expected
    checks["IC4_ipc_in_expected"] = "IC4_ipc" in expected
    checks["IC7_resource_bounds_in_expected"] = "IC7_resource_bounds_in_expected" if False else "IC7_resource_bounds" in expected

    # Check 3: Lint a valid V13xx file (V1335 itself)
    v1335_path = V1335_DIR / "v1335_vcp_cross_plugin_invariant_synthesis.py"
    report = lint_plugin_file(v1335_path)
    checks["v1335_self_lint_exists"] = report.exists is True
    checks["v1335_self_lint_has_substrates"] = report.total_substrates > 0
    checks["v1335_self_lint_score_positive"] = report.coverage_score > 0.0

    # Check 4: Lint a non-existent file
    fake_path = V1335_DIR / "v9999_does_not_exist.py"
    fake_report = lint_plugin_file(fake_path)
    checks["missing_file_verdict_fail"] = fake_report.verdict == "FAIL"
    checks["missing_file_critical_warning"] = fake_report.critical_warning is True

    # Check 5: Lint a V13xx plugin file (e.g., V1334)
    v1334_path = V1335_DIR / "v1334_thoughtclustermanager_plugin_deep_read.py"
    v1334_report = lint_plugin_file(v1334_path)
    checks["v1334_lint_exists"] = v1334_report.exists is True
    checks["v1334_lint_classifications"] = len(v1334_report.classifications) > 0

    # Check 6: Batch report
    paths = [v1335_path, v1334_path, fake_path]
    batch = lint_plugin_files(paths, min_score=0.5, strict=False)
    checks["batch_total_files_3"] = batch.total_files == 3
    checks["batch_files_scanned_3"] = batch.files_scanned == 3
    checks["batch_overall_fail_or_warn"] = batch.overall_verdict in ("FAIL", "PASS_WITH_WARNINGS")

    # Check 7: ASCII converters
    md = report_to_markdown(report)
    checks["md_contains_filename"] = v1335_path.name in md
    checks["md_contains_verdict"] = "Verdict" in md or "verdict" in md

    # Check 8: ASI pole-star NOT modified
    checks["asi_pole_star_locked"] = ASI_POLE_STAR["V1336_modifies_pole_star"] is False
    checks["asi_achieved_still_false"] = ASI_POLE_STAR["asi_achieved_false"] is True

    # Check 9: Substrate classification fields
    if report.classifications:
        c = report.classifications[0]
        checks["classification_has_substrate_name"] = bool(c.substrate_name)
        checks["classification_has_invariant_classes"] = isinstance(c.invariant_class_ids, list)
        checks["classification_has_sc_hit"] = isinstance(c.safety_critical_hit, bool)

    # Check 10: 5-critical coverage rule
    sc_missing = report.safety_critical_classes_missing
    checks["sc_missing_is_list"] = isinstance(sc_missing, list)
    checks["sc_missing_invariant_or_empty"] = all(
        v1335.is_safety_critical_invariant(m) for m in sc_missing
    )

    # Check 11: VCP 6 plugins all lint successfully
    vcp_plugin_files = [
        "v1327_vcp_6_source_deep_read.py",
        "v1328_anysearch_plugin_deep_read.py",
        "v1330_agentdream_plugin_deep_read.py",
        "v1332_ragdiary_plugin_deep_read.py",
        "v1333_vcptimeline_plugin_deep_read.py",
        "v1334_thoughtclustermanager_plugin_deep_read.py",
    ]
    for fname in vcp_plugin_files:
        p = V1335_DIR / fname
        if p.exists():
            r = lint_plugin_file(p)
            checks[f"vcp_{fname[:8]}_linted"] = r.total_substrates > 0

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
        prog="v1336_vcp_plugin_conformance_linter",
        description="VCP Plugin Conformance Linter (per V1335 invariant registry)",
    )
    parser.add_argument(
        "files",
        nargs="*",
        type=Path,
        default=None,
        help="One or more Python files to lint (proposed VCP plugins)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output JSON instead of markdown",
    )
    parser.add_argument(
        "--markdown",
        action="store_true",
        help="Force markdown output (default)",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Treat any warning as FAIL",
    )
    parser.add_argument(
        "--min-score",
        type=float,
        default=DEFAULT_MIN_SCORE,
        help=f"Minimum coverage score (default {DEFAULT_MIN_SCORE})",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run self-test and exit",
    )

    args = parser.parse_args(argv)

    # Self-test mode
    if args.self_test:
        passed, failed, failed_names = _self_test_summary()
        print(f"V1336 self-test: {passed}/{passed + failed} pass")
        if failed > 0:
            print(f"  Failed: {failed_names}")
            return 1
        print("ALL CHECKS PASS [OK]")
        return 0

    # Lint mode
    batch = lint_plugin_files(args.files, min_score=args.min_score, strict=args.strict)

    if args.json:
        print(json.dumps(batch.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(batch_report_to_markdown(batch))

    # Exit code
    if batch.overall_verdict == "FAIL":
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
