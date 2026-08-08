#!/usr/bin/env python3
"""
VCP Substrate Example: IC3_schema (SchemaInvariants)

Demonstrates manifestVersion=1.0.0 + pluginType=synchronous|asynchronous +
protocol=stdio + configSchema typed.

Per V1335: Schema invariants = manifestVersion=1.0.0 / pluginType=synchronous|asynchronous /
protocol=stdio / configSchema typed / enum domain check.

Run: python example_ic3_schema.py
"""
from __future__ import annotations

import sys
from typing import Literal


PLUGIN_MANIFEST = {
    "manifestVersion": "1.0.0",
    "pluginType": "synchronous",  # or "asynchronous"
    "protocol": "stdio",  # JSON-RPC 2.0 over stdin/stdout
    "configSchema": {
        "max_results": int,
        "timeout_ms": int,
        "domains": list,
    },
}


def validate_manifest(manifest: dict) -> bool:
    """Validate mandatory manifest fields."""
    if manifest.get("manifestVersion") != "1.0.0":
        return False
    if manifest.get("pluginType") not in ("synchronous", "asynchronous"):
        return False
    if manifest.get("protocol") != "stdio":
        return False
    return True


def _self_test() -> dict:
    checks = {}
    checks["manifest_version_correct"] = PLUGIN_MANIFEST["manifestVersion"] == "1.0.0"
    checks["plugin_type_enum"] = PLUGIN_MANIFEST["pluginType"] in ("synchronous", "asynchronous")
    checks["protocol_stdio"] = PLUGIN_MANIFEST["protocol"] == "stdio"
    checks["manifest_validates"] = validate_manifest(PLUGIN_MANIFEST)
    checks["bad_manifest_rejected"] = not validate_manifest({"manifestVersion": "0.0.1"})
    return checks


if __name__ == "__main__":
    results = _self_test()
    for k, v in results.items():
        status = "OK" if v else "FAIL"
        print(f"  {k}: {status}")
    if not all(results.values()):
        sys.exit(1)
    print("ALL CHECKS PASS")
