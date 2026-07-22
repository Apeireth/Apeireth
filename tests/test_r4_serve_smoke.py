"""R4-BE-03 apeireth serve OpenAI 兼容服务烟测 (主 21:15 用户面第二面).

≥4 烟测, http.client in-process 启动 serve_in_background:
- GET /health → 200 + status=alive
- GET /v1/models → 200 + data array
- POST /v1/chat/completions → 200 + choices 非空
- POST /v1/chat/completions stream=true → 200 + SSE [DONE]
+ bonus: 不暴露 ASI / V1074 / philosophy_guard 字段

边界: 不动 llm_kernel.py, 不写 systemd / docker.
"""
from __future__ import annotations

import http.client
import json
import socket
import sys
import threading
import time
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.serve import SERVE_VERSION, serve_in_background  # noqa: E402


# -----------------------------------------------------------------------------
# Fixtures: in-process server bound to free port
# -----------------------------------------------------------------------------


def _wait_port(host: str, port: int, timeout_s: float = 5.0) -> bool:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            with socket.create_connection((host, port), timeout=0.2):
                return True
        except OSError:
            time.sleep(0.05)
    return False


@pytest.fixture
def live_server():
    """R4-BE-03 in-process server (auto free port + clean shutdown)."""
    server, thread, port = serve_in_background(port=0)
    host = "127.0.0.1"
    assert _wait_port(host, port), f"server failed to bind on {port}"
    try:
        yield host, port
    finally:
        try:
            server.shutdown()  # Python-level call, no bash keyword trigger
        except Exception:
            pass
        try:
            server.server_close()
        except Exception:
            pass
        # Give daemon thread time to exit cleanly
        thread.join(timeout=2.0)


def _http(method: str, host: str, port: int, path: str,
          body: dict = None) -> tuple[int, dict, bytes]:
    """R4-BE-03 minimal http client (no requests dep)."""
    conn = http.client.HTTPConnection(host, port, timeout=5)
    payload = json.dumps(body).encode("utf-8") if body is not None else b""
    headers = {"Content-Type": "application/json"} if body is not None else {}
    conn.request(method, path, body=payload, headers=headers)
    resp = conn.getresponse()
    raw = resp.read()
    hdrs = {"Content-Type": resp.getheader("Content-Type", "")}
    conn.close()
    try:
        parsed = json.loads(raw.decode("utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError):
        parsed = {}
    return resp.status, hdrs, parsed if isinstance(parsed, dict) else {}


# -----------------------------------------------------------------------------
# Tests
# -----------------------------------------------------------------------------


class TestServeHealth:
    """烟测 1: GET /health."""

    def test_health_200_alive(self, live_server):
        host, port = live_server
        status, hdrs, body = _http("GET", host, port, "/health")
        assert status == 200
        assert body.get("status") == "alive"
        assert body.get("version") == SERVE_VERSION
        assert body.get("engine") == "apeireth"

    def test_health_no_asi_leak(self, live_server):
        """R4-BE-03: 不暴露 ASI 分数 / V1074 / philosophy_guard."""
        host, port = live_server
        _status, _hdrs, body = _http("GET", host, port, "/health")
        forbidden = ("asi", "v1074", "philosophy", "guard", "score")
        for k in body.keys():
            assert not any(f in k.lower() for f in forbidden), \
                f"health leaked internal key: {k}"


class TestServeModels:
    """烟测 2: GET /v1/models."""

    def test_models_200_data_array(self, live_server):
        host, port = live_server
        status, hdrs, body = _http("GET", host, port, "/v1/models")
        assert status == 200
        assert body.get("object") == "list"
        data = body.get("data")
        assert isinstance(data, list)
        assert len(data) >= 1
        m = data[0]
        assert m.get("object") == "model"
        assert isinstance(m.get("id"), str) and m["id"]
        assert m.get("owned_by") == "apeireth"


class TestServeChatCompletion:
    """烟测 3: POST /v1/chat/completions (OpenAI 标准)."""

    def test_chat_completion_basic(self, live_server):
        host, port = live_server
        status, hdrs, body = _http(
            "POST", host, port, "/v1/chat/completions",
            body={
                "model": "apeireth-default",
                "messages": [{"role": "user", "content": "hello apeireth"}],
            },
        )
        assert status == 200
        # OpenAI 标准字段
        assert body.get("object") == "chat.completion"
        assert isinstance(body.get("id"), str) and body["id"].startswith("chatcmpl-")
        assert isinstance(body.get("created"), int)
        assert body.get("model")
        choices = body.get("choices")
        assert isinstance(choices, list) and len(choices) >= 1
        ch0 = choices[0]
        assert ch0.get("index") == 0
        msg = ch0.get("message", {})
        assert msg.get("role") == "assistant"
        assert isinstance(msg.get("content"), str) and msg["content"]
        assert ch0.get("finish_reason") == "stop"
        # usage 三件套
        usage = body.get("usage", {})
        assert usage.get("prompt_tokens", 0) > 0
        assert usage.get("completion_tokens", 0) > 0
        assert usage["total_tokens"] == usage["prompt_tokens"] + usage["completion_tokens"]

    def test_chat_completion_bad_request_empty_messages(self, live_server):
        host, port = live_server
        status, _hdrs, body = _http(
            "POST", host, port, "/v1/chat/completions",
            body={"model": "x", "messages": []},
        )
        assert status == 400
        assert body.get("error", {}).get("code") == "bad_request"


class TestServeStreaming:
    """烟测 4: POST /v1/chat/completions stream=true (SSE)."""

    def test_stream_sse_chunks(self, live_server):
        host, port = live_server
        conn = http.client.HTTPConnection(host, port, timeout=5)
        payload = json.dumps({
            "model": "apeireth-default",
            "stream": True,
            "messages": [{"role": "user", "content": "stream smoke test"}],
        }).encode("utf-8")
        conn.request(
            "POST", "/v1/chat/completions",
            body=payload,
            headers={"Content-Type": "application/json"},
        )
        resp = conn.getresponse()
        assert resp.status == 200
        ctype = resp.getheader("Content-Type", "")
        assert "text/event-stream" in ctype
        raw = resp.read().decode("utf-8")
        conn.close()
        # SSE: 至少 1 个 data: 行 + 末尾 data: [DONE]\n\n
        assert "data: " in raw
        assert "data: [DONE]" in raw
        # 解析第一个 chunk
        first_data_line = next(
            (line[len("data: "):] for line in raw.split("\n") if line.startswith("data: ") and line != "data: [DONE]"),
            None,
        )
        assert first_data_line is not None, "no SSE data chunk found"
        chunk = json.loads(first_data_line)
        assert chunk.get("object") == "chat.completion.chunk"
        assert isinstance(chunk.get("choices"), list) and len(chunk["choices"]) >= 1


class TestServeRouting:
    """bonus: 404 + OpenAI 标准错误格式."""

    def test_not_found_404(self, live_server):
        host, port = live_server
        status, _hdrs, body = _http("GET", host, port, "/v1/unknown")
        assert status == 404
        assert body.get("error", {}).get("type") == "invalid_request_error"

    def test_post_unknown_404(self, live_server):
        host, port = live_server
        status, _hdrs, body = _http("POST", host, port, "/v1/foo", body={})
        assert status == 404


if __name__ == "__main__":
    pytest.main([__file__, "-v"])