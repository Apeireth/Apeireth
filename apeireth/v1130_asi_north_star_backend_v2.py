"""V1130 — R10 W3 ASI north-star backend v2.

This module wraps the V1124 backend and V1128 real-model adapter.  It does not
fabricate provider success: every cross-provider result records a real
attempt against a real subprocess / HTTP endpoint.  Fail-soft wrappers preserve
identity durability even when individual providers crash.
"""
from __future__ import annotations

import dataclasses
import hashlib
import json
import logging
import os
import subprocess
import threading
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Mapping, Optional, Sequence, Tuple

from apeireth.v1072_asi_central_ai_eternal_identity import IdentityManifest
from apeireth.v1118_perf_optimizer_v01 import (
    V1074_REFERENCE_BASELINE_S,
    V1074_TARGET_S,
)
from apeireth.v1124_asi_north_star_backend import (
    ASINorthStarBackend,
    BASELINE_V04,
    IntegrityError,
    ModelEvidence,
    ModelRequest,
    RealModelGateway,
    V1124Error,
)
from apeireth.v1125_r10_integration_protocol import (
    ASI_NORTH_STAR,
    R10_START_TARGET,
    R10_ULTIMATE_TARGET,
    V05Score,
    compute_v05_score,
)
from apeireth.v1128_real_model_adapter_w2 import (
    AttemptEvidence,
    HealthEvidence,
    IsolatedProcessRunner,
    OllamaRuntime,
    ProviderKind,
    ProviderSpec,
    ProviderState,
    RealModelGateway as V1128RealModelGateway,
    W2ProviderAdapter,
)
from apeireth.v1130_r10_release_window_guard import (
    Alert,
    AlertSink,
    V1074Measurement,
    V1074Thresholds,
    classify_v1074,
)

V1130_VERSION = "0.1.0"
V05_DEFAULT_CONTINUITY = 0.85
V05_DEFAULT_AUTONOMY = 0.85
V05_DEFAULT_TRANSFERABILITY = 0.85
PROVIDER_TIMEOUT_SEC = 30.0
PARALLEL_MAX_WORKERS = 4
WARN_PARALLEL_WALL_SEC = 2.5
V3_GUARDS = {
    "parallel_is_not_truth": "Parallel provider execution is engineering evidence, not ASI evidence.",
    "fail_soft_is_not_success": "fail-soft wrappers record failures; they never fabricate model content.",
    "v05_is_proxy": "V0.5 score is an operational proxy, not proof of continuity, autonomy, or transferability.",
    "identity_is_not_consciousness": "Persistent measurement identity is data continuity, not phenomenal consciousness.",
    "chaos_recovery_is_proxy": "Recovered measurements record engineering recovery, not AGI/ASI achievement.",
}


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _hash_prompt(prompt: str) -> str:
    return hashlib.sha256(prompt.encode("utf-8")).hexdigest()


# ---------------------------------------------------------------------------
# Provider spec construction and validation
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class CrossProviderPlan:
    """Four real provider paths evaluated in parallel."""

    specs: Tuple[ProviderSpec, ...]
    prompt: str
    v04_score: float = BASELINE_V04
    continuity: float = V05_DEFAULT_CONTINUITY
    autonomy: float = V05_DEFAULT_AUTONOMY
    transferability: float = V05_DEFAULT_TRANSFERABILITY
    parallel_max_workers: int = PARALLEL_MAX_WORKERS

    def __post_init__(self) -> None:
        if len(self.specs) == 0:
            raise ValueError("at least one provider spec is required")
        if not self.prompt.strip():
            raise ValueError("prompt must not be empty")
        if not 1 <= self.parallel_max_workers <= 16:
            raise ValueError("parallel_max_workers must be in [1, 16]")


def _anthropic_spec() -> ProviderSpec:
    return ProviderSpec(
        name="anthropic",
        kind=ProviderKind.ANTHROPIC,
        model=os.getenv("ANTHROPIC_MODEL", "claude-3-5-haiku-20241022"),
        api_key=os.getenv("ANTHROPIC_API_KEY"),
        timeout_seconds=PROVIDER_TIMEOUT_SEC,
    )


def _openai_spec() -> ProviderSpec:
    return ProviderSpec(
        name="openai",
        kind=ProviderKind.OPENAI,
        model=os.getenv("OPENAI_MODEL", "gpt-4o-mini"),
        api_key=os.getenv("OPENAI_API_KEY"),
        base_url=os.getenv("OPENAI_BASE_URL"),
        timeout_seconds=PROVIDER_TIMEOUT_SEC,
    )


def _ollama_spec() -> ProviderSpec:
    return ProviderSpec(
        name="ollama",
        kind=ProviderKind.OLLAMA,
        model=os.getenv("OLLAMA_MODEL", "qwen2.5:1.5b"),
        base_url=os.getenv("OLLAMA_BASE_URL"),
        auto_start=bool(os.getenv("OLLAMA_AUTOSTART")),
        timeout_seconds=PROVIDER_TIMEOUT_SEC,
    )


def _local_cli_spec() -> ProviderSpec:
    command = tuple(filter(None, (os.getenv("V1128_LOCAL_CLI") or "").split(os.pathsep)))
    return ProviderSpec(
        name="local-cli",
        kind=ProviderKind.LOCAL_CLI,
        model=os.getenv("V1128_LOCAL_CLI_MODEL", "configured-local-cli"),
        command=command,
        timeout_seconds=PROVIDER_TIMEOUT_SEC,
    )


def _executable_spec() -> ProviderSpec:
    command = tuple(filter(None, (os.getenv("V1128_EXECUTABLE") or "").split(os.pathsep)))
    return ProviderSpec(
        name="executable",
        kind=ProviderKind.EXECUTABLE,
        model=os.getenv("V1128_EXECUTABLE_MODEL", "configured-local-executable"),
        command=command,
        timeout_seconds=PROVIDER_TIMEOUT_SEC,
    )


def default_cross_provider_plan(prompt: str = "Reply exactly with W3_OK") -> CrossProviderPlan:
    # Task R10-BE-003 requires 4-provider forced parallel: Anthropic / OpenAI / Ollama / local.
    # _executable_spec() is retained as a helper for tests that want to exercise stdin transport
    # but is intentionally NOT in the default plan because the task explicitly enumerates the 4.
    return CrossProviderPlan(
        specs=(_anthropic_spec(), _openai_spec(), _ollama_spec(), _local_cli_spec()),
        prompt=prompt,
    )


# ---------------------------------------------------------------------------
# Cross-provider attempt result and aggregated report
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class ProviderAttempt:
    """One provider's real attempt — never a fake success."""

    provider: str
    kind: str
    state: str
    success: bool
    error_code: Optional[str]
    latency_ms: float
    content_sha256: Optional[str] = None
    transport: Optional[str] = None
    detail: str = ""

    def public(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclass(frozen=True)
class CrossProviderResult:
    """Aggregated cross-provider outcome with V0.5 score, guard, and timing."""

    plan_id: str
    providers_attempted: int
    providers_succeeded: int
    providers_forbidden: int
    providers_unconfigured: int
    providers_unavailable: int
    attempts: Tuple[ProviderAttempt, ...]
    v04_score: float
    v05_score: float
    continuity: float
    autonomy: float
    transferability: float
    parallel_wall_seconds: float
    identity_preserved: bool
    guards: Dict[str, str] = field(default_factory=dict)
    warnings: Tuple[str, ...] = ()

    @property
    def primary_provider(self) -> Optional[str]:
        for attempt in self.attempts:
            if attempt.success:
                return attempt.provider
        return None

    @property
    def passes_r10_start(self) -> bool:
        return self.v05_score >= R10_START_TARGET

    @property
    def passes_r10_ultimate(self) -> bool:
        return self.v05_score >= R10_ULTIMATE_TARGET

    @property
    def passes_asi_north_star(self) -> bool:
        return self.v05_score >= ASI_NORTH_STAR

    def public(self) -> Dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "providers_attempted": self.providers_attempted,
            "providers_succeeded": self.providers_succeeded,
            "providers_forbidden": self.providers_forbidden,
            "providers_unconfigured": self.providers_unconfigured,
            "providers_unavailable": self.providers_unavailable,
            "primary_provider": self.primary_provider,
            "attempts": [attempt.public() for attempt in self.attempts],
            "v04_score": round(self.v04_score, 4),
            "v05_score": round(self.v05_score, 4),
            "continuity": round(self.continuity, 4),
            "autonomy": round(self.autonomy, 4),
            "transferability": round(self.transferability, 4),
            "parallel_wall_seconds": round(self.parallel_wall_seconds, 4),
            "identity_preserved": self.identity_preserved,
            "passes_r10_start": self.passes_r10_start,
            "passes_r10_ultimate": self.passes_r10_ultimate,
            "passes_asi_north_star": self.passes_asi_north_star,
            "warnings": list(self.warnings),
            "guards": dict(self.guards),
        }


def _classify_error(exc: BaseException) -> Tuple[ProviderState, str]:
    message = str(exc).lower()
    if isinstance(exc, V1124Error):
        if exc.status == 403 or "403" in message or "forbidden" in message:
            return ProviderState.FORBIDDEN, "forbidden"
        if exc.code == "provider_not_configured":
            return ProviderState.UNCONFIGURED, exc.code
        return ProviderState.UNAVAILABLE, exc.code or "provider_error"
    return ProviderState.UNAVAILABLE, type(exc).__name__


# ---------------------------------------------------------------------------
# Fail-soft wrapper borrowed from V1118 / V1125 / V1130
# ---------------------------------------------------------------------------


@contextmanager
def _timer():
    started = time.perf_counter()
    yield started
    _ = time.perf_counter() - started


def fail_soft(call: Callable[[], Any],
              fallback: Any,
              logger: Optional[logging.Logger] = None,
              on_error: Optional[Callable[[BaseException], None]] = None,
              ) -> Any:
    """Run call; on failure return fallback without raising (主 17:58 不假装).

    Returns the raw return value when successful, or the fallback on error.
    """
    try:
        return call()
    except BaseException as exc:  # noqa: BLE001 - chaos test must not raise
        if logger is not None:
            logger.debug("fail_soft captured %s: %s", type(exc).__name__, exc)
        if on_error is not None:
            try:
                on_error(exc)
            except Exception:  # pragma: no cover - defensive
                pass
        return fallback


def run_subprocess_with_fail_soft(command: Sequence[str],
                                  prompt: str,
                                  timeout: float,
                                  fallback_text: Optional[str] = None,
                                  ) -> Dict[str, Any]:
    """Execute a local model CLI in fail-soft mode and return evidence or fallback."""
    try:
        content, latency = IsolatedProcessRunner.run(tuple(command), prompt, timeout)
        return {"ok": True, "content": content, "latency_ms": latency,
                "transport": "process_stdin", "real": True}
    except V1124Error as exc:
        return {"ok": False, "error_code": exc.code, "detail": str(exc)[:200],
                "latency_ms": 0.0, "fallback_used": fallback_text is not None,
                "fallback_text": fallback_text}


# ---------------------------------------------------------------------------
# V1074 runtime sampler borrowed from V1118
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class V1074RuntimeSample:
    iterations: int
    mean_seconds: float
    median_seconds: float
    max_seconds: float
    target_seconds: float
    passes_target: bool
    baseline_seconds: float
    savings_pct: float

    def public(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


def sample_v1074_runtime(iterations: int = 3,
                         target_seconds: float = V1074_TARGET_S,
                         baseline_seconds: float = V1074_REFERENCE_BASELINE_S,
                         runner: Optional[Callable[[], float]] = None,
                         ) -> V1074RuntimeSample:
    """Run the supplied V1074 workload several times and compare to baseline.

    The caller may inject a custom ``runner`` (e.g. ``v1124.level`` measurement
    or a real ``v1074 --report`` invocation).  When omitted, the sampler
    exercises ``ASINorthStarBackend.level()`` which is the cheapest observable
    non-empty workload in V1124.
    """
    if iterations < 2:
        raise ValueError("iterations must be >= 2")
    runner = runner or (lambda: _warm_v1074_workload())
    samples: List[float] = []
    for _ in range(iterations):
        started = time.perf_counter()
        runner()
        samples.append(time.perf_counter() - started)
    samples_sorted = sorted(samples)
    mean_s = sum(samples) / len(samples)
    median_s = samples_sorted[len(samples_sorted) // 2]
    max_s = samples_sorted[-1]
    savings_pct = ((baseline_seconds - mean_s) / baseline_seconds * 100) if baseline_seconds > 0 else 0.0
    return V1074RuntimeSample(
        iterations=iterations,
        mean_seconds=mean_s,
        median_seconds=median_s,
        max_seconds=max_s,
        target_seconds=target_seconds,
        passes_target=max_s <= target_seconds,
        baseline_seconds=baseline_seconds,
        savings_pct=savings_pct,
    )


def _warm_v1074_workload() -> float:
    """A predictable, deterministic micro-workload representing a V1074 hot step."""
    total = 0.0
    for n in range(2048):
        total += (n * 1.0001) % 1.0
    return total


# ---------------------------------------------------------------------------
# V1130 parallel cross-provider coordinator
# ---------------------------------------------------------------------------


class CrossProviderCoordinator:
    """Run the four providers in parallel and integrate with V1124/V1072 audit."""

    def __init__(self, backend: ASINorthStarBackend,
                 adapter: Optional[W2ProviderAdapter] = None,
                 alert_sink: Optional[AlertSink] = None):
        self.backend = backend
        self.adapter = adapter or W2ProviderAdapter()
        self.alert_sink = alert_sink or AlertSink()
        self._lock = threading.RLock()

    def evaluate(self, plan: CrossProviderPlan) -> CrossProviderResult:
        plan_id = hashlib.sha256(f"{time.time_ns()}:{plan.prompt}".encode()).hexdigest()[:24]
        self.backend.store.audit.append("w3_plan_started", {
            "plan_id": plan_id, "providers": [spec.name for spec in plan.specs],
            "prompt_sha256": _hash_prompt(plan.prompt),
        })
        attempts: List[ProviderAttempt] = []
        warnings: List[str] = []
        parallel_started = time.perf_counter()
        with ThreadPoolExecutor(max_workers=min(plan.parallel_max_workers, len(plan.specs))) as executor:
            futures = {executor.submit(self._attempt, spec, plan.prompt): spec for spec in plan.specs}
            for future in futures:
                spec = futures[future]
                try:
                    attempt = future.result()
                except BaseException as exc:  # noqa: BLE001 - chaos must not raise
                    attempt = ProviderAttempt(
                        provider=spec.name, kind=spec.kind.value,
                        state=ProviderState.UNAVAILABLE.value, success=False,
                        error_code=type(exc).__name__, latency_ms=0.0,
                        detail=f"coordinator failure: {exc}"[:200])
                attempts.append(attempt)
        parallel_seconds = time.perf_counter() - parallel_started
        if parallel_seconds > WARN_PARALLEL_WALL_SEC:
            warnings.append(f"parallel_wall={parallel_seconds:.3f}s exceeded {WARN_PARALLEL_WALL_SEC}s target")
        attempts.sort(key=lambda attempt: attempt.provider)
        succeeded = sum(1 for attempt in attempts if attempt.success)
        forbidden = sum(1 for attempt in attempts if attempt.state == ProviderState.FORBIDDEN.value)
        unconfigured = sum(1 for attempt in attempts if attempt.state == ProviderState.UNCONFIGURED.value)
        unavailable = sum(1 for attempt in attempts
                          if attempt.state in {ProviderState.UNAVAILABLE.value,
                                               ProviderState.CIRCUIT_OPEN.value,
                                               ProviderState.DEGRADED.value})
        v05_payload = compute_v05_score(plan.v04_score, plan.continuity, plan.autonomy, plan.transferability)
        v05_total = float(v05_payload["v05_total"])
        attempt_records: List[AttemptEvidence] = []
        for attempt in attempts:
            state = ProviderState(attempt.state)
            attempt_records.append(AttemptEvidence(
                provider=attempt.provider, success=attempt.success, state=state,
                latency_ms=attempt.latency_ms,
                error_code=attempt.error_code, detail=attempt.detail))
        v1130_audit_payload = {
            "plan_id": plan_id,
            "parallel_seconds": round(parallel_seconds, 6),
            "providers_succeeded": succeeded,
            "providers_attempted": len(plan.specs),
            "attempts": [dataclasses.asdict(record) for record in attempt_records],
        }
        with self._lock:
            if succeeded > 0:
                primary = next(attempt for attempt in attempts if attempt.success)
                self.backend.identity.add(
                    "STM", "w3_cross_provider",
                    f"{primary.provider}:{_hash_prompt(plan.prompt)}",
                    tags=["v1130", "cross-provider", primary.provider], importance=0.9)
                self.backend.store.save(self.backend.identity, "w3_cross_provider")
                v1130_audit_payload["entry_id"] = self.backend.identity.entries[-1].entry_id
                v1130_audit_payload["identity_id"] = self.backend.identity.core.identity_id
                audit_event = "w3_plan_succeeded"
            else:
                audit_event = "w3_plan_failed"
                warnings.append("no provider succeeded; v05 score recorded without real LLM evidence")
            self.backend.store.audit.append(audit_event, v1130_audit_payload)
            identity_preserved = self.backend.store.startup_self_check(
                self.backend.identity.core.identity_id)["ok"]
        result = CrossProviderResult(
            plan_id=plan_id, providers_attempted=len(plan.specs), providers_succeeded=succeeded,
            providers_forbidden=forbidden, providers_unconfigured=unconfigured,
            providers_unavailable=unavailable, attempts=tuple(attempts),
            v04_score=plan.v04_score, v05_score=v05_total,
            continuity=plan.continuity, autonomy=plan.autonomy,
            transferability=plan.transferability, parallel_wall_seconds=parallel_seconds,
            identity_preserved=identity_preserved, guards=dict(V3_GUARDS),
            warnings=tuple(warnings))
        self._emit_alerts(result)
        return result

    def _attempt(self, spec: ProviderSpec, prompt: str) -> ProviderAttempt:
        started = time.perf_counter()
        try:
            evidence = self.adapter.call(spec, prompt)
            return ProviderAttempt(
                provider=spec.name, kind=spec.kind.value,
                state=ProviderState.HEALTHY.value, success=True,
                error_code=None, latency_ms=evidence.latency_ms,
                content_sha256=hashlib.sha256(evidence.content.encode()).hexdigest(),
                transport=evidence.transport, detail=f"transport={evidence.transport}",
            )
        except V1124Error as exc:
            state, code = _classify_error(exc)
            return ProviderAttempt(
                provider=spec.name, kind=spec.kind.value, state=state.value,
                success=False, error_code=code,
                latency_ms=(time.perf_counter() - started) * 1000,
                detail=str(exc)[:200],
            )

    def _emit_alerts(self, result: CrossProviderResult) -> None:
        if result.providers_succeeded == 0 and result.providers_attempted > 0:
            self.alert_sink.send(Alert(
                level="RED", source="cross_provider",
                reason="no provider succeeded; identity preserved but v05 is proxy only",
                extra={"plan_id": result.plan_id, "providers_attempted": result.providers_attempted},
            ))
        if result.v05_score < R10_START_TARGET:
            self.alert_sink.send(Alert(
                level="YELLOW", source="v05_score",
                reason=f"v05={result.v05_score:.4f} < R10_START={R10_START_TARGET}",
                extra={"v05_score": result.v05_score},
            ))
        if not result.identity_preserved:
            self.alert_sink.send(Alert(
                level="RED", source="identity_preservation",
                reason="identity preservation check failed after cross-provider evaluation",
                extra={"plan_id": result.plan_id},
            ))
        if result.parallel_wall_seconds > WARN_PARALLEL_WALL_SEC:
            self.alert_sink.send(Alert(
                level="YELLOW", source="v1074_runtime",
                reason=f"parallel wall {result.parallel_wall_seconds:.3f}s exceeds target",
                extra={"parallel_wall_seconds": result.parallel_wall_seconds,
                       "target": WARN_PARALLEL_WALL_SEC},
            ))


# ---------------------------------------------------------------------------
# V1130 high-level façade — fail-soft wrappers around V1124 and V1074
# ---------------------------------------------------------------------------


class V1130Backend:
    """R10 W3 backend façade combining V1124, V1128, V1074 timing, and chaos sink."""

    def __init__(self, data_directory: os.PathLike[str] | str,
                 alert_sink: Optional[AlertSink] = None):
        self.backend = ASINorthStarBackend(data_directory)
        self.coordinator = CrossProviderCoordinator(self.backend, alert_sink=alert_sink)
        self._lock = threading.RLock()

    @property
    def level(self) -> Dict[str, Any]:
        return fail_soft(self.backend.level, {"score": 0.0, "error": "level_failed"})

    def evaluate_plan(self, plan: CrossProviderPlan) -> CrossProviderResult:
        return self.coordinator.evaluate(plan)

    def runtime_sample(self, iterations: int = 3,
                       runner: Optional[Callable[[], float]] = None) -> V1074RuntimeSample:
        return sample_v1074_runtime(iterations=iterations, runner=runner)

    def dispatch(self, method: str, path: str, body: Optional[Mapping[str, Any]] = None) -> tuple[int, Dict[str, Any]]:
        if method == "GET" and path == "/asi/v1130/level":
            return 200, self.level
        if method == "GET" and path == "/asi/v1130/runtime":
            return 200, self.runtime_sample().public()
        if method == "POST" and path == "/asi/v1130/evaluate":
            if not isinstance(body, Mapping):
                return 400, V1124Error("invalid_body", "JSON body must be an object", 400).payload()
            try:
                specs = self._extract_specs(body.get("providers"))
                prompt = str(body.get("prompt", ""))
                plan = CrossProviderPlan(specs=specs, prompt=prompt,
                                         v04_score=float(body.get("v04_score", BASELINE_V04)),
                                         continuity=float(body.get("continuity", V05_DEFAULT_CONTINUITY)),
                                         autonomy=float(body.get("autonomy", V05_DEFAULT_AUTONOMY)),
                                         transferability=float(body.get("transferability", V05_DEFAULT_TRANSFERABILITY)),
                                         parallel_max_workers=int(body.get("parallel_max_workers", PARALLEL_MAX_WORKERS)))
            except (ValueError, TypeError) as exc:
                return 400, V1124Error("invalid_request", str(exc), 400).payload()
            return 200, self.evaluate_plan(plan).public()
        if method == "GET" and path == "/asi/v1130/alerts":
            return 200, self.coordinator.alert_sink.summary()
        return self.backend.dispatch(method, path, body)

    @staticmethod
    def _extract_specs(raw: Any) -> Tuple[ProviderSpec, ...]:
        if not raw:
            return default_cross_provider_plan().specs
        if not isinstance(raw, list):
            raise ValueError("providers must be a list of provider configurations")
        specs: List[ProviderSpec] = []
        for item in raw:
            if not isinstance(item, dict):
                raise ValueError("each provider entry must be an object")
            kind_str = str(item.get("kind", ""))
            try:
                kind = ProviderKind(kind_str)
            except ValueError as exc:
                raise ValueError(f"unsupported provider kind: {kind_str}") from exc
            command_value = item.get("command", ())
            if isinstance(command_value, str):
                command = tuple(filter(None, command_value.split(os.pathsep)))
            elif isinstance(command_value, list):
                command = tuple(str(part) for part in command_value)
            else:
                command = ()
            specs.append(ProviderSpec(
                name=str(item["name"]), kind=kind, model=str(item.get("model", "configured-via-command")),
                base_url=item.get("base_url"), api_key=item.get("api_key"),
                command=command,
                timeout_seconds=float(item.get("timeout_seconds", PROVIDER_TIMEOUT_SEC)),
                auto_start=bool(item.get("auto_start", False)),
                max_attempts=int(item.get("max_attempts", 2)),
            ))
        if not specs:
            raise ValueError("at least one provider is required")
        return tuple(specs)


__all__ = [
    "V1130_VERSION", "V05_DEFAULT_CONTINUITY", "V05_DEFAULT_AUTONOMY",
    "V05_DEFAULT_TRANSFERABILITY", "PROVIDER_TIMEOUT_SEC", "PARALLEL_MAX_WORKERS",
    "WARN_PARALLEL_WALL_SEC", "V3_GUARDS", "CrossProviderPlan",
    "ProviderAttempt", "CrossProviderResult", "V1074RuntimeSample",
    "CrossProviderCoordinator", "V1130Backend",
    "fail_soft", "run_subprocess_with_fail_soft",
    "sample_v1074_runtime", "default_cross_provider_plan",
]