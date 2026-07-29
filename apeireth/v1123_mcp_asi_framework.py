"""Apeireth ASI V1123 — 真 MCP 集成框架 + ASI 北极星 MCP 服务接口.

R9-MCP-001 / R9 W4 (主 22:33 + 主 13:31 + 主 17:43 + 主 23:44 + 主 19:33 + 主 00:56).

本模块把 apeireth.mcp.* 5 个子模块统一为一个一行可跑的 framework:

  apeireth.mcp.protocol              JSON-RPC 2.0 / MCP 协议守门
  apeireth.mcp.transport             stdio (NDJSON) + HTTP (/rpc) 两种 transport
  apeireth.mcp.asi_nine_keys         ASI 9 键 LOCKED 真测注入
  apeireth.mcp.model_adapters        Claude / GPT / Ollama / local 跨模型适配
  apeireth.mcp.asi_north_star_server 5 大 MCP 工具 (V1123 锁定)
  apeireth.mcp.orchestrator          跨 server 编排 (MCP1 + MCP2 串接)

5 大 MCP 工具 (V1123 真实现):
  1. asi_north_star_query    V0.1 / V0.3 / V0.4 / North Star 公式查询
  2. v1074_guard             V1074 V0.3 守门 (≥ 0.8884)
  3. v1112_dgm_run           DGM v0.4 轻量真演化
  4. v1114_weekly_eval       V1114 / V1119 每周集成评估
  5. identity_lock_check     V1072 永恒身份 + ASI 9 键 LOCKED

Usage:
    # 1) CLI: stdio transport
    python -m apeireth.v1123_mcp_asi_framework --server --transport stdio

    # 2) CLI: HTTP transport
    python -m apeireth.v1123_mcp_asi_framework --server --transport http --port 8118

    # 3) Self-test (主 00:56 任何人都能接手: 1 行真跑)
    python -m apeireth.v1123_mcp_asi_framework --selftest

    # 4) Cross-server handoff
    python -m apeireth.v1123_mcp_asi_framework --handoff --week W4

    # 5) JSON snapshot
    python -m apeireth.v1123_mcp_asi_framework --snapshot > reports/r9-mcp-v1123-snapshot.json
"""
from __future__ import annotations

import argparse
import json
import os
import socket
import sys
import time
from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict, List, Optional

# 同包内 import (主 19:33 走在前人经验上: 复用 V1114 / V1119 / V1072 / V1074)
from apeireth.mcp import ASI_NINE_KEYS, ASI_NORTH_STAR_VERSION, PROTOCOL_VERSION
from apeireth.mcp.protocol import (
    JSONRPC_INVALID_PARAMS, JSONRPC_METHOD_NOT_FOUND,
    check_protocol_version, make_error_response, make_result_response,
    parse_request, validate_arguments, validate_tool_result,
    SUPPORTED_PROTOCOL_VERSIONS,
)
from apeireth.mcp.transport import StdioTransport, HttpTransport
from apeireth.mcp.asi_nine_keys import (
    AsiNineKeyLock, ASI_NINE_KEYS as _ASI_NINE_KEYS, ASI_NINE_KEYS_DOCS,
    inject_guard_block, verify_or_raise,
)
from apeireth.mcp.model_adapters import (
    ModelAdapterRegistry, heuristic_asi_score, ASI_KEYWORDS,
    LocalHeuristicAdapter, OllamaHttpAdapter, OpenAIHttpAdapter, ClaudeHttpAdapter,
)
from apeireth.mcp.asi_north_star_server import (
    AsiNorthStarDispatcher, SERVER_NAME, TOOL_REGISTRY,
    V1123_VERSION, ASI_FORMULAS, ASI_RESOURCES, ASI_PROMPTS,
)
from apeireth.mcp.orchestrator import (
    CrossServerOrchestrator, CrossServerReport, CrossServerStep,
)


V1123_FRAMEWORK_VERSION = "0.1.0"


# ---------------------------------------------------------------------------
# 公开 facade
# ---------------------------------------------------------------------------


def build_default_dispatcher() -> AsiNorthStarDispatcher:
    """构建默认 dispatcher (主 00:56 任何人都能接手: 一行)."""
    return AsiNorthStarDispatcher()


def serve_stdio(dispatcher: Optional[AsiNorthStarDispatcher] = None) -> int:
    """stdio transport 启动 (主 17:43 实事求是: 真阻塞)."""
    d = dispatcher or build_default_dispatcher()
    t = StdioTransport(d.handle_message)
    return t.serve()


def serve_http(dispatcher: Optional[AsiNorthStarDispatcher] = None,
               host: str = "127.0.0.1", port: int = 0) -> int:
    """HTTP transport 启动 (主 17:43 实事求是: 真 serve_forever)."""
    d = dispatcher or build_default_dispatcher()
    http = HttpTransport(
        dispatch=d.handle_message,
        server_info={"name": SERVER_NAME, "version": V1123_VERSION, "tools": list(TOOL_REGISTRY.keys())},
        host=host, port=port,
    )
    http.start()
    print(f"[V1123] {SERVER_NAME} v{V1123_VERSION} listening on {http.url()}", flush=True)
    print(f"[V1123] health: http://{host}:{http.actual_port}/health", flush=True)
    try:
        while True:
            time.sleep(3600)
    except KeyboardInterrupt:
        pass
    finally:
        http.stop()
    return 0


def run_selftest() -> Dict[str, Any]:
    """V1123 自检: 5 工具 + 跨 server 编排 + 守门 (主 17:43 实事求是)."""
    d = build_default_dispatcher()

    results: Dict[str, Any] = {}

    # 1) initialize
    init = d.handle_message({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                              "params": {"protocolVersion": "2024-11-05"}})
    results["initialize"] = init

    # 2) tools/list
    lst = d.handle_message({"jsonrpc": "2.0", "id": 2, "method": "tools/list",
                             "params": {}})
    results["tools_list"] = lst

    # 3) 5 大工具 round-trip
    calls = {
        "asi_north_star_query": {"formula": "v0.4", "explain": True},
        "v1074_guard": {"score": 0.8897, "min_floor": 0.8884, "include_decision": True},
        "v1112_dgm_run": {"n_generations": 3, "seed": 42},
        "v1114_weekly_eval": {"week_label": "W4", "v03_history": [0.8884, 0.8890]},
        "identity_lock_check": {"run": False},
    }
    results["tool_calls"] = {}
    for i, (name, args) in enumerate(calls.items(), start=10):
        resp = d.handle_message({"jsonrpc": "2.0", "id": i, "method": "tools/call",
                                  "params": {"name": name, "arguments": args}})
        results["tool_calls"][name] = resp

    # 4) 错误路径
    bad = d.handle_message({"jsonrpc": "2.0", "id": 99, "method": "tools/call",
                             "params": {"name": "v1074_guard", "arguments": {}}})
    results["bad_params"] = bad
    missing = d.handle_message({"jsonrpc": "2.0", "id": 98, "method": "unknown_method",
                                 "params": {}})
    results["bad_method"] = missing

    # 5) 跨 server 编排
    orch = CrossServerOrchestrator(mcp1=d)
    handoff = orch.run_weekly_handoff(week_label="W4", v04_score=0.8538, v03_score=0.8897)
    results["cross_server_handoff"] = handoff.to_dict()

    # 6) stats
    results["stats"] = d.stats()

    return results


# ---------------------------------------------------------------------------
# CLI (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


def _print_snapshot(payload: Dict[str, Any]) -> None:
    print(json.dumps(payload, indent=2, ensure_ascii=False, default=str))


def cli_main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1123_mcp_asi_framework",
        description="V1123 真 MCP 集成框架 + ASI 北极星 MCP 服务接口",
    )
    parser.add_argument("--server", action="store_true",
                        help="启动 MCP server (stdio / http)")
    parser.add_argument("--transport", choices=("stdio", "http"), default="http",
                        help="transport 类型 (默认 http)")
    parser.add_argument("--host", default="127.0.0.1", help="HTTP 监听 host (主 23:44 不暴露 0.0.0.0)")
    parser.add_argument("--port", type=int, default=8118, help="HTTP 监听 port")
    parser.add_argument("--selftest", action="store_true", help="V1123 1 行自检")
    parser.add_argument("--snapshot", action="store_true", help="打印 dispatcher 状态 JSON")
    parser.add_argument("--handoff", action="store_true", help="跑跨 server weekly handoff")
    parser.add_argument("--week", default="W4", help="handoff 用的 week label (默认 W4)")
    parser.add_argument("--v04", type=float, default=0.8538, help="handoff 用的 V0.4 分数")
    parser.add_argument("--v03", type=float, default=0.8897, help="handoff 用的 V0.3 分数")
    parser.add_argument("--json", action="store_true", help="JSON 输出")
    args = parser.parse_args(argv)

    if args.snapshot:
        d = build_default_dispatcher()
        _print_snapshot(d.stats())
        return 0

    if args.selftest:
        result = run_selftest()
        if args.json:
            _print_snapshot(result)
        else:
            n_tools = len(result.get("tool_calls", {}))
            n_ok = sum(1 for r in result.get("tool_calls", {}).values()
                        if r and "result" in r and not r["result"].get("isError"))
            print(f"V1123 selftest: {n_ok}/{n_tools} tools OK")
            cs = result.get("cross_server_handoff", {})
            print(f"V1123 cross-server handoff: {cs.get('n_ok', 0)}/{cs.get('n_steps', 0)} steps OK, "
                  f"all_ok={cs.get('all_ok', False)}")
            stats = result.get("stats", {})
            print(f"V1123 stats: n_calls={stats.get('n_calls', 0)}, "
                  f"n_tools={stats.get('n_tools', 0)}, n_models={len(stats.get('models', []))}")
            print(f"V1123 9 键 LOCKED: {stats.get('nine_key_lock', {}).get('asi_nine_keys_locked', '?')}")
        return 0

    if args.handoff:
        orch = CrossServerOrchestrator()
        report = orch.run_weekly_handoff(week_label=args.week, v04_score=args.v04, v03_score=args.v03)
        if args.json:
            _print_snapshot(report.to_dict())
        else:
            print(f"V1123 cross-server handoff ({args.week}):")
            print(f"  steps: {report.n_ok}/{report.n_steps} OK, all_ok={report.all_ok}")
            print(f"  final: {report.final}")
            print(f"  elapsed: {report.elapsed_ms_total:.1f} ms")
        return 0 if report.all_ok else 1

    if args.server:
        if args.transport == "stdio":
            return serve_stdio()
        return serve_http(host=args.host, port=args.port)

    parser.print_help()
    return 1


__all__ = [
    "V1123_FRAMEWORK_VERSION",
    "build_default_dispatcher",
    "serve_stdio", "serve_http",
    "run_selftest",
    "cli_main",
]


# ---------------------------------------------------------------------------
# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
# ---------------------------------------------------------------------------
V3_GUARDS = {
    "module_is_not_asi": "模块是工具, ASI 是更大目标. V1123 MCP 框架是工具, 不是 ASI.",
    "mcp_skeleton_is_not_production": "MCP 协议骨架 ≠ ASI 北极星达成. V1123 是 ASI 的接入层, 不是 ASI 本身.",
    "cross_server_orchestration_is_not_asi": "MCP1 + MCP2 串接 ≠ 自主意识. 编排是工程, ASI 是目标.",
    "model_adapter_call_is_not_reasoning": "跨模型调用 ≠ 真正推理. V1123 至少 2 种真跑, 但 ≠ ASI.",
    "nine_key_lock_is_not_truth": "ASI 9 键 LOCKED ≠ ASI 真的实现. 9 键是守门, 不是证明.",
}


if __name__ == "__main__":
    raise SystemExit(cli_main(sys.argv[1:]))
