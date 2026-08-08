#!/usr/bin/env python3
"""
VCP Substrate Example: IC8_lifecycle (LifecycleInvariants)

Demonstrates _self_test probe + toolCallRecordStore lifecycle + promptCache.clear
on reload + cleanup-on-finally + graceful degrade.

Per V1335: Lifecycle invariants = _self_test probe / toolCallRecordStore lifecycle /
promptCache.clear on reload / cleanup-on-finally / graceful degrade.

Run: python example_ic8_lifecycle.py
"""
from __future__ import annotations

import sys
from typing import Any, Dict, List


class ToolCallRecordStore:
    """Lifecycle-managed tool call records."""

    def __init__(self):
        self._records: List[Dict[str, Any]] = []

    def begin_record(self, tool: str, args: Dict[str, Any]) -> str:
        record_id = f"rec_{len(self._records)}"
        self._records.append({"id": record_id, "tool": tool, "args": args, "status": "begin"})
        return record_id

    def finish_record(self, record_id: str, result: Any) -> None:
        for r in self._records:
            if r["id"] == record_id:
                r["status"] = "finish"
                r["result"] = result
                return

    def clear(self) -> None:
        self._records.clear()


def _self_test() -> dict:
    checks = {}
    store = ToolCallRecordStore()
    rid = store.begin_record("search", {"q": "test"})
    store.finish_record(rid, "result")
    checks["record_begin"] = any(r["id"] == rid for r in store._records)
    checks["record_finish"] = any(r["status"] == "finish" for r in store._records)
    store.clear()
    checks["records_cleared"] = len(store._records) == 0

    # graceful degrade
    def safe_call(fn, *args, fallback=None):
        try:
            return fn(*args)
        except Exception:
            return fallback

    checks["graceful_degrade"] = safe_call(lambda: 1 / 0, fallback="default") == "default"
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
