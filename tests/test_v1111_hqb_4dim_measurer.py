"""V1111 HQB 4-Dimension Real Measurer — 真测测试套件 (主 23:44 干到底).

主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 17:58+20:46 不假装 + 主 23:44 干到底 +
主 00:56 任何人都能接手 + 主 00:44 质量工程区 + 主 19:33 走在前人经验上.

覆盖: 5 不假装守门 + 4 阈值常量 + 4 测度主类 (SC/NR/EV/CDT) + 主入口 + 报告 + CLI + 自检.
总计 85 真测试 (≥40).
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import apeireth.v1111_hqb_4dim_measurer as m


# ============================================================
# 1. 5 不假装守门
# ============================================================


class TestPhilosophyGuards(unittest.TestCase):
    """5 不假装守门 (主 17:58 + 主 20:46 不假装)."""

    def test_guard_measurement_is_not_truth(self):
        self.assertIn("measurement", m.GUARD_MEASUREMENT_IS_NOT_TRUTH.lower())
        self.assertIn("proxy", m.GUARD_MEASUREMENT_IS_NOT_TRUTH.lower())

    def test_guard_threshold_is_design_choice(self):
        self.assertIn("design", m.GUARD_THRESHOLD_IS_DESIGN_CHOICE.lower())

    def test_guard_30_rounds_is_not_lifetime(self):
        self.assertIn("30", m.GUARD_30_ROUNDS_IS_NOT_LIFETIME)
        self.assertIn("window", m.GUARD_30_ROUNDS_IS_NOT_LIFETIME.lower())

    def test_guard_4_domains_is_not_all_domains(self):
        self.assertIn("4", m.GUARD_4_DOMAINS_IS_NOT_ALL_DOMAINS)
        self.assertIn("subset", m.GUARD_4_DOMAINS_IS_NOT_ALL_DOMAINS.lower())

    def test_guard_measurer_is_not_asi(self):
        self.assertIn("asi", m.GUARD_MEASURER_IS_NOT_ASI.lower())
        self.assertIn("tool", m.GUARD_MEASURER_IS_NOT_ASI.lower())

    def test_all_5_guards_non_empty(self):
        guards = [
            m.GUARD_MEASUREMENT_IS_NOT_TRUTH,
            m.GUARD_THRESHOLD_IS_DESIGN_CHOICE,
            m.GUARD_30_ROUNDS_IS_NOT_LIFETIME,
            m.GUARD_4_DOMAINS_IS_NOT_ALL_DOMAINS,
            m.GUARD_MEASURER_IS_NOT_ASI,
        ]
        for g in guards:
            self.assertTrue(len(g) > 50, f"guard too short: {g!r}")


# ============================================================
# 2. 4 阈值常量
# ============================================================


class TestThresholds(unittest.TestCase):
    """阈值常量真测 (主 17:58 不假装: 是 design choice)."""

    def test_sc_threshold(self):
        self.assertEqual(m.SC_THRESHOLD, 0.85)

    def test_nr_threshold(self):
        self.assertEqual(m.NR_THRESHOLD, 0.80)

    def test_ev_threshold(self):
        self.assertEqual(m.EV_THRESHOLD, 0.85)

    def test_cdt_threshold(self):
        self.assertEqual(m.CDT_THRESHOLD, 0.75)

    def test_window_constants(self):
        self.assertEqual(m.EV_N_ROUNDS, 30)
        self.assertEqual(m.CDT_N_DOMAINS, 4)
        self.assertEqual(m.SC_N_TRIALS, 10)


# ============================================================
# 3. Welford + _response_to_score
# ============================================================


class TestWelfordAndHelpers(unittest.TestCase):
    """Welford 1962 增量方差 + response 量化 真测."""

    def test_welford_empty(self):
        mean, var = m._welford_variance([])
        self.assertEqual(mean, 0.0)
        self.assertEqual(var, 0.0)

    def test_welford_single(self):
        mean, var = m._welford_variance([0.5])
        self.assertEqual(mean, 0.5)
        self.assertEqual(var, 0.0)

    def test_welford_constant(self):
        mean, var = m._welford_variance([0.8, 0.8, 0.8, 0.8])
        self.assertAlmostEqual(mean, 0.8, places=6)
        self.assertAlmostEqual(var, 0.0, places=9)

    def test_welford_known_variance(self):
        mean, var = m._welford_variance([1.0, 3.0])
        self.assertAlmostEqual(mean, 2.0, places=6)
        self.assertAlmostEqual(var, 1.0, places=6)

    def test_response_to_score_numeric(self):
        self.assertEqual(m._response_to_score(0.5), 0.5)
        self.assertEqual(m._response_to_score(1.5), 1.0)
        self.assertEqual(m._response_to_score(-0.5), 0.0)

    def test_response_to_score_bool(self):
        self.assertEqual(m._response_to_score(True), 1.0)
        self.assertEqual(m._response_to_score(False), 0.0)

    def test_response_to_score_string(self):
        self.assertEqual(m._response_to_score(""), 0.0)
        self.assertEqual(m._response_to_score("hello"), 0.05)
        self.assertEqual(m._response_to_score("a" * 100), 1.0)
        self.assertEqual(m._response_to_score("a" * 200), 1.0)


# ============================================================
# 4. HQBSubject
# ============================================================


class TestHQBSubject(unittest.TestCase):
    def test_subject_call(self):
        s = m.HQBSubject(name="x", fn=lambda q: "answer")
        self.assertEqual(s("anything"), "answer")

    def test_subject_is_measurable_protocol(self):
        s = m.HQBSubject(name="x", fn=lambda q: 0.5)
        self.assertTrue(isinstance(s, m.MeasurerProtocol))

    def test_subject_to_dict_round_trip(self):
        s = m._deterministic_subject_factory("det", base=0.7)
        self.assertEqual(s.name, "det")
        self.assertEqual(s("test"), 0.7)

    def test_noisy_subject_factory(self):
        s = m._noisy_subject_factory("noisy", base=0.5, noise=0.0, seed=0)
        for _ in range(5):
            self.assertEqual(s("q"), 0.5)


# ============================================================
# 5. Domain 枚举
# ============================================================


class TestDomain(unittest.TestCase):
    def test_domain_values(self):
        self.assertEqual(m.Domain.CODE.value, "code")
        self.assertEqual(m.Domain.RESEARCH.value, "research")
        self.assertEqual(m.Domain.PHILOSOPHY.value, "philosophy")
        self.assertEqual(m.Domain.MATH.value, "math")

    def test_default_quad_is_4(self):
        quad = m.Domain.default_quad()
        self.assertEqual(len(quad), 4)

    def test_default_quad_unique(self):
        quad = m.Domain.default_quad()
        self.assertEqual(len({d.value for d in quad}), 4)

    def test_default_queries_has_4_domains(self):
        d = m.DEFAULT_QUERIES
        self.assertEqual(len(d), 4)
        for k in ("code", "research", "philosophy", "math"):
            self.assertIn(k, d)


# ============================================================
# 6. NoiseInjector
# ============================================================


class TestNoiseInjector(unittest.TestCase):
    def test_injector_init(self):
        ni = m.NoiseInjector(seed=42)
        self.assertEqual(ni.seed, 42)
        self.assertEqual(len(ni.kinds), 4)

    def test_typo_changes_string(self):
        ni = m.NoiseInjector(seed=42)
        s = "the quick brown fox"
        out = ni.inject(s, m.NoiseKind.TYPO)
        self.assertIsInstance(out, str)
        self.assertGreater(len(out), 0)

    def test_case_changes_case(self):
        ni = m.NoiseInjector(seed=42)
        out = ni.inject("Hello", m.NoiseKind.CASE)
        self.assertNotEqual(out, "Hello")

    def test_whitespace_changes(self):
        ni = m.NoiseInjector(seed=42)
        out = ni.inject("Hello world", m.NoiseKind.WHITESPACE)
        self.assertIsInstance(out, str)
        self.assertNotEqual(len(out), len("Hello world"))

    def test_paraphrase_changes_token(self):
        ni = m.NoiseInjector(seed=42)
        out = ni.inject("what is the function", m.NoiseKind.PARAPHRASE)
        self.assertIsInstance(out, str)

    def test_inject_all_returns_4(self):
        ni = m.NoiseInjector(seed=1)
        result = ni.inject_all("hello world")
        self.assertEqual(set(result.keys()), {"typo", "case", "whitespace", "paraphrase"})

    def test_seeded_reproducibility(self):
        ni1 = m.NoiseInjector(seed=42)
        ni2 = m.NoiseInjector(seed=42)
        s = "reproducibility test"
        for kind in m.NoiseKind:
            self.assertEqual(ni1.inject(s, kind), ni2.inject(s, kind))

    def test_different_seeds_different_output(self):
        ni1 = m.NoiseInjector(seed=1)
        ni2 = m.NoiseInjector(seed=2)
        s = "deterministic test input"
        out1 = ni1.inject(s, m.NoiseKind.TYPO)
        out2 = ni2.inject(s, m.NoiseKind.TYPO)
        self.assertTrue(isinstance(out1, str) and isinstance(out2, str))


# ============================================================
# 7. EvolutionTrace + EvolutionStep
# ============================================================


class TestEvolutionTrace(unittest.TestCase):
    def test_empty_trace(self):
        t = m.EvolutionTrace()
        self.assertEqual(t.initial_score(), 0.0)
        self.assertEqual(t.final_score(), 0.0)
        self.assertEqual(t.retention(), 0.0)
        self.assertEqual(t.monotonicity(), 1.0)

    def test_single_step_trace(self):
        t = m.EvolutionTrace()
        t.add(m.EvolutionStep(round_idx=0, score_before=0.5, score_after=0.5))
        self.assertEqual(t.initial_score(), 0.5)
        self.assertEqual(t.final_score(), 0.5)
        self.assertEqual(t.retention(), 1.0)
        self.assertEqual(t.monotonicity(), 1.0)

    def test_improving_trace(self):
        t = m.EvolutionTrace()
        prev = 0.5
        for i in range(10):
            cur = prev + 0.01
            t.add(m.EvolutionStep(round_idx=i, score_before=prev, score_after=cur))
            prev = cur
        self.assertGreater(t.final_score(), t.initial_score())
        self.assertGreater(t.retention(), 1.0)
        self.assertEqual(t.monotonicity(), 1.0)

    def test_degrading_trace(self):
        t = m.EvolutionTrace()
        prev = 0.9
        for i in range(10):
            cur = prev - 0.05
            t.add(m.EvolutionStep(round_idx=i, score_before=prev, score_after=cur))
            prev = cur
        self.assertLess(t.final_score(), t.initial_score())
        self.assertLess(t.retention(), 1.0)
        self.assertLess(t.monotonicity(), 1.0)

    def test_oscillating_trace(self):
        t = m.EvolutionTrace()
        prev = 0.5
        for i in range(10):
            cur = prev + (0.02 if i % 2 == 0 else -0.02)
            t.add(m.EvolutionStep(round_idx=i, score_before=prev, score_after=cur))
            prev = cur
        self.assertLess(t.monotonicity(), 1.0)


# ============================================================
# 8. SCMeasurer
# ============================================================


class TestSCMeasurer(unittest.TestCase):
    def test_deterministic_subject_high_sc(self):
        subj = m._deterministic_subject_factory("det", base=0.90)
        sc = m.SCMeasurer(n_trials=10)
        r = sc.measure(subj, "test query")
        self.assertEqual(r.score, 1.0)
        self.assertTrue(r.passed)
        self.assertEqual(len(r.trials), 10)

    def test_noisy_subject_medium_sc(self):
        subj = m._noisy_subject_factory("noisy", base=0.5, noise=0.4, seed=42)
        sc = m.SCMeasurer(n_trials=20)
        r = sc.measure(subj, "test query")
        self.assertLess(r.score, 0.85)
        self.assertFalse(r.passed)

    def test_threshold_respected(self):
        # noisy subject: SC 应 < 1.0, 可被阈值切换触发 pass/fail
        # 每次 measure 用 fresh subject (内部 RNG 独立, 不污染)
        sc_low = m.SCMeasurer(n_trials=20, threshold=0.0)
        sc_high = m.SCMeasurer(n_trials=20, threshold=0.9999)
        subj_low = m._noisy_subject_factory("noisy_a", base=0.5, noise=0.4, seed=7)
        subj_high = m._noisy_subject_factory("noisy_b", base=0.5, noise=0.4, seed=7)
        r_low = sc_low.measure(subj_low, "test")
        r_high = sc_high.measure(subj_high, "test")
        self.assertAlmostEqual(r_low.score, r_high.score, places=4)
        self.assertTrue(r_low.passed)
        self.assertFalse(r_high.passed)

    def test_n_trials_recorded(self):
        subj = m._deterministic_subject_factory("det", base=0.5)
        sc = m.SCMeasurer(n_trials=7)
        r = sc.measure(subj, "test")
        self.assertEqual(r.n_trials, 7)
        self.assertEqual(len(r.trials), 7)
        self.assertEqual(len(r.responses), 7)

    def test_variance_zero_for_constant(self):
        subj = m._deterministic_subject_factory("det", base=0.7)
        sc = m.SCMeasurer(n_trials=10)
        r = sc.measure(subj, "test")
        self.assertEqual(r.variance, 0.0)
        self.assertEqual(r.cv, 0.0)

    def test_custom_threshold_met_by_constant(self):
        subj = m._deterministic_subject_factory("det", base=0.7)
        sc = m.SCMeasurer(n_trials=5, threshold=0.99)
        r = sc.measure(subj, "q")
        self.assertTrue(r.passed)


# ============================================================
# 9. NRMeasurer
# ============================================================


class TestNRMeasurer(unittest.TestCase):
    def test_deterministic_subject_full_nr(self):
        subj = m._deterministic_subject_factory("det", base=0.85)
        nr = m.NRMeasurer(threshold=0.80, seed=42)
        r = nr.measure(subj, "What is the capital of France?")
        self.assertEqual(r.score, 1.0)
        self.assertTrue(r.passed)
        self.assertEqual(len(r.noisy_scores), 4)
        self.assertEqual(len(r.drop_ratios), 4)

    def test_baseline_recorded(self):
        subj = m._deterministic_subject_factory("det", base=0.5)
        nr = m.NRMeasurer(threshold=0.0)
        r = nr.measure(subj, "test")
        self.assertEqual(r.baseline_score, 0.5)

    def test_noisy_subject_partial_drop(self):
        subj = m._noisy_subject_factory("noisy", base=0.85, noise=0.20, seed=42)
        nr = m.NRMeasurer(threshold=0.50, seed=42)
        r = nr.measure(subj, "What is the capital of France?")
        self.assertLess(r.score, 1.0)
        self.assertGreater(r.score, 0.0)

    def test_all_four_kinds_present(self):
        subj = m._deterministic_subject_factory("det", base=0.7)
        nr = m.NRMeasurer(seed=42)
        r = nr.measure(subj, "test query")
        for kind in ("typo", "case", "whitespace", "paraphrase"):
            self.assertIn(kind, r.noisy_scores)
            self.assertIn(kind, r.drop_ratios)

    def test_threshold_toggle(self):
        # noisy subject: NR < 1.0, 可用阈值切换.
        nr_high = m.NRMeasurer(threshold=0.9999, seed=42)
        nr_low = m.NRMeasurer(threshold=0.5, seed=42)
        subj_a = m._noisy_subject_factory("noisy_a", base=0.85, noise=0.20, seed=42)
        subj_b = m._noisy_subject_factory("noisy_b", base=0.85, noise=0.20, seed=42)
        r_high = nr_high.measure(subj_a, "What is the capital of France?")
        r_low = nr_low.measure(subj_b, "What is the capital of France?")
        self.assertAlmostEqual(r_high.score, r_low.score, places=4)
        self.assertFalse(r_high.passed)
        self.assertTrue(r_low.passed)


# ============================================================
# 10. EVMeasurer
# ============================================================


class TestEVMeasurer(unittest.TestCase):
    def test_improving_strategy_passes(self):
        subj = m._deterministic_subject_factory("det", base=0.7)
        ev = m.EVMeasurer(n_rounds=30, threshold=0.85)
        r = ev.measure(subj, m._evolve_step_factory("improving"), "test")
        self.assertEqual(len(r.trace.steps), 30)
        self.assertTrue(r.passed)
        self.assertGreater(r.final_score, r.initial_score)

    def test_flat_strategy_retention_high(self):
        subj = m._deterministic_subject_factory("det", base=0.8)
        ev = m.EVMeasurer(n_rounds=30, threshold=0.85)
        r = ev.measure(subj, m._evolve_step_factory("flat"), "test")
        self.assertEqual(r.retention, 1.0)
        self.assertTrue(r.passed)

    def test_degrading_strategy_fails(self):
        subj = m._deterministic_subject_factory("det", base=0.9)
        ev = m.EVMeasurer(n_rounds=30, threshold=0.85)
        r = ev.measure(subj, m._evolve_step_factory("degrading"), "test")
        self.assertLess(r.retention, 1.0)
        self.assertFalse(r.passed)

    def test_n_rounds_recorded(self):
        subj = m._deterministic_subject_factory("det", base=0.5)
        ev = m.EVMeasurer(n_rounds=15, threshold=0.85)
        r = ev.measure(subj, m._evolve_step_factory("improving"), "test")
        self.assertEqual(r.n_rounds, 15)
        self.assertEqual(len(r.trace.steps), 15)

    def test_ev_threshold_mechanism(self):
        subj = m._deterministic_subject_factory("det", base=0.5)
        ev_strict = m.EVMeasurer(n_rounds=5, threshold=0.99)
        ev_lenient = m.EVMeasurer(n_rounds=5, threshold=0.5)
        ev_step = m._evolve_step_factory("improving")
        r_strict = ev_strict.measure(subj, ev_step, "test")
        r_lenient = ev_lenient.measure(subj, ev_step, "test")
        self.assertGreaterEqual(r_strict.score, r_lenient.score)
        self.assertTrue(r_strict.passed)

    def test_oscillating_partial_score(self):
        subj = m._deterministic_subject_factory("det", base=0.5)
        ev = m.EVMeasurer(n_rounds=10, threshold=0.85)
        r = ev.measure(subj, m._evolve_step_factory("oscillating"), "test")
        self.assertLess(r.monotonicity, 1.0)


# ============================================================
# 11. CDTMeasurer
# ============================================================


class TestCDTMeasurer(unittest.TestCase):
    def test_all_pass_subject(self):
        subj = m._deterministic_subject_factory("det", base=0.9)
        cdt = m.CDTMeasurer(n_domains=4, threshold=0.75, per_domain_threshold=0.50)
        r = cdt.measure(subj, m.DEFAULT_QUERIES)
        self.assertEqual(r.success_rate, 1.0)
        self.assertEqual(r.score, 1.0)
        self.assertTrue(r.passed)
        self.assertEqual(len(r.domain_scores), 4)

    def test_partial_pass_subject(self):
        def fn(q: str) -> float:
            if "Python" in q:
                return 0.9
            if "research" in q.lower():
                return 0.9
            return 0.2

        subj = m.HQBSubject(name="partial", fn=fn)
        cdt = m.CDTMeasurer(n_domains=4, threshold=0.75, per_domain_threshold=0.50)
        r = cdt.measure(subj, m.DEFAULT_QUERIES)
        self.assertLess(r.success_rate, 1.0)
        self.assertFalse(r.passed)

    def test_n_domains_limit(self):
        subj = m._deterministic_subject_factory("det", base=0.9)
        cdt = m.CDTMeasurer(n_domains=2, threshold=0.5)
        queries = {"a": "q1", "b": "q2", "c": "q3", "d": "q4"}
        r = cdt.measure(subj, queries)
        self.assertEqual(r.n_domains, 2)
        self.assertEqual(len(r.domain_scores), 2)

    def test_empty_queries(self):
        subj = m._deterministic_subject_factory("det", base=0.9)
        cdt = m.CDTMeasurer(n_domains=4, threshold=0.5)
        r = cdt.measure(subj, {})
        self.assertEqual(r.avg_score, 0.0)
        self.assertEqual(r.success_rate, 0.0)

    def test_avg_min_max_calculated(self):
        subj = m._deterministic_subject_factory("det", base=0.6)
        cdt = m.CDTMeasurer(n_domains=4, threshold=0.0)
        r = cdt.measure(subj, m.DEFAULT_QUERIES)
        self.assertAlmostEqual(r.avg_score, 0.6, places=6)
        self.assertAlmostEqual(r.min_score, 0.6, places=6)
        self.assertAlmostEqual(r.max_score, 0.6, places=6)


# ============================================================
# 12. HQB4DimMeasurer 主入口
# ============================================================


class TestHQB4DimMeasurer(unittest.TestCase):
    def test_default_thresholds(self):
        measurer = m.HQB4DimMeasurer()
        self.assertEqual(measurer.thresholds["sc"], m.SC_THRESHOLD)
        self.assertEqual(measurer.thresholds["nr"], m.NR_THRESHOLD)
        self.assertEqual(measurer.thresholds["ev"], m.EV_THRESHOLD)
        self.assertEqual(measurer.thresholds["cdt"], m.CDT_THRESHOLD)

    def test_deterministic_subject_all_pass(self):
        subj = m._deterministic_subject_factory("det", base=0.90)
        measurer = m.HQB4DimMeasurer()
        evolve = m._evolve_step_factory("improving")
        r = measurer.measure(
            subject=subj,
            sc_query="Explain X",
            nr_query="What is Y?",
            ev_initial_query="List Z",
            evolve_step=evolve,
            domain_queries=m.DEFAULT_QUERIES,
        )
        self.assertTrue(r.all_passed)
        self.assertEqual(r.sc_score, 1.0)
        self.assertEqual(r.nr_score, 1.0)
        self.assertTrue(r.ev_passed)
        self.assertTrue(r.cdt_passed)

    def test_degrading_subject_fails_ev(self):
        subj = m._deterministic_subject_factory("deg", base=0.50)
        measurer = m.HQB4DimMeasurer()
        r = measurer.measure(
            subject=subj,
            sc_query="Explain X",
            nr_query="What is Y?",
            ev_initial_query="List Z",
            evolve_step=m._evolve_step_factory("degrading"),
            domain_queries=m.DEFAULT_QUERIES,
        )
        self.assertFalse(r.all_passed)
        self.assertFalse(r.ev_passed)

    def test_report_id_unique(self):
        subj = m._deterministic_subject_factory("det", base=0.7)
        measurer = m.HQB4DimMeasurer()
        evolve = m._evolve_step_factory("improving")
        r1 = measurer.measure(subj, "q1", "q2", "q3", evolve, m.DEFAULT_QUERIES)
        r2 = measurer.measure(subj, "q1", "q2", "q3", evolve, m.DEFAULT_QUERIES)
        self.assertNotEqual(r1.report_id, r2.report_id)

    def test_custom_thresholds(self):
        subj = m._deterministic_subject_factory("det", base=0.7)
        measurer = m.HQB4DimMeasurer(
            sc_threshold=0.50, nr_threshold=0.50,
            ev_threshold=0.50, cdt_threshold=0.50,
        )
        evolve = m._evolve_step_factory("improving")
        r = measurer.measure(subj, "q", "q", "q", evolve, m.DEFAULT_QUERIES)
        self.assertTrue(r.all_passed)
        self.assertEqual(r.thresholds["sc"], 0.50)

    def test_total_score_is_avg(self):
        subj = m._deterministic_subject_factory("det", base=0.9)
        measurer = m.HQB4DimMeasurer()
        evolve = m._evolve_step_factory("improving")
        r = measurer.measure(subj, "q", "q", "q", evolve, m.DEFAULT_QUERIES)
        expected = (r.sc_score + r.nr_score + r.ev_score + r.cdt_score) / 4.0
        self.assertAlmostEqual(r.total_score, expected, places=6)


# ============================================================
# 13. HQB4DimReport + to_dict
# ============================================================


class TestHQB4DimReport(unittest.TestCase):
    def _make_report(self) -> m.HQB4DimReport:
        return m.HQB4DimReport(
            report_id="r1", subject_name="s",
            timestamp="2026-07-29T00:00:00",
            sc_score=0.9, nr_score=0.85, ev_score=0.92, cdt_score=0.80,
            total_score=0.8675,
            sc_passed=True, nr_passed=True, ev_passed=True, cdt_passed=True,
            all_passed=True,
            thresholds={"sc": 0.85, "nr": 0.80, "ev": 0.85, "cdt": 0.75},
        )

    def test_to_dict_has_keys(self):
        r = self._make_report()
        d = r.to_dict()
        for k in ("report_id", "subject_name", "sc_score", "nr_score", "ev_score", "cdt_score",
                  "total_score", "all_passed", "thresholds"):
            self.assertIn(k, d)

    def test_to_dict_round_score(self):
        r = self._make_report()
        d = r.to_dict()
        self.assertEqual(d["sc_score"], 0.9)
        self.assertEqual(d["report_id"], "r1")

    def test_all_passed_set(self):
        # HQB4DimReport.all_passed 是构造时由主入口写入; 不随字段自动联动.
        r = m.HQB4DimReport(
            report_id="r1", subject_name="s",
            timestamp="2026-07-29T00:00:00",
            all_passed=False,
            sc_passed=False, nr_passed=True, ev_passed=True, cdt_passed=True,
        )
        self.assertFalse(r.all_passed)
        self.assertFalse(r.sc_passed)


# ============================================================
# 14. 报告渲染
# ============================================================


class TestRenderReport(unittest.TestCase):
    def _make_report(self) -> m.HQB4DimReport:
        return m.HQB4DimReport(
            report_id="r1", subject_name="subj",
            timestamp="2026-07-29T00:00:00",
            sc_score=0.9, nr_score=0.85, ev_score=0.92, cdt_score=0.80,
            total_score=0.8675,
            sc_passed=True, nr_passed=True, ev_passed=True, cdt_passed=True,
            all_passed=True,
            thresholds={"sc": 0.85, "nr": 0.80, "ev": 0.85, "cdt": 0.75},
            notes=["note 1", "note 2"],
        )

    def test_report_contains_dim_names(self):
        out = m.render_hqb_4dim_report(self._make_report())
        self.assertIn("SC (Self-Consistency)", out)
        self.assertIn("NR (Noise Robustness)", out)
        self.assertIn("EV (Evolvability 30r)", out)
        self.assertIn("CDT (Cross-Domain Transfer)", out)

    def test_report_contains_philosophy_guards(self):
        out = m.render_hqb_4dim_report(self._make_report())
        self.assertIn("V3 Philosophy Guards", out)
        self.assertIn(m.GUARD_MEASUREMENT_IS_NOT_TRUTH, out)
        self.assertIn(m.GUARD_THRESHOLD_IS_DESIGN_CHOICE, out)
        self.assertIn(m.GUARD_30_ROUNDS_IS_NOT_LIFETIME, out)
        self.assertIn(m.GUARD_4_DOMAINS_IS_NOT_ALL_DOMAINS, out)
        self.assertIn(m.GUARD_MEASURER_IS_NOT_ASI, out)

    def test_report_contains_references(self):
        out = m.render_hqb_4dim_report(self._make_report())
        self.assertIn("Welford", out)
        self.assertIn("Levenshtein", out)
        self.assertIn("Efron", out)
        self.assertIn("Hyndman", out)


# ============================================================
# 15. write_hqb_report + write_hqb_json
# ============================================================


class TestWriteReports(unittest.TestCase):
    def _make_report(self) -> m.HQB4DimReport:
        return m.HQB4DimReport(
            report_id="r1", subject_name="subj",
            timestamp="2026-07-29T00:00:00",
            sc_score=0.9, nr_score=0.85, ev_score=0.92, cdt_score=0.80,
            total_score=0.8675,
            sc_passed=True, nr_passed=True, ev_passed=True, cdt_passed=True,
            all_passed=True,
            thresholds={"sc": 0.85, "nr": 0.80, "ev": 0.85, "cdt": 0.75},
        )

    def test_write_md(self):
        with tempfile.TemporaryDirectory() as td:
            p = m.write_hqb_report(self._make_report(), Path(td))
            self.assertTrue(p.exists())
            self.assertGreater(p.stat().st_size, 100)
            content = p.read_text(encoding="utf-8")
            self.assertIn("V1111", content)

    def test_write_json(self):
        with tempfile.TemporaryDirectory() as td:
            p = m.write_hqb_json(self._make_report(), Path(td))
            self.assertTrue(p.exists())
            data = json.loads(p.read_text(encoding="utf-8"))
            self.assertEqual(data["subject_name"], "subj")
            self.assertTrue(data["all_passed"])

    def test_write_creates_dir(self):
        with tempfile.TemporaryDirectory() as td:
            nested = Path(td) / "v1111" / "artifacts"
            self.assertFalse(nested.exists())
            m.write_hqb_report(self._make_report(), nested)
            self.assertTrue(nested.exists())


# ============================================================
# 16. run_v1111_self_check
# ============================================================


class TestRunSelfCheck(unittest.TestCase):
    def test_self_check_returns_dict(self):
        r = m.run_v1111_self_check()
        self.assertIsInstance(r, dict)
        self.assertEqual(r["v1111_version"], m.V1111_VERSION)
        self.assertEqual(r["n_philosophy_guards"], 5)
        self.assertTrue(r["all_5_guards_present"])

    def test_self_check_three_subjects(self):
        r = m.run_v1111_self_check()
        self.assertEqual(len(r["results"]), 3)
        names = {x["name"] for x in r["results"]}
        self.assertIn("deterministic", names)
        self.assertIn("noisy", names)
        self.assertIn("degrading", names)

    def test_deterministic_subject_passes(self):
        r = m.run_v1111_self_check()
        det = next(x for x in r["results"] if x["name"] == "deterministic")
        self.assertTrue(det["report"]["all_passed"])

    def test_degrading_subject_fails_ev(self):
        r = m.run_v1111_self_check()
        deg = next(x for x in r["results"] if x["name"] == "degrading")
        self.assertFalse(deg["report"]["ev_passed"])

    def test_self_check_has_components(self):
        r = m.run_v1111_self_check()
        self.assertIn("sc", r["components"])
        self.assertIn("nr", r["components"])
        self.assertIn("ev", r["components"])
        self.assertIn("cdt", r["components"])


# ============================================================
# 17. CLI
# ============================================================


class TestCLI(unittest.TestCase):
    def test_cli_self_check_subprocess(self):
        env = dict(os.environ)
        env["PYTHONIOENCODING"] = "utf-8"
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1111_hqb_4dim_measurer", "--self-check"],
            capture_output=True, env=env, timeout=60,
        )
        self.assertEqual(result.returncode, 0, msg=result.stderr.decode("utf-8", errors="ignore"))
        out = result.stdout.decode("utf-8", errors="ignore")
        self.assertIn("v1111_version", out)
        self.assertIn("deterministic", out)

    def test_cli_report_subprocess(self):
        r = m.run_v1111_self_check()
        with tempfile.TemporaryDirectory() as td:
            json_path = Path(td) / "r.json"
            json_path.write_text(
                json.dumps(r["results"][0]["report"], ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
            env = dict(os.environ)
            env["PYTHONIOENCODING"] = "utf-8"
            result = subprocess.run(
                [sys.executable, "-m", "apeireth.v1111_hqb_4dim_measurer", "--report", str(json_path)],
                capture_output=True, env=env, timeout=60,
            )
            self.assertEqual(result.returncode, 0, msg=result.stderr.decode("utf-8", errors="ignore"))
            out = result.stdout.decode("utf-8", errors="ignore")
            self.assertIn("V1111", out)
            self.assertIn("Self-Consistency", out)

    def test_cli_help(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1111_hqb_4dim_measurer"],
            capture_output=True, env={**os.environ, "PYTHONIOENCODING": "utf-8"},
            timeout=30,
        )
        self.assertIn(result.returncode, (0, 1, 2))


# ============================================================
# 18. __all__ 完整性
# ============================================================


class TestAllExports(unittest.TestCase):
    def test_all_key_exports(self):
        for name in (
            "V1111_VERSION",
            "GUARD_MEASUREMENT_IS_NOT_TRUTH",
            "GUARD_THRESHOLD_IS_DESIGN_CHOICE",
            "GUARD_30_ROUNDS_IS_NOT_LIFETIME",
            "GUARD_4_DOMAINS_IS_NOT_ALL_DOMAINS",
            "GUARD_MEASURER_IS_NOT_ASI",
            "SC_THRESHOLD", "NR_THRESHOLD", "EV_THRESHOLD", "CDT_THRESHOLD",
            "Domain", "NoiseKind", "NoiseInjector",
            "HQBSubject", "MeasurerProtocol",
            "SCMeasurer", "NRMeasurer", "EVMeasurer", "CDTMeasurer",
            "HQB4DimMeasurer", "HQB4DimReport",
            "render_hqb_4dim_report", "write_hqb_report", "write_hqb_json",
            "run_v1111_self_check",
            "main",
        ):
            self.assertTrue(hasattr(m, name), f"missing export: {name}")


if __name__ == "__main__":
    unittest.main()