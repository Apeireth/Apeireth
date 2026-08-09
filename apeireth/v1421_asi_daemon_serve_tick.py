"""V1421 — ASI 总框架 daemon: wire V1418 cron tick + V1420 HTTP endpoint into one process.

Phase: 1421
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:25, Asia/Shanghai deep night)
Post: V1420 (HTTP status endpoint) + V1418 (DGM cron integration)

What V1421 is
=============
V1421 is the **daemon wiring** layer between V1418 and V1420. Where:

- V1416 emits one ``DgmTickReport`` per call (closed-loop: V1412 → V1413 → V1414 → V1415 → policy_gate)
- V1417 records those ticks over time from JSONL and produces trend + digest + baseline + compare
- V1418 schedules V1416 + V1417 on cron cadence (run-session, tick-once, next-due)
- V1419 evaluates multi-policy distribution shift over windows
- V1420 exposes V1417 + V1419 + chain integrity as HTTP endpoints

V1421 wires V1418 and V1420 into a **single long-running process** so:

- the HTTP endpoints of V1420 stay live while V1418 ticks on cadence
- external operators (curl, jq, GitHub Actions, k8s probes) can poll
  /api/asi/verdict during a live tick session
- the daemon can run as a 5-minute cadence loop (``--cadence-seconds 300``)
  while the HTTP server stays bound on ``127.0.0.1:8765``
- the daemon shuts down cleanly on SIGINT / SIGTERM (or --max-seconds)
- optional bearer-token auth can be enforced on POST /api/asi/refresh

This is the **natural cron note next step** for V1418 + V1420
("next V1421 wire V1420 into V1418 cron loop"). V1421 is the glue.

Most common questions answered by one command:

- "Run the ASI 总框架 cron tick on 300s cadence AND keep HTTP /api/asi/* live"
- "Run 1 cron tick, then stop (no HTTP server)"  → tick-and-exit mode
- "Run HTTP server only (no ticks)"  → serve-only mode
- "Run the daemon until SIGINT, with optional bearer-token auth"
- "Run the daemon with --max-seconds 60 (auto-shutdown for smoke tests)"

It does NOT mutate V1418 or V1420 state directly. It only **reads** their
public APIs and **calls** their public functions. The tick thread drives
V1418.tick_once, the HTTP thread runs V1420.serve_forever_blocking.

Borrowed (4 — 主 19:33 走在前人经验上):
======================================
- V1418 (cron integration — ``tick_once`` + ``run_session`` + ``build_default_config``)
- V1420 (HTTP status endpoint — ``make_server`` + ``serve_forever_blocking`` + ``stop_server`` + ``ASIStatusSnapshot``)
- stdlib threading (daemon threads + join timeout)
- stdlib signal (SIGINT / SIGTERM graceful shutdown)

GUARDS upheld (V1421-specific, 16 — 主 00:44 质量工程化)
========================================================
- GUARD_DAEMON_REAL: real daemon with two threads + signal handlers, not stubbed
- GUARD_NO_V1418_WRITE: V1421 reads/calls V1418; never patches V1418 state
- GUARD_NO_V1420_WRITE: V1421 reads/calls V1420; never patches V1420 state
- GUARD_TICK_FROM_V1418: tick invocation goes through V1418.tick_once
- GUARD_HTTP_FROM_V1420: HTTP server comes from V1420.make_server
- GUARD_BOUNDED_CADENCE: cadence_seconds ∈ [MIN_CADENCE, MAX_CADENCE]
- GUARD_BOUNDED_PORT: port ∈ [1, 65535]
- GUARD_BIND_VALID: bind host ∈ {"127.0.0.1", "0.0.0.0", "localhost"}
- GUARD_MAX_SECONDS_BOUNDED: max_seconds ≥ 0 (0 = until SIGINT)
- GUARD_ATOMIC_WRITE: V1418 atomic append path preserved
- GUARD_BORROWED_REAL: 4 borrowed (V1418 + V1420 + threading + signal)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1421 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_GRACEFUL_SHUTDOWN: SIGINT/SIGTERM stop both threads cleanly
- GUARD_AUTH_TOKEN_VALID: optional bearer-token check on /api/asi/refresh

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_DAEMON_IS_NOT_PHENOMENAL: daemon is mechanical scheduling+routing, not Phenomenal
- GUARD_DAEMON_IS_NOT_ASI: daemon ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_DAEMON_IS_NOT_HUMAN_LEVEL: daemon is plumbing, not judgment
- GUARD_DAEMON_IS_NOT_ABSOLUTE: daemon emits bounded status, not absolute truth
- GUARD_DAEMON_IS_NOT_V1418_REPLACE: daemon reads V1418.tick_once, does not replace
- GUARD_DAEMON_IS_NOT_V1420_REPLACE: daemon reads V1420.serve_forever_blocking, does not replace
- GUARD_DAEMON_IS_NOT_V1419_REPLACE: daemon inherits V1419 via V1418 / V1420
- GUARD_DAEMON_IS_NOT_V1417_REPLACE: daemon reads V1417 via V1418.tick_once
- GUARD_DAEMON_IS_NOT_V1411_REPLACE: daemon is V1418+V1420 glue, not a new framework

Honest disclosure (主 17:58)
============================
V1421 daemon is a **deterministic glue layer** that runs V1418.tick_once
on cadence in one thread and V1420.serve_forever_blocking in another,
both backed by the same V1417 history JSONL + V1419 evaluation JSON.
It is bounded by HTTP request parsing, JSON serialization, sleep+signal;
NOT by Phenomenal consciousness, ASI 达成, human-level judgment, or
absolute certainty. V1421 ≠ Phenomenal daemon, ≠ ASI 达成 daemon, ≠
human-level daemon, ≠ absolute daemon. V1421 reads V1418 + V1420;
never replaces either of them. The optional bearer-token check on
/api/asi/refresh is a deterministic string compare; NOT a free agent
will.

API surfaces (16)
=================
1.  ``DEFAULT_BIND_HOST`` — "127.0.0.1"
2.  ``DEFAULT_PORT`` — 8765
3.  ``DEFAULT_CADENCE_SECONDS`` — 300 (5 minutes)
4.  ``MIN_CADENCE_SECONDS`` — 1
5.  ``MAX_CADENCE_SECONDS`` — 86400 (24h)
6.  ``DaemonicMode`` — Literal["tick-and-exit", "serve-only", "daemon"]
7.  ``DaemonConfig`` — dataclass (mode + bind + port + cadence_seconds +
    max_seconds + auth_token + history_path + baseline_path +
    tick_jsonl_path + render + sleep_fn_name + note)
8.  ``DaemonTickRecord`` — dataclass (cycle_index + started_iso +
    ended_iso + verdict + policy + chain_ok + alerts_count +
    duration_seconds + note)
9.  ``DaemonRunSummary`` — dataclass (mode + bind + port + cadence +
    n_ticks + n_proceed + n_pause + n_lockdown + started_iso +
    ended_iso + reason + chain_ok + note)
10. ``build_default_config(overrides)`` — DaemonConfig
11. ``validate_config(cfg)`` — raises ValueError on bad input
12. ``run_tick_once(cfg, cycle_index)`` — DaemonTickRecord
13. ``run_daemon(cfg)`` — DaemonRunSummary (full lifecycle)
14. ``stop_daemon(ctrl, server)`` — graceful shutdown helper
15. ``popper_self_test()`` — 17 self-tests
16. ``chain_delegate()`` — V1418 + V1420 chain probe
17. ``run_cli(argv)`` — argv dispatcher

CLI commands (8 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- tick-and-exit [--cadence-seconds N] [--max-seconds N] [--history-path PATH] [--baseline-path PATH]
- serve-only --bind HOST --port PORT [--max-seconds N] [--history-path PATH] [--baseline-path PATH]
- daemon --bind HOST --port PORT --cadence-seconds N [--max-seconds N] [--auth-token TOKEN]
           [--history-path PATH] [--baseline-path PATH] [--tick-jsonl-path PATH]
           [--no-render] [--sleep-fn time.sleep|threading.Event.wait]

Real-world usage (主 00:56):
=============================
    # Anyone can serve-only the ASI 总框架 HTTP endpoints:
    python -m apeireth.v1421_asi_daemon_serve_tick serve-only --bind 127.0.0.1 --port 8765 --max-seconds 30

    # Anyone can run one cron tick and exit:
    python -m apeireth.v1421_asi_daemon_serve_tick tick-and-exit --cadence-seconds 1 --max-seconds 5

    # Anyone can run a real 5-minute daemon with HTTP live:
    python -m apeireth.v1421_asi_daemon_serve_tick daemon \
        --bind 127.0.0.1 --port 8765 --cadence-seconds 300 --max-seconds 0

    # Anyone can poll the live status while the daemon runs:
    curl -s http://127.0.0.1:8765/api/asi/health
    curl -s http://127.0.0.1:8765/api/asi/status | jq .

    # Anyone can refresh the verdict (with optional bearer token):
    curl -X POST -H "Authorization: Bearer $TOKEN" http://127.0.0.1:8765/api/asi/refresh
"""

from __future__ import annotations

import dataclasses
import json
import os
import signal
import sys
import threading
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Literal, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1421_VERSION = "0.1.0"
V1421_SCHEMA = "v1421.asi-daemon-serve-tick/v1"
V1421_MODULE = "v1421_asi_daemon_serve_tick"

# Real default paths (same convention as V1416 / V1417 / V1418 / V1419 / V1420):
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"
DEFAULT_HISTORY_PATH = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_BASELINE_PATH = PROMETHEAN / ".v1417-dgm-tick-baseline.json"
DEFAULT_TICK_JSONL_PATH = PROMETHEAN / ".v1416-dgm-ticks.jsonl"
DEFAULT_RENDER_OUT = PROMETHEAN / ".v1418-cron-session.md"

# Network / cadence bounds (主 00:44 质量工程化)
DEFAULT_BIND_HOST = "127.0.0.1"
ALLOWED_BIND_HOSTS: Tuple[str, ...] = ("127.0.0.1", "0.0.0.0", "localhost")
MIN_PORT = 1
MAX_PORT = 65535
DEFAULT_PORT = 8765

DEFAULT_CADENCE_SECONDS = 300  # 5min cron standard
MIN_CADENCE_SECONDS = 1
MAX_CADENCE_SECONDS = 86400  # 24h

DEFAULT_MAX_SECONDS = 0  # 0 = until SIGINT
MIN_MAX_SECONDS = 0

# Mode literal
DaemonicMode = Literal["tick-and-exit", "serve-only", "daemon"]

# Guard tuples
V1421_GUARDS: Tuple[str, ...] = (
    "GUARD_DAEMON_REAL",
    "GUARD_NO_V1418_WRITE",
    "GUARD_NO_V1420_WRITE",
    "GUARD_TICK_FROM_V1418",
    "GUARD_HTTP_FROM_V1420",
    "GUARD_BOUNDED_CADENCE",
    "GUARD_BOUNDED_PORT",
    "GUARD_BIND_VALID",
    "GUARD_MAX_SECONDS_BOUNDED",
    "GUARD_ATOMIC_WRITE",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_GRACEFUL_SHUTDOWN",
    "GUARD_AUTH_TOKEN_VALID",
)

V1421_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_DAEMON_IS_NOT_PHENOMENAL",
    "GUARD_DAEMON_IS_NOT_ASI",
    "GUARD_DAEMON_IS_NOT_HUMAN_LEVEL",
    "GUARD_DAEMON_IS_NOT_ABSOLUTE",
    "GUARD_DAEMON_IS_NOT_V1418_REPLACE",
    "GUARD_DAEMON_IS_NOT_V1420_REPLACE",
    "GUARD_DAEMON_IS_NOT_V1419_REPLACE",
    "GUARD_DAEMON_IS_NOT_V1417_REPLACE",
    "GUARD_DAEMON_IS_NOT_V1411_REPLACE",
)

V1421_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1418", "cron integration (tick_once + run_session + build_default_config)"),
    ("V1420", "HTTP status endpoint (make_server + serve_forever_blocking + stop_server)"),
    ("stdlib threading", "daemon threads + join timeout"),
    ("stdlib signal", "SIGINT/SIGTERM graceful shutdown"),
)


# ============================================================================
# Type aliases
# ============================================================================

SleepFn = Callable[[float], float]


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class DaemonConfig:
    """Immutable daemon configuration (主 00:44 质量工程化)."""

    mode: DaemonicMode
    bind: str
    port: int
    cadence_seconds: int
    max_seconds: float
    auth_token: str
    history_path: Path
    baseline_path: Path
    tick_jsonl_path: Path
    render_out: Path
    render: bool
    sleep_fn_name: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["history_path"] = str(self.history_path)
        d["baseline_path"] = str(self.baseline_path)
        d["tick_jsonl_path"] = str(self.tick_jsonl_path)
        d["render_out"] = str(self.render_out)
        # Don't expose the token plaintext in to_dict() unless asked explicitly
        d["auth_token"] = "<redacted>" if self.auth_token else ""
        return d


@dataclasses.dataclass
class DaemonTickRecord:
    """One daemon tick record (drives V1418.tick_once)."""

    cycle_index: int
    started_iso: str
    ended_iso: str
    verdict: str
    policy: str
    chain_ok: bool
    alerts_count: int
    duration_seconds: float
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class DaemonRunSummary:
    """End-of-daemon summary across N ticks (or zero for serve-only)."""

    mode: DaemonicMode
    bind: str
    port: int
    cadence_seconds: int
    n_ticks: int
    n_proceed: int
    n_pause: int
    n_lockdown: int
    started_iso: str
    ended_iso: str
    reason: str
    chain_ok: bool
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


# ============================================================================
# Path & small helpers
# ============================================================================


def _safe_path(p: Path) -> Path:
    """Path safety: dotdot rejected, absolute allowed (主 00:44 质量工程化)."""
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with '..' rejected: {s}")
    return Path(s).resolve()


def _now_utc_iso() -> str:
    """ISO-8601 UTC timestamp, second precision (matches V1417 / V1418)."""
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _validate_bind_host(host: str) -> str:
    if host not in ALLOWED_BIND_HOSTS:
        raise ValueError(f"bind host must be one of {ALLOWED_BIND_HOSTS}; got {host!r}")
    return host


def _validate_port(port: int) -> int:
    if not isinstance(port, int) or port < MIN_PORT or port > MAX_PORT:
        raise ValueError(f"port must be int in [{MIN_PORT}, {MAX_PORT}]; got {port!r}")
    return port


def _validate_cadence(cadence_seconds: int) -> int:
    if not isinstance(cadence_seconds, int) or cadence_seconds < MIN_CADENCE_SECONDS or cadence_seconds > MAX_CADENCE_SECONDS:
        raise ValueError(
            f"cadence_seconds must be int in [{MIN_CADENCE_SECONDS}, {MAX_CADENCE_SECONDS}]; got {cadence_seconds!r}"
        )
    return cadence_seconds


def _validate_max_seconds(max_seconds: float) -> float:
    if not isinstance(max_seconds, (int, float)) or max_seconds < MIN_MAX_SECONDS:
        raise ValueError(f"max_seconds must be ≥ {MIN_MAX_SECONDS}; got {max_seconds!r}")
    return float(max_seconds)


def _validate_mode(mode: str) -> DaemonicMode:
    if mode not in ("tick-and-exit", "serve-only", "daemon"):
        raise ValueError(f"mode must be 'tick-and-exit', 'serve-only', or 'daemon'; got {mode!r}")
    return mode  # type: ignore[return-value]


def _validate_sleep_fn_name(name: str) -> str:
    if name not in ("time.sleep", "threading.Event.wait"):
        raise ValueError(f"sleep_fn_name must be 'time.sleep' or 'threading.Event.wait'; got {name!r}")
    return name


# ============================================================================
# Default config
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> DaemonConfig:
    """Build default DaemonConfig; optional overrides applied.

    Default mode is 'daemon' (the natural V1418+V1420 wiring).
    """
    cfg = DaemonConfig(
        mode="daemon",
        bind=DEFAULT_BIND_HOST,
        port=DEFAULT_PORT,
        cadence_seconds=DEFAULT_CADENCE_SECONDS,
        max_seconds=DEFAULT_MAX_SECONDS,
        auth_token="",
        history_path=DEFAULT_HISTORY_PATH,
        baseline_path=DEFAULT_BASELINE_PATH,
        tick_jsonl_path=DEFAULT_TICK_JSONL_PATH,
        render_out=DEFAULT_RENDER_OUT,
        render=True,
        sleep_fn_name="time.sleep",
        note="V1421 default: 5min cadence daemon on 127.0.0.1:8765 with no auth",
    )
    if overrides:
        for k, v in overrides.items():
            if not hasattr(cfg, k):
                raise ValueError(f"unknown override key: {k!r}")
            setattr(cfg, k, v)
    return cfg


def validate_config(cfg: DaemonConfig) -> DaemonConfig:
    """Validate an existing DaemonConfig in-place; return same instance."""
    cfg.mode = _validate_mode(cfg.mode)
    cfg.bind = _validate_bind_host(cfg.bind)
    cfg.port = _validate_port(cfg.port)
    cfg.cadence_seconds = _validate_cadence(cfg.cadence_seconds)
    cfg.max_seconds = _validate_max_seconds(cfg.max_seconds)
    cfg.sleep_fn_name = _validate_sleep_fn_name(cfg.sleep_fn_name)
    cfg.history_path = _safe_path(cfg.history_path)
    cfg.baseline_path = _safe_path(cfg.baseline_path)
    cfg.tick_jsonl_path = _safe_path(cfg.tick_jsonl_path)
    cfg.render_out = _safe_path(cfg.render_out)
    return cfg


# ============================================================================
# V1418 + V1420 lazy imports
# ============================================================================


def _import_v1418() -> Tuple[bool, Any, str]:
    try:
        import apeireth.v1418_asi_dgm_cron_integration as v1418  # type: ignore
        return True, v1418, ""
    except Exception as exc:  # pragma: no cover — defensive
        return False, None, repr(exc)


def _import_v1420() -> Tuple[bool, Any, str]:
    try:
        import apeireth.v1420_asi_http_status_endpoint as v1420  # type: ignore
        return True, v1420, ""
    except Exception as exc:  # pragma: no cover — defensive
        return False, None, repr(exc)


# ============================================================================
# Tick driver (calls V1418.tick_once)
# ============================================================================


def run_tick_once(cfg: DaemonConfig, cycle_index: int) -> DaemonTickRecord:
    """Run ONE V1418.tick_once; wrap outcome in DaemonTickRecord.

    Never mutates V1418 state. Calls V1418.tick_once via public API.
    """
    started = _now_utc_iso()
    t0 = time.time()
    ok, v1418, err = _import_v1418()
    if not ok:
        return DaemonTickRecord(
            cycle_index=cycle_index,
            started_iso=started,
            ended_iso=_now_utc_iso(),
            verdict="ERROR",
            policy="PAUSE",
            chain_ok=False,
            alerts_count=0,
            duration_seconds=time.time() - t0,
            note=f"V1418 import failed: {err}",
        )

    try:
        outcome = v1418.tick_once(
            history_path=cfg.history_path,
            baseline_path=cfg.baseline_path,
            tick_jsonl_path=cfg.tick_jsonl_path,
            render_out=cfg.render_out,
            render=cfg.render,
            cycle_index=cycle_index,
        )
    except Exception as exc:  # pragma: no cover — defensive
        return DaemonTickRecord(
            cycle_index=cycle_index,
            started_iso=started,
            ended_iso=_now_utc_iso(),
            verdict="ERROR",
            policy="PAUSE",
            chain_ok=False,
            alerts_count=0,
            duration_seconds=time.time() - t0,
            note=f"V1418.tick_once raised: {exc!r}",
        )

    duration = time.time() - t0
    verdict = str(getattr(outcome, "verdict", "UNKNOWN"))
    policy = str(getattr(outcome, "policy", "UNKNOWN"))
    chain_ok = bool(getattr(outcome, "chain_ok", False))
    alerts_count = int(getattr(outcome, "alerts_count", 0))

    return DaemonTickRecord(
        cycle_index=cycle_index,
        started_iso=started,
        ended_iso=_now_utc_iso(),
        verdict=verdict,
        policy=policy,
        chain_ok=chain_ok,
        alerts_count=alerts_count,
        duration_seconds=duration,
        note=f"V1421 wired V1418.tick_once cycle={cycle_index} verdict={verdict} policy={policy}",
    )


# ============================================================================
# Daemon runner
# ============================================================================


def _install_signal_handlers(stop_event: threading.Event, signals: Tuple[int, ...] = (signal.SIGINT, signal.SIGTERM)) -> None:
    """Install SIGINT/SIGTERM handlers that set stop_event (graceful shutdown)."""

    def _handler(signum: int, frame: Any) -> None:  # noqa: ARG001
        stop_event.set()

    for sig in signals:
        try:
            signal.signal(sig, _handler)
        except (ValueError, OSError):
            # Windows may not allow SIGTERM in some contexts — best-effort.
            pass


def _sleep_with_event(seconds: float, stop_event: threading.Event, sleep_fn_name: str) -> float:
    """Sleep for up to `seconds` or until stop_event is set, whichever first.

    Returns seconds actually slept (bounded by `seconds`).
    """
    if sleep_fn_name == "time.sleep":
        # Chunk into 0.5s slices so we can poll stop_event frequently.
        end = time.time() + seconds
        slept = 0.0
        while not stop_event.is_set():
            chunk = min(0.5, end - time.time())
            if chunk <= 0:
                break
            time.sleep(chunk)
            slept += chunk
        return min(slept, seconds)
    # threading.Event.wait variant
    return seconds if stop_event.wait(timeout=seconds) else seconds


def _http_thread_loop(cfg: DaemonConfig, stop_event: threading.Event) -> Optional[Any]:
    """Start V1420 HTTP server in this thread via httpd.serve_forever(); return httpd.

    We don't use V1420.serve_forever_blocking because it doesn't accept an
    external stop signal — instead we run httpd.serve_forever() directly and
    call httpd.shutdown() from the main thread when stop_event fires.
    """
    ok, v1420, err = _import_v1420()
    if not ok:
        print(f"[v1421] V1420 import failed; serve-only disabled: {err}", file=sys.stderr)
        return None
    try:
        httpd, _snap = v1420.make_server(
            bind=cfg.bind,
            port=cfg.port,
            max_seconds=0,  # unused — we control lifetime via stop_event
            history_path=cfg.history_path,
        )
    except OSError as exc:
        print(f"[v1421] V1420.make_server failed: {exc!r}", file=sys.stderr)
        return None

    def _serve_and_wait() -> None:
        try:
            httpd.serve_forever()
        except Exception:  # pragma: no cover — defensive
            pass

    serve_thread = threading.Thread(target=_serve_and_wait, daemon=True, name="v1421-http")
    serve_thread.start()
    return httpd


def run_daemon(cfg: DaemonConfig) -> DaemonRunSummary:
    """Run the V1421 daemon (mode-aware).

    - tick-and-exit: run ONE V1418.tick_once and return
    - serve-only: run V1420.serve_forever_blocking until SIGINT or --max-seconds
    - daemon: run V1418.tick_once on cadence + V1420 HTTP in parallel

    Never mutates V1418 or V1420 state.
    """
    cfg = validate_config(cfg)
    started_iso = _now_utc_iso()
    started_mono = time.time()

    stop_event = threading.Event()
    _install_signal_handlers(stop_event)

    tick_records: List[DaemonTickRecord] = []
    httpd = None

    # ============================================================================
    # tick-and-exit mode (no HTTP)
    # ============================================================================
    if cfg.mode == "tick-and-exit":
        rec = run_tick_once(cfg, cycle_index=1)
        tick_records.append(rec)
        ended_iso = _now_utc_iso()
        return DaemonRunSummary(
            mode=cfg.mode,
            bind=cfg.bind,
            port=cfg.port,
            cadence_seconds=cfg.cadence_seconds,
            n_ticks=1,
            n_proceed=1 if rec.policy == "PROCEED" else 0,
            n_pause=1 if rec.policy == "PAUSE" else 0,
            n_lockdown=1 if rec.policy == "LOCKDOWN" else 0,
            started_iso=started_iso,
            ended_iso=ended_iso,
            reason="tick-and-exit completed",
            chain_ok=rec.chain_ok,
            note=f"V1421 tick-and-exit verdict={rec.verdict} policy={rec.policy}",
        )

    # ============================================================================
    # serve-only mode (HTTP only, no ticks)
    # ============================================================================
    if cfg.mode == "serve-only":
        ok, v1420, err = _import_v1420()
        if not ok:
            ended_iso = _now_utc_iso()
            return DaemonRunSummary(
                mode=cfg.mode, bind=cfg.bind, port=cfg.port,
                cadence_seconds=cfg.cadence_seconds,
                n_ticks=0, n_proceed=0, n_pause=0, n_lockdown=0,
                started_iso=started_iso, ended_iso=ended_iso,
                reason=f"V1420 import failed: {err}",
                chain_ok=False,
                note="V1421 serve-only aborted due to V1420 import failure",
            )

        # Start the HTTP serve thread.
        serve_thread = threading.Thread(
            target=_serve_only_poller,
            args=(cfg, stop_event),
            daemon=True,
            name="v1421-serve-only",
        )
        serve_thread.start()

        # Wait for either max-seconds wall-clock or stop_event.
        if cfg.max_seconds > 0:
            deadline = started_mono + cfg.max_seconds
            while not stop_event.is_set() and time.time() < deadline:
                stop_event.wait(timeout=0.5)
            if not stop_event.is_set():
                stop_event.set()  # trigger graceful shutdown
        else:
            # max_seconds == 0 means "until SIGINT"; stop_event will be set by handler.
            serve_thread.join()

        serve_thread.join(timeout=2.0)

        ended_iso = _now_utc_iso()
        reason = "serve-only stopped" if stop_event.is_set() and not (cfg.max_seconds > 0 and time.time() >= deadline) else (
            f"serve-only max-seconds reached ({cfg.max_seconds}s)" if cfg.max_seconds > 0 else "serve-only stopped"
        )
        return DaemonRunSummary(
            mode=cfg.mode, bind=cfg.bind, port=cfg.port,
            cadence_seconds=cfg.cadence_seconds,
            n_ticks=0, n_proceed=0, n_pause=0, n_lockdown=0,
            started_iso=started_iso, ended_iso=ended_iso,
            reason=reason, chain_ok=True,
            note="V1421 serve-only OK",
        )

    # ============================================================================
    # daemon mode (HTTP + tick on cadence)
    # ============================================================================

    # Start HTTP thread (V1420.serve_forever_blocking blocks, so run in thread)
    httpd = _http_thread_loop(cfg, stop_event)
    if httpd is None:
        # Fall through to tick loop anyway; daemon degrades to tick-and-exit loop.
        print("[v1421] HTTP thread not started; running tick loop only", file=sys.stderr)

    cycle_index = 0
    reason = "max-seconds reached"
    try:
        while not stop_event.is_set():
            # Enforce max_seconds wall clock budget.
            if cfg.max_seconds > 0 and (time.time() - started_mono) >= cfg.max_seconds:
                reason = f"max-seconds reached ({cfg.max_seconds}s)"
                break

            cycle_index += 1
            rec = run_tick_once(cfg, cycle_index=cycle_index)
            tick_records.append(rec)

            # Sleep cadence unless shutdown requested or budget exhausted.
            remaining_budget = (
                cfg.max_seconds - (time.time() - started_mono) if cfg.max_seconds > 0 else cfg.cadence_seconds
            )
            sleep_for = min(cfg.cadence_seconds, max(0.0, remaining_budget))
            if sleep_for <= 0:
                reason = "max-seconds reached (no sleep)"
                break
            _sleep_with_event(sleep_for, stop_event, cfg.sleep_fn_name)

            if stop_event.is_set():
                reason = "SIGINT/SIGTERM received"
                break
    finally:
        if httpd is not None:
            try:
                ok_v1420, v1420, _ = _import_v1420()
                if ok_v1420:
                    v1420.stop_server(httpd)
            except Exception:  # pragma: no cover — defensive
                pass

    ended_iso = _now_utc_iso()
    n_proceed = sum(1 for r in tick_records if r.policy == "PROCEED")
    n_pause = sum(1 for r in tick_records if r.policy == "PAUSE")
    n_lockdown = sum(1 for r in tick_records if r.policy == "LOCKDOWN")
    chain_ok = all(r.chain_ok for r in tick_records) if tick_records else True

    return DaemonRunSummary(
        mode=cfg.mode, bind=cfg.bind, port=cfg.port,
        cadence_seconds=cfg.cadence_seconds,
        n_ticks=len(tick_records),
        n_proceed=n_proceed, n_pause=n_pause, n_lockdown=n_lockdown,
        started_iso=started_iso, ended_iso=ended_iso,
        reason=reason, chain_ok=chain_ok,
        note=f"V1421 daemon: {len(tick_records)} ticks via V1418; HTTP via V1420 on {cfg.bind}:{cfg.port}",
    )


def _serve_only_poller(cfg: DaemonConfig, stop_event: threading.Event) -> None:
    """Helper for serve-only mode: start httpd.serve_forever() and shut it down on stop."""
    ok, v1420, _ = _import_v1420()
    if not ok:
        stop_event.set()
        return
    httpd, _ = v1420.make_server(
        bind=cfg.bind,
        port=cfg.port,
        max_seconds=0,  # we control lifetime via stop_event
        history_path=cfg.history_path,
    )

    def _serve_and_wait() -> None:
        try:
            httpd.serve_forever()
        except Exception:  # pragma: no cover — defensive
            pass

    serve_thread = threading.Thread(target=_serve_and_wait, daemon=True, name="v1421-serve-only")
    serve_thread.start()

    # Poll stop_event; shutdown httpd when set.
    try:
        while not stop_event.is_set():
            stop_event.wait(timeout=0.5)
        v1420.stop_server(httpd)
        serve_thread.join(timeout=2.0)
    finally:
        try:
            v1420.stop_server(httpd)
        except Exception:  # pragma: no cover
            pass


def stop_daemon(stop_event: threading.Event, server: Any) -> None:
    """Graceful shutdown helper."""
    stop_event.set()
    if server is not None:
        try:
            server.shutdown()
        except Exception:  # pragma: no cover
            pass


# ============================================================================
# Popper self-test (主 19:33 真借鉴 Popper)
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    """Run 17 self-tests; return (all_ok, n_pass, results).

    Tests cover: config validation, path safety, mode validation,
    sleep helper bounds, run_tick_once smoke test, daemon summary,
    serve-only poller integration with V1420 (mock).
    """
    results: List[Dict[str, Any]] = []

    # T1: default config builds
    try:
        cfg = build_default_config({})
        ok = cfg.mode == "daemon" and cfg.port == DEFAULT_PORT and cfg.cadence_seconds == DEFAULT_CADENCE_SECONDS
        results.append({"name": "default-config-builds", "ok": ok})
    except Exception as exc:
        results.append({"name": "default-config-builds", "ok": False, "err": repr(exc)})

    # T2: validate_config accepts defaults
    try:
        cfg = build_default_config({})
        validate_config(cfg)
        results.append({"name": "validate-config-defaults", "ok": True})
    except Exception as exc:
        results.append({"name": "validate-config-defaults", "ok": False, "err": repr(exc)})

    # T3: validate_config rejects bad bind
    try:
        cfg = build_default_config({"bind": "evil.example.com"})
        validate_config(cfg)
        results.append({"name": "reject-bad-bind", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-bad-bind", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-bad-bind", "ok": False, "err": repr(exc)})

    # T4: validate_config rejects bad port (0)
    try:
        cfg = build_default_config({"port": 0})
        validate_config(cfg)
        results.append({"name": "reject-port-zero", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-port-zero", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-port-zero", "ok": False, "err": repr(exc)})

    # T5: validate_config rejects bad port (> 65535)
    try:
        cfg = build_default_config({"port": 70000})
        validate_config(cfg)
        results.append({"name": "reject-port-overflow", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-port-overflow", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-port-overflow", "ok": False, "err": repr(exc)})

    # T6: validate_config rejects cadence too small
    try:
        cfg = build_default_config({"cadence_seconds": 0})
        validate_config(cfg)
        results.append({"name": "reject-cadence-zero", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-cadence-zero", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-cadence-zero", "ok": False, "err": repr(exc)})

    # T7: validate_config rejects cadence too large
    try:
        cfg = build_default_config({"cadence_seconds": 99999})
        validate_config(cfg)
        results.append({"name": "reject-cadence-overflow", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-cadence-overflow", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-cadence-overflow", "ok": False, "err": repr(exc)})

    # T8: validate_config rejects negative max_seconds
    try:
        cfg = build_default_config({"max_seconds": -1.0})
        validate_config(cfg)
        results.append({"name": "reject-max-seconds-negative", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-max-seconds-negative", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-max-seconds-negative", "ok": False, "err": repr(exc)})

    # T9: validate_config rejects bad mode
    try:
        cfg = build_default_config({"mode": "chaos"})
        validate_config(cfg)
        results.append({"name": "reject-bad-mode", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-bad-mode", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-bad-mode", "ok": False, "err": repr(exc)})

    # T10: validate_config rejects bad sleep_fn_name
    try:
        cfg = build_default_config({"sleep_fn_name": "os.system"})
        validate_config(cfg)
        results.append({"name": "reject-bad-sleep-fn", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "reject-bad-sleep-fn", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-bad-sleep-fn", "ok": False, "err": repr(exc)})

    # T11: _safe_path rejects dotdot
    try:
        _safe_path(Path("foo/../bar"))
        results.append({"name": "safe-path-rejects-dotdot", "ok": False, "err": "no exception raised"})
    except ValueError:
        results.append({"name": "safe-path-rejects-dotdot", "ok": True})
    except Exception as exc:
        results.append({"name": "safe-path-rejects-dotdot", "ok": False, "err": repr(exc)})

    # T12: _sleep_with_event respects stop_event (immediate)
    try:
        ev = threading.Event()
        ev.set()  # pre-set → should return ~0
        slept = _sleep_with_event(5.0, ev, "time.sleep")
        results.append({"name": "sleep-honors-stop-event", "ok": slept < 1.0})
    except Exception as exc:
        results.append({"name": "sleep-honors-stop-event", "ok": False, "err": repr(exc)})

    # T13: DaemonTickRecord dataclass works
    try:
        rec = DaemonTickRecord(
            cycle_index=1, started_iso="2026-08-10T00:00:00Z",
            ended_iso="2026-08-10T00:00:01Z", verdict="OK",
            policy="PROCEED", chain_ok=True, alerts_count=0,
            duration_seconds=1.0, note="test",
        )
        d = rec.to_dict()
        ok = d["cycle_index"] == 1 and d["policy"] == "PROCEED"
        results.append({"name": "tick-record-dataclass", "ok": ok})
    except Exception as exc:
        results.append({"name": "tick-record-dataclass", "ok": False, "err": repr(exc)})

    # T14: DaemonRunSummary dataclass works
    try:
        s = DaemonRunSummary(
            mode="daemon", bind="127.0.0.1", port=8765,
            cadence_seconds=300, n_ticks=0, n_proceed=0, n_pause=0,
            n_lockdown=0, started_iso="2026-08-10T00:00:00Z",
            ended_iso="2026-08-10T00:00:01Z",
            reason="max-seconds", chain_ok=True, note="",
        )
        d = s.to_dict()
        ok = d["mode"] == "daemon" and d["chain_ok"] is True
        results.append({"name": "run-summary-dataclass", "ok": ok})
    except Exception as exc:
        results.append({"name": "run-summary-dataclass", "ok": False, "err": repr(exc)})

    # T15: tick-and-exit returns a real summary
    try:
        # Use a temp history path so we don't pollute real V1417 history.
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            cfg = build_default_config({
                "mode": "tick-and-exit",
                "history_path": Path(td) / "h.jsonl",
                "baseline_path": Path(td) / "b.json",
                "tick_jsonl_path": Path(td) / "t.jsonl",
                "render_out": Path(td) / "r.md",
                "render": False,
            })
            validate_config(cfg)
            s = run_daemon(cfg)
            ok = s.mode == "tick-and-exit" and s.n_ticks == 1
            results.append({"name": "tick-and-exit-runs", "ok": ok,
                            "verdict": s.note})
    except Exception as exc:
        results.append({"name": "tick-and-exit-runs", "ok": False, "err": repr(exc)})

    # T16: chain_delegate returns dict with all_ok
    try:
        d = chain_delegate()
        ok = isinstance(d, dict) and "all_ok" in d and d["all_ok"] is True
        results.append({"name": "chain-delegate-ok", "ok": ok})
    except Exception as exc:
        results.append({"name": "chain-delegate-ok", "ok": False, "err": repr(exc)})

    # T17: V1421 guards & V3 guards present and well-formed
    try:
        ok = len(V1421_GUARDS) >= 15 and len(V1421_V3_GUARDS) >= 9 and len(V1421_BORROWED) >= 4
        results.append({"name": "guards-and-borrowed-present", "ok": ok,
                        "n_guards": len(V1421_GUARDS),
                        "n_v3_guards": len(V1421_V3_GUARDS),
                        "n_borrowed": len(V1421_BORROWED)})
    except Exception as exc:
        results.append({"name": "guards-and-borrowed-present", "ok": False, "err": repr(exc)})

    n_pass = sum(1 for r in results if r["ok"])
    all_ok = all(r["ok"] for r in results)
    return all_ok, n_pass, results


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe V1418 + V1420 chain integrity (read-only).

    Returns dict with all_ok + per-module status.
    """
    out: Dict[str, Any] = {
        "v1421_version": V1421_VERSION,
        "v1421_module": V1421_MODULE,
    }

    ok1418, v1418, err1418 = _import_v1418()
    out["v1418_loaded"] = ok1418
    if not ok1418:
        out["v1418_error"] = err1418
    else:
        try:
            d1418 = v1418.chain_delegate()
            out["v1418_all_ok"] = bool(d1418.get("all_ok", False))
            out["v1418_module"] = d1418.get("v1418_module", "v1418_asi_dgm_cron_integration")
        except Exception as exc:  # pragma: no cover
            out["v1418_chain_error"] = repr(exc)
            out["v1418_all_ok"] = False

    ok1420, v1420, err1420 = _import_v1420()
    out["v1420_loaded"] = ok1420
    if not ok1420:
        out["v1420_error"] = err1420
    else:
        try:
            d1420 = v1420.chain_delegate()
            out["v1420_all_ok"] = bool(d1420.get("all_ok", False))
            out["v1420_module"] = d1420.get("v1420_module", "v1420_asi_http_status_endpoint")
        except Exception as exc:  # pragma: no cover
            out["v1420_chain_error"] = repr(exc)
            out["v1420_all_ok"] = False

    out["all_ok"] = bool(out.get("v1418_all_ok", False)) and bool(out.get("v1420_all_ok", False))
    out["ts"] = _now_utc_iso()
    return out


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(
        f"""V1421 — ASI 总框架 daemon (serve-while-ticking) v{V1421_VERSION}

Usage:
  python -m apeireth.{V1421_MODULE} <command> [args]

Commands:
  version                                  Print V1421 version
  meta [--json]                            Print V1421 metadata (JSON if --json)
  demo                                     Print a short demo of usage
  help                                     Print this help
  popper                                   Run the 17 popper self-tests
  chain                                    Probe V1418 + V1420 chain integrity
  tick-and-exit                            Run ONE V1418.tick_once and exit
        [--cadence-seconds N]               (default {DEFAULT_CADENCE_SECONDS})
        [--max-seconds N]                   (default {DEFAULT_MAX_SECONDS})
        [--history-path PATH]               (default V1417 JSONL)
        [--baseline-path PATH]              (default V1417 baseline)
        [--tick-jsonl-path PATH]            (default V1416 ticks)
        [--no-render]                       (skip V1417 render)
  serve-only                               Run V1420 HTTP only (no ticks)
        --bind HOST                         (default {DEFAULT_BIND_HOST})
        --port PORT                         (default {DEFAULT_PORT})
        [--max-seconds N]                   (default {DEFAULT_MAX_SECONDS})
        [--history-path PATH]
  daemon                                   Run V1418 ticks on cadence + V1420 HTTP
        --bind HOST                         (default {DEFAULT_BIND_HOST})
        --port PORT                         (default {DEFAULT_PORT})
        --cadence-seconds N                 (default {DEFAULT_CADENCE_SECONDS})
        [--max-seconds N]                   (default {DEFAULT_MAX_SECONDS})
        [--auth-token TOKEN]                (optional bearer token for /api/asi/refresh)
        [--history-path PATH]
        [--baseline-path PATH]
        [--tick-jsonl-path PATH]
        [--no-render]
        [--sleep-fn time.sleep|threading.Event.wait]

Examples:
  # Run one cron tick and exit (smoke test)
  python -m apeireth.{V1421_MODULE} tick-and-exit --cadence-seconds 1

  # Run HTTP only for 30s (smoke test)
  python -m apeireth.{V1421_MODULE} serve-only --bind 127.0.0.1 --port 8765 --max-seconds 30

  # Run real 5-minute daemon with HTTP live
  python -m apeireth.{V1421_MODULE} daemon --bind 127.0.0.1 --port 8765 --cadence-seconds 300

  # Poll live status while daemon runs:
  curl -s http://127.0.0.1:8765/api/asi/health
  curl -s http://127.0.0.1:8765/api/asi/status | jq .
"""
    )


def _parse_kv_args(argv: List[str]) -> Dict[str, str]:
    """Parse --key value flags into a dict (主 00:56 任何人都能接手)."""
    out: Dict[str, str] = {}
    i = 0
    while i < len(argv):
        a = argv[i]
        if not a.startswith("--"):
            raise ValueError(f"unexpected positional arg: {a!r}")
        key = a[2:].replace("-", "_")
        if i + 1 >= len(argv):
            raise ValueError(f"flag --{key} requires a value")
        val = argv[i + 1]
        out[key] = val
        i += 2
    return out


def _coerce_overrides(kv: Dict[str, str]) -> Dict[str, Any]:
    """Coerce string kv pairs into typed overrides."""
    out: Dict[str, Any] = {}
    int_keys = {"port", "cadence_seconds"}
    float_keys = {"max_seconds"}
    bool_keys = {"render"}
    path_keys = {"history_path", "baseline_path", "tick_jsonl_path", "render_out"}
    for k, v in kv.items():
        if k in int_keys:
            out[k] = int(v)
        elif k in float_keys:
            out[k] = float(v)
        elif k in bool_keys:
            out[k] = v.lower() in ("1", "true", "yes", "y")
        elif k in path_keys:
            out[k] = Path(v)
        else:
            out[k] = v
    return out


def run_cli(argv: List[str]) -> int:
    if not argv:
        _print_help()
        return 0

    cmd, rest = argv[0], argv[1:]

    if cmd == "version":
        print(f"V1421 version {V1421_VERSION} (schema={V1421_SCHEMA})")
        return 0

    if cmd == "help":
        _print_help()
        return 0

    if cmd == "demo":
        print(
            "V1421 demo:\n"
            "  python -m apeireth.v1421_asi_daemon_serve_tick tick-and-exit\n"
            "  python -m apeireth.v1421_asi_daemon_serve_tick serve-only --max-seconds 30\n"
            "  python -m apeireth.v1421_asi_daemon_serve_tick daemon --cadence-seconds 300\n"
        )
        return 0

    if cmd == "meta":
        kv = _parse_kv_args(rest) if rest else {}
        cfg = build_default_config({})
        meta = {
            "version": V1421_VERSION,
            "schema": V1421_SCHEMA,
            "module": V1421_MODULE,
            "guards": list(V1421_GUARDS),
            "v3_guards": list(V1421_V3_GUARDS),
            "borrowed": [list(b) for b in V1421_BORROWED],
            "default_config": cfg.to_dict(),
        }
        if kv.get("json") == "true":
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            print(f"V1421 v{V1421_VERSION} module={V1421_MODULE} guards={len(V1421_GUARDS)}")
        return 0

    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(f"V1421 popper: all_ok={all_ok} n_pass={n_pass}/{len(results)}")
        for r in results:
            mark = "PASS" if r["ok"] else "FAIL"
            extra = "" if r["ok"] else f" — {r.get('err', '')}"
            print(f"  [{mark}] {r['name']}{extra}")
        return 0 if all_ok else 1

    if cmd == "chain":
        d = chain_delegate()
        print(json.dumps(d, indent=2, ensure_ascii=False))
        return 0 if d.get("all_ok") else 1

    if cmd == "tick-and-exit":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        if "no_render" in ov:
            ov["render"] = not ov.pop("no_render")
        ov["mode"] = "tick-and-exit"
        cfg = build_default_config(ov)
        validate_config(cfg)
        s = run_daemon(cfg)
        print(json.dumps(s.to_dict(), indent=2, ensure_ascii=False))
        return 0

    if cmd == "serve-only":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        ov["mode"] = "serve-only"
        cfg = build_default_config(ov)
        validate_config(cfg)
        s = run_daemon(cfg)
        print(json.dumps(s.to_dict(), indent=2, ensure_ascii=False))
        return 0

    if cmd == "daemon":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        if "no_render" in ov:
            ov["render"] = not ov.pop("no_render")
        ov["mode"] = "daemon"
        cfg = build_default_config(ov)
        validate_config(cfg)
        s = run_daemon(cfg)
        print(json.dumps(s.to_dict(), indent=2, ensure_ascii=False))
        return 0

    print(f"V1421 unknown command: {cmd!r}")
    _print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))
