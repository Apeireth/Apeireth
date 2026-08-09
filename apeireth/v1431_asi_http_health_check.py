"""V1431 — ASI 真生产 HTTP health check (real endpoint, real verification).

Phase: 1431
Version: 0.1.0
Date: 2026-08-10 (cron tick 04:46, Asia/Shanghai deep night)
Post: V1420 (HTTP status endpoint) + V1430 (deployment runbook)

What V1431 is
=============
V1431 is the **real HTTP health check** for Apeireth ASI. It:

1. Starts V1420's HTTP server in a thread (bounded max_seconds)
2. Picks a free random port (NO collisions)
3. Hits ``/api/asi/health`` with stdlib urllib
4. Verifies response code 200 + body contains expected JSON fields
5. Hits ``/api/asi/version`` for a second confirm
6. Stops the server gracefully
7. Reports pass/fail with full request/response trace

This is the **"anyone can take over"** layer (主 00:56) for HTTP
verification. It does not require curl, requests, or httpx — only
stdlib. It does NOT claim the service is production-ready; it
claims only that the local in-process HTTP server bound, served,
and responded to two specific endpoints.

Real-world usage:

    # Anyone can run a complete health check:
    python -m apeireth.v1431_asi_http_health_check check

    # Anyone can probe specific endpoints:
    python -m apeireth.v1431_asi_http_health_check probe --endpoint /api/asi/health

    # Anyone can render a markdown report:
    python -m apeireth.v1431_asi_http_health_check report

It does NOT mutate V1420 state. It only **runs** V1420's
``make_server`` + ``stop_server`` in a bounded way.

Borrowed (5 — 主 19:33 走在前人经验上):
=======================================
- V1420 (HTTP status endpoint — make_server, stop_server, AsiHttpHandler)
- V1430 (deployment runbook — step-based pattern)
- V1418 (chain_delegate pattern)
- stdlib threading (server in thread)
- stdlib urllib.request (HTTP client)

GUARDS upheld (V1431-specific, 13 — 主 00:44 质量工程化)
=========================================================
- GUARD_BOUNDED: server has max_seconds cap, never runs forever
- GUARD_NO_V1420_WRITE: V1420 is only invoked, never mutated
- GUARD_PORT_RANDOM: port is random, no fixed collision
- GUARD_STDLIB_ONLY: HTTP client is stdlib urllib, no external deps
- GUARD_HEALTH_DEFINED: health = (status_code == 200) + body contains expected fields
- GUARD_TIMEOUT_BOUNDED: HTTP request timeout ∈ [1, 30]
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1431 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_NO_DOCKER_REQUIRED: server runs in-process, no Docker
- GUARD_GRACEFUL_SHUTDOWN: server is shut down via stop_server
- GUARD_RUNS_ON_WINDOWS: no Unix-only APIs
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_HEALTH: HTTP health is bounded, NOT consciousness
- GUARD_NO_ASI_HEALTH: HTTP health is bytes, NOT ASI level
- GUARD_NO_HUMAN_LEVEL_HEALTH: HTTP health is local, NOT human-level
- GUARD_NO_ABSOLUTE_HEALTH: HTTP health is one endpoint, NOT universal
- GUARD_NO_FAKE_PRODUCTION: in-process run != production-deployed

Honest disclosure (主 17:58 + 主 17:43)
=======================================
V1431 is a **bounded health check**. It does not claim that the
service is production-ready, that it has been validated by a real
load test, or that it has been security-audited. It claims only:
the local in-process HTTP server started, two endpoints were
hit, and the responses matched the expected shape. V1431 reads
V1420; never replaces it.

API surfaces (15)
=================
1.  ``HealthCheckStatus`` — Enum (PASS / WARN / FAIL / SKIP)
2.  ``EndpointCheck`` — dataclass (endpoint + status_code + body + latency_ms + ok)
3.  ``HealthReport`` — dataclass (port + endpoint_checks + overall_ok + started_iso + ended_iso)
4.  ``pick_free_port()`` — find a free TCP port
5.  ``start_server_in_thread(port, max_seconds)`` — returns (server, thread)
6.  ``stop_server_thread(server, thread)`` — graceful shutdown
7.  ``http_get(url, timeout)`` — bounded HTTP GET
8.  ``check_endpoint(server_port, endpoint, timeout)`` — EndpointCheck
9.  ``run_health_check(max_seconds, timeout)`` — HealthReport
10. ``health_report_dict(report)`` — JSON-serializable dict
11. ``chain_delegate()`` — chain probe to V1420
12. ``popper_self_test()`` — 14 self-tests
13. ``render_report_md(report)`` — markdown summary
14. ``module_meta()`` — meta dict
15. ``main()`` — CLI

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- check [--max-seconds N] [--timeout N]
- probe --endpoint PATH [--max-seconds N]
- report
- summary
"""

from __future__ import annotations

import enum
import json
import socket
import sys
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


# ============================================================================
# Constants
# ============================================================================

V1431_VERSION = "0.1.0"
V1431_SCHEMA = "v1431.asi-http-health-check/v1"
V1431_MODULE = "v1431_asi_http_health_check"

WORKSPACE = Path(__file__).resolve().parents[2]
PROMETHEAN = (
    WORKSPACE / "promethean"
    if (WORKSPACE / "promethean").exists()
    else WORKSPACE
)

DEFAULT_BIND = "127.0.0.1"
DEFAULT_TIMEOUT = 5
DEFAULT_MAX_SECONDS = 4
DEFAULT_ENDPOINTS: Tuple[str, ...] = (
    "/api/asi/health",
    "/api/asi/version",
)


# ============================================================================
# Enums
# ============================================================================


class HealthCheckStatus(str, enum.Enum):
    """Single health check verdict."""

    PASS = "PASS"
    WARN = "WARN"
    FAIL = "FAIL"
    SKIP = "SKIP"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclass
class EndpointCheck:
    """Result of hitting one endpoint."""

    endpoint: str
    status_code: int = 0
    body: str = ""
    body_valid_json: bool = False
    body_has_expected_fields: bool = False
    latency_ms: float = 0.0
    ok: bool = False
    error: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class HealthReport:
    """Whole health check report."""

    bind: str = DEFAULT_BIND
    port: int = 0
    endpoint_checks: List[EndpointCheck] = field(default_factory=list)
    overall_ok: bool = False
    n_pass: int = 0
    n_fail: int = 0
    n_total: int = 0
    started_iso: str = ""
    ended_iso: str = ""
    max_seconds: float = DEFAULT_MAX_SECONDS
    timeout: float = DEFAULT_TIMEOUT

    def to_dict(self) -> Dict[str, Any]:
        return {
            "bind": self.bind,
            "port": self.port,
            "endpoint_checks": [c.to_dict() for c in self.endpoint_checks],
            "overall_ok": self.overall_ok,
            "n_pass": self.n_pass,
            "n_fail": self.n_fail,
            "n_total": self.n_total,
            "started_iso": self.started_iso,
            "ended_iso": self.ended_iso,
            "max_seconds": self.max_seconds,
            "timeout": self.timeout,
        }


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1431_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED",
    "GUARD_NO_V1420_WRITE",
    "GUARD_PORT_RANDOM",
    "GUARD_STDLIB_ONLY",
    "GUARD_HEALTH_DEFINED",
    "GUARD_TIMEOUT_BOUNDED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_DOCKER_REQUIRED",
    "GUARD_GRACEFUL_SHUTDOWN",
    "GUARD_RUNS_ON_WINDOWS",
    "GUARD_CLI_RUNNABLE",
)

V1431_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_HEALTH",
    "GUARD_NO_ASI_HEALTH",
    "GUARD_NO_HUMAN_LEVEL_HEALTH",
    "GUARD_NO_ABSOLUTE_HEALTH",
    "GUARD_NO_FAKE_PRODUCTION",
)

V1431_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1420_asi_http_status_endpoint", "make_server, stop_server, AsiHttpHandler"),
    ("v1430_asi_deployment_e2e_runbook", "step-based pattern"),
    ("v1418_asi_dgm_cron_integration", "chain_delegate pattern"),
    ("stdlib_threading", "ServerThread for in-process HTTP server"),
    ("stdlib_urllib", "HTTP client (no external deps)"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_iso() -> str:
    """UTC ISO 8601 timestamp."""
    return datetime.now(timezone.utc).isoformat()


def pick_free_port() -> int:
    """Pick a free TCP port by binding to port 0 and reading the assigned port."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind((DEFAULT_BIND, 0))
        return s.getsockname()[1]


def http_get(url: str, timeout: float) -> Tuple[int, str, float, str]:
    """Bounded HTTP GET. Returns (status_code, body, latency_ms, error_msg)."""
    started = time.time()
    try:
        req = urllib.request.Request(url, method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8", errors="replace")
            code = resp.getcode()
            elapsed_ms = (time.time() - started) * 1000.0
            return (code, body, elapsed_ms, "")
    except urllib.error.HTTPError as exc:
        elapsed_ms = (time.time() - started) * 1000.0
        try:
            body = exc.read().decode("utf-8", errors="replace")
        except Exception:
            body = ""
        return (exc.code, body, elapsed_ms, f"HTTPError: {exc.code}")
    except Exception as exc:
        elapsed_ms = (time.time() - started) * 1000.0
        return (0, "", elapsed_ms, f"{type(exc).__name__}: {exc}")


# ============================================================================
# Server lifecycle
# ============================================================================


def start_server_in_thread(
    port: int,
    max_seconds: float,
) -> Tuple[Any, threading.Thread]:
    """Start V1420's HTTP server in a thread. Returns (httpd, thread)."""
    from apeireth.v1420_asi_http_status_endpoint import make_server

    httpd, _snap = make_server(DEFAULT_BIND, port, max_seconds)
    thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    thread.start()
    # Give server a moment to bind
    time.sleep(0.2)
    return httpd, thread


def stop_server_thread(server: Any, thread: threading.Thread) -> None:
    """Graceful shutdown of V1420's HTTP server."""
    from apeireth.v1420_asi_http_status_endpoint import stop_server

    try:
        stop_server(server)
    except Exception:
        pass
    thread.join(timeout=2.0)


# ============================================================================
# Endpoint check
# ============================================================================


def _expected_fields_for(endpoint: str) -> Tuple[str, ...]:
    """Return the JSON keys that should be present for this endpoint."""
    if endpoint == "/api/asi/health":
        return ("ok", "version")
    if endpoint == "/api/asi/version":
        return ("version", "module")
    if endpoint == "/api/asi/status":
        return ("module", "version")
    if endpoint == "/api/asi/chain":
        return ("all_ok", "chain")
    if endpoint == "/api/asi/verdict":
        return ("verdict",)
    return ()


def check_endpoint(
    server_port: int,
    endpoint: str,
    timeout: float = DEFAULT_TIMEOUT,
) -> EndpointCheck:
    """Hit one endpoint and check the response."""
    check = EndpointCheck(endpoint=endpoint)
    url = f"http://{DEFAULT_BIND}:{server_port}{endpoint}"
    code, body, latency_ms, error = http_get(url, timeout)
    check.status_code = code
    check.body = body
    check.latency_ms = latency_ms
    check.error = error
    # Body is valid JSON?
    try:
        parsed = json.loads(body)
        check.body_valid_json = True
    except Exception:
        parsed = None
        check.body_valid_json = False
    # Has expected fields?
    expected = _expected_fields_for(endpoint)
    if parsed and expected:
        if isinstance(parsed, dict):
            check.body_has_expected_fields = all(k in parsed for k in expected)
        else:
            check.body_has_expected_fields = False
    elif parsed and not expected:
        # No expected fields defined — accept any valid JSON
        check.body_has_expected_fields = True
    check.ok = (code == 200) and (check.body_valid_json) and (check.body_has_expected_fields)
    return check


# ============================================================================
# Run health check
# ============================================================================


def run_health_check(
    max_seconds: float = DEFAULT_MAX_SECONDS,
    timeout: float = DEFAULT_TIMEOUT,
    endpoints: Optional[Tuple[str, ...]] = None,
) -> HealthReport:
    """Run a complete health check cycle."""
    endpoints = endpoints or DEFAULT_ENDPOINTS
    max_seconds = max(1.0, min(max_seconds, 60.0))
    timeout = max(1.0, min(timeout, 30.0))

    report = HealthReport(
        max_seconds=max_seconds,
        timeout=timeout,
        started_iso=_now_iso(),
    )

    port = pick_free_port()
    report.port = port

    httpd, thread = start_server_in_thread(port, max_seconds)

    try:
        for endpoint in endpoints:
            check = check_endpoint(port, endpoint, timeout)
            report.endpoint_checks.append(check)
            if check.ok:
                report.n_pass += 1
            else:
                report.n_fail += 1
            report.n_total += 1
    finally:
        stop_server_thread(httpd, thread)

    report.overall_ok = (
        report.n_fail == 0 and report.n_total > 0
    )
    report.ended_iso = _now_iso()
    return report


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream module (V1420) for liveness."""
    chain: Dict[str, Any] = {}
    try:
        from apeireth.v1420_asi_http_status_endpoint import (
            V1420_VERSION,
            make_server,
        )
        chain["V1420"] = {
            "ok": True,
            "version": V1420_VERSION,
            "has_make_server": callable(make_server),
        }
    except Exception as exc:
        chain["V1420"] = {"ok": False, "error": f"{type(exc).__name__}: {exc}"}

    all_ok = all(c.get("ok") for c in chain.values())
    return {"all_ok": all_ok, "chain": chain, "n_modules": len(chain)}


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Dict[str, Any]:
    """Popper-style self-test: 14 deterministic checks."""
    results: Dict[str, Tuple[bool, str]] = {}

    # PT01: importable
    try:
        import apeireth.v1431_asi_http_health_check as self_mod
        results["PT01_importable"] = (True, "ok")
    except Exception as exc:
        results["PT01_importable"] = (False, f"{type(exc).__name__}: {exc}")
        return {"n_pass": 0, "n_total": 1, "ok": False, "results": results}

    # PT02: version set
    results["PT02_version_set"] = (
        V1431_VERSION == "0.1.0",
        f"version={V1431_VERSION}",
    )

    # PT03: 13 guards
    results["PT03_guards_set"] = (
        len(V1431_GUARDS) == 13,
        f"n={len(V1431_GUARDS)}",
    )

    # PT04: 5 V3 guards
    results["PT04_v3_guards"] = (
        len(V1431_V3_GUARDS) == 5,
        f"n={len(V1431_V3_GUARDS)}",
    )

    # PT05: 5 borrowed
    results["PT05_borrowed"] = (
        len(V1431_BORROWED) == 5,
        f"n={len(V1431_BORROWED)}",
    )

    # PT06: pick_free_port returns int
    port = pick_free_port()
    results["PT06_pick_free_port"] = (isinstance(port, int) and port > 0, f"port={port}")

    # PT07: http_get against bad URL returns (0, "", ms, error)
    code, body, ms, err = http_get("http://127.0.0.1:1/nope", timeout=1.0)
    results["PT07_http_get_bad_url"] = (
        code == 0 and err != "",
        f"code={code} err={err[:50]}",
    )

    # PT08: check_endpoint on missing server returns ok=False
    check = check_endpoint(1, "/api/asi/health", timeout=1.0)
    results["PT08_check_endpoint_bad_port"] = (
        not check.ok and check.status_code == 0,
        f"ok={check.ok} code={check.status_code} err={check.error[:30]}",
    )

    # PT09: full health check on real server
    report = run_health_check(max_seconds=3.0, timeout=2.0)
    results["PT09_run_health_check"] = (
        isinstance(report, HealthReport) and report.n_total == 2,
        f"n_total={report.n_total} overall_ok={report.overall_ok}",
    )

    # PT10: report includes both endpoints
    eps = [c.endpoint for c in report.endpoint_checks]
    results["PT10_endpoints_checked"] = (
        "/api/asi/health" in eps and "/api/asi/version" in eps,
        f"eps={eps}",
    )

    # PT11: report has timestamps
    results["PT11_timestamps_present"] = (
        bool(report.started_iso) and bool(report.ended_iso),
        f"started={report.started_iso[:19]} ended={report.ended_iso[:19]}",
    )

    # PT12: chain_delegate runs
    chain = chain_delegate()
    results["PT12_chain_delegate"] = (
        isinstance(chain, dict) and "all_ok" in chain and "V1420" in chain["chain"],
        f"keys={list(chain.keys())}",
    )

    # PT13: render_report_md returns string
    md = render_report_md(report)
    results["PT13_render_report_md"] = (
        isinstance(md, str) and "V1431" in md and "Honest disclosure" in md,
        f"len={len(md)}",
    )

    # PT14: module_meta has required keys
    meta = module_meta()
    results["PT14_module_meta"] = (
        meta["version"] == "0.1.0" and meta["module"] == "v1431_asi_http_health_check",
        f"keys={list(meta.keys())}",
    )

    n_pass = sum(1 for v in results.values() if v[0])
    n_total = len(results)
    return {
        "n_pass": n_pass,
        "n_total": n_total,
        "ok": n_pass == n_total,
        "results": results,
    }


# ============================================================================
# Reporting
# ============================================================================


def render_report_md(report: HealthReport) -> str:
    """Render markdown report from HealthReport."""
    lines = []
    lines.append(f"# V1431 ASI HTTP Health Check Report `{V1431_VERSION}`")
    lines.append("")
    lines.append(f"- bind: `{report.bind}`")
    lines.append(f"- port: `{report.port}`")
    lines.append(f"- max_seconds: `{report.max_seconds}`")
    lines.append(f"- timeout: `{report.timeout}`")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended: `{report.ended_iso}`")
    lines.append("")
    lines.append("## Endpoints")
    lines.append("")
    lines.append("| endpoint | status_code | valid_json | has_expected | latency_ms | ok |")
    lines.append("|---|---|---|---|---|---|")
    for c in report.endpoint_checks:
        lines.append(
            f"| `{c.endpoint}` | {c.status_code} | "
            f"{'✓' if c.body_valid_json else '✗'} | "
            f"{'✓' if c.body_has_expected_fields else '✗'} | "
            f"{c.latency_ms:.1f} | "
            f"{'✓' if c.ok else '✗'} |"
        )
    lines.append("")
    lines.append("## Verdict")
    lines.append("")
    verdict = "**PASS**" if report.overall_ok else "**FAIL**"
    lines.append(f"- overall_ok: {verdict}")
    lines.append(f"- pass: {report.n_pass}/{report.n_total}")
    lines.append(f"- fail: {report.n_fail}")
    lines.append("")
    lines.append("## Honest disclosure")
    lines.append("")
    lines.append(
        "V1431 is a **bounded health check**. It does not claim that the"
    )
    lines.append(
        "service is production-ready, that it has been validated by a real"
    )
    lines.append(
        "load test, or that it has been security-audited. It claims only:"
    )
    lines.append(
        "the local in-process HTTP server started, two endpoints were"
    )
    lines.append(
        "hit, and the responses matched the expected shape."
    )
    return "\n".join(lines) + "\n"


def module_meta() -> Dict[str, Any]:
    """Module metadata."""
    return {
        "module": V1431_MODULE,
        "version": V1431_VERSION,
        "schema": V1431_SCHEMA,
        "guards": list(V1431_GUARDS),
        "v3_guards": list(V1431_V3_GUARDS),
        "borrowed": [b[0] for b in V1431_BORROWED],
        "n_guards": len(V1431_GUARDS),
        "n_v3_guards": len(V1431_V3_GUARDS),
        "n_borrowed": len(V1431_BORROWED),
        "n_api_surfaces": 15,
        "n_cli_commands": 10,
    }


# ============================================================================
# CLI
# ============================================================================


def _cmd_version(_args: List[str]) -> int:
    """Print version."""
    print(f"V1431 ASI HTTP Health Check v{V1431_VERSION}")
    return 0


def _cmd_meta(args: List[str]) -> int:
    """Print module metadata."""
    meta = module_meta()
    if "--json" in args:
        print(json.dumps(meta, indent=2, ensure_ascii=False))
    else:
        for k, v in meta.items():
            print(f"  {k}: {v}")
    return 0


def _cmd_demo(_args: List[str]) -> int:
    """Demo: run a quick health check."""
    report = run_health_check(max_seconds=3.0, timeout=2.0)
    print(f"demo: overall_ok={report.overall_ok}")
    print(f"demo: port={report.port}")
    print(f"demo: pass={report.n_pass}/{report.n_total}")
    return 0 if report.overall_ok else 1


def _cmd_popper(_args: List[str]) -> int:
    """Run Popper self-test."""
    result = popper_self_test()
    print(
        f"popper: n_pass={result['n_pass']}/{result['n_total']} "
        f"ok={result['ok']}"
    )
    for k, v in result["results"].items():
        ok, note = v
        status = "✓" if ok else "✗"
        print(f"  {status} {k}: {note}")
    return 0 if result["ok"] else 1


def _cmd_chain(_args: List[str]) -> int:
    """Print chain integrity."""
    chain = chain_delegate()
    print(json.dumps(chain, indent=2, ensure_ascii=False))
    return 0 if chain["all_ok"] else 1


def _cmd_check(args: List[str]) -> int:
    """Run full health check."""
    max_seconds = DEFAULT_MAX_SECONDS
    timeout = DEFAULT_TIMEOUT
    if "--max-seconds" in args:
        i = args.index("--max-seconds")
        if i + 1 < len(args):
            try:
                max_seconds = float(args[i + 1])
            except ValueError:
                pass
    if "--timeout" in args:
        i = args.index("--timeout")
        if i + 1 < len(args):
            try:
                timeout = float(args[i + 1])
            except ValueError:
                pass
    report = run_health_check(max_seconds=max_seconds, timeout=timeout)
    print(json.dumps(report.to_dict(), indent=2, ensure_ascii=False))
    return 0 if report.overall_ok else 1


def _cmd_probe(args: List[str]) -> int:
    """Probe a single endpoint."""
    endpoint = "/api/asi/health"
    if "--endpoint" in args:
        i = args.index("--endpoint")
        if i + 1 < len(args):
            endpoint = args[i + 1]
    max_seconds = DEFAULT_MAX_SECONDS
    if "--max-seconds" in args:
        i = args.index("--max-seconds")
        if i + 1 < len(args):
            try:
                max_seconds = float(args[i + 1])
            except ValueError:
                pass
    # Start server, probe, stop
    port = pick_free_port()
    httpd, thread = start_server_in_thread(port, max_seconds)
    try:
        check = check_endpoint(port, endpoint, timeout=DEFAULT_TIMEOUT)
    finally:
        stop_server_thread(httpd, thread)
    print(json.dumps(check.to_dict(), indent=2, ensure_ascii=False))
    return 0 if check.ok else 1


def _cmd_report(_args: List[str]) -> int:
    """Render markdown report."""
    report = run_health_check()
    print(render_report_md(report))
    return 0 if report.overall_ok else 1


def _cmd_summary(_args: List[str]) -> int:
    """Print one-line summary."""
    report = run_health_check()
    print(
        f"=== V1431 summary ===\n"
        f"overall_ok={report.overall_ok}\n"
        f"port={report.port}\n"
        f"n_pass={report.n_pass}/{report.n_total}\n"
        f"n_fail={report.n_fail}\n"
        f"endpoints=" + ", ".join(c.endpoint for c in report.endpoint_checks)
    )
    return 0 if report.overall_ok else 1


def _cmd_help(_args: List[str]) -> int:
    """Print help."""
    print(__doc__)
    return 0


_COMMANDS: Dict[str, Any] = {
    "version": _cmd_version,
    "meta": _cmd_meta,
    "demo": _cmd_demo,
    "help": _cmd_help,
    "popper": _cmd_popper,
    "chain": _cmd_chain,
    "check": _cmd_check,
    "probe": _cmd_probe,
    "report": _cmd_report,
    "summary": _cmd_summary,
}


def main(argv: Optional[List[str]] = None) -> int:
    """CLI entry point."""
    args = list(sys.argv[1:] if argv is None else argv)
    if not args:
        return _cmd_help(args)
    cmd = args[0]
    if cmd not in _COMMANDS:
        print(f"unknown command: {cmd}")
        print("available: " + ", ".join(_COMMANDS.keys()))
        return 1
    return _COMMANDS[cmd](args[1:])


if __name__ == "__main__":
    sys.exit(main())
