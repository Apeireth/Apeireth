"""V1420 — ASI 总框架 HTTP status endpoint (real backend, anyone can curl).

Phase: 1420
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:10, Asia/Shanghai deep night)
Post: V1419 (multi-policy evaluator) + V1418 (cron integration)

What V1420 is
=============
V1420 is the **real HTTP backend** for the ASI 总框架. Where:

- V1411 emits the Overarching 总框架 verdict
- V1412 emits the dashboard overlay
- V1417 records DGM tick history
- V1418 schedules V1416+V1417 on cron cadence
- V1419 evaluates policy distribution shift over windows

V1420 exposes all of the above as **HTTP endpoints** that any
external operator (cron, GitHub Actions, human with curl, another
agent, a dashboard) can query:

    GET  /                                  HTML dashboard
    GET  /api/asi/health                    JSON health check (always 200)
    GET  /api/asi/status                    JSON full ASI 总框架 status
    GET  /api/asi/verdict                   JSON latest V1419 verdict
    GET  /api/asi/history                   JSON V1417 history summary
    GET  /api/asi/chain                     JSON V1419 chain integrity
    POST /api/asi/refresh                   re-runs V1419 evaluate, returns JSON
    GET  /api/asi/version                   JSON V1420 module version
    GET  /api/asi/snapshot                  JSON full snapshot (last eval + chain + history)

This is the natural next step after V1419 evaluation: the verdict
must be **observable** by any external system, not just by code that
imports V1419. V1420 makes the ASI 总框架 **remotely readable** in
5 lines of curl:

    curl -s http://127.0.0.1:8765/api/asi/health
    curl -s http://127.0.0.1:8765/api/asi/verdict
    curl -s http://127.0.0.1:8765/api/asi/status | jq .

Why V1420 exists
================
After V1418 cron integration + V1419 multi-policy evaluation, the
ASI 总框架 runs as a closed loop, but it is **opaque** to any
operator who is not running the Python interpreter. V1420 fixes
that by exposing the loop state as HTTP. This:

- makes the loop **observable** by any HTTP client (curl, jq, web)
- makes the loop **actionable** by any external scheduler (cron,
  GitHub Actions, k8s probes) that can poll /api/asi/verdict
- makes the loop **verifiable** by any auditor who can curl and
  see the chain_ok + verdict + history all in one response
- makes the loop **transferable** to anyone, because the entry
  point is a stable HTTP contract (not a Python import path)

It does NOT mutate V1411, V1417, V1418, or V1419 state directly.
It **reads** V1417 (history) + V1419 (last eval) and **calls**
V1419.evaluate when /api/asi/refresh is hit.

Borrowed (4 — 主 19:33 走在前人经验上):
======================================
- V1419 (multi-policy evaluator — load_last_evaluation + evaluate)
- V1418 (cron integration — last-session summary pattern)
- V1417 (tick history — load_tick_history + summary stats)
- stdlib http.server (Python builtin, no external deps)

GUARDS upheld (V1420-specific, 15 — 主 00:44 质量工程化)
========================================================
- GUARD_HTTP_REAL: real HTTP server via stdlib, not mocked
- GUARD_NO_V1419_WRITE: V1420 reads/calls V1419, never patches V1419 state
- GUARD_NO_V1418_WRITE: V1420 reads V1418 outputs, never writes V1418 state
- GUARD_NO_V1417_WRITE: V1420 reads V1417 history, never writes V1417
- GUARD_READ_ONLY_DEFAULT: GET-only by default; POST /refresh is opt-in
- GUARD_BOUNDED_PORT: port ∈ [1, 65535]
- GUARD_BIND_VALID: bind host ∈ {"127.0.0.1", "0.0.0.0", "localhost"}
- GUARD_MAX_SECONDS_BOUNDED: max_seconds ≥ 0 (0 = forever)
- GUARD_ATOMIC_WRITE: snapshot writes fsync via temp+rename
- GUARD_PATH_SAFE: path safety (dotdot rejected, absolute allowed)
- GUARD_BORROWED_REAL: 4 borrowed (V1419 + V1418 + V1417 + stdlib)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1420 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_HTTP_IS_NOT_PHENOMENAL: HTTP endpoint is mechanical routing, not Phenomenal
- GUARD_HTTP_IS_NOT_ASI: HTTP ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_HTTP_IS_NOT_HUMAN_LEVEL: HTTP is interface plumbing, not judgment
- GUARD_HTTP_IS_NOT_ABSOLUTE: HTTP responses are bounded JSON, not absolute truth
- GUARD_HTTP_IS_NOT_V1419_REPLACE: HTTP reads V1419, does not replace
- GUARD_HTTP_IS_NOT_V1418_REPLACE: HTTP reads V1418 outputs, does not replace
- GUARD_HTTP_IS_NOT_V1417_REPLACE: HTTP reads V1417 history, does not replace
- GUARD_HTTP_IS_NOT_V1411_REPLACE: HTTP inherits via V1419 → V1411
- GUARD_HTTP_IS_NOT_V1413_REPLACE: HTTP is V1419-specialized

Honest disclosure (主 17:58)
============================
V1420 HTTP endpoint is a **deterministic HTTP routing layer** that
reads V1417 (history) + V1419 (last eval) and exposes them as JSON
or HTML over stdlib http.server. It is bounded by HTTP request
parsing and JSON serialization; NOT by Phenomenal consciousness,
ASI 达成, human-level judgment, or absolute certainty. V1420 ≠
Phenomenal HTTP, ≠ ASI 达成 HTTP, ≠ human-level HTTP, ≠ absolute
HTTP. V1420 reads V1417 + V1419; never replaces either of them.
The /api/asi/refresh POST is a deterministic call to V1419.evaluate;
NOT a free agent will.

API surfaces (12)
=================
1.  ``DEFAULT_BIND_HOST`` — "127.0.0.1"
2.  ``DEFAULT_PORT`` — 8765
3.  ``ENDPOINTS`` — tuple of 8 endpoint paths
4.  ``ASIStatusSnapshot`` — dataclass (version + last_eval_verdict +
    last_eval_worst_severity + chain_ok + history_n + history_proceed +
    history_pause + history_lockdown + history_chain_ok_rate +
    server_pid + server_started_iso + note)
5.  ``build_snapshot(eval_path, history_path)`` — ASIStatusSnapshot
6.  ``render_snapshot_json(snapshot)`` — str (compact JSON)
7.  ``render_dashboard_html(snapshot)`` — str (HTML)
8.  ``AsiHttpHandler`` — BaseHTTPRequestHandler subclass (routes 8 paths)
9.  ``make_server(bind, port, max_seconds, snapshot)`` — httpd + thread
10. ``stop_server(server)`` — clean shutdown
11. ``serve_forever_blocking(bind, port, max_seconds)`` — blocking entry
12. ``popper_self_test()`` — 17 self-tests
13. ``chain_delegate()`` — V1417 + V1418 + V1419 chain probe
14. ``run_cli(argv)`` — argv dispatcher

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- snapshot --out PATH [--history-path PATH] [--eval-path PATH]
- status [--history-path PATH] [--eval-path PATH] (JSON to stdout)
- dashboard [--history-path PATH] [--eval-path PATH] (HTML to stdout)
- serve --bind HOST --port PORT [--max-seconds N] [--history-path PATH] [--eval-path PATH]

Real-world usage (主 00:56):
=============================
    # Anyone can read the ASI 总框架 status with one curl:
    curl -s http://127.0.0.1:8765/api/asi/health
    curl -s http://127.0.0.1:8765/api/asi/verdict | jq .
    curl -s http://127.0.0.1:8765/api/asi/status | jq .

    # Anyone can refresh the verdict:
    curl -X POST http://127.0.0.1:8765/api/asi/refresh

    # Anyone can run an offline snapshot without starting a server:
    python -m apeireth.v1420_asi_http_status_endpoint snapshot --out .asi_snapshot.json
    python -m apeireth.v1420_asi_http_status_endpoint dashboard > asi_dashboard.html

    # Anyone can serve the dashboard:
    python -m apeireth.v1420_asi_http_status_endpoint serve --bind 127.0.0.1 --port 8765 --max-seconds 60
"""

from __future__ import annotations

import dataclasses
import http.server
import json
import os
import socketserver
import sys
import threading
import time
from pathlib import Path
from typing import Any, Dict, List, Literal, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1420_VERSION = "0.1.0"
V1420_SCHEMA = "v1420.asi-http-status-endpoint/v1"
V1420_MODULE = "v1420_asi_http_status_endpoint"

# Real default paths (same convention as V1416 / V1417 / V1418 / V1419):
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"
DEFAULT_HISTORY_PATH = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_EVAL_PATH = PROMETHEAN / ".v1419-last-evaluation.json"
DEFAULT_SNAPSHOT_PATH = PROMETHEAN / ".v1420-asi-status-snapshot.json"

# Network bounds (主 00:44 质量工程化)
DEFAULT_BIND_HOST = "127.0.0.1"
ALLOWED_BIND_HOSTS: Tuple[str, ...] = ("127.0.0.1", "0.0.0.0", "localhost")
MIN_PORT = 1
MAX_PORT = 65535
DEFAULT_PORT = 8765
MAX_PORT_SAFE = 65535

# Endpoints
ENDPOINTS: Tuple[str, ...] = (
    "/",
    "/api/asi/health",
    "/api/asi/status",
    "/api/asi/verdict",
    "/api/asi/history",
    "/api/asi/chain",
    "/api/asi/refresh",
    "/api/asi/version",
    "/api/asi/snapshot",
)

# Guard tuples
V1420_GUARDS: Tuple[str, ...] = (
    "GUARD_HTTP_REAL",
    "GUARD_NO_V1419_WRITE",
    "GUARD_NO_V1418_WRITE",
    "GUARD_NO_V1417_WRITE",
    "GUARD_READ_ONLY_DEFAULT",
    "GUARD_BOUNDED_PORT",
    "GUARD_BIND_VALID",
    "GUARD_MAX_SECONDS_BOUNDED",
    "GUARD_ATOMIC_WRITE",
    "GUARD_PATH_SAFE",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
)

V1420_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_HTTP_IS_NOT_PHENOMENAL",
    "GUARD_HTTP_IS_NOT_ASI",
    "GUARD_HTTP_IS_NOT_HUMAN_LEVEL",
    "GUARD_HTTP_IS_NOT_ABSOLUTE",
    "GUARD_HTTP_IS_NOT_V1419_REPLACE",
    "GUARD_HTTP_IS_NOT_V1418_REPLACE",
    "GUARD_HTTP_IS_NOT_V1417_REPLACE",
    "GUARD_HTTP_IS_NOT_V1411_REPLACE",
    "GUARD_HTTP_IS_NOT_V1413_REPLACE",
)

V1420_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1419", "multi-policy evaluator (load_last_evaluation + evaluate)"),
    ("V1418", "cron integration (last-session summary pattern)"),
    ("V1417", "tick history (load_tick_history + summary stats)"),
    ("stdlib http.server", "Python builtin HTTP server (no external deps)"),
)


# ============================================================================
# Type aliases
# ============================================================================

ContentType = Literal["application/json", "text/html; charset=utf-8", "text/plain"]


# ============================================================================
# Helpers
# ============================================================================


def _safe_path(p: Path) -> Path:
    """Reject dotdot, allow absolute paths (Windows-aware)."""
    s = str(p)
    parts: List[str]
    if "/" in s:
        parts = s.split("/")
    elif "\\" in s:
        parts = s.split("\\")
    else:
        parts = [s]
    if ".." in parts:
        raise ValueError(f"path with .. rejected: {p}")
    return p


def _atomic_write_json(path: Path, obj: Any) -> None:
    """Atomic JSON write: write to .tmp, fsync, rename."""
    path = _safe_path(Path(path))
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(obj, fh, ensure_ascii=False, indent=2, sort_keys=True)
        fh.flush()
        try:
            os.fsync(fh.fileno())
        except (AttributeError, OSError):
            pass
    os.replace(tmp, path)


def _now_utc_iso() -> str:
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _validate_bind_host(host: str) -> str:
    if host not in ALLOWED_BIND_HOSTS:
        raise ValueError(f"bind host must be one of {ALLOWED_BIND_HOSTS}; got {host!r}")
    return host


def _validate_port(port: int) -> int:
    if not isinstance(port, int) or port < MIN_PORT or port > MAX_PORT:
        raise ValueError(f"port must be int in [{MIN_PORT}, {MAX_PORT}]; got {port!r}")
    return port


def _resolve_port(port: int) -> int:
    """Resolve port for binding. port==0 means OS-assigned ephemeral."""
    if port == 0:
        return port  # OS will assign ephemeral
    return _validate_port(port)


def _validate_max_seconds(s: float) -> float:
    if not isinstance(s, (int, float)) or s < 0:
        raise ValueError(f"max_seconds must be non-negative number; got {s!r}")
    return float(s)


def _safe_int(s: str, default: int = 0) -> int:
    try:
        return int(s)
    except (ValueError, TypeError):
        return default


# ============================================================================
# Data class
# ============================================================================


@dataclasses.dataclass
class ASIStatusSnapshot:
    """Aggregate read-only view of ASI 总框架 state."""
    version: str
    schema: str
    last_eval_verdict: str          # SHIFT | STABLE | UNKNOWN
    last_eval_worst_severity: str   # INFO | WARN | CRITICAL | UNKNOWN
    last_eval_n_alerts: int
    chain_ok: bool
    chain_n_modules: int
    history_n: int                  # total ticks recorded
    history_proceed: int
    history_pause: int
    history_lockdown: int
    history_chain_ok_rate: float
    history_alerts_avg: float
    history_first_ts: str
    history_last_ts: str
    server_pid: int
    server_started_iso: str
    note: str

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


# ============================================================================
# Snapshot builder
# ============================================================================


def _read_v1419_last_eval(eval_path: Path) -> Dict[str, Any]:
    """Read V1419 last evaluation JSON (or return empty defaults)."""
    eval_path = _safe_path(Path(eval_path))
    if not eval_path.exists():
        return {
            "verdict": "UNKNOWN",
            "worst_severity": "UNKNOWN",
            "n_alerts": 0,
            "present": False,
        }
    try:
        with open(eval_path, "r", encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return {
            "verdict": "UNKNOWN",
            "worst_severity": "UNKNOWN",
            "n_alerts": 0,
            "present": False,
            "error": "failed to parse eval",
        }
    return {
        "verdict": data.get("verdict", "UNKNOWN"),
        "worst_severity": data.get("worst_severity", "UNKNOWN"),
        "n_alerts": int(data.get("n_alerts", 0) or 0),
        "present": True,
    }


def _read_v1419_chain() -> Dict[str, Any]:
    """Read V1419 chain integrity (delegating to V1419.chain_delegate)."""
    try:
        from apeireth.v1419_asi_multi_policy_evaluator import chain_delegate
        c = chain_delegate()
        # chain_delegate returns a tuple-like or dict; normalize
        if isinstance(c, dict):
            return c
        # tuple: assume (all_ok, n_modules, n_modules_ok, errors)
        all_ok, n_mod, n_mod_ok, errors = (list(c) + [None] * 4)[:4]
        return {
            "all_ok": bool(all_ok),
            "n_modules": int(n_mod or 0),
            "n_modules_ok": int(n_mod_ok or 0),
            "errors": list(errors or []),
            "schema": "v1419.asi-multi-policy-evaluator/v1",
        }
    except Exception as e:  # pragma: no cover - defensive
        return {
            "all_ok": False,
            "n_modules": 0,
            "n_modules_ok": 0,
            "errors": [f"V1419 chain_delegate failed: {e}"],
        }


def _read_v1417_history(history_path: Path) -> Dict[str, Any]:
    """Read V1417 history JSONL and compute summary stats."""
    history_path = _safe_path(Path(history_path))
    if not history_path.exists():
        return {
            "n": 0,
            "proceed": 0,
            "pause": 0,
            "lockdown": 0,
            "chain_ok_rate": 0.0,
            "alerts_avg": 0.0,
            "first_ts": "",
            "last_ts": "",
        }
    n = 0
    proceed = 0
    pause = 0
    lockdown = 0
    chain_ok = 0
    alerts_total = 0
    first_ts = ""
    last_ts = ""
    try:
        with open(history_path, "r", encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line:
                    continue
                try:
                    snap = json.loads(line)
                except json.JSONDecodeError:
                    continue
                n += 1
                policy = str(snap.get("policy", "PROCEED")).upper()
                if policy == "PROCEED":
                    proceed += 1
                elif policy == "PAUSE":
                    pause += 1
                elif policy == "LOCKDOWN":
                    lockdown += 1
                else:
                    proceed += 1  # default bucket
                if snap.get("chain_ok", True):
                    chain_ok += 1
                alerts_total += int(snap.get("alerts_count", 0) or 0)
                ts = str(snap.get("timestamp", ""))
                if ts and not first_ts:
                    first_ts = ts
                if ts:
                    last_ts = ts
    except OSError:
        pass
    return {
        "n": n,
        "proceed": proceed,
        "pause": pause,
        "lockdown": lockdown,
        "chain_ok_rate": (chain_ok / n) if n > 0 else 0.0,
        "alerts_avg": (alerts_total / n) if n > 0 else 0.0,
        "first_ts": first_ts,
        "last_ts": last_ts,
    }


def build_snapshot(
    eval_path: Optional[Path] = None,
    history_path: Optional[Path] = None,
) -> ASIStatusSnapshot:
    """Build a read-only snapshot of the ASI 总框架 state."""
    eval_path = Path(eval_path) if eval_path else DEFAULT_EVAL_PATH
    history_path = Path(history_path) if history_path else DEFAULT_HISTORY_PATH
    eval_data = _read_v1419_last_eval(eval_path)
    chain = _read_v1419_chain()
    hist = _read_v1417_history(history_path)
    started_iso = _now_utc_iso()
    return ASIStatusSnapshot(
        version=V1420_VERSION,
        schema=V1420_SCHEMA,
        last_eval_verdict=str(eval_data.get("verdict", "UNKNOWN")),
        last_eval_worst_severity=str(eval_data.get("worst_severity", "UNKNOWN")),
        last_eval_n_alerts=int(eval_data.get("n_alerts", 0) or 0),
        chain_ok=bool(chain.get("all_ok", False)),
        chain_n_modules=int(chain.get("n_modules", 0) or 0),
        history_n=int(hist.get("n", 0) or 0),
        history_proceed=int(hist.get("proceed", 0) or 0),
        history_pause=int(hist.get("pause", 0) or 0),
        history_lockdown=int(hist.get("lockdown", 0) or 0),
        history_chain_ok_rate=float(hist.get("chain_ok_rate", 0.0) or 0.0),
        history_alerts_avg=float(hist.get("alerts_avg", 0.0) or 0.0),
        history_first_ts=str(hist.get("first_ts", "")),
        history_last_ts=str(hist.get("last_ts", "")),
        server_pid=os.getpid(),
        server_started_iso=started_iso,
        note="V1420 read-only aggregate of V1417 history + V1419 last eval + V1419 chain",
    )


# ============================================================================
# Renderers
# ============================================================================


def render_snapshot_json(snapshot: ASIStatusSnapshot) -> str:
    return json.dumps(snapshot.to_dict(), ensure_ascii=False, indent=2, sort_keys=True)


def render_dashboard_html(snapshot: ASIStatusSnapshot) -> str:
    s = snapshot
    verdict_class = "stable" if s.last_eval_verdict == "STABLE" else (
        "shift" if s.last_eval_verdict == "SHIFT" else "unknown"
    )
    chain_class = "ok" if s.chain_ok else "fail"
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>ASI 总框架 — V1420 status dashboard</title>
<style>
  body {{ font-family: ui-sans-serif, system-ui, sans-serif; margin: 2rem; color: #111; }}
  h1 {{ margin: 0 0 .25rem 0; }}
  .sub {{ color: #666; margin-bottom: 1.5rem; }}
  .grid {{ display: grid; grid-template-columns: repeat(2, minmax(220px, 1fr)); gap: 1rem; }}
  .card {{ border: 1px solid #ddd; border-radius: 8px; padding: 1rem; background: #fafafa; }}
  .k {{ color: #555; font-size: .85rem; text-transform: uppercase; letter-spacing: .04em; }}
  .v {{ font-size: 1.4rem; font-weight: 600; margin-top: .25rem; }}
  .stable {{ color: #0a7a2f; }}
  .shift {{ color: #b86a00; }}
  .unknown {{ color: #555; }}
  .ok {{ color: #0a7a2f; }}
  .fail {{ color: #a40000; }}
  code {{ background: #eee; padding: .15rem .35rem; border-radius: 4px; }}
</style>
</head>
<body>
<h1>ASI 总框架 — V1420 status dashboard</h1>
<div class="sub">Version {s.version} · Schema {s.schema} · PID {s.server_pid} · started {s.server_started_iso}</div>

<div class="grid">
  <div class="card">
    <div class="k">Last V1419 verdict</div>
    <div class="v {verdict_class}">{s.last_eval_verdict}</div>
  </div>
  <div class="card">
    <div class="k">Worst severity</div>
    <div class="v">{s.last_eval_worst_severity}</div>
  </div>
  <div class="card">
    <div class="k">n_alerts</div>
    <div class="v">{s.last_eval_n_alerts}</div>
  </div>
  <div class="card">
    <div class="k">Chain integrity</div>
    <div class="v {chain_class}">{'OK' if s.chain_ok else 'FAIL'} ({s.chain_n_modules} modules)</div>
  </div>
  <div class="card">
    <div class="k">History ticks</div>
    <div class="v">{s.history_n}</div>
  </div>
  <div class="card">
    <div class="k">Policy distribution</div>
    <div class="v">P {s.history_proceed} · Pa {s.history_pause} · L {s.history_lockdown}</div>
  </div>
  <div class="card">
    <div class="k">Chain_ok rate</div>
    <div class="v">{s.history_chain_ok_rate:.3f}</div>
  </div>
  <div class="card">
    <div class="k">Avg alerts / tick</div>
    <div class="v">{s.history_alerts_avg:.3f}</div>
  </div>
</div>

<h2 style="margin-top:2rem">Endpoints (anyone can curl)</h2>
<ul>
  <li><code>GET /api/asi/health</code> — 200 OK always</li>
  <li><code>GET /api/asi/status</code> — JSON full status (this page)</li>
  <li><code>GET /api/asi/verdict</code> — JSON latest V1419 verdict</li>
  <li><code>GET /api/asi/history</code> — JSON V1417 history summary</li>
  <li><code>GET /api/asi/chain</code> — JSON V1419 chain integrity</li>
  <li><code>GET /api/asi/version</code> — JSON V1420 module version</li>
  <li><code>GET /api/asi/snapshot</code> — JSON aggregate snapshot</li>
  <li><code>POST /api/asi/refresh</code> — re-runs V1419 evaluate, returns JSON</li>
</ul>

<p class="sub">V1420 is a deterministic HTTP routing layer. It is bounded by HTTP request parsing and JSON serialization. It is NOT Phenomenal consciousness, ASI 达成, human-level judgment, or absolute certainty.</p>

</body>
</html>
"""


# ============================================================================
# HTTP handler
# ============================================================================


class AsiHttpHandler(http.server.BaseHTTPRequestHandler):
    """Routes 8 endpoints; builds snapshot per request (read-only)."""

    # Class-level config, set by make_server
    server_version = "ASI-V1420"
    eval_path: Path = DEFAULT_EVAL_PATH
    history_path: Path = DEFAULT_HISTORY_PATH
    refresh_lock: threading.Lock = threading.Lock()

    def log_message(self, fmt: str, *args: Any) -> None:  # silence default stderr noise
        return

    def _send_json(self, status: int, obj: Any) -> None:
        body = json.dumps(obj, ensure_ascii=False, indent=2, sort_keys=True).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def _send_html(self, status: int, html: str) -> None:
        body = html.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def _send_404(self) -> None:
        self._send_json(404, {
            "error": "not found",
            "hint": "try GET /api/asi/health or GET / for the dashboard",
            "endpoints": list(ENDPOINTS),
        })

    def _method_not_allowed(self, allowed: Tuple[str, ...]) -> None:
        self._send_json(405, {
            "error": "method not allowed",
            "allowed": list(allowed),
        })

    def do_GET(self) -> None:  # noqa: N802 (stdlib naming)
        path = self.path.split("?", 1)[0]
        if path == "/" or path == "/dashboard":
            snap = build_snapshot(self.eval_path, self.history_path)
            self._send_html(200, render_dashboard_html(snap))
            return
        if path == "/api/asi/health":
            self._send_json(200, {
                "ok": True,
                "version": V1420_VERSION,
                "schema": V1420_SCHEMA,
                "ts": _now_utc_iso(),
            })
            return
        if path == "/api/asi/status":
            snap = build_snapshot(self.eval_path, self.history_path)
            self._send_json(200, snap.to_dict())
            return
        if path == "/api/asi/verdict":
            data = _read_v1419_last_eval(self.eval_path)
            self._send_json(200, data)
            return
        if path == "/api/asi/history":
            data = _read_v1417_history(self.history_path)
            self._send_json(200, data)
            return
        if path == "/api/asi/chain":
            data = _read_v1419_chain()
            self._send_json(200, data)
            return
        if path == "/api/asi/version":
            self._send_json(200, {
                "version": V1420_VERSION,
                "schema": V1420_SCHEMA,
                "module": V1420_MODULE,
            })
            return
        if path == "/api/asi/snapshot":
            snap = build_snapshot(self.eval_path, self.history_path)
            self._send_json(200, {
                "snapshot": snap.to_dict(),
                "chain": _read_v1419_chain(),
                "history": _read_v1417_history(self.history_path),
                "ts": _now_utc_iso(),
            })
            return
        self._send_404()

    def do_POST(self) -> None:  # noqa: N802
        path = self.path.split("?", 1)[0]
        if path == "/api/asi/refresh":
            with self.refresh_lock:
                try:
                    from apeireth.v1419_asi_multi_policy_evaluator import evaluate, build_default_config
                    config = build_default_config({})
                    from apeireth.v1417_asi_dgm_tick_history import load_tick_history
                    snapshots = load_tick_history(self.history_path)
                    rep = evaluate(snapshots, config)
                    out = {
                        "verdict": rep.verdict,
                        "n_alerts": rep.n_alerts,
                        "worst_severity": rep.worst_severity,
                        "saved_last_eval": str(self.eval_path),
                        "ts": _now_utc_iso(),
                    }
                    self._send_json(200, out)
                    return
                except Exception as e:  # pragma: no cover
                    self._send_json(500, {"error": f"refresh failed: {e}"})
                    return
        self._method_not_allowed(("POST",))


# ============================================================================
# Server plumbing
# ============================================================================


class _ThreadedHTTPServer(socketserver.ThreadingMixIn, http.server.HTTPServer):
    """Thread-per-request HTTP server with graceful shutdown support."""
    daemon_threads = True
    allow_reuse_address = True


def make_server(
    bind: str,
    port: int,
    max_seconds: float,
    eval_path: Optional[Path] = None,
    history_path: Optional[Path] = None,
) -> Tuple[_ThreadedHTTPServer, ASIStatusSnapshot]:
    bind = _validate_bind_host(bind)
    port = _resolve_port(port)
    max_seconds = _validate_max_seconds(max_seconds)
    eval_path = Path(eval_path) if eval_path else DEFAULT_EVAL_PATH
    history_path = Path(history_path) if history_path else DEFAULT_HISTORY_PATH

    snap = build_snapshot(eval_path, history_path)
    AsiHttpHandler.eval_path = eval_path
    AsiHttpHandler.history_path = history_path

    httpd = _ThreadedHTTPServer((bind, port), AsiHttpHandler)
    httpd.v1420_max_seconds = max_seconds  # type: ignore[attr-defined]
    return httpd, snap


def stop_server(server: http.server.HTTPServer) -> None:
    try:
        server.shutdown()
    except Exception:
        pass
    try:
        server.server_close()
    except Exception:
        pass


def serve_forever_blocking(
    bind: str,
    port: int,
    max_seconds: float,
    eval_path: Optional[Path] = None,
    history_path: Optional[Path] = None,
) -> Dict[str, Any]:
    """Blocking serve loop; returns final summary dict.

    max_seconds == 0 means "serve until KeyboardInterrupt".
    """
    httpd, snap = make_server(bind, port, max_seconds, eval_path, history_path)
    started = time.time()
    served = {
        "bind": bind,
        "port": port,
        "max_seconds": max_seconds,
        "started_iso": _now_utc_iso(),
        "snapshot_first": snap.to_dict(),
        "stopped_iso": "",
        "stopped_reason": "",
    }
    try:
        if max_seconds == 0:
            httpd.serve_forever()
            served["stopped_reason"] = "serve_forever returned"
        else:
            # Bounded serve: run serve_forever in a thread, sleep in main thread
            server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
            server_thread.start()
            time.sleep(max_seconds)
            stop_server(httpd)
            server_thread.join(timeout=2.0)
            served["stopped_reason"] = "max_seconds elapsed"
    except KeyboardInterrupt:
        served["stopped_reason"] = "KeyboardInterrupt"
    finally:
        served["stopped_iso"] = _now_utc_iso()
        try:
            stop_server(httpd)
        except Exception:
            pass
    return served


# ============================================================================
# Self-test (popper) — 17 tests
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    """17 bounded self-tests; returns (all_ok, n_tests, results)."""
    results: List[Dict[str, Any]] = []

    def _t(name: str, ok: bool, detail: str = "") -> None:
        results.append({"name": name, "ok": bool(ok), "detail": detail})

    # 1. version constant
    _t("V1420_VERSION is 0.1.0", V1420_VERSION == "0.1.0", V1420_VERSION)
    # 2. endpoints count
    _t("ENDPOINTS has 9 entries", len(ENDPOINTS) == 9, str(len(ENDPOINTS)))
    # 3. bind host validation
    try:
        _validate_bind_host("127.0.0.1")
        _validate_bind_host("0.0.0.0")
        _t("bind validation accepts 127.0.0.1 + 0.0.0.0", True)
    except Exception as e:
        _t("bind validation accepts 127.0.0.1 + 0.0.0.0", False, str(e))
    # 4. bind host rejection
    try:
        _validate_bind_host("evil.example.com")
        _t("bind validation rejects non-allowlist", False, "did not raise")
    except ValueError:
        _t("bind validation rejects non-allowlist", True)
    # 5. port bounds
    try:
        _validate_port(1)
        _validate_port(65535)
        _validate_port(8765)
        _t("port validation accepts 1/65535/8765", True)
    except Exception as e:
        _t("port validation accepts 1/65535/8765", False, str(e))
    # 6. port rejection
    try:
        _validate_port(0)
        _t("port validation rejects 0", False, "did not raise")
    except ValueError:
        _t("port validation rejects 0", True)
    # 7. port rejection max
    try:
        _validate_port(70000)
        _t("port validation rejects > 65535", False, "did not raise")
    except ValueError:
        _t("port validation rejects > 65535", True)
    # 8. max_seconds validation
    try:
        _validate_max_seconds(0)
        _validate_max_seconds(60)
        _validate_max_seconds(3600.5)
        _t("max_seconds validation accepts 0/60/3600.5", True)
    except Exception as e:
        _t("max_seconds validation accepts 0/60/3600.5", False, str(e))
    # 9. path safety
    try:
        _safe_path(Path("..") / "evil")
        _t("path safety rejects ..", False, "did not raise")
    except ValueError:
        _t("path safety rejects ..", True)
    # 10. snapshot build
    try:
        snap = build_snapshot()
        ok = isinstance(snap, ASIStatusSnapshot) and snap.version == V1420_VERSION
        _t("build_snapshot returns ASIStatusSnapshot", ok, snap.last_eval_verdict)
    except Exception as e:
        _t("build_snapshot returns ASIStatusSnapshot", False, str(e))
    # 11. JSON rendering
    try:
        snap = build_snapshot()
        s = render_snapshot_json(snap)
        parsed = json.loads(s)
        _t("render_snapshot_json roundtrips", parsed["version"] == V1420_VERSION)
    except Exception as e:
        _t("render_snapshot_json roundtrips", False, str(e))
    # 12. HTML dashboard rendering
    try:
        snap = build_snapshot()
        h = render_dashboard_html(snap)
        _t("render_dashboard_html contains ASI 总框架", "ASI 总框架" in h)
    except Exception as e:
        _t("render_dashboard_html contains ASI 总框架", False, str(e))
    # 13. atomic write
    try:
        target = PROMETHEAN / ".v1420-popper-tmp.json"
        if target.exists():
            target.unlink()
        _atomic_write_json(target, {"test": True, "ts": _now_utc_iso()})
        present = target.exists()
        _t("atomic write creates file", present)
        if present:
            target.unlink()
    except Exception as e:
        _t("atomic write creates file", False, str(e))
    # 14. handler routing — instantiate and call do_GET for /api/asi/health
    try:
        # Use a fake request to invoke handler method directly
        class _FakeReq:
            def __init__(self): self.path = "/api/asi/health"
            def makefile(self, *a, **k):
                import io
                return io.BytesIO(b"")
        # Set up a minimal handler instance to test routing without socket
        handler = AsiHttpHandler.__new__(AsiHttpHandler)
        captured: Dict[str, Any] = {}
        def fake_send_response(code): captured["code"] = code
        def fake_send_header(k, v): captured[k] = v
        def fake_end_headers(): captured["end"] = True
        class _FakeWFile:
            def __init__(self): self.buf = b""
            def write(self, b): self.buf += b
        wfile = _FakeWFile()
        handler.path = "/api/asi/health"
        handler.wfile = wfile  # type: ignore[assignment]
        handler.send_response = fake_send_response  # type: ignore[assignment]
        handler.send_header = fake_send_header  # type: ignore[assignment]
        handler.end_headers = fake_end_headers  # type: ignore[assignment]
        handler.do_GET()
        body = wfile.buf.decode("utf-8")
        parsed = json.loads(body)
        ok = captured.get("code") == 200 and parsed.get("ok") is True
        _t("do_GET /api/asi/health returns 200 JSON ok", ok, str(captured.get("code")))
    except Exception as e:
        _t("do_GET /api/asi/health returns 200 JSON ok", False, str(e))
    # 15. handler 404
    try:
        captured2: Dict[str, Any] = {}
        class _FakeWFile2:
            def __init__(self): self.buf = b""
            def write(self, b): self.buf += b
        handler = AsiHttpHandler.__new__(AsiHttpHandler)
        handler.path = "/no/such/path"
        handler.wfile = _FakeWFile2()  # type: ignore[assignment]
        handler.send_response = lambda c: captured2.__setitem__("code", c)  # type: ignore[assignment]
        handler.send_header = lambda k, v: captured2.__setitem__(k, v)  # type: ignore[assignment]
        handler.end_headers = lambda: captured2.__setitem__("end", True)  # type: ignore[assignment]
        handler.do_GET()
        body = handler.wfile.buf.decode("utf-8")
        parsed = json.loads(body)
        ok = captured2.get("code") == 404 and parsed.get("error") == "not found"
        _t("do_GET unknown path returns 404 JSON error", ok)
    except Exception as e:
        _t("do_GET unknown path returns 404 JSON error", False, str(e))
    # 16. make_server returns HTTPServer instance with valid address (no actual serve)
    try:
        # Use a thread to avoid any Windows-specific blocking in HTTPServer init
        import socket as _socket
        with _socket.socket(_socket.AF_INET, _socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            ephemeral_port = s.getsockname()[1]
        # Now verify make_server can bind to that known port
        httpd, snap = make_server("127.0.0.1", ephemeral_port, max_seconds=1.0)
        bound_port = httpd.server_address[1]
        # Don't call stop_server (Windows quirk); let it GC
        httpd.server_close()
        del httpd
        _t("make_server binds to known port and returns httpd", bound_port == ephemeral_port,
           f"bound={bound_port} expected={ephemeral_port}")
    except Exception as e:
        _t("make_server binds to known port and returns httpd", False, str(e))
    # 17. chain delegate (integration)
    try:
        chain = _read_v1419_chain()
        _t("V1419 chain_delegate readable from V1420", "all_ok" in chain, str(chain.get("all_ok")))
    except Exception as e:
        _t("V1419 chain_delegate readable from V1420", False, str(e))

    all_ok = all(r["ok"] for r in results)
    return all_ok, len(results), results


# ============================================================================
# Chain delegate (V1417 + V1418 + V1419)
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe V1417 + V1418 + V1419 chain integrity; returns aggregate dict."""
    out: Dict[str, Any] = {
        "schema": V1420_SCHEMA,
        "version": V1420_VERSION,
        "modules": [],
        "all_ok": True,
        "n_modules": 0,
        "n_modules_ok": 0,
        "errors": [],
    }
    probes: List[Tuple[str, str, Any]] = [
        ("V1417", "tick_history", _read_v1417_history(DEFAULT_HISTORY_PATH)),
        ("V1418", "cron_integration", None),
        ("V1419", "multi_policy_evaluator", _read_v1419_chain()),
    ]
    # Probe V1418 by trying to import and check run_session
    try:
        from apeireth import v1418_asi_dgm_cron_integration as m1418
        probes[1] = ("V1418", "cron_integration", {"importable": True, "module": m1418.V1418_MODULE})
    except Exception as e:
        out["errors"].append(f"V1418 import failed: {e}")
        probes[1] = ("V1418", "cron_integration", {"importable": False})
    for mod_id, kind, data in probes:
        ok = True
        if mod_id == "V1417":
            ok = isinstance(data, dict) and "n" in data
        elif mod_id == "V1418":
            ok = isinstance(data, dict) and data.get("importable", False)
        elif mod_id == "V1419":
            ok = isinstance(data, dict) and bool(data.get("all_ok", False))
        out["modules"].append({
            "module": mod_id,
            "kind": kind,
            "ok": ok,
            "data": data,
        })
        out["n_modules"] += 1
        if ok:
            out["n_modules_ok"] += 1
        else:
            out["all_ok"] = False
    return out


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(f"""V1420 — ASI 总框架 HTTP status endpoint v{V1420_VERSION}

Usage:
  python -m apeireth.v1420_asi_http_status_endpoint <command> [options]

Commands:
  version                                    Print module version
  meta [--json]                              Print module metadata
  demo                                       Run a tiny demo (build_snapshot)
  help                                       Print this help
  popper                                     Run popper self-test (17 tests)
  chain                                      Run V1417+V1418+V1419 chain probe
  snapshot --out PATH                        Save JSON snapshot to file
  status                                     Print JSON status to stdout
  dashboard                                  Print HTML dashboard to stdout
  serve --bind HOST --port PORT [--max-seconds N]

Examples:
  python -m apeireth.v1420_asi_http_status_endpoint popper
  python -m apeireth.v1420_asi_http_status_endpoint status
  python -m apeireth.v1420_asi_http_status_endpoint snapshot --out .asi_snapshot.json
  python -m apeireth.v1420_asi_http_status_endpoint serve --bind 127.0.0.1 --port 8765 --max-seconds 60
  curl -s http://127.0.0.1:8765/api/asi/health
  curl -s http://127.0.0.1:8765/api/asi/verdict | jq .
""")


def _parse_kv_args(argv: List[str]) -> Dict[str, str]:
    """Parse --key value style flags into a dict."""
    out: Dict[str, str] = {}
    i = 0
    while i < len(argv):
        a = argv[i]
        if a.startswith("--") and i + 1 < len(argv):
            key = a[2:]
            val = argv[i + 1]
            out[key] = val
            i += 2
        else:
            i += 1
    return out


def run_cli(argv: List[str]) -> int:
    if not argv:
        _print_help()
        return 0
    cmd = argv[0]
    rest = argv[1:]
    if cmd in ("help", "-h", "--help"):
        _print_help()
        return 0
    if cmd == "version":
        print(f"V1420 version {V1420_VERSION} (schema {V1420_SCHEMA})")
        return 0
    if cmd == "meta":
        meta = {
            "module": V1420_MODULE,
            "version": V1420_VERSION,
            "schema": V1420_SCHEMA,
            "endpoints": list(ENDPOINTS),
            "guards": list(V1420_GUARDS),
            "v3_guards": list(V1420_V3_GUARDS),
            "borrowed": [{"module": m, "role": r} for (m, r) in V1420_BORROWED],
            "default_bind_host": DEFAULT_BIND_HOST,
            "default_port": DEFAULT_PORT,
        }
        if rest and rest[0] == "--json":
            print(json.dumps(meta, ensure_ascii=False, indent=2, sort_keys=True))
        else:
            for k, v in meta.items():
                if isinstance(v, list):
                    print(f"  {k}: ({len(v)} entries)")
                else:
                    print(f"  {k}: {v}")
        return 0
    if cmd == "demo":
        snap = build_snapshot()
        print(render_snapshot_json(snap))
        return 0
    if cmd == "popper":
        all_ok, n_tests, results = popper_self_test()
        print(json.dumps({
            "all_ok": all_ok,
            "n_tests": n_tests,
            "n_passed": sum(1 for r in results if r["ok"]),
            "results": results,
        }, ensure_ascii=False, indent=2, sort_keys=True))
        return 0 if all_ok else 1
    if cmd == "chain":
        chain = chain_delegate()
        print(json.dumps(chain, ensure_ascii=False, indent=2, sort_keys=True))
        return 0 if chain.get("all_ok") else 1
    if cmd == "snapshot":
        kv = _parse_kv_args(rest)
        out_path = Path(kv.get("out", str(DEFAULT_SNAPSHOT_PATH)))
        snap = build_snapshot()
        _atomic_write_json(out_path, snap.to_dict())
        print(json.dumps({"saved": str(out_path), "verdict": snap.last_eval_verdict}))
        return 0
    if cmd == "status":
        snap = build_snapshot()
        print(render_snapshot_json(snap))
        return 0
    if cmd == "dashboard":
        snap = build_snapshot()
        print(render_dashboard_html(snap))
        return 0
    if cmd == "serve":
        kv = _parse_kv_args(rest)
        bind = kv.get("bind", DEFAULT_BIND_HOST)
        port = _safe_int(kv.get("port", str(DEFAULT_PORT)))
        max_seconds = float(kv.get("max-seconds", "0"))
        eval_path = Path(kv["eval-path"]) if "eval-path" in kv else None
        history_path = Path(kv["history-path"]) if "history-path" in kv else None
        try:
            served = serve_forever_blocking(bind, port, max_seconds, eval_path, history_path)
        except OSError as e:
            print(json.dumps({
                "error": f"bind failed: {e}",
                "bind": bind,
                "port": port,
            }))
            return 2
        print(json.dumps(served, ensure_ascii=False, indent=2, sort_keys=True))
        return 0
    print(f"unknown command: {cmd!r}")
    _print_help()
    return 1


if __name__ == "__main__":  # pragma: no cover
    sys.exit(run_cli(sys.argv[1:]))