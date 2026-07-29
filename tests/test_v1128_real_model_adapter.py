"""V1128 W2 tests: real processes, files, ports and configured provider probes; no mocks."""
from __future__ import annotations

import json
import os
import socket
import sys
import threading
import time
from pathlib import Path

import pytest

from apeireth.v1124_asi_north_star_backend import ASINorthStarBackend, V1124Error
from apeireth.v1128_real_model_adapter_w2 import (
    DEFAULT_OLLAMA_MODELS,
    V1128_VERSION,
    W2_TARGET,
    AttemptEvidence,
    HealthEvidence,
    IsolatedProcessRunner,
    OllamaRuntime,
    ProviderKind,
    ProviderSpec,
    ProviderState,
    ProviderUnavailable,
    RoutedEvidence,
    V3_GUARDS,
    W2MeasurementCoordinator,
    W2ProviderAdapter,
    default_provider_specs,
)


def stdin_command(prefix: str = "EXEC") -> tuple[str, ...]:
    return (sys.executable, "-c", f"import sys; print('{prefix}:'+sys.stdin.read())")


def cli_command() -> tuple[str, ...]:
    return (sys.executable, "-c", "import sys; print('CLI:'+sys.argv[1])", "{prompt}")


def fail_command() -> tuple[str, ...]:
    return (sys.executable, "-c", "import sys; print('crashed',file=sys.stderr);sys.exit(9)")


def sleep_command() -> tuple[str, ...]:
    return (sys.executable, "-c", "import time;time.sleep(5);print('late')")


def executable_spec(name="exec", command=None, attempts=1, timeout=2.0):
    return ProviderSpec(name, ProviderKind.EXECUTABLE, "real-python-process",
                        command=command or stdin_command(), max_attempts=attempts,
                        timeout_seconds=timeout)


def cli_spec(name="cli", command=None):
    return ProviderSpec(name, ProviderKind.LOCAL_CLI, "real-python-cli", command=command or cli_command())


@pytest.fixture
def backend(tmp_path: Path):
    return ASINorthStarBackend(tmp_path / "backend")


# Constants, DTO contracts, validation (13)
def test_version_and_w2_target():
    assert V1128_VERSION == "0.1.0" and W2_TARGET == 0.90


def test_required_ollama_models_named():
    assert DEFAULT_OLLAMA_MODELS == ("qwen2.5:1.5b", "llama3.2:3b")


def test_v3_guards_cover_no_fabrication():
    assert len(V3_GUARDS) >= 5 and "fallback_is_not_model_output" in V3_GUARDS


def test_provider_kinds_are_four_real_paths():
    assert {kind.value for kind in ProviderKind} == {"anthropic", "ollama", "local_cli", "executable"}


def test_spec_public_redacts_key():
    public = ProviderSpec("a", ProviderKind.ANTHROPIC, "claude", api_key="secret").public()
    assert public["credential_configured"] is True and "secret" not in json.dumps(public)


def test_spec_rejects_empty_name():
    with pytest.raises(ValueError):
        ProviderSpec("", ProviderKind.EXECUTABLE, "m", command=stdin_command()).validate()


def test_spec_rejects_empty_model():
    with pytest.raises(ValueError):
        ProviderSpec("x", ProviderKind.EXECUTABLE, "", command=stdin_command()).validate()


def test_spec_rejects_bad_timeout():
    with pytest.raises(ValueError):
        ProviderSpec("x", ProviderKind.EXECUTABLE, "m", timeout_seconds=0).validate()


def test_spec_rejects_too_many_attempts():
    with pytest.raises(ValueError):
        ProviderSpec("x", ProviderKind.EXECUTABLE, "m", max_attempts=6).validate()


def test_health_public_serializes_enum():
    result = HealthEvidence("x", ProviderState.HEALTHY, 1.0, "ok").public()
    assert result["healthy"] and result["state"] == "healthy"


def test_attempt_public_serializes_enum():
    result = AttemptEvidence("x", False, ProviderState.UNAVAILABLE, 2.0, "down").public()
    assert result["state"] == "unavailable" and not result["success"]


def test_routed_failure_is_honest():
    routed = RoutedEvidence(None, None, (), False)
    assert not routed.success and routed.public()["evidence"] is None


def test_default_specs_always_expose_four_paths():
    specs = default_provider_specs()
    assert len(specs) == 4 and {spec.kind for spec in specs} == set(ProviderKind)


# Real isolated process execution (11)
def test_executable_reads_real_stdin():
    content, latency = IsolatedProcessRunner.run(stdin_command(), "hello", 2)
    assert content == "EXEC:hello" and latency > 0


def test_cli_wrapper_passes_real_argument():
    content, _ = IsolatedProcessRunner.run(cli_command(), "hello world", 2, prompt_as_argument=True)
    assert content == "CLI:hello world"


def test_cli_without_placeholder_appends_prompt():
    command = (sys.executable, "-c", "import sys;print(sys.argv[1])")
    content, _ = IsolatedProcessRunner.run(command, "appended", 2, prompt_as_argument=True)
    assert content == "appended"


def test_process_nonzero_is_not_success():
    with pytest.raises(ProviderUnavailable) as error:
        IsolatedProcessRunner.run(fail_command(), "x", 2)
    assert error.value.code == "provider_process_error" and "crashed" in str(error.value)


def test_process_timeout_kills_child():
    started = time.perf_counter()
    with pytest.raises(ProviderUnavailable) as error:
        IsolatedProcessRunner.run(sleep_command(), "x", 0.1)
    assert error.value.code == "provider_timeout" and time.perf_counter() - started < 3


def test_process_empty_stdout_rejected():
    command = (sys.executable, "-c", "pass")
    with pytest.raises(ProviderUnavailable) as error:
        IsolatedProcessRunner.run(command, "x", 2)
    assert error.value.code == "provider_invalid_response"


def test_process_missing_executable_rejected():
    with pytest.raises(ProviderUnavailable) as error:
        IsolatedProcessRunner.run(("definitely-missing-v1128-binary",), "x", 1)
    assert error.value.code == "provider_not_configured"


def test_process_unicode_stdout_parsed():
    content, _ = IsolatedProcessRunner.run(stdin_command("楚零"), "真实", 2)
    assert content == "楚零:真实"


def test_process_multiline_stdout_preserved():
    command = (sys.executable, "-c", "print('a');print('b')")
    content, _ = IsolatedProcessRunner.run(command, "x", 2)
    assert content == "a\nb"


def test_process_output_limit_enforced():
    command = (sys.executable, "-c", "print('x'*1048577)")
    with pytest.raises(ProviderUnavailable) as error:
        IsolatedProcessRunner.run(command, "x", 3)
    assert error.value.code == "provider_response_too_large"


def test_process_is_distinct_pid():
    command = (sys.executable, "-c", "import os;print(os.getpid())")
    content, _ = IsolatedProcessRunner.run(command, "x", 2)
    assert int(content) != os.getpid()


# Honest health: actual filesystem, credentials and network probes (10)
def test_executable_shallow_health_configured():
    health = W2ProviderAdapter().health(executable_spec(), deep=False)
    assert health.state == ProviderState.CONFIGURED


def test_executable_deep_health_real_inference():
    health = W2ProviderAdapter().health(executable_spec(), deep=True)
    assert health.healthy and health.latency_ms > 0


def test_cli_deep_health_real_inference():
    health = W2ProviderAdapter().health(cli_spec(), deep=True)
    assert health.healthy


def test_missing_local_command_unconfigured():
    spec = ProviderSpec("missing", ProviderKind.EXECUTABLE, "none")
    assert W2ProviderAdapter().health(spec).state == ProviderState.UNCONFIGURED


def test_missing_binary_unconfigured():
    spec = executable_spec(command=("v1128-missing-program",))
    assert W2ProviderAdapter().health(spec).state == ProviderState.UNCONFIGURED


def test_anthropic_absent_key_honest(monkeypatch):
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    spec = ProviderSpec("anthropic-none", ProviderKind.ANTHROPIC, "claude")
    health = W2ProviderAdapter().health(spec, deep=True)
    assert health.state == ProviderState.UNCONFIGURED and "absent" in health.detail


def test_anthropic_present_key_not_claimed_valid_without_call():
    spec = ProviderSpec("anthropic-set", ProviderKind.ANTHROPIC, "claude", api_key="configured-not-validated")
    health = W2ProviderAdapter().health(spec, deep=False)
    assert health.state == ProviderState.CONFIGURED and not health.healthy


def test_ollama_real_probe_reports_fact():
    spec = ProviderSpec("ollama-real", ProviderKind.OLLAMA, DEFAULT_OLLAMA_MODELS[0],
                        base_url=os.getenv("OLLAMA_BASE_URL") or "http://127.0.0.1:11434", timeout_seconds=1)
    health = OllamaRuntime().probe(spec, timeout=0.5)
    assert health.state in {ProviderState.HEALTHY, ProviderState.UNAVAILABLE}
    if health.healthy:
        assert "real GET" in health.detail
    else:
        assert "failed" in health.detail


def test_ollama_remote_is_not_auto_started():
    spec = ProviderSpec("remote", ProviderKind.OLLAMA, "qwen", base_url="http://127.0.0.1:1", auto_start=True)
    health = OllamaRuntime(executable=sys.executable).ensure_running(spec, startup_timeout=0.1)
    # 127.0.0.1 is local; python serve exits, and must be reported unavailable.
    assert not health.healthy


def test_ollama_missing_binary_reports_unconfigured():
    spec = ProviderSpec("ollama-none", ProviderKind.OLLAMA, "qwen", base_url="http://127.0.0.1:1", auto_start=True)
    health = OllamaRuntime(executable="").ensure_running(spec, startup_timeout=0.1)
    assert health.state in {ProviderState.UNCONFIGURED, ProviderState.UNAVAILABLE}


# Adapter calls, fallback, retry and circuit recovery (12)
def test_executable_adapter_real_call():
    result = W2ProviderAdapter().call(executable_spec(), "ping")
    assert result.real and result.transport == "process_stdin" and result.content == "EXEC:ping"


def test_cli_adapter_real_call():
    result = W2ProviderAdapter().call(cli_spec(), "ping")
    assert result.real and result.transport == "cli_argument" and result.content == "CLI:ping"


def test_empty_prompt_rejected():
    with pytest.raises(V1124Error) as error:
        W2ProviderAdapter().call(executable_spec(), " ")
    assert error.value.code == "invalid_prompt"


def test_retry_really_attempts_twice(tmp_path):
    counter = tmp_path / "count"
    script = tmp_path / "fail.py"
    script.write_text("from pathlib import Path\np=Path(r'%s');n=int(p.read_text())+1;p.write_text(str(n));raise SystemExit(2)\n" % counter)
    counter.write_text("0")
    spec = executable_spec(command=(sys.executable, str(script)), attempts=2)
    with pytest.raises(V1124Error):
        W2ProviderAdapter(failure_threshold=5).call(spec, "x")
    assert counter.read_text() == "2"


def test_circuit_opens_after_real_process_crashes():
    adapter = W2ProviderAdapter(failure_threshold=2, recovery_seconds=10)
    spec = executable_spec(name="crasher", command=fail_command())
    for _ in range(2):
        with pytest.raises(V1124Error):
            adapter.call(spec, "x")
    assert adapter.circuit("crasher").state == "open"


def test_open_circuit_rejects_without_new_process(tmp_path):
    marker = tmp_path / "runs"
    script = tmp_path / "fail.py"
    script.write_text("from pathlib import Path\np=Path(r'%s');p.write_text(p.read_text()+'x');raise SystemExit(1)\n" % marker)
    marker.write_text("")
    adapter = W2ProviderAdapter(failure_threshold=1, recovery_seconds=10)
    spec = executable_spec(name="open", command=(sys.executable, str(script)))
    with pytest.raises(V1124Error): adapter.call(spec, "x")
    with pytest.raises(V1124Error) as error: adapter.call(spec, "x")
    assert error.value.code == "circuit_open" and marker.read_text() == "x"


def test_circuit_half_open_recovers_with_real_process():
    adapter = W2ProviderAdapter(failure_threshold=1, recovery_seconds=0.02)
    bad = executable_spec(name="recover", command=fail_command())
    with pytest.raises(V1124Error): adapter.call(bad, "x")
    time.sleep(0.03)
    result = adapter.call(executable_spec(name="recover"), "ok")
    assert result.content == "EXEC:ok" and adapter.circuit("recover").state == "closed"


def test_route_first_success_no_fallback():
    routed = W2ProviderAdapter().route([executable_spec()], "x")
    assert routed.success and not routed.fallback_used and routed.selected_provider == "exec"


def test_route_crash_falls_to_real_second_process():
    first = executable_spec(name="bad", command=fail_command())
    second = executable_spec(name="good")
    routed = W2ProviderAdapter().route([first, second], "fallback")
    assert routed.success and routed.fallback_used and routed.selected_provider == "good"
    assert [attempt.success for attempt in routed.attempts] == [False, True]


def test_route_all_fail_returns_no_fabricated_evidence():
    routed = W2ProviderAdapter().route([
        executable_spec(name="bad1", command=fail_command()),
        executable_spec(name="bad2", command=fail_command()),
    ], "x")
    assert not routed.success and routed.evidence is None and routed.selected_provider is None


def test_anthropic_to_local_fallback_without_key(monkeypatch):
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    anthropic = ProviderSpec("anthropic", ProviderKind.ANTHROPIC, "claude")
    routed = W2ProviderAdapter().route([anthropic, executable_spec(name="local")], "x")
    assert routed.success and routed.selected_provider == "local"
    assert routed.attempts[0].state == ProviderState.UNCONFIGURED


def test_circuit_health_reports_open():
    adapter = W2ProviderAdapter(failure_threshold=1, recovery_seconds=10)
    spec = executable_spec(name="health-open", command=fail_command())
    with pytest.raises(V1124Error): adapter.call(spec, "x")
    assert adapter.health(spec).state == ProviderState.CIRCUIT_OPEN


# V1124 integration, chaos persistence and comparison (10)
def test_coordinator_real_measurement_persists_identity(backend):
    result = W2MeasurementCoordinator(backend).measure([executable_spec()], "persist")
    assert result["route"]["success"] and result["identity_preserved"]
    assert backend.store.load().entries[-1].kind == "w2_model_measurement"


def test_chaos_provider_crash_preserves_identity(backend):
    identity_id = backend.identity.core.identity_id
    result = W2MeasurementCoordinator(backend).measure([
        executable_spec(command=fail_command())], "crash")
    assert not result["route"]["success"] and result["identity_preserved"]
    assert backend.store.load().core.identity_id == identity_id


def test_chaos_failure_writes_started_and_failed_audit(backend):
    W2MeasurementCoordinator(backend).measure([executable_spec(command=fail_command())], "crash")
    events = [record["event"] for record in backend.store.audit.records()]
    assert events[-2:] == ["w2_measurement_started", "w2_measurement_failed"]


def test_success_writes_started_snapshot_succeeded(backend):
    W2MeasurementCoordinator(backend).measure([executable_spec()], "ok")
    events = [record["event"] for record in backend.store.audit.records()]
    assert events[-3:] == ["w2_measurement_started", "identity_snapshot_committed", "w2_measurement_succeeded"]


def test_failure_does_not_add_fake_identity_entry(backend):
    before = len(backend.identity.entries)
    W2MeasurementCoordinator(backend).measure([executable_spec(command=fail_command())], "fail")
    assert len(backend.identity.entries) == before


def test_measure_requires_provider(backend):
    with pytest.raises(ValueError):
        W2MeasurementCoordinator(backend).measure([], "x")


def test_comparison_has_real_success_and_honest_failure(backend):
    coordinator = W2MeasurementCoordinator(backend)
    table = coordinator.compare([executable_spec(name="ok"),
                                 executable_spec(name="bad", command=fail_command())], "compare")
    assert table["providers_attempted"] == 2 and table["providers_succeeded"] == 1
    assert table["rows"][0]["asi_level_proxy"] == 0.8538
    assert table["rows"][1]["asi_level_proxy"] is None


def test_comparison_never_claims_w2_target(backend):
    table = W2MeasurementCoordinator(backend).compare([executable_spec()], "x")
    assert table["target_claimed"] is False and table["w2_target"] == 0.90


def test_comparison_contains_no_raw_content(backend):
    table = W2MeasurementCoordinator(backend).compare([executable_spec()], "secret")
    assert "EXEC:secret" not in json.dumps(table)
    assert len(table["rows"][0]["content_sha256"]) == 64


def test_backend_restart_after_provider_crash(backend, tmp_path):
    directory = backend.store.directory
    identity_id = backend.identity.core.identity_id
    W2MeasurementCoordinator(backend).measure([executable_spec(command=fail_command())], "boom")
    restarted = ASINorthStarBackend(directory)
    assert restarted.identity.core.identity_id == identity_id


# Optional genuine external-provider acceptance: never mocked, honest skip/failure.
def test_configured_anthropic_real_probe_is_truthful():
    key = os.getenv("ANTHROPIC_API_KEY")
    if not key:
        pytest.skip("ANTHROPIC_API_KEY unavailable; absence already covered")
    spec = ProviderSpec("anthropic-live", ProviderKind.ANTHROPIC,
                        os.getenv("ANTHROPIC_MODEL", "claude-3-5-haiku-20241022"),
                        api_key=key, timeout_seconds=20, max_attempts=1)
    health = W2ProviderAdapter().health(spec, deep=True)
    assert health.state in {ProviderState.HEALTHY, ProviderState.FORBIDDEN,
                            ProviderState.UNAVAILABLE, ProviderState.DEGRADED}
    if health.state == ProviderState.HEALTHY:
        assert "real inference succeeded" in health.detail


def test_installed_ollama_required_models_real_probe():
    runtime = OllamaRuntime()
    spec = ProviderSpec("ollama-live", ProviderKind.OLLAMA, DEFAULT_OLLAMA_MODELS[0],
                        base_url=os.getenv("OLLAMA_BASE_URL"), auto_start=True, timeout_seconds=30)
    health = runtime.ensure_running(spec, startup_timeout=1.0)
    try:
        assert health.state in {ProviderState.HEALTHY, ProviderState.UNCONFIGURED, ProviderState.UNAVAILABLE}
        if health.healthy:
            # If installed, run a real configured model or honestly record it missing.
            if any(model.startswith(DEFAULT_OLLAMA_MODELS[0]) for model in health.models):
                evidence = W2ProviderAdapter(ollama=runtime).call(spec, "Reply with W2_OK")
                assert evidence.real and evidence.provider == "ollama"
            else:
                assert DEFAULT_OLLAMA_MODELS[0] not in health.models
    finally:
        runtime.stop_started_process()
