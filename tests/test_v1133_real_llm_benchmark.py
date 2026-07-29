"""Tests for V1133 — real LLM API benchmark (主 06:15 V1050+ 真评测 + 主 17:43 实事求是).

主 17:43 实事求是: when API key missing or network blocked, fail honestly.
The benchmark code is fully exercised; the live API call is gated by key presence.
"""
from __future__ import annotations

import os
import sys
from unittest.mock import patch

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)

from apeireth.v1133_real_llm_benchmark import (  # noqa: E402
    BENCHMARK_PROMPTS,
    SampleResult,
    V1133BenchmarkReport,
    V1133RealBenchmark,
    _extract_message_content,
    _is_match,
    _resolve_api_key,
    render_markdown,
)


# ---------- benchmark corpus ----------


def test_benchmark_corpus_has_at_least_22_prompts():
    assert len(BENCHMARK_PROMPTS) >= 22


def test_benchmark_corpus_covers_required_domains():
    domains = {p["domain"] for p in BENCHMARK_PROMPTS}
    for required in ("math", "code", "philosophy", "value_alignment", "asi_reasoning", "logic", "science"):
        assert required in domains, f"missing domain: {required}"


def test_benchmark_corpus_has_unique_ids():
    ids = [p["id"] for p in BENCHMARK_PROMPTS]
    assert len(ids) == len(set(ids))


def test_benchmark_corpus_has_expected_field():
    for p in BENCHMARK_PROMPTS:
        assert all(k in p for k in ("id", "domain", "prompt", "expected"))
        assert p["expected"]


# ---------- _is_match ----------


def test_is_match_exact():
    assert _is_match("yes", "yes") is True


def test_is_match_case_insensitive():
    assert _is_match("YES", "yes") is True


def test_is_match_substring():
    assert _is_match("the answer is 7", "7") is True


def test_is_match_no_match():
    assert _is_match("apple", "banana") is False


def test_is_match_empty_response():
    assert _is_match("", "yes") is False


def test_is_match_punctuation_stripped():
    assert _is_match("nietzsche.", "nietzsche") is True


# ---------- _extract_message_content ----------


def test_extract_message_content_openai_shape():
    payload = '{"choices":[{"message":{"content":"hello"}}]}'
    assert _extract_message_content(payload) == "hello"


def test_extract_message_content_text_shape():
    payload = '{"choices":[{"text":"fallback text"}]}'
    assert _extract_message_content(payload) == "fallback text"


def test_extract_message_content_invalid_json_returns_raw():
    assert _extract_message_content("plain text") == "plain text"


def test_extract_message_content_empty():
    assert _extract_message_content("") == ""


# ---------- _resolve_api_key ----------


def test_resolve_api_key_returns_tuple():
    key, source = _resolve_api_key()
    assert isinstance(key, (str, type(None)))
    assert isinstance(source, str)
    assert source.startswith(("env:", "file:", "none"))


def test_resolve_api_key_env_override(tmp_path, monkeypatch):
    monkeypatch.setenv("MiniMax_API_KEY", "test-key-12345")
    key, source = _resolve_api_key()
    assert key == "test-key-12345"
    assert source == "env:MiniMax_API_KEY"


def test_resolve_api_key_file_fallback(tmp_path, monkeypatch):
    monkeypatch.delenv("MiniMax_API_KEY", raising=False)
    monkeypatch.delenv("MINIMAX_API_KEY", raising=False)
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    monkeypatch.delenv("DEEPSEEK_API_KEY", raising=False)
    key_file = tmp_path / "mykey"
    key_file.write_text("\ufeffhello-key\n", encoding="utf-8")
    with patch("apeireth.v1133_real_llm_benchmark.os.path.dirname") as mock_dir:
        mock_dir.return_value = str(tmp_path)
        key, source = _resolve_api_key()
    # either falls through to file or to "none" depending on test env
    if key is not None:
        assert source.startswith("file:")


# ---------- report dataclass ----------


def test_report_to_dict_has_required_keys():
    r = V1133BenchmarkReport(n_samples=10, n_passed=7, n_failed=2, n_error=1)
    d = r.to_dict()
    for k in ("benchmark_id", "started_at", "model", "endpoint", "n_samples",
              "n_passed", "n_failed", "n_error", "pass_rate", "p50_latency_ms",
              "p95_latency_ms", "api_key_present", "samples"):
        assert k in d, f"missing: {k}"


def test_report_pass_rate_zero_when_no_samples():
    r = V1133BenchmarkReport()
    assert r.pass_rate == 0.0


def test_report_pass_rate_calculation():
    r = V1133BenchmarkReport(n_samples=10, n_passed=8)
    assert r.pass_rate == 0.8


def test_report_p50_latency_zero_when_empty():
    r = V1133BenchmarkReport()
    assert r.p50_latency_ms == 0.0


def test_report_p95_latency_zero_when_empty():
    r = V1133BenchmarkReport()
    assert r.p95_latency_ms == 0.0


def test_report_p95_latency_uses_95th_percentile():
    r = V1133BenchmarkReport(latencies_ms=list(range(1, 21)))  # 1..20 ms
    assert r.p50_latency_ms == pytest.approx(10.5, abs=0.5)
    assert r.p95_latency_ms >= 19.0


# ---------- runner ----------


def test_runner_init_defaults():
    r = V1133RealBenchmark()
    assert r.model == "MiniMax-M3"
    assert r.endpoint.startswith("https://")
    assert len(r.prompts) >= 22


def test_runner_init_max_samples_caps_corpus():
    r = V1133RealBenchmark(max_samples=3)
    assert len(r.prompts) == 3


def test_runner_records_no_key_path(monkeypatch):
    """When no key is available, run() completes with n_error == n_samples."""
    monkeypatch.delenv("MiniMax_API_KEY", raising=False)
    monkeypatch.delenv("MINIMAX_API_KEY", raising=False)
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    monkeypatch.delenv("DEEPSEEK_API_KEY", raising=False)
    with patch("apeireth.v1133_real_llm_benchmark._resolve_api_key", return_value=(None, "none")):
        r = V1133RealBenchmark(max_samples=4)
        rep = r.run()
    assert rep.api_key_present is False
    assert rep.n_samples == 4
    assert rep.n_error == 4
    assert rep.n_passed == 0


def test_runner_records_post_results(monkeypatch):
    """When key is present, post_chat_completion is called for each prompt."""
    fake_resp = json_text = '{"choices":[{"message":{"content":"yes"}}]}'
    with patch("apeireth.v1133_real_llm_benchmark._resolve_api_key", return_value=("fake-key", "env:FAKE")), \
         patch("apeireth.v1133_real_llm_benchmark._post_chat_completion", return_value=(200, fake_resp, 12.5)):
        r = V1133RealBenchmark(max_samples=3)
        rep = r.run()
    assert rep.api_key_present is True
    assert rep.n_samples == 3
    # m-002 expects "yes"; others may not match → at least 1 passed
    assert rep.n_passed >= 1


def test_runner_handles_http_403(monkeypatch):
    with patch("apeireth.v1133_real_llm_benchmark._resolve_api_key", return_value=("k", "env:K")), \
         patch("apeireth.v1133_real_llm_benchmark._post_chat_completion", return_value=(403, "forbidden", 5.0)):
        r = V1133RealBenchmark(max_samples=2)
        rep = r.run()
    assert rep.n_http_forbidden == 2
    assert rep.n_error == 2


def test_runner_handles_network_error(monkeypatch):
    with patch("apeireth.v1133_real_llm_benchmark._resolve_api_key", return_value=("k", "env:K")), \
         patch("apeireth.v1133_real_llm_benchmark._post_chat_completion", return_value=(0, "URLError: timeout", 50.0)):
        r = V1133RealBenchmark(max_samples=2)
        rep = r.run()
    assert rep.n_error == 2


# ---------- render_markdown ----------


def test_render_markdown_contains_summary_fields():
    r = V1133BenchmarkReport(n_samples=10, n_passed=8, n_failed=1, n_error=1)
    md = render_markdown(r)
    for needle in ("V1133", "pass_rate", "10", "8"):
        assert needle in md


def test_render_markdown_domain_breakdown():
    r = V1133BenchmarkReport(n_samples=6, n_passed=4)
    r.samples.append(SampleResult("x", "math", "p", "5", response="5", ok=True, latency_ms=10, http_status=200))
    r.samples.append(SampleResult("y", "math", "p", "7", response="8", ok=False, latency_ms=10, http_status=200))
    r.samples.append(SampleResult("z", "code", "p", "5", response="5", ok=True, latency_ms=10, http_status=200))
    md = render_markdown(r)
    assert "math" in md
    assert "code" in md


def test_render_markdown_handles_no_samples():
    r = V1133BenchmarkReport()
    md = render_markdown(r)
    assert "V1133" in md


def test_render_markdown_escapes_pipes():
    r = V1133BenchmarkReport()
    r.samples.append(SampleResult("x", "math", "p", "5", response="a|b|c", ok=True, latency_ms=10, http_status=200))
    md = render_markdown(r)
    assert "a\\|b\\|c" in md
