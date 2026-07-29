"""V1114 Weekly Integration Evaluator — 真测 (R9 / R9-INT-003).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 20:55 红皇后守门.

测试覆盖 (≥15):
  - 4 选 1 主轨道切换规则 (4 tests): A/B/C/D 阈值 + halt 强制
  - 5 halting 信号 (5 tests): perf_regression / candidate_collapse / locked_in / red_queen / no_new_lift
  - ASI 北极星 dashboard (1 test)
  - V3 守门 6 项 (1 test)
  - Markdown 渲染 (1 test)
  - CLI 入口 (2 tests)
  - Constants / 自检 (3 tests)

运行: pytest -q tests/test_v1114_weekly_evaluator.py
"""
from __future__ import annotations

import json
import unittest
from unittest import mock

import apeireth.v1114_weekly_integration_evaluator as m
from apeireth.v1114_weekly_integration_evaluator import (
    ASI_NORTH_STAR,
    V1074_V03_MIN,
    V04_W4_TARGET,
    HaltingSignals,
    TrackDecision,
    choose_main_track,
    compute_dashboard,
    evaluate_halting_signals,
    render_markdown,
    check_halt_signal_1_perf_regression,
    check_halt_signal_2_candidate_collapse,
    check_halt_signal_3_locked_in,
    check_halt_signal_4_red_queen,
    check_halt_signal_5_no_new_lift,
    run_guard_self_check,
    evaluate_week,
)


# ---------------------------------------------------------------------------
# 常量与模块结构自检 (3 tests)
# ---------------------------------------------------------------------------

class TestV1114Constants(unittest.TestCase):
    def test_version_present(self):
        self.assertTrue(hasattr(m, "VERSION"))
        self.assertEqual(m.VERSION, "0.1.0")

    def test_thresholds_present(self):
        for const in ("ASI_NORTH_STAR", "V1074_V03_MIN", "V04_W4_TARGET",
                      "V04_TRACK_C_THRESHOLD", "V04_TRACK_D_THRESHOLD",
                      "V04_TRACK_B_THRESHOLD"):
            self.assertTrue(hasattr(m, const), f"missing {const}")
        # ASI 北极星 LOCKED 0.9800
        self.assertEqual(m.ASI_NORTH_STAR, 0.9800)
        # V1074 守门
        self.assertEqual(m.V1074_V03_MIN, 0.8884)
        # W4 终点目标
        self.assertEqual(m.V04_W4_TARGET, 0.85)

    def test_philosophy_and_v3_guards(self):
        # 主哲学 9 键 (3×3)
        self.assertEqual(len(m.PHILOSOPHY_9_KEYS), 9)
        # V3 守门 6 项
        self.assertEqual(len(m.V3_GUARDS), 6)
        # 4 选 1 主轨道
        self.assertEqual(set(m.TRACK_DEFS.keys()), {"A", "B", "C", "D"})


# ---------------------------------------------------------------------------
# 4 选 1 主轨道自动切换决策树 (5 tests)
# ---------------------------------------------------------------------------

class TestChooseMainTrack(unittest.TestCase):
    """覆盖 R9-ROADMAP-001 §7 + R9-INT-002 §5 的决策树."""

    def _no_halt(self) -> HaltingSignals:
        return HaltingSignals()  # 全 False

    def test_track_C_when_v04_above_threshold(self):
        """V0.4 ≥ 0.83 → Track C (跨小模型, 鲁棒性证明)."""
        t = choose_main_track(v04_score=0.83, halting=self._no_halt())
        self.assertEqual(t.track, "C")
        self.assertIn("跨小模型", t.track_name)
        self.assertIn("0.8300", t.rationale)
        self.assertFalse(t.halt_override)

    def test_track_D_when_v04_in_d_range(self):
        """0.82 ≤ V0.4 < 0.83 → Track D (DGM v0.4 双维 ROI 最高)."""
        t = choose_main_track(v04_score=0.8202, halting=self._no_halt())
        self.assertEqual(t.track, "D")
        self.assertIn("DGM", t.track_name)

    def test_track_B_when_v04_in_b_range(self):
        """0.80 ≤ V0.4 < 0.82 → Track B (HQB 4 维稳健补)."""
        t = choose_main_track(v04_score=0.81, halting=self._no_halt())
        self.assertEqual(t.track, "B")
        self.assertIn("HQB", t.track_name)

    def test_track_A_when_v04_below_threshold(self):
        """V0.4 < 0.80 → Track A (Rust hot path 救生圈)."""
        t = choose_main_track(v04_score=0.75, halting=self._no_halt())
        self.assertEqual(t.track, "A")
        self.assertIn("Rust", t.track_name)

    def test_halt_overrides_to_track_C(self):
        """任何 halt 信号触发 → 强制切 Track C (红皇后守门)."""
        halt = HaltingSignals(perf_regression=True)
        t = choose_main_track(v04_score=0.84, halting=halt)
        self.assertEqual(t.track, "C")
        self.assertTrue(t.halt_override)
        self.assertIn("HALT 触发", t.rationale)


# ---------------------------------------------------------------------------
# 5 Halting 信号 (5 tests)
# ---------------------------------------------------------------------------

class TestHaltingSignals(unittest.TestCase):
    def test_signal_1_perf_regression_triggered(self):
        """连续 3 轮 V0.3 下降 ≥ 0.005/轮 → 触发."""
        history = [0.8900, 0.8850, 0.8800, 0.8750]  # 连续 3 轮下降 ≥ 0.005
        self.assertTrue(check_halt_signal_1_perf_regression(history))

    def test_signal_1_perf_regression_not_triggered_when_recovering(self):
        """中间有反弹 → 不触发."""
        history = [0.8900, 0.8850, 0.8800, 0.8900]  # 最后一轮反弹
        self.assertFalse(check_halt_signal_1_perf_regression(history))

    def test_signal_2_candidate_collapse_triggered(self):
        """unique ratio < 0.5 → 触发."""
        self.assertTrue(check_halt_signal_2_candidate_collapse(0.3))
        self.assertFalse(check_halt_signal_2_candidate_collapse(0.7))
        # 边界
        self.assertFalse(check_halt_signal_2_candidate_collapse(0.5))

    def test_signal_3_locked_in_triggered(self):
        """fitness std < 0.01 + cross_dim_drop ≥ 0.10 → 触发."""
        self.assertTrue(check_halt_signal_3_locked_in(0.005, 0.15))
        self.assertFalse(check_halt_signal_3_locked_in(0.05, 0.05))
        self.assertFalse(check_halt_signal_3_locked_in(0.005, 0.05))  # cross_dim 不足

    def test_signal_4_red_queen_triggered(self):
        """V0.3 +0.001/轮 × 30 但 cross_model < 0.01 → 触发."""
        # 30 个 0.001 步长 = 总 +0.029 起步, 末尾追加确保 avg ≥ 0.001
        history = [0.8800 + i * 0.0015 for i in range(30)]
        history.append(0.8800 + 30 * 0.0015)  # = 0.9250
        self.assertTrue(check_halt_signal_4_red_queen(history, cross_model_lift=0.005))
        self.assertFalse(check_halt_signal_4_red_queen(history, cross_model_lift=0.02))

    def test_signal_5_no_new_lift_triggered(self):
        """累计 V0.3 lift < +0.02 (N=20) → 触发."""
        history = [0.8900 + i * 0.0005 for i in range(20)]
        history.append(0.8910)  # 总 lift = +0.01 < 0.02
        self.assertTrue(check_halt_signal_5_no_new_lift(history))
        # 充分 lift
        good_history = [0.8800 + i * 0.002 for i in range(21)]
        self.assertFalse(check_halt_signal_5_no_new_lift(good_history))

    def test_halting_signals_any_triggered(self):
        """HaltingSignals.any_triggered() 聚合."""
        all_false = HaltingSignals()
        self.assertFalse(all_false.any_triggered())
        self.assertEqual(all_false.triggered_list(), [])

        one_true = HaltingSignals(red_queen_trap=True)
        self.assertTrue(one_true.any_triggered())
        self.assertEqual(one_true.triggered_list(), ["4_red_queen"])


# ---------------------------------------------------------------------------
# ASI 北极星 dashboard (1 test)
# ---------------------------------------------------------------------------

class TestComputeDashboard(unittest.TestCase):
    def test_dashboard_prefers_v1077_over_v1103(self):
        """dashboard 优先用 V1077 V0.4 (W2 末已验证与 V1103 测法一致)."""
        v1074 = {"v03_score": 0.8890, "all_ok": True, "philosophy_guard_ok": True}
        v1077 = {"v04_score": 0.8202, "n_dims_filled": 16}
        v1103 = {"v04_score": 0.8188, "abs_headroom": 0.1612}
        d = compute_dashboard(v1074, v1077, v1103)
        self.assertEqual(d["v03_score"], 0.8890)
        self.assertEqual(d["v04_score"], 0.8202)  # V1077 优先
        self.assertEqual(d["v04_v1077"], 0.8202)
        self.assertEqual(d["v04_v1103"], 0.8188)
        self.assertEqual(d["asi_north_star"], 0.9800)
        self.assertAlmostEqual(d["abs_headroom"], 0.9800 - 0.8202, places=4)
        self.assertTrue(d["v1074_all_ok"])

    def test_dashboard_falls_back_to_v1103(self):
        """V1077 失败时降级到 V1103."""
        v1074 = {"v03_score": 0.8890, "all_ok": True, "philosophy_guard_ok": True}
        v1077 = {"v04_score": 0.0, "n_dims_filled": 0}  # 失败
        v1103 = {"v04_score": 0.8188, "abs_headroom": 0.1612}
        d = compute_dashboard(v1074, v1077, v1103)
        self.assertEqual(d["v04_score"], 0.8188)  # fallback


# ---------------------------------------------------------------------------
# V3 守门自检 (1 test)
# ---------------------------------------------------------------------------

class TestRunGuardSelfCheck(unittest.TestCase):
    def test_guards_all_pass_when_clean(self):
        """干净状态 → 6 项 V3 守门全过."""
        dashboard = {"v03_score": 0.8900, "v04_score": 0.8202, "v1074_all_ok": True, "philosophy_guard_ok": True}
        halting = HaltingSignals()
        g = run_guard_self_check(dashboard, halting)
        self.assertTrue(g["philosophy_9_keys_locked"])
        self.assertTrue(g["v3_guards_all_pass"])
        self.assertTrue(g["v1074_v03_above_floor"])
        self.assertFalse(g["halt_any_triggered"])
        self.assertEqual(len(g["v3_guards"]), 6)

    def test_guards_fail_when_v04_above_floor(self):
        """V1074 V0.3 < 0.8884 → 守门不通过."""
        dashboard = {"v03_score": 0.8800, "v04_score": 0.8202, "v1074_all_ok": False}
        halting = HaltingSignals()
        g = run_guard_self_check(dashboard, halting)
        self.assertFalse(g["v1074_v03_above_floor"])

    def test_guards_flag_red_queen_trap(self):
        """红皇后陷阱触发 → red_queen_is_not_asi = False."""
        dashboard = {"v03_score": 0.8900, "v04_score": 0.8202}
        halting = HaltingSignals(red_queen_trap=True)
        g = run_guard_self_check(dashboard, halting)
        self.assertFalse(g["v3_guards"]["red_queen_is_not_asi"])
        self.assertFalse(g["v3_guards_all_pass"])
        self.assertTrue(g["halt_any_triggered"])


# ---------------------------------------------------------------------------
# Markdown 渲染 (1 test)
# ---------------------------------------------------------------------------

class TestRenderMarkdown(unittest.TestCase):
    def test_markdown_contains_key_sections(self):
        report = {
            "week_label": "W3",
            "timestamp": 1700000000.0,
            "version": "0.1.0",
            "dashboard": {
                "v03_score": 0.8900,
                "v04_v1077": 0.8202,
                "v04_v1103": 0.8188,
                "v04_score": 0.8202,
                "asi_north_star": 0.9800,
                "abs_headroom": 0.1598,
                "rel_headroom_pct": 16.31,
                "n_dims_filled": 16,
                "v1074_all_ok": True,
                "philosophy_guard_ok": True,
            },
            "halting_signals": {k: False for k in ["perf_regression", "candidate_collapse", "locked_in_self_consistency", "red_queen_trap", "no_new_lift"]},
            "track_decision": {
                "track": "D",
                "track_name": "DGM v0.4 真演化",
                "rationale": "V0.4=0.8202 ∈ [0.82, 0.83) → 维持 Track D",
                "expected_lift": "+0.010~+0.030",
                "halt_override": False,
                "v1060_committed": True,
                "confidence": 0.85,
            },
            "guards": {
                "philosophy_9_keys_locked": True,
                "v3_guards_all_pass": True,
                "v1074_v03_above_floor": True,
                "v3_guards": {"runner_is_not_asi": True},
                "halt_any_triggered": False,
            },
            "all_ok": True,
            "v03_history": [0.8884, 0.8890, 0.8900],
        }
        md = render_markdown(report)
        # 关键章节
        for section in ["ASI 北极星 Dashboard", "4 选 1 主轨道决策", "5 Halting 信号", "守门自检"]:
            self.assertIn(section, md)
        # 关键数字
        self.assertIn("0.9800", md)  # 北极星
        self.assertIn("0.8202", md)  # V0.4
        self.assertIn("Track D", md)


# ---------------------------------------------------------------------------
# CLI 入口 (2 tests)
# ---------------------------------------------------------------------------

class TestCLIMain(unittest.TestCase):
    def test_cli_help(self):
        """--help 应正常输出."""
        from apeireth.v1114_weekly_integration_evaluator import main
        with self.assertRaises(SystemExit) as cm:
            main(["--help"])
        self.assertEqual(cm.exception.code, 0)

    def test_cli_with_v03_history(self):
        """--v03-history 应能解析逗号分隔浮点."""
        from apeireth.v1114_weekly_integration_evaluator import main
        # 用 --json 跑一次 (mock 三件套, 仅测试参数解析 + 决策树)
        with mock.patch.object(m, "run_v1074", return_value={"v03_score": 0.8900, "all_ok": True, "philosophy_guard_ok": True}), \
             mock.patch.object(m, "run_v1077", return_value={"v04_score": 0.8202, "n_dims_filled": 16}), \
             mock.patch.object(m, "run_v1103", return_value={"v04_score": 0.8188, "abs_headroom": 0.1612}):
            rc = main(["--week", "W3", "--json", "--v03-history", "0.8884,0.8890"])
        self.assertEqual(rc, 0)


# ---------------------------------------------------------------------------
# 完整 evaluate_week (1 test, 真跑守门不真跑三件套)
# ---------------------------------------------------------------------------

class TestEvaluateWeek(unittest.TestCase):
    def test_evaluate_week_full(self):
        """完整 evaluate_week 编排 (mock 三件套)."""
        with mock.patch.object(m, "run_v1074", return_value={"v03_score": 0.8900, "all_ok": True, "philosophy_guard_ok": True}), \
             mock.patch.object(m, "run_v1077", return_value={"v04_score": 0.8202, "n_dims_filled": 16}), \
             mock.patch.object(m, "run_v1103", return_value={"v04_score": 0.8188, "abs_headroom": 0.1612}):
            report = evaluate_week(
                week_label="W3",
                v03_history=[0.8884, 0.8892, 0.8900],
                unique_ratio=1.0,
                fitness_std=0.05,
                cross_dim_drop=0.0,
                cross_model_lift=0.02,
            )
        # dashboard
        self.assertEqual(report["dashboard"]["v03_score"], 0.8900)
        self.assertEqual(report["dashboard"]["v04_score"], 0.8202)
        # track: V0.4=0.8202 ∈ [0.82, 0.83) → Track D
        self.assertEqual(report["track_decision"]["track"], "D")
        # halting: 5 信号全 False
        self.assertFalse(report["halting_signals"]["perf_regression"])
        self.assertFalse(report["halting_signals"]["red_queen_trap"])
        # all_ok
        self.assertTrue(report["all_ok"])
        # version
        self.assertEqual(report["version"], "0.1.0")
        # 历史 append
        self.assertEqual(len(report["v03_history"]), 4)


if __name__ == "__main__":
    unittest.main()