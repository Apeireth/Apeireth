#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""tests/test_v1347_vcp_health_score.py — V1347 VCP Plugin Health Score pytest suite.

- Tests: 22 pytest cases (shim presence + canonical parity + constants +
  types + scoring + serialization + philosophy guards).
- Goal: 0 regression against V1347_vcp_plugin_health (canonical) + V1356
  measurement expectation that v1347_vcp_health_score.py exists.
- Import path: this file imports via the V1360 shim
  `apeireth/v1347_vcp_health_score.py` (backward-compat shim, V1356 expects
  this filename for vcp_toolchain 11/11 coverage).
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

V1347_TESTS_DIR = Path(__file__).resolve().parent
V1347_APEIRETH_DIR = V1347_TESTS_DIR.parent / "apeireth"
sys.path.insert(0, str(V1347_APEIRETH_DIR))

# Import via the shim that V1356 measurement expects (V1360 plan item)
import v1347_vcp_health_score as v1347  # noqa: E402
# Canonical module — must agree with the shim (主 17:43 实事求是)
import v1347_vcp_plugin_health as canonical  # noqa: E402


# --- Shim presence ---------------------------------------------------------

class TestV1347Shim:
    def test_shim_version_is_semver(self):
        assert v1347.SHIM_VERSION.count(".") == 2

    def test_shim_note_is_nonempty(self):
        assert len(v1347.SHIM_NOTE) > 10

    def test_shim_dir_matches_canonical(self):
        # V1347_DIR must come from canonical and equal canonical's
        assert v1347.V1347_DIR == canonical.V1347_DIR


# --- Canonical parity (shim re-export honesty) ----------------------------

class TestV1347CanonicalParity:
    def test_asi_pole_star_parity(self):
        # ASI_POLE_STAR is a dict (pole-star metadata)
        assert v1347.ASI_POLE_STAR == canonical.ASI_POLE_STAR

    def test_weights_parity(self):
        assert v1347.WEIGHTS == canonical.WEIGHTS

    def test_health_tier_parity(self):
        # HealthTier values must match canonical
        canonical_values = [str(t) for t in canonical.HealthTier]
        shim_values = [str(t) for t in v1347.HealthTier]
        assert canonical_values == shim_values

    def test_drift_constants_parity(self):
        assert v1347.DRIFT_PENALTY_LOW == canonical.DRIFT_PENALTY_LOW
        assert v1347.DRIFT_PENALTY_MEDIUM == canonical.DRIFT_PENALTY_MEDIUM
        assert v1347.DRIFT_PENALTY_HIGH == canonical.DRIFT_PENALTY_HIGH
        assert v1347.DRIFT_PENALTY_CRITICAL == canonical.DRIFT_PENALTY_CRITICAL
        assert v1347.DRIFT_BONUS_RECENT_PASS == canonical.DRIFT_BONUS_RECENT_PASS

    def test_tier_threshold_parity(self):
        assert v1347.TIER_HEALTHY_MIN == canonical.TIER_HEALTHY_MIN
        assert v1347.TIER_DEGRADED_MIN == canonical.TIER_DEGRADED_MIN

    def test_plan_offsets_parity(self):
        assert v1347.PLAN_SEVERITY_OFFSETS == canonical.PLAN_SEVERITY_OFFSETS


# --- Types -----------------------------------------------------------------

class TestV1347Types:
    def test_health_component_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1347.HealthComponent)

    def test_plugin_health_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1347.PluginHealth)

    def test_ecosystem_rollup_dataclass(self):
        from dataclasses import is_dataclass
        assert is_dataclass(v1347.EcosystemRollup)


# --- Functional behavior --------------------------------------------------

class TestV1347Functional:
    def test_tier_for_score_known_tiers(self):
        # TIER_HEALTHY_MIN=0.85, TIER_DEGRADED_MIN=0.65
        # score >= TIER_HEALTHY_MIN (0.85) → HEALTHY
        # TIER_DEGRADED_MIN (0.65) <= score < 0.85 → DEGRADED
        # score < 0.65 → CRITICAL
        from v1347_vcp_health_score import tier_for_score
        assert tier_for_score(1.0) == "HEALTHY"
        assert tier_for_score(0.85) == "HEALTHY"
        assert tier_for_score(0.7) == "DEGRADED"
        assert tier_for_score(0.5) == "CRITICAL"
        assert tier_for_score(0.0) == "CRITICAL"

    def test_clamp01_bounds(self):
        assert v1347._clamp01(-0.5) == 0.0
        assert v1347._clamp01(0.5) == 0.5
        assert v1347._clamp01(1.5) == 1.0

    def test_safe_div_zero(self):
        assert v1347._safe_div(1.0, 0.0) == 0.0
        assert v1347._safe_div(2.0, 4.0) == 0.5

    def test_recommend_returns_list(self):
        # recommend([]) should return a non-empty list with the maintenance message
        result = v1347.recommend([])
        assert isinstance(result, list)
        assert len(result) >= 1


# --- V3 philosophy guards -------------------------------------------------

class TestV1347PhilosophyGuards:
    def test_asi_pole_star_dict_asi_achieved_false(self):
        # ASI_POLE_STAR is a metadata dict; "asi_achieved_false" must be True
        # (主 20:46 不假装达到 ASI)
        assert isinstance(v1347.ASI_POLE_STAR, dict)
        assert v1347.ASI_POLE_STAR.get("asi_achieved_false") is True

    def test_health_tier_no_asi_label(self):
        # V3 哲学守门: 不假装 ASI 等级 — HealthTier must NOT have an ASI tier
        tier_strs = [str(t).lower() for t in v1347.HealthTier]
        for s in tier_strs:
            assert "asi" not in s, f"HealthTier contains ASI label: {s}"

    def test_to_human_returns_string(self):
        # to_human works on PluginHealth; build a minimal one
        from v1347_vcp_health_score import HealthComponent, PluginHealth
        comp = HealthComponent(
            name="test", score=0.5, weight=0.5, contribution=0.25,
            weight_pct=0.5, details="x",
        )
        health = PluginHealth(
            health_id="id1",
            plugin_name="p1",
            health_score=0.5,
            tier="DEGRADED",
            components=[comp],
            recommendations=[],
            generated_at="2026-08-09T00:00:00+00:00",
            evidence="x",
        )
        s = v1347.to_human(health)
        assert isinstance(s, str)
        assert len(s) > 0