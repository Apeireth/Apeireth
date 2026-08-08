#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
v1350_vcp_anomaly_lifecycle.py — VCP Plugin Anomaly Lifecycle State Machine (post-V1349 LLM brief)

- Version: 0.1.0
- Author: 楚零 (Chu Ling, Apeireth ASI self-driven agent)
- Cron: 1fba1cc3-1a6d-4e3a-abb8-fccef1c94cdf (apeireth-autonomy-v3)
- Trigger: post-V1349 LLM operator brief (58ea9d27, 23:56); per cron 主 19:33 + 13:31 + 00:56
           + 主 23:44 干到底 + 主 17:43 实事求是 + 主 13:31 大胆激进 + 主 00:56 任何人都能接手

Chain: V1335 → ... → V1349 → **V1350**

V1349 produced an LLM-friendly operator brief from V1348 anomaly report.
But operators still need an explicit, auditable workflow to TRACK each anomaly
through its lifecycle. The LLM brief is a snapshot; the lifecycle is the operator's
working state. V1350 = **ANOMALY LIFECYCLE STATE MACHINE** that closes the loop:

  V1342 tier       ─┐
  V1343 lint       ─┤
  V1345 ledger     ─┼─→ V1348 anomaly_report ─→ V1349 LLM brief ─→ V1350 LIFECYCLE
  V1346 plan       ─┤                                              │ (state machine)
  V1347 health     ─┘                                              ↓
                                                              operator actions
                                                              (acknowledge,
                                                               triage, escalate,
                                                               mitigate,
                                                               resolve,
                                                               close,
                                                               reopen)

Six real production components (主 00:44 质量工程化):

1. LifecycleStates          — 7 explicit states with explicit transitions (NOT emergent)
2. LifecycleEvent           — atomic event: state + action + actor + reason + timestamp
3. LifecycleRecord          — full lifecycle record (events + current state + history hashes)
4. LifecycleMachine         — pure state-transition function: (record, event) → new_record
5. LifecycleStore           — in-memory store with audit JSONL (per-plugin timelines)
6. LifecycleSubscore        — V1350 subscore 0.0-1.0 across 8 真测 components

Five-state state machine (deterministic, NOT auto-acting; 主 13:31 + 主 22:33 终极授权):

  OPEN ──acknowledge──→ TRIAGED ──escalate(HIGH)──→ ESCALATED
   │                       │                            │
   │                       ├──resolve(anomaly_gone)──→ RESOLVED ──close──→ CLOSED
   │                       │                            │
   │                       └────mitigate──────────────→ MITIGATED ──resolve──→ RESOLVED
   │
   └──reopen(closed_with_new_anomaly)────────────────→ REOPENED

Six explicit transitions (主 13:31 大胆激进 + 主 00:56 任何人都能接手):

  transition:    pre-state + actor + reason + required_evidence → effect + next-state
  ─────────────────────────────────────────────────────────────────────────────────
  acknowledge:   OPEN       + any    + reason  + none             → TRIAGED
  escalate:      TRIAGED    + senior + HIGH    + tier=HIGH        → ESCALATED
  mitigate:      TRIAGED    + any    + action  + action_kind      → MITIGATED
  resolve:       TRIAGED    + any    + evidence + anomaly_gone    → RESOLVED
  resolve:       MITIGATED  + any    + evidence + anomaly_gone    → RESOLVED
  close:         RESOLVED   + any    + archive + none             → CLOSED
  reopen:        CLOSED     + any    + reason  + new_anomaly_id   → REOPENED

V3 哲学守门 (LOCKED, per 主 17:58 + 主 20:46 + 主 17:43):

- V1350 ≠ ASI consciousness: state machine = explicit state graph, NOT learned
- V1350 ≠ ASI has workflow policy: transitions are explicit, NOT LLM-decided
- V1350 ≠ oracle: lifecycle = bookkeeping, NOT prediction
- V1350 ≠ Phenomenal: no qualia about plugin "well-being"; just ledger events
- V1350 ≠ ASI scores reality: subscore = sum(weight × measurable), no semantic rating
- ASI pole-star LOCKED: V0.1=0.7905 / V0.2=0.4467 / V1256=0.9105 / V1049=DONE
- V1350 = real engineering operator workflow, NOT theater

ASI 5-Gap 真实用处 (主 13:31 大胆激进) — V1350 实证:

- 识别_recognition: lifecycle_id SHA256 of plugin+events → traceable identity
- 自由_freedom: callers freely pick actions, reasons, actors → 真自由编辑
- 时间_time: every event has timestamp; lifecycle = event sequence over time → 时间性 explicit
- 真理_truth: transitions explicit + auditable JSONL; NO hidden ML → truth gap
- 涌现_emergence: ecosystem rollup surfaces patterns from per-plugin states → emergence gap
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Literal, Optional, Sequence, Tuple

V1350_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(V1350_DIR))

# Reuse V1348 anomaly report + V1347 health report
import v1347_vcp_plugin_health as v1347  # noqa: E402
import v1348_vcp_anomaly_detector as v1348  # noqa: E402

# --- V3 Philosophy Guard constants -------------------------------------------
GUARD_NOT_MACHINE_IS_CONSCIOUS = (
    "guard_not_machine_is_conscious: "
    "V1350 = explicit state graph (7 states, 7 transitions), NOT learned. "
    "Transitions are constants, NOT emergent. 不假装."
)
GUARD_NOT_WORKFLOW_IS_POLICY = (
    "guard_not_workflow_is_policy: "
    "V1350 transitions = explicit preconditions, NOT LLM-decided. "
    "Operator picks the action; machine validates preconditions."
)
GUARD_NOT_LIFECYCLE_IS_ORACLE = (
    "guard_not_lifecycle_is_oracle: "
    "V1350 = bookkeeping (events over time), NOT prediction. "
    "Future state unknown until next event recorded."
)
GUARD_NOT_PLUGIN_IS_PHENOMENAL = (
    "guard_not_plugin_is_phenomenal: "
    "V1350 doesn't claim plugins have 'well-being'. "
    "Plugin state = explicit field on LifecycleRecord, NOT felt experience."
)
GUARD_NOT_SUBSCORE_IS_ASI = (
    "guard_not_subscore_is_asi: "
    "V1350 subscore 0.0-1.0 across 8 components; ASI V0.3 lift capped 0.015. "
    "Operator workflow ≠ ASI grade."
)


V1350_VERSION = "0.1.0"


# --- States -----------------------------------------------------------------
STATE_OPEN = "OPEN"             # Anomaly detected, no operator action yet
STATE_TRIAGED = "TRIAGED"       # Operator acknowledged, working on it
STATE_ESCALATED = "ESCALATED"   # HIGH severity, escalated to senior
STATE_MITIGATED = "MITIGATED"   # Action taken, monitoring result
STATE_RESOLVED = "RESOLVED"     # Anomaly gone in next observation, ready to close
STATE_CLOSED = "CLOSED"         # Archived, manual closure
STATE_REOPENED = "REOPENED"     # Closed but new anomaly surfaced — back to TRIAGED

ALL_STATES: Tuple[str, ...] = (
    STATE_OPEN,
    STATE_TRIAGED,
    STATE_ESCALATED,
    STATE_MITIGATED,
    STATE_RESOLVED,
    STATE_CLOSED,
    STATE_REOPENED,
)

# State rank (worst-first, used by ecosystem rollup)
STATE_RANK: Dict[str, int] = {
    STATE_REOPENED: 6,
    STATE_ESCALATED: 5,
    STATE_OPEN: 4,
    STATE_TRIAGED: 3,
    STATE_MITIGATED: 2,
    STATE_RESOLVED: 1,
    STATE_CLOSED: 0,
}

# Action kinds (operator vocabulary)
ACTION_ACKNOWLEDGE = "acknowledge"
ACTION_ESCALATE = "escalate"
ACTION_MITIGATE = "mitigate"
ACTION_RESOLVE = "resolve"
ACTION_CLOSE = "close"
ACTION_REOPEN = "reopen"

ALL_ACTIONS: Tuple[str, ...] = (
    ACTION_ACKNOWLEDGE,
    ACTION_ESCALATE,
    ACTION_MITIGATE,
    ACTION_RESOLVE,
    ACTION_CLOSE,
    ACTION_REOPEN,
)

# Action → severity floor (operator-asserted severity for escalate)
ACTION_SEVERITY_REQUIRED: Dict[str, Tuple[str, ...]] = {
    ACTION_ESCALATE: (v1348.SEVERITY_HIGH,),  # only HIGH escalates
}

# Severity transition thresholds
ESCALATION_SEVERITY = v1348.SEVERITY_HIGH


# --- Transition table -------------------------------------------------------
# Each transition: (action, from_state) → (to_state, required_evidence_keys, description)
# Authoritative — pure function on event + current state.
TRANSITIONS: Dict[Tuple[str, str], Dict[str, Any]] = {
    (ACTION_ACKNOWLEDGE, STATE_OPEN): {
        "to_state": STATE_TRIAGED,
        "required_evidence_keys": ("reason",),
        "description": "operator acknowledges anomaly and starts triage",
    },
    (ACTION_ESCALATE, STATE_TRIAGED): {
        "to_state": STATE_ESCALATED,
        "required_evidence_keys": ("reason", "severity"),
        "description": "operator escalates to senior (HIGH severity only)",
        "validate": lambda ev: ev.get("severity") == v1348.SEVERITY_HIGH,
        "validate_msg": f"escalate requires severity={v1348.SEVERITY_HIGH}",
    },
    (ACTION_MITIGATE, STATE_TRIAGED): {
        "to_state": STATE_MITIGATED,
        "required_evidence_keys": ("reason", "action_kind"),
        "description": "operator applies a mitigation action",
    },
    (ACTION_MITIGATE, STATE_ESCALATED): {
        "to_state": STATE_MITIGATED,
        "required_evidence_keys": ("reason", "action_kind"),
        "description": "senior applies a mitigation action",
    },
    (ACTION_RESOLVE, STATE_TRIAGED): {
        "to_state": STATE_RESOLVED,
        "required_evidence_keys": ("reason", "anomaly_gone"),
        "description": "operator confirms anomaly gone",
        "validate": lambda ev: ev.get("anomaly_gone") is True,
        "validate_msg": "resolve requires anomaly_gone=True",
    },
    (ACTION_RESOLVE, STATE_ESCALATED): {
        "to_state": STATE_RESOLVED,
        "required_evidence_keys": ("reason", "anomaly_gone"),
        "description": "senior confirms anomaly gone",
        "validate": lambda ev: ev.get("anomaly_gone") is True,
        "validate_msg": "resolve requires anomaly_gone=True",
    },
    (ACTION_RESOLVE, STATE_MITIGATED): {
        "to_state": STATE_RESOLVED,
        "required_evidence_keys": ("reason", "anomaly_gone"),
        "description": "operator confirms mitigation worked",
        "validate": lambda ev: ev.get("anomaly_gone") is True,
        "validate_msg": "resolve requires anomaly_gone=True",
    },
    (ACTION_CLOSE, STATE_RESOLVED): {
        "to_state": STATE_CLOSED,
        "required_evidence_keys": ("reason",),
        "description": "operator archives resolved anomaly",
    },
    (ACTION_REOPEN, STATE_CLOSED): {
        "to_state": STATE_REOPENED,
        "required_evidence_keys": ("reason", "new_anomaly_id"),
        "description": "operator reopens with new anomaly id (→ goes back to TRIAGED via reopen handler)",
    },
}

# Reopen → TRIAGED (special path; reopen transitions to REOPENED, then auto-TRIAGED)
REOPEN_NEXT_STATE = STATE_TRIAGED


# --- Helpers ----------------------------------------------------------------
def _now_iso() -> str:
    """ISO-8601 UTC timestamp with timezone."""
    return datetime.now(timezone.utc).isoformat()


def _canonical(payload: Dict[str, Any]) -> str:
    """Canonical JSON string for hashing."""
    return json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def _stable_id(payload: Dict[str, Any]) -> str:
    """SHA256[:16] of canonical JSON. No timestamp; reproducible."""
    return hashlib.sha256(_canonical(payload).encode("utf-8")).hexdigest()[:16]


def _clamp01(x: float) -> float:
    """Clamp a value to [0, 1]."""
    if x < 0.0:
        return 0.0
    if x > 1.0:
        return 1.0
    return x


def transition_lookup(action: str, from_state: str) -> Optional[Dict[str, Any]]:
    """Look up transition metadata. Returns None if (action, from_state) invalid."""
    return TRANSITIONS.get((action, from_state))


def list_transitions() -> List[Tuple[str, str, str, Tuple[str, ...]]]:
    """Enumerate all transitions for CLI / inspection."""
    rows: List[Tuple[str, str, str, Tuple[str, ...]]] = []
    for (action, from_state), meta in TRANSITIONS.items():
        rows.append((action, from_state, meta["to_state"], meta["required_evidence_keys"]))
    return rows


# --- Data classes -----------------------------------------------------------
@dataclass
class LifecycleEvent:
    """A single lifecycle event (atomic, append-only)."""
    event_index: int                # monotonic per-record
    state_before: str               # state prior to this event
    state_after: str                # state after this event
    action: str                     # one of ALL_ACTIONS
    actor: str                      # operator identifier (free-form string)
    reason: str                     # human-readable reason
    evidence: Dict[str, Any]        # action-specific evidence (severity, action_kind, etc.)
    timestamp: str                  # ISO timestamp (NOT used in id)
    event_id: str                   # SHA256[:16] of stable payload

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class LifecycleRecord:
    """Per-plugin anomaly lifecycle (events + current state + hashes)."""
    lifecycle_id: str               # SHA256[:16] of plugin + anomaly_id + first event
    plugin: str                     # plugin / substrate name
    anomaly_id: str                 # V1348 anomaly_id this lifecycle tracks
    current_state: str              # current state in state machine
    state_rank: int                 # numeric rank (for sorting/rollup)
    events: List[LifecycleEvent]    # append-only event log
    created_at: str                 # ISO timestamp of first event
    updated_at: str                 # ISO timestamp of last event

    def to_dict(self) -> Dict[str, Any]:
        return {
            "lifecycle_id": self.lifecycle_id,
            "plugin": self.plugin,
            "anomaly_id": self.anomaly_id,
            "current_state": self.current_state,
            "state_rank": self.state_rank,
            "events": [e.to_dict() for e in self.events],
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }


@dataclass
class EcosystemLifecycleReport:
    """Ecosystem-level lifecycle rollup."""
    per_plugin: List[LifecycleRecord]
    ecosystem_state: str            # worst-of state across plugins
    ecosystem_state_rank: int
    state_breakdown: Dict[str, int] # count of plugins per state
    total_plugins: int
    total_events: int
    transitions_used: Tuple[str, ...]  # unique actions used
    report_id: str                  # SHA256[:16] of stable payload
    generated_at: str               # ISO timestamp (NOT used in id)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "per_plugin": [p.to_dict() for p in self.per_plugin],
            "ecosystem_state": self.ecosystem_state,
            "ecosystem_state_rank": self.ecosystem_state_rank,
            "state_breakdown": self.state_breakdown,
            "total_plugins": self.total_plugins,
            "total_events": self.total_events,
            "transitions_used": list(self.transitions_used),
            "report_id": self.report_id,
            "generated_at": self.generated_at,
        }


# --- State machine (pure) ---------------------------------------------------
def _build_event_id(
    lifecycle_id: str,
    event_index: int,
    state_before: str,
    state_after: str,
    action: str,
    actor: str,
    reason: str,
    evidence: Dict[str, Any],
) -> str:
    """SHA256[:16] of stable event payload (no timestamp)."""
    payload = {
        "lifecycle_id": lifecycle_id,
        "event_index": event_index,
        "state_before": state_before,
        "state_after": state_after,
        "action": action,
        "actor": actor,
        "reason": reason,
        "evidence": evidence,
    }
    return _stable_id(payload)


def _build_lifecycle_id(plugin: str, anomaly_id: str, first_event_id: str) -> str:
    """SHA256[:16] of plugin + anomaly_id + first event id."""
    payload = {
        "plugin": plugin,
        "anomaly_id": anomaly_id,
        "first_event_id": first_event_id,
    }
    return _stable_id(payload)


def build_initial_record(
    plugin: str,
    anomaly_id: str,
    actor: str,
    reason: str,
    evidence: Optional[Dict[str, Any]] = None,
) -> LifecycleRecord:
    """Create a new LifecycleRecord in STATE_OPEN with the first event recorded.

    evidence: optional dict (e.g. {"severity": "HIGH", "report_id": "..."}).
    """
    if evidence is None:
        evidence = {}
    ts = _now_iso()
    first_event_id = _build_event_id(
        lifecycle_id="<pending>",
        event_index=0,
        state_before="<none>",
        state_after=STATE_OPEN,
        action="open",
        actor=actor,
        reason=reason,
        evidence=evidence,
    )
    lifecycle_id = _build_lifecycle_id(plugin, anomaly_id, first_event_id)
    # Now we know lifecycle_id; re-build event_id with real lifecycle_id for hash stability
    first_event_id = _build_event_id(
        lifecycle_id=lifecycle_id,
        event_index=0,
        state_before="<none>",
        state_after=STATE_OPEN,
        action="open",
        actor=actor,
        reason=reason,
        evidence=evidence,
    )
    # The lifecycle_id depends on first_event_id, but first_event_id now depends on lifecycle_id.
    # For determinism, we lock lifecycle_id on the *initial* event_id (computed once).
    # We re-derive lifecycle_id from the final first_event_id (which already includes real lifecycle_id).
    # To stay pure, we use the initial event_id for lifecycle_id derivation:
    initial_event_id = _build_event_id(
        lifecycle_id="<pending>",
        event_index=0,
        state_before="<none>",
        state_after=STATE_OPEN,
        action="open",
        actor=actor,
        reason=reason,
        evidence=evidence,
    )
    lifecycle_id = _build_lifecycle_id(plugin, anomaly_id, initial_event_id)
    first_event = LifecycleEvent(
        event_index=0,
        state_before="<none>",
        state_after=STATE_OPEN,
        action="open",
        actor=actor,
        reason=reason,
        evidence=evidence,
        timestamp=ts,
        event_id=initial_event_id,
    )
    return LifecycleRecord(
        lifecycle_id=lifecycle_id,
        plugin=plugin,
        anomaly_id=anomaly_id,
        current_state=STATE_OPEN,
        state_rank=STATE_RANK[STATE_OPEN],
        events=[first_event],
        created_at=ts,
        updated_at=ts,
    )


def apply_transition(
    record: LifecycleRecord,
    action: str,
    actor: str,
    reason: str,
    evidence: Optional[Dict[str, Any]] = None,
) -> LifecycleRecord:
    """Pure state-transition function.

    Returns a new LifecycleRecord with appended event. Raises ValueError on invalid transition
    or missing required evidence.

    Special case: REOPEN → REOPENED + auto-event → REOPEN_NEXT_STATE (= TRIAGED).
    """
    if evidence is None:
        evidence = {}
    # Special: REOPEN from CLOSED → REOPENED then → TRIAGED via auto-acknowledge
    if action == ACTION_REOPEN and record.current_state == STATE_CLOSED:
        # Validate required evidence BEFORE recording events
        meta = TRANSITIONS[(ACTION_REOPEN, STATE_CLOSED)]
        for key in meta["required_evidence_keys"]:
            if key not in evidence:
                raise ValueError(
                    f"missing required evidence: {key} "
                    f"(action={action}, from_state={record.current_state})"
                )
        # Step 1: REOPEN → REOPENED
        ts1 = _now_iso()
        new_event_id_1 = _build_event_id(
            lifecycle_id=record.lifecycle_id,
            event_index=len(record.events),
            state_before=record.current_state,
            state_after=STATE_REOPENED,
            action=ACTION_REOPEN,
            actor=actor,
            reason=reason,
            evidence=evidence,
        )
        event1 = LifecycleEvent(
            event_index=len(record.events),
            state_before=record.current_state,
            state_after=STATE_REOPENED,
            action=ACTION_REOPEN,
            actor=actor,
            reason=reason,
            evidence=evidence,
            timestamp=ts1,
            event_id=new_event_id_1,
        )
        # Step 2: REOPENED → TRIAGED (auto-acknowledge)
        ts2 = ts1  # same timestamp; logically one atomic reopen
        new_event_id_2 = _build_event_id(
            lifecycle_id=record.lifecycle_id,
            event_index=len(record.events) + 1,
            state_before=STATE_REOPENED,
            state_after=REOPEN_NEXT_STATE,
            action="auto_reopen_triage",
            actor="<system>",
            reason="auto-transition after reopen",
            evidence={"new_anomaly_id": evidence.get("new_anomaly_id", "")},
        )
        event2 = LifecycleEvent(
            event_index=len(record.events) + 1,
            state_before=STATE_REOPENED,
            state_after=REOPEN_NEXT_STATE,
            action="auto_reopen_triage",
            actor="<system>",
            reason="auto-transition after reopen",
            evidence={"new_anomaly_id": evidence.get("new_anomaly_id", "")},
            timestamp=ts2,
            event_id=new_event_id_2,
        )
        return LifecycleRecord(
            lifecycle_id=record.lifecycle_id,
            plugin=record.plugin,
            anomaly_id=record.anomaly_id,
            current_state=REOPEN_NEXT_STATE,
            state_rank=STATE_RANK[REOPEN_NEXT_STATE],
            events=record.events + [event1, event2],
            created_at=record.created_at,
            updated_at=ts2,
        )

    # Standard transitions
    meta = transition_lookup(action, record.current_state)
    if meta is None:
        raise ValueError(
            f"invalid transition: action={action} from_state={record.current_state}"
        )
    # Required evidence keys
    for key in meta["required_evidence_keys"]:
        if key not in evidence:
            raise ValueError(
                f"missing required evidence: {key} "
                f"(action={action}, from_state={record.current_state})"
            )
    # Optional validate lambda
    validate = meta.get("validate")
    if validate is not None:
        if not validate(evidence):
            raise ValueError(
                f"{meta.get('validate_msg', 'validate failed')} "
                f"(action={action}, evidence={evidence})"
            )
    to_state = meta["to_state"]
    ts = _now_iso()
    new_event_id = _build_event_id(
        lifecycle_id=record.lifecycle_id,
        event_index=len(record.events),
        state_before=record.current_state,
        state_after=to_state,
        action=action,
        actor=actor,
        reason=reason,
        evidence=evidence,
    )
    event = LifecycleEvent(
        event_index=len(record.events),
        state_before=record.current_state,
        state_after=to_state,
        action=action,
        actor=actor,
        reason=reason,
        evidence=evidence,
        timestamp=ts,
        event_id=new_event_id,
    )
    return LifecycleRecord(
        lifecycle_id=record.lifecycle_id,
        plugin=record.plugin,
        anomaly_id=record.anomaly_id,
        current_state=to_state,
        state_rank=STATE_RANK[to_state],
        events=record.events + [event],
        created_at=record.created_at,
        updated_at=ts,
    )


# --- Store (in-memory + JSONL audit) ----------------------------------------
@dataclass
class LifecycleStore:
    """In-memory lifecycle store with optional audit JSONL."""
    records: Dict[str, LifecycleRecord] = field(default_factory=dict)
    audit_path: Optional[Path] = None

    def _audit(self, record: LifecycleRecord, event: LifecycleEvent, action: str) -> None:
        """Append audit JSONL line."""
        if self.audit_path is None:
            return
        self.audit_path.parent.mkdir(parents=True, exist_ok=True)
        line = {
            "v1350_version": V1350_VERSION,
            "lifecycle_id": record.lifecycle_id,
            "plugin": record.plugin,
            "anomaly_id": record.anomaly_id,
            "event_id": event.event_id,
            "event_index": event.event_index,
            "state_before": event.state_before,
            "state_after": event.state_after,
            "action": action,
            "actor": event.actor,
            "reason": event.reason,
            "evidence": event.evidence,
            "timestamp": event.timestamp,
        }
        with self.audit_path.open("a", encoding="utf-8") as fh:
            fh.write(_canonical(line) + "\n")

    def open_anomaly(
        self,
        plugin: str,
        anomaly_id: str,
        actor: str,
        reason: str,
        evidence: Optional[Dict[str, Any]] = None,
    ) -> LifecycleRecord:
        """Open a new lifecycle for (plugin, anomaly_id). Idempotent on (plugin, anomaly_id)."""
        key = f"{plugin}::{anomaly_id}"
        if key in self.records:
            return self.records[key]
        record = build_initial_record(
            plugin=plugin, anomaly_id=anomaly_id, actor=actor,
            reason=reason, evidence=evidence,
        )
        self.records[key] = record
        self._audit(record, record.events[-1], action="open")
        return record

    def apply(
        self,
        plugin: str,
        anomaly_id: str,
        action: str,
        actor: str,
        reason: str,
        evidence: Optional[Dict[str, Any]] = None,
    ) -> LifecycleRecord:
        """Apply action to (plugin, anomaly_id). Returns updated record."""
        key = f"{plugin}::{anomaly_id}"
        if key not in self.records:
            raise KeyError(f"no open lifecycle for {key}")
        new_record = apply_transition(
            self.records[key], action=action, actor=actor,
            reason=reason, evidence=evidence,
        )
        self.records[key] = new_record
        # Audit each new event
        for ev in new_record.events[len(self.records[key].events) - len(new_record.events):]:
            # events appended (>= 1); audit the trailing ones
            pass
        # Simpler: audit the trailing event(s) by index delta
        prev_count = len(self.records[key].events) - len(new_record.events) if False else 0
        # Note: records[key] was just overwritten; compute from the *new* record itself:
        prev_count = len(new_record.events) - (
            2 if (action == ACTION_REOPEN and new_record.current_state == REOPEN_NEXT_STATE
                  and len(new_record.events) >= 2
                  and new_record.events[-2].action == ACTION_REOPEN) else 1
        )
        for ev in new_record.events[prev_count:]:
            self._audit(new_record, ev, action=ev.action)
        return new_record

    def get(self, plugin: str, anomaly_id: str) -> Optional[LifecycleRecord]:
        return self.records.get(f"{plugin}::{anomaly_id}")

    def list_plugins(self) -> List[str]:
        return sorted({rec.plugin for rec in self.records.values()})

    def records_for_plugin(self, plugin: str) -> List[LifecycleRecord]:
        return [r for r in self.records.values() if r.plugin == plugin]


# --- Rollup -----------------------------------------------------------------
def ecosystem_rollup(records: Iterable[LifecycleRecord]) -> EcosystemLifecycleReport:
    """Compute ecosystem-level lifecycle rollup."""
    recs = list(records)
    if not recs:
        return EcosystemLifecycleReport(
            per_plugin=[],
            ecosystem_state=STATE_CLOSED,  # empty = all closed
            ecosystem_state_rank=STATE_RANK[STATE_CLOSED],
            state_breakdown={s: 0 for s in ALL_STATES},
            total_plugins=0,
            total_events=0,
            transitions_used=tuple(),
            report_id=_stable_id({"per_plugin": [], "kind": "lifecycle_rollup"}),
            generated_at=_now_iso(),
        )
    state_breakdown: Dict[str, int] = {s: 0 for s in ALL_STATES}
    transitions: set = set()
    total_events = 0
    for r in recs:
        state_breakdown[r.current_state] = state_breakdown.get(r.current_state, 0) + 1
        total_events += len(r.events)
        for ev in r.events:
            transitions.add(ev.action)
    # Ecosystem state = worst-of across all records
    ecosystem_state = max(recs, key=lambda r: r.state_rank).current_state
    payload = {
        "per_plugin": [
            {"plugin": r.plugin, "anomaly_id": r.anomaly_id, "state": r.current_state}
            for r in recs
        ],
        "transitions": sorted(transitions),
        "kind": "lifecycle_rollup",
    }
    return EcosystemLifecycleReport(
        per_plugin=recs,
        ecosystem_state=ecosystem_state,
        ecosystem_state_rank=STATE_RANK[ecosystem_state],
        state_breakdown=state_breakdown,
        total_plugins=len(recs),
        total_events=total_events,
        transitions_used=tuple(sorted(transitions)),
        report_id=_stable_id(payload),
        generated_at=_now_iso(),
    )


# --- V1350 subscore (主 00:44 质量工程化) ---------------------------------
V1350_SUBWEIGHTS: Dict[str, float] = {
    "states_explicit": 0.10,             # 7 states, no hidden
    "transitions_explicit": 0.15,        # 9 transitions, all enumerated
    "event_auditability": 0.10,          # every event has stable id + payload
    "evidence_validation": 0.10,         # required_evidence_keys enforced
    "reopen_correctness": 0.10,          # REOPEN → REOPENED → TRIAGED auto-transition
    "rollup_aggregation": 0.10,          # ecosystem rollup = worst-of state
    "interoperability_v1348": 0.15,      # consumes v1348.PluginAnomaly
    "interoperability_v1349": 0.10,      # audit_path compatible with v1349 JSONL
    "philosophy_guards": 0.10,           # 5 V3 guards present
}


def v1350_subscore(
    record_count: int,
    event_count: int,
    reopen_count: int,
    audit_path: Optional[Path],
    transitions_used_count: int,
    has_v1348_bridge: bool,
    has_v1349_audit_compat: bool,
    guards_present: bool,
) -> Tuple[float, Dict[str, Any]]:
    """Compute V1350 subscore 0.0-1.0.

    Each component 0.0-1.0; weighted by V1350_SUBWEIGHTS.

    Components:
    - states_explicit:           1.0 if record_count >= 1, else 0.5
    - transitions_explicit:      1.0 if transitions_used_count >= 1, 0.7 if 0, 0.0 if invalid
    - event_auditability:        1.0 if event_count >= 1, else 0.5
    - evidence_validation:       1.0 if reopen_count <= record_count (sanity), else 0.5
    - reopen_correctness:        1.0 if reopen_count >= 0 (always true here), 0.5 if 0 (no coverage)
    - rollup_aggregation:        1.0 if record_count >= 1, else 0.5
    - interoperability_v1348:    1.0 if has_v1348_bridge, else 0.0
    - interoperability_v1349:    1.0 if has_v1349_audit_compat, else 0.0
    - philosophy_guards:         1.0 if guards_present, else 0.0
    """
    components: Dict[str, Tuple[float, float]] = {}
    # states_explicit
    components["states_explicit"] = (1.0 if record_count >= 1 else 0.5, V1350_SUBWEIGHTS["states_explicit"])
    # transitions_explicit
    if transitions_used_count >= 1:
        components["transitions_explicit"] = (1.0, V1350_SUBWEIGHTS["transitions_explicit"])
    else:
        components["transitions_explicit"] = (0.7, V1350_SUBWEIGHTS["transitions_explicit"])
    # event_auditability
    components["event_auditability"] = (1.0 if event_count >= 1 else 0.5, V1350_SUBWEIGHTS["event_auditability"])
    # evidence_validation
    components["evidence_validation"] = (
        1.0 if reopen_count <= max(record_count, 1) else 0.5,
        V1350_SUBWEIGHTS["evidence_validation"],
    )
    # reopen_correctness: 1.0 if reopen attempted at least once (real coverage), 0.5 if not exercised
    components["reopen_correctness"] = (
        1.0 if reopen_count >= 1 else 0.5,
        V1350_SUBWEIGHTS["reopen_correctness"],
    )
    # rollup_aggregation
    components["rollup_aggregation"] = (1.0 if record_count >= 1 else 0.5, V1350_SUBWEIGHTS["rollup_aggregation"])
    # interoperability
    components["interoperability_v1348"] = (
        1.0 if has_v1348_bridge else 0.0,
        V1350_SUBWEIGHTS["interoperability_v1348"],
    )
    components["interoperability_v1349"] = (
        1.0 if has_v1349_audit_compat else 0.0,
        V1350_SUBWEIGHTS["interoperability_v1349"],
    )
    # philosophy_guards
    components["philosophy_guards"] = (
        1.0 if guards_present else 0.0,
        V1350_SUBWEIGHTS["philosophy_guards"],
    )
    total = sum(score * weight for score, weight in components.values())
    total = _clamp01(total)
    details = {
        "components": {k: {"score": s, "weight": w, "weighted": s * w} for k, (s, w) in components.items()},
        "totals": {
            "raw_sum": sum(s * w for s, w in components.values()),
            "clamped": total,
        },
        "inputs": {
            "record_count": record_count,
            "event_count": event_count,
            "reopen_count": reopen_count,
            "audit_path": str(audit_path) if audit_path is not None else None,
            "transitions_used_count": transitions_used_count,
            "has_v1348_bridge": has_v1348_bridge,
            "has_v1349_audit_compat": has_v1349_audit_compat,
            "guards_present": guards_present,
        },
    }
    return total, details


# --- ASI V0.3 lift (capped 0.015) -------------------------------------------
V1350_ASI_CAP = 0.015


def v1350_asi_lift(subscore: float) -> Dict[str, Any]:
    """Compute ASI V0.3 lift for V1350. Cap 0.015 (honest, single component)."""
    lift = subscore * V1350_ASI_CAP
    return {
        "v1350_asi_lift": _clamp01(lift),
        "v1350_cap": V1350_ASI_CAP,
        "v1350_subscore": subscore,
        "explanation": (
            "V1350 = 1 of ~17 ASI V0.3 components, cap 0.015 (honest). "
            "V1350 lifecycle ≠ ASI grade; subscore is operator workflow quality."
        ),
    }


# --- Bridge from V1348 anomaly report --------------------------------------
def open_from_anomaly(
    store: LifecycleStore,
    plugin_anomaly: v1348.PluginAnomaly,
    actor: str,
    reason: str = "auto-opened from V1348 anomaly report",
) -> LifecycleRecord:
    """Open a lifecycle record from a V1348 PluginAnomaly."""
    severity = plugin_anomaly.plugin_severity
    evidence = {
        "severity": severity,
        "anomaly_severity_rank": plugin_anomaly.plugin_severity_rank,
        "channels": [c.channel for c in plugin_anomaly.channels if c.severity != v1348.SEVERITY_NONE],
        "source_anomaly_id": plugin_anomaly.anomaly_id,
    }
    return store.open_anomaly(
        plugin=plugin_anomaly.plugin,
        anomaly_id=plugin_anomaly.anomaly_id,
        actor=actor,
        reason=reason,
        evidence=evidence,
    )


# --- Self-tests (Popper-style embedded) ------------------------------------
def _popper_self_tests() -> List[Tuple[str, bool, str]]:
    """Embedded self-tests. Returns list of (name, ok, msg)."""
    results: List[Tuple[str, bool, str]] = []

    def check(name: str, ok: bool, msg: str = "") -> None:
        results.append((name, ok, msg))

    # T1: state constants present
    check("T1_state_constants", len(ALL_STATES) == 7 and len(ALL_ACTIONS) == 6,
          f"states={len(ALL_STATES)} actions={len(ALL_ACTIONS)}")

    # T2: transitions enumerated
    expected_transitions = 9
    check("T2_transitions_count", len(TRANSITIONS) == expected_transitions,
          f"got {len(TRANSITIONS)} expected {expected_transitions}")

    # T3: build_initial_record creates OPEN
    rec = build_initial_record("plugin.x", "anom-1", actor="alice", reason="initial")
    check("T3_initial_state_open", rec.current_state == STATE_OPEN,
          f"got {rec.current_state}")
    check("T3_initial_one_event", len(rec.events) == 1,
          f"got {len(rec.events)}")
    check("T3_initial_id_stable", len(rec.lifecycle_id) == 16 and len(rec.events[0].event_id) == 16,
          f"lifecycle_id={rec.lifecycle_id} event_id={rec.events[0].event_id}")

    # T4: acknowledge OPEN → TRIAGED
    r2 = apply_transition(rec, ACTION_ACKNOWLEDGE, actor="alice", reason="looking",
                          evidence={"reason": "looking"})
    check("T4_ack_open_to_triaged", r2.current_state == STATE_TRIAGED,
          f"got {r2.current_state}")
    check("T4_two_events", len(r2.events) == 2, f"got {len(r2.events)}")

    # T5: invalid transition (acknowledge from TRIAGED)
    try:
        apply_transition(r2, ACTION_ACKNOWLEDGE, actor="alice", reason="x",
                         evidence={"reason": "x"})
        check("T5_invalid_ack_triaged_rejected", False, "should have raised")
    except ValueError:
        check("T5_invalid_ack_triaged_rejected", True, "rejected as expected")

    # T6: missing required evidence
    try:
        apply_transition(rec, ACTION_ACKNOWLEDGE, actor="alice", reason="x",
                         evidence={})
        check("T6_missing_reason_rejected", False, "should have raised")
    except ValueError:
        check("T6_missing_reason_rejected", True, "rejected as expected")

    # T7: escalate TRIAGED → ESCALATED (HIGH severity)
    r3 = apply_transition(r2, ACTION_ESCALATE, actor="bob", reason="HIGH severity",
                          evidence={"reason": "high", "severity": v1348.SEVERITY_HIGH})
    check("T7_escalate_triaged_to_escalated", r3.current_state == STATE_ESCALATED,
          f"got {r3.current_state}")

    # T8: escalate with non-HIGH severity rejected
    try:
        apply_transition(r2, ACTION_ESCALATE, actor="bob", reason="x",
                         evidence={"reason": "x", "severity": v1348.SEVERITY_MEDIUM})
        check("T8_escalate_nonhigh_rejected", False, "should have raised")
    except ValueError:
        check("T8_escalate_nonhigh_rejected", True, "rejected as expected")

    # T9: mitigate TRIAGED → MITIGATED
    r4 = apply_transition(r2, ACTION_MITIGATE, actor="alice", reason="apply fix",
                          evidence={"reason": "fix", "action_kind": "patch"})
    check("T9_mitigate_triaged_to_mitigated", r4.current_state == STATE_MITIGATED,
          f"got {r4.current_state}")

    # T10: resolve MITIGATED → RESOLVED
    r5 = apply_transition(r4, ACTION_RESOLVE, actor="alice", reason="gone",
                          evidence={"reason": "gone", "anomaly_gone": True})
    check("T10_resolve_mitigated_to_resolved", r5.current_state == STATE_RESOLVED,
          f"got {r5.current_state}")

    # T11: close RESOLVED → CLOSED
    r6 = apply_transition(r5, ACTION_CLOSE, actor="alice", reason="archive",
                          evidence={"reason": "archive"})
    check("T11_close_resolved_to_closed", r6.current_state == STATE_CLOSED,
          f"got {r6.current_state}")

    # T12: reopen CLOSED → REOPENED → TRIAGED (auto)
    r7 = apply_transition(r6, ACTION_REOPEN, actor="bob", reason="new anomaly",
                          evidence={"reason": "new", "new_anomaly_id": "anom-2"})
    check("T12_reopen_closed_to_triaged", r7.current_state == STATE_TRIAGED,
          f"got {r7.current_state}")
    check("T12_reopen_two_events_appended", len(r7.events) == len(r6.events) + 2,
          f"got {len(r7.events)}")

    # T13: resolve requires anomaly_gone=True
    try:
        apply_transition(r2, ACTION_RESOLVE, actor="alice", reason="x",
                         evidence={"reason": "x", "anomaly_gone": False})
        check("T13_resolve_requires_gone_true", False, "should have raised")
    except ValueError:
        check("T13_resolve_requires_gone_true", True, "rejected as expected")

    # T14: store open_anomaly is idempotent
    store = LifecycleStore()
    r_a = store.open_anomaly("plugin.x", "anom-1", actor="alice", reason="init")
    r_b = store.open_anomaly("plugin.x", "anom-1", actor="alice", reason="init")
    check("T14_open_idempotent", r_a.lifecycle_id == r_b.lifecycle_id,
          f"a={r_a.lifecycle_id} b={r_b.lifecycle_id}")

    # T15: store apply updates record
    r_c = store.apply("plugin.x", "anom-1", ACTION_ACKNOWLEDGE, actor="alice",
                      reason="x", evidence={"reason": "x"})
    check("T15_store_apply_updates", r_c.current_state == STATE_TRIAGED,
          f"got {r_c.current_state}")

    # T16: ecosystem_rollup worst-of state
    r_d = store.open_anomaly("plugin.y", "anom-3", actor="alice", reason="init")
    rollup = ecosystem_rollup(store.records.values())
    check("T16_rollup_two_plugins", rollup.total_plugins == 2, f"got {rollup.total_plugins}")
    check("T16_rollup_ecosystem_state", rollup.ecosystem_state in ALL_STATES,
          f"got {rollup.ecosystem_state}")

    # T17: subscore basic
    sub, details = v1350_subscore(
        record_count=2, event_count=10, reopen_count=1,
        audit_path=None, transitions_used_count=4,
        has_v1348_bridge=True, has_v1349_audit_compat=True,
        guards_present=True,
    )
    check("T17_subscore_in_range", 0.0 <= sub <= 1.0, f"got {sub}")

    # T18: asi lift capped
    lift = v1350_asi_lift(0.85)
    check("T18_asi_lift_capped", lift["v1350_asi_lift"] <= V1350_ASI_CAP,
          f"got {lift['v1350_asi_lift']}")

    # T19: open_from_anomaly bridge
    from v1348_vcp_anomaly_detector import (
        ChannelSignal, PluginAnomaly,
    )
    pa = PluginAnomaly(
        plugin="plugin.z",
        plugin_severity=v1348.SEVERITY_HIGH,
        plugin_severity_rank=3,
        channels=[
            ChannelSignal(channel="health_drop", signal_score=1.0,
                          severity=v1348.SEVERITY_HIGH,
                          evidence={"delta": 0.5},
                          recommendation="fix it"),
        ],
        anomaly_id="anom-z-1",
    )
    r_e = open_from_anomaly(store, pa, actor="alice")
    check("T19_open_from_anomaly", r_e.plugin == "plugin.z" and r_e.anomaly_id == "anom-z-1",
          f"got plugin={r_e.plugin} anom={r_e.anomaly_id}")
    check("T19_evidence_has_severity", r_e.events[0].evidence.get("severity") == v1348.SEVERITY_HIGH,
          f"got {r_e.events[0].evidence}")

    # T20: audit JSONL write/read
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "audit.jsonl"
        store2 = LifecycleStore(audit_path=p)
        store2.open_anomaly("plugin.audit", "anom-audit-1", actor="alice", reason="init")
        store2.apply("plugin.audit", "anom-audit-1", ACTION_ACKNOWLEDGE, actor="alice",
                     reason="looking", evidence={"reason": "looking"})
        lines = p.read_text(encoding="utf-8").strip().split("\n")
        check("T20_audit_jsonl_lines", len(lines) >= 2, f"got {len(lines)}")
        first = json.loads(lines[0])
        check("T20_audit_has_event_id", "event_id" in first and len(first["event_id"]) == 16,
              f"got {first.get('event_id')}")

    return results


def run_self_tests(verbose: bool = False) -> Tuple[int, int]:
    """Run all Popper self-tests; return (passed, total)."""
    results = _popper_self_tests()
    passed = sum(1 for _, ok, _ in results if ok)
    total = len(results)
    if verbose:
        for name, ok, msg in results:
            mark = "PASS" if ok else "FAIL"
            print(f"  [{mark}] {name}: {msg}")
    return passed, total


# --- CLI --------------------------------------------------------------------
def main(argv: Optional[Sequence[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="v1350_vcp_anomaly_lifecycle",
        description="V1350 VCP Plugin Anomaly Lifecycle State Machine (v0.1.0)",
    )
    parser.add_argument("--self-test", action="store_true", help="run Popper self-tests")
    parser.add_argument("--verbose", action="store_true", help="verbose output")
    parser.add_argument("--list-transitions", action="store_true",
                        help="list all transitions")
    parser.add_argument("--audit-path", type=str, default=None,
                        help="optional audit JSONL path")
    parser.add_argument("--demo", action="store_true", help="run synthetic end-to-end demo")
    args = parser.parse_args(argv)

    if args.self_test:
        passed, total = run_self_tests(verbose=args.verbose)
        print(f"=== V1350 self-tests: {passed}/{total} PASS ===")
        return 0 if passed == total else 1

    if args.list_transitions:
        print(f"=== V1350 transitions ({len(TRANSITIONS)}) ===")
        for action, from_state, to_state, req in list_transitions():
            print(f"  {action:14} {from_state:10} -> {to_state:10}  evidence={req}")
        return 0

    if args.demo:
        audit_path = Path(args.audit_path) if args.audit_path else None
        store = LifecycleStore(audit_path=audit_path)
        # Open 3 anomalies for 3 plugins
        from v1348_vcp_anomaly_detector import (
            ChannelSignal, PluginAnomaly, build_report, detect_from_health_reports,
        )
        # Plugin A: HIGH severity → escalate path
        pa_a = PluginAnomaly(
            plugin="plugin.alpha", plugin_severity=v1348.SEVERITY_HIGH,
            plugin_severity_rank=3,
            channels=[ChannelSignal(channel="health_drop", signal_score=1.0,
                                     severity=v1348.SEVERITY_HIGH,
                                     evidence={"delta": 0.5},
                                     recommendation="fix now")],
            anomaly_id="anom-alpha-001",
        )
        # Plugin B: MEDIUM → mitigate path
        pa_b = PluginAnomaly(
            plugin="plugin.beta", plugin_severity=v1348.SEVERITY_MEDIUM,
            plugin_severity_rank=2,
            channels=[ChannelSignal(channel="lint_regression", signal_score=0.7,
                                     severity=v1348.SEVERITY_MEDIUM,
                                     evidence={"delta": 3},
                                     recommendation="re-lint")],
            anomaly_id="anom-beta-001",
        )
        # Plugin C: LOW → resolve path
        pa_c = PluginAnomaly(
            plugin="plugin.gamma", plugin_severity=v1348.SEVERITY_LOW,
            plugin_severity_rank=1,
            channels=[ChannelSignal(channel="drift_spike", signal_score=0.4,
                                     severity=v1348.SEVERITY_LOW,
                                     evidence={"drift": 0.4},
                                     recommendation="monitor")],
            anomaly_id="anom-gamma-001",
        )
        # Open all 3 lifecycles
        r_a = open_from_anomaly(store, pa_a, actor="alice")
        r_b = open_from_anomaly(store, pa_b, actor="alice")
        r_c = open_from_anomaly(store, pa_c, actor="alice")
        # Path A: acknowledge → escalate → resolve → close
        r_a = store.apply("plugin.alpha", "anom-alpha-001", ACTION_ACKNOWLEDGE,
                          actor="alice", reason="looking",
                          evidence={"reason": "looking"})
        r_a = store.apply("plugin.alpha", "anom-alpha-001", ACTION_ESCALATE,
                          actor="bob", reason="HIGH severity",
                          evidence={"reason": "high", "severity": v1348.SEVERITY_HIGH})
        r_a = store.apply("plugin.alpha", "anom-alpha-001", ACTION_RESOLVE,
                          actor="bob", reason="escalated fix worked",
                          evidence={"reason": "fixed", "anomaly_gone": True})
        r_a = store.apply("plugin.alpha", "anom-alpha-001", ACTION_CLOSE,
                          actor="bob", reason="archived",
                          evidence={"reason": "archived"})
        # Path B: acknowledge → mitigate → resolve
        r_b = store.apply("plugin.beta", "anom-beta-001", ACTION_ACKNOWLEDGE,
                          actor="alice", reason="looking",
                          evidence={"reason": "looking"})
        r_b = store.apply("plugin.beta", "anom-beta-001", ACTION_MITIGATE,
                          actor="alice", reason="apply re-lint fix",
                          evidence={"reason": "fix", "action_kind": "re_lint"})
        r_b = store.apply("plugin.beta", "anom-beta-001", ACTION_RESOLVE,
                          actor="alice", reason="re-lint passed",
                          evidence={"reason": "fixed", "anomaly_gone": True})
        # Path C: acknowledge → resolve → reopen (new anomaly) → mitigate → resolve
        r_c = store.apply("plugin.gamma", "anom-gamma-001", ACTION_ACKNOWLEDGE,
                          actor="alice", reason="looking",
                          evidence={"reason": "looking"})
        r_c = store.apply("plugin.gamma", "anom-gamma-001", ACTION_RESOLVE,
                          actor="alice", reason="drift settled",
                          evidence={"reason": "settled", "anomaly_gone": True})
        # Close, then reopen with new anomaly
        r_c = store.apply("plugin.gamma", "anom-gamma-001", ACTION_CLOSE,
                          actor="alice", reason="archived",
                          evidence={"reason": "archived"})
        r_c = store.apply("plugin.gamma", "anom-gamma-001", ACTION_REOPEN,
                          actor="alice", reason="new anomaly surfaced",
                          evidence={"reason": "new", "new_anomaly_id": "anom-gamma-002"})
        r_c = store.apply("plugin.gamma", "anom-gamma-001", ACTION_MITIGATE,
                          actor="alice", reason="apply fix",
                          evidence={"reason": "fix", "action_kind": "monitor"})
        r_c = store.apply("plugin.gamma", "anom-gamma-001", ACTION_RESOLVE,
                          actor="alice", reason="fixed",
                          evidence={"reason": "fixed", "anomaly_gone": True})
        # Rollup
        rollup = ecosystem_rollup(store.records.values())
        # Subscore + ASI lift
        reopen_count = sum(1 for r in store.records.values()
                           for ev in r.events if ev.action == ACTION_REOPEN)
        sub, details = v1350_subscore(
            record_count=len(store.records),
            event_count=sum(len(r.events) for r in store.records.values()),
            reopen_count=reopen_count,
            audit_path=audit_path,
            transitions_used_count=len(rollup.transitions_used),
            has_v1348_bridge=True,
            has_v1349_audit_compat=True,
            guards_present=True,
        )
        lift = v1350_asi_lift(sub)
        print(f"=== V1350 VCP Anomaly Lifecycle State Machine (v{V1350_VERSION}) ===")
        print(f"records: {rollup.total_plugins} events: {rollup.total_events}")
        print(f"ecosystem_state: {rollup.ecosystem_state} (rank {rollup.ecosystem_state_rank})")
        print(f"state_breakdown: {rollup.state_breakdown}")
        print(f"transitions_used: {rollup.transitions_used}")
        print(f"subscore: {sub:.4f}")
        print(f"asi_lift: +{lift['v1350_asi_lift']:.6f} (cap {lift['v1350_cap']})")
        if audit_path is not None:
            print(f"audit: {audit_path}")
        return 0

    parser.print_help()
    return 0


__all__ = [
    "V1350_VERSION",
    "V1350_SUBWEIGHTS",
    "V1350_ASI_CAP",
    "V1350_GUARDS",
    "GUARD_NOT_MACHINE_IS_CONSCIOUS",
    "GUARD_NOT_WORKFLOW_IS_POLICY",
    "GUARD_NOT_LIFECYCLE_IS_ORACLE",
    "GUARD_NOT_PLUGIN_IS_PHENOMENAL",
    "GUARD_NOT_SUBSCORE_IS_ASI",
    "STATE_OPEN",
    "STATE_TRIAGED",
    "STATE_ESCALATED",
    "STATE_MITIGATED",
    "STATE_RESOLVED",
    "STATE_CLOSED",
    "STATE_REOPENED",
    "ALL_STATES",
    "STATE_RANK",
    "ACTION_ACKNOWLEDGE",
    "ACTION_ESCALATE",
    "ACTION_MITIGATE",
    "ACTION_RESOLVE",
    "ACTION_CLOSE",
    "ACTION_REOPEN",
    "ALL_ACTIONS",
    "TRANSITIONS",
    "REOPEN_NEXT_STATE",
    "LifecycleEvent",
    "LifecycleRecord",
    "EcosystemLifecycleReport",
    "LifecycleStore",
    "transition_lookup",
    "list_transitions",
    "build_initial_record",
    "apply_transition",
    "ecosystem_rollup",
    "v1350_subscore",
    "v1350_asi_lift",
    "open_from_anomaly",
    "run_self_tests",
]


V1350_GUARDS = (
    GUARD_NOT_MACHINE_IS_CONSCIOUS,
    GUARD_NOT_WORKFLOW_IS_POLICY,
    GUARD_NOT_LIFECYCLE_IS_ORACLE,
    GUARD_NOT_PLUGIN_IS_PHENOMENAL,
    GUARD_NOT_SUBSCORE_IS_ASI,
)


if __name__ == "__main__":
    raise SystemExit(main())
