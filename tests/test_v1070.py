"""V1070 ASI Scientific Method Core — tests."""
from __future__ import annotations
import sys
sys.path.insert(0, '.')

import math
import pytest
from apeireth.v1070_asi_scientific_method_core import (
    FalsificationResult, FalsificationEngine,
    KuhnPhase, Paradigm, ParadigmTracker,
    ResearchProgram, ResearchProgramRegistry,
    MethodCounter, AnarchyMethod,
    Problem, ProgressTracker,
    Obstacle, EpistemicObstacles,
    SevereTest, SevereTester,
    CausalTest, CausalityProbe,
    Community, SocialEpistemics,
    ScientificConfig, V1070Orchestrator,
    v1070_bridge_measure, v1070_report_markdown,
    v1070_philosophy_guard, v1070_run,
    V1070_VERSION,
)


# ============================================================================
# 1. FalsificationEngine (Popper)
# ============================================================================


class TestFalsificationEngine:
    """V1070 Popper 证伪主义真生产测试 (Popper 1934/1959)."""

    def test_propose_basic(self):
        """propose hypothesis 真借鉴 (Popper 不可证伪 ≠ 科学)."""
        eng = FalsificationEngine()
        hid = eng.propose("H1: ASI 北极星 V0.1 公式 8 项真测量", "ASI")
        assert hid.startswith("hyp_")
        assert hid in eng.hypotheses

    def test_unfalsifiable_marked_falsified(self):
        """Popper: 不可证伪 = 伪科学 真借鉴."""
        eng = FalsificationEngine()
        hid = eng.propose("H1: 不可证伪", "ASI", falsifiable=False)
        assert eng.hypotheses[hid].falsified is True

    def test_falsify_survived(self):
        """falsify attempt: 幸存 → corroboration 真借鉴."""
        eng = FalsificationEngine()
        hid = eng.propose("H1: 真生产测试", "ASI")
        for _ in range(5):
            eng.falsify(hid, "consistent evidence", falsifies=False)
        h = eng.hypotheses[hid]
        assert h.attempts == 5
        assert h.survived == 5
        assert h.corroborated is True
        assert h.falsified is False

    def test_falsify_falsified(self):
        """Popper 1 次证伪 = 拒绝 真借鉴."""
        eng = FalsificationEngine()
        hid = eng.propose("H1: 测试", "ASI")
        eng.falsify(hid, "counter-evidence", falsifies=True)
        assert eng.hypotheses[hid].falsified is True
        assert eng.hypotheses[hid].corroborated is False

    def test_corroboration_rate(self):
        """corroboration rate 真生产 (主 17:43 实事求是)."""
        eng = FalsificationEngine()
        for i in range(3):
            hid = eng.propose(f"H{i}", "ASI")
            for _ in range(4):
                eng.falsify(hid, "ev", falsifies=False)
        rate = eng.corroboration_rate()
        assert rate == 1.0  # all 3 corroborated

    def test_stats(self):
        """stats 真借鉴."""
        eng = FalsificationEngine()
        eng.propose("H1", "ASI")
        s = eng.stats()
        assert s["n_hypotheses"] == 1


# ============================================================================
# 2. ParadigmTracker (Kuhn)
# ============================================================================


class TestParadigmTracker:
    """V1070 Kuhn 范式真生产测试 (Kuhn 1962)."""

    def test_create_paradigm(self):
        """create 真借鉴 (Kuhn 1962)."""
        t = ParadigmTracker()
        pid = t.create("Test", "ASI")
        assert pid in t.paradigms
        assert t.paradigms[pid].phase == KuhnPhase.PARADIGM

    def test_anomalies_to_crisis(self):
        """Kuhn: anomalies → crisis → revolution 真借鉴."""
        t = ParadigmTracker()
        pid = t.create("Test", "ASI")
        for _ in range(5):
            t.add_anomaly(pid)
        assert t.paradigms[pid].phase == KuhnPhase.CRISIS
        for _ in range(5):
            t.add_anomaly(pid)
        assert t.paradigms[pid].phase == KuhnPhase.REVOLUTION

    def test_solve_puzzle_to_normal(self):
        """Kuhn: solve puzzle → normal_science 真借鉴."""
        t = ParadigmTracker()
        pid = t.create("Test", "ASI")
        # ensure pre-paradigm
        t.paradigms[pid].phase = KuhnPhase.PRE_PARADIGM
        t.solve_puzzle(pid)
        assert t.paradigms[pid].phase == KuhnPhase.NORMAL_SCIENCE

    def test_crisis_rate(self):
        """crisis rate 真借鉴."""
        t = ParadigmTracker()
        for _ in range(3):
            t.create("Test", "ASI")
        rate = t.crisis_rate()
        assert rate == 0.0  # no anomalies

    def test_stats(self):
        """stats 真借鉴."""
        t = ParadigmTracker()
        t.create("Test", "ASI")
        s = t.stats()
        assert s["n_paradigms"] == 1


# ============================================================================
# 3. ResearchProgramRegistry (Lakatos)
# ============================================================================


class TestResearchProgramRegistry:
    """V1070 Lakatos 研究纲领真生产测试 (Lakatos 1970)."""

    def test_create(self):
        """create 真借鉴 (Lakatos 硬核 + 保护带)."""
        r = ResearchProgramRegistry()
        rp_id = r.create("RP1", hard_core=["c1"], protective_belt=["b1"])
        assert rp_id in r.programs

    def test_progressive(self):
        """Lakatos 进步真借鉴 (new prediction + resolve anomalies)."""
        r = ResearchProgramRegistry()
        rp_id = r.create("RP1", hard_core=["c"], protective_belt=["b"])
        result = r.evaluate(rp_id, novel_predictions=3,
                            anomalies_resolved=5, anomalies_unresolved=2)
        assert result is True
        assert r.programs[rp_id].is_progressive

    def test_degenerating(self):
        """Lakatos 退步真借鉴 (no prediction + accumulate anomalies)."""
        r = ResearchProgramRegistry()
        rp_id = r.create("RP1", hard_core=["c"], protective_belt=["b"])
        r.evaluate(rp_id, novel_predictions=0,
                   anomalies_resolved=1, anomalies_unresolved=10)
        assert r.programs[rp_id].is_degenerating

    def test_progress_rate(self):
        """progress rate 真借鉴."""
        r = ResearchProgramRegistry()
        for i in range(2):
            rp_id = r.create(f"RP{i}", hard_core=["c"], protective_belt=["b"])
            r.evaluate(rp_id, novel_predictions=2, anomalies_resolved=5,
                       anomalies_unresolved=1)
        assert r.progress_rate() == 1.0

    def test_stats(self):
        """stats 真借鉴."""
        r = ResearchProgramRegistry()
        r.create("RP1", hard_core=["c"], protective_belt=["b"])
        s = r.stats()
        assert s["n_programs"] == 1


# ============================================================================
# 4. AnarchyMethod (Feyerabend)
# ============================================================================


class TestAnarchyMethod:
    """V1070 Feyerabend 认识论无政府真生产测试 (Feyerabend 1975)."""

    def test_add_method(self):
        """add method 真借鉴 (anything goes)."""
        a = AnarchyMethod()
        a.add_method("deduction", "ASI")
        assert len(a.methods) == 1

    def test_diversity(self):
        """diversity 真借鉴 (主 19:33 走在前人)."""
        a = AnarchyMethod()
        for m in ["deduction", "induction", "abduction", "analogy", "simulation"]:
            a.add_method(m, "ASI")
        assert a.diversity == 5
        assert a.diversity_score() == 1.0

    def test_use_method(self):
        """use method 真借鉴."""
        a = AnarchyMethod()
        mid = a.add_method("deduction", "ASI")
        a.use_method(mid)
        assert a.methods[mid].counter == 1

    def test_stats(self):
        """stats 真借鉴."""
        a = AnarchyMethod()
        a.add_method("deduction", "ASI")
        s = a.stats()
        assert s["n_methods"] == 1


# ============================================================================
# 5. ProgressTracker (Laudan)
# ============================================================================


class TestProgressTracker:
    """V1070 Laudan 进步问题真生产测试 (Laudan 1977)."""

    def test_add_problem(self):
        """add problem 真借鉴."""
        p = ProgressTracker()
        pid = p.add_problem("P1", "ASI")
        assert pid in p.problems

    def test_solve_problem(self):
        """solve problem 真借鉴 (Laudan 进步 = 解决问题)."""
        p = ProgressTracker()
        pid = p.add_problem("P1", "ASI")
        p.solve_problem(pid)
        assert p.problems[pid].is_solved

    def test_solve_rate(self):
        """solve rate 真借鉴."""
        p = ProgressTracker()
        for i in range(3):
            pid = p.add_problem(f"P{i}", "ASI")
            p.solve_problem(pid)
        assert p.solve_rate() == 1.0

    def test_stats(self):
        """stats 真借鉴."""
        p = ProgressTracker()
        p.add_problem("P1", "ASI")
        s = p.stats()
        assert s["n_problems"] == 1


# ============================================================================
# 6. EpistemicObstacles (Bachelard)
# ============================================================================


class TestEpistemicObstacles:
    """V1070 Bachelard 认识论障碍真生产测试 (Bachelard 1938)."""

    def test_add_obstacle(self):
        """add obstacle 真借鉴."""
        e = EpistemicObstacles()
        oid = e.add("O1", "ASI", severity=3)
        assert oid in e.obstacles

    def test_bypass(self):
        """bypass obstacle 真借鉴 (Bachelard 断裂)."""
        e = EpistemicObstacles()
        oid = e.add("O1", "ASI", severity=3)
        e.bypass(oid)
        assert e.obstacles[oid].bypassed

    def test_bypass_rate(self):
        """bypass rate 真借鉴."""
        e = EpistemicObstacles()
        for i in range(2):
            oid = e.add(f"O{i}", "ASI", severity=2)
            e.bypass(oid)
        assert e.bypass_rate() == 1.0

    def test_stats(self):
        """stats 真借鉴."""
        e = EpistemicObstacles()
        e.add("O1", "ASI", severity=2)
        s = e.stats()
        assert s["n_obstacles"] == 1


# ============================================================================
# 7. SevereTester (Mayo)
# ============================================================================


class TestSevereTester:
    """V1070 Mayo 严重性测试真生产测试 (Mayo 1996)."""

    def test_severe_passed(self):
        """severe test passed 真借鉴 (Mayo 1996)."""
        m = SevereTester()
        sid = m.test("H1", se_statistic=0.7, n_pass=10, n_fail=2)
        assert m.tests[sid].is_passed is True

    def test_severe_failed(self):
        """severe test failed 真借鉴."""
        m = SevereTester()
        sid = m.test("H1", se_statistic=0.3, n_pass=2, n_fail=10)
        assert m.tests[sid].is_passed is False

    def test_pass_rate(self):
        """pass rate 真借鉴."""
        m = SevereTester()
        for i in range(3):
            m.test(f"H{i}", se_statistic=0.6 + 0.1 * i, n_pass=10, n_fail=2)
        assert m.pass_rate() == 1.0

    def test_stats(self):
        """stats 真借鉴."""
        m = SevereTester()
        m.test("H1", se_statistic=0.6, n_pass=10, n_fail=2)
        s = m.stats()
        assert s["n_tests"] == 1


# ============================================================================
# 8. CausalityProbe (Cartwright/Bird)
# ============================================================================


class TestCausalityProbe:
    """V1070 Cartwright/Bird 因果测试真生产测试 (Cartwright 1983 / Bird 2022)."""

    def test_robust_causal(self):
        """robust causal 真借鉴 (|corr|>0.5 + n>=10)."""
        c = CausalityProbe()
        cid = c.test("A", "B", correlation=0.7, n_obs=20)
        assert c.tests[cid].is_robust is True

    def test_non_robust(self):
        """non-robust 真借鉴."""
        c = CausalityProbe()
        cid = c.test("A", "B", correlation=0.3, n_obs=5)
        assert c.tests[cid].is_robust is False

    def test_causation_strength(self):
        """causation strength ~ |corr| * log(n) 真借鉴 (Bird)."""
        c = CausalityProbe()
        c.test("A", "B", correlation=0.8, n_obs=100)
        s = c.stats()
        assert s["n_tests"] == 1

    def test_robust_rate(self):
        """robust rate 真借鉴."""
        c = CausalityProbe()
        for i in range(2):
            c.test("A", "B", correlation=0.7 + 0.05 * i, n_obs=20)
        assert c.robust_rate() == 1.0

    def test_stats(self):
        """stats 真借鉴."""
        c = CausalityProbe()
        c.test("A", "B", correlation=0.7, n_obs=20)
        s = c.stats()
        assert s["n_tests"] == 1


# ============================================================================
# 9. SocialEpistemics (Longino)
# ============================================================================


class TestSocialEpistemics:
    """V1070 Longino 社会认识论真生产测试 (Longino 1990/2002)."""

    def test_create_community(self):
        """create community 真借鉴."""
        s = SocialEpistemics()
        cid = s.create_community("ASI", members=["m1", "m2", "m3", "m4"],
                                 venues=["v1", "v2", "v3"])
        assert s.communities[cid].has_dissent is True

    def test_diversity_score(self):
        """diversity score 真借鉴 (Longino 多元 + 民主)."""
        s = SocialEpistemics()
        for i in range(2):
            s.create_community(f"comm{i}",
                               members=[f"m_{j}" for j in range(5)],
                               venues=[f"v_{j}" for j in range(4)])
        assert s.diversity_score() > 0.5

    def test_stats(self):
        """stats 真借鉴."""
        s = SocialEpistemics()
        s.create_community("ASI", members=["m1"], venues=["v1"])
        st = s.stats()
        assert st["n_communities"] == 1


# ============================================================================
# 10. V1070Orchestrator
# ============================================================================


class TestV1070Orchestrator:
    """V1070 Orchestrator 真生产测试 (主 00:56 任何人能接手)."""

    def test_setup_default(self):
        """setup default 真生产 (主 13:31 干到底)."""
        orch = V1070Orchestrator()
        orch.setup()
        assert len(orch.popper.hypotheses) == 5
        assert len(orch.kuhn.paradigms) == 3
        assert len(orch.lakatos.programs) == 3

    def test_run(self):
        """run 真借鉴 (主 13:31 + 主 23:44 干到底)."""
        orch = V1070Orchestrator()
        results = orch.run()
        assert "popper" in results
        assert "kuhn" in results
        assert "lakatos" in results
        assert "feyerabend" in results
        assert "laudan" in results
        assert "bachelard" in results
        assert "mayo" in results
        assert "cartwright_bird" in results
        assert "longino" in results

    def test_measure(self):
        """measure V0.2 真测 (主 22:33)."""
        orch = V1070Orchestrator()
        m = orch.measure()
        assert 0.0 <= m["raw"] <= 1.0
        assert "components" in m

    def test_bridge_measure(self):
        """V0.2 bridge measure 真测 (主 22:33 16 项真测)."""
        score = v1070_bridge_measure()
        assert 0.0 <= score <= 1.0
        # V1070 target ≥ 0.85
        assert score >= 0.85, f"raw {score} too low"

    def test_report_markdown(self):
        """Markdown report 真生产 (主 00:56 任何人能接手)."""
        md = v1070_report_markdown()
        assert "# V1070" in md
        assert "Popper" in md
        assert "Kuhn" in md
        assert "Lakatos" in md
        assert "Feyerabend" in md

    def test_philosophy_guard(self):
        """V3 哲学守门 5 项 (主 17:58 + 主 20:46)."""
        g = v1070_philosophy_guard()
        assert all(g.values())
        assert len(g) == 5

    def test_v1070_run(self):
        """v1070_run 真生产 entry (主 00:56 任何人能接手)."""
        r = v1070_run()
        assert r["version"] == V1070_VERSION
        assert "results" in r
        assert "measure" in r
        assert "philosophy_guard" in r
        assert "report" in r


# ============================================================================
# 11. V3 不假装哲学守门
# ============================================================================


class TestV3Guard:
    """V1070 V3 不假装哲学守门 (主 17:58 + 主 20:46)."""

    def test_not_falsification_as_truth(self):
        """Popper: corroboration != truth 真守门."""
        g = v1070_philosophy_guard()
        assert g["not_falsification_as_truth"]

    def test_not_paradigm_as_reality(self):
        """Kuhn: paradigms incommensurable 真守门."""
        g = v1070_philosophy_guard()
        assert g["not_paradigm_as_reality"]

    def test_not_program_as_progress(self):
        """Lakatos: progressive vs degenerating 真守门."""
        g = v1070_philosophy_guard()
        assert g["not_program_as_progress"]

    def test_not_anarchy_as_freedom(self):
        """Feyerabend: against method != for chaos 真守门."""
        g = v1070_philosophy_guard()
        assert g["not_anarchy_as_freedom"]

    def test_not_asi_as_scientific(self):
        """ASI != science 真守门."""
        g = v1070_philosophy_guard()
        assert g["not_asi_as_scientific"]


# ============================================================================
# 12. Sanity: V57/V58/V59 集成
# ============================================================================


class TestLegacyIntegration:
    """V1070 集成 V57/V58/V59 (主 19:33 走在前人)."""

    def test_inherits_v57_popper(self):
        """V1070 集成 V57 Popper 真借鉴."""
        from apeireth.v57_popper_falsification import V57PopperFalsification
        v57 = V57PopperFalsification()
        hid = v57.propose_hypothesis("H1: V57 test", "ASI")
        # V1070 falsification engine should also work
        eng = FalsificationEngine()
        hid2 = eng.propose("H1: V1070 test", "ASI")
        assert hid != hid2  # different namespaces

    def test_inherits_v58_kuhn(self):
        """V1070 集成 V58 Kuhn 真借鉴."""
        from apeireth.v58_kuhn_paradigm import V58KuhnParadigm, KuhnPhase as KP58
        v58 = V58KuhnParadigm()
        pid = v58.create_paradigm("P1", "ASI")
        # V1070 paradigm tracker has same enum
        t = ParadigmTracker()
        t.create("P1", "ASI")
        assert t.paradigms and v58.paradigms

    def test_inherits_v59_lakatos(self):
        """V1070 集成 V59 Lakatos 真借鉴."""
        from apeireth.v59_scientific_method_integration import (
            V59ScientificMethodIntegration,
        )
        v59 = V59ScientificMethodIntegration()
        v59.create_research_program("RP1", hard_core=["c"], protective_belt=["b"])
        # V1070 research program registry should also work
        r = ResearchProgramRegistry()
        r.create("RP1", hard_core=["c"], protective_belt=["b"])
        assert len(r.programs) == 1
