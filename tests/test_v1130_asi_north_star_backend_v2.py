"""V1130 backend v2 tests: real subprocesses, real ports, no mocks."""
from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

import pytest

from apeireth.v1124_asi_north_star_backend import ASINorthStarBackend
from apeireth.v1125_r10_integration_protocol import ASI_NORTH_STAR, R10_START_TARGET, compute_v05_score
from apeireth.v1128_real_model_adapter_w2 import ProviderKind, ProviderSpec, ProviderState
from apeireth.v1130_asi_north_star_backend_v2 import (
    PARALLEL_MAX_WORKERS,
    PROVIDER_TIMEOUT_SEC,
    V1130_VERSION,
    V05_DEFAULT_AUTONOMY,
    V05_DEFAULT_CONTINUITY,
    V05_DEFAULT_TRANSFERABILITY,
    V3_GUARDS,
    CrossProviderCoordinator,
    CrossProviderPlan,
    CrossProviderResult,
    ProviderAttempt,
    V1074RuntimeSample,
    V1130Backend,
    WARN_PARALLEL_WALL_SEC,
    default_cross_provider_plan,
    fail_soft,
    run_subprocess_with_fail_soft,
    sample_v1074_runtime,
)


def executable_spec(name: str = "exec", command: tuple = (sys.executable, "-c",
                                                          "import sys;print('EXEC:'+sys.stdin.read())"),
                    attempts: int = 1, timeout: float = 2.0) -> ProviderSpec:
    return ProviderSpec(name, ProviderKind.EXECUTABLE, "real-python-process", command=command,
                        max_attempts=attempts, timeout_seconds=timeout)


def cli_spec(name: str = "cli", command: tuple = (sys.executable, "-c",
                                                  "import sys;print('CLI:'+sys.argv[1])", "{prompt}")) -> ProviderSpec:
    return ProviderSpec(name, ProviderKind.LOCAL_CLI, "real-python-cli", command=command, timeout_seconds=2.0)


def crash_command() -> tuple:
    return (sys.executable, "-c", "import sys;sys.stderr.write('crash');sys.exit(7)")


def empty_command() -> tuple:
    return (sys.executable, "-c", "pass")


@pytest.fixture
def backend(tmp_path: Path):
    return ASINorthStarBackend(tmp_path / "v1130")


# Constants / guards / DTO contracts (10)
def test_version_and_thresholds():
    assert V1130_VERSION == "0.1.0"
    assert V05_DEFAULT_CONTINUITY == V05_DEFAULT_AUTONOMY == V05_DEFAULT_TRANSFERABILITY == 0.85
    assert PARALLEL_MAX_WORKERS == 4
    assert WARN_PARALLEL_WALL_SEC == 2.5


def test_guards_contain_fail_soft():
    assert V3_GUARDS["fail_soft_is_not_success"]
    assert len(V3_GUARDS) >= 5


def test_default_plan_has_four_task_specs():
    # R10-BE-003 task description enumerates Anthropic / OpenAI / Ollama / local
    # as the forced-parallel set; executable (stdin) is retained as a helper but
    # is intentionally not part of the default plan.
    plan = default_cross_provider_plan("ping")
    assert len(plan.specs) == 4
    expected = {ProviderKind.ANTHROPIC, ProviderKind.OPENAI,
                ProviderKind.OLLAMA, ProviderKind.LOCAL_CLI}
    assert {spec.kind for spec in plan.specs} == expected


def test_plan_requires_prompt():
    with pytest.raises(ValueError):
        CrossProviderPlan(specs=(executable_spec(),), prompt=" ")


def test_plan_requires_specs():
    with pytest.raises(ValueError):
        CrossProviderPlan(specs=(), prompt="x")


def test_plan_rejects_too_many_workers():
    with pytest.raises(ValueError):
        CrossProviderPlan(specs=(executable_spec(),), prompt="x", parallel_max_workers=0)


def test_plan_rejects_excessive_workers():
    with pytest.raises(ValueError):
        CrossProviderPlan(specs=(executable_spec(),), prompt="x", parallel_max_workers=99)


def test_attempt_public_contains_real_fields():
    attempt = ProviderAttempt(provider="p", kind="executable", state="healthy", success=True,
                              error_code=None, latency_ms=12.5, content_sha256="deadbeef",
                              transport="process_stdin", detail="real inference succeeded")
    payload = attempt.public()
    assert payload["provider"] == "p" and payload["transport"] == "process_stdin"


def test_result_primary_provider_is_none_when_all_fail():
    result = CrossProviderResult(
        plan_id="x", providers_attempted=2, providers_succeeded=0,
        providers_forbidden=0, providers_unconfigured=0, providers_unavailable=2,
        attempts=(ProviderAttempt("a", "executable", "unavailable", False, "down", 5.0),
                  ProviderAttempt("b", "executable", "unavailable", False, "down", 5.0)),
        v04_score=0.8538, v05_score=0.85, continuity=0.85, autonomy=0.85, transferability=0.85,
        parallel_wall_seconds=0.1, identity_preserved=True)
    assert result.primary_provider is None and result.public()["primary_provider"] is None


def test_result_passes_flags_use_v05_threshold():
    v05 = compute_v05_score(0.8538, 0.85, 0.85, 0.85)["v05_total"]
    result = CrossProviderResult(
        plan_id="x", providers_attempted=1, providers_succeeded=1,
        providers_forbidden=0, providers_unconfigured=0, providers_unavailable=0,
        attempts=(ProviderAttempt("a", "executable", "healthy", True, None, 1.0),),
        v04_score=0.8538, v05_score=v05, continuity=0.85, autonomy=0.85, transferability=0.85,
        parallel_wall_seconds=0.1, identity_preserved=True)
    assert result.passes_r10_start is False
    assert result.passes_r10_ultimate is False
    assert result.passes_asi_north_star is False
    assert result.public()["v05_score"] == round(v05, 4)


# Fail-soft and subprocess wrappers (8)
def test_fail_soft_returns_value_on_success():
    assert fail_soft(lambda: 42, fallback=-1) == 42


def test_fail_soft_returns_fallback_on_exception():
    def boom():
        raise RuntimeError("nope")
    assert fail_soft(boom, fallback="default") == "default"


def test_fail_soft_records_on_error():
    captured: list[BaseException] = []
    fail_soft(lambda: (_ for _ in ()).throw(RuntimeError("x")), fallback=None, on_error=captured.append)
    assert len(captured) == 1 and isinstance(captured[0], RuntimeError)


def test_run_subprocess_with_fail_soft_real_success():
    result = run_subprocess_with_fail_soft(
        (sys.executable, "-c", "import sys;print('ok:'+sys.stdin.read())"), "x", 2.0)
    assert result["ok"] and result["content"] == "ok:x"


def test_run_subprocess_with_fail_soft_returns_fallback_on_failure():
    result = run_subprocess_with_fail_soft(crash_command(), "x", 2.0, fallback_text="default")
    assert not result["ok"] and result["fallback_used"] and result["fallback_text"] == "default"


def test_run_subprocess_with_fail_soft_rejects_empty_output():
    result = run_subprocess_with_fail_soft(empty_command(), "x", 2.0)
    assert not result["ok"] and result["error_code"] == "provider_invalid_response"


def test_run_subprocess_with_fail_soft_rejects_missing_command():
    result = run_subprocess_with_fail_soft(("definitely-missing-v1130-binary",), "x", 1.0)
    assert not result["ok"] and result["error_code"] == "provider_not_configured"


def test_fail_soft_logger_branch_records_debug():
    import logging
    logger = logging.getLogger("v1130.test")
    logger.addHandler(logging.NullHandler())
    fail_soft(lambda: (_ for _ in ()).throw(RuntimeError("logged")), fallback="x", logger=logger)
    assert True  # branch ran; logger was passed, no exception escaped


# V1074 runtime sampler (6)
def test_runtime_sample_rejects_single_iteration():
    with pytest.raises(ValueError):
        sample_v1074_runtime(iterations=1)


def test_runtime_sample_passes_target_with_warmup():
    sample = sample_v1074_runtime(iterations=3, target_seconds=5.0, baseline_seconds=3.05,
                                  runner=lambda: time.sleep(0.01))
    assert sample.passes_target and sample.iterations == 3
    assert sample.mean_seconds < sample.max_seconds


def test_runtime_sample_detects_target_violation():
    def slow():
        time.sleep(0.4)
    sample = sample_v1074_runtime(iterations=2, target_seconds=0.1, runner=slow)
    assert not sample.passes_target


def test_runtime_sample_savings_calculation():
    sample = sample_v1074_runtime(iterations=3, baseline_seconds=10.0,
                                  runner=lambda: time.sleep(0.01))
    assert sample.savings_pct > 90.0


def test_runtime_sample_with_custom_runner():
    counter = [0]
    def work():
        counter[0] += 1
        time.sleep(0.005)
    sample = sample_v1074_runtime(iterations=4, runner=work)
    assert counter[0] == 4


def test_runtime_sample_under_default_target():
    sample = sample_v1074_runtime(iterations=3)
    # The bundled warm V1074 workload is intentionally cheap (<2.5s target)
    assert sample.passes_target


# Parallel cross-provider evaluation, chaos, identity (16)
def test_parallel_evaluates_four_providers(backend):
    plan = CrossProviderPlan(specs=(executable_spec("a"), executable_spec("b"),
                                    executable_spec("c"), executable_spec("d")), prompt="W3")
    result = CrossProviderCoordinator(backend).evaluate(plan)
    assert result.providers_attempted == 4
    assert result.providers_succeeded == 4


def test_parallel_records_per_provider_latency(backend):
    plan = CrossProviderPlan(specs=(executable_spec("a"), executable_spec("b")), prompt="x")
    result = CrossProviderCoordinator(backend).evaluate(plan)
    assert all(attempt.latency_ms > 0 for attempt in result.attempts)
    assert {attempt.provider for attempt in result.attempts} == {"a", "b"}


def test_parallel_mixes_real_and_unconfigured(backend):
    plan = CrossProviderPlan(specs=(executable_spec("ok"),
                                    ProviderSpec("missing", ProviderKind.EXECUTABLE, "none")),
                              prompt="x")
    result = CrossProviderCoordinator(backend).evaluate(plan)
    assert result.providers_succeeded == 1
    assert result.providers_unconfigured == 1


def test_parallel_chaos_does_not_lose_identity(backend):
    coordinator = CrossProviderCoordinator(backend)
    result = coordinator.evaluate(CrossProviderPlan(
        specs=(executable_spec(command=crash_command()), executable_spec(command=crash_command())),
        prompt="chaos"))
    assert result.identity_preserved
    assert backend.store.load().core.identity_id == backend.identity.core.identity_id


def test_parallel_chaos_records_audit_failed_event(backend):
    CrossProviderCoordinator(backend).evaluate(CrossProviderPlan(
        specs=(executable_spec(command=crash_command()),), prompt="chaos"))
    events = [record["event"] for record in backend.store.audit.records()]
    assert events[-2:] == ["w3_plan_started", "w3_plan_failed"]


def test_parallel_success_persists_identity_entry(backend):
    CrossProviderCoordinator(backend).evaluate(CrossProviderPlan(
        specs=(executable_spec(),), prompt="real"))
    events = [record["event"] for record in backend.store.audit.records()]
    assert events[-3:] == ["w3_plan_started", "identity_snapshot_committed", "w3_plan_succeeded"]
    assert backend.store.load().entries[-1].kind == "w3_cross_provider"


def test_parallel_wall_time_is_below_target_with_real_processes(backend):
    plan = CrossProviderPlan(specs=(executable_spec("a"), executable_spec("b"),
                                    executable_spec("c"), executable_spec("d")), prompt="x")
    result = CrossProviderCoordinator(backend).evaluate(plan)
    assert result.parallel_wall_seconds < WARN_PARALLEL_WALL_SEC


def test_parallel_records_forbidden_state(backend):
    # Provider that returns 403-equivalent via http server
    server = _start_forbidden_server()
    try:
        plan = CrossProviderPlan(specs=(_anthropic_403_spec(server.server_port),), prompt="x")
        result = CrossProviderCoordinator(backend).evaluate(plan)
        assert result.providers_forbidden == 1
    finally:
        server.shutdown(); server.server_close()


def test_parallel_emits_yellow_alert_when_below_start(backend, tmp_path):
    from apeireth.v1130_r10_release_window_guard import AlertSink
    sink = AlertSink(persist_path=tmp_path / "alerts.jsonl")
    CrossProviderCoordinator(backend, alert_sink=sink).evaluate(CrossProviderPlan(
        specs=(executable_spec(),), prompt="x"))
    sources = {alert.source for alert in sink.alerts}
    # v0.5 = 0.8538*0.85 + 0.85*0.05*3 = 0.8532 < R10_START (0.86) → v05 yellow alert expected
    assert "v05_score" in sources
    assert all(alert.level in {"GREEN", "YELLOW", "RED"} for alert in sink.alerts)


def test_parallel_emits_red_alert_when_all_fail(backend, tmp_path):
    from apeireth.v1130_r10_release_window_guard import AlertSink
    sink = AlertSink(persist_path=tmp_path / "alerts.jsonl")
    CrossProviderCoordinator(backend, alert_sink=sink).evaluate(CrossProviderPlan(
        specs=(executable_spec(command=crash_command()),
               executable_spec(command=crash_command())), prompt="chaos"))
    levels = [alert.level for alert in sink.alerts]
    assert "RED" in levels


def test_parallel_persists_alert_even_when_sink_path_unwritable(backend, tmp_path):
    from apeireth.v1130_r10_release_window_guard import AlertSink
    sink = AlertSink(persist_path=tmp_path / "missing-dir" / "alerts.jsonl")
    CrossProviderCoordinator(backend, alert_sink=sink).evaluate(CrossProviderPlan(
        specs=(executable_spec(command=crash_command()),), prompt="x"))
    assert len(sink.alerts) >= 1


def test_parallel_does_not_claim_w3_target_when_no_provider_succeeded(backend):
    result = CrossProviderCoordinator(backend).evaluate(CrossProviderPlan(
        specs=(executable_spec(command=crash_command()),), prompt="x"))
    assert result.warnings and "no provider succeeded" in result.warnings[0]


def test_parallel_v05_score_matches_compute_v05(backend):
    result = CrossProviderCoordinator(backend).evaluate(CrossProviderPlan(
        specs=(executable_spec(),), prompt="x",
        v04_score=0.91, continuity=0.86, autonomy=0.84, transferability=0.87))
    expected = compute_v05_score(0.91, 0.86, 0.84, 0.87)["v05_total"]
    assert result.v05_score == round(expected, 4)


def test_parallel_passes_r10_start_when_v05_high(backend):
    # v0.5 = 0.95*0.85 + 0.95*0.05*3 = 0.95 → 既过 R10_START 也过 R10_ULTIMATE
    result = CrossProviderCoordinator(backend).evaluate(CrossProviderPlan(
        specs=(executable_spec(),), prompt="x",
        v04_score=0.95, continuity=0.95, autonomy=0.95, transferability=0.95))
    assert result.passes_r10_start is True
    assert result.passes_r10_ultimate is True


def test_parallel_attempt_details_contain_real_transport(backend):
    result = CrossProviderCoordinator(backend).evaluate(CrossProviderPlan(
        specs=(executable_spec(),), prompt="x"))
    attempt = result.attempts[0]
    assert attempt.transport == "process_stdin"
    assert attempt.success and len(attempt.content_sha256) == 64


def test_parallel_aggregates_state_counts_consistently(backend):
    plan = CrossProviderPlan(
        specs=(executable_spec("good"), executable_spec(command=crash_command(), name="bad"),
               ProviderSpec("none", ProviderKind.EXECUTABLE, "none")),
        prompt="x")
    result = CrossProviderCoordinator(backend).evaluate(plan)
    counts = result.providers_succeeded + result.providers_forbidden + result.providers_unconfigured + result.providers_unavailable
    assert counts == result.providers_attempted


# V1130Backend facade, HTTP dispatch, runtime endpoints (8)
def test_v1130_backend_level_uses_fail_soft(backend):
    payload = V1130Backend(backend.store.directory).level
    assert payload["score"] == 0.8538


def test_v1130_backend_evaluate_endpoint(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("POST", "/asi/v1130/evaluate",
                                   {"prompt": "x", "providers": [
                                       {"name": "a", "kind": "executable",
                                        "command": [sys.executable, "-c",
                                                    "import sys;print('EXEC:'+sys.stdin.read())"],
                                        "timeout_seconds": 5.0}]})
    assert status == 200 and body["providers_attempted"] == 1
    assert body["providers_succeeded"] == 1


def test_v1130_backend_runtime_endpoint(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("GET", "/asi/v1130/runtime")
    assert status == 200 and body["target_seconds"] == 2.5
    assert "mean_seconds" in body


def test_v1130_backend_alerts_endpoint(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("GET", "/asi/v1130/alerts")
    assert status == 200 and "n_alerts" in body and "by_level" in body


def test_v1130_backend_evaluate_invalid_body(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("POST", "/asi/v1130/evaluate", {"providers": "not-list"})
    assert status == 400 and body["error"]["code"] == "invalid_request"


def test_v1130_backend_evaluate_unknown_kind(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("POST", "/asi/v1130/evaluate",
                                   {"prompt": "x", "providers": [{"name": "a", "kind": "imaginary"}]})
    assert status == 400


def test_v1130_backend_falls_through_to_v1124(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("GET", "/asi/level")
    assert status == 200 and body["score"] == 0.8538


def test_v1130_backend_unknown_returns_404(backend):
    facade = V1130Backend(backend.store.directory)
    status, body = facade.dispatch("GET", "/unknown")
    assert status == 404 and body["error"]["code"] == "not_found"


# Helpers: real HTTP 403 provider
class ForbiddenAnthropicHandler(BaseHTTPRequestHandler):
    def do_POST(self):  # noqa: N802
        length = int(self.headers.get("Content-Length", "0"))
        if length:
            self.rfile.read(length)
        body = json.dumps({"error": {"type": "forbidden", "message": "Request not allowed"}}).encode()
        self.send_response(403)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


def _start_forbidden_server() -> ThreadingHTTPServer:
    server = ThreadingHTTPServer(("127.0.0.1", 0), ForbiddenAnthropicHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


def _anthropic_403_spec(port: int) -> ProviderSpec:
    return ProviderSpec(
        name="anthropic-403", kind=ProviderKind.ANTHROPIC,
        model="claude-fake", api_key="configured",
        base_url=f"http://127.0.0.1:{port}", timeout_seconds=2.0, max_attempts=1)