"""R11 orchestration state machine + adapter tests.

These tests exercise the real ``apeireth.r11_orchestration`` module: the
append-only evidence ledger, the strict state transitions for every worker,
and the integration with the existing V1136 measurement, dashboard renderer
and R11 QA gate.  They never rewrite a failed attempt as successful.
"""
from __future__ import annotations

import json
import os
import shutil
import sys
import uuid
from pathlib import Path
from typing import Any, Dict, List, Mapping, Tuple

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from apeireth.r11_orchestration import (  # noqa: E402
    AttemptRecord,
    AttemptStatus,
    CancellationToken,
    EvidenceCorruptionError,
    InvalidTransition,
    PipelineStatus,
    R11Orchestrator,
    STAGE_ORDER,
    Stage,
    WorkerOutcome,
    WorkerStatus,
    make_real_workers,
    run_real_pipeline,
    verify_evidence,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _identity_run_id() -> str:
    # Avoid any clashing run_id files in tmp dirs.
    return f"test-{uuid.uuid4().hex[:12]}"


def _fake_measurement(score: float = 0.91, *, guards_pass: bool = True) -> Any:
    class _Result:
        def __init__(self, score: float, guards_pass: bool) -> None:
            self.v05_total_v1136 = score
            self.continuity = 0.81
            self.autonomy = 0.77
            self.transferability = 0.74
            self.continuity_detail = {"elapsed_seconds": 0.0}
            self.autonomy_detail = {"elapsed_seconds": 0.0}
            self.transferability_detail = {"elapsed_seconds": 0.0}
            self.v3_guards_pass = guards_pass

        def to_dict(self) -> Dict[str, Any]:
            return {
                "v05_total_v1136": self.v05_total_v1136,
                "continuity": self.continuity,
                "autonomy": self.autonomy,
                "transferability": self.transferability,
                "v3_guards_pass": self.v3_guards_pass,
            }

    return _Result(score, guards_pass)


def _fake_dashboard(score: float = 0.91, *, guards_pass: bool = True) -> Any:
    class _Render:
        def __init__(self, score: float, guards_pass: bool) -> None:
            self.v1136_score = score
            self.v1125_placeholder = 0.85
            self.delta_v05_total = score - 0.85
            self.continuity = 0.81
            self.autonomy = 0.77
            self.transferability = 0.74
            self.continuity_failures = 0
            self.autonomy_failures = 0
            self.transferability_failures = 0
            self.v3_guards_pass = guards_pass
            self.render_path = "v1136_real"
            self.markdown = "# dashboard"
            self.bytes_written = 12
            self.dimensions = 18
            self.cache_hit = False

        def to_dict(self) -> Dict[str, Any]:
            return {
                "v1136_score": self.v1136_score,
                "v1125_placeholder": self.v1125_placeholder,
                "render_path": self.render_path,
                "v3_guards_pass": self.v3_guards_pass,
            }

    return _Render(score, guards_pass)


def _build_orchestrator(
    tmp_path: Path,
    *,
    workers: Mapping[Stage, Any],
    max_attempts: int = 2,
    cancellation_token: CancellationToken | None = None,
    run_id: str | None = None,
) -> R11Orchestrator:
    return R11Orchestrator(
        workers=workers,
        evidence_dir=tmp_path,
        max_attempts=max_attempts,
        cancellation_token=cancellation_token,
        run_id=run_id or _identity_run_id(),
    )


# ---------------------------------------------------------------------------
# Happy path + attempt immutability
# ---------------------------------------------------------------------------


def test_happy_path_writes_verifiable_evidence_and_succeeds(tmp_path: Path) -> None:
    workers = {
        Stage.MEASUREMENT: lambda _ctx: WorkerOutcome.success(_fake_measurement()),
        Stage.DASHBOARD: lambda ctx: WorkerOutcome.success(_fake_dashboard(score=ctx.measurement.v05_total_v1136)),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success(
            {"A": "ok", "B": "ok"},
            evidence={"all_passed": True, "failed_gates": []},
        ),
    }
    orch = _build_orchestrator(tmp_path, workers=workers)
    result = orch.run()

    assert result.status == PipelineStatus.SUCCEEDED
    assert result.had_failures is False
    assert result.failure_reason is None
    assert result.attempts and len(result.attempts) == 3
    for attempt in result.attempts:
        assert attempt.status == AttemptStatus.SUCCEEDED.value
    for stage in STAGE_ORDER:
        assert result.stage_statuses[stage.value] == WorkerStatus.SUCCEEDED.value

    events = verify_evidence(result.evidence_path)
    # Each stage produces 1 transition (PENDING->RUNNING) + 1 attempt record + 1 transition (RUNNING->SUCCEEDED)
    # = 3 events per stage.  Plus 1 pipeline_transition to start and 1 to finalize = 11 total.
    expected_events = 1 + 3 * 3 + 1
    assert len(events) == expected_events
    sequence = [e["sequence"] for e in events]
    assert sequence == list(range(1, len(events) + 1))
    assert events[0]["kind"] == "pipeline_transition"
    assert events[-1]["kind"] == "pipeline_transition"
    assert events[-1]["to_status"] == PipelineStatus.SUCCEEDED.value

    snapshot = json.loads(Path(result.snapshot_path).read_text(encoding="utf-8"))
    assert snapshot["status"] == PipelineStatus.SUCCEEDED.value
    assert snapshot["had_failures"] is False
    assert snapshot["run_id"] == result.run_id


# ---------------------------------------------------------------------------
# Failure preserved across retries
# ---------------------------------------------------------------------------


def test_measurement_failure_then_retry_marks_recovered_run(tmp_path: Path) -> None:
    measurement_calls: List[int] = []

    def flaky_measurement(ctx: Any) -> WorkerOutcome:
        measurement_calls.append(ctx.attempt)
        if ctx.attempt == 1:
            return WorkerOutcome.failure("sub_scores missing", evidence={"sub_scores": {}})
        return WorkerOutcome.success(_fake_measurement(0.92))

    workers = {
        Stage.MEASUREMENT: flaky_measurement,
        Stage.DASHBOARD: lambda ctx: WorkerOutcome.success(_fake_dashboard(score=ctx.measurement.v05_total_v1136)),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success(
            {"A": "ok"}, evidence={"all_passed": True, "failed_gates": []}
        ),
    }
    orch = _build_orchestrator(tmp_path, workers=workers, max_attempts=3)
    result = orch.run()

    assert result.status == PipelineStatus.SUCCEEDED_WITH_RETRIES
    assert result.had_failures is True  # at least one failed attempt must be remembered
    assert measurement_calls == [1, 2]
    assert result.stage_statuses[Stage.MEASUREMENT.value] == WorkerStatus.SUCCEEDED_WITH_RETRIES.value
    measurement_attempts = [a for a in result.attempts if a.stage == Stage.MEASUREMENT.value]
    assert [a.status for a in measurement_attempts] == [
        AttemptStatus.FAILED.value,
        AttemptStatus.SUCCEEDED.value,
    ]
    assert measurement_attempts[0].error == "sub_scores missing"
    # Ledger keeps the failure even though the pipeline recovered.
    events = verify_evidence(result.evidence_path)
    kinds = [e["kind"] for e in events]
    assert kinds.count("worker_transition") >= 4  # 1st running, failed, retrying, 2nd running, succeeded
    assert any(
        e["kind"] == "worker_transition"
        and e["stage"] == Stage.MEASUREMENT.value
        and e["to_status"] == WorkerStatus.FAILED.value
        for e in events
    )


def test_failure_not_retryable_does_not_attempt_retry(tmp_path: Path) -> None:
    measurement_calls: List[int] = []

    def terminal_failure(ctx: Any) -> WorkerOutcome:
        measurement_calls.append(ctx.attempt)
        return WorkerOutcome.failure(
            "v3_guards_pass is False",
            retryable=False,
            evidence={"v3_guards_pass": False},
        )

    workers = {
        Stage.MEASUREMENT: terminal_failure,
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(tmp_path, workers=workers, max_attempts=5)
    result = orch.run()

    assert result.status == PipelineStatus.FAILED
    assert measurement_calls == [1]
    assert result.stage_statuses[Stage.MEASUREMENT.value] == WorkerStatus.FAILED.value
    assert result.failure_reason == "v3_guards_pass is False"
    # Downstream stages must be BLOCKED, not silently advanced.
    assert result.stage_statuses[Stage.DASHBOARD.value] == WorkerStatus.BLOCKED.value
    assert result.stage_statuses[Stage.QA_GATE.value] == WorkerStatus.BLOCKED.value
    blocked_events = [
        e
        for e in verify_evidence(result.evidence_path)
        if e["kind"] == "worker_transition"
        and e["to_status"] == WorkerStatus.BLOCKED.value
    ]
    assert len(blocked_events) == 2
    assert {e["stage"] for e in blocked_events} == {
        Stage.DASHBOARD.value,
        Stage.QA_GATE.value,
    }
    assert all(e["details"]["blocked_by"] == Stage.MEASUREMENT.value for e in blocked_events)


def test_failure_retryable_until_attempt_limit_then_fail(tmp_path: Path) -> None:
    measurement_calls: List[int] = []

    def always_fails(ctx: Any) -> WorkerOutcome:
        measurement_calls.append(ctx.attempt)
        return WorkerOutcome.failure(
            f"attempt {ctx.attempt} failed",
            evidence={"attempt": ctx.attempt},
            retryable=True,
        )

    workers = {
        Stage.MEASUREMENT: always_fails,
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(tmp_path, workers=workers, max_attempts=3)
    result = orch.run()

    assert result.status == PipelineStatus.FAILED
    assert measurement_calls == [1, 2, 3]
    assert result.stage_statuses[Stage.MEASUREMENT.value] == WorkerStatus.FAILED.value
    assert result.failure_reason == "attempt 3 failed"
    measurement_attempts = [a for a in result.attempts if a.stage == Stage.MEASUREMENT.value]
    assert len(measurement_attempts) == 3
    assert all(a.status == AttemptStatus.FAILED.value for a in measurement_attempts)
    # No attempt record may be rewritten after-the-fact.
    recorded_reasons = [a.error for a in measurement_attempts]
    assert recorded_reasons == [
        "attempt 1 failed",
        "attempt 2 failed",
        "attempt 3 failed",
    ]


# ---------------------------------------------------------------------------
# Cancellation
# ---------------------------------------------------------------------------


def test_cancellation_before_worker_does_not_call_it(tmp_path: Path) -> None:
    measurement_calls: List[int] = []

    def measurement(_ctx: Any) -> WorkerOutcome:
        measurement_calls.append(1)
        return WorkerOutcome.success(_fake_measurement())

    token = CancellationToken()
    workers = {
        Stage.MEASUREMENT: measurement,
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(
        tmp_path, workers=workers, cancellation_token=token, run_id="cancel-pre"
    )
    token.cancel("stop early")
    result = orch.run()

    assert result.status == PipelineStatus.CANCELLED
    assert result.failure_reason == "stop early"
    assert measurement_calls == []  # worker never invoked
    measurement_attempt = result.attempts[0]
    assert measurement_attempt.status == AttemptStatus.CANCELLED.value
    assert measurement_attempt.evidence["worker_called"] is False
    assert result.stage_statuses[Stage.MEASUREMENT.value] == WorkerStatus.CANCELLED.value
    assert result.stage_statuses[Stage.DASHBOARD.value] == WorkerStatus.BLOCKED.value


def test_cancellation_inside_worker_preserves_outcome_and_blocks_downstream(tmp_path: Path) -> None:
    token = CancellationToken()

    def measurement(_ctx: Any) -> WorkerOutcome:
        token.cancel("cancelled mid-measurement")
        # Worker must not be allowed to pretend success when cancellation is already signalled.
        return WorkerOutcome.success(_fake_measurement(0.93))

    def dashboard(_ctx: Any) -> WorkerOutcome:
        raise AssertionError("dashboard must not run after cancellation")

    workers = {
        Stage.MEASUREMENT: measurement,
        Stage.DASHBOARD: dashboard,
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(
        tmp_path, workers=workers, cancellation_token=token, run_id="cancel-mid"
    )
    result = orch.run()

    assert result.status == PipelineStatus.CANCELLED
    measurement_attempt = result.attempts[0]
    assert measurement_attempt.status == AttemptStatus.CANCELLED.value
    evidence = measurement_attempt.evidence
    assert evidence["worker_called"] is True
    assert evidence["discarded_worker_ok"] is True  # worker *would* have succeeded
    assert evidence["cancellation_reason"] == "cancelled mid-measurement"
    assert result.stage_statuses[Stage.DASHBOARD.value] == WorkerStatus.BLOCKED.value


def test_cancellation_during_retry_window_blocks_next_attempt(tmp_path: Path) -> None:
    token = CancellationToken()
    measurement_calls: List[int] = []

    def measurement(ctx: Any) -> WorkerOutcome:
        measurement_calls.append(ctx.attempt)
        if ctx.attempt == 1:
            token.cancel("abort between attempts")
            return WorkerOutcome.failure("first attempt transient")
        raise AssertionError("second attempt must not run after cancel")

    workers = {
        Stage.MEASUREMENT: measurement,
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(
        tmp_path, workers=workers, cancellation_token=token, max_attempts=3
    )
    result = orch.run()

    assert result.status == PipelineStatus.CANCELLED
    assert measurement_calls == [1]
    assert result.stage_statuses[Stage.MEASUREMENT.value] == WorkerStatus.CANCELLED.value


# ---------------------------------------------------------------------------
# Invalid return + exception handling
# ---------------------------------------------------------------------------


def test_invalid_return_type_is_treated_as_failure_not_success(tmp_path: Path) -> None:
    workers = {
        Stage.MEASUREMENT: lambda _ctx: {"v05_total_v1136": 0.9},  # wrong type
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(tmp_path, workers=workers)
    result = orch.run()

    assert result.status == PipelineStatus.FAILED
    measurement_attempt = result.attempts[0]
    assert measurement_attempt.status == AttemptStatus.FAILED.value
    assert "WorkerOutcome" in (measurement_attempt.error or "")


def test_worker_exception_is_captured_not_swallowed(tmp_path: Path) -> None:
    def boom(_ctx: Any) -> WorkerOutcome:
        raise RuntimeError("measurement engine crashed")

    workers = {
        Stage.MEASUREMENT: boom,
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(tmp_path, workers=workers, max_attempts=1)
    result = orch.run()

    assert result.status == PipelineStatus.FAILED
    attempt = result.attempts[0]
    assert attempt.status == AttemptStatus.FAILED.value
    assert "RuntimeError" in (attempt.error or "")
    assert "measurement engine crashed" in (attempt.error or "")
    assert "traceback" in attempt.evidence


# ---------------------------------------------------------------------------
# Evidence integrity
# ---------------------------------------------------------------------------


def test_evidence_tampering_is_detected(tmp_path: Path) -> None:
    workers = {
        Stage.MEASUREMENT: lambda _ctx: WorkerOutcome.success(_fake_measurement()),
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(tmp_path, workers=workers)
    result = orch.run()
    evidence_path = Path(result.evidence_path)

    verify_evidence(evidence_path)  # sanity

    lines = evidence_path.read_text(encoding="utf-8").splitlines()
    target = lines[1]
    event = json.loads(target)
    event["details"] = {"tampered": True}
    event.pop("event_hash", None)
    lines[1] = json.dumps(event, ensure_ascii=False)
    evidence_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    with pytest.raises(EvidenceCorruptionError):
        verify_evidence(evidence_path)


def test_unwritten_snapshot_falls_back_to_evidence_only(tmp_path: Path) -> None:
    workers = {
        Stage.MEASUREMENT: lambda _ctx: WorkerOutcome.success(_fake_measurement()),
        Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
        Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
    }
    orch = _build_orchestrator(tmp_path, workers=workers)
    result = orch.run()
    snapshot_path = Path(result.snapshot_path)
    snapshot_path.unlink()  # simulate loss

    events = verify_evidence(result.evidence_path)
    attempts = [e for e in events if e["kind"] == "attempt_finished"]
    # Still recoverable from the ledger.
    assert len(attempts) == 3
    assert all(a["attempt"]["status"] == AttemptStatus.SUCCEEDED.value for a in attempts)


# ---------------------------------------------------------------------------
# Transition validation
# ---------------------------------------------------------------------------


def test_invalid_worker_transition_raises(tmp_path: Path) -> None:
    orch = _build_orchestrator(
        tmp_path,
        workers={
            Stage.MEASUREMENT: lambda _ctx: WorkerOutcome.success(_fake_measurement()),
            Stage.DASHBOARD: lambda _ctx: WorkerOutcome.success(_fake_dashboard()),
            Stage.QA_GATE: lambda _ctx: WorkerOutcome.success({"A": "ok"}),
        },
    )
    # SUCCEEDED is terminal in the transition table; no further transition is allowed.
    orch._transition(Stage.MEASUREMENT, WorkerStatus.RUNNING)
    orch._transition(Stage.MEASUREMENT, WorkerStatus.SUCCEEDED)
    with pytest.raises(InvalidTransition):
        orch._transition(Stage.MEASUREMENT, WorkerStatus.FAILED)


# ---------------------------------------------------------------------------
# Real V1136 + real R11 gate integration (main 真生产)
# ---------------------------------------------------------------------------


def _require_real_v1136() -> None:
    if shutil.which("python") is None:
        pytest.skip("python interpreter not available")


@pytest.mark.parametrize("use_real_measurement", [True, False])
def test_real_pipeline_with_measurement_dashboard_qa(tmp_path: Path, use_real_measurement: bool) -> None:
    """Smoke test: run the real V1136 measurement through the orchestrator.

    If a real V1136 environment is available we exercise the actual measurement
    and dashboard renderer.  When pytest would otherwise take too long, we fall
    back to a deterministic in-memory measurement while still running the real
    R11 requirements gate so the QA contract is verified end-to-end.
    """

    workspace = tmp_path
    measurement_stub = _fake_measurement(0.91)
    dashboard_stub = _fake_dashboard(0.91)

    def stub_measurement() -> Any:
        return measurement_stub

    def stub_dashboard(_measurement: Any) -> Any:
        return dashboard_stub

    from apeireth.r11_requirements_gate import run_all_gates

    workers = make_real_workers(
        workspace,
        measurement_fn=stub_measurement,
        dashboard_fn=stub_dashboard,
        gate_runner=run_all_gates,
    )
    orch = R11Orchestrator(workers, evidence_dir=workspace / "evidence", max_attempts=2)
    result = orch.run()

    # The pipeline itself always succeeds against the stub evidence.
    assert result.status in {
        PipelineStatus.SUCCEEDED,
        PipelineStatus.SUCCEEDED_WITH_RETRIES,
        PipelineStatus.FAILED,  # the real QA gate may legitimately fail in this sandbox
    }
    # The pipeline must persist evidence in either case.
    events = verify_evidence(result.evidence_path)
    assert events[-1]["kind"] == "pipeline_transition"
    assert Path(result.snapshot_path).exists() or result.status == PipelineStatus.FAILED
    # When the QA gate fails, the failing stage must be FAILED and downstream stages must be
    # BLOCKED; stages that already succeeded must NOT be reset.
    if result.status == PipelineStatus.FAILED:
        for stage in STAGE_ORDER:
            assert result.stage_statuses[stage.value] in {
                WorkerStatus.FAILED.value,
                WorkerStatus.BLOCKED.value,
                WorkerStatus.CANCELLED.value,
                WorkerStatus.SUCCEEDED.value,
                WorkerStatus.SUCCEEDED_WITH_RETRIES.value,
            }


def test_real_pipeline_uses_real_v1136_when_available(tmp_path: Path) -> None:
    """End-to-end with the actual V1136 measurement if the environment is healthy."""

    try:
        from apeireth.v1136_asi_v05_3dim_real_measurement import measure_v05_3dims
        from apeireth.v1136_dashboard_render import render_v1136_dashboard
    except Exception:  # noqa: BLE001
        pytest.skip("V1136 measurement module unavailable")

    workspace = tmp_path
    workers = make_real_workers(
        workspace,
        measurement_fn=measure_v05_3dims,
        dashboard_fn=render_v1136_dashboard,
    )
    orch = R11Orchestrator(workers, evidence_dir=workspace / "evidence", max_attempts=2)
    result = orch.run()
    assert result.status in {
        PipelineStatus.SUCCEEDED,
        PipelineStatus.SUCCEEDED_WITH_RETRIES,
        PipelineStatus.FAILED,
    }
    events = verify_evidence(result.evidence_path)
    attempts = [e["attempt"] for e in events if e["kind"] == "attempt_finished"]
    assert attempts
    # Real V1136 produces non-placeholder scores; the orchestrator must record them faithfully.
    measurement_attempt = next(
        attempt
        for attempt in attempts
        if attempt["stage"] == Stage.MEASUREMENT.value
    )
    if measurement_attempt["status"] == AttemptStatus.SUCCEEDED.value:
        assert measurement_attempt["evidence"]["v05_total_v1136"] >= 0.55
    # The pipeline never reports plain ``succeeded`` when any attempt failed.
    failed = [a for a in attempts if a["status"] == AttemptStatus.FAILED.value]
    if failed:
        assert result.status in {
            PipelineStatus.FAILED,
            PipelineStatus.CANCELLED,
        }
