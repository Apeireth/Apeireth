"""apeireth.mcp.transport — stdio (NDJSON) + HTTP (/rpc) 两种 transport (主 19:33 走在前人经验上).

V1123 复用 V1097 已验证的 NDJSON 模式 (避免重复踩坑) + 新增 stdlib HTTP server
作为第二种 transport (SSE 兼容). 不引入 mcp 第三方 SDK (项目无 pyproject/requirements).

stdio:
  - 每行一个 JSON 对象 (newline-delimited JSON-RPC 2.0)
  - 输入: stdin (read line by line, blocking)
  - 输出: stdout (每响应 1 行)

HTTP:
  - POST /rpc 接 JSON-RPC envelope
  - 返回 JSON-RPC envelope (不是 SSE, 但 MCP client 仍可消费)
  - /health GET → {"ok": true, "server": "apeireth-asi-mcp"}
  - /tools GET → 当前 tool 列表
"""
from __future__ import annotations

import json
import os
import sys
import threading
from dataclasses import dataclass
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable, Dict, List, Optional, TextIO


# 借鉴 V1097 transport: 单条 JSON 对象 1 行
NDJSON_LINE_LIMIT = 1 * 1024 * 1024  # 1 MiB / 行 (主 17:43 守门)


# ---------------------------------------------------------------------------
# 1. stdio transport
# ---------------------------------------------------------------------------


class StdioTransport:
    """同步 stdio NDJSON 循环 (主 17:43 实事求是: 真阻塞读, 真写).

    用法:
        t = StdioTransport(dispatcher.dispatch)
        t.serve()              # 阻塞, 处理到 EOF
        t.serve_oneshot()      # 处理 1 个请求就退出 (test 友好)
    """

    def __init__(
        self,
        dispatch: Callable[[Dict[str, Any]], Optional[Dict[str, Any]]],
        stdin: Optional[TextIO] = None,
        stdout: Optional[TextIO] = None,
    ) -> None:
        self.dispatch = dispatch
        self.stdin = stdin or sys.stdin
        self.stdout = stdout or sys.stdout
        self.n_handled = 0
        self.n_errors = 0
        self._lock = threading.Lock()

    def _write(self, payload: Dict[str, Any]) -> None:
        with self._lock:
            self.stdout.write(json.dumps(payload, ensure_ascii=False))
            self.stdout.write("\n")
            self.stdout.flush()

    def serve(self) -> int:
        for raw_line in self.stdin:
            line = raw_line.strip()
            if not line:
                continue
            self._handle_line(line)
        return 0

    def serve_oneshot(self) -> Optional[Dict[str, Any]]:
        """test 入口: 读 1 行 → 1 响应."""
        line = self.stdin.readline()
        if not line:
            return None
        line = line.strip()
        if not line:
            return None
        # 不通过 self._handle_line (避免 stdout 写死)
        return self._dispatch_text(line, write_to_stdout=True)

    def _handle_line(self, line: str) -> Optional[Dict[str, Any]]:
        return self._dispatch_text(line, write_to_stdout=True)

    def _dispatch_text(self, line: str, write_to_stdout: bool) -> Optional[Dict[str, Any]]:
        if len(line) > NDJSON_LINE_LIMIT:
            resp = {
                "jsonrpc": "2.0", "id": None,
                "error": {"code": -32700, "message": "line too long"},
            }
            if write_to_stdout:
                self._write(resp)
            self.n_errors += 1
            return resp
        try:
            raw = json.loads(line)
        except json.JSONDecodeError as exc:
            resp = {
                "jsonrpc": "2.0", "id": None,
                "error": {"code": -32700, "message": f"JSON parse error: {exc}"},
            }
            if write_to_stdout:
                self._write(resp)
            self.n_errors += 1
            return resp
        resp = self.dispatch(raw)
        if resp is None:
            return None
        if write_to_stdout:
            self._write(resp)
        self.n_handled += 1
        return resp


# ---------------------------------------------------------------------------
# 2. HTTP /rpc transport
# ---------------------------------------------------------------------------


@dataclass
class HttpTransport:
    """POST /rpc 接 JSON-RPC envelope (主 17:43 实事求是: 不假装 SSE).

    host='127.0.0.1' (主 23:44 干到底: 默认 localhost, 避免暴露 0.0.0.0)
    port=0 → 动态分配 (test 友好)
    """

    dispatch: Callable[[Dict[str, Any]], Optional[Dict[str, Any]]]
    server_info: Dict[str, Any]
    host: str = "127.0.0.1"
    port: int = 0
    _server: Optional[ThreadingHTTPServer] = None
    _thread: Optional[threading.Thread] = None

    def __post_init__(self) -> None:
        if self._server is not None:
            return
        outer = self

        class _Handler(BaseHTTPRequestHandler):
            def log_message(self, fmt: str, *args: Any) -> None:  # noqa: A003
                # 静默 stderr (主 23:44 干到底: 不污染测试输出)
                pass

            def _send_json(self, status: int, payload: Dict[str, Any]) -> None:
                body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
                self.send_response(status)
                self.send_header("Content-Type", "application/json; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

            def do_GET(self) -> None:  # noqa: N802
                if self.path == "/health":
                    self._send_json(200, {
                        "ok": True, "server": outer.server_info.get("name", "apeireth-asi-mcp"),
                    })
                    return
                if self.path == "/tools":
                    self._send_json(200, {
                        "tools": outer.server_info.get("tools", []),
                    })
                    return
                self._send_json(404, {"error": "not found"})

            def do_POST(self) -> None:  # noqa: N802
                if self.path != "/rpc":
                    self._send_json(404, {"error": "not found"})
                    return
                length = int(self.headers.get("Content-Length", "0") or 0)
                if length <= 0 or length > 8 * 1024 * 1024:
                    self._send_json(400, {
                        "jsonrpc": "2.0", "id": None,
                        "error": {"code": -32600, "message": "invalid content length"},
                    })
                    return
                body = self.rfile.read(length)
                try:
                    raw = json.loads(body.decode("utf-8"))
                except json.JSONDecodeError as exc:
                    self._send_json(400, {
                        "jsonrpc": "2.0", "id": None,
                        "error": {"code": -32700, "message": f"JSON parse error: {exc}"},
                    })
                    return
                resp = outer.dispatch(raw)
                if resp is None:
                    self._send_json(204, {})
                    return
                self._send_json(200, resp)

        self._server = ThreadingHTTPServer((self.host, self.port), _Handler)
        # 把动态 port 同步回 self
        self.port = self._server.server_address[1]

    @property
    def actual_port(self) -> int:
        if self._server is None:
            return 0
        return int(self._server.server_address[1])

    def url(self) -> str:
        return f"http://{self.host}:{self.actual_port}/rpc"

    def start(self) -> None:
        if self._server is None or self._thread is not None:
            return
        server = self._server

        def _run() -> None:
            try:
                server.serve_forever()
            except Exception:  # noqa: BLE001
                pass

        self._thread = threading.Thread(target=_run, daemon=True, name="v1123-http")
        self._thread.start()

    def stop(self) -> None:
        if self._server is None:
            return
        try:
            self._server.shutdown()
        except Exception:  # noqa: BLE001
            pass
        self._server = None
        self._thread = None


__all__ = ["StdioTransport", "HttpTransport", "NDJSON_LINE_LIMIT"]
