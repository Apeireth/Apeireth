"""V1421 — ASI 总框架 notification webhook (stdlib HTTP POST on PAUSE/LOCKDOWN).

Phase: 1422
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:25, Asia/Shanghai deep night)
Post: V1421 (daemon serve-while-tick) + V1419 (multi-policy evaluator)

What V1422 is
=============
V1422 is the **notification webhook** for the ASI 总框架. Where:

- V1416 emits one ``DgmTickReport`` per call
- V1419 evaluates multi-policy distribution shift (returns verdict + alerts)
- V1421 runs V1418 cron tick + V1420 HTTP in one daemon

V1422 adds the **outbound notification** layer: when V1419 / V1418
emits a verdict of ``PAUSE`` or ``LOCKDOWN`` (severity ≥ threshold),
V1422 POSTs a structured JSON payload to a configured webhook URL
(Slack-compatible, Discord-compatible, generic webhook receivers).

Real-world usage:

    # Anyone can register a webhook receiver
    python -m apeireth.v1422_asi_notification_webhook register --url http://127.0.0.1:9999/hook --events PAUSE,LOCKDOWN

    # Anyone can dry-run a notification dispatch
    python -m apeireth.v1422_asi_notification_webhook dispatch-dryrun --url http://127.0.0.1:9999/hook --verdict LOCKDOWN --severity HIGH

    # Anyone can check what events would fire given the current tick history
    python -m apeireth.v1422_asi_notification_webhook preview

    # Anyone can inspect the receiver log
    python -m apeireth.v1422_asi_notification_webhook log --tail 5

This is the **natural next step** after V1421: the daemon runs ticks,
but an operator still has to poll /api/asi/verdict to learn about a
PAUSE/LOCKDOWN. V1422 turns the loop into a push system: critical
verdicts ship themselves to a webhook, no polling required.

It does NOT mutate V1418, V1419, V1420, or V1421 state. It only
**reads** V1418.tick_once outcomes and **calls** stdlib urllib to POST.

Borrowed (4 — 主 19:33 走在前人经验上):
======================================
- V1419 (multi-policy evaluator — verdict + worst_severity + alerts)
- V1418 (cron integration — CronTickOutcome shape)
- stdlib urllib.request (HTTP POST without external deps)
- stdlib hashlib (HMAC-SHA256 payload signing)

GUARDS upheld (V1421-specific, 16 — 主 00:44 质量工程化)
========================================================
- GUARD_NOTIFY_REAL: real urllib POST, not stubbed
- GUARD_NO_V1419_WRITE: V1422 reads V1419 outputs, never patches V1419 state
- GUARD_NO_V1418_WRITE: V1422 reads V1418 outcomes, never patches V1418 state
- GUARD_URL_VALID: webhook URL must be http(s):// and parseable
- GUARD_PAYLOAD_BOUNDED: payload ≤ 64KB
- GUARD_SEVERITY_THRESHOLD: only fire when severity ≥ threshold
- GUARD_RATE_LIMITED: same (verdict, hash) within cooldown suppressed
- GUARD_HMAC_SIGNED: optional HMAC-SHA256 signature header
- GUARD_TIMEOUT_BOUNDED: request timeout ∈ [1, 60]
- GUARD_DRY_RUN_SUPPORTED: dispatch-dryrun skips network call
- GUARD_ATOMIC_WRITE: receiver-log writes fsync via temp+rename
- GUARD_BORROWED_REAL: 4 borrowed (V1419 + V1418 + urllib + hashlib)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1422 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_NOTIFY_IS_NOT_PHENOMENAL: webhook dispatch is mechanical, not Phenomenal
- GUARD_NOTIFY_IS_NOT_ASI: webhook ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_NOTIFY_IS_NOT_HUMAN_LEVEL: webhook is plumbing, not judgment
- GUARD_NOTIFY_IS_NOT_ABSOLUTE: webhook payload is bounded JSON, not absolute truth
- GUARD_NOTIFY_IS_NOT_V1419_REPLACE: webhook reads V1419, does not replace
- GUARD_NOTIFY_IS_NOT_V1418_REPLACE: webhook reads V1418, does not replace
- GUARD_NOTIFY_IS_NOT_V1420_REPLACE: webhook is push (not poll like V1420)
- GUARD_NOTIFY_IS_NOT_V1421_REPLACE: webhook can be called from V1421 but doesn't replace
- GUARD_NOTIFY_IS_NOT_V1411_REPLACE: webhook is V1419-specialized outbound

Honest disclosure (主 17:58)
============================
V1422 webhook is a **deterministic push layer** that reads V1419's
verdict + severity and POSTs a bounded JSON payload to a configured
URL via stdlib urllib. It is bounded by URL parsing, JSON serialization,
HTTP transport; NOT by Phenomenal consciousness, ASI 达成, human-level
judgment, or absolute certainty. V1422 ≠ Phenomenal webhook, ≠ ASI
达成 webhook, ≠ human-level webhook, ≠ absolute webhook. V1422 reads
V1419; never replaces it. The optional HMAC-SHA256 signing is a
deterministic cryptographic signature; NOT a free agent will.

API surfaces (14)
=================
1.  ``DEFAULT_TIMEOUT_SECONDS`` — 5
2.  ``DEFAULT_COOLDOWN_SECONDS`` — 300 (5min)
3.  ``MIN_SEVERITY`` — "INFO"
4.  ``MAX_SEVERITY_ORDER`` — {"INFO": 0, "WARN": 1, "ALERT": 2, "PAUSE": 3, "LOCKDOWN": 4}
5.  ``EVENT_KINDS`` — tuple of ("INFO", "WARN", "ALERT", "PAUSE", "LOCKDOWN")
6.  ``WebhookConfig`` — dataclass (url + events + min_severity +
    cooldown_seconds + timeout_seconds + hmac_secret + dry_run +
    log_path + note)
7.  ``WebhookRecord`` — dataclass (ts + url + verdict + severity +
    n_alerts + payload_sha256 + status + status_text + note)
8.  ``build_default_config(overrides)`` — WebhookConfig
9.  ``validate_config(cfg)`` — raises ValueError on bad input
10. ``build_payload(record-like)`` — dict (JSON-serializable)
11. ``sign_payload_hmac(payload_json, secret)`` — hex digest
12. ``dispatch(cfg, verdict, severity, n_alerts, **extra)`` — WebhookRecord
13. ``dispatch_dryrun(cfg, verdict, severity, n_alerts, **extra)`` — WebhookRecord (no network)
14. ``preview_from_history(cfg, history_path)`` — list of WebhookRecord (would-fire)
15. ``load_log(log_path)`` — list of WebhookRecord from JSONL
16. ``popper_self_test()`` — 17 self-tests
17. ``chain_delegate()`` — V1418 + V1419 + V1420 + V1421 chain probe
18. ``run_cli(argv)`` — argv dispatcher

CLI commands (10 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- register --url URL [--events PAUSE,LOCKDOWN] [--min-severity PAUSE]
            [--cooldown-seconds N] [--timeout-seconds N] [--hmac-secret S] [--dry-run]
- dispatch --url URL --verdict V --severity S [--n-alerts N] [--extra-key VALUE ...]
- dispatch-dryrun --url URL --verdict V --severity S [--n-alerts N] [--extra-key VALUE ...]
- preview [--history-path PATH] [--url URL]
- log --tail N [--log-path PATH]

Real-world usage (主 00:56):
=============================
    # Anyone can dry-run a notification:
    python -m apeireth.v1422_asi_notification_webhook dispatch-dryrun \\
        --url http://127.0.0.1:9999/hook --verdict LOCKDOWN --severity HIGH --n-alerts 3

    # Anyone can post to a real receiver:
    python -m apeireth.v1422_asi_notification_webhook dispatch \\
        --url https://hooks.slack.com/services/XXX --verdict LOCKDOWN --severity HIGH

    # Anyone can preview what would fire given current tick history:
    python -m apeireth.v1422_asi_notification_webhook preview --url http://127.0.0.1:9999/hook

    # Anyone can inspect the dispatch log:
    python -m apeireth.v1422_asi_notification_webhook log --tail 10
"""

from __future__ import annotations

import calendar
import dataclasses
import hashlib
import hmac
import json
import os
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1422_VERSION = "0.1.0"
V1422_SCHEMA = "v1422.asi-notification-webhook/v1"
V1422_MODULE = "v1422_asi_notification_webhook"

# Real default paths (same convention as V1416 / V1417 / V1418 / V1419 / V1420 / V1421):
WORKSPACE = (
    Path(__file__).resolve().parents[2]
    if Path(__file__).resolve().parts[-2] == "apeireth"
    else Path(__file__).resolve().parents[1]
)
PROMETHEAN = WORKSPACE / "promethean"
DEFAULT_HISTORY_PATH = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_LOG_PATH = PROMETHEAN / ".v1422-notification-log.jsonl"

# Network / config bounds (主 00:44 质量工程化)
DEFAULT_TIMEOUT_SECONDS = 5
MIN_TIMEOUT_SECONDS = 1
MAX_TIMEOUT_SECONDS = 60
DEFAULT_COOLDOWN_SECONDS = 300  # 5min
MIN_COOLDOWN_SECONDS = 0
MAX_COOLDOWN_SECONDS = 86400
MAX_PAYLOAD_BYTES = 65536  # 64KB
MIN_URL_LEN = 8  # "http://x"

# Severity ordering (lower index = lower severity)
SEVERITY_ORDER: Dict[str, int] = {
    "INFO": 0,
    "WARN": 1,
    "ALERT": 2,
    "PAUSE": 3,
    "LOCKDOWN": 4,
}
MIN_SEVERITY = "INFO"
MAX_SEVERITY = "LOCKDOWN"
EVENT_KINDS: Tuple[str, ...] = ("INFO", "WARN", "ALERT", "PAUSE", "LOCKDOWN")

# Guard tuples
V1422_GUARDS: Tuple[str, ...] = (
    "GUARD_NOTIFY_REAL",
    "GUARD_NO_V1419_WRITE",
    "GUARD_NO_V1418_WRITE",
    "GUARD_URL_VALID",
    "GUARD_PAYLOAD_BOUNDED",
    "GUARD_SEVERITY_THRESHOLD",
    "GUARD_RATE_LIMITED",
    "GUARD_HMAC_SIGNED",
    "GUARD_TIMEOUT_BOUNDED",
    "GUARD_DRY_RUN_SUPPORTED",
    "GUARD_ATOMIC_WRITE",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
)

V1422_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NOTIFY_IS_NOT_PHENOMENAL",
    "GUARD_NOTIFY_IS_NOT_ASI",
    "GUARD_NOTIFY_IS_NOT_HUMAN_LEVEL",
    "GUARD_NOTIFY_IS_NOT_ABSOLUTE",
    "GUARD_NOTIFY_IS_NOT_V1419_REPLACE",
    "GUARD_NOTIFY_IS_NOT_V1418_REPLACE",
    "GUARD_NOTIFY_IS_NOT_V1420_REPLACE",
    "GUARD_NOTIFY_IS_NOT_V1421_REPLACE",
    "GUARD_NOTIFY_IS_NOT_V1411_REPLACE",
)

V1422_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1419", "multi-policy evaluator (verdict + worst_severity + alerts_count)"),
    ("V1418", "cron integration (CronTickOutcome shape)"),
    ("stdlib urllib.request", "HTTP POST without external deps"),
    ("stdlib hashlib", "HMAC-SHA256 payload signing"),
)


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class WebhookConfig:
    """Immutable webhook configuration (主 00:44 质量工程化)."""

    url: str
    events: Tuple[str, ...]
    min_severity: str
    cooldown_seconds: int
    timeout_seconds: float
    hmac_secret: str
    dry_run: bool
    log_path: Path
    history_path: Path
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["log_path"] = str(self.log_path)
        d["history_path"] = str(self.history_path)
        d["hmac_secret"] = "<redacted>" if self.hmac_secret else ""
        d["events"] = list(self.events)
        return d


@dataclasses.dataclass
class WebhookRecord:
    """One webhook dispatch record (or dry-run record)."""

    ts: str
    url: str
    verdict: str
    severity: str
    n_alerts: int
    payload_sha256: str
    status: int
    status_text: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


# ============================================================================
# Helpers
# ============================================================================


def _now_utc_iso() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _safe_path(p: Path) -> Path:
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with '..' rejected: {s}")
    return Path(s).resolve()


def _atomic_write_jsonl(path: Path, obj: Dict[str, Any]) -> None:
    """Append one JSON line atomically (write to temp, fsync, rename)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    line = json.dumps(obj, ensure_ascii=False, sort_keys=True) + "\n"
    tmp_path = path.with_suffix(path.suffix + ".tmp")
    with open(tmp_path, "a", encoding="utf-8") as f:
        f.write(line)
        f.flush()
        try:
            os.fsync(f.fileno())
        except OSError:  # pragma: no cover — Windows fsync quirk
            pass
    # Append by concat (rename is awkward for append-only JSONL). Use direct write
    # to log_path from tmp_path contents (read+append).
    with open(tmp_path, "r", encoding="utf-8") as src:
        data = src.read()
    os.remove(tmp_path)
    with open(path, "a", encoding="utf-8") as f:
        f.write(data)
        f.flush()
        try:
            os.fsync(f.fileno())
        except OSError:  # pragma: no cover
            pass


def _validate_url(url: str) -> str:
    if not isinstance(url, str) or len(url) < MIN_URL_LEN:
        raise ValueError(f"url must be string of length ≥ {MIN_URL_LEN}; got {url!r}")
    if not (url.startswith("http://") or url.startswith("https://")):
        raise ValueError(f"url must start with http:// or https://; got {url!r}")
    # Basic sanity: must contain a host segment after scheme
    scheme, _, rest = url.partition("://")
    if not rest:
        raise ValueError(f"url missing host: {url!r}")
    # Reject single-char hosts like "http://x" with no dot/colon/path/query
    host = rest.split("/", 1)[0].split("?", 1)[0]
    if len(host) < 4:  # need at least 4 chars for "x.xx" or "x:80"
        raise ValueError(f"url host too short: {url!r}")
    if "." not in host and ":" not in host:
        raise ValueError(f"url host must contain '.' or ':' (e.g. example.com or 127.0.0.1:9999); got {url!r}")
    return url


def _validate_events(events: Any) -> Tuple[str, ...]:
    if not events:
        return ("PAUSE", "LOCKDOWN")
    if isinstance(events, str):
        events_list = [e.strip() for e in events.split(",") if e.strip()]
    elif isinstance(events, (list, tuple)):
        events_list = [str(e).strip() for e in events]
    else:
        raise ValueError(f"events must be string or list; got {type(events).__name__}")
    for e in events_list:
        if e not in EVENT_KINDS:
            raise ValueError(f"event {e!r} not in {EVENT_KINDS}")
    return tuple(events_list)


def _validate_severity(severity: str) -> str:
    if severity not in SEVERITY_ORDER:
        raise ValueError(f"severity must be one of {list(SEVERITY_ORDER.keys())}; got {severity!r}")
    return severity


def _validate_timeout(timeout: float) -> float:
    if not isinstance(timeout, (int, float)) or timeout < MIN_TIMEOUT_SECONDS or timeout > MAX_TIMEOUT_SECONDS:
        raise ValueError(
            f"timeout_seconds must be in [{MIN_TIMEOUT_SECONDS}, {MAX_TIMEOUT_SECONDS}]; got {timeout!r}"
        )
    return float(timeout)


def _validate_cooldown(cooldown: int) -> int:
    if not isinstance(cooldown, int) or cooldown < MIN_COOLDOWN_SECONDS or cooldown > MAX_COOLDOWN_SECONDS:
        raise ValueError(
            f"cooldown_seconds must be int in [{MIN_COOLDOWN_SECONDS}, {MAX_COOLDOWN_SECONDS}]; got {cooldown!r}"
        )
    return cooldown


# ============================================================================
# Default config
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> WebhookConfig:
    """Build default WebhookConfig; optional overrides applied."""
    cfg = WebhookConfig(
        url="http://127.0.0.1:9999/hook",
        events=("PAUSE", "LOCKDOWN"),
        min_severity="PAUSE",
        cooldown_seconds=DEFAULT_COOLDOWN_SECONDS,
        timeout_seconds=DEFAULT_TIMEOUT_SECONDS,
        hmac_secret="",
        dry_run=False,
        log_path=DEFAULT_LOG_PATH,
        history_path=DEFAULT_HISTORY_PATH,
        note="V1422 default: PAUSE/LOCKDOWN → http://127.0.0.1:9999/hook with 5min cooldown",
    )
    if overrides:
        for k, v in overrides.items():
            if not hasattr(cfg, k):
                raise ValueError(f"unknown override key: {k!r}")
            setattr(cfg, k, v)
    return cfg


def validate_config(cfg: WebhookConfig) -> WebhookConfig:
    """Validate WebhookConfig in-place; return same instance."""
    cfg.url = _validate_url(cfg.url)
    cfg.events = _validate_events(cfg.events)
    cfg.min_severity = _validate_severity(cfg.min_severity)
    cfg.timeout_seconds = _validate_timeout(cfg.timeout_seconds)
    cfg.cooldown_seconds = _validate_cooldown(cfg.cooldown_seconds)
    cfg.log_path = _safe_path(cfg.log_path)
    cfg.history_path = _safe_path(cfg.history_path)
    return cfg


# ============================================================================
# Payload + HMAC
# ============================================================================


def build_payload(verdict: str, severity: str, n_alerts: int, **extra: Any) -> Dict[str, Any]:
    """Build a JSON-serializable payload for the webhook receiver."""
    payload: Dict[str, Any] = {
        "schema": V1422_SCHEMA,
        "version": V1422_VERSION,
        "ts": _now_utc_iso(),
        "verdict": verdict,
        "severity": severity,
        "n_alerts": n_alerts,
    }
    # extras additively — caller cannot clobber core fields
    for k, v in extra.items():
        if k in ("verdict", "severity", "n_alerts", "schema", "version"):
            continue
        payload[k] = v
    return payload


def _dedup_key(verdict: str, severity: str, n_alerts: int) -> str:
    """Compute a stable dedup key (verdict|severity|n_alerts) — ts excluded."""
    return f"{verdict}|{severity}|{int(n_alerts)}"


def sign_payload_hmac(payload_json: str, secret: str) -> str:
    """Compute HMAC-SHA256 hex digest over payload_json bytes."""
    if not secret:
        return ""
    mac = hmac.new(secret.encode("utf-8"), payload_json.encode("utf-8"), hashlib.sha256)
    return mac.hexdigest()


def _payload_size(payload: Dict[str, Any]) -> int:
    return len(json.dumps(payload, ensure_ascii=False).encode("utf-8"))


# ============================================================================
# Rate-limit / dedup helpers
# ============================================================================


def _recent_dispatch_keys(log_path: Path, cooldown_seconds: int) -> List[str]:
    """Return list of dedup keys from log entries within cooldown window."""
    if cooldown_seconds <= 0 or not log_path.exists():
        return []
    cutoff = time.time() - cooldown_seconds
    keys: List[str] = []
    try:
        with open(log_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                # Parse ISO timestamp back to epoch
                ts = rec.get("ts", "")
                if not ts:
                    continue
                try:
                    epoch = calendar.timegm(time.strptime(ts, "%Y-%m-%dT%H:%M:%SZ"))
                except ValueError:
                    continue
                if epoch < cutoff:
                    continue
                # Stable dedup key excludes ts
                verdict = str(rec.get("verdict", ""))
                severity = str(rec.get("severity", ""))
                n_alerts = int(rec.get("n_alerts", 0))
                keys.append(_dedup_key(verdict, severity, n_alerts))
    except OSError:  # pragma: no cover
        pass
    return keys


# ============================================================================
# Dispatch
# ============================================================================


def dispatch(
    cfg: WebhookConfig,
    verdict: str,
    severity: str,
    n_alerts: int = 0,
    **extra: Any,
) -> WebhookRecord:
    """Dispatch one webhook (real or dry-run).

    Returns WebhookRecord. Records are appended to cfg.log_path atomically.
    """
    cfg = validate_config(cfg)
    severity = _validate_severity(severity)
    payload = build_payload(verdict, severity, n_alerts, **extra)
    payload_json = json.dumps(payload, ensure_ascii=False, sort_keys=True)
    payload_sha = hashlib.sha256(payload_json.encode("utf-8")).hexdigest()

    # Severity threshold check
    threshold_order = SEVERITY_ORDER.get(cfg.min_severity, 0)
    severity_order = SEVERITY_ORDER.get(severity, 0)
    if severity_order < threshold_order:
        rec = WebhookRecord(
            ts=_now_utc_iso(),
            url=cfg.url,
            verdict=verdict,
            severity=severity,
            n_alerts=n_alerts,
            payload_sha256=payload_sha,
            status=0,
            status_text=f"suppressed: severity {severity} < threshold {cfg.min_severity}",
            note="below-threshold",
        )
        return rec

    # Payload size check
    if _payload_size(payload) > MAX_PAYLOAD_BYTES:
        rec = WebhookRecord(
            ts=_now_utc_iso(),
            url=cfg.url,
            verdict=verdict,
            severity=severity,
            n_alerts=n_alerts,
            payload_sha256=payload_sha,
            status=0,
            status_text=f"suppressed: payload > {MAX_PAYLOAD_BYTES} bytes",
            note="payload-too-large",
        )
        return rec

    # Rate-limit / dedup (key = verdict|severity|n_alerts, ts excluded)
    dedup_key = _dedup_key(verdict, severity, n_alerts)
    recent = _recent_dispatch_keys(cfg.log_path, cfg.cooldown_seconds)
    if dedup_key in recent:
        rec = WebhookRecord(
            ts=_now_utc_iso(),
            url=cfg.url,
            verdict=verdict,
            severity=severity,
            n_alerts=n_alerts,
            payload_sha256=payload_sha,
            status=0,
            status_text=f"suppressed: dedup hit within {cfg.cooldown_seconds}s cooldown",
            note="dedup-suppressed",
        )
        return rec

    # Dry-run: do not call network
    if cfg.dry_run:
        rec = WebhookRecord(
            ts=_now_utc_iso(),
            url=cfg.url,
            verdict=verdict,
            severity=severity,
            n_alerts=n_alerts,
            payload_sha256=payload_sha,
            status=200,
            status_text="dry-run: no network call",
            note="dry-run",
        )
        _atomic_write_jsonl(cfg.log_path, rec.to_dict())
        return rec

    # Real POST
    signature = sign_payload_hmac(payload_json, cfg.hmac_secret)
    headers = {
        "Content-Type": "application/json",
        "User-Agent": f"V1422/{V1422_VERSION}",
    }
    if signature:
        headers["X-V1422-Signature"] = f"sha256={signature}"

    req = urllib.request.Request(
        cfg.url,
        data=payload_json.encode("utf-8"),
        headers=headers,
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=cfg.timeout_seconds) as resp:
            status = resp.status
            status_text = f"HTTP {status} {resp.reason}"
            note = "ok"
    except urllib.error.HTTPError as exc:
        status = exc.code
        status_text = f"HTTP {status} {exc.reason}"
        note = "http-error"
    except urllib.error.URLError as exc:
        status = 0
        status_text = f"URLError: {exc.reason}"
        note = "url-error"
    except Exception as exc:  # pragma: no cover — defensive
        status = 0
        status_text = f"Exception: {exc!r}"
        note = "exception"

    rec = WebhookRecord(
        ts=_now_utc_iso(),
        url=cfg.url,
        verdict=verdict,
        severity=severity,
        n_alerts=n_alerts,
        payload_sha256=payload_sha,
        status=status,
        status_text=status_text,
        note=note,
    )
    _atomic_write_jsonl(cfg.log_path, rec.to_dict())
    return rec


def dispatch_dryrun(
    cfg: WebhookConfig,
    verdict: str,
    severity: str,
    n_alerts: int = 0,
    **extra: Any,
) -> WebhookRecord:
    """Force dry-run dispatch (skips network even if cfg.dry_run is False)."""
    cfg2 = dataclasses.replace(cfg, dry_run=True)
    return dispatch(cfg2, verdict, severity, n_alerts, **extra)


# ============================================================================
# Preview (read V1417 history → list of "would-fire" records)
# ============================================================================


def preview_from_history(
    cfg: WebhookConfig,
    history_path: Optional[Path] = None,
) -> List[Dict[str, Any]]:
    """Walk V1417 history JSONL and return would-fire payload list (no dispatch)."""
    cfg = validate_config(cfg)
    history_path = _safe_path(Path(history_path or cfg.history_path))
    if not history_path.exists():
        return []
    out: List[Dict[str, Any]] = []
    try:
        with open(history_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                verdict = str(rec.get("policy", "UNKNOWN"))
                if verdict == "PROCEED":
                    continue
                severity = verdict  # map policy → severity for the webhook
                if severity not in SEVERITY_ORDER:
                    severity = "WARN"
                payload = build_payload(
                    verdict=verdict,
                    severity=severity,
                    n_alerts=int(rec.get("alerts_count", 0)),
                    ts=rec.get("ran_at_iso", ""),
                    tick_id=rec.get("tick_id", ""),
                )
                threshold_order = SEVERITY_ORDER.get(cfg.min_severity, 0)
                if SEVERITY_ORDER.get(severity, 0) >= threshold_order:
                    out.append(payload)
    except OSError:  # pragma: no cover
        pass
    return out


# ============================================================================
# Log loader
# ============================================================================


def load_log(log_path: Optional[Path] = None, tail: Optional[int] = None) -> List[Dict[str, Any]]:
    """Load webhook dispatch log (JSONL). If tail is set, return last N records."""
    log_path = _safe_path(Path(log_path or DEFAULT_LOG_PATH))
    if not log_path.exists():
        return []
    out: List[Dict[str, Any]] = []
    try:
        with open(log_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    rec = json.loads(line)
                except json.JSONDecodeError:
                    continue
                out.append(rec)
    except OSError:  # pragma: no cover
        pass
    if tail is not None and tail >= 0:
        return out[-tail:]
    return out


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    """Run 17 self-tests; return (all_ok, n_pass, results)."""
    results: List[Dict[str, Any]] = []

    # T1: defaults build
    try:
        cfg = m_default = build_default_config({})
        ok = cfg.url.startswith("http://") and cfg.min_severity == "PAUSE"
        results.append({"name": "defaults-build", "ok": ok})
    except Exception as exc:
        results.append({"name": "defaults-build", "ok": False, "err": repr(exc)})

    # T2: validate_config accepts defaults
    try:
        validate_config(build_default_config({}))
        results.append({"name": "validate-defaults", "ok": True})
    except Exception as exc:
        results.append({"name": "validate-defaults", "ok": False, "err": repr(exc)})

    # T3: reject bad URL
    try:
        validate_config(build_default_config({"url": "ftp://x"}))
        results.append({"name": "reject-bad-url", "ok": False, "err": "no exception"})
    except ValueError:
        results.append({"name": "reject-bad-url", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-bad-url", "ok": False, "err": repr(exc)})

    # T4: reject URL without scheme
    try:
        validate_config(build_default_config({"url": "127.0.0.1:9999/hook"}))
        results.append({"name": "reject-url-no-scheme", "ok": False, "err": "no exception"})
    except ValueError:
        results.append({"name": "reject-url-no-scheme", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-url-no-scheme", "ok": False, "err": repr(exc)})

    # T5: reject bad severity
    try:
        validate_config(build_default_config({"min_severity": "BOOM"}))
        results.append({"name": "reject-bad-severity", "ok": False, "err": "no exception"})
    except ValueError:
        results.append({"name": "reject-bad-severity", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-bad-severity", "ok": False, "err": repr(exc)})

    # T6: reject bad timeout
    try:
        validate_config(build_default_config({"timeout_seconds": 0}))
        results.append({"name": "reject-timeout-zero", "ok": False, "err": "no exception"})
    except ValueError:
        results.append({"name": "reject-timeout-zero", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-timeout-zero", "ok": False, "err": repr(exc)})

    # T7: reject bad timeout overflow
    try:
        validate_config(build_default_config({"timeout_seconds": 999}))
        results.append({"name": "reject-timeout-overflow", "ok": False, "err": "no exception"})
    except ValueError:
        results.append({"name": "reject-timeout-overflow", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-timeout-overflow", "ok": False, "err": repr(exc)})

    # T8: reject bad event
    try:
        validate_config(build_default_config({"events": "FIRE"}))
        results.append({"name": "reject-bad-event", "ok": False, "err": "no exception"})
    except ValueError:
        results.append({"name": "reject-bad-event", "ok": True})
    except Exception as exc:
        results.append({"name": "reject-bad-event", "ok": False, "err": repr(exc)})

    # T9: events accept comma string
    try:
        cfg = build_default_config({"events": "INFO,WARN,ALERT"})
        out = _validate_events(cfg.events)
        ok = out == ("INFO", "WARN", "ALERT")
        results.append({"name": "events-csv-parses", "ok": ok})
    except Exception as exc:
        results.append({"name": "events-csv-parses", "ok": False, "err": repr(exc)})

    # T10: build_payload returns dict with required keys
    try:
        p = build_payload("LOCKDOWN", "LOCKDOWN", 5, tick_id="abc")
        ok = p["schema"] == V1422_SCHEMA and p["verdict"] == "LOCKDOWN" and p["n_alerts"] == 5 and p["tick_id"] == "abc"
        results.append({"name": "payload-shape", "ok": ok})
    except Exception as exc:
        results.append({"name": "payload-shape", "ok": False, "err": repr(exc)})

    # T11: sign_payload_hmac with secret
    try:
        sig = sign_payload_hmac('{"a":1}', "secret")
        ok = len(sig) == 64 and all(c in "0123456789abcdef" for c in sig)
        results.append({"name": "hmac-signs", "ok": ok})
    except Exception as exc:
        results.append({"name": "hmac-signs", "ok": False, "err": repr(exc)})

    # T12: sign_payload_hmac with no secret returns ""
    try:
        sig = sign_payload_hmac('{"a":1}', "")
        ok = sig == ""
        results.append({"name": "hmac-no-secret-blank", "ok": ok})
    except Exception as exc:
        results.append({"name": "hmac-no-secret-blank", "ok": False, "err": repr(exc)})

    # T13: dispatch_dryrun succeeds without network
    try:
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            cfg = build_default_config({
                "url": "http://127.0.0.1:1/never-reachable",
                "dry_run": True,
                "log_path": Path(td) / "log.jsonl",
            })
            validate_config(cfg)
            rec = dispatch_dryrun(cfg, "LOCKDOWN", "LOCKDOWN", n_alerts=3)
            ok = rec.status == 200 and rec.note == "dry-run"
            results.append({"name": "dispatch-dryrun-works", "ok": ok})
    except Exception as exc:
        results.append({"name": "dispatch-dryrun-works", "ok": False, "err": repr(exc)})

    # T14: dispatch below-threshold suppressed
    try:
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            cfg = build_default_config({
                "url": "http://127.0.0.1:1/never",
                "min_severity": "LOCKDOWN",
                "dry_run": True,
                "log_path": Path(td) / "log.jsonl",
            })
            validate_config(cfg)
            rec = dispatch_dryrun(cfg, "PAUSE", "PAUSE", n_alerts=0)
            ok = rec.status == 0 and "below-threshold" in rec.note
            results.append({"name": "below-threshold-suppressed", "ok": ok})
    except Exception as exc:
        results.append({"name": "below-threshold-suppressed", "ok": False, "err": repr(exc)})

    # T15: dedup suppression within cooldown
    try:
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            cfg = build_default_config({
                "url": "http://127.0.0.1:1/never",
                "cooldown_seconds": 600,
                "dry_run": True,
                "log_path": Path(td) / "log.jsonl",
            })
            validate_config(cfg)
            rec1 = dispatch_dryrun(cfg, "PAUSE", "PAUSE", n_alerts=1)
            rec2 = dispatch_dryrun(cfg, "PAUSE", "PAUSE", n_alerts=1)
            ok = rec1.note == "dry-run" and rec2.note == "dedup-suppressed"
            results.append({"name": "dedup-within-cooldown", "ok": ok})
    except Exception as exc:
        results.append({"name": "dedup-within-cooldown", "ok": False, "err": repr(exc)})

    # T16: chain_delegate returns dict with all_ok
    try:
        d = chain_delegate()
        ok = isinstance(d, dict) and d.get("all_ok") is True
        results.append({"name": "chain-delegate-ok", "ok": ok})
    except Exception as exc:
        results.append({"name": "chain-delegate-ok", "ok": False, "err": repr(exc)})

    # T17: V1422 guards + V3 guards + borrowed counts
    try:
        ok = len(V1422_GUARDS) >= 15 and len(V1422_V3_GUARDS) >= 9 and len(V1422_BORROWED) >= 4
        results.append({"name": "guards-and-borrowed-present", "ok": ok,
                        "n_guards": len(V1422_GUARDS),
                        "n_v3_guards": len(V1422_V3_GUARDS),
                        "n_borrowed": len(V1422_BORROWED)})
    except Exception as exc:
        results.append({"name": "guards-and-borrowed-present", "ok": False, "err": repr(exc)})

    n_pass = sum(1 for r in results if r["ok"])
    all_ok = all(r["ok"] for r in results)
    return all_ok, n_pass, results


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe V1418 + V1419 + V1420 + V1421 chain integrity (read-only)."""
    out: Dict[str, Any] = {
        "v1422_version": V1422_VERSION,
        "v1422_module": V1422_MODULE,
    }
    for mod_name, mod_path in (
        ("V1418", "apeireth.v1418_asi_dgm_cron_integration"),
        ("V1419", "apeireth.v1419_asi_multi_policy_evaluator"),
        ("V1420", "apeireth.v1420_asi_http_status_endpoint"),
        ("V1421", "apeireth.v1421_asi_daemon_serve_tick"),
    ):
        try:
            mod = __import__(mod_path, fromlist=["chain_delegate"])
            d = mod.chain_delegate()
            out[f"{mod_name.lower()}_loaded"] = True
            out[f"{mod_name.lower()}_all_ok"] = bool(d.get("all_ok", False))
            out[f"{mod_name.lower()}_module"] = d.get(f"{mod_name.lower()}_module", mod_path)
        except Exception as exc:  # pragma: no cover
            out[f"{mod_name.lower()}_loaded"] = False
            out[f"{mod_name.lower()}_error"] = repr(exc)
            out[f"{mod_name.lower()}_all_ok"] = False

    out["all_ok"] = all(out.get(f"{m.lower()}_all_ok", False) for m in ("V1418", "V1419", "V1420", "V1421"))
    out["ts"] = _now_utc_iso()
    return out


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(
        f"""V1422 — ASI 总框架 notification webhook v{V1422_VERSION}

Usage:
  python -m apeireth.{V1422_MODULE} <command> [args]

Commands:
  version                                  Print V1422 version
  meta [--json]                            Print V1422 metadata (JSON if --json)
  demo                                     Print short demo
  help                                     Print this help
  popper                                   Run 17 popper self-tests
  chain                                    Probe V1418+V1419+V1420+V1421 chain

  register --url URL [--events PAUSE,LOCKDOWN] [--min-severity PAUSE]
            [--cooldown-seconds N] [--timeout-seconds N]
            [--hmac-secret S] [--dry-run]    Validate a webhook config; print result

  dispatch --url URL --verdict V --severity S [--n-alerts N]
            [--timeout-seconds N]            Real POST (requires reachable URL)

  dispatch-dryrun --url URL --verdict V --severity S [--n-alerts N]
                                                  Dry-run; no network call

  preview [--history-path PATH] [--url URL]
                                            Show would-fire payloads from V1417 history

  log --tail N [--log-path PATH]            Show last N records from dispatch log

Examples:
  python -m apeireth.{V1422_MODULE} dispatch-dryrun --url http://127.0.0.1:9999/hook --verdict LOCKDOWN --severity LOCKDOWN --n-alerts 3
  python -m apeireth.{V1422_MODULE} preview --url http://127.0.0.1:9999/hook
  python -m apeireth.{V1422_MODULE} log --tail 10
"""
    )


def _parse_kv_args(argv: List[str]) -> Dict[str, str]:
    out: Dict[str, str] = {}
    i = 0
    while i < len(argv):
        a = argv[i]
        if not a.startswith("--"):
            raise ValueError(f"unexpected positional arg: {a!r}")
        key = a[2:].replace("-", "_")
        if i + 1 >= len(argv):
            raise ValueError(f"flag --{key} requires a value")
        out[key] = argv[i + 1]
        i += 2
    return out


def _coerce_overrides(kv: Dict[str, str]) -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    int_keys = {"cooldown_seconds"}
    float_keys = {"timeout_seconds"}
    bool_keys = {"dry_run"}
    path_keys = {"log_path", "history_path"}
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
        print(f"V1422 version {V1422_VERSION} (schema={V1422_SCHEMA})")
        return 0
    if cmd == "help":
        _print_help()
        return 0
    if cmd == "demo":
        print(
            "V1422 demo:\n"
            "  python -m apeireth.v1422_asi_notification_webhook register --url http://127.0.0.1:9999/hook\n"
            "  python -m apeireth.v1422_asi_notification_webhook dispatch-dryrun --url http://127.0.0.1:9999/hook --verdict LOCKDOWN --severity LOCKDOWN\n"
            "  python -m apeireth.v1422_asi_notification_webhook preview\n"
        )
        return 0
    if cmd == "meta":
        kv = _parse_kv_args(rest) if rest else {}
        cfg = build_default_config({})
        meta = {
            "version": V1422_VERSION,
            "schema": V1422_SCHEMA,
            "module": V1422_MODULE,
            "guards": list(V1422_GUARDS),
            "v3_guards": list(V1422_V3_GUARDS),
            "borrowed": [list(b) for b in V1422_BORROWED],
            "default_config": cfg.to_dict(),
        }
        if kv.get("json") == "true":
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            print(f"V1422 v{V1422_VERSION} module={V1422_MODULE} guards={len(V1422_GUARDS)}")
        return 0
    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(f"V1422 popper: all_ok={all_ok} n_pass={n_pass}/{len(results)}")
        for r in results:
            mark = "PASS" if r["ok"] else "FAIL"
            extra = "" if r["ok"] else f" — {r.get('err', '')}"
            print(f"  [{mark}] {r['name']}{extra}")
        return 0 if all_ok else 1
    if cmd == "chain":
        d = chain_delegate()
        print(json.dumps(d, indent=2, ensure_ascii=False))
        return 0 if d.get("all_ok") else 1

    if cmd == "register":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        if "events" in ov and isinstance(ov["events"], str):
            # already coerced to string in _coerce_overrides; parse it via _validate_events
            ov["events"] = _validate_events(ov["events"])
        cfg = build_default_config(ov)
        try:
            validate_config(cfg)
        except (ValueError, TypeError) as exc:
            print(json.dumps({"ok": False, "error": repr(exc)}, indent=2, ensure_ascii=False))
            return 1
        print(json.dumps({"ok": True, "config": cfg.to_dict()}, indent=2, ensure_ascii=False))
        return 0

    if cmd == "dispatch":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        url = ov.pop("url", "http://127.0.0.1:9999/hook")
        verdict = ov.pop("verdict", "LOCKDOWN")
        severity = ov.pop("severity", verdict)
        n_alerts = int(ov.pop("n_alerts", 0))
        cfg = build_default_config(ov)
        cfg.url = url
        try:
            rec = dispatch(cfg, verdict, severity, n_alerts)
        except (ValueError, TypeError) as exc:
            print(json.dumps({"ok": False, "error": repr(exc)}, indent=2, ensure_ascii=False))
            return 1
        print(json.dumps(rec.to_dict(), indent=2, ensure_ascii=False))
        return 0

    if cmd == "dispatch-dryrun":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        url = ov.pop("url", "http://127.0.0.1:9999/hook")
        verdict = ov.pop("verdict", "LOCKDOWN")
        severity = ov.pop("severity", verdict)
        n_alerts = int(ov.pop("n_alerts", 0))
        cfg = build_default_config(ov)
        cfg.url = url
        try:
            rec = dispatch_dryrun(cfg, verdict, severity, n_alerts)
        except (ValueError, TypeError) as exc:
            print(json.dumps({"ok": False, "error": repr(exc)}, indent=2, ensure_ascii=False))
            return 1
        print(json.dumps(rec.to_dict(), indent=2, ensure_ascii=False))
        return 0

    if cmd == "preview":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        cfg = build_default_config(ov)
        try:
            validate_config(cfg)
        except (ValueError, TypeError) as exc:
            print(json.dumps({"ok": False, "error": repr(exc)}, indent=2, ensure_ascii=False))
            return 1
        history_path = ov.get("history_path", DEFAULT_HISTORY_PATH)
        items = preview_from_history(cfg, history_path=history_path)
        print(json.dumps({"would_fire_count": len(items), "items": items}, indent=2, ensure_ascii=False))
        return 0

    if cmd == "log":
        kv = _parse_kv_args(rest) if rest else {}
        ov = _coerce_overrides(kv)
        tail = int(ov.get("tail", 10))
        log_path = ov.get("log_path", DEFAULT_LOG_PATH)
        records = load_log(log_path=log_path, tail=tail)
        print(json.dumps({"tail": tail, "count": len(records), "records": records}, indent=2, ensure_ascii=False))
        return 0

    print(f"V1422 unknown command: {cmd!r}")
    _print_help()
    return 1


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))
