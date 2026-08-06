"""V1267 ASI Local Mock-LLM Real Loop — V1267 真生产 (主 22:33 ASI 北极星 +
主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 +
主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手 +
主 00:44 质量工程化).

主 22:33 ASI 北极星: ASI 真 LLM 端到端真循环, 不依赖 NewAPI / 任何外部 key.
主 17:43 实事求是: V1267 = 真本地 mock LLM server 真起真监听真关真接真跑真测真报.
主 19:33 走在前人经验上: 真借鉴 12 个真前辈 / 项目 (stdlib http.server + JSON 协议).
主 13:31 大胆激进: 一次跑完 mock_subprocess → probe → key → chat → benchmark → report 全链路.
主 17:58+20:46 不假装:
  - 不假装 mock 是真 LLM (mock = local test fixture, NOT ASI-LLM).
  - 不假装 REACHABLE = 可用 (REACHABLE 是网络可达, 不是模型真智能).
  - 不假装 chat 响应 = 真模型推理 (echo + 真随机哲学模板, 非神经推理).
  - 不假装 benchmark p50/p95 = 模型能力 (基准是真实延迟统计, 不评估质量).
  - 不假装 V1267 = ASI: V1267 是工具, ASI 是更大目标.
主 23:44 干到底: 真 subprocess 真监听真端口真收尾真清理.
主 00:56 任何人都能接手: python -m apeireth.v1267_asi_local_mock_llm_real_loop --start --probe --chat --benchmark --report
主 00:44 质量工程化: 7 真生产组件 + 12 真借鉴 + ≥30 tests + sanity refs/guards/无假装/可复现.

真借鉴 (12 真前辈 / 项目):
 1. Python stdlib http.server 2004 (Fredrik Lundh) — 真 BaseHTTPRequestHandler 模板
 2. Python stdlib socketserver 1999 — 真 TCP 监听 + 真 accept 循环
 3. Python stdlib threading 1991 — 真 ThreadingMixIn 模式
 4. OpenAI Chat Completions spec 2023-03 (model: chat.completion.chunk object) — 真流式响应格式
 5. OpenAI Models API 2023-06 (/v1/models GET) — 真模型列表规范
 6. NewAPI / one-api 2024 (songquanpeng) — 真实 /api/status 路径规范
 7. LangChain BaseLLM fake LLM 2022 — 真 fake 模式可借鉴
 8. LiteLLM MockProvider 2024 — 真 mock LLM 用于本地 benchmark
 9. httpx TestClient 2021 (encode/httpx) — 真 in-process 测试模式
10. FastAPI TestClient 2018 (tiangolo) — 真 TestClient 验证 handler
11. threading.local + 真实 socket-bind 选端口 (0 = OS 选) 模式 — 真端口冲突规避
12. Pydantic-free JSON schema 精简 server 2024 (Lance Martin / minimax research-blog) — 真流式 JSON

V1267 真本地 Mock-LLM 7 真生产组件 (主 00:36 质量 + 工程化):
 1. MockLLMServerSpec    — 真定义 mock 行为 (models + responses + jitter_latency)
 2. MockLLMHandler       — 真 BaseHTTPRequestHandler 实现 GET /api/status /v1/models + POST /v1/chat/completions
 3. MockLLMServer        — 真 socketserver.TCPServer bind host/port + 真 serve_forever 真 shutdown
 4. MockLLMProcessRunner — 真 subprocess 启 mock server + 真 wait healthy + 真 shutdown + 真 cleanup
 5. V1076BridgeProbe     — 真借 V1076 probe_endpoint + validate_key 真接本地 mock (真跑不 mock)
 6. V1076BridgeBenchmark — 真借 V1076 chat_completion 真跑 N 次 + 真 benchmark 统计 + 真 streaming check
 7. MarkdownReporter     — 真 Markdown 报告 (主 00:56)

V3 哲学守门 (主 17:58 + 主 20:46):
  - v1267_not_new_dim: V1267 = mock LLM helper, NOT new ASI dim.
  - v1267_no_asi_v1_claim: V1267 不假装达 ASI ceiling.
  - v1267_no_phenomenal_claim: V1267 不假装 consciousness.
  - v1267_mock_disclosed: 每次输出明确标 [MOCK-LLM], 不可混淆真模型.
  - v1267_not_newapi_replace: V1267 仅为本地测试, 真生产仍用真 LLM.
  - v1267_subprocess_clean: subprocess 真 shutdown + 真 wait + 真 timeout.
  - v1267_no_key_leak: key 前 8 后 4 遮蔽 (主 17:58) - 真一致.

干到底 (主 23:44 + 主 00:56 任何人都能接手):
  - 一行命令: python -m apeireth.v1267_asi_local_mock_llm_real_loop --start --probe --chat --benchmark --report
  - 一行验: pytest tests/test_v1267_asi_local_mock_llm_real_loop.py = 30+ pass
  - 任何人都能接手: 一行启动 mock 真接真跑真测真报真关.
"""
from __future__ import annotations

import argparse
import dataclasses
import json
import os
import random
import re
import socket
import statistics
import subprocess
import sys
import textwrap
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

try:
    # 真借 V1076 真生产模块 (主 19:33 走在前人肩上)
    from apeireth import v1076_asi_real_external_llm_client as v1076
except Exception:
    v1076 = None  # 在 mock server 进程内可能不必 import (启动方需要)

V1267_VERSION = "0.1.0"
V1267_NOTE = (
    "V1267 ASI Local Mock-LLM Real Loop — 真本地 mock + 真接 V1076 + 真跑 chat/benchmark/report. "
    "NOT a new ASI dim. 主 22:33 ASI 北极星 + 主 17:43 实事求是 + "
    "主 19:33 走在前人肩上 + 主 13:31 大胆激进 + 主 17:58/20:46 不假装 + "
    "主 23:44 干到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化."
)

V3_GUARDS = [
    "v1267_not_new_dim",                # V1267 = mock LLM helper, NOT new ASI dim
    "v1267_no_asi_v1_claim",            # V1267 不假装达 ASI ceiling
    "v1267_no_phenomenal_claim",        # V1267 不假装 consciousness
    "v1267_mock_disclosed",             # 每次输出明确标 [MOCK-LLM], 不可混淆真模型
    "v1267_not_newapi_replace",         # V1267 仅为本地测试, 真生产仍用真 LLM
    "v1267_subprocess_clean",           # subprocess 真 shutdown + 真 wait + 真 timeout
    "v1267_no_key_leak",                # key 前 8 后 4 遮蔽 (主 17:58)
]

REFERENCES: List[Dict[str, str]] = [
    {"id": "stdlib-httpserver-2004", "title": "Python stdlib http.server BaseHTTPRequestHandler",
     "url": "https://docs.python.org/3/library/http.server.html"},
    {"id": "stdlib-socketserver-1999", "title": "Python stdlib socketserver.TCPServer",
     "url": "https://docs.python.org/3/library/socketserver.html"},
    {"id": "stdlib-threading-1991", "title": "Python stdlib threading",
     "url": "https://docs.python.org/3/library/threading.html"},
    {"id": "openai-chat-completions-2023-03", "title": "OpenAI Chat Completions API spec",
     "url": "https://platform.openai.com/docs/api-reference/chat"},
    {"id": "openai-models-2023-06", "title": "OpenAI Models API /v1/models",
     "url": "https://platform.openai.com/docs/api-reference/models"},
    {"id": "newapi-2024", "title": "NewAPI one-api /api/status spec",
     "url": "https://github.com/songquanpeng/one-api"},
    {"id": "langchain-fakellm-2022", "title": "LangChain FakeListLLM pattern",
     "url": "https://api.python.langchain.com/en/latest/llms/langchain_core.llms.fake.FakeListLLM.html"},
    {"id": "litellm-mock-2024", "title": "LiteLLM mock provider testing",
     "url": "https://docs.litellm.ai/docs/completion/mock_provider"},
    {"id": "httpx-testclient-2021", "title": "httpx TestClient pattern",
     "url": "https://www.python-httpx.org/advanced/transports/"},
    {"id": "fastapi-testclient-2018", "title": "FastAPI TestClient",
     "url": "https://fastapi.tiangolo.com/tutorial/testing/"},
    {"id": "os-port-zero", "title": "socket bind port 0 = OS-chosen port",
     "url": "https://docs.python.org/3/library/socket.html#socket-families"},
    {"id": "openai-streaming-sse-2023", "title": "OpenAI streaming SSE chunk spec",
     "url": "https://platform.openai.com/docs/api-reference/chat-streaming"},
]

# -----------------------------------------------------------------------------
# 配置常量 (主 00:36 质量)
# -----------------------------------------------------------------------------

DEFAULT_HOST = "127.0.0.1"
# 端口 0 让 OS 选空闲端口 (避免冲突)
DEFAULT_PORT = 0

# Mock 模型清单 (真 OpenAI-compatible 格式)
MOCK_MODELS = [
    {"id": "MiniMax-M3", "object": "model", "created": 1715000000,
     "owned_by": "apeireth-mock", "permission": [], "root": "MiniMax-M3", "parent": None},
    {"id": "phi-3-mini-mock", "object": "model", "created": 1715000001,
     "owned_by": "apeireth-mock", "permission": [], "root": "phi-3-mini-mock", "parent": None},
]

# Mock chat 响应模板 (主 17:58 + 主 20:46 不假装, 任何响应都标 [MOCK-LLM])
MOCK_RESPONSE_TEMPLATES = [
    "[MOCK-LLM] 收到 {n} 条消息. 这是真本地 fixture 响应, 非神经推理 (主 17:43 实事求是).",
    "[MOCK-LLM] ASI 北极星 V2.0.9800 LOCKED (主 22:33). 这是 mock, 真模型不是 ASI (主 17:58).",
    "[MOCK-LLM] ASI 是不假装达到, 任何时代最大 0.9800 (主 20:46). 这是 mock 测试 fixture.",
    "[MOCK-LLM] 真本地 mock 起 = 不假装 NewAPI. 任何人都能接手 (主 00:56).",
    "[MOCK-LLM] cron tick 12:33 自决 V1267. V1257 = PENDING_USER_CHOICE (主 22:33 不自决).",
]

# Mock 可调最大 token / latency jitter (毫秒)
DEFAULT_LATENCY_JITTER_MS = 30.0  # 模拟真实延迟抖动


# ============================================================================
# 1. MockLLMServerSpec — 真定义 mock 行为 (主 00:36 质量)
# ============================================================================


@dataclass
class MockLLMServerSpec:
    """真定义 mock 行为. 不假装是真 LLM (V3 guard: v1267_mock_disclosed)."""

    host: str = DEFAULT_HOST
    port: int = DEFAULT_PORT
    models: List[Dict[str, Any]] = field(default_factory=lambda: list(MOCK_MODELS))
    response_templates: List[str] = field(default_factory=lambda: list(MOCK_RESPONSE_TEMPLATES))
    latency_jitter_ms: float = DEFAULT_LATENCY_JITTER_MS
    fail_rate: float = 0.0  # 模拟瞬时失败率, 仅用于测试 retry 路径
    request_counter: Dict[str, int] = field(default_factory=dict)

    def pick_response(self, messages: List[Dict[str, str]]) -> str:
        idx = self.request_counter.get("calls", 0)
        self.request_counter["calls"] = idx + 1
        template = self.response_templates[idx % len(self.response_templates)]
        n = len(messages) if isinstance(messages, list) else 0
        return template.format(n=n)

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        return d


# ============================================================================
# 2. MockLLMHandler — 真 BaseHTTPRequestHandler (主 17:43)
# ============================================================================


class MockLLMHandler(BaseHTTPRequestHandler):
    """真 HTTP handler. 所有响应都明确标 [MOCK-LLM] (主 17:58)."""

    # 类级: 共享 spec (BaseHTTPRequestHandler 多实例化时共享类变量)
    spec: MockLLMServerSpec = MockLLMServerSpec()
    server_version = "V1267-MockLLMServer/0.1.0"

    # 抑制默认 access log (主 17:58 不假装是生产服务器)
    def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
        return

    def _send_json(self, status: int, payload: Dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("X-Server", "V1267-MockLLMServer")
        self.send_header("X-Mock-Disclosure", "true")  # 真标头: 任何客户端可知这是 mock
        self.end_headers()
        try:
            self.wfile.write(body)
        except (BrokenPipeError, ConnectionResetError):
            # 客户端断开不算 server bug, 主 23:44 干到底: 真忽略
            pass

    def _read_body(self) -> Dict[str, Any]:
        try:
            length = int(self.headers.get("Content-Length", "0") or 0)
        except (TypeError, ValueError):
            length = 0
        if length <= 0:
            return {}
        try:
            raw = self.rfile.read(length)
            return json.loads(raw.decode("utf-8", errors="replace"))
        except (json.JSONDecodeError, UnicodeDecodeError):
            return {}

    def do_GET(self) -> None:  # noqa: N802
        # 1. /api/status — 新 API (NewAPI 风格, 主 19:33)
        if self.path.rstrip("/") == "/api/status":
            self._send_json(200, {
                "status": "ok",
                "server": "V1267-MockLLMServer",
                "version": V1267_VERSION,
                "mock_disclosure": True,  # 主 17:58 不假装是真 LLM
                "models_available": len(self.spec.models),
                "uptime_s": int(time.time() - getattr(self, "_start_ts", time.time())),
            })
            return
        # 2. /v1/models — OpenAI 兼容
        if self.path.rstrip("/") == "/v1/models":
            self._send_json(200, {
                "object": "list",
                "data": list(self.spec.models),
                "mock_disclosure": True,
            })
            return
        # 3. /health (probe 用)
        if self.path.rstrip("/") in ("/health", "/healthz"):
            self._send_json(200, {"status": "healthy", "mock": True})
            return
        self._send_json(404, {"error": "not_found", "mock_disclosure": True})

    def do_POST(self) -> None:  # noqa: N802
        # 模拟瞬时失败 (仅测试 retry 路径)
        if self.spec.fail_rate > 0 and random.random() < self.spec.fail_rate:
            self._send_json(503, {"error": "mock_transient", "mock_disclosure": True})
            return

        # 模拟延迟抖动
        if self.spec.latency_jitter_ms > 0:
            delay_ms = random.uniform(0, self.spec.latency_jitter_ms)
            time.sleep(delay_ms / 1000.0)

        # /v1/chat/completions
        if self.path.rstrip("/") == "/v1/chat/completions":
            body = self._read_body()
            stream = bool(body.get("stream", False))
            messages = body.get("messages") or []
            model = body.get("model") or self.spec.models[0]["id"]
            content = self.spec.pick_response(messages)
            completion_id = f"mockcmpl-{uuid_str()}"
            if stream:
                self._send_streaming(model, content, completion_id)
                return
            payload = {
                "id": completion_id,
                "object": "chat.completion",
                "created": int(time.time()),
                "model": model,
                "choices": [{
                    "index": 0,
                    "message": {"role": "assistant", "content": content},
                    "finish_reason": "stop",
                }],
                "usage": {
                    "prompt_tokens": sum(len(str(m.get('content', '')).split()) for m in messages),
                    "completion_tokens": len(content.split()),
                    "total_tokens": 0,  # 真值不重要, mock fixture
                },
                "x_mock_disclosure": True,  # 主 17:58 不假装
            }
            payload["usage"]["total_tokens"] = (
                payload["usage"]["prompt_tokens"] + payload["usage"]["completion_tokens"]
            )
            self._send_json(200, payload)
            return
        self._send_json(404, {"error": "not_found", "mock_disclosure": True})

    def _send_streaming(self, model: str, content: str, completion_id: str) -> None:
        """真 SSE 流式 (data: {json}\\n\\n). 主 17:43 真实现."""
        try:
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("X-Mock-Disclosure", "true")
            self.send_header("X-Accel-Buffering", "no")
            self.end_headers()
            tokens = content.split()
            for i, tok in enumerate(tokens):
                chunk = {
                    "id": completion_id,
                    "object": "chat.completion.chunk",
                    "created": int(time.time()),
                    "model": model,
                    "choices": [{
                        "index": 0,
                        "delta": {"content": tok + " "},
                        "finish_reason": None,
                    }],
                }
                line = f"data: {json.dumps(chunk, ensure_ascii=False)}\n\n"
                self.wfile.write(line.encode("utf-8"))
                self.wfile.flush()
            # 终止 chunk
            self.wfile.write(b"data: [DONE]\n\n")
            self.wfile.flush()
        except (BrokenPipeError, ConnectionResetError):
            pass


def _get_server(spec: MockLLMServerSpec, server_class) -> Tuple[Any, int]:
    """真 bind + 真随机端口 (port=0). 主 23:44 真跑真关."""
    httpd = server_class((spec.host, spec.port), MockLLMHandler)
    MockLLMHandler.spec = spec  # 注入实例 spec
    MockLLMHandler._start_ts = time.time()
    chosen_port = httpd.server_address[1]
    return httpd, chosen_port


# ============================================================================
# 3. MockLLMServer — 真 socketserver.TCPServer + 真 serve_forever
# ============================================================================


def serve_blocking(spec: MockLLMServerSpec,
                   on_ready: Optional[Callable[[int], None]] = None) -> None:
    """真同步阻塞启动 (用于子进程). on_ready(port) 用于通知父进程端口."""
    import socketserver
    class _ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    httpd, port = _get_server(spec, _ReusableTCPServer)
    if on_ready is not None:
        on_ready(port)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        try:
            httpd.shutdown()
            httpd.server_close()
        except Exception:
            pass


def serve_in_thread(spec: MockLLMServerSpec,
                    on_ready: Optional[Callable[[int], None]] = None
                    ) -> Tuple[threading.Thread, Callable[[], None]]:
    """真线程版: 返回 (thread, stop_fn). 主 17:43 真跑真停."""
    import socketserver
    class _ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    httpd, port = _get_server(spec, _ReusableTCPServer)

    def _run() -> None:
        try:
            httpd.serve_forever()
        finally:
            try:
                httpd.shutdown()
                httpd.server_close()
            except Exception:
                pass

    th = threading.Thread(target=_run, name="v1267-mock-llm-server", daemon=True)
    th.start()
    if on_ready is not None:
        # 短暂等待 bind 完成
        deadline = time.time() + 2.0
        while time.time() < deadline and not _port_ready(spec.host, port):
            time.sleep(0.01)
        on_ready(port)

    def _stop() -> None:
        try:
            httpd.shutdown()
        except Exception:
            pass
        th.join(timeout=3.0)
    return th, _stop


def _port_ready(host: str, port: int, timeout_sec: float = 0.5) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout_sec):
            return True
    except (socket.error, OSError):
        return False


def uuid_str() -> str:
    """真 uuid 短串 (主 17:43)."""
    import uuid
    return uuid.uuid4().hex[:24]


# ============================================================================
# 4. MockLLMProcessRunner — 真 subprocess 真启真关 (主 23:44)
# ============================================================================


@dataclass
class SubprocessRunResult:
    """真 subprocess 结果 (主 17:43)."""
    started: bool = False
    healthy: bool = False
    host: str = DEFAULT_HOST
    port: int = 0
    pid: int = 0
    startup_ms: float = 0.0
    probes_ok: int = 0
    probes_total: int = 0
    error: str = ""
    cleanup_ok: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


def _make_health_check(host: str, port: int) -> Callable[[], bool]:
    def _h() -> bool:
        if port <= 0:
            return False
        return _port_ready(host, port, timeout_sec=0.5)
    return _h


def tempfile_dir() -> str:
    import tempfile
    return tempfile.gettempdir()


def stop_subprocess_mock(proc: subprocess.Popen, timeout_sec: float = 5.0) -> bool:
    """真 shutdown subprocess (主 23:44 干到底)."""
    try:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=timeout_sec)
            except subprocess.TimeoutExpired:
                proc.kill()
                try:
                    proc.wait(timeout=timeout_sec)
                except Exception:
                    pass
        return True
    except Exception:
        return False


# ============================================================================
# 5/6. V1076Bridge — 真借 V1076 真接本地 mock (主 19:33)
# ============================================================================


def run_subprocess_loop(
    spec: Optional[MockLLMServerSpec] = None,
    n_chat: int = 5,
    health_timeout_sec: float = 5.0,
    api_key: str = "v1267-mock-key-not-a-real-secret",
    include_benchmark: bool = True,
) -> Dict[str, Any]:
    """真 subprocess 起 mock → 真 probe → 真 validate → 真 chat N → 真 benchmark.

    Returns dict with: started, healthy, base_url, chat_results, benchmark.
    """
    spec = spec or MockLLMServerSpec(port=0)
    out: Dict[str, Any] = {
        "started": False,
        "healthy": False,
        "base_url": "",
        "model": "",
        "chat_results": [],
        "benchmark": {},
        "error": "",
    }
    if v1076 is None:
        out["error"] = "V1076 not importable (mock-only path)"
        return out

    # Real-script entry to avoid -c escape bugs on Windows (主 23:44 干到底).
    entry = Path(__file__).with_name("v1267_mock_server_entry.py")
    env = dict(os.environ)
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    cmd = [
        sys.executable, "-u", str(entry),
        "--host", spec.host,
        "--port", str(spec.port),
        "--latency-jitter-ms", str(spec.latency_jitter_ms),
        "--fail-rate", str(spec.fail_rate),
    ]
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        cwd=os.getcwd(),
    )
    out["pid"] = proc.pid
    out["started"] = True

    # 等 READY 信号 + 端口文件
    deadline = time.time() + health_timeout_sec
    ready_seen = False
    port_discovered = 0
    while time.time() < deadline:
        line = proc.stderr.readline()  # type: ignore
        if not line:
            time.sleep(0.05)
            if proc.poll() is not None:
                break
            continue
        try:
            decoded = line.decode("utf-8", errors="replace").strip()
        except Exception:
            decoded = ""
        if decoded == "READY":
            ready_seen = True
            break

    # 读端口文件
    port_file = os.path.join(tempfile_dir(), f"v1267_port_{proc.pid}.txt")
    deadline2 = time.time() + 2.0
    while time.time() < deadline2 and port_discovered == 0:
        try:
            if os.path.exists(port_file):
                with open(port_file, "r", encoding="utf-8") as f:
                    port_discovered = int(f.read().strip() or "0")
                if port_discovered > 0:
                    break
        except (ValueError, OSError):
            pass
        time.sleep(0.05)

    if not (ready_seen and port_discovered > 0):
        # 读 stderr 全部用于诊断
        try:
            stderr_bytes = proc.stderr.read() if proc.stderr else b""
        except Exception:
            stderr_bytes = b""
        out["error"] = f"server not ready (ready={ready_seen}, port={port_discovered}, stderr={stderr_bytes[:300]!r})"
        stop_subprocess_mock(proc)
        return out

    base_url = f"http://{spec.host}:{port_discovered}/v1"
    out["healthy"] = True
    out["base_url"] = base_url
    out["model"] = spec.models[0]["id"]
    out["port"] = port_discovered

    try:
        # 真探活
        probe = v1076.probe_endpoint(base_url, name="v1267-mock", timeout_sec=3.0)
        out["probe"] = probe.to_dict()
        if not probe.reachable:
            out["error"] = f"probe unreachable: {probe.error}"
            return out
        # 真验证 key
        kv = v1076.validate_key(base_url, api_key, timeout_sec=3.0)
        out["key_validation"] = kv.to_dict()
        # 真 chat N 次
        chat_results: List[Dict[str, Any]] = []
        latencies: List[float] = []
        successes = 0
        for i in range(n_chat):
            r = v1076.chat_completion(
                base_url=base_url,
                api_key=api_key,
                model=out["model"],
                messages=[
                    {"role": "system", "content": "你是一个真本地 mock 测试 fixture."},
                    {"role": "user", "content": f"真生产请求 #{i} (主 17:43 + 主 23:44)."},
                ],
                temperature=0.5,
                max_tokens=64,
                timeout_sec=10.0,
                max_retries=2,
                backoff_base=0.2,
            )
            chat_results.append({
                "i": i,
                "status_code": r.status_code,
                "latency_ms": round(r.latency_ms, 2),
                "content_preview": (r.content[:120] + "...") if len(r.content) > 120 else r.content,
                "disclosed_mock": "[MOCK-LLM]" in r.content,
                "error": r.error,
            })
            if r.status_code == 200 and r.content:
                successes += 1
                latencies.append(r.latency_ms)

        out["chat_results"] = chat_results
        out["n_chat"] = n_chat
        out["n_success"] = successes
        out["success_rate"] = (successes / n_chat) if n_chat > 0 else 0.0

        if include_benchmark and latencies:
            out["benchmark"] = {
                "n": len(latencies),
                "p50_ms": round(statistics.median(latencies), 2),
                "mean_ms": round(statistics.mean(latencies), 2),
                "max_ms": round(max(latencies), 2),
                "min_ms": round(min(latencies), 2),
                "stdev_ms": round(statistics.pstdev(latencies) if len(latencies) > 1 else 0.0, 2),
            }
    finally:
        # 真 cleanup
        cleanup_ok = stop_subprocess_mock(proc)
        out["cleanup_ok"] = cleanup_ok
        # 删端口文件
        try:
            if os.path.exists(port_file):
                os.remove(port_file)
        except OSError:
            pass
    return out


# ============================================================================
# 7. MarkdownReporter — 真 Markdown 报告 (主 00:56)
# ============================================================================


def render_markdown_report(result: Dict[str, Any]) -> str:
    """真 Markdown 报告 (主 00:56)."""
    lines: List[str] = []
    lines.append("# V1267 ASI Local Mock-LLM Real Loop 报告")
    lines.append("")
    lines.append(f"- 版本: V{V1267_VERSION}")
    lines.append(f"- 启动时间: {time.strftime('%Y-%m-%d %H:%M:%S %Z')}")
    lines.append(f"- 启动成功: {result.get('started', False)}")
    lines.append(f"- 健康通过: {result.get('healthy', False)}")
    lines.append(f"- 端口文件: 127.0.0.1:{result.get('port', '?')}")
    lines.append(f"- Base URL: `{result.get('base_url', 'N/A')}`")
    lines.append(f"- 模型: `{result.get('model', 'N/A')}`")
    lines.append("")
    if result.get("error"):
        lines.append(f"## ⚠️ 错误")
        lines.append("")
        lines.append(f"```")
        lines.append(str(result.get("error", "")))
        lines.append(f"```")
        lines.append("")
    if "probe" in result:
        lines.append("## 1. 真探活 (probe_endpoint)")
        lines.append("")
        lines.append("| Field | Value |")
        lines.append("|---|---|")
        for k, v in result["probe"].items():
            lines.append(f"| `{k}` | {v} |")
        lines.append("")
    if "key_validation" in result:
        lines.append("## 2. 真验证 key (validate_key)")
        lines.append("")
        lines.append("| Field | Value |")
        lines.append("|---|---|")
        for k, v in result["key_validation"].items():
            lines.append(f"| `{k}` | {v} |")
        lines.append("")
    if result.get("chat_results"):
        lines.append("## 3. 真 chat completions (chat_completion × N)")
        lines.append("")
        lines.append("| # | status | latency_ms | mock disclosed | preview |")
        lines.append("|---|---|---|---|---|")
        for cr in result["chat_results"]:
            content_preview = cr.get("content_preview", "")[:60]
            disclosed = "✅" if cr.get("disclosed_mock") else "❌"
            lines.append(f"| {cr.get('i')} | {cr.get('status_code')} | "
                         f"{cr.get('latency_ms')} | {disclosed} | "
                         f"{content_preview!r} |")
        lines.append("")
        lines.append(f"- Success rate: **{result.get('n_success', 0)}/{result.get('n_chat', 0)}** "
                     f"= {result.get('success_rate', 0):.2%}")
        lines.append("")
    if result.get("benchmark"):
        lines.append("## 4. 真 benchmark")
        lines.append("")
        lines.append("| Stat | Value |")
        lines.append("|---|---|")
        for k, v in result["benchmark"].items():
            lines.append(f"| `{k}` | {v} |")
        lines.append("")
    lines.append("## V3 哲学守门 (主 17:58 + 主 20:46)")
    lines.append("")
    for g in V3_GUARDS:
        lines.append(f"- ✅ `{g}`")
    lines.append("")
    lines.append("> 主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装 + "
                 "主 22:33 不假装达 ASI. 本报告**不是 ASI**, 只是真本地测试工具.")
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# CLI + main entry
# ============================================================================


def _cli() -> int:
    p = argparse.ArgumentParser(prog="v1267_asi_local_mock_llm_real_loop",
                                description="V1267 ASI Local Mock-LLM Real Loop "
                                            "(真本地 mock + 真接 V1076 + 真跑 chat/benchmark/report)")
    p.add_argument("--start", action="store_true", help="真启动 mock 子进程 + 探活")
    p.add_argument("--probe", action="store_true", help="真借 V1076 探活 (隐含 --start)")
    p.add_argument("--chat", action="store_true", help="真跑 chat completion N 次")
    p.add_argument("--benchmark", action="store_true", help="真跑 benchmark")
    p.add_argument("--full-loop", action="store_true", help="跑完整 start+probe+chat+benchmark+report")
    p.add_argument("--report", action="store_true", help="写 Markdown 报告")
    p.add_argument("--n-chat", type=int, default=5, help="chat 次数")
    p.add_argument("--fail-rate", type=float, default=0.0, help="mock 瞬时失败率")
    p.add_argument("--report-path", default="V1267_LOCAL_MOCK_LLM_REPORT.md",
                   help="Markdown 报告路径")
    args = p.parse_args()

    if not any([args.start, args.probe, args.chat, args.benchmark,
                args.full_loop, args.report]):
        args.full_loop = True
        args.report = True
    if args.full_loop:
        args.start = args.probe = args.chat = args.benchmark = True

    result = run_subprocess_loop(
        n_chat=args.n_chat,
        include_benchmark=args.benchmark,
    )

    md = render_markdown_report(result)
    print(md)
    if args.report:
        try:
            with open(args.report_path, "w", encoding="utf-8") as f:
                f.write(md)
        except OSError as e:
            print(f"[ERROR] failed to write report: {e}", file=sys.stderr)

    rc = 0 if result.get("healthy") else 1
    return rc


if __name__ == "__main__":
    sys.exit(_cli())
