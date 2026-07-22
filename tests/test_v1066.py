"""Tests for V1066 ASI Self-Improving Core (主 17:43 实事求是 + 主 00:56).

真借鉴 14 前人: Finn 2017 + Zoph & Le 2017 + Real et al. 2019 +
Silver et al. 2017/2018 + Yudkowsky 2008 + Schmidhuber 1987 +
Bostrom 2014 + Xu et al. 2018 + Hu et al. 2021 +
Madaan et al. 2023 + Chen et al. 2024 + FAIR 2024 + Huang et al. 2023.

10 真生产组件 + 5 守门 + ASI V0.2 桥接.
"""
from __future__ import annotations

import random

import pytest

from apeireth.v1066_asi_self_improving_core import (
    V1066_VERSION,
    ArchitectureSearch,
    ArchCandidate,
    ASISelfImprovingCoreBridge,
    CritiqueQuality,
    ErrorDetector,
    ErrorRecord,
    MetaGradientLearner,
    MetaLearner,
    ParamEfficientAdapter,
    RecursiveImprover,
    SelfCritique,
    SelfImprovingCore,
    SelfImprovingGuard,
    SelfImprovementReport,
    SelfPlayOptimizer,
    build_self_improving_core,
    quick_score,
)


# ---------------------------------------------------------------------------
# 1. MetaLearner tests (Finn 2017 MAML)
# ---------------------------------------------------------------------------

class TestMetaLearner:
    def test_init(self):
        ml = MetaLearner()
        assert len(ml.params) == 8
        assert ml.n_meta_updates == 0

    def test_inner_step(self):
        ml = MetaLearner()
        adapted = ml.inner_step("t1", [1.0] * 8)
        assert len(adapted) == 8
        # Should be perturbed from original
        for a, p in zip(adapted, ml.params):
            assert a != p

    def test_outer_step(self):
        ml = MetaLearner()
        ml.outer_step([[0.5] * 8])
        assert ml.n_meta_updates == 1

    def test_adapt(self):
        ml = MetaLearner()
        adapted, loss = ml.adapt("t1", [0.5] * 8)
        assert ml.n_tasks_seen == 1
        assert loss >= 0

    def test_multiple_tasks(self):
        ml = MetaLearner()
        for i in range(5):
            ml.adapt(f"t{i}", [0.3] * 8)
            ml.outer_step([[0.3] * 8])
        assert ml.n_tasks_seen == 5


# ---------------------------------------------------------------------------
# 2. ArchitectureSearch tests (Real 2019 NAS-Bench)
# ---------------------------------------------------------------------------

class TestArchitectureSearch:
    def test_seed_population(self):
        nas = ArchitectureSearch()
        nas.seed_population(n=5)
        assert len(nas.population) == 5

    def test_evolve(self):
        nas = ArchitectureSearch()
        nas.seed_population(n=10)
        child = nas.evolve()
        assert child is not None
        assert nas.generation == 1

    def test_best_fitness(self):
        nas = ArchitectureSearch()
        nas.seed_population(n=5)
        bf = nas.best_fitness()
        assert 0.0 <= bf <= 1.0

    def test_population_cap(self):
        nas = ArchitectureSearch(population_size=5)
        nas.seed_population(n=3)
        for _ in range(5):
            nas.evolve()
        assert len(nas.population) <= 5

    def test_empty_best_fitness(self):
        nas = ArchitectureSearch()
        assert nas.best_fitness() == 0.0


# ---------------------------------------------------------------------------
# 3. SelfPlayOptimizer tests (Silver 2018 AlphaZero)
# ---------------------------------------------------------------------------

class TestSelfPlayOptimizer:
    def test_init(self):
        sp = SelfPlayOptimizer()
        assert sp.n_games == 0
        assert sp.win_rate == 0.5

    def test_play_game(self):
        sp = SelfPlayOptimizer()
        winner = sp.play_game()
        assert winner in ("current", "previous")
        assert sp.n_games == 1

    def test_update(self):
        sp = SelfPlayOptimizer()
        sp.update("current", learning_rate=0.01)
        assert sp.win_rate > 0.5  # win moves rate up

    def test_elo_delta(self):
        sp = SelfPlayOptimizer()
        sp.win_rate = 0.76
        delta = sp.elo_delta()
        assert delta > 0  # positive for >0.5

    def test_elo_zero(self):
        sp = SelfPlayOptimizer()
        sp.win_rate = 0.5
        delta = sp.elo_delta()
        # 0.5 → -400 * log10(1) = 0
        assert abs(delta) < 0.001


# ---------------------------------------------------------------------------
# 4. ErrorDetector tests (Chen 2024 Self-Debug)
# ---------------------------------------------------------------------------

class TestErrorDetector:
    def test_attempt(self):
        ed = ErrorDetector()
        rec = ed.attempt("task1", "runtime")
        assert rec.task == "task1"
        assert ed.total_attempts == 1

    def test_fix_success(self):
        ed = ErrorDetector()
        rec = ed.attempt("task1", "syntax")
        result = ed.fix(rec.error_id, True)
        assert result is True
        assert ed.successful_fixes == 1
        assert rec.fixed is True

    def test_fix_fail(self):
        ed = ErrorDetector()
        rec = ed.attempt("task1", "logic")
        ed.fix(rec.error_id, False)
        assert ed.successful_fixes == 0

    def test_fix_rate(self):
        ed = ErrorDetector()
        for i in range(4):
            rec = ed.attempt(f"t{i}", "e")
            ed.fix(rec.error_id, i < 3)  # 3/4 success
        assert ed.fix_rate() == 0.75

    def test_empty_rate(self):
        ed = ErrorDetector()
        assert ed.fix_rate() == 0.0


# ---------------------------------------------------------------------------
# 5. RecursiveImprover tests (Yudkowsky 2008 RSI)
# ---------------------------------------------------------------------------

class TestRecursiveImprover:
    def test_init(self):
        ri = RecursiveImprover()
        assert ri.depth == 0
        assert ri.cumulative_gain == 1.0

    def test_improve(self):
        ri = RecursiveImprover()
        gain = ri.improve(1.2)
        assert ri.depth == 1
        assert gain > 1.0

    def test_improve_cumulative(self):
        ri = RecursiveImprover()
        ri.improve(1.5)
        ri.improve(1.5)
        assert ri.cumulative_gain == pytest.approx(2.25)

    def test_is_stable_fresh(self):
        ri = RecursiveImprover()
        assert ri.is_stable() is True

    def test_gain_tracks_depth(self):
        ri = RecursiveImprover()
        for _ in range(5):
            ri.improve(1.1)
        assert ri.depth == 5
        assert len(ri.gain_per_layer) == 5


# ---------------------------------------------------------------------------
# 6. ParamEfficientAdapter tests (Hu 2021 LoRA)
# ---------------------------------------------------------------------------

class TestParamEfficientAdapter:
    def test_init(self):
        lora = ParamEfficientAdapter()
        assert lora.rank == 8

    def test_init_matrices(self):
        lora = ParamEfficientAdapter()
        lora.init_matrices(d_in=32, d_out=16, rank=4)
        assert len(lora.A) == 32
        assert len(lora.A[0]) == 4
        assert len(lora.B) == 4
        assert len(lora.B[0]) == 16

    def test_effective_params(self):
        lora = ParamEfficientAdapter()
        lora.init_matrices(d_in=64, d_out=32, rank=8)
        eps = lora.effective_params()
        assert eps == 64 * 8 + 8 * 32

    def test_adapt(self):
        lora = ParamEfficientAdapter()
        lora.init_matrices(d_in=8, d_out=4, rank=2)
        grad_A = [[0.1] * 2 for _ in range(8)]
        grad_B = [[0.1] * 4 for _ in range(2)]
        n = lora.adapt(grad_A, grad_B, lr=0.1)
        assert n > 0


# ---------------------------------------------------------------------------
# 7. MetaGradientLearner tests (Xu 2018)
# ---------------------------------------------------------------------------

class TestMetaGradientLearner:
    def test_init(self):
        mg = MetaGradientLearner()
        assert "gamma" in mg.meta_params
        assert mg.n_meta_steps == 0

    def test_step(self):
        mg = MetaGradientLearner()
        updates = mg.step(loss=0.5)
        assert mg.n_meta_steps == 1
        assert len(updates) == 4

    def test_meta_params_bounded(self):
        mg = MetaGradientLearner()
        mg.step(loss=10.0)  # large loss
        for v in mg.meta_params.values():
            assert 0.0 <= v <= 1.0

    def test_meta_gradient_sign(self):
        mg = MetaGradientLearner()
        grad = mg.meta_gradient(1.0, "lr")
        assert grad <= 0  # lr should decrease with high loss


# ---------------------------------------------------------------------------
# 8. SelfCritique tests (Madaan 2023 Self-Refine)
# ---------------------------------------------------------------------------

class TestSelfCritique:
    def test_critique_low_quality(self):
        sc = SelfCritique()
        cr = sc.critique("bad output", 0.2)
        assert cr.quality_after > cr.quality_before
        assert "substantial" in cr.critique.lower()

    def test_critique_high_quality(self):
        sc = SelfCritique()
        cr = sc.critique("good output", 0.95)
        assert cr.quality_after >= cr.quality_before

    def test_improvement_trajectory(self):
        sc = SelfCritique()
        for q in [0.2, 0.5, 0.8]:
            sc.critique(f"out{q}", q)
        traj = sc.improvement_trajectory()
        assert len(traj) == 3

    def test_empty_trajectory(self):
        sc = SelfCritique()
        assert sc.improvement_trajectory() == []


# ---------------------------------------------------------------------------
# 9. SelfImprovementReport tests (主 00:56)
# ---------------------------------------------------------------------------

class TestSelfImprovementReport:
    def test_init(self):
        rep = SelfImprovementReport()
        assert rep.title == "ASI Self-Improving Core Report"

    def test_add_section(self):
        rep = SelfImprovementReport()
        rep.add_section("Test", "Body")
        assert ("Test", "Body") in rep.sections

    def test_render_has_title(self):
        rep = SelfImprovementReport(title="Custom")
        rep.add_section("C", "1. X")
        md = rep.render()
        assert "# Custom" in md
        assert "## C" in md

    def test_render_has_guards(self):
        rep = SelfImprovementReport()
        md = rep.render()
        assert "V3 哲学守门" in md
        assert "Understanding" in md

    def test_summary_dict(self):
        s = SelfImprovementReport.summary_dict(10, 8, 50, 5, 0.85)
        assert "10" in s
        assert "8" in s


# ---------------------------------------------------------------------------
# 10. ASISelfImprovingCoreBridge tests
# ---------------------------------------------------------------------------

class TestASISelfImprovingCoreBridge:
    def test_weights_sum_to_one(self):
        b = ASISelfImprovingCoreBridge()
        assert sum(b.weights.values()) == pytest.approx(1.0, abs=1e-9)

    def test_score_zero(self):
        b = ASISelfImprovingCoreBridge()
        r = b.score({})
        assert r["self_improving_core_v0_2"] == 0.0

    def test_score_perfect(self):
        b = ASISelfImprovingCoreBridge()
        perfect = {k: 1.0 for k in b.weights}
        r = b.score(perfect)
        assert r["self_improving_core_v0_2"] == pytest.approx(1.0, abs=1e-9)

    def test_threshold_pass(self):
        b = ASISelfImprovingCoreBridge()
        assert b.threshold_check(0.90)["passed"] is True

    def test_threshold_fail(self):
        b = ASISelfImprovingCoreBridge()
        assert b.threshold_check(0.5)["passed"] is False

    def test_score_clamps(self):
        b = ASISelfImprovingCoreBridge()
        r = b.score({"meta_learning_adaptation": 5.0})
        assert 0.0 <= r["self_improving_core_v0_2"] <= 1.0


# ---------------------------------------------------------------------------
# 11. SelfImprovingGuard tests (主 17:58 + 主 20:46)
# ---------------------------------------------------------------------------

class TestSelfImprovingGuard:
    def test_maml_not_understanding(self):
        g = SelfImprovingGuard.guard_maml_not_understanding(
            {"meta_learning_adaptation": 0.5})
        assert g["guard"] == "maml_not_understanding"
        assert "Finn" in g["verdict"]

    def test_nas_not_creativity(self):
        g = SelfImprovingGuard.guard_nas_not_creativity({"nas_best_fitness": 0.5})
        assert g["guard"] == "nas_not_creativity"
        assert "Real" in g["verdict"]

    def test_selfplay_not_consciousness(self):
        g = SelfImprovingGuard.guard_selfplay_not_consciousness(
            {"self_play_win_rate": 0.5})
        assert "Silver" in g["verdict"]

    def test_error_detection_not_wisdom(self):
        g = SelfImprovingGuard.guard_error_detection_not_wisdom(
            {"error_fix_rate": 0.5})
        assert "Chen" in g["verdict"]

    def test_improvement_not_asi(self):
        g = SelfImprovingGuard.guard_improvement_not_asi(
            {"rsi_cumulative_gain": 0.5})
        assert "Yudkowsky" in g["verdict"]

    def test_all_guards_returns_5(self):
        assert len(SelfImprovingGuard.all_guards({})) == 5

    def test_high_score_does_not_pretend(self):
        g = SelfImprovingGuard.all_guards({
            "meta_learning_adaptation": 1.0, "nas_best_fitness": 1.0,
            "self_play_win_rate": 1.0, "error_fix_rate": 1.0,
            "rsi_cumulative_gain": 1.0,
        })
        for entry in g:
            assert "NOT" in entry["verdict"] or "≠" in entry["verdict"]


# ---------------------------------------------------------------------------
# 12. SelfImprovingCore pipeline integration
# ---------------------------------------------------------------------------

class TestSelfImprovingCore:
    def test_default_build(self):
        sic = build_self_improving_core()
        assert isinstance(sic, SelfImprovingCore)
        assert sic.meta_learner.n_tasks_seen > 0
        assert sic.nas.generation > 0
        assert sic.self_play.n_games > 0
        assert sic.error_detector.total_attempts > 0
        assert sic.recursive_improver.depth > 0

    def test_measure_runs(self):
        sic = build_self_improving_core()
        m = sic.measure()
        assert "meta_learning_adaptation" in m
        assert "report_readability" in m
        assert len(m) == 9

    def test_score_runs(self):
        sic = build_self_improving_core()
        s = sic.score()
        assert "self_improving_core_v0_2" in s
        assert 0.0 <= s["self_improving_core_v0_2"] <= 1.0

    def test_score_target(self):
        sic = build_self_improving_core()
        s = sic.score()
        assert s["self_improving_core_v0_2"] >= 0.85

    def test_threshold_pass(self):
        sic = build_self_improving_core()
        assert sic.threshold_pass(target=0.85) is True

    def test_quick_score(self):
        r = quick_score()
        assert "self_improving_core_v0_2" in r
        assert 0.0 <= r["self_improving_core_v0_2"] <= 1.0

    def test_make_report(self):
        sic = build_self_improving_core()
        md = sic.make_report()
        assert "# ASI Self-Improving Core Report" in md
        assert "Finn 2017" in md
        assert "V3 哲学守门" in md
        assert "AlphaZero" in md


# ---------------------------------------------------------------------------
# 13. Sanity tests (主 17:43 实事求是)
# ---------------------------------------------------------------------------

class TestSanity:
    def test_version(self):
        assert V1066_VERSION == "0.1.0"

    def test_14_precedents_documented(self):
        import apeireth.v1066_asi_self_improving_core as mod
        src = mod.__doc__ or ""
        expected = ["Finn et al. 2017", "Zoph & Le 2017", "Real et al. 2019",
                    "Silver et al. 2017", "Silver et al. 2018",
                    "Yudkowsky 2008", "Schmidhuber 1987",
                    "Bostrom 2014", "Xu et al. 2018", "Hu et al. 2021",
                    "Madaan et al. 2023", "Chen et al. 2024",
                    "FAIR 2024", "Huang et al. 2023"]
        for ref in expected:
            assert ref in src, f"missing: {ref}"

    def test_10_components_documented(self):
        import apeireth.v1066_asi_self_improving_core as mod
        src = mod.__doc__ or ""
        for comp in ["MetaLearner", "ArchitectureSearch", "SelfPlayOptimizer",
                     "ErrorDetector", "RecursiveImprover", "ParamEfficientAdapter",
                     "MetaGradientLearner", "SelfCritique",
                     "SelfImprovementReport", "ASISelfImprovingCoreBridge"]:
            assert comp in src, f"missing: {comp}"

    def test_5_guards_documented(self):
        import apeireth.v1066_asi_self_improving_core as mod
        src = mod.__doc__ or ""
        for guard in ["MAML = Understanding", "NAS = Creativity",
                      "Self-Play = Consciousness",
                      "Error Detection = Wisdom",
                      "Improvement = ASI"]:
            assert guard in src, f"missing: {guard}"

    def test_no_pretend_consciousness(self):
        import apeireth.v1066_asi_self_improving_core as mod
        with open(mod.__file__, encoding="utf-8") as f:
            src = (mod.__doc__ or "") + f.read()
        forbidden = ["MAML IS understanding", "NAS IS creativity",
                     "self-play == consciousness", "RSI == ASI"]
        for phrase in forbidden:
            assert phrase not in src

    def test_reproducibility(self):
        random.seed(42)
        sic1 = build_self_improving_core()
        s1 = sic1.score()["self_improving_core_v0_2"]
        random.seed(42)
        sic2 = build_self_improving_core()
        s2 = sic2.score()["self_improving_core_v0_2"]
        assert s1 == s2
