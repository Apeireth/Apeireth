"""R11 measurement -> dashboard -> QA orchestration with durable evidence.

The pipeline deliberately keeps every worker attempt immutable.  A failed attempt may
be followed by a new retry attempt, but it is never rewritten as successful; therefore
a recovered run ends as ``succeeded_with_retries`` rather than plain ``succeeded``.

The event ledger is append-only JSONL with a SHA-256 hash chain.  It remains useful
when a worker fails or the run is cancelled before the final snapshot is written.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import threading
import time
import traceback
import uuid
from dataclasses import asdict, dataclass, field, is_dataclass
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, Mapping, Optional, Sequence, Tuple


SCHEMA_VERSION = "r11-orchestration-v1"
STAGE_ORDER: Tuple["Stage", ...]


class Stage(str, Enum):
    MEASUREMENT = "measurement"
    DASHBOARD = "dashboard"
    QA_GATE = "qa_gate"


STAGE_ORDER = (Stage.MEASUREMENT, Stage.DASHBOARD, Stage.QA_GATE)


class WorkerStatus(str, Enum):
    PENDING = "pending"
    RUNNING = "running"
    RETRYING = "retrying"
    SUCCEEDED = "succeeded"
    SUCCEEDED_WITH_RETRIES = "succeeded_with_retries"
    FAILED = "failed"
    CANCELLED = "cancelled"
    BLOCKED = "blocked"


class AttemptStatus(str, Enum):
    SUCCEEDED = "succeeded"
    FAILED = "failed"
    CANCELLED = "cancelled"


class PipelineStatus(str, Enum):
    PENDING = "pending"
    RUNNING = "running"
    SUCCEEDED = "succeeded"
    SUCCEEDED_WITH_RETRIES = "succeeded_with_retries"
    FAILED = "failed"
    CANCELLED = "cancelled"


_TERMINAL_PIPELINE_STATUSES = {
    PipelineStatus.SUCCEEDED,
    PipelineStatus.SUCCEEDED_WITH_RETRIES,
    PipelineStatus.FAILED,
    PipelineStatus.CANCELLED,
}

_ALLOWED_WORKER_TRANSITIONS = {
    WorkerStatus.PENDING: {WorkerStatus.RUNNING, WorkerStatus.CANCELLED, WorkerStatus.BLOCKED},
    WorkerStatus.RUNNING: {
        WorkerStatus.SUCCEEDED,
        WorkerStatus.SUCCEEDED_WITH_RETRIES,
        WorkerStatus.FAILED,
        WorkerStatus.CANCELLED,
    },
    WorkerStatus.FAILED: {WorkerStatus.RETRYING, WorkerStatus.CANCELLED},
    WorkerStatus.RETRYING: {WorkerStatus.RUNNING, WorkerStatus.CANCELLED},
    WorkerStatus.SUCCEEDED: set(),
    WorkerStatus.SUCCEEDED_WITH_RETRIES: set(),
    WorkerStatus.CANCELLED: set(),
    WorkerStatus.BLOCKED: set(),
}


class InvalidTransition(RuntimeError):
    """Raised when code attempts an undeclared worker transition."""


class EvidenceCorruptionError(RuntimeError):
    """Raised when the append-only evidence hash chain cannot be verified."""


@dataclass(frozen=True)
class WorkerOutcome:
    """Explicit worker decision; raw truthy/falsy values are never used as gate status."""

    ok: bool
    value: Any = None
    evidence: Mapping[str, Any] = field(default_factory=dict)
    reason: Optional[str] = None
    retryable: bool = True

    @classmethod
    def success(cls, value: Any = None, evidence: Optional[Mapping[str, Any]] = None) -> "WorkerOutcome":
        return cls(ok=True, value=value, evidence=evidence or {})

    @classmethod
    def failure(
        cls,
        reason: str,
        *,
        value: Any = None,
        evidence: Optional[Mapping[str, Any]] = None,
        retryable: bool = True,
    ) -> "WorkerOutcome":
        if not reason:
            raise ValueError("failure reason must not be empty")
        return cls(
            ok=False,
            value=value,
            evidence=evidence or {},
            reason=reason,
            retryable=retryable,
        )


@dataclass(frozen=True)
class WorkerContext:
    run_id: str
    stage: Stage
    attempt: int
    measurement: Any = None
    dashboard: Any = None


Worker = Callable[[WorkerContext], WorkerOutcome]


@dataclass(frozen=True)
class AttemptRecord:
    stage: str
    attempt: int
    status: str
    started_at: float
    finished_at: float
    evidence: Mapping[str, Any]
    error: Optional[str] = None
    retryable: bool = False

    def to_dict(self) -> Dict[str, Any]:
        return {
            "stage": self.stage,
            "attempt": self.attempt,
            "status": self.status,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": round(max(0.0, self.finished_at - self.started_at), 6),
            "evidence": _jsonable(self.evidence),
            "error": self.error,
            "retryable": self.retryable,
        }


@dataclass(frozen=True)
class PipelineResult:
    run_id: str
    status: PipelineStatus
    stage_statuses: Mapping[str, str]
    attempts: Tuple[AttemptRecord, ...]
    stage_evidence: Mapping[str, Any]
    evidence_path: str
    snapshot_path: str
    started_at: float
    finished_at: float
    failure_reason: Optional[str]
    outputs: Mapping[str, Any] = field(repr=False, compare=False)

    @property
    def succeeded(self) -> bool:
        return self.status in {
            PipelineStatus.SUCCEEDED,
            PipelineStatus.SUCCEEDED_WITH_RETRIES,
        }

    @property
    def had_failures(self) -> bool:
        return any(a.status == AttemptStatus.FAILED.value for a in self.attempts)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "schema_version": SCHEMA_VERSION,
            "run_id": self.run_id,
            "status": self.status.value,
            "stage_statuses": dict(self.stage_statuses),
            "attempts": [a.to_dict() for a in self.attempts],
            "stage_evidence": _jsonable(self.stage_evidence),
            "evidence_path": self.evidence_path,
            "snapshot_path": self.snapshot_path,
            "started_at": self.started_at,
            "finished_at": self.finished_at,
            "elapsed_seconds": round(max(0.0, self.finished_at - self.started_at), 6),
            "failure_reason": self.failure_reason,
            "had_failures": self.had_failures,
        }


class CancellationToken:
    """Thread-safe cooperative cancellation checked before and after every worker call."""

    def __init__(self) -> None:
        self._event = threading.Event()
        self._lock = threading.Lock()
        self._reason: Optional[str] = None

    def cancel(self, reason: str = "cancelled by caller") -> bool:
        if not reason:
            raise ValueError("cancellation reason must not be empty")
        with self._lock:
            if self._event.is_set():
                return False
            self._reason = reason
            self._event.set()
            return True

    @property
    def cancelled(self) -> bool:
        return self._event.is_set()

    @property
    def reason(self) -> str:
        return self._reason or "cancelled by caller"


def _jsonable(value: Any) -> Any:
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, Path):
        return str(value)
    if is_dataclass(value) and not isinstance(value, type):
        return _jsonable(asdict(value))
    if isinstance(value, Mapping):
        return {str(k): _jsonable(v) for k, v in value.items()}
    if isinstance(value, (list, tuple, set, frozenset)):
        return [_jsonable(v) for v in value]
    to_dict = getattr(value, "to_dict", None)
    if callable(to_dict):
        return _jsonable(to_dict())
    return repr(value)


def _digest_event(event_without_hash: Mapping[str, Any]) -> str:
    canonical = json.dumps(
        _jsonable(event_without_hash),
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return hashlib.sha256(canonical).hexdigest()


def verify_evidence(path: Path | str) -> Tuple[Dict[str, Any], ...]:
    """Verify sequence numbers and the full SHA-256 chain, returning parsed events."""

    evidence_path = Path(path)
    events = []
    previous_hash = "0" * 64
    with evidence_path.open("r", encoding="utf-8") as handle:
        for expected_sequence, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError as exc:
                raise EvidenceCorruptionError(
                    f"invalid JSON at evidence line {expected_sequence}: {exc}"
                ) from exc
            if event.get("sequence") != expected_sequence:
                raise EvidenceCorruptionError(
                    f"sequence mismatch at line {expected_sequence}: {event.get('sequence')}"
                )
            if event.get("prev_hash") != previous_hash:
                raise EvidenceCorruptionError(f"prev_hash mismatch at line {expected_sequence}")
            actual_hash = event.get("event_hash")
            unsigned = dict(event)
            unsigned.pop("event_hash", None)
            expected_hash = _digest_event(unsigned)
            if actual_hash != expected_hash:
                raise EvidenceCorruptionError(f"event_hash mismatch at line {expected_sequence}")
            previous_hash = actual_hash
            events.append(event)
    if not events:
        raise EvidenceCorruptionError("evidence ledger is empty")
    return tuple(events)


class R11Orchestrator:
    """Three-stage, one-shot orchestrator with durable failure/cancel/retry state."""

    def __init__(
        self,
        workers: Mapping[Stage | str, Worker],
        *,
        evidence_dir: Path | str,
        max_attempts: int | Mapping[Stage | str, int] = 2,
        cancellation_token: Optional[CancellationToken] = None,
        run_id: Optional[str] = None,
    ) -> None:
        normalised: Dict[Stage, Worker] = {}
        for key, worker in workers.items():
            stage = key if isinstance(key, Stage) else Stage(key)
            if not callable(worker):
                raise TypeError(f"worker for {stage.value} must be callable")
            normalised[stage] = worker
        missing = [stage.value for stage in STAGE_ORDER if stage not in normalised]
        if missing:
            raise ValueError(f"missing workers: {missing}")
        self._workers = normalised
        self._attempt_limits = self._normalise_attempt_limits(max_attempts)
        self.token = cancellation_token or CancellationToken()

        self.run_id = run_id or uuid.uuid4().hex
        if not re.fullmatch(r"[A-Za-z0-9._-]+", self.run_id):
            raise ValueError("run_id may contain only letters, digits, '.', '_' and '-'")
        root = Path(evidence_dir).resolve()
        root.mkdir(parents=True, exist_ok=True)
        self.evidence_path = root / f"r11-orchestration-{self.run_id}.events.jsonl"
        self.snapshot_path = root / f"r11-orchestration-{self.run_id}.snapshot.json"
        # Exclusive creation prevents accidental loss of an older failed run.
        self.evidence_path.touch(exist_ok=False)
        if self.snapshot_path.exists():
            raise FileExistsError(self.snapshot_path)

        self.status = PipelineStatus.PENDING
        self.stage_statuses: Dict[Stage, WorkerStatus] = {
            stage: WorkerStatus.PENDING for stage in STAGE_ORDER
        }
        self._attempts: list[AttemptRecord] = []
        self._outputs: Dict[Stage, Any] = {}
        self._stage_evidence: Dict[Stage, Any] = {}
        self._sequence = 0
        self._last_hash = "0" * 64
        self._started_at = 0.0
        self._finished_at = 0.0
        self._has_run = False

    @staticmethod
    def _normalise_attempt_limits(
        value: int | Mapping[Stage | str, int],
    ) -> Dict[Stage, int]:
        if isinstance(value, int):
            if value < 1:
                raise ValueError("max_attempts must be >= 1")
            return {stage: value for stage in STAGE_ORDER}
        limits = {stage: 1 for stage in STAGE_ORDER}
        for key, limit in value.items():
            stage = key if isinstance(key, Stage) else Stage(key)
            if not isinstance(limit, int) or limit < 1:
                raise ValueError(f"max_attempts for {stage.value} must be >= 1")
            limits[stage] = limit
        return limits

    def _append_event(self, kind: str, **fields: Any) -> None:
        self._sequence += 1
        event: Dict[str, Any] = {
            "schema_version": SCHEMA_VERSION,
            "sequence": self._sequence,
            "timestamp": time.time(),
            "run_id": self.run_id,
            "kind": kind,
            "prev_hash": self._last_hash,
            **_jsonable(fields),
        }
        event["event_hash"] = _digest_event(event)
        encoded = json.dumps(event, ensure_ascii=False, sort_keys=True) + "\n"
        with self.evidence_path.open("a", encoding="utf-8", newline="\n") as handle:
            handle.write(encoded)
            handle.flush()
            os.fsync(handle.fileno())
        self._last_hash = event["event_hash"]

    def _transition(self, stage: Stage, target: WorkerStatus, **details: Any) -> None:
        current = self.stage_statuses[stage]
        if target not in _ALLOWED_WORKER_TRANSITIONS[current]:
            raise InvalidTransition(f"{stage.value}: {current.value} -> {target.value}")
        self.stage_statuses[stage] = target
        self._append_event(
            "worker_transition",
            stage=stage.value,
            from_status=current.value,
            to_status=target.value,
            details=details,
        )

    def _record_attempt(self, record: AttemptRecord) -> None:
        self._attempts.append(record)
        self._append_event("attempt_finished", attempt=record.to_dict())

    def _context(self, stage: Stage, attempt: int) -> WorkerContext:
        return WorkerContext(
            run_id=self.run_id,
            stage=stage,
            attempt=attempt,
            measurement=self._outputs.get(Stage.MEASUREMENT),
            dashboard=self._outputs.get(Stage.DASHBOARD),
        )

    def _cancel_without_call(self, stage: Stage, attempt: int) -> None:
        now = time.time()
        current = self.stage_statuses[stage]
        if current == WorkerStatus.FAILED:
            self._transition(stage, WorkerStatus.CANCELLED, reason=self.token.reason)
        else:
            self._transition(stage, WorkerStatus.CANCELLED, reason=self.token.reason)
        self._record_attempt(
            AttemptRecord(
                stage=stage.value,
                attempt=attempt,
                status=AttemptStatus.CANCELLED.value,
                started_at=now,
                finished_at=now,
                evidence={"cancellation_reason": self.token.reason, "worker_called": False},
                error=self.token.reason,
            )
        )

    def _run_stage(self, stage: Stage) -> bool:
        limit = self._attempt_limits[stage]
        for attempt in range(1, limit + 1):
            if self.token.cancelled:
                self._cancel_without_call(stage, attempt)
                return False

            self._transition(stage, WorkerStatus.RUNNING, attempt=attempt)
            started = time.time()
            try:
                outcome = self._workers[stage](self._context(stage, attempt))
                if not isinstance(outcome, WorkerOutcome):
                    raise TypeError(
                        f"{stage.value} worker must return WorkerOutcome, got {type(outcome).__name__}"
                    )
            except Exception as exc:  # noqa: BLE001 - worker failures are evidence, not control flow
                outcome = WorkerOutcome.failure(
                    f"{type(exc).__name__}: {exc}",
                    evidence={
                        "exception_type": type(exc).__name__,
                        "exception_message": str(exc),
                        "traceback": traceback.format_exc(),
                    },
                    retryable=True,
                )
            finished = time.time()
            evidence = _jsonable(outcome.evidence)

            # Cancellation wins as the state, but never erases the worker outcome.
            if self.token.cancelled:
                cancellation_evidence = {
                    "cancellation_reason": self.token.reason,
                    "worker_called": True,
                    "discarded_worker_ok": outcome.ok,
                    "discarded_worker_reason": outcome.reason,
                    "worker_evidence": evidence,
                }
                self._transition(stage, WorkerStatus.CANCELLED, attempt=attempt, reason=self.token.reason)
                self._record_attempt(
                    AttemptRecord(
                        stage=stage.value,
                        attempt=attempt,
                        status=AttemptStatus.CANCELLED.value,
                        started_at=started,
                        finished_at=finished,
                        evidence=cancellation_evidence,
                        error=(
                            self.token.reason
                            if outcome.ok or not outcome.reason
                            else f"{self.token.reason}; worker_failure={outcome.reason}"
                        ),
                    )
                )
                return False

            if outcome.ok:
                terminal = (
                    WorkerStatus.SUCCEEDED_WITH_RETRIES
                    if attempt > 1
                    else WorkerStatus.SUCCEEDED
                )
                self._transition(stage, terminal, attempt=attempt)
                self._record_attempt(
                    AttemptRecord(
                        stage=stage.value,
                        attempt=attempt,
                        status=AttemptStatus.SUCCEEDED.value,
                        started_at=started,
                        finished_at=finished,
                        evidence=evidence,
                    )
                )
                self._outputs[stage] = outcome.value
                self._stage_evidence[stage] = evidence
                return True

            reason = outcome.reason or "worker returned failure without reason"
            self._transition(stage, WorkerStatus.FAILED, attempt=attempt, reason=reason)
            self._record_attempt(
                AttemptRecord(
                    stage=stage.value,
                    attempt=attempt,
                    status=AttemptStatus.FAILED.value,
                    started_at=started,
                    finished_at=finished,
                    evidence=evidence,
                    error=reason,
                    retryable=outcome.retryable,
                )
            )
            can_retry = outcome.retryable and attempt < limit
            if not can_retry:
                return False
            if self.token.cancelled:
                self._transition(stage, WorkerStatus.CANCELLED, reason=self.token.reason)
                return False
            self._transition(stage, WorkerStatus.RETRYING, failed_attempt=attempt, next_attempt=attempt + 1)
            self._append_event(
                "retry_scheduled",
                stage=stage.value,
                failed_attempt=attempt,
                next_attempt=attempt + 1,
                failure_reason=reason,
            )
        return False  # pragma: no cover - loop exits through explicit branches

    def _block_after(self, stage: Stage) -> None:
        blocked = False
        for candidate in STAGE_ORDER:
            if candidate == stage:
                blocked = True
                continue
            if blocked and self.stage_statuses[candidate] == WorkerStatus.PENDING:
                self._transition(
                    candidate,
                    WorkerStatus.BLOCKED,
                    blocked_by=stage.value,
                    upstream_status=self.stage_statuses[stage].value,
                )

    def _finalize(self, status: PipelineStatus, reason: Optional[str]) -> PipelineResult:
        if status not in _TERMINAL_PIPELINE_STATUSES:
            raise ValueError(f"not a terminal pipeline status: {status.value}")
        previous = self.status
        self.status = status
        self._finished_at = time.time()
        self._append_event(
            "pipeline_transition",
            from_status=previous.value,
            to_status=status.value,
            reason=reason,
        )
        result = PipelineResult(
            run_id=self.run_id,
            status=status,
            stage_statuses={k.value: v.value for k, v in self.stage_statuses.items()},
            attempts=tuple(self._attempts),
            stage_evidence={k.value: v for k, v in self._stage_evidence.items()},
            evidence_path=str(self.evidence_path),
            snapshot_path=str(self.snapshot_path),
            started_at=self._started_at,
            finished_at=self._finished_at,
            failure_reason=reason,
            outputs={k.value: v for k, v in self._outputs.items()},
        )
        temporary = self.snapshot_path.with_suffix(self.snapshot_path.suffix + ".tmp")
        temporary.write_text(
            json.dumps(result.to_dict(), ensure_ascii=False, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        os.replace(temporary, self.snapshot_path)
        return result

    def run(self) -> PipelineResult:
        if self._has_run:
            raise RuntimeError("R11Orchestrator is one-shot; create a new run_id for another run")
        self._has_run = True
        self._started_at = time.time()
        self.status = PipelineStatus.RUNNING
        self._append_event(
            "pipeline_transition",
            from_status=PipelineStatus.PENDING.value,
            to_status=PipelineStatus.RUNNING.value,
        )

        for stage in STAGE_ORDER:
            if not self._run_stage(stage):
                self._block_after(stage)
                stage_status = self.stage_statuses[stage]
                if stage_status == WorkerStatus.CANCELLED:
                    return self._finalize(PipelineStatus.CANCELLED, self.token.reason)
                last_error = next(
                    (
                        attempt.error
                        for attempt in reversed(self._attempts)
                        if attempt.stage == stage.value and attempt.error
                    ),
                    f"{stage.value} failed",
                )
                return self._finalize(PipelineStatus.FAILED, last_error)

        recovered = any(a.status == AttemptStatus.FAILED.value for a in self._attempts)
        final_status = (
            PipelineStatus.SUCCEEDED_WITH_RETRIES
            if recovered
            else PipelineStatus.SUCCEEDED
        )
        return self._finalize(final_status, None)


def make_real_workers(
    workspace: Path | str,
    *,
    measurement_fn: Optional[Callable[[], Any]] = None,
    dashboard_fn: Optional[Callable[[Any], Any]] = None,
    gate_runner: Optional[Callable[[Path], Mapping[str, Any]]] = None,
) -> Dict[Stage, Worker]:
    """Compose the existing V1136 measurement, dashboard renderer, and R11 QA gate."""

    workspace_path = Path(workspace).resolve()
    if measurement_fn is None:
        from apeireth.v1136_asi_v05_3dim_real_measurement import measure_v05_3dims

        measurement_fn = measure_v05_3dims
    if dashboard_fn is None:
        from apeireth.v1136_dashboard_render import render_v1136_dashboard

        dashboard_fn = render_v1136_dashboard
    if gate_runner is None:
        from apeireth.r11_requirements_gate import run_all_gates

        gate_runner = run_all_gates

    def measurement_worker(_: WorkerContext) -> WorkerOutcome:
        measurement = measurement_fn()
        evidence = _jsonable(measurement)
        guards_pass = bool(getattr(measurement, "v3_guards_pass", False))
        if not guards_pass:
            return WorkerOutcome.failure(
                "V1136 measurement v3_guards_pass is False",
                value=measurement,
                evidence=evidence,
                retryable=False,
            )
        return WorkerOutcome.success(measurement, evidence)

    def dashboard_worker(context: WorkerContext) -> WorkerOutcome:
        if context.measurement is None:
            return WorkerOutcome.failure(
                "dashboard received no successful V1136 measurement",
                retryable=False,
            )
        dashboard = dashboard_fn(context.measurement)
        evidence = _jsonable(dashboard)
        measured_score = float(getattr(context.measurement, "v05_total_v1136"))
        rendered_score = float(getattr(dashboard, "v1136_score"))
        render_path = getattr(dashboard, "render_path", None)
        if rendered_score != measured_score or render_path != "v1136_real":
            return WorkerOutcome.failure(
                "dashboard did not consume the exact V1136 real measurement",
                value=dashboard,
                evidence={
                    "measurement_score": measured_score,
                    "dashboard_score": rendered_score,
                    "render_path": render_path,
                    "dashboard": evidence,
                },
                retryable=False,
            )
        if not bool(getattr(dashboard, "v3_guards_pass", False)):
            return WorkerOutcome.failure(
                "dashboard reports v3_guards_pass=False",
                value=dashboard,
                evidence=evidence,
                retryable=False,
            )
        return WorkerOutcome.success(dashboard, evidence)

    def qa_worker(context: WorkerContext) -> WorkerOutcome:
        if context.measurement is None or context.dashboard is None:
            return WorkerOutcome.failure(
                "QA gate requires successful measurement and dashboard outputs",
                retryable=False,
            )
        results = gate_runner(workspace_path)
        evidence = {
            name: _jsonable(result)
            for name, result in results.items()
        }
        failed = [
            name
            for name, result in results.items()
            if not bool(getattr(result, "passed", False))
        ]
        if failed:
            return WorkerOutcome.failure(
                f"QA gates failed: {failed}",
                value=results,
                evidence={"all_passed": False, "failed_gates": failed, "results": evidence},
                retryable=False,
            )
        return WorkerOutcome.success(
            results,
            {"all_passed": True, "failed_gates": [], "results": evidence},
        )

    return {
        Stage.MEASUREMENT: measurement_worker,
        Stage.DASHBOARD: dashboard_worker,
        Stage.QA_GATE: qa_worker,
    }


def run_real_pipeline(
    workspace: Path | str,
    *,
    evidence_dir: Optional[Path | str] = None,
    max_attempts: int = 2,
    cancellation_token: Optional[CancellationToken] = None,
) -> PipelineResult:
    workspace_path = Path(workspace).resolve()
    target = Path(evidence_dir).resolve() if evidence_dir else workspace_path / "reports" / "r11-orchestration-evidence"
    orchestrator = R11Orchestrator(
        make_real_workers(workspace_path),
        evidence_dir=target,
        max_attempts=max_attempts,
        cancellation_token=cancellation_token,
    )
    return orchestrator.run()


def _cli(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="R11 V1136 -> dashboard -> QA orchestration")
    parser.add_argument("--workspace", default=str(Path(__file__).resolve().parent.parent))
    parser.add_argument("--evidence-dir", default=None)
    parser.add_argument("--max-attempts", type=int, default=2)
    args = parser.parse_args(argv)
    result = run_real_pipeline(
        args.workspace,
        evidence_dir=args.evidence_dir,
        max_attempts=args.max_attempts,
    )
    print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2))
    return 0 if result.succeeded else 1


__all__ = [
    "SCHEMA_VERSION",
    "Stage",
    "STAGE_ORDER",
    "WorkerStatus",
    "AttemptStatus",
    "PipelineStatus",
    "InvalidTransition",
    "EvidenceCorruptionError",
    "WorkerOutcome",
    "WorkerContext",
    "AttemptRecord",
    "PipelineResult",
    "CancellationToken",
    "verify_evidence",
    "R11Orchestrator",
    "make_real_workers",
    "run_real_pipeline",
]


if __name__ == "__main__":
    raise SystemExit(_cli())
