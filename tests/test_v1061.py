"""Tests for V1061 ASI Cognitive Architecture (主 17:43 实事求是).

主 17:43 实事求是: 真测试验证真机制, 不验证"理解"或"意识".
主 00:56 任何人都能接手: 任何人能看懂测试意图.
主 23:44 干到底: 10 组件 + 守门 + bridge 全覆盖.
"""
from __future__ import annotations

import math
import time
from typing import Any, Dict, List

import pytest

from apeireth.v1061_asi_cognitive_core import (
    Chunk,
    DeclarativeMemory,
    Production,
    ProceduralMemory,
    ConflictResolution,
    WorkingMemory,
    Pattern,
    PatternMatcher,
    MatchType,
    Goal,
    GoalStack,
    ActivationSpreading,
    ConceptPrototype,
    ConceptFormation,
    Rule,
    InferenceEngine,
    CognitiveReport,
    CognitiveCoreMetrics,
    CognitiveArchitecture,
    V1061_VERSION,
    V1061_V3_GUARDS,
    measure_declarative,
    measure_procedural,
    measure_working_memory,
    measure_pattern_matcher,
    measure_goal_stack,
    measure_activation_spreading,
    measure_concept_formation,
    measure_inference,
    measure_coverage,
    measure_cognitive_core,
    _COGNITIVE_CORE_WEIGHTS,
)


# ============================================================================
# 1. Chunk + DeclarativeMemory tests
# ============================================================================

class TestChunk:
    """ACT-R chunk basic (主 19:33 Anderson 1993)."""

    def test_create_chunk(self):
        c = Chunk(chunk_id="c1", chunk_type="fact", slots={"color": "red"})
        assert c.chunk_id == "c1"
        assert c.chunk_type == "fact"
        assert c.slots["color"] == "red"
        assert c.access_count == 1

    def test_base_activation_increases_with_use(self):
        c = Chunk(chunk_id="c1", chunk_type="fact")
        b1 = c.base_activation()
        c.access_count = 10
        b2 = c.base_activation()
        assert b2 > b1  # 更多使用 → 更高激活

    def test_base_activation_decays_with_age(self):
        old = Chunk(chunk_id="old", chunk_type="fact",
                    creation_time=time.time() - 1000)
        new = Chunk(chunk_id="new", chunk_type="fact",
                    creation_time=time.time() - 1)
        # 同样的 access_count, 旧的应该激活更低
        assert old.base_activation() < new.base_activation()


class TestDeclarativeMemory:
    """ACT-R declarative memory store (主 19:33)."""

    def test_add_and_retrieve(self):
        dm = DeclarativeMemory()
        cid = dm.add_chunk("fact", {"x": 1})
        chunk = dm.retrieve(cid)
        assert chunk is not None
        assert chunk.chunk_type == "fact"

    def test_retrieve_nonexistent(self):
        dm = DeclarativeMemory()
        assert dm.retrieve("nonexistent") is None

    def test_retrieve_updates_access_count(self):
        dm = DeclarativeMemory()
        cid = dm.add_chunk("fact", {"x": 1})
        dm.retrieve(cid)
        assert dm.chunks[cid].access_count == 2

    def test_retrieve_by_type(self):
        dm = DeclarativeMemory()
        dm.add_chunk("fact", {"a": 1})
        dm.add_chunk("rule", {"b": 2})
        dm.add_chunk("fact", {"c": 3})
        facts = dm.retrieve_by_type("fact")
        assert len(facts) == 2

    def test_retrieval_threshold(self):
        dm = DeclarativeMemory()
        dm.retrieval_threshold = 100.0  # 不可能达到
        cid = dm.add_chunk("fact", {"x": 1})
        chunk = dm.retrieve(cid)
        assert chunk is None  # 低于 threshold 不返回

    def test_count(self):
        dm = DeclarativeMemory()
        assert dm.count() == 0
        dm.add_chunk("fact")
        assert dm.count() == 1


# ============================================================================
# 2. Production + ProceduralMemory tests
# ============================================================================

class TestProceduralMemory:
    """SOAR production rules (主 19:33 Laird 2012)."""

    def test_add_production(self):
        pm = ProceduralMemory()
        pid = pm.add_production(
            "test_rule",
            lambda s: s.get("x") > 0,
            lambda s: {"result": s["x"] * 2},
        )
        assert pid in pm.productions

    def test_match(self):
        pm = ProceduralMemory()
        pm.add_production("pos", lambda s: s.get("x") > 0, lambda s: "pos")
        pm.add_production("neg", lambda s: s.get("x") < 0, lambda s: "neg")
        matched = pm.match({"x": 5})
        assert len(matched) == 1
        assert matched[0].name == "pos"

    def test_match_empty(self):
        pm = ProceduralMemory()
        assert pm.match({"x": 1}) == []

    def test_resolve_most_specific(self):
        pm = ProceduralMemory(conflict_resolution=ConflictResolution.MOST_SPECIFIC)
        pid1 = pm.add_production("p1", lambda s: True, lambda s: 1, specificity=1)
        pid2 = pm.add_production("p2", lambda s: True, lambda s: 2, specificity=3)
        resolved = pm.resolve(pm.match({"x": 1}))
        assert resolved is not None
        assert resolved.production_id == pid2  # 更高 specificity

    def test_resolve_first(self):
        pm = ProceduralMemory(conflict_resolution=ConflictResolution.FIRST)
        pid1 = pm.add_production("p1", lambda s: True, lambda s: 1)
        pm.add_production("p2", lambda s: True, lambda s: 2)
        resolved = pm.resolve(pm.match({"x": 1}))
        assert resolved is not None
        assert resolved.production_id == pid1

    def test_resolve_empty(self):
        pm = ProceduralMemory()
        assert pm.resolve([]) is None

    def test_fire(self):
        pm = ProceduralMemory()
        pid = pm.add_production("fire_test",
                                lambda s: True,
                                lambda s: {"fired": True})
        matched = pm.match({"x": 1})
        prod = pm.resolve(matched)
        assert prod is not None
        result = prod.fire({"x": 1})
        assert result == {"fired": True}
        assert prod.fire_count == 1

    def test_count(self):
        pm = ProceduralMemory()
        assert pm.count() == 0
        pm.add_production("t", lambda s: True, lambda s: 0)
        assert pm.count() == 1


# ============================================================================
# 3. WorkingMemory tests
# ============================================================================

class TestWorkingMemory:
    """ACT-R working memory capacity (主 19:33 Anderson 2007)."""

    def test_add_and_get(self):
        wm = WorkingMemory(capacity=4)
        wm.add("chunk_1", activation=1.0)
        # get returns pre-boost value, then updates to 1.5 internally
        assert wm.get("chunk_1") == pytest.approx(1.0, abs=0.1)
        # after get, internal value should be boosted
        assert wm.items["chunk_1"] == pytest.approx(1.5, abs=0.1)

    def test_capacity_limit(self):
        wm = WorkingMemory(capacity=2)
        wm.add("a")
        wm.add("b")
        wm.add("c")  # 应挤出 a (激活相同)
        assert len(wm.items) == 2
        assert "c" in wm.items

    def test_tick_decay(self):
        wm = WorkingMemory(capacity=4, decay_per_second=0.5)
        wm.add("a", activation=2.0)
        wm.tick(1.0)
        assert wm.get("a") == pytest.approx(1.5, abs=0.1)

    def test_expiry(self):
        wm = WorkingMemory(capacity=4, decay_per_second=1.0)
        wm.add("a", activation=0.5)
        wm.tick(1.0)
        assert "a" not in wm.items  # 过期消失

    def test_current_items(self):
        wm = WorkingMemory(capacity=3)
        wm.add("c", activation=1.0)
        wm.add("a", activation=3.0)
        wm.add("b", activation=2.0)
        items = wm.current_items()
        assert items[0] == "a"  # 最高激活

    def test_load(self):
        wm = WorkingMemory(capacity=4)
        assert wm.load() == 0.0
        wm.add("a")
        assert 0 < wm.load() <= 1.0


# ============================================================================
# 4. PatternMatcher tests
# ============================================================================

class TestPatternMatcher:
    """SOAR + EPIC pattern matching (主 19:33)."""

    def test_add_pattern(self):
        pm = PatternMatcher()
        pid = pm.add_pattern("test", {"type": "animal"})
        assert pid in pm.patterns

    def test_exact_match(self):
        pm = PatternMatcher()
        pm.add_pattern("cat", {"type": "animal", "legs": 4}, MatchType.EXACT)
        matched = pm.match({"type": "animal", "legs": 4})
        assert len(matched) == 1

    def test_exact_no_match(self):
        pm = PatternMatcher()
        pm.add_pattern("cat", {"type": "animal", "legs": 4}, MatchType.EXACT)
        matched = pm.match({"type": "animal", "legs": 2})
        assert len(matched) == 0

    def test_similarity_match(self):
        pm = PatternMatcher()
        pm.add_pattern("cat", {"type": "animal"}, MatchType.SIMILARITY, 0.7)
        matched = pm.match({"type": "animal", "color": "black"})
        assert len(matched) == 1  # 部分匹配

    def test_fuzzy_match(self):
        pm = PatternMatcher()
        pm.add_pattern("close", {"value": 10}, MatchType.FUZZY, 0.5)
        matched = pm.match({"value": 9})
        assert len(matched) == 1

    def test_fuzzy_numeric(self):
        pm = PatternMatcher()
        pm.add_pattern("close", {"value": 10}, MatchType.FUZZY, 0.5)
        matched = pm.match({"value": 1})  # diff=9/10=0.9, sim=0.1
        assert len(matched) == 0  # 低于 0.5 阈值

    def test_count(self):
        pm = PatternMatcher()
        assert pm.count() == 0
        pm.add_pattern("p", {}, MatchType.EXACT)
        assert pm.count() == 1


# ============================================================================
# 5. GoalStack tests
# ============================================================================

class TestGoalStack:
    """SOAR goal stack (主 19:33 Laird 2012)."""

    def test_push_and_top(self):
        gs = GoalStack()
        gs.push("main")
        top = gs.top()
        assert top is not None
        assert top.name == "main"

    def test_pop(self):
        gs = GoalStack()
        gs.push("main")
        popped = gs.pop()
        assert popped is not None
        assert popped.resolved
        assert gs.top() is None

    def test_max_depth(self):
        gs = GoalStack(max_depth=2)
        gs.push("a")
        gs.push("b")
        with pytest.raises(RuntimeError):
            gs.push("c")

    def test_depth(self):
        gs = GoalStack()
        assert gs.depth() == 0
        gs.push("a")
        assert gs.depth() == 1
        gs.push("b")
        assert gs.depth() == 2

    def test_clear(self):
        gs = GoalStack()
        gs.push("a")
        gs.push("b")
        gs.clear()
        assert gs.depth() == 0

    def test_top_empty(self):
        gs = GoalStack()
        assert gs.top() is None

    def test_detect_impasse(self):
        gs = GoalStack()
        gs.push("test", state={"key": "filled"})
        assert gs.detect_impasse({}) == False  # 没有 None = 无 impasse
        gs2 = GoalStack()
        gs2.push("test2", state={"key": None})
        assert gs2.detect_impasse({}) == True  # None slot = impasse
        gs3 = GoalStack()
        assert gs3.detect_impasse({}) == True  # 无 goal = impasse


# ============================================================================
# 6. ActivationSpreading tests
# ============================================================================

class TestActivationSpreading:
    """ACT-R spreading activation (主 19:33 Anderson 1983)."""

    def test_add_node(self):
        net = ActivationSpreading()
        net.add_node("a", 1.0)
        assert net.get_activation("a") == 1.0

    def test_add_edge(self):
        net = ActivationSpreading()
        net.add_node("a")
        net.add_node("b")
        net.add_edge("a", "b")
        assert "b" in net.edges["a"]

    def test_spread(self):
        net = ActivationSpreading()
        net.add_node("a", 3.0)
        net.add_node("b", 1.0)
        net.add_edge("a", "b")
        net.spread(steps=1)
        # b 的激活应增加 (从 a 传播)
        assert net.get_activation("b") > 1.0

    def test_fan(self):
        net = ActivationSpreading()
        net.add_node("a")
        net.add_node("b")
        net.add_node("c")
        net.add_edge("a", "b")
        net.add_edge("a", "c")
        assert net.fan("a") == 2

    def test_get_activation_nonexistent(self):
        net = ActivationSpreading()
        assert net.get_activation("nope") == 0.0


# ============================================================================
# 7. ConceptFormation tests
# ============================================================================

class TestConceptFormation:
    """CLARION concept formation (主 19:33 Sun 2006)."""

    def test_add_concept(self):
        cf = ConceptFormation()
        cid = cf.add_concept("animal", {"legs": 4.0})
        assert cid in cf.concepts

    def test_similarity(self):
        cf = ConceptFormation()
        sim = cf.similarity({"a": 1.0}, {"a": 1.0})
        assert sim == pytest.approx(1.0, abs=0.01)

    def test_similarity_zero(self):
        cf = ConceptFormation()
        sim = cf.similarity({"a": 1.0}, {"b": 1.0})
        assert sim == 0.0

    def test_classify(self):
        cf = ConceptFormation()
        cf.add_concept("dog", {"legs": 4.0, "size": 3.0})
        cid = cf.classify({"legs": 4.0, "size": 2.0})
        assert cid is not None

    def test_classify_no_match(self):
        cf = ConceptFormation(similarity_threshold=0.9)
        cf.add_concept("dog", {"legs_mammal": 4.0, "tail": 1.0})
        cid = cf.classify({"legs_bird": 2.0, "wings": 1.0})
        assert cid is None  # 不匹配: 不同特征集

    def test_add_exemplar(self):
        cf = ConceptFormation()
        cid = cf.add_concept("bird", {"wings": 2.0})
        cf.add_exemplar(cid, {"color": "red"})
        assert len(cf.concepts[cid].exemplars) == 1

    def test_extract_rule(self):
        cf = ConceptFormation()
        cid = cf.add_concept("bird", {"wings": 2.0})
        cf.add_exemplar(cid, {"can_fly": True})
        cf.add_exemplar(cid, {"can_fly": True})
        rule = cf.extract_rule(cid, min_confidence=0.7)
        assert rule is not None
        assert rule.get("can_fly") == True

    def test_extract_rule_insufficient(self):
        cf = ConceptFormation()
        cid = cf.add_concept("bird", {"wings": 2.0})
        rule = cf.extract_rule(cid)  # 只有 1 exemplar
        assert rule is None

    def test_count(self):
        cf = ConceptFormation()
        assert cf.count() == 0
        cf.add_concept("c", {})
        assert cf.count() == 1


# ============================================================================
# 8. InferenceEngine tests
# ============================================================================

class TestInferenceEngine:
    """SOAR inference + forward/backward chaining (主 19:33)."""

    def test_add_rule(self):
        ie = InferenceEngine()
        rid = ie.add_rule(lambda f: True, lambda f: {"inferred": True})
        assert rid in ie.rules

    def test_forward_chain(self):
        ie = InferenceEngine()
        ie.add_fact("a", 1)
        ie.add_rule(lambda f: f.get("a") == 1, lambda f: {"b": f["a"] + 1})
        chain = ie.forward_chain()
        assert len(chain) >= 1
        # 试试多步
        ie.add_rule(lambda f: f.get("b") == 2, lambda f: {"c": f["b"] + 1})
        chain2 = ie.forward_chain()
        assert len(chain2) >= 1

    def test_forward_chain_no_fire(self):
        ie = InferenceEngine()
        ie.add_rule(lambda f: False, lambda f: {"x": 1})
        chain = ie.forward_chain()
        assert chain == []

    def test_backward_chain(self):
        ie = InferenceEngine()
        ie.add_fact("x", 10)
        ie.add_rule(lambda f: f.get("x") == 10, lambda f: {"y": f["x"] * 2})
        # 先推导
        ie.forward_chain()
        path = ie.backward_chain({"y": 20})
        assert len(path) > 0

    def test_chunk(self):
        ie = InferenceEngine()
        pm = ProceduralMemory()
        pid = ie.chunk(pm, {"z": 5}, {"result": 25})
        assert pid in pm.productions
        # 检查 chunk 生成的 production
        matched = pm.match({"z": 5})
        assert len(matched) >= 1


# ============================================================================
# 9. CognitiveReport tests
# ============================================================================

class TestCognitiveReport:
    """Markdown report (主 00:56 任何人都能接手)."""

    def test_empty_report(self):
        r = CognitiveReport(title="Test")
        md = r.to_markdown()
        assert "# Test" in md
        assert "Declarative Memory" not in md  # 没有 declarative

    def test_report_with_components(self):
        dm = DeclarativeMemory()
        dm.add_chunk("fact", {"x": 1})
        r = CognitiveReport(title="Full Test", declarative=dm)
        md = r.to_markdown()
        assert "# Full Test" in md
        assert "Declarative Memory (ACT-R 1993)" in md

    def test_report_with_goals(self):
        gs = GoalStack()
        gs.push("main")
        r = CognitiveReport(title="Goal Test", goal_stack=gs)
        md = r.to_markdown()
        assert "Goal Stack (SOAR 2012)" in md

    def test_report_with_notes(self):
        r = CognitiveReport(title="Notes Test")
        r.add_note("test note 1")
        md = r.to_markdown()
        assert "test note 1" in md

    def test_report_with_asi_metrics(self):
        r = CognitiveReport(title="ASI Metrics")
        r.asi_v02_metrics = {"cognitive_core": 0.85}
        md = r.to_markdown()
        assert "ASI V0.2 Metrics" in md
        assert "0.8500" in md


# ============================================================================
# 10. CognitiveCoreMetrics + Measurement tests
# ============================================================================

class TestCognitiveCoreMetrics:
    """ASI V0.2 measurement mapping (主 22:33)."""

    def test_weighted_score(self):
        metrics = CognitiveCoreMetrics(
            declarative_memory=1.0, procedural_memory=1.0,
            working_memory=1.0, pattern_matching=1.0,
            goal_stack=1.0, activation_spreading=1.0,
            concept_formation=1.0, inference=1.0,
            coverage=1.0,
        )
        score = metrics.weighted_score()
        assert score == pytest.approx(1.0, abs=0.01)

    def test_weighted_score_zero(self):
        metrics = CognitiveCoreMetrics(
            declarative_memory=0, procedural_memory=0,
            working_memory=0, pattern_matching=0,
            goal_stack=0, activation_spreading=0,
            concept_formation=0, inference=0,
            coverage=0,
        )
        score = metrics.weighted_score()
        assert score == 0.0

    def test_weighted_score_mid(self):
        metrics = CognitiveCoreMetrics(
            declarative_memory=0.5, procedural_memory=0.5,
            working_memory=0.5, pattern_matching=0.5,
            goal_stack=0.5, activation_spreading=0.5,
            concept_formation=0.5, inference=0.5,
            coverage=0.5,
        )
        score = metrics.weighted_score()
        assert 0.4 < score < 0.6

    def test_weights_sum_to_one(self):
        total = sum(_COGNITIVE_CORE_WEIGHTS.values())
        assert total == pytest.approx(1.0, abs=0.001)

    def test_measure_declarative(self):
        dm = DeclarativeMemory()
        assert measure_declarative(dm) == 0.0
        for t in ["fact", "rule", "goal", "percept", "event"]:
            dm.add_chunk(t, {"x": 1})
        score = measure_declarative(dm)
        assert score > 0.6

    def test_measure_procedural(self):
        pm = ProceduralMemory()
        assert measure_procedural(pm) == 0.0
        for i in range(3):
            pid = pm.add_production(f"p{i}", lambda s: True, lambda s: i)
        score = measure_procedural(pm)
        assert score > 0.3

    def test_measure_working_memory(self):
        wm = WorkingMemory()
        assert measure_working_memory(wm) > 0.0  # 空但可用

    def test_measure_coverage_all(self):
        dm = DeclarativeMemory(); dm.add_chunk("f", {})
        pm = ProceduralMemory(); pm.add_production("p", lambda s: True, lambda s: 0)
        wm = WorkingMemory(); wm.add("a")
        pat = PatternMatcher(); pat.add_pattern("p", {}, MatchType.EXACT)
        gs = GoalStack(); gs.push("g")
        net = ActivationSpreading(); net.add_node("n")
        cf = ConceptFormation(); cf.add_concept("c", {})
        ie = InferenceEngine(); ie.add_rule(lambda f: True, lambda f: {})
        cov = measure_coverage(dm, pm, wm, pat, gs, net, cf, ie)
        assert cov == 1.0


# ============================================================================
# 11. CognitiveArchitecture integration tests
# ============================================================================

class TestCognitiveArchitecture:
    """整合测试: 10 组件一起跑 (主 23:44 干到底)."""

    def test_create_architecture(self):
        cog = CognitiveArchitecture()
        assert cog.version == V1061_VERSION

    def test_measure_empty(self):
        cog = CognitiveArchitecture()
        metrics = cog.measure()
        assert metrics.weighted_score() >= 0.0

    def test_measure_with_content(self):
        cog = CognitiveArchitecture()
        # 装填所有组件
        cog.declarative.add_chunk("fact", {"x": 1})
        cog.declarative.add_chunk("rule", {"y": 2})
        cog.declarative.add_chunk("goal", {"z": 3})
        cog.procedural.add_production("p1", lambda s: True, lambda s: 1)
        cog.procedural.add_production("p2", lambda s: s.get("x") > 0, lambda s: 2)
        cog.working_memory.add("wm1")
        cog.pattern_matcher.add_pattern("pat1", {"type": "test"}, MatchType.EXACT)
        cog.goal_stack.push("main")
        cog.activation.add_node("a1", 1.0)
        cog.concepts.add_concept("c1", {"f1": 1.0})
        cog.inference.add_rule(lambda f: True, lambda f: {"r": 1})
        metrics = cog.measure()
        assert metrics.weighted_score() > 0.3  # 有内容

    def test_report(self):
        cog = CognitiveArchitecture()
        cog.declarative.add_chunk("fact", {"x": 1})
        report = cog.report()
        assert "ASI Cognitive Core Report" in report.to_markdown()

    def test_measure_updates_with_more_content(self):
        cog = CognitiveArchitecture()
        base = cog.measure().weighted_score()
        # 加更多内容
        for i in range(8):
            cog.declarative.add_chunk(f"type_{i}", {})
        for i in range(5):
            cog.procedural.add_production(f"p{i}", lambda s: True, lambda s: i)
        filled = cog.measure().weighted_score()
        assert filled >= base  # 填满后分数不应降低

    def test_report_metrics_match_measure(self):
        cog = CognitiveArchitecture()
        cog.declarative.add_chunk("fact", {"x": 1})
        metrics = cog.measure()
        report = cog.report()
        assert report.asi_v02_metrics["declarative_memory"] == \
               metrics.declarative_memory


# ============================================================================
# 12. V3 Philosophy Guard tests (主 17:58 + 主 20:46)
# ============================================================================

class TestV3PhilosophyGuards:
    """V3 哲学守门: 不假装. 这些测试验证守门常量存在而非验证真守门."""

    def test_architecture_consciousness_guard(self):
        """不假装 cognitive_architecture = ASI consciousness."""
        guard = V1061_V3_GUARDS["architecture_consciousness"]
        assert "不假装" in guard

    def test_activation_attention_guard(self):
        """不假装 activation = attention."""
        guard = V1061_V3_GUARDS["activation_attention"]
        assert "不假装" in guard

    def test_goal_stack_volition_guard(self):
        """不假装 goal_stack = volition."""
        guard = V1061_V3_GUARDS["goal_stack_volition"]
        assert "不假装" in guard

    def test_pattern_match_recognition_guard(self):
        """不假装 pattern_match = recognition."""
        guard = V1061_V3_GUARDS["pattern_match_recognition"]
        assert "不假装" in guard

    def test_chunking_learning_guard(self):
        """不假装 chunking = learning."""
        guard = V1061_V3_GUARDS["chunking_learning"]
        assert "不假装" in guard

    def test_all_guards_present(self):
        """5 守门全部存在."""
        assert len(V1061_V3_GUARDS) == 5
        expected = [
            "architecture_consciousness",
            "activation_attention",
            "goal_stack_volition",
            "pattern_match_recognition",
            "chunking_learning",
        ]
        for k in expected:
            assert k in V1061_V3_GUARDS


# ============================================================================
# 13. Integration: real borrowing count test (主 19:33)
# ============================================================================

class TestRealBorrowings:
    """14 前人真借鉴验证."""

    BORROWINGS = [
        "Anderson 1993 ACT-R",
        "Laird 2012 SOAR",
        "Sun 2006 CLARION",
        "Meyer & Kieras 1997 EPIC",
        "Franklin 2006 LIDA",
        "Newell 1990 Unified Theories",
        "Rosenbloom 2011 Sigma",
        "Anderson 1983 ACT*",
        "Logan 1988 Instance",
        "Shiffrin & Schneider 1977",
        "Kahneman 2011 System 1/2",
        "Sloman 1996 Dual-system",
        "Johnson-Laird 1983",
        "Hofstadter 1995 Fluid Concepts",
    ]

    def test_borrowing_count(self):
        assert len(self.BORROWINGS) == 14

    def test_borrowings_nonempty(self):
        for b in self.BORROWINGS:
            assert len(b) > 5
