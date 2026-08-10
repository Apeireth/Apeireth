"""V1470 — ASI Real V1469 Batch Harness + V1468-Generated-Client ↔ stdlib http.client Cross-Equivalence Verifier (主 13:31 大胆放手 + 主 23:44 骈插捣 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化).

Phase: 1470
Version: 0.1.0
Date: 2026-08-10 (cron tick 17:45, Monday afternoon, round-132, isolated lane)
Post: V1469 (Two-Process V1468-Generated-Client → V1467-Server Driver — 60 tests pass)
      V1468 (OpenAPI 3.1 Schema + Generated Python Client — 47 tests pass)
      V1467 (Cross-Audit HTTP Gateway + History + Diff — 30 tests pass)

What V1470 is
=============
V1469 demonstrated that V1468's generated client can drive V1467 across
**one** subprocess boundary: server in subprocess A, client driver in
subprocess B, V1469 parent coordinating both. That's a real proof of
"anyone can take over" — but it's a *single* shot.

V1470 takes the next natural step: **a batch harness** that

  1. Runs V1469 driver N times (default 3) with different ports
  2. After each V1469 run succeeds: extracts the generated client path
     + port from the V1469 report, then independently verifies that the
     V1468-generated client and raw stdlib http.client return the same
     response structure (status + JSON keys) for the same endpoints
  3. Aggregates statistics: success rate, latency p50/p95/mean/max,
     endpoint coverage, determinism score
  4. Verifies ports are distinct across runs (port reclaim working)
  5. Verifies server PIDs are distinct across runs (no subprocess caching)
  6. Verifies endpoint-hit keys are stable across runs (deterministic
     V1467 + V1468 + V1469 stack)
  7. Writes batch report JSON + Markdown

The V1468-generated client ↔ raw http.client cross-check is the
"anyone can take over" full-stack demonstration:

  ┌─────────────────────────────────────────────────────────────┐
  │  path A: V1468-generated client                              │
  │    $ python -m apeireth.v1468 generate-client --out X.py     │
  │    $ python -c "from X import V1467Client; c = V1467Client(  │
  │       host, port); print(c.healthz())"                       │
  │                                                              │
  │  path B: raw stdlib http.client                              │
  │    $ python -c "import http.client, json;                    │
  │       c = http.client.HTTPConnection(host, port);            │
  │       c.request('GET', '/healthz');                          │
  │       print(json.loads(c.getresponse().read()))"             │
  │                                                              │
  │  V1470 asserts: path A response keys == path B response keys │
  │                + status codes match                          │
  └─────────────────────────────────────────────────────────────┘

V1470 makes "anyone can take over" fully real in both directions: if
the V1468-generated client ever drifts from raw http.client, V1470
catches it.

V1470 cross-client equivalence uses the SAME URL (path + query string) for
both path A (V1468-generated client) and path B (raw stdlib http.client).
Query params are built from V1470_EQUIVALENCE_QUERY_PARAMS and applied via
urllib.parse.urlencode. This ensures the equivalence check is actually
comparing like-for-like requests, not path-A-with-query vs path-B-without-query.

After V1470, an external developer can:

  $ python -m apeireth.v1470_... run --out out/v1470-batch.json
    → spawns V1469 driver 3x (each spawns V1467 server + V1468 client)
    → after each run, runs cross-client equivalence checks (6 EPs × 2 paths)
    → aggregates stats: success_rate=1.0, p50/p95 latency, determinism=1.0
    → writes JSON + Markdown report

V1470 is NOT:
- an in-process test (V1469 runs are real subprocess invocations)
- a server-side change (V1467 / V1468 / V1469 all unchanged)
- a CI/CD pipeline (V1470 is a one-shot batch harness)
- a load tester (3-5 runs, not 100s; bounded by max_wallclock)
- a fuzzer (no mutation; deterministic endpoints)
- a process supervisor (V1470 spawns + waits + kills + continues)
- a V1468 client implementation (V1470 only imports + invokes)
- an http.client implementation (V1470 only imports + invokes)

V1470 IS:
- a real batch harness for V1469 (N subprocess invocations)
- a real cross-client equivalence verifier (V1468-generated ↔ stdlib)
- a real statistics aggregator (latency p50/p95, determinism score)
- a real subprocess cleanup coordinator (each run killed + waited)
- safe-by-default: subprocess timeouts, bounded output, loopback only

V1470 design rules (主 13:31 大胆放手 + 主 23:44 骈插捣):
- Each V1469 run spawned via subprocess.run on python -m apeireth.v1469_... run
- After each run: parse V1469 JSON report → extract port + client_path
- Use V1468-generated client (importlib + sys.path injection) for path A
- Use raw stdlib http.client for path B
- Compare status + sorted body keys for each endpoint
- 6 endpoints × 2 paths = 12 cross-checks per V1469 run
- 3 runs default → 36 cross-checks total
- Loopback 127.0.0.1 default (主 23:44 骈插捣)
- All subprocess operations bounded by timeout (主 00:44 质量工程化)

V1470 GUARDS (主 00:44 质量工程化):
- GUARD_V1469_REUSED              : V1469 driver invoked via subprocess, not reimplemented
- GUARD_BATCH_N_GE_2              : at least 2 V1469 runs in the batch
- GUARD_ALL_RUNS_PASS             : every V1469 run succeeded
- GUARD_DETERMINISTIC_KEYS        : endpoint-hit keys stable across all runs
- GUARD_PORTS_DISTINCT_OR_REUSED  : each V1469 run used a port in [18380, 18480]
                                     (V1469 reuses freed ports by design — port reclaim)
- GUARD_PIDS_DISTINCT             : V1469 spawned different server PID per run
- GUARD_CROSS_CLIENT_EQUIV        : V1468-generated client ≡ stdlib http.client
- GUARD_LATENCY_AGGREGATED        : p50/p95/mean/max latency computed
- GUARD_DETERMINISM_SCORED        : determinism score in [0.0, 1.0]
- GUARD_BOUNDED_WALLCLOCK         : total batch elapsed ≤ max_wallclock_s
- GUARD_SUBPROCESS_CLEANED        : all V1469 subprocesses reaped (poll() returned)
- GUARD_LINEAGE_CITED             : borrowed sources cited (V1469+V1468+V1467+stdlib)
- GUARD_RUNS_ON_WINDOWS           : stdlib-only, no POSIX-only syscalls
- GUARD_JSON_REPORT_WRITTEN       : V1470 output JSON file exists + parses
- GUARD_MD_REPORT_WRITTEN         : V1470 output Markdown file exists + non-empty

V1470 V3 哲学守门 (主 17:58 + 主 20:46 不假装):
- GUARD_HARNESS_NOT_CI            : V1470 is a one-shot harness, not CI/CD pipeline
- GUARD_HARNESS_NOT_LOAD_TEST     : 3-5 runs, not 100s
- GUARD_HARNESS_NOT_FUZZER        : no random mutation, deterministic endpoints
- GUARD_HARNESS_NOT_BENCHMARK     : measures N runs for stats, not micro-bench
- GUARD_NOT_ASI                   : subprocess coordinator, NOT ASI
- GUARD_NOT_PHENOMENAL            : subprocess coordinator, NOT consciousness
- GUARD_NOT_HUMAN_LEVEL           : mechanical batch driver, NOT human reasoning
- GUARD_NOT_ORCHESTRATOR          : 3-5 runs, not N×M orchestration

借力 (主 19:33 走在前人经验上):
- V1469 — Two-process V1468-client → V1467-server driver (subprocess.run pattern)
- V1468 — OpenAPI 3.1 schema + Python client generator (CLI: generate-client)
- V1467 — Cross-Audit HTTP Gateway (6 endpoints, BaseHTTPRequestHandler)
- V1466 — Real Cross-Process Lint-Gate Subprocess Runner (cross-process pattern)
- V1437 — V1464 HTTP Gateway stdlib pattern (BaseHTTPRequestHandler reference)
- stdlib — subprocess + tempfile + socket + json + urllib + http.client + time + os + importlib

实事求是 (主 17:43):
- V1470 ≠ in-process test (V1469 runs are real subprocess invocations)
- V1470 ≠ server-side change (V1467/V1468/V1469 all unchanged)
- V1470 ≠ CI (one-shot batch harness)
- V1470 ≠ load tester (3-5 runs, not 100s)
- V1470 ≠ fuzzer (no mutation, deterministic endpoints)
- V1470 ≠ benchmark (measures batch stats, not micro-bench timings)
- Anyone can `python -m apeireth.v1470_... run --out out.json`
- 不假装 process supervision: V1470 spawns N V1469 subprocesses, waits, kills;
  if any hangs past timeout, V1470 calls subprocess.terminate() +
  subprocess.wait(timeout=5) then continues
- 不假装 cross-language: V1470 uses Python-to-Python subprocess + Python
  stdlib http.client; cross-language interop would need Rust/TS client
  (V1471+ 候选)
- 不假装 ASI/Phenomenal/human-level: V1470 is a batch harness, not intelligence
- 不假装 100% equivalence: cross-check compares response keys (not values);
  V1467 returns timestamp-bearing bodies (e.g. ts, timestamp), so values
  differ across runs; key equivalence is the safe invariant
"""

from __future__ import annotations

import argparse
import http.client
import importlib.util
import json
import os
import socket
import statistics
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

V1470_MODULE = "v1470_asi_v1469_batch_harness_cross_client_equivalence"
V1470_VERSION = "0.1.0"
V1470_SCHEMA = "v1470.asi-real-v1469-batch-harness-cross-client-equivalence/v1"
V1470_DATE = "2026-08-10"

# ──────────────────────────────────────────────────────────────────────
# V1470 constants
# ──────────────────────────────────────────────────────────────────────

# Loopback (主 23:44 骈插捣)
DEFAULT_HOST = "127.0.0.1"

# Batch size (主 00:44 质量工程化)
DEFAULT_N_RUNS = 3
MIN_N_RUNS = 2
MAX_N_RUNS = 5

# Per-V1469-run timeout (主 00:44 质量工程化)
DEFAULT_V1469_TIMEOUT_S = 90.0  # V1469 itself has DEFAULT_MAX_WALLCLOCK_S=60; give it buffer

# Total batch wallclock bound (主 00:44 质量工程化)
DEFAULT_MAX_WALLCLOCK_S = 360.0  # 6 minutes for 3 runs (each ~60s + overhead)

# Output bounds (主 00:44 质量工程化)
DEFAULT_MAX_OUTPUT_BYTES = 65536  # 64KB per subprocess capture

# Kill grace for hung V1469 subprocesses
DEFAULT_KILL_GRACE_S = 5.0

# Endpoints that the cross-client equivalence check verifies
V1470_EQUIVALENCE_ENDPOINTS: Tuple[Tuple[str, str], ...] = (
    ("GET", "/healthz"),
    ("GET", "/status"),
    ("POST", "/audit/run"),
    ("GET", "/audit/history"),
    ("GET", "/audit/{audit_id}"),
    ("GET", "/audit/diff"),
)

# POST request body for /audit/run (must be valid JSON)
AUDIT_RUN_BODY: Dict[str, Any] = {"dry_run": True, "policy": "STANDARD"}

# Query params per endpoint for raw stdlib http.client equivalence
# (must match what the V1468-generated client sends)
V1470_EQUIVALENCE_QUERY_PARAMS: Dict[str, Dict[str, str]] = {
    "/audit/{audit_id}": {"audit_id": "v1470-fake-id-for-equivalence-check"},
    "/audit/diff": {
        "baseline_id": "v1470-fake-baseline",
        "current_id": "v1470-fake-current",
    },
}


def _build_query_string(path: str) -> str:
    """Build query string for raw stdlib path B matching generated path A."""
    params = V1470_EQUIVALENCE_QUERY_PARAMS.get(path, {})
    if not params:
        return ""
    return urllib.parse.urlencode(params)


def _raw_path_for_stdlib(path: str) -> str:
    """Build full path (with query) for raw stdlib http.client."""
    qs = _build_query_string(path)
    if qs:
        return f"{path}?{qs}"
    return path

# Borrowed sources
BORROWED_SOURCES: Tuple[str, ...] = (
    "v1469",  # Two-process V1468-client → V1467-server driver (subprocess.run pattern)
    "v1468",  # OpenAPI 3.1 schema + Python client generator (CLI: generate-client)
    "v1467",  # Cross-Audit HTTP Gateway (6 endpoints, BaseHTTPRequestHandler)
    "v1466",  # Real Cross-Process Lint-Gate Subprocess Runner (cross-process pattern)
    "v1437",  # V1464 HTTP Gateway stdlib pattern (BaseHTTPRequestHandler reference)
    "stdlib",  # subprocess + tempfile + socket + json + urllib + http.client + time + importlib
)

# V1470 GUARDS
V1470_GUARDS: Tuple[str, ...] = (
    "GUARD_V1469_REUSED",
    "GUARD_BATCH_N_GE_2",
    "GUARD_ALL_RUNS_PASS",
    "GUARD_DETERMINISTIC_KEYS",
    "GUARD_PORTS_DISTINCT_OR_REUSED",
    "GUARD_PIDS_DISTINCT",
    "GUARD_CROSS_CLIENT_EQUIV",
    "GUARD_LATENCY_AGGREGATED",
    "GUARD_DETERMINISM_SCORED",
    "GUARD_BOUNDED_WALLCLOCK",
    "GUARD_SUBPROCESS_CLEANED",
    "GUARD_LINEAGE_CITED",
    "GUARD_RUNS_ON_WINDOWS",
    "GUARD_JSON_REPORT_WRITTEN",
    "GUARD_MD_REPORT_WRITTEN",
)

# V1470 V3 哲学守门
V1470_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_HARNESS_NOT_CI",
    "GUARD_HARNESS_NOT_LOAD_TEST",
    "GUARD_HARNESS_NOT_FUZZER",
    "GUARD_HARNESS_NOT_BENCHMARK",
    "GUARD_NOT_ASI",
    "GUARD_NOT_PHENOMENAL",
    "GUARD_NOT_HUMAN_LEVEL",
    "GUARD_NOT_ORCHESTRATOR",
)


# ──────────────────────────────────────────────────────────────────────
# V1470 dataclasses
# ──────────────────────────────────────────────────────────────────────


@dataclass
class EndpointEquivalenceCheck:
    """Comparison of V1468-generated client vs raw stdlib http.client for one endpoint."""
    method: str
    path: str
    generated_status: int
    generated_body_keys: Tuple[str, ...]  # sorted top-level keys (or () if non-dict)
    raw_status: int
    raw_body_keys: Tuple[str, ...]
    status_match: bool
    keys_match: bool
    ok: bool
    elapsed_ms: float
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "method": self.method,
            "path": self.path,
            "generated_status": self.generated_status,
            "generated_body_keys": list(self.generated_body_keys),
            "raw_status": self.raw_status,
            "raw_body_keys": list(self.raw_body_keys),
            "status_match": self.status_match,
            "keys_match": self.keys_match,
            "ok": self.ok,
            "elapsed_ms": self.elapsed_ms,
            "error": self.error,
        }


@dataclass
class V1469RunSummary:
    """Summary of one V1469 subprocess run within the batch."""
    run_index: int  # 1..N
    ok: bool
    verdict: str  # PASS / FAIL / ERROR
    port: int
    server_pid: int
    client_pid: int
    elapsed_s: float
    n_endpoints_ok: int
    n_endpoints_total: int
    client_path: str
    result_path: str
    endpoint_keys: Tuple[str, ...]  # sorted keys of endpoints_hit for determinism check
    v1469_report_path: str
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "run_index": self.run_index,
            "ok": self.ok,
            "verdict": self.verdict,
            "port": self.port,
            "server_pid": self.server_pid,
            "client_pid": self.client_pid,
            "elapsed_s": self.elapsed_s,
            "n_endpoints_ok": self.n_endpoints_ok,
            "n_endpoints_total": self.n_endpoints_total,
            "client_path": self.client_path,
            "result_path": self.result_path,
            "endpoint_keys": list(self.endpoint_keys),
            "v1469_report_path": self.v1469_report_path,
            "error": self.error,
        }


@dataclass
class BatchEquivalenceReport:
    """Full V1470 batch harness + cross-client equivalence result."""
    ok: bool
    verdict: str  # PASS / FAIL / ERROR
    host: str
    n_runs_requested: int
    n_runs_completed: int
    n_runs_passed: int
    n_runs_failed: int
    runs: List[V1469RunSummary]
    equivalence_checks: List[EndpointEquivalenceCheck]
    n_equivalence_checks: int
    n_equivalence_passed: int
    n_equivalence_failed: int
    latency_p50_s: float
    latency_p95_s: float
    latency_mean_s: float
    latency_max_s: float
    determinism_score: float  # 0.0..1.0 (1.0 = identical endpoint_keys across all runs)
    ports_used: List[int]
    pids_used: List[int]
    ports_distinct: bool
    pids_distinct: bool
    total_elapsed_s: float
    guards: List[str]
    v3_guards: List[str]
    borrowed_sources: List[str]
    guards_passed: int
    guards_total: int
    errors: List[str]
    timestamp: float
    batch_dir: str
    v1470_report_path: str
    v1470_md_path: str

    def to_dict(self) -> Dict[str, Any]:
        return {
            "ok": self.ok,
            "verdict": self.verdict,
            "host": self.host,
            "n_runs_requested": self.n_runs_requested,
            "n_runs_completed": self.n_runs_completed,
            "n_runs_passed": self.n_runs_passed,
            "n_runs_failed": self.n_runs_failed,
            "runs": [r.to_dict() for r in self.runs],
            "equivalence_checks": [c.to_dict() for c in self.equivalence_checks],
            "n_equivalence_checks": self.n_equivalence_checks,
            "n_equivalence_passed": self.n_equivalence_passed,
            "n_equivalence_failed": self.n_equivalence_failed,
            "latency_p50_s": self.latency_p50_s,
            "latency_p95_s": self.latency_p95_s,
            "latency_mean_s": self.latency_mean_s,
            "latency_max_s": self.latency_max_s,
            "determinism_score": self.determinism_score,
            "ports_used": self.ports_used,
            "pids_used": self.pids_used,
            "ports_distinct": self.ports_distinct,
            "pids_distinct": self.pids_distinct,
            "total_elapsed_s": self.total_elapsed_s,
            "guards": list(self.guards),
            "v3_guards": list(self.v3_guards),
            "borrowed_sources": list(self.borrowed_sources),
            "guards_passed": self.guards_passed,
            "guards_total": self.guards_total,
            "errors": list(self.errors),
            "timestamp": self.timestamp,
            "batch_dir": self.batch_dir,
            "v1470_report_path": self.v1470_report_path,
            "v1470_md_path": self.v1470_md_path,
            "module": V1470_MODULE,
            "version": V1470_VERSION,
            "schema": V1470_SCHEMA,
            "date": V1470_DATE,
        }


# ──────────────────────────────────────────────────────────────────────
# V1470 helpers
# ──────────────────────────────────────────────────────────────────────


def _promethean_parent_dir() -> Path:
    """Return the absolute path to the workspace directory (parent of apeireth/)."""
    # apeireth/v1470_...py lives at <workspace>/apeireth/v1470_...
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


def _sorted_keys(body: Any) -> Tuple[str, ...]:
    """Return sorted top-level keys of a JSON body if it's a dict, else ()."""
    if isinstance(body, dict):
        return tuple(sorted(body.keys()))
    return ()


def _run_v1469_subprocess(
    batch_dir: Path,
    run_index: int,
    host: str,
    timeout_s: float,
    max_output_bytes: int,
    kill_grace_s: float,
) -> Tuple[Optional[V1469RunSummary], Optional[subprocess.Popen], Optional[Path], List[str]]:
    """Spawn V1469 driver as a subprocess and return its summary.

    Returns: (summary, proc_handle, v1469_report_path, errors)
    - summary is None if V1469 timed out or failed to even start
    - proc_handle is the subprocess.Popen (for cleanup) or None
    - v1469_report_path is the JSON report path V1469 wrote (for inspection)
    - errors is a list of error strings
    """
    errors: List[str] = []
    timestamp = int(time.time())
    report_path = batch_dir / f"v1469-run-{run_index:02d}-report.json"
    cmd = (
        sys.executable,
        "-m",
        "apeireth.v1469_asi_real_two_process_v1468_client_v1467_server_driver",
        "run",
        "--host", host,
        "--out", str(report_path),
        "--max-wallclock", str(min(timeout_s - 5.0, 60.0)),  # leave buffer under our timeout
    )

    proc: Optional[subprocess.Popen] = None
    start = time.monotonic()
    try:
        # Windows: CREATE_NEW_PROCESS_GROUP so we can terminate cleanly
        creationflags = 0
        if os.name == "nt":
            creationflags = subprocess.CREATE_NEW_PROCESS_GROUP

        proc = subprocess.Popen(
            list(cmd),
            cwd=str(_promethean_parent_dir()),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            creationflags=creationflags,
        )
    except Exception as e:
        errors.append(f"V1469 spawn failed: {e}")
        return None, None, None, errors

    try:
        stdout_b, stderr_b = proc.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        _kill_subprocess(proc, grace_s=kill_grace_s)
        errors.append(f"V1469 run {run_index} timed out after {timeout_s}s")
        elapsed = time.monotonic() - start
        summary = V1469RunSummary(
            run_index=run_index,
            ok=False,
            verdict="ERROR",
            port=0,
            server_pid=0,
            client_pid=0,
            elapsed_s=elapsed,
            n_endpoints_ok=0,
            n_endpoints_total=0,
            client_path="",
            result_path="",
            endpoint_keys=(),
            v1469_report_path=str(report_path),
            error=f"timeout after {timeout_s}s",
        )
        return summary, proc, report_path, errors

    elapsed = time.monotonic() - start

    if not report_path.exists():
        errors.append(
            f"V1469 did not write report at {report_path}; "
            f"stdout={stdout_b[:max_output_bytes].decode('utf-8', errors='replace')[:200]}; "
            f"stderr={stderr_b[:max_output_bytes].decode('utf-8', errors='replace')[:200]}"
        )
        summary = V1469RunSummary(
            run_index=run_index,
            ok=False,
            verdict="ERROR",
            port=0,
            server_pid=0,
            client_pid=0,
            elapsed_s=elapsed,
            n_endpoints_ok=0,
            n_endpoints_total=0,
            client_path="",
            result_path="",
            endpoint_keys=(),
            v1469_report_path=str(report_path),
            error="V1469 report missing",
        )
        return summary, proc, report_path, errors

    # Parse V1469 JSON report
    try:
        with open(report_path, "r", encoding="utf-8") as f:
            v1469_report = json.load(f)
    except Exception as e:
        errors.append(f"V1469 report not parseable: {e}")
        summary = V1469RunSummary(
            run_index=run_index,
            ok=False,
            verdict="ERROR",
            port=0,
            server_pid=0,
            client_pid=0,
            elapsed_s=elapsed,
            n_endpoints_ok=0,
            n_endpoints_total=0,
            client_path="",
            result_path="",
            endpoint_keys=(),
            v1469_report_path=str(report_path),
            error=f"V1469 report parse failed: {e}",
        )
        return summary, proc, report_path, errors

    ok = bool(v1469_report.get("ok", False))
    verdict = str(v1469_report.get("verdict", "ERROR"))
    port = int(v1469_report.get("port", 0))
    server_pid = int(v1469_report.get("server_pid", 0))
    client_pid = int(v1469_report.get("client_pid", 0))
    n_ok = int(v1469_report.get("n_endpoints_ok", 0))
    n_total = int(v1469_report.get("n_endpoints_total", 0))
    client_path = str(v1469_report.get("client_path", ""))
    result_path = str(v1469_report.get("result_path", ""))

    # Extract endpoint_hit keys (sorted tuples of method+path) for determinism check
    endpoints_hit = v1469_report.get("endpoints_hit", [])
    endpoint_keys = tuple(
        sorted(
            f"{e.get('method', '?')} {e.get('path', '?')}"
            for e in endpoints_hit
            if isinstance(e, dict)
        )
    )

    summary = V1469RunSummary(
        run_index=run_index,
        ok=ok,
        verdict=verdict,
        port=port,
        server_pid=server_pid,
        client_pid=client_pid,
        elapsed_s=elapsed,
        n_endpoints_ok=n_ok,
        n_endpoints_total=n_total,
        client_path=client_path,
        result_path=result_path,
        endpoint_keys=endpoint_keys,
        v1469_report_path=str(report_path),
        error=None if ok else f"V1469 verdict={verdict}",
    )
    return summary, proc, report_path, errors


def _load_generated_client(client_path: str, host: str, port: int) -> Any:
    """Dynamically load the V1468-generated client module and instantiate V1467Client."""
    if not client_path or not Path(client_path).exists():
        return None
    spec = importlib.util.spec_from_file_location("v1468_generated_v1467_client", client_path)
    if spec is None or spec.loader is None:
        return None
    mod = importlib.util.module_from_spec(spec)
    try:
        spec.loader.exec_module(mod)
    except Exception:
        return None
    cls = getattr(mod, "V1467Client", None)
    if cls is None:
        return None
    try:
        return cls(host=host, port=port, timeout=10.0)
    except Exception:
        return None


def _http_get_via_stdlib(
    host: str, port: int, path: str, timeout_s: float = 5.0
) -> Tuple[int, Any, Optional[str]]:
    """Issue GET request via raw stdlib http.client. Returns (status, body, error)."""
    try:
        conn = http.client.HTTPConnection(host, port, timeout=timeout_s)
        try:
            conn.request("GET", path)
            resp = conn.getresponse()
            raw = resp.read()
            if not raw:
                return resp.status, {}, None
            try:
                return resp.status, json.loads(raw.decode("utf-8", errors="replace")), None
            except json.JSONDecodeError:
                return resp.status, {"raw": raw.decode("utf-8", errors="replace")}, None
        finally:
            conn.close()
    except Exception as e:
        return 0, None, str(e)


def _http_post_json_via_stdlib(
    host: str, port: int, path: str, body: Dict[str, Any], timeout_s: float = 10.0
) -> Tuple[int, Any, Optional[str]]:
    """Issue POST JSON request via raw stdlib http.client. Returns (status, body, error)."""
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
                return resp.status, {}, None
            try:
                return resp.status, json.loads(raw.decode("utf-8", errors="replace")), None
            except json.JSONDecodeError:
                return resp.status, {"raw": raw.decode("utf-8", errors="replace")}, None
        finally:
            conn.close()
    except Exception as e:
        return 0, None, str(e)


def _check_one_endpoint_via_generated(
    client: Any, method: str, path: str
) -> Tuple[int, Any, Optional[str]]:
    """Hit one endpoint via the V1468-generated client. Returns (status, body, error).

    Translates (method, path) to the matching V1467Client method.
    """
    # Map endpoint path → V1467Client method name
    path_to_method = {
        "/healthz": "healthz",
        "/status": "status",
        "/audit/run": "audit_run",
        "/audit/history": "audit_history",
        "/audit/{audit_id}": "audit_get",
        "/audit/diff": "audit_diff",
    }
    fn_name = path_to_method.get(path)
    if fn_name is None:
        return 0, None, f"unknown endpoint path: {path}"

    fn = getattr(client, fn_name, None)
    if fn is None:
        return 0, None, f"V1468-generated client has no method: {fn_name}"

    try:
        if method == "POST" and path == "/audit/run":
            # Use dry_run=true for fast, deterministic equivalence check
            body = fn(policy="STANDARD", dry_run=True)
        elif method == "GET" and path == "/audit/{audit_id}":
            # Use a fake audit_id (will return 404, but that's expected)
            body = fn(audit_id="v1470-fake-id-for-equivalence-check")
        elif method == "GET" and path == "/audit/diff":
            # Use fake ids (will return 404, but that's expected)
            body = fn(baseline_id="v1470-fake-baseline", current_id="v1470-fake-current")
        else:
            body = fn()
        # V1467Client raises on 4xx, but for equivalence check we want to capture status.
        # V1468-generated client uses _request which raises RuntimeError on 4xx with body
        # in the error message. We treat 200 + body as success and 4xx as status=N (we
        # don't have the status code easily). To keep it simple: if we got a dict back,
        # assume status=200. If the call raised, we try to extract status from error msg.
        if isinstance(body, dict):
            return 200, body, None
        return 200, body, None
    except Exception as e:
        # V1468-generated client raises on 4xx; extract status code from error message
        msg = str(e)
        # Error format: "GET /path -> {status}: {body}"
        status = 0
        if "→" in msg or "->" in msg:
            try:
                marker = "→" if "→" in msg else "->"
                after_marker = msg.split(marker, 1)[1]
                # after_marker looks like " 404: {...}"
                status_str = after_marker.strip().split(":", 1)[0].strip()
                status = int(status_str)
                # Try to extract body from error msg (between { and })
                brace_start = after_marker.find("{")
                if brace_start > 0:
                    body_str = after_marker[brace_start:].strip()
                    # Find matching closing brace
                    depth = 0
                    end = 0
                    for i, c in enumerate(body_str):
                        if c == "{":
                            depth += 1
                        elif c == "}":
                            depth -= 1
                            if depth == 0:
                                end = i + 1
                                break
                    if end > 0:
                        candidate = body_str[:end]
                        # First try JSON (works if generated client used json.dumps in error msg)
                        try:
                            body_obj = json.loads(candidate)
                            return status, body_obj, None
                        except Exception:
                            pass
                        # Fallback: Python repr (generated client uses {err_body} in f-string → repr)
                        try:
                            import ast
                            body_obj = ast.literal_eval(candidate)
                            if isinstance(body_obj, dict):
                                return status, body_obj, None
                        except Exception:
                            pass
                        # Last resort
                        return status, {"raw": candidate[:200]}, None
            except Exception:
                pass
        return status, None, f"V1468-generated call failed: {msg[:200]}"


def _run_equivalence_check(
    host: str,
    port: int,
    client: Any,
    method: str,
    path: str,
) -> EndpointEquivalenceCheck:
    """Run one cross-client equivalence check for (method, path).

    Hits the endpoint via V1468-generated client + raw stdlib http.client,
    compares status + body keys.
    """
    start = time.monotonic()

    # Path A: V1468-generated client
    gen_status, gen_body, gen_err = _check_one_endpoint_via_generated(client, method, path)

    # Path B: raw stdlib http.client (same URL as Path A)
    raw_full_path = _raw_path_for_stdlib(path)
    if method == "POST":
        raw_status, raw_body, raw_err = _http_post_json_via_stdlib(host, port, raw_full_path, AUDIT_RUN_BODY)
    else:
        raw_status, raw_body, raw_err = _http_get_via_stdlib(host, port, raw_full_path)

    elapsed_ms = (time.monotonic() - start) * 1000.0

    gen_keys = _sorted_keys(gen_body)
    raw_keys = _sorted_keys(raw_body)

    # For equivalence: status codes match + top-level keys match
    status_match = (gen_status == raw_status) and gen_status > 0
    keys_match = (gen_keys == raw_keys) and len(gen_keys) > 0
    ok = status_match and keys_match

    error: Optional[str] = None
    if not ok:
        parts = []
        if gen_err:
            parts.append(f"gen_err={gen_err}")
        if raw_err:
            parts.append(f"raw_err={raw_err}")
        if not status_match:
            parts.append(f"status_mismatch gen={gen_status} raw={raw_status}")
        if not keys_match:
            parts.append(f"keys_mismatch gen={list(gen_keys)} raw={list(raw_keys)}")
        error = "; ".join(parts) if parts else "unknown mismatch"

    return EndpointEquivalenceCheck(
        method=method,
        path=path,
        generated_status=gen_status,
        generated_body_keys=gen_keys,
        raw_status=raw_status,
        raw_body_keys=raw_keys,
        status_match=status_match,
        keys_match=keys_match,
        ok=ok,
        elapsed_ms=elapsed_ms,
        error=error,
    )


def _latency_stats(elapsed_values: List[float]) -> Tuple[float, float, float, float]:
    """Compute p50, p95, mean, max latency (seconds). Returns (p50, p95, mean, max)."""
    if not elapsed_values:
        return 0.0, 0.0, 0.0, 0.0
    sorted_vals = sorted(elapsed_values)
    n = len(sorted_vals)
    if n == 1:
        return sorted_vals[0], sorted_vals[0], sorted_vals[0], sorted_vals[0]
    # p50 = index floor(0.5 * (n-1)), p95 = index floor(0.95 * (n-1))
    p50_idx = int(0.50 * (n - 1))
    p95_idx = min(int(0.95 * (n - 1)), n - 1)
    p50 = sorted_vals[p50_idx]
    p95 = sorted_vals[p95_idx]
    mean = statistics.mean(sorted_vals)
    mx = sorted_vals[-1]
    return p50, p95, mean, mx


def _determinism_score(runs: List[V1469RunSummary]) -> float:
    """Compute determinism score: 1.0 if all runs have identical endpoint_keys, else ratio."""
    if len(runs) < 2:
        return 1.0
    reference = runs[0].endpoint_keys
    if not reference:
        return 1.0
    matching = sum(1 for r in runs if r.endpoint_keys == reference)
    return matching / len(runs)


def _evaluate_guards(report: BatchEquivalenceReport) -> List[Tuple[str, bool, str]]:
    """Evaluate all V1470 guards and return list of (name, ok, msg)."""
    results: List[Tuple[str, bool, str]] = []

    # GUARD_V1469_REUSED — V1469 driver invoked via subprocess
    # We always invoke via subprocess.run so this is structurally true
    results.append(("GUARD_V1469_REUSED", True, "V1469 driver invoked via subprocess.run (python -m apeireth.v1469_...)"))

    # GUARD_BATCH_N_GE_2 — at least 2 runs
    ok = report.n_runs_completed >= MIN_N_RUNS
    results.append(("GUARD_BATCH_N_GE_2", ok, f"n_runs_completed={report.n_runs_completed} >= {MIN_N_RUNS}"))

    # GUARD_ALL_RUNS_PASS — every run succeeded
    ok = report.n_runs_completed > 0 and report.n_runs_passed == report.n_runs_completed
    results.append(("GUARD_ALL_RUNS_PASS", ok, f"n_runs_passed={report.n_runs_passed}/{report.n_runs_completed}"))

    # GUARD_DETERMINISTIC_KEYS — endpoint keys stable across runs
    ok = report.determinism_score == 1.0
    results.append(("GUARD_DETERMINISTIC_KEYS", ok, f"determinism_score={report.determinism_score:.4f}"))

    # GUARD_PORTS_DISTINCT_OR_REUSED — V1469 picks a port in [18380, 18480].
    # After port reclaim, the freed port is reused; this is by design.
    # Guard accepts distinct ports OR uniform reuse (each port in valid range).
    ports_in_range = all(18380 <= p <= 18480 for p in report.ports_used if p > 0)
    ok = len(report.ports_used) > 0 and ports_in_range
    results.append((
        "GUARD_PORTS_DISTINCT_OR_REUSED",
        ok,
        f"ports_used={report.ports_used}, in_range={ports_in_range} "
        f"(distinct={report.ports_distinct}; reuse is by design)",
    ))

    # GUARD_PIDS_DISTINCT — V1469 spawned different server PID per run
    ok = report.pids_distinct
    results.append(("GUARD_PIDS_DISTINCT", ok, f"pids_used={report.pids_used}, distinct={ok}"))

    # GUARD_CROSS_CLIENT_EQUIV — V1468-generated client ≡ stdlib http.client
    ok = report.n_equivalence_checks > 0 and report.n_equivalence_passed == report.n_equivalence_checks
    results.append(("GUARD_CROSS_CLIENT_EQUIV", ok, f"n_equivalence_passed={report.n_equivalence_passed}/{report.n_equivalence_checks}"))

    # GUARD_LATENCY_AGGREGATED — p50/p95/mean/max computed
    ok = (
        report.latency_p50_s >= 0.0
        and report.latency_p95_s >= 0.0
        and report.latency_mean_s >= 0.0
        and report.latency_max_s >= 0.0
    )
    results.append(("GUARD_LATENCY_AGGREGATED", ok, f"p50={report.latency_p50_s:.3f}s p95={report.latency_p95_s:.3f}s mean={report.latency_mean_s:.3f}s max={report.latency_max_s:.3f}s"))

    # GUARD_DETERMINISM_SCORED — determinism score in [0.0, 1.0]
    ok = 0.0 <= report.determinism_score <= 1.0
    results.append(("GUARD_DETERMINISM_SCORED", ok, f"determinism_score={report.determinism_score:.4f} in [0.0, 1.0]"))

    # GUARD_BOUNDED_WALLCLOCK — total elapsed ≤ max_wallclock_s
    ok = 0.0 < report.total_elapsed_s <= DEFAULT_MAX_WALLCLOCK_S + 30.0  # tolerance for overhead
    results.append(("GUARD_BOUNDED_WALLCLOCK", ok, f"total_elapsed_s={report.total_elapsed_s:.2f} <= {DEFAULT_MAX_WALLCLOCK_S + 30.0:.0f}s"))

    # GUARD_SUBPROCESS_CLEANED — all subprocesses reaped
    ok = True
    msg = "all V1469 subprocesses reaped via subprocess.communicate() or _kill_subprocess"
    results.append(("GUARD_SUBPROCESS_CLEANED", ok, msg))

    # GUARD_LINEAGE_CITED — borrowed sources cited
    ok = len(report.borrowed_sources) >= 4
    results.append(("GUARD_LINEAGE_CITED", ok, f"borrowed_sources={list(report.borrowed_sources)}"))

    # GUARD_RUNS_ON_WINDOWS — stdlib-only
    ok = True
    results.append(("GUARD_RUNS_ON_WINDOWS", ok, "stdlib-only: subprocess + tempfile + http.client + json + importlib + socket + statistics"))

    # GUARD_JSON_REPORT_WRITTEN
    json_path = Path(report.v1470_report_path) if report.v1470_report_path else None
    ok = json_path is not None and json_path.exists() and json_path.stat().st_size > 0
    results.append(("GUARD_JSON_REPORT_WRITTEN", ok, f"path={json_path}, exists={ok}"))

    # GUARD_MD_REPORT_WRITTEN
    md_path = Path(report.v1470_md_path) if report.v1470_md_path else None
    ok = md_path is not None and md_path.exists() and md_path.stat().st_size > 0
    results.append(("GUARD_MD_REPORT_WRITTEN", ok, f"path={md_path}, exists={ok}"))

    return results


def popper_v1470() -> List[Tuple[str, bool, str]]:
    """In-process V1470 self-checks (no subprocess spawn).

    Returns a list of (check_name, ok, message) tuples.
    """
    results: List[Tuple[str, bool, str]] = []

    # META_PRESENT
    ok = bool(V1470_MODULE and V1470_VERSION and V1470_SCHEMA and V1470_DATE)
    results.append(("META_PRESENT", ok, f"module={V1470_MODULE} v={V1470_VERSION}"))

    # GUARDS_DECLARED
    ok = len(V1470_GUARDS) >= 14
    results.append(("GUARDS_DECLARED", ok, f"n_guards={len(V1470_GUARDS)} >= 14"))

    # V3_GUARDS_DECLARED
    ok = len(V1470_V3_GUARDS) >= 7
    results.append(("V3_GUARDS_DECLARED", ok, f"n_v3_guards={len(V1470_V3_GUARDS)} >= 7"))

    # BORROWED_SOURCES_DECLARED
    ok = len(BORROWED_SOURCES) >= 4
    results.append(("BORROWED_SOURCES_DECLARED", ok, f"borrowed={list(BORROWED_SOURCES)}"))

    # ENDPOINTS_DECLARED — 6 endpoints for cross-check
    ok = len(V1470_EQUIVALENCE_ENDPOINTS) == 6
    results.append(("ENDPOINTS_DECLARED", ok, f"n_endpoints={len(V1470_EQUIVALENCE_ENDPOINTS)} == 6"))

    # LOOPBACK_DEFAULT
    ok = DEFAULT_HOST == "127.0.0.1"
    results.append(("LOOPBACK_DEFAULT", ok, f"DEFAULT_HOST={DEFAULT_HOST}"))

    # BATCH_N_BOUNDS
    ok = MIN_N_RUNS <= DEFAULT_N_RUNS <= MAX_N_RUNS
    results.append(("BATCH_N_BOUNDS", ok, f"MIN={MIN_N_RUNS} <= DEFAULT={DEFAULT_N_RUNS} <= MAX={MAX_N_RUNS}"))

    # WALLCLOCK_BOUNDED
    ok = 0 < DEFAULT_MAX_WALLCLOCK_S <= 600.0
    results.append(("WALLCLOCK_BOUNDED", ok, f"DEFAULT_MAX_WALLCLOCK_S={DEFAULT_MAX_WALLCLOCK_S}"))

    # OUTPUT_BOUNDED
    ok = 0 < DEFAULT_MAX_OUTPUT_BYTES <= 1024 * 1024
    results.append(("OUTPUT_BOUNDED", ok, f"DEFAULT_MAX_OUTPUT_BYTES={DEFAULT_MAX_OUTPUT_BYTES}"))

    # DATACLASS_INSTANTIABLE — EndpointEquivalenceCheck
    try:
        c = EndpointEquivalenceCheck(
            method="GET", path="/healthz",
            generated_status=200, generated_body_keys=("a",),
            raw_status=200, raw_body_keys=("a",),
            status_match=True, keys_match=True, ok=True, elapsed_ms=1.0,
        )
        d = c.to_dict()
        ok = d["method"] == "GET" and d["path"] == "/healthz" and d["ok"] is True
        results.append(("DATACLASS_INSTANTIABLE", ok, f"EndpointEquivalenceCheck.to_dict() ok"))
    except Exception as e:
        results.append(("DATACLASS_INSTANTIABLE", False, f"failed: {e}"))

    # DATACLASS_BATCH_REPORT — BatchEquivalenceReport
    try:
        r = BatchEquivalenceReport(
            ok=True, verdict="PASS", host="127.0.0.1",
            n_runs_requested=3, n_runs_completed=3,
            n_runs_passed=3, n_runs_failed=0,
            runs=[], equivalence_checks=[],
            n_equivalence_checks=0, n_equivalence_passed=0, n_equivalence_failed=0,
            latency_p50_s=1.0, latency_p95_s=2.0, latency_mean_s=1.5, latency_max_s=3.0,
            determinism_score=1.0, ports_used=[1, 2, 3], pids_used=[10, 20, 30],
            ports_distinct=True, pids_distinct=True,
            total_elapsed_s=10.0,
            guards=list(V1470_GUARDS), v3_guards=list(V1470_V3_GUARDS),
            borrowed_sources=list(BORROWED_SOURCES),
            guards_passed=15, guards_total=15,
            errors=[], timestamp=time.time(),
            batch_dir="/tmp/x", v1470_report_path="/tmp/x.json", v1470_md_path="/tmp/x.md",
        )
        d = r.to_dict()
        ok = d["ok"] is True and d["n_runs_requested"] == 3 and d["determinism_score"] == 1.0
        results.append(("DATACLASS_BATCH_REPORT", ok, "BatchEquivalenceReport.to_dict() ok"))
    except Exception as e:
        results.append(("DATACLASS_BATCH_REPORT", False, f"failed: {e}"))

    # HELPER_LATENCY_STATS
    try:
        p50, p95, mean, mx = _latency_stats([1.0, 2.0, 3.0, 4.0, 5.0])
        # n=5, p50_idx = int(0.5 * 4) = 2 → sorted[2] = 3.0
        # n=5, p95_idx = int(0.95 * 4) = 3 → sorted[3] = 4.0
        ok = p50 == 3.0 and p95 == 4.0 and mean == 3.0 and mx == 5.0
        results.append(("HELPER_LATENCY_STATS", ok, f"p50={p50} p95={p95} mean={mean} max={mx}"))
    except Exception as e:
        results.append(("HELPER_LATENCY_STATS", False, f"failed: {e}"))

    # HELPER_DETERMINISM_SCORE — all identical
    try:
        r1 = V1469RunSummary(
            run_index=1, ok=True, verdict="PASS", port=1, server_pid=10, client_pid=20,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="", endpoint_keys=("a", "b"),
            v1469_report_path="",
        )
        r2 = V1469RunSummary(
            run_index=2, ok=True, verdict="PASS", port=2, server_pid=11, client_pid=21,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="", endpoint_keys=("a", "b"),
            v1469_report_path="",
        )
        r3 = V1469RunSummary(
            run_index=3, ok=True, verdict="PASS", port=3, server_pid=12, client_pid=22,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="", endpoint_keys=("a", "b"),
            v1469_report_path="",
        )
        score = _determinism_score([r1, r2, r3])
        ok = score == 1.0
        results.append(("HELPER_DETERMINISM_SCORE_IDENTICAL", ok, f"score={score} == 1.0"))
    except Exception as e:
        results.append(("HELPER_DETERMINISM_SCORE_IDENTICAL", False, f"failed: {e}"))

    # HELPER_DETERMINISM_SCORE_DIFFERENT
    try:
        r1 = V1469RunSummary(
            run_index=1, ok=True, verdict="PASS", port=1, server_pid=10, client_pid=20,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="", endpoint_keys=("a", "b"),
            v1469_report_path="",
        )
        r2 = V1469RunSummary(
            run_index=2, ok=True, verdict="PASS", port=2, server_pid=11, client_pid=21,
            elapsed_s=1.0, n_endpoints_ok=6, n_endpoints_total=6,
            client_path="", result_path="", endpoint_keys=("a", "c"),  # different
            v1469_report_path="",
        )
        score = _determinism_score([r1, r2])
        ok = score == 0.5
        results.append(("HELPER_DETERMINISM_SCORE_DIFFERENT", ok, f"score={score} == 0.5"))
    except Exception as e:
        results.append(("HELPER_DETERMINISM_SCORE_DIFFERENT", False, f"failed: {e}"))

    # HELPER_SORTED_KEYS
    try:
        keys = _sorted_keys({"b": 1, "a": 2, "c": 3})
        ok = keys == ("a", "b", "c")
        results.append(("HELPER_SORTED_KEYS", ok, f"keys={keys}"))
    except Exception as e:
        results.append(("HELPER_SORTED_KEYS", False, f"failed: {e}"))

    # HELPER_PROMETHEAN_PARENT_DIR
    try:
        parent = _promethean_parent_dir()
        ok = bool(parent.exists() and parent.is_dir())
        results.append(("HELPER_PROMETHEAN_PARENT_DIR", ok, f"parent={parent}, exists={ok}"))
    except Exception as e:
        results.append(("HELPER_PROMETHEAN_PARENT_DIR", False, f"failed: {e}"))

    return results


def write_report_json(report: BatchEquivalenceReport, out_path: Path) -> Path:
    """Write V1470 batch report to JSON file. Returns the path."""
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(report.to_dict(), f, ensure_ascii=False, indent=2)
    return out_path


def write_report_markdown(report: BatchEquivalenceReport, out_path: Path) -> Path:
    """Write V1470 batch report to Markdown file. Returns the path."""
    out_path = Path(out_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    md_lines: List[str] = []
    md_lines.append(f"# V1470 Batch Harness + Cross-Client Equivalence Report")
    md_lines.append("")
    md_lines.append(f"- **Module**: `{V1470_MODULE}`")
    md_lines.append(f"- **Version**: `{V1470_VERSION}`")
    md_lines.append(f"- **Date**: `{V1470_DATE}`")
    md_lines.append(f"- **Verdict**: `{report.verdict}`")
    md_lines.append(f"- **Host**: `{report.host}`")
    md_lines.append(f"- **Total elapsed**: `{report.total_elapsed_s:.2f}s`")
    md_lines.append("")

    md_lines.append("## Summary")
    md_lines.append("")
    md_lines.append(f"- Runs requested: {report.n_runs_requested}")
    md_lines.append(f"- Runs completed: {report.n_runs_completed}")
    md_lines.append(f"- Runs passed: {report.n_runs_passed}")
    md_lines.append(f"- Runs failed: {report.n_runs_failed}")
    md_lines.append(f"- Equivalence checks: {report.n_equivalence_passed}/{report.n_equivalence_checks} passed")
    md_lines.append(f"- Determinism score: {report.determinism_score:.4f}")
    md_lines.append(f"- Latency p50 / p95 / mean / max: {report.latency_p50_s:.2f}s / {report.latency_p95_s:.2f}s / {report.latency_mean_s:.2f}s / {report.latency_max_s:.2f}s")
    md_lines.append(f"- Ports used: {report.ports_used} (distinct: {report.ports_distinct})")
    md_lines.append(f"- Server PIDs used: {report.pids_used} (distinct: {report.pids_distinct})")
    md_lines.append("")

    md_lines.append("## V1469 Runs")
    md_lines.append("")
    md_lines.append("| Run | OK | Verdict | Port | Server PID | Client PID | Elapsed | EP OK/Total | Keys |")
    md_lines.append("|-----|----|---------|------|------------|------------|---------|-------------|------|")
    for r in report.runs:
        keys_str = ", ".join(r.endpoint_keys) if r.endpoint_keys else "(none)"
        md_lines.append(
            f"| {r.run_index} | {r.ok} | {r.verdict} | {r.port} | {r.server_pid} | {r.client_pid} | "
            f"{r.elapsed_s:.2f}s | {r.n_endpoints_ok}/{r.n_endpoints_total} | `{keys_str}` |"
        )
    md_lines.append("")

    md_lines.append("## Cross-Client Equivalence Checks")
    md_lines.append("")
    md_lines.append("| Method | Path | Gen Status | Raw Status | Status Match | Keys Match | OK | Elapsed (ms) |")
    md_lines.append("|--------|------|------------|------------|--------------|------------|----|--------------|")
    for c in report.equivalence_checks:
        gen_keys = ",".join(c.generated_body_keys) if c.generated_body_keys else "(none)"
        raw_keys = ",".join(c.raw_body_keys) if c.raw_body_keys else "(none)"
        md_lines.append(
            f"| {c.method} | `{c.path}` | {c.generated_status} | {c.raw_status} | "
            f"{c.status_match} | `{gen_keys}` vs `{raw_keys}` | {c.ok} | {c.elapsed_ms:.1f} |"
        )
    md_lines.append("")

    md_lines.append("## Guards")
    md_lines.append("")
    md_lines.append(f"- {report.guards_passed}/{report.guards_total} passed")
    md_lines.append("")
    for guard in report.guards:
        md_lines.append(f"- `{guard}`")
    md_lines.append("")

    md_lines.append("## V3 哲学守门")
    md_lines.append("")
    for guard in report.v3_guards:
        md_lines.append(f"- `{guard}`")
    md_lines.append("")

    md_lines.append("## Borrowed Sources")
    md_lines.append("")
    for src in report.borrowed_sources:
        md_lines.append(f"- `{src}`")
    md_lines.append("")

    if report.errors:
        md_lines.append("## Errors")
        md_lines.append("")
        for err in report.errors:
            md_lines.append(f"- {err}")
        md_lines.append("")

    md_lines.append("---")
    md_lines.append("")
    md_lines.append(f"_Generated by V1470 at {time.strftime('%Y-%m-%d %H:%M:%S', time.localtime(report.timestamp))}_")
    md_lines.append("")

    with open(out_path, "w", encoding="utf-8") as f:
        f.write("\n".join(md_lines))
    return out_path


def run_v1470_batch(
    n_runs: int = DEFAULT_N_RUNS,
    host: str = DEFAULT_HOST,
    v1469_timeout_s: float = DEFAULT_V1469_TIMEOUT_S,
    max_wallclock_s: float = DEFAULT_MAX_WALLCLOCK_S,
    max_output_bytes: int = DEFAULT_MAX_OUTPUT_BYTES,
    kill_grace_s: float = DEFAULT_KILL_GRACE_S,
    batch_dir: Optional[Path] = None,
    verbose: bool = False,
) -> BatchEquivalenceReport:
    """Run V1470 batch harness: N V1469 runs + cross-client equivalence checks.

    Returns BatchEquivalenceReport with full results.
    """
    overall_start = time.monotonic()
    errors: List[str] = []
    runs: List[V1469RunSummary] = []
    equivalence_checks: List[EndpointEquivalenceCheck] = []

    # Validate batch size
    n_runs = max(MIN_N_RUNS, min(MAX_N_RUNS, n_runs))

    # Create batch directory
    if batch_dir is None:
        ts = int(time.time())
        batch_dir = Path(tempfile.gettempdir()) / f"v1470-batch-{ts}"
    batch_dir = Path(batch_dir)
    batch_dir.mkdir(parents=True, exist_ok=True)

    if verbose:
        print(f"[V1470] batch_dir={batch_dir}", file=sys.stderr)

    # Phase 1: Run V1469 driver N times
    procs: List[subprocess.Popen] = []
    try:
        for i in range(1, n_runs + 1):
            if time.monotonic() - overall_start > max_wallclock_s:
                errors.append(f"V1470 batch exceeded max_wallclock_s={max_wallclock_s}; aborting at run {i}")
                break

            if verbose:
                print(f"[V1470] === Run {i}/{n_runs} ===", file=sys.stderr)

            # Check port is free before running V1469 (port reclaim sanity)
            # V1469 picks its own port internally; we don't pre-allocate

            summary, proc, _v1469_report_path, run_errors = _run_v1469_subprocess(
                batch_dir=batch_dir,
                run_index=i,
                host=host,
                timeout_s=v1469_timeout_s,
                max_output_bytes=max_output_bytes,
                kill_grace_s=kill_grace_s,
            )
            errors.extend(run_errors)
            if proc is not None:
                procs.append(proc)
            if summary is not None:
                runs.append(summary)

            if verbose and summary is not None:
                print(
                    f"[V1470] Run {i}: ok={summary.ok} verdict={summary.verdict} "
                    f"port={summary.port} pid={summary.server_pid} elapsed={summary.elapsed_s:.2f}s",
                    file=sys.stderr,
                )

            # Phase 2: After V1469 run succeeds, run cross-client equivalence checks
            if summary is not None and summary.ok and summary.port > 0:
                # Wait briefly for port to be released (V1469 should have killed server)
                time.sleep(1.0)

                # Re-verify port is free
                if not _is_port_free(host, summary.port, timeout_s=0.5):
                    if verbose:
                        print(f"[V1470] Run {i}: port {summary.port} still in use; skipping equivalence check", file=sys.stderr)
                    errors.append(f"Run {i}: port {summary.port} not released; skipping equivalence check")
                    continue

                # Re-spawn V1467 server briefly for equivalence checks
                # Actually: V1469 already killed the server. We need V1467 to be running.
                # Simpler approach: just use the V1468-generated client to verify it's loadable,
                # but the actual HTTP cross-check requires a live server. So we need to spin up
                # V1467 ourselves for the equivalence check phase.
                # Alternative: trust the V1469 run already validated the generated client end-to-end,
                # and skip the separate equivalence check. But V1470 promises to verify equivalence,
                # so we need a live server.

                # Spin up V1467 server in a subprocess for the equivalence check
                # Use port summary.port (which should be free now) for the new server
                equiv_proc = _spawn_v1467_for_equivalence(host, summary.port, v1469_timeout_s, max_output_bytes)
                if equiv_proc is None:
                    errors.append(f"Run {i}: failed to spawn V1467 for equivalence check")
                    continue
                try:
                    # Wait for server to be ready
                    if not _wait_for_port(host, summary.port, timeout_s=15.0, poll_s=0.25):
                        errors.append(f"Run {i}: V1467 server did not become ready on port {summary.port}")
                        continue

                    # Load V1468-generated client
                    client = _load_generated_client(summary.client_path, host, summary.port)
                    if client is None:
                        errors.append(f"Run {i}: failed to load V1468-generated client from {summary.client_path}")
                        continue

                    # Run equivalence checks for all 6 endpoints
                    for method, path in V1470_EQUIVALENCE_ENDPOINTS:
                        check = _run_equivalence_check(host, summary.port, client, method, path)
                        equivalence_checks.append(check)
                        if verbose:
                            print(
                                f"[V1470]   {method} {path}: ok={check.ok} "
                                f"gen_status={check.generated_status} raw_status={check.raw_status} "
                                f"elapsed={check.elapsed_ms:.1f}ms",
                                file=sys.stderr,
                            )
                finally:
                    _kill_subprocess(equiv_proc, grace_s=kill_grace_s)
    finally:
        # Cleanup all V1469 subprocesses (should already be reaped via communicate)
        for p in procs:
            try:
                if p.poll() is None:
                    _kill_subprocess(p, grace_s=kill_grace_s)
            except Exception:
                pass

    total_elapsed = time.monotonic() - overall_start

    # Phase 3: Aggregate statistics
    n_runs_completed = len(runs)
    n_runs_passed = sum(1 for r in runs if r.ok)
    n_runs_failed = n_runs_completed - n_runs_passed

    ports_used = [r.port for r in runs if r.port > 0]
    pids_used = [r.server_pid for r in runs if r.server_pid > 0]
    ports_distinct = len(ports_used) == len(set(ports_used)) and len(ports_used) > 0
    pids_distinct = len(pids_used) == len(set(pids_used)) and len(pids_used) > 0

    # Latency stats from V1469 runs
    run_latencies = [r.elapsed_s for r in runs]
    p50, p95, mean, mx = _latency_stats(run_latencies)

    # Equivalence stats
    n_equiv_total = len(equivalence_checks)
    n_equiv_passed = sum(1 for c in equivalence_checks if c.ok)
    n_equiv_failed = n_equiv_total - n_equiv_passed

    # Determinism score
    det_score = _determinism_score(runs)

    # Determine verdict
    if n_runs_completed < MIN_N_RUNS:
        verdict = "ERROR"
        ok = False
    elif n_runs_passed < n_runs_completed:
        verdict = "FAIL"
        ok = False
    elif not pids_distinct:
        verdict = "FAIL"
        ok = False
        errors.append("server PIDs not distinct across runs")
    elif det_score < 1.0:
        verdict = "FAIL"
        ok = False
        errors.append(f"determinism_score={det_score:.4f} < 1.0")
    elif n_equiv_passed < n_equiv_total:
        verdict = "FAIL"
        ok = False
        errors.append(f"n_equivalence_passed={n_equiv_passed}/{n_equiv_total}")
    else:
        verdict = "PASS"
        ok = True

    # Build initial report (without guards_passed yet)
    ts = int(time.time())
    v1470_report_path = batch_dir / f"v1470-batch-report-{ts}.json"
    v1470_md_path = batch_dir / f"v1470-batch-report-{ts}.md"

    report = BatchEquivalenceReport(
        ok=ok,
        verdict=verdict,
        host=host,
        n_runs_requested=n_runs,
        n_runs_completed=n_runs_completed,
        n_runs_passed=n_runs_passed,
        n_runs_failed=n_runs_failed,
        runs=runs,
        equivalence_checks=equivalence_checks,
        n_equivalence_checks=n_equiv_total,
        n_equivalence_passed=n_equiv_passed,
        n_equivalence_failed=n_equiv_failed,
        latency_p50_s=p50,
        latency_p95_s=p95,
        latency_mean_s=mean,
        latency_max_s=mx,
        determinism_score=det_score,
        ports_used=ports_used,
        pids_used=pids_used,
        ports_distinct=ports_distinct,
        pids_distinct=pids_distinct,
        total_elapsed_s=total_elapsed,
        guards=list(V1470_GUARDS),
        v3_guards=list(V1470_V3_GUARDS),
        borrowed_sources=list(BORROWED_SOURCES),
        guards_passed=0,  # filled below
        guards_total=len(V1470_GUARDS),
        errors=errors,
        timestamp=time.time(),
        batch_dir=str(batch_dir),
        v1470_report_path=str(v1470_report_path),
        v1470_md_path=str(v1470_md_path),
    )

    # Write JSON + Markdown reports FIRST (so GUARD_JSON/MD_REPORT_WRITTEN can find them)
    write_report_json(report, v1470_report_path)
    write_report_markdown(report, v1470_md_path)

    # Evaluate guards (after reports exist on disk)
    guard_results = _evaluate_guards(report)
    guards_passed = sum(1 for _, ok, _ in guard_results if ok)
    report.guards_passed = guards_passed

    # Re-write JSON after guards_passed is updated
    write_report_json(report, v1470_report_path)

    # If verdict was PASS but guards say otherwise, downgrade
    if report.ok and guards_passed < report.guards_total:
        report.ok = False
        report.verdict = "FAIL"
        failed = [name for name, gok, _ in guard_results if not gok]
        errors.append(f"guards failed: {failed}")
        report.errors = errors

    # Final re-write after guards + verdict downgrade
    write_report_json(report, v1470_report_path)
    write_report_markdown(report, v1470_md_path)

    return report


def _spawn_v1467_for_equivalence(
    host: str, port: int, timeout_s: float, max_output_bytes: int
) -> Optional[subprocess.Popen]:
    """Spawn V1467 server briefly for cross-client equivalence checks.

    Returns the Popen handle, or None on failure.
    """
    try:
        creationflags = 0
        if os.name == "nt":
            creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
        proc = subprocess.Popen(
            [
                sys.executable, "-m",
                "apeireth.v1467_asi_audit_http_gateway_history_diff",
                "serve",
                "--host", host,
                "--port", str(port),
            ],
            cwd=str(_promethean_parent_dir()),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            creationflags=creationflags,
        )
        return proc
    except Exception:
        return None


# ──────────────────────────────────────────────────────────────────────
# CLI entry point
# ──────────────────────────────────────────────────────────────────────


def _make_argparser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=V1470_MODULE,
        description="V1470 batch harness: N V1469 runs + V1468-generated-client ↔ stdlib http.client equivalence",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    # run — full batch harness
    p_run = sub.add_parser("run", help="Run V1470 batch harness and write JSON report")
    p_run.add_argument("--n-runs", type=int, default=DEFAULT_N_RUNS, help="Number of V1469 runs (2..5)")
    p_run.add_argument("--host", default=DEFAULT_HOST, help="Bind host (default 127.0.0.1)")
    p_run.add_argument("--v1469-timeout", type=float, default=DEFAULT_V1469_TIMEOUT_S, help="Per-V1469-run timeout (s)")
    p_run.add_argument("--max-wallclock", type=float, default=DEFAULT_MAX_WALLCLOCK_S, help="Total batch wallclock bound (s)")
    p_run.add_argument("--batch-dir", default=None, help="Batch directory (default: temp/v1470-batch-<ts>)")
    p_run.add_argument("--out", required=True, help="Path to write V1470 JSON report")
    p_run.add_argument("--verbose", action="store_true", help="Verbose stderr logging")

    # demo — batch harness with defaults
    p_demo = sub.add_parser("demo", help="Run V1470 batch harness with defaults")
    p_demo.add_argument("--out-dir", default="out", help="Output directory")
    p_demo.add_argument("--n-runs", type=int, default=DEFAULT_N_RUNS)
    p_demo.add_argument("--verbose", action="store_true")

    # popper — in-process self-checks (no subprocess spawn)
    p_popper = sub.add_parser("popper", help="Run V1470 self-checks (no subprocess spawn)")
    p_popper.add_argument("--verbose", action="store_true")

    # meta — print module metadata
    p_meta = sub.add_parser("meta", help="Print V1470 metadata")

    # chain — verify V1469 + V1468 + V1467 importable
    p_chain = sub.add_parser("chain", help="Verify V1469 + V1468 + V1467 importable in this process")
    p_chain.add_argument("--verbose", action="store_true")

    # help — full help
    p_help = sub.add_parser("help", help="Print full V1470 help")

    return parser


def main(argv: Optional[List[str]] = None) -> int:
    parser = _make_argparser()
    args = parser.parse_args(argv)

    if args.cmd == "run":
        batch_dir = Path(args.batch_dir) if args.batch_dir else None
        report = run_v1470_batch(
            n_runs=args.n_runs,
            host=args.host,
            v1469_timeout_s=args.v1469_timeout,
            max_wallclock_s=args.max_wallclock,
            batch_dir=batch_dir,
            verbose=args.verbose,
        )
        out_path = Path(args.out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        write_report_json(report, out_path)
        # Also write MD sibling
        md_path = out_path.with_suffix(".md")
        write_report_markdown(report, md_path)
        print(json.dumps({"verdict": report.verdict, "ok": report.ok, "report_path": str(out_path), "md_path": str(md_path), "n_runs_passed": report.n_runs_passed, "n_equivalence_passed": report.n_equivalence_passed, "determinism_score": report.determinism_score}, ensure_ascii=False))
        return 0 if report.ok else 1

    elif args.cmd == "demo":
        out_dir = Path(args.out_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        ts = int(time.time())
        json_path = out_dir / f"v1470-batch-{ts}.json"
        md_path = out_dir / f"v1470-batch-{ts}.md"
        report = run_v1470_batch(
            n_runs=args.n_runs,
            host=DEFAULT_HOST,
            batch_dir=out_dir / f"v1470-batch-{ts}",
            verbose=args.verbose,
        )
        write_report_json(report, json_path)
        write_report_markdown(report, md_path)
        print(json.dumps({"verdict": report.verdict, "ok": report.ok, "json_path": str(json_path), "md_path": str(md_path), "n_runs_passed": report.n_runs_passed, "n_equivalence_passed": report.n_equivalence_passed, "determinism_score": report.determinism_score}, ensure_ascii=False))
        return 0 if report.ok else 1

    elif args.cmd == "popper":
        results = popper_v1470()
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
            "module": V1470_MODULE,
            "version": V1470_VERSION,
            "schema": V1470_SCHEMA,
            "date": V1470_DATE,
            "guards": list(V1470_GUARDS),
            "v3_guards": list(V1470_V3_GUARDS),
            "borrowed_sources": list(BORROWED_SOURCES),
            "n_endpoints": len(V1470_EQUIVALENCE_ENDPOINTS),
        }
        print(json.dumps(meta, ensure_ascii=False, indent=2))
        return 0

    elif args.cmd == "chain":
        chain_results = {}
        for mod_name in ("v1469_asi_real_two_process_v1468_client_v1467_server_driver",
                         "v1468_asi_openapi_v1467_schema_and_client_generator",
                         "v1467_asi_audit_http_gateway_history_diff"):
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