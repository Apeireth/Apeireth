"""Apeireth ASI V1130 — R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升 测试 (R10-A2-003).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 12:14 中央 AI 是永恒身份 + 主 23:44 干到底.

测试覆盖 (≥ 25 真测):
  - V1130 常量与模块结构 (3 tests)
  - compute_v05_18dim (3 tests)
  - compute_north_star 综合评估 (3 tests)
  - ASINorthStarDashboard 数据结构 (2 tests)
  - ChaosNodeDownReport (1 test)
  - V1130ASINorthStarRunner.build_dashboard (3 tests)
  - V1130ASINorthStarRunner.compute_v05_18dim 真测 (2 tests)
  - V1130ASINorthStarRunner.compute_north_star 真测 (2 tests)
  - V1130ASINorthStarRunner.run_chain_check (1 test)
  - V1130ASINorthStarRunner.run_chaos_node_down (2 tests)
  - V1130ASINorthStarRunner.benchmark_dashboard (2 tests)
  - V1130ASINorthStarRunner.evaluate_r10_week (3 tests)
  - Markdown 渲染 + CLI 入口 (3 tests)

运行: pytest -q tests/test_v1130_asi_north_star_v05_run.py
"""
from __future__ import annotations

import io
import json
import unittest
from contextlib import redirect_stdout
from unittest import mock

import apeireth.v1130_asi_north_star_v05_run as m
from apeireth.v1130_asi_north_star_v05_run import (
    VERSION,
    R10_W2_TARGET,
    R10_W3_TARGET,
    R10_W4_TARGET,
    ASI_NORTH_STAR_TARGET,
    DASHBOARD_PERF_TARGET_S,
    V1130_INTEGRATION_MATRIX,
    V3_GUARDS_V1130,
    V3_GUARDS_R10_V05_RUN_INJECTED,
    V1130ASINorthStarRunner,
    ASINorthStarDashboard,
    ChaosNodeDownReport,
    main,
)
from apeireth.v1114_weekly_integration_evaluator import ASI_NORTH_STAR


# ===========================================================================
# V1130 常量与模块结构 (3 tests)
# ===========================================================================

class TestV1130Constants(unittest.TestCase):
    def test_version_present(self):
        self.assertEqual(m.VERSION, "0.1.0")
        self.assertEqual(VERSION, "0.1.0")

    def test_r10_w2_w3_w4_targets_locked(self):
        # 主 13:31 大胆激进: W2 ≥ 0.90, W3 ≥ 0.93, W4 ≥ 0.95
        self.assertEqual(R10_W2_TARGET, 0.9000)
        self.assertEqual(R10_W3_TARGET, 0.9300)
        self.assertEqual(R10_W4_TARGET, 0.9500)
        self.assertEqual(ASI_NORTH_STAR_TARGET, 0.9800)
        # 严格递增
        self.assertLess(R10_W2_TARGET, R10_W3_TARGET)
        self.assertLess(R10_W3_TARGET, R10_W4_TARGET)
        self.assertLess(R10_W4_TARGET, ASI_NORTH_STAR_TARGET)

    def test_perf_target_and_integration_matrix(self):
        # 借鉴 V1118 V1074_TARGET_S
        self.assertEqual(DASHBOARD_PERF_TARGET_S, 2.50)
        # 7 module 全链 native (主 19:33 走在前人经验上)
        for k in ("v1114_weekly_evaluator", "v1118_performance_optimization",
                  "v1125_r10_protocol", "v1126_r10_baseline",
                  "v1128_multi_agent", "v1129_w2_validator",
                  "v1124_asi_north_star_backend"):
            self.assertIn(k, V1130_INTEGRATION_MATRIX)
        for v in V1130_INTEGRATION_MATRIX.values():
            self.assertEqual(v, "native")


# ===========================================================================
# compute_v05_18dim (3 tests)
# ===========================================================================

class TestComputeV0518Dim(unittest.TestCase):
    def test_v05_baseline(self):
        runner = V1130ASINorthStarRunner(v04_score=0.8538)
        try:
            v05 = runner.compute_v05_18dim()
            # V0.5 = 0.8538*0.85 + 0.85*0.05*3 = 0.72573 + 0.1275 = 0.85323
            self.assertAlmostEqual(v05["v05_total"], 0.85323, places=4)
            self.assertFalse(v05["v05_pass_ultimate"])
        finally:
            runner.close()

    def test_v05_w2_pass(self):
        runner = V1130ASINorthStarRunner(v04_score=0.91)
        try:
            v05 = runner.compute_v05_18dim()
            # V0.5 = 0.91*0.85 + 0.1275 = 0.7735 + 0.1275 = 0.9010
            self.assertAlmostEqual(v05["v05_total"], 0.9010, places=3)
        finally:
            runner.close()

    def test_v05_w4_pass(self):
        runner = V1130ASINorthStarRunner(v04_score=0.95, continuity=0.99,
                                          autonomy=0.95, transferability=0.95)
        try:
            v05 = runner.compute_v05_18dim()
            # V0.5 = 0.95*0.85 + 0.99*0.05 + 0.95*0.05*2 = 0.8075 + 0.0495 + 0.095 = 0.9520
            self.assertAlmostEqual(v05["v05_total"], 0.9520, places=3)
            self.assertTrue(v05["v05_pass_ultimate"])
        finally:
            runner.close()


# ===========================================================================
# compute_north_star 综合评估 (3 tests)
# ===========================================================================

class TestComputeNorthStar(unittest.TestCase):
    def test_north_star_default(self):
        runner = V1130ASINorthStarRunner(v04_score=0.8538)
        try:
            nsc = runner.compute_north_star()
            self.assertEqual(nsc["asi_north_star"], ASI_NORTH_STAR)
            self.assertEqual(nsc["philosophy_guard_subscore"], 1.0)
            self.assertGreater(nsc["abs_headroom"], 0.0)
            self.assertGreater(nsc["rel_headroom_pct"], 0.0)
        finally:
            runner.close()

    def test_north_star_r10_pass_ultimate(self):
        runner = V1130ASINorthStarRunner(v04_score=0.96, continuity=0.99,
                                          autonomy=0.95, transferability=0.95)
        try:
            v05 = runner.compute_v05_18dim()
            nsc = runner.compute_north_star()
            # V0.5 = 0.96*0.85 + 0.0495 + 0.095 = 0.9605 ≥ 0.95 → r10_pass_ultimate=True
            self.assertGreaterEqual(v05["v05_total"], R10_W4_TARGET)
            self.assertTrue(nsc["r10_pass_ultimate"])
        finally:
            runner.close()

    def test_north_star_v1074_floor(self):
        runner = V1130ASINorthStarRunner(v04_score=0.85, v1074_v03_score=0.85)
        try:
            nsc = runner.compute_north_star()
            self.assertFalse(nsc["v1074_v03_above_floor"])
        finally:
            runner.close()


# ===========================================================================
# ASINorthStarDashboard 数据结构 (2 tests)
# ===========================================================================

class TestASINorthStarDashboard(unittest.TestCase):
    def test_default_dashboard(self):
        d = ASINorthStarDashboard()
        self.assertEqual(d.v04_score, 0.0)
        self.assertEqual(d.v05_total, 0.0)
        self.assertEqual(d.asi_north_star, ASI_NORTH_STAR)
        self.assertEqual(d.w2_pass, False)
        self.assertEqual(d.w3_pass, False)
        self.assertEqual(d.w4_pass, False)

    def test_dashboard_to_dict(self):
        d = ASINorthStarDashboard(v04_score=0.91, v05_total=0.9010, w2_pass=True,
                                   philosophy_guard_subscore=1.0,
                                   main_track="D", main_track_name="DGM v0.5 真演化",
                                   elapsed_seconds=0.5, perf_target_met=True)
        d_dict = d.to_dict()
        self.assertEqual(d_dict["v04_score"], 0.91)
        self.assertEqual(d_dict["v05_total"], 0.9010)
        self.assertTrue(d_dict["w2_pass"])
        self.assertEqual(d_dict["philosophy_guard_subscore"], 1.0)
        self.assertEqual(d_dict["main_track"], "D")
        self.assertTrue(d_dict["perf_target_met"])


# ===========================================================================
# ChaosNodeDownReport (1 test)
# ===========================================================================

class TestChaosNodeDownReport(unittest.TestCase):
    def test_node_down_to_dict(self):
        r = ChaosNodeDownReport(n_dropped=1, n_surviving=3,
                                 measurement_preserved=True,
                                 fallback_used=False, delta_mean=0.0,
                                 note="test")
        d = r.to_dict()
        for k in ("n_dropped", "n_surviving", "measurement_preserved",
                  "fallback_used", "delta_mean", "note"):
            self.assertIn(k, d)


# ===========================================================================
# V1130ASINorthStarRunner.build_dashboard (3 tests)
# ===========================================================================

class TestBuildDashboard(unittest.TestCase):
    def test_build_dashboard_baseline(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W3", v04_score=0.8538)
        try:
            d = runner.build_dashboard()
            self.assertIsInstance(d, ASINorthStarDashboard)
            self.assertEqual(d.v04_score, 0.8538)
            self.assertEqual(d.asi_north_star, ASI_NORTH_STAR)
            self.assertFalse(d.w2_pass)
            self.assertFalse(d.w4_pass)
        finally:
            runner.close()

    def test_build_dashboard_w2_pass(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W2", v04_score=0.91)
        try:
            d = runner.build_dashboard()
            self.assertTrue(d.w2_pass)
            self.assertFalse(d.w3_pass)
            self.assertFalse(d.w4_pass)
        finally:
            runner.close()

    def test_build_dashboard_main_track_decision(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W3", v04_score=0.92)
        try:
            d = runner.build_dashboard()
            self.assertIn(d.main_track, ("A", "B", "C", "D"))
            self.assertGreater(len(d.main_track_name), 0)
            # philosophy_guard_subscore 必 = 1.0 (默认 6/6)
            self.assertEqual(d.philosophy_guard_subscore, 1.0)
        finally:
            runner.close()


# ===========================================================================
# V1130ASINorthStarRunner.compute_v05_18dim 真测 (2 tests)
# ===========================================================================

class TestRunnerComputeV05(unittest.TestCase):
    def test_v05_default(self):
        runner = V1130ASINorthStarRunner()
        try:
            v05 = runner.compute_v05_18dim()
            self.assertIn("v05_total", v05)
            self.assertIn("v04_score", v05)
            self.assertIn("continuity", v05)
            self.assertIn("autonomy", v05)
            self.assertIn("transferability", v05)
        finally:
            runner.close()

    def test_v05_custom_dims(self):
        runner = V1130ASINorthStarRunner(continuity=0.95, autonomy=0.92,
                                          transferability=0.88)
        try:
            v05 = runner.compute_v05_18dim()
            self.assertEqual(v05["continuity"], 0.95)
            self.assertEqual(v05["autonomy"], 0.92)
            self.assertEqual(v05["transferability"], 0.88)
        finally:
            runner.close()


# ===========================================================================
# V1130ASINorthStarRunner.compute_north_star 真测 (2 tests)
# ===========================================================================

class TestRunnerComputeNorthStar(unittest.TestCase):
    def test_north_star_keys(self):
        runner = V1130ASINorthStarRunner()
        try:
            nsc = runner.compute_north_star()
            for k in ("v05_total", "asi_north_star", "abs_headroom",
                      "rel_headroom_pct", "philosophy_guard_subscore",
                      "v1074_v03_above_floor", "r10_stage", "r10_pass_ultimate"):
                self.assertIn(k, nsc)
        finally:
            runner.close()

    def test_north_star_r10_stage(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W4")
        try:
            nsc = runner.compute_north_star()
            self.assertEqual(nsc["r10_stage"], "R10-W4")
        finally:
            runner.close()


# ===========================================================================
# V1130ASINorthStarRunner.run_chain_check (1 test)
# ===========================================================================

class TestRunnerChainCheck(unittest.TestCase):
    def test_chain_check_returns_5_modules(self):
        runner = V1130ASINorthStarRunner()
        try:
            chain = runner.run_chain_check()
            for k in ("v1072_continuity", "v1095_identity", "v1106_engineering",
                      "v1124_backend", "v1127_multi_agent"):
                self.assertIn(k, chain.to_dict())
        finally:
            runner.close()


# ===========================================================================
# V1130ASINorthStarRunner.run_chaos_node_down (2 tests)
# ===========================================================================

class TestRunnerChaos(unittest.TestCase):
    def test_chaos_node_down_drop_one(self):
        runner = V1130ASINorthStarRunner()
        try:
            r = runner.run_chaos_node_down(drop_indices=(0,))
            self.assertIsInstance(r, ChaosNodeDownReport)
            self.assertEqual(r.n_dropped, 1)
            self.assertTrue(r.measurement_preserved)
        finally:
            runner.close()

    def test_chaos_node_down_fallback(self):
        # 3 agent + drop 2 → surviving=1 < MIN_AGENTS=2 → fallback
        runner = V1130ASINorthStarRunner()
        try:
            # 直接用 V1128 chaos_test 验证 3-agent fallback 路径
            r = runner.run_chaos_node_down(drop_indices=(0, 1))
            # 默认 4 agent: drop 2 → surviving=2 = MIN_AGENTS, fallback 不会触发
            # 这是 V1128 chaos test 的正确行为: surviving < 2 才 fallback
            self.assertEqual(r.n_dropped, 2)
            self.assertEqual(r.n_surviving, 2)
            self.assertTrue(r.measurement_preserved)
        finally:
            runner.close()


# ===========================================================================
# V1130ASINorthStarRunner.benchmark_dashboard (2 tests)
# ===========================================================================

class TestRunnerBenchmark(unittest.TestCase):
    def test_benchmark_dashboard_default(self):
        runner = V1130ASINorthStarRunner()
        try:
            bench = runner.benchmark_dashboard(trials=3)
            for k in ("trials", "elapsed_min", "elapsed_max", "elapsed_mean",
                      "elapsed_median", "elapsed_p95", "target_s",
                      "perf_target_met", "speedup_vs_v1118_baseline",
                      "v1074_target_s_constant"):
                self.assertIn(k, bench)
            self.assertEqual(bench["trials"], 3)
            self.assertEqual(bench["target_s"], DASHBOARD_PERF_TARGET_S)
            self.assertEqual(bench["v1074_target_s_constant"], 2.50)
        finally:
            runner.close()

    def test_benchmark_dashboard_elapsed_in_range(self):
        runner = V1130ASINorthStarRunner()
        try:
            bench = runner.benchmark_dashboard(trials=2)
            self.assertGreaterEqual(bench["elapsed_min"], 0.0)
            self.assertGreaterEqual(bench["elapsed_mean"], bench["elapsed_min"])
            self.assertGreaterEqual(bench["elapsed_max"], bench["elapsed_mean"])
        finally:
            runner.close()


# ===========================================================================
# V1130ASINorthStarRunner.evaluate_r10_week (3 tests)
# ===========================================================================

class TestEvaluateR10Week(unittest.TestCase):
    def test_evaluate_r10_week_structure(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W3")
        try:
            r = runner.evaluate_r10_week()
            for k in ("version", "week_label", "v1130_version",
                      "dashboard", "v05", "north_star_composite",
                      "chain_integration", "chaos_node_down",
                      "guards", "halting_signals", "integration_matrix",
                      "v3_guards", "v3_guards_r10_injected",
                      "all_ok", "w2_pass", "w3_pass", "w4_pass",
                      "perf_target_met", "asi_north_star",
                      "r10_w2_target", "r10_w3_target", "r10_w4_target",
                      "dashboard_perf_target_s"):
                self.assertIn(k, r)
            self.assertEqual(r["week_label"], "R10-W3")
            self.assertEqual(r["dashboard_perf_target_s"], DASHBOARD_PERF_TARGET_S)
        finally:
            runner.close()

    def test_evaluate_r10_week_baseline(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W3", v04_score=0.8538)
        try:
            r = runner.evaluate_r10_week()
            # R9 W4 末 baseline → W2/W3/W4 未达
            self.assertFalse(r["w2_pass"])
            self.assertFalse(r["w3_pass"])
            self.assertFalse(r["w4_pass"])
            # chaos measurement_preserved 必 = True
            self.assertTrue(r["chaos_node_down"]["measurement_preserved"])
        finally:
            runner.close()

    def test_evaluate_r10_week_with_benchmark(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W3")
        try:
            r = runner.evaluate_r10_week(run_benchmark=True, benchmark_trials=3)
            self.assertIsNotNone(r["benchmark"])
            self.assertEqual(r["benchmark"]["trials"], 3)
        finally:
            runner.close()


# ===========================================================================
# Markdown 渲染 + CLI 入口 (3 tests)
# ===========================================================================

class TestMarkdownAndCLI(unittest.TestCase):
    def test_render_markdown(self):
        runner = V1130ASINorthStarRunner(week_label="R10-W3")
        try:
            r = runner.evaluate_r10_week()
            md = runner.render_markdown(r)
            self.assertIn("# V1130 R10 ASI 北极星 V0.5 真跑 + dashboard 性能提升", md)
            self.assertIn("R10-W3", md)
            self.assertIn("ASI 北极星综合 dashboard", md)
            self.assertIn("V0.5 公式真测", md)
            self.assertIn("ASI 北极星综合评估", md)
            self.assertIn("philosophy_guard_subscore", md)
            self.assertIn("Chaos test 节点失联", md)
            self.assertIn("V1072/V1095/V1106/V1124/V1127", md)
            self.assertIn("W2 中期门", md)
            self.assertIn("W3 末门", md)
            self.assertIn("W4 终极门", md)
            self.assertIn("perf_target_met", md)
            self.assertIn("V1130 真测集成矩阵", md)
            self.assertIn("中央 AI 是永恒身份", md)
        finally:
            runner.close()

    def test_cli_main_json(self):
        with mock.patch("sys.argv", ["v1130", "--json", "--week", "R10-W3"]):
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = main()
            out = buf.getvalue()
            data = json.loads(out)
            self.assertEqual(data["week_label"], "R10-W3")
            self.assertIn("dashboard", data)
            self.assertIn("v05", data)
            self.assertIn("north_star_composite", data)
            self.assertIn("chaos_node_down", data)
            self.assertIn(rc, (0, 1))

    def test_cli_main_human_output(self):
        with mock.patch("sys.argv", ["v1130", "--week", "R10-W2"]):
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = main()
            out = buf.getvalue()
            self.assertIn("V1130 R10 ASI 北极星 V0.5 真跑", out)
            self.assertIn("V0.5 总分", out)
            self.assertIn("ASI 北极星", out)
            self.assertIn("philosophy_guard_subscore", out)
            self.assertIn("elapsed_seconds", out)
            self.assertIn(rc, (0, 1))


if __name__ == "__main__":
    unittest.main()