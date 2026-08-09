"""Tests for V1424 — ASI 总框架 benchmark 真接 LLM."""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1424_asi_real_llm_benchmark as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    assert m.V1424_VERSION == "0.1.0"
    assert m.V1424_SCHEMA == "v1424.asi-real-llm-benchmark/v1"
    assert m.V1424_MODULE == "v1424_asi_real_llm_benchmark"


def test_guards_and_v3_guards_well_formed():
    assert len(m.V1424_GUARDS) >= 20
    assert len(m.V1424_V3_GUARDS) >= 9
    assert len(m.V1424_BORROWED) >= 6
    keys = [b[0] for b in m.V1424_BORROWED]
    assert "V1034" in keys
    assert "V1422" in keys
    assert "V1423" in keys


def test_providers_complete():
    """PROVIDERS must contain all 5 expected entries."""
    expected = {"newapi", "openai", "anthropic", "generic", "deterministic"}
    assert set(m.PROVIDERS) == expected


def test_benchmark_order_complete():
    """BENCHMARK_ORDER must contain all 4 V1034 benchmarks."""
    assert set(m.BENCHMARK_ORDER) == {"MMLU", "GSM8K", "HUMANEVAL", "HELLASWAG"}


# ============================================================================
# Validation helpers
# ============================================================================


def test_validate_provider_accepts_all():
    for p in m.PROVIDERS:
        assert m._validate_provider(p) == p


def test_validate_provider_rejects_unknown():
    with pytest.raises(ValueError):
        m._validate_provider("FOO")


def test_validate_endpoint_accepts_https():
    assert m._validate_endpoint("https://api.example.com/v1") == "https://api.example.com/v1"


def test_validate_endpoint_accepts_http():
    assert m._validate_endpoint("http://127.0.0.1:8080/v1") == "http://127.0.0.1:8080/v1"


def test_validate_endpoint_rejects_ftp():
    with pytest.raises(ValueError):
        m._validate_endpoint("ftp://x")


def test_validate_endpoint_rejects_whitespace():
    with pytest.raises(ValueError):
        m._validate_endpoint("http://example.com/foo bar")


def test_validate_endpoint_rejects_short():
    with pytest.raises(ValueError):
        m._validate_endpoint("http://")


def test_validate_model_accepts_normal():
    assert m._validate_model("gpt-4o-mini") == "gpt-4o-mini"


def test_validate_model_rejects_empty():
    with pytest.raises(ValueError):
        m._validate_model("")


def test_validate_model_rejects_too_long():
    with pytest.raises(ValueError):
        m._validate_model("a" * 200)


def test_validate_timeout_accepts():
    assert m._validate_timeout(30) == 30


def test_validate_timeout_rejects_zero():
    with pytest.raises(ValueError):
        m._validate_timeout(0)


def test_validate_timeout_rejects_huge():
    with pytest.raises(ValueError):
        m._validate_timeout(999)


def test_validate_max_tokens_accepts():
    assert m._validate_max_tokens(256) == 256


def test_validate_max_tokens_rejects_huge():
    with pytest.raises(ValueError):
        m._validate_max_tokens(99999)


def test_validate_temperature_accepts():
    assert m._validate_temperature(0.5) == 0.5


def test_validate_temperature_rejects_huge():
    with pytest.raises(ValueError):
        m._validate_temperature(3.0)


def test_validate_max_retries_accepts():
    assert m._validate_max_retries(1) == 1


def test_validate_max_retries_rejects_huge():
    with pytest.raises(ValueError):
        m._validate_max_retries(99)


def test_validate_max_samples_accepts():
    assert m._validate_max_samples(22) == 22


def test_validate_max_samples_rejects_huge():
    with pytest.raises(ValueError):
        m._validate_max_samples(100)


# ============================================================================
# Default config + validation
# ============================================================================


def test_default_config_is_deterministic():
    """Default config must use deterministic provider (no env required)."""
    cfg = m.build_default_config()
    assert cfg.provider == "deterministic"
    assert cfg.base_url == ""
    assert cfg.model == "deterministic-v0"


def test_default_config_validates():
    """Default config validates without raising."""
    cfg = m.build_default_config()
    cfg2 = m.validate_config(cfg)
    assert cfg2 is cfg


def test_real_provider_requires_base_url():
    """Provider=openai without base_url must fail validation."""
    cfg = m.build_default_config({"provider": "openai", "model": "gpt-4"})
    # Note: key may or may not be in env, but base_url is empty → must fail
    if os.environ.get("APEIRETH_LLM_KEY"):
        pytest.skip("APEIRETH_LLM_KEY is set in env")
    with pytest.raises(ValueError):
        m.validate_config(cfg)


def test_real_provider_requires_key_when_base_url_set():
    """Provider=openai with base_url but no key must fail validation."""
    if os.environ.get("APEIRETH_LLM_KEY"):
        pytest.skip("APEIRETH_LLM_KEY is set in env")
    cfg = m.build_default_config(
        {
            "provider": "openai",
            "model": "gpt-4",
            "base_url": "https://api.example.com/v1",
        }
    )
    with pytest.raises(ValueError):
        m.validate_config(cfg)


def test_real_provider_passes_when_key_set(monkeypatch):
    """Provider=openai with base_url AND key must pass validation."""
    monkeypatch.setenv("APEIRETH_LLM_KEY", "sk-test-fake")
    cfg = m.build_default_config(
        {
            "provider": "openai",
            "model": "gpt-4",
            "base_url": "https://api.example.com/v1",
        }
    )
    cfg2 = m.validate_config(cfg)
    assert cfg2.provider == "openai"


# ============================================================================
# Deterministic predictor (MOCK mode)
# ============================================================================


def test_deterministic_predict_returns_mock_string():
    pred, pt, ct = m._deterministic_predict("What is 2+2?", 64)
    assert pred.startswith("deterministic:")
    assert pt > 0
    assert ct > 0


def test_deterministic_predict_deterministic():
    """Same question → same prediction (deterministic, no randomness)."""
    p1, _, _ = m._deterministic_predict("hello world", 64)
    p2, _, _ = m._deterministic_predict("hello world", 64)
    assert p1 == p2


def test_predict_dispatches_deterministic():
    """predict() with provider=deterministic returns mode=MOCK."""
    cfg = m.build_default_config({"provider": "deterministic"})
    pred, pt, ct, status, mode = m.predict(cfg, "What is 2+2?")
    assert mode == "MOCK"
    assert status == 200
    assert pred.startswith("deterministic:")


# ============================================================================
# Sample index (V1034 integration)
# ============================================================================


def test_sample_index_has_22_items():
    items = m._build_sample_index()
    assert len(items) == 22


def test_sample_index_benchmark_distribution():
    """22 items must split as 10 MMLU + 5 GSM8K + 3 HUMANEVAL + 4 HELLASWAG."""
    items = m._build_sample_index()
    bench_count: dict = {}
    for _, s in items:
        bench_count[s["_benchmark"]] = bench_count.get(s["_benchmark"], 0) + 1
    assert bench_count["MMLU"] == 10
    assert bench_count["GSM8K"] == 5
    assert bench_count["HUMANEVAL"] == 3
    assert bench_count["HELLASWAG"] == 4


def test_sample_index_unique_ids():
    items = m._build_sample_index()
    ids = [sid for sid, _ in items]
    assert len(set(ids)) == 22


def test_extract_question_mmlu():
    sample = {"question": "What is 2+2?", "answer": "4", "_benchmark": "MMLU", "_kind": "mcq"}
    assert m._extract_question(sample, "mcq") == "What is 2+2?"


def test_extract_question_humaneval():
    sample = {"prompt": "def foo():", "reference": "pass", "_benchmark": "HUMANEVAL", "_kind": "code"}
    assert m._extract_question(sample, "code") == "def foo():"


def test_extract_ground_truth_variants():
    a = {"answer": "4"}
    b = {"reference": "pass"}
    assert m._extract_ground_truth(a, "mcq") == "4"
    assert m._extract_ground_truth(b, "code") == "pass"


# ============================================================================
# Atomic JSONL append
# ============================================================================


def test_append_result_log_creates_file():
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "log.jsonl"
        ok = m._append_result_log(logp, {"a": 1, "b": "x"})
        assert ok is True
        assert logp.exists()


def test_append_result_log_appends_existing():
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "log.jsonl"
        m._append_result_log(logp, {"a": 1})
        m._append_result_log(logp, {"b": 2})
        m._append_result_log(logp, {"c": 3})
        lines = logp.read_text(encoding="utf-8").splitlines()
        assert len(lines) == 3


# ============================================================================
# End-to-end deterministic benchmark
# ============================================================================


def test_run_benchmark_deterministic_smoke():
    """End-to-end: deterministic mode, 22 samples, all MOCK."""
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config(
            {
                "provider": "deterministic",
                "max_samples": 22,
                "log_path": Path(td) / "results.jsonl",
            }
        )
        report = m.run_benchmark(cfg)
        assert report.n_samples == 22
        assert report.n_mock == 22
        assert report.n_real == 0
        assert report.provider == "deterministic"
        # per_benchmark must have all 4 keys
        assert set(report.per_benchmark.keys()) == {"MMLU", "GSM8K", "HUMANEVAL", "HELLASWAG"}


def test_run_benchmark_writes_log():
    """End-to-end: run writes per-sample records to log."""
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "results.jsonl"
        cfg = m.build_default_config({"provider": "deterministic", "max_samples": 22, "log_path": logp})
        m.run_benchmark(cfg)
        assert logp.exists()
        lines = logp.read_text(encoding="utf-8").splitlines()
        assert len(lines) == 22
        # Each line is JSON with sample_id
        rec = json.loads(lines[0])
        assert "sample_id" in rec
        assert "mode" in rec
        assert rec["mode"] == "MOCK"


def test_run_benchmark_smaller_max_samples():
    """max_samples=5 runs only 5 samples."""
    with tempfile.TemporaryDirectory() as td:
        cfg = m.build_default_config(
            {"provider": "deterministic", "max_samples": 5, "log_path": Path(td) / "r.jsonl"}
        )
        report = m.run_benchmark(cfg)
        assert report.n_samples == 5


# ============================================================================
# SampleResult / BenchmarkReport dataclass
# ============================================================================


def test_sample_result_roundtrip():
    r = m.SampleResult(
        sample_id="mmlu-0",
        benchmark="MMLU",
        mode="REAL",
        question="What is 2+2?",
        ground_truth="4",
        prediction="4",
        correct=True,
        score=1.0,
        latency_seconds=0.5,
        prompt_tokens=10,
        completion_tokens=5,
        cost_usd=0.0001,
        http_status=200,
    )
    d = r.to_dict()
    assert d["sample_id"] == "mmlu-0"
    assert d["correct"] is True
    assert d["mode"] == "REAL"


def test_benchmark_report_roundtrip():
    r = m.BenchmarkReport(
        provider="deterministic",
        model="deterministic-v0",
        n_samples=22,
        n_correct=10,
        accuracy=10 / 22,
        n_mock=22,
        n_real=0,
        total_latency_seconds=1.0,
        total_tokens=200,
        total_cost_usd=0.0,
        started_iso="2026-08-10T00-00-00Z",
        ended_iso="2026-08-10T00-00-01Z",
        per_benchmark={"MMLU": {"n_samples": 10, "n_correct": 5, "accuracy": 0.5, "n_mock": 10}},
    )
    d = r.to_dict()
    assert d["n_samples"] == 22
    assert d["per_benchmark"]["MMLU"]["n_correct"] == 5


# ============================================================================
# Popper self-test
# ============================================================================


def test_popper_self_test_all_pass():
    all_ok, n_pass, results = m.popper_self_test()
    assert all_ok is True
    assert n_pass >= 17


def test_popper_self_test_covers_required():
    _, _, results = m.popper_self_test()
    names = {r["name"] for r in results}
    assert "module_constants_present" in names
    assert "providers_complete" in names
    assert "borrowed_complete" in names
    assert "sample_index_has_22" in names
    assert "end_to_end_deterministic_benchmark" in names


# ============================================================================
# Chain delegation
# ============================================================================


def test_chain_delegate_returns_v1424_true():
    chain = m.chain_delegate()
    assert chain.get("v1424") is True


def test_chain_delegate_includes_v1034():
    chain = m.chain_delegate()
    assert "V1034" in chain


# ============================================================================
# CLI dispatch
# ============================================================================


def test_cli_version():
    rc = m.run_cli(["version"])
    assert rc == 0


def test_cli_help():
    rc = m.run_cli(["help"])
    assert rc == 0


def test_cli_meta():
    rc = m.run_cli(["meta"])
    assert rc == 0


def test_cli_meta_json():
    rc = m.run_cli(["meta", "--json"])
    assert rc == 0


def test_cli_demo():
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_cli_popper():
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_cli_chain():
    rc = m.run_cli(["chain"])
    assert rc == 0


def test_cli_list_samples():
    rc = m.run_cli(["list-samples"])
    assert rc == 0


def test_cli_list_samples_benchmark_filter():
    rc = m.run_cli(["list-samples", "--benchmark", "MMLU"])
    assert rc == 0


def test_cli_run_deterministic_smoke():
    """End-to-end: real CLI run with deterministic provider."""
    with tempfile.TemporaryDirectory() as td:
        rc = m.run_cli(
            [
                "run",
                "--provider", "deterministic",
                "--max-samples", "5",
                "--log-path", str(Path(td) / "r.jsonl"),
            ]
        )
        assert rc == 0


def test_cli_show_result():
    """End-to-end: show-result after run."""
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "r.jsonl"
        m.run_cli(
            [
                "run",
                "--provider", "deterministic",
                "--max-samples", "3",
                "--log-path", str(logp),
            ]
        )
        rc = m.run_cli(["show-result", "--sample-id", "mmlu-0", "--log-path", str(logp)])
        assert rc == 0


def test_cli_show_result_not_found():
    """show-result with no matching sample_id returns 0 with not-found message."""
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "r.jsonl"
        m.run_cli(
            [
                "run",
                "--provider", "deterministic",
                "--max-samples", "3",
                "--log-path", str(logp),
            ]
        )
        rc = m.run_cli(["show-result", "--sample-id", "nonexistent", "--log-path", str(logp)])
        assert rc == 0


def test_cli_report():
    """End-to-end: report aggregates the log."""
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "r.jsonl"
        m.run_cli(
            [
                "run",
                "--provider", "deterministic",
                "--max-samples", "5",
                "--log-path", str(logp),
            ]
        )
        rc = m.run_cli(["report", "--log-path", str(logp)])
        assert rc == 0


def test_cli_report_no_log():
    """report with no log file returns 0 with not-found message."""
    with tempfile.TemporaryDirectory() as td:
        rc = m.run_cli(["report", "--log-path", str(Path(td) / "missing.jsonl")])
        assert rc == 0


def test_cli_run_requires_base_url_for_real_provider():
    """run with provider=openai but no key + no base-url should fail with rc=1."""
    # We have to clear env first
    old_key = os.environ.pop(m.ENV_KEY, None)
    old_base = os.environ.pop(m.ENV_BASE, None)
    try:
        rc = m.run_cli(["run", "--provider", "openai", "--model", "gpt-4"])
        assert rc == 1
    finally:
        if old_key is not None:
            os.environ[m.ENV_KEY] = old_key
        if old_base is not None:
            os.environ[m.ENV_BASE] = old_base


def test_cli_unknown_command():
    rc = m.run_cli(["bogus"])
    assert rc == 1


# ============================================================================
# V3 philosophical guards
# ============================================================================


def test_v3_guards_include_required_philosophical_constraints():
    guards = list(m.V1424_V3_GUARDS)
    assert any("PHENOMENAL" in g for g in guards)
    assert any("ASI" in g for g in guards)
    assert any("HUMAN_LEVEL" in g for g in guards)
    assert any("ABSOLUTE" in g for g in guards)
    assert any("V1034_REPLACE" in g for g in guards)
    assert any("V1423_REPLACE" in g for g in guards)
    assert any("PRETEND_ASI" in g for g in guards)


def test_v3_guards_block_pretend_asi():
    """GUARD_BENCH_IS_NOT_PRETEND_ASI is the key guard (主 17:43 实事求是)."""
    guards_text = " ".join(m.V1424_V3_GUARDS)
    assert "PRETEND_ASI" in guards_text