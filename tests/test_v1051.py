"""Tests for v1051_real_llm_benchmark — V1051 ASI benchmark 真接 LLM 真跑.

V1051 = 真接 V1034 benchmark 接 LLM API 真跑 (主 06:15 + 主 00:36 + 主 23:44 + 主 22:33 +
   主 19:33 + 主 17:43 + 主 17:33).

测试覆盖:
- 22 真样本 (10+5+3+4)
- 4 真评测函数
- 4 heuristic fallback
- LLM caller 配置检查
- 真跑 (没 API key 自动 fallback)
- 哲学守门: benchmark ≠ ASI, fallback ≠ LLM.

主 17:43 实事求是: 默认测试不真调 LLM (没 API key), 真 fallback.
主 19:33: 真环境变量 / 真 httpx fallback 单元测试.
"""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import os
import re

import pytest

from apeireth.v1051_real_llm_benchmark import (
    V1051_VERSION,
    MMLU_SAMPLES_22,
    GSM8K_SAMPLES_22,
    HUMANEVAL_SAMPLES_22,
    HELLASWAG_SAMPLES_22,
    _eval_mmlu,
    _eval_gsm8k,
    _eval_humaneval,
    _eval_hellaswag,
    heuristic_mmlu_predictor,
    heuristic_gsm8k_predictor,
    heuristic_humaneval_predictor,
    heuristic_hellaswag_predictor,
    LLMCallResult,
    V1051LLMCaller,
    BenchmarkRunResult,
    V1051RealLLMBenchmark,
)


# ============================================================================
# 常量 & 22 真样本 (主 17:43 实事求是)
# ============================================================================


def test_version_set():
    assert V1051_VERSION == "0.1.0"


def test_mmlu_samples_count():
    """V1051 真样本 MMLU = 10 (主 17:43 实事求是)."""
    assert len(MMLU_SAMPLES_22) == 10


def test_gsm8k_samples_count():
    """V1051 真样本 GSM8K = 5."""
    assert len(GSM8K_SAMPLES_22) == 5


def test_humaneval_samples_count():
    """V1051 真样本 HumanEval = 3."""
    assert len(HUMANEVAL_SAMPLES_22) == 3


def test_hellaswag_samples_count():
    """V1051 真样本 HellaSwag = 4."""
    assert len(HELLASWAG_SAMPLES_22) == 4


def test_total_22_samples():
    """V1051 真样本 总数 = 22."""
    total = len(MMLU_SAMPLES_22) + len(GSM8K_SAMPLES_22) + len(HUMANEVAL_SAMPLES_22) + len(HELLASWAG_SAMPLES_22)
    assert total == 22


def test_mmlu_sample_structure():
    """V1051 真样本 MMLU 字段 (主 17:43 实事求是)."""
    for s in MMLU_SAMPLES_22:
        assert "question" in s
        assert "answer" in s
        assert "subject" in s


def test_gsm8k_sample_structure():
    for s in GSM8K_SAMPLES_22:
        assert "question" in s
        assert "answer" in s
        assert "solution" in s


def test_humaneval_sample_structure():
    for s in HUMANEVAL_SAMPLES_22:
        assert "prompt" in s
        assert "test" in s
        assert "reference" in s


def test_hellaswag_sample_structure():
    for s in HELLASWAG_SAMPLES_22:
        assert "context" in s
        assert "answer" in s
        assert "label" in s


# ============================================================================
# 评测函数 (主 17:43 实事求是)
# ============================================================================


def test_eval_mmlu_exact_match():
    ok, score = _eval_mmlu("Paris", "Paris")
    assert ok is True
    assert score == 1.0


def test_eval_mmlu_partial_match():
    ok, score = _eval_mmlu("Paris", "Paris is the capital")
    assert ok is True
    assert score >= 0.5


def test_eval_mmlu_no_match():
    ok, score = _eval_mmlu("Paris", "London")
    assert ok is False
    assert score == 0.0


def test_eval_gsm8k_exact_number():
    ok, score = _eval_gsm8k("120", "The answer is 120.")
    assert ok is True
    assert score == 1.0


def test_eval_gsm8k_no_match():
    ok, score = _eval_gsm8k("120", "I don't know")
    assert ok is False
    assert score == 0.0


def test_eval_humaneval_exact():
    ok, score = _eval_humaneval("return a + b", "def add(a,b): return a + b")
    assert ok is True


def test_eval_humaneval_token_match():
    ok, score = _eval_humaneval("return n % 2 == 0", "return n % 2")
    assert ok is True


def test_eval_humaneval_no_match():
    ok, score = _eval_humaneval("return a + b", "completely different")
    assert ok is False


def test_eval_hellaswag_exact():
    ok, score = _eval_hellaswag("mat", "mat")
    assert ok is True


def test_eval_hellaswag_partial():
    ok, score = _eval_hellaswag("mat", "the mat")
    assert ok is True


def test_eval_hellaswag_no_match():
    ok, score = _eval_hellaswag("mat", "car")
    assert ok is False


# ============================================================================
# Heuristic predictors (主 17:43 真 fallback)
# ============================================================================


def test_heuristic_mmlu_capital_france():
    """V1051 真 fallback heuristic MMLU 首都."""
    assert heuristic_mmlu_predictor("The capital of France is:") == "Paris"


def test_heuristic_mmlu_water():
    assert heuristic_mmlu_predictor("H2O is the chemical formula for:") == "water"


def test_heuristic_mmlu_jupiter():
    assert heuristic_mmlu_predictor("The largest planet in our solar system is:") == "Jupiter"


def test_heuristic_gsm8k_simple():
    """V1051 真 fallback heuristic GSM8K."""
    assert heuristic_gsm8k_predictor("Janet has 3 apples. She gives 1 to her friend. How many apples does she have now?") == "2"


def test_heuristic_gsm8k_train():
    assert heuristic_gsm8k_predictor("If a train travels 60 miles per hour for 2 hours, how far does it go?") == "120"


def test_heuristic_gsm8k_pages():
    assert heuristic_gsm8k_predictor("A book has 200 pages. Tom reads 50 pages on Monday and 30 on Tuesday. How many pages are left?") == "120"


def test_heuristic_humaneval_add():
    """V1051 真 fallback heuristic HumanEval."""
    pred = heuristic_humaneval_predictor('def add(a, b):\n    """Return the sum of a and b."""\n')
    assert "a + b" in pred or "a+b" in pred


def test_heuristic_humaneval_even():
    pred = heuristic_humaneval_predictor('def is_even(n):\n    """Return True if n is even."""\n')
    assert "2" in pred


def test_heuristic_humaneval_max():
    pred = heuristic_humaneval_predictor('def max_of_three(a, b, c):\n    """Return the maximum of a, b, c."""\n')
    assert "max" in pred


def test_heuristic_hellaswag_cat():
    """V1051 真 fallback heuristic HellaSwag."""
    assert heuristic_hellaswag_predictor("The cat sat on the") == "mat"


def test_heuristic_hellaswag_chef():
    assert heuristic_hellaswag_predictor("The chef carefully chopped the vegetables and put them in the") == "pan"


# ============================================================================
# LLMCaller 配置 (主 17:43 实事求是)
# ============================================================================


def test_llm_caller_unconfigured_no_api_key():
    """V1051 真测 没 API key 不假装 (主 17:43 实事求是)."""
    caller = V1051LLMCaller(api_key="", base_url="https://example.com", model="test")
    assert caller.is_configured() is False


def test_llm_caller_configured_with_key():
    caller = V1051LLMCaller(api_key="sk-test-key", base_url="https://example.com", model="test")
    assert caller.is_configured() is True


def test_llm_caller_unconfigured_call_returns_failure():
    """V1051 真测 没 API key 真调返回 ok=False (主 17:43 实事求是)."""
    caller = V1051LLMCaller(api_key="", base_url="https://example.com", model="test")
    r = caller.call("Hello")
    assert r.ok is False
    assert "OPENAI_API_KEY" in r.error or "configured" in r.error.lower()


def test_llm_caller_call_log_appends():
    """V1051 真测 call_log 累积."""
    caller = V1051LLMCaller(api_key="", base_url="https://example.com", model="test")
    caller.call("test1")
    caller.call("test2")
    assert caller.n_calls() == 2


def test_llm_caller_n_successful_zero_when_no_key():
    caller = V1051LLMCaller(api_key="", base_url="https://example.com", model="test")
    caller.call("test")
    assert caller.n_successful() == 0


def test_llm_caller_env_override(monkeypatch):
    """V1051 真测 环境变量真覆盖 (主 19:33 OpenAI 真借鉴)."""
    monkeypatch.setenv("OPENAI_API_KEY", "sk-env-test")
    monkeypatch.setenv("OPENAI_BASE_URL", "https://env.example.com/v1")
    monkeypatch.setenv("OPENAI_MODEL", "env-model")
    caller = V1051LLMCaller()
    assert caller.api_key == "sk-env-test"
    assert caller.base_url == "https://env.example.com/v1"
    assert caller.model == "env-model"


# ============================================================================
# LLMCallResult 数据结构
# ============================================================================


def test_llm_call_result_to_dict():
    r = LLMCallResult(
        prompt="test",
        response="response",
        model="m",
        elapsed_seconds=0.1,
        ok=True,
    )
    d = r.to_dict()
    assert d["model"] == "m"
    assert d["ok"] is True
    assert d["elapsed_seconds"] == 0.1


def test_llm_call_result_with_error():
    r = LLMCallResult(
        prompt="test",
        response="",
        model="m",
        elapsed_seconds=0.0,
        ok=False,
        error="oops",
    )
    d = r.to_dict()
    assert d["error"] == "oops"
    assert d["ok"] is False


def test_benchmark_run_result_to_dict():
    r = BenchmarkRunResult(
        benchmark="MMLU",
        n_samples=10,
        n_correct=7,
        accuracy=0.7,
        llm_used=5,
        fallback_used=5,
    )
    d = r.to_dict()
    assert d["benchmark"] == "MMLU"
    assert d["accuracy"] == 0.7
    assert d["llm_used"] == 5


# ============================================================================
# V1051 真跑 (主 17:43 实事求是: 没 API key 自动 fallback)
# ============================================================================


def test_run_mmlu_no_api_key_uses_fallback():
    """V1051 真测 没 API key 真 fallback (主 17:43 实事求是)."""
    bench = V1051RealLLMBenchmark(api_key="")
    r = bench.run_mmlu()
    assert r.benchmark == "MMLU"
    assert r.n_samples == 10
    assert r.llm_used == 0  # 没真调 LLM
    assert r.fallback_used == 10  # 真 fallback
    assert r.n_correct >= 5  # heuristic 至少对一半


def test_run_gsm8k_no_api_key_uses_fallback():
    bench = V1051RealLLMBenchmark(api_key="")
    r = bench.run_gsm8k()
    assert r.benchmark == "GSM8K"
    assert r.n_samples == 5
    assert r.llm_used == 0
    assert r.fallback_used == 5
    assert r.n_correct >= 3


def test_run_humaneval_no_api_key_uses_fallback():
    bench = V1051RealLLMBenchmark(api_key="")
    r = bench.run_humaneval()
    assert r.benchmark == "HumanEval"
    assert r.n_samples == 3
    assert r.fallback_used == 3


def test_run_hellaswag_no_api_key_uses_fallback():
    bench = V1051RealLLMBenchmark(api_key="")
    r = bench.run_hellaswag()
    assert r.benchmark == "HellaSwag"
    assert r.n_samples == 4
    assert r.fallback_used == 4
    assert r.n_correct >= 3


def test_run_all_no_api_key():
    """V1051 真跑 all 4 benchmarks, 真 fallback (主 17:43 实事求是)."""
    bench = V1051RealLLMBenchmark(api_key="")
    result = bench.run_all()
    assert result["n_samples"] == 22
    assert result["llm_used"] == 0
    assert result["fallback_used"] == 22
    assert "benchmarks" in result
    assert len(result["benchmarks"]) == 4
    assert result["api_configured"] is False
    # 总体 accuracy 在合理范围
    assert 0.0 <= result["overall_accuracy"] <= 1.0


def test_run_all_results_stored():
    """V1051 真跑 results 存到 self.results."""
    bench = V1051RealLLMBenchmark(api_key="")
    bench.run_all()
    assert "MMLU" in bench.results
    assert "GSM8K" in bench.results
    assert "HumanEval" in bench.results
    assert "HellaSwag" in bench.results


# ============================================================================
# 真 _extract_prediction (主 17:43 实事求是)
# ============================================================================


def test_extract_prediction_with_llm_response():
    bench = V1051RealLLMBenchmark(api_key="")
    pred, used = bench._extract_prediction("MMLU", MMLU_SAMPLES_22[0], "Paris", heuristic_mmlu_predictor)
    assert pred == "Paris"
    assert used is True


def test_extract_prediction_gsm8k_extracts_number():
    bench = V1051RealLLMBenchmark(api_key="")
    pred, used = bench._extract_prediction("GSM8K", GSM8K_SAMPLES_22[0], "The answer is 2.", heuristic_gsm8k_predictor)
    assert pred == "2"
    assert used is True


def test_extract_prediction_humaneval_extracts_code():
    bench = V1051RealLLMBenchmark(api_key="")
    pred, used = bench._extract_prediction("HumanEval", HUMANEVAL_SAMPLES_22[0], "```python\nreturn a + b\n```", heuristic_humaneval_predictor)
    assert "a + b" in pred
    assert used is True


def test_extract_prediction_empty_response_uses_heuristic():
    """V1051 真测 空 response 真 fallback (主 17:43 实事求是)."""
    bench = V1051RealLLMBenchmark(api_key="")
    pred, used = bench._extract_prediction("MMLU", MMLU_SAMPLES_22[0], "", heuristic_mmlu_predictor)
    assert used is False
    assert pred == "Paris"  # heuristic fallback 真答


def test_extract_prediction_hellaswag_with_response():
    bench = V1051RealLLMBenchmark(api_key="")
    pred, used = bench._extract_prediction("HellaSwag", HELLASWAG_SAMPLES_22[0], "mat", heuristic_hellaswag_predictor)
    assert pred == "mat"
    assert used is True


# ============================================================================
# _build_prompt
# ============================================================================


def test_build_prompt_mmlu():
    bench = V1051RealLLMBenchmark(api_key="")
    user_p, sys_p = bench._build_prompt("MMLU", MMLU_SAMPLES_22[0])
    assert "Paris" in user_p or "France" in user_p
    assert sys_p is not None


def test_build_prompt_gsm8k():
    bench = V1051RealLLMBenchmark(api_key="")
    user_p, sys_p = bench._build_prompt("GSM8K", GSM8K_SAMPLES_22[0])
    assert "Janet" in user_p or "apples" in user_p
    assert sys_p is not None


def test_build_prompt_humaneval():
    bench = V1051RealLLMBenchmark(api_key="")
    user_p, sys_p = bench._build_prompt("HumanEval", HUMANEVAL_SAMPLES_22[0])
    assert "def add" in user_p


def test_build_prompt_hellaswag():
    bench = V1051RealLLMBenchmark(api_key="")
    user_p, sys_p = bench._build_prompt("HellaSwag", HELLASWAG_SAMPLES_22[0])
    assert "cat sat" in user_p


# ============================================================================
# stats (主 00:56 任何人都能接手)
# ============================================================================


def test_stats_has_version():
    bench = V1051RealLLMBenchmark(api_key="")
    s = bench.stats()
    assert s["version"] == V1051_VERSION
    assert "philosophy" in s


def test_stats_initial_zero_calls():
    bench = V1051RealLLMBenchmark(api_key="")
    s = bench.stats()
    assert s["n_llm_calls"] == 0
    assert s["n_successful_calls"] == 0


def test_stats_api_configured_field():
    bench = V1051RealLLMBenchmark(api_key="")
    s = bench.stats()
    assert "api_configured" in s
    assert s["api_configured"] is False


def test_stats_model_field():
    bench = V1051RealLLMBenchmark(api_key="", model="custom-model")
    s = bench.stats()
    assert s["model"] == "custom-model"


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_module_does_not_pretend_phenomenal():
    """V3 哲学守门: 不假装 Phenomenal consciousness."""
    import apeireth.v1051_real_llm_benchmark as m
    src = m.__doc__ or ""
    assert "Phenomenal" in src or "不假装" in src


def test_module_does_not_pretend_asi_solved():
    """V3 哲学守门: benchmark ≠ ASI. V1051 真接 LLM 真跑 ≠ ASI 真测."""
    import apeireth.v1051_real_llm_benchmark as m
    assert hasattr(m, "V3_GUARDS")
    assert "benchmark_is_not_asi" in m.V3_GUARDS


def test_fallback_is_not_llm_guarded():
    """V3 哲学守门: heuristic fallback ≠ LLM 真跑."""
    import apeireth.v1051_real_llm_benchmark as m
    assert "fallback_is_not_llm" in m.V3_GUARDS
    text = m.V3_GUARDS["fallback_is_not_llm"]
    assert "heuristic" in text.lower() or "fallback" in text.lower()


def test_module_does_not_pretend_consciousness():
    """V3 哲学守门: structure_is_not_consciousness 必备."""
    import apeireth.v1051_real_llm_benchmark as m
    assert "structure_is_not_consciousness" in m.V3_GUARDS


def test_production_is_not_safety_guarded():
    import apeireth.v1051_real_llm_benchmark as m
    assert "production_is_not_safety" in m.V3_GUARDS


# ============================================================================
# 集成测试 (主 00:36 工程化)
# ============================================================================


def test_integration_full_pipeline_no_api_key():
    """V1051 真生产 完整 pipeline: 没 API key, 全 fallback, 真标."""
    bench = V1051RealLLMBenchmark(api_key="")
    result = bench.run_all()
    # 22 样本全 fallback
    assert result["n_samples"] == 22
    assert result["llm_used"] == 0
    assert result["fallback_used"] == 22
    # accuracy 合理
    assert 0.0 <= result["overall_accuracy"] <= 1.0
    # stats 也反映
    s = bench.stats()
    assert s["n_benchmarks"] == 4
    # 真调次数 = 22 (都尝试了, 都没成功)
    assert s["n_llm_calls"] == 22
    assert s["n_successful_calls"] == 0


def test_integration_with_fake_api_key_will_try_but_fail():
    """V1051 真测 假 API key 会真尝试 (主 17:43 实事求是)."""
    bench = V1051RealLLMBenchmark(api_key="sk-fake-xyz", base_url="https://nonexistent.example.com", timeout=2)
    r = bench.run_mmlu()
    # 真尝试 LLM, 全失败, fallback
    assert r.n_samples == 10
    # llm_used 可能是 0 (因为 extract_prediction 只在 response 非空时算 used)
    # fallback_used 应该 = 10 因为 LLM 都失败了
    assert r.fallback_used == 10


def test_all_exports_present():
    """V1051 真测 __all__ 导出完整 (主 00:56 任何人都能接手)."""
    from apeireth import v1051_real_llm_benchmark as m
    expected = [
        "V1051_VERSION",
        "MMLU_SAMPLES_22", "GSM8K_SAMPLES_22", "HUMANEVAL_SAMPLES_22", "HELLASWAG_SAMPLES_22",
        "_eval_mmlu", "_eval_gsm8k", "_eval_humaneval", "_eval_hellaswag",
        "heuristic_mmlu_predictor", "heuristic_gsm8k_predictor",
        "heuristic_humaneval_predictor", "heuristic_hellaswag_predictor",
        "LLMCallResult", "V1051LLMCaller",
        "BenchmarkRunResult", "V1051RealLLMBenchmark",
    ]
    for name in expected:
        assert name in m.__all__, f"missing export: {name}"


def test_demo_runs():
    """V1051 真测 _demo 函数能跑 (主 00:56 任何人都能接手)."""
    from apeireth.v1051_real_llm_benchmark import _demo
    try:
        _demo()
    except Exception as e:
        pytest.fail(f"_demo crashed: {e}")


def test_results_are_consistent():
    """V1051 真测 results 字典一致."""
    bench = V1051RealLLMBenchmark(api_key="")
    bench.run_mmlu()
    bench.run_gsm8k()
    assert "MMLU" in bench.results
    assert "GSM8K" in bench.results
    assert "HumanEval" not in bench.results
    assert "HellaSwag" not in bench.results


def test_each_benchmark_returns_benchmark_run_result():
    """V1051 真测 每个 benchmark 返回 BenchmarkRunResult."""
    bench = V1051RealLLMBenchmark(api_key="")
    assert isinstance(bench.run_mmlu(), BenchmarkRunResult)
    assert isinstance(bench.run_gsm8k(), BenchmarkRunResult)
    assert isinstance(bench.run_humaneval(), BenchmarkRunResult)
    assert isinstance(bench.run_hellaswag(), BenchmarkRunResult)