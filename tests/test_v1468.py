"""Tests for V1468 — ASI Real OpenAPI 3.1 Schema + Generated Python Client for V1467.

Run: python -m pytest tests/test_v1468.py -v

Coverage:
  - Module metadata + bounded defaults
  - EndpointDescriptor dataclass (6 endpoints)
  - generate_openapi_schema: openapi=3.1.0, paths complete, request/query/path/response declared
  - schema_has_all_v1467_endpoints: 6 endpoints match
  - write_openapi_schema: writes JSON to disk
  - generate_python_client_source: parses without SyntaxError, has 6 methods
  - write_python_client: writes Python module to disk
  - _client_method_source: one method per descriptor
  - _py_type_for_schema: type mapping
  - popper_v1468: 19 in-process self-checks
  - run_v1468_smoke: boots V1467 + hits 8 endpoints via http.client
  - Generated client roundtrip: V1468.generate-client → import → use against V1467
"""

from __future__ import annotations

import importlib.util
import json
import os
import socket
import sys
import tempfile
import threading
import time
from pathlib import Path

import pytest

# Ensure repo root is importable
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1468_asi_openapi_v1467_schema_and_client_generator import (
    V1468_MODULE, V1468_VERSION, V1468_SCHEMA, V1468_DATE,
    OPENAPI_VERSION, OPENAPI_INFO_TITLE, OPENAPI_INFO_VERSION,
    BORROWED_SOURCES, V1468_GUARDS, V1468_V3_GUARDS,
    EndpointDescriptor, V1467_ENDPOINT_DESCRIPTORS,
    generate_openapi_schema, write_openapi_schema,
    generate_python_client_source, write_python_client,
    schema_paths_methods, schema_has_all_v1467_endpoints,
    _descriptor_to_operation, _method_name_for, _py_type_for_schema,
    _operation_id, _client_method_source,
    popper_v1468, run_v1468_smoke,
    main,
)


# ──────────────────────────────────────────────────────────────────────
# Module metadata + constants
# ──────────────────────────────────────────────────────────────────────


def test_module_metadata():
    assert V1468_MODULE == "v1468_asi_openapi_v1467_schema_and_client_generator"
    assert V1468_VERSION == "0.1.0"
    assert V1468_SCHEMA == "v1468.asi-openapi-v1467-schema-and-client-generator/v1"
    assert V1468_DATE == "2026-08-10"


def test_openapi_constants():
    assert OPENAPI_VERSION == "3.1.0"
    assert "Apeireth" in OPENAPI_INFO_TITLE
    assert OPENAPI_INFO_VERSION == "1.0.0"


def test_borrowed_sources_declared():
    assert len(BORROWED_SOURCES) == 5
    assert "v1467" in BORROWED_SOURCES
    assert "stdlib" in BORROWED_SOURCES


def test_guards_declared():
    assert len(V1468_GUARDS) >= 14
    assert len(V1468_V3_GUARDS) >= 7
    # V3 guards explicit anti-ASI / anti-Phenomenal claims
    assert any("NOT_ASI" in g for g in V1468_V3_GUARDS)
    assert any("NOT_PHENOMENAL" in g for g in V1468_V3_GUARDS)


# ──────────────────────────────────────────────────────────────────────
# EndpointDescriptor dataclass + descriptors
# ──────────────────────────────────────────────────────────────────────


def test_endpoint_descriptor_dataclass():
    d = EndpointDescriptor(
        method="GET",
        path="/x",
        summary="x",
        description="x",
    )
    assert d.method == "GET"
    assert d.path == "/x"
    assert d.path_params == ()
    assert d.query_params == ()
    assert d.request_schema is None
    assert d.response_200_schema is None


def test_six_endpoints_declared():
    assert len(V1467_ENDPOINT_DESCRIPTORS) == 6
    methods = {d.method for d in V1467_ENDPOINT_DESCRIPTORS}
    assert methods == {"GET", "POST"}


def test_endpoints_have_required_fields():
    for d in V1467_ENDPOINT_DESCRIPTORS:
        assert d.method in ("GET", "POST")
        assert d.path.startswith("/")
        assert d.summary != ""
        assert d.description != ""


def test_audit_run_has_request_body():
    """POST /audit/run must declare requestBody with policy/audit_host/dry_run."""
    audit_run = next(d for d in V1467_ENDPOINT_DESCRIPTORS if d.path == "/audit/run")
    assert audit_run.method == "POST"
    assert audit_run.request_schema is not None
    props = audit_run.request_schema["properties"]
    assert "policy" in props
    assert "audit_host" in props
    assert "dry_run" in props


def test_audit_diff_has_required_query_params():
    """GET /audit/diff must declare baseline_id + current_id as required."""
    diff = next(d for d in V1467_ENDPOINT_DESCRIPTORS if d.path == "/audit/diff")
    qp_names = {q[0] for q in diff.query_params}
    assert "baseline_id" in qp_names
    assert "current_id" in qp_names
    # Both should be required
    for name, _type, required in diff.query_params:
        if name in ("baseline_id", "current_id"):
            assert required is True


def test_audit_get_has_path_param():
    """GET /audit/{audit_id} must declare audit_id as path param."""
    audit_get = next(d for d in V1467_ENDPOINT_DESCRIPTORS if "{audit_id}" in d.path)
    assert "audit_id" in audit_get.path_params


# ──────────────────────────────────────────────────────────────────────
# generate_openapi_schema
# ──────────────────────────────────────────────────────────────────────


def test_generate_schema_openapi_3_1():
    schema = generate_openapi_schema()
    assert schema["openapi"] == "3.1.0"


def test_generate_schema_info_present():
    schema = generate_openapi_schema()
    info = schema["info"]
    assert info["title"] == OPENAPI_INFO_TITLE
    assert info["version"] == OPENAPI_INFO_VERSION
    assert "x_generator" in info
    assert info["x_generator"] == V1468_MODULE


def test_generate_schema_has_six_paths():
    schema = generate_openapi_schema()
    paths = schema["paths"]
    assert len(paths) == 6
    assert "/healthz" in paths
    assert "/status" in paths
    assert "/audit/run" in paths
    assert "/audit/history" in paths
    assert "/audit/{audit_id}" in paths
    assert "/audit/diff" in paths


def test_generate_schema_paths_methods():
    schema = generate_openapi_schema()
    methods_paths = schema_paths_methods(schema)
    assert len(methods_paths) == 6
    # Sorted output
    assert methods_paths == sorted(methods_paths)
    # All V1467 endpoints present
    assert ("GET", "/healthz") in methods_paths
    assert ("POST", "/audit/run") in methods_paths


def test_schema_has_all_v1467_endpoints():
    schema = generate_openapi_schema()
    assert schema_has_all_v1467_endpoints(schema) is True


def test_schema_post_audit_run_has_request_body():
    schema = generate_openapi_schema()
    post_op = schema["paths"]["/audit/run"]["post"]
    assert "requestBody" in post_op
    body_schema = post_op["requestBody"]["content"]["application/json"]["schema"]
    assert "policy" in body_schema["properties"]


def test_schema_diff_has_query_params():
    schema = generate_openapi_schema()
    diff_op = schema["paths"]["/audit/diff"]["get"]
    params = diff_op.get("parameters", [])
    names = {p["name"]: p for p in params}
    assert "baseline_id" in names
    assert "current_id" in names
    assert names["baseline_id"]["required"] is True
    assert names["current_id"]["required"] is True


def test_schema_get_audit_id_has_path_param():
    schema = generate_openapi_schema()
    get_op = schema["paths"]["/audit/{audit_id}"]["get"]
    params = get_op.get("parameters", [])
    path_params = [p for p in params if p.get("in") == "path"]
    assert any(p["name"] == "audit_id" for p in path_params)


def test_schema_every_endpoint_has_200_response():
    schema = generate_openapi_schema()
    n_200 = 0
    for path, ops in schema["paths"].items():
        for method, op in ops.items():
            if method.lower() not in {"get", "post"}:
                continue
            if "200" in op.get("responses", {}):
                n_200 += 1
    assert n_200 == 6


def test_schema_components_present():
    schema = generate_openapi_schema()
    assert "components" in schema
    assert "schemas" in schema["components"]
    assert "AuditHistoryEntry" in schema["components"]["schemas"]
    assert "AuditDiff" in schema["components"]["schemas"]


def test_schema_roundtrip_json():
    schema = generate_openapi_schema()
    s = json.dumps(schema)
    rt = json.loads(s)
    assert rt == schema


def test_schema_deterministic():
    s1 = generate_openapi_schema()
    s2 = generate_openapi_schema()
    assert s1 == s2


def test_write_openapi_schema_creates_file():
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "openapi.json"
        written = write_openapi_schema(out)
        assert written == out
        assert out.exists()
        with open(out, encoding="utf-8") as f:
            data = json.load(f)
        assert data["openapi"] == "3.1.0"
        assert len(data["paths"]) == 6


# ──────────────────────────────────────────────────────────────────────
# Python client generator
# ──────────────────────────────────────────────────────────────────────


def test_generate_client_parses():
    """Generated client source must compile without SyntaxError."""
    src = generate_python_client_source()
    compile(src, "<v1468_generated>", "exec")


def test_generate_client_has_class():
    src = generate_python_client_source()
    assert "class V1467Client:" in src


def test_generate_client_has_six_methods():
    """One method per endpoint."""
    src = generate_python_client_source()
    expected_methods = {"healthz", "status", "audit_run",
                        "audit_history", "audit_get", "audit_diff"}
    found = 0
    for m in expected_methods:
        if f"def {m}(" in src:
            found += 1
    assert found == 6, f"expected 6 methods, found {found}"


def test_generate_client_uses_only_stdlib():
    """Generated client must use only stdlib (no third-party imports)."""
    src = generate_python_client_source()
    # Check for common 3rd-party imports that should NOT appear
    forbidden = ["import requests", "import httpx", "import aiohttp",
                 "import urllib3", "import flask", "import fastapi"]
    for f in forbidden:
        assert f not in src, f"found forbidden import: {f}"
    # stdlib imports OK
    assert "import json" in src
    assert "import urllib.parse" in src


def test_write_python_client_creates_file():
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "v1467_client.py"
        written = write_python_client(out)
        assert written == out
        assert out.exists()
        assert out.stat().st_size > 500  # non-trivial file
        # Verify it parses
        with open(out, encoding="utf-8") as f:
            src = f.read()
        compile(src, str(out), "exec")


def test_method_name_for_simple_path():
    d = EndpointDescriptor(method="GET", path="/healthz", summary="", description="")
    assert _method_name_for(d) == "healthz"


def test_method_name_for_action_path():
    d = EndpointDescriptor(method="POST", path="/audit/run", summary="", description="")
    assert _method_name_for(d) == "audit_run"


def test_method_name_for_path_param():
    d = EndpointDescriptor(method="GET", path="/audit/{audit_id}", summary="", description="")
    assert _method_name_for(d) == "audit_get"


def test_py_type_for_schema():
    assert _py_type_for_schema({"type": "string"}) == "str"
    assert _py_type_for_schema({"type": "integer"}) == "int"
    assert _py_type_for_schema({"type": "number"}) == "float"
    assert _py_type_for_schema({"type": "boolean"}) == "bool"
    assert _py_type_for_schema({"type": "array"}) == "List[Any]"
    assert _py_type_for_schema({"type": "object"}) == "Dict[str, Any]"


def test_operation_id_unique_per_endpoint():
    ids = [_operation_id(d.method, d.path) for d in V1467_ENDPOINT_DESCRIPTORS]
    assert len(ids) == len(set(ids)), f"duplicate operationIds: {ids}"


def test_descriptor_to_operation_basic():
    d = EndpointDescriptor(
        method="GET", path="/x", summary="X", description="X endpoint",
        response_200_schema={"type": "object"},
    )
    op = _descriptor_to_operation(d)
    assert op["summary"] == "X"
    assert op["description"] == "X endpoint"
    assert "responses" in op
    assert "200" in op["responses"]


def test_descriptor_to_operation_with_path_param():
    d = EndpointDescriptor(
        method="GET", path="/audit/{audit_id}", summary="S", description="D",
        path_params=("audit_id",),
    )
    op = _descriptor_to_operation(d)
    params = op.get("parameters", [])
    path_params = [p for p in params if p.get("in") == "path"]
    assert any(p["name"] == "audit_id" for p in path_params)


def test_descriptor_to_operation_with_query_params():
    d = EndpointDescriptor(
        method="GET", path="/audit/diff", summary="S", description="D",
        query_params=(("baseline_id", "string", True), ("current_id", "string", True)),
    )
    op = _descriptor_to_operation(d)
    params = op.get("parameters", [])
    q_params = [p for p in params if p.get("in") == "query"]
    names = {p["name"] for p in q_params}
    assert names == {"baseline_id", "current_id"}


def test_client_method_source_includes_docstring():
    d = next(d for d in V1467_ENDPOINT_DESCRIPTORS if d.path == "/healthz")
    src = _client_method_source(d, "V1467Client")
    assert "def healthz(self)" in src
    assert '"""' in src or "'''" in src  # docstring
    assert "GET" in src


# ──────────────────────────────────────────────────────────────────────
# popper_v1468 — 19 in-process self-checks (主 17:43 实事求是)
# ──────────────────────────────────────────────────────────────────────


def test_popper_v1468_all_pass():
    """All 19 popper checks must pass for V1468 to be considered 真生产."""
    r = popper_v1468()
    assert r["popper_pass"] is True, f"failed: {r['failed']}"
    assert r["passed"] == 19
    assert r["failed"] == []
    expected = {
        "META_PRESENT", "GUARDS_DECLARED", "V3_GUARDS_DECLARED",
        "BORROWED_SOURCES_DECLARED", "OPENAPI_VERSION_3_1",
        "ENDPOINTS_DECLARED", "SCHEMA_OPENAPI_3_1", "SCHEMA_PATHS_COMPLETE",
        "SCHEMA_REQUEST_DECLARED", "SCHEMA_QUERY_DECLARED",
        "SCHEMA_PATH_DECLARED", "SCHEMA_RESPONSES_DECLARED",
        "CLIENT_GENERATED", "CLIENT_HAS_ALL_METHODS",
        "SMOKE_BOOTS_V1467", "SMOKE_VERDICT_PASS", "SMOKE_N_ENDPOINTS_OK",
        "SCHEMA_ROUNDTRIP", "DETERMINISTIC_SCHEMA",
    }
    assert set(r["checks"].keys()) == expected


# ──────────────────────────────────────────────────────────────────────
# run_v1468_smoke — boots V1467 + hits 8 endpoints
# ──────────────────────────────────────────────────────────────────────


def test_smoke_verdict_pass():
    """Smoke test should boot V1467 + hit 8 endpoints with PASS verdict."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_history = Path(tmp) / "history.jsonl"
        summary = run_v1468_smoke(history_path=tmp_history)
    assert summary["boot_ok"] is True
    assert summary["verdict"] == "PASS"
    assert summary["n_pass"] >= 6
    assert summary["n_fail"] == 0


def test_smoke_all_endpoints_ok():
    """Each endpoint in smoke summary must have ok=True."""
    with tempfile.TemporaryDirectory() as tmp:
        tmp_history = Path(tmp) / "history.jsonl"
        summary = run_v1468_smoke(history_path=tmp_history)
    for ep, check in summary["endpoint_checks"].items():
        assert check.get("ok") is True, f"{ep} failed: {check}"


# ──────────────────────────────────────────────────────────────────────
# Generated client end-to-end (write → import → use against V1467)
# ──────────────────────────────────────────────────────────────────────


def test_generated_client_end_to_end():
    """Generate client → import → use against V1467 → verify roundtrip."""
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        # 1. Generate client
        client_path = tmpdir / "v1467_client.py"
        write_python_client(client_path)
        # 2. Generate schema (parallel)
        schema_path = tmpdir / "openapi.json"
        write_openapi_schema(schema_path)

        # 3. Boot V1467 server
        history_path = tmpdir / "history.jsonl"
        # Ensure parent dir on sys.path for the generated client
        sys.path.insert(0, str(tmpdir))
        sys.path.insert(0, str(ROOT))

        from apeireth.v1467_asi_audit_http_gateway_history_diff import make_gateway_server
        server, _state, port = make_gateway_server(
            host="127.0.0.1", port=0, history_path=history_path,
        )
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()

        try:
            # 4. Import generated client
            spec = importlib.util.spec_from_file_location("v1467_client_e2e", client_path)
            assert spec is not None
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)  # type: ignore[union-attr]

            # 5. Use client
            c = mod.V1467Client("127.0.0.1", port, timeout=15.0)

            # 5a. healthz
            h = c.healthz()
            assert h["ok"] is True
            assert h["module"] == "v1467_asi_audit_http_gateway_history_diff"

            # 5b. status
            s = c.status()
            assert "v1467" in s
            assert "chain" in s

            # 5c. audit_run dry
            a = c.audit_run(dry_run=True)
            assert a["dry_run"] is True
            assert a["verdict"] == "PASS"

            # 5d. audit_history
            hist = c.audit_history()
            assert "entries" in hist
            assert isinstance(hist["entries"], list)

            # 5e. audit_diff (empty params → 400)
            with pytest.raises(RuntimeError) as exc_info:
                c.audit_diff("", "")
            assert "400" in str(exc_info.value)

            # 5f. audit_get (nonexistent → 404)
            with pytest.raises(RuntimeError) as exc_info:
                c.audit_get("nonexistent-audit-id")
            assert "404" in str(exc_info.value)
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2.0)


# ──────────────────────────────────────────────────────────────────────
# CLI commands (subprocess-level integration)
# ──────────────────────────────────────────────────────────────────────


def test_cli_status_returns_json():
    """python -m v1468_... status → JSON output."""
    import subprocess as sp
    result = sp.run(
        [sys.executable, "-m",
         "apeireth.v1468_asi_openapi_v1467_schema_and_client_generator", "status"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=15,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    data = json.loads(result.stdout)
    assert data["module"] == V1468_MODULE
    assert data["openapi_version"] == "3.1.0"
    assert data["n_endpoints"] == 6


def test_cli_popper_returns_pass():
    import subprocess as sp
    result = sp.run(
        [sys.executable, "-m",
         "apeireth.v1468_asi_openapi_v1467_schema_and_client_generator", "popper"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=30,
    )
    data = json.loads(result.stdout)
    assert data["popper_pass"] is True
    assert data["passed"] == 19


def test_cli_smoke_returns_pass():
    import subprocess as sp
    result = sp.run(
        [sys.executable, "-m",
         "apeireth.v1468_asi_openapi_v1467_schema_and_client_generator", "smoke"],
        cwd=str(ROOT), capture_output=True, text=True, timeout=60,
    )
    data = json.loads(result.stdout)
    assert data["verdict"] == "PASS"
    assert data["n_pass"] >= 6


def test_cli_generate_schema_writes_file():
    import subprocess as sp
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "openapi.json"
        result = sp.run(
            [sys.executable, "-m",
             "apeireth.v1468_asi_openapi_v1467_schema_and_client_generator",
             "generate-schema", "--out", str(out)],
            cwd=str(ROOT), capture_output=True, text=True, timeout=15,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert out.exists()
        with open(out, encoding="utf-8") as f:
            data = json.load(f)
        assert data["openapi"] == "3.1.0"


def test_cli_generate_client_writes_file():
    import subprocess as sp
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "v1467_client.py"
        result = sp.run(
            [sys.executable, "-m",
             "apeireth.v1468_asi_openapi_v1467_schema_and_client_generator",
             "generate-client", "--out", str(out)],
            cwd=str(ROOT), capture_output=True, text=True, timeout=15,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert out.exists()
        assert out.stat().st_size > 500


def test_main_function_entry():
    """main() with --help returns 0 via subprocess (argparse exits on --help)."""
    import subprocess as sp
    result = sp.run(
        [sys.executable, "-m",
         "apeireth.v1468_asi_openapi_v1467_schema_and_client_generator", "--help"],
        cwd=str(ROOT), capture_output=True, timeout=15,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "generate-schema" in out
    assert "generate-client" in out
    assert "smoke" in out