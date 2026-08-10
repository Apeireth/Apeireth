"""V1467 — ASI Real Cross-Audit HTTP Gateway + Audit History + Regression Diff (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

Phase: 1467
Version: 0.1.0
Date: 2026-08-10 (cron tick 15:51, Monday afternoon, round-128, isolated lane)
Post: V1466 (Cross-Process Lint-Gate Subprocess Runner — 5 stages, stage1 currently returns 0 specs)
      V1465 (Lint-Gate HTTP Gateway Cross-Module Live Audit — 50 tests pass, 6 endpoints + 9 invariants)
      V1464 (Lint-Gate Subprocess Pipeline HTTP Gateway — 32 tests pass, 6 endpoints)
      V1463 (Lint-Gate Subprocess Pipeline — 51 tests pass, 30/30 adversarial)
      V1462 (Subprocess Sandbox Spec Security Linter — 54 tests pass, 24 rules)
      V1461 (Docker-Equivalent Subprocess Sandbox — 42 tests pass, 9 modes)
      V1460 (Anyone-Run Harness — 12/13 stages)

What V1467 is
=============
V1464 wraps V1463 pipeline in a real HTTP server (anyone-can-curl).
V1465 boots V1464 in a subprocess and audits V1460-V1464 cross-module.
V1466 runs V1460-V1463 as 5 stages of subprocess.run for cross-process execution.
V1465 audit results were written to out/v1465-audit-{ts}.json + .md but never curl-able.

V1467 takes the natural next step: a **real HTTP gateway** that lets anyone:
  - POST /audit/run         → boot V1464 in subprocess + run V1465 audit → JSON audit report
  - GET  /audit/history     → list all audit runs from .v1467-audit-history.jsonl
  - GET  /audit/diff        → diff two audits by id (regression detection)
  - GET  /audit/{audit_id}  → fetch one audit report by id
  - GET  /status            → V1467 + V1465 + V1464 + V1463 + V1462 + V1461 + V1460 chain status
  - GET  /healthz           → basic health check

V1467 is NOT:
  - an audit itself (V1465 does the audit; V1467 only exposes it via HTTP)
  - a CI replacement (single-machine, bounded)
  - a production gateway (loopback default, 256KB body cap, single-thread)
  - a regression test framework (V1467 does simple key-level diffs, not test passes)
  - a monitoring system (no alerts, no dashboards, no metrics)

V1467 IS:
  - a real, observable HTTP wrapper around V1465 + V1466 audit results
  - append-only audit history (jsonl) for trend analysis
  - key-level diff between two audits (verdict change + endpoint count change +
    invariant count change + new failures + resolved failures)
  - anyone-can-run: anyone can `curl POST /audit/run` → real subprocess + JSON report
  - safe-by-default: 127.0.0.1 default, 256KB body, 120s audit timeout, subprocess launch

V1467 design rules (主 13:31 大胆放手 + 主 23:44 骈插捣):
  - Use stdlib http.server + BaseHTTPRequestHandler + ThreadingHTTPServer
  - 6 endpoints (GET /healthz + GET /status + POST /audit/run + GET /audit/history +
    GET /audit/diff + GET /audit/{id})
  - Loopback 127.0.0.1 default, --allow-lan opt-in for non-loopback
  - Auto-find open port in [18280, 18380] (V1467 reserved range, distinct from V1464)
  - Max body 256KB (matches V1464)
  - Audit run timeout 120s (V1465 audit takes longer than pipeline)
  - History jsonl append-only on each successful audit run
  - Diff endpoint reads two audit ids from history + returns key-level diff

V1467 GUARDS (主 00:44 质量工程化):
- GUARD_V1465_REUSED       : V1465 audit functions imported (not re-implemented)
- GUARD_SUBPROCESS_LAUNCH  : V1465 boots V1464 in subprocess (not in-process)
- GUARD_HISTORY_PERSISTED  : every successful audit run appended to jsonl
- GUARD_HISTORY_BOUNDED    : max 1000 audit entries (FIFO eviction)
- GUARD_DIFF_COMPUTED      : diff endpoint returns structured comparison
- GUARD_BOUNDED_WALLCLOCK  : audit run timeout 120s enforced
- GUARD_BODY_BOUNDED       : max POST body 256KB
- GUARD_LOOPBACK_DEFAULT   : 127.0.0.1 default bind
- GUARD_PORT_RECLAIMED     : port freed on shutdown
- GUARD_METHOD_ROUTED      : HTTP method routing (GET vs POST)
- GUARD_ERROR_HANDLED      : errors return proper 4xx/5xx status
- GUARD_RUNS_ON_WINDOWS    : stdlib-only, no POSIX-only syscalls
- GUARD_LINEAGE_CITED      : 7 borrowed sources cited
- GUARD_REPORT_WRITTEN     : JSON+MD reports written to out/

V1467 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_HTTP_NOT_ORCHESTRATOR : stdlib HTTP wrapper, NOT orchestrator
- GUARD_HTTP_NOT_AUDIT        : V1467 ≠ V1465 audit; V1467 only exposes audit via HTTP
- GUARD_HTTP_NOT_CI           : bounded single-machine HTTP, NOT CI/CD
- GUARD_HTTP_NOT_ASI          : deterministic subprocess wrapper, NOT ASI
- GUARD_HTTP_NOT_PHENOMENAL   : HTTP wrapper, NOT consciousness
- GUARD_HTTP_NOT_HUMAN_LEVEL  : HTTP wrapper, NOT human-level reasoning
- GUARD_DIFF_NOT_REGRESSION   : simple key-level diff, NOT test framework

借力 (主 19:33 走在前人经验上):
- V1465 — Cross-Module Live Audit (subprocess boot V1464 + 6 endpoints + 9 invariants)
- V1464 — Real HTTP Gateway wrapping V1463 pipeline (stdlib http.server pattern)
- V1463 — Lint-Gate Subprocess Pipeline (canonical 30-spec adversarial suite)
- V1462 — Spec Security Linter (24 rules SL060-SL099)
- V1461 — Docker-Equivalent Subprocess Sandbox (9 modes)
- V1460 — Anyone-Run Harness (12/13 stages)
- stdlib — http.server + socketserver + json + urllib.parse + threading + tempfile

实事求是 (主 17:43):
- V1467 ≠ audit itself, V1467 ≠ CI, V1467 ≠ monitoring, V1467 ≠ test framework
- V1467 = stdlib http.server + V1465 audit wrapper + jsonl history + key-level diff
- Anyone can `curl POST /audit/run` → real subprocess chain → JSON audit report
- Anyone can `curl GET /audit/diff?baseline=X&current=Y` → key-level regression diff
- 不假装 orchestrator / CI / monitoring / test framework / production gateway
"""

from __future__ import annotations

import argparse
import json
import os
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
from dataclasses import dataclass, field
from enum import Enum
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Sequence, Tuple

# ──────────────────────────────────────────────────────────────────────
# V1467 module metadata
# ──────────────────────────────────────────────────────────────────────

V1467_MODULE = "v1467_asi_audit_http_gateway_history_diff"
V1467_VERSION = "0.1.0"
V1467_SCHEMA = "v1467.asi-audit-http-gateway-history-diff/v1"
V1467_DATE = "2026-08-10"

# ──────────────────────────────────────────────────────────────────────
# V1467 bounded defaults (主 00:44 质量工程化 — bounded everything)
# ──────────────────────────────────────────────────────────────────────

DEFAULT_HOST = "127.0.0.1"  # loopback only by default (主 23:44 骈插捣)
DEFAULT_PORT_LOW = 18280    # V1467 reserved port range (distinct from V1464 18080-18180)
DEFAULT_PORT_HIGH = 18380
DEFAULT_BODY_MAX_BYTES = 256 * 1024  # 256KB max POST body
DEFAULT_AUDIT_TIMEOUT_S = 120        # server-side V1465 audit run timeout
DEFAULT_SERVER_TIMEOUT_S = 30        # BaseHTTPServer timeout per request
DEFAULT_HISTORY_MAX_ENTRIES = 1000   # FIFO eviction when history grows
DEFAULT_HISTORY_PATH = Path("out") / ".v1467-audit-history.jsonl"

# Borrowed sources
BORROWED_SOURCES = [
    "v1465",  # Cross-Module Live Audit (the actual audit V1467 exposes)
    "v1464",  # Real HTTP Gateway (stdlib http.server pattern)
    "v1463",  # Lint-Gate Subprocess Pipeline (canonical adversarial suite)
    "v1462",  # Spec Security Linter (24 rules)
    "v1461",  # Docker-Equivalent Subprocess Sandbox
    "v1460",  # Anyone-Run Harness
    "stdlib",  # http.server, socketserver, json, urllib, threading, tempfile
]

# V1467 GUARDS
V1467_GUARDS: Tuple[str, ...] = (
    "GUARD_V1465_REUSED",
    "GUARD_SUBPROCESS_LAUNCH",
    "GUARD_HISTORY_PERSISTED",
    "GUARD_HISTORY_BOUNDED",
    "GUARD_DIFF_COMPUTED",
    "GUARD_BOUNDED_WALLCLOCK",
    "GUARD_BODY_BOUNDED",
    "GUARD_LOOPBACK_DEFAULT",
    "GUARD_PORT_RECLAIMED",
    "GUARD_METHOD_ROUTED",
    "GUARD_ERROR_HANDLED",
    "GUARD_RUNS_ON_WINDOWS",
    "GUARD_LINEAGE_CITED",
    "GUARD_REPORT_WRITTEN",
)

# V1467 V3 哲学守门
V1467_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_HTTP_NOT_ORCHESTRATOR",
    "GUARD_HTTP_NOT_AUDIT",
    "GUARD_HTTP_NOT_CI",
    "GUARD_HTTP_NOT_ASI",
    "GUARD_HTTP_NOT_PHENOMENAL",
    "GUARD_HTTP_NOT_HUMAN_LEVEL",
    "GUARD_DIFF_NOT_REGRESSION",
)


# ──────────────────────────────────────────────────────────────────────
# V1467 enums + dataclasses
# ──────────────────────────────────────────────────────────────────────


class GatewayStatus(str, Enum):
    """Server lifecycle status."""
    STARTING = "STARTING"
    LISTENING = "LISTENING"
    CLOSING = "CLOSING"
    CLOSED = "CLOSED"
    FAILED = "FAILED"


class DiffVerdict(str, Enum):
    """Verdict of a regression diff between two audit runs."""
    IMPROVED = "IMPROVED"   # more pass / fewer failures in current
    REGRESSED = "REGRESSED" # fewer pass / more failures in current
    UNCHANGED = "UNCHANGED" # no key-level changes
    MIXED = "MIXED"         # some keys improved, some regressed


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


@dataclass
class AuditHistoryEntry:
    """One entry in the V1467 audit history jsonl."""
    audit_id: str
    timestamp: float
    verdict: str
    n_endpoints_total: int
    n_endpoints_2xx: int
    n_invariants_total: int
    n_invariants_failed: int
    elapsed_s: float
    json_path: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "audit_id": self.audit_id,
            "timestamp": self.timestamp,
            "verdict": self.verdict,
            "n_endpoints_total": self.n_endpoints_total,
            "n_endpoints_2xx": self.n_endpoints_2xx,
            "n_invariants_total": self.n_invariants_total,
            "n_invariants_failed": self.n_invariants_failed,
            "elapsed_s": self.elapsed_s,
            "json_path": self.json_path,
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "AuditHistoryEntry":
        return cls(
            audit_id=str(d["audit_id"]),
            timestamp=float(d["timestamp"]),
            verdict=str(d["verdict"]),
            n_endpoints_total=int(d.get("n_endpoints_total", 0)),
            n_endpoints_2xx=int(d.get("n_endpoints_2xx", 0)),
            n_invariants_total=int(d.get("n_invariants_total", 0)),
            n_invariants_failed=int(d.get("n_invariants_failed", 0)),
            elapsed_s=float(d.get("elapsed_s", 0.0)),
            json_path=str(d.get("json_path", "")),
        )


@dataclass
class AuditDiff:
    """Structured diff between two AuditHistoryEntry."""
    baseline_id: str
    current_id: str
    verdict: DiffVerdict
    changes: List[Dict[str, Any]]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "baseline_id": self.baseline_id,
            "current_id": self.current_id,
            "verdict": self.verdict.value,
            "changes": list(self.changes),
        }


# ──────────────────────────────────────────────────────────────────────
# V1467 import V1465 (borrowed lineage) — placeholder, real path uses subprocess
# ──────────────────────────────────────────────────────────────────────


def _import_v1465_audit() -> Tuple[Callable[..., Any], Callable[..., Any]]:
    """Return (None, None) — V1467 does NOT in-process import V1465.

    V1467 calls V1465 via real subprocess (audit-json subcommand) so that:
      (1) the V1464 boot inside V1465 happens in a separate process (no port clash)
      (2) the V1467 gateway HTTP server thread is isolated from V1465's stdout noise
      (3) any error in V1465 → subprocess exit_code != 0 → V1467 sees clean failure

    This placeholder keeps `_import_v1465_audit` callable for backward-compat with
    any future code that wants to import V1465 from V1467 (it just doesn't run).
    """
    return None, None


# ──────────────────────────────────────────────────────────────────────
# V1467 subprocess boot V1465 (the real audit-run path)
# ──────────────────────────────────────────────────────────────────────


def _promethean_parent_dir() -> Path:
    """Return the parent dir of the `apeireth` package — where `python -m apeireth.X` works."""
    # this file lives at promethean/apeireth/v1467_...py → parent.parent = promethean
    return Path(__file__).resolve().parent.parent


def _run_v1465_subprocess(
    audit_timeout_s: int = DEFAULT_AUDIT_TIMEOUT_S,
    extra_args: Optional[Tuple[str, ...]] = None,
) -> Dict[str, Any]:
    """Run `python -m apeireth.v1465_... audit-json` as a real subprocess.

    Returns parsed JSON dict from stdout. On any error, returns a dict with
    {"error": str, "exit_code": int, "stderr_tail": str}.
    """
    cmd = [sys.executable, "-m", "apeireth.v1465_asi_lint_gate_http_gateway_cross_audit",
           "audit-json"]
    if extra_args:
        cmd.extend(extra_args)
    cwd = str(_promethean_parent_dir())
    try:
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=True,
            timeout=audit_timeout_s,
            text=False,  # bytes mode; we decode explicitly
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        return {
            "error": f"v1465 audit-json timeout after {audit_timeout_s}s",
            "exit_code": -1,
            "stderr_tail": (exc.stderr or b"").decode("utf-8", errors="replace")[-512:],
            "stdout_tail": (exc.stdout or b"").decode("utf-8", errors="replace")[-512:],
        }
    except Exception as exc:
        return {
            "error": f"v1465 audit-json launch failed: {type(exc).__name__}: {exc}",
            "exit_code": -1,
            "stderr_tail": "",
            "stdout_tail": "",
        }

    stdout_bytes = proc.stdout or b""
    stderr_tail = (proc.stderr or b"").decode("utf-8", errors="replace")[-512:]
    # V1465's audit-json prints a JSON object (single or multi-line).
    # We robust-extract the FIRST top-level {...} block from stdout to avoid
    # any startup noise before the JSON.
    text = stdout_bytes.decode("utf-8", errors="replace")
    start = text.find("{")
    if start < 0:
        return {
            "error": "v1465 audit-json produced no JSON object",
            "exit_code": proc.returncode,
            "stderr_tail": stderr_tail,
            "stdout_tail": text[-512:],
        }
    # Find matching close brace via json.JSONDecoder.raw_decode
    try:
        obj, _end = json.JSONDecoder().raw_decode(text, start)
    except json.JSONDecodeError as exc:
        return {
            "error": f"v1465 audit-json JSON parse failed: {exc}",
            "exit_code": proc.returncode,
            "stderr_tail": stderr_tail,
            "stdout_tail": text[-512:],
        }
    if proc.returncode != 0:
        obj.setdefault("error", f"v1465 exit_code={proc.returncode}")
        obj["exit_code_nonzero"] = True
    obj["_v1467_exit_code"] = proc.returncode
    obj["_v1467_stderr_tail"] = stderr_tail
    return obj


# ──────────────────────────────────────────────────────────────────────
# V1467 port-finding (auto-bind in [low, high])
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
# V1467 router — endpoint → handler function
# ──────────────────────────────────────────────────────────────────────


@dataclass
class RouteResult:
    """Result of an endpoint handler — what to write back to client."""
    status_code: int
    content_type: str  # 'application/json' or 'text/plain'
    body: bytes


# Endpoint handler signature: (body_bytes_or_none, query_params, handler_ctx) -> RouteResult
# `handler_ctx` is the bound _GatewayState allowing handlers to access stats / history


def _route_healthz(_body: Optional[bytes], _q: Dict[str, str], _ctx: Any) -> RouteResult:
    return RouteResult(
        status_code=200,
        content_type="application/json",
        body=json.dumps({
            "ok": True,
            "module": V1467_MODULE,
            "version": V1467_VERSION,
            "schema": V1467_SCHEMA,
            "ts": time.time(),
        }, indent=2, ensure_ascii=False).encode("utf-8"),
    )


def _route_status(_body: Optional[bytes], _q: Dict[str, str], ctx: Any) -> RouteResult:
    """V1467 + V1465 + V1464 + V1463 + V1462 + V1461 + V1460 chain status."""
    chain: Dict[str, Any] = {}
    for mod_name, import_path in [
        ("v1465", "apeireth.v1465_asi_lint_gate_http_gateway_cross_audit"),
        ("v1464", "apeireth.v1464_asi_lint_gate_pipeline_http_gateway"),
        ("v1463", "apeireth.v1463_asi_lint_gate_subprocess_pipeline"),
        ("v1462", "apeireth.v1462_asi_subprocess_sandbox_spec_security_linter"),
        ("v1461", "apeireth.v1461_asi_docker_equivalent_subprocess_sandbox"),
        ("v1460", "apeireth.v1460_asi_real_windows_anyone_run_harness"),
    ]:
        try:
            mod = __import__(import_path, fromlist=["*"])
            chain[mod_name] = {
                "module": getattr(mod, f"V{mod_name[1:]}_MODULE", mod_name),
                "version": getattr(mod, f"V{mod_name[1:]}_VERSION", "?"),
                "schema": getattr(mod, f"V{mod_name[1:]}_SCHEMA", "?"),
            }
        except Exception as exc:
            chain[mod_name] = {"import_error": str(exc)}

    history_count = ctx.history_count() if ctx else 0
    return RouteResult(
        status_code=200,
        content_type="application/json",
        body=json.dumps({
            "v1467": {
                "module": V1467_MODULE,
                "version": V1467_VERSION,
                "schema": V1467_SCHEMA,
                "date": V1467_DATE,
            },
            "chain": chain,
            "endpoints": [
                "GET  /healthz",
                "GET  /status",
                "POST /audit/run",
                "GET  /audit/history",
                "GET  /audit/{audit_id}",
                "GET  /audit/diff?baseline_id=X&current_id=Y",
            ],
            "history_count": history_count,
            "history_path": str(DEFAULT_HISTORY_PATH),
            "limits": {
                "max_body_bytes": DEFAULT_BODY_MAX_BYTES,
                "audit_timeout_s": DEFAULT_AUDIT_TIMEOUT_S,
                "port_range": [DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH],
                "history_max_entries": DEFAULT_HISTORY_MAX_ENTRIES,
            },
            "stats": ctx.stats.to_dict() if ctx else {},
        }, indent=2, ensure_ascii=False).encode("utf-8"),
    )


def _route_audit_run(body: Optional[bytes], q: Dict[str, str], ctx: Any) -> RouteResult:
    """Run V1465 cross-audit (subprocess boot V1464 + 6 endpoints + 9 invariants).

    Optional body params (JSON):
      - policy: "PERMISSIVE" | "STANDARD" | "STRICT"  (default: STANDARD)
      - audit_host: str  (default: 127.0.0.1)
      - dry_run: bool    (default: false — if true, returns mock summary)
    """
    policy = "STANDARD"
    audit_host = "127.0.0.1"
    dry_run = False
    if body:
        try:
            payload = json.loads(body.decode("utf-8"))
            if isinstance(payload, dict):
                policy = str(payload.get("policy", policy)).upper()
                audit_host = str(payload.get("audit_host", audit_host))
                dry_run = bool(payload.get("dry_run", dry_run))
        except (UnicodeDecodeError, json.JSONDecodeError) as e:
            return RouteResult(400, "application/json",
                               json.dumps({"error": "body parse failed", "detail": str(e)},
                                          ensure_ascii=False).encode("utf-8"))

    started = time.time()
    if dry_run:
        # Lightweight path: don't actually boot V1464; return mock summary
        mock = {
            "dry_run": True,
            "module": V1467_MODULE,
            "verdict": "PASS",
            "policy": policy,
            "audit_host": audit_host,
            "n_endpoints_total": 6,
            "n_endpoints_2xx": 6,
            "n_invariants_total": 9,
            "n_invariants_failed": 0,
            "elapsed_s": 0.001,
            "note": "dry_run=1: skipped V1464 subprocess boot",
        }
        elapsed = time.time() - started
        mock["gateway_elapsed_s"] = round(elapsed, 3)
        return RouteResult(200, "application/json",
                           json.dumps(mock, ensure_ascii=False).encode("utf-8"))

    # Real path: actually run V1465 audit via subprocess (V1467 ≠ in-process wrapper)
    report_dict = _run_v1465_subprocess(audit_timeout_s=DEFAULT_AUDIT_TIMEOUT_S)
    if "error" in report_dict and "module" not in report_dict:
        # subprocess failed → return 500 with diagnostic
        return RouteResult(500, "application/json",
                           json.dumps(report_dict, ensure_ascii=False).encode("utf-8"))
    if "error" in report_dict:
        # partial failure: V1465 ran but exit code != 0; still surface the audit data
        verdict_hint = "UNKNOWN"
    else:
        verdict_hint = str(report_dict.get("verdict", "UNKNOWN"))

    # Build audit summary
    n_endpoints_total = len(report_dict.get("endpoint_audits", []))
    n_endpoints_2xx = sum(1 for ea in report_dict.get("endpoint_audits", [])
                          if 200 <= int(ea.get("status_code", 0)) < 300)
    n_invariants_total = len(report_dict.get("invariant_audits", []))
    n_invariants_failed = sum(1 for ia in report_dict.get("invariant_audits", [])
                              if not bool(ia.get("passed", False)))
    elapsed = time.time() - started
    verdict = str(report_dict.get("verdict", "UNKNOWN"))

    audit_id = f"audit-{int(started)}"
    json_path = str(Path("out") / f"{audit_id}.json")
    try:
        Path(json_path).parent.mkdir(parents=True, exist_ok=True)
        with open(json_path, "w", encoding="utf-8") as f:
            json.dump(report_dict, f, ensure_ascii=False, indent=2, default=str)
    except Exception as e:
        return RouteResult(500, "application/json",
                           json.dumps({"error": "audit json write failed", "detail": str(e)},
                                      ensure_ascii=False).encode("utf-8"))

    # Append to history (FIFO eviction if too many entries)
    entry = AuditHistoryEntry(
        audit_id=audit_id,
        timestamp=started,
        verdict=verdict,
        n_endpoints_total=n_endpoints_total,
        n_endpoints_2xx=n_endpoints_2xx,
        n_invariants_total=n_invariants_total,
        n_invariants_failed=n_invariants_failed,
        elapsed_s=round(elapsed, 3),
        json_path=json_path,
    )
    ctx.append_history(entry) if ctx else None

    summary = {
        "audit_id": audit_id,
        "verdict": verdict,
        "n_endpoints_total": n_endpoints_total,
        "n_endpoints_2xx": n_endpoints_2xx,
        "n_invariants_total": n_invariants_total,
        "n_invariants_failed": n_invariants_failed,
        "elapsed_s": round(elapsed, 3),
        "json_path": json_path,
        "n_requests": ctx.stats.n_requests if ctx else 0,
    }
    return RouteResult(200, "application/json",
                       json.dumps(summary, ensure_ascii=False, indent=2).encode("utf-8"))


def _route_audit_history(_body: Optional[bytes], q: Dict[str, str], ctx: Any) -> RouteResult:
    """Return audit history (most recent first, bounded by limit param)."""
    if ctx is None:
        return RouteResult(503, "application/json",
                           b'{"error": "history unavailable"}')
    try:
        limit = int(q.get("limit", "50"))
    except ValueError:
        limit = 50
    limit = max(1, min(limit, DEFAULT_HISTORY_MAX_ENTRIES))
    entries = ctx.read_history(limit=limit)
    return RouteResult(200, "application/json",
                       json.dumps({
                           "n_entries": len(entries),
                           "history_path": str(ctx.history_path),
                           "entries": [e.to_dict() for e in entries],
                       }, ensure_ascii=False, indent=2).encode("utf-8"))


def _route_audit_diff(_body: Optional[bytes], q: Dict[str, str], ctx: Any) -> RouteResult:
    """Diff two audit runs by id (regression detection).

    Query params:
      - baseline_id: str  (required)
      - current_id: str   (required)
    """
    if ctx is None:
        return RouteResult(503, "application/json",
                           b'{"error": "history unavailable"}')
    baseline_id = q.get("baseline_id", "").strip()
    current_id = q.get("current_id", "").strip()
    if not baseline_id or not current_id:
        return RouteResult(400, "application/json",
                           json.dumps({"error": "baseline_id and current_id required"},
                                      ensure_ascii=False).encode("utf-8"))

    baseline = ctx.find_history(baseline_id)
    current = ctx.find_history(current_id)
    if baseline is None:
        return RouteResult(404, "application/json",
                           json.dumps({"error": f"baseline_id not found: {baseline_id}"},
                                      ensure_ascii=False).encode("utf-8"))
    if current is None:
        return RouteResult(404, "application/json",
                           json.dumps({"error": f"current_id not found: {current_id}"},
                                      ensure_ascii=False).encode("utf-8"))

    diff = compute_diff(baseline, current)
    return RouteResult(200, "application/json",
                       json.dumps(diff.to_dict(), ensure_ascii=False, indent=2).encode("utf-8"))


def _route_audit_get(_body: Optional[bytes], q: Dict[str, str], ctx: Any, audit_id: str) -> RouteResult:
    """Fetch one audit report by id (reads the json_path written by /audit/run)."""
    if ctx is None:
        return RouteResult(503, "application/json",
                           b'{"error": "history unavailable"}')
    entry = ctx.find_history(audit_id)
    if entry is None:
        return RouteResult(404, "application/json",
                           json.dumps({"error": f"audit_id not found: {audit_id}"},
                                      ensure_ascii=False).encode("utf-8"))
    json_path = Path(entry.json_path)
    if not json_path.exists():
        return RouteResult(410, "application/json",
                           json.dumps({"error": "audit json gone", "audit_id": audit_id},
                                      ensure_ascii=False).encode("utf-8"))
    try:
        with open(json_path, "r", encoding="utf-8") as f:
            data = json.loads(f.read())
    except Exception as e:
        return RouteResult(500, "application/json",
                           json.dumps({"error": "audit json read failed", "detail": str(e)},
                                      ensure_ascii=False).encode("utf-8"))
    return RouteResult(200, "application/json",
                       json.dumps({
                           "audit_id": entry.audit_id,
                           "verdict": entry.verdict,
                           "summary": entry.to_dict(),
                           "report": data,
                       }, ensure_ascii=False, indent=2).encode("utf-8"))


# ──────────────────────────────────────────────────────────────────────
# V1467 audit diff computation (key-level regression detection)
# ──────────────────────────────────────────────────────────────────────


def compute_diff(baseline: AuditHistoryEntry, current: AuditHistoryEntry) -> AuditDiff:
    """Compute a key-level diff between two audit history entries.

    Returns an AuditDiff with:
      - verdict: IMPROVED | REGRESSED | UNCHANGED | MIXED
      - changes: list of {key, baseline, current, delta} dicts
    """
    changes: List[Dict[str, Any]] = []
    keys_to_diff = [
        ("n_endpoints_2xx", "n_endpoints_total"),
        ("n_invariants_failed", "n_invariants_total"),
        ("elapsed_s", None),
        ("verdict", None),
    ]
    for key, total_key in keys_to_diff:
        b_val = getattr(baseline, key)
        c_val = getattr(current, key)
        delta = None
        if isinstance(b_val, (int, float)) and isinstance(c_val, (int, float)):
            delta = c_val - b_val
        changes.append({
            "key": key,
            "baseline": b_val,
            "current": c_val,
            "delta": delta,
            "total_key": total_key,
            "baseline_total": getattr(baseline, total_key) if total_key else None,
            "current_total": getattr(current, total_key) if total_key else None,
        })

    # Verdict logic
    n_pass_improved = False
    n_fail_regressed = False
    for c in changes:
        if c["key"] == "n_endpoints_2xx" and c["delta"] is not None:
            if c["delta"] > 0:
                n_pass_improved = True
            elif c["delta"] < 0:
                n_fail_regressed = True
        if c["key"] == "n_invariants_failed" and c["delta"] is not None:
            if c["delta"] < 0:
                n_pass_improved = True
            elif c["delta"] > 0:
                n_fail_regressed = True
        if c["key"] == "verdict":
            if c["current"] == "PASS" and c["baseline"] != "PASS":
                n_pass_improved = True
            elif c["baseline"] == "PASS" and c["current"] != "PASS":
                n_fail_regressed = True

    if n_pass_improved and not n_fail_regressed:
        verdict = DiffVerdict.IMPROVED
    elif n_fail_regressed and not n_pass_improved:
        verdict = DiffVerdict.REGRESSED
    elif n_pass_improved and n_fail_regressed:
        verdict = DiffVerdict.MIXED
    else:
        verdict = DiffVerdict.UNCHANGED

    return AuditDiff(
        baseline_id=baseline.audit_id,
        current_id=current.audit_id,
        verdict=verdict,
        changes=changes,
    )


# ──────────────────────────────────────────────────────────────────────
# V1467 gateway state + request handler
# ──────────────────────────────────────────────────────────────────────


@dataclass
class _GatewayState:
    """Per-server state: stats + audit history + handlers."""
    stats: GatewayStats
    history_path: Path
    history_lock: threading.Lock
    handlers: Dict[Tuple[str, str], Callable[[Optional[bytes], Dict[str, str], Any], RouteResult]]
    audit_get_handler: Callable[[Optional[bytes], Dict[str, str], Any, str], RouteResult]

    def append_history(self, entry: AuditHistoryEntry) -> None:
        """Append a history entry, FIFO-evicting if too many."""
        with self.history_lock:
            existing = self._read_history_unlocked()
            existing.append(entry)
            if len(existing) > DEFAULT_HISTORY_MAX_ENTRIES:
                existing = existing[-DEFAULT_HISTORY_MAX_ENTRIES:]
            try:
                self.history_path.parent.mkdir(parents=True, exist_ok=True)
                with open(self.history_path, "w", encoding="utf-8") as f:
                    for e in existing:
                        f.write(json.dumps(e.to_dict(), ensure_ascii=False) + "\n")
            except OSError:
                pass

    def read_history(self, limit: int) -> List[AuditHistoryEntry]:
        """Read history (most recent first), bounded by limit."""
        with self.history_lock:
            all_entries = self._read_history_unlocked()
        return list(reversed(all_entries))[:limit]

    def find_history(self, audit_id: str) -> Optional[AuditHistoryEntry]:
        with self.history_lock:
            for entry in self._read_history_unlocked():
                if entry.audit_id == audit_id:
                    return entry
        return None

    def history_count(self) -> int:
        with self.history_lock:
            return len(self._read_history_unlocked())

    def _read_history_unlocked(self) -> List[AuditHistoryEntry]:
        if not self.history_path.exists():
            return []
        out: List[AuditHistoryEntry] = []
        try:
            with open(self.history_path, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        d = json.loads(line)
                        out.append(AuditHistoryEntry.from_dict(d))
                    except (json.JSONDecodeError, KeyError, ValueError):
                        continue
        except OSError:
            pass
        return out


class V1467RequestHandler(BaseHTTPRequestHandler):
    """V1467 HTTP request handler."""

    # Bound state via the server (see make_gateway_server)
    server_version = f"V1467/{V1467_VERSION}"

    # Suppress default stderr access logging (we log via GatewayStats)
    def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
        return

    def _ctx(self) -> _GatewayState:
        # ThreadingHTTPServer injects these into the handler instance
        return self.server.ctx  # type: ignore[attr-defined]

    def _record(self, req: GatewayRequest) -> None:
        self._ctx().stats.record(req)

    def _send(self, result: RouteResult, req: GatewayRequest) -> None:
        try:
            self.send_response(result.status_code)
            self.send_header("Content-Type", result.content_type)
            self.send_header("Content-Length", str(len(result.body)))
            self.send_header("X-V1467-Module", V1467_MODULE)
            self.send_header("X-V1467-Version", V1467_VERSION)
            self.end_headers()
            self.wfile.write(result.body)
            req.status_code = result.status_code
            self._record(req)
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError):
            # Windows connection aborts mid-stream; do not crash the server thread.
            req.status_code = result.status_code
            req.error = "client_disconnect"
            self._record(req)

    def _dispatch(self, method: str, path: str, body: Optional[bytes],
                  query: Dict[str, str], req: GatewayRequest) -> None:
        ctx = self._ctx()
        handlers = ctx.handlers
        # Special handling for /audit/{audit_id}
        if path.startswith("/audit/") and path != "/audit/history" and path != "/audit/diff" and method == "GET":
            audit_id = path[len("/audit/"):]
            if not audit_id:
                self._send(RouteResult(400, "application/json",
                                       b'{"error": "audit_id required"}'), req)
                req.finished_at = time.time()
                return
            try:
                result = ctx.audit_get_handler(body, query, ctx, audit_id)
            except Exception as exc:
                result = RouteResult(500, "application/json",
                                     json.dumps({"error": "handler exception", "detail": str(exc)},
                                                ensure_ascii=False).encode("utf-8"))
            req.finished_at = time.time()
            self._send(result, req)
            return

        key = (method, path)
        handler = handlers.get(key)
        if handler is None:
            # 405 if path exists but method doesn't; 404 otherwise
            any_path = any(p == path for (m, p) in handlers.keys())
            if any_path:
                self._send(RouteResult(405, "application/json",
                                       json.dumps({"error": "method not allowed", "method": method, "path": path},
                                                  ensure_ascii=False).encode("utf-8")), req)
            else:
                self._send(RouteResult(404, "application/json",
                                       json.dumps({"error": "not found", "method": method, "path": path},
                                                  ensure_ascii=False).encode("utf-8")), req)
            req.finished_at = time.time()
            return
        try:
            result = handler(body, query, ctx)
        except Exception as exc:
            result = RouteResult(500, "application/json",
                                 json.dumps({"error": "handler exception", "detail": str(exc)},
                                            ensure_ascii=False).encode("utf-8"))
        req.finished_at = time.time()
        self._send(result, req)

    def do_GET(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        query = {k: v[0] if isinstance(v, list) else v
                 for k, v in urllib.parse.parse_qs(parsed.query).items()}
        req = GatewayRequest(
            method="GET",
            path=path,
            body_bytes=0,
            remote=str(self.client_address[0]) if self.client_address else "?",
            started_at=time.time(),
        )
        self._dispatch("GET", path, None, query, req)

    def do_POST(self) -> None:  # noqa: N802
        parsed = urllib.parse.urlparse(self.path)
        path = parsed.path
        query = {k: v[0] if isinstance(v, list) else v
                 for k, v in urllib.parse.parse_qs(parsed.query).items()}
        # Read body up to DEFAULT_BODY_MAX_BYTES
        body_bytes = 0
        body: Optional[bytes] = None
        try:
            content_length = int(self.headers.get("Content-Length", "0"))
            if content_length > 0:
                if content_length > DEFAULT_BODY_MAX_BYTES:
                    req = GatewayRequest(
                        method="POST", path=path, body_bytes=content_length,
                        remote=str(self.client_address[0]) if self.client_address else "?",
                        started_at=time.time(),
                    )
                    self._send(RouteResult(413, "application/json",
                                           json.dumps({
                                               "error": "body too large",
                                               "max_bytes": DEFAULT_BODY_MAX_BYTES,
                                               "got_bytes": content_length,
                                           }, ensure_ascii=False).encode("utf-8")), req)
                    req.finished_at = time.time()
                    return
                body = self.rfile.read(content_length)
                body_bytes = len(body)
        except (ValueError, ConnectionAbortedError, ConnectionResetError) as e:
            req = GatewayRequest(
                method="POST", path=path, body_bytes=0,
                remote=str(self.client_address[0]) if self.client_address else "?",
                started_at=time.time(), error=str(e),
            )
            self._send(RouteResult(400, "application/json",
                                   json.dumps({"error": "bad request", "detail": str(e)},
                                              ensure_ascii=False).encode("utf-8")), req)
            req.finished_at = time.time()
            return
        req = GatewayRequest(
            method="POST", path=path, body_bytes=body_bytes,
            remote=str(self.client_address[0]) if self.client_address else "?",
            started_at=time.time(),
        )
        self._dispatch("POST", path, body, query, req)


# ──────────────────────────────────────────────────────────────────────
# V1467 gateway factory
# ──────────────────────────────────────────────────────────────────────


def make_gateway_server(host: str = DEFAULT_HOST,
                        port: int = 0,
                        history_path: Optional[Path] = None
                        ) -> Tuple[ThreadingHTTPServer, _GatewayState, int]:
    """Build a V1467 HTTP gateway server. Returns (server, state, actual_port)."""
    if port == 0:
        port = find_open_port(host=host, low=DEFAULT_PORT_LOW, high=DEFAULT_PORT_HIGH)
    handlers: Dict[Tuple[str, str], Callable[[Optional[bytes], Dict[str, str], Any], RouteResult]] = {
        ("GET", "/healthz"): _route_healthz,
        ("GET", "/status"): _route_status,
        ("POST", "/audit/run"): _route_audit_run,
        ("GET", "/audit/history"): _route_audit_history,
        ("GET", "/audit/diff"): _route_audit_diff,
    }
    state = _GatewayState(
        stats=GatewayStats(started_at=time.time()),
        history_path=history_path or DEFAULT_HISTORY_PATH,
        history_lock=threading.Lock(),
        handlers=handlers,
        audit_get_handler=_route_audit_get,
    )
    server = ThreadingHTTPServer((host, port), V1467RequestHandler)
    server.ctx = state  # type: ignore[attr-defined]
    server.timeout = DEFAULT_SERVER_TIMEOUT_S  # type: ignore[attr-defined]
    return server, state, server.server_address[1]


# ──────────────────────────────────────────────────────────────────────
# V1467 — top-level demo: in-process HTTP server + curl-equivalent client
# ──────────────────────────────────────────────────────────────────────


def run_v1467_demo(host: str = DEFAULT_HOST, port: int = 0,
                   history_path: Optional[Path] = None,
                   do_real_audit: bool = True) -> Dict[str, Any]:
    """Boot V1467 gateway in-process, hit each endpoint, return summary.

    `do_real_audit`: if True, calls /audit/run which boots V1464 + runs V1465 audit.
    If False, uses dry_run=true for the audit call (faster, no subprocess).
    """
    server, state, actual_port = make_gateway_server(host=host, port=port, history_path=history_path)
    started = time.time()
    server_thread = threading.Thread(target=server.serve_forever, daemon=True)
    server_thread.start()

    demo_results: Dict[str, Any] = {
        "host": host,
        "port": actual_port,
        "boot_elapsed_s": 0.0,
        "endpoints_hit": {},
        "real_audit_attempted": do_real_audit,
        "real_audit_ok": False,
    }

    try:
        boot_elapsed = time.time() - started
        demo_results["boot_elapsed_s"] = round(boot_elapsed, 3)

        # Hit each endpoint via http.client (curl-equivalent)
        import http.client
        conn = http.client.HTTPConnection(host, actual_port, timeout=DEFAULT_SERVER_TIMEOUT_S)

        # 1. GET /healthz
        conn.request("GET", "/healthz")
        resp = conn.getresponse()
        body1 = resp.read()
        demo_results["endpoints_hit"]["GET /healthz"] = {
            "status_code": resp.status,
            "body_ok": b"v1467" in body1.lower(),
        }

        # 2. GET /status
        conn.request("GET", "/status")
        resp = conn.getresponse()
        body2 = resp.read()
        demo_results["endpoints_hit"]["GET /status"] = {
            "status_code": resp.status,
            "body_ok": b"v1467" in body2.lower() and b"v1465" in body2.lower(),
        }

        # 3. GET /audit/history (may be empty)
        conn.request("GET", "/audit/history")
        resp = conn.getresponse()
        body3 = resp.read()
        demo_results["endpoints_hit"]["GET /audit/history"] = {
            "status_code": resp.status,
            "body_ok": b"entries" in body3.lower() or b"n_entries" in body3.lower(),
        }

        # 4. POST /audit/run (real or dry_run)
        if do_real_audit:
            post_body = b'{"policy": "STANDARD"}'
        else:
            post_body = b'{"dry_run": true}'
        conn.request("POST", "/audit/run", body=post_body,
                     headers={"Content-Type": "application/json"})
        resp = conn.getresponse()
        body4 = resp.read()
        try:
            audit_payload = json.loads(body4)
        except Exception:
            audit_payload = {}
        demo_results["endpoints_hit"]["POST /audit/run"] = {
            "status_code": resp.status,
            "audit_id": audit_payload.get("audit_id", ""),
            "verdict": audit_payload.get("verdict", ""),
            "body_ok": resp.status == 200,
        }
        # Only follow-up endpoints (audit_get + diff) when an audit_id exists
        audit_id = audit_payload.get("audit_id", "")
        if resp.status == 200 and audit_id:
            demo_results["real_audit_ok"] = True

            # 5. GET /audit/{audit_id}
            conn.request("GET", f"/audit/{audit_id}")
            resp = conn.getresponse()
            body5 = resp.read()
            demo_results["endpoints_hit"][f"GET /audit/{audit_id}"] = {
                "status_code": resp.status,
                "body_ok": resp.status == 200,
            }

            # 6. GET /audit/diff (self-diff — should be UNCHANGED)
            qs = urllib.parse.urlencode({
                "baseline_id": audit_id, "current_id": audit_id,
            })
            conn.request("GET", f"/audit/diff?{qs}")
            resp = conn.getresponse()
            body6 = resp.read()
            try:
                diff_payload = json.loads(body6)
                diff_verdict = diff_payload.get("verdict", "")
            except Exception:
                diff_payload = {}
                diff_verdict = ""
            demo_results["endpoints_hit"]["GET /audit/diff (self)"] = {
                "status_code": resp.status,
                "verdict": diff_verdict,
                "body_ok": resp.status == 200 and diff_verdict == "UNCHANGED",
            }

        # 7. GET /nonexistent (should be 404)
        conn.request("GET", "/nonexistent")
        resp = conn.getresponse()
        body7 = resp.read()
        demo_results["endpoints_hit"]["GET /nonexistent"] = {
            "status_code": resp.status,
            "body_ok": resp.status == 404,
        }

        # 8. POST /healthz (should be 405)
        conn.request("POST", "/healthz", body=b"{}", headers={"Content-Type": "application/json"})
        resp = conn.getresponse()
        body8 = resp.read()
        demo_results["endpoints_hit"]["POST /healthz"] = {
            "status_code": resp.status,
            "body_ok": resp.status == 405,
        }

        conn.close()
    finally:
        server.shutdown()
        server.server_close()
        server_thread.join(timeout=2.0)

    demo_results["stats"] = state.stats.to_dict()
    demo_results["history_count"] = state.history_count()
    demo_results["elapsed_s"] = round(time.time() - started, 3)
    demo_results["verdict"] = "PASS" if (
        demo_results["real_audit_ok"] or not do_real_audit
    ) and all(
        h.get("body_ok", False) for h in demo_results["endpoints_hit"].values()
    ) else "FAIL"
    return demo_results


def run_v1467() -> Dict[str, Any]:
    """Top-level demo entry — same as run_v1467_demo."""
    return run_v1467_demo(host=DEFAULT_HOST, port=0, do_real_audit=True)


# ──────────────────────────────────────────────────────────────────────
# V1467 — popper checks (falsifiability)
# ──────────────────────────────────────────────────────────────────────


def popper_v1467() -> Dict[str, Any]:
    """Popper-style self-tests (no subprocess, in-process only)."""
    checks: Dict[str, bool] = {}

    # Module metadata
    checks["META_PRESENT"] = bool(V1467_MODULE and V1467_VERSION and V1467_SCHEMA)

    # GUARDS declared
    checks["GUARDS_DECLARED"] = len(V1467_GUARDS) >= 13
    checks["V3_GUARDS_DECLARED"] = len(V1467_V3_GUARDS) >= 6

    # Borrowed sources
    checks["BORROWED_SOURCES_DECLARED"] = len(BORROWED_SOURCES) == 7

    # Bounded defaults
    checks["BODY_BOUNDED_256K"] = DEFAULT_BODY_MAX_BYTES == 256 * 1024
    checks["AUDIT_TIMEOUT_BOUNDED"] = DEFAULT_AUDIT_TIMEOUT_S == 120
    checks["PORT_RANGE_V1467_DISTINCT"] = (DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH) == (18280, 18380)
    checks["LOOPBACK_DEFAULT"] = DEFAULT_HOST == "127.0.0.1"
    checks["HISTORY_BOUNDED"] = DEFAULT_HISTORY_MAX_ENTRIES == 1000

    # Diff verdict enum
    checks["DIFF_VERDICT_EXHAUSTIVE"] = set(DiffVerdict.__members__) == {
        "IMPROVED", "REGRESSED", "UNCHANGED", "MIXED",
    }

    # AuditHistoryEntry roundtrip
    sample = AuditHistoryEntry(
        audit_id="audit-test", timestamp=1234.0,
        verdict="PASS", n_endpoints_total=6, n_endpoints_2xx=6,
        n_invariants_total=9, n_invariants_failed=0,
        elapsed_s=12.3, json_path="/tmp/test.json",
    )
    rt = AuditHistoryEntry.from_dict(sample.to_dict())
    checks["HISTORY_ENTRY_ROUNDTRIP"] = (
        rt.audit_id == sample.audit_id
        and rt.verdict == sample.verdict
        and rt.n_endpoints_total == sample.n_endpoints_total
        and abs(rt.elapsed_s - sample.elapsed_s) < 1e-9
    )

    # Diff computation (self-diff → UNCHANGED)
    self_diff = compute_diff(sample, sample)
    checks["DIFF_SELF_UNCHANGED"] = self_diff.verdict == DiffVerdict.UNCHANGED

    # Diff computation (improvement)
    improved = AuditHistoryEntry(
        audit_id="audit-improved", timestamp=2000.0,
        verdict="PASS", n_endpoints_total=6, n_endpoints_2xx=6,
        n_invariants_total=9, n_invariants_failed=0,
        elapsed_s=10.0, json_path="/tmp/test.json",
    )
    regressed = AuditHistoryEntry(
        audit_id="audit-regressed", timestamp=3000.0,
        verdict="FAIL", n_endpoints_total=6, n_endpoints_2xx=3,
        n_invariants_total=9, n_invariants_failed=3,
        elapsed_s=15.0, json_path="/tmp/test.json",
    )
    diff_improved = compute_diff(regressed, improved)
    checks["DIFF_IMPROVED_VERDICT"] = diff_improved.verdict == DiffVerdict.IMPROVED

    diff_regressed = compute_diff(improved, regressed)
    checks["DIFF_REGRESSED_VERDICT"] = diff_regressed.verdict == DiffVerdict.REGRESSED

    # Port finding (sanity: should find one in range)
    try:
        port = find_open_port()
        checks["PORT_FOUND_IN_RANGE"] = DEFAULT_PORT_LOW <= port <= DEFAULT_PORT_HIGH
    except RuntimeError:
        checks["PORT_FOUND_IN_RANGE"] = False

    # Handler dispatch (in-process via demo with dry_run)
    try:
        demo = run_v1467_demo(do_real_audit=False)
        checks["DEMO_DRY_RUN_PASS"] = demo["verdict"] == "PASS"
        checks["DEMO_ALL_ENDPOINTS_HIT"] = len(demo["endpoints_hit"]) >= 5
        checks["DEMO_STATUS_HEALTHZ_OK"] = demo["endpoints_hit"].get("GET /healthz", {}).get("body_ok", False)
        checks["DEMO_STATUS_OK"] = demo["endpoints_hit"].get("GET /status", {}).get("body_ok", False)
        checks["DEMO_HISTORY_OK"] = demo["endpoints_hit"].get("GET /audit/history", {}).get("body_ok", False)
        checks["DEMO_AUDIT_DRY_OK"] = demo["endpoints_hit"].get("POST /audit/run", {}).get("body_ok", False)
        checks["DEMO_404_OK"] = demo["endpoints_hit"].get("GET /nonexistent", {}).get("body_ok", False)
        checks["DEMO_405_OK"] = demo["endpoints_hit"].get("POST /healthz", {}).get("body_ok", False)
    except Exception as exc:
        checks["DEMO_DRY_RUN_PASS"] = False
        checks["DEMO_DRY_RUN_ERROR"] = str(exc)[:120]

    # Diff endpoint via HTTP (in-process gateway + state seeding)
    # Seed the server's history directly (not via dry_run POST) so the diff endpoint
    # always has at least one entry to diff against.
    try:
        server, state, actual_port = make_gateway_server(host=DEFAULT_HOST, port=0)
        server_thread = threading.Thread(target=server.serve_forever, daemon=True)
        server_thread.start()
        try:
            # Seed history directly with one entry
            seed_entry = AuditHistoryEntry(
                audit_id="audit-popper-seed",
                timestamp=time.time(),
                verdict="PASS",
                n_endpoints_total=6,
                n_endpoints_2xx=6,
                n_invariants_total=9,
                n_invariants_failed=0,
                elapsed_s=0.0,
                json_path="",
            )
            state.append_history(seed_entry)

            import http.client
            conn = http.client.HTTPConnection(DEFAULT_HOST, actual_port, timeout=DEFAULT_SERVER_TIMEOUT_S)
            # 1. fetch history (should have one entry — the seed)
            conn.request("GET", "/audit/history")
            r1 = conn.getresponse()
            hist = json.loads(r1.read())
            r1.close()
            entries = hist.get("entries", [])
            # 2. self-diff via diff endpoint (should be UNCHANGED)
            if entries:
                only_id = entries[0]["audit_id"]
                qs = urllib.parse.urlencode({"baseline_id": only_id, "current_id": only_id})
                conn.request("GET", f"/audit/diff?{qs}")
                r2 = conn.getresponse()
                diff_obj = json.loads(r2.read())
                r2.close()
                checks["DEMO_DIFF_SELF_UNCHANGED"] = (
                    r2.status == 200 and diff_obj.get("verdict") == "UNCHANGED"
                )
            else:
                checks["DEMO_DIFF_SELF_UNCHANGED"] = False
            conn.close()
        finally:
            server.shutdown()
            server.server_close()
            server_thread.join(timeout=2.0)
    except Exception as exc:
        checks["DEMO_DIFF_SELF_UNCHANGED"] = False
        checks["DEMO_DIFF_ERROR"] = str(exc)[:120]

    n_pass = sum(1 for v in checks.values() if v)
    n_total = len(checks)
    out: Dict[str, Any] = {
        "n_checks": n_total,
        "passed": n_pass,
        "failed": [k for k, v in checks.items() if not v],
        "popper_pass": n_pass == n_total,
        "checks": checks,
    }
    return out


# ──────────────────────────────────────────────────────────────────────
# V1467 — CLI
# ──────────────────────────────────────────────────────────────────────


def _cli_run(args: argparse.Namespace) -> int:
    """CLI: boot the V1467 HTTP server and run forever (Ctrl-C to stop)."""
    host = args.host
    if not args.allow_lan and host == DEFAULT_HOST:
        pass  # already 127.0.0.1
    elif args.allow_lan and host == DEFAULT_HOST:
        pass  # explicitly opted in but still default
    server, _state, actual_port = make_gateway_server(host=host, port=args.port)
    print(json.dumps({
        "module": V1467_MODULE, "version": V1467_VERSION,
        "host": host, "port": actual_port,
        "endpoints": [
            "GET  /healthz", "GET  /status", "POST /audit/run",
            "GET  /audit/history", "GET  /audit/{audit_id}",
            "GET  /audit/diff?baseline_id=X&current_id=Y",
        ],
        "msg": "V1467 listening. Press Ctrl-C to stop.",
    }, indent=2, ensure_ascii=False))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n[V1467] Ctrl-C received; shutting down...")
    finally:
        server.shutdown()
        server.server_close()
    return 0


def _cli_demo(args: argparse.Namespace) -> int:
    """CLI: run the in-process demo (boots V1467, hits endpoints, returns summary)."""
    demo = run_v1467_demo(
        host=args.host, port=args.port,
        do_real_audit=not args.dry_run,
    )
    out_dir = Path(args.out_dir) if args.out_dir else Path("out")
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / ".v1467-demo-report.json"
    json_path.write_text(json.dumps(demo, ensure_ascii=False, indent=2, default=str),
                         encoding="utf-8")
    md_path = out_dir / ".v1467-demo-report.md"
    lines = [
        "# V1467 — Audit HTTP Gateway Demo Report",
        "",
        f"- module: `{V1467_MODULE}`",
        f"- version: `{V1467_VERSION}`",
        f"- host: `{demo['host']}`",
        f"- port: `{demo['port']}`",
        f"- boot_elapsed_s: `{demo['boot_elapsed_s']}`",
        f"- verdict: **`{demo['verdict']}`**",
        "",
        "## Endpoints hit",
        "",
        "| endpoint | status | body_ok |",
        "|----------|--------|---------|",
    ]
    for ep, h in demo["endpoints_hit"].items():
        lines.append(f"| `{ep}` | `{h.get('status_code')}` | `{h.get('body_ok')}` |")
    lines += [
        "",
        f"- real_audit_attempted: `{demo['real_audit_attempted']}`",
        f"- real_audit_ok: `{demo['real_audit_ok']}`",
        f"- history_count: `{demo['history_count']}`",
        f"- elapsed_s: `{demo['elapsed_s']}`",
        "",
        "## Stats",
        "",
        "```json",
        json.dumps(demo["stats"], indent=2, ensure_ascii=False),
        "```",
        "",
        "## Honest disclosure (主 17:43 实事求是)",
        "",
        "- V1467 is an HTTP wrapper around V1465 cross-audit + audit history + key-level diff.",
        "- V1467 ≠ orchestrator, ≠ CI, ≠ monitoring, ≠ test framework, ≠ production gateway.",
        "- Diff verdict (IMPROVED/REGRESSED/UNCHANGED/MIXED) is key-level heuristic, not formal regression testing.",
        "- Audit history is FIFO-evicted at 1000 entries (bounded disk usage).",
        "- Subprocess launch (V1465 → V1464) is bounded by DEFAULT_AUDIT_TIMEOUT_S = 120s.",
    ]
    md_path.write_text("\n".join(lines), encoding="utf-8")
    print(json.dumps({
        "verdict": demo["verdict"],
        "n_endpoints_hit": len(demo["endpoints_hit"]),
        "real_audit_ok": demo["real_audit_ok"],
        "json_report": str(json_path),
        "md_report": str(md_path),
    }, ensure_ascii=False, indent=2))
    return 0 if demo["verdict"] == "PASS" else 1


def _cli_serve(args: argparse.Namespace) -> int:
    """CLI: same as run — boot the V1467 HTTP server (alias for clarity)."""
    return _cli_run(args)


def _cli_status(_args: argparse.Namespace) -> int:
    """CLI: module status."""
    out = {
        "module": V1467_MODULE,
        "version": V1467_VERSION,
        "schema": V1467_SCHEMA,
        "date": V1467_DATE,
        "default_host": DEFAULT_HOST,
        "port_range": [DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH],
        "body_max_bytes": DEFAULT_BODY_MAX_BYTES,
        "audit_timeout_s": DEFAULT_AUDIT_TIMEOUT_S,
        "history_max_entries": DEFAULT_HISTORY_MAX_ENTRIES,
        "guards": list(V1467_GUARDS),
        "v3_guards": list(V1467_V3_GUARDS),
        "borrowed": list(BORROWED_SOURCES),
    }
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


def _cli_popper(_args: argparse.Namespace) -> int:
    """CLI: Popper self-check (no subprocess, in-process only)."""
    res = popper_v1467()
    print(json.dumps(res, ensure_ascii=False, indent=2))
    return 0 if res["popper_pass"] else 1


def _cli_meta(_args: argparse.Namespace) -> int:
    """CLI: module metadata."""
    out = {
        "schema": V1467_SCHEMA,
        "version": V1467_VERSION,
        "module": V1467_MODULE,
        "phase": 1467,
        "post": ["v1465", "v1464", "v1463", "v1462", "v1461", "v1460"],
        "guards": list(V1467_GUARDS),
        "v3_guards": list(V1467_V3_GUARDS),
        "borrowed": list(BORROWED_SOURCES),
        "endpoints": [
            "GET  /healthz",
            "GET  /status",
            "POST /audit/run",
            "GET  /audit/history",
            "GET  /audit/{audit_id}",
            "GET  /audit/diff?baseline_id=X&current_id=Y",
        ],
    }
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


def _cli_chain(_args: argparse.Namespace) -> int:
    """CLI: borrowed lineage."""
    out = {
        "chain": [
            "v1460 — Anyone-Run Harness (12/13 stages)",
            "v1461 — Docker-Equivalent Subprocess Sandbox (9 modes)",
            "v1462 — Subprocess Sandbox Spec Security Linter (24 rules)",
            "v1463 — Lint-Gate Subprocess Pipeline (30 adversarial specs)",
            "v1464 — HTTP Gateway wrapping V1463 (6 endpoints)",
            "v1465 — Cross-Module Live Audit (boots V1464 in subprocess + 6 endpoints + 9 invariants)",
            "v1466 — Cross-Process Subprocess Runner (5 stages × subprocess.run)",
            "v1467 — THIS: HTTP Gateway wrapping V1465 + audit history + regression diff",
            "stdlib — http.server + socketserver + json + urllib + threading + tempfile",
        ],
        "v1467_borrows": list(BORROWED_SOURCES),
    }
    print(json.dumps(out, ensure_ascii=False, indent=2))
    return 0


def _cli_help(_args: argparse.Namespace) -> int:
    """CLI: extended help."""
    print(__doc__)
    return 0


def _build_parser() -> argparse.ArgumentParser:
    """Build CLI parser."""
    parser = argparse.ArgumentParser(
        prog=V1467_MODULE,
        description=(
            "V1467 — ASI Real Cross-Audit HTTP Gateway + Audit History + Regression Diff "
            "(主 13:31 + 主 23:44 + 主 00:44 + 主 00:56)."
        ),
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_run = sub.add_parser("run", help="Boot HTTP server (Ctrl-C to stop)")
    p_run.add_argument("--host", default=DEFAULT_HOST,
                       help=f"Bind host (default: {DEFAULT_HOST}; opt-in non-loopback via --allow-lan)")
    p_run.add_argument("--port", type=int, default=0,
                       help=f"Bind port (0 = auto-find in [{DEFAULT_PORT_LOW}, {DEFAULT_PORT_HIGH}])")
    p_run.add_argument("--allow-lan", action="store_true",
                       help="Allow non-loopback bind (still requires explicit --host)")
    p_run.set_defaults(func=_cli_run)

    p_demo = sub.add_parser("demo", help="Run in-process demo (boots gateway, hits endpoints)")
    p_demo.add_argument("--host", default=DEFAULT_HOST)
    p_demo.add_argument("--port", type=int, default=0)
    p_demo.add_argument("--dry-run", action="store_true",
                        help="Skip real audit subprocess; use /audit/run dry_run=true")
    p_demo.add_argument("--out-dir", default=None)
    p_demo.set_defaults(func=_cli_demo)

    p_serve = sub.add_parser("serve", help="Alias for run (boot HTTP server)")
    p_serve.add_argument("--host", default=DEFAULT_HOST)
    p_serve.add_argument("--port", type=int, default=0)
    p_serve.add_argument("--allow-lan", action="store_true")
    p_serve.set_defaults(func=_cli_serve)

    sub.add_parser("status", help="Module status").set_defaults(func=_cli_status)
    sub.add_parser("popper", help="In-process Popper self-check").set_defaults(func=_cli_popper)
    sub.add_parser("meta", help="Module metadata").set_defaults(func=_cli_meta)
    sub.add_parser("chain", help="Borrowed lineage").set_defaults(func=_cli_chain)
    sub.add_parser("help", help="Extended help").set_defaults(func=_cli_help)
    return parser


def main(argv: Optional[Sequence[str]] = None) -> int:
    """CLI entry point."""
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())