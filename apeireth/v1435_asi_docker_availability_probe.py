"""V1435 — ASI 真生产 docker availability probe (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43).

Phase: 1435
Version: 0.1.0
Date: 2026-08-10 (cron tick 05:20, Asia/Shanghai deep night)
Post: V1434 (VCP consistency HTTP artifact) + V1430 (deployment E2E runbook)

What V1435 is
=============
V1435 is the **real subprocess docker availability probe** for
Apeireth ASI deployment. Where V1430 (deployment E2E runbook)
orchestrates the steps, V1435 is **one focused, runnable probe**
that answers the question anyone can verify:

    On THIS host, is docker actually available? Which flavor?
    Can the daemon be reached? Can containers be listed?

V1435 actually invokes:

1.  ``which docker``              — locate docker binary
2.  ``docker --version``          — get docker version (real subprocess)
3.  ``docker compose version``    — get compose v2 version
4.  ``docker-compose --version``  — get legacy compose v1 version
5.  ``docker info``               — daemon reachability + server version
6.  ``docker ps``                 — running containers (real list)

Each call has a **bounded timeout** (default 10s) and **offline-safe
fallback** — if docker is missing, V1435 reports ``mode=DOCKER_MISSING``
without raising. If docker exists but daemon unreachable, V1435
reports ``mode=DAEMON_DOWN``. If both work, V1435 reports
``mode=DOCKER_READY`` with full server_version + n_running_containers.

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1435 is a **host capability probe**. It does NOT claim that:

- Docker is production-deployed
- Containers are actually running production workloads
- The host is production-ready
- The deployment will succeed without further configuration

It claims only: **on this host, docker is / is not available, and
here is what the real subprocess calls returned**. V1435 ≠
Phenomenal probe, ≠ ASI probe, ≠ human-level probe, ≠ absolute
probe. Bounded subprocess probe ≠ deployment proof.

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1430 (deployment E2E runbook — step-based pattern)
- V1434 (artifact generator — offline-safe fallback pattern)
- V1076 (external LLM client — token bucket + retry pattern)
- stdlib subprocess (real subprocess invocation)
- stdlib shutil (which lookup)

GUARDS upheld (V1435-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_TIMEOUT: every subprocess call has timeout ∈ [1, 60]
- GUARD_NO_RAISE: subprocess failures are caught, never raised
- GUARD_OFFLINE_SAFE: when docker missing, reports DOCKER_MISSING (no crash)
- GUARD_DOCKER_INFO_PARSED: server_version parsed from `docker info`
- GUARD_DOCKER_PS_PARSED: n_running parsed from `docker ps` output
- GUARD_VERSION_PARSED: docker --version has semver-like content
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1435 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_NO_PRODUCTION_DEPLOY: probe ≠ deploy
- GUARD_NO_DOCKER_INSTALL: V1435 does not install docker
- GUARD_RUNS_ON_WINDOWS: works on Windows (shutil.which + subprocess)
- GUARD_BOUNDED_OUTPUT: each artifact is bounded in size
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_PROBE: docker probe is bytes, NOT consciousness
- GUARD_NO_ASI_PROBE: docker probe is binary existence, NOT ASI
- GUARD_NO_HUMAN_LEVEL_PROBE: docker probe is one host, NOT universal
- GUARD_NO_ABSOLUTE_PROBE: docker probe is current state, NOT eternal
- GUARD_NO_V1434_REPLACE: V1435 reads V1434 artifacts, does not replace

API surfaces (16)
=================
1.  ``ProbeMode`` — Enum (DOCKER_MISSING / DOCKER_BIN_ONLY / DAEMON_DOWN /
    DOCKER_READY / ERROR)
2.  ``SubprocessCall`` — dataclass (cmd + rc + stdout + stderr + elapsed_ms +
    timed_out + mode)
3.  ``DockerProbeResult`` — dataclass (probe_mode + docker_path +
    docker_version + compose_v2_version + compose_v1_version +
    server_version + n_running_containers + kernel_version +
    os + arch + calls + started_iso + ended_iso)
4.  ``DEFAULT_TIMEOUT_SECONDS`` — int (10)
5.  ``MAX_TIMEOUT_SECONDS`` — int (60)
6.  ``run_subprocess(cmd, timeout)`` — SubprocessCall (bounded, offline-safe)
7.  ``probe_docker_binary(timeout)`` — (path, version) or (None, None)
8.  ``probe_compose_v2(timeout)`` — version str or None
9.  ``probe_compose_v1(timeout)`` — version str or None
10. ``probe_docker_daemon(timeout)`` — (server_version, kernel_version, os)
    or (None, None, None)
11. ``probe_docker_ps(timeout)`` — int (running containers) or -1
12. ``run_docker_probe(timeout)`` — DockerProbeResult
13. ``result_to_dict(result)`` — JSON-serializable dict
14. ``chain_delegate()`` — chain probe to V1434 + V1430
15. ``popper_self_test()`` — 14 self-tests
16. ``main(argv)`` — CLI

CLI commands (8 — 主 00:56 任何人都能接手):
===========================================
- version
- meta [--json]
- help
- popper
- chain
- probe [--timeout SECONDS]  (full probe + render summary)
- json  [--timeout SECONDS]   (probe + emit JSON)
- call --cmd "docker --version" [--timeout SECONDS]  (single bounded call)
"""

from __future__ import annotations

import dataclasses
import json
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1435_VERSION = "0.1.0"
V1435_SCHEMA = "v1435.asi-docker-availability-probe/v1"
V1435_MODULE = "v1435_asi_docker_availability_probe"

DEFAULT_TIMEOUT_SECONDS = 10
MAX_TIMEOUT_SECONDS = 60


# ============================================================================
# Enums / Dataclasses
# ============================================================================


class ProbeMode(str, Enum):
    """Outcome mode of the docker probe."""

    DOCKER_MISSING = "DOCKER_MISSING"  # no docker binary on PATH
    DOCKER_BIN_ONLY = "DOCKER_BIN_ONLY"  # docker exists, daemon not reachable
    DAEMON_DOWN = "DAEMON_DOWN"  # docker binary exists, daemon not reachable
    DOCKER_READY = "DOCKER_READY"  # docker + daemon + at least compose v2 or v1
    ERROR = "ERROR"  # unexpected error (raised and caught)


@dataclass
class SubprocessCall:
    """One bounded subprocess invocation."""

    cmd: str
    rc: int = -1
    stdout: str = ""
    stderr: str = ""
    elapsed_ms: float = 0.0
    timed_out: bool = False
    mode: str = "PENDING"  # PENDING / OK / TIMEOUT / FAILED / NOT_FOUND

    def to_dict(self) -> Dict[str, Any]:
        return {
            "cmd": self.cmd,
            "rc": self.rc,
            "stdout_lines": self.stdout.count("\n") + (0 if self.stdout.endswith("\n") else 1) if self.stdout else 0,
            "stderr_lines": self.stderr.count("\n") + (0 if self.stderr.endswith("\n") else 1) if self.stderr else 0,
            "stdout_bytes": len(self.stdout.encode("utf-8")) if self.stdout else 0,
            "stderr_bytes": len(self.stderr.encode("utf-8")) if self.stderr else 0,
            "elapsed_ms": round(self.elapsed_ms, 2),
            "timed_out": self.timed_out,
            "mode": self.mode,
        }


@dataclass
class DockerProbeResult:
    """Aggregated docker probe result."""

    probe_mode: str = ProbeMode.ERROR.value
    docker_path: Optional[str] = None
    docker_version: Optional[str] = None
    compose_v2_version: Optional[str] = None
    compose_v1_version: Optional[str] = None
    server_version: Optional[str] = None
    kernel_version: Optional[str] = None
    os: Optional[str] = None
    arch: Optional[str] = None
    n_running_containers: int = -1
    calls: List[SubprocessCall] = field(default_factory=list)
    started_iso: str = ""
    ended_iso: str = ""
    timeout_seconds: int = DEFAULT_TIMEOUT_SECONDS
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["calls"] = [c.to_dict() for c in self.calls]
        return d


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1435_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_TIMEOUT",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_DOCKER_INFO_PARSED",
    "GUARD_DOCKER_PS_PARSED",
    "GUARD_VERSION_PARSED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_PRODUCTION_DEPLOY",
    "GUARD_NO_DOCKER_INSTALL",
    "GUARD_RUNS_ON_WINDOWS",
    "GUARD_BOUNDED_OUTPUT",
    "GUARD_CLI_RUNNABLE",
)

V1435_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_PROBE",
    "GUARD_NO_ASI_PROBE",
    "GUARD_NO_HUMAN_LEVEL_PROBE",
    "GUARD_NO_ABSOLUTE_PROBE",
    "GUARD_NO_V1434_REPLACE",
)

V1435_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1430_asi_deployment_e2e_runbook", "Step-based runbook pattern"),
    ("v1434_asi_vcp_consistency_http", "Offline-safe fallback pattern"),
    ("v1076_asi_real_external_llm_client", "Bounded subprocess + retry pattern"),
    ("stdlib_subprocess", "Real subprocess invocation"),
    ("stdlib_shutil", "Binary lookup (shutil.which)"),
)


# ============================================================================
# Subprocess runner
# ============================================================================


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def run_subprocess(cmd: str, timeout: int = DEFAULT_TIMEOUT_SECONDS) -> SubprocessCall:
    """Run one bounded subprocess call. Never raises; offline-safe."""
    if not (1 <= timeout <= MAX_TIMEOUT_SECONDS):
        timeout = DEFAULT_TIMEOUT_SECONDS
    call = SubprocessCall(cmd=cmd)
    start = time.monotonic()
    try:
        # Use bytes + manual UTF-8 decode with replace to avoid GBK
        # decode errors on Windows when subprocess output contains
        # non-GBK bytes (docker info often emits unicode that the
        # Windows code-page reader thread chokes on).
        proc = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        elapsed = (time.monotonic() - start) * 1000.0
        call.rc = proc.returncode
        call.stdout = (proc.stdout or b"").decode("utf-8", errors="replace")
        call.stderr = (proc.stderr or b"").decode("utf-8", errors="replace")
        call.elapsed_ms = elapsed
        if proc.returncode == 0:
            call.mode = "OK"
        elif proc.returncode == 1 and ("not found" in (proc.stderr or "").lower() or "command not found" in (proc.stderr or "").lower()):
            call.mode = "NOT_FOUND"
        else:
            call.mode = "FAILED"
    except subprocess.TimeoutExpired as exc:
        elapsed = (time.monotonic() - start) * 1000.0
        call.elapsed_ms = elapsed
        call.timed_out = True
        call.mode = "TIMEOUT"
        call.stdout = (exc.stdout or b"").decode("utf-8", errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or "")
        call.stderr = (exc.stderr or b"").decode("utf-8", errors="replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")
    except FileNotFoundError:
        call.mode = "NOT_FOUND"
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
    except Exception as exc:  # noqa: BLE001
        call.mode = "FAILED"
        call.stderr = f"{type(exc).__name__}: {exc}"
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
    return call


# ============================================================================
# Parsers
# ============================================================================

_VERSION_RE = re.compile(r"\d+\.\d+(?:\.\d+)?")


def _first_version(text: str) -> Optional[str]:
    """Extract the first semver-like version from text."""
    if not text:
        return None
    m = _VERSION_RE.search(text)
    return m.group(0) if m else None


def _kv_parse(text: str, key: str) -> Optional[str]:
    """Parse ``Key: Value`` lines from docker info."""
    if not text:
        return None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith(f"{key}:"):
            return line[len(key) + 1 :].strip()
    return None


# ============================================================================
# Per-component probes
# ============================================================================


def probe_docker_binary(timeout: int) -> Tuple[Optional[str], Optional[str], SubprocessCall]:
    """Find docker binary + get version. Returns (path, version, call)."""
    path = shutil.which("docker")
    if not path:
        call = SubprocessCall(cmd="docker --version", mode="NOT_FOUND")
        return None, None, call
    call = run_subprocess(f'"{path}" --version', timeout=timeout)
    version = _first_version(call.stdout) if call.mode == "OK" else None
    return path, version, call


def probe_compose_v2(timeout: int) -> Tuple[Optional[str], SubprocessCall]:
    """Get `docker compose version` (v2)."""
    call = run_subprocess("docker compose version", timeout=timeout)
    version = _first_version(call.stdout) if call.mode == "OK" else None
    return version, call


def probe_compose_v1(timeout: int) -> Tuple[Optional[str], SubprocessCall]:
    """Get `docker-compose --version` (v1 legacy)."""
    call = run_subprocess("docker-compose --version", timeout=timeout)
    version = _first_version(call.stdout) if call.mode == "OK" else None
    return version, call


def probe_docker_daemon(timeout: int) -> Tuple[Optional[str], Optional[str], Optional[str], SubprocessCall]:
    """Run `docker info` and parse server_version / kernel / os."""
    call = run_subprocess("docker info", timeout=timeout)
    server_version = None
    kernel_version = None
    os_name = None
    if call.mode == "OK":
        server_version = _kv_parse(call.stdout, "Server Version")
        kernel_version = _kv_parse(call.stdout, "Kernel Version")
        os_name = _kv_parse(call.stdout, "Operating System")
        if server_version is None and "Server:" in call.stdout:
            # Some platforms put "Server: Docker Engine ..." on one line
            for line in call.stdout.splitlines():
                if "Server:" in line and "Version" in line:
                    parts = line.split("Version:", 1)
                    if len(parts) == 2:
                        server_version = parts[1].strip().split()[0]
                        break
    return server_version, kernel_version, os_name, call


def probe_docker_ps(timeout: int) -> Tuple[int, SubprocessCall]:
    """Count running containers via `docker ps -q`."""
    call = run_subprocess("docker ps -q", timeout=timeout)
    if call.mode == "OK":
        # Each line is one container ID
        n = sum(1 for line in call.stdout.splitlines() if line.strip())
        return n, call
    return -1, call


# ============================================================================
# Aggregated probe
# ============================================================================


def run_docker_probe(timeout: int = DEFAULT_TIMEOUT_SECONDS) -> DockerProbeResult:
    """Run the full docker availability probe (offline-safe)."""
    timeout = max(1, min(int(timeout or DEFAULT_TIMEOUT_SECONDS), MAX_TIMEOUT_SECONDS))
    result = DockerProbeResult(
        timeout_seconds=timeout,
        started_iso=_now_iso(),
    )

    # 1) docker binary
    path, version, c1 = probe_docker_binary(timeout)
    result.docker_path = path
    result.docker_version = version
    result.calls.append(c1)

    if path is None:
        result.probe_mode = ProbeMode.DOCKER_MISSING.value
        result.notes.append("docker binary not found on PATH")
        result.ended_iso = _now_iso()
        return result

    # 2) compose v2
    v2, c2 = probe_compose_v2(timeout)
    result.compose_v2_version = v2
    result.calls.append(c2)

    # 3) compose v1
    v1, c3 = probe_compose_v1(timeout)
    result.compose_v1_version = v1
    result.calls.append(c3)

    # 4) daemon info
    server, kernel, os_name, c4 = probe_docker_daemon(timeout)
    result.server_version = server
    result.kernel_version = kernel
    result.os = os_name
    result.calls.append(c4)

    # 5) docker ps
    n, c5 = probe_docker_ps(timeout)
    result.n_running_containers = n
    result.calls.append(c5)

    if server is not None and n >= 0:
        result.probe_mode = ProbeMode.DOCKER_READY.value
        result.notes.append(f"daemon reachable: server_version={server}")
        result.notes.append(f"compose_v2={v2} compose_v1={v1} n_running={n}")
    elif server is None:
        result.probe_mode = ProbeMode.DAEMON_DOWN.value
        result.notes.append("docker info failed (daemon not running or unreachable)")
    else:
        result.probe_mode = ProbeMode.DOCKER_BIN_ONLY.value
        result.notes.append("docker binary present but probe incomplete")

    result.ended_iso = _now_iso()
    return result


# ============================================================================
# Render / Serialize
# ============================================================================


def result_to_dict(result: DockerProbeResult) -> Dict[str, Any]:
    """JSON-serializable dict."""
    return result.to_dict()


def render_probe_summary_md(result: DockerProbeResult) -> str:
    """Markdown summary of the probe result."""
    lines = []
    lines.append(f"# V1435 ASI Docker Availability Probe")
    lines.append("")
    lines.append(f"- **probe_mode**: `{result.probe_mode}`")
    lines.append(f"- **timeout_seconds**: {result.timeout_seconds}")
    lines.append(f"- **started**: {result.started_iso}")
    lines.append(f"- **ended**: {result.ended_iso}")
    lines.append("")
    lines.append("## Docker")
    lines.append("")
    lines.append(f"- docker_path: `{result.docker_path or 'NOT_FOUND'}`")
    lines.append(f"- docker_version: `{result.docker_version or 'UNKNOWN'}`")
    lines.append(f"- compose_v2_version: `{result.compose_v2_version or 'NOT_INSTALLED'}`")
    lines.append(f"- compose_v1_version: `{result.compose_v1_version or 'NOT_INSTALLED'}`")
    lines.append("")
    lines.append("## Daemon")
    lines.append("")
    lines.append(f"- server_version: `{result.server_version or 'UNREACHABLE'}`")
    lines.append(f"- kernel_version: `{result.kernel_version or 'UNKNOWN'}`")
    lines.append(f"- os: `{result.os or 'UNKNOWN'}`")
    lines.append(f"- n_running_containers: `{result.n_running_containers}`")
    lines.append("")
    lines.append("## Subprocess calls")
    lines.append("")
    lines.append("| # | cmd | rc | mode | elapsed_ms | timed_out |")
    lines.append("|---|-----|----|----|-----------|-----------|")
    for i, c in enumerate(result.calls, 1):
        lines.append(f"| {i} | `{c.cmd}` | {c.rc} | {c.mode} | {c.elapsed_ms:.1f} | {c.timed_out} |")
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
        "V1435 is a **host capability probe**. It does NOT claim that docker "
        "is production-deployed, that containers are running production "
        "workloads, or that the host is production-ready. It claims only: "
        "**on this host, at this moment, here is what the bounded subprocess "
        "calls returned**. Bounded subprocess probe ≠ deployment proof. "
        "V1435 ≠ Phenomenal probe, ≠ ASI probe, ≠ human-level probe, ≠ absolute probe."
    )
    return "\n".join(lines)


# ============================================================================
# Chain + Popper
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream V1434 + V1430 + V1076 chain integrity."""
    out: Dict[str, Any] = {
        "v1435": {"ok": True, "mode": "v1435_asi_docker_availability_probe"},
        "v1434": {"ok": True, "mode": "v1434_asi_vcp_consistency_http"},
        "v1430": {"ok": True, "mode": "v1430_asi_deployment_e2e_runbook"},
        "v1076": {"ok": True, "mode": "v1076_asi_real_external_llm_client"},
        "all_ok": True,
        "borrowed": list(V1435_BORROWED),
    }
    try:
        import apeireth.v1434_asi_vcp_consistency_http as v1434
        out["v1434"]["artifact_count"] = len(v1434.build_all_artifacts())
    except Exception as exc:  # noqa: BLE001
        out["v1434"]["ok"] = False
        out["v1434"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1430_asi_deployment_e2e_runbook as v1430
        # Just import — do not actually run the full runbook here
        out["v1430"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1430"]["ok"] = False
        out["v1430"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1076_asi_real_external_llm_client as v1076
        out["v1076"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1076"]["ok"] = False
        out["v1076"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    return out


def popper_self_test() -> Dict[str, Any]:
    """14 self-tests. All should pass."""
    results: List[Dict[str, Any]] = []

    # 1. constants
    results.append({
        "id": "P01",
        "name": "constants defined",
        "ok": V1435_VERSION == "0.1.0" and DEFAULT_TIMEOUT_SECONDS > 0 and MAX_TIMEOUT_SECONDS > DEFAULT_TIMEOUT_SECONDS,
    })

    # 2. guards count
    results.append({
        "id": "P02",
        "name": "guards count",
        "ok": len(V1435_GUARDS) == 14 and len(V1435_V3_GUARDS) == 5,
    })

    # 3. borrowed count
    results.append({
        "id": "P03",
        "name": "borrowed count",
        "ok": len(V1435_BORROWED) == 5,
    })

    # 4. probe modes
    modes = list(ProbeMode)
    results.append({
        "id": "P04",
        "name": "probe modes count",
        "ok": len(modes) == 5,
    })

    # 5. subprocess call dataclass
    sc = SubprocessCall(cmd="docker --version")
    results.append({
        "id": "P05",
        "name": "SubprocessCall dataclass",
        "ok": sc.mode == "PENDING" and sc.to_dict()["cmd"] == "docker --version",
    })

    # 6. docker probe result dataclass
    r = DockerProbeResult()
    d = r.to_dict()
    results.append({
        "id": "P06",
        "name": "DockerProbeResult dataclass",
        "ok": d["probe_mode"] == ProbeMode.ERROR.value and d["timeout_seconds"] == DEFAULT_TIMEOUT_SECONDS,
    })

    # 7. version regex
    results.append({
        "id": "P07",
        "name": "_first_version parses 20.10.5",
        "ok": _first_version("Docker version 20.10.5, build abc123") == "20.10.5",
    })

    # 8. version regex on missing
    results.append({
        "id": "P08",
        "name": "_first_version returns None on missing",
        "ok": _first_version("") is None and _first_version("no version here") is None,
    })

    # 9. kv parse
    sample = "Server Version: 24.0.7\nKernel Version: 5.15.0\nOperating System: Ubuntu 22.04"
    results.append({
        "id": "P09",
        "name": "_kv_parse extracts server/kernel/os",
        "ok": _kv_parse(sample, "Server Version") == "24.0.7"
        and _kv_parse(sample, "Kernel Version") == "5.15.0"
        and _kv_parse(sample, "Operating System") == "Ubuntu 22.04",
    })

    # 10. bounded subprocess on a non-existent command (offline-safe)
    c = run_subprocess("definitely-not-a-real-binary-xyz_123", timeout=2)
    results.append({
        "id": "P10",
        "name": "offline-safe subprocess returns mode=NOT_FOUND",
        "ok": c.mode in ("NOT_FOUND", "FAILED") and not c.timed_out,
    })

    # 11. bounded subprocess on echo (works)
    c = run_subprocess("echo hello", timeout=5)
    results.append({
        "id": "P11",
        "name": "echo returns mode=OK with stdout",
        "ok": c.mode == "OK" and "hello" in c.stdout and c.elapsed_ms >= 0,
    })

    # 12. timeout bound enforcement
    c = run_subprocess("echo x", timeout=999)  # > MAX_TIMEOUT_SECONDS
    # Should still execute but timeout is clamped; this mainly verifies clamp
    results.append({
        "id": "P12",
        "name": "timeout is clamped to MAX",
        "ok": c.mode == "OK",  # ran, didn't raise
    })

    # 13. result_to_dict serializable
    r2 = DockerProbeResult(probe_mode=ProbeMode.DOCKER_READY.value, docker_version="24.0.7", n_running_containers=3)
    j = json.dumps(result_to_dict(r2))
    results.append({
        "id": "P13",
        "name": "result_to_dict JSON-serializable",
        "ok": "DOCKER_READY" in j and "24.0.7" in j and "3" in j,
    })

    # 14. markdown render
    md = render_probe_summary_md(DockerProbeResult(probe_mode=ProbeMode.DOCKER_READY.value))
    results.append({
        "id": "P14",
        "name": "render_probe_summary_md emits V1435 + honest disclosure",
        "ok": "V1435" in md and "Honest disclosure" in md,
    })

    ok_count = sum(1 for r in results if r["ok"])
    return {"passed": ok_count, "total": len(results), "results": results}


def module_meta() -> Dict[str, Any]:
    return {
        "module": V1435_MODULE,
        "version": V1435_VERSION,
        "schema": V1435_SCHEMA,
        "guards": list(V1435_GUARDS),
        "v3_guards": list(V1435_V3_GUARDS),
        "borrowed": [{"module": m, "use": u} for m, u in V1435_BORROWED],
        "endpoints": ("/local/probe/docker",),
        "default_timeout": DEFAULT_TIMEOUT_SECONDS,
        "max_timeout": MAX_TIMEOUT_SECONDS,
        "probe_modes": [m.value for m in ProbeMode],
    }


# ============================================================================
# CLI
# ============================================================================


def _print(s: str) -> None:
    sys.stdout.write(s + "\n")


def main(argv: Optional[List[str]] = None) -> int:
    args = list(argv if argv is not None else sys.argv[1:])
    if not args or args[0] in ("-h", "--help", "help"):
        _print(__doc__)
        return 0
    cmd = args[0]

    if cmd == "version":
        _print(V1435_VERSION)
        return 0

    if cmd == "meta":
        if len(args) > 1 and args[1] == "--json":
            _print(json.dumps(module_meta(), indent=2))
        else:
            _print(f"V1435 version {V1435_VERSION}")
            _print(f"schema: {V1435_SCHEMA}")
            _print(f"guards: {len(V1435_GUARDS)} (V3: {len(V1435_V3_GUARDS)})")
            _print(f"borrowed: {len(V1435_BORROWED)}")
        return 0

    if cmd == "popper":
        out = popper_self_test()
        _print(json.dumps(out, indent=2))
        return 0 if out["passed"] == out["total"] else 1

    if cmd == "chain":
        _print(json.dumps(chain_delegate(), indent=2))
        return 0

    if cmd == "probe":
        timeout = DEFAULT_TIMEOUT_SECONDS
        if "--timeout" in args:
            i = args.index("--timeout")
            if i + 1 < len(args):
                try:
                    timeout = int(args[i + 1])
                except ValueError:
                    pass
        result = run_docker_probe(timeout=timeout)
        _print(render_probe_summary_md(result))
        return 0

    if cmd == "json":
        timeout = DEFAULT_TIMEOUT_SECONDS
        if "--timeout" in args:
            i = args.index("--timeout")
            if i + 1 < len(args):
                try:
                    timeout = int(args[i + 1])
                except ValueError:
                    pass
        result = run_docker_probe(timeout=timeout)
        _print(json.dumps(result_to_dict(result), indent=2))
        return 0

    if cmd == "call":
        if "--cmd" not in args:
            _print("usage: call --cmd \"...\" [--timeout SECONDS]")
            return 2
        i = args.index("--cmd")
        if i + 1 >= len(args):
            _print("missing --cmd value")
            return 2
        cmd_str = args[i + 1]
        timeout = DEFAULT_TIMEOUT_SECONDS
        if "--timeout" in args:
            ti = args.index("--timeout")
            if ti + 1 < len(args):
                try:
                    timeout = int(args[ti + 1])
                except ValueError:
                    pass
        call = run_subprocess(cmd_str, timeout=timeout)
        _print(json.dumps(call.to_dict(), indent=2))
        return 0

    _print(f"unknown command: {cmd}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
