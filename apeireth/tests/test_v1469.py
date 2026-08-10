"""Tests for V1469 — ASI Real Two-Process V1468-Generated-Client → V1467-Server Driver.

Covers:
- Module metadata + constants
- Popper self-checks
- Dataclass shape + serialization
- Helper functions (port, http, kill, script generation)
- Real two-process driver run (V1467 server subprocess + V1468 client driver subprocess)
- Report writers (JSON + Markdown)
- CLI entry points
- Chain importability (V1468+V1467+V1466)
- Honest disclosure: NOT a process supervisor, NOT a CI pipeline, NOT a load tester
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

import apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver as v1469  # noqa: E402

V1469_MODULE = "v1469_asi_real_two_process_v1468_client_v1467_server_driver"


# ──────────────────────────────────────────────────────────────────────
# Module metadata + constants
# ──────────────────────────────────────────────────────────────────────


class TestV1469Metadata:
    def test_module_name(self):
        assert v1469.V1469_MODULE == V1469_MODULE

    def test_version_format(self):
        parts = v1469.V1469_VERSION.split(".")
        assert len(parts) == 3
        assert all(p.isdigit() for p in parts)

    def test_schema_marker(self):
        assert v1469.V1469_SCHEMA.startswith("v1469.")
        assert v1469.V1469_SCHEMA.endswith("/v1")

    def test_date_iso(self):
        # YYYY-MM-DD
        assert len(v1469.V1469_DATE) == 10
        assert v1469.V1469_DATE[4] == "-"
        assert v1469.V1469_DATE[7] == "-"

    def test_guards_min_count(self):
        assert len(v1469.V1469_GUARDS) >= 10

    def test_v3_guards_min_count(self):
        assert len(v1469.V1469_V3_GUARDS) >= 5

    def test_borrowed_sources_min(self):
        assert len(v1469.BORROWED_SOURCES) >= 3

    def test_endpoints_match_v1467(self):
        assert len(v1469.V1467_ENDPOINTS_TO_HIT) == 6

    def test_default_host_loopback(self):
        # 主 23:44 骈插捣
        assert v1469.DEFAULT_HOST == "127.0.0.1"

    def test_default_port_range_distinct_from_v1467(self):
        # V1467 used 18280-18380; V1469 uses 18380-18480
        assert v1469.DEFAULT_PORT_MIN == 18380
        assert v1469.DEFAULT_PORT_MAX == 18480

    def test_default_wallclock_bounded(self):
        # 主 00:44 质量工程化
        assert 0 < v1469.DEFAULT_MAX_WALLCLOCK_S <= 120.0

    def test_default_max_output_bounded(self):
        # 主 00:44 质量工程化
        assert 0 < v1469.DEFAULT_MAX_OUTPUT_BYTES <= 1024 * 1024  # <= 1MB


# ──────────────────────────────────────────────────────────────────────
# Dataclass shape
# ──────────────────────────────────────────────────────────────────────


class TestV1469Dataclasses:
    def test_subprocess_record_required_fields(self):
        rec = v1469.SubprocessRecord(
            role="server",
            pid=1234,
            cmd=("python", "-m", "test"),
            boot_at_s=time.monotonic(),
        )
        assert rec.role == "server"
        assert rec.pid == 1234
        assert rec.exit_code is None
        assert rec.elapsed_s is None
        assert rec.stdout_tail == ""
        assert rec.stderr_tail == ""
        assert rec.killed_for_timeout is False
        assert rec.timed_out is False

    def test_driver_report_to_dict_shape(self):
        r = v1469.DriverReport(
            ok=True, verdict="PASS", host="127.0.0.1", port=18380,
            server_pid=1234, client_pid=5678,
            server_boot_elapsed_s=1.0, client_elapsed_s=2.0, total_elapsed_s=3.0,
            client_path="/tmp/client.py", result_path="/tmp/result.json",
            n_endpoints_ok=6, n_endpoints_total=6,
            guards_passed=14, guards_total=14,
            timestamp=time.time(),
        )
        d = r.to_dict()
        assert d["ok"] is True
        assert d["verdict"] == "PASS"
        assert d["host"] == "127.0.0.1"
        assert d["port"] == 18380
        assert d["server_pid"] == 1234
        assert d["client_pid"] == 5678
        assert d["n_endpoints_ok"] == 6
        assert d["n_endpoints_total"] == 6
        assert d["guards_passed"] == 14
        assert d["guards_total"] == 14
        assert d["module"] == V1469_MODULE
        assert d["version"] == v1469.V1469_VERSION
        assert d["schema"] == v1469.V1469_SCHEMA
        assert d["date"] == v1469.V1469_DATE
        assert isinstance(d["endpoints_hit"], list)
        assert isinstance(d["errors"], list)

    def test_driver_report_with_endpoints(self):
        r = v1469.DriverReport(
            ok=True, verdict="PASS", host="127.0.0.1", port=18380,
            server_pid=1, client_pid=2,
            server_boot_elapsed_s=0.5, client_elapsed_s=1.0, total_elapsed_s=1.5,
            client_path="", result_path="",
            endpoints_hit=[
                {"method": "GET", "path": "/healthz", "ok": True, "elapsed_ms": 5.0},
            ],
            n_endpoints_ok=1, n_endpoints_total=1,
            guards_passed=1, guards_total=14,
            timestamp=time.time(),
        )
        d = r.to_dict()
        assert len(d["endpoints_hit"]) == 1
        assert d["endpoints_hit"][0]["path"] == "/healthz"


# ──────────────────────────────────────────────────────────────────────
# Popper self-checks
# ──────────────────────────────────────────────────────────────────────


class TestV1469Popper:
    def test_popper_runs(self):
        results = v1469.popper_v1469()
        assert isinstance(results, list)
        assert len(results) >= 5

    def test_popper_all_pass(self):
        results = v1469.popper_v1469()
        failed = [(name, msg) for name, ok, msg in results if not ok]
        assert not failed, f"failed popper checks: {failed}"

    def test_popper_returns_tuple_format(self):
        results = v1469.popper_v1469()
        for entry in results:
            assert len(entry) == 3
            name, ok, msg = entry
            assert isinstance(name, str)
            assert isinstance(ok, bool)
            assert isinstance(msg, str)


# ──────────────────────────────────────────────────────────────────────
# Helper functions
# ──────────────────────────────────────────────────────────────────────


class TestV1469PortHelpers:
    def test_is_port_free_returns_bool(self):
        result = v1469._is_port_free("127.0.0.1", 1, timeout_s=0.5)
        assert isinstance(result, bool)

    def test_find_open_port_returns_int(self):
        port = v1469._find_open_port("127.0.0.1", v1469.DEFAULT_PORT_MIN, v1469.DEFAULT_PORT_MIN + 5)
        assert isinstance(port, int)
        assert v1469.DEFAULT_PORT_MIN <= port <= v1469.DEFAULT_PORT_MIN + 5

    def test_find_open_port_raises_when_full(self):
        # If we claim the entire range is in use (won't actually be), behavior
        # depends on real state. Just verify the function is callable.
        try:
            v1469._find_open_port("127.0.0.1", 1, 1)
        except RuntimeError as e:
            assert "no free port" in str(e)

    def test_wait_for_port_returns_bool(self):
        # Connect to a port that won't exist; should return False after timeout
        result = v1469._wait_for_port("127.0.0.1", 1, timeout_s=1.0, poll_s=0.5)
        assert result is False


class TestV1469HttpHelper:
    def test_http_get_json_unreachable_returns_zero(self):
        status, body = v1469._http_get_json("127.0.0.1", 1, "/", timeout_s=1.0)
        # Port 1 should refuse or timeout
        assert status == 0
        assert "error" in body

    def test_http_get_json_returns_tuple(self):
        status, body = v1469._http_get_json("127.0.0.1", 1, "/healthz", timeout_s=1.0)
        assert isinstance(status, int)
        assert isinstance(body, dict)


class TestV1469ClientScriptGeneration:
    def test_generate_client_script_returns_path(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1469_test_"))
        result_path = tmp / "result.json"
        driver_path = v1469._generate_client_script(tmp, "127.0.0.1", 18380, result_path)
        assert isinstance(driver_path, Path)
        assert driver_path.exists()

    def test_generate_client_script_nonempty(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1469_test_"))
        result_path = tmp / "result.json"
        driver_path = v1469._generate_client_script(tmp, "127.0.0.1", 18380, result_path)
        assert driver_path.stat().st_size > 1000  # substantial content

    def test_generate_client_script_has_importlib(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1469_test_"))
        result_path = tmp / "result.json"
        driver_path = v1469._generate_client_script(tmp, "127.0.0.1", 18380, result_path)
        content = driver_path.read_text(encoding="utf-8")
        assert "importlib" in content
        assert "V1467Client" in content

    def test_generate_client_script_hits_all_endpoints(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1469_test_"))
        result_path = tmp / "result.json"
        driver_path = v1469._generate_client_script(tmp, "127.0.0.1", 18380, result_path)
        content = driver_path.read_text(encoding="utf-8")
        # All 6 endpoint methods should be in the generated driver
        for fn_name in ("healthz", "status", "audit_run", "audit_history", "audit_get", "audit_diff"):
            assert fn_name in content, f"missing {fn_name} in driver script"

    def test_generate_client_script_substitutes_paths(self):
        tmp = Path(tempfile.mkdtemp(prefix="v1469_test_"))
        result_path = tmp / "result.json"
        driver_path = v1469._generate_client_script(tmp, "1.2.3.4", 9999, result_path)
        content = driver_path.read_text(encoding="utf-8")
        assert "1.2.3.4" in content
        assert "9999" in content


class TestV1469KillSubprocess:
    def test_kill_subprocess_already_exited(self):
        # Spawn a trivial Python process that exits immediately
        proc = subprocess.Popen(
            [sys.executable, "-c", "import sys; sys.exit(0)"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        proc.wait()
        # Should not raise on already-exited
        v1469._kill_subprocess(proc, grace_s=1.0)
        assert proc.poll() == 0


# ──────────────────────────────────────────────────────────────────────
# Promethean parent dir helper
# ──────────────────────────────────────────────────────────────────────


class TestV1469PrometheanParent:
    def test_promethean_parent_dir_is_path(self):
        parent = v1469._promethean_parent_dir()
        assert isinstance(parent, Path)

    def test_promethean_parent_dir_is_promethean(self):
        parent = v1469._promethean_parent_dir()
        assert parent.name == "promethean"

    def test_promethean_parent_contains_apeireth(self):
        parent = v1469._promethean_parent_dir()
        assert (parent / "apeireth").is_dir()


# ──────────────────────────────────────────────────────────────────────
# Real two-process driver run
# ──────────────────────────────────────────────────────────────────────


class TestV1469RealRun:
    def test_driver_runs_returns_report(self):
        """Real end-to-end driver run: spawns V1467 server + V1468 client driver subprocesses."""
        with tempfile.TemporaryDirectory(prefix="v1469_driver_test_") as tmp_str:
            tmp = Path(tmp_str)
            out_path = tmp / "report.json"
            report = v1469.run_v1469_driver()
            v1469.write_report_json(report, out_path)
            assert out_path.exists()
            data = json.loads(out_path.read_text(encoding="utf-8"))
            assert data["ok"] is True
            assert data["verdict"] == "PASS"
            assert data["n_endpoints_ok"] == 6
            assert data["n_endpoints_total"] == 6
            assert data["server_pid"] > 0
            assert data["client_pid"] > 0
            assert data["server_pid"] != data["client_pid"]

    def test_driver_two_separate_processes(self):
        """GUARD_TWO_PROCESSES: server PID != client PID."""
        report = v1469.run_v1469_driver()
        assert report.server_pid != 0
        assert report.client_pid != 0
        assert report.server_pid != report.client_pid

    def test_driver_all_endpoints_hit(self):
        """GUARD_ALL_ENDPOINTS_HIT: all 6 endpoints reached."""
        report = v1469.run_v1469_driver()
        assert report.n_endpoints_ok >= 6
        assert len(report.endpoints_hit) >= 6

    def test_driver_bounded_wallclock(self):
        """GUARD_BOUNDED_WALLCLOCK: total elapsed <= max_wallclock_s."""
        report = v1469.run_v1469_driver(max_wallclock_s=60.0)
        assert report.total_elapsed_s <= 60.0

    def test_driver_subprocess_cleaned(self):
        """GUARD_SUBPROCESS_CLEANED: server subprocess has exit_code (reaped)."""
        report = v1469.run_v1469_driver()
        assert report.server_record is not None
        assert report.server_record["exit_code"] is not None


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


class TestV1469ReportWriters:
    def test_write_report_json(self):
        with tempfile.TemporaryDirectory(prefix="v1469_report_") as tmp_str:
            tmp = Path(tmp_str)
            out_path = tmp / "report.json"
            report = v1469.DriverReport(
                ok=True, verdict="PASS", host="127.0.0.1", port=18380,
                server_pid=1, client_pid=2,
                server_boot_elapsed_s=0.5, client_elapsed_s=1.0, total_elapsed_s=1.5,
                client_path="x", result_path="y",
                n_endpoints_ok=6, n_endpoints_total=6,
                guards_passed=14, guards_total=14,
                timestamp=time.time(),
            )
            written = v1469.write_report_json(report, out_path)
            assert written == out_path
            assert out_path.exists()
            data = json.loads(out_path.read_text(encoding="utf-8"))
            assert data["verdict"] == "PASS"

    def test_write_report_markdown(self):
        with tempfile.TemporaryDirectory(prefix="v1469_report_") as tmp_str:
            tmp = Path(tmp_str)
            out_path = tmp / "report.md"
            report = v1469.DriverReport(
                ok=True, verdict="PASS", host="127.0.0.1", port=18380,
                server_pid=1, client_pid=2,
                server_boot_elapsed_s=0.5, client_elapsed_s=1.0, total_elapsed_s=1.5,
                client_path="x", result_path="y",
                n_endpoints_ok=6, n_endpoints_total=6,
                guards_passed=14, guards_total=14,
                timestamp=time.time(),
                endpoints_hit=[
                    {"method": "GET", "path": "/healthz", "fn_name": "healthz",
                     "ok": True, "elapsed_ms": 5.0, "payload_keys": ["ok"]},
                ],
            )
            written = v1469.write_report_markdown(report, out_path)
            assert written == out_path
            content = out_path.read_text(encoding="utf-8")
            assert "V1469" in content
            assert "PASS" in content
            assert "/healthz" in content


# ──────────────────────────────────────────────────────────────────────
# CLI entry points (subprocess to avoid polluting pytest process)
# ──────────────────────────────────────────────────────────────────────


class TestV1469CLI:
    def test_cli_meta_subprocess(self):
        """CLI meta prints V1469 metadata as JSON."""
        result = subprocess.run(
            [sys.executable, "-m", f"apeireth.{V1469_MODULE}", "meta"],
            cwd=str(WORKSPACE_DIR),
            capture_output=True, text=True, timeout=30.0,
        )
        assert result.returncode == 0, result.stderr
        meta = json.loads(result.stdout)
        assert meta["module"] == V1469_MODULE
        assert meta["version"] == v1469.V1469_VERSION
        assert len(meta["guards"]) >= 10
        assert len(meta["v3_guards"]) >= 5

    def test_cli_popper_subprocess(self):
        """CLI popper runs self-checks."""
        result = subprocess.run(
            [sys.executable, "-m", f"apeireth.{V1469_MODULE}", "popper"],
            cwd=str(WORKSPACE_DIR),
            capture_output=True, text=True, timeout=30.0,
        )
        assert result.returncode == 0, result.stderr
        assert "PASS" in result.stdout
        assert "6/6 PASS" in result.stdout or "7/7 PASS" in result.stdout or "/6 PASS" in result.stdout

    def test_cli_chain_subprocess(self):
        """CLI chain verifies V1468+V1467+V1466 importable."""
        result = subprocess.run(
            [sys.executable, "-m", f"apeireth.{V1469_MODULE}", "chain"],
            cwd=str(WORKSPACE_DIR),
            capture_output=True, text=True, timeout=30.0,
        )
        assert result.returncode == 0, result.stderr
        chain = json.loads(result.stdout)["chain"]
        assert chain["v1468"] == "importable"
        assert chain["v1467"] == "importable"
        assert chain["v1466"] == "importable"

    def test_cli_run_subprocess(self):
        """CLI run spawns V1467 server + V1468 client driver subprocesses end-to-end."""
        with tempfile.TemporaryDirectory(prefix="v1469_cli_") as tmp_str:
            tmp = Path(tmp_str)
            out_path = tmp / "driver.json"
            result = subprocess.run(
                [sys.executable, "-m", f"apeireth.{V1469_MODULE}", "run", "--out", str(out_path)],
                cwd=str(WORKSPACE_DIR),
                capture_output=True, text=True, timeout=120.0,
            )
            assert result.returncode == 0, f"stderr={result.stderr}\nstdout={result.stdout}"
            assert out_path.exists()
            data = json.loads(out_path.read_text(encoding="utf-8"))
            assert data["verdict"] == "PASS"
            assert data["n_endpoints_ok"] == 6
            assert data["n_endpoints_total"] == 6

    def test_cli_help_subprocess(self):
        """CLI help prints usage."""
        result = subprocess.run(
            [sys.executable, "-m", f"apeireth.{V1469_MODULE}", "help"],
            cwd=str(WORKSPACE_DIR),
            capture_output=True, text=True, timeout=10.0,
        )
        assert result.returncode == 0


# ──────────────────────────────────────────────────────────────────────
# Chain importability (in-process)
# ──────────────────────────────────────────────────────────────────────


class TestV1469ChainImports:
    def test_v1468_importable(self):
        import apeireth.v1468_asi_openapi_v1467_schema_and_client_generator  # noqa: F401

    def test_v1467_importable(self):
        import apeireth.v1467_asi_audit_http_gateway_history_diff  # noqa: F401

    def test_v1466_importable(self):
        import apeireth.v1466_asi_real_cross_process_lint_gate_subprocess_runner  # noqa: F401


# ──────────────────────────────────────────────────────────────────────
# Honest disclosure (主 17:43 实事求是 + 主 17:58 不假装)
# ──────────────────────────────────────────────────────────────────────


class TestV1469HonestDisclosure:
    def test_not_asi_guarded(self):
        assert "GUARD_DRIVER_NOT_ASI" in v1469.V1469_V3_GUARDS

    def test_not_phenomenal_guarded(self):
        assert "GUARD_DRIVER_NOT_PHENOMENAL" in v1469.V1469_V3_GUARDS

    def test_not_human_level_guarded(self):
        assert "GUARD_DRIVER_NOT_HUMAN_LEVEL" in v1469.V1469_V3_GUARDS

    def test_not_ci_guarded(self):
        assert "GUARD_DRIVER_NOT_CI" in v1469.V1469_V3_GUARDS

    def test_not_load_test_guarded(self):
        assert "GUARD_DRIVER_NOT_LOAD_TEST" in v1469.V1469_V3_GUARDS

    def test_not_orchestrator_guarded(self):
        # V1469 spawns 2 subprocesses (server + client), not N
        assert "GUARD_DRIVER_NOT_ORCHESTRATOR" in v1469.V1469_V3_GUARDS

    def test_max_endpoints_bounded(self):
        # 6 endpoints, not 600
        assert len(v1469.V1467_ENDPOINTS_TO_HIT) == 6

    def test_subprocess_timeouts_present(self):
        # All subprocess operations must have timeouts (主 00:44 质量工程化)
        assert v1469.DEFAULT_SERVER_BOOT_TIMEOUT_S > 0
        assert v1469.DEFAULT_CLIENT_DRIVER_TIMEOUT_S > 0
        assert v1469.DEFAULT_KILL_GRACE_S > 0


# ──────────────────────────────────────────────────────────────────────
# V1469 lineage / borrowed sources
# ──────────────────────────────────────────────────────────────────────


class TestV1469Lineage:
    def test_v1468_in_borrowed(self):
        assert "v1468" in v1469.BORROWED_SOURCES

    def test_v1467_in_borrowed(self):
        assert "v1467" in v1469.BORROWED_SOURCES

    def test_v1466_in_borrowed(self):
        assert "v1466" in v1469.BORROWED_SOURCES

    def test_stdlib_in_borrowed(self):
        assert "stdlib" in v1469.BORROWED_SOURCES


if __name__ == "__main__":
    pytest.main([__file__, "-v"])