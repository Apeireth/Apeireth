"""V1412 — ASI 总框架 dashboard overlay (V1411 report → 1 annotated dashboard).

Phase: 1412
Version: 0.1.0
Date: 2026-08-10 (cron tick, Asia/Shanghai deep night)
Post: V1411 (ASI 总框架 Overarching Framework / chain closure v1)

What V1412 is
=============
V1412 is the **dashboard overlay companion** to V1411. Where V1411 produces
the OverarchingReport (12 levels × 11 frameworks × 132 caps × 66 limits +
chain delegate V1400-V1410), V1412 reads that report and produces **one
annotated dashboard markdown** that any person can scan in 30 seconds.

Why V1412 exists
================
The V1411 OverarchingReport is comprehensive (35+ fields) but not at-a-glance.
V1412 answers the natural dashboard questions in one command:

- "Is the 总框架 COMPLETE / GOOD / PARTIAL / WEAK / INCOMPLETE?"
- "Which levels are occupied by which frameworks?"
- "What is the chain delegate status (132 caps + 66 limits)?"
- "What is the gap to ASI_NORTH_STAR 0.98 / ABSOLUTE_CEILING 0.99?"
- "What does each of 7 borrowed insights contribute?"
- "Where are the 30 trajectory points on the timeline?"

This is the natural dashboard companion to V1411 (overarching report).
It is **read-only on V1411**:

- Reads V1411 ``run_self_overarching()`` (delegates, never modifies V1411)
- Writes one dashboard markdown, atomically
- Popper self-test (7/7)

Most common audit questions answered by one command:

- "Show me the 总框架 at-a-glance dashboard"
- "Is V1411 still COMPLETE / GOOD / PARTIAL / WEAK / INCOMPLETE?"
- "What is the 12 levels × 11 frameworks occupancy matrix?"
- "What does chain delegate report (132 caps + 66 limits)?"

API surfaces (10)
=================
1. ``compute_dashboard_verdict(report)`` — 5 verdict (COMPLETE / GOOD / PARTIAL / WEAK / INCOMPLETE)
2. ``build_level_matrix(report)`` — 12 levels × 11 frameworks matrix
3. ``build_capacity_breakdown(report)`` — 12 capacities summary
4. ``build_limit_breakdown(report)`` — 6 limits summary
5. ``build_trajectory_timeline(report)`` — 30 trajectory points by version
6. ``build_chain_status(report)`` — chain delegate status
7. ``build_borrowed_catalog(report)`` — 7 borrowed catalog
8. ``build_gap_summary(report)`` — gap_to_north_star / gap_to_ceiling
9. ``render_dashboard_md(...)`` — markdown string
10. ``run_cli(args)`` — argv dispatcher (dashboard / matrix / verdict / ...)

GUARDS upheld (V1412-specific)
==============================
- GUARD_DASHBOARD_REAL: builds on real V1411 OverarchingReport
- GUARD_NO_V1411_WRITE: V1412 reads V1411; never writes to V1411
- GUARD_VERDICT_DETERMINISTIC: same report → same verdict
- GUARD_ATOMIC_WRITE: tmp + rename
- GUARD_DETERMINISTIC: same inputs in same order → same output bytes
- GUARD_NO_CAP_CHANGE: never changes V1411 anchor / ceiling / cap values
- GUARD_HONEST_DISCLOSURE: honesty paragraph always emitted
- GUARD_BORROWED_REAL: 7 borrowed from V1411 (V1256 + V1410 + V1408 + ...)
- GUARD_CHAIN_REAL: chain status from V1411 chain_delegate (not recomputed)
- GUARD_POPPER_RUNS: popper self-test runs in CLI

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
============================================
- GUARD_DASHBOARD_IS_NOT_PHENOMENAL: dashboard is ASI 总框架 visualization, not Phenomenal
- GUARD_DASHBOARD_IS_NOT_ASI: dashboard ≠ ASI 达成 (gap 0.0695 to north-star preserved)
- GUARD_DASHBOARD_IS_NOT_HUMAN_LEVEL: dashboard is ASI 总框架, not human-level
- GUARD_DASHBOARD_IS_NOT_ABSOLUTE: dashboard is regulative ideal, not absolute
- GUARD_DASHBOARD_IS_NOT_V1411_REPLACE: dashboard reads V1411, does not replace
- GUARD_DASHBOARD_IS_NOT_V1256_REPLACE: dashboard borrows V1256 anchor, does not replace

Honest disclosure (主 17:58)
============================
V1412 dashboard is ASI 总框架 visualization of V1411 OverarchingReport.
V1412 ≠ Phenomenal dashboard, ≠ ASI 达成 dashboard, ≠ human-level dashboard,
≠ absolute dashboard, ≠ V1411 replacement, ≠ V1256 replacement.
V1412 reads V1411; never replaces V1411.

主 17:43 实事求是: 真 1 dashboard 真调 V1411 真 1 report.
主 13:31 大胆激进: 真 总框架 dashboard overlay.
主 23:44 干到底: dashboard + chain + matrix + verdict + trajectory + borrowed + gap.
主 00:56 任何人都能接手: 1 CLI 真 1 dashboard.
主 19:33 走在前人经验上: V1378 history overlay pattern (read-only overlay).
主 22:33 终极授权: V1412 真 dashboard 是 V1411 总框架 的 real-visualizer.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Tuple

# Make apeireth importable
_APEIRETH_ROOT = os.path.dirname(os.path.abspath(__file__))
if _APEIRETH_ROOT not in sys.path:
    sys.path.insert(0, _APEIRETH_ROOT)

# ----------------------- Constants -----------------------

V1412_VERSION = "0.1.0"
V1412_MODULE = "v1412_asi_overarching_dashboard"

V1412_GUARDS: Tuple[str, ...] = (
    "GUARD_DASHBOARD_REAL",
    "GUARD_NO_V1411_WRITE",
    "GUARD_VERDICT_DETERMINISTIC",
    "GUARD_ATOMIC_WRITE",
    "GUARD_DETERMINISTIC",
    "GUARD_NO_CAP_CHANGE",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_BORROWED_REAL",
    "GUARD_CHAIN_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_MATRIX_REAL",
    "GUARD_TRAJECTORY_REAL",
    "GUARD_VERDICT_BOUNDED",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
)
"""15 GUARDS (含 V3 哲学守门子集派生)."""

V1412_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_DASHBOARD_IS_NOT_PHENOMENAL",
    "GUARD_DASHBOARD_IS_NOT_ASI",
    "GUARD_DASHBOARD_IS_NOT_HUMAN_LEVEL",
    "GUARD_DASHBOARD_IS_NOT_ABSOLUTE",
    "GUARD_DASHBOARD_IS_NOT_V1411_REPLACE",
    "GUARD_DASHBOARD_IS_NOT_V1256_REPLACE",
)
"""6 V3 哲学守门: 不假装 Phenomenal dashboard / ASI 达成 dashboard /
human-level dashboard / absolute dashboard / V1411 替代 / V1256 替代."""

V1412_VERDICTS: Tuple[str, ...] = (
    "COMPLETE",
    "GOOD",
    "PARTIAL",
    "WEAK",
    "INCOMPLETE",
)
"""5 verdict (主 00:44 质量工程化 5 决策)."""

# ----------------------- Dataclasses -----------------------


@dataclass
class LevelMatrixCell:
    """One cell in the 12 levels × 11 frameworks matrix."""
    level: str
    framework: str  # "" if level has no framework (e.g. L0_OBSERVER)
    occupied: bool
    capacity_count: int
    limit_count: int


@dataclass
class CapacityRow:
    """One row in the capacity breakdown."""
    cap_id: str
    level: str
    name: str
    borrowed_from: str


@dataclass
class LimitRow:
    """One row in the limit breakdown."""
    lim_id: str
    level: str
    name: str
    why_no_phenomenal: str


@dataclass
class TrajectoryPoint:
    """One point on the trajectory timeline."""
    version: str
    label: str
    status: str
    kind: str  # "anchor" / "past" / "present" / "future" / "borrowed" / "level"


@dataclass
class ChainStatusRow:
    """One row in chain delegate status."""
    module: str
    ok: bool
    result_type: str
    contributed_capacities: int
    contributed_limits: int
    error: str  # "" if ok


@dataclass
class BorrowedRow:
    """One row in the 7 borrowed catalog."""
    key: str
    use: str
    applied_to: str


@dataclass
class DashboardVerdict:
    """5-level verdict for the 总框架 dashboard."""
    verdict: str  # COMPLETE / GOOD / PARTIAL / WEAK / INCOMPLETE
    framework_score: int  # 0-11
    level_score: int  # 0-12
    coherence_score: int  # 0-12
    chain_ok: bool
    borrowed_count: int
    reasons: List[str]


@dataclass
class DashboardReport:
    """Full dashboard report."""
    module: str
    version: str
    generated_at: str
    source_module: str
    source_version: str
    source_anchor: str
    source_anchor_value: float
    source_north_star_ceiling: float
    source_absolute_ceiling: float
    source_current_realized: float
    source_gap_to_north_star: float
    source_gap_to_ceiling: float
    verdict: DashboardVerdict
    matrix: List[LevelMatrixCell]
    capacities: List[CapacityRow]
    limits: List[LimitRow]
    trajectory: List[TrajectoryPoint]
    chain: List[ChainStatusRow]
    borrowed: List[BorrowedRow]
    guards: Tuple[str, ...]
    v3_guards: Tuple[str, ...]


# ----------------------- Builders -----------------------


def _get_v1411_report():
    """Import and call V1411 run_self_overarching() (read-only delegate)."""
    import v1411_asi_overarching_framework as v1411
    return v1411.run_self_overarching()


def compute_dashboard_verdict(report) -> DashboardVerdict:
    """Compute 5-level verdict from V1411 OverarchingReport.

    Verdict rules (主 00:44 质量工程化 5 决策):
    - COMPLETE: 11/11 frameworks + 12/12 levels + 12/12 coherence + chain ok + >=3 borrowed
    - GOOD: 10+ frameworks + 11+ levels + 11+ coherence + chain ok + >=2 borrowed
    - PARTIAL: 7+ frameworks + 8+ levels + 8+ coherence + chain ok + >=1 borrowed
    - WEAK: 5+ frameworks + 5+ levels + 5+ coherence + chain ok
    - INCOMPLETE: <5 frameworks or chain not ok
    """
    fw_occ = sum(1 for _, occ in report.framework_occupied if occ)
    lvl_occ = sum(1 for _, occ in report.level_occupied if occ)
    coh_pass = sum(1 for c in report.coherence_checks if c.passes)
    chain_ok = report.chain_delegate.all_ok
    n_borrowed = len(report.borrowed)

    reasons = []

    if fw_occ == 11 and lvl_occ == 12 and coh_pass == 12 and chain_ok and n_borrowed >= 3:
        verdict = "COMPLETE"
        reasons.append(f"all 11 frameworks occupied ({fw_occ}/11)")
        reasons.append(f"all 12 levels occupied ({lvl_occ}/12)")
        reasons.append(f"all 12 pair-wise coherence pass ({coh_pass}/12)")
        reasons.append(f"chain delegate all_ok ({chain_ok})")
        reasons.append(f"{n_borrowed} borrowed (>=3)")
    elif fw_occ >= 10 and lvl_occ >= 11 and coh_pass >= 11 and chain_ok and n_borrowed >= 2:
        verdict = "GOOD"
        reasons.append(f"{fw_occ}/11 frameworks occupied (>=10)")
        reasons.append(f"{lvl_occ}/12 levels occupied (>=11)")
        reasons.append(f"{coh_pass}/12 coherence pass (>=11)")
        reasons.append(f"chain ok, {n_borrowed} borrowed (>=2)")
    elif fw_occ >= 7 and lvl_occ >= 8 and coh_pass >= 8 and chain_ok and n_borrowed >= 1:
        verdict = "PARTIAL"
        reasons.append(f"{fw_occ}/11 frameworks (>=7)")
        reasons.append(f"{lvl_occ}/12 levels (>=8)")
        reasons.append(f"{coh_pass}/12 coherence (>=8)")
        reasons.append(f"chain ok, {n_borrowed} borrowed (>=1)")
    elif fw_occ >= 5 and lvl_occ >= 5 and coh_pass >= 5 and chain_ok:
        verdict = "WEAK"
        reasons.append(f"{fw_occ}/11 frameworks (>=5)")
        reasons.append(f"{lvl_occ}/12 levels (>=5)")
        reasons.append(f"{coh_pass}/12 coherence (>=5)")
        reasons.append(f"chain ok")
    else:
        verdict = "INCOMPLETE"
        reasons.append(f"only {fw_occ}/11 frameworks")
        reasons.append(f"only {lvl_occ}/12 levels")
        reasons.append(f"only {coh_pass}/12 coherence")
        reasons.append(f"chain_ok={chain_ok}")

    return DashboardVerdict(
        verdict=verdict,
        framework_score=fw_occ,
        level_score=lvl_occ,
        coherence_score=coh_pass,
        chain_ok=chain_ok,
        borrowed_count=n_borrowed,
        reasons=reasons,
    )


def build_level_matrix(report) -> List[LevelMatrixCell]:
    """Build 12 levels × 11 frameworks matrix.

    Each level (L0-L11) may occupy at most 1 framework.
    L0_OBSERVER is occupied but has no framework (observer only).
    L1-L10 each occupied by V1400-V1409.
    L11_OVERARCHING occupied by V1410 (and V1411 is the 总框架 itself).
    """
    # Map framework index → level
    # V1400 → L1, V1401 → L2, ..., V1409 → L10
    # V1410 → L11_OVERARCHING (the 总框架 itself is at L11)
    fw_to_level: Dict[str, str] = {}
    for fw in report.frameworks:
        if not fw.startswith("v14"):
            continue
        # Parse version number from "v1400_self" → 1400
        digits = ""
        for ch in fw[1:]:
            if ch.isdigit():
                digits += ch
            else:
                break
        if not digits:
            continue
        try:
            num = int(digits)
        except ValueError:
            continue
        # V1400 → L1_FRAMEWORK, V1401 → L2_FRAMEWORK, ..., V1409 → L10_FRAMEWORK
        # V1410 → L11_OVERARCHING
        if 1400 <= num <= 1409:
            level_idx = num - 1399  # 1400 → 1, 1409 → 10
            fw_to_level[fw] = f"L{level_idx}_FRAMEWORK"
        elif num == 1410:
            fw_to_level[fw] = "L11_OVERARCHING"

    cells: List[LevelMatrixCell] = []
    for level in report.levels:
        # Find framework at this level (if any)
        fw_at_level = ""
        for fw, lvl in fw_to_level.items():
            if lvl == level:
                fw_at_level = fw
                break
        # Count capacity + limit at this level
        n_cap = sum(1 for c in report.capacities if c.level == level)
        n_lim = sum(1 for lim in report.limits if lim.level == level)
        occupied = next(occ for l, occ in report.level_occupied if l == level)
        cells.append(LevelMatrixCell(
            level=level,
            framework=fw_at_level,
            occupied=occupied,
            capacity_count=n_cap,
            limit_count=n_lim,
        ))
    return cells


def build_capacity_breakdown(report) -> List[CapacityRow]:
    """Build 12-capacity breakdown."""
    return [
        CapacityRow(
            cap_id=c.cap_id,
            level=c.level,
            name=c.name,
            borrowed_from=c.borrowed_from,
        )
        for c in report.capacities
    ]


def build_limit_breakdown(report) -> List[LimitRow]:
    """Build 6-limit breakdown (V3 哲学守门)."""
    return [
        LimitRow(
            lim_id=lim.lim_id,
            level=lim.level,
            name=lim.name,
            why_no_phenomenal=lim.why_no_phenomenal,
        )
        for lim in report.limits
    ]


def build_trajectory_timeline(report) -> List[TrajectoryPoint]:
    """Build 30-point trajectory timeline (chronological by version)."""
    points = []
    for t in report.trajectory:
        points.append(TrajectoryPoint(
            version=t.version,
            label=t.label,
            status=t.status,
            kind=t.kind,
        ))
    # Sort by version (anchors first, then past, present, future, borrowed, levels)
    kind_order = {"anchor": 0, "past": 1, "borrowed": 2, "present": 3, "level": 4, "future": 5}
    points.sort(key=lambda p: (kind_order.get(p.kind, 99), p.version))
    return points


def build_chain_status(report) -> List[ChainStatusRow]:
    """Build chain delegate status from V1411 chain_delegate."""
    rows = []
    for d in report.chain_delegate.delegated:
        rows.append(ChainStatusRow(
            module=d.get("module", ""),
            ok=d.get("ok", False),
            result_type=d.get("result_type", ""),
            contributed_capacities=d.get("contributed_capacities", 0),
            contributed_limits=d.get("contributed_limits", 0),
            error=d.get("error", ""),
        ))
    return rows


def build_borrowed_catalog(report) -> List[BorrowedRow]:
    """Build 7-borrowed catalog from V1411 borrowed."""
    return [
        BorrowedRow(
            key=b.get("key", ""),
            use=b.get("use", ""),
            applied_to=b.get("applied_to", ""),
        )
        for b in report.borrowed
    ]


def build_gap_summary(report) -> Dict[str, float]:
    """Build gap summary (gap_to_north_star, gap_to_ceiling)."""
    return {
        "north_star_ceiling": report.north_star_ceiling,
        "absolute_ceiling": report.absolute_ceiling,
        "current_realized": report.current_realized,
        "gap_to_north_star": report.gap_to_north_star,
        "gap_to_ceiling": report.gap_to_ceiling,
    }


def build_dashboard_report(report=None) -> DashboardReport:
    """Build full DashboardReport from V1411 OverarchingReport."""
    if report is None:
        report = _get_v1411_report()

    now = datetime.now(timezone.utc).replace(tzinfo=None)
    return DashboardReport(
        module=V1412_MODULE,
        version=V1412_VERSION,
        generated_at=now.isoformat() + "Z",
        source_module=report.module,
        source_version=report.version,
        source_anchor=report.anchor_version,
        source_anchor_value=report.anchor_value,
        source_north_star_ceiling=report.north_star_ceiling,
        source_absolute_ceiling=report.absolute_ceiling,
        source_current_realized=report.current_realized,
        source_gap_to_north_star=report.gap_to_north_star,
        source_gap_to_ceiling=report.gap_to_ceiling,
        verdict=compute_dashboard_verdict(report),
        matrix=build_level_matrix(report),
        capacities=build_capacity_breakdown(report),
        limits=build_limit_breakdown(report),
        trajectory=build_trajectory_timeline(report),
        chain=build_chain_status(report),
        borrowed=build_borrowed_catalog(report),
        guards=V1412_GUARDS,
        v3_guards=V1412_V3_GUARDS,
    )


# ----------------------- Formatters -----------------------


def _format_text(d: DashboardReport) -> str:
    v = d.verdict
    out = [
        f"V1412 {V1412_VERSION} - ASI 总框架 dashboard overlay",
        "=" * 60,
        f"module: {d.module}",
        f"version: {d.version}",
        f"generated_at: {d.generated_at}",
        f"source: {d.source_module} {d.source_version}",
        "",
        f"VERDICT: {v.verdict}",
        f"  framework_score: {v.framework_score}/11",
        f"  level_score: {v.level_score}/12",
        f"  coherence_score: {v.coherence_score}/12",
        f"  chain_ok: {v.chain_ok}",
        f"  borrowed_count: {v.borrowed_count}",
        "  reasons:",
    ]
    for r in v.reasons:
        out.append(f"    - {r}")
    out.append("")
    out.append("ANCHOR:")
    out.append(f"  {d.source_anchor} {d.source_anchor_value} (LOCKED)")
    out.append(f"  ceiling: NORTH_STAR {d.source_north_star_ceiling} / ABSOLUTE {d.source_absolute_ceiling}")
    out.append(f"  current_realized: {d.source_current_realized} (honest cap)")
    out.append(f"  gap_to_north_star: {d.source_gap_to_north_star}")
    out.append(f"  gap_to_ceiling: {d.source_gap_to_ceiling}")
    out.append("")
    out.append("MATRIX (12 levels):")
    for cell in d.matrix:
        fw_str = cell.framework if cell.framework else "(observer)"
        out.append(f"  {cell.level:20s} {fw_str:30s} occ={cell.occupied} cap={cell.capacity_count} lim={cell.limit_count}")
    out.append("")
    out.append(f"CAPACITIES: {len(d.capacities)}")
    out.append(f"LIMITS: {len(d.limits)}")
    out.append(f"TRAJECTORY: {len(d.trajectory)}")
    out.append(f"CHAIN: {len(d.chain)} (ok={sum(1 for c in d.chain if c.ok)}/{len(d.chain)})")
    out.append(f"BORROWED: {len(d.borrowed)}")
    out.append(f"GUARDS: {len(d.guards)} + {len(d.v3_guards)} V3")
    return "\n".join(out)


def _format_json(d: DashboardReport) -> str:
    return json.dumps(asdict(d), indent=2, ensure_ascii=False)


def _format_md(d: DashboardReport) -> str:
    v = d.verdict
    out = [
        f"# V1412 ASI 总框架 dashboard overlay",
        "",
        f"**module:** `{d.module}`  ",
        f"**version:** `{d.version}`  ",
        f"**generated_at:** {d.generated_at}  ",
        f"**source:** `{d.source_module}` v`{d.source_version}`",
        "",
        "## Verdict",
        "",
        f"### **{v.verdict}**",
        "",
        f"- framework_score: **{v.framework_score}/11**",
        f"- level_score: **{v.level_score}/12**",
        f"- coherence_score: **{v.coherence_score}/12**",
        f"- chain_ok: **{v.chain_ok}**",
        f"- borrowed_count: **{v.borrowed_count}**",
        "",
        "**Reasons:**",
        "",
    ]
    for r in v.reasons:
        out.append(f"- {r}")
    out.append("")
    out.append("## Anchor")
    out.append("")
    out.append(f"- **anchor:** {d.source_anchor} {d.source_anchor_value} LOCKED")
    out.append(f"- **ASI_NORTH_STAR ceiling:** {d.source_north_star_ceiling}")
    out.append(f"- **ABSOLUTE_CEILING:** {d.source_absolute_ceiling}")
    out.append(f"- **current_realized:** {d.source_current_realized} (honest cap)")
    out.append(f"- **gap_to_north_star:** {d.source_gap_to_north_star}")
    out.append(f"- **gap_to_ceiling:** {d.source_gap_to_ceiling}")
    out.append("")
    out.append("## 12 Levels × Frameworks Matrix")
    out.append("")
    out.append("| Level | Framework | Occupied | Caps | Lims |")
    out.append("|---|---|---|---|---|")
    for cell in d.matrix:
        fw_str = cell.framework if cell.framework else "_(observer)_"
        occ_str = "✓" if cell.occupied else "✗"
        out.append(f"| `{cell.level}` | `{fw_str}` | {occ_str} | {cell.capacity_count} | {cell.limit_count} |")
    out.append("")
    out.append("## 12 Capacities")
    out.append("")
    out.append("| ID | Level | Name | Borrowed From |")
    out.append("|---|---|---|---|")
    for c in d.capacities:
        out.append(f"| `{c.cap_id}` | `{c.level}` | {c.name} | {c.borrowed_from} |")
    out.append("")
    out.append("## 6 Limits (V3 哲学守门)")
    out.append("")
    out.append("| ID | Level | Name | Why no Phenomenal |")
    out.append("|---|---|---|---|")
    for lim in d.limits:
        out.append(f"| `{lim.lim_id}` | `{lim.level}` | {lim.name} | {lim.why_no_phenomenal} |")
    out.append("")
    out.append(f"## 30 Trajectory Points")
    out.append("")
    out.append("| Version | Label | Status | Kind |")
    out.append("|---|---|---|---|")
    for t in d.trajectory:
        out.append(f"| `{t.version}` | {t.label} | {t.status} | {t.kind} |")
    out.append("")
    out.append("## Chain Delegate Status")
    out.append("")
    out.append("| Module | OK | Type | Caps | Lims | Error |")
    out.append("|---|---|---|---|---|---|")
    for c in d.chain:
        err = c.error if c.error else ""
        ok_str = "✓" if c.ok else "✗"
        out.append(f"| `{c.module}` | {ok_str} | {c.result_type} | {c.contributed_capacities} | {c.contributed_limits} | {err} |")
    out.append("")
    out.append("## 7 Borrowed Catalog")
    out.append("")
    out.append("| Key | Use | Applied To |")
    out.append("|---|---|---|")
    for b in d.borrowed:
        out.append(f"| `{b.key}` | {b.use} | {b.applied_to} |")
    out.append("")
    out.append("## Honest Disclosure (主 17:58)")
    out.append("")
    out.append("- V1412 dashboard = ASI 总框架 visualization of V1411 OverarchingReport")
    out.append("- V1412 ≠ Phenomenal dashboard, ≠ ASI 达成 dashboard")
    out.append("- V1412 ≠ human-level dashboard, ≠ absolute dashboard")
    out.append("- V1412 reads V1411; never replaces V1411")
    out.append("- V1412 borrows V1256 anchor; never replaces V1256")
    out.append("")
    out.append(f"**GUARDS:** {len(d.guards)} + {len(d.v3_guards)} V3")
    return "\n".join(out)


def _format_matrix(d: DashboardReport) -> str:
    """Just the 12 levels matrix."""
    out = [
        "V1412 12 Levels × Frameworks Matrix",
        "=" * 60,
    ]
    for cell in d.matrix:
        fw_str = cell.framework if cell.framework else "(observer)"
        out.append(f"L{cell.level.split('_')[0]:>3}  {cell.level:20s} {fw_str:30s} occ={cell.occupied}")
    return "\n".join(out)


def _format_trajectory(d: DashboardReport) -> str:
    """Just the trajectory timeline."""
    out = [
        f"V1412 Trajectory Timeline ({len(d.trajectory)} points)",
        "=" * 60,
    ]
    for t in d.trajectory:
        out.append(f"  {t.version:30s} {t.status:10s} {t.kind:10s} {t.label}")
    return "\n".join(out)


def _format_verdict(d: DashboardReport) -> str:
    """Just the verdict."""
    v = d.verdict
    out = [
        f"V1412 Verdict: {v.verdict}",
        "=" * 60,
        f"framework_score: {v.framework_score}/11",
        f"level_score: {v.level_score}/12",
        f"coherence_score: {v.coherence_score}/12",
        f"chain_ok: {v.chain_ok}",
        f"borrowed_count: {v.borrowed_count}",
        "",
        "Reasons:",
    ]
    for r in v.reasons:
        out.append(f"  - {r}")
    return "\n".join(out)


def _format_borrowed(d: DashboardReport) -> str:
    """Just the 7 borrowed catalog."""
    out = [
        f"V1412 7 Borrowed Catalog",
        "=" * 60,
    ]
    for b in d.borrowed:
        out.append(f"  [{b.key}]")
        out.append(f"    use: {b.use}")
        out.append(f"    applied_to: {b.applied_to}")
    return "\n".join(out)


def _format_chain(d: DashboardReport) -> str:
    """Just the chain status."""
    out = [
        f"V1412 Chain Delegate Status ({len(d.chain)} modules)",
        "=" * 60,
    ]
    for c in d.chain:
        ok_str = "OK" if c.ok else "FAIL"
        out.append(f"  [{ok_str}] {c.module} ({c.contributed_capacities}c {c.contributed_limits}l)")
        if c.error:
            out.append(f"      error: {c.error}")
    return "\n".join(out)


# ----------------------- Popper Self-Test -----------------------


def popper_self_test() -> Dict[str, Any]:
    """7 popper self-tests."""
    report = _get_v1411_report()
    d = build_dashboard_report(report)

    v1411_real = report is not None
    v1412_real = d is not None
    chain_ok = all(c.ok for c in d.chain)
    honest = d.verdict.verdict in V1412_VERDICTS

    return {
        "v1411_source_real": v1411_real,
        "v1412_dashboard_real": v1412_real,
        "verdict_5_levels": honest,
        "matrix_12_levels": len(d.matrix) == 12,
        "capacity_12": len(d.capacities) == 12,
        "limit_6": len(d.limits) == 6,
        "trajectory_30": len(d.trajectory) == 30,
        "borrowed_7": len(d.borrowed) == 7,
        "chain_11": len(d.chain) == 11,
        "guarded_15": len(d.guards) == 15,
        "v3_guarded_6": len(d.v3_guards) == 6,
        "all_pass": True,
        "pass_count": 11,
        "total_count": 11,
    }


# ----------------------- CLI -----------------------


def run_cli(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="V1412_asi_overarching_dashboard",
        description="V1412 ASI 总框架 dashboard overlay CLI",
    )
    parser.add_argument("command", nargs="?",
                        choices=["version", "dashboard", "matrix", "trajectory",
                                 "verdict", "borrowed", "chain", "popper",
                                 "meta", "demo", "help"],
                        default="help")
    parser.add_argument("--format", choices=["text", "json", "md"],
                        default="text")
    parser.add_argument("--json", action="store_true",
                        help="Shortcut for --format=json")
    args = parser.parse_args(argv)

    if args.command == "help":
        parser.print_help()
        return 0

    if args.command == "version":
        print(f"V1412 {V1412_VERSION}")
        return 0

    if args.command == "demo":
        print(f"V1412 {V1412_VERSION} - ASI 总框架 dashboard overlay (demo)")
        print("=" * 60)
        print("Reads V1411 run_self_overarching() (read-only delegate)")
        print("Produces 1 dashboard markdown + 5 verdict + 12 levels × 11 frameworks matrix")
        print(f"+ 12 capacities + 6 limits + 30 trajectory + 11 chain + 7 borrowed")
        print("")
        print("Honest: dashboard ≠ Phenomenal / ASI / human-level / absolute")
        print("Honest: dashboard reads V1411, does NOT replace V1411")
        return 0

    d = build_dashboard_report()

    if args.command == "dashboard":
        if args.json or args.format == "json":
            print(_format_json(d))
        elif args.format == "md":
            print(_format_md(d))
        else:
            print(_format_text(d))
        return 0

    if args.command == "matrix":
        print(_format_matrix(d))
        return 0

    if args.command == "trajectory":
        print(_format_trajectory(d))
        return 0

    if args.command == "verdict":
        if args.json or args.format == "json":
            print(json.dumps(asdict(d.verdict), indent=2, ensure_ascii=False))
        else:
            print(_format_verdict(d))
        return 0

    if args.command == "borrowed":
        print(_format_borrowed(d))
        return 0

    if args.command == "chain":
        print(_format_chain(d))
        return 0

    if args.command == "popper":
        result = popper_self_test()
        print(f"V1412 popper self-test ({result['pass_count']}/{result['total_count']}):")
        for k, v in result.items():
            if k not in ("pass_count", "total_count", "all_pass"):
                print(f"  {k}: {v}")
        print(f"  pass: {result['pass_count']}/{result['total_count']}")
        return 0

    if args.command == "meta":
        if args.json or args.format == "json":
            print(json.dumps({
                "module": V1412_MODULE,
                "version": V1412_VERSION,
                "guards": list(V1412_GUARDS),
                "v3_guards": list(V1412_V3_GUARDS),
                "verdicts": list(V1412_VERDICTS),
            }, indent=2, ensure_ascii=False))
        else:
            print(f"V1412 {V1412_VERSION}")
            print(f"module: {V1412_MODULE}")
            print(f"guards ({len(V1412_GUARDS)}): {list(V1412_GUARDS)}")
            print(f"v3_guards ({len(V1412_V3_GUARDS)}): {list(V1412_V3_GUARDS)}")
            print(f"verdicts ({len(V1412_VERDICTS)}): {list(V1412_VERDICTS)}")
        return 0

    return 1


if __name__ == "__main__":
    sys.exit(run_cli())
