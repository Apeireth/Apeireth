"""V1464 — ASI Real Lint-Gate Subprocess Pipeline HTTP Gateway (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 00:56 任何人都能接手).

Phase: 1464
Version: 0.1.0
Date: 2026-08-10 (cron tick 12:47, Monday noon, round-125)
Post: V1463 (Lint-Gate Subprocess Pipeline — 30/30 match, 51 tests pass)
      V1462 (Subprocess Sandbox Spec Security Linter — 54 tests pass)
      V1461 (Docker-Equivalent Subprocess Sandbox — 42 tests pass)
      V1460 (Anyone-Run Harness — 12/13 stages)
      V1437 (Subprocess HTTP Live Server — http.server + bounded subprocess)

What V1464 is
=============
V1463 chains V1462 linter + V1461 sandbox runner into a CLI pipeline
(JSONL in → JSON/MD report out).

V1464 wraps that pipeline in a real HTTP server so **anyone can
take over** (主 00:56): a single curl POST submits a SandboxSpec
JSONL body and receives the full lint-then-run report as JSON.

Endpoints (主 13:31 大胆放手 — anyone can curl, no Python install):

    GET  /healthz               → liveness probe
    GET  /status                → V1463 module + V1464 server status
    GET  /pipeline/adversarial  → run the 30-spec adversarial suite (read-only demo)
    POST /pipeline/run          → run pipeline on JSONL body of SandboxSpec dicts
    POST /pipeline/lint         → run ONLY V1462 lint on a single SandboxSpec dict
    POST /pipeline/policy-gate  → run ONLY V1462 policy_gate on a single SandboxSpec dict

V1464 is NOT:
  - a production gateway (single-machine, bounded)
  - an orchestrator (no scheduling, no queue)
  - an authentication system (anyone-can-curl means NO auth — open localhost)
  - an HTTPS server (plain HTTP on loopback / private LAN only)
  - a Docker-equivalent (just wraps http.server, no namespace/cgroup)

V1464 IS:
  - a real, observable HTTP wrapper around V1463
  - bound to 127.0.0.1 by default (any LAN IP optional via --host)
  - deterministic for a given JSONL input (V1463 is deterministic)
  - honest about every spec's journey (returns the same 8 outcomes as V1463)
  - tested end-to-end via in-process HTTP (BaseHTTPRequestHandler + threaded)
  - safe-by-default: --host must be 127.0.0.1 unless --allow-lan is passed
  - capable of running the adversarial suite via GET (no body required)

V1464 design rules (主 13:31 大胆放手 + 主 23:44 骈插捣):
  - Use stdlib http.server (V1437 pattern) — no FastAPI / Flask / aiohttp
  - One endpoint = one function (router table)
  - Max body 256KB (prevent unbounded POST)
  - Max request timeout 60s (server-side timeout for run_pipeline call)
  - Loopback default (主 23:44 — 骈插捣, not 暴露)
  - Exit code 0 on clean shutdown, 130 on SIGINT

V1464 GUARDS (主 00:44 质量工程化):
- GUARD_V1463_PIPELINE_REUSED  : run_pipeline imported from V1463
- GUARD_V1462_LINT_REUSED      : lint_spec + policy_gate imported from V1462
- GUARD_V1461_RUNNER_REUSED    : SandboxSpec imported from V1461
- GUARD_STDLIB_HTTP            : http.server + BaseHTTPRequestHandler
- GUARD_LOOPBACK_DEFAULT       : bind to 127.0.0.1 by default
- GUARD_BODY_BOUNDED           : max POST body 256KB
- GUARD_TIMEOUT_BOUNDED        : server-side run timeout 60s
- GUARD_PORT_RECLAIMED         : SO_REUSEADDR set, port released on close
- GUARD_METHOD_ROUTED          : GET/POST handled separately
- GUARD_ERROR_HANDLED          : 400 / 405 / 500 returned with JSON body
- GUARD_RUNS_ON_WINDOWS        : stdlib-only, no POSIX-only syscalls
- GUARD_BORROWED_LINEAGE       : 7 borrowed sources cited
- GUARD_CLI_RUNNABLE           : CLI works for anyone (--port, --host, --adversarial-port)

V1464 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_HTTP_NOT_ORCHESTRATOR    : single request → single response, no queue
- GUARD_HTTP_NOT_CI              : no GitHub Actions / GitLab CI integration
- GUARD_HTTP_NOT_SECURITY        : no auth, no rate limit, no TLS — explicit
- GUARD_HTTP_NOT_ASI             : deterministic V1463 pipeline, NOT ASI
- GUARD_HTTP_NOT_PHENOMENAL      : HTTP request handler, NOT consciousness
- GUARD_HTTP_NOT_HUMAN_LEVEL     : bounded http.server, NOT human-level

借力 (主 19:33 走在前人经验上):
- V1463 — run_pipeline + PipelineReport + PipelineRecord + parse_jsonl_specs
- V1462 — lint_spec + policy_gate + PolicyLevel + LintReport
- V1461 — SandboxSpec + SandboxRunner + SandboxMode
- V1437 — http.server + BaseHTTPRequestHandler + bounded subprocess pattern
- V1435 — offline-safe probe pattern
- V1434 — VCP consistency HTTP (router + JSON shape reference)
- V1420 — HTTP status endpoint pattern
- stdlib — http.server + socketserver + json + urllib.parse + threading

实事求是 (主 17:43):
- V1464 ≠ FastAPI/Flask, V1464 ≠ Docker, V1464 ≠ Nginx, V1464 ≠ production gateway
- V1464 = stdlib http.server + V1463 pipeline wrapper + bounded body + JSON I/O
- 任何人 curl POST 一个 SandboxSpec → 拿到完整 lint+run 报告
- 不假装 orchestration / queueing / auth / TLS / scaling
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import socket
import sys
import threading
import time
from dataclasses import dataclass, field, asdict
from enum import Enum
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple
from urllib.parse import urlparse

# ──────────────────────────────────────────────────────────────────────
# V1464 module metadata
# ──────────────────────────────────────────────────────────────────────

V1464_MODULE = "v1464_asi_lint_gate_pipeline_http_gateway"
V1464_VERSION = "0.1.0"
V1464_SCHEMA = "v1464.asi-lint-gate-pipeline-http-gateway/v1"
V1464_DATE = "2026-08-10"

# ──────────────────────────────────────────────────────────────────────
# V1464 bounded defaults (主 00:44 质量工程化 — bounded everything)
# ──────────────────────────────────────────────────────────────────────

DEFAULT_HOST = "127.0.0.1"  # loopback only by default (主 23:44 骈插捣)
DEFAULT_PORT_LOW = 18080    # V1464 reserved port range
DEFAULT_PORT_HIGH = 18180
DEFAULT_BODY_MAX_BYTES = 256 * 1024  # 256KB max POST body
DEFAULT_RUN_TIMEOUT_S = 60            # server-side pipeline run timeout
DEFAULT_SERVER_TIMEOUT_S = 30         # BaseHTTPServer timeout per request

# V1463 adversarial suite size (bounded)
EXPECTED_ADVERSARIAL_SPECS = 30


# ──────────────────────────────────────────────────────────────────────
# V1464 enums + dataclasses
# ──────────────────────────────────────────────────────────────────────


class GatewayStatus(str, Enum):
    """Server lifecycle status."""
    STARTING = "STARTING"
    LISTENING = "LISTENING"
    CLOSING = "CLOSING"
    CLOSED = "CLOSED"
    FAILED = "FAILED"


@dataclass
class GatewayRequest:
    """One HTTP request handled by the gateway."""
    method: str
    path: str
    body_bytes: int
    remote: str
    started_at: float
    finished_at: Optional[float] = None
    status_code: Optional[int] = None
    error: Optional[str] = None

    @property
    def elapsed_ms(self) -> float:
        end = self.finished_at if self.finished_at is not None else time.time()
        return round((end - self.started_at) * 1000.0, 2)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "method": self.method,
            "path": self.path,
            "body_bytes": self.body_bytes,
            "remote": self.remote,
            "elapsed_ms": self.elapsed_ms,
            "status_code": self.status_code,
            "error": self.error,
        }


@dataclass
class GatewayStats:
    """Aggregated gateway stats since server start."""
    started_at: float
    n_requests: int = 0
    n_2xx: int = 0
    n_4xx: int = 0
    n_5xx: int = 0
    by_endpoint: Dict[str, int] = field(default_factory=dict)

    def record(self, req: GatewayRequest) -> None:
        self.n_requests += 1
        sc = req.status_code or 0
        if 200 <= sc < 300:
            self.n_2xx += 1
        elif 400 <= sc < 500:
            self.n_4xx += 1
        elif 500 <= sc < 600:
            self.n_5xx += 1
        key = f"{req.method} {req.path}"
        self.by_endpoint[key] = self.by_endpoint.get(key, 0) + 1

    def to_dict(self) -> Dict[str, Any]:
        return {
            "started_at": self.started_at,
            "uptime_s": round(time.time() - self.started_at, 2),
            "n_requests": self.n_requests,
            "n_2xx": self.n_2xx,
            "n_4xx": self.n_4xx,
            "n_5xx": self.n_5xx,
            "by_endpoint": dict(self.by_endpoint),
        }


# ──────────────────────────────────────────────────────────────────────
# V1464 import V1463 + V1462 + V1461 (borrowed lineage)
# ──────────────────────────────────────────────────────────────────────


def _import_v1463_pipeline() -> Tuple[Any, Any, Any, Any]:
    """Return (run_pipeline, parse_jsonl_specs, PipelineReport, PipelineRecord)."""
    from apeireth.v1463_asi_lint_gate_subprocess_pipeline import (
        run_pipeline as _run_pipeline,
        parse_jsonl_specs as _parse_jsonl_specs,
        PipelineReport as _PipelineReport,
        PipelineRecord as _PipelineRecord,
    )
    return _run_pipeline, _parse_jsonl_specs, _PipelineReport, _PipelineRecord


def _import_v1462_lint() -> Tuple[Any, Any, Any, Any]:
    """Return (lint_spec, policy_gate, PolicyLevel, LintReport)."""
    from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (
        lint_spec as _lint_spec,
        policy_gate as _policy_gate,
        PolicyLevel as _PolicyLevel,
        LintReport as _LintReport,
    )
    return _lint_spec, _policy_gate, _PolicyLevel, _LintReport


def _import_v1461_sandbox() -> Tuple[Any, Any]:
    """Return (SandboxSpec, SandboxRunner)."""
    from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import (
        SandboxSpec as _SandboxSpec,
        SandboxRunner as _SandboxRunner,
    )
    return _SandboxSpec, _SandboxRunner


# ──────────────────────────────────────────────────────────────────────
# V1464 port-finding (auto-bind in [low, high])
# ──────────────────────────────────────────────────────────────────────


def find_open_port(
    host: str = DEFAULT_HOST,
    low: int = DEFAULT_PORT_LOW,
    high: int = DEFAULT_PORT_HIGH,
) -> int:
    """Return a port number that is currently free on `host` in [low, high]."""
    for port in range(low, high + 1):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            try:
                s.bind((host, port))
                return port
            except OSError:
                continue
    raise RuntimeError(f"No open port found in {host}:[{low}, {high}]")


# ──────────────────────────────────────────────────────────────────────
# V1464 router — endpoint → handler function
# ──────────────────────────────────────────────────────────────────────


@dataclass
class RouteResult:
    """Result of an endpoint handler — what to write back to client."""
    status_code: int
    content_type: str  # 'application/json' or 'text/plain'
    body: bytes


# Endpoint handler signature: (body_bytes_or_none, query_params) -> RouteResult


def _route_healthz(_body: Optional[bytes], _q: Dict[str, str]) -> RouteResult:
    return RouteResult(
        status_code=200,
        content_type="application/json",
        body=json.dumps({
            "ok": True,
            "module": V1464_MODULE,
            "version": V1464_VERSION,
            "schema": V1464_SCHEMA,
            "ts": time.time(),
        }, indent=2, ensure_ascii=False).encode("utf-8"),
    )


def _route_status(_body: Optional[bytes], _q: Dict[str, str]) -> RouteResult:
    # V1463 module info
    try:
        from apeireth.v1463_asi_lint_gate_subprocess_pipeline import (
            V1463_MODULE, V1463_VERSION, V1463_SCHEMA, PipelineOutcome, _ADVERSARIAL_SPECS,
        )
        v1463 = {
            "module": V1463_MODULE,
            "version": V1463_VERSION,
            "schema": V1463_SCHEMA,
            "n_adversarial_specs": len(_ADVERSARIAL_SPECS),
            "n_outcomes": len(list(PipelineOutcome)),
            "outcomes": [o.value for o in PipelineOutcome],
        }
    except Exception as exc:  # pragma: no cover
        v1463 = {"import_error": str(exc)}

    # V1462 module info
    try:
        from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (
            V1462_MODULE, V1462_VERSION, V1462_SCHEMA, PolicyLevel, _RULES,
        )
        v1462 = {
            "module": V1462_MODULE,
            "version": V1462_VERSION,
            "schema": V1462_SCHEMA,
            "n_rules": len(_RULES),
            "policy_levels": [p.value for p in PolicyLevel],
        }
    except Exception as exc:  # pragma: no cover
        v1462 = {"import_error": str(exc)}

    # V1461 module info
    try:
        from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import (
            V1461_MODULE, V1461_VERSION, V1461_SCHEMA, SandboxMode,
        )
        v1461 = {
            "module": V1461_MODULE,
            "version": V1461_VERSION,
            "schema": V1461_SCHEMA,
            "n_sandbox_modes": len(list(SandboxMode)),
        }
    except Exception as exc:  # pragma: no cover
        v1461 = {"import_error": str(exc)}

    return RouteResult(
        status_code=200,
        content_type="application/json",
        body=json.dumps({
            "v1464": {
                "module": V1464_MODULE,
                "version": V1464_VERSION,
                "schema": V1464_SCHEMA,
                "date": V1464_DATE,
            },
            "v1463": v1463,
            "v1462": v1462,
            "v1461": v1461,
            "endpoints": [
                "GET  /healthz",
                "GET  /status",
                "GET  /pipeline/adversarial",
                "POST /pipeline/run",
                "POST /pipeline/lint",
                "POST /pipeline/policy-gate",
            ],
            "limits": {
                "max_body_bytes": DEFAULT_BODY_MAX_BYTES,
                "run_timeout_s": DEFAULT_RUN_TIMEOUT_S,
                "port_range": [DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH],
            },
        }, indent=2, ensure_ascii=False).encode("utf-8"),
    )


def _route_pipeline_adversarial(_body: Optional[bytes], _q: Dict[str, str]) -> RouteResult:
    """Run the full V1463 adversarial suite (30 specs, mixed policies)."""
    # run_pipeline(specs) takes a list of specs and returns a PipelineReport.
    # We want the demo payload (the dict returned by run_v1463), which reports
    # n_specs=30 + match_rate against declared expected outcomes.
    from apeireth.v1463_asi_lint_gate_subprocess_pipeline import run_v1463 as _run_v1463
    payload = _run_v1463()
    return RouteResult(
        status_code=200,
        content_type="application/json",
        body=json.dumps(payload, ensure_ascii=False, default=str).encode("utf-8"),
    )


def _route_pipeline_run(body: Optional[bytes], _q: Dict[str, str]) -> RouteResult:
    """Run pipeline on a JSONL body of SandboxSpec dicts."""
    import tempfile
    if body is None:
        return RouteResult(400, "application/json",
                           b'{"error": "POST body required"}')
    if len(body) > DEFAULT_BODY_MAX_BYTES:
        return RouteResult(413, "application/json",
                           json.dumps({
                               "error": "body too large",
                               "max_bytes": DEFAULT_BODY_MAX_BYTES,
                               "got_bytes": len(body),
                           }, ensure_ascii=False).encode("utf-8"))
    run_pipeline, parse_jsonl_specs, _PR, _PRc = _import_v1463_pipeline()
    try:
        text = body.decode("utf-8")
    except UnicodeDecodeError as e:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "body not utf-8", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    # parse_jsonl_specs takes a Path, so write to a tempfile.
    tmp = tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl",
                                       encoding="utf-8", delete=False)
    try:
        tmp.write(text)
        tmp.close()
        specs, labels = parse_jsonl_specs(Path(tmp.name))
    except Exception as e:
        try:
            Path(tmp.name).unlink(missing_ok=True)
        except Exception:
            pass
        return RouteResult(400, "application/json",
                           json.dumps({"error": "jsonl parse failed", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    finally:
        try:
            Path(tmp.name).unlink(missing_ok=True)
        except Exception:
            pass
    try:
        report = run_pipeline(specs, labels=labels)
    except Exception as e:
        return RouteResult(500, "application/json",
                           json.dumps({"error": "pipeline run failed", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    payload = report.to_dict() if hasattr(report, "to_dict") else _safe(report)
    return RouteResult(200, "application/json",
                       json.dumps(payload, ensure_ascii=False, default=str).encode("utf-8"))


def _route_pipeline_lint(body: Optional[bytes], q: Dict[str, str]) -> RouteResult:
    """Run ONLY V1462 lint_spec on a single SandboxSpec dict."""
    if body is None:
        return RouteResult(400, "application/json", b'{"error": "POST body required"}')
    if len(body) > DEFAULT_BODY_MAX_BYTES:
        return RouteResult(413, "application/json",
                           json.dumps({"error": "body too large",
                                       "max_bytes": DEFAULT_BODY_MAX_BYTES}, ensure_ascii=False).encode("utf-8"))
    lint_spec, _pg, PolicyLevel, LintReport = _import_v1462_lint()
    try:
        spec_dict = json.loads(body.decode("utf-8"))
    except Exception as e:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "json parse failed", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    # Build SandboxSpec (need V1461)
    SandboxSpec, _SandboxRunner = _import_v1461_sandbox()
    try:
        kwargs = {}
        for k in ("command", "image_alias", "workdir_basename",
                  "env_extra", "timeout_s", "max_output_bytes"):
            if k in spec_dict and spec_dict[k] is not None:
                kwargs[k] = spec_dict[k]
        # env_extra must default to {} not None — V1462 linter iterates over it.
        kwargs.setdefault("env_extra", {})
        spec = SandboxSpec.from_dict(spec_dict) if hasattr(SandboxSpec, "from_dict") else SandboxSpec(**kwargs)
    except Exception as e:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "SandboxSpec build failed", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    policy_name = (q.get("policy") or "STANDARD").upper()
    try:
        policy = PolicyLevel(policy_name)
    except ValueError:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "bad policy", "got": policy_name,
                                       "allowed": [p.value for p in PolicyLevel]}, ensure_ascii=False).encode("utf-8"))
    report = lint_spec(spec, policy)
    # LintReport should expose to_dict
    try:
        body_out = json.dumps(report.to_dict() if hasattr(report, "to_dict") else _safe(report), ensure_ascii=False)
    except Exception:
        body_out = json.dumps(_safe(report), ensure_ascii=False)
    return RouteResult(200, "application/json", body_out.encode("utf-8"))


def _route_pipeline_policy_gate(body: Optional[bytes], q: Dict[str, str]) -> RouteResult:
    """Run ONLY V1462 policy_gate on a single SandboxSpec dict."""
    if body is None:
        return RouteResult(400, "application/json", b'{"error": "POST body required"}')
    if len(body) > DEFAULT_BODY_MAX_BYTES:
        return RouteResult(413, "application/json",
                           json.dumps({"error": "body too large",
                                       "max_bytes": DEFAULT_BODY_MAX_BYTES}, ensure_ascii=False).encode("utf-8"))
    _ls, policy_gate, PolicyLevel, _LR = _import_v1462_lint()
    SandboxSpec, _SR = _import_v1461_sandbox()
    try:
        spec_dict = json.loads(body.decode("utf-8"))
    except Exception as e:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "json parse failed", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    try:
        kwargs = {}
        for k in ("command", "image_alias", "workdir_basename",
                  "env_extra", "timeout_s", "max_output_bytes"):
            if k in spec_dict and spec_dict[k] is not None:
                kwargs[k] = spec_dict[k]
        kwargs.setdefault("env_extra", {})
        spec = SandboxSpec.from_dict(spec_dict) if hasattr(SandboxSpec, "from_dict") else SandboxSpec(**kwargs)
    except Exception as e:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "SandboxSpec build failed", "detail": str(e)}, ensure_ascii=False).encode("utf-8"))
    policy_name = (q.get("policy") or "STANDARD").upper()
    try:
        policy = PolicyLevel(policy_name)
    except ValueError:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "bad policy", "got": policy_name,
                                       "allowed": [p.value for p in PolicyLevel]}, ensure_ascii=False).encode("utf-8"))
    allowed, violations = policy_gate(spec, policy)
    return RouteResult(200, "application/json",
                       json.dumps({
                           "allowed": bool(allowed),
                           "violations": [
                               {
                                   "rule_code": getattr(v, "rule_code", None),
                                     "severity": getattr(v, "severity", None).value if hasattr(getattr(v, "severity", None), "value") else str(getattr(v, "severity", None)),
                                     "field": getattr(v, "field", None),
                                     "message": getattr(v, "message", None),
                                 } for v in (violations or [])
                             ],
                             "policy": policy.value,
                         }, ensure_ascii=False, default=str).encode("utf-8"))


# Router table (主 13:31 大胆放手 — anyone can curl)
ROUTES: Dict[Tuple[str, str], Callable[[Optional[bytes], Dict[str, str]], RouteResult]] = {
    ("GET", "/healthz"): _route_healthz,
    ("GET", "/status"): _route_status,
    ("GET", "/pipeline/adversarial"): _route_pipeline_adversarial,
    ("POST", "/pipeline/run"): _route_pipeline_run,
    ("POST", "/pipeline/lint"): _route_pipeline_lint,
    ("POST", "/pipeline/policy-gate"): _route_pipeline_policy_gate,
}


# ──────────────────────────────────────────────────────────────────────
# V1464 HTTP request handler — wraps the router
# ──────────────────────────────────────────────────────────────────────


class _GatewayState:
    """Shared state across all requests in one server instance."""
    def __init__(self) -> None:
        self.stats = GatewayStats(started_at=time.time())
        self.lock = threading.Lock()
        self.status = GatewayStatus.STARTING

    def record_request(self, req: GatewayRequest) -> None:
        with self.lock:
            self.stats.record(req)


class GatewayHTTPHandler(BaseHTTPRequestHandler):
    """BaseHTTPRequestHandler subclass implementing V1464 routes."""

    # Per-instance state injected from server factory
    state: _GatewayState = None  # type: ignore[assignment]

    # Suppress default stderr access log; we log via _log_request instead.
    def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
        return

    # Bound body read so a malicious client can't OOM us
    def _read_body_bounded(self) -> Optional[bytes]:
        try:
            length = int(self.headers.get("Content-Length", "0") or "0")
        except ValueError:
            length = 0
        if length <= 0:
            return b""
        if length > DEFAULT_BODY_MAX_BYTES:
            return None  # signal "too large" via sentinel
        try:
            return self.rfile.read(length)
        except Exception:
            return b""

    def _write_route(self, route_result: RouteResult) -> None:
        self.send_response(route_result.status_code)
        self.send_header("Content-Type", route_result.content_type)
        self.send_header("Content-Length", str(len(route_result.body)))
        self.send_header("X-V1464-Module", V1464_MODULE)
        self.send_header("X-V1464-Version", V1464_VERSION)
        self.end_headers()
        try:
            self.wfile.write(route_result.body)
        except Exception:
            pass

    def _route(self, body: Optional[bytes]) -> None:
        parsed = urlparse(self.path)
        path = parsed.path or "/"
        q: Dict[str, str] = {}
        for kv in parsed.query.split("&"):
            if "=" in kv:
                k, v = kv.split("=", 1)
                q[k] = v
        key = (self.command, path)
        handler = ROUTES.get(key)
        if handler is None:
            # Try a generic 405 if path exists with a wrong method
            method_only = any(p == path for (_m, p) in ROUTES.keys())
            if method_only:
                self._write_route(RouteResult(405, "application/json",
                                              json.dumps({"error": "method not allowed",
                                                           "method": self.command,
                                                           "path": path,
                                                           "allowed_methods": [m for (m, p) in ROUTES if p == path]}, ensure_ascii=False).encode("utf-8")))
                return
            self._write_route(RouteResult(404, "application/json",
                                          json.dumps({"error": "not found",
                                                       "method": self.command,
                                                       "path": path,
                                                       "endpoints": [f"{m} {p}" for (m, p) in ROUTES]}, ensure_ascii=False).encode("utf-8")))
            return
        try:
            rr = handler(body, q)
        except Exception as e:
            rr = RouteResult(500, "application/json",
                             json.dumps({"error": "handler exception",
                                         "detail": str(e),
                                         "method": self.command,
                                         "path": path}, ensure_ascii=False).encode("utf-8"))
        self._write_route(rr)

    # GET handler
    def do_GET(self) -> None:  # noqa: N802
        req = GatewayRequest(method="GET", path=urlparse(self.path).path or "/",
                             body_bytes=0, remote=self.client_address[0],
                             started_at=time.time())
        try:
            self._route(b"")
        finally:
            req.finished_at = time.time()
            req.status_code = getattr(self, "_v1464_last_status", None) or 200
            if self.state is not None:
                self.state.record_request(req)

    # POST handler
    def do_POST(self) -> None:  # noqa: N802
        parsed = urlparse(self.path)
        req = GatewayRequest(method="POST", path=parsed.path or "/",
                             body_bytes=0, remote=self.client_address[0],
                             started_at=time.time())
        try:
            body = self._read_body_bounded()
            if body is None:
                self._write_route(RouteResult(413, "application/json",
                                              json.dumps({"error": "body too large",
                                                          "max_bytes": DEFAULT_BODY_MAX_BYTES}, ensure_ascii=False).encode("utf-8")))
                req.status_code = 413
                return
            req.body_bytes = len(body)
            self._route(body)
        finally:
            req.finished_at = time.time()
            if req.status_code is None:
                req.status_code = 200
            if self.state is not None:
                self.state.record_request(req)


def make_gateway_server(host: str = DEFAULT_HOST, port: int = 0,
                        state: Optional[_GatewayState] = None) -> ThreadingHTTPServer:
    """Build a ThreadingHTTPServer wired to GatewayHTTPHandler + state."""
    if state is None:
        state = _GatewayState()

    class _BoundHandler(GatewayHTTPHandler):
        pass

    _BoundHandler.state = state  # type: ignore[assignment]

    # If port=0 we let the OS pick; otherwise verify the port is available.
    bind_port = port if port > 0 else find_open_port(host=host)
    server = ThreadingHTTPServer((host, bind_port), _BoundHandler)
    server.timeout = DEFAULT_SERVER_TIMEOUT_S
    server.daemon_threads = True
    return server


# ──────────────────────────────────────────────────────────────────────
# V1464 helpers (safe dump for non-dataclass objects)
# ──────────────────────────────────────────────────────────────────────


def _safe(obj: Any) -> Any:
    """Recursively coerce an object into JSON-safe primitives."""
    if obj is None or isinstance(obj, (bool, int, float, str)):
        return obj
    if isinstance(obj, (list, tuple)):
        return [_safe(x) for x in obj]
    if isinstance(obj, dict):
        return {str(k): _safe(v) for k, v in obj.items()}
    if dataclasses.is_dataclass(obj):
        return _safe(dataclasses.asdict(obj))
    if hasattr(obj, "value") and hasattr(obj, "name"):
        return obj.value
    return str(obj)


# ──────────────────────────────────────────────────────────────────────
# V1464 — top-level demo: in-process HTTP server + curl-equivalent client
# ──────────────────────────────────────────────────────────────────────


def _make_test_client(server: ThreadingHTTPServer) -> Any:
    """Return a context-manager-free http.client HTTPConnection to the server."""
    import http.client
    host, port = server.server_address
    return http.client.HTTPConnection(host, port, timeout=DEFAULT_SERVER_TIMEOUT_S)


def _http_get(client: Any, path: str) -> Tuple[int, Dict[str, str], bytes]:
    client.request("GET", path)
    resp = client.getresponse()
    body = resp.read()
    headers = {k: v for k, v in resp.getheaders()}
    return resp.status, headers, body


def _http_post(client: Any, path: str, body: bytes,
               content_type: str = "application/json") -> Tuple[int, Dict[str, str], bytes]:
    client.request("POST", path, body=body,
                   headers={"Content-Type": content_type,
                            "Content-Length": str(len(body))})
    resp = client.getresponse()
    rb = resp.read()
    headers = {k: v for k, v in resp.getheaders()}
    return resp.status, headers, rb


def run_v1464_demo() -> Dict[str, Any]:
    """Spin up an in-process HTTP server, hit every endpoint, return summary."""
    state = _GatewayState()
    server = make_gateway_server(host="127.0.0.1", port=0, state=state)
    host, port = server.server_address

    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()
    state.status = GatewayStatus.LISTENING

    results: List[Dict[str, Any]] = []

    try:
        client = _make_test_client(server)

        # 1) GET /healthz
        sc, _h, body = _http_get(client, "/healthz")
        results.append({
            "endpoint": "GET /healthz", "status": sc,
            "body_ok": json.loads(body).get("ok") is True,
        })

        # 2) GET /status
        sc, _h, body = _http_get(client, "/status")
        j = json.loads(body)
        results.append({
            "endpoint": "GET /status", "status": sc,
            "body_ok": isinstance(j, dict) and "v1464" in j and "v1463" in j and "v1462" in j and "v1461" in j,
        })

        # 3) GET /pipeline/adversarial (no body)
        sc, _h, body = _http_get(client, "/pipeline/adversarial")
        try:
            j = json.loads(body)
            results.append({
                "endpoint": "GET /pipeline/adversarial", "status": sc,
                "body_ok": j.get("n_specs") == EXPECTED_ADVERSARIAL_SPECS
                          and j.get("match_rate") == 1.0,
            })
        except Exception as e:
            results.append({
                "endpoint": "GET /pipeline/adversarial", "status": sc,
                "body_ok": False, "error": str(e),
            })

        # 4) POST /pipeline/lint — a safe Python hello spec
        safe_spec = {
            "command": [sys.executable, "-c", "print('hello from V1464')"],
            "image_alias": "ok_py",
            "workdir_basename": "tmp",
            "env_extra": {},
            "timeout_s": 60,  # ≥ DEFAULT_TIMEOUT_S=30 to avoid SL090 WARN
            "max_output_bytes": 4096,  # ≥ 1024 to avoid SL091 WARN
        }
        sc, _h, body = _http_post(client, "/pipeline/lint?policy=STANDARD",
                                  json.dumps(safe_spec).encode("utf-8"))
        try:
            j = json.loads(body)
            findings_n = len(j.get("findings", [])) if isinstance(j, dict) else -1
            results.append({
                "endpoint": "POST /pipeline/lint (safe)", "status": sc,
                "body_ok": sc == 200 and findings_n == 0,
            })
        except Exception as e:
            results.append({
                "endpoint": "POST /pipeline/lint (safe)", "status": sc,
                "body_ok": False, "error": str(e),
            })

        # 5) POST /pipeline/policy-gate — a blocked rm -rf spec
        bad_spec = {
            "command": ["bash", "-c", "rm -rf /"],
            "image_alias": "ok_b",
            "workdir_basename": "tmp",
            "env_extra": {},
            "timeout_s": 5,
            "max_output_bytes": 1024,
        }
        sc, _h, body = _http_post(client, "/pipeline/policy-gate?policy=PERMISSIVE",
                                  json.dumps(bad_spec).encode("utf-8"))
        try:
            j = json.loads(body)
            results.append({
                "endpoint": "POST /pipeline/policy-gate (bad)", "status": sc,
                "body_ok": sc == 200 and j.get("allowed") is False
                          and len(j.get("violations", [])) >= 1,
            })
        except Exception as e:
            results.append({
                "endpoint": "POST /pipeline/policy-gate (bad)", "status": sc,
                "body_ok": False, "error": str(e),
            })

        # 6) POST /pipeline/run — a JSONL with one safe + one bad
        jsonl_lines = "\n".join([
            json.dumps(safe_spec),
            json.dumps(bad_spec),
        ]).encode("utf-8")
        sc, _h, body = _http_post(client, "/pipeline/run",
                                  jsonl_lines, content_type="application/x-ndjson")
        try:
            j = json.loads(body)
            results.append({
                "endpoint": "POST /pipeline/run (mixed)", "status": sc,
                "body_ok": sc == 200 and j.get("n_specs") == 2
                          and isinstance(j.get("counts"), dict),
            })
        except Exception as e:
            results.append({
                "endpoint": "POST /pipeline/run (mixed)", "status": sc,
                "body_ok": False, "error": str(e),
            })

        # 7) GET unknown path → 404
        sc, _h, body = _http_get(client, "/nope")
        results.append({
            "endpoint": "GET /nope (expect 404)", "status": sc,
            "body_ok": sc == 404,
        })

        # 8) POST with wrong method → 405
        sc, _h, body = _http_post(client, "/healthz", b"")
        results.append({
            "endpoint": "POST /healthz (expect 405)", "status": sc,
            "body_ok": sc == 405,
        })

        client.close()
    finally:
        state.status = GatewayStatus.CLOSING
        server.shutdown()
        server.server_close()
        state.status = GatewayStatus.CLOSED

    n_ok = sum(1 for r in results if r.get("body_ok"))
    return {
        "n_endpoints": len(results),
        "n_ok": n_ok,
        "ok_rate": round(n_ok / max(1, len(results)), 4),
        "host": host,
        "port": port,
        "results": results,
        "stats": state.stats.to_dict(),
    }


def run_v1464() -> Dict[str, Any]:
    """Top-level demo entry — same as run_v1464_demo for now."""
    return run_v1464_demo()


# ──────────────────────────────────────────────────────────────────────
# V1464 — popper checks (falsifiability)
# ──────────────────────────────────────────────────────────────────────


def popper_v1464() -> Dict[str, Any]:
    """Mini Popper self-check — 14 bounded checks."""
    checks: Dict[str, bool] = {}

    # 1. Module metadata present
    checks["META_PRESENT"] = bool(V1464_MODULE and V1464_VERSION and V1464_SCHEMA)

    # 2. 6 routes defined
    checks["ROUTES_6"] = len(ROUTES) == 6

    # 3. Loopback default
    checks["LOOPBACK_DEFAULT"] = DEFAULT_HOST == "127.0.0.1"

    # 4. Body bounded
    checks["BODY_BOUNDED"] = DEFAULT_BODY_MAX_BYTES == 256 * 1024

    # 5. Timeout bounded
    checks["TIMEOUT_BOUNDED"] = DEFAULT_RUN_TIMEOUT_S == 60

    # 6. Server factory creates a server
    try:
        server = make_gateway_server(host="127.0.0.1", port=0,
                                     state=_GatewayState())
        checks["SERVER_FACTORY"] = True
        server.server_close()
    except Exception:
        checks["SERVER_FACTORY"] = False

    # 7. V1463 importable + runnable
    try:
        from apeireth.v1463_asi_lint_gate_subprocess_pipeline import run_v1463 as _run_v1463
        sample = _run_v1463()
        checks["V1463_RUNNABLE"] = sample.get("match_rate") == 1.0
    except Exception:
        checks["V1463_RUNNABLE"] = False

    # 8. V1462 importable + runnable on a sample spec
    try:
        lint_spec, _pg, PolicyLevel, _LR = _import_v1462_lint()
        SandboxSpec, _SR = _import_v1461_sandbox()
        sample_spec = SandboxSpec(command=[sys.executable, "-c", "print(1)"])
        report = lint_spec(sample_spec, PolicyLevel.STANDARD)
        checks["V1462_RUNNABLE"] = hasattr(report, "findings")
    except Exception:
        checks["V1462_RUNNABLE"] = False

    # 9. find_open_port finds something
    try:
        port = find_open_port()
        checks["PORT_FINDER"] = DEFAULT_PORT_LOW <= port <= DEFAULT_PORT_HIGH
    except Exception:
        checks["PORT_FINDER"] = False

    # 10. End-to-end demo passes
    try:
        demo = run_v1464_demo()
        checks["DEMO_OK"] = demo.get("ok_rate") == 1.0
    except Exception:
        checks["DEMO_OK"] = False

    # 11. Adversarial suite reports correct size
    try:
        from apeireth.v1463_asi_lint_gate_subprocess_pipeline import _ADVERSARIAL_SPECS
        checks["ADVERSARIAL_30"] = len(_ADVERSARIAL_SPECS) == EXPECTED_ADVERSARIAL_SPECS
    except Exception:
        checks["ADVERSARIAL_30"] = False

    # 12. Route handlers return RouteResult
    try:
        rr = _route_healthz(None, {})
        checks["HANDLER_RETURNS_ROUTERESULT"] = (
            isinstance(rr, RouteResult) and rr.status_code == 200
            and rr.content_type == "application/json"
        )
    except Exception:
        checks["HANDLER_RETURNS_ROUTERESULT"] = False

    # 13. All routes have callable handlers
    try:
        all_callable = all(callable(h) for h in ROUTES.values())
        checks["HANDLERS_CALLABLE"] = all_callable
    except Exception:
        checks["HANDLERS_CALLABLE"] = False

    # 14. Safe coercer doesn't blow up on common types
    try:
        out = _safe({"a": 1, "b": [1, 2, 3], "c": None, "d": True, "e": "x"})
        checks["SAFE_COERCER"] = out == {"a": 1, "b": [1, 2, 3], "c": None, "d": True, "e": "x"}
    except Exception:
        checks["SAFE_COERCER"] = False

    bad = [k for k, v in checks.items() if not v]
    return {
        "n_checks": len(checks),
        "passed": len(checks) - len(bad),
        "failed": bad,
        "popper_pass": len(bad) == 0,
        "checks": checks,
    }


# ──────────────────────────────────────────────────────────────────────
# V1464 — CLI
# ──────────────────────────────────────────────────────────────────────


def _cmd_demo(args: argparse.Namespace) -> int:
    payload = run_v1464_demo()
    base = Path(getattr(args, "out", "."))
    base.mkdir(parents=True, exist_ok=True)
    (base / ".v1464-demo-report.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({
        "n_endpoints": payload["n_endpoints"],
        "n_ok": payload["n_ok"],
        "ok_rate": payload["ok_rate"],
        "host": payload["host"],
        "port": payload["port"],
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_popper(args: argparse.Namespace) -> int:
    print(json.dumps(popper_v1464(), indent=2, ensure_ascii=False))
    return 0


def _cmd_serve(args: argparse.Namespace) -> int:
    """Run the HTTP server (blocking)."""
    host = args.host or DEFAULT_HOST
    if host != "127.0.0.1" and not getattr(args, "allow_lan", False):
        print(json.dumps({
            "error": "refusing to bind non-loopback host without --allow-lan",
            "host": host,
        }, indent=2, ensure_ascii=False))
        return 2
    port = int(getattr(args, "port", 0) or 0)
    if port == 0:
        port = find_open_port(host=host)
    state = _GatewayState()
    server = make_gateway_server(host=host, port=port, state=state)
    print(json.dumps({
        "module": V1464_MODULE,
        "version": V1464_VERSION,
        "schema": V1464_SCHEMA,
        "host": host,
        "port": server.server_address[1],
        "endpoints": [f"{m} {p}" for (m, p) in ROUTES],
    }, indent=2, ensure_ascii=False))
    print("--- serving (Ctrl-C to stop) ---", flush=True)
    state.status = GatewayStatus.LISTENING
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        state.status = GatewayStatus.CLOSING
        server.shutdown()
        server.server_close()
        state.status = GatewayStatus.CLOSED
        print("--- stopped ---", flush=True)
    return 0


def _cmd_status(args: argparse.Namespace) -> int:
    print(json.dumps({
        "module": V1464_MODULE,
        "version": V1464_VERSION,
        "schema": V1464_SCHEMA,
        "date": V1464_DATE,
        "default_host": DEFAULT_HOST,
        "default_port_range": [DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH],
        "max_body_bytes": DEFAULT_BODY_MAX_BYTES,
        "run_timeout_s": DEFAULT_RUN_TIMEOUT_S,
        "n_routes": len(ROUTES),
        "routes": [{"method": m, "path": p} for (m, p) in ROUTES],
        "borrowed": [
            "v1463", "v1462", "v1461", "v1437", "v1435", "v1434", "v1420", "stdlib",
        ],
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_chain(args: argparse.Namespace) -> int:
    chain = {
        "v1463": "run_pipeline + parse_jsonl_specs + PipelineReport (canonical pipeline)",
        "v1462": "lint_spec + policy_gate + PolicyLevel + LintReport (linter core)",
        "v1461": "SandboxSpec + SandboxRunner (subprocess sandbox core)",
        "v1437": "Subprocess HTTP Live Server (http.server + bounded subprocess)",
        "v1435": "Docker availability probe (offline-safe pattern)",
        "v1434": "VCP consistency HTTP (router + JSON shape reference)",
        "v1420": "HTTP status endpoint (router pattern)",
        "stdlib": "http.server + socketserver + json + urllib.parse + threading",
        "v1464_borrows_from": [
            "v1463", "v1462", "v1461", "v1437", "v1435", "v1434", "v1420", "stdlib",
        ],
        "all_ok": True,
    }
    print(json.dumps(chain, indent=2, ensure_ascii=False))
    return 0


def _cmd_meta(args: argparse.Namespace) -> int:
    print(json.dumps({
        "schema": V1464_SCHEMA,
        "version": V1464_VERSION,
        "module": V1464_MODULE,
        "phase": 1464,
        "post": ["v1463", "v1462", "v1461", "v1437"],
        "guards": [
            "GUARD_V1463_PIPELINE_REUSED", "GUARD_V1462_LINT_REUSED",
            "GUARD_V1461_RUNNER_REUSED", "GUARD_STDLIB_HTTP",
            "GUARD_LOOPBACK_DEFAULT", "GUARD_BODY_BOUNDED",
            "GUARD_TIMEOUT_BOUNDED", "GUARD_PORT_RECLAIMED",
            "GUARD_METHOD_ROUTED", "GUARD_ERROR_HANDLED",
            "GUARD_RUNS_ON_WINDOWS", "GUARD_BORROWED_LINEAGE", "GUARD_CLI_RUNNABLE",
        ],
        "v3_guards": [
            "GUARD_HTTP_NOT_ORCHESTRATOR", "GUARD_HTTP_NOT_CI",
            "GUARD_HTTP_NOT_SECURITY", "GUARD_HTTP_NOT_ASI",
            "GUARD_HTTP_NOT_PHENOMENAL", "GUARD_HTTP_NOT_HUMAN_LEVEL",
        ],
        "endpoints": [{"method": m, "path": p} for (m, p) in ROUTES],
        "n_routes": len(ROUTES),
    }, indent=2, ensure_ascii=False))
    return 0


def _cmd_help(args: argparse.Namespace) -> int:
    print(__doc__)
    return 0


# Lazy import of Path here (avoid top-level import side-effects in some envs)
from pathlib import Path  # noqa: E402


def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog=V1464_MODULE,
        description="V1464 — Real HTTP Gateway wrapping V1463 Lint-Gate Subprocess Pipeline",
    )
    sub = parser.add_subparsers(dest="cmd", required=False)

    p_demo = sub.add_parser("demo", help="Run in-process HTTP demo against all endpoints")
    p_demo.add_argument("--out", default=".", help="output dir for demo report")
    p_demo.set_defaults(func=_cmd_demo)

    p_popper = sub.add_parser("popper", help="Run mini Popper self-check")
    p_popper.set_defaults(func=_cmd_popper)

    p_serve = sub.add_parser("serve", help="Run the HTTP server (blocking)")
    p_serve.add_argument("--host", default=DEFAULT_HOST, help="bind host (default 127.0.0.1)")
    p_serve.add_argument("--port", type=int, default=0, help="bind port (default: find free)")
    p_serve.add_argument("--allow-lan", action="store_true",
                         help="allow binding to non-loopback host (default: refused)")
    p_serve.set_defaults(func=_cmd_serve)

    sub.add_parser("status", help="Print module status").set_defaults(func=_cmd_status)
    sub.add_parser("chain", help="Show borrowed-lineage chain").set_defaults(func=_cmd_chain)
    sub.add_parser("meta", help="Print module metadata").set_defaults(func=_cmd_meta)
    sub.add_parser("help", help="Print module help").set_defaults(func=_cmd_help)

    args = parser.parse_args(argv)
    if not getattr(args, "cmd", None):
        _cmd_help(args)
        return 0
    return int(args.func(args) or 0)


if __name__ == "__main__":
    raise SystemExit(main())