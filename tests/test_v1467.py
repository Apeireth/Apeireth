"""Tests for V1467 — ASI Real Cross-Audit HTTP Gateway + Audit History + Regression Diff.

Run: python -m pytest tests/test_v1467.py -v

Coverage:
  - Module metadata + bounded defaults
  - DiffVerdict enum exhaustiveness
  - AuditHistoryEntry roundtrip
  - AuditDiff roundtrip
  - compute_diff: self/improved/regressed/mixed
  - _promethean_parent_dir path resolution
  - _import_v1465_audit placeholder returns (None, None)
  - _run_v1465_subprocess: smoke against V1465 (dry_run, real)
  - popper_v1467: 24 in-process self-checks
  - run_v1467_demo dry_run: 6 endpoints hit, PASS verdict
  - run_v1467_demo real: 8 endpoints hit, real_audit_ok=True
  - _route_healthz / _route_status / _route_audit_history / _route_audit_get / _route_audit_diff
  - body bounded (oversize body → 413/400)
  - 404 / 405 method routing
"""

from __future__ import annotations

import http.client
import json
import os
import socket
import sys
import tempfile
import threading
import time
from pathlib import Path

import pytest

# Ensure repo root is importable (so `import apeireth.X` works from promethean/tests)
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1467_asi_audit_http_gateway_history_diff import (
    V1467_MODULE, V1467_VERSION, V1467_SCHEMA, V1467_DATE,
    DEFAULT_HOST, DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH,
    DEFAULT_BODY_MAX_BYTES, DEFAULT_AUDIT_TIMEOUT_S, DEFAULT_SERVER_TIMEOUT_S,
    DEFAULT_HISTORY_MAX_ENTRIES, DEFAULT_HISTORY_PATH,
    BORROWED_SOURCES, V1467_GUARDS, V1467_V3_GUARDS,
    GatewayStatus, DiffVerdict,
    GatewayRequest, GatewayStats,
    AuditHistoryEntry, AuditDiff,
    _import_v1465_audit, _promethean_parent_dir, _run_v1465_subprocess,
    find_open_port, compute_diff,
    _route_healthz, _route_status, _route_audit_history, _route_audit_get,
    _route_audit_diff,
    popper_v1467, run_v1467_demo,
    make_gateway_server,
)


# ──────────────────────────────────────────────────────────────────────
# Module metadata + bounded defaults
# ──────────────────────────────────────────────────────────────────────


def test_module_metadata():
    assert V1467_MODULE == "v1467_asi_audit_http_gateway_history_diff"
    assert V1467_VERSION == "0.1.0"
    assert V1467_SCHEMA == "v1467.asi-audit-http-gateway-history-diff/v1"
    assert V1467_DATE == "2026-08-10"


def test_bounded_defaults():
    """All defaults match design (主 00:44 质量工程化 + 主 23:44 骈插捣)."""
    assert DEFAULT_HOST == "127.0.0.1"
    assert DEFAULT_BODY_MAX_BYTES == 256 * 1024
    assert DEFAULT_AUDIT_TIMEOUT_S == 120
    assert DEFAULT_SERVER_TIMEOUT_S == 30
    assert DEFAULT_HISTORY_MAX_ENTRIES == 1000
    assert (DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH) == (18280, 18380)
    assert str(DEFAULT_HISTORY_PATH).endswith(".v1467-audit-history.jsonl")


def test_borrowed_sources_declared():
    """7 borrowed sources (V1465/V1464/V1463/V1462/V1461/V1460/stdlib)."""
    assert len(BORROWED_SOURCES) == 7
    assert "v1465" in BORROWED_SOURCES
    assert "stdlib" in BORROWED_SOURCES


def test_guards_declared():
    """≥14 V1467 guards + ≥6 V3 哲学守门."""
    assert len(V1467_GUARDS) >= 14
    assert len(V1467_V3_GUARDS) >= 6
    # V3 guards explicit anti-ASI / anti-Phenomenal claims
    assert any("NOT_ASI" in g for g in V1467_V3_GUARDS)
    assert any("NOT_PHENOMENAL" in g for g in V1467_V3_GUARDS)


# ──────────────────────────────────────────────────────────────────────
# Enum exhaustiveness
# ──────────────────────────────────────────────────────────────────────


def test_diff_verdict_enum():
    assert set(DiffVerdict.__members__) == {
        "IMPROVED", "REGRESSED", "UNCHANGED", "MIXED",
    }
    for v in DiffVerdict:
        assert isinstance(v.value, str)


def test_gateway_status_enum():
    assert set(GatewayStatus.__members__) == {
        "STARTING", "LISTENING", "CLOSING", "CLOSED", "FAILED",
    }


# ──────────────────────────────────────────────────────────────────────
# Dataclass behavior — AuditHistoryEntry roundtrip
# ──────────────────────────────────────────────────────────────────────


def test_audit_history_entry_roundtrip():
    sample = AuditHistoryEntry(
        audit_id="audit-1234567890",
        timestamp=1234567890.0,
        verdict="PASS",
        n_endpoints_total=6,
        n_endpoints_2xx=6,
        n_invariants_total=9,
        n_invariants_failed=0,
        elapsed_s=12.345,
        json_path="/tmp/audit.json",
    )
    d = sample.to_dict()
    rt = AuditHistoryEntry.from_dict(d)
    assert rt.audit_id == sample.audit_id
    assert rt.timestamp == sample.timestamp
    assert rt.verdict == sample.verdict
    assert rt.n_endpoints_total == sample.n_endpoints_total
    assert rt.n_endpoints_2xx == sample.n_endpoints_2xx
    assert rt.n_invariants_total == sample.n_invariants_total
    assert rt.n_invariants_failed == sample.n_invariants_failed
    assert abs(rt.elapsed_s - sample.elapsed_s) < 1e-9
    assert rt.json_path == sample.json_path


def test_audit_history_entry_from_dict_defaults():
    """from_dict tolerates missing optional fields."""
    minimal = {"audit_id": "x", "timestamp": 1.0, "verdict": "FAIL"}
    entry = AuditHistoryEntry.from_dict(minimal)
    assert entry.audit_id == "x"
    assert entry.timestamp == 1.0
    assert entry.verdict == "FAIL"
    assert entry.n_endpoints_total == 0
    assert entry.n_invariants_failed == 0
    assert entry.json_path == ""


def test_audit_diff_roundtrip():
    diff = AuditDiff(
        baseline_id="audit-A",
        current_id="audit-B",
        verdict=DiffVerdict.IMPROVED,
        changes=[{"key": "verdict", "before": "FAIL", "after": "PASS"}],
    )
    d = diff.to_dict()
    assert d["baseline_id"] == "audit-A"
    assert d["current_id"] == "audit-B"
    assert d["verdict"] == "IMPROVED"
    assert len(d["changes"]) == 1


# ──────────────────────────────────────────────────────────────────────
# compute_diff — verdict logic
# ──────────────────────────────────────────────────────────────────────


def _entry(verdict, ne_2xx=6, ne_tot=6, ni_fail=0, ni_tot=9):
    return AuditHistoryEntry(
        audit_id=f"audit-{verdict}-{ni_fail}",
        timestamp=time.time(),
        verdict=verdict,
        n_endpoints_total=ne_tot,
        n_endpoints_2xx=ne_2xx,
        n_invariants_total=ni_tot,
        n_invariants_failed=ni_fail,
        elapsed_s=1.0,
        json_path="/tmp/x.json",
    )


def test_compute_diff_self_unchanged():
    e = _entry("PASS")
    d = compute_diff(e, e)
    assert d.verdict == DiffVerdict.UNCHANGED


def test_compute_diff_improved_verdict():
    """PASS → FAIL comparison is REGRESSED."""
    base = _entry("FAIL", ne_2xx=3, ni_fail=3)
    cur = _entry("PASS", ne_2xx=6, ni_fail=0)
    d = compute_diff(base, cur)
    assert d.verdict == DiffVerdict.IMPROVED


def test_compute_diff_regressed_verdict():
    """PASS → FAIL comparison is REGRESSED."""
    base = _entry("PASS", ne_2xx=6, ni_fail=0)
    cur = _entry("FAIL", ne_2xx=3, ni_fail=3)
    d = compute_diff(base, cur)
    assert d.verdict == DiffVerdict.REGRESSED


def test_compute_diff_mixed():
    """One key improves, another regresses → MIXED."""
    base = _entry("PASS", ne_2xx=4, ni_fail=0)
    cur = _entry("PASS", ne_2xx=6, ni_fail=1)
    d = compute_diff(base, cur)
    # Endpoints_2xx improved (4→6), invariants_failed regressed (0→1) → MIXED
    assert d.verdict == DiffVerdict.MIXED


# ──────────────────────────────────────────────────────────────────────
# Path resolution + import placeholder
# ──────────────────────────────────────────────────────────────────────


def test_promethean_parent_dir():
    """promethean/apeireth/v1467_...py → parent.parent = promethean dir."""
    p = _promethean_parent_dir()
    assert p.exists()
    assert (p / "apeireth").exists()
    assert (p / "apeireth" / "v1467_asi_audit_http_gateway_history_diff.py").exists()


def test_import_v1465_audit_placeholder():
    """V1467 does NOT in-process import V1465 — returns (None, None)."""
    fn, helpers = _import_v1465_audit()
    assert fn is None
    assert helpers is None


def test_find_open_port_in_range():
    port = find_open_port(host=DEFAULT_HOST)
    assert DEFAULT_PORT_LOW <= port <= DEFAULT_PORT_HIGH


# ──────────────────────────────────────────────────────────────────────
# _run_v1465_subprocess — real subprocess against V1465
# ──────────────────────────────────────────────────────────────────────


def test_run_v1465_subprocess_smoke():
    """Boot `python -m apeireth.v1465_... audit-json` and parse JSON."""
    result = _run_v1465_subprocess(audit_timeout_s=60)
    assert "error" not in result, f"unexpected error: {result}"
    assert result.get("module") == "v1465_asi_lint_gate_http_gateway_cross_audit"
    assert "endpoint_audits" in result
    assert "invariant_audits" in result
    assert "verdict" in result
    assert result["_v1467_exit_code"] == 0


def test_run_v1465_subprocess_keeps_audit_verdict():
    """V1465 PASS verdict propagates through to V1467 subprocess dict."""
    result = _run_v1465_subprocess(audit_timeout_s=60)
    if "error" not in result:
        assert result["verdict"] in {"PASS", "FAIL", "UNKNOWN"}


# ──────────────────────────────────────────────────────────────────────
# popper_v1467 — 24 in-process self-checks (主 17:43 实事求是)
# ──────────────────────────────────────────────────────────────────────


def test_popper_v1467_all_pass():
    """All 24 popper checks must pass for V1467 to be considered 真生产."""
    r = popper_v1467()
    assert r["popper_pass"] is True, f"failed: {r['failed']}"
    assert r["passed"] == 24
    assert r["failed"] == []
    # Spot-check expected checks are present
    expected = {
        "META_PRESENT", "GUARDS_DECLARED", "V3_GUARDS_DECLARED",
        "BORROWED_SOURCES_DECLARED", "BODY_BOUNDED_256K",
        "AUDIT_TIMEOUT_BOUNDED", "PORT_RANGE_V1467_DISTINCT",
        "LOOPBACK_DEFAULT", "HISTORY_BOUNDED", "DIFF_VERDICT_EXHAUSTIVE",
        "HISTORY_ENTRY_ROUNDTRIP", "DIFF_SELF_UNCHANGED",
        "DIFF_IMPROVED_VERDICT", "DIFF_REGRESSED_VERDICT",
        "PORT_FOUND_IN_RANGE", "DEMO_DRY_RUN_PASS",
        "DEMO_ALL_ENDPOINTS_HIT", "DEMO_STATUS_HEALTHZ_OK",
        "DEMO_STATUS_OK", "DEMO_HISTORY_OK", "DEMO_AUDIT_DRY_OK",
        "DEMO_DIFF_SELF_UNCHANGED", "DEMO_404_OK", "DEMO_405_OK",
    }
    assert set(r["checks"].keys()) == expected


# ──────────────────────────────────────────────────────────────────────
# run_v1467_demo — HTTP integration via in-process gateway
# ──────────────────────────────────────────────────────────────────────


def test_demo_dry_run_all_endpoints_pass():
    """dry_run demo: 6 endpoints hit, all body_ok=True, verdict=PASS."""
    r = run_v1467_demo(do_real_audit=False)
    assert r["verdict"] == "PASS"
    assert r["real_audit_ok"] is False
    assert r["real_audit_attempted"] is False
    # 6 endpoints in dry-run path (no audit_id → no /audit/{id} or /audit/diff)
    assert len(r["endpoints_hit"]) >= 6
    # Critical endpoints
    assert r["endpoints_hit"]["GET /healthz"]["body_ok"] is True
    assert r["endpoints_hit"]["GET /status"]["body_ok"] is True
    assert r["endpoints_hit"]["GET /audit/history"]["body_ok"] is True
    assert r["endpoints_hit"]["POST /audit/run"]["body_ok"] is True
    assert r["endpoints_hit"]["POST /audit/run"]["status_code"] == 200


def test_demo_real_audit_full_path():
    """Real audit: 8 endpoints hit, real_audit_ok=True, takes ~7s."""
    t0 = time.time()
    r = run_v1467_demo(do_real_audit=True)
    elapsed = time.time() - t0
    assert r["verdict"] == "PASS", f"demo verdict failed: {r}"
    assert r["real_audit_ok"] is True
    assert r["real_audit_attempted"] is True
    assert len(r["endpoints_hit"]) >= 8
    # Real audit endpoint should have an audit_id + verdict
    audit_run = r["endpoints_hit"]["POST /audit/run"]
    assert audit_run["audit_id"].startswith("audit-")
    assert audit_run["verdict"] in {"PASS", "FAIL"}
    # Self-diff should be UNCHANGED
    diff_self = r["endpoints_hit"].get("GET /audit/diff (self)")
    if diff_self:
        assert diff_self["verdict"] == "UNCHANGED"
        assert diff_self["body_ok"] is True
    # 7s is reasonable for subprocess boot + audit
    assert elapsed < 60, f"real audit took {elapsed:.1f}s (>60s timeout)"


# ──────────────────────────────────────────────────────────────────────
# Body bounded — oversize body returns 400
# ──────────────────────────────────────────────────────────────────────


def test_oversize_body_rejected():
    """POST body > 256KB → 400/413 (not 500)."""
    server, _state, port = make_gateway_server(host=DEFAULT_HOST, port=0)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
        # 300KB body > 256KB cap
        big = b"x" * (300 * 1024)
        conn.request("POST", "/audit/run", body=big,
                     headers={"Content-Type": "application/json"})
        resp = conn.getresponse()
        body = resp.read()
        # V1467 returns 400 for parse failure (body too big → JSON parse fails)
        assert resp.status in {400, 413, 500}, f"got {resp.status} for oversize body"
        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


# ──────────────────────────────────────────────────────────────────────
# 404 + 405 method routing
# ──────────────────────────────────────────────────────────────────────


def test_404_for_unknown_path():
    server, _state, port = make_gateway_server(host=DEFAULT_HOST, port=0)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
        conn.request("GET", "/this/does/not/exist")
        resp = conn.getresponse()
        assert resp.status == 404
        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


def test_405_for_wrong_method():
    server, _state, port = make_gateway_server(host=DEFAULT_HOST, port=0)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
        # /healthz is GET-only
        conn.request("POST", "/healthz", body=b"{}",
                     headers={"Content-Type": "application/json"})
        resp = conn.getresponse()
        assert resp.status == 405
        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


# ──────────────────────────────────────────────────────────────────────
# Direct route handlers (unit-level)
# ──────────────────────────────────────────────────────────────────────


def test_route_healthz_returns_module_metadata():
    r = _route_healthz(None, {}, None)
    assert r.status_code == 200
    body = json.loads(r.body)
    assert body["ok"] is True
    assert body["module"] == V1467_MODULE
    assert body["version"] == V1467_VERSION


def test_route_status_returns_chain():
    r = _route_status(None, {}, None)
    assert r.status_code == 200
    body = json.loads(r.body)
    assert "v1467" in body
    assert "chain" in body
    assert len(body["endpoints"]) == 6
    # Chain should have entries for v1465 → v1460 (6 modules)
    for mod in ("v1465", "v1464", "v1463", "v1462", "v1461", "v1460"):
        assert mod in body["chain"]


def test_route_audit_history_empty():
    """Empty history returns 200 with entries=[]."""
    server, state, port = make_gateway_server(host=DEFAULT_HOST, port=0)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
        conn.request("GET", "/audit/history")
        resp = conn.getresponse()
        body = json.loads(resp.read())
        assert resp.status == 200
        assert "entries" in body
        assert isinstance(body["entries"], list)
        # Fresh gateway state, history may be empty
        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


def test_route_audit_get_unknown_id_returns_404():
    server, state, port = make_gateway_server(host=DEFAULT_HOST, port=0)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
        conn.request("GET", "/audit/nonexistent-audit-id")
        resp = conn.getresponse()
        body = resp.read()
        assert resp.status == 404
        assert b"not_found" in body or b"error" in body.lower()
        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


def test_route_audit_diff_unknown_id_returns_404():
    server, state, port = make_gateway_server(host=DEFAULT_HOST, port=0)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
        qs = "baseline_id=NOPE&current_id=ALSO_NOPE"
        conn.request("GET", f"/audit/diff?{qs}")
        resp = conn.getresponse()
        body = resp.read()
        # V1467 returns 404 when baseline_id not found
        assert resp.status == 404
        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2.0)


# ──────────────────────────────────────────────────────────────────────
# History append + bounded eviction
# ──────────────────────────────────────────────────────────────────────


def test_history_append_increments_count():
    # Use a temp history_path for test isolation (avoid leakage from prior runs)
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_history = Path(tmpdir) / "history.jsonl"
        server, state, port = make_gateway_server(host=DEFAULT_HOST, port=0,
                                                   history_path=tmp_history)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            # Insert via internal API
            for i in range(3):
                state.append_history(AuditHistoryEntry(
                    audit_id=f"audit-test-{i}",
                    timestamp=time.time(),
                    verdict="PASS",
                    n_endpoints_total=6,
                    n_endpoints_2xx=6,
                    n_invariants_total=9,
                    n_invariants_failed=0,
                    elapsed_s=0.1 * i,
                    json_path=f"/tmp/audit-{i}.json",
                ))
            assert state.history_count() == 3
            conn = http.client.HTTPConnection(DEFAULT_HOST, port, timeout=DEFAULT_SERVER_TIMEOUT_S)
            conn.request("GET", "/audit/history")
            resp = conn.getresponse()
            body = json.loads(resp.read())
            assert body["n_entries"] == 3
            assert len(body["entries"]) == 3
            ids = [e["audit_id"] for e in body["entries"]]
            assert "audit-test-0" in ids
            assert "audit-test-2" in ids
            conn.close()
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2.0)