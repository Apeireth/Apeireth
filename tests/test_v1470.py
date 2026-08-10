"""Tests for V1470 — ASI Real V1469 Batch Harness + V1468-Generated-Client ↔ stdlib http.client Cross-Equivalence.

Run: python -m pytest tests/test_v1470.py -v

Coverage:
  - Module metadata + bounded defaults
  - EndpointEquivalenceCheck + V1469RunSummary + BatchEquivalenceReport dataclasses
  - _build_query_string / _raw_path_for_stdlib (cross-client URL parity)
  - _sorted_keys / _latency_stats / _determinism_score helpers
  - popper_v1470: 16 in-process self-checks
  - write_report_json / write_report_markdown
  - run_v1470_batch: real subprocess boot V1469 + cross-client equivalence
  - V1470 CLI: meta + popper + chain commands
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

from apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence import (
    V1470_MODULE, V1470_VERSION, V1470_SCHEMA, V1470_DATE,
    V1470_GUARDS, V1470_V3_GUARDS, BORROWED_SOURCES,
    V1470_EQUIVALENCE_ENDPOINTS, V1470_EQUIVALENCE_QUERY_PARAMS,
    DEFAULT_HOST, DEFAULT_N_RUNS, MIN_N_RUNS, MAX_N_RUNS,
    DEFAULT_MAX_WALLCLOCK_S, DEFAULT_MAX_OUTPUT_BYTES,
    AUDIT_RUN_BODY,
    EndpointEquivalenceCheck, V1469RunSummary, BatchEquivalenceReport,
    _build_query_string, _raw_path_for_stdlib, _sorted_keys,
    _latency_stats, _determinism_score,
    popper_v1470,
    write_report_json, write_report_markdown,
)


# ──────────────────────────────────────────────────────────────────────
# Module metadata + constants
# ──────────────────────────────────────────────────────────────────────


def test_module_metadata():
    assert V1470_MODULE == "v1470_asi_v1469_batch_harness_cross_client_equivalence"
    assert V1470_VERSION == "0.1.0"
    assert V1470_SCHEMA == "v1470.asi-real-v1469-batch-harness-cross-client-equivalence/v1"
    assert V1470_DATE == "2026-08-10"


def test_bounded_defaults():
    assert DEFAULT_HOST == "127.0.0.1"
    assert MIN_N_RUNS <= DEFAULT_N_RUNS <= MAX_N_RUNS
    assert 0 < DEFAULT_MAX_WALLCLOCK_S <= 600.0
    assert 0 < DEFAULT_MAX_OUTPUT_BYTES <= 1024 * 1024


def test_borrowed_sources_declared():
    assert len(BORROWED_SOURCES) >= 4
    assert "v1469" in BORROWED_SOURCES
    assert "v1468" in BORROWED_SOURCES
    assert "v1467" in BORROWED_SOURCES
    assert "stdlib" in BORROWED_SOURCES


def test_guards_declared():
    assert len(V1470_GUARDS) >= 14
    assert len(V1470_V3_GUARDS) >= 7
    assert any("NOT_ASI" in g for g in V1470_V3_GUARDS)
    assert any("NOT_PHENOMENAL" in g for g in V1470_V3_GUARDS)


def test_endpoints_declared():
    assert len(V1470_EQUIVALENCE_ENDPOINTS) == 6
    # All 6 V1467 endpoints covered
    paths = {p for _, p in V1470_EQUIVALENCE_ENDPOINTS}
    assert "/healthz" in paths
    assert "/status" in paths
    assert "/audit/run" in paths
    assert "/audit/history" in paths
    assert "/audit/{audit_id}" in paths
    assert "/audit/diff" in paths


# ──────────────────────────────────────────────────────────────────────
# Query string helpers — these fix the V1470 cross-client equivalence bug
# ──────────────────────────────────────────────────────────────────────


def test_build_query_string_for_diff():
    """Cross-client bug fix: /audit/diff must include query params for raw stdlib."""
    qs = _build_query_string("/audit/diff")
    assert "baseline_id=" in qs
    assert "current_id=" in qs


def test_build_query_string_for_audit_id():
    qs = _build_query_string("/audit/{audit_id}")
    assert "audit_id=" in qs


def test_build_query_string_for_get_endpoints():
    assert _build_query_string("/healthz") == ""
    assert _build_query_string("/status") == ""
    assert _build_query_string("/audit/history") == ""


def test_raw_path_for_stdlib_adds_query():
    p = _raw_path_for_stdlib("/audit/diff")
    assert p.startswith("/audit/diff?")
    assert "baseline_id=" in p
    assert "current_id=" in p


def test_raw_path_for_stdlib_passthrough():
    assert _raw_path_for_stdlib("/healthz") == "/healthz"
    assert _raw_path_for_stdlib("/status") == "/status"


def test_raw_path_for_stdlib_audit_get():
    p = _raw_path_for_stdlib("/audit/{audit_id}")
    assert p.startswith("/audit/{audit_id}?")
    assert "audit_id=" in p


# ──────────────────────────────────────────────────────────────────────
# Helper functions
# ──────────────────────────────────────────────────────────────────────


def test_sorted_keys_for_dict():
    assert _sorted_keys({"b": 1, "a": 2}) == ("a", "b")


def test_sorted_keys_for_non_dict():
    assert _sorted_keys([1, 2, 3]) == ()
    assert _sorted_keys("foo") == ()
    assert _sorted_keys(None) == ()


def test_latency_stats_basic():
    p50, p95, mean, mx = _latency_stats([1.0, 2.0, 3.0, 4.0, 5.0])
    assert p50 == 3.0  # sorted[2]
    assert p95 == 4.0  # sorted[3]
    assert mean == 3.0
    assert mx == 5.0


def test_latency_stats_empty():
    p50, p95, mean, mx = _latency_stats([])
    assert p50 == 0.0
    assert p95 == 0.0
    assert mean == 0.0
    assert mx == 0.0


def test_latency_stats_single():
    p50, p95, mean, mx = _latency_stats([7.0])
    assert p50 == 7.0
    assert p95 == 7.0
    assert mean == 7.0
    assert mx == 7.0


def test_determinism_score_identical():
    r1 = _mk_run(1, ("a", "b"))
    r2 = _mk_run(2, ("a", "b"))
    r3 = _mk_run(3, ("a", "b"))
    assert _determinism_score([r1, r2, r3]) == 1.0


def test_determinism_score_different():
    r1 = _mk_run(1, ("a", "b"))
    r2 = _mk_run(2, ("a", "c"))
    assert _determinism_score([r1, r2]) == 0.5


def test_determinism_score_single_run():
    r1 = _mk_run(1, ("a", "b"))
    assert _determinism_score([r1]) == 1.0


def _mk_run(idx, endpoint_keys):
    return V1469RunSummary(
        run_index=idx, ok=True, verdict="PASS", port=18380 + idx,
        server_pid=1000 + idx, client_pid=2000 + idx,
        elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
        client_path="", result_path="",
        endpoint_keys=endpoint_keys, v1469_report_path="",
    )


# ──────────────────────────────────────────────────────────────────────
# Dataclasses
# ──────────────────────────────────────────────────────────────────────


def test_endpoint_equivalence_check_dataclass():
    c = EndpointEquivalenceCheck(
        method="GET", path="/healthz",
        generated_status=200, generated_body_keys=("a",),
        raw_status=200, raw_body_keys=("a",),
        status_match=True, keys_match=True, ok=True, elapsed_ms=1.0,
    )
    d = c.to_dict()
    assert d["method"] == "GET"
    assert d["path"] == "/healthz"
    assert d["ok"] is True
    assert d["generated_status"] == 200


def test_v1469_run_summary_dataclass():
    s = V1469RunSummary(
        run_index=1, ok=True, verdict="PASS", port=18380,
        server_pid=100, client_pid=200,
        elapsed_s=2.0, n_endpoints_ok=6, n_endpoints_total=6,
        client_path="/c.py", result_path="/r.json",
        endpoint_keys=("a",), v1469_report_path="/v1469.json",
    )
    d = s.to_dict()
    assert d["run_index"] == 1
    assert d["ok"] is True
    assert d["port"] == 18380


def test_batch_equivalence_report_dataclass():
    r = BatchEquivalenceReport(
        ok=True, verdict="PASS", host="127.0.0.1",
        n_runs_requested=2, n_runs_completed=2, n_runs_passed=2, n_runs_failed=0,
        runs=[], equivalence_checks=[],
        n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
        latency_p50_s=1.0, latency_p95_s=2.0, latency_mean_s=1.5, latency_max_s=3.0,
        determinism_score=1.0, ports_used=[1, 2], pids_used=[10, 20],
        ports_distinct=True, pids_distinct=True,
        total_elapsed_s=10.0,
        guards=list(V1470_GUARDS), v3_guards=list(V1470_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=15, guards_total=15,
        errors=[], timestamp=time.time(),
        batch_dir="/tmp/x", v1470_report_path="/tmp/x.json", v1470_md_path="/tmp/x.md",
    )
    d = r.to_dict()
    assert d["ok"] is True
    assert d["n_runs_requested"] == 2
    assert d["determinism_score"] == 1.0
    assert d["module"] == V1470_MODULE


# ──────────────────────────────────────────────────────────────────────
# Popper
# ──────────────────────────────────────────────────────────────────────


def test_popper_v1470_runs_and_passes():
    results = popper_v1470()
    assert len(results) >= 16
    failed = [(n, m) for n, ok, m in results if not ok]
    assert not failed, f"popper failed: {failed}"


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


def test_write_report_json(tmp_path):
    r = BatchEquivalenceReport(
        ok=True, verdict="PASS", host="127.0.0.1",
        n_runs_requested=2, n_runs_completed=2, n_runs_passed=2, n_runs_failed=0,
        runs=[], equivalence_checks=[],
        n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
        latency_p50_s=1.0, latency_p95_s=2.0, latency_mean_s=1.5, latency_max_s=3.0,
        determinism_score=1.0, ports_used=[1, 2], pids_used=[10, 20],
        ports_distinct=True, pids_distinct=True,
        total_elapsed_s=10.0,
        guards=list(V1470_GUARDS), v3_guards=list(V1470_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=15, guards_total=15,
        errors=[], timestamp=time.time(),
        batch_dir="/tmp/x", v1470_report_path="/tmp/x.json", v1470_md_path="/tmp/x.md",
    )
    out = tmp_path / "r.json"
    write_report_json(r, out)
    assert out.exists()
    assert out.stat().st_size > 0
    parsed = json.loads(out.read_text(encoding="utf-8"))
    assert parsed["ok"] is True


def test_write_report_markdown(tmp_path):
    r = BatchEquivalenceReport(
        ok=True, verdict="PASS", host="127.0.0.1",
        n_runs_requested=2, n_runs_completed=2, n_runs_passed=2, n_runs_failed=0,
        runs=[_mk_run(1, ("a", "b"))], equivalence_checks=[],
        n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
        latency_p50_s=1.0, latency_p95_s=2.0, latency_mean_s=1.5, latency_max_s=3.0,
        determinism_score=1.0, ports_used=[1, 2], pids_used=[10, 20],
        ports_distinct=True, pids_distinct=True,
        total_elapsed_s=10.0,
        guards=list(V1470_GUARDS), v3_guards=list(V1470_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=15, guards_total=15,
        errors=[], timestamp=time.time(),
        batch_dir="/tmp/x", v1470_report_path="/tmp/x.json", v1470_md_path="/tmp/x.md",
    )
    out = tmp_path / "r.md"
    write_report_markdown(r, out)
    assert out.exists()
    assert out.stat().st_size > 0
    text = out.read_text(encoding="utf-8")
    assert "V1470" in text
    assert "PASS" in text


# ──────────────────────────────────────────────────────────────────────
# CLI subprocess
# ──────────────────────────────────────────────────────────────────────


def test_cli_meta_via_subprocess():
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence", "meta"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    meta = json.loads(proc.stdout.decode())
    assert meta["module"] == V1470_MODULE
    assert meta["version"] == V1470_VERSION
    assert len(meta["guards"]) >= 14


def test_cli_popper_via_subprocess():
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence", "popper"],
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
        [sys.executable, "-m", "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence", "chain"],
        cwd=str(ROOT),
        capture_output=True,
        timeout=15,
    )
    assert proc.returncode == 0, proc.stderr.decode()
    chain = json.loads(proc.stdout.decode())
    assert chain["all_ok"] is True
    assert "v1469_asi_real_two_process_v1468_client_v1467_server_driver" in chain["chain"]


def test_cli_run_via_subprocess_real_batch(tmp_path):
    """Full batch: V1470 spawns V1469 N=2, then runs cross-client equivalence."""
    out_json = tmp_path / "v1470-batch.json"
    proc = subprocess.run(
        [
            sys.executable, "-m",
            "apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence",
            "run",
            "--n-runs", "2",
            "--host", DEFAULT_HOST,
            "--max-wallclock", "300",
            "--out", str(out_json),
        ],
        cwd=str(ROOT),
        capture_output=True,
        timeout=300,
    )
    assert proc.returncode == 0, f"stderr={proc.stderr.decode()[:500]}"
    assert out_json.exists()
    parsed = json.loads(out_json.read_text(encoding="utf-8"))
    # Core invariants for "anyone can take over" + cross-client equivalence
    assert parsed["ok"] is True
    assert parsed["verdict"] == "PASS"
    assert parsed["n_runs_completed"] == 2
    assert parsed["n_runs_passed"] == 2
    # All 6 endpoints × 2 runs = 12 equivalence checks must pass
    assert parsed["n_equivalence_checks"] == 12
    assert parsed["n_equivalence_passed"] == 12
    assert parsed["determinism_score"] == 1.0
    # PIDs must be distinct (each V1469 spawns fresh subprocess)
    assert parsed["pids_distinct"] is True
    # Ports in valid V1469 range [18380, 18480]
    for p in parsed["ports_used"]:
        assert 18380 <= p <= 18480