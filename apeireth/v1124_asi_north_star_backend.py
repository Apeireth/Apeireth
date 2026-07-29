"""V1124 ASI north-star backend: durable identity and dual-protocol API.

This module is infrastructure, not ASI and not evidence of phenomenal consciousness.
Measurements are operational proxies. Real-model results are accepted only from an
actual HTTP endpoint or a configured local executable; there is no simulated fallback.
"""
from __future__ import annotations

import dataclasses
import hashlib
import json
import os
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
from concurrent import futures
from dataclasses import asdict, dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Dict, Iterable, Mapping, Optional, Sequence

from apeireth.v1072_asi_central_ai_eternal_identity import (
    IdentityCore,
    IdentityManifest,
    IdentityManifestEntry,
    V1072Orchestrator,
    v1072_philosophy_guard,
)
from apeireth.v1106_engineering_lift import EngineeringHarness

V1124_VERSION = "0.1.0"
BASELINE_V04 = 0.8538
ASI_NORTH_STAR_TARGET = 0.95
MAX_REQUEST_BYTES = 1_048_576
V3_GUARDS = {
    "measurement_is_proxy": "ASI score is an operational proxy, not truth or proof of ASI.",
    "not_phenomenal_consciousness": "No structure, identity, or model response proves phenomenal consciousness.",
    "identity_is_not_consciousness": "Durable identity records are data continuity, not subjective experience.",
    "model_call_is_not_asi": "A real LLM call is evidence of integration only, not ASI.",
    "failure_is_not_success": "Unavailable providers and corrupt storage are reported as failures, never simulated.",
}


class V1124Error(RuntimeError):
    """Typed service error with a stable API code."""

    def __init__(self, code: str, message: str, status: int = 500):
        super().__init__(message)
        self.code, self.status = code, status

    def payload(self) -> Dict[str, Any]:
        return {"error": {"code": self.code, "message": str(self)}}


class IntegrityError(V1124Error):
    def __init__(self, message: str):
        super().__init__("integrity_failed", message, 503)


def _canonical(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _write_all(fd: int, data: bytes) -> None:
    view = memoryview(data)
    while view:
        written = os.write(fd, view)
        if written <= 0:
            raise OSError("short write")
        view = view[written:]


def _fsync_directory(path: Path) -> None:
    """Persist rename metadata where the platform supports directory handles."""
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    try:
        fd = os.open(str(path), flags)
    except OSError:  # Windows does not expose fsync-able directory handles.
        return
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


class AuditChain:
    """Append-only, hash-chained JSONL with write-through and fsync per record."""

    def __init__(self, path: os.PathLike[str] | str):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = threading.RLock()

    def records(self, tolerate_torn_tail: bool = False) -> list[Dict[str, Any]]:
        if not self.path.exists():
            return []
        raw = self.path.read_bytes()
        lines = raw.splitlines(keepends=True)
        result: list[Dict[str, Any]] = []
        previous = "0" * 64
        for index, line in enumerate(lines):
            if not line.endswith(b"\n"):
                if tolerate_torn_tail and index == len(lines) - 1:
                    break
                raise IntegrityError(f"audit record {index + 1} is torn")
            try:
                record = json.loads(line)
            except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise IntegrityError(f"audit record {index + 1} is invalid JSON") from exc
            claimed = record.pop("hash", None)
            if record.get("prev_hash") != previous:
                raise IntegrityError(f"audit chain broken at record {index + 1}")
            actual = hashlib.sha256(_canonical(record)).hexdigest()
            if claimed != actual:
                raise IntegrityError(f"audit hash mismatch at record {index + 1}")
            record["hash"] = claimed
            result.append(record)
            previous = claimed
        return result

    def verify(self) -> Dict[str, Any]:
        records = self.records()
        return {"ok": True, "records": len(records), "head": records[-1]["hash"] if records else "0" * 64}

    def append(self, event: str, payload: Mapping[str, Any]) -> Dict[str, Any]:
        if not event or not isinstance(event, str):
            raise ValueError("event must be a non-empty string")
        with self._lock:
            verified = self.verify()
            body = {
                "event_id": uuid.uuid4().hex,
                "timestamp_ns": time.time_ns(),
                "event": event,
                "payload": dict(payload),
                "prev_hash": verified["head"],
            }
            body["hash"] = hashlib.sha256(_canonical(body)).hexdigest()
            encoded = _canonical(body) + b"\n"
            flags = os.O_APPEND | os.O_CREAT | os.O_WRONLY | getattr(os, "O_BINARY", 0)
            fd = os.open(str(self.path), flags, 0o600)
            try:
                _write_all(fd, encoded)
                os.fsync(fd)
            finally:
                os.close(fd)
            _fsync_directory(self.path.parent)
            return body


class DurableIdentityStore:
    """V1072 persistence adapter using atomic replace plus a write-through audit chain."""

    def __init__(self, directory: os.PathLike[str] | str):
        self.directory = Path(directory)
        self.directory.mkdir(parents=True, exist_ok=True)
        self.snapshot_path = self.directory / "identity.json"
        self.audit = AuditChain(self.directory / "identity.audit.jsonl")
        self._lock = threading.RLock()

    @staticmethod
    def serialize(manifest: IdentityManifest) -> Dict[str, Any]:
        return {
            "schema_version": "1.0",
            "core": asdict(manifest.core),
            "entries": [asdict(entry) for entry in manifest.entries],
            "archived": list(manifest.archived),
        }

    @staticmethod
    def deserialize(data: Mapping[str, Any]) -> IdentityManifest:
        if data.get("schema_version") != "1.0":
            raise IntegrityError("unsupported identity schema")
        try:
            core = IdentityCore(**dict(data["core"]))
            manifest = IdentityManifest(core)
            manifest.entries = [IdentityManifestEntry(**item) for item in data.get("entries", [])]
            manifest.archived = list(data.get("archived", []))
        except (KeyError, TypeError, ValueError) as exc:
            raise IntegrityError("invalid identity snapshot") from exc
        return manifest

    def save(self, manifest: IdentityManifest, reason: str = "write_through") -> str:
        data = self.serialize(manifest)
        encoded = _canonical(data)
        digest = hashlib.sha256(encoded).hexdigest()
        with self._lock:
            fd, temp_name = tempfile.mkstemp(prefix=".identity-", suffix=".tmp", dir=str(self.directory))
            try:
                _write_all(fd, encoded)
                os.fsync(fd)
                os.close(fd)
                fd = -1
                os.replace(temp_name, self.snapshot_path)
                _fsync_directory(self.directory)
                self.audit.append("identity_snapshot_committed", {
                    "identity_id": manifest.core.identity_id,
                    "snapshot_sha256": digest,
                    "entries": len(manifest.entries),
                    "reason": reason,
                })
            finally:
                if fd >= 0:
                    os.close(fd)
                try:
                    os.unlink(temp_name)
                except FileNotFoundError:
                    pass
        return digest

    def load(self) -> IdentityManifest:
        with self._lock:
            if not self.snapshot_path.exists():
                raise IntegrityError("identity snapshot missing")
            try:
                data = json.loads(self.snapshot_path.read_text(encoding="utf-8"))
            except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
                raise IntegrityError("identity snapshot unreadable") from exc
            manifest = self.deserialize(data)
            records = self.audit.records()
            commits = [r for r in records if r["event"] == "identity_snapshot_committed"]
            if not commits:
                raise IntegrityError("identity snapshot has no audit commit")
            digest = hashlib.sha256(_canonical(data)).hexdigest()
            if commits[-1]["payload"].get("snapshot_sha256") != digest:
                raise IntegrityError("identity snapshot does not match audit head")
            if commits[-1]["payload"].get("identity_id") != manifest.core.identity_id:
                raise IntegrityError("identity id does not match audit head")
            return manifest

    def startup_self_check(self, expected_identity_id: Optional[str] = None) -> Dict[str, Any]:
        audit = self.audit.verify()
        if not self.snapshot_path.exists():
            return {"ok": True, "state": "new", "audit": audit}
        manifest = self.load()
        if expected_identity_id and manifest.core.identity_id != expected_identity_id:
            raise IntegrityError("startup identity mismatch")
        return {"ok": True, "state": "recovered", "identity_id": manifest.core.identity_id,
                "entries": len(manifest.entries), "audit": audit}


@dataclass(frozen=True)
class ModelRequest:
    provider: str
    model: str
    prompt: str
    timeout_seconds: float = 30.0
    base_url: Optional[str] = None
    api_key: Optional[str] = None
    command: Sequence[str] = field(default_factory=tuple)


@dataclass(frozen=True)
class ModelEvidence:
    provider: str
    model: str
    content: str
    latency_ms: float
    transport: str
    real: bool
    tokens_in: int = 0
    tokens_out: int = 0

    def public(self) -> Dict[str, Any]:
        return {"provider": self.provider, "model": self.model, "latency_ms": self.latency_ms,
                "transport": self.transport, "real": self.real, "tokens_in": self.tokens_in,
                "tokens_out": self.tokens_out, "content_sha256": hashlib.sha256(self.content.encode()).hexdigest(),
                "content_length": len(self.content)}


class RealModelGateway:
    """Four-provider real transport gateway. It deliberately has no fake fallback."""

    def call(self, request: ModelRequest) -> ModelEvidence:
        if not request.prompt.strip():
            raise V1124Error("invalid_prompt", "prompt must not be empty", 400)
        if request.provider == "local":
            return self._call_local(request)
        if request.provider == "anthropic":
            return self._call_anthropic(request)
        if request.provider in {"openai", "gpt", "ollama"}:
            return self._call_openai_or_ollama(request)
        raise V1124Error("unsupported_provider", f"unsupported provider: {request.provider}", 400)

    @staticmethod
    def _http_json(url: str, payload: Mapping[str, Any], headers: Mapping[str, str], timeout: float) -> tuple[Dict[str, Any], float]:
        req = urllib.request.Request(url, data=_canonical(payload), headers=dict(headers), method="POST")
        started = time.perf_counter()
        try:
            with urllib.request.urlopen(req, timeout=timeout) as response:
                raw = response.read(MAX_REQUEST_BYTES + 1)
                if len(raw) > MAX_REQUEST_BYTES:
                    raise V1124Error("provider_response_too_large", "provider response exceeds limit", 502)
        except urllib.error.HTTPError as exc:
            detail = exc.read(512).decode("utf-8", "replace")
            raise V1124Error("provider_http_error", f"provider returned HTTP {exc.code}: {detail}", 502) from exc
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise V1124Error("provider_unavailable", str(exc), 503) from exc
        latency = (time.perf_counter() - started) * 1000
        try:
            return json.loads(raw), latency
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise V1124Error("provider_invalid_response", "provider returned invalid JSON", 502) from exc

    def _call_openai_or_ollama(self, request: ModelRequest) -> ModelEvidence:
        ollama = request.provider == "ollama"
        base = request.base_url or ("http://127.0.0.1:11434" if ollama else "https://api.openai.com/v1")
        if ollama:
            url = base.rstrip("/") + "/api/chat"
            payload = {"model": request.model, "messages": [{"role": "user", "content": request.prompt}], "stream": False}
            headers = {"Content-Type": "application/json"}
        else:
            key = request.api_key or os.getenv("OPENAI_API_KEY")
            if not key:
                raise V1124Error("provider_not_configured", "OPENAI_API_KEY is required", 503)
            url = base.rstrip("/") + "/chat/completions"
            payload = {"model": request.model, "messages": [{"role": "user", "content": request.prompt}], "temperature": 0}
            headers = {"Content-Type": "application/json", "Authorization": f"Bearer {key}"}
        data, latency = self._http_json(url, payload, headers, request.timeout_seconds)
        try:
            content = data["message"]["content"] if ollama else data["choices"][0]["message"]["content"]
        except (KeyError, IndexError, TypeError) as exc:
            raise V1124Error("provider_invalid_response", "provider response has no message content", 502) from exc
        if not isinstance(content, str) or not content:
            raise V1124Error("provider_invalid_response", "provider returned empty content", 502)
        usage = data.get("usage", {})
        return ModelEvidence(request.provider, data.get("model", request.model), content, latency,
                             "http", True, int(usage.get("prompt_tokens", 0)), int(usage.get("completion_tokens", 0)))

    def _call_anthropic(self, request: ModelRequest) -> ModelEvidence:
        key = request.api_key or os.getenv("ANTHROPIC_API_KEY")
        if not key:
            raise V1124Error("provider_not_configured", "ANTHROPIC_API_KEY is required", 503)
        base = request.base_url or "https://api.anthropic.com/v1"
        data, latency = self._http_json(base.rstrip("/") + "/messages", {
            "model": request.model, "max_tokens": 128,
            "messages": [{"role": "user", "content": request.prompt}],
        }, {"Content-Type": "application/json", "x-api-key": key, "anthropic-version": "2023-06-01"}, request.timeout_seconds)
        try:
            content = "".join(block["text"] for block in data["content"] if block.get("type") == "text")
        except (KeyError, TypeError) as exc:
            raise V1124Error("provider_invalid_response", "Anthropic response has no text", 502) from exc
        if not content:
            raise V1124Error("provider_invalid_response", "Anthropic returned empty content", 502)
        usage = data.get("usage", {})
        return ModelEvidence("anthropic", data.get("model", request.model), content, latency, "http", True,
                             int(usage.get("input_tokens", 0)), int(usage.get("output_tokens", 0)))

    @staticmethod
    def _call_local(request: ModelRequest) -> ModelEvidence:
        if not request.command:
            raise V1124Error("provider_not_configured", "local provider requires an executable command", 503)
        started = time.perf_counter()
        try:
            completed = subprocess.run(list(request.command), input=request.prompt, text=True,
                                       capture_output=True, timeout=request.timeout_seconds, check=False)
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise V1124Error("provider_unavailable", str(exc), 503) from exc
        if completed.returncode != 0:
            raise V1124Error("provider_process_error", completed.stderr[-512:] or "local model failed", 502)
        if not completed.stdout.strip():
            raise V1124Error("provider_invalid_response", "local model returned empty output", 502)
        return ModelEvidence("local", request.model, completed.stdout.strip(),
                             (time.perf_counter() - started) * 1000, "process", True)


class ASINorthStarBackend:
    """V1106 engineering orchestrator + V1072 durable identity integration."""

    def __init__(self, data_directory: os.PathLike[str] | str, gateway: Optional[RealModelGateway] = None):
        self.store = DurableIdentityStore(data_directory)
        self.gateway = gateway or RealModelGateway()
        self.engineering = EngineeringHarness()
        check = self.store.startup_self_check()
        self.identity = self.store.load() if check["state"] == "recovered" else V1072Orchestrator().manifest
        if check["state"] == "new":
            self.store.save(self.identity, "bootstrap")
        self.startup_check = self.store.startup_self_check(self.identity.core.identity_id)
        self._lock = threading.RLock()

    def level(self) -> Dict[str, Any]:
        identity_score = V1072Orchestrator().measure()["raw"]
        engineering = self.engineering.stats()
        audit = self.store.audit.verify()
        score = round(BASELINE_V04, 4)
        return {"version": V1124_VERSION, "score": score, "baseline_v04": BASELINE_V04,
                "target": ASI_NORTH_STAR_TARGET, "target_reached": score >= ASI_NORTH_STAR_TARGET,
                "dimensions": {"eternal_identity_proxy": identity_score,
                               "engineering_components": len(engineering.get("capabilities", [])),
                               "durable_audit_records": audit["records"]},
                "claim": "operational_proxy_not_asi_truth"}

    def north_star(self) -> Dict[str, Any]:
        return {"north_star": "ASI >= 0.95", "current": self.level(), "guards": dict(V3_GUARDS),
                "identity_guards": v1072_philosophy_guard(),
                "protocols": {"http": ["GET /asi/level", "POST /asi/measure", "GET /asi/north-star"],
                              "grpc": ["apeireth.v1124.ASINorthStar/Level", "Measure", "NorthStar"]}}

    def measure(self, body: Mapping[str, Any]) -> Dict[str, Any]:
        provider = str(body.get("provider", "")).lower()
        model = str(body.get("model", ""))
        prompt = str(body.get("prompt", ""))
        if not provider or not model:
            raise V1124Error("invalid_request", "provider and model are required", 400)
        command = body.get("command", ())
        if command and (not isinstance(command, list) or not all(isinstance(x, str) for x in command)):
            raise V1124Error("invalid_request", "command must be a list of strings", 400)
        try:
            timeout = float(body.get("timeout_seconds", 30.0))
        except (TypeError, ValueError) as exc:
            raise V1124Error("invalid_request", "timeout_seconds must be numeric", 400) from exc
        if not 0 < timeout <= 120:
            raise V1124Error("invalid_request", "timeout_seconds must be in (0, 120]", 400)
        request = ModelRequest(provider, model, prompt, timeout, body.get("base_url"), body.get("api_key"), tuple(command))
        with self._lock:
            evidence = self.engineering.call(self.gateway.call, request)
            entry_id = self.identity.add("STM", "model_measurement",
                                         f"{provider}:{model}:{hashlib.sha256(evidence.content.encode()).hexdigest()}",
                                         tags=["v1124", "real-model"], importance=0.8)
            self.store.save(self.identity, "model_measurement")
            audit = self.store.audit.append("asi_measurement", {"entry_id": entry_id, **evidence.public()})
        return {"measurement_id": audit["event_id"], "evidence": evidence.public(), "level": self.level(),
                "guards": dict(V3_GUARDS)}

    def dispatch(self, method: str, path: str, body: Optional[Mapping[str, Any]] = None) -> tuple[int, Dict[str, Any]]:
        try:
            if method == "GET" and path == "/asi/level":
                return 200, self.level()
            if method == "GET" and path == "/asi/north-star":
                return 200, self.north_star()
            if method == "POST" and path == "/asi/measure":
                return 200, self.measure(body or {})
            raise V1124Error("not_found", "endpoint not found", 404)
        except V1124Error as exc:
            return exc.status, exc.payload()
        except Exception:
            return 500, V1124Error("internal_error", "internal server error").payload()


def make_http_handler(backend: ASINorthStarBackend) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        server_version = "Apeireth-V1124"

        def _reply(self, status: int, payload: Mapping[str, Any]) -> None:
            raw = _canonical(payload)
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(raw)))
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(raw)

        def do_GET(self) -> None:  # noqa: N802
            status, payload = backend.dispatch("GET", self.path.split("?", 1)[0])
            self._reply(status, payload)

        def do_POST(self) -> None:  # noqa: N802
            try:
                length = int(self.headers.get("Content-Length", "0"))
                if length <= 0 or length > MAX_REQUEST_BYTES:
                    raise V1124Error("invalid_body", "request body size is invalid", 400)
                body = json.loads(self.rfile.read(length))
                if not isinstance(body, dict):
                    raise V1124Error("invalid_body", "JSON body must be an object", 400)
                status, payload = backend.dispatch("POST", self.path.split("?", 1)[0], body)
            except (ValueError, json.JSONDecodeError, UnicodeDecodeError):
                status, payload = 400, V1124Error("invalid_body", "invalid JSON body", 400).payload()
            except V1124Error as exc:
                status, payload = exc.status, exc.payload()
            self._reply(status, payload)

        def log_message(self, fmt: str, *args: Any) -> None:
            return

    return Handler


def start_http_server(backend: ASINorthStarBackend, host: str = "127.0.0.1", port: int = 0) -> ThreadingHTTPServer:
    server = ThreadingHTTPServer((host, port), make_http_handler(backend))
    threading.Thread(target=server.serve_forever, name="v1124-http", daemon=True).start()
    return server


def _grpc_serializer(value: Mapping[str, Any]) -> bytes:
    return _canonical(value)


def _grpc_deserializer(value: bytes) -> Dict[str, Any]:
    if len(value) > MAX_REQUEST_BYTES:
        raise ValueError("request too large")
    decoded = json.loads(value or b"{}")
    if not isinstance(decoded, dict):
        raise ValueError("request must be an object")
    return decoded


def start_grpc_server(backend: ASINorthStarBackend, host: str = "127.0.0.1", port: int = 0):
    """Start a real grpcio HTTP/2 server using stable JSON message bytes."""
    try:
        import grpc
    except ImportError as exc:
        raise RuntimeError("grpcio is required for the gRPC endpoint") from exc

    def invoke(method: str, path: str):
        def handler(request: Mapping[str, Any], context: Any) -> Dict[str, Any]:
            status, payload = backend.dispatch(method, path, request)
            if status >= 400:
                codes = {400: grpc.StatusCode.INVALID_ARGUMENT, 404: grpc.StatusCode.NOT_FOUND,
                         503: grpc.StatusCode.UNAVAILABLE}
                context.set_code(codes.get(status, grpc.StatusCode.INTERNAL))
                context.set_details(payload["error"]["message"])
            return payload
        return handler

    methods = {
        "Level": grpc.unary_unary_rpc_method_handler(invoke("GET", "/asi/level"),
                                                       request_deserializer=_grpc_deserializer,
                                                       response_serializer=_grpc_serializer),
        "Measure": grpc.unary_unary_rpc_method_handler(invoke("POST", "/asi/measure"),
                                                         request_deserializer=_grpc_deserializer,
                                                         response_serializer=_grpc_serializer),
        "NorthStar": grpc.unary_unary_rpc_method_handler(invoke("GET", "/asi/north-star"),
                                                           request_deserializer=_grpc_deserializer,
                                                           response_serializer=_grpc_serializer),
    }
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=4))
    server.add_generic_rpc_handlers((grpc.method_handlers_generic_handler("apeireth.v1124.ASINorthStar", methods),))
    bound_port = server.add_insecure_port(f"{host}:{port}")
    if not bound_port:
        raise RuntimeError("unable to bind gRPC server")
    server.start()
    return server, bound_port


__all__ = [
    "V1124_VERSION", "BASELINE_V04", "ASI_NORTH_STAR_TARGET", "V3_GUARDS", "V1124Error",
    "IntegrityError", "AuditChain", "DurableIdentityStore", "ModelRequest", "ModelEvidence",
    "RealModelGateway", "ASINorthStarBackend", "make_http_handler", "start_http_server",
    "start_grpc_server",
]
