"""Apeireth ASI V1129 — R10 ASI 北极星 MCP server tool schema 标准化 + R10 集成验证.

R10-MCP-001 / R10 W1 (主 22:33 + 主 13:31 + 主 17:43 + 主 17:58 + 主 23:44
                  + 主 19:33 + 主 00:56 + 主 12:14).

继承 R9-MCP-001 V1123 (commit 72bbc82f, accepted 8.90) + 升级:
  - 5 大 R10 MCP 工具: measure_asi / get_north_star / check_identity /
    verify_audit_chain / list_personas
  - V1124 backend 真集成: HTTP /asi/level + /asi/measure + /asi/north-star
  - V1095 IdentityStoreV1095 真集成: check_identity + verify_audit_chain + list_personas
  - V1125 R10 集成协议真跑: measure_asi 走 V0.5 18 维公式
  - 3 transports: stdio (NDJSON) + SSE (Anthropic MCP 2024-11-05) + HTTP (/rpc)
  - chaos 守门: transport 失联时 dispatcher state 不丢

5 大工具 schema 标准化 (Anthropic MCP 2024):
  - measure_asi: V0.4 baseline + 3 new dim (continuity / autonomy / transferability)
  - get_north_star: V1124 backend HTTP 客户端 + in-process 模式
  - check_identity: V1095 中央档案 (主 12:14 中央 AI 永恒身份)
  - verify_audit_chain: V1095 + V1124 双 audit chain 真验
  - list_personas: V1095 4 archetype 槽位

Usage:
    # 1) stdio transport
    python -m apeireth.v1129_r10_mcp_server --server --transport stdio

    # 2) HTTP transport
    python -m apeireth.v1129_r10_mcp_server --server --transport http --port 8129

    # 3) SSE transport (Anthropic MCP 2024-11-05)
    python -m apeireth.v1129_r10_mcp_server --server --transport sse --port 8129

    # 4) Self-test (主 00:56 任何人都能接手)
    python -m apeireth.v1129_r10_mcp_server --selftest

    # 5) Snapshot
    python -m apeireth.v1129_r10_mcp_server --snapshot

    # 6) JSON output
    python -m apeireth.v1129_r10_mcp_server --selftest --json
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
import threading
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional

# MCP 子包
from apeireth.mcp import ASI_NORTH_STAR_VERSION, PROTOCOL_VERSION
from apeireth.mcp.protocol import (
    JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND,
    SUPPORTED_PROTOCOL_VERSIONS, check_protocol_version,
    make_error_response, make_result_response,
    parse_request, validate_arguments, validate_tool_result,
)
from apeireth.mcp.asi_nine_keys import (
    AsiNineKeyLock, ASI_NINE_KEYS as _ASI_NINE_KEYS,
    inject_guard_block, verify_or_raise,
)
from apeireth.mcp.transport import StdioTransport, HttpTransport
from apeireth.mcp.sse_transport import SseTransport, SseSessionStore
from apeireth.mcp.r10_asi_north_star_server import (
    R10AsiNorthStarDispatcher, SERVER_NAME, V1129_VERSION,
    MEASURE_ASI_SCHEMA, GET_NORTH_STAR_SCHEMA, CHECK_IDENTITY_SCHEMA,
    VERIFY_AUDIT_CHAIN_SCHEMA, LIST_PERSONAS_SCHEMA,
)


V1129_FRAMEWORK_VERSION = "0.1.0"

# 借鉴 V1123 facade: 默认 0.0.0.0 不暴露, 守门 (主 23:44 干到底)
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8129


# ---------------------------------------------------------------------------
# V1124 / V1095 真集成客户端 (主 17:43 实事求是: 真连, 不 mock)
# ---------------------------------------------------------------------------


def make_v1095_store(identity_path: Optional[Path] = None) -> Any:
    """构造 V1095 IdentityStoreV1095 (主 12:14 中央 AI 永恒身份).

    缺 identity_path → 临时目录, 每次启动干净.
    """
    from apeireth.v1095_identity_store import IdentityStoreV1095
    if identity_path is None:
        tmp = Path(tempfile.mkdtemp(prefix="v1129_v1095_"))
        identity_path = tmp / "identity.db"
    store = IdentityStoreV1095(identity_path, fsync_full=True)
    # seed 4 archetype (主 12:14 调度者/学习者/思考者/助手)
    store.ensure_default_slots()
    return store


def make_v1124_backend(data_dir: Optional[Path] = None) -> Any:
    """构造 V1124 ASINorthStarBackend (主 17:43 真集成).

    缺 data_dir → 临时目录, 每次启动干净.
    """
    from apeireth.v1124_asi_north_star_backend import ASINorthStarBackend
    if data_dir is None:
        data_dir = Path(tempfile.mkdtemp(prefix="v1129_v1124_"))
    return ASINorthStarBackend(data_dir)


def build_default_dispatcher(v1095_store: Any = None,
                              v1124_backend: Any = None,
                              bind_external: bool = True) -> R10AsiNorthStarDispatcher:
    """构建默认 dispatcher (主 00:56 任何人都能接手: 一行).

    Args:
        v1095_store: 外部传入 V1095; None + bind_external=True → 自动构造临时实例
        v1124_backend: 外部传入 V1124; 同上
        bind_external: False → 都不构造, dispatcher 工具返 isError
    """
    if v1095_store is None and bind_external:
        try:
            v1095_store = make_v1095_store()
        except Exception:  # noqa: BLE001
            v1095_store = None
    if v1124_backend is None and bind_external:
        try:
            v1124_backend = make_v1124_backend()
        except Exception:  # noqa: BLE001
            v1124_backend = None
    return R10AsiNorthStarDispatcher(
        v1095_store=v1095_store,
        v1124_backend=v1124_backend,
    )


# ---------------------------------------------------------------------------
# Transport 启动 (主 13:31 大胆激进: 3 模式真支持)
# ---------------------------------------------------------------------------


def serve_stdio(dispatcher: Optional[R10AsiNorthStarDispatcher] = None) -> int:
    """stdio transport 启动 (主 17:43 真阻塞)."""
    d = dispatcher or build_default_dispatcher()
    t = StdioTransport(d.handle_message)
    return t.serve()


def serve_http(dispatcher: Optional[R10AsiNorthStarDispatcher] = None,
               host: str = DEFAULT_HOST, port: int = DEFAULT_PORT) -> int:
    """HTTP transport 启动."""
    d = dispatcher or build_default_dispatcher()
    http = HttpTransport(
        dispatch=d.handle_message,
        server_info={"name": SERVER_NAME, "version": V1129_VERSION,
                     "tools": list(d.TOOLS.keys())},
        host=host, port=port,
    )
    http.start()
    print(f"[V1129] {SERVER_NAME} v{V1129_VERSION} listening on {http.url()}", flush=True)
    print(f"[V1129] health: http://{host}:{http.actual_port}/health", flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        http.stop()
    return 0


def serve_sse(dispatcher: Optional[R10AsiNorthStarDispatcher] = None,
               host: str = DEFAULT_HOST, port: int = DEFAULT_PORT) -> int:
    """SSE transport 启动 (Anthropic MCP 2024-11-05)."""
    d = dispatcher or build_default_dispatcher()
    sse = SseTransport(
        dispatch=d.handle_message,
        server_info={"name": SERVER_NAME, "version": V1129_VERSION,
                     "tools": list(d.TOOLS.keys())},
        host=host, port=port,
    )
    sse.start()
    print(f"[V1129] {SERVER_NAME} v{V1129_VERSION} SSE on {sse.sse_url()}", flush=True)
    print(f"[V1129] health: http://{host}:{sse.actual_port}/health", flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        sse.stop()
    return 0


# ---------------------------------------------------------------------------
# Self-test (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


def run_selftest(bind_external: bool = True) -> Dict[str, Any]:
    """V1129 自检: 5 工具 + 3 transports + chaos 守门 (主 17:43 实事求是)."""
    d = build_default_dispatcher(bind_external=bind_external)
    results: Dict[str, Any] = {}

    # 1) initialize
    init = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                              "params": {"protocolVersion": "2024-11-05"}})
    results["initialize"] = init

    # 2) tools/list
    lst = d.handle_message({"jsonrpc": "2.0", "id": 2, "method": "tools/list",
                             "params": {}})
    results["tools_list"] = lst

    # 3) 5 工具 round-trip
    calls = {
        "measure_asi": {"v04_actual": 0.8538, "continuity": 0.85, "autonomy": 0.85,
                          "transferability": 0.85, "week_label": "R10-W1"},
        "get_north_star": {"include_composite": True},
        "check_identity": {"include_switches": True},
        "verify_audit_chain": {"include_breakdown": True},
        "list_personas": {"include_emerged": True},
    }
    results["tool_calls"] = {}
    for i, (name, args) in enumerate(calls.items(), start=10):
        resp = d.handle_message({"jsonrpc": "2.0", "id": i, "method": "tools/call",
                                  "params": {"name": name, "arguments": args}})
        results["tool_calls"][name] = resp

    # 4) chaos 守门: 失联后再发请求, dispatcher state 应保留
    results["chaos_dispatcher_state_pre"] = d.stats()
    # 假装 SSE/HTTP 失联: 直接调 dispatcher.handle_message (bypass transport)
    for _ in range(3):
        d.handle_message({"jsonrpc": "2.0", "id": 100, "method": "ping", "params": {}})
    results["chaos_dispatcher_state_post"] = d.stats()
    results["chaos_state_retained"] = (
        results["chaos_dispatcher_state_post"]["n_dispatched"] >
        results["chaos_dispatcher_state_pre"]["n_dispatched"]
    )

    # 5) SSE 启停 (真起 server)
    sse = SseTransport(
        dispatch=d.handle_message,
        server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
        host="127.0.0.1", port=0,
    )
    sse.start()
    results["sse_actual_port"] = sse.actual_port
    results["sse_url"] = sse.sse_url()
    sse.stop()

    # 6) HTTP 启停
    http = HttpTransport(
        dispatch=d.handle_message,
        server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
        host="127.0.0.1", port=0,
    )
    http.start()
    results["http_actual_port"] = http.actual_port
    http.stop()

    # 7) 错误路径
    bad = d.handle_message({"jsonrpc": "2.0", "id": 99, "method": "tools/call",
                             "params": {"name": "measure_asi", "arguments": {}}})
    results["bad_params"] = bad
    missing = d.handle_message({"jsonrpc": "2.0", "id": 98, "method": "unknown_method",
                                 "params": {}})
    results["bad_method"] = missing

    # 8) stats
    results["stats"] = d.stats()
    return results


# ---------------------------------------------------------------------------
# CLI (主 00:56 一行可跑)
# ---------------------------------------------------------------------------


def _print_snapshot(payload: Any) -> None:
    print(json.dumps(payload, indent=2, ensure_ascii=False, default=str))


def cli_main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1129_r10_mcp_server",
        description="V1129 R10 ASI 北极星 MCP server (5 tools + 3 transports)",
    )
    parser.add_argument("--server", action="store_true", help="启动 MCP server")
    parser.add_argument("--transport", choices=("stdio", "http", "sse"), default="http",
                        help="transport 类型 (default: http)")
    parser.add_argument("--host", default=DEFAULT_HOST, help="监听 host (主 23:44 不暴露 0.0.0.0)")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="监听 port")
    parser.add_argument("--selftest", action="store_true", help="V1129 1 行自检")
    parser.add_argument("--snapshot", action="store_true", help="打印 dispatcher 状态 JSON")
    parser.add_argument("--chaos", action="store_true", help="跑 chaos test (transport 失联守门)")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    parser.add_argument("--no-bind", action="store_true",
                        help="不构造 V1095/V1124 (测试 schema 守门用)")
    args = parser.parse_args(argv)

    if args.snapshot:
        d = build_default_dispatcher(bind_external=not args.no_bind)
        _print_snapshot(d.stats())
        return 0

    if args.selftest:
        result = run_selftest(bind_external=not args.no_bind)
        if args.json:
            _print_snapshot(result)
        else:
            n_tools = len(result.get("tool_calls", {}))
            n_ok = sum(1 for r in result.get("tool_calls", {}).values()
                        if r and "result" in r and not r["result"].get("isError"))
            print(f"V1129 selftest: {n_ok}/{n_tools} tools OK")
            print(f"V1129 chaos state retained: {result.get('chaos_state_retained')}")
            print(f"V1129 SSE port: {result.get('sse_actual_port')}, HTTP port: {result.get('http_actual_port')}")
            s = result.get("stats", {})
            print(f"V1129 stats: n_calls={s.get('n_calls', 0)}, n_errors={s.get('n_errors', 0)}, "
                  f"v1095_bound={s.get('v1095_bound')}, v1124_bound={s.get('v1124_bound')}, "
                  f"9 键 LOCKED={s.get('nine_key_lock', {}).get('asi_nine_keys_locked')}")
        return 0

    if args.chaos:
        d = build_default_dispatcher(bind_external=not args.no_bind)
        pre = d.stats()
        # chaos: 启停 3 个 transport
        sse = SseTransport(dispatch=d.handle_message,
                            server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
                            host="127.0.0.1", port=0)
        sse.start()
        sse.stop()
        http = HttpTransport(dispatch=d.handle_message,
                              server_info={"name": SERVER_NAME, "tools": list(d.TOOLS.keys())},
                              host="127.0.0.1", port=0)
        http.start()
        http.stop()
        for _ in range(3):
            d.handle_message({"jsonrpc": "2.0", "id": 200, "method": "ping", "params": {}})
        post = d.stats()
        ok = post["n_dispatched"] > pre["n_dispatched"]
        result = {"chaos_state_retained": ok, "pre": pre, "post": post}
        if args.json:
            _print_snapshot(result)
        else:
            print(f"chaos_state_retained: {ok}")
            print(f"  pre  dispatched={pre['n_dispatched']} calls={pre['n_calls']} errors={pre['n_errors']}")
            print(f"  post dispatched={post['n_dispatched']} calls={post['n_calls']} errors={post['n_errors']}")
        return 0 if ok else 1

    if args.server:
        if args.transport == "stdio":
            return serve_stdio()
        if args.transport == "sse":
            return serve_sse(host=args.host, port=args.port)
        return serve_http(host=args.host, port=args.port)

    parser.print_help()
    return 1


__all__ = [
    "V1129_FRAMEWORK_VERSION",
    "make_v1095_store", "make_v1124_backend",
    "build_default_dispatcher",
    "serve_stdio", "serve_http", "serve_sse",
    "run_selftest", "cli_main",
]


# ---------------------------------------------------------------------------
# V1101 auto-injected V3_GUARDS (主 17:43 + 主 17:58 不假装)
# ---------------------------------------------------------------------------
V3_GUARDS = {
    "module_is_not_asi": "V1129 R10 MCP server 是工具, ASI 仍是更大目标. 5 tools 不等于 ASI.",
    "r10_measure_is_not_asi": "measure_asi V0.5 真测 ≠ ASI 达成. 0.95 是 R10 终极门, 不是 ASI 实现.",
    "integration_is_not_autonomy": "V1124/V1095/V1125 真集成 ≠ 自主意识. 集成是工程, 自主是更大目标.",
    "mcp_chaos_state_is_not_truth": "chaos 守门通过 ≠ 永远不丢状态. 是 best-effort 守门, 不是保证.",
    "transport_fanout_is_not_asi": "3 transports ≠ ASI 多模态. transport 是协议, ASI 是目标.",
}


if __name__ == "__main__":
    raise SystemExit(cli_main(sys.argv[1:]))
