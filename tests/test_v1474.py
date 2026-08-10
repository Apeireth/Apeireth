"""Tests for V1474 — V1473 multi-stream alert aggregator + cross-stream incident correlation."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

import pytest

# Ensure promethean parent on sys.path
_PROMETHEAN_PARENT = Path(__file__).resolve().parent.parent
_APEIRETH_DIR = _PROMETHEAN_PARENT / "apeireth"
if str(_PROMETHEAN_PARENT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_PARENT))
if str(_APEIRETH_DIR) not in sys.path:
    sys.path.insert(0, str(_APEIRETH_DIR))

import v1474_asi_v1473_multi_stream_aggregator as v1474  # noqa: E402
import apeireth.v1473_asi_v1472_alerting_engine as v1473  # noqa: E402


# ──────────────────────────────────────────────────────────────────────
# Test helpers
# ──────────────────────────────────────────────────────────────────────


def _run_python_module(module: str, *args: str, timeout_s: float = 60.0, cwd: Path = _PROMETHEAN_PARENT) -> subprocess.CompletedProcess:
    """Run `python -m <module> <args...>` with explicit UTF-8 encoding (avoid Windows GBK None-stdout bug)."""
    cmd = [sys.executable, "-m", module, *args]
    return subprocess.run(
        cmd,
        capture_output=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout_s,
        cwd=str(cwd),
    )


def _http_get_json(host: str, port: int, path: str, timeout_s: float = 5.0):
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


def _write_jsonl(path: Path, events: list) -> None:
    """Write a list of dicts as JSONL."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for ev in events:
            f.write(json.dumps(ev, separators=(",", ":")) + "\n")


# ──────────────────────────────────────────────────────────────────────
# Test 1: Popper self-checks (CLI)
# ──────────────────────────────────────────────────────────────────────


def test_v1474_popper_via_cli():
    """Run V1474 popper CLI; expect 34/34 pass."""
    result = _run_python_module("apeireth.v1474_asi_v1473_multi_stream_aggregator", "popper", timeout_s=30.0)
    assert result.returncode == 0, f"popper failed:\nstdout={result.stdout}\nstderr={result.stderr}"
    assert "34/34" in result.stdout, f"expected 34/34, got:\n{result.stdout}"


# ──────────────────────────────────────────────────────────────────────
# Test 2: Popper direct API
# ──────────────────────────────────────────────────────────────────────


def test_v1474_popper_direct():
    """Run popper_v1474() directly; expect 34/34 pass."""
    results = v1474.popper_v1474(verbose=False)
    passed = sum(1 for _, ok, _ in results if ok)
    total = len(results)
    assert passed == total == 34, f"expected 34/34, got {passed}/{total}"


# ──────────────────────────────────────────────────────────────────────
# Test 3: Module constants
# ──────────────────────────────────────────────────────────────────────


def test_v1474_constants():
    """Verify V1474 module constants."""
    assert v1474.MODULE_NAME == "v1474_asi_v1473_multi_stream_aggregator"
    assert v1474.MODULE_PHASE == 1474
    assert v1474.MODULE_VERSION == "0.1.0"
    assert v1474.DEFAULT_HOST == "127.0.0.1"
    assert v1474.DEFAULT_DIGEST_PORT_MIN == 19180
    assert v1474.DEFAULT_DIGEST_PORT_MAX == 19280
    # Port ranges distinct from V1473 (18980), V1472 (18780), V1471 (18580)
    assert v1474.DEFAULT_DIGEST_PORT_MIN > 19080
    assert v1474.DEFAULT_DIGEST_PORT_MIN > 18880
    assert v1474.DEFAULT_DIGEST_PORT_MIN > 18680


def test_v1474_enums():
    """Verify V1474 enums."""
    assert len(v1474.FleetIncidentState) == 3
    assert len(v1474.FleetShutdownReason) == 5
    assert len(v1474.AlertSeverity) == 3  # reused from V1473
    assert len(v1474.AlertState) == 4  # reused from V1473
    assert v1474.FleetIncidentState.NEW.value == "NEW"
    assert v1474.FleetIncidentState.OPEN.value == "OPEN"
    assert v1474.FleetIncidentState.CLOSED.value == "CLOSED"
    assert v1474.FleetShutdownReason.RUNTIME_LIMIT.value == "RUNTIME_LIMIT"
    assert v1474.FleetShutdownReason.ALL_STREAMS_GONE.value == "ALL_STREAMS_GONE"


# ──────────────────────────────────────────────────────────────────────
# Test 4: StreamTarget dataclass
# ──────────────────────────────────────────────────────────────────────


def test_v1474_stream_target_isolation():
    """Verify per-stream isolation — no shared mutable state."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "s1.jsonl"
        p2 = tmp / "s2.jsonl"
        agg = v1474.V1474MultiStreamAggregator(
            stream_paths=[p1, p2], max_runtime_s=5, max_streams=4,
        )
        assert len(agg.streams) == 2
        assert agg.streams[0].stream_id == "stream-A"
        assert agg.streams[1].stream_id == "stream-B"
        # Different recent_events lists
        assert agg.streams[0].recent_events is not agg.streams[1].recent_events
        # Different alert dicts
        assert agg.streams[0].alerts is not agg.streams[1].alerts
        # Each stream has alerts for all rules
        n_rules = len(agg.rules)
        assert len(agg.streams[0].alerts) == n_rules
        assert len(agg.streams[1].alerts) == n_rules
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_custom_stream_ids():
    """Verify custom stream_ids are respected."""
    tmp = Path(tempfile.mkdtemp())
    try:
        paths = [tmp / "a.jsonl", tmp / "b.jsonl", tmp / "c.jsonl"]
        agg = v1474.V1474MultiStreamAggregator(
            stream_paths=paths, stream_ids=["alpha", "beta", "gamma"],
            max_runtime_s=5, max_streams=8,
        )
        assert agg.streams[0].stream_id == "alpha"
        assert agg.streams[1].stream_id == "beta"
        assert agg.streams[2].stream_id == "gamma"
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 5: Cross-stream correlation logic
# ──────────────────────────────────────────────────────────────────────


def test_v1474_correlation_fires_on_threshold():
    """Same rule firing on K+ streams → fleet incident."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "a.jsonl"
        p2 = tmp / "b.jsonl"
        st_a = v1474.StreamTarget(stream_id="A", stream_path=p1)
        st_b = v1474.StreamTarget(stream_id="B", stream_path=p2)
        now = time.time()
        for st in (st_a, st_b):
            st.alerts["R001"] = v1473.AlertRecord(
                rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
                state=v1473.AlertState.FIRING,
                first_seen_at=now - 5.0, last_updated_at=now - 1.0,
                transition_count=1, last_message="firing", last_event_ts=now - 1.0,
            )
        active: dict = {}
        new_or_updated, closed = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now, active_incidents=active,
        )
        assert len(new_or_updated) == 1
        assert new_or_updated[0].rule_id == "R001"
        assert new_or_updated[0].severity == v1473.AlertSeverity.CRITICAL
        assert new_or_updated[0].n_streams == 2
        assert sorted(new_or_updated[0].stream_ids) == ["A", "B"]
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_correlation_threshold_enforced():
    """Same rule on only 1 stream → no fleet incident."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "a.jsonl"
        p2 = tmp / "b.jsonl"
        st_a = v1474.StreamTarget(stream_id="A", stream_path=p1)
        st_b = v1474.StreamTarget(stream_id="B", stream_path=p2)
        now = time.time()
        # Only A fires
        st_a.alerts["R001"] = v1473.AlertRecord(
            rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
            state=v1473.AlertState.FIRING,
            first_seen_at=now, last_updated_at=now, transition_count=1,
            last_message="firing", last_event_ts=now,
        )
        st_b.alerts["R001"] = v1473.AlertRecord(
            rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
            state=v1473.AlertState.INACTIVE,
            first_seen_at=0.0, last_updated_at=0.0, transition_count=0,
            last_message="", last_event_ts=None,
        )
        active: dict = {}
        new_or_updated, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now, active_incidents=active,
        )
        assert len(new_or_updated) == 0
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_correlation_window_enforced():
    """Same rule firing on K+ streams but first_fired_at too old → no incident."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "a.jsonl"
        p2 = tmp / "b.jsonl"
        st_a = v1474.StreamTarget(stream_id="A", stream_path=p1)
        st_b = v1474.StreamTarget(stream_id="B", stream_path=p2)
        now = time.time()
        for st in (st_a, st_b):
            st.alerts["R001"] = v1473.AlertRecord(
                rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
                state=v1473.AlertState.FIRING,
                first_seen_at=now - 200.0, last_updated_at=now - 100.0,
                transition_count=1, last_message="firing", last_event_ts=now - 100.0,
            )
        active: dict = {}
        new_or_updated, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now, active_incidents=active,
        )
        assert len(new_or_updated) == 0
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_correlation_incident_state_machine():
    """NEW → OPEN transition + CLOSED after grace."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "a.jsonl"
        p2 = tmp / "b.jsonl"
        st_a = v1474.StreamTarget(stream_id="A", stream_path=p1)
        st_b = v1474.StreamTarget(stream_id="B", stream_path=p2)
        now = time.time()
        # Both streams fire R001
        for st in (st_a, st_b):
            st.alerts["R001"] = v1473.AlertRecord(
                rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
                state=v1473.AlertState.FIRING,
                first_seen_at=now, last_updated_at=now, transition_count=1,
                last_message="firing", last_event_ts=now,
            )
        active: dict = {}
        # Initial: NEW state
        new_or_updated, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=5.0,
            now=now, active_incidents=active,
        )
        assert len(active) == 1
        inc = list(active.values())[0]
        assert inc.state == v1474.FleetIncidentState.NEW

        # After grace, still firing → promote to OPEN
        new_or_updated, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=5.0,
            now=now + 6.0, active_incidents=active,
        )
        inc = list(active.values())[0]
        assert inc.state == v1474.FleetIncidentState.OPEN

        # Condition clears → close after grace
        for st in (st_a, st_b):
            st.alerts["R001"].state = v1473.AlertState.INACTIVE
        _, closed = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=5.0,
            now=now + 12.0, active_incidents=active,
        )
        assert len(closed) == 1
        assert closed[0].state == v1474.FleetIncidentState.CLOSED
        assert len(active) == 0
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_correlation_dedup():
    """Same rule with same n_streams only fires once (not duplicated)."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "a.jsonl"
        p2 = tmp / "b.jsonl"
        st_a = v1474.StreamTarget(stream_id="A", stream_path=p1)
        st_b = v1474.StreamTarget(stream_id="B", stream_path=p2)
        now = time.time()
        for st in (st_a, st_b):
            st.alerts["R001"] = v1473.AlertRecord(
                rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
                state=v1473.AlertState.FIRING,
                first_seen_at=now, last_updated_at=now, transition_count=1,
                last_message="firing", last_event_ts=now,
            )
        active: dict = {}
        # First call: opens incident
        v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now, active_incidents=active,
        )
        assert len(active) == 1
        first_inc = list(active.values())[0]

        # Second call: same conditions → should refresh, NOT add new
        new_or_updated, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now + 1.0, active_incidents=active,
        )
        assert len(active) == 1, "should not duplicate incident"
        assert list(active.values())[0].incident_key == first_inc.incident_key
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 6: Validation
# ──────────────────────────────────────────────────────────────────────


def test_v1474_validation_empty_streams():
    """Empty stream list should be rejected."""
    with pytest.raises(ValueError):
        v1474.V1474MultiStreamAggregator(stream_paths=[], max_runtime_s=10)


def test_v1474_validation_too_many_streams():
    """Stream count > max_streams should be rejected."""
    tmp = Path(tempfile.mkdtemp())
    try:
        paths = [tmp / f"s{i}.jsonl" for i in range(5)]
        with pytest.raises(ValueError):
            v1474.V1474MultiStreamAggregator(stream_paths=paths, max_runtime_s=10, max_streams=4)
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_validation_bounds():
    """Out-of-bounds parameters should be rejected."""
    with pytest.raises(ValueError):
        v1474.V1474MultiStreamAggregator(
            stream_paths=[Path("a.jsonl")], max_runtime_s=10000,
        )
    with pytest.raises(ValueError):
        v1474.V1474MultiStreamAggregator(
            stream_paths=[Path("a.jsonl")], eval_interval_s=0.1,
        )
    with pytest.raises(ValueError):
        v1474.V1474MultiStreamAggregator(
            stream_paths=[Path("a.jsonl")], incident_threshold=1,
        )


# ──────────────────────────────────────────────────────────────────────
# Test 7: Reuse from V1473
# ──────────────────────────────────────────────────────────────────────


def test_v1474_reuses_v1473_helpers():
    """V1474 imports V1473 helpers (主 19:33 站在前人肩上)."""
    assert v1474._evaluate_rule is v1473._evaluate_rule
    assert v1474._transition_state is v1473._transition_state
    assert len(v1474._make_builtin_rules()) == 7
    # Same rule ids
    rule_ids = {r.rule_id for r in v1474._make_builtin_rules()}
    assert "R001_VERDICT_REGRESSED" in rule_ids
    assert "R005_STREAM_STALE" in rule_ids


# ──────────────────────────────────────────────────────────────────────
# Test 8: Demo via CLI
# ──────────────────────────────────────────────────────────────────────


def test_v1474_demo_via_cli():
    """Run V1474 demo; expect exit 0 + aggregator log + JSONL streams written."""
    tmp = Path(tempfile.mkdtemp())
    try:
        out_dir = tmp / "demo-out"
        result = _run_python_module(
            "apeireth.v1474_asi_v1473_multi_stream_aggregator",
            "demo", "--max-runtime", "20", "--out-dir", str(out_dir),
            timeout_s=60.0,
        )
        assert result.returncode == 0, f"demo failed:\nstdout={result.stdout}\nstderr={result.stderr}"
        # Output files written
        assert (out_dir / "AggregatorReport.json").exists()
        assert (out_dir / "AggregatorReport.md").exists()
        assert (out_dir / "aggregator.log").exists()
        assert (out_dir / "alert-stream.jsonl").exists()
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 9: Run via CLI with synthetic streams
# ──────────────────────────────────────────────────────────────────────


def test_v1474_run_via_cli_against_synthetic_streams():
    """Run V1474 run against 2 synthetic JSONL streams."""
    tmp = Path(tempfile.mkdtemp())
    try:
        out_dir = tmp / "run-out"
        stream_a = tmp / "stream-a.jsonl"
        stream_b = tmp / "stream-b.jsonl"
        now = time.time()
        events_a = [
            {"ts": now - 5.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
            {"ts": now - 3.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
            {"ts": now - 1.0, "verdict": "REGRESSED", "n_invariants_failed": 2, "n_endpoints_2xx": 4},
        ]
        events_b = [
            {"ts": now - 5.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
            {"ts": now - 3.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
            {"ts": now - 1.0, "verdict": "REGRESSED", "n_invariants_failed": 2, "n_endpoints_2xx": 4},
        ]
        _write_jsonl(stream_a, events_a)
        _write_jsonl(stream_b, events_b)

        result = _run_python_module(
            "apeireth.v1474_asi_v1473_multi_stream_aggregator",
            "run",
            "--stream", str(stream_a), "--stream", str(stream_b),
            "--max-runtime", "10",
            "--eval-interval", "1.0",
            "--out-dir", str(out_dir),
            timeout_s=30.0,
        )
        assert result.returncode == 0, f"run failed:\nstdout={result.stdout}\nstderr={result.stderr}"
        # Report file
        assert (out_dir / "AggregatorReport.json").exists()
        with (out_dir / "AggregatorReport.json").open(encoding="utf-8") as f:
            report = json.load(f)
        assert report["n_streams"] == 2
        assert "stream-A" in report["stream_ids"]
        assert "stream-B" in report["stream_ids"]
        # Digest port present
        assert report["digest_port"] is not None
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 10: /digest HTTP endpoint
# ──────────────────────────────────────────────────────────────────────


def test_v1474_digest_endpoint_serves_json():
    """Run V1474 demo and query /digest + /streams + /healthz endpoints."""
    tmp = Path(tempfile.mkdtemp())
    try:
        out_dir = tmp / "demo-endpoint"
        # Start aggregator in a subprocess (so we can hit endpoints)
        import threading
        result_holder: dict = {}

        def _run_aggregator():
            try:
                r = _run_python_module(
                    "apeireth.v1474_asi_v1473_multi_stream_aggregator",
                    "demo", "--max-runtime", "15", "--out-dir", str(out_dir),
                    timeout_s=60.0,
                )
                result_holder["returncode"] = r.returncode
                result_holder["stdout"] = r.stdout
            except Exception as e:
                result_holder["error"] = str(e)

        thread = threading.Thread(target=_run_aggregator, daemon=True)
        thread.start()

        # Wait for aggregator to start + read digest port from log
        time.sleep(3.0)
        port = None
        for _ in range(20):
            log_path = out_dir / "aggregator.log"
            if log_path.exists():
                text = log_path.read_text(encoding="utf-8", errors="replace")
                for line in text.splitlines():
                    if "digest server listening on" in line:
                        try:
                            port = int(line.rsplit(":", 1)[-1].strip())
                        except (ValueError, IndexError):
                            pass
                        break
            if port:
                break
            time.sleep(0.5)

        assert port is not None, "could not find digest port from log"

        # Hit endpoints
        status, body = _http_get_json("127.0.0.1", port, "/healthz", timeout_s=3.0)
        assert status == 200
        assert body.get("status") == "ok"

        status, body = _http_get_json("127.0.0.1", port, "/digest", timeout_s=3.0)
        assert status == 200
        assert "per_stream" in body
        assert "fleet_incidents" in body
        assert body["n_streams"] >= 2

        status, body = _http_get_json("127.0.0.1", port, "/streams", timeout_s=3.0)
        assert status == 200
        assert "streams" in body
        assert len(body["streams"]) >= 2

        # 404 for unknown path
        status, _ = _http_get_json("127.0.0.1", port, "/unknown-path", timeout_s=3.0)
        assert status == 404

        thread.join(timeout=30.0)
        assert "returncode" in result_holder
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 11: Report JSON / Markdown roundtrip
# ──────────────────────────────────────────────────────────────────────


def test_v1474_report_json_roundtrip():
    """AggregatorReport JSON is parseable + has all required keys."""
    tmp = Path(tempfile.mkdtemp())
    try:
        stream_a = tmp / "a.jsonl"
        stream_b = tmp / "b.jsonl"
        events = [
            {"ts": time.time() - 1.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
        ]
        _write_jsonl(stream_a, events)
        _write_jsonl(stream_b, events)

        agg = v1474.V1474MultiStreamAggregator(
            stream_paths=[stream_a, stream_b], max_runtime_s=5,
            eval_interval_s=1.0, out_dir=tmp / "out",
        )
        report = agg.run()
        assert report.module == v1474.MODULE_NAME
        assert report.phase == 1474
        assert report.n_streams == 2
        assert len(report.stream_stats) == 2
        # Report JSON exists and parseable
        with agg.report_json_path.open(encoding="utf-8") as f:
            data = json.load(f)
        assert data["module"] == v1474.MODULE_NAME
        assert data["phase"] == 1474
        assert "guards" in data
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


def test_v1474_report_markdown_written():
    """AggregatorReport Markdown is written."""
    tmp = Path(tempfile.mkdtemp())
    try:
        stream_a = tmp / "a.jsonl"
        stream_b = tmp / "b.jsonl"
        events = [{"ts": time.time() - 1.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5}]
        _write_jsonl(stream_a, events)
        _write_jsonl(stream_b, events)

        agg = v1474.V1474MultiStreamAggregator(
            stream_paths=[stream_a, stream_b], max_runtime_s=5,
            eval_interval_s=1.0, out_dir=tmp / "out",
        )
        agg.run()
        assert agg.report_md_path.exists()
        text = agg.report_md_path.read_text(encoding="utf-8")
        assert "v1474_asi_v1473_multi_stream_aggregator" in text
        assert "Per-stream stats" in text
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 12: Custom rules
# ──────────────────────────────────────────────────────────────────────


def test_v1474_custom_rule_added():
    """Custom rules can be added via constructor."""
    tmp = Path(tempfile.mkdtemp())
    try:
        stream_a = tmp / "a.jsonl"
        stream_b = tmp / "b.jsonl"
        _write_jsonl(stream_a, [])
        _write_jsonl(stream_b, [])
        custom_rule = v1473.AlertRule(
            rule_id="CUSTOM_TEST",
            severity=v1473.AlertSeverity.INFO,
            condition_type=v1473.RuleConditionType.VERDICT_EQUALS,
            params={"verdict": "REGRESSED"},
        )
        agg = v1474.V1474MultiStreamAggregator(
            stream_paths=[stream_a, stream_b], max_runtime_s=5,
            eval_interval_s=1.0, custom_rules=[custom_rule],
            out_dir=tmp / "out",
        )
        # Built-in + custom rules present
        rule_ids = {r.rule_id for r in agg.rules}
        assert "CUSTOM_TEST" in rule_ids
        assert "R001_VERDICT_REGRESSED" in rule_ids
        # All rules initialized for each stream
        for st in agg.streams:
            assert "CUSTOM_TEST" in st.alerts
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 13: Determinism
# ──────────────────────────────────────────────────────────────────────


def test_v1474_deterministic_correlation():
    """Same inputs → same fleet incidents (deterministic)."""
    tmp = Path(tempfile.mkdtemp())
    try:
        p1 = tmp / "a.jsonl"
        p2 = tmp / "b.jsonl"
        st_a = v1474.StreamTarget(stream_id="A", stream_path=p1)
        st_b = v1474.StreamTarget(stream_id="B", stream_path=p2)
        now = time.time()
        for st in (st_a, st_b):
            st.alerts["R001"] = v1473.AlertRecord(
                rule_id="R001", severity=v1473.AlertSeverity.CRITICAL,
                state=v1473.AlertState.FIRING,
                first_seen_at=now, last_updated_at=now, transition_count=1,
                last_message="firing", last_event_ts=now,
            )
        # Run correlation twice
        active1: dict = {}
        n1, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now, active_incidents=active1,
        )
        active2: dict = {}
        n2, _ = v1474._correlate_fleet_incidents(
            streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=15.0,
            now=now, active_incidents=active2,
        )
        # Same number of incidents, same keys
        assert len(n1) == len(n2)
        keys1 = sorted(inc.incident_key for inc in n1)
        keys2 = sorted(inc.incident_key for inc in n2)
        assert keys1 == keys2
    finally:
        import shutil
        try:
            shutil.rmtree(tmp)
        except OSError:
            pass


# ──────────────────────────────────────────────────────────────────────
# Test 14: CLI commands
# ──────────────────────────────────────────────────────────────────────


def test_v1474_meta_via_cli():
    """meta CLI prints module metadata."""
    result = _run_python_module(
        "apeireth.v1474_asi_v1473_multi_stream_aggregator", "meta",
        timeout_s=15.0,
    )
    assert result.returncode == 0
    assert "v1474" in result.stdout.lower()
    assert "phase" in result.stdout.lower()
    assert "max_streams" in result.stdout.lower()


def test_v1474_chain_via_cli():
    """chain CLI shows lineage."""
    result = _run_python_module(
        "apeireth.v1474_asi_v1473_multi_stream_aggregator", "chain",
        timeout_s=15.0,
    )
    assert result.returncode == 0
    assert "V1474" in result.stdout
    assert "V1473" in result.stdout
    assert "V1472" in result.stdout
    assert "V1471" in result.stdout


def test_v1474_help_via_cli():
    """help CLI shows usage."""
    result = _run_python_module(
        "apeireth.v1474_asi_v1473_multi_stream_aggregator", "help",
        timeout_s=15.0,
    )
    assert result.returncode == 0
    assert "V1474" in result.stdout
    assert "{run,demo,popper,meta,chain,help}" in result.stdout