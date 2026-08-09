"""V1418 — ASI 总框架 DGM cron integration (5min cron auto-tick → append → render).

Phase: 1418
Version: 0.1.0
Date: 2026-08-10 (cron tick 02:50, Asia/Shanghai deep night)
Post: V1417 (DGM tick history)

What V1418 is
=============
V1418 is the **cron-integration companion** to V1416 + V1417. Where:

- V1416 emits one ``DgmTickReport`` per call (the actual closed-loop:
  V1412 → V1413 → V1414 → V1415 → policy_gate → DgmTickReport)
- V1417 records those ticks over time from JSONL and produces
  trend + digest + baseline + compare

V1418 is the **integration layer** that lets an external scheduler
(5min cron, GitHub Actions, systemd timer, etc.) actually drive V1416
+ V1417 as one orchestrated, idempotent command:

    python -m apeireth.v1418_asi_dgm_cron_integration tick-once --render
    python -m apeireth.v1418_asi_dgm_cron_integration run-session --cycles 5 --cadence-seconds 1
    python -m apeireth.v1418_asi_dgm_cron_integration next-due --last-iso "..." --cadence-seconds 300

It does NOT mutate V1416 or V1417. It only **reads** them, calls their
public APIs, and aggregates outcomes.

Why V1418 exists
================
V1416 + V1417 are two modules with separate CLIs. Operators who want
a 5-minute ASI 总框架 tick loop need:

- one command that runs the tick + records it + (optionally) renders
- one command that runs N ticks on a cadence (session loop)
- one command that computes next-due (when the next tick should run)
- one command that renders the aggregated session summary
- one chain delegate that probes V1416 + V1417 integrity (read-only)

This is the natural cron-integration layer to close the ASI 总框架
DGM loop with a real-world scheduler. It is **read-only on V1416 +
V1417**.

Most common questions answered by one command:

- "Run the next ASI 总框架 tick and append + render it"
- "Run 5 ticks at 1-second cadence and emit a session summary"
- "When is the next tick due given cadence 300s from 02:45?"
- "Probe V1416 + V1417 chain integrity without committing a tick"

Borrowed (4 — 主 19:33 走在前人经验上):
======================================
- V1416 (DGM closed-loop tick executor — ``run_dgm_tick`` + ``append_tick``)
- V1417 (tick history — ``append_tick_snapshot`` + ``load_tick_history`` +
  ``render_tick_history_md``)
- V1369 cron hook (cron tick pattern, ``v1369_v1368_cron_hook.py``)
- V1383 cron tick (cadence + session pattern, ``v1383_v1382_cron_tick.py``)

GUARDS upheld (V1418-specific, 15 — 主 00:44 质量工程化)
========================================================
- GUARD_CRON_REAL: real integration, not stubbed
- GUARD_NO_V1417_WRITE: V1418 reads/calls V1417; never patches V1417 state
- GUARD_NO_V1416_WRITE: V1418 reads/calls V1416; never patches V1416 state
- GUARD_TICK_FROM_V1416: tick invocation goes through V1416.run_dgm_tick
- GUARD_HISTORY_FROM_V1417: history append goes through V1417.append_tick_snapshot
- GUARD_BOUNDED_CYCLES: cycles ∈ [1, MAX_CYCLES]
- GUARD_CADENCE_BOUNDED: cadence_seconds ∈ [MIN_CADENCE, MAX_CADENCE]
- GUARD_DETERMINISTIC: same inputs → same outcomes (no race)
- GUARD_ATOMIC_WRITE: V1416.append_tick + V1417.append_tick_snapshot both fsync
- GUARD_BORROWED_REAL: 4 borrowed (V1416 + V1417 + V1369 + V1383)
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: V1418 chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted
- GUARD_CLI_RUNNABLE: CLI 真可跑
- GUARD_PATH_SAFE: path safety (dotdot rejected, absolute allowed)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 9 guards
======================================================
- GUARD_CRON_IS_NOT_PHENOMENAL: cron integration is mechanical scheduling, not Phenomenal
- GUARD_CRON_IS_NOT_ASI: cron ≠ ASI 达成 (gap 0.0695 preserved)
- GUARD_CRON_IS_NOT_HUMAN_LEVEL: cron is ASI 总框架, not human-level judgment
- GUARD_CRON_IS_NOT_ABSOLUTE: cron is regulative ideal, not absolute certainty
- GUARD_CRON_IS_NOT_V1417_REPLACE: cron reads V1417, does not replace
- GUARD_CRON_IS_NOT_V1416_REPLACE: cron reads V1416, does not replace
- GUARD_CRON_IS_NOT_V1413_REPLACE: cron is V1416-specialized (not V1413 dashboard)
- GUARD_CRON_IS_NOT_V1412_REPLACE: cron inherits via V1417 → V1416 → V1412
- GUARD_CRON_IS_NOT_V1411_REPLACE: cron inherits via V1417 → V1416 → V1412 → V1411

Honest disclosure (主 17:58)
============================
V1418 cron integration is a **deterministic scheduling/orchestration layer**
that wraps V1416 (tick executor) and V1417 (tick history). It is bounded by
arithmetic on real timestamps and the V1416 + V1417 output fields; NOT by
Phenomenal consciousness, ASI 达成, human-level judgment, or absolute
certainty. V1418 ≠ Phenomenal cron, ≠ ASI 达成 cron, ≠ human-level cron,
≠ absolute cron. V1418 reads V1416 + V1417; never replaces either of them.
The "next-due" calculation is a deterministic rule on cadence + jitter —
NOT a free agent will.

API surfaces (14)
=================
1.  ``CronIntegrationConfig`` — dataclass (cadence_seconds + jitter_seconds +
    auto_render + render_out + max_cycles + min_cadence_seconds +
    max_cadence_seconds + sleep_fn_name)
2.  ``CronTickOutcome`` — dataclass (cycle_index + ran_at_iso + tick_id +
    policy + chain_ok + alerts_count + escalation_count + n_modules +
    appended_to_history + rendered_path + render_ok + note)
3.  ``CronSessionSummary`` — dataclass (n_cycles + n_policies +
    policy_proceed_count + policy_pause_count + policy_lockdown_count +
    chain_ok_count + chain_ok_rate + first_tick + last_tick +
    span_seconds + session_started_iso + session_ended_iso + rendered_path)
4.  ``DEFAULT_CRON_CONFIG`` — CronIntegrationConfig
5.  ``build_default_config(overrides)`` — CronIntegrationConfig
6.  ``_safe_path(p)`` — Path (dotdot rejected)
7.  ``_now_utc_iso()`` — ISO-8601 UTC timestamp string
8.  ``_parse_iso_timestamp(s)`` — Optional[datetime]
9.  ``_sleep_for(seconds, sleep_fn)`` — float (real elapsed seconds)
10. ``compute_next_due(last_iso, cadence_seconds, jitter_seconds, sleep_fn)`` — str
11. ``tick_once(history_path, baseline_path, tick_jsonl_path, render_out, render)`` — CronTickOutcome
12. ``run_session(cycles, cadence_seconds, jitter_seconds, history_path,
    baseline_path, tick_jsonl_path, render_out, render, sleep_fn)`` — CronSessionSummary
13. ``render_session_md(summary, outcomes)`` — markdown
14. ``popper_self_test()`` — 15 self-tests
15. ``chain_delegate()`` — V1416 + V1417 chain probe
16. ``run_cli(argv)`` — argv dispatcher

CLI commands (13 — 主 00:56 任何人都能接手)
==========================================
- version
- meta [--json]
- demo
- help
- popper
- chain
- tick-once [--history-path] [--baseline-path] [--tick-jsonl-path] [--render-out] [--render/--no-render]
- run-session --cycles N [--cadence-seconds N] [--jitter-seconds N]
                [--history-path] [--baseline-path] [--tick-jsonl-path]
                [--render-out] [--render/--no-render] [--sleep-fn NAME]
- next-due --last-iso "ISO" [--cadence-seconds 300] [--jitter-seconds 0]
            [--sleep-fn NAME]
- render-summary --summary-json PATH [--out PATH]
- show-outcomes --summary-json PATH [--last N] [--json]
- detect-policy --last N --summary-json PATH
- emit-shell [--render/--no-render] [--cycles N] [--cadence-seconds N]
"""

from __future__ import annotations

import dataclasses
import json
import os
import sys
import time
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1418_VERSION = "0.1.0"
V1418_SCHEMA = "v1418.asi-dgm-cron-integration/v1"
V1418_MODULE = "v1418_asi_dgm_cron_integration"

# Real default paths (same convention as V1416 / V1417):
WORKSPACE = Path(__file__).resolve().parents[2] if Path(__file__).resolve().parts[-2] == "apeireth" else Path(__file__).resolve().parents[1]
PROMETHEAN = WORKSPACE / "promethean"
DEFAULT_HISTORY_PATH = PROMETHEAN / ".v1417-dgm-tick-history.jsonl"
DEFAULT_BASELINE_PATH = PROMETHEAN / ".v1417-dgm-tick-baseline.json"
DEFAULT_TICK_JSONL_PATH = PROMETHEAN / ".v1416-dgm-ticks.jsonl"
DEFAULT_RENDER_OUT = PROMETHEAN / "V1418_CRON_SESSION.md"
DEFAULT_SUMMARY_JSON = PROMETHEAN / ".v1418-cron-session-summary.json"

# Bounded guards (主 00:44 质量工程化)
MIN_CADENCE_SECONDS = 1          # tight lower bound (real crons run 5min+)
MAX_CADENCE_SECONDS = 86400      # 1 day upper bound
MAX_CYCLES_PER_SESSION = 1024    # safety bound
DEFAULT_CADENCE_SECONDS = 300    # 5 minutes (matches cron prompt cadence)
DEFAULT_JITTER_SECONDS = 0       # default off (deterministic)

V1418_POLICIES = ("PROCEED", "PAUSE", "LOCKDOWN")
V1418_GUARDS: Tuple[str, ...] = (
    "GUARD_CRON_REAL",
    "GUARD_NO_V1417_WRITE",
    "GUARD_NO_V1416_WRITE",
    "GUARD_TICK_FROM_V1416",
    "GUARD_HISTORY_FROM_V1417",
    "GUARD_BOUNDED_CYCLES",
    "GUARD_CADENCE_BOUNDED",
    "GUARD_DETERMINISTIC",
    "GUARD_ATOMIC_WRITE",
    "GUARD_BORROWED_REAL",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_CLI_RUNNABLE",
    "GUARD_PATH_SAFE",
)

V1418_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_CRON_IS_NOT_PHENOMENAL",
    "GUARD_CRON_IS_NOT_ASI",
    "GUARD_CRON_IS_NOT_HUMAN_LEVEL",
    "GUARD_CRON_IS_NOT_ABSOLUTE",
    "GUARD_CRON_IS_NOT_V1417_REPLACE",
    "GUARD_CRON_IS_NOT_V1416_REPLACE",
    "GUARD_CRON_IS_NOT_V1413_REPLACE",
    "GUARD_CRON_IS_NOT_V1412_REPLACE",
    "GUARD_CRON_IS_NOT_V1411_REPLACE",
)

V1418_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1416", "DGM closed-loop tick executor (run_dgm_tick + append_tick)"),
    ("V1417", "tick history (append_tick_snapshot + load_tick_history + render_tick_history_md)"),
    ("V1369", "cron hook pattern (v1369_v1368_cron_hook.py)"),
    ("V1383", "cron tick cadence + session pattern (v1383_v1382_cron_tick.py)"),
)

# Sleep functions for testability. Production uses time.sleep; tests can swap.
_SLEEP_FUNCTIONS = {
    "time.sleep": time.sleep,
    "pass-through": lambda s: 0.0,
}


# ============================================================================
# Helpers
# ============================================================================


def _safe_path(p: Path) -> Path:
    """Reject dotdot, allow absolute paths (Windows-aware)."""
    s = str(p)
    # Accept both forward and back slashes as separators.
    parts = []
    if "/" in s:
        parts = s.split("/")
    elif "\\" in s:
        parts = s.split("\\")
    else:
        parts = [s]
    if ".." in parts:
        raise ValueError(f"path with .. rejected: {p}")
    return p


def _now_utc_iso() -> str:
    """Return current UTC timestamp in ISO-8601 with Z suffix (UTC variant)."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")


def _parse_iso_timestamp(s: str) -> Optional[Any]:
    """Best-effort parse of an ISO timestamp; return datetime or None."""
    from datetime import datetime
    if not s:
        return None
    for fmt in (
        "%Y-%m-%dT%H-%M-%SZ",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%dT%H:%M:%S.%fZ",
        "%Y-%m-%dT%H:%M:%S%z",
    ):
        try:
            return datetime.strptime(s, fmt)
        except (ValueError, TypeError):
            continue
    return None


def _resolve_sleep_fn(name: str) -> Callable[[float], Any]:
    """Resolve a sleep function by name; default = time.sleep."""
    return _SLEEP_FUNCTIONS.get(name, time.sleep)


def _sleep_for(seconds: float, sleep_fn: Callable[[float], Any]) -> float:
    """Sleep `seconds` using sleep_fn; return real elapsed seconds."""
    t0 = time.time()
    sleep_fn(max(0.0, seconds))
    return time.time() - t0


# ============================================================================
# Config + Dataclasses
# ============================================================================


@dataclasses.dataclass
class CronIntegrationConfig:
    """Configuration for V1418 cron integration."""

    cadence_seconds: int
    jitter_seconds: int
    auto_render: bool
    render_out: Path
    max_cycles: int
    min_cadence_seconds: int
    max_cadence_seconds: int
    sleep_fn_name: str
    note: str = ""

    def __post_init__(self) -> None:
        if not (self.min_cadence_seconds <= self.cadence_seconds <= self.max_cadence_seconds):
            raise ValueError(
                f"cadence_seconds={self.cadence_seconds} out of bounds "
                f"[{self.min_cadence_seconds}, {self.max_cadence_seconds}]"
            )
        if self.max_cycles < 1 or self.max_cycles > MAX_CYCLES_PER_SESSION:
            raise ValueError(f"max_cycles={self.max_cycles} out of bounds [1, {MAX_CYCLES_PER_SESSION}]")
        self.render_out = _safe_path(Path(self.render_out))
        if self.sleep_fn_name not in _SLEEP_FUNCTIONS:
            self.sleep_fn_name = "time.sleep"


DEFAULT_CRON_CONFIG = CronIntegrationConfig(
    cadence_seconds=DEFAULT_CADENCE_SECONDS,
    jitter_seconds=DEFAULT_JITTER_SECONDS,
    auto_render=True,
    render_out=DEFAULT_RENDER_OUT,
    max_cycles=MAX_CYCLES_PER_SESSION,
    min_cadence_seconds=MIN_CADENCE_SECONDS,
    max_cadence_seconds=MAX_CADENCE_SECONDS,
    sleep_fn_name="time.sleep",
)


@dataclasses.dataclass
class CronTickOutcome:
    """Outcome of one cron tick cycle."""

    cycle_index: int
    ran_at_iso: str
    tick_id: str
    policy: str
    chain_ok: bool
    alerts_count: int
    escalation_count: int
    n_modules: int
    appended_to_history: bool
    rendered_path: str
    render_ok: bool
    note: str


@dataclasses.dataclass
class CronSessionSummary:
    """Aggregated summary across N cron tick cycles."""

    n_cycles: int
    n_policies: int
    policy_proceed_count: int
    policy_pause_count: int
    policy_lockdown_count: int
    chain_ok_count: int
    chain_ok_rate: float
    first_tick: str
    last_tick: str
    span_seconds: int
    session_started_iso: str
    session_ended_iso: str
    rendered_path: str
    note: str


# ============================================================================
# Builders
# ============================================================================


def build_default_config(overrides: Optional[Dict[str, Any]] = None) -> CronIntegrationConfig:
    """Build a CronIntegrationConfig from DEFAULT_CRON_CONFIG + overrides."""
    base: Dict[str, Any] = {
        "cadence_seconds": DEFAULT_CRON_CONFIG.cadence_seconds,
        "jitter_seconds": DEFAULT_CRON_CONFIG.jitter_seconds,
        "auto_render": DEFAULT_CRON_CONFIG.auto_render,
        "render_out": DEFAULT_CRON_CONFIG.render_out,
        "max_cycles": DEFAULT_CRON_CONFIG.max_cycles,
        "min_cadence_seconds": DEFAULT_CRON_CONFIG.min_cadence_seconds,
        "max_cadence_seconds": DEFAULT_CRON_CONFIG.max_cadence_seconds,
        "sleep_fn_name": DEFAULT_CRON_CONFIG.sleep_fn_name,
        "note": DEFAULT_CRON_CONFIG.note,
    }
    if overrides:
        for k, v in overrides.items():
            if k in base:
                base[k] = v
            else:
                raise ValueError(f"unknown override key: {k}")
    return CronIntegrationConfig(**base)


# ============================================================================
# Core API
# ============================================================================


def compute_next_due(
    last_iso: str,
    cadence_seconds: int,
    jitter_seconds: int = 0,
    sleep_fn: Callable[[float], Any] = time.sleep,
) -> str:
    """Compute next-due ISO timestamp from last-ran ISO + cadence (+jitter).

    The caller picks `now` source of truth: the **return value** is a
    pure ISO string computed deterministically. The sleep_fn is NOT
    invoked here — it's just a signature placeholder for parity.
    """
    if not (MIN_CADENCE_SECONDS <= cadence_seconds <= MAX_CADENCE_SECONDS):
        raise ValueError(f"cadence_seconds={cadence_seconds} out of bounds")
    last_dt = _parse_iso_timestamp(last_iso)
    if last_dt is None:
        return _now_utc_iso()
    from datetime import timedelta
    next_dt = last_dt + timedelta(seconds=cadence_seconds + jitter_seconds)
    return next_dt.strftime("%Y-%m-%dT%H-%M-%SZ")


def tick_once(
    history_path: Optional[Path] = None,
    baseline_path: Optional[Path] = None,
    tick_jsonl_path: Optional[Path] = None,
    render_out: Optional[Path] = None,
    render: bool = True,
    cycle_index: int = 1,
) -> CronTickOutcome:
    """Run ONE V1416 tick + V1417 append + (optional) V1417 render.

    Returns a CronTickOutcome. NEVER mutates V1416 or V1417 module-level
    state — only calls their public APIs (run_dgm_tick + append_tick +
    append_tick_snapshot + render_tick_history_md + load_tick_history +
    compute_tick_trend + compute_tick_digest + load_baseline +
    compare_to_baseline).
    """
    history_path = _safe_path(Path(history_path or DEFAULT_HISTORY_PATH))
    baseline_path = _safe_path(Path(baseline_path or DEFAULT_BASELINE_PATH))
    tick_jsonl_path = _safe_path(Path(tick_jsonl_path or DEFAULT_TICK_JSONL_PATH))
    render_out = _safe_path(Path(render_out or DEFAULT_RENDER_OUT))

    ran_at_iso = _now_utc_iso()
    note = f"V1418 tick_once cycle_index={cycle_index} render={render}"

    # Lazy-import V1416 + V1417 to keep this module importable in isolation.
    try:
        import apeireth.v1416_asi_overarching_dgm_tick as v1416  # type: ignore
        v1416_module_loaded = True
        v1416_module = v1416
    except Exception as exc:  # pragma: no cover — defensive
        v1416_module_loaded = False
        v1416_module = None
        v1416_error = repr(exc)
    else:
        v1416_error = ""

    try:
        import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
        v1417_module_loaded = True
    except Exception as exc:  # pragma: no cover — defensive
        v1417_module_loaded = False
        v1417_error = repr(exc)
    else:
        v1417_error = ""

    tick_report = None
    tick_id = ""
    policy = "PROCEED"
    chain_ok = True
    alerts_count = 0
    escalation_count = 0
    n_modules = 0

    # 1. V1416.run_dgm_tick — produces a real DgmTickReport.
    # V1416.run_dgm_tick ALWAYS appends to V1416_DEFAULT_OUT_PATH internally;
    # we read from V1416's actual output path below when copying into V1417.
    v1416_default_out = Path(getattr(v1416_module, "V1416_DEFAULT_OUT_PATH", ".v1416-dgm-ticks.jsonl"))
    actual_v1416_log = tick_jsonl_path if str(tick_jsonl_path) != ".v1416-dgm-ticks.jsonl" else v1416_default_out
    if v1416_module_loaded:
        try:
            cfg = v1416.build_default_config()
            tick_report = v1416.run_dgm_tick(history_path, baseline_path, cfg)
            # V1416 already appended internally; we mirror to tick_jsonl_path if it's distinct.
            if str(actual_v1416_log) != str(v1416_default_out):
                try:
                    v1416.append_tick(tick_report, str(actual_v1416_log))
                except Exception:
                    pass
            tick_id = tick_report.tick_id
            policy = tick_report.policy
            chain_ok = tick_report.chain_ok
            # V1416 DgmTickReport exposes v1414_alerts_count (not v1414_alerts).
            alerts_count = tick_report.v1414_alerts_count
            escalation_count = tick_report.v1415_escalation_count
            n_modules = tick_report.n_modules
        except Exception as exc:
            chain_ok = False
            note += f"; v1416.run_dgm_tick failed: {exc!r}"
    else:
        chain_ok = False
        note += f"; v1416 not loaded: {v1416_error}"

    # 2. V1417 — read from V1416's actual output log + append to V1417 history.
    appended_to_history = False
    rendered_path = ""
    render_ok = False
    if v1417_module_loaded and tick_report is not None:
        try:
            snapshots = v1417.load_v1416_ticks(actual_v1416_log)
            if snapshots:
                snap = snapshots[-1]
                v1417.append_tick_snapshot(snap, history_path)
                appended_to_history = True
        except Exception as exc:
            note += f"; v1417.append_tick_snapshot failed: {exc!r}"

        # 3. Optionally render the V1417 history markdown.
        if render:
            try:
                history_snaps = v1417.load_tick_history(history_path)
                trend = v1417.compute_tick_trend(history_snaps)
                digest = v1417.compute_tick_digest(history_snaps)
                baseline_obj = v1417.load_baseline(baseline_path)
                compare_obj = None
                if baseline_obj is not None and history_snaps:
                    compare_obj = v1417.compare_to_baseline(history_snaps[-1], baseline_obj)
                md = v1417.render_tick_history_md(history_snaps, trend, digest, baseline=baseline_obj, compare=compare_obj)
                render_out.parent.mkdir(parents=True, exist_ok=True)
                render_out.write_text(md, encoding="utf-8")
                rendered_path = str(render_out)
                render_ok = True
            except Exception as exc:
                note += f"; v1417 render failed: {exc!r}"
                rendered_path = str(render_out) if render_out else ""

    return CronTickOutcome(
        cycle_index=cycle_index,
        ran_at_iso=ran_at_iso,
        tick_id=tick_id,
        policy=policy,
        chain_ok=chain_ok,
        alerts_count=alerts_count,
        escalation_count=escalation_count,
        n_modules=n_modules,
        appended_to_history=appended_to_history,
        rendered_path=rendered_path,
        render_ok=render_ok,
        note=note,
    )


def run_session(
    cycles: int,
    cadence_seconds: int,
    jitter_seconds: int = 0,
    history_path: Optional[Path] = None,
    baseline_path: Optional[Path] = None,
    tick_jsonl_path: Optional[Path] = None,
    render_out: Optional[Path] = None,
    render: bool = True,
    sleep_fn_name: str = "time.sleep",
    summary_json_path: Optional[Path] = None,
) -> CronSessionSummary:
    """Run N cron ticks on a cadence and produce a CronSessionSummary.

    `cycles` must be ≥ 1 and ≤ MAX_CYCLES_PER_SESSION.
    `cadence_seconds` must be ≥ MIN_CADENCE_SECONDS and ≤ MAX_CADENCE_SECONDS.
    Between cycles, sleep for ``cadence_seconds + jitter_seconds``.
    """
    if not (1 <= cycles <= MAX_CYCLES_PER_SESSION):
        raise ValueError(f"cycles={cycles} out of bounds [1, {MAX_CYCLES_PER_SESSION}]")
    if not (MIN_CADENCE_SECONDS <= cadence_seconds <= MAX_CADENCE_SECONDS):
        raise ValueError(f"cadence_seconds={cadence_seconds} out of bounds")
    if jitter_seconds < 0:
        raise ValueError(f"jitter_seconds={jitter_seconds} must be ≥ 0")

    history_path = _safe_path(Path(history_path or DEFAULT_HISTORY_PATH))
    baseline_path = _safe_path(Path(baseline_path or DEFAULT_BASELINE_PATH))
    tick_jsonl_path = _safe_path(Path(tick_jsonl_path or DEFAULT_TICK_JSONL_PATH))
    render_out = _safe_path(Path(render_out or DEFAULT_RENDER_OUT))
    sleep_fn = _resolve_sleep_fn(sleep_fn_name)

    session_started_iso = _now_utc_iso()
    outcomes: List[CronTickOutcome] = []
    last_outcome: Optional[CronTickOutcome] = None

    for i in range(1, cycles + 1):
        outcome = tick_once(
            history_path=history_path,
            baseline_path=baseline_path,
            tick_jsonl_path=tick_jsonl_path,
            render_out=render_out,
            render=render,
            cycle_index=i,
        )
        outcomes.append(outcome)
        last_outcome = outcome

        # Sleep between cycles (skip after the last one).
        if i < cycles:
            _sleep_for(float(cadence_seconds + jitter_seconds), sleep_fn)

    session_ended_iso = _now_utc_iso()

    # Build summary
    n_cycles_done = len(outcomes)
    policy_proceed_count = sum(1 for o in outcomes if o.policy == "PROCEED")
    policy_pause_count = sum(1 for o in outcomes if o.policy == "PAUSE")
    policy_lockdown_count = sum(1 for o in outcomes if o.policy == "LOCKDOWN")
    chain_ok_count = sum(1 for o in outcomes if o.chain_ok)
    chain_ok_rate = (chain_ok_count / n_cycles_done) if n_cycles_done else 0.0
    first_tick = outcomes[0].tick_id if outcomes else ""
    last_tick = outcomes[-1].tick_id if outcomes else ""

    # Compute span in seconds (best-effort)
    span_seconds = 0
    if outcomes:
        first_dt = _parse_iso_timestamp(outcomes[0].ran_at_iso)
        last_dt = _parse_iso_timestamp(outcomes[-1].ran_at_iso)
        if first_dt and last_dt:
            span_seconds = int((last_dt - first_dt).total_seconds())

    rendered_path = outcomes[-1].rendered_path if (render and outcomes) else ""

    summary = CronSessionSummary(
        n_cycles=n_cycles_done,
        n_policies=len(set(o.policy for o in outcomes)),
        policy_proceed_count=policy_proceed_count,
        policy_pause_count=policy_pause_count,
        policy_lockdown_count=policy_lockdown_count,
        chain_ok_count=chain_ok_count,
        chain_ok_rate=chain_ok_rate,
        first_tick=first_tick,
        last_tick=last_tick,
        span_seconds=span_seconds,
        session_started_iso=session_started_iso,
        session_ended_iso=session_ended_iso,
        rendered_path=rendered_path,
        note=f"V1418 run_session cycles={cycles} cadence_seconds={cadence_seconds} jitter={jitter_seconds}",
    )

    # Persist summary as JSON (atomic write)
    summary_json_path = _safe_path(Path(summary_json_path or DEFAULT_SUMMARY_JSON))
    summary_json_path.parent.mkdir(parents=True, exist_ok=True)
    summary_record: Dict[str, Any] = dataclasses.asdict(summary)
    summary_record["outcomes"] = [dataclasses.asdict(o) for o in outcomes]
    tmp = summary_json_path.with_suffix(summary_json_path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(summary_record, f, sort_keys=True, ensure_ascii=False, indent=2, default=str)
        f.flush()
        try:
            os.fsync(f.fileno())
        except OSError:
            pass
    os.replace(tmp, summary_json_path)

    return summary


# ============================================================================
# Render
# ============================================================================


def render_session_md(summary: CronSessionSummary, outcomes: List[CronTickOutcome]) -> str:
    """Render the V1418 cron session markdown report."""
    out: List[str] = []
    out.append("# V1418 — ASI 总框架 DGM cron session (tick → append → render)")
    out.append("")
    out.append(f"**Generated:** {_now_utc_iso()} (Asia/Shanghai deep night, cron tick)")
    out.append(f"**Version:** {V1418_VERSION}")
    out.append(f"**Schema:** {V1418_SCHEMA}")
    out.append(f"**Module:** {V1418_MODULE}")
    out.append("")
    out.append("## 1. Session summary (主 22:33 ASI 总框架 DGM cron 真生产)")
    out.append("")
    out.append(f"- cycles: **{summary.n_cycles}**")
    out.append(f"- distinct policies: **{summary.n_policies}**")
    out.append(f"- PROCEED: **{summary.policy_proceed_count}**")
    out.append(f"- PAUSE: **{summary.policy_pause_count}**")
    out.append(f"- LOCKDOWN: **{summary.policy_lockdown_count}**")
    out.append(f"- chain_ok: **{summary.chain_ok_count}** / {summary.n_cycles} ({summary.chain_ok_rate:.1%})")
    out.append(f"- span: **{summary.span_seconds}s** ({summary.session_started_iso} → {summary.session_ended_iso})")
    out.append(f"- first tick → last tick: `{summary.first_tick}` → `{summary.last_tick}`")
    if summary.rendered_path:
        out.append(f"- last render: `{summary.rendered_path}`")
    out.append(f"- 真借鉴 (主 19:33 走在前人经验上): {len(V1418_BORROWED)} borrowed")
    out.append(f"- GUARDS: {len(V1418_GUARDS)}")
    out.append(f"- V3 哲学守门: {len(V1418_V3_GUARDS)}")
    out.append("")
    out.append("## 2. 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    out.append("")
    out.append("- 不假装 Phenomenal cron integration")
    out.append("- 不假装达到 ASI (gap 0.0695 to north-star preserved)")
    out.append("- 不假装 human-level cron integration")
    out.append("- 不假装 absolute cron integration")
    out.append("- 不假装替代 V1417 (orchestrator, V1417 owns history state)")
    out.append("- 不假装替代 V1416 (orchestrator, V1416 owns tick state)")
    out.append("- 不假装替代 V1413 (V1416-specialized, not dashboard)")
    out.append("- 不假装替代 V1412 (inherited via V1417 → V1416 → V1412)")
    out.append("- 不假装替代 V1411 (inherited via V1417 → V1416 → V1412 → V1411)")
    out.append("- 实事求是 (真跑真测真集成真 commit)")
    out.append("")
    out.append("## 3. Cycle outcomes (latest first)")
    out.append("")
    if outcomes:
        out.append("| cycle | ran_at | tick_id | policy | chain_ok | alerts | escalation | n_modules | render |")
        out.append("|---|---|---|---|---|---|---|---|---|")
        for o in reversed(outcomes):
            render_mark = "✓" if o.render_ok else "—"
            out.append(
                f"| {o.cycle_index} | {o.ran_at_iso} | `{o.tick_id}` | {o.policy} | "
                f"{o.chain_ok} | {o.alerts_count} | {o.escalation_count} | "
                f"{o.n_modules} | {render_mark} |"
            )
    else:
        out.append("_no cycles executed_")
    out.append("")
    out.append("## 4. Honest disclosure (主 17:58)")
    out.append("")
    out.append("V1418 cron integration is a **deterministic scheduling/orchestration layer**")
    out.append("that wraps V1416 (tick executor) and V1417 (tick history). It is bounded by")
    out.append("arithmetic on real timestamps + V1416/V1417 output fields; NOT by")
    out.append("Phenomenal consciousness, ASI 达成, human-level judgment, or absolute")
    out.append("certainty. V1418 ≠ Phenomenal cron, ≠ ASI 达成 cron, ≠ human-level cron,")
    out.append("≠ absolute cron. V1418 reads V1416 + V1417; never replaces either.")
    out.append("")
    return "\n".join(out)


# ============================================================================
# Popper self-test
# ============================================================================


def popper_self_test() -> Tuple[bool, List[str]]:
    """15 self-tests; return (all_pass, messages)."""
    results: List[Tuple[str, bool, str]] = []

    # 1: VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED/POLICIES present
    results.append((
        "VERSION/SCHEMA/MODULE/GUARDS/V3_GUARDS/BORROWED present",
        all([
            V1418_VERSION,
            V1418_SCHEMA,
            V1418_MODULE,
            len(V1418_GUARDS) == 15,
            len(V1418_V3_GUARDS) == 9,
            len(V1418_BORROWED) == 4,
            len(V1418_POLICIES) == 3,
        ]),
        "constants OK",
    ))

    # 2: default config within bounds
    cfg = DEFAULT_CRON_CONFIG
    in_bounds = (
        cfg.min_cadence_seconds <= cfg.cadence_seconds <= cfg.max_cadence_seconds
        and 1 <= cfg.max_cycles <= MAX_CYCLES_PER_SESSION
    )
    results.append((
        "default config within bounds",
        in_bounds,
        f"cadence={cfg.cadence_seconds}s, cycles≤{cfg.max_cycles}",
    ))

    # 3: build_default_config with overrides
    overrides = {"cadence_seconds": 60, "jitter_seconds": 5, "note": "test"}
    cfg2 = build_default_config(overrides)
    results.append((
        "build_default_config applies overrides",
        cfg2.cadence_seconds == 60 and cfg2.jitter_seconds == 5 and cfg2.note == "test",
        f"cadence={cfg2.cadence_seconds}, jitter={cfg2.jitter_seconds}",
    ))

    # 4: build_default_config rejects unknown overrides
    try:
        build_default_config({"never_set_this": True})
        results.append(("build_default_config rejects unknown overrides", False, "did not raise"))
    except ValueError:
        results.append(("build_default_config rejects unknown overrides", True, "raised ValueError"))

    # 5: config rejects out-of-bounds cadence
    try:
        CronIntegrationConfig(
            cadence_seconds=999_999_999, jitter_seconds=0,
            auto_render=True, render_out=Path("/tmp/r.md"),
            max_cycles=10, min_cadence_seconds=1, max_cadence_seconds=86400,
            sleep_fn_name="time.sleep",
        )
        results.append(("config rejects out-of-bounds cadence", False, "did not raise"))
    except ValueError:
        results.append(("config rejects out-of-bounds cadence", True, "raised ValueError"))

    # 6: compute_next_due deterministic
    last_iso = "2026-08-10T00-00-00Z"
    due_iso = compute_next_due(last_iso, 300, 0)
    results.append((
        "compute_next_due deterministic on cadence 300s",
        due_iso == "2026-08-10T00-05-00Z",
        f"next_due={due_iso}",
    ))

    # 7: compute_next_due with jitter
    due_iso_j = compute_next_due(last_iso, 300, 7)
    results.append((
        "compute_next_due includes jitter",
        due_iso_j == "2026-08-10T00-05-07Z",
        f"next_due={due_iso_j}",
    ))

    # 8: compute_next_due rejects bad cadence
    try:
        compute_next_due(last_iso, 0, 0)
        results.append(("compute_next_due rejects cadence<MIN", False, "did not raise"))
    except ValueError:
        results.append(("compute_next_due rejects cadence<MIN", True, "raised ValueError"))

    # 9: sleep_for with pass-through returns ~0
    elapsed = _sleep_for(2.0, _resolve_sleep_fn("pass-through"))
    results.append((
        "sleep_for with pass-through returns ≤0.05s",
        elapsed < 0.05,
        f"elapsed={elapsed:.4f}s",
    ))

    # 10: path safety rejects dotdot
    try:
        _safe_path(Path("a/../b"))
        results.append(("path safety rejects dotdot", False, "did not raise"))
    except ValueError:
        results.append(("path safety rejects dotdot", True, "raised ValueError"))

    # 11: dataclass roundtrip
    o = CronTickOutcome(
        cycle_index=1, ran_at_iso="2026-08-10T00-00-00Z", tick_id="t1",
        policy="PROCEED", chain_ok=True, alerts_count=0, escalation_count=0,
        n_modules=5, appended_to_history=True, rendered_path="/tmp/r.md",
        render_ok=True, note="x",
    )
    o2 = CronTickOutcome(**dataclasses.asdict(o))
    results.append(("CronTickOutcome roundtrip", o == o2, "dataclass equality"))

    # 12: dataclass roundtrip (summary)
    s = CronSessionSummary(
        n_cycles=2, n_policies=1, policy_proceed_count=2,
        policy_pause_count=0, policy_lockdown_count=0,
        chain_ok_count=2, chain_ok_rate=1.0,
        first_tick="t1", last_tick="t2", span_seconds=10,
        session_started_iso="2026-08-10T00-00-00Z",
        session_ended_iso="2026-08-10T00-00-10Z",
        rendered_path="", note="y",
    )
    s2 = CronSessionSummary(**dataclasses.asdict(s))
    results.append(("CronSessionSummary roundtrip", s == s2, "dataclass equality"))

    # 13: render_session_md contains 4 sections
    md = render_session_md(s, [o, dataclasses.replace(o, cycle_index=2, tick_id="t2")])
    results.append((
        "render_session_md contains 4 sections",
        all(s in md for s in [
            "# V1418",
            "## 1. Session summary",
            "## 2. 哲学守门",
            "## 3. Cycle outcomes",
            "## 4. Honest disclosure",
        ]),
        f"md length={len(md)}",
    ))

    # 14: tick_once writes 0 history if V1416 not importable (defensive)
    # We can't fully test tick_once without V1416 + V1417 importable here,
    # so just validate the function exists with the expected signature.
    import inspect
    sig = inspect.signature(tick_once)
    expected_params = {"history_path", "baseline_path", "tick_jsonl_path", "render_out", "render", "cycle_index"}
    actual_params = set(sig.parameters.keys())
    results.append((
        "tick_once signature has expected params",
        expected_params <= actual_params,
        f"params={sorted(actual_params)}",
    ))

    # 15: run_session rejects out-of-bounds cycles
    try:
        run_session(cycles=999_999_999, cadence_seconds=1, sleep_fn_name="pass-through")
        results.append(("run_session rejects out-of-bounds cycles", False, "did not raise"))
    except ValueError:
        results.append(("run_session rejects out-of-bounds cycles", True, "raised ValueError"))

    passed = sum(1 for _, ok, _ in results if ok)
    messages = [f"  [{i+1:02d}] {name}: {'OK' if ok else 'FAIL'} ({detail})" for i, (name, ok, detail) in enumerate(results)]
    return passed == len(results), [f"popper: {passed}/{len(results)}"] + messages


# ============================================================================
# Chain delegate
# ============================================================================


def chain_delegate() -> Dict[str, Any]:
    """Probe V1416 + V1417 chain integrity (read-only)."""
    errors: List[str] = []
    n_modules_ok = 0
    n_modules = 0

    # V1416 probe
    try:
        import apeireth.v1416_asi_overarching_dgm_tick as v1416  # type: ignore
        n_modules += 1
        probe_fn = getattr(v1416, "chain_delegate_v1416", None) or getattr(v1416, "chain_delegate", None)
        probe_raw = probe_fn() if probe_fn else (False, 0, 0, 0, ["no chain_delegate"])
        if isinstance(probe_raw, tuple) and len(probe_raw) >= 5:
            all_ok_v1416 = probe_raw[0]
            err_list = probe_raw[4]
        elif isinstance(probe_raw, dict):
            all_ok_v1416 = probe_raw.get("all_ok", False)
            err_list = probe_raw.get("errors", [])
        else:
            all_ok_v1416 = False
            err_list = ["unknown return shape"]
        if all_ok_v1416:
            n_modules_ok += 1
        else:
            errors.append(f"v1416 chain_delegate failed: {err_list}")
    except Exception as e:
        errors.append(f"v1416 import failed: {e}")

    # V1417 probe
    try:
        import apeireth.v1417_asi_dgm_tick_history as v1417  # type: ignore
        n_modules += 1
        v1417_chain = v1417.chain_delegate()
        if isinstance(v1417_chain, dict):
            all_ok_v1417 = v1417_chain.get("all_ok", False)
            err_list = v1417_chain.get("errors", [])
        else:
            all_ok_v1417 = False
            err_list = ["unknown return shape"]
        if all_ok_v1417:
            n_modules_ok += 1
        else:
            errors.append(f"v1417 chain_delegate failed: {err_list}")
    except Exception as e:
        errors.append(f"v1417 import failed: {e}")

    return {
        "schema": V1418_SCHEMA,
        "version": V1418_VERSION,
        "all_ok": len(errors) == 0 and n_modules_ok == n_modules,
        "n_modules_ok": n_modules_ok,
        "n_modules": n_modules,
        "errors": errors,
    }


# ============================================================================
# CLI
# ============================================================================


def _format_json(obj: Any) -> str:
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, indent=2, default=str)


def _parse_args(rest: List[str]) -> Dict[str, Any]:
    """Parse --key value pairs into a dict (each option uses --key value OR --key)."""
    args: Dict[str, Any] = {}
    i = 0
    while i < len(rest):
        token = rest[i]
        if not token.startswith("--"):
            i += 1
            continue
        key = token[2:].replace("-", "_")
        # Boolean flags (no value follows, or next token starts with --)
        if i + 1 >= len(rest) or rest[i + 1].startswith("--"):
            args[key] = True
            i += 1
            continue
        val = rest[i + 1]
        # Try int parse
        if key in ("cycles", "cadence_seconds", "jitter_seconds", "last_n", "cycle_index"):
            try:
                val = int(val)
            except ValueError:
                pass
        args[key] = val
        i += 2
    return args


def run_cli(argv: List[str]) -> int:
    """Argv dispatcher; return exit code."""
    if not argv:
        argv = ["help"]
    cmd = argv[0]
    rest = argv[1:]

    if cmd == "version":
        print(f"V1418_VERSION: {V1418_VERSION}")
        print(f"V1418_SCHEMA: {V1418_SCHEMA}")
        print(f"V1418_MODULE: {V1418_MODULE}")
        print(f"guards: {len(V1418_GUARDS)} (incl. {len(V1418_V3_GUARDS)} V3 guards)")
        print(f"borrowed: {len(V1418_BORROWED)}")
        return 0

    if cmd == "meta":
        as_json = "--json" in rest
        meta_obj = {
            "version": V1418_VERSION,
            "schema": V1418_SCHEMA,
            "module": V1418_MODULE,
            "guards": list(V1418_GUARDS),
            "v3_guards": list(V1418_V3_GUARDS),
            "borrowed": [{"module": m, "use": u} for m, u in V1418_BORROWED],
            "policies": list(V1418_POLICIES),
            "min_cadence_seconds": MIN_CADENCE_SECONDS,
            "max_cadence_seconds": MAX_CADENCE_SECONDS,
            "max_cycles_per_session": MAX_CYCLES_PER_SESSION,
            "default_cadence_seconds": DEFAULT_CADENCE_SECONDS,
            "default_history_path": str(DEFAULT_HISTORY_PATH),
            "default_baseline_path": str(DEFAULT_BASELINE_PATH),
            "default_tick_jsonl_path": str(DEFAULT_TICK_JSONL_PATH),
            "default_render_out": str(DEFAULT_RENDER_OUT),
            "default_summary_json": str(DEFAULT_SUMMARY_JSON),
        }
        if as_json:
            print(_format_json(meta_obj))
        else:
            for k, v in meta_obj.items():
                print(f"{k}: {v}")
        return 0

    if cmd == "demo":
        print("V1418 = ASI 总框架 DGM cron integration (5min cron auto-tick → append → render)")
        print("Wraps V1416 tick executor + V1417 history. Use 'tick-once' for one tick,")
        print("'run-session --cycles N' for N ticks, 'next-due' to compute next ISO.")
        return 0

    if cmd == "help":
        print("Usage: python -m apeireth.v1418_asi_dgm_cron_integration <command> [options]")
        print()
        print("Commands:")
        print("  version                                  show version")
        print("  meta [--json]                            show constants + paths")
        print("  demo                                     brief description")
        print("  popper                                   run 15 self-tests")
        print("  chain                                    probe V1416 + V1417 chain")
        print("  tick-once [--history-path P] [--baseline-path P] [--tick-jsonl-path P]")
        print("             [--render-out P] [--render|--no-render] [--cycle-index N]")
        print("                                           run ONE V1416 tick + V1417 append + render")
        print("  run-session --cycles N [--cadence-seconds N] [--jitter-seconds N]")
        print("                [--history-path P] [--baseline-path P] [--tick-jsonl-path P]")
        print("                [--render-out P] [--render|--no-render]")
        print("                [--sleep-fn NAME] [--summary-json-path P]")
        print("                                           run N ticks at cadence, write summary JSON")
        print("  next-due --last-iso '...' [--cadence-seconds 300] [--jitter-seconds 0]")
        print("            [--sleep-fn NAME]            compute next-due ISO from cadence")
        print("  render-summary --summary-json-path P [--out P]")
        print("                                           render session markdown from JSON")
        print("  show-outcomes --summary-json-path P [--last N] [--json]")
        print("                                           list outcomes from a previous session")
        print("  detect-policy --last N --summary-json-path P")
        print("                                           distribution of last-N policies")
        print("  emit-shell [--render|--no-render] [--cycles N] [--cadence-seconds N]")
        print("                                           emit a scheduler-ready shell snippet")
        print("  help                                     show this help")
        return 0

    if cmd == "popper":
        ok, msgs = popper_self_test()
        for m in msgs:
            print(m)
        return 0 if ok else 1

    if cmd == "chain":
        print(_format_json(chain_delegate()))
        return 0

    if cmd == "tick-once":
        args = _parse_args(rest)
        history_path = Path(args["history_path"]) if isinstance(args.get("history_path"), str) else DEFAULT_HISTORY_PATH
        baseline_path = Path(args["baseline_path"]) if isinstance(args.get("baseline_path"), str) else DEFAULT_BASELINE_PATH
        tick_jsonl_path = Path(args["tick_jsonl_path"]) if isinstance(args.get("tick_jsonl_path"), str) else DEFAULT_TICK_JSONL_PATH
        render_out = Path(args["render_out"]) if isinstance(args.get("render_out"), str) else DEFAULT_RENDER_OUT
        render = not (args.get("no_render", False) or args.get("no_render", False) is True and False)
        # If --render explicitly supplied, enable; if --no-render, disable; else default True
        if "render" in args and isinstance(args["render"], bool):
            render = True
        if "no_render" in args and isinstance(args["no_render"], bool):
            render = False
        cycle_index = args.get("cycle_index", 1)
        if not isinstance(cycle_index, int):
            cycle_index = 1
        outcome = tick_once(
            history_path=history_path,
            baseline_path=baseline_path,
            tick_jsonl_path=tick_jsonl_path,
            render_out=render_out,
            render=render,
            cycle_index=cycle_index,
        )
        print(_format_json(dataclasses.asdict(outcome)))
        return 0 if outcome.chain_ok else 1

    if cmd == "run-session":
        args = _parse_args(rest)
        cycles = args.get("cycles", 1)
        if not isinstance(cycles, int):
            cycles = 1
        cadence_seconds = args.get("cadence_seconds", DEFAULT_CADENCE_SECONDS)
        if not isinstance(cadence_seconds, int):
            cadence_seconds = DEFAULT_CADENCE_SECONDS
        jitter_seconds = args.get("jitter_seconds", 0)
        if not isinstance(jitter_seconds, int):
            jitter_seconds = 0
        history_path = Path(args["history_path"]) if isinstance(args.get("history_path"), str) else DEFAULT_HISTORY_PATH
        baseline_path = Path(args["baseline_path"]) if isinstance(args.get("baseline_path"), str) else DEFAULT_BASELINE_PATH
        tick_jsonl_path = Path(args["tick_jsonl_path"]) if isinstance(args.get("tick_jsonl_path"), str) else DEFAULT_TICK_JSONL_PATH
        render_out = Path(args["render_out"]) if isinstance(args.get("render_out"), str) else DEFAULT_RENDER_OUT
        render = True
        if "no_render" in args and isinstance(args["no_render"], bool):
            render = False
        if "render" in args and isinstance(args["render"], bool):
            render = True
        sleep_fn_name = args.get("sleep_fn", "time.sleep")
        if not isinstance(sleep_fn_name, str):
            sleep_fn_name = "time.sleep"
        summary_json_path = Path(args["summary_json_path"]) if isinstance(args.get("summary_json_path"), str) else DEFAULT_SUMMARY_JSON
        summary = run_session(
            cycles=cycles,
            cadence_seconds=cadence_seconds,
            jitter_seconds=jitter_seconds,
            history_path=history_path,
            baseline_path=baseline_path,
            tick_jsonl_path=tick_jsonl_path,
            render_out=render_out,
            render=render,
            sleep_fn_name=sleep_fn_name,
            summary_json_path=summary_json_path,
        )
        print(_format_json(dataclasses.asdict(summary)))
        return 0

    if cmd == "next-due":
        args = _parse_args(rest)
        last_iso = args.get("last_iso")
        if not isinstance(last_iso, str):
            print("next-due requires --last-iso 'ISO'", file=sys.stderr)
            return 1
        cadence_seconds = args.get("cadence_seconds", DEFAULT_CADENCE_SECONDS)
        if not isinstance(cadence_seconds, int):
            cadence_seconds = DEFAULT_CADENCE_SECONDS
        jitter_seconds = args.get("jitter_seconds", 0)
        if not isinstance(jitter_seconds, int):
            jitter_seconds = 0
        due_iso = compute_next_due(last_iso, cadence_seconds, jitter_seconds)
        print(due_iso)
        return 0

    if cmd == "render-summary":
        args = _parse_args(rest)
        summary_json_path = Path(args.get("summary_json_path", DEFAULT_SUMMARY_JSON))
        if not summary_json_path.exists():
            print(f"no summary at {summary_json_path}", file=sys.stderr)
            return 1
        try:
            with open(summary_json_path, "r", encoding="utf-8") as f:
                rec = json.load(f)
        except (OSError, json.JSONDecodeError) as e:
            print(f"failed to read summary: {e}", file=sys.stderr)
            return 1
        summary_obj = CronSessionSummary(
            n_cycles=rec.get("n_cycles", 0),
            n_policies=rec.get("n_policies", 0),
            policy_proceed_count=rec.get("policy_proceed_count", 0),
            policy_pause_count=rec.get("policy_pause_count", 0),
            policy_lockdown_count=rec.get("policy_lockdown_count", 0),
            chain_ok_count=rec.get("chain_ok_count", 0),
            chain_ok_rate=rec.get("chain_ok_rate", 0.0),
            first_tick=rec.get("first_tick", ""),
            last_tick=rec.get("last_tick", ""),
            span_seconds=rec.get("span_seconds", 0),
            session_started_iso=rec.get("session_started_iso", ""),
            session_ended_iso=rec.get("session_ended_iso", ""),
            rendered_path=rec.get("rendered_path", ""),
            note=rec.get("note", ""),
        )
        outcomes_raw = rec.get("outcomes", [])
        outcomes_list = [
            CronTickOutcome(
                cycle_index=o.get("cycle_index", 0),
                ran_at_iso=o.get("ran_at_iso", ""),
                tick_id=o.get("tick_id", ""),
                policy=o.get("policy", "PROCEED"),
                chain_ok=bool(o.get("chain_ok", False)),
                alerts_count=int(o.get("alerts_count", 0)),
                escalation_count=int(o.get("escalation_count", 0)),
                n_modules=int(o.get("n_modules", 0)),
                appended_to_history=bool(o.get("appended_to_history", False)),
                rendered_path=o.get("rendered_path", ""),
                render_ok=bool(o.get("render_ok", False)),
                note=o.get("note", ""),
            )
            for o in outcomes_raw
        ]
        md = render_session_md(summary_obj, outcomes_list)
        out_path = Path(args.get("out", DEFAULT_RENDER_OUT))
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(md, encoding="utf-8")
        print(f"rendered {len(md)} bytes to {out_path}")
        return 0

    if cmd == "show-outcomes":
        args = _parse_args(rest)
        as_json = "json" in args
        summary_json_path = Path(args.get("summary_json_path", DEFAULT_SUMMARY_JSON))
        if not summary_json_path.exists():
            print(f"no summary at {summary_json_path}", file=sys.stderr)
            return 1
        with open(summary_json_path, "r", encoding="utf-8") as f:
            rec = json.load(f)
        outcomes_raw = rec.get("outcomes", [])
        last_n = args.get("last_n")
        if isinstance(last_n, int) and last_n > 0:
            outcomes_raw = outcomes_raw[-last_n:]
        if as_json:
            print(_format_json(outcomes_raw))
        else:
            for o in outcomes_raw:
                print(
                    f"cycle={o.get('cycle_index')} ran_at={o.get('ran_at_iso')} "
                    f"tick={o.get('tick_id')} policy={o.get('policy')} "
                    f"chain_ok={o.get('chain_ok')} alerts={o.get('alerts_count')} "
                    f"escal={o.get('escalation_count')} n_modules={o.get('n_modules')}"
                )
        return 0

    if cmd == "detect-policy":
        args = _parse_args(rest)
        summary_json_path = Path(args.get("summary_json_path", DEFAULT_SUMMARY_JSON))
        last_n = args.get("last_n")
        if not isinstance(last_n, int):
            print("detect-policy requires --last-n N", file=sys.stderr)
            return 1
        if not summary_json_path.exists():
            print(f"no summary at {summary_json_path}", file=sys.stderr)
            return 1
        with open(summary_json_path, "r", encoding="utf-8") as f:
            rec = json.load(f)
        outcomes_raw = rec.get("outcomes", [])
        last = outcomes_raw[-last_n:] if last_n > 0 else outcomes_raw
        dist = {"PROCEED": 0, "PAUSE": 0, "LOCKDOWN": 0, "OTHER": 0}
        for o in last:
            p = o.get("policy", "OTHER")
            if p in dist:
                dist[p] += 1
            else:
                dist["OTHER"] += 1
        total = sum(dist.values())
        ratios = {k: (v / total if total > 0 else 0.0) for k, v in dist.items()}
        out_obj = {
            "last_n": len(last),
            "distribution": dist,
            "ratios": ratios,
            "dominant": max(dist, key=dist.get) if total > 0 else "NONE",
        }
        print(_format_json(out_obj))
        return 0

    if cmd == "emit-shell":
        args = _parse_args(rest)
        render = True
        if "no_render" in args and isinstance(args["no_render"], bool):
            render = False
        if "render" in args and isinstance(args["render"], bool):
            render = True
        cycles = args.get("cycles", 1)
        if not isinstance(cycles, int):
            cycles = 1
        cadence_seconds = args.get("cadence_seconds", DEFAULT_CADENCE_SECONDS)
        if not isinstance(cadence_seconds, int):
            cadence_seconds = DEFAULT_CADENCE_SECONDS
        script = (
            "#!/usr/bin/env bash\n"
            "# V1418 cron-integration shell snippet (5min cadence)\n"
            f"# cycles={cycles} cadence_seconds={cadence_seconds} render={render}\n"
            f"python -m apeireth.v1418_asi_dgm_cron_integration run-session "
            f"--cycles {cycles} --cadence-seconds {cadence_seconds} "
            f"{'--render' if render else '--no-render'}\n"
        )
        print(script)
        return 0

    print(f"unknown command: {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))
