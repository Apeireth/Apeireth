#!/usr/bin/env python3
"""
VCP Substrate Example: IC7_resource_bounds (ResourceBoundsInvariants)

Demonstrates max_results clamp + token budgets + timeout clamp + BATCH_MAX.

Per V1335: Resource bounds invariants = max_results clamp / token budgets /
timeout clamp / BATCH_MAX / DOMAINS_MAX / SAFE budgets.

Run: python example_ic7_resource_bounds.py
"""
from __future__ import annotations

import sys


MAX_RESULTS: int = 100
BATCH_MAX: int = 50
DOMAINS_MAX: int = 20


def clamp_max_results(n: int, max_n: int = MAX_RESULTS) -> int:
    """Clamp result count to [0, max_n]."""
    return max(0, min(max_n, n))


def truncate_to_token_budget(text: str, max_tokens: int) -> str:
    """Approximate token budget: 1 token ≈ 4 chars."""
    max_chars = max_tokens * 4
    if len(text) <= max_chars:
        return text
    return text[:max_chars] + "..."


def clamp_timeout_ms(timeout_ms: int, lo: int = 100, hi: int = 60000) -> int:
    """Clamp timeout to [lo, hi] ms."""
    return max(lo, min(hi, timeout_ms))


def _self_test() -> dict:
    checks = {}
    checks["clamp_max_negative"] = clamp_max_results(-5) == 0
    checks["clamp_max_overflow"] = clamp_max_results(150) == 100
    checks["clamp_max_in_range"] = clamp_max_results(50) == 50
    truncated = truncate_to_token_budget("hello world this is a test", 2)
    checks["truncate_budget"] = truncated.endswith("...")
    checks["truncate_within_budget"] = truncate_to_token_budget("short", 100) == "short"
    checks["clamp_timeout_below"] = clamp_timeout_ms(50) == 100
    checks["clamp_timeout_above"] = clamp_timeout_ms(120000) == 60000
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
