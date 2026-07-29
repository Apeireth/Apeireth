"""V1128 W2 real-model adapter and honest provider fallback.

The adapter performs real network/process I/O.  It never substitutes canned text for a
failed model.  A successful transport is integration evidence, not proof of ASI or of
phenomenal consciousness (主 17:43 实事求是 + 主 17:58 不假装).
"""
from __future__ import annotations

import hashlib
import json
import os
import shlex
import shutil
import subprocess
import threading
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, Iterable, Mapping, Optional, Sequence

from apeireth.v1106_engineering_lift import CircuitBreaker, exponential_backoff
from apeireth.v1124_asi_north_star_backend import (
    ASINorthStarBackend,
    ModelEvidence,
    ModelRequest,
    RealModelGateway,
    V1124Error,
)

V1128_VERSION = "0.1.0"
W2_TARGET = 0.90
DEFAULT_OLLAMA_MODELS = ("qwen2.5:1.5b", "llama3.2:3b")
OUTPUT_LIMIT_BYTES = 1_048_576
V3_GUARDS = {
    "unavailable_is_not_success": "Unavailable, forbidden, and unconfigured providers remain failures.",
    "transport_is_not_intelligence": "A real response proves transport execution, not intelligence or ASI.",
    "comparison_is_not_truth": "Cross-provider ASI levels are operational proxies, not ground truth.",
    "fallback_is_not_model_output": "Fallback selects another real provider; it never fabricates content.",
    "identity_is_not_consciousness": "Persistent measurement identity is not phenomenal consciousness.",
}


class ProviderState(str, Enum):
    HEALTHY = "healthy"
    CONFIGURED = "configured_unverified"
    UNCONFIGURED = "unconfigured"
    UNAVAILABLE = "unavailable"
    FORBIDDEN = "forbidden"
    DEGRADED = "degraded"
    CIRCUIT_OPEN = "circuit_open"


class ProviderKind(str, Enum):
    ANTHROPIC = "anthropic"
    OPENAI = "openai"
    OLLAMA = "ollama"
    LOCAL_CLI = "local_cli"
    EXECUTABLE = "executable"


@dataclass(frozen=True)
class ProviderSpec:
    """A concrete provider configuration; secrets are never serialized."""

    name: str
    kind: ProviderKind
    model: str
    base_url: Optional[str] = None
    api_key: Optional[str] = None
    command: Sequence[str] = field(default_factory=tuple)
    timeout_seconds: float = 30.0
    auto_start: bool = False
    max_attempts: int = 2

    def validate(self) -> None:
        if not self.name.strip():
            raise ValueError("provider name must not be empty")
        if not self.model.strip():
            raise ValueError("model must not be empty")
        if not 0 < self.timeout_seconds <= 120:
            raise ValueError("timeout_seconds must be in (0, 120]")
        if not 1 <= self.max_attempts <= 5:
            raise ValueError("max_attempts must be in [1, 5]")
        if self.command and not all(isinstance(item, str) and item for item in self.command):
            raise ValueError("command must contain non-empty strings")

    def public(self) -> Dict[str, Any]:
        return {"name": self.name, "kind": self.kind.value, "model": self.model,
                "base_url": self.base_url, "command_configured": bool(self.command),
                "timeout_seconds": self.timeout_seconds, "auto_start": self.auto_start,
                "max_attempts": self.max_attempts, "credential_configured": bool(self.api_key)}


@dataclass(frozen=True)
class HealthEvidence:
    provider: str
    state: ProviderState
    latency_ms: float
    detail: str
    checked_at_ns: int = field(default_factory=time.time_ns)
    models: Sequence[str] = field(default_factory=tuple)

    @property
    def healthy(self) -> bool:
        return self.state == ProviderState.HEALTHY

    def public(self) -> Dict[str, Any]:
        data = asdict(self)
        data["state"] = self.state.value
        data["models"] = list(self.models)
        data["healthy"] = self.healthy
        return data


@dataclass(frozen=True)
class AttemptEvidence:
    provider: str
    success: bool
    state: ProviderState
    latency_ms: float
    error_code: Optional[str] = None
    detail: str = ""

    def public(self) -> Dict[str, Any]:
        data = asdict(self)
        data["state"] = self.state.value
        return data


@dataclass(frozen=True)
class RoutedEvidence:
    selected_provider: Optional[str]
    evidence: Optional[ModelEvidence]
    attempts: Sequence[AttemptEvidence]
    fallback_used: bool

    @property
    def success(self) -> bool:
        return self.evidence is not None and self.evidence.real

    def public(self) -> Dict[str, Any]:
        return {"success": self.success, "selected_provider": self.selected_provider,
                "fallback_used": self.fallback_used,
                "evidence": self.evidence.public() if self.evidence else None,
                "attempts": [attempt.public() for attempt in self.attempts]}


class ProviderUnavailable(V1124Error):
    def __init__(self, code: str, message: str, status: int = 503):
        super().__init__(code, message, status)


class OllamaRuntime:
    """Probe a real Ollama endpoint and optionally launch the installed daemon."""

    def __init__(self, executable: Optional[str] = None):
        self.executable = executable or shutil.which("ollama")
        self._process: Optional[subprocess.Popen[str]] = None
        self._lock = threading.Lock()

    @staticmethod
    def base_url(spec: ProviderSpec) -> str:
        return (spec.base_url or os.getenv("OLLAMA_BASE_URL") or "http://127.0.0.1:11434").rstrip("/")

    def probe(self, spec: ProviderSpec, timeout: float = 2.0) -> HealthEvidence:
        started = time.perf_counter()
        url = self.base_url(spec) + "/api/tags"
        try:
            with urllib.request.urlopen(url, timeout=timeout) as response:
                raw = response.read(OUTPUT_LIMIT_BYTES + 1)
                if len(raw) > OUTPUT_LIMIT_BYTES:
                    raise ValueError("Ollama tags response exceeds limit")
                data = json.loads(raw)
            models = tuple(item.get("name", "") for item in data.get("models", []) if item.get("name"))
            return HealthEvidence(spec.name, ProviderState.HEALTHY,
                                  (time.perf_counter() - started) * 1000,
                                  f"real GET {url} returned {len(models)} models", models=models)
        except (urllib.error.URLError, TimeoutError, OSError, ValueError, json.JSONDecodeError) as exc:
            return HealthEvidence(spec.name, ProviderState.UNAVAILABLE,
                                  (time.perf_counter() - started) * 1000,
                                  f"real GET {url} failed: {type(exc).__name__}: {str(exc)[:160]}")

    def ensure_running(self, spec: ProviderSpec, startup_timeout: float = 8.0) -> HealthEvidence:
        current = self.probe(spec)
        if current.healthy or not spec.auto_start:
            return current
        url = self.base_url(spec)
        if not url.startswith(("http://127.0.0.1", "http://localhost")):
            return HealthEvidence(spec.name, ProviderState.UNAVAILABLE, current.latency_ms,
                                  "remote Ollama endpoints are never auto-started")
        if not self.executable:
            return HealthEvidence(spec.name, ProviderState.UNCONFIGURED, current.latency_ms,
                                  "ollama executable not found; install Ollama or configure a remote URL")
        with self._lock:
            if self._process is None or self._process.poll() is not None:
                flags: Dict[str, Any] = {"stdin": subprocess.DEVNULL, "stdout": subprocess.DEVNULL,
                                         "stderr": subprocess.DEVNULL, "text": True}
                if os.name == "nt":
                    flags["creationflags"] = getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
                else:
                    flags["start_new_session"] = True
                try:
                    self._process = subprocess.Popen([self.executable, "serve"], **flags)
                except OSError as exc:
                    return HealthEvidence(spec.name, ProviderState.UNAVAILABLE, current.latency_ms,
                                          f"ollama serve failed: {exc}")
        deadline = time.monotonic() + startup_timeout
        while time.monotonic() < deadline:
            checked = self.probe(spec, timeout=min(1.0, max(0.1, deadline - time.monotonic())))
            if checked.healthy:
                return checked
            if self._process is not None and self._process.poll() is not None:
                break
            time.sleep(0.1)
        return HealthEvidence(spec.name, ProviderState.UNAVAILABLE, current.latency_ms,
                              "ollama serve did not become healthy before deadline")

    def stop_started_process(self) -> None:
        with self._lock:
            process, self._process = self._process, None
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)


class IsolatedProcessRunner:
    """Run a real local CLI in a separate process group with bounded output parsing."""

    @staticmethod
    def run(command: Sequence[str], prompt: str, timeout: float,
            prompt_as_argument: bool = False) -> tuple[str, float]:
        if not command:
            raise ProviderUnavailable("provider_not_configured", "local command is not configured")
        resolved = shutil.which(command[0]) if not Path(command[0]).exists() else command[0]
        if not resolved:
            raise ProviderUnavailable("provider_not_configured", f"executable not found: {command[0]}")
        argv = [resolved, *command[1:]]
        send_stdin: Optional[str] = prompt
        if prompt_as_argument:
            argv = [part.replace("{prompt}", prompt) for part in argv]
            if not any("{prompt}" in part for part in command):
                argv.append(prompt)
            send_stdin = None
        options: Dict[str, Any] = {"stdin": subprocess.PIPE if send_stdin is not None else subprocess.DEVNULL,
                                   "stdout": subprocess.PIPE, "stderr": subprocess.PIPE, "text": True}
        if os.name == "nt":
            options["creationflags"] = getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)
        else:
            options["start_new_session"] = True
        started = time.perf_counter()
        try:
            process = subprocess.Popen(argv, **options)
            stdout, stderr = process.communicate(send_stdin, timeout=timeout)
        except subprocess.TimeoutExpired as exc:
            process.kill()
            process.communicate()
            raise ProviderUnavailable("provider_timeout", f"local provider exceeded {timeout}s") from exc
        except OSError as exc:
            raise ProviderUnavailable("provider_unavailable", str(exc)) from exc
        latency = (time.perf_counter() - started) * 1000
        if process.returncode != 0:
            raise ProviderUnavailable("provider_process_error",
                                      f"local provider exited {process.returncode}: {stderr[-512:]}", 502)
        encoded = stdout.encode("utf-8")
        if len(encoded) > OUTPUT_LIMIT_BYTES:
            raise ProviderUnavailable("provider_response_too_large", "local stdout exceeds 1 MiB", 502)
        content = stdout.strip()
        if not content:
            raise ProviderUnavailable("provider_invalid_response", "local provider returned empty stdout", 502)
        return content, latency


class W2ProviderAdapter:
    """Real provider calls behind V1106 circuit breakers and bounded retry."""

    RETRYABLE_CODES = {"provider_unavailable", "provider_timeout", "provider_http_error",
                       "provider_process_error"}

    def __init__(self, gateway: Optional[RealModelGateway] = None,
                 ollama: Optional[OllamaRuntime] = None,
                 failure_threshold: int = 3, recovery_seconds: float = 5.0):
        self.gateway = gateway or RealModelGateway()
        self.ollama = ollama or OllamaRuntime()
        self.failure_threshold = failure_threshold
        self.recovery_seconds = recovery_seconds
        self._circuits: Dict[str, CircuitBreaker] = {}
        self._lock = threading.Lock()

    def circuit(self, name: str) -> CircuitBreaker:
        with self._lock:
            if name not in self._circuits:
                self._circuits[name] = CircuitBreaker(failure_threshold=self.failure_threshold,
                                                      timeout_seconds=self.recovery_seconds,
                                                      name=f"v1128-{name}")
            return self._circuits[name]

    @staticmethod
    def _credential(spec: ProviderSpec) -> Optional[str]:
        if spec.api_key:
            return spec.api_key
        if spec.kind == ProviderKind.ANTHROPIC:
            return os.getenv("ANTHROPIC_API_KEY")
        if spec.kind == ProviderKind.OPENAI:
            return os.getenv("OPENAI_API_KEY")
        return None

    def health(self, spec: ProviderSpec, deep: bool = False) -> HealthEvidence:
        spec.validate()
        started = time.perf_counter()
        if self.circuit(spec.name).state == "open":
            return HealthEvidence(spec.name, ProviderState.CIRCUIT_OPEN, 0.0, "V1106 circuit is open")
        if spec.kind == ProviderKind.OLLAMA:
            return self.ollama.ensure_running(spec) if spec.auto_start else self.ollama.probe(spec)
        if spec.kind == ProviderKind.ANTHROPIC:
            if not self._credential(spec):
                return HealthEvidence(spec.name, ProviderState.UNCONFIGURED, 0.0,
                                      "ANTHROPIC_API_KEY is absent; no request was fabricated")
            if not deep:
                return HealthEvidence(spec.name, ProviderState.CONFIGURED, 0.0,
                                      "credential exists but is not claimed valid until a real request succeeds")
        if spec.kind == ProviderKind.OPENAI:
            if not self._credential(spec):
                return HealthEvidence(spec.name, ProviderState.UNCONFIGURED, 0.0,
                                      "OPENAI_API_KEY is absent; no request was fabricated")
            if not deep:
                return HealthEvidence(spec.name, ProviderState.CONFIGURED, 0.0,
                                      "credential exists but is not claimed valid until a real request succeeds")
        if spec.kind in {ProviderKind.LOCAL_CLI, ProviderKind.EXECUTABLE}:
            if not spec.command:
                return HealthEvidence(spec.name, ProviderState.UNCONFIGURED, 0.0, "local command is absent")
            executable = spec.command[0]
            if not (Path(executable).exists() or shutil.which(executable)):
                return HealthEvidence(spec.name, ProviderState.UNCONFIGURED, 0.0,
                                      f"executable not found: {executable}")
            if not deep:
                return HealthEvidence(spec.name, ProviderState.CONFIGURED, 0.0,
                                      "executable exists but output is unverified")
        if deep:
            try:
                evidence = self.call(spec, "Reply with only: HEALTHY")
                return HealthEvidence(spec.name, ProviderState.HEALTHY,
                                      evidence.latency_ms, "real inference succeeded")
            except V1124Error as exc:
                return HealthEvidence(spec.name, self._state_for_error(exc),
                                      (time.perf_counter() - started) * 1000,
                                      f"{exc.code}: {str(exc)[:160]}")
        return HealthEvidence(spec.name, ProviderState.DEGRADED, 0.0, "health mode is unsupported")

    @staticmethod
    def _state_for_error(exc: V1124Error) -> ProviderState:
        message = str(exc).lower()
        if exc.status == 403 or "http 403" in message or "forbidden" in message:
            return ProviderState.FORBIDDEN
        if exc.code == "provider_not_configured":
            return ProviderState.UNCONFIGURED
        if exc.code == "circuit_open":
            return ProviderState.CIRCUIT_OPEN
        return ProviderState.UNAVAILABLE

    def _once(self, spec: ProviderSpec, prompt: str) -> ModelEvidence:
        if spec.kind == ProviderKind.ANTHROPIC:
            return self.gateway.call(ModelRequest("anthropic", spec.model, prompt, spec.timeout_seconds,
                                                  spec.base_url, self._credential(spec)))
        if spec.kind == ProviderKind.OPENAI:
            return self.gateway.call(ModelRequest("openai", spec.model, prompt, spec.timeout_seconds,
                                                  spec.base_url, self._credential(spec)))
        if spec.kind == ProviderKind.OLLAMA:
            health = self.ollama.ensure_running(spec) if spec.auto_start else self.ollama.probe(spec)
            if not health.healthy:
                raise ProviderUnavailable("provider_unavailable", health.detail)
            return self.gateway.call(ModelRequest("ollama", spec.model, prompt, spec.timeout_seconds,
                                                  self.ollama.base_url(spec)))
        if spec.kind in {ProviderKind.LOCAL_CLI, ProviderKind.EXECUTABLE}:
            content, latency = IsolatedProcessRunner.run(spec.command, prompt, spec.timeout_seconds,
                                                         prompt_as_argument=spec.kind == ProviderKind.LOCAL_CLI)
            transport = "cli_argument" if spec.kind == ProviderKind.LOCAL_CLI else "process_stdin"
            return ModelEvidence(spec.kind.value, spec.model, content, latency, transport, True)
        raise ProviderUnavailable("unsupported_provider", f"unsupported provider kind: {spec.kind}", 400)

    def call(self, spec: ProviderSpec, prompt: str) -> ModelEvidence:
        spec.validate()
        if not prompt.strip():
            raise V1124Error("invalid_prompt", "prompt must not be empty", 400)
        circuit = self.circuit(spec.name)
        last_error: Optional[V1124Error] = None
        for attempt in range(spec.max_attempts):
            try:
                result = circuit.call(self._once, spec, prompt)
                if result is None:
                    raise ProviderUnavailable("circuit_open", f"provider circuit open: {spec.name}")
                return result
            except V1124Error as exc:
                last_error = exc
                if exc.code not in self.RETRYABLE_CODES or attempt == spec.max_attempts - 1:
                    raise
                time.sleep(exponential_backoff(attempt, base_seconds=0.02, max_seconds=0.25, jitter="none"))
        assert last_error is not None
        raise last_error

    def route(self, specs: Iterable[ProviderSpec], prompt: str) -> RoutedEvidence:
        attempts: list[AttemptEvidence] = []
        for index, spec in enumerate(specs):
            started = time.perf_counter()
            try:
                evidence = self.call(spec, prompt)
                attempts.append(AttemptEvidence(spec.name, True, ProviderState.HEALTHY,
                                                evidence.latency_ms, detail="real inference succeeded"))
                return RoutedEvidence(spec.name, evidence, tuple(attempts), fallback_used=index > 0)
            except V1124Error as exc:
                attempts.append(AttemptEvidence(spec.name, False, self._state_for_error(exc),
                                                (time.perf_counter() - started) * 1000,
                                                error_code=exc.code, detail=str(exc)[:200]))
        return RoutedEvidence(None, None, tuple(attempts), fallback_used=len(attempts) > 1)


class W2MeasurementCoordinator:
    """Preserve V1124 identity/audit state across provider crashes."""

    def __init__(self, backend: ASINorthStarBackend, adapter: Optional[W2ProviderAdapter] = None):
        self.backend = backend
        self.adapter = adapter or W2ProviderAdapter()
        self._lock = threading.RLock()

    def measure(self, specs: Sequence[ProviderSpec], prompt: str) -> Dict[str, Any]:
        if not specs:
            raise ValueError("at least one provider is required")
        measurement_id = hashlib.sha256(f"{time.time_ns()}:{prompt}".encode()).hexdigest()[:24]
        prompt_hash = hashlib.sha256(prompt.encode()).hexdigest()
        with self._lock:
            self.backend.store.audit.append("w2_measurement_started", {
                "measurement_id": measurement_id, "prompt_sha256": prompt_hash,
                "providers": [spec.name for spec in specs],
            })
            routed = self.adapter.route(specs, prompt)
            if routed.success:
                assert routed.evidence is not None
                evidence = routed.evidence
                entry_id = self.backend.identity.add(
                    "STM", "w2_model_measurement",
                    f"{routed.selected_provider}:{evidence.model}:{hashlib.sha256(evidence.content.encode()).hexdigest()}",
                    tags=["v1128", "real-model", routed.selected_provider or "unknown"], importance=0.85,
                )
                self.backend.store.save(self.backend.identity, "w2_model_measurement")
                self.backend.store.audit.append("w2_measurement_succeeded", {
                    "measurement_id": measurement_id, "entry_id": entry_id,
                    "selected_provider": routed.selected_provider,
                    "evidence": evidence.public(),
                })
            else:
                self.backend.store.audit.append("w2_measurement_failed", {
                    "measurement_id": measurement_id,
                    "attempts": [attempt.public() for attempt in routed.attempts],
                    "identity_preserved": True,
                })
        return {"measurement_id": measurement_id, "route": routed.public(),
                "identity_preserved": self.backend.store.startup_self_check(
                    self.backend.identity.core.identity_id)["ok"],
                "level": self.backend.level(), "guards": dict(V3_GUARDS)}

    def compare(self, specs: Sequence[ProviderSpec], prompt: str) -> Dict[str, Any]:
        rows: list[Dict[str, Any]] = []
        for spec in specs:
            result = self.measure([spec], prompt)
            route = result["route"]
            evidence = route["evidence"]
            rows.append({"provider": spec.name, "kind": spec.kind.value, "model": spec.model,
                         "success": route["success"], "state": route["attempts"][-1]["state"],
                         "latency_ms": evidence["latency_ms"] if evidence else None,
                         "content_sha256": evidence["content_sha256"] if evidence else None,
                         "asi_level_proxy": result["level"]["score"] if evidence else None,
                         "error_code": None if evidence else route["attempts"][-1]["error_code"]})
        successes = sum(1 for row in rows if row["success"])
        return {"version": V1128_VERSION, "w2_target": W2_TARGET, "rows": rows,
                "providers_attempted": len(rows), "providers_succeeded": successes,
                "target_claimed": False,
                "truth_note": f"{successes}/{len(rows)} providers succeeded; failures were not scored",
                "guards": dict(V3_GUARDS)}


def default_provider_specs() -> list[ProviderSpec]:
    """Return five genuinely executable provider paths, whether configured or not.

    R10-BE-003 task description lists 4 forced-parallel providers (Anthropic /
    OpenAI / Ollama / local); we expose 5 here because executable (stdin) is a
    distinct transport surface used by standalone tests. The forced-parallel
    default is enforced by V1130.default_cross_provider_plan which deliberately
    uses only the 4 enumerated in the task.
    """
    cli = tuple(shlex.split(os.getenv("V1128_LOCAL_CLI", ""), posix=os.name != "nt"))
    executable = tuple(shlex.split(os.getenv("V1128_EXECUTABLE", ""), posix=os.name != "nt"))
    return [
        ProviderSpec("anthropic", ProviderKind.ANTHROPIC,
                     os.getenv("ANTHROPIC_MODEL", "claude-3-5-haiku-20241022"),
                     api_key=os.getenv("ANTHROPIC_API_KEY")),
        ProviderSpec("openai", ProviderKind.OPENAI,
                     os.getenv("OPENAI_MODEL", "gpt-4o-mini"),
                     api_key=os.getenv("OPENAI_API_KEY"),
                     base_url=os.getenv("OPENAI_BASE_URL")),
        ProviderSpec("ollama-qwen", ProviderKind.OLLAMA, DEFAULT_OLLAMA_MODELS[0],
                     base_url=os.getenv("OLLAMA_BASE_URL"), auto_start=True),
        ProviderSpec("local-cli", ProviderKind.LOCAL_CLI,
                     os.getenv("V1128_LOCAL_CLI_MODEL", "configured-local-cli"), command=cli),
        ProviderSpec("local-executable", ProviderKind.EXECUTABLE,
                     os.getenv("V1128_EXECUTABLE_MODEL", "configured-local-executable"), command=executable),
    ]


__all__ = [
    "V1128_VERSION", "W2_TARGET", "DEFAULT_OLLAMA_MODELS", "V3_GUARDS",
    "ProviderState", "ProviderKind", "ProviderSpec", "HealthEvidence", "AttemptEvidence",
    "RoutedEvidence", "ProviderUnavailable", "OllamaRuntime", "IsolatedProcessRunner",
    "W2ProviderAdapter", "W2MeasurementCoordinator", "default_provider_specs",
]
