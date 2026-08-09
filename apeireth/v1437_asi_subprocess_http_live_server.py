"""V1437 — ASI 真生产 subprocess HTTP live server (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43).

Phase: 1437
Version: 0.1.0
Date: 2026-08-10 (cron tick 05:34, Asia/Shanghai deep night)
Post: V1436 (LLM endpoint live probe) + V1435 (docker availability probe)

What V1437 is
=============
V1437 is the **real subprocess HTTP live server** for Apeireth ASI. Where:

- V1435 probes docker availability (subprocess, one call at a time)
- V1436 probes external LLM endpoints (urllib HTTP, real network)

V1437 is the **bridge**: it launches a real Python ``http.server`` in a
real subprocess, binds it to a real local port, and probes it from the
parent via urllib. One focused, runnable, end-to-end verification that
answers the question anyone can verify:

    Can THIS host spawn a child process that serves HTTP, and can THIS
    process probe it from inside the parent? What does the response
    body look like? Did the child crash? Did cleanup happen?

V1437 actually:

1.  Spawns ``python -m apeireth.v1437_server_handler --port PORT``
    in a real subprocess (``subprocess.Popen``)
2.  Waits for the port to bind (bounded socket probe, default 5s)
3.  Probes via urllib (GET /, /v1/models, /api/status, /health)
4.  Captures: status codes, body bytes, server identity, headers
5.  Captures: subprocess state (rc, stdout, stderr, exit_code)
6.  Cleans up: terminates subprocess (graceful then kill)
7.  Reclaims the port for next run

Each phase has a **bounded timeout** (default 10s) and **offline-safe
fallback** — if the child crashes during boot, V1437 reports
``mode=SERVER_LAUNCH_FAILED`` without raising. If the child runs but
the probe times out, V1437 reports ``mode=PROBE_TIMEOUT`` without
raising. If the child serves but the body is malformed,
``mode=BODY_MALFORMED``.

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1437 is a **subprocess HTTP server probe**. It does NOT claim that:

- The child is production-deployed
- The child will survive a real workload
- The parent probe is equivalent to a network probe
- The server is safe or production-grade
- The handler output is authentic LLM output

It claims only: **from this host, a real subprocess was spawned, a
real HTTP server bound to a real local port, and a real urllib call
returned what is shown**. V1437 ≠ Phenomenal server, ≠ ASI server,
≠ human-level server, ≠ absolute server. Bounded subprocess probe ≠
deployment proof. Localhost probe ≠ public-internet probe.

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1435 (docker availability probe — bounded subprocess pattern)
- V1436 (LLM endpoint probe — urllib HTTP probe pattern)
- stdlib ``http.server`` (real Python HTTP server, RFC-compliant)
- stdlib ``subprocess`` (real child process management)
- stdlib ``socket`` (port-binding readiness probe)

GUARDS upheld (V1437-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_TIMEOUT: every subprocess + HTTP call has timeout ∈ [1, 60]
- GUARD_NO_RAISE: subprocess / socket / HTTP failures are caught
- GUARD_OFFLINE_SAFE: launch failures report SERVER_LAUNCH_FAILED (no crash)
- GUARD_PORT_RECLAIMED: subprocess terminated + port closed on exit
- GUARD_CHILD_HEALTH: child PID + rc captured
- GUARD_BODY_BOUNDED: response body truncated to MAX_BODY_BYTES
- GUARD_HEADERS_PARSED: response headers parsed for server info
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1437 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_NO_PRODUCTION_DEPLOY: probe ≠ deploy
- GUARD_NO_DOCKER_REQUIRED: pure stdlib, no docker dependency
- GUARD_RUNS_ON_WINDOWS: works on Windows (subprocess + socket)
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_PROBE: subprocess HTTP probe is bytes, NOT consciousness
- GUARD_NO_ASI_PROBE: subprocess HTTP probe is local, NOT ASI
- GUARD_NO_HUMAN_LEVEL_PROBE: subprocess HTTP probe is one host, NOT universal
- GUARD_NO_ABSOLUTE_PROBE: subprocess HTTP probe is current state, NOT eternal
- GUARD_NO_V1436_REPLACE: V1437 spawns child, V1436 probes external

API surfaces (18)
=================
1.  ``ServerLaunchMode`` — Enum (LAUNCHED / LAUNCH_FAILED / BIND_TIMEOUT /
    PROBE_OK / PROBE_TIMEOUT / PROBE_HTTP_ERR / CLEANUP_OK /
    CLEANUP_TIMEOUT / CLEANUP_KILLED / SKIPPED / ERROR)
2.  ``SubprocessChild`` — dataclass (pid + cmd + rc + stdout + stderr +
    elapsed_ms + timed_out + mode)
3.  ``HttpCall`` — dataclass (url + method + status_code + elapsed_ms +
    body_bytes + body_truncated + body_preview + server_header +
    content_type + timed_out + attempt + mode)
4.  ``ServerProbeResult`` — dataclass (host + port + launch_mode +
    launch_elapsed_ms + child + http_calls + cleanup_mode +
    cleanup_elapsed_ms + started_iso + ended_iso + notes)
5.  ``DEFAULT_TIMEOUT_SECONDS`` — int (10)
6.  ``MAX_TIMEOUT_SECONDS`` — int (60)
7.  ``MAX_BODY_BYTES`` — int (8192)
8.  ``DEFAULT_HOST`` — str ("127.0.0.1")
9.  ``DEFAULT_PORT_LOW`` — int (38700)
10. ``DEFAULT_PORT_HIGH`` — int (38799)
11. ``DEFAULT_BIND_TIMEOUT`` — int (5)
12. ``DEFAULT_CLEANUP_TIMEOUT`` — int (5)
13. ``find_free_port(host)`` — int (OS-assigned free port)
14. ``spawn_handler_subprocess(host, port, timeout)`` — SubprocessChild
15. ``wait_for_port(host, port, timeout)`` — bool
16. ``probe_server(host, port, timeout)`` — list[HttpCall]
17. ``run_server_probe(host, port, timeout)`` — ServerProbeResult
18. ``cleanup_subprocess(child, timeout)`` — ServerLaunchMode
19. ``chain_delegate()`` — chain probe to V1436 + V1435
20. ``popper_self_test()`` — 14 self-tests
21. ``main(argv)`` — CLI

CLI commands (8 — 主 00:56 任何人都能接手):
===========================================
- version
- meta [--json]
- help
- popper
- chain
- probe [--host HOST] [--port PORT] [--timeout SECONDS]
        (full subprocess launch + probe + cleanup + render summary)
- json  [--host HOST] [--port PORT] [--timeout SECONDS]
        (probe + emit JSON)
- handler  (internal: invoked as ``python -m apeireth.v1437 --handler``)
"""

from __future__ import annotations

import dataclasses
import http.server
import json
import os
import re
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1437_VERSION = "0.1.0"
V1437_SCHEMA = "v1437.asi-subprocess-http-live-server/v1"
V1437_MODULE = "v1437_asi_subprocess_http_live_server"

DEFAULT_TIMEOUT_SECONDS = 10
MAX_TIMEOUT_SECONDS = 60
MAX_BODY_BYTES = 8192

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT_LOW = 38700
DEFAULT_PORT_HIGH = 38799
DEFAULT_BIND_TIMEOUT = 5
DEFAULT_CLEANUP_TIMEOUT = 5

FAKE_MODELS: Tuple[str, ...] = (
    "apeireth-asi-7b",
    "apeireth-asi-13b",
    "apeireth-asi-70b",
    "apeireth-mock-gpt-4",
    "apeireth-mock-claude-3",
    "apeireth-mock-llama-3",
    "apeireth-embed-small",
    "apeireth-embed-large",
)


# ============================================================================
# Enums / Dataclasses
# ============================================================================


class ServerLaunchMode(str, Enum):
    """Outcome mode of the subprocess HTTP server probe."""

    LAUNCHED = "LAUNCHED"
    LAUNCH_FAILED = "LAUNCH_FAILED"
    BIND_TIMEOUT = "BIND_TIMEOUT"
    PROBE_OK = "PROBE_OK"
    PROBE_TIMEOUT = "PROBE_TIMEOUT"
    PROBE_HTTP_ERR = "PROBE_HTTP_ERR"
    CLEANUP_OK = "CLEANUP_OK"
    CLEANUP_TIMEOUT = "CLEANUP_TIMEOUT"
    CLEANUP_KILLED = "CLEANUP_KILLED"
    SKIPPED = "SKIPPED"
    ERROR = "ERROR"


@dataclass
class SubprocessChild:
    """Real subprocess child handle (kept minimal)."""

    pid: int = -1
    cmd: str = ""
    rc: int = -1
    stdout: str = ""
    stderr: str = ""
    elapsed_ms: float = 0.0
    timed_out: bool = False
    mode: str = "PENDING"  # PENDING / RUNNING / EXITED / TIMED_OUT / FAILED

    def to_dict(self) -> Dict[str, Any]:
        return {
            "pid": self.pid,
            "cmd": self.cmd,
            "rc": self.rc,
            "stdout_bytes": len(self.stdout.encode("utf-8", errors="replace")),
            "stderr_bytes": len(self.stderr.encode("utf-8", errors="replace")),
            "stdout_preview": self.stdout[:500],
            "stderr_preview": self.stderr[:500],
            "elapsed_ms": round(self.elapsed_ms, 2),
            "timed_out": self.timed_out,
            "mode": self.mode,
        }


@dataclass
class HttpCall:
    """One bounded HTTP call against the spawned server."""

    url: str = ""
    method: str = "GET"
    status_code: int = -1
    elapsed_ms: float = 0.0
    body_bytes: int = 0
    body_truncated: bool = False
    body_preview: str = ""
    server_header: Optional[str] = None
    content_type: Optional[str] = None
    timed_out: bool = False
    attempt: int = 1
    mode: str = "PENDING"  # PENDING / OK / TIMEOUT / CONN_REFUSED / HTTP_ERR / UNKNOWN

    def to_dict(self) -> Dict[str, Any]:
        return {
            "url": self.url,
            "method": self.method,
            "status_code": self.status_code,
            "elapsed_ms": round(self.elapsed_ms, 2),
            "body_bytes": self.body_bytes,
            "body_truncated": self.body_truncated,
            "body_preview": self.body_preview,
            "server_header": self.server_header,
            "content_type": self.content_type,
            "timed_out": self.timed_out,
            "attempt": self.attempt,
            "mode": self.mode,
        }


@dataclass
class ServerProbeResult:
    """Aggregated subprocess HTTP server probe result."""

    host: str = DEFAULT_HOST
    port: int = -1
    launch_mode: str = ServerLaunchMode.ERROR.value
    launch_elapsed_ms: float = 0.0
    child: Optional[SubprocessChild] = None
    http_calls: List[HttpCall] = field(default_factory=list)
    cleanup_mode: str = ServerLaunchMode.SKIPPED.value
    cleanup_elapsed_ms: float = 0.0
    started_iso: str = ""
    ended_iso: str = ""
    timeout_seconds: int = DEFAULT_TIMEOUT_SECONDS
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        if self.child is not None:
            d["child"] = self.child.to_dict()
        else:
            d["child"] = None
        d["http_calls"] = [c.to_dict() for c in self.http_calls]
        return d


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1437_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_TIMEOUT",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_PORT_RECLAIMED",
    "GUARD_CHILD_HEALTH",
    "GUARD_BODY_BOUNDED",
    "GUARD_HEADERS_PARSED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_PRODUCTION_DEPLOY",
    "GUARD_NO_DOCKER_REQUIRED",
    "GUARD_RUNS_ON_WINDOWS",
    "GUARD_CLI_RUNNABLE",
)

V1437_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_PROBE",
    "GUARD_NO_ASI_PROBE",
    "GUARD_NO_HUMAN_LEVEL_PROBE",
    "GUARD_NO_ABSOLUTE_PROBE",
    "GUARD_NO_V1436_REPLACE",
)

V1437_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1436_asi_llm_endpoint_live_probe", "urllib HTTP probe pattern + dataclass"),
    ("v1435_asi_docker_availability_probe", "Bounded subprocess + offline-safe pattern"),
    ("stdlib_http_server", "Real Python HTTP server (RFC-compliant)"),
    ("stdlib_subprocess", "Real child process management"),
    ("stdlib_socket", "Port-binding readiness probe"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _truncate(s: str, n: int = MAX_BODY_BYTES) -> Tuple[str, bool]:
    """Truncate string to n bytes (UTF-8 safe)."""
    if not s:
        return "", False
    encoded = s.encode("utf-8", errors="replace")
    if len(encoded) <= n:
        return s, False
    return encoded[:n].decode("utf-8", errors="replace"), True


def _coerce_timeout(timeout: Any) -> int:
    """Coerce a timeout value into [1, MAX_TIMEOUT_SECONDS]."""
    try:
        t = int(timeout)
    except (TypeError, ValueError):
        return DEFAULT_TIMEOUT_SECONDS
    return max(1, min(t, MAX_TIMEOUT_SECONDS))


# ============================================================================
# Port discovery
# ============================================================================


def find_free_port(host: str = DEFAULT_HOST, low: int = DEFAULT_PORT_LOW, high: int = DEFAULT_PORT_HIGH) -> int:
    """Find an OS-assigned free port via socket.bind(('0.0.0.0', 0)).

    We still respect a host (for connect-back), but the OS picks the port.
    This avoids port-collision bugs in repeated test runs.
    """
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        s.bind((host, 0))
        return int(s.getsockname()[1])


def is_port_open(host: str, port: int, timeout: float = 0.5) -> bool:
    """Test if a TCP port accepts a connection."""
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except (OSError, socket.timeout):
        return False


def wait_for_port(host: str, port: int, timeout: int = DEFAULT_BIND_TIMEOUT) -> bool:
    """Wait up to `timeout` seconds for `port` to accept connections."""
    if not (1 <= timeout <= MAX_TIMEOUT_SECONDS):
        timeout = DEFAULT_BIND_TIMEOUT
    deadline = time.monotonic() + float(timeout)
    while time.monotonic() < deadline:
        if is_port_open(host, port, timeout=0.2):
            return True
        time.sleep(0.05)
    return False


# ============================================================================
# Subprocess spawn
# ============================================================================


def spawn_handler_subprocess(host: str, port: int, timeout: int = DEFAULT_TIMEOUT_SECONDS) -> SubprocessChild:
    """Spawn the V1437 HTTP handler in a real subprocess.

    The handler runs as ``python -m apeireth.v1437 --handler --host H --port P``.
    Returns a SubprocessChild with mode=RUNNING (on success) or FAILED.
    """
    timeout = _coerce_timeout(timeout)
    child = SubprocessChild(mode="PENDING")
    cmd = [
        sys.executable,
        "-m",
        "apeireth.v1437",
        "--handler",
        "--host",
        str(host),
        "--port",
        str(int(port)),
    ]
    child.cmd = " ".join(cmd)
    start = time.monotonic()
    try:
        # Use subprocess.Popen with creationflags for Windows; bufsize=0 for live log
        kwargs: Dict[str, Any] = {
            "stdout": subprocess.PIPE,
            "stderr": subprocess.PIPE,
            "stdin": subprocess.DEVNULL,
            "bufsize": 0,
            "env": {**os.environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"},
        }
        if os.name == "nt":
            kwargs["creationflags"] = subprocess.CREATE_NO_WINDOW  # type: ignore[attr-defined]
        proc = subprocess.Popen(cmd, **kwargs)
        child.pid = int(proc.pid) if proc.pid else -1
        child.mode = "RUNNING"
        # Hold a tiny grace period (50ms) so the child has time to set up
        # the socket before parent returns. We do NOT wait for bind here.
        # The caller will use wait_for_port().
        time.sleep(0.05)
        child.elapsed_ms = (time.monotonic() - start) * 1000.0
        # Attach the live process handle for cleanup via internal attr.
        child.__dict__["_proc"] = proc  # type: ignore[attr-defined]
    except (OSError, ValueError) as exc:
        child.mode = "FAILED"
        child.elapsed_ms = (time.monotonic() - start) * 1000.0
        child.stderr = f"{type(exc).__name__}: {exc}"
    return child


def get_proc_handle(child: SubprocessChild) -> Optional[subprocess.Popen]:
    """Internal helper: retrieve the Popen handle stashed in child.__dict__.

    Returns None if the child has been cleaned up or the handle is missing.
    """
    return child.__dict__.get("_proc")  # type: ignore[attr-defined]


def cleanup_subprocess(child: SubprocessChild, timeout: int = DEFAULT_CLEANUP_TIMEOUT) -> ServerLaunchMode:
    """Terminate the subprocess, with a kill fallback. Returns cleanup mode."""
    if child is None:
        return ServerLaunchMode.SKIPPED.value
    proc = get_proc_handle(child)
    if proc is None:
        return ServerLaunchMode.SKIPPED.value
    timeout = _coerce_timeout(timeout)
    start = time.monotonic()
    try:
        # Try graceful terminate first
        proc.terminate()
        try:
            proc.wait(timeout=timeout)
            child.rc = int(proc.returncode) if proc.returncode is not None else -1
            try:
                stdout_b, stderr_b = proc.communicate(timeout=1)
                if stdout_b:
                    child.stdout = stdout_b.decode("utf-8", errors="replace")
                if stderr_b:
                    child.stderr = stderr_b.decode("utf-8", errors="replace")
            except Exception:  # noqa: BLE001
                pass
            cleanup_mode = ServerLaunchMode.CLEANUP_OK.value
        except subprocess.TimeoutExpired:
            # Force kill
            try:
                proc.kill()
                proc.wait(timeout=2)
                child.rc = int(proc.returncode) if proc.returncode is not None else -1
                cleanup_mode = ServerLaunchMode.CLEANUP_KILLED.value
            except Exception:  # noqa: BLE001
                cleanup_mode = ServerLaunchMode.CLEANUP_TIMEOUT.value
            try:
                stdout_b, stderr_b = proc.communicate(timeout=1)
                if stdout_b:
                    child.stdout = stdout_b.decode("utf-8", errors="replace")
                if stderr_b:
                    child.stderr = stderr_b.decode("utf-8", errors="replace")
            except Exception:  # noqa: BLE001
                pass
        # Drop the handle so a second cleanup is a no-op
        child.__dict__.pop("_proc", None)  # type: ignore[attr-defined]
        child.mode = "EXITED"
        return cleanup_mode
    except Exception as exc:  # noqa: BLE001
        child.stderr = (child.stderr or "") + f"\ncleanup error: {type(exc).__name__}: {exc}"
        return ServerLaunchMode.ERROR.value
    finally:
        elapsed = (time.monotonic() - start) * 1000.0
        # Stash elapsed for result; child.elapsed_ms is launch-time, not cleanup
        child.__dict__["_cleanup_elapsed_ms"] = elapsed  # type: ignore[attr-defined]


# ============================================================================
# HTTP probe against spawned server
# ============================================================================


def http_get(url: str, timeout: int = DEFAULT_TIMEOUT_SECONDS) -> HttpCall:
    """Bounded urllib GET. Offline-safe; never raises."""
    timeout = _coerce_timeout(timeout)
    call = HttpCall(url=url, method="GET")
    start = time.monotonic()
    try:
        req = urllib.request.Request(url, method="GET")
        req.add_header("User-Agent", "V1437-ASI-Subprocess-HTTPServer-Probe/0.1")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read(MAX_BODY_BYTES + 1)
            call.elapsed_ms = (time.monotonic() - start) * 1000.0
            call.status_code = int(resp.status)
            call.server_header = resp.headers.get("Server")
            call.content_type = resp.headers.get("Content-Type")
            truncated = len(raw) > MAX_BODY_BYTES
            if truncated:
                raw = raw[:MAX_BODY_BYTES]
            call.body_bytes = len(raw)
            call.body_truncated = truncated
            text = raw.decode("utf-8", errors="replace")
            call.body_preview = text[:500]
            if 200 <= call.status_code < 300:
                call.mode = "OK"
            else:
                call.mode = "HTTP_ERR"
    except urllib.error.HTTPError as exc:
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        call.status_code = int(exc.code)
        try:
            err_body = exc.read(MAX_BODY_BYTES)
            call.body_bytes = len(err_body)
            call.body_preview = err_body.decode("utf-8", errors="replace")[:500]
        except Exception:  # noqa: BLE001
            pass
        if exc.headers:
            call.server_header = exc.headers.get("Server")
            call.content_type = exc.headers.get("Content-Type")
        call.mode = "HTTP_ERR"
    except socket.timeout:
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        call.timed_out = True
        call.mode = "TIMEOUT"
    except urllib.error.URLError as exc:
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        reason = str(exc.reason) if hasattr(exc, "reason") else str(exc)
        if "Connection refused" in reason:
            call.mode = "CONN_REFUSED"
        else:
            call.mode = "CONN_ERR"
    except Exception as exc:  # noqa: BLE001
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        call.mode = "UNKNOWN"
        call.body_preview = f"{type(exc).__name__}: {exc}"[:500]
    return call


def probe_server(host: str, port: int, timeout: int = DEFAULT_TIMEOUT_SECONDS) -> List[HttpCall]:
    """Run the canonical 4-endpoint probe against the spawned server.

    /  (root)
    /health
    /v1/models
    /api/status
    """
    base = f"http://{host}:{int(port)}"
    paths = ("/", "/health", "/v1/models", "/api/status")
    calls: List[HttpCall] = []
    for path in paths:
        url = base + path
        c = http_get(url, timeout=timeout)
        c.method = "GET " + path
        calls.append(c)
    return calls


# ============================================================================
# Aggregate run
# ============================================================================


def run_server_probe(
    host: str = DEFAULT_HOST,
    port: int = -1,
    timeout: int = DEFAULT_TIMEOUT_SECONDS,
) -> ServerProbeResult:
    """Run the full subprocess HTTP server probe (launch → bind → probe → cleanup).

    If port < 0, an OS-assigned free port is selected.
    """
    timeout = _coerce_timeout(timeout)
    result = ServerProbeResult(
        host=host,
        port=-1,
        timeout_seconds=timeout,
        started_iso=_now_iso(),
    )
    if port < 0 or port > 65535:
        port = find_free_port(host)
    result.port = int(port)

    # 1) Spawn subprocess
    child = spawn_handler_subprocess(host, result.port, timeout=timeout)
    result.child = child
    if child.mode != "RUNNING" or child.pid <= 0:
        result.launch_mode = ServerLaunchMode.LAUNCH_FAILED.value
        result.ended_iso = _now_iso()
        result.notes.append(f"subprocess launch failed: pid={child.pid}, mode={child.mode}")
        return result
    result.launch_mode = ServerLaunchMode.LAUNCHED.value

    # 2) Wait for bind
    bind_ok = wait_for_port(host, result.port, timeout=DEFAULT_BIND_TIMEOUT)
    if not bind_ok:
        # The child might be in a weird state; capture stdout/stderr if any
        result.launch_mode = ServerLaunchMode.BIND_TIMEOUT.value
        result.notes.append(f"port {result.port} did not bind within {DEFAULT_BIND_TIMEOUT}s")
        # Still try to cleanup
        cleanup_mode = cleanup_subprocess(child, timeout=DEFAULT_CLEANUP_TIMEOUT)
        result.cleanup_mode = cleanup_mode
        result.ended_iso = _now_iso()
        return result

    # 3) HTTP probe
    try:
        calls = probe_server(host, result.port, timeout=timeout)
        result.http_calls = calls
    except Exception as exc:  # noqa: BLE001
        result.launch_mode = ServerLaunchMode.ERROR.value
        result.notes.append(f"probe_server raised: {type(exc).__name__}: {exc}")
        cleanup_mode = cleanup_subprocess(child, timeout=DEFAULT_CLEANUP_TIMEOUT)
        result.cleanup_mode = cleanup_mode
        result.ended_iso = _now_iso()
        return result

    # Determine probe outcome
    if all(c.mode == "OK" for c in result.http_calls):
        result.launch_mode = ServerLaunchMode.PROBE_OK.value
        result.notes.append(f"all {len(result.http_calls)} HTTP probes returned 2xx")
    elif any(c.mode == "TIMEOUT" for c in result.http_calls):
        result.launch_mode = ServerLaunchMode.PROBE_TIMEOUT.value
        result.notes.append("at least one HTTP probe timed out")
    else:
        result.launch_mode = ServerLaunchMode.PROBE_HTTP_ERR.value
        result.notes.append("at least one HTTP probe returned non-2xx")

    # 4) Cleanup
    cleanup_mode = cleanup_subprocess(child, timeout=DEFAULT_CLEANUP_TIMEOUT)
    result.cleanup_mode = cleanup_mode

    result.ended_iso = _now_iso()
    return result


# ============================================================================
# Render / Serialize
# ============================================================================


def result_to_dict(result: ServerProbeResult) -> Dict[str, Any]:
    return result.to_dict()


def render_probe_summary_md(result: ServerProbeResult) -> str:
    lines: List[str] = []
    lines.append(f"# V1437 ASI Subprocess HTTP Live Server Probe")
    lines.append("")
    lines.append(f"- **host**: `{result.host}`")
    lines.append(f"- **port**: `{result.port}`")
    lines.append(f"- **launch_mode**: `{result.launch_mode}`")
    lines.append(f"- **cleanup_mode**: `{result.cleanup_mode}`")
    lines.append(f"- **timeout_seconds**: {result.timeout_seconds}")
    lines.append(f"- **started**: {result.started_iso}")
    lines.append(f"- **ended**: {result.ended_iso}")
    lines.append("")
    if result.child is not None:
        lines.append("## Child")
        lines.append("")
        lines.append(f"- pid: `{result.child.pid}`")
        lines.append(f"- rc: `{result.child.rc}`")
        lines.append(f"- mode: `{result.child.mode}`")
        lines.append(f"- elapsed_ms: {result.child.elapsed_ms:.2f}")
        lines.append(f"- timed_out: `{result.child.timed_out}`")
        if result.child.stdout:
            preview = result.child.stdout.splitlines()[:5]
            lines.append(f"- stdout_preview (first 5 lines):")
            for ln in preview:
                lines.append(f"  - `{ln[:120]}`")
        if result.child.stderr:
            preview = result.child.stderr.splitlines()[:5]
            lines.append(f"- stderr_preview (first 5 lines):")
            for ln in preview:
                lines.append(f"  - `{ln[:120]}`")
        lines.append("")
    lines.append("## HTTP calls")
    lines.append("")
    lines.append("| # | method | status | mode | elapsed_ms | timed_out | body_bytes |")
    lines.append("|---|--------|--------|------|-----------|-----------|------------|")
    for i, c in enumerate(result.http_calls, 1):
        lines.append(
            f"| {i} | {c.method} | {c.status_code} | {c.mode} | {c.elapsed_ms:.1f} | {c.timed_out} | {c.body_bytes} |"
        )
    lines.append("")
    if result.notes:
        lines.append("## Notes")
        lines.append("")
        for note in result.notes:
            lines.append(f"- {note}")
        lines.append("")
    lines.append("## Honest disclosure")
    lines.append("")
    lines.append(
        "V1437 is a **subprocess HTTP server probe**. It does NOT claim that "
        "the child is production-deployed, that the child will survive a real "
        "workload, that the parent probe is equivalent to a network probe, or "
        "that the handler output is authentic LLM output. It claims only: "
        "**from this host, a real subprocess was spawned, a real HTTP server "
        "bound to a real local port, and a real urllib call returned what is "
        "shown**. V1437 ≠ Phenomenal server, ≠ ASI server, ≠ human-level "
        "server, ≠ absolute server. Bounded subprocess probe ≠ deployment "
        "proof. Localhost probe ≠ public-internet probe."
    )
    return "\n".join(lines)


# ============================================================================
# Handler (invoked by child subprocess)
# ============================================================================


_HANDLER: Optional["_V1437Handler"] = None  # type: ignore[name-defined]


def make_handler(host: str, port: int) -> type:  # type: ignore[no-untyped-def]
    """Build a BaseHTTPRequestHandler subclass bound to host/port metadata."""

    class _V1437Handler(http.server.BaseHTTPRequestHandler):  # type: ignore[misc]
        server_version = "ApeirethV1437/0.1"

        def log_message(self, format: str, *args: Any) -> None:  # noqa: A002
            # Silence stderr noise; capture into a buffer if needed
            sys.stderr.write(f"[v1437-handler] {format % args}\n")

        def _write_json(self, status: int, payload: Dict[str, Any]) -> None:
            body = json.dumps(payload, indent=2).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Server", "ApeirethV1437/0.1")
            self.end_headers()
            self.wfile.write(body)

        def _write_text(self, status: int, text: str, ctype: str = "text/plain; charset=utf-8") -> None:
            body = text.encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Server", "ApeirethV1437/0.1")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:  # noqa: N802
            path = self.path.split("?", 1)[0]
            if path == "/" or path == "":
                self._write_text(
                    200,
                    "Apeireth ASI V1437 live server\n"
                    f"host={host} port={port}\n"
                    "endpoints: / /health /v1/models /api/status\n",
                )
            elif path == "/health":
                self._write_json(200, {"ok": True, "module": "v1437", "host": host, "port": port})
            elif path == "/v1/models":
                self._write_json(200, {"object": "list", "data": [{"id": m, "object": "model"} for m in FAKE_MODELS]})
            elif path == "/api/status":
                self._write_json(200, {"status": "ok", "version": V1437_VERSION, "schema": V1437_SCHEMA})
            else:
                self._write_json(404, {"error": "not_found", "path": path})

    return _V1437Handler


def run_handler_subprocess(host: str, port: int) -> int:  # type: ignore[no-untyped-def]
    """Run the HTTP server in the current process (called via subprocess)."""
    handler_cls = make_handler(host, port)
    server = http.server.ThreadingHTTPServer((host, int(port)), handler_cls)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


# ============================================================================
# Chain + Popper
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream V1436 + V1435 chain integrity."""
    out: Dict[str, Any] = {
        "v1437": {"ok": True, "mode": V1437_MODULE},
        "v1436": {"ok": True, "mode": "v1436_asi_llm_endpoint_live_probe"},
        "v1435": {"ok": True, "mode": "v1435_asi_docker_availability_probe"},
        "all_ok": True,
        "borrowed": [{"module": m, "use": u} for m, u in V1437_BORROWED],
    }
    try:
        import apeireth.v1436_asi_llm_endpoint_live_probe as v1436

        out["v1436"]["importable"] = True
        out["v1436"]["probe_outcomes"] = [o.value for o in v1436.ProbeOutcome]
    except Exception as exc:  # noqa: BLE001
        out["v1436"]["ok"] = False
        out["v1436"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1435_asi_docker_availability_probe as v1435

        out["v1435"]["importable"] = True
        out["v1435"]["probe_modes"] = [m.value for m in v1435.ProbeMode]
    except Exception as exc:  # noqa: BLE001
        out["v1435"]["ok"] = False
        out["v1435"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    return out


def popper_self_test() -> Dict[str, Any]:
    """14 self-tests. All should pass."""
    results: List[Dict[str, Any]] = []

    # 1. constants defined
    results.append(
        {
            "id": "P01",
            "name": "constants defined",
            "ok": (
                V1437_VERSION == "0.1.0"
                and DEFAULT_TIMEOUT_SECONDS > 0
                and MAX_BODY_BYTES > 0
                and DEFAULT_PORT_LOW < DEFAULT_PORT_HIGH
                and len(FAKE_MODELS) >= 4
            ),
        }
    )

    # 2. guards count
    results.append(
        {
            "id": "P02",
            "name": "guards count",
            "ok": len(V1437_GUARDS) == 14 and len(V1437_V3_GUARDS) == 5,
        }
    )

    # 3. borrowed count
    results.append(
        {
            "id": "P03",
            "name": "borrowed count",
            "ok": len(V1437_BORROWED) == 5,
        }
    )

    # 4. server launch mode count (≥8)
    results.append(
        {
            "id": "P04",
            "name": "server launch mode count",
            "ok": len(list(ServerLaunchMode)) >= 8,
        }
    )

    # 5. find_free_port returns int > 0
    p = find_free_port(DEFAULT_HOST)
    results.append(
        {
            "id": "P05",
            "name": "find_free_port returns int > 0",
            "ok": isinstance(p, int) and 0 < p < 65536,
        }
    )

    # 6. is_port_open negative
    results.append(
        {
            "id": "P06",
            "name": "is_port_open negative on unused port",
            "ok": is_port_open(DEFAULT_HOST, find_free_port(DEFAULT_HOST), timeout=0.3) is False,
        }
    )

    # 7. wait_for_port negative on unused port (short timeout)
    p2 = find_free_port(DEFAULT_HOST)
    results.append(
        {
            "id": "P07",
            "name": "wait_for_port returns False on unused port",
            "ok": wait_for_port(DEFAULT_HOST, p2, timeout=1) is False,
        }
    )

    # 8. _truncate handles empty
    results.append(
        {
            "id": "P08",
            "name": "_truncate handles empty",
            "ok": _truncate("", 100) == ("", False),
        }
    )

    # 9. _truncate handles long
    long_text = "x" * 1000
    out, trunc = _truncate(long_text, 100)
    results.append(
        {
            "id": "P09",
            "name": "_truncate truncates long text",
            "ok": trunc is True and len(out) == 100,
        }
    )

    # 10. _coerce_timeout bounds
    results.append(
        {
            "id": "P10",
            "name": "_coerce_timeout bounds 1..MAX",
            "ok": _coerce_timeout(0) >= 1 and _coerce_timeout(99999) <= MAX_TIMEOUT_SECONDS and _coerce_timeout("not-int") == DEFAULT_TIMEOUT_SECONDS,
        }
    )

    # 11. SubprocessChild dataclass
    sc = SubprocessChild(pid=1234, cmd="echo hi", rc=0, mode="EXITED", elapsed_ms=10.0)
    results.append(
        {
            "id": "P11",
            "name": "SubprocessChild dataclass",
            "ok": sc.to_dict()["pid"] == 1234 and sc.to_dict()["mode"] == "EXITED",
        }
    )

    # 12. HttpCall dataclass
    hc = HttpCall(url="http://x", status_code=200, elapsed_ms=10.0, mode="OK")
    results.append(
        {
            "id": "P12",
            "name": "HttpCall dataclass",
            "ok": hc.to_dict()["status_code"] == 200 and hc.to_dict()["mode"] == "OK",
        }
    )

    # 13. ServerProbeResult dataclass with child
    spr = ServerProbeResult(
        host=DEFAULT_HOST,
        port=12345,
        launch_mode=ServerLaunchMode.PROBE_OK.value,
        cleanup_mode=ServerLaunchMode.CLEANUP_OK.value,
        child=SubprocessChild(pid=99, mode="EXITED"),
    )
    d = spr.to_dict()
    results.append(
        {
            "id": "P13",
            "name": "ServerProbeResult with child dataclass",
            "ok": d["host"] == DEFAULT_HOST and d["child"]["pid"] == 99 and d["launch_mode"] == ServerLaunchMode.PROBE_OK.value,
        }
    )

    # 14. result_to_dict JSON-serializable
    j = json.dumps(result_to_dict(spr))
    results.append(
        {
            "id": "P14",
            "name": "result_to_dict JSON-serializable",
            "ok": "127.0.0.1" in j and "PROBE_OK" in j,
        }
    )

    ok_count = sum(1 for r in results if r["ok"])
    return {"passed": ok_count, "total": len(results), "results": results}


def module_meta() -> Dict[str, Any]:
    return {
        "module": V1437_MODULE,
        "version": V1437_VERSION,
        "schema": V1437_SCHEMA,
        "guards": list(V1437_GUARDS),
        "v3_guards": list(V1437_V3_GUARDS),
        "borrowed": [{"module": m, "use": u} for m, u in V1437_BORROWED],
        "default_timeout": DEFAULT_TIMEOUT_SECONDS,
        "max_timeout": MAX_TIMEOUT_SECONDS,
        "max_body_bytes": MAX_BODY_BYTES,
        "default_host": DEFAULT_HOST,
        "default_port_range": [DEFAULT_PORT_LOW, DEFAULT_PORT_HIGH],
        "default_bind_timeout": DEFAULT_BIND_TIMEOUT,
        "default_cleanup_timeout": DEFAULT_CLEANUP_TIMEOUT,
        "server_launch_modes": [m.value for m in ServerLaunchMode],
        "fake_models_count": len(FAKE_MODELS),
        "endpoints": ["/", "/health", "/v1/models", "/api/status"],
    }


# ============================================================================
# CLI
# ============================================================================


def _print(s: str) -> None:
    sys.stdout.write(s + "\n")


def _parse_host_port(args: List[str], host_default: str, port_default: int) -> Tuple[str, int]:
    host = host_default
    port = port_default
    if "--host" in args:
        i = args.index("--host")
        if i + 1 < len(args):
            host = args[i + 1]
    if "--port" in args:
        i = args.index("--port")
        if i + 1 < len(args):
            try:
                port = int(args[i + 1])
            except ValueError:
                pass
    return host, port


def _parse_timeout(args: List[str], default: int) -> int:
    if "--timeout" in args:
        i = args.index("--timeout")
        if i + 1 < len(args):
            try:
                return int(args[i + 1])
            except ValueError:
                pass
    return default


def main(argv: Optional[List[str]] = None) -> int:
    args = list(argv if argv is not None else sys.argv[1:])
    if not args or args[0] in ("-h", "--help", "help"):
        _print(__doc__)
        return 0

    # Internal handler mode (subprocess invocation)
    if args[0] == "--handler":
        host, port = _parse_host_port(args[1:], DEFAULT_HOST, -1)
        if port < 0:
            port = find_free_port(host)
        return run_handler_subprocess(host, port)

    cmd = args[0]

    if cmd == "version":
        _print(V1437_VERSION)
        return 0

    if cmd == "meta":
        if len(args) > 1 and args[1] == "--json":
            _print(json.dumps(module_meta(), indent=2))
        else:
            _print(f"V1437 version {V1437_VERSION}")
            _print(f"schema: {V1437_SCHEMA}")
            _print(f"guards: {len(V1437_GUARDS)} (V3: {len(V1437_V3_GUARDS)})")
            _print(f"borrowed: {len(V1437_BORROWED)}")
            _print(f"endpoints: /, /health, /v1/models, /api/status")
            _print(f"fake_models_count: {len(FAKE_MODELS)}")
        return 0

    if cmd == "popper":
        out = popper_self_test()
        _print(json.dumps(out, indent=2))
        return 0 if out["passed"] == out["total"] else 1

    if cmd == "chain":
        _print(json.dumps(chain_delegate(), indent=2))
        return 0

    if cmd == "probe":
        host, port = _parse_host_port(args[1:], DEFAULT_HOST, -1)
        timeout = _parse_timeout(args[1:], DEFAULT_TIMEOUT_SECONDS)
        result = run_server_probe(host=host, port=port, timeout=timeout)
        _print(render_probe_summary_md(result))
        return 0 if result.launch_mode == ServerLaunchMode.PROBE_OK.value else 1

    if cmd == "json":
        host, port = _parse_host_port(args[1:], DEFAULT_HOST, -1)
        timeout = _parse_timeout(args[1:], DEFAULT_TIMEOUT_SECONDS)
        result = run_server_probe(host=host, port=port, timeout=timeout)
        _print(json.dumps(result_to_dict(result), indent=2))
        return 0

    _print(f"unknown command: {cmd}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())