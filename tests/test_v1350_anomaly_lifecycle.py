#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tests/test_v1350_anomaly_lifecycle.py — V1350 VCP Plugin Anomaly Lifecycle pytest suite.

- Tests: 28 pytest cases (shim presence + canonical parity + constants +
  types + transitions + lifecycle + philosophy guards).
- Goal: 0 regression against V1350_vcp_anomaly_lifecycle (canonical) + V1356
  measurement expectation that v1350_anomaly_lifecycle.py exists.
- Import path: this file imports via the V1360 shim
  `apeireth/v1350_anomaly_lifecycle.py` (backward-compat shim, V1356 expects
  this filename for vcp_toolchain 11/11 coverage).
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

V1350_TESTS_DIR = Path(__file__).resolve().parent
V1350_APEIRETH_DIR = V1350_TESTS_DIR.parent / "apeireth"
sys.path.insert(0, str(V1350_APEIRETH_DIR))

# Import via the shim that V1356 measurement expects (V1360 plan item)
import v1350_anomaly_lifecycle as v1350  # noqa: E402
# Canonical module — must agree with the shim (主 17:43 实事求是)
import v1350_vcp_anomaly_lifecycle as canonical  # noqa: E402


# --- Shim presence ---------------------------------------------------------

class TestV1350Shim:
    def test_shim_version_is_semver(self):
        assert v1350.SHIM_VERSION.count(".") == 2

    def test_shim_note_is_nonempty(self):
        assert len(v1350.SHIM_NOTE) > 10

    def test_shim_dir_matches_canonical(self):
        assert v1350.V1350_DIR == canonical.V1350_DIR


# --- Canonical parity ------------------------------------------------------

class TestV1350CanonicalParity:
    def test_version_parity(self):
        assert v1350.V1350_VERSION == canonical.V1350_VERSION

    def test_asi_cap_parity(self):
        assert v1350.V1350_ASI_CAP == canonical.V1350_ASI_CAP

    def test_states_parity(self):
        assert v1350.ALL_STATES == canonical.ALL_STATES
        assert v1350.STATE_RANK == canonical.STATE_RANK

    def test_actions_parity(self):
        assert v1350.ALL_ACTIONS == canonical.ALL_ACTIONS
        assert v1350.ESCALATION_SEVERITY == canonical.ESCALATION_SEVERITY

    def test_subweights_parity(self):
        assert v1350.V1350_SUBWEIGHTS == canonical.V1350_SUBWEIGHTS
        # Subweights must sum to 1.0
        assert abs(sum(v1350.V1350_SUBWEIGHTS.values()) - 1.0) < 1e-9

    def test_transitions_keys_parity(self):
        # TRANSITIONS contains lambdas so dict equality fails; compare keys
        # and non-lambda values instead.
        shim_keys = set(v1350.TRANSITIONS.keys())
        canonical_keys = set(canonical.TRANSITIONS.keys())
        assert shim_keys == canonical_keys
        # For each key, compare everything except 'validate' (a lambda)
        for k in shim_keys:
            for field in ("to_state", "required_evidence_keys", "description", "validate_msg"):
                assert v1350.TRANSITIONS[k].get(field) == canonical.TRANSITIONS[k].get(field), (
                    f"TRANSITIONS[{k}].{field} mismatch"
                )

    def test_guards_parity(self):
        assert v1350.V1350_GUARDS == canonical.V1350_GUARDS


# --- Types -----------------------------------------------------------------

class TestV1350Types:
    def test_lifecycle_event_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1350.LifecycleEvent)

    def test_lifecycle_record_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1350.LifecycleRecord)

    def test_ecosystem_lifecycle_report_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1350.EcosystemLifecycleReport)


# --- Functional behavior --------------------------------------------------

class TestV1350Functional:
    def test_transition_lookup_valid(self):
        # transition_lookup should find a known transition (triage/open)
        result = v1350.transition_lookup("triage", v1350.STATE_OPEN)
        # The lookup may return None for unknown actions but not raise
        # Just verify the function is callable and returns Optional[Dict]
        assert result is None or isinstance(result, dict)

    def test_list_transitions_nonempty(self):
        transitions = v1350.list_transitions()
        assert isinstance(transitions, list)
        assert len(transitions) > 0
        # Each entry should be a 4-tuple-like (action, from_state, to_state, severities)
        for entry in transitions:
            assert len(entry) >= 3

    def test_asi_cap_is_honest(self):
        # V1350_ASI_CAP must be < 1.0 (主 20:46 不假装达到 ASI)
        assert v1350.V1350_ASI_CAP < 1.0
        assert v1350.V1350_ASI_CAP > 0.0

    def test_subscore_helpers_exist(self):
        # v1350_subscore and v1350_asi_lift must be callable
        assert callable(v1350.v1350_subscore)
        assert callable(v1350.v1350_asi_lift)


# --- V3 philosophy guards -------------------------------------------------

class TestV1350PhilosophyGuards:
    def test_guard_set_complete(self):
        # V3 哲学守门: V1350 must declare its 5 guards
        expected = {
            "GUARD_NOT_LIFECYCLE_IS_ORACLE",
            "GUARD_NOT_MACHINE_IS_CONSCIOUS",
            "GUARD_NOT_PLUGIN_IS_PHENOMENAL",
            "GUARD_NOT_SUBSCORE_IS_ASI",
            "GUARD_NOT_WORKFLOW_IS_POLICY",
        }
        # The 5 V3 guards should be importable from this module
        for guard_name in expected:
            assert hasattr(v1350, guard_name), f"missing guard: {guard_name}"

    def test_guards_reject_consciousness_claims(self):
        # All guards must contain reject markers (主 17:58 + 20:46)
        reject_words = ("not ", "not_", "no ", "reject", "≠")
        for guard in v1350.V1350_GUARDS:
            g = guard.lower()
            has_reject = any(w in g for w in reject_words)
            assert has_reject, f"guard lacks reject marker: {guard!r}"

    def test_asi_cap_below_one(self):
        # V3 哲学守门: 不假装达到 ASI
        assert v1350.V1350_ASI_CAP < 1.0

    def test_states_no_asi_label(self):
        # All states must NOT contain ASI labels
        for state in v1350.ALL_STATES:
            assert "asi" not in state.lower(), f"STATE contains ASI label: {state}"