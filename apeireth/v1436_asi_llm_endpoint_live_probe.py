"""V1436 — ASI 真生产 LLM endpoint live probe (主 13:31 + 主 23:44 + 主 00:56 + 主 17:43).

Phase: 1436
Version: 0.1.0
Date: 2026-08-10 (cron tick 05:20, Asia/Shanghai deep night)
Post: V1435 (docker availability probe) + V1424 (benchmark mock-mode LLM)

What V1436 is
=============
V1436 is the **real HTTP LLM endpoint probe** for Apeireth ASI. Where:

- V1424 ran benchmarks in MOCK mode (no real network) tagged mode=MOCK
- V1076 is the full external LLM client (key + completion + streaming)
- V1435 probes docker availability (subprocess)

V1436 is **one focused, runnable probe** that answers the question
anyone can verify:

    Given an LLM endpoint URL (e.g. https://api.example.com),
    can THIS host actually reach it? Does it respond to GET /v1/models?
    What does the response body look like? Did auth get challenged?

V1436 actually invokes:

1.  ``GET {base_url}/v1/models``      — list models (OpenAI-compatible)
2.  ``GET {base_url}/api/status``     — Bocha-style status check
3.  ``GET {base_url}/``               — root reachability probe
4.  Retry with exponential backoff (1s, 2s, 4s, capped at timeout)
5.  Parse response status + body + headers
6.  Detect auth challenges (401 / 403 / x-api-key required)

Each call has a **bounded timeout** (default 10s) and **offline-safe
fallback** — if the endpoint is unreachable, V1436 reports
``mode=ENDPOINT_UNREACHABLE`` without raising. If reachable but
401, reports ``mode=AUTH_REQUIRED``. If reachable and 200, reports
``mode=ENDPOINT_REACHABLE`` + parsed body shape.

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1436 is an **endpoint reachability probe**. It does NOT claim:

- The endpoint is production-deployed
- The model will return correct answers
- The API key is valid
- The endpoint is safe or trustworthy
- Responses are reproducible or stable

It claims only: **from this host, with the configured URL, here
is what the bounded HTTP calls returned**. V1436 ≠ Phenomenal
probe, ≠ ASI probe, ≠ human-level probe, ≠ absolute probe, ≠
real completion call. Probe ≠ chat. Probe ≠ ASI 达成.

Borrowed (5 — 主 19:33 走在前人经验上):
========================================
- V1424 (benchmark 真接 LLM — providers list pattern)
- V1076 (external LLM client — retry + backoff pattern)
- V1435 (docker probe — bounded subprocess pattern)
- stdlib urllib.request (real HTTP GET)
- stdlib json (parse response body)

GUARDS upheld (V1436-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_TIMEOUT: every HTTP call has timeout ∈ [1, 60]
- GUARD_NO_RAISE: HTTP failures are caught, never raised
- GUARD_OFFLINE_SAFE: unreachable endpoint reports UNREACHABLE (no crash)
- GUARD_URL_VALIDATED: URL is validated (scheme ∈ http/https)
- GUARD_NO_API_KEY_REQUIRED: V1436 probes reachability, NOT key validity
- GUARD_RETRY_CAPPED: retries are bounded (1, 2, 4s)
- GUARD_RESPONSE_BOUNDED: response body truncated to MAX_BODY_BYTES
- GUARD_HEADERS_PARSED: response headers parsed for server info
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1436 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_NO_COMPLETION_CALL: V1436 only probes /v1/models, never POSTs chat
- GUARD_NO_PRODUCTION_DEPLOY: probe ≠ deploy
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
=======================================================
- GUARD_NO_PHENOMENAL_PROBE: HTTP probe is bytes, NOT consciousness
- GUARD_NO_ASI_PROBE: HTTP probe is reachability, NOT ASI
- GUARD_NO_HUMAN_LEVEL_PROBE: HTTP probe is one URL, NOT universal
- GUARD_NO_ABSOLUTE_PROBE: HTTP probe is current state, NOT eternal
- GUARD_NO_V1424_REPLACE: V1436 probes reachability, V1424 does benchmark

API surfaces (17)
=================
1.  ``ProbeOutcome`` — Enum (ENDPOINT_REACHABLE / ENDPOINT_UNREACHABLE /
    AUTH_REQUIRED / PARSE_OK / PARSE_FAIL / SKIPPED / ERROR)
2.  ``HttpCall`` — dataclass (url + method + status_code + elapsed_ms +
    body_bytes + body_truncated + server_header + content_type +
    timed_out + attempt + mode)
3.  ``EndpointProbeResult`` — dataclass (endpoint_url + probe_outcome +
    http_calls + models_count + models_sample + server +
    content_type + started_iso + ended_iso + timeout_seconds)
4.  ``DEFAULT_TIMEOUT_SECONDS`` — int (10)
5.  ``MAX_TIMEOUT_SECONDS`` — int (60)
6.  ``MAX_BODY_BYTES`` — int (8192)
7.  ``DEFAULT_ENDPOINTS`` — tuple (3 OpenAI-compatible URLs)
8.  ``validate_url(url)`` — bool (scheme ∈ http/https)
9.  ``http_get(url, timeout)`` — HttpCall (bounded, offline-safe)
10. ``http_get_with_retry(url, timeout, max_attempts)`` — HttpCall
11. ``probe_models_endpoint(base_url, timeout)`` — (HttpCall, int, list)
12. ``probe_status_endpoint(base_url, timeout)`` — HttpCall
13. ``probe_root_endpoint(base_url, timeout)`` — HttpCall
14. ``run_endpoint_probe(base_url, timeout)`` — EndpointProbeResult
15. ``result_to_dict(result)`` — JSON-serializable dict
16. ``chain_delegate()`` — chain probe to V1435 + V1424 + V1076
17. ``popper_self_test()`` — 14 self-tests
18. ``main(argv)`` — CLI

CLI commands (8 — 主 00:56 任何人都能接手):
===========================================
- version
- meta [--json]
- help
- popper
- chain
- probe --url URL [--timeout SECONDS]  (full probe + render summary)
- json  --url URL [--timeout SECONDS]  (probe + emit JSON)
- call --url URL [--timeout SECONDS]   (single bounded GET)
"""

from __future__ import annotations

import dataclasses
import json
import re
import socket
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

V1436_VERSION = "0.1.0"
V1436_SCHEMA = "v1436.asi-llm-endpoint-live-probe/v1"
V1436_MODULE = "v1436_asi_llm_endpoint_live_probe"

DEFAULT_TIMEOUT_SECONDS = 10
MAX_TIMEOUT_SECONDS = 60
MAX_BODY_BYTES = 8192
MAX_RETRIES = 3  # attempts: 1, 2, 4 sec backoff capped

DEFAULT_ENDPOINTS: Tuple[str, ...] = (
    "https://api.openai.com",
    "https://api.anthropic.com",
    "http://localhost:11434",  # Ollama default
)


# ============================================================================
# Enums / Dataclasses
# ============================================================================


class ProbeOutcome(str, Enum):
    """Outcome mode of the LLM endpoint probe."""

    ENDPOINT_REACHABLE = "ENDPOINT_REACHABLE"
    ENDPOINT_UNREACHABLE = "ENDPOINT_UNREACHABLE"
    AUTH_REQUIRED = "AUTH_REQUIRED"
    PARSE_OK = "PARSE_OK"
    PARSE_FAIL = "PARSE_FAIL"
    SKIPPED = "SKIPPED"
    ERROR = "ERROR"


@dataclass
class HttpCall:
    """One bounded HTTP call."""

    url: str
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
    mode: str = "PENDING"  # PENDING / OK / TIMEOUT / DNS_FAIL / CONN_REFUSED / HTTP_ERR / UNKNOWN

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
class EndpointProbeResult:
    """Aggregated LLM endpoint probe result."""

    endpoint_url: str = ""
    probe_outcome: str = ProbeOutcome.ERROR.value
    http_calls: List[HttpCall] = field(default_factory=list)
    models_count: int = -1
    models_sample: List[str] = field(default_factory=list)
    server: Optional[str] = None
    content_type: Optional[str] = None
    started_iso: str = ""
    ended_iso: str = ""
    timeout_seconds: int = DEFAULT_TIMEOUT_SECONDS
    notes: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["http_calls"] = [c.to_dict() for c in self.http_calls]
        return d


# ============================================================================
# GUARDS / BORROWED
# ============================================================================

V1436_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_TIMEOUT",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_URL_VALIDATED",
    "GUARD_NO_API_KEY_REQUIRED",
    "GUARD_RETRY_CAPPED",
    "GUARD_RESPONSE_BOUNDED",
    "GUARD_HEADERS_PARSED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_COMPLETION_CALL",
    "GUARD_NO_PRODUCTION_DEPLOY",
    "GUARD_CLI_RUNNABLE",
)

V1436_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_PROBE",
    "GUARD_NO_ASI_PROBE",
    "GUARD_NO_HUMAN_LEVEL_PROBE",
    "GUARD_NO_ABSOLUTE_PROBE",
    "GUARD_NO_V1424_REPLACE",
)

V1436_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("v1424_asi_real_llm_benchmark", "Provider list + benchmark pattern"),
    ("v1076_asi_real_external_llm_client", "Retry + exponential backoff pattern"),
    ("v1435_asi_docker_availability_probe", "Bounded subprocess/offline-safe pattern"),
    ("stdlib_urllib_request", "Real HTTP GET"),
    ("stdlib_json", "Response body parsing"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def validate_url(url: str) -> bool:
    """Validate URL scheme ∈ {http, https} and host non-empty."""
    if not url or not isinstance(url, str):
        return False
    try:
        parsed = urllib.parse.urlparse(url)
    except Exception:  # noqa: BLE001
        return False
    if parsed.scheme not in ("http", "https"):
        return False
    if not parsed.netloc:
        return False
    return True


def _truncate(s: str, n: int = MAX_BODY_BYTES) -> Tuple[str, bool]:
    """Truncate string to n bytes (UTF-8 safe). Returns (text, truncated)."""
    if not s:
        return "", False
    encoded = s.encode("utf-8", errors="replace")
    if len(encoded) <= n:
        return s, False
    return encoded[:n].decode("utf-8", errors="replace"), True


# ============================================================================
# HTTP runner
# ============================================================================


def http_get(url: str, timeout: int = DEFAULT_TIMEOUT_SECONDS) -> HttpCall:
    """Run one bounded HTTP GET. Never raises; offline-safe."""
    if not (1 <= timeout <= MAX_TIMEOUT_SECONDS):
        timeout = DEFAULT_TIMEOUT_SECONDS
    call = HttpCall(url=url, method="GET")
    start = time.monotonic()
    try:
        req = urllib.request.Request(url, method="GET")
        req.add_header("User-Agent", "V1436-ASI-Endpoint-Probe/0.1")
        # Read with bounded buffer
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            raw = resp.read(MAX_BODY_BYTES + 1)  # read one extra to detect truncation
            elapsed = (time.monotonic() - start) * 1000.0
            call.elapsed_ms = elapsed
            call.status_code = resp.status
            call.server_header = resp.headers.get("Server")
            call.content_type = resp.headers.get("Content-Type")
            truncated = len(raw) > MAX_BODY_BYTES
            if truncated:
                raw = raw[:MAX_BODY_BYTES]
            call.body_bytes = len(raw)
            call.body_truncated = truncated
            text = raw.decode("utf-8", errors="replace")
            call.body_preview = text[:500]
            if 200 <= resp.status < 300:
                call.mode = "OK"
            elif resp.status in (401, 403):
                call.mode = "AUTH_REQUIRED"
            else:
                call.mode = "HTTP_ERR"
    except urllib.error.HTTPError as exc:
        elapsed = (time.monotonic() - start) * 1000.0
        call.elapsed_ms = elapsed
        call.status_code = exc.code
        try:
            err_body = exc.read(MAX_BODY_BYTES)
            call.body_bytes = len(err_body)
            call.body_preview = err_body.decode("utf-8", errors="replace")[:500]
        except Exception:  # noqa: BLE001
            pass
        if exc.headers:
            call.server_header = exc.headers.get("Server")
            call.content_type = exc.headers.get("Content-Type")
        if exc.code in (401, 403):
            call.mode = "AUTH_REQUIRED"
        elif 400 <= exc.code < 500:
            call.mode = "HTTP_ERR"
        else:
            call.mode = "HTTP_ERR"
    except socket.timeout:
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        call.timed_out = True
        call.mode = "TIMEOUT"
    except urllib.error.URLError as exc:
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        reason = str(exc.reason) if hasattr(exc, "reason") else str(exc)
        if "Name or service not known" in reason or "nodename" in reason.lower() or "getaddrinfo" in reason.lower():
            call.mode = "DNS_FAIL"
        elif "Connection refused" in reason:
            call.mode = "CONN_REFUSED"
        else:
            call.mode = "CONN_ERR"
    except Exception as exc:  # noqa: BLE001
        call.elapsed_ms = (time.monotonic() - start) * 1000.0
        call.mode = "UNKNOWN"
        call.body_preview = f"{type(exc).__name__}: {exc}"[:500]
    return call


def http_get_with_retry(url: str, timeout: int = DEFAULT_TIMEOUT_SECONDS, max_attempts: int = MAX_RETRIES) -> HttpCall:
    """Bounded HTTP GET with exponential backoff retry."""
    if not (1 <= max_attempts <= 5):
        max_attempts = MAX_RETRIES
    last = http_get(url, timeout=timeout)
    last.attempt = 1
    if last.mode == "OK" or last.mode == "AUTH_REQUIRED":
        return last
    # Retry on TIMEOUT / DNS_FAIL / CONN_ERR / UNKNOWN
    for attempt in range(2, max_attempts + 1):
        backoff = min(2 ** (attempt - 1), timeout)
        time.sleep(backoff)
        c = http_get(url, timeout=timeout)
        c.attempt = attempt
        if c.mode in ("OK", "AUTH_REQUIRED", "HTTP_ERR"):
            return c
        last = c
    return last


# ============================================================================
# Per-endpoint probes
# ============================================================================


def probe_models_endpoint(base_url: str, timeout: int) -> Tuple[HttpCall, int, List[str]]:
    """Probe /v1/models (OpenAI-compatible). Returns (call, n_models, sample)."""
    url = base_url.rstrip("/") + "/v1/models"
    call = http_get_with_retry(url, timeout=timeout)
    n_models = -1
    sample: List[str] = []
    if call.mode == "OK":
        try:
            data = json.loads(call.body_preview)
            if isinstance(data, dict) and "data" in data and isinstance(data["data"], list):
                items = data["data"]
                n_models = len(items)
                sample = [str(item.get("id", item.get("name", "?"))) for item in items[:5]]
            elif isinstance(data, list):
                n_models = len(data)
                sample = [str(item.get("id", item.get("name", "?"))) for item in data[:5]]
        except Exception:  # noqa: BLE001
            pass
    return call, n_models, sample


def probe_status_endpoint(base_url: str, timeout: int) -> HttpCall:
    """Probe /api/status (Bocha-style)."""
    url = base_url.rstrip("/") + "/api/status"
    return http_get_with_retry(url, timeout=timeout)


def probe_root_endpoint(base_url: str, timeout: int) -> HttpCall:
    """Probe root URL."""
    return http_get_with_retry(base_url, timeout=timeout)


# ============================================================================
# Aggregated probe
# ============================================================================


def run_endpoint_probe(base_url: str, timeout: int = DEFAULT_TIMEOUT_SECONDS) -> EndpointProbeResult:
    """Run full LLM endpoint probe (offline-safe)."""
    timeout = max(1, min(int(timeout or DEFAULT_TIMEOUT_SECONDS), MAX_TIMEOUT_SECONDS))
    result = EndpointProbeResult(
        endpoint_url=base_url,
        timeout_seconds=timeout,
        started_iso=_now_iso(),
    )

    if not validate_url(base_url):
        result.probe_outcome = ProbeOutcome.SKIPPED.value
        result.notes.append(f"invalid URL: {base_url}")
        result.ended_iso = _now_iso()
        return result

    # 1) /v1/models (primary)
    models_call, n_models, sample = probe_models_endpoint(base_url, timeout)
    models_call.method = "GET /v1/models"
    result.http_calls.append(models_call)
    result.models_count = n_models
    result.models_sample = sample

    # 2) /api/status (secondary)
    status_call = probe_status_endpoint(base_url, timeout)
    status_call.method = "GET /api/status"
    result.http_calls.append(status_call)

    # 3) root (tertiary)
    root_call = probe_root_endpoint(base_url, timeout)
    root_call.method = "GET /"
    result.http_calls.append(root_call)

    # Determine outcome
    primary = result.http_calls[0]
    if primary.mode == "OK" and n_models >= 0:
        result.probe_outcome = ProbeOutcome.PARSE_OK.value if sample else ProbeOutcome.ENDPOINT_REACHABLE.value
        result.server = primary.server_header
        result.content_type = primary.content_type
        result.notes.append(f"/v1/models OK: n_models={n_models}")
    elif primary.mode == "OK":
        result.probe_outcome = ProbeOutcome.ENDPOINT_REACHABLE.value
        result.server = primary.server_header
        result.content_type = primary.content_type
        result.notes.append("/v1/models OK but body not parseable as OpenAI /v1/models")
    elif primary.mode == "AUTH_REQUIRED":
        result.probe_outcome = ProbeOutcome.AUTH_REQUIRED.value
        result.server = primary.server_header
        result.notes.append("/v1/models returns 401/403 — API key required")
    elif primary.mode in ("TIMEOUT", "DNS_FAIL", "CONN_REFUSED", "CONN_ERR", "UNKNOWN"):
        result.probe_outcome = ProbeOutcome.ENDPOINT_UNREACHABLE.value
        result.notes.append(f"/v1/models mode={primary.mode} — endpoint not reachable")
    elif primary.mode == "HTTP_ERR":
        result.probe_outcome = ProbeOutcome.PARSE_FAIL.value
        result.server = primary.server_header
        result.notes.append(f"/v1/models HTTP {primary.status_code} — unexpected response")
    else:
        result.probe_outcome = ProbeOutcome.ERROR.value
        result.notes.append("unknown state")

    result.ended_iso = _now_iso()
    return result


# ============================================================================
# Render / Serialize
# ============================================================================


def result_to_dict(result: EndpointProbeResult) -> Dict[str, Any]:
    return result.to_dict()


def render_probe_summary_md(result: EndpointProbeResult) -> str:
    lines = []
    lines.append(f"# V1436 ASI LLM Endpoint Live Probe")
    lines.append("")
    lines.append(f"- **endpoint_url**: `{result.endpoint_url}`")
    lines.append(f"- **probe_outcome**: `{result.probe_outcome}`")
    lines.append(f"- **timeout_seconds**: {result.timeout_seconds}")
    lines.append(f"- **started**: {result.started_iso}")
    lines.append(f"- **ended**: {result.ended_iso}")
    lines.append("")
    lines.append("## Models")
    lines.append("")
    lines.append(f"- models_count: `{result.models_count}`")
    if result.models_sample:
        lines.append(f"- models_sample: `{', '.join(result.models_sample)}`")
    lines.append("")
    lines.append("## Server")
    lines.append("")
    lines.append(f"- server: `{result.server or 'UNKNOWN'}`")
    lines.append(f"- content_type: `{result.content_type or 'UNKNOWN'}`")
    lines.append("")
    lines.append("## HTTP calls")
    lines.append("")
    lines.append("| # | method | url | status | mode | elapsed_ms | timed_out |")
    lines.append("|---|--------|-----|--------|------|-----------|-----------|")
    for i, c in enumerate(result.http_calls, 1):
        url_short = c.url.replace("https://", "").replace("http://", "")
        if len(url_short) > 50:
            url_short = url_short[:47] + "..."
        lines.append(f"| {i} | {c.method} | `{url_short}` | {c.status_code} | {c.mode} | {c.elapsed_ms:.1f} | {c.timed_out} |")
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
        "V1436 is an **endpoint reachability probe**. It does NOT claim that "
        "the endpoint is production-deployed, that the model will return "
        "correct answers, that the API key is valid, or that responses are "
        "reproducible. It claims only: **from this host, with the configured "
        "URL, here is what the bounded HTTP calls returned**. V1436 ≠ "
        "Phenomenal probe, ≠ ASI probe, ≠ human-level probe, ≠ absolute "
        "probe, ≠ real completion call. Probe ≠ chat. Probe ≠ ASI 达成."
    )
    return "\n".join(lines)


# ============================================================================
# Chain + Popper
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe upstream V1435 + V1424 + V1076 chain integrity."""
    out: Dict[str, Any] = {
        "v1436": {"ok": True, "mode": "v1436_asi_llm_endpoint_live_probe"},
        "v1435": {"ok": True, "mode": "v1435_asi_docker_availability_probe"},
        "v1424": {"ok": True, "mode": "v1424_asi_real_llm_benchmark"},
        "v1076": {"ok": True, "mode": "v1076_asi_real_external_llm_client"},
        "all_ok": True,
        "borrowed": list(V1436_BORROWED),
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
        import apeireth.v1424_asi_real_llm_benchmark as v1424
        out["v1424"]["importable"] = True
    except Exception as exc:  # noqa: BLE001
        out["v1424"]["ok"] = False
        out["v1424"]["error"] = f"{type(exc).__name__}: {exc}"
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
        "ok": V1436_VERSION == "0.1.0" and DEFAULT_TIMEOUT_SECONDS > 0 and MAX_BODY_BYTES > 0 and MAX_RETRIES >= 1,
    })

    # 2. guards count
    results.append({
        "id": "P02",
        "name": "guards count",
        "ok": len(V1436_GUARDS) == 14 and len(V1436_V3_GUARDS) == 5,
    })

    # 3. borrowed count
    results.append({
        "id": "P03",
        "name": "borrowed count",
        "ok": len(V1436_BORROWED) == 5,
    })

    # 4. probe outcomes
    results.append({
        "id": "P04",
        "name": "probe outcomes count",
        "ok": len(list(ProbeOutcome)) == 7,
    })

    # 5. validate_url accepts http/https
    results.append({
        "id": "P05",
        "name": "validate_url accepts http/https",
        "ok": validate_url("https://api.example.com")
        and validate_url("http://localhost:11434")
        and not validate_url("")
        and not validate_url("ftp://x")
        and not validate_url("not-a-url"),
    })

    # 6. http_get on bad URL → offline-safe
    c = http_get("http://this-host-definitely-does-not-exist-xyz-1436.invalid", timeout=2)
    results.append({
        "id": "P06",
        "name": "http_get offline-safe on bad URL",
        "ok": c.mode in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN") and c.timed_out is False,
    })

    # 7. http_get retries: bad URL still doesn't raise
    c2 = http_get_with_retry("http://nope-1436.invalid", timeout=1, max_attempts=2)
    results.append({
        "id": "P07",
        "name": "http_get_with_retry offline-safe",
        "ok": c2.mode in ("DNS_FAIL", "CONN_ERR", "TIMEOUT", "UNKNOWN"),
    })

    # 8. truncate handles empty
    results.append({
        "id": "P08",
        "name": "_truncate handles empty",
        "ok": _truncate("", 100) == ("", False),
    })

    # 9. truncate handles long
    long_text = "x" * 1000
    out, trunc = _truncate(long_text, 100)
    results.append({
        "id": "P09",
        "name": "_truncate truncates long text",
        "ok": trunc is True and len(out) == 100,
    })

    # 10. HttpCall dataclass
    hc = HttpCall(url="http://x", status_code=200, elapsed_ms=12.0)
    results.append({
        "id": "P10",
        "name": "HttpCall dataclass",
        "ok": hc.to_dict()["status_code"] == 200 and hc.to_dict()["elapsed_ms"] == 12.0,
    })

    # 11. EndpointProbeResult dataclass
    er = EndpointProbeResult(endpoint_url="http://x", probe_outcome="PARSE_OK", models_count=3)
    results.append({
        "id": "P11",
        "name": "EndpointProbeResult dataclass",
        "ok": er.to_dict()["models_count"] == 3 and er.to_dict()["probe_outcome"] == "PARSE_OK",
    })

    # 12. run_endpoint_probe on bad URL → SKIPPED
    r = run_endpoint_probe("not-a-url", timeout=2)
    results.append({
        "id": "P12",
        "name": "run_endpoint_probe invalid URL → SKIPPED",
        "ok": r.probe_outcome == ProbeOutcome.SKIPPED.value,
    })

    # 13. render markdown
    r2 = EndpointProbeResult(endpoint_url="http://x", probe_outcome="ENDPOINT_REACHABLE", models_count=5)
    md = render_probe_summary_md(r2)
    results.append({
        "id": "P13",
        "name": "render_probe_summary_md emits honest disclosure",
        "ok": "V1436" in md and "Honest disclosure" in md and "PARSE_OK" not in md,  # since we set ENDPOINT_REACHABLE
    })

    # 14. result_to_dict JSON-serializable
    r3 = EndpointProbeResult(endpoint_url="http://x", probe_outcome="ENDPOINT_REACHABLE", models_count=7)
    r3.http_calls.append(HttpCall(url="http://x/v1/models", status_code=200, mode="OK", elapsed_ms=10))
    j = json.dumps(result_to_dict(r3))
    results.append({
        "id": "P14",
        "name": "result_to_dict JSON-serializable with calls",
        "ok": "ENDPOINT_REACHABLE" in j and "v1/models" in j and "7" in j,
    })

    ok_count = sum(1 for r in results if r["ok"])
    return {"passed": ok_count, "total": len(results), "results": results}


def module_meta() -> Dict[str, Any]:
    return {
        "module": V1436_MODULE,
        "version": V1436_VERSION,
        "schema": V1436_SCHEMA,
        "guards": list(V1436_GUARDS),
        "v3_guards": list(V1436_V3_GUARDS),
        "borrowed": [{"module": m, "use": u} for m, u in V1436_BORROWED],
        "endpoints": ("/v1/models", "/api/status", "/"),
        "default_timeout": DEFAULT_TIMEOUT_SECONDS,
        "max_timeout": MAX_TIMEOUT_SECONDS,
        "max_body_bytes": MAX_BODY_BYTES,
        "max_retries": MAX_RETRIES,
        "default_endpoints": list(DEFAULT_ENDPOINTS),
        "probe_outcomes": [o.value for o in ProbeOutcome],
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
        _print(V1436_VERSION)
        return 0

    if cmd == "meta":
        if len(args) > 1 and args[1] == "--json":
            _print(json.dumps(module_meta(), indent=2))
        else:
            _print(f"V1436 version {V1436_VERSION}")
            _print(f"schema: {V1436_SCHEMA}")
            _print(f"guards: {len(V1436_GUARDS)} (V3: {len(V1436_V3_GUARDS)})")
            _print(f"borrowed: {len(V1436_BORROWED)}")
            _print(f"default_endpoints: {len(DEFAULT_ENDPOINTS)}")
        return 0

    if cmd == "popper":
        out = popper_self_test()
        _print(json.dumps(out, indent=2))
        return 0 if out["passed"] == out["total"] else 1

    if cmd == "chain":
        _print(json.dumps(chain_delegate(), indent=2))
        return 0

    if cmd == "probe":
        if "--url" not in args:
            _print("usage: probe --url URL [--timeout SECONDS]")
            return 2
        i = args.index("--url")
        if i + 1 >= len(args):
            _print("missing --url value")
            return 2
        url = args[i + 1]
        timeout = DEFAULT_TIMEOUT_SECONDS
        if "--timeout" in args:
            ti = args.index("--timeout")
            if ti + 1 < len(args):
                try:
                    timeout = int(args[ti + 1])
                except ValueError:
                    pass
        result = run_endpoint_probe(url, timeout=timeout)
        _print(render_probe_summary_md(result))
        return 0

    if cmd == "json":
        if "--url" not in args:
            _print("usage: json --url URL [--timeout SECONDS]")
            return 2
        i = args.index("--url")
        if i + 1 >= len(args):
            _print("missing --url value")
            return 2
        url = args[i + 1]
        timeout = DEFAULT_TIMEOUT_SECONDS
        if "--timeout" in args:
            ti = args.index("--timeout")
            if ti + 1 < len(args):
                try:
                    timeout = int(args[ti + 1])
                except ValueError:
                    pass
        result = run_endpoint_probe(url, timeout=timeout)
        _print(json.dumps(result_to_dict(result), indent=2))
        return 0

    if cmd == "call":
        if "--url" not in args:
            _print("usage: call --url URL [--timeout SECONDS]")
            return 2
        i = args.index("--url")
        if i + 1 >= len(args):
            _print("missing --url value")
            return 2
        url = args[i + 1]
        timeout = DEFAULT_TIMEOUT_SECONDS
        if "--timeout" in args:
            ti = args.index("--timeout")
            if ti + 1 < len(args):
                try:
                    timeout = int(args[ti + 1])
                except ValueError:
                    pass
        call = http_get_with_retry(url, timeout=timeout)
        _print(json.dumps(call.to_dict(), indent=2))
        return 0

    _print(f"unknown command: {cmd}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
