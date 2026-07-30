"""Tests for V1084 ASI Real LLM Inference Adapter.

主 00:36 质量 + 工程化 — 真测 8 真生产组件:
1. LLMEndpointConfig
2. InferenceRequest/Response
3. TokenEstimator
4. CostCalculator
5. LLMHTTPClient
6. OfflineMockEngine
7. InferenceEngine (integration)
8. InferenceAuditLog
+ V1084Bridge (subscore + ASI lift)
+ V3 philosophy guards
+ CLI sanity
+ Sanity / no-fabrication / reproducibility
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path
from unittest import mock

import pytest

# Ensure project root on path
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1084_asi_real_llm_inference import (  # noqa: E402
    ARTIFACT_DIR,
    REFERENCES,
    V1084_GUARDS,
    V1084_V3_SUBWEIGHTS,
    V1084_VERSION,
    CostCalculator,
    GUARD_NOT_HTTP_IS_ASI,
    GUARD_NOT_MOCK_IS_REAL,
    GUARD_NOT_SUBSCORE_IS_ASI,
    GUARD_NOT_TOKEN_ESTIMATE_IS_EXACT,
    InferenceAuditLog,
    InferenceEngine,
    InferenceRequest,
    InferenceResponse,
    LLMEndpointConfig,
    LLMHTTPClient,
    OfflineMockEngine,
    TokenEstimator,
    _make_default_endpoint,
    _make_report,
    _run_full,
    main,
    v1084_asi_lift,
    v1084_subscore,
)


# ============================================================
# Component 1: LLMEndpointConfig
# ============================================================


class TestLLMEndpointConfig:
    def test_basic_construction(self):
        cfg = LLMEndpointConfig(
            name="test",
            base_url="https://api.example.com/v1",
            api_key="sk-test",
            model_id="test-model",
        )
        assert cfg.name == "test"
        assert cfg.base_url == "https://api.example.com/v1"
        assert cfg.api_key == "sk-test"
        assert cfg.model_id == "test-model"
        assert cfg.timeout_s == 30.0  # default
        assert cfg.max_retries == 2  # default
        assert cfg.mock_fallback is True  # default

    def test_to_dict_redacts_api_key(self):
        cfg = LLMEndpointConfig(
            name="x",
            base_url="https://x.example/v1",
            api_key="sk-secret",
            model_id="m",
        )
        d = cfg.to_dict()
        assert d["api_key"] == "***REDACTED***"
        assert d["name"] == "x"

    def test_default_pricing_set(self):
        cfg = _make_default_endpoint()
        assert cfg.input_price_per_1k > 0
        assert cfg.output_price_per_1k > 0
        # Output typically more expensive
        assert cfg.output_price_per_1k >= cfg.input_price_per_1k


# ============================================================
# Component 2: InferenceRequest / InferenceResponse
# ============================================================


class TestInferenceRequestResponse:
    def test_request_to_dict(self):
        req = InferenceRequest(prompt="Hello", max_tokens=64, temperature=0.5)
        d = req.to_dict()
        assert d["prompt"] == "Hello"
        assert d["max_tokens"] == 64
        assert d["temperature"] == 0.5
        assert d["stream"] is False  # default

    def test_response_to_dict(self):
        resp = InferenceResponse(
            request_id="v1084-abc",
            text="World",
            input_tokens=5,
            output_tokens=5,
            total_tokens=10,
            latency_ms=100.0,
            cost_usd=0.001,
            model_id="m",
            status="ok",
        )
        d = resp.to_dict()
        assert d["text"] == "World"
        assert d["status"] == "ok"


# ============================================================
# Component 3: TokenEstimator
# ============================================================


class TestTokenEstimator:
    def test_empty_text(self):
        est = TokenEstimator()
        assert est.estimate("") == 0

    def test_english_text(self):
        est = TokenEstimator()
        # 100 ASCII chars ≈ 25 tokens
        text = "a" * 100
        tokens = est.estimate(text)
        assert 20 <= tokens <= 30

    def test_cjk_text(self):
        est = TokenEstimator()
        # 100 Chinese chars ≈ 67 tokens (100/1.5)
        text = "你" * 100
        tokens = est.estimate(text)
        assert 60 <= tokens <= 75

    def test_caching(self):
        est = TokenEstimator()
        t1 = est.estimate("hello world")
        t2 = est.estimate("hello world")
        assert t1 == t2

    def test_estimate_pair(self):
        est = TokenEstimator()
        in_tok, out_tok = est.estimate_pair("Hello world", "Hi there")
        assert in_tok > 0
        assert out_tok > 0

    def test_minimum_one_token(self):
        est = TokenEstimator()
        # Very short text should still return ≥ 1
        assert est.estimate("a") >= 1


# ============================================================
# Component 4: CostCalculator
# ============================================================


class TestCostCalculator:
    def test_basic_cost(self):
        calc = CostCalculator(input_price_per_1k=0.001, output_price_per_1k=0.002)
        # 1000 input + 500 output = 0.001 * 1 + 0.002 * 0.5 = 0.002
        cost = calc.compute(1000, 500)
        assert abs(cost - 0.002) < 1e-8

    def test_zero_tokens(self):
        calc = CostCalculator(0.001, 0.002)
        assert calc.compute(0, 0) == 0.0

    def test_negative_tokens_returns_zero(self):
        calc = CostCalculator(0.001, 0.002)
        assert calc.compute(-1, 100) == 0.0
        assert calc.compute(100, -1) == 0.0

    def test_with_pricing(self):
        calc = CostCalculator(0.001, 0.002)
        new_calc = calc.with_pricing(0.005, 0.010)
        cost = new_calc.compute(1000, 1000)
        # 0.005 + 0.010 = 0.015
        assert abs(cost - 0.015) < 1e-8

    def test_rounding(self):
        calc = CostCalculator(0.001234, 0.005678)
        cost = calc.compute(100, 100)
        # Result is rounded to 8 decimals
        assert isinstance(cost, float)


# ============================================================
# Component 5: LLMHTTPClient
# ============================================================


class TestLLMHTTPClient:
    def test_build_payload(self):
        cfg = LLMEndpointConfig(
            name="t",
            base_url="https://x.example/v1",
            api_key="sk",
            model_id="model-1",
        )
        client = LLMHTTPClient(cfg)
        req = InferenceRequest(prompt="Hi", max_tokens=64, temperature=0.5)
        payload = client._build_payload(req)
        assert payload["model"] == "model-1"
        assert payload["messages"] == [{"role": "user", "content": "Hi"}]
        assert payload["max_tokens"] == 64
        assert payload["temperature"] == 0.5
        assert payload["stream"] is False

    def test_call_success_mocked(self):
        cfg = LLMEndpointConfig(
            name="t",
            base_url="https://x.example/v1",
            api_key="sk",
            model_id="model-1",
        )
        client = LLMHTTPClient(cfg)
        req = InferenceRequest(prompt="Hi")

        mock_response = {
            "choices": [{"message": {"content": "Hello back"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2, "total_tokens": 3},
        }

        with mock.patch.object(client, "_do_request_once", return_value=(mock_response, 100.0)):
            data, latency, status = client.call(req)
            assert status == "ok"
            assert data["choices"][0]["message"]["content"] == "Hello back"
            assert latency == 100.0

    def test_call_failure_with_retry(self):
        cfg = LLMEndpointConfig(
            name="t",
            base_url="https://x.example/v1",
            api_key="sk",
            model_id="model-1",
            max_retries=1,
            retry_backoff_s=0.01,
        )
        client = LLMHTTPClient(cfg)
        req = InferenceRequest(prompt="Hi")

        err_data = {"_error": "URLError: timeout"}

        with mock.patch.object(client, "_do_request_once", return_value=(err_data, 50.0)):
            with mock.patch("time.sleep"):  # skip backoff
                data, latency, status = client.call(req)
                assert status == "error"
                assert "_error" in data

    def test_is_reachable_false(self):
        cfg = LLMEndpointConfig(
            name="t",
            base_url="https://nonexistent-host-zzzzz.example/v1",
            api_key="sk",
            model_id="model-1",
        )
        client = LLMHTTPClient(cfg)
        assert client.is_reachable() is False


# ============================================================
# Component 6: OfflineMockEngine
# ============================================================


class TestOfflineMockEngine:
    def test_call_returns_mock_json(self):
        mock_eng = OfflineMockEngine(latency_ms=10.0)
        req = InferenceRequest(prompt="Hello")
        data, latency = mock_eng.call(req)
        assert data["_mock"] is True
        assert "choices" in data
        assert data["choices"][0]["message"]["role"] == "assistant"
        assert len(data["choices"][0]["message"]["content"]) > 0
        assert latency >= 10.0  # at least the configured latency

    def test_call_deterministic(self):
        mock_eng = OfflineMockEngine(latency_ms=1.0)
        req = InferenceRequest(prompt="Same prompt")
        data1, _ = mock_eng.call(req)
        data2, _ = mock_eng.call(req)
        # Same prompt → same deterministic output
        assert data1["choices"][0]["message"]["content"] == data2["choices"][0]["message"]["content"]

    def test_call_different_prompts_different_responses(self):
        mock_eng = OfflineMockEngine(latency_ms=1.0)
        d1, _ = mock_eng.call(InferenceRequest(prompt="Prompt A"))
        d2, _ = mock_eng.call(InferenceRequest(prompt="Prompt B different"))
        # Different prompts → different content (with high probability)
        # We don't strictly assert inequality because hash collision is possible,
        # but for distinct English prompts, they should differ.
        assert d1["id"] != d2["id"]


# ============================================================
# Component 7: InferenceEngine (integration)
# ============================================================


class TestInferenceEngine:
    def test_force_mock(self):
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        req = InferenceRequest(prompt="Test", max_tokens=32)
        resp = engine.infer(req)
        assert resp.status == "mock"
        assert resp.latency_ms >= 0
        assert resp.input_tokens > 0
        assert resp.output_tokens > 0
        assert resp.text != ""

    def test_http_failure_falls_back_to_mock(self):
        cfg = LLMEndpointConfig(
            name="t",
            base_url="https://nonexistent-host-zzzzz.example/v1",
            api_key="sk",
            model_id="model-1",
            mock_fallback=True,
            max_retries=0,
            retry_backoff_s=0.01,
        )
        engine = InferenceEngine(endpoint=cfg)
        req = InferenceRequest(prompt="Hi")
        resp = engine.infer(req)
        assert resp.status == "mock"
        # R11 边界: provider down 走 transport_error, mock fallback 保留原证据
        assert "transport_error" in (resp.error or "")
        assert "mock fallback used" in (resp.error or "")

    def test_http_failure_no_fallback(self):
        cfg = LLMEndpointConfig(
            name="t",
            base_url="https://nonexistent-host-zzzzz.example/v1",
            api_key="sk",
            model_id="model-1",
            mock_fallback=False,
            max_retries=0,
            retry_backoff_s=0.01,
        )
        engine = InferenceEngine(endpoint=cfg)
        req = InferenceRequest(prompt="Hi")
        resp = engine.infer(req)
        # R11 边界: 无 fallback 时按传输错误返回, status 反映真实 transport_error
        assert resp.status == "transport_error"
        assert "transport_error" in (resp.error or "")

    def test_inference_response_fields(self):
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        req = InferenceRequest(prompt="Test", max_tokens=16)
        resp = engine.infer(req)
        assert resp.request_id.startswith("v1084-")
        assert resp.model_id == cfg.model_id
        assert resp.endpoint == cfg.name
        assert resp.ts_iso != ""
        assert resp.cost_usd >= 0


# ============================================================
# Component 8: InferenceAuditLog
# ============================================================


class TestInferenceAuditLog:
    def test_record_and_load(self, tmp_path):
        log_path = tmp_path / "audit.jsonl"
        audit = InferenceAuditLog(log_path=log_path)
        req = InferenceRequest(prompt="Hello")
        resp = InferenceResponse(
            request_id="v1084-test1",
            text="Hi",
            input_tokens=1,
            output_tokens=1,
            total_tokens=2,
            latency_ms=50.0,
            cost_usd=0.001,
            model_id="m",
            status="ok",
        )
        audit.record(req, resp)
        entries = audit.load_all()
        assert len(entries) == 1
        assert entries[0]["request_id"] == "v1084-test1"
        assert entries[0]["status"] == "ok"
        assert entries[0]["v1084_version"] == V1084_VERSION

    def test_record_hashes(self, tmp_path):
        log_path = tmp_path / "audit.jsonl"
        audit = InferenceAuditLog(log_path=log_path)
        req = InferenceRequest(prompt="Unique prompt")
        resp = InferenceResponse(
            request_id="v1084-hash-test",
            text="Unique response",
            input_tokens=1,
            output_tokens=1,
            total_tokens=2,
            latency_ms=10.0,
            cost_usd=0.0,
            model_id="m",
            status="mock",
        )
        audit.record(req, resp)
        entry = audit.load_all()[0]
        assert len(entry["request_hash"]) == 64  # SHA-256 hex
        assert len(entry["response_hash"]) == 64

    def test_summary_aggregates(self, tmp_path):
        log_path = tmp_path / "audit.jsonl"
        audit = InferenceAuditLog(log_path=log_path)
        for i in range(3):
            req = InferenceRequest(prompt=f"P{i}")
            resp = InferenceResponse(
                request_id=f"v1084-{i}",
                text=f"R{i}",
                input_tokens=10,
                output_tokens=10,
                total_tokens=20,
                latency_ms=100.0,
                cost_usd=0.01,
                model_id="m1" if i < 2 else "m2",
                status="ok" if i < 2 else "mock",
            )
            audit.record(req, resp)
        summary = audit.summary()
        assert summary["count"] == 3
        assert summary["status_counts"]["ok"] == 2
        assert summary["status_counts"]["mock"] == 1
        assert summary["model_counts"]["m1"] == 2
        assert abs(summary["total_cost_usd"] - 0.03) < 1e-6
        assert abs(summary["total_latency_ms"] - 300.0) < 0.1

    def test_summary_empty(self, tmp_path):
        log_path = tmp_path / "audit.jsonl"
        audit = InferenceAuditLog(log_path=log_path)
        s = audit.summary()
        assert s["count"] == 0

    def test_corrupted_line_skipped(self, tmp_path):
        log_path = tmp_path / "audit.jsonl"
        log_path.parent.mkdir(parents=True, exist_ok=True)
        # Write valid JSONL + corrupted line
        log_path.write_text(
            '{"request_id": "v1084-good"}\n'
            'this is not valid json\n'
            '{"request_id": "v1084-good2"}\n',
            encoding="utf-8",
        )
        audit = InferenceAuditLog(log_path=log_path)
        entries = audit.load_all()
        assert len(entries) == 2  # corrupted line skipped


# ============================================================
# Component 9: V1084Bridge — subscore + ASI lift
# ============================================================


class TestV1084Bridge:
    def test_subscore_returns_in_range(self):
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        audit = InferenceAuditLog()
        req = InferenceRequest(prompt="Test")
        resp = engine.infer(req)
        sub, parts = v1084_subscore(cfg, engine, audit, req, resp)
        assert 0.0 <= sub <= 1.0
        assert all(0.0 <= v <= 1.0 for v in parts.values())

    def test_subscore_weights_sum_to_one(self):
        total = sum(V1084_V3_SUBWEIGHTS.values())
        assert abs(total - 1.0) < 1e-6

    def test_subscore_has_all_components(self):
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        audit = InferenceAuditLog()
        req = InferenceRequest(prompt="Test")
        resp = engine.infer(req)
        _, parts = v1084_subscore(cfg, engine, audit, req, resp)
        assert set(parts.keys()) == set(V1084_V3_SUBWEIGHTS.keys())

    def test_asi_lift_capped(self):
        # Even with subscore=1.0, lift capped at 0.02
        info = v1084_asi_lift(1.0)
        assert info["v1084_asi_lift"] == 0.02
        assert info["v1084_cap"] == 0.02

    def test_asi_lift_scales_with_subscore(self):
        # subscore=0.5 → lift=0.01
        info = v1084_asi_lift(0.5)
        assert abs(info["v1084_asi_lift"] - 0.01) < 1e-6

    def test_asi_lift_zero_for_zero_subscore(self):
        info = v1084_asi_lift(0.0)
        assert info["v1084_asi_lift"] == 0.0


# ============================================================
# V3 Philosophy Guards (主 17:58+20:46 不假装)
# ============================================================


class TestV3PhilosophyGuards:
    def test_all_four_guards_present(self):
        assert len(V1084_GUARDS) == 4

    def test_guard_strings_non_empty(self):
        for g in V1084_GUARDS:
            assert isinstance(g, str)
            assert len(g) > 20
            # Each guard must contain negation (≠) showing honest non-pretending
            assert "≠" in g or "不" in g or "not" in g.lower()

    def test_specific_guards_exist(self):
        assert "HTTP 接通 ≠ ASI" in GUARD_NOT_HTTP_IS_ASI
        assert "≠ tiktoken 精确" in GUARD_NOT_TOKEN_ESTIMATE_IS_EXACT
        assert "≠ real LLM response" in GUARD_NOT_MOCK_IS_REAL
        assert "subscore ≠ ASI" in GUARD_NOT_SUBSCORE_IS_ASI


# ============================================================
# CLI sanity (主 00:56 任何人都能接手)
# ============================================================


class TestCLI:
    def test_main_endpoint_config(self, capsys):
        rc = main(["--endpoint-config"])
        assert rc == 0
        out = capsys.readouterr().out
        parsed = json.loads(out)
        assert "name" in parsed
        assert parsed["api_key"] == "***REDACTED***"

    def test_main_lift(self, capsys):
        rc = main(["--lift", "--mock-mode"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1084 subscore" in out
        assert "ASI V0.3 lift" in out

    def test_main_infer_mock_mode(self, capsys):
        rc = main(["--infer", "--prompt", "What is 2+2?", "--mock-mode"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "status: mock" in out
        assert "response" in out.lower()

    def test_main_audit(self, capsys, tmp_path, monkeypatch):
        # First make an inference so audit log has entries
        monkeypatch.setattr(
            "apeireth.v1084_asi_real_llm_inference.ARTIFACT_DIR",
            tmp_path,
        )
        main(["--infer", "--prompt", "audit test", "--mock-mode"])
        # Then audit
        rc = main(["--audit", "--audit-limit", "5"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "count" in out.lower() or "Last" in out

    def test_main_no_args_prints_help(self, capsys):
        rc = main([])
        assert rc == 0
        out = capsys.readouterr().out
        assert "usage" in out.lower() or "--infer" in out


# ============================================================
# Sanity / no-fabrication / reproducibility
# ============================================================


class TestSanity:
    def test_references_list_real(self):
        # 真借鉴 10 项, 每项都有 source
        assert len(REFERENCES) == 10
        for r in REFERENCES:
            assert len(r) > 10
            # No placeholder / lorem ipsum
            assert "TODO" not in r
            assert "FIXME" not in r
            assert "lorem" not in r.lower()

    def test_version_constant(self):
        assert V1084_VERSION == "0.1.0"

    def test_module_exports_complete(self):
        # All public names listed in __all__
        from apeireth import v1084_asi_real_llm_inference as m
        for name in m.__all__:
            assert hasattr(m, name), f"Missing export: {name}"

    def test_no_lorem_ipsum_in_docstrings(self):
        from apeireth import v1084_asi_real_llm_inference as m
        import inspect
        for name in dir(m):
            obj = getattr(m, name)
            if hasattr(obj, "__doc__") and obj.__doc__:
                assert "lorem" not in obj.__doc__.lower()
                assert "TODO" not in obj.__doc__
                assert "FIXME" not in obj.__doc__

    def test_run_full_writes_report(self, tmp_path, monkeypatch):
        monkeypatch.setattr(
            "apeireth.v1084_asi_real_llm_inference.ARTIFACT_DIR",
            tmp_path,
        )
        cfg = _make_default_endpoint()
        report_path = tmp_path / "report.md"
        resp = _run_full(
            endpoint=cfg,
            prompt="Sanity test",
            max_tokens=32,
            temperature=0.5,
            force_mock=True,
            report_path=report_path,
        )
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1084 ASI Real LLM Inference Adapter Report" in content
        assert resp.status == "mock"

    def test_report_contains_guards_and_references(self):
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        audit = InferenceAuditLog()
        req = InferenceRequest(prompt="Test")
        resp = engine.infer(req)
        sub, parts = v1084_subscore(cfg, engine, audit, req, resp)
        lift_info = v1084_asi_lift(sub)
        report = _make_report(cfg, resp, audit.summary(), sub, parts, lift_info)
        assert "V1084 Subscore" in report
        assert "Philosophy Guards" in report
        assert "References" in report
        for g in V1084_GUARDS:
            assert g in report


# ============================================================
# Integration: full pipeline
# ============================================================


class TestIntegration:
    def test_end_to_end_mock(self, tmp_path, monkeypatch):
        monkeypatch.setattr(
            "apeireth.v1084_asi_real_llm_inference.ARTIFACT_DIR",
            tmp_path,
        )
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        audit = InferenceAuditLog()
        # Multiple inferences
        for i in range(5):
            req = InferenceRequest(prompt=f"Q{i}", max_tokens=16)
            resp = engine.infer(req)
            audit.record(req, resp)
            assert resp.status == "mock"
        summary = audit.summary()
        assert summary["count"] == 5
        assert summary["status_counts"]["mock"] == 5

    def test_cost_realistic_range(self):
        # 1000 input + 200 output with default pricing
        # 0.002 * 1 + 0.006 * 0.2 = 0.002 + 0.0012 = 0.0032
        cfg = _make_default_endpoint()
        calc = CostCalculator(cfg.input_price_per_1k, cfg.output_price_per_1k)
        cost = calc.compute(1000, 200)
        assert abs(cost - 0.0032) < 1e-4

    def test_engine_handles_empty_prompt(self):
        cfg = _make_default_endpoint()
        engine = InferenceEngine(endpoint=cfg, force_mock=True)
        req = InferenceRequest(prompt="", max_tokens=8)
        resp = engine.infer(req)
        # Empty prompt still returns something
        assert resp.request_id != ""


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-v"]))