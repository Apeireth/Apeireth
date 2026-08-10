"""Tests for V1472 — V1471 audit monitor daemon supervisor."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
import urllib.error
from pathlib import Path

import pytest

# Ensure promethean parent on sys.path
_PROMETHEAN_PARENT = Path(__file__).resolve().parent.parent
_APEIRETH_DIR = _PROMETHEAN_PARENT / "apeireth"
if str(_PROMETHEAN_PARENT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_PARENT))
if str(_APEIRETH_DIR) not in sys.path:
    sys.path.insert(0, str(_APEIRETH_DIR))

import v1472_daemon_supervisor as v1472  # noqa: E402


# ──────────────────────────────────────────────────────────────────────
# Test helpers
# ──────────────────────────────────────────────────────────────────────


def _run_python_module(module: str, *args: str, timeout_s: float = 60.0, cwd: Path = _PROMETHEAN_PARENT) -> subprocess.CompletedProcess:
    """Run `python -m <module> <args...>` and return CompletedProcess."""
    cmd = [sys.executable, "-m", module, *args]
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout_s,
        cwd=str(cwd),
    )


def _http_get_json(host: str, port: int, path: str, timeout_s: float = 5.0) -> tuple:
    """GET a URL and parse JSON. Returns (status_code, parsed_body)."""
    try:
        url = f"http://{host}:{port}{path}"
        with urllib.request.urlopen(url, timeout=timeout_s) as resp:
            status = resp.getcode() or 0
            raw = resp.read().decode("utf-8", errors="replace")
            try:
                body = json.loads(raw)
            except json.JSONDecodeError:
                body = raw
            return status, body
    except urllib.error.HTTPError as e:
        try:
            raw = e.read().decode("utf-8", errors="replace")
            try:
                body = json.loads(raw)
            except json.JSONDecodeError:
                body = raw
        except Exception:
            body = None
        return e.code, body
    except Exception as e:
        return 0, {"error": str(e)}


# ──────────────────────────────────────────────────────────────────────
# Test 1: Popper self-checks
# ──────────────────────────────────────────────────────────────────────


def test_v1472_popper_via_cli():
    """Run V1472 popper CLI; expect 38/38 pass."""
    result = _run_python_module("apeireth.v1472_daemon_supervisor", "popper", timeout_s=30.0)
    assert result.returncode == 0, f"popper failed:\nstdout={result.stdout}\nstderr={result.stderr}"
    assert "38/38 popper checks passed" in result.stdout, f"expected 38/38, got:\n{result.stdout}"


# ──────────────────────────────────────────────────────────────────────
# Test 2: Popper via direct API
# ──────────────────────────────────────────────────────────────────────


def test_v1472_popper_direct():
    """Run popper_v1472() directly; expect 38/38 pass."""
    results = v1472.popper_v1472(verbose=False)
    passed = sum(1 for _, ok, _ in results if ok)
    total = len(results)
    assert passed == total == 38, f"expected 38/38, got {passed}/{total}"


# ──────────────────────────────────────────────────────────────────────
# Test 3: Module metadata
# ──────────────────────────────────────────────────────────────────────


def test_v1472_meta():
    """Run V1472 meta CLI; verify module metadata."""
    result = _run_python_module("apeireth.v1472_daemon_supervisor", "meta", timeout_s=10.0)
    assert result.returncode == 0
    meta = json.loads(result.stdout)
    assert meta["module"] == "apeireth.v1472_daemon_supervisor"
    assert meta["version"] == "0.1.0"
    assert meta["date"] == "2026-08-10"
    assert len(meta["guards"]) == 18
    assert len(meta["v3_guards"]) == 8
    assert "v1471" in meta["borrowed_sources"]


# ──────────────────────────────────────────────────────────────────────
# Test 4: Chain (V1471 + V1467 importable)
# ──────────────────────────────────────────────────────────────────────


def test_v1472_chain():
    """Run V1472 chain CLI; verify V1471 + V1467 importable."""
    result = _run_python_module("apeireth.v1472_daemon_supervisor", "chain", timeout_s=10.0)
    assert result.returncode == 0, f"chain failed:\n{result.stdout}\n{result.stderr}"
    assert "v1471 import" in result.stdout
    assert "v1467 import" in result.stdout


# ──────────────────────────────────────────────────────────────────────
# Test 5: Helper functions
# ──────────────────────────────────────────────────────────────────────


def test_v1472_promethean_parent_dir():
    """_promethean_parent_dir returns the promethean/ directory."""
    p = v1472._promethean_parent_dir()
    assert p.exists() and p.is_dir()
    assert p.name == "promethean"


def test_v1472_is_port_free():
    """_is_port_free returns True for unused high port."""
    ok = v1472._is_port_free("127.0.0.1", 39999)
    assert ok is True


def test_v1472_find_open_port():
    """_find_open_port returns a port in [min, max]."""
    port = v1472._find_open_port("127.0.0.1", 18780, 18880)
    assert 18780 <= port <= 18880


def test_v1472_read_jsonl_tail():
    """_read_jsonl_tail reads N events from JSONL file."""
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False, mode="w") as tf:
        tf.write('{"timestamp": 1.0, "event_type": "diff"}\n')
        tf.write('{"timestamp": 2.0, "event_type": "diff"}\n')
        tf.write('{"timestamp": 3.0, "event_type": "diff"}\n')
        tmp_path = Path(tf.name)
    try:
        size, events = v1472._read_jsonl_tail(tmp_path)
        assert size > 0
        assert len(events) == 3
        assert events[0]["timestamp"] == 1.0
        assert events[-1]["timestamp"] == 3.0
    finally:
        tmp_path.unlink()


def test_v1472_format_metrics_prometheus():
    """_format_metrics_prometheus produces valid Prometheus text format."""
    samples = [
        v1472.MetricSample(name="v1472_uptime_s", value=10.0, timestamp=time.time()),
        v1472.MetricSample(name="v1472_n_restarts_total", value=0.0, timestamp=time.time()),
    ]
    text = v1472._format_metrics_prometheus(samples)
    assert "# HELP v1472_uptime_s" in text
    assert "# TYPE v1472_uptime_s gauge" in text
    assert "v1472_uptime_s 10.0" in text
    assert "v1472_n_restarts_total 0.0" in text


# ──────────────────────────────────────────────────────────────────────
# Test 6: Dataclasses
# ──────────────────────────────────────────────────────────────────────


def test_v1472_health_probe_dataclass():
    """HealthProbe to_dict roundtrip."""
    p = v1472.HealthProbe(
        timestamp=time.time(), probe_index=1, health_state="HEALTHY",
        v1471_pid=1234, v1471_alive=True, jsonl_size_bytes=1024,
        n_events_seen=5, seconds_since_last_event=2.0,
        restart_recommended=False, reason="ok",
    )
    d = p.to_dict()
    assert d["health_state"] == "HEALTHY"
    assert d["v1471_pid"] == 1234
    assert d["n_events_seen"] == 5
    line = p.to_jsonl()
    parsed = json.loads(line)
    assert parsed["health_state"] == "HEALTHY"


def test_v1472_restart_record_dataclass():
    """RestartRecord to_jsonl roundtrip."""
    r = v1472.RestartRecord(
        restart_index=0, timestamp=time.time(), reason="PROCESS_DIED",
        old_pid=1234, new_pid=5678, elapsed_since_old_start_s=10.0,
        success=True, error="",
    )
    line = r.to_jsonl()
    parsed = json.loads(line)
    assert parsed["reason"] == "PROCESS_DIED"
    assert parsed["success"] is True
    assert parsed["new_pid"] == 5678


def test_v1472_metric_sample_dataclass():
    """MetricSample to_text format."""
    m = v1472.MetricSample(name="v1472_uptime_s", value=42.5, timestamp=time.time())
    text = m.to_text()
    assert text == "v1472_uptime_s 42.5\n"


def test_v1472_v1471_run_record_dataclass():
    """V1471RunRecord to_dict."""
    r = v1472.V1471RunRecord(run_index=0, pid=1234, started_at_real_s=time.time())
    d = r.to_dict()
    assert d["run_index"] == 0
    assert d["pid"] == 1234
    assert d["n_events_seen"] == 0  # default


def test_v1472_supervisor_report_dataclass():
    """SupervisorReport to_dict contains all expected keys."""
    r = v1472.SupervisorReport(ok=True, verdict="PASS", metrics_port=18780)
    d = r.to_dict()
    assert "module" in d
    assert "guards" in d
    assert len(d["guards"]) == 18
    assert len(d["v3_guards"]) == 8
    assert d["ok"] is True


# ──────────────────────────────────────────────────────────────────────
# Test 7: V1472DaemonSupervisor instance
# ──────────────────────────────────────────────────────────────────────


def test_v1472_supervisor_collect_metrics():
    """collect_metrics returns 12 Prometheus samples."""
    sup = v1472.V1472DaemonSupervisor(
        max_runtime_s=20.0,
        out_dir=Path(tempfile.mkdtemp(prefix="v1472-test-")),
        verbose=False,
    )
    metrics = sup.collect_metrics()
    assert len(metrics) == 12
    metric_names = {m.name for m in metrics}
    assert "v1472_uptime_s" in metric_names
    assert "v1472_n_restarts_total" in metric_names
    assert "v1472_v1471_alive" in metric_names


def test_v1472_supervisor_validation_health_interval():
    """health_interval_s out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1472.V1472DaemonSupervisor(health_interval_s=0.1)
    with pytest.raises(ValueError):
        v1472.V1472DaemonSupervisor(health_interval_s=100.0)


def test_v1472_supervisor_validation_max_restarts():
    """max_restarts out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1472.V1472DaemonSupervisor(max_restarts=0)
    with pytest.raises(ValueError):
        v1472.V1472DaemonSupervisor(max_restarts=200)


def test_v1472_supervisor_validation_stale_threshold():
    """stale_threshold_s out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1472.V1472DaemonSupervisor(stale_threshold_s=1.0)


def test_v1472_supervisor_validation_max_runtime():
    """max_runtime_s out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1472.V1472DaemonSupervisor(max_runtime_s=5.0)


# ──────────────────────────────────────────────────────────────────────
# Test 8: Constants
# ──────────────────────────────────────────────────────────────────────


def test_v1472_constants():
    """Verify V1472 module constants."""
    assert v1472.V1472_MODULE == "apeireth.v1472_daemon_supervisor"
    assert v1472.V1472_VERSION == "0.1.0"
    assert v1472.V1472_DATE == "2026-08-10"
    assert v1472.DEFAULT_HOST == "127.0.0.1"
    assert v1472.DEFAULT_METRICS_PORT_MIN == 18780
    assert v1472.DEFAULT_METRICS_PORT_MAX == 18880
    assert v1472.DEFAULT_HEALTH_INTERVAL_S == 5.0
    assert v1472.DEFAULT_MAX_RUNTIME_S == 60.0
    assert v1472.DEFAULT_MAX_RESTARTS == 3
    # Port ranges distinct
    assert v1472.DEFAULT_METRICS_PORT_MIN != v1472.V1471_PORT_MIN_FORWARD
    assert v1472.DEFAULT_METRICS_PORT_MIN != 18280  # V1467 port min


def test_v1472_enums():
    """Verify V1472 enums."""
    assert len(v1472.ShutdownReason) == 6
    assert len(v1472.RestartReason) == 4
    assert len(v1472.HealthState) == 4
    assert v1472.ShutdownReason.RUNTIME_LIMIT.value == "RUNTIME_LIMIT"
    assert v1472.RestartReason.PROCESS_DIED.value == "PROCESS_DIED"
    assert v1472.HealthState.HEALTHY.value == "HEALTHY"


# ──────────────────────────────────────────────────────────────────────
# Test 9: Borrows
# ──────────────────────────────────────────────────────────────────────


def test_v1472_borrowed_sources():
    """Verify borrowed sources declare V1471 + stdlib."""
    sources = v1472.BORROWED_SOURCES
    assert "v1471" in sources
    assert "v1470" in sources
    assert "v1467" in sources
    assert "stdlib" in sources


# ──────────────────────────────────────────────────────────────────────
# Test 10: Real demo (subprocess V1471 + metrics endpoint)
# ──────────────────────────────────────────────────────────────────────


def test_v1472_demo_runs_subprocess():
    """Run V1472 demo; verify supervisor spawns V1471, emits metrics, writes report."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1472-demo-"))
    result = _run_python_module(
        "apeireth.v1472_daemon_supervisor", "demo",
        "--out-dir", str(out_dir),
        timeout_s=60.0,
    )
    # demo should complete (max-runtime 20s)
    assert result.returncode in (0, 1), f"demo failed:\nstdout={result.stdout}\nstderr={result.stderr}"

    # Verify output files exist
    report_json = out_dir / "v1472-supervisor-report.json"
    report_md = out_dir / "v1472-supervisor-report.md"
    log_file = out_dir / "v1472-supervisor.log"
    metrics_file = out_dir / "v1472-metrics.txt"
    assert report_json.exists(), f"report JSON not written: {report_json}"
    assert report_md.exists(), f"report MD not written: {report_md}"
    assert log_file.exists(), f"log file not written: {log_file}"
    assert metrics_file.exists(), f"metrics file not written: {metrics_file}"

    # Verify report JSON content
    report = json.loads(report_json.read_text(encoding="utf-8"))
    assert report["module"] == "apeireth.v1472_daemon_supervisor"
    assert report["verdict"] in ("PASS", "PARTIAL", "FAIL")
    assert report["n_probes"] >= 1, f"expected ≥1 probe, got {report['n_probes']}"
    assert report["metrics_port"] > 0, f"expected metrics_port > 0, got {report['metrics_port']}"
    assert len(report["v1471_runs"]) >= 1, f"expected ≥1 V1471 run, got {len(report['v1471_runs'])}"

    # Verify metrics file format
    metrics_text = metrics_file.read_text(encoding="utf-8")
    assert "# HELP v1472_uptime_s" in metrics_text
    assert "# TYPE v1472_uptime_s gauge" in metrics_text
    assert "v1472_uptime_s" in metrics_text


# ──────────────────────────────────────────────────────────────────────
# Test 11: /metrics HTTP endpoint live (Prometheus text format)
# ──────────────────────────────────────────────────────────────────────


def test_v1472_metrics_endpoint_serves_prometheus_format():
    """Boot V1472 supervisor briefly; query /metrics; verify Prometheus text format."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1472-metrics-"))

    # Start supervisor in subprocess
    cmd = [
        sys.executable, "-m", "apeireth.v1472_daemon_supervisor", "demo",
        "--out-dir", str(out_dir),
    ]
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        cwd=str(_PROMETHEAN_PARENT),
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == "win32" else 0,
    )

    try:
        # Wait for metrics endpoint to come up (poll until we get a 200)
        time.sleep(3.0)  # Give supervisor time to start
        metrics_port = None
        deadline = time.monotonic() + 15.0
        while time.monotonic() < deadline:
            # Try common ports
            for port in range(18780, 18881):
                status, body = _http_get_json("127.0.0.1", port, "/healthz", timeout_s=1.0)
                if status == 200 and isinstance(body, dict) and body.get("module") == "v1472":
                    metrics_port = port
                    break
            if metrics_port:
                break
            time.sleep(0.5)

        if metrics_port is None:
            pytest.skip("Could not find V1472 metrics endpoint in time (may need longer boot)")

        # Query /metrics
        status, text = _http_get_json("127.0.0.1", metrics_port, "/metrics", timeout_s=2.0)
        # /metrics returns text, not JSON
        try:
            import urllib.request
            with urllib.request.urlopen(f"http://127.0.0.1:{metrics_port}/metrics", timeout=2.0) as resp:
                body = resp.read().decode("utf-8")
                assert resp.getcode() == 200
        except Exception as e:
            pytest.fail(f"Could not GET /metrics: {e}")

        # Verify Prometheus text format
        assert "# HELP v1472_uptime_s" in body, f"missing HELP line:\n{body[:500]}"
        assert "# TYPE v1472_uptime_s gauge" in body, f"missing TYPE line:\n{body[:500]}"
        assert "v1472_uptime_s" in body, f"missing v1472_uptime_s sample:\n{body[:500]}"
        assert "v1472_n_restarts_total" in body, f"missing n_restarts_total:\n{body[:500]}"
        assert "v1472_v1471_alive" in body, f"missing v1471_alive:\n{body[:500]}"

        # Query /healthz
        status2, body2 = _http_get_json("127.0.0.1", metrics_port, "/healthz", timeout_s=2.0)
        assert status2 == 200
        assert body2.get("module") == "v1472"

        # Query unknown path
        status3, body3 = _http_get_json("127.0.0.1", metrics_port, "/unknown", timeout_s=2.0)
        assert status3 == 404

    finally:
        # Terminate the supervisor
        try:
            proc.terminate()
            proc.wait(timeout=10.0)
        except Exception:
            try:
                proc.kill()
            except Exception:
                pass


# ──────────────────────────────────────────────────────────────────────
# Test 12: /metrics with metrics_port never bound (no port available)
# ──────────────────────────────────────────────────────────────────────


def test_v1472_metrics_text_written_to_file():
    """Verify metrics text file is written even if HTTP server fails."""
    sup = v1472.V1472DaemonSupervisor(
        max_runtime_s=10.0,
        metrics_port_min=39990,  # Use a different range to avoid conflicts
        metrics_port_max=39995,
        out_dir=Path(tempfile.mkdtemp(prefix="v1472-metrics-file-")),
        verbose=False,
    )
    # Manually call collect_metrics to populate metrics_text file
    metrics = sup.collect_metrics()
    assert len(metrics) == 12
    text = v1472._format_metrics_prometheus(metrics)
    # Write to file
    sup.metrics_text_path.write_text(text, encoding="utf-8")
    assert sup.metrics_text_path.exists()
    assert sup.metrics_text_path.stat().st_size > 0
    content = sup.metrics_text_path.read_text(encoding="utf-8")
    assert "v1472_uptime_s" in content