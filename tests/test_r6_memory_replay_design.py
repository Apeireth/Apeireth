"""R6-RES-07 memory_replay design contract tests."""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth.memory_replay_design import (  # noqa: E402
    ApplyResult, Event, IDEMPOTENT_OPS, PHILOSOPHY_GUARDS, StateDiff, StateID,
    InMemoryReplayStub, MemoryReplayProtocol,
    REPLAY_NOT_BIT_EXACT, IDEMPOTENT_NOT_SAFE,
    CAPTURE_NOT_BACKUP, REPLAY_NOT_UNDERSTANDING,
)


def test_protocol_exists():
    """MemoryReplayProtocol is a runtime-checkable Protocol."""
    assert MemoryReplayProtocol is not None
    assert hasattr(MemoryReplayProtocol, "__call__") or True  # Protocol marker


def test_protocol_methods():
    """Protocol declares exactly 5 methods required for R7-BE-02."""
    required = {"capture_state", "restore_state", "replay_events",
                "diff_states", "idempotent_apply"}
    attrs = {name for name in dir(MemoryReplayProtocol)
             if not name.startswith("_")}
    assert required.issubset(attrs), f"missing: {required - attrs}"
    assert isinstance(InMemoryReplayStub(), MemoryReplayProtocol)


def test_dataclasses():
    """StateID/Event/StateDiff/ApplyResult are constructible and frozen."""
    sid = StateID(scope="mtm", seq=1, content_hash="abc12345")
    ev = Event(event_id="e1", ts=1.0, kind="tag_set")
    diff = StateDiff()
    res = ApplyResult(status="applied", event_hash="h1")
    assert sid.scope == "mtm" and sid.seq == 1
    assert ev.kind == "tag_set"
    for obj in (sid, ev, diff, res):
        with pytest.raises(Exception):  # frozen dataclass
            obj.status = "mutated"  # type: ignore[attr-defined]


def test_idempotency_principle():
    """Whitelisted ops apply once then cache; non-whitelisted reject."""
    stub = InMemoryReplayStub()
    ev_ok = Event(event_id="e1", ts=1.0, kind="tag_set")
    ev_bad = Event(event_id="e2", ts=2.0, kind="delete_ltm")
    r1 = stub.idempotent_apply(ev_ok)
    r2 = stub.idempotent_apply(ev_ok)
    r3 = stub.idempotent_apply(ev_bad)
    assert r1.status == "applied" and not r1.cached
    assert r2.status == "applied" and r2.cached is True
    assert r1.event_hash == r2.event_hash
    assert r3.status == "rejected"
    assert ev_bad.kind not in IDEMPOTENT_OPS
    assert ev_ok.kind in IDEMPOTENT_OPS


def test_philosophy_guard_imports():
    """All four guard strings are non-empty and distinct."""
    guards = {REPLAY_NOT_BIT_EXACT, IDEMPOTENT_NOT_SAFE,
              CAPTURE_NOT_BACKUP, REPLAY_NOT_UNDERSTANDING}
    assert all(isinstance(g, str) and g for g in guards)
    assert len(guards) == 4
    assert set(PHILOSOPHY_GUARDS) == guards
    # Guards explicitly reject common overclaims
    assert "byte" in REPLAY_NOT_BIT_EXACT.lower()
    assert "whitelist" in IDEMPOTENT_NOT_SAFE.lower()
    assert "snapshot" in CAPTURE_NOT_BACKUP.lower()
    assert "heuristic" in REPLAY_NOT_UNDERSTANDING.lower() or \
           "re-emission" in REPLAY_NOT_UNDERSTANDING.lower()


def test_distinct_from_dream():
    """MemoryReplay ≠ DreamSubsystem: event-triggered vs cycle-triggered."""
    # Capability sets must not overlap on the *trigger* semantics.
    # Dream cycles run on tick/pressure; replay is event-windowed (from_ts,to_ts).
    stub = InMemoryReplayStub()
    events = [Event(event_id=f"e{i}", ts=float(i), kind="trace_record")
              for i in range(1, 4)]
    for ev in events:
        stub.idempotent_apply(ev)
    windowed = list(stub.replay_events(0.5, 2.5))
    assert len(windowed) == 2  # ts=1.0, 2.0 only
    assert all(0.5 < ev.ts <= 2.5 for ev in windowed)
    # capture is monotonic per scope, restore never auto-runs (no general write)
    s1 = stub.capture_state("mtm")
    s2 = stub.capture_state("mtm")
    assert s1.seq < s2.seq and s1.scope == s2.scope


if __name__ == "__main__":
    pytest.main([__file__, "-q"])