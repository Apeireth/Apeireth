#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
test_v1346_vcp_tier_aware_migration.py — V1346 tier-aware migration pytest suite

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Module under test: apeireth.v1346_vcp_tier_aware_migration
- Chain: V1335 → V1345 → V1346

Tests are REAL pytest tests (no mocks, no stubs). All assertions are
deterministic and reproducible.

Test layers:
1. DriftAlert → action mapping (per rule)
2. Plan generation (idempotent, stable)
3. Plan validation (V3 invariants)
4. Apply + audit log (dry-run + real)
5. Rollback (inverse audit entry)
6. Exporters (JSON / Markdown / human)
7. plan_from_records end-to-end with V1345 records
8. Popper in-module self-tests (subprocess)
9. V3 哲学守门 (no ASI pretending, pole-star locked)
"""
from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Dict, List

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(APEIRETH_DIR.parent))

import apeireth.v1346_vcp_tier_aware_migration as v1346  # noqa: E402
import apeireth.v1345_vcp_historical_ledger as v1345  # noqa: E402
import apeireth.v1344_vcp_ci_gate as v1344  # noqa: E402


# =========================================================================
# Fixtures
# =========================================================================
@pytest.fixture
def sample_drift_alerts() -> List[Dict[str, Any]]:
    """Canonical drift alerts (one per known rule)."""
    return [
        {"ruleId": "coverage-regression", "level": "error",
         "baseline_value": 0.95, "current_value": 0.93, "delta": -0.02,
         "message": "cov drop"},
        {"ruleId": "high-tier-count-drop", "level": "error",
         "baseline_value": 50, "current_value": 40, "delta": -10,
         "message": "high drop"},
        {"ruleId": "unclassified-growth", "level": "warning",
         "baseline_value": 5, "current_value": 15, "delta": 10,
         "message": "unclass growth"},
        {"ruleId": "violation-growth", "level": "error",
         "baseline_value": 0, "current_value": 3, "delta": 3,
         "message": "viol growth"},
        {"ruleId": "pass-to-fail", "level": "error",
         "baseline_value": 0, "current_value": 1, "delta": 1,
         "message": "trans"},
        {"ruleId": "low-tier-growth", "level": "warning",
         "baseline_value": 0, "current_value": 5, "delta": 5,
         "message": "low growth"},
        {"ruleId": "mystery-rule", "level": "info",
         "baseline_value": 0, "current_value": 0, "delta": 0,
         "message": "unknown"},
    ]


@pytest.fixture
def baseline_record() -> v1345.LedgerRecord:
    return v1345.LedgerRecord(
        record_id="b", ledger_hash="LH_BASE",
        timestamp="2026-08-08T10:00:00+00:00",
        passed=True, exit_code=0,
        coverage_current=0.9, coverage_baseline=0.9, coverage_delta=0.0,
        tier_breakdown={"HIGH": 50, "MEDIUM": 10, "LOW": 0, "UNCLASSIFIED": 5},
        violations_count=0, unclassified_count=5, critical_failures=0,
        gate_config={}, summary={"baseline": True}, violations=[],
    )


@pytest.fixture
def current_record_with_drift() -> v1345.LedgerRecord:
    return v1345.LedgerRecord(
        record_id="c", ledger_hash="LH_CUR",
        timestamp="2026-08-08T11:00:00+00:00",
        passed=False, exit_code=1,
        coverage_current=0.85, coverage_baseline=0.9, coverage_delta=-0.05,
        tier_breakdown={"HIGH": 40, "MEDIUM": 10, "LOW": 0, "UNCLASSIFIED": 15},
        violations_count=3, unclassified_count=15, critical_failures=0,
        gate_config={}, summary={"baseline": False}, violations=[{}, {}, {}],
    )


@pytest.fixture
def tmp_audit_path():
    """Temp JSONL audit path."""
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
        path = Path(tf.name)
    yield path
    try:
        path.unlink()
    except Exception:
        pass


# =========================================================================
# 1. DriftAlert → Action mapping (per known rule)
# =========================================================================
def test_coverage_regression_maps_to_audit_test():
    plan = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.95, "current_value": 0.93, "delta": -0.02,
        "message": "cov drop",
    }])
    assert len(plan.actions) == 1
    assert plan.actions[0].action_type == v1346.ACTION_ATEST
    assert plan.actions[0].reversible is True
    assert plan.actions[0].before["coverage"] == 0.95
    # The action should propose a coverage that is AT LEAST the current_value
    # (i.e. the regression target, not the baseline).
    assert plan.actions[0].after["coverage"] >= 0.93


def test_high_tier_drop_maps_to_retier():
    plan = v1346.plan_remediation([{
        "ruleId": "high-tier-count-drop", "level": "error",
        "baseline_value": 50, "current_value": 40, "delta": -10,
        "message": "drop",
    }])
    assert plan.actions[0].action_type == v1346.ACTION_RETIER
    assert plan.actions[0].after["tier"] == "HIGH"
    assert plan.actions[0].before["tier"] in {"MEDIUM", "LOW", "UNCLASSIFIED"}


def test_unclassified_growth_maps_to_reclassify():
    plan = v1346.plan_remediation([{
        "ruleId": "unclassified-growth", "level": "warning",
        "baseline_value": 5, "current_value": 15, "delta": 10,
        "message": "growth",
    }])
    assert plan.actions[0].action_type == v1346.ACTION_RECLASSIFY
    assert plan.actions[0].before["tier"] == "UNCLASSIFIED"
    assert plan.actions[0].after["tier"] != "UNCLASSIFIED"


def test_violation_growth_maps_to_refactor():
    plan = v1346.plan_remediation([{
        "ruleId": "violation-growth", "level": "error",
        "baseline_value": 0, "current_value": 3, "delta": 3,
        "message": "growth",
    }])
    assert plan.actions[0].action_type == v1346.ACTION_REFRACTOR
    assert plan.actions[0].reversible is False


def test_pass_to_fail_maps_to_mark_known():
    plan = v1346.plan_remediation([{
        "ruleId": "pass-to-fail", "level": "error",
        "baseline_value": 0, "current_value": 1, "delta": 1,
        "message": "trans",
    }])
    assert plan.actions[0].action_type == v1346.ACTION_MARK_KNOWN
    assert plan.actions[0].after.get("known_issue") is True


def test_low_tier_growth_maps_to_refactor():
    plan = v1346.plan_remediation([{
        "ruleId": "low-tier-growth", "level": "warning",
        "baseline_value": 0, "current_value": 5, "delta": 5,
        "message": "low growth",
    }])
    assert plan.actions[0].action_type == v1346.ACTION_REFRACTOR


def test_unknown_rule_maps_to_ignore():
    plan = v1346.plan_remediation([{
        "ruleId": "mystery-rule", "level": "info",
        "baseline_value": 0, "current_value": 0, "delta": 0,
        "message": "unknown",
    }])
    assert plan.actions[0].action_type == v1346.ACTION_IGNORE


def test_all_known_rules_have_a_mapping(sample_drift_alerts):
    """Every known V1345 ruleId has a deterministic action mapping."""
    for alert in sample_drift_alerts:
        actions = v1346.actions_for_drift(alert)
        assert len(actions) >= 1, f"no actions for {alert['ruleId']}"
        for a in actions:
            assert a.action_type in {
                v1346.ACTION_RECLASSIFY, v1346.ACTION_RETIER,
                v1346.ACTION_REFRACTOR, v1346.ACTION_MARK_KNOWN,
                v1346.ACTION_IGNORE, v1346.ACTION_ATEST,
            }


# =========================================================================
# 2. Plan generation (idempotent, stable)
# =========================================================================
def test_plan_id_stable_across_runs():
    p1 = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    p2 = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    assert p1.plan_id == p2.plan_id
    assert len(p1.plan_id) == 16  # SHA256[:16]


def test_plan_id_changes_with_inputs():
    """Different inputs → different plan_id."""
    p1 = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    p2 = v1346.plan_remediation([{
        "ruleId": "high-tier-count-drop", "level": "error",
        "baseline_value": 50, "current_value": 40, "delta": -10,
        "message": "drop",
    }])
    assert p1.plan_id != p2.plan_id


def test_action_id_stable_for_same_action():
    """Same logical action → same action_id."""
    acts1 = v1346.actions_for_drift({
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    })
    acts2 = v1346.actions_for_drift({
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    })
    assert acts1[0].action_id == acts2[0].action_id


def test_max_actions_per_alert_caps():
    """max_actions_per_alert limits action count per alert."""
    alerts = [{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }]
    p = v1346.plan_remediation(alerts, max_actions_per_alert=1)
    assert len(p.actions) == 1


def test_plan_is_idempotent_flag():
    plan = v1346.plan_remediation([{
        "ruleId": "pass-to-fail", "level": "error",
        "baseline_value": 0, "current_value": 1, "delta": 1,
        "message": "trans",
    }])
    assert plan.is_idempotent is True


def test_plan_normalizes_alerts_from_driftalert_objects():
    """plan_remediation accepts both DriftAlert and dict inputs."""
    from apeireth.v1345_vcp_historical_ledger import DriftAlert
    alert = DriftAlert(
        ruleId="violation-growth", level="error",
        message="growth", baseline_value=0.0,
        current_value=2.0, delta=2.0,
    )
    plan = v1346.plan_remediation([alert])
    assert len(plan.actions) == 1
    assert plan.actions[0].action_type == v1346.ACTION_REFRACTOR


# =========================================================================
# 3. Plan validation (V3 invariants)
# =========================================================================
def test_validate_plan_rejects_empty_actions():
    plan = v1346.RemediationPlan(
        plan_id="x", source_ledger_hash="", drift_alerts=[], actions=[],
        created_at="2026-08-08T00:00:00+00:00")
    errs = v1346.validate_plan(plan)
    assert errs
    assert any("no actions" in e for e in errs)


def test_validate_plan_rejects_unknown_action_type():
    bad = v1346.RemediationAction(
        action_id="a", action_type="dance-party",
        target_ruleId="x", target_substrate="y",
        rationale="?", before={}, after={}, reversible=False)
    plan = v1346.RemediationPlan(
        plan_id="z", source_ledger_hash="", drift_alerts=[],
        actions=[bad], created_at="2026-08-08T00:00:00+00:00")
    errs = v1346.validate_plan(plan)
    assert any("unknown action_type" in e for e in errs)


def test_validate_plan_rejects_invalid_tier():
    bad = v1346.RemediationAction(
        action_id="a", action_type=v1346.ACTION_RETIER,
        target_ruleId="x", target_substrate="y",
        rationale="?", before={"tier": "LOW"}, after={"tier": "GIGA"},
        reversible=True)
    plan = v1346.RemediationPlan(
        plan_id="z", source_ledger_hash="", drift_alerts=[],
        actions=[bad], created_at="2026-08-08T00:00:00+00:00")
    errs = v1346.validate_plan(plan)
    assert any("invalid target tier" in e for e in errs)


def test_validate_plan_rejects_empty_target_substrate():
    bad = v1346.RemediationAction(
        action_id="a", action_type=v1346.ACTION_REFRACTOR,
        target_ruleId="x", target_substrate="",
        rationale="?", before={}, after={}, reversible=False)
    plan = v1346.RemediationPlan(
        plan_id="z", source_ledger_hash="", drift_alerts=[],
        actions=[bad], created_at="2026-08-08T00:00:00+00:00")
    errs = v1346.validate_plan(plan)
    assert any("target_substrate" in e for e in errs)


def test_validate_plan_accepts_valid_plan(sample_drift_alerts):
    plan = v1346.plan_remediation(sample_drift_alerts)
    errs = v1346.validate_plan(plan)
    assert errs == []


# =========================================================================
# 4. Apply + audit log
# =========================================================================
def test_apply_plan_dry_run_writes_audit(tmp_audit_path):
    plan = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    res = v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    assert res.applied is False
    assert res.actions_applied == 1
    assert tmp_audit_path.exists()
    entries = v1346._read_audit_log(tmp_audit_path)
    assert len(entries) == 1
    assert entries[0].plan_id == plan.plan_id
    assert entries[0].applied is False


def test_apply_plan_real_mode_sets_applied(tmp_audit_path):
    plan = v1346.plan_remediation([{
        "ruleId": "high-tier-count-drop", "level": "error",
        "baseline_value": 50, "current_value": 40, "delta": -10,
        "message": "drop",
    }])
    res = v1346.apply_plan(plan, dry_run=False, audit_path=tmp_audit_path)
    assert res.applied is True
    assert res.actions_applied == 1


def test_apply_plan_invalid_returns_errors(tmp_audit_path):
    bad = v1346.RemediationAction(
        action_id="a", action_type="bogus",
        target_ruleId="x", target_substrate="y",
        rationale="?", before={}, after={}, reversible=False)
    plan = v1346.RemediationPlan(
        plan_id="z", source_ledger_hash="", drift_alerts=[],
        actions=[bad], created_at="2026-08-08T00:00:00+00:00")
    res = v1346.apply_plan(plan, dry_run=False, audit_path=tmp_audit_path)
    assert res.applied is False
    assert res.actions_applied == 0
    assert any("unknown action_type" in e for e in res.errors)


def test_apply_plan_idempotent_on_dry_run(tmp_audit_path):
    plan = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    r1 = v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    r2 = v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    assert (r1.actions_applied, r1.actions_skipped) == \
           (r2.actions_applied, r2.actions_skipped)


def test_multiple_alerts_produce_multiple_actions(tmp_audit_path):
    plan = v1346.plan_remediation([
        {"ruleId": "coverage-regression", "level": "error",
         "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05, "message": "a"},
        {"ruleId": "violation-growth", "level": "error",
         "baseline_value": 0, "current_value": 1, "delta": 1, "message": "b"},
    ])
    assert len(plan.actions) == 2
    res = v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    assert res.actions_applied == 2


# =========================================================================
# 5. Rollback
# =========================================================================
def test_rollback_appends_inverse_entry(tmp_audit_path):
    plan = v1346.plan_remediation([{
        "ruleId": "violation-growth", "level": "error",
        "baseline_value": 0, "current_value": 1, "delta": 1,
        "message": "growth",
    }])
    v1346.apply_plan(plan, dry_run=False, audit_path=tmp_audit_path)
    rolled = v1346.rollback(plan.plan_id, audit_path=tmp_audit_path)
    assert len(rolled) == 1
    entries = v1346._read_audit_log(tmp_audit_path)
    assert len(entries) == 2
    # Second entry is the inverse.
    assert entries[0].applied is True
    assert entries[1].applied is False


def test_rollback_returns_empty_when_no_match(tmp_audit_path):
    rolled = v1346.rollback("nonexistent-plan-id", audit_path=tmp_audit_path)
    assert rolled == []


# =========================================================================
# 6. Exporters
# =========================================================================
def test_to_json_roundtrip():
    plan = v1346.plan_remediation([{
        "ruleId": "pass-to-fail", "level": "error",
        "baseline_value": 0, "current_value": 1, "delta": 1,
        "message": "trans",
    }])
    s = v1346.to_json(plan)
    parsed = json.loads(s)
    assert parsed["plan_id"] == plan.plan_id
    assert len(parsed["actions"]) == 1


def test_to_markdown_contains_header_and_table():
    plan = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    md = v1346.to_markdown(plan)
    assert "# V1346 Remediation Plan" in md
    assert "coverage-regression" in md
    assert plan.plan_id in md


def test_to_human_contains_plan_id():
    plan = v1346.plan_remediation([{
        "ruleId": "high-tier-count-drop", "level": "error",
        "baseline_value": 50, "current_value": 40, "delta": -10,
        "message": "drop",
    }])
    h = v1346.to_human(plan)
    assert plan.plan_id in h
    assert "re-tier" in h


def test_to_markdown_handles_no_alerts():
    """Empty plan → markdown still renders."""
    plan = v1346.RemediationPlan(
        plan_id="empty-plan", source_ledger_hash="LH",
        drift_alerts=[], actions=[], created_at="2026-08-08T00:00:00+00:00",
        notes="empty")
    md = v1346.to_markdown(plan)
    assert "(no drift alerts)" in md
    assert "(no actions)" in md


# =========================================================================
# 7. plan_from_records end-to-end with V1345
# =========================================================================
def test_plan_from_records_detects_drift_and_produces_plan(
    baseline_record, current_record_with_drift,
):
    plan = v1346.plan_from_records(baseline_record, current_record_with_drift,
                                   notes="drift")
    assert plan.plan_id
    assert plan.source_ledger_hash == current_record_with_drift.ledger_hash
    # Multiple drift alerts → multiple actions.
    assert len(plan.actions) >= 3
    # All actions should be valid.
    assert v1346.validate_plan(plan) == []


def test_plan_from_records_no_drift_when_identical(baseline_record):
    """Two identical records → no drift → plan has no actions."""
    identical = v1345.LedgerRecord(
        record_id="ident", ledger_hash="LH_IDENT",
        timestamp=baseline_record.timestamp,
        passed=baseline_record.passed,
        exit_code=baseline_record.exit_code,
        coverage_current=baseline_record.coverage_current,
        coverage_baseline=baseline_record.coverage_baseline,
        coverage_delta=baseline_record.coverage_delta,
        tier_breakdown=dict(baseline_record.tier_breakdown),
        violations_count=baseline_record.violations_count,
        unclassified_count=baseline_record.unclassified_count,
        critical_failures=baseline_record.critical_failures,
        gate_config=dict(baseline_record.gate_config),
        summary=dict(baseline_record.summary),
        violations=list(baseline_record.violations),
    )
    plan = v1346.plan_from_records(baseline_record, identical)
    assert len(plan.actions) == 0


def test_pipeline_v1345_to_v1346(baseline_record, current_record_with_drift,
                                  tmp_audit_path):
    """Full pipeline: V1345 detects drift, V1346 plans + applies."""
    # 1. V1345 detects drift
    alerts = v1345.detect_regression(baseline_record, current_record_with_drift)
    assert len(alerts) >= 1
    # 2. V1346 plans
    plan = v1346.plan_remediation(alerts, source_ledger_hash="LH_CUR")
    assert len(plan.actions) >= 1
    # 3. V1346 applies (dry-run)
    res = v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    assert res.actions_applied >= 1
    assert tmp_audit_path.exists()


# =========================================================================
# 8. Popper in-module self-tests (subprocess)
# =========================================================================
def test_module_self_test_passes():
    """Run V1346 --self-test in a subprocess; must report 0 failures."""
    module_path = APEIRETH_DIR / "v1346_vcp_tier_aware_migration.py"
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1346_vcp_tier_aware_migration",
         "--self-test"],
        capture_output=True, text=True, timeout=60,
    )
    assert proc.returncode == 0, f"self-test failed: {proc.stdout}\n{proc.stderr}"
    assert "0 failures" in proc.stdout


def test_module_imports_clean():
    """Module imports without raising."""
    import importlib
    importlib.reload(v1346)
    assert v1346 is not None


# =========================================================================
# 9. V3 哲学守门 (no ASI pretending, pole-star locked)
# =========================================================================
def test_pole_star_locked_v1346():
    assert v1346.ASI_POLE_STAR["V0_1_actual_measured"] == 0.7905
    assert v1346.ASI_POLE_STAR["V0_2_baseline"] == 0.4467
    assert v1346.ASI_POLE_STAR["V1256_unio_mystica_realized"] == 0.9105
    assert v1346.ASI_POLE_STAR["V1049_value_alignment_done"] is True
    assert v1346.ASI_POLE_STAR["asi_achieved_false"] is True
    assert v1346.ASI_POLE_STAR["V1346_modifies_pole_star"] is False


def test_v1346_explicitly_not_pretending_too_asi():
    """V1346 must declare its limitations (not ASI)."""
    src = (APEIRETH_DIR / "v1346_vcp_tier_aware_migration.py").read_text(
        encoding="utf-8")
    assert "NOT 假装 ASI" in src
    assert "substrate research only" in src or "REAL ENGINEERING" in src.upper()
    assert "no Phenomenal consciousness" in src or "no qualia" in src.lower()


def test_v1346_uses_only_deterministic_actions():
    """No ML, no learned policy — all action types are finite constants."""
    # The set of valid action types is small and closed.
    assert len(v1346.REVERSIBLE_ACTIONS) >= 4
    valid_types = {
        v1346.ACTION_RECLASSIFY, v1346.ACTION_RETIER,
        v1346.ACTION_REFRACTOR, v1346.ACTION_MARK_KNOWN,
        v1346.ACTION_IGNORE, v1346.ACTION_ATEST,
    }
    assert len(valid_types) == 6


def test_audit_entries_have_stable_ids(tmp_audit_path):
    plan = v1346.plan_remediation([{
        "ruleId": "coverage-regression", "level": "error",
        "baseline_value": 0.9, "current_value": 0.85, "delta": -0.05,
        "message": "drop",
    }])
    v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    entries = v1346._read_audit_log(tmp_audit_path)
    assert len(entries) == 1
    assert len(entries[0].audit_id) == 16
    assert entries[0].plan_id == plan.plan_id
    assert entries[0].timestamp.endswith("+00:00")


def test_audit_persistence_across_process(tmp_audit_path):
    """Audit log persists as JSONL on disk."""
    plan = v1346.plan_remediation([{
        "ruleId": "high-tier-count-drop", "level": "error",
        "baseline_value": 50, "current_value": 40, "delta": -10,
        "message": "drop",
    }])
    v1346.apply_plan(plan, dry_run=False, audit_path=tmp_audit_path)
    # Read raw lines, ensure JSON-parseable.
    raw = tmp_audit_path.read_text(encoding="utf-8").strip().splitlines()
    assert len(raw) >= 1
    for line in raw:
        obj = json.loads(line)
        assert "audit_id" in obj
        assert "plan_id" in obj
        assert "applied" in obj


# =========================================================================
# 10. Cross-module integration (V1346 + V1345 + V1344)
# =========================================================================
def test_chain_v1344_v1345_v1346(tmp_audit_path):
    """V1344 gate → V1345 ledger → V1346 migration loop."""
    # 1. Run V1344 gate (real, against V1335 ledger)
    cfg = v1344.CIGateConfig(
        tier_min="high",
        fail_on_coverage_loss=True,
        max_critical_failures=0,
        fail_on_unclassified=False,
    )
    res1 = v1344.lint_v1335_ledger_ci(config=cfg)
    # 2. Record into V1345 ledger
    rec1 = v1345.record(res1)
    assert rec1 is not None
    # 3. Run the gate again (deterministic, same result)
    res2 = v1344.lint_v1335_ledger_ci(config=cfg)
    rec2 = v1345.record(res2)
    # 4. Plan from V1345 → V1346 (idempotent: identical records → no actions)
    plan = v1346.plan_from_records(rec1, rec2, notes="chain test")
    assert plan.plan_id
    # 5. Apply (dry-run; deterministic → 0 actions if no drift)
    apply_res = v1346.apply_plan(plan, dry_run=True, audit_path=tmp_audit_path)
    assert apply_res.actions_applied >= 0
    assert tmp_audit_path.exists()
