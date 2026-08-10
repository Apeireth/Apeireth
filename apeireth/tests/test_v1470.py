"""Tests for V1470 — ASI Real V1469 Batch Harness + Cross-Client Equivalence Verifier.

Covers:
- Module metadata + constants
- Popper self-checks
- Dataclass shape + serialization
- Helper functions (latency stats, determinism score, sorted keys)
- Real batch harness run (N=2 V1469 subprocess invocations + cross-checks)
- Report writers (JSON + Markdown)
- CLI entry points
- Chain importability (V1469+V1468+V1467)
- Honest disclosure: NOT a CI pipeline, NOT a load tester, NOT a benchmark
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import pytest

# Make apeireth importable
APEIRETH_DIR = Path(__file__).resolve().parent.parent
WORKSPACE_DIR = APEIRETH_DIR.parent
sys.path.insert(0, str(WORKSPACE_DIR))

import apeireth.v1470_asi_v1469_batch_harness_cross_client_equivalence as v1470  # noqa: E402

V1470_MODULE = "v1470_asi_v1469_batch_harness_cross_client_equivalence"


# ──────────────────────────────────────────────────────────────────────
# Module metadata + constants
# ──────────────────────────────────────────────────────────────────────


class TestV1470Metadata:
    def test_module_name(self):
        assert v1470.V1470_MODULE == V1470_MODULE

    def test_version_format(self):
        parts = v1470.V1470_VERSION.split(".")
        assert len(parts) == 3
        assert all(p.isdigit() for p in parts)

    def test_schema_marker(self):
        assert v1470.V1470_SCHEMA.startswith("v1470.")
        assert v1470.V1470_SCHEMA.endswith("/v1")

    def test_date_iso(self):
        # YYYY-MM-DD
        assert len(v1470.V1470_DATE) == 10
        assert v1470.V1470_DATE[4] == "-"
        assert v1470.V1470_DATE[7] == "-"

    def test_guards_min_count(self):
        assert len(v1470.V1470_GUARDS) >= 14

    def test_v3_guards_min_count(self):
        assert len(v1470.V1470_V3_GUARDS) >= 7

    def test_borrowed_sources_min(self):
        assert len(v1470.BORROWED_SOURCES) >= 4

    def test_endpoints_count_six(self):
        # Cross-check verifies 6 endpoints × 2 client paths = 12 checks per run
        assert len(v1470.V1470_EQUIVALENCE_ENDPOINTS) == 6

    def test_default_host_loopback(self):
        # 主 23:44 骈插捣
        assert v1470.DEFAULT_HOST == "127.0.0.1"

    def test_default_n_runs_bounded(self):
        assert v1470.MIN_N_RUNS <= v1470.DEFAULT_N_RUNS <= v1470.MAX_N_RUNS
        assert v1470.MIN_N_RUNS == 2
        assert v1470.MAX_N_RUNS == 5

    def test_default_wallclock_bounded(self):
        # 主 00:44 质量工程化
        assert 0 < v1470.DEFAULT_MAX_WALLCLOCK_S <= 600.0

    def test_default_max_output_bounded(self):
        # 主 00:44 质量工程化
        assert 0 < v1470.DEFAULT_MAX_OUTPUT_BYTES <= 1024 * 1024

    def test_v1469_timeout_bounded(self):
        assert 0 < v1470.DEFAULT_V1469_TIMEOUT_S <= 180.0


# ──────────────────────────────────────────────────────────────────────
# Dataclass shape
# ──────────────────────────────────────────────────────────────────────


class TestV1470Dataclasses:
    def test_endpoint_equivalence_check_to_dict_shape(self):
        c = v1470.EndpointEquivalenceCheck(
            method="GET", path="/healthz",
            generated_status=200, generated_body_keys=("a", "b"),
            raw_status=200, raw_body_keys=("a", "b"),
            status_match=True, keys_match=True, ok=True, elapsed_ms=5.0,
        )
        d = c.to_dict()
        assert d["method"] == "GET"
        assert d["path"] == "/healthz"
        assert d["generated_status"] == 200
        assert d["raw_status"] == 200
        assert d["status_match"] is True
        assert d["keys_match"] is True
        assert d["ok"] is True
        assert d["elapsed_ms"] == 5.0
        assert d["generated_body_keys"] == ["a", "b"]
        assert d["raw_body_keys"] == ["a", "b"]
        assert d["error"] is None

    def test_v1469_run_summary_to_dict_shape(self):
        s = v1470.V1469RunSummary(
            run_index=1, ok=True, verdict="PASS", port=18380,
            server_pid=1234, client_pid=5678,
            elapsed_s=10.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="/tmp/c.py", result_path="/tmp/r.json",
            endpoint_keys=("GET /healthz", "GET /status"),
            v1469_report_path="/tmp/v1469.json",
        )
        d = s.to_dict()
        assert d["run_index"] == 1
        assert d["ok"] is True
        assert d["verdict"] == "PASS"
        assert d["port"] == 18380
        assert d["server_pid"] == 1234
        assert d["client_pid"] == 5678
        assert d["n_endpoints_ok"] == 6
        assert d["n_endpoints_total"] == 6
        assert d["endpoint_keys"] == ["GET /healthz", "GET /status"]

    def test_batch_equivalence_report_to_dict_shape(self):
        r = v1470.BatchEquivalenceReport(
            ok=True, verdict="PASS", host="127.0.0.1",
            n_runs_requested=3, n_runs_completed=3,
            n_runs_passed=3, n_runs_failed=0,
            runs=[], equivalence_checks=[],
            n_equivalence_checks=18, n_equivalence_passed=18, n_equivalence_failed=0,
            latency_p50_s=10.0, latency_p95_s=15.0, latency_mean_s=12.0, latency_max_s=20.0,
            determinism_score=1.0,
            ports_used=[18380, 18381, 18382], pids_used=[100, 101, 102],
            ports_distinct=True, pids_distinct=True,
            total_elapsed_s=45.0,
            guards=list(v1470.V1470_GUARDS), v3_guards=list(v1470.V1470_V3_GUARDS),
            borrowed_sources=list(v1470.BORROWED_SOURCES),
            guards_passed=15, guards_total=15,
            errors=[], timestamp=time.time(),
            batch_dir="/tmp/v1470-batch-12345",
            v1470_report_path="/tmp/v1470-batch-12345/report.json",
            v1470_md_path="/tmp/v1470-batch-12345/report.md",
        )
        d = r.to_dict()
        assert d["ok"] is True
        assert d["verdict"] == "PASS"
        assert d["n_runs_requested"] == 3
        assert d["n_runs_passed"] == 3
        assert d["n_equivalence_checks"] == 18
        assert d["determinism_score"] == 1.0
        assert d["ports_used"] == [18380, 18381, 18382]
        assert d["pids_distinct"] is True
        assert d["module"] == V1470_MODULE
        assert d["version"] == v1470.V1470_VERSION
        assert d["schema"] == v1470.V1470_SCHEMA
        assert d["date"] == v1470.V1470_DATE


# ──────────────────────────────────────────────────────────────────────
# Popper self-checks
# ──────────────────────────────────────────────────────────────────────


class TestV1470Popper:
    def test_popper_runs(self):
        results = v1470.popper_v1470()
        assert isinstance(results, list)
        assert len(results) >= 10

    def test_popper_all_pass(self):
        results = v1470.popper_v1470()
        failed = [(name, msg) for name, ok, msg in results if not ok]
        assert not failed, f"failed popper checks: {failed}"

    def test_popper_returns_tuple_format(self):
        results = v1470.popper_v1470()
        for entry in results:
            assert len(entry) == 3
            name, ok, msg = entry
            assert isinstance(name, str)
            assert isinstance(ok, bool)
            assert isinstance(msg, str)

    def test_popper_verbose_succeeds(self):
        # Just exercise the verbose branch
        import io
        from contextlib import redirect_stderr
        buf = io.StringIO()
        with redirect_stderr(buf):
            results = v1470.popper_v1470()
        assert len(results) >= 10
        assert "META_PRESENT" in [r[0] for r in results]


# ──────────────────────────────────────────────────────────────────────
# Helper functions
# ──────────────────────────────────────────────────────────────────────


class TestV1470Helpers:
    def test_sorted_keys_dict(self):
        keys = v1470._sorted_keys({"b": 1, "a": 2, "c": 3})
        assert keys == ("a", "b", "c")

    def test_sorted_keys_empty_dict(self):
        keys = v1470._sorted_keys({})
        assert keys == ()

    def test_sorted_keys_non_dict(self):
        keys = v1470._sorted_keys([1, 2, 3])
        assert keys == ()

    def test_sorted_keys_none(self):
        keys = v1470._sorted_keys(None)
        assert keys == ()

    def test_latency_stats_single(self):
        p50, p95, mean, mx = v1470._latency_stats([5.0])
        assert p50 == 5.0 and p95 == 5.0 and mean == 5.0 and mx == 5.0

    def test_latency_stats_multiple(self):
        p50, p95, mean, mx = v1470._latency_stats([1.0, 2.0, 3.0, 4.0, 5.0])
        # n=5, p50_idx = int(0.5 * 4) = 2 → sorted[2] = 3.0
        assert p50 == 3.0
        # p95_idx = int(0.95 * 4) = 3 → sorted[3] = 4.0
        assert p95 == 4.0
        assert mean == 3.0
        assert mx == 5.0

    def test_latency_stats_empty(self):
        p50, p95, mean, mx = v1470._latency_stats([])
        assert p50 == 0.0 and p95 == 0.0 and mean == 0.0 and mx == 0.0

    def test_determinism_score_all_identical(self):
        r1 = v1470.V1469RunSummary(
            run_index=1, ok=True, verdict="PASS", port=1, server_pid=10, client_pid=20,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="",
            endpoint_keys=("GET /a", "GET /b"),
            v1469_report_path="",
        )
        r2 = v1470.V1469RunSummary(
            run_index=2, ok=True, verdict="PASS", port=2, server_pid=11, client_pid=21,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="",
            endpoint_keys=("GET /a", "GET /b"),
            v1469_report_path="",
        )
        assert v1470._determinism_score([r1, r2]) == 1.0

    def test_determinism_score_partial(self):
        r1 = v1470.V1469RunSummary(
            run_index=1, ok=True, verdict="PASS", port=1, server_pid=10, client_pid=20,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="",
            endpoint_keys=("GET /a", "GET /b"),
            v1469_report_path="",
        )
        r2 = v1470.V1469RunSummary(
            run_index=2, ok=True, verdict="PASS", port=2, server_pid=11, client_pid=21,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="",
            endpoint_keys=("GET /a", "GET /c"),  # different
            v1469_report_path="",
        )
        r3 = v1470.V1469RunSummary(
            run_index=3, ok=True, verdict="PASS", port=3, server_pid=12, client_pid=22,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="",
            endpoint_keys=("GET /a", "GET /b"),  # matches r1
            v1469_report_path="",
        )
        # 2/3 match r1 → 0.6667
        score = v1470._determinism_score([r1, r2, r3])
        assert abs(score - (2/3)) < 0.01

    def test_determinism_score_single_run(self):
        r1 = v1470.V1469RunSummary(
            run_index=1, ok=True, verdict="PASS", port=1, server_pid=10, client_pid=20,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="",
            endpoint_keys=("GET /a",),
            v1469_report_path="",
        )
        # Single run → trivially 1.0
        assert v1470._determinism_score([r1]) == 1.0

    def test_is_port_free_returns_bool(self):
        result = v1470._is_port_free("127.0.0.1", 1, timeout_s=0.5)
        assert isinstance(result, bool)

    def test_wait_for_port_unreachable(self):
        # Port 1 should not be listening
        result = v1470._wait_for_port("127.0.0.1", 1, timeout_s=1.0, poll_s=0.25)
        assert result is False

    def test_promethean_parent_dir_exists(self):
        parent = v1470._promethean_parent_dir()
        assert parent.exists()
        assert parent.is_dir()
        # Should be the workspace directory
        assert (parent / "apeireth").exists()

    def test_kill_subprocess_already_exited(self):
        proc = subprocess.Popen(
            [sys.executable, "-c", "import sys; sys.exit(0)"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        proc.wait()
        v1470._kill_subprocess(proc, grace_s=1.0)
        assert proc.poll() == 0


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


class TestV1470ReportWriters:
    def test_write_report_json(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1470_test_"))
        out_path = tmp / "report.json"
        r = v1470.BatchEquivalenceReport(
            ok=True, verdict="PASS", host="127.0.0.1",
            n_runs_requested=3, n_runs_completed=3,
            n_runs_passed=3, n_runs_failed=0,
            runs=[], equivalence_checks=[],
            n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
            latency_p50_s=1.0, latency_p95_s=2.0, latency_mean_s=1.5, latency_max_s=3.0,
            determinism_score=1.0,
            ports_used=[18380, 18381, 18382], pids_used=[100, 101, 102],
            ports_distinct=True, pids_distinct=True,
            total_elapsed_s=10.0,
            guards=list(v1470.V1470_GUARDS), v3_guards=list(v1470.V1470_V3_GUARDS),
            borrowed_sources=list(v1470.BORROWED_SOURCES),
            guards_passed=15, guards_total=15,
            errors=[], timestamp=time.time(),
            batch_dir=str(tmp), v1470_report_path=str(out_path), v1470_md_path=str(out_path.with_suffix(".md")),
        )
        written = v1470.write_report_json(r, out_path)
        assert written.exists()
        with open(written, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert data["verdict"] == "PASS"
        assert data["n_runs_passed"] == 3

    def test_write_report_markdown(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1470_test_"))
        out_path = tmp / "report.md"
        r = v1470.BatchEquivalenceReport(
            ok=True, verdict="PASS", host="127.0.0.1",
            n_runs_requested=3, n_runs_completed=3,
            n_runs_passed=3, n_runs_failed=0,
            runs=[], equivalence_checks=[],
            n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
            latency_p50_s=1.0, latency_p95_s=2.0, latency_mean_s=1.5, latency_max_s=3.0,
            determinism_score=1.0,
            ports_used=[18380, 18381, 18382], pids_used=[100, 101, 102],
            ports_distinct=True, pids_distinct=True,
            total_elapsed_s=10.0,
            guards=list(v1470.V1470_GUARDS), v3_guards=list(v1470.V1470_V3_GUARDS),
            borrowed_sources=list(v1470.BORROWED_SOURCES),
            guards_passed=15, guards_total=15,
            errors=[], timestamp=time.time(),
            batch_dir=str(tmp), v1470_report_path="", v1470_md_path=str(out_path),
        )
        written = v1470.write_report_markdown(r, out_path)
        assert written.exists()
        content = written.read_text(encoding="utf-8")
        assert "V1470" in content
        assert "PASS" in content
        assert "Determinism score" in content


# ──────────────────────────────────────────────────────────────────────
# CLI entry points
# ──────────────────────────────────────────────────────────────────────


class TestV1470Cli:
    def test_cli_meta(self):
        from io import StringIO
        from contextlib import redirect_stdout
        buf = StringIO()
        with redirect_stdout(buf):
            exit_code = v1470.main(["meta"])
        assert exit_code == 0
        output = buf.getvalue()
        data = json.loads(output)
        assert data["module"] == V1470_MODULE
        assert data["version"] == v1470.V1470_VERSION
        assert len(data["guards"]) >= 14

    def test_cli_chain(self):
        from io import StringIO
        from contextlib import redirect_stdout
        buf = StringIO()
        with redirect_stdout(buf):
            exit_code = v1470.main(["chain"])
        assert exit_code == 0
        output = buf.getvalue()
        data = json.loads(output)
        assert data["all_ok"] is True
        assert "v1469" in str(data["chain"])
        assert "v1468" in str(data["chain"])
        assert "v1467" in str(data["chain"])

    def test_cli_popper(self):
        from io import StringIO
        from contextlib import redirect_stdout
        buf = StringIO()
        with redirect_stdout(buf):
            exit_code = v1470.main(["popper"])
        assert exit_code == 0
        output = buf.getvalue()
        data = json.loads(output)
        assert data["n_passed"] == data["n_checks"]
        assert data["n_failed"] == 0

    def test_cli_help(self):
        from io import StringIO
        from contextlib import redirect_stdout
        buf = StringIO()
        with redirect_stdout(buf):
            exit_code = v1470.main(["help"])
        assert exit_code == 0
        assert "V1470" in buf.getvalue()

    def test_cli_run_unknown_subcmd_fails(self):
        # argparse raises SystemExit when no subcommand is given
        with pytest.raises(SystemExit) as exc_info:
            v1470.main([])
        assert exc_info.value.code != 0


# ──────────────────────────────────────────────────────────────────────
# Chain importability
# ──────────────────────────────────────────────────────────────────────


class TestV1470Chain:
    def test_v1469_importable(self):
        import apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver as v1469
        assert v1469.V1469_MODULE is not None

    def test_v1468_importable(self):
        import apeireth.v1468_asi_openapi_v1467_schema_and_client_generator as v1468
        assert v1468.V1468_MODULE is not None

    def test_v1467_importable(self):
        import apeireth.v1467_asi_audit_http_gateway_history_diff as v1467
        assert v1467.V1467_MODULE is not None


# ──────────────────────────────────────────────────────────────────────
# Guard evaluation
# ──────────────────────────────────────────────────────────────────────


class TestV1470Guards:
    def test_evaluate_guards_returns_list(self):
        r = v1470.BatchEquivalenceReport(
            ok=True, verdict="PASS", host="127.0.0.1",
            n_runs_requested=3, n_runs_completed=3,
            n_runs_passed=3, n_runs_failed=0,
            runs=[], equivalence_checks=[],
            n_equivalence_checks=18, n_equivalence_passed=18, n_equivalence_failed=0,
            latency_p50_s=10.0, latency_p95_s=15.0, latency_mean_s=12.0, latency_max_s=20.0,
            determinism_score=1.0,
            ports_used=[18380, 18381, 18382], pids_used=[100, 101, 102],
            ports_distinct=True, pids_distinct=True,
            total_elapsed_s=45.0,
            guards=list(v1470.V1470_GUARDS), v3_guards=list(v1470.V1470_V3_GUARDS),
            borrowed_sources=list(v1470.BORROWED_SOURCES),
            guards_passed=15, guards_total=15,
            errors=[], timestamp=time.time(),
            batch_dir="/tmp/x", v1470_report_path="/tmp/x.json", v1470_md_path="/tmp/x.md",
        )
        results = v1470._evaluate_guards(r)
        assert isinstance(results, list)
        assert len(results) == len(v1470.V1470_GUARDS)
        # All guards should pass for a well-formed PASS report
        for name, ok, msg in results:
            assert isinstance(name, str)
            assert isinstance(ok, bool)
            assert isinstance(msg, str)

    def test_evaluate_guards_detects_failure(self):
        # Build a report that should fail many guards
        r = v1470.BatchEquivalenceReport(
            ok=False, verdict="FAIL", host="127.0.0.1",
            n_runs_requested=3, n_runs_completed=1,  # not enough
            n_runs_passed=0, n_runs_failed=1,
            runs=[], equivalence_checks=[],
            n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
            latency_p50_s=0.0, latency_p95_s=0.0, latency_mean_s=0.0, latency_max_s=0.0,
            determinism_score=0.0,
            ports_used=[18380], pids_used=[100],  # only one
            ports_distinct=False, pids_distinct=False,
            total_elapsed_s=0.0,
            guards=list(v1470.V1470_GUARDS), v3_guards=list(v1470.V1470_V3_GUARDS),
            borrowed_sources=list(v1470.BORROWED_SOURCES),
            guards_passed=0, guards_total=15,
            errors=[], timestamp=time.time(),
            batch_dir="/nonexistent", v1470_report_path="/nonexistent.json", v1470_md_path="/nonexistent.md",
        )
        results = v1470._evaluate_guards(r)
        failed = [(name, msg) for name, ok, msg in results if not ok]
        # Should have several failures
        assert len(failed) >= 3, f"expected multiple guard failures, got: {results}"


# ──────────────────────────────────────────────────────────────────────
# Honest disclosure
# ──────────────────────────────────────────────────────────────────────


class TestV1470HonestDisclosure:
    def test_v1470_v3_guards_contain_not_asi(self):
        assert "GUARD_NOT_ASI" in v1470.V1470_V3_GUARDS

    def test_v1470_v3_guards_contain_not_phenomenal(self):
        assert "GUARD_NOT_PHENOMENAL" in v1470.V1470_V3_GUARDS

    def test_v1470_v3_guards_contain_not_human_level(self):
        assert "GUARD_NOT_HUMAN_LEVEL" in v1470.V1470_V3_GUARDS

    def test_v1470_v3_guards_contain_not_load_test(self):
        assert "GUARD_HARNESS_NOT_LOAD_TEST" in v1470.V1470_V3_GUARDS

    def test_v1470_v3_guards_contain_not_orchestrator(self):
        assert "GUARD_NOT_ORCHESTRATOR" in v1470.V1470_V3_GUARDS

    def test_docstring_states_not_load_tester(self):
        # 主 17:58 + 主 20:46 不假装
        assert "load tester" in v1470.__doc__.lower()

    def test_docstring_states_not_benchmark(self):
        assert "benchmark" in v1470.__doc__.lower()

    def test_docstring_states_not_ci(self):
        assert "ci" in v1470.__doc__.lower()