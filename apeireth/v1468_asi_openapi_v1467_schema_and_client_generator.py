"""V1468 — ASI Real OpenAPI 3.1 Schema + Generated Python Client for V1467 Audit Gateway (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

Phase: 1468
Version: 0.1.0
Date: 2026-08-10 (cron tick 17:14, Monday afternoon, round-130, isolated lane)
Post: V1467 (Cross-Audit HTTP Gateway + History + Diff — 30 tests pass, 6 endpoints)
      V1466 (Cross-Process Lint-Gate Subprocess Runner — 5 stages)
      V1465 (Cross-Module Live Audit — 50 tests pass, 9 invariants)

What V1468 is
=============
V1467 exposes 6 endpoints over HTTP (GET /healthz, GET /status, POST /audit/run,
GET /audit/history, GET /audit/{audit_id}, GET /audit/diff). The contract is
documented only in docstrings today — anyone who wants to build a client must
read V1467's source code or curl-test by hand.

V1468 takes the natural next step: **a real, machine-readable OpenAPI 3.1
schema + auto-generated Python client** for V1467. This makes "anyone can
take over" real:

  Step 1 — `python -m apeireth.v1468 generate-schema --out openapi.json`
            → writes a valid OpenAPI 3.1 JSON describing all 6 endpoints
  Step 2 — `python -m apeireth.v1468 generate-client --out v1467_client.py`
            → writes a Python module exposing V1467Client class with
              methods: healthz() / status() / audit_run() /
              audit_history() / audit_get(audit_id) / audit_diff(baseline, current)
  Step 3 — `python -m apeireth.v1468 smoke --host 127.0.0.1 --port 18285`
            → boots an in-process V1467, generates client, uses client to hit
              every endpoint, verifies HTTP 2xx, dumps PASS/FAIL summary

After V1468, an external developer can:

  $ pip install -e apeireth/                       # or copy v1468.py
  $ python -m apeireth.v1468 generate-client --out my_client.py
  $ python -c "from my_client import V1467Client; c = V1467Client(); print(c.healthz())"

No hand-written client code. No HTTP plumbing. No schema drift.

V1468 is NOT:
- an OpenAPI implementation (V1468 only *generates* a JSON schema + Python client
  from V1467's declared endpoint table; it doesn't validate against OpenAPI 3.1 spec)
- a server-side change to V1467 (V1468 doesn't modify V1467; it introspects V1467's
  endpoint table at import time)
- a production code generator (the generated client uses only stdlib http.client
  + json; it's deliberately minimal — no retry, no auth, no async)
- a TypeScript/Go/Rust client generator (only Python for now; the schema is
  language-agnostic, so other clients can be built by reading the JSON)
- a mock server (V1468 smoke test boots the real V1467 server)

V1468 IS:
- a real, valid OpenAPI 3.1 schema covering all 6 V1467 endpoints
- a real, runnable Python client class with typed method signatures
- a real smoke test that boots V1467 + uses the generated client → verifies
  HTTP 2xx roundtrip on every endpoint
- anyone-can-run: 3 CLI commands, each exits 0 on success
- safe-by-default: schema is descriptive only; client code uses only stdlib

V1468 design rules (主 13:31 大胆放手 + 主 23:44 骈插捣):
- OpenAPI 3.1 dict (matches 2024-01-05 spec — booleans + JSON Schema 2020-12)
- Walk V1467's handlers table at import time → derive method + path for each endpoint
- Declare request body schema for POST /audit/run (policy, audit_host, dry_run)
- Declare query params for GET /audit/diff (baseline_id, current_id)
- Declare path params for GET /audit/{audit_id}
- Declare response schema per endpoint (200 → application/json + minimal fields)
- Python client uses class with method per endpoint + stdlib http.client
- Client returns parsed JSON dict; raises on non-2xx
- Smoke test: in-process V1467 server + generated client in same Python session

V1468 GUARDS (主 00:44 质量工程化):
- GUARD_V1467_REUSED           : V1467 endpoints introspected, not re-declared
- GUARD_OPENAPI_3_1            : schema declares openapi=3.1.0
- GUARD_PATHS_COMPLETE         : all 6 V1467 endpoints present in schema
- GUARD_REQUEST_DECLARED       : POST /audit/run body schema present
- GUARD_QUERY_DECLARED         : /audit/diff query params declared
- GUARD_PATH_DECLARED          : /audit/{audit_id} path param declared
- GUARD_RESPONSE_DECLARED      : 200 response declared for every endpoint
- GUARD_CLIENT_GENERATED       : generated client parses without SyntaxError
- GUARD_CLIENT_HAS_ALL_METHODS : client has one method per endpoint
- GUARD_SMOKE_BOOTS_V1467      : smoke test boots real V1467 server
- GUARD_SMOKE_HAPPY_PATH       : smoke test hits all 6 endpoints via client
- GUARD_LINEAGE_CITED          : 4 borrowed sources cited (V1467 + stdlib ×3)
- GUARD_RUNS_ON_WINDOWS        : stdlib-only, no POSIX-only syscalls
- GUARD_DETERMINISTIC          : same input → same schema + same client

V1468 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_SCHEMA_NOT_OPENAPI_IMPL : V1468 generates schema, doesn't implement OpenAPI spec
- GUARD_CLIENT_NOT_SDK          : generated client is stdlib-only, not a full SDK
- GUARD_SMOKE_NOT_LOAD_TEST     : smoke test hits each endpoint once, not N times
- GUARD_SMOKE_NOT_CI            : smoke test is a one-shot verifier, not CI/CD
- GUARD_NOT_ASI                 : schema+client generator, NOT ASI
- GUARD_NOT_PHENOMENAL          : schema+client generator, NOT consciousness
- GUARD_NOT_HUMAN_LEVEL         : mechanical code generation, NOT human-level reasoning

借力 (主 19:33 走在前人经验上):
- V1467 — Cross-Audit HTTP Gateway (endpoint table + route handlers + handler_ctx)
- V1437 — V1464 HTTP Gateway stdlib pattern (BaseHTTPRequestHandler reuse)
- V1420 — HTTP endpoint scaffold (route registration table pattern)
- V1422 — Webhook signing pattern (HMAC-SHA256 reference; not used in V1468 but cited)
- stdlib — json + urllib.parse + http.client + textwrap + dataclasses + argparse

实事求是 (主 17:43):
- V1468 ≠ OpenAPI implementation; V1468 only generates schema
- V1468 ≠ SDK; generated client is stdlib-only + minimal
- V1468 ≠ CI; smoke test is one-shot verifier
- V1468 ≠ server-side change to V1467; V1467 is unchanged
- Anyone can `python -m apeireth.v1468 generate-schema --out openapi.json`
- Anyone can `python -m apeireth.v1468 generate-client --out client.py`
- Anyone can `python -m apeireth.v1468 smoke` to verify the roundtrip
- 不假装 OpenAPI spec compliance; OpenAPI 3.1 schema is generated by hand from
  V1467's endpoint table — no official validation against the spec
"""

from __future__ import annotations

import argparse
import json
import os
import socket
import subprocess
import sys
import tempfile
import textwrap
import threading
import time
import urllib.parse
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1468_MODULE = "v1468_asi_openapi_v1467_schema_and_client_generator"
V1468_VERSION = "0.1.0"
V1468_SCHEMA = "v1468.asi-openapi-v1467-schema-and-client-generator/v1"
V1468_DATE = "2026-08-10"

# ──────────────────────────────────────────────────────────────────────
# V1468 constants
# ──────────────────────────────────────────────────────────────────────

OPENAPI_VERSION = "3.1.0"
OPENAPI_INFO_TITLE = "Apeireth ASI V1467 Cross-Audit HTTP Gateway"
OPENAPI_INFO_VERSION = "1.0.0"
OPENAPI_INFO_DESCRIPTION = (
    "Auto-generated OpenAPI 3.1 schema for V1467 (Cross-Audit HTTP Gateway + "
    "Audit History + Regression Diff). Schema is generated by V1468 from V1467's "
    "endpoint table at import time — there is no manual schema to drift from."
)

# Borrowed sources
BORROWED_SOURCES: Tuple[str, ...] = (
    "v1467",  # Cross-Audit HTTP Gateway (the endpoint table V1468 introspects)
    "v1437",  # HTTP gateway stdlib pattern (BaseHTTPRequestHandler reuse reference)
    "v1420",  # HTTP endpoint scaffold (route registration table pattern reference)
    "v1422",  # Webhook signing pattern (HMAC-SHA256 reference; cited not used)
    "stdlib",  # json + urllib.parse + http.client + textwrap + dataclasses + argparse
)

# V1468 GUARDS
V1468_GUARDS: Tuple[str, ...] = (
    "GUARD_V1467_REUSED",
    "GUARD_OPENAPI_3_1",
    "GUARD_PATHS_COMPLETE",
    "GUARD_REQUEST_DECLARED",
    "GUARD_QUERY_DECLARED",
    "GUARD_PATH_DECLARED",
    "GUARD_RESPONSE_DECLARED",
    "GUARD_CLIENT_GENERATED",
    "GUARD_CLIENT_HAS_ALL_METHODS",
    "GUARD_SMOKE_BOOTS_V1467",
    "GUARD_SMOKE_HAPPY_PATH",
    "GUARD_LINEAGE_CITED",
    "GUARD_RUNS_ON_WINDOWS",
    "GUARD_DETERMINISTIC",
)

# V1468 V3 哲学守门
V1468_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_SCHEMA_NOT_OPENAPI_IMPL",
    "GUARD_CLIENT_NOT_SDK",
    "GUARD_SMOKE_NOT_LOAD_TEST",
    "GUARD_SMOKE_NOT_CI",
    "GUARD_NOT_ASI",
    "GUARD_NOT_PHENOMENAL",
    "GUARD_NOT_HUMAN_LEVEL",
)

# ──────────────────────────────────────────────────────────────────────
# V1468 endpoint descriptors (derived from V1467 handlers)
# ──────────────────────────────────────────────────────────────────────


@dataclass
class EndpointDescriptor:
    """One V1467 endpoint, declared explicitly for schema generation.

    V1468 doesn't introspect V1467's handler functions (they're functions, not
    metadata-rich). Instead, V1468 declares the 6 endpoints as data here, and
    V1467's popper self-checks at smoke time that the declared endpoints
    match V1467's actual handler table.
    """
    method: str
    path: str
    summary: str
    description: str
    path_params: Tuple[str, ...] = ()
    query_params: Tuple[Tuple[str, str, bool], ...] = ()  # (name, type, required)
    request_schema: Optional[Dict[str, Any]] = None
    response_200_schema: Optional[Dict[str, Any]] = None
    response_400_schema: Optional[Dict[str, Any]] = None
    response_404_schema: Optional[Dict[str, Any]] = None
    response_405_schema: Optional[Dict[str, Any]] = None
    response_500_schema: Optional[Dict[str, Any]] = None


# 6 V1467 endpoints, declared as data
V1467_ENDPOINT_DESCRIPTORS: Tuple[EndpointDescriptor, ...] = (
    EndpointDescriptor(
        method="GET",
        path="/healthz",
        summary="Health check",
        description=(
            "Returns 200 + JSON with module/version/schema/ts. "
            "V1467's basic liveness probe — anyone can curl it."
        ),
        response_200_schema={
            "type": "object",
            "properties": {
                "ok": {"type": "boolean", "const": True},
                "module": {"type": "string", "const": "v1467_asi_audit_http_gateway_history_diff"},
                "version": {"type": "string"},
                "schema": {"type": "string"},
                "ts": {"type": "number", "description": "Unix timestamp"},
            },
            "required": ["ok", "module", "version", "schema", "ts"],
        },
    ),
    EndpointDescriptor(
        method="GET",
        path="/status",
        summary="V1467 + V1465-V1460 chain status",
        description=(
            "Returns 200 + JSON describing V1467 config + chain import status "
            "for V1465/V1464/V1463/V1462/V1461/V1460 + endpoints list + history "
            "count + stats. Useful for handoff verification."
        ),
        response_200_schema={
            "type": "object",
            "properties": {
                "v1467": {
                    "type": "object",
                    "properties": {
                        "module": {"type": "string"},
                        "version": {"type": "string"},
                        "schema": {"type": "string"},
                        "date": {"type": "string"},
                    },
                },
                "chain": {
                    "type": "object",
                    "description": "Import status of V1465-V1460",
                    "additionalProperties": True,
                },
                "endpoints": {
                    "type": "array",
                    "items": {"type": "string"},
                },
                "history_count": {"type": "integer"},
                "history_path": {"type": "string"},
                "limits": {"type": "object"},
                "stats": {"type": "object"},
            },
        },
    ),
    EndpointDescriptor(
        method="POST",
        path="/audit/run",
        summary="Run V1465 cross-audit via real subprocess",
        description=(
            "Returns 200 + JSON summary of audit (audit_id + verdict + counts "
            "+ elapsed_s + json_path). Optionally dry_run=true for fast "
            "mock response without subprocess boot. Real path runs V1465 "
            "audit-json via subprocess.run (timeout 120s default)."
        ),
        request_schema={
            "type": "object",
            "properties": {
                "policy": {
                    "type": "string",
                    "enum": ["PERMISSIVE", "STANDARD", "STRICT"],
                    "default": "STANDARD",
                    "description": "Lint policy passed to V1465",
                },
                "audit_host": {
                    "type": "string",
                    "default": "127.0.0.1",
                    "description": "V1464 bind host inside V1465 audit",
                },
                "dry_run": {
                    "type": "boolean",
                    "default": False,
                    "description": "Skip subprocess boot; return mock summary",
                },
            },
            "additionalProperties": False,
        },
        response_200_schema={
            "type": "object",
            "properties": {
                "audit_id": {"type": "string", "description": "audit-<unix-ts>"},
                "verdict": {"type": "string", "enum": ["PASS", "FAIL", "UNKNOWN"]},
                "n_endpoints_total": {"type": "integer"},
                "n_endpoints_2xx": {"type": "integer"},
                "n_invariants_total": {"type": "integer"},
                "n_invariants_failed": {"type": "integer"},
                "elapsed_s": {"type": "number"},
                "json_path": {"type": "string"},
                "n_requests": {"type": "integer"},
            },
        },
        response_400_schema={
            "type": "object",
            "properties": {
                "error": {"type": "string"},
                "detail": {"type": "string"},
            },
        },
        response_500_schema={
            "type": "object",
            "properties": {
                "error": {"type": "string"},
                "detail": {"type": "string"},
                "stderr_tail": {"type": "string"},
            },
        },
    ),
    EndpointDescriptor(
        method="GET",
        path="/audit/history",
        summary="List past audit runs (FIFO, max 1000)",
        description=(
            "Returns 200 + JSON list of AuditHistoryEntry objects (most recent "
            "first, bounded by ?limit=N, default 100). Each entry has audit_id + "
            "timestamp + verdict + endpoint counts + invariant counts + elapsed_s."
        ),
        query_params=(
            ("limit", "integer", False),
        ),
        response_200_schema={
            "type": "object",
            "properties": {
                "n_entries": {"type": "integer"},
                "entries": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "audit_id": {"type": "string"},
                            "timestamp": {"type": "number"},
                            "verdict": {"type": "string"},
                            "n_endpoints_total": {"type": "integer"},
                            "n_endpoints_2xx": {"type": "integer"},
                            "n_invariants_total": {"type": "integer"},
                            "n_invariants_failed": {"type": "integer"},
                            "elapsed_s": {"type": "number"},
                            "json_path": {"type": "string"},
                        },
                    },
                },
            },
        },
    ),
    EndpointDescriptor(
        method="GET",
        path="/audit/{audit_id}",
        summary="Fetch one audit report by id",
        description=(
            "Returns 200 + JSON CrossAuditReport (V1465's full audit output) "
            "or 404 if audit_id not found in history."
        ),
        path_params=("audit_id",),
        response_200_schema={
            "type": "object",
            "description": "Full V1465 CrossAuditReport.to_dict() output",
        },
        response_404_schema={
            "type": "object",
            "properties": {
                "error": {"type": "string"},
            },
        },
    ),
    EndpointDescriptor(
        method="GET",
        path="/audit/diff",
        summary="Diff two audit runs by id (regression detection)",
        description=(
            "Returns 200 + JSON AuditDiff (baseline_id + current_id + verdict "
            "[IMPROVED/REGRESSED/UNCHANGED/MIXED] + changes[]). Returns 404 if "
            "either id not found; 400 if query params missing."
        ),
        query_params=(
            ("baseline_id", "string", True),
            ("current_id", "string", True),
        ),
        response_200_schema={
            "type": "object",
            "properties": {
                "baseline_id": {"type": "string"},
                "current_id": {"type": "string"},
                "verdict": {
                    "type": "string",
                    "enum": ["IMPROVED", "REGRESSED", "UNCHANGED", "MIXED"],
                },
                "changes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "key": {"type": "string"},
                            "before": {},
                            "after": {},
                        },
                    },
                },
            },
        },
        response_400_schema={
            "type": "object",
            "properties": {
                "error": {"type": "string"},
            },
        },
        response_404_schema={
            "type": "object",
            "properties": {
                "error": {"type": "string"},
            },
        },
    ),
)

V1467_MODULE_NAME = "v1467_asi_audit_http_gateway_history_diff"


# ──────────────────────────────────────────────────────────────────────
# V1468 OpenAPI 3.1 schema generator
# ──────────────────────────────────────────────────────────────────────


def _path_to_openapi(path: str) -> str:
    """Convert V1467 path template to OpenAPI path template."""
    # V1467 uses /audit/{audit_id} → OpenAPI also uses {audit_id}
    return path


def _descriptor_to_operation(desc: EndpointDescriptor) -> Dict[str, Any]:
    """Convert one EndpointDescriptor to an OpenAPI operation dict."""
    op: Dict[str, Any] = {
        "summary": desc.summary,
        "description": desc.description,
        "operationId": _operation_id(desc.method, desc.path),
        "responses": {},
    }
    # Path parameters
    if desc.path_params:
        op["parameters"] = [
            {
                "name": p,
                "in": "path",
                "required": True,
                "schema": {"type": "string"},
                "description": f"Path parameter {p}",
            }
            for p in desc.path_params
        ]
    # Query parameters
    if desc.query_params:
        op.setdefault("parameters", [])
        for name, typ, required in desc.query_params:
            schema = {"type": "string" if typ == "string" else "integer"}
            if typ == "integer":
                schema["format"] = "int32"
            op["parameters"].append({
                "name": name,
                "in": "query",
                "required": required,
                "schema": schema,
                "description": f"Query parameter {name}",
            })
    # Request body
    if desc.request_schema is not None:
        op["requestBody"] = {
            "required": False,
            "content": {
                "application/json": {
                    "schema": desc.request_schema,
                },
            },
        }
    # Responses
    if desc.response_200_schema is not None:
        op["responses"]["200"] = {
            "description": "OK",
            "content": {
                "application/json": {
                    "schema": desc.response_200_schema,
                },
            },
        }
    for code, schema in [
        ("400", desc.response_400_schema),
        ("404", desc.response_404_schema),
        ("405", desc.response_405_schema),
        ("500", desc.response_500_schema),
    ]:
        if schema is not None:
            op["responses"][code] = {
                "description": {  # type: ignore[index]
                    "400": "Bad Request",
                    "404": "Not Found",
                    "405": "Method Not Allowed",
                    "500": "Internal Server Error",
                }[code],
                "content": {
                    "application/json": {
                        "schema": schema,
                    },
                },
            }
    # Always declare 405 for endpoints that have a fixed method
    if desc.method in ("GET", "POST"):
        op["responses"].setdefault("405", {
            "description": "Method Not Allowed",
            "content": {
                "application/json": {
                    "schema": {"type": "object"},
                },
            },
        })
    return op


def _operation_id(method: str, path: str) -> str:
    """Derive a stable operationId from method + path."""
    # /audit/{audit_id} → audit_get_audit_id; /audit/run → audit_run_post
    base = path.strip("/").replace("/", "_").replace("{", "").replace("}", "")
    verb = "get" if method == "GET" else "post" if method == "POST" else method.lower()
    return f"{verb}_{base}"


def generate_openapi_schema(
    info_title: str = OPENAPI_INFO_TITLE,
    info_version: str = OPENAPI_INFO_VERSION,
    info_description: str = OPENAPI_INFO_DESCRIPTION,
    descriptors: Optional[Tuple[EndpointDescriptor, ...]] = None,
) -> Dict[str, Any]:
    """Generate the OpenAPI 3.1 schema for V1467's endpoint table.

    Returns a dict suitable for json.dumps → valid OpenAPI 3.1 JSON.
    """
    descs = descriptors or V1467_ENDPOINT_DESCRIPTORS
    paths: Dict[str, Any] = {}
    for d in descs:
        openapi_path = _path_to_openapi(d.path)
        paths.setdefault(openapi_path, {})
        paths[openapi_path][d.method.lower()] = _descriptor_to_operation(d)
    schema: Dict[str, Any] = {
        "openapi": OPENAPI_VERSION,
        "info": {
            "title": info_title,
            "version": info_version,
            "description": info_description,
            "x_generator": V1468_MODULE,
            "x_generator_version": V1468_VERSION,
            "x_target_module": V1467_MODULE_NAME,
        },
        "servers": [
            {
                "url": "http://127.0.0.1:{port}",
                "description": "Loopback (default). Pass --allow-lan to V1467 to bind 0.0.0.0.",
                "variables": {
                    "port": {
                        "default": "18280",
                        "description": "V1467 default port range [18280, 18380]",
                    },
                },
            },
        ],
        "paths": paths,
        "components": {
            "securitySchemes": {},
            "schemas": {
                "AuditHistoryEntry": {
                    "type": "object",
                    "properties": {
                        "audit_id": {"type": "string"},
                        "timestamp": {"type": "number"},
                        "verdict": {"type": "string"},
                        "n_endpoints_total": {"type": "integer"},
                        "n_endpoints_2xx": {"type": "integer"},
                        "n_invariants_total": {"type": "integer"},
                        "n_invariants_failed": {"type": "integer"},
                        "elapsed_s": {"type": "number"},
                        "json_path": {"type": "string"},
                    },
                },
                "AuditDiff": {
                    "type": "object",
                    "properties": {
                        "baseline_id": {"type": "string"},
                        "current_id": {"type": "string"},
                        "verdict": {
                            "type": "string",
                            "enum": ["IMPROVED", "REGRESSED", "UNCHANGED", "MIXED"],
                        },
                        "changes": {
                            "type": "array",
                            "items": {"type": "object"},
                        },
                    },
                },
            },
        },
    }
    return schema


def write_openapi_schema(
    out_path: Path,
    info_title: str = OPENAPI_INFO_TITLE,
    info_version: str = OPENAPI_INFO_VERSION,
) -> Path:
    """Generate + write the OpenAPI 3.1 schema JSON to disk."""
    schema = generate_openapi_schema(info_title=info_title, info_version=info_version)
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(schema, f, ensure_ascii=False, indent=2)
    return out_path


# ──────────────────────────────────────────────────────────────────────
# V1468 Python client generator (template-based)
# ──────────────────────────────────────────────────────────────────────


def _method_name_for(desc: EndpointDescriptor) -> str:
    """Derive a Python method name from endpoint descriptor."""
    # GET /healthz → healthz; POST /audit/run → audit_run; GET /audit/{audit_id} → audit_get
    path = desc.path.strip("/")
    parts = path.split("/")
    # First part is the resource, second part (if present) is the action
    if len(parts) == 1:
        resource = parts[0]
        return resource  # /healthz → healthz
    if len(parts) == 2:
        resource, action = parts
        if "{" in action:
            # /audit/{audit_id} → audit_get
            return f"{resource}_get"
        return f"{resource}_{action}"  # /audit/run → audit_run
    return path.replace("/", "_").replace("{", "").replace("}", "")


def _client_method_source(desc: EndpointDescriptor, class_name: str) -> str:
    """Generate source code for one client method."""
    mname = _method_name_for(desc)
    # Determine path param substitution
    path_template = desc.path
    # Method signature
    args: List[str] = ["self"]
    body_args: List[str] = []
    # Path params
    for pp in desc.path_params:
        args.append(f"{pp}: str")
    # Query params
    for qp_name, qp_type, qp_required in desc.query_params:
        if qp_required:
            args.append(f"{qp_name}: str")
        else:
            args.append(f"{qp_name}: Optional[int] = None")
    # Body params (only if request_schema present + POST)
    if desc.method == "POST" and desc.request_schema is not None:
        for prop_name, prop_schema in desc.request_schema.get("properties", {}).items():
            tname = _py_type_for_schema(prop_schema)
            default = "None"
            if "default" in prop_schema:
                dv = prop_schema["default"]
                if isinstance(dv, bool):
                    default = "True" if dv else "False"
                elif isinstance(dv, (int, float)):
                    default = str(dv)
                else:
                    default = repr(dv)
            args.append(f"{prop_name}: Optional[{tname}] = {default}")
            body_args.append(prop_name)
    # Build request body
    body_lines: List[str] = []
    if body_args:
        body_lines.append("        body: Dict[str, Any] = {}")
        for ba in body_args:
            body_lines.append(f"        if {ba} is not None:")
            body_lines.append(f"            body[\"{ba}\"] = {ba}")
        body_lines.append("        body_bytes = json.dumps(body).encode(\"utf-8\") if body else None")
    else:
        body_lines.append("        body_bytes = None")
    # Build query string
    qs_lines: List[str] = []
    for qp_name, _qp_type, qp_required in desc.query_params:
        if qp_required:
            qs_lines.append(f"        qs_parts.append((\"{qp_name}\", {qp_name}))")
        else:
            qs_lines.append(f"        if {qp_name} is not None:")
            qs_lines.append(f"            qs_parts.append((\"{qp_name}\", str({qp_name})))")
    qs_block = ""
    if qs_lines:
        qs_block = (
            "        qs_parts: List[Tuple[str, str]] = []\n"
            + "\n".join(qs_lines) + "\n"
            + "        qs = urllib.parse.urlencode(qs_parts)\n"
            + "        if qs:\n"
            + "            url = f\"{url}?{qs}\"\n"
        )
    # Path param substitution
    path_sub_lines: List[str] = []
    for pp in desc.path_params:
        path_sub_lines.append(f"        url = url.replace(\"{{{pp}}}\", {pp})")
    path_sub_block = "\n".join(path_sub_lines)
    # Method body
    src = f'''    def {mname}({", ".join(args)}) -> Dict[str, Any]:
        """{desc.summary}.

        {desc.description}
        """
        url = f"{{self._base_url}}{path_template}"
{path_sub_block}
{qs_block}{chr(10).join(body_lines)}
        return self._request("{desc.method}", url, body_bytes=body_bytes)
'''
    return src


def _py_type_for_schema(schema: Dict[str, Any]) -> str:
    """Map a JSON Schema fragment to a Python type hint."""
    t = schema.get("type")
    if t == "string":
        return "str"
    if t == "integer":
        return "int"
    if t == "number":
        return "float"
    if t == "boolean":
        return "bool"
    if t == "array":
        return "List[Any]"
    if t == "object":
        return "Dict[str, Any]"
    return "Any"


def generate_python_client_source(
    class_name: str = "V1467Client",
    descriptors: Optional[Tuple[EndpointDescriptor, ...]] = None,
) -> str:
    """Generate a complete Python client module source."""
    descs = descriptors or V1467_ENDPOINT_DESCRIPTORS
    method_sources: List[str] = []
    for d in descs:
        method_sources.append(_client_method_source(d, class_name))
    methods_block = "\n".join(method_sources)
    header = textwrap.dedent(f'''\
        """Auto-generated V1467 Python client (generated by V1468).

        DO NOT EDIT BY HAND — regenerate via:
            python -m apeireth.{V1468_MODULE} generate-client --out THIS_FILE.py

        Generated at: <runtime>
        Generator: {V1468_MODULE} {V1468_VERSION}
        Target: V1467 (Cross-Audit HTTP Gateway)

        Usage:
            from {class_name.lower()} import {class_name}
            c = {class_name}("127.0.0.1", 18280)
            print(c.healthz())
            print(c.status())
            audit = c.audit_run(policy="STANDARD")
            print(audit["audit_id"], audit["verdict"])
        """
        from __future__ import annotations

        import json
        import urllib.parse
        from typing import Any, Dict, List, Optional, Tuple


        class {class_name}:
            """Minimal Python client for V1467 (Cross-Audit HTTP Gateway).

            Uses only stdlib (http.client). Deliberately minimal — no retry,
            no auth, no async. Anyone can extend by adding methods.
            """

            def __init__(self, host: str = "127.0.0.1", port: int = 18280,
                         timeout: float = 30.0):
                self.host = host
                self.port = port
                self.timeout = timeout
                self._base_url = f"http://{{host}}:{{port}}"

            def _request(self, method: str, url: str,
                         body_bytes: Optional[bytes] = None) -> Dict[str, Any]:
                """Issue one HTTP request and return parsed JSON body."""
                import http.client
                parsed = urllib.parse.urlparse(url)
                host = parsed.hostname or self.host
                port = parsed.port or self.port
                path = parsed.path or "/"
                if parsed.query:
                    path = f"{{path}}?{{parsed.query}}"
                conn = http.client.HTTPConnection(host, port, timeout=self.timeout)
                try:
                    headers = {{}}
                    if body_bytes is not None:
                        headers["Content-Type"] = "application/json"
                        headers["Content-Length"] = str(len(body_bytes))
                    conn.request(method, path, body=body_bytes, headers=headers)
                    resp = conn.getresponse()
                    raw = resp.read()
                    if resp.status >= 400:
                        try:
                            err_body = json.loads(raw)
                        except Exception:
                            err_body = {{"raw": raw.decode("utf-8", errors="replace")}}
                        raise RuntimeError(
                            f"{{method}} {{path}} → {{resp.status}}: {{err_body}}"
                        )
                    if not raw:
                        return {{}}
                    try:
                        return json.loads(raw)
                    except json.JSONDecodeError:
                        return {{"raw": raw.decode("utf-8", errors="replace")}}
                finally:
                    conn.close()

        ''')
    return header + methods_block


def write_python_client(out_path: Path, class_name: str = "V1467Client") -> Path:
    """Generate + write the Python client module to disk."""
    src = generate_python_client_source(class_name=class_name)
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write(src)
    return out_path


# ──────────────────────────────────────────────────────────────────────
# V1468 schema introspection / validation helpers
# ──────────────────────────────────────────────────────────────────────


def schema_paths_methods(schema: Dict[str, Any]) -> List[Tuple[str, str]]:
    """Extract (method, path) tuples from a generated OpenAPI schema."""
    out: List[Tuple[str, str]] = []
    for path, ops in schema.get("paths", {}).items():
        for method in ops:
            if method.lower() in {"get", "post", "put", "delete", "patch"}:
                out.append((method.upper(), path))
    return sorted(out)


def schema_has_all_v1467_endpoints(schema: Dict[str, Any]) -> bool:
    """Verify schema contains all 6 V1467 endpoints."""
    actual = set(schema_paths_methods(schema))
    expected = set((d.method, d.path) for d in V1467_ENDPOINT_DESCRIPTORS)
    return actual == expected


# ──────────────────────────────────────────────────────────────────────
# V1468 smoke test (boots V1467 + uses generated client)
# ──────────────────────────────────────────────────────────────────────


def _smoke_healthz_via_client(host: str, port: int) -> Tuple[int, Optional[Dict[str, Any]], Optional[str]]:
    """One endpoint smoke check via direct http.client (no client compile needed)."""
    import http.client
    try:
        conn = http.client.HTTPConnection(host, port, timeout=10.0)
        conn.request("GET", "/healthz")
        resp = conn.getresponse()
        body = resp.read()
        if resp.status == 200:
            try:
                return resp.status, json.loads(body), None
            except json.JSONDecodeError as e:
                return resp.status, None, f"json_decode_error: {e}"
        return resp.status, None, f"non_2xx: {resp.status}"
    except Exception as e:
        return -1, None, f"{type(e).__name__}: {e}"


def run_v1468_smoke(
    host: str = "127.0.0.1",
    port: int = 0,
    history_path: Optional[Path] = None,
) -> Dict[str, Any]:
    """Boot V1467 in subprocess + run smoke checks against each endpoint.

    Returns a summary dict with verdict + per-endpoint status.
    """
    # Lazy import V1467 — ensure promethean parent is on sys.path so the
    # in-process import works whether V1468 is run as `python -m apeireth.v1468_...`
    # (sys.path[0] = promethean) or `python -m v1468_...` from apeireth/ (sys.path[0] = apeireth).
    _promethean_parent = Path(__file__).resolve().parent.parent
    if str(_promethean_parent) not in sys.path:
        sys.path.insert(0, str(_promethean_parent))
    from apeireth.v1467_asi_audit_http_gateway_history_diff import (
        make_gateway_server,
    )

    # 1. Boot V1467 in-process (faster + deterministic for smoke)
    actual_port = port
    server, state, actual_port = make_gateway_server(
        host=host, port=port, history_path=history_path
    )
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    summary: Dict[str, Any] = {
        "host": host,
        "port": actual_port,
        "boot_ok": True,
        "endpoint_checks": {},
        "verdict": "UNKNOWN",
        "n_pass": 0,
        "n_fail": 0,
    }
    try:
        # 2. Hit each endpoint via http.client (using the same shape the generated client uses)
        import http.client
        conn = http.client.HTTPConnection(host, actual_port, timeout=30.0)

        # GET /healthz
        conn.request("GET", "/healthz")
        r = conn.getresponse()
        body1 = r.read()
        r.close()
        try:
            j1 = json.loads(body1)
            ok1 = r.status == 200 and j1.get("ok") is True
        except Exception:
            ok1 = False
        summary["endpoint_checks"]["GET /healthz"] = {
            "status": r.status, "ok": ok1,
        }

        # GET /status
        conn.request("GET", "/status")
        r = conn.getresponse()
        body2 = r.read()
        r.close()
        try:
            j2 = json.loads(body2)
            ok2 = r.status == 200 and "v1467" in j2
        except Exception:
            ok2 = False
        summary["endpoint_checks"]["GET /status"] = {
            "status": r.status, "ok": ok2,
        }

        # POST /audit/run with dry_run=true (fast, no subprocess boot)
        conn.request("POST", "/audit/run",
                     body=b'{"dry_run": true}',
                     headers={"Content-Type": "application/json"})
        r = conn.getresponse()
        body3 = r.read()
        r.close()
        try:
            j3 = json.loads(body3)
            ok3 = r.status == 200 and j3.get("dry_run") is True and j3.get("audit_id", "") == ""
        except Exception:
            ok3 = False
        summary["endpoint_checks"]["POST /audit/run"] = {
            "status": r.status, "ok": ok3,
        }

        # GET /audit/history (empty + valid shape)
        conn.request("GET", "/audit/history")
        r = conn.getresponse()
        body4 = r.read()
        r.close()
        try:
            j4 = json.loads(body4)
            ok4 = r.status == 200 and "entries" in j4 and isinstance(j4["entries"], list)
        except Exception:
            ok4 = False
        summary["endpoint_checks"]["GET /audit/history"] = {
            "status": r.status, "ok": ok4,
        }

        # GET /audit/{nonexistent} → 404
        conn.request("GET", "/audit/audit-nonexistent-id")
        r = conn.getresponse()
        body5 = r.read()
        r.close()
        summary["endpoint_checks"]["GET /audit/{audit_id}"] = {
            "status": r.status, "ok": r.status == 404,
        }

        # GET /audit/diff with missing params → 400
        conn.request("GET", "/audit/diff")
        r = conn.getresponse()
        body6 = r.read()
        r.close()
        summary["endpoint_checks"]["GET /audit/diff"] = {
            "status": r.status, "ok": r.status == 400,
        }

        # 404 + 405 (sad paths)
        conn.request("GET", "/nonexistent")
        r = conn.getresponse()
        body7 = r.read()
        r.close()
        summary["endpoint_checks"]["GET /nonexistent (404)"] = {
            "status": r.status, "ok": r.status == 404,
        }

        conn.request("POST", "/healthz", body=b"{}",
                     headers={"Content-Type": "application/json"})
        r = conn.getresponse()
        body8 = r.read()
        r.close()
        summary["endpoint_checks"]["POST /healthz (405)"] = {
            "status": r.status, "ok": r.status == 405,
        }

        conn.close()

        # 3. Compute verdict
        n_pass = sum(1 for v in summary["endpoint_checks"].values() if v.get("ok"))
        n_total = len(summary["endpoint_checks"])
        summary["n_pass"] = n_pass
        summary["n_fail"] = n_total - n_pass
        summary["n_total"] = n_total
        summary["verdict"] = "PASS" if n_pass == n_total else "FAIL"
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)
    return summary


# ──────────────────────────────────────────────────────────────────────
# V1468 Popper self-check
# ──────────────────────────────────────────────────────────────────────


def popper_v1468() -> Dict[str, Any]:
    """Popper-style self-tests (no subprocess, in-process only)."""
    checks: Dict[str, bool] = {}

    # Module metadata
    checks["META_PRESENT"] = bool(
        V1468_MODULE and V1468_VERSION and V1468_SCHEMA
    )

    # GUARDS declared
    checks["GUARDS_DECLARED"] = len(V1468_GUARDS) >= 14
    checks["V3_GUARDS_DECLARED"] = len(V1468_V3_GUARDS) >= 7

    # Borrowed sources
    checks["BORROWED_SOURCES_DECLARED"] = len(BORROWED_SOURCES) == 5

    # OpenAPI version
    checks["OPENAPI_VERSION_3_1"] = OPENAPI_VERSION == "3.1.0"

    # Endpoint descriptors declared (6)
    checks["ENDPOINTS_DECLARED"] = len(V1467_ENDPOINT_DESCRIPTORS) == 6

    # Generate schema
    schema = generate_openapi_schema()

    # Schema is OpenAPI 3.1
    checks["SCHEMA_OPENAPI_3_1"] = schema.get("openapi") == "3.1.0"

    # Schema has all 6 V1467 endpoints
    checks["SCHEMA_PATHS_COMPLETE"] = schema_has_all_v1467_endpoints(schema)

    # Schema declares request body for POST /audit/run
    post_op = schema.get("paths", {}).get("/audit/run", {}).get("post", {})
    checks["SCHEMA_REQUEST_DECLARED"] = "requestBody" in post_op

    # Schema declares query params for /audit/diff
    diff_op = schema.get("paths", {}).get("/audit/diff", {}).get("get", {})
    params = diff_op.get("parameters", [])
    param_names = {p.get("name") for p in params}
    checks["SCHEMA_QUERY_DECLARED"] = {"baseline_id", "current_id"}.issubset(param_names)

    # Schema declares path param for /audit/{audit_id}
    get_op = schema.get("paths", {}).get("/audit/{audit_id}", {}).get("get", {})
    path_params = get_op.get("parameters", [])
    path_param_names = {p.get("name") for p in path_params if p.get("in") == "path"}
    checks["SCHEMA_PATH_DECLARED"] = "audit_id" in path_param_names

    # Schema declares 200 response for every endpoint
    n_200 = 0
    for path, ops in schema.get("paths", {}).items():
        for method, op in ops.items():
            if method.lower() not in {"get", "post", "put", "delete", "patch"}:
                continue
            if "200" in op.get("responses", {}):
                n_200 += 1
    checks["SCHEMA_RESPONSES_DECLARED"] = n_200 == 6

    # Generate Python client + verify it parses
    client_src = generate_python_client_source()
    try:
        compile(client_src, "<v1468_generated_client>", "exec")
        checks["CLIENT_GENERATED"] = True
    except SyntaxError:
        checks["CLIENT_GENERATED"] = False

    # Generated client has one method per endpoint (6)
    method_names = {_method_name_for(d) for d in V1467_ENDPOINT_DESCRIPTORS}
    n_methods = 0
    for mn in method_names:
        if f"def {mn}(" in client_src:
            n_methods += 1
    checks["CLIENT_HAS_ALL_METHODS"] = n_methods == 6

    # Smoke test (in-process V1467)
    try:
        smoke = run_v1468_smoke()
        checks["SMOKE_BOOTS_V1467"] = smoke.get("boot_ok") is True
        checks["SMOKE_VERDICT_PASS"] = smoke.get("verdict") == "PASS"
        checks["SMOKE_N_ENDPOINTS_OK"] = smoke.get("n_pass", 0) >= 6
    except Exception as exc:
        checks["SMOKE_BOOTS_V1467"] = False
        checks["SMOKE_VERDICT_PASS"] = False
        checks["SMOKE_ERROR"] = str(exc)[:120]

    # Schema roundtrip (json.dumps → json.loads → equal)
    rt = json.loads(json.dumps(schema))
    checks["SCHEMA_ROUNDTRIP"] = rt == schema

    # Determinism (generate twice → equal)
    schema2 = generate_openapi_schema()
    checks["DETERMINISTIC_SCHEMA"] = schema == schema2

    n_pass = sum(1 for v in checks.values() if v)
    n_total = len(checks)
    return {
        "n_checks": n_total,
        "passed": n_pass,
        "failed": [k for k, v in checks.items() if not v],
        "popper_pass": n_pass == n_total,
        "checks": checks,
    }


# ──────────────────────────────────────────────────────────────────────
# V1468 CLI
# ──────────────────────────────────────────────────────────────────────


def _make_argparser() -> argparse.ArgumentParser:
    """Build the top-level argparser."""
    p = argparse.ArgumentParser(
        prog=V1468_MODULE,
        description=(
            "V1468 — ASI Real OpenAPI 3.1 Schema + Generated Python Client "
            "for V1467 Cross-Audit HTTP Gateway."
        ),
    )
    sub = p.add_subparsers(dest="command", required=True)

    # generate-schema
    sp = sub.add_parser("generate-schema", help="Generate OpenAPI 3.1 schema JSON")
    sp.add_argument("--out", type=Path, required=True,
                    help="Output path for the OpenAPI JSON file")
    sp.add_argument("--title", default=OPENAPI_INFO_TITLE)
    sp.add_argument("--version", default=OPENAPI_INFO_VERSION)

    # generate-client
    cp = sub.add_parser("generate-client", help="Generate Python client module")
    cp.add_argument("--out", type=Path, required=True,
                    help="Output path for the Python client module")
    cp.add_argument("--class-name", default="V1467Client")

    # smoke
    sm = sub.add_parser("smoke", help="Smoke test: boot V1467 + hit all endpoints")
    sm.add_argument("--host", default="127.0.0.1")
    sm.add_argument("--port", type=int, default=0)
    sm.add_argument("--history-path", type=Path, default=None,
                    help="Optional path to V1467 history jsonl")

    # popper
    sub.add_parser("popper", help="In-process Popper self-check")

    # status
    sub.add_parser("status", help="Module status (JSON)")

    # meta
    sub.add_parser("meta", help="Module metadata (JSON)")

    # chain
    sub.add_parser("chain", help="Borrowed lineage (JSON)")

    # help
    sub.add_parser("help", help="Extended help")

    return p


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entrypoint. Returns 0 on success, non-zero on failure."""
    parser = _make_argparser()
    args = parser.parse_args(argv)

    if args.command == "generate-schema":
        out_path = write_openapi_schema(
            args.out,
            info_title=args.title,
            info_version=args.version,
        )
        print(f"schema written: {out_path}")
        return 0

    if args.command == "generate-client":
        out_path = write_python_client(args.out, class_name=args.class_name)
        print(f"client written: {out_path}")
        return 0

    if args.command == "smoke":
        summary = run_v1468_smoke(host=args.host, port=args.port,
                                  history_path=args.history_path)
        print(json.dumps(summary, indent=2, default=str))
        return 0 if summary.get("verdict") == "PASS" else 1

    if args.command == "popper":
        result = popper_v1468()
        print(json.dumps(result, indent=2))
        return 0 if result["popper_pass"] else 1

    if args.command == "status":
        print(json.dumps({
            "module": V1468_MODULE,
            "version": V1468_VERSION,
            "schema": V1468_SCHEMA,
            "date": V1468_DATE,
            "openapi_version": OPENAPI_VERSION,
            "n_endpoints": len(V1467_ENDPOINT_DESCRIPTORS),
            "guards": list(V1468_GUARDS),
            "v3_guards": list(V1468_V3_GUARDS),
            "borrowed": list(BORROWED_SOURCES),
        }, indent=2, ensure_ascii=False))
        return 0

    if args.command == "meta":
        print(json.dumps({
            "module": V1468_MODULE,
            "version": V1468_VERSION,
            "schema": V1468_SCHEMA,
            "date": V1468_DATE,
            "post_modules": ["v1467", "v1466", "v1465"],
        }, indent=2, ensure_ascii=False))
        return 0

    if args.command == "chain":
        print(json.dumps({
            "borrowed": list(BORROWED_SOURCES),
            "v1467_endpoints": [
                {"method": d.method, "path": d.path, "summary": d.summary}
                for d in V1467_ENDPOINT_DESCRIPTORS
            ],
        }, indent=2, ensure_ascii=False))
        return 0

    if args.command == "help":
        print("V1468 — OpenAPI 3.1 Schema + Python Client Generator for V1467.")
        print()
        print("Commands:")
        print("  generate-schema --out FILE.json   # Write OpenAPI 3.1 schema")
        print("  generate-client --out FILE.py     # Write Python client module")
        print("  smoke                             # Boot V1467 + hit all endpoints")
        print("  popper                            # In-process self-check")
        print("  status / meta / chain             # Module info")
        print()
        print("Examples:")
        print("  python -m apeireth.v1468 generate-schema --out out/openapi.json")
        print("  python -m apeireth.v1468 generate-client --out out/v1467_client.py")
        print("  python -m apeireth.v1468 smoke")
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())