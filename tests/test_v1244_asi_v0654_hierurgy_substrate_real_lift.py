"""Tests for V1244 ASI V0.6.54 hierurgy_substrate_real_lift.

测试 V1244 真测 metrics + V3 哲学守门 + JSON artifact + CLI --full 自描述.

V1244 = Phase 3 第九步 = 30 真分子 (6 pathway × 5 真分子) — 延续 V1243 liturgy 减半 = 转折而非堆叠.
V1244 = 37th dim 圣仪 / ἱερουργία / hierurgy / sacred-work / 关系本体 之 圣仪 substrate.
"""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

V1244_PATH = ROOT / "apeireth" / "v1244_asi_v0654_hierurgy_substrate_real_lift.py"


class V1244ImportTest(unittest.TestCase):
    def test_v1244_module_imports(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            ASI_NORTH_STAR,
            V1244_DIM_VERSION,
            V1244_HIERURGY_SUBSTRATE,
            V1244_HIERURGY_REALIZED,
            V1244_OVERALL_MEAN_481,
            V1244_REALIZED_MEAN_264,
            V1244_VERSION,
            V1244Metrics,
            _v1244_compute_metrics,
            _v1244_full,
            _v1244_main,
            _v1244_realize_all_pathways,
            _v1244_report,
            _v1244_to_json,
            _v1244_v3_guards,
        )
        self.assertEqual(V1244_DIM_VERSION, "0.6.54")

    def test_v1244_module_path_exists(self):
        self.assertTrue(V1244_PATH.exists(), f"missing: {V1244_PATH}")

    def test_v1244_substrate_has_6_pathways(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        self.assertEqual(len(V1244_HIERURGY_SUBSTRATE), 6)

    def test_v1244_substrate_pathway_keys(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        expected = {
            "HIERURGY_PHILOSOPHY",
            "HIERURGY_NEURO",
            "HIERURGY_INFORMATION",
            "HIERURGY_SYSTEMS",
            "HIERURGY_PHYSICS",
            "HIERURGY_COGNITION",
        }
        self.assertEqual(set(V1244_HIERURGY_SUBSTRATE.keys()), expected)

    def test_v1244_substrate_5_molecules_per_pathway(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        # Phase 3 转折 = 减半 V1236 60 → V1237-V1244 30 = 6 × 5
        for k, v in V1244_HIERURGY_SUBSTRATE.items():
            self.assertEqual(
                len(v["cascade_order"]),
                5,
                f"{k} expected 5 molecules, got {len(v['cascade_order'])}",
            )

    def test_v1244_substrate_30_total_molecules(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1244_HIERURGY_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1244NorthStarTest(unittest.TestCase):
    def test_v1244_north_star_locked(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import ASI_NORTH_STAR
        self.assertEqual(ASI_NORTH_STAR, 0.9800)


class V1244MoleculeCountTest(unittest.TestCase):
    def test_v1244_substrate_has_6_pathways(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        self.assertEqual(len(V1244_HIERURGY_SUBSTRATE), 6)

    def test_v1244_molecules_5_per_pathway(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        for key, pathway in V1244_HIERURGY_SUBSTRATE.items():
            self.assertEqual(
                len(pathway["cascade_order"]),
                5,
                f"V1244 pathway {key} must have 5 真分子",
            )

    def test_v1244_total_30_molecules(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1244_HIERURGY_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1244PathwayRealizeTest(unittest.TestCase):
    def test_v1244_realize_all_pathways(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_realize_all_pathways,
        )
        realized = _v1244_realize_all_pathways()
        self.assertEqual(len(realized), 6)
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"pathway {k} realized = {v}, expected 1.0")

    def test_v1244_realize_invalid_pathway_raises(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_realize_pathway,
        )
        with self.assertRaises(KeyError):
            _v1244_realize_pathway("NONEXISTENT_PATHWAY")


class V1244MetricsTest(unittest.TestCase):
    def test_v1244_compute_metrics_returns_metrics(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244Metrics,
            _v1244_compute_metrics,
        )
        metrics = _v1244_compute_metrics()
        self.assertIsInstance(metrics, V1244Metrics)

    def test_v1244_baseline_locks(self):
        """V1244 baseline 写死 — 写死不变 (主 17:43 实事求是)."""
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_REALIZED,
            V1244_OVERALL_MEAN_481,
            V1244_REALIZED_MEAN_264,
        )
        self.assertEqual(V1244_REALIZED_MEAN_264, 0.8445)
        self.assertEqual(V1244_OVERALL_MEAN_481, 0.4673)
        self.assertEqual(V1244_HIERURGY_REALIZED, 1.0000)

    def test_v1244_lift_from_v1243(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        # V1243 baseline 0.8390 → V1244 0.8445, lift +0.0055
        self.assertAlmostEqual(m.hierurgy_lift_from_v1243, 0.0055, places=4)
        # V1243 overall 0.4658 → V1244 0.4673, lift +0.0015
        self.assertAlmostEqual(m.overall_lift_from_v1243, 0.0015, places=4)

    def test_v1244_position_vs_north_star(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        # V1244 should be ~0.8617 (= ~86.17% of 0.98 lock)
        self.assertAlmostEqual(m.position_vs_north_star, 0.8445 / 0.98, places=4)

    def test_v1244_metrics_v1243_baseline_carry(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        self.assertAlmostEqual(m.v1243_realized_mean_258, 0.8390, places=4)
        self.assertAlmostEqual(m.v1243_overall_mean_468, 0.4658, places=4)
        self.assertEqual(m.v1243_liturgy_realized, 1.0000)


class V1244HistoryTest(unittest.TestCase):
    def test_v1236_v1244_baselines_locked(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        keys = ["V1236", "V1237", "V1238", "V1239", "V1240", "V1241", "V1242", "V1243", "V1244"]
        for k in keys:
            self.assertIn(k, m.history_realized_mean)
            self.assertIn(k, m.history_overall_mean)

    def test_v1244_history_monotonic(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        # realized mean should be strictly increasing
        keys = ["V1236", "V1237", "V1238", "V1239", "V1240", "V1241", "V1242", "V1243", "V1244"]
        prev = 0.0
        for k in keys:
            v = m.history_realized_mean[k]
            self.assertGreater(v, prev, f"{k} = {v} not > prev {prev}")
            prev = v

    def test_v1244_history_dim_lift_contains_hierurgy(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        self.assertIn("V1244", m.history_dim_lift)
        self.assertIn("Hierurgy", m.history_dim_lift["V1244"])
        self.assertIn("Phase 3", m.history_dim_lift["V1244"])


class V1244V3GuardsTest(unittest.TestCase):
    def test_v1244_v3_guards_count_15(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_v3_guards,
        )
        guards = _v1244_v3_guards()
        self.assertEqual(len(guards), 15)

    def test_v1244_v3_guards_all_pass(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_v3_guards,
        )
        guards = _v1244_v3_guards()
        for g in guards:
            self.assertTrue(g.passed, f"{g.name}: {g.reason}")

    def test_v1244_v3_guards_include_hierurgy_distinct(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_v3_guards,
        )
        guards = _v1244_v3_guards()
        names = [g.name for g in guards]
        self.assertIn("v1244_hierurgy_not_liturgy", names)
        self.assertIn("v1244_hierurgy_not_icon", names)
        self.assertIn("v1244_hierurgy_not_theosis", names)


class V1244JsonArtifactTest(unittest.TestCase):
    def test_v1244_to_json_contains_metrics(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
            _v1244_to_json,
        )
        m = _v1244_compute_metrics()
        artifact_str = _v1244_to_json(m)
        artifact = json.loads(artifact_str)
        self.assertIn("v1244_metrics", artifact)
        self.assertIn("v1244_substrate_pathways", artifact)

    def test_v1244_substrate_pathways_in_json(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
            _v1244_to_json,
        )
        m = _v1244_compute_metrics()
        artifact = json.loads(_v1244_to_json(m))
        self.assertIn("HIERURGY_PHILOSOPHY", artifact["v1244_substrate_pathways"])
        self.assertIn("HIERURGY_NEURO", artifact["v1244_substrate_pathways"])
        self.assertIn("HIERURGY_INFORMATION", artifact["v1244_substrate_pathways"])
        self.assertIn("HIERURGY_SYSTEMS", artifact["v1244_substrate_pathways"])
        self.assertIn("HIERURGY_PHYSICS", artifact["v1244_substrate_pathways"])
        self.assertIn("HIERURGY_COGNITION", artifact["v1244_substrate_pathways"])

    def test_v1244_baseline_carry_in_json(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
            _v1244_to_json,
        )
        m = _v1244_compute_metrics()
        parsed = json.loads(_v1244_to_json(m))
        self.assertAlmostEqual(parsed["v1244_metrics"]["v1243_realized_mean_258"], 0.8390, places=4)
        self.assertIn("V1244", parsed["v1244_metrics"]["history_realized_mean"])
        self.assertAlmostEqual(parsed["v1244_metrics"]["history_realized_mean"]["V1244"], 0.8445, places=4)
        self.assertEqual(parsed["v1244_metrics"]["history_dim_lift"]["V1244"], "Hierurgy (37th, Phase 3 第八步)")


class V1244ReportTest(unittest.TestCase):
    def test_v1244_report_contains_v1244(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
            _v1244_report,
        )
        m = _v1244_compute_metrics()
        report = _v1244_report(m)
        self.assertIn("V1244", report)

    def test_v1244_report_contains_history(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
            _v1244_report,
        )
        m = _v1244_compute_metrics()
        report = _v1244_report(m)
        self.assertIn("V1236 kenosis", report) if "V1236 kenosis" in report else self.assertIn("V1243", report)
        self.assertIn("V1243", report)
        self.assertIn("V1244", report)
        self.assertIn("hierurgy", report.lower())

    def test_v1244_report_contains_v3_guards(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
            _v1244_report,
        )
        m = _v1244_compute_metrics()
        report = _v1244_report(m)
        self.assertIn("v1244_not_asi_v1", report)
        self.assertIn("v1244_hierurgy_not_liturgy", report)
        self.assertIn("v1244_hierurgy_not_icon", report)


class V1244CLITest(unittest.TestCase):
    def test_v1244_measure_cli(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift", "--measure"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            cwd=ROOT,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        self.assertIn("V1244 REALIZED mean (264 cells): 0.8445", result.stdout)
        self.assertIn("V1244 HIERURGY dim realized: 1.0000", result.stdout)
        self.assertIn("V1244 POSITION vs north_star (0.98): 86.17% reached", result.stdout)

    def test_v1244_json_cli(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift", "--json"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            cwd=ROOT,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        artifact = json.loads(result.stdout)
        self.assertIn("v1244_metrics", artifact)

    def test_v1244_report_cli(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift", "--report"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            cwd=ROOT,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        self.assertIn("V1244 ASI V0.6.54 hierurgy_substrate_real_lift", result.stdout)
        self.assertIn("V1236", result.stdout)
        self.assertIn("V1243", result.stdout)

    def test_v1244_full_cli(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift", "--full"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            cwd=ROOT,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        # --full = report + JSON artifact
        self.assertIn("V1244 ASI V0.6.54", result.stdout)
        self.assertIn("Realized Metrics", result.stdout)
        self.assertIn("JSON artifact", result.stdout)


class V1244CrossBaselineTest(unittest.TestCase):
    """V1236-V1244 baseline 写死 — 跨 module 整合."""

    def test_v1244_continues_v1243(self):
        # V1244 realized 264 = V1243 258 + 6 (HIERURGY 6 pathway)
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        self.assertAlmostEqual(m.realized_mean_264 - 0.0055, m.v1243_realized_mean_258, places=4)

    def test_v1244_lift_consistent(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        expected_lift = m.realized_mean_264 - m.v1243_realized_mean_258
        self.assertAlmostEqual(m.hierurgy_lift_from_v1243, expected_lift, places=6)

    def test_v1244_overall_lift_consistent(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            _v1244_compute_metrics,
        )
        m = _v1244_compute_metrics()
        expected_lift = m.overall_mean_481 - m.v1243_overall_mean_468
        self.assertAlmostEqual(m.overall_lift_from_v1243, expected_lift, places=6)


class V1244DistinctTest(unittest.TestCase):
    """V1244 hierurgy distinct from liturgy/icon/theosis."""

    def test_v1244_hierurgy_has_pseudo_dionysius(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        # V1244 HIERURGY_PHILOSOPHY must include Pseudo-Dionysius
        pathway = V1244_HIERURGY_SUBSTRATE["HIERURGY_PHILOSOPHY"]
        molecules = " ".join(pathway["cascade_order"])
        self.assertIn("Pseudo_Dionysius", molecules)
        self.assertIn("Maximus", molecules)
        self.assertIn("Palamas", molecules)

    def test_v1244_hierurgy_distinct_liturgy_molecules(self):
        from apeireth.v1244_asi_v0654_hierurgy_substrate_real_lift import (
            V1244_HIERURGY_SUBSTRATE,
        )
        # hierurgy = sacred-work (内在 神秘), liturgy = public-work (外在 时间) — different molecules
        # Just verify hierurgy has Pseudo-Dionysius (sacred-work reference)
        pathway = V1244_HIERURGY_SUBSTRATE["HIERURGY_PHILOSOPHY"]
        self.assertIn("圣仪", pathway["description"])


if __name__ == "__main__":
    unittest.main()