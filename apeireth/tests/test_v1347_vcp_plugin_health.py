#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Tests for v1347_vcp_plugin_health.py — VCP Plugin Health Score (post-V1346 tier-aware migration)

Chain: V1335 → ... → V1345 → V1346 → V1347

V1347 = deterministic 5-component weighted health scoring.
- tier (V1342)        weight 0.25
- lint (V1343)        weight 0.25
- coverage (V1343)    weight 0.20
- drift (V1345)       weight 0.15
- plan (V1346)        weight 0.15

This test file covers:
- 18 module self-tests proxy (via _self_test_safely)
- 24 pytest tests across 5 categories: tier classification, scoring math,
  determinism, export format, edge cases, integration with V1346.
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

# Bootstrap import path.
_TESTS_DIR = Path(__file__).resolve().parent
_REPO_DIR = _TESTS_DIR.parent
sys.path.insert(0, str(_REPO_DIR))

import v1342_vcp_quality_tiers as v1342  # noqa: E402
import v1343_vcp_tier_aware_linter as v1343  # noqa: E402
import v1345_vcp_historical_ledger as v1345  # noqa: E402
import v1346_vcp_tier_aware_migration as v1346  # noqa: E402
import v1347_vcp_plugin_health as v1347  # noqa: E402


# --- Test fixtures ----------------------------------------------------------
def _mk_v1342(high: int = 7, medium: int = 2, low: int = 1) -> v1342.QualityTierReport:
    return v1342.QualityTierReport(
        total_substrates=high + medium + low,
        high_confidence_count=high,
        medium_confidence_count=medium,
        low_confidence_count=low,
        v1335_manual_count=2,
        v1341_pattern_count=high - 2 if high > 2 else 0,
        high_coverage_score=high / max(high + medium + low, 1),
        medium_plus_high_coverage_score=(high + medium) / max(high + medium + low, 1),
        all_coverage_score=1.0,
        tier_entries={},
        per_tier_per_class={},
    )


def _mk_v1343(
    pass_5_critical: int = 5,
    coverage_score: float = 0.85,
) -> v1343.TierAwareLintReport:
    return v1343.TierAwareLintReport(
        total_substrates=10,
        included_substrates=8,
        excluded_substrates=2,
        tier_min="LOW",
        tier_histogram={"HIGH": 5, "MED": 3, "LOW": 2},
        included_tier_histogram={"HIGH": 5, "MED": 3},
        safety_critical_covered=pass_5_critical,
        safety_critical_missing=[],
        pass_5_critical=pass_5_critical,
        coverage_score=coverage_score,
        raw_coverage_score=coverage_score + 0.05,
        filter_loss=0.05,
        results=[],
    )


def _mk_history(passing_count: int = 3, failing_count: int = 0) -> list:
    """Build a ledger history of (passing_count) passing then (failing_count) failing."""
    out = []
    for i in range(passing_count):
        r = v1345.LedgerRecord(
            record_id="",
            ledger_hash=f"LP{i}",
            timestamp=f"2026-08-0{i+1}T00:00:00+00:00",
            passed=True,
            exit_code=0,
            coverage_current=0.90,
            coverage_baseline=0.90,
            coverage_delta=0.0,
            tier_breakdown={"HIGH": 50},
            violations_count=0,
            unclassified_count=0,
            critical_failures=0,
            gate_config={},
            summary={},
            violations=[],
        )
        r.record_id = v1345._record_id(r)
        out.append(r)
    for j in range(failing_count):
        r = v1345.LedgerRecord(
            record_id="",
            ledger_hash=f"LF{j}",
            timestamp=f"2026-08-0{j+1 + passing_count}T00:00:00+00:00",
            passed=False,
            exit_code=1,
            coverage_current=0.80,
            coverage_baseline=0.90,
            coverage_delta=-0.10,
            tier_breakdown={"HIGH": 30},
            violations_count=10,
            unclassified_count=5,
            critical_failures=5,
            gate_config={},
            summary={},
            violations=[],
        )
        r.record_id = v1345._record_id(r)
        out.append(r)
    return out


def _mk_plan(action_type: str = "ignore", n: int = 1) -> v1346.RemediationPlan:
    actions = []
    for i in range(n):
        actions.append(v1346.RemediationAction(
            action_id="",
            action_type=action_type,
            target_ruleId=f"rule_{i}",
            target_substrate=f"substrate_{i}",
            rationale="test",
            before={},
            after={},
            reversible=(action_type in {"ignore", "mark-known", "reclassify", "re-tier"}),
        ))
    for a in actions:
        a.action_id = v1346._action_id(a)
    return v1346.RemediationPlan(
        plan_id="",
        source_ledger_hash="L_TEST",
        drift_alerts=[],
        actions=actions,
        created_at=v1347._now_iso(),
        notes="test",
    )


# ============================================================================
# Category 1: Self-tests (proxy for 18 Popper cases)
# ============================================================================
def test_v1347_popper_self_tests_pass():
    ok, fails = v1347._self_test_safely()
    assert ok, f"V1347 self-tests failed: {fails}"


# ============================================================================
# Category 2: tier_for_score (boundary cases)
# ============================================================================
def test_v1347_tier_healthy_boundary_above():
    """0.95 → HEALTHY"""
    assert v1347.tier_for_score(0.95) == "HEALTHY"


def test_v1347_tier_healthy_boundary_exact():
    """0.85 (boundary) → HEALTHY"""
    assert v1347.tier_for_score(0.85) == "HEALTHY"


def test_v1347_tier_degraded_boundary_below():
    """0.84 → DEGRADED"""
    assert v1347.tier_for_score(0.84) == "DEGRADED"


def test_v1347_tier_degraded_boundary_exact():
    """0.65 (boundary) → DEGRADED"""
    assert v1347.tier_for_score(0.65) == "DEGRADED"


def test_v1347_tier_critical_below():
    """0.0 → CRITICAL"""
    assert v1347.tier_for_score(0.0) == "CRITICAL"


# ============================================================================
# Category 3: Component scoring math
# ============================================================================
def test_v1347_score_tier_known_formula():
    """6 HIGH + 4 MED + 0 LOW out of 10 → 0.80"""
    r = _mk_v1342(high=6, medium=4, low=0)
    s, d = v1347.score_tier(r)
    assert abs(s - 0.8) < 1e-9, f"tier score should be 0.8, got {s}"
    assert d["total_substrates"] == 10


def test_v1347_score_lint_partial():
    """3/5 critical → 0.6"""
    r = _mk_v1343(pass_5_critical=3)
    s, d = v1347.score_lint(r)
    assert abs(s - 0.6) < 1e-9, f"lint score should be 0.6, got {s}"
    assert d["pass_5_critical"] == 3


def test_v1347_score_coverage_passthrough():
    """coverage_score is passed through as-is"""
    r = _mk_v1343(coverage_score=0.73)
    s, _ = v1347.score_coverage(r)
    assert abs(s - 0.73) < 1e-9


def test_v1347_score_plan_none_is_one():
    """No plan → score 1.0 (no penalty for absence)"""
    s, d = v1347.score_plan(None)
    assert s == 1.0
    assert "no active plan" in d.get("reason", "")


def test_v1347_score_plan_severity_ladder():
    """Plan score decreases with severity: ignore > mark-known > reclassify > re-tier > audit-test > refactor."""
    ignore_s, _ = v1347.score_plan(_mk_plan("ignore"))
    mark_s, _ = v1347.score_plan(_mk_plan("mark-known"))
    reclassify_s, _ = v1347.score_plan(_mk_plan("reclassify"))
    retier_s, _ = v1347.score_plan(_mk_plan("re-tier"))
    audit_s, _ = v1347.score_plan(_mk_plan("audit-test"))
    refactor_s, _ = v1347.score_plan(_mk_plan("refactor"))
    assert ignore_s == 1.0, f"ignore should be 1.0, got {ignore_s}"
    assert mark_s < ignore_s, f"mark-known {mark_s} should be < ignore {ignore_s}"
    assert reclassify_s < mark_s, "reclassify < mark-known"
    assert retier_s == reclassify_s, "re-tier == reclassify (same offset)"
    assert audit_s < reclassify_s, "audit-test < reclassify"
    assert refactor_s < audit_s, f"refactor {refactor_s} < audit-test {audit_s}"


def test_v1347_score_drift_no_history_is_clean():
    """No history → drift score 1.0 (assumed clean)"""
    s, d = v1347.score_drift([])
    assert s == 1.0
    assert "no history" in d["reason"]


def test_v1347_score_drift_passing_streak_high():
    """3 passing records → ~1.0"""
    s, _ = v1347.score_drift(_mk_history(passing_count=3, failing_count=0))
    assert s >= 0.99, f"3-pass streak should ~1.0, got {s}"


def test_v1347_score_drift_failing_latest_low():
    """2 passing + 1 failing latest → much lower"""
    s, _ = v1347.score_drift(_mk_history(passing_count=2, failing_count=1))
    assert s < 0.65, f"failing-latest drift should be low, got {s}"


# ============================================================================
# Category 4: health_score determinism + content-addressing
# ============================================================================
def test_v1347_health_score_idempotent():
    """Same inputs → same health_id"""
    v2_r = _mk_v1342()
    v3_r = _mk_v1343()
    h1 = v1347.health_score("alpha", v2_r, v3_r, None, None)
    h2 = v1347.health_score("alpha", v2_r, v3_r, None, None)
    assert h1.health_id == h2.health_id
    assert h1.health_score == h2.health_score


def test_v1347_health_score_plugin_name_changes_id():
    """Different plugin_name → different health_id"""
    v2_r = _mk_v1342()
    v3_r = _mk_v1343()
    h1 = v1347.health_score("alpha", v2_r, v3_r, None, None)
    h2 = v1347.health_score("beta", v2_r, v3_r, None, None)
    assert h1.health_id != h2.health_id


def test_v1347_health_id_is_16_hex():
    """health_id format: 16 hex chars"""
    h = v1347.health_score("p", None, None, None, None)
    assert len(h.health_id) == 16
    assert all(c in "0123456789abcdef" for c in h.health_id)


def test_v1347_health_score_in_range():
    """health_score is in [0, 1]"""
    h = v1347.health_score("p", None, None, None, None)
    assert 0.0 <= h.health_score <= 1.0


def test_v1347_health_components_count_five():
    """5 weighted components"""
    h = v1347.health_score("p", _mk_v1342(), _mk_v1343(), None, None)
    assert len(h.components) == 5
    names = {c.name for c in h.components}
    assert names == {"tier", "lint", "coverage", "drift", "plan"}


def test_v1347_weights_sum_to_one():
    """Sum of weights = 1.0 (invariance)"""
    s = sum(v1347.WEIGHTS.values())
    assert abs(s - 1.0) < 1e-9


def test_v1347_components_contribution_match():
    """Each component.contribution = score * weight"""
    h = v1347.health_score("p", _mk_v1342(), _mk_v1343(), None, None)
    for c in h.components:
        expected = c.score * c.weight
        assert abs(c.contribution - expected) < 1e-9, f"{c.name}: {c.contribution} != {expected}"


# ============================================================================
# Category 5: Exporters roundtrip + edge cases
# ============================================================================
def test_v1347_to_json_roundtrip():
    """to_json produces valid JSON with all fields"""
    h = v1347.health_score("p", _mk_v1342(), _mk_v1343(), None, None)
    j = v1347.to_json(h)
    d = json.loads(j)
    assert d["plugin_name"] == "p"
    assert "health_score" in d
    assert "tier" in d
    assert "components" in d
    assert "recommendations" in d
    assert "health_id" in d


def test_v1347_to_markdown_has_components():
    """to_markdown lists all 5 components"""
    h = v1347.health_score("p", _mk_v1342(), _mk_v1343(), None, None)
    md = v1347.to_markdown(h)
    for c in h.components:
        assert c.name in md, f"markdown missing {c.name}"


def test_v1347_to_human_has_lines():
    """to_human prints score + components"""
    h = v1347.health_score("p", _mk_v1342(), _mk_v1343(), None, None)
    text = v1347.to_human(h)
    assert "health_score" in text
    assert "tier:" in text
    for c in h.components:
        assert c.name in text


def test_v1347_ecosystem_rollup_empty_healthy():
    """Empty ecosystem → HEALTHY + 0 plugins"""
    r = v1347.ecosystem_rollup([])
    assert r.tier == "HEALTHY"
    assert r.plugin_count == 0
    assert r.worst_plugin is None
    assert r.best_plugin is None
    assert r.tier_breakdown == {"HEALTHY": 0, "DEGRADED": 0, "CRITICAL": 0}


def test_v1347_ecosystem_rollup_worst_of():
    """Ecosystem tier = worst-of all plugin tiers"""
    h_healthy = v1347.PluginHealth(
        health_id="hh", plugin_name="h_plugin", health_score=0.95,
        tier="HEALTHY", components=[], recommendations=[],
        generated_at=v1347._now_iso(),
    )
    h_crit = v1347.PluginHealth(
        health_id="cc", plugin_name="c_plugin", health_score=0.30,
        tier="CRITICAL", components=[], recommendations=[],
        generated_at=v1347._now_iso(),
    )
    r = v1347.ecosystem_rollup([h_healthy, h_crit])
    assert r.tier == "CRITICAL", "rollup tier must be worst-of"
    assert r.worst_plugin == "c_plugin"
    assert r.best_plugin == "h_plugin"
    assert r.tier_breakdown["HEALTHY"] == 1
    assert r.tier_breakdown["CRITICAL"] == 1
    assert abs(r.total_score - 0.625) < 1e-9  # (0.95 + 0.30) / 2


def test_v1347_recommend_degraded():
    """When tier/lint/coverage < 0.7, recommend produces 3+ items"""
    bad_v2 = _mk_v1342(high=0, medium=2, low=8)  # tier score = 0.1
    bad_v3 = _mk_v1343(pass_5_critical=0, coverage_score=0.5)
    h = v1347.health_score("bad_plugin", bad_v2, bad_v3, None, _mk_plan("refactor"))
    recs = v1347.recommend(h.components)
    assert len(recs) >= 3
    # At least one should mention tier/lint/coverage
    joined = " ".join(recs).lower()
    assert any(kw in joined for kw in ("tier", "lint", "coverage", "drift", "plan"))


def test_v1347_recommend_healthy_minimal():
    """When all components healthy, recommend produces <= 1 item"""
    good_v2 = _mk_v1342(high=10, medium=0, low=0)
    good_v3 = _mk_v1343(pass_5_critical=5, coverage_score=0.99)
    good_hist = _mk_history(passing_count=3, failing_count=0)
    h = v1347.health_score("good_plugin", good_v2, good_v3, good_hist, None)
    recs = v1347.recommend(h.components)
    assert len(recs) == 1, f"healthy should produce 1 rec, got {len(recs)}: {recs}"


def test_v1347_cli_self_test_via_subprocess():
    """CLI --self-test exits 0"""
    import subprocess

    res = subprocess.run(
        [sys.executable, str(_REPO_DIR / "v1347_vcp_plugin_health.py"), "--self-test"],
        capture_output=True, text=True, timeout=15,
    )
    assert res.returncode == 0, f"CLI self-test exit={res.returncode}\n{res.stderr}"
    assert "PASS" in res.stdout


def test_v1347_cli_demo_runs():
    """CLI demo (--plugin foo) runs without error"""
    import subprocess

    res = subprocess.run(
        [sys.executable, str(_REPO_DIR / "v1347_vcp_plugin_health.py"), "--plugin", "demo"],
        capture_output=True, text=True, timeout=15,
    )
    assert res.returncode == 0, f"CLI demo exit={res.returncode}\n{res.stderr}"
    assert "health_score" in res.stdout


# ============================================================================
# Category 6: Integration with V1346 plan + V1345 history
# ============================================================================
def test_v1347_with_active_refactor_plan_yields_low_plan_score():
    """V1346 plan with refactor → plan_score = 0.85"""
    plan = _mk_plan("refactor")
    v2_r = _mk_v1342()
    v3_r = _mk_v1343()
    h = v1347.health_score("p", v2_r, v3_r, None, plan)
    plan_c = next(c for c in h.components if c.name == "plan")
    assert plan_c.score == 0.85


def test_v1347_with_passing_history_yields_high_drift_score():
    """3 passing records + good everything → drift >= 0.95"""
    hist = _mk_history(passing_count=3, failing_count=0)
    v2_r = _mk_v1342(high=10, medium=0, low=0)
    v3_r = _mk_v1343(pass_5_critical=5, coverage_score=0.95)
    h = v1347.health_score("p", v2_r, v3_r, hist, None)
    drift_c = next(c for c in h.components if c.name == "drift")
    assert drift_c.score >= 0.95, f"drift should be high, got {drift_c.score}"


def test_v1347_ple_no_regression_with_other_chain():
    """V1347 imports don't disturb V1335-V1346 chain"""
    import v1335_vcp_cross_plugin_invariant_synthesis as v1335  # noqa: F401
    import v1336_vcp_plugin_conformance_linter as v1336  # noqa: F401
    import v1340_vcp_cookbook_validator as v1340  # noqa: F401
    import v1346_vcp_tier_aware_migration as v1346  # noqa: F401

    # Just ensure they all import cleanly + ID stable
    v2_r = _mk_v1342()
    v3_r = _mk_v1343()
    h = v1347.health_score("integration_check", v2_r, v3_r, None, None)
    assert h.health_id != "", "health_id should be populated"


def test_v1347_recommend_idempotent():
    """recommend() is pure — same components → same output"""
    v2_r = _mk_v1342()
    v3_r = _mk_v1343()
    h1 = v1347.health_score("p", v2_r, v3_r, None, None)
    h2 = v1347.health_score("p", v2_r, v3_r, None, None)
    r1 = v1347.recommend(h1.components)
    r2 = v1347.recommend(h2.components)
    assert r1 == r2


def test_v1347_score_drift_handles_single_record():
    """Single-record history doesn't trigger passing streak bonus"""
    one_record = _mk_history(passing_count=1, failing_count=0)
    s, d = v1347.score_drift(one_record)
    # 1 record < 3 → no streak bonus, but latest passed = clean
    assert s >= 0.95, f"single passing should be clean, got {s}"
    assert d["history_size"] == 1
    assert d["recent_pass_streak"] == 1


if __name__ == "__main__":
    import pytest

    sys.exit(pytest.main([__file__, "-v"]))
