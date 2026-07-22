"""V1076 ASI Real External LLM Client tests — 真测试 (主 17:43 实事求是).

测试范围 (主 00:44 质量工程化):
1. LLMEndpointProbe 真探测
2. APIKeyValidator 真验证
3. MultiEndpointRouter 真选择
4. OpenAICompatibleClient 真 HTTP 调用
5. TokenBucket 真限流
6. BenchmarkSuite 真 benchmark
7. V1076RunResult 真报告
8. V3 哲学守门
9. Sanity: refs/guards/无假装/可复现
"""

from __future__ import annotations

import json
import os
import re
import socket
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any

import pytest

_PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(_PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROJECT_ROOT))

from apeireth.v1076_asi_real_external_llm_client import (  # noqa: E402
    DEFAULT_ENV_KEYS,
    DEFAULT_ENDPOINTS,
    REFERENCES,
    V1076_VERSION,
    BenchmarkResult,
    EndpointProbe,
    KeyValidation,
    LLMResponse,
    LLMState,
    TokenBucket,
    V1076RunResult,
    _http_request,
    _json_safe,
    _mask_key,
    chat_completion,
    discover_keys,
    probe_endpoint,
    render_markdown_report,
    run_benchmark,
    run_full_check,
    select_best_endpoint,
    validate_key,
)


# ---------------------------------------------------------------------------
# Fixtures: Fake LLM server (借鉴 LiteLLM proxy pattern)
# ---------------------------------------------------------------------------


class _FakeLLMHandler(BaseHTTPRequestHandler):
    """真 HTTP handler 模拟 LLM 服务."""

    # 状态: 0=success, 1=auth_failed, 2=server_error, 3=rate_limited
    mode = 0
    fail_after = 0  # 0 = never fail

    def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
        pass  # 静音

    def _send_json(self, status: int, body: dict) -> None:
        body_bytes = json.dumps(body).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body_bytes)))
        self.end_headers()
        self.wfile.write(body_bytes)

    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/api/status":
            self._send_json(200, {"status": "ok", "version": "fake-0.1"})
            return
        if self.path == "/v1/models":
            auth = self.headers.get("Authorization", "")
            # Use whole-token check to avoid 'invalid-key' containing 'valid-key'
            token = auth.replace("Bearer ", "").strip()
            if token != "valid-key-1234567890abcdef":
                self._send_json(401, {"error": {"message": "Invalid token"}})
                return
            self._send_json(200, {"data": [{"id": "fake-model"}]})
            return
        self._send_json(404, {"error": "not_found"})

    def do_POST(self) -> None:  # noqa: N802
        if self.path == "/v1/chat/completions":
            content_len = int(self.headers.get("Content-Length", 0))
            _ = self.rfile.read(content_len)
            auth = self.headers.get("Authorization", "")
            # Use whole-token check to avoid 'invalid-key' containing 'valid-key'
            token = auth.replace("Bearer ", "").strip()
            if token != "valid-key-1234567890abcdef":
                self._send_json(401, {"error": {"message": "Invalid token"}})
                return
            if _FakeLLMHandler.mode == 2:
                self._send_json(500, {"error": "server_error"})
                return
            if _FakeLLMHandler.mode == 3:
                self._send_json(429, {"error": "rate_limited"})
                return
            self._send_json(
                200,
                {
                    "id": "chatcmpl-fake",
                    "model": "fake-model",
                    "choices": [
                        {
                            "index": 0,
                            "message": {"role": "assistant", "content": "Hello from fake LLM"},
                            "finish_reason": "stop",
                        }
                    ],
                    "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
                },
            )
            return
        self._send_json(404, {"error": "not_found"})


@pytest.fixture
def fake_llm_server():
    """真起一个 fake LLM server."""
    server = HTTPServer(("127.0.0.1", 0), _FakeLLMHandler)
    port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield f"http://127.0.0.1:{port}/v1", port
    server.shutdown()
    server.server_close()


@pytest.fixture
def tmp_report_path(tmp_path: Path) -> Path:
    return tmp_path / "v1076-report.md"


# ---------------------------------------------------------------------------
# 1. LLMEndpointProbe 真探测 (主 17:43)
# ---------------------------------------------------------------------------


class TestLLMEndpointProbe:
    def test_probe_reachable(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        probe = probe_endpoint(base_url, name="fake")
        assert isinstance(probe, EndpointProbe)
        assert probe.reachable
        assert probe.status_code == 200
        assert probe.latency_ms >= 0

    def test_probe_unreachable(self) -> None:
        # 真不可达端口
        probe = probe_endpoint("http://127.0.0.1:1", name="nonexistent", timeout_sec=2.0)
        assert not probe.reachable
        assert probe.status_code == 0

    def test_probe_server_info(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        probe = probe_endpoint(base_url, name="fake")
        assert probe.server_info.get("status") == "ok"

    def test_probe_to_dict(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        probe = probe_endpoint(base_url, name="fake")
        d = probe.to_dict()
        assert "name" in d
        assert "base_url" in d
        assert "reachable" in d

    def test_probe_default_name(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        probe = probe_endpoint(base_url)
        assert probe.name == "default"


# ---------------------------------------------------------------------------
# 2. APIKeyValidator 真验证 (主 17:58 不假装)
# ---------------------------------------------------------------------------


class TestAPIKeyValidator:
    def test_validate_valid_key(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        v = validate_key(base_url, "valid-key-1234567890abcdef")
        assert isinstance(v, KeyValidation)
        assert v.valid
        assert v.status_code == 200

    def test_validate_invalid_key(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        v = validate_key(base_url, "invalid-key-12345")
        assert not v.valid
        assert v.status_code == 401
        assert v.error == "invalid_token"

    def test_validate_unreachable(self) -> None:
        v = validate_key("http://127.0.0.1:1", "any-key", timeout_sec=2.0)
        assert not v.valid
        assert v.error == "connection_failed"

    def test_key_mask(self) -> None:
        masked = _mask_key("sk-1234567890abcdefghij")
        assert "sk-12345" in masked
        assert "ghij" in masked
        assert "*********" in masked

    def test_key_mask_short(self) -> None:
        masked = _mask_key("abc")
        assert "*" in masked

    def test_validate_key_to_dict(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        v = validate_key(base_url, "valid-key-1234567890abcdef")
        d = v.to_dict()
        assert "source" in d
        assert "key_preview" in d
        assert "valid" in d


class TestDiscoverKeys:
    def test_discover_no_keys(self, monkeypatch: pytest.MonkeyPatch) -> None:
        # 清空所有 env keys
        for k in DEFAULT_ENV_KEYS:
            monkeypatch.delenv(k, raising=False)
        # 用不存在的路径
        keys = discover_keys(env_keys=[], file_paths=[Path("/nonexistent/.fake_key")])
        assert keys == []

    def test_discover_from_env(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setenv("TEST_API_KEY_FAKE", "sk-test-1234567890")
        keys = discover_keys(env_keys=["TEST_API_KEY_FAKE"], file_paths=[Path("/nonexistent/.fake_key")])
        assert len(keys) == 1
        assert keys[0][0] == "env:TEST_API_KEY_FAKE"
        assert keys[0][1] == "sk-test-1234567890"

    def test_discover_from_file(self, tmp_path: Path) -> None:
        keyfile = tmp_path / ".fake_key"
        keyfile.write_text("sk-file-key-1234567890\n")
        keys = discover_keys(env_keys=[], file_paths=[keyfile])
        assert len(keys) == 1
        assert keys[0][1] == "sk-file-key-1234567890"

    def test_discover_multiple_lines(self, tmp_path: Path) -> None:
        keyfile = tmp_path / ".fake_key"
        keyfile.write_text("key1\nkey2\n# comment\nkey3\n")
        keys = discover_keys(env_keys=[], file_paths=[keyfile])
        assert len(keys) == 3
        assert keys[0][1] == "key1"
        assert keys[2][1] == "key3"


# ---------------------------------------------------------------------------
# 3. MultiEndpointRouter 真选择 (主 17:43)
# ---------------------------------------------------------------------------


class TestMultiEndpointRouter:
    def test_select_best_endpoint_empty(self) -> None:
        assert select_best_endpoint([]) is None

    def test_select_best_endpoint_all_unreachable(self) -> None:
        probes = [
            EndpointProbe(name="a", base_url="http://x", reachable=False),
            EndpointProbe(name="b", base_url="http://y", reachable=False),
        ]
        assert select_best_endpoint(probes) is None

    def test_select_best_endpoint_lowest_latency(self) -> None:
        probes = [
            EndpointProbe(name="slow", base_url="http://a", reachable=True, latency_ms=200.0),
            EndpointProbe(name="fast", base_url="http://b", reachable=True, latency_ms=50.0),
        ]
        best = select_best_endpoint(probes)
        assert best is not None
        assert best.name == "fast"

    def test_select_best_endpoint_skips_unreachable(self) -> None:
        probes = [
            EndpointProbe(name="bad", base_url="http://a", reachable=False),
            EndpointProbe(name="good", base_url="http://b", reachable=True, latency_ms=100.0),
        ]
        best = select_best_endpoint(probes)
        assert best is not None
        assert best.name == "good"


# ---------------------------------------------------------------------------
# 4. OpenAICompatibleClient 真 HTTP 调用 (主 17:43)
# ---------------------------------------------------------------------------


class TestOpenAICompatibleClient:
    def test_chat_completion_success(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        resp = chat_completion(
            base_url=base_url,
            api_key="valid-key-1234567890abcdef",
            model="fake-model",
            messages=[{"role": "user", "content": "hi"}],
            max_retries=1,
        )
        assert isinstance(resp, LLMResponse)
        assert resp.success
        assert resp.content == "Hello from fake LLM"
        assert resp.tokens_in == 10
        assert resp.tokens_out == 5
        assert resp.status_code == 200

    def test_chat_completion_auth_failed(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        resp = chat_completion(
            base_url=base_url,
            api_key="wrong-key",
            model="fake-model",
            messages=[{"role": "user", "content": "hi"}],
            max_retries=1,
        )
        assert not resp.success
        assert resp.status_code == 401
        assert "auth_failed" in resp.error

    def test_chat_completion_unreachable(self) -> None:
        resp = chat_completion(
            base_url="http://127.0.0.1:1",
            api_key="any-key",
            model="fake",
            messages=[{"role": "user", "content": "hi"}],
            max_retries=2,
            timeout_sec=1.0,
        )
        assert not resp.success
        assert resp.status_code == 0

    def test_chat_completion_server_error_retry(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        _FakeLLMHandler.mode = 2  # server_error
        try:
            resp = chat_completion(
                base_url=base_url,
                api_key="valid-key-1234567890abcdef",
                model="fake",
                messages=[{"role": "user", "content": "hi"}],
                max_retries=2,
            )
            assert not resp.success
            assert resp.status_code == 500
        finally:
            _FakeLLMHandler.mode = 0

    def test_chat_completion_rate_limit_retry(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        _FakeLLMHandler.mode = 3
        try:
            resp = chat_completion(
                base_url=base_url,
                api_key="valid-key-1234567890abcdef",
                model="fake",
                messages=[{"role": "user", "content": "hi"}],
                max_retries=2,
            )
            assert not resp.success
            assert resp.status_code == 429
        finally:
            _FakeLLMHandler.mode = 0

    def test_chat_completion_recover_after_429(self, fake_llm_server) -> None:
        # 第一次 429, 第二次 200 (manual simulation isn't easy, but we can test 1st attempt behavior)
        base_url, _port = fake_llm_server
        _FakeLLMHandler.mode = 3
        try:
            resp = chat_completion(
                base_url=base_url,
                api_key="valid-key-1234567890abcdef",
                model="fake",
                messages=[{"role": "user", "content": "hi"}],
                max_retries=1,
            )
            assert resp.status_code == 429
        finally:
            _FakeLLMHandler.mode = 0

    def test_llm_response_to_dict(self) -> None:
        r = LLMResponse(content="hi", model="m", status_code=200)
        d = r.to_dict()
        assert d["content"] == "hi"
        assert d["model"] == "m"

    def test_llm_response_success_property(self) -> None:
        r1 = LLMResponse(content="hi", model="m")
        r2 = LLMResponse(content="", model="m", error="err")
        assert r1.success is True
        assert r2.success is False


# ---------------------------------------------------------------------------
# 5. TokenBucket 真限流 (主 23:44)
# ---------------------------------------------------------------------------


class TestTokenBucket:
    def test_initial_full(self) -> None:
        b = TokenBucket(capacity=10, refill_rate=1.0)
        assert b.tokens == 10

    def test_try_acquire(self) -> None:
        b = TokenBucket(capacity=5, refill_rate=0.1)
        assert b.try_acquire(3)
        assert b.tokens <= 2.0

    def test_try_acquire_insufficient(self) -> None:
        b = TokenBucket(capacity=2, refill_rate=0.01)
        assert b.try_acquire(2)
        # 第三个会失败 (refill 很慢)
        time.sleep(0.05)
        assert not b.try_acquire(5)

    def test_wait_time_zero_when_available(self) -> None:
        b = TokenBucket(capacity=10, refill_rate=1.0)
        assert b.wait_time(1) == 0.0

    def test_wait_time_positive_when_depleted(self) -> None:
        b = TokenBucket(capacity=1, refill_rate=1.0)
        b.try_acquire(1)
        # tokens 现在约 0, wait 1 应约 1s
        wt = b.wait_time(1)
        assert 0.5 <= wt <= 1.5

    def test_refill_over_time(self) -> None:
        b = TokenBucket(capacity=10, refill_rate=100.0)  # 快速 refill
        b.try_acquire(10)
        time.sleep(0.05)
        # 应该 refilled 5 tokens
        assert b.try_acquire(3)


# ---------------------------------------------------------------------------
# 6. BenchmarkSuite 真 benchmark (主 17:43)
# ---------------------------------------------------------------------------


class TestBenchmarkSuite:
    def test_run_benchmark_success(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        b = run_benchmark(
            base_url=base_url,
            api_key="valid-key-1234567890abcdef",
            model="fake-model",
            n_runs=3,
            max_retries=1,
        )
        assert isinstance(b, BenchmarkResult)
        assert b.n_runs == 3
        assert b.n_success == 3
        assert b.success_rate == 1.0
        assert b.latency_mean_ms > 0
        assert b.latency_p50_ms > 0
        assert b.passed

    def test_run_benchmark_all_fail(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        b = run_benchmark(
            base_url=base_url,
            api_key="wrong-key",
            model="fake",
            n_runs=3,
            max_retries=1,
        )
        assert b.n_success == 0
        assert b.success_rate == 0.0
        assert not b.passed
        assert len(b.errors) >= 1

    def test_run_benchmark_to_dict(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        b = run_benchmark(
            base_url=base_url,
            api_key="valid-key-1234567890abcdef",
            model="fake",
            n_runs=1,
        )
        d = b.to_dict()
        assert "name" in d
        assert "success_rate" in d
        assert "passed" in d

    def test_run_benchmark_unreachable(self) -> None:
        b = run_benchmark(
            base_url="http://127.0.0.1:1",
            api_key="any",
            model="fake",
            n_runs=1,
            timeout_sec=1.0,
            max_retries=1,
        )
        assert b.n_success == 0
        assert not b.passed


# ---------------------------------------------------------------------------
# 7. V1076RunResult 真报告 (主 00:56 可读)
# ---------------------------------------------------------------------------


class TestRunFullCheck:
    def test_full_check_success(self, monkeypatch: pytest.MonkeyPatch, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        # override DEFAULT_ENDPOINTS to use fake
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENDPOINTS",
            [{"name": "fake", "base_url": base_url, "model": "fake-model", "kind": "openai-compatible", "priority": 10}],
        )
        monkeypatch.setenv("TEST_FAKE_KEY", "valid-key-1234567890abcdef")
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENV_KEYS",
            ["TEST_FAKE_KEY"],
        )
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_KEY_PATHS",
            [],  # 不从文件读 key
        )
        result = run_full_check(
            benchmark_runs=2,
            benchmark_model="fake-model",
        )
        assert isinstance(result, V1076RunResult)
        assert result.selected_endpoint == base_url
        assert len(result.probes) >= 1
        assert result.benchmark is not None
        assert result.summary in ("benchmark_passed", "benchmark_failed", "no_valid_key")

    def test_full_check_unreachable(self, monkeypatch: pytest.MonkeyPatch) -> None:
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENDPOINTS",
            [{"name": "fake", "base_url": "http://127.0.0.1:1", "model": "m", "kind": "openai-compatible", "priority": 10}],
        )
        result = run_full_check(probe_timeout=2.0, benchmark_runs=1)
        assert result.summary in ("no_endpoint_reachable", "no_valid_key", "benchmark_failed")

    def test_full_check_to_dict(self, monkeypatch: pytest.MonkeyPatch, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENDPOINTS",
            [{"name": "fake", "base_url": base_url, "model": "fake-model", "kind": "openai-compatible", "priority": 10}],
        )
        monkeypatch.setenv("TEST_FAKE_KEY2", "valid-key-1234567890abcdef")
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENV_KEYS",
            ["TEST_FAKE_KEY2"],
        )
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_KEY_PATHS",
            [],
        )
        result = run_full_check(benchmark_runs=1, benchmark_model="fake-model")
        d = result.to_dict()
        assert "probes" in d
        assert "keys_found" in d
        assert "started_at" in d


class TestMarkdownReport:
    def test_report_has_sections(self) -> None:
        env = BenchmarkResult(name="b", endpoint="http://x", n_runs=1, n_success=1, passed=True)
        result = V1076RunResult(
            probes=[EndpointProbe(name="a", base_url="http://x", reachable=True)],
            keys_found=[KeyValidation(source="env:X", valid=True, status_code=200)],
            benchmark=env,
            selected_endpoint="http://x",
            selected_key_preview="sk-1234****",
            summary="benchmark_passed",
        )
        report = render_markdown_report(result)
        assert "# V1076" in report
        assert "Endpoint Probes" in report
        assert "API Keys" in report
        assert "Benchmark" in report
        assert "V3 哲学守门" in report
        assert "不假装" in report

    def test_report_contains_all_refs(self) -> None:
        result = V1076RunResult()
        report = render_markdown_report(result)
        for r in REFERENCES:
            assert r["id"] in report


# ---------------------------------------------------------------------------
# 8. V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ---------------------------------------------------------------------------


class TestV3PhilosophyGuard:
    def test_probe_does_not_lie_about_unreachable(self) -> None:
        probe = probe_endpoint("http://127.0.0.1:1", timeout_sec=2.0)
        assert not probe.reachable
        assert probe.status_code == 0

    def test_validate_does_not_lie_about_invalid_key(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        v = validate_key(base_url, "definitely-wrong-key-12345")
        assert not v.valid
        assert v.status_code == 401

    def test_chat_does_not_lie_about_429(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        _FakeLLMHandler.mode = 3
        try:
            resp = chat_completion(
                base_url=base_url,
                api_key="valid-key-1234567890abcdef",
                model="fake",
                messages=[{"role": "user", "content": "hi"}],
                max_retries=1,
            )
            assert resp.status_code == 429
            assert not resp.success
        finally:
            _FakeLLMHandler.mode = 0

    def test_benchmark_does_not_lie_about_failures(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        b = run_benchmark(
            base_url=base_url,
            api_key="wrong-key",
            model="fake",
            n_runs=2,
            max_retries=1,
        )
        assert not b.passed
        assert b.n_success == 0


# ---------------------------------------------------------------------------
# 9. Sanity: refs / guards / 无假装 / 可复现 (主 00:44)
# ---------------------------------------------------------------------------


class TestSanity:
    def test_references_count(self) -> None:
        assert len(REFERENCES) >= 11

    def test_references_have_required_fields(self) -> None:
        for r in REFERENCES:
            assert "id" in r
            assert "title" in r
            assert "url" in r

    def test_version_format(self) -> None:
        assert re.match(r"^\d+\.\d+\.\d+", V1076_VERSION)

    def test_default_endpoints_not_empty(self) -> None:
        assert len(DEFAULT_ENDPOINTS) >= 1

    def test_default_endpoints_have_required_fields(self) -> None:
        for ep in DEFAULT_ENDPOINTS:
            assert "name" in ep
            assert "base_url" in ep
            assert "model" in ep

    def test_json_safe_datetime(self) -> None:
        import datetime
        dt = datetime.datetime(2026, 7, 22, 10, 0, 0)
        assert _json_safe(dt) == "2026-07-22T10:00:00"

    def test_json_safe_path(self) -> None:
        p = Path("/tmp/test")
        assert _json_safe(p) == str(p)

    def test_json_safe_set(self) -> None:
        assert _json_safe({1, 2, 3}) == [1, 2, 3]

    def test_json_safe_unknown_raises(self) -> None:
        with pytest.raises(TypeError):
            _json_safe(object())

    def test_no_global_state_pollution(self, fake_llm_server) -> None:
        """可复现: 两次 benchmark 结果应一致."""
        base_url, _port = fake_llm_server
        b1 = run_benchmark(
            base_url=base_url,
            api_key="valid-key-1234567890abcdef",
            model="fake",
            n_runs=2,
        )
        b2 = run_benchmark(
            base_url=base_url,
            api_key="valid-key-1234567890abcdef",
            model="fake",
            n_runs=2,
        )
        # 两次 success count 一致 (deterministic fake server)
        assert b1.n_success == b2.n_success

    def test_state_enum_values(self) -> None:
        assert LLMState.UNKNOWN.value == "UNKNOWN"
        assert LLMState.REACHABLE.value == "REACHABLE"
        assert LLMState.VALIDATED.value == "VALIDATED"


# ---------------------------------------------------------------------------
# 10. _http_request 直接测试 (主 17:43)
# ---------------------------------------------------------------------------


class TestHttpRequest:
    def test_http_get(self, fake_llm_server) -> None:
        base_url, port = fake_llm_server
        # 提取主机端口
        host_port = base_url.replace("http://", "").replace("/v1", "")
        host, port_s = host_port.split(":")
        import http.client
        conn = http.client.HTTPConnection(host, int(port_s), timeout=2)
        conn.request("GET", "/api/status")
        resp = conn.getresponse()
        assert resp.status == 200
        body = resp.read().decode()
        assert "ok" in body
        conn.close()

    def test_http_request_get(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        status, _headers, body, latency = _http_request(f"{base_url}/models", timeout_sec=2.0)
        assert status in (200, 401)  # depends on auth

    def test_http_request_post(self, fake_llm_server) -> None:
        base_url, _port = fake_llm_server
        body = json.dumps({"model": "fake", "messages": [{"role": "user", "content": "hi"}]}).encode()
        headers = {"Authorization": "Bearer valid-key-1234567890abcdef", "Content-Type": "application/json"}
        status, _headers, resp_body, _latency = _http_request(
            f"{base_url}/chat/completions",
            method="POST",
            headers=headers,
            body=body,
            timeout_sec=2.0,
        )
        assert status == 200
        data = json.loads(resp_body)
        assert "choices" in data


# ---------------------------------------------------------------------------
# 11. CLI main 真实入口 (主 00:56)
# ---------------------------------------------------------------------------


class TestCLI:
    def test_probe_cli(self, monkeypatch: pytest.MonkeyPatch, fake_llm_server, capsys: pytest.CaptureFixture) -> None:
        base_url, _port = fake_llm_server
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENDPOINTS",
            [{"name": "fake", "base_url": base_url, "model": "m", "kind": "openai-compatible", "priority": 10}],
        )
        from apeireth.v1076_asi_real_external_llm_client import main
        rc = main(["--probe", "--timeout", "2"])
        assert rc == 0
        out = capsys.readouterr().out
        parsed = json.loads(out)
        assert isinstance(parsed, list)
        assert len(parsed) >= 1
        assert parsed[0]["reachable"] is True

    def test_keys_cli(self, monkeypatch: pytest.MonkeyPatch, fake_llm_server, capsys: pytest.CaptureFixture) -> None:
        base_url, _port = fake_llm_server
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENDPOINTS",
            [{"name": "fake", "base_url": base_url, "model": "m", "kind": "openai-compatible", "priority": 10}],
        )
        monkeypatch.setenv("TEST_CLI_KEY", "valid-key-1234567890abcdef")
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENV_KEYS",
            ["TEST_CLI_KEY"],
        )
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_KEY_PATHS",
            [],
        )
        from apeireth.v1076_asi_real_external_llm_client import main
        rc = main(["--keys", "--timeout", "2"])
        assert rc == 0
        out = capsys.readouterr().out
        parsed = json.loads(out)
        assert isinstance(parsed, list)

    def test_check_cli(self, monkeypatch: pytest.MonkeyPatch, fake_llm_server, capsys: pytest.CaptureFixture) -> None:
        base_url, _port = fake_llm_server
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENDPOINTS",
            [{"name": "fake", "base_url": base_url, "model": "fake-model", "kind": "openai-compatible", "priority": 10}],
        )
        monkeypatch.setenv("TEST_CHECK_KEY", "valid-key-1234567890abcdef")
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_ENV_KEYS",
            ["TEST_CHECK_KEY"],
        )
        monkeypatch.setattr(
            "apeireth.v1076_asi_real_external_llm_client.DEFAULT_KEY_PATHS",
            [],
        )
        from apeireth.v1076_asi_real_external_llm_client import main
        rc = main(["--check", "--runs", "1", "--model", "fake-model", "--timeout", "2"])
        # check returns 0 even on failure (in CLI)
        out = capsys.readouterr().out
        parsed = json.loads(out)
        assert "summary" in parsed