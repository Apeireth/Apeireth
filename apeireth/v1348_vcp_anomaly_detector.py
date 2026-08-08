#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1348_vcp_anomaly_detector.py — VCP Plugin Anomaly Detector (post-V1347 health score)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1347 plugin health score (b5c355b5, 23:38); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 + 主 17:43 实事求是 + 主 13:31 大胆激进
- Chain: V1335 → ... → V1347 → **V1348**

V1347 stopped at "score a snapshot". V1348 = **ANOMALY DETECTION** (make health temporal):

  V1342 tier       ─┐
  V1343 lint       ─┤
  V1345 ledger     ─┼─→ multi-signal detector → anomaly_report
  V1346 plan       ─┤
  V1347 health     ─┘

Five deterministic anomaly channels (NOT ML, NOT learned thresholds):

  1. tier_jump         — tier changed between consecutive ledger records (sudden re-tier)
  2. lint_regression   — pass_5_critical count fell below historical floor (regression)
  3. drift_spike       — V1345 latest drift penalty exceeds drift_floor (sudden drift)
  4. plan_acceleration — V1346 plan count in recent window > plan_floor (remediation surge)
  5. health_drop       — V1347 health_score delta between two snapshots < -drop_floor (deterioration)

Severity ladder (per-signal; deterministic, threshold-based):
  signal_score <  LOW_FLOOR  → NONE
  LOW_FLOOR  <= score < MED_FLOOR  → LOW
  MED_FLOOR  <= score < HIGH_FLOOR  → MEDIUM
  score >= HIGH_FLOOR  → HIGH

Plugin-level severity = max(severity across channels) (worst-of principle).
Ecosystem-level rollup = worst-of plugin severity + per-severity counts.

anomaly_id = SHA256[:16] of stable payload (rule, plugin, channels, severities) —
content-addressed, no timestamp, reproducible. (Per V1347 precedent.)

V1348 = **DETECTION LAYER (NOT 假装 ASI judgments, NOT alert-flooder)**:
- Pure function: same inputs → same anomaly_report (no ML, no LLM, no fuzz)
- All thresholds are constants (reproducible, audit-friendly)
- Worst-of principle: surface only the most pressing signal per plugin
- Outputs are actionable: each anomaly carries a ruleId + recommendation
- Operators can subscribe to severity >= threshold (no info-dump)

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ? V1348 ≠ anomaly = oracle: detector = arithmetic, NOT learned judgment
- ? V1348 ≠ ASI has anomaly policy: thresholds = constants, NOT semantic
- ? V1348 = detection layer on V1342-V1347, NOT adjustment-of-model
- ? V1348 ≠ Phenomenal consciousness: detector has no qualia
- ? ASI pole-star LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE
- ? V1348 = real engineering observability (5-channel + rollup), NOT theater

ASI 5-Gap 真实用处 (主 13:31 大胆激进) — V1348 实证:
- 识别_recognition: anomaly_id is SHA256 of rule+plugin+channels → 识别 gap
- 自由_freedom: callers freely set thresholds and channel enable flags → 真自由编辑
- 时间_time: ledger history is the time axis → 时间性 explicit
- 真理_truth: thresholds are constants, fully determined by inputs → truth gap
- 涌现_emergence: ecosystem rollup surfaces patterns from per-plugin severities → emergence gap
"""
from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Literal, Optional, Sequence, Tuple

V1348_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1348_DIR))

import v1342_vcp_quality_tiers as v1342  # noqa: E402
import v1343_vcp_tier_aware_linter as v1343  # noqa: E402
import v1345_vcp_historical_ledger as v1345  # noqa: E402
import v1346_vcp_tier_aware_migration as v1346  # noqa: E402
import v1347_vcp_plugin_health as v1347  # noqa: E402

# --- Tier rank helper (v1342 uses lowercase strings; mirror for ordering) ----
TIER_RANK: Dict[str, int] = {
    "low": 0,
    "v1335_manual": 1,
    "medium": 2,
    "high": 3,
}


def tier_rank(tier: str) -> int:
    """Rank a tier string for delta computation (low < manual < medium < high)."""
    return TIER_RANK.get(tier, 0)

# --- ASI Pole-star (LOCKED) -------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,
    "V1348_modifies_pole_star": False,
}

# --- Anomaly channels -------------------------------------------------------
CHANNEL_TIER_JUMP = "tier_jump"
CHANNEL_LINT_REGRESSION = "lint_regression"
CHANNEL_DRIFT_SPIKE = "drift_spike"
CHANNEL_PLAN_ACCELERATION = "plan_acceleration"
CHANNEL_HEALTH_DROP = "health_drop"

ALL_CHANNELS: Tuple[str, ...] = (
    CHANNEL_TIER_JUMP,
    CHANNEL_LINT_REGRESSION,
    CHANNEL_DRIFT_SPIKE,
    CHANNEL_PLAN_ACCELERATION,
    CHANNEL_HEALTH_DROP,
)

# --- Severity ladder --------------------------------------------------------
SEVERITY_NONE = "NONE"
SEVERITY_LOW = "LOW"
SEVERITY_MEDIUM = "MEDIUM"
SEVERITY_HIGH = "HIGH"

SEVERITY_ORDER: Dict[str, int] = {
    SEVERITY_NONE: 0,
    SEVERITY_LOW: 1,
    SEVERITY_MEDIUM: 2,
    SEVERITY_HIGH: 3,
}

# --- Default thresholds (constants; overridable per-call) -------------------
DEFAULT_THRESHOLDS: Dict[str, float] = {
    # tier_jump: tier_change == 1 → 0.33, == 2 → 0.66, >= 3 → 1.00
    "tier_jump_weight": 0.34,
    # lint_regression: drop ratio = (floor - current_pass) / max_pass
    # 1 dropped pass = 0.20, 2 = 0.40, 3 = 0.60, 4 = 0.80, 5 = 1.00
    "lint_regression_weight": 0.20,
    # drift_spike: drift_penalty / max_drift_penalty (already 0..1)
    "drift_spike_weight": 1.00,
    # plan_acceleration: plan_count / window_runs
    # 1 plan in 5 runs = 0.20, 2 = 0.40, 3 = 0.60, 4 = 0.80, 5 = 1.00
    "plan_acceleration_weight": 0.20,
    # health_drop: max(0, -delta) / drop_window
    "health_drop_weight": 0.50,
    # Severity floors (signal_score → severity)
    "low_floor": 0.34,
    "medium_floor": 0.67,
    "high_floor": 1.00,
    # Plan acceleration window (most-recent N runs)
    "plan_window": 5,
    # Health drop window (consecutive snapshots to compare)
    "health_window": 2,
    # Lint regression history floor (minimum pass_5_critical seen)
    "lint_history_floor": 5,
}

# --- Recommendation map (per channel × severity) ---------------------------
RECOMMENDATIONS: Dict[Tuple[str, str], str] = {
    (CHANNEL_TIER_JUMP, SEVERITY_LOW): "review tier change in V1342; document justification",
    (CHANNEL_TIER_JUMP, SEVERITY_MEDIUM): "verify tier change against V1342 rule set; consider reclassify plan",
    (CHANNEL_TIER_JUMP, SEVERITY_HIGH): "halt automation; manual review of tier change; V1346 ignore-action required",
    (CHANNEL_LINT_REGRESSION, SEVERITY_LOW): "rerun V1343 linter; confirm not transient",
    (CHANNEL_LINT_REGRESSION, SEVERITY_MEDIUM): "open refactor ticket; add test coverage",
    (CHANNEL_LINT_REGRESSION, SEVERITY_HIGH): "block CI; fix 5-critical violations before merge",
    (CHANNEL_DRIFT_SPIKE, SEVERITY_LOW): "inspect V1345 ledger; verify drift detection rule",
    (CHANNEL_DRIFT_SPIKE, SEVERITY_MEDIUM): "V1346 mark-known plan if drift is benign; else re-tier",
    (CHANNEL_DRIFT_SPIKE, SEVERITY_HIGH): "V1346 refactor + audit-test actions; track in CI gate",
    (CHANNEL_PLAN_ACCELERATION, SEVERITY_LOW): "review V1346 plan cadence; ensure idempotency",
    (CHANNEL_PLAN_ACCELERATION, SEVERITY_MEDIUM): "consolidate plans; check for repeated mark-known patterns",
    (CHANNEL_PLAN_ACCELERATION, SEVERITY_HIGH): "audit plan generator; possible rule miscalibration",
    (CHANNEL_HEALTH_DROP, SEVERITY_LOW): "compare V1347 snapshots; isolate regressed component",
    (CHANNEL_HEALTH_DROP, SEVERITY_MEDIUM): "run V1342+V1343 fresh; verify scoring inputs",
    (CHANNEL_HEALTH_DROP, SEVERITY_HIGH): "freeze ecosystem rollup; triage components individually",
}


# --- Data classes -----------------------------------------------------------
@dataclass
class ChannelSignal:
    """One anomaly signal from one channel for one plugin."""
    channel: str                       # one of ALL_CHANNELS
    signal_score: float                # 0..1 (per-channel weighted)
    severity: str                      # NONE / LOW / MEDIUM / HIGH
    evidence: Dict[str, Any]           # channel-specific evidence
    recommendation: str                # actionable string

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class PluginAnomaly:
    """Anomaly report for a single plugin (worst-of across channels)."""
    plugin: str                        # plugin / substrate name
    plugin_severity: str               # max severity across channels
    plugin_severity_rank: int          # numeric rank for sorting
    channels: List[ChannelSignal]      # all channel signals (including NONE)
    anomaly_id: str                    # SHA256[:16] of stable payload

    def to_dict(self) -> Dict[str, Any]:
        return {
            "plugin": self.plugin,
            "plugin_severity": self.plugin_severity,
            "plugin_severity_rank": self.plugin_severity_rank,
            "channels": [c.to_dict() for c in self.channels],
            "anomaly_id": self.anomaly_id,
        }


@dataclass
class EcosystemAnomalyReport:
    """Ecosystem-level anomaly rollup."""
    per_plugin: List[PluginAnomaly]
    ecosystem_severity: str            # worst-of plugin severities
    ecosystem_severity_rank: int
    severity_breakdown: Dict[str, int]  # count of plugins per severity
    total_plugins: int
    enabled_channels: Tuple[str, ...]
    thresholds_used: Dict[str, float]
    report_id: str                     # SHA256[:16] of stable payload
    generated_at: str                  # ISO timestamp (NOT used in id)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "per_plugin": [p.to_dict() for p in self.per_plugin],
            "ecosystem_severity": self.ecosystem_severity,
            "ecosystem_severity_rank": self.ecosystem_severity_rank,
            "severity_breakdown": self.severity_breakdown,
            "total_plugins": self.total_plugins,
            "enabled_channels": list(self.enabled_channels),
            "thresholds_used": dict(self.thresholds_used),
            "report_id": self.report_id,
            "generated_at": self.generated_at,
        }


# --- Severity helper --------------------------------------------------------
def severity_for_score(score: float, thresholds: Dict[str, float]) -> str:
    """Map a 0..1 signal_score to a severity bucket using threshold constants."""
    if score >= thresholds["high_floor"]:
        return SEVERITY_HIGH
    if score >= thresholds["medium_floor"]:
        return SEVERITY_MEDIUM
    if score >= thresholds["low_floor"]:
        return SEVERITY_LOW
    return SEVERITY_NONE


def max_severity(severities: Iterable[str]) -> str:
    """Return the worst severity in an iterable."""
    best = SEVERITY_NONE
    best_rank = -1
    for s in severities:
        r = SEVERITY_ORDER.get(s, 0)
        if r > best_rank:
            best = s
            best_rank = r
    return best


# --- ID helper --------------------------------------------------------------
def _stable_id(payload: Dict[str, Any]) -> str:
    """SHA256[:16] of canonical JSON. No timestamp; reproducible."""
    s = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(s.encode("utf-8")).hexdigest()[:16]


# --- Channel detectors ------------------------------------------------------
def detect_tier_jump(
    plugin: str,
    tier_history: Sequence[str],
    thresholds: Dict[str, float],
) -> ChannelSignal:
    """Detect sudden tier change between consecutive records.

    Evidence: tier_history (oldest → newest); takes last two entries.
    signal_score = abs(newest_rank - previous_rank) * tier_jump_weight, clamped 0..1.
    """
    if len(tier_history) < 2:
        return ChannelSignal(
            channel=CHANNEL_TIER_JUMP,
            signal_score=0.0,
            severity=SEVERITY_NONE,
            evidence={"reason": "insufficient_history", "history_len": len(tier_history)},
            recommendation="",
        )
    prev, curr = tier_history[-2], tier_history[-1]
    prev_rank = tier_rank(prev)
    curr_rank = tier_rank(curr)
    diff = abs(curr_rank - prev_rank)
    score = min(1.0, diff * thresholds["tier_jump_weight"])
    severity = severity_for_score(score, thresholds)
    evidence = {
        "previous_tier": prev,
        "current_tier": curr,
        "previous_rank": prev_rank,
        "current_rank": curr_rank,
        "rank_delta": diff,
    }
    rec = RECOMMENDATIONS.get((CHANNEL_TIER_JUMP, severity), "")
    return ChannelSignal(CHANNEL_TIER_JUMP, score, severity, evidence, rec)


def detect_lint_regression(
    plugin: str,
    current_pass: int,
    historical_floor: int,
    thresholds: Dict[str, float],
) -> ChannelSignal:
    """Detect 5-critical lint regression vs historical floor.

    signal_score = (floor - current) / 5 if current < floor, else 0; scaled by weight.
    """
    if current_pass >= historical_floor:
        return ChannelSignal(
            channel=CHANNEL_LINT_REGRESSION,
            signal_score=0.0,
            severity=SEVERITY_NONE,
            evidence={"current_pass": current_pass, "floor": historical_floor},
            recommendation="",
        )
    dropped = historical_floor - current_pass
    raw = dropped / 5.0
    score = min(1.0, raw * thresholds["lint_regression_weight"] * 5.0)
    # simpler: score = dropped * lint_regression_weight, capped at 1.0
    score = min(1.0, dropped * thresholds["lint_regression_weight"])
    severity = severity_for_score(score, thresholds)
    evidence = {
        "current_pass": current_pass,
        "historical_floor": historical_floor,
        "dropped_count": dropped,
    }
    rec = RECOMMENDATIONS.get((CHANNEL_LINT_REGRESSION, severity), "")
    return ChannelSignal(CHANNEL_LINT_REGRESSION, score, severity, evidence, rec)


def detect_drift_spike(
    plugin: str,
    latest_drift_penalty: float,
    thresholds: Dict[str, float],
) -> ChannelSignal:
    """Detect drift penalty spike (already 0..1 from V1345).

    signal_score = latest_drift_penalty * weight, clamped 0..1.
    """
    raw = max(0.0, min(1.0, latest_drift_penalty))
    score = min(1.0, raw * thresholds["drift_spike_weight"])
    severity = severity_for_score(score, thresholds)
    evidence = {"latest_drift_penalty": latest_drift_penalty}
    rec = RECOMMENDATIONS.get((CHANNEL_DRIFT_SPIKE, severity), "")
    return ChannelSignal(CHANNEL_DRIFT_SPIKE, score, severity, evidence, rec)


def detect_plan_acceleration(
    plugin: str,
    recent_plan_count: int,
    window: int,
    thresholds: Dict[str, float],
) -> ChannelSignal:
    """Detect remediation-plan surge in recent window.

    signal_score = (recent_plan_count / window) * weight, clamped 0..1.
    """
    safe_window = max(1, window)
    ratio = recent_plan_count / safe_window
    score = min(1.0, ratio * thresholds["plan_acceleration_weight"] * safe_window)
    # simpler: score = recent_plan_count * weight, capped at 1.0
    score = min(1.0, recent_plan_count * thresholds["plan_acceleration_weight"])
    severity = severity_for_score(score, thresholds)
    evidence = {
        "recent_plan_count": recent_plan_count,
        "window": window,
    }
    rec = RECOMMENDATIONS.get((CHANNEL_PLAN_ACCELERATION, severity), "")
    return ChannelSignal(CHANNEL_PLAN_ACCELERATION, score, severity, evidence, rec)


def detect_health_drop(
    plugin: str,
    recent_scores: Sequence[float],
    thresholds: Dict[str, float],
) -> ChannelSignal:
    """Detect V1347 health_score drop between two snapshots.

    signal_score = max(0, previous - current) * weight, clamped 0..1.
    """
    if len(recent_scores) < 2:
        return ChannelSignal(
            channel=CHANNEL_HEALTH_DROP,
            signal_score=0.0,
            severity=SEVERITY_NONE,
            evidence={"reason": "insufficient_history", "history_len": len(recent_scores)},
            recommendation="",
        )
    previous, current = recent_scores[-2], recent_scores[-1]
    drop = max(0.0, previous - current)
    score = min(1.0, drop * thresholds["health_drop_weight"] * 2.0)
    # simpler: score = drop * weight, capped at 1.0
    score = min(1.0, drop * thresholds["health_drop_weight"])
    severity = severity_for_score(score, thresholds)
    evidence = {
        "previous_score": previous,
        "current_score": current,
        "delta": previous - current,
    }
    rec = RECOMMENDATIONS.get((CHANNEL_HEALTH_DROP, severity), "")
    return ChannelSignal(CHANNEL_HEALTH_DROP, score, severity, evidence, rec)


# --- Per-plugin assembly ----------------------------------------------------
def _build_anomaly_id(plugin: str, channels: Sequence[ChannelSignal]) -> str:
    payload = {
        "plugin": plugin,
        "channels": [
            {
                "channel": c.channel,
                "signal_score": round(c.signal_score, 6),
                "severity": c.severity,
                "evidence": c.evidence,
            }
            for c in channels
        ],
    }
    return _stable_id(payload)


def analyze_plugin(
    plugin: str,
    *,
    tier_history: Sequence[str],
    current_lint_pass: int,
    historical_lint_floor: int,
    latest_drift_penalty: float,
    recent_plan_count: int,
    recent_health_scores: Sequence[float],
    thresholds: Optional[Dict[str, float]] = None,
    enabled_channels: Optional[Sequence[str]] = None,
) -> PluginAnomaly:
    """Run all enabled channels for one plugin; return worst-of severity.

    enabled_channels semantics:
      - None  → all channels (default)
      - empty tuple/list → NO channels (explicit no-op)
    """
    t = dict(thresholds or DEFAULT_THRESHOLDS)
    if enabled_channels is None:
        enabled = ALL_CHANNELS
    else:
        enabled = tuple(enabled_channels)

    channels: List[ChannelSignal] = []
    if CHANNEL_TIER_JUMP in enabled:
        channels.append(detect_tier_jump(plugin, tier_history, t))
    if CHANNEL_LINT_REGRESSION in enabled:
        channels.append(detect_lint_regression(plugin, current_lint_pass, historical_lint_floor, t))
    if CHANNEL_DRIFT_SPIKE in enabled:
        channels.append(detect_drift_spike(plugin, latest_drift_penalty, t))
    if CHANNEL_PLAN_ACCELERATION in enabled:
        channels.append(
            detect_plan_acceleration(
                plugin, recent_plan_count, t["plan_window"], t
            )
        )
    if CHANNEL_HEALTH_DROP in enabled:
        channels.append(detect_health_drop(plugin, recent_health_scores, t))

    plugin_severity = max_severity(c.severity for c in channels)
    rank = SEVERITY_ORDER.get(plugin_severity, 0)
    aid = _build_anomaly_id(plugin, channels)

    return PluginAnomaly(
        plugin=plugin,
        plugin_severity=plugin_severity,
        plugin_severity_rank=rank,
        channels=channels,
        anomaly_id=aid,
    )


# --- Ecosystem rollup -------------------------------------------------------
def _build_report_id(per_plugin: Sequence[PluginAnomaly], enabled: Sequence[str], thresholds: Dict[str, float]) -> str:
    payload = {
        "per_plugin": [p.to_dict() for p in per_plugin],
        "enabled_channels": sorted(enabled),
        "thresholds": {k: round(v, 6) for k, v in sorted(thresholds.items())},
    }
    return _stable_id(payload)


def build_report(
    per_plugin: Sequence[PluginAnomaly],
    *,
    enabled_channels: Optional[Sequence[str]] = None,
    thresholds: Optional[Dict[str, float]] = None,
) -> EcosystemAnomalyReport:
    """Aggregate per-plugin anomalies into an ecosystem rollup."""
    if enabled_channels is None:
        enabled = ALL_CHANNELS
    else:
        enabled = tuple(enabled_channels)
    t = dict(thresholds or DEFAULT_THRESHOLDS)

    eco_severity = max_severity(p.plugin_severity for p in per_plugin)
    eco_rank = SEVERITY_ORDER.get(eco_severity, 0)

    breakdown: Dict[str, int] = {SEVERITY_NONE: 0, SEVERITY_LOW: 0, SEVERITY_MEDIUM: 0, SEVERITY_HIGH: 0}
    for p in per_plugin:
        breakdown[p.plugin_severity] = breakdown.get(p.plugin_severity, 0) + 1

    rid = _build_report_id(per_plugin, enabled, t)
    return EcosystemAnomalyReport(
        per_plugin=list(per_plugin),
        ecosystem_severity=eco_severity,
        ecosystem_severity_rank=eco_rank,
        severity_breakdown=breakdown,
        total_plugins=len(per_plugin),
        enabled_channels=enabled,
        thresholds_used=t,
        report_id=rid,
        generated_at=datetime.now(timezone.utc).isoformat(),
    )


# --- Convenience: detect from V1347 health reports --------------------------
def detect_from_health_reports(
    health_reports: Sequence[Any],  # v1347.PluginHealthReport-like
    *,
    tier_history_map: Optional[Dict[str, Sequence[str]]] = None,
    lint_pass_map: Optional[Dict[str, int]] = None,
    lint_floor_map: Optional[Dict[str, int]] = None,
    drift_penalty_map: Optional[Dict[str, float]] = None,
    plan_count_map: Optional[Dict[str, int]] = None,
    thresholds: Optional[Dict[str, float]] = None,
    enabled_channels: Optional[Sequence[str]] = None,
) -> EcosystemAnomalyReport:
    """Top-level entry: derive anomaly inputs from V1347 health reports + maps.

    Each map is keyed by plugin name. Missing plugin → channel reports NONE.
    """
    tier_history_map = tier_history_map or {}
    lint_pass_map = lint_pass_map or {}
    lint_floor_map = lint_floor_map or {}
    drift_penalty_map = drift_penalty_map or {}
    plan_count_map = plan_count_map or {}

    # Per-plugin score history from health_reports
    score_history: Dict[str, List[float]] = {}
    for rep in health_reports:
        for plugin in rep.per_plugin:  # type: ignore[attr-defined]
            name = plugin.plugin
            score_history.setdefault(name, []).append(plugin.health_score)

    # Get plugin union from score history + map keys
    plugin_union: List[str] = list(score_history.keys())
    for m in (lint_pass_map, drift_penalty_map, plan_count_map, tier_history_map):
        for k in m.keys():
            if k not in plugin_union:
                plugin_union.append(k)

    per_plugin: List[PluginAnomaly] = []
    default_floor = (thresholds or DEFAULT_THRESHOLDS)["lint_history_floor"]
    for plugin in sorted(plugin_union):
        per_plugin.append(
            analyze_plugin(
                plugin,
                tier_history=tier_history_map.get(plugin, []),
                current_lint_pass=lint_pass_map.get(plugin, 0),
                historical_lint_floor=lint_floor_map.get(plugin, default_floor),
                latest_drift_penalty=drift_penalty_map.get(plugin, 0.0),
                recent_plan_count=plan_count_map.get(plugin, 0),
                recent_health_scores=score_history.get(plugin, []),
                thresholds=thresholds,
                enabled_channels=enabled_channels,
            )
        )

    return build_report(
        per_plugin,
        enabled_channels=enabled_channels,
        thresholds=thresholds,
    )


# --- Popper self-tests (always-run, no pytest needed) -----------------------
def _popper_self_tests() -> List[Tuple[str, bool, str]]:
    """Return list of (name, passed, detail) for embedded Popper tests."""
    results: List[Tuple[str, bool, str]] = []
    t = dict(DEFAULT_THRESHOLDS)

    # 1. Stable ID: same input → same id
    a1 = _stable_id({"x": 1, "y": "abc"})
    a2 = _stable_id({"y": "abc", "x": 1})  # sort_keys
    results.append(("stable_id_is_key_order_invariant", a1 == a2, f"{a1} vs {a2}"))

    # 2. Severity mapping: 0.0 → NONE, 0.5 → LOW, 0.8 → MEDIUM, 1.0 → HIGH
    s_none = severity_for_score(0.0, t)
    s_low = severity_for_score(0.5, t)
    s_med = severity_for_score(0.8, t)
    s_high = severity_for_score(1.0, t)
    results.append((
        "severity_ladder_monotonic",
        s_none == SEVERITY_NONE and s_low == SEVERITY_LOW and s_med == SEVERITY_MEDIUM and s_high == SEVERITY_HIGH,
        f"none={s_none} low={s_low} med={s_med} high={s_high}",
    ))

    # 3. max_severity: NONE + LOW + MED + HIGH → HIGH
    mx = max_severity([SEVERITY_NONE, SEVERITY_LOW, SEVERITY_MEDIUM, SEVERITY_HIGH])
    results.append(("max_severity_worst_of", mx == SEVERITY_HIGH, f"got {mx}"))

    # 4. tier_jump: high → high no jump
    sig = detect_tier_jump("p1", ["high", "high"], t)
    results.append(("tier_jump_no_change_NONE", sig.severity == SEVERITY_NONE, f"severity={sig.severity}"))

    # 5. tier_jump: low → high (3 ranks) → MED or HIGH
    sig = detect_tier_jump("p1", ["low", "high"], t)
    results.append(("tier_jump_3_ranks", sig.severity in (SEVERITY_MEDIUM, SEVERITY_HIGH), f"severity={sig.severity} score={sig.signal_score}"))

    # 6. lint_regression: pass=5, floor=5 → NONE
    sig = detect_lint_regression("p1", 5, 5, t)
    results.append(("lint_regression_no_drop_NONE", sig.severity == SEVERITY_NONE, f"severity={sig.severity}"))

    # 7. lint_regression: pass=0, floor=5 → HIGH
    sig = detect_lint_regression("p1", 0, 5, t)
    results.append(("lint_regression_full_drop_HIGH", sig.severity == SEVERITY_HIGH, f"severity={sig.severity} score={sig.signal_score}"))

    # 8. drift_spike: penalty=0.0 → NONE
    sig = detect_drift_spike("p1", 0.0, t)
    results.append(("drift_spike_zero_NONE", sig.severity == SEVERITY_NONE, f"severity={sig.severity}"))

    # 9. drift_spike: penalty=1.0 → HIGH
    sig = detect_drift_spike("p1", 1.0, t)
    results.append(("drift_spike_full_HIGH", sig.severity == SEVERITY_HIGH, f"severity={sig.severity}"))

    # 10. plan_acceleration: 0 plans → NONE
    sig = detect_plan_acceleration("p1", 0, 5, t)
    results.append(("plan_accel_zero_NONE", sig.severity == SEVERITY_NONE, f"severity={sig.severity}"))

    # 11. plan_acceleration: 5 plans in window=5 → HIGH
    sig = detect_plan_acceleration("p1", 5, 5, t)
    results.append(("plan_accel_full_HIGH", sig.severity == SEVERITY_HIGH, f"severity={sig.severity}"))

    # 12. health_drop: 0.8 → 0.5 → some drop
    sig = detect_health_drop("p1", [0.8, 0.5], t)
    results.append(("health_drop_positive", sig.signal_score > 0.0, f"score={sig.signal_score} severity={sig.severity}"))

    # 13. health_drop: 0.5 → 0.8 → no drop
    sig = detect_health_drop("p1", [0.5, 0.8], t)
    results.append(("health_drop_recovery_NONE", sig.severity == SEVERITY_NONE, f"severity={sig.severity}"))

    # 14. analyze_plugin: empty everything → NONE severity, anomaly_id present
    pa = analyze_plugin("p_empty", tier_history=[], current_lint_pass=5, historical_lint_floor=5, latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[])
    results.append(("analyze_plugin_empty_NONE", pa.plugin_severity == SEVERITY_NONE, f"severity={pa.plugin_severity}"))
    results.append(("anomaly_id_nonempty", len(pa.anomaly_id) == 16, f"id={pa.anomaly_id}"))

    # 15. analyze_plugin: full anomaly across all channels → HIGH
    pa = analyze_plugin(
        "p_hot",
        tier_history=["low", "high"],
        current_lint_pass=0,
        historical_lint_floor=5,
        latest_drift_penalty=1.0,
        recent_plan_count=10,
        recent_health_scores=[0.95, 0.40],
    )
    results.append(("analyze_plugin_full_HIGH", pa.plugin_severity == SEVERITY_HIGH, f"severity={pa.plugin_severity}"))

    # 16. build_report: 3 plugins with mixed severities → worst-of correct
    pa_low = analyze_plugin("p_low", tier_history=[], current_lint_pass=5, historical_lint_floor=5, latest_drift_penalty=0.1, recent_plan_count=0, recent_health_scores=[])
    pa_med = analyze_plugin("p_med", tier_history=[], current_lint_pass=3, historical_lint_floor=5, latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[])
    pa_high = analyze_plugin("p_high", tier_history=["low", "high"], current_lint_pass=0, historical_lint_floor=5, latest_drift_penalty=0.0, recent_plan_count=0, recent_health_scores=[])
    rep = build_report([pa_low, pa_med, pa_high])
    results.append(("ecosystem_rollup_worst_HIGH", rep.ecosystem_severity == SEVERITY_HIGH, f"eco={rep.ecosystem_severity}"))
    results.append(("ecosystem_breakdown_sums_to_total", sum(rep.severity_breakdown.values()) == rep.total_plugins, f"sum={sum(rep.severity_breakdown.values())} total={rep.total_plugins}"))

    # 17. channel disable: only drift enabled → only 1 channel signal
    pa = analyze_plugin(
        "p_drift_only",
        tier_history=["low", "high"],
        current_lint_pass=0,
        historical_lint_floor=5,
        latest_drift_penalty=0.5,
        recent_plan_count=5,
        recent_health_scores=[0.9, 0.3],
        enabled_channels=[CHANNEL_DRIFT_SPIKE],
    )
    results.append(("channel_disable_respected", len(pa.channels) == 1 and pa.channels[0].channel == CHANNEL_DRIFT_SPIKE, f"n_channels={len(pa.channels)}"))

    # 18. recommendation populated for HIGH severity
    pa = analyze_plugin(
        "p_rec",
        tier_history=["low", "high"],
        current_lint_pass=0,
        historical_lint_floor=5,
        latest_drift_penalty=0.0,
        recent_plan_count=0,
        recent_health_scores=[],
    )
    has_rec = any(c.recommendation for c in pa.channels)
    results.append(("recommendation_populated_for_severity", has_rec, f"has_rec={has_rec}"))

    return results


def run_self_tests(verbose: bool = False) -> Tuple[int, int]:
    """Run Popper self-tests. Returns (passed, total)."""
    results = _popper_self_tests()
    passed = sum(1 for _, ok, _ in results if ok)
    total = len(results)
    if verbose:
        for name, ok, detail in results:
            mark = "PASS" if ok else "FAIL"
            print(f"[{mark}] {name} — {detail}")
        print(f"\n{passed}/{total} passed")
    return passed, total


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser(description="V1348 VCP Plugin Anomaly Detector")
    p.add_argument("--self-test", action="store_true", help="run Popper self-tests")
    p.add_argument("--verbose", action="store_true", help="verbose output")
    args = p.parse_args()

    if args.self_test:
        passed, total = run_self_tests(verbose=args.verbose)
        sys.exit(0 if passed == total else 1)
    else:
        passed, total = run_self_tests(verbose=True)
        sys.exit(0 if passed == total else 1)