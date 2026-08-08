#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tests/test_v1348_anomaly_detector.py — V1348 VCP Plugin Anomaly Detector pytest suite.

- Tests: 22 pytest cases (shim presence + canonical parity + constants +
  types + severity helpers + philosophy guards).
- Goal: 0 regression against V1348_vcp_anomaly_detector (canonical) + V1356
  measurement expectation that v1348_anomaly_detector.py exists.
- Import path: this file imports via the V1360 shim
  `apeireth/v1348_anomaly_detector.py` (backward-compat shim, V1356 expects
  this filename for vcp_toolchain 11/11 coverage).
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

V1348_TESTS_DIR = Path(__file__).resolve().parent
V1348_APEIRETH_DIR = V1348_TESTS_DIR.parent / "apeireth"
sys.path.insert(0, str(V1348_APEIRETH_DIR))

# Import via the shim that V1356 measurement expects (V1360 plan item)
import v1348_anomaly_detector as v1348  # noqa: E402
# Canonical module — must agree with the shim (主 17:43 实事求是)
import v1348_vcp_anomaly_detector as canonical  # noqa: E402


# --- Shim presence ---------------------------------------------------------

class TestV1348Shim:
    def test_shim_version_is_semver(self):
        assert v1348.SHIM_VERSION.count(".") == 2

    def test_shim_note_is_nonempty(self):
        assert len(v1348.SHIM_NOTE) > 10

    def test_shim_dir_matches_canonical(self):
        assert v1348.V1348_DIR == canonical.V1348_DIR


# --- Canonical parity ------------------------------------------------------

class TestV1348CanonicalParity:
    def test_asi_pole_star_parity(self):
        assert v1348.ASI_POLE_STAR == canonical.ASI_POLE_STAR

    def test_all_channels_parity(self):
        assert v1348.ALL_CHANNELS == canonical.ALL_CHANNELS

    def test_channel_constants_parity(self):
        assert v1348.CHANNEL_TIER_JUMP == canonical.CHANNEL_TIER_JUMP
        assert v1348.CHANNEL_LINT_REGRESSION == canonical.CHANNEL_LINT_REGRESSION
        assert v1348.CHANNEL_DRIFT_SPIKE == canonical.CHANNEL_DRIFT_SPIKE
        assert v1348.CHANNEL_PLAN_ACCELERATION == canonical.CHANNEL_PLAN_ACCELERATION
        assert v1348.CHANNEL_HEALTH_DROP == canonical.CHANNEL_HEALTH_DROP

    def test_severity_order_parity(self):
        assert v1348.SEVERITY_ORDER == canonical.SEVERITY_ORDER

    def test_tier_rank_parity(self):
        assert v1348.TIER_RANK == canonical.TIER_RANK

    def test_default_thresholds_parity(self):
        assert v1348.DEFAULT_THRESHOLDS == canonical.DEFAULT_THRESHOLDS


# --- Types -----------------------------------------------------------------

class TestV1348Types:
    def test_channel_signal_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1348.ChannelSignal)

    def test_plugin_anomaly_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1348.PluginAnomaly)

    def test_ecosystem_anomaly_report_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1348.EcosystemAnomalyReport)


# --- Functional behavior --------------------------------------------------

class TestV1348Functional:
    def test_tier_rank_known_keys(self):
        # TIER_RANK has keys: 'low'=0, 'medium'=2, 'high'=3, 'v1335_manual'=1
        # medium must outrank low
        assert v1348.tier_rank("medium") > v1348.tier_rank("low")
        assert v1348.tier_rank("high") > v1348.tier_rank("medium")
        assert v1348.tier_rank("low") == 0

    def test_max_severity_picks_highest(self):
        from v1348_anomaly_detector import (
            SEVERITY_LOW, SEVERITY_MEDIUM, SEVERITY_HIGH, max_severity,
        )
        assert max_severity([SEVERITY_LOW, SEVERITY_HIGH, SEVERITY_MEDIUM]) == SEVERITY_HIGH
        assert max_severity([SEVERITY_LOW]) == SEVERITY_LOW
        assert max_severity([]) == canonical.SEVERITY_NONE

    def test_severity_for_score_thresholds(self):
        # DEFAULT_THRESHOLDS: low_floor=0.34, medium_floor=0.67, high_floor=1.0
        # score >= 1.0 → HIGH; >= 0.67 → MEDIUM; >= 0.34 → LOW; else NONE
        from v1348_anomaly_detector import severity_for_score
        assert severity_for_score(1.0, v1348.DEFAULT_THRESHOLDS) == canonical.SEVERITY_HIGH
        assert severity_for_score(0.8, v1348.DEFAULT_THRESHOLDS) == canonical.SEVERITY_MEDIUM
        assert severity_for_score(0.5, v1348.DEFAULT_THRESHOLDS) == canonical.SEVERITY_LOW
        assert severity_for_score(0.1, v1348.DEFAULT_THRESHOLDS) == canonical.SEVERITY_NONE


# --- V3 philosophy guards -------------------------------------------------

class TestV1348PhilosophyGuards:
    def test_asi_pole_star_dict_asi_achieved_false(self):
        # ASI_POLE_STAR is a metadata dict; "asi_achieved_false" must be True
        # (主 20:46 不假装达到 ASI)
        assert isinstance(v1348.ASI_POLE_STAR, dict)
        assert v1348.ASI_POLE_STAR.get("asi_achieved_false") is True

    def test_recommendations_no_asi_label(self):
        # RECOMMENDATIONS keys are tuples like ('tier_jump', 'LOW')
        # V3 哲学守门: 不假装 ASI 等级 — keys must NOT contain ASI labels
        for key in v1348.RECOMMENDATIONS:
            key_str = " ".join(str(k) for k in key).lower()
            assert "asi" not in key_str, f"RECOMMENDATIONS contains ASI label: {key}"

    def test_severity_order_contains_none(self):
        # SEVERITY_ORDER must contain "none" as the lowest severity
        assert canonical.SEVERITY_NONE in v1348.SEVERITY_ORDER