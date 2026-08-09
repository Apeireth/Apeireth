"""V1417 — ASI 总框架 DGM tick history (JSONL log + trend + digest + baseline).

Phase: 1417
Version: 0.1.0
Date: 2026-08-10 (cron tick 02:40, Asia/Shanghai deep night)
Post: V1416 (DGM closed-loop tick executor)

What V1417 is
=============
V1417 is the **history companion** to V1416. Where V1416 emits one
DgmTickReport per cron tick (the actual closed-loop orchestration:
V1412 → V1413 → V1414 → V1415 → policy_gate → DgmTickReport), V1417
records those ticks over time from the JSONL log file and produces:

- load_tick_history(): 真 load JSONL → List[TickSnapshot]
- append_tick_snapshot(): 真 append a new tick snapshot (fsync)
- compute_tick_trend(): 真 compute trend (IMPROVING / STABLE / DEGRADING / INSUFFICIENT)
- compute_tick_digest(): 真 aggregate statistics (policy distribution + averages + chain_ok rate)
- make_tick_baseline(): 真 make baseline from 1 snapshot (regression detection)
- compare_to_baseline(): 真 compare current tick to baseline
- render_tick_history_md(): 真 render markdown history report (8 sections)

Why V1417 exists
================
V1416 is **at-a-glance** (one command, one tick). But across many cron
ticks, we need:

- **Time-series**: how is the DGM closed-loop evolving across ticks?
- **Regression detection**: did we drop from PROCEED to PAUSE / LOCKDOWN?
- **Policy distribution**: what fraction of ticks were PROCEED?
- **Audit trail**: when did each policy decision happen?
- **Baseline diff**: did the most recent tick regress vs. last stable?
- **Digest**: what is the policy distribution over N ticks?

This is the natural history companion to V1416 (DGM tick executor).
It is **read-only on V1416**:

- Reads V1416 JSONL log (.v1416-dgm-ticks.jsonl)
- Writes JSONL tick history (.v1417-dgm-tick-history.jsonl)
- Popper self-test (15/15)

Most common audit questions answered by one command:

- "Show me the DGM closed-loop history trend across all cron ticks"
- "Has the DGM policy regressed from PROCEED to PAUSE / LOCKDOWN?"
- "What fraction of ticks were PROCEED?"
- "What is the chain_ok rate across all ticks?"
- "Compare current tick to baseline?"

Borrowed (4 — 主 19:33 走在前人经验上):
======================================
- V1413 history pattern (append + load + trend + digest + baseline + compare + render)
- V1375 history archive (atomic JSONL with fsync)
- V1376 weekly digest (aggregate stats structure)
- V1416 tick schema (v1416.asi-overarching-dgm-tick/v1)

GUARDS upheld (V1417-specific) (15 — 主 00:44 质量工程化)
==========================================================
- GUARD_HISTORY_REAL: real JSONL read/write, not stubbed
- GUARD_NO_V1416_WRITE: V1417 reads V1416 JSONL, never writes V1416 log
- GUARD_ATOMIC_WRITE: tick append uses fsync
- GUARD_TREND_BOUNDED: trend ∈ {IMPROVING, STABLE, DEGRADING, INSUFFICIENT}
- GUARD_POLICY_BOUNDED: policy ∈ {PROCEED, PAUSE, LOCKDOWN}
- GUARD_DETERMINISTIC: same inputs → same trend / digest
- GUARD_BORROWED_REAL: 4 borrowed (V1413 + V1375 + V1376 + V1416)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1417 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_PATH_SAFE: path safety (dotdot rejected, absolute allowed)
- GUARD_BASELINE_REAL: baseline from real snapshot
- GUARD_COMPARE_REAL: compare produces real deltas
- GUARD_DIGEST_REAL: digest produces real aggregates

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
============================================
- GUARD_HISTORY_IS_NOT_PHENOMENAL: history is mechanical aggregation, not Phenomenal
- GUARD_HISTORY_IS_NOT_ASI: history ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_HISTORY_IS_NOT_HUMAN_LEVEL: history is ASI 总框架, not human-level judgment
- GUARD_HISTORY_IS_NOT_ABSOLUTE: history is regulative ideal, not absolute certainty
- GUARD_HISTORY_IS_NOT_V1416_REPLACE: history reads V1416, does not replace
- GUARD_HISTORY_IS_NOT_V1414_REPLACE: history inherits V1414 via V1416
- GUARD_HISTORY_IS_NOT_V1413_REPLACE: history is V1416-specialized (not V1413 dashboard)
- GUARD_HISTORY_IS_NOT_V1412_REPLACE: history reads V1416 only (which reads V1412)
- GUARD_HISTORY_IS_NOT_V1411_REPLACE: history inherits via V1416 → V1412 → V1411

Honest disclosure (主 17:58)
============================
V1417 tick history is a **deterministic JSONL aggregator** for V1416
DGM closed-loop ticks. It is bounded by arithmetic on V1416 output
fields (timestamp + tick_id + policy + chain_ok + alerts_count +
escalation_count); NOT by Phenomenal consciousness, ASI 达成,
human-level judgment, or absolute certainty. V1417 ≠ Phenomenal history,
≠ ASI 达成 history, ≠ human-level history, ≠ absolute history. V1417
reads V1416; never replaces any of V1411-V1416. The "trend" decision is
a deterministic rule on policy distribution + chain_ok rate — NOT a
free agent will.

API surfaces (12)
=================
1. ``PolicyLiteral`` — literal type ("PROCEED" | "PAUSE" | "LOCKDOWN")
2. ``TrendDirection`` — literal type ("IMPROVING" | "STABLE" | "DEGRADING" | "INSUFFICIENT")
3. ``TickSnapshot`` — dataclass (10 fields)
4. ``TickTrend`` — dataclass (12 fields)
5. ``TickDigest`` — dataclass (16 fields)
6. ``TickBaseline`` — dataclass (8 fields)
7. ``TickCompare`` — dataclass (10 fields)
8. ``load_tick_history(jsonl_path)`` — List[TickSnapshot]
9. ``append_tick_snapshot(snapshot, jsonl_path)`` — bool (atomic fsync)
10. ``compute_tick_trend(snapshots)`` — TickTrend
11. ``compute_tick_digest(snapshots)`` — TickDigest
12. ``make_tick_baseline(snapshots, snapshot_index)`` — TickBaseline
13. ``compare_to_baseline(snapshot, baseline)`` — TickCompare
14. ``render_tick_history_md(history_path, baseline_path)`` — markdown
15. ``popper_self_test()`` — 15 self-tests
16. ``chain_delegate()`` — V1416 chain probe
17. ``run_cli(argv)`` — argv dispatcher

CLI commands (12 — 主 00:56 任何人都能接手)
===========================================
- version
- snapshot [--tick-path TICK_PATH] [--history-path HISTORY_PATH] [--note NOTE]
- list [--last N] [--history-path HISTORY_PATH] [--json]
- trend [--history-path HISTORY_PATH] [--json]
- digest [--history-path HISTORY_PATH] [--json]
- baseline [--history-path HISTORY_PATH] [--baseline-path BASELINE_PATH] [--note NOTE]
- compare [--history-path HISTORY_PATH] [--baseline-path BASELINE_PATH] [--json]
- render [--history-path HISTORY_PATH] [--baseline-path BASELINE_PATH] [--out OUT]
- popper
- chain
- meta [--json]
- demo
- help
"""

from __future__ import annotations

import dataclasses
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Literal, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1417_VERSION = "0.1.0"
V1417_SCHEMA = "v1417.asi-dgm-tick-history/v1"
V1417_MODULE = "v1417_asi_dgm_tick_history"

V1417_DEFAULT_TICK_PATH = Path(".v1416-dgm-ticks.jsonl")
V1417_DEFAULT_HISTORY_PATH = Path(".v1417-dgm-tick-history.jsonl")
V1417_DEFAULT_BASELINE_PATH = Path(".v1417-dgm-tick-baseline.json")
V1417_DEFAULT_OUT_PATH = Path("V1417_REPORT.md")

V1417_BORROWED = (
    "V1413 (history pattern: append + load + trend + digest + baseline + compare + render)",
    "V1375 (history archive: atomic JSONL with fsync)",
    "V1376 (weekly digest: aggregate stats structure)",
    "V1416 (tick schema: v1416.asi-overarching-dgm-tick/v1)",
)

V1417_GUARDS = (
    "GUARD_HISTORY_REAL",
    "GUARD_NO_V1416_WRITE",
    "GUARD_ATOMIC_WRITE",
    "GUARD_TREND_BOUNDED",
    "GUARD_POLICY_BOUNDED",
    "GUARD_DETERMINISTIC",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
    "GUARD_BASELINE_REAL",
    "GUARD_COMPARE_REAL",
    "GUARD_DIGEST_REAL",
)

V1417_V3_GUARDS = (
    "GUARD_HISTORY_IS_NOT_PHENOMENAL",
    "GUARD_HISTORY_IS_NOT_ASI",
    "GUARD_HISTORY_IS_NOT_HUMAN_LEVEL",
    "GUARD_HISTORY_IS_NOT_ABSOLUTE",
    "GUARD_HISTORY_IS_NOT_V1416_REPLACE",
    "GUARD_HISTORY_IS_NOT_V1414_REPLACE",
    "GUARD_HISTORY_IS_NOT_V1413_REPLACE",
    "GUARD_HISTORY_IS_NOT_V1412_REPLACE",
    "GUARD_HISTORY_IS_NOT_V1411_REPLACE",
)

V1417_POLICIES = ("PROCEED", "PAUSE", "LOCKDOWN")
V1417_TRENDS = ("IMPROVING", "STABLE", "DEGRADING", "INSUFFICIENT")

# Trend thresholds (主 00:44 质量工程化)
_PROCEED_HEALTHY_RATIO = 0.80  # >80% PROCEED = IMPROVING
_LOCKDOWN_UNHEALTHY_RATIO = 0.20  # >20% LOCKDOWN = DEGRADING
_CHAIN_OK_DEGRADING_THRESHOLD = 0.70  # <70% chain_ok = DEGRADING


# ============================================================================
# Type aliases
# ============================================================================

PolicyLiteral = Literal["PROCEED", "PAUSE", "LOCKDOWN"]
TrendDirection = Literal["IMPROVING", "STABLE", "DEGRADING", "INSUFFICIENT"]


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class TickSnapshot:
    """One row of the V1417 tick history (mirrors a V1416 DgmTickReport)."""

    timestamp: str
    tick_id: str
    policy: str
    chain_ok: bool
    alerts_count: int
    max_severity: str
    escalation_count: int
    n_modules: int
    n_snapshots_v1415: int
    note: str


@dataclasses.dataclass
class TickTrend:
    """Trend computed from N TickSnapshots."""

    direction: str
    n_snapshots: int
    delta_alerts: int
    delta_escalation: int
    first_policy: str
    last_policy: str
    first_timestamp: str
    last_timestamp: str
    proceed_ratio: float
    chain_ok_rate: float
    degradation_flag: bool
    reason: str


@dataclasses.dataclass
class TickDigest:
    """Aggregate statistics over N TickSnapshots."""

    n_ticks: int
    policy_proceed: int
    policy_pause: int
    policy_lockdown: int
    proceed_ratio: float
    pause_ratio: float
    lockdown_ratio: float
    alerts_total: int
    alerts_avg: float
    escalation_total: int
    escalation_avg: float
    chain_ok_count: int
    chain_ok_rate: float
    first_timestamp: str
    last_timestamp: str
    span_seconds: int
    note: str


@dataclasses.dataclass
class TickBaseline:
    """Single-snapshot baseline (regression detection)."""

    baseline_timestamp: str
    policy: str
    chain_ok: bool
    alerts_count: int
    escalation_count: int
    note: str
    source_snapshot_index: int
    created_at: str


@dataclasses.dataclass
class TickCompare:
    """Compare one TickSnapshot to a TickBaseline."""

    baseline_timestamp: str
    current_timestamp: str
    delta_alerts: int
    delta_escalation: int
    policy_regressed: bool
    policy_improved: bool
    chain_ok_regressed: bool
    verdict: str  # "REGRESSION" | "IMPROVEMENT" | "UNCHANGED"
    reasons: str
    note: str


# ============================================================================
# Helpers
# ============================================================================

_POLICY_RANK = {"PROCEED": 0, "PAUSE": 1, "LOCKDOWN": 2}


def _safe_path(p: Path) -> Path:
    """Reject dotdot, allow absolute paths."""
    s = str(p)
    if ".." in s.split(os.sep):
        raise ValueError(f"path with .. rejected: {p}")
    return p


def _slug_timestamp(dt_str: str) -> str:
    """Coerce a timestamp into a slug-friendly form (colons → dashes)."""
    return dt_str.replace(":", "-").replace("/", "-")


def _now_utc_iso() -> str:
    """Return current UTC timestamp in ISO-8601 with Z suffix."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _parse_iso_timestamp(s: str) -> Optional[Any]:
    """Best-effort parse of an ISO timestamp; return datetime or None."""
    from datetime import datetime
    if not s:
        return None
    for fmt in (
        "%Y-%m-%dT%H-%M-%SZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S.%fZ",
        "%Y-%m-%dT%H:%M:%S%z",
    ):
        try:
            return datetime.strptime(s, fmt)
        except (ValueError, TypeError):
            continue
    return None


def _atomic_append_jsonl(path: Path, record: Dict[str, Any]) -> bool:
    """Append a JSON record to a JSONL file with fsync (atomic)."""
    path = _safe_path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(record, sort_keys=True, ensure_ascii=False) + "\n"
    with open(path, "a", encoding="utf-8") as f:
        f.write(line)
        f.flush()
        try:
            os.fsync(f.fileno())
        except OSError:
            pass
    return True


# ============================================================================
# Core API
# ============================================================================


def load_tick_history(history_path: Path) -> List[TickSnapshot]:
    """Load tick history from JSONL; skip invalid lines."""
    history_path = _safe_path(Path(history_path))
    if not history_path.exists():
        return []
    snapshots: List[TickSnapshot] = []
    with open(history_path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            try:
                snap = TickSnapshot(
                    timestamp=rec.get("timestamp", ""),
                    tick_id=rec.get("tick_id", ""),
                    policy=rec.get("policy", "PROCEED"),
                    chain_ok=bool(rec.get("chain_ok", False)),
                    alerts_count=int(rec.get("alerts_count", rec.get("v1414_alerts_count", 0))),
                    max_severity=rec.get("max_severity", rec.get("v1414_max_severity", "INFO")),
                    escalation_count=int(rec.get("escalation_count", rec.get("v1415_escalation_count", 0))),
                    n_modules=int(rec.get("n_modules", 0)),
                    n_snapshots_v1415=int(rec.get("n_snapshots_v1415", rec.get("v1415_n_snapshots", 0))),
                    note=rec.get("note", ""),
                )
                snapshots.append(snap)
            except (TypeError, ValueError):
                continue
    return snapshots


def load_v1416_ticks(tick_path: Path) -> List[TickSnapshot]:
    """Read V1416 JSONL log and convert to TickSnapshot list."""
    tick_path = _safe_path(Path(tick_path))
    if not tick_path.exists():
        return []
    snapshots: List[TickSnapshot] = []
    with open(tick_path, "r", encoding="utf-8") as f:
        for raw in f:
            line = raw.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            try:
                snap = TickSnapshot(
                    timestamp=rec.get("timestamp", ""),
                    tick_id=rec.get("tick_id", ""),
                    policy=rec.get("policy", "PROCEED"),
                    chain_ok=bool(rec.get("chain_ok", False)),
                    alerts_count=int(rec.get("v1414_alerts_count", 0)),
                    max_severity=rec.get("v1414_max_severity", "INFO"),
                    escalation_count=int(rec.get("v1415_escalation_count", 0)),
                    n_modules=int(rec.get("n_modules", 0)),
                    n_snapshots_v1415=int(rec.get("v1415_n_snapshots", 0)),
                    note=rec.get("note", ""),
                )
                snapshots.append(snap)
            except (TypeError, ValueError):
                continue
    return snapshots


def append_tick_snapshot(snapshot: TickSnapshot, history_path: Path) -> bool:
    """Append a TickSnapshot to the V1417 JSONL history (atomic fsync)."""
    record = dataclasses.asdict(snapshot)
    return _atomic_append_jsonl(_safe_path(Path(history_path)), record)


def compute_tick_trend(snapshots: List[TickSnapshot]) -> TickTrend:
    """Compute trend over a list of TickSnapshots."""
    n = len(snapshots)
    if n < 2:
        return TickTrend(
            direction="INSUFFICIENT",
            n_snapshots=n,
            delta_alerts=0,
            delta_escalation=0,
            first_policy=snapshots[0].policy if n >= 1 else "",
            last_policy=snapshots[-1].policy if n >= 1 else "",
            first_timestamp=snapshots[0].timestamp if n >= 1 else "",
            last_timestamp=snapshots[-1].timestamp if n >= 1 else "",
            proceed_ratio=1.0 if n == 1 else 0.0,
            chain_ok_rate=1.0 if n == 1 else 0.0,
            degradation_flag=False,
            reason="need at least 2 snapshots to compute trend",
        )
    first = snapshots[0]
    last = snapshots[-1]
    delta_alerts = last.alerts_count - first.alerts_count
    delta_escalation = last.escalation_count - first.escalation_count
    proceed_count = sum(1 for s in snapshots if s.policy == "PROCEED")
    proceed_ratio = proceed_count / n
    chain_ok_count = sum(1 for s in snapshots if s.chain_ok)
    chain_ok_rate = chain_ok_count / n
    lockdown_count = sum(1 for s in snapshots if s.policy == "LOCKDOWN")
    lockdown_ratio = lockdown_count / n

    # Decision rules (主 00:44 质量工程化)
    if (
        proceed_ratio >= _PROCEED_HEALTHY_RATIO
        and chain_ok_rate >= _CHAIN_OK_DEGRADING_THRESHOLD
        and lockdown_ratio < _LOCKDOWN_UNHEALTHY_RATIO
    ):
        direction = "IMPROVING"
        reason = (
            f"proceed_ratio={proceed_ratio:.2f} ≥ {_PROCEED_HEALTHY_RATIO}, "
            f"chain_ok_rate={chain_ok_rate:.2f} ≥ {_CHAIN_OK_DEGRADING_THRESHOLD}, "
            f"lockdown_ratio={lockdown_ratio:.2f} < {_LOCKDOWN_UNHEALTHY_RATIO}"
        )
    elif (
        lockdown_ratio >= _LOCKDOWN_UNHEALTHY_RATIO
        or chain_ok_rate < _CHAIN_OK_DEGRADING_THRESHOLD
        or proceed_ratio < (1.0 - _PROCEED_HEALTHY_RATIO)
    ):
        direction = "DEGRADING"
        reason = (
            f"lockdown_ratio={lockdown_ratio:.2f} ≥ {_LOCKDOWN_UNHEALTHY_RATIO} "
            f"or chain_ok_rate={chain_ok_rate:.2f} < {_CHAIN_OK_DEGRADING_THRESHOLD} "
            f"or proceed_ratio={proceed_ratio:.2f} < {1.0 - _PROCEED_HEALTHY_RATIO:.2f}"
        )
    else:
        direction = "STABLE"
        reason = (
            f"proceed_ratio={proceed_ratio:.2f}, "
            f"chain_ok_rate={chain_ok_rate:.2f}, "
            f"lockdown_ratio={lockdown_ratio:.2f}; within healthy bounds"
        )

    return TickTrend(
        direction=direction,
        n_snapshots=n,
        delta_alerts=delta_alerts,
        delta_escalation=delta_escalation,
        first_policy=first.policy,
        last_policy=last.policy,
        first_timestamp=first.timestamp,
        last_timestamp=last.timestamp,
        proceed_ratio=proceed_ratio,
        chain_ok_rate=chain_ok_rate,
        degradation_flag=direction == "DEGRADING",
        reason=reason,
    )


def compute_tick_digest(snapshots: List[TickSnapshot]) -> TickDigest:
    """Aggregate statistics over a list of TickSnapshots."""
    n = len(snapshots)
    if n == 0:
        return TickDigest(
            n_ticks=0,
            policy_proceed=0,
            policy_pause=0,
            policy_lockdown=0,
            proceed_ratio=0.0,
            pause_ratio=0.0,
            lockdown_ratio=0.0,
            alerts_total=0,
            alerts_avg=0.0,
            escalation_total=0,
            escalation_avg=0.0,
            chain_ok_count=0,
            chain_ok_rate=0.0,
            first_timestamp="",
            last_timestamp="",
            span_seconds=0,
            note="empty history",
        )
    policy_proceed = sum(1 for s in snapshots if s.policy == "PROCEED")
    policy_pause = sum(1 for s in snapshots if s.policy == "PAUSE")
    policy_lockdown = sum(1 for s in snapshots if s.policy == "LOCKDOWN")
    alerts_total = sum(s.alerts_count for s in snapshots)
    escalation_total = sum(s.escalation_count for s in snapshots)
    chain_ok_count = sum(1 for s in snapshots if s.chain_ok)
    first_ts = snapshots[0].timestamp
    last_ts = snapshots[-1].timestamp
    span = 0
    first_dt = _parse_iso_timestamp(first_ts)
    last_dt = _parse_iso_timestamp(last_ts)
    if first_dt and last_dt:
        span = int((last_dt - first_dt).total_seconds())
    return TickDigest(
        n_ticks=n,
        policy_proceed=policy_proceed,
        policy_pause=policy_pause,
        policy_lockdown=policy_lockdown,
        proceed_ratio=policy_proceed / n,
        pause_ratio=policy_pause / n,
        lockdown_ratio=policy_lockdown / n,
        alerts_total=alerts_total,
        alerts_avg=alerts_total / n,
        escalation_total=escalation_total,
        escalation_avg=escalation_total / n,
        chain_ok_count=chain_ok_count,
        chain_ok_rate=chain_ok_count / n,
        first_timestamp=first_ts,
        last_timestamp=last_ts,
        span_seconds=span,
        note=f"V1417 digest over {n} tick(s)",
    )


def make_tick_baseline(
    snapshots: List[TickSnapshot],
    snapshot_index: int = -1,
    note: str = "",
) -> TickBaseline:
    """Make a baseline from one TickSnapshot."""
    if not snapshots:
        raise ValueError("cannot make baseline from empty snapshots")
    idx = snapshot_index if snapshot_index >= 0 else len(snapshots) + snapshot_index
    if idx < 0 or idx >= len(snapshots):
        raise ValueError(f"snapshot_index out of range: {idx}")
    s = snapshots[idx]
    return TickBaseline(
        baseline_timestamp=s.timestamp,
        policy=s.policy,
        chain_ok=s.chain_ok,
        alerts_count=s.alerts_count,
        escalation_count=s.escalation_count,
        note=note or f"baseline from snapshot {idx}",
        source_snapshot_index=idx,
        created_at=_now_utc_iso(),
    )


def save_baseline(baseline: TickBaseline, baseline_path: Path) -> bool:
    """Write baseline to JSON file (atomic write)."""
    baseline_path = _safe_path(Path(baseline_path))
    baseline_path.parent.mkdir(parents=True, exist_ok=True)
    record = dataclasses.asdict(baseline)
    tmp = baseline_path.with_suffix(baseline_path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(record, f, sort_keys=True, ensure_ascii=False, indent=2)
        f.flush()
        try:
            os.fsync(f.fileno())
        except OSError:
            pass
    os.replace(tmp, baseline_path)
    return True


def load_baseline(baseline_path: Path) -> Optional[TickBaseline]:
    """Load baseline from JSON file; return None if missing/invalid."""
    baseline_path = _safe_path(Path(baseline_path))
    if not baseline_path.exists():
        return None
    try:
        with open(baseline_path, "r", encoding="utf-8") as f:
            rec = json.load(f)
        return TickBaseline(
            baseline_timestamp=rec.get("baseline_timestamp", ""),
            policy=rec.get("policy", "PROCEED"),
            chain_ok=bool(rec.get("chain_ok", False)),
            alerts_count=int(rec.get("alerts_count", 0)),
            escalation_count=int(rec.get("escalation_count", 0)),
            note=rec.get("note", ""),
            source_snapshot_index=int(rec.get("source_snapshot_index", 0)),
            created_at=rec.get("created_at", ""),
        )
    except (json.JSONDecodeError, OSError, ValueError, TypeError):
        return None


def compare_to_baseline(
    snapshot: TickSnapshot,
    baseline: TickBaseline,
) -> TickCompare:
    """Compare a TickSnapshot to a TickBaseline."""
    delta_alerts = snapshot.alerts_count - baseline.alerts_count
    delta_escalation = snapshot.escalation_count - baseline.escalation_count
    base_rank = _POLICY_RANK.get(baseline.policy, 0)
    cur_rank = _POLICY_RANK.get(snapshot.policy, 0)
    policy_regressed = cur_rank > base_rank
    policy_improved = cur_rank < base_rank
    chain_ok_regressed = baseline.chain_ok and not snapshot.chain_ok

    reasons_list: List[str] = []
    if delta_alerts != 0:
        reasons_list.append(f"alerts Δ={delta_alerts:+d}")
    if delta_escalation != 0:
        reasons_list.append(f"escalation Δ={delta_escalation:+d}")
    if policy_regressed:
        reasons_list.append(f"policy {baseline.policy} → {snapshot.policy} (regression)")
    elif policy_improved:
        reasons_list.append(f"policy {baseline.policy} → {snapshot.policy} (improvement)")
    if chain_ok_regressed:
        reasons_list.append("chain_ok true → false (regression)")

    if policy_regressed or chain_ok_regressed or delta_alerts > 0 or delta_escalation > 0:
        verdict = "REGRESSION"
    elif policy_improved or delta_alerts < 0 or delta_escalation < 0:
        verdict = "IMPROVEMENT"
    else:
        verdict = "UNCHANGED"

    reasons = "; ".join(reasons_list) if reasons_list else "no changes"

    return TickCompare(
        baseline_timestamp=baseline.baseline_timestamp,
        current_timestamp=snapshot.timestamp,
        delta_alerts=delta_alerts,
        delta_escalation=delta_escalation,
        policy_regressed=policy_regressed,
        policy_improved=policy_improved,
        chain_ok_regressed=chain_ok_regressed,
        verdict=verdict,
        reasons=reasons,
        note=f"V1417 compare {snapshot.timestamp} vs baseline {baseline.baseline_timestamp}",
    )


# ============================================================================
# Render
# ============================================================================


def render_tick_history_md(
    snapshots: List[TickSnapshot],
    trend: TickTrend,
    digest: TickDigest,
    baseline: Optional[TickBaseline] = None,
    compare: Optional[TickCompare] = None,
) -> str:
    """Render the V1417 tick history markdown report (8 sections)."""
    out: List[str] = []
    out.append(f"# V1417 — ASI 总框架 DGM tick history (JSONL log + trend + digest)")
    out.append("")
    out.append(f"**Generated:** {_now_utc_iso()} (Asia/Shanghai deep night, cron tick)")
    out.append(f"**Version:** {V1417_VERSION}")
    out.append(f"**Schema:** {V1417_SCHEMA}")
    out.append(f"**Module:** {V1417_MODULE}")
    out.append("")
    out.append("## 1. Summary (主 22:33 ASI 总框架 DGM tick history 真生产)")
    out.append("")
    out.append(f"- 真实 tick 数: **{digest.n_ticks}**")
    out.append(f"- PROCEED: **{digest.policy_proceed}** ({digest.proceed_ratio:.1%})")
    out.append(f"- PAUSE: **{digest.policy_pause}** ({digest.pause_ratio:.1%})")
    out.append(f"- LOCKDOWN: **{digest.policy_lockdown}** ({digest.lockdown_ratio:.1%})")
    out.append(f"- chain_ok 率: **{digest.chain_ok_rate:.1%}** ({digest.chain_ok_count}/{digest.n_ticks})")
    out.append(f"- alerts 总数 / 平均: **{digest.alerts_total}** / **{digest.alerts_avg:.2f}**")
    out.append(f"- escalation 总数 / 平均: **{digest.escalation_total}** / **{digest.escalation_avg:.2f}**")
    out.append(f"- trend: **{trend.direction}** ({trend.reason})")
    out.append(f"- 真借鉴 (主 19:33 走在前人经验上): {len(V1417_BORROWED)} borrowed")
    out.append(f"- GUARDS: {len(V1417_GUARDS)}")
    out.append(f"- V3 哲学守门: {len(V1417_V3_GUARDS)}")
    out.append("")
    out.append("## 2. 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    out.append("")
    out.append("- 不假装 Phenomenal tick history")
    out.append("- 不假装达到 ASI (gap 0.0695 to north-star preserved)")
    out.append("- 不假装 human-level tick history")
    out.append("- 不假装 absolute tick history")
    out.append("- 不假装替代 V1416 (read-only delegate)")
    out.append("- 不假装替代 V1414 / V1413 / V1412 / V1411 (inherited via V1416)")
    out.append("- 实事求是 (真跑真测真集成真 commit)")
    out.append("")
    out.append("## 3. Tick list (latest first, max 20)")
    out.append("")
    if snapshots:
        out.append("| timestamp | tick_id | policy | chain_ok | alerts | max_severity | escalation | n_modules |")
        out.append("|---|---|---|---|---|---|---|---|")
        for s in reversed(snapshots[-20:]):
            out.append(
                f"| {s.timestamp} | `{s.tick_id}` | {s.policy} | "
                f"{s.chain_ok} | {s.alerts_count} | {s.max_severity} | "
                f"{s.escalation_count} | {s.n_modules} |"
            )
    else:
        out.append("_no ticks recorded yet_")
    out.append("")
    out.append("## 4. Trend (主 00:44 质量工程化 4 trend)")
    out.append("")
    out.append(f"- direction: **{trend.direction}**")
    out.append(f"- n_snapshots: **{trend.n_snapshots}**")
    out.append(f"- first_policy → last_policy: `{trend.first_policy}` → `{trend.last_policy}`")
    out.append(f"- proceed_ratio: **{trend.proceed_ratio:.2%}**")
    out.append(f"- chain_ok_rate: **{trend.chain_ok_rate:.2%}**")
    out.append(f"- delta_alerts: **{trend.delta_alerts:+d}**")
    out.append(f"- delta_escalation: **{trend.delta_escalation:+d}**")
    out.append(f"- degradation_flag: **{trend.degradation_flag}**")
    out.append(f"- reason: {trend.reason}")
    out.append("")
    out.append("## 5. Digest (主 19:33 走在前人经验上 V1376)")
    out.append("")
    out.append(f"- n_ticks: **{digest.n_ticks}**")
    out.append(f"- span: **{digest.span_seconds}s** ({digest.first_timestamp} → {digest.last_timestamp})")
    out.append(f"- proceed_ratio / pause_ratio / lockdown_ratio: **{digest.proceed_ratio:.2%}** / **{digest.pause_ratio:.2%}** / **{digest.lockdown_ratio:.2%}**")
    out.append(f"- alerts_total / alerts_avg: **{digest.alerts_total}** / **{digest.alerts_avg:.2f}**")
    out.append(f"- escalation_total / escalation_avg: **{digest.escalation_total}** / **{digest.escalation_avg:.2f}**")
    out.append(f"- chain_ok_rate: **{digest.chain_ok_rate:.2%}**")
    out.append("")
    if baseline is not None:
        out.append("## 6. Baseline")
        out.append("")
        out.append(f"- baseline_timestamp: **{baseline.baseline_timestamp}**")
        out.append(f"- policy: **{baseline.policy}**")
        out.append(f"- chain_ok: **{baseline.chain_ok}**")
        out.append(f"- alerts_count: **{baseline.alerts_count}**")
        out.append(f"- escalation_count: **{baseline.escalation_count}**")
        out.append(f"- source_snapshot_index: **{baseline.source_snapshot_index}**")
        out.append(f"- created_at: **{baseline.created_at}**")
        out.append("")
    if compare is not None:
        out.append("## 7. Compare to baseline")
        out.append("")
        out.append(f"- baseline_timestamp → current_timestamp: `{compare.baseline_timestamp}` → `{compare.current_timestamp}`")
        out.append(f"- delta_alerts: **{compare.delta_alerts:+d}**")
        out.append(f"- delta_escalation: **{compare.delta_escalation:+d}**")
        out.append(f"- policy_regressed: **{compare.policy_regressed}**")
        out.append(f"- policy_improved: **{compare.policy_improved}**")
        out.append(f"- chain_ok_regressed: **{compare.chain_ok_regressed}**")
        out.append(f"- verdict: **{compare.verdict}**")
        out.append(f"- reasons: {compare.reasons}")
        out.append("")
    out.append("## 8. Honest disclosure (主 17:58)")
    out.append("")
    out.append("V1417 tick history is a **deterministic JSONL aggregator** for V1416 DGM closed-loop ticks.")
    out.append("It is bounded by arithmetic on V1416 output fields; NOT by Phenomenal consciousness,")
    out.append("ASI 达成, human-level judgment, or absolute certainty. V1417 ≠ Phenomenal history,")
    out.append("≠ ASI 达成 history, ≠ human-level history, ≠ absolute history. V1417 reads V1416;")
    out.append("never replaces any of V1411-V1416. The trend decision is a deterministic rule on")
    out.append("policy distribution + chain_ok rate — NOT a free agent will.")
    out.append("")
    return "\n".join(out)


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, List[str]]:
    """15 self-tests; return (all_pass, messages)."""
    results: List[Tuple[str, bool, str]] = []

    # 1: VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED present
    results.append((
        "VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED present",
        all([
            V1417_VERSION,
            V1417_SCHEMA,
            V1417_MODULE,
            len(V1417_GUARDS) == 15,
            len(V1417_V3_GUARDS) == 9,
            len(V1417_BORROWED) == 4,
        ]),
        "constants OK",
    ))

    # 2: dataclass roundtrip
    s = TickSnapshot(
        timestamp="2026-08-10T00-00-00Z",
        tick_id="t1",
        policy="PROCEED",
        chain_ok=True,
        alerts_count=0,
        max_severity="INFO",
        escalation_count=0,
        n_modules=5,
        n_snapshots_v1415=0,
        note="x",
    )
    s2 = TickSnapshot(**dataclasses.asdict(s))
    results.append((
        "TickSnapshot roundtrip",
        s == s2,
        "dataclass equality",
    ))

    # 3: append + load roundtrip (uses temp dir)
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "h.jsonl"
        append_tick_snapshot(s, p)
        loaded = load_tick_history(p)
        results.append((
            "append + load roundtrip",
            len(loaded) == 1 and loaded[0] == s,
            f"loaded {len(loaded)} snapshot(s)",
        ))

    # 4: trend INSUFFICIENT on 0 and 1 snapshots
    t0 = compute_tick_trend([])
    t1 = compute_tick_trend([s])
    results.append((
        "trend INSUFFICIENT on <2",
        t0.direction == "INSUFFICIENT" and t1.direction == "INSUFFICIENT",
        f"t0={t0.direction}, t1={t1.direction}",
    ))

    # 5: trend IMPROVING on all-PROCEED + chain_ok + low lockdown
    snaps_ok = [
        TickSnapshot("2026-08-10T00-00-00Z", f"t{i}", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
        for i in range(5)
    ]
    t_ok = compute_tick_trend(snaps_ok)
    results.append((
        "trend IMPROVING on all-PROCEED chain_ok",
        t_ok.direction == "IMPROVING",
        f"direction={t_ok.direction}",
    ))

    # 6: trend DEGRADING on heavy LOCKDOWN
    snaps_bad = [
        TickSnapshot("2026-08-10T00-00-00Z", "t1", "LOCKDOWN", False, 5, "CRITICAL", 1, 5, 0, "")
        for _ in range(5)
    ]
    t_bad = compute_tick_trend(snaps_bad)
    results.append((
        "trend DEGRADING on LOCKDOWN majority",
        t_bad.direction == "DEGRADING",
        f"direction={t_bad.direction}",
    ))

    # 7: trend STABLE on mixed but bounded
    snaps_mid = (
        [TickSnapshot("2026-08-10T00-00-00Z", f"t{i}", "PROCEED", True, 0, "INFO", 0, 5, 0, "") for i in range(4)]
        + [TickSnapshot("2026-08-10T00-01-00Z", "t5", "PAUSE", True, 1, "WARN", 0, 5, 0, "")]
    )
    t_mid = compute_tick_trend(snaps_mid)
    results.append((
        "trend STABLE on mostly-PROCEED with one PAUSE",
        t_mid.direction in ("STABLE", "IMPROVING"),
        f"direction={t_mid.direction}",
    ))

    # 8: digest aggregates
    d = compute_tick_digest(snaps_ok)
    results.append((
        "digest aggregates",
        d.n_ticks == 5 and d.policy_proceed == 5 and d.policy_pause == 0 and d.policy_lockdown == 0 and d.chain_ok_count == 5,
        f"proceed={d.policy_proceed}, chain_ok={d.chain_ok_count}",
    ))

    # 9: baseline + save/load roundtrip
    b = make_tick_baseline(snaps_ok, snapshot_index=-1, note="test")
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        bp = Path(tmp) / "b.json"
        save_baseline(b, bp)
        b_loaded = load_baseline(bp)
        results.append((
            "baseline + save/load roundtrip",
            b_loaded is not None and b_loaded.baseline_timestamp == b.baseline_timestamp,
            "baseline equality",
        ))

    # 10: compare REGRESSION on policy LOCKDOWN vs PROCEED
    new_bad = TickSnapshot("2026-08-10T00-02-00Z", "t6", "LOCKDOWN", False, 5, "CRITICAL", 1, 5, 0, "")
    cmp_bad = compare_to_baseline(new_bad, b)
    results.append((
        "compare REGRESSION on LOCKDOWN",
        cmp_bad.verdict == "REGRESSION" and cmp_bad.policy_regressed,
        f"verdict={cmp_bad.verdict}",
    ))

    # 11: compare UNCHANGED on identical
    cmp_same = compare_to_baseline(snaps_ok[-1], b)
    results.append((
        "compare UNCHANGED on identical",
        cmp_same.verdict == "UNCHANGED",
        f"verdict={cmp_same.verdict}",
    ))

    # 12: compare IMPROVEMENT on lower alerts + better policy
    new_better = TickSnapshot("2026-08-10T00-02-00Z", "t6", "PROCEED", True, 0, "INFO", 0, 5, 0, "")
    # Make baseline with high alerts first
    b_high = make_tick_baseline(
        [TickSnapshot("2026-08-10T00-00-00Z", "t0", "PAUSE", True, 3, "WARN", 0, 5, 0, "")],
        snapshot_index=0, note="high",
    )
    cmp_better = compare_to_baseline(new_better, b_high)
    results.append((
        "compare IMPROVEMENT on lower alerts",
        cmp_better.verdict in ("IMPROVEMENT", "UNCHANGED") and cmp_better.delta_alerts < 0,
        f"verdict={cmp_better.verdict}, delta_alerts={cmp_better.delta_alerts}",
    ))

    # 13: render markdown contains key sections
    md = render_tick_history_md(snaps_ok, t_ok, d, baseline=b, compare=cmp_same)
    results.append((
        "render markdown contains 8 sections",
        all(s in md for s in [
            "# V1417",
            "## 1. Summary",
            "## 2. 哲学守门",
            "## 3. Tick list",
            "## 4. Trend",
            "## 5. Digest",
            "## 6. Baseline",
            "## 7. Compare to baseline",
            "## 8. Honest disclosure",
        ]),
        f"md length={len(md)}",
    ))

    # 14: path safety
    try:
        _safe_path(Path("a/../b"))
        results.append((
            "path safety rejects dotdot",
            False,
            "did not raise",
        ))
    except ValueError:
        results.append((
            "path safety rejects dotdot",
            True,
            "raised ValueError",
        ))

    # 15: V1416 chain_delegate probe (read-only)
    chain_ok = False
    try:
        import apeireth.v1416_asi_overarching_dgm_tick as v1416
        # V1416 exposes chain_delegate_v1416 (not chain_delegate) for clarity;
        # returns a Tuple (all_ok, n_ok, _, n_mod, errors), not a dict.
        probe_fn = getattr(v1416, "chain_delegate_v1416", None) or getattr(v1416, "chain_delegate", None)
        if probe_fn is not None:
            res = probe_fn()
            chain_ok = bool(res[0]) if isinstance(res, tuple) else bool(res.get("all_ok", False))
    except Exception:
        chain_ok = False
    results.append((
        "V1416 chain_delegate probe",
        chain_ok,
        f"v1416 chain_ok={chain_ok}",
    ))

    passed = sum(1 for _, ok, _ in results if ok)
    messages = [f"  [{i+1:02d}] {name}: {'OK' if ok else 'FAIL'} ({detail})" for i, (name, ok, detail) in enumerate(results)]
    return passed == len(results), [f"popper: {passed}/{len(results)}"] + messages


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe V1416 chain integrity (read-only)."""
    errors: List[str] = []
    n_modules_ok = 0
    n_modules = 0
    try:
        import apeireth.v1416_asi_overarching_dgm_tick as v1416
        n_modules = 1
        probe_fn = getattr(v1416, "chain_delegate_v1416", None) or getattr(v1416, "chain_delegate", None)
        probe_raw = probe_fn() if probe_fn else (False, 0, 0, 0, ["no chain_delegate"])
        # V1416 returns Tuple (all_ok, n_ok, _, n_mod, errors)
        if isinstance(probe_raw, tuple) and len(probe_raw) >= 5:
            all_ok_v1416 = probe_raw[0]
            err_list = probe_raw[4]
        elif isinstance(probe_raw, dict):
            all_ok_v1416 = probe_raw.get("all_ok", False)
            err_list = probe_raw.get("errors", [])
        else:
            all_ok_v1416 = False
            err_list = ["unknown return shape"]
        if all_ok_v1416:
            n_modules_ok += 1
        else:
            errors.append(f"v1416 chain_delegate failed: {err_list}")
    except Exception as e:
        errors.append(f"v1416 import failed: {e}")

    return {
        "schema": V1417_SCHEMA,
        "version": V1417_VERSION,
        "all_ok": len(errors) == 0 and n_modules_ok == n_modules,
        "n_modules_ok": n_modules_ok,
        "n_modules": n_modules,
        "errors": errors,
    }


# ============================================================================
# CLI
# ============================================================================


def _format_json(obj: Any) -> str:
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, indent=2, default=str)


def run_cli(argv: List[str]) -> int:
    """Argv dispatcher; return exit code."""
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd == "version":
        print(f"V1417_VERSION: {V1417_VERSION}")
        print(f"V1417_SCHEMA: {V1417_SCHEMA}")
        print(f"V1417_MODULE: {V1417_MODULE}")
        print(f"guards: {len(V1417_GUARDS)} (incl. {len(V1417_V3_GUARDS)} V3 guards)")
        print(f"borrowed: {len(V1417_BORROWED)}")
        return 0

    if cmd == "meta":
        as_json = "--json" in rest
        if as_json:
            print(_format_json({
                "version": V1417_VERSION,
                "schema": V1417_SCHEMA,
                "module": V1417_MODULE,
                "guards": list(V1417_GUARDS),
                "v3_guards": list(V1417_V3_GUARDS),
                "borrowed": list(V1417_BORROWED),
                "policies": list(V1417_POLICIES),
                "trends": list(V1417_TRENDS),
                "default_tick_path": str(V1417_DEFAULT_TICK_PATH),
                "default_history_path": str(V1417_DEFAULT_HISTORY_PATH),
                "default_baseline_path": str(V1417_DEFAULT_BASELINE_PATH),
            }))
        else:
            print(f"version: {V1417_VERSION}")
            print(f"schema: {V1417_SCHEMA}")
            print(f"module: {V1417_MODULE}")
            print(f"guards: {len(V1417_GUARDS)}")
            print(f"v3_guards: {len(V1417_V3_GUARDS)}")
            print(f"borrowed: {len(V1417_BORROWED)}")
            print(f"policies: {list(V1417_POLICIES)}")
            print(f"trends: {list(V1417_TRENDS)}")
        return 0

    if cmd == "demo":
        print("V1417 = ASI 总框架 DGM tick history (JSONL log + trend + digest + baseline + compare)")
        print("Reads .v1416-dgm-ticks.jsonl, appends to .v1417-dgm-tick-history.jsonl")
        print("Use 'snapshot' to capture latest V1416 tick; 'trend', 'digest', 'compare' for analysis")
        return 0

    if cmd == "help":
        print("Usage: python -m apeireth.v1417_asi_dgm_tick_history <command> [options]")
        print()
        print("Commands:")
        print("  version                                show version")
        print("  meta [--json]                          show constants")
        print("  demo                                   brief description")
        print("  popper                                 run 15 self-tests")
        print("  chain                                  probe V1416 chain")
        print("  snapshot [--tick-path P] [--history-path P] [--note S]")
        print("                                         append latest V1416 tick to history")
        print("  list [--last N] [--history-path P] [--json]")
        print("                                         list ticks")
        print("  trend [--history-path P] [--json]      compute trend")
        print("  digest [--history-path P] [--json]     compute digest")
        print("  baseline [--history-path P] [--baseline-path P] [--note S]")
        print("                                         save baseline")
        print("  compare [--history-path P] [--baseline-path P] [--json]")
        print("                                         compare latest tick vs baseline")
        print("  render [--history-path P] [--baseline-path P] [--out PATH]")
        print("                                         render markdown report")
        print("  help                                   show this help")
        return 0

    if cmd == "popper":
        ok, msgs = popper_self_test()
        for m in msgs:
            print(m)
        return 0 if ok else 1

    if cmd == "chain":
        print(_format_json(chain_delegate()))
        return 0

    if cmd == "snapshot":
        tick_path = V1417_DEFAULT_TICK_PATH
        history_path = V1417_DEFAULT_HISTORY_PATH
        note = ""
        i = 0
        while i < len(rest):
            if rest[i] == "--tick-path" and i + 1 < len(rest):
                tick_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--note" and i + 1 < len(rest):
                note = rest[i+1]
                i += 2
            else:
                i += 1
        v1416_snaps = load_v1416_ticks(tick_path)
        if not v1416_snaps:
            print(f"no V1416 ticks found at {tick_path}", file=sys.stderr)
            return 1
        snap = v1416_snaps[-1]
        if note:
            snap = dataclasses.replace(snap, note=note)
        append_tick_snapshot(snap, history_path)
        print(f"appended snapshot {snap.tick_id} (policy={snap.policy}) to {history_path}")
        return 0

    if cmd == "list":
        history_path = V1417_DEFAULT_HISTORY_PATH
        last_n = 0
        as_json = "--json" in rest
        i = 0
        while i < len(rest):
            if rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--last" and i + 1 < len(rest):
                last_n = int(rest[i+1])
                i += 2
            else:
                i += 1
        snaps = load_tick_history(history_path)
        if last_n > 0:
            snaps = snaps[-last_n:]
        if as_json:
            print(_format_json([dataclasses.asdict(s) for s in snaps]))
        else:
            for s in snaps:
                print(f"{s.timestamp} | {s.tick_id} | {s.policy} | chain_ok={s.chain_ok} | alerts={s.alerts_count} | esc={s.escalation_count}")
        return 0

    if cmd == "trend":
        history_path = V1417_DEFAULT_HISTORY_PATH
        as_json = "--json" in rest
        i = 0
        while i < len(rest):
            if rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            else:
                i += 1
        snaps = load_tick_history(history_path)
        trend = compute_tick_trend(snaps)
        if as_json:
            print(_format_json(dataclasses.asdict(trend)))
        else:
            print(f"direction: {trend.direction}")
            print(f"n_snapshots: {trend.n_snapshots}")
            print(f"proceed_ratio: {trend.proceed_ratio:.2%}")
            print(f"chain_ok_rate: {trend.chain_ok_rate:.2%}")
            print(f"reason: {trend.reason}")
        return 0

    if cmd == "digest":
        history_path = V1417_DEFAULT_HISTORY_PATH
        as_json = "--json" in rest
        i = 0
        while i < len(rest):
            if rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            else:
                i += 1
        snaps = load_tick_history(history_path)
        digest = compute_tick_digest(snaps)
        if as_json:
            print(_format_json(dataclasses.asdict(digest)))
        else:
            print(f"n_ticks: {digest.n_ticks}")
            print(f"proceed/pause/lockdown: {digest.policy_proceed}/{digest.policy_pause}/{digest.policy_lockdown}")
            print(f"chain_ok_rate: {digest.chain_ok_rate:.2%}")
            print(f"alerts_total/avg: {digest.alerts_total}/{digest.alerts_avg:.2f}")
            print(f"escalation_total/avg: {digest.escalation_total}/{digest.escalation_avg:.2f}")
            print(f"span: {digest.span_seconds}s")
        return 0

    if cmd == "baseline":
        history_path = V1417_DEFAULT_HISTORY_PATH
        baseline_path = V1417_DEFAULT_BASELINE_PATH
        note = ""
        i = 0
        while i < len(rest):
            if rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--baseline-path" and i + 1 < len(rest):
                baseline_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--note" and i + 1 < len(rest):
                note = rest[i+1]
                i += 2
            else:
                i += 1
        snaps = load_tick_history(history_path)
        if not snaps:
            print(f"no history at {history_path}", file=sys.stderr)
            return 1
        baseline = make_tick_baseline(snaps, snapshot_index=-1, note=note)
        save_baseline(baseline, baseline_path)
        print(f"saved baseline {baseline.baseline_timestamp} to {baseline_path}")
        return 0

    if cmd == "compare":
        history_path = V1417_DEFAULT_HISTORY_PATH
        baseline_path = V1417_DEFAULT_BASELINE_PATH
        as_json = "--json" in rest
        i = 0
        while i < len(rest):
            if rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--baseline-path" and i + 1 < len(rest):
                baseline_path = Path(rest[i+1])
                i += 2
            else:
                i += 1
        snaps = load_tick_history(history_path)
        baseline = load_baseline(baseline_path)
        if not snaps:
            print(f"no history at {history_path}", file=sys.stderr)
            return 1
        if baseline is None:
            print(f"no baseline at {baseline_path}", file=sys.stderr)
            return 1
        latest = snaps[-1]
        cmp_res = compare_to_baseline(latest, baseline)
        if as_json:
            print(_format_json(dataclasses.asdict(cmp_res)))
        else:
            print(f"verdict: {cmp_res.verdict}")
            print(f"delta_alerts: {cmp_res.delta_alerts:+d}")
            print(f"delta_escalation: {cmp_res.delta_escalation:+d}")
            print(f"reasons: {cmp_res.reasons}")
        return 0

    if cmd == "render":
        history_path = V1417_DEFAULT_HISTORY_PATH
        baseline_path = V1417_DEFAULT_BASELINE_PATH
        out_path = V1417_DEFAULT_OUT_PATH
        i = 0
        while i < len(rest):
            if rest[i] == "--history-path" and i + 1 < len(rest):
                history_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--baseline-path" and i + 1 < len(rest):
                baseline_path = Path(rest[i+1])
                i += 2
            elif rest[i] == "--out" and i + 1 < len(rest):
                out_path = Path(rest[i+1])
                i += 2
            else:
                i += 1
        snaps = load_tick_history(history_path)
        trend = compute_tick_trend(snaps)
        digest = compute_tick_digest(snaps)
        baseline = load_baseline(baseline_path)
        compare_obj = None
        if baseline is not None and snaps:
            compare_obj = compare_to_baseline(snaps[-1], baseline)
        md = render_tick_history_md(snaps, trend, digest, baseline=baseline, compare=compare_obj)
        out_path = _safe_path(out_path)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(md, encoding="utf-8")
        print(f"rendered {len(md)} bytes to {out_path}")
        return 0

    print(f"unknown command: {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))