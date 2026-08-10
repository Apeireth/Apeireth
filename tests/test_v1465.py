"""Tests for V1465 — ASI Real Lint-Gate HTTP Gateway Cross-Module Live Audit.

Run: python -m pytest tests/test_v1465.py -v
"""

from __future__ import annotations

import json
import socket
import subprocess
import sys
import tempfile
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import pytest

# Ensure repo root is importable
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import (
    V1465_MODULE, V1465_VERSION, V1465_SCHEMA, V1465_DATE,
    DEFAULT_AUDIT_HOST, DEFAULT_AUDIT_WALLCLOCK_S,
    DEFAULT_AUDIT_BODY_MAX_BYTES,
    V1464_ENDPOINTS, CROSS_MODULE_INVARIANTS, INVARIANT_CHECKS,
    AuditOutcome, AuditVerdict,
    EndpointAudit, InvariantAudit, ServerBoot, CrossAuditReport,
    _audit_endpoint, _audit_oversize_body, _audit_invariants,
    _find_open_port, _wait_for_port, _boot_v1464_server, _stop_v1464_server,
    _make_test_specs, _summarize, _verdict,
    write_report_json, write_report_markdown,
    popper_v1465, run_v1465_audit,
)


# ──────────────────────────────────────────────────────────────────────
# Module metadata
# ──────────────────────────────────────────────────────────────────────


def test_module_metadata():
    assert V1465_MODULE == "v1465_asi_lint_gate_http_gateway_cross_audit"
    assert V1465_VERSION == "0.1.0"
    assert V1465_SCHEMA == "v1465.asi-lint-gate-http-gateway-cross-audit/v1"
    assert V1465_DATE == "2026-08-10"


def test_bounded_defaults():
    assert DEFAULT_AUDIT_HOST == "127.0.0.1"
    assert DEFAULT_AUDIT_WALLCLOCK_S == 30
    assert DEFAULT_AUDIT_BODY_MAX_BYTES == 256 * 1024


def test_v1464_endpoints_declared():
    """V1464 has 6 endpoints; V1465 audits all 6 + 3 sad paths = 9 total."""
    assert len(V1464_ENDPOINTS) == 6
    methods = {m for (m, _p) in V1464_ENDPOINTS}
    paths = {p for (_m, p) in V1464_ENDPOINTS}
    assert methods == {"GET", "POST"}
    assert paths == {"/healthz", "/status", "/pipeline/adversarial",
                     "/pipeline/lint", "/pipeline/run", "/pipeline/policy-gate"}


def test_cross_module_invariants_declared():
    """9 cross-module invariants covering V1460-V1464."""
    assert len(CROSS_MODULE_INVARIANTS) == 9
    module_ids = {mid for (mid, _, _) in CROSS_MODULE_INVARIANTS}
    assert module_ids == {"v1464", "v1463", "v1462", "v1461", "v1460"}


# ──────────────────────────────────────────────────────────────────────
# Dataclass behavior
# ──────────────────────────────────────────────────────────────────────


def test_endpoint_audit_to_dict():
    ea = EndpointAudit(method="GET", path="/x",
                       outcome=AuditOutcome.PASS, status_code=200,
                       body_bytes=42, elapsed_ms=3.5, response_shape_ok=True,
                       expected_keys=["ok"], actual_keys=["ok", "module"])
    d = ea.to_dict()
    assert d["method"] == "GET"
    assert d["path"] == "/x"
    assert d["outcome"] == "PASS"
    assert d["status_code"] == 200
    assert d["body_bytes"] == 42
    assert d["elapsed_ms"] == 3.5
    assert d["response_shape_ok"] is True
    assert d["expected_keys"] == ["ok"]
    assert d["actual_keys"] == ["ok", "module"]


def test_invariant_audit_to_dict():
    ia = InvariantAudit(module_id="v1464", invariant="loopback_default",
                        outcome=AuditOutcome.PASS,
                        expected="127.0.0.1", actual="127.0.0.1")
    d = ia.to_dict()
    assert d["module_id"] == "v1464"
    assert d["invariant"] == "loopback_default"
    assert d["outcome"] == "PASS"
    assert d["expected"] == "127.0.0.1"


def test_server_boot_to_dict():
    sb = ServerBoot(outcome=AuditOutcome.PASS, pid=1234,
                    host="127.0.0.1", port=18080, boot_elapsed_ms=200.5)
    d = sb.to_dict()
    assert d["outcome"] == "PASS"
    assert d["pid"] == 1234
    assert d["port"] == 18080


def test_cross_audit_report_elapsed():
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time(), finished_at=time.time() + 2.5)
    assert rep.elapsed_s == 2.5


def test_cross_audit_report_to_dict():
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time())
    d = rep.to_dict()
    assert d["module"] == "t"
    assert d["version"] == "0"
    assert d["schema"] == "s"
    assert d["verdict"] == AuditVerdict.PASS.value
    assert "server_boot" in d
    assert "endpoint_audits" in d
    assert "invariant_audits" in d
    assert "summary" in d


# ──────────────────────────────────────────────────────────────────────
# Cross-module invariant checks (in-process, fast)
# ──────────────────────────────────────────────────────────────────────


def test_v1464_loopback_default():
    ok, exp, act = INVARIANT_CHECKS["v1464_loopback_default"]()
    assert ok is True
    assert exp == "127.0.0.1"
    assert act == "127.0.0.1"


def test_v1464_body_bounded():
    ok, exp, act = INVARIANT_CHECKS["v1464_body_bounded"]()
    assert ok is True
    assert exp == 256 * 1024
    assert act == 256 * 1024


def test_v1464_n_routes_6():
    ok, exp, act = INVARIANT_CHECKS["v1464_n_routes_6"]()
    assert ok is True
    assert exp == 6
    assert act == 6


def test_v1463_adversarial_30():
    ok, exp, act = INVARIANT_CHECKS["v1463_adversarial_30"]()
    assert ok is True
    assert exp == 30
    assert act == 30


def test_v1463_match_rate_1():
    ok, exp, act = INVARIANT_CHECKS["v1463_match_rate_1"]()
    assert ok is True
    assert exp == 1.0


def test_v1462_n_rules_24():
    ok, exp, act = INVARIANT_CHECKS["v1462_n_rules_24"]()
    assert ok is True
    assert exp == 24
    assert act == 24


def test_v1462_policy_levels_3():
    ok, exp, act = INVARIANT_CHECKS["v1462_policy_levels_3"]()
    assert ok is True
    assert exp == 3
    assert act == 3


def test_v1461_sandbox_modes_9():
    ok, exp, act = INVARIANT_CHECKS["v1461_sandbox_modes_9"]()
    assert ok is True
    assert exp == 9
    assert act == 9


def test_v1460_stages_13():
    ok, exp, act = INVARIANT_CHECKS["v1460_stages_12_or_13"]()
    assert ok is True
    assert exp == "12 or 13"
    assert act == 13


def test_audit_invariants_all_pass():
    audits = _audit_invariants()
    assert len(audits) == 9
    for a in audits:
        assert a.outcome == AuditOutcome.PASS, (
            f"invariant {a.module_id}/{a.invariant} failed: {a.error}")


# ──────────────────────────────────────────────────────────────────────
# HTTP helpers
# ──────────────────────────────────────────────────────────────────────


def test_http_get_bad_port_times_out():
    """Connecting to a port nothing is listening on should fail fast."""
    import http.client
    # Use a port likely to be closed; rely on timeout/connection refused.
    with pytest.raises((ConnectionRefusedError, OSError, http.client.HTTPException)):
        # Port 1 is rarely open; expect refusal
        from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import _http_get
        _http_get("127.0.0.1", 1, "/", timeout_s=1.0)


def test_find_open_port_in_range():
    port = _find_open_port()
    assert 18080 <= port <= 18180


def test_wait_for_port_returns_false_for_dead_port():
    """Wait for port 1 (likely closed) — should return False quickly."""
    # Port 1 should refuse quickly
    start = time.time()
    result = _wait_for_port("127.0.0.1", 1, deadline_s=0.5)
    elapsed = time.time() - start
    assert result is False
    assert elapsed < 1.0


def test_make_test_specs():
    safe, bad, jsonl = _make_test_specs()
    assert "command" in safe
    assert "image_alias" in safe
    assert "workdir_basename" in safe
    assert "env_extra" in safe
    assert safe["timeout_s"] >= 30  # avoid SL090 WARN
    assert safe["max_output_bytes"] >= 1024  # avoid SL091 WARN
    assert bad["command"] == ["bash", "-c", "rm -rf /"]
    # jsonl should parse as 2 lines
    lines = [l for l in jsonl.decode("utf-8").split("\n") if l.strip()]
    assert len(lines) == 2
    for line in lines:
        json.loads(line)  # valid JSON


# ──────────────────────────────────────────────────────────────────────
# Per-endpoint audit helper (in-process via mock server)
# ──────────────────────────────────────────────────────────────────────


def _start_mock_server(handler, host="127.0.0.1"):
    """Start a minimal mock HTTP server with `handler`."""
    import threading
    port = _find_open_port(host=host)
    server = HTTPServer((host, port), handler)
    t = threading.Thread(target=server.serve_forever, daemon=True)
    t.start()
    return server, host, port


class _OKHandler(BaseHTTPRequestHandler):
    def log_message(self, *args, **kwargs): pass

    def do_GET(self): self._ok()
    def do_POST(self): self._ok()

    def _ok(self):
        body = json.dumps({"ok": True, "module": "mock", "version": "0"}).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class _BadHandler(BaseHTTPRequestHandler):
    def log_message(self, *args, **kwargs): pass

    def do_GET(self):
        self.send_response(500)
        self.end_headers()


class _EchoHandler(BaseHTTPRequestHandler):
    def log_message(self, *args, **kwargs): pass

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0") or "0")
        body = self.rfile.read(length) if length > 0 else b""
        out = json.dumps({"echo_len": len(body), "shape_ok": True}).encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)


def test_audit_endpoint_pass():
    server, host, port = _start_mock_server(_OKHandler)
    try:
        audit = _audit_endpoint(host, port, "GET", "/healthz",
                                expected_status=200,
                                expected_keys=["ok", "module"])
        assert audit.outcome == AuditOutcome.PASS
        assert audit.status_code == 200
        assert audit.response_shape_ok is True
    finally:
        server.shutdown(); server.server_close()


def test_audit_endpoint_wrong_status():
    server, host, port = _start_mock_server(_BadHandler)
    try:
        audit = _audit_endpoint(host, port, "GET", "/x",
                                expected_status=200)
        assert audit.outcome == AuditOutcome.FAIL
        assert audit.error and "status mismatch" in audit.error
    finally:
        server.shutdown(); server.server_close()


def test_audit_endpoint_missing_keys():
    server, host, port = _start_mock_server(_OKHandler)
    try:
        audit = _audit_endpoint(host, port, "GET", "/healthz",
                                expected_status=200,
                                expected_keys=["ok", "missing_key"])
        assert audit.outcome == AuditOutcome.FAIL
        assert audit.error and "missing keys" in audit.error
    finally:
        server.shutdown(); server.server_close()


def test_audit_endpoint_post_with_body():
    server, host, port = _start_mock_server(_EchoHandler)
    try:
        audit = _audit_endpoint(host, port, "POST", "/echo",
                                body=b'{"x":1}', expected_status=200,
                                expected_keys=["echo_len", "shape_ok"])
        assert audit.outcome == AuditOutcome.PASS
        assert audit.body_bytes > 0
    finally:
        server.shutdown(); server.server_close()


def test_audit_oversize_body_pass_on_413():
    """When server returns 413, helper records PASS."""
    class _413Handler(BaseHTTPRequestHandler):
        def log_message(self, *args, **kwargs): pass
        def do_POST(self):
            body = json.dumps({"error": "body too large", "max_bytes": 256*1024}).encode("utf-8")
            self.send_response(413)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

    server, host, port = _start_mock_server(_413Handler)
    try:
        audit = _audit_oversize_body(host, port, "/x",
                                     body=b"x" * 300_000)
        assert audit.outcome == AuditOutcome.PASS
        assert audit.status_code == 413
        assert audit.response_shape_ok is True
    finally:
        server.shutdown(); server.server_close()


def test_audit_oversize_body_pass_on_connection_abort():
    """When server closes connection (Windows quirk), helper still PASSes."""
    class _AbortHandler(BaseHTTPRequestHandler):
        def log_message(self, *args, **kwargs): pass
        def do_POST(self):
            # Force close without responding
            self.connection.close()

    server, host, port = _start_mock_server(_AbortHandler)
    try:
        audit = _audit_oversize_body(host, port, "/x",
                                     body=b"x" * 300_000,
                                     timeout_s=2.0)
        assert audit.outcome == AuditOutcome.PASS
        assert audit.status_code == 413
        assert "connection_aborted" in (audit.error or "")
    finally:
        server.shutdown(); server.server_close()


# ──────────────────────────────────────────────────────────────────────
# Summarize + verdict
# ──────────────────────────────────────────────────────────────────────


def test_summarize_empty_report():
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time(), verdict=AuditVerdict.PASS)
    s = _summarize(rep)
    assert s["endpoint_total"] == 0
    assert s["invariant_total"] == 0
    assert s["happy_path_total"] == 6
    assert s["sad_path_total"] == 3


def test_verdict_pass_when_all_pass():
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time(),
                           server_boot=ServerBoot(outcome=AuditOutcome.PASS),
                           verdict=AuditVerdict.PASS)
    rep.endpoint_audits = [EndpointAudit(method="GET", path="/x",
                                          outcome=AuditOutcome.PASS)]
    rep.invariant_audits = [InvariantAudit(module_id="m", invariant="i",
                                            outcome=AuditOutcome.PASS)]
    assert _verdict(rep) == AuditVerdict.PASS


def test_verdict_fail_when_boot_fails():
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time(),
                           server_boot=ServerBoot(outcome=AuditOutcome.FAIL))
    assert _verdict(rep) == AuditVerdict.FAIL


def test_verdict_fail_when_endpoint_fails():
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time(),
                           server_boot=ServerBoot(outcome=AuditOutcome.PASS))
    rep.endpoint_audits = [EndpointAudit(method="GET", path="/x",
                                          outcome=AuditOutcome.FAIL)]
    rep.invariant_audits = [InvariantAudit(module_id="m", invariant="i",
                                            outcome=AuditOutcome.PASS)]
    assert _verdict(rep) == AuditVerdict.FAIL


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


def test_write_report_json_and_md(tmp_path):
    rep = CrossAuditReport(module="t", version="0", schema="s", date="d",
                           started_at=time.time(), finished_at=time.time(),
                           server_boot=ServerBoot(outcome=AuditOutcome.PASS,
                                                  pid=1234, port=18080,
                                                  boot_elapsed_ms=200.0),
                           endpoint_audits=[
                               EndpointAudit(method="GET", path="/x",
                                             outcome=AuditOutcome.PASS,
                                             status_code=200, response_shape_ok=True,
                                             elapsed_ms=1.5, note="test"),
                           ],
                           invariant_audits=[
                               InvariantAudit(module_id="m", invariant="i",
                                              outcome=AuditOutcome.PASS,
                                              expected=1, actual=1),
                           ],
                           verdict=AuditVerdict.PASS,
                           summary={"endpoint_pass": 1, "endpoint_total": 1,
                                    "invariant_pass": 1, "invariant_total": 1,
                                    "endpoint_fail": 0, "endpoint_error": 0,
                                    "invariant_fail": 0, "invariant_error": 0,
                                    "happy_path_total": 6, "sad_path_total": 3})
    json_path = tmp_path / "r.json"
    md_path = tmp_path / "r.md"
    write_report_json(rep, json_path)
    write_report_markdown(rep, md_path)
    assert json_path.exists()
    assert json_path.stat().st_size > 0
    parsed = json.loads(json_path.read_text(encoding="utf-8"))
    assert parsed["verdict"] == "PASS"
    assert parsed["summary"]["endpoint_pass"] == 1
    assert md_path.exists()
    md_text = md_path.read_text(encoding="utf-8")
    assert "V1465 Audit Report" in md_text
    assert "Endpoint Audits" in md_text
    assert "Cross-Module Invariants" in md_text


# ──────────────────────────────────────────────────────────────────────
# Popper self-check
# ──────────────────────────────────────────────────────────────────────


def test_popper_v1465_passes():
    result = popper_v1465()
    assert result["n_checks"] >= 10
    assert result["passed"] == result["n_checks"]
    assert result["popper_pass"] is True
    assert result["failed"] == []


# ──────────────────────────────────────────────────────────────────────
# Full audit (real subprocess + real HTTP) — the headline test
# ──────────────────────────────────────────────────────────────────────


def test_run_v1465_audit_end_to_end():
    """The headline test: real subprocess + real HTTP + real cross-modular audit."""
    report = run_v1465_audit()
    assert report.module == V1465_MODULE
    assert report.verdict == AuditVerdict.PASS
    assert report.server_boot is not None
    assert report.server_boot.outcome == AuditOutcome.PASS
    # 6 happy paths + 3 sad paths = 9 endpoint audits
    assert len(report.endpoint_audits) == 9
    # All endpoints should pass
    for ea in report.endpoint_audits:
        assert ea.outcome == AuditOutcome.PASS, (
            f"{ea.method} {ea.path} failed: {ea.error}")
    # 9 cross-module invariants should all pass
    assert len(report.invariant_audits) == 9
    for ia in report.invariant_audits:
        assert ia.outcome == AuditOutcome.PASS, (
            f"{ia.module_id}/{ia.invariant} failed: {ia.error}")


def test_run_v1465_audit_writes_reports(tmp_path):
    """The audit CLI writes both JSON and MD reports to disk."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["audit", "--out", str(tmp_path)])
    assert rc == 0  # PASS verdict
    jsons = list(tmp_path.glob("v1465-audit-*.json"))
    mds = list(tmp_path.glob("v1465-audit-*.md"))
    assert len(jsons) >= 1
    assert len(mds) >= 1
    latest_json = tmp_path / "v1465-audit-latest.json"
    latest_md = tmp_path / "v1465-audit-latest.md"
    assert latest_json.exists()
    assert latest_md.exists()
    # Parse the latest JSON
    parsed = json.loads(latest_json.read_text(encoding="utf-8"))
    assert parsed["verdict"] == "PASS"
    assert parsed["summary"]["endpoint_pass"] == 9
    assert parsed["summary"]["invariant_pass"] == 9


def test_audit_json_cli_emits_to_stdout(capsys):
    """audit-json CLI prints the full report to stdout."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["audit-json"])
    assert rc == 0
    captured = capsys.readouterr()
    payload = json.loads(captured.out)
    assert payload["module"] == V1465_MODULE
    assert payload["verdict"] == "PASS"
    assert len(payload["endpoint_audits"]) == 9
    assert len(payload["invariant_audits"]) == 9


def test_invariants_cli():
    """invariants CLI prints just the invariant audit (in-process, fast)."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["invariants"])
    assert rc == 0


def test_popper_cli():
    """popper CLI prints popper self-check."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["popper"])
    assert rc == 0


def test_status_cli():
    """status CLI prints module metadata."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["status"])
    assert rc == 0


def test_chain_cli():
    """chain CLI prints borrowed-lineage chain."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["chain"])
    assert rc == 0


def test_meta_cli():
    """meta CLI prints module metadata."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["meta"])
    assert rc == 0


def test_help_cli():
    """help CLI prints module help."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main(["help"])
    assert rc == 0


def test_no_subcommand_prints_help():
    """Running with no subcommand should print help and exit 0."""
    from apeireth.v1465_asi_lint_gate_http_gateway_cross_audit import main as v1465_main
    rc = v1465_main([])
    assert rc == 0


# ──────────────────────────────────────────────────────────────────────
# Cross-modular chain (V1465 imports V1460-V1464 — borrow lineage)
# ──────────────────────────────────────────────────────────────────────


def test_v1465_imports_v1464():
    """V1465 must successfully import V1464 (used by V1464_ENDPOINTS + invariants)."""
    from apeireth.v1464_asi_lint_gate_pipeline_http_gateway import (
        ROUTES as _ROUTES, DEFAULT_HOST as _DEFAULT_HOST,
        DEFAULT_BODY_MAX_BYTES as _DEFAULT_BODY_MAX_BYTES,
    )
    assert len(_ROUTES) == 6
    assert _DEFAULT_HOST == "127.0.0.1"


def test_v1465_imports_v1463():
    from apeireth.v1463_asi_lint_gate_subprocess_pipeline import (
        _ADVERSARIAL_SPECS as _A, run_v1463 as _r,
    )
    assert len(_A) == 30
    assert _r()["match_rate"] == 1.0


def test_v1465_imports_v1462():
    from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (
        _RULES as _R, PolicyLevel as _P,
    )
    assert len(_R) == 24
    assert len(list(_P)) == 3


def test_v1465_imports_v1461():
    from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import (
        SandboxSpec as _S, SandboxMode as _M,
    )
    # SandboxSpec is a dataclass — check fields not raw attributes
    import dataclasses
    field_names = {f.name for f in dataclasses.fields(_S)}
    assert "command" in field_names
    assert len(list(_M)) == 9


def test_v1465_imports_v1460():
    from apeireth.v1460_asi_real_windows_anyone_run_harness import (
        STAGE_NAMES as _S, EXPECTED_N_STAGES as _N,
    )
    assert len(_S) == 13
    assert _N == 13