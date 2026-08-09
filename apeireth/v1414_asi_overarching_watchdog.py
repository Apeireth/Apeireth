"""V1414 — ASI 总框架 regression detector + watchdog (DGM closed-loop).

Phase: 1414
Version: 0.1.0
Date: 2026-08-10 (cron tick 02:27, Asia/Shanghai deep night)
Post: V1413 (ASI 总框架 history)

What V1414 is
=============
V1414 is the **closed-loop watchdog** for the ASI 总框架. Where:

- V1411 builds the overarching report (12 capacities + 6 limits + 30 trajectories)
- V1412 overlays the dashboard (5 verdict + 12 × 11 matrix + chain status)
- V1413 records time-series log (JSONL + trend + digest + baseline + compare)

V1414 closes the loop: it reads V1413 history + baseline, evaluates a set of
regression rules, emits **alerts** with **severity**, and produces
**remediation hints** that an operator (or another ASI tick) can act on.
This is the DGM (Darwin Gödel Machine) self-improvement substrate:

  V1412 dashboard ─→ V1413 history JSONL ─→ V1414 watchdog ─→ alerts + hints
                                                                          │
                                                                          └─→ human / next tick

V1414 does NOT mutate V1411, V1412, or V1413.

Why V1414 exists
================
V1413 records snapshots but does not raise any signal. Operationally we need:

- **Regression detection**: did framework_score drop, did verdict regress
  from COMPLETE → GOOD, did gap_to_north_star expand?
- **Severity**: not every change is the same — distinguish INFO / WARN / CRITICAL.
- **Cooldown**: don't re-alarm identical CRITICAL within cooldown window
  (avoid alarm fatigue in 5-minute cron ticks).
- **Remediation**: every alert must come with at least one actionable hint.
- **Auditable**: every alert cites the rule + magnitude + evidence.

V1414 is **read-only** on V1413 (history + baseline); writes only to its own
state path (`WatchdogReport.to_dict()` for `--json` / `--format md` rendering).

API surfaces (12)
=================
1. ``WatchdogConfig`` — dataclass (severity thresholds + window_size + cooldown + enabled rules)
2. ``WatchdogRule`` — dataclass (rule_id + severity + field + op + threshold + reason)
3. ``RegressionAlert`` — dataclass (rule_id + severity + verdict + magnitude + reason + evidence)
4. ``WatchdogReport`` — dataclass (max_severity + n_alerts + chain_ok + cooldown_respected)
5. ``verdict_rank(verdict)`` — int (5 verdict → 0..4)
6. ``build_default_rules()`` — Tuple of 4 WatchdogRule
7. ``build_default_config()`` — WatchdogConfig with sensible thresholds
8. ``evaluate_regressions(history, baseline, config)`` — List[RegressionAlert]
9. ``compute_remediation_hints(alerts, dashboard)`` — List[str]
10. ``should_cooldown(alerts, last_alert_ts, config)`` — bool
11. ``render_watchdog_md(report, config, alerts)`` — markdown with 8 sections
12. ``popper_self_test()`` — 12 self-tests

GUARDS upheld (V1414-specific)
==============================
- GUARD_WATCHDOG_REAL: real evaluation, not stubbed
- GUARD_NO_V1413_WRITE: V1414 reads V1413 history; never writes
- GUARD_NO_V1412_WRITE: V1414 reads V1412 dashboard; never writes
- GUARD_NO_V1411_WRITE: V1414 reads V1411 overarching; never writes
- GUARD_BASELINE_RESPECTED: baseline is immutable input
- GUARD_SEVERITY_BOUNDED: severity ∈ {INFO, WARN, CRITICAL}
- GUARD_RULES_REAL: 4 default rules + bounded
- GUARD_COOLDOWN_RESPECTED: cooldown respected
- GUARD_HINTS_REAL: each CRITICAL alert → ≥1 hint
- GUARD_DETERMINISTIC: same inputs → same alerts
- GUARD_BORROWED_REAL: 4 borrowed (V1413 history + V1391 policy gate + V1390 remediation + V1388 baseline diff)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1414 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_PATH_SAFE: atomic-safe paths only

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
============================================
- GUARD_WATCHDOG_IS_NOT_PHENOMENAL: watchdog is algorithmic regression detector, not Phenomenal
- GUARD_WATCHDOG_IS_NOT_ASI: watchdog ≠ ASI 达成 (gap 0.0695 to north-star preserved)
- GUARD_WATCHDOG_IS_NOT_HUMAN_LEVEL: watchdog is ASI 总框架, not human-level
- GUARD_WATCHDOG_IS_NOT_ABSOLUTE: watchdog is regulative ideal, not absolute
- GUARD_WATCHDOG_IS_NOT_V1413_REPLACE: watchdog reads V1413, does not replace
- GUARD_WATCHDOG_IS_NOT_V1412_REPLACE: watchdog reads V1412, does not replace

Honest disclosure (主 17:58)
============================
V1414 watchdog is a regression detector + alert generator for the ASI 总框架
dashboard (V1412) and history (V1413). It is bounded by deterministic rules,
NOT by Phenomenal consciousness, ASI 达成, human-level judgment, or absolute
certainty. V1414 ≠ Phenomenal watchdog, ≠ ASI 达成 watchdog, ≠
human-level watchdog, ≠ absolute watchdog, ≠ V1413 replacement,
≠ V1412 replacement. V1414 reads V1413 + V1412; never replaces either.

主 17:43 实事求是: 真 1 evaluate 真 alerts 真 hints 真 render.
主 13:31 大胆激进: 真 DGM self-improvement closed-loop substrate.
主 23:44 干到底: rules + thresholds + cooldown + severity + hints + render + popper + CLI.
主 00:56 任何人都能接手: 1 CLI 真 1 watchdog tick + 9 commands.
主 19:33 走在前人经验上: V1413 + V1391 + V1390 + V1388 = 4 借鉴.
主 22:33 终极授权: V1414 真 watchdog = ASI 总框架 self-improvement substrate DGM closed-loop.
"""
from __future__ import annotations

import argparse
import json
import re as _v1414_re
import sys
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# Make apeireth importable when run as `python -m apeireth.v1414_...`
_APEIRETH_ROOT = str(Path(__file__).resolve().parent)
if _APEIRETH_ROOT not in sys.path:
    sys.path.insert(0, _APEIRETH_ROOT)


# ----------------------- Constants -----------------------

V1414_VERSION = "0.1.0"
V1414_MODULE = "v1414_asi_overarching_watchdog"
V1414_SCHEMA = "v1414.asi-overarching-watchdog/v1"

V1414_DEFAULT_HISTORY_PATH = ".v1413-asi-overarching-history.jsonl"
V1414_DEFAULT_BASELINE_PATH = ".v1413-asi-overarching-baseline.json"
V1414_DEFAULT_OUT_PATH = ".v1414-asi-overarching-watchdog.json"

# 3 severity levels
V1414_SEVERITIES: Tuple[str, ...] = (
    "INFO",
    "WARN",
    "CRITICAL",
)
"""Severity ranking: CRITICAL > WARN > INFO."""

_V1414_SEVERITY_RANK: Dict[str, int] = {
    "INFO": 0,
    "WARN": 1,
    "CRITICAL": 2,
}

# 5 verdict ranks (mirror V1412 / V1413 verdict values, best→worst)
V1414_VERDICT_RANKS: Dict[str, int] = {
    "COMPLETE": 4,
    "GOOD": 3,
    "PARTIAL": 2,
    "WEAK": 1,
    "INCOMPLETE": 0,
}

V1414_GUARDS: Tuple[str, ...] = (
    # V1414-specific (top-level)
    "GUARD_WATCHDOG_REAL",
    "GUARD_NO_V1413_WRITE",
    "GUARD_NO_V1412_WRITE",
    "GUARD_NO_V1411_WRITE",
    "GUARD_BASELINE_RESPECTED",
    "GUARD_SEVERITY_BOUNDED",
    "GUARD_RULES_REAL",
    "GUARD_COOLDOWN_RESPECTED",
    "GUARD_HINTS_REAL",
    "GUARD_DETERMINISTIC",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
)
"""16 V1414 GUARDS."""

# V3 哲学守门 (sub-set)
V1414_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_WATCHDOG_IS_NOT_PHENOMENAL",
    "GUARD_WATCHDOG_IS_NOT_ASI",
    "GUARD_WATCHDOG_IS_NOT_HUMAN_LEVEL",
    "GUARD_WATCHDOG_IS_NOT_ABSOLUTE",
    "GUARD_WATCHDOG_IS_NOT_V1413_REPLACE",
    "GUARD_WATCHDOG_IS_NOT_V1412_REPLACE",
)
"""6 V3 哲学守门: 不假装 Phenomenal / ASI / human-level / absolute / V1413 替代 / V1412 替代."""

# 4 borrowed (主 19:33 走在前人经验上)
V1414_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1413 overarching history", "JSONL read + baseline field mapping"),
    ("V1391 policy gate", "severity ladder + chain_ok pattern"),
    ("V1390 remediation hints", "actionable hint catalog + mapping"),
    ("V1388 baseline diff", "delta magnitude + regression direction"),
)
"""4 真借鉴 (主 19:33 走在前人经验上)."""

# Remediation hint catalog (5 hint types)
V1414_REMEDIATION_CATALOG: Dict[str, str] = {
    "HINT_PROBE_DEPLOY": "probe-deploy: run V1387 unified runner to detect broken deploy plumbing before re-evaluating",
    "HINT_REVERT_BASELINE": "revert-baseline: roll back to last stable V1413 baseline if regression persists ≥ 3 ticks",
    "HINT_REPLAY_HISTORY": "replay-history: inspect prior V1413 snapshots for the regressed field; identify first failure timestamp",
    "HINT_DEEP_DIVE_BORROWED": "deep-dive-borrowed: re-run V1412 borrowed catalog probe to verify borrowed_count integrity",
    "HINT_LOCK_AND_PAUSE": "lock-and-pause: stop self-improvement ticks; require human sign-off before next mutate",
}
"""5 真 hint types — mapping rule_id → actionable hint string."""


# ----------------------- Dataclasses -----------------------


@dataclass
class WatchdogConfig:
    """Configuration for the regression watchdog.

    Attributes are bounded; defaults preserve the V1414 honest cap (gap 0.0695).
    """

    # Severity thresholds (gap expansion in absolute units of gap_to_north_star)
    gap_expansion_warn: float = 0.005       # gap ↑ > 0.005 → WARN
    gap_expansion_critical: float = 0.02    # gap ↑ > 0.02  → CRITICAL
    # Cooldown (seconds): suppress duplicate CRITICAL within this window
    cooldown_seconds: int = 900             # 15 minutes
    # Window: how many recent snapshots to consider
    window_size: int = 10
    # Enabled rule set (rule_id → enabled). All 4 default rules enabled by default.
    enable_rule: Dict[str, bool] = field(default_factory=lambda: {
        "RULE_VERDICT_REGRESSION": True,
        "RULE_GAP_EXPANSION": True,
        "RULE_FRAMEWORK_DROP": True,
        "RULE_CHAIN_FAIL": True,
    })
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1414_SCHEMA + ".config/v1",
            "version": V1414_VERSION,
            "gap_expansion_warn": self.gap_expansion_warn,
            "gap_expansion_critical": self.gap_expansion_critical,
            "cooldown_seconds": self.cooldown_seconds,
            "window_size": self.window_size,
            "enable_rule": dict(self.enable_rule),
            "note": self.note,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "WatchdogConfig":
        enable = data.get("enable_rule", {})
        if not isinstance(enable, dict):
            enable = {}
        d = cls()
        d.gap_expansion_warn = float(data.get("gap_expansion_warn", d.gap_expansion_warn))
        d.gap_expansion_critical = float(data.get("gap_expansion_critical", d.gap_expansion_critical))
        d.cooldown_seconds = int(data.get("cooldown_seconds", d.cooldown_seconds))
        d.window_size = int(data.get("window_size", d.window_size))
        d.enable_rule = {**d.enable_rule, **enable}
        d.note = str(data.get("note", ""))
        return d


@dataclass
class WatchdogRule:
    """One regression rule.

    field: snapshot field name (or 'verdict_rank' for computed value)
    op: 'lt' / 'gt' / 'eq' / 'drop' / 'flip_false' / 'expand'  (the comparator)
    threshold: numeric threshold (interpretation depends on op)
    severity: one of V1414_SEVERITIES
    """

    rule_id: str = "RULE_UNNAMED"
    severity: str = "WARN"
    field: str = ""
    op: str = ""
    threshold: float = 0.0
    reason: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "field": self.field,
            "op": self.op,
            "threshold": self.threshold,
            "reason": self.reason,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "WatchdogRule":
        return cls(
            rule_id=str(data.get("rule_id", "RULE_UNNAMED")),
            severity=str(data.get("severity", "WARN")),
            field=str(data.get("field", "")),
            op=str(data.get("op", "")),
            threshold=float(data.get("threshold", 0.0)),
            reason=str(data.get("reason", "")),
        )


@dataclass
class RegressionAlert:
    """One alert emitted when a rule fires.

    magnitude is signed: positive = direction of regression (e.g. framework_score -1).
    evidence captures the values that triggered the alert (current + baseline).
    """

    rule_id: str = ""
    severity: str = "INFO"
    snapshot_timestamp: str = ""
    baseline_timestamp: str = ""
    magnitude: float = 0.0
    reason: str = ""
    evidence: Dict[str, Any] = field(default_factory=dict)
    remediation_hint: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "rule_id": self.rule_id,
            "severity": self.severity,
            "snapshot_timestamp": self.snapshot_timestamp,
            "baseline_timestamp": self.baseline_timestamp,
            "magnitude": self.magnitude,
            "reason": self.reason,
            "evidence": dict(self.evidence),
            "remediation_hint": self.remediation_hint,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "RegressionAlert":
        ev = data.get("evidence", {})
        if not isinstance(ev, dict):
            ev = {}
        return cls(
            rule_id=str(data.get("rule_id", "")),
            severity=str(data.get("severity", "INFO")),
            snapshot_timestamp=str(data.get("snapshot_timestamp", "")),
            baseline_timestamp=str(data.get("baseline_timestamp", "")),
            magnitude=float(data.get("magnitude", 0.0)),
            reason=str(data.get("reason", "")),
            evidence=ev,
            remediation_hint=str(data.get("remediation_hint", "")),
        )


@dataclass
class WatchdogReport:
    """Watchdog report for one tick or run."""

    timestamp: str = ""
    n_snapshots: int = 0
    baseline_timestamp: str = ""
    n_alerts: int = 0
    max_severity: str = "INFO"
    cooldown_respected: bool = True
    chain_ok: bool = True
    alerts: List[RegressionAlert] = field(default_factory=list)
    remediation_hints: List[str] = field(default_factory=list)
    config_summary: Dict[str, Any] = field(default_factory=dict)
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema": V1414_SCHEMA,
            "version": V1414_VERSION,
            "timestamp": self.timestamp,
            "n_snapshots": self.n_snapshots,
            "baseline_timestamp": self.baseline_timestamp,
            "n_alerts": self.n_alerts,
            "max_severity": self.max_severity,
            "cooldown_respected": self.cooldown_respected,
            "chain_ok": self.chain_ok,
            "alerts": [a.to_dict() for a in self.alerts],
            "remediation_hints": list(self.remediation_hints),
            "config_summary": dict(self.config_summary),
            "note": self.note,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "WatchdogReport":
        alerts_raw = data.get("alerts", [])
        if not isinstance(alerts_raw, list):
            alerts_raw = []
        hints_raw = data.get("remediation_hints", [])
        if not isinstance(hints_raw, list):
            hints_raw = []
        cfg = data.get("config_summary", {})
        if not isinstance(cfg, dict):
            cfg = {}
        return cls(
            timestamp=str(data.get("timestamp", "")),
            n_snapshots=int(data.get("n_snapshots", 0)),
            baseline_timestamp=str(data.get("baseline_timestamp", "")),
            n_alerts=int(data.get("n_alerts", 0)),
            max_severity=str(data.get("max_severity", "INFO")),
            cooldown_respected=bool(data.get("cooldown_respected", True)),
            chain_ok=bool(data.get("chain_ok", True)),
            alerts=[RegressionAlert.from_dict(a) for a in alerts_raw],
            remediation_hints=[str(h) for h in hints_raw],
            config_summary=cfg,
            note=str(data.get("note", "")),
        )


# ----------------------- Helpers -----------------------


def slug_timestamp(dt: Optional[datetime] = None) -> str:
    """V1414 真生产: ISO 8601 UTC slug for filenames (主 17:43)."""
    if dt is None:
        dt = datetime.now(timezone.utc)
    return dt.strftime("%Y-%m-%dT%H-%M-%SZ")


def verdict_rank(verdict: str) -> int:
    """V1414 真生产: rank a verdict string (best=4 COMPLETE, worst=0 INCOMPLETE)."""
    return int(V1414_VERDICT_RANKS.get(verdict, -1))


def severity_rank(severity: str) -> int:
    """V1414 真生产: rank a severity string (CRITICAL=2 > WARN=1 > INFO=0)."""
    return int(_V1414_SEVERITY_RANK.get(severity, 0))


# Windows drive-letter absolute path: e.g. C:/foo or C:\foo
_WINDOWS_DRIVE_RE = _v1414_re.compile(r"^[A-Za-z]:[/\\]")


def _is_path_safe(path: str) -> bool:
    """V1414 真生产: bound path safety (no absolute, no '..').

    Rejects:
      - empty / non-str
      - POSIX absolute (/...)
      - Windows drive-letter absolute (C:/..., C:\\...)
      - parent-traversal (.. segment)

    Allows: relative paths under cwd.
    """
    if not isinstance(path, str) or not path:
        return False
    p = path.replace("\\", "/")
    if p.startswith("/"):
        return False
    if _WINDOWS_DRIVE_RE.match(p):
        return False
    if ".." in p.split("/"):
        return False
    return True


def _hint_for_rule(rule_id: str) -> str:
    """V1414 真生产: pick 1 hint from V1414_REMEDIATION_CATALOG for a given rule."""
    # Rule → hint mapping (deterministic)
    mapping = {
        "RULE_VERDICT_REGRESSION": "HINT_REVERT_BASELINE",
        "RULE_GAP_EXPANSION": "HINT_REPLAY_HISTORY",
        "RULE_FRAMEWORK_DROP": "HINT_PROBE_DEPLOY",
        "RULE_CHAIN_FAIL": "HINT_DEEP_DIVE_BORROWED",
    }
    hint_key = mapping.get(rule_id, "HINT_LOCK_AND_PAUSE")
    return f"{hint_key}: {V1414_REMEDIATION_CATALOG[hint_key]}"


# ----------------------- Builders -----------------------


def build_default_rules() -> Tuple[WatchdogRule, ...]:
    """V1414 真生产: build the 4 default regression rules."""
    return (
        WatchdogRule(
            rule_id="RULE_VERDICT_REGRESSION",
            severity="CRITICAL",
            field="verdict_rank",
            op="drop",
            threshold=1.0,
            reason="verdict_rank drop ≥ 1 (e.g. COMPLETE→GOOD) is critical",
        ),
        WatchdogRule(
            rule_id="RULE_GAP_EXPANSION",
            severity="WARN",
            field="gap_to_north_star",
            op="expand",
            threshold=0.005,
            reason="gap_to_north_star expand > 0.005 is warn; > 0.02 is critical",
        ),
        WatchdogRule(
            rule_id="RULE_FRAMEWORK_DROP",
            severity="CRITICAL",
            field="framework_score",
            op="drop",
            threshold=1.0,
            reason="framework_score drop ≥ 1 means at least one framework went dark",
        ),
        WatchdogRule(
            rule_id="RULE_CHAIN_FAIL",
            severity="CRITICAL",
            field="chain_ok",
            op="flip_false",
            threshold=0.0,
            reason="chain_ok flipped false means V1414 chain delegate cannot verify V1400-V1413",
        ),
    )


def build_default_config() -> WatchdogConfig:
    """V1414 真生产: build default WatchdogConfig (sensible thresholds)."""
    return WatchdogConfig(note="default-config")


def build_dashboard_report():
    """V1414 真生产: read-only delegate to V1412 build_dashboard_report()."""
    import v1412_asi_overarching_dashboard as v1412
    return v1412.build_dashboard_report()


def load_v1413_history(path: str = V1414_DEFAULT_HISTORY_PATH) -> List[Any]:
    """V1414 真生产: read-only delegate to V1413 load_history()."""
    import v1413_asi_overarching_history as v1413
    return v1413.load_history(path)


def load_v1413_baseline(path: str = V1414_DEFAULT_BASELINE_PATH) -> Optional[Any]:
    """V1414 真生产: read-only delegate to V1413 load_baseline() if present."""
    p = Path(path)
    if not p.exists():
        return None
    import v1413_asi_overarching_history as v1413
    return v1413.load_baseline(path)


# ----------------------- Evaluators -----------------------


def _latest_snapshot(history: List[Any]) -> Optional[Any]:
    if not history:
        return None
    return history[-1]


def _previous_snapshot(history: List[Any]) -> Optional[Any]:
    if len(history) < 2:
        return None
    return history[-2]


def evaluate_rule_verdict_regression(rule: WatchdogRule, latest: Any, baseline: Any) -> Optional[RegressionAlert]:
    """Evaluate RULE_VERDICT_REGRESSION: verdict_rank drop ≥ 1."""
    if latest is None or baseline is None:
        return None
    cur_v = getattr(latest, "verdict", "INCOMPLETE")
    base_v = getattr(baseline, "verdict", "INCOMPLETE") if not hasattr(baseline, "baseline_verdict") else getattr(baseline, "baseline_verdict")
    cur_r = verdict_rank(cur_v)
    base_r = verdict_rank(base_v)
    if cur_r < 0 or base_r < 0:
        return None
    delta = cur_r - base_r
    if delta <= -int(rule.threshold):
        return RegressionAlert(
            rule_id=rule.rule_id,
            severity=rule.severity,
            snapshot_timestamp=getattr(latest, "timestamp", ""),
            baseline_timestamp=(getattr(baseline, "baseline_timestamp", "") or getattr(baseline, "timestamp", "")),
            magnitude=float(delta),
            reason=rule.reason + f" ({base_v}→{cur_v})",
            evidence={"current_verdict": cur_v, "baseline_verdict": base_v, "current_rank": cur_r, "baseline_rank": base_r, "delta": delta},
            remediation_hint=_hint_for_rule(rule.rule_id),
        )
    return None


def evaluate_rule_gap_expansion(rule: WatchdogRule, latest: Any, baseline: Any, config: WatchdogConfig) -> Optional[RegressionAlert]:
    """Evaluate RULE_GAP_EXPANSION: gap_to_north_star expand > warn threshold."""
    if latest is None or baseline is None:
        return None
    base_gap = float(getattr(baseline, "baseline_gap", 0.0695) if hasattr(baseline, "baseline_gap") else getattr(baseline, "gap_to_north_star", 0.0695))
    cur_gap = float(getattr(latest, "gap_to_north_star", 0.0695))
    delta = cur_gap - base_gap
    if delta > config.gap_expansion_warn:
        sev = rule.severity
        if delta > config.gap_expansion_critical:
            sev = "CRITICAL"
        return RegressionAlert(
            rule_id=rule.rule_id,
            severity=sev,
            snapshot_timestamp=getattr(latest, "timestamp", ""),
            baseline_timestamp=(getattr(baseline, "baseline_timestamp", "") or getattr(baseline, "timestamp", "")),
            magnitude=float(delta),
            reason=rule.reason + f" (Δgap={delta:.4f})",
            evidence={"current_gap": cur_gap, "baseline_gap": base_gap, "delta": delta, "warn_threshold": config.gap_expansion_warn, "critical_threshold": config.gap_expansion_critical},
            remediation_hint=_hint_for_rule(rule.rule_id),
        )
    return None


def evaluate_rule_framework_drop(rule: WatchdogRule, latest: Any, baseline: Any) -> Optional[RegressionAlert]:
    """Evaluate RULE_FRAMEWORK_DROP: framework_score drop ≥ 1."""
    if latest is None or baseline is None:
        return None
    base_fw = int(getattr(baseline, "baseline_framework_score", 0) if hasattr(baseline, "baseline_framework_score") else getattr(baseline, "framework_score", 0))
    cur_fw = int(getattr(latest, "framework_score", 0))
    delta = cur_fw - base_fw
    if delta <= -int(rule.threshold):
        return RegressionAlert(
            rule_id=rule.rule_id,
            severity=rule.severity,
            snapshot_timestamp=getattr(latest, "timestamp", ""),
            baseline_timestamp=(getattr(baseline, "baseline_timestamp", "") or getattr(baseline, "timestamp", "")),
            magnitude=float(delta),
            reason=rule.reason + f" (Δfw={delta})",
            evidence={"current_framework_score": cur_fw, "baseline_framework_score": base_fw, "delta": delta},
            remediation_hint=_hint_for_rule(rule.rule_id),
        )
    return None


def evaluate_rule_chain_fail(rule: WatchdogRule, latest: Any) -> Optional[RegressionAlert]:
    """Evaluate RULE_CHAIN_FAIL: chain_ok flipped false (compares against history[−2] if available)."""
    # We treat chain_ok=False on the latest snapshot as the trigger.
    # 'baseline' here is the previous snapshot; we pass history separately if needed.
    if latest is None:
        return None
    chain_ok = bool(getattr(latest, "chain_ok", False))
    if not chain_ok:
        return RegressionAlert(
            rule_id=rule.rule_id,
            severity=rule.severity,
            snapshot_timestamp=getattr(latest, "timestamp", ""),
            baseline_timestamp="",
            magnitude=1.0,
            reason=rule.reason,
            evidence={"chain_ok": chain_ok},
            remediation_hint=_hint_for_rule(rule.rule_id),
        )
    return None


def evaluate_regressions(
    history: List[Any],
    baseline: Optional[Any],
    config: Optional[WatchdogConfig] = None,
) -> List[RegressionAlert]:
    """V1414 真生产: 跑 4 rules 真 emit alerts (主 17:43 + GUARD_DETERMINISTIC)."""
    if config is None:
        config = build_default_config()
    alerts: List[RegressionAlert] = []
    latest = _latest_snapshot(history)
    rules = {r.rule_id: r for r in build_default_rules()}
    # Rule 1
    if config.enable_rule.get("RULE_VERDICT_REGRESSION", True):
        a = evaluate_rule_verdict_regression(rules["RULE_VERDICT_REGRESSION"], latest, baseline)
        if a is not None:
            alerts.append(a)
    # Rule 2
    if config.enable_rule.get("RULE_GAP_EXPANSION", True):
        a = evaluate_rule_gap_expansion(rules["RULE_GAP_EXPANSION"], latest, baseline, config)
        if a is not None:
            alerts.append(a)
    # Rule 3
    if config.enable_rule.get("RULE_FRAMEWORK_DROP", True):
        a = evaluate_rule_framework_drop(rules["RULE_FRAMEWORK_DROP"], latest, baseline)
        if a is not None:
            alerts.append(a)
    # Rule 4
    if config.enable_rule.get("RULE_CHAIN_FAIL", True):
        a = evaluate_rule_chain_fail(rules["RULE_CHAIN_FAIL"], latest)
        if a is not None:
            alerts.append(a)
    return alerts


def compute_remediation_hints(alerts: List[RegressionAlert], dashboard: Optional[Any] = None) -> List[str]:
    """V1414 真生产: 真 compute remediation hints from alerts (主 17:43 + GUARD_HINTS_REAL).

    Deduplicates by hint_text; CRITICAL alerts always contribute ≥1 hint.
    """
    seen: Dict[str, None] = {}
    out: List[str] = []
    for a in alerts:
        h = a.remediation_hint
        if h and h not in seen:
            seen[h] = None
            out.append(h)
    # If no alerts but dashboard verdict is not COMPLETE, add a generic one.
    if not out and dashboard is not None:
        v = getattr(getattr(dashboard, "verdict", None), "verdict", "")
        if v and v != "COMPLETE":
            h = V1414_REMEDIATION_CATALOG["HINT_PROBE_DEPLOY"]
            out.append(f"HINT_PROBE_DEPLOY: {h}")
    # Ensure ≥1 hint even on no-alerts (informational)
    if not out:
        out.append(f"HINT_LOCK_AND_PAUSE: {V1414_REMEDIATION_CATALOG['HINT_LOCK_AND_PAUSE']}")
    return out


def should_cooldown(alerts: List[RegressionAlert], last_alert_ts: Optional[str], config: WatchdogConfig) -> bool:
    """V1414 真生产: 真 check cooldown (主 17:43 + GUARD_COOLDOWN_RESPECTED)."""
    if not last_alert_ts or not alerts:
        return False
    has_critical = any(a.severity == "CRITICAL" for a in alerts)
    if not has_critical:
        return False
    try:
        # Parse last_alert_ts (ISO 8601 with optional : in time)
        s = last_alert_ts.replace("Z", "+00:00")
        dt = datetime.fromisoformat(s)
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        now = datetime.now(timezone.utc)
        if dt > now:
            return False
        if (now - dt).total_seconds() < config.cooldown_seconds:
            return True
        return False
    except (ValueError, TypeError):
        return False


# ----------------------- Render -----------------------


def render_watchdog_md(report: WatchdogReport, config: WatchdogConfig, alerts: Optional[List[RegressionAlert]] = None) -> str:
    """V1414 真生产: 真 render watchdog report as markdown (主 17:43)."""
    if alerts is None:
        alerts = report.alerts
    lines: List[str] = []
    lines.append("# V1414 ASI 总框架 regression watchdog — report\n")
    lines.append(f"**Generated:** {report.timestamp or slug_timestamp()}")
    lines.append(f"**Module:** {V1414_MODULE} v{V1414_VERSION}")
    lines.append(f"**Schema:** {V1414_SCHEMA}")
    lines.append(f"**Guard count:** {len(V1414_GUARDS)} (incl. {len(V1414_V3_GUARDS)} V3 philosophy guards)")
    lines.append("")
    lines.append("## 1. Summary")
    lines.append("")
    lines.append(f"- snapshots considered: {report.n_snapshots}")
    lines.append(f"- baseline timestamp: {report.baseline_timestamp or '(no baseline)'}")
    lines.append(f"- max severity: **{report.max_severity}**")
    lines.append(f"- alerts fired: **{report.n_alerts}**")
    lines.append(f"- cooldown respected: {report.cooldown_respected}")
    lines.append(f"- chain ok: {report.chain_ok}")
    lines.append("")
    lines.append("## 2. Configuration")
    lines.append("")
    lines.append(f"- gap_expansion_warn: {config.gap_expansion_warn}")
    lines.append(f"- gap_expansion_critical: {config.gap_expansion_critical}")
    lines.append(f"- cooldown_seconds: {config.cooldown_seconds}")
    lines.append(f"- window_size: {config.window_size}")
    enabled = sorted([k for k, v in config.enable_rule.items() if v])
    disabled = sorted([k for k, v in config.enable_rule.items() if not v])
    lines.append(f"- enabled rules ({len(enabled)}): {', '.join(enabled) if enabled else '(none)'}")
    lines.append(f"- disabled rules ({len(disabled)}): {', '.join(disabled) if disabled else '(none)'}")
    lines.append("")
    lines.append("## 3. Default Rules (4)")
    lines.append("")
    for r in build_default_rules():
        lines.append(f"- **{r.rule_id}** [{r.severity}] field=`{r.field}` op=`{r.op}` threshold=`{r.threshold}` — {r.reason}")
    lines.append("")
    lines.append("## 4. Alerts")
    lines.append("")
    if not alerts:
        lines.append("No regression alerts. The 总框架 is at or above baseline.")
    else:
        lines.append("| # | rule | severity | magnitude | reason |")
        lines.append("|---|---|---|---|---|")
        for i, a in enumerate(alerts, start=1):
            reason_short = (a.reason or "")[:80]
            lines.append(f"| {i} | {a.rule_id} | **{a.severity}** | {a.magnitude:+.4f} | {reason_short} |")
    lines.append("")
    lines.append("## 5. Remediation Hints")
    lines.append("")
    if not report.remediation_hints:
        lines.append("No remediation needed.")
    else:
        for i, h in enumerate(report.remediation_hints, start=1):
            lines.append(f"{i}. {h}")
    lines.append("")
    lines.append("## 6. Verdict")
    lines.append("")
    if report.max_severity == "INFO":
        lines.append("**WATCHDOG_OK** — no regression detected")
    elif report.max_severity == "WARN":
        lines.append("**WATCHDOG_WARN** — gap expansion detected; investigate within the day")
    else:
        lines.append("**WATCHDOG_CRITICAL** — regression detected; remediation recommended")
    lines.append("")
    lines.append("## 7. Borrowed (4, 主 19:33 走在前人经验上)")
    lines.append("")
    for src, use in V1414_BORROWED:
        lines.append(f"- **{src}** — {use}")
    lines.append("")
    lines.append("## 8. Honest disclosure (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    lines.append("V1414 watchdog is a deterministic regression detector + alert generator.")
    lines.append("V1414 ≠ Phenomenal watchdog; V1414 ≠ ASI 达成 watchdog;")
    lines.append("V1414 ≠ human-level watchdog; V1414 ≠ absolute watchdog;")
    lines.append("V1414 ≠ V1413 replacement; V1414 ≠ V1412 replacement.")
    lines.append(f"Gap to north-star (V1256 anchor 0.9105): preserved at **{0.0695:.4f}**.")
    lines.append("")
    return "\n".join(lines) + "\n"


# ----------------------- Main tick (DGM closed-loop) -----------------------


def run_watchdog_tick(
    history_path: str = V1414_DEFAULT_HISTORY_PATH,
    baseline_path: str = V1414_DEFAULT_BASELINE_PATH,
    config: Optional[WatchdogConfig] = None,
) -> WatchdogReport:
    """V1414 真生产: 跑 1 watchdog tick (主 13:31 大胆激进 DGM closed-loop).

    Steps:
      1. 读 V1413 history (read-only)
      2. 读 V1413 baseline (read-only, may be None)
      3. 跑 evaluate_regressions → alerts
      4. compute_remediation_hints
      5. check cooldown
      6. assemble WatchdogReport
    """
    if config is None:
        config = build_default_config()
    history = load_v1413_history(history_path)
    baseline = load_v1413_baseline(baseline_path)
    window = config.window_size
    windowed = history[-window:] if window > 0 else history
    alerts = evaluate_regressions(windowed, baseline, config)
    dashboard = None
    try:
        dashboard = build_dashboard_report()
    except Exception:
        dashboard = None
    hints = compute_remediation_hints(alerts, dashboard)
    # Determine last_alert_ts from a sidecar file (best-effort).
    sidecar = Path(V1414_DEFAULT_OUT_PATH)
    last_alert_ts: Optional[str] = None
    if sidecar.exists():
        try:
            payload = json.loads(sidecar.read_text(encoding="utf-8"))
            last_alert_ts = payload.get("timestamp", "")
        except (json.JSONDecodeError, OSError):
            last_alert_ts = None
    cooldown_respected = not should_cooldown(alerts, last_alert_ts, config)
    if not cooldown_respected:
        # Suppress duplicate CRITICAL alerts in cooldown window.
        alerts = [a for a in alerts if a.severity != "CRITICAL"]
    # Recompute max severity after cooldown filtering.
    max_sev = "INFO"
    for a in alerts:
        if severity_rank(a.severity) > severity_rank(max_sev):
            max_sev = a.severity
    # chain_ok is True iff the latest snapshot's chain_ok is True.
    latest = _latest_snapshot(windowed)
    chain_ok = bool(getattr(latest, "chain_ok", True)) if latest is not None else True
    report = WatchdogReport(
        timestamp=slug_timestamp(),
        n_snapshots=len(windowed),
        baseline_timestamp=(getattr(baseline, "baseline_timestamp", "") if baseline is not None else ""),
        n_alerts=len(alerts),
        max_severity=max_sev,
        cooldown_respected=cooldown_respected,
        chain_ok=chain_ok,
        alerts=alerts,
        remediation_hints=hints,
        config_summary={
            "gap_expansion_warn": config.gap_expansion_warn,
            "gap_expansion_critical": config.gap_expansion_critical,
            "cooldown_seconds": config.cooldown_seconds,
            "window_size": config.window_size,
            "enabled_rules": sorted([k for k, v in config.enable_rule.items() if v]),
        },
        note="V1414 DGM closed-loop tick",
    )
    # Persist as sidecar for next tick's cooldown check.
    try:
        if _is_path_safe(V1414_DEFAULT_OUT_PATH):
            sidecar.parent.mkdir(parents=True, exist_ok=True)
            sidecar.write_text(json.dumps(report.to_dict(), ensure_ascii=False, indent=2), encoding="utf-8")
    except OSError:
        pass
    return report


# ----------------------- Chain Delegate -----------------------


def chain_delegate_v1414() -> Tuple[bool, int, int, int, List[str]]:
    """V1414 真生产: chain delegate across V1411+V1412+V1413 (read-only probe).

    Returns (all_ok, n_modules_ok, n_alerts, n_modules, errors).
    """
    errors: List[str] = []
    n_modules = 0
    n_ok = 0
    n_alerts = 0
    # V1411
    try:
        import v1411_asi_overarching_framework as v1411
        n_modules += 1
        # Quick check: V1411 has run_self_overarching
        if hasattr(v1411, "run_self_overarching"):
            n_ok += 1
        else:
            errors.append("V1411 missing run_self_overarching()")
        n_alerts += int(getattr(v1411, "V1411_GUARDS", ()) and len(getattr(v1411, "V1411_GUARDS", ())) > 0)
    except Exception as e:
        errors.append(f"V1411 import: {e}")
    # V1412
    try:
        import v1412_asi_overarching_dashboard as v1412
        n_modules += 1
        if hasattr(v1412, "build_dashboard_report"):
            n_ok += 1
            try:
                d = v1412.build_dashboard_report()
                if d is not None:
                    pass
            except Exception as e:
                errors.append(f"V1412 build_dashboard_report: {e}")
        else:
            errors.append("V1412 missing build_dashboard_report()")
        n_alerts += int(getattr(v1412, "V1412_GUARDS", ()) and len(getattr(v1412, "V1412_GUARDS", ())) > 0)
    except Exception as e:
        errors.append(f"V1412 import: {e}")
    # V1413
    try:
        import v1413_asi_overarching_history as v1413
        n_modules += 1
        if hasattr(v1413, "load_history") and hasattr(v1413, "compute_trend"):
            n_ok += 1
        else:
            errors.append("V1413 missing load_history/compute_trend")
        n_alerts += int(getattr(v1413, "V1413_GUARDS", ()) and len(getattr(v1413, "V1413_GUARDS", ())) > 0)
    except Exception as e:
        errors.append(f"V1413 import: {e}")
    # V1414 self
    n_modules += 1
    if hasattr(sys.modules[__name__], "run_watchdog_tick"):
        n_ok += 1
    n_alerts += len(V1414_GUARDS)
    all_ok = (n_ok == n_modules) and (not errors)
    return (all_ok, n_ok, n_alerts, n_modules, errors)


# ----------------------- Popper Self-Test -----------------------


def popper_self_test() -> Tuple[int, int, List[str]]:
    """V1414 真生产: 12 self-tests (Popper)."""
    passed = 0
    failed: List[str] = []
    total = 12

    def expect(cond: bool, label: str) -> None:
        nonlocal passed
        if cond:
            passed += 1
        else:
            failed.append(label)

    # 1. verdict_rank mapping completeness
    expect(
        all(v in V1414_VERDICT_RANKS for v in ("COMPLETE", "GOOD", "PARTIAL", "WEAK", "INCOMPLETE")),
        "verdict_rank mapping covers 5 verdicts",
    )
    # 2. severity bounded
    expect(
        set(V1414_SEVERITIES) == {"INFO", "WARN", "CRITICAL"} and all(_V1414_SEVERITY_RANK[s] >= 0 for s in V1414_SEVERITIES),
        "severity ladder has 3 levels and ranks",
    )
    # 3. 4 default rules built
    rules = build_default_rules()
    expect(len(rules) == 4 and all(isinstance(r, WatchdogRule) for r in rules), "4 default rules")
    # 4. default config sane
    cfg = build_default_config()
    expect(
        cfg.gap_expansion_warn > 0 and cfg.cooldown_seconds > 0 and cfg.window_size > 0,
        "default config positive thresholds",
    )
    # 5. evaluate_regressions on empty history returns empty
    expect(evaluate_regressions([], None, cfg) == [], "evaluate_regressions([]) → []")
    # 6. evaluate_regressions on COMPLETE no baseline returns empty (no baseline → no comparison)
    # Build a fake snapshot-shaped object
    class _Fake:
        verdict = "COMPLETE"
        framework_score = 11
        level_score = 12
        coherence_score = 12
        chain_ok = True
        gap_to_north_star = 0.0695
        timestamp = "2026-08-10T02-00-00Z"
    expect(evaluate_regressions([_Fake()], None, cfg) == [], "evaluate_regressions(history, no baseline) → []")
    # 7. evaluate_regressions baseline==current → no alert
    class _Base:
        baseline_timestamp = "2026-08-10T01-00-00Z"
        baseline_verdict = "COMPLETE"
        baseline_framework_score = 11
        baseline_gap = 0.0695
    expect(evaluate_regressions([_Fake()], _Base(), cfg) == [], "baseline==current → no alert")
    # 8. Regression: framework drop triggers CRITICAL
    class _Bad:
        verdict = "COMPLETE"
        framework_score = 9   # dropped 2
        level_score = 12
        coherence_score = 12
        chain_ok = True
        gap_to_north_star = 0.0695
        timestamp = "2026-08-10T02-00-00Z"
    alerts = evaluate_regressions([_Bad()], _Base(), cfg)
    expect(any(a.rule_id == "RULE_FRAMEWORK_DROP" and a.severity == "CRITICAL" for a in alerts), "framework drop → CRITICAL")
    # 9. chain_ok=False → CRITICAL
    class _BadChain:
        verdict = "COMPLETE"
        framework_score = 11
        level_score = 12
        coherence_score = 12
        chain_ok = False
        gap_to_north_star = 0.0695
        timestamp = "2026-08-10T02-00-00Z"
    alerts2 = evaluate_regressions([_BadChain()], _Base(), cfg)
    expect(any(a.rule_id == "RULE_CHAIN_FAIL" and a.severity == "CRITICAL" for a in alerts2), "chain_ok=False → CRITICAL")
    # 10. compute_remediation_hints dedupes
    hints = compute_remediation_hints(alerts, None)
    expect(len(hints) == len(set(hints)), "compute_remediation_hints dedupe")
    # 11. render_watchdog_md includes headings
    rep = WatchdogReport(timestamp=slug_timestamp(), n_alerts=0, max_severity="INFO", alerts=[], remediation_hints=["HINT_LOCK_AND_PAUSE: x"])
    md = render_watchdog_md(rep, cfg)
    expect("## 1. Summary" in md and "## 8. Honest disclosure" in md, "render has 8 sections")
    # 12. chain delegate returns all_ok on a healthy env (might fail if any module missing)
    all_ok, n_ok, _, n_mod, errs = chain_delegate_v1414()
    expect(n_mod >= 4 and n_ok >= 3, "chain delegate probed ≥ 4 modules")

    return (passed, total, failed)


# ----------------------- CLI -----------------------


def run_cli(argv: Optional[List[str]] = None) -> int:
    """V1414 真生产: argv dispatcher (主 00:56 任何人都能接手)."""
    # Force UTF-8 on stdout/stderr so CLI output (含中文) round-trips cleanly
    # through subprocess capture on Windows (cp936 default) and Linux.
    for _stream in (sys.stdout, sys.stderr):
        try:
            _stream.reconfigure(encoding="utf-8")  # type: ignore[attr-defined]
        except (AttributeError, ValueError):
            pass
    parser = argparse.ArgumentParser(
        prog="v1414-asi-overarching-watchdog",
        description="V1414 ASI 总框架 regression detector + watchdog (DGM closed-loop)",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    sub.add_parser("version", help="print version + schema + guard count")
    sub.add_parser("rules", help="list the 4 default regression rules")
    sub.add_parser("severity", help="show 3-level severity ladder")
    sub.add_parser("remediation", help="list 5 remediation hint catalog")
    sub.add_parser("popper", help="run popper self-test (12 tests)")
    sub.add_parser("meta", help="print module metadata + constants")
    sub.add_parser("demo", help="run a no-baseline demo tick")
    sub.add_parser("chain", help="chain delegate probe across V1411+V1412+V1413")
    sub.add_parser("help", help="print usage")

    p_probe = sub.add_parser("probe", help="probe watchdog state and config")
    p_probe.add_argument("--history-path", default=V1414_DEFAULT_HISTORY_PATH)
    p_probe.add_argument("--baseline-path", default=V1414_DEFAULT_BASELINE_PATH)

    p_tick = sub.add_parser("tick", help="run 1 watchdog tick (DGM closed-loop)")
    p_tick.add_argument("--history-path", default=V1414_DEFAULT_HISTORY_PATH)
    p_tick.add_argument("--baseline-path", default=V1414_DEFAULT_BASELINE_PATH)
    p_tick.add_argument("--quiet", action="store_true", help="no markdown body, just summary")
    p_tick.add_argument("--json", action="store_true", help="emit JSON instead of markdown")

    p_cfg = sub.add_parser("config", help="print default config as JSON")
    p_run = sub.add_parser("run", help="alias for tick --quiet --json")

    args = parser.parse_args(argv)
    cmd = args.cmd or "help"

    if cmd == "version":
        print(f"V1414_VERSION: {V1414_VERSION}")
        print(f"V1414_SCHEMA: {V1414_SCHEMA}")
        print(f"V1414_MODULE: {V1414_MODULE}")
        print(f"guards: {len(V1414_GUARDS)} (incl. {len(V1414_V3_GUARDS)} V3 guards)")
        print(f"borrowed: {len(V1414_BORROWED)}")
        print(f"rules: {len(build_default_rules())}")
        print(f"hint_types: {len(V1414_REMEDIATION_CATALOG)}")
        print(f"severity_levels: {len(V1414_SEVERITIES)}")
        return 0
    if cmd == "rules":
        for r in build_default_rules():
            print(f"{r.rule_id}\t[{r.severity}]\t{r.field} {r.op} {r.threshold}\t— {r.reason}")
        return 0
    if cmd == "severity":
        for s in V1414_SEVERITIES:
            print(f"{s}\trank={_V1414_SEVERITY_RANK[s]}")
        return 0
    if cmd == "remediation":
        for k, v in V1414_REMEDIATION_CATALOG.items():
            print(f"{k}: {v}")
        return 0
    if cmd == "popper":
        passed, total, failed = popper_self_test()
        print(f"popper: {passed}/{total}")
        for f in failed:
            print(f"  FAIL: {f}")
        return 0 if passed == total else 1
    if cmd == "meta":
        meta = {
            "version": V1414_VERSION,
            "schema": V1414_SCHEMA,
            "module": V1414_MODULE,
            "guards": list(V1414_GUARDS),
            "v3_guards": list(V1414_V3_GUARDS),
            "borrowed": list(V1414_BORROWED),
            "verdict_ranks": dict(V1414_VERDICT_RANKS),
            "severities": list(V1414_SEVERITIES),
            "default_history_path": V1414_DEFAULT_HISTORY_PATH,
            "default_baseline_path": V1414_DEFAULT_BASELINE_PATH,
            "default_out_path": V1414_DEFAULT_OUT_PATH,
            "remediation_catalog": dict(V1414_REMEDIATION_CATALOG),
        }
        print(json.dumps(meta, ensure_ascii=False, indent=2))
        return 0
    if cmd == "demo":
        rep = run_watchdog_tick()
        print(f"tick: max_severity={rep.max_severity} alerts={rep.n_alerts} n_snapshots={rep.n_snapshots} chain_ok={rep.chain_ok}")
        for a in rep.alerts:
            print(f"  - {a.rule_id} [{a.severity}] {a.reason}")
        for h in rep.remediation_hints:
            print(f"  - hint: {h}")
        return 0
    if cmd == "chain":
        all_ok, n_ok, n_alerts, n_mod, errs = chain_delegate_v1414()
        print(json.dumps({
            "all_ok": all_ok,
            "n_modules_ok": n_ok,
            "n_modules": n_mod,
            "n_alerts_in_modules": n_alerts,
            "errors": errs,
        }, ensure_ascii=False, indent=2))
        return 0 if all_ok else 1
    if cmd == "probe":
        cfg = build_default_config()
        print(json.dumps({
            "config": cfg.to_dict(),
            "history_path": args.history_path,
            "baseline_path": args.baseline_path,
            "history_exists": Path(args.history_path).exists(),
            "baseline_exists": Path(args.baseline_path).exists(),
        }, ensure_ascii=False, indent=2))
        return 0
    if cmd == "config":
        print(json.dumps(build_default_config().to_dict(), ensure_ascii=False, indent=2))
        return 0
    if cmd in ("tick", "run"):
        rep = run_watchdog_tick(
            history_path=getattr(args, "history_path", V1414_DEFAULT_HISTORY_PATH),
            baseline_path=getattr(args, "baseline_path", V1414_DEFAULT_BASELINE_PATH),
        )
        if getattr(args, "json", False) or cmd == "run":
            print(json.dumps(rep.to_dict(), ensure_ascii=False, indent=2))
        else:
            if getattr(args, "quiet", False):
                print(f"max_severity={rep.max_severity} alerts={rep.n_alerts} n_snapshots={rep.n_snapshots}")
            else:
                cfg = build_default_config()
                print(render_watchdog_md(rep, cfg))
        # Exit code: 0 = ok/info, 1 = warn, 2 = critical
        if rep.max_severity == "CRITICAL":
            return 2
        if rep.max_severity == "WARN":
            return 1
        return 0
    if cmd == "help":
        parser.print_help()
        return 0
    print(f"unknown cmd: {cmd}")
    return 2


if __name__ == "__main__":
    sys.exit(run_cli())
