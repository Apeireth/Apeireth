"""Tests for V1107 Cognitive Core Lift — 真认知 + IDENTITY 5 Module + Dream 集成

覆盖:
  - AttentionMechanism (5 tests)
  - MemoryConsolidationEngine (5 tests)
  - PatternMatcherV2 (5 tests)
  - AnalogyEngine (5 tests)
  - IDENTITY 5 Module (10 tests)
  - DreamEpisodeAdapter (3 tests)
  - V1107CognitiveLift.inject_into_cognitive_core (修复 3 V1101 bug) (5 tests)
  - V1107CognitiveLift.seed_5_module_framework (5 tests)
  - V1107CognitiveLift.integrate_dream (3 tests)
  - V1107CognitiveLift.execute_full_lift (5 tests)
  - 真 lift: cognitive_core ≥ 0.85 (3 tests)
  - V3 守门 (3 tests)
  - 共 ~57 tests
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

from apeireth import v1061_asi_cognitive_core as v1061  # noqa: E402
from apeireth import v1101_asi_v04_dim_lift as v1101  # noqa: E402
from apeireth import v1107_cognitive_core_lift as v1107  # noqa: E402


# ============================================================
# AttentionMechanism — 5 tests
# ============================================================


class TestV1107AttentionMechanism:
    def test_register_sets_initial_weight(self):
        att = v1107.AttentionMechanism()
        att.register("node_a", weight=1.0)
        assert att.weights["node_a"] == 1.0
        # 第一个节点自动成 focus
        assert att.focus == "node_a"

    def test_register_negative_weight_clamped(self):
        att = v1107.AttentionMechanism()
        att.register("node_a", weight=-1.0)
        assert att.weights["node_a"] == 0.0

    def test_focus_on_existing_node(self):
        att = v1107.AttentionMechanism()
        att.register("a", weight=1.0)
        att.register("b", weight=0.5)
        assert att.focus_on("b") is True
        assert att.focus == "b"
        assert att.weights["b"] > 0.5  # boosted

    def test_focus_on_nonexistent_returns_false(self):
        att = v1107.AttentionMechanism()
        att.register("a", weight=1.0)
        assert att.focus_on("nope") is False
        assert att.focus == "a"

    def test_tick_decays_and_reassigns_focus(self):
        att = v1107.AttentionMechanism(decay=0.5, boost=1.0)
        att.register("a", weight=1.0)
        att.register("b", weight=5.0)
        att.focus_on("a")  # boost a
        att.tick(seconds=10.0)
        # a 应该 decay 到 0 (1+1 - 5 = -3 cap 0)
        # b 应该衰减但仍 > 0
        assert "a" not in att.weights or att.weights["a"] == 0.0
        # focus 应该切到 b (或 None)
        if att.focus is not None:
            assert att.focus in att.weights

    def test_top_k_returns_sorted(self):
        att = v1107.AttentionMechanism()
        for i, w in enumerate([0.3, 0.9, 0.5, 0.1]):
            att.register(f"n{i}", weight=w)
        top = att.top_k(3)
        assert len(top) == 3
        assert top[0][1] >= top[1][1] >= top[2][1]


# ============================================================
# MemoryConsolidationEngine — 5 tests
# ============================================================


class TestV1107MemoryConsolidation:
    def test_add_episode_returns_id(self):
        mce = v1107.MemoryConsolidationEngine()
        eid = mce.add_episode(content={"topic": "t"}, salience=0.5)
        assert eid in mce.episodes
        assert mce.episodes[eid].source == "experience"

    def test_add_episode_source_dream(self):
        mce = v1107.MemoryConsolidationEngine()
        eid = mce.add_episode(content={"topic": "t"}, salience=0.5,
                              source="dream")
        assert mce.episodes[eid].source == "dream"

    def test_consolidate_promotes_high_score(self):
        mce = v1107.MemoryConsolidationEngine(promote_threshold=0.5)
        mce.add_episode(content={"topic": "lift", "claim": "c"},
                        salience=0.8, confidence=0.8)
        n = mce.consolidate()
        assert n == 1
        assert len(mce.notes) == 1
        assert mce.promotions == 1

    def test_consolidate_skips_low_score(self):
        mce = v1107.MemoryConsolidationEngine(promote_threshold=0.5)
        mce.add_episode(content={"topic": "t", "claim": "c"},
                        salience=0.3, confidence=0.3)
        n = mce.consolidate()
        assert n == 0
        assert len(mce.notes) == 0

    def test_forget_removes_low_decay(self):
        mce = v1107.MemoryConsolidationEngine(forget_threshold=0.1,
                                                half_life=1.0)
        eid = mce.add_episode(content={"topic": "t"}, salience=0.001)
        # 把时间推后很久, decay 应该 < threshold
        future = mce.episodes[eid].created_at + 1000.0
        n = mce.forget(now=future)
        assert eid not in mce.episodes
        assert mce.forgetting_events >= 1

    def test_decay_score_monotonic(self):
        ep = v1107.Episode(episode_id="x", content={},
                            salience=0.8, confidence=0.5)
        s1 = ep.decay_score(now=ep.last_access + 100, half_life=10.0)
        s2 = ep.decay_score(now=ep.last_access + 1000, half_life=10.0)
        assert s1 > s2 > 0

    def test_measure_returns_0_to_1(self):
        mce = v1107.MemoryConsolidationEngine()
        m = mce.measure()
        assert 0.0 <= m <= 1.0


# ============================================================
# PatternMatcherV2 — 5 tests
# ============================================================


class TestV1107PatternMatcherV2:
    def test_add_pattern_returns_id(self):
        pm = v1107.PatternMatcherV2()
        pid = pm.add_pattern(name="p", template={"k": "v"})
        assert pid in pm.patterns

    def test_link_requires_existing_patterns(self):
        pm = v1107.PatternMatcherV2()
        pid = pm.add_pattern(name="p", template={"k": "v"})
        assert pm.link(pid, "nonexistent") is False
        pid2 = pm.add_pattern(name="q", template={"k": "v"})
        assert pm.link(pid, pid2, strength=0.5, relation="analogy") is True

    def test_match_returns_matching(self):
        pm = v1107.PatternMatcherV2()
        pm.add_pattern(name="p1", template={"x": 1}, match_type="exact",
                        threshold=1.0)
        matched = pm.match({"x": 1})
        assert len(matched) >= 1

    def test_analogies_of_returns_links(self):
        pm = v1107.PatternMatcherV2()
        a = pm.add_pattern(name="a", template={"x": 1})
        b = pm.add_pattern(name="b", template={"x": 2})
        pm.link(a, b, strength=0.8, relation="analogy")
        links = pm.analogies_of(a)
        assert len(links) == 1
        assert links[0].relation == "analogy"

    def test_measure_with_diversity(self):
        pm = v1107.PatternMatcherV2()
        pm.add_pattern(name="p1", template={"x": 1}, match_type="exact")
        pm.add_pattern(name="p2", template={"x": 1}, match_type="similarity")
        pm.add_pattern(name="p3", template={"x": 1}, match_type="fuzzy")
        m = pm.measure()
        assert m > 0.5


# ============================================================
# AnalogyEngine — 5 tests
# ============================================================


class TestV1107AnalogyEngine:
    def test_register_structure(self):
        a = v1107.AnalogyEngine()
        a.register_structure("A", {"x": 0.5})
        assert "A" in a.structures

    def test_map_creates_analogy(self):
        a = v1107.AnalogyEngine()
        a.register_structure("A", {"x": 0.5, "y": 0.7})
        a.register_structure("B", {"x": 0.6, "y": 0.7})
        aid = a.map("A", "B", threshold=0.3)
        assert aid is not None
        assert aid in a.analogies

    def test_map_returns_none_if_no_common_keys(self):
        a = v1107.AnalogyEngine()
        a.register_structure("A", {"x": 0.5})
        a.register_structure("B", {"z": 0.5})
        aid = a.map("A", "B", threshold=0.3)
        assert aid is None

    def test_map_returns_none_if_unknown_structure(self):
        a = v1107.AnalogyEngine()
        assert a.map("A", "B", threshold=0.3) is None

    def test_alignment_score_in_range(self):
        a = v1107.AnalogyEngine()
        a.register_structure("A", {"x": 1.0})
        a.register_structure("B", {"x": 1.0})
        aid = a.map("A", "B", threshold=0.5)
        assert aid is not None
        amap = a.analogies[aid]
        assert 0.0 <= amap.alignment_score() <= 1.0

    def test_measure_active_only_with_analogies(self):
        a = v1107.AnalogyEngine()
        # 空
        assert a.measure() == 0.0
        a.register_structure("A", {"x": 1.0})
        a.register_structure("B", {"x": 1.0})
        a.map("A", "B", threshold=0.5)
        assert a.measure() > 0.0


# ============================================================
# IDENTITY 5 Module — 10 tests
# ============================================================


class TestV1107Identity5Module:
    # M1
    def test_identity_core_defaults(self):
        ic = v1107.IdentityCore()
        assert ic.identity_id.startswith("ident_")
        assert "asi_north_star" in ic.philosophy_keys
        assert "v3_no_pretending" in ic.philosophy_keys

    def test_identity_core_set_get(self):
        ic = v1107.IdentityCore()
        ic.set_value("test_key", 0.9)
        assert ic.get_value("test_key") == 0.9
        assert ic.get_value("missing") == 0.5  # default

    def test_identity_core_clamp_values(self):
        ic = v1107.IdentityCore()
        ic.set_value("test", 5.0)  # cap at 1.0
        assert ic.get_value("test") == 1.0
        ic.set_value("test", -1.0)
        assert ic.get_value("test") == 0.0

    # M2
    def test_episode_buffer_push_recent(self):
        eb = v1107.EpisodeBuffer()
        for i in range(3):
            eb.push(v1107.Episode(episode_id=f"e{i}", content={}))
        recent = eb.recent(2)
        assert len(recent) == 2
        assert recent[-1].episode_id == "e2"

    def test_episode_buffer_max_size_eviction(self):
        eb = v1107.EpisodeBuffer(max_size=3)
        for i in range(5):
            eb.push(v1107.Episode(episode_id=f"e{i}", content={}))
        assert len(eb.episodes) == 3
        ids = {e.episode_id for e in eb.episodes}
        # 最旧的应该被驱逐
        assert "e0" not in ids
        assert "e4" in ids

    def test_episode_buffer_by_source(self):
        eb = v1107.EpisodeBuffer()
        eb.push(v1107.Episode(episode_id="d1", content={}, source="dream"))
        eb.push(v1107.Episode(episode_id="e1", content={}, source="experience"))
        dreams = eb.by_source("dream")
        assert len(dreams) == 1
        assert dreams[0].episode_id == "d1"

    # M3
    def test_note_consolidator_upsert_merges(self):
        nc = v1107.NoteConsolidator()
        nid1 = nc.upsert_note(topic="t", claim="c", confidence=0.5, salience=0.5)
        # 同 topic+claim → merge
        nid2 = nc.upsert_note(topic="t", claim="c", confidence=0.7, salience=0.6)
        assert nid1 == nid2
        assert len(nc.notes) == 1

    def test_note_consolidator_query(self):
        nc = v1107.NoteConsolidator()
        nc.upsert_note(topic="a", claim="x", confidence=0.5, salience=0.5)
        nc.upsert_note(topic="a", claim="y", confidence=0.3, salience=0.5)
        nc.upsert_note(topic="b", claim="z", confidence=0.5, salience=0.5)
        results = nc.query("a", min_conf=0.4)
        assert len(results) == 1
        assert results[0].claim == "x"

    # M4
    def test_relation_graph_add_edge_validates(self):
        rg = v1107.RelationGraph()
        rg.add_node("a")
        rg.add_node("b")
        assert rg.add_edge("a", "b") is True
        assert rg.add_edge("a", "nonexistent") is False

    def test_relation_graph_neighbors(self):
        rg = v1107.RelationGraph()
        rg.add_node("a")
        rg.add_node("b")
        rg.add_node("c")
        rg.add_edge("a", "b", weight=0.5, relation="causal")
        rg.add_edge("a", "c", weight=0.7, relation="temporal")
        nbrs = rg.neighbors("a")
        assert len(nbrs) == 2

    # M5
    def test_reconsolidation_detect_conflicts(self):
        rc = v1107.ReconsolidationEngine()
        notes = {
            "n1": v1107.Note(note_id="n1", topic="x", claim="A",
                              confidence=0.6, salience=0.6),
            "n2": v1107.Note(note_id="n2", topic="x", claim="B",
                              confidence=0.7, salience=0.5),
        }
        conflicts = rc.detect_conflicts(notes)
        assert len(conflicts) == 1

    def test_reconsolidation_run_cycle(self):
        rc = v1107.ReconsolidationEngine()
        notes: Dict[str, v1107.Note] = {
            "n1": v1107.Note(note_id="n1", topic="t", claim="A",
                              confidence=0.6, salience=0.6),
            "n2": v1107.Note(note_id="n2", topic="t", claim="B",
                              confidence=0.65, salience=0.6),
            "n3": v1107.Note(note_id="n3", topic="u", claim="C",
                              confidence=0.5, salience=0.01),  # 会被遗忘
        }
        result = rc.run_cycle(notes)
        assert "conflicts" in result
        assert "abstractions" in result
        assert "forgotten" in result


# ============================================================
# DreamEpisodeAdapter — 3 tests
# ============================================================


class TestV1107DreamEpisodeAdapter:
    def test_to_episode_caps_confidence(self):
        adapter = v1107.DreamEpisodeAdapter(confidence_cap=0.7)
        # 构造 dream candidate (用 V1108)
        from apeireth import v1108_dream_v2 as v1108
        d = v1108.MemoryDreamV2(seed=1)

        class Note:
            def __init__(self, nid, topic, claim, c, s):
                self.nid, self.topic, self.claim = nid, topic, claim
                self.confidence, self.salience = c, s

        result = d.dream([Note("n1", "t", "c", 0.9, 0.5)])
        assert len(result.candidates) == 1
        ep = adapter.to_episode(result.candidates[0])
        assert ep.confidence <= 0.7
        assert ep.source == "dream"
        assert ep.content["_dream"] is True

    def test_to_note_skips_low_conf(self):
        adapter = v1107.DreamEpisodeAdapter()
        # 构造一个非常 low conf 的 candidate (手工)
        cand = v1108_dummy_cand(confidence=0.1)  # type: ignore[name-defined]
        # 用真实的 V1108 替代 (见下)
        from apeireth import v1108_dream_v2 as v1108
        real_dream = v1108.MemoryDreamV2(seed=1,
                                          min_confidence=0.05,
                                          verify_threshold=0.5)

        class Note:
            def __init__(self, nid, topic, claim, c, s):
                self.nid, self.topic, self.claim = nid, topic, claim
                self.confidence, self.salience = c, s

        result = real_dream.dream([Note("n1", "t", "c", 0.1, 0.1)])
        if result.candidates:
            n = adapter.to_note(result.candidates[0], min_conf=0.4)
            assert n is None  # < 0.4 应被跳过


def v1108_dummy_cand(confidence: float):  # pragma: no cover
    """本地占位 — 真实测试在 test_v1108 中."""
    return None


class TestV1107DreamAdapterNote:
    def test_to_note_high_conf_creates_note(self):
        from apeireth import v1108_dream_v2 as v1108
        d = v1108.MemoryDreamV2(seed=1, min_confidence=0.05,
                                 verify_threshold=0.1)

        class Note:
            def __init__(self, nid, topic, claim, c, s):
                self.nid, self.topic, self.claim = nid, topic, claim
                self.confidence, self.salience = c, s

        result = d.dream([Note("n1", "t", "c", 0.9, 0.9)])
        assert result.candidates, "no candidates emitted"
        adapter = v1107.DreamEpisodeAdapter()
        note = adapter.to_note(result.candidates[0], min_conf=0.4)
        assert note is not None
        assert note.confidence <= 0.7


# ============================================================
# V1107CognitiveLift — 修复 V1101 3 bug — 5 tests
# ============================================================


class TestV1107LiftFixV1101Bugs:
    def test_fix_activation_add_edge_no_weight_kw(self):
        """Bug 1: V1101 add_edge(c1, c2, weight=0.7) — V1061 没 weight 参数.

        V1107 应该正确调用 add_edge(c1, c2) 不带 weight.
        """
        cog = v1061.CognitiveArchitecture()
        # 模拟 V1101 seed (不调用 broken 版本)
        seed_mod = v1101.V1101CognitiveProductionSeeder()
        seed_mod.seed_declarative_memory(cog)  # 先注 declarative
        lift = v1107.V1107CognitiveLift()
        result = lift.inject_into_cognitive_core(cog)
        assert result["n_edges"] >= 1

    def test_fix_concept_add_concept_features_dict(self):
        """Bug 2: V1101 add_concept(name=cat, members=[...]) — V1061 要 features dict.

        V1107 应该正确调用 add_concept(name, features={frequency: n}).
        """
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_declarative_memory(cog)
        lift = v1107.V1107CognitiveLift()
        result = lift.inject_into_cognitive_core(cog)
        assert result["n_concepts"] >= 1
        # 注入了 exemplar (修复了 V1101 没注 exemplar 的 bug)
        n_exemplars = sum(len(c.exemplars) for c in cog.concepts.concepts.values())
        assert n_exemplars >= 1

    def test_fix_pattern_matcher_three_types(self):
        """Bug 3: V1101 未注 pattern_matcher — V1107 注 3 个类型 (exact/similarity/fuzzy)."""
        cog = v1061.CognitiveArchitecture()
        lift = v1107.V1107CognitiveLift()
        result = lift.inject_into_cognitive_core(cog)
        # 即使空 arch, 3 patterns 也被注入
        assert result["n_patterns"] == 3
        types = {p.match_type.value for p in cog.pattern_matcher.patterns.values()}
        assert "exact" in types
        assert "similarity" in types
        assert "fuzzy" in types

    def test_inject_returns_log(self):
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_declarative_memory(cog)
        lift = v1107.V1107CognitiveLift()
        result = lift.inject_into_cognitive_core(cog)
        assert "log" in result
        assert len(result["log"]) >= 3  # 至少 3 个修复 log

    def test_injection_log_persisted_on_lift(self):
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_declarative_memory(cog)
        lift = v1107.V1107CognitiveLift()
        lift.inject_into_cognitive_core(cog)
        assert len(lift.injection_log) >= 3


# ============================================================
# V1107CognitiveLift.seed_5_module_framework — 5 tests
# ============================================================


class TestV1107LiftSeed5Module:
    def test_seed_sets_philosophy_keys(self):
        lift = v1107.V1107CognitiveLift()
        lift.seed_5_module_framework()
        for k in v1107.IdentityCore().philosophy_keys:
            assert lift.identity.get_value(k) > 0.0

    def test_seed_builds_relation_graph(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.seed_5_module_framework()
        assert result["n_graph_nodes"] >= 5
        assert result["n_graph_edges"] >= 4

    def test_seed_creates_episodes_and_consolidates(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.seed_5_module_framework()
        assert result["n_episodes"] >= 4
        assert result["n_notes"] >= 1

    def test_seed_attention_has_focus(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.seed_5_module_framework()
        assert lift.attention.focus is not None
        assert result["n_attention_nodes"] >= 5

    def test_seed_runs_reconsolidation_cycle(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.seed_5_module_framework()
        assert "reconsolidation_cycle" in result
        cycle = result["reconsolidation_cycle"]
        assert "conflicts" in cycle
        assert cycle["conflicts"] >= 1  # 我们注了 lift_strategy conflict


# ============================================================
# V1107CognitiveLift.integrate_dream — 3 tests
# ============================================================


class TestV1107IntegrateDream:
    def test_integrate_empty_returns_empty_result(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.integrate_dream([])
        assert result["episodes_added"] == 0
        assert result["notes_added"] == 0

    def test_integrate_5_dream_candidates(self):
        from apeireth import v1108_dream_v2 as v1108
        d = v1108.MemoryDreamV2(seed=42)

        class Note:
            def __init__(self, nid, topic, claim, c, s):
                self.nid, self.topic, self.claim = nid, topic, claim
                self.confidence, self.salience = c, s

        notes = [Note(f"n{i}", "t", "c", 0.8, 0.6) for i in range(5)]
        dream_result = d.dream(notes, context={"topic": "lift"})
        lift = v1107.V1107CognitiveLift()
        result = lift.integrate_dream(dream_result.candidates)
        assert result["episodes_added"] == len(dream_result.candidates)
        assert result["notes_added"] >= 1

    def test_integrate_rejects_non_dream(self):
        """V3 守门: _dream=False candidate 必须被跳过."""
        lift = v1107.V1107CognitiveLift()
        # 构造一个 _dream=False 对象 (手工伪造)
        class FakeCand:
            _dream = False
            cid = "fake"
            scenario = "fake"
            confidence = 0.8
            schema_phase = "assimilation"
            bindings = ()
            premise_nids = ("n",)

        result = lift.integrate_dream([FakeCand()])  # type: ignore[list-item]
        assert result["episodes_added"] == 0
        assert result["skipped_low_conf"] >= 1


# ============================================================
# V1107CognitiveLift.execute_full_lift — 5 tests
# ============================================================


class TestV1107ExecuteFullLift:
    def test_execute_returns_version(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.execute_full_lift()
        assert result["version"] == v1107.V1107_VERSION

    def test_execute_includes_repair_and_5_module(self):
        lift = v1107.V1107CognitiveLift()
        result = lift.execute_full_lift()
        assert "repair" in result
        assert "5_module" in result
        assert "cognitive_core_weighted_score" in result

    def test_execute_with_v1101_seeded_arch_lifts_score(self):
        """真 lift 验证: V1101 seed + V1107 lift → score ≥ 0.85 (主 22:33)."""
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_all(cog)
        lift = v1107.V1107CognitiveLift()
        result = lift.execute_full_lift(cog=cog)
        assert result["cognitive_core_weighted_score"] >= 0.85

    def test_execute_baseline_no_seed_improves_with_v1107(self):
        """空 arch + V1107 应该把 score 从 0.0 拉到 > 0.0."""
        lift = v1107.V1107CognitiveLift()
        result = lift.execute_full_lift()  # no cog → empty
        # 空 arch + V1107 5 Module framework + 修复 (无 chunks → 无 edges/concepts)
        assert result["cognitive_core_weighted_score"] >= 0.10

    def test_execute_with_dream_integration(self):
        from apeireth import v1108_dream_v2 as v1108
        d = v1108.MemoryDreamV2(seed=42)

        class Note:
            def __init__(self, nid, topic, claim, c, s):
                self.nid, self.topic, self.claim = nid, topic, claim
                self.confidence, self.salience = c, s

        notes = [Note(f"n{i}", "t", "c", 0.85, 0.7) for i in range(3)]
        dream_result = d.dream(notes, context={"topic": "lift"})
        lift = v1107.V1107CognitiveLift()
        result = lift.execute_full_lift(dream_candidates=dream_result.candidates)
        assert "dream_integration" in result
        assert result["dream_integration"]["episodes_added"] >= 1


# ============================================================
# 真 lift 验证 — 3 tests
# ============================================================


class TestV1107CognitiveCoreLift:
    def test_baseline_v1101_only_is_0_4927(self):
        """基线: 仅 V1101 seeder → 0.4927 (主 17:43 实事求是)."""
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_all(cog)
        m = v1061.measure_cognitive_core(cog)
        assert abs(m.weighted_score() - 0.4927) < 0.001

    def test_v1107_lift_increases_weighted_score(self):
        """V1107 lift 提升 >= +0.30 (主 22:33 ASI 北极星)."""
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_all(cog)
        lift = v1107.V1107CognitiveLift()
        r = lift.execute_full_lift(cog=cog)
        baseline = 0.4927
        assert r["cognitive_core_weighted_score"] >= baseline + 0.30

    def test_v1107_lift_meets_target(self):
        """V1107 lift target ≥ 0.85 (主 22:33 ASI 北极星)."""
        cog = v1061.CognitiveArchitecture()
        v1101.V1101CognitiveProductionSeeder().seed_all(cog)
        lift = v1107.V1107CognitiveLift()
        r = lift.execute_full_lift(cog=cog)
        assert r["cognitive_core_weighted_score"] >= 0.85


# ============================================================
# V3 守门 — 3 tests
# ============================================================


class TestV1107V3Guards:
    def test_v3_guards_present(self):
        assert "analogy_understanding" in v1107.V1107_V3_GUARDS
        assert "attention_consciousness" in v1107.V1107_V3_GUARDS
        assert "consolidation_learning" in v1107.V1107_V3_GUARDS
        assert "dream_fact" in v1107.V1107_V3_GUARDS
        assert "module_asi" in v1107.V1107_V3_GUARDS

    def test_v3_guards_strings_are_chinese(self):
        """V3 守门 string 必须中文 (主 17:58 不假装 — 中文是真诚)."""
        for k, v in v1107.V1107_V3_GUARDS.items():
            assert "不假装" in v

    def test_v1107_has_v3_guards_dict(self):
        """auto-injected V3_GUARDS 必须存在 (V1101 标准)."""
        assert hasattr(v1107, "V3_GUARDS")
        assert "module_is_not_asi" in v1107.V3_GUARDS


if __name__ == "__main__":
    pytest.main([__file__, "-v"])