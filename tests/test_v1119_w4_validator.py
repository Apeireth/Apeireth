"""Apeireth ASI V1119 — W4 集成验证工具 + R10 移交 checklist 测试.

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 20:55 红皇后守门 + 主 23:44 干到底.

测试覆盖 (≥20):
  - 常量与模块结构 (3 tests)
  - R9ComponentStatus + HandoffCheck dataclass (3 tests)
  - R10 差距评估 compute_r10_gap (4 tests)
  - 移交 checklist 自动生成 (5 tests)
  - R10 路径建议 compute_r10_path_recommendation (4 tests)
  - W4 真跑 evaluate_w4 (5 tests)
  - Markdown 渲染 render_markdown_w4 (2 tests)
  - CLI 入口 main (3 tests)

运行: pytest -q tests/test_v1119_w4_validator.py
"""
from __future__ import annotations

import io
import json
import sys
import unittest
from contextlib import redirect_stdout
from unittest import mock

import apeireth.v1119_w4_integration_validator as m
from apeireth.v1119_w4_integration_validator import (
    VERSION,
    W4_TARGET,
    R10_START_TARGET,
    R10_MID_TARGET,
    ASI_NORTH,
    R9ComponentStatus,
    HandoffCheck,
    W4Evaluation,
    evaluate_w4,
    compute_r10_gap,
    compute_handoff_checklist,
    compute_r10_path_recommendation,
    render_markdown_w4,
    main,
)
# 显式从 V1114 拿 (避免 namespace 冲突)
from apeireth.v1114_weekly_integration_evaluator import (
    V1074_V03_MIN,
    HaltingSignals,
    TrackDecision,
    compute_dashboard as v1114_compute_dashboard,
    choose_main_track as v1114_choose_main_track,
)


# ---------------------------------------------------------------------------
# 常量与模块结构 (3 tests)
# ---------------------------------------------------------------------------

class TestV1119Constants(unittest.TestCase):
    def test_version_present(self):
        self.assertEqual(m.VERSION, "0.1.0")

    def test_targets_and_asi(self):
        # W4 / R10 / ASI LOCKED (主 22:33 + 主 13:31)
        self.assertEqual(W4_TARGET, 0.85)
        self.assertEqual(R10_START_TARGET, 0.86)
        self.assertEqual(R10_MID_TARGET, 0.90)
        self.assertEqual(ASI_NORTH, 0.9800)

    def test_v3_guards_w4_injected(self):
        # V1119 auto-injected V3 守门 (继承 V1101 + W4-specific)
        self.assertIn("w4_evaluator_is_not_asi", m.V3_GUARDS_W4_INJECTED)
        self.assertIn("handoff_checklist_is_not_receipt", m.V3_GUARDS_W4_INJECTED)
        self.assertIn("r10_path_is_not_guarantee", m.V3_GUARDS_W4_INJECTED)
        self.assertGreaterEqual(len(m.V3_GUARDS_W4_INJECTED), 4)


# ---------------------------------------------------------------------------
# R9ComponentStatus + HandoffCheck dataclass (3 tests)
# ---------------------------------------------------------------------------

class TestR9ComponentStatus(unittest.TestCase):
    def test_default_is_w3_baseline(self):
        s = R9ComponentStatus()
        # W3 末: V1060 已 commit, 其余 W4 在做
        self.assertTrue(s.v1060_committed)
        self.assertFalse(s.v1061_cognitive_core_done)
        self.assertFalse(s.v1062_world_model_done)
        self.assertFalse(s.v1093_dgm_v04_500loc)
        self.assertEqual(s.interface_freeze_count, 1)
        self.assertEqual(s.interface_freeze_target, 5)
        self.assertLess(s.test_coverage_pct, 0.30)

    def test_custom_status(self):
        s = R9ComponentStatus(
            v1061_cognitive_core_done=True,
            v1093_dgm_v04_500loc=True,
            interface_freeze_count=5,
            test_coverage_pct=0.45,
        )
        self.assertTrue(s.v1061_cognitive_core_done)
        self.assertTrue(s.v1093_dgm_v04_500loc)
        self.assertEqual(s.interface_freeze_count, 5)
        self.assertAlmostEqual(s.test_coverage_pct, 0.45)

    def test_to_dict(self):
        s = R9ComponentStatus()
        d = s.to_dict()
        self.assertIn("v1060_committed", d)
        self.assertIn("test_coverage_pct", d)
        self.assertEqual(len(d), 10)


class TestHandoffCheck(unittest.TestCase):
    def test_dataclass_creation(self):
        c = HandoffCheck(
            id="test_id", title="测试项", status=True,
            actual=0.95, threshold=0.85, note="主 17:43 test", section="metric",
        )
        self.assertEqual(c.id, "test_id")
        self.assertEqual(c.status, True)
        self.assertEqual(c.section, "metric")

    def test_to_dict_round_trip(self):
        c = HandoffCheck(
            id="x", title="y", status=False,
            actual=0.50, threshold=0.85, note="fail", section="guard",
        )
        d = c.to_dict()
        self.assertEqual(d["id"], "x")
        self.assertEqual(d["status"], False)
        self.assertEqual(d["section"], "guard")
        self.assertEqual(d["note"], "fail")


# ---------------------------------------------------------------------------
# R10 差距评估 compute_r10_gap (4 tests)
# ---------------------------------------------------------------------------

class TestComputeR10Gap(unittest.TestCase):
    def test_clean_passes_w4(self):
        # V0.4 = 0.86 ≥ W4_TARGET 0.85 → passes_w4 + passes_r10_start; gap 负 (过线)
        d = {"v03_score": 0.89, "v04_score": 0.86}
        gap = compute_r10_gap(d)
        self.assertTrue(gap["passes_w4"])
        self.assertTrue(gap["passes_r10_start"])
        # 0.85 - 0.86 = -0.01 (过线, 负差距)
        self.assertAlmostEqual(gap["gap_to_w4"], -0.01, places=4)
        self.assertEqual(gap["w4_target"], 0.85)

    def test_below_w4_target(self):
        # V0.4 = 0.82 < 0.85 → not_pass
        d = {"v03_score": 0.89, "v04_score": 0.82}
        gap = compute_r10_gap(d)
        self.assertFalse(gap["passes_w4"])
        self.assertFalse(gap["passes_r10_start"])
        self.assertAlmostEqual(gap["gap_to_w4"], 0.03, places=2)
        self.assertAlmostEqual(gap["gap_to_w4"], 0.85 - 0.82, places=4)

    def test_asi_headroom_calc(self):
        d = {"v03_score": 0.89, "v04_score": 0.50}
        gap = compute_r10_gap(d)
        # headroom_rel_pct = (0.9800 - 0.50) / 0.9800 * 100
        expected = round((0.98 - 0.50) / 0.98 * 100, 2)
        self.assertAlmostEqual(gap["headroom_rel_pct"], expected, places=2)
        # gap_to_asi should be 0.4800
        self.assertAlmostEqual(gap["gap_to_asi"], 0.48, places=4)

    def test_all_targets_evaluated(self):
        d = {"v03_score": 0.89, "v04_score": 0.80}
        gap = compute_r10_gap(d)
        # should have all 4 gap keys
        for k in ("gap_to_w4", "gap_to_r10_start", "gap_to_r10_mid", "gap_to_asi"):
            self.assertIn(k, gap)
        # V0.4 = 0.80 < 0.85, < 0.86, < 0.90, < 0.98 → gaps all positive
        self.assertEqual(gap["passes_w4"], False)
        self.assertEqual(gap["passes_r10_mid"], False)


# ---------------------------------------------------------------------------
# 移交 checklist 自动生成 (5 tests)
# ---------------------------------------------------------------------------

class TestHandoffChecklistGen(unittest.TestCase):
    def _setup_baseline(self) -> tuple:
        dashboard = {"v03_score": 0.8897, "v04_score": 0.8202, "v04_v1077": 0.8202, "v04_v1103": 0.8188}
        halting = HaltingSignals()
        guards = {
            "v3_guards_all_pass": True,
            "philosophy_9_keys_locked": True,
        }
        component = R9ComponentStatus()
        track = TrackDecision(
            track="D", track_name="DGM v0.4 真演化",
            rationale="V0.4=0.8202 ∈ [0.82, 0.83) → 维持 Track D",
            expected_lift="+0.010~+0.030", halt_override=False,
            v1060_committed=True, confidence=0.85,
        )
        return dashboard, halting, guards, component, track

    def _w3_baseline_mock(self):
        """返回 V1114 兼容 key 的 W3 末 baseline (fetch_three_pieces 风格)."""
        return (
            {"module": "V1074", "v03_score": 0.8897, "all_ok": True, "philosophy_guard_ok": True,
             "rc": 0, "elapsed_ms": 0, "source": "test_mock"},
            {"module": "V1077", "v04_score": 0.8202, "n_dims_filled": 16, "rc": 0,
             "elapsed_ms": 0, "source": "test_mock"},
            {"module": "V1103", "v04_score": 0.8188, "top_n": 5, "lift_p2": 0.1447, "rc": 0,
             "elapsed_ms": 0, "source": "test_mock"},
            False,  # used_live
        )

    def test_default_state_has_at_least_12_checks(self):
        d, h, g, c, t = self._setup_baseline()
        checks = compute_handoff_checklist(d, h, g, t, c)
        self.assertGreaterEqual(len(checks), 12,
            f"移交 checklist must have ≥12 items, got {len(checks)}")

    def test_checklist_has_all_required_categories(self):
        d, h, g, c, t = self._setup_baseline()
        checks = compute_handoff_checklist(d, h, g, t, c)
        sections = {x.section for x in checks}
        self.assertIn("metric", sections)
        self.assertIn("guard", sections)
        self.assertIn("component", sections)
        self.assertIn("meta", sections)

    def test_checklist_default_w3_baseline_mostly_fail(self):
        # W3 末 baseline: V1061/V1062/V1093 等都 False, 应该清单上很多 fail
        d, h, g, c, t = self._setup_baseline()
        checks = compute_handoff_checklist(d, h, g, t, c)
        n_pass = sum(1 for x in checks if x.status)
        n_total = len(checks)
        # 主要 fails
        self.assertLess(n_pass, n_total)
        # 但 V1074 V0.3 守门 + V3 守门 + philosophy 应 pass
        pass_ids = {x.id for x in checks if x.status}
        self.assertIn("v1074_v03_floor", pass_ids)
        self.assertIn("v3_guards_all_pass", pass_ids)
        self.assertIn("philosophy_9_keys_locked", pass_ids)
        self.assertIn("asi_north_star_locked", pass_ids)
        self.assertIn("v1060_committed", pass_ids)
        # V1077/V1103 W4 末目标未达 → fail
        self.assertIn("v1077_v04_w4_target", {x.id for x in checks if not x.status})
        self.assertIn("v1103_v04_w4_target", {x.id for x in checks if not x.status})

    def test_checklist_passes_when_all_components_done(self):
        # 全部完成: handoff_ready 路径
        d, h, g, c, t = self._setup_baseline()
        d["v04_score"] = 0.87  # 强制提升
        d["v04_v1077"] = 0.87
        d["v04_v1103"] = 0.87
        c.v1061_cognitive_core_done = True
        c.v1062_world_model_done = True
        c.v1093_dgm_v04_500loc = True
        c.v1078_rl_done = True
        c.v1097_mcp_round2_done = True
        c.interface_freeze_count = 5
        c.test_coverage_pct = 0.40
        checks = compute_handoff_checklist(d, h, g, t, c)
        n_pass = sum(1 for x in checks if x.status)
        n_total = len(checks)
        # 至少 12/14 通过
        self.assertGreaterEqual(n_pass, max(12, int(n_total * 0.80)))

    def test_halt_signal_flip_in_checklist(self):
        # 触发 perf_regression → checklist.no_halting_signals 应 FAIL
        d, h, g, c, t = self._setup_baseline()
        h.perf_regression = True
        checks = compute_handoff_checklist(d, h, g, t, c)
        no_halt = next(x for x in checks if x.id == "no_halting_signals")
        self.assertFalse(no_halt.status)
        self.assertIn("perf_regression", str(no_halt.actual))


# ---------------------------------------------------------------------------
# R10 路径建议 (4 tests)
# ---------------------------------------------------------------------------

class TestR10PathRecommendation(unittest.TestCase):
    def _mk_checks(self, n_pass: int = 0, n_total: int = 14) -> list:
        return [
            HandoffCheck(id=f"c{i}", title="t", status=(i < n_pass),
                         actual=0.0, threshold=0.85, section="metric")
            for i in range(n_total)
        ]

    def test_paths_contains_p0_when_v04_below(self):
        d = {"v03_score": 0.89, "v04_score": 0.82}
        gap = compute_r10_gap(d)
        checks = self._mk_checks(0)
        c = R9ComponentStatus()
        t = TrackDecision(track="D", track_name="T", rationale="r", expected_lift="+0.01", halt_override=False, v1060_committed=True)
        paths = compute_r10_path_recommendation(gap, checks, c, t)
        # 主 13:31 大胆激进: 必须含 [P0] 补 V0.4 缺口
        self.assertTrue(any("[P0]" in p and "补 V0.4 缺口" in p for p in paths),
            "should contain P0 V0.4 gap path")

    def test_paths_recommends_r10_start_when_passes(self):
        d = {"v03_score": 0.89, "v04_score": 0.87}
        gap = compute_r10_gap(d)
        checks = self._mk_checks(0)
        c = R9ComponentStatus()
        t = TrackDecision(track="C", track_name="T", rationale="r", expected_lift="+0.01", halt_override=False, v1060_committed=True)
        paths = compute_r10_path_recommendation(gap, checks, c, t)
        # V0.4 ≥ 0.86 → "R10 起点已达" / "直接启动 R10 P0"
        text = " ".join(paths)
        self.assertTrue("R10 起点已达" in text or "直接启动 R10" in text)

    def test_paths_includes_component_recommendations(self):
        d = {"v03_score": 0.89, "v04_score": 0.80}
        gap = compute_r10_gap(d)
        checks = self._mk_checks(0)
        c = R9ComponentStatus()  # all False except V1060
        t = TrackDecision(track="B", track_name="T", rationale="r", expected_lift="+0.01", halt_override=False, v1060_committed=True)
        paths = compute_r10_path_recommendation(gap, checks, c, t)
        text = " ".join(paths)
        # V1061/V1062/V1093/V1078 done 都 False → 全部应出现
        self.assertIn("V1061", text)
        self.assertIn("V1062", text)
        self.assertIn("V1093", text)
        self.assertIn("接口冻结", text)
        self.assertIn("测试覆盖", text)

    def test_paths_count_at_least_3(self):
        d = {"v03_score": 0.89, "v04_score": 0.80}
        gap = compute_r10_gap(d)
        checks = self._mk_checks(0)
        c = R9ComponentStatus()
        t = TrackDecision(track="A", track_name="T", rationale="r", expected_lift="+0.01", halt_override=False, v1060_committed=True)
        paths = compute_r10_path_recommendation(gap, checks, c, t)
        self.assertGreaterEqual(len(paths), 3)


# ---------------------------------------------------------------------------
# W4 真跑 evaluate_w4 (5 tests)
# ---------------------------------------------------------------------------

class TestEvaluateW4(unittest.TestCase):
    def _w3_baseline_mock(self):
        """返回 V1114 兼容 key 的 W3 末 baseline (fetch_three_pieces 风格)."""
        return (
            {"module": "V1074", "v03_score": 0.8897, "all_ok": True, "philosophy_guard_ok": True,
             "rc": 0, "elapsed_ms": 0, "source": "test_mock"},
            {"module": "V1077", "v04_score": 0.8202, "n_dims_filled": 16, "rc": 0,
             "elapsed_ms": 0, "source": "test_mock"},
            {"module": "V1103", "v04_score": 0.8188, "top_n": 5, "lift_p2": 0.1447, "rc": 0,
             "elapsed_ms": 0, "source": "test_mock"},
            False,  # used_live
        )

    def test_default_w3_baseline_runs_without_live(self):
        # 默认 fallback → 不调 subprocess
        with mock.patch.object(m, "fetch_three_pieces") as fetch:
            fetch.return_value = self._w3_baseline_mock()
            r = evaluate_w4(week_label="W4")
            self.assertEqual(r.week_label, "W4")
            self.assertEqual(r.version, VERSION)
            self.assertGreaterEqual(r.n_checks_total, 12)
            # W3 末 baseline: all_ok 应 False (V0.4 = 0.8202 < 0.85)
            self.assertFalse(r.all_ok)
            self.assertFalse(r.handoff_ready)
            # 但 dashboard + dashboard[handoff_checklist] 都有数据
            self.assertIn("v03_score", r.dashboard)
            self.assertGreater(len(r.handoff_checklist), 0)
            # r10_gap 距 W4 末应有正缺口
            self.assertEqual(r.r10_gap["passes_w4"], False)
            # verify dashboard.v04_score 正确读 baseline 0.8202
            self.assertAlmostEqual(r.dashboard["v04_score"], 0.8202, places=4)

    def test_evaluate_returns_typed_dataclass(self):
        r = evaluate_w4(week_label="W4", live=False)
        self.assertIsInstance(r, W4Evaluation)
        d = r.to_dict()
        # keys 完整
        for k in ("week_label", "dashboard", "halting_signals", "track_decision",
                  "guards", "r10_gap", "handoff_checklist", "r10_path_recommendation",
                  "all_ok", "handoff_ready", "n_checks_pass", "n_checks_total"):
            self.assertIn(k, d)

    def test_evaluate_w4_full_pass_with_best_components(self):
        # 全部 component done + V0.4 = 0.87 → handoff_ready
        c = R9ComponentStatus(
            v1060_committed=True,
            v1061_cognitive_core_done=True,
            v1062_world_model_done=True,
            v1093_dgm_v04_500loc=True,
            v1078_rl_done=True,
            v1097_mcp_round2_done=True,
            interface_freeze_count=5,
            test_coverage_pct=0.40,
        )
        with mock.patch.object(m, "fetch_three_pieces") as fetch:
            # V1114 兼容 keys
            fetch.return_value = (
                {"module": "V1074", "v03_score": 0.92, "all_ok": True,
                 "philosophy_guard_ok": True, "source": "test_mock"},
                {"module": "V1077", "v04_score": 0.87, "n_dims_filled": 17,
                 "source": "test_mock"},
                {"module": "V1103", "v04_score": 0.86, "top_n": 5,
                 "source": "test_mock"},
                False,
            )
            r = evaluate_w4(week_label="W4", component=c)
            # V0.4 = 0.87 ≥ 0.85 + ≥ 0.86 → passes_w4 + passes_r10_start
            self.assertEqual(r.r10_gap["passes_w4"], True)
            self.assertEqual(r.r10_gap["passes_r10_start"], True)
            # handoff_ready 应 True (≥12/14 = 85%)
            self.assertTrue(r.handoff_ready)
            self.assertGreaterEqual(r.n_checks_pass, 12)
            # track 应该是 Track C (V0.4 ≥ 0.83)
            self.assertEqual(r.track_decision["track"], "C")

    def test_evaluate_w4_live_flag_passed_through(self):
        # live=True 应传给 fetch_three_pieces
        with mock.patch.object(m, "fetch_three_pieces") as fetch:
            fetch.return_value = (
                {"module": "V1074", "v03_score": 0.89, "source": "live"},
                {"module": "V1077", "v04_score": 0.85, "source": "live"},
                {"module": "V1103", "v04_score": 0.84, "source": "live"},
                True,
            )
            r = evaluate_w4(week_label="W4", live=True)
            # live=True 应被传入
            fetch.assert_called_once()
            args, kwargs = fetch.call_args
            if args:
                self.assertEqual(args[0], True)
            else:
                self.assertEqual(kwargs.get("live"), True)
            self.assertEqual(r.raw["used_live"], True)

    def test_evaluate_w4_week_lift_calc(self):
        # week_lift_v04 = v04_actual - w3_v04_baseline (默认 0.8202)
        with mock.patch.object(m, "fetch_three_pieces") as fetch:
            fetch.return_value = (
                {"module": "V1074", "v03_score": 0.89, "source": "mock"},
                {"module": "V1077", "v04_score": 0.85, "source": "mock"},
                {"module": "V1103", "v04_score": 0.84, "source": "mock"},
                False,
            )
            r = evaluate_w4(week_label="W4", live=False)
            # 0.85 - 0.8202 = +0.0298
            self.assertAlmostEqual(r.week_lift_v04, 0.0298, places=4)


# ---------------------------------------------------------------------------
# Markdown 渲染 (2 tests)
# ---------------------------------------------------------------------------

class TestRenderMarkdownW4(unittest.TestCase):
    def test_markdown_contains_key_sections(self):
        with mock.patch.object(m, "fetch_three_pieces") as fetch:
            fetch.return_value = (
                {"module": "V1074", "v03_score": 0.8897, "all_ok": True, "source": "mock"},
                {"module": "V1077", "v04_score": 0.8202, "source": "mock"},
                {"module": "V1103", "v04_score": 0.8188, "source": "mock"},
                False,
            )
            r = evaluate_w4(week_label="W4")
            md = render_markdown_w4(r)
            # 关键章节
            self.assertIn("ASI 北极星 Dashboard", md)
            self.assertIn("R10 起点差距评估", md)
            self.assertIn("W4 末主轨道决策", md)
            self.assertIn("5 Halting 信号真跑", md)
            self.assertIn("R9 → R10 移交 Checklist", md)
            self.assertIn("R10 起点路径建议", md)
            self.assertIn("V3 守门", md)
            self.assertIn("handoff_ready", md)

    def test_markdown_includes_handoff_checklist_table(self):
        with mock.patch.object(m, "fetch_three_pieces") as fetch:
            fetch.return_value = (
                {"module": "V1074", "v03_score": 0.89, "source": "mock"},
                {"module": "V1077", "v04_score": 0.82, "source": "mock"},
                {"module": "V1103", "v04_score": 0.82, "source": "mock"},
                False,
            )
            r = evaluate_w4(week_label="W4")
            md = render_markdown_w4(r)
            # checklist 应包含 ≥12 行
            self.assertGreater(len(r.handoff_checklist), 12)
            # 包含至少一个具体 ID
            self.assertIn("v1074_v03_floor", md)


# ---------------------------------------------------------------------------
# CLI 入口 (3 tests)
# ---------------------------------------------------------------------------

class TestCLIMain(unittest.TestCase):
    def test_help_returns_0(self):
        # argparse --help 会调 sys.exit(0), 应当被 SystemExit 接住
        with self.assertRaises(SystemExit) as cm:
            with redirect_stdout(io.StringIO()):
                main(["--help"])
        self.assertEqual(cm.exception.code, 0)
        # 也验证 --help 输出内容 (再 parse 一次, redirect stderr + stdout)
        # argparse --help 默认发到 stdout, 重定向后者
        out = io.StringIO()
        with self.assertRaises(SystemExit):
            with redirect_stdout(out):
                main(["--help"])
        text = out.getvalue()
        self.assertIn("--week", text)
        self.assertIn("--handoff", text)
        self.assertIn("--live", text)

    def test_json_output_is_valid_json(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            with mock.patch.object(m, "fetch_three_pieces") as fetch:
                fetch.return_value = (
                    {"module": "V1074", "v03_score": 0.8897, "all_ok": True, "source": "mock"},
                    {"module": "V1077", "v04_score": 0.8202, "source": "mock"},
                    {"module": "V1103", "v04_score": 0.8188, "source": "mock"},
                    False,
                )
                rc = main(["--week", "W4", "--json"])
        self.assertEqual(rc, 0)
        out = buf.getvalue()
        data = json.loads(out)
        self.assertEqual(data["week_label"], "W4")
        self.assertIn("dashboard", data)
        self.assertIn("handoff_checklist", data)
        self.assertIn("r10_gap", data)
        self.assertIn("all_ok", data)
        self.assertIn("handoff_ready", data)
        self.assertGreaterEqual(len(data["handoff_checklist"]), 12)

    def test_handoff_flag_includes_paths(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            with mock.patch.object(m, "fetch_three_pieces") as fetch:
                fetch.return_value = (
                    {"module": "V1074", "v03_score": 0.8897, "all_ok": True, "source": "mock"},
                    {"module": "V1077", "v04_score": 0.8202, "source": "mock"},
                    {"module": "V1103", "v04_score": 0.8188, "source": "mock"},
                    False,
                )
                rc = main(["--week", "W4", "--handoff"])
        self.assertEqual(rc, 0)
        text = buf.getvalue()
        self.assertIn("R10", text)
        # W3 baseline → V0.4 < 0.85 → 应提到 "补 V0.4 缺口"
        self.assertIn("补 V0.4 缺口", text)


# ---------------------------------------------------------------------------
# W4 vs V1114 一致性 (3 tests)
# ---------------------------------------------------------------------------

class TestV1119V1114Consistency(unittest.TestCase):
    """验证 V1119 与 V1114 decide engine 一致 (主 19:33 走在前人经验上)."""

    def test_same_dashboard_via_v1114_import(self):
        # V1119 复用 V1114.compute_dashboard → 结果应一致
        # V1114 兼容 key: v03_score / v04_score / n_dims_filled
        v1074 = {"v03_score": 0.8897, "all_ok": True, "philosophy_guard_ok": True}
        v1077 = {"v04_score": 0.8202, "n_dims_filled": 16}
        v1103 = {"v04_score": 0.8188, "top_n": 5}
        # V1119 暴露的 compute_dashboard 来自 V1114 import
        d1 = v1114_compute_dashboard(v1074, v1077, v1103)
        self.assertIn("v03_score", d1)
        self.assertEqual(d1["v04_score"], 0.8202)
        self.assertEqual(d1["v04_v1103"], 0.8188)

    def test_choose_main_track_consistent_for_d_threshold(self):
        # V0.4 = 0.82 → Track D
        h = HaltingSignals()
        t = v1114_choose_main_track(0.82, h)
        self.assertEqual(t.track, "D")

    def test_choose_main_track_c_when_above_threshold(self):
        # V0.4 = 0.85 → Track C (≥ 0.83)
        h = HaltingSignals()
        t = v1114_choose_main_track(0.85, h)
        self.assertEqual(t.track, "C")


if __name__ == "__main__":
    # ponytail: 直接 run 时显示 verbose output (主 00:56 一行可跑)
    unittest.main(verbosity=2)
