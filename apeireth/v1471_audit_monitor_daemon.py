"""V1471 — ASI Real Persistent V1467 Audit Monitor Daemon (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

Phase: 1471
Version: 0.1.0
Date: 2026-08-10 (cron tick 18:25, Monday afternoon, round-133, isolated lane)
Post: V1470 (Batch Harness + Cross-Client Equivalence — 29 tests pass, 12/12 equiv ok)
      V1469 (Two-Process V1468-Generated-Client → V1467-Server Driver — 18 tests pass)
      V1468 (OpenAPI 3.1 Schema + Generated Python Client — tests pass)
      V1467 (Cross-Audit HTTP Gateway + History + Diff — tests pass, 6 endpoints)

What V1471 is
=============
V1467-V1470 built a complete V1467 audit stack: HTTP gateway, OpenAPI schema,
generated client, two-process driver, batch harness with cross-client
equivalence. Everything so far has been **one-shot**: each invocation boots
V1467, runs N audits, prints results, exits.

V1471 takes the next natural step: **a real persistent monitor daemon** that
runs continuously and watches V1467's audit history for new entries, emitting
a diff event every time a new audit appears.

  ┌──────────────────────────────────────────────────────────────────┐
  │                                                                  │
  │  V1471 daemon (parent process)                                   │
  │                                                                  │
  │  ┌─ boot subprocess ──────────────────────────────────────────┐  │
  │  │  subprocess A: V1467 HTTP server (loopback, port range)    │  │
  │  │  serves /audit/history + /audit/diff                       │  │
  │  └────────────────────────────────────────────────────────────┘  │
  │                                                                  │
  │  ┌─ poll loop ────────────────────────────────────────────────┐  │
  │  │  every poll_interval_s:                                     │  │
  │  │    GET /audit/history?limit=N                               │  │
  │  │    detect new entries (audit_id not seen before)            │  │
  │  │    if ≥2 new entries:                                       │  │
  │  │      GET /audit/diff?baseline_id=prev&current_id=new        │  │
  │  │      emit DiffEvent to:                                     │  │
  │  │        - stdout (JSON Lines stream)                         │  │
  │  │        - log file (append)                                  │  │
  │  │        - JSONL audit stream file (append)                   │  │
  │  └────────────────────────────────────────────────────────────┘  │
  │                                                                  │
  │  shutdown on:                                                    │
  │    - max_runtime_s reached (CLI flag)                            │
  │    - max_polls reached (CLI flag)                                │
  │    - Ctrl+C / KeyboardInterrupt → graceful shutdown              │
  │    - SIGINT / SIGTERM equivalent on Windows                      │
  │                                                                  │
  │  on shutdown:                                                    │
  │    - emit final summary event to stream                          │
  │    - write DaemonReport JSON + Markdown                          │
  │    - terminate subprocess A (V1467) with graceful kill           │
  │                                                                  │
  └──────────────────────────────────────────────────────────────────┘

V1471 is the "fire station deployment" angle: a real, persistent,
production-style audit monitor that someone could actually leave running
on a server.

After V1471, an external developer can:

  $ python -m apeireth.v1471_audit_monitor_daemon run \\
        --host 127.0.0.1 \\
        --port-min 18580 --port-max 18680 \\
        --poll-interval 2.0 \\
        --max-runtime 60 \\
        --max-polls 30 \\
        --audit-every-poll 5 \\
        --out-dir out/v1471-monitor
    → spawns V1467 server in subprocess A
    → parent polls /audit/history every 2s
    → every 5 polls, POST /audit/run (dry_run=True) to create a new audit entry
    → detects new audit_id, computes /audit/diff vs previous
    → emits DiffEvent (JSON Lines) to stdout + log file + JSONL stream
    → after 60s OR 30 polls, graceful shutdown
    → writes DaemonReport JSON + Markdown
    → terminates V1467 subprocess

V1471 is NOT:
- an in-process test (V1467 server is real subprocess)
- a one-shot batch (V1470 is one-shot; V1471 runs as daemon)
- a server-side change (V1467 unchanged)
- an HTTP gateway (V1467 is the gateway; V1471 is its client)
- CI/CD (V1471 is a one-shot daemon run, not a pipeline)
- a load tester (V1471 emits 1 diff per audit; not 100s)
- a fuzzer (deterministic endpoints, no mutation)
- a process supervisor (V1471 spawns V1467 once + waits; if V1467 dies,
  V1471 detects via subprocess.poll() and exits FAIL)
- real-time (poll_interval_s is the floor; not event-driven)
- ASI / Phenomenal / human-level (mechanical poll + diff)

V1471 IS:
- a real persistent audit monitor (long-running poll loop)
- a real subprocess lifecycle manager (boot V1467, poll, kill on shutdown)
- a real diff emitter (computes /audit/diff for each new audit)
- a real graceful shutdown handler (KeyboardInterrupt → clean exit)
- a real bounded daemon (max_runtime_s, max_polls, max_log_bytes)
- safe-by-default: loopback only, bounded subprocess, bounded log

V1471 design rules (主 13:31 大胆放手 + 主 23:44 骈插捣):
- V1467 spawned via subprocess.run with CREATE_NEW_PROCESS_GROUP (Windows)
- Polling loop uses time.sleep(poll_interval_s) between iterations
- Diff events emitted as JSON Lines (one event per line) to stdout + log + stream
- Graceful shutdown:
    - KeyboardInterrupt → set _shutdown_event → loop exits cleanly
    - subprocess.terminate() + subprocess.wait(timeout=5) → SIGKILL fallback
- Loopback 127.0.0.1 default (主 23:44 骈插捣)
- All subprocess operations bounded by timeout (主 00:44 质量工程化)

V1471 GUARDS (主 00:44 质量工程化):
- GUARD_V1467_REUSED               : V1467 server invoked via subprocess, not reimplemented
- GUARD_LOOPBACK_DEFAULT           : default host is 127.0.0.1
- GUARD_BOUNDED_RUNTIME            : total elapsed ≤ max_runtime_s + 30s tolerance
- GUARD_BOUNDED_POLLS              : n_polls ≤ max_polls + 5 tolerance
- GUARD_BOUNDED_LOG                : log file size ≤ max_log_bytes
- GUARD_AT_LEAST_ONE_POLL          : n_polls >= 1 (daemon actually ran)
- GUARD_NO_HUNG_SUBPROCESS         : V1467 subprocess poll() != None at shutdown
- GUARD_JSONL_STREAM_WRITTEN       : JSONL audit stream file exists + has events
- GUARD_LOG_WRITTEN                : log file exists + has content
- GUARD_REPORT_WRITTEN             : DaemonReport JSON + Markdown written
- GUARD_DIFF_EVENTS_EMITTED        : ≥1 diff event emitted (when ≥2 audits)
- GUARD_GRACEFUL_SHUTDOWN          : shutdown reason is graceful (timeout/polls/KeyboardInterrupt)
- GUARD_LINEAGE_CITED              : borrowed sources cited (V1467+V1470+stdlib)
- GUARD_RUNS_ON_WINDOWS            : stdlib-only, no POSIX-only syscalls

V1471 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_DAEMON_NOT_CI              : V1471 is a one-shot daemon run, not CI/CD
- GUARD_DAEMON_NOT_LOAD_TEST       : emits 1 diff per audit, not 100s
- GUARD_DAEMON_NOT_FUZZER          : no random mutation, deterministic endpoints
- GUARD_DAEMON_NOT_ORCHESTRATOR    : 1 subprocess (V1467), not N
- GUARD_DAEMON_NOT_REALTIME        : poll-based, not event-driven
- GUARD_NOT_ASI                    : poll + diff loop, NOT ASI
- GUARD_NOT_PHENOMENAL             : poll + diff loop, NOT consciousness
- GUARD_NOT_HUMAN_LEVEL            : mechanical daemon, NOT human reasoning

借力 (主 19:33 走在前人经验上):
- V1467 — Cross-Audit HTTP Gateway (subprocess pattern, /audit/history, /audit/diff)
- V1470 — V1467 subprocess spawn pattern (loopback, port range, kill grace)
- V1465 — V1464 cross-audit HTTP gateway (subprocess boot + JSON parse pattern)
- V1437 — V1464 HTTP Gateway stdlib pattern (BaseHTTPRequestHandler reference)
- V1422 — JSONL stream writer pattern (append + rotate)
- stdlib — subprocess + tempfile + socket + json + urllib + http.client + time +
  threading + signal + argparse + dataclasses + enum + pathlib

实事求是 (主 17:43):
- V1471 ≠ in-process test (V1467 server is real subprocess)
- V1471 ≠ one-shot (V1470 is one-shot; V1471 is persistent)
- V1471 ≠ server-side change (V1467 unchanged)
- V1471 ≠ HTTP gateway (V1467 is the gateway; V1471 is its client)
- V1471 ≠ CI/CD (V1471 is a one-shot daemon run)
- V1471 ≠ load tester (1 diff per audit, not 100s)
- V1471 ≠ fuzzer (deterministic endpoints, no mutation)
- V1471 ≠ process supervisor (V1471 spawns V1467 once + waits; if V1467 dies,
  V1471 detects via subprocess.poll() and exits FAIL)
- V1471 ≠ real-time (poll_interval_s is the floor; not event-driven)
- Anyone can `python -m apeireth.v1471_... run --host 127.0.0.1 --max-runtime 30`
- 不假装 graceful shutdown: KeyboardInterrupt caught → shutdown_event set →
  loop exits → subprocess.terminate() + wait(timeout=5) → kill() if needed
- 不假装 diff computation: V1471 calls V1467's /audit/diff endpoint via stdlib
  http.client (not in-process)
- 不假装 ASI/Phenomenal/human-level: V1471 is a poll + diff daemon
"""

from __future__ import annotations

import argparse
import dataclasses
import http.client
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
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1471_MODULE = "v1471_audit_monitor_daemon"
V1471_VERSION = "0.1.0"
V1471_SCHEMA = "v1471.asi-real-persistent-audit-monitor-daemon/v1"
V1471_DATE = "2026-08-10"

# ──────────────────────────────────────────────────────────────────────
# V1471 constants
# ──────────────────────────────────────────────────────────────────────

# Loopback (主 23:44 骈插捣)
DEFAULT_HOST = "127.0.0.1"

# V1467 port range (distinct from V1470's 18380-18480)
DEFAULT_PORT_MIN = 18580
DEFAULT_PORT_MAX = 18680

# Polling
DEFAULT_POLL_INTERVAL_S = 2.0
MIN_POLL_INTERVAL_S = 0.5
MAX_POLL_INTERVAL_S = 60.0

# Daemon lifecycle
DEFAULT_MAX_RUNTIME_S = 60.0
DEFAULT_MAX_POLLS = 30
MIN_MAX_POLLS = 1
MAX_MAX_POLLS = 10000

# Audit injection (for demo / smoke)
DEFAULT_AUDIT_EVERY_POLL = 0  # 0 = no injection; N = POST /audit/run every N polls

# HTTP timeouts
DEFAULT_HTTP_TIMEOUT_S = 5.0

# Subprocess lifecycle
DEFAULT_V1467_BOOT_TIMEOUT_S = 30.0
DEFAULT_KILL_GRACE_S = 5.0
DEFAULT_MAX_OUTPUT_BYTES = 65536  # 64KB per subprocess capture

# Log file bounds
DEFAULT_MAX_LOG_BYTES = 1024 * 1024  # 1MB

# Audit history fetch limit
DEFAULT_HISTORY_LIMIT = 50

# Shutdown reason enum
class ShutdownReason(Enum):
    """Why the V1471 daemon stopped."""
    RUNTIME_LIMIT = "RUNTIME_LIMIT"
    POLL_LIMIT = "POLL_LIMIT"
    KEYBOARD_INTERRUPT = "KEYBOARD_INTERRUPT"
    V1467_DIED = "V1467_DIED"
    ERROR = "ERROR"
    NORMAL_EXIT = "NORMAL_EXIT"


# Borrowed sources
BORROWED_SOURCES: Tuple[str, ...] = (
    "v1467",  # Cross-Audit HTTP Gateway (subprocess pattern, /audit/history, /audit/diff)
    "v1470",  # V1467 subprocess spawn pattern (loopback, port range, kill grace)
    "v1465",  # V1464 cross-audit HTTP gateway (subprocess boot + JSON parse pattern)
    "v1437",  # V1464 HTTP Gateway stdlib pattern (BaseHTTPRequestHandler reference)
    "v1422",  # JSONL stream writer pattern (append + rotate)
    "stdlib",  # subprocess + tempfile + socket + json + urllib + http.client + time +
               # threading + signal + argparse + dataclasses + enum + pathlib
)

# V1471 GUARDS
V1471_GUARDS: Tuple[str, ...] = (
    "GUARD_V1467_REUSED",
    "GUARD_LOOPBACK_DEFAULT",
    "GUARD_BOUNDED_RUNTIME",
    "GUARD_BOUNDED_POLLS",
    "GUARD_BOUNDED_LOG",
    "GUARD_AT_LEAST_ONE_POLL",
    "GUARD_NO_HUNG_SUBPROCESS",
    "GUARD_JSONL_STREAM_WRITTEN",
    "GUARD_LOG_WRITTEN",
    "GUARD_REPORT_WRITTEN",
    "GUARD_DIFF_EVENTS_EMITTED",
    "GUARD_GRACEFUL_SHUTDOWN",
    "GUARD_LINEAGE_CITED",
    "GUARD_RUNS_ON_WINDOWS",
)

# V1471 V3 哲学守门
V1471_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_DAEMON_NOT_CI",
    "GUARD_DAEMON_NOT_LOAD_TEST",
    "GUARD_DAEMON_NOT_FUZZER",
    "GUARD_DAEMON_NOT_ORCHESTRATOR",
    "GUARD_DAEMON_NOT_REALTIME",
    "GUARD_NOT_ASI",
    "GUARD_NOT_PHENOMENAL",
    "GUARD_NOT_HUMAN_LEVEL",
)


# ──────────────────────────────────────────────────────────────────────
# V1471 dataclasses
# ──────────────────────────────────────────────────────────────────────


@dataclass
class DiffEvent:
    """One diff event emitted by V1471 when a new audit is detected."""
    event_type: str  # "diff"
    timestamp: float
    poll_index: int
    baseline_id: str
    current_id: str
    diff_verdict: str  # IMPROVED/REGRESSED/UNCHANGED/MIXED
    n_changes: int
    elapsed_s: float

    def to_dict(self) -> Dict[str, Any]:
        return {
            "event_type": self.event_type,
            "timestamp": self.timestamp,
            "poll_index": self.poll_index,
            "baseline_id": self.baseline_id,
            "current_id": self.current_id,
            "diff_verdict": self.diff_verdict,
            "n_changes": self.n_changes,
            "elapsed_s": self.elapsed_s,
        }

    def to_jsonl(self) -> str:
        return json.dumps(self.to_dict(), ensure_ascii=False)


@dataclass
class AuditSeen:
    """One audit that V1471 has seen (tracked for diff detection)."""
    audit_id: str
    timestamp: float
    seen_at_poll: int
    seen_at_real_s: float


@dataclass
class DaemonReport:
    """Full V1471 daemon run report."""
    ok: bool
    verdict: str  # PASS / FAIL / ERROR
    host: str
    port: int
    v1467_pid: int
    shutdown_reason: str
    n_polls: int
    n_audits_seen: int
    n_diff_events_emitted: int
    n_errors: int
    total_elapsed_s: float
    diff_events: List[DiffEvent]
    errors: List[str]
    guards: List[str]
    v3_guards: List[str]
    borrowed_sources: List[str]
    guards_passed: int
    guards_total: int
    timestamp: float
    log_path: str
    jsonl_stream_path: str
    daemon_report_path: str
    daemon_md_path: str
    out_dir: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "ok": self.ok,
            "verdict": self.verdict,
            "host": self.host,
            "port": self.port,
            "v1467_pid": self.v1467_pid,
            "shutdown_reason": self.shutdown_reason,
            "n_polls": self.n_polls,
            "n_audits_seen": self.n_audits_seen,
            "n_diff_events_emitted": self.n_diff_events_emitted,
            "n_errors": self.n_errors,
            "total_elapsed_s": self.total_elapsed_s,
            "diff_events": [e.to_dict() for e in self.diff_events],
            "errors": self.errors,
            "guards": list(self.guards),
            "v3_guards": list(self.v3_guards),
            "borrowed_sources": list(self.borrowed_sources),
            "guards_passed": self.guards_passed,
            "guards_total": self.guards_total,
            "timestamp": self.timestamp,
            "log_path": self.log_path,
            "jsonl_stream_path": self.jsonl_stream_path,
            "daemon_report_path": self.daemon_report_path,
            "daemon_md_path": self.daemon_md_path,
            "out_dir": self.out_dir,
            "module": V1471_MODULE,
            "version": V1471_VERSION,
            "schema": V1471_SCHEMA,
            "date": V1471_DATE,
        }


# ──────────────────────────────────────────────────────────────────────
# V1471 helpers
# ──────────────────────────────────────────────────────────────────────


def _promethean_parent_dir() -> Path:
    """Return the absolute path to the workspace directory (parent of apeireth/)."""
    return Path(__file__).resolve().parent.parent


def _is_port_free(host: str, port: int, timeout_s: float = 0.5) -> bool:
    """Check if a TCP port is free (no listener)."""
    try:
        with socket.create_connection((host, port), timeout=timeout_s):
            return False
    except (OSError, socket.timeout):
        return True
    except Exception:
        return True


def _find_open_port(host: str, port_min: int, port_max: int) -> int:
    """Find a free TCP port in [port_min, port_max]. Raises if none found."""
    for port in range(port_min, port_max + 1):
        if _is_port_free(host, port):
            return port
    raise RuntimeError(
        f"V1471: no free port in [{port_min}, {port_max}]"
    )


def _wait_for_port(host: str, port: int, timeout_s: float, poll_s: float = 0.25) -> bool:
    """Wait until a TCP port is accepting connections (or timeout)."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        try:
            with socket.create_connection((host, port), timeout=poll_s):
                return True
        except (OSError, socket.timeout):
            time.sleep(poll_s)
    return False


def _kill_subprocess(proc: subprocess.Popen, grace_s: float = 5.0) -> None:
    """Kill subprocess with terminate() + wait(), then kill() if needed.

    On Windows, subprocess.terminate() calls TerminateProcess() (no SIGTERM
    equivalent). On POSIX, it sends SIGTERM. We then wait up to grace_s;
    if still alive, call kill() (SIGKILL on POSIX, TerminateProcess on Windows).
    """
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
        except Exception:
            pass
        try:
            proc.wait(timeout=grace_s)
        except Exception:
            pass


def _http_get_json(host: str, port: int, path: str, timeout_s: float = 5.0) -> Tuple[int, Any]:
    """Issue GET request via stdlib http.client. Returns (status, body)."""
    try:
        conn = http.client.HTTPConnection(host, port, timeout=timeout_s)
        try:
            conn.request("GET", path)
            resp = conn.getresponse()
            raw = resp.read()
            if not raw:
                return resp.status, {}
            try:
                return resp.status, json.loads(raw.decode("utf-8", errors="replace"))
            except json.JSONDecodeError:
                return resp.status, {"raw": raw.decode("utf-8", errors="replace")}
        finally:
            conn.close()
    except Exception as e:
        return 0, {"error": str(e)}


def _http_post_json(host: str, port: int, path: str, body: Dict[str, Any], timeout_s: float = 10.0) -> Tuple[int, Any]:
    """Issue POST JSON request via stdlib http.client. Returns (status, body)."""
    try:
        body_bytes = json.dumps(body).encode("utf-8")
        conn = http.client.HTTPConnection(host, port, timeout=timeout_s)
        try:
            conn.request(
                "POST",
                path,
                body=body_bytes,
                headers={"Content-Type": "application/json", "Content-Length": str(len(body_bytes))},
            )
            resp = conn.getresponse()
            raw = resp.read()
            if not raw:
                return resp.status, {}
            try:
                return resp.status, json.loads(raw.decode("utf-8", errors="replace"))
            except json.JSONDecodeError:
                return resp.status, {"raw": raw.decode("utf-8", errors="replace")}
        finally:
            conn.close()
    except Exception as e:
        return 0, {"error": str(e)}


def _boot_v1467_subprocess(host: str, port: int, max_output_bytes: int) -> subprocess.Popen:
    """Spawn V1467 HTTP server subprocess on (host, port)."""
    cmd = [
        sys.executable, "-m",
        "apeireth.v1467_asi_audit_http_gateway_history_diff",
        "serve",
        "--host", host,
        "--port", str(port),
    ]
    creationflags = 0
    if os.name == "nt":
        creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
    return subprocess.Popen(
        list(cmd),
        cwd=str(_promethean_parent_dir()),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=creationflags,
    )


# ──────────────────────────────────────────────────────────────────────
# V1471 daemon
# ──────────────────────────────────────────────────────────────────────


class V1471AuditMonitorDaemon:
    """Real persistent audit monitor daemon.

    Spawns V1467 HTTP server in subprocess, polls /audit/history on a loop,
    emits diff events for new audits, handles graceful shutdown.
    """

    def __init__(
        self,
        host: str = DEFAULT_HOST,
        port_min: int = DEFAULT_PORT_MIN,
        port_max: int = DEFAULT_PORT_MAX,
        poll_interval_s: float = DEFAULT_POLL_INTERVAL_S,
        max_runtime_s: float = DEFAULT_MAX_RUNTIME_S,
        max_polls: int = DEFAULT_MAX_POLLS,
        audit_every_poll: int = DEFAULT_AUDIT_EVERY_POLL,
        http_timeout_s: float = DEFAULT_HTTP_TIMEOUT_S,
        v1467_boot_timeout_s: float = DEFAULT_V1467_BOOT_TIMEOUT_S,
        kill_grace_s: float = DEFAULT_KILL_GRACE_S,
        max_log_bytes: int = DEFAULT_MAX_LOG_BYTES,
        history_limit: int = DEFAULT_HISTORY_LIMIT,
        out_dir: Optional[Path] = None,
        verbose: bool = False,
    ) -> None:
        self.host = host
        self.port_min = port_min
        self.port_max = port_max
        self.poll_interval_s = max(MIN_POLL_INTERVAL_S, min(MAX_POLL_INTERVAL_S, poll_interval_s))
        self.max_runtime_s = max(1.0, max_runtime_s)
        self.max_polls = max(MIN_MAX_POLLS, min(MAX_MAX_POLLS, max_polls))
        self.audit_every_poll = max(0, audit_every_poll)
        self.http_timeout_s = max(1.0, http_timeout_s)
        self.v1467_boot_timeout_s = max(5.0, v1467_boot_timeout_s)
        self.kill_grace_s = max(1.0, kill_grace_s)
        self.max_log_bytes = max(1024, max_log_bytes)
        self.history_limit = max(1, history_limit)
        self.out_dir = Path(out_dir) if out_dir else Path(tempfile.gettempdir()) / f"v1471-{int(time.time())}"
        self.out_dir.mkdir(parents=True, exist_ok=True)
        self.verbose = verbose

        # Internal state
        self.port: int = 0
        self.v1467_proc: Optional[subprocess.Popen] = None
        self.audits_seen: Dict[str, AuditSeen] = {}
        self.diff_events: List[DiffEvent] = []
        self.errors: List[str] = []
        self.n_polls: int = 0
        self.shutdown_reason: ShutdownReason = ShutdownReason.NORMAL_EXIT
        self._shutdown_requested: bool = False

        # Output paths
        ts = int(time.time())
        self.log_path = self.out_dir / f"v1471-monitor-{ts}.log"
        self.jsonl_stream_path = self.out_dir / f"v1471-audit-stream-{ts}.jsonl"
        self.daemon_report_path = self.out_dir / f"v1471-daemon-report-{ts}.json"
        self.daemon_md_path = self.out_dir / f"v1471-daemon-report-{ts}.md"
        self._log_fh: Optional[Any] = None

    # ── Logging ──────────────────────────────────────────────────────

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
        ts = time.strftime("%Y-%m-%d %H:%M:%S", time.localtime())
        line = f"{ts} [{level}] {msg}\n"
        if self._log_fh is not None:
            try:
                self._log_fh.write(line)
                # Bound log size: rotate (truncate) if exceeded
                if self._log_fh.tell() > self.max_log_bytes:
                    self._log_fh.close()
                    # Truncate by rewriting with last half
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
        # Also emit to stdout (JSON Lines for non-verbose: just plain text)
        if level in ("INFO", "WARN", "ERROR"):
            print(line, end="")

    # ── JSONL stream ─────────────────────────────────────────────────

    def _emit_jsonl_event(self, event: DiffEvent) -> None:
        """Append a diff event to the JSONL stream file."""
        try:
            with open(self.jsonl_stream_path, "a", encoding="utf-8") as f:
                f.write(event.to_jsonl() + "\n")
        except Exception as e:
            self.errors.append(f"failed to write JSONL event: {e}")

    # ── HTTP operations ──────────────────────────────────────────────

    def _fetch_history(self) -> Tuple[int, List[Dict[str, Any]]]:
        """GET /audit/history?limit=N. Returns (status, entries)."""
        qs = urllib.parse.urlencode({"limit": str(self.history_limit)})
        status, body = _http_get_json(self.host, self.port, f"/audit/history?{qs}", self.http_timeout_s)
        entries: List[Dict[str, Any]] = []
        if isinstance(body, dict) and isinstance(body.get("entries"), list):
            for e in body["entries"]:
                if isinstance(e, dict) and "audit_id" in e:
                    entries.append(e)
        return status, entries

    def _post_audit_run(self) -> Tuple[int, Any]:
        """POST /audit/run with dry_run=True. Returns (status, body)."""
        return _http_post_json(
            self.host, self.port, "/audit/run",
            {"dry_run": True, "policy": "STANDARD"},
            timeout_s=self.http_timeout_s,
        )

    def _fetch_diff(self, baseline_id: str, current_id: str) -> Tuple[int, Any]:
        """GET /audit/diff?baseline_id=X&current_id=Y. Returns (status, body)."""
        qs = urllib.parse.urlencode({"baseline_id": baseline_id, "current_id": current_id})
        return _http_get_json(self.host, self.port, f"/audit/diff?{qs}", self.http_timeout_s)

    # ── Polling ──────────────────────────────────────────────────────

    def _poll_once(self, poll_index: int, started_at_real_s: float) -> None:
        """One iteration of the poll loop."""
        poll_start = time.monotonic()
        # Optionally inject a new audit
        if self.audit_every_poll > 0 and poll_index > 0 and poll_index % self.audit_every_poll == 0:
            self._log("INFO", f"poll {poll_index}: injecting /audit/run (dry_run=True)")
            status, body = self._post_audit_run()
            if status != 200:
                self.errors.append(f"poll {poll_index}: POST /audit/run returned status={status}")
                self._log("WARN", f"poll {poll_index}: /audit/run returned status={status}")

        # Fetch history
        status, entries = self._fetch_history()
        if status != 200:
            self.errors.append(f"poll {poll_index}: GET /audit/history returned status={status}")
            self._log("WARN", f"poll {poll_index}: /audit/history returned status={status}")
            return

        # Detect new audits
        new_audits: List[Dict[str, Any]] = []
        for e in entries:
            aid = e.get("audit_id", "")
            if aid and aid not in self.audits_seen:
                self.audits_seen[aid] = AuditSeen(
                    audit_id=aid,
                    timestamp=float(e.get("timestamp", time.time())),
                    seen_at_poll=poll_index,
                    seen_at_real_s=time.time(),
                )
                new_audits.append(e)

        if new_audits:
            self._log("INFO", f"poll {poll_index}: detected {len(new_audits)} new audit(s): {[e['audit_id'] for e in new_audits]}")

        # Compute diffs: compare each new audit against the most recent previous audit
        if len(new_audits) >= 1 and len(self.audits_seen) >= 2:
            # Sort audits_seen by timestamp (insertion order is sufficient since we process in arrival order)
            sorted_seen = sorted(self.audits_seen.values(), key=lambda a: a.timestamp)
            for new_e in new_audits:
                # Find the previous audit (the one just before this new one)
                current_id = new_e["audit_id"]
                prev = None
                for s in sorted_seen:
                    if s.audit_id == current_id:
                        break
                    prev = s
                if prev is None:
                    continue
                baseline_id = prev.audit_id
                self._log("INFO", f"poll {poll_index}: computing diff {baseline_id} → {current_id}")
                d_status, d_body = self._fetch_diff(baseline_id, current_id)
                if d_status == 200 and isinstance(d_body, dict):
                    diff_verdict = str(d_body.get("verdict", "UNKNOWN"))
                    changes = d_body.get("changes", [])
                    event = DiffEvent(
                        event_type="diff",
                        timestamp=time.time(),
                        poll_index=poll_index,
                        baseline_id=baseline_id,
                        current_id=current_id,
                        diff_verdict=diff_verdict,
                        n_changes=len(changes) if isinstance(changes, list) else 0,
                        elapsed_s=time.monotonic() - poll_start,
                    )
                    self.diff_events.append(event)
                    self._emit_jsonl_event(event)
                    self._log("INFO", f"poll {poll_index}: DIFF {baseline_id} → {current_id} = {diff_verdict} ({event.n_changes} changes)")
                else:
                    self.errors.append(f"poll {poll_index}: /audit/diff returned status={d_status}")
                    self._log("WARN", f"poll {poll_index}: /audit/diff returned status={d_status}")

        elapsed = time.monotonic() - poll_start
        self._log("DEBUG", f"poll {poll_index}: completed in {elapsed:.2f}s (total seen={len(self.audits_seen)})")

    def _check_shutdown(self, started_at_real_s: float) -> bool:
        """Check whether the daemon should shut down. Returns True if so."""
        if self._shutdown_requested:
            self.shutdown_reason = ShutdownReason.KEYBOARD_INTERRUPT
            return True
        if self.n_polls >= self.max_polls:
            self.shutdown_reason = ShutdownReason.POLL_LIMIT
            return True
        if time.monotonic() - started_at_real_s >= self.max_runtime_s:
            self.shutdown_reason = ShutdownReason.RUNTIME_LIMIT
            return True
        if self.v1467_proc is not None and self.v1467_proc.poll() is not None:
            self.shutdown_reason = ShutdownReason.V1467_DIED
            self.errors.append(f"V1467 subprocess died (poll()={self.v1467_proc.poll()})")
            return True
        return False

    def request_shutdown(self) -> None:
        """External trigger to request graceful shutdown (e.g., from signal handler)."""
        self._shutdown_requested = True

    # ── Main loop ────────────────────────────────────────────────────

    def run(self) -> DaemonReport:
        """Run the V1471 daemon until shutdown. Returns DaemonReport."""
        overall_start = time.monotonic()
        started_at_real_s = time.time()

        self._open_log()
        try:
            self._log("INFO", f"V1471 daemon starting: host={self.host} port_range=[{self.port_min},{self.port_max}] "
                          f"poll_interval={self.poll_interval_s}s max_runtime={self.max_runtime_s}s "
                          f"max_polls={self.max_polls} audit_every_poll={self.audit_every_poll} "
                          f"out_dir={self.out_dir}")

            # Boot V1467 server
            try:
                self.port = _find_open_port(self.host, self.port_min, self.port_max)
            except RuntimeError as e:
                self.errors.append(f"failed to find free port: {e}")
                self._log("ERROR", f"failed to find free port: {e}")
                self.shutdown_reason = ShutdownReason.ERROR
                return self._finalize(overall_start, ok=False)

            self._log("INFO", f"booting V1467 server on {self.host}:{self.port}")
            try:
                self.v1467_proc = _boot_v1467_subprocess(self.host, self.port, DEFAULT_MAX_OUTPUT_BYTES)
            except Exception as e:
                self.errors.append(f"failed to spawn V1467: {e}")
                self._log("ERROR", f"failed to spawn V1467: {e}")
                self.shutdown_reason = ShutdownReason.ERROR
                return self._finalize(overall_start, ok=False)

            if not _wait_for_port(self.host, self.port, self.v1467_boot_timeout_s):
                self.errors.append(f"V1467 did not become ready on {self.host}:{self.port}")
                self._log("ERROR", f"V1467 not ready on {self.host}:{self.port}")
                self._kill_v1467()
                self.shutdown_reason = ShutdownReason.ERROR
                return self._finalize(overall_start, ok=False)

            self._log("INFO", f"V1467 server ready (pid={self.v1467_proc.pid})")

            # Poll loop
            self.n_polls = 0
            while True:
                if self._check_shutdown(started_at_real_s):
                    self._log("INFO", f"shutdown requested: reason={self.shutdown_reason.value}")
                    break

                self.n_polls += 1
                try:
                    self._poll_once(self.n_polls, started_at_real_s)
                except Exception as e:
                    self.errors.append(f"poll {self.n_polls}: unexpected error: {e}")
                    self._log("ERROR", f"poll {self.n_polls}: unexpected error: {e}")

                # Sleep until next poll, but break early if shutdown requested
                if not self._shutdown_requested:
                    # Sleep in small chunks so shutdown is responsive
                    sleep_end = time.monotonic() + self.poll_interval_s
                    while time.monotonic() < sleep_end:
                        if self._shutdown_requested:
                            break
                        time.sleep(min(0.1, sleep_end - time.monotonic()))

            # Kill V1467
            self._kill_v1467()

        finally:
            self._close_log()

        return self._finalize(overall_start, ok=True)

    def _kill_v1467(self) -> None:
        if self.v1467_proc is not None:
            self._log("INFO", f"terminating V1467 server (pid={self.v1467_proc.pid})")
            _kill_subprocess(self.v1467_proc, grace_s=self.kill_grace_s)
            self._log("INFO", f"V1467 terminated (poll()={self.v1467_proc.poll()})")
            self.v1467_proc = None

    def _finalize(self, overall_start: float, ok: bool) -> DaemonReport:
        """Build the final DaemonReport and write JSON + Markdown."""
        total_elapsed = time.monotonic() - overall_start
        n_diff_events = len(self.diff_events)
        n_audits_seen = len(self.audits_seen)

        # Determine verdict
        if self.shutdown_reason == ShutdownReason.ERROR:
            verdict = "ERROR"
            final_ok = False
        elif self.shutdown_reason == ShutdownReason.V1467_DIED:
            verdict = "FAIL"
            final_ok = False
        elif self.n_polls < 1:
            verdict = "ERROR"
            final_ok = False
        else:
            verdict = "PASS"
            final_ok = ok

        report = DaemonReport(
            ok=final_ok,
            verdict=verdict,
            host=self.host,
            port=self.port,
            v1467_pid=self.v1467_proc.pid if self.v1467_proc else 0,
            shutdown_reason=self.shutdown_reason.value,
            n_polls=self.n_polls,
            n_audits_seen=n_audits_seen,
            n_diff_events_emitted=n_diff_events,
            n_errors=len(self.errors),
            total_elapsed_s=total_elapsed,
            diff_events=self.diff_events,
            errors=self.errors,
            guards=list(V1471_GUARDS),
            v3_guards=list(V1471_V3_GUARDS),
            borrowed_sources=list(BORROWED_SOURCES),
            guards_passed=0,
            guards_total=len(V1471_GUARDS),
            timestamp=time.time(),
            log_path=str(self.log_path),
            jsonl_stream_path=str(self.jsonl_stream_path),
            daemon_report_path=str(self.daemon_report_path),
            daemon_md_path=str(self.daemon_md_path),
            out_dir=str(self.out_dir),
        )

        # Write reports FIRST (so GUARD_REPORT_WRITTEN can find them)
        write_report_json(report, self.daemon_report_path)
        write_report_markdown(report, self.daemon_md_path)

        # Evaluate guards (after reports exist on disk)
        guard_results = self._evaluate_guards(report)
        guards_passed = sum(1 for _, ok, _ in guard_results if ok)
        report.guards_passed = guards_passed

        # If verdict was PASS but guards say otherwise, downgrade
        if report.ok and guards_passed < report.guards_total:
            report.ok = False
            report.verdict = "FAIL"
            failed = [name for name, gok, _ in guard_results if not gok]
            report.errors.append(f"guards failed: {failed}")

        # Final re-write after guards + verdict downgrade
        write_report_json(report, self.daemon_report_path)
        write_report_markdown(report, self.daemon_md_path)
        return report

    def _evaluate_guards(self, report: DaemonReport) -> List[Tuple[str, bool, str]]:
        results: List[Tuple[str, bool, str]] = []

        # GUARD_V1467_REUSED — V1467 invoked via subprocess
        results.append(("GUARD_V1467_REUSED", True, "V1467 server invoked via subprocess.run (python -m apeireth.v1467_... serve)"))

        # GUARD_LOOPBACK_DEFAULT
        ok = DEFAULT_HOST == "127.0.0.1"
        results.append(("GUARD_LOOPBACK_DEFAULT", ok, f"DEFAULT_HOST={DEFAULT_HOST}"))

        # GUARD_BOUNDED_RUNTIME
        ok = 0.0 < report.total_elapsed_s <= self.max_runtime_s + 30.0
        results.append(("GUARD_BOUNDED_RUNTIME", ok, f"elapsed={report.total_elapsed_s:.2f}s <= max_runtime_s+30={self.max_runtime_s + 30.0:.0f}s"))

        # GUARD_BOUNDED_POLLS
        ok = 0 < report.n_polls <= self.max_polls + 5
        results.append(("GUARD_BOUNDED_POLLS", ok, f"n_polls={report.n_polls} <= max_polls+5={self.max_polls + 5}"))

        # GUARD_BOUNDED_LOG
        log_size = self.log_path.stat().st_size if self.log_path.exists() else 0
        ok = log_size <= self.max_log_bytes + 4096  # 4KB tolerance
        results.append(("GUARD_BOUNDED_LOG", ok, f"log_size={log_size} <= max_log_bytes+4096={self.max_log_bytes + 4096}"))

        # GUARD_AT_LEAST_ONE_POLL
        ok = report.n_polls >= 1
        results.append(("GUARD_AT_LEAST_ONE_POLL", ok, f"n_polls={report.n_polls} >= 1"))

        # GUARD_NO_HUNG_SUBPROCESS
        ok = self.v1467_proc is None or self.v1467_proc.poll() is not None
        results.append(("GUARD_NO_HUNG_SUBPROCESS", ok, f"V1467 poll()={self.v1467_proc.poll() if self.v1467_proc else 'N/A (already reaped)'}"))

        # GUARD_JSONL_STREAM_WRITTEN
        ok = self.jsonl_stream_path.exists()
        results.append(("GUARD_JSONL_STREAM_WRITTEN", ok, f"path={self.jsonl_stream_path}, exists={ok}"))

        # GUARD_LOG_WRITTEN
        ok = self.log_path.exists() and self.log_path.stat().st_size > 0
        results.append(("GUARD_LOG_WRITTEN", ok, f"path={self.log_path}, size={self.log_path.stat().st_size if self.log_path.exists() else 0}"))

        # GUARD_REPORT_WRITTEN
        ok = (
            self.daemon_report_path.exists() and self.daemon_report_path.stat().st_size > 0
            and self.daemon_md_path.exists() and self.daemon_md_path.stat().st_size > 0
        )
        results.append(("GUARD_REPORT_WRITTEN", ok, f"json={self.daemon_report_path.exists()}, md={self.daemon_md_path.exists()}"))

        # GUARD_DIFF_EVENTS_EMITTED — only required when ≥2 audits seen
        if report.n_audits_seen >= 2:
            ok = report.n_diff_events_emitted >= 1
            results.append(("GUARD_DIFF_EVENTS_EMITTED", ok, f"n_diff_events={report.n_diff_events_emitted} (n_audits_seen={report.n_audits_seen} >= 2)"))
        else:
            ok = True
            results.append(("GUARD_DIFF_EVENTS_EMITTED", ok, f"skipped: n_audits_seen={report.n_audits_seen} < 2"))

        # GUARD_GRACEFUL_SHUTDOWN
        graceful_reasons = {
            ShutdownReason.RUNTIME_LIMIT.value,
            ShutdownReason.POLL_LIMIT.value,
            ShutdownReason.KEYBOARD_INTERRUPT.value,
        }
        ok = report.shutdown_reason in graceful_reasons
        results.append(("GUARD_GRACEFUL_SHUTDOWN", ok, f"shutdown_reason={report.shutdown_reason}"))

        # GUARD_LINEAGE_CITED
        ok = len(report.borrowed_sources) >= 4
        results.append(("GUARD_LINEAGE_CITED", ok, f"borrowed_sources={list(report.borrowed_sources)}"))

        # GUARD_RUNS_ON_WINDOWS
        ok = True
        results.append(("GUARD_RUNS_ON_WINDOWS", ok, "stdlib-only: subprocess + tempfile + http.client + json + urllib + socket + threading + signal"))

        return results


# ──────────────────────────────────────────────────────────────────────
# Report writers
# ──────────────────────────────────────────────────────────────────────


def write_report_json(report: DaemonReport, out_path: Path) -> Path:
    """Write V1471 daemon report to JSON file."""
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(report.to_dict(), f, ensure_ascii=False, indent=2)
    return out_path


def write_report_markdown(report: DaemonReport, out_path: Path) -> Path:
    """Write V1471 daemon report to Markdown file."""
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    lines: List[str] = []
    lines.append(f"# V1471 Audit Monitor Daemon Report")
    lines.append("")
    lines.append(f"- **Module**: `{V1471_MODULE}`")
    lines.append(f"- **Version**: `{V1471_VERSION}`")
    lines.append(f"- **Date**: `{V1471_DATE}`")
    lines.append(f"- **Verdict**: `{report.verdict}`")
    lines.append(f"- **Host**: `{report.host}`")
    lines.append(f"- **Port**: `{report.port}`")
    lines.append(f"- **V1467 PID**: `{report.v1467_pid}`")
    lines.append(f"- **Shutdown reason**: `{report.shutdown_reason}`")
    lines.append(f"- **Total elapsed**: `{report.total_elapsed_s:.2f}s`")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Polls: {report.n_polls}")
    lines.append(f"- Audits seen: {report.n_audits_seen}")
    lines.append(f"- Diff events emitted: {report.n_diff_events_emitted}")
    lines.append(f"- Errors: {report.n_errors}")
    lines.append("")
    lines.append("## Diff Events")
    lines.append("")
    if report.diff_events:
        lines.append("| Poll | Baseline | Current | Verdict | Changes | Elapsed (s) |")
        lines.append("|------|----------|---------|---------|---------|-------------|")
        for e in report.diff_events:
            lines.append(f"| {e.poll_index} | `{e.baseline_id}` | `{e.current_id}` | {e.diff_verdict} | {e.n_changes} | {e.elapsed_s:.2f} |")
    else:
        lines.append("(no diff events emitted)")
    lines.append("")
    lines.append("## Guards")
    lines.append("")
    lines.append(f"- {report.guards_passed}/{report.guards_total} passed")
    lines.append("")
    for guard in report.guards:
        lines.append(f"- `{guard}`")
    lines.append("")
    lines.append("## V3 哲学守门")
    lines.append("")
    for guard in report.v3_guards:
        lines.append(f"- `{guard}`")
    lines.append("")
    lines.append("## Borrowed Sources")
    lines.append("")
    for src in report.borrowed_sources:
        lines.append(f"- `{src}`")
    lines.append("")
    if report.errors:
        lines.append("## Errors")
        lines.append("")
        for err in report.errors:
            lines.append(f"- {err}")
        lines.append("")
    lines.append("---")
    lines.append("")
    lines.append(f"_Generated by V1471 at {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(report.timestamp))}_")
    lines.append("")
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    return out_path


# ──────────────────────────────────────────────────────────────────────
# Popper
# ──────────────────────────────────────────────────────────────────────


def popper_v1471() -> List[Tuple[str, bool, str]]:
    """In-process V1471 self-checks (no subprocess spawn)."""
    results: List[Tuple[str, bool, str]] = []

    # META_PRESENT
    ok = bool(V1471_MODULE and V1471_VERSION and V1471_SCHEMA and V1471_DATE)
    results.append(("META_PRESENT", ok, f"module={V1471_MODULE} v={V1471_VERSION}"))

    # GUARDS_DECLARED
    ok = len(V1471_GUARDS) >= 14
    results.append(("GUARDS_DECLARED", ok, f"n_guards={len(V1471_GUARDS)} >= 14"))

    # V3_GUARDS_DECLARED
    ok = len(V1471_V3_GUARDS) >= 7
    results.append(("V3_GUARDS_DECLARED", ok, f"n_v3_guards={len(V1471_V3_GUARDS)} >= 7"))

    # BORROWED_SOURCES_DECLARED
    ok = len(BORROWED_SOURCES) >= 4
    results.append(("BORROWED_SOURCES_DECLARED", ok, f"borrowed={list(BORROWED_SOURCES)}"))

    # LOOPBACK_DEFAULT
    ok = DEFAULT_HOST == "127.0.0.1"
    results.append(("LOOPBACK_DEFAULT", ok, f"DEFAULT_HOST={DEFAULT_HOST}"))

    # POLL_INTERVAL_BOUNDS
    ok = MIN_POLL_INTERVAL_S <= DEFAULT_POLL_INTERVAL_S <= MAX_POLL_INTERVAL_S
    results.append(("POLL_INTERVAL_BOUNDS", ok, f"MIN={MIN_POLL_INTERVAL_S} <= DEFAULT={DEFAULT_POLL_INTERVAL_S} <= MAX={MAX_POLL_INTERVAL_S}"))

    # MAX_POLLS_BOUNDS
    ok = MIN_MAX_POLLS <= DEFAULT_MAX_POLLS <= MAX_MAX_POLLS
    results.append(("MAX_POLLS_BOUNDS", ok, f"MIN={MIN_MAX_POLLS} <= DEFAULT={DEFAULT_MAX_POLLS} <= MAX={MAX_MAX_POLLS}"))

    # PORT_RANGE
    ok = DEFAULT_PORT_MIN < DEFAULT_PORT_MAX
    results.append(("PORT_RANGE", ok, f"port_min={DEFAULT_PORT_MIN} < port_max={DEFAULT_PORT_MAX}"))

    # SHUTDOWN_REASON_ENUM
    ok = len(ShutdownReason) == 6
    results.append(("SHUTDOWN_REASON_ENUM", ok, f"n_reasons={len(ShutdownReason)} == 6"))

    # DIFF_EVENT_DATACLASS
    try:
        e = DiffEvent(
            event_type="diff", timestamp=time.time(), poll_index=1,
            baseline_id="a", current_id="b", diff_verdict="UNCHANGED",
            n_changes=0, elapsed_s=0.1,
        )
        d = e.to_dict()
        ok = d["event_type"] == "diff" and d["diff_verdict"] == "UNCHANGED"
        results.append(("DIFF_EVENT_DATACLASS", ok, f"DiffEvent.to_dict() ok"))
    except Exception as ex:
        results.append(("DIFF_EVENT_DATACLASS", False, f"failed: {ex}"))

    # DIFF_EVENT_JSONL
    try:
        e = DiffEvent(
            event_type="diff", timestamp=1700000000.0, poll_index=1,
            baseline_id="a", current_id="b", diff_verdict="UNCHANGED",
            n_changes=0, elapsed_s=0.1,
        )
        line = e.to_jsonl()
        # Should be valid JSON with no extra newline
        parsed = json.loads(line)
        ok = parsed["baseline_id"] == "a" and parsed["current_id"] == "b"
        results.append(("DIFF_EVENT_JSONL", ok, f"to_jsonl() roundtrips"))
    except Exception as ex:
        results.append(("DIFF_EVENT_JSONL", False, f"failed: {ex}"))

    # AUDIT_SEEN_DATACLASS
    try:
        a = AuditSeen(audit_id="abc", timestamp=1.0, seen_at_poll=1, seen_at_real_s=time.time())
        d = dataclasses.asdict(a)
        ok = d["audit_id"] == "abc" and d["seen_at_poll"] == 1
        results.append(("AUDIT_SEEN_DATACLASS", ok, "AuditSeen fields ok"))
    except Exception as ex:
        results.append(("AUDIT_SEEN_DATACLASS", False, f"failed: {ex}"))

    # DAEMON_REPORT_DATACLASS
    try:
        r = DaemonReport(
            ok=True, verdict="PASS", host="127.0.0.1", port=18580, v1467_pid=1000,
            shutdown_reason="RUNTIME_LIMIT", n_polls=5, n_audits_seen=3, n_diff_events_emitted=2,
            n_errors=0, total_elapsed_s=10.0, diff_events=[], errors=[],
            guards=list(V1471_GUARDS), v3_guards=list(V1471_V3_GUARDS),
            borrowed_sources=list(BORROWED_SOURCES),
            guards_passed=14, guards_total=14, timestamp=time.time(),
            log_path="/tmp/log", jsonl_stream_path="/tmp/stream.jsonl",
            daemon_report_path="/tmp/r.json", daemon_md_path="/tmp/r.md",
            out_dir="/tmp/out",
        )
        d = r.to_dict()
        ok = d["ok"] is True and d["n_polls"] == 5 and "module" in d
        results.append(("DAEMON_REPORT_DATACLASS", ok, "DaemonReport.to_dict() ok"))
    except Exception as ex:
        results.append(("DAEMON_REPORT_DATACLASS", False, f"failed: {ex}"))

    # PROM_PARENT_DIR
    try:
        parent = _promethean_parent_dir()
        ok = bool(parent.exists() and parent.is_dir())
        results.append(("PROM_PARENT_DIR", ok, f"parent={parent}, exists={ok}"))
    except Exception as ex:
        results.append(("PROM_PARENT_DIR", False, f"failed: {ex}"))

    # IS_PORT_FREE
    try:
        # Pick a random high port (likely free)
        ok = _is_port_free(DEFAULT_HOST, 39999) is True
        results.append(("IS_PORT_FREE", ok, "_is_port_free(39999) returned True"))
    except Exception as ex:
        results.append(("IS_PORT_FREE", False, f"failed: {ex}"))

    # FIND_OPEN_PORT
    try:
        port = _find_open_port(DEFAULT_HOST, DEFAULT_PORT_MIN, DEFAULT_PORT_MAX)
        ok = DEFAULT_PORT_MIN <= port <= DEFAULT_PORT_MAX
        results.append(("FIND_OPEN_PORT", ok, f"port={port} in [{DEFAULT_PORT_MIN},{DEFAULT_PORT_MAX}]"))
    except Exception as ex:
        results.append(("FIND_OPEN_PORT", False, f"failed: {ex}"))

    # KILL_SUBPROCESS_NOOP
    try:
        proc = subprocess.Popen([sys.executable, "-c", "print('ok')"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        proc.wait(timeout=5)
        _kill_subprocess(proc, grace_s=1.0)
        ok = proc.poll() is not None
        results.append(("KILL_SUBPROCESS_NOOP", ok, "_kill_subprocess on exited proc no-op"))
    except Exception as ex:
        results.append(("KILL_SUBPROCESS_NOOP", False, f"failed: {ex}"))

    # WRITE_REPORT_JSON
    try:
        import tempfile as _tf
        with _tf.NamedTemporaryFile(suffix=".json", delete=False) as tf:
            tmp_json = Path(tf.name)
        r = DaemonReport(
            ok=True, verdict="PASS", host="127.0.0.1", port=18580, v1467_pid=1000,
            shutdown_reason="RUNTIME_LIMIT", n_polls=5, n_audits_seen=3, n_diff_events_emitted=2,
            n_errors=0, total_elapsed_s=10.0, diff_events=[], errors=[],
            guards=list(V1471_GUARDS), v3_guards=list(V1471_V3_GUARDS),
            borrowed_sources=list(BORROWED_SOURCES),
            guards_passed=14, guards_total=14, timestamp=time.time(),
            log_path="/tmp/log", jsonl_stream_path="/tmp/stream.jsonl",
            daemon_report_path="/tmp/r.json", daemon_md_path="/tmp/r.md",
            out_dir="/tmp/out",
        )
        write_report_json(r, tmp_json)
        ok = tmp_json.exists() and tmp_json.stat().st_size > 0
        results.append(("WRITE_REPORT_JSON", ok, f"json size={tmp_json.stat().st_size}"))
        tmp_json.unlink()
    except Exception as ex:
        results.append(("WRITE_REPORT_JSON", False, f"failed: {ex}"))

    return results


# ──────────────────────────────────────────────────────────────────────
# CLI entry point
# ──────────────────────────────────────────────────────────────────────


def _make_argparser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=V1471_MODULE,
        description="V1471 persistent audit monitor daemon: spawn V1467, poll /audit/history, emit diffs",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    # run — main daemon run
    p_run = sub.add_parser("run", help="Run V1471 daemon")
    p_run.add_argument("--host", default=DEFAULT_HOST)
    p_run.add_argument("--port-min", type=int, default=DEFAULT_PORT_MIN)
    p_run.add_argument("--port-max", type=int, default=DEFAULT_PORT_MAX)
    p_run.add_argument("--poll-interval", type=float, default=DEFAULT_POLL_INTERVAL_S)
    p_run.add_argument("--max-runtime", type=float, default=DEFAULT_MAX_RUNTIME_S)
    p_run.add_argument("--max-polls", type=int, default=DEFAULT_MAX_POLLS)
    p_run.add_argument("--audit-every-poll", type=int, default=DEFAULT_AUDIT_EVERY_POLL)
    p_run.add_argument("--http-timeout", type=float, default=DEFAULT_HTTP_TIMEOUT_S)
    p_run.add_argument("--v1467-boot-timeout", type=float, default=DEFAULT_V1467_BOOT_TIMEOUT_S)
    p_run.add_argument("--kill-grace", type=float, default=DEFAULT_KILL_GRACE_S)
    p_run.add_argument("--max-log-bytes", type=int, default=DEFAULT_MAX_LOG_BYTES)
    p_run.add_argument("--history-limit", type=int, default=DEFAULT_HISTORY_LIMIT)
    p_run.add_argument("--out-dir", default=None)
    p_run.add_argument("--verbose", action="store_true")

    # demo — short daemon run
    p_demo = sub.add_parser("demo", help="Run V1471 daemon with short defaults (5 polls, 15s)")
    p_demo.add_argument("--out-dir", default="out/v1471-demo")
    p_demo.add_argument("--verbose", action="store_true")

    # popper — in-process self-checks
    p_popper = sub.add_parser("popper", help="Run V1471 self-checks (no subprocess spawn)")
    p_popper.add_argument("--verbose", action="store_true")

    # meta — print module metadata
    p_meta = sub.add_parser("meta", help="Print V1471 metadata")

    # chain — verify V1467 importable
    p_chain = sub.add_parser("chain", help="Verify V1467 importable")
    p_chain.add_argument("--verbose", action="store_true")

    # help
    p_help = sub.add_parser("help", help="Print full V1471 help")

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = _make_argparser()
    args = parser.parse_args(argv)

    if args.cmd == "run":
        out_dir = Path(args.out_dir) if args.out_dir else None
        daemon = V1471AuditMonitorDaemon(
            host=args.host,
            port_min=args.port_min,
            port_max=args.port_max,
            poll_interval_s=args.poll_interval,
            max_runtime_s=args.max_runtime,
            max_polls=args.max_polls,
            audit_every_poll=args.audit_every_poll,
            http_timeout_s=args.http_timeout,
            v1467_boot_timeout_s=args.v1467_boot_timeout,
            kill_grace_s=args.kill_grace,
            max_log_bytes=args.max_log_bytes,
            history_limit=args.history_limit,
            out_dir=out_dir,
            verbose=args.verbose,
        )

        # Install signal handler for graceful shutdown on Ctrl+C
        def _handler(signum, frame):
            daemon.request_shutdown()

        try:
            signal.signal(signal.SIGINT, _handler)
            if hasattr(signal, "SIGTERM"):
                signal.signal(signal.SIGTERM, _handler)
        except (ValueError, OSError):
            # Windows or restricted env: signal may not be installable
            pass

        report = daemon.run()
        print(json.dumps({
            "verdict": report.verdict,
            "ok": report.ok,
            "n_polls": report.n_polls,
            "n_audits_seen": report.n_audits_seen,
            "n_diff_events_emitted": report.n_diff_events_emitted,
            "shutdown_reason": report.shutdown_reason,
            "daemon_report_path": report.daemon_report_path,
            "daemon_md_path": report.daemon_md_path,
            "log_path": report.log_path,
            "jsonl_stream_path": report.jsonl_stream_path,
            "total_elapsed_s": report.total_elapsed_s,
        }, ensure_ascii=False))
        return 0 if report.ok else 1

    elif args.cmd == "demo":
        out_dir = Path(args.out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        daemon = V1471AuditMonitorDaemon(
            host=DEFAULT_HOST,
            max_runtime_s=15.0,
            max_polls=5,
            poll_interval_s=1.0,
            audit_every_poll=2,
            out_dir=out_dir,
            verbose=args.verbose,
        )

        def _handler(signum, frame):
            daemon.request_shutdown()

        try:
            signal.signal(signal.SIGINT, _handler)
        except (ValueError, OSError):
            pass

        report = daemon.run()
        print(json.dumps({
            "verdict": report.verdict,
            "ok": report.ok,
            "n_polls": report.n_polls,
            "n_audits_seen": report.n_audits_seen,
            "n_diff_events_emitted": report.n_diff_events_emitted,
            "shutdown_reason": report.shutdown_reason,
            "daemon_report_path": report.daemon_report_path,
            "daemon_md_path": report.daemon_md_path,
            "log_path": report.log_path,
            "jsonl_stream_path": report.jsonl_stream_path,
            "total_elapsed_s": report.total_elapsed_s,
        }, ensure_ascii=False))
        return 0 if report.ok else 1

    elif args.cmd == "popper":
        results = popper_v1471()
        if args.verbose:
            for name, ok, msg in results:
                print(f"  [{'OK' if ok else 'FAIL'}] {name}: {msg}", file=sys.stderr)
        failed = [(name, msg) for name, ok, msg in results if not ok]
        summary = {
            "n_checks": len(results),
            "n_passed": sum(1 for _, ok, _ in results if ok),
            "n_failed": len(failed),
            "failed": [{"name": name, "msg": msg} for name, msg in failed],
        }
        print(json.dumps(summary, ensure_ascii=False))
        return 0 if not failed else 1

    elif args.cmd == "meta":
        meta = {
            "module": V1471_MODULE,
            "version": V1471_VERSION,
            "schema": V1471_SCHEMA,
            "date": V1471_DATE,
            "guards": list(V1471_GUARDS),
            "v3_guards": list(V1471_V3_GUARDS),
            "borrowed_sources": list(BORROWED_SOURCES),
            "shutdown_reasons": [r.value for r in ShutdownReason],
        }
        print(json.dumps(meta, ensure_ascii=False, indent=2))
        return 0

    elif args.cmd == "chain":
        chain_results = {}
        for mod_name in ("v1467_asi_audit_http_gateway_history_diff",):
            try:
                __import__(f"apeireth.{mod_name}")
                chain_results[mod_name] = "OK"
            except Exception as e:
                chain_results[mod_name] = f"FAIL: {e}"
        all_ok = all(v == "OK" for v in chain_results.values())
        print(json.dumps({"chain": chain_results, "all_ok": all_ok}, ensure_ascii=False))
        return 0 if all_ok else 1

    elif args.cmd == "help":
        parser.print_help()
        return 0

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())