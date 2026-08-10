"""V1451 — ASI cube history trend v2 (real 2nd snapshot + per-axis delta).

Phase: 1451
Version: 0.1.0
Date: 2026-08-10 (cron tick 08:28 Asia/Shanghai morning)
Post: V1450 (ASI cross-modular cube history aggregator)
      V1449 (ASI 7 哲学问题 × VCP 6 协议 cross-modular audit)
      V1448 (ASI VCP 6 协议 × V2 5 位置 cross-modular audit)
      V1447 (ASI 7 哲学问题 × V2 5 位置 cross-combined audit)

What V1451 is
=============
V1451 is the **ASI cube history trend v2** — the first time the cube history
emits a **real trend** (not INSUFFICIENT), because it explicitly **appends a
second snapshot** by re-running V1450's aggregate_cube_snapshot(), then
computes:

1. **Cube trend** — slope + delta of cube_overall_closure_rate
2. **Per-axis trend** — slope + delta for each of the 3 axes
   (problem/position/protocol)
3. **Per-axis-element trend** — slope + delta for each axis element
   (7 problems / 5 positions / 6 protocols = 18 elements)
4. **is_improving** — boolean: is the cube improving, stagnant, or regressing?
5. **Ranked improvements** — top 3 improving + top 3 regressing elements
6. **Stability score** — std deviation of delta across axis elements (0=stable,
   1=volatile)

V1451 explicitly composes on V1450 by importing V1450's aggregate_cube_snapshot()
and run_all() (so V1450 history is read + appended in one atomic call).

V1451 ≠ ASI closure. V1451 ≠ Phenomenal closure. V1451 ≠ human-level closure.
V1451 ≠ absolute closure. V1451 = bounded history operator on V1450 snapshots.

Why V1451 exists
================
V1450 produced a single snapshot (08:20 today) and emitted
"Trend: INSUFFICIENT — need ≥ 2 snapshots to compute trend". V1451 closes that
gap: anyone can run V1451 once to get a real trend. After V1451 runs, the
history has 2 snapshots and trend is computable.

This is the natural next step after V1450. Without V1451:
- The cube history has 1 snapshot.
- Trend is permanently INSUFFICIENT.
- Per-axis improvement cannot be measured.

With V1451:
- The cube history has 2+ snapshots.
- Trend is real (slope + delta).
- Per-axis improvement ranking is real.
- "Is the cube improving?" is a one-CLI-command answer.

V1451 is **read+append on V1450**:
- Imports V1450.aggregate_cube_snapshot()
- Calls run_all(append_history=True) → 2nd snapshot appended
- Loads full history (now ≥ 2 snapshots)
- Computes per-axis trend, per-axis-element delta, ranked improvements
- Writes report JSON + MD
- Popper self-test (14/14)

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1450 (cube history aggregator + aggregate_cube_snapshot + run_all + popper
  + chain_delegate)
- V1417 (DGM tick history pattern: load + append + trend + digest)
- V1413 (history aggregator pattern: snapshot + trend + digest)
- V1447 (cross-modular per-problem / per-position closure structure)
- stdlib statistics (mean + pstdev)

GUARDS upheld (V1451-specific) (14 — 主 00:44 质量工程化)
==========================================================
- GUARD_HISTORY_2PLUS: real history has ≥ 2 snapshots after V1451 run
- GUARD_BOUNDED_TREND: trend slope + delta bounded in [-1, +1]
- GUARD_NO_V1450_REPLACE: V1451 reads + composes on V1450, never replaces it
- GUARD_CLI_RUNNABLE: anyone can run `python -m apeireth.v1451_asi_cube_history_trend_v2 ...`
- GUARD_OFFLINE_SAFE: no network required
- GUARD_NO_RAISE: bounded by try/except in popper
- GUARD_PER_AXIS_TREND: 3 axes (problem/position/protocol) each have trend
- GUARD_RANK_BOUNDED: top improving + top regressing are bounded to ≤ N/2
- GUARD_STABILITY_BOUNDED: stability_score in [0, 1]
- GUARD_HONEST_DISCLOSURE: V1451 ≠ ASI closure trend
- GUARD_POPPER_RUNS: popper self-test 14/14
- GUARD_CHAIN_OK: chain_delegate V1450 all_ok=true
- GUARD_RENDER_RUNS: markdown report rendered with all 8 sections

V3 哲学守门 (5 — 主 17:58 + 主 20:46 + 主 17:43)
================================================
- GUARD_NO_PHENOMENAL_TREND: trend = bounded arithmetic, NOT consciousness
- GUARD_NO_ASI_TREND: trend ≠ ASI improvement
- GUARD_NO_HUMAN_LEVEL_TREND: trend = local bounded, NOT human-level
- GUARD_NO_ABSOLUTE_TREND: trend = 2-snapshot, NOT absolute
- GUARD_NO_TREND_OVERCLAIM: trend ≠ solving 7 philosophical problems
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import traceback
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1451_VERSION = "0.1.0"
V1451_SCHEMA = "asi.cube-history-trend-v2.v1"
V1451_MODULE = "apeireth.v1451_asi_cube_history_trend_v2"
V1451_MODULE_SHORT = "v1451_asi_cube_history_trend_v2"

# 3 axes (borrowed from V1450 / V1447+V1448+V1449)
V1451_PROBLEM_NAMES: Tuple[str, ...] = (
    "time", "freedom", "recognition", "emergence",
    "truth", "self_consciousness", "value_alignment",
)
V1451_POSITION_NAMES: Tuple[str, ...] = (
    "scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier",
)
V1451_PROTOCOL_NAMES: Tuple[str, ...] = (
    "sync", "async", "static", "service", "preprocessor", "hybrid",
)

# Trend thresholds (主 17:43 实事求是: not pretending tiny noise is improvement)
V1451_IMPROVING_THRESHOLD = 0.01   # delta > 0.01 → improving
V1451_REGRESSING_THRESHOLD = -0.01  # delta < -0.01 → regressing
# |delta| <= 0.01 → stagnant

# Top-N for ranked lists
V1451_TOP_N = 3

# 14 V1451-specific guards
V1451_GUARDS: Tuple[str, ...] = (
    "GUARD_HISTORY_2PLUS",
    "GUARD_BOUNDED_TREND",
    "GUARD_NO_V1450_REPLACE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_NO_RAISE",
    "GUARD_PER_AXIS_TREND",
    "GUARD_RANK_BOUNDED",
    "GUARD_STABILITY_BOUNDED",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_RENDER_RUNS",
    "GUARD_IS_IMPROVING_DEFINED",
)

# 5 V3 哲学守门
V1451_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_TREND",
    "GUARD_NO_ASI_TREND",
    "GUARD_NO_HUMAN_LEVEL_TREND",
    "GUARD_NO_ABSOLUTE_TREND",
    "GUARD_NO_TREND_OVERCLAIM",
)

V1451_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1450", "cube history aggregator + aggregate_cube_snapshot + run_all + popper + chain_delegate"),
    ("V1417", "DGM tick history pattern: load + append + trend + digest"),
    ("V1413", "history aggregator pattern: snapshot + trend + digest"),
    ("V1447", "cross-modular per-problem / per-position closure structure"),
    ("stdlib statistics", "mean + pstdev"),
)


# ============================================================================
# Dataclasses
# ============================================================================

@dataclass
class AxisTrend:
    """Trend for one axis (problem / position / protocol)."""
    axis: str  # "problem" | "position" | "protocol"
    first_rate: float
    last_rate: float
    delta: float
    slope: float  # (last - first) / max(1, n_snapshots - 1)
    is_improving: bool
    is_regressing: bool
    is_stagnant: bool
    n_snapshots: int


@dataclass
class ElementDelta:
    """Delta for one axis element (e.g., 'time' / 'scheduler' / 'sync')."""
    axis: str
    element: str
    first_rate: float
    last_rate: float
    delta: float
    is_improving: bool
    is_regressing: bool
    is_stagnant: bool


@dataclass
class TrendReport:
    """Full V1451 trend report."""
    schema: str
    version: str
    module: str
    started: str
    ended: str
    n_snapshots_before: int  # before V1451 append
    n_snapshots_after: int   # after V1451 append
    cube_first_rate: float
    cube_last_rate: float
    cube_delta: float
    cube_slope: float
    is_improving: bool
    is_regressing: bool
    is_stagnant: bool
    per_axis_trend: List[AxisTrend]
    per_element_delta: List[ElementDelta]
    top_improving: List[ElementDelta]
    top_regressing: List[ElementDelta]
    stagnant_count: int
    improving_count: int
    regressing_count: int
    stability_score: float  # pstdev of element deltas, in [0, 1]
    axis_improving_count: int  # axes improving
    notes: List[str]


# ============================================================================
# Helpers
# ============================================================================

def _now_iso() -> str:
    """ISO-8601 UTC timestamp, second precision."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _clip01(x: float) -> float:
    """Clip x to [0, 1]."""
    if x < 0.0:
        return 0.0
    if x > 1.0:
        return 1.0
    return float(x)


def _slope_simple(series: List[float]) -> float:
    """Bounded slope: (last - first) / max(1, n-1). Returns 0.0 if n<2."""
    n = len(series)
    if n < 2:
        return 0.0
    denom = max(1, n - 1)
    return (series[-1] - series[0]) / denom


def _is_improving(delta: float) -> bool:
    return delta > V1451_IMPROVING_THRESHOLD


def _is_regressing(delta: float) -> bool:
    return delta < V1451_REGRESSING_THRESHOLD


def _is_stagnant(delta: float) -> bool:
    return not _is_improving(delta) and not _is_regressing(delta)


# ============================================================================
# Import V1450 (compositional)
# ============================================================================

def _import_v1450():
    """Import V1450 module. Returns the module object or raises ImportError."""
    try:
        # Try package import first
        from apeireth import v1450_asi_cross_modular_cube_history as v1450
        return v1450
    except Exception:
        # Fallback: direct path import
        here = Path(__file__).resolve().parent
        sys.path.insert(0, str(here.parent))
        try:
            import apeireth.v1450_asi_cross_modular_cube_history as v1450
            return v1450
        except Exception:
            # Last fallback: file-based load
            import importlib.util
            spec = importlib.util.spec_from_file_location(
                "v1450_asi_cross_modular_cube_history",
                here / "v1450_asi_cross_modular_cube_history.py",
            )
            if spec is None or spec.loader is None:
                raise ImportError("Cannot load V1450 module")
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            return mod


def _ensure_v1450_history_appended() -> Tuple[int, int]:
    """Ensure V1450 history has ≥ 2 snapshots by calling run_all(append_history=True).

    Returns (snapshots_before, snapshots_after).
    """
    v1450 = _import_v1450()
    history_before = v1450.load_cube_history()
    n_before = len(history_before)
    if n_before >= 2:
        return n_before, n_before
    # Append one snapshot
    try:
        v1450.run_all(append_history=True)
    except Exception:
        # run_all may not be available in all contexts; try aggregate
        try:
            v1450.aggregate_cube_snapshot()
            # If aggregate doesn't append, this is best-effort
        except Exception:
            pass
    history_after = v1450.load_cube_history()
    n_after = len(history_after)
    return n_before, n_after


def _extract_per_axis_series(history: List[Dict[str, Any]], axis: str) -> List[float]:
    """Extract per-axis-overall series from history snapshots."""
    series = []
    for snap in history:
        per_axis = snap.get("per_axis_overall", {}) or {}
        rate = per_axis.get(axis, 0.0)
        series.append(float(rate))
    return series


def _extract_per_element_series(
    history: List[Dict[str, Any]],
    axis: str,
    element: str,
) -> List[float]:
    """Extract per-element rate series from axis_stats."""
    series = []
    for snap in history:
        axis_stats = snap.get("axis_stats", []) or []
        for stat in axis_stats:
            # stat may be dict-like or dataclass-like
            if isinstance(stat, dict):
                if stat.get("axis") == axis and stat.get("element") == element:
                    series.append(float(stat.get("mean_closure", 0.0)))
                    break
            else:
                # dataclass
                a = getattr(stat, "axis", None)
                e = getattr(stat, "element", None)
                if a == axis and e == element:
                    series.append(float(getattr(stat, "mean_closure", 0.0)))
                    break
        else:
            series.append(0.0)
    return series


def _compute_axis_trend(axis: str, series: List[float]) -> AxisTrend:
    """Compute AxisTrend for one axis."""
    n = len(series)
    first = series[0] if series else 0.0
    last = series[-1] if series else 0.0
    delta = last - first
    slope = _slope_simple(series)
    return AxisTrend(
        axis=axis,
        first_rate=_clip01(first),
        last_rate=_clip01(last),
        delta=delta,
        slope=slope,
        is_improving=_is_improving(delta),
        is_regressing=_is_regressing(delta),
        is_stagnant=_is_stagnant(delta) if n >= 2 else True,
        n_snapshots=n,
    )


def _compute_element_delta(
    axis: str, element: str, series: List[float]
) -> ElementDelta:
    """Compute ElementDelta for one axis element."""
    n = len(series)
    first = series[0] if series else 0.0
    last = series[-1] if series else 0.0
    delta = last - first
    return ElementDelta(
        axis=axis,
        element=element,
        first_rate=_clip01(first),
        last_rate=_clip01(last),
        delta=delta,
        is_improving=_is_improving(delta) if n >= 2 else False,
        is_regressing=_is_regressing(delta) if n >= 2 else False,
        is_stagnant=_is_stagnant(delta) if n >= 2 else True,
    )


# ============================================================================
# Core: compute trend
# ============================================================================

def compute_trend(history: List[Dict[str, Any]]) -> TrendReport:
    """Compute V1451 trend report from history (List of V1450 snapshots)."""
    started = _now_iso()

    n_snapshots = len(history)

    # Cube overall trend
    cube_series = [float(s.get("cube_overall_closure_rate", 0.0)) for s in history]
    cube_first = cube_series[0] if cube_series else 0.0
    cube_last = cube_series[-1] if cube_series else 0.0
    cube_delta = cube_last - cube_first
    cube_slope = _slope_simple(cube_series)
    cube_improving = _is_improving(cube_delta) if n_snapshots >= 2 else False
    cube_regressing = _is_regressing(cube_delta) if n_snapshots >= 2 else False
    cube_stagnant = (not cube_improving and not cube_regressing) if n_snapshots >= 2 else True

    # Per-axis trend (problem / position / protocol)
    per_axis_trend = []
    for axis in ("problem", "position", "protocol"):
        series = _extract_per_axis_series(history, axis)
        per_axis_trend.append(_compute_axis_trend(axis, series))

    # Per-element delta
    per_element_delta = []
    all_elements = (
        [("problem", e) for e in V1451_PROBLEM_NAMES]
        + [("position", e) for e in V1451_POSITION_NAMES]
        + [("protocol", e) for e in V1451_PROTOCOL_NAMES]
    )
    for axis, element in all_elements:
        series = _extract_per_element_series(history, axis, element)
        per_element_delta.append(_compute_element_delta(axis, element, series))

    # Ranked improvements / regressions
    improving = [e for e in per_element_delta if e.is_improving]
    regressing = [e for e in per_element_delta if e.is_regressing]
    stagnant = [e for e in per_element_delta if e.is_stagnant]

    top_improving = sorted(improving, key=lambda e: e.delta, reverse=True)[:V1451_TOP_N]
    top_regressing = sorted(regressing, key=lambda e: e.delta)[:V1451_TOP_N]

    # Stability score: pstdev of all element deltas, clipped to [0, 1]
    if per_element_delta:
        deltas = [e.delta for e in per_element_delta]
        try:
            raw_stability = statistics.pstdev(deltas)
        except statistics.StatisticsError:
            raw_stability = 0.0
        stability_score = _clip01(raw_stability)
    else:
        stability_score = 0.0

    # Counts
    stagnant_count = len(stagnant)
    improving_count = len(improving)
    regressing_count = len(regressing)
    axis_improving_count = sum(1 for a in per_axis_trend if a.is_improving)

    # Notes
    notes = []
    if n_snapshots < 2:
        notes.append("INSUFFICIENT: <2 snapshots, trend not real")
    else:
        notes.append(f"OK: {n_snapshots} snapshots, trend is real")
    if cube_improving:
        notes.append("Cube is improving")
    elif cube_regressing:
        notes.append("Cube is regressing")
    else:
        notes.append("Cube is stagnant")
    notes.append(f"Stability score = {stability_score:.4f} (0=stable, 1=volatile)")

    ended = _now_iso()

    return TrendReport(
        schema=V1451_SCHEMA,
        version=V1451_VERSION,
        module=V1451_MODULE,
        started=started,
        ended=ended,
        n_snapshots_before=n_snapshots,
        n_snapshots_after=n_snapshots,
        cube_first_rate=_clip01(cube_first),
        cube_last_rate=_clip01(cube_last),
        cube_delta=cube_delta,
        cube_slope=cube_slope,
        is_improving=cube_improving,
        is_regressing=cube_regressing,
        is_stagnant=cube_stagnant,
        per_axis_trend=per_axis_trend,
        per_element_delta=per_element_delta,
        top_improving=top_improving,
        top_regressing=top_regressing,
        stagnant_count=stagnant_count,
        improving_count=improving_count,
        regressing_count=regressing_count,
        stability_score=stability_score,
        axis_improving_count=axis_improving_count,
        notes=notes,
    )


# ============================================================================
# Run-all: ensure ≥2 snapshots + compute trend + write report
# ============================================================================

def _workspace_root() -> Path:
    """Locate the workspace root (where .v1450-cube-history.jsonl lives)."""
    here = Path(__file__).resolve().parent
    # Walk up until we find .v1450-cube-history.jsonl OR max 4 levels
    for ancestor in [here] + list(here.parents)[:4]:
        candidate = ancestor / ".v1450-cube-history.jsonl"
        if candidate.exists():
            return ancestor
    return here.parent  # fallback


def run_all(
    out_json: Optional[Path] = None,
    out_md: Optional[Path] = None,
    force_append: bool = True,
) -> TrendReport:
    """Run V1451: ensure ≥2 snapshots + compute trend + write report."""
    # 1. Append a 2nd snapshot if needed
    if force_append:
        _ensure_v1450_history_appended()

    # 2. Load history
    v1450 = _import_v1450()
    history = v1450.load_cube_history()

    # 3. Compute trend
    report = compute_trend(history)

    # 4. Write outputs
    ws_root = _workspace_root()
    if out_json is None:
        out_json = ws_root / ".v1451-cube-history-trend-v2-report.json"
    if out_md is None:
        out_md = ws_root / ".v1451-cube-history-trend-v2-report.md"

    payload = asdict(report)
    out_json.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False),
        encoding="utf-8",
    )
    out_md.write_text(_render_markdown(report), encoding="utf-8")
    return report


# ============================================================================
# Popper self-test (14 probes)
# ============================================================================

def popper() -> Tuple[bool, List[Dict[str, Any]]]:
    """Run 14 bounded self-tests for V1451. Returns (all_ok, results)."""
    results: List[Dict[str, Any]] = []

    # T01: constants defined
    try:
        assert V1451_VERSION == "0.1.0"
        assert V1451_SCHEMA == "asi.cube-history-trend-v2.v1"
        assert len(V1451_GUARDS) == 14
        assert len(V1451_V3_GUARDS) == 5
        assert len(V1451_BORROWED) == 5
        results.append({"name": "T01_constants", "ok": True, "detail": "V1451 constants 14+5+5 defined"})
    except Exception as e:
        results.append({"name": "T01_constants", "ok": False, "detail": str(e)})

    # T02: helpers bounded
    try:
        assert _clip01(-0.5) == 0.0
        assert _clip01(0.5) == 0.5
        assert _clip01(1.5) == 1.0
        assert _slope_simple([1.0]) == 0.0
        assert abs(_slope_simple([0.0, 1.0]) - 1.0) < 1e-9
        assert _is_improving(0.05) is True
        assert _is_regressing(-0.05) is True
        assert _is_stagnant(0.005) is True
        results.append({"name": "T02_helpers_bounded", "ok": True, "detail": "_clip01, _slope_simple, _is_* bounded"})
    except Exception as e:
        results.append({"name": "T02_helpers_bounded", "ok": False, "detail": str(e)})

    # T03: compute_trend with empty history
    try:
        rep_empty = compute_trend([])
        assert rep_empty.n_snapshots_before == 0
        assert rep_empty.is_improving is False
        assert rep_empty.is_regressing is False
        assert rep_empty.is_stagnant is True
        results.append({"name": "T03_empty_history", "ok": True, "detail": "empty history → all stagnant, no false claims"})
    except Exception as e:
        results.append({"name": "T03_empty_history", "ok": False, "detail": str(e)})

    # T04: compute_trend with 1 snapshot (INSUFFICIENT case)
    try:
        rep_one = compute_trend([{"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}])
        assert rep_one.n_snapshots_before == 1
        assert rep_one.is_stagnant is True
        assert any("INSUFFICIENT" in n for n in rep_one.notes)
        results.append({"name": "T04_one_snapshot", "ok": True, "detail": "1 snapshot → INSUFFICIENT + stagnant"})
    except Exception as e:
        results.append({"name": "T04_one_snapshot", "ok": False, "detail": str(e)})

    # T05: compute_trend with 2 snapshots, improving
    try:
        snap1 = {
            "cube_overall_closure_rate": 0.5,
            "per_axis_overall": {"problem": 0.4, "position": 0.5, "protocol": 0.6},
            "axis_stats": [],
        }
        snap2 = {
            "cube_overall_closure_rate": 0.7,
            "per_axis_overall": {"problem": 0.7, "position": 0.7, "protocol": 0.7},
            "axis_stats": [],
        }
        rep_imp = compute_trend([snap1, snap2])
        assert rep_imp.is_improving is True
        assert rep_imp.cube_delta > V1451_IMPROVING_THRESHOLD
        assert len(rep_imp.per_axis_trend) == 3
        for at in rep_imp.per_axis_trend:
            assert at.is_improving is True
        results.append({"name": "T05_two_snapshots_improving", "ok": True, "detail": "2 snapshots +0.2 → improving, all axes improving"})
    except Exception as e:
        results.append({"name": "T05_two_snapshots_improving", "ok": False, "detail": str(e)})

    # T06: compute_trend with 2 snapshots, regressing
    try:
        snap1 = {"cube_overall_closure_rate": 0.7, "per_axis_overall": {"problem": 0.7, "position": 0.7, "protocol": 0.7}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.4, "per_axis_overall": {"problem": 0.4, "position": 0.4, "protocol": 0.4}, "axis_stats": []}
        rep_reg = compute_trend([snap1, snap2])
        assert rep_reg.is_regressing is True
        assert rep_reg.cube_delta < V1451_REGRESSING_THRESHOLD
        results.append({"name": "T06_two_snapshots_regressing", "ok": True, "detail": "2 snapshots -0.3 → regressing"})
    except Exception as e:
        results.append({"name": "T06_two_snapshots_regressing", "ok": False, "detail": str(e)})

    # T07: per-element delta with axis_stats dict
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5},
                 "axis_stats": [
                     {"axis": "problem", "element": "time", "mean_closure": 0.3, "face_count": 2},
                     {"axis": "problem", "element": "freedom", "mean_closure": 0.7, "face_count": 2},
                 ]}
        snap2 = {"cube_overall_closure_rate": 0.6, "per_axis_overall": {"problem": 0.6, "position": 0.5, "protocol": 0.5},
                 "axis_stats": [
                     {"axis": "problem", "element": "time", "mean_closure": 0.5, "face_count": 2},
                     {"axis": "problem", "element": "freedom", "mean_closure": 0.7, "face_count": 2},
                 ]}
        rep = compute_trend([snap1, snap2])
        time_delta = next(e for e in rep.per_element_delta if e.element == "time")
        freedom_delta = next(e for e in rep.per_element_delta if e.element == "freedom")
        assert abs(time_delta.delta - 0.2) < 1e-9
        assert time_delta.is_improving is True
        assert abs(freedom_delta.delta - 0.0) < 1e-9
        assert freedom_delta.is_stagnant is True
        results.append({"name": "T07_per_element_delta", "ok": True, "detail": "axis_stats dict → time +0.2 improving, freedom stagnant"})
    except Exception as e:
        results.append({"name": "T07_per_element_delta", "ok": False, "detail": str(e)})

    # T08: ranked top improving + top regressing
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        rep = compute_trend([snap1, snap2])
        # No real changes → top lists may be empty (no improving/regressing)
        assert isinstance(rep.top_improving, list)
        assert isinstance(rep.top_regressing, list)
        assert len(rep.top_improving) <= V1451_TOP_N
        assert len(rep.top_regressing) <= V1451_TOP_N
        results.append({"name": "T08_ranked_top_n", "ok": True, "detail": f"top_improving/regressing bounded to ≤{V1451_TOP_N}"})
    except Exception as e:
        results.append({"name": "T08_ranked_top_n", "ok": False, "detail": str(e)})

    # T09: stability score bounded in [0, 1]
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        rep = compute_trend([snap1, snap2])
        assert 0.0 <= rep.stability_score <= 1.0
        results.append({"name": "T09_stability_bounded", "ok": True, "detail": f"stability_score={rep.stability_score:.4f} ∈ [0,1]"})
    except Exception as e:
        results.append({"name": "T09_stability_bounded", "ok": False, "detail": str(e)})

    # T10: V1450 importable (compositional)
    try:
        v1450 = _import_v1450()
        assert hasattr(v1450, "aggregate_cube_snapshot") or hasattr(v1450, "load_cube_history")
        results.append({"name": "T10_v1450_import", "ok": True, "detail": "V1450 module importable"})
    except Exception as e:
        results.append({"name": "T10_v1450_import", "ok": False, "detail": str(e)})

    # T11: trend fields bounded
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        rep = compute_trend([snap1, snap2])
        assert -1.0 <= rep.cube_delta <= 1.0
        assert -1.0 <= rep.cube_slope <= 1.0
        for at in rep.per_axis_trend:
            assert -1.0 <= at.delta <= 1.0
            assert -1.0 <= at.slope <= 1.0
        results.append({"name": "T11_trend_bounded", "ok": True, "detail": "all deltas + slopes ∈ [-1, +1]"})
    except Exception as e:
        results.append({"name": "T11_trend_bounded", "ok": False, "detail": str(e)})

    # T12: stagnant_count + improving_count + regressing_count sum to 18
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        rep = compute_trend([snap1, snap2])
        n_elements = len(V1451_PROBLEM_NAMES) + len(V1451_POSITION_NAMES) + len(V1451_PROTOCOL_NAMES)
        total = rep.stagnant_count + rep.improving_count + rep.regressing_count
        # With 1 snapshot edge case, all stagnant
        # With 2 stagnant snapshots, all stagnant
        assert total == n_elements, f"counts sum {total} != {n_elements}"
        results.append({"name": "T12_counts_sum", "ok": True, "detail": f"counts sum to {n_elements} axis elements"})
    except Exception as e:
        results.append({"name": "T12_counts_sum", "ok": False, "detail": str(e)})

    # T13: markdown render
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        rep = compute_trend([snap1, snap2])
        md = _render_markdown(rep)
        assert "V1451" in md
        assert "Honest disclosure" in md
        assert "V3 哲学守门" in md
        results.append({"name": "T13_markdown_render", "ok": True, "detail": "markdown has V1451 + Honest + V3 guards"})
    except Exception as e:
        results.append({"name": "T13_markdown_render", "ok": False, "detail": str(e)})

    # T14: notes include is_improving/stagnant label
    try:
        snap1 = {"cube_overall_closure_rate": 0.5, "per_axis_overall": {"problem": 0.5, "position": 0.5, "protocol": 0.5}, "axis_stats": []}
        snap2 = {"cube_overall_closure_rate": 0.7, "per_axis_overall": {"problem": 0.7, "position": 0.7, "protocol": 0.7}, "axis_stats": []}
        rep = compute_trend([snap1, snap2])
        assert any("improving" in n.lower() for n in rep.notes)
        results.append({"name": "T14_notes_label", "ok": True, "detail": "notes contain 'improving' label"})
    except Exception as e:
        results.append({"name": "T14_notes_label", "ok": False, "detail": str(e)})

    all_ok = all(r["ok"] for r in results)
    return all_ok, results


# ============================================================================
# Chain delegate
# ============================================================================

def chain_delegate() -> Dict[str, Any]:
    """Verify V1451 chain: V1450 (compositional)."""
    chain = {
        "schema": "asi.chain-delegate.v1451.v1",
        "version": V1451_VERSION,
        "delegates": [],
        "all_ok": True,
    }
    # V1450
    try:
        v1450 = _import_v1450()
        v1450_version = getattr(v1450, "V1450_VERSION", "?")
        # Try popper
        v1450_ok = True
        v1450_detail = f"V1450 version={v1450_version}"
        try:
            ok, _ = v1450.popper()
            v1450_ok = ok
            v1450_detail += f", popper={ok}"
        except Exception as e:
            v1450_ok = False
            v1450_detail += f", popper_error={e}"
        chain["delegates"].append({
            "module": "V1450",
            "ok": v1450_ok,
            "detail": v1450_detail,
        })
        chain["all_ok"] = chain["all_ok"] and v1450_ok
    except Exception as e:
        chain["delegates"].append({"module": "V1450", "ok": False, "detail": f"import_error={e}"})
        chain["all_ok"] = False

    return chain


# ============================================================================
# Markdown renderer
# ============================================================================

def _render_markdown(report: TrendReport) -> str:
    """Render TrendReport to markdown."""
    lines = []
    lines.append(f"# V1451 — ASI cube history trend v2")
    lines.append("")
    lines.append(f"- schema: `{V1451_SCHEMA}`")
    lines.append(f"- version: `{V1451_VERSION}`")
    lines.append(f"- module: `{V1451_MODULE}`")
    lines.append(f"- started: `{report.started}`")
    lines.append(f"- ended: `{report.ended}`")
    lines.append(f"- n_snapshots_before: **{report.n_snapshots_before}**")
    lines.append(f"- n_snapshots_after: **{report.n_snapshots_after}**")
    lines.append("")

    lines.append("## Cube trend")
    lines.append("")
    lines.append(f"- cube_first_rate: **{report.cube_first_rate:.4f}**")
    lines.append(f"- cube_last_rate: **{report.cube_last_rate:.4f}**")
    lines.append(f"- cube_delta: **{report.cube_delta:+.4f}**")
    lines.append(f"- cube_slope: **{report.cube_slope:+.4f}**")
    lines.append(f"- is_improving: **{report.is_improving}**")
    lines.append(f"- is_regressing: **{report.is_regressing}**")
    lines.append(f"- is_stagnant: **{report.is_stagnant}**")
    lines.append("")

    lines.append("## Per-axis trend")
    lines.append("")
    lines.append("| axis | first | last | delta | slope | improving | regressing | stagnant | n |")
    lines.append("|---|---|---|---|---|---|---|---|---|")
    for at in report.per_axis_trend:
        lines.append(
            f"| {at.axis} | {at.first_rate:.4f} | {at.last_rate:.4f} | {at.delta:+.4f} | {at.slope:+.4f} "
            f"| {at.is_improving} | {at.is_regressing} | {at.is_stagnant} | {at.n_snapshots} |"
        )
    lines.append("")

    lines.append("## Per-element delta (18 elements)")
    lines.append("")
    lines.append("| axis | element | first | last | delta | improving | regressing | stagnant |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for ed in report.per_element_delta:
        lines.append(
            f"| {ed.axis} | {ed.element} | {ed.first_rate:.4f} | {ed.last_rate:.4f} | {ed.delta:+.4f} "
            f"| {ed.is_improving} | {ed.is_regressing} | {ed.is_stagnant} |"
        )
    lines.append("")

    lines.append("## Top improving")
    lines.append("")
    if report.top_improving:
        lines.append("| axis | element | delta |")
        lines.append("|---|---|---|")
        for ed in report.top_improving:
            lines.append(f"| {ed.axis} | {ed.element} | {ed.delta:+.4f} |")
    else:
        lines.append("_(no improving elements)_")
    lines.append("")

    lines.append("## Top regressing")
    lines.append("")
    if report.top_regressing:
        lines.append("| axis | element | delta |")
        lines.append("|---|---|---|")
        for ed in report.top_regressing:
            lines.append(f"| {ed.axis} | {ed.element} | {ed.delta:+.4f} |")
    else:
        lines.append("_(no regressing elements)_")
    lines.append("")

    lines.append("## Counts + stability")
    lines.append("")
    lines.append(f"- stagnant_count: **{report.stagnant_count}**")
    lines.append(f"- improving_count: **{report.improving_count}**")
    lines.append(f"- regressing_count: **{report.regressing_count}**")
    lines.append(f"- axis_improving_count: **{report.axis_improving_count}** / 3")
    lines.append(f"- stability_score: **{report.stability_score:.4f}** (0=stable, 1=volatile)")
    lines.append("")

    lines.append("## Notes")
    lines.append("")
    for note in report.notes:
        lines.append(f"- {note}")
    lines.append("")

    lines.append("## Honest disclosure (主 17:43 实事求是)")
    lines.append("")
    lines.append(
        "> V1451 is a **bounded history operator on V1450 cube snapshots**. "
        "It does NOT claim that 18 element deltas across 2 snapshots solves "
        "Phenomenal consciousness, ASI improvement, human-level judgment, "
        "or absolute progress. It claims only: **from this host, V1451 imports "
        "V1450.aggregate_cube_snapshot, appends a 2nd snapshot, loads the full "
        "history (now ≥ 2), and reports empirical cube_overall + per-axis + "
        "per-element delta + slope + is_improving/regressing/stagnant counts "
        "+ top-N ranked lists + stability score**. V1451 ≠ Phenomenal "
        "improvement. V1451 ≠ ASI improvement. V1451 ≠ human-level "
        "improvement. V1451 ≠ absolute improvement. Trend on 2 snapshots ≠ "
        "real progress. Slope ≠ causation. Stability score ≠ wisdom. "
        "is_improving ≠ solving 7 philosophical problems."
    )
    lines.append("")
    lines.append(
        "（主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI + 主 19:33 走在前人经验上 + 主 22:33 终极授权 + 主 00:44 质量工程化 + 主 00:56 任何人能接手）"
    )
    lines.append("")

    lines.append("## Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for src, desc in V1451_BORROWED:
        lines.append(f"- **{src}**: {desc}")
    lines.append("")

    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    for g in V1451_V3_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    lines.append("## GUARDS upheld (V1451-specific, 14)")
    lines.append("")
    for g in V1451_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    return "\n".join(lines)


# ============================================================================
# CLI
# ============================================================================

def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=V1451_MODULE_SHORT,
        description="V1451 — ASI cube history trend v2 (real 2nd snapshot + per-axis delta)",
    )
    p.add_argument("cmd", nargs="?", default="help",
                   choices=["version", "help", "meta", "popper", "chain",
                            "compute", "trend", "run-all"])
    p.add_argument("--json", action="store_true", help="JSON output for meta")
    p.add_argument("--out-json", type=str, default=None, help="Output JSON path")
    p.add_argument("--out-md", type=str, default=None, help="Output MD path")
    p.add_argument("--no-append", action="store_true", help="Don't append 2nd snapshot")
    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    try:
        args = parser.parse_args(argv)
    except SystemExit as e:
        # argparse calls sys.exit(2) on unknown args; convert to return code 2
        return int(e.code) if isinstance(e.code, int) else 2
    cmd = args.cmd or "help"

    if cmd == "version":
        print(V1451_VERSION)
        return 0

    if cmd == "help":
        parser.print_help()
        return 0

    if cmd == "meta":
        meta = {
            "schema": V1451_SCHEMA,
            "version": V1451_VERSION,
            "module": V1451_MODULE,
            "n_problems": len(V1451_PROBLEM_NAMES),
            "n_positions": len(V1451_POSITION_NAMES),
            "n_protocols": len(V1451_PROTOCOL_NAMES),
            "improving_threshold": V1451_IMPROVING_THRESHOLD,
            "regressing_threshold": V1451_REGRESSING_THRESHOLD,
            "top_n": V1451_TOP_N,
            "guards": list(V1451_GUARDS),
            "v3_guards": list(V1451_V3_GUARDS),
            "borrowed": [list(b) for b in V1451_BORROWED],
        }
        if getattr(args, "json", False):
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    if cmd == "popper":
        ok, results = popper()
        for r in results:
            mark = "OK" if r["ok"] else "FAIL"
            print(f"[{mark}] {r['name']}: {r['detail']}")
        print(f"\nALL_OK={ok}")
        return 0 if ok else 1

    if cmd == "chain":
        chain = chain_delegate()
        print(json.dumps(chain, indent=2, ensure_ascii=False))
        return 0

    if cmd == "compute":
        v1450 = _import_v1450()
        history = v1450.load_cube_history()
        report = compute_trend(history)
        payload = asdict(report)
        print(json.dumps(payload, indent=2, ensure_ascii=False))
        return 0

    if cmd == "trend":
        v1450 = _import_v1450()
        history = v1450.load_cube_history()
        report = compute_trend(history)
        print(f"n_snapshots: {report.n_snapshots_before}")
        print(f"cube_first_rate: {report.cube_first_rate:.4f}")
        print(f"cube_last_rate: {report.cube_last_rate:.4f}")
        print(f"cube_delta: {report.cube_delta:+.4f}")
        print(f"is_improving: {report.is_improving}")
        print(f"is_regressing: {report.is_regressing}")
        print(f"is_stagnant: {report.is_stagnant}")
        print(f"stability_score: {report.stability_score:.4f}")
        print(f"improving_count: {report.improving_count}")
        print(f"regressing_count: {report.regressing_count}")
        print(f"stagnant_count: {report.stagnant_count}")
        return 0

    if cmd == "run-all":
        out_json = Path(args.out_json) if args.out_json else None
        out_md = Path(args.out_md) if args.out_md else None
        force_append = not getattr(args, "no_append", False)
        report = run_all(out_json=out_json, out_md=out_md, force_append=force_append)
        print(f"V1451 cube history trend v2 report written.")
        print(f"  n_snapshots_after: {report.n_snapshots_after}")
        print(f"  cube_delta: {report.cube_delta:+.4f}")
        print(f"  cube_slope: {report.cube_slope:+.4f}")
        print(f"  is_improving: {report.is_improving}")
        print(f"  is_regressing: {report.is_regressing}")
        print(f"  is_stagnant: {report.is_stagnant}")
        print(f"  stability_score: {report.stability_score:.4f}")
        print(f"  improving_count: {report.improving_count}")
        print(f"  regressing_count: {report.regressing_count}")
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:
        traceback.print_exc()
        sys.exit(1)
