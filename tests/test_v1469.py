"""Tests for V1469 — ASI Real Two-Process V1468-Generated-Client → V1467-Server Driver.

Run: python -m pytest tests/test_v1469.py -v

Coverage:
  - Module metadata + bounded defaults
  - SubprocessRecord + DriverReport dataclasses
  - _find_open_port: returns free port in [18380, 18480]
  - _kill_subprocess: terminate + wait on real subprocess
  - popper_v1469: in-process self-checks
  - run_v1469_driver: real subprocess boot V1467 + V1468 client → 6 endpoints
  - write_report_json / write_report_markdown
  - V1469 CLI: meta + popper + chain commands
"""

from __future__ import annotations

import dataclasses
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver import (
    V1469_MODULE, V1469_VERSION, V1469_SCHEMA, V1469_DATE,
    V1469_GUARDS, V1469_V3_GUARDS, BORROWED_SOURCES,
    DEFAULT_HOST, DEFAULT_PORT_MIN, DEFAULT_PORT_MAX,
    SubprocessRecord, DriverReport,
    _is_port_free, _find_open_port, _kill_subprocess,
    _promethean_parent_dir, popper_v1469,
    write_report_json, write_report_markdown,
)


# ──────────────────────────────────────────────────────────────────────
# Module metadata + constants
# ──────────────────────────────────────────────────────────────────────


def test_module_metadata():
    assert V1469_MODULE == "v1469_asi_real_two_process_v1468_client_v1467_server_driver"
    assert V1469_VERSION == "0.1.0"
    assert V1469_SCHEMA == "v1469.asi-real-two-process-v1468-client-v1467-server-driver/v1"
    assert V1469_DATE == "2026-08-10"


def test_bounded_defaults():
    assert DEFAULT_HOST == "127.0.0.1"
    assert 18380 <= DEFAULT_PORT_MIN <= DEFAULT_PORT_MAX <= 18500


def test_borrowed_sources_declared():
    assert len(BORROWED_SOURCES) >= 4
    assert "v1468" in BORROWED_SOURCES
    assert "v1467" in BORROWED_SOURCES
    assert "stdlib" in BORROWED_SOURCES


def test_guards_declared():
    assert len(V1469_GUARDS) >= 14
    assert len(V1469_V3_GUARDS) >= 7
    # V3 guards explicit anti-ASI / anti-Phenomenal claims
    assert any("NOT_ASI" in g for g in V1469_V3_GUARDS)
    assert any("NOT_PHENOMENAL" in g for g in V1469_V3_GUARDS)


# ──────────────────────────────────────────────────────────────────────
# SubprocessRecord + DriverReport dataclasses
# ──────────────────────────────────────────────────────────────────────


def test_subprocess_record_dataclass():
    r = SubprocessRecord(
        role="server",
        pid=1234,
        cmd=("python", "-c", "pass"),
        boot_at_s=time.time(),
        exit_code=0,
        elapsed_s=5.0,
        stdout_tail="hi",
        stderr_tail="",
        killed_for_timeout=False,
        timed_out=False,
    )
    d = dataclasses.asdict(r)
    assert d["role"] == "server"
    assert d["pid"] == 1234
    assert d["exit_code"] == 0
    assert list(d["cmd"]) == ["python", "-c", "pass"]
    assert d["elapsed_s"] == 5.0


def test_driver_report_dataclass():
    rep = DriverReport(
        ok=True, verdict="PASS", host="127.0.0.1", port=18380,
        server_pid=100, client_pid=200,
        server_boot_elapsed_s=1.0, client_elapsed_s=4.0, total_elapsed_s=5.0,
        client_path="/tmp/c.py", result_path="/tmp/r.json",
        endpoints_hit=[], n_endpoints_ok=6, n_endpoints_total=6,
        guards=list(V1469_GUARDS), v3_guards=list(V1469_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=14, guards_total=14,
        server_record={"role": "server"}, client_record={"role": "client"},
        errors=[], timestamp=time.time(),
    )
    d = rep.to_dict()
    assert d["ok"] is True
    assert d["verdict"] == "PASS"
    assert d["n_endpoints_ok"] == 6
    assert d["guards_passed"] == 14
    assert "module" in d
    assert "schema" in d


# ──────────────────────────────────────────────────────────────────────
# Port helpers
# ──────────────────────────────────────────────────────────────────────


def test_is_port_free_for_unbound_port():
    # Pick a random high port and check it's free
    assert _is_port_free(DEFAULT_HOST, 39999) is True


def test_find_open_port_returns_in_range():
    port = _find_open_port(DEFAULT_HOST, DEFAULT_PORT_MIN, DEFAULT_PORT_MAX)
    assert DEFAULT_PORT_MIN <= port <= DEFAULT_PORT_MAX


def test_find_open_port_raises_when_all_bound():
    # Bind two sockets in range to exhaust
    s1 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s2 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s1.bind((DEFAULT_HOST, DEFAULT_PORT_MIN))
        s1.listen(1)
        s2.bind((DEFAULT_HOST, DEFAULT_PORT_MIN + 1))
        s2.listen(1)
        with pytest.raises(RuntimeError):
            _find_open_port(DEFAULT_HOST, DEFAULT_PORT_MIN, DEFAULT_PORT_MIN + 1)
    finally:
        s1.close()
        s2.close()


# NOTE: socket import for test_find_open_port_raises_when_all_bound
import socket


# ──────────────────────────────────────────────────────────────────────
# Subprocess helpers
# ──────────────────────────────────────────────────────────────────────


def test_kill_subprocess_on_already_exited():
    """subprocess that already finished: kill is a no-op."""
    proc = subprocess.Popen(
        [sys.executable, "-c", "print('hi')"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    proc.wait(timeout=5)
    # Should not raise even though proc is already exited
    _kill_subprocess(proc, grace_s=1.0)
    assert proc.poll() is not None


def test_kill_subprocess_on_live_proc():
    """subprocess that runs forever: kill terminates it."""
    creationflags = subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0
    proc = subprocess.Popen(
        [sys.executable, "-c", "import time; time.sleep(60)"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=creationflags,
    )
    time.sleep(0.5)
    assert proc.poll() is None  # still running
    _kill_subprocess(proc, grace_s=3.0)
    assert proc.poll() is not None  # terminated


# ──────────────────────────────────────────────────────────────────────
# Popper
# ──────────────────────────────────────────────────────────────────────


def test_popper_v1469_runs_and_passes():
    results = popper_v1469()
    assert len(results) >= 6
    failed = [(n, m) for n, ok, m in results if not ok]
    assert not failed, f"popper failed: {failed}"


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


def test_write_report_json(tmp_path):
    rep = DriverReport(
        ok=True, verdict="PASS", host="127.0.0.1", port=18380,
        server_pid=100, client_pid=200,
        server_boot_elapsed_s=1.0, client_elapsed_s=4.0, total_elapsed_s=5.0,
        client_path="/tmp/c.py", result_path="/tmp/r.json",
        endpoints_hit=[], n_endpoints_ok=6, n_endpoints_total=6,
        guards=list(V1469_GUARDS), v3_guards=list(V1469_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=14, guards_total=14,
        server_record={"role": "server"}, client_record={"role": "client"},
        errors=[], timestamp=time.time(),
    )
    out = tmp_path / "r.json"
    write_report_json(rep, out)
    assert out.exists()
    assert out.stat().st_size > 0
    parsed = json.loads(out.read_text(encoding="utf-8"))
    assert parsed["ok"] is True
    assert parsed["verdict"] == "PASS"


def test_write_report_markdown(tmp_path):
    rep = DriverReport(
        ok=True, verdict="PASS", host="127.0.0.1", port=18380,
        server_pid=100, client_pid=200,
        server_boot_elapsed_s=1.0, client_elapsed_s=4.0, total_elapsed_s=5.0,
        client_path="/tmp/c.py", result_path="/tmp/r.json",
        endpoints_hit=[], n_endpoints_ok=6, n_endpoints_total=6,
        guards=list(V1469_GUARDS), v3_guards=list(V1469_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=14, guards_total=14,
        server_record={"role": "server"}, client_record={"role": "client"},
        errors=[], timestamp=time.time(),
    )
    out = tmp_path / "r.md"
    write_report_markdown(rep, out)
    assert out.exists()
    assert out.stat().st_size > 0
    text = out.read_text(encoding="utf-8")
    assert "V1469" in text
    assert "PASS" in text


# ──────────────────────────────────────────────────────────────────────
# CLI subprocess (real subprocess boot)
# ──────────────────────────────────────────────────────────────────────


def test_cli_meta_via_subprocess(tmp_path):
    """`python -m apeireth.v1469_... meta` exits 0 + prints metadata."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver", "meta"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    meta = json.loads(proc.stdout.decode())
    assert meta["module"] == V1469_MODULE
    assert meta["version"] == V1469_VERSION
    assert len(meta["guards"]) >= 14


def test_cli_popper_via_subprocess(tmp_path):
    """`python -m apeireth.v1469_... popper` exits 0 with all checks passed."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver", "popper"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    # Popper CLI prints one line per check + summary; verify all checks PASS
    out = proc.stdout.decode()
    assert "V1469 popper:" in out
    # All checks should say PASS
    fail_lines = [l for l in out.splitlines() if "[FAIL]" in l]
    assert not fail_lines, f"failed popper checks: {fail_lines}"


def test_cli_chain_via_subprocess(tmp_path):
    """`python -m apeireth.v1469_... chain` verifies V1468 + V1467 importable."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver", "chain"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=30,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    chain = json.loads(proc.stdout.decode())
    assert all(v == "importable" for v in chain["chain"].values()), chain
    assert "v1468" in chain["chain"]
    assert "v1467" in chain["chain"]


def test_cli_run_via_subprocess_real_two_process(tmp_path):
    """`python -m apeireth.v1469_... run --out out.json` boots V1467 + V1468 client driver."""
    out_json = tmp_path / "v1469-driver.json"
    proc = subprocess.run(
        [
            sys.executable, "-m",
            "apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver",
            "run",
            "--host", DEFAULT_HOST,
            "--out", str(out_json),
            "--max-wallclock", "60",
        ],
        cwd=str(ROOT),
        capture_output=True,
        timeout=120,
    )
    assert proc.returncode == 0, f"stderr={proc.stderr.decode()[:500]}"
    assert out_json.exists(), "V1469 driver did not write report"
    parsed = json.loads(out_json.read_text(encoding="utf-8"))
    assert parsed["ok"] is True
    assert parsed["verdict"] == "PASS"
    assert parsed["n_endpoints_ok"] >= 4  # at minimum 4 happy paths; 2 sad paths may 404
    assert parsed["server_pid"] > 0
    assert parsed["client_pid"] > 0
    assert parsed["server_pid"] != parsed["client_pid"]