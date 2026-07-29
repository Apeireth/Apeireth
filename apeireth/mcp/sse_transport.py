"""apeireth.mcp.sse_transport — Server-Sent Events (SSE) transport (Anthropic MCP 2024-11-05).

V1129 引入: 补足 stdio + HTTP(/rpc) 之外的第 3 种 MCP transport.

协议 (对齐 Anthropic MCP SSE transport 规范):
  - GET  /sse                  → 建立 SSE 长连接, 第 1 个 event = "endpoint" data=/messages?session_id=xxx
  - POST /messages?session_id=xxx → 发送 JSON-RPC 请求, 服务器通过对应 SSE 流回响应
  - GET  /health               → {"ok": true, "sessions": n}
  - GET  /tools                → 工具列表 (与 HTTP transport 一致)

状态机:
  - session 建立后, 所有响应通过 SSE 推送 (event: message, data: <json-rpc 2.0 envelope>)
  - SSE 连接断开 → 清理 session 资源, 不影响 dispatcher 状态
  - chaos: 客户端断开时 server 仍然能继续接受 POST, 等下一个 SSE 重连

借鉴:
  - V1097 StdioMCPClient 模式
  - V1123 HttpTransport 模式
  - Anthropic MCP 2024-11-05 spec (https://modelcontextprotocol.io/specification/2024-11-05/transport)
"""
from __future__ import annotations

import json
import os
import queue
import socket
import threading
import time
import uuid
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable, Dict, List, Optional, Set, Tuple


# 借鉴 V1097 transport: 单条 JSON 对象 1 行 (这里用 SSE event: message 包装)
SSE_MAX_BODY_BYTES = 8 * 1024 * 1024
SSE_PING_INTERVAL = 15.0        # 15s ping 保活 (主 23:44 干到底: 不让 client 误以为断了)
SSE_QUEUE_MAX = 256             # 每 session 缓冲上限 (避免内存爆炸)
SSE_SESSION_TTL = 60.0 * 30     # session 30 分钟无活动清理


# ---------------------------------------------------------------------------
# Session 管理
# ---------------------------------------------------------------------------


@dataclass
class SseSession:
    """SSE session — 一个 client 一个 session.

    chaos 守门 (主 23:44 干到底): client 断线不丢 server 内部状态, 只丢 client buffer.
    """

    session_id: str
    outbox: "queue.Queue[Dict[str, Any]]" = field(default_factory=lambda: queue.Queue(maxsize=SSE_QUEUE_MAX))
    sse_alive: bool = False           # SSE 流是否还连着
    created_at: float = field(default_factory=time.time)
    last_active_at: float = field(default_factory=time.time)
    n_messages_sent: int = 0
    n_messages_dropped: int = 0

    def touch(self) -> None:
        self.last_active_at = time.time()

    def push(self, payload: Dict[str, Any]) -> bool:
        """推到 outbox, 成功返 True, 满则丢弃 + 计数 (主 17:43 实事求是: 透明)."""
        try:
            self.outbox.put_nowait(payload)
        except queue.Full:
            self.n_messages_dropped += 1
            return False
        return True


class SseSessionStore:
    """thread-safe SSE session 存储."""

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._sessions: Dict[str, SseSession] = {}

    def create(self) -> SseSession:
        sid = uuid.uuid4().hex
        with self._lock:
            sess = SseSession(session_id=sid)
            self._sessions[sid] = sess
            return sess

    def get(self, sid: str) -> Optional[SseSession]:
        with self._lock:
            return self._sessions.get(sid)

    def remove(self, sid: str) -> None:
        with self._lock:
            self._sessions.pop(sid, None)

    def stats(self) -> Dict[str, Any]:
        with self._lock:
            return {
                "n_sessions": len(self._sessions),
                "n_sse_alive": sum(1 for s in self._sessions.values() if s.sse_alive),
                "sessions": [
                    {"id": s.session_id, "sse_alive": s.sse_alive,
                     "n_messages_sent": s.n_messages_sent,
                     "n_messages_dropped": s.n_messages_dropped,
                     "age_seconds": round(time.time() - s.created_at, 2)}
                    for s in self._sessions.values()
                ],
            }

    def reap_expired(self) -> int:
        """清理超过 SSE_SESSION_TTL 的 session (主 17:43 实事求是: 不泄漏)."""
        now = time.time()
        with self._lock:
            stale = [sid for sid, s in self._sessions.items()
                     if (now - s.last_active_at) > SSE_SESSION_TTL and not s.sse_alive]
            for sid in stale:
                del self._sessions[sid]
            return len(stale)


# ---------------------------------------------------------------------------
# SSE Transport
# ---------------------------------------------------------------------------


@dataclass
class SseTransport:
    """SSE transport (主 13:31 大胆激进: 真接 SSE, 不 mock).

    端口默认 0 (动态分配, test 友好).
    """

    dispatch: Callable[[Dict[str, Any]], Optional[Dict[str, Any]]]
    server_info: Dict[str, Any]
    host: str = "127.0.0.1"
    port: int = 0
    session_store: SseSessionStore = field(default_factory=SseSessionStore)
    _server: Optional[ThreadingHTTPServer] = None
    _thread: Optional[threading.Thread] = None
    _reaper_thread: Optional[threading.Thread] = None
    _stopped: bool = False
    n_dispatched: int = 0

    def __post_init__(self) -> None:
        if self._server is not None:
            return
        outer = self

        def _sse_format(event: str, data: str) -> bytes:
            # 标准 SSE 格式: event: <event>\ndata: <data>\n\n
            return f"event: {event}\ndata: {data}\n\n".encode("utf-8")

        def _send_json(handler: BaseHTTPRequestHandler, status: int,
                        payload: Mapping[str, Any]) -> None:
            body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
            handler.send_response(status)
            handler.send_header("Content-Type", "application/json; charset=utf-8")
            handler.send_header("Content-Length", str(len(body)))
            handler.end_headers()
            handler.wfile.write(body)

        class _Handler(BaseHTTPRequestHandler):
            server_version = "Apeireth-V1129-SSE"

            def log_message(self, fmt: str, *args: Any) -> None:  # noqa: A003
                # 静默 stderr
                pass

            def _read_json_body(self) -> Optional[Dict[str, Any]]:
                length = int(self.headers.get("Content-Length", "0") or 0)
                if length <= 0 or length > SSE_MAX_BODY_BYTES:
                    return None
                raw = self.rfile.read(length)
                try:
                    obj = json.loads(raw.decode("utf-8"))
                except (UnicodeDecodeError, json.JSONDecodeError):
                    return None
                return obj if isinstance(obj, dict) else None

            # ---------- GET /sse ----------

            def do_GET(self) -> None:  # noqa: N802
                if self.path == "/health":
                    _send_json(self, 200, {"ok": True, "transport": "sse",
                                            **outer.session_store.stats()})
                    return
                if self.path == "/tools":
                    _send_json(self, 200, {
                        "tools": outer.server_info.get("tools", []),
                    })
                    return
                if self.path.startswith("/sse"):
                    self._handle_sse_stream()
                    return
                _send_json(self, 404, {"error": "not found"})

            def _handle_sse_stream(self) -> None:
                # 建立新 session
                sess = outer.session_store.create()
                sess.sse_alive = True
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.send_header("Cache-Control", "no-cache")
                self.send_header("Connection", "keep-alive")
                self.send_header("X-Accel-Buffering", "no")
                self.end_headers()
                try:
                    # 1) endpoint 事件
                    self.wfile.write(_sse_format(
                        "endpoint", f"/messages?session_id={sess.session_id}"))
                    self.wfile.flush()
                    # 2) 持续推送 outbox 消息
                    last_ping = time.time()
                    while not outer._stopped:
                        try:
                            payload = sess.outbox.get(timeout=1.0)
                        except queue.Empty:
                            # ping 保活
                            if time.time() - last_ping >= SSE_PING_INTERVAL:
                                try:
                                    self.wfile.write(_sse_format("ping", str(int(time.time()))))
                                    self.wfile.flush()
                                except (OSError, ConnectionError):
                                    break
                                last_ping = time.time()
                            continue
                        data = json.dumps(payload, ensure_ascii=False)
                        self.wfile.write(_sse_format("message", data))
                        self.wfile.flush()
                        sess.n_messages_sent += 1
                        last_ping = time.time()
                except (OSError, ConnectionError, BrokenPipeError):
                    # client 断线 — chaos 守门: server 状态保留
                    pass
                finally:
                    sess.sse_alive = False

            # ---------- POST /messages ----------

            def do_POST(self) -> None:  # noqa: N802
                # 解析 session_id
                from urllib.parse import urlparse, parse_qs
                parsed = urlparse(self.path)
                qs = parse_qs(parsed.query)
                sid_list = qs.get("session_id", [])
                sid = sid_list[0] if sid_list else ""
                if not sid:
                    _send_json(self, 400, {"error": "missing session_id"})
                    return
                sess = outer.session_store.get(sid)
                if sess is None:
                    # chaos 守门: session 已被 reaper 清掉, 但 dispatcher 仍能工作
                    # → 接收请求, 即时回错误 (不丢请求, 透明)
                    _send_json(self, 410, {
                        "error": "session expired, please reconnect /sse",
                        "session_id": sid,
                    })
                    return
                sess.touch()
                req = self._read_json_body()
                if req is None:
                    _send_json(self, 400, {"error": "invalid JSON body"})
                    return
                resp = outer.dispatch(req)
                if resp is None:
                    _send_json(self, 204, {})
                    return
                # chaos 守门: SSE 失联时仍可 POST + 收同步响应 (退化)
                if not sess.sse_alive:
                    # 直接返回同步响应 (degraded 模式)
                    _send_json(self, 200, {
                        "mode": "degraded_no_sse",
                        "note": "SSE stream disconnected, returned synchronously",
                        "response": resp,
                    })
                    return
                # 正常路径: 响应推到 SSE outbox
                if not sess.push(resp):
                    _send_json(self, 503, {
                        "error": "SSE outbox full, message dropped",
                        "session_id": sid,
                        "n_dropped": sess.n_messages_dropped,
                    })
                    return
                outer.n_dispatched += 1
                _send_json(self, 202, {"accepted": True, "session_id": sid})

        self._server = ThreadingHTTPServer((self.host, self.port), _Handler)
        self.port = self._server.server_address[1]

    @property
    def actual_port(self) -> int:
        if self._server is None:
            return 0
        return int(self._server.server_address[1])

    def sse_url(self) -> str:
        return f"http://{self.host}:{self.actual_port}/sse"

    def messages_url(self, session_id: str = "") -> str:
        return f"http://{self.host}:{self.actual_port}/messages?session_id={session_id}"

    def start(self) -> None:
        if self._server is None or self._thread is not None:
            return
        server = self._server

        def _run() -> None:
            try:
                server.serve_forever()
            except Exception:  # noqa: BLE001
                pass

        self._thread = threading.Thread(target=_run, daemon=True, name="v1129-sse")
        self._thread.start()

        # 启动 reaper 线程 (主 23:44 干到底: 不泄漏)
        def _reap() -> None:
            while not self._stopped:
                try:
                    self.session_store.reap_expired()
                except Exception:  # noqa: BLE001
                    pass
                time.sleep(30.0)

        self._reaper_thread = threading.Thread(target=_reap, daemon=True, name="v1129-sse-reaper")
        self._reaper_thread.start()

    def stop(self) -> None:
        self._stopped = True
        if self._server is None:
            return
        try:
            self._server.shutdown()
        except Exception:  # noqa: BLE001
            pass
        self._server = None
        self._thread = None
        self._reaper_thread = None


__all__ = [
    "SseTransport", "SseSession", "SseSessionStore",
    "SSE_MAX_BODY_BYTES", "SSE_PING_INTERVAL", "SSE_QUEUE_MAX", "SSE_SESSION_TTL",
]
