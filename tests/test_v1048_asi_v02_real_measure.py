"""Tests for v1048 ASI V0.2 16 项真生产测量 (主 17:43 实事求是 + 主 22:33 ASI 北极星 +
主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 +
主 23:44 干到底 + 主 00:56 任何人都能接手).
"""
from __future__ import annotations

import math
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from apeireth.v1048_asi_v02_real_measure import (
    V02_MEASUREMENT_FUNCS,
    V02_REAL_WEIGHTS,
    V02RealMeasurement,
    V1048ASIV02RealMeasure,
    V1048_VERSION,
    measure_asi_v02_real,
    measure_cognitive_core,
    measure_cross_domain,
    measure_engineering,
    measure_neurosymbolic,
    measure_phi_proxy,
    measure_plugin_core,
    measure_real_production,
    measure_reinforcement_learning,
    measure_rubric_open,
    measure_scientific_method,
    measure_self_improving_core,
    measure_self_organizing_core,
    measure_v2_philosophy,
    measure_vcp_4,
    measure_world_model,
)


# ----------------------------------------------------------------------
# Constants and weight validation
# ----------------------------------------------------------------------


class TestWeights:
    def test_weights_sum_to_1(self):
        s = sum(V02_REAL_WEIGHTS.values())
        assert abs(s - 1.0) < 1e-9, f"V02_REAL_WEIGHTS sum = {s}, expected 1.0"

    def test_weights_have_16_keys(self):
        assert len(V02_REAL_WEIGHTS) == 16, (
            f"V02 formula should have 16 components, got {len(V02_REAL_WEIGHTS)}"
        )

    def test_weights_in_unit_interval(self):
        for comp, w in V02_REAL_WEIGHTS.items():
            assert 0.0 < w <= 1.0, f"{comp} weight {w} out of range"

    def test_measurement_functions_have_16_entries(self):
        assert len(V02_MEASUREMENT_FUNCS) == 16

    def test_weights_and_functions_keys_match(self):
        assert set(V02_REAL_WEIGHTS.keys()) == set(V02_MEASUREMENT_FUNCS.keys())

    def test_v0_1_keys_present(self):
        for k in [
            "phi_proxy", "capabilities", "cross_domain", "engineering",
            "vcp_4", "v2_philosophy", "rubric_open", "real_production",
        ]:
            assert k in V02_REAL_WEIGHTS, f"V0.1 legacy key {k} missing"

    def test_v0_2_new_keys_present(self):
        for k in [
            "cognitive_core", "self_organizing_core", "plugin_core",
            "self_improving_core", "neurosymbolic", "world_model",
            "reinforcement_learning", "scientific_method",
        ]:
            assert k in V02_REAL_WEIGHTS, f"V0.2 new key {k} missing"


# ----------------------------------------------------------------------
# Individual measurement functions
# ----------------------------------------------------------------------


class TestIndividualMeasurements:
    def test_cognitive_core_in_unit_interval(self):
        s = measure_cognitive_core()
        assert 0.0 <= s <= 1.0, f"cognitive_core {s} out of [0,1]"

    def test_self_organizing_core_in_unit_interval(self):
        s = measure_self_organizing_core()
        assert 0.0 <= s <= 1.0

    def test_plugin_core_in_unit_interval(self):
        s = measure_plugin_core()
        assert 0.0 <= s <= 1.0

    def test_self_improving_core_in_unit_interval(self):
        s = measure_self_improving_core()
        assert 0.0 <= s <= 1.0

    def test_neurosymbolic_in_unit_interval(self):
        s = measure_neurosymbolic()
        assert 0.0 <= s <= 1.0

    def test_world_model_in_unit_interval(self):
        s = measure_world_model()
        assert 0.0 <= s <= 1.0

    def test_reinforcement_learning_in_unit_interval(self):
        s = measure_reinforcement_learning()
        assert 0.0 <= s <= 1.0

    def test_scientific_method_in_unit_interval(self):
        s = measure_scientific_method()
        assert 0.0 <= s <= 1.0

    def test_phi_proxy_in_unit_interval(self):
        s = measure_phi_proxy()
        assert 0.0 <= s <= 1.0

    def test_capabilities_in_unit_interval(self):
        from apeireth.v1048_asi_v02_real_measure import measure_capabilities
        s = measure_capabilities()
        assert 0.0 <= s <= 1.0

    def test_cross_domain_in_unit_interval(self):
        s = measure_cross_domain()
        assert 0.0 <= s <= 1.0

    def test_engineering_in_unit_interval(self):
        s = measure_engineering()
        assert 0.0 <= s <= 1.0

    def test_vcp_4_in_unit_interval(self):
        s = measure_vcp_4()
        assert 0.0 <= s <= 1.0

    def test_v2_philosophy_in_unit_interval(self):
        s = measure_v2_philosophy()
        assert 0.0 <= s <= 1.0

    def test_rubric_open_in_unit_interval(self):
        s = measure_rubric_open()
        assert 0.0 <= s <= 1.0

    def test_real_production_in_unit_interval(self):
        s = measure_real_production()
        assert 0.0 <= s <= 1.0


# ----------------------------------------------------------------------
# Real production anchors (these should be > 0 in a real production repo)
# ----------------------------------------------------------------------


class TestRealProductionAnchors:
    def test_cognitive_core_positive(self):
        # V43 真生产模块 + V1045 真借鉴 → 应当 > 0
        s = measure_cognitive_core()
        assert s > 0.3, f"cognitive_core {s} should reflect real V43 production"

    def test_self_organizing_core_positive(self):
        s = measure_self_organizing_core()
        assert s > 0.3, f"self_organizing_core {s} should reflect V47 production"

    def test_plugin_core_positive(self):
        s = measure_plugin_core()
        assert s > 0.3, f"plugin_core {s} should reflect V48 production"

    def test_self_improving_core_positive(self):
        s = measure_self_improving_core()
        assert s > 0.3, f"self_improving_core {s} should reflect V49 production"

    def test_neurosymbolic_positive(self):
        s = measure_neurosymbolic()
        assert s > 0.3, f"neurosymbolic {s} should reflect V51 production"

    def test_world_model_positive(self):
        s = measure_world_model()
        assert s > 0.3, f"world_model {s} should reflect V52 production"

    def test_reinforcement_learning_positive(self):
        s = measure_reinforcement_learning()
        assert s > 0.3, f"reinforcement_learning {s} should reflect V53 production"

    def test_scientific_method_positive(self):
        s = measure_scientific_method()
        assert s > 0.3, f"scientific_method {s} should reflect V57 production"

    def test_vcp_4_full_coverage(self):
        # 6 plugins + 4 contexts + sync executions → expect ~1.0
        s = measure_vcp_4()
        assert s > 0.7, f"vcp_4 {s} should be high (6 plugins + 4 contexts)"

    def test_phi_proxy_full(self):
        # tests >= 2000 → expect 1.0
        s = measure_phi_proxy()
        assert s > 0.5, f"phi_proxy {s} should reflect >1000 tests"

    def test_capabilities_high(self):
        from apeireth.v1048_asi_v02_real_measure import measure_capabilities
        s = measure_capabilities()
        assert s > 0.5, f"capabilities {s} should be high (>750 v-modules)"

    def test_cross_domain_full(self):
        # >= 50 ASI-/APEIRETH- docs → 1.0
        s = measure_cross_domain()
        assert s > 0.5, f"cross_domain {s} should be high"

    def test_engineering_high(self):
        # >= 25000 lines in apeireth/v*.py → 1.0
        s = measure_engineering()
        assert s > 0.5, f"engineering {s} should be high"

    def test_real_production_high(self):
        # >= 500 commits → 1.0
        s = measure_real_production()
        assert s > 0.3, f"real_production {s} should be >0.3 (>150 commits)"


# ----------------------------------------------------------------------
# V0.2 total measurement
# ----------------------------------------------------------------------


class TestMeasureAsiV02Real:
    def test_measure_returns_dataclass(self):
        m = measure_asi_v02_real()
        assert isinstance(m, V02RealMeasurement)
        assert m.measurement_id.startswith("m_")
        assert m.total >= 0.0
        assert m.total <= 1.0 + 1e-9  # allow tiny float error

    def test_measure_has_16_component_scores(self):
        m = measure_asi_v02_real()
        assert len(m.component_scores) == 16
        assert len(m.contributions) == 16

    def test_measure_exceeds_v0_1(self):
        # 主 22:33 ASI 北极星: V0.1 = 0.7905 真测; V0.2 真生产应超越之 (干到底)
        m = measure_asi_v02_real()
        assert m.total >= 0.79, f"V0.2 real = {m.total}, should be ≥0.79 (V0.1 真测 0.7905)"

    def test_measure_level_is_asi(self):
        m = measure_asi_v02_real()
        assert m.level == "ASI", f"V0.2 real level = {m.level}, expected ASI (≥0.7)"

    def test_measure_total_ge_v0_2_legacy(self):
        # V1002 V0.2 = 0.4467 (旧版); V1048 V0.2 真生产应该远超之
        m = measure_asi_v02_real()
        assert m.total >= 0.7, f"V0.2 real = {m.total}, should be >> 0.4467 legacy"

    def test_measure_with_overrides(self):
        m = measure_asi_v02_real(scores={"phi_proxy": 0.5})
        assert m.component_scores["phi_proxy"] == 0.5

    def test_measure_with_invalid_override_clamps(self):
        m = measure_asi_v02_real(scores={"phi_proxy": 5.0})
        assert m.component_scores["phi_proxy"] == 1.0

    def test_measure_with_negative_override_clamps(self):
        m = measure_asi_v02_real(scores={"engineering": -0.5})
        assert m.component_scores["engineering"] == 0.0


# ----------------------------------------------------------------------
# V1048ASIV02RealMeasure class
# ----------------------------------------------------------------------


class TestV1048Class:
    def test_class_starts_empty(self):
        m = V1048ASIV02RealMeasure()
        assert m.n_measurements() == 0
        assert m.average_total() == 0.0

    def test_measure_appends(self):
        m = V1048ASIV02RealMeasure()
        result = m.measure()
        assert m.n_measurements() == 1
        assert m.average_total() == result.total

    def test_multiple_measurements_average(self):
        m = V1048ASIV02RealMeasure()
        r1 = m.measure()
        r2 = m.measure()
        expected = (r1.total + r2.total) / 2
        assert abs(m.average_total() - expected) < 1e-9

    def test_average_by_component_has_16(self):
        m = V1048ASIV02RealMeasure()
        m.measure()
        m.measure()
        avg = m.average_by_component()
        assert len(avg) == 16

    def test_stats_includes_not_pretend(self):
        m = V1048ASIV02RealMeasure()
        stats = m.stats()
        assert stats["not_pretend"] is True
        assert stats["n_components"] == 16
        assert stats["version"] == V1048_VERSION
        assert abs(stats["v02_weights_sum"] - 1.0) < 1e-9


# ----------------------------------------------------------------------
# Version & module metadata
# ----------------------------------------------------------------------


class TestModuleMetadata:
    def test_version_is_string(self):
        assert isinstance(V1048_VERSION, str)
        assert V1048_VERSION  # non-empty


# ----------------------------------------------------------------------
# Determinism (call twice → same score)
# ----------------------------------------------------------------------


class TestDeterminism:
    def test_phi_proxy_deterministic(self):
        s1 = measure_phi_proxy()
        s2 = measure_phi_proxy()
        assert s1 == s2

    def test_capabilities_deterministic(self):
        from apeireth.v1048_asi_v02_real_measure import measure_capabilities
        s1 = measure_capabilities()
        s2 = measure_capabilities()
        assert s1 == s2

    def test_cross_domain_deterministic(self):
        s1 = measure_cross_domain()
        s2 = measure_cross_domain()
        assert s1 == s2

    def test_engineering_deterministic(self):
        s1 = measure_engineering()
        s2 = measure_engineering()
        assert s1 == s2

    def test_real_production_deterministic(self):
        s1 = measure_real_production()
        s2 = measure_real_production()
        assert s1 == s2

    def test_v2_philosophy_deterministic(self):
        s1 = measure_v2_philosophy()
        s2 = measure_v2_philosophy()
        assert s1 == s2

    def test_rubric_open_deterministic(self):
        s1 = measure_rubric_open()
        s2 = measure_rubric_open()
        assert s1 == s2


# ----------------------------------------------------------------------
# Integration / end-to-end
# ----------------------------------------------------------------------


class TestIntegration:
    def test_full_measure_workflow(self):
        m = V1048ASIV02RealMeasure()
        result = m.measure()
        assert result.total > 0.5, f"V0.2 real {result.total} should be substantial"
        assert result.level == "ASI"
        # All 16 contributions should sum to total
        csum = sum(info["contribution"] for info in result.contributions.values())
        assert abs(csum - result.total) < 1e-3

    def test_demo_runs(self):
        from apeireth.v1048_asi_v02_real_measure import _demo
        # Just check it doesn't raise
        _demo()

    def test_anchored_to_v0_1(self):
        # 主 22:33 ASI 北极星: V1048 V0.2 真生产 > V0.1 = 0.7905
        m = V1048ASIV02RealMeasure()
        result = m.measure()
        assert result.total >= 0.7905, (
            f"V0.2 real {result.total} should be ≥ V0.1 0.7905 (干到底)"
        )