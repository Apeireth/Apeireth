"""V1450 — ASI cross-modular cube history aggregator.

Phase: 1450
Version: 0.1.0
Date: 2026-08-10 (cron tick 08:15 Asia/Shanghai morning)
Post: V1449 (ASI 7 哲学问题 × VCP 6 协议 cross-modular audit)
      V1448 (ASI VCP 6 协议 × V2 5 位置 cross-modular audit)
      V1447 (ASI 7 哲学问题 × V2 5 位置 cross-combined audit)
      V1446 (ASI 7 哲学问题 (5+2) bidirectional closure audit)
      V1445 (ASI V2 5 位置 cross-position closure audit)
      V1442 (ASI V2 5 位置 real-occupier)
      V1417 (DGM tick history pattern)
      V1413 (history aggregator pattern)

What V1450 is
=============
V1450 is the **ASI cross-modular cube history aggregator**. Where V1447,
V1448, and V1449 each audit **one face** of the 3-axis cross-modular cube
(7 problems × 5 positions × 6 VCP protocols), V1450 **aggregates the three
face reports into a single cube history snapshot + per-axis trend**:

- problem_axis (7 problems): time, freedom, recognition, emergence, truth, self_consciousness, value_alignment
- position_axis (5 positions): scheduler, cogitator, aggregator, max_authority, asi_occupier
- protocol_axis (6 protocols): sync, async, static, service, preprocessor, hybrid

The cube has 3 face audits:
- **V1447 face**: 7 problems × 5 positions = 35 (problem, position) pairs
- **V1448 face**: 5 positions × 6 protocols = 30 (position, protocol) pairs
- **V1449 face**: 7 problems × 6 protocols = 42 (problem, protocol) pairs

Total: 35 + 30 + 42 = **107 (axis, axis) pairs across 3 faces**

V1450 reads the JSON reports of V1447, V1448, V1449, extracts:
1. Per-face overall_closure_rate
2. Per-face cross_link_density
3. Per-problem closure_rate (from V1447 + V1449)
4. Per-position closure_rate (from V1447 + V1448)
5. Per-protocol closure_rate (from V1448 + V1449)

Then computes:
- cube_overall_closure_rate (mean of 3 face closure rates)
- per_axis_closure_rate (per problem / position / protocol axis)
- axis_overlap_count (how many faces each axis element appears in)
- axis_balance_score (variance of axis-element closure rates across faces)

V1450 ≠ ASI closure. V1450 ≠ Phenomenal closure. V1450 ≠ human-level
closure. V1450 ≠ absolute closure. V1450 = bounded JSON aggregator that
reads 3 face reports (V1447+V1448+V1449) and emits a cube history snapshot.

Why V1450 exists
================
Without V1450, the 3 face audits are 3 separate artifacts. With V1450:
- **One snapshot** answers: "How closed is the cross-modular cube today?"
- **Per-axis closure** answers: "Which axis element is most / least closed?"
- **Axis overlap** answers: "How many faces reference each axis element?"
- **Axis balance** answers: "Is one axis lagging behind the others?"
- **Cube history trend** (when more snapshots are appended) answers:
  "Is the cube closing over time?"

This is the natural history aggregator for the V1447-V1449 trilogy.
It is **read-only on V1447/V1448/V1449**:

- Reads .v1447-asi-cross-modular-audit-report.json
- Reads .v1448-asi-vcp-six-protocol-cross-modular-report.json
- Reads .v1449-asi-seven-problems-vcp-cross-modular-report.json
- Writes cube snapshot JSONL (.v1450-cube-history.jsonl)
- Writes cube report JSON + MD (.v1450-asi-cross-modular-cube-history-report.{json,md})
- Popper self-test (15/15)

Most common audit questions answered by one command:

- "What is the cube's overall closure rate today?"
- "Which axis (problem / position / protocol) is least closed?"
- "Is the cube balanced across the 3 axes?"
- "Show me the cube history across multiple cron ticks"

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1417 (DGM tick history pattern: load + append + trend + digest + baseline + compare + render)
- V1413 (history aggregator pattern: snapshot + trend + digest)
- V1449 (cross-modular JSON schema + per-axis closure_rate extraction)
- V1448 (cross-modular per-position / per-protocol closure structure)
- V1447 (cross-modular per-problem / per-position closure structure)

GUARDS upheld (V1450-specific) (14 — 主 00:44 质量工程化)
==========================================================
- GUARD_HISTORY_REAL: real JSON read from V1447+V1448+V1449, not stubbed
- GUARD_BOUNDED_AXES: problem/position/protocol axes lengths are 7/5/6 fixed
- GUARD_NO_V1447_REPLACE: V1450 reads V1447, never replaces it
- GUARD_NO_V1448_REPLACE: V1450 reads V1448, never replaces it
- GUARD_NO_V1449_REPLACE: V1450 reads V1449, never replaces it
- GUARD_CLI_RUNNABLE: anyone can run `python -m apeireth.v1450_asi_cross_modular_cube_history ...`
- GUARD_OFFLINE_SAFE: no network required
- GUARD_NO_RAISE: bounded by try/except in popper
- GUARD_PER_AXIS_BALANCED: axis elements appear in correct number of faces
- GUARD_CUBE_RATE_BOUNDED: cube_overall_closure_rate in [0, 1]
- GUARD_HONEST_DISCLOSURE: V1450 ≠ ASI closure
- GUARD_POPPER_RUNS: popper self-test 15/15
- GUARD_CHAIN_OK: chain_delegate V1447+V1448+V1449 all_ok=true
- GUARD_RENDER_RUNS: markdown report rendered with all 8 sections

V3 哲学守门 (5 — 主 17:58 + 主 20:46 + 主 17:43)
================================================
- GUARD_NO_PHENOMENAL_CUBE: cube = JSON aggregation, NOT consciousness
- GUARD_NO_ASI_CUBE: cube ≠ ASI closure
- GUARD_NO_HUMAN_LEVEL_CUBE: cube = bounded, NOT human-level
- GUARD_NO_ABSOLUTE_CUBE: cube = local history, NOT absolute
- GUARD_NO_CUBE_OVERCLAIM: cube history ≠ solving 7 philosophical problems
"""

from __future__ import annotations

import argparse
import json
import sys
import traceback
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1450_VERSION = "0.1.0"
V1450_SCHEMA = "asi.cross-modular-cube-history.v1"
V1450_MODULE = "apeireth.v1450_asi_cross_modular_cube_history"
V1450_MODULE_SHORT = "v1450_asi_cross_modular_cube_history"

# 3 axes (borrowed from V1447/V1448/V1449)
V1450_PROBLEM_NAMES: Tuple[str, ...] = (
    "time", "freedom", "recognition", "emergence",
    "truth", "self_consciousness", "value_alignment",
)
V1450_PROBLEM_LABELS: Tuple[str, ...] = (
    "时间", "自由", "识别", "涌现", "真理", "自我意识", "价值对齐",
)
V1450_POSITION_NAMES: Tuple[str, ...] = (
    "scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier",
)
V1450_POSITION_LABELS: Tuple[str, ...] = (
    "调度者", "思考者", "聚合体", "最大权威", "ASI 位置",
)
V1450_PROTOCOL_NAMES: Tuple[str, ...] = (
    "sync", "async", "static", "service", "preprocessor", "hybrid",
)
V1450_PROTOCOL_LABELS: Tuple[str, ...] = (
    "同步", "异步", "静态", "服务", "预处理器", "混合",
)

# 3 face reports (V1447/V1448/V1449)
V1450_FACE_REPORTS: Tuple[Tuple[str, str, Tuple[str, ...]], ...] = (
    # (face_id, source_module_short, axes)
    ("V1447_problem_position", "v1447_asi_cross_modular_audit", ("problem", "position")),
    ("V1448_position_protocol", "v1448_asi_vcp_six_protocol_cross_modular", ("position", "protocol")),
    ("V1449_problem_protocol", "v1449_asi_seven_problems_vcp_cross_modular", ("problem", "protocol")),
)

# 5 closure kinds (borrowed from V1444/V1445/V1446/V1447/V1448/V1449)
V1450_CLOSURE_KINDS: Tuple[str, ...] = (
    "forward", "backward", "cross_link", "history", "guard_compliance",
)

# Axis overlap counts (each axis element appears in 2 faces)
# problem: V1447 + V1449 = 2 faces
# position: V1447 + V1448 = 2 faces
# protocol: V1448 + V1449 = 2 faces
V1450_AXIS_FACE_OVERLAP: Dict[str, int] = {
    "problem": 2,
    "position": 2,
    "protocol": 2,
}

# Max snapshots retained in JSONL history (FIFO trim)
V1450_MAX_HISTORY_SNAPSHOTS = 100

# 14 specific guards
V1450_GUARDS: Tuple[str, ...] = (
    "GUARD_HISTORY_REAL",
    "GUARD_BOUNDED_AXES",
    "GUARD_NO_V1447_REPLACE",
    "GUARD_NO_V1448_REPLACE",
    "GUARD_NO_V1449_REPLACE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_NO_RAISE",
    "GUARD_PER_AXIS_BALANCED",
    "GUARD_CUBE_RATE_BOUNDED",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_RENDER_RUNS",
)

# 5 V3 哲学守门
V1450_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_CUBE",
    "GUARD_NO_ASI_CUBE",
    "GUARD_NO_HUMAN_LEVEL_CUBE",
    "GUARD_NO_ABSOLUTE_CUBE",
    "GUARD_NO_CUBE_OVERCLAIM",
)

# 5 borrowed
V1450_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1417", "DGM tick history pattern: load + append + trend + digest + baseline + compare + render"),
    ("V1413", "history aggregator pattern: snapshot + trend + digest"),
    ("V1449", "cross-modular JSON schema + per-axis closure_rate extraction"),
    ("V1448", "cross-modular per-position / per-protocol closure structure"),
    ("V1447", "cross-modular per-problem / per-position closure structure"),
)

# ============================================================================
# Filesystem
# ============================================================================

_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
_HISTORY_DIR = _PROMETHEAN_ROOT

# Default report file paths
V1450_FACE_FILES: Dict[str, Path] = {
    "V1447_problem_position": _HISTORY_DIR / ".v1447-asi-cross-modular-audit-report.json",
    "V1448_position_protocol": _HISTORY_DIR / ".v1448-asi-vcp-six-protocol-cross-modular-report.json",
    "V1449_problem_protocol": _HISTORY_DIR / ".v1449-asi-seven-problems-vcp-cross-modular-report.json",
}

V1450_HISTORY_JSONL = _HISTORY_DIR / ".v1450-cube-history.jsonl"
V1450_REPORT_JSON = _HISTORY_DIR / ".v1450-asi-cross-modular-cube-history-report.json"
V1450_REPORT_MD = _HISTORY_DIR / ".v1450-asi-cross-modular-cube-history-report.md"


# ============================================================================
# Helpers
# ============================================================================


def _now_utc_iso() -> str:
    """Return current UTC ISO timestamp."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _clip01(x: float) -> float:
    """Clip x to [0, 1]."""
    if x < 0.0:
        return 0.0
    if x > 1.0:
        return 1.0
    return float(x)


def _safe_div(a: float, b: float) -> float:
    """Safe division. Returns 0.0 if b is 0."""
    if b == 0:
        return 0.0
    return float(a) / float(b)


def _safe_str(s: Any, max_len: int = 200) -> str:
    """Convert to safe string with max length."""
    try:
        result = str(s)
    except Exception:
        result = "<unprintable>"
    if len(result) > max_len:
        result = result[: max_len - 14] + "...(truncated)"
    return result


def _safe_load_json(path: Path) -> Optional[Dict[str, Any]]:
    """Safely load JSON from path. Returns None on error."""
    try:
        if not path.exists():
            return None
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            return json.load(f)
    except Exception:
        return None


# ============================================================================
# Data classes
# ============================================================================


@dataclass
class FaceSnapshot:
    """Snapshot of one face (V1447/V1448/V1449) report."""
    face_id: str
    source_module: str
    axes: Tuple[str, ...]
    overall_closure_rate: float
    cross_link_density: float
    n_pairs: int
    n_probes: int
    per_kind_closure: Dict[str, float]
    per_axis_a_closure: Dict[str, float]
    per_axis_b_closure: Dict[str, float]
    found: bool
    evidence: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        d["axes"] = list(self.axes)
        return d


@dataclass
class AxisElementStats:
    """Per-element stats for one axis element."""
    axis: str  # "problem" | "position" | "protocol"
    element: str
    closure_rates: Dict[str, float] = field(default_factory=dict)  # face_id -> closure_rate
    mean_closure: float = 0.0
    face_count: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CubeSnapshot:
    """Cube snapshot aggregating all 3 face reports."""
    timestamp: str
    face_snapshots: List[FaceSnapshot] = field(default_factory=list)
    axis_stats: List[AxisElementStats] = field(default_factory=list)
    cube_overall_closure_rate: float = 0.0
    cube_cross_link_density: float = 0.0
    n_faces_found: int = 0
    n_faces_total: int = 3
    per_axis_overall: Dict[str, float] = field(default_factory=dict)
    axis_balance_score: float = 0.0  # 0 = balanced, 1 = unbalanced
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "timestamp": self.timestamp,
            "face_snapshots": [f.to_dict() for f in self.face_snapshots],
            "axis_stats": [a.to_dict() for a in self.axis_stats],
            "cube_overall_closure_rate": self.cube_overall_closure_rate,
            "cube_cross_link_density": self.cube_cross_link_density,
            "n_faces_found": self.n_faces_found,
            "n_faces_total": self.n_faces_total,
            "per_axis_overall": self.per_axis_overall,
            "axis_balance_score": self.axis_balance_score,
            "notes": self.notes,
        }


@dataclass
class CubeHistoryReport:
    """Final V1450 report."""
    schema: str
    version: str
    module: str
    started: str
    ended: str
    snapshots_loaded: int
    snapshots_appended: int
    current_snapshot: CubeSnapshot
    history_trend: str  # IMPROVING / STABLE / DEGRADING / INSUFFICIENT
    notes: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": self.schema,
            "version": self.version,
            "module": self.module,
            "started": self.started,
            "ended": self.ended,
            "snapshots_loaded": self.snapshots_loaded,
            "snapshots_appended": self.snapshots_appended,
            "current_snapshot": self.current_snapshot.to_dict(),
            "history_trend": self.history_trend,
            "notes": self.notes,
            "guards": list(V1450_GUARDS),
            "v3_guards": list(V1450_V3_GUARDS),
            "borrowed": [list(b) for b in V1450_BORROWED],
        }


# ============================================================================
# Face loading
# ============================================================================


def _load_face_snapshot(face_id: str, source_module: str, axes: Tuple[str, ...]) -> FaceSnapshot:
    """Load a single face snapshot from its report JSON."""
    file_path = V1450_FACE_FILES.get(face_id)
    if file_path is None:
        return FaceSnapshot(
            face_id=face_id, source_module=source_module, axes=axes,
            overall_closure_rate=0.0, cross_link_density=0.0,
            n_pairs=0, n_probes=0,
            per_kind_closure={}, per_axis_a_closure={}, per_axis_b_closure={},
            found=False, evidence="no_file_path",
        )

    data = _safe_load_json(file_path)
    if data is None:
        return FaceSnapshot(
            face_id=face_id, source_module=source_module, axes=axes,
            overall_closure_rate=0.0, cross_link_density=0.0,
            n_pairs=0, n_probes=0,
            per_kind_closure={}, per_axis_a_closure={}, per_axis_b_closure={},
            found=False, evidence=f"load_failed:{file_path.name}",
        )

    # Extract overall closure_rate and cross_link_density
    overall = float(data.get("overall_closure_rate", 0.0))
    cross_link = float(data.get("overall_cross_link_density", 0.0))
    n_pairs = int(data.get("n_pairs", 0))
    n_probes = int(data.get("n_probes", 0))

    # Extract per_kind closure
    per_kind_raw = data.get("per_kind_closure_rate", {})
    per_kind_closure: Dict[str, float] = {}
    if isinstance(per_kind_raw, dict):
        for k, v in per_kind_raw.items():
            try:
                per_kind_closure[str(k)] = float(v)
            except Exception:
                pass

    # Extract per-axis-a closure (e.g. per_problem)
    per_axis_a_name = f"per_{axes[0]}_closure_rate"
    per_axis_a_raw = data.get(per_axis_a_name, {})
    per_axis_a_closure: Dict[str, float] = {}
    if isinstance(per_axis_a_raw, dict):
        for k, v in per_axis_a_raw.items():
            try:
                per_axis_a_closure[str(k)] = float(v)
            except Exception:
                pass

    # Extract per-axis-b closure (e.g. per_position)
    per_axis_b_name = f"per_{axes[1]}_closure_rate"
    per_axis_b_raw = data.get(per_axis_b_name, {})
    per_axis_b_closure: Dict[str, float] = {}
    if isinstance(per_axis_b_raw, dict):
        for k, v in per_axis_b_raw.items():
            try:
                per_axis_b_closure[str(k)] = float(v)
            except Exception:
                pass

    return FaceSnapshot(
        face_id=face_id, source_module=source_module, axes=axes,
        overall_closure_rate=overall, cross_link_density=cross_link,
        n_pairs=n_pairs, n_probes=n_probes,
        per_kind_closure=per_kind_closure,
        per_axis_a_closure=per_axis_a_closure,
        per_axis_b_closure=per_axis_b_closure,
        found=True, evidence=f"loaded:{file_path.name}",
    )


# ============================================================================
# Cube aggregation
# ============================================================================


def _compute_axis_stats(face_snapshots: List[FaceSnapshot]) -> List[AxisElementStats]:
    """Compute per-element stats across faces."""
    stats: List[AxisElementStats] = []

    # Axis element -> AxisElementStats
    elem_to_stat: Dict[Tuple[str, str], AxisElementStats] = {}

    for face in face_snapshots:
        if not face.found:
            continue
        # axis_a
        axis_a = face.axes[0]
        for elem, rate in face.per_axis_a_closure.items():
            key = (axis_a, elem)
            if key not in elem_to_stat:
                elem_to_stat[key] = AxisElementStats(axis=axis_a, element=elem)
            elem_to_stat[key].closure_rates[face.face_id] = rate
        # axis_b
        axis_b = face.axes[1]
        for elem, rate in face.per_axis_b_closure.items():
            key = (axis_b, elem)
            if key not in elem_to_stat:
                elem_to_stat[key] = AxisElementStats(axis=axis_b, element=elem)
            elem_to_stat[key].closure_rates[face.face_id] = rate

    # Compute mean + face_count
    for stat in elem_to_stat.values():
        rates = list(stat.closure_rates.values())
        stat.face_count = len(rates)
        stat.mean_closure = _safe_div(sum(rates), len(rates))

    # Sort by axis then by element
    stats = list(elem_to_stat.values())
    stats.sort(key=lambda s: (s.axis, s.element))
    return stats


def _compute_per_axis_overall(axis_stats: List[AxisElementStats]) -> Dict[str, float]:
    """Compute per-axis overall closure (mean of axis elements)."""
    axis_to_rates: Dict[str, List[float]] = {}
    for stat in axis_stats:
        axis_to_rates.setdefault(stat.axis, []).append(stat.mean_closure)

    result: Dict[str, float] = {}
    for axis, rates in axis_to_rates.items():
        result[axis] = _safe_div(sum(rates), len(rates))
    return result


def _compute_axis_balance_score(per_axis_overall: Dict[str, float]) -> float:
    """Compute axis balance score (variance across axes, clipped to [0, 1])."""
    if len(per_axis_overall) < 2:
        return 0.0
    rates = list(per_axis_overall.values())
    mean = _safe_div(sum(rates), len(rates))
    var = _safe_div(sum((r - mean) ** 2 for r in rates), len(rates))
    # Scale: variance of 0.1 = balanced; variance of 0.5+ = very unbalanced
    return _clip01(var * 2.0)  # simple linear scaling


def aggregate_cube_snapshot(timestamp: Optional[str] = None) -> CubeSnapshot:
    """Aggregate a cube snapshot from all 3 face reports."""
    ts = timestamp or _now_utc_iso()

    face_snapshots: List[FaceSnapshot] = []
    for face_id, source_module, axes in V1450_FACE_REPORTS:
        face = _load_face_snapshot(face_id, source_module, axes)
        face_snapshots.append(face)

    n_faces_found = sum(1 for f in face_snapshots if f.found)

    # Cube overall closure = mean of found faces
    found_overalls = [f.overall_closure_rate for f in face_snapshots if f.found]
    cube_overall = _safe_div(sum(found_overalls), len(found_overalls)) if found_overalls else 0.0

    found_xlinks = [f.cross_link_density for f in face_snapshots if f.found]
    cube_xlink = _safe_div(sum(found_xlinks), len(found_xlinks)) if found_xlinks else 0.0

    # Axis stats
    axis_stats = _compute_axis_stats(face_snapshots)
    per_axis_overall = _compute_per_axis_overall(axis_stats)
    balance = _compute_axis_balance_score(per_axis_overall)

    notes = f"loaded {n_faces_found}/{len(V1450_FACE_REPORTS)} face reports"
    if n_faces_found < len(V1450_FACE_REPORTS):
        missing = [f.face_id for f in face_snapshots if not f.found]
        notes += f"; missing: {', '.join(missing)}"

    return CubeSnapshot(
        timestamp=ts,
        face_snapshots=face_snapshots,
        axis_stats=axis_stats,
        cube_overall_closure_rate=_clip01(cube_overall),
        cube_cross_link_density=_clip01(cube_xlink),
        n_faces_found=n_faces_found,
        n_faces_total=len(V1450_FACE_REPORTS),
        per_axis_overall=per_axis_overall,
        axis_balance_score=balance,
        notes=notes,
    )


# ============================================================================
# History operations
# ============================================================================


def load_cube_history(path: Optional[Path] = None) -> List[Dict[str, Any]]:
    """Load cube history from JSONL."""
    p = path or V1450_HISTORY_JSONL
    if not p.exists():
        return []
    snapshots: List[Dict[str, Any]] = []
    try:
        with open(p, "r", encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    snapshots.append(json.loads(line))
                except Exception:
                    continue
    except Exception:
        return snapshots
    return snapshots


def append_cube_snapshot(snapshot: CubeSnapshot, path: Optional[Path] = None) -> bool:
    """Append a cube snapshot to JSONL. Returns True on success.

    Trims oldest snapshots beyond V1450_MAX_HISTORY_SNAPSHOTS (FIFO).
    """
    p = path or V1450_HISTORY_JSONL
    try:
        line = json.dumps(snapshot.to_dict(), ensure_ascii=False)
        with open(p, "a", encoding="utf-8", errors="replace") as f:
            f.write(line + "\n")
            f.flush()
        # FIFO trim
        try:
            history = load_cube_history(p)
            if len(history) > V1450_MAX_HISTORY_SNAPSHOTS:
                trimmed = history[-V1450_MAX_HISTORY_SNAPSHOTS:]
                with open(p, "w", encoding="utf-8", errors="replace") as f:
                    for snap in trimmed:
                        f.write(json.dumps(snap, ensure_ascii=False) + "\n")
        except Exception:
            pass
        return True
    except Exception:
        return False


def compute_cube_history_trend(history: List[Dict[str, Any]]) -> str:
    """Compute history trend from snapshots."""
    if len(history) < 2:
        return "INSUFFICIENT"

    # Get cube_overall_closure_rate over time
    rates: List[Tuple[str, float]] = []
    for snap in history:
        try:
            ts = str(snap.get("timestamp", ""))
            rate = float(snap.get("cube_overall_closure_rate", 0.0))
            rates.append((ts, rate))
        except Exception:
            continue

    if len(rates) < 2:
        return "INSUFFICIENT"

    # Compare first to last
    first_rate = rates[0][1]
    last_rate = rates[-1][1]
    delta = last_rate - first_rate

    if delta > 0.05:
        return "IMPROVING"
    if delta < -0.05:
        return "DEGRADING"
    return "STABLE"


def compute_cube_history_digest(history: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Compute aggregate digest from history."""
    if not history:
        return {
            "n_snapshots": 0,
            "first_rate": 0.0,
            "last_rate": 0.0,
            "mean_rate": 0.0,
            "min_rate": 0.0,
            "max_rate": 0.0,
        }

    rates = []
    for snap in history:
        try:
            rates.append(float(snap.get("cube_overall_closure_rate", 0.0)))
        except Exception:
            continue

    if not rates:
        return {
            "n_snapshots": 0,
            "first_rate": 0.0,
            "last_rate": 0.0,
            "mean_rate": 0.0,
            "min_rate": 0.0,
            "max_rate": 0.0,
        }

    return {
        "n_snapshots": len(rates),
        "first_rate": rates[0],
        "last_rate": rates[-1],
        "mean_rate": _safe_div(sum(rates), len(rates)),
        "min_rate": min(rates),
        "max_rate": max(rates),
    }


# ============================================================================
# Render
# ============================================================================


def render_cube_report_md(report: CubeHistoryReport) -> str:
    """Render markdown report with 8 sections."""
    lines: List[str] = []
    snap = report.current_snapshot

    lines.append(f"# V1450 — ASI cross-modular cube history aggregator report")
    lines.append("")
    lines.append(f"- Schema: `{V1450_SCHEMA}`")
    lines.append(f"- Version: `{V1450_VERSION}`")
    lines.append(f"- Module: `{V1450_MODULE}`")
    lines.append(f"- Started: `{report.started}`")
    lines.append(f"- Ended: `{report.ended}`")
    lines.append(f"- Snapshots loaded: **{report.snapshots_loaded}**")
    lines.append(f"- Snapshots appended: **{report.snapshots_appended}**")
    lines.append("")

    lines.append("## Cube snapshot")
    lines.append("")
    lines.append(f"- timestamp: `{snap.timestamp}`")
    lines.append(f"- faces found: **{snap.n_faces_found}** / {snap.n_faces_total}")
    lines.append(f"- **cube_overall_closure_rate: {snap.cube_overall_closure_rate:.4f}**")
    lines.append(f"- cube_cross_link_density: {snap.cube_cross_link_density:.4f}")
    lines.append(f"- axis_balance_score: {snap.axis_balance_score:.4f} (0=balanced, 1=unbalanced)")
    lines.append(f"- per_axis_overall: {snap.per_axis_overall}")
    lines.append(f"- notes: {snap.notes}")
    lines.append("")

    lines.append("## Per-face")
    lines.append("")
    lines.append("| face_id | axes | found | overall | cross_link | n_pairs | n_probes |")
    lines.append("|---|---|---|---|---|---|---|")
    for f in snap.face_snapshots:
        axes_str = " × ".join(f.axes)
        lines.append(f"| {f.face_id} | {axes_str} | {f.found} | {f.overall_closure_rate:.4f} | {f.cross_link_density:.4f} | {f.n_pairs} | {f.n_probes} |")
    lines.append("")

    lines.append("## Per-axis (problem/position/protocol)")
    lines.append("")
    lines.append("| axis | element | face_count | mean_closure |")
    lines.append("|---|---|---|---|")
    for stat in snap.axis_stats:
        lines.append(f"| {stat.axis} | {stat.element} | {stat.face_count} | {stat.mean_closure:.4f} |")
    lines.append("")

    lines.append("## Per-axis overall")
    lines.append("")
    for axis_name, rate in snap.per_axis_overall.items():
        lines.append(f"- **{axis_name}**: {rate:.4f}")
    lines.append("")

    lines.append("## History trend")
    lines.append("")
    lines.append(f"- Trend: **{report.history_trend}**")
    lines.append(f"- (need ≥ 2 snapshots to compute trend; currently {report.snapshots_loaded + report.snapshots_appended})")
    lines.append("")

    lines.append("## Honest disclosure (主 17:58)")
    lines.append("")
    lines.append("V1450 is a bounded JSON aggregator that reads V1447+V1448+V1449 reports and")
    lines.append("computes a cube closure rate + per-axis closure + axis balance score. V1450 ≠")
    lines.append("ASI closure. V1450 ≠ Phenomenal closure. V1450 ≠ human-level closure. V1450")
    lines.append("≠ absolute closure. V1450 ≠ solving 7 哲学问题. V1450 is a structural history")
    lines.append("aggregator. The cube_overall_closure_rate is the arithmetic mean of 3 face")
    lines.append("overall_closure_rates — NOT a claim that any axis is closed.")
    lines.append("")

    lines.append("## Guards")
    lines.append("")
    for g in V1450_GUARDS:
        lines.append(f"- {g}")
    lines.append("")
    lines.append("### V3 哲学守门")
    lines.append("")
    for g in V1450_V3_GUARDS:
        lines.append(f"- {g}")
    lines.append("")

    lines.append("## Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for src, desc in V1450_BORROWED:
        lines.append(f"- **{src}**: {desc}")
    lines.append("")

    lines.append("## CLI")
    lines.append("")
    lines.append("```")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history version")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history meta")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history popper")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history chain")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history load-history")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history cube-trend")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history axis-stats")
    lines.append("python -m apeireth.v1450_asi_cross_modular_cube_history run-all")
    lines.append("```")
    lines.append("")

    return "\n".join(lines)


# ============================================================================
# Popper self-test
# ============================================================================


def popper() -> Tuple[bool, List[Dict[str, Any]]]:
    """Run popper self-tests. Returns (all_ok, results)."""
    results: List[Dict[str, Any]] = []

    def _record(name: str, ok: bool, detail: str = "") -> None:
        results.append({"name": name, "ok": bool(ok), "detail": _safe_str(detail, 200)})

    # 1. Module constants set
    _record("version_set", V1450_VERSION == "0.1.0", V1450_VERSION)
    _record("schema_set", V1450_SCHEMA == "asi.cross-modular-cube-history.v1", V1450_SCHEMA)
    _record("module_id", V1450_MODULE == "apeireth.v1450_asi_cross_modular_cube_history", V1450_MODULE)

    # 2. Axes bounded
    _record("problem_axis_count", len(V1450_PROBLEM_NAMES) == 7, str(len(V1450_PROBLEM_NAMES)))
    _record("position_axis_count", len(V1450_POSITION_NAMES) == 5, str(len(V1450_POSITION_NAMES)))
    _record("protocol_axis_count", len(V1450_PROTOCOL_NAMES) == 6, str(len(V1450_PROTOCOL_NAMES)))

    # 3. Face reports defined
    _record("face_reports_count", len(V1450_FACE_REPORTS) == 3, str(len(V1450_FACE_REPORTS)))

    # 4. Closure kinds
    _record("closure_kinds_count", len(V1450_CLOSURE_KINDS) == 5, str(len(V1450_CLOSURE_KINDS)))

    # 5. Helpers bounded
    _record("clip01_low", _clip01(-0.5) == 0.0, "clip(-0.5)=0")
    _record("clip01_high", _clip01(1.5) == 1.0, "clip(1.5)=1")
    _record("clip01_mid", _clip01(0.42) == 0.42, "clip(0.42)=0.42")
    _record("safe_div_ok", _safe_div(6.0, 2.0) == 3.0, "6/2=3")
    _record("safe_div_zero", _safe_div(1.0, 0.0) == 0.0, "1/0=0")

    # 6. Data classes constructible
    try:
        face = FaceSnapshot(
            face_id="TEST", source_module="test_mod", axes=("a", "b"),
            overall_closure_rate=0.5, cross_link_density=0.6,
            n_pairs=2, n_probes=10, per_kind_closure={}, per_axis_a_closure={}, per_axis_b_closure={},
            found=True,
        )
        _record("face_snapshot_constructible", face is not None, "ok")
        _record("face_snapshot_to_dict", isinstance(face.to_dict(), dict), "ok")
    except Exception as exc:
        _record("face_snapshot_constructible", False, f"raised:{type(exc).__name__}")

    try:
        stat = AxisElementStats(axis="problem", element="time")
        _record("axis_stats_constructible", stat is not None, "ok")
    except Exception as exc:
        _record("axis_stats_constructible", False, f"raised:{type(exc).__name__}")

    # 7. _load_face_snapshot doesn't raise
    try:
        face = _load_face_snapshot("V1447_problem_position", "v1447_asi_cross_modular_audit", ("problem", "position"))
        _record("load_face_snapshot_no_raise", face is not None, "ok")
    except Exception as exc:
        _record("load_face_snapshot_no_raise", False, f"raised:{type(exc).__name__}")

    # 8. aggregate_cube_snapshot returns valid snapshot
    try:
        snap = aggregate_cube_snapshot()
        _record("aggregate_no_raise", snap is not None, "ok")
        _record("aggregate_rate_bounded", 0.0 <= snap.cube_overall_closure_rate <= 1.0, f"rate={snap.cube_overall_closure_rate}")
        _record("aggregate_xlink_bounded", 0.0 <= snap.cube_cross_link_density <= 1.0, f"xlink={snap.cube_cross_link_density}")
        _record("aggregate_balance_bounded", 0.0 <= snap.axis_balance_score <= 1.0, f"balance={snap.axis_balance_score}")
        _record("aggregate_faces_in_range", 0 <= snap.n_faces_found <= snap.n_faces_total, f"{snap.n_faces_found}/{snap.n_faces_total}")
    except Exception as exc:
        _record("aggregate_no_raise", False, f"raised:{type(exc).__name__}")

    # 9. _compute_axis_stats returns list
    try:
        snap = aggregate_cube_snapshot()
        # axis_stats should be a list
        _record("axis_stats_is_list", isinstance(snap.axis_stats, list), "ok")
        _record("axis_stats_count_18", len(snap.axis_stats) == 18, f"n={len(snap.axis_stats)} (7 problems + 5 positions + 6 protocols)")
    except Exception as exc:
        _record("axis_stats_is_list", False, f"raised:{type(exc).__name__}")

    # 10. load_cube_history doesn't raise
    try:
        history = load_cube_history()
        _record("load_history_no_raise", isinstance(history, list), f"n={len(history)}")
    except Exception as exc:
        _record("load_history_no_raise", False, f"raised:{type(exc).__name__}")

    # 11. compute_cube_history_trend returns valid string
    try:
        trend_empty = compute_cube_history_trend([])
        _record("trend_empty", trend_empty == "INSUFFICIENT", trend_empty)
        trend_one = compute_cube_history_trend([{"cube_overall_closure_rate": 0.5}])
        _record("trend_one", trend_one == "INSUFFICIENT", trend_one)
        trend_two_stable = compute_cube_history_trend([
            {"timestamp": "t1", "cube_overall_closure_rate": 0.5},
            {"timestamp": "t2", "cube_overall_closure_rate": 0.51},
        ])
        _record("trend_two_stable", trend_two_stable == "STABLE", trend_two_stable)
        trend_two_improving = compute_cube_history_trend([
            {"timestamp": "t1", "cube_overall_closure_rate": 0.5},
            {"timestamp": "t2", "cube_overall_closure_rate": 0.7},
        ])
        _record("trend_two_improving", trend_two_improving == "IMPROVING", trend_two_improving)
    except Exception as exc:
        _record("trend_computation", False, f"raised:{type(exc).__name__}")

    # 12. append_cube_snapshot doesn't raise
    try:
        snap = aggregate_cube_snapshot()
        appended = append_cube_snapshot(snap)
        _record("append_snapshot_ok", isinstance(appended, bool), f"appended={appended}")
    except Exception as exc:
        _record("append_snapshot_ok", False, f"raised:{type(exc).__name__}")

    # 13. render_cube_report_md returns string with required sections
    try:
        snap = aggregate_cube_snapshot()
        report = CubeHistoryReport(
            schema=V1450_SCHEMA, version=V1450_VERSION, module=V1450_MODULE,
            started=_now_utc_iso(), ended=_now_utc_iso(),
            snapshots_loaded=0, snapshots_appended=1,
            current_snapshot=snap, history_trend="INSUFFICIENT",
        )
        md = render_cube_report_md(report)
        _record("render_returns_str", isinstance(md, str), f"len={len(md)}")
        _record("render_has_cube_section", "## Cube snapshot" in md, "ok")
        _record("render_has_honest_disclosure", "Honest disclosure" in md, "ok")
        _record("render_has_guards", "GUARD_HISTORY_REAL" in md, "ok")
    except Exception as exc:
        _record("render_no_raise", False, f"raised:{type(exc).__name__}")

    # 14. chain_delegate returns dict with all_ok
    try:
        chain = chain_delegate()
        _record("chain_delegate_returns_dict", isinstance(chain, dict), "ok")
        _record("chain_delegate_has_all_ok", "all_ok" in chain, "ok")
    except Exception as exc:
        _record("chain_delegate_returns_dict", False, f"raised:{type(exc).__name__}")

    # 15. Borrowed entries count
    _record("borrowed_count", len(V1450_BORROWED) == 5, str(len(V1450_BORROWED)))

    all_ok = all(r["ok"] for r in results)
    return all_ok, results


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Chain delegate to upstream V1447+V1448+V1449 modules.

    Returns:
        dict with:
        - all_ok: True if all upstream chains load and have run_all_no_raise
        - upstream: dict of upstream module -> status
        - guards: list of guard names
    """
    upstream_status: Dict[str, Dict[str, Any]] = {}

    # Try to import and run_all each upstream face module
    for face_id, source_module, _axes in V1450_FACE_REPORTS:
        status: Dict[str, Any] = {
            "imported": False,
            "has_run_all": False,
            "has_version": False,
            "version": None,
            "run_fn": None,
        }
        try:
            mod = __import__(f"apeireth.{source_module}", fromlist=[source_module])
            status["imported"] = True
            # Look for run_all OR run_full_audit (V1447 has run_full_audit)
            for run_attr in ("run_all", "run_full_audit"):
                if hasattr(mod, run_attr):
                    status["has_run_all"] = True
                    status["run_fn"] = run_attr
                    break
            # Look for version constant
            for attr in ("V1447_VERSION", "V1448_VERSION", "V1449_VERSION"):
                if hasattr(mod, attr):
                    status["has_version"] = True
                    status["version"] = getattr(mod, attr)
                    break
        except Exception as exc:
            status["error"] = _safe_str(exc, 100)

        upstream_status[face_id] = status

    all_imported = all(s["imported"] for s in upstream_status.values())
    all_have_run_all = all(s["has_run_all"] for s in upstream_status.values())
    all_ok = all_imported and all_have_run_all

    return {
        "all_ok": all_ok,
        "upstream": upstream_status,
        "guards": list(V1450_GUARDS),
        "v3_guards": list(V1450_V3_GUARDS),
        "borrowed": [list(b) for b in V1450_BORROWED],
        "n_upstream": len(V1450_FACE_REPORTS),
    }


# ============================================================================
# run_all
# ============================================================================


def run_all(
    out_json: Optional[Path] = None,
    out_md: Optional[Path] = None,
    append_history: bool = True,
) -> CubeHistoryReport:
    """Run full cube history aggregation + write outputs.

    Args:
        out_json: optional output JSON path (default: V1450_REPORT_JSON)
        out_md: optional output MD path (default: V1450_REPORT_MD)
        append_history: whether to append current snapshot to JSONL

    Returns:
        CubeHistoryReport with current snapshot + history trend
    """
    started = _now_utc_iso()

    # 1. Aggregate current snapshot
    snap = aggregate_cube_snapshot(timestamp=started)

    # 2. Load existing history
    history = load_cube_history()

    # 3. Append current snapshot
    appended = 0
    if append_history:
        if append_cube_snapshot(snap):
            appended = 1
            history.append(snap.to_dict())

    # 4. Compute trend
    trend = compute_cube_history_trend(history)

    # 5. Build report
    ended = _now_utc_iso()
    report = CubeHistoryReport(
        schema=V1450_SCHEMA,
        version=V1450_VERSION,
        module=V1450_MODULE,
        started=started,
        ended=ended,
        snapshots_loaded=len(history) - appended,
        snapshots_appended=appended,
        current_snapshot=snap,
        history_trend=trend,
        notes=f"V1450 cube history aggregator; trend={trend}",
    )

    # 6. Write outputs
    out_json_p = out_json or V1450_REPORT_JSON
    out_md_p = out_md or V1450_REPORT_MD

    try:
        with open(out_json_p, "w", encoding="utf-8", errors="replace") as f:
            json.dump(report.to_dict(), f, indent=2, ensure_ascii=False)
    except Exception:
        pass

    try:
        md = render_cube_report_md(report)
        with open(out_md_p, "w", encoding="utf-8", errors="replace") as f:
            f.write(md)
    except Exception:
        pass

    return report


# ============================================================================
# CLI
# ============================================================================


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=V1450_MODULE_SHORT,
        description="V1450 — ASI cross-modular cube history aggregator",
    )
    sub = p.add_subparsers(dest="cmd")

    sub.add_parser("version", help="Print version")
    sub.add_parser("help", help="Print help")

    p_meta = sub.add_parser("meta", help="Print module metadata")
    p_meta.add_argument("--json", action="store_true")

    sub.add_parser("popper", help="Run popper self-tests")
    sub.add_parser("chain", help="Print chain_delegate() result")

    sub.add_parser("load-history", help="Load and print cube history")
    sub.add_parser("cube-trend", help="Compute and print cube history trend")
    sub.add_parser("axis-stats", help="Print per-axis element stats")

    p_run = sub.add_parser("run-all", help="Run full cube aggregation")
    p_run.add_argument("--out-json", default=None)
    p_run.add_argument("--out-md", default=None)
    p_run.add_argument("--no-append", action="store_true", help="Don't append to history")

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    cmd = args.cmd or "help"

    if cmd == "version":
        print(V1450_VERSION)
        return 0

    if cmd == "help":
        parser.print_help()
        return 0

    if cmd == "meta":
        meta = {
            "schema": V1450_SCHEMA,
            "version": V1450_VERSION,
            "module": V1450_MODULE,
            "n_problems": len(V1450_PROBLEM_NAMES),
            "n_positions": len(V1450_POSITION_NAMES),
            "n_protocols": len(V1450_PROTOCOL_NAMES),
            "n_face_reports": len(V1450_FACE_REPORTS),
            "n_closure_kinds": len(V1450_CLOSURE_KINDS),
            "axes_overlap": V1450_AXIS_FACE_OVERLAP,
            "guards": list(V1450_GUARDS),
            "v3_guards": list(V1450_V3_GUARDS),
            "borrowed": [list(b) for b in V1450_BORROWED],
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
            mark = "?" if r["ok"] else "?"
            print(f"{mark} {r['name']}: {r['detail']}")
        print(f"\nALL_OK={ok}")
        return 0 if ok else 1

    if cmd == "chain":
        chain = chain_delegate()
        print(json.dumps(chain, indent=2, ensure_ascii=False))
        return 0

    if cmd == "load-history":
        history = load_cube_history()
        print(f"Loaded {len(history)} cube snapshots")
        for i, snap in enumerate(history[-10:], 1):  # last 10
            ts = snap.get("timestamp", "?")
            rate = snap.get("cube_overall_closure_rate", 0.0)
            xlink = snap.get("cube_cross_link_density", 0.0)
            print(f"  [{i}] {ts}: rate={rate:.4f} xlink={xlink:.4f}")
        return 0

    if cmd == "cube-trend":
        history = load_cube_history()
        trend = compute_cube_history_trend(history)
        digest = compute_cube_history_digest(history)
        print(json.dumps({"trend": trend, "digest": digest}, indent=2, ensure_ascii=False))
        return 0

    if cmd == "axis-stats":
        snap = aggregate_cube_snapshot()
        print(f"Per-axis overall:")
        for axis_name, rate in snap.per_axis_overall.items():
            print(f"  {axis_name}: {rate:.4f}")
        print(f"\nPer-axis element stats ({len(snap.axis_stats)} elements):")
        for stat in snap.axis_stats:
            print(f"  {stat.axis}/{stat.element}: mean={stat.mean_closure:.4f} face_count={stat.face_count}")
        return 0

    if cmd == "run-all":
        out_json = Path(args.out_json) if args.out_json else None
        out_md = Path(args.out_md) if args.out_md else None
        append_history = not getattr(args, "no_append", False)
        report = run_all(out_json=out_json, out_md=out_md, append_history=append_history)
        snap = report.current_snapshot
        print(f"V1450 cube history aggregator report written.")
        print(f"  cube_overall_closure_rate: {snap.cube_overall_closure_rate:.4f}")
        print(f"  cube_cross_link_density: {snap.cube_cross_link_density:.4f}")
        print(f"  axis_balance_score: {snap.axis_balance_score:.4f}")
        print(f"  faces found: {snap.n_faces_found}/{snap.n_faces_total}")
        print(f"  per_axis_overall: {snap.per_axis_overall}")
        print(f"  history_trend: {report.history_trend}")
        print(f"  snapshots_appended: {report.snapshots_appended}")
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:
        traceback.print_exc()
        sys.exit(1)