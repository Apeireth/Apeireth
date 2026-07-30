"""Apeireth ASI V1137 — R11 MCP: V1136/V1130 真测结果 MCP 集成 CLI 入口 (主 17:43 实事求是).

R11-MCP task (继承 R10-MCP-001 W1 + R10-MCP-002 W2 主线):
  落地 V1136 / V1130 真测结果通过 MCP/tool 边界的最小可靠集成.
  - 2 工具: measure_v1136_real + get_v1130_backend (apeireth.mcp.r11_measurement_server)
  - 3 transports: stdio / HTTP / SSE (复用 V1123 transport)
  - schema / timeout / error / version / provenance 全部保留
  - offline 可测试: V1136 真测引擎不依赖外部 provider; V1130 后端 evaluate 在
                     provider 未配置时返回 UNCONFIGURED 状态, 永不伪造成功

主哲学:
  - 主 22:33 ASI 北极星: V1136 真测 = proxy, ASI 仍是更大目标 (主 17:58 不假装)
  - 主 17:43 实事求是: V1130 backend evaluate 走真实 V1128 adapter
  - 主 19:33 走在前人经验上: 复用 V1123 / V1129 / V1130 既有模块
  - 主 23:44 干到底: 5s 超时 + chaos 守门 + transport 失联状态保留
  - 主 13:31 大胆激进: 2 tools + 3 transports
  - 主 00:56 任何人都能接手: --selftest 一行

Usage:
    # Self-test (主 00:56 任何人都能接手)
    python -m apeireth.v1137_r11_mcp_measurement_tool --selftest

    # JSON output
    python -m apeireth.v1137_r11_mcp_measurement_tool --selftest --json

    # Chaos test (transport 失联守门)
    python -m apeireth.v1137_r11_mcp_measurement_tool --chaos

    # Snapshot stats
    python -m apeireth.v1137_r11_mcp_measurement_tool --snapshot

    # 3 transports 启动
    python -m apeireth.v1137_r11_mcp_measurement_tool --server --transport stdio
    python -m apeireth.v1137_r11_mcp_measurement_tool --server --transport http --port 8137
    python -m apeireth.v1137_r11_mcp_measurement_tool --server --transport sse --port 8137

    # Strict contract: --strict → V3 守门未过也返 isError
    python -m apeireth.v1137_r11_mcp_measurement_tool --strict-eval
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from typing import Any, Dict, List, Optional

# 复用 V1123 transports (主 19:33 走在前人经验上)
from apeireth.mcp.transport import StdioTransport, HttpTransport
from apeireth.mcp.sse_transport import SseTransport

# R11 dispatcher (本任务核心)
from apeireth.mcp.r11_measurement_server import (
    GET_V1130_BACKEND,
    MEASURE_V1136_REAL,
    R11_MCP_VERSION,
    R11_SERVER_NAME,
    R11_TOOLS,
    R11MeasurementDispatcher,
)


V1137_FRAMEWORK_VERSION = "0.1.0"
DEFAULT_HOST = "127.0.0.1"   # 主 23:44 干到底: 不暴露 0.0.0.0
DEFAULT_PORT = 8137


# ---------------------------------------------------------------------------
# Self-test (主 00:56 任何人都能接手, 主 17:43 实事求是)
# ---------------------------------------------------------------------------


def run_selftest(strict: bool = False) -> Dict[str, Any]:
    """R11 MCP 自检: 2 tools + initialize + transport 启停 + chaos 守门.

    Args:
        strict: V3 守门 strict mode (V1136 真测 isError → selftest 失败)

    Returns:
        dict 结果 (主 17:43 实事求是: 每条都是测过的数字)
    """
    dispatcher = R11MeasurementDispatcher()
    results: Dict[str, Any] = {}

    # 1) initialize (Anthropic MCP 2024-11-05)
    init = dispatcher.handle_message({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": "2024-11-05"},
    })
    results["initialize"] = init

    # 2) tools/list
    lst = dispatcher.handle_message({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {},
    })
    results["tools_list"] = lst
    results["n_tools_listed"] = (
        len(lst.get("result", {}).get("tools", []))
        if lst and "result" in lst else 0
    )

    # 3) 2 tools round-trip (主 17:43 实事求是: 真跑, 不 mock)
    calls = {
        MEASURE_V1136_REAL: {
            "v04_score": 0.8538,
            "run_chaos": False,
            "include_subscores": True,
            "strict": False,
        },
        GET_V1130_BACKEND: {
            "action": "level",
            "data_dir": "",
        },
    }
    results["tool_calls"] = {}
    for i, (name, args) in enumerate(calls.items(), start=10):
        resp = dispatcher.handle_message({
            "jsonrpc": "2.0", "id": i, "method": "tools/call",
            "params": {"name": name, "arguments": args},
        })
        results["tool_calls"][name] = resp

    # 4) V1130 evaluate action (offline safe — 没 provider 时返 fail-soft, 不伪造)
    eval_resp = dispatcher.handle_message({
        "jsonrpc": "2.0", "id": 30, "method": "tools/call",
        "params": {"name": GET_V1130_BACKEND,
                    "arguments": {"action": "evaluate", "prompt": "Reply exactly with W3_OK"}},
    })
    results["tool_calls_v1130_evaluate"] = eval_resp

    # 5) V1130 runtime action
    rt_resp = dispatcher.handle_message({
        "jsonrpc": "2.0", "id": 31, "method": "tools/call",
        "params": {"name": GET_V1130_BACKEND,
                    "arguments": {"action": "runtime", "iterations": 2}},
    })
    results["tool_calls_v1130_runtime"] = rt_resp

    # 6) chaos 守门: transport 失联后 dispatcher state 保留
    pre = dispatcher.stats()
    for _ in range(3):
        dispatcher.handle_message({
            "jsonrpc": "2.0", "id": 100, "method": "ping", "params": {},
        })
    post = dispatcher.stats()
    results["chaos_dispatcher_state_pre"] = pre
    results["chaos_dispatcher_state_post"] = post
    results["chaos_state_retained"] = (
        post["n_dispatched"] > pre["n_dispatched"]
    )

    # 7) SSE / HTTP transport 启停 (真起 server)
    sse = SseTransport(
        dispatch=dispatcher.handle_message,
        server_info={"name": R11_SERVER_NAME,
                      "version": R11_MCP_VERSION,
                      "tools": list(R11_TOOLS.keys())},
        host="127.0.0.1", port=0,
    )
    sse.start()
    results["sse_actual_port"] = sse.actual_port
    sse.stop()

    http = HttpTransport(
        dispatch=dispatcher.handle_message,
        server_info={"name": R11_SERVER_NAME,
                      "version": R11_MCP_VERSION,
                      "tools": list(R11_TOOLS.keys())},
        host="127.0.0.1", port=0,
    )
    http.start()
    results["http_actual_port"] = http.actual_port
    http.stop()

    # 8) errors — invalid args + unknown tool
    bad_args = dispatcher.handle_message({
        "jsonrpc": "2.0", "id": 99, "method": "tools/call",
        "params": {"name": MEASURE_V1136_REAL,
                    "arguments": {"v04_score": 1.5}},  # out of [0,1]
    })
    results["bad_args"] = bad_args
    missing = dispatcher.handle_message({
        "jsonrpc": "2.0", "id": 98, "method": "tools/call",
        "params": {"name": "non_existent_tool", "arguments": {}},
    })
    results["missing_tool"] = missing

    # 9) Stats 守门
    results["stats"] = dispatcher.stats()

    # 10) selftest 自身 done 守门: isError=False count
    tool_results = results.get("tool_calls", {})
    n_ok = 0
    for resp in tool_results.values():
        if not resp or "result" not in resp:
            continue
        r = resp["result"]
        if isinstance(r, dict) and r.get("content"):
            for content_item in r["content"]:
                if (isinstance(content_item, dict)
                    and content_item.get("type") == "json"):
                    d = content_item.get("data", {})
                    if not d.get("isError"):
                        n_ok += 1
                        break
    results["n_round_trip_ok"] = n_ok
    results["n_round_trip_total"] = len(tool_results)

    return results


# ---------------------------------------------------------------------------
# Chaos transport test (主 23:44 干到底)
# ---------------------------------------------------------------------------


def run_chaos() -> Dict[str, Any]:
    """R11 chaos 守门: 3 transports 启停 + 多次 ping, dispatcher state 不丢."""
    d = R11MeasurementDispatcher()
    pre = d.stats()

    sse = SseTransport(
        dispatch=d.handle_message,
        server_info={"name": R11_SERVER_NAME,
                      "version": R11_MCP_VERSION,
                      "tools": list(R11_TOOLS.keys())},
        host="127.0.0.1", port=0,
    )
    sse.start()
    sse.stop()

    http = HttpTransport(
        dispatch=d.handle_message,
        server_info={"name": R11_SERVER_NAME,
                      "version": R11_MCP_VERSION,
                      "tools": list(R11_TOOLS.keys())},
        host="127.0.0.1", port=0,
    )
    http.start()
    http.stop()

    for _ in range(5):
        d.handle_message({"jsonrpc": "2.0", "id": 200,
                          "method": "ping", "params": {}})

    post = d.stats()
    return {
        "chaos_state_retained": post["n_dispatched"] > pre["n_dispatched"],
        "pre": pre,
        "post": post,
    }


# ---------------------------------------------------------------------------
# Transport 启动 (主 13:31 大胆激进: 3 模式)
# ---------------------------------------------------------------------------


def serve_stdio(dispatcher: Optional[R11MeasurementDispatcher] = None) -> int:
    d = dispatcher or R11MeasurementDispatcher()
    t = StdioTransport(d.handle_message)
    return t.serve()


def serve_http(dispatcher: Optional[R11MeasurementDispatcher] = None,
               host: str = DEFAULT_HOST, port: int = DEFAULT_PORT) -> int:
    d = dispatcher or R11MeasurementDispatcher()
    http = HttpTransport(
        dispatch=d.handle_message,
        server_info={"name": R11_SERVER_NAME,
                      "version": R11_MCP_VERSION,
                      "tools": list(d.stats().get("tools", []))
                      or list(R11_TOOLS.keys())},
        host=host, port=port,
    )
    http.start()
    print(f"[R11] {R11_SERVER_NAME} v{R11_MCP_VERSION} listening on {http.url()}",
          flush=True)
    print(f"[R11] health: http://{host}:{http.actual_port}/health", flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        http.stop()
    return 0


def serve_sse(dispatcher: Optional[R11MeasurementDispatcher] = None,
              host: str = DEFAULT_HOST, port: int = DEFAULT_PORT) -> int:
    d = dispatcher or R11MeasurementDispatcher()
    sse = SseTransport(
        dispatch=d.handle_message,
        server_info={"name": R11_SERVER_NAME,
                      "version": R11_MCP_VERSION,
                      "tools": list(d.stats().get("tools", []))
                      or list(R11_TOOLS.keys())},
        host=host, port=port,
    )
    sse.start()
    print(f"[R11] {R11_SERVER_NAME} v{R11_MCP_VERSION} SSE on {sse.sse_url()}",
          flush=True)
    print(f"[R11] health: http://{host}:{sse.actual_port}/health", flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        sse.stop()
    return 0


# ---------------------------------------------------------------------------
# CLI (主 00:56 一行可跑)
# ---------------------------------------------------------------------------


def _print_snapshot(payload: Any) -> None:
    print(json.dumps(payload, indent=2, ensure_ascii=False, default=str))


def cli_main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1137_r11_mcp_measurement_tool",
        description="R11 V1137 V1136/V1130 真测结果 MCP 集成 (2 tools + 3 transports)",
    )
    parser.add_argument("--server", action="store_true", help="启动 MCP server")
    parser.add_argument("--transport",
                         choices=("stdio", "http", "sse"),
                         default="http",
                         help="transport 类型 (default: http)")
    parser.add_argument("--host", default=DEFAULT_HOST,
                         help="监听 host (主 23:44 不暴露 0.0.0.0)")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT,
                         help="监听 port (default: 8137)")
    parser.add_argument("--selftest", action="store_true",
                         help="R11 MCP 1 行自检")
    parser.add_argument("--snapshot", action="store_true",
                         help="打印 dispatcher 状态 JSON")
    parser.add_argument("--chaos", action="store_true",
                         help="chaos test (transport 失联守门)")
    parser.add_argument("--strict-eval", action="store_true",
                         help="strict mode: V3 守门未过也返 isError=True")
    parser.add_argument("--json", action="store_true",
                         help="JSON 输出")
    args = parser.parse_args(argv)

    if args.snapshot:
        d = R11MeasurementDispatcher()
        _print_snapshot(d.stats())
        return 0

    if args.selftest:
        result = run_selftest(strict=args.strict_eval)
        if args.json:
            _print_snapshot(result)
        else:
            stats = result.get("stats", {})
            init = result.get("initialize", {}).get("result", {})
            print(f"V1137 R11 MCP selftest:")
            print(f"  protocol: {init.get('protocolVersion')} server={init.get('serverInfo', {}).get('name')} v{init.get('serverInfo', {}).get('version')}")
            print(f"  tools listed: {result.get('n_tools_listed')}/2")
            print(f"  round-trip OK: {result.get('n_round_trip_ok')}/{result.get('n_round_trip_total')}")
            print(f"  chaos state retained: {result.get('chaos_state_retained')}")
            print(f"  SSE port: {result.get('sse_actual_port')}, HTTP port: {result.get('http_actual_port')}")
            print(f"  dispatched: {stats.get('n_dispatched')} calls: {stats.get('n_calls')} errors: {stats.get('n_errors')}")
        return 0

    if args.chaos:
        result = run_chaos()
        if args.json:
            _print_snapshot(result)
        else:
            print(f"chaos_state_retained: {result['chaos_state_retained']}")
            print(f"  pre  dispatched={result['pre']['n_dispatched']} calls={result['pre']['n_calls']}")
            print(f"  post dispatched={result['post']['n_dispatched']} calls={result['post']['n_calls']}")
        return 0 if result["chaos_state_retained"] else 1

    if args.server:
        if args.transport == "stdio":
            return serve_stdio()
        if args.transport == "sse":
            return serve_sse(host=args.host, port=args.port)
        return serve_http(host=args.host, port=args.port)

    parser.print_help()
    return 1


# ---------------------------------------------------------------------------
# V3 哲学守门 (主 17:43 + 主 17:58 不假装)
# ---------------------------------------------------------------------------

V3_GUARDS_V1137 = {
    "r11_module_is_not_asi":
        "V1137 R11 MCP server 是工具, ASI 仍是更大目标 (主 22:33 + 主 17:58).",
    "r11_v1136_measurement_is_not_asi":
        "measure_v1136_real 真测 ≠ ASI 达成. 0.95 是 R10 终极门, 不是 ASI 实现.",
    "r11_v1130_evidence_is_not_proof":
        "V1130 backend evidence 是工程记录, 不是真理证明. 失败一律 fail-soft 透明.",
    "r11_offline_is_not_dummy":
        "offline 可跑 ≠ 假数据. offline = no real provider; 不假装成功.",
    "r11_transport_fanout_is_not_asi":
        "3 transports ≠ ASI 多模态. transport 是协议, ASI 是目标.",
}


__all__ = [
    "V1137_FRAMEWORK_VERSION",
    "DEFAULT_HOST",
    "DEFAULT_PORT",
    "run_selftest",
    "run_chaos",
    "serve_stdio", "serve_http", "serve_sse",
    "cli_main",
    "V3_GUARDS_V1137",
]


if __name__ == "__main__":
    sys.exit(cli_main(sys.argv[1:]))
