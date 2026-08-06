"""V1202 ASI V0.6.12 双 dim lift 测试 (rubric_open + self_organizing_core 联合 lift).

主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手.

测试覆盖:
- 常量 + dataclass 基础
- rubric_open 10 sub-dim 真测 (5 V1160 复用 + 5 V1202 新)
- self_organizing_core 10 sub-dim 真测 (5 V1165 复用 + 5 V1202 新)
- 3-formula (主 17:43)
- ASI recompute + delta + gap
- inflation guard
- artifact 写入 + JSON 读回
- 哲学守门 (主 17:58 + 20:46)
- dim_lifts 字段 + weight + lift_contribution
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

# 路径设置
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1202_asi_v0612_dual_dim_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1201_RECOMPUTE,
    V1202_DIM_VERSION,
    V1202_RUBRIC_OPEN_SUBDIM_NAMES,
    V1202_RUBRIC_OPEN_TARGET,
    V1202_RUBRIC_OPEN_WEIGHT,
    V1202_SELF_ORGANIZING_SUBDIM_NAMES,
    V1202_SELF_ORGANIZING_CORE_TARGET,
    V1202_SELF_ORGANIZING_WEIGHT,
    V1202_VERSION,
    V1202LiftEntry,
    V1202Report,
    V1202SubDimEvidence,
    measure_v1202,
    measure_v1202_additive,
    measure_v1202_corrected,
    measure_v1202_full,
    render_report_md,
)


# Module-level cache (主 00:44 质量工程化: 避免 7 类 × 多次 measure 重复跑 20 sub-dim)
_CACHED_REP: V1202Report = measure_v1202_full(write_artifact=False)


def _get_rep() -> V1202Report:
    return _CACHED_REP


class TestV1202Constants(unittest.TestCase):
    """V1202 常量 (主 00:44 质量工程化)."""

    def test_asu_north_star_locked(self):
        """ASI 北极星 = 0.9800 LOCKED 主 22:33."""
        self.assertEqual(ASI_NORTH_STAR, 0.9800)

    def test_v1202_dim_version(self):
        """V1202 dim_version = 0.6.12."""
        self.assertEqual(V1202_DIM_VERSION, "0.6.12")

    def test_v1202_version(self):
        """V1202 version = 0.1.0."""
        self.assertEqual(V1202_VERSION, "0.1.0")

    def test_v1201_baseline(self):
        """V1201 baseline = 0.9624."""
        self.assertAlmostEqual(V1201_RECOMPUTE, 0.9624, places=4)

    def test_subdim_names_count(self):
        """rubric_open 10 sub-dim names, self_organizing_core 10 sub-dim names."""
        self.assertEqual(len(V1202_RUBRIC_OPEN_SUBDIM_NAMES), 10)
        self.assertEqual(len(V1202_SELF_ORGANIZING_SUBDIM_NAMES), 10)

    def test_rubric_open_subdim_names(self):
        """R1-R5 V1160 复用 + R6-R10 V1202 新."""
        expected = [
            "evaluate_week_real", "halting_signals_real", "dashboard_render_real",
            "v3_guards_real", "track_decision_real",
            "halting_signal_real_run", "v1074_v03_above_floor_real",
            "v03_history_real", "all_ok_real", "guards_v3_guards_real",
        ]
        self.assertEqual(V1202_RUBRIC_OPEN_SUBDIM_NAMES, tuple(expected))

    def test_self_organizing_subdim_names(self):
        """S1-S5 V1165 复用 + S6-S10 V1202 新."""
        expected = [
            "autopoietic_closure", "autocatalytic_raf", "requisite_variety",
            "dissipative_export", "chemoton_coupling",
            "mr_closure_real", "adaptive_diversity_real", "order_param_dominance_real",
            "report_readability_real", "measure_dict_complete_real",
        ]
        self.assertEqual(V1202_SELF_ORGANIZING_SUBDIM_NAMES, tuple(expected))

    def test_targets(self):
        """V1202 targets 合理."""
        self.assertEqual(V1202_RUBRIC_OPEN_TARGET, 0.94)
        self.assertEqual(V1202_SELF_ORGANIZING_CORE_TARGET, 0.97)

    def test_weights(self):
        """V1202 2 dim weight 0.05 each (V1201 沿用)."""
        self.assertEqual(V1202_RUBRIC_OPEN_WEIGHT, 0.05)
        self.assertEqual(V1202_SELF_ORGANIZING_WEIGHT, 0.05)


class TestV1202Measure(unittest.TestCase):
    """V1202 measure 主入口 (主 00:56 任何人都能接手)."""

    def setUp(self):
        self.rep = _get_rep()

    def test_measure_v1202_returns_float(self):
        """measure_v1202() → formula_2 recompute float."""
        v = measure_v1202()
        self.assertIsInstance(v, float)
        self.assertGreater(v, 0.96)
        self.assertLess(v, 0.98)

    def test_measure_v1202_additive(self):
        """measure_v1202_additive() → formula_1 additive float."""
        v = measure_v1202_additive()
        self.assertIsInstance(v, float)

    def test_measure_v1202_corrected(self):
        """measure_v1202_corrected() → formula_3 corrected float."""
        v = measure_v1202_corrected()
        self.assertIsInstance(v, float)

    def test_inflation_guard_zero(self):
        """inflation guard: additive vs recompute = 0 (V1197 fix, no inflation)."""
        self.assertAlmostEqual(self.rep.inflation_gap_additive_vs_recompute, 0.0, places=4)
        self.assertAlmostEqual(self.rep.inflation_gap_additive_vs_corrected, 0.0, places=4)

    def test_asi_recompute_in_range(self):
        """ASI recompute 应在 V1201 和 north_star 之间."""
        self.assertGreater(self.rep.asi_recompute_lifted, V1201_RECOMPUTE)
        self.assertLess(self.rep.asi_recompute_lifted, ASI_NORTH_STAR)

    def test_position_pct_above_98(self):
        """position_pct ≥ 98% of north_star (V1201 = 98.20%, V1202 推高)."""
        self.assertGreaterEqual(self.rep.position_pct_recompute, 98.0)

    def test_gap_to_north_star_positive(self):
        """gap to north_star 仍正 (V1202 不到 ASI 终极, 主 17:58+20:46)."""
        self.assertGreater(self.rep.gap_to_north_star_recompute, 0.0)

    def test_n_dims_lifted_2(self):
        """2 dim 联合 lift."""
        self.assertEqual(self.rep.n_dims_lifted, 2)
        self.assertEqual(self.rep.n_dims_pass, 2)
        self.assertEqual(self.rep.n_dims_partial, 0)
        self.assertEqual(self.rep.n_dims_missing, 0)


class TestV1202Lift(unittest.TestCase):
    """V1202 2 dim lift 内容."""

    def setUp(self):
        self.rep = _get_rep()

    def test_rubric_open_lift_entry(self):
        """rubric_open dim_lifts entry."""
        e = self.rep.dim_lifts.get("rubric_open")
        self.assertIsNotNone(e)
        self.assertIsInstance(e, V1202LiftEntry)
        self.assertEqual(e.dim, "rubric_open")
        self.assertEqual(e.weight, V1202_RUBRIC_OPEN_WEIGHT)
        self.assertEqual(e.sub_dim_count, 10)
        self.assertEqual(e.status, "R")
        self.assertAlmostEqual(e.lift_contribution, e.delta * 0.05, places=4)

    def test_self_organizing_lift_entry(self):
        """self_organizing_core dim_lifts entry."""
        e = self.rep.dim_lifts.get("self_organizing_core")
        self.assertIsNotNone(e)
        self.assertIsInstance(e, V1202LiftEntry)
        self.assertEqual(e.dim, "self_organizing_core")
        self.assertEqual(e.weight, V1202_SELF_ORGANIZING_WEIGHT)
        self.assertEqual(e.sub_dim_count, 10)
        self.assertEqual(e.status, "R")
        self.assertAlmostEqual(e.lift_contribution, e.delta * 0.05, places=4)

    def test_rubric_open_lift_positive(self):
        """rubric_open Δ > 0 (V1201 0.8643 → V1202 > 0.8643)."""
        e = self.rep.dim_lifts["rubric_open"]
        self.assertGreater(e.delta, 0.0)

    def test_self_organizing_lift_positive(self):
        """self_organizing_core Δ > 0 (V1201 0.9095 → V1202 > 0.9095)."""
        e = self.rep.dim_lifts["self_organizing_core"]
        self.assertGreater(e.delta, 0.0)

    def test_total_asi_delta(self):
        """ASI 总 Δ ≈ 2 dim contribution 之和 (主 17:43 实事求是)."""
        ro_c = self.rep.dim_lifts["rubric_open"].lift_contribution
        so_c = self.rep.dim_lifts["self_organizing_core"].lift_contribution
        expected_delta = ro_c + so_c
        self.assertAlmostEqual(self.rep.asi_recompute_delta, expected_delta, places=4)

    def test_3_formula_consistent(self):
        """3-formula 几乎相同 (no inflation)."""
        self.assertAlmostEqual(self.rep.formula_1_additive, self.rep.formula_2_recompute, places=4)
        self.assertAlmostEqual(self.rep.formula_2_recompute, self.rep.formula_3_corrected, places=4)


class TestV1202SubDim(unittest.TestCase):
    """V1202 20 sub-dim 真测细节."""

    def setUp(self):
        self.rep = _get_rep()

    def test_rubric_open_subdim_count(self):
        """rubric_open 10 sub-dim 都被测."""
        self.assertEqual(len(self.rep.rubric_open_sub_dim_scores), 10)
        for name in V1202_RUBRIC_OPEN_SUBDIM_NAMES:
            self.assertIn(name, self.rep.rubric_open_sub_dim_scores)
            self.assertIn(name, self.rep.rubric_open_sub_dim_evidence)

    def test_self_organizing_subdim_count(self):
        """self_organizing_core 10 sub-dim 都被测."""
        self.assertEqual(len(self.rep.self_organizing_sub_dim_scores), 10)
        for name in V1202_SELF_ORGANIZING_SUBDIM_NAMES:
            self.assertIn(name, self.rep.self_organizing_sub_dim_scores)
            self.assertIn(name, self.rep.self_organizing_sub_dim_evidence)

    def test_rubric_open_aggregate_above_85(self):
        """rubric_open aggregate 应在 0.85-0.96 区间 (V1201 0.8643 + lift)."""
        scores = list(self.rep.rubric_open_sub_dim_scores.values())
        agg = sum(scores) / 10.0
        self.assertGreater(agg, 0.85)
        self.assertLess(agg, 0.97)

    def test_self_organizing_aggregate_above_90(self):
        """self_organizing_core aggregate 应在 0.90-0.99 区间."""
        scores = list(self.rep.self_organizing_sub_dim_scores.values())
        agg = sum(scores) / 10.0
        self.assertGreater(agg, 0.85)
        self.assertLess(agg, 1.0)

    def test_rubric_open_evidence_has_checks(self):
        """R1-R10 evidence 有 checks dict."""
        for name, ev in self.rep.rubric_open_sub_dim_evidence.items():
            self.assertIsInstance(ev, V1202SubDimEvidence)
            self.assertGreater(len(ev.checks), 0)

    def test_self_organizing_evidence_has_checks(self):
        """S1-S10 evidence 有 checks dict."""
        for name, ev in self.rep.self_organizing_sub_dim_evidence.items():
            self.assertIsInstance(ev, V1202SubDimEvidence)
            self.assertGreater(len(ev.checks), 0)

    def test_subdim_scores_in_range(self):
        """所有 sub-dim score ∈ [0, 1]."""
        for name, s in self.rep.rubric_open_sub_dim_scores.items():
            self.assertGreaterEqual(s, 0.0, f"{name} below 0")
            self.assertLessEqual(s, 1.0, f"{name} above 1")
        for name, s in self.rep.self_organizing_sub_dim_scores.items():
            self.assertGreaterEqual(s, 0.0, f"{name} below 0")
            self.assertLessEqual(s, 1.0, f"{name} above 1")

    def test_r5_track_decision_partial_honest(self):
        """R5 track_decision_real < 1.0 (主 17:43 实事求是: choose_main_track 需 args)."""
        s = self.rep.rubric_open_sub_dim_scores.get("track_decision_real", 0.0)
        self.assertLess(s, 1.0, "R5 should be partial (0.6) - choose_main_track() 需 v04_score+halting args")
        self.assertGreater(s, 0.0)

    def test_s5_chemoton_partial_honest(self):
        """S5 chemoton_coupling < 1.0 (主 17:43 实事求是: V1065 硬编码 0.6667)."""
        s = self.rep.self_organizing_sub_dim_scores.get("chemoton_coupling", 0.0)
        self.assertLess(s, 1.0, "S5 should be partial (0.833) - V1065 chemoton_coupling 0.6667 < 1.0")


class TestV1202PhilosophyGuard(unittest.TestCase):
    """V3 哲学守门 (主 17:58 + 20:46)."""

    def setUp(self):
        self.rep = _get_rep()

    def test_not_asi_ultimate(self):
        """V1202 ≠ ASI 终极 (gap to north_star > 0)."""
        self.assertGreater(self.rep.gap_to_north_star_recompute, 0.0)

    def test_not_v1160_v1165_replacement(self):
        """V1202 ≠ V1160/V1165 全替代 (V1202 是 10+10 sub-dim 扩展, V1160/V1165 仍 own 5+5)."""
        # V1202 用了 10 sub-dim (V1160 5 + V1202 5)
        self.assertEqual(self.rep.dim_lifts["rubric_open"].sub_dim_count, 10)
        self.assertEqual(self.rep.dim_lifts["self_organizing_core"].sub_dim_count, 10)

    def test_not_v1201_replacement(self):
        """V1202 ≠ V1201 全替代 (V1201 V0.6.11, V1202 V0.6.12 升级路径)."""
        self.assertNotEqual(self.rep.dim_version, "0.6.11")
        self.assertEqual(self.rep.dim_version, "0.6.12")

    def test_not_pretend_r5_full(self):
        """不假装 R5 track_decision_real 修复 (仍 partial)."""
        s = self.rep.rubric_open_sub_dim_scores.get("track_decision_real", 0.0)
        self.assertLess(s, 1.0)

    def test_not_pretend_s5_full(self):
        """不假装 S5 chemoton_coupling 修复 (V1065 硬编码 0.6667, 6 tests 1 strict fail = 0.833)."""
        s = self.rep.self_organizing_sub_dim_scores.get("chemoton_coupling", 0.0)
        self.assertLess(s, 1.0)


class TestV1202Artifact(unittest.TestCase):
    """V1202 artifact 写入 + JSON 读回."""

    def test_artifact_written(self):
        """artifact 写入 + JSON 读回一致."""
        with tempfile.TemporaryDirectory() as tmp:
            rep = measure_v1202_full(write_artifact=True, artifact_dir=tmp)
            self.assertTrue(os.path.exists(rep.artifact_path))
            with open(rep.artifact_path, encoding="utf-8") as f:
                d = json.load(f)
            self.assertEqual(d["version"], V1202_VERSION)
            self.assertEqual(d["dim_version"], V1202_DIM_VERSION)
            self.assertEqual(d["snapshot_id"], rep.snapshot_id)
            self.assertAlmostEqual(d["formula_2_recompute"], rep.formula_2_recompute, places=4)
            self.assertEqual(d["n_dims_lifted"], 2)

    def test_artifact_has_20_subdim(self):
        """artifact 含 20 sub-dim (10 rubric_open + 10 self_organizing_core)."""
        with tempfile.TemporaryDirectory() as tmp:
            rep = measure_v1202_full(write_artifact=True, artifact_dir=tmp)
            with open(rep.artifact_path, encoding="utf-8") as f:
                d = json.load(f)
            self.assertEqual(len(d["rubric_open_sub_dim_scores"]), 10)
            self.assertEqual(len(d["self_organizing_sub_dim_scores"]), 10)

    def test_to_dict_from_dict_roundtrip(self):
        """to_dict / from_dict 往返."""
        with tempfile.TemporaryDirectory() as tmp:
            rep1 = measure_v1202_full(write_artifact=True, artifact_dir=tmp)
            with open(rep1.artifact_path, encoding="utf-8") as f:
                d = json.load(f)
            rep2 = V1202Report.from_dict(d)
            self.assertEqual(rep1.snapshot_id, rep2.snapshot_id)
            self.assertAlmostEqual(rep1.formula_2_recompute, rep2.formula_2_recompute, places=4)
            self.assertEqual(set(rep1.dim_lifts.keys()), set(rep2.dim_lifts.keys()))


class TestV1202Integration(unittest.TestCase):
    """V1202 集成 + 报告."""

    def test_summary_line(self):
        """summary_line 含 snapshot_id."""
        rep = measure_v1202_full(write_artifact=False)
        line = rep.summary_line()
        self.assertIn("V1202 ASI V0.6.12", line)
        self.assertIn(rep.snapshot_id, line)

    def test_report_md_contains_subsections(self):
        """render_report_md 含 5+ subsection."""
        rep = measure_v1202_full(write_artifact=False)
        md = render_report_md(rep)
        self.assertIn("V1202 ASI V0.6.12", md)
        self.assertIn("3-formula", md)
        self.assertIn("ASI 北极星", md)
        self.assertIn("rubric_open", md)
        self.assertIn("self_organizing_core", md)
        self.assertIn("rubric_open 10 sub-dim", md)
        self.assertIn("self_organizing_core 10 sub-dim", md)


class TestV1202Measurements(unittest.TestCase):
    """V1202 测量真实值 (主 17:43 实事求是)."""

    def setUp(self):
        self.rep = _get_rep()

    def test_r1_evaluate_week_full(self):
        """R1 evaluate_week_real = 1.0 (V1114 真有 10 keys)."""
        self.assertAlmostEqual(
            self.rep.rubric_open_sub_dim_scores["evaluate_week_real"], 1.0, places=4
        )

    def test_r6_halting_signal_real_run_full(self):
        """R6 halting_signal_real_run = 1.0 (5 halting signals 真跑)."""
        self.assertAlmostEqual(
            self.rep.rubric_open_sub_dim_scores["halting_signal_real_run"], 1.0, places=4
        )

    def test_r10_guards_v3_guards_full(self):
        """R10 guards_v3_guards_real = 1.0 (V3_GUARDS 6 keys 全覆盖)."""
        self.assertAlmostEqual(
            self.rep.rubric_open_sub_dim_scores["guards_v3_guards_real"], 1.0, places=4
        )

    def test_s1_autopoietic_closure_full(self):
        """S1 autopoietic_closure = 1.0."""
        self.assertAlmostEqual(
            self.rep.self_organizing_sub_dim_scores["autopoietic_closure"], 1.0, places=4
        )

    def test_s6_mr_closure_full(self):
        """S6 mr_closure_real = 1.0 (V1065 measure['mr_closure'] = 1.0)."""
        self.assertAlmostEqual(
            self.rep.self_organizing_sub_dim_scores["mr_closure_real"], 1.0, places=4
        )


if __name__ == "__main__":
    unittest.main()
