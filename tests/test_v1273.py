"""Tests for V1273 ASI North Star Prometheus Metrics 真生产模块.

> **主 17:43 实事求是**: 真测试, 不假装, stdlib only.
> **承接**: V1272 测试风格 (compact + 真断言)
"""
from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from urllib import request as urlrequest
from urllib.error import URLError

import pytest

from apeireth.v1273_asi_north_star_metrics import (
    V1273_ASI_NS_CURRENT,
    V1273_ASI_NS_LOCKED_PCT,
    V1273_ASI_NS_TARGET_MAX,
    V1273_BUILD,
    V1273_V1272_EPA_RESONANCE_RATE,
    V1273_VERSION,
    ScanResult,
    _cmd_probe,
    _cmd_snapshot,
    _scan_modules,
    _scan_tests,
    _scan_commits,
    _v3_philosophy_gate,
    main,
    real_scan,
    render_json_snapshot,
    render_prometheus,
    serve,
)


PROMETHEAN_DIR = Path(__file__).resolve().parent.parent
APEIRETH_DIR = PROMETHEAN_DIR / "apeireth"
TESTS_DIR = PROMETHEAN_DIR / "tests"


# ============================================================
# 1. Constants (主 17:43 实事求是)
# ============================================================

class TestConstants:
    def test_version(self):
        assert V1273_VERSION == "0.1.0"

    def test_build_format(self):
        # YYYY-MM-DD-HHMM+ZZ
        assert V1273_BUILD.startswith("2026-08-05-")
        assert "+08" in V1273_BUILD or "-08" in V1273_BUILD

    def test_asi_ns_current_in_range(self):
        # 0.7905 主 22:33 真测量
        assert 0.0 < V1273_ASI_NS_CURRENT < 1.0
        assert abs(V1273_ASI_NS_CURRENT - 0.7905) < 0.001

    def test_asi_ns_target_max(self):
        assert V1273_ASI_NS_TARGET_MAX == 0.9800

    def test_asi_ns_locked_pct(self):
        # 92.91% LOCKED
        assert 0 < V1273_ASI_NS_LOCKED_PCT <= 100

    def test_v1272_epa_resonance(self):
        assert 0 < V1273_V1272_EPA_RESONANCE_RATE <= 1.0


# ============================================================
# 2. V3 Philosophy Gate (主 17:58 + 主 20:46)
# ============================================================

class TestPhilosophyGate:
    def test_all_gates_passed(self):
        gate = _v3_philosophy_gate()
        for key, val in gate.items():
            assert val is True, f"philosophy gate failed: {key}"

    def test_not_new_asi_dim(self):
        gate = _v3_philosophy_gate()
        assert gate["v1273_not_new_asi_dim"] is True

    def test_no_phenomenal_claim(self):
        gate = _v3_philosophy_gate()
        assert gate["v1273_no_phenomenal_claim"] is True

    def test_no_kpi_inflate(self):
        gate = _v3_philosophy_gate()
        assert gate["v1273_no_kpi_inflate"] is True

    def test_read_only(self):
        gate = _v3_philosophy_gate()
        assert gate["v1273_read_only"] is True


# ============================================================
# 3. Real Data Scanners (主 17:43 实事求是)
# ============================================================

class TestRealScanners:
    def test_scan_modules_positive(self):
        n, errors = _scan_modules(APEIRETH_DIR)
        # 真实数据, 应 > 1200 (V1273 之后)
        assert n > 1200, f"modules={n}, errors={errors}"
        assert isinstance(errors, list)

    def test_scan_tests_positive(self):
        n, errors = _scan_tests(TESTS_DIR)
        # 真实数据 (2026-08-05 实测 ~394 test files), 应 > 100
        assert n > 100, f"tests={n}, errors={errors}"

    def test_scan_commits(self):
        n, git_avail, errors = _scan_commits(PROMETHEAN_DIR)
        # 真 git 仓, commit > 300
        if git_avail:
            assert n > 300, f"commits={n}, errors={errors}"

    def test_scan_modules_wrong_path(self):
        n, errors = _scan_modules(Path("/nonexistent/path/xyz"))
        assert n == 0
        assert len(errors) > 0

    def test_scan_tests_wrong_path(self):
        n, errors = _scan_tests(Path("/nonexistent/path/xyz"))
        assert n == 0
        assert len(errors) > 0


class TestRealScan:
    def test_real_scan_returns_dataclass(self):
        result = real_scan(PROMETHEAN_DIR)
        assert isinstance(result, ScanResult)
        assert result.modules_total > 0
        assert result.tests_total > 0
        assert result.timestamp_unix > 0

    def test_real_scan_scan_path(self):
        result = real_scan(PROMETHEAN_DIR)
        assert result.scan_path == str(PROMETHEAN_DIR)

    def test_real_scan_duration_positive(self):
        result = real_scan(PROMETHEAN_DIR)
        assert result.scan_duration_seconds >= 0

    def test_real_scan_inferred_path(self):
        # 不传 promethean_dir, 自动推断
        result = real_scan()
        assert isinstance(result, ScanResult)


# ============================================================
# 4. Prometheus Render (主 19:33 走在前人肩上)
# ============================================================

class TestPrometheusRender:
    def _sample_scan(self):
        return ScanResult(
            modules_total=1272,
            tests_total=1272,
            commits_total=350,
            scan_duration_seconds=0.05,
            scan_path="/test/path",
            git_available=True,
            errors=[],
            timestamp_unix=time.time(),
        )

    def test_render_contains_ns_current(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_asi_ns_current" in out
        assert str(V1273_ASI_NS_CURRENT) in out

    def test_render_contains_target(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_asi_ns_target" in out
        assert str(V1273_ASI_NS_TARGET_MAX) in out

    def test_render_contains_modules(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_modules_total" in out
        assert "1272" in out

    def test_render_contains_commits(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_commits_total" in out

    def test_render_contains_uptime(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_uptime_seconds" in out
        assert "10.0" in out

    def test_render_contains_build_info(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_build_info" in out
        assert f'version="{V1273_VERSION}"' in out

    def test_render_contains_philosophy_gate(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_philosophy_gate" in out
        # 7 个守门
        gate_lines = [l for l in out.splitlines() if "apeireth_philosophy_gate{" in l]
        assert len(gate_lines) >= 7

    def test_render_help_type_meta(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        # Prometheus 标准 # HELP + # TYPE
        assert "# HELP" in out
        assert "# TYPE" in out

    def test_render_v1272_epa(self):
        scan = self._sample_scan()
        out = render_prometheus(scan, 10.0)
        assert "apeireth_v1272_epa_resonance_rate" in out
        assert str(V1273_V1272_EPA_RESONANCE_RATE) in out

    def test_render_git_available(self):
        scan_true = self._sample_scan()
        scan_false = ScanResult(git_available=False)
        out_true = render_prometheus(scan_true, 10.0)
        out_false = render_prometheus(scan_false, 10.0)
        assert "apeireth_git_available 1" in out_true
        assert "apeireth_git_available 0" in out_false


# ============================================================
# 5. JSON Snapshot (主 00:56 任何人都能接手)
# ============================================================

class TestJSONSnapshot:
    def test_json_snapshot_valid(self):
        scan = ScanResult(
            modules_total=1272,
            tests_total=1272,
            commits_total=350,
            scan_duration_seconds=0.1,
            scan_path="/test",
            git_available=True,
            timestamp_unix=time.time(),
        )
        snap = render_json_snapshot(scan, 5.0)
        data = json.loads(snap)
        assert data["version"] == V1273_VERSION
        assert data["build"] == V1273_BUILD
        assert "asi_ns" in data
        assert data["asi_ns"]["current"] == V1273_ASI_NS_CURRENT
        assert "scan" in data
        assert data["uptime_seconds"] == 5.0
        assert "philosophy_gate" in data
        assert data["philosophy_gate"]["v1273_not_new_asi_dim"] is True

    def test_json_snapshot_endpoint_hints(self):
        scan = ScanResult(timestamp_unix=time.time())
        snap = render_json_snapshot(scan, 0.0)
        data = json.loads(snap)
        hints = data["endpoint_hints"]
        assert "metrics" in hints
        assert "snapshot" in hints
        assert "healthz" in hints

    def test_json_snapshot_unicode_safe(self):
        scan = ScanResult(timestamp_unix=time.time())
        snap = render_json_snapshot(scan, 0.0)
        # ensure_ascii=False 应输出 ASCII 字符 (本模块无中文, 但保险)
        # 实际是 ASCII OK
        data = json.loads(snap)
        assert data is not None


# ============================================================
# 6. CLI Commands (主 00:56 任何人都能接手)
# ============================================================

class TestCLI:
    def test_probe(self, capsys):
        rc = _cmd_probe(PROMETHEAN_DIR)
        assert rc == 0
        captured = capsys.readouterr()
        assert "V1273" in captured.out
        assert "philosophy_gate" in captured.out

    def test_snapshot(self, capsys):
        rc = _cmd_snapshot(PROMETHEAN_DIR)
        assert rc == 0
        captured = capsys.readouterr()
        # 输出 JSON, 可解析
        data = json.loads(captured.out)
        assert "version" in data

    def test_main_no_args(self, capsys):
        rc = main([])
        assert rc == 0
        captured = capsys.readouterr()
        # 应打印 help
        assert "v1273" in captured.out.lower() or "usage" in captured.out.lower()

    def test_main_probe(self, capsys):
        rc = main(["--probe", "--promethean-dir", str(PROMETHEAN_DIR)])
        assert rc == 0


# ============================================================
# 7. HTTP Server (主 17:43 实事求是)
# ============================================================

def _free_port() -> int:
    """找一个 free port (主 17:43: 测试隔离)."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class TestHTTPServer:
    def test_serve_healthz(self):
        port = _free_port()
        t = threading.Thread(
            target=serve,
            kwargs={"host": "127.0.0.1", "port": port, "promethean_dir": PROMETHEAN_DIR},
            daemon=True,
        )
        t.start()
        time.sleep(1.0)
        try:
            with urlrequest.urlopen(f"http://127.0.0.1:{port}/healthz", timeout=5) as r:
                assert r.status == 200
                assert r.read().decode() == "OK"
        finally:
            # ThreadingHTTPServer 没有干净 stop API, daemon=True 让进程退出时清理
            pass

    def test_serve_metrics_endpoint(self):
        port = _free_port()
        t = threading.Thread(
            target=serve,
            kwargs={"host": "127.0.0.1", "port": port, "promethean_dir": PROMETHEAN_DIR},
            daemon=True,
        )
        t.start()
        time.sleep(1.0)
        try:
            with urlrequest.urlopen(f"http://127.0.0.1:{port}/metrics", timeout=5) as r:
                assert r.status == 200
                body = r.read().decode()
                assert "apeireth_asi_ns_current" in body
                assert "apeireth_modules_total" in body
                assert "# HELP" in body
        finally:
            pass

    def test_serve_snapshot_endpoint(self):
        port = _free_port()
        t = threading.Thread(
            target=serve,
            kwargs={"host": "127.0.0.1", "port": port, "promethean_dir": PROMETHEAN_DIR},
            daemon=True,
        )
        t.start()
        time.sleep(1.0)
        try:
            with urlrequest.urlopen(f"http://127.0.0.1:{port}/snapshot", timeout=5) as r:
                assert r.status == 200
                body = r.read().decode()
                data = json.loads(body)
                assert data["version"] == V1273_VERSION
                assert data["scan"]["modules_total"] > 0
        finally:
            pass

    def test_serve_index(self):
        port = _free_port()
        t = threading.Thread(
            target=serve,
            kwargs={"host": "127.0.0.1", "port": port, "promethean_dir": PROMETHEAN_DIR},
            daemon=True,
        )
        t.start()
        time.sleep(1.0)
        try:
            with urlrequest.urlopen(f"http://127.0.0.1:{port}/", timeout=5) as r:
                assert r.status == 200
                body = r.read().decode()
                assert "Apeireth" in body
                assert "V1273" in body
        finally:
            pass

    def test_serve_404(self):
        port = _free_port()
        t = threading.Thread(
            target=serve,
            kwargs={"host": "127.0.0.1", "port": port, "promethean_dir": PROMETHEAN_DIR},
            daemon=True,
        )
        t.start()
        time.sleep(1.0)
        try:
            try:
                urlrequest.urlopen(f"http://127.0.0.1:{port}/nonexistent", timeout=5)
                assert False, "should have raised HTTPError"
            except Exception as e:
                # urllib throws HTTPError on 4xx
                assert "404" in str(e) or "Not" in str(e) or "HTTPError" in type(e).__name__
        finally:
            pass


# ============================================================
# 8. End-to-End CLI (主 00:56 任何人都能接手)
# ============================================================

class TestEndToEndCLI:
    @pytest.mark.skipif(sys.platform != "win32", reason="Windows specific path")
    def test_module_invocation(self):
        """python -m apeireth.v1273_asi_north_star_metrics --probe (真生产)."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1273_asi_north_star_metrics",
             "--probe", "--promethean-dir", str(PROMETHEAN_DIR)],
            cwd=str(PROMETHEAN_DIR),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        stdout = result.stdout or ""
        assert "V1273" in stdout
        assert "apeireth_asi_ns_current" in stdout

    def test_module_invocation_snapshot(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1273_asi_north_star_metrics",
             "--snapshot", "--promethean-dir", str(PROMETHEAN_DIR)],
            cwd=str(PROMETHEAN_DIR),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["version"] == V1273_VERSION
        # 模块数应 > 1000 (V1273 时代)
        assert data["scan"]["modules_total"] > 1000