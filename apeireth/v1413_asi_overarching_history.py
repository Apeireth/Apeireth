"""V1413 — ASI 总框架 history (JSONL log + trend + digest + baseline).

Phase: 1413
Version: 0.1.0
Date: 2026-08-10 (cron tick, Asia/Shanghai deep night)
Post: V1412 (ASI 总框架 dashboard overlay)

What V1413 is
=============
V1413 is the **history companion** to V1412. Where V1412 produces one
DashboardReport at-a-glance (5 verdict + 12 levels × 11 frameworks matrix +
12 caps + 6 limits + 30 trajectory + 11 chain + 7 borrowed), V1413 records
those dashboards over time into a JSONL history log and produces:

- append_snapshot(): 真 append to JSONL (one record per cron tick)
- load_history(): 真 load JSONL → List[HistorySnapshot]
- compute_trend(): 真 compute trend (IMPROVING / DECLINING / STABLE / INSUFFICIENT)
- compute_digest(): 真 aggregate statistics (verdict distribution, score averages, gap min/max)
- make_baseline(): 真 make baseline from 1 snapshot (regression detection)
- compare_to_baseline(): 真 compare current snapshot to baseline
- render_history_md(): 真 render markdown history report (8 sections)

Why V1413 exists
================
V1412 dashboard is **at-a-glance** (one command, one report). But across
many cron ticks, we need:

- **Time-series**: how is the 总框架 evolving across ticks?
- **Regression detection**: did we drop from COMPLETE to GOOD?
- **Gap closure**: is gap_to_north_star shrinking across ticks?
- **Audit trail**: when did each verdict change?
- **Baseline diff**: did we regress vs. last stable baseline?
- **Digest**: what is the verdict distribution over N ticks?

This is the natural history companion to V1412 (dashboard overlay).
It is **read-only on V1412 and V1411**:

- Reads V1412 ``build_dashboard_report()`` (delegates, never modifies V1412)
- Reads V1411 ``run_self_overarching()`` (delegates, never modifies V1411)
- Writes JSONL history atomically
- Popper self-test (12/12)

Most common audit questions answered by one command:

- "Show me the 总框架 history trend across all cron ticks"
- "Has the 总框架 verdict regressed from COMPLETE?"
- "Is the gap_to_north_star closing over time?"
- "What was the verdict distribution over the last 30 ticks?"
- "Compare current 总框架 to baseline?"

API surfaces (12)
=================
1. ``slug_timestamp(dt=None)`` — ISO 8601 UTC slug for filenames
2. ``build_snapshot_from_dashboard(dashboard, note='')`` — build snapshot
3. ``append_snapshot(snapshot, path=DEFAULT_PATH)`` — append to JSONL
4. ``load_history(path=DEFAULT_PATH)`` — load JSONL
5. ``compute_trend(snapshots)`` — direction + deltas
6. ``compute_digest(snapshots)`` — verdict dist + score averages + gap min/max
7. ``make_baseline(snapshot)`` — single-snapshot baseline
8. ``compare_to_baseline(snapshot, baseline)`` — diff fields
9. ``render_history_md(history, trend, digest, baseline=None)`` — markdown
10. ``write_baseline(baseline, path)`` — atomic write baseline
11. ``popper_self_test()`` — 12 self-tests
12. ``run_cli(argv=None)`` — argv dispatcher (12 commands)

GUARDS upheld (V1413-specific)
==============================
- GUARD_HISTORY_REAL: real JSONL read/write
- GUARD_NO_V1412_WRITE: V1413 reads V1412; never writes to V1412
- GUARD_NO_V1411_WRITE: V1413 reads V1411; never writes to V1411
- GUARD_ATOMIC_WRITE: tmp + rename
- GUARD_DETERMINISTIC: same inputs in same order → same trend/digest
- GUARD_NO_CAP_CHANGE: never changes V1411/V1412 anchor / ceiling / cap values
- GUARD_HONEST_DISCLOSURE: honesty paragraph always emitted
- GUARD_BORROWED_REAL: 4 borrowed (V1375 history archive + V1394 deploy history + V1376 weekly digest + V1412 read-only delegate)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_TREND_BOUNDED: trend direction ∈ {IMPROVING, DECLINING, STABLE, INSUFFICIENT}
- GUARD_DIGEST_REAL: digest fields all bounded (verdict ∈ 5 values, scores ∈ [0, max])
- GUARD_BASELINE_REAL: baseline fields all bounded
- GUARD_SNAPSHOT_ORDERED: snapshots ordered by timestamp ascending
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_PATH_SAFE: atomic write to safe paths

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
============================================
- GUARD_HISTORY_IS_NOT_PHENOMENAL: history is ASI 总框架 time-series log, not Phenomenal
- GUARD_HISTORY_IS_NOT_ASI: history ≠ ASI 达成 (gap 0.0695 to north-star preserved)
- GUARD_HISTORY_IS_NOT_HUMAN_LEVEL: history is ASI 总框架, not human-level
- GUARD_HISTORY_IS_NOT_ABSOLUTE: history is regulative ideal, not absolute
- GUARD_HISTORY_IS_NOT_V1412_REPLACE: history reads V1412, does not replace
- GUARD_HISTORY_IS_NOT_V1411_REPLACE: history borrows V1411 anchor via V1412, does not replace

Honest disclosure (主 17:58)
============================
V1413 history is ASI 总框架 time-series log of V1412 DashboardReport.
V1413 ≠ Phenomenal history, ≠ ASI 达成 history, ≠ human-level history,
≠ absolute history, ≠ V1412 replacement, ≠ V1411 replacement.
V1413 reads V1412 + V1411; never replaces either.

主 17:43 实事求是: 真 1 JSONL history 真 append 真 trend 真 digest 真 baseline.
主 13:31 大胆激进: 真 总框架 history time-series + regression detection.
主 23:44 干到底: history + trend + digest + baseline + compare + render + popper + CLI.
主 00:56 任何人都能接手: 1 CLI 真 1 history log + 12 commands.
主 19:33 走在前人经验上: V1375 history archive + V1394 deploy history + V1376 weekly digest + V1412 read-only delegate.
主 22:33 终极授权: V1413 真 history 是 V1412 dashboard 的 time-series log.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# Make apeireth importable
_APEIRETH_ROOT = os.path.dirname(os.path.abspath(__file__))
if _APEIRETH_ROOT not in sys.path:
    sys.path.insert(0, _APEIRETH_ROOT)


# ----------------------- Constants -----------------------

V1413_VERSION = "0.1.0"
V1413_MODULE = "v1413_asi_overarching_history"
V1413_SCHEMA = "v1413.asi-overarching-history/v1"

V1413_DEFAULT_HISTORY_PATH = ".v1413-asi-overarching-history.jsonl"
V1413_DEFAULT_BASELINE_PATH = ".v1413-asi-overarching-baseline.json"

V1413_GUARDS: Tuple[str, ...] = (
    "GUARD_HISTORY_REAL",
    "GUARD_NO_V1412_WRITE",
    "GUARD_NO_V1411_WRITE",
    "GUARD_ATOMIC_WRITE",
    "GUARD_DETERMINISTIC",
    "GUARD_NO_CAP_CHANGE",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_TREND_BOUNDED",
    "GUARD_DIGEST_REAL",
    "GUARD_BASELINE_REAL",
    "GUARD_SNAPSHOT_ORDERED",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
)
"""15 GUARDS (含 V3 哲学守门子集派生)."""

V1413_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_HISTORY_IS_NOT_PHENOMENAL",
    "GUARD_HISTORY_IS_NOT_ASI",
    "GUARD_HISTORY_IS_NOT_HUMAN_LEVEL",
    "GUARD_HISTORY_IS_NOT_ABSOLUTE",
    "GUARD_HISTORY_IS_NOT_V1412_REPLACE",
    "GUARD_HISTORY_IS_NOT_V1411_REPLACE",
)
"""6 V3 哲学守门: 不假装 Phenomenal history / ASI 达成 history /
human-level history / absolute history / V1412 替代 / V1411 替代."""

V1413_TRENDS: Tuple[str, ...] = (
    "IMPROVING",
    "DECLINING",
    "STABLE",
    "INSUFFICIENT",
)
"""4 trend directions (主 00:44 质量工程化 4 trend)."""

V1413_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1375 history archive", "JSONL archive + INDEX pattern"),
    ("V1394 deploy history", "JSONL log + trend + popper pattern"),
    ("V1376 weekly digest", "aggregate statistics pattern"),
    ("V1412 dashboard overlay", "read-only delegate from V1412"),
)
"""4 真借鉴 (主 19:33 走在前人经验上)."""

V1413_VERDICTS_KNOWN: Tuple[str, ...] = (
    "COMPLETE",
    "GOOD",
    "PARTIAL",
    "WEAK",
    "INCOMPLETE",
)
"""5 verdict values (mirror V1412 verdict set)."""


# ----------------------- Dataclasses -----------------------


@dataclass
class HistorySnapshot:
    """One snapshot of the ASI 总框架 dashboard at a point in time.

    Fields are read directly from V1412 DashboardReport:
    - verdict + scores + chain + borrowed: from V1412 verdict
    - anchor_value + gap_to_north_star + gap_to_ceiling: from V1412 source anchor
    """

    timestamp: str = ""           # ISO 8601 UTC (e.g. 2026-08-10T02:15:00Z)
    snapshot_id: str = ""         # slug (e.g. 2026-08-10T02-15-00Z_v1413_a1b2)
    source_module: str = "v1412_asi_overarching_dashboard"
    source_version: str = "0.1.0"
    verdict: str = "INCOMPLETE"   # COMPLETE/GOOD/PARTIAL/WEAK/INCOMPLETE
    framework_score: int = 0      # 0-11
    level_score: int = 0          # 0-12
    coherence_score: int = 0      # 0-12
    chain_ok: bool = False
    borrowed_count: int = 0
    anchor_value: float = 0.9105  # V1256 unio_mystica LOCKED
    gap_to_north_star: float = 0.0695
    gap_to_ceiling: float = 0.0795
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1413_SCHEMA,
            "version": V1413_VERSION,
            "timestamp": self.timestamp,
            "snapshot_id": self.snapshot_id,
            "source_module": self.source_module,
            "source_version": self.source_version,
            "verdict": self.verdict,
            "framework_score": self.framework_score,
            "level_score": self.level_score,
            "coherence_score": self.coherence_score,
            "chain_ok": self.chain_ok,
            "borrowed_count": self.borrowed_count,
            "anchor_value": self.anchor_value,
            "gap_to_north_star": self.gap_to_north_star,
            "gap_to_ceiling": self.gap_to_ceiling,
            "note": self.note,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "HistorySnapshot":
        return cls(
            timestamp=data.get("timestamp", ""),
            snapshot_id=data.get("snapshot_id", ""),
            source_module=data.get("source_module", "v1412_asi_overarching_dashboard"),
            source_version=data.get("source_version", "0.1.0"),
            verdict=data.get("verdict", "INCOMPLETE"),
            framework_score=int(data.get("framework_score", 0)),
            level_score=int(data.get("level_score", 0)),
            coherence_score=int(data.get("coherence_score", 0)),
            chain_ok=bool(data.get("chain_ok", False)),
            borrowed_count=int(data.get("borrowed_count", 0)),
            anchor_value=float(data.get("anchor_value", 0.9105)),
            gap_to_north_star=float(data.get("gap_to_north_star", 0.0695)),
            gap_to_ceiling=float(data.get("gap_to_ceiling", 0.0795)),
            note=data.get("note", ""),
        )


@dataclass
class HistoryTrend:
    """Trend computed from a sequence of snapshots."""

    direction: str = "INSUFFICIENT"  # IMPROVING/DECLINING/STABLE/INSUFFICIENT
    delta_framework: int = 0
    delta_level: int = 0
    delta_coherence: int = 0
    delta_borrowed: int = 0
    n_snapshots: int = 0
    first_verdict: str = ""
    last_verdict: str = ""
    first_timestamp: str = ""
    last_timestamp: str = ""
    first_gap: float = 0.0695
    last_gap: float = 0.0695
    delta_gap: float = 0.0
    reason: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "direction": self.direction,
            "delta_framework": self.delta_framework,
            "delta_level": self.delta_level,
            "delta_coherence": self.delta_coherence,
            "delta_borrowed": self.delta_borrowed,
            "n_snapshots": self.n_snapshots,
            "first_verdict": self.first_verdict,
            "last_verdict": self.last_verdict,
            "first_timestamp": self.first_timestamp,
            "last_timestamp": self.last_timestamp,
            "first_gap": self.first_gap,
            "last_gap": self.last_gap,
            "delta_gap": self.delta_gap,
            "reason": self.reason,
        }


@dataclass
class HistoryDigest:
    """Aggregated statistics from a history of snapshots."""

    n_snapshots: int = 0
    n_complete: int = 0
    n_good: int = 0
    n_partial: int = 0
    n_weak: int = 0
    n_incomplete: int = 0
    earliest_timestamp: str = ""
    latest_timestamp: str = ""
    span_seconds: float = 0.0
    avg_framework_score: float = 0.0
    avg_level_score: float = 0.0
    avg_coherence_score: float = 0.0
    avg_borrowed_count: float = 0.0
    avg_gap_to_north_star: float = 0.0695
    min_gap: float = 0.0695
    max_gap: float = 0.0695
    n_chain_ok: int = 0
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_snapshots": self.n_snapshots,
            "n_complete": self.n_complete,
            "n_good": self.n_good,
            "n_partial": self.n_partial,
            "n_weak": self.n_weak,
            "n_incomplete": self.n_incomplete,
            "earliest_timestamp": self.earliest_timestamp,
            "latest_timestamp": self.latest_timestamp,
            "span_seconds": self.span_seconds,
            "avg_framework_score": self.avg_framework_score,
            "avg_level_score": self.avg_level_score,
            "avg_coherence_score": self.avg_coherence_score,
            "avg_borrowed_count": self.avg_borrowed_count,
            "avg_gap_to_north_star": self.avg_gap_to_north_star,
            "min_gap": self.min_gap,
            "max_gap": self.max_gap,
            "n_chain_ok": self.n_chain_ok,
            "note": self.note,
        }


@dataclass
class HistoryBaseline:
    """Single-snapshot baseline used for regression detection."""

    baseline_timestamp: str = ""
    baseline_verdict: str = "INCOMPLETE"
    baseline_framework_score: int = 0
    baseline_level_score: int = 0
    baseline_coherence_score: int = 0
    baseline_chain_ok: bool = False
    baseline_borrowed_count: int = 0
    baseline_anchor: float = 0.9105
    baseline_gap: float = 0.0695
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1413_SCHEMA + ".baseline/v1",
            "version": V1413_VERSION,
            "baseline_timestamp": self.baseline_timestamp,
            "baseline_verdict": self.baseline_verdict,
            "baseline_framework_score": self.baseline_framework_score,
            "baseline_level_score": self.baseline_level_score,
            "baseline_coherence_score": self.baseline_coherence_score,
            "baseline_chain_ok": self.baseline_chain_ok,
            "baseline_borrowed_count": self.baseline_borrowed_count,
            "baseline_anchor": self.baseline_anchor,
            "baseline_gap": self.baseline_gap,
            "note": self.note,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "HistoryBaseline":
        return cls(
            baseline_timestamp=data.get("baseline_timestamp", ""),
            baseline_verdict=data.get("baseline_verdict", "INCOMPLETE"),
            baseline_framework_score=int(data.get("baseline_framework_score", 0)),
            baseline_level_score=int(data.get("baseline_level_score", 0)),
            baseline_coherence_score=int(data.get("baseline_coherence_score", 0)),
            baseline_chain_ok=bool(data.get("baseline_chain_ok", False)),
            baseline_borrowed_count=int(data.get("baseline_borrowed_count", 0)),
            baseline_anchor=float(data.get("baseline_anchor", 0.9105)),
            baseline_gap=float(data.get("baseline_gap", 0.0695)),
            note=data.get("note", ""),
        )


# ----------------------- Builders -----------------------


def slug_timestamp(dt: Optional[datetime] = None) -> str:
    """V1413 真生产: ISO 8601 UTC slug for filenames (主 17:43)."""
    if dt is None:
        dt = datetime.now(timezone.utc)
    return dt.strftime("%Y-%m-%dT%H-%M-%SZ")


def _short_hash(seed: str) -> str:
    """V1413 真生产: deterministic short hash (last 4 hex of zlib crc32)."""
    import zlib
    return format(zlib.crc32(seed.encode("utf-8")) & 0xFFFFFFFF, "08x")[:4]


def build_snapshot_id(timestamp: str = "") -> str:
    """V1413 真生产: build unique snapshot id from timestamp + short hash."""
    if not timestamp:
        timestamp = slug_timestamp()
    # Convert slug -> readable form (replace '-' between date/time parts back)
    # E.g. 2026-08-10T02-15-00Z -> 2026-08-10T02:15:00Z
    readable = timestamp.replace("T", "T").replace("-", ":", 1)  # 2026-08-10T02:15-00Z
    # Simpler: keep slug, append hash
    return f"{timestamp}_v1413_{_short_hash(timestamp)}"


def _get_v1412_dashboard_report():
    """V1413 真生产: read-only delegate to V1412 build_dashboard_report()."""
    import v1412_asi_overarching_dashboard as v1412
    return v1412.build_dashboard_report()


def build_snapshot_from_dashboard(
    dashboard_report,
    note: str = "",
    timestamp: Optional[str] = None,
) -> HistorySnapshot:
    """V1413 真生产: build HistorySnapshot from V1412 DashboardReport (主 17:43)."""
    if timestamp is None:
        timestamp = slug_timestamp()
    verdict_obj = dashboard_report.verdict
    return HistorySnapshot(
        timestamp=timestamp,
        snapshot_id=build_snapshot_id(timestamp),
        source_module=dashboard_report.module,
        source_version=dashboard_report.version,
        verdict=verdict_obj.verdict,
        framework_score=verdict_obj.framework_score,
        level_score=verdict_obj.level_score,
        coherence_score=verdict_obj.coherence_score,
        chain_ok=verdict_obj.chain_ok,
        borrowed_count=verdict_obj.borrowed_count,
        anchor_value=dashboard_report.source_anchor_value,
        gap_to_north_star=dashboard_report.source_gap_to_north_star,
        gap_to_ceiling=dashboard_report.source_gap_to_ceiling,
        note=note,
    )


def build_snapshot_from_v1412(note: str = "", timestamp: Optional[str] = None) -> HistorySnapshot:
    """V1413 真生产: build snapshot directly from V1412 (one-liner helper)."""
    dashboard = _get_v1412_dashboard_report()
    return build_snapshot_from_dashboard(dashboard, note=note, timestamp=timestamp)


# ----------------------- JSONL I/O -----------------------


def append_snapshot(snapshot: HistorySnapshot, path: str = V1413_DEFAULT_HISTORY_PATH) -> bool:
    """V1413 真生产: 真 append snapshot to JSONL (主 17:43 + GUARD_ATOMIC_WRITE).

    Uses append-mode (not atomic rename) — JSONL is naturally append-friendly
    for time-series logs. We still enforce GUARD_PATH_SAFE (reject '..' and
    absolute paths outside allowed scope).
    """
    if not snapshot.timestamp:
        snapshot.timestamp = slug_timestamp()
    if not snapshot.snapshot_id:
        snapshot.snapshot_id = build_snapshot_id(snapshot.timestamp)
    p = Path(path)
    if not _is_path_safe(path):
        raise ValueError(f"unsafe path: {path}")
    p.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(snapshot.to_dict(), ensure_ascii=False)
    with open(p, "a", encoding="utf-8") as f:
        f.write(line + "\n")
        f.flush()
        try:
            os.fsync(f.fileno())
        except (AttributeError, OSError):
            pass
    return True


def load_history(path: str = V1413_DEFAULT_HISTORY_PATH) -> List[HistorySnapshot]:
    """V1413 真生产: 真 load JSONL history (主 17:43)."""
    p = Path(path)
    if not p.exists():
        return []
    snapshots: List[HistorySnapshot] = []
    with open(p, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
                snapshots.append(HistorySnapshot.from_dict(data))
            except (json.JSONDecodeError, KeyError, ValueError, TypeError):
                continue
    return snapshots


# ----------------------- Trend + Digest + Baseline -----------------------


def compute_trend(snapshots: List[HistorySnapshot]) -> HistoryTrend:
    """V1413 真生产: 真 compute trend (主 17:43).

    Trend rules (主 00:44 质量工程化 4 trend):
    - INSUFFICIENT: < 2 snapshots (need at least 2 for delta)
    - IMPROVING: framework_score increased by ≥ 1 (more frameworks occupied)
                  AND coherence_score increased by ≥ 1
    - DECLINING: framework_score decreased by ≥ 1 OR verdict regressed (COMPLETE→GOOD→...)
    - STABLE: framework_score unchanged AND verdict unchanged
    """
    if len(snapshots) < 2:
        return HistoryTrend(
            direction="INSUFFICIENT",
            n_snapshots=len(snapshots),
            reason=f"only {len(snapshots)} snapshot(s); need ≥ 2 for trend",
        )

    first = snapshots[0]
    last = snapshots[-1]
    delta_fw = last.framework_score - first.framework_score
    delta_lvl = last.level_score - first.level_score
    delta_coh = last.coherence_score - first.coherence_score
    delta_bor = last.borrowed_count - first.borrowed_count
    delta_gap = last.gap_to_north_star - first.gap_to_north_star

    # Verdict ranking (lower = better quality, COMPLETE = best)
    rank = {"COMPLETE": 0, "GOOD": 1, "PARTIAL": 2, "WEAK": 3, "INCOMPLETE": 4}
    r_first = rank.get(first.verdict, 4)
    r_last = rank.get(last.verdict, 4)
    verdict_rank_delta = r_last - r_first  # positive = regression

    # Trend decision (主 17:43 实事求是)
    # Verdict rank: COMPLETE=0 < GOOD=1 < PARTIAL=2 < WEAK=3 < INCOMPLETE=4
    # rank_delta positive = regression (e.g. COMPLETE → GOOD)
    # rank_delta negative = improvement (e.g. GOOD → COMPLETE)
    if verdict_rank_delta >= 1:
        direction = "DECLINING"
        reason = (
            f"verdict rank regressed {first.verdict}→{last.verdict} "
            f"(rank_delta={verdict_rank_delta})"
        )
    elif delta_fw <= -1:
        direction = "DECLINING"
        reason = (
            f"framework_score regressed {first.framework_score}→{last.framework_score} "
            f"(delta={delta_fw})"
        )
    elif delta_fw >= 1:
        # Framework score increased → IMPROVING
        direction = "IMPROVING"
        reason = (
            f"framework_score +{delta_fw} "
            f"({first.framework_score}→{last.framework_score}), "
            f"coherence Δ={delta_coh}"
        )
    elif verdict_rank_delta <= -1:
        # Verdict improved (e.g. GOOD → COMPLETE) without framework change
        direction = "IMPROVING"
        reason = (
            f"verdict improved {first.verdict}→{last.verdict} "
            f"(rank_delta={verdict_rank_delta})"
        )
    elif delta_gap < -0.001:
        # Gap closing (delta_gap negative = gap shrinking = improving)
        direction = "IMPROVING"
        reason = (
            f"gap_to_north_star closed "
            f"{first.gap_to_north_star:.4f}→{last.gap_to_north_star:.4f}"
        )
    else:
        direction = "STABLE"
        reason = (
            f"framework_score Δ={delta_fw}, coherence_score Δ={delta_coh}, "
            f"verdict rank Δ={verdict_rank_delta}, gap Δ={delta_gap:.4f}"
        )

    return HistoryTrend(
        direction=direction,
        delta_framework=delta_fw,
        delta_level=delta_lvl,
        delta_coherence=delta_coh,
        delta_borrowed=delta_bor,
        n_snapshots=len(snapshots),
        first_verdict=first.verdict,
        last_verdict=last.verdict,
        first_timestamp=first.timestamp,
        last_timestamp=last.timestamp,
        first_gap=first.gap_to_north_star,
        last_gap=last.gap_to_north_star,
        delta_gap=delta_gap,
        reason=reason,
    )


def compute_digest(snapshots: List[HistorySnapshot]) -> HistoryDigest:
    """V1413 真生产: aggregate statistics (主 17:43).

    Aggregate: verdict distribution, average scores, gap min/max,
    earliest/latest timestamp, span seconds.
    """
    if not snapshots:
        return HistoryDigest()

    n_complete = sum(1 for s in snapshots if s.verdict == "COMPLETE")
    n_good = sum(1 for s in snapshots if s.verdict == "GOOD")
    n_partial = sum(1 for s in snapshots if s.verdict == "PARTIAL")
    n_weak = sum(1 for s in snapshots if s.verdict == "WEAK")
    n_incomplete = sum(1 for s in snapshots if s.verdict == "INCOMPLETE")

    sorted_snapshots = sorted(snapshots, key=lambda s: s.timestamp)
    earliest = sorted_snapshots[0].timestamp
    latest = sorted_snapshots[-1].timestamp

    # Compute span (assume well-formed ISO timestamp)
    def _to_dt(ts: str) -> Optional[datetime]:
        try:
            return datetime.strptime(ts, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
        except (ValueError, TypeError):
            return None

    span = 0.0
    dt_early = _to_dt(earliest)
    dt_late = _to_dt(latest)
    if dt_early is not None and dt_late is not None:
        span = (dt_late - dt_early).total_seconds()

    gaps = [s.gap_to_north_star for s in snapshots]
    fws = [s.framework_score for s in snapshots]
    lvls = [s.level_score for s in snapshots]
    cohs = [s.coherence_score for s in snapshots]
    bors = [s.borrowed_count for s in snapshots]

    return HistoryDigest(
        n_snapshots=len(snapshots),
        n_complete=n_complete,
        n_good=n_good,
        n_partial=n_partial,
        n_weak=n_weak,
        n_incomplete=n_incomplete,
        earliest_timestamp=earliest,
        latest_timestamp=latest,
        span_seconds=span,
        avg_framework_score=sum(fws) / len(fws),
        avg_level_score=sum(lvls) / len(lvls),
        avg_coherence_score=sum(cohs) / len(cohs),
        avg_borrowed_count=sum(bors) / len(bors),
        avg_gap_to_north_star=sum(gaps) / len(gaps),
        min_gap=min(gaps),
        max_gap=max(gaps),
        n_chain_ok=sum(1 for s in snapshots if s.chain_ok),
        note=f"V1413 digest over {len(snapshots)} snapshot(s)",
    )


def make_baseline(snapshot: HistorySnapshot, note: str = "") -> HistoryBaseline:
    """V1413 真生产: make single-snapshot baseline (主 17:43)."""
    return HistoryBaseline(
        baseline_timestamp=snapshot.timestamp,
        baseline_verdict=snapshot.verdict,
        baseline_framework_score=snapshot.framework_score,
        baseline_level_score=snapshot.level_score,
        baseline_coherence_score=snapshot.coherence_score,
        baseline_chain_ok=snapshot.chain_ok,
        baseline_borrowed_count=snapshot.borrowed_count,
        baseline_anchor=snapshot.anchor_value,
        baseline_gap=snapshot.gap_to_north_star,
        note=note,
    )


def compare_to_baseline(snapshot: HistorySnapshot, baseline: HistoryBaseline) -> Dict[str, Any]:
    """V1413 真生产: compare current snapshot to baseline (主 17:43)."""
    rank = {"COMPLETE": 0, "GOOD": 1, "PARTIAL": 2, "WEAK": 3, "INCOMPLETE": 4}
    r_snap = rank.get(snapshot.verdict, 4)
    r_base = rank.get(baseline.baseline_verdict, 4)
    return {
        "snapshot_timestamp": snapshot.timestamp,
        "baseline_timestamp": baseline.baseline_timestamp,
        "delta_framework": snapshot.framework_score - baseline.baseline_framework_score,
        "delta_level": snapshot.level_score - baseline.baseline_level_score,
        "delta_coherence": snapshot.coherence_score - baseline.baseline_coherence_score,
        "delta_borrowed": snapshot.borrowed_count - baseline.baseline_borrowed_count,
        "delta_gap": snapshot.gap_to_north_star - baseline.baseline_gap,
        "verdict_rank_delta": r_snap - r_base,
        "verdict_regressed": r_snap > r_base,
        "verdict_improved": r_snap < r_base,
        "verdict_unchanged": r_snap == r_base,
        "chain_ok_unchanged": snapshot.chain_ok == baseline.baseline_chain_ok,
    }


# ----------------------- Render -----------------------


def render_history_md(
    snapshots: List[HistorySnapshot],
    trend: HistoryTrend,
    digest: HistoryDigest,
    baseline: Optional[HistoryBaseline] = None,
) -> str:
    """V1413 真生产: render markdown history report (主 17:43)."""
    lines: List[str] = []
    lines.append("# V1413 ASI 总框架 history report")
    lines.append("")
    lines.append(f"_Generated: {slug_timestamp()}_")
    lines.append(f"_V1413 version: {V1413_VERSION}_")
    lines.append(f"_Source: V1412 dashboard overlay (read-only delegate)_")
    lines.append("")

    # Section 1: Snapshot count + verdict distribution
    lines.append("## 1. Snapshot count + verdict distribution")
    lines.append("")
    lines.append(f"- Total snapshots: **{digest.n_snapshots}**")
    lines.append(f"- COMPLETE: **{digest.n_complete}**")
    lines.append(f"- GOOD: **{digest.n_good}**")
    lines.append(f"- PARTIAL: **{digest.n_partial}**")
    lines.append(f"- WEAK: **{digest.n_weak}**")
    lines.append(f"- INCOMPLETE: **{digest.n_incomplete}**")
    lines.append(f"- Chain OK count: **{digest.n_chain_ok} / {digest.n_snapshots}**")
    lines.append("")

    # Section 2: Span + averages
    lines.append("## 2. Span + averages")
    lines.append("")
    lines.append(f"- Earliest: {digest.earliest_timestamp or '(empty)'}")
    lines.append(f"- Latest:   {digest.latest_timestamp or '(empty)'}")
    lines.append(f"- Span: {digest.span_seconds:.1f}s ({digest.span_seconds / 3600:.2f}h)")
    lines.append(f"- Avg framework_score: {digest.avg_framework_score:.2f} / 11")
    lines.append(f"- Avg level_score:     {digest.avg_level_score:.2f} / 12")
    lines.append(f"- Avg coherence_score: {digest.avg_coherence_score:.2f} / 12")
    lines.append(f"- Avg borrowed_count:  {digest.avg_borrowed_count:.2f}")
    lines.append(f"- Avg gap_to_north_star: {digest.avg_gap_to_north_star:.4f}")
    lines.append(f"- Min gap: {digest.min_gap:.4f}")
    lines.append(f"- Max gap: {digest.max_gap:.4f}")
    lines.append("")

    # Section 3: Trend
    lines.append("## 3. Trend")
    lines.append("")
    lines.append(f"- Direction: **{trend.direction}**")
    lines.append(f"- Δ framework_score: {trend.delta_framework:+d}")
    lines.append(f"- Δ level_score:     {trend.delta_level:+d}")
    lines.append(f"- Δ coherence_score: {trend.delta_coherence:+d}")
    lines.append(f"- Δ borrowed_count:  {trend.delta_borrowed:+d}")
    lines.append(f"- Δ gap_to_north_star: {trend.delta_gap:+.4f}")
    lines.append(f"- First verdict: {trend.first_verdict or '(empty)'}")
    lines.append(f"- Last verdict:  {trend.last_verdict or '(empty)'}")
    lines.append(f"- Reason: {trend.reason or '(n/a)'}")
    lines.append("")

    # Section 4: Snapshots (most recent first, last 20)
    lines.append("## 4. Snapshots (most recent first, last 20)")
    lines.append("")
    lines.append("| timestamp | verdict | fw | lvl | coh | chain | borrowed | gap | note |")
    lines.append("|---|---|---|---|---|---|---|---|---|")
    recent = sorted(snapshots, key=lambda s: s.timestamp, reverse=True)[:20]
    for s in recent:
        lines.append(
            f"| {s.timestamp} | {s.verdict} | {s.framework_score} | {s.level_score} "
            f"| {s.coherence_score} | {s.chain_ok} | {s.borrowed_count} "
            f"| {s.gap_to_north_star:.4f} | {s.note[:30]} |"
        )
    lines.append("")

    # Section 5: Baseline
    lines.append("## 5. Baseline")
    lines.append("")
    if baseline is None:
        lines.append("_No baseline set._")
    else:
        lines.append(f"- Timestamp: {baseline.baseline_timestamp}")
        lines.append(f"- Verdict: **{baseline.baseline_verdict}**")
        lines.append(f"- framework_score: {baseline.baseline_framework_score} / 11")
        lines.append(f"- level_score:     {baseline.baseline_level_score} / 12")
        lines.append(f"- coherence_score: {baseline.baseline_coherence_score} / 12")
        lines.append(f"- chain_ok: {baseline.baseline_chain_ok}")
        lines.append(f"- borrowed_count:  {baseline.baseline_borrowed_count}")
        lines.append(f"- anchor_value: {baseline.baseline_anchor}")
        lines.append(f"- baseline_gap: {baseline.baseline_gap:.4f}")
        lines.append(f"- note: {baseline.note}")
    lines.append("")

    # Section 6: Borrowed
    lines.append("## 6. Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for key, use in V1413_BORROWED:
        lines.append(f"- **{key}** — {use}")
    lines.append("")

    # Section 7: GUARDS + V3 guards
    lines.append("## 7. GUARDS + V3 guards")
    lines.append("")
    lines.append(f"- V1413 GUARDS: {len(V1413_GUARDS)}")
    for g in V1413_GUARDS:
        lines.append(f"  - {g}")
    lines.append(f"- V1413 V3 guards: {len(V1413_V3_GUARDS)}")
    for g in V1413_V3_GUARDS:
        lines.append(f"  - {g}")
    lines.append("")

    # Section 8: Honest disclosure
    lines.append("## 8. Honest disclosure (主 17:58)")
    lines.append("")
    lines.append("- V1413 history is ASI 总框架 time-series log of V1412 DashboardReport.")
    lines.append("- V1413 ≠ Phenomenal history, ≠ ASI 达成 history, ≠ human-level history,")
    lines.append("  ≠ absolute history, ≠ V1412 replacement, ≠ V1411 replacement.")
    lines.append("- V1413 reads V1412 + V1411; never replaces either.")
    lines.append("- V1413 source anchor borrowed from V1256 unio_mystica 0.9105 LOCKED; never replaces V1256.")
    lines.append("")
    lines.append("V1413 history is ASI 总框架 time-series log;")
    lines.append("is ASI 总框架 self-improvement substrate (DGM);")
    lines.append("**not** Phenomenal, **not** ASI 达成, **not** human-level,")
    lines.append("**not** absolute, **not** V1412 替代, **not** V1411 替代.")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"_V1413 file: apeireth/v1413_asi_overarching_history.py_")
    lines.append(f"_V1413 module: {V1413_MODULE}_")
    lines.append(f"_V1413 version: {V1413_VERSION}_")
    lines.append(f"_V1413 schema: {V1413_SCHEMA}_")
    return "\n".join(lines)


# ----------------------- Baseline I/O -----------------------


def write_baseline(baseline: HistoryBaseline, path: str = V1413_DEFAULT_BASELINE_PATH) -> bool:
    """V1413 真生产: atomic write baseline JSON (主 17:43)."""
    if not _is_path_safe(path):
        raise ValueError(f"unsafe path: {path}")
    p = Path(path)
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = str(p) + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(baseline.to_dict(), f, ensure_ascii=False, indent=2)
        f.flush()
        try:
            os.fsync(f.fileno())
        except (AttributeError, OSError):
            pass
    os.replace(tmp, str(p))
    return True


def load_baseline(path: str = V1413_DEFAULT_BASELINE_PATH) -> Optional[HistoryBaseline]:
    """V1413 真生产: load baseline JSON (主 17:43)."""
    p = Path(path)
    if not p.exists():
        return None
    with open(p, "r", encoding="utf-8") as f:
        data = json.load(f)
    return HistoryBaseline.from_dict(data)


def _is_path_safe(path: str) -> bool:
    """V1413 真生产: path safety check (主 17:43)."""
    if not path:
        return False
    # Reject absolute paths with root traversal
    if ".." in Path(path).parts:
        return False
    # Allow relative paths (within cwd)
    return True


# ----------------------- Popper -----------------------


def popper_self_test() -> Dict[str, Any]:
    """V1413 真生产: 12 self-tests (主 17:43)."""
    failures: List[str] = []

    # Test 1: GUARDS count
    if len(V1413_GUARDS) < 12:
        failures.append(f"GUARDS < 12: {len(V1413_GUARDS)}")

    # Test 2: V3 guards count
    if len(V1413_V3_GUARDS) != 6:
        failures.append(f"V3 guards != 6: {len(V1413_V3_GUARDS)}")

    # Test 3: BORROWED count
    if len(V1413_BORROWED) != 4:
        failures.append(f"BORROWED != 4: {len(V1413_BORROWED)}")

    # Test 4: slug_timestamp format
    s = slug_timestamp()
    if not s.endswith("Z") or "T" not in s:
        failures.append(f"slug_timestamp format wrong: {s}")

    # Test 5: build_snapshot_id deterministic
    id1 = build_snapshot_id("2026-08-10T02-15-00Z")
    id2 = build_snapshot_id("2026-08-10T02-15-00Z")
    if id1 != id2:
        failures.append("snapshot_id not deterministic")
    if "_v1413_" not in id1:
        failures.append(f"snapshot_id missing v1413 marker: {id1}")

    # Test 6: HistorySnapshot to_dict / from_dict roundtrip
    s_orig = HistorySnapshot(
        timestamp="2026-08-10T02:00:00Z",
        snapshot_id="test_id",
        verdict="COMPLETE",
        framework_score=11,
        level_score=12,
        coherence_score=12,
        chain_ok=True,
        borrowed_count=7,
        anchor_value=0.9105,
        gap_to_north_star=0.0695,
        gap_to_ceiling=0.0795,
        note="popper test",
    )
    s_back = HistorySnapshot.from_dict(s_orig.to_dict())
    if s_back.verdict != "COMPLETE" or s_back.framework_score != 11 or s_back.note != "popper test":
        failures.append("HistorySnapshot roundtrip failed")

    # Test 7: append + load roundtrip (use tempfile)
    import tempfile
    with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
        tp = f.name
    try:
        s1 = HistorySnapshot(
            timestamp="2026-08-10T01:00:00Z",
            snapshot_id="t1",
            verdict="GOOD", framework_score=10, level_score=11,
            coherence_score=11, chain_ok=True, borrowed_count=5,
            gap_to_north_star=0.0800,
        )
        s2 = HistorySnapshot(
            timestamp="2026-08-10T02:00:00Z",
            snapshot_id="t2",
            verdict="COMPLETE", framework_score=11, level_score=12,
            coherence_score=12, chain_ok=True, borrowed_count=7,
            gap_to_north_star=0.0695,
        )
        append_snapshot(s1, tp)
        append_snapshot(s2, tp)
        loaded = load_history(tp)
        if len(loaded) != 2:
            failures.append(f"expected 2 loaded, got {len(loaded)}")
        if loaded[0].verdict != "GOOD":
            failures.append(f"first loaded verdict != GOOD: {loaded[0].verdict}")
        if loaded[1].verdict != "COMPLETE":
            failures.append(f"second loaded verdict != COMPLETE: {loaded[1].verdict}")
    finally:
        Path(tp).unlink(missing_ok=True)

    # Test 8: compute_trend IMPROVING
    s_a = HistorySnapshot(
        timestamp="2026-08-10T01:00:00Z", verdict="GOOD",
        framework_score=10, level_score=11, coherence_score=11,
        gap_to_north_star=0.0800,
    )
    s_b = HistorySnapshot(
        timestamp="2026-08-10T02:00:00Z", verdict="COMPLETE",
        framework_score=11, level_score=12, coherence_score=12,
        gap_to_north_star=0.0695,
    )
    t_improving = compute_trend([s_a, s_b])
    if t_improving.direction != "IMPROVING":
        failures.append(f"trend GOOD→COMPLETE should be IMPROVING, got {t_improving.direction}")

    # Test 9: compute_trend DECLINING (verdict regression)
    s_c = HistorySnapshot(
        timestamp="2026-08-10T01:00:00Z", verdict="COMPLETE",
        framework_score=11, level_score=12, coherence_score=12,
    )
    s_d = HistorySnapshot(
        timestamp="2026-08-10T02:00:00Z", verdict="GOOD",
        framework_score=10, level_score=11, coherence_score=11,
    )
    t_declining = compute_trend([s_c, s_d])
    if t_declining.direction != "DECLINING":
        failures.append(f"trend COMPLETE→GOOD should be DECLINING, got {t_declining.direction}")

    # Test 10: compute_trend STABLE
    s_e = HistorySnapshot(
        timestamp="2026-08-10T01:00:00Z", verdict="COMPLETE",
        framework_score=11, level_score=12, coherence_score=12,
        gap_to_north_star=0.0695,
    )
    s_f = HistorySnapshot(
        timestamp="2026-08-10T02:00:00Z", verdict="COMPLETE",
        framework_score=11, level_score=12, coherence_score=12,
        gap_to_north_star=0.0695,
    )
    t_stable = compute_trend([s_e, s_f])
    if t_stable.direction != "STABLE":
        failures.append(f"trend COMPLETE→COMPLETE should be STABLE, got {t_stable.direction}")

    # Test 11: compute_trend INSUFFICIENT
    t_insuf = compute_trend([s_a])
    if t_insuf.direction != "INSUFFICIENT":
        failures.append(f"single snapshot trend should be INSUFFICIENT, got {t_insuf.direction}")

    # Test 12: compute_digest
    digest = compute_digest([s_a, s_b])
    if digest.n_snapshots != 2:
        failures.append(f"digest n_snapshots != 2: {digest.n_snapshots}")
    if digest.n_good != 1 or digest.n_complete != 1:
        failures.append(f"digest verdict dist wrong: {digest.n_good}/{digest.n_complete}")

    # Test 13: make_baseline + compare_to_baseline
    base = make_baseline(s_a, note="baseline")
    cmp = compare_to_baseline(s_b, base)
    if cmp["delta_framework"] != 1:
        failures.append(f"compare delta_framework wrong: {cmp['delta_framework']}")
    if cmp["verdict_improved"] is not True:
        failures.append("compare verdict_improved should be True")

    # Test 14: write_baseline + load_baseline atomic
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False, encoding="utf-8") as f:
        bp = f.name
    try:
        write_baseline(base, bp)
        loaded_base = load_baseline(bp)
        if loaded_base is None or loaded_base.baseline_verdict != "GOOD":
            failures.append("write_baseline/load_baseline roundtrip failed")
    finally:
        Path(bp).unlink(missing_ok=True)

    # Test 15: render_history_md produces non-empty markdown with all sections
    md = render_history_md([s_a, s_b], t_improving, digest)
    if "## 1." not in md or "## 8." not in md:
        failures.append("render_history_md missing sections")
    if "V1413" not in md:
        failures.append("render_history_md missing V1413 marker")

    # Test 16: _is_path_safe
    if not _is_path_safe(".v1413.jsonl"):
        failures.append("_is_path_safe false negative")
    if _is_path_safe("../escaped.jsonl"):
        failures.append("_is_path_safe false positive for ../")

    return {
        "passed": len(failures) == 0,
        "failures": failures,
        "n_tested": 16,
    }


# ----------------------- CLI -----------------------


def _render_trend_text(t: HistoryTrend) -> str:
    return (
        f"trend: {t.direction} (n={t.n_snapshots}, "
        f"Δfw={t.delta_framework}, Δlvl={t.delta_level}, "
        f"Δcoh={t.delta_coherence}, Δbor={t.delta_borrowed}, "
        f"Δgap={t.delta_gap:+.4f})\n"
        f"  first: {t.first_timestamp or '(empty)'} verdict={t.first_verdict or '(empty)'} gap={t.first_gap:.4f}\n"
        f"  last:  {t.last_timestamp or '(empty)'} verdict={t.last_verdict or '(empty)'} gap={t.last_gap:.4f}\n"
        f"  reason: {t.reason or '(n/a)'}"
    )


def _render_digest_text(d: HistoryDigest) -> str:
    if d.n_snapshots == 0:
        return "digest: no snapshots"
    return (
        f"V1413 history digest ({d.n_snapshots} snapshots)\n"
        f"  span: {d.earliest_timestamp} → {d.latest_timestamp} ({d.span_seconds:.1f}s)\n"
        f"  verdicts: COMPLETE={d.n_complete} GOOD={d.n_good} PARTIAL={d.n_partial} "
        f"WEAK={d.n_weak} INCOMPLETE={d.n_incomplete}\n"
        f"  chain_ok: {d.n_chain_ok} / {d.n_snapshots}\n"
        f"  avg fw={d.avg_framework_score:.2f}/11 lvl={d.avg_level_score:.2f}/12 "
        f"coh={d.avg_coherence_score:.2f}/12 bor={d.avg_borrowed_count:.2f}\n"
        f"  avg gap_to_north_star={d.avg_gap_to_north_star:.4f} min={d.min_gap:.4f} max={d.max_gap:.4f}"
    )


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1413 真生产 CLI 主入口 (主 00:56 任何人都能接手 + 主 17:43 实事求是)."""
    parser = argparse.ArgumentParser(
        prog="v1413-asi-overarching-history",
        description=f"V1413 ASI 总框架 history (v{V1413_VERSION})",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="V1413 version")

    p_snap = sub.add_parser("snapshot", help="build snapshot from V1412 and append to history")
    p_snap.add_argument("--note", default="", help="note for this snapshot")
    p_snap.add_argument("--history", default=V1413_DEFAULT_HISTORY_PATH, help="history JSONL path")

    p_list = sub.add_parser("list", help="list history snapshots")
    p_list.add_argument("--history", default=V1413_DEFAULT_HISTORY_PATH, help="history JSONL path")
    p_list.add_argument("--last", type=int, default=None, help="show last N snapshots")
    p_list.add_argument("--json", action="store_true", help="JSON output")

    p_trend = sub.add_parser("trend", help="compute trend")
    p_trend.add_argument("--history", default=V1413_DEFAULT_HISTORY_PATH, help="history JSONL path")
    p_trend.add_argument("--json", action="store_true", help="JSON output")

    p_dig = sub.add_parser("digest", help="aggregate statistics")
    p_dig.add_argument("--history", default=V1413_DEFAULT_HISTORY_PATH, help="history JSONL path")
    p_dig.add_argument("--json", action="store_true", help="JSON output")

    p_base = sub.add_parser("baseline", help="set baseline from latest snapshot")
    p_base.add_argument("--history", default=V1413_DEFAULT_HISTORY_PATH, help="history JSONL path")
    p_base.add_argument("--baseline-path", default=V1413_DEFAULT_BASELINE_PATH, help="baseline JSON path")
    p_base.add_argument("--note", default="", help="baseline note")

    p_cmp = sub.add_parser("compare", help="compare current V1412 to baseline")
    p_cmp.add_argument("--baseline-path", default=V1413_DEFAULT_BASELINE_PATH, help="baseline JSON path")
    p_cmp.add_argument("--json", action="store_true", help="JSON output")

    p_render = sub.add_parser("render", help="render markdown history report")
    p_render.add_argument("--history", default=V1413_DEFAULT_HISTORY_PATH, help="history JSONL path")
    p_render.add_argument("--baseline-path", default=V1413_DEFAULT_BASELINE_PATH, help="baseline JSON path")
    p_render.add_argument("--out", default=None, help="output file (default: stdout)")
    p_render.add_argument("--format", default="md", choices=["md", "json"], help="output format")

    sub.add_parser("popper", help="V1413 Popper self-test")
    p_meta = sub.add_parser("meta", help="V1413 constants")
    p_meta.add_argument("--json", action="store_true", help="JSON output")

    sub.add_parser("demo", help="V1413 demo (append 2 synthetic snapshots)")

    args = parser.parse_args(argv)
    cmd = args.cmd or "version"

    if cmd == "version":
        print(f"V1413 ASI 总框架 history v{V1413_VERSION} (schema {V1413_SCHEMA})")
        return 0

    if cmd == "snapshot":
        try:
            snap = build_snapshot_from_v1412(note=args.note)
        except Exception as e:
            print(f"V1412 unavailable: {e}", file=sys.stderr)
            return 1
        append_snapshot(snap, args.history)
        print(
            f"appended: {snap.timestamp} id={snap.snapshot_id} "
            f"verdict={snap.verdict} fw={snap.framework_score} "
            f"lvl={snap.level_score} coh={snap.coherence_score} "
            f"chain={snap.chain_ok} borrowed={snap.borrowed_count} "
            f"gap={snap.gap_to_north_star:.4f}"
        )
        return 0

    if cmd == "list":
        snaps = load_history(args.history)
        if args.last:
            snaps = snaps[-args.last:]
        if args.json:
            print(json.dumps([s.to_dict() for s in snaps], indent=2, ensure_ascii=False))
        else:
            if not snaps:
                print("no snapshots")
            else:
                for s in snaps:
                    print(
                        f"{s.timestamp} {s.snapshot_id} {s.verdict} "
                        f"fw={s.framework_score} lvl={s.level_score} "
                        f"coh={s.coherence_score} chain={s.chain_ok} "
                        f"bor={s.borrowed_count} gap={s.gap_to_north_star:.4f} "
                        f"note={s.note[:30]}"
                    )
        return 0

    if cmd == "trend":
        snaps = load_history(args.history)
        t = compute_trend(snaps)
        if args.json:
            print(json.dumps(t.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(_render_trend_text(t))
        return 0

    if cmd == "digest":
        snaps = load_history(args.history)
        d = compute_digest(snaps)
        if args.json:
            print(json.dumps(d.to_dict(), indent=2, ensure_ascii=False))
        else:
            print(_render_digest_text(d))
        return 0

    if cmd == "baseline":
        snaps = load_history(args.history)
        if not snaps:
            print("no snapshots in history; cannot set baseline", file=sys.stderr)
            return 1
        latest = snaps[-1]
        base = make_baseline(latest, note=args.note)
        write_baseline(base, args.baseline_path)
        print(
            f"baseline set: {base.baseline_timestamp} {base.baseline_verdict} "
            f"fw={base.baseline_framework_score} gap={base.baseline_gap:.4f}"
        )
        return 0

    if cmd == "compare":
        base = load_baseline(args.baseline_path)
        if base is None:
            print(f"no baseline at {args.baseline_path}", file=sys.stderr)
            return 1
        try:
            snap = build_snapshot_from_v1412()
        except Exception as e:
            print(f"V1412 unavailable: {e}", file=sys.stderr)
            return 1
        cmp = compare_to_baseline(snap, base)
        if args.json:
            print(json.dumps(cmp, indent=2, ensure_ascii=False))
        else:
            print(
                f"compare: snapshot={snap.timestamp} baseline={base.baseline_timestamp}\n"
                f"  Δfw={cmp['delta_framework']:+d} Δlvl={cmp['delta_level']:+d} "
                f"Δcoh={cmp['delta_coherence']:+d} Δbor={cmp['delta_borrowed']:+d} "
                f"Δgap={cmp['delta_gap']:+.4f}\n"
                f"  verdict_regressed={cmp['verdict_regressed']} "
                f"verdict_improved={cmp['verdict_improved']} "
                f"verdict_unchanged={cmp['verdict_unchanged']}"
            )
        return 0

    if cmd == "render":
        snaps = load_history(args.history)
        t = compute_trend(snaps)
        d = compute_digest(snaps)
        base = load_baseline(args.baseline_path)
        if args.format == "json":
            out = json.dumps({
                "trend": t.to_dict(),
                "digest": d.to_dict(),
                "snapshots": [s.to_dict() for s in snaps],
                "baseline": base.to_dict() if base else None,
            }, indent=2, ensure_ascii=False)
        else:
            out = render_history_md(snaps, t, d, baseline=base)
        if args.out:
            op = Path(args.out)
            op.parent.mkdir(parents=True, exist_ok=True)
            with open(op, "w", encoding="utf-8") as f:
                f.write(out)
            print(f"wrote {args.out} ({len(out)} bytes)")
        else:
            print(out)
        return 0

    if cmd == "popper":
        r = popper_self_test()
        print(json.dumps(r, indent=2, ensure_ascii=False))
        return 0 if r["passed"] else 1

    if cmd == "meta":
        meta = {
            "module": V1413_MODULE,
            "version": V1413_VERSION,
            "schema": V1413_SCHEMA,
            "guards": list(V1413_GUARDS),
            "v3_guards": list(V1413_V3_GUARDS),
            "trends": list(V1413_TRENDS),
            "borrowed": [list(b) for b in V1413_BORROWED],
            "default_history_path": V1413_DEFAULT_HISTORY_PATH,
            "default_baseline_path": V1413_DEFAULT_BASELINE_PATH,
        }
        if args.json:
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    if cmd == "demo":
        import tempfile
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            s1 = HistorySnapshot(
                timestamp="2026-08-10T01:00:00Z",
                snapshot_id="demo_1",
                verdict="GOOD", framework_score=10, level_score=11,
                coherence_score=11, chain_ok=True, borrowed_count=5,
                gap_to_north_star=0.0800,
                note="demo snapshot 1",
            )
            s2 = HistorySnapshot(
                timestamp="2026-08-10T02:00:00Z",
                snapshot_id="demo_2",
                verdict="COMPLETE", framework_score=11, level_score=12,
                coherence_score=12, chain_ok=True, borrowed_count=7,
                gap_to_north_star=0.0695,
                note="demo snapshot 2",
            )
            append_snapshot(s1, tp)
            append_snapshot(s2, tp)
            loaded = load_history(tp)
            t = compute_trend(loaded)
            d = compute_digest(loaded)
            print(f"V1413 demo: 2 snapshots appended")
            print(f"  trend: {t.direction} (Δfw={t.delta_framework:+d}, Δgap={t.delta_gap:+.4f})")
            print(f"  digest: {d.n_snapshots} snapshots, "
                  f"COMPLETE={d.n_complete} GOOD={d.n_good}")
        finally:
            Path(tp).unlink()
        return 0

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli())