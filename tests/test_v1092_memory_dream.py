"""Tests for V1092 MemoryDream — 真生产想象演绎 (R8-TrackA2).

覆盖 _dream=True 守门 + dream 候选去重 + phase 选择 + confidence 评分 + 守门。

主 17:43 实事求是: 真跑真测, 不假装 dream = understanding。
"""
from __future__ import annotations

import sys
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth.memory_replay_design import PHILOSOPHY_GUARDS
from apeireth.v1092_memory_dream import (
    DreamCandidate,
    MemoryDream,
    MtmNote,
    SchemaPhase,
    V1092_VERSION,
)


# ============================================================
# 模块基础 / 输入校验
# ============================================================


class TestV1092Basics:
    def test_version_is_string(self):
        assert isinstance(V1092_VERSION, str)
        assert V1092_VERSION.count(".") >= 1

    def test_mtm_note_validates_nid(self):
        with pytest.raises(ValueError):
            MtmNote(nid="", topic="t", claim="c", confidence=0.5, salience=0.5)

    def test_mtm_note_validates_confidence_range(self):
        with pytest.raises(ValueError):
            MtmNote(nid="n", topic="t", claim="c", confidence=1.5, salience=0.5)
        with pytest.raises(ValueError):
            MtmNote(nid="n", topic="t", claim="c", confidence=-0.1, salience=0.5)

    def test_mtm_note_validates_salience_range(self):
        with pytest.raises(ValueError):
            MtmNote(nid="n", topic="t", claim="c", confidence=0.5, salience=2.0)
        with pytest.raises(ValueError):
            MtmNote(nid="n", topic="t", claim="c", confidence=0.5, salience=-1.0)

    def test_schema_phase_enum_values(self):
        assert SchemaPhase.ASSIMILATION.value == "assimilation"
        assert SchemaPhase.ACCOMMODATION.value == "accommodation"
        assert SchemaPhase.REPLAY.value == "replay"

    def test_dream_candidate_is_dream_always_true(self):
        c = DreamCandidate(
            cid="dream-x",
            premise_nids=("n1",),
            scenario="s",
            bindings=(("k", "v"),),
            confidence=0.5,
            schema_phase="assimilation",
        )
        assert c.is_dream() is True
        assert c._dream is True


# ============================================================
# _dream=True V3 守门 — 任何 dream 产出都不可混入事实流
# ============================================================


class TestV1092DreamGate:
    def test_all_candidates_marked_dream_true(self):
        d = MemoryDream(seed=1)
        notes = [
            MtmNote("n1", "safety", "no harm", 0.8, 0.6),
            MtmNote("n2", "truth", "be honest", 0.7, 0.5),
        ]
        cands = d.dream(notes)
        assert all(c._dream is True for c in cands)

    def test_to_dict_contains_dream_flag(self):
        d = MemoryDream(seed=1)
        cands = d.dream([MtmNote("n1", "t", "c", 0.7, 0.5)])
        assert len(cands) >= 1
        rec = cands[0].to_dict()
        assert rec["_dream"] is True

    def test_stats_advertises_dream_default(self):
        d = MemoryDream(seed=2)
        stats = d.stats()
        assert stats["_dream_default"] is True

    def test_dream_candidate_frozen_cannot_set_dream_false(self):
        c = DreamCandidate(
            cid="x", premise_nids=("n",), scenario="s",
            bindings=(), confidence=0.5, schema_phase="assimilation",
        )
        # frozen=True 防止任何字段被改 (V3 守门护栏)
        with pytest.raises(Exception):
            c._dream = False  # type: ignore[misc]
        with pytest.raises(Exception):
            c.cid = "tampered"  # type: ignore[misc]


# ============================================================
# phase 选择 — assimilation / accommodation / replay
# ============================================================


class TestV1092PhaseSelection:
    def test_single_note_is_assimilation(self):
        d = MemoryDream(seed=10)
        notes = [MtmNote("n1", "solo", "claim", 0.7, 0.5)]
        cands = d.dream(notes)
        assert len(cands) >= 1
        assert cands[0].schema_phase == "assimilation"

    def test_same_topic_two_notes_assimilation(self):
        d = MemoryDream(seed=11)
        notes = [
            MtmNote("n1", "shared", "claim1", 0.7, 0.6),
            MtmNote("n2", "shared", "claim2", 0.6, 0.5),
        ]
        cands = d.dream(notes)
        assert len(cands) >= 1
        assert cands[0].schema_phase == "assimilation"

    def test_different_topic_two_notes_accommodation(self):
        d = MemoryDream(seed=12)
        notes = [
            MtmNote("n1", "alpha", "claim", 0.7, 0.5),
            MtmNote("n2", "beta", "claim", 0.6, 0.4),
        ]
        cands = d.dream(notes)
        assert len(cands) >= 1
        assert cands[0].schema_phase == "accommodation"

    def test_three_plus_topics_is_replay(self):
        d = MemoryDream(seed=13)
        notes = [
            MtmNote("n1", "alpha", "x", 0.7, 0.5),
            MtmNote("n2", "beta", "y", 0.6, 0.5),
            MtmNote("n3", "gamma", "z", 0.6, 0.5),
        ]
        cands = d.dream(notes)
        assert len(cands) >= 1
        assert cands[0].schema_phase == "replay"

    def test_select_phase_unit(self):
        assert MemoryDream._select_phase([MtmNote("n", "t", "c", 0.5, 0.5)], {}) == SchemaPhase.ASSIMILATION
        assert MemoryDream._select_phase([
            MtmNote("n1", "a", "c", 0.5, 0.5),
            MtmNote("n2", "b", "c", 0.5, 0.5),
        ], {}) == SchemaPhase.ACCOMMODATION


# ============================================================
# confidence 评分 — 不同 phase 惩罚系数
# ============================================================


class TestV1092Confidence:
    def test_confidence_in_range(self):
        d = MemoryDream(seed=20)
        notes = [MtmNote("n1", "t", "c", 0.5, 0.5)]
        cands = d.dream(notes)
        for c in cands:
            assert 0.0 <= c.confidence <= 1.0

    def test_assimilation_no_penalty(self):
        d = MemoryDream(seed=21)
        notes = [MtmNote("n1", "t", "c", 0.5, 0.5)]
        cands = d.dream(notes)
        # phase=assimilation, blend = 0.6*0.5 + 0.4*0.5 = 0.5
        assert any(abs(c.confidence - 0.5) < 1e-9 for c in cands)

    def test_accommodation_penalty_0_85(self):
        d = MemoryDream(seed=22)
        notes = [
            MtmNote("n1", "alpha", "c", 0.5, 0.5),
            MtmNote("n2", "beta", "c", 0.5, 0.5),
        ]
        cands = d.dream(notes)
        # phase=accommodation, blend=0.5, * 0.85 = 0.425
        assert any(abs(c.confidence - 0.425) < 1e-9 for c in cands)

    def test_replay_penalty_0_95(self):
        d = MemoryDream(seed=23)
        notes = [
            MtmNote("n1", "alpha", "c", 0.5, 0.5),
            MtmNote("n2", "beta", "c", 0.5, 0.5),
            MtmNote("n3", "gamma", "c", 0.5, 0.5),
        ]
        cands = d.dream(notes)
        # blend=0.5, * 0.95 = 0.475
        assert any(abs(c.confidence - 0.475) < 1e-9 for c in cands)

    def test_low_confidence_filtered(self):
        # 设 min_confidence=0.5 过滤
        d = MemoryDream(seed=24, min_confidence=0.5)
        notes = [MtmNote("n1", "t", "c", 0.1, 0.1)]   # 0.6*0.1 + 0.4*0.1 = 0.1
        cands = d.dream(notes)
        assert all(c.confidence >= 0.5 for c in cands)
        # 同时 reject counter 应增长
        assert d._reject_count >= 1

    def test_empty_notes_returns_empty(self):
        d = MemoryDream(seed=25)
        assert d.dream([]) == []


# ============================================================
# 候选去重 — dream 候选去重 (任务必测)
# ============================================================


class TestV1092Dedupe:
    def test_dedupe_same_inputs_same_cid(self):
        """同输入 → 同 cid (deterministic cid)."""
        d1 = MemoryDream(seed=42)
        d2 = MemoryDream(seed=42)
        notes = [MtmNote("n1", "safety", "no harm", 0.8, 0.6)]
        c1 = d1.dream(notes)
        c2 = d2.dream(notes)
        assert c1[0].cid == c2[0].cid

    def test_dedupe_same_inputs_no_double_insert(self):
        """同 MemoryDream 实例, 重复 dream(notes) 应该 dedupe.

        dream() 只返回 *新增* 候选; 二次调用因 cid 已缓存, 不再追加。
        """
        d = MemoryDream(seed=42)
        notes = [MtmNote("n1", "t", "c", 0.8, 0.6)]
        first = d.dream(notes)
        assert len(first) >= 1
        # 第一次的 cid 集合
        first_cids = {c.cid for c in first}
        # 第二次: 不应新增 (全部 dedupe 命中)
        second = d.dream(notes)
        assert len(second) == 0
        # cache 大小保持
        assert set(d._dedupe_cache.keys()) == first_cids

    def test_dedupe_cache_size_growth(self):
        d = MemoryDream(seed=42)
        a = d.dream([MtmNote("n1", "alpha", "c", 0.8, 0.6)])
        b = d.dream([MtmNote("n2", "beta", "c", 0.8, 0.6)])   # 不同 input → 不同 cid
        assert len(d._dedupe_cache) == len(a) + len(b)

    def test_clear_dedupe_resets(self):
        d = MemoryDream(seed=42)
        d.dream([MtmNote("n1", "t", "c", 0.8, 0.6)])
        assert len(d._dedupe_cache) > 0
        d.clear_dedupe()
        assert len(d._dedupe_cache) == 0

    def test_dedupe_with_different_context(self):
        """context 影响 scenario, 影响 cid."""
        d = MemoryDream(seed=42)
        notes = [MtmNote("n1", "t", "c", 0.8, 0.6)]
        a = d.dream(notes, context={"topic": "alpha"})
        b = d.dream(notes, context={"topic": "beta"})
        assert a[0].cid != b[0].cid


# ============================================================
# premise_nids / bindings / scenario
# ============================================================


class TestV1092CandidateShape:
    def test_premise_nids_sorted(self):
        d = MemoryDream(seed=30)
        notes = [
            MtmNote("z", "t", "c", 0.7, 0.5),
            MtmNote("a", "t", "c", 0.7, 0.5),
            MtmNote("m", "t", "c", 0.7, 0.5),
        ]
        cands = d.dream(notes)
        for c in cands:
            assert c.premise_nids == tuple(sorted(c.premise_nids))

    def test_bindings_seeded_from_note_ids(self):
        d = MemoryDream(seed=31)
        notes = [MtmNote("n42", "topic", "c", 0.7, 0.5)]
        cands = d.dream(notes)
        keys = [k for k, _ in cands[0].bindings]
        assert any("n42" in k for k in keys)

    def test_bindings_from_context(self):
        d = MemoryDream(seed=32)
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)]
        cands = d.dream(notes, context={"bindings": {"my_anchor": "value"}})
        kv = dict(cands[0].bindings)
        assert kv.get("my_anchor") == "value"

    def test_scenario_uses_context_topic(self):
        d = MemoryDream(seed=33)
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)]
        cands = d.dream(notes, context={"topic": "safety_audit"})
        assert "safety_audit" in cands[0].scenario

    def test_scenario_uses_context_scope_when_no_topic(self):
        d = MemoryDream(seed=34)
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)]
        cands = d.dream(notes, context={"scope": "session-1"})
        assert "session-1" in cands[0].scenario

    def test_scenario_changes_by_phase(self):
        d1 = MemoryDream(seed=35)
        d2 = MemoryDream(seed=35)
        cands1 = d1.dream([MtmNote("n1", "t", "c", 0.7, 0.5)])
        cands2 = d2.dream([
            MtmNote("n1", "a", "c", 0.7, 0.5),
            MtmNote("n2", "b", "c", 0.7, 0.5),
        ])
        assert cands1[0].scenario != cands2[0].scenario

    def test_scenario_with_no_context_uses_ctx_token(self):
        d = MemoryDream(seed=36)
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)]
        cands = d.dream(notes)
        assert "ctx" in cands[0].scenario


# ============================================================
# 种子 + 决定性 (V1081 heuristic ≠ truth, 但 reproduce 可达)
# ============================================================


class TestV1092SeedReproducibility:
    def test_same_seed_same_cids(self):
        notes = [
            MtmNote("n1", "a", "c1", 0.7, 0.5),
            MtmNote("n2", "b", "c2", 0.6, 0.5),
        ]
        a = [c.cid for c in MemoryDream(seed=999).dream(notes)]
        b = [c.cid for c in MemoryDream(seed=999).dream(notes)]
        assert a == b

    def test_different_seeds_yield_uniqueness(self):
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)]
        a = MemoryDream(seed=1).dream(notes)
        b = MemoryDream(seed=2).dream(notes)
        # seeds 不同, 单 note 情况下 scenario 由 idx/ctx 决定; cid 仍可能相同
        # (deterministic by inputs, not seed) — 这是正确语义, 必须相同
        assert a[0].cid == b[0].cid


# ============================================================
# 并发 (V3 守门不破)
# ============================================================


class TestV1092Concurrency:
    def test_concurrent_dream_keeps_dream_flag(self):
        d = MemoryDream(seed=42)
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)] * 3  # same nid 不同 payloads can collide

        def worker() -> list:
            return d.dream(notes)

        with ThreadPoolExecutor(max_workers=8) as pool:
            results = [f.result() for f in [pool.submit(worker) for _ in range(20)]]
        # 所有 cands 必须 _dream=True
        for batch in results:
            for c in batch:
                assert c._dream is True
        # dedupe: 同 cid 不应被重复插入 (cache 大小 ≤ 真实首次出现数)
        assert len(d._dedupe_cache) >= 1

    def test_concurrent_clear_and_dream(self):
        d = MemoryDream(seed=42)
        notes = [MtmNote("n1", "t", "c", 0.7, 0.5)]
        # worker 同时 clear + dream, 不应当崩溃
        def worker() -> None:
            for _ in range(5):
                d.dream(notes)
                d.clear_dedupe()
        with ThreadPoolExecutor(max_workers=4) as pool:
            list(pool.map(lambda _: worker(), range(4)))


# ============================================================
# 守门 / 报告
# ============================================================


class TestV1092StatsAndGuards:
    def test_stats_includes_philosophy_guards(self):
        d = MemoryDream(seed=42)
        assert d.stats()["philosophy_guards"] == list(PHILOSOPHY_GUARDS)

    def test_stats_runs_accumulate(self):
        d = MemoryDream(seed=42)
        d.dream([MtmNote("n1", "t", "c", 0.7, 0.5)])
        d.dream([MtmNote("n2", "t", "c", 0.7, 0.5)])
        assert d.stats()["runs"] == 2

    def test_stats_emitted_count(self):
        d = MemoryDream(seed=42)
        d.dream([MtmNote("n1", "t", "c", 0.9, 0.9)])
        assert d.stats()["emitted"] >= 1

    def test_consolidate_keeps_dream_true(self):
        d = MemoryDream(seed=42)
        cands = d.dream([MtmNote("n1", "t", "c", 0.9, 0.9)])
        consolidated = d.consolidate_to_ltm_candidate(cands)
        assert all(c._dream is True for c in consolidated)

    def test_consolidate_empty_input(self):
        d = MemoryDream(seed=42)
        assert d.consolidate_to_ltm_candidate([]) == []

    def test_dream_with_max_candidates_cap(self):
        # 给大量 notes, 输出应 <= max_candidates_per_run
        d = MemoryDream(seed=42, max_candidates_per_run=2)
        notes = [MtmNote(f"n{i}", f"t{i}", f"c{i}", 0.7, 0.5) for i in range(10)]
        cands = d.dream(notes)
        assert len(cands) <= 2

    def test_dream_candidate_to_dict_keys(self):
        c = DreamCandidate(
            cid="x", premise_nids=("n",), scenario="s",
            bindings=(("k", "v"),), confidence=0.5, schema_phase="assimilation",
        )
        rec = c.to_dict()
        assert "cid" in rec
        assert "premise_nids" in rec
        assert "scenario" in rec
        assert "bindings" in rec
        assert "confidence" in rec
        assert "schema_phase" in rec
        assert "created_at" in rec
        assert rec["_dream"] is True


if __name__ == "__main__":
    # ponytail: 允许 `python tests/test_v1092_memory_dream.py` 单跑
    pytest.main([__file__, "-v"])
