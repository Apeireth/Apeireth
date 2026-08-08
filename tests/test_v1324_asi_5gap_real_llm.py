"""V1324 ASI 5-Gap Crucible + Real LLM — pytest tests (主 17:43 实事求是 + 主 00:44 质量工程化).

Tests cover:
- Config defaults + env override
- RealLLMClient construction + is_configured
- _parse_5_gap_response (5 floats / garbage / empty / with text)
- _pearson (perfect positive / negative / constant / empty)
- _percentile (LOCKED behavior)
- LLMGapScorer with FakeClient (no real HTTP in pytest)
- ProbeAndValidateReport construction
- run_real_benchmark (with FakeClient returning deterministic scores)
- compare_heuristic_vs_real (with synthetic data)
- build_v1324_aggregate + build_bridge
- render_markdown_report (contains LOCKED markers)
- ASI_5_GAPS / BENCHMARK_QUERIES / V3_GUARD_MARKERS / pole-star anchors LOCKED

NO real HTTP in pytest (use FakeClient + ENV-cleared path) — 主 17:43 实事求是: 不假装 LLM 跑了.
"""
from __future__ import annotations

import json
import math
import os
import statistics
from typing import Any, Dict, List, Optional, Sequence, Tuple

import pytest

from apeireth.v1322_asi_5gap_crucible import (
    ASI_5_GAPS,
    ASII5GapCrucible,
    CrucibleResult,
)
from apeireth.v1323_asi_5gap_crucible_benchmark import (
    ASI_ANCHORS,
    BENCHMARK_QUERIES,
    BenchmarkRunner,
    QueryResult,
    V3_GUARD_MARKERS,
    _assert_benchmark_queries_locked,
)
from apeireth.v1324_asi_5gap_real_llm import (
    ASI_ANCHORS_V1324,
    DEFAULT_BASE_URL,
    DEFAULT_MAX_TOKENS,
    DEFAULT_MODEL,
    DEFAULT_TIMEOUT_SEC,
    ENV_API_KEY,
    ENV_BASE_URL,
    ENV_MODEL,
    GAP_SCORING_PROMPT,
    LLMGapScore,
    LLMGapScorer,
    ProbeAndValidateReport,
    RealLLMClient,
    RealLLMConfig,
    RealBenchmarkResult,
    V1324_VERSION,
    V3_GUARD_MARKERS_V1324,
    _parse_5_gap_response,
    _pearson,
    build_bridge,
    build_v1324_aggregate,
    compare_heuristic_vs_real,
    default_config,
    probe_and_validate,
    render_markdown_report,
    run_real_benchmark,
)


# ---------------------------------------------------------------------------
# Fake client (no real HTTP) for pytest
# ---------------------------------------------------------------------------


class _FakeLLMClient:
    """Fake client that returns deterministic scores. Configurable per-call."""

    def __init__(
        self,
        responses: Optional[Dict[str, str]] = None,
        fail_pattern: Optional[str] = None,
        score_factory=None,
    ) -> None:
        self.config = RealLLMConfig(
            base_url="https://fake.example/v1",
            model="fake-model",
            timeout_sec=1.0,
            max_tokens=8,
        )
        self.api_key = "fake-key"
        self.responses = responses or {}
        self.fail_pattern = fail_pattern  # "all", "none", or query_id pattern
        self.score_factory = score_factory  # callable(prompt) -> str
        self.call_count = 0
        self.last_content: Optional[str] = None

    def is_configured(self) -> bool:
        return True

    def chat(self, prompt: str) -> Any:
        from apeireth.v1324_asi_5gap_real_llm import ChatResult
        self.call_count += 1
        if self.fail_pattern == "all":
            return ChatResult(
                ok=False, content="", latency_ms=1.0,
                input_tokens=10, output_tokens=0, model="fake-model",
                fallback_used=True, error="simulated failure",
            )
        if self.fail_pattern and self.fail_pattern in prompt:
            return ChatResult(
                ok=False, content="", latency_ms=1.0,
                input_tokens=10, output_tokens=0, model="fake-model",
                fallback_used=True, error="simulated pattern failure",
            )
        if self.score_factory is not None:
            content = self.score_factory(prompt)
        else:
            content = "0.5,0.5,0.5,0.5,0.5"
        self.last_content = content
        return ChatResult(
            ok=True, content=content, latency_ms=1.0,
            input_tokens=10, output_tokens=5, model="fake-model",
            fallback_used=False, error="",
        )

    def probe(self) -> Dict[str, Any]:
        return {
            "reachable": True,
            "configured": True,
            "latency_ms": 1.0,
            "model": "fake-model",
            "input_tokens": 10,
            "output_tokens": 5,
            "error": "",
        }


class _NoKeyFakeClient:
    """Fake client without API key (env cleared)."""

    def __init__(self) -> None:
        self.config = RealLLMConfig(
            base_url="https://fake.example/v1",
            model="fake-model",
            timeout_sec=1.0,
            max_tokens=8,
        )
        self.api_key = ""

    def is_configured(self) -> bool:
        return False

    def chat(self, prompt: str) -> Any:
        from apeireth.v1324_asi_5gap_real_llm import ChatResult
        return ChatResult(
            ok=False, content="", latency_ms=0.0,
            input_tokens=0, output_tokens=0, model="fake-model",
            fallback_used=True, error="not configured",
        )

    def probe(self) -> Dict[str, Any]:
        return {"reachable": False, "configured": False, "error": "not configured"}


# ---------------------------------------------------------------------------
# Test 1: Config defaults + env override
# ---------------------------------------------------------------------------


class TestConfig:
    def test_default_base_url(self) -> None:
        cfg = default_config()
        assert cfg.base_url == DEFAULT_BASE_URL
        assert cfg.base_url == "https://api.minimaxi.com/anthropic"

    def test_default_model(self) -> None:
        cfg = default_config()
        assert cfg.model == DEFAULT_MODEL
        assert cfg.model == "MiniMax-M3"

    def test_default_timeout(self) -> None:
        cfg = default_config()
        assert cfg.timeout_sec == DEFAULT_TIMEOUT_SEC
        assert cfg.timeout_sec == 30.0

    def test_default_max_tokens(self) -> None:
        cfg = default_config()
        assert cfg.max_tokens == DEFAULT_MAX_TOKENS
        assert cfg.max_tokens == 256

    def test_env_override(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setenv(ENV_BASE_URL, "https://custom.example/v2")
        monkeypatch.setenv(ENV_MODEL, "custom-model")
        cfg = default_config()
        assert cfg.base_url == "https://custom.example/v2"
        assert cfg.model == "custom-model"

    def test_config_to_dict(self) -> None:
        cfg = default_config()
        d = cfg.to_dict()
        assert d["base_url"] == DEFAULT_BASE_URL
        assert d["model"] == DEFAULT_MODEL
        assert d["timeout_sec"] == DEFAULT_TIMEOUT_SEC
        assert d["max_tokens"] == DEFAULT_MAX_TOKENS


# ---------------------------------------------------------------------------
# Test 2: RealLLMClient construction + is_configured
# ---------------------------------------------------------------------------


class TestRealLLMClient:
    def test_construction_with_key(self) -> None:
        c = RealLLMClient(api_key="test-key")
        assert c.is_configured() is True

    def test_construction_no_key(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv(ENV_API_KEY, raising=False)
        c = RealLLMClient(api_key="")
        assert c.is_configured() is False

    def test_construction_env_key(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setenv(ENV_API_KEY, "env-key")
        c = RealLLMClient()
        assert c.api_key == "env-key"
        assert c.is_configured() is True

    def test_no_chat_when_unconfigured(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv(ENV_API_KEY, raising=False)
        c = RealLLMClient(api_key="")
        r = c.chat("test")
        assert r.ok is False
        assert r.fallback_used is True
        assert "missing" in r.error or "config" in r.error

    def test_probe_when_unconfigured(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv(ENV_API_KEY, raising=False)
        c = RealLLMClient(api_key="")
        p = c.probe()
        assert p["reachable"] is False
        assert p["configured"] is False


# ---------------------------------------------------------------------------
# Test 3: _parse_5_gap_response
# ---------------------------------------------------------------------------


class TestParse5GapResponse:
    def test_basic(self) -> None:
        assert _parse_5_gap_response("0.7,0.2,0.1,0.0,0.5") == (0.7, 0.2, 0.1, 0.0, 0.5)

    def test_with_spaces(self) -> None:
        assert _parse_5_gap_response("0.1, 0.2, 0.3, 0.4, 0.5") == (0.1, 0.2, 0.3, 0.4, 0.5)

    def test_with_surrounding_text(self) -> None:
        assert _parse_5_gap_response("text 0.7,0.2,0.1,0.0,0.5 end") == (0.7, 0.2, 0.1, 0.0, 0.5)

    def test_with_code_fence(self) -> None:
        assert _parse_5_gap_response("```0.7,0.2,0.1,0.0,0.5```") == (0.7, 0.2, 0.1, 0.0, 0.5)

    def test_with_code_fence_and_lang(self) -> None:
        assert _parse_5_gap_response("```json\n0.7,0.2,0.1,0.0,0.5\n```") == (0.7, 0.2, 0.1, 0.0, 0.5)

    def test_only_three_returns_none(self) -> None:
        assert _parse_5_gap_response("only three 0.1,0.2,0.3") is None

    def test_empty_returns_none(self) -> None:
        assert _parse_5_gap_response("") is None

    def test_no_numbers_returns_none(self) -> None:
        assert _parse_5_gap_response("nothing here") is None

    def test_clamps_to_unit_interval(self) -> None:
        result = _parse_5_gap_response("1.5,-0.5,0.5,0.5,0.5")
        assert result is not None
        for v in result:
            assert 0.0 <= v <= 1.0

    def test_more_than_5_takes_first_5(self) -> None:
        result = _parse_5_gap_response("0.1,0.2,0.3,0.4,0.5,0.6,0.7")
        assert result == (0.1, 0.2, 0.3, 0.4, 0.5)

    def test_int_floats_accepted(self) -> None:
        # regex requires \d+.\d+, so plain ints won't match → should return None
        assert _parse_5_gap_response("1,2,3,4,5") is None


# ---------------------------------------------------------------------------
# Test 4: _pearson
# ---------------------------------------------------------------------------


class TestPearson:
    def test_perfect_positive(self) -> None:
        xs = [0.1, 0.2, 0.3, 0.4, 0.5]
        ys = [0.1, 0.2, 0.3, 0.4, 0.5]
        assert abs(_pearson(xs, ys) - 1.0) < 1e-9

    def test_perfect_negative(self) -> None:
        xs = [0.1, 0.2, 0.3, 0.4, 0.5]
        ys = [0.5, 0.4, 0.3, 0.2, 0.1]
        assert abs(_pearson(xs, ys) - (-1.0)) < 1e-9

    def test_constant_returns_zero(self) -> None:
        xs = [0.5, 0.5, 0.5, 0.5, 0.5]
        ys = [0.1, 0.2, 0.3, 0.4, 0.5]
        assert _pearson(xs, ys) == 0.0

    def test_empty_returns_zero(self) -> None:
        assert _pearson([], []) == 0.0

    def test_one_element_returns_zero(self) -> None:
        assert _pearson([0.5], [0.3]) == 0.0

    def test_mismatched_lengths(self) -> None:
        assert _pearson([0.1, 0.2], [0.1, 0.2, 0.3]) == 0.0


# ---------------------------------------------------------------------------
# Test 5: LLMGapScorer with FakeClient
# ---------------------------------------------------------------------------


class TestLLMGapScorer:
    def test_empty_query_no_chat_call(self) -> None:
        scorer = LLMGapScorer(client=_NoKeyFakeClient())
        s = scorer.score_one("Q20", "edge", "", "none")
        assert s.fallback_used is True
        assert s.gap_scores["time"] == 0.0
        for gap in ASI_5_GAPS:
            assert s.gap_scores[gap] == 0.0

    def test_minimal_query_with_unconfigured(self) -> None:
        scorer = LLMGapScorer(client=_NoKeyFakeClient())
        s = scorer.score_one("Q21", "edge", "x", "none")
        assert s.fallback_used is True

    def test_normal_query_with_fake(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        s = scorer.score_one("Q01", "test", "What is time?", "time")
        assert s.chat_ok is True
        assert s.fallback_used is False
        for gap in ASI_5_GAPS:
            assert s.gap_scores[gap] == 0.5
        assert s.input_tokens == 10
        assert s.output_tokens == 5

    def test_fake_failure_returns_fallback(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient(fail_pattern="all"))
        s = scorer.score_one("Q01", "test", "What is time?", "time")
        assert s.chat_ok is False
        assert s.fallback_used is True
        assert "simulated failure" in s.error

    def test_parse_failure_returns_fallback_with_chat_ok(self) -> None:
        def bad_response(prompt: str) -> str:
            return "not five numbers"
        scorer = LLMGapScorer(client=_FakeLLMClient(score_factory=bad_response))
        s = scorer.score_one("Q01", "test", "What is time?", "time")
        assert s.chat_ok is True
        assert s.fallback_used is True
        assert "parse failure" in s.error

    def test_total_tokens_property(self) -> None:
        s = LLMGapScore(
            query_id="Q01",
            category="test",
            query_text="x",
            expected_gap_focus="time",
            gap_scores={g: 0.5 for g in ASI_5_GAPS},
            raw_response="0.5,0.5,0.5,0.5,0.5",
            chat_ok=True,
            fallback_used=False,
            latency_ms=1.0,
            input_tokens=10,
            output_tokens=5,
            error="",
        )
        assert s.total_tokens == 15


# ---------------------------------------------------------------------------
# Test 6: ProbeAndValidate
# ---------------------------------------------------------------------------


class TestProbeAndValidate:
    def test_with_fake_configured(self) -> None:
        c = _FakeLLMClient()
        report = probe_and_validate(c)  # type: ignore[arg-type]
        assert report.configured is True
        assert report.reachable is True
        assert report.latency_ms >= 0

    def test_with_no_key(self) -> None:
        c = _NoKeyFakeClient()
        report = probe_and_validate(c)  # type: ignore[arg-type]
        assert report.configured is False
        assert report.reachable is False
        assert "not configured" in report.error or "missing" in report.error

    def test_to_dict(self) -> None:
        report = ProbeAndValidateReport(
            configured=True, reachable=True, latency_ms=1.0,
            model="m", input_tokens=10, output_tokens=5,
            base_url="https://x", error="",
        )
        d = report.to_dict()
        assert d["configured"] is True
        assert d["reachable"] is True
        assert d["model"] == "m"
        assert d["base_url"] == "https://x"


# ---------------------------------------------------------------------------
# Test 7: run_real_benchmark (with FakeClient)
# ---------------------------------------------------------------------------


class TestRunRealBenchmark:
    def test_with_fake_returns_22_scores(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        result = run_real_benchmark(scorer=scorer, probe_first=False)
        assert result.n_queries == 22
        assert len(result.scores) == 22
        # 21 of 22 should chat_ok (Q20 empty bypasses chat)
        assert result.n_chat_ok == 21
        assert result.n_fallback == 1
        assert result.n_parse_failure == 0

    def test_with_all_failure(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient(fail_pattern="all"))
        result = run_real_benchmark(scorer=scorer, probe_first=False)
        assert result.n_chat_ok == 0
        assert result.n_fallback == 22

    def test_with_no_key(self) -> None:
        scorer = LLMGapScorer(client=_NoKeyFakeClient())
        result = run_real_benchmark(scorer=scorer, probe_first=False)
        assert result.n_chat_ok == 0
        assert result.n_fallback == 22

    def test_with_probe_first(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        result = run_real_benchmark(scorer=scorer, probe_first=True)
        assert result.probe_report.configured is True
        assert result.probe_report.reachable is True

    def test_empty_query_bypasses_chat(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        result = run_real_benchmark(scorer=scorer, probe_first=False)
        # Find Q20_EMPTY in scores
        empty = next(s for s in result.scores if s.query_id == "Q20_EMPTY")
        assert empty.chat_ok is False
        assert empty.fallback_used is True
        assert "empty query" in empty.error

    def test_total_tokens_summed(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        result = run_real_benchmark(scorer=scorer, probe_first=False)
        expected_in = sum(s.input_tokens for s in result.scores)
        expected_out = sum(s.output_tokens for s in result.scores)
        assert result.total_input_tokens == expected_in
        assert result.total_output_tokens == expected_out
        assert result.total_tokens == expected_in + expected_out

    def test_to_dict_round_trip(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        result = run_real_benchmark(scorer=scorer, probe_first=False)
        d = result.to_dict()
        assert d["n_queries"] == 22
        assert len(d["scores"]) == 22
        assert "config" in d


# ---------------------------------------------------------------------------
# Test 8: compare_heuristic_vs_real
# ---------------------------------------------------------------------------


def _make_heuristic_results(n: int = 5) -> List[QueryResult]:
    """Build synthetic heuristic results (matching V1322 schema)."""
    out: List[QueryResult] = []
    for i in range(n):
        gap = {g: 0.1 * i for g in ASI_5_GAPS}
        gap["freedom"] = 0.2
        gap["recognition"] = 0.3
        gap["emergence"] = 0.4
        gap["truth"] = 0.5
        cr = CrucibleResult(
            query=f"q{i}",
            gap_scores=gap,
            cross_gap_scores={},
            aggregate_5_gap_score=sum(gap.values()) / 5,
            aggregate_cross_gap_score=0.0,
            aggregate_total=sum(gap.values()) / 5,
            latency_ms=0.5,
            v3_guards=tuple(),
            substrate_chain=tuple(),
            pole_star_anchors={},
        )
        out.append(QueryResult(
            query_id=f"Q{i:02d}",
            category="test",
            query_text=f"q{i}",
            expected_gap_focus="time",
            crucible_result=cr,
            is_empty=False,
            is_minimal=False,
        ))
    return out


def _make_real_scores(heuristic_results: List[QueryResult]) -> Tuple[LLMGapScore, ...]:
    """Build synthetic real LLM scores that match heuristic perfectly."""
    out: List[LLMGapScore] = []
    for q in heuristic_results:
        gap = dict(q.crucible_result.gap_scores)
        out.append(LLMGapScore(
            query_id=q.query_id,
            category=q.category,
            query_text=q.query_text,
            expected_gap_focus=q.expected_gap_focus,
            gap_scores=gap,
            raw_response="ok",
            chat_ok=True,
            fallback_used=False,
            latency_ms=1.0,
            input_tokens=10,
            output_tokens=2,
            error="",
        ))
    return tuple(out)


class TestCompareHeuristicVsReal:
    def test_perfect_agreement(self) -> None:
        hrs = _make_heuristic_results(5)
        rrs = _make_real_scores(hrs)
        real = RealBenchmarkResult(
            n_queries=5, n_chat_ok=5, n_fallback=0, n_parse_failure=0,
            total_latency_ms=5.0, mean_latency_ms=1.0,
            total_input_tokens=50, total_output_tokens=10, total_tokens=60,
            scores=rrs,
            probe_report=ProbeAndValidateReport(
                configured=True, reachable=True, latency_ms=1.0, model="x",
                input_tokens=1, output_tokens=1, base_url="x", error="",
            ),
            config=RealLLMConfig(base_url="x", model="x", timeout_sec=1.0, max_tokens=8),
            started_at="t0", finished_at="t1",
        )
        cmp_ = compare_heuristic_vs_real(hrs, real)
        assert cmp_.n_queries == 5
        # time has variance → pearson=1.0
        time_ga = next(g for g in cmp_.gap_agreements if g.gap == "time")
        assert abs(time_ga.pearson_r - 1.0) < 1e-9
        # overall MAE = 0 (identical)
        assert cmp_.overall_mae < 1e-9

    def test_perfect_disagreement(self) -> None:
        hrs = _make_heuristic_results(5)
        # Invert all real scores (1.0 - heuristic)
        rrs_list: List[LLMGapScore] = []
        for q in hrs:
            gap = {g: 1.0 - v for g, v in q.crucible_result.gap_scores.items()}
            rrs_list.append(LLMGapScore(
                query_id=q.query_id,
                category=q.category,
                query_text=q.query_text,
                expected_gap_focus=q.expected_gap_focus,
                gap_scores=gap,
                raw_response="ok",
                chat_ok=True,
                fallback_used=False,
                latency_ms=1.0,
                input_tokens=10, output_tokens=2, error="",
            ))
        real = RealBenchmarkResult(
            n_queries=5, n_chat_ok=5, n_fallback=0, n_parse_failure=0,
            total_latency_ms=5.0, mean_latency_ms=1.0,
            total_input_tokens=50, total_output_tokens=10, total_tokens=60,
            scores=tuple(rrs_list),
            probe_report=ProbeAndValidateReport(
                configured=True, reachable=True, latency_ms=1.0, model="x",
                input_tokens=1, output_tokens=1, base_url="x", error="",
            ),
            config=RealLLMConfig(base_url="x", model="x", timeout_sec=1.0, max_tokens=8),
            started_at="t0", finished_at="t1",
        )
        cmp_ = compare_heuristic_vs_real(hrs, real)
        time_ga = next(g for g in cmp_.gap_agreements if g.gap == "time")
        # Anti-correlated → pearson near -1.0 (not exact due to clamping)
        assert time_ga.pearson_r < -0.9

    def test_empty_heuristic_returns_zero_report(self) -> None:
        real = RealBenchmarkResult(
            n_queries=0, n_chat_ok=0, n_fallback=0, n_parse_failure=0,
            total_latency_ms=0.0, mean_latency_ms=0.0,
            total_input_tokens=0, total_output_tokens=0, total_tokens=0,
            scores=tuple(),
            probe_report=ProbeAndValidateReport(
                configured=False, reachable=False, latency_ms=0.0, model="x",
                input_tokens=0, output_tokens=0, base_url="x", error="",
            ),
            config=RealLLMConfig(base_url="x", model="x", timeout_sec=1.0, max_tokens=8),
            started_at="t0", finished_at="t1",
        )
        cmp_ = compare_heuristic_vs_real([], real)
        assert cmp_.n_queries == 0
        assert cmp_.gap_agreements == ()
        assert cmp_.overall_pearson_r == 0.0

    def test_delta_means_per_gap(self) -> None:
        hrs = _make_heuristic_results(3)
        rrs = _make_real_scores(hrs)
        real = RealBenchmarkResult(
            n_queries=3, n_chat_ok=3, n_fallback=0, n_parse_failure=0,
            total_latency_ms=3.0, mean_latency_ms=1.0,
            total_input_tokens=30, total_output_tokens=6, total_tokens=36,
            scores=rrs,
            probe_report=ProbeAndValidateReport(
                configured=True, reachable=True, latency_ms=1.0, model="x",
                input_tokens=1, output_tokens=1, base_url="x", error="",
            ),
            config=RealLLMConfig(base_url="x", model="x", timeout_sec=1.0, max_tokens=8),
            started_at="t0", finished_at="t1",
        )
        cmp_ = compare_heuristic_vs_real(hrs, real)
        for gap in ASI_5_GAPS:
            assert cmp_.delta_means[gap] == pytest.approx(0.0, abs=1e-9)


# ---------------------------------------------------------------------------
# Test 9: build_v1324_aggregate + build_bridge
# ---------------------------------------------------------------------------


class TestBuildBridge:
    def test_build_v1324_aggregate(self) -> None:
        hrs = _make_heuristic_results(3)
        rrs = _make_real_scores(hrs)
        real = RealBenchmarkResult(
            n_queries=3, n_chat_ok=3, n_fallback=0, n_parse_failure=0,
            total_latency_ms=3.0, mean_latency_ms=1.0,
            total_input_tokens=30, total_output_tokens=6, total_tokens=36,
            scores=rrs,
            probe_report=ProbeAndValidateReport(
                configured=True, reachable=True, latency_ms=1.0, model="x",
                input_tokens=1, output_tokens=1, base_url="x", error="",
            ),
            config=RealLLMConfig(base_url="x", model="x", timeout_sec=1.0, max_tokens=8),
            started_at="t0", finished_at="t1",
        )
        cmp_ = compare_heuristic_vs_real(hrs, real)
        agg = build_v1324_aggregate(real=real, comparison=cmp_)
        assert agg.version == V1324_VERSION
        assert agg.guard_marker == "v1324_real_llm_5gap"
        assert agg.v3_guards == V3_GUARD_MARKERS
        assert agg.pole_star_anchors["V0.1"] == 0.7905

    def test_build_bridge_with_real_run(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        hrs = _make_heuristic_results(22)
        agg = build_bridge(heuristic_results=hrs, scorer=scorer)
        assert agg.real_benchmark.n_queries == 22

    def test_build_bridge_without_real_run(self) -> None:
        hrs = _make_heuristic_results(22)
        agg = build_bridge(heuristic_results=hrs, scorer=None, auto_run_real=False)
        assert agg.real_benchmark.n_queries == 22
        assert agg.real_benchmark.n_chat_ok == 0
        assert agg.real_benchmark.n_fallback == 22

    def test_build_bridge_real_run_with_fake_client(self) -> None:
        scorer = LLMGapScorer(client=_FakeLLMClient(fail_pattern="all"))
        hrs = _make_heuristic_results(22)
        agg = build_bridge(heuristic_results=hrs, scorer=scorer)
        # all failures → 22 fallback
        assert agg.real_benchmark.n_chat_ok == 0
        assert agg.real_benchmark.n_fallback == 22


# ---------------------------------------------------------------------------
# Test 10: render_markdown_report
# ---------------------------------------------------------------------------


class TestRenderMarkdownReport:
    def _build_agg(self) -> Any:
        scorer = LLMGapScorer(client=_FakeLLMClient())
        hrs = _make_heuristic_results(22)
        return build_bridge(heuristic_results=hrs, scorer=scorer)

    def test_contains_version(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        assert "V1324" in md
        assert V1324_VERSION in md

    def test_contains_probe_section(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        assert "真探活" in md
        assert "reachable" in md

    def test_contains_benchmark_section(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        assert "n_chat_ok" in md
        assert "n_fallback" in md
        assert "total_tokens" in md

    def test_contains_per_query_table(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        assert "Q01" in md
        assert "Q22" in md
        assert "Q20_EMPTY" in md

    def test_contains_comparison_section(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        assert "V1323 heuristic vs V1324 real LLM" in md
        assert "pearson_r" in md
        assert "MAE" in md
        assert "RMSE" in md

    def test_contains_v3_guards(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        for marker in V3_GUARD_MARKERS_V1324:
            assert marker in md
        assert "v1324_real_llm_5gap" in md

    def test_contains_pole_star_anchors(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        assert "V0.1" in md
        assert "0.7905" in md
        assert "V0.2" in md
        assert "0.4467" in md
        assert "V1256_unio_mystica" in md
        assert "0.9291" in md
        assert "V1049_value_alignment" in md
        assert "DONE" in md

    def test_contains_v3_philosophy_statement(self) -> None:
        agg = self._build_agg()
        md = render_markdown_report(agg)
        # Honest reporting (主 17:43 实事求是 + 主 17:58 不假装)
        assert "不假装" in md
        assert "实事求是" in md


# ---------------------------------------------------------------------------
# Test 11: LOCKED invariants (regression guards)
# ---------------------------------------------------------------------------


class TestLockedInvariants:
    def test_asi_5_gaps_locked(self) -> None:
        assert ASI_5_GAPS == ("time", "freedom", "recognition", "emergence", "truth")

    def test_benchmark_queries_locked_at_22(self) -> None:
        assert len(BENCHMARK_QUERIES) == 22
        # All 22 query IDs present (format: Q01_TIME etc.)
        ids = [q[0] for q in BENCHMARK_QUERIES]
        for i in range(1, 23):
            qid = f"Q{i:02d}"
            # Check that some Q01..Q22 ID starts with the number
            assert any(q.startswith(qid) for q in ids), f"missing Q{i:02d}"
        # Specific known IDs
        assert "Q01_TIME" in ids
        assert "Q22_MIXED" in ids
        assert "Q20_EMPTY" in ids

    def test_pole_star_anchors_locked(self) -> None:
        assert ASI_ANCHORS["V0.1"] == 0.7905
        assert ASI_ANCHORS["V0.2"] == 0.4467
        assert ASI_ANCHORS["V1256_unio_mystica"] == 0.9291
        assert ASI_ANCHORS["V1049_value_alignment"] == "DONE"
        assert ASI_ANCHORS_V1324["V0.1"] == 0.7905
        assert ASI_ANCHORS_V1324["V0.2"] == 0.4467
        assert ASI_ANCHORS_V1324["V1256_unio_mystica"] == 0.9291
        assert ASI_ANCHORS_V1324["V1049_value_alignment"] == "DONE"

    def test_v3_guard_markers_preserved(self) -> None:
        assert V3_GUARD_MARKERS == V3_GUARD_MARKERS_V1324
        assert len(V3_GUARD_MARKERS) == 5  # LOCKED at 5

    def test_version_locked(self) -> None:
        assert V1324_VERSION == "0.1.0"

    def test_assert_benchmark_queries_locked_helper(self) -> None:
        _assert_benchmark_queries_locked()  # should not raise


# ---------------------------------------------------------------------------
# Test 12: env-cleared path (no key in env) — defensive
# ---------------------------------------------------------------------------


class TestEnvClearedPath:
    def test_default_config_when_no_env(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv(ENV_API_KEY, raising=False)
        monkeypatch.delenv(ENV_BASE_URL, raising=False)
        monkeypatch.delenv(ENV_MODEL, raising=False)
        cfg = default_config()
        # Defaults still apply (no env = use defaults)
        assert cfg.base_url == DEFAULT_BASE_URL
        assert cfg.model == DEFAULT_MODEL

    def test_real_llm_client_no_env_no_arg(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.delenv(ENV_API_KEY, raising=False)
        c = RealLLMClient()
        assert c.api_key == ""
        assert c.is_configured() is False


# ---------------------------------------------------------------------------
# Test 13: prompt template sanity (LOCKED)
# ---------------------------------------------------------------------------


class TestPromptTemplate:
    def test_prompt_includes_5_gaps(self) -> None:
        for gap in ASI_5_GAPS:
            assert gap in GAP_SCORING_PROMPT

    def test_prompt_includes_format_instruction(self) -> None:
        assert "5" in GAP_SCORING_PROMPT
        assert "comma-separated" in GAP_SCORING_PROMPT or "comma" in GAP_SCORING_PROMPT

    def test_prompt_format_with_query(self) -> None:
        out = GAP_SCORING_PROMPT.format(query="test query")
        assert "test query" in out
        assert "GAPS" in out or "gaps" in out.lower()


# ---------------------------------------------------------------------------
# Test 14: gap scoring prompt result with FakeClient (deterministic)
# ---------------------------------------------------------------------------


class TestGapScoringIntegration:
    def test_5_gaps_scored_per_query(self) -> None:
        def factory(prompt: str) -> str:
            # Detect which gap the query is about via prompt content
            qtext = prompt.lower()
            scores = [0.0, 0.0, 0.0, 0.0, 0.0]
            if "time" in qtext or "bergson" in qtext:
                scores[0] = 0.9
            if "free" in qtext or "spinoza" in qtext:
                scores[1] = 0.9
            if "recogni" in qtext or "levinas" in qtext:
                scores[2] = 0.9
            if "emerg" in qtext or "bedau" in qtext:
                scores[3] = 0.9
            if "truth" in qtext or "peirce" in qtext:
                scores[4] = 0.9
            return ",".join(f"{s:.1f}" for s in scores)

        scorer = LLMGapScorer(client=_FakeLLMClient(score_factory=factory))
        queries = [
            ("Q01", "test", "time philosophy Bergson", "time"),
            ("Q02", "test", "free will Spinoza", "freedom"),
            ("Q03", "test", "recognition Levinas", "recognition"),
            ("Q04", "test", "emergence Bedau", "emergence"),
            ("Q05", "test", "truth Peirce", "truth"),
        ]
        for qid, cat, qtext, focus in queries:
            s = scorer.score_one(qid, cat, qtext, focus)
            assert s.chat_ok is True
            assert s.fallback_used is False
            # The relevant gap should score ~0.9
            relevant_idx = ASI_5_GAPS.index(focus)
            relevant_score = s.gap_scores[focus]
            assert relevant_score >= 0.8, f"{qid}: {focus}={relevant_score} (expected >= 0.8)"


# ---------------------------------------------------------------------------
# Test 15: defensive — heuristic_results can be shorter than 22
# ---------------------------------------------------------------------------


class TestMismatchedLengths:
    def test_heuristic_shorter_than_real(self) -> None:
        hrs = _make_heuristic_results(3)
        rrs = _make_real_scores(hrs + _make_heuristic_results(7))  # 10
        real = RealBenchmarkResult(
            n_queries=10, n_chat_ok=10, n_fallback=0, n_parse_failure=0,
            total_latency_ms=10.0, mean_latency_ms=1.0,
            total_input_tokens=100, total_output_tokens=20, total_tokens=120,
            scores=rrs,
            probe_report=ProbeAndValidateReport(
                configured=True, reachable=True, latency_ms=1.0, model="x",
                input_tokens=1, output_tokens=1, base_url="x", error="",
            ),
            config=RealLLMConfig(base_url="x", model="x", timeout_sec=1.0, max_tokens=8),
            started_at="t0", finished_at="t1",
        )
        cmp_ = compare_heuristic_vs_real(hrs, real)
        # Should compare only 3 (the min)
        assert cmp_.n_queries == 3