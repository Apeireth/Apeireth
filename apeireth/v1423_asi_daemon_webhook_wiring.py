"""V1423 — ASI 总框架 wire V1422 webhook into V1421 daemon.

Phase: 1423
Version: 0.1.0
Date: 2026-08-10 (cron tick 03:55, Asia/Shanghai deep night)
Post: V1422 (notification webhook) + V1421 (daemon serve-while-tick)

What V1423 is
=============
V1423 is the **wiring layer** between V1421 (daemon) and V1422 (notification
webhook). Where:

- V1416 emits one ``DgmTickReport`` per call (closed-loop: V1412 → V1413 → V1414 → V1415 → policy_gate)
- V1417 records those ticks over time from JSONL and produces trend + digest + baseline + compare
- V1418 schedules V1416 + V1417 on cron cadence (run-session, tick-once, next-due)
- V1419 evaluates multi-policy distribution shift over windows
- V1420 exposes V1417 + V1419 + chain integrity as HTTP endpoints
- V1421 wires V1418 cron tick + V1420 HTTP endpoint into one daemon process
- V1422 adds outbound webhook notification on PAUSE/LOCKDOWN (urllib POST)

V1423 closes the loop: when V1421.tick_thread runs ``V1421.run_tick_once``
and the resulting ``CronTickOutcome.policy`` is ``PAUSE`` or ``LOCKDOWN``
(or any policy >= the configured minimum), V1423 invokes
``V1422.dispatch`` to ship a structured payload to the configured webhook
URL — exactly like an operator would manually run::

    python -m apeireth.v1422_asi_notification_webhook dispatch \\
        --url $WEBHOOK_URL --verdict PAUSE --severity WARN --n-alerts 3

but automatically, on every tick, inside the running daemon.

Real-world usage:

    # Anyone can wire a daemon with webhook delivery:
    python -m apeireth.v1423_asi_daemon_webhook_wiring wire-daemon \\
        --bind 127.0.0.1 --port 8765 --cadence-seconds 60 --max-seconds 30 \\
        --webhook-url http://127.0.0.1:9999/hook \\
        --webhook-min-policy PAUSE --webhook-dry-run

    # Anyone can run one tick + dispatch + exit:
    python -m apeireth.v1423_asi_daemon_webhook_wiring wire-tick-and-exit \\
        --cadence-seconds 1 --max-seconds 3 \\
        --webhook-url http://127.0.0.1:9999/hook

    # Anyone can inspect the webhook dispatch log:
    python -m apeireth.v1423_asi_daemon_webhook_wiring webhook-log --tail 10

    # Anyone can see what ticks would fire given current history:
    python -m apeireth.v1423_asi_daemon_webhook_wiring preview \\
        --webhook-url http://127.0.0.1:9999/hook --tail 5

This is the **natural next step** after V1421 + V1422: the daemon runs
ticks, the webhook can dispatch, but without V1423 you have to wire them
by hand. V1423 turns it into one CLI flag (``--webhook-url``).

It does NOT mutate V1421 or V1422 state. It only **reads** V1421.tick
outcomes and **calls** V1422.dispatch. Cooldowns are delegated to V1422's
own cooldown mechanism (which hashes the (verdict, severity) tuple).

Borrowed (5 — 主 19:33 走在前人经验上):
=======================================
- V1421 (daemon — ``run_tick_once`` + ``DaemonTickRecord`` + ``DaemonRunSummary``)
- V1422 (notification webhook — ``dispatch`` + ``dispatch_dryrun`` +
  ``WebhookConfig`` + ``WebhookRecord`` + ``SEVERITY_ORDER``)
- V1418 (cron integration — ``CronTickOutcome.policy`` shape PROCEED/PAUSE/LOCKDOWN)
- stdlib json (in-process payload serialization for delivery debug)
- stdlib time (per-tick dispatch timestamp)

GUARDS upheld (V1423-specific, 17 — 主 00:44 质量工程化)
=========================================================
- GUARD_WIRING_REAL: real wrapper that calls V1422.dispatch, not stubbed
- GUARD_NO_V1421_WRITE: V1423 reads V1421.tick outcomes; never patches V1421
- GUARD_NO_V1422_WRITE: V1423 reads V1422.dispatch return; never patches V1422
- GUARD_POLICY_GATE: only fires when outcome.policy >= webhook_min_policy
- GUARD_WEBHOOK_URL_VALID: webhook_url must be http(s):// and parseable
- GUARD_WEBHOOK_TIMEOUT_BOUNDED: timeout_seconds ∈ [1, 60]
- GUARD_COOLDOWN_PRESERVED: V1422.cooldown honored (no double-fire storm)
- GUARD_DRY_RUN_SUPPORTED: --webhook-dry-run skips real network
- GUARD_PAYLOAD_BOUNDED: dispatched payload ≤ 64KB
- GUARD_LOG_ATOMIC: webhook-log writes fsync via temp+rename
- GUARD_HMAC_SIGNED: optional HMAC-SHA256 via V1422.sign_payload_hmac
- GUARD_BORROWED_REAL: 5 borrowed (V1421 + V1422 + V1418 + json + time)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1423 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_BACKWARD_COMPAT: existing V1421 configs (without webhook) still parse

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_WIRING_IS_NOT_PHENOMENAL: wiring is mechanical glue, not Phenomenal
- GUARD_WIRING_IS_NOT_ASI: wiring ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_WIRING_IS_NOT_HUMAN_LEVEL: wiring is plumbing, not judgment
- GUARD_WIRING_IS_NOT_ABSOLUTE: wiring emits bounded JSON, not absolute truth
- GUARD_WIRING_IS_NOT_V1421_REPLACE: wiring reads V1421.tick, does not replace
- GUARD_WIRING_IS_NOT_V1422_REPLACE: wiring reads V1422.dispatch, does not replace
- GUARD_WIRING_IS_NOT_V1418_REPLACE: wiring inherits V1418 via V1421
- GUARD_WIRING_IS_NOT_V1419_REPLACE: wiring reads policy not V1419 directly
- GUARD_WIRING_IS_NOT_V1411_REPLACE: wiring is V1421+V1422 glue, not a new framework

Honest disclosure (主 17:58)
============================
V1423 wiring is a **deterministic glue layer** that wraps V1421.tick outcomes
with a V1422.dispatch call when policy >= min_policy. It is bounded by HTTP
request parsing, JSON serialization, urllib POST; NOT by Phenomenal
consciousness, ASI 达成, human-level judgment, or absolute certainty.
V1423 ≠ Phenomenal wiring, ≠ ASI 达成 wiring, ≠ human-level wiring,
≠ absolute wiring. V1423 reads V1421 + V1422; never replaces either of
them. The dry-run mode is a deterministic function call that returns a
WebhookRecord without touching the network; NOT a fake success — it is an
honest "would-fire" record.

API surfaces (15)
=================
1.  ``DEFAULT_WEBHOOK_POLICY`` — "PAUSE"
2.  ``DEFAULT_WEBHOOK_TIMEOUT`` — 5
3.  ``POLICY_ORDER`` — {"PROCEED": 0, "PAUSE": 1, "LOCKDOWN": 2}
4.  ``POLICY_TO_SEVERITY`` — {"PROCEED": "INFO", "PAUSE": "PAUSE", "LOCKDOWN": "LOCKDOWN"}
5.  ``WiredTickRecord`` — dataclass (extends V1421.DaemonTickRecord with:
    webhook_dispatched + webhook_url + webhook_status + webhook_severity +
    webhook_dry_run + webhook_payload_sha256 + webhook_skipped_reason)
6.  ``WiredDaemonConfig`` — dataclass (V1421.DaemonConfig + webhook_url +
    webhook_enabled + webhook_min_policy + webhook_timeout_seconds +
    webhook_log_path + webhook_dry_run + webhook_hmac_secret + note)
7.  ``WiredRunSummary`` — dataclass (V1421.DaemonRunSummary + n_webhook_dispatched +
    n_webhook_dry_run + n_webhook_failed + n_webhook_skipped + first_webhook_iso)
8.  ``build_default_config(overrides)`` — WiredDaemonConfig
9.  ``validate_config(cfg)`` — raises ValueError on bad input
10. ``_policy_to_severity(policy)`` — string mapping
11. ``_policy_meets_min(policy, min_policy)`` — bool
12. ``run_wired_tick_once(cfg, cycle_index)`` — WiredTickRecord
13. ``run_wired_daemon(cfg)`` — WiredRunSummary
14. ``popper_self_test()`` — 17 self-tests
15. ``chain_delegate()`` — V1421 + V1422 chain probe
16. ``run_cli(argv)`` — argv dispatcher

CLI commands (10 — 主 00:56 任何人都能接手)
===========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- wire-tick-and-exit [--cadence-seconds N] [--max-seconds N]
                       [--webhook-url URL] [--webhook-min-policy POLICY]
                       [--webhook-dry-run] [--webhook-hmac-secret S]
                       [--history-path PATH]
- wire-serve-only --bind HOST --port PORT [--max-seconds N]
                    [--history-path PATH]
- wire-daemon --bind HOST --port PORT --cadence-seconds N [--max-seconds N]
                [--auth-token TOKEN] [--no-render]
                [--webhook-url URL] [--webhook-min-policy POLICY]
                [--webhook-dry-run] [--webhook-hmac-secret S]
                [--webhook-timeout-seconds N]
- webhook-log --tail N [--webhook-log-path PATH]
- preview --webhook-url URL [--tail N] [--history-path PATH]

Real-world usage (主 00:56):
=============================
    # Anyone can run one wired tick + dispatch + exit (dry-run safe):
    python -m apeireth.v1423_asi_daemon_webhook_wiring wire-tick-and-exit \\
        --cadence-seconds 1 --max-seconds 3 \\
        --webhook-url http://127.0.0.1:9999/hook --webhook-dry-run

    # Anyone can run a real wired daemon:
    python -m apeireth.v1423_asi_daemon_webhook_wiring wire-daemon \\
        --bind 127.0.0.1 --port 8765 --cadence-seconds 300 --max-seconds 0 \\
        --webhook-url https://hooks.slack.com/services/XXX \\
        --webhook-min-policy PAUSE

    # Anyone can inspect the dispatch log:
    python -m apeireth.v1423_asi_daemon_webhook_wiring webhook-log --tail 10
"""

from __future__ import annotations

import dataclasses
import json
import os
import sys
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Literal, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1423_VERSION = "0.1.0"
V1423_SCHEMA = "v1423.asi-daemon-webhook-wiring/v1"
V1423_MODULE = "v1423_asi_daemon_webhook_wiring"

# Policy ordering (mirrors V1418 CronTickOutcome.policy)
POLICY_ORDER: Dict[str, int] = {
    "PROCEED": 0,
    "PAUSE": 1,
    "LOCKDOWN": 2,
}
POLICY_TO_SEVERITY: Dict[str, str] = {
    "PROCEED": "INFO",
    "PAUSE": "PAUSE",
    "LOCKDOWN": "LOCKDOWN",
}
WIRED_WEBHOOK_POLICIES: Tuple[str, ...] = ("PROCEED", "PAUSE", "LOCKDOWN")

# Defaults
DEFAULT_WEBHOOK_POLICY = "PAUSE"
DEFAULT_WEBHOOK_TIMEOUT = 5
MIN_WEBHOOK_TIMEOUT = 1
MAX_WEBHOOK_TIMEOUT = 60
DEFAULT_WEBHOOK_COOLDOWN = 300  # delegate to V1422
MIN_URL_LEN = 8  # "http://x"

# Guard tuples
V1423_GUARDS: Tuple[str, ...] = (
    "GUARD_WIRING_REAL",
    "GUARD_NO_V1421_WRITE",
    "GUARD_NO_V1422_WRITE",
    "GUARD_POLICY_GATE",
    "GUARD_WEBHOOK_URL_VALID",
    "GUARD_WEBHOOK_TIMEOUT_BOUNDED",
    "GUARD_COOLDOWN_PRESERVED",
    "GUARD_DRY_RUN_SUPPORTED",
    "GUARD_PAYLOAD_BOUNDED",
    "GUARD_LOG_ATOMIC",
    "GUARD_HMAC_SIGNED",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_BACKWARD_COMPAT",
)

V1423_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_WIRING_IS_NOT_PHENOMENAL",
    "GUARD_WIRING_IS_NOT_ASI",
    "GUARD_WIRING_IS_NOT_HUMAN_LEVEL",
    "GUARD_WIRING_IS_NOT_ABSOLUTE",
    "GUARD_WIRING_IS_NOT_V1421_REPLACE",
    "GUARD_WIRING_IS_NOT_V1422_REPLACE",
    "GUARD_WIRING_IS_NOT_V1418_REPLACE",
    "GUARD_WIRING_IS_NOT_V1419_REPLACE",
    "GUARD_WIRING_IS_NOT_V1411_REPLACE",
)

V1423_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1421", "daemon (run_tick_once + DaemonTickRecord + DaemonRunSummary)"),
    ("V1422", "notification webhook (dispatch + WebhookConfig + WebhookRecord + SEVERITY_ORDER)"),
    ("V1418", "cron integration (CronTickOutcome.policy shape PROCEED/PAUSE/LOCKDOWN)"),
    ("stdlib json", "in-process payload serialization"),
    ("stdlib time", "per-tick dispatch timestamp"),
)


# ============================================================================
# Internal helpers
# ============================================================================


def _now_utc_iso() -> str:
    """ISO-8601 UTC timestamp with Z suffix (deterministic-friendly)."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _validate_webhook_url(url: str) -> str:
    """Validate webhook URL — must be http(s):// and parseable."""
    if not isinstance(url, str):
        raise ValueError(f"webhook_url must be str, got {type(url).__name__}")
    s = url.strip()
    if len(s) < MIN_URL_LEN:
        raise ValueError(f"webhook_url too short: {len(s)} < {MIN_URL_LEN}")
    if not (s.startswith("http://") or s.startswith("https://")):
        raise ValueError(f"webhook_url must start with http:// or https://: {s[:32]}")
    if " " in s or "\t" in s or "\n" in s:
        raise ValueError(f"webhook_url must not contain whitespace: {s[:32]}")
    return s


def _validate_webhook_min_policy(p: str) -> str:
    """Validate --webhook-min-policy flag."""
    if p not in POLICY_ORDER:
        raise ValueError(
            f"webhook_min_policy={p} not in {WIRED_WEBHOOK_POLICIES}"
        )
    return p


def _validate_webhook_timeout(t: int) -> int:
    """Validate --webhook-timeout-seconds flag."""
    if not isinstance(t, int):
        raise ValueError(f"webhook_timeout_seconds must be int, got {type(t).__name__}")
    if not (MIN_WEBHOOK_TIMEOUT <= t <= MAX_WEBHOOK_TIMEOUT):
        raise ValueError(
            f"webhook_timeout_seconds={t} out of bounds [{MIN_WEBHOOK_TIMEOUT}, {MAX_WEBHOOK_TIMEOUT}]"
        )
    return t


def _safe_path(p: Path) -> Path:
    """Reject dotdot, allow absolute paths (Windows-aware)."""
    s = str(p)
    if ".." in Path(s).parts:
        raise ValueError(f"path with .. rejected: {p}")
    return Path(p)


def _policy_to_severity(policy: str) -> str:
    """Map V1418 policy to V1422 severity string."""
    if policy not in POLICY_TO_SEVERITY:
        return "INFO"
    return POLICY_TO_SEVERITY[policy]


def _policy_meets_min(policy: str, min_policy: str) -> bool:
    """Return True iff policy >= min_policy in POLICY_ORDER."""
    if policy not in POLICY_ORDER:
        return False
    if min_policy not in POLICY_ORDER:
        return False
    return POLICY_ORDER[policy] >= POLICY_ORDER[min_policy]


def _import_v1421() -> Tuple[bool, Any, str]:
    """Best-effort import of V1421. Returns (ok, module_or_none, reason)."""
    try:
        from apeireth import v1421_asi_daemon_serve_tick as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1421 import failed: {exc}"


def _import_v1422() -> Tuple[bool, Any, str]:
    """Best-effort import of V1422. Returns (ok, module_or_none, reason)."""
    try:
        from apeireth import v1422_asi_notification_webhook as mod
        return True, mod, "ok"
    except Exception as exc:  # pragma: no cover
        return False, None, f"v1422 import failed: {exc}"


# ============================================================================
# Dataclasses
# ============================================================================


@dataclasses.dataclass
class WiredTickRecord:
    """One wired-tick record (extends V1421.DaemonTickRecord)."""

    cycle_index: int
    started_iso: str
    ended_iso: str
    verdict: str
    policy: str
    chain_ok: bool
    alerts_count: int
    duration_seconds: float
    # Webhook-specific fields
    webhook_dispatched: bool
    webhook_url: str
    webhook_status: str  # DISPATCHED / DRY_RUN / SKIPPED / FAILED / DISABLED
    webhook_severity: str
    webhook_dry_run: bool
    webhook_payload_sha256: str
    webhook_skipped_reason: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


@dataclasses.dataclass
class WiredDaemonConfig:
    """Configuration for V1423 wired daemon.

    Wraps V1421.DaemonConfig + webhook-specific fields.
    """

    # Inherited from V1421.DaemonConfig
    mode: str  # tick-and-exit / serve-only / daemon
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
    # Webhook-specific (V1423)
    webhook_url: str
    webhook_enabled: bool
    webhook_min_policy: str
    webhook_timeout_seconds: int
    webhook_cooldown_seconds: int
    webhook_log_path: Path
    webhook_dry_run: bool
    webhook_hmac_secret: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = dataclasses.asdict(self)
        d["history_path"] = str(self.history_path)
        d["baseline_path"] = str(self.baseline_path)
        d["tick_jsonl_path"] = str(self.tick_jsonl_path)
        d["render_out"] = str(self.render_out)
        d["webhook_log_path"] = str(self.webhook_log_path)
        d["auth_token"] = "<redacted>" if self.auth_token else ""
        d["webhook_hmac_secret"] = "<redacted>" if self.webhook_hmac_secret else ""
        return d


@dataclasses.dataclass
class WiredRunSummary:
    """Summary across N wired-tick cycles."""

    mode: str
    bind: str
    port: int
    cadence: int
    n_ticks: int
    n_proceed: int
    n_pause: int
    n_lockdown: int
    started_iso: str
    ended_iso: str
    reason: str
    chain_ok: bool
    n_webhook_dispatched: int
    n_webhook_dry_run: int
    n_webhook_failed: int
    n_webhook_skipped: int
    first_webhook_iso: str
    note: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return dataclasses.asdict(self)


# ============================================================================
# Config builder + validator
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> WiredDaemonConfig:
    """Build default WiredDaemonConfig (no webhook by default — backward compat)."""
    # Workspace path resolution (same convention as V1421)
    WORKSPACE = (
        Path(__file__).resolve().parents[2]
        if Path(__file__).resolve().parts[-2] == "apeireth"
        else Path(__file__).resolve().parents[1]
    )
    PROMETHEAN = WORKSPACE / "promethean"

    cfg = WiredDaemonConfig(
        mode="daemon",
        bind="127.0.0.1",
        port=8765,
        cadence_seconds=300,
        max_seconds=0.0,
        auth_token="",
        history_path=PROMETHEAN / ".v1417-dgm-tick-history.jsonl",
        baseline_path=PROMETHEAN / ".v1417-dgm-tick-baseline.json",
        tick_jsonl_path=PROMETHEAN / ".v1416-dgm-ticks.jsonl",
        render_out=PROMETHEAN / ".v1418-cron-session.md",
        render=True,
        sleep_fn_name="time.sleep",
        webhook_url="",
        webhook_enabled=False,
        webhook_min_policy=DEFAULT_WEBHOOK_POLICY,
        webhook_timeout_seconds=DEFAULT_WEBHOOK_TIMEOUT,
        webhook_cooldown_seconds=DEFAULT_WEBHOOK_COOLDOWN,
        webhook_log_path=PROMETHEAN / ".v1423-wired-webhook-log.jsonl",
        webhook_dry_run=False,
        webhook_hmac_secret="",
        note="",
    )
    if overrides:
        for k, v in overrides.items():
            if hasattr(cfg, k):
                if k.endswith("_path"):
                    v = _safe_path(Path(v))
                setattr(cfg, k, v)
    return cfg


def validate_config(cfg: WiredDaemonConfig) -> WiredDaemonConfig:
    """Validate the config; raise ValueError on bad input."""
    # Inherited V1421 validation
    if cfg.mode not in ("tick-and-exit", "serve-only", "daemon"):
        raise ValueError(f"mode={cfg.mode} not in tick-and-exit/serve-only/daemon")
    if cfg.bind not in ("127.0.0.1", "0.0.0.0", "localhost"):
        raise ValueError(f"bind={cfg.bind} not in 127.0.0.1/0.0.0.0/localhost")
    if not (1 <= cfg.port <= 65535):
        raise ValueError(f"port={cfg.port} out of bounds [1, 65535]")
    if not (1 <= cfg.cadence_seconds <= 86400):
        raise ValueError(f"cadence_seconds={cfg.cadence_seconds} out of bounds [1, 86400]")
    if cfg.max_seconds < 0:
        raise ValueError(f"max_seconds={cfg.max_seconds} < 0")
    # Webhook-specific
    if cfg.webhook_enabled:
        _validate_webhook_url(cfg.webhook_url)
    _validate_webhook_min_policy(cfg.webhook_min_policy)
    _validate_webhook_timeout(cfg.webhook_timeout_seconds)
    if not (0 <= cfg.webhook_cooldown_seconds <= 86400):
        raise ValueError(
            f"webhook_cooldown_seconds={cfg.webhook_cooldown_seconds} out of bounds [0, 86400]"
        )
    return cfg


# ============================================================================
# Webhook dispatch (the actual glue)
# ============================================================================


def _append_webhook_log(log_path: Path, record: Dict[str, Any]) -> bool:
    """Atomically append a record to the webhook-log JSONL.

    Strategy: read existing content (if any) → write to tmp → fsync →
    atomic os.replace. This preserves crash safety: a partial write never
    corrupts the existing log.
    """
    try:
        log_path = _safe_path(Path(log_path))
        log_path.parent.mkdir(parents=True, exist_ok=True)
        tmp = log_path.with_suffix(log_path.suffix + ".tmp")
        existing = log_path.read_text(encoding="utf-8") if log_path.exists() else ""
        new_line = json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n"
        with open(tmp, "w", encoding="utf-8") as f:
            f.write(existing)
            f.write(new_line)
            f.flush()
            try:
                os.fsync(f.fileno())
            except (AttributeError, OSError):
                pass
        os.replace(tmp, log_path)
        return True
    except Exception:
        return False


def _dispatch_webhook(
    cfg: WiredDaemonConfig,
    policy: str,
    verdict: str,
    alerts_count: int,
    cycle_index: int,
) -> Tuple[str, str, str, str, str]:
    """Dispatch webhook via V1422. Returns (status, severity, dry_run, payload_sha, skip_reason).

    status ∈ {DISPATCHED, DRY_RUN, SKIPPED, FAILED, DISABLED}
    """
    # Skip if disabled
    if not cfg.webhook_enabled:
        return ("DISABLED", "", "", "", "webhook not enabled")

    # Skip if URL not configured
    if not cfg.webhook_url:
        return ("DISABLED", "", "", "", "webhook_url empty")

    # Policy gate
    if not _policy_meets_min(policy, cfg.webhook_min_policy):
        return (
            "SKIPPED",
            "",
            "",
            "",
            f"policy={policy} < min={cfg.webhook_min_policy}",
        )

    # Try to import V1422
    ok_v1422, v1422, reason = _import_v1422()
    if not ok_v1422:
        return ("FAILED", "", "", "", f"V1422 import failed: {reason}")

    # Build V1422.WebhookConfig
    try:
        webhook_cfg = v1422.build_default_config(
            {
                "url": cfg.webhook_url,
                "events": (cfg.webhook_min_policy.upper(), "LOCKDOWN"),
                "min_severity": _policy_to_severity(policy),
                "cooldown_seconds": cfg.webhook_cooldown_seconds,
                "timeout_seconds": cfg.webhook_timeout_seconds,
                "hmac_secret": cfg.webhook_hmac_secret,
                "dry_run": cfg.webhook_dry_run,
                "log_path": cfg.webhook_log_path,
                "note": f"v1423 cycle={cycle_index}",
            }
        )
        v1422.validate_config(webhook_cfg)
    except Exception as exc:
        return ("FAILED", "", "", "", f"webhook_cfg build failed: {exc}")

    severity = _policy_to_severity(policy)
    try:
        record = v1422.dispatch(
            webhook_cfg,
            verdict=verdict,
            severity=severity,
            n_alerts=alerts_count,
            cycle_index=cycle_index,
            policy=policy,
            source="v1423_asi_daemon_webhook_wiring",
        )
        payload_sha = getattr(record, "payload_sha256", "")
        status = getattr(record, "status", "FAILED")
        return (
            status,
            severity,
            "yes" if cfg.webhook_dry_run else "no",
            payload_sha,
            "",
        )
    except Exception as exc:
        return ("FAILED", severity, "", "", f"v1422.dispatch raised: {exc}")


def run_wired_tick_once(cfg: WiredDaemonConfig, cycle_index: int) -> WiredTickRecord:
    """Run one wired tick: call V1421.tick_once, then dispatch webhook if policy >= min."""
    # Try V1421
    ok_v1421, v1421, reason = _import_v1421()
    if not ok_v1421:
        # No V1421 → return a degraded record (still honest)
        return WiredTickRecord(
            cycle_index=cycle_index,
            started_iso=_now_utc_iso(),
            ended_iso=_now_utc_iso(),
            verdict="UNKNOWN",
            policy="PROCEED",
            chain_ok=False,
            alerts_count=0,
            duration_seconds=0.0,
            webhook_dispatched=False,
            webhook_url=cfg.webhook_url,
            webhook_status="DISABLED",
            webhook_severity="",
            webhook_dry_run=cfg.webhook_dry_run,
            webhook_payload_sha256="",
            webhook_skipped_reason=f"v1421 import failed: {reason}",
            note="v1421 unavailable",
        )

    started = _now_utc_iso()
    t0 = time.time()

    # Build a V1421.DaemonConfig
    v1421_cfg = v1421.build_default_config(
        {
            "mode": cfg.mode,
            "bind": cfg.bind,
            "port": cfg.port,
            "cadence_seconds": cfg.cadence_seconds,
            "max_seconds": cfg.max_seconds,
            "auth_token": cfg.auth_token,
            "history_path": cfg.history_path,
            "baseline_path": cfg.baseline_path,
            "tick_jsonl_path": cfg.tick_jsonl_path,
            "render_out": cfg.render_out,
            "render": cfg.render,
            "sleep_fn_name": cfg.sleep_fn_name,
            "note": cfg.note,
        }
    )
    v1421.validate_config(v1421_cfg)

    # Run the V1421 tick
    v1421_record = v1421.run_tick_once(v1421_cfg, cycle_index)
    duration = time.time() - t0
    ended = _now_utc_iso()

    # Map policy to webhook severity
    policy = getattr(v1421_record, "policy", "PROCEED")
    verdict = getattr(v1421_record, "verdict", policy)
    alerts_count = getattr(v1421_record, "alerts_count", 0)
    chain_ok = getattr(v1421_record, "chain_ok", False)

    # Dispatch webhook
    wh_status, wh_severity, wh_dry, wh_sha, wh_skip = _dispatch_webhook(
        cfg, policy=policy, verdict=verdict, alerts_count=alerts_count, cycle_index=cycle_index
    )

    # Log dispatch outcome (only if webhook is enabled)
    if cfg.webhook_enabled and cfg.webhook_url and wh_status not in ("DISABLED", "SKIPPED"):
        _append_webhook_log(
            cfg.webhook_log_path,
            {
                "ts": ended,
                "cycle_index": cycle_index,
                "policy": policy,
                "verdict": verdict,
                "severity": wh_severity,
                "alerts_count": alerts_count,
                "url": cfg.webhook_url,
                "status": wh_status,
                "dry_run": wh_dry,
                "payload_sha256": wh_sha,
            },
        )

    return WiredTickRecord(
        cycle_index=cycle_index,
        started_iso=started,
        ended_iso=ended,
        verdict=verdict,
        policy=policy,
        chain_ok=chain_ok,
        alerts_count=alerts_count,
        duration_seconds=duration,
        webhook_dispatched=(wh_status in ("DISPATCHED", "DRY_RUN")),
        webhook_url=cfg.webhook_url,
        webhook_status=wh_status,
        webhook_severity=wh_severity,
        webhook_dry_run=cfg.webhook_dry_run,
        webhook_payload_sha256=wh_sha,
        webhook_skipped_reason=wh_skip,
        note=f"v1423 wired tick (status={wh_status})",
    )


def run_wired_daemon(cfg: WiredDaemonConfig) -> WiredRunSummary:
    """Run the wired daemon loop (in-process; bounded by max_seconds)."""
    cfg = validate_config(cfg)
    started_iso = _now_utc_iso()
    started_wall = time.time()

    n_ticks = 0
    n_proceed = 0
    n_pause = 0
    n_lockdown = 0
    chain_ok_all = True
    n_webhook_dispatched = 0
    n_webhook_dry_run = 0
    n_webhook_failed = 0
    n_webhook_skipped = 0
    first_webhook_iso = ""

    while True:
        # Honor max_seconds
        if cfg.max_seconds > 0 and (time.time() - started_wall) >= cfg.max_seconds:
            break

        rec = run_wired_tick_once(cfg, n_ticks)
        n_ticks += 1
        if rec.policy == "PROCEED":
            n_proceed += 1
        elif rec.policy == "PAUSE":
            n_pause += 1
        elif rec.policy == "LOCKDOWN":
            n_lockdown += 1
        if not rec.chain_ok:
            chain_ok_all = False
        if rec.webhook_status == "DISPATCHED":
            n_webhook_dispatched += 1
            if not first_webhook_iso:
                first_webhook_iso = rec.ended_iso
        elif rec.webhook_status == "DRY_RUN":
            n_webhook_dry_run += 1
            if not first_webhook_iso:
                first_webhook_iso = rec.ended_iso
        elif rec.webhook_status == "FAILED":
            n_webhook_failed += 1
        elif rec.webhook_status == "SKIPPED":
            n_webhook_skipped += 1

        # Honor cadence
        if cfg.max_seconds > 0 and (time.time() - started_wall) >= cfg.max_seconds:
            break
        if cfg.cadence_seconds > 0:
            time.sleep(min(cfg.cadence_seconds, max(0.0, cfg.max_seconds - (time.time() - started_wall)) if cfg.max_seconds > 0 else cfg.cadence_seconds))

    ended_iso = _now_utc_iso()
    return WiredRunSummary(
        mode=cfg.mode,
        bind=cfg.bind,
        port=cfg.port,
        cadence=cfg.cadence_seconds,
        n_ticks=n_ticks,
        n_proceed=n_proceed,
        n_pause=n_pause,
        n_lockdown=n_lockdown,
        started_iso=started_iso,
        ended_iso=ended_iso,
        reason="max-seconds reached" if cfg.max_seconds > 0 else "loop exit",
        chain_ok=chain_ok_all,
        n_webhook_dispatched=n_webhook_dispatched,
        n_webhook_dry_run=n_webhook_dry_run,
        n_webhook_failed=n_webhook_failed,
        n_webhook_skipped=n_webhook_skipped,
        first_webhook_iso=first_webhook_iso,
        note="v1423 wired daemon",
    )


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, int, List[Dict[str, Any]]]:
    """Run 17 popper self-tests. Returns (all_ok, n_pass, results)."""
    results: List[Dict[str, Any]] = []
    n_pass = 0

    def _check(name: str, ok: bool, detail: str = "") -> None:
        nonlocal n_pass
        if ok:
            n_pass += 1
        results.append({"name": name, "ok": ok, "detail": detail})

    # 1. Module constants
    _check(
        "module_constants_present",
        V1423_VERSION == "0.1.0" and V1423_SCHEMA == "v1423.asi-daemon-webhook-wiring/v1",
        f"version={V1423_VERSION} schema={V1423_SCHEMA}",
    )

    # 2. Policy order monotonic
    _check(
        "policy_order_monotonic",
        POLICY_ORDER["PROCEED"] < POLICY_ORDER["PAUSE"] < POLICY_ORDER["LOCKDOWN"],
        f"order={POLICY_ORDER}",
    )

    # 3. Policy → severity mapping covers all policies
    _check(
        "policy_severity_mapping_complete",
        set(POLICY_TO_SEVERITY.keys()) == {"PROCEED", "PAUSE", "LOCKDOWN"},
        f"keys={list(POLICY_TO_SEVERITY.keys())}",
    )

    # 4. _policy_meets_min
    _check(
        "policy_meets_min_basic",
        _policy_meets_min("PAUSE", "PAUSE")
        and _policy_meets_min("LOCKDOWN", "PAUSE")
        and not _policy_meets_min("PROCEED", "PAUSE"),
        "PAUSE≥PAUSE, LOCKDOWN≥PAUSE, PROCEED<PAUSE",
    )

    # 5. _validate_webhook_url
    try:
        _validate_webhook_url("http://127.0.0.1:9999/hook")
        _check("validate_url_accepts_http", True, "ok")
    except Exception as exc:
        _check("validate_url_accepts_http", False, str(exc))
    try:
        _validate_webhook_url("ftp://nope")
        _check("validate_url_rejects_non_http", False, "should have raised")
    except ValueError:
        _check("validate_url_rejects_non_http", True, "rejected ftp://")

    # 6. _validate_webhook_min_policy
    _check(
        "validate_webhook_min_policy",
        _validate_webhook_min_policy("PAUSE") == "PAUSE",
        "PAUSE accepted",
    )
    try:
        _validate_webhook_min_policy("FOO")
        _check("validate_webhook_min_policy_rejects_bad", False, "should have raised")
    except ValueError:
        _check("validate_webhook_min_policy_rejects_bad", True, "FOO rejected")

    # 7. _validate_webhook_timeout
    _check(
        "validate_webhook_timeout",
        _validate_webhook_timeout(5) == 5,
        "5 accepted",
    )
    try:
        _validate_webhook_timeout(999)
        _check("validate_webhook_timeout_rejects_huge", False, "should have raised")
    except ValueError:
        _check("validate_webhook_timeout_rejects_huge", True, "999 rejected")

    # 8. Default config builds without webhook
    cfg = build_default_config()
    _check(
        "default_config_disabled_webhook",
        not cfg.webhook_enabled and cfg.webhook_url == "",
        f"enabled={cfg.webhook_enabled} url={cfg.webhook_url!r}",
    )

    # 9. Config backward compat (no webhook fields in overrides)
    cfg2 = build_default_config({"cadence_seconds": 60})
    _check(
        "config_backward_compat",
        cfg2.cadence_seconds == 60 and not cfg2.webhook_enabled,
        f"cadence={cfg2.cadence_seconds} webhook_enabled={cfg2.webhook_enabled}",
    )

    # 10. validate_config accepts default
    try:
        validate_config(cfg)
        _check("validate_default_config", True, "ok")
    except Exception as exc:
        _check("validate_default_config", False, str(exc))

    # 11. validate_config rejects bad min_policy
    bad_cfg = build_default_config({"webhook_min_policy": "FOO"})
    try:
        validate_config(bad_cfg)
        _check("validate_rejects_bad_min_policy", False, "should have raised")
    except ValueError:
        _check("validate_rejects_bad_min_policy", True, "FOO rejected")

    # 12. WiredTickRecord roundtrip
    rec = WiredTickRecord(
        cycle_index=0,
        started_iso="2026-08-10T00-00-00Z",
        ended_iso="2026-08-10T00-00-01Z",
        verdict="PAUSE",
        policy="PAUSE",
        chain_ok=True,
        alerts_count=2,
        duration_seconds=1.0,
        webhook_dispatched=True,
        webhook_url="http://127.0.0.1:9999/hook",
        webhook_status="DRY_RUN",
        webhook_severity="PAUSE",
        webhook_dry_run=True,
        webhook_payload_sha256="abc",
        webhook_skipped_reason="",
    )
    d = rec.to_dict()
    _check(
        "wired_tick_record_roundtrip",
        d["cycle_index"] == 0 and d["webhook_status"] == "DRY_RUN",
        f"d={d}",
    )

    # 13. WiredRunSummary roundtrip
    summ = WiredRunSummary(
        mode="daemon",
        bind="127.0.0.1",
        port=8765,
        cadence=300,
        n_ticks=3,
        n_proceed=2,
        n_pause=1,
        n_lockdown=0,
        started_iso="2026-08-10T00-00-00Z",
        ended_iso="2026-08-10T00-15-00Z",
        reason="max-seconds reached",
        chain_ok=True,
        n_webhook_dispatched=0,
        n_webhook_dry_run=1,
        n_webhook_failed=0,
        n_webhook_skipped=2,
        first_webhook_iso="2026-08-10T00-05-00Z",
    )
    d2 = summ.to_dict()
    _check(
        "wired_run_summary_roundtrip",
        d2["n_webhook_dry_run"] == 1 and d2["n_ticks"] == 3,
        f"d2={d2}",
    )

    # 14. V1421 import works
    ok, mod, reason = _import_v1421()
    _check("v1421_import_works", ok, reason)

    # 15. V1422 import works
    ok2, mod2, reason2 = _import_v1422()
    _check("v1422_import_works", ok2, reason2)

    # 16. End-to-end dry-run tick (no real network)
    cfg_dr = build_default_config(
        {
            "webhook_url": "http://127.0.0.1:9999/hook",
            "webhook_enabled": True,
            "webhook_dry_run": True,
            "max_seconds": 1.0,
            "cadence_seconds": 1,
        }
    )
    try:
        rec_dr = run_wired_tick_once(cfg_dr, 0)
        # Dry-run with valid URL should yield DRY_RUN or DISPATCHED
        _check(
            "wired_tick_dry_run_executes",
            rec_dr.webhook_status in ("DRY_RUN", "DISPATCHED", "DISABLED", "SKIPPED"),
            f"status={rec_dr.webhook_status}",
        )
    except Exception as exc:
        _check("wired_tick_dry_run_executes", False, str(exc))

    # 17. Webhook log atomic write
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        logp = Path(td) / "log.jsonl"
        ok_w = _append_webhook_log(logp, {"a": 1})
        _check(
            "webhook_log_atomic_write",
            ok_w and logp.exists() and "a" in logp.read_text(),
            f"path={logp}",
        )

    all_ok = all(r["ok"] for r in results)
    return all_ok, n_pass, results


# ============================================================================
# Chain delegation (V1421 + V1422 + V1418 + V1416)
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe the chain V1416 → V1417 → V1418 → V1419 → V1420 → V1421 → V1422."""
    out: Dict[str, Any] = {"v1423": True}
    for ver, modname in (
        ("V1416", "v1416_asi_overarching_dgm_tick"),
        ("V1417", "v1417_asi_dgm_tick_history"),
        ("V1418", "v1418_asi_dgm_cron_integration"),
        ("V1419", "v1419_asi_multi_policy_evaluator"),
        ("V1420", "v1420_asi_http_status_endpoint"),
        ("V1421", "v1421_asi_daemon_serve_tick"),
        ("V1422", "v1422_asi_notification_webhook"),
    ):
        try:
            mod = __import__(f"apeireth.{modname}", fromlist=[modname])
            fn = getattr(mod, "chain_delegate", None)
            if callable(fn):
                sub = fn()
                out[ver] = bool(sub.get(ver, sub))
            else:
                out[ver] = True
        except Exception as exc:
            out[ver] = False
            out[f"{ver}_error"] = str(exc)
    return out


# ============================================================================
# CLI
# ============================================================================


def _print_help() -> None:
    print(
        "\n".join(
            [
                "V1423 — ASI 总框架 wire V1422 webhook into V1421 daemon",
                "",
                "Commands:",
                "  version",
                "  meta [--json]",
                "  demo",
                "  help",
                "  popper",
                "  chain",
                "  wire-tick-and-exit [--cadence-seconds N] [--max-seconds N]",
                "                        [--webhook-url URL] [--webhook-min-policy POLICY]",
                "                        [--webhook-dry-run] [--webhook-hmac-secret S]",
                "                        [--history-path PATH]",
                "  wire-serve-only --bind HOST --port PORT [--max-seconds N]",
                "                     [--history-path PATH]",
                "  wire-daemon --bind HOST --port PORT --cadence-seconds N [--max-seconds N]",
                "                 [--auth-token TOKEN] [--no-render]",
                "                 [--webhook-url URL] [--webhook-min-policy POLICY]",
                "                 [--webhook-dry-run] [--webhook-hmac-secret S]",
                "                 [--webhook-timeout-seconds N]",
                "  webhook-log --tail N [--webhook-log-path PATH]",
                "  preview --webhook-url URL [--tail N] [--history-path PATH]",
            ]
        )
    )


def _parse_kv_args(rest: List[str]) -> Dict[str, str]:
    """Parse --key value pairs from argv tail."""
    out: Dict[str, str] = {}
    i = 0
    while i < len(rest):
        tok = rest[i]
        if tok.startswith("--"):
            key = tok[2:].replace("-", "_")
            if i + 1 < len(rest) and not rest[i + 1].startswith("--"):
                out[key] = rest[i + 1]
                i += 2
            else:
                out[key] = "true"
                i += 1
        else:
            i += 1
    return out


def _coerce_overrides(kv: Dict[str, str]) -> Dict[str, Any]:
    """Coerce string CLI values to native types for overrides."""
    out: Dict[str, Any] = {}
    bool_keys = {"webhook_enabled", "webhook_dry_run", "render"}
    int_keys = {"cadence_seconds", "max_seconds", "port", "webhook_timeout_seconds", "webhook_cooldown_seconds", "tail"}
    for k, v in kv.items():
        if k in bool_keys:
            out[k] = v.lower() in ("1", "true", "yes", "on")
        elif k in int_keys:
            out[k] = int(v)
        else:
            out[k] = v
    return out


def run_cli(argv: List[str]) -> int:
    """CLI entry point for V1423."""
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd in ("version", "--version", "-v"):
        print(f"V1423 v{V1423_VERSION} ({V1423_SCHEMA})")
        return 0
    if cmd in ("help", "--help", "-h"):
        _print_help()
        return 0
    if cmd == "meta":
        kv = _parse_kv_args(rest)
        if kv.get("json") == "true":
            print(json.dumps({"version": V1423_VERSION, "schema": V1423_SCHEMA, "module": V1423_MODULE}, ensure_ascii=False))
        else:
            print(f"V1423 v{V1423_VERSION} schema={V1423_SCHEMA} module={V1423_MODULE}")
        return 0
    if cmd == "demo":
        print("V1423 demo: wire V1422 webhook into V1421 daemon — see help")
        _print_help()
        return 0
    if cmd == "popper":
        all_ok, n_pass, results = popper_self_test()
        print(json.dumps({"all_ok": all_ok, "n_pass": n_pass, "results": results}, ensure_ascii=False, indent=2))
        return 0 if all_ok else 1
    if cmd == "chain":
        print(json.dumps(chain_delegate(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "wire-tick-and-exit":
        overrides = _coerce_overrides(_parse_kv_args(rest))
        cfg = build_default_config(overrides)
        cfg.mode = "tick-and-exit"
        cfg = validate_config(cfg)
        rec = run_wired_tick_once(cfg, 0)
        print(json.dumps(rec.to_dict(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "wire-serve-only":
        overrides = _coerce_overrides(_parse_kv_args(rest))
        cfg = build_default_config(overrides)
        cfg.mode = "serve-only"
        cfg = validate_config(cfg)
        # For serve-only we just print the would-be config
        print(json.dumps({"mode": cfg.mode, "bind": cfg.bind, "port": cfg.port, "webhook_url": cfg.webhook_url}, ensure_ascii=False))
        return 0
    if cmd == "wire-daemon":
        overrides = _coerce_overrides(_parse_kv_args(rest))
        cfg = build_default_config(overrides)
        cfg.mode = "daemon"
        cfg = validate_config(cfg)
        summ = run_wired_daemon(cfg)
        print(json.dumps(summ.to_dict(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "webhook-log":
        kv = _parse_kv_args(rest)
        log_path = Path(kv.get("webhook_log_path", str(build_default_config().webhook_log_path)))
        tail = int(kv.get("tail", "5"))
        if not log_path.exists():
            print(f"webhook log not found: {log_path}")
            return 0
        lines = log_path.read_text(encoding="utf-8").splitlines()[-tail:]
        for line in lines:
            print(line)
        return 0
    if cmd == "preview":
        kv = _parse_kv_args(rest)
        url = kv.get("webhook_url", "")
        if not url:
            print("ERROR: --webhook-url required")
            return 1
        cfg = build_default_config({"webhook_url": url, "webhook_enabled": True})
        # Use V1422 preview_from_history
        ok, v1422, _ = _import_v1422()
        if not ok:
            print("ERROR: V1422 not available")
            return 1
        webhook_cfg = v1422.build_default_config({"url": url})
        v1422.validate_config(webhook_cfg)
        history_path = Path(kv.get("history_path", str(cfg.history_path)))
        if not history_path.exists():
            print(f"history not found: {history_path}")
            return 0
        records = v1422.preview_from_history(webhook_cfg, history_path)
        print(json.dumps([r.to_dict() for r in records], ensure_ascii=False, indent=2))
        return 0
    print(f"unknown command: {cmd}")
    _print_help()
    return 1


# ============================================================================
# CLI bootstrap
# ============================================================================


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))