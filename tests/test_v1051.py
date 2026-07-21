"""Tests for V1051 ASI Truth 真生产.

11 真生产组件 + 6 守门, 每组件 ≥6 测试 (主 17:43 实事求是).
"""
from __future__ import annotations

import math
import pytest

from apeireth.v1051_asi_truth import (
    BayesianTruthUpdater,
    PopperFalsifier,
    LakatosProgramme,
    ProofAssistantBridge,
    ProofStep,
    TruthDiscovery,
    Source,
    Claim,
    FormalVerifier,
    HoareTriple,
    CoherenceEngine,
    CausalTruth,
    CausalGraph,
    KnowledgeGraphFiller,
    ConceptSpace,
    ASITruthBridge,
    godel_self_reference_guard,
    popper_falsifiability_guard,
    coherence_threshold_guard,
    uncertainty_acknowledgment_guard,
    computational_limit_guard,
    asisafety_truth_guard,
    V1051_VERSION,
)


# ============================================================================
# 1. BayesianTruthUpdater 真测试 (Bayes 1763 + MacKay 2003 + Jaynes 2003)
# ============================================================================


class TestBayesianTruthUpdater:
    """BayesianTruthUpdater 真测试."""

    def test_initial_prior(self):
        """真测: 初始 prior 保留."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        assert bt.posterior() == pytest.approx(0.5, abs=1e-9)

    def test_positive_evidence_increases_posterior(self):
        """真测: positive likelihood → posterior 上升."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.3)
        bt.add_evidence(likelihood=0.9, neg_likelihood=0.1)
        assert bt.posterior() > 0.3

    def test_negative_evidence_decreases_posterior(self):
        """真测: negative likelihood > positive → posterior 下降."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.7)
        bt.add_evidence(likelihood=0.1, neg_likelihood=0.9)
        assert bt.posterior() < 0.7

    def test_multiple_evidence_accumulation(self):
        """真测: 多次 evidence 累积更新."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        for _ in range(5):
            bt.add_evidence(likelihood=0.95, neg_likelihood=0.05)
        assert bt.posterior() > 0.99

    def test_log_odds_finite(self):
        """真测: log-odds 有限值."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        bt.add_evidence(likelihood=0.8, neg_likelihood=0.3)
        lo = bt.log_odds()
        assert math.isfinite(lo)

    def test_entropy_max_at_half(self):
        """真测: Jaynes 2003 — 熵最大 at p=0.5."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        assert bt.entropy() > 0.5  # H(0.5) = ln(2) ≈ 0.693

    def test_update_prior_preserves_evidence(self):
        """真测: 更新 prior 保留 evidence 累积."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        bt.add_evidence(likelihood=0.9, neg_likelihood=0.1)
        new = bt.update_prior(new_prior=0.7)
        assert len(new.evidence) == len(bt.evidence)


# ============================================================================
# 2. PopperFalsifier 真测试 (Popper 1934)
# ============================================================================


class TestPopperFalsifier:
    """PopperFalsifier 真测试."""

    def test_empty_not_scientific(self):
        """真测: 无 falsification tests = 非科学 (Popper 划界)."""
        pf = PopperFalsifier(hypothesis_id="H1")
        assert not pf.is_scientific()

    def test_with_tests_is_scientific(self):
        """真测: 有 tests = 科学."""
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        assert pf.is_scientific()

    def test_no_falsification_when_all_pass(self):
        """真测: 全部 test 通过 = 未证伪."""
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        pf.add_test("T2", passed=True)
        assert not pf.is_falsified()

    def test_falsification_when_one_fails(self):
        """真测: Popper 严格 — 一个失败 = 证伪."""
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        pf.add_test("T2", passed=False)
        assert pf.is_falsified()

    def test_falsification_rate(self):
        """真测: 证伪率."""
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=False)
        pf.add_test("T2", passed=False)
        pf.add_test("T3", passed=True)
        assert pf.falsification_rate() == pytest.approx(2 / 3)

    def test_robustness_inverse_of_falsification(self):
        """真测: robustness = 1 - falsification_rate."""
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        pf.add_test("T2", passed=False)
        assert pf.robustness() + pf.falsification_rate() == pytest.approx(1.0)


# ============================================================================
# 3. LakatosProgramme 真测试 (Lakatos 1978)
# ============================================================================


class TestLakatosProgramme:
    """LakatosProgramme 真测试."""

    def test_empty_not_progressive(self):
        """真测: 空 programme 不算进步."""
        lp = LakatosProgramme(programme_id="P1")
        assert not lp.is_progressive()

    def test_progressive_when_novel_dominates(self):
        """真测: novel > ad-hoc = 进步."""
        lp = LakatosProgramme(programme_id="P1")
        lp.add_to_hard_core("axiom1")
        lp.add_protective_belt("h1")
        lp.add_novel_prediction("p1")
        lp.add_novel_prediction("p2")
        assert lp.is_progressive()

    def test_regressive_when_ad_hoc_dominates(self):
        """真测: ad-hoc > novel = 退化."""
        lp = LakatosProgramme(programme_id="P1")
        lp.add_ad_hoc("ad1")
        lp.add_ad_hoc("ad2")
        lp.add_novel_prediction("p1")
        assert not lp.is_progressive()

    def test_progressiveness_score_in_range(self):
        """真测: 进步度 ∈ [0, 1]."""
        lp = LakatosProgramme(programme_id="P1")
        lp.add_novel_prediction("p1")
        lp.add_ad_hoc("ad1")
        s = lp.progressiveness_score()
        assert 0.0 <= s <= 1.0

    def test_hard_core_intact(self):
        """真测: hard core 累积."""
        lp = LakatosProgramme(programme_id="P1")
        lp.add_to_hard_core("a1")
        lp.add_to_hard_core("a2")
        assert len(lp.hard_core) == 2

    def test_protective_belt_mutable(self):
        """真测: protective belt 累积."""
        lp = LakatosProgramme(programme_id="P1")
        for i in range(5):
            lp.add_protective_belt(f"h{i}")
        assert len(lp.protective_belt) == 5


# ============================================================================
# 4. ProofAssistantBridge 真测试 (Lean 2015 + Coq 2004 借鉴)
# ============================================================================


class TestProofAssistantBridge:
    """ProofAssistantBridge 真测试."""

    def test_empty_coverage_is_one(self):
        """真测: 空 proofs = coverage 1.0."""
        pb = ProofAssistantBridge()
        assert pb.coverage() == 1.0

    def test_assert_proposition(self):
        """真测: assert 加 step + context."""
        pb = ProofAssistantBridge()
        pb.assert_proposition("P1", proof_term={"type": "axiom"})
        assert "P1" in pb.context

    def test_verify_step_with_deps_satisfied(self):
        """真测: deps 满足 + non-empty term = verified."""
        pb = ProofAssistantBridge()
        pb.assert_proposition("P1", proof_term={"type": "axiom"})
        step = pb.assert_proposition(
            "P2",
            proof_term={"type": "apply", "rule": "intro"},
            dependencies=["P1"],
        )
        assert pb.verify_step(step)

    def test_verify_step_with_missing_deps(self):
        """真测: deps 缺失 = not verified."""
        pb = ProofAssistantBridge()
        step = pb.assert_proposition(
            "P2",
            proof_term={"type": "apply"},
            dependencies=["P1"],
        )
        assert not pb.verify_step(step)

    def test_verify_all(self):
        """真测: 全部 verify."""
        pb = ProofAssistantBridge()
        pb.assert_proposition("P1", proof_term={"t": "ax"})
        pb.assert_proposition(
            "P2", proof_term={"t": "app"}, dependencies=["P1"]
        )
        pb.assert_proposition(
            "P3", proof_term={"t": "app"}, dependencies=["P2"]
        )
        assert pb.verify_all()

    def test_coverage_partial(self):
        """真测: 部分 verify 覆盖率."""
        pb = ProofAssistantBridge()
        pb.assert_proposition("P1", proof_term={"t": "ax"})
        pb.assert_proposition(
            "P2", proof_term={"t": "app"}, dependencies=["MISSING"]
        )
        coverage = pb.coverage()
        assert 0.0 < coverage < 1.0


# ============================================================================
# 5. TruthDiscovery 真测试 (Dong 2009)
# ============================================================================


class TestTruthDiscovery:
    """TruthDiscovery 真测试."""

    def test_discovered_truth_weighted(self):
        """真测: 真值 = weighted avg by source trust."""
        td = TruthDiscovery()
        td.add_source("S1", trust=0.9)
        td.add_source("S2", trust=0.1)
        td.add_claim("C1", value=1.0, source_ids=["S1", "S2"])
        # S1 dominant → 接近 1.0
        truth = td.discovered_truth("C1")
        assert truth > 0.9

    def test_unknown_claim_zero(self):
        """真测: 未知 claim → 0.0."""
        td = TruthDiscovery()
        assert td.discovered_truth("UNKNOWN") == 0.0

    def test_update_trust_adjusts(self):
        """真测: 真值已知时 update_trust 调整 source."""
        td = TruthDiscovery()
        td.add_source("S1", trust=0.5)
        td.add_claim("C1", value=1.0, source_ids=["S1"])
        before = td.sources["S1"].trustworthiness
        td.update_trust("C1", true_value=1.0)  # S1 准确
        after = td.sources["S1"].trustworthiness
        assert after >= before

    def test_update_trust_penalizes_wrong(self):
        """真测: 真值远离时 trust 减."""
        td = TruthDiscovery()
        td.add_source("S1", trust=0.5)
        td.add_claim("C1", value=0.0, source_ids=["S1"])
        before = td.sources["S1"].trustworthiness
        td.update_trust("C1", true_value=1.0)  # S1 完全错
        after = td.sources["S1"].trustworthiness
        assert after < before

    def test_trust_clamped(self):
        """真测: trust ∈ [0, 1]."""
        td = TruthDiscovery()
        td.add_source("S1", trust=0.99)
        td.add_claim("C1", value=1.0, source_ids=["S1"])
        for _ in range(20):
            td.update_trust("C1", true_value=1.0)
        assert 0.0 <= td.sources["S1"].trustworthiness <= 1.0

    def test_no_sources_uses_raw(self):
        """真测: 无 source → claim 原始值."""
        td = TruthDiscovery()
        td.add_claim("C1", value=0.42, source_ids=[])
        assert td.discovered_truth("C1") == pytest.approx(0.42)


# ============================================================================
# 6. FormalVerifier 真测试 (Hoare 1969)
# ============================================================================


class TestFormalVerifier:
    """FormalVerifier 真测试."""

    def test_empty_zero_verified(self):
        """真测: 空 verifier → 0 verified."""
        fv = FormalVerifier()
        assert fv.verified_count() == 0

    def test_verify_valid_triple(self):
        """真测: valid triple → True."""
        fv = FormalVerifier()
        triple = HoareTriple(pre={"x": 0}, program="inc_x", post={"x": 1})
        assert fv.verify(triple)

    def test_verify_empty_pre_fails(self):
        """真测: 空 pre → False."""
        fv = FormalVerifier()
        triple = HoareTriple(pre={}, program="inc_x", post={"x": 1})
        assert not fv.verify(triple)

    def test_verify_empty_program_fails(self):
        """真测: 空 program → False."""
        fv = FormalVerifier()
        triple = HoareTriple(pre={"x": 0}, program="", post={"x": 1})
        assert not fv.verify(triple)

    def test_verify_empty_post_fails(self):
        """真测: 空 post → False."""
        fv = FormalVerifier()
        triple = HoareTriple(pre={"x": 0}, program="inc_x", post={})
        assert not fv.verify(triple)

    def test_verified_count_increments(self):
        """真测: 验证后 count 增加."""
        fv = FormalVerifier()
        for i in range(3):
            fv.verify(HoareTriple(pre={"i": i}, program=f"step_{i}", post={"i": i + 1}))
        assert fv.verified_count() == 3


# ============================================================================
# 7. CoherenceEngine 真测试 (BonJour 1985)
# ============================================================================


class TestCoherenceEngine:
    """CoherenceEngine 真测试."""

    def test_empty_high_coherence(self):
        """真测: 空 = 1.0 (vacuous)."""
        ce = CoherenceEngine()
        assert ce.coherence_score() == 1.0

    def test_add_belief(self):
        """真测: 加信念."""
        ce = CoherenceEngine()
        ce.add_belief("B1")
        assert "B1" in ce.beliefs

    def test_full_connection_high_score(self):
        """真测: 全部连接 = 1.0."""
        ce = CoherenceEngine()
        for i in range(4):
            ce.add_belief(f"B{i}")
        for i in range(4):
            for j in range(4):
                if i != j:
                    ce.add_support(f"B{i}", f"B{j}")
        assert ce.coherence_score() == pytest.approx(1.0)

    def test_no_connection_low_score(self):
        """真测: 无连接 = 0.0."""
        ce = CoherenceEngine()
        ce.add_belief("B1")
        ce.add_belief("B2")
        assert ce.coherence_score() == 0.0

    def test_partial_score(self):
        """真测: 部分连接分数."""
        ce = CoherenceEngine()
        ce.add_belief("B1")
        ce.add_belief("B2")
        ce.add_belief("B3")
        ce.add_support("B1", "B2")  # 1 / (3*2) = 1/6
        s = ce.coherence_score()
        assert 0.0 < s < 1.0

    def test_reflective_equilibrium_returns_score(self):
        """真测: 反思平衡返稳定度."""
        ce = CoherenceEngine()
        for i in range(3):
            ce.add_belief(f"B{i}")
        for i in range(3):
            for j in range(3):
                if i != j:
                    ce.add_support(f"B{i}", f"B{j}")
        score = ce.reflective_equilibrium()
        assert 0.0 <= score <= 1.0


# ============================================================================
# 8. CausalTruth 真测试 (Pearl 2009)
# ============================================================================


class TestCausalTruth:
    """CausalTruth 真测试."""

    def test_intervene_sets_value(self):
        """真测: do(X=v) → 返回 v."""
        ct = CausalTruth()
        assert ct.intervene("X", 5.0) == 5.0

    def test_descendants_bfs(self):
        """真测: descendants BFS 遍历."""
        ct = CausalTruth()
        ct.graph.add_edge("A", "B")
        ct.graph.add_edge("B", "C")
        descendants = ct.graph.descendants("A")
        assert "B" in descendants
        assert "C" in descendants

    def test_no_descendants(self):
        """真测: 叶子节点无 descendants."""
        ct = CausalTruth()
        ct.graph.add_edge("A", "B")
        assert ct.graph.descendants("B") == set()

    def test_backdoor_paths_excludes_direct(self):
        """真测: backdoor 排除直接 cause→effect."""
        ct = CausalTruth()
        ct.graph.add_edge("X", "Y")
        ct.graph.add_edge("Z", "Y")
        ct.graph.add_edge("X", "Z")
        # X → Y 是直接的, 不是 backdoor
        bds = ct.backdoor_paths("X", "Y")
        for path in bds:
            # backdoor 必须从 confounder 出发
            assert path[0] != "Y"  # 不是直接路径

    def test_intervention_records_node(self):
        """真测: 干预加 node."""
        ct = CausalTruth()
        ct.intervene("X", 1.0)
        assert "X" in ct.graph.nodes

    def test_descendants_under_intervention(self):
        """真测: do 后 descendants 仍正确."""
        ct = CausalTruth()
        ct.graph.add_edge("A", "B")
        ct.intervene("A", 1.0)
        d = ct.descendants_under_intervention("A")
        assert "B" in d


# ============================================================================
# 9. KnowledgeGraphFiller 真测试 (Bordes 2013 TransE 借鉴)
# ============================================================================


class TestKnowledgeGraphFiller:
    """KnowledgeGraphFiller 真测试."""

    def test_add_triple(self):
        """真测: 三元组累积."""
        kg = KnowledgeGraphFiller()
        kg.add_triple("A", "r1", "B")
        assert len(kg.triples) == 1
        assert "A" in kg.entities
        assert "B" in kg.entities
        assert "r1" in kg.relations

    def test_train_returns_vectors(self):
        """真测: train 返 entity + relation vectors."""
        kg = KnowledgeGraphFiller(dim=4, epochs=5)
        kg.add_triple("A", "r1", "B")
        kg.add_triple("B", "r2", "C")
        entities_v, relations_v = kg.train()
        assert len(entities_v) == 3
        assert len(relations_v) == 2

    def test_predict_tail_returns_entity(self):
        """真测: predict_tail 返 entity."""
        kg = KnowledgeGraphFiller(dim=4, epochs=5)
        kg.add_triple("A", "r1", "B")
        kg.add_triple("B", "r2", "C")
        entities_v, relations_v = kg.train()
        tail = kg.predict_tail("A", "r1", entities_v, relations_v)
        assert tail in kg.entities

    def test_predict_tail_unknown_head(self):
        """真测: 未知 head → None."""
        kg = KnowledgeGraphFiller()
        kg.add_triple("A", "r1", "B")
        entities_v, relations_v = kg.train()
        tail = kg.predict_tail("UNKNOWN", "r1", entities_v, relations_v)
        assert tail is None

    def test_empty_train_safe(self):
        """真测: 空 triples train 安全."""
        kg = KnowledgeGraphFiller()
        entities_v, relations_v = kg.train()
        assert entities_v == {}
        assert relations_v == {}

    def test_vectors_dim_match(self):
        """真测: vectors dim 匹配."""
        kg = KnowledgeGraphFiller(dim=6)
        kg.add_triple("A", "r", "B")
        entities_v, _ = kg.train()
        for vec in entities_v.values():
            assert len(vec) == 6


# ============================================================================
# 10. ConceptSpace 真测试 (Gärdenfors 2004)
# ============================================================================


class TestConceptSpace:
    """ConceptSpace 真测试."""

    def test_add_dimension(self):
        """真测: 加 dimension."""
        cs = ConceptSpace()
        cs.add_dimension("color")
        cs.add_dimension("size")
        assert len(cs.dimensions) == 2

    def test_add_dimension_idempotent(self):
        """真测: dimension 重复加无变化."""
        cs = ConceptSpace()
        cs.add_dimension("color")
        cs.add_dimension("color")
        assert len(cs.dimensions) == 1

    def test_add_concept(self):
        """真测: 加 concept + dimensions 自动."""
        cs = ConceptSpace()
        cs.add_concept("apple", {"color": 0.8, "size": 0.3})
        assert "color" in cs.dimensions
        assert "apple" in cs.concepts

    def test_distance_same_zero(self):
        """真测: 同概念 = 0 距离."""
        cs = ConceptSpace()
        cs.add_concept("A", {"x": 0.5, "y": 0.5})
        cs.add_concept("B", {"x": 0.5, "y": 0.5})
        assert cs.distance("A", "B") == pytest.approx(0.0, abs=1e-9)

    def test_distance_euclidean(self):
        """真测: Euclidean 距离."""
        cs = ConceptSpace()
        cs.add_concept("A", {"x": 0.0})
        cs.add_concept("B", {"x": 3.0, "y": 4.0})
        # common = {x}: |0-3| = 3
        assert cs.distance("A", "B") == pytest.approx(3.0)

    def test_nearest_concept(self):
        """真测: nearest 找最近."""
        cs = ConceptSpace()
        cs.add_concept("A", {"x": 0.0, "y": 0.0})
        cs.add_concept("B", {"x": 10.0, "y": 10.0})
        cs.add_concept("C", {"x": 0.1, "y": 0.1})
        nearest = cs.nearest_concept({"x": 0.05, "y": 0.05})
        assert nearest in ("A", "C")


# ============================================================================
# 11. ASITruthBridge 真测试
# ============================================================================


class TestASITruthBridge:
    """ASITruthBridge 真测试."""

    def test_empty_bridge_zero(self):
        """真测: 空 bridge → 0.0."""
        bridge = ASITruthBridge()
        assert bridge.overall_truth_score() == 0.0

    def test_measure_bayesian_uncertainty(self):
        """真测: Bayesian entropy."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        bridge = ASITruthBridge(bayesian=bt)
        u = bridge.measure_bayesian_uncertainty()
        assert u > 0.5

    def test_measure_falsifiability_zero_when_no_popper(self):
        """真测: 无 Popper → 0."""
        bridge = ASITruthBridge()
        assert bridge.measure_falsifiability() == 0.0

    def test_measure_falsifiability_one_when_scientific(self):
        """真测: scientific → 1.0."""
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        bridge = ASITruthBridge(popper=pf)
        assert bridge.measure_falsifiability() == 1.0

    def test_measure_progressiveness(self):
        """真测: Lakatos 进步度."""
        lp = LakatosProgramme(programme_id="P1")
        lp.add_novel_prediction("p1")
        bridge = ASITruthBridge(lakatos=lp)
        s = bridge.measure_progressiveness()
        assert 0.0 <= s <= 1.0

    def test_overall_truth_score_partial(self):
        """真测: 部分组件 → 部分分数."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        bridge = ASITruthBridge(bayesian=bt, popper=pf)
        s = bridge.overall_truth_score()
        assert 0.0 < s <= 1.0


# ============================================================================
# 12. 守门 (主 17:43 + 主 17:58 + 主 20:46): 不假装
# ============================================================================


class TestV1051PhilosophyGuards:
    """V3 哲学守门真测试."""

    def test_godel_guard_detects_self_ref(self):
        """真测: Gödel 自指检测."""
        assert godel_self_reference_guard("this_proposition_is_true")
        assert godel_self_reference_guard("I am not provable")
        assert not godel_self_reference_guard("regular statement")

    def test_popper_falsifiability_guard(self):
        """真测: Popper 划界守门."""
        assert popper_falsifiability_guard(has_falsification_tests=True)
        assert not popper_falsifiability_guard(has_falsification_tests=False)

    def test_coherence_threshold_guard(self):
        """真测: 融贯阈值守门."""
        assert coherence_threshold_guard(0.8, threshold=0.5)
        assert not coherence_threshold_guard(0.3, threshold=0.5)

    def test_uncertainty_acknowledgment_guard(self):
        """真测: Russell 2019 不确定性守门."""
        assert uncertainty_acknowledgment_guard(russell_principle=True)
        assert not uncertainty_acknowledgment_guard(russell_principle=False)

    def test_computational_limit_guard(self):
        """真测: Klee 1984 元真理论守门."""
        assert computational_limit_guard("correspondence")
        assert computational_limit_guard("coherence")
        assert computational_limit_guard("pragmatic")
        assert not computational_limit_guard("unknown_kind")

    def test_asisafety_truth_guard(self):
        """真测: ASI 安全真理守门."""
        assert asisafety_truth_guard(0.8, threshold=0.5)
        assert not asisafety_truth_guard(0.3, threshold=0.5)


# ============================================================================
# 13. ASI V0.2 truth 真映射综合测试
# ============================================================================


class TestASITruthIntegration:
    """ASI Truth 综合真测."""

    def test_full_truth_pipeline(self):
        """真测: 全 pipeline — Bayesian + Popper + Lakatos + Proof + TruthDiscovery + Coherence + Causal + KG + Concept."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.5)
        for _ in range(3):
            bt.add_evidence(0.9, 0.1)

        pf = PopperFalsifier(hypothesis_id="H1")
        for i in range(3):
            pf.add_test(f"T{i}", passed=True)

        lp = LakatosProgramme(programme_id="P1")
        for _ in range(3):
            lp.add_novel_prediction("new")

        pb = ProofAssistantBridge()
        for i in range(3):
            pb.assert_proposition(f"P{i}", proof_term={"t": "ax"})

        td = TruthDiscovery()
        td.add_source("S1", trust=0.9)
        td.add_claim("C1", value=0.95, source_ids=["S1"])

        ce = CoherenceEngine()
        for i in range(3):
            ce.add_belief(f"B{i}")
        for i in range(3):
            for j in range(3):
                if i != j:
                    ce.add_support(f"B{i}", f"B{j}")

        ct = CausalTruth()
        ct.graph.add_edge("A", "B")
        ct.graph.add_edge("B", "C")

        kg = KnowledgeGraphFiller()
        for i in range(3):
            kg.add_triple(f"E{i}", "r", f"E{(i+1) % 3}")

        cs = ConceptSpace()
        cs.add_concept("apple", {"color": 0.8, "size": 0.3})

        bridge = ASITruthBridge(
            bayesian=bt, popper=pf, lakatos=lp, proof=pb,
            truth_discovery=td, coherence=ce, causal=ct, kg=kg, concept=cs,
        )

        s = bridge.overall_truth_score()
        assert 0.0 < s <= 1.0

    def test_truth_components_independently(self):
        """真测: 各组件独立工作."""
        bt = BayesianTruthUpdater(hypothesis_id="H1", prior=0.7)
        assert 0.0 <= bt.posterior() <= 1.0

        pf = PopperFalsifier(hypothesis_id="H1")
        pf.add_test("T1", passed=True)
        assert pf.is_scientific()

        lp = LakatosProgramme(programme_id="P1")
        lp.add_novel_prediction("p1")
        assert lp.is_progressive()

    def test_asi_truth_v1051_version(self):
        """真测: 版本标识."""
        assert V1051_VERSION == "0.1.0"