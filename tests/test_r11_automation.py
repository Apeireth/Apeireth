"""R11 automation boundary tests: live-compatible transport vs offline deterministic path.

The default suite is network-independent.  ``test_opt_in_live_provider`` is only
run when the operator explicitly supplies R11_LIVE_PROVIDER, R11_LIVE_BASE_URL,
R11_LIVE_MODEL and R11_LIVE_CREDENTIAL.  The local HTTP server tests exercise the
same OpenAI-compatible wire path without claiming that a stub is a live model.
"""
from __future__ import annotations

import json
import os
import socket
import ssl
import sys
import threading
from dataclasses import replace
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from unittest import mock

import pytest

from apeireth.v1084_asi_real_llm_inference import (
    InferenceEngine,
    InferenceRequest,
    LLMEndpointConfig,
    LLMHTTPClient,
    OfflineMockEngine,
)
from apeireth.v1118_perf_optimizer_v01 import SubmoduleResultCache
from apeireth.v1130_asi_north_star_perf import DASHBOARD_DIMENSIONS
from apeireth.v1136_asi_v05_3dim_real_measurement import V1136Result
from apeireth.v1136_dashboard_render import render_v1136_dashboard


class _StubHTTPServer(ThreadingHTTPServer):
    """A real local socket serving a controlled OpenAI-compatible response."""

    allow_reuse_address = True

    def __init__(self, response_status: int, response_body: Any):
        super().__init__(("127.0.0.1", 0), _StubHandler)
        self.response_status = response_status
        self.response_body = response_body
        self.requests: list[dict[str, Any]] = []


class _StubHandler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:  # noqa: N802 - http.server contract
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length)
        try:
            payload = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            payload = {"_invalid": True}
        self.server.requests.append({  # type: ignore[attr-defined]
            "path": self.path,
            "payload": payload,
            "authorization": self.headers.get("Authorization"),
        })
        body = json.dumps(self.server.response_body).encode("utf-8")  # type: ignore[attr-defined]
        self.send_response(self.server.response_status)  # type: ignore[attr-defined]
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args: Any, **_kwargs: Any) -> None:
        return


@pytest.fixture
def stub_server():
    servers: list[_StubHTTPServer] = []

    def start(status: int = 200, body: Any | None = None) -> _StubHTTPServer:
        # 主 17:43 实事求是: port=0 让 OS 分配, 避免与早先未完全释放的 stub 冲突.
        # 当 OS 偶发分配到 0/保留端口导致 bind 失败时, 主动 retry 3 次缓解 flake.
        last_err: Exception | None = None
        for _ in range(3):
            try:
                server = _StubHTTPServer(
                    status, _valid_response() if body is None else body
                )
                break
            except OSError as e:  # pragma: no cover - 端口耗尽, 让 fixture 报清楚
                last_err = e
                continue
        else:  # pragma: no cover
            assert last_err is not None
            raise RuntimeError(
                f"stub_server: 3 次 bind 都失败, 最近一次: {last_err}"
            ) from last_err
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        server._r11_thread = thread  # type: ignore[attr-defined]
        servers.append(server)
        return server

    yield start
    for server in servers:
        server.shutdown()
        server.server_close()
        # 加长 join 超时, 避免 P95 拉长时 thread 还活着, 端口未释放
        # (上一行 server_close 之后 OS 通常已释放, 但 worker thread 可能还在读队列)
        server._r11_thread.join(timeout=5)  # type: ignore[attr-defined]


@pytest.fixture
def unreachable_endpoint():
    """Reserve an ephemeral TCP port without listening on it.

    Keeping the socket bound prevents another process from claiming the port while
    the test exercises a real connection-refused path.
    """
    guard = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    guard.bind(("127.0.0.1", 0))
    host, port = guard.getsockname()
    try:
        yield f"http://{host}:{port}/v1"
    finally:
        guard.close()


def _valid_response(content: str = "LIVE_COMPAT_OK") -> dict[str, Any]:
    return {
        "id": "r11-stub",
        "object": "chat.completion",
        "model": "stub-model",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content},
            "finish_reason": "stop",
        }],
        "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5},
    }


def _config(base_url: str, *, fallback: bool = False, **kwargs: Any) -> LLMEndpointConfig:
    return LLMEndpointConfig(
        name=kwargs.pop("name", "openai-compatible-stub"),
        base_url=base_url,
        api_key="r11-test-key",
        model_id=kwargs.pop("model_id", "gpt-4o-mini"),
        max_retries=kwargs.pop("max_retries", 0),
        retry_backoff_s=kwargs.pop("retry_backoff_s", 0.0),
        timeout_s=kwargs.pop("timeout_s", 1.0),
        mock_fallback=fallback,
        **kwargs,
    )


def _endpoint(server: _StubHTTPServer) -> str:
    return f"http://127.0.0.1:{server.server_address[1]}/v1"


class TestLiveCompatibleWirePath:
    @pytest.mark.parametrize(
        ("provider", "model"),
        (("openai", "gpt-4o-mini"), ("minimax", "MiniMax-M3")),
    )
    def test_local_real_http_path_is_not_offline(self, stub_server, provider: str, model: str):
        """Both provider names use the live OpenAI-compatible transport contract."""
        server = stub_server(body=_valid_response(f"{provider.upper()}_WIRE_OK"))
        cfg = _config(_endpoint(server), name=provider, model_id=model)
        response = InferenceEngine(cfg).infer(InferenceRequest(prompt="wire check"))

        assert response.status == "ok"
        assert response.text == f"{provider.upper()}_WIRE_OK"
        assert response.model_id == model
        assert response.endpoint == provider
        assert not response.error
        assert len(server.requests) == 1
        assert server.requests[0]["path"].endswith("/chat/completions")
        assert server.requests[0]["payload"]["model"] == model
        assert server.requests[0]["authorization"] == "Bearer r11-test-key"

    def test_http_503_is_an_explicit_provider_error(self, stub_server):
        server = stub_server(status=503, body={"error": {"message": "overloaded"}})
        client = LLMHTTPClient(_config(_endpoint(server)))
        data, _latency, status = client.call(InferenceRequest(prompt="503"))

        assert status == "http_error"
        assert data["_status"] == "http_error"
        assert "HTTPError 503" in data["_error"]

    def test_http_error_is_not_retried_as_a_transport_failure(self, stub_server):
        server = stub_server(status=503, body={"error": {"message": "overloaded"}})
        client = LLMHTTPClient(_config(_endpoint(server), max_retries=2))

        _data, _latency, status = client.call(InferenceRequest(prompt="do not retry 503"))

        assert status == "http_error"
        assert len(server.requests) == 1

    def test_ssl_error_is_an_explicit_transport_error(self):
        client = LLMHTTPClient(_config("https://provider.invalid/v1"))
        with mock.patch("urllib.request.urlopen", side_effect=ssl.SSLError("certificate verify failed")):
            data, _latency, status = client.call(InferenceRequest(prompt="ssl"))

        assert status == "transport_error"
        assert data["_status"] == "transport_error"
        assert "SSLError" in data["_error"]


class TestProviderDownAndOfflineBoundary:
    def test_provider_down_fallback_preserves_transport_evidence(self, unreachable_endpoint: str):
        cfg = _config(unreachable_endpoint, fallback=True)
        response = InferenceEngine(cfg).infer(InferenceRequest(prompt="provider down"))

        assert response.status == "mock"
        assert response.text.startswith("[MOCK-LLM-")
        assert "transport_error" in (response.error or "")
        assert "mock fallback used" in (response.error or "")

    def test_provider_down_without_fallback_is_not_success(self, unreachable_endpoint: str):
        cfg = _config(unreachable_endpoint, fallback=False)
        response = InferenceEngine(cfg).infer(InferenceRequest(prompt="provider down"))

        assert response.status == "transport_error"
        assert response.text == ""
        assert "transport_error" in (response.error or "")

    def test_force_mock_never_calls_live_client_and_is_reproducible(self):
        cfg = _config("https://provider.invalid/v1", fallback=False)
        engine = InferenceEngine(cfg, force_mock=True)
        request = InferenceRequest(prompt="same deterministic prompt")
        with mock.patch.object(engine.http, "call", side_effect=AssertionError("offline path touched HTTP")):
            first = engine.infer(request)
            second = engine.infer(request)

        assert first.status == second.status == "mock"
        assert first.text == second.text
        assert first.text.startswith("[MOCK-LLM-")
        assert first.error is None

    def test_offline_engine_stays_deterministic_without_wall_clock_assertion(self):
        engine = OfflineMockEngine(latency_ms=0.0)
        request = InferenceRequest(prompt="stable")
        first, _ = engine.call(request)
        second, _ = engine.call(request)
        assert first["_mock"] is second["_mock"] is True
        assert first["id"] == second["id"]
        assert first["choices"][0]["message"]["content"] == second["choices"][0]["message"]["content"]


class TestPartialAndVersionBoundaries:
    @pytest.mark.parametrize(
        "body",
        (
            {"id": "partial", "object": "chat.completion", "usage": {}},
            {"choices": [{"index": 0, "message": {"role": "assistant", "content": ""}}]},
            [],
        ),
        ids=("missing_choices", "empty_content", "non_object_json"),
    )
    def test_incomplete_http_200_is_partial_not_ok(self, stub_server, body: Any):
        server = stub_server(body=body)
        response = InferenceEngine(_config(_endpoint(server))).infer(InferenceRequest(prompt="partial"))

        assert response.status == "partial"
        assert response.text == ""
        assert "PartialResponse" in (response.error or "")

    def test_partial_response_can_fallback_but_keeps_reason(self, stub_server):
        server = stub_server(body={"choices": [{"index": 0, "message": {}}]})
        response = InferenceEngine(_config(_endpoint(server), fallback=True)).infer(
            InferenceRequest(prompt="partial fallback")
        )

        assert response.status == "mock"
        assert response.text.startswith("[MOCK-LLM-")
        assert "partial" in (response.error or "").lower()
        assert "mock fallback used" in (response.error or "")

    def test_provider_api_version_mismatch_is_explicit(self, stub_server):
        server = stub_server(body={**_valid_response(), "api_version": "legacy-2023"})
        cfg = _config(_endpoint(server), expected_api_version="2024-11-05")
        response = InferenceEngine(cfg).infer(InferenceRequest(prompt="version mismatch"))

        assert response.status == "version_mismatch"
        assert response.text == ""
        assert "ProviderVersionMismatch" in (response.error or "")
        assert "legacy-2023" in (response.error or "")

    def test_version_mismatch_fallback_is_still_marked_mock(self, stub_server):
        server = stub_server(body={**_valid_response(), "version": "0.1.0"})
        cfg = _config(_endpoint(server), fallback=True, expected_api_version="0.2.0")
        response = InferenceEngine(cfg).infer(InferenceRequest(prompt="version fallback"))

        assert response.status == "mock"
        assert "version_mismatch" in (response.error or "")
        assert "mock fallback used" in (response.error or "")


class TestOptInLiveProvider:
    def test_opt_in_live_provider(self):
        provider = os.getenv("R11_LIVE_PROVIDER", "").strip().lower()
        base_url = os.getenv("R11_LIVE_BASE_URL", "").strip()
        model = os.getenv("R11_LIVE_MODEL", "").strip()
        credential = os.getenv("R11_LIVE_CREDENTIAL", "").strip()
        if not all((provider, base_url, model, credential)):
            pytest.skip("set R11_LIVE_PROVIDER/BASE_URL/MODEL/CREDENTIAL to run live provider")
        if provider not in {"minimax", "openai"}:
            pytest.fail("R11_LIVE_PROVIDER must be minimax or openai")

        cfg = LLMEndpointConfig(
            name=provider,
            base_url=base_url,
            api_key=credential,
            model_id=model,
            timeout_s=10.0,
            max_retries=0,
            mock_fallback=False,
        )
        response = InferenceEngine(cfg).infer(InferenceRequest(prompt="Reply with R11_LIVE_OK", max_tokens=16))
        assert response.status == "ok"
        assert response.text.strip()
        assert not response.error
        assert response.endpoint == provider


def _dashboard_result() -> V1136Result:
    continuity = {"sub_scores": {"provider": 0.0}, "failures": ["provider_version_mismatch: expected 2024-11-05"], "implemented": 0, "failed": 1, "total": 1, "elapsed_seconds": 0.01}
    autonomy = {"sub_scores": {"router": 0.8}, "failures": [], "implemented": 1, "failed": 0, "total": 1, "elapsed_seconds": 0.02}
    transferability = {"sub_scores": {"backend": 0.9}, "failures": [], "implemented": 1, "failed": 0, "total": 1, "elapsed_seconds": 0.03}
    return V1136Result(
        continuity=0.6,
        autonomy=0.8,
        transferability=0.9,
        v05_total_v1136=0.81,
        v05_total_v1125=0.82,
        v04_score=0.85,
        delta_v05_total=-0.01,
        continuity_detail=continuity,
        autonomy_detail=autonomy,
        transferability_detail=transferability,
        chaos_report=None,
        v3_guards_pass=False,
        elapsed_seconds=0.1,
        timestamp=1.0,
    )


class TestDashboardRenderingBoundaries:
    def test_dashboard_reports_real_score_failures_and_dimensions(self):
        result = _dashboard_result()
        rendered = render_v1136_dashboard(result, cache=SubmoduleResultCache(maxsize=2))

        assert rendered.render_path == "v1136_real"
        assert rendered.v1136_score == result.v05_total_v1136
        assert rendered.dimensions == len(DASHBOARD_DIMENSIONS) == 18
        assert rendered.continuity_failures == 1
        assert "provider_version_mismatch" in rendered.markdown
        provider_row = next(line for line in rendered.markdown.splitlines() if "`provider`" in line)
        guard_line = next(line for line in rendered.markdown.splitlines() if "V3 guards_pass" in line)
        assert "failed" in provider_row.lower()
        assert guard_line.strip().endswith("False")

    def test_dashboard_cache_does_not_hide_changed_failure_state(self):
        first = _dashboard_result()
        second = replace(first, continuity_detail={**first.continuity_detail, "failures": ["provider_down"]})
        cache = SubmoduleResultCache(maxsize=2)
        cold = render_v1136_dashboard(first, cache=cache)
        changed = render_v1136_dashboard(second, cache=cache)

        assert cold.cache_hit is False
        assert changed.cache_hit is False
        assert "provider_down" in changed.markdown
        assert "provider_version_mismatch" not in changed.markdown


if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__, "-q"]))
