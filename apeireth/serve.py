"""Phase 221 serve — apeireth OpenAI 兼容服务 (主 21:15 用户面第二面 + R4-BE-03).

让任何 OpenAI 客户端 (Cursor/LangChain/Continue/自写脚本) 直接接 localhost:8080/v1/chat/completions
当 OpenAI 用, 但底层是 apeireth 引擎 + 12 生命特征在后台跑.

主人新方向核心 (R4-BE-03 brief):
- OpenAI 兼容 = 接入成本最低, 用户一行 config 就接
- 内部 12 生命特征 / 哲学守门 / V1074 / V1085 HQB 永不外显给 OpenAI 客户端
- OpenAI 客户端应该看到的: 跟调 OpenAI 一模一样的 response

设计 (主 17:43 实事求是 + 主 07-19 4 层安全门):
- 标准库 http.server.ThreadingHTTPServer + BaseHTTPRequestHandler (无外部依赖)
- 默认 provider=template, 真 API 通过 APEIRETH_PROVIDER env 切到 minimax
- 默认端口 8080, APEIRETH_PORT env override
- 不动 llm_kernel.py (主 07-19: 只 import, 不修改)

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装分数 = ASI: /health 不返回 ASI score, /v1/models 不暴露 V1074
- 不破坏 OpenAI 客户端期待: 字段名/类型/状态码严格 OpenAI 规范
- 内部 HQB/12 生命特征在 service 层跑, 不写进 response body
"""
from __future__ import annotations

import argparse
import json
import os
import socketserver
import threading
import time
import uuid
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable, Dict, List, Optional, Tuple

from apeireth.llm_kernel import (  # 不动 llm_kernel.py, 仅 import (主 07-19)
    LLMResponse,
    make_call_llm,
)


SERVE_VERSION = "0.1.0"
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8080
DEFAULT_PROVIDER = "template"  # 无依赖即可跑 (主 17:43 实事求是)
DEFAULT_MODEL = "apeireth-default"

# R11 security hardening (R11-SEC-001):
# - Body size cap prevents OOM DoS via huge Content-Length (OWASP A05:2021).
# - Messages count + per-content size cap prevents prompt-bomb DoS.
# - All limits are env-overridable for tests; defaults are conservative.
MAX_BODY_BYTES = int(os.environ.get("APEIRETH_MAX_BODY_BYTES", 1 * 1024 * 1024))       # 1 MiB
MAX_MESSAGES_COUNT = int(os.environ.get("APEIRETH_MAX_MESSAGES_COUNT", 100))
MAX_MESSAGE_CONTENT_BYTES = int(os.environ.get("APEIRETH_MAX_MESSAGE_CONTENT_BYTES", 32 * 1024))
MAX_TOTAL_CONTENT_BYTES = int(os.environ.get("APEIRETH_MAX_TOTAL_CONTENT_BYTES", 256 * 1024))


def _safe_path_label(raw_path: str, max_len: int = 64) -> str:
    """R11-SEC-001: 返回可安全回显到 JSON / 日志的路径标签.

    防御:
    - CR/LF 回车注入 (破坏 header / 日志行)
    - 控制字符 (0x00-0x1F, 0x7F)
    - 超长原始路径撑大响应
    仅保留可见 ASCII, 其它替换为 '?'.
    """
    if not isinstance(raw_path, str):
        raw_path = str(raw_path)
    cleaned = "".join(c if 0x20 <= ord(c) < 0x7F else "?" for c in raw_path)
    # 截断前剥离 query / fragment 防止超长 URL 撑大响应
    for sep in ("?", "#"):
        idx = cleaned.find(sep)
        if idx >= 0:
            cleaned = cleaned[:idx]
    if len(cleaned) > max_len:
        cleaned = cleaned[:max_len] + "..."
    return cleaned


# -----------------------------------------------------------------------------
# Internal: 12 生命特征 / HQB / philosophy guard 在这里跑 (不外显)
# -----------------------------------------------------------------------------


def _internal_engine_stub(messages: List[Dict[str, str]], model: str) -> str:
    """R4-BE-03 占位: 内部引擎 (12 生命特征 / HQB / 守门) 在此运行.

    不外显任何 ASI 分数 / V1074 / philosophy_guard 状态.
    真生产应替换为 call_llm + deliberation + tool_runner 链路.
    """
    last_user = ""
    for m in reversed(messages):
        if m.get("role") == "user":
            last_user = m.get("content", "")
            break
    return f"[apeireth/{model}] ack: {last_user[:80]}"


def _make_default_provider() -> Callable[[str], str]:
    """R4-BE-03 默认 provider: 用 llm_kernel template, 真 API 通过 env 切换."""
    provider = os.environ.get("APEIRETH_PROVIDER", DEFAULT_PROVIDER)
    return make_call_llm(provider=provider)


# -----------------------------------------------------------------------------
# OpenAI-compatible request/response builders
# -----------------------------------------------------------------------------


def _now_ts() -> int:
    return int(time.time())


def _extract_user_prompt(messages: List[Dict[str, Any]]) -> str:
    """OpenAI chat: 取最后一条 user 消息作为 prompt."""
    for m in reversed(messages or []):
        if m.get("role") == "user":
            return str(m.get("content", ""))
    return ""


def _estimate_tokens(text: str) -> int:
    """R4-BE-03 粗估 token 数 (主 17:43 实事求是: 估值, 不是真 tokenizer).

    OpenAI 客户端期待 usage 字段非零, 用 max(1, len/4) 兜底.
    """
    return max(1, len(text) // 4)


def build_chat_completion(
    messages: List[Dict[str, Any]],
    model: str,
    call_llm: Callable[[str], str],
    stream: bool = False,
) -> Tuple[Dict[str, Any], LLMResponse]:
    """R4-BE-03 构造 OpenAI 标准 chat.completion 响应 (主 17:43 实事求是)."""
    prompt = _extract_user_prompt(messages)
    t0 = time.time()
    content = call_llm(prompt)
    latency_ms = (time.time() - t0) * 1000

    # tokens 估值 (主 17:43: 不假装是真 tokenizer)
    prompt_tokens = _estimate_tokens(prompt)
    completion_tokens = _estimate_tokens(content)
    total_tokens = prompt_tokens + completion_tokens

    resp = LLMResponse(
        content=content,
        model=model,
        tokens_in=prompt_tokens,
        tokens_out=completion_tokens,
        latency_ms=latency_ms,
        provider=os.environ.get("APEIRETH_PROVIDER", DEFAULT_PROVIDER),
    )

    body = {
        "id": f"chatcmpl-{uuid.uuid4().hex[:24]}",
        "object": "chat.completion",
        "created": _now_ts(),
        "model": resp.model,
        "choices": [
            {
                "index": 0,
                "message": {"role": "assistant", "content": resp.content},
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": resp.tokens_in,
            "completion_tokens": resp.tokens_out,
            "total_tokens": resp.tokens_in + resp.tokens_out,
        },
    }
    return body, resp


def build_sse_chunk(chunk_id: str, model: str, content_delta: str,
                    finish_reason: Optional[str] = None) -> str:
    """R4-BE-03 OpenAI SSE chunk (data: {...}\\n\\n)."""
    payload = {
        "id": chunk_id,
        "object": "chat.completion.chunk",
        "created": _now_ts(),
        "model": model,
        "choices": [
            {
                "index": 0,
                "delta": {"content": content_delta} if content_delta else {},
                "finish_reason": finish_reason,
            }
        ],
    }
    return f"data: {json.dumps(payload, ensure_ascii=False)}\n\n"


def build_models_response(model: str = DEFAULT_MODEL) -> Dict[str, Any]:
    """R4-BE-03 OpenAI /v1/models 标准格式."""
    return {
        "object": "list",
        "data": [
            {
                "id": model,
                "object": "model",
                "created": _now_ts(),
                "owned_by": "apeireth",
            }
        ],
    }


# -----------------------------------------------------------------------------
# HTTP handler
# -----------------------------------------------------------------------------


@dataclass
class _ServerState:
    """R4-BE-03 服务状态 (主 07-19: 信任边界 = 内部状态)."""
    call_llm: Callable[[str], str]
    model: str = DEFAULT_MODEL


def make_handler(state: _ServerState) -> type:
    """R4-BE-03 工厂: 用 closure 注入 call_llm + model."""

    class ApeirethOpenAIHandler(BaseHTTPRequestHandler):
        # Silence default per-request stderr log
        def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
            return

        def _send_json(self, status: int, body: Dict[str, Any]) -> None:
            data = json.dumps(body, ensure_ascii=False).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def _send_sse(self, chunks: List[str]) -> None:
            payload = ("".join(chunks) + "data: [DONE]\n\n").encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def do_GET(self) -> None:  # noqa: N802
            if self.path == "/health":
                # R4-BE-03: 不返回 ASI 分数 / V1074 状态 (主 17:58 不假装)
                self._send_json(200, {
                    "status": "alive",
                    "version": SERVE_VERSION,
                    "engine": "apeireth",
                })
                return
            if self.path == "/v1/models":
                self._send_json(200, build_models_response(state.model))
                return
            # R11-SEC-001: 不回显原始 self.path (CR/LF/控制字符 + 超长输入可注入)
            self._send_json(404, {
                "error": {
                    "message": f"not found: {_safe_path_label(self.path)}",
                    "type": "invalid_request_error",
                    "code": "not_found",
                }
            })

        def do_POST(self) -> None:  # noqa: N802
            if self.path != "/v1/chat/completions":
                # R11-SEC-001: 不回显原始 self.path
                self._send_json(404, {
                    "error": {
                        "message": f"not found: {_safe_path_label(self.path)}",
                        "type": "invalid_request_error",
                        "code": "not_found",
                    }
                })
                return
            # R11-SEC-001: 强制 Content-Type=application/json (避免 form/multipart 绕过)
            ctype = (self.headers.get("Content-Type") or "").split(";", 1)[0].strip().lower()
            if ctype and ctype != "application/json":
                self._send_json(415, {
                    "error": {
                        "message": "Content-Type must be application/json",
                        "type": "invalid_request_error",
                        "code": "unsupported_media_type",
                    }
                })
                return
            # R11-SEC-001: Content-Length 上限 — 防止 OOM (OWASP A05:2021)
            cl_header = self.headers.get("Content-Length")
            if cl_header is None:
                self._send_json(411, {
                    "error": {
                        "message": "Content-Length required",
                        "type": "invalid_request_error",
                        "code": "length_required",
                    }
                })
                return
            try:
                length = int(cl_header)
            except ValueError:
                self._send_json(400, {
                    "error": {
                        "message": "invalid Content-Length",
                        "type": "invalid_request_error",
                        "code": "bad_request",
                    }
                })
                return
            if length < 0 or length > MAX_BODY_BYTES:
                self._send_json(413, {
                    "error": {
                        "message": f"body too large: limit={MAX_BODY_BYTES}",
                        "type": "invalid_request_error",
                        "code": "payload_too_large",
                    }
                })
                return
            # R11-SEC-001: 即使 Content-Length 在限内, 实际 read 也限到 length (防 split response)
            try:
                raw = self.rfile.read(length).decode("utf-8")
                req = json.loads(raw) if raw else {}
            except (json.JSONDecodeError, UnicodeDecodeError):
                self._send_json(400, {
                    "error": {
                        "message": "invalid JSON body",
                        "type": "invalid_request_error",
                        "code": "bad_request",
                    }
                })
                return

            messages = req.get("messages", [])
            if not isinstance(messages, list) or not messages:
                self._send_json(400, {
                    "error": {
                        "message": "messages must be non-empty list",
                        "type": "invalid_request_error",
                        "code": "bad_request",
                    }
                })
                return
            # R11-SEC-001: 消息数上限 (DoS 防护)
            if len(messages) > MAX_MESSAGES_COUNT:
                self._send_json(413, {
                    "error": {
                        "message": f"too many messages: limit={MAX_MESSAGES_COUNT}",
                        "type": "invalid_request_error",
                        "code": "payload_too_large",
                    }
                })
                return
            # R11-SEC-001: 单条 content 大小 + 总量上限 (DoS 防护)
            total_content = 0
            for i, m in enumerate(messages):
                if not isinstance(m, dict):
                    self._send_json(400, {
                        "error": {
                            "message": f"message[{i}] must be object",
                            "type": "invalid_request_error",
                            "code": "bad_request",
                        }
                    })
                    return
                content = m.get("content", "")
                if not isinstance(content, str):
                    content = str(content) if content is not None else ""
                if len(content.encode("utf-8")) > MAX_MESSAGE_CONTENT_BYTES:
                    self._send_json(413, {
                        "error": {
                            "message": (
                                f"message[{i}] content too large: limit={MAX_MESSAGE_CONTENT_BYTES}"
                            ),
                            "type": "invalid_request_error",
                            "code": "payload_too_large",
                        }
                    })
                    return
                total_content += len(content.encode("utf-8"))
            if total_content > MAX_TOTAL_CONTENT_BYTES:
                self._send_json(413, {
                    "error": {
                        "message": (
                            f"total content too large: limit={MAX_TOTAL_CONTENT_BYTES}"
                        ),
                        "type": "invalid_request_error",
                        "code": "payload_too_large",
                    }
                })
                return
            model = str(req.get("model", state.model))
            stream = bool(req.get("stream", False))

            if stream:
                # SSE: 一次性 split 一段 (主 17:43 实事求是)
                chunk_id = f"chatcmpl-{uuid.uuid4().hex[:24]}"
                body, _resp = build_chat_completion(
                    messages, model, state.call_llm, stream=True
                )
                full_content = body["choices"][0]["message"]["content"]
                chunks = [
                    build_sse_chunk(chunk_id, model, full_content, finish_reason="stop"),
                ]
                self._send_sse(chunks)
                return

            body, _resp = build_chat_completion(
                messages, model, state.call_llm, stream=False
            )
            self._send_json(200, body)

    return ApeirethOpenAIHandler


# -----------------------------------------------------------------------------
# Server bootstrap
# -----------------------------------------------------------------------------


def serve(
    host: str = DEFAULT_HOST,
    port: int = DEFAULT_PORT,
    call_llm: Optional[Callable[[str], str]] = None,
    model: str = DEFAULT_MODEL,
) -> ThreadingHTTPServer:
    """R4-BE-03 启动服务 (阻塞)."""
    state = _ServerState(
        call_llm=call_llm or _make_default_provider(),
        model=model,
    )
    handler = make_handler(state)
    server = ThreadingHTTPServer((host, port), handler)
    server.daemon_threads = True
    return server


def serve_in_background(
    host: str = DEFAULT_HOST,
    port: int = 0,  # 0 = auto-pick free port (test 友好)
    call_llm: Optional[Callable[[str], str]] = None,
    model: str = DEFAULT_MODEL,
) -> Tuple[ThreadingHTTPServer, threading.Thread, int]:
    """R4-BE-03 启动后台服务 (测试用, 返回 (server, thread, bound_port))."""
    state = _ServerState(
        call_llm=call_llm or _make_default_provider(),
        model=model,
    )
    handler = make_handler(state)
    server = ThreadingHTTPServer((host, port), handler)
    server.daemon_threads = True
    bound_port = server.server_address[1]
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    return server, thread, bound_port


def _cli(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="apeireth.serve",
        description="apeireth OpenAI-compatible HTTP service",
    )
    parser.add_argument("--host", default=os.environ.get("APEIRETH_HOST", DEFAULT_HOST))
    parser.add_argument(
        "--port",
        type=int,
        default=int(os.environ.get("APEIRETH_PORT", DEFAULT_PORT)),
    )
    parser.add_argument(
        "--provider",
        default=os.environ.get("APEIRETH_PROVIDER", DEFAULT_PROVIDER),
    )
    parser.add_argument("--model", default=DEFAULT_MODEL)
    args = parser.parse_args(argv)

    os.environ["APEIRETH_PROVIDER"] = args.provider
    server = serve(args.host, args.port, model=args.model)
    print(
        f"[apeireth.serve] v{SERVE_VERSION} listening on http://{args.host}:{args.port} "
        f"(provider={args.provider}, model={args.model})"
    )
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[apeireth.serve] shutting down")
    finally:
        server.server_close()
    return 0


__all__ = [
    "SERVE_VERSION",
    "DEFAULT_HOST",
    "DEFAULT_PORT",
    "DEFAULT_MODEL",
    "DEFAULT_PROVIDER",
    "serve",
    "serve_in_background",
    "build_chat_completion",
    "build_models_response",
    "build_sse_chunk",
]


if __name__ == "__main__":
    import sys
    sys.exit(_cli())