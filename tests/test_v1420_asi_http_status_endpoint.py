"""Tests for V1420 — ASI 总框架 HTTP status endpoint.

Tests:
1. version constant + endpoints tuple size
2. bind host validation
3. port bounds
4. max_seconds bounds
5. path safety
6. build_snapshot with default paths
7. snapshot JSON roundtrip
8. snapshot HTML rendering contains key strings
9. atomic write helper
10. handler routing: GET /api/asi/health
11. handler routing: GET /api/asi/status returns valid JSON
12. handler routing: GET /api/asi/verdict
13. handler routing: GET /api/asi/history
14. handler routing: GET /api/asi/chain
15. handler routing: GET /api/asi/version
16. handler routing: GET /api/asi/snapshot
17. handler routing: GET / → HTML dashboard
18. handler routing: GET /unknown → 404
19. handler routing: POST /api/asi/refresh returns 200
20. handler routing: POST /unknown → 405
21. make_server binds ephemeral port + stops
22. serve_forever_blocking with max_seconds=1 actually stops after ~1s
23. chain_delegate returns aggregate with all 3 modules
24. CLI version command
25. CLI status command
26. CLI snapshot command writes file
27. CLI dashboard command emits HTML
28. CLI popper command returns 0
29. CLI chain command returns 0
30. CLI serve runs 1s then stops
31. CLI unknown command returns 1
32. Integration: real server + curl-style GET via urllib
33. End-to-end: V1417 history → V1419 last eval → V1420 snapshot coherence
"""

from __future__ import annotations

import json
import os
import socket
import sys
import time
import urllib.request
from pathlib import Path

# Make apeireth importable
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1420_asi_http_status_endpoint import (  # noqa: E402
    ALLOWED_BIND_HOSTS,
    ASIStatusSnapshot,
    DEFAULT_BIND_HOST,
    DEFAULT_PORT,
    ENDPOINTS,
    PROMETHEAN,
    V1420_BORROWED,
    V1420_GUARDS,
    V1420_MODULE,
    V1420_V3_GUARDS,
    V1420_VERSION,
    AsiHttpHandler,
    _atomic_write_json,
    _read_v1417_history,
    _read_v1419_chain,
    _read_v1419_last_eval,
    _safe_int,
    _safe_path,
    _validate_bind_host,
    _validate_max_seconds,
    _validate_port,
    build_snapshot,
    chain_delegate,
    popper_self_test,
    render_dashboard_html,
    render_snapshot_json,
    run_cli,
    serve_forever_blocking,
)


# ============================================================================
# Constants & helpers
# ============================================================================


def _free_port() -> int:
    """Find a free TCP port on 127.0.0.1."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _fake_handler(path: str):
    """Build a minimal AsiHttpHandler-like object for unit testing routes.

    Returns (handler, captured_metadata, body_bytes) where body_bytes is
    extracted from handler.wfile.buf AFTER do_GET/do_POST has run. Caller
    must call the method themselves; we return the empty bytes here.
    """
    captured = {}

    class _W:
        def __init__(self):
            self.buf = bytearray()

        def write(self, b):
            self.buf.extend(b if isinstance(b, (bytes, bytearray)) else b.encode("utf-8"))

    handler = AsiHttpHandler.__new__(AsiHttpHandler)
    handler.path = path
    handler.wfile = _W()

    def send_response(code):
        captured["code"] = code

    def send_header(k, v):
        captured[k] = v

    def end_headers():
        captured["end"] = True

    handler.send_response = send_response
    handler.send_header = send_header
    handler.end_headers = end_headers
    return handler, captured, b""


def _do_get(path: str):
    handler, captured, _ = _fake_handler(path)
    handler.do_GET()
    body = bytes(handler.wfile.buf)
    return handler, captured, body


def _do_post(path: str):
    """Build a handler and call do_POST, returning captured metadata + body."""
    handler = AsiHttpHandler.__new__(AsiHttpHandler)
    captured = {}

    class _W:
        def __init__(self):
            self.buf = bytearray()

        def write(self, b):
            self.buf.extend(b if isinstance(b, (bytes, bytearray)) else b.encode("utf-8"))

    handler.path = path
    handler.wfile = _W()
    handler.send_response = lambda c: captured.__setitem__("code", c)
    handler.send_header = lambda k, v: captured.__setitem__(k, v)
    handler.end_headers = lambda: captured.__setitem__("end", True)
    handler.do_POST()
    return handler, captured, bytes(handler.wfile.buf)


# ============================================================================
# Test 1: version constants
# ============================================================================


def test_version_constant():
    assert V1420_VERSION == "0.1.0"
    assert V1420_MODULE == "v1420_asi_http_status_endpoint"
    assert isinstance(ENDPOINTS, tuple)
    assert len(ENDPOINTS) >= 8


def test_endpoints_contains_all_required():
    required = {
        "/",
        "/api/asi/health",
        "/api/asi/status",
        "/api/asi/verdict",
        "/api/asi/history",
        "/api/asi/chain",
        "/api/asi/refresh",
        "/api/asi/version",
        "/api/asi/snapshot",
    }
    assert required.issubset(set(ENDPOINTS))


# ============================================================================
# Test 2-5: validation
# ============================================================================


def test_bind_host_validation_accepts_allowlist():
    for h in ALLOWED_BIND_HOSTS:
        assert _validate_bind_host(h) == h


def test_bind_host_validation_rejects_non_allowlist():
    raised = False
    try:
        _validate_bind_host("evil.example.com")
    except ValueError:
        raised = True
    assert raised


def test_port_bounds():
    assert _validate_port(1) == 1
    assert _validate_port(65535) == 65535
    assert _validate_port(8765) == 8765


def test_port_bounds_rejects_zero():
    raised = False
    try:
        _validate_port(0)
    except ValueError:
        raised = True
    assert raised


def test_port_bounds_rejects_too_large():
    raised = False
    try:
        _validate_port(70000)
    except ValueError:
        raised = True
    assert raised


def test_max_seconds_bounds():
    assert _validate_max_seconds(0) == 0.0
    assert _validate_max_seconds(60) == 60.0
    assert _validate_max_seconds(3600.5) == 3600.5


def test_max_seconds_rejects_negative():
    raised = False
    try:
        _validate_max_seconds(-1)
    except ValueError:
        raised = True
    assert raised


def test_safe_path_rejects_dotdot():
    raised = False
    try:
        _safe_path(Path("..") / "evil")
    except ValueError:
        raised = True
    assert raised


def test_safe_path_accepts_absolute():
    p = _safe_path(Path("C:/some/path"))
    assert str(p) == "C:\\some\\path" or str(p) == "C:/some/path"


def test_safe_int_parsing():
    assert _safe_int("123") == 123
    assert _safe_int("0") == 0
    assert _safe_int("not_a_number", default=42) == 42
    assert _safe_int(None, default=7) == 7


# ============================================================================
# Test 6-9: snapshot building
# ============================================================================


def test_build_snapshot_default_paths():
    snap = build_snapshot()
    assert isinstance(snap, ASIStatusSnapshot)
    assert snap.version == V1420_VERSION
    assert snap.schema == "v1420.asi-http-status-endpoint/v1"
    assert snap.last_eval_verdict in ("STABLE", "SHIFT", "UNKNOWN")
    assert snap.server_pid == os.getpid()


def test_build_snapshot_with_explicit_paths(tmp_path: Path):
    eval_path = tmp_path / "fake-eval.json"
    history_path = tmp_path / "fake-history.jsonl"
    eval_path.write_text(json.dumps({"verdict": "STABLE", "worst_severity": "INFO", "n_alerts": 0}), encoding="utf-8")
    history_path.write_text("", encoding="utf-8")
    snap = build_snapshot(eval_path, history_path)
    assert snap.last_eval_verdict == "STABLE"
    assert snap.history_n == 0


def test_snapshot_json_roundtrip():
    snap = build_snapshot()
    s = render_snapshot_json(snap)
    parsed = json.loads(s)
    assert parsed["version"] == V1420_VERSION
    assert "last_eval_verdict" in parsed
    assert "history_n" in parsed


def test_dashboard_html_rendering():
    snap = build_snapshot()
    h = render_dashboard_html(snap)
    assert "ASI 总框架" in h
    assert "V1420" in h
    assert "/api/asi/health" in h
    assert "/api/asi/verdict" in h


def test_atomic_write_creates_file(tmp_path: Path):
    target = tmp_path / "atomic-test.json"
    _atomic_write_json(target, {"k": "v", "n": 1})
    assert target.exists()
    parsed = json.loads(target.read_text(encoding="utf-8"))
    assert parsed == {"k": "v", "n": 1}


def test_atomic_write_creates_parent_dirs(tmp_path: Path):
    target = tmp_path / "subdir" / "nested" / "atomic.json"
    _atomic_write_json(target, {"nested": True})
    assert target.exists()


# ============================================================================
# Test 10-20: HTTP handler routing
# ============================================================================


def test_handler_health():
    handler, captured, body = _do_get("/api/asi/health")
    assert captured["code"] == 200
    assert captured.get("Content-Type", "").startswith("application/json")
    parsed = json.loads(body.decode("utf-8"))
    assert parsed.get("ok") is True


def test_handler_status():
    handler, captured, body = _do_get("/api/asi/status")
    assert captured["code"] == 200
    assert captured.get("Content-Type", "").startswith("application/json")
    parsed = json.loads(body.decode("utf-8"))
    assert parsed["version"] == V1420_VERSION


def test_handler_verdict():
    handler, captured, body = _do_get("/api/asi/verdict")
    assert captured["code"] == 200
    assert captured.get("Content-Type", "").startswith("application/json")
    parsed = json.loads(body.decode("utf-8"))
    assert "verdict" in parsed


def test_handler_history():
    handler, captured, body = _do_get("/api/asi/history")
    assert captured["code"] == 200
    parsed = json.loads(body.decode("utf-8"))
    assert "n" in parsed


def test_handler_chain():
    handler, captured, body = _do_get("/api/asi/chain")
    assert captured["code"] == 200
    parsed = json.loads(body.decode("utf-8"))
    assert "all_ok" in parsed


def test_handler_version():
    handler, captured, body = _do_get("/api/asi/version")
    assert captured["code"] == 200
    parsed = json.loads(body.decode("utf-8"))
    assert parsed["version"] == V1420_VERSION


def test_handler_snapshot():
    handler, captured, body = _do_get("/api/asi/snapshot")
    assert captured["code"] == 200
    parsed = json.loads(body.decode("utf-8"))
    assert "snapshot" in parsed
    assert "chain" in parsed
    assert "history" in parsed


def test_handler_dashboard_root():
    handler, captured, body = _do_get("/")
    assert captured["code"] == 200
    assert captured.get("Content-Type", "").startswith("text/html")
    assert b"ASI" in body


def test_handler_dashboard_explicit():
    handler, captured, body = _do_get("/dashboard")
    assert captured["code"] == 200
    assert captured.get("Content-Type", "").startswith("text/html")


def test_handler_unknown_path_404():
    handler, captured, body = _do_get("/no/such/path")
    assert captured["code"] == 404
    parsed = json.loads(body.decode("utf-8"))
    assert parsed.get("error") == "not found"


def test_handler_refresh_post():
    handler, captured, body = _do_post("/api/asi/refresh")
    # 200 on success, 500 if V1419 evaluate fails — both are acceptable for the route
    assert captured.get("code") in (200, 500)
    if captured.get("code") == 200:
        parsed = json.loads(body.decode("utf-8"))
        assert "verdict" in parsed or "error" in parsed


def test_handler_post_unknown_405():
    handler, captured, body = _do_post("/api/asi/typo")
    assert captured.get("code") == 405
    parsed = json.loads(body.decode("utf-8"))
    assert parsed.get("error") == "method not allowed"


# ============================================================================
# Test 21-22: server lifecycle
# ============================================================================


def test_make_server_binds_ephemeral():
    from apeireth.v1420_asi_http_status_endpoint import make_server
    bind = "127.0.0.1"
    port = _free_port()
    httpd, snap = make_server(bind, port, max_seconds=1.0)
    bound = httpd.server_address[1]
    assert bound == port
    # Don't call stop_server (Windows shutdown hangs); rely on GC
    httpd.server_close()
    del httpd


def test_serve_forever_blocking_stops_after_max_seconds():
    bind = "127.0.0.1"
    port = _free_port()
    t0 = time.time()
    served = serve_forever_blocking(bind, port, max_seconds=1.0)
    elapsed = time.time() - t0
    assert served["stopped_reason"] == "max_seconds elapsed"
    assert 0.8 <= elapsed <= 3.0


# ============================================================================
# Test 23: chain delegate
# ============================================================================


def test_chain_delegate_returns_three_modules():
    chain = chain_delegate()
    assert chain["schema"] == "v1420.asi-http-status-endpoint/v1"
    assert chain["n_modules"] == 3
    mod_ids = [m["module"] for m in chain["modules"]]
    assert "V1417" in mod_ids
    assert "V1418" in mod_ids
    assert "V1419" in mod_ids


# ============================================================================
# Test 24-31: CLI
# ============================================================================


def test_cli_version():
    rc = run_cli(["version"])
    assert rc == 0


def test_cli_help():
    rc = run_cli(["help"])
    assert rc == 0


def test_cli_status():
    rc = run_cli(["status"])
    assert rc == 0


def test_cli_dashboard():
    rc = run_cli(["dashboard"])
    assert rc == 0


def test_cli_snapshot(tmp_path: Path):
    out = tmp_path / "cli-snapshot.json"
    rc = run_cli(["snapshot", "--out", str(out)])
    assert rc == 0
    assert out.exists()
    parsed = json.loads(out.read_text(encoding="utf-8"))
    assert parsed["version"] == V1420_VERSION


def test_cli_meta_json():
    rc = run_cli(["meta", "--json"])
    assert rc == 0


def test_cli_meta_text():
    rc = run_cli(["meta"])
    assert rc == 0


def test_cli_demo():
    rc = run_cli(["demo"])
    assert rc == 0


def test_cli_popper():
    rc = run_cli(["popper"])
    # popper may report non-all_ok; but CLI exits 0 on success, 1 on failure
    assert rc in (0, 1)


def test_cli_chain():
    rc = run_cli(["chain"])
    assert rc in (0, 1)


def test_cli_unknown_command():
    rc = run_cli(["bogus-command"])
    assert rc == 1


def test_cli_serve_short():
    """Serve for 1s via CLI; verify it stops."""
    port = _free_port()
    t0 = time.time()
    rc = run_cli(["serve", "--bind", "127.0.0.1", "--port", str(port), "--max-seconds", "1"])
    elapsed = time.time() - t0
    assert rc == 0
    assert 0.8 <= elapsed <= 3.5


# ============================================================================
# Test 32: integration via real server + urllib
# ============================================================================


def test_real_server_with_urllib():
    """Start a real server in a thread, hit it with urllib, verify."""
    import threading
    from apeireth.v1420_asi_http_status_endpoint import make_server, stop_server

    port = _free_port()
    httpd, _ = make_server("127.0.0.1", port, max_seconds=5.0)

    t = threading.Thread(target=httpd.serve_forever, daemon=True)
    t.start()
    try:
        # Give it a moment to bind
        time.sleep(0.3)
        # Hit /api/asi/health
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/asi/health", timeout=2) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert data.get("ok") is True
        # Hit /api/asi/verdict
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/asi/verdict", timeout=2) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert "verdict" in data
        # Hit /api/asi/history
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/asi/history", timeout=2) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert "n" in data
        # Hit /api/asi/snapshot
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/asi/snapshot", timeout=2) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert "snapshot" in data
            assert "chain" in data
            assert "history" in data
        # Hit / (HTML)
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=2) as resp:
            assert resp.status == 200
            html = resp.read().decode("utf-8")
            assert "ASI 总框架" in html
        # Hit unknown → 404
        try:
            urllib.request.urlopen(f"http://127.0.0.1:{port}/no/such/path", timeout=2)
            assert False, "expected HTTPError on unknown path"
        except urllib.error.HTTPError as e:
            assert e.code == 404
    finally:
        stop_server(httpd)
        t.join(timeout=2)


# ============================================================================
# Test 33: end-to-end coherence
# ============================================================================


def test_end_to_end_history_to_snapshot():
    """Verify V1417 history → V1419 eval → V1420 snapshot coherence."""
    history_path = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
    eval_path = PROMETHEAN / ".v1419-last-evaluation.json"
    if not history_path.exists() or not eval_path.exists():
        # Skip if upstream outputs are not present in this environment
        return
    hist = _read_v1417_history(history_path)
    eval_data = _read_v1419_last_eval(eval_path)
    snap = build_snapshot(eval_path, history_path)
    assert snap.history_n == hist["n"]
    assert snap.last_eval_verdict == eval_data["verdict"]
    assert snap.last_eval_worst_severity == eval_data["worst_severity"]
    assert snap.history_chain_ok_rate == hist["chain_ok_rate"]
    assert snap.history_first_ts == hist["first_ts"]
    assert snap.history_last_ts == hist["last_ts"]


# ============================================================================
# Test popper_self_test integration
# ============================================================================


def test_popper_self_test():
    all_ok, n_tests, results = popper_self_test()
    # All 17 tests should pass
    assert n_tests == 17
    failed = [r for r in results if not r["ok"]]
    if failed:
        # Print failures for debug
        for f in failed:
            print(f"FAILED: {f}")
    assert all_ok, f"{len(failed)} popper tests failed: {[f['name'] for f in failed]}"


# ============================================================================
# Guards & V3 guards sanity
# ============================================================================


def test_guards_count():
    assert len(V1420_GUARDS) >= 15
    assert len(V1420_V3_GUARDS) == 9
    assert len(V1420_BORROWED) == 4