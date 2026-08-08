#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tests/test_v1350_anomaly_lifecycle.py — pytest tests for V1350 state machine.

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1349 LLM operator brief (58ea9d27, 23:56)
- Chain: V1335 → ... → V1349 → **V1350**

Tests cover:
- State + action constants
- Transition table integrity
- build_initial_record (deterministic lifecycle_id)
- apply_transition (valid + invalid + missing evidence)
- Reopen atomic (REOPENED → TRIAGED in one call)
- LifecycleStore (idempotency + audit JSONL)
- ecosystem_rollup (worst-of state, breakdown)
- v1350_subscore (in-range, components present)
- v1350_asi_lift (capped)
- open_from_anomaly (V1348 bridge)
- Self-test integration (V1350 in-process)
- Interoperability with V1348/V1349
- Chain regression (V1335 → V1349 not broken)
"""
from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import pytest

# Make apeireth importable
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROMETHEAN_ROOT / "apeireth"))

import v1348_vcp_anomaly_detector as v1348  # noqa: E402
import v1350_vcp_anomaly_lifecycle as v1350  # noqa: E402


# ============================================================================
# Fixtures
# ============================================================================
@pytest.fixture
def sample_anomaly_high():
    """Sample HIGH-severity plugin anomaly from V1348."""
    return v1348.PluginAnomaly(
        plugin="plugin.alpha",
        plugin_severity=v1348.SEVERITY_HIGH,
        plugin_severity_rank=3,
        channels=[
            v1348.ChannelSignal(
                channel=v1348.CHANNEL_HEALTH_DROP,
                signal_score=1.0,
                severity=v1348.SEVERITY_HIGH,
                evidence={"delta": 0.5},
                recommendation="investigate now",
            ),
            v1348.ChannelSignal(
                channel=v1348.CHANNEL_LINT_REGRESSION,
                signal_score=0.3,
                severity=v1348.SEVERITY_LOW,
                evidence={"delta": 1},
                recommendation="re-lint",
            ),
        ],
        anomaly_id="anom-test-alpha-001",
    )


@pytest.fixture
def sample_anomaly_medium():
    """Sample MEDIUM-severity plugin anomaly."""
    return v1348.PluginAnomaly(
        plugin="plugin.beta",
        plugin_severity=v1348.SEVERITY_MEDIUM,
        plugin_severity_rank=2,
        channels=[
            v1348.ChannelSignal(
                channel=v1348.CHANNEL_TIER_JUMP,
                signal_score=0.7,
                severity=v1348.SEVERITY_MEDIUM,
                evidence={"delta": 2},
                recommendation="re-tier",
            ),
        ],
        anomaly_id="anom-test-beta-001",
    )


@pytest.fixture
def sample_anomaly_low():
    """Sample LOW-severity plugin anomaly."""
    return v1348.PluginAnomaly(
        plugin="plugin.gamma",
        plugin_severity=v1348.SEVERITY_LOW,
        plugin_severity_rank=1,
        channels=[
            v1348.ChannelSignal(
                channel=v1348.CHANNEL_DRIFT_SPIKE,
                signal_score=0.4,
                severity=v1348.SEVERITY_LOW,
                evidence={"drift": 0.4},
                recommendation="monitor",
            ),
        ],
        anomaly_id="anom-test-gamma-001",
    )


@pytest.fixture
def fresh_store():
    """In-memory LifecycleStore, no audit."""
    return v1350.LifecycleStore()


@pytest.fixture
def audit_store():
    """LifecycleStore with temp audit JSONL."""
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "audit.jsonl"
        store = v1350.LifecycleStore(audit_path=p)
        yield store, p


# ============================================================================
# Test 1: Constants
# ============================================================================
class TestConstants:
    def test_states_count(self):
        assert len(v1350.ALL_STATES) == 7

    def test_actions_count(self):
        assert len(v1350.ALL_ACTIONS) == 6

    def test_state_rank_complete(self):
        for s in v1350.ALL_STATES:
            assert s in v1350.STATE_RANK

    def test_state_rank_unique(self):
        ranks = list(v1350.STATE_RANK.values())
        assert len(ranks) == len(set(ranks))

    def test_guards_present(self):
        assert len(v1350.V1350_GUARDS) >= 5
        assert v1350.GUARD_NOT_MACHINE_IS_CONSCIOUS in v1350.V1350_GUARDS
        assert v1350.GUARD_NOT_WORKFLOW_IS_POLICY in v1350.V1350_GUARDS
        assert v1350.GUARD_NOT_LIFECYCLE_IS_ORACLE in v1350.V1350_GUARDS
        assert v1350.GUARD_NOT_PLUGIN_IS_PHENOMENAL in v1350.V1350_GUARDS
        assert v1350.GUARD_NOT_SUBSCORE_IS_ASI in v1350.V1350_GUARDS

    def test_subweights_sum_to_one(self):
        total = sum(v1350.V1350_SUBWEIGHTS.values())
        assert abs(total - 1.0) < 1e-9, f"subweights sum to {total}"

    def test_asi_cap_value(self):
        assert v1350.V1350_ASI_CAP == 0.015


# ============================================================================
# Test 2: Transition table
# ============================================================================
class TestTransitionTable:
    def test_transition_count(self):
        assert len(v1350.TRANSITIONS) == 9

    def test_all_transitions_have_to_state(self):
        for (action, from_state), meta in v1350.TRANSITIONS.items():
            assert "to_state" in meta
            assert meta["to_state"] in v1350.ALL_STATES

    def test_all_transitions_have_required_evidence_keys(self):
        for (action, from_state), meta in v1350.TRANSITIONS.items():
            assert "required_evidence_keys" in meta
            assert isinstance(meta["required_evidence_keys"], tuple)
            assert "reason" in meta["required_evidence_keys"]

    def test_transition_lookup_valid(self):
        meta = v1350.transition_lookup(v1350.ACTION_ACKNOWLEDGE, v1350.STATE_OPEN)
        assert meta is not None
        assert meta["to_state"] == v1350.STATE_TRIAGED

    def test_transition_lookup_invalid(self):
        meta = v1350.transition_lookup(v1350.ACTION_CLOSE, v1350.STATE_OPEN)
        assert meta is None

    def test_list_transitions_complete(self):
        rows = v1350.list_transitions()
        assert len(rows) == 9


# ============================================================================
# Test 3: build_initial_record
# ============================================================================
class TestBuildInitialRecord:
    def test_initial_state_open(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        assert rec.current_state == v1350.STATE_OPEN
        assert rec.state_rank == v1350.STATE_RANK[v1350.STATE_OPEN]

    def test_initial_has_one_event(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        assert len(rec.events) == 1
        assert rec.events[0].action == "open"
        assert rec.events[0].state_before == "<none>"
        assert rec.events[0].state_after == v1350.STATE_OPEN

    def test_lifecycle_id_deterministic(self):
        r1 = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        assert r1.lifecycle_id == r2.lifecycle_id

    def test_lifecycle_id_length(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        assert len(rec.lifecycle_id) == 16
        assert len(rec.events[0].event_id) == 16

    def test_different_plugin_different_id(self):
        r1 = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.build_initial_record("p2", "a1", actor="alice", reason="init")
        assert r1.lifecycle_id != r2.lifecycle_id

    def test_different_anomaly_different_id(self):
        r1 = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.build_initial_record("p1", "a2", actor="alice", reason="init")
        assert r1.lifecycle_id != r2.lifecycle_id

    def test_evidence_recorded(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init",
                                         evidence={"severity": "HIGH", "foo": "bar"})
        assert rec.events[0].evidence == {"severity": "HIGH", "foo": "bar"}

    def test_none_evidence_default(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init",
                                         evidence=None)
        assert rec.events[0].evidence == {}

    def test_timestamps_present(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        assert "T" in rec.created_at  # ISO format
        assert rec.created_at == rec.updated_at


# ============================================================================
# Test 4: apply_transition (valid + invalid)
# ============================================================================
class TestApplyTransition:
    def test_acknowledge_open_to_triaged(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        assert r2.current_state == v1350.STATE_TRIAGED
        assert len(r2.events) == 2
        assert r2.events[-1].state_before == v1350.STATE_OPEN
        assert r2.events[-1].state_after == v1350.STATE_TRIAGED

    def test_escalate_triaged_to_escalated_high(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_ESCALATE,
                                    actor="bob", reason="HIGH severity",
                                    evidence={"reason": "high", "severity": v1348.SEVERITY_HIGH})
        assert r3.current_state == v1350.STATE_ESCALATED

    def test_escalate_nonhigh_rejected(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        with pytest.raises(ValueError, match="HIGH"):
            v1350.apply_transition(r2, v1350.ACTION_ESCALATE,
                                   actor="bob", reason="x",
                                   evidence={"reason": "x", "severity": v1348.SEVERITY_MEDIUM})

    def test_escalate_low_rejected(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        with pytest.raises(ValueError):
            v1350.apply_transition(r2, v1350.ACTION_ESCALATE,
                                   actor="bob", reason="x",
                                   evidence={"reason": "x", "severity": v1348.SEVERITY_LOW})

    def test_mitigate_triaged(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_MITIGATE,
                                    actor="alice", reason="fix",
                                    evidence={"reason": "fix", "action_kind": "patch"})
        assert r3.current_state == v1350.STATE_MITIGATED

    def test_mitigate_escalated(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_ESCALATE,
                                    actor="bob", reason="HIGH",
                                    evidence={"reason": "high", "severity": v1348.SEVERITY_HIGH})
        r4 = v1350.apply_transition(r3, v1350.ACTION_MITIGATE,
                                    actor="bob", reason="fix",
                                    evidence={"reason": "fix", "action_kind": "rollback"})
        assert r4.current_state == v1350.STATE_MITIGATED

    def test_resolve_requires_gone_true(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        with pytest.raises(ValueError, match="anomaly_gone"):
            v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                   actor="alice", reason="x",
                                   evidence={"reason": "x", "anomaly_gone": False})

    def test_resolve_triaged_with_gone(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="gone",
                                    evidence={"reason": "gone", "anomaly_gone": True})
        assert r3.current_state == v1350.STATE_RESOLVED

    def test_resolve_mitigated(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_MITIGATE,
                                    actor="alice", reason="fix",
                                    evidence={"reason": "fix", "action_kind": "patch"})
        r4 = v1350.apply_transition(r3, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="fixed",
                                    evidence={"reason": "fixed", "anomaly_gone": True})
        assert r4.current_state == v1350.STATE_RESOLVED

    def test_close_resolved(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="gone",
                                    evidence={"reason": "gone", "anomaly_gone": True})
        r4 = v1350.apply_transition(r3, v1350.ACTION_CLOSE,
                                    actor="alice", reason="archive",
                                    evidence={"reason": "archive"})
        assert r4.current_state == v1350.STATE_CLOSED

    def test_invalid_transition_rejected(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        with pytest.raises(ValueError, match="invalid transition"):
            v1350.apply_transition(rec, v1350.ACTION_CLOSE,
                                   actor="alice", reason="x",
                                   evidence={"reason": "x"})

    def test_missing_evidence_rejected(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        with pytest.raises(ValueError, match="missing required evidence"):
            v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                   actor="alice", reason="x", evidence={})

    def test_immutability_original_record_unchanged(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        original_len = len(rec.events)
        original_state = rec.current_state
        v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                               actor="alice", reason="looking",
                               evidence={"reason": "looking"})
        # Original unchanged
        assert len(rec.events) == original_len
        assert rec.current_state == original_state


# ============================================================================
# Test 5: Reopen atomic
# ============================================================================
class TestReopen:
    def test_reopen_closed_to_triaged(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="gone",
                                    evidence={"reason": "gone", "anomaly_gone": True})
        r4 = v1350.apply_transition(r3, v1350.ACTION_CLOSE,
                                    actor="alice", reason="archive",
                                    evidence={"reason": "archive"})
        r5 = v1350.apply_transition(r4, v1350.ACTION_REOPEN,
                                    actor="bob", reason="new anomaly",
                                    evidence={"reason": "new", "new_anomaly_id": "a2"})
        assert r5.current_state == v1350.REOPEN_NEXT_STATE

    def test_reopen_appends_two_events(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="gone",
                                    evidence={"reason": "gone", "anomaly_gone": True})
        r4 = v1350.apply_transition(r3, v1350.ACTION_CLOSE,
                                    actor="alice", reason="archive",
                                    evidence={"reason": "archive"})
        r5 = v1350.apply_transition(r4, v1350.ACTION_REOPEN,
                                    actor="bob", reason="new anomaly",
                                    evidence={"reason": "new", "new_anomaly_id": "a2"})
        assert len(r5.events) == len(r4.events) + 2

    def test_reopen_includes_reopened_state_in_history(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="gone",
                                    evidence={"reason": "gone", "anomaly_gone": True})
        r4 = v1350.apply_transition(r3, v1350.ACTION_CLOSE,
                                    actor="alice", reason="archive",
                                    evidence={"reason": "archive"})
        r5 = v1350.apply_transition(r4, v1350.ACTION_REOPEN,
                                    actor="bob", reason="new anomaly",
                                    evidence={"reason": "new", "new_anomaly_id": "a2"})
        # Second-to-last event should be REOPENED
        assert r5.events[-2].state_after == v1350.STATE_REOPENED
        assert r5.events[-2].action == v1350.ACTION_REOPEN
        # Last event should be the auto-triage
        assert r5.events[-1].action == "auto_reopen_triage"
        assert r5.events[-1].state_after == v1350.STATE_TRIAGED

    def test_reopen_requires_new_anomaly_id(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        r2 = v1350.apply_transition(rec, v1350.ACTION_ACKNOWLEDGE,
                                    actor="alice", reason="looking",
                                    evidence={"reason": "looking"})
        r3 = v1350.apply_transition(r2, v1350.ACTION_RESOLVE,
                                    actor="alice", reason="gone",
                                    evidence={"reason": "gone", "anomaly_gone": True})
        r4 = v1350.apply_transition(r3, v1350.ACTION_CLOSE,
                                    actor="alice", reason="archive",
                                    evidence={"reason": "archive"})
        with pytest.raises(ValueError, match="missing required evidence"):
            v1350.apply_transition(r4, v1350.ACTION_REOPEN,
                                   actor="bob", reason="new",
                                   evidence={"reason": "new"})


# ============================================================================
# Test 6: LifecycleStore
# ============================================================================
class TestLifecycleStore:
    def test_open_new(self, fresh_store):
        rec = fresh_store.open_anomaly("p1", "a1", actor="alice", reason="init")
        assert rec.current_state == v1350.STATE_OPEN
        assert fresh_store.get("p1", "a1") == rec

    def test_open_idempotent(self, fresh_store):
        r1 = fresh_store.open_anomaly("p1", "a1", actor="alice", reason="init")
        r2 = fresh_store.open_anomaly("p1", "a1", actor="alice", reason="init")
        assert r1.lifecycle_id == r2.lifecycle_id
        # Events not duplicated
        assert len(r2.events) == 1

    def test_apply_updates_record(self, fresh_store):
        fresh_store.open_anomaly("p1", "a1", actor="alice", reason="init")
        r2 = fresh_store.apply("p1", "a1", v1350.ACTION_ACKNOWLEDGE,
                               actor="alice", reason="looking",
                               evidence={"reason": "looking"})
        assert r2.current_state == v1350.STATE_TRIAGED
        # Same id, but new events
        original = fresh_store.get("p1", "a1")
        assert original.current_state == v1350.STATE_TRIAGED
        assert len(original.events) == 2

    def test_apply_unknown_record_rejected(self, fresh_store):
        with pytest.raises(KeyError):
            fresh_store.apply("p-unknown", "a-unknown", v1350.ACTION_ACKNOWLEDGE,
                              actor="alice", reason="x",
                              evidence={"reason": "x"})

    def test_list_plugins(self, fresh_store, sample_anomaly_high, sample_anomaly_medium):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        v1350.open_from_anomaly(fresh_store, sample_anomaly_medium, actor="alice")
        plugins = fresh_store.list_plugins()
        assert sorted(plugins) == ["plugin.alpha", "plugin.beta"]

    def test_records_for_plugin(self, fresh_store, sample_anomaly_high):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        recs = fresh_store.records_for_plugin("plugin.alpha")
        assert len(recs) == 1
        assert recs[0].plugin == "plugin.alpha"

    def test_audit_jsonl_written(self, audit_store):
        store, path = audit_store
        store.open_anomaly("p1", "a1", actor="alice", reason="init")
        store.apply("p1", "a1", v1350.ACTION_ACKNOWLEDGE, actor="alice",
                    reason="looking", evidence={"reason": "looking"})
        lines = path.read_text(encoding="utf-8").strip().split("\n")
        assert len(lines) >= 2
        first = json.loads(lines[0])
        assert "event_id" in first
        assert first["plugin"] == "p1"
        assert first["anomaly_id"] == "a1"

    def test_audit_event_ids_distinct(self, audit_store):
        store, path = audit_store
        store.open_anomaly("p1", "a1", actor="alice", reason="init")
        store.apply("p1", "a1", v1350.ACTION_ACKNOWLEDGE, actor="alice",
                    reason="looking", evidence={"reason": "looking"})
        store.apply("p1", "a1", v1350.ACTION_RESOLVE, actor="alice",
                    reason="gone", evidence={"reason": "gone", "anomaly_gone": True})
        lines = path.read_text(encoding="utf-8").strip().split("\n")
        ids = [json.loads(l)["event_id"] for l in lines]
        assert len(set(ids)) == len(ids)  # all distinct


# ============================================================================
# Test 7: ecosystem_rollup
# ============================================================================
class TestEcosystemRollup:
    def test_empty_rollup(self):
        rollup = v1350.ecosystem_rollup([])
        assert rollup.total_plugins == 0
        assert rollup.ecosystem_state == v1350.STATE_CLOSED
        assert rollup.total_events == 0

    def test_single_plugin_rollup(self, fresh_store, sample_anomaly_high):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        rollup = v1350.ecosystem_rollup(fresh_store.records.values())
        assert rollup.total_plugins == 1
        assert rollup.ecosystem_state == v1350.STATE_OPEN
        assert rollup.state_breakdown[v1350.STATE_OPEN] == 1

    def test_multi_plugin_rollup(self, fresh_store,
                                  sample_anomaly_high, sample_anomaly_medium, sample_anomaly_low):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        v1350.open_from_anomaly(fresh_store, sample_anomaly_medium, actor="alice")
        v1350.open_from_anomaly(fresh_store, sample_anomaly_low, actor="alice")
        rollup = v1350.ecosystem_rollup(fresh_store.records.values())
        assert rollup.total_plugins == 3
        assert rollup.ecosystem_state == v1350.STATE_OPEN  # all OPEN (worst-of)
        assert rollup.state_breakdown[v1350.STATE_OPEN] == 3

    def test_worst_of_state_mixed(self, fresh_store,
                                   sample_anomaly_high, sample_anomaly_low):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        # Move low to RESOLVED
        v1350.open_from_anomaly(fresh_store, sample_anomaly_low, actor="alice")
        fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                          v1350.ACTION_ACKNOWLEDGE, actor="alice",
                          reason="looking", evidence={"reason": "looking"})
        fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                          v1350.ACTION_RESOLVE, actor="alice",
                          reason="gone", evidence={"reason": "gone", "anomaly_gone": True})
        rollup = v1350.ecosystem_rollup(fresh_store.records.values())
        # Worst-of = OPEN (still open for plugin.alpha)
        assert rollup.ecosystem_state == v1350.STATE_OPEN
        # Breakdown correct
        assert rollup.state_breakdown[v1350.STATE_OPEN] == 1
        assert rollup.state_breakdown[v1350.STATE_RESOLVED] == 1

    def test_rollup_transitions_used(self, fresh_store, sample_anomaly_high):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        fresh_store.apply("plugin.alpha", sample_anomaly_high.anomaly_id,
                          v1350.ACTION_ACKNOWLEDGE, actor="alice",
                          reason="looking", evidence={"reason": "looking"})
        rollup = v1350.ecosystem_rollup(fresh_store.records.values())
        # "open" + "acknowledge"
        assert "open" in rollup.transitions_used
        assert "acknowledge" in rollup.transitions_used

    def test_rollup_report_id_deterministic(self, fresh_store, sample_anomaly_high):
        v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        r1 = v1350.ecosystem_rollup(fresh_store.records.values())
        r2 = v1350.ecosystem_rollup(fresh_store.records.values())
        # report_id doesn't include generated_at
        assert r1.report_id == r2.report_id


# ============================================================================
# Test 8: v1350_subscore
# ============================================================================
class TestSubscore:
    def test_subscore_in_range(self):
        sub, _ = v1350.v1350_subscore(
            record_count=1, event_count=1, reopen_count=0,
            audit_path=None, transitions_used_count=1,
            has_v1348_bridge=True, has_v1349_audit_compat=True,
            guards_present=True,
        )
        assert 0.0 <= sub <= 1.0

    def test_subscore_components_present(self):
        sub, details = v1350.v1350_subscore(
            record_count=1, event_count=1, reopen_count=0,
            audit_path=None, transitions_used_count=1,
            has_v1348_bridge=True, has_v1349_audit_compat=True,
            guards_present=True,
        )
        assert "components" in details
        assert "totals" in details
        assert "inputs" in details
        # All 9 components
        expected = {"states_explicit", "transitions_explicit", "event_auditability",
                    "evidence_validation", "reopen_correctness", "rollup_aggregation",
                    "interoperability_v1348", "interoperability_v1349", "philosophy_guards"}
        assert set(details["components"].keys()) == expected

    def test_subscore_no_records_lower(self):
        sub_empty, _ = v1350.v1350_subscore(
            record_count=0, event_count=0, reopen_count=0,
            audit_path=None, transitions_used_count=0,
            has_v1348_bridge=True, has_v1349_audit_compat=True,
            guards_present=True,
        )
        sub_real, _ = v1350.v1350_subscore(
            record_count=1, event_count=1, reopen_count=0,
            audit_path=None, transitions_used_count=1,
            has_v1348_bridge=True, has_v1349_audit_compat=True,
            guards_present=True,
        )
        assert sub_empty < sub_real

    def test_subscore_no_bridges_lower(self):
        sub_full, _ = v1350.v1350_subscore(
            record_count=1, event_count=1, reopen_count=0,
            audit_path=None, transitions_used_count=1,
            has_v1348_bridge=True, has_v1349_audit_compat=True,
            guards_present=True,
        )
        sub_no_bridge, _ = v1350.v1350_subscore(
            record_count=1, event_count=1, reopen_count=0,
            audit_path=None, transitions_used_count=1,
            has_v1348_bridge=False, has_v1349_audit_compat=False,
            guards_present=True,
        )
        assert sub_no_bridge < sub_full


# ============================================================================
# Test 9: v1350_asi_lift
# ============================================================================
class TestASILift:
    def test_asi_lift_capped(self):
        lift = v1350.v1350_asi_lift(1.0)
        assert lift["v1350_asi_lift"] <= v1350.V1350_ASI_CAP

    def test_asi_lift_proportional(self):
        lift_high = v1350.v1350_asi_lift(1.0)
        lift_low = v1350.v1350_asi_lift(0.5)
        assert lift_high["v1350_asi_lift"] > lift_low["v1350_asi_lift"]

    def test_asi_lift_zero_subscore(self):
        lift = v1350.v1350_asi_lift(0.0)
        assert lift["v1350_asi_lift"] == 0.0

    def test_asi_lift_explanation(self):
        lift = v1350.v1350_asi_lift(0.85)
        assert "V1350" in lift["explanation"]
        assert "honest" in lift["explanation"].lower()


# ============================================================================
# Test 10: open_from_anomaly (V1348 bridge)
# ============================================================================
class TestOpenFromAnomaly:
    def test_open_creates_record(self, fresh_store, sample_anomaly_high):
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        assert rec.plugin == "plugin.alpha"
        assert rec.anomaly_id == "anom-test-alpha-001"
        assert rec.current_state == v1350.STATE_OPEN

    def test_evidence_captures_severity(self, fresh_store, sample_anomaly_high):
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        ev = rec.events[0].evidence
        assert ev["severity"] == v1348.SEVERITY_HIGH
        assert ev["anomaly_severity_rank"] == 3

    def test_evidence_captures_channels(self, fresh_store, sample_anomaly_high):
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        ev = rec.events[0].evidence
        # Channels list filters out NONE
        assert v1348.CHANNEL_HEALTH_DROP in ev["channels"]
        # Lint regression has LOW severity (still included)
        assert v1348.CHANNEL_LINT_REGRESSION in ev["channels"]


# ============================================================================
# Test 11: Self-tests integration
# ============================================================================
class TestSelfTests:
    def test_self_tests_all_pass(self):
        passed, total = v1350.run_self_tests(verbose=False)
        assert passed == total
        assert total >= 20  # At least 20 Popper tests


# ============================================================================
# Test 12: Interoperability
# ============================================================================
class TestInterop:
    def test_audit_jsonl_v1349_compatible_format(self, audit_store):
        """V1350 audit JSONL lines have the same shape as V1349 audit lines
        (event_id, plugin, anomaly_id, action, actor, reason, evidence, timestamp)."""
        store, path = audit_store
        store.open_anomaly("plugin.interop", "anom-interop-1", actor="alice", reason="init")
        store.apply("plugin.interop", "anom-interop-1", v1350.ACTION_ACKNOWLEDGE,
                    actor="alice", reason="looking", evidence={"reason": "looking"})
        lines = path.read_text(encoding="utf-8").strip().split("\n")
        for line in lines:
            d = json.loads(line)
            # Required fields
            assert "v1350_version" in d  # version field (vs V1349's v1349_version)
            assert "event_id" in d
            assert "plugin" in d
            assert "anomaly_id" in d
            assert "action" in d
            assert "actor" in d
            assert "reason" in d
            assert "evidence" in d
            assert "timestamp" in d

    def test_v1348_bridge_severity_propagates(self, fresh_store, sample_anomaly_medium):
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_medium, actor="alice")
        ev = rec.events[0].evidence
        assert ev["severity"] == v1348.SEVERITY_MEDIUM


# ============================================================================
# Test 13: End-to-end scenario
# ============================================================================
class TestEndToEnd:
    def test_full_lifecycle_high_severity(self, fresh_store, sample_anomaly_high):
        """Full path: HIGH → ack → escalate → resolve → close."""
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_high, actor="alice")
        rec = fresh_store.apply("plugin.alpha", sample_anomaly_high.anomaly_id,
                                v1350.ACTION_ACKNOWLEDGE, actor="alice",
                                reason="looking", evidence={"reason": "looking"})
        rec = fresh_store.apply("plugin.alpha", sample_anomaly_high.anomaly_id,
                                v1350.ACTION_ESCALATE, actor="bob",
                                reason="HIGH", evidence={"reason": "high",
                                                        "severity": v1348.SEVERITY_HIGH})
        rec = fresh_store.apply("plugin.alpha", sample_anomaly_high.anomaly_id,
                                v1350.ACTION_RESOLVE, actor="bob",
                                reason="fixed", evidence={"reason": "fixed",
                                                         "anomaly_gone": True})
        rec = fresh_store.apply("plugin.alpha", sample_anomaly_high.anomaly_id,
                                v1350.ACTION_CLOSE, actor="bob",
                                reason="archive", evidence={"reason": "archive"})
        assert rec.current_state == v1350.STATE_CLOSED
        assert len(rec.events) == 5  # open + ack + escalate + resolve + close

    def test_full_lifecycle_medium_mitigate(self, fresh_store, sample_anomaly_medium):
        """MEDIUM → ack → mitigate → resolve."""
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_medium, actor="alice")
        rec = fresh_store.apply("plugin.beta", sample_anomaly_medium.anomaly_id,
                                v1350.ACTION_ACKNOWLEDGE, actor="alice",
                                reason="looking", evidence={"reason": "looking"})
        rec = fresh_store.apply("plugin.beta", sample_anomaly_medium.anomaly_id,
                                v1350.ACTION_MITIGATE, actor="alice",
                                reason="apply fix", evidence={"reason": "fix",
                                                             "action_kind": "patch"})
        rec = fresh_store.apply("plugin.beta", sample_anomaly_medium.anomaly_id,
                                v1350.ACTION_RESOLVE, actor="alice",
                                reason="fixed", evidence={"reason": "fixed",
                                                         "anomaly_gone": True})
        assert rec.current_state == v1350.STATE_RESOLVED
        assert len(rec.events) == 4  # open + ack + mitigate + resolve

    def test_reopen_after_close(self, fresh_store, sample_anomaly_low):
        """LOW → ack → resolve → close → reopen → mitigate → resolve."""
        rec = v1350.open_from_anomaly(fresh_store, sample_anomaly_low, actor="alice")
        rec = fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                                v1350.ACTION_ACKNOWLEDGE, actor="alice",
                                reason="looking", evidence={"reason": "looking"})
        rec = fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                                v1350.ACTION_RESOLVE, actor="alice",
                                reason="fixed", evidence={"reason": "fixed",
                                                         "anomaly_gone": True})
        rec = fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                                v1350.ACTION_CLOSE, actor="alice",
                                reason="archive", evidence={"reason": "archive"})
        assert rec.current_state == v1350.STATE_CLOSED
        rec = fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                                v1350.ACTION_REOPEN, actor="alice",
                                reason="new", evidence={"reason": "new",
                                                       "new_anomaly_id": "anom-2"})
        assert rec.current_state == v1350.STATE_TRIAGED  # auto-transitioned
        # Now in TRIAGED, can mitigate
        rec = fresh_store.apply("plugin.gamma", sample_anomaly_low.anomaly_id,
                                v1350.ACTION_MITIGATE, actor="alice",
                                reason="fix", evidence={"reason": "fix",
                                                       "action_kind": "monitor"})
        assert rec.current_state == v1350.STATE_MITIGATED


# ============================================================================
# Test 14: Determinism + reproducibility
# ============================================================================
class TestDeterminism:
    def test_same_inputs_same_id(self):
        r1 = v1350.build_initial_record("p1", "a1", actor="alice",
                                        reason="init", evidence={"x": 1})
        r2 = v1350.build_initial_record("p1", "a1", actor="alice",
                                        reason="init", evidence={"x": 1})
        assert r1.lifecycle_id == r2.lifecycle_id
        assert r1.events[0].event_id == r2.events[0].event_id

    def test_evidence_order_irrelevant(self):
        r1 = v1350.build_initial_record("p1", "a1", actor="alice",
                                        reason="init", evidence={"a": 1, "b": 2})
        r2 = v1350.build_initial_record("p1", "a1", actor="alice",
                                        reason="init", evidence={"b": 2, "a": 1})
        assert r1.lifecycle_id == r2.lifecycle_id


# ============================================================================
# Test 15: Dataclass shape
# ============================================================================
class TestDataclassShape:
    def test_lifecycle_event_to_dict(self):
        ev = v1350.LifecycleEvent(
            event_index=0, state_before="<none>", state_after=v1350.STATE_OPEN,
            action="open", actor="alice", reason="init",
            evidence={}, timestamp="2026-01-01T00:00:00+00:00",
            event_id="0000000000000000",
        )
        d = ev.to_dict()
        assert d["event_index"] == 0
        assert d["state_after"] == v1350.STATE_OPEN
        assert d["action"] == "open"

    def test_lifecycle_record_to_dict(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        d = rec.to_dict()
        assert "lifecycle_id" in d
        assert "plugin" in d
        assert "anomaly_id" in d
        assert "current_state" in d
        assert "events" in d

    def test_ecosystem_report_to_dict(self):
        rec = v1350.build_initial_record("p1", "a1", actor="alice", reason="init")
        rollup = v1350.ecosystem_rollup([rec])
        d = rollup.to_dict()
        assert "per_plugin" in d
        assert "ecosystem_state" in d
        assert "state_breakdown" in d
        assert "report_id" in d


# ============================================================================
# Test 16: Chain regression (V1335-V1349 not broken)
# ============================================================================
class TestChainRegression:
    def test_v1348_anomaly_detector_still_works(self):
        """Smoke-test V1348 hasn't regressed."""
        recs = v1348.build_report([
            v1348.PluginAnomaly(
                plugin="p1", plugin_severity=v1348.SEVERITY_HIGH,
                plugin_severity_rank=3,
                channels=[v1348.ChannelSignal(
                    channel=v1348.CHANNEL_HEALTH_DROP, signal_score=1.0,
                    severity=v1348.SEVERITY_HIGH, evidence={"delta": 0.5},
                    recommendation="fix")],
                anomaly_id="a1",
            ),
        ])
        assert recs.ecosystem_severity == v1348.SEVERITY_HIGH

    def test_v1350_uses_v1348_constants(self):
        """V1350 references V1348 severity constants."""
        assert v1350.ESCALATION_SEVERITY == v1348.SEVERITY_HIGH
        assert v1348.SEVERITY_HIGH in v1350.ACTION_SEVERITY_REQUIRED[v1350.ACTION_ESCALATE]
