"""V1002 真生产 tests (主 23:44)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1002_asi_v02_measure import (
    V1002_VERSION, ASI_V02_WEIGHTS, V02Measurement,
    compute_asi_v02_total, measure_real_production, V1002ASIV02Measure,
)


class TestV1002:
    def test_16_components(self):
        assert len(ASI_V02_WEIGHTS) == 16

    def test_weights_sum(self):
        total = sum(ASI_V02_WEIGHTS.values())
        assert abs(total - 1.0) < 0.001

    def test_compute_v02_asi_level(self):
        m = compute_asi_v02_total({k: 0.9 for k in ASI_V02_WEIGHTS})
        assert m.level == "ASI"
        assert m.total >= 0.7

    def test_compute_v02_ani_level(self):
        m = compute_asi_v02_total({k: 0.0 for k in ASI_V02_WEIGHTS})
        assert m.level == "ANI"
        assert m.total < 0.3

    def test_compute_v02_agi_level(self):
        m = compute_asi_v02_total({k: 0.5 for k in ASI_V02_WEIGHTS})
        assert m.level == "AGI"

    def test_compute_v02_clamp(self):
        m = compute_asi_v02_total({"phi_proxy": 5.0})
        assert m.contributions["phi_proxy"]["raw_score"] == 1.0

    def test_measure_real_production(self):
        scores = measure_real_production()
        assert "phi_proxy" in scores
        assert "capabilities" in scores
        assert "real_production" in scores

    def test_measure_class(self):
        m = V1002ASIV02Measure()
        result = m.measure()
        assert result.level in ("ANI", "AGI", "ASI")

    def test_measure_class_with_custom_scores(self):
        m = V1002ASIV02Measure()
        result = m.measure({"phi_proxy": 0.9, "capabilities": 0.9})
        # raw_score 应在 [0, 1] 范围
        assert 0 <= result.contributions["phi_proxy"]["raw_score"] <= 1.0
        assert result.contributions["capabilities"]["raw_score"] <= 1.0

    def test_n_measurements(self):
        m = V1002ASIV02Measure()
        m.measure()
        m.measure()
        assert m.n_measurements() == 2

    def test_average_total(self):
        m = V1002ASIV02Measure()
        m.measure({"phi_proxy": 0.5})
        m.measure({"phi_proxy": 0.7})
        avg = m.average_total()
        assert avg > 0.05

    def test_stats(self):
        m = V1002ASIV02Measure()
        m.measure()
        s = m.stats()
        assert s["n_components"] == 16
        assert s["version"] == V1002_VERSION

    def test_real_production_scores_bounded(self):
        scores = measure_real_production()
        for k, v in scores.items():
            assert 0 <= v <= 1.0

    def test_16_components_weights(self):
        # V21 V0.1 8 项 + V54 8 项
        v21_components = ["phi_proxy", "capabilities", "cross_domain",
                          "engineering", "vcp_4", "v2_philosophy",
                          "rubric_open", "real_production"]
        v54_components = ["cognitive_core", "self_organizing_core",
                          "plugin_core", "self_improving_core",
                          "neurosymbolic", "world_model",
                          "reinforcement_learning", "scientific_method"]
        for c in v21_components + v54_components:
            assert c in ASI_V02_WEIGHTS

    def test_each_component_has_valid_weight(self):
        for comp, w in ASI_V02_WEIGHTS.items():
            assert 0 < w <= 0.2  # 没有权重 > 0.2