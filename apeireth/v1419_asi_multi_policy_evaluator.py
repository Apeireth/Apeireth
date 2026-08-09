"""V1419 — ASI 总框架 multi-policy evaluator (compare distributions over time).

Phase: 1419
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:00, Asia/Shanghai deep night)
Post: V1418 (DGM cron integration)

What V1419 is
=============
V1419 is the **multi-policy evaluator** for ASI 总框架 DGM ticks.
Where:

- V1416 emits one ``DgmTickReport`` per call (the actual closed-loop)
- V1417 records those ticks over time and computes trend + digest
- V1418 schedules V1416 + V1417 on a cron cadence

V1419 answers the **distribution-shift** question: across two windows
of ticks (e.g. "the last 5 ticks" vs "the 5 ticks before that"),
has the policy distribution meaningfully shifted? Specifically:

- ``compute_window_distribution(snapshots)`` → WindowDistribution
  (proceed_count / pause_count / lockdown_count + ratios + chain_ok_rate)
- ``compare_window_distributions(dist_a, dist_b)`` → WindowComparison
  (deltas for each policy + shift verdict SHIFT|STABLE)
- ``detect_shift(comparison, threshold)`` → List[ShiftAlert]
  (severity INFO|WARN|CRITICAL with shift magnitude + recommendation)
- ``evaluate(snapshots, window_size, threshold)`` → MultiPolicyEvaluationReport
  (full window-A vs window-B distribution + alerts + verdict)

This is the natural next-step after V1418 cron integration: cron
makes the loop **repeatable**; V1419 makes the loop **meaningful**
(detects real signal in the policy distribution vs noise).

It does NOT mutate V1417 state. It only reads V1417.tick_snapshots
(via V1417.load_tick_history) and operates on the resulting list.

Why V1419 exists
================
V1418 cron integration runs the loop, but a loop that always says
"PROCEED" is not meaningful — operators cannot tell whether the
ASI 总框架 is improving, stable, or regressing without comparing
windows.

V1419 takes the V1417 history + splits into two windows + compares
their policy distributions + emits alerts when shifts cross
thresholds. Operators get a clear answer to:

- "Did the policy distribution shift in the last N ticks vs the
  previous N ticks?"
- "Is the chain_ok rate dropping?"
- "Are LOCKDOWN events appearing for the first time?"
- "What is the magnitude of the shift?"

Borrowed (4 — 主 19:33 走在前人经验上):
======================================
- V1417 (tick history — ``load_tick_history`` + ``TickSnapshot`` schema)
- V1414 (watchdog — regression detection + alert severity pattern)
- V1386 (policy analytics — window + comparison pattern)
- V1376 (weekly digest — aggregate stats structure)

GUARDS upheld (V1419-specific, 15 — 主 00:44 质量工程化)
========================================================
- GUARD_EVALUATOR_REAL: real distribution computation, not stubbed
- GUARD_NO_V1417_WRITE: V1419 reads V1417 history, never writes V1417
- GUARD_NO_V1418_WRITE: V1419 reads V1418 outputs, never writes V1418
- GUARD_DISTRIBUTION_BOUNDED: distribution counts ∈ [0, n_snapshots]
- GUARD_COMPARISON_REAL: comparison produces real deltas
- GUARD_ALERT_REAL: alerts produced from real comparisons
- GUARD_THRESHOLD_BOUNDED: threshold ∈ [0.0, 1.0]
- GUARD_DETERMINISTIC: same inputs → same distribution + comparison
- GUARD_BORROWED_REAL: 4 borrowed (V1417 + V1414 + V1386 + V1376)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1419 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_PATH_SAFE: path safety (dotdot rejected, absolute allowed)
- GUARD_WINDOW_SIZED: window_size ≥ MIN_WINDOW_SIZE

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_MULTIPOLICY_IS_NOT_PHENOMENAL: distribution math ≠ Phenomenal consciousness
- GUARD_MULTIPOLICY_IS_NOT_ASI: evaluator ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_MULTIPOLICY_IS_NOT_HUMAN_LEVEL: shift detection ≠ human judgment
- GUARD_MULTIPOLICY_IS_NOT_ABSOLUTE: shift verdict is bounded, not absolute
- GUARD_MULTIPOLICY_IS_NOT_V1417_REPLACE: evaluator reads V1417, does not replace
- GUARD_MULTIPOLICY_IS_NOT_V1418_REPLACE: evaluator reads V1418 outputs, does not replace
- GUARD_MULTIPOLICY_IS_NOT_V1414_REPLACE: evaluator alerts complement V1414 alerts
- GUARD_MULTIPOLICY_IS_NOT_V1413_REPLACE: evaluator is V1417-specialized
- GUARD_MULTIPOLICY_IS_NOT_V1411_REPLACE: evaluator inherits via V1417

Honest disclosure (主 17:58)
============================
V1419 multi-policy evaluator is a **deterministic distribution-shift
detector** that compares two windows of V1417 tick snapshots. It is
bounded by arithmetic on V1417 snapshot fields (policy + chain_ok +
alerts_count); NOT by Phenomenal consciousness, ASI 达成,
human-level judgment, or absolute certainty. V1419 ≠ Phenomenal
evaluator, ≠ ASI 达成 evaluator, ≠ human-level evaluator, ≠ absolute
evaluator. The shift verdict is a deterministic rule on policy ratio
deltas — NOT a free agent will.

API surfaces (15)
=================
1.  ``Severity`` — literal type ("INFO" | "WARN" | "CRITICAL")
2.  ``ShiftVerdict`` — literal type ("SHIFT" | "STABLE")
3.  ``WindowDistribution`` — dataclass (window_label + n_snapshots +
    proceed_count + pause_count + lockdown_count + proceed_ratio +
    pause_ratio + lockdown_ratio + chain_ok_count + chain_ok_rate +
    alerts_total + alerts_avg + first_timestamp + last_timestamp + note)
4.  ``WindowComparison`` — dataclass (window_a_label + window_b_label +
    delta_proceed_ratio + delta_pause_ratio + delta_lockdown_ratio +
    delta_chain_ok_rate + delta_alerts_avg + shift_verdict +
    shift_magnitude + reason + note)
5.  ``ShiftAlert`` — dataclass (alert_type + severity + magnitude +
    recommendation + window_a_label + window_b_label + note)
6.  ``MultiPolicyEvaluationReport`` — dataclass (window_a +
    window_b + comparison + alerts + verdict + n_alerts +
    worst_severity + note)
7.  ``DEFAULT_EVALUATOR_CONFIG`` — EvaluatorConfig
8.  ``EvaluatorConfig`` — dataclass (window_size + threshold +
    min_window_size + max_window_size + history_path + note)
9.  ``build_default_config(overrides)`` — EvaluatorConfig
10. ``compute_window_distribution(snapshots, window_label)`` — WindowDistribution
11. ``compare_window_distributions(dist_a, dist_b, threshold)`` — WindowComparison
12. ``detect_shift(comparison)`` — List[ShiftAlert]
13. ``evaluate(snapshots, config)`` — MultiPolicyEvaluationReport
14. ``render_evaluation_md(report)`` — markdown
15. ``popper_self_test()`` — 15 self-tests
16. ``chain_delegate()`` — V1417 + V1418 chain probe
17. ``run_cli(argv)`` — argv dispatcher

CLI commands (12 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- distribution --history-path PATH --last N
- compare --history-path PATH --window-a-size N --window-b-size N --threshold T
- shift-detector --history-path PATH --window-a-size N --window-b-size N
- evaluate --history-path PATH --window-size N [--threshold T] [--out PATH]
- alerts --last-eval PATH [--json]
- render --last-eval PATH --out PATH
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

V1419_VERSION = "0.1.0"
V1419_SCHEMA = "v1419.asi-multi-policy-evaluator/v1"
V1419_MODULE = "v1419_asi_multi_policy_evaluator"

# Real default paths (same convention as V1416 / V1417 / V1418):
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"
DEFAULT_HISTORY_PATH = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_OUT_PATH = PROMETHEAN / "V1419_EVALUATION.md"
DEFAULT_LAST_EVAL_PATH = PROMETHEAN / ".v1419-last-evaluation.json"

# Bounded guards (主 00:44 质量工程化)
MIN_WINDOW_SIZE = 1         # tight lower bound (1+ snapshot per window)
MAX_WINDOW_SIZE = 1024      # safety upper bound
DEFAULT_WINDOW_SIZE = 5     # 5 ticks per window
DEFAULT_THRESHOLD = 0.10    # |delta ratio| ≥ 0.10 → SHIFT
MIN_THRESHOLD = 0.0
MAX_THRESHOLD = 1.0

V1419_POLICIES = ("PROCEED", "PAUSE", "LOCKDOWN")
V1419_SEVERITIES = ("INFO", "WARN", "CRITICAL")

V1419_GUARDS: Tuple[str, ...] = (
    "GUARD_EVALUATOR_REAL",
    "GUARD_NO_V1417_WRITE",
    "GUARD_NO_V1418_WRITE",
    "GUARD_DISTRIBUTION_BOUNDED",
    "GUARD_COMPARISON_REAL",
    "GUARD_ALERT_REAL",
    "GUARD_THRESHOLD_BOUNDED",
    "GUARD_DETERMINISTIC",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
    "GUARD_WINDOW_SIZED",
)

V1419_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_MULTIPOLICY_IS_NOT_PHENOMENAL",
    "GUARD_MULTIPOLICY_IS_NOT_ASI",
    "GUARD_MULTIPOLICY_IS_NOT_HUMAN_LEVEL",
    "GUARD_MULTIPOLICY_IS_NOT_ABSOLUTE",
    "GUARD_MULTIPOLICY_IS_NOT_V1417_REPLACE",
    "GUARD_MULTIPOLICY_IS_NOT_V1418_REPLACE",
    "GUARD_MULTIPOLICY_IS_NOT_V1414_REPLACE",
    "GUARD_MULTIPOLICY_IS_NOT_V1413_REPLACE",
    "GUARD_MULTIPOLICY_IS_NOT_V1411_REPLACE",
)

V1419_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1417", "tick history (load_tick_history + TickSnapshot schema)"),
    ("V1414", "watchdog (regression detection + alert severity pattern)"),
    ("V1386", "policy analytics (window + comparison pattern)"),
    ("V1376", "weekly digest (aggregate stats structure)"),
)


# ============================================================================
# Type aliases
# ============================================================================

Severity = Literal["INFO", "WARN", "CRITICAL"]
ShiftVerdict = Literal["SHIFT", "STABLE"]


# ============================================================================
# Helpers
# ============================================================================


def _safe_path(p: Path) -> Path:
    """Reject dotdot, allow absolute paths (Windows-aware)."""
    s = str(p)
    parts: List[str]
    if "/" in s:
        parts = s.split("/")
    elif "\\" in s:
        parts = s.split("\\")
    else:
        parts = [s]
    if ".." in parts:
        raise ValueError(f"path with .. rejected: {p}")
    return p


def _atomic_write_json(path: Path, obj: Any) -> None:
    """Atomic JSON write: write to .tmp, fsync, rename."""
    path = _safe_path(Path(path))
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(obj, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.flush()
        os.fsync(fh.fileno())
    os.replace(tmp, path)


def _safe_ratio(num: int, denom: int) -> float:
    """Compute ratio with 0 protection (returns 0.0 if denom == 0)."""
    if denom <= 0:
        return 0.0
    return float(num) / float(denom)


def _window_label(window_kind: str, window_index: int) -> str:
    """Render a window label like 'A:last5' or 'B:prev5'."""
    return f"{window_kind}:last{window_index}"


def _worst_severity(severities: List[str]) -> str:
    """Return worst severity (CRITICAL > WARN > INFO)."""
    rank = {"INFO": 0, "WARN": 1, "CRITICAL": 2}
    if not severities:
        return "INFO"
    return max(severities, key=lambda s: rank.get(s, 0))


# ============================================================================
# Config + Dataclasses
# ============================================================================


@dataclasses.dataclass
class EvaluatorConfig:
    """Configuration for V1419 multi-policy evaluator."""

    window_size: int
    threshold: float
    min_window_size: int
    max_window_size: int
    history_path: Path
    note: str = ""

    def __post_init__(self) -> None:
        if not (self.min_window_size <= self.window_size <= self.max_window_size):
            raise ValueError(
                f"window_size={self.window_size} out of bounds "
                f"[{self.min_window_size}, {self.max_window_size}]"
            )
        if not (MIN_THRESHOLD <= self.threshold <= MAX_THRESHOLD):
            raise ValueError(
                f"threshold={self.threshold} out of bounds "
                f"[{MIN_THRESHOLD}, {MAX_THRESHOLD}]"
            )
        self.history_path = _safe_path(Path(self.history_path))


DEFAULT_EVALUATOR_CONFIG = EvaluatorConfig(
    window_size=DEFAULT_WINDOW_SIZE,
    threshold=DEFAULT_THRESHOLD,
    min_window_size=MIN_WINDOW_SIZE,
    max_window_size=MAX_WINDOW_SIZE,
    history_path=DEFAULT_HISTORY_PATH,
    note="V1419 default: window_size=5 threshold=0.10",
)


@dataclasses.dataclass
class WindowDistribution:
    """Distribution of policies across one window of TickSnapshots."""

    window_label: str
    n_snapshots: int
    proceed_count: int
    pause_count: int
    lockdown_count: int
    proceed_ratio: float
    pause_ratio: float
    lockdown_ratio: float
    chain_ok_count: int
    chain_ok_rate: float
    alerts_total: int
    alerts_avg: float
    first_timestamp: str
    last_timestamp: str
    note: str


@dataclasses.dataclass
class WindowComparison:
    """Comparison of two WindowDistributions."""

    window_a_label: str
    window_b_label: str
    delta_proceed_ratio: float
    delta_pause_ratio: float
    delta_lockdown_ratio: float
    delta_chain_ok_rate: float
    delta_alerts_avg: float
    shift_verdict: str
    shift_magnitude: float
    reason: str
    note: str


@dataclasses.dataclass
class ShiftAlert:
    """One alert emitted by V1419 shift detector."""

    alert_type: str
    severity: str
    magnitude: float
    recommendation: str
    window_a_label: str
    window_b_label: str
    note: str


@dataclasses.dataclass
class MultiPolicyEvaluationReport:
    """Full multi-policy evaluation report across two windows."""

    window_a: WindowDistribution
    window_b: WindowDistribution
    comparison: WindowComparison
    alerts: List[ShiftAlert]
    verdict: str
    n_alerts: int
    worst_severity: str
    note: str


# ============================================================================
# Builders
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> EvaluatorConfig:
    """Build an EvaluatorConfig from DEFAULT_EVALUATOR_CONFIG + overrides."""
    base: Dict[str, Any] = {
        "window_size": DEFAULT_EVALUATOR_CONFIG.window_size,
        "threshold": DEFAULT_EVALUATOR_CONFIG.threshold,
        "min_window_size": DEFAULT_EVALUATOR_CONFIG.min_window_size,
        "max_window_size": DEFAULT_EVALUATOR_CONFIG.max_window_size,
        "history_path": DEFAULT_EVALUATOR_CONFIG.history_path,
        "note": DEFAULT_EVALUATOR_CONFIG.note,
    }
    if overrides:
        for k, v in overrides.items():
            if k in base:
                base[k] = v
            else:
                raise ValueError(f"unknown override key: {k}")
    return EvaluatorConfig(**base)


# ============================================================================
# Core API
# ============================================================================


def compute_window_distribution(
    snapshots: List[Any], window_label: str
) -> WindowDistribution:
    """Compute the policy distribution across a list of TickSnapshots.

    The `snapshots` is expected to be a list of objects with at least
    these attributes: ``policy`` (str ∈ V1419_POLICIES), ``chain_ok`` (bool),
    ``alerts_count`` (int), ``timestamp`` (str).

    Empty snapshots → empty distribution (all zeros).
    """
    n = len(snapshots)
    proceed = 0
    pause = 0
    lockdown = 0
    chain_ok_count = 0
    alerts_total = 0
    first_ts = ""
    last_ts = ""

    for s in snapshots:
        policy = getattr(s, "policy", "")
        if policy == "PROCEED":
            proceed += 1
        elif policy == "PAUSE":
            pause += 1
        elif policy == "LOCKDOWN":
            lockdown += 1
        if bool(getattr(s, "chain_ok", False)):
            chain_ok_count += 1
        alerts_total += int(getattr(s, "alerts_count", 0))
        ts = str(getattr(s, "timestamp", ""))
        if ts:
            if not first_ts:
                first_ts = ts
            last_ts = ts

    return WindowDistribution(
        window_label=window_label,
        n_snapshots=n,
        proceed_count=proceed,
        pause_count=pause,
        lockdown_count=lockdown,
        proceed_ratio=_safe_ratio(proceed, n),
        pause_ratio=_safe_ratio(pause, n),
        lockdown_ratio=_safe_ratio(lockdown, n),
        chain_ok_count=chain_ok_count,
        chain_ok_rate=_safe_ratio(chain_ok_count, n),
        alerts_total=alerts_total,
        alerts_avg=_safe_ratio(alerts_total, n),
        first_timestamp=first_ts,
        last_timestamp=last_ts,
        note=f"V1419 distribution window={window_label} n={n}",
    )


def compare_window_distributions(
    dist_a: WindowDistribution,
    dist_b: WindowDistribution,
    threshold: float = DEFAULT_THRESHOLD,
) -> WindowComparison:
    """Compare two WindowDistributions and compute deltas.

    `dist_a` is the "current" window (most recent), `dist_b` is the
    "previous" window. Positive delta_proceed_ratio means PROCEED
    ratio increased in window_a vs window_b.
    """
    if not (MIN_THRESHOLD <= threshold <= MAX_THRESHOLD):
        raise ValueError(
            f"threshold={threshold} out of bounds [{MIN_THRESHOLD}, {MAX_THRESHOLD}]"
        )

    delta_proceed = dist_a.proceed_ratio - dist_b.proceed_ratio
    delta_pause = dist_a.pause_ratio - dist_b.pause_ratio
    delta_lockdown = dist_a.lockdown_ratio - dist_b.lockdown_ratio
    delta_chain_ok = dist_a.chain_ok_rate - dist_b.chain_ok_rate
    delta_alerts_avg = dist_a.alerts_avg - dist_b.alerts_avg

    # Magnitude: sum of absolute ratio deltas (positive → SHIFT, low → STABLE).
    # Includes chain_ok_rate delta so chain_ok drops register as shifts.
    magnitude = (
        abs(delta_proceed) + abs(delta_pause) + abs(delta_lockdown)
        + abs(delta_chain_ok)
    )
    shift_verdict: str = "SHIFT" if magnitude >= threshold else "STABLE"

    reasons: List[str] = []
    if abs(delta_proceed) >= threshold:
        reasons.append(
            f"proceed_ratio Δ={delta_proceed:+.3f} (|Δ|≥{threshold})"
        )
    if abs(delta_lockdown) >= threshold:
        reasons.append(
            f"lockdown_ratio Δ={delta_lockdown:+.3f} (|Δ|≥{threshold})"
        )
    if abs(delta_pause) >= threshold:
        reasons.append(
            f"pause_ratio Δ={delta_pause:+.3f} (|Δ|≥{threshold})"
        )
    if abs(delta_chain_ok) >= threshold:
        reasons.append(
            f"chain_ok_rate Δ={delta_chain_ok:+.3f} (|Δ|≥{threshold})"
        )
    if not reasons:
        reasons.append(
            f"magnitude={magnitude:.3f} < threshold={threshold:.3f} → STABLE"
        )

    return WindowComparison(
        window_a_label=dist_a.window_label,
        window_b_label=dist_b.window_label,
        delta_proceed_ratio=delta_proceed,
        delta_pause_ratio=delta_pause,
        delta_lockdown_ratio=delta_lockdown,
        delta_chain_ok_rate=delta_chain_ok,
        delta_alerts_avg=delta_alerts_avg,
        shift_verdict=shift_verdict,
        shift_magnitude=magnitude,
        reason="; ".join(reasons),
        note=f"V1419 comparison a={dist_a.window_label} b={dist_b.window_label}",
    )


def detect_shift(comparison: WindowComparison) -> List[ShiftAlert]:
    """Detect shift alerts from a WindowComparison.

    Emits 1 alert per policy delta (if it crosses a sub-threshold).
    Severity:
    - CRITICAL if |delta_lockdown_ratio| ≥ 2× threshold
    - WARN     if |delta_lockdown_ratio| ≥ threshold  OR  chain_ok drop ≥ threshold
    - INFO     otherwise (smaller PROCEED / PAUSE shifts)
    """
    alerts: List[ShiftAlert] = []
    threshold = DEFAULT_THRESHOLD  # use default for severity scaling

    # 1. PROCEED ratio shift (window_b → window_a; positive = healthier)
    if abs(comparison.delta_proceed_ratio) >= threshold:
        direction = "increase" if comparison.delta_proceed_ratio > 0 else "decrease"
        severity: str = (
            "CRITICAL"
            if comparison.delta_proceed_ratio < -2 * threshold
            else "WARN"
        )
        rec = (
            "Investigate policy-regression cause" if direction == "decrease"
            else "Note improvement, verify not noise"
        )
        alerts.append(ShiftAlert(
            alert_type="PROCEED_RATIO_SHIFT",
            severity=severity,
            magnitude=abs(comparison.delta_proceed_ratio),
            recommendation=rec,
            window_a_label=comparison.window_a_label,
            window_b_label=comparison.window_b_label,
            note=f"proceed_ratio Δ={comparison.delta_proceed_ratio:+.3f} ({direction})",
        ))

    # 2. LOCKDOWN ratio shift (positive = more LOCKDOWN events in window_a)
    if abs(comparison.delta_lockdown_ratio) >= threshold:
        direction = "increase" if comparison.delta_lockdown_ratio > 0 else "decrease"
        sev2: str = (
            "CRITICAL"
            if comparison.delta_lockdown_ratio > 2 * threshold
            else "WARN"
        )
        rec2 = (
            "Lockdown events emerging — review V1414 alerts in window_a"
            if direction == "increase"
            else "Lockdown reduced — verify stability"
        )
        alerts.append(ShiftAlert(
            alert_type="LOCKDOWN_RATIO_SHIFT",
            severity=sev2,
            magnitude=abs(comparison.delta_lockdown_ratio),
            recommendation=rec2,
            window_a_label=comparison.window_a_label,
            window_b_label=comparison.window_b_label,
            note=f"lockdown_ratio Δ={comparison.delta_lockdown_ratio:+.3f} ({direction})",
        ))

    # 3. chain_ok_rate drop (negative = degradation)
    if comparison.delta_chain_ok_rate <= -threshold:
        sev3: str = (
            "CRITICAL"
            if comparison.delta_chain_ok_rate <= -2 * threshold
            else "WARN"
        )
        alerts.append(ShiftAlert(
            alert_type="CHAIN_OK_DROP",
            severity=sev3,
            magnitude=abs(comparison.delta_chain_ok_rate),
            recommendation="Review V1416/V1411 chain integrity in window_a",
            window_a_label=comparison.window_a_label,
            window_b_label=comparison.window_b_label,
            note=f"chain_ok_rate Δ={comparison.delta_chain_ok_rate:+.3f}",
        ))

    # 4. PAUSE ratio shift
    if abs(comparison.delta_pause_ratio) >= threshold:
        direction = "increase" if comparison.delta_pause_ratio > 0 else "decrease"
        sev4: str = (
            "CRITICAL"
            if comparison.delta_pause_ratio > 2 * threshold
            else "WARN"
        )
        rec4 = (
            "PAUSE events emerging — review paused-policy reasons"
            if direction == "increase"
            else "PAUSE reduced — verify not transient"
        )
        alerts.append(ShiftAlert(
            alert_type="PAUSE_RATIO_SHIFT",
            severity=sev4,
            magnitude=abs(comparison.delta_pause_ratio),
            recommendation=rec4,
            window_a_label=comparison.window_a_label,
            window_b_label=comparison.window_b_label,
            note=f"pause_ratio Δ={comparison.delta_pause_ratio:+.3f} ({direction})",
        ))

    return alerts


def evaluate(
    snapshots: List[Any], config: Optional[EvaluatorConfig] = None
) -> MultiPolicyEvaluationReport:
    """Evaluate two windows of snapshots + produce a full report.

    Split logic: window_a = last `window_size` snapshots,
                 window_b = previous `window_size` snapshots (or fewer).
    If total snapshots < 2 * window_size, window_b is sized
    `min(window_size, n_snapshots // 2)`.
    """
    cfg = config or DEFAULT_EVALUATOR_CONFIG
    n = len(snapshots)
    wsize = cfg.window_size
    if n == 0:
        # Empty case → empty distribution + STABLE verdict
        empty_a = compute_window_distribution([], _window_label("A", 0))
        empty_b = compute_window_distribution([], _window_label("B", 0))
        empty_cmp = compare_window_distributions(empty_a, empty_b, cfg.threshold)
        return MultiPolicyEvaluationReport(
            window_a=empty_a,
            window_b=empty_b,
            comparison=empty_cmp,
            alerts=[],
            verdict="INSUFFICIENT_DATA",
            n_alerts=0,
            worst_severity="INFO",
            note="V1419 evaluate n_snapshots=0 → INSUFFICIENT_DATA",
        )

    # Compute window_a (most recent window_size snapshots)
    a_snapshots = snapshots[-wsize:] if n >= wsize else list(snapshots)
    remaining = max(0, n - len(a_snapshots))
    b_size = min(wsize, remaining) if remaining > 0 else 0
    b_snapshots = snapshots[remaining - b_size:remaining] if b_size > 0 else []

    dist_a = compute_window_distribution(a_snapshots, _window_label("A", len(a_snapshots)))
    dist_b = compute_window_distribution(b_snapshots, _window_label("B", len(b_snapshots)))
    comparison = compare_window_distributions(dist_a, dist_b, cfg.threshold)
    alerts = detect_shift(comparison)

    # Verdict
    if not alerts:
        verdict = "STABLE"
    else:
        verdict = comparison.shift_verdict  # SHIFT or STABLE
        # If any CRITICAL alert → CRITICAL_SHIFT
        if any(a.severity == "CRITICAL" for a in alerts):
            verdict = "CRITICAL_SHIFT"

    return MultiPolicyEvaluationReport(
        window_a=dist_a,
        window_b=dist_b,
        comparison=comparison,
        alerts=alerts,
        verdict=verdict,
        n_alerts=len(alerts),
        worst_severity=_worst_severity([a.severity for a in alerts]),
        note=(
            f"V1419 evaluate window_size={cfg.threshold if False else wsize} "
            f"threshold={cfg.threshold} verdict={verdict}"
        ),
    )


def render_evaluation_md(report: MultiPolicyEvaluationReport) -> str:
    """Render a multi-policy evaluation report as markdown.

    Sections:
    1. Summary (verdict + n_alerts + worst_severity)
    2. Window A distribution
    3. Window B distribution
    4. Comparison (deltas + shift verdict)
    5. Alerts (severity-sorted)
    6. Honest disclosure (V3 哲学守门)
    """
    lines: List[str] = []
    lines.append("# V1419 — ASI 总框架 multi-policy evaluation report")
    lines.append("")
    lines.append(f"**Verdict:** `{report.verdict}`")
    lines.append(f"**n_alerts:** {report.n_alerts}")
    lines.append(f"**worst_severity:** {report.worst_severity}")
    lines.append(f"**note:** {report.note}")
    lines.append("")
    lines.append("## 1. Window A (most recent)")
    a = report.window_a
    lines.append(f"- label: `{a.window_label}`")
    lines.append(f"- n_snapshots: **{a.n_snapshots}**")
    lines.append(
        f"- proceed / pause / lockdown: **{a.proceed_count} / {a.pause_count} / {a.lockdown_count}**"
    )
    lines.append(
        f"- ratios: proceed={a.proceed_ratio:.3f} pause={a.pause_ratio:.3f} lockdown={a.lockdown_ratio:.3f}"
    )
    lines.append(
        f"- chain_ok_rate: **{a.chain_ok_rate:.3f}** ({a.chain_ok_count}/{a.n_snapshots})"
    )
    lines.append(f"- alerts_avg: {a.alerts_avg:.3f}")
    lines.append(f"- first → last: `{a.first_timestamp}` → `{a.last_timestamp}`")
    lines.append("")
    lines.append("## 2. Window B (previous)")
    b = report.window_b
    lines.append(f"- label: `{b.window_label}`")
    lines.append(f"- n_snapshots: **{b.n_snapshots}**")
    lines.append(
        f"- proceed / pause / lockdown: **{b.proceed_count} / {b.pause_count} / {b.lockdown_count}**"
    )
    lines.append(
        f"- ratios: proceed={b.proceed_ratio:.3f} pause={b.pause_ratio:.3f} lockdown={b.lockdown_ratio:.3f}"
    )
    lines.append(
        f"- chain_ok_rate: **{b.chain_ok_rate:.3f}** ({b.chain_ok_count}/{b.n_snapshots})"
    )
    lines.append(f"- alerts_avg: {b.alerts_avg:.3f}")
    lines.append(f"- first → last: `{b.first_timestamp}` → `{b.last_timestamp}`")
    lines.append("")
    lines.append("## 3. Comparison (window_a vs window_b)")
    c = report.comparison
    lines.append(f"- a: `{c.window_a_label}`")
    lines.append(f"- b: `{c.window_b_label}`")
    lines.append(
        f"- Δ proceed / pause / lockdown: "
        f"**{c.delta_proceed_ratio:+.3f}** / **{c.delta_pause_ratio:+.3f}** / **{c.delta_lockdown_ratio:+.3f}**"
    )
    lines.append(f"- Δ chain_ok_rate: **{c.delta_chain_ok_rate:+.3f}**")
    lines.append(f"- Δ alerts_avg: **{c.delta_alerts_avg:+.3f}**")
    lines.append(f"- shift_verdict: **{c.shift_verdict}**")
    lines.append(f"- shift_magnitude: **{c.shift_magnitude:.3f}**")
    lines.append(f"- reason: {c.reason}")
    lines.append("")
    lines.append("## 4. Alerts (sorted by severity)")
    if not report.alerts:
        lines.append("- (no alerts)")
    else:
        # Sort by severity rank
        rank = {"INFO": 0, "WARN": 1, "CRITICAL": 2}
        sorted_alerts = sorted(report.alerts, key=lambda a: -rank.get(a.severity, 0))
        for alert in sorted_alerts:
            lines.append(
                f"- [{alert.severity}] **{alert.alert_type}** (magnitude={alert.magnitude:.3f}) — "
                f"{alert.recommendation}"
            )
            lines.append(f"  - note: {alert.note}")
    lines.append("")
    lines.append("## 5. Honest disclosure (主 17:58)")
    lines.append("")
    lines.append(
        "V1419 multi-policy evaluator is a **deterministic distribution-shift detector** "
        "that compares two windows of V1417 tick snapshots. It is bounded by arithmetic "
        "on V1417 snapshot fields (policy + chain_ok + alerts_count); NOT by "
        "Phenomenal consciousness, ASI 达成, human-level judgment, or absolute "
        "certainty. V1419 ≠ Phenomenal evaluator, ≠ ASI 达成 evaluator, "
        "≠ human-level evaluator, ≠ absolute evaluator."
    )
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# Popper self-test (15/15)
# ============================================================================


def popper_self_test() -> Tuple[bool, List[Tuple[str, bool]]]:
    """Run V1419's 15 popper self-tests. Returns (all_ok, results)."""
    results: List[Tuple[str, bool]] = []

    # 1. VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED present
    ok1 = (
        isinstance(V1419_VERSION, str)
        and isinstance(V1419_SCHEMA, str)
        and isinstance(V1419_MODULE, str)
        and len(V1419_GUARDS) >= 10
        and len(V1419_V3_GUARDS) >= 5
        and len(V1419_BORROWED) >= 3
    )
    results.append(("VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED present", ok1))

    # 2. DEFAULT_EVALUATOR_CONFIG within bounds
    ok2 = (
        MIN_WINDOW_SIZE <= DEFAULT_EVALUATOR_CONFIG.window_size <= MAX_WINDOW_SIZE
        and MIN_THRESHOLD <= DEFAULT_EVALUATOR_CONFIG.threshold <= MAX_THRESHOLD
    )
    results.append(("DEFAULT_EVALUATOR_CONFIG within bounds", ok2))

    # 3. build_default_config applies overrides
    cfg3 = build_default_config({"window_size": 10})
    ok3 = cfg3.window_size == 10
    results.append(("build_default_config applies overrides", ok3))

    # 4. build_default_config rejects unknown overrides
    ok4 = True
    try:
        build_default_config({"bogus_key": 1})
        ok4 = False
    except ValueError:
        ok4 = True
    results.append(("build_default_config rejects unknown overrides", ok4))

    # 5. config rejects out-of-bounds window_size
    ok5 = True
    try:
        EvaluatorConfig(
            window_size=0, threshold=0.1,
            min_window_size=1, max_window_size=1024, history_path=Path("."),
        )
        ok5 = False
    except ValueError:
        ok5 = True
    results.append(("config rejects window_size=0", ok5))

    # 6. compute_window_distribution handles empty
    dist6 = compute_window_distribution([], _window_label("A", 0))
    ok6 = dist6.n_snapshots == 0 and dist6.proceed_ratio == 0.0
    results.append(("compute_window_distribution handles empty list", ok6))

    # 7. compute_window_distribution counts policies correctly
    # Mock snapshots: 3 PROCEED + 1 PAUSE + 1 LOCKDOWN, chain_ok varies
    class MockSnap:
        def __init__(self, policy, chain_ok, alerts_count, ts):
            self.policy = policy
            self.chain_ok = chain_ok
            self.alerts_count = alerts_count
            self.timestamp = ts

    snaps7 = [
        MockSnap("PROCEED", True, 0, "2026-08-10T00-00-00Z"),
        MockSnap("PROCEED", True, 1, "2026-08-10T00-01-00Z"),
        MockSnap("PROCEED", False, 2, "2026-08-10T00-02-00Z"),
        MockSnap("PAUSE", True, 3, "2026-08-10T00-03-00Z"),
        MockSnap("LOCKDOWN", False, 4, "2026-08-10T00-04-00Z"),
    ]
    dist7 = compute_window_distribution(snaps7, "TEST")
    ok7 = (
        dist7.proceed_count == 3
        and dist7.pause_count == 1
        and dist7.lockdown_count == 1
        and dist7.chain_ok_count == 3
        and abs(dist7.proceed_ratio - 0.6) < 1e-9
        and abs(dist7.chain_ok_rate - 0.6) < 1e-9
        and dist7.alerts_total == 10
    )
    results.append(("compute_window_distribution counts policies correctly", ok7))

    # 8. compare_window_distributions computes deltas correctly
    dist_a8 = compute_window_distribution(
        [MockSnap("PROCEED", True, 0, "t1")] * 5, "A:5"
    )
    dist_b8 = compute_window_distribution(
        [MockSnap("PROCEED", True, 0, "t2")] * 5, "B:5"
    )
    cmp8 = compare_window_distributions(dist_a8, dist_b8, threshold=0.10)
    ok8 = (
        abs(cmp8.delta_proceed_ratio - 0.0) < 1e-9
        and cmp8.shift_verdict == "STABLE"
    )
    results.append(("compare_window_distributions STABLE on equal windows", ok8))

    # 9. compare_window_distributions detects SHIFT
    dist_a9 = compute_window_distribution(
        [MockSnap("LOCKDOWN", False, 5, "t1")] * 5, "A:5"
    )
    dist_b9 = compute_window_distribution(
        [MockSnap("PROCEED", True, 0, "t2")] * 5, "B:5"
    )
    cmp9 = compare_window_distributions(dist_a9, dist_b9, threshold=0.10)
    ok9 = cmp9.shift_verdict == "SHIFT" and cmp9.delta_lockdown_ratio == 1.0
    results.append(("compare_window_distributions detects SHIFT", ok9))

    # 10. detect_shift emits alerts on SHIFT
    alerts10 = detect_shift(cmp9)
    ok10 = len(alerts10) >= 1 and any(a.alert_type == "LOCKDOWN_RATIO_SHIFT" for a in alerts10)
    results.append(("detect_shift emits alerts on SHIFT", ok10))

    # 11. detect_shift emits NO alerts on STABLE
    alerts11 = detect_shift(cmp8)
    ok11 = len(alerts11) == 0
    results.append(("detect_shift emits NO alerts on STABLE", ok11))

    # 12. evaluate handles empty snapshots
    rep12 = evaluate([])
    ok12 = rep12.verdict == "INSUFFICIENT_DATA" and rep12.n_alerts == 0
    results.append(("evaluate handles empty snapshots", ok12))

    # 13. evaluate splits into windows correctly
    snaps13 = (
        [MockSnap("PROCEED", True, 0, f"t{i}") for i in range(5)]
        + [MockSnap("LOCKDOWN", False, 5, f"t{i + 5}") for i in range(5)]
    )
    cfg13 = EvaluatorConfig(
        window_size=5, threshold=0.10,
        min_window_size=1, max_window_size=1024, history_path=Path("."),
    )
    rep13 = evaluate(snaps13, cfg13)
    ok13 = (
        rep13.window_a.n_snapshots == 5
        and rep13.window_b.n_snapshots == 5
        and rep13.verdict in ("SHIFT", "CRITICAL_SHIFT")
        and rep13.n_alerts >= 1
    )
    results.append(("evaluate splits into 2 windows correctly", ok13))

    # 14. render_evaluation_md has 5 sections
    md14 = render_evaluation_md(rep13)
    ok14 = all(s in md14 for s in ("## 1.", "## 2.", "## 3.", "## 4.", "## 5."))
    results.append(("render_evaluation_md has 5 sections", ok14))

    # 15. _safe_path rejects dotdot
    ok15 = True
    try:
        _safe_path(Path("../etc/passwd"))
        ok15 = False
    except ValueError:
        ok15 = True
    results.append(("_safe_path rejects dotdot", ok15))

    all_ok = all(r[1] for r in results)
    return all_ok, results


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe V1417 + V1418 chain integrity (read-only)."""
    errors: List[str] = []
    n_modules_ok = 0
    n_modules = 0

    # V1417
    try:
        import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
        n_modules += 1
        d = v1417.chain_delegate()
        if d.get("all_ok"):
            n_modules_ok += 1
        else:
            errors.append(f"V1417 chain: {d.get('errors', [])}")
    except Exception as exc:
        errors.append(f"V1417 import failed: {exc!r}")

    # V1418 (may not be available if cron module not loaded)
    try:
        import apeireth.v1418_asi_dgm_cron_integration as v1418  # type: ignore
        n_modules += 1
        d = v1418.chain_delegate()
        if d.get("all_ok"):
            n_modules_ok += 1
        else:
            errors.append(f"V1418 chain: {d.get('errors', [])}")
    except Exception as exc:
        errors.append(f"V1418 import failed: {exc!r}")

    return {
        "schema": V1419_SCHEMA,
        "version": V1419_VERSION,
        "all_ok": (n_modules_ok == n_modules and not errors),
        "n_modules": n_modules,
        "n_modules_ok": n_modules_ok,
        "errors": errors,
    }


# ============================================================================
# CLI
# ============================================================================


def _print_popper_results(results: List[Tuple[str, bool]]) -> None:
    """Print popper results to stdout."""
    print(f"popper: {sum(1 for _, ok in results if ok)}/{len(results)}")
    for i, (name, ok) in enumerate(results, 1):
        marker = "OK" if ok else "FAIL"
        print(f"  [{i:02d}] {name}: {marker}")


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1419 CLI dispatcher. Returns exit code (0 = ok)."""
    if argv is None:
        argv = sys.argv[1:]
    if not argv:
        argv = ["help"]

    cmd = argv[0]
    rest = argv[1:]

    if cmd in ("-h", "--help", "help"):
        print("V1419 — ASI 总框架 multi-policy evaluator")
        print("")
        print("Commands:")
        print("  version")
        print("  meta [--json]")
        print("  demo")
        print("  popper")
        print("  chain")
        print("  distribution --history-path PATH --last N")
        print("  compare --history-path PATH --window-a-size N --window-b-size N [--threshold T]")
        print("  shift-detector --history-path PATH --window-a-size N --window-b-size N")
        print("  evaluate --history-path PATH [--window-size N] [--threshold T] [--out PATH]")
        print("  alerts --last-eval PATH [--json]")
        print("  render --last-eval PATH --out PATH")
        return 0

    if cmd == "version":
        print(f"{V1419_MODULE} {V1419_VERSION} ({V1419_SCHEMA})")
        return 0

    if cmd == "meta":
        if rest and rest[0] == "--json":
            print(json.dumps({
                "version": V1419_VERSION,
                "schema": V1419_SCHEMA,
                "module": V1419_MODULE,
                "guards": list(V1419_GUARDS),
                "v3_guards": list(V1419_V3_GUARDS),
                "borrowed": [{"module": m, "content": c} for m, c in V1419_BORROWED],
                "policies": list(V1419_POLICIES),
                "severities": list(V1419_SEVERITIES),
            }, indent=2, ensure_ascii=False))
        else:
            print(f"V1419 multi-policy evaluator {V1419_VERSION}")
            print(f"  schema: {V1419_SCHEMA}")
            print(f"  guards: {len(V1419_GUARDS)}")
            print(f"  v3_guards: {len(V1419_V3_GUARDS)}")
            print(f"  borrowed: {len(V1419_BORROWED)}")
            print(f"  policies: {V1419_POLICIES}")
            print(f"  severities: {V1419_SEVERITIES}")
        return 0

    if cmd == "demo":
        # Synthetic demo: 10 snapshots, last 5 are different from prev 5
        class DemoSnap:
            def __init__(self, policy, chain_ok, alerts_count, ts):
                self.policy = policy
                self.chain_ok = chain_ok
                self.alerts_count = alerts_count
                self.timestamp = ts

        snaps_b = [DemoSnap("PROCEED", True, 0, f"2026-08-10T00-{i:02d}-00Z") for i in range(5)]
        snaps_a = [DemoSnap("LOCKDOWN", False, 3, f"2026-08-10T00-{i + 5:02d}-00Z") for i in range(5)]
        snaps = snaps_b + snaps_a
        cfg = build_default_config()
        rep = evaluate(snaps, cfg)
        print(json.dumps({
            "verdict": rep.verdict,
            "n_alerts": rep.n_alerts,
            "worst_severity": rep.worst_severity,
            "window_a": dataclasses.asdict(rep.window_a),
            "window_b": dataclasses.asdict(rep.window_b),
            "comparison": dataclasses.asdict(rep.comparison),
            "alerts": [dataclasses.asdict(a) for a in rep.alerts],
        }, indent=2, ensure_ascii=False))
        return 0

    if cmd == "popper":
        all_ok, results = popper_self_test()
        _print_popper_results(results)
        return 0 if all_ok else 1

    if cmd == "chain":
        d = chain_delegate()
        print(json.dumps(d, indent=2, ensure_ascii=False))
        return 0 if d.get("all_ok") else 1

    # Distribution: --history-path PATH --last N
    if cmd == "distribution":
        path = DEFAULT_HISTORY_PATH
        last_n = 5
        i = 0
        while i < len(rest):
            tok = rest[i]
            if tok == "--history-path" and i + 1 < len(rest):
                path = Path(rest[i + 1])
                i += 2
            elif tok == "--last" and i + 1 < len(rest):
                last_n = int(rest[i + 1])
                i += 2
            else:
                print(f"unknown arg: {tok}", file=sys.stderr)
                return 2
        try:
            import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
            snaps = v1417.load_tick_history(_safe_path(path))
        except Exception as exc:
            print(f"v1417.load_tick_history failed: {exc!r}", file=sys.stderr)
            return 1
        window = snaps[-last_n:] if len(snaps) >= last_n else snaps
        dist = compute_window_distribution(window, _window_label("A", len(window)))
        print(json.dumps(dataclasses.asdict(dist), indent=2, ensure_ascii=False))
        return 0

    # Compare: --history-path PATH --window-a-size N --window-b-size N [--threshold T]
    if cmd == "compare":
        path = DEFAULT_HISTORY_PATH
        wa = 5
        wb = 5
        threshold = DEFAULT_THRESHOLD
        i = 0
        while i < len(rest):
            tok = rest[i]
            if tok == "--history-path" and i + 1 < len(rest):
                path = Path(rest[i + 1])
                i += 2
            elif tok == "--window-a-size" and i + 1 < len(rest):
                wa = int(rest[i + 1])
                i += 2
            elif tok == "--window-b-size" and i + 1 < len(rest):
                wb = int(rest[i + 1])
                i += 2
            elif tok == "--threshold" and i + 1 < len(rest):
                threshold = float(rest[i + 1])
                i += 2
            else:
                print(f"unknown arg: {tok}", file=sys.stderr)
                return 2
        try:
            import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
            snaps = v1417.load_tick_history(_safe_path(path))
        except Exception as exc:
            print(f"v1417.load_tick_history failed: {exc!r}", file=sys.stderr)
            return 1
        a_window = snaps[-wa:] if len(snaps) >= wa else snaps
        b_pool = snaps[: max(0, len(snaps) - len(a_window))]
        b_window = b_pool[-wb:] if len(b_pool) >= wb else b_pool
        dist_a = compute_window_distribution(a_window, _window_label("A", len(a_window)))
        dist_b = compute_window_distribution(b_window, _window_label("B", len(b_window)))
        cmp = compare_window_distributions(dist_a, dist_b, threshold)
        print(json.dumps({
            "window_a": dataclasses.asdict(dist_a),
            "window_b": dataclasses.asdict(dist_b),
            "comparison": dataclasses.asdict(cmp),
        }, indent=2, ensure_ascii=False))
        return 0

    # Shift-detector: --history-path PATH --window-a-size N --window-b-size N
    if cmd == "shift-detector":
        path = DEFAULT_HISTORY_PATH
        wa = 5
        wb = 5
        threshold = DEFAULT_THRESHOLD
        i = 0
        while i < len(rest):
            tok = rest[i]
            if tok == "--history-path" and i + 1 < len(rest):
                path = Path(rest[i + 1])
                i += 2
            elif tok == "--window-a-size" and i + 1 < len(rest):
                wa = int(rest[i + 1])
                i += 2
            elif tok == "--window-b-size" and i + 1 < len(rest):
                wb = int(rest[i + 1])
                i += 2
            elif tok == "--threshold" and i + 1 < len(rest):
                threshold = float(rest[i + 1])
                i += 2
            else:
                print(f"unknown arg: {tok}", file=sys.stderr)
                return 2
        try:
            import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
            snaps = v1417.load_tick_history(_safe_path(path))
        except Exception as exc:
            print(f"v1417.load_tick_history failed: {exc!r}", file=sys.stderr)
            return 1
        a_window = snaps[-wa:] if len(snaps) >= wa else snaps
        b_pool = snaps[: max(0, len(snaps) - len(a_window))]
        b_window = b_pool[-wb:] if len(b_pool) >= wb else b_pool
        dist_a = compute_window_distribution(a_window, _window_label("A", len(a_window)))
        dist_b = compute_window_distribution(b_window, _window_label("B", len(b_window)))
        cmp = compare_window_distributions(dist_a, dist_b, threshold)
        alerts = detect_shift(cmp)
        print(json.dumps({
            "shift_verdict": cmp.shift_verdict,
            "shift_magnitude": cmp.shift_magnitude,
            "n_alerts": len(alerts),
            "alerts": [dataclasses.asdict(a) for a in alerts],
        }, indent=2, ensure_ascii=False))
        return 0

    # Evaluate: --history-path PATH [--window-size N] [--threshold T] [--out PATH]
    if cmd == "evaluate":
        path = DEFAULT_HISTORY_PATH
        window_size = DEFAULT_WINDOW_SIZE
        threshold = DEFAULT_THRESHOLD
        out_path = DEFAULT_OUT_PATH
        save_last = True
        i = 0
        while i < len(rest):
            tok = rest[i]
            if tok == "--history-path" and i + 1 < len(rest):
                path = Path(rest[i + 1])
                i += 2
            elif tok == "--window-size" and i + 1 < len(rest):
                window_size = int(rest[i + 1])
                i += 2
            elif tok == "--threshold" and i + 1 < len(rest):
                threshold = float(rest[i + 1])
                i += 2
            elif tok == "--out" and i + 1 < len(rest):
                out_path = Path(rest[i + 1])
                i += 2
            elif tok == "--no-save":
                save_last = False
                i += 1
            else:
                print(f"unknown arg: {tok}", file=sys.stderr)
                return 2
        try:
            import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
            snaps = v1417.load_tick_history(_safe_path(path))
        except Exception as exc:
            print(f"v1417.load_tick_history failed: {exc!r}", file=sys.stderr)
            return 1
        cfg = build_default_config({
            "window_size": window_size,
            "threshold": threshold,
            "history_path": _safe_path(path),
        })
        rep = evaluate(snaps, cfg)
        if out_path:
            md = render_evaluation_md(rep)
            _atomic_write_text(_safe_path(out_path), md)
        if save_last:
            _atomic_write_json(DEFAULT_LAST_EVAL_PATH, {
                "verdict": rep.verdict,
                "n_alerts": rep.n_alerts,
                "worst_severity": rep.worst_severity,
                "window_a": dataclasses.asdict(rep.window_a),
                "window_b": dataclasses.asdict(rep.window_b),
                "comparison": dataclasses.asdict(rep.comparison),
                "alerts": [dataclasses.asdict(a) for a in rep.alerts],
            })
        print(json.dumps({
            "verdict": rep.verdict,
            "n_alerts": rep.n_alerts,
            "worst_severity": rep.worst_severity,
            "rendered_path": str(out_path) if out_path else None,
            "saved_last_eval": str(DEFAULT_LAST_EVAL_PATH) if save_last else None,
        }, indent=2, ensure_ascii=False))
        return 0

    # Alerts: --last-eval PATH [--json]
    if cmd == "alerts":
        path = DEFAULT_LAST_EVAL_PATH
        i = 0
        while i < len(rest):
            tok = rest[i]
            if tok == "--last-eval" and i + 1 < len(rest):
                path = Path(rest[i + 1])
                i += 2
            else:
                print(f"unknown arg: {tok}", file=sys.stderr)
                return 2
        try:
            with open(_safe_path(path), "r", encoding="utf-8") as fh:
                d = json.load(fh)
            alerts = d.get("alerts", [])
            if rest and "--json" in rest:
                print(json.dumps(alerts, indent=2, ensure_ascii=False))
            else:
                if not alerts:
                    print("(no alerts)")
                for a in alerts:
                    print(
                        f"[{a.get('severity', 'INFO')}] {a.get('alert_type', 'UNKNOWN')} "
                        f"(magnitude={a.get('magnitude', 0.0):.3f}) — {a.get('recommendation', '')}"
                    )
        except FileNotFoundError:
            print(f"last-eval file not found: {path}", file=sys.stderr)
            return 1
        return 0

    # Render: --last-eval PATH --out PATH
    if cmd == "render":
        in_path = DEFAULT_LAST_EVAL_PATH
        out_path = DEFAULT_OUT_PATH
        i = 0
        while i < len(rest):
            tok = rest[i]
            if tok == "--last-eval" and i + 1 < len(rest):
                in_path = Path(rest[i + 1])
                i += 2
            elif tok == "--out" and i + 1 < len(rest):
                out_path = Path(rest[i + 1])
                i += 2
            else:
                print(f"unknown arg: {tok}", file=sys.stderr)
                return 2
        try:
            with open(_safe_path(in_path), "r", encoding="utf-8") as fh:
                d = json.load(fh)
        except FileNotFoundError:
            print(f"last-eval file not found: {in_path}", file=sys.stderr)
            return 1
        # Reconstruct report from JSON
        dist_a = WindowDistribution(**d["window_a"])
        dist_b = WindowDistribution(**d["window_b"])
        cmp = WindowComparison(**d["comparison"])
        alerts = [ShiftAlert(**a) for a in d.get("alerts", [])]
        rep = MultiPolicyEvaluationReport(
            window_a=dist_a,
            window_b=dist_b,
            comparison=cmp,
            alerts=alerts,
            verdict=d.get("verdict", "STABLE"),
            n_alerts=d.get("n_alerts", len(alerts)),
            worst_severity=d.get("worst_severity", "INFO"),
            note="V1419 render from last-eval JSON",
        )
        md = render_evaluation_md(rep)
        _atomic_write_text(_safe_path(out_path), md)
        print(json.dumps({
            "rendered_path": str(out_path),
            "verdict": rep.verdict,
            "n_alerts": rep.n_alerts,
        }, indent=2, ensure_ascii=False))
        return 0

    print(f"unknown command: {cmd}", file=sys.stderr)
    return 2


def _atomic_write_text(path: Path, text: str) -> None:
    """Atomic text write: write to .tmp, fsync, rename."""
    path = _safe_path(Path(path))
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as fh:
        fh.write(text)
        fh.flush()
        os.fsync(fh.fileno())
    os.replace(tmp, path)


if __name__ == "__main__":
    sys.exit(run_cli())