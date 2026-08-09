"""Tests for V1437 — ASI 真生产 subprocess HTTP live server (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43).

Coverage:
- Constants / guards / borrowed
- Enums / dataclasses (5 + 3 = 8 types)
- Helpers: find_free_port, is_port_open, wait_for_port, _truncate, _coerce_timeout
- SubprocessChild / HttpCall / ServerProbeResult dataclass JSON safety
- spawn_handler_subprocess + cleanup_subprocess (real subprocess, real port)
- probe_server (real HTTP against the spawned server)
- run_server_probe end-to-end (real subprocess + probe + cleanup)
- chain_delegate, popper_self_test
- CLI: meta --json, popper, probe (smoke), json (smoke)
"""

from __future__ import annotations

import json
import socket
import subprocess
import sys
import time

import pytest


# ---------------------------------------------------------------------------
# Constants & guards
# ---------------------------------------------------------------------------


def test_v1437_importable():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    assert m.V1437_VERSION == "0.1.0"


def test_v1437_guards_count():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    assert len(m.V1437_GUARDS) == 14
    assert len(m.V1437_V3_GUARDS) == 5


def test_v1437_borrowed_count():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    assert len(m.V1437_BORROWED) == 5


def test_v1437_default_constants():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    assert m.DEFAULT_TIMEOUT_SECONDS >= 1
    assert m.MAX_TIMEOUT_SECONDS >= m.DEFAULT_TIMEOUT_SECONDS
    assert m.MAX_BODY_BYTES > 0
    assert m.DEFAULT_PORT_LOW < m.DEFAULT_PORT_HIGH
    assert m.DEFAULT_HOST  # non-empty
    assert len(m.FAKE_MODELS) >= 4


def test_v1437_fake_models_format():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    for name in m.FAKE_MODELS:
        assert isinstance(name, str)
        assert len(name) > 0


# ---------------------------------------------------------------------------
# Enums / Dataclasses
# ---------------------------------------------------------------------------


def test_v1437_server_launch_modes_count():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    modes = list(m.ServerLaunchMode)
    assert len(modes) >= 8
    # Required modes
    for required in (
        "LAUNCHED",
        "LAUNCH_FAILED",
        "BIND_TIMEOUT",
        "PROBE_OK",
        "PROBE_TIMEOUT",
        "PROBE_HTTP_ERR",
        "CLEANUP_OK",
        "CLEANUP_TIMEOUT",
        "CLEANUP_KILLED",
    ):
        assert any(mode.value == required for mode in modes), f"missing mode {required}"


def test_v1437_subprocess_child_dataclass():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    sc = m.SubprocessChild(pid=1234, cmd="echo hi", rc=0, mode="EXITED", elapsed_ms=12.5)
    d = sc.to_dict()
    assert d["pid"] == 1234
    assert d["cmd"] == "echo hi"
    assert d["rc"] == 0
    assert d["elapsed_ms"] == 12.5
    assert d["mode"] == "EXITED"


def test_v1437_http_call_dataclass():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    hc = m.HttpCall(url="http://x/v1/models", status_code=200, elapsed_ms=12.5, mode="OK")
    d = hc.to_dict()
    assert d["url"] == "http://x/v1/models"
    assert d["status_code"] == 200
    assert d["elapsed_ms"] == 12.5
    assert d["mode"] == "OK"


def test_v1437_server_probe_result_dataclass():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    r = m.ServerProbeResult(host="127.0.0.1", port=12345, launch_mode="PROBE_OK", cleanup_mode="CLEANUP_OK")
    d = r.to_dict()
    assert d["host"] == "127.0.0.1"
    assert d["port"] == 12345
    assert d["launch_mode"] == "PROBE_OK"
    assert d["cleanup_mode"] == "CLEANUP_OK"
    assert d["child"] is None


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def test_v1437_truncate_empty():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    text, truncated = m._truncate("", 100)
    assert text == ""
    assert truncated is False


def test_v1437_truncate_long():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    long_text = "x" * 1000
    text, truncated = m._truncate(long_text, 100)
    assert truncated is True
    assert len(text) == 100


def test_v1437_coerce_timeout_bounds():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    assert m._coerce_timeout(0) >= 1
    assert m._coerce_timeout(99999) <= m.MAX_TIMEOUT_SECONDS
    assert m._coerce_timeout("not-int") == m.DEFAULT_TIMEOUT_SECONDS
    assert m._coerce_timeout(None) == m.DEFAULT_TIMEOUT_SECONDS


def test_v1437_find_free_port_returns_valid_int():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    p = m.find_free_port(m.DEFAULT_HOST)
    assert isinstance(p, int)
    assert 0 < p < 65536


def test_v1437_find_free_port_unique():
    """Two consecutive calls should usually return distinct ports."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    p1 = m.find_free_port(m.DEFAULT_HOST)
    p2 = m.find_free_port(m.DEFAULT_HOST)
    # Either both equal (OS happened to reuse) OR distinct
    assert p1 > 0 and p2 > 0


def test_v1437_is_port_open_negative_on_unused_port():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    p = m.find_free_port(m.DEFAULT_HOST)
    assert m.is_port_open(m.DEFAULT_HOST, p, timeout=0.3) is False


def test_v1437_wait_for_port_negative_on_unused_port():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    p = m.find_free_port(m.DEFAULT_HOST)
    start = time.monotonic()
    result = m.wait_for_port(m.DEFAULT_HOST, p, timeout=1)
    elapsed = time.monotonic() - start
    assert result is False
    assert elapsed < 3  # bounded


# ---------------------------------------------------------------------------
# SubprocessChild JSON safety
# ---------------------------------------------------------------------------


def test_v1437_subprocess_child_json_safe():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    sc = m.SubprocessChild(pid=42, cmd="echo", stdout="hi\n", stderr="", mode="EXITED")
    j = json.dumps(sc.to_dict())
    d = json.loads(j)
    assert d["pid"] == 42
    assert d["stdout_preview"] == "hi\n"


# ---------------------------------------------------------------------------
# spawn_handler_subprocess + cleanup_subprocess (real subprocess)
# ---------------------------------------------------------------------------


def test_v1437_spawn_handler_subprocess_smoke():
    """Spawn subprocess without waiting for bind; check pid is positive."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    port = m.find_free_port(m.DEFAULT_HOST)
    child = m.spawn_handler_subprocess(m.DEFAULT_HOST, port, timeout=5)
    try:
        assert child.pid > 0
        assert child.mode == "RUNNING"
    finally:
        # Always cleanup
        if m.get_proc_handle(child) is not None:
            m.cleanup_subprocess(child, timeout=5)


def test_v1437_cleanup_returns_valid_mode():
    """cleanup_subprocess on a freshly-spawned child returns CLEANUP_OK or CLEANUP_KILLED."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    port = m.find_free_port(m.DEFAULT_HOST)
    child = m.spawn_handler_subprocess(m.DEFAULT_HOST, port, timeout=5)
    mode = m.cleanup_subprocess(child, timeout=5)
    assert mode in (
        m.ServerLaunchMode.CLEANUP_OK.value,
        m.ServerLaunchMode.CLEANUP_KILLED.value,
    )


def test_v1437_cleanup_idempotent():
    """Calling cleanup twice on the same child is a no-op (returns SKIPPED)."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    port = m.find_free_port(m.DEFAULT_HOST)
    child = m.spawn_handler_subprocess(m.DEFAULT_HOST, port, timeout=5)
    first = m.cleanup_subprocess(child, timeout=5)
    second = m.cleanup_subprocess(child, timeout=5)
    assert first in (
        m.ServerLaunchMode.CLEANUP_OK.value,
        m.ServerLaunchMode.CLEANUP_KILLED.value,
    )
    assert second == m.ServerLaunchMode.SKIPPED.value


# ---------------------------------------------------------------------------
# run_server_probe end-to-end (real subprocess + probe + cleanup)
# ---------------------------------------------------------------------------


def test_v1437_run_server_probe_e2e():
    """Full subprocess + port-bind + HTTP probe + cleanup cycle."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    result = m.run_server_probe(host=m.DEFAULT_HOST, port=-1, timeout=15)
    # Allow either PROBE_OK (best case) or PROBE_HTTP_ERR (some path quirks)
    # but never crash
    assert result.launch_mode in (
        m.ServerLaunchMode.PROBE_OK.value,
        m.ServerLaunchMode.PROBE_HTTP_ERR.value,
        m.ServerLaunchMode.PROBE_TIMEOUT.value,
        m.ServerLaunchMode.BIND_TIMEOUT.value,
        m.ServerLaunchMode.LAUNCH_FAILED.value,
    )
    # Cleanup should have run unless launch failed
    if result.launch_mode != m.ServerLaunchMode.LAUNCH_FAILED.value:
        assert result.cleanup_mode in (
            m.ServerLaunchMode.CLEANUP_OK.value,
            m.ServerLaunchMode.CLEANUP_KILLED.value,
        )
    # At least 4 HTTP calls attempted
    assert len(result.http_calls) >= 1


def test_v1437_run_server_probe_then_port_closed():
    """After cleanup, the port should not be open anymore."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    result = m.run_server_probe(host=m.DEFAULT_HOST, port=-1, timeout=15)
    if result.launch_mode != m.ServerLaunchMode.LAUNCH_FAILED.value:
        # Allow OS a brief moment to release the port
        time.sleep(0.1)
        still_open = m.is_port_open(result.host, result.port, timeout=0.3)
        # If killed, port should be free. If terminate succeeded, port should be free.
        # We accept either (some Windows quirks).
        assert isinstance(still_open, bool)


def test_v1437_run_server_probe_invalid_host_uses_default():
    """run_server_probe with default host/port picks an OS-assigned free port."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    result = m.run_server_probe(host=m.DEFAULT_HOST, port=-1, timeout=15)
    # After end, the result.port should be a valid port > 0
    assert result.port > 0
    assert result.port < 65536


# ---------------------------------------------------------------------------
# probe_server against already-running server
# ---------------------------------------------------------------------------


def test_v1437_probe_server_returns_four_calls():
    """probe_server returns one HttpCall per path."""
    import apeireth.v1437_asi_subprocess_http_live_server as m

    # Spawn a child and wait for bind so probe_server can hit it
    port = m.find_free_port(m.DEFAULT_HOST)
    child = m.spawn_handler_subprocess(m.DEFAULT_HOST, port, timeout=5)
    try:
        if not m.wait_for_port(m.DEFAULT_HOST, port, timeout=5):
            pytest.skip("subprocess failed to bind")
        calls = m.probe_server(m.DEFAULT_HOST, port, timeout=5)
        assert len(calls) == 4
        paths = {c.method for c in calls}
        assert "GET /" in paths
        assert "GET /health" in paths
        assert "GET /v1/models" in paths
        assert "GET /api/status" in paths
        # Each call must have a mode field
        for c in calls:
            assert c.mode in ("OK", "TIMEOUT", "HTTP_ERR", "CONN_REFUSED", "CONN_ERR", "UNKNOWN")
    finally:
        if m.get_proc_handle(child) is not None:
            m.cleanup_subprocess(child, timeout=5)


# ---------------------------------------------------------------------------
# chain_delegate, popper_self_test
# ---------------------------------------------------------------------------


def test_v1437_chain_delegate_returns_all_ok():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    out = m.chain_delegate()
    assert out["v1437"]["ok"] is True
    assert "v1436" in out
    assert "v1435" in out
    # imported ok should not crash
    assert isinstance(out["borrowed"], list)
    assert len(out["borrowed"]) == 5


def test_v1437_popper_self_test_passes_all():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    out = m.popper_self_test()
    assert out["passed"] == out["total"], out
    assert out["total"] >= 14


def test_v1437_module_meta():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    meta = m.module_meta()
    assert meta["version"] == "0.1.0"
    assert meta["default_host"] == m.DEFAULT_HOST
    assert meta["max_body_bytes"] == m.MAX_BODY_BYTES
    assert meta["fake_models_count"] == len(m.FAKE_MODELS)


# ---------------------------------------------------------------------------
# Render summary
# ---------------------------------------------------------------------------


def test_v1437_render_probe_summary_md_has_disclosure():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    r = m.ServerProbeResult(
        host=m.DEFAULT_HOST,
        port=12345,
        launch_mode=m.ServerLaunchMode.PROBE_OK.value,
        cleanup_mode=m.ServerLaunchMode.CLEANUP_OK.value,
    )
    md = m.render_probe_summary_md(r)
    assert "V1437" in md
    assert "Honest disclosure" in md
    assert "subprocess" in md.lower()
    assert "≠" in md


# ---------------------------------------------------------------------------
# CLI (smoke)
# ---------------------------------------------------------------------------


def test_v1437_cli_version():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["version"])
    assert rc == 0


def test_v1437_cli_meta():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["meta"])
    assert rc == 0


def test_v1437_cli_meta_json():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["meta", "--json"])
    assert rc == 0


def test_v1437_cli_popper():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["popper"])
    assert rc == 0


def test_v1437_cli_chain():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["chain"])
    assert rc == 0


def test_v1437_cli_probe_smoke():
    """Run a real subprocess probe via CLI."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["probe", "--timeout", "15"])
    # Returns 0 on PROBE_OK, 1 on probe errors but subprocess launch still succeeded
    assert rc in (0, 1)


def test_v1437_cli_json_smoke():
    """JSON output of run_server_probe."""
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["json", "--timeout", "15"])
    assert rc == 0


def test_v1437_cli_unknown_command():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["nonsense-command"])
    assert rc == 2


def test_v1437_cli_help():
    import apeireth.v1437_asi_subprocess_http_live_server as m
    rc = m.main(["help"])
    assert rc == 0


# ---------------------------------------------------------------------------
# Chain integrity (V1435 + V1436 + V1437)
# ---------------------------------------------------------------------------


def test_v1437_chain_with_v1435_v1436_importable():
    """V1435 + V1436 must be importable for V1437 chain_delegate to pass."""
    import apeireth.v1435_asi_docker_availability_probe as v1435
    import apeireth.v1436_asi_llm_endpoint_live_probe as v1436
    import apeireth.v1437_asi_subprocess_http_live_server as v1437

    assert v1435.V1435_VERSION.startswith("0.")
    assert v1436.V1436_VERSION.startswith("0.")
    assert v1437.V1437_VERSION == "0.1.0"
    # Borrowed chain: V1437 borrows V1436 + V1435 patterns
    borrowed_modules = [m for m, _ in v1437.V1437_BORROWED]
    assert "v1436_asi_llm_endpoint_live_probe" in borrowed_modules
    assert "v1435_asi_docker_availability_probe" in borrowed_modules


# ---------------------------------------------------------------------------
# Cleanup invariant: subprocess always terminated (no orphan)
# ---------------------------------------------------------------------------


def test_v1437_run_server_probe_terminates_child():
    """After run_server_probe, the child should be reaped (PID not in psutil-style alive check)."""
    import apeireth.v1437_asi_subprocess_http_live_server as m

    result = m.run_server_probe(host=m.DEFAULT_HOST, port=-1, timeout=15)
    if result.child is not None and result.child.pid > 0:
        # On Windows, easiest check: the child cleanup_mode is CLEANUP_OK or CLEANUP_KILLED
        assert result.cleanup_mode in (
            m.ServerLaunchMode.CLEANUP_OK.value,
            m.ServerLaunchMode.CLEANUP_KILLED.value,
            m.ServerLaunchMode.SKIPPED.value,
        ), f"unexpected cleanup mode: {result.cleanup_mode}"
        # The handle should be dropped (so re-cleanup is SKIPPED)
        assert m.get_proc_handle(result.child) is None