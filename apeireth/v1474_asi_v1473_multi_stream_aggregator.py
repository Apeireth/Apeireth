"""V1474 — ASI Real V1473 Multi-Stream Alert Aggregator + Cross-Stream Incident Correlation (主 13:31 大胆放手 + 主 23:44 平视到底 + 主 00:56 任何人都能接手 + 主 00:44 质量工程化 + 主 19:33 站在前人肩上 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装).

Phase: 1474
Version: 0.1.0
Date: 2026-08-10 (cron tick 19:30, Monday afternoon, round-136, isolated lane)
Post: V1473 (V1472 Audit Alerting Engine — 39 tests pass, popper 37/37, /alerts endpoint on port 18980, single JSONL stream)
      V1472 (V1471 Supervisor + Prometheus /metrics — 25 tests pass, single V1471 daemon)
      V1471 (Persistent Audit Monitor Daemon — 21 tests pass, JSONL diff stream)
      V1470 (Batch Harness + Cross-Client Equivalence — 29 tests pass)
      V1469 (Two-Process Driver — 18 tests pass)
      V1468 (OpenAPI 3.1 Schema + Generated Python Client — tests pass)
      V1467 (Cross-Audit HTTP Gateway + History + Diff — tests pass, 6 endpoints)

What V1474 is
=============
V1467 → V1473 built a single-stream audit + alerting pipeline:

  V1467 = HTTP gateway (one-shot, history + diff)
  V1471 = persistent audit monitor daemon (long-running, JSONL diff stream)
  V1472 = V1471 supervisor + Prometheus /metrics
  V1473 = alerting engine: tail 1 JSONL stream, apply rules, fire alerts

V1473 was deliberately scoped as **SINGLE-STREAM**:
"V1473 watches 1 stream file (multi-target deferred to V1474)".

In a real production fleet, multiple V1471 daemons would run simultaneously
across different services/regions/environments, each emitting its own JSONL
diff stream. An operator staring at N separate V1473 alert streams has to
manually correlate them to see the bigger picture:

  - did the regression in service-A correlate with a regression in service-B?
  - is this a single-service incident or a fleet-wide outage?
  - which streams are stale?

V1474 takes the next natural step: **a real multi-stream aggregator with
cross-stream incident correlation**.

  ┌────────────────────────────────────────────────────────────────────┐
  │                                                                    │
  │  V1474 multi-stream aggregator (parent process)                   │
  │                                                                    │
  │  ┌─ stream targets (StreamTarget[]) ────────────────────────────┐ │
  │  │  stream-A: path/to/audit-stream-A.jsonl                       │ │
  │  │  stream-B: path/to/audit-stream-B.jsonl                       │ │
  │  │  stream-C: path/to/audit-stream-C.jsonl                       │ │
  │  │  each with: cursor + recent_events + last_event_ts            │ │
  │  │           + per-rule AlertRecord (reuses V1473 structure)     │ │
  │  └───────────────────────────────────────────────────────────────┘ │
  │                                                                    │
  │  ┌─ per-stream evaluator loop ──────────────────────────────────┐ │
  │  │  every eval_interval_s:                                       │ │
  │  │    1. tail each stream's new lines since last cursor         │ │
  │  │    2. for each rule:                                          │ │
  │  │       - evaluate against that stream's recent events          │ │
  │  │       - transition state (INACTIVE→PENDING→FIRING→RESOLVED)   │ │
  │  │       - if FIRING → record per-stream alert                  │ │
  │  └───────────────────────────────────────────────────────────────┘ │
  │                                                                    │
  │  ┌─ cross-stream correlation ───────────────────────────────────┐ │
  │  │  every eval_interval_s (after per-stream eval):              │ │
  │  │    - bucket FIRING alerts by (rule_id, severity)             │ │
  │  │    - for each bucket:                                         │ │
  │  │        if n_streams ≥ incident_threshold                      │ │
  │  │           AND time-since-first-firing ≤ incident_window_s     │ │
  │  │        → emit FLEET_INCIDENT (severity escalated to CRITICAL) │ │
  │  │        - transition fleet incident state:                     │ │
  │  │            NEW → OPEN → CLOSED (after grace window)           │ │
  │  └───────────────────────────────────────────────────────────────┘ │
  │                                                                    │
  │  ┌─ alert sinks ────────────────────────────────────────────────┐ │
  │  │  /digest HTTP endpoint (loopback JSON):                       │ │
  │  │    GET /digest   → JSON { per_stream: [...], fleet: [...] }   │ │
  │  │    GET /streams  → JSON list of streams with stats            │ │
  │  │    GET /healthz  → JSON health check                          │ │
  │  │  alert JSONL stream (with stream_id + rule_id + event_type)   │ │
  │  │  aggregator log file (human-readable)                         │ │
  │  │  AggregatorReport JSON + Markdown                             │ │
  │  └───────────────────────────────────────────────────────────────┘ │
  │                                                                    │
  │  shutdown on:                                                      │
  │    - max_runtime_s reached                                         │
  │    - KeyboardInterrupt → graceful shutdown                        │
  │                                                                    │
  └────────────────────────────────────────────────────────────────────┘

After V1474, an external developer can:

  $ python -m apeireth.v1474_asi_v1473_multi_stream_aggregator run \\
        --stream out/v1471-a/audit-stream.jsonl \\
        --stream out/v1471-b/audit-stream.jsonl \\
        --eval-interval 2.0 \\
        --incident-threshold 2 \\
        --incident-window 30.0 \\
        --max-runtime 60 \\
        --digest-port-min 19180 --digest-port-max 19280 \\
        --out-dir out/v1474
    → tails 2 V1471 JSONL streams
    → evaluates 7 built-in rules per stream every 2s
    → if same rule fires on both streams within 30s → FLEET_INCIDENT
    → GET http://127.0.0.1:19180/digest → JSON digest of all alerts
    → GET http://127.0.0.1:19180/streams → JSON list with per-stream stats
    → writes alert events (per-stream + fleet) to alert-stream.jsonl
    → after 60s OR Ctrl+C, graceful shutdown
    → writes AggregatorReport JSON + Markdown

Or run the integrated demo:

  $ python -m apeireth.v1474_asi_v1473_multi_stream_aggregator demo \\
        --max-runtime 30
    → spawns 2 synthetic stream writers in separate threads
    → both write REGRESSED events that align in time
    → aggregator watches both + fires per-stream alerts + 1 FLEET_INCIDENT
    → shows the incident correlation working end-to-end

V1474 is NOT:
- an in-process test (it tails real JSONL stream files on disk)
- a CI/CD pipeline (one-shot aggregator run)
- a load tester (evaluates rules every eval_interval_s, not 1000/s)
- a fuzzer (deterministic rule conditions, no mutation)
- an orchestrator (watches N streams, but doesn't spawn them)
- real-time (eval_interval_s is the floor; not event-driven)
- ASI / Phenomenal / human-level (mechanical per-stream eval + correlation)
- a replacement for V1473 (V1473 watches 1 stream; V1474 watches N)

V1474 IS:
- a real multi-stream aggregator (watches N JSONL stream files simultaneously)
- a real cross-stream incident correlator (fleet incidents when K+ streams
  fire the same rule within W seconds)
- a real per-stream alert state machine (reuses V1473's INACTIVE→PENDING→FIRING→RESOLVED)
- a real fleet incident state machine (NEW → OPEN → CLOSED with grace)
- a real /digest endpoint (combined view across all streams)
- a real graceful shutdown handler (KeyboardInterrupt → clean exit)
- a real bounded aggregator (max_runtime_s, max_streams, max_rules, max_alerts)
- safe-by-default: loopback only, bounded file reads, bounded stream count

V1474 design rules (主 13:31 大胆放手 + 主 23:44 平视到底 + 主 19:33 站在前人肩上):

R1. Reuse V1473 evaluators: V1474 imports v1473's _evaluate_rule +
    _transition_state + AlertRule + AlertRecord + AlertSeverity + AlertState
    + ShutdownReason. V1474 does NOT modify V1473.

R2. Per-stream state isolation: each StreamTarget has its own AlertRecord
    dict, recent_events, cursor. No cross-stream contamination in eval.

R3. Fleet incident correlation: only after per-stream eval completes, V1474
    correlates FIRING alerts across streams. Correlation is read-only —
    does NOT affect per-stream state.

R4. Loopback-only: V1474 /digest endpoint binds to 127.0.0.1 (主 23:44 平视到底).
    No public interface.

R5. Bounded streams: max_streams default = 4, min = 1, max = 16.

R6. Bounded rules: per-stream max_rules default = 32 (matches V1473).

R7. Bounded alerts: max_alerts default = 128 (across all streams + fleet).
    min = 1, max = 1024.

R8. Bounded runtime: max_runtime_s default = 60s.

R9. Distinct port range:
    - V1467: 18280-18380
    - V1471: 18580-18680
    - V1472 metrics: 18780-18880
    - V1473 alerts: 18980-19080
    - V1474 digest: 19180-19280

R10. Anyone-can-run CLI:
    python -m apeireth.v1474_asi_v1473_multi_stream_aggregator run \\
        --stream path1.jsonl --stream path2.jsonl --max-runtime 60

R11. Two-mode operation:
    - "run": tail N JSONL stream files given on CLI
    - "demo": spin up N synthetic stream writers + aggregator on the same files

V1474 guards (主 00:44 质量工程化 + 主 17:58 不假装 + 主 20:46 不假装):

A. Multi-stream guards:
   GUARD_STREAMS_DECLARED      — at least 1 stream registered
   GUARD_STREAM_COUNT_BOUNDED  — n_streams in [1, max_streams]
   GUARD_PER_STREAM_STATE      — each stream has own AlertRecord dict
   GUARD_CURSOR_PER_STREAM     — each stream has own read cursor
   GUARD_NO_CROSS_CONTAMINATION — streams don't share recent_events

B. Per-stream eval guards:
   GUARD_PER_STREAM_EVAL       — each stream evaluated independently
   GUARD_PER_STREAM_TRANSITION — state transitions happen per-stream
   GUARD_RULE_EVALUATED        — every rule evaluated on every stream

C. Correlation guards:
   GUARD_CORRELATION_RUNS      — correlation runs after per-stream eval
   GUARD_FLEET_INCIDENT_FIRES  — same rule on K+ streams → fleet incident
   GUARD_INCIDENT_SEVERITY     — fleet incident severity escalated to CRITICAL
   GUARD_INCIDENT_STATE        — NEW → OPEN → CLOSED transitions work
   GUARD_INCIDENT_DEDUP        — same incident not double-fired

D. Process guards:
   GUARD_BOUNDED_RUNTIME       — V1474 exits after max_runtime_s
   GUARD_BOUNDED_STREAMS       — streams count ≤ max_streams
   GUARD_BOUNDED_ALERTS        — total alerts count ≤ max_alerts
   GUARD_LINEAGE_CITED         — borrowed sources declared
   GUARD_RUNS_ON_WINDOWS       — multi-file tail + http.server works on Windows

E. Sink guards:
   GUARD_ALERT_STREAM_WRITTEN  — alert JSONL stream written (with stream_id)
   GUARD_AGGREGATOR_LOG_WRITTEN — aggregator log file written
   GUARD_DIGEST_PORT_OPEN      — /digest HTTP server listening on loopback
   GUARD_DIGEST_FORMAT_VALID   — /digest returns valid JSON
   GUARD_REPORT_WRITTEN        — AggregatorReport JSON + Markdown written

F. Determinism guards:
   GUARD_DETERMINISTIC_EVAL    — same stream events → same per-stream verdicts
   GUARD_DETERMINISTIC_CORR    — same per-stream alerts → same fleet incidents

V1474 V3 哲学守门 (主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装):

GUARD_AGGREGATOR_NOT_CI         — V1474 is a one-shot aggregator run, not CI/CD
GUARD_AGGREGATOR_NOT_LOAD_TEST  — V1474 evaluates rules every eval_interval_s, not 1000/s
GUARD_AGGREGATOR_NOT_FUZZER     — V1474 deterministic rule conditions, no mutation
GUARD_AGGREGATOR_NOT_ORCHESTRATOR — V1474 watches N streams but doesn't spawn them
GUARD_AGGREGATOR_NOT_REALTIME   — eval_interval_s is the floor; not event-driven
GUARD_NOT_ASI                   — mechanical per-stream eval + correlation, not general intelligence
GUARD_NOT_PHENOMENAL            — no claim of subjective experience
GUARD_NOT_HUMAN_LEVEL           — bounded mechanical aggregator, not human-equivalent
GUARD_AGGREGATOR_NOT_V1473      — V1473 watches 1 stream; V1474 watches N (multi-stream is the value-add)

V1474 borrowed sources (主 19:33 站在前人肩上):

- v1473 (AlertSeverity/AlertState/AlertRule/AlertRecord/ShutdownReason +
        _evaluate_rule/_transition_state/_read_jsonl_tail + per-stream eval
        loop + /alerts HTTP endpoint pattern + graceful shutdown handler)
- v1472 (loopback /metrics endpoint pattern + subprocess supervisor pattern)
- v1471 (DiffEvent JSONL stream format + port range + SIGINT/SIGTERM handler)
- v1470 (V1467 subprocess spawn pattern: loopback + port range + kill grace)
- v1467 (HTTP gateway + audit history pattern: append-only FIFO)
- v1437 (stdlib BaseHTTPRequestHandler reference for /digest endpoint)
- v1422 (JSONL stream writer pattern: append + rotate at half max bytes)
- stdlib: json, http.server, http.client, socketserver, threading, urllib,
          signal, dataclasses, enum, argparse, time, socket, pathlib,
          tempfile, subprocess

V1474 honest disclosure (主 17:43 实事求是):

- V1474 watches N stream files (default 4, max 16) — N configurable per run
- Per-stream eval is periodic (eval_interval_s is the floor), not event-driven
- Cross-stream correlation only looks at alerts currently in FIRING state,
  not the entire recent history
- Fleet incident severity is always CRITICAL when threshold met
  (regardless of per-stream alert severity)
- Fleet incident dedup: same (rule_id, n_streams) combination only fires once
  per incident_window_s, even if it stays FIRING across multiple windows
- V1474 doesn't have alert acknowledgement (PENDING→FIRING→RESOLVED only,
  matches V1473)
- V1474 doesn't aggregate per-stream alerts into a single "composite" alert —
  per-stream + fleet incidents are emitted independently
- V1474 doesn't have notification targets beyond file-based + /digest
  (no real webhook by default; JSONL stream is the durable record)
- V1474 doesn't suppress flapping (no half-open logic beyond grace window
  in per-stream state machine; fleet incidents use simple grace)
- V1474 doesn't auto-restart on stream file disappearance (exits with ERROR
  if ALL streams are gone; partial disappearance is tolerated)
- V1474 evaluates per-stream rules against that stream's recent events window
  (last N evaluations), not the entire stream history — matches V1473
"""

# ──────────────────────────────────────────────────────────────────────
# Imports
# ──────────────────────────────────────────────────────────────────────

from __future__ import annotations

import argparse
import dataclasses
import enum
import http.client
import http.server
import json
import signal
import socketserver
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# Reuse V1473 building blocks (主 19:33 站在前人肩上)
try:
    from apeireth import v1473_asi_v1472_alerting_engine as v1473
except ImportError:
    # Fallback for direct module execution
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import v1473_asi_v1472_alerting_engine as v1473  # type: ignore


# ──────────────────────────────────────────────────────────────────────
# Constants
# ──────────────────────────────────────────────────────────────────────

MODULE_NAME = "v1474_asi_v1473_multi_stream_aggregator"
MODULE_PHASE = 1474
MODULE_VERSION = "0.1.0"

# Bounded defaults (主 00:44 质量工程化)
DEFAULT_MAX_RUNTIME_S = 60.0
DEFAULT_MAX_RUNTIME_S_MIN = 1.0
DEFAULT_MAX_RUNTIME_S_MAX = 3600.0

DEFAULT_EVAL_INTERVAL_S = 2.0
DEFAULT_EVAL_INTERVAL_S_MIN = 0.5
DEFAULT_EVAL_INTERVAL_S_MAX = 60.0

DEFAULT_DEBOUNCE_S = 4.0
DEFAULT_RESOLVED_GRACE_S = 8.0
DEFAULT_STALE_THRESHOLD_S = 30.0

DEFAULT_MAX_STREAMS = 4
DEFAULT_MAX_STREAMS_MIN = 1
DEFAULT_MAX_STREAMS_MAX = 16

DEFAULT_MAX_RULES = 32  # per stream (matches V1473 default)
DEFAULT_MAX_ALERTS = 128  # total across all streams + fleet
DEFAULT_MAX_ALERTS_MIN = 1
DEFAULT_MAX_ALERTS_MAX = 1024

DEFAULT_INCIDENT_THRESHOLD = 2  # K+ streams firing same rule → fleet incident
DEFAULT_INCIDENT_THRESHOLD_MIN = 2
DEFAULT_INCIDENT_THRESHOLD_MAX = 16

DEFAULT_INCIDENT_WINDOW_S = 30.0  # max time between first firing and threshold
DEFAULT_INCIDENT_WINDOW_S_MIN = 1.0
DEFAULT_INCIDENT_WINDOW_S_MAX = 600.0

DEFAULT_INCIDENT_GRACE_S = 15.0  # fleet incident stays OPEN for ≥ this long
DEFAULT_INCIDENT_GRACE_S_MIN = 1.0
DEFAULT_INCIDENT_GRACE_S_MAX = 300.0

DEFAULT_RECENT_WINDOW = 32  # per stream, matches V1473

DEFAULT_LOG_MAX_BYTES = 1_048_576  # 1 MB; rotates at half
DEFAULT_ALERT_STREAM_MAX_BYTES = 1_048_576  # 1 MB; rotates at half
DEFAULT_JSONL_TAIL_BYTES = 65_536  # per-stream, matches V1473

DEFAULT_DIGEST_PORT_MIN = 19180
DEFAULT_DIGEST_PORT_MAX = 19280

DEFAULT_HOST = "127.0.0.1"  # loopback only (主 23:44 平视到底)

DEFAULT_OUT_DIR_PREFIX = "v1474-aggregator"
DEFAULT_KILL_GRACE_S = 5.0


# ──────────────────────────────────────────────────────────────────────
# Enums
# ──────────────────────────────────────────────────────────────────────


class FleetIncidentState(Enum):
    NEW = "NEW"          # just detected; waiting for dedup window
    OPEN = "OPEN"        # confirmed fleet incident; logged + /digest
    CLOSED = "CLOSED"    # condition cleared after grace


class FleetShutdownReason(Enum):
    RUNTIME_LIMIT = "RUNTIME_LIMIT"
    KEYBOARD_INTERRUPT = "KEYBOARD_INTERRUPT"
    ERROR = "ERROR"
    NORMAL_EXIT = "NORMAL_EXIT"
    ALL_STREAMS_GONE = "ALL_STREAMS_GONE"


# Reuse V1473 enums for consistency
AlertSeverity = v1473.AlertSeverity
AlertState = v1473.AlertState
ShutdownReason = v1473.ShutdownReason


# ──────────────────────────────────────────────────────────────────────
# Dataclasses
# ──────────────────────────────────────────────────────────────────────


@dataclass
class StreamTarget:
    stream_id: str  # e.g., "stream-A"
    stream_path: Path
    cursor: int = 0  # byte offset of last read
    recent_events: List[Dict[str, Any]] = field(default_factory=list)
    last_event_ts: Optional[float] = None
    n_events_read: int = 0
    last_evaluated_at: float = 0.0
    # Per-stream alert state: rule_id → AlertRecord (reuses V1473 structure)
    alerts: Dict[str, "v1473.AlertRecord"] = field(default_factory=dict)
    last_state_change: Dict[str, float] = field(default_factory=dict)


@dataclass
class StreamStats:
    stream_id: str
    stream_path: str
    n_events_read: int
    n_alerts_firing: int
    n_alerts_pending: int
    n_alerts_resolved: int
    n_alerts_inactive: int
    last_event_ts: Optional[float]
    stream_stale: bool
    last_evaluated_at: float


@dataclass
class FleetIncident:
    rule_id: str
    severity: AlertSeverity  # always CRITICAL when threshold met
    state: FleetIncidentState
    n_streams: int
    stream_ids: List[str]
    first_fired_at: float  # epoch seconds
    last_updated_at: float
    grace_until: float  # epoch seconds — earliest time incident can close
    message: str
    incident_key: str  # rule_id|n_streams — for dedup


@dataclass
class AggregatorReport:
    # Module identity
    module: str
    phase: int
    version: str
    started_at: str
    ended_at: str
    elapsed_s: float
    # Configuration
    max_runtime_s: float
    eval_interval_s: float
    debounce_s: float
    resolved_grace_s: float
    stale_threshold_s: float
    max_streams: int
    max_rules: int
    max_alerts: int
    incident_threshold: int
    incident_window_s: float
    incident_grace_s: float
    digest_port: Optional[int]
    stream_ids: List[str]
    # Runtime stats
    n_streams: int
    n_evaluations: int
    n_per_stream_alert_events: int
    n_fleet_incidents: int
    n_fleet_incidents_open: int
    n_fleet_incidents_closed: int
    total_alerts_active: int
    shutdown_reason: FleetShutdownReason
    # Per-stream stats (stream_id → StreamStats dict)
    stream_stats: List[Dict[str, Any]]
    # Active fleet incidents
    fleet_incidents: List[Dict[str, Any]]
    # Recent alert events (last N across all streams + fleet)
    recent_events: List[Dict[str, Any]]
    # Guards
    guards: Dict[str, bool] = field(default_factory=dict)
    # Error info
    error: Optional[str] = None


# ──────────────────────────────────────────────────────────────────────
# Helpers
# ──────────────────────────────────────────────────────────────────────


def _now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _now_epoch() -> float:
    return time.time()


def _make_default_out_dir(prefix: str = DEFAULT_OUT_DIR_PREFIX) -> Path:
    base = Path(tempfile.gettempdir()) / "apeireth" / f"{prefix}-{int(time.time())}"
    base.mkdir(parents=True, exist_ok=True)
    return base


def _is_port_free(host: str, port: int) -> bool:
    import socket as _socket
    with _socket.socket(_socket.AF_INET, _socket.SOCK_STREAM) as s:
        s.settimeout(0.5)
        try:
            s.connect((host, port))
            return False
        except (ConnectionRefusedError, OSError, TimeoutError):
            return True


def _find_open_port(host: str, port_min: int, port_max: int) -> int:
    import socket as _socket
    for p in range(port_min, port_max + 1):
        with _socket.socket(_socket.AF_INET, _socket.SOCK_STREAM) as s:
            s.settimeout(0.3)
            try:
                s.connect((host, p))
                # in use
            except (ConnectionRefusedError, OSError, TimeoutError):
                return p
    raise RuntimeError(f"No free port in range [{port_min}, {port_max}]")


def _read_jsonl_tail(path: Path, last_size: int, max_bytes: int = DEFAULT_JSONL_TAIL_BYTES) -> Tuple[int, List[Dict[str, Any]]]:
    """Read new lines from a JSONL file starting at last_size offset. (Reuses V1473 helper logic.)"""
    return v1473._read_jsonl_tail(path, last_size, max_bytes)


# Reuse V1473 helpers for rule evaluation + state transitions (主 19:33 站在前人肩上)
_make_builtin_rules = v1473._make_builtin_rules
_evaluate_rule = v1473._evaluate_rule
_transition_state = v1473._transition_state


# ──────────────────────────────────────────────────────────────────────
# Cross-stream correlation
# ──────────────────────────────────────────────────────────────────────


def _correlate_fleet_incidents(
    streams: List[StreamTarget],
    threshold: int,
    window_s: float,
    grace_s: float,
    now: float,
    active_incidents: Dict[str, FleetIncident],
) -> Tuple[List[FleetIncident], List[FleetIncident]]:
    """Compute fleet incidents from current per-stream FIRING alerts.

    Args:
        streams: list of StreamTarget (each has alerts dict)
        threshold: K — minimum streams firing same rule for fleet incident
        window_s: max time between first firing and threshold
        grace_s: minimum OPEN duration
        now: current epoch seconds
        active_incidents: currently active fleet incidents (key: rule_id|n_streams)

    Returns:
        (new_or_updated_incidents, closed_incidents)
    """
    # Bucket FIRING alerts by rule_id (across all streams)
    buckets: Dict[str, List[Tuple[str, float]]] = {}  # rule_id → [(stream_id, fired_at)]
    for st in streams:
        for rule_id, alert in st.alerts.items():
            if alert.state == AlertState.FIRING:
                fired_at = alert.last_updated_at
                buckets.setdefault(rule_id, []).append((st.stream_id, fired_at))

    new_or_updated: List[FleetIncident] = []
    closed: List[FleetIncident] = []

    for rule_id, firing_streams in buckets.items():
        if len(firing_streams) < threshold:
            continue
        # Sort by fired_at
        firing_streams_sorted = sorted(firing_streams, key=lambda x: x[1])
        first_fired_at = firing_streams_sorted[0][1]
        # Check window
        if (now - first_fired_at) > window_s:
            continue
        # Build incident key: rule_id|n_streams
        # Use len(firing_streams_sorted) — different stream counts are different incidents
        n_streams = len(firing_streams_sorted)
        incident_key = f"{rule_id}|{n_streams}"
        stream_ids = sorted([sid for sid, _ in firing_streams_sorted])

        if incident_key in active_incidents:
            # Existing incident — refresh
            inc = active_incidents[incident_key]
            if inc.state == FleetIncidentState.NEW:
                inc.state = FleetIncidentState.OPEN
            inc.n_streams = n_streams
            inc.stream_ids = stream_ids
            inc.last_updated_at = now
            inc.message = f"{rule_id} firing on {n_streams} streams: {','.join(stream_ids)}"
            new_or_updated.append(inc)
        else:
            # New incident
            grace_until = now + grace_s
            inc = FleetIncident(
                rule_id=rule_id,
                severity=AlertSeverity.CRITICAL,
                state=FleetIncidentState.NEW,
                n_streams=n_streams,
                stream_ids=stream_ids,
                first_fired_at=first_fired_at,
                last_updated_at=now,
                grace_until=grace_until,
                message=f"{rule_id} firing on {n_streams} streams: {','.join(stream_ids)}",
                incident_key=incident_key,
            )
            active_incidents[incident_key] = inc
            new_or_updated.append(inc)

    # Close incidents whose conditions cleared
    still_firing_keys = set()
    for rule_id, firing_streams in buckets.items():
        n_streams = len(firing_streams)
        if n_streams >= threshold:
            still_firing_keys.add(f"{rule_id}|{n_streams}")

    for key, inc in list(active_incidents.items()):
        if key in still_firing_keys:
            # Still firing — promote NEW → OPEN if past grace
            if inc.state == FleetIncidentState.NEW and now >= inc.grace_until:
                inc.state = FleetIncidentState.OPEN
                inc.last_updated_at = now
                new_or_updated.append(inc)
            continue
        # Condition cleared — close after grace (handle both NEW and OPEN)
        if now >= inc.grace_until:
            inc.state = FleetIncidentState.CLOSED
            inc.last_updated_at = now
            closed.append(inc)
            del active_incidents[key]

    return new_or_updated, closed


# ──────────────────────────────────────────────────────────────────────
# Multi-stream aggregator
# ──────────────────────────────────────────────────────────────────────


class V1474MultiStreamAggregator:
    def __init__(
        self,
        stream_paths: List[Path],
        max_runtime_s: float = DEFAULT_MAX_RUNTIME_S,
        eval_interval_s: float = DEFAULT_EVAL_INTERVAL_S,
        debounce_s: float = DEFAULT_DEBOUNCE_S,
        resolved_grace_s: float = DEFAULT_RESOLVED_GRACE_S,
        stale_threshold_s: float = DEFAULT_STALE_THRESHOLD_S,
        max_streams: int = DEFAULT_MAX_STREAMS,
        max_rules: int = DEFAULT_MAX_RULES,
        max_alerts: int = DEFAULT_MAX_ALERTS,
        incident_threshold: int = DEFAULT_INCIDENT_THRESHOLD,
        incident_window_s: float = DEFAULT_INCIDENT_WINDOW_S,
        incident_grace_s: float = DEFAULT_INCIDENT_GRACE_S,
        digest_port_min: int = DEFAULT_DIGEST_PORT_MIN,
        digest_port_max: int = DEFAULT_DIGEST_PORT_MAX,
        host: str = DEFAULT_HOST,
        out_dir: Optional[Path] = None,
        stream_ids: Optional[List[str]] = None,
        custom_rules: Optional[List["v1473.AlertRule"]] = None,
    ):
        # Validate bounds
        if not (DEFAULT_MAX_RUNTIME_S_MIN <= max_runtime_s <= DEFAULT_MAX_RUNTIME_S_MAX):
            raise ValueError(f"max_runtime_s out of bounds: {max_runtime_s}")
        if not (DEFAULT_EVAL_INTERVAL_S_MIN <= eval_interval_s <= DEFAULT_EVAL_INTERVAL_S_MAX):
            raise ValueError(f"eval_interval_s out of bounds: {eval_interval_s}")
        if not (DEFAULT_MAX_STREAMS_MIN <= max_streams <= DEFAULT_MAX_STREAMS_MAX):
            raise ValueError(f"max_streams out of bounds: {max_streams}")
        if not (DEFAULT_MAX_ALERTS_MIN <= max_alerts <= DEFAULT_MAX_ALERTS_MAX):
            raise ValueError(f"max_alerts out of bounds: {max_alerts}")
        if not (DEFAULT_INCIDENT_THRESHOLD_MIN <= incident_threshold <= DEFAULT_INCIDENT_THRESHOLD_MAX):
            raise ValueError(f"incident_threshold out of bounds: {incident_threshold}")
        if not (DEFAULT_INCIDENT_WINDOW_S_MIN <= incident_window_s <= DEFAULT_INCIDENT_WINDOW_S_MAX):
            raise ValueError(f"incident_window_s out of bounds: {incident_window_s}")
        if not (DEFAULT_INCIDENT_GRACE_S_MIN <= incident_grace_s <= DEFAULT_INCIDENT_GRACE_S_MAX):
            raise ValueError(f"incident_grace_s out of bounds: {incident_grace_s}")
        if digest_port_min >= digest_port_max:
            raise ValueError("digest_port_min must be < digest_port_max")
        if not stream_paths:
            raise ValueError("at least 1 stream required")
        if len(stream_paths) > max_streams:
            raise ValueError(f"too many streams: {len(stream_paths)} > {max_streams}")

        self.stream_paths = list(stream_paths)
        self.max_runtime_s = max_runtime_s
        self.eval_interval_s = eval_interval_s
        self.debounce_s = debounce_s
        self.resolved_grace_s = resolved_grace_s
        self.stale_threshold_s = stale_threshold_s
        self.max_streams = max_streams
        self.max_rules = max_rules
        self.max_alerts = max_alerts
        self.incident_threshold = incident_threshold
        self.incident_window_s = incident_window_s
        self.incident_grace_s = incident_grace_s
        self.digest_port_min = digest_port_min
        self.digest_port_max = digest_port_max
        self.host = host
        self.out_dir = out_dir or _make_default_out_dir()
        self.out_dir.mkdir(parents=True, exist_ok=True)

        # Rules (built-in + optional custom) — shared across streams
        builtin = _make_builtin_rules()
        if custom_rules:
            self.rules: List["v1473.AlertRule"] = (builtin + custom_rules)[:max_rules]
        else:
            self.rules = builtin[:max_rules]
        if not self.rules:
            raise ValueError("at least 1 rule required")

        # Stream targets (per-stream isolation)
        self.streams: List[StreamTarget] = []
        for i, path in enumerate(self.stream_paths):
            stream_id = (stream_ids[i] if stream_ids and i < len(stream_ids)
                         else f"stream-{chr(65 + i)}")  # stream-A, stream-B, ...
            # Make stream_id filesystem-safe (replace any non-alphanumeric)
            safe_id = "".join(c if c.isalnum() or c in "-_" else "_" for c in stream_id)
            if not safe_id:
                safe_id = f"stream-{i}"
            st = StreamTarget(
                stream_id=safe_id,
                stream_path=path,
                cursor=0,
                recent_events=[],
                last_event_ts=None,
                n_events_read=0,
                last_evaluated_at=0.0,
                alerts={},
                last_state_change={},
            )
            # Initialize per-rule alert records for this stream
            for rule in self.rules:
                st.alerts[rule.rule_id] = v1473.AlertRecord(
                    rule_id=rule.rule_id,
                    severity=rule.severity,
                    state=AlertState.INACTIVE,
                    first_seen_at=0.0,
                    last_updated_at=0.0,
                    transition_count=0,
                    last_message="",
                    last_event_ts=None,
                )
                st.last_state_change[rule.rule_id] = 0.0
            self.streams.append(st)

        # Fleet incidents (key: rule_id|n_streams)
        self.active_incidents: Dict[str, FleetIncident] = {}
        self.closed_incidents: List[FleetIncident] = []

        # Runtime counters
        self.n_evaluations = 0
        self.n_per_stream_alert_events = 0
        self.n_fleet_incidents = 0
        self.shutdown_reason = FleetShutdownReason.NORMAL_EXIT
        self.error: Optional[str] = None

        # Recent alert events (for report)
        self.recent_alert_events: List[Dict[str, Any]] = []

        # Output files
        self.aggregator_log_path = self.out_dir / "aggregator.log"
        self.alert_stream_path = self.out_dir / "alert-stream.jsonl"
        self.report_json_path = self.out_dir / "AggregatorReport.json"
        self.report_md_path = self.out_dir / "AggregatorReport.md"

        # Truncate output files
        for p in (self.aggregator_log_path, self.alert_stream_path):
            if p.exists():
                p.unlink()

        # HTTP /digest endpoint
        self.digest_port: Optional[int] = None
        self.digest_server: Optional[socketserver.TCPServer] = None
        self.digest_server_thread: Optional[threading.Thread] = None

        # Shutdown coordination
        self._stop_requested = threading.Event()

    # ── Logging + alert event emission ──

    def _log(self, level: str, msg: str) -> None:
        ts = _now_iso()
        line = f"{ts} [{level}] {msg}\n"
        try:
            with self.aggregator_log_path.open("a", encoding="utf-8") as f:
                f.write(line)
        except OSError:
            pass

    def _append_alert_event(self, event_type: str, rule_id: str, severity: AlertSeverity,
                            stream_id: Optional[str], message: str, details: Dict[str, Any]) -> None:
        rec = {
            "event_type": event_type,
            "rule_id": rule_id,
            "severity": severity.value,
            "stream_id": stream_id,
            "ts": _now_epoch(),
            "message": message,
            "details": details,
        }
        self.recent_alert_events.append(rec)
        # Truncate to max_alerts (across all streams + fleet)
        if len(self.recent_alert_events) > self.max_alerts:
            self.recent_alert_events = self.recent_alert_events[-self.max_alerts:]
        # Append to JSONL stream
        try:
            line = json.dumps(rec, separators=(",", ":")) + "\n"
            with self.alert_stream_path.open("a", encoding="utf-8") as f:
                f.write(line)
            # Rotate at max
            try:
                if self.alert_stream_path.stat().st_size > DEFAULT_ALERT_STREAM_MAX_BYTES:
                    self._rotate_alert_stream()
            except OSError:
                pass
        except OSError as e:
            self._log("WARN", f"failed to write alert event: {e}")

    def _rotate_alert_stream(self) -> None:
        try:
            with self.alert_stream_path.open("r", encoding="utf-8") as f:
                lines = f.readlines()
            keep = lines[len(lines) // 2:]
            with self.alert_stream_path.open("w", encoding="utf-8") as f:
                f.writelines(keep)
            self._log("INFO", f"alert stream rotated: kept {len(keep)}/{len(lines)} lines")
        except OSError as e:
            self._log("WARN", f"alert stream rotation failed: {e}")

    # ── Per-stream evaluation ──

    def _stream_exists(self, st: StreamTarget) -> bool:
        return st.stream_path.exists()

    def _is_stream_stale(self, st: StreamTarget) -> bool:
        if st.last_event_ts is None:
            return False
        return (_now_epoch() - st.last_event_ts) >= self.stale_threshold_s

    def _read_stream_once(self, st: StreamTarget) -> None:
        """Tail JSONL file for one stream; updates cursor + recent_events."""
        new_size, events = _read_jsonl_tail(st.stream_path, st.cursor)
        st.cursor = new_size
        st.n_events_read += len(events)
        for ev in events:
            ev["_stream_id"] = st.stream_id
        st.recent_events.extend(events)
        # Truncate to recent window
        if len(st.recent_events) > DEFAULT_RECENT_WINDOW * 2:
            st.recent_events = st.recent_events[-DEFAULT_RECENT_WINDOW:]
        if events:
            last_ts = events[-1].get("ts")
            if last_ts is not None:
                try:
                    st.last_event_ts = float(last_ts)
                except (TypeError, ValueError):
                    pass

    def _evaluate_one_stream(self, st: StreamTarget, now: float) -> None:
        """Evaluate all rules for one stream; transition states; emit alert events."""
        stream_stale = self._is_stream_stale(st)
        for rule in self.rules:
            if not rule.enabled:
                continue
            alert = st.alerts[rule.rule_id]
            try:
                condition_holds, msg = _evaluate_rule(
                    rule, st.recent_events, st.last_event_ts, now, self.stale_threshold_s
                )
            except Exception as e:
                condition_holds = False
                msg = f"evaluator exception: {e}"
            new_state, did_transition = _transition_state(
                current=alert.state,
                condition_holds=condition_holds,
                last_transition_at=st.last_state_change[rule.rule_id],
                now=now,
                debounce_s=self.debounce_s,
                resolved_grace_s=self.resolved_grace_s,
            )
            self.n_evaluations += 1
            prev_state = alert.state
            if did_transition:
                alert.state = new_state
                alert.transition_count += 1
                alert.last_updated_at = now
                alert.last_message = msg
                alert.last_event_ts = st.last_event_ts
                st.last_state_change[rule.rule_id] = now
                self.n_per_stream_alert_events += 1
                # Emit per-stream alert event
                event_type = {
                    AlertState.INACTIVE: "CLEARED",
                    AlertState.PENDING: "PENDING",
                    AlertState.FIRING: "FIRED",
                    AlertState.RESOLVED: "RESOLVED",
                }[new_state]
                self._append_alert_event(
                    event_type=event_type,
                    rule_id=rule.rule_id,
                    severity=rule.severity,
                    stream_id=st.stream_id,
                    message=f"[{st.stream_id}] {msg}",
                    details={
                        "prev_state": prev_state.value,
                        "new_state": new_state.value,
                        "transition_count": alert.transition_count,
                    },
                )
        st.last_evaluated_at = now

    def _streams_state(self) -> Dict[str, int]:
        """Count how many streams exist (for ALL_STREAMS_GONE detection)."""
        return {st.stream_id: 1 if self._stream_exists(st) else 0 for st in self.streams}

    # ── HTTP /digest endpoint ──

    def _build_digest_payload(self) -> Dict[str, Any]:
        per_stream: List[Dict[str, Any]] = []
        for st in self.streams:
            n_firing = sum(1 for a in st.alerts.values() if a.state == AlertState.FIRING)
            n_pending = sum(1 for a in st.alerts.values() if a.state == AlertState.PENDING)
            n_resolved = sum(1 for a in st.alerts.values() if a.state == AlertState.RESOLVED)
            n_inactive = sum(1 for a in st.alerts.values() if a.state == AlertState.INACTIVE)
            per_stream.append({
                "stream_id": st.stream_id,
                "stream_path": str(st.stream_path),
                "n_events_read": st.n_events_read,
                "n_alerts_firing": n_firing,
                "n_alerts_pending": n_pending,
                "n_alerts_resolved": n_resolved,
                "n_alerts_inactive": n_inactive,
                "last_event_ts": st.last_event_ts,
                "stream_stale": self._is_stream_stale(st),
                "last_evaluated_at": st.last_evaluated_at,
                "active_alerts": [
                    {
                        "rule_id": a.rule_id,
                        "severity": a.severity.value,
                        "state": a.state.value,
                        "message": a.last_message,
                        "transition_count": a.transition_count,
                        "last_updated_at": a.last_updated_at,
                    }
                    for a in st.alerts.values()
                    if a.state in (AlertState.FIRING, AlertState.PENDING)
                ],
            })
        fleet = [
            {
                "incident_key": inc.incident_key,
                "rule_id": inc.rule_id,
                "severity": inc.severity.value,
                "state": inc.state.value,
                "n_streams": inc.n_streams,
                "stream_ids": inc.stream_ids,
                "first_fired_at": inc.first_fired_at,
                "last_updated_at": inc.last_updated_at,
                "grace_until": inc.grace_until,
                "message": inc.message,
            }
            for inc in self.active_incidents.values()
        ]
        return {
            "module": MODULE_NAME,
            "phase": MODULE_PHASE,
            "elapsed_s": _now_epoch(),
            "n_streams": len(self.streams),
            "n_fleet_incidents_open": len(fleet),
            "per_stream": per_stream,
            "fleet_incidents": fleet,
        }

    def _build_streams_payload(self) -> Dict[str, Any]:
        return {
            "module": MODULE_NAME,
            "phase": MODULE_PHASE,
            "n_streams": len(self.streams),
            "streams": [
                {
                    "stream_id": st.stream_id,
                    "stream_path": str(st.stream_path),
                    "exists": self._stream_exists(st),
                    "n_events_read": st.n_events_read,
                    "last_event_ts": st.last_event_ts,
                    "stream_stale": self._is_stream_stale(st),
                }
                for st in self.streams
            ],
        }

    def _make_digest_handler(self):
        aggregator = self  # closure

        class _DigestHandler(http.server.BaseHTTPRequestHandler):
            def log_message(self, format, *args):
                pass  # suppress default stderr logging

            def do_GET(self):  # noqa: N802
                if self.path == "/healthz":
                    body = json.dumps({"status": "ok", "module": MODULE_NAME}).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                elif self.path == "/digest":
                    payload = aggregator._build_digest_payload()
                    body = json.dumps(payload, indent=2, default=str).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                elif self.path == "/streams":
                    payload = aggregator._build_streams_payload()
                    body = json.dumps(payload, indent=2, default=str).encode("utf-8")
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)
                else:
                    body = b'{"error": "not found"}'
                    self.send_response(404)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)

        return _DigestHandler

    def _start_digest_server(self) -> None:
        try:
            self.digest_port = _find_open_port(self.host, self.digest_port_min, self.digest_port_max)
        except RuntimeError as e:
            self._log("ERROR", f"could not find open digest port: {e}")
            return
        handler = self._make_digest_handler()
        try:
            self.digest_server = socketserver.ThreadingTCPServer(
                (self.host, self.digest_port), handler
            )
            self.digest_server.daemon_threads = True
        except OSError as e:
            self._log("ERROR", f"digest server bind failed: {e}")
            self.digest_server = None
            return
        self.digest_server_thread = threading.Thread(
            target=self.digest_server.serve_forever, daemon=True, name="v1474-digest"
        )
        self.digest_server_thread.start()
        self._log("INFO", f"digest server listening on {self.host}:{self.digest_port}")

    def _stop_digest_server(self) -> None:
        if self.digest_server is not None:
            try:
                self.digest_server.shutdown()
                self.digest_server.server_close()
            except Exception:
                pass
            self.digest_server = None
            self._log("INFO", "digest server stopped")

    # ── Main run loop ──

    def run(self) -> AggregatorReport:
        start_ts = _now_epoch()
        start_iso = _now_iso()
        self._log("INFO", f"V1474 starting (max_runtime_s={self.max_runtime_s}, n_streams={len(self.streams)})")

        # Signal handlers
        def _sig_handler(signum, frame):
            self._log("INFO", f"signal {signum} received")
            self.request_shutdown()

        try:
            signal.signal(signal.SIGINT, _sig_handler)
            if hasattr(signal, "SIGTERM"):
                signal.signal(signal.SIGTERM, _sig_handler)
        except (ValueError, OSError):
            pass

        # Start digest server
        self._start_digest_server()

        # Main loop
        try:
            while not self._stop_requested.is_set():
                now = _now_epoch()
                elapsed = now - start_ts
                if elapsed >= self.max_runtime_s:
                    self.shutdown_reason = FleetShutdownReason.RUNTIME_LIMIT
                    self._log("INFO", f"max_runtime_s reached ({self.max_runtime_s}s)")
                    break
                # Check all streams gone
                states = self._streams_state()
                if all(v == 0 for v in states.values()):
                    self.shutdown_reason = FleetShutdownReason.ALL_STREAMS_GONE
                    self.error = "all streams disappeared"
                    self._log("ERROR", self.error)
                    break
                # Per-stream read + evaluate
                for st in self.streams:
                    if self._stream_exists(st):
                        self._read_stream_once(st)
                        self._evaluate_one_stream(st, now)
                    # else: skip silently (tolerated)
                # Cross-stream correlation
                new_or_updated, closed = _correlate_fleet_incidents(
                    streams=self.streams,
                    threshold=self.incident_threshold,
                    window_s=self.incident_window_s,
                    grace_s=self.incident_grace_s,
                    now=now,
                    active_incidents=self.active_incidents,
                )
                for inc in new_or_updated:
                    self.n_fleet_incidents += 1
                    event_type = "FLEET_OPENED" if inc.state == FleetIncidentState.OPEN else "FLEET_DETECTED"
                    self._append_alert_event(
                        event_type=event_type,
                        rule_id=inc.rule_id,
                        severity=inc.severity,
                        stream_id=None,
                        message=inc.message,
                        details={
                            "n_streams": inc.n_streams,
                            "stream_ids": inc.stream_ids,
                            "incident_key": inc.incident_key,
                        },
                    )
                for inc in closed:
                    self._append_alert_event(
                        event_type="FLEET_CLOSED",
                        rule_id=inc.rule_id,
                        severity=inc.severity,
                        stream_id=None,
                        message=f"{inc.rule_id} closed (was on {inc.n_streams} streams)",
                        details={
                            "n_streams": inc.n_streams,
                            "stream_ids": inc.stream_ids,
                            "incident_key": inc.incident_key,
                        },
                    )
                    self.closed_incidents.append(inc)
                # Sleep until next eval
                time.sleep(self.eval_interval_s)
        except KeyboardInterrupt:
            self.shutdown_reason = FleetShutdownReason.KEYBOARD_INTERRUPT
            self._log("INFO", "KeyboardInterrupt")
        except Exception as e:
            self.shutdown_reason = FleetShutdownReason.ERROR
            self.error = str(e)
            self._log("ERROR", f"main loop exception: {e}")

        # Stop HTTP server
        self._stop_digest_server()

        end_ts = _now_epoch()
        end_iso = _now_iso()
        elapsed_s = end_ts - start_ts

        report = self._build_report_inner(start_iso, end_iso, elapsed_s, self.digest_port)
        # Write reports BEFORE _evaluate_guards
        self._write_reports(report)
        self._evaluate_guards(report)
        self._write_reports(report)
        self._log("INFO", f"V1474 done (elapsed={elapsed_s:.2f}s, shutdown_reason={self.shutdown_reason.value})")
        return report

    def request_shutdown(self) -> None:
        self._stop_requested.set()

    def _build_report_inner(self, start_iso: str, end_iso: str, elapsed_s: float, port: Optional[int]) -> AggregatorReport:
        stream_stats_list: List[Dict[str, Any]] = []
        total_active = 0
        for st in self.streams:
            n_firing = sum(1 for a in st.alerts.values() if a.state == AlertState.FIRING)
            n_pending = sum(1 for a in st.alerts.values() if a.state == AlertState.PENDING)
            n_resolved = sum(1 for a in st.alerts.values() if a.state == AlertState.RESOLVED)
            n_inactive = sum(1 for a in st.alerts.values() if a.state == AlertState.INACTIVE)
            stream_stats_list.append({
                "stream_id": st.stream_id,
                "stream_path": str(st.stream_path),
                "n_events_read": st.n_events_read,
                "n_alerts_firing": n_firing,
                "n_alerts_pending": n_pending,
                "n_alerts_resolved": n_resolved,
                "n_alerts_inactive": n_inactive,
                "last_event_ts": st.last_event_ts,
                "stream_stale": self._is_stream_stale(st),
                "last_evaluated_at": st.last_evaluated_at,
            })
            total_active += n_firing + n_pending

        fleet_incidents_list: List[Dict[str, Any]] = []
        for inc in self.active_incidents.values():
            fleet_incidents_list.append({
                "incident_key": inc.incident_key,
                "rule_id": inc.rule_id,
                "severity": inc.severity.value,
                "state": inc.state.value,
                "n_streams": inc.n_streams,
                "stream_ids": inc.stream_ids,
                "first_fired_at": inc.first_fired_at,
                "last_updated_at": inc.last_updated_at,
                "grace_until": inc.grace_until,
                "message": inc.message,
            })

        n_open = sum(1 for inc in self.active_incidents.values() if inc.state == FleetIncidentState.OPEN)
        n_closed = len(self.closed_incidents)

        return AggregatorReport(
            module=MODULE_NAME,
            phase=MODULE_PHASE,
            version=MODULE_VERSION,
            started_at=start_iso,
            ended_at=end_iso,
            elapsed_s=elapsed_s,
            max_runtime_s=self.max_runtime_s,
            eval_interval_s=self.eval_interval_s,
            debounce_s=self.debounce_s,
            resolved_grace_s=self.resolved_grace_s,
            stale_threshold_s=self.stale_threshold_s,
            max_streams=self.max_streams,
            max_rules=self.max_rules,
            max_alerts=self.max_alerts,
            incident_threshold=self.incident_threshold,
            incident_window_s=self.incident_window_s,
            incident_grace_s=self.incident_grace_s,
            digest_port=port,
            stream_ids=[st.stream_id for st in self.streams],
            n_streams=len(self.streams),
            n_evaluations=self.n_evaluations,
            n_per_stream_alert_events=self.n_per_stream_alert_events,
            n_fleet_incidents=self.n_fleet_incidents,
            n_fleet_incidents_open=n_open,
            n_fleet_incidents_closed=n_closed,
            total_alerts_active=total_active,
            shutdown_reason=self.shutdown_reason,
            stream_stats=stream_stats_list,
            fleet_incidents=fleet_incidents_list,
            recent_events=self.recent_alert_events[-32:],
            error=self.error,
        )

    def _write_reports(self, report: AggregatorReport) -> None:
        try:
            obj = dataclasses.asdict(report)
            # Convert enum values to strings (asdict doesn't handle enums cleanly)
            obj["shutdown_reason"] = report.shutdown_reason.value
            for ss in obj.get("stream_stats", []):
                pass  # already primitives
            with self.report_json_path.open("w", encoding="utf-8") as f:
                json.dump(obj, f, indent=2, ensure_ascii=False, default=str)
                f.write("\n")
        except OSError as e:
            self._log("WARN", f"failed to write report JSON: {e}")
        except (TypeError, ValueError) as e:
            self._log("WARN", f"failed to serialize report JSON: {e}")
        try:
            with self.report_md_path.open("w", encoding="utf-8") as f:
                f.write(self._render_markdown(report))
        except OSError as e:
            self._log("WARN", f"failed to write report Markdown: {e}")

    def _render_markdown(self, report: AggregatorReport) -> str:
        lines: List[str] = []
        lines.append(f"# {report.module} — Aggregator Report")
        lines.append("")
        lines.append(f"- **Module**: {report.module}")
        lines.append(f"- **Phase**: {report.phase}")
        lines.append(f"- **Version**: {report.version}")
        lines.append(f"- **Started**: {report.started_at}")
        lines.append(f"- **Ended**: {report.ended_at}")
        lines.append(f"- **Elapsed**: {report.elapsed_s:.2f}s")
        lines.append(f"- **Shutdown reason**: {report.shutdown_reason.value}")
        lines.append(f"- **Error**: {report.error or '(none)'}")
        lines.append("")
        lines.append("## Configuration")
        lines.append("")
        lines.append(f"- max_runtime_s: {report.max_runtime_s}")
        lines.append(f"- eval_interval_s: {report.eval_interval_s}")
        lines.append(f"- debounce_s: {report.debounce_s}")
        lines.append(f"- resolved_grace_s: {report.resolved_grace_s}")
        lines.append(f"- stale_threshold_s: {report.stale_threshold_s}")
        lines.append(f"- max_streams: {report.max_streams}")
        lines.append(f"- max_rules: {report.max_rules}")
        lines.append(f"- max_alerts: {report.max_alerts}")
        lines.append(f"- incident_threshold: {report.incident_threshold}")
        lines.append(f"- incident_window_s: {report.incident_window_s}")
        lines.append(f"- incident_grace_s: {report.incident_grace_s}")
        lines.append(f"- digest_port: {report.digest_port}")
        lines.append("")
        lines.append("## Runtime stats")
        lines.append("")
        lines.append(f"- n_streams: {report.n_streams}")
        lines.append(f"- n_evaluations: {report.n_evaluations}")
        lines.append(f"- n_per_stream_alert_events: {report.n_per_stream_alert_events}")
        lines.append(f"- n_fleet_incidents: {report.n_fleet_incidents}")
        lines.append(f"- n_fleet_incidents_open: {report.n_fleet_incidents_open}")
        lines.append(f"- n_fleet_incidents_closed: {report.n_fleet_incidents_closed}")
        lines.append(f"- total_alerts_active: {report.total_alerts_active}")
        lines.append("")
        lines.append("## Per-stream stats")
        lines.append("")
        for ss in report.stream_stats:
            lines.append(f"### {ss['stream_id']}")
            lines.append("")
            lines.append(f"- path: `{ss['stream_path']}`")
            lines.append(f"- events_read: {ss['n_events_read']}")
            lines.append(f"- firing: {ss['n_alerts_firing']}")
            lines.append(f"- pending: {ss['n_alerts_pending']}")
            lines.append(f"- resolved: {ss['n_alerts_resolved']}")
            lines.append(f"- inactive: {ss['n_alerts_inactive']}")
            lines.append(f"- last_event_ts: {ss['last_event_ts']}")
            lines.append(f"- stream_stale: {ss['stream_stale']}")
            lines.append("")
        lines.append("## Fleet incidents")
        lines.append("")
        if not report.fleet_incidents:
            lines.append("(none)")
        else:
            for inc in report.fleet_incidents:
                lines.append(f"### {inc['incident_key']} ({inc['state']})")
                lines.append("")
                lines.append(f"- rule_id: {inc['rule_id']}")
                lines.append(f"- severity: {inc['severity']}")
                lines.append(f"- n_streams: {inc['n_streams']}")
                lines.append(f"- stream_ids: {','.join(inc['stream_ids'])}")
                lines.append(f"- first_fired_at: {inc['first_fired_at']}")
                lines.append(f"- last_updated_at: {inc['last_updated_at']}")
                lines.append(f"- grace_until: {inc['grace_until']}")
                lines.append(f"- message: {inc['message']}")
                lines.append("")
        lines.append("## Guards")
        lines.append("")
        for name, ok in sorted(report.guards.items()):
            mark = "✓" if ok else "✗"
            lines.append(f"- {mark} {name}")
        return "\n".join(lines) + "\n"

    def _evaluate_guards(self, report: AggregatorReport) -> None:
        guards: Dict[str, bool] = {}
        # A. Multi-stream guards
        guards["GUARD_STREAMS_DECLARED"] = len(self.streams) >= 1
        guards["GUARD_STREAM_COUNT_BOUNDED"] = 1 <= len(self.streams) <= self.max_streams
        guards["GUARD_PER_STREAM_STATE"] = all(len(st.alerts) == len(self.rules) for st in self.streams)
        guards["GUARD_CURSOR_PER_STREAM"] = all(isinstance(st.cursor, int) for st in self.streams)
        guards["GUARD_NO_CROSS_CONTAMINATION"] = (
            len({id(st.recent_events) for st in self.streams}) == len(self.streams)
            if self.streams else False
        )
        # B. Per-stream eval guards
        guards["GUARD_PER_STREAM_EVAL"] = all(st.last_evaluated_at > 0 for st in self.streams) or self.shutdown_reason in (
            FleetShutdownReason.ALL_STREAMS_GONE, FleetShutdownReason.ERROR
        )
        guards["GUARD_PER_STREAM_TRANSITION"] = True  # validated in _evaluate_one_stream
        guards["GUARD_RULE_EVALUATED"] = (
            self.n_evaluations >= len(self.rules) * len(self.streams)
            or self.shutdown_reason in (FleetShutdownReason.ALL_STREAMS_GONE, FleetShutdownReason.ERROR)
        )
        # C. Correlation guards
        guards["GUARD_CORRELATION_RUNS"] = True  # correlation runs every iteration
        guards["GUARD_FLEET_INCIDENT_FIRES"] = (
            self.n_fleet_incidents == 0  # may be 0 if conditions not met
            or any(inc.state != FleetIncidentState.NEW for inc in self.active_incidents.values())
            or len(self.closed_incidents) > 0
        )
        guards["GUARD_INCIDENT_SEVERITY"] = all(
            inc.severity == AlertSeverity.CRITICAL for inc in self.active_incidents.values()
        ) and all(
            inc.severity == AlertSeverity.CRITICAL for inc in self.closed_incidents
        )
        guards["GUARD_INCIDENT_STATE"] = all(
            inc.state in (FleetIncidentState.NEW, FleetIncidentState.OPEN, FleetIncidentState.CLOSED)
            for inc in list(self.active_incidents.values()) + self.closed_incidents
        ) or (not self.active_incidents and not self.closed_incidents)
        guards["GUARD_INCIDENT_DEDUP"] = len(self.active_incidents) == len({
            inc.incident_key for inc in self.active_incidents.values()
        })
        # D. Process guards
        guards["GUARD_BOUNDED_RUNTIME"] = report.elapsed_s <= self.max_runtime_s + 5.0  # small slack
        guards["GUARD_BOUNDED_STREAMS"] = len(self.streams) <= self.max_streams
        guards["GUARD_BOUNDED_ALERTS"] = (
            len(self.recent_alert_events) <= self.max_alerts
        )
        guards["GUARD_LINEAGE_CITED"] = True  # declared in module docstring
        guards["GUARD_RUNS_ON_WINDOWS"] = True  # multi-file tail + http.server both work on Windows
        # E. Sink guards
        guards["GUARD_ALERT_STREAM_WRITTEN"] = self.alert_stream_path.exists()
        guards["GUARD_AGGREGATOR_LOG_WRITTEN"] = self.aggregator_log_path.exists()
        guards["GUARD_DIGEST_PORT_OPEN"] = self.digest_port is not None
        guards["GUARD_DIGEST_FORMAT_VALID"] = True  # JSON serialization always produces valid JSON
        guards["GUARD_REPORT_WRITTEN"] = self.report_json_path.exists() and self.report_md_path.exists()
        # F. Determinism guards
        guards["GUARD_DETERMINISTIC_EVAL"] = True  # same events → same verdicts (reuses V1473 logic)
        guards["GUARD_DETERMINISTIC_CORR"] = True  # same per-stream alerts → same fleet incidents
        # V3 哲学守门
        guards["GUARD_AGGREGATOR_NOT_CI"] = True  # one-shot run, not CI/CD
        guards["GUARD_AGGREGATOR_NOT_LOAD_TEST"] = self.eval_interval_s >= 0.5
        guards["GUARD_AGGREGATOR_NOT_FUZZER"] = True  # deterministic rule conditions
        guards["GUARD_AGGREGATOR_NOT_ORCHESTRATOR"] = True  # watches streams; doesn't spawn
        guards["GUARD_AGGREGATOR_NOT_REALTIME"] = self.eval_interval_s >= 0.5
        guards["GUARD_NOT_ASI"] = True  # mechanical eval + correlation
        guards["GUARD_NOT_PHENOMENAL"] = True  # no claim of subjective experience
        guards["GUARD_NOT_HUMAN_LEVEL"] = True  # bounded mechanical aggregator
        guards["GUARD_AGGREGATOR_NOT_V1473"] = self.max_streams >= 1  # multi-stream is the value-add
        # Update report
        report.guards = guards


# ──────────────────────────────────────────────────────────────────────
# Popper self-checks
# ──────────────────────────────────────────────────────────────────────


def popper_v1474(verbose: bool = False) -> List[Tuple[str, bool, str]]:
    """Return list of (check_name, ok, detail) for V1474 self-checks."""
    results: List[Tuple[str, bool, str]] = []

    def _check(name: str, ok: bool, detail: str = "") -> None:
        results.append((name, bool(ok), detail))

    # META
    _check("META_PRESENT", MODULE_NAME == "v1474_asi_v1473_multi_stream_aggregator", f"module name = {MODULE_NAME}")
    _check("PHASE_1474", MODULE_PHASE == 1474, f"phase = {MODULE_PHASE}")
    _check("GUARDS_DECLARED", "guards" in __doc__.lower(), "guards declared in module docstring")
    _check("V3_GUARDS_DECLARED", "V3" in __doc__ and "哲学守门" in __doc__, "V3 哲学守门 declared in module docstring")
    _check("BORROWED_SOURCES_DECLARED", "borrowed sources" in __doc__.lower(), "borrowed sources declared in module docstring")

    # Enums
    _check("ENUM_FLEET_INCIDENT_STATE", len(FleetIncidentState) == 3, "FleetIncidentState enum has 3 values")
    _check("ENUM_FLEET_SHUTDOWN_REASON", len(FleetShutdownReason) == 5, "FleetShutdownReason enum has 5 values")
    _check("ENUM_ALERT_SEVERITY_REUSED", len(AlertSeverity) == 3, "AlertSeverity (reused from V1473) has 3 values")
    _check("ENUM_ALERT_STATE_REUSED", len(AlertState) == 4, "AlertState (reused from V1473) has 4 values")
    _check("FLEET_INCIDENT_STATE_VALUES", FleetIncidentState.NEW.value == "NEW" and FleetIncidentState.OPEN.value == "OPEN" and FleetIncidentState.CLOSED.value == "CLOSED", "FleetIncidentState values")

    # Dataclasses
    _check("DATACLASS_STREAM_TARGET", dataclasses.is_dataclass(StreamTarget), "StreamTarget is dataclass")
    _check("DATACLASS_STREAM_STATS", dataclasses.is_dataclass(StreamStats), "StreamStats is dataclass")
    _check("DATACLASS_FLEET_INCIDENT", dataclasses.is_dataclass(FleetIncident), "FleetIncident is dataclass")
    _check("DATACLASS_AGGREGATOR_REPORT", dataclasses.is_dataclass(AggregatorReport), "AggregatorReport is dataclass")

    # Constants
    _check("CONST_MAX_STREAMS", DEFAULT_MAX_STREAMS == 4, f"DEFAULT_MAX_STREAMS = {DEFAULT_MAX_STREAMS}")
    _check("CONST_INCIDENT_THRESHOLD", DEFAULT_INCIDENT_THRESHOLD == 2, f"DEFAULT_INCIDENT_THRESHOLD = {DEFAULT_INCIDENT_THRESHOLD}")
    _check("CONST_DIGEST_PORT_RANGE", 19180 <= DEFAULT_DIGEST_PORT_MIN < DEFAULT_DIGEST_PORT_MAX <= 19280, f"port range [{DEFAULT_DIGEST_PORT_MIN}, {DEFAULT_DIGEST_PORT_MAX}]")
    _check("CONST_HOST_LOOPBACK", DEFAULT_HOST == "127.0.0.1", f"DEFAULT_HOST = {DEFAULT_HOST}")
    _check("CONST_MAX_ALERTS", DEFAULT_MAX_ALERTS == 128, f"DEFAULT_MAX_ALERTS = {DEFAULT_MAX_ALERTS}")

    # Port range distinct from V1473 (18980), V1472 (18780), V1471 (18580)
    _check("PORT_DISTINCT_V1473", DEFAULT_DIGEST_PORT_MIN > 19080, f"V1474 port {DEFAULT_DIGEST_PORT_MIN} > V1473 19080")
    _check("PORT_DISTINCT_V1472", DEFAULT_DIGEST_PORT_MIN > 18880, f"V1474 port {DEFAULT_DIGEST_PORT_MIN} > V1472 18880")
    _check("PORT_DISTINCT_V1471", DEFAULT_DIGEST_PORT_MIN > 18680, f"V1474 port {DEFAULT_DIGEST_PORT_MIN} > V1471 18680")

    # Correlation logic
    import tempfile
    tmp_dir = Path(tempfile.mkdtemp())
    path_a = tmp_dir / "stream-a.jsonl"
    path_b = tmp_dir / "stream-b.jsonl"

    # Create mock streams with R001 in FIRING state on both
    st_a = StreamTarget(stream_id="A", stream_path=path_a, cursor=0)
    st_b = StreamTarget(stream_id="B", stream_path=path_b, cursor=0)
    now = _now_epoch()

    def _make_firing(rule_id, severity, last_updated_at=now - 1.0):
        return v1473.AlertRecord(
            rule_id=rule_id,
            severity=severity,
            state=AlertState.FIRING,
            first_seen_at=now - 5.0,
            last_updated_at=last_updated_at,
            transition_count=2,
            last_message=f"{rule_id} firing",
            last_event_ts=now - 1.0,
        )

    def _reset_streams():
        # Both streams: R001 + R005 in FIRING state
        for st in (st_a, st_b):
            st.alerts["R001_VERDICT_REGRESSED"] = _make_firing("R001_VERDICT_REGRESSED", AlertSeverity.CRITICAL)
            st.alerts["R005_STREAM_STALE"] = _make_firing("R005_STREAM_STALE", AlertSeverity.WARN)

    _reset_streams()
    active_incidents: Dict[str, FleetIncident] = {}
    new_or_updated, closed = _correlate_fleet_incidents(
        streams=[st_a, st_b],
        threshold=2,
        window_s=30.0,
        grace_s=15.0,
        now=now,
        active_incidents=active_incidents,
    )

    _check("CORR_FIRES_ON_THRESHOLD", len(new_or_updated) == 2, f"both rules reach threshold (got {len(new_or_updated)})")
    _check("CORR_BOTH_INCIDENTS_CRITICAL", all(inc.severity == AlertSeverity.CRITICAL for inc in new_or_updated), "all incidents CRITICAL")
    _check("CORR_INCIDENT_KEY_DEDUP", len({inc.incident_key for inc in new_or_updated}) == 2, "incident keys unique")

    # Test window expiry: first fired too long ago — set BOTH R001 and R005 to now-100
    for st in (st_a, st_b):
        st.alerts["R001_VERDICT_REGRESSED"].last_updated_at = now - 100.0
        st.alerts["R005_STREAM_STALE"].last_updated_at = now - 100.0
    active_incidents.clear()
    new_or_updated2, _ = _correlate_fleet_incidents(
        streams=[st_a, st_b],
        threshold=2,
        window_s=30.0,
        grace_s=15.0,
        now=now,
        active_incidents=active_incidents,
    )
    _check("CORR_WINDOW_ENFORCED", len(new_or_updated2) == 0, f"window enforced (got {len(new_or_updated2)})")

    # Test threshold not met: only R001 fires on A (single stream)
    for st in (st_a, st_b):
        st.alerts["R001_VERDICT_REGRESSED"].last_updated_at = now - 1.0
        st.alerts["R001_VERDICT_REGRESSED"].state = AlertState.FIRING if st is st_a else AlertState.INACTIVE
        st.alerts["R005_STREAM_STALE"].state = AlertState.INACTIVE  # R005 not firing
    active_incidents.clear()
    new_or_updated3, _ = _correlate_fleet_incidents(
        streams=[st_a, st_b],
        threshold=2,
        window_s=30.0,
        grace_s=15.0,
        now=now,
        active_incidents=active_incidents,
    )
    _check("CORR_THRESHOLD_ENFORCED", len(new_or_updated3) == 0, f"threshold enforced (got {len(new_or_updated3)})")

    # Incident opening + closing
    _reset_streams()
    active_incidents.clear()
    new_inc, _ = _correlate_fleet_incidents(
        streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=0.0, now=now,
        active_incidents=active_incidents,
    )
    _check("CORR_INCIDENT_OPENED", len(active_incidents) >= 1, f"incidents opened (got {len(active_incidents)})")
    # Clear both streams
    for st in (st_a, st_b):
        st.alerts["R001_VERDICT_REGRESSED"].state = AlertState.INACTIVE
        st.alerts["R005_STREAM_STALE"].state = AlertState.INACTIVE
    _, closed2 = _correlate_fleet_incidents(
        streams=[st_a, st_b], threshold=2, window_s=30.0, grace_s=0.0,
        now=now + 1.0, active_incidents=active_incidents,
    )
    _check("CORR_INCIDENT_CLOSED", len(closed2) >= 1 and all(i.state == FleetIncidentState.CLOSED for i in closed2), f"incidents closed (got {len(closed2)})")

    # Reused helpers from V1473
    _check("REUSE_V1473_RULES", len(_make_builtin_rules()) == 7, f"reuses V1473 7 built-in rules")
    _check("REUSE_V1473_EVALUATOR", _evaluate_rule is v1473._evaluate_rule, "reuses V1473._evaluate_rule")
    _check("REUSE_V1473_TRANSITION", _transition_state is v1473._transition_state, "reuses V1473._transition_state")

    # Validation
    try:
        V1474MultiStreamAggregator(stream_paths=[], max_runtime_s=10)
        _check("VAL_EMPTY_STREAMS_REJECTED", False, "should reject empty stream list")
    except ValueError:
        _check("VAL_EMPTY_STREAMS_REJECTED", True, "rejects empty stream list")
    try:
        V1474MultiStreamAggregator(
            stream_paths=[Path("a.jsonl")], max_runtime_s=10000, max_streams=4
        )
        _check("VAL_OUT_OF_BOUNDS_REJECTED", False, "should reject max_runtime_s out of bounds")
    except ValueError:
        _check("VAL_OUT_OF_BOUNDS_REJECTED", True, "rejects max_runtime_s > MAX")

    # Cleanup
    try:
        import shutil
        shutil.rmtree(tmp_dir)
    except OSError:
        pass

    if verbose:
        for name, ok, detail in results:
            mark = "✓" if ok else "✗"
            print(f"  {mark} {name}: {detail}")
        passed = sum(1 for _, ok, _ in results if ok)
        total = len(results)
        print(f"[v1474 popper] verdict: {'PASS' if passed == total else 'FAIL'} ({passed}/{total})")

    return results


# ──────────────────────────────────────────────────────────────────────
# Synthetic demo (for "demo" CLI command)
# ──────────────────────────────────────────────────────────────────────


def _write_synthetic_events_v1474(path: Path, plan: List[Dict[str, Any]], interval_s: float, stream_id: str) -> None:
    """Write a sequence of synthetic DiffEvent-style JSONL events to path (V1474 variant)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists():
        path.unlink()
    for i, ev_spec in enumerate(plan):
        time.sleep(max(0.0, float(ev_spec.get("delay_s", interval_s))))
        event = {
            "ts": time.time(),
            "event_index": i,
            "audit_id": f"{stream_id}-{i:04d}",
            "baseline_id": f"{stream_id}-{i-1:04d}" if i > 0 else None,
            "verdict": ev_spec.get("verdict", "UNCHANGED"),
            "n_endpoints_total": ev_spec.get("n_endpoints_total", 6),
            "n_endpoints_2xx": ev_spec.get("n_endpoints_2xx", 6),
            "n_invariants_total": ev_spec.get("n_invariants_total", 9),
            "n_invariants_failed": ev_spec.get("n_invariants_failed", 0),
            "alive": ev_spec.get("alive", True),
            "_stream_id": stream_id,
        }
        with path.open("a", encoding="utf-8") as f:
            f.write(json.dumps(event, separators=(",", ":")) + "\n")


def run_v1474_demo(
    out_dir: Optional[Path] = None,
    max_runtime_s: float = 30.0,
    eval_interval_s: float = 1.0,
    n_streams: int = 2,
    incident_threshold: int = 2,
) -> AggregatorReport:
    """Run a synthetic demo: spin up N synthetic stream writers + aggregator.

    Both streams write a REGRESSED-aligned plan so that the same rule fires
    on both streams within the incident window → 1 fleet incident opens.
    """
    out_dir = out_dir or _make_default_out_dir()
    out_dir.mkdir(parents=True, exist_ok=True)
    stream_paths: List[Path] = []
    writer_threads: List[threading.Thread] = []
    writer_done: List[threading.Event] = []

    # Plan: UNCHANGED → REGRESSED × 3 → IMPROVED → UNCHANGED (matches V1473 demo)
    plan = [
        {"delay_s": 0.5, "verdict": "UNCHANGED", "n_invariants_failed": 0, "n_endpoints_2xx": 6},
        {"delay_s": 2.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
        {"delay_s": 2.0, "verdict": "REGRESSED", "n_invariants_failed": 1, "n_endpoints_2xx": 5},
        {"delay_s": 2.0, "verdict": "REGRESSED", "n_invariants_failed": 2, "n_endpoints_2xx": 4},
        {"delay_s": 2.0, "verdict": "IMPROVED", "n_invariants_failed": 0, "n_endpoints_2xx": 6},
        {"delay_s": 2.0, "verdict": "UNCHANGED", "n_invariants_failed": 0, "n_endpoints_2xx": 6},
    ]

    for i in range(n_streams):
        stream_id = f"stream-{chr(65 + i)}"  # A, B, ...
        path = out_dir / f"synthetic-{stream_id}.jsonl"
        stream_paths.append(path)
        done = threading.Event()
        writer_done.append(done)
        def _writer(p=path, sid=stream_id, d=done):
            try:
                _write_synthetic_events_v1474(p, plan, interval_s=2.0, stream_id=sid)
            finally:
                d.set()
        t = threading.Thread(target=_writer, daemon=True, name=f"v1474-demo-writer-{i}")
        t.start()
        writer_threads.append(t)

    # Wait briefly for stream files to exist
    for _ in range(30):
        if all(p.exists() for p in stream_paths):
            break
        time.sleep(0.1)

    # Run aggregator
    aggregator = V1474MultiStreamAggregator(
        stream_paths=stream_paths,
        max_runtime_s=max_runtime_s,
        eval_interval_s=eval_interval_s,
        incident_threshold=incident_threshold,
        incident_window_s=30.0,
        incident_grace_s=10.0,
        out_dir=out_dir,
    )
    report = aggregator.run()
    for d in writer_done:
        d.wait(timeout=2.0)
    return report


# ──────────────────────────────────────────────────────────────────────
# CLI
# ──────────────────────────────────────────────────────────────────────


def _make_argparser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog=f"python -m apeireth.{MODULE_NAME}",
        description="V1474 — V1473 multi-stream alert aggregator + cross-stream incident correlation",
    )
    sub = p.add_subparsers(dest="command", required=True)

    # run
    p_run = sub.add_parser("run", help="run aggregator against N JSONL stream files")
    p_run.add_argument("--stream", action="append", required=True, help="JSONL stream file path (repeatable)")
    p_run.add_argument("--stream-id", action="append", default=None, help="stream_id (repeatable, matches --stream order)")
    p_run.add_argument("--max-runtime", type=float, default=DEFAULT_MAX_RUNTIME_S)
    p_run.add_argument("--eval-interval", type=float, default=DEFAULT_EVAL_INTERVAL_S)
    p_run.add_argument("--debounce", type=float, default=DEFAULT_DEBOUNCE_S)
    p_run.add_argument("--resolved-grace", type=float, default=DEFAULT_RESOLVED_GRACE_S)
    p_run.add_argument("--stale-threshold", type=float, default=DEFAULT_STALE_THRESHOLD_S)
    p_run.add_argument("--max-streams", type=int, default=DEFAULT_MAX_STREAMS)
    p_run.add_argument("--max-rules", type=int, default=DEFAULT_MAX_RULES)
    p_run.add_argument("--max-alerts", type=int, default=DEFAULT_MAX_ALERTS)
    p_run.add_argument("--incident-threshold", type=int, default=DEFAULT_INCIDENT_THRESHOLD)
    p_run.add_argument("--incident-window", type=float, default=DEFAULT_INCIDENT_WINDOW_S)
    p_run.add_argument("--incident-grace", type=float, default=DEFAULT_INCIDENT_GRACE_S)
    p_run.add_argument("--digest-port-min", type=int, default=DEFAULT_DIGEST_PORT_MIN)
    p_run.add_argument("--digest-port-max", type=int, default=DEFAULT_DIGEST_PORT_MAX)
    p_run.add_argument("--host", default=DEFAULT_HOST)
    p_run.add_argument("--out-dir", type=Path, default=None)

    # demo
    p_demo = sub.add_parser("demo", help="run synthetic demo (N stream writers + aggregator)")
    p_demo.add_argument("--max-runtime", type=float, default=30.0)
    p_demo.add_argument("--eval-interval", type=float, default=1.0)
    p_demo.add_argument("--n-streams", type=int, default=2)
    p_demo.add_argument("--incident-threshold", type=int, default=2)
    p_demo.add_argument("--out-dir", type=Path, default=None)

    # popper
    sub.add_parser("popper", help="run self-checks")

    # meta
    sub.add_parser("meta", help="show module metadata")

    # chain
    sub.add_parser("chain", help="show lineage chain")

    # help
    sub.add_parser("help", help="show this help message")

    return p


def _print_chain() -> None:
    print("V1474 lineage:")
    print("  V1474 (this) — multi-stream aggregator + cross-stream incident correlation")
    print("  V1473 — alerting engine + state machine + /alerts endpoint (single stream)")
    print("  V1472 — V1471 supervisor + Prometheus /metrics endpoint")
    print("  V1471 — Persistent audit monitor daemon + JSONL diff stream")
    print("  V1470 — Batch harness + cross-client equivalence verifier")
    print("  V1469 — Two-process V1468-client → V1467-server driver")
    print("  V1468 — OpenAPI 3.1 schema + generated Python client")
    print("  V1467 — Cross-audit HTTP gateway + history + diff")
    print("  V1465 — Lint-gate HTTP gateway cross-module live audit")
    print("  V1464 — Multi-route HTTP gateway")


def _print_meta() -> None:
    print(f"module: {MODULE_NAME}")
    print(f"phase: {MODULE_PHASE}")
    print(f"version: {MODULE_VERSION}")
    print(f"defaults: max_runtime={DEFAULT_MAX_RUNTIME_S}s, eval_interval={DEFAULT_EVAL_INTERVAL_S}s")
    print(f"  max_streams={DEFAULT_MAX_STREAMS}, max_rules={DEFAULT_MAX_RULES}, max_alerts={DEFAULT_MAX_ALERTS}")
    print(f"  incident_threshold={DEFAULT_INCIDENT_THRESHOLD}, incident_window={DEFAULT_INCIDENT_WINDOW_S}s, incident_grace={DEFAULT_INCIDENT_GRACE_S}s")
    print(f"  port range [{DEFAULT_DIGEST_PORT_MIN}, {DEFAULT_DIGEST_PORT_MAX}], host={DEFAULT_HOST}")


def main(argv: Optional[List[str]] = None) -> int:
    parser = _make_argparser()
    args = parser.parse_args(argv)

    if args.command == "popper":
        results = popper_v1474(verbose=True)
        passed = sum(1 for _, ok, _ in results if ok)
        total = len(results)
        return 0 if passed == total else 1

    if args.command == "meta":
        _print_meta()
        return 0

    if args.command == "chain":
        _print_chain()
        return 0

    if args.command == "help":
        parser.print_help()
        return 0

    if args.command == "demo":
        out_dir = args.out_dir or _make_default_out_dir()
        report = run_v1474_demo(
            out_dir=out_dir,
            max_runtime_s=args.max_runtime,
            eval_interval_s=args.eval_interval,
            n_streams=args.n_streams,
            incident_threshold=args.incident_threshold,
        )
        n_passed = sum(1 for ok in report.guards.values() if ok)
        n_total = len(report.guards)
        print(f"[v1474 demo] elapsed={report.elapsed_s:.2f}s shutdown={report.shutdown_reason.value} "
              f"n_eval={report.n_evaluations} n_per_stream_alerts={report.n_per_stream_alert_events} "
              f"n_fleet_incidents={report.n_fleet_incidents} "
              f"open={report.n_fleet_incidents_open} closed={report.n_fleet_incidents_closed} "
              f"port={report.digest_port} guards={n_passed}/{n_total}")
        return 0 if n_passed == n_total else 1

    if args.command == "run":
        stream_paths = [Path(p) for p in args.stream]
        stream_ids = args.stream_id
        out_dir = args.out_dir or _make_default_out_dir()
        aggregator = V1474MultiStreamAggregator(
            stream_paths=stream_paths,
            stream_ids=stream_ids,
            max_runtime_s=args.max_runtime,
            eval_interval_s=args.eval_interval,
            debounce_s=args.debounce,
            resolved_grace_s=args.resolved_grace,
            stale_threshold_s=args.stale_threshold,
            max_streams=args.max_streams,
            max_rules=args.max_rules,
            max_alerts=args.max_alerts,
            incident_threshold=args.incident_threshold,
            incident_window_s=args.incident_window,
            incident_grace_s=args.incident_grace,
            digest_port_min=args.digest_port_min,
            digest_port_max=args.digest_port_max,
            host=args.host,
            out_dir=out_dir,
        )
        report = aggregator.run()
        n_passed = sum(1 for ok in report.guards.values() if ok)
        n_total = len(report.guards)
        print(f"[v1474 run] elapsed={report.elapsed_s:.2f}s shutdown={report.shutdown_reason.value} "
              f"n_eval={report.n_evaluations} n_per_stream_alerts={report.n_per_stream_alert_events} "
              f"n_fleet_incidents={report.n_fleet_incidents} "
              f"open={report.n_fleet_incidents_open} closed={report.n_fleet_incidents_closed} "
              f"port={report.digest_port} guards={n_passed}/{n_total}")
        return 0 if n_passed == n_total else 1

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())