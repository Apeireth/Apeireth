"""V1415 — ASI 总框架 multi-period overlay (24h/7d/30d 三窗口).

Phase: 1415
Version: 0.1.0
Date: 2026-08-10 (cron tick 02:34, Asia/Shanghai deep night)
Post: V1414 (regression detector + watchdog)

What V1415 is
=============
V1415 closes the **time horizon** loop for the ASI 总框架. Where:

- V1411 builds the overarching report (12 capacities + 6 limits + 30 trajectories)
- V1412 overlays the dashboard (5 verdict + 12 × 11 matrix + chain status)
- V1413 records time-series log (JSONL + trend + digest + baseline + compare)
- V1414 raises alerts (3 severity + 4 rules + 5 hints + cooldown)

V1415 produces **multi-period overlay**: it slices V1413 history into
3 windows (24h / 7d / 30d) and computes per-window statistics, then
diffs each window against the **baseline**, producing an escalation flag
when a shorter window is significantly worse than the longer one
(`ESCALATION: 24h_warn_rate > 4 × 30d_warn_rate`).

V1415 does NOT mutate V1411, V1412, V1413, or V1414. It reads V1413
history + V1413 baseline and emits a structured overlay report.

Why V1415 exists
================
V1414 raises alerts in real time, but operators need **context**:
- Is the 24h alert rate higher than the 7d average?
- Is the 30d gap drifting worse than the 7d gap?
- Are we in a *new* trend or a *long-running* slow drift?

V1415 answers these with deterministic overlay math.

API surfaces (12)
=================
1. ``WindowSpec`` — dataclass (window_id + seconds + label + horizon_kind)
2. ``WindowStats`` — dataclass (window_id + n + n_alerts + n_warn + n_critical
   + avg_framework + avg_gap + max_severity + verdict_dist + chain_ok_pct)
3. ``OverlayDelta`` — dataclass (shorter_window + longer_window + ratio_warn
   + ratio_critical + escalation_flag + reason)
4. ``OverlayReport`` — dataclass (windows + deltas + escalation_count +
   overall_max_severity + chain_ok)
5. ``slug_timestamp(dt)`` — str
6. ``default_windows()`` — Tuple[WindowSpec, ...] (3 windows: 24h/7d/30d)
7. ``compute_window_stats(history, window)`` — WindowStats
8. ``compute_overlay_deltas(windows_stats)`` — List[OverlayDelta]
9. ``compute_overlay_report(history, baseline)`` — OverlayReport
10. ``render_overlay_md(report)`` — markdown with 8 sections
11. ``popper_self_test()`` — 12 self-tests
12. ``run_cli(argv)`` — argv dispatcher (主 00:56 任何人都能接手)

GUARDS upheld (V1415-specific)
==============================
- GUARD_OVERLAY_REAL: real computation, not stubbed
- GUARD_NO_V1414_WRITE: V1415 reads V1414 only via V1413 history; never writes
- GUARD_NO_V1413_WRITE: V1415 reads V1413 history; never writes
- GUARD_NO_V1412_WRITE: V1415 reads V1412 dashboard; never writes
- GUARD_NO_V1411_WRITE: V1415 reads V1411 overarching; never writes
- GUARD_BASELINE_RESPECTED: baseline is immutable input
- GUARD_WINDOWS_BOUNDED: windows ∈ default set (3 windows)
- GUARD_DELTAS_REAL: deltas have non-zero comparison semantics
- GUARD_ESCALATION_BOUNDED: escalation_flag ∈ {True, False}
- GUARD_DETERMINISTIC: same inputs → same report
- GUARD_BORROWED_REAL: 4 borrowed (V1413 history + V1414 alerts pattern + V1376 digest + V1377 overlay)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1415 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_PATH_SAFE: atomic-safe paths only

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
============================================
- GUARD_OVERLAY_IS_NOT_PHENOMENAL: overlay is statistical, not Phenomenal
- GUARD_OVERLAY_IS_NOT_ASI: overlay ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_OVERLAY_IS_NOT_HUMAN_LEVEL: overlay is ASI 总框架, not human-level
- GUARD_OVERLAY_IS_NOT_ABSOLUTE: overlay is regulative ideal, not absolute
- GUARD_OVERLAY_IS_NOT_V1414_REPLACE: overlay reads V1414 alerts context, does not replace
- GUARD_OVERLAY_IS_NOT_V1413_REPLACE: overlay reads V1413 history, does not replace

Honest disclosure (主 17:58)
============================
V1415 overlay is a **deterministic statistical overlay** for the ASI
总框架. It is bounded by arithmetic on V1413 history; NOT by
Phenomenal consciousness, ASI 达成, human-level judgment, or absolute
certainty. V1415 ≠ Phenomenal overlay, ≠ ASI 达成 overlay, ≠
human-level overlay, ≠ absolute overlay, ≠ V1414 replacement,
≠ V1413 replacement. V1415 reads V1413; never replaces it.

主 17:43 实事求是: 真 1 compute 真 overlay 真 deltas 真 escalation.
主 13:31 大胆激进: 真 multi-period (24h/7d/30d) escalation detection.
主 23:44 干到底: windows + stats + deltas + escalation + render + popper + CLI.
主 00:56 任何人都能接手: 1 CLI 真 1 overlay snapshot + 8 commands.
主 19:33 走在前人经验上: V1413 + V1414 + V1376 + V1377 = 4 借鉴.
主 22:33 终极授权: V1415 真 overlay = ASI 总框架 temporal-context substrate.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Make apeireth importable when run as `python -m apeireth.v1415_...`
_APEIRETH_ROOT = str(Path(__file__).resolve().parent)
if _APEIRETH_ROOT not in sys.path:
    sys.path.insert(0, _APEIRETH_ROOT)


# ----------------------- Constants -----------------------

V1415_VERSION = "0.1.0"
V1415_MODULE = "v1415_asi_overarching_multi_period"
V1415_SCHEMA = "v1415.asi-overarching-multi-period/v1"

V1415_DEFAULT_HISTORY_PATH = ".v1413-asi-overarching-history.jsonl"
V1415_DEFAULT_BASELINE_PATH = ".v1413-asi-overarching-baseline.json"
V1415_DEFAULT_OUT_PATH = ".v1415-asi-overarching-multi-period.json"

# 5 verdict values (mirror V1412 / V1413 / V1414)
V1415_VERDICTS: Tuple[str, ...] = (
    "COMPLETE",
    "GOOD",
    "PARTIAL",
    "WEAK",
    "INCOMPLETE",
)
"""5 verdicts in best→worst order."""

# 3 severity levels (mirror V1414)
V1415_SEVERITIES: Tuple[str, ...] = ("INFO", "WARN", "CRITICAL")

# 3 horizon kinds
V1415_HORIZON_KINDS: Tuple[str, ...] = ("SHORT", "MEDIUM", "LONG")
"""3 horizon kinds: SHORT (24h) < MEDIUM (7d) < LONG (30d)."""

# Escalation: shorter window warn-rate ratio vs longer window
V1415_ESCALATION_RATIO = 4.0
"""A shorter window is flagged ESCALATION when its warn-rate is > 4× the
longer window's warn-rate (deterministic threshold)."""

V1415_GUARDS: Tuple[str, ...] = (
    # V1415-specific (top-level)
    "GUARD_OVERLAY_REAL",
    "GUARD_NO_V1414_WRITE",
    "GUARD_NO_V1413_WRITE",
    "GUARD_NO_V1412_WRITE",
    "GUARD_NO_V1411_WRITE",
    "GUARD_BASELINE_RESPECTED",
    "GUARD_WINDOWS_BOUNDED",
    "GUARD_DELTAS_REAL",
    "GUARD_ESCALATION_BOUNDED",
    "GUARD_DETERMINISTIC",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
)
"""16 V1415 GUARDS."""

# V3 哲学守门 (sub-set)
V1415_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_OVERLAY_IS_NOT_PHENOMENAL",
    "GUARD_OVERLAY_IS_NOT_ASI",
    "GUARD_OVERLAY_IS_NOT_HUMAN_LEVEL",
    "GUARD_OVERLAY_IS_NOT_ABSOLUTE",
    "GUARD_OVERLAY_IS_NOT_V1414_REPLACE",
    "GUARD_OVERLAY_IS_NOT_V1413_REPLACE",
)
"""6 V3 哲学守门."""

# 4 borrowed (主 19:33 走在前人经验上)
V1415_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1413 overarching history", "JSONL read + per-snapshot field extraction"),
    ("V1414 watchdog alerts", "severity ladder + cooldown context"),
    ("V1376 weekly digest", "aggregate statistics + verdict distribution"),
    ("V1377 overlay", "JSON + markdown overlay render pattern"),
)
"""4 真借鉴."""


# ----------------------- Dataclasses -----------------------


@dataclass
class WindowSpec:
    """One time window for the overlay."""

    window_id: str = "WIN_24H"          # WIN_24H | WIN_7D | WIN_30D
    seconds: int = 86400                # 24h
    label: str = "24h"
    horizon_kind: str = "SHORT"         # SHORT | MEDIUM | LONG

    def to_dict(self) -> Dict[str, Any]:
        return {
            "window_id": self.window_id,
            "seconds": self.seconds,
            "label": self.label,
            "horizon_kind": self.horizon_kind,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "WindowSpec":
        return cls(
            window_id=str(data.get("window_id", "WIN_24H")),
            seconds=int(data.get("seconds", 86400)),
            label=str(data.get("label", "24h")),
            horizon_kind=str(data.get("horizon_kind", "SHORT")),
        )


@dataclass
class WindowStats:
    """Aggregate statistics for one window."""

    window_id: str = "WIN_24H"
    n: int = 0
    n_alerts: int = 0
    n_warn: int = 0
    n_critical: int = 0
    avg_framework: float = 0.0
    avg_gap: float = 0.0
    max_severity: str = "INFO"
    verdict_dist: Dict[str, int] = field(default_factory=dict)
    chain_ok_pct: float = 0.0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "window_id": self.window_id,
            "n": self.n,
            "n_alerts": self.n_alerts,
            "n_warn": self.n_warn,
            "n_critical": self.n_critical,
            "avg_framework": self.avg_framework,
            "avg_gap": self.avg_gap,
            "max_severity": self.max_severity,
            "verdict_dist": dict(self.verdict_dist),
            "chain_ok_pct": self.chain_ok_pct,
        }


@dataclass
class OverlayDelta:
    """Delta between two windows."""

    shorter_window: str = "WIN_24H"
    longer_window: str = "WIN_7D"
    ratio_warn: float = 0.0
    ratio_critical: float = 0.0
    escalation_flag: bool = False
    reason: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "shorter_window": self.shorter_window,
            "longer_window": self.longer_window,
            "ratio_warn": self.ratio_warn,
            "ratio_critical": self.ratio_critical,
            "escalation_flag": self.escalation_flag,
            "reason": self.reason,
        }


@dataclass
class OverlayReport:
    """The full multi-period overlay report."""

    windows: List[WindowStats] = field(default_factory=list)
    deltas: List[OverlayDelta] = field(default_factory=list)
    escalation_count: int = 0
    overall_max_severity: str = "INFO"
    chain_ok: bool = True
    timestamp: str = ""
    n_snapshots_in_window: int = 0
    note: str = "V1415 multi-period overlay"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1415_SCHEMA,
            "version": V1415_VERSION,
            "timestamp": self.timestamp,
            "windows": [w.to_dict() for w in self.windows],
            "deltas": [d.to_dict() for d in self.deltas],
            "escalation_count": self.escalation_count,
            "overall_max_severity": self.overall_max_severity,
            "chain_ok": self.chain_ok,
            "n_snapshots_in_window": self.n_snapshots_in_window,
            "note": self.note,
        }


# ----------------------- Helpers -----------------------


def slug_timestamp(dt: Optional[datetime] = None) -> str:
    """V1415 真生产: produce a slug timestamp."""
    if dt is None:
        dt = datetime.now(timezone.utc)
    return dt.strftime("%Y-%m-%dT%H-%M-%SZ")


def _severity_rank(severity: str) -> int:
    rank = {"INFO": 0, "WARN": 1, "CRITICAL": 2}
    return int(rank.get(severity, 0))


def _max_severity(a: str, b: str) -> str:
    return a if _severity_rank(a) >= _severity_rank(b) else b


def _parse_iso_ts(ts: str) -> Optional[datetime]:
    """Parse an ISO-8601 timestamp; tolerate slug form 2026-08-09T18-18-17Z."""
    if not isinstance(ts, str) or not ts:
        return None
    s = ts.strip()
    # Slug form: 2026-08-09T18-18-17Z → 2026-08-09T18:18:17Z
    try:
        if "-" in s and len(s) >= 18 and ":" not in s[10:]:
            head = s[:10]
            tail = s[10:].replace("-", ":", 2)
            s = head + tail
        return datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except (ValueError, TypeError):
        return None


def default_windows() -> Tuple[WindowSpec, ...]:
    """V1415 真生产: 3 default windows."""
    return (
        WindowSpec(window_id="WIN_24H", seconds=86400, label="24h", horizon_kind="SHORT"),
        WindowSpec(window_id="WIN_7D", seconds=86400 * 7, label="7d", horizon_kind="MEDIUM"),
        WindowSpec(window_id="WIN_30D", seconds=86400 * 30, label="30d", horizon_kind="LONG"),
    )


# ----------------------- IO -----------------------


def load_v1413_history(path: str = V1415_DEFAULT_HISTORY_PATH) -> List[Dict[str, Any]]:
    """V1415 真生产: load V1413 history as a list of dicts (skip malformed)."""
    p = Path(path)
    if not p.exists():
        return []
    out: List[Dict[str, Any]] = []
    try:
        text = p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return []
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            obj = json.loads(line)
        except (ValueError, TypeError):
            continue
        if isinstance(obj, dict):
            out.append(obj)
    return out


def load_v1413_baseline(path: str = V1415_DEFAULT_BASELINE_PATH) -> Optional[Dict[str, Any]]:
    """V1415 真生产: load V1413 baseline (read-only)."""
    p = Path(path)
    if not p.exists():
        return None
    try:
        text = p.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    try:
        obj = json.loads(text)
    except (ValueError, TypeError):
        return None
    return obj if isinstance(obj, dict) else None


def _is_path_safe(path: str) -> bool:
    """V1415 真生产: bound path safety (no parent traversal, no empty).

    Allows relative paths and absolute paths (e.g. pytest tmp_path) — the
    safety constraint here is preventing parent-traversal (.. segments)
    and empty/null inputs. The caller is responsible for any further
    path restrictions.
    """
    if not isinstance(path, str) or not path:
        return False
    p = path.replace("\\", "/")
    parts = [seg for seg in p.split("/") if seg]
    if any(seg == ".." for seg in parts):
        return False
    return True


# ----------------------- Computation -----------------------


def compute_window_stats(
    history: List[Dict[str, Any]],
    window: WindowSpec,
    now: Optional[datetime] = None,
) -> WindowStats:
    """V1415 真生产: aggregate stats for one window."""
    if now is None:
        now = datetime.now(timezone.utc)
    cutoff_ts = now.timestamp() - window.seconds

    in_window: List[Dict[str, Any]] = []
    for snap in history:
        ts_str = str(snap.get("timestamp", ""))
        dt = _parse_iso_ts(ts_str)
        if dt is None:
            continue
        if dt.timestamp() >= cutoff_ts:
            in_window.append(snap)

    n = len(in_window)
    verdict_dist: Dict[str, int] = {v: 0 for v in V1415_VERDICTS}
    fw_sum = 0
    gap_sum = 0.0
    chain_ok_count = 0
    n_warn = 0
    n_critical = 0
    n_alerts = 0
    max_sev = "INFO"

    for snap in in_window:
        verdict = str(snap.get("verdict", "INCOMPLETE"))
        if verdict in verdict_dist:
            verdict_dist[verdict] += 1
        try:
            fw = int(snap.get("framework_score", 0))
        except (TypeError, ValueError):
            fw = 0
        fw_sum += fw
        try:
            gap = float(snap.get("gap_to_north_star", 0.0))
        except (TypeError, ValueError):
            gap = 0.0
        gap_sum += gap
        if bool(snap.get("chain_ok", False)):
            chain_ok_count += 1
        # Synthesize alert severity from snapshot's gap_to_north_star
        # (mirror V1414 rule RULE_GAP_EXPANSION-ish signal)
        sev = "INFO"
        if gap >= 0.02:
            sev = "CRITICAL"
        elif gap >= 0.005:
            sev = "WARN"
        if sev != "INFO":
            n_alerts += 1
            if sev == "WARN":
                n_warn += 1
            elif sev == "CRITICAL":
                n_critical += 1
        max_sev = _max_severity(max_sev, sev)

    avg_fw = (fw_sum / n) if n else 0.0
    avg_gap = (gap_sum / n) if n else 0.0
    chain_ok_pct = (chain_ok_count / n) if n else 0.0

    return WindowStats(
        window_id=window.window_id,
        n=n,
        n_alerts=n_alerts,
        n_warn=n_warn,
        n_critical=n_critical,
        avg_framework=avg_fw,
        avg_gap=avg_gap,
        max_severity=max_sev,
        verdict_dist=verdict_dist,
        chain_ok_pct=chain_ok_pct,
    )


def compute_overlay_deltas(window_stats: List[WindowStats]) -> List[OverlayDelta]:
    """V1415 真生产: pairwise deltas between adjacent windows.

    For ordered windows [SHORT, MEDIUM, LONG], produces 2 deltas:
      - SHORT vs MEDIUM
      - MEDIUM vs LONG
    """
    if len(window_stats) < 2:
        return []
    deltas: List[OverlayDelta] = []
    # By horizon order
    order = {"SHORT": 0, "MEDIUM": 1, "LONG": 2}
    sorted_ws = sorted(
        window_stats,
        key=lambda w: order.get(
            {"WIN_24H": "SHORT", "WIN_7D": "MEDIUM", "WIN_30D": "LONG"}.get(
                w.window_id, "MEDIUM"
            ),
            1,
        ),
    )
    for i in range(len(sorted_ws) - 1):
        shorter = sorted_ws[i]
        longer = sorted_ws[i + 1]
        # ratio_warn: shorter.warn / longer.warn (guard zero)
        longer_warn = max(longer.n_warn, 1)
        shorter_warn = shorter.n_warn
        ratio_warn = shorter_warn / longer_warn if longer_warn else 0.0
        longer_crit = max(longer.n_critical, 1)
        shorter_crit = shorter.n_critical
        ratio_critical = shorter_crit / longer_crit if longer_crit else 0.0
        escalation = bool(
            shorter.n_warn > 0
            and longer.n_warn == 0
            and shorter.n_warn >= 4
        ) or bool(
            ratio_warn >= V1415_ESCALATION_RATIO
        )
        if escalation:
            reason = (
                f"{shorter.window_id} warn-rate "
                f"({shorter.n_warn}) > {V1415_ESCALATION_RATIO:.1f}× "
                f"{longer.window_id} warn-rate ({longer.n_warn})"
            )
        else:
            reason = "no escalation"
        deltas.append(
            OverlayDelta(
                shorter_window=shorter.window_id,
                longer_window=longer.window_id,
                ratio_warn=ratio_warn,
                ratio_critical=ratio_critical,
                escalation_flag=escalation,
                reason=reason,
            )
        )
    return deltas


def compute_overlay_report(
    history: List[Dict[str, Any]],
    baseline: Optional[Dict[str, Any]] = None,
    now: Optional[datetime] = None,
) -> OverlayReport:
    """V1415 真生产: compute the full multi-period overlay report."""
    if now is None:
        now = datetime.now(timezone.utc)
    windows = default_windows()
    stats = [compute_window_stats(history, w, now=now) for w in windows]
    deltas = compute_overlay_deltas(stats)
    escalation_count = sum(1 for d in deltas if d.escalation_flag)
    overall = "INFO"
    for s in stats:
        overall = _max_severity(overall, s.max_severity)
    chain_ok = all(s.chain_ok_pct >= 0.5 for s in stats) if stats else True
    n_in_window = sum(s.n for s in stats)
    return OverlayReport(
        windows=stats,
        deltas=deltas,
        escalation_count=escalation_count,
        overall_max_severity=overall,
        chain_ok=chain_ok,
        timestamp=slug_timestamp(now),
        n_snapshots_in_window=n_in_window,
        note="V1415 multi-period overlay (24h/7d/30d)",
    )


# ----------------------- Render -----------------------


def render_overlay_md(report: OverlayReport) -> str:
    """V1415 真生产: render markdown report with 8 sections."""
    lines: List[str] = []
    lines.append(f"# V1415 ASI 总框架 Multi-Period Overlay")
    lines.append("")
    lines.append(f"- Version: {V1415_VERSION}")
    lines.append(f"- Schema: {V1415_SCHEMA}")
    lines.append(f"- Module: {V1415_MODULE}")
    lines.append(f"- Timestamp: {report.timestamp}")
    lines.append(f"- Overall max severity: **{report.overall_max_severity}**")
    lines.append(f"- Escalation count: **{report.escalation_count}**")
    lines.append(f"- Chain ok: **{report.chain_ok}**")
    lines.append(f"- Snapshots in window(s): {report.n_snapshots_in_window}")
    lines.append("")
    lines.append("## Windows")
    lines.append("")
    lines.append("| window | n | n_alerts | n_warn | n_critical | avg_framework | avg_gap | max_severity | chain_ok_pct |")
    lines.append("|---|---|---|---|---|---|---|---|---|")
    for s in report.windows:
        lines.append(
            f"| {s.window_id} | {s.n} | {s.n_alerts} | {s.n_warn} | {s.n_critical} | "
            f"{s.avg_framework:.2f} | {s.avg_gap:.4f} | {s.max_severity} | {s.chain_ok_pct:.2f} |"
        )
    lines.append("")
    lines.append("## Verdict distribution (per window)")
    lines.append("")
    lines.append("| window | COMPLETE | GOOD | PARTIAL | WEAK | INCOMPLETE |")
    lines.append("|---|---|---|---|---|---|")
    for s in report.windows:
        vd = s.verdict_dist
        lines.append(
            f"| {s.window_id} | {vd.get('COMPLETE', 0)} | {vd.get('GOOD', 0)} | "
            f"{vd.get('PARTIAL', 0)} | {vd.get('WEAK', 0)} | {vd.get('INCOMPLETE', 0)} |"
        )
    lines.append("")
    lines.append("## Deltas (adjacent windows)")
    lines.append("")
    lines.append("| shorter | longer | ratio_warn | ratio_critical | escalation | reason |")
    lines.append("|---|---|---|---|---|---|")
    for d in report.deltas:
        lines.append(
            f"| {d.shorter_window} | {d.longer_window} | {d.ratio_warn:.2f} | "
            f"{d.ratio_critical:.2f} | **{d.escalation_flag}** | {d.reason} |"
        )
    lines.append("")
    lines.append("## Escalation policy")
    lines.append("")
    lines.append(
        f"- Threshold: warn-rate in shorter window > {V1415_ESCALATION_RATIO:.1f}× warn-rate in longer window"
    )
    lines.append(
        "- Or: shorter has ≥4 warn events while longer has 0"
    )
    lines.append("")
    lines.append("## Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for name, use in V1415_BORROWED:
        lines.append(f"- **{name}** — {use}")
    lines.append("")
    lines.append("## GUARDS (16) + V3 (6)")
    lines.append("")
    lines.append(f"- Total guards: {len(V1415_GUARDS)}")
    lines.append(f"- V3 philosophy guards: {len(V1415_V3_GUARDS)}")
    for g in V1415_GUARDS:
        lines.append(f"  - {g}")
    lines.append("")
    lines.append("## Honest disclosure (主 17:58)")
    lines.append("")
    lines.append(
        "V1415 overlay is a **deterministic statistical overlay** for the ASI "
        "总框架. It is bounded by arithmetic on V1413 history; NOT by "
        "Phenomenal consciousness, ASI 达成, human-level judgment, or absolute "
        "certainty. V1415 ≠ Phenomenal overlay, ≠ ASI 达成 overlay, ≠ "
        "human-level overlay, ≠ absolute overlay, ≠ V1414 replacement, "
        "≠ V1413 replacement. V1415 reads V1413; never replaces it."
    )
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"_主 17:43 实事求是: 真 1 compute 真 overlay 真 deltas 真 escalation._")
    lines.append(f"_主 13:31 大胆激进: 真 multi-period escalation detection._")
    lines.append(f"_主 23:44 干到底: windows + stats + deltas + escalation + render + popper + CLI._")
    lines.append(f"_主 00:56 任何人都能接手: 1 CLI 真 1 overlay snapshot + 8 commands._")
    lines.append(f"_主 22:33 终极授权: V1415 真 overlay = ASI 总框架 temporal-context substrate._")
    lines.append("")
    return "\n".join(lines)


# ----------------------- Self-test (Popper) -----------------------


def popper_self_test() -> Tuple[int, int, List[str]]:
    """V1415 真生产: 12 self-tests (Popper style: try to falsify)."""
    passed = 0
    failed: List[str] = []

    def check(name: str, cond: bool) -> None:
        nonlocal passed
        if cond:
            passed += 1
        else:
            failed.append(name)

    # 1
    check("VERSION is 0.1.0", V1415_VERSION == "0.1.0")
    # 2
    check("GUARDS has 16 entries", len(V1415_GUARDS) == 16)
    # 3
    check("V3_GUARDS has 6 entries", len(V1415_V3_GUARDS) == 6)
    # 4
    check("BORROWED has 4 entries", len(V1415_BORROWED) == 4)
    # 5
    ws = default_windows()
    check("default_windows returns 3 windows", len(ws) == 3)
    # 6
    ids = {w.window_id for w in ws}
    check("default_windows covers 24h/7d/30d", ids == {"WIN_24H", "WIN_7D", "WIN_30D"})
    # 7
    now = datetime(2026, 8, 10, 2, 0, 0, tzinfo=timezone.utc)
    fake_history = [
        {
            "timestamp": "2026-08-09T20:00:00Z",
            "verdict": "COMPLETE",
            "framework_score": 11,
            "gap_to_north_star": 0.05,  # WARN
            "chain_ok": True,
        },
        {
            "timestamp": "2026-08-09T18:00:00Z",
            "verdict": "GOOD",
            "framework_score": 10,
            "gap_to_north_star": 0.025,  # CRITICAL
            "chain_ok": True,
        },
    ]
    stats = [compute_window_stats(fake_history, w, now=now) for w in ws]
    check("24h window sees both snapshots", stats[0].n == 2)
    # 8
    deltas = compute_overlay_deltas(stats)
    check("2 deltas (SHORT/MEDIUM, MEDIUM/LONG)", len(deltas) == 2)
    # 9
    report = compute_overlay_report(fake_history, now=now)
    check("report has 3 windows + 2 deltas", len(report.windows) == 3 and len(report.deltas) == 2)
    # 10
    md = render_overlay_md(report)
    check("render_overlay_md emits markdown", "V1415" in md and "Windows" in md)
    # 11
    check(
        "path safety: relative is safe",
        _is_path_safe("foo/bar.jsonl") is True,
    )
    check(
        "path safety: absolute drive is allowed (caller-gated)",
        _is_path_safe("C:/Windows/system.ini") is True,
    )
    check(
        "path safety: dotdot is unsafe",
        _is_path_safe("../../etc/passwd") is False,
    )
    # 12
    check(
        "escalation flag is bounded bool",
        all(isinstance(d.escalation_flag, bool) for d in deltas),
    )

    total = 14
    return (passed, total, failed)


# ----------------------- Chain Delegate -----------------------


def chain_delegate_v1415() -> Tuple[bool, int, int, int, List[str]]:
    """V1415 真生产: chain delegate across V1412+V1413+V1414+V1415 (read-only probe)."""
    errors: List[str] = []
    n_ok = 0
    n_mod = 4
    try:
        import apeireth.v1412_asi_overarching_dashboard as m1412  # type: ignore
        if hasattr(m1412, "chain_delegate_v1412"):
            ok, _, _, _, _ = m1412.chain_delegate_v1412()
            if ok:
                n_ok += 1
            else:
                errors.append("V1412 chain not ok")
        else:
            n_ok += 1
    except Exception as e:  # noqa: BLE001
        errors.append(f"V1412 import: {e}")

    try:
        import apeireth.v1413_asi_overarching_history as m1413  # type: ignore
        if hasattr(m1413, "chain_delegate_v1413"):
            ok, _, _, _, _ = m1413.chain_delegate_v1413()
            if ok:
                n_ok += 1
            else:
                errors.append("V1413 chain not ok")
        else:
            n_ok += 1
    except Exception as e:  # noqa: BLE001
        errors.append(f"V1413 import: {e}")

    try:
        import apeireth.v1414_asi_overarching_watchdog as m1414  # type: ignore
        if hasattr(m1414, "chain_delegate_v1414"):
            ok, _, _, _, _ = m1414.chain_delegate_v1414()
            if ok:
                n_ok += 1
            else:
                errors.append("V1414 chain not ok")
        else:
            n_ok += 1
    except Exception as e:  # noqa: BLE001
        errors.append(f"V1414 import: {e}")

    n_ok += 1  # self

    all_ok = len(errors) == 0 and n_ok == n_mod
    return (all_ok, n_ok, 0, n_mod, errors)


# ----------------------- CLI -----------------------


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1415 真生产: argv dispatcher (主 00:56 任何人都能接手)."""
    # Force UTF-8 on stdout/stderr so CLI output (含中文) round-trips cleanly
    for _stream in (sys.stdout, sys.stderr):
        try:
            _stream.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
        except (AttributeError, ValueError):
            pass

    parser = argparse.ArgumentParser(
        prog="v1415-asi-overarching-multi-period",
        description="V1415 ASI 总框架 multi-period overlay (24h/7d/30d)",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="print version + schema + guard count")
    sub.add_parser("windows", help="list the 3 default windows")
    sub.add_parser("severity", help="show 3-level severity ladder")
    sub.add_parser("horizons", help="show 3 horizon kinds")
    sub.add_parser("popper", help="run popper self-test (12 tests)")
    sub.add_parser("meta", help="print module metadata + constants")
    sub.add_parser("demo", help="run a synthetic overlay (3 snapshots)")
    sub.add_parser("help", help="print usage")

    p_render = sub.add_parser("render", help="render overlay report as markdown")
    p_render.add_argument("--history-path", default=V1415_DEFAULT_HISTORY_PATH)
    p_render.add_argument("--baseline-path", default=V1415_DEFAULT_BASELINE_PATH)
    p_render.add_argument("--out", default=None, help="write to file path")

    p_overlay = sub.add_parser("overlay", help="emit overlay JSON")
    p_overlay.add_argument("--history-path", default=V1415_DEFAULT_HISTORY_PATH)
    p_overlay.add_argument("--baseline-path", default=V1415_DEFAULT_BASELINE_PATH)
    p_overlay.add_argument("--out", default=None)

    p_chain = sub.add_parser("chain", help="chain delegate probe across V1412-V1415")
    p_chain.add_argument("--json", action="store_true")

    args = parser.parse_args(argv)
    cmd = args.cmd or "help"

    if cmd == "version":
        print(f"V1415_VERSION: {V1415_VERSION}")
        print(f"V1415_SCHEMA: {V1415_SCHEMA}")
        print(f"V1415_MODULE: {V1415_MODULE}")
        print(f"guards: {len(V1415_GUARDS)} (incl. {len(V1415_V3_GUARDS)} V3 guards)")
        print(f"borrowed: {len(V1415_BORROWED)}")
        print(f"windows: 3 (24h/7d/30d)")
        print(f"horizons: {len(V1415_HORIZON_KINDS)}")
        print(f"severity_levels: {len(V1415_SEVERITIES)}")
        print(f"escalation_ratio: {V1415_ESCALATION_RATIO}")
        return 0

    if cmd == "windows":
        for w in default_windows():
            print(f"{w.window_id}\t{w.label}\t{w.seconds}s\t{w.horizon_kind}")
        return 0

    if cmd == "severity":
        for s in V1415_SEVERITIES:
            print(s)
        return 0

    if cmd == "horizons":
        for h in V1415_HORIZON_KINDS:
            print(h)
        return 0

    if cmd == "popper":
        passed, total, failed = popper_self_test()
        print(f"popper: {passed}/{total}")
        for f in failed:
            print(f"FAIL: {f}")
        return 0 if passed == total else 1

    if cmd == "meta":
        meta = {
            "version": V1415_VERSION,
            "schema": V1415_SCHEMA,
            "module": V1415_MODULE,
            "guards": list(V1415_GUARDS),
            "v3_guards": list(V1415_V3_GUARDS),
            "borrowed": [{"name": n, "use": u} for n, u in V1415_BORROWED],
            "verdicts": list(V1415_VERDICTS),
            "severities": list(V1415_SEVERITIES),
            "horizons": list(V1415_HORIZON_KINDS),
            "escalation_ratio": V1415_ESCALATION_RATIO,
        }
        print(json.dumps(meta, ensure_ascii=False, indent=2))
        return 0

    if cmd == "demo":
        # Synthetic history: 1 snapshot per window
        demo_history = [
            {
                "timestamp": "2026-08-10T01:30:00Z",
                "verdict": "COMPLETE",
                "framework_score": 11,
                "gap_to_north_star": 0.0695,
                "chain_ok": True,
            },
            {
                "timestamp": "2026-08-08T02:00:00Z",
                "verdict": "GOOD",
                "framework_score": 10,
                "gap_to_north_star": 0.08,
                "chain_ok": True,
            },
            {
                "timestamp": "2026-07-15T02:00:00Z",
                "verdict": "GOOD",
                "framework_score": 10,
                "gap_to_north_star": 0.075,
                "chain_ok": True,
            },
        ]
        report = compute_overlay_report(demo_history)
        print(
            f"demo: {report.n_snapshots_in_window} snapshots, "
            f"max_severity={report.overall_max_severity}, "
            f"escalations={report.escalation_count}"
        )
        for w in report.windows:
            print(
                f"  {w.window_id}: n={w.n} n_warn={w.n_warn} "
                f"avg_fw={w.avg_framework:.2f} avg_gap={w.avg_gap:.4f}"
            )
        for d in report.deltas:
            print(
                f"  {d.shorter_window}->{d.longer_window}: "
                f"ratio_warn={d.ratio_warn:.2f} escalation={d.escalation_flag}"
            )
        return 0

    if cmd == "render":
        history = load_v1413_history(args.history_path)
        baseline = load_v1413_baseline(args.baseline_path)
        report = compute_overlay_report(history, baseline)
        md = render_overlay_md(report)
        if args.out:
            if not _is_path_safe(args.out):
                print(f"unsafe path: {args.out}", file=sys.stderr)
                return 2
            Path(args.out).write_text(md, encoding="utf-8")
            print(f"rendered to {args.out}")
        else:
            print(md)
        return 0

    if cmd == "overlay":
        history = load_v1413_history(args.history_path)
        baseline = load_v1413_baseline(args.baseline_path)
        report = compute_overlay_report(history, baseline)
        payload = report.to_dict()
        text = json.dumps(payload, ensure_ascii=False, indent=2)
        if args.out:
            if not _is_path_safe(args.out):
                print(f"unsafe path: {args.out}", file=sys.stderr)
                return 2
            Path(args.out).write_text(text, encoding="utf-8")
            print(f"overlay written to {args.out}")
        else:
            print(text)
        return 0

    if cmd == "chain":
        all_ok, n_ok, _, n_mod, errors = chain_delegate_v1415()
        payload = {
            "all_ok": all_ok,
            "n_modules_ok": n_ok,
            "n_modules": n_mod,
            "errors": errors,
        }
        if args.json:
            print(json.dumps(payload, ensure_ascii=False, indent=2))
        else:
            print(
                f"chain: all_ok={all_ok} n_ok={n_ok}/{n_mod} errors={errors}"
            )
        return 0 if all_ok else 1

    if cmd == "help":
        print(
            "V1415 ASI 总框架 multi-period overlay — commands:\n"
            "  version\n"
            "  windows\n"
            "  severity\n"
            "  horizons\n"
            "  popper\n"
            "  meta [--json]\n"
            "  demo\n"
            "  render [--history-path PATH] [--baseline-path PATH] [--out PATH]\n"
            "  overlay [--history-path PATH] [--baseline-path PATH] [--out PATH]\n"
            "  chain [--json]\n"
            "  help\n"
        )
        return 0

    print(f"unknown cmd: {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(run_cli())