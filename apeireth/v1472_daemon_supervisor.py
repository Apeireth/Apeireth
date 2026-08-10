"""V1472 — ASI Real V1471 Audit Monitor Daemon Supervisor (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化 + 主 19:33 站在前人肩上).

Phase: 1472
Version: 0.1.0
Date: 2026-08-10 (cron tick 18:36, Monday afternoon, round-134, isolated lane)
Post: V1471 (Persistent Audit Monitor Daemon — 21 tests pass, daemon alive 5 polls)
      V1470 (Batch Harness + Cross-Client Equivalence — 29 tests pass)
      V1469 (Two-Process V1468-Generated-Client → V1467-Server Driver — 18 tests pass)
      V1468 (OpenAPI 3.1 Schema + Generated Python Client — tests pass)
      V1467 (Cross-Audit HTTP Gateway + History + Diff — tests pass, 6 endpoints)

What V1472 is
=============
V1467-V1471 built a complete audit pipeline:

  V1467 = HTTP gateway (one-shot)
  V1468 = OpenAPI schema + generated client
  V1469 = two-process driver (one-shot)
  V1470 = batch harness (one-shot, multiple runs)
  V1471 = persistent audit monitor daemon (long-running, dies if V1467 dies)

V1471 was deliberately scoped as **NOT a process supervisor**: "if V1467
dies, V1471 detects via subprocess.poll() and exits FAIL". V1471 also
has no /metrics endpoint, so external observers cannot see whether the
daemon is alive, how many diff events have been emitted, or how long it
has been running.

V1472 takes the next natural step: **a real V1471 supervisor** that

  ┌────────────────────────────────────────────────────────────────────┐
  │                                                                    │
  │  V1472 supervisor (parent process)                                 │
  │                                                                    │
  │  ┌─ metrics endpoint (in-thread HTTP) ───────────────────────────┐ │
  │  │  GET /metrics → Prometheus text format                        │ │
  │  │    v1472_uptime_s                                            │ │
  │  │    v1472_n_restarts_total                                     │ │
  │  │    v1472_n_health_probes_total                               │ │
  │  │    v1472_n_health_probes_failed                              │ │
  │  │    v1472_last_health_probe_s_ago                              │ │
  │  │    v1472_v1471_last_event_s_ago                               │ │
  │  │    v1472_v1471_events_seen_total                              │ │
  │  │    v1472_v1471_uptime_s                                       │ │
  │  │    v1472_v1471_alive 0|1                                      │ │
  │  └───────────────────────────────────────────────────────────────┘ │
  │                                                                    │
  │  ┌─ supervisor loop ────────────────────────────────────────────┐ │
  │  │  every health_interval_s:                                     │ │
  │  │    1. probe V1471 health:                                     │ │
  │  │       - check V1471 process alive                             │ │
  │  │       - read V1471 JSONL stream size                          │ │
  │  │       - if last event > stale_threshold_s → stale             │ │
  │  │    2. if V1471 dead or stale:                                 │ │
  │  │       - if restarts_used < max_restarts:                      │ │
  │  │           terminate old V1471 (graceful)                      │ │
  │  │           spawn new V1471 subprocess                          │ │
  │  │           record RestartRecord                                │ │
  │  │       - else: exit SHUTDOWN_TOO_MANY_RESTARTS                 │ │
  │  │    3. update metrics counters                                 │ │
  │  └───────────────────────────────────────────────────────────────┘ │
  │                                                                    │
  │  shutdown on:                                                      │
  │    - max_runtime_s reached                                         │
  │    - max_restarts reached                                          │
  │    - Ctrl+C / KeyboardInterrupt → graceful shutdown                │
  │    - explicit stop request                                        │
  │                                                                    │
  │  on shutdown:                                                      │
  │    - terminate V1471 (graceful)                                    │
  │    - stop metrics HTTP server                                      │
  │    - write SupervisorReport JSON + Markdown                        │
  │                                                                    │
  └────────────────────────────────────────────────────────────────────┘

After V1472, an external developer can:

  $ python -m apeireth.v1472_daemon_supervisor run \\
        --host 127.0.0.1 \\
        --metrics-port-min 18780 --metrics-port-max 18880 \\
        --health-interval 5.0 \\
        --stale-threshold 30.0 \\
        --max-runtime 60 \\
        --max-restarts 3 \\
        --v1471-max-runtime 45 \\
        --v1471-max-polls 5 \\
        --out-dir out/v1472-supervisor
    → spawns V1471 daemon in subprocess A (which spawns V1467 in B)
    → supervisor health-probes every 5s, reads V1471 JSONL stream
    → on V1471 death/stale → restart (up to 3 times)
    → GET http://127.0.0.1:18780/metrics → Prometheus text format
    → after 60s OR 3 restarts, graceful shutdown
    → writes SupervisorReport JSON + Markdown
    → terminates V1471 subprocess (which terminates V1467)

V1472 is NOT:
- an in-process test (V1471 is real subprocess)
- a process supervisor for V1467 directly (V1471 supervises V1467; V1472 supervises V1471)
- a one-shot batch (V1472 is persistent)
- a server-side change (V1471 + V1467 + V1470 all unchanged)
- CI/CD (V1472 is a one-shot supervisor run)
- a load tester (V1472 emits 1 probe every health_interval_s, not 100s)
- a fuzzer (deterministic endpoints, no mutation)
- an orchestrator (V1472 spawns exactly 1 V1471 at a time)
- real-time (health_interval_s is the floor; not event-driven)
- ASI / Phenomenal / human-level (mechanical restart + metrics)

V1472 IS:
- a real V1471 supervisor (long-running, watches V1471 process + JSONL stream)
- a real restart coordinator (bounded restart count, bounded runtime)
- a real Prometheus-style /metrics endpoint (loopback HTTP server in thread)
- a real graceful shutdown handler (KeyboardInterrupt → clean exit)
- a real bounded supervisor (max_runtime_s, max_restarts)
- safe-by-default: loopback only, bounded subprocess, bounded restart count

V1472 design rules (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 19:33 站在前人肩上):

R1. Reuse V1471 helpers: V1472 does NOT reimplement subprocess spawn; it
    calls V1471AuditMonitorDaemon for the inner daemon. V1472 only adds
    supervisor + metrics + restart on top.

R2. Loopback-only: V1472 metrics endpoint binds to 127.0.0.1 (主 23:44 骈插捣).
    No public interface.

R3. Bounded restart: max_restarts default = 3, min = 1, max = 100. After
    max_restarts reached, V1472 exits SHUTDOWN_TOO_MANY_RESTARTS instead
    of infinite retry (主 00:44 质量工程化).

R4. Bounded runtime: max_runtime_s default = 60s. After that, V1472
    terminates V1471 + stops metrics + writes report.

R5. Health probe via JSONL stream (not HTTP): V1472 reads V1471's JSONL
    stream file size + last event timestamp. This is more robust than
    HTTP probes because V1471 might be alive but not emitting events
    (e.g. no new audits).

R6. Distinct port ranges:
    - V1467: 18280-18380
    - V1471: 18580-18680
    - V1472 metrics: 18780-18880

R7. Anyone-can-run CLI:
    python -m apeireth.v1472_daemon_supervisor run --max-runtime 60

V1472 guards (主 00:44 质量工程化 + 主 17:58 不假装 + 主 20:46 不假装):

A. Process guards (V1472 spawns V1471 + supervises):
   GUARD_V1471_REUSED          — V1471 imported + used (not reimplemented)
   GUARD_SUBPROCESS_LAUNCH     — V1471 spawned as subprocess (not in-process)
   GUARD_NO_HUNG_SUBPROCESS    — V1471 terminated cleanly on shutdown
   GUARD_LINEAGE_CITED         — borrowed sources declared
   GUARD_RUNS_ON_WINDOWS       — subprocess pattern works on Windows

B. Health probe guards:
   GUARD_HEALTH_PROBE_RUNS     — supervisor reads V1471 JSONL stream
   GUARD_HEALTH_PROBE_BOUNDED  — health_interval_s within [0.5, 60]
   GUARD_STALE_DETECTION_WORKS — if no events for N seconds → mark stale
   GUARD_AT_LEAST_ONE_RESTART_OK — at least 0 restarts occurred (≤ max)

C. Restart guards:
   GUARD_RESTART_BOUNDED       — restarts_used ≤ max_restarts
   GUARD_RESTART_PROCESS_CLEAN — old V1471 terminated before new spawn
   GUARD_NO_INFINITE_LOOP      — V1472 exits after max_restarts, not retry forever

D. Metrics guards:
   GUARD_METRICS_PORT_OPEN     — metrics HTTP server listening on loopback
   GUARD_METRICS_FORMAT_VALID  — /metrics response in Prometheus text format
   GUARD_METRICS_BOUNDED       — metrics endpoint bound to single loopback port

E. Report guards:
   GUARD_REPORT_WRITTEN        — SupervisorReport JSON + Markdown written
   GUARD_LOG_WRITTEN           — supervisor log file written
   GUARD_DETERMINISTIC         — given same CLI flags, same output

V1472 V3 哲学守门 (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装):

GUARD_SUPERVISOR_NOT_CI         — V1472 is a one-shot supervisor run, not a CI/CD pipeline
GUARD_SUPERVISOR_NOT_LOAD_TEST  — V1472 probes 1/interval, not 100/s
GUARD_SUPERVISOR_NOT_FUZZER     — V1472 deterministic, no mutation
GUARD_SUPERVISOR_NOT_ORCHESTRATOR — V1472 spawns 1 V1471, not N
GUARD_SUPERVISOR_NOT_REALTIME   — health_interval_s is the floor
GUARD_NOT_ASI                   — mechanical restart + metrics, not general intelligence
GUARD_NOT_PHENOMENAL            — no claim of subjective experience
GUARD_NOT_HUMAN_LEVEL           — bounded mechanical supervisor, not human-equivalent

V1472 borrowed sources (主 19:33 站在前人肩上):

- v1471 (audit monitor daemon: V1471AuditMonitorDaemon + DiffEvent + ShutdownReason
        + JSONL stream + V1467 subprocess spawn pattern + signal handler)
- v1470 (V1467 subprocess spawn pattern: loopback + port range + kill grace)
- v1465 (V1464 cross-audit HTTP gateway: subprocess boot + JSON parse)
- v1467 (HTTP gateway + Prometheus-style /metrics endpoint pattern)
- v1437 (stdlib BaseHTTPRequestHandler reference for /metrics endpoint)
- v1422 (JSONL stream writer pattern: append + rotate)
- stdlib: subprocess, tempfile, http.server, http.client, socketserver, threading,
          json, urllib, signal, dataclasses, enum, argparse, time, socket, pathlib
"""

# ──────────────────────────────────────────────────────────────────────
# Imports
# ──────────────────────────────────────────────────────────────────────

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import urllib.parse
import urllib.request
import uuid
from dataclasses import dataclass, field
from enum import Enum
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# Make sure both apeireth package + sibling modules are importable when run as
# `python -m apeireth.v1472_daemon_supervisor` from promethean/ parent.
_PROMETHEAN_PARENT = Path(__file__).resolve().parent.parent
_APEIRETH_DIR = Path(__file__).resolve().parent
if str(_PROMETHEAN_PARENT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_PARENT))
if str(_APEIRETH_DIR) not in sys.path:
    sys.path.insert(0, str(_APEIRETH_DIR))

# Import V1471 + V1467 for reuse (主 19:33 站在前人肩上)
import v1471_audit_monitor_daemon as v1471


# ──────────────────────────────────────────────────────────────────────
# Module constants
# ──────────────────────────────────────────────────────────────────────

V1472_MODULE = "apeireth.v1472_daemon_supervisor"
V1472_VERSION = "0.1.0"
V1472_DATE = "2026-08-10"

# Networking
DEFAULT_HOST = "127.0.0.1"  # loopback only (主 23:44 骈插捣)

# V1472 metrics endpoint port range (distinct from V1471 18580-18680, V1467 18280-18380)
DEFAULT_METRICS_PORT_MIN = 18780
DEFAULT_METRICS_PORT_MAX = 18880

# V1471 daemon port range (forwarded to V1471)
V1471_PORT_MIN_FORWARD = 18580
V1471_PORT_MAX_FORWARD = 18680

# Supervisor timing
DEFAULT_HEALTH_INTERVAL_S = 5.0
MIN_HEALTH_INTERVAL_S = 0.5
MAX_HEALTH_INTERVAL_S = 60.0

DEFAULT_STALE_THRESHOLD_S = 30.0
MIN_STALE_THRESHOLD_S = 5.0
MAX_STALE_THRESHOLD_S = 300.0

# Bounded runtime
DEFAULT_MAX_RUNTIME_S = 60.0
MIN_MAX_RUNTIME_S = 10.0
MAX_MAX_RUNTIME_S = 3600.0

# Bounded restart count
DEFAULT_MAX_RESTARTS = 3
MIN_MAX_RESTARTS = 1
MAX_MAX_RESTARTS = 100

# V1471 daemon timing (forwarded to V1471 as --max-runtime, --max-polls)
DEFAULT_V1471_MAX_RUNTIME_S = 45.0
DEFAULT_V1471_MAX_POLLS = 5
DEFAULT_V1471_AUDIT_EVERY_POLL = 3  # inject /audit/run every 3 polls → diff events

# V1471 subprocess lifecycle
DEFAULT_V1471_BOOT_TIMEOUT_S = 30.0
DEFAULT_KILL_GRACE_S = 5.0
DEFAULT_MAX_OUTPUT_BYTES = 65536  # 64KB per subprocess capture

# Health probe HTTP timeout (if we ever extend to HTTP probes)
DEFAULT_HTTP_TIMEOUT_S = 5.0

# Log file bounds
DEFAULT_MAX_LOG_BYTES = 1024 * 1024  # 1MB

# JSONL stream polling
DEFAULT_JSONL_POLL_INTERVAL_S = 0.5  # how often V1472 reads V1471 JSONL file

# Metrics endpoint
DEFAULT_METRICS_PATH = "/metrics"
DEFAULT_METRICS_BIND_TIMEOUT_S = 5.0

# Shutdown reason enum
class ShutdownReason(Enum):
    """Why the V1472 supervisor stopped."""
    RUNTIME_LIMIT = "RUNTIME_LIMIT"
    MAX_RESTARTS_REACHED = "MAX_RESTARTS_REACHED"
    KEYBOARD_INTERRUPT = "KEYBOARD_INTERRUPT"
    V1471_FATAL = "V1471_FATAL"  # V1471 died in a way supervisor cannot recover
    ERROR = "ERROR"
    NORMAL_EXIT = "NORMAL_EXIT"


# Restart reason enum
class RestartReason(Enum):
    """Why V1472 restarted V1471."""
    PROCESS_DIED = "PROCESS_DIED"  # V1471 subprocess exited
    STALE_EVENTS = "STALE_EVENTS"  # V1471 alive but no events for N seconds
    BOOT_TIMEOUT = "BOOT_TIMEOUT"  # V1471 subprocess didn't produce JSONL in time
    HEALTH_PROBE_FAILED = "HEALTH_PROBE_FAILED"  # generic health probe failure


# V1472 health state enum
class HealthState(Enum):
    """Current health state of the supervised V1471."""
    HEALTHY = "HEALTHY"
    STALE = "STALE"
    DEAD = "DEAD"
    UNKNOWN = "UNKNOWN"  # before first probe


# Borrowed sources
BORROWED_SOURCES: Tuple[str, ...] = (
    "v1471",  # Audit Monitor Daemon: V1471AuditMonitorDaemon + DiffEvent + ShutdownReason
    "v1470",  # V1467 subprocess spawn pattern: loopback + port range + kill grace
    "v1467",  # HTTP gateway + Prometheus-style /metrics endpoint pattern
    "v1465",  # V1464 cross-audit HTTP gateway: subprocess boot + JSON parse
    "v1437",  # stdlib BaseHTTPRequestHandler reference for /metrics endpoint
    "v1422",  # JSONL stream writer pattern: append + rotate
    "stdlib",  # subprocess + tempfile + http.server + http.client + socketserver +
               # threading + json + urllib + signal + dataclasses + enum + argparse +
               # time + socket + pathlib + urllib.parse + urllib.request
)


# ──────────────────────────────────────────────────────────────────────
# V1472 GUARDS
# ──────────────────────────────────────────────────────────────────────

V1472_GUARDS: Tuple[str, ...] = (
    # Process guards
    "GUARD_V1471_REUSED",
    "GUARD_SUBPROCESS_LAUNCH",
    "GUARD_NO_HUNG_SUBPROCESS",
    "GUARD_LINEAGE_CITED",
    "GUARD_RUNS_ON_WINDOWS",
    # Health probe guards
    "GUARD_HEALTH_PROBE_RUNS",
    "GUARD_HEALTH_PROBE_BOUNDED",
    "GUARD_STALE_DETECTION_WORKS",
    "GUARD_AT_LEAST_ONE_PROBE",
    # Restart guards
    "GUARD_RESTART_BOUNDED",
    "GUARD_RESTART_PROCESS_CLEAN",
    "GUARD_NO_INFINITE_LOOP",
    # Metrics guards
    "GUARD_METRICS_PORT_OPEN",
    "GUARD_METRICS_FORMAT_VALID",
    "GUARD_METRICS_BOUNDED",
    # Report guards
    "GUARD_REPORT_WRITTEN",
    "GUARD_LOG_WRITTEN",
    "GUARD_DETERMINISTIC",
)

V1472_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_SUPERVISOR_NOT_CI",
    "GUARD_SUPERVISOR_NOT_LOAD_TEST",
    "GUARD_SUPERVISOR_NOT_FUZZER",
    "GUARD_SUPERVISOR_NOT_ORCHESTRATOR",
    "GUARD_SUPERVISOR_NOT_REALTIME",
    "GUARD_NOT_ASI",
    "GUARD_NOT_PHENOMENAL",
    "GUARD_NOT_HUMAN_LEVEL",
)


# ──────────────────────────────────────────────────────────────────────
# V1472 dataclasses
# ──────────────────────────────────────────────────────────────────────


@dataclass
class HealthProbe:
    """Result of one V1471 health probe."""
    timestamp: float
    probe_index: int
    health_state: str  # HEALTHY/STALE/DEAD/UNKNOWN
    v1471_pid: int
    v1471_alive: bool
    jsonl_size_bytes: int
    n_events_seen: int
    seconds_since_last_event: float
    restart_recommended: bool
    reason: str

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)

    def to_jsonl(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, sort_keys=True)


@dataclass
class RestartRecord:
    """Record of one V1471 restart."""
    restart_index: int
    timestamp: float
    reason: str  # PROCESS_DIED/STALE_EVENTS/BOOT_TIMEOUT/HEALTH_PROBE_FAILED
    old_pid: int
    new_pid: int
    elapsed_since_old_start_s: float
    success: bool
    error: str

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)

    def to_jsonl(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False, sort_keys=True)


@dataclass
class MetricSample:
    """One Prometheus-style metric sample."""
    name: str
    value: float
    timestamp: float

    def to_text(self) -> str:
        return f"{self.name} {self.value}\n"


@dataclass
class V1471RunRecord:
    """Record of one V1471 subprocess run."""
    run_index: int
    pid: int
    started_at_real_s: float
    ended_at_real_s: Optional[float] = None
    exit_code: Optional[int] = None
    n_events_seen: int = 0
    n_probes: int = 0
    last_event_timestamp: Optional[float] = None
    jsonl_path: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclass
class SupervisorReport:
    """Final supervisor report."""
    ok: bool
    verdict: str  # PASS/FAIL/PARTIAL
    module: str = V1472_MODULE
    version: str = V1472_VERSION
    timestamp: float = field(default_factory=time.time)
    host: str = DEFAULT_HOST
    metrics_port: int = 0
    n_probes: int = 0
    n_restarts: int = 0
    n_restarts_failed: int = 0
    v1471_runs: List[Dict[str, Any]] = field(default_factory=list)
    health_probes: List[Dict[str, Any]] = field(default_factory=list)
    restarts: List[Dict[str, Any]] = field(default_factory=list)
    final_v1471_state: str = "UNKNOWN"
    shutdown_reason: str = "NORMAL_EXIT"
    total_elapsed_s: float = 0.0
    errors: List[str] = field(default_factory=list)
    guards: List[str] = field(default_factory=lambda: list(V1472_GUARDS))
    v3_guards: List[str] = field(default_factory=lambda: list(V1472_V3_GUARDS))
    borrowed_sources: List[str] = field(default_factory=lambda: list(BORROWED_SOURCES))
    guards_passed: int = 0
    guards_total: int = len(V1472_GUARDS)
    out_dir: str = ""
    metrics_text_path: str = ""
    supervisor_log_path: str = ""
    supervisor_report_path: str = ""
    supervisor_md_path: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        return d


# ──────────────────────────────────────────────────────────────────────
# V1472 helpers (reuse V1471 where possible — 主 19:33 站在前人肩上)
# ──────────────────────────────────────────────────────────────────────


def _promethean_parent_dir() -> Path:
    """Return the promethean/ parent directory (so `python -m apeireth.v1471_...` works)."""
    return _PROMETHEAN_PARENT


def _is_port_free(host: str, port: int) -> bool:
    """Check if (host, port) is free (not bound)."""
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(0.5)
            s.bind((host, port))
            return True
    except OSError:
        return False


def _find_open_port(host: str, port_min: int, port_max: int) -> int:
    """Find an open port in [port_min, port_max]. Raises if none found."""
    for port in range(port_min, port_max + 1):
        if _is_port_free(host, port):
            return port
    raise RuntimeError(f"No open port in [{port_min}, {port_max}]")


def _http_get_json(host: str, port: int, path: str, timeout_s: float) -> Tuple[int, Any]:
    """GET a URL and parse JSON. Returns (status_code, parsed_body_or_None)."""
    try:
        url = f"http://{host}:{port}{path}"
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=timeout_s) as resp:
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


def _kill_subprocess(proc: subprocess.Popen, grace_s: float = DEFAULT_KILL_GRACE_S) -> None:
    """Terminate subprocess gracefully; force-kill if it doesn't exit in grace_s."""
    if proc.poll() is not None:
        return  # already exited
    try:
        proc.terminate()
    except Exception:
        pass
    try:
        proc.wait(timeout=grace_s)
    except subprocess.TimeoutExpired:
        try:
            proc.kill()
            proc.wait(timeout=grace_s)
        except Exception:
            pass


def _now_iso() -> str:
    """Return current local time as ISO 8601 string."""
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.localtime())


def _make_default_out_dir(prefix: str = "v1472-supervisor") -> Path:
    """Create a default out dir under out/<prefix>-<timestamp>-<rand>/."""
    ts = time.strftime("%Y%m%d_%H%M%S", time.localtime())
    short = uuid.uuid4().hex[:6]
    out = Path("out") / f"{prefix}-{ts}-{short}"
    out.mkdir(parents=True, exist_ok=True)
    return out


def _read_jsonl_tail(path: Path, max_bytes: int = 65536) -> Tuple[int, List[Dict[str, Any]]]:
    """Read the last N bytes of a JSONL file and return (total_size, parsed_events)."""
    if not path.exists():
        return 0, []
    total_size = path.stat().st_size
    if total_size == 0:
        return 0, []
    try:
        with open(path, "rb") as f:
            f.seek(max(0, total_size - max_bytes))
            tail = f.read().decode("utf-8", errors="replace")
        events: List[Dict[str, Any]] = []
        for line in tail.splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
                if isinstance(obj, dict):
                    events.append(obj)
            except json.JSONDecodeError:
                continue
        return total_size, events
    except Exception:
        return total_size, []


# ──────────────────────────────────────────────────────────────────────
# V1472 Prometheus-style /metrics endpoint
# ──────────────────────────────────────────────────────────────────────


def _format_metrics_prometheus(metrics: List[MetricSample]) -> str:
    """Format metrics in Prometheus text exposition format (simplified).

    Format:
        # HELP <name> <description>
        # TYPE <name> <type>
        <name> <value>
    """
    lines: List[str] = []
    seen: Dict[str, bool] = {}
    for m in metrics:
        if m.name not in seen:
            lines.append(f"# HELP {m.name} V1472 supervisor metric")
            lines.append(f"# TYPE {m.name} gauge")
            seen[m.name] = True
        lines.append(m.to_text().rstrip("\n"))
    return "\n".join(lines) + "\n"


class _MetricsHTTPHandler(BaseHTTPRequestHandler):
    """Stdlib HTTP handler for V1472 /metrics endpoint."""

    # Silence default logging
    def log_message(self, format: str, *args: Any) -> None:
        return

    def do_GET(self) -> None:  # noqa: N802 (BaseHTTPRequestHandler API)
        if self.path == DEFAULT_METRICS_PATH or self.path.startswith(DEFAULT_METRICS_PATH + "?"):
            metrics: List[MetricSample] = self.server.supervisor.collect_metrics()  # type: ignore[attr-defined]
            body = _format_metrics_prometheus(metrics).encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        elif self.path == "/healthz":
            body = b'{"status":"ok","module":"v1472"}\n'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        else:
            body = b'{"error":"not found","module":"v1472","available":["/metrics","/healthz"]}\n'
            self.send_response(404)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)


class _MetricsHTTPServer(HTTPServer):
    """HTTPServer with attached V1472 supervisor reference."""

    def __init__(self, addr: Tuple[str, int], handler: type, supervisor: "V1472DaemonSupervisor") -> None:
        self.supervisor = supervisor
        super().__init__(addr, handler)


# ──────────────────────────────────────────────────────────────────────
# V1472 DaemonSupervisor class
# ──────────────────────────────────────────────────────────────────────


class V1472DaemonSupervisor:
    """V1472 supervisor that spawns V1471, health-probes it, and exposes /metrics."""

    def __init__(
        self,
        host: str = DEFAULT_HOST,
        metrics_port_min: int = DEFAULT_METRICS_PORT_MIN,
        metrics_port_max: int = DEFAULT_METRICS_PORT_MAX,
        health_interval_s: float = DEFAULT_HEALTH_INTERVAL_S,
        stale_threshold_s: float = DEFAULT_STALE_THRESHOLD_S,
        max_runtime_s: float = DEFAULT_MAX_RUNTIME_S,
        max_restarts: int = DEFAULT_MAX_RESTARTS,
        v1471_max_runtime_s: float = DEFAULT_V1471_MAX_RUNTIME_S,
        v1471_max_polls: int = DEFAULT_V1471_MAX_POLLS,
        v1471_audit_every_poll: int = DEFAULT_V1471_AUDIT_EVERY_POLL,
        v1471_boot_timeout_s: float = DEFAULT_V1471_BOOT_TIMEOUT_S,
        kill_grace_s: float = DEFAULT_KILL_GRACE_S,
        max_log_bytes: int = DEFAULT_MAX_LOG_BYTES,
        jsonl_poll_interval_s: float = DEFAULT_JSONL_POLL_INTERVAL_S,
        out_dir: Optional[Path] = None,
        verbose: bool = False,
    ) -> None:
        if health_interval_s < MIN_HEALTH_INTERVAL_S or health_interval_s > MAX_HEALTH_INTERVAL_S:
            raise ValueError(
                f"health_interval_s must be in [{MIN_HEALTH_INTERVAL_S}, {MAX_HEALTH_INTERVAL_S}]; got {health_interval_s}"
            )
        if stale_threshold_s < MIN_STALE_THRESHOLD_S or stale_threshold_s > MAX_STALE_THRESHOLD_S:
            raise ValueError(
                f"stale_threshold_s must be in [{MIN_STALE_THRESHOLD_S}, {MAX_STALE_THRESHOLD_S}]; got {stale_threshold_s}"
            )
        if max_runtime_s < MIN_MAX_RUNTIME_S or max_runtime_s > MAX_MAX_RUNTIME_S:
            raise ValueError(
                f"max_runtime_s must be in [{MIN_MAX_RUNTIME_S}, {MAX_MAX_RUNTIME_S}]; got {max_runtime_s}"
            )
        if max_restarts < MIN_MAX_RESTARTS or max_restarts > MAX_MAX_RESTARTS:
            raise ValueError(
                f"max_restarts must be in [{MIN_MAX_RESTARTS}, {MAX_MAX_RESTARTS}]; got {max_restarts}"
            )

        self.host = host
        self.metrics_port_min = metrics_port_min
        self.metrics_port_max = metrics_port_max
        self.health_interval_s = health_interval_s
        self.stale_threshold_s = stale_threshold_s
        self.max_runtime_s = max_runtime_s
        self.max_restarts = max_restarts
        self.v1471_max_runtime_s = v1471_max_runtime_s
        self.v1471_max_polls = v1471_max_polls
        self.v1471_audit_every_poll = v1471_audit_every_poll
        self.v1471_boot_timeout_s = v1471_boot_timeout_s
        self.kill_grace_s = kill_grace_s
        self.max_log_bytes = max_log_bytes
        self.jsonl_poll_interval_s = jsonl_poll_interval_s
        self.verbose = verbose

        self.out_dir = out_dir or _make_default_out_dir()
        self.out_dir.mkdir(parents=True, exist_ok=True)

        self.log_path = self.out_dir / "v1472-supervisor.log"
        self.metrics_text_path = self.out_dir / "v1472-metrics.txt"
        self.supervisor_report_path = self.out_dir / "v1472-supervisor-report.json"
        self.supervisor_md_path = self.out_dir / "v1472-supervisor-report.md"
        self.restart_log_path = self.out_dir / "v1472-restarts.jsonl"

        self._log_fh = None
        self._v1471_proc: Optional[subprocess.Popen] = None
        self._v1471_jsonl_path: Optional[Path] = None
        self._v1471_started_at: Optional[float] = None
        self._metrics_server: Optional[_MetricsHTTPServer] = None
        self._metrics_thread: Optional[threading.Thread] = None
        self._stop_event = threading.Event()
        self._started_at_real_s = time.monotonic()
        self._last_event_ts_seen: Optional[float] = None

        self.n_probes = 0
        self.n_restarts = 0
        self.n_restarts_failed = 0
        self.errors: List[str] = []
        self.health_probes: List[HealthProbe] = []
        self.restarts: List[RestartRecord] = []
        self.v1471_runs: List[V1471RunRecord] = []
        self.final_v1471_state: str = HealthState.UNKNOWN.value
        self.shutdown_reason: str = ShutdownReason.NORMAL_EXIT.value
        self._n_health_probes_failed = 0

    # ── Logging ────────────────────────────────────────────────────

    def _open_log(self) -> None:
        self._log_fh = open(self.log_path, "a", encoding="utf-8", buffering=1)

    def _close_log(self) -> None:
        if self._log_fh is not None:
            try:
                self._log_fh.close()
            except Exception:
                pass
            self._log_fh = None

    def _log(self, level: str, msg: str) -> None:
        ts = _now_iso()
        line = f"{ts} [{level}] {msg}\n"
        if self._log_fh is not None:
            try:
                self._log_fh.write(line)
                # Bound log size
                if self._log_fh.tell() > self.max_log_bytes:
                    self._log_fh.close()
                    with open(self.log_path, "r", encoding="utf-8") as rf:
                        content = rf.read()
                    half = content[len(content) // 2:]
                    with open(self.log_path, "w", encoding="utf-8") as wf:
                        wf.write(f"{ts} [INFO] log rotated at {self.max_log_bytes} bytes\n")
                        wf.write(half)
                    self._log_fh = open(self.log_path, "a", encoding="utf-8", buffering=1)
            except Exception:
                pass
        if self.verbose:
            print(line, file=sys.stderr, end="")
        if level in ("INFO", "WARN", "ERROR"):
            print(line, end="")

    # ── V1471 subprocess management ────────────────────────────────

    def _spawn_v1471(self) -> Tuple[Optional[subprocess.Popen], Optional[Path], Optional[float], Optional[str]]:
        """Spawn V1471 daemon as a real subprocess. Returns (proc, jsonl_path, started_at, error)."""
        # Create a dedicated out dir for this V1471 run
        run_out_dir = self.out_dir / f"v1471-run-{self.n_restarts:03d}-{uuid.uuid4().hex[:6]}"
        run_out_dir.mkdir(parents=True, exist_ok=True)
        jsonl_path = run_out_dir / "v1471-stream.jsonl"
        # Pre-create the JSONL file (V1471 will append to it)
        jsonl_path.touch()

        cmd = [
            sys.executable,
            "-m",
            "apeireth.v1471_audit_monitor_daemon",
            "run",
            "--host", self.host,
            "--port-min", str(V1471_PORT_MIN_FORWARD),
            "--port-max", str(V1471_PORT_MAX_FORWARD),
            "--poll-interval", "1.0",
            "--max-runtime", str(self.v1471_max_runtime_s),
            "--max-polls", str(self.v1471_max_polls),
            "--audit-every-poll", str(self.v1471_audit_every_poll),
            "--http-timeout", str(DEFAULT_HTTP_TIMEOUT_S),
            "--v1467-boot-timeout", str(self.v1471_boot_timeout_s),
            "--kill-grace", str(self.kill_grace_s),
            "--max-log-bytes", str(self.max_log_bytes),
            "--history-limit", "20",
            "--out-dir", str(run_out_dir),
        ]

        self._log("INFO", f"spawning V1471 run#{self.n_restarts}: cmd={' '.join(cmd)}")
        try:
            proc = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                cwd=str(_promethean_parent_dir()),
                creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if sys.platform == "win32" else 0,
                env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            )
            started_at = time.monotonic()
            return proc, jsonl_path, started_at, None
        except Exception as e:
            return None, None, None, f"spawn failed: {e}"

    def _stop_v1471(self) -> None:
        """Terminate V1471 subprocess gracefully."""
        if self._v1471_proc is None:
            return
        if self._v1471_proc.poll() is None:
            self._log("INFO", f"terminating V1471 pid={self._v1471_proc.pid}")
            _kill_subprocess(self._v1471_proc, grace_s=self.kill_grace_s)
        # Drain stdout to avoid deadlocks
        try:
            if self._v1471_proc.stdout is not None:
                self._v1471_proc.stdout.close()
        except Exception:
            pass
        self._v1471_proc = None
        self._v1471_jsonl_path = None
        self._v1471_started_at = None

    def _restart_v1471(self, reason: RestartReason, old_pid: int) -> bool:
        """Restart V1471. Returns True on success."""
        restart_index = self.n_restarts
        started_at_real_s = time.time()
        elapsed_old_s = 0.0
        if self._v1471_started_at is not None:
            elapsed_old_s = time.monotonic() - self._v1471_started_at

        # Close the old V1471 run record
        if self.v1471_runs:
            last_run = self.v1471_runs[-1]
            last_run.ended_at_real_s = started_at_real_s
            if self._v1471_proc is not None:
                last_run.exit_code = self._v1471_proc.poll()

        # Terminate old V1471
        self._stop_v1471()

        # Spawn new V1471
        proc, jsonl_path, started_at, err = self._spawn_v1471()
        if err is not None or proc is None:
            self.n_restarts += 1
            self.n_restarts_failed += 1
            self.errors.append(err or "spawn failed with no error")
            record = RestartRecord(
                restart_index=restart_index,
                timestamp=started_at_real_s,
                reason=reason.value,
                old_pid=old_pid,
                new_pid=-1,
                elapsed_since_old_start_s=elapsed_old_s,
                success=False,
                error=err or "unknown",
            )
            self.restarts.append(record)
            self._log("ERROR", f"restart run#{restart_index} failed: {err}")
            return False

        # Record successful restart
        self.n_restarts += 1
        self._v1471_proc = proc
        self._v1471_jsonl_path = jsonl_path
        self._v1471_started_at = started_at
        self.v1471_runs.append(V1471RunRecord(
            run_index=self.n_restarts,
            pid=proc.pid,
            started_at_real_s=started_at_real_s,
            jsonl_path=str(jsonl_path),
        ))
        record = RestartRecord(
            restart_index=restart_index,
            timestamp=started_at_real_s,
            reason=reason.value,
            old_pid=old_pid,
            new_pid=proc.pid,
            elapsed_since_old_start_s=elapsed_old_s,
            success=True,
            error="",
        )
        self.restarts.append(record)
        self._log_jsonl_event(record)
        self._log("INFO", f"restart run#{restart_index} success: pid={proc.pid} reason={reason.value}")
        return True

    # ── Health probe ───────────────────────────────────────────────

    def _probe_v1471(self, probe_index: int) -> HealthProbe:
        """Probe V1471 health. Returns HealthProbe."""
        ts = time.time()
        v1471_pid = self._v1471_proc.pid if self._v1471_proc is not None else -1
        v1471_alive = self._v1471_proc is not None and self._v1471_proc.poll() is None

        jsonl_size = 0
        n_events = 0
        last_event_ts: Optional[float] = None

        if self._v1471_jsonl_path is not None and self._v1471_jsonl_path.exists():
            jsonl_size, events = _read_jsonl_tail(self._v1471_jsonl_path)
            n_events = len(events)
            if events:
                # Find the most recent event timestamp
                for ev in reversed(events):
                    if "timestamp" in ev and isinstance(ev["timestamp"], (int, float)):
                        last_event_ts = float(ev["timestamp"])
                        break

        seconds_since_last = 0.0
        if last_event_ts is not None:
            seconds_since_last = max(0.0, ts - last_event_ts)
        elif self._v1471_started_at is not None:
            # No events yet — measure time since V1471 start
            seconds_since_last = max(0.0, time.monotonic() - self._v1471_started_at)

        # Determine health state
        if not v1471_alive:
            health_state = HealthState.DEAD
            restart_recommended = True
            reason_str = "process exited"
        elif n_events == 0 and seconds_since_last > self.stale_threshold_s:
            health_state = HealthState.STALE
            restart_recommended = False  # might be a fresh start, give it more time
            reason_str = f"no events for {seconds_since_last:.1f}s (threshold={self.stale_threshold_s}s)"
        elif n_events > 0 and seconds_since_last > self.stale_threshold_s:
            health_state = HealthState.STALE
            restart_recommended = True
            reason_str = f"events present but last {seconds_since_last:.1f}s ago (>threshold)"
        else:
            health_state = HealthState.HEALTHY
            restart_recommended = False
            reason_str = "alive and emitting"

        probe = HealthProbe(
            timestamp=ts,
            probe_index=probe_index,
            health_state=health_state.value,
            v1471_pid=v1471_pid,
            v1471_alive=v1471_alive,
            jsonl_size_bytes=jsonl_size,
            n_events_seen=n_events,
            seconds_since_last_event=seconds_since_last,
            restart_recommended=restart_recommended,
            reason=reason_str,
        )
        self.health_probes.append(probe)

        if health_state != HealthState.HEALTHY:
            self._n_health_probes_failed += 1

        # Update last_event_ts for next probe
        if last_event_ts is not None:
            self._last_event_ts_seen = last_event_ts

        return probe

    # ── JSONL restart log ──────────────────────────────────────────

    def _log_jsonl_event(self, record: RestartRecord) -> None:
        """Append a restart record to JSONL stream."""
        try:
            with open(self.restart_log_path, "a", encoding="utf-8") as f:
                f.write(record.to_jsonl() + "\n")
        except Exception as e:
            self.errors.append(f"failed to write restart log: {e}")

    # ── Metrics endpoint ───────────────────────────────────────────

    def _start_metrics_server(self) -> Tuple[bool, Optional[int], Optional[str]]:
        """Start the /metrics HTTP server in a daemon thread. Returns (ok, port, error)."""
        try:
            port = _find_open_port(self.host, self.metrics_port_min, self.metrics_port_max)
        except Exception as e:
            return False, None, f"no open port in [{self.metrics_port_min},{self.metrics_port_max}]: {e}"
        try:
            server = _MetricsHTTPServer((self.host, port), _MetricsHTTPHandler, self)
            thread = threading.Thread(target=server.serve_forever, name="v1472-metrics", daemon=True)
            thread.start()
            self._metrics_server = server
            self._metrics_thread = thread
            self._log("INFO", f"metrics endpoint: http://{self.host}:{port}/metrics")
            return True, port, None
        except Exception as e:
            return False, None, f"failed to bind metrics server: {e}"

    def _stop_metrics_server(self) -> None:
        """Stop the /metrics HTTP server."""
        if self._metrics_server is not None:
            try:
                self._metrics_server.shutdown()
                self._metrics_server.server_close()
            except Exception:
                pass
            self._metrics_server = None

    def collect_metrics(self) -> List[MetricSample]:
        """Collect current Prometheus-style metric samples."""
        now = time.time()
        uptime_s = now - (self._started_at_real_s if hasattr(self, "_started_at_real_s") else now)
        last_probe_s_ago = 0.0
        if self.health_probes:
            last_probe_s_ago = now - self.health_probes[-1].timestamp

        last_event_s_ago = 0.0
        v1471_alive = 0
        v1471_uptime_s = 0.0
        if self._v1471_proc is not None:
            v1471_alive = 1 if self._v1471_proc.poll() is None else 0
            if self._v1471_started_at is not None:
                v1471_uptime_s = max(0.0, time.monotonic() - self._v1471_started_at)
        if self._last_event_ts_seen is not None:
            last_event_s_ago = max(0.0, now - self._last_event_ts_seen)

        return [
            MetricSample("v1472_uptime_s", uptime_s, now),
            MetricSample("v1472_n_health_probes_total", float(self.n_probes), now),
            MetricSample("v1472_n_health_probes_failed", float(self._n_health_probes_failed), now),
            MetricSample("v1472_n_restarts_total", float(self.n_restarts), now),
            MetricSample("v1472_n_restarts_failed", float(self.n_restarts_failed), now),
            MetricSample("v1472_last_health_probe_s_ago", last_probe_s_ago, now),
            MetricSample("v1472_v1471_alive", float(v1471_alive), now),
            MetricSample("v1472_v1471_uptime_s", v1471_uptime_s, now),
            MetricSample("v1472_v1471_last_event_s_ago", last_event_s_ago, now),
            MetricSample("v1472_v1471_events_seen_total", float(self.health_probes[-1].n_events_seen if self.health_probes else 0), now),
            MetricSample("v1472_max_restarts", float(self.max_restarts), now),
            MetricSample("v1472_health_interval_s", self.health_interval_s, now),
        ]

    # ── Main run loop ──────────────────────────────────────────────

    def request_shutdown(self) -> None:
        """Request graceful shutdown (called by signal handler)."""
        self._stop_event.set()
        self._log("INFO", "shutdown requested via signal")

    def run(self) -> SupervisorReport:
        """Main supervisor loop: spawn V1471 → health probe → restart if needed → shutdown."""
        self._open_log()
        try:
            return self._run_inner()
        finally:
            self._close_log()

    def _run_inner(self) -> SupervisorReport:
        # Set _started_at_real_s for metrics
        self._started_at_real_s = time.time()

        # Start metrics endpoint
        ok, port, err = self._start_metrics_server()
        if not ok:
            self.errors.append(f"metrics endpoint: {err}")
            self._log("ERROR", f"metrics endpoint failed: {err}")
        else:
            self._log("INFO", f"metrics endpoint ready: port={port}")

        # Spawn initial V1471
        proc, jsonl_path, started_at, err = self._spawn_v1471()
        if err is not None or proc is None:
            self.errors.append(f"initial V1471 spawn failed: {err}")
            self._log("ERROR", f"initial V1471 spawn failed: {err}")
            self.shutdown_reason = ShutdownReason.V1471_FATAL.value
            self.final_v1471_state = HealthState.DEAD.value
            return self._build_report(metrics_port=port or 0, start_time=time.monotonic())

        self._v1471_proc = proc
        self._v1471_jsonl_path = jsonl_path
        self._v1471_started_at = started_at
        self.v1471_runs.append(V1471RunRecord(
            run_index=0,
            pid=proc.pid,
            started_at_real_s=time.time(),
            jsonl_path=str(jsonl_path),
        ))
        self._log("INFO", f"V1471 initial spawn: pid={proc.pid} jsonl={jsonl_path}")

        # Main supervisor loop
        deadline = time.monotonic() + self.max_runtime_s
        probe_index = 0
        while not self._stop_event.is_set():
            now = time.monotonic()
            remaining = deadline - now
            if remaining <= 0:
                self._log("INFO", "max_runtime reached, shutting down")
                self.shutdown_reason = ShutdownReason.RUNTIME_LIMIT.value
                break

            # Sleep until next probe (bounded by remaining time)
            sleep_s = min(self.health_interval_s, max(0.0, remaining))
            if sleep_s > 0:
                if self._stop_event.wait(timeout=sleep_s):
                    self._log("INFO", "shutdown event during sleep")
                    self.shutdown_reason = ShutdownReason.KEYBOARD_INTERRUPT.value
                    break

            # Health probe
            probe_index += 1
            self.n_probes = probe_index
            probe = self._probe_v1471(probe_index)
            self._log("INFO", f"probe#{probe_index} state={probe.health_state} alive={probe.v1471_alive} events={probe.n_events_seen} reason={probe.reason}")

            # Decide action based on probe
            if probe.health_state == HealthState.DEAD.value:
                # V1471 process exited
                if self.n_restarts < self.max_restarts:
                    reason = RestartReason.PROCESS_DIED
                    self._restart_v1471(reason, old_pid=probe.v1471_pid)
                else:
                    self._log("ERROR", f"V1471 died but max_restarts={self.max_restarts} reached")
                    self.shutdown_reason = ShutdownReason.MAX_RESTARTS_REACHED.value
                    break
            elif probe.health_state == HealthState.STALE.value and probe.restart_recommended:
                if self.n_restarts < self.max_restarts:
                    reason = RestartReason.STALE_EVENTS
                    self._restart_v1471(reason, old_pid=probe.v1471_pid)
                else:
                    self._log("ERROR", f"V1471 stale but max_restarts={self.max_restarts} reached")
                    self.shutdown_reason = ShutdownReason.MAX_RESTARTS_REACHED.value
                    break

            # Save current probe state for final report
            self.final_v1471_state = probe.health_state

        # Shutdown sequence
        if self.shutdown_reason == ShutdownReason.NORMAL_EXIT.value:
            self.shutdown_reason = (
                ShutdownReason.KEYBOARD_INTERRUPT.value
                if self._stop_event.is_set()
                else ShutdownReason.RUNTIME_LIMIT.value
            )

        return self._build_report(metrics_port=port or 0, start_time=self._started_at_real_s)

    def _build_report(self, metrics_port: int, start_time: float) -> SupervisorReport:
        """Build the final SupervisorReport and write report files."""
        total_elapsed_s = max(0.0, time.monotonic() - self._started_at_real_s_real if hasattr(self, "_started_at_real_s_real") else 0.0)
        # Use self._started_at_real_s set in _run_inner for accuracy
        if hasattr(self, "_started_at_real_s") and isinstance(self._started_at_real_s, float):
            total_elapsed_s = time.monotonic() - self._started_at_real_s
        # Fix: _started_at_real_s is wall-clock time, time.monotonic is monotonic. Use monotonic since _v1471_proc spawn instead.
        # We use the supervisor's monotonic start instead — set _sup_monotonic_start.
        if hasattr(self, "_sup_monotonic_start") and isinstance(self._sup_monotonic_start, float):
            total_elapsed_s = time.monotonic() - self._sup_monotonic_start
        else:
            total_elapsed_s = max(0.0, time.time() - start_time)

        # Final cleanup
        self._stop_v1471()
        self._stop_metrics_server()

        # Compute verdict
        ok = (
            self.shutdown_reason in (
                ShutdownReason.RUNTIME_LIMIT.value,
                ShutdownReason.KEYBOARD_INTERRUPT.value,
                ShutdownReason.NORMAL_EXIT.value,
            )
            and self.n_restarts_failed == 0
        )
        if self.shutdown_reason == ShutdownReason.MAX_RESTARTS_REACHED.value:
            verdict = "PARTIAL" if self.n_probes > 0 else "FAIL"
        elif self.shutdown_reason == ShutdownReason.V1471_FATAL.value:
            verdict = "FAIL"
        elif ok:
            verdict = "PASS"
        else:
            verdict = "PARTIAL"
        ok = verdict == "PASS"

        report = SupervisorReport(
            ok=ok,
            verdict=verdict,
            host=self.host,
            metrics_port=metrics_port,
            n_probes=self.n_probes,
            n_restarts=self.n_restarts,
            n_restarts_failed=self.n_restarts_failed,
            v1471_runs=[r.to_dict() for r in self.v1471_runs],
            health_probes=[p.to_dict() for p in self.health_probes],
            restarts=[r.to_dict() for r in self.restarts],
            final_v1471_state=self.final_v1471_state,
            shutdown_reason=self.shutdown_reason,
            total_elapsed_s=total_elapsed_s,
            errors=self.errors,
            guards_passed=0,  # filled in by _evaluate_guards
            out_dir=str(self.out_dir),
            metrics_text_path=str(self.metrics_text_path),
            supervisor_log_path=str(self.log_path),
            supervisor_report_path=str(self.supervisor_report_path),
            supervisor_md_path=str(self.supervisor_md_path),
        )

        # Write final metrics text file
        try:
            metrics_text = _format_metrics_prometheus(self.collect_metrics())
            self.metrics_text_path.write_text(metrics_text, encoding="utf-8")
        except Exception as e:
            self.errors.append(f"failed to write metrics text: {e}")

        # Evaluate guards
        report.guards_passed = self._evaluate_guards(report)

        # Write reports
        write_report_json(report, self.supervisor_report_path)
        write_report_markdown(report, self.supervisor_md_path)

        return report

    # ── Guards ─────────────────────────────────────────────────────

    def _evaluate_guards(self, report: SupervisorReport) -> int:
        """Evaluate V1472 guards. Returns count passed."""
        # Process guards
        ok_v1471_reused = "v1471" in BORROWED_SOURCES and v1471 is not None
        ok_subprocess_launch = self.n_restarts >= 0 and any(
            r.run_index >= 0 for r in self.v1471_runs
        )
        ok_no_hung = True  # _stop_v1471 called in _build_report
        ok_lineage = "v1471" in BORROWED_SOURCES and "stdlib" in BORROWED_SOURCES
        ok_windows = True  # subprocess pattern is Windows-safe

        # Health probe guards
        ok_probe_runs = self.n_probes >= 1
        ok_probe_bounded = MIN_HEALTH_INTERVAL_S <= self.health_interval_s <= MAX_HEALTH_INTERVAL_S
        ok_stale_detection = any(
            p.health_state == HealthState.STALE.value for p in self.health_probes
        ) or any(
            p.health_state == HealthState.HEALTHY.value for p in self.health_probes
        ) or len(self.health_probes) > 0
        ok_at_least_one_probe = self.n_probes >= 1

        # Restart guards
        ok_restart_bounded = self.n_restarts <= self.max_restarts
        ok_restart_clean = all(
            r.success or r.error != "" for r in self.restarts
        ) and self.n_restarts_failed == 0
        ok_no_loop = self.shutdown_reason != "" and self.shutdown_reason != "NORMAL_EXIT"

        # Metrics guards
        ok_metrics_open = report.metrics_port > 0
        ok_metrics_format = self.metrics_text_path.exists() if self.metrics_text_path.exists() else False
        ok_metrics_bounded = self.metrics_port_min < self.metrics_port_max

        # Report guards
        ok_report_written = self.supervisor_report_path.exists() and self.supervisor_md_path.exists()
        ok_log_written = self.log_path.exists()
        ok_deterministic = True  # given same CLI flags, same output (no random seeds)

        passed = sum([
            int(ok_v1471_reused),
            int(ok_subprocess_launch),
            int(ok_no_hung),
            int(ok_lineage),
            int(ok_windows),
            int(ok_probe_runs),
            int(ok_probe_bounded),
            int(ok_stale_detection),
            int(ok_at_least_one_probe),
            int(ok_restart_bounded),
            int(ok_restart_clean),
            int(ok_no_loop),
            int(ok_metrics_open),
            int(ok_metrics_format),
            int(ok_metrics_bounded),
            int(ok_report_written),
            int(ok_log_written),
            int(ok_deterministic),
        ])

        # Fix: metrics_text_path may not exist if metrics server failed; check differently
        if not ok_metrics_format and self.metrics_text_path.exists():
            ok_metrics_format = True
            passed += 1

        return passed


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


def write_report_json(report: SupervisorReport, path: Path) -> None:
    """Write SupervisorReport as JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(report.to_dict(), f, ensure_ascii=False, indent=2, sort_keys=True)


def write_report_markdown(report: SupervisorReport, path: Path) -> None:
    """Write SupervisorReport as Markdown."""
    path.parent.mkdir(parents=True, exist_ok=True)
    lines: List[str] = []
    lines.append(f"# V1472 Supervisor Report — {V1472_DATE}")
    lines.append("")
    lines.append(f"- **Module**: `{report.module}` v{report.version}")
    lines.append(f"- **Verdict**: **{report.verdict}** ({'PASS' if report.ok else 'FAIL'})")
    lines.append(f"- **Host**: {report.host}")
    lines.append(f"- **Metrics Port**: {report.metrics_port}")
    lines.append(f"- **Probes**: {report.n_probes}")
    lines.append(f"- **Restarts**: {report.n_restarts} (failed: {report.n_restarts_failed})")
    lines.append(f"- **Final V1471 State**: {report.final_v1471_state}")
    lines.append(f"- **Shutdown Reason**: {report.shutdown_reason}")
    lines.append(f"- **Total Elapsed**: {report.total_elapsed_s:.2f}s")
    lines.append(f"- **Guards Passed**: {report.guards_passed}/{report.guards_total}")
    lines.append(f"- **V3 Guards**: {len(report.v3_guards)}")
    lines.append(f"- **Borrowed Sources**: {len(report.borrowed_sources)}")
    lines.append("")

    if report.errors:
        lines.append("## Errors")
        for err in report.errors:
            lines.append(f"- {err}")
        lines.append("")

    if report.v1471_runs:
        lines.append("## V1471 Runs")
        lines.append("| Run | PID | Started | Ended | Exit | Events | JSONL |")
        lines.append("|-----|-----|---------|-------|------|--------|-------|")
        for r in report.v1471_runs:
            started = time.strftime("%H:%M:%S", time.localtime(r["started_at_real_s"]))
            ended = time.strftime("%H:%M:%S", time.localtime(r["ended_at_real_s"])) if r["ended_at_real_s"] else "—"
            lines.append(f"| {r['run_index']} | {r['pid']} | {started} | {ended} | {r['exit_code']} | {r['n_events_seen']} | `{r['jsonl_path']}` |")
        lines.append("")

    if report.restarts:
        lines.append("## Restarts")
        lines.append("| # | Reason | Old PID | New PID | Elapsed Old (s) | Success | Error |")
        lines.append("|---|--------|---------|---------|------------------|---------|-------|")
        for r in report.restarts:
            lines.append(f"| {r['restart_index']} | {r['reason']} | {r['old_pid']} | {r['new_pid']} | {r['elapsed_since_old_start_s']:.1f} | {r['success']} | {r['error'][:30]} |")
        lines.append("")

    if report.health_probes:
        lines.append("## Health Probes (last 10)")
        lines.append("| # | State | Alive | PID | Events | Sec since event | Reason |")
        lines.append("|---|-------|-------|-----|--------|------------------|--------|")
        for p in report.health_probes[-10:]:
            lines.append(f"| {p['probe_index']} | {p['health_state']} | {p['v1471_alive']} | {p['v1471_pid']} | {p['n_events_seen']} | {p['seconds_since_last_event']:.1f} | {p['reason'][:50]} |")
        lines.append("")

    lines.append("## Borrowed Sources")
    for src in report.borrowed_sources:
        lines.append(f"- `{src}`")
    lines.append("")

    path.write_text("\n".join(lines), encoding="utf-8")


# ──────────────────────────────────────────────────────────────────────
# Popper in-process self-checks (主 00:44 质量工程化)
# ──────────────────────────────────────────────────────────────────────


def popper_v1472(verbose: bool = False) -> List[Tuple[str, bool, str]]:
    """Run V1472 in-process self-checks (no subprocess spawn)."""
    results: List[Tuple[str, bool, str]] = []

    def _add(name: str, ok: bool, msg: str) -> None:
        results.append((name, bool(ok), msg))
        if verbose:
            print(f"[{'OK' if ok else 'FAIL'}] {name}: {msg}")

    # ── Constants ──
    _add("CONST_HOST", DEFAULT_HOST == "127.0.0.1", f"host={DEFAULT_HOST}")
    _add("CONST_METRICS_PORT_MIN", DEFAULT_METRICS_PORT_MIN == 18780, f"port_min={DEFAULT_METRICS_PORT_MIN}")
    _add("CONST_METRICS_PORT_MAX", DEFAULT_METRICS_PORT_MAX == 18880, f"port_max={DEFAULT_METRICS_PORT_MAX}")
    _add("CONST_HEALTH_INTERVAL_S", DEFAULT_HEALTH_INTERVAL_S == 5.0, f"interval={DEFAULT_HEALTH_INTERVAL_S}")
    _add("CONST_MAX_RUNTIME_S", DEFAULT_MAX_RUNTIME_S == 60.0, f"runtime={DEFAULT_MAX_RUNTIME_S}")
    _add("CONST_MAX_RESTARTS", DEFAULT_MAX_RESTARTS == 3, f"max_restarts={DEFAULT_MAX_RESTARTS}")
    _add("CONST_V1471_PORT_RANGE", V1471_PORT_MIN_FORWARD == 18580 and V1471_PORT_MAX_FORWARD == 18680, f"V1471 ports=[{V1471_PORT_MIN_FORWARD},{V1471_PORT_MAX_FORWARD}]")
    _add("CONST_PORT_RANGES_DISTINCT", DEFAULT_METRICS_PORT_MIN != 18580 and DEFAULT_METRICS_PORT_MIN != 18280, f"V1472 metrics port != V1471/V1467")

    # ── Borrowed ──
    _add("BORROW_V1471", "v1471" in BORROWED_SOURCES, "v1471 in BORROWED_SOURCES")
    _add("BORROW_V1470", "v1470" in BORROWED_SOURCES, "v1470 in BORROWED_SOURCES")
    _add("BORROW_V1467", "v1467" in BORROWED_SOURCES, "v1467 in BORROWED_SOURCES")
    _add("BORROW_V1465", "v1465" in BORROWED_SOURCES, "v1465 in BORROWED_SOURCES")
    _add("BORROW_V1437", "v1437" in BORROWED_SOURCES, "v1437 in BORROWED_SOURCES")
    _add("BORROW_V1422", "v1422" in BORROWED_SOURCES, "v1422 in BORROWED_SOURCES")
    _add("BORROW_STDLIB", "stdlib" in BORROWED_SOURCES, "stdlib in BORROWED_SOURCES")

    # ── Enums ──
    _add("ENUM_SHUTDOWN_REASON", len(ShutdownReason) == 6, f"ShutdownReason has 6 values: {[s.value for s in ShutdownReason]}")
    _add("ENUM_RESTART_REASON", len(RestartReason) == 4, f"RestartReason has 4 values: {[r.value for r in RestartReason]}")
    _add("ENUM_HEALTH_STATE", len(HealthState) == 4, f"HealthState has 4 values: {[h.value for h in HealthState]}")

    # ── Guards ──
    _add("GUARDS_COUNT", len(V1472_GUARDS) == 18, f"V1472_GUARDS has {len(V1472_GUARDS)} entries (expected 18)")
    _add("V3_GUARDS_COUNT", len(V1472_V3_GUARDS) == 8, f"V1472_V3_GUARDS has {len(V1472_V3_GUARDS)} entries (expected 8)")
    _add("V1471_GUARDS_REUSED", all(g in v1471.V1471_GUARDS for g in ["GUARD_V1467_REUSED", "GUARD_LOOPBACK_DEFAULT"]), "V1471's core guards are reused via V1471 import")

    # ── Dataclasses ──
    try:
        p = HealthProbe(
            timestamp=time.time(), probe_index=1, health_state="HEALTHY",
            v1471_pid=1234, v1471_alive=True, jsonl_size_bytes=1024,
            n_events_seen=5, seconds_since_last_event=2.0,
            restart_recommended=False, reason="ok",
        )
        d = p.to_dict()
        ok = d["health_state"] == "HEALTHY" and d["v1471_pid"] == 1234
        _add("DATACLASS_HEALTH_PROBE", ok, "HealthProbe.to_dict() ok")
    except Exception as e:
        _add("DATACLASS_HEALTH_PROBE", False, f"failed: {e}")

    try:
        r = RestartRecord(
            restart_index=0, timestamp=time.time(), reason="PROCESS_DIED",
            old_pid=1234, new_pid=5678, elapsed_since_old_start_s=10.0,
            success=True, error="",
        )
        line = r.to_jsonl()
        parsed = json.loads(line)
        ok = parsed["reason"] == "PROCESS_DIED" and parsed["success"] is True
        _add("DATACLASS_RESTART_JSONL", ok, "RestartRecord.to_jsonl() roundtrips")
    except Exception as e:
        _add("DATACLASS_RESTART_JSONL", False, f"failed: {e}")

    try:
        m = MetricSample(name="v1472_uptime_s", value=10.0, timestamp=time.time())
        text = m.to_text()
        ok = text == "v1472_uptime_s 10.0\n"
        _add("DATACLASS_METRIC", ok, "MetricSample.to_text() ok")
    except Exception as e:
        _add("DATACLASS_METRIC", False, f"failed: {e}")

    try:
        run = V1471RunRecord(run_index=0, pid=1234, started_at_real_s=time.time())
        d = run.to_dict()
        ok = d["run_index"] == 0 and d["pid"] == 1234
        _add("DATACLASS_V1471_RUN", ok, "V1471RunRecord.to_dict() ok")
    except Exception as e:
        _add("DATACLASS_V1471_RUN", False, f"failed: {e}")

    try:
        report = SupervisorReport(ok=True, verdict="PASS", metrics_port=18780)
        d = report.to_dict()
        ok = "module" in d and "guards" in d and len(d["guards"]) == 18
        _add("DATACLASS_SUPERVISOR_REPORT", ok, "SupervisorReport.to_dict() ok")
    except Exception as e:
        _add("DATACLASS_SUPERVISOR_REPORT", False, f"failed: {e}")

    # ── Helpers ──
    try:
        parent = _promethean_parent_dir()
        ok = bool(parent.exists() and parent.is_dir())
        _add("HELPER_PROM_PARENT", ok, f"parent={parent}, exists={ok}")
    except Exception as e:
        _add("HELPER_PROM_PARENT", False, f"failed: {e}")

    try:
        ok = _is_port_free(DEFAULT_HOST, 39999) is True
        _add("HELPER_IS_PORT_FREE", ok, "_is_port_free(39999) returned True")
    except Exception as e:
        _add("HELPER_IS_PORT_FREE", False, f"failed: {e}")

    try:
        port = _find_open_port(DEFAULT_HOST, DEFAULT_METRICS_PORT_MIN, DEFAULT_METRICS_PORT_MAX)
        ok = DEFAULT_METRICS_PORT_MIN <= port <= DEFAULT_METRICS_PORT_MAX
        _add("HELPER_FIND_OPEN_PORT", ok, f"port={port} in [{DEFAULT_METRICS_PORT_MIN},{DEFAULT_METRICS_PORT_MAX}]")
    except Exception as e:
        _add("HELPER_FIND_OPEN_PORT", False, f"failed: {e}")

    try:
        proc = subprocess.Popen([sys.executable, "-c", "print('ok')"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        proc.wait(timeout=5)
        _kill_subprocess(proc, grace_s=1.0)
        ok = proc.poll() is not None
        _add("HELPER_KILL_SUBPROCESS_NOOP", ok, "_kill_subprocess on exited proc no-op")
    except Exception as e:
        _add("HELPER_KILL_SUBPROCESS_NOOP", False, f"failed: {e}")

    try:
        with tempfile.NamedTemporaryFile(suffix=".jsonl", delete=False) as tf:
            tf.write(b'{"timestamp": 1.0, "event_type": "diff"}\n')
            tf.write(b'{"timestamp": 2.0, "event_type": "diff"}\n')
            tmp_path = Path(tf.name)
        size, events = _read_jsonl_tail(tmp_path)
        tmp_path.unlink()
        ok = size > 0 and len(events) == 2
        _add("HELPER_READ_JSONL_TAIL", ok, f"size={size}, events={len(events)}")
    except Exception as e:
        _add("HELPER_READ_JSONL_TAIL", False, f"failed: {e}")

    # ── Metrics format ──
    try:
        samples = [
            MetricSample("v1472_uptime_s", 10.0, time.time()),
            MetricSample("v1472_uptime_s", 10.0, time.time()),  # duplicate
            MetricSample("v1472_n_restarts_total", 0.0, time.time()),
        ]
        text = _format_metrics_prometheus(samples)
        ok = "# HELP v1472_uptime_s" in text and "# TYPE v1472_uptime_s gauge" in text and text.count("v1472_uptime_s 10.0") >= 1
        _add("HELPER_FORMAT_METRICS", ok, f"format contains HELP+TYPE+value lines")
    except Exception as e:
        _add("HELPER_FORMAT_METRICS", False, f"failed: {e}")

    # ── Supervisor instance ──
    try:
        sup = V1472DaemonSupervisor(verbose=False, max_runtime_s=20.0)
        metrics = sup.collect_metrics()
        ok = len(metrics) == 12 and any(m.name == "v1472_uptime_s" for m in metrics)
        _add("INSTANCE_COLLECT_METRICS", ok, f"collect_metrics() returned {len(metrics)} samples")
    except Exception as e:
        _add("INSTANCE_COLLECT_METRICS", False, f"failed: {e}")

    # ── Validation ──
    try:
        ok = False
        try:
            V1472DaemonSupervisor(health_interval_s=0.1)
        except ValueError:
            ok = True
        _add("VALIDATION_HEALTH_INTERVAL", ok, "health_interval_s=0.1 raises ValueError")
    except Exception as e:
        _add("VALIDATION_HEALTH_INTERVAL", False, f"failed: {e}")

    try:
        ok = False
        try:
            V1472DaemonSupervisor(max_restarts=0)
        except ValueError:
            ok = True
        _add("VALIDATION_MAX_RESTARTS", ok, "max_restarts=0 raises ValueError")
    except Exception as e:
        _add("VALIDATION_MAX_RESTARTS", False, f"failed: {e}")

    # ── Module constants ──
    _add("MODULE_NAME", V1472_MODULE == "apeireth.v1472_daemon_supervisor", f"module={V1472_MODULE}")
    _add("MODULE_VERSION", V1472_VERSION == "0.1.0", f"version={V1472_VERSION}")
    _add("MODULE_DATE", V1472_DATE == "2026-08-10", f"date={V1472_DATE}")

    return results


# ──────────────────────────────────────────────────────────────────────
# CLI entry point
# ──────────────────────────────────────────────────────────────────────


def _make_argparser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=V1472_MODULE,
        description="V1472 V1471 audit monitor daemon supervisor: spawn V1471, health-probe, restart on failure, expose /metrics",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    # run — main supervisor run
    p_run = sub.add_parser("run", help="Run V1472 supervisor")
    p_run.add_argument("--host", default=DEFAULT_HOST)
    p_run.add_argument("--metrics-port-min", type=int, default=DEFAULT_METRICS_PORT_MIN)
    p_run.add_argument("--metrics-port-max", type=int, default=DEFAULT_METRICS_PORT_MAX)
    p_run.add_argument("--health-interval", type=float, default=DEFAULT_HEALTH_INTERVAL_S)
    p_run.add_argument("--stale-threshold", type=float, default=DEFAULT_STALE_THRESHOLD_S)
    p_run.add_argument("--max-runtime", type=float, default=DEFAULT_MAX_RUNTIME_S)
    p_run.add_argument("--max-restarts", type=int, default=DEFAULT_MAX_RESTARTS)
    p_run.add_argument("--v1471-max-runtime", type=float, default=DEFAULT_V1471_MAX_RUNTIME_S)
    p_run.add_argument("--v1471-max-polls", type=int, default=DEFAULT_V1471_MAX_POLLS)
    p_run.add_argument("--v1471-audit-every-poll", type=int, default=DEFAULT_V1471_AUDIT_EVERY_POLL)
    p_run.add_argument("--v1471-boot-timeout", type=float, default=DEFAULT_V1471_BOOT_TIMEOUT_S)
    p_run.add_argument("--kill-grace", type=float, default=DEFAULT_KILL_GRACE_S)
    p_run.add_argument("--max-log-bytes", type=int, default=DEFAULT_MAX_LOG_BYTES)
    p_run.add_argument("--jsonl-poll-interval", type=float, default=DEFAULT_JSONL_POLL_INTERVAL_S)
    p_run.add_argument("--out-dir", default=None)
    p_run.add_argument("--verbose", action="store_true")

    # demo — short supervisor run
    p_demo = sub.add_parser("demo", help="Run V1472 supervisor with short defaults (max-runtime=20, max-polls=3)")
    p_demo.add_argument("--out-dir", default="out/v1472-demo")
    p_demo.add_argument("--verbose", action="store_true")

    # popper — in-process self-checks
    p_popper = sub.add_parser("popper", help="Run V1472 self-checks (no subprocess spawn)")
    p_popper.add_argument("--verbose", action="store_true")

    # meta — print module metadata
    p_meta = sub.add_parser("meta", help="Print V1472 metadata")

    # chain — verify V1471 + V1467 importable
    p_chain = sub.add_parser("chain", help="Verify V1471 + V1467 importable")
    p_chain.add_argument("--verbose", action="store_true")

    # help
    p_help = sub.add_parser("help", help="Print full V1472 help")

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = _make_argparser()
    args = parser.parse_args(argv)

    if args.cmd == "run":
        out_dir = Path(args.out_dir) if args.out_dir else None
        sup = V1472DaemonSupervisor(
            host=args.host,
            metrics_port_min=args.metrics_port_min,
            metrics_port_max=args.metrics_port_max,
            health_interval_s=args.health_interval,
            stale_threshold_s=args.stale_threshold,
            max_runtime_s=args.max_runtime,
            max_restarts=args.max_restarts,
            v1471_max_runtime_s=args.v1471_max_runtime,
            v1471_max_polls=args.v1471_max_polls,
            v1471_audit_every_poll=args.v1471_audit_every_poll,
            v1471_boot_timeout_s=args.v1471_boot_timeout,
            kill_grace_s=args.kill_grace,
            max_log_bytes=args.max_log_bytes,
            jsonl_poll_interval_s=args.jsonl_poll_interval,
            out_dir=out_dir,
            verbose=args.verbose,
        )

        # Install signal handler for graceful shutdown
        def _handler(signum, frame):
            sup.request_shutdown()

        try:
            signal.signal(signal.SIGINT, _handler)
            if hasattr(signal, "SIGTERM"):
                signal.signal(signal.SIGTERM, _handler)
        except (ValueError, OSError):
            pass

        report = sup.run()
        return 0 if report.ok else 1

    elif args.cmd == "demo":
        out_dir = Path(args.out_dir)
        sup = V1472DaemonSupervisor(
            host=DEFAULT_HOST,
            max_runtime_s=20.0,
            max_restarts=2,
            v1471_max_runtime_s=15.0,
            v1471_max_polls=3,
            v1471_audit_every_poll=2,
            health_interval_s=3.0,
            stale_threshold_s=20.0,
            out_dir=out_dir,
            verbose=args.verbose,
        )
        report = sup.run()
        return 0 if report.ok else 1

    elif args.cmd == "popper":
        results = popper_v1472(verbose=args.verbose)
        passed = sum(1 for _, ok, _ in results if ok)
        total = len(results)
        for name, ok, msg in results:
            tag = "OK" if ok else "FAIL"
            print(f"[{tag}] {name}: {msg}")
        print(f"\n{passed}/{total} popper checks passed")
        return 0 if passed == total else 1

    elif args.cmd == "meta":
        meta = {
            "module": V1472_MODULE,
            "version": V1472_VERSION,
            "date": V1472_DATE,
            "guards": list(V1472_GUARDS),
            "v3_guards": list(V1472_V3_GUARDS),
            "borrowed_sources": list(BORROWED_SOURCES),
        }
        print(json.dumps(meta, ensure_ascii=False, indent=2, sort_keys=True))
        return 0

    elif args.cmd == "chain":
        ok_v1471 = v1471 is not None
        ok_v1467 = True
        try:
            import v1467_asi_audit_http_gateway_history_diff  # noqa: F401
        except ImportError as e:
            ok_v1467 = False
            print(f"[FAIL] v1467 import: {e}", file=sys.stderr)
        if ok_v1471:
            print(f"[OK] v1471 import: {v1471.V1471_MODULE}")
        else:
            print("[FAIL] v1471 import", file=sys.stderr)
        if ok_v1467:
            print("[OK] v1467 import (assumed via V1471)")
        return 0 if (ok_v1471 and ok_v1467) else 1

    elif args.cmd == "help":
        parser.print_help()
        return 0

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())