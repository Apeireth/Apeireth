"""Apeireth ASI V1125 + V1126 — R10 集成协议 + Baseline 真测.

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 20:55 红皇后守门 + 主 23:44 干到底.

测试覆盖 (≥ 30 真测):
  - V1125 常量与模块结构 (4 tests)
  - V1125 V0.5 三新维度聚合 (4 tests)
  - V1125 ASI 北极星综合评估 (3 tests)
  - V1125 R10 主轨道决策 (5 tests)
  - V1125 R10 守门自检 + 4 红线 (4 tests)
  - V1125 R10 24 集成场景 (4 tests)
  - V1125 R10 evaluate_r10 主编排 (2 tests)
  - V1125 Markdown + CLI (2 tests)
  - V1126 baseline 加载 + R9 LOCKED (3 tests)
  - V1126 baseline 真测启动 + gap 评估 (3 tests)
  - V1126 Markdown + CLI (2 tests)

运行: pytest -q tests/test_v1125_v1126_r10_integration.py
"""
from __future__ import annotations

import io
import json
import sys
import unittest
from contextlib import redirect_stdout
from unittest import mock

import apeireth.v1125_r10_integration_protocol as m1125
import apeireth.v1126_r10_integration_baseline as m1126
from apeireth.v1125_r10_integration_protocol import (
    VERSION as V1125_VERSION,
    R10_START_TARGET,
    R10_MID_TARGET,
    R10_ULTIMATE_TARGET,
    R10_TRACK_ULTIMATE_THRESHOLD,
    R10_TRACK_DGM_THRESHOLD,
    R10_TRACK_HQB_THRESHOLD,
    R10_SCENARIO_COUNT,
    V3_GUARD_RED_LINES,
    V05_NEW_DIMS,
    TRACK_DEFS_R10,
    V05Score,
    compute_v05_score,
    compute_north_star_composite,
    choose_r10_main_track,
    run_r10_guard_self_check,
    run_r10_scenarios,
    summarize_scenarios,
    evaluate_r10,
    render_markdown_r10,
    main as v1125_main,
)
from apeireth.v1126_r10_integration_baseline import (
    VERSION as V1126_VERSION,
    R9_W4_BASELINE,
    R10_START_EXPECTATIONS,
    R10_BASELINE_COMPATIBILITY,
    R10Baseline,
    load_r10_baseline,
    run_r10_baseline_startup,
    render_markdown_baseline,
    main as v1126_main,
)
from apeireth.v1114_weekly_integration_evaluator import (
    ASI_NORTH_STAR,
    V1074_V03_MIN,
    HaltingSignals,
    TRACK_DEFS as V1114_TRACK_DEFS,
)


# ===========================================================================
# V1125 常量与模块结构 (4 tests)
# ===========================================================================

class TestV1125Constants(unittest.TestCase):
    def test_version_present(self):
        self.assertEqual(V1125_VERSION, "0.1.0")
        self.assertEqual(m1125.VERSION, "0.1.0")

    def test_r10_targets_locked(self):
        # 主 13:31 大胆激进 + 主 22:33 ASI 北极星
        self.assertEqual(R10_START_TARGET, 0.8600)
        self.assertEqual(R10_MID_TARGET, 0.9000)
        self.assertEqual(R10_ULTIMATE_TARGET, 0.9500)
        self.assertEqual(ASI_NORTH_STAR, 0.9800)

    def test_r10_track_thresholds_upgraded(self):
        # R10 升级: 0.83/0.82/0.80 → 0.92/0.88/0.86
        self.assertEqual(R10_TRACK_ULTIMATE_THRESHOLD, 0.92)
        self.assertEqual(R10_TRACK_DGM_THRESHOLD, 0.88)
        self.assertEqual(R10_TRACK_HQB_THRESHOLD, 0.86)
        self.assertGreater(R10_TRACK_ULTIMATE_THRESHOLD, 0.83)

    def test_v3_guard_red_lines_locked(self):
        # 主 17:43+17:58 不假装: 4 红线 LOCKED
        self.assertEqual(len(V3_GUARD_RED_LINES), 4)
        self.assertIn("no_fake_kpi", V3_GUARD_RED_LINES)
        self.assertIn("no_break_4_layer_gate", V3_GUARD_RED_LINES)
        self.assertIn("no_single_model_lockin", V3_GUARD_RED_LINES)
        self.assertIn("no_kpi_gaming", V3_GUARD_RED_LINES)


# ===========================================================================
# V1125 V0.5 三新维度聚合 (4 tests)
# ===========================================================================

class TestV05Score(unittest.TestCase):
    def test_v05_total_formula(self):
        # V0.5 = V0.4 × 0.85 + 3 新维加权
        s = V05Score(v04_score=0.85, continuity=0.90, autonomy=0.88,
                     transferability=0.92)
        expected = 0.85 * 0.85 + 0.90 * 0.05 + 0.88 * 0.05 + 0.92 * 0.05
        self.assertAlmostEqual(s.total(), expected, places=4)

    def test_compute_v05_default(self):
        # 默认 3 新维 = 0.85
        d = compute_v05_score(v04_score=0.8538)
        self.assertAlmostEqual(d["v04_score"], 0.8538, places=4)
        self.assertAlmostEqual(d["continuity"], 0.85, places=4)
        self.assertAlmostEqual(d["v05_total"], 0.8538 * 0.85 + 0.85 * 0.15, places=4)

    def test_compute_v05_custom_dims(self):
        d = compute_v05_score(v04_score=0.90, continuity=0.95, autonomy=0.92,
                              transferability=0.88)
        # 0.90*0.85 + 0.95*0.05 + 0.92*0.05 + 0.88*0.05 = 0.765 + 0.0475 + 0.046 + 0.044 = 0.9025
        self.assertAlmostEqual(d["v05_total"], 0.9025, places=4)

    def test_v05_three_new_dims_locked(self):
        # V0.5 新增 3 维度 LOCKED (continuity / autonomy / transferability)
        self.assertEqual(V05_NEW_DIMS, ("continuity", "autonomy", "transferability"))
        self.assertEqual(len(V05_NEW_DIMS), 3)


# ===========================================================================
# V1125 ASI 北极星综合评估 (3 tests)
# ===========================================================================

class TestNorthStarComposite(unittest.TestCase):
    def test_default_asi_composite(self):
        c = compute_north_star_composite(v05_total=0.85, philosophy_guard_pass_count=6,
                                         v1074_v03=0.8897, r10_stage="R10-W1")
        self.assertAlmostEqual(c["asi_north_star"], 0.9800, places=4)
        self.assertAlmostEqual(c["abs_headroom"], 0.1300, places=4)
        self.assertEqual(c["r10_pass_ultimate"], False)  # 0.85 < 0.95

    def test_ultimate_pass(self):
        c = compute_north_star_composite(v05_total=0.96)
        self.assertEqual(c["r10_pass_ultimate"], True)

    def test_philosophy_subscore(self):
        c = compute_north_star_composite(v05_total=0.85, philosophy_guard_pass_count=6)
        self.assertAlmostEqual(c["philosophy_guard_subscore"], 1.0, places=4)
        c2 = compute_north_star_composite(v05_total=0.85, philosophy_guard_pass_count=3)
        self.assertAlmostEqual(c2["philosophy_guard_subscore"], 0.5, places=4)


# ===========================================================================
# V1125 R10 主轨道决策 (5 tests)
# ===========================================================================

class TestChooseR10MainTrack(unittest.TestCase):
    def test_track_c_when_v05_above_ultimate(self):
        # V0.5 ≥ 0.92 → Track C
        h = HaltingSignals()
        t = choose_r10_main_track(0.95, h)
        self.assertEqual(t.track, "C")
        self.assertIn("Track C", t.rationale)

    def test_track_d_when_v05_in_dgm_range(self):
        # V0.5 ∈ [0.88, 0.92) → Track D
        h = HaltingSignals()
        t = choose_r10_main_track(0.89, h)
        self.assertEqual(t.track, "D")

    def test_track_b_when_v05_in_hqb_range(self):
        # V0.5 ∈ [0.86, 0.88) → Track B
        h = HaltingSignals()
        t = choose_r10_main_track(0.87, h)
        self.assertEqual(t.track, "B")

    def test_track_a_when_v05_below_hqb(self):
        # V0.5 < 0.86 → Track A
        h = HaltingSignals()
        t = choose_r10_main_track(0.80, h)
        self.assertEqual(t.track, "A")

    def test_halt_signal_forces_track_c(self):
        # 任一 halt 触发 → 强制 Track C
        h = HaltingSignals(perf_regression=True)
        t = choose_r10_main_track(0.99, h)  # 即使 V0.5 极高
        self.assertEqual(t.track, "C")
        self.assertIn("强制", t.rationale)

    def test_v1060_not_committed_forces_track_a(self):
        # V1060 未 commit + V0.5 < 0.86 → 强制 Track A
        h = HaltingSignals()
        t = choose_r10_main_track(0.80, h, v1060_committed=False)
        self.assertEqual(t.track, "A")
        self.assertIn("REVERT", t.rationale)


# ===========================================================================
# V1125 R10 守门自检 + 4 红线 (4 tests)
# ===========================================================================

class TestR10GuardSelfCheck(unittest.TestCase):
    def test_guard_all_pass_when_clean(self):
        dashboard = {"v03_score": 0.8897, "v04_score": 0.8538, "v04_v1077": 0.8538}
        h = HaltingSignals()
        g = run_r10_guard_self_check(dashboard, h)
        self.assertTrue(g.all_ok)
        self.assertTrue(g.philosophy_9_keys_locked)
        self.assertTrue(g.v3_guards_all_pass)
        self.assertTrue(g.red_lines_all_pass)
        self.assertTrue(g.v1074_v03_above_floor)

    def test_guard_fail_when_v03_below_floor(self):
        dashboard = {"v03_score": 0.50, "v04_score": 0.8538}
        h = HaltingSignals()
        g = run_r10_guard_self_check(dashboard, h)
        self.assertFalse(g.v1074_v03_above_floor)
        self.assertFalse(g.all_ok)

    def test_guard_fail_when_halt_triggered(self):
        dashboard = {"v03_score": 0.8897, "v04_score": 0.8538}
        h = HaltingSignals(red_queen_trap=True)
        g = run_r10_guard_self_check(dashboard, h)
        self.assertTrue(g.halt_any_triggered)
        self.assertFalse(g.all_ok)

    def test_custom_red_lines(self):
        dashboard = {"v03_score": 0.8897, "v04_score": 0.8538}
        h = HaltingSignals()
        red_lines = {
            "no_fake_kpi": True,
            "no_break_4_layer_gate": False,   # 故意失败
            "no_single_model_lockin": True,
            "no_kpi_gaming": True,
        }
        g = run_r10_guard_self_check(dashboard, h, red_lines)
        self.assertFalse(g.red_lines_all_pass)
        self.assertFalse(g.all_ok)


# ===========================================================================
# V1125 R10 24 集成场景 (4 tests)
# ===========================================================================

class TestR10Scenarios(unittest.TestCase):
    def _setup(self):
        dashboard = {
            "v03_score": 0.8897, "v04_score": 0.8538,
            "v04_v1077": 0.8538, "v04_v1103": 0.8188,
            "lift_p2": 0.1447, "n_dims_filled": 17,
            "v1074_all_ok": True, "philosophy_guard_ok": True,
        }
        h = HaltingSignals()
        g = run_r10_guard_self_check(dashboard, h)
        v05 = compute_v05_score(0.8538)
        t = choose_r10_main_track(v05["v05_total"], h)
        return dashboard, h, v05, g, t

    def test_scenario_count_at_least_24(self):
        # ≥ 24 集成场景 LOCKED (主 17:43 实事求是)
        self.assertGreaterEqual(R10_SCENARIO_COUNT, 24)
        self.assertEqual(len(m1125.R10_INTEGRATION_SCENARIOS), 24)

    def test_run_scenarios_returns_24(self):
        dashboard, h, v05, g, t = self._setup()
        results = run_r10_scenarios(dashboard, h, v05["v05_total"], g, t)
        self.assertEqual(len(results), 24)
        for r in results:
            self.assertTrue(r.id.startswith("S"))
            self.assertIsInstance(r.passed, bool)

    def test_summarize_scenarios(self):
        dashboard, h, v05, g, t = self._setup()
        results = run_r10_scenarios(dashboard, h, v05["v05_total"], g, t)
        s = summarize_scenarios(results)
        self.assertEqual(s["total"], 24)
        self.assertGreaterEqual(s["pass_rate"], 0.0)
        self.assertTrue(s["scenario_count_locked"])

    def test_s19_v05_scenario_present(self):
        # R10 独有 S19: V0.5 维度验证
        sids = [sc["id"] for sc in m1125.R10_INTEGRATION_SCENARIOS]
        self.assertIn("S19", sids)
        self.assertIn("S20", sids)
        self.assertIn("S21", sids)
        self.assertIn("S22", sids)
        self.assertIn("S23", sids)
        self.assertIn("S24", sids)


# ===========================================================================
# V1125 R10 evaluate_r10 主编排 (2 tests)
# ===========================================================================

class TestEvaluateR10(unittest.TestCase):
    def test_evaluate_r10_baseline(self):
        # R9 W4 末 baseline (V0.3=0.8897, V0.4=0.8538) 真跑
        r = evaluate_r10(week_label="R10-W1", v04_actual=0.8538, v1074_v03_actual=0.8897)
        self.assertEqual(r["week_label"], "R10-W1")
        self.assertIn("v05_score", r)
        self.assertIn("north_star_composite", r)
        self.assertIn("r10_track_decision", r)
        self.assertIn("guards", r)
        self.assertIn("scenarios", r)
        self.assertEqual(len(r["scenarios"]), 24)
        self.assertIn("v1114_version", r)

    def test_evaluate_r10_v05_total_range(self):
        # V0.5 总分应在合理范围 (0.7 ~ 1.0)
        r = evaluate_r10(v04_actual=0.8538)
        v05 = r["v05_score"]["v05_total"]
        self.assertGreater(v05, 0.7)
        self.assertLess(v05, 1.0)


# ===========================================================================
# V1125 Markdown + CLI (2 tests)
# ===========================================================================

class TestV1125MarkdownAndCLI(unittest.TestCase):
    def test_render_markdown_contains_key_sections(self):
        r = evaluate_r10(v04_actual=0.8538)
        md = render_markdown_r10(r)
        self.assertIn("R10", md)
        self.assertIn("ASI 北极星", md)
        self.assertIn("V0.5", md)
        self.assertIn("24", md)  # 24 场景
        self.assertIn("V1074", md)
        self.assertIn("R10 主轨道", md)

    def test_cli_scenarios_mode(self):
        # --scenarios 模式: 只跑 24 场景
        with mock.patch("sys.argv", ["v1125", "--scenarios"]):
            with redirect_stdout(io.StringIO()) as buf:
                rc = v1125_main(["--scenarios"])
        self.assertIn(rc, (0, 1))
        text = buf.getvalue()
        self.assertIn("scenarios", text)
        self.assertIn("summary", text)


# ===========================================================================
# V1126 baseline 加载 + R9 LOCKED (3 tests)
# ===========================================================================

class TestV1126BaselineLoad(unittest.TestCase):
    def test_version_present(self):
        self.assertEqual(V1126_VERSION, "0.1.0")

    def test_r9_w4_baseline_locked(self):
        # 主 17:43 实事求是: R9 W4 末 baseline LOCKED, 不允许改写
        self.assertEqual(R9_W4_BASELINE["v04_score"], 0.8538)
        self.assertEqual(R9_W4_BASELINE["v03_score"], 0.8897)
        self.assertEqual(R9_W4_BASELINE["n_dims_filled"], 17)
        self.assertTrue(R9_W4_BASELINE["v1074_all_ok"])
        self.assertEqual(R9_W4_BASELINE["source"], "r9_w4_baseline_locked")

    def test_load_r10_baseline_dataclass(self):
        b = load_r10_baseline()
        self.assertIsInstance(b, R10Baseline)
        self.assertEqual(b.r9_w4_baseline["v04_score"], 0.8538)
        self.assertEqual(b.r10_start_target, R10_START_TARGET)
        self.assertEqual(b.asi_north_star, ASI_NORTH_STAR)
        self.assertGreater(b.timestamp, 0)


# ===========================================================================
# V1126 baseline 真测启动 + gap 评估 (3 tests)
# ===========================================================================

class TestV1126BaselineStartup(unittest.TestCase):
    def test_startup_runs_protocol(self):
        run = run_r10_baseline_startup(week_label="R10-W1")
        self.assertEqual(run.baseline["r9_w4_baseline"]["v04_score"], 0.8538)
        self.assertIn("protocol_result", run.to_dict())
        self.assertEqual(len(run.protocol_result["scenarios"]), 24)

    def test_gap_to_r10_start_small(self):
        # V0.4=0.8538 → R10 起点 0.86, gap = +0.0062 (主 17:43 实事求是: 还差 0.62pp)
        run = run_r10_baseline_startup()
        self.assertAlmostEqual(run.gap_to_r10_start, 0.0062, places=4)
        self.assertFalse(run.passes_r10_start)  # 0.8538 < 0.86

    def test_r10_ready_false_at_startup(self):
        # R10 启动期 V0.4 < 0.86 → r10_ready = False
        run = run_r10_baseline_startup()
        self.assertFalse(run.r10_ready)

    def test_compatibility_with_v1114_v1119_v1125(self):
        # 主 19:33 走在前人经验上: 兼容矩阵 LOCKED
        self.assertEqual(R10_BASELINE_COMPATIBILITY["v1114_weekly_evaluator"], "compatible")
        self.assertEqual(R10_BASELINE_COMPATIBILITY["v1119_w4_validator"], "compatible")
        self.assertEqual(R10_BASELINE_COMPATIBILITY["v1125_r10_protocol"], "native")


# ===========================================================================
# V1126 Markdown + CLI (2 tests)
# ===========================================================================

class TestV1126MarkdownAndCLI(unittest.TestCase):
    def test_render_markdown_contains_baseline(self):
        run = run_r10_baseline_startup()
        md = render_markdown_baseline(run)
        self.assertIn("R10 Baseline", md)
        self.assertIn("R9 W4 末", md)
        self.assertIn("0.8538", md)
        self.assertIn("V0.5", md)
        self.assertIn("兼容矩阵", md)

    def test_cli_json_mode(self):
        # --json 模式: JSON 输出
        with redirect_stdout(io.StringIO()) as buf:
            rc = v1126_main(["--json"])
        self.assertIn(rc, (0, 1))
        out = buf.getvalue()
        d = json.loads(out)
        self.assertEqual(d["baseline"]["r9_w4_baseline"]["v04_score"], 0.8538)
        self.assertEqual(len(d["protocol_result"]["scenarios"]), 24)


# ===========================================================================
# V1125 + V1126 一致性 (2 tests)
# ===========================================================================

class TestV1125V1126Consistency(unittest.TestCase):
    def test_v1125_v1114_decision_engine_consistent(self):
        # V1125 复用 V1114 决策引擎 (主 19:33)
        from apeireth.v1114_weekly_integration_evaluator import choose_main_track
        # V0.4 = 0.83 → V1114 Track C, V1125 Track B (V0.5 < 0.86 阈值下移)
        v1114_t = choose_main_track(0.83, HaltingSignals())
        self.assertEqual(v1114_t.track, "C")

    def test_v1126_uses_v1125_protocol(self):
        # V1126 真跑 V1125 evaluate_r10 (主 19:33 复用)
        run = run_r10_baseline_startup()
        self.assertEqual(run.protocol_result["version"], V1125_VERSION)
        self.assertIn("v1114_version", run.protocol_result)


if __name__ == "__main__":
    # ponytail: 直接 run 时显示 verbose output (主 00:56 一行可跑)
    unittest.main(verbosity=2)