"""Test V1166 — ASI real_llm_benchmark V0.6 真补 (5 sub-dim 真测).

主 17:43 实事求是: 测试覆盖 constants / dataclasses / helpers / _measure_*
with mocked V1133 reports (不实际触发 HTTP benchmark).
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import pytest


class TestV1166Constants:
    def test_version_present(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import V1166_VERSION
        assert V1166_VERSION == "0.1.0"

    def test_dim_version(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import V1166_DIM_VERSION
        assert V1166_DIM_VERSION == "0.6"

    def test_subdim_names_locked(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import V1166_SUBDIM_NAMES
        assert V1166_SUBDIM_NAMES == (
            "api_key_resolution_real",
            "endpoint_reachability_real",
            "sample_coverage_real",
            "pass_rate_real",
            "latency_distribution_real",
        )

    def test_v1133_baseline_constants(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import (
            V1133_BASELINE_PASS_RATE, V1133_BENCHMARK_SAMPLES_TOTAL,
            TARGET_REAL_LLM_BENCHMARK_V06, V1133_REPORT_FIELDS,
        )
        assert V1133_BASELINE_PASS_RATE == 0.95
        assert V1133_BENCHMARK_SAMPLES_TOTAL == 22
        assert TARGET_REAL_LLM_BENCHMARK_V06 == 0.85
        assert "pass_rate" in V1133_REPORT_FIELDS
        assert "p95_latency_ms" in V1133_REPORT_FIELDS

    def test_artifact_dir_default(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import DEFAULT_ARTIFACT_DIR
        assert DEFAULT_ARTIFACT_DIR == "artifacts"

    def test_threshold_constants_present(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import (
            _N_SAMPLES_FULL, _N_SAMPLES_MIN, _PASS_RATE_MIN,
            _P50_LATENCY_MAX_MS, _P95_LATENCY_MAX_MS,
        )
        assert _N_SAMPLES_FULL == 22
        assert _N_SAMPLES_MIN == 8
        assert _PASS_RATE_MIN == 0.5
        assert _P50_LATENCY_MAX_MS > 0.0
        assert _P95_LATENCY_MAX_MS > _P50_LATENCY_MAX_MS


class TestSafeHelpers:
    def test_safe_import_returns_none_on_missing(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _safe_import
        assert _safe_import("nonexistent.module.xyz") is None

    def test_safe_import_returns_module_on_present(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _safe_import
        mod = _safe_import("apeireth.v1166_asi_real_llm_benchmark_v06_real_measure")
        assert mod is not None

    def test_call_safely_with_none(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _call_safely
        ok, r = _call_safely(None, 1, 2)
        assert ok is False
        assert r is None

    def test_call_safely_with_callable(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _call_safely
        ok, r = _call_safely(lambda x: x * 2, 3)
        assert ok is True
        assert r == 6

    def test_call_safely_with_raising(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _call_safely
        def boom():
            raise RuntimeError("x")
        ok, r = _call_safely(boom)
        assert ok is False
        assert r is None

    def test_attr_first_picks_first_existing(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _attr_first
        class Obj:
            a = 1
        assert _attr_first(Obj, ["nope", "a", "z"]) == 1

    def test_attr_first_returns_none_on_missing(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _attr_first
        assert _attr_first(object, ["nope", "nada"]) is None

    def test_safe_field_defaults_when_missing(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _safe_field
        class O: pass
        assert _safe_field(O(), "x", 42) == 42

    def test_safe_field_returns_value(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _safe_field
        class O:
            x = 5
        assert _safe_field(O(), "x", 0) == 5

    def test_safe_callable_field_with_property(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _safe_callable_field
        class O:
            @property
            def x(self):
                return 7
        assert _safe_callable_field(O(), "x", 0.0) == 7


class TestSubDimEvidence:
    def test_default_init(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import SubDimEvidence
        e = SubDimEvidence(name="x", score=0.5)
        assert e.name == "x"
        assert e.score == 0.5
        assert e.checks == {}
        assert e.notes == []
        assert e.raw == {}

    def test_to_dict(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import SubDimEvidence
        e = SubDimEvidence(name="x", score=0.5, checks={"a": True}, notes=["n1"], raw={"k": 1})
        d = e.to_dict()
        assert d["name"] == "x"
        assert d["score"] == 0.5
        assert d["checks"] == {"a": True}
        assert d["notes"] == ["n1"]
        assert d["raw"] == {"k": 1}


class TestRealLLMBenchmarkReport:
    def test_default_init_generates_snapshot_id(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import RealLLMBenchmarkReport
        r = RealLLMBenchmarkReport()
        assert r.snapshot_id.startswith("v1166-")
        assert r.version == "0.1.0"
        assert r.dim_version == "0.6"
        assert r.total == 0.0
        assert r.n_subdims_total == 5

    def test_summary_line_format(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import RealLLMBenchmarkReport
        r = RealLLMBenchmarkReport(
            total=0.5, n_subdims_passed=2, n_subdims_partial=2, n_subdims_missing=1,
            v1133_pass_rate=0.95, v1133_api_key_present=True,
        )
        line = r.summary_line()
        assert "total=0.5000" in line
        assert "V1133 baseline 0.9500" in line
        assert "2 pass / 2 partial / 1 missing" in line
        assert "api_key_present=True" in line

    def test_to_from_dict_roundtrip(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import (
            RealLLMBenchmarkReport, SubDimEvidence,
        )
        r = RealLLMBenchmarkReport(
            total=0.7, snapshot_id="v1166-test",
            v1133_benchmark_id="bench-123", v1133_endpoint="https://api.test/v1",
            v1133_model="test-model", v1133_pass_rate=0.9, v1133_api_key_present=True,
            v1133_api_key_source="env:OPENAI_API_KEY",
        )
        r.sub_dim_scores = {"api_key_resolution_real": 0.9}
        r.sub_dim_evidence["api_key_resolution_real"] = SubDimEvidence(
            name="api_key_resolution_real", score=0.9, checks={"k": True}, raw={"v": 1},
        )
        r2 = RealLLMBenchmarkReport.from_dict(r.to_dict())
        assert r2.snapshot_id == "v1166-test"
        assert r2.total == 0.7
        assert r2.sub_dim_scores["api_key_resolution_real"] == 0.9
        assert r2.sub_dim_evidence["api_key_resolution_real"].score == 0.9
        assert r2.sub_dim_evidence["api_key_resolution_real"].checks == {"k": True}
        assert r2.v1133_benchmark_id == "bench-123"
        assert r2.v1133_api_key_source == "env:OPENAI_API_KEY"

    def test_from_dict_handles_missing_evidence(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import RealLLMBenchmarkReport
        r = RealLLMBenchmarkReport.from_dict({"snapshot_id": "x", "total": 0.3})
        assert r.snapshot_id == "x"
        assert r.total == 0.3
        assert r.sub_dim_evidence == {}


class TestMeasureApiKeyResolution:
    """L1 — api_key_resolution_real 真测."""

    def test_present_env(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_api_key_resolution
        report = _make_fake_report(api_key_present=True, api_key_source="env:OPENAI_API_KEY")
        score, ev = _measure_api_key_resolution(report)
        assert 0.0 <= score <= 1.0
        assert ev.score == score

    def test_present_file(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_api_key_resolution
        report = _make_fake_report(api_key_present=True, api_key_source="file:~/.openai/key")
        score, _ = _measure_api_key_resolution(report)
        assert score > 0.5

    def test_missing(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_api_key_resolution
        report = _make_fake_report(api_key_present=False, api_key_source="none")
        score, ev = _measure_api_key_resolution(report)
        assert score < 0.1  # 允许极小 check_bonus (主 17:43 实事求是 — 不假装设硬 0)


class TestMeasureEndpointReachability:
    """L2 — endpoint_reachability_real 真测."""

    def test_full_coverage(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_endpoint_reachability
        report = _make_fake_report(n_samples=22, n_error=0, endpoint="https://api.test/v1")
        score, ev = _measure_endpoint_reachability(report)
        assert 0.0 <= score <= 1.0

    def test_some_errors(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_endpoint_reachability
        report = _make_fake_report(n_samples=22, n_error=5, endpoint="https://api.test/v1")
        score, _ = _measure_endpoint_reachability(report)
        assert score < 1.0

    def test_no_endpoint(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_endpoint_reachability
        report = _make_fake_report(n_samples=22, n_error=22, endpoint="")
        score, _ = _measure_endpoint_reachability(report)
        assert score < 0.1


class TestMeasureSampleCoverage:
    """L3 — sample_coverage_real 真测."""

    def test_full_22(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_sample_coverage
        report = _make_fake_report(n_samples=22, n_passed=22, n_failed=0, n_error=0)
        score, _ = _measure_sample_coverage(report)
        assert score >= 0.8

    def test_minimum_8(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_sample_coverage
        report = _make_fake_report(n_samples=8, n_passed=4, n_failed=4, n_error=0)
        score, _ = _measure_sample_coverage(report)
        assert 0.0 < score < 1.0

    def test_zero(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_sample_coverage
        report = _make_fake_report(n_samples=0, n_passed=0, n_failed=0, n_error=0)
        score, _ = _measure_sample_coverage(report)
        assert score < 0.1


class TestMeasurePassRate:
    """L4 — pass_rate_real 真测."""

    def test_high_pass_rate(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_pass_rate
        report = _make_fake_report(pass_rate=0.95, p50_latency_ms=1000.0, n_passed=21)
        score, _ = _measure_pass_rate(report)
        assert score >= 0.9

    def test_mid_pass_rate(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_pass_rate
        report = _make_fake_report(pass_rate=0.6, p50_latency_ms=2000.0, n_passed=13)
        score, _ = _measure_pass_rate(report)
        assert 0.5 < score < 0.95

    def test_no_pass(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_pass_rate
        report = _make_fake_report(pass_rate=0.0, p50_latency_ms=0.0, n_passed=0)
        score, _ = _measure_pass_rate(report)
        assert score < 0.1


class TestMeasureLatencyDistribution:
    """L5 — latency_distribution_real 真测."""

    def test_good_latency(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_latency_distribution
        report = _make_fake_report(p50_latency_ms=1500.0, p95_latency_ms=3000.0, latencies_ms=[1500]*20)
        score, _ = _measure_latency_distribution(report)
        assert score >= 0.6

    def test_no_latency(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_latency_distribution
        report = _make_fake_report(p50_latency_ms=0.0, p95_latency_ms=0.0, latencies_ms=[])
        score, _ = _measure_latency_distribution(report)
        assert score < 0.1

    def test_high_latency(self):
        from apeireth.v1166_asi_real_llm_benchmark_v06_real_measure import _measure_latency_distribution
        report = _make_fake_report(p50_latency_ms=50000.0, p95_latency_ms=90000.0, latencies_ms=[50000]*20)
        score, _ = _measure_latency_distribution(report)
        assert score < 0.5


class TestMeasureFullAggregation:
    """主入口聚合 — with mocked V1133 report (no HTTP)."""

    def test_aggregate_5_subdim(self, monkeypatch):
        from apeireth import v1166_asi_real_llm_benchmark_v06_real_measure as mod

        # Patch _get_v1133_report to return a fake report
        fake_rep = _make_fake_report(
            api_key_present=True, api_key_source="env:OPENAI_API_KEY",
            n_samples=22, n_passed=20, n_failed=2, n_error=0, n_http_forbidden=0,
            endpoint="https://api.test/v1", pass_rate=0.91,
            p50_latency_ms=1500.0, p95_latency_ms=3000.0,
            latencies_ms=[1500]*22, benchmark_id="bench-x", model="gpt-test",
        )
        def fake_get(max_samples=None, timeout=30.0):
            return True, fake_rep, "ok"
        monkeypatch.setattr(mod, "_get_v1133_report", fake_get)

        rep = mod.measure_real_llm_benchmark_full(write_artifact=False)
        assert 0.0 < rep.total <= 1.0
        assert len(rep.sub_dim_scores) == 5
        assert all(name in rep.sub_dim_scores for name in mod.V1166_SUBDIM_NAMES)

    def test_unavailable_v1133_returns_zero(self, monkeypatch):
        from apeireth import v1166_asi_real_llm_benchmark_v06_real_measure as mod

        def fake_get(max_samples=None, timeout=30.0):
            return False, None, "v1133_module_not_found"
        monkeypatch.setattr(mod, "_get_v1133_report", fake_get)

        rep = mod.measure_real_llm_benchmark_full(write_artifact=False)
        assert rep.total == 0.0
        assert any("V1133 unavailable" in n for n in rep.notes)


# ============================================================================
# Helpers — fake V1133 report
# ============================================================================


def _make_fake_report(
    api_key_present: bool = False,
    api_key_source: str = "none",
    benchmark_id: str = "bench-fake",
    model: str = "gpt-fake",
    endpoint: str = "https://api.fake/v1",
    n_samples: int = 22,
    n_passed: int = 0,
    n_failed: int = 0,
    n_error: int = 0,
    n_http_forbidden: int = 0,
    pass_rate: float = 0.0,
    p50_latency_ms: float = 0.0,
    p95_latency_ms: float = 0.0,
    latencies_ms: list = None,
) -> object:
    """Build a fake V1133BenchmarkReport-like object (duck-typed)."""

    class _FakeReport:
        pass

    rep = _FakeReport()
    rep.api_key_present = api_key_present
    rep.api_key_source = api_key_source
    rep.benchmark_id = benchmark_id
    rep.model = model
    rep.endpoint = endpoint
    rep.n_samples = n_samples
    rep.n_passed = n_passed
    rep.n_failed = n_failed
    rep.n_error = n_error
    rep.n_http_forbidden = n_http_forbidden
    rep.pass_rate = pass_rate
    rep.p50_latency_ms = p50_latency_ms
    rep.p95_latency_ms = p95_latency_ms
    rep.latencies_ms = latencies_ms if latencies_ms is not None else []
    rep.started_at = 0.0
    return rep
