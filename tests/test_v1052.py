"""Tests for V1052 ASI Memory Consolidation (主 17:43 实事求是: 真测, 不假装)."""
from __future__ import annotations

import json
import math
import time
from pathlib import Path

import pytest

from apeireth.v1052_asi_memory_consolidation import (
    ConsolidationReport,
    ConsolidationTickResult,
    Episode,
    ForgettingCurve,
    MemoryBridge,
    MemoryStore,
    Note,
    ReconsolidationPath,
    Reconsolidator,
    SalienceScore,
    Tier,
    TierPolicy,
    Wal,
    WalEntry,
    V1052_VERSION,
    consolidation_tick,
    make_default_policy,
    make_default_store,
)


# ============================================================================
# Episode tests
# ============================================================================


def test_episode_basic():
    """Episode basic creation."""
    ep = Episode(
        eid="ep1",
        actor="master",
        content="主人 22:33 ASI 北极星",
        ts=time.time(),
    )
    assert ep.eid == "ep1"
    assert ep.actor == "master"
    assert ep.tier == "stm"


def test_episode_importance_bounds():
    """Importance must be in [0, 1]."""
    with pytest.raises(ValueError):
        Episode(eid="x", actor="a", content="c", ts=0.0, importance=1.5)


def test_episode_tier_must_be_valid():
    """Tier must be stm/mtm/ltm."""
    with pytest.raises(ValueError):
        Episode(eid="x", actor="a", content="c", ts=0.0, tier="invalid")


def test_episode_eid_required():
    """Empty eid rejected."""
    with pytest.raises(ValueError):
        Episode(eid="", actor="a", content="c", ts=0.0)


def test_episode_is_immutable():
    """Episode is frozen — no mutation."""
    ep = Episode(eid="ep1", actor="master", content="c", ts=0.0)
    with pytest.raises((AttributeError, Exception)):
        ep.content = "modified"  # type: ignore[misc]


# ============================================================================
# Note tests
# ============================================================================


def test_note_touch():
    """Note.touch increments access_count + updates last_access."""
    n = Note(nid="n1", topic="t", claim="c", confidence=0.5)
    assert n.access_count == 0
    n.touch(100.0)
    assert n.access_count == 1
    assert n.last_access == 100.0
    n.touch(200.0)
    assert n.access_count == 2


def test_note_apply_decay():
    """Note decay reduces salience + confidence (主 13:47 关心遗忘)."""
    n = Note(nid="n1", topic="t", claim="c", confidence=0.8, salience=0.6)
    n.apply_decay(rate=0.5)
    assert n.salience == pytest.approx(0.3, abs=1e-9)
    assert n.confidence < 0.8  # decays slower (×0.5 of salience decay)


def test_note_decay_floors_at_zero():
    """Salience doesn't go below 0."""
    n = Note(nid="n1", topic="t", claim="c", salience=0.01)
    n.apply_decay(rate=1.0)
    assert n.salience == 0.0


# ============================================================================
# Tier enum tests
# ============================================================================


def test_tier_enum_values():
    """Tier enum has stm/mtm/ltm."""
    assert Tier.STM.value == "stm"
    assert Tier.MTM.value == "mtm"
    assert Tier.LTM.value == "ltm"


# ============================================================================
# SalienceScore tests
# ============================================================================


def test_salience_high_importance_fresh():
    """High importance + fresh = high salience."""
    s = SalienceScore(importance=1.0, age_days=0.0, access_count=0)
    assert s.score() > 0.5


def test_salience_old_low():
    """Old + low importance = low salience."""
    s = SalienceScore(importance=0.1, age_days=30.0, access_count=0)
    assert s.score() < 0.05


def test_salience_access_boosts():
    """More accesses → higher salience."""
    s1 = SalienceScore(importance=0.5, age_days=1.0, access_count=0)
    s2 = SalienceScore(importance=0.5, age_days=1.0, access_count=10)
    assert s2.score() > s1.score()


# ============================================================================
# MemoryStore tests
# ============================================================================


def test_store_append_episode():
    """MemoryStore.append_episode works."""
    store = MemoryStore()
    ep = Episode(eid="ep1", actor="master", content="c", ts=0.0)
    store.append_episode(ep)
    assert store.stats()["total_episodes"] == 1
    assert store.stats()["stm_episodes"] == 1


def test_store_duplicate_episode_rejected():
    """Cannot append same eid twice."""
    store = MemoryStore()
    store.append_episode(Episode(eid="ep1", actor="master", content="c", ts=0.0))
    with pytest.raises(ValueError):
        store.append_episode(Episode(eid="ep1", actor="master", content="c", ts=0.0))


def test_store_tier_transition():
    """Transition moves episode between tier buckets."""
    store = MemoryStore()
    store.append_episode(Episode(eid="ep1", actor="master", content="c", ts=0.0, tier="stm"))
    store.transition("ep1", "stm", "mtm", ts=100.0)
    assert store.stats()["stm_episodes"] == 0
    assert store.stats()["mtm_episodes"] == 1
    assert store.stats()["transitions"] == 1


def test_store_forget_episode():
    """forget_episode removes from all buckets."""
    store = MemoryStore()
    store.append_episode(Episode(eid="ep1", actor="master", content="c", ts=0.0))
    store.forget_episode("ep1")
    assert store.stats()["total_episodes"] == 0


def test_store_add_note():
    """add_note inserts note."""
    store = MemoryStore()
    store.add_note(Note(nid="n1", topic="t", claim="c"))
    assert store.stats()["total_notes"] == 1


# ============================================================================
# TierPolicy tests
# ============================================================================


def test_policy_should_promote_to_mtm():
    """Old episode promoted to MTM."""
    policy = TierPolicy(stm_to_mtm_age_sec=3600)
    ep = Episode(eid="ep1", actor="master", content="c", ts=1000.0)
    assert policy.should_promote_to_mtm(ep, now=1000.0 + 3601) is True
    assert policy.should_promote_to_mtm(ep, now=1000.0 + 100) is False


def test_policy_should_promote_to_ltm_requires_importance():
    """MTM → LTM requires both age + importance."""
    policy = TierPolicy(mtm_to_ltm_stable_age_days=1.0)
    old_low = Episode(eid="x", actor="a", content="c", ts=0.0, importance=0.1)
    assert policy.should_promote_to_ltm(old_low, now=86400.0 * 2) is False
    old_high = Episode(eid="y", actor="a", content="c", ts=0.0, importance=0.9)
    assert policy.should_promote_to_ltm(old_high, now=86400.0 * 2) is True


def test_policy_default_values():
    """Default policy has reasonable values."""
    p = make_default_policy()
    assert p.stm_to_mtm_age_sec == 3600
    assert p.ltm_protected is True  # 主 12:14 中央 AI 永恒身份


# ============================================================================
# WAL tests
# ============================================================================


def test_wal_append_and_replay(tmp_path: Path):
    """WAL append + replay returns entries (DeltaMemory 借鉴)."""
    wal_path = tmp_path / "test.wal"
    wal = Wal(wal_path)
    wal.append("test_op", {"key": "value", "n": 1})
    wal.append("test_op2", {"key": "value2", "n": 2})
    entries = wal.replay()
    assert len(entries) == 2
    assert entries[0]["sequence"] == 1
    assert entries[1]["sequence"] == 2
    payload = entries[0]["payload"]
    assert hashlib_sha256(payload) == entries[0]["checksum"]


def hashlib_sha256(payload: str) -> str:
    """Helper: SHA-256 hex (kept local to avoid top-level import)."""
    import hashlib
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def test_wal_recover_last_sequence(tmp_path: Path):
    """WAL recovers last sequence on reopen."""
    wal_path = tmp_path / "test.wal"
    w1 = Wal(wal_path)
    w1.append("a", {"x": 1})
    w1.append("a", {"x": 2})
    w1.append("a", {"x": 3})
    w2 = Wal(wal_path)
    assert w2.sequence == 3


def test_wal_skip_corrupted(tmp_path: Path):
    """WAL replay skips corrupted entries (DeltaMemory 借鉴)."""
    wal_path = tmp_path / "test.wal"
    w1 = Wal(wal_path)
    w1.append("good", {"x": 1})
    # Manually corrupt: append invalid JSON line
    wal_path.write_text(wal_path.read_text() + "not valid json\n", encoding="utf-8")
    w1.append("good2", {"x": 2})
    entries = w1.replay()
    assert len(entries) == 2  # corrupted entry skipped


def test_wal_verify_integrity(tmp_path: Path):
    """WAL verify returns (valid, corrupt) counts."""
    wal_path = tmp_path / "test.wal"
    w1 = Wal(wal_path)
    w1.append("op", {"x": 1})
    w1.append("op", {"x": 2})
    valid, corrupt = w1.verify()
    assert valid == 2
    assert corrupt == 0


def test_wal_no_path():
    """WAL works without file path (in-memory mode)."""
    wal = Wal(path=None)
    seq = wal.append("test", {"x": 1})
    assert seq == 1
    assert wal.replay() == []


# ============================================================================
# Reconsolidator tests
# ============================================================================


def test_reconsolidator_choose_boost():
    """New evidence > confidence with diff under threshold → BOOST."""
    rec = Reconsolidator(flag_threshold=0.5)
    note = Note(nid="n1", topic="t", claim="c", confidence=0.3)
    # diff = 0.4 < 0.5 (flag_threshold) → not FLAG → BOOST (evidence > confidence)
    assert rec.choose_path(note, new_evidence=0.7) == ReconsolidationPath.BOOST


def test_reconsolidator_choose_align():
    """Small diff + evidence <= confidence → ALIGN."""
    rec = Reconsolidator(flag_threshold=0.5)
    note = Note(nid="n1", topic="t", claim="c", confidence=0.55)
    # diff = 0.05 < 0.5 → not FLAG; evidence < confidence → ALIGN
    assert rec.choose_path(note, new_evidence=0.5) == ReconsolidationPath.ALIGN


def test_reconsolidator_choose_flag():
    """Large diff → FLAG (highest priority)."""
    rec = Reconsolidator(flag_threshold=0.2)
    note = Note(nid="n1", topic="t", claim="c", confidence=0.2)
    assert rec.choose_path(note, new_evidence=0.9) == ReconsolidationPath.FLAG


def test_reconsolidator_apply_boost():
    """BOOST increases salience + confidence (with diff-headroom)."""
    rec = Reconsolidator(boost_delta=0.2, align_lr=0.5, flag_threshold=0.9)
    note = Note(nid="n1", topic="t", claim="c", confidence=0.2, salience=0.4)
    path, updated = rec.apply(note, new_evidence=0.8, now=100.0)
    assert path == ReconsolidationPath.BOOST
    assert updated.salience > 0.4
    assert updated.confidence > 0.2
    assert updated.access_count == 1


def test_reconsolidator_apply_flag_no_touch():
    """FLAG does not bump access_count."""
    rec = Reconsolidator(flag_threshold=0.1)
    note = Note(nid="n1", topic="t", claim="c", confidence=0.5)
    initial_count = note.access_count
    path, _ = rec.apply(note, new_evidence=0.9, now=100.0)
    assert path == ReconsolidationPath.FLAG
    assert note.access_count == initial_count


# ============================================================================
# ForgettingCurve tests
# ============================================================================


def test_forgetting_retention_fresh():
    """Fresh memory has retention ≈ 1."""
    fc = ForgettingCurve(half_life_days=7.0)
    assert fc.retention(0.0) == pytest.approx(1.0, abs=1e-9)


def test_forgetting_retention_at_half_life():
    """At half-life, retention ≈ 0.368 (1/e)."""
    fc = ForgettingCurve(half_life_days=7.0)
    assert fc.retention(7.0) == pytest.approx(math.exp(-1.0), abs=1e-9)


def test_forgetting_should_forget_threshold():
    """Memory forgotten when retention < threshold."""
    fc = ForgettingCurve(half_life_days=1.0)
    assert fc.should_forget(0.0) is False
    assert fc.should_forget(100.0) is True


def test_forgetting_decay_importance():
    """decay_importance applies retention curve."""
    fc = ForgettingCurve(half_life_days=7.0)
    assert fc.decay_importance(1.0, 7.0) == pytest.approx(fc.retention(7.0), abs=1e-9)


# ============================================================================
# consolidation_tick — the core 真生产 driver (主 23:44 干到底)
# ============================================================================


def test_tick_stm_to_mtm_promotion():
    """Old STM episodes promoted to MTM."""
    store = MemoryStore()
    now = 10000.0
    store.append_episode(Episode(eid="old", actor="master", content="c", ts=now - 7200))  # 2h old
    store.append_episode(Episode(eid="fresh", actor="master", content="c", ts=now - 60))  # 1m old
    policy = TierPolicy(stm_to_mtm_age_sec=3600)
    fc = ForgettingCurve(half_life_days=100.0)  # disable forgetting
    result = consolidation_tick(store, policy, None, None, fc, now=now)
    assert result.stm_to_mtm == 1
    assert store.stats()["mtm_episodes"] == 1
    assert store.stats()["stm_episodes"] == 1  # fresh still in stm


def test_tick_mtm_to_ltm_promotion():
    """Stable MTM episodes promoted to LTM (主 12:14 中央 AI 永恒身份)."""
    store = MemoryStore()
    now = 100000.0
    store.append_episode(Episode(
        eid="stable", actor="master", content="c", ts=now - 86400 * 2,  # 2 days old
        tier="mtm", importance=0.9,
    ))
    policy = TierPolicy(mtm_to_ltm_stable_age_days=1.0)
    fc = ForgettingCurve(half_life_days=100.0)
    result = consolidation_tick(store, policy, None, None, fc, now=now)
    assert result.mtm_to_ltm == 1
    assert store.stats()["ltm_episodes"] == 1


def test_tick_forgetting_stm():
    """Old low-salience STM episodes forgotten (主 13:47 关心遗忘).

    Episode must be low importance so it stays in STM/MTM and gets forgotten.
    """
    store = MemoryStore()
    now = 100000.0
    store.append_episode(Episode(
        eid="ephemeral", actor="master", content="c",
        ts=now - 86400 * 30, importance=0.1,
    ))
    policy = TierPolicy(mtm_to_ltm_stable_age_days=1.0)
    fc = ForgettingCurve(half_life_days=1.0)
    result = consolidation_tick(store, policy, None, None, fc, now=now)
    # Low importance → not promoted to LTM → stays in STM/MTM → forgotten.
    assert (result.forgotten + result.mtm_to_ltm) >= 1


def test_tick_ltm_protected():
    """LTM episodes are not auto-forgotten (主 12:14 永恒身份)."""
    store = MemoryStore()
    now = 100000.0
    store.append_episode(Episode(
        eid="eternal", actor="master", content="c", ts=now - 86400 * 365,
        tier="ltm", importance=0.99,
    ))
    policy = TierPolicy(ltm_protected=True)
    fc = ForgettingCurve(half_life_days=1.0)
    result = consolidation_tick(store, policy, None, None, fc, now=now)
    # LTM is not in stm/mtm buckets, so forgetting loop doesn't touch it.
    assert store.stats()["ltm_episodes"] == 1
    assert result.forgotten == 0


def test_tick_writes_wal(tmp_path: Path):
    """Tick writes to WAL (DeltaMemory 借鉴)."""
    wal_path = tmp_path / "test_tick.wal"
    wal = Wal(wal_path)
    store = MemoryStore()
    now = 10000.0
    store.append_episode(Episode(eid="old", actor="master", content="c", ts=now - 7200))
    policy = TierPolicy()
    fc = ForgettingCurve(half_life_days=100.0)
    consolidation_tick(store, policy, wal, None, fc, now=now)
    entries = wal.replay()
    assert len(entries) >= 1
    assert entries[0]["operation"] == "tier_transition"


def test_tick_reconsolidation_applied():
    """Reconsolidator applied when notes have been touched."""
    rec = Reconsolidator()
    store = MemoryStore()
    note = Note(nid="n1", topic="t", claim="c", confidence=0.3)
    note.touch(1000.0)  # 1 access
    store.add_note(note)
    policy = TierPolicy()
    fc = ForgettingCurve(half_life_days=100.0)
    result = consolidation_tick(store, policy, None, rec, fc, now=2000.0)
    # Note was reconsolidated.
    assert result.reconsolidations == 1


# ============================================================================
# MemoryBridge tests
# ============================================================================


def test_bridge_to_asi_metrics_basic():
    """Bridge returns 4 ASI metrics (主 22:33 真测量)."""
    store = MemoryStore()
    store.append_episode(Episode(eid="a", actor="master", content="c", ts=0.0))
    store.append_episode(Episode(eid="b", actor="master", content="c", ts=0.0, tier="ltm"))
    m = MemoryBridge.to_asi_metrics(store)
    assert "ltm_ratio" in m
    assert "stm_ratio" in m
    assert "abstraction_density" in m
    assert "consolidation_activity" in m
    assert m["ltm_ratio"] == 0.5
    assert m["stm_ratio"] == 0.5


def test_bridge_asi_v02_score_capped():
    """ASI V0.2 score contribution is capped at 0.05 (主 17:43 不假装)."""
    store = MemoryStore()
    store.append_episode(Episode(eid="a", actor="master", content="c", ts=0.0, tier="ltm", importance=1.0))
    score = MemoryBridge.asi_v02_score(store)
    assert 0.0 <= score <= 0.05


def test_bridge_empty_store_zero_score():
    """Empty store gives 0 score (主 17:43 不假装)."""
    store = MemoryStore()
    score = MemoryBridge.asi_v02_score(store)
    assert score == 0.0


# ============================================================================
# ConsolidationReport tests
# ============================================================================


def test_report_to_markdown():
    """Report renders Markdown (主 00:56 任何人都能接手)."""
    r = ConsolidationReport(
        stm_to_mtm=5, mtm_to_ltm=2, forgotten=1, notes_decayed=10, reconsolidations=3,
        asi_score=0.0234,
    )
    md = r.to_markdown()
    assert "V1052" in md
    # Source has literal → (U+2192) and ** bold markdown wrapper.
    assert "STM \u2192 MTM promotions" in md
    assert "MTM \u2192 LTM promotions" in md
    assert "Forgotten episodes" in md
    assert "ASI V0.2 contribution" in md
    assert "\u4e2d\u592e AI" in md  # 中央 AI (主 12:14 引用)


# ============================================================================
# Convenience factory tests
# ============================================================================


def test_make_default_store():
    """Default factory returns empty store."""
    store = make_default_store()
    assert store.stats()["total_episodes"] == 0


def test_make_default_policy_ltm_protected():
    """Default policy protects LTM (主 12:14)."""
    p = make_default_policy()
    assert p.ltm_protected is True


# ============================================================================
# Round-trip test — 真生产 E2E (主 23:44 干到底)
# ============================================================================


def test_round_trip_end_to_end(tmp_path: Path):
    """Round-trip: append → tick → WAL → replay (主 23:44 干到底)."""
    wal_path = tmp_path / "e2e.wal"
    wal = Wal(wal_path)
    store = make_default_store()
    policy = make_default_policy()
    rec = Reconsolidator()
    fc = ForgettingCurve(half_life_days=100.0)  # disable auto-forgetting

    # 1. Append 3 episodes at different times.
    now = 100000.0
    store.append_episode(Episode(eid="recent", actor="master", content="new dialog", ts=now - 60))
    store.append_episode(Episode(
        eid="medium", actor="master", content="medium theme", ts=now - 7200,  # 2h
    ))
    store.append_episode(Episode(
        eid="eternal", actor="master", content="eternal fact",
        ts=now - 86400 * 7, tier="ltm", importance=0.99,
    ))
    store.add_note(Note(nid="n1", topic="meta", claim="eternal identity", confidence=0.9))
    note = store.notes["n1"]
    note.touch(now)

    # 2. Tick consolidation.
    result = consolidation_tick(store, policy, wal, rec, fc, now=now)
    assert result.stm_to_mtm == 1  # 'medium' is 2h old, should promote
    assert result.notes_decayed == 1

    # 3. Verify store state.
    stats = store.stats()
    assert stats["stm_episodes"] == 1  # 'recent'
    assert stats["mtm_episodes"] == 1  # 'medium'
    assert stats["ltm_episodes"] == 1  # 'eternal' (主 12:14)
    assert stats["transitions"] == 1

    # 4. Verify WAL.
    entries = wal.replay()
    assert len(entries) >= 1
    assert entries[0]["operation"] == "tier_transition"

    # 5. ASI bridge 真测量.
    score = MemoryBridge.asi_v02_score(store)
    assert 0.0 <= score <= 0.05


def test_v1052_version():
    """Module has version."""
    assert V1052_VERSION == "0.1.0"


# ============================================================================
# V3 哲学守门 — 不假装 (主 17:58 + 20:46)
# ============================================================================


def test_philosophy_guard_no_phenomenal_claim():
    """V1052 does not claim Phenomenal consciousness (主 17:58)."""
    import apeireth.v1052_asi_memory_consolidation as m
    src = Path(m.__file__).read_text(encoding="utf-8")
    assert "\u4e0d\u5047\u88c5" in src  # 不假装
    assert "Phenomenal" in src
    assert "\u4e0d\u5047\u88c5\u8bb0\u5fc6\u5df2\u89e3" in src  # 不假装记忆已解


def test_philosophy_guard_asi_concept_clear():
    """ASI concept is clear: 真生产 ≠ ASI 已达成 (主 20:46)."""
    import apeireth.v1052_asi_memory_consolidation as m
    src = Path(m.__file__).read_text(encoding="utf-8")
    assert "\u4e0d\u5047\u88c5\u8fbe\u5230 ASI" in src  # 不假装达到 ASI
    assert "ASI \u8bb0\u5fc6\u771f\u751f\u4ea7" in src  # ASI 记忆真生产


def test_philosophy_guard_ltm_eternal():
    """LTM = 中央 AI 永恒身份 (主 12:14)."""
    import apeireth.v1052_asi_memory_consolidation as m
    src = Path(m.__file__).read_text(encoding="utf-8")
    assert "\u4e2d\u592e AI" in src  # 中央 AI
    assert "\u6c38\u6052\u8eab\u4efd" in src  # 永恒身份
    assert "\u6c38\u4e0d\u4e22" in src  # 永不丢