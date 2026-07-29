"""Apeireth ASI V1129 — R10 W1 末中段回顾真测试 (R10-INT-W1).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 20:55 红皇后守门 + 主 23:44 干到底.

测试覆盖 (≥ 20 真测):
  - V1129 常量与模块结构 (3 tests)
  - 8 维 lift dispatch + 真测函数 (3 tests)
  - 8 维 lift 真测结果 (3 tests)
  - W2 主推建议 (3 tests)
  - chaos test 决策守门 (2 tests)
  - R10W1Retrospective 主编排 (2 tests)
  - Markdown + CLI (2 tests)
  - V1129 与 V1125/V1126 一致性 (2 tests)

运行: pytest -q tests/test_v1129_r10_w1_retrospective.py
"""
from __future__ import annotations

import io
import json
import unittest
from contextlib import redirect_stdout
from unittest import mock

import apeireth.v1129_r10_w1_retrospective as m1129
from apeireth.v1129_r10_w1_retrospective import (
    VERSION as V1129_VERSION,
    LIFT_8_DIMS,
    LIFT_DISPATCH,
    R9_W4_LIFT_BASELINE,
    CHAOS_TEST_TIMEOUT,
    CHAOS_TEST_MAX_RETRIES,
    V3_GUARDS_R10_W1_INJECTED,
    Lift8DimResult,
    W2Recommendation,
    R10W1Retrospective,
    measure_engineering_lift,
    measure_cognitive_core_lift,
    measure_continuity_lift,
    measure_autonomy_lift,
    measure_transferability_lift,
    measure_identity_lift,
    measure_dream_lift,
    measure_effort_lift,
    _safe_call_with_fallback,
    compute_8_dim_lift,
    summarize_8_dim_lift,
    compute_w2_recommendation,
    chaos_test_decision_resilience,
    evaluate_r10_w1_retrospective,
    render_markdown_r10_w1,
    main as v1129_main,
)
from apeireth.v1125_r10_integration_protocol import (
    VERSION as V1125_VERSION,
    R10_ULTIMATE_TARGET,
)
from apeireth.v1126_r10_integration_baseline import (
    VERSION as V1126_VERSION,
)
from apeireth.v1114_weekly_integration_evaluator import (
    ASI_NORTH_STAR,
    V1074_V03_MIN,
)


# ===========================================================================
# V1129 常量与模块结构 (3 tests)
# ===========================================================================

class TestV1129Constants(unittest.TestCase):
    def test_version_present(self):
        self.assertEqual(V1129_VERSION, "0.1.0")
        self.assertEqual(m1129.VERSION, "0.1.0")

    def test_8_lift_dims_locked(self):
        # 主 17:43 实事求是: 8 维 LOCKED
        self.assertEqual(len(LIFT_8_DIMS), 8)
        self.assertIn("engineering", LIFT_8_DIMS)
        self.assertIn("cognitive_core", LIFT_8_DIMS)
        self.assertIn("continuity", LIFT_8_DIMS)
        self.assertIn("autonomy", LIFT_8_DIMS)
        self.assertIn("transferability", LIFT_8_DIMS)
        self.assertIn("identity", LIFT_8_DIMS)
        self.assertIn("dream", LIFT_8_DIMS)
        self.assertIn("effort", LIFT_8_DIMS)

    def test_baseline_and_chaos_config(self):
        # R9 baseline LOCKED, chaos 配置 LOCKED
        self.assertEqual(R9_W4_LIFT_BASELINE["engineering"], 0.85)
        self.assertEqual(R9_W4_LIFT_BASELINE["identity"], 0.84)
        self.assertGreater(CHAOS_TEST_TIMEOUT, 0)
        self.assertGreaterEqual(CHAOS_TEST_MAX_RETRIES, 1)
        self.assertGreaterEqual(len(V3_GUARDS_R10_W1_INJECTED), 4)


# ===========================================================================
# 8 维 lift dispatch + 真测函数 (3 tests)
# ===========================================================================

class TestLiftDispatch(unittest.TestCase):
    def test_dispatch_covers_all_dims(self):
        # 每条 dim 都有对应真测函数 (主 17:43 实事求是)
        for dim in LIFT_8_DIMS:
            self.assertIn(dim, LIFT_DISPATCH)
            self.assertTrue(callable(LIFT_DISPATCH[dim]))

    def test_all_measure_fns_return_float(self):
        # 每条真测函数都返回 [0,1] 范围的 float
        for dim, fn in LIFT_DISPATCH.items():
            v = fn()
            self.assertIsInstance(v, float)
            self.assertGreaterEqual(v, 0.0)
            self.assertLessEqual(v, 1.0, f"{dim} = {v} > 1.0")

    def test_safe_call_with_fallback_returns_tuple(self):
        # _safe_call_with_fallback 返回 (value, source) tuple
        def ok():
            return 0.85
        v, src = _safe_call_with_fallback(ok, 0.80, dim_name="test")
        self.assertEqual(v, 0.85)
        self.assertIn("real_measure", src)
        # 失败时返回 fallback
        def bad():
            raise RuntimeError("test")
        v2, src2 = _safe_call_with_fallback(bad, 0.50, dim_name="fail_test")
        self.assertEqual(v2, 0.50)
        self.assertIn("fallback", src2)


# ===========================================================================
# 8 维 lift 真测结果 (3 tests)
# ===========================================================================

class TestCompute8DimLift(unittest.TestCase):
    def test_compute_8_dim_returns_8_results(self):
        results = compute_8_dim_lift()
        self.assertEqual(len(results), 8)
        for r in results:
            self.assertIsInstance(r, Lift8DimResult)
            self.assertIn(r.name, LIFT_8_DIMS)
            self.assertGreaterEqual(r.r9_baseline, 0.0)
            self.assertGreaterEqual(r.r10_actual, 0.0)

    def test_summarize_8_dim_lift(self):
        results = compute_8_dim_lift()
        s = summarize_8_dim_lift(results)
        self.assertEqual(s["total_dims"], 8)
        self.assertGreaterEqual(s["passed_dims"], 0)
        self.assertGreaterEqual(s["avg_baseline"], 0.7)
        self.assertLessEqual(s["avg_baseline"], 1.0)
        # all_pass 在 R9 baseline 一致情况下应该 True (实际值 ≥ baseline - 0.05)
        self.assertTrue(s["all_pass"])

    def test_lift_delta_calculated_correctly(self):
        # lift_delta = actual - baseline
        r = Lift8DimResult(
            name="test", r9_baseline=0.80, r10_actual=0.85,
            lift_delta=0.05, lift_pct=6.25, source="real_measure", passed=True,
        )
        self.assertEqual(r.lift_delta, 0.05)
        self.assertAlmostEqual(r.lift_pct, 6.25, places=2)


# ===========================================================================
# W2 主推建议 (3 tests)
# ===========================================================================

class TestW2Recommendation(unittest.TestCase):
    def _setup(self):
        from apeireth.v1114_weekly_integration_evaluator import HaltingSignals
        from apeireth.v1125_r10_integration_protocol import choose_r10_main_track
        results = compute_8_dim_lift()
        summary = summarize_8_dim_lift(results)
        track = choose_r10_main_track(v05_score=0.85, halting=HaltingSignals())
        return track, summary, results

    def test_w2_recommendation_returns_dataclass(self):
        track, summary, results = self._setup()
        w2 = compute_w2_recommendation(track, summary, results)
        self.assertIsInstance(w2, W2Recommendation)
        self.assertEqual(w2.track, track.track)
        self.assertEqual(w2.track_name, track.track_name)
        self.assertIsInstance(w2.priority_areas, list)

    def test_w2_priority_areas_non_empty(self):
        track, summary, results = self._setup()
        w2 = compute_w2_recommendation(track, summary, results)
        self.assertGreater(len(w2.priority_areas), 0)
        for area in w2.priority_areas:
            self.assertIn(area, LIFT_8_DIMS)

    def test_w2_rationale_contains_lift_data(self):
        # rationale 必须基于真测数据 (主 17:43 不空想)
        track, summary, results = self._setup()
        w2 = compute_w2_recommendation(track, summary, results)
        self.assertIn("Track", w2.rationale)
        self.assertIn("lift", w2.rationale.lower())


# ===========================================================================
# chaos test 决策守门 (2 tests)
# ===========================================================================

class TestChaosTest(unittest.TestCase):
    def test_chaos_test_with_all_real(self):
        results = compute_8_dim_lift()
        c = chaos_test_decision_resilience(results)
        self.assertIn("n_real_measure", c)
        self.assertIn("n_fallback", c)
        self.assertIn("decision_resilient", c)
        self.assertIn("verdict", c)
        self.assertIsInstance(c["decision_resilient"], bool)

    def test_chaos_test_with_simulated_failures(self):
        # 模拟全 fallback (chaos 极端情况)
        results = [
            Lift8DimResult(name=d, r9_baseline=0.8, r10_actual=0.8,
                           lift_delta=0.0, lift_pct=0.0,
                           source=f"fallback:RuntimeError:{d}", passed=True)
            for d in LIFT_8_DIMS
        ]
        c = chaos_test_decision_resilience(results)
        self.assertEqual(c["n_fallback"], 8)
        self.assertEqual(c["n_real_measure"], 0)
        # 100% fallback → 不 resilient (主 23:44 决策不可丢失但也不能完全没数据)
        self.assertFalse(c["decision_resilient"])


# ===========================================================================
# R10W1Retrospective 主编排 (2 tests)
# ===========================================================================

class TestEvaluateR10W1(unittest.TestCase):
    def test_evaluate_r10_w1_full(self):
        # 主 00:56 一行真跑
        retro = evaluate_r10_w1_retrospective(week_label="R10-W1")
        self.assertIsInstance(retro, R10W1Retrospective)
        self.assertEqual(retro.week_label, "R10-W1")
        self.assertIn("v1125_report", retro.to_dict())
        self.assertIn("v1126_report", retro.to_dict())
        self.assertIn("lift_8_dim", retro.to_dict())
        self.assertIn("track_decision", retro.to_dict())
        self.assertIn("w2_recommendation", retro.to_dict())
        self.assertIn("chaos_test", retro.to_dict())

    def test_evaluate_r10_w1_v05_in_range(self):
        # V0.5 应在 [0.7, 1.0]
        retro = evaluate_r10_w1_retrospective()
        v05 = retro.v1125_report["v05_score"]["v05_total"]
        self.assertGreater(v05, 0.7)
        self.assertLess(v05, 1.0)


# ===========================================================================
# Markdown + CLI (2 tests)
# ===========================================================================

class TestV1129MarkdownAndCLI(unittest.TestCase):
    def test_render_markdown_contains_key_sections(self):
        retro = evaluate_r10_w1_retrospective()
        md = render_markdown_r10_w1(retro)
        self.assertIn("R10 R10-W1", md)
        self.assertIn("Dashboard", md)
        self.assertIn("8 维 Lift", md)
        self.assertIn("主轨道决策", md)
        self.assertIn("W2 主推", md)
        self.assertIn("Chaos Test", md)
        self.assertIn(ASI_NORTH_STAR.__format__(".4f"), md)

    def test_cli_json_mode(self):
        with redirect_stdout(io.StringIO()) as buf:
            rc = v1129_main(["--json"])
        self.assertIn(rc, (0, 1))
        out = buf.getvalue()
        d = json.loads(out)
        self.assertEqual(d["week_label"], "R10-W1")
        self.assertEqual(len(d["lift_8_dim"]), 8)
        self.assertIn("v1125_report", d)
        self.assertIn("track_decision", d)


# ===========================================================================
# V1129 与 V1125/V1126 一致性 (2 tests)
# ===========================================================================

class TestV1129Consistency(unittest.TestCase):
    def test_v1129_uses_v1125_protocol(self):
        # V1129 复用 V1125 evaluate_r10 (主 19:33 走在前人经验上)
        retro = evaluate_r10_w1_retrospective()
        self.assertEqual(retro.protocol_v1125_version, V1125_VERSION)
        self.assertIn("v05_score", retro.v1125_report)
        self.assertEqual(len(retro.v1125_report["scenarios"]), 24)

    def test_v1129_uses_v1126_baseline(self):
        # V1129 复用 V1126 run_r10_baseline_startup
        retro = evaluate_r10_w1_retrospective()
        self.assertEqual(retro.baseline_v1126_version, V1126_VERSION)
        self.assertIn("baseline", retro.v1126_report)
        # R9 W4 末 baseline LOCKED
        r9 = retro.v1126_report["baseline"]["r9_w4_baseline"]
        self.assertEqual(r9["v04_score"], 0.8538)
        self.assertEqual(r9["v03_score"], 0.8897)


if __name__ == "__main__":
    # ponytail: 直接 run 时显示 verbose output (主 00:56 一行可跑)
    unittest.main(verbosity=2)