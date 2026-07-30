"""V1153 tests — ASI V0.6 formal spec."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth import v1153_asi_v06_formal_spec as v1153


# ============================================================================
# Module API
# ============================================================================


class TestV1153ModuleAPI:
    def test_version_constant(self):
        assert v1153.V1153_VERSION == "0.1.0"

    def test_north_star_locked(self):
        assert v1153.ASI_NORTH_STAR == 0.9800

    def test_v05_dims_17(self):
        assert len(v1153.V05_17DIMS) == 17

    def test_v06_new_dims_4(self):
        assert len(v1153.V06_NEW_DIMS) == 4
        for dim in ["llm_bridge", "multi_agent_dag", "vcp_real_run", "vcp_deep_read"]:
            assert dim in v1153.V06_NEW_DIMS

    def test_v06_21_dims(self):
        assert len(v1153.V06_21DIMS) == 21

    def test_weights_sum_to_one(self):
        s = sum(v1153.V06_DIM_WEIGHTS.values())
        assert abs(s - 1.0) < 1e-9


# ============================================================================
# V0.6 spec measurement
# ============================================================================


class TestV1153MeasureSpec:
    def test_measure_v06_returns_spec(self):
        spec = v1153.measure_v06_spec()
        assert spec.n_dims == 21
        assert 0.0 <= spec.asi_v06_score <= 1.0
        assert spec.north_star == 0.9800
        assert len(spec.dim_results) == 21

    def test_measure_v06_gap_is_correct(self):
        spec = v1153.measure_v06_spec()
        expected_gap = spec.asi_v06_score - spec.north_star
        assert abs(spec.gap - expected_gap) < 1e-9

    def test_measure_v06_has_dim_results_for_each(self):
        spec = v1153.measure_v06_spec()
        dims_in_results = {r.dim for r in spec.dim_results}
        assert dims_in_results == set(v1153.V06_21DIMS)

    def test_measure_v06_snapshot_id_format(self):
        spec = v1153.measure_v06_spec()
        assert spec.snapshot_id.startswith("v1153-")
        assert len(spec.snapshot_id) == len("v1153-") + 8

    def test_measure_v06_status_classification(self):
        spec = v1153.measure_v06_spec()
        # 应该有 R 状态 (主 17:43 不假装)
        total = spec.n_real + spec.n_hardcoded + spec.n_partial + spec.n_missing
        assert total == spec.n_dims


# ============================================================================
# V0.6 acceptance
# ============================================================================


class TestV1153Acceptance:
    def test_acceptance_returns_5_tests(self):
        acc = v1153.run_v06_acceptance()
        assert acc.n_tests == 5
        assert acc.n_pass + acc.n_fail == 5

    def test_acceptance_5_pass(self):
        # 应该全 pass (主 17:43 实事求是: spec 必须可证伪 + 真跑过)
        acc = v1153.run_v06_acceptance()
        assert acc.n_fail == 0
        assert acc.n_pass == 5

    def test_acceptance_tests_have_names(self):
        acc = v1153.run_v06_acceptance()
        names = {t["name"] for t in acc.tests}
        assert "spec_formula_present" in names
        assert "n_dims_21" in names
        assert "weights_sum_one" in names
        assert "asi_v06_in_range" in names
        assert "gap_correctly_computed" in names

    def test_acceptance_snapshot_id(self):
        acc = v1153.run_v06_acceptance()
        assert acc.snapshot_id.startswith("v1153-acc-")


# ============================================================================
# V0.6 V0.5 dim 真测
# ============================================================================


class TestV1153V05Dims:
    def test_cross_domain_locked(self):
        v = v1153._measure_v05_cross_domain()
        assert v == 1.0

    def test_vcp_4_locked(self):
        v = v1153._measure_v05_vcp_4()
        assert abs(v - 0.9588) < 1e-9

    def test_eternal_identity_locked(self):
        v = v1153._measure_v05_eternal_identity()
        assert abs(v - 0.8441) < 1e-9

    def test_capabilities_fallback(self):
        # 真调 V1144, 应有 fallback 值
        v = v1153._measure_v05_capabilities()
        assert 0.0 <= v <= 1.0


# ============================================================================
# V0.6 新 4 dim 真测
# ============================================================================


class TestV1153V06NewDims:
    def test_llm_bridge_measures(self):
        v = v1153._measure_llm_bridge()
        # 应该 > 0 (V1152 真存在 + 真跑 benchmark)
        assert v > 0.0
        assert v <= 1.0

    def test_multi_agent_dag_measures(self):
        v = v1153._measure_multi_agent_dag()
        # 应该 > 0 (V1149 真存在 + 真跑)
        assert v > 0.0
        assert v <= 1.0

    def test_vcp_real_run_measures(self):
        v = v1153._measure_vcp_real_run()
        # 应该 > 0 (artifact 存在 + 5 repos 真跑)
        assert v > 0.0
        assert v <= 1.0

    def test_vcp_deep_read_measures(self):
        # 这个可能因为 deep_read_repo 调用而失败/耗时
        v = v1153._measure_vcp_deep_read()
        assert 0.0 <= v <= 1.0


# ============================================================================
# Render
# ============================================================================


class TestV1153RenderMarkdown:
    def test_render_v06_md_has_spec(self):
        spec = v1153.measure_v06_spec()
        md = v1153.render_v06_md(spec)
        assert "ASI V0.6" in md
        assert "21 dim" in md or "21" in md
        assert "Formula" in md or "ASI_V06" in md
        assert "0.9800" in md  # north star

    def test_render_v06_md_with_acceptance(self):
        spec = v1153.measure_v06_spec()
        acc = v1153.run_v06_acceptance()
        md = v1153.render_v06_md(spec, acc)
        assert "Acceptance" in md
        assert "5/5" in md or "n_pass" in md


# ============================================================================
# To_dict
# ============================================================================


class TestV1153ToDict:
    def test_dim_result_to_dict(self):
        spec = v1153.measure_v06_spec()
        r = spec.dim_results[0]
        d = r.to_dict()
        assert "dim" in d
        assert "weight" in d
        assert "value" in d
        assert "status" in d

    def test_spec_to_dict_rounded(self):
        spec = v1153.measure_v06_spec()
        d = spec.to_dict()
        assert isinstance(d["asi_v06_score"], float)
        assert isinstance(d["gap"], float)
        assert isinstance(d["dim_results"], list)

    def test_acceptance_to_dict(self):
        acc = v1153.run_v06_acceptance()
        d = acc.to_dict()
        assert "n_tests" in d
        assert "n_pass" in d
        assert "n_fail" in d


# ============================================================================
# V3 Philosophy guards
# ============================================================================


class TestV1153Guards:
    """V1153 V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_v06_is_not_asi_grade(self):
        # V0.6 是 spec, 不是结果
        # asi_v06_score 是计算结果, 但不等于 ASI 等级 (主 22:33 北极星)
        spec = v1153.measure_v06_spec()
        assert spec.asi_v06_score < 0.98  # 我们还没达到

    def test_weights_are_locked(self):
        # 权重 = 1.0 LOCKED (主 22:33 终极授权)
        assert abs(sum(v1153.V06_DIM_WEIGHTS.values()) - 1.0) < 1e-9

    def test_north_star_locked(self):
        # 北极星 = 0.9800 LOCKED (主 22:33)
        assert v1153.ASI_NORTH_STAR == 0.9800