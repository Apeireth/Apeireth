"""Tests for V1077 ASI V0.4 Full-Dimension Real Measurement Framework.
主 17:43 实事求是 + 主 23:44 干到底.
"""
from __future__ import annotations
import json
import pytest
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from apeireth.v1077_asi_v04_full_measurement import (
    V1077_VERSION,
    V04_WEIGHTS,
    V04_DIM_ORDER,
    REFERENCES,
    DimensionSpec,
    DimensionRegistry,
    MeasurementRunner,
    FullMeasurementAggregator,
    DimensionResult,
    FullMeasurementResult,
    V04WeightRecalibrator,
    V04ScoreComputer,
    RealProductionValidator,
    V04ReportGenerator,
    ASIProductionIntegrationBridge,
    V3PhilosophyGuard,
)


class TestV1077Imports:
    """Test imports & basic structure."""

    def test_version(self):
        assert V1077_VERSION == "0.1.0"

    def test_weights_sum_to_one(self):
        assert abs(sum(V04_WEIGHTS.values()) - 1.0) < 1e-9

    def test_dim_order_matches_weights(self):
        assert set(V04_DIM_ORDER) == set(V04_WEIGHTS.keys())

    def test_dim_order_length(self):
        assert len(V04_DIM_ORDER) == 17

    def test_references_nonempty(self):
        assert len(REFERENCES) >= 11
        assert all("id" in r and "title" in r for r in REFERENCES)


class TestDimensionRegistry:
    """Test DimensionRegistry."""

    def test_n_dims(self):
        r = DimensionRegistry()
        assert r.n_dims() == 17

    def test_all_specs(self):
        r = DimensionRegistry()
        specs = r.all_specs()
        assert len(specs) == 17
        for s in specs:
            assert isinstance(s, DimensionSpec)
            assert s.name
            assert s.weight >= 0  # rubric_open can be 0 by design
            assert s.module_id
            assert s.measurement_kind

    def test_get_known_dim(self):
        r = DimensionRegistry()
        s = r.get("cognitive_core")
        assert s is not None
        assert s.module_id == "V1061"

    def test_get_unknown_dim(self):
        r = DimensionRegistry()
        s = r.get("doesnt_exist")
        assert s is None


class TestMeasurementRunner:
    """Test MeasurementRunner."""

    def test_phi_proxy(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        result = runner.measure("phi_proxy")
        assert "score" in result
        assert 0.0 <= result["score"] <= 1.0

    def test_capabilities(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        result = runner.measure("capabilities")
        assert result["score"] > 0
        assert result["score"] <= 1.0

    def test_scientific_method(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        result = runner.measure("scientific_method")
        assert result["score"] > 0  # V1070 bridge_measure = 1.0

    def test_unknown_dim(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        result = runner.measure("doesnt_exist")
        assert result["score"] == 0.0
        assert "error" in result

    def test_quick_score_module(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        # V1065 has quick_score() — should work
        result = runner.measure("self_organizing_core")
        assert result["score"] > 0
        # error key may be absent on success
        assert result.get("error") is None

    def test_quick_score_with_build(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        # V1062 has build_world_model() + quick_score()
        result = runner.measure("world_model")
        assert result["score"] > 0

    def test_bridge_call_module(self):
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        # V1069 has v1069_bridge_measure()
        result = runner.measure("reinforcement_learning")
        assert result["score"] > 0

    def test_open_rubric_score_filled(self):
        """R12 fix: rubric_open 0.0 → > 0 via V36 HQB 4-dim real measure on V1003 V4."""
        r = DimensionRegistry()
        runner = MeasurementRunner(r)
        result = runner.measure("rubric_open")
        assert result["score"] > 0.0, f"rubric_open must be > 0, got {result}"
        assert 0.0 <= result["score"] <= 1.0
        # 真测 V36 HQB 4 维 应有 raw.method 标识
        assert result.get("raw", {}).get("method") == "v36_hqb_4dim_v1003_v4"
        # V1003 提供 7 哲学问题, 全 anchor + 全 refs
        raw = result["raw"]
        assert raw["n_answers"] >= 7
        assert raw["sc"] == 1.0  # 7 答案全 anchor
        assert raw["nr"] == 1.0  # 7 key 唯一 (>= 7/7)
        assert raw["ev"] > 0.0   # 答案长度 > 0
        assert raw["cdt"] > 0.0  # references > 0


class TestFullMeasurementAggregator:
    """Test FullMeasurementAggregator."""

    def test_aggregate_basic(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        assert isinstance(result, FullMeasurementResult)
        assert 0.0 <= result.v04_score <= 1.0
        assert result.n_dims_measured == 17
        assert result.n_dims_filled >= 13  # at least 13/17 should be filled
        assert result.runtime_ms > 0

    def test_aggregate_all_17_filled_r12_fix(self):
        """R12 fix: V1077 dashboard dim 缺口闭合. 17/17 维度都必须 score > 0."""
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        assert result.n_dims_filled == 17, (
            f"n_dims_filled must be 17/17 after R12 fix, got {result.n_dims_filled}/17. "
            f"zero-score dims: {[n for n, s in result.dim_breakdown.items() if s <= 0.0]}"
        )
        # rubric_open 必须是 17 之一
        assert "rubric_open" in result.dim_breakdown
        assert result.dim_breakdown["rubric_open"] > 0.0
        # 权重必须 sum=1.0 + rubric_open 有非零权重
        assert abs(result.total_weight - 1.0) < 1e-9
        weights = V04_WEIGHTS
        assert weights["rubric_open"] > 0.0
        assert weights["eternal_identity"] > 0.0

    def test_aggregate_weights_sum(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        assert abs(result.total_weight - 1.0) < 1e-9

    def test_dim_breakdown_keys(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        assert set(result.dim_breakdown.keys()) == set(V04_DIM_ORDER)

    def test_philosophy_guard(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        assert result.philosophy_guard_ok

    def test_dim_results_quality(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        for name, dr in result.dim_results.items():
            assert isinstance(dr, DimensionResult)
            assert dr.name == name
            assert 0.0 <= dr.score <= 1.0


class TestV04WeightRecalibrator:
    """Test V04WeightRecalibrator."""

    def test_no_failed_dims(self):
        rec = V04WeightRecalibrator()
        weights = rec.recompute_weights({n: 0.5 for n in V04_DIM_ORDER})
        assert abs(sum(weights.values()) - 1.0) < 1e-9

    def test_all_failed(self):
        rec = V04WeightRecalibrator()
        weights = rec.recompute_weights({n: 0.0 for n in V04_DIM_ORDER})
        # With all failed, weights stay the same
        assert abs(sum(weights.values()) - 1.0) < 1e-9

    def test_some_failed(self):
        rec = V04WeightRecalibrator()
        bd = {n: 0.5 for n in V04_DIM_ORDER}
        bd["cognitive_core"] = 0.0  # failed
        weights = rec.recompute_weights(bd)
        # Weights should still sum to 1.0
        assert abs(sum(weights.values()) - 1.0) < 1e-9
        # Failed dim weight should be 0
        assert weights["cognitive_core"] == 0.0


class TestV04ScoreComputer:
    """Test V04ScoreComputer."""

    def test_compute_basic(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        scorer = V04ScoreComputer()
        info = scorer.compute(result)
        assert "v04_score" in info
        assert 0.0 <= info["v04_score"] <= 1.0
        assert info["n_dims_filled"] >= 13
        assert abs(sum(info["weights_used"].values()) - 1.0) < 1e-9


class TestRealProductionValidator:
    """Test RealProductionValidator."""

    def test_validate(self):
        v = RealProductionValidator()
        result = v.validate()
        assert "deploy_dir" in result
        assert "exists" in result
        assert "all_present" in result


class TestV04ReportGenerator:
    """Test V04ReportGenerator."""

    def test_generate_markdown(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        gen = V04ReportGenerator()
        scorer = V04ScoreComputer()
        score_info = scorer.compute(result)
        md = gen.generate(result, score_info["v04_score"], score_info["weights_used"])
        assert "# ASI V0.4 Full-Dimension Measurement Report" in md
        assert "V0.4 Score" in md
        assert "哲学守门" in md or "philosophy" in md.lower()
        assert "17" in md  # 17 dimensions


class TestASIProductionIntegrationBridge:
    """Test ASIProductionIntegrationBridge end-to-end."""

    def test_run_full(self):
        bridge = ASIProductionIntegrationBridge()
        result = bridge.run_full()
        assert "v04_score" in result
        assert result["v04_score"] > 0
        assert result["n_dims_filled"] >= 13
        assert "report_markdown" in result
        assert "deploy_validation" in result
        assert "philosophy_guard_ok" in result


class TestV3PhilosophyGuard:
    """Test V3PhilosophyGuard."""

    def test_check_all(self):
        agg = FullMeasurementAggregator()
        result = agg.aggregate()
        guard = V3PhilosophyGuard()
        checks = guard.check_all(result, result.v04_score)
        assert len(checks) == 6
        assert all(v is True for v in checks.values())

    def test_explain(self):
        guard = V3PhilosophyGuard()
        s = guard.explain()
        assert "不假装 measurement" in s or "measurement_is_not_asi" in s
        assert len(s) > 100


class TestEndToEndCLI:
    """Test CLI execution."""

    def test_main_returns_zero(self):
        from apeireth.v1077_asi_v04_full_measurement import main
        rc = main(["--quiet"])
        assert rc == 0

    def test_main_with_json(self, capsys):
        from apeireth.v1077_asi_v04_full_measurement import main
        rc = main(["--quiet", "--json"])
        assert rc == 0
        captured = capsys.readouterr()
        # JSON output should appear in stdout
        assert "v04_score" in captured.out or "v04" in captured.out.lower()

    def test_main_with_report(self, tmp_path, monkeypatch):
        # Use --report to write Markdown
        from apeireth.v1077_asi_v04_full_measurement import main
        rc = main(["--quiet", "--report"])
        assert rc == 0
        # Check reports dir exists
        reports_dir = Path(__file__).resolve().parent.parent / "reports"
        report_path = reports_dir / "v1077_report.md"
        if report_path.exists():
            content = report_path.read_text(encoding="utf-8")
            assert "ASI V0.4" in content