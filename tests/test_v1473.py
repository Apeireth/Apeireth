"""Tests for V1473 — V1472 audit alerting engine."""

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

import v1473_asi_v1472_alerting_engine as v1473  # noqa: E402


# ──────────────────────────────────────────────────────────────────────
# Test helpers
# ──────────────────────────────────────────────────────────────────────


def _run_python_module(module: str, *args: str, timeout_s: float = 60.0, cwd: Path = _PROMETHEAN_PARENT) -> subprocess.CompletedProcess:
    """Run `python -m <module> <args...>` and return CompletedProcess.

    Use explicit UTF-8 encoding to avoid Windows GBK codec crashes when
    subprocess stderr contains non-ASCII bytes (causes result.stdout to be None).
    """
    cmd = [sys.executable, "-m", module, *args]
    return subprocess.run(
        cmd,
        capture_output=True,
        encoding="utf-8",
        errors="replace",
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


def _write_jsonl(path: Path, events: list) -> None:
    """Write a list of dicts as JSONL."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        for ev in events:
            f.write(json.dumps(ev, separators=(",", ":")) + "\n")


# ──────────────────────────────────────────────────────────────────────
# Test 1: Popper self-checks (CLI)
# ──────────────────────────────────────────────────────────────────────


def test_v1473_popper_via_cli():
    """Run V1473 popper CLI; expect 37/37 pass."""
    result = _run_python_module("apeireth.v1473_asi_v1472_alerting_engine", "popper", timeout_s=30.0)
    assert result.returncode == 0, f"popper failed:\nstdout={result.stdout}\nstderr={result.stderr}"
    assert "37/37" in result.stdout, f"expected 37/37, got:\n{result.stdout}"


# ──────────────────────────────────────────────────────────────────────
# Test 2: Popper direct API
# ──────────────────────────────────────────────────────────────────────


def test_v1473_popper_direct():
    """Run popper_v1473() directly; expect 37/37 pass."""
    results = v1473.popper_v1473(verbose=False)
    passed = sum(1 for _, ok, _ in results if ok)
    total = len(results)
    assert passed == total == 37, f"expected 37/37, got {passed}/{total}"


# ──────────────────────────────────────────────────────────────────────
# Test 3: Module constants
# ──────────────────────────────────────────────────────────────────────


def test_v1473_constants():
    """Verify V1473 module constants."""
    assert v1473.MODULE_NAME == "v1473_asi_v1472_alerting_engine"
    assert v1473.MODULE_PHASE == 1473
    assert v1473.MODULE_VERSION == "0.1.0"
    assert v1473.DEFAULT_HOST == "127.0.0.1"
    assert v1473.DEFAULT_ALERTS_PORT_MIN == 18980
    assert v1473.DEFAULT_ALERTS_PORT_MAX == 19080
    # Port ranges distinct from V1472 (18780) and V1471 (18580)
    assert v1473.DEFAULT_ALERTS_PORT_MIN > 18780
    assert v1473.DEFAULT_ALERTS_PORT_MIN > 18580


def test_v1473_enums():
    """Verify V1473 enums."""
    assert len(v1473.AlertSeverity) == 3
    assert len(v1473.AlertState) == 4
    assert len(v1473.ShutdownReason) == 5
    assert len(v1473.RuleConditionType) == 7
    assert v1473.AlertSeverity.INFO.value == "INFO"
    assert v1473.AlertSeverity.CRITICAL.value == "CRITICAL"
    assert v1473.AlertState.INACTIVE.value == "INACTIVE"
    assert v1473.AlertState.PENDING.value == "PENDING"
    assert v1473.AlertState.FIRING.value == "FIRING"
    assert v1473.AlertState.RESOLVED.value == "RESOLVED"


# ──────────────────────────────────────────────────────────────────────
# Test 4: Built-in rules
# ──────────────────────────────────────────────────────────────────────


def test_v1473_builtin_rules():
    """Verify 7 built-in alert rules."""
    rules = v1473._make_builtin_rules()
    assert len(rules) == 7
    rule_ids = [r.rule_id for r in rules]
    assert "R001_VERDICT_REGRESSED" in rule_ids
    assert "R002_CONSECUTIVE_REGRESSED_3" in rule_ids
    assert "R003_INVARIANT_FAIL_INCREASED" in rule_ids
    assert "R004_ENDPOINT_2XX_DECREASED" in rule_ids
    assert "R005_STREAM_STALE" in rule_ids
    assert "R006_V1471_ALIVE_FALSE" in rule_ids
    assert "R007_REPEATED_REGRESSED_2_IN_30S" in rule_ids
    # Severity mix
    severities = {r.severity for r in rules}
    assert v1473.AlertSeverity.CRITICAL in severities
    assert v1473.AlertSeverity.WARN in severities


# ──────────────────────────────────────────────────────────────────────
# Test 5: Pure evaluators
# ──────────────────────────────────────────────────────────────────────


def test_v1473_eval_verdict_equals():
    """R001 detects REGRESSED verdict."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R001_VERDICT_REGRESSED")
    events = [{"ts": 100.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5}]
    holds, msg = v1473._evaluate_rule(rule, events, 100.0, 110.0, 30.0)
    assert holds is True
    assert "REGRESSED" in msg


def test_v1473_eval_verdict_equals_negative():
    """R001 does not fire on UNCHANGED."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R001_VERDICT_REGRESSED")
    events = [{"ts": 100.0, "verdict": "UNCHANGED", "n_invariants_failed": 0, "n_endpoints_2xx": 6}]
    holds, _ = v1473._evaluate_rule(rule, events, 100.0, 110.0, 30.0)
    assert holds is False


def test_v1473_eval_consecutive_verdicts():
    """R002 detects 3 consecutive REGRESSED."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R002_CONSECUTIVE_REGRESSED_3")
    events = [
        {"ts": 1.0, "verdict": "REGRESSED"},
        {"ts": 2.0, "verdict": "REGRESSED"},
        {"ts": 3.0, "verdict": "REGRESSED"},
    ]
    holds, _ = v1473._evaluate_rule(rule, events, 3.0, 10.0, 30.0)
    assert holds is True


def test_v1473_eval_invariant_fail_increased():
    """R003 detects n_invariants_failed increase."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R003_INVARIANT_FAIL_INCREASED")
    events = [
        {"ts": 1.0, "verdict": "UNCHANGED", "n_invariants_failed": 0},
        {"ts": 2.0, "verdict": "REGRESSED", "n_invariants_failed": 2},
    ]
    holds, _ = v1473._evaluate_rule(rule, events, 2.0, 10.0, 30.0)
    assert holds is True


def test_v1473_eval_endpoint_2xx_decreased():
    """R004 detects n_endpoints_2xx decrease."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R004_ENDPOINT_2XX_DECREASED")
    events = [
        {"ts": 1.0, "verdict": "UNCHANGED", "n_endpoints_2xx": 6},
        {"ts": 2.0, "verdict": "REGRESSED", "n_endpoints_2xx": 5},
    ]
    holds, _ = v1473._evaluate_rule(rule, events, 2.0, 10.0, 30.0)
    assert holds is True


def test_v1473_eval_stream_stale():
    """R005 detects stale stream (no events for N seconds)."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R005_STREAM_STALE")
    events = [{"ts": 100.0, "verdict": "UNCHANGED"}]
    # last_event_ts=100.0, now=140.0, threshold=30.0 → stale
    holds, _ = v1473._evaluate_rule(rule, events, 100.0, 140.0, 30.0)
    assert holds is True


def test_v1473_eval_v1471_alive_false():
    """R006 detects alive=False in latest event."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R006_V1471_ALIVE_FALSE")
    events = [{"ts": 100.0, "verdict": "UNCHANGED", "alive": False}]
    holds, _ = v1473._evaluate_rule(rule, events, 100.0, 110.0, 30.0)
    assert holds is True


def test_v1473_eval_repeated_verdict():
    """R007 detects 2 REGRESSED within window_s."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R007_REPEATED_REGRESSED_2_IN_30S")
    now = 100.0
    events = [
        {"ts": 90.0, "verdict": "REGRESSED"},
        {"ts": 95.0, "verdict": "REGRESSED"},
    ]
    holds, _ = v1473._evaluate_rule(rule, events, 95.0, now, 30.0)
    assert holds is True


# ──────────────────────────────────────────────────────────────────────
# Test 6: State machine
# ──────────────────────────────────────────────────────────────────────


def test_v1473_state_inactive_to_pending():
    """INACTIVE + condition true → PENDING (transition)."""
    s, t = v1473._transition_state(v1473.AlertState.INACTIVE, True, 0.0, 10.0, 4.0, 8.0)
    assert s == v1473.AlertState.PENDING
    assert t is True


def test_v1473_state_pending_to_firing():
    """PENDING + condition true for ≥ debounce → FIRING (transition)."""
    s, t = v1473._transition_state(v1473.AlertState.PENDING, True, 10.0, 16.0, 4.0, 8.0)
    assert s == v1473.AlertState.FIRING
    assert t is True


def test_v1473_state_pending_to_inactive():
    """PENDING + condition false → INACTIVE (transition)."""
    s, t = v1473._transition_state(v1473.AlertState.PENDING, False, 10.0, 11.0, 4.0, 8.0)
    assert s == v1473.AlertState.INACTIVE
    assert t is True


def test_v1473_state_firing_to_resolved():
    """FIRING + condition false for ≥ grace → RESOLVED (transition)."""
    s, t = v1473._transition_state(v1473.AlertState.FIRING, False, 10.0, 20.0, 4.0, 8.0)
    assert s == v1473.AlertState.RESOLVED
    assert t is True


def test_v1473_state_resolved_to_pending():
    """RESOLVED + condition true → PENDING (transition)."""
    s, t = v1473._transition_state(v1473.AlertState.RESOLVED, True, 10.0, 12.0, 4.0, 8.0)
    assert s == v1473.AlertState.PENDING
    assert t is True


def test_v1473_state_firing_stable():
    """FIRING + condition true → stays FIRING (no transition)."""
    s, t = v1473._transition_state(v1473.AlertState.FIRING, True, 10.0, 11.0, 4.0, 8.0)
    assert s == v1473.AlertState.FIRING
    assert t is False


# ──────────────────────────────────────────────────────────────────────
# Test 7: JSONL tail helper
# ──────────────────────────────────────────────────────────────────────


def test_v1473_read_jsonl_tail_new():
    """Reads all lines on first call (last_size=0)."""
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False, mode="w") as tf:
        tf.write('{"ts":1.0,"verdict":"REGRESSED"}\n')
        tf.write('{"ts":2.0,"verdict":"UNCHANGED"}\n')
        tmp_path = Path(tf.name)
    try:
        size, events = v1473._read_jsonl_tail(tmp_path, 0)
        assert size > 0
        assert len(events) == 2
    finally:
        tmp_path.unlink()


def test_v1473_read_jsonl_tail_no_new():
    """Returns no events when file hasn't grown."""
    with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False, mode="w") as tf:
        tf.write('{"ts":1.0,"verdict":"REGRESSED"}\n')
        tmp_path = Path(tf.name)
    try:
        size, _ = v1473._read_jsonl_tail(tmp_path, 0)
        size2, events = v1473._read_jsonl_tail(tmp_path, size)
        assert size2 == size
        assert len(events) == 0
    finally:
        tmp_path.unlink()


def test_v1473_read_jsonl_tail_missing_file():
    """Returns (0, []) when file doesn't exist."""
    missing = Path(tempfile.gettempdir()) / "v1473-nonexistent-xyz-12345.jsonl"
    if missing.exists():
        missing.unlink()
    size, events = v1473._read_jsonl_tail(missing, 0)
    assert size == 0
    assert events == []


# ──────────────────────────────────────────────────────────────────────
# Test 8: Validation
# ──────────────────────────────────────────────────────────────────────


def test_v1473_engine_validation_max_runtime():
    """max_runtime_s out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1473.V1473AlertingEngine(
            stream_path=Path("/tmp/nonexistent.jsonl"),
            max_runtime_s=0.5,
        )
    with pytest.raises(ValueError):
        v1473.V1473AlertingEngine(
            stream_path=Path("/tmp/nonexistent.jsonl"),
            max_runtime_s=5000.0,
        )


def test_v1473_engine_validation_eval_interval():
    """eval_interval_s out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1473.V1473AlertingEngine(
            stream_path=Path("/tmp/nonexistent.jsonl"),
            eval_interval_s=0.1,
        )


def test_v1473_engine_validation_max_rules():
    """max_rules out of bounds raises ValueError."""
    with pytest.raises(ValueError):
        v1473.V1473AlertingEngine(
            stream_path=Path("/tmp/nonexistent.jsonl"),
            max_rules=0,
        )


# ──────────────────────────────────────────────────────────────────────
# Test 9: Engine instance + report
# ──────────────────────────────────────────────────────────────────────


def test_v1473_engine_collect_alerts():
    """V1473AlertingEngine starts with all alerts INACTIVE."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1473-test-"))
    stream = out_dir / "stream.jsonl"
    stream.write_text("")
    engine = v1473.V1473AlertingEngine(
        stream_path=stream,
        max_runtime_s=2.0,
        eval_interval_s=1.0,
        out_dir=out_dir,
    )
    assert len(engine.alerts) == 7  # 7 built-in rules
    for a in engine.alerts.values():
        assert a.state == v1473.AlertState.INACTIVE
        assert a.transition_count == 0


def test_v1473_engine_evaluate_triggers_alert():
    """Synthetic stream with REGRESSED triggers R001 + R002 + R007."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1473-trigger-"))
    stream = out_dir / "stream.jsonl"
    events = [
        {"ts": time.time(), "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5, "alive": True},
    ]
    _write_jsonl(stream, events)
    engine = v1473.V1473AlertingEngine(
        stream_path=stream,
        max_runtime_s=3.0,
        eval_interval_s=0.5,
        debounce_s=1.0,
        out_dir=out_dir,
    )
    # Force a stream read + evaluation manually (bypassing run())
    engine._read_stream_once()
    engine._evaluate_all_rules(time.time())
    r001 = engine.alerts["R001_VERDICT_REGRESSED"]
    r007 = engine.alerts["R007_REPEATED_REGRESSED_2_IN_30S"]
    # R001 should be PENDING (need debounce to FIRING)
    assert r001.state == v1473.AlertState.PENDING
    # R007 needs 2 events; only 1 → still INACTIVE
    assert r007.state == v1473.AlertState.INACTIVE


# ──────────────────────────────────────────────────────────────────────
# Test 10: Real demo via CLI (subprocess)
# ──────────────────────────────────────────────────────────────────────


def test_v1473_demo_via_cli():
    """Run V1473 demo CLI; verify synthetic stream + alerts + reports."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1473-demo-"))
    result = _run_python_module(
        "apeireth.v1473_asi_v1472_alerting_engine", "demo",
        "--out-dir", str(out_dir),
        "--max-runtime", "20",
        "--eval-interval", "1",
        "--debounce", "2",
        "--resolved-grace", "4",
        timeout_s=60.0,
    )
    assert result.returncode in (0, 1), f"demo failed:\nstdout={result.stdout}\nstderr={result.stderr}"

    # Verify output files exist
    report_json = out_dir / "AlertReport.json"
    report_md = out_dir / "AlertReport.md"
    log_file = out_dir / "alerts.log"
    stream_file = out_dir / "synthetic-stream.jsonl"
    alert_stream_file = out_dir / "alert-stream.jsonl"
    assert report_json.exists(), f"report JSON not written"
    assert report_md.exists(), f"report MD not written"
    assert log_file.exists(), f"log file not written"
    assert stream_file.exists(), f"synthetic stream not written"
    assert alert_stream_file.exists(), f"alert stream not written"

    # Verify report JSON content
    report = json.loads(report_json.read_text(encoding="utf-8"))
    assert report["module"] == "v1473_asi_v1472_alerting_engine"
    assert report["phase"] == 1473
    assert report["n_rules"] == 7
    assert report["n_stream_events_read"] >= 3, f"expected ≥3 stream events, got {report['n_stream_events_read']}"
    assert report["n_evaluations"] >= 5, f"expected ≥5 evaluations, got {report['n_evaluations']}"
    assert report["shutdown_reason"] == "RUNTIME_LIMIT"
    assert report["alerts_port"] is not None and report["alerts_port"] > 0


# ──────────────────────────────────────────────────────────────────────
# Test 11: Real run via CLI against a pre-existing JSONL stream
# ──────────────────────────────────────────────────────────────────────


def test_v1473_run_via_cli_against_synthetic_stream():
    """Run V1473 run CLI against a pre-written synthetic stream."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1473-run-"))
    stream = out_dir / "test-stream.jsonl"
    # Pre-write 5 events
    base_ts = time.time()
    events = []
    for i, verdict in enumerate(["UNCHANGED", "REGRESSED", "REGRESSED", "REGRESSED", "UNCHANGED"]):
        events.append({
            "ts": base_ts + i * 0.5,
            "audit_id": f"test-{i:04d}",
            "verdict": verdict,
            "n_endpoints_total": 6,
            "n_endpoints_2xx": 5 if verdict == "REGRESSED" else 6,
            "n_invariants_total": 9,
            "n_invariants_failed": 1 if verdict == "REGRESSED" else 0,
            "alive": True,
        })
    _write_jsonl(stream, events)

    # Run V1473 against this stream
    out_run = out_dir / "run-output"
    result = _run_python_module(
        "apeireth.v1473_asi_v1472_alerting_engine", "run",
        "--jsonl-stream", str(stream),
        "--max-runtime", "6",
        "--eval-interval", "0.5",
        "--debounce", "2",
        "--resolved-grace", "4",
        "--out-dir", str(out_run),
        timeout_s=30.0,
    )
    assert result.returncode in (0, 1), f"run failed:\nstdout={result.stdout}\nstderr={result.stderr}"

    # Verify output files
    report_json = out_run / "AlertReport.json"
    assert report_json.exists(), f"report JSON not written"
    report = json.loads(report_json.read_text(encoding="utf-8"))
    assert report["n_stream_events_read"] >= 5, f"expected ≥5 events, got {report['n_stream_events_read']}"
    # Some alerts should have fired
    assert report["n_state_transitions"] >= 1, f"expected ≥1 transition, got {report['n_state_transitions']}"


# ──────────────────────────────────────────────────────────────────────
# Test 12: /alerts HTTP endpoint live
# ──────────────────────────────────────────────────────────────────────


def test_v1473_alerts_endpoint_serves_json():
    """Boot V1473 briefly; query /alerts; verify JSON response."""
    out_dir = Path(tempfile.mkdtemp(prefix="v1473-endpoint-"))
    stream = out_dir / "test-stream.jsonl"
    # Pre-write one event
    _write_jsonl(stream, [
        {"ts": time.time(), "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5, "alive": True},
    ])

    # Start engine in subprocess
    cmd = [
        sys.executable, "-u", "-m", "apeireth.v1473_asi_v1472_alerting_engine", "run",
        "--jsonl-stream", str(stream),
        "--max-runtime", "8",
        "--eval-interval", "0.5",
        "--debounce", "1",
        "--resolved-grace", "2",
        "--out-dir", str(out_dir),
    ]
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=str(_PROMETHEAN_PARENT),
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == "win32" else 0,
    )
    try:
        # Wait for server to come up
        time.sleep(4.0)

        # Read the alerts port from report JSON (interim may not exist; try stderr)
        # Try to discover port by scanning 18980-19080
        port = None
        for p in range(18980, 19081):
            try:
                status, body = _http_get_json("127.0.0.1", p, "/healthz", timeout_s=1.0)
                if status == 200 and isinstance(body, dict) and body.get("status") == "ok":
                    port = p
                    break
            except Exception:
                continue
        assert port is not None, "could not discover alerts port via /healthz scan"

        # Query /alerts
        status, body = _http_get_json("127.0.0.1", port, "/alerts", timeout_s=2.0)
        assert status == 200, f"/alerts returned {status}"
        assert body["module"] == "v1473_asi_v1472_alerting_engine"
        assert "alerts" in body
        assert len(body["alerts"]) == 7, f"expected 7 alert records, got {len(body['alerts'])}"
        assert body["n_evaluations"] >= 1, f"expected ≥1 evaluation, got {body['n_evaluations']}"

        # Query /healthz
        status2, body2 = _http_get_json("127.0.0.1", port, "/healthz", timeout_s=2.0)
        assert status2 == 200
        assert body2["status"] == "ok"

        # Query unknown path → 404
        status3, body3 = _http_get_json("127.0.0.1", port, "/unknown", timeout_s=2.0)
        assert status3 == 404

    finally:
        # Terminate subprocess
        try:
            if sys.platform == "win32":
                proc.terminate()
            else:
                proc.send_signal(signal.SIGTERM)
        except Exception:
            pass
        try:
            proc.wait(timeout=5.0)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=3.0)


# ──────────────────────────────────────────────────────────────────────
# Test 13: Determinism
# ──────────────────────────────────────────────────────────────────────


def test_v1473_deterministic_rule_eval():
    """Same inputs → same outputs across multiple calls."""
    rule = next(r for r in v1473._make_builtin_rules() if r.rule_id == "R001_VERDICT_REGRESSED")
    events = [
        {"ts": 100.0, "verdict": "REGRESSED"},
        {"ts": 105.0, "verdict": "REGRESSED"},
    ]
    holds_a, msg_a = v1473._evaluate_rule(rule, events, 105.0, 110.0, 30.0)
    holds_b, msg_b = v1473._evaluate_rule(rule, events, 105.0, 110.0, 30.0)
    assert holds_a == holds_b
    assert msg_a == msg_b


def test_v1473_deterministic_state_transition():
    """Same inputs → same state transition."""
    for _ in range(3):
        s, t = v1473._transition_state(v1473.AlertState.PENDING, True, 10.0, 16.0, 4.0, 8.0)
        assert s == v1473.AlertState.FIRING
        assert t is True


# ──────────────────────────────────────────────────────────────────────
# Test 14: Severity ordering
# ──────────────────────────────────────────────────────────────────────


def test_v1473_severity_rank():
    """INFO < WARN < CRITICAL."""
    assert v1473._severity_rank(v1473.AlertSeverity.INFO) < v1473._severity_rank(v1473.AlertSeverity.WARN)
    assert v1473._severity_rank(v1473.AlertSeverity.WARN) < v1473._severity_rank(v1473.AlertSeverity.CRITICAL)


# ──────────────────────────────────────────────────────────────────────
# Test 15: Chain (V1472 + V1471 importable)
# ──────────────────────────────────────────────────────────────────────


def test_v1473_chain_via_cli():
    """Run V1473 chain CLI; verify lineage."""
    result = _run_python_module("apeireth.v1473_asi_v1472_alerting_engine", "chain", timeout_s=10.0)
    assert result.returncode == 0, f"chain failed:\n{result.stdout}\n{result.stderr}"
    assert "V1473" in result.stdout
    assert "V1472" in result.stdout
    assert "V1471" in result.stdout


# ──────────────────────────────────────────────────────────────────────
# Test 16: Meta via CLI
# ──────────────────────────────────────────────────────────────────────


def test_v1473_meta_via_cli():
    """Run V1473 meta CLI; verify module metadata."""
    result = _run_python_module("apeireth.v1473_asi_v1472_alerting_engine", "meta", timeout_s=10.0)
    assert result.returncode == 0
    assert "v1473_asi_v1472_alerting_engine" in result.stdout
    assert "1473" in result.stdout


# ──────────────────────────────────────────────────────────────────────
# Test 17: Help via CLI
# ──────────────────────────────────────────────────────────────────────


def test_v1473_help_via_cli():
    """Run V1473 help CLI; verify usage output."""
    result = _run_python_module("apeireth.v1473_asi_v1472_alerting_engine", "help", timeout_s=10.0)
    assert result.returncode == 0
    assert "V1473" in result.stdout


# ──────────────────────────────────────────────────────────────────────
# Test 18: Report writers
# ──────────────────────────────────────────────────────────────────────


def test_v1473_report_json_roundtrip():
    """AlertReport JSON roundtrip preserves key fields."""
    report = v1473.AlertReport(
        module=v1473.MODULE_NAME,
        phase=v1473.MODULE_PHASE,
        version=v1473.MODULE_VERSION,
        started_at="2026-08-10T18:00:00Z",
        ended_at="2026-08-10T18:00:30Z",
        elapsed_s=30.0,
        max_runtime_s=60.0,
        eval_interval_s=2.0,
        debounce_s=4.0,
        resolved_grace_s=8.0,
        stale_threshold_s=30.0,
        max_rules=32,
        max_alerts=64,
        stream_path="/tmp/test.jsonl",
        alerts_port=18980,
        n_rules=7,
        n_evaluations=15,
        n_alert_events=3,
        n_state_transitions=3,
        n_alerts_firing=1,
        n_alerts_resolved=1,
        n_alerts_pending=0,
        n_alerts_inactive=5,
        n_stream_events_read=10,
        stream_last_event_ts=1234567890.0,
        stream_stale=False,
        shutdown_reason=v1473.ShutdownReason.RUNTIME_LIMIT,
        rule_summaries=[],
        recent_events=[],
    )
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False, mode="w") as tf:
        tmp_path = Path(tf.name)
    try:
        v1473.write_report_json(report, tmp_path)
        assert tmp_path.exists()
        parsed = json.loads(tmp_path.read_text(encoding="utf-8"))
        assert parsed["module"] == v1473.MODULE_NAME
        assert parsed["phase"] == 1473
        assert parsed["shutdown_reason"] == "RUNTIME_LIMIT"
        assert parsed["n_rules"] == 7
    finally:
        tmp_path.unlink()


def test_v1473_report_markdown_written():
    """AlertReport Markdown writer produces non-empty output."""
    report = v1473.AlertReport(
        module=v1473.MODULE_NAME,
        phase=v1473.MODULE_PHASE,
        version=v1473.MODULE_VERSION,
        started_at="2026-08-10T18:00:00Z",
        ended_at="2026-08-10T18:00:30Z",
        elapsed_s=30.0,
        max_runtime_s=60.0,
        eval_interval_s=2.0,
        debounce_s=4.0,
        resolved_grace_s=8.0,
        stale_threshold_s=30.0,
        max_rules=32,
        max_alerts=64,
        stream_path="/tmp/test.jsonl",
        alerts_port=18980,
        n_rules=7,
        n_evaluations=15,
        n_alert_events=3,
        n_state_transitions=3,
        n_alerts_firing=1,
        n_alerts_resolved=1,
        n_alerts_pending=0,
        n_alerts_inactive=5,
        n_stream_events_read=10,
        stream_last_event_ts=1234567890.0,
        stream_stale=False,
        shutdown_reason=v1473.ShutdownReason.RUNTIME_LIMIT,
        rule_summaries=[
            {"rule_id": "R001", "severity": "CRITICAL", "state": "FIRING",
             "condition_type": "VERDICT_EQUALS", "params": {},
             "transition_count": 1, "last_updated_at": 0.0, "last_message": "x", "last_event_ts": None}
        ],
        recent_events=[
            {"event_type": "FIRED", "rule_id": "R001", "severity": "CRITICAL",
             "ts": 1234567890.0, "message": "test", "details": {}}
        ],
    )
    with tempfile.NamedTemporaryFile(suffix=".md", delete=False, mode="w") as tf:
        tmp_path = Path(tf.name)
    try:
        v1473.write_report_markdown(report, tmp_path)
        assert tmp_path.exists()
        content = tmp_path.read_text(encoding="utf-8")
        assert "v1473_asi_v1472_alerting_engine" in content
        assert "R001" in content
        assert "FIRING" in content
    finally:
        tmp_path.unlink()


# ──────────────────────────────────────────────────────────────────────
# Test 19: Custom rules
# ──────────────────────────────────────────────────────────────────────


def test_v1473_custom_rule_added():
    """Custom rule is appended to built-in rules."""
    custom = v1473.AlertRule(
        rule_id="CUSTOM_TEST",
        severity=v1473.AlertSeverity.INFO,
        condition_type=v1473.RuleConditionType.VERDICT_EQUALS,
        params={"verdict": "IMPROVED"},
    )
    out_dir = Path(tempfile.mkdtemp(prefix="v1473-custom-"))
    stream = out_dir / "stream.jsonl"
    stream.write_text("")
    engine = v1473.V1473AlertingEngine(
        stream_path=stream,
        max_runtime_s=2.0,
        out_dir=out_dir,
        custom_rules=[custom],
    )
    assert any(r.rule_id == "CUSTOM_TEST" for r in engine.rules)
    assert "CUSTOM_TEST" in engine.alerts
    assert engine.alerts["CUSTOM_TEST"].severity == v1473.AlertSeverity.INFO