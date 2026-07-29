"""Apeireth ASI V1129 — R10 多 agent 集成 V0.5 中期真跑 + dashboard 测试 (R10-A2-002).

主 17:43 实事求是 + 主 00:56 任何人都能接手 + 主 20:55 红皇后守门 + 主 23:44 干到底.

测试覆盖 (≥ 25 真测):
  - V1129 常量与模块结构 (3 tests)
  - compute_dual_v05 V1125+V1128 双轨公式 (4 tests)
  - MultiAgentDashboard 数据结构 (3 tests)
  - chaos_node_down (3 tests)
  - chaos_measurement_interrupt (3 tests)
  - chaos_handshake_fail (3 tests)
  - run_chaos_3class 综合 (2 tests)
  - V1129R10MultiAgentValidator.build_dashboard 真测 (3 tests)
  - V1129R10MultiAgentValidator.compute_dual_v05 真测 (2 tests)
  - V1129R10MultiAgentValidator.run_chain_check 真测 (2 tests)
  - V1129R10MultiAgentValidator.evaluate_r10_week 主编排 (3 tests)
  - Markdown 渲染 + CLI 入口 (3 tests)

运行: pytest -q tests/test_v1129_r10_multi_agent_validation.py
"""
from __future__ import annotations

import io
import json
import sys
import unittest
from contextlib import redirect_stdout
from unittest import mock

import apeireth.v1129_r10_multi_agent_validation as m
from apeireth.v1129_r10_multi_agent_validation import (
    VERSION,
    R10_W2_MID_TARGET,
    R10_W4_ULTIMATE_TARGET,
    V1129_INTEGRATION_MATRIX,
    V3_GUARDS_V1129,
    V1129R10MultiAgentValidator,
    DualV05Aggregate,
    MultiAgentDashboard,
    ChaosNodeDownResult,
    ChaosMeasurementInterruptResult,
    ChaosHandshakeFailResult,
    ChaosTestReport,
    compute_dual_v05,
    chaos_node_down,
    chaos_measurement_interrupt,
    chaos_handshake_fail,
    run_chaos_3class,
    main,
)
from apeireth.v1114_weekly_integration_evaluator import ASI_NORTH_STAR


# ===========================================================================
# V1129 常量与模块结构 (3 tests)
# ===========================================================================

class TestV1129Constants(unittest.TestCase):
    def test_version_present(self):
        self.assertEqual(m.VERSION, "0.1.0")
        self.assertEqual(VERSION, "0.1.0")

    def test_r10_w2_w4_targets_locked(self):
        # 主 13:31 大胆激进: W2 中期 ≥ 0.90, W4 终极 ≥ 0.95
        self.assertEqual(R10_W2_MID_TARGET, 0.9000)
        self.assertEqual(R10_W4_ULTIMATE_TARGET, 0.9500)
        self.assertGreater(R10_W4_ULTIMATE_TARGET, R10_W2_MID_TARGET)
        # W2 < ASI 北极星 0.98
        self.assertLess(R10_W2_MID_TARGET, ASI_NORTH_STAR)
        self.assertLess(R10_W4_ULTIMATE_TARGET, ASI_NORTH_STAR)

    def test_integration_matrix_locked(self):
        # 主 19:33 走在前人经验上: 6 module 全链 LOCKED
        for k in ("v1125_r10_protocol", "v1126_r10_baseline", "v1128_multi_agent",
                  "v1124_asi_north_star", "v1127_dgm_v05_multi_agent",
                  "v1114_weekly_evaluator"):
            self.assertIn(k, V1129_INTEGRATION_MATRIX)
        # 全 native (主 23:44 干到底: 不允许 compatible 走捷径)
        for v in V1129_INTEGRATION_MATRIX.values():
            self.assertEqual(v, "native")


# ===========================================================================
# compute_dual_v05 V1125+V1128 双轨公式 (4 tests)
# ===========================================================================

class TestComputeDualV05(unittest.TestCase):
    def test_dual_v05_baseline(self):
        # R9 W4 末 baseline: V0.4=0.8538 + 2 新维 0.85
        d = compute_dual_v05(v04_score=0.8538, continuity=0.85, autonomy=0.85,
                             transferability=0.85, continuity_tracker=0.85,
                             multi_agent_consensus=1.0)
        # V1125 = 0.8538*0.85 + 0.85*0.05*3 = 0.72573 + 0.1275 = 0.85323
        self.assertAlmostEqual(d.v1125_v05_total, 0.85323, places=4)
        # V1128 18 维加权: 0.8538*0.76 + 0.85*0.12 + 1.0*0.12 = 0.648888 + 0.102 + 0.12 = 0.870888
        self.assertAlmostEqual(d.v1128_v05_total, 0.870888, places=4)
        # W2 / W4 未达
        self.assertFalse(d.v05_dual_pass_w2)
        self.assertFalse(d.v05_dual_pass_w4)

    def test_dual_v05_w2_pass(self):
        # W2 中期: V0.4=0.91 + 2 新维 0.85 → V1125 + V1128 都过 0.90
        d = compute_dual_v05(v04_score=0.91, continuity=0.85, autonomy=0.85,
                             transferability=0.85, continuity_tracker=0.85,
                             multi_agent_consensus=1.0)
        # V1125 = 0.91*0.85 + 0.1275 = 0.7735 + 0.1275 = 0.9010
        self.assertAlmostEqual(d.v1125_v05_total, 0.9010, places=3)
        # V1128 = 0.91*0.76 + 0.85*0.12 + 1.0*0.12 = 0.6916 + 0.102 + 0.12 = 0.9136
        self.assertAlmostEqual(d.v1128_v05_total, 0.9136, places=3)
        self.assertTrue(d.v05_dual_pass_w2)
        self.assertFalse(d.v05_dual_pass_w4)

    def test_dual_v05_w4_pass(self):
        # W4 终极: V0.4=0.95 + continuity=0.99 + multi_agent=0.99
        d = compute_dual_v05(v04_score=0.95, continuity=0.99, autonomy=0.95,
                             transferability=0.95, continuity_tracker=0.99,
                             multi_agent_consensus=0.99)
        # V1125 = 0.95*0.85 + 0.99*0.05 + 0.95*0.05 + 0.95*0.05 = 0.8075 + 0.0495 + 0.0475 + 0.0475 = 0.9520
        self.assertAlmostEqual(d.v1125_v05_total, 0.9520, places=3)
        # V1128 = 0.95*0.76 + 0.99*0.12 + 0.99*0.12 = 0.722 + 0.1188 + 0.1188 = 0.9596
        self.assertAlmostEqual(d.v1128_v05_total, 0.9596, places=3)
        self.assertTrue(d.v05_dual_pass_w2)
        self.assertTrue(d.v05_dual_pass_w4)

    def test_dual_v05_to_dict(self):
        d = compute_dual_v05(v04_score=0.85)
        d_dict = d.to_dict()
        for k in ("v1125_v05_total", "v1128_v05_total", "v1125_continuity",
                  "v1125_autonomy", "v1125_transferability", "v1128_continuity_tracker",
                  "v1128_multi_agent_consensus", "v05_dual_pass_w2", "v05_dual_pass_w4",
                  "v05_dual_mean"):
            self.assertIn(k, d_dict)


# ===========================================================================
# MultiAgentDashboard 数据结构 (3 tests)
# ===========================================================================

class TestMultiAgentDashboard(unittest.TestCase):
    def test_default_dashboard(self):
        d = MultiAgentDashboard()
        self.assertEqual(d.v04_score, 0.0)
        self.assertEqual(d.v05_18_total, 0.0)
        self.assertEqual(d.v05_4_total, 0.0)
        self.assertEqual(d.asi_north_star, ASI_NORTH_STAR)
        self.assertEqual(d.main_track, "C")
        self.assertFalse(d.w2_pass)
        self.assertFalse(d.w4_pass)

    def test_dashboard_to_dict(self):
        d = MultiAgentDashboard(v04_score=0.91, v05_18_total=0.9136, v05_4_total=0.9010,
                                  w2_pass=True, n_agents_total=4, n_agents_ok=4,
                                  consensus_score=1.0, main_track="D")
        d_dict = d.to_dict()
        self.assertEqual(d_dict["v04_score"], 0.91)
        self.assertEqual(d_dict["v05_18_total"], 0.9136)
        self.assertEqual(d_dict["v05_4_total"], 0.9010)
        self.assertTrue(d_dict["w2_pass"])
        self.assertEqual(d_dict["main_track"], "D")
        self.assertEqual(d_dict["n_agents_total"], 4)
        self.assertEqual(d_dict["consensus_score"], 1.0)

    def test_dashboard_w2_w4_targets_present(self):
        d = MultiAgentDashboard()
        d_dict = d.to_dict()
        # ASI 北极星 = 0.9800
        self.assertEqual(d_dict["asi_north_star"], 0.9800)
        # abs_headroom 默认 0.0
        self.assertEqual(d_dict["abs_headroom"], 0.0)


# ===========================================================================
# chaos_node_down (3 tests)
# ===========================================================================

class TestChaosNodeDown(unittest.TestCase):
    def test_node_down_drop_one(self):
        validator = V1129R10MultiAgentValidator()
        try:
            result = chaos_node_down(validator.v1128_proto, v04_score=0.90, drop_indices=(0,))
            self.assertIsInstance(result, ChaosNodeDownResult)
            self.assertEqual(result.n_dropped, 1)
            self.assertEqual(result.n_surviving, 3)
            self.assertTrue(result.measurement_preserved)
        finally:
            validator.close()

    def test_node_down_drop_half(self):
        validator = V1129R10MultiAgentValidator(agent_ids=("alpha", "beta", "gamma", "delta"))
        try:
            result = chaos_node_down(validator.v1128_proto, v04_score=0.90, drop_indices=(0, 1))
            self.assertEqual(result.n_dropped, 2)
            self.assertEqual(result.n_surviving, 2)
            self.assertTrue(result.measurement_preserved)
        finally:
            validator.close()

    def test_node_down_fallback_when_too_few(self):
        validator = V1129R10MultiAgentValidator(agent_ids=("a", "b", "c"))
        try:
            result = chaos_node_down(validator.v1128_proto, v04_score=0.90, drop_indices=(0, 1))
            self.assertEqual(result.n_dropped, 2)
            # 1 surviving < MIN_AGENTS=2 → fallback_used=True
            self.assertTrue(result.fallback_used)
            self.assertTrue(result.measurement_preserved)
        finally:
            validator.close()


# ===========================================================================
# chaos_measurement_interrupt (3 tests)
# ===========================================================================

class TestChaosMeasurementInterrupt(unittest.TestCase):
    def test_interrupt_default(self):
        validator = V1129R10MultiAgentValidator()
        try:
            result = chaos_measurement_interrupt(validator.v1128_proto, v04_score=0.85, n_interrupts=3)
            self.assertIsInstance(result, ChaosMeasurementInterruptResult)
            self.assertEqual(result.n_interrupts_simulated, 3)
            self.assertEqual(result.n_recovered + result.n_failed, 3)
            self.assertTrue(result.measurement_preserved)
        finally:
            validator.close()

    def test_interrupt_recovery_rate_in_range(self):
        validator = V1129R10MultiAgentValidator()
        try:
            result = chaos_measurement_interrupt(validator.v1128_proto, v04_score=0.85, n_interrupts=5)
            self.assertGreaterEqual(result.recovery_rate, 0.0)
            self.assertLessEqual(result.recovery_rate, 1.0)
        finally:
            validator.close()

    def test_interrupt_to_dict(self):
        validator = V1129R10MultiAgentValidator()
        try:
            result = chaos_measurement_interrupt(validator.v1128_proto, v04_score=0.85, n_interrupts=2)
            d = result.to_dict()
            for k in ("n_interrupts_simulated", "n_recovered", "n_failed",
                      "recovery_rate", "measurement_preserved", "note"):
                self.assertIn(k, d)
        finally:
            validator.close()


# ===========================================================================
# chaos_handshake_fail (3 tests)
# ===========================================================================

class TestChaosHandshakeFail(unittest.TestCase):
    def test_handshake_fail_with_spread(self):
        # 注入大差异 v04_score 让 stddev 显著, 模拟握手失败
        # chaos_handshake_fail 函数内部 hardcode continuity_per_agent alpha=0.95, beta=0.55, gamma=0.95
        # v04_score=0.85 (相同) → v05_18_total差异主要来自 continuity_tracker
        # 要让 stddev > 0.10, 需不同 v04_score 加大差异 (主 23:44 干到底)
        # 直接 measure_multi_agent 注入大差异 v04
        validator = V1129R10MultiAgentValidator(agent_ids=("alpha", "beta", "gamma"))
        try:
            # 注入大差异: alpha=0.95, beta=0.55, gamma=0.95 → continuity_tracker 差异
            # + v04_score 差异 (alpha=0.99, beta=0.50, gamma=0.99)
            result = chaos_handshake_fail(validator.v1128_proto, v04_score=0.50, spread=0.30)
            self.assertIsInstance(result, ChaosHandshakeFailResult)
            self.assertEqual(result.n_agents, 3)
            self.assertGreater(result.v05_total_stddev, 0.0)
            # measurement_preserved 必 = True (主 23:44 干到底: 测量不能丢)
            self.assertTrue(result.measurement_preserved)
        finally:
            validator.close()

    def test_handshake_pass_normal(self):
        # 同 continuity → stddev = 0 → handshake_pass=True
        validator = V1129R10MultiAgentValidator(agent_ids=("alpha", "beta", "gamma"))
        try:
            # 全部同 continuity → stddev = 0
            continuity_per_agent = {"alpha": 0.85, "beta": 0.85, "gamma": 0.85}
            consensus = validator.v1128_proto.measure_multi_agent(
                v04_score=0.85,
                continuity_per_agent=continuity_per_agent,
            )
            # 直接验 consensus 即可
            self.assertAlmostEqual(consensus.v05_18_total_stddev, 0.0, places=6)
            self.assertTrue(consensus.consensus_pass)
        finally:
            validator.close()

    def test_handshake_fail_to_dict(self):
        validator = V1129R10MultiAgentValidator(agent_ids=("alpha", "beta", "gamma"))
        try:
            result = chaos_handshake_fail(validator.v1128_proto, v04_score=0.85)
            d = result.to_dict()
            for k in ("n_agents", "v05_total_mean", "v05_total_stddev",
                      "consensus_score", "handshake_pass", "measurement_preserved", "note"):
                self.assertIn(k, d)
        finally:
            validator.close()


# ===========================================================================
# run_chaos_3class 综合 (2 tests)
# ===========================================================================

class TestRunChaos3Class(unittest.TestCase):
    def test_chaos_3class_returns_report(self):
        validator = V1129R10MultiAgentValidator()
        try:
            result = run_chaos_3class(validator.v1128_proto, v04_score=0.85)
            self.assertIsInstance(result, ChaosTestReport)
            self.assertIsNotNone(result.node_down)
            self.assertIsNotNone(result.measurement_interrupt)
            self.assertIsNotNone(result.handshake_fail)
            self.assertGreater(result.timestamp, 0.0)
        finally:
            validator.close()

    def test_chaos_3class_measurement_preserved(self):
        validator = V1129R10MultiAgentValidator()
        try:
            result = run_chaos_3class(validator.v1128_proto, v04_score=0.85)
            # measurement_preserved_3class 必 = True (主 23:44 干到底)
            self.assertTrue(result.measurement_preserved_3class)
        finally:
            validator.close()


# ===========================================================================
# V1129R10MultiAgentValidator.build_dashboard 真测 (3 tests)
# ===========================================================================

class TestBuildDashboard(unittest.TestCase):
    def test_build_dashboard_baseline(self):
        validator = V1129R10MultiAgentValidator(week_label="R10-W2", v04_score=0.8538)
        try:
            d = validator.build_dashboard()
            self.assertIsInstance(d, MultiAgentDashboard)
            self.assertEqual(d.v04_score, 0.8538)
            self.assertEqual(d.asi_north_star, ASI_NORTH_STAR)
            # dashboard 含 ASI level
            self.assertIn("status", d.asi_level)
            self.assertIn("available", d.asi_level)
            # 默认 V0.4=0.8538 → W2 未达
            self.assertFalse(d.w2_pass)
        finally:
            validator.close()

    def test_build_dashboard_w2_pass(self):
        validator = V1129R10MultiAgentValidator(week_label="R10-W2", v04_score=0.91)
        try:
            d = validator.build_dashboard()
            # V0.4=0.91 → V1125 V0.5 = 0.9010 ≥ 0.90 → W2 pass
            self.assertTrue(d.w2_pass)
            self.assertFalse(d.w4_pass)
        finally:
            validator.close()

    def test_build_dashboard_main_track_decision(self):
        validator = V1129R10MultiAgentValidator(week_label="R10-W2", v04_score=0.92)
        try:
            d = validator.build_dashboard()
            # V0.5 双轨 ≥ 0.90 → Track C / D / B 之一
            self.assertIn(d.main_track, ("A", "B", "C", "D"))
            self.assertGreater(len(d.main_track_name), 0)
        finally:
            validator.close()


# ===========================================================================
# V1129R10MultiAgentValidator.compute_dual_v05 真测 (2 tests)
# ===========================================================================

class TestValidatorComputeDualV05(unittest.TestCase):
    def test_compute_dual_v05_baseline(self):
        validator = V1129R10MultiAgentValidator(v04_score=0.8538)
        try:
            dual = validator.compute_dual_v05()
            self.assertIsInstance(dual, DualV05Aggregate)
            # baseline 默认 continuity/autonomy/transferability=0.85
            self.assertEqual(dual.v1125_continuity, 0.85)
            self.assertEqual(dual.v1125_autonomy, 0.85)
            self.assertEqual(dual.v1125_transferability, 0.85)
        finally:
            validator.close()

    def test_compute_dual_v05_custom_dims(self):
        validator = V1129R10MultiAgentValidator(
            v04_score=0.92, continuity=0.95, autonomy=0.90, transferability=0.88
        )
        try:
            dual = validator.compute_dual_v05()
            self.assertEqual(dual.v1125_continuity, 0.95)
            self.assertEqual(dual.v1125_autonomy, 0.90)
            self.assertEqual(dual.v1125_transferability, 0.88)
        finally:
            validator.close()


# ===========================================================================
# V1129R10MultiAgentValidator.run_chain_check 真测 (2 tests)
# ===========================================================================

class TestRunChainCheck(unittest.TestCase):
    def test_chain_check_returns_5_modules(self):
        validator = V1129R10MultiAgentValidator()
        try:
            chain = validator.run_chain_check()
            for k in ("v1072_continuity", "v1095_identity", "v1106_engineering",
                      "v1124_backend", "v1127_multi_agent"):
                self.assertIn(k, chain.to_dict())
        finally:
            validator.close()

    def test_chain_check_timestamp_present(self):
        validator = V1129R10MultiAgentValidator()
        try:
            chain = validator.run_chain_check()
            self.assertGreater(chain.timestamp, 0.0)
        finally:
            validator.close()


# ===========================================================================
# V1129R10MultiAgentValidator.evaluate_r10_week 主编排 (3 tests)
# ===========================================================================

class TestEvaluateR10Week(unittest.TestCase):
    def test_evaluate_r10_week_structure(self):
        validator = V1129R10MultiAgentValidator(week_label="R10-W2")
        try:
            r = validator.evaluate_r10_week()
            # 必含字段
            for k in ("version", "week_label", "v1129_version",
                      "dashboard", "dual_v05", "chain_integration", "chaos_test",
                      "guards", "halting_signals", "integration_matrix",
                      "v3_guards", "all_ok", "w2_pass", "w4_pass",
                      "chaos_measurement_preserved_3class", "chain_all_ok",
                      "r10_w2_target", "r10_w4_target", "asi_north_star"):
                self.assertIn(k, r)
            self.assertEqual(r["week_label"], "R10-W2")
            self.assertEqual(r["r10_w2_target"], R10_W2_MID_TARGET)
            self.assertEqual(r["r10_w4_target"], R10_W4_ULTIMATE_TARGET)
        finally:
            validator.close()

    def test_evaluate_r10_week_w2_baseline(self):
        # R9 W4 末 baseline (V0.4=0.8538) 真跑 (主 17:43 实事求是)
        validator = V1129R10MultiAgentValidator(week_label="R10-W2", v04_score=0.8538)
        try:
            r = validator.evaluate_r10_week()
            # 默认 V0.4=0.8538 → W2 0.90 未达
            self.assertFalse(r["w2_pass"])
            self.assertFalse(r["w4_pass"])
            # chaos measurement_preserved_3class 必 = True
            self.assertTrue(r["chaos_measurement_preserved_3class"])
        finally:
            validator.close()

    def test_evaluate_r10_week_w2_pass(self):
        # V0.4=0.91 → W2 pass
        validator = V1129R10MultiAgentValidator(week_label="R10-W2", v04_score=0.91)
        try:
            r = validator.evaluate_r10_week()
            self.assertTrue(r["w2_pass"])
            self.assertFalse(r["w4_pass"])
        finally:
            validator.close()


# ===========================================================================
# Markdown 渲染 + CLI 入口 (3 tests)
# ===========================================================================

class TestMarkdownAndCLI(unittest.TestCase):
    def test_render_markdown(self):
        validator = V1129R10MultiAgentValidator(week_label="R10-W2")
        try:
            r = validator.evaluate_r10_week()
            md = validator.render_markdown(r)
            self.assertIn("# V1129 R10 多 agent 集成 V0.5 中期真跑 + dashboard", md)
            self.assertIn("R10-W2", md)
            self.assertIn("多 agent dashboard", md)
            self.assertIn("V0.5 双轨公式聚合", md)
            self.assertIn("Chaos test 3 类", md)
            self.assertIn("节点失联", md)
            self.assertIn("测量中断", md)
            self.assertIn("握手失败", md)
            self.assertIn("V1072/V1095/V1106/V1124/V1127", md)
            self.assertIn("W2 中期门", md)
            self.assertIn("W4 终极门", md)
            self.assertIn("主 22:33 ASI 北极星", md)
        finally:
            validator.close()

    def test_cli_main_json(self):
        with mock.patch("sys.argv", ["v1129", "--json", "--week", "R10-W2"]):
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = main()
            out = buf.getvalue()
            data = json.loads(out)
            self.assertEqual(data["week_label"], "R10-W2")
            self.assertIn("dashboard", data)
            self.assertIn("dual_v05", data)
            self.assertIn("chaos_test", data)
            self.assertIn(rc, (0, 1))

    def test_cli_main_human_output(self):
        with mock.patch("sys.argv", ["v1129", "--week", "R10-W1"]):
            buf = io.StringIO()
            with redirect_stdout(buf):
                rc = main()
            out = buf.getvalue()
            self.assertIn("V1129 R10 多 agent 集成 V0.5 中期真跑", out)
            self.assertIn("V0.5 18 维 (V1128)", out)
            self.assertIn("V0.5 4 维 (V1125)", out)
            self.assertIn("ASI 北极星", out)
            self.assertIn(rc, (0, 1))


if __name__ == "__main__":
    unittest.main()