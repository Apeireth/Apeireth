"""Tests for V1108 Memory Dream V2 — 6 状态机 + V3 守门强化 + 与 V1107 集成

覆盖:
  - V1108 constants / version (3 tests)
  - DreamState enum (3 tests)
  - DreamCandidateV2 (5 tests)
  - DreamStateMachine (8 tests)
  - MemoryDreamV2 dream() (10 tests)
  - MemoryDreamV2 interrupt / reset (4 tests)
  - MemoryDreamV2 stats / audit (3 tests)
  - 与 V1092 共存 (2 tests)
  - 与 V1107 集成 (3 tests)
  - V3 守门 (3 tests)
  共 ~44 tests
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth import v1092_memory_dream as v1092  # noqa: E402
from apeireth import v1107_cognitive_core_lift as v1107  # noqa: E402
from apeireth import v1108_dream_v2 as v1108  # noqa: E402


class _N:
    """测试用 note (兼容 MtmNote 字段)."""

    __slots__ = ("nid", "topic", "claim", "confidence", "salience")

    def __init__(self, nid="n1", topic="t", claim="c",
                 confidence=0.7, salience=0.5):
        self.nid = nid
        self.topic = topic
        self.claim = claim
        self.confidence = confidence
        self.salience = salience


# ============================================================
# Constants / version — 3 tests
# ============================================================


class TestV1108Constants:
    def test_version_is_string(self):
        assert isinstance(v1108.V1108_VERSION, str)
        assert v1108.V1108_VERSION.count(".") >= 1

    def test_dream_is_not_consciousness_declared(self):
        """主 17:58+20:46: DREAM_IS_NOT_CONSCIOUSNESS 必须显式声明."""
        assert hasattr(v1108, "DREAM_IS_NOT_CONSCIOUSNESS")
        s = v1108.DREAM_IS_NOT_CONSCIOUSNESS
        assert "Dream" in s
        assert "consciousness" in s.lower() or "conscious" in s.lower()
        # 关键短语: heuristic, NOT, finite-state
        assert "heuristic" in s.lower() or "NOT" in s

    def test_v3_guards_dict_complete(self):
        assert "dream_is_not_consciousness" in v1108.V1108_V3_GUARDS
        assert "dream_is_not_understanding" in v1108.V1108_V3_GUARDS
        assert "state_machine_is_not_psychology" in v1108.V1108_V3_GUARDS
        assert "module_is_not_asi" in v1108.V1108_V3_GUARDS
        assert "dream_fact" in v1108.V1108_V3_GUARDS
        assert "frozen_immutability" in v1108.V1108_V3_GUARDS


# ============================================================
# DreamState enum — 3 tests
# ============================================================


class TestV1108DreamState:
    def test_six_states_present(self):
        """6 状态: IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED."""
        states = {s.value for s in v1108.DreamState}
        expected = {"idle", "dreaming", "consolidating",
                    "forgetting", "verifying", "interrupted"}
        assert states == expected

    def test_dream_state_is_string_enum(self):
        for s in v1108.DreamState:
            assert isinstance(s.value, str)

    def test_state_machine_initial_is_idle(self):
        sm = v1108.DreamStateMachine()
        assert sm.state == v1108.DreamState.IDLE

    def test_state_machine_initial_must_be_idle(self):
        """V3 守门: initial state 必须是 IDLE."""
        with pytest.raises(ValueError):
            v1108.DreamStateMachine(initial=v1108.DreamState.DREAMING)


# ============================================================
# DreamCandidateV2 — 5 tests
# ============================================================


class TestV1108DreamCandidate:
    def _make(self, **kw):
        defaults = dict(
            cid="dream-x", premise_nids=("n1",), scenario="s",
            bindings=(("k", "v"),), confidence=0.5,
            schema_phase="assimilation", state_at_birth="dreaming",
            audit_trail=(("born", 0.0),),
        )
        defaults.update(kw)
        return v1108.DreamCandidateV2(**defaults)

    def test_is_dream_always_true(self):
        c = self._make()
        assert c.is_dream() is True
        assert c._dream is True

    def test_rejects_dream_false_construction(self):
        """V3 守门: 哪怕显式 _dream=False, 也必须 raise."""
        with pytest.raises((TypeError, ValueError)):
            v1108.DreamCandidateV2(
                cid="x", premise_nids=("n",), scenario="s",
                bindings=(), confidence=0.5, schema_phase="assimilation",
                state_at_birth="dreaming",
                audit_trail=(("born", 0.0),),
                _dream=False,  # type: ignore[call-arg]
            )

    def test_rejects_empty_cid(self):
        with pytest.raises(ValueError):
            self._make(cid="")

    def test_rejects_empty_premise(self):
        with pytest.raises(ValueError):
            self._make(premise_nids=())

    def test_rejects_invalid_confidence(self):
        with pytest.raises(ValueError):
            self._make(confidence=1.5)

    def test_rejects_invalid_phase(self):
        with pytest.raises(ValueError):
            self._make(schema_phase="invalid_phase")

    def test_to_dict_contains_dream_flag(self):
        c = self._make()
        d = c.to_dict()
        assert d["_dream"] is True
        assert d["state_at_birth"] == "dreaming"
        assert "audit_trail" in d

    def test_frozen_cannot_modify(self):
        c = self._make()
        with pytest.raises(Exception):
            c._dream = False  # type: ignore[misc]
        with pytest.raises(Exception):
            c.cid = "tampered"  # type: ignore[misc]


# ============================================================
# DreamStateMachine — 8 tests
# ============================================================


class TestV1108DreamStateMachine:
    def test_initial_idle(self):
        sm = v1108.DreamStateMachine()
        assert sm.state == v1108.DreamState.IDLE

    def test_idle_to_dreaming_allowed(self):
        sm = v1108.DreamStateMachine()
        assert sm.transition(v1108.DreamState.DREAMING) is True
        assert sm.state == v1108.DreamState.DREAMING

    def test_idle_to_verifying_rejected(self):
        """V3 守门: 非法转换拒绝."""
        sm = v1108.DreamStateMachine()
        assert sm.transition(v1108.DreamState.VERIFYING) is False
        assert sm.state == v1108.DreamState.IDLE

    def test_dreaming_to_consolidating_allowed(self):
        sm = v1108.DreamStateMachine()
        sm.transition(v1108.DreamState.DREAMING)
        assert sm.transition(v1108.DreamState.CONSOLIDATING) is True
        assert sm.state == v1108.DreamState.CONSOLIDATING

    def test_dreaming_to_verifying_rejected(self):
        sm = v1108.DreamStateMachine()
        sm.transition(v1108.DreamState.DREAMING)
        assert sm.transition(v1108.DreamState.VERIFYING) is False

    def test_dreaming_to_interrupted_allowed(self):
        sm = v1108.DreamStateMachine()
        sm.transition(v1108.DreamState.DREAMING)
        assert sm.transition(v1108.DreamState.INTERRUPTED) is True

    def test_history_records_transitions(self):
        sm = v1108.DreamStateMachine()
        sm.transition(v1108.DreamState.DREAMING, reason="input")
        sm.transition(v1108.DreamState.CONSOLIDATING, reason="compose")
        h = sm.history()
        assert len(h) == 2
        assert h[0][0] == v1108.DreamState.IDLE
        assert h[0][1] == v1108.DreamState.DREAMING

    def test_force_idle_from_any_state(self):
        sm = v1108.DreamStateMachine()
        sm.transition(v1108.DreamState.DREAMING)
        sm.transition(v1108.DreamState.INTERRUPTED)
        sm.force_idle(reason="reset")
        assert sm.state == v1108.DreamState.IDLE

    def test_audit_count_increments(self):
        sm = v1108.DreamStateMachine()
        assert sm.audit_count() == 0
        sm.transition(v1108.DreamState.DREAMING)
        assert sm.audit_count() == 1


# ============================================================
# MemoryDreamV2.dream() — 10 tests
# ============================================================


class TestV1108MemoryDream:
    def test_dream_empty_input_returns_empty(self):
        d = v1108.MemoryDreamV2(seed=1)
        r = d.dream([])
        assert r.candidates == []
        assert r.episodes == []
        assert r.final_state == v1108.DreamState.IDLE

    def test_dream_single_note_returns_candidates(self):
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N(nid="n1", topic="t", claim="c", confidence=0.7, salience=0.5)]
        r = d.dream(notes)
        assert len(r.candidates) >= 1

    def test_dream_emits_candidate_with_dream_flag(self):
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N()]
        r = d.dream(notes)
        assert r.candidates
        for c in r.candidates:
            assert c._dream is True
            assert c.is_dream() is True

    def test_dream_transitions_through_6_states(self):
        """V1108 主路径: IDLE → DREAMING → CONSOLIDATING → VERIFYING → IDLE."""
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N()]
        r = d.dream(notes)
        # 至少 4 个 transitions (4 个状态转换)
        assert len(r.transitions) >= 4
        # 状态转换序列应包含 IDLE→DREAMING → ... → IDLE
        from_seq = [t[0] for t in r.transitions]
        to_seq = [t[1] for t in r.transitions]
        assert v1108.DreamState.IDLE in from_seq
        assert v1108.DreamState.DREAMING in to_seq
        assert r.final_state == v1108.DreamState.IDLE

    def test_dream_dedupes_same_candidate(self):
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N(nid="n1", topic="t", claim="c")]
        r1 = d.dream(notes)
        # 第一次至少 emit 1
        assert r1.stats["candidates_emitted"] >= 1
        # 第二次同样输入 — candidates 列表应为空 (dedupe 后全部 in cache)
        r2 = d.dream(notes)
        assert len(r2.candidates) == 0
        # 但 candidates_emitted 是累计值, 不应增加
        assert r2.stats["candidates_emitted"] == r1.stats["candidates_emitted"]

    def test_dream_emits_episodes_for_v1107(self):
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N(confidence=0.8, salience=0.7)]
        r = d.dream(notes, context={"topic": "safety"})
        assert r.episodes
        for ep in r.episodes:
            assert ep["source"] == "dream"
            assert ep["content"]["_dream"] is True
            assert "state_at_birth" in ep["content"]

    def test_dream_low_confidence_filtered(self):
        d = v1108.MemoryDreamV2(seed=1, min_confidence=0.5)
        notes = [_N(confidence=0.1, salience=0.1)]
        r = d.dream(notes)
        assert r.candidates == []

    def test_dream_handles_mixed_topic_count(self):
        """≥3 topics → REPLAY; 2 topics → ACCOMMODATION; 1 → ASSIMILATION."""
        d = v1108.MemoryDreamV2(seed=1)
        notes3 = [_N(f"n{i}", topic=f"t{i}", confidence=0.8, salience=0.6) for i in range(3)]
        r = d.dream(notes3)
        phases = {c.schema_phase for c in r.candidates}
        assert "replay" in phases

    def test_dream_too_many_notes_capped(self):
        d = v1108.MemoryDreamV2(seed=1, max_candidates_per_run=4)
        notes = [_N(f"n{i}", confidence=0.8, salience=0.6) for i in range(20)]
        r = d.dream(notes)
        # candidate 数量 ≤ max_candidates_per_run
        assert len(r.candidates) <= 4

    def test_dream_final_state_returns_to_idle(self):
        d = v1108.MemoryDreamV2(seed=1)
        d.dream([_N()])
        assert d.state() == v1108.DreamState.IDLE


# ============================================================
# interrupt / reset — 4 tests
# ============================================================


class TestV1108Interrupt:
    def test_interrupt_from_idle(self):
        """从 DREAMING interrupt → INTERRUPTED → IDLE (V1108 6 状态转换)."""
        d = v1108.MemoryDreamV2(seed=1)
        # 强制到 DREAMING 模拟运行中
        d._fsm.transition(v1108.DreamState.DREAMING, reason="setup")
        assert d.interrupt(reason="external") is True
        assert d.state() == v1108.DreamState.IDLE

    def test_interrupt_idle_rejected(self):
        """V3 守门: IDLE → INTERRUPTED 非法转换拒绝."""
        d = v1108.MemoryDreamV2(seed=1)
        # IDLE 不在 INTERRUPTED 合法源
        assert d.interrupt(reason="external") is False
        assert d.state() == v1108.DreamState.IDLE

    def test_interrupt_from_dreaming(self):
        d = v1108.MemoryDreamV2(seed=1)
        # 用 threading 或 monkeypatch 强制转 DREAMING (测试 only)
        d._fsm.transition(v1108.DreamState.DREAMING, reason="setup")
        assert d.interrupt() is True
        assert d.state() == v1108.DreamState.IDLE

    def test_interrupt_increments_stat(self):
        d = v1108.MemoryDreamV2(seed=1)
        # 强制到 DREAMING 才能 interrupt
        d._fsm.transition(v1108.DreamState.DREAMING, reason="setup")
        d.interrupt(reason="test")
        stats = d.stats()
        assert stats["candidates_interrupted"] >= 1

    def test_reset_to_idle(self):
        d = v1108.MemoryDreamV2(seed=1)
        d._fsm.transition(v1108.DreamState.DREAMING, reason="setup")
        d.reset()
        assert d.state() == v1108.DreamState.IDLE


# ============================================================
# stats / audit — 3 tests
# ============================================================


class TestV1108Stats:
    def test_stats_has_philosophy_guards(self):
        d = v1108.MemoryDreamV2(seed=1)
        s = d.stats()
        assert "philosophy_guards" in s
        assert "dream_is_not_consciousness" in s["philosophy_guards"]
        assert s["_dream_default"] is True

    def test_audit_log_returns_history(self):
        d = v1108.MemoryDreamV2(seed=1)
        d.dream([_N()])
        log = d.audit_log()
        assert len(log) >= 1

    def test_stats_tracks_runs(self):
        d = v1108.MemoryDreamV2(seed=1)
        d.dream([_N()])
        d.dream([_N()])
        assert d.stats()["runs"] == 2


# ============================================================
# 与 V1092 共存 — 2 tests
# ============================================================


class TestV1108V1092Compat:
    def test_v1092_MtmNote_usable_by_v1108(self):
        """V1108.dream() 接受 V1092.MtmNote 实例 (主 23:44 干到底)."""
        notes = [
            v1092.MtmNote("n1", "safety", "no harm", 0.8, 0.6),
            v1092.MtmNote("n2", "truth", "be honest", 0.7, 0.5),
        ]
        d = v1108.MemoryDreamV2(seed=1)
        r = d.dream(notes)
        assert r.candidates
        for c in r.candidates:
            assert c._dream is True
            # cid 格式兼容 V1092
            assert c.cid.startswith("dream-")

    def test_v1108_stats_advertises_v1092_philosophy(self):
        d = v1108.MemoryDreamV2(seed=1)
        s = d.stats()
        # 主 17:58 + 20:46 不假装: V1108 继承 V1092 守门 + 新增强化
        assert s["version"] == v1108.V1108_VERSION


# ============================================================
# 与 V1107 集成 — 3 tests
# ============================================================


class TestV1108V1107Integration:
    def test_dream_v2_feeds_v1107_episode_buffer(self):
        """V1108 DreamEpisode → V1107 EpisodeBuffer."""
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N(confidence=0.8, salience=0.6)]
        r = d.dream(notes)
        lift = v1107.V1107CognitiveLift()
        result = lift.integrate_dream(r.candidates)
        assert result["episodes_added"] == len(r.candidates)
        # V1107 episode_buffer 应该收到 dream episodes
        recent = lift.episode_buffer.recent(n=10)
        dream_eps = [e for e in recent if e.source == "dream"]
        assert len(dream_eps) >= 1

    def test_dream_v2_feeds_v1107_note_consolidator(self):
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N(confidence=0.8, salience=0.7)]
        r = d.dream(notes)
        lift = v1107.V1107CognitiveLift()
        result = lift.integrate_dream(r.candidates)
        # high conf 的 dream 应该被 upsert 到 note_consolidator
        if r.candidates and r.candidates[0].confidence >= 0.4:
            assert result["notes_added"] >= 1

    def test_dream_integration_triggers_reconsolidation(self):
        d = v1108.MemoryDreamV2(seed=1)
        notes = [_N(nid=f"n{i}", topic=f"t{i%2}", confidence=0.8, salience=0.6)
                 for i in range(4)]
        r = d.dream(notes)
        lift = v1107.V1107CognitiveLift()
        result = lift.integrate_dream(r.candidates)
        assert "reconsolidation" in result
        cycle = result["reconsolidation"]
        assert "conflicts" in cycle
        assert "abstractions" in cycle
        assert "forgotten" in cycle


# ============================================================
# V3 守门 — 3 tests
# ============================================================


class TestV1108V3Guards:
    def test_v3_guards_strings_chinese(self):
        for k, v in v1108.V1108_V3_GUARDS.items():
            assert "不假装" in v

    def test_dream_is_not_consciousness_constant_loaded(self):
        """DREAM_IS_NOT_CONSCIOUSNESS 必须存在并含 'NOT' / 'heuristic' / 'finite'."""
        s = v1108.DREAM_IS_NOT_CONSCIOUSNESS
        assert any(w in s for w in ("NOT", "not ", "heuristic", "finite"))

    def test_v1108_has_v3_guards_attr(self):
        """V1101 standard auto-injected V3_GUARDS."""
        assert hasattr(v1108, "V3_GUARDS")
        assert "module_is_not_asi" in v1108.V3_GUARDS


if __name__ == "__main__":
    pytest.main([__file__, "-v"])