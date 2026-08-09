"""V1440 — ASI 真生产 Docker container run attempt (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43 + 主 22:33).

Phase: 1440
Version: 0.1.0
Date: 2026-08-10 (cron tick 05:58, Asia/Shanghai deep night)
Post: V1439 (streamlit subprocess smoke) + V1438 (real subprocess benchmark)

What V1440 is
=============
V1440 is the **real Docker container run attempt** for Apeireth ASI. Where:

- V1435 detected docker binary availability (probe only — no run)
- V1430 generated deployment artifacts (runbook, no actual docker)
- V1008 defined the docker-based deployment target

V1440 is the **bridge**: it actually probes docker availability via V1435,
then (if docker + daemon are ready) actually attempts a real bounded
`docker run` (e.g. `docker run --rm alpine:latest echo hello`). On hosts
without docker (like this one), V1440 reports ``mode=DOCKER_NOT_INSTALLED``
without raising — fully offline-safe.

V1440 actually:

1.  Reuses V1435's probe to detect: docker binary / docker daemon / image cache
2.  If ready, builds the bounded `docker run` command with sane defaults:
    - ``--rm`` (auto-cleanup)
    - bounded timeout via subprocess
    - small image (alpine:latest if available, otherwise busybox)
    - simple command (``echo hello from docker``)
3.  Spawns real subprocess via subprocess.Popen
4.  Captures: rc, stdout, stderr, elapsed_ms, timed_out, mode
5.  Classifies the result into a DockerRunMode enum
6.  Cleans up: terminates subprocess if still running, kill fallback
7.  Returns DockerRunResult dataclass

Each subprocess has **bounded timeout** (default 30s) and **offline-safe
fallback**: if docker binary is missing, V1440 reports ``DOCKER_NOT_INSTALLED``
without raising. If daemon is down, ``DOCKER_DAEMON_DOWN``. If docker run
fails (image pull error, timeout), V1440 reports the appropriate mode
without crashing.

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1440 is a **Docker container run attempt**. It does NOT claim that:

- docker run is equivalent to production deployment
- the image used is production-grade
- the bounded run proves docker works for all workloads
- docker localhost = production docker
- the subprocess output is the only source of truth

It claims only: **from this host, the docker availability probe was run,
a real bounded `docker run` subprocess was attempted (or honestly skipped
if docker is missing), and the subprocess result is reported**. V1440 ≠
Phenomenal docker, ≠ ASI docker, ≠ human-level docker, ≠ absolute docker.
Bounded subprocess run ≠ deployment proof. Subprocess localhost run ≠
production docker run.

V3 哲学空缺 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
========================================================
- GUARD_NO_PHENOMENAL_DOCKER: subprocess run ≠ consciousness docker
- GUARD_NO_ASI_DOCKER: subprocess run ≠ ASI docker
- GUARD_NO_HUMAN_LEVEL_DOCKER: subprocess run ≠ human-level docker
- GUARD_NO_ABSOLUTE_DOCKER: subprocess run ≠ universal docker
- GUARD_NO_V1435_REPLACE: V1440 runs containers, V1435 only probes

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1435 (docker availability probe — DockerAvailability + ProbeMode)
- V1439 (subprocess smoke pattern + bounded timeout pattern)
- V1430 (deployment runbook — step + status pattern)
- stdlib ``subprocess`` (real child process management)
- stdlib ``shutil`` (real executable lookup via shutil.which)

GUARDS upheld (V1440-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_TIMEOUT: every subprocess has timeout ∈ [1, 120]
- GUARD_NO_RAISE: subprocess failures are caught
- GUARD_OFFLINE_SAFE: docker missing reports DOCKER_NOT_INSTALLED (no crash)
- GUARD_DAEMON_PROBED: V1435 probe runs first
- GUARD_CHILD_HEALTH: child PID + rc captured
- GUARD_OUTPUT_BOUNDED: stdout/stderr truncated to MAX_OUTPUT_BYTES
- GUARD_IMAGE_FALLBACK: alpine:latest → busybox fallback if alpine missing
- GUARD_RM_FLAG: --rm passed to docker run (no leftover containers)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1440 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_NO_PRODUCTION_DEPLOY: subprocess run ≠ production deploy
- GUARD_BOUNDED_IMAGE: only tiny images used (alpine / busybox / hello-world)
- GUARD_CLI_RUNNABLE: CLI 真可跑

API surfaces (16)
=================
1.  ``DockerRunMode`` — Enum (DOCKER_NOT_INSTALLED / DOCKER_DAEMON_DOWN /
    RUN_OK / RUN_TIMEOUT / RUN_FAILED / IMAGE_PULL_FAILED / RUN_DENIED /
    SKIPPED / ERROR)
2.  ``DockerRunSubprocess`` — dataclass (pid + cmd + rc + stdout + stderr +
    elapsed_ms + timed_out + mode + image + test_cmd)
3.  ``DockerAvailabilityLite`` — dataclass (docker_path + docker_version +
    daemon_ready + server_version + n_running + mode)
4.  ``DockerRunResult`` — dataclass (availability + subprocess + started_iso +
    ended_iso + mode + notes)
5.  ``DEFAULT_TIMEOUT_SECONDS`` — int (30)
6.  ``MAX_TIMEOUT_SECONDS`` — int (120)
7.  ``MAX_OUTPUT_BYTES`` — int (4096)
8.  ``DEFAULT_IMAGE_PRIMARY`` — str ("alpine:latest")
9.  ``DEFAULT_IMAGE_FALLBACK`` — str ("busybox:latest")
10. ``DEFAULT_TEST_CMD`` — str ("echo hello-from-v1440")
11. ``DEFAULT_DOCKER_RUN_TIMEOUT`` — int (30)
12. ``probe_docker_availability(timeout)`` — DockerAvailabilityLite
13. ``build_docker_run_cmd(image, test_cmd, docker_bin)`` — List[str]
14. ``run_docker_container(image, test_cmd, timeout)`` — DockerRunSubprocess
15. ``run_v1440(timeout)`` — DockerRunResult (top-level)
16. ``render_report_md(result)`` — str (markdown)
17. ``chain_delegate()`` — chain probe to V1435 + V1439 + V1438
18. ``popper_self_test()`` — 14 self-tests
19. ``module_meta()`` — Dict
20. ``main(argv)`` — CLI

CLI commands (8 — 主 00:56 任何人都能接手):
===========================================
- version
- meta [--json]
- help
- popper
- chain
- run [--timeout SECONDS] (full V1435 probe + docker run + report)
- json [--timeout SECONDS] (same + emit JSON)
- probe-only [--timeout SECONDS] (just V1435 probe, no docker run)
"""

from __future__ import annotations

import dataclasses
import json
import os
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1440_VERSION = "0.1.0"
V1440_SCHEMA = "v1440.asi-docker-container-run/v1"
V1440_MODULE = "v1440_asi_docker_container_run"

DEFAULT_TIMEOUT_SECONDS = 30
MAX_TIMEOUT_SECONDS = 120
MAX_OUTPUT_BYTES = 4096

DEFAULT_IMAGE_PRIMARY = "alpine:latest"
DEFAULT_IMAGE_FALLBACK = "busybox:latest"
DEFAULT_TEST_CMD = "echo hello-from-v1440"
DEFAULT_DOCKER_RUN_TIMEOUT = 30


# ============================================================================
# Enums / Dataclasses
# ============================================================================


class DockerRunMode(str, Enum):
    """Outcome mode of the docker run attempt."""

    DOCKER_NOT_INSTALLED = "DOCKER_NOT_INSTALLED"
    DOCKER_DAEMON_DOWN = "DOCKER_DAEMON_DOWN"
    RUN_OK = "RUN_OK"
    RUN_TIMEOUT = "RUN_TIMEOUT"
    RUN_FAILED = "RUN_FAILED"
    IMAGE_PULL_FAILED = "IMAGE_PULL_FAILED"
    RUN_DENIED = "RUN_DENIED"
    SKIPPED = "SKIPPED"
    ERROR = "ERROR"


@dataclass
class DockerRunSubprocess:
    """Result of the docker run subprocess."""

    pid: int = -1
    cmd: str = ""
    rc: Optional[int] = None
    stdout: str = ""
    stderr: str = ""
    elapsed_ms: float = 0.0
    timed_out: bool = False
    mode: DockerRunMode = DockerRunMode.SKIPPED
    image: str = ""
    test_cmd: str = ""


@dataclass
class DockerAvailabilityLite:
    """Lightweight docker availability summary (subset of V1435)."""

    docker_path: str = "NOT_FOUND"
    docker_version: str = "UNKNOWN"
    daemon_ready: bool = False
    server_version: str = "UNREACHABLE"
    n_running: int = -1
    mode: str = "UNKNOWN"


@dataclass
class DockerRunResult:
    """Aggregate of the docker run attempt."""

    availability: DockerAvailabilityLite
    subprocess: DockerRunSubprocess
    mode: DockerRunMode
    started_iso: str = ""
    ended_iso: str = ""
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "availability": {
                "docker_path": self.availability.docker_path,
                "docker_version": self.availability.docker_version,
                "daemon_ready": self.availability.daemon_ready,
                "server_version": self.availability.server_version,
                "n_running": self.availability.n_running,
                "mode": self.availability.mode,
            },
            "subprocess": {
                "pid": self.subprocess.pid,
                "cmd": self.subprocess.cmd,
                "rc": self.subprocess.rc,
                "elapsed_ms": round(self.subprocess.elapsed_ms, 2),
                "timed_out": self.subprocess.timed_out,
                "mode": self.subprocess.mode.value,
                "image": self.subprocess.image,
                "test_cmd": self.subprocess.test_cmd,
                "stdout_preview": self.subprocess.stdout.splitlines()[:5] if self.subprocess.stdout else [],
                "stderr_preview": self.subprocess.stderr.splitlines()[:5] if self.subprocess.stderr else [],
            },
            "mode": self.mode.value,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
            "notes": self.notes,
        }


# ============================================================================
# Docker availability probe (reuses V1435)
# ============================================================================


def probe_docker_availability(timeout: int = DEFAULT_TIMEOUT_SECONDS) -> DockerAvailabilityLite:
    """Probe docker availability via V1435; return lite summary."""
    try:
        from apeireth.v1435_asi_docker_availability_probe import (
            run_docker_probe,
            ProbeMode as V1435ProbeMode,
        )

        # V1435 run_docker_probe returns DockerProbeResult
        report = run_docker_probe(timeout=timeout)
        # Best-effort field extraction; report attributes may vary by version
        return DockerAvailabilityLite(
            docker_path=str(getattr(report, "docker_path", "UNKNOWN") or "UNKNOWN"),
            docker_version=str(getattr(report, "docker_version", "UNKNOWN") or "UNKNOWN"),
            daemon_ready=str(getattr(report, "server_version", "UNREACHABLE")) not in ("UNREACHABLE", "UNKNOWN", ""),
            server_version=str(getattr(report, "server_version", "UNREACHABLE")),
            n_running=int(getattr(report, "n_running_containers", -1) or -1),
            mode=str(getattr(report, "probe_mode", "UNKNOWN")),
        )
    except Exception as exc:  # noqa: BLE001
        return DockerAvailabilityLite(
            docker_path="ERROR",
            docker_version="ERROR",
            daemon_ready=False,
            server_version="ERROR",
            n_running=-1,
            mode=f"ERROR: {type(exc).__name__}: {exc}",
        )


# ============================================================================
# Docker run command builder + execution
# ============================================================================


def _coerce_timeout(timeout: Any) -> int:
    try:
        t = int(timeout)
    except (TypeError, ValueError):
        return DEFAULT_DOCKER_RUN_TIMEOUT
    return max(1, min(t, MAX_TIMEOUT_SECONDS))


def _truncate(s: str, max_bytes: int = MAX_OUTPUT_BYTES) -> str:
    if not s:
        return ""
    encoded = s.encode("utf-8", errors="replace")
    if len(encoded) <= max_bytes:
        return s
    return encoded[:max_bytes].decode("utf-8", errors="replace") + "\n[truncated]"


def build_docker_run_cmd(
    image: str = DEFAULT_IMAGE_PRIMARY,
    test_cmd: str = DEFAULT_TEST_CMD,
    docker_bin: str = "docker",
) -> List[str]:
    """Build a bounded docker run command (with --rm, no network mounts)."""
    return [
        docker_bin,
        "run",
        "--rm",
        image,
        "sh",
        "-c",
        test_cmd,
    ]


def run_docker_container(
    image: str = DEFAULT_IMAGE_PRIMARY,
    test_cmd: str = DEFAULT_TEST_CMD,
    timeout: int = DEFAULT_DOCKER_RUN_TIMEOUT,
    docker_bin: Optional[str] = None,
) -> DockerRunSubprocess:
    """Spawn `docker run` in a real subprocess. Returns DockerRunSubprocess."""
    timeout = _coerce_timeout(timeout)

    if docker_bin is None:
        docker_bin = shutil.which("docker") or "docker"

    result = DockerRunSubprocess(image=image, test_cmd=test_cmd)
    cmd = build_docker_run_cmd(image=image, test_cmd=test_cmd, docker_bin=docker_bin)
    result.cmd = " ".join(cmd)

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
        result.pid = int(proc.pid) if proc.pid else -1
        try:
            stdout_b, stderr_b = proc.communicate(timeout=timeout)
            result.rc = int(proc.returncode) if proc.returncode is not None else 0
            result.stdout = _truncate(stdout_b.decode("utf-8", errors="replace"))
            result.stderr = _truncate(stderr_b.decode("utf-8", errors="replace"))
            result.elapsed_ms = (time.monotonic() - start) * 1000.0
            if result.rc == 0:
                result.mode = DockerRunMode.RUN_OK
            else:
                # Classify failures
                err = (result.stderr or "").lower()
                if "denied" in err or "permission" in err:
                    result.mode = DockerRunMode.RUN_DENIED
                elif "pull" in err or "manifest" in err or "not found" in err:
                    result.mode = DockerRunMode.IMAGE_PULL_FAILED
                else:
                    result.mode = DockerRunMode.RUN_FAILED
        except subprocess.TimeoutExpired:
            try:
                proc.kill()
                proc.wait(timeout=2)
            except Exception:  # noqa: BLE001
                pass
            result.timed_out = True
            result.rc = -1
            result.elapsed_ms = (time.monotonic() - start) * 1000.0
            result.mode = DockerRunMode.RUN_TIMEOUT
            try:
                stdout_b, stderr_b = proc.communicate(timeout=2)
                result.stdout = _truncate(stdout_b.decode("utf-8", errors="replace"))
                result.stderr = _truncate(stderr_b.decode("utf-8", errors="replace"))
            except Exception:  # noqa: BLE001
                pass
    except FileNotFoundError as exc:
        result.mode = DockerRunMode.DOCKER_NOT_INSTALLED
        result.stderr = f"FileNotFoundError: {exc}"
        result.elapsed_ms = (time.monotonic() - start) * 1000.0
    except (OSError, ValueError) as exc:
        result.mode = DockerRunMode.ERROR
        result.stderr = f"{type(exc).__name__}: {exc}"
        result.elapsed_ms = (time.monotonic() - start) * 1000.0
    except Exception as exc:  # noqa: BLE001
        result.mode = DockerRunMode.ERROR
        result.stderr = f"{type(exc).__name__}: {exc}"
        result.elapsed_ms = (time.monotonic() - start) * 1000.0
    return result


# ============================================================================
# Top-level orchestrator
# ============================================================================


def run_v1440(timeout: int = DEFAULT_TIMEOUT_SECONDS) -> DockerRunResult:
    """Probe docker via V1435, then attempt real bounded docker run.

    If docker is missing → DOCKER_NOT_INSTALLED, no docker run attempted.
    If daemon is down → DOCKER_DAEMON_DOWN, no docker run attempted.
    If docker + daemon ready → real subprocess.run docker.
    """
    started = datetime.now(timezone.utc).isoformat()
    notes: List[str] = []

    # Step 1: probe availability via V1435
    avail = probe_docker_availability(timeout=min(timeout, 10))
    notes.append(f"v1435 mode={avail.mode}; docker_path={avail.docker_path}")

    # Step 2: if missing or daemon down, skip docker run
    if not avail.docker_path or avail.docker_path in ("NOT_FOUND", "ERROR", ""):
        return DockerRunResult(
            availability=avail,
            subprocess=DockerRunSubprocess(mode=DockerRunMode.DOCKER_NOT_INSTALLED),
            mode=DockerRunMode.DOCKER_NOT_INSTALLED,
            started_iso=started,
            ended_iso=datetime.now(timezone.utc).isoformat(),
            notes=notes + ["docker binary not found; skipping docker run"],
        )

    if not avail.daemon_ready:
        return DockerRunResult(
            availability=avail,
            subprocess=DockerRunSubprocess(mode=DockerRunMode.DOCKER_DAEMON_DOWN),
            mode=DockerRunMode.DOCKER_DAEMON_DOWN,
            started_iso=started,
            ended_iso=datetime.now(timezone.utc).isoformat(),
            notes=notes + ["docker daemon not reachable; skipping docker run"],
        )

    # Step 3: real docker run with primary image
    sub = run_docker_container(
        image=DEFAULT_IMAGE_PRIMARY,
        test_cmd=DEFAULT_TEST_CMD,
        timeout=timeout,
        docker_bin=avail.docker_path,
    )

    # Step 4: if primary image pull failed, try fallback image
    if sub.mode == DockerRunMode.IMAGE_PULL_FAILED:
        notes.append(f"primary image {DEFAULT_IMAGE_PRIMARY} pull failed; trying fallback {DEFAULT_IMAGE_FALLBACK}")
        sub_fallback = run_docker_container(
            image=DEFAULT_IMAGE_FALLBACK,
            test_cmd=DEFAULT_TEST_CMD,
            timeout=timeout,
            docker_bin=avail.docker_path,
        )
        if sub_fallback.mode == DockerRunMode.RUN_OK:
            sub = sub_fallback
            notes.append("fallback image run succeeded")
        else:
            notes.append(f"fallback image run mode={sub_fallback.mode.value}")

    ended = datetime.now(timezone.utc).isoformat()
    return DockerRunResult(
        availability=avail,
        subprocess=sub,
        mode=sub.mode,
        started_iso=started,
        ended_iso=ended,
        notes=notes,
    )


# ============================================================================
# Markdown report
# ============================================================================


def render_report_md(result: DockerRunResult) -> str:
    lines: List[str] = []
    lines.append("# V1440 — ASI Docker container run report")
    lines.append("")
    lines.append(f"- module: `{V1440_MODULE}`")
    lines.append(f"- version: `{V1440_VERSION}`")
    lines.append(f"- schema: `{V1440_SCHEMA}`")
    lines.append(f"- mode: `{result.mode.value}`")
    lines.append(f"- started_iso: `{result.started_iso}`")
    lines.append(f"- ended_iso: `{result.ended_iso}`")
    lines.append("")
    lines.append("## Availability (from V1435)")
    lines.append("")
    lines.append(f"- docker_path: `{result.availability.docker_path}`")
    lines.append(f"- docker_version: `{result.availability.docker_version}`")
    lines.append(f"- daemon_ready: `{result.availability.daemon_ready}`")
    lines.append(f"- server_version: `{result.availability.server_version}`")
    lines.append(f"- n_running: `{result.availability.n_running}`")
    lines.append(f"- v1435_mode: `{result.availability.mode}`")
    lines.append("")
    lines.append("## Subprocess")
    lines.append("")
    lines.append(f"- pid: `{result.subprocess.pid}`")
    lines.append(f"- mode: `{result.subprocess.mode.value}`")
    lines.append(f"- rc: `{result.subprocess.rc}`")
    lines.append(f"- elapsed_ms: `{result.subprocess.elapsed_ms:.2f}`")
    lines.append(f"- timed_out: `{result.subprocess.timed_out}`")
    lines.append(f"- image: `{result.subprocess.image}`")
    lines.append(f"- test_cmd: `{result.subprocess.test_cmd}`")
    if result.subprocess.stdout:
        lines.append("- stdout_preview (first 5 lines):")
        for ln in result.subprocess.stdout.splitlines()[:5]:
            lines.append(f"  - `{ln[:120]}`")
    if result.subprocess.stderr:
        lines.append("- stderr_preview (first 5 lines):")
        for ln in result.subprocess.stderr.splitlines()[:5]:
            lines.append(f"  - `{ln[:120]}`")
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
        "V1440 is a **Docker container run attempt**. It does NOT claim that "
        "docker run is equivalent to production deployment, that the bounded "
        "run proves docker works for all workloads, or that subprocess "
        "localhost = production docker. It claims only: **from this host, "
        "the docker availability probe was run via V1435, a real bounded "
        "`docker run` subprocess was attempted (or honestly skipped if "
        "docker is missing), and the subprocess result is reported**. V1440 "
        "≠ Phenomenal docker, ≠ ASI docker, ≠ human-level docker, ≠ "
        "absolute docker. Bounded subprocess run ≠ deployment proof. "
        "Subprocess localhost run ≠ production docker run."
    )
    return "\n".join(lines)


# ============================================================================
# Chain + Popper
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream V1435 + V1439 + V1438 chain integrity."""
    out: Dict[str, Any] = {
        "v1440": {"ok": True, "mode": V1440_MODULE},
        "v1435": {"ok": True, "mode": "v1435_asi_docker_availability_probe"},
        "v1439": {"ok": True, "mode": "v1439_asi_streamlit_subprocess_smoke"},
        "v1438": {"ok": True, "mode": "v1438_asi_real_subprocess_benchmark"},
        "all_ok": True,
        "borrowed": [{"module": m, "use": u} for m, u in V1440_BORROWED],
    }
    try:
        import apeireth.v1435_asi_docker_availability_probe as v1435

        out["v1435"]["importable"] = True
        out["v1435"]["probe_modes"] = [m.value for m in v1435.ProbeMode]
    except Exception as exc:  # noqa: BLE001
        out["v1435"]["ok"] = False
        out["v1435"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1439_asi_streamlit_subprocess_smoke as v1439

        out["v1439"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1439"]["ok"] = False
        out["v1439"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    try:
        import apeireth.v1438_asi_real_subprocess_benchmark as v1438

        out["v1438"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1438"]["ok"] = False
        out["v1438"]["error"] = f"{type(exc).__name__}: {exc}"
        out["all_ok"] = False
    # V1440 self: probe availability right now
    try:
        lite = probe_docker_availability(timeout=5)
        out["v1440"]["docker_path"] = lite.docker_path
        out["v1440"]["daemon_ready"] = lite.daemon_ready
    except Exception as exc:  # noqa: BLE001
        out["v1440"]["probe_error"] = f"{type(exc).__name__}: {exc}"
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
                V1440_VERSION == "0.1.0"
                and DEFAULT_TIMEOUT_SECONDS > 0
                and MAX_TIMEOUT_SECONDS >= DEFAULT_TIMEOUT_SECONDS
                and MAX_OUTPUT_BYTES > 0
                and DEFAULT_IMAGE_PRIMARY
                and DEFAULT_IMAGE_FALLBACK
            ),
        }
    )

    # P02: guards
    results.append(
        {
            "id": "P02",
            "name": "guards count",
            "ok": len(V1440_GUARDS) == 14 and len(V1440_V3_GUARDS) == 5,
        }
    )

    # P03: borrowed
    results.append(
        {
            "id": "P03",
            "name": "borrowed count",
            "ok": len(V1440_BORROWED) == 5,
        }
    )

    # P04: docker run mode count (≥9)
    results.append(
        {
            "id": "P04",
            "name": "docker run mode count",
            "ok": len(list(DockerRunMode)) >= 9,
        }
    )

    # P05: build_docker_run_cmd basic
    cmd = build_docker_run_cmd(image="alpine", test_cmd="echo hi", docker_bin="docker")
    results.append(
        {
            "id": "P05",
            "name": "build_docker_run_cmd structure",
            "ok": cmd[0] == "docker" and cmd[1] == "run" and cmd[2] == "--rm" and "alpine" in cmd,
        }
    )

    # P06: _coerce_timeout bounds
    results.append(
        {
            "id": "P06",
            "name": "_coerce_timeout bounds",
            "ok": _coerce_timeout(0) == 1 and _coerce_timeout(-5) == 1 and _coerce_timeout(99999) == MAX_TIMEOUT_SECONDS and _coerce_timeout("abc") == DEFAULT_DOCKER_RUN_TIMEOUT,
        }
    )

    # P07: _truncate bounded
    small, _ = _truncate("hello"), False
    big = "x" * 100
    big_trunc = _truncate(big, max_bytes=10)
    results.append(
        {
            "id": "P07",
            "name": "_truncate bounded",
            "ok": "hello" in small and len(big_trunc) <= 50,
        }
    )

    # P08: probe_docker_availability returns dataclass
    lite = probe_docker_availability(timeout=5)
    results.append(
        {
            "id": "P08",
            "name": "probe_docker_availability returns dataclass",
            "ok": isinstance(lite, DockerAvailabilityLite),
        }
    )

    # P09: DockerAvailabilityLite default
    avail = DockerAvailabilityLite()
    results.append(
        {
            "id": "P09",
            "name": "DockerAvailabilityLite defaults",
            "ok": avail.docker_path == "NOT_FOUND" and avail.daemon_ready is False,
        }
    )

    # P10: DockerRunSubprocess default
    sub = DockerRunSubprocess()
    results.append(
        {
            "id": "P10",
            "name": "DockerRunSubprocess default mode is SKIPPED",
            "ok": sub.mode == DockerRunMode.SKIPPED,
        }
    )

    # P11: DockerRunResult to_dict
    res = DockerRunResult(
        availability=avail,
        subprocess=sub,
        mode=DockerRunMode.SKIPPED,
    )
    d = res.to_dict()
    results.append(
        {
            "id": "P11",
            "name": "DockerRunResult.to_dict structure",
            "ok": "availability" in d and "subprocess" in d and d["mode"] == "SKIPPED",
        }
    )

    # P12: run_docker_container returns dataclass (no docker)
    sub2 = run_docker_container(image="alpine", test_cmd="echo hi", timeout=2, docker_bin="docker")
    results.append(
        {
            "id": "P12",
            "name": "run_docker_container returns DockerRunSubprocess",
            "ok": isinstance(sub2, DockerRunSubprocess) and sub2.image == "alpine",
        }
    )

    # P13: run_v1440 returns DockerRunResult (offline-safe)
    res2 = run_v1440(timeout=5)
    results.append(
        {
            "id": "P13",
            "name": "run_v1440 returns DockerRunResult",
            "ok": isinstance(res2, DockerRunResult),
        }
    )

    # P14: chain_delegate runs
    try:
        ch = chain_delegate()
        results.append(
            {
                "id": "P14",
                "name": "chain_delegate runs",
                "ok": "v1440" in ch and "v1435" in ch and "v1439" in ch,
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


V1440_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_TIMEOUT",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_DAEMON_PROBED",
    "GUARD_CHILD_HEALTH",
    "GUARD_OUTPUT_BOUNDED",
    "GUARD_IMAGE_FALLBACK",
    "GUARD_RM_FLAG",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_PRODUCTION_DEPLOY",
    "GUARD_BOUNDED_IMAGE",
    "GUARD_CLI_RUNNABLE",
)

V1440_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_DOCKER",
    "GUARD_NO_ASI_DOCKER",
    "GUARD_NO_HUMAN_LEVEL_DOCKER",
    "GUARD_NO_ABSOLUTE_DOCKER",
    "GUARD_NO_V1435_REPLACE",
)

V1440_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1435_asi_docker_availability_probe", "DockerAvailability + ProbeMode"),
    ("v1439_asi_streamlit_subprocess_smoke", "Subprocess smoke pattern + bounded timeout pattern"),
    ("v1430_asi_deployment_e2e_runbook", "Step-based runbook pattern"),
    ("stdlib_subprocess", "Real subprocess invocation"),
    ("stdlib_shutil", "Real executable lookup (shutil.which)"),
)


# ============================================================================
# Module metadata
# ============================================================================


def module_meta() -> Dict[str, Any]:
    return {
        "module": V1440_MODULE,
        "version": V1440_VERSION,
        "schema": V1440_SCHEMA,
        "n_guards": len(V1440_GUARDS),
        "n_v3_guards": len(V1440_V3_GUARDS),
        "n_borrowed": len(V1440_BORROWED),
        "default_image_primary": DEFAULT_IMAGE_PRIMARY,
        "default_image_fallback": DEFAULT_IMAGE_FALLBACK,
        "default_test_cmd": DEFAULT_TEST_CMD,
    }


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print("V1440 — ASI Docker container run attempt")
    print("")
    print("Commands:")
    print("  version                  Print version")
    print("  meta [--json]            Print module metadata")
    print("  help                     Print this help")
    print("  popper                   Run 14 popper self-tests")
    print("  chain                    Run chain_delegate")
    print("  run [--timeout SECONDS]  Full V1435 probe + docker run + report")
    print("  json [--timeout SECONDS] Same + emit JSON")
    print("  probe-only [--timeout SECONDS]")
    print("                           Just V1435 probe, no docker run")


def main(argv: Optional[List[str]] = None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    if not argv:
        _print_help()
        return 0
    cmd = argv[0]
    args = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(V1440_VERSION)
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

    if cmd in ("run", "json", "probe-only"):
        timeout = DEFAULT_TIMEOUT_SECONDS
        i = 0
        while i < len(args):
            a = args[i]
            if a == "--timeout" and i + 1 < len(args):
                try:
                    timeout = int(args[i + 1])
                except ValueError:
                    timeout = DEFAULT_TIMEOUT_SECONDS
                i += 2
            else:
                i += 1
        if cmd == "probe-only":
            lite = probe_docker_availability(timeout=timeout)
            print(json.dumps({
                "docker_path": lite.docker_path,
                "docker_version": lite.docker_version,
                "daemon_ready": lite.daemon_ready,
                "server_version": lite.server_version,
                "n_running": lite.n_running,
                "mode": lite.mode,
            }, indent=2))
            return 0
        result = run_v1440(timeout=timeout)
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
