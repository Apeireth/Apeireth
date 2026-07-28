"""MemoryReplay design sketch + Protocol (R6-RES-07 / R7-BE-02 幂等重放准备).

Design only. No real backend. Stubs raise NotImplementedError so tests
verify protocol shape, not behavior.

守门 (主23:44 + V3 + V1081):
- replay ≠ bit-exact: approximate reconstruction, never byte-equal
- idempotent ≠ safe: holds only for whitelisted op set
- capture ≠ backup: state snapshot for diff/restore, not history
- replay ≠ understanding: heuristic re-emission, not phenomenal recall
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Iterator, Protocol, runtime_checkable


# ---------- Dataclasses ---------------------------------------------------


@dataclass(frozen=True)
class StateID:
    """Opaque: scope + monotonic seq + content sha256[:8]."""
    scope: str
    seq: int
    content_hash: str = ""


@dataclass(frozen=True)
class Event:
    """One replayable event; ts unix-seconds; payload opaque json-safe."""
    event_id: str
    ts: float
    kind: str
    payload: tuple[tuple[str, Any], ...] = ()


@dataclass(frozen=True)
class StateDiff:
    """diff_states(a,b); symmetric in (a,b) up to sign."""
    added: tuple[Event, ...] = ()
    removed: tuple[Event, ...] = ()
    changed: tuple[tuple[Event, Event], ...] = ()


@dataclass(frozen=True)
class ApplyResult:
    """idempotent_apply outcome; same event twice → same status+hash."""
    status: str            # "applied"|"cached"|"rejected"|"failed"
    event_hash: str
    cached: bool = False
    reason: str = ""


# ---------- Philosophy guards --------------------------------------------


REPLAY_NOT_BIT_EXACT = "replay is approximate; never claim byte-equality"
IDEMPOTENT_NOT_SAFE = "idempotency holds only for whitelisted ops"
CAPTURE_NOT_BACKUP = "capture is snapshot for diff/restore, not history"
REPLAY_NOT_UNDERSTANDING = "heuristic re-emission; not phenomenal recall"

PHILOSOPHY_GUARDS: tuple[str, ...] = (
    REPLAY_NOT_BIT_EXACT, IDEMPOTENT_NOT_SAFE,
    CAPTURE_NOT_BACKUP, REPLAY_NOT_UNDERSTANDING,
)


# ---------- Idempotency whitelist ---------------------------------------


# Ops eligible for idempotent_apply. Outside this set must reject loudly
# so callers don't treat replay as a generic write path.
IDEMPOTENT_OPS: frozenset[str] = frozenset({
    "tag_set", "anchor_link", "anchor_unlink",
    "score_record", "phase_emit", "trace_record",
})


# ---------- Protocol -----------------------------------------------------


@runtime_checkable
class MemoryReplayProtocol(Protocol):
    """R7-BE-02 memory replay contract. Five required methods."""

    def capture_state(self, scope: str) -> StateID: ...
    def restore_state(self, state_id: StateID) -> bool: ...
    def replay_events(self, from_ts: float, to_ts: float) -> Iterator[Event]: ...
    def diff_states(self, state_a: StateID, state_b: StateID) -> StateDiff: ...
    def idempotent_apply(self, event: Event) -> ApplyResult:
        """Apply iff kind ∈ IDEMPOTENT_OPS; else rejected. Same event twice
        yields identical status+event_hash with cached=True on 2nd call."""
        ...


# ---------- Reference stub (for protocol-shape tests) --------------------


class InMemoryReplayStub:
    """Minimal stub: tracks applied events + last captures."""

    def __init__(self) -> None:
        self._applied: dict[str, ApplyResult] = {}
        self._events: list[Event] = []
        self._seq: int = 0

    def capture_state(self, scope: str) -> StateID:
        self._seq += 1
        return StateID(scope=scope, seq=self._seq, content_hash=str(len(self._events)))

    def restore_state(self, state_id: StateID) -> bool:
        return bool(state_id.content_hash)

    def replay_events(self, from_ts: float, to_ts: float) -> Iterator[Event]:
        return iter(ev for ev in self._events if from_ts < ev.ts <= to_ts)

    def diff_states(self, state_a: StateID, state_b: StateID) -> StateDiff:
        return StateDiff()  # stub: no real diff

    def idempotent_apply(self, event: Event) -> ApplyResult:
        if event.kind not in IDEMPOTENT_OPS:
            return ApplyResult(status="rejected", event_hash=event.event_id,
                                reason=f"op {event.kind!r} not whitelisted")
        prev = self._applied.get(event.event_id)
        if prev is not None:
            return ApplyResult(status=prev.status, event_hash=prev.event_hash,
                                cached=True)
        result = ApplyResult(status="applied", event_hash=event.event_id)
        self._applied[event.event_id] = result
        self._events.append(event)
        return result


__all__ = [
    "StateID", "Event", "StateDiff", "ApplyResult",
    "MemoryReplayProtocol", "InMemoryReplayStub", "IDEMPOTENT_OPS",
    "PHILOSOPHY_GUARDS",
    "REPLAY_NOT_BIT_EXACT", "IDEMPOTENT_NOT_SAFE",
    "CAPTURE_NOT_BACKUP", "REPLAY_NOT_UNDERSTANDING",
]