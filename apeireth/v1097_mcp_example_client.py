"""V1097 R8 MCP Example Client — 演示外部 Agent 调用 v1097 MCP server.

主 22:33 ASI 北极星 + 主 23:44 干到底 + 主 20:46 不假装 + V3 + V1081.

用法:
    # 作为模块跑 (起一个 stdio server 子进程, 演示完整 lifecycle)
    python -m apeireth.v1097_mcp_example_client

    # 作为库 import, 连接已运行的 stdio server
    from apeireth.v1097_mcp_example_client import StdioMCPClient
    with StdioMCPClient() as client:
        result = client.call_tool("memory_add", {"content": "hello"})

    # 连接 SSE server
    from apeireth.v1097_mcp_example_client import HttpMCPClient
    client = HttpMCPClient("http://127.0.0.1:8765/rpc")
    print(client.call_tool("memory_get", {"memory_id": "abc"}))

设计要点 (主 19:33 走在前人经验上):
  1. StdioMCPClient: spawn 子进程 + NDJSON over stdin/stdout
  2. HttpMCPClient: POST /rpc (SSE server 接收)
  3. 自动等待 fsync: server 端 write 工具 fsync 后才响应 success
  4. 错误传递: JSON-RPC error → 抛出 MCPClientError, 不 swallow
  5. demo() 演示 7 工具各一次 + 一次 dream
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any, Dict, List, Optional


EXAMPLE_CLIENT_VERSION = "0.1.0"


class MCPClientError(RuntimeError):
    """MCP 调用错误 (JSON-RPC 错误或工具返回 isError=True)."""


# ============================================================================
# 1. StdioMCPClient — 子进程 + NDJSON
# ============================================================================


class StdioMCPClient:
    """连接 stdio MCP server 子进程.

    with StdioMCPClient(base_dir=tmp_path) as client:
        ...
    """

    def __init__(
        self,
        python_exe: Optional[str] = None,
        base: Optional[str] = None,
        server_args: Optional[List[str]] = None,
    ) -> None:
        self.python_exe = python_exe or sys.executable
        self.base = base or str(Path.home() / ".apeireth" / "mcp_client_demo")
        self.server_args = server_args or []
        self.proc: Optional[subprocess.Popen] = None
        self._req_id = 0
        self._lock = threading.Lock()
        self._stderr_thread: Optional[threading.Thread] = None

    # ----- lifecycle -----

    def __enter__(self) -> "StdioMCPClient":
        self.start()
        return self

    def __exit__(self, *exc: Any) -> None:
        self.stop()

    def start(self) -> None:
        # 准备 base dir (每次启动前清空, 保证 demo 干净)
        Path(self.base).mkdir(parents=True, exist_ok=True)
        cmd = [
            self.python_exe, "-m", "apeireth.v1097_mcp_memory_server",
            "--serve", "--transport", "stdio",
            "--base", self.base,
        ] + list(self.server_args)
        env = {**os.environ, "PYTHONIOENCODING": "utf-8"}
        self.proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            bufsize=0,
        )
        # 后台收 stderr 防止 pipe buffer 满
        self._stderr_thread = threading.Thread(
            target=self._drain_stderr, daemon=True,
        )
        self._stderr_thread.start()
        # initialize handshake
        init = self.call("initialize", {
            "protocolVersion": "2024-11-05",
            "clientInfo": {"name": "v1097-example-client", "version": EXAMPLE_CLIENT_VERSION},
            "capabilities": {},
        })
        # 发送 initialized 通知
        self.notify("notifications/initialized", {})

    def stop(self) -> None:
        if self.proc is None:
            return
        try:
            if self.proc.stdin:
                self.proc.stdin.close()
        except OSError:
            pass
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()
        self.proc = None

    def _drain_stderr(self) -> None:
        if self.proc is None or self.proc.stderr is None:
            return
        for line in self.proc.stderr:
            try:
                sys.stderr.write("[server] " + line.decode("utf-8", errors="replace"))
            except Exception:
                pass

    # ----- request / notify -----

    def call(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        with self._lock:
            self._req_id += 1
            req_id = self._req_id
            req = {
                "jsonrpc": "2.0",
                "id": req_id,
                "method": method,
                "params": params,
            }
            line = (json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8")
            assert self.proc is not None and self.proc.stdin is not None
            self.proc.stdin.write(line)
            self.proc.stdin.flush()
            # 同步读 response (按 id 匹配; 简化: 按顺序读)
            assert self.proc.stdout is not None
            while True:
                raw = self.proc.stdout.readline()
                if not raw:
                    raise MCPClientError("server closed stdout")
                try:
                    msg = json.loads(raw.decode("utf-8"))
                except json.JSONDecodeError:
                    continue
                if msg.get("id") == req_id:
                    if "error" in msg:
                        raise MCPClientError(
                            f"RPC error {msg['error'].get('code')}: {msg['error'].get('message')}"
                        )
                    return msg.get("result", {})

    def notify(self, method: str, params: Dict[str, Any]) -> None:
        with self._lock:
            req = {"jsonrpc": "2.0", "method": method, "params": params}
            line = (json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8")
            assert self.proc is not None and self.proc.stdin is not None
            self.proc.stdin.write(line)
            self.proc.stdin.flush()

    # ----- tool helpers -----

    def list_tools(self) -> List[Dict[str, Any]]:
        return self.call("tools/list", {}).get("tools", [])

    def call_tool(self, name: str, arguments: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        res = self.call("tools/call", {"name": name, "arguments": arguments or {}})
        if res.get("isError"):
            text = ""
            for c in res.get("content", []):
                if c.get("type") == "text":
                    text = c.get("text", "")
                    break
            raise MCPClientError(f"tool {name} failed: {text}")
        for c in res.get("content", []):
            if c.get("type") == "json":
                return c["data"]
        return {}


# ============================================================================
# 2. HttpMCPClient — POST /rpc (against SSE server)
# ============================================================================


class HttpMCPClient:
    """用 urllib (stdlib only) 连接 SSE server 的 /rpc 端点."""

    def __init__(self, rpc_url: str, auth_token: Optional[str] = None) -> None:
        self.rpc_url = rpc_url
        self.auth_token = auth_token
        self._req_id = 0
        self._lock = threading.Lock()

    def _headers(self) -> Dict[str, str]:
        headers = {"Content-Type": "application/json"}
        if self.auth_token:
            headers["Authorization"] = f"Bearer {self.auth_token}"
        return headers

    def call(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        import urllib.request
        import urllib.error
        with self._lock:
            self._req_id += 1
            req_id = self._req_id
        req = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params,
        }
        data = json.dumps(req, ensure_ascii=False).encode("utf-8")
        rq = urllib.request.Request(
            self.rpc_url, data=data,
            headers=self._headers(),
        )
        try:
            with urllib.request.urlopen(rq, timeout=30) as resp:
                raw = resp.read()
                if not raw:
                    return {}
                msg = json.loads(raw.decode("utf-8"))
        except urllib.error.HTTPError as e:
            raise MCPClientError(f"HTTP {e.code}: {e.read().decode('utf-8', 'replace')}") from e
        if "error" in msg:
            raise MCPClientError(
                f"RPC error {msg['error'].get('code')}: {msg['error'].get('message')}"
            )
        return msg.get("result", {})

    def notify(self, method: str, params: Dict[str, Any]) -> None:
        # HTTP transport: 通知 → 服务端不响应, 仍然 POST 但丢弃响应
        import urllib.request
        req = {"jsonrpc": "2.0", "method": method, "params": params}
        data = json.dumps(req, ensure_ascii=False).encode("utf-8")
        rq = urllib.request.Request(
            self.rpc_url, data=data,
            headers=self._headers(),
        )
        try:
            with urllib.request.urlopen(rq, timeout=5) as resp:
                resp.read()
        except Exception:
            pass

    def list_tools(self) -> List[Dict[str, Any]]:
        return self.call("tools/list", {}).get("tools", [])

    def call_tool(self, name: str, arguments: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        res = self.call("tools/call", {"name": name, "arguments": arguments or {}})
        if res.get("isError"):
            text = ""
            for c in res.get("content", []):
                if c.get("type") == "text":
                    text = c.get("text", "")
                    break
            raise MCPClientError(f"tool {name} failed: {text}")
        for c in res.get("content", []):
            if c.get("type") == "json":
                return c["data"]
        return {}


# ============================================================================
# 3. Demo — 跑一个完整 lifecycle
# ============================================================================


def demo_stdio() -> Dict[str, Any]:
    """启动 stdio server 子进程, 演示 7 工具各一次 + dream."""
    import tempfile
    base = tempfile.mkdtemp(prefix="v1097_mcp_demo_")
    print(f"=== V1097 MCP Demo (stdio) — base={base} ===")
    with StdioMCPClient(base=base) as client:
        # 1. tools/list
        tools = client.list_tools()
        names = [t["name"] for t in tools]
        print(f"  [1] tools/list → {len(tools)} tools: {names}")
        assert set(names) == {
            "memory_add", "memory_search", "memory_get",
            "identity_get", "identity_set_persona",
            "memory_replay", "memory_dream",
        }

        # 2. memory_add episode
        ep = client.call_tool("memory_add", {
            "content": "ASI 北极星: 真生产不停, 数字涨不涨不重要",
            "kind": "episode", "actor": "external_agent",
            "tags": ["philosophy", "ASI"], "importance": 0.6,
        })
        ep_id = ep["id"]
        print(f"  [2] memory_add episode → id={ep_id}, checksum={ep['checksum']}")

        # 3. memory_add note
        nt = client.call_tool("memory_add", {
            "content": "V3 守门: 中央 AI 完整位置 = 调度 + 思考 + 关系集合体",
            "kind": "note", "actor": "external_agent",
            "tags": ["philosophy", "V3"], "importance": 0.65,
            "evidence": [ep_id],
        })
        nt_id = nt["id"]
        print(f"  [3] memory_add note → id={nt_id}, wal_seq={nt['wal_sequence']}")

        # 4. memory_search
        sr = client.call_tool("memory_search", {"tags": ["philosophy"], "limit": 10})
        print(f"  [4] memory_search → count={sr['count']}")

        # 5. memory_get
        got = client.call_tool("memory_get", {"memory_id": ep_id})
        print(f"  [5] memory_get → kind={got.get('kind')}, content[:20]={got.get('content', '')[:20]}")

        # 6. identity_get
        idc = client.call_tool("identity_get", {})
        print(f"  [6] identity_get → name={idc.get('name')}, v={idc.get('version')}")

        # 7. identity_set_persona
        per = client.call_tool("identity_set_persona", {
            "persona": {"stance": "ASI 北极星 + V3 守门 + 真生产不停", "demo_run": True},
        })
        print(f"  [7] identity_set_persona → keys={list(per['persona'].keys())}")

        # 8. memory_replay
        now = time.time()
        rp = client.call_tool("memory_replay", {
            "from_ts": now - 60, "to_ts": now + 5,
        })
        print(f"  [8] memory_replay → events={rp['count']}, skipped={rp['skipped']}")

        # 9. memory_dream
        dr = client.call_tool("memory_dream", {"top_k": 3})
        print(f"  [9] memory_dream → clusters={dr['count']}")
        for ins in dr.get("insights", [])[:3]:
            print(f"        - {ins[:80]}")

    print("=== Demo OK ===")
    return {"ok": True, "base": base}


def demo_http(rpc_url: str = "http://127.0.0.1:8765/rpc") -> Dict[str, Any]:
    """连接已运行的 SSE server, 演示 1-2 个工具."""
    print(f"=== V1097 MCP Demo (HTTP) — {rpc_url} ===")
    client = HttpMCPClient(rpc_url)
    idc = client.call_tool("identity_get", {})
    print(f"  identity_get → name={idc.get('name')}")
    sr = client.call_tool("memory_search", {"query": "", "limit": 5})
    print(f"  memory_search → count={sr['count']}")
    return {"ok": True, "name": idc.get("name")}


def cli_main(argv: Optional[List[str]] = None) -> int:
    import argparse
    p = argparse.ArgumentParser(prog="v1097_mcp_example_client")
    p.add_argument("--transport", choices=("stdio", "http"), default="stdio")
    p.add_argument("--url", default="http://127.0.0.1:8765/rpc")
    args = p.parse_args(argv)
    if args.transport == "stdio":
        demo_stdio()
    else:
        demo_http(args.url)
    return 0


__all__ = [
    "EXAMPLE_CLIENT_VERSION",
    "MCPClientError",
    "StdioMCPClient",
    "HttpMCPClient",
    "demo_stdio",
    "demo_http",
]


if __name__ == "__main__":
    raise SystemExit(cli_main())


# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
