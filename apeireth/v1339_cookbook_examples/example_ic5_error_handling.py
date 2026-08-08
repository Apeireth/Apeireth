#!/usr/bin/env python3
"""
VCP Substrate Example: IC5_error_handling (ErrorHandlingInvariants)

Demonstrates {success:false, error} envelope + structured error messages +
helpful available-* lists.

Per V1335: Error handling invariants = {success:false, error} envelope /
structured error messages / helpful available-* lists.

Run: python example_ic5_error_handling.py
"""
from __future__ import annotations

import sys
from typing import Any, Dict, List


def format_error(message: str, available: List[str] = None) -> Dict[str, Any]:
    """{success:false, error} envelope with optional available list."""
    out = {"success": False, "error": message}
    if available:
        out["available"] = available
    return out


def format_success(data: Any) -> Dict[str, Any]:
    """{success:true, data} envelope."""
    return {"success": True, "data": data}


def _self_test() -> dict:
    checks = {}
    err = format_error("chain not found", available=["chain1", "chain2"])
    checks["error_envelope"] = err["success"] is False
    checks["error_message"] = err["error"] == "chain not found"
    checks["available_list"] = err.get("available") == ["chain1", "chain2"]
    ok = format_success({"result": 42})
    checks["success_envelope"] = ok["success"] is True
    checks["success_data"] = ok["data"] == {"result": 42}
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
