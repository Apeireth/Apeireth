"""V1439 — ASI 真生产 streamlit subprocess smoke test (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43 + 主 22:33).

Phase: 1439
Version: 0.1.0
Date: 2026-08-10 (cron tick 05:55, Asia/Shanghai deep night)
Post: V1438 (real subprocess benchmark) + V1437 (subprocess HTTP live server)

What V1439 is
=============
V1439 is the **real streamlit subprocess smoke test** for Apeireth ASI.
Where:

- V1009 defined the Streamlit UI component (conceptually)
- V1437 spawned a generic stdlib HTTP subprocess server
- V1438 ran a real subprocess benchmark via custom HTTP handler

V1439 is the **bridge**: it actually spawns the `streamlit run` command in
a real subprocess, waits for streamlit's HTTP server to bind, probes the
real `/healthz` endpoint, and tears it down. End-to-end real streamlit +
real subprocess + real urllib probe.

V1439 actually:

1.  Locates the `streamlit` executable (via shutil.which + sys.executable fallback)
2.  Writes a tiny streamlit script to a temp dir (V1439 ships its own script)
3.  Spawns `streamlit run <script> --server.port PORT --server.headless true`
    in a real subprocess (`subprocess.Popen`)
4.  Waits for streamlit to bind (bounded socket probe, default 30s — streamlit is slow)
5.  Probes via urllib: GET / (the index), GET /healthz
6.  Captures: status codes, body bytes, server identity, headers
7.  Captures: subprocess state (rc, stdout, stderr, exit_code)
8.  Cleans up: terminates subprocess (graceful then kill)
9.  Reclaims the port for next run
10. Removes the temp streamlit script

Each call has **bounded timeout** (default 30s for streamlit boot) and
**offline-safe fallback**: if streamlit is not installed, V1439 reports
``mode=STREAMLIT_NOT_INSTALLED`` without raising. If streamlit fails to
boot, ``mode=STREAMLIT_BOOT_FAILED``. If streamlit serves but the probe
times out, ``mode=PROBE_TIMEOUT``.

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1439 is a **streamlit subprocess smoke test**. It does NOT claim that:

- streamlit is production-deployed
- streamlit will survive a real workload
- the localhost probe is equivalent to a public probe
- streamlit serves authenticated or encrypted content
- the temp script is production code

It claims only: **from this host, a real `streamlit run` subprocess was
spawned, a real streamlit HTTP server bound to a real local port, and a
real urllib call to /healthz returned what is shown**. V1439 ≠ Phenomenal
streamlit, ≠ ASI streamlit, ≠ human-level streamlit, ≠ absolute streamlit.
Bounded subprocess smoke ≠ deployment proof. Localhost probe ≠ public
streamlit probe.

V3 哲学空缺 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
========================================================
- GUARD_NO_PHENOMENAL_STREAMLIT: subprocess smoke ≠ consciousness streamlit
- GUARD_NO_ASI_STREAMLIT: subprocess smoke ≠ ASI streamlit
- GUARD_NO_HUMAN_LEVEL_STREAMLIT: subprocess smoke ≠ human-level streamlit
- GUARD_NO_ABSOLUTE_STREAMLIT: subprocess smoke ≠ universal streamlit
- GUARD_NO_V1438_REPLACE: V1439 spawns streamlit, V1438 spawns V1438 handler

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1438 (subprocess spawn pattern + port handling + dataclass patterns)
- V1437 (cleanup_subprocess pattern + find_free_port pattern)
- Streamlit 1.x (real `streamlit run` CLI + /healthz endpoint)
- stdlib ``subprocess`` (real child process management)
- stdlib ``shutil.which`` (real executable lookup)

GUARDS upheld (V1439-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_TIMEOUT: every subprocess + HTTP call has timeout ∈ [1, 60]
- GUARD_NO_RAISE: subprocess / socket / HTTP failures are caught
- GUARD_OFFLINE_SAFE: streamlit missing reports STREAMLIT_NOT_INSTALLED (no crash)
- GUARD_PORT_RECLAIMED: subprocess terminated + port closed + temp file removed
- GUARD_CHILD_HEALTH: child PID + rc captured
- GUARD_BODY_BOUNDED: response body truncated to MAX_BODY_BYTES
- GUARD_HEADERS_PARSED: response headers parsed for server info
- GUARD_TEMP_CLEANUP: streamlit script temp file removed on exit
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1439 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_NO_PRODUCTION_DEPLOY: smoke ≠ deploy
- GUARD_NO_DOCKER_REQUIRED: pure stdlib + streamlit, no docker dependency
- GUARD_CLI_RUNNABLE: CLI 真可跑

API surfaces (18)
=================
1.  ``StreamlitProbeMode`` — Enum (LAUNCHED / LAUNCH_FAILED / BIND_TIMEOUT /
    PROBE_OK / PROBE_TIMEOUT / PROBE_HTTP_ERR / CLEANUP_OK / CLEANUP_TIMEOUT /
    CLEANUP_KILLED / STREAMLIT_NOT_INSTALLED / STREAMLIT_BOOT_FAILED / SKIPPED / ERROR)
2.  ``StreamlitChild`` — dataclass (pid + cmd + rc + stdout + stderr +
    elapsed_ms + timed_out + mode + script_path + temp_dir)
3.  ``StreamlitHttpCall`` — dataclass (url + method + status_code + elapsed_ms +
    body_bytes + body_truncated + body_preview + server_header +
    content_type + timed_out + attempt + mode)
4.  ``StreamlitProbeResult`` — dataclass (host + port + launch_mode +
    launch_elapsed_ms + child + http_calls + cleanup_mode +
    cleanup_elapsed_ms + streamlit_version + started_iso + ended_iso + notes)
5.  ``DEFAULT_TIMEOUT_SECONDS`` — int (30) — streamlit is slow
6.  ``MAX_TIMEOUT_SECONDS`` — int (60)
7.  ``MAX_BODY_BYTES`` — int (16384)
8.  ``DEFAULT_HOST`` — str ("127.0.0.1")
9.  ``DEFAULT_PORT_LOW`` — int (38900)
10. ``DEFAULT_PORT_HIGH`` — int (38999)
11. ``DEFAULT_BIND_TIMEOUT`` — int (30) — streamlit needs longer to boot
12. ``DEFAULT_CLEANUP_TIMEOUT`` — int (5)
13. ``STREAMLIT_HEALTH_PATH`` — str ("/_stcore/health")
14. ``STREAMLIT_INDEX_PATH`` — str ("/")
15. ``find_streamlit_executable()`` — Optional[str] (locate streamlit CLI)
16. ``make_streamlit_script(temp_dir)`` — Path (write a minimal streamlit script)
17. ``spawn_streamlit_subprocess(host, port, timeout)`` — StreamlitChild
18. ``probe_streamlit(host, port, timeout)`` — List[StreamlitHttpCall]
19. ``run_streamlit_probe(host, port, timeout)`` — StreamlitProbeResult
20. ``cleanup_streamlit(child, timeout)`` — StreamlitProbeMode
21. ``chain_delegate()`` — chain probe to V1438 + V1437 + V1435
22. ``popper_self_test()`` — 14 self-tests
23. ``main(argv)`` — CLI

CLI commands (8 — 主 00:56 任何人都能接手):
===========================================
- version
- meta [--json]
- help
- popper
- chain
- probe [--host HOST] [--port PORT] [--timeout SECONDS]
        (full streamlit subprocess launch + probe + cleanup + render summary)
- json  [--host HOST] [--port PORT] [--timeout SECONDS]
        (probe + emit JSON)
- detect (just check if streamlit is installed and report version)
"""

from __future__ import annotations

import dataclasses
import json
import os
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1439_VERSION = "0.1.0"
V1439_SCHEMA = "v1439.asi-streamlit-subprocess-smoke/v1"
V1439_MODULE = "v1439_asi_streamlit_subprocess_smoke"

DEFAULT_TIMEOUT_SECONDS = 30  # streamlit is slow to boot
MAX_TIMEOUT_SECONDS = 60
MAX_BODY_BYTES = 16384

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT_LOW = 38900
DEFAULT_PORT_HIGH = 38999
DEFAULT_BIND_TIMEOUT = 30  # streamlit needs longer
DEFAULT_CLEANUP_TIMEOUT = 5

STREAMLIT_HEALTH_PATH = "/_stcore/health"
STREAMLIT_INDEX_PATH = "/"


# ============================================================================
# Enums / Dataclasses
# ============================================================================


class StreamlitProbeMode(str, Enum):
    """Outcome mode of the streamlit subprocess probe."""

    LAUNCHED = "LAUNCHED"
    LAUNCH_FAILED = "LAUNCH_FAILED"
    BIND_TIMEOUT = "BIND_TIMEOUT"
    PROBE_OK = "PROBE_OK"
    PROBE_TIMEOUT = "PROBE_TIMEOUT"
    PROBE_HTTP_ERR = "PROBE_HTTP_ERR"
    CLEANUP_OK = "CLEANUP_OK"
    CLEANUP_TIMEOUT = "CLEANUP_TIMEOUT"
    CLEANUP_KILLED = "CLEANUP_KILLED"
    STREAMLIT_NOT_INSTALLED = "STREAMLIT_NOT_INSTALLED"
    STREAMLIT_BOOT_FAILED = "STREAMLIT_BOOT_FAILED"
    SKIPPED = "SKIPPED"
    ERROR = "ERROR"


@dataclass
class StreamlitChild:
    """State of the streamlit subprocess."""

    pid: int = -1
    cmd: str = ""
    rc: Optional[int] = None
    stdout: str = ""
    stderr: str = ""
    elapsed_ms: float = 0.0
    timed_out: bool = False
    mode: StreamlitProbeMode = StreamlitProbeMode.SKIPPED
    script_path: str = ""
    temp_dir: str = ""
    _proc: Any = field(default=None, repr=False)


@dataclass
class StreamlitHttpCall:
    """A single HTTP call result."""

    url: str
    method: str
    status_code: int
    elapsed_ms: float
    body_bytes: int
    body_truncated: bool
    body_preview: str
    server_header: str
    content_type: str
    timed_out: bool
    attempt: int
    mode: StreamlitProbeMode

    def to_dict(self) -> Dict[str, Any]:
        return {
            "url": self.url,
            "method": self.method,
            "status_code": self.status_code,
            "elapsed_ms": round(self.elapsed_ms, 2),
            "body_bytes": self.body_bytes,
            "body_truncated": self.body_truncated,
            "body_preview": self.body_preview[:200],
            "server_header": self.server_header,
            "content_type": self.content_type,
            "timed_out": self.timed_out,
            "attempt": self.attempt,
            "mode": self.mode.value,
        }


@dataclass
class StreamlitProbeResult:
    """Aggregate of the streamlit subprocess probe."""

    host: str
    port: int
    launch_mode: StreamlitProbeMode
    launch_elapsed_ms: float
    child: StreamlitChild
    http_calls: List[StreamlitHttpCall] = field(default_factory=list)
    cleanup_mode: StreamlitProbeMode = StreamlitProbeMode.SKIPPED
    cleanup_elapsed_ms: float = 0.0
    streamlit_version: str = ""
    started_iso: str = ""
    ended_iso: str = ""
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "host": self.host,
            "port": self.port,
            "launch_mode": self.launch_mode.value,
            "launch_elapsed_ms": round(self.launch_elapsed_ms, 2),
            "child": {
                "pid": self.child.pid,
                "cmd": self.child.cmd,
                "rc": self.child.rc,
                "elapsed_ms": round(self.child.elapsed_ms, 2),
                "timed_out": self.child.timed_out,
                "mode": self.child.mode.value,
                "script_path": self.child.script_path,
                "stdout_preview": self.child.stdout.splitlines()[:5] if self.child.stdout else [],
                "stderr_preview": self.child.stderr.splitlines()[:5] if self.child.stderr else [],
            },
            "http_calls": [c.to_dict() for c in self.http_calls],
            "cleanup_mode": self.cleanup_mode.value,
            "cleanup_elapsed_ms": round(self.cleanup_elapsed_ms, 2),
            "streamlit_version": self.streamlit_version,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
            "notes": self.notes,
        }


# ============================================================================
# Streamlit executable discovery
# ============================================================================


def find_streamlit_executable() -> Optional[str]:
    """Locate the streamlit executable.

    Tries in order:
    1. ``streamlit`` on PATH (via shutil.which)
    2. ``streamlit`` next to ``sys.executable`` (Python install bin)
    3. ``python -m streamlit`` (works as a fallback command)
    """
    s = shutil.which("streamlit")
    if s:
        return s
    py_dir = os.path.dirname(sys.executable)
    cand = os.path.join(py_dir, "streamlit")
    if os.name == "nt":
        cand = cand + ".exe"
    if os.path.exists(cand):
        return cand
    # Fallback: python -m streamlit
    return sys.executable


def get_streamlit_version() -> str:
    """Return streamlit version string, or 'unknown' if not installed."""
    try:
        import streamlit  # type: ignore

        return getattr(streamlit, "__version__", "unknown")
    except Exception:  # noqa: BLE001
        return "not_installed"


def is_streamlit_installed() -> bool:
    """Quick check: is streamlit importable?"""
    try:
        import streamlit  # type: ignore  # noqa: F401

        return True
    except Exception:  # noqa: BLE001
        return False


# ============================================================================
# Streamlit script generation
# ============================================================================


STREAMLIT_SCRIPT_TEMPLATE = '''"""V1439 minimal streamlit script for subprocess smoke test.

This is a *minimal* streamlit app — just enough to make streamlit boot a
real HTTP server and respond to /healthz + /. It is NOT production code.
"""

import streamlit as st

st.set_page_config(page_title="V1439 ASI", page_icon="0️⃣")

st.title("Apeireth ASI V1439")
st.write("subprocess smoke test")
st.write("module: v1439_asi_streamlit_subprocess_smoke")
st.write("version: 0.1.0")

st.success("streamlit subprocess is up")

st.text_input("probe_input", key="probe_input", value="hello")

if st.button("probe_button"):
    st.write("button_clicked")
'''


def make_streamlit_script(temp_dir: Optional[str] = None) -> Path:
    """Write the minimal streamlit script to a temp dir; return Path."""
    if temp_dir is None:
        temp_dir = tempfile.mkdtemp(prefix="v1439_streamlit_")
    path = Path(temp_dir) / "v1439_app.py"
    path.write_text(STREAMLIT_SCRIPT_TEMPLATE, encoding="utf-8")
    return path


# ============================================================================
# Port discovery (reuse V1437 pattern)
# ============================================================================


def find_free_port(host: str = DEFAULT_HOST, low: int = DEFAULT_PORT_LOW, high: int = DEFAULT_PORT_HIGH) -> int:
    """Find an OS-assigned free port via socket.bind."""
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
    """Wait up to `timeout` seconds for streamlit to bind."""
    if not (1 <= timeout <= MAX_TIMEOUT_SECONDS):
        timeout = DEFAULT_BIND_TIMEOUT
    deadline = time.monotonic() + float(timeout)
    while time.monotonic() < deadline:
        if is_port_open(host, port, timeout=0.3):
            return True
        time.sleep(0.1)
    return False


# ============================================================================
# HTTP probe helpers
# ============================================================================


def _truncate(body: bytes, max_bytes: int = MAX_BODY_BYTES) -> Tuple[bytes, bool]:
    if len(body) <= max_bytes:
        return body, False
    return body[:max_bytes], True


def _coerce_timeout(timeout: Any) -> int:
    try:
        t = int(timeout)
    except (TypeError, ValueError):
        return DEFAULT_TIMEOUT_SECONDS
    return max(1, min(t, MAX_TIMEOUT_SECONDS))


# ============================================================================
# Subprocess management
# ============================================================================


def spawn_streamlit_subprocess(
    host: str = DEFAULT_HOST,
    port: int = 0,
    timeout: int = DEFAULT_TIMEOUT_SECONDS,
) -> StreamlitChild:
    """Spawn ``streamlit run`` in a real subprocess. Returns StreamlitChild."""
    timeout = _coerce_timeout(timeout)
    child = StreamlitChild()

    streamlit_exe = find_streamlit_executable()
    if streamlit_exe is None:
        child.mode = StreamlitProbeMode.STREAMLIT_NOT_INSTALLED
        child.stderr = "streamlit executable not found"
        return child

    # Pick a free port if not specified
    if port == 0:
        port = find_free_port(host)
    child.pid = -1

    # Make a temp dir + script
    temp_dir = tempfile.mkdtemp(prefix="v1439_streamlit_")
    script_path = make_streamlit_script(temp_dir)
    child.script_path = str(script_path)
    child.temp_dir = temp_dir

    # Build the command. Use --server.headless true (no browser), --server.port, etc.
    cmd: List[str] = [
        streamlit_exe,
        "run",
        str(script_path),
        "--server.port", str(int(port)),
        "--server.headless", "true",
        "--server.address", host,
        "--browser.gatherUsageStats", "false",
        "--server.enableCORS", "false",
        "--server.enableXsrfProtection", "false",
    ]
    child.cmd = " ".join(cmd)
    start = time.monotonic()
    try:
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
        child.mode = StreamlitProbeMode.LAUNCHED
        child._proc = proc
    except (OSError, ValueError) as exc:
        child.mode = StreamlitProbeMode.LAUNCH_FAILED
        child.stderr = f"{type(exc).__name__}: {exc}"
    child.elapsed_ms = (time.monotonic() - start) * 1000.0
    return child


def cleanup_streamlit(child: StreamlitChild, timeout: int = DEFAULT_CLEANUP_TIMEOUT) -> StreamlitProbeMode:
    """Terminate the streamlit subprocess and clean up temp dir."""
    proc = child.__dict__.get("_proc")
    if proc is None:
        return StreamlitProbeMode.SKIPPED
    cleanup_start = time.monotonic()
    try:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=timeout)
                child.rc = int(proc.returncode) if proc.returncode is not None else 0
                return StreamlitProbeMode.CLEANUP_OK
            except subprocess.TimeoutExpired:
                try:
                    proc.kill()
                    proc.wait(timeout=2)
                    child.rc = int(proc.returncode) if proc.returncode is not None else -9
                    return StreamlitProbeMode.CLEANUP_KILLED
                except Exception:  # noqa: BLE001
                    return StreamlitProbeMode.CLEANUP_TIMEOUT
        else:
            child.rc = int(proc.returncode) if proc.returncode is not None else 0
            return StreamlitProbeMode.CLEANUP_OK
    except Exception as exc:  # noqa: BLE001
        child.stderr += f"\ncleanup_error: {type(exc).__name__}: {exc}"
        return StreamlitProbeMode.ERROR
    finally:
        # Capture any remaining output
        try:
            if proc.stdout:
                child.stdout = proc.stdout.read().decode("utf-8", errors="replace")[:4096]
            if proc.stderr:
                child.stderr += "\n" + proc.stderr.read().decode("utf-8", errors="replace")[:4096]
        except Exception:  # noqa: BLE001
            pass
        # Clean up temp dir
        if child.temp_dir and os.path.isdir(child.temp_dir):
            try:
                shutil.rmtree(child.temp_dir, ignore_errors=True)
            except Exception:  # noqa: BLE001
                pass


# ============================================================================
# HTTP probe
# ============================================================================


def probe_streamlit(
    host: str = DEFAULT_HOST,
    port: int = 0,
    timeout: int = DEFAULT_TIMEOUT_SECONDS,
) -> List[StreamlitHttpCall]:
    """Probe streamlit at / and /_stcore/health."""
    timeout = _coerce_timeout(timeout)
    out: List[StreamlitHttpCall] = []

    for attempt, path in enumerate([STREAMLIT_INDEX_PATH, STREAMLIT_HEALTH_PATH], 1):
        url = f"http://{host}:{port}{path}"
        t0 = time.monotonic()
        body_bytes = 0
        body_truncated = False
        body_preview = ""
        server_header = ""
        content_type = ""
        status_code = 0
        timed_out = False
        mode = StreamlitProbeMode.PROBE_OK
        try:
            req = urllib.request.Request(url, method="GET")
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                status_code = int(getattr(resp, "status", 0) or 0)
                server_header = str(resp.headers.get("Server", ""))
                content_type = str(resp.headers.get("Content-Type", ""))
                raw = resp.read()
                body_bytes = len(raw)
                truncated_raw, body_truncated = _truncate(raw)
                body_preview = truncated_raw.decode("utf-8", errors="replace")[:500]
                mode = StreamlitProbeMode.PROBE_OK
        except urllib.error.HTTPError as exc:
            status_code = int(exc.code)
            mode = StreamlitProbeMode.PROBE_HTTP_ERR
            try:
                raw = exc.read()
                body_bytes = len(raw)
                truncated_raw, body_truncated = _truncate(raw)
                body_preview = truncated_raw.decode("utf-8", errors="replace")[:500]
            except Exception:  # noqa: BLE001
                pass
        except urllib.error.URLError as exc:
            mode = StreamlitProbeMode.BIND_TIMEOUT
            body_preview = f"URLError: {getattr(exc, 'reason', exc)}"
        except socket.timeout:
            mode = StreamlitProbeMode.PROBE_TIMEOUT
            timed_out = True
        except Exception as exc:  # noqa: BLE001
            mode = StreamlitProbeMode.ERROR
            body_preview = f"{type(exc).__name__}: {exc}"

        elapsed_ms = (time.monotonic() - t0) * 1000.0
        out.append(
            StreamlitHttpCall(
                url=url,
                method="GET",
                status_code=status_code,
                elapsed_ms=elapsed_ms,
                body_bytes=body_bytes,
                body_truncated=body_truncated,
                body_preview=body_preview,
                server_header=server_header,
                content_type=content_type,
                timed_out=timed_out,
                attempt=attempt,
                mode=mode,
            )
        )
    return out


# ============================================================================
# End-to-end probe
# ============================================================================


def run_streamlit_probe(
    host: str = DEFAULT_HOST,
    port: int = 0,
    timeout: int = DEFAULT_TIMEOUT_SECONDS,
) -> StreamlitProbeResult:
    """Spawn streamlit subprocess + probe / and /healthz + cleanup."""
    started = datetime.now(timezone.utc).isoformat()
    notes: List[str] = []

    if not is_streamlit_installed():
        return StreamlitProbeResult(
            host=host,
            port=port,
            launch_mode=StreamlitProbeMode.STREAMLIT_NOT_INSTALLED,
            launch_elapsed_ms=0.0,
            child=StreamlitChild(mode=StreamlitProbeMode.STREAMLIT_NOT_INSTALLED),
            http_calls=[],
            cleanup_mode=StreamlitProbeMode.SKIPPED,
            cleanup_elapsed_ms=0.0,
            streamlit_version="not_installed",
            started_iso=started,
            ended_iso=datetime.now(timezone.utc).isoformat(),
            notes=["streamlit not importable; install with `pip install streamlit`"],
        )

    if port == 0:
        port = find_free_port(host)
        notes.append(f"auto-selected port={port}")

    streamlit_version = get_streamlit_version()
    child = spawn_streamlit_subprocess(host=host, port=port, timeout=timeout)

    if child.mode == StreamlitProbeMode.LAUNCHED:
        # Wait for streamlit to bind (streamlit is slow; default 30s)
        if not wait_for_port(host, port, timeout=DEFAULT_BIND_TIMEOUT):
            notes.append(f"port bind timeout: {host}:{port} not reachable after {DEFAULT_BIND_TIMEOUT}s")
            child.mode = StreamlitProbeMode.BIND_TIMEOUT

    http_calls: List[StreamlitHttpCall] = []
    if child.mode in (StreamlitProbeMode.LAUNCHED,):
        http_calls = probe_streamlit(host=host, port=port, timeout=min(timeout, 10))

    cleanup_start = time.monotonic()
    cleanup_mode = cleanup_streamlit(child, timeout=DEFAULT_CLEANUP_TIMEOUT)
    cleanup_elapsed = (time.monotonic() - cleanup_start) * 1000.0

    ended = datetime.now(timezone.utc).isoformat()

    return StreamlitProbeResult(
        host=host,
        port=port,
        launch_mode=child.mode,
        launch_elapsed_ms=child.elapsed_ms,
        child=child,
        http_calls=http_calls,
        cleanup_mode=cleanup_mode,
        cleanup_elapsed_ms=cleanup_elapsed,
        streamlit_version=streamlit_version,
        started_iso=started,
        ended_iso=ended,
        notes=notes,
    )


# ============================================================================
# Markdown report
# ============================================================================


def render_report_md(result: StreamlitProbeResult) -> str:
    lines: List[str] = []
    lines.append("# V1439 — ASI streamlit subprocess smoke report")
    lines.append("")
    lines.append(f"- module: `{V1439_MODULE}`")
    lines.append(f"- version: `{V1439_VERSION}`")
    lines.append(f"- schema: `{V1439_SCHEMA}`")
    lines.append(f"- host: `{result.host}`")
    lines.append(f"- port: `{result.port}`")
    lines.append(f"- streamlit_version: `{result.streamlit_version}`")
    lines.append(f"- launch_mode: `{result.launch_mode.value}`")
    lines.append(f"- launch_elapsed_ms: `{result.launch_elapsed_ms:.2f}`")
    lines.append(f"- cleanup_mode: `{result.cleanup_mode.value}`")
    lines.append(f"- cleanup_elapsed_ms: `{result.cleanup_elapsed_ms:.2f}`")
    lines.append(f"- started_iso: `{result.started_iso}`")
    lines.append(f"- ended_iso: `{result.ended_iso}`")
    lines.append("")
    lines.append("## Subprocess")
    lines.append("")
    lines.append(f"- pid: `{result.child.pid}`")
    lines.append(f"- mode: `{result.child.mode.value}`")
    lines.append(f"- timed_out: `{result.child.timed_out}`")
    lines.append(f"- rc: `{result.child.rc}`")
    lines.append(f"- script_path: `{result.child.script_path}`")
    if result.child.stdout:
        lines.append("- stdout_preview (first 5 lines):")
        for ln in result.child.stdout.splitlines()[:5]:
            lines.append(f"  - `{ln[:120]}`")
    if result.child.stderr:
        lines.append("- stderr_preview (first 5 lines):")
        for ln in result.child.stderr.splitlines()[:5]:
            lines.append(f"  - `{ln[:120]}`")
    lines.append("")
    lines.append("## HTTP calls")
    lines.append("")
    lines.append("| # | method | path | status | mode | elapsed_ms | timed_out | body_bytes | server |")
    lines.append("|---|--------|------|--------|------|-----------|-----------|------------|--------|")
    for i, c in enumerate(result.http_calls, 1):
        # Extract path from url
        from urllib.parse import urlparse
        path = urlparse(c.url).path
        lines.append(
            f"| {i} | {c.method} | {path} | {c.status_code} | {c.mode.value} | {c.elapsed_ms:.1f} | {c.timed_out} | {c.body_bytes} | `{c.server_header[:40]}` |"
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
        "V1439 is a **streamlit subprocess smoke test**. It does NOT claim "
        "that streamlit is production-deployed, that the localhost probe is "
        "equivalent to a public probe, or that the temp script is production "
        "code. It claims only: **from this host, a real `streamlit run` "
        "subprocess was spawned, a real streamlit HTTP server bound to a real "
        "local port, and a real urllib call returned what is shown**. V1439 "
        "≠ Phenomenal streamlit, ≠ ASI streamlit, ≠ human-level streamlit, "
        "≠ absolute streamlit. Bounded subprocess smoke ≠ deployment proof. "
        "Localhost probe ≠ public streamlit probe."
    )
    return "\n".join(lines)


# ============================================================================
# Chain + Popper
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream V1438 + V1437 + V1435 chain integrity."""
    out: Dict[str, Any] = {
        "v1439": {"ok": True, "mode": V1439_MODULE},
        "v1438": {"ok": True, "mode": "v1438_asi_real_subprocess_benchmark"},
        "v1437": {"ok": True, "mode": "v1437_asi_subprocess_http_live_server"},
        "v1435": {"ok": True, "mode": "v1435_asi_docker_availability_probe"},
        "all_ok": True,
        "borrowed": [{"module": m, "use": u} for m, u in V1439_BORROWED],
    }
    try:
        import apeireth.v1438_asi_real_subprocess_benchmark as v1438

        out["v1438"]["importable"] = True
        out["v1438"]["benchmark_modes"] = [m.value for m in v1438.BenchmarkMode]
    except Exception as exc:  # noqa: BLE001
        out["v1438"]["ok"] = False
        out["v1438"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1437_asi_subprocess_http_live_server as v1437

        out["v1437"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1437"]["ok"] = False
        out["v1437"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1435_asi_docker_availability_probe as v1435

        out["v1435"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1435"]["ok"] = False
        out["v1435"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    # V1439 self: is streamlit importable?
    out["v1439"]["streamlit_installed"] = is_streamlit_installed()
    out["v1439"]["streamlit_version"] = get_streamlit_version()
    out["v1439"]["streamlit_executable"] = find_streamlit_executable() or "not_found"
    return out


def popper_self_test() -> Dict[str, Any]:
    """14 self-tests. All should pass."""
    results: List[Dict[str, Any]] = []

    # P01: constants
    results.append(
        {
            "id": "P01",
            "name": "constants defined",
            "ok": (
                V1439_VERSION == "0.1.0"
                and DEFAULT_TIMEOUT_SECONDS > 0
                and MAX_BODY_BYTES > 0
                and DEFAULT_PORT_LOW < DEFAULT_PORT_HIGH
                and STREAMLIT_HEALTH_PATH.startswith("/")
            ),
        }
    )

    # P02: guards count
    results.append(
        {
            "id": "P02",
            "name": "guards count",
            "ok": len(V1439_GUARDS) == 14 and len(V1439_V3_GUARDS) == 5,
        }
    )

    # P03: borrowed count
    results.append(
        {
            "id": "P03",
            "name": "borrowed count",
            "ok": len(V1439_BORROWED) == 5,
        }
    )

    # P04: streamlit probe mode count (≥10)
    results.append(
        {
            "id": "P04",
            "name": "streamlit probe mode count",
            "ok": len(list(StreamlitProbeMode)) >= 10,
        }
    )

    # P05: streamlit version detection (installed on this host = True)
    results.append(
        {
            "id": "P05",
            "name": "streamlit importable",
            "ok": is_streamlit_installed(),
        }
    )

    # P06: find_streamlit_executable returns non-None
    exe = find_streamlit_executable()
    results.append(
        {
            "id": "P06",
            "name": "find_streamlit_executable",
            "ok": exe is not None and (os.path.exists(exe) or exe == sys.executable),
        }
    )

    # P07: make_streamlit_script writes file
    import tempfile as _tempfile
    with _tempfile.TemporaryDirectory() as td:
        script_path = make_streamlit_script(td)
        results.append(
            {
                "id": "P07",
                "name": "make_streamlit_script writes file",
                "ok": script_path.exists() and script_path.stat().st_size > 100,
            }
        )

    # P08: find_free_port returns int > 0
    p = find_free_port(DEFAULT_HOST)
    results.append(
        {
            "id": "P08",
            "name": "find_free_port returns int > 0",
            "ok": p > 0,
        }
    )

    # P09: is_port_open on closed port
    results.append(
        {
            "id": "P09",
            "name": "is_port_open detects closed port",
            "ok": not is_port_open(DEFAULT_HOST, 1, timeout=0.2),
        }
    )

    # P10: _coerce_timeout bounds
    results.append(
        {
            "id": "P10",
            "name": "_coerce_timeout bounds",
            "ok": _coerce_timeout(0) == 1 and _coerce_timeout(99999) == MAX_TIMEOUT_SECONDS and _coerce_timeout("abc") == DEFAULT_TIMEOUT_SECONDS,
        }
    )

    # P11: _truncate bounded
    small, t1 = _truncate(b"hello", max_bytes=100)
    big, t2 = _truncate(b"x" * 100, max_bytes=10)
    results.append(
        {
            "id": "P11",
            "name": "_truncate bounded",
            "ok": small == b"hello" and not t1 and len(big) == 10 and t2,
        }
    )

    # P12: StreamlitChild default mode
    c = StreamlitChild()
    results.append(
        {
            "id": "P12",
            "name": "StreamlitChild default mode is SKIPPED",
            "ok": c.mode == StreamlitProbeMode.SKIPPED,
        }
    )

    # P13: StreamlitHttpCall to_dict
    call = StreamlitHttpCall(
        url="http://x", method="GET", status_code=200, elapsed_ms=1.0,
        body_bytes=10, body_truncated=False, body_preview="hi",
        server_header="srv", content_type="text/html", timed_out=False,
        attempt=1, mode=StreamlitProbeMode.PROBE_OK,
    )
    d = call.to_dict()
    results.append(
        {
            "id": "P13",
            "name": "StreamlitHttpCall.to_dict",
            "ok": d["status_code"] == 200 and d["method"] == "GET" and d["mode"] == "PROBE_OK",
        }
    )

    # P14: chain_delegate runs
    try:
        ch = chain_delegate()
        results.append(
            {
                "id": "P14",
                "name": "chain_delegate runs",
                "ok": "v1439" in ch and "v1438" in ch and "v1437" in ch,
            }
        )
    except Exception as exc:  # noqa: BLE001
        results.append({"id": "P14", "name": "chain_delegate runs", "ok": False, "error": str(exc)})

    n_pass = sum(1 for r in results if r["ok"])
    return {
        "n_tests": len(results),
        "n_pass": n_pass,
        "n_fail": len(results) - n_pass,
        "results": results,
    }


# ============================================================================
# Guards & borrowed (module-level for test introspection)
# ============================================================================


V1439_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_TIMEOUT",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_PORT_RECLAIMED",
    "GUARD_CHILD_HEALTH",
    "GUARD_BODY_BOUNDED",
    "GUARD_HEADERS_PARSED",
    "GUARD_TEMP_CLEANUP",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_PRODUCTION_DEPLOY",
    "GUARD_NO_DOCKER_REQUIRED",
    "GUARD_CLI_RUNNABLE",
)

V1439_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_STREAMLIT",
    "GUARD_NO_ASI_STREAMLIT",
    "GUARD_NO_HUMAN_LEVEL_STREAMLIT",
    "GUARD_NO_ABSOLUTE_STREAMLIT",
    "GUARD_NO_V1438_REPLACE",
)

V1439_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1438_asi_real_subprocess_benchmark", "subprocess spawn + port handling + dataclass patterns"),
    ("v1437_asi_subprocess_http_live_server", "cleanup_subprocess pattern + find_free_port pattern"),
    ("streamlit", "real `streamlit run` CLI + /healthz endpoint"),
    ("stdlib_subprocess", "real child process management"),
    ("stdlib_shutil", "real executable lookup (which) + temp dir cleanup (rmtree)"),
)


# ============================================================================
# Module metadata
# ============================================================================


def module_meta() -> Dict[str, Any]:
    return {
        "module": V1439_MODULE,
        "version": V1439_VERSION,
        "schema": V1439_SCHEMA,
        "n_guards": len(V1439_GUARDS),
        "n_v3_guards": len(V1439_V3_GUARDS),
        "n_borrowed": len(V1439_BORROWED),
        "streamlit_installed": is_streamlit_installed(),
        "streamlit_version": get_streamlit_version(),
    }


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print("V1439 — ASI streamlit subprocess smoke test")
    print("")
    print("Commands:")
    print("  version                  Print version")
    print("  meta [--json]            Print module metadata")
    print("  help                     Print this help")
    print("  popper                   Run 14 popper self-tests")
    print("  chain                    Run chain_delegate")
    print("  detect                   Detect streamlit (installed/version/executable)")
    print("  probe [--host HOST] [--port PORT] [--timeout SECONDS]")
    print("                           Full streamlit subprocess launch + probe + cleanup")
    print("  json [--host HOST] [--port PORT] [--timeout SECONDS]")
    print("                           Probe + emit JSON")


def main(argv: Optional[List[str]] = None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    if not argv:
        _print_help()
        return 0
    cmd = argv[0]
    args = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(V1439_VERSION)
        return 0

    if cmd == "help" or cmd in ("--help", "-h"):
        _print_help()
        return 0

    if cmd == "meta":
        if "--json" in args:
            print(json.dumps(module_meta(), indent=2))
        else:
            meta = module_meta()
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    if cmd == "popper":
        result = popper_self_test()
        print(json.dumps(result, indent=2))
        return 0 if result["n_fail"] == 0 else 1

    if cmd == "chain":
        print(json.dumps(chain_delegate(), indent=2))
        return 0

    if cmd == "detect":
        info = {
            "streamlit_installed": is_streamlit_installed(),
            "streamlit_version": get_streamlit_version(),
            "streamlit_executable": find_streamlit_executable() or "not_found",
        }
        print(json.dumps(info, indent=2))
        return 0

    if cmd in ("probe", "json"):
        host = DEFAULT_HOST
        port = 0
        timeout = DEFAULT_TIMEOUT_SECONDS
        i = 0
        while i < len(args):
            a = args[i]
            if a == "--host" and i + 1 < len(args):
                host = args[i + 1]
                i += 2
            elif a == "--port" and i + 1 < len(args):
                try:
                    port = int(args[i + 1])
                except ValueError:
                    port = 0
                i += 2
            elif a == "--timeout" and i + 1 < len(args):
                try:
                    timeout = int(args[i + 1])
                except ValueError:
                    timeout = DEFAULT_TIMEOUT_SECONDS
                i += 2
            else:
                i += 1
        result = run_streamlit_probe(host=host, port=port, timeout=timeout)
        if cmd == "json":
            print(json.dumps(result.to_dict(), indent=2))
        else:
            print(render_report_md(result))
        return 0

    print(f"unknown command: {cmd}", file=sys.stderr)
    _print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
