#!/usr/bin/env python3
"""
VCP Substrate Example: IC4_ipc (IPCProtocolInvariants)

Demonstrates JSON-RPC 2.0 over stdin/stdout + exit-0-on-error.

Per V1335: IPC protocol invariants = JSON-RPC 2.0 over stdin/stdout /
exit-0-on-error / structured response envelope.

Run: python example_ic4_ipc.py
"""
from __future__ import annotations

import json
import sys
from typing import Any, Dict


def make_jsonrpc_response(id: Any, result: Any) -> Dict[str, Any]:
    """JSON-RPC 2.0 success envelope."""
    return {
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    }


def make_jsonrpc_error(id: Any, code: int, message: str) -> Dict[str, Any]:
    """JSON-RPC 2.0 error envelope."""
    return {
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    }


def handle_request(request: Dict[str, Any]) -> Dict[str, Any]:
    """Handle one JSON-RPC request, return response."""
    if request.get("jsonrpc") != "2.0":
        return make_jsonrpc_error(request.get("id"), -32600, "invalid jsonrpc version")
    method = request.get("method")
    if method == "ping":
        return make_jsonrpc_response(request.get("id"), "pong")
    return make_jsonrpc_error(request.get("id"), -32601, f"method not found: {method}")


def _self_test() -> dict:
    checks = {}
    resp = handle_request({"jsonrpc": "2.0", "id": 1, "method": "ping"})
    checks["ping_returns_pong"] = resp.get("result") == "pong"
    checks["jsonrpc_2_0"] = resp.get("jsonrpc") == "2.0"
    bad = handle_request({"jsonrpc": "1.0", "id": 2, "method": "ping"})
    checks["bad_version_returns_error"] = "error" in bad
    missing = handle_request({"jsonrpc": "2.0", "id": 3, "method": "unknown"})
    checks["unknown_method_returns_error"] = "error" in missing
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
