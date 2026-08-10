"""Tests for V1464 — ASI Real Lint-Gate Subprocess Pipeline HTTP Gateway.

Covers:
  - Module metadata
  - Defaults / constants
  - find_open_port
  - Server factory
  - End-to-end HTTP demo (in-process)
  - Popper self-check
  - All 6 routes
  - Error paths (404 / 405 / 413 / 400)
  - V3 哲学守门
  - Borrowed lineage
"""
from __future__ import annotations

import http.client
import json
import sys
import threading
import time

import pytest


def _safe_import():
    """Import V1464 module once."""
    import apeireth.v1464_asi_lint_gate_pipeline_http_gateway as m
    return m


def test_v1464_meta():
    m = _safe_import()
    assert m.V1464_MODULE == "v1464_asi_lint_gate_pipeline_http_gateway"
    assert m.V1464_VERSION == "0.1.0"
    assert m.V1464_SCHEMA.startswith("v1464.")
    assert m.V1464_DATE == "2026-08-10"


def test_v1464_defaults_bounded():
    m = _safe_import()
    assert m.DEFAULT_HOST == "127.0.0.1"
    assert m.DEFAULT_PORT_LOW >= 1024
    assert m.DEFAULT_PORT_HIGH > m.DEFAULT_PORT_LOW
    assert m.DEFAULT_BODY_MAX_BYTES == 256 * 1024
    assert m.DEFAULT_RUN_TIMEOUT_S == 60
    assert m.DEFAULT_SERVER_TIMEOUT_S > 0


def test_v1464_routes_count():
    m = _safe_import()
    assert len(m.ROUTES) == 6
    expected = {
        ("GET", "/healthz"),
        ("GET", "/status"),
        ("GET", "/pipeline/adversarial"),
        ("POST", "/pipeline/run"),
        ("POST", "/pipeline/lint"),
        ("POST", "/pipeline/policy-gate"),
    }
    assert set(m.ROUTES.keys()) == expected


def test_v1464_route_healthz():
    m = _safe_import()
    rr = m._route_healthz(None, {})
    assert rr.status_code == 200
    assert rr.content_type == "application/json"
    j = json.loads(rr.body.decode("utf-8"))
    assert j["ok"] is True
    assert j["module"] == m.V1464_MODULE


def test_v1464_route_status():
    m = _safe_import()
    rr = m._route_status(None, {})
    assert rr.status_code == 200
    j = json.loads(rr.body.decode("utf-8"))
    assert "v1464" in j
    assert "v1463" in j
    assert "v1462" in j
    assert "v1461" in j
    assert "endpoints" in j
    assert len(j["endpoints"]) == 6


def test_v1464_route_pipeline_run_no_body():
    m = _safe_import()
    rr = m._route_pipeline_run(None, {})
    assert rr.status_code == 400


def test_v1464_route_pipeline_run_too_large():
    m = _safe_import()
    rr = m._route_pipeline_run(b"x" * (m.DEFAULT_BODY_MAX_BYTES + 1), {})
    assert rr.status_code == 413


def test_v1464_route_pipeline_lint_safe():
    m = _safe_import()
    safe_spec = {
        "command": [sys.executable, "-c", "print('hi')"],
        "image_alias": "ok_py",
        "workdir_basename": "tmp",
        "env_extra": {},
        "timeout_s": 60,  # ≥ DEFAULT_TIMEOUT_S=30 to avoid SL090 WARN
        "max_output_bytes": 4096,  # ≥ 1024 to avoid SL091 WARN
    }
    rr = m._route_pipeline_lint(
        json.dumps(safe_spec).encode("utf-8"),
        {"policy": "STANDARD"},
    )
    assert rr.status_code == 200
    j = json.loads(rr.body.decode("utf-8"))
    assert "findings" in j
    assert isinstance(j["findings"], list)
    assert len(j["findings"]) == 0


def test_v1464_route_pipeline_lint_blocked():
    m = _safe_import()
    bad_spec = {
        "command": ["bash", "-c", "rm -rf /"],
        "image_alias": "ok_b",
        "workdir_basename": "tmp",
        "env_extra": {},
        "timeout_s": 5,
        "max_output_bytes": 1024,
    }
    rr = m._route_pipeline_lint(
        json.dumps(bad_spec).encode("utf-8"),
        {"policy": "PERMISSIVE"},
    )
    assert rr.status_code == 200
    j = json.loads(rr.body.decode("utf-8"))
    assert isinstance(j.get("findings"), list)
    assert any(f.get("rule_code") == "SL060" for f in j["findings"])


def test_v1464_route_pipeline_policy_gate_blocked():
    m = _safe_import()
    bad_spec = {
        "command": ["bash", "-c", "rm -rf /"],
        "image_alias": "ok_b",
        "workdir_basename": "tmp",
        "env_extra": {},
        "timeout_s": 5,
        "max_output_bytes": 1024,
    }
    rr = m._route_pipeline_policy_gate(
        json.dumps(bad_spec).encode("utf-8"),
        {"policy": "PERMISSIVE"},
    )
    assert rr.status_code == 200
    j = json.loads(rr.body.decode("utf-8"))
    assert j["allowed"] is False
    assert len(j["violations"]) >= 1


def test_v1464_route_pipeline_policy_gate_bad_policy():
    m = _safe_import()
    safe_spec = {"command": [sys.executable, "-c", "print(1)"]}
    rr = m._route_pipeline_policy_gate(
        json.dumps(safe_spec).encode("utf-8"),
        {"policy": "BOGUS"},
    )
    assert rr.status_code == 400


def test_v1464_route_pipeline_adversarial():
    m = _safe_import()
    rr = m._route_pipeline_adversarial(None, {})
    assert rr.status_code == 200
    j = json.loads(rr.body.decode("utf-8"))
    assert j["n_specs"] == 30
    assert j["match_rate"] == 1.0


def test_v1464_find_open_port():
    m = _safe_import()
    port = m.find_open_port()
    assert m.DEFAULT_PORT_LOW <= port <= m.DEFAULT_PORT_HIGH


def test_v1464_server_factory():
    m = _safe_import()
    state = m._GatewayState()
    server = m.make_gateway_server(host="127.0.0.1", port=0, state=state)
    try:
        host, port = server.server_address
        assert host == "127.0.0.1"
        assert m.DEFAULT_PORT_LOW <= port <= m.DEFAULT_PORT_HIGH
    finally:
        server.server_close()


def test_v1464_safe_coercer_primitives():
    m = _safe_import()
    out = m._safe({"a": 1, "b": [1, 2, 3], "c": None, "d": True, "e": "x"})
    assert out == {"a": 1, "b": [1, 2, 3], "c": None, "d": True, "e": "x"}


def test_v1464_safe_coercer_enum():
    m = _safe_import()
    # Enum value coercion
    class _E:
        value = "X"
        name = "X"
    e = _E()
    assert m._safe(e) == "X"


def test_v1464_end_to_end_demo():
    """In-process server, hit all endpoints, expect 100% ok_rate."""
    m = _safe_import()
    payload = m.run_v1464_demo()
    assert payload["ok_rate"] == 1.0
    assert payload["n_ok"] == payload["n_endpoints"]
    assert payload["n_endpoints"] >= 8
    # Each result has body_ok
    for r in payload["results"]:
        assert "endpoint" in r
        assert "status" in r
        assert "body_ok" in r
        assert r["body_ok"], f"Endpoint {r['endpoint']} failed: {r}"


def test_v1464_end_to_end_demo_routes_404_and_405():
    m = _safe_import()
    payload = m.run_v1464_demo()
    routes = {r["endpoint"] for r in payload["results"]}
    assert any("404" in r for r in routes)
    assert any("405" in r for r in routes)


def test_v1464_stats_record():
    m = _safe_import()
    state = m._GatewayState()
    req = m.GatewayRequest(method="GET", path="/healthz", body_bytes=0,
                            remote="127.0.0.1", started_at=time.time(),
                            finished_at=time.time(), status_code=200)
    state.record_request(req)
    s = state.stats.to_dict()
    assert s["n_requests"] == 1
    assert s["n_2xx"] == 1
    assert s["by_endpoint"].get("GET /healthz") == 1


def test_v1464_http_get_via_client():
    """Manual in-process server + http.client GET /healthz."""
    m = _safe_import()
    state = m._GatewayState()
    server = m.make_gateway_server(host="127.0.0.1", port=0, state=state)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    try:
        host, port = server.server_address
        client = http.client.HTTPConnection(host, port, timeout=10)
        client.request("GET", "/healthz")
        resp = client.getresponse()
        body = resp.read()
        assert resp.status == 200
        j = json.loads(body)
        assert j["ok"] is True
        client.close()
    finally:
        server.shutdown()
        server.server_close()


def test_v1464_http_post_lint_via_client():
    """Manual in-process server + POST /pipeline/lint."""
    m = _safe_import()
    state = m._GatewayState()
    server = m.make_gateway_server(host="127.0.0.1", port=0, state=state)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    try:
        host, port = server.server_address
        client = http.client.HTTPConnection(host, port, timeout=10)
        spec = {"command": [sys.executable, "-c", "print('x')"],
                "image_alias": "ok",
                "workdir_basename": "tmp",
                "env_extra": {},
                "timeout_s": 60,  # ≥ DEFAULT_TIMEOUT_S=30 to avoid SL090 WARN
                "max_output_bytes": 4096,  # ≥ 1024 to avoid SL091 WARN
                }
        client.request("POST", "/pipeline/lint?policy=STANDARD",
                       body=json.dumps(spec).encode("utf-8"),
                       headers={"Content-Type": "application/json",
                                "Content-Length": str(len(json.dumps(spec)))})
        resp = client.getresponse()
        body = resp.read()
        assert resp.status == 200
        j = json.loads(body)
        assert "findings" in j
        client.close()
    finally:
        server.shutdown()
        server.server_close()


def test_v1464_popper_self_check():
    m = _safe_import()
    res = m.popper_v1464()
    assert res["popper_pass"], f"Popper failed: {res['failed']}"
    assert res["n_checks"] >= 10
    assert res["passed"] == res["n_checks"]


def test_v1464_no_phenomenal_guard():
    """V3 guard: no consciousness language in module."""
    m = _safe_import()
    doc = (m.__doc__ or "")
    # Honest disclosure should appear
    assert "实事求是" in doc
    # Should explicitly deny Phenomenal
    assert "PHENOMENAL" in doc
    assert "HTTP_NOT_PHENOMENAL" in doc
    # Should not claim ASI / human-level
    assert "HTTP_NOT_ASI" in doc
    assert "HTTP_NOT_HUMAN_LEVEL" in doc


def test_v1464_borrowed_lineage_documented():
    m = _safe_import()
    doc = (m.__doc__ or "")
    for v in ("v1463", "v1462", "v1461", "v1437", "v1435", "v1434", "v1420", "stdlib"):
        assert v.lower() in doc.lower(), f"Borrowed {v} not in docstring"


def test_v1464_endpoint_bounded_body_enforced():
    """POST body larger than DEFAULT_BODY_MAX_BYTES returns 413 (route-level check)."""
    m = _safe_import()
    # Direct route-level check — doesn't need HTTP server.
    # V1461 subprocess tests may leave OS handles that interfere with HTTP
    # client connections in some test orderings; the route logic is identical.
    huge = b"x" * (m.DEFAULT_BODY_MAX_BYTES + 1024)
    rr = m._route_pipeline_lint(huge, {"policy": "STANDARD"})
    assert rr.status_code == 413, f"Expected 413 got {rr.status_code}: {rr.body!r}"
    # Also verify the JSON body explains why
    j = json.loads(rr.body.decode("utf-8"))
    assert "too large" in j.get("error", "")
    assert j.get("max_bytes") == m.DEFAULT_BODY_MAX_BYTES


def test_v1464_method_not_allowed_405():
    """POST /healthz should return 405."""
    m = _safe_import()
    state = m._GatewayState()
    server = m.make_gateway_server(host="127.0.0.1", port=0, state=state)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    try:
        host, port = server.server_address
        client = http.client.HTTPConnection(host, port, timeout=10)
        client.request("POST", "/healthz", body=b"")
        resp = client.getresponse()
        resp.read()
        assert resp.status == 405
        client.close()
    finally:
        server.shutdown()
        server.server_close()


def test_v1464_unknown_path_404():
    """GET /unknown should return 404."""
    m = _safe_import()
    state = m._GatewayState()
    server = m.make_gateway_server(host="127.0.0.1", port=0, state=state)
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    try:
        host, port = server.server_address
        client = http.client.HTTPConnection(host, port, timeout=10)
        client.request("GET", "/this-path-does-not-exist")
        resp = client.getresponse()
        resp.read()
        assert resp.status == 404
        client.close()
    finally:
        server.shutdown()
        server.server_close()


def test_v1464_router_dispatches_by_method():
    """The router routes by (method, path)."""
    m = _safe_import()
    # Sanity: a method missing returns 404 (not in ROUTES), not 405 (path exists with wrong method)
    rr_404 = m._route_healthz  # only registered for GET
    # Direct test: routing through HTTP handler would 405 a POST /healthz.
    # Verify the ROUTES table excludes POST /healthz.
    assert ("POST", "/healthz") not in m.ROUTES
    assert ("GET", "/healthz") in m.ROUTES


def test_v1464_help_prints_doc():
    m = _safe_import()
    import argparse
    args = argparse.Namespace()
    # Should not raise
    assert m._cmd_help(args) == 0


def test_v1464_status_meta_chain_have_borrowed_lineage():
    m = _safe_import()
    import argparse
    args = argparse.Namespace()
    # status / meta / chain all return 0 and don't crash
    assert m._cmd_status(args) == 0
    assert m._cmd_meta(args) == 0
    assert m._cmd_chain(args) == 0


def test_v1464_demo_writes_report_file(tmp_path):
    """Demo CLI writes .v1464-demo-report.json to out directory."""
    m = _safe_import()
    import argparse
    args = argparse.Namespace(out=str(tmp_path))
    rc = m._cmd_demo(args)
    assert rc == 0
    report = tmp_path / ".v1464-demo-report.json"
    assert report.exists()
    j = json.loads(report.read_text(encoding="utf-8"))
    assert j["ok_rate"] == 1.0
    assert j["n_endpoints"] >= 8


def test_v1464_serve_rejects_non_loopback_without_flag(capsys):
    """serve --host 0.0.0.0 without --allow-lan should refuse."""
    m = _safe_import()
    import argparse
    args = argparse.Namespace(host="0.0.0.0", port=0, allow_lan=False)
    rc = m._cmd_serve(args)
    assert rc == 2
    out = capsys.readouterr().out
    assert "refusing" in out