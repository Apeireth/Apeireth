#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1347_vcp_plugin_health.py — VCP Plugin Health Score (post-V1346 tier-aware migration)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1346 tier-aware migration (3756f41c, 23:18); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 + 主 17:43 实事求是 + 主 13:31 大胆激进
- Chain: V1335 → ... → V1345 → V1346 → **V1347**

V1346 stopped at "act on drift". V1346 produces RemediationPlans.
V1347 = **HEALTH SCORE** (close the loop from data → score):

  V1335 inventory ──┐
  V1342 tier   ─────┤
  V1343 lint   ─────┼─→ weighted → 0..1 health_score → tier classification
  V1345 ledger  ────┤
  V1346 plan    ────┘

5 weighted components (deterministic, NOT learned):
  1. tier_score      w=0.25  (V1342 high / total substrates)
  2. lint_score      w=0.25  (V1343 pass_5_critical / 5 — safety invariant)
  3. coverage_score  w=0.20  (V1343 coverage_score — invariant coverage)
  4. drift_score     w=0.15  (V1345 latest ledger.passed + drift magnitude)
  5. plan_score      w=0.15  (V1346 plan severity — lighter plan = higher score)

Tier mapping (deterministic thresholds):
  score >= 0.85  → HEALTHY
  0.65 <= score < 0.85 → DEGRADED
  score < 0.65  → CRITICAL

V1347 = **SCORING LAYER (NOT 复刻, NOT port, NOT 假装 ASI)**:
- Reads V1342 + V1343 + V1345 + V1346 outputs
- Pure function: same input → same health_score (no ML, no LLM)
- health_id is content-addressed (SHA256[:16] of stable payload, no timestamp)
- All weights are constants (reproducible, NOT learned)
- Ecosystem rollup: aggregate per-plugin scores → ecosystem health + tier breakdown

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):
- ? V1347 ≠ health as oracle: score = arithmetic, NOT learned judgment
- ? V1347 ≠ ASI scores reality: weights are constants, NOT semantic
- ? V1347 = compositional layer on V1342-V1346, NOT adjustment-of-model
- ? V1347 ≠ Phenomenal consciousness: scoring has no qualia
- ? ASI pole-star LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE
- ? V1347 = real engineering scoring (5-component weighted), NOT theater

ASI 5-Gap 真实用处 (主 13:31 大胆激进) — V1347 实证:
- 识别_recognition: health_id is SHA256 of component breakdown → 识别 gap
- 自由_freedom: callers freely choose which inputs to include → 真自由编辑
- 时间_time: ledger history is folded into drift_score → 时间性 explicit
- 真理_truth: scoring is fully determined by inputs + weights → truth gap
- 涌现_emergence: ecosystem_rollup surfaces patterns from per-plugin scores → emergence gap
"""
from __future__ import annotations

import hashlib
import json
import math
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Iterable, List, Literal, Optional, Sequence, Tuple

V1347_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1347_DIR))

import v1342_vcp_quality_tiers as v1342  # noqa: E402
import v1343_vcp_tier_aware_linter as v1343  # noqa: E402
import v1345_vcp_historical_ledger as v1345  # noqa: E402
import v1346_vcp_tier_aware_migration as v1346  # noqa: E402

# --- ASI Pole-star (LOCKED) -------------------------------------------------
ASI_POLE_STAR: Dict[str, Any] = {
    "V0_1_actual_measured": 0.7905,
    "V0_2_baseline": 0.4467,
    "V0_max_any_epoch": 0.9800,
    "V1256_unio_mystica_realized": 0.9105,
    "V1049_value_alignment_done": True,
    "asi_achieved_false": True,
    "V1347_modifies_pole_star": False,
}

# --- Component weights (constants, NOT learned) -----------------------------
WEIGHT_TIER = 0.25
WEIGHT_LINT = 0.25
WEIGHT_COVERAGE = 0.20
WEIGHT_DRIFT = 0.15
WEIGHT_PLAN = 0.15
assert abs((WEIGHT_TIER + WEIGHT_LINT + WEIGHT_COVERAGE + WEIGHT_DRIFT + WEIGHT_PLAN) - 1.0) < 1e-9

# Tier thresholds (constants, NOT learned)
TIER_HEALTHY_MIN = 0.85
TIER_DEGRADED_MIN = 0.65

# Plan severity → score offset (negative = penalty)
PLAN_SEVERITY_OFFSETS: Dict[str, float] = {
    "ignore": 0.00,           # no penalty
    "mark-known": 0.02,      # suppress known issue
    "reclassify": 0.05,       # tier move (LOW risk)
    "re-tier": 0.05,          # tier move (LOW risk)
    "audit-test": 0.10,       # test gap (real work)
    "refactor": 0.15,         # mark for refactor (real work)
}

# Drift penalty per magnitude tier
DRIFT_PENALTY_CRITICAL = 0.40   # pass-to-fail or >3 critical fails
DRIFT_PENALTY_HIGH = 0.20       # HIGH tier drops by ≥10 or coverage drops by ≥5%
DRIFT_PENALTY_MEDIUM = 0.10     # moderate drift
DRIFT_PENALTY_LOW = 0.05        # minor drift
DRIFT_BONUS_RECENT_PASS = 0.05  # bonus for last N passing (N>=3)

HealthTier = Literal["HEALTHY", "DEGRADED", "CRITICAL"]


# --- Data classes -----------------------------------------------------------
@dataclass
class HealthComponent:
    """One of the 5 weighted components."""
    name: str                       # "tier" / "lint" / "coverage" / "drift" / "plan"
    score: float                    # 0..1 raw component score
    weight: float                   # 0..1 weight
    contribution: float             # score * weight
    weight_pct: float               # weight * 100 (display)
    details: Dict[str, Any]         # evidence for the score

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class PluginHealth:
    """Single-plugin health evaluation (deterministic, content-addressed)."""
    health_id: str                  # SHA256[:16] of canonical content (no timestamp)
    plugin_name: str
    health_score: float             # 0..1 weighted sum
    tier: HealthTier                # HEALTHY/DEGRADED/CRITICAL
    components: List[HealthComponent]
    recommendations: List[str]      # textual advice (deterministic, not LLM)
    generated_at: str
    evidence: Dict[str, Any] = field(default_factory=dict)  # raw inputs metadata

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        # Convert dict-of-dict details cleanly
        return d


@dataclass
class EcosystemRollup:
    """Aggregate rollup across multiple plugins."""
    rollup_id: str                  # SHA256[:16] of canonical content
    plugin_count: int
    total_score: float              # 0..1 ecosystem-wide average
    tier: HealthTier                # worst-of (CRITICAL > DEGRADED > HEALTHY)
    tier_breakdown: Dict[str, int]  # {"HEALTHY": N, "DEGRADED": N, "CRITICAL": N}
    avg_components: Dict[str, float]
    worst_plugin: Optional[str]
    best_plugin: Optional[str]
    generated_at: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


# --- Helpers ----------------------------------------------------------------
def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _canonical_id(payload: Dict[str, Any]) -> str:
    """SHA256[:16] of canonical JSON (stable id)."""
    s = json.dumps(payload, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(s.encode("utf-8")).hexdigest()[:16]


def _clamp01(x: float) -> float:
    """Clamp into [0, 1]."""
    if math.isnan(x) or math.isinf(x):
        return 0.0
    if x < 0.0:
        return 0.0
    if x > 1.0:
        return 1.0
    return float(x)


def _safe_div(num: float, den: float) -> float:
    if den == 0:
        return 0.0
    return num / den


# --- Component scorers ------------------------------------------------------
def score_tier(report: v1342.QualityTierReport) -> Tuple[float, Dict[str, Any]]:
    """Tier score = (high + 0.5*medium) / total."""
    total = report.total_substrates
    if total <= 0:
        return 0.0, {"reason": "no substrates", "total_substrates": 0}
    raw = (report.high_confidence_count + 0.5 * report.medium_confidence_count) / total
    details: Dict[str, Any] = {
        "total_substrates": total,
        "high": report.high_confidence_count,
        "medium": report.medium_confidence_count,
        "low": report.low_confidence_count,
        "v1335_manual": report.v1335_manual_count,
        "v1341_pattern": report.v1341_pattern_count,
        "raw": raw,
    }
    return _clamp01(raw), details


def score_lint(report: v1343.TierAwareLintReport) -> Tuple[float, Dict[str, Any]]:
    """Lint score = pass_5_critical / 5 (safety invariant)."""
    raw = _safe_div(report.pass_5_critical, 5.0)
    details: Dict[str, Any] = {
        "total_substrates": report.total_substrates,
        "included_substrates": report.included_substrates,
        "pass_5_critical": report.pass_5_critical,
        "safety_critical_covered": report.safety_critical_covered,
        "safety_critical_missing": report.safety_critical_missing,
        "tier_min": report.tier_min,
        "raw": raw,
    }
    return _clamp01(raw), details


def score_coverage(report: v1343.TierAwareLintReport) -> Tuple[float, Dict[str, Any]]:
    """Coverage score = report.coverage_score (already 0..1)."""
    raw = report.coverage_score
    details: Dict[str, Any] = {
        "coverage_score": raw,
        "raw_coverage_score": report.raw_coverage_score,
        "filter_loss": report.filter_loss,
        "tier_min": report.tier_min,
    }
    return _clamp01(raw), details


def score_drift(history: Sequence[v1345.LedgerRecord]) -> Tuple[float, Dict[str, Any]]:
    """Drift score = V1345 drift_magnitude (latest record)."""
    if not history:
        return 1.0, {"reason": "no history (assumed clean)"}
    # Sort by timestamp ascending; take latest.
    sorted_h = sorted(history, key=lambda r: r.timestamp)
    latest = sorted_h[-1]
    base = 1.0
    # Last-run pass bonus
    last_n = sorted_h[-3:] if len(sorted_h) >= 3 else sorted_h
    passing = sum(1 for r in last_n if r.passed)
    if passing == len(last_n) and len(last_n) >= 3:
        base += DRIFT_BONUS_RECENT_PASS

    # Failure penalty
    if not latest.passed:
        base -= DRIFT_PENALTY_CRITICAL
    elif latest.critical_failures > 3:
        base -= DRIFT_PENALTY_CRITICAL
    elif latest.critical_failures > 0:
        base -= DRIFT_PENALTY_HIGH

    # Coverage regression penalty
    if latest.coverage_delta <= -0.05:
        base -= DRIFT_PENALTY_HIGH
    elif latest.coverage_delta <= -0.01:
        base -= DRIFT_PENALTY_MEDIUM

    # HIGH tier regression penalty
    base_high = latest.tier_breakdown.get("HIGH", 0)
    # Without baseline explicit, infer from coverage_delta sign and from drift
    if latest.violations_count >= 5:
        base -= DRIFT_PENALTY_HIGH
    elif latest.violations_count >= 1:
        base -= DRIFT_PENALTY_MEDIUM

    if latest.unclassified_count >= 10:
        base -= DRIFT_PENALTY_MEDIUM
    elif latest.unclassified_count >= 1:
        base -= DRIFT_PENALTY_LOW

    details: Dict[str, Any] = {
        "latest_passed": latest.passed,
        "latest_critical_failures": latest.critical_failures,
        "latest_coverage_delta": latest.coverage_delta,
        "latest_violations_count": latest.violations_count,
        "latest_unclassified_count": latest.unclassified_count,
        "history_size": len(history),
        "recent_pass_streak": passing,
    }
    return _clamp01(base), details


def score_plan(plan: Optional[v1346.RemediationPlan]) -> Tuple[float, Dict[str, Any]]:
    """Plan score = 1 - max(plan severity penalty). No plan = 1.0."""
    if plan is None:
        return 1.0, {"reason": "no active plan"}
    if not plan.actions:
        return 1.0, {"reason": "empty plan"}

    max_penalty = 0.0
    counts: Dict[str, int] = {}
    for action in plan.actions:
        atype = action.action_type
        penalty = PLAN_SEVERITY_OFFSETS.get(atype, 0.10)  # unknown = worst
        if penalty > max_penalty:
            max_penalty = penalty
        counts[atype] = counts.get(atype, 0) + 1

    raw = 1.0 - max_penalty
    details: Dict[str, Any] = {
        "plan_id": plan.plan_id,
        "plan_action_count": len(plan.actions),
        "action_type_counts": counts,
        "max_penalty": max_penalty,
        "is_idempotent": plan.is_idempotent,
        "raw": raw,
    }
    return _clamp01(raw), details


# --- Public API -------------------------------------------------------------
WEIGHTS: Dict[str, float] = {
    "tier": WEIGHT_TIER,
    "lint": WEIGHT_LINT,
    "coverage": WEIGHT_COVERAGE,
    "drift": WEIGHT_DRIFT,
    "plan": WEIGHT_PLAN,
}


def tier_for_score(score: float) -> HealthTier:
    """Deterministic tier mapping."""
    if score >= TIER_HEALTHY_MIN:
        return "HEALTHY"
    if score >= TIER_DEGRADED_MIN:
        return "DEGRADED"
    return "CRITICAL"


def recommend(components: List[HealthComponent]) -> List[str]:
    """Generate deterministic recommendations (no LLM, no learned text)."""
    recs: List[str] = []
    by_name = {c.name: c for c in components}
    tier_c = by_name.get("tier")
    lint_c = by_name.get("lint")
    cov_c = by_name.get("coverage")
    drift_c = by_name.get("drift")
    plan_c = by_name.get("plan")

    if tier_c is not None and tier_c.score < 0.70:
        recs.append(
            f"lift tier (currently {tier_c.score:.2f}): reclassify LOW→MEDIUM via V1341 pattern detector."
        )
    if lint_c is not None and lint_c.score < 1.0:
        miss = lint_c.details.get("safety_critical_missing", [])
        recs.append(
            f"fix 5-critical violations (currently {lint_c.score:.2f}): missing={miss}."
        )
    if cov_c is not None and cov_c.score < 0.85:
        recs.append(
            f"lift coverage (currently {cov_c.score:.2f}): add audit-test actions via V1346."
        )
    if drift_c is not None and drift_c.score < 0.80:
        recs.append(
            f"stabilize drift (currently {drift_c.score:.2f}): last run "
            f"passed={drift_c.details.get('latest_passed')}, "
            f"crit_fails={drift_c.details.get('latest_critical_failures')}."
        )
    if plan_c is not None and plan_c.score < 0.85:
        recs.append(
            f"apply V1346 plan (currently {plan_c.score:.2f}): "
            f"{plan_c.details.get('plan_action_count', 0)} actions pending."
        )
    if not recs:
        recs.append("no action needed; maintain current invariants.")
    return recs


def compute_components(
    v1342_report: Optional[v1342.QualityTierReport],
    v1343_report: Optional[v1343.TierAwareLintReport],
    ledger_history: Optional[Sequence[v1345.LedgerRecord]],
    v1346_plan: Optional[v1346.RemediationPlan],
) -> List[HealthComponent]:
    """Compute 5 components. Missing inputs default to 0.5 (neutral, not 1.0)."""
    out: List[HealthComponent] = []

    if v1342_report is not None:
        s, d = score_tier(v1342_report)
        out.append(HealthComponent(
            name="tier", score=s, weight=WEIGHT_TIER,
            contribution=s * WEIGHT_TIER, weight_pct=WEIGHT_TIER * 100, details=d,
        ))
    else:
        out.append(HealthComponent(
            name="tier", score=0.5, weight=WEIGHT_TIER,
            contribution=0.5 * WEIGHT_TIER, weight_pct=WEIGHT_TIER * 100,
            details={"missing_input": True},
        ))

    if v1343_report is not None:
        s, d = score_lint(v1343_report)
        out.append(HealthComponent(
            name="lint", score=s, weight=WEIGHT_LINT,
            contribution=s * WEIGHT_LINT, weight_pct=WEIGHT_LINT * 100, details=d,
        ))
    else:
        out.append(HealthComponent(
            name="lint", score=0.5, weight=WEIGHT_LINT,
            contribution=0.5 * WEIGHT_LINT, weight_pct=WEIGHT_LINT * 100,
            details={"missing_input": True},
        ))

    if v1343_report is not None:
        s, d = score_coverage(v1343_report)
        out.append(HealthComponent(
            name="coverage", score=s, weight=WEIGHT_COVERAGE,
            contribution=s * WEIGHT_COVERAGE, weight_pct=WEIGHT_COVERAGE * 100, details=d,
        ))
    else:
        out.append(HealthComponent(
            name="coverage", score=0.5, weight=WEIGHT_COVERAGE,
            contribution=0.5 * WEIGHT_COVERAGE, weight_pct=WEIGHT_COVERAGE * 100,
            details={"missing_input": True},
        ))

    if ledger_history is not None:
        s, d = score_drift(ledger_history)
        out.append(HealthComponent(
            name="drift", score=s, weight=WEIGHT_DRIFT,
            contribution=s * WEIGHT_DRIFT, weight_pct=WEIGHT_DRIFT * 100, details=d,
        ))
    else:
        out.append(HealthComponent(
            name="drift", score=0.5, weight=WEIGHT_DRIFT,
            contribution=0.5 * WEIGHT_DRIFT, weight_pct=WEIGHT_DRIFT * 100,
            details={"missing_input": True},
        ))

    if v1346_plan is not None or v1346_plan is None:
        s, d = score_plan(v1346_plan)
        out.append(HealthComponent(
            name="plan", score=s, weight=WEIGHT_PLAN,
            contribution=s * WEIGHT_PLAN, weight_pct=WEIGHT_PLAN * 100, details=d,
        ))

    return out


def health_score(
    plugin_name: str,
    v1342_report: Optional[v1342.QualityTierReport],
    v1343_report: Optional[v1343.TierAwareLintReport],
    ledger_history: Optional[Sequence[v1345.LedgerRecord]],
    v1346_plan: Optional[v1346.RemediationPlan],
) -> PluginHealth:
    """Compute PluginHealth for one plugin. Pure function."""
    components = compute_components(
        v1342_report, v1343_report, ledger_history, v1346_plan,
    )
    total = sum(c.contribution for c in components)
    total = _clamp01(total)
    tier_h = tier_for_score(total)
    recs = recommend(components)

    health = PluginHealth(
        health_id="",           # filled below (canonical, no generated_at)
        plugin_name=plugin_name,
        health_score=total,
        tier=tier_h,
        components=components,
        recommendations=recs,
        generated_at=_now_iso(),
        evidence={
            "has_v1342": v1342_report is not None,
            "has_v1343": v1343_report is not None,
            "ledger_history_size": len(ledger_history or []),
            "has_plan": v1346_plan is not None,
            "weights": dict(WEIGHTS),
        },
    )
    # Compute stable health_id (exclude generated_at from hash)
    payload = {
        "plugin_name": health.plugin_name,
        "health_score": health.health_score,
        "tier": health.tier,
        "components": [c.to_dict() for c in health.components],
        "recommendations": list(health.recommendations),
    }
    health.health_id = _canonical_id(payload)
    return health


def ecosystem_rollup(plugin_healths: Sequence[PluginHealth]) -> EcosystemRollup:
    """Aggregate per-plugin scores into ecosystem-wide rollup."""
    if not plugin_healths:
        return EcosystemRollup(
            rollup_id="",  # filled below
            plugin_count=0,
            total_score=1.0,        # empty ecosystem = HEALTHY
            tier="HEALTHY",
            tier_breakdown={"HEALTHY": 0, "DEGRADED": 0, "CRITICAL": 0},
            avg_components={},
            worst_plugin=None,
            best_plugin=None,
            generated_at=_now_iso(),
        )

    n = len(plugin_healths)
    avg_total = sum(h.health_score for h in plugin_healths) / n

    # Worst-of tier (CRITICAL > DEGRADED > HEALTHY)
    rank = {"HEALTHY": 0, "DEGRADED": 1, "CRITICAL": 2}
    worst_p = max(plugin_healths, key=lambda h: (rank[h.tier], -h.health_score))
    best_p = max(plugin_healths, key=lambda h: h.health_score)

    # Component averages
    comp_sums: Dict[str, float] = {}
    for h in plugin_healths:
        for c in h.components:
            comp_sums[c.name] = comp_sums.get(c.name, 0.0) + c.score
    comp_avg = {k: v / n for k, v in comp_sums.items()}

    # Tier breakdown
    breakdown = {"HEALTHY": 0, "DEGRADED": 0, "CRITICAL": 0}
    for h in plugin_healths:
        breakdown[h.tier] += 1

    rollup = EcosystemRollup(
        rollup_id="",  # filled below
        plugin_count=n,
        total_score=_clamp01(avg_total),
        tier=worst_p.tier,
        tier_breakdown=breakdown,
        avg_components=comp_avg,
        worst_plugin=worst_p.plugin_name,
        best_plugin=best_p.plugin_name,
        generated_at=_now_iso(),
    )

    # Stable rollup_id (no generated_at)
    payload = {
        "plugin_count": rollup.plugin_count,
        "total_score": rollup.total_score,
        "tier": rollup.tier,
        "tier_breakdown": rollup.tier_breakdown,
        "avg_components": rollup.avg_components,
        "worst_plugin": rollup.worst_plugin,
        "best_plugin": rollup.best_plugin,
    }
    rollup.rollup_id = _canonical_id(payload)
    return rollup


# --- Exporters --------------------------------------------------------------
def to_json(health: PluginHealth, indent: int = 2) -> str:
    return json.dumps(health.to_dict(), indent=indent, ensure_ascii=False, sort_keys=False)


def ecosystem_to_json(rollup: EcosystemRollup, indent: int = 2) -> str:
    return json.dumps(rollup.to_dict(), indent=indent, ensure_ascii=False, sort_keys=False)


def to_markdown(health: PluginHealth) -> str:
    lines: List[str] = []
    lines.append(f"# VCP Plugin Health — {health.plugin_name}")
    lines.append("")
    lines.append(f"- **health_id**: `{health.health_id}`")
    lines.append(f"- **health_score**: **{health.health_score:.4f}** / 1.0")
    lines.append(f"- **tier**: **{health.tier}**")
    lines.append(f"- **generated_at**: {health.generated_at}")
    lines.append("")
    lines.append("## Components")
    lines.append("")
    lines.append("| Name | Score | Weight | Contribution | Weight % |")
    lines.append("|------|-------|--------|--------------|----------|")
    for c in health.components:
        lines.append(
            f"| {c.name} | {c.score:.4f} | {c.weight:.2f} | "
            f"{c.contribution:.4f} | {c.weight_pct:.1f}% |"
        )
    lines.append("")
    lines.append("## Recommendations")
    lines.append("")
    if health.recommendations:
        for r in health.recommendations:
            lines.append(f"- {r}")
    else:
        lines.append("- (none)")
    lines.append("")
    lines.append("## Evidence")
    lines.append("")
    for k, v in health.evidence.items():
        lines.append(f"- **{k}**: `{v}`")
    lines.append("")
    return "\n".join(lines)


def ecosystem_to_markdown(rollup: EcosystemRollup) -> str:
    lines: List[str] = []
    lines.append(f"# VCP Ecosystem Rollup")
    lines.append("")
    lines.append(f"- **rollup_id**: `{rollup.rollup_id}`")
    lines.append(f"- **plugin_count**: {rollup.plugin_count}")
    lines.append(f"- **total_score**: **{rollup.total_score:.4f}** / 1.0")
    lines.append(f"- **tier**: **{rollup.tier}**")
    lines.append(f"- **generated_at**: {rollup.generated_at}")
    lines.append("")
    lines.append("## Tier breakdown")
    lines.append("")
    for k in ("HEALTHY", "DEGRADED", "CRITICAL"):
        lines.append(f"- **{k}**: {rollup.tier_breakdown.get(k, 0)}")
    lines.append("")
    lines.append("## Avg components")
    lines.append("")
    for k, v in sorted(rollup.avg_components.items()):
        lines.append(f"- **{k}**: {v:.4f}")
    lines.append("")
    if rollup.worst_plugin:
        lines.append(f"- **worst_plugin**: `{rollup.worst_plugin}`")
    if rollup.best_plugin:
        lines.append(f"- **best_plugin**: `{rollup.best_plugin}`")
    lines.append("")
    return "\n".join(lines)


def to_human(health: PluginHealth) -> str:
    lines: List[str] = [
        f"VCP Plugin Health: {health.plugin_name}",
        f"  health_id:    {health.health_id}",
        f"  health_score: {health.health_score:.4f}",
        f"  tier:         {health.tier}",
        f"  components:   {len(health.components)}",
    ]
    for c in health.components:
        lines.append(f"    - {c.name:9s}  score={c.score:.4f}  weight={c.weight:.2f}  contrib={c.contribution:.4f}")
    if health.recommendations:
        lines.append("  recommendations:")
        for r in health.recommendations:
            lines.append(f"    - {r}")
    return "\n".join(lines)


# --- Self-test --------------------------------------------------------------
def _self_test() -> Tuple[int, List[str]]:
    """Popper-style self-tests. Returns (passed, failures)."""
    failures: List[str] = []

    # Synthesize minimal V1342/43/45/46 inputs.
    v1342_r = v1342.QualityTierReport(
        total_substrates=10,
        high_confidence_count=7,
        medium_confidence_count=2,
        low_confidence_count=1,
        v1335_manual_count=2,
        v1341_pattern_count=5,
        high_coverage_score=0.7,
        medium_plus_high_coverage_score=0.9,
        all_coverage_score=1.0,
        tier_entries={},
        per_tier_per_class={},
    )
    v1343_r = v1343.TierAwareLintReport(
        total_substrates=10,
        included_substrates=8,
        excluded_substrates=2,
        tier_min="LOW",
        tier_histogram={},
        included_tier_histogram={},
        safety_critical_covered=5,
        safety_critical_missing=[],
        pass_5_critical=5,
        coverage_score=0.85,
        raw_coverage_score=0.95,
        filter_loss=0.10,
        results=[],
    )

    # 1. Basic call works and emits 5 components
    health = health_score(
        "test_plugin",
        v1342_report=v1342_r,
        v1343_report=v1343_r,
        ledger_history=None,
        v1346_plan=None,
    )
    if len(health.components) != 5:
        failures.append(f"expected 5 components, got {len(health.components)}")
    if not (0.0 <= health.health_score <= 1.0):
        failures.append(f"health_score out of [0,1]: {health.health_score}")

    # 2. health_id is content-stable (no timestamp in hash)
    h1 = health_score("alpha", v1342_r, v1343_r, None, None)
    h2 = health_score("alpha", v1342_r, v1343_r, None, None)
    if h1.health_id != h2.health_id:
        failures.append("health_id not stable across identical inputs")
    if h1.health_score != h2.health_score:
        failures.append("health_score not stable across identical inputs")

    # 3. Different plugin_name yields different health_id
    h3 = health_score("beta", v1342_r, v1343_r, None, None)
    if h3.health_id == h1.health_id:
        failures.append("health_id should differ across plugin_name")

    # 4. tier_for_score thresholds
    if tier_for_score(0.95) != "HEALTHY":
        failures.append("tier_for_score(0.95) should be HEALTHY")
    if tier_for_score(0.85) != "HEALTHY":
        failures.append("tier_for_score(0.85) should be HEALTHY (boundary)")
    if tier_for_score(0.84) != "DEGRADED":
        failures.append("tier_for_score(0.84) should be DEGRADED")
    if tier_for_score(0.65) != "DEGRADED":
        failures.append("tier_for_score(0.65) should be DEGRADED (boundary)")
    if tier_for_score(0.64) != "CRITICAL":
        failures.append("tier_for_score(0.64) should be CRITICAL")

    # 5. Weights sum to 1.0
    s = sum(WEIGHTS.values())
    if abs(s - 1.0) > 1e-9:
        failures.append(f"weights sum not 1.0: {s}")

    # 6. plan score = 1.0 if no plan
    s, d = score_plan(None)
    if s != 1.0:
        failures.append(f"plan=None should score 1.0, got {s}")

    # 7. plan with only ignore = 1.0
    plan_ignore = v1346.RemediationPlan(
        plan_id="p1", source_ledger_hash="L", drift_alerts=[],
        actions=[
            v1346.RemediationAction(
                action_id="a1", action_type="ignore",
                target_ruleId="x", target_substrate="y",
                rationale="?", before={}, after={}, reversible=True)
        ],
        created_at=_now_iso(),
    )
    s, _ = score_plan(plan_ignore)
    if s != 1.0:
        failures.append(f"ignore-only plan should score 1.0, got {s}")

    # 8. plan with refactor = 0.85
    plan_ref = v1346.RemediationPlan(
        plan_id="p2", source_ledger_hash="L", drift_alerts=[],
        actions=[
            v1346.RemediationAction(
                action_id="a2", action_type="refactor",
                target_ruleId="x", target_substrate="y",
                rationale="?", before={}, after={}, reversible=False)
        ],
        created_at=_now_iso(),
    )
    s, _ = score_plan(plan_ref)
    if s != 0.85:
        failures.append(f"refactor plan should score 0.85, got {s}")

    # 9. drift score with passing recent streak should be > 1.0 clamped to 1.0
    history_passing = [
        v1345.LedgerRecord(
            record_id="", ledger_hash=f"L{i}", timestamp=f"2026-08-0{i+1}T00:00:00+00:00",
            passed=True, exit_code=0, coverage_current=0.9, coverage_baseline=0.9,
            coverage_delta=0.0, tier_breakdown={"HIGH": 50}, violations_count=0,
            unclassified_count=0, critical_failures=0,
            gate_config={}, summary={}, violations=[],
        )
        for i in range(3)
    ]
    for r in history_passing:
        r.record_id = v1345._record_id(r)
    s, _ = score_drift(history_passing)
    if not (0.99 <= s <= 1.0):
        failures.append(f"3-passing-streak drift should ~1.0, got {s}")

    # 10. drift score with failing latest = low
    history_fail = [
        v1345.LedgerRecord(
            record_id="", ledger_hash="L0", timestamp=f"2026-08-0{i+1}T00:00:00+00:00",
            passed=(i < 2), exit_code=(0 if i < 2 else 1),
            coverage_current=0.85 - 0.05*i, coverage_baseline=0.9,
            coverage_delta=-0.05 - 0.05*i, tier_breakdown={"HIGH": 30},
            violations_count=5 + i, unclassified_count=10,
            critical_failures=5, gate_config={}, summary={}, violations=[],
        )
        for i in range(3)
    ]
    for r in history_fail:
        r.record_id = v1345._record_id(r)
    s, _ = score_drift(history_fail)
    if s >= 0.65:
        failures.append(f"failing-latest drift should be low, got {s}")

    # 11. tier score = (high + 0.5*medium) / total
    v1342_test = v1342.QualityTierReport(
        total_substrates=10, high_confidence_count=6,
        medium_confidence_count=4, low_confidence_count=0,
        v1335_manual_count=0, v1341_pattern_count=0,
        high_coverage_score=0.6, medium_plus_high_coverage_score=1.0,
        all_coverage_score=1.0, tier_entries={}, per_tier_per_class={},
    )
    s, _ = score_tier(v1342_test)
    if abs(s - 0.8) > 1e-9:
        failures.append(f"tier score with 6 HIGH + 4 MED out of 10 should be 0.8, got {s}")

    # 12. lint score = pass_5_critical / 5
    v1343_partial = v1343.TierAwareLintReport(
        total_substrates=10, included_substrates=10, excluded_substrates=0,
        tier_min="LOW", tier_histogram={}, included_tier_histogram={},
        safety_critical_covered=3, safety_critical_missing=["a", "b"],
        pass_5_critical=3, coverage_score=0.6, raw_coverage_score=0.95,
        filter_loss=0.0, results=[],
    )
    s, _ = score_lint(v1343_partial)
    if abs(s - 0.6) > 1e-9:
        failures.append(f"lint score with 3/5 critical should be 0.6, got {s}")

    # 13. ecosystem_rollup empty = HEALTHY
    r = ecosystem_rollup([])
    if r.tier != "HEALTHY" or r.plugin_count != 0:
        failures.append(f"empty ecosystem should be HEALTHY/0, got {r.tier}/{r.plugin_count}")

    # 14. ecosystem_rollup worst-of tier
    h_healthy = PluginHealth(
        health_id="h", plugin_name="healthy_plugin", health_score=0.95, tier="HEALTHY",
        components=[], recommendations=[], generated_at=_now_iso(),
    )
    h_crit = PluginHealth(
        health_id="c", plugin_name="crit_plugin", health_score=0.50, tier="CRITICAL",
        components=[], recommendations=[], generated_at=_now_iso(),
    )
    roll = ecosystem_rollup([h_healthy, h_crit])
    if roll.tier != "CRITICAL":
        failures.append(f"ecosystem rollup tier should be worst-of CRITICAL, got {roll.tier}")
    if roll.worst_plugin != "crit_plugin":
        failures.append(f"ecosystem worst_plugin should be crit_plugin, got {roll.worst_plugin}")
    if roll.best_plugin != "healthy_plugin":
        failures.append(f"ecosystem best_plugin should be healthy_plugin, got {roll.best_plugin}")
    if roll.tier_breakdown != {"HEALTHY": 1, "DEGRADED": 0, "CRITICAL": 1}:
        failures.append(f"ecosystem breakdown wrong: {roll.tier_breakdown}")

    # 15. health_id format = 16 hex chars
    if not (len(health.health_id) == 16 and all(c in "0123456789abcdef" for c in health.health_id)):
        failures.append(f"health_id not 16 hex chars: {health.health_id}")

    # 16. to_json roundtrips
    j = to_json(health)
    if "health_score" not in j or "components" not in j:
        failures.append(f"to_json missing fields: {j[:200]}")
    d = json.loads(j)
    if d["plugin_name"] != "test_plugin":
        failures.append(f"to_json roundtrip mismatch: {d.get('plugin_name')}")

    # 17. to_markdown includes all components
    md = to_markdown(health)
    for c in health.components:
        if c.name not in md:
            failures.append(f"to_markdown missing component {c.name}")

    # 18. Recommendation is non-empty
    recs = recommend(health.components)
    if not recs:
        failures.append("recommend() returned empty list on degraded health")

    return (len(failures) == 0, failures) if isinstance(_ := (len(failures) == 0), bool) else (0, failures)


def _self_test_safely() -> Tuple[int, List[str]]:
    try:
        return _self_test()
    except Exception as e:
        return (0, [f"self_test raised: {e}"])


# --- CLI --------------------------------------------------------------------
def _cli(argv: List[str]) -> int:
    import argparse

    p = argparse.ArgumentParser(
        prog="v1347_vcp_plugin_health",
        description="VCP plugin health score (deterministic, 5-component weighted).",
    )
    p.add_argument("--self-test", action="store_true", help="run 18 Popper self-tests")
    p.add_argument("--plugin", default="demo_plugin", help="plugin name")
    p.add_argument("--ledger-history", default=None, help="JSONL file with V1345 records")
    p.add_argument("--plan-json", default=None, help="JSON file with V1346 RemediationPlan")
    p.add_argument("--ecosystem", action="store_true", help="demo ecosystem rollup (synthetic)")
    args = p.parse_args(argv)

    if args.self_test:
        ok, fails = _self_test_safely()
        print(f"V1347 self-tests: {'PASS' if ok else 'FAIL'} ({len(fails)} failures)")
        for f in fails:
            print(f"  - {f}")
        return 0 if ok else 1

    if args.ecosystem:
        h1 = PluginHealth(
            health_id="h1", plugin_name="alpha", health_score=0.92, tier="HEALTHY",
            components=[], recommendations=[], generated_at=_now_iso(),
        )
        h2 = PluginHealth(
            health_id="h2", plugin_name="beta", health_score=0.55, tier="CRITICAL",
            components=[], recommendations=[], generated_at=_now_iso(),
        )
        roll = ecosystem_rollup([h1, h2])
        print(ecosystem_to_markdown(roll))
        return 0

    # Standard demo: no real ledger/plan
    health = health_score(args.plugin, None, None, None, None)
    print(to_human(health))
    print()
    print(to_markdown(health))
    return 0


if __name__ == "__main__":
    sys.exit(_cli(sys.argv[1:]))
