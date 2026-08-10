"""Tests for V1471 — ASI Real Persistent V1467 Audit Monitor Daemon.

Run: python -m pytest tests/test_v1471.py -v

Coverage:
  - Module metadata + bounded defaults
  - DiffEvent + AuditSeen + DaemonReport dataclasses
  - ShutdownReason enum (6 reasons)
  - _find_open_port / _kill_subprocess helpers
  - popper_v1471: 18 in-process self-checks
  - write_report_json / write_report_markdown
  - V1471AuditMonitorDaemon: real subprocess boot + poll + diff + graceful shutdown
  - V1471 CLI: meta + popper + chain commands
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

from apeireth.v1471_audit_monitor_daemon import (
    V1471_MODULE, V1471_VERSION, V1471_SCHEMA, V1471_DATE,
    V1471_GUARDS, V1471_V3_GUARDS, BORROWED_SOURCES,
    DEFAULT_HOST, DEFAULT_PORT_MIN, DEFAULT_PORT_MAX,
    DEFAULT_POLL_INTERVAL_S, MIN_POLL_INTERVAL_S, MAX_POLL_INTERVAL_S,
    DEFAULT_MAX_RUNTIME_S, DEFAULT_MAX_POLLS, MIN_MAX_POLLS, MAX_MAX_POLLS,
    DEFAULT_MAX_LOG_BYTES, DEFAULT_HISTORY_LIMIT,
    ShutdownReason,
    DiffEvent, AuditSeen, DaemonReport,
    V1471AuditMonitorDaemon,
    _is_port_free, _find_open_port, _kill_subprocess,
    _promethean_parent_dir,
    popper_v1471,
    write_report_json, write_report_markdown,
)


# ──────────────────────────────────────────────────────────────────────
# Module metadata + constants
# ──────────────────────────────────────────────────────────────────────


def test_module_metadata():
    assert V1471_MODULE == "v1471_audit_monitor_daemon"
    assert V1471_VERSION == "0.1.0"
    assert V1471_SCHEMA == "v1471.asi-real-persistent-audit-monitor-daemon/v1"
    assert V1471_DATE == "2026-08-10"


def test_bounded_defaults():
    assert DEFAULT_HOST == "127.0.0.1"
    assert DEFAULT_PORT_MIN < DEFAULT_PORT_MAX
    assert DEFAULT_PORT_MIN == 18580  # distinct from V1470's 18380-18480
    assert MIN_POLL_INTERVAL_S <= DEFAULT_POLL_INTERVAL_S <= MAX_POLL_INTERVAL_S
    assert MIN_MAX_POLLS <= DEFAULT_MAX_POLLS <= MAX_MAX_POLLS
    assert 0 < DEFAULT_MAX_RUNTIME_S <= 600.0
    assert DEFAULT_MAX_LOG_BYTES >= 4096
    assert DEFAULT_HISTORY_LIMIT >= 1


def test_borrowed_sources_declared():
    assert len(BORROWED_SOURCES) >= 4
    assert "v1467" in BORROWED_SOURCES
    assert "stdlib" in BORROWED_SOURCES


def test_guards_declared():
    assert len(V1471_GUARDS) >= 14
    assert len(V1471_V3_GUARDS) >= 7
    assert any("NOT_ASI" in g for g in V1471_V3_GUARDS)
    assert any("NOT_PHENOMENAL" in g for g in V1471_V3_GUARDS)


def test_shutdown_reason_enum():
    assert len(ShutdownReason) == 6
    values = {r.value for r in ShutdownReason}
    assert "RUNTIME_LIMIT" in values
    assert "POLL_LIMIT" in values
    assert "KEYBOARD_INTERRUPT" in values
    assert "V1467_DIED" in values
    assert "ERROR" in values
    assert "NORMAL_EXIT" in values


# ──────────────────────────────────────────────────────────────────────
# Dataclasses
# ──────────────────────────────────────────────────────────────────────


def test_diff_event_dataclass():
    e = DiffEvent(
        event_type="diff", timestamp=1700000000.0, poll_index=3,
        baseline_id="audit-a", current_id="audit-b",
        diff_verdict="IMPROVED", n_changes=2, elapsed_s=0.5,
    )
    d = e.to_dict()
    assert d["event_type"] == "diff"
    assert d["baseline_id"] == "audit-a"
    assert d["current_id"] == "audit-b"
    assert d["diff_verdict"] == "IMPROVED"
    assert d["n_changes"] == 2
    assert d["poll_index"] == 3


def test_diff_event_jsonl():
    e = DiffEvent(
        event_type="diff", timestamp=1700000000.0, poll_index=1,
        baseline_id="a", current_id="b", diff_verdict="UNCHANGED",
        n_changes=0, elapsed_s=0.1,
    )
    line = e.to_jsonl()
    parsed = json.loads(line)
    assert parsed["baseline_id"] == "a"
    assert parsed["current_id"] == "b"


def test_audit_seen_dataclass():
    a = AuditSeen(audit_id="abc", timestamp=1.0, seen_at_poll=1, seen_at_real_s=time.time())
    d = dataclasses.asdict(a)
    assert d["audit_id"] == "abc"
    assert d["seen_at_poll"] == 1


def test_daemon_report_dataclass():
    r = DaemonReport(
        ok=True, verdict="PASS", host="127.0.0.1", port=18580, v1467_pid=1000,
        shutdown_reason="RUNTIME_LIMIT", n_polls=5, n_audits_seen=3, n_diff_events_emitted=2,
        n_errors=0, total_elapsed_s=10.0, diff_events=[], errors=[],
        guards=list(V1471_GUARDS), v3_guards=list(V1471_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=14, guards_total=14, timestamp=time.time(),
        log_path="/tmp/log", jsonl_stream_path="/tmp/stream.jsonl",
        daemon_report_path="/tmp/r.json", daemon_md_path="/tmp/r.md",
        out_dir="/tmp/out",
    )
    d = r.to_dict()
    assert d["ok"] is True
    assert d["n_polls"] == 5
    assert d["module"] == V1471_MODULE
    assert d["shutdown_reason"] == "RUNTIME_LIMIT"


# ──────────────────────────────────────────────────────────────────────
# Helpers
# ──────────────────────────────────────────────────────────────────────


def test_is_port_free_for_unbound_port():
    assert _is_port_free(DEFAULT_HOST, 39999) is True


def test_find_open_port_returns_in_range():
    port = _find_open_port(DEFAULT_HOST, DEFAULT_PORT_MIN, DEFAULT_PORT_MAX)
    assert DEFAULT_PORT_MIN <= port <= DEFAULT_PORT_MAX


def test_kill_subprocess_on_already_exited():
    proc = subprocess.Popen([sys.executable, "-c", "print('hi')"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    proc.wait(timeout=5)
    _kill_subprocess(proc, grace_s=1.0)
    assert proc.poll() is not None


def test_promethean_parent_dir():
    parent = _promethean_parent_dir()
    assert parent.exists()
    assert parent.is_dir()


# ──────────────────────────────────────────────────────────────────────
# Popper
# ──────────────────────────────────────────────────────────────────────


def test_popper_v1471_runs_and_passes():
    results = popper_v1471()
    assert len(results) >= 18
    failed = [(n, m) for n, ok, m in results if not ok]
    assert not failed, f"popper failed: {failed}"


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


def test_write_report_json(tmp_path):
    r = DaemonReport(
        ok=True, verdict="PASS", host="127.0.0.1", port=18580, v1467_pid=1000,
        shutdown_reason="RUNTIME_LIMIT", n_polls=5, n_audits_seen=3, n_diff_events_emitted=2,
        n_errors=0, total_elapsed_s=10.0, diff_events=[], errors=[],
        guards=list(V1471_GUARDS), v3_guards=list(V1471_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=14, guards_total=14, timestamp=time.time(),
        log_path="/tmp/log", jsonl_stream_path="/tmp/stream.jsonl",
        daemon_report_path="/tmp/r.json", daemon_md_path="/tmp/r.md",
        out_dir="/tmp/out",
    )
    out = tmp_path / "r.json"
    write_report_json(r, out)
    assert out.exists()
    assert out.stat().st_size > 0
    parsed = json.loads(out.read_text(encoding="utf-8"))
    assert parsed["ok"] is True


def test_write_report_markdown(tmp_path):
    r = DaemonReport(
        ok=True, verdict="PASS", host="127.0.0.1", port=18580, v1467_pid=1000,
        shutdown_reason="RUNTIME_LIMIT", n_polls=5, n_audits_seen=3, n_diff_events_emitted=2,
        n_errors=0, total_elapsed_s=10.0,
        diff_events=[DiffEvent(
            event_type="diff", timestamp=time.time(), poll_index=1,
            baseline_id="a", current_id="b", diff_verdict="UNCHANGED",
            n_changes=0, elapsed_s=0.1,
        )],
        errors=[],
        guards=list(V1471_GUARDS), v3_guards=list(V1471_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=14, guards_total=14, timestamp=time.time(),
        log_path="/tmp/log", jsonl_stream_path="/tmp/stream.jsonl",
        daemon_report_path="/tmp/r.json", daemon_md_path="/tmp/r.md",
        out_dir="/tmp/out",
    )
    out = tmp_path / "r.md"
    write_report_markdown(r, out)
    assert out.exists()
    assert out.stat().st_size > 0
    text = out.read_text(encoding="utf-8")
    assert "V1471" in text
    assert "PASS" in text
    assert "Diff Events" in text


# ──────────────────────────────────────────────────────────────────────
# CLI subprocess
# ──────────────────────────────────────────────────────────────────────


def test_cli_meta_via_subprocess():
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1471_audit_monitor_daemon", "meta"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    meta = json.loads(proc.stdout.decode())
    assert meta["module"] == V1471_MODULE
    assert meta["version"] == V1471_VERSION
    assert len(meta["guards"]) >= 14
    assert "RUNTIME_LIMIT" in meta["shutdown_reasons"]


def test_cli_popper_via_subprocess():
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1471_audit_monitor_daemon", "popper"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    summary = json.loads(proc.stdout.decode())
    assert summary["n_passed"] == summary["n_checks"]
    assert summary["n_failed"] == 0


def test_cli_chain_via_subprocess():
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1471_audit_monitor_daemon", "chain"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    chain = json.loads(proc.stdout.decode())
    assert chain["all_ok"] is True
    assert "v1467_asi_audit_http_gateway_history_diff" in chain["chain"]


def _extract_json_summary(stdout_text: str) -> dict:
    """Extract the final JSON summary line from CLI stdout (which has log lines + JSON)."""
    # The JSON summary is the last line that starts with '{'
    last_json_line = None
    for line in stdout_text.splitlines():
        line = line.strip()
        if line.startswith("{") and line.endswith("}"):
            last_json_line = line
    if last_json_line is None:
        raise AssertionError(f"no JSON summary found in stdout:\n{stdout_text[:500]}")
    return json.loads(last_json_line)


def test_cli_demo_via_subprocess_real_subprocess(tmp_path):
    """`python -m apeireth.v1471_audit_monitor_daemon demo` boots V1467 + runs daemon."""
    out_dir = tmp_path / "v1471-demo"
    proc = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1471_audit_monitor_daemon",
            "demo",
            "--out-dir", str(out_dir),
        ],
        cwd=str(ROOT),
        capture_output=True,
        timeout=60,
    )
    assert proc.returncode == 0, f"stderr={proc.stderr.decode()[:500]}"
    summary = _extract_json_summary(proc.stdout.decode())
    assert summary["ok"] is True
    assert summary["verdict"] == "PASS"
    assert summary["n_polls"] >= 1
    assert summary["shutdown_reason"] in ("RUNTIME_LIMIT", "POLL_LIMIT", "KEYBOARD_INTERRUPT")
    # Daemon report + log + JSONL stream must exist
    report_path = Path(summary["daemon_report_path"])
    md_path = Path(summary["daemon_md_path"])
    log_path = Path(summary["log_path"])
    assert report_path.exists()
    assert md_path.exists()
    assert log_path.exists()
    # Report should have key fields
    parsed = json.loads(report_path.read_text(encoding="utf-8"))
    assert parsed["ok"] is True
    assert parsed["n_polls"] >= 1
    assert parsed["guards_passed"] >= 13  # at most 1 guard may fail (e.g., V1467_DIED path not exercised)


def test_cli_run_via_subprocess_real_subprocess_with_audit_injection(tmp_path):
    """`python -m apeireth.v1471_audit_monitor_daemon run --audit-every-poll 1` injects audits."""
    out_dir = tmp_path / "v1471-injection"
    proc = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1471_audit_monitor_daemon",
            "run",
            "--host", DEFAULT_HOST,
            "--port-min", "18780",
            "--port-max", "18880",
            "--max-runtime", "15",
            "--max-polls", "5",
            "--audit-every-poll", "1",
            "--out-dir", str(out_dir),
        ],
        cwd=str(ROOT),
        capture_output=True,
        timeout=60,
    )
    assert proc.returncode == 0, f"stderr={proc.stderr.decode()[:500]}"
    summary = _extract_json_summary(proc.stdout.decode())
    assert summary["ok"] is True
    assert summary["n_audits_seen"] >= 2  # at least baseline + 1 injection
    # Should have emitted diff events (at least 1 since we injected audits)
    assert summary["n_diff_events_emitted"] >= 1