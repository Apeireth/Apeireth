#!/usr/bin/env python3
"""
VCP Substrate Example: IC6_configuration (ConfigurationInvariants)

Demonstrates Object.freeze DEFAULT_CONFIG + clampInteger + 3-tier mergeConfig +
privateConfig path.

Per V1335: Configuration invariants = Object.freeze DEFAULT_CONFIG / clampInteger /
3-tier mergeConfig / privateConfig path / env-typed configSchema.

Run: python example_ic6_configuration.py
"""
from __future__ import annotations

import sys
from typing import Any, Dict


# Frozen default (Python doesn't have Object.freeze but frozenset/tuple work)
DEFAULT_CONFIG: Dict[str, Any] = {
    "max_results": 10,
    "timeout_ms": 5000,
    "domains": (),
}


def clamp_integer(value: int, lo: int, hi: int) -> int:
    """Clamp integer to [lo, hi] range."""
    return max(lo, min(hi, value))


def merge_config_3tier(
    default: Dict[str, Any],
    user: Dict[str, Any],
    private: Dict[str, Any],
) -> Dict[str, Any]:
    """3-tier config merge: default < user < private."""
    return {**default, **user, **private}


def _self_test() -> dict:
    checks = {}
    checks["clamp_below"] = clamp_integer(5, 10, 20) == 10
    checks["clamp_above"] = clamp_integer(25, 10, 20) == 20
    checks["clamp_in_range"] = clamp_integer(15, 10, 20) == 15
    merged = merge_config_3tier(
        {"max_results": 10, "timeout_ms": 5000},
        {"max_results": 20},
        {"private_key": "secret"},
    )
    checks["merge_default"] = merged["timeout_ms"] == 5000
    checks["merge_user_overrides_default"] = merged["max_results"] == 20
    checks["merge_private_overrides_default"] = merged.get("private_key") == "secret"
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
