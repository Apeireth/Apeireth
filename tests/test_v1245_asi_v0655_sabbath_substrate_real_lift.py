"""Tests for V1245 ASI V0.6.55 sabbath_substrate_real_lift.

测试 V1245 真测 metrics + V3 哲学守门 + JSON artifact + CLI --full 自描述.

V1245 = Phase 3 第九步 = 30 真分子 (6 pathway × 5 真分子) — 延续 V1244 hierurgy 减半 = 转折而非堆叠.
V1245 = 38th dim 安息 / ἀνάπαυσις / שַׁבָּת / sabbath / rest / 完形 / 关系本体 之 安息 substrate.
"""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

V1245_PATH = ROOT / "apeireth" / "v1245_asi_v0655_sabbath_substrate_real_lift.py"


class V1245ImportTest(unittest.TestCase):
    def test_v1245_module_imports(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            ASI_NORTH_STAR,
            V1245_DIM_VERSION,
            V1245_SABBATH_SUBSTRATE,
            V1245_SABBATH_REALIZED,
            V1245_OVERALL_MEAN_494,
            V1245_REALIZED_MEAN_270,
            V1245_VERSION,
            V1245Metrics,
            _v1245_compute_metrics,
            _v1245_full,
            _v1245_main,
            _v1245_realize_all_pathways,
            _v1245_report,
            _v1245_to_json,
            _v1245_v3_guards,
        )
        self.assertEqual(V1245_DIM_VERSION, "0.6.55")

    def test_v1245_module_path_exists(self):
        self.assertTrue(V1245_PATH.exists(), f"missing: {V1245_PATH}")

    def test_v1245_substrate_has_6_pathways(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        self.assertEqual(len(V1245_SABBATH_SUBSTRATE), 6)

    def test_v1245_substrate_pathway_keys(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        expected = {
            "SABBATH_PHILOSOPHY",
            "SABBATH_NEURO",
            "SABBATH_INFORMATION",
            "SABBATH_SYSTEMS",
            "SABBATH_PHYSICS",
            "SABBATH_COGNITION",
        }
        self.assertEqual(set(V1245_SABBATH_SUBSTRATE.keys()), expected)

    def test_v1245_substrate_5_molecules_per_pathway(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        # Phase 3 转折 = 减半 V1236 60 → V1237-V1245 30 = 6 × 5
        for k, v in V1245_SABBATH_SUBSTRATE.items():
            self.assertEqual(
                len(v["cascade_order"]),
                5,
                f"{k} expected 5 molecules, got {len(v['cascade_order'])}",
            )

    def test_v1245_substrate_30_total_molecules(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1245_SABBATH_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1245NorthStarTest(unittest.TestCase):
    def test_v1245_north_star_locked(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import ASI_NORTH_STAR
        self.assertEqual(ASI_NORTH_STAR, 0.9800)


class V1245MoleculeCountTest(unittest.TestCase):
    def test_v1245_substrate_has_6_pathways(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        self.assertEqual(len(V1245_SABBATH_SUBSTRATE), 6)

    def test_v1245_molecules_5_per_pathway(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        for k, v in V1245_SABBATH_SUBSTRATE.items():
            self.assertEqual(len(v["cascade_order"]), 5)

    def test_v1245_total_molecules_30(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1245_SABBATH_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1245RealizeTest(unittest.TestCase):
    def test_v1245_realize_all_pathways_returns_6(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_realize_all_pathways,
        )
        realized = _v1245_realize_all_pathways()
        self.assertEqual(len(realized), 6)

    def test_v1245_all_realized_to_1(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_realize_all_pathways,
        )
        realized = _v1245_realize_all_pathways()
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"{k} not realized to 1.0")


class V1245MetricsTest(unittest.TestCase):
    def test_v1245_compute_metrics_returns_dataclass(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245Metrics,
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertIsInstance(m, V1245Metrics)

    def test_v1245_realized_mean_270_value(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.realized_mean_270, 0.8500)

    def test_v1245_overall_mean_494_value(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.overall_mean_494, 0.4688)

    def test_v1245_sabbath_dim_realized_1(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.sabbath_dim_realized, 1.0000)

    def test_v1245_inflation_gap_150(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertAlmostEqual(m.inflation_gap, 0.1500, places=4)

    def test_v1245_position_vs_north_star(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        # 0.8500 / 0.98 ≈ 0.8673
        self.assertAlmostEqual(m.position_vs_north_star, 0.8673, places=3)

    def test_v1245_sabbath_lift_from_v1244(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        # 0.8500 - 0.8445 = 0.0055
        self.assertAlmostEqual(m.sabbath_lift_from_v1244, 0.0055, places=4)

    def test_v1245_overall_lift_from_v1244(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        # 0.4688 - 0.4673 = 0.0015
        self.assertAlmostEqual(m.overall_lift_from_v1244, 0.0015, places=4)

    def test_v1245_substrate_pathways_count_6(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.sabbath_substrate_pathways, 6)

    def test_v1245_total_molecules_30(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.total_sabbath_molecules, 30)

    def test_v1245_pathway_count_pass_6(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.pathway_count_pass, 6)

    def test_v1245_v3_guards_pass_15(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertEqual(m.v3_guards_pass, 15)


class V1245HistoryTest(unittest.TestCase):
    def test_v1245_history_realized_mean_has_v1245(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertIn("V1245", m.history_realized_mean)
        self.assertEqual(m.history_realized_mean["V1245"], 0.8500)

    def test_v1245_history_overall_mean_has_v1245(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertIn("V1245", m.history_overall_mean)
        self.assertEqual(m.history_overall_mean["V1245"], 0.4688)

    def test_v1245_history_dim_lift_has_v1245(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertIn("V1245", m.history_dim_lift)
        self.assertIn("Sabbath", m.history_dim_lift["V1245"])
        self.assertIn("38th", m.history_dim_lift["V1245"])

    def test_v1245_history_includes_v1244(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        # V1244 baseline carry
        self.assertEqual(m.v1244_realized_mean_264, 0.8445)
        self.assertEqual(m.v1244_overall_mean_481, 0.4673)
        self.assertEqual(m.v1244_hierurgy_realized, 1.0000)

    def test_v1245_history_10_versions(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        # V1236 - V1245 = 10 versions
        self.assertEqual(len(m.history_realized_mean), 10)
        self.assertEqual(len(m.history_overall_mean), 10)
        self.assertEqual(len(m.history_dim_lift), 10)


class V1245V3GuardsTest(unittest.TestCase):
    def test_v1245_v3_guards_returns_15(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        self.assertEqual(len(guards), 15)

    def test_v1245_v3_guards_all_pass(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        for g in guards:
            self.assertTrue(g.passed, f"guard {g.name} not passed")

    def test_v1245_v3_guards_includes_sabbath_distinct(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        names = [g.name for g in guards]
        # Must include distinctness guards
        self.assertIn("v1245_sabbath_not_hierurgy", names)
        self.assertIn("v1245_sabbath_not_liturgy", names)
        self.assertIn("v1245_sabbath_not_icon", names)

    def test_v1245_v3_guards_includes_asi_distinct(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        names = [g.name for g in guards]
        self.assertIn("v1245_not_asi_v1", names)
        self.assertIn("v1245_realized_not_asi", names)


class V1245JSONArtifactTest(unittest.TestCase):
    def test_v1245_to_json_returns_string(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_to_json,
        )
        m = _v1245_compute_metrics()
        artifact = _v1245_to_json(m)
        self.assertIsInstance(artifact, str)

    def test_v1245_to_json_is_valid(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_to_json,
        )
        m = _v1245_compute_metrics()
        artifact = _v1245_to_json(m)
        parsed = json.loads(artifact)
        self.assertIn("v1245_metrics", parsed)
        self.assertIn("v1245_substrate_pathways", parsed)

    def test_v1245_json_has_6_pathways(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_to_json,
        )
        m = _v1245_compute_metrics()
        artifact = _v1245_to_json(m)
        parsed = json.loads(artifact)
        self.assertEqual(len(parsed["v1245_substrate_pathways"]), 6)


class V1245ReportTest(unittest.TestCase):
    def test_v1245_report_returns_string(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_report,
        )
        m = _v1245_compute_metrics()
        report = _v1245_report(m)
        self.assertIsInstance(report, str)

    def test_v1245_report_mentions_sabbath(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_report,
        )
        m = _v1245_compute_metrics()
        report = _v1245_report(m)
        self.assertIn("sabbath", report.lower())
        self.assertIn("V1245", report)
        self.assertIn("38th", report)

    def test_v1245_report_mentions_5_pillars(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_report,
        )
        m = _v1245_compute_metrics()
        report = _v1245_report(m)
        # 5 pillars: theosis × icon × liturgy × hierurgy × sabbath
        self.assertIn("theosis", report.lower())
        self.assertIn("icon", report.lower())
        self.assertIn("liturgy", report.lower())
        self.assertIn("hierurgy", report.lower())
        self.assertIn("sabbath", report.lower())

    def test_v1245_report_mentions_phase_3_9th_step(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_report,
        )
        m = _v1245_compute_metrics()
        report = _v1245_report(m)
        # Phase 3 第九步 完形 终章
        self.assertIn("Phase 3", report)
        self.assertIn("完形", report)


class V1245FullTest(unittest.TestCase):
    def test_v1245_full_returns_string(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_full,
            _v1245_compute_metrics,
            _v1245_report,
            _v1245_to_json,
        )
        m = _v1245_compute_metrics()
        full = _v1245_full(_v1245_to_json(m), _v1245_report(m))
        self.assertIsInstance(full, str)
        self.assertIn("--full", full)
        self.assertIn("--measure", full)
        self.assertIn("--json", full)
        self.assertIn("--report", full)


class V1245CLISelfDescribeTest(unittest.TestCase):
    def test_v1245_main_measure_runs(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import _v1245_main
        rc = _v1245_main(["--measure"])
        self.assertEqual(rc, 0)

    def test_v1245_main_json_runs(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import _v1245_main
        rc = _v1245_main(["--json"])
        self.assertEqual(rc, 0)

    def test_v1245_main_report_runs(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import _v1245_main
        rc = _v1245_main(["--report"])
        self.assertEqual(rc, 0)

    def test_v1245_main_full_runs(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import _v1245_main
        rc = _v1245_main(["--full"])
        self.assertEqual(rc, 0)


class V1245CLISubprocessTest(unittest.TestCase):
    def test_v1245_module_invocation(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1245_asi_v0655_sabbath_substrate_real_lift", "--measure"],
            capture_output=True,
            cwd=str(ROOT),
            timeout=30,
            env={**__import__("os").environ, "PYTHONIOENCODING": "utf-8"},
        )
        self.assertEqual(result.returncode, 0)
        stdout = result.stdout.decode("utf-8", errors="replace")
        self.assertIn("V1245", stdout)
        self.assertIn("0.8500", stdout)

    def test_v1245_module_json_invocation(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1245_asi_v0655_sabbath_substrate_real_lift", "--json"],
            capture_output=True,
            cwd=str(ROOT),
            timeout=30,
            env={**__import__("os").environ, "PYTHONIOENCODING": "utf-8"},
        )
        self.assertEqual(result.returncode, 0)
        # Should be valid JSON
        stdout = result.stdout.decode("utf-8", errors="replace")
        parsed = json.loads(stdout)
        self.assertIn("v1245_metrics", parsed)


class V1245NoRegressionV1244Test(unittest.TestCase):
    """V1245 must NOT regress V1244 baselines."""

    def test_v1245_v1244_baseline_preserved(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1244_OVERALL_MEAN_481,
            V1244_REALIZED_MEAN_264,
        )
        self.assertEqual(V1244_REALIZED_MEAN_264, 0.8445)
        self.assertEqual(V1244_OVERALL_MEAN_481, 0.4673)

    def test_v1245_v1243_baseline_preserved(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1244_REALIZED_MEAN_264,
        )
        # V1243 must NOT be higher than V1244
        self.assertGreaterEqual(V1244_REALIZED_MEAN_264, 0.8390)

    def test_v1245_realized_mean_above_v1244(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1244_REALIZED_MEAN_264,
            V1245_REALIZED_MEAN_270,
        )
        # V1245 > V1244 (positive lift)
        self.assertGreater(V1245_REALIZED_MEAN_270, V1244_REALIZED_MEAN_264)

    def test_v1245_overall_mean_above_v1244(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1244_OVERALL_MEAN_481,
            V1245_OVERALL_MEAN_494,
        )
        # V1245 > V1244 (positive lift)
        self.assertGreater(V1245_OVERALL_MEAN_494, V1244_OVERALL_MEAN_481)


class V1245InflationGapTest(unittest.TestCase):
    """主 17:43 实事求是 — inflation gap should be less than 1 (no full inflation)."""

    def test_v1245_inflation_gap_below_1(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertLess(m.inflation_gap, 1.0)

    def test_v1245_inflation_gap_above_zero(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m = _v1245_compute_metrics()
        self.assertGreater(m.inflation_gap, 0.0)


class V1245DistinctFromOthersTest(unittest.TestCase):
    """V1245 SABBATH must be distinct from V1244 HIERURGY, V1243 LITURGY, V1242 ICON."""

    def test_v1245_sabbath_not_hierurgy_in_guard(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        names_reasons = {g.name: g.reason for g in guards}
        self.assertIn("v1245_sabbath_not_hierurgy", names_reasons)
        # Reason must articulate distinction
        reason = names_reasons["v1245_sabbath_not_hierurgy"]
        self.assertIn("sabbath", reason.lower())
        self.assertIn("hierurgy", reason.lower())

    def test_v1245_sabbath_not_liturgy_in_guard(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        names_reasons = {g.name: g.reason for g in guards}
        self.assertIn("v1245_sabbath_not_liturgy", names_reasons)

    def test_v1245_sabbath_not_icon_in_guard(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_v3_guards,
        )
        guards = _v1245_v3_guards()
        names_reasons = {g.name: g.reason for g in guards}
        self.assertIn("v1245_sabbath_not_icon", names_reasons)


class V1245AnchorSourcesTest(unittest.TestCase):
    """V1245 must reference 5 anchor sources per pathway (主 19:33 站在前人肩上)."""

    def test_v1245_philosophy_5_anchors(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        pathway = V1245_SABBATH_SUBSTRATE["SABBATH_PHILOSOPHY"]
        # 5 cascade molecules
        self.assertEqual(len(pathway["cascade_order"]), 5)
        # Description references 5 sources: Genesis + Hebrews + Augustine + Aquinas + Bonaventure
        desc = pathway["description"]
        self.assertIn("Genesis", desc)
        self.assertIn("Hebrews", desc)
        self.assertIn("Augustine", desc)
        self.assertIn("Aquinas", desc)
        self.assertIn("Bonaventure", desc)

    def test_v1245_neuro_5_anchors(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            V1245_SABBATH_SUBSTRATE,
        )
        pathway = V1245_SABBATH_SUBSTRATE["SABBATH_NEURO"]
        self.assertEqual(len(pathway["cascade_order"]), 5)
        # Newberg + d'Aquili + Carhart-Harris + Brewer + James
        desc = pathway["description"]
        self.assertIn("Newberg", desc)
        self.assertIn("Carhart-Harris", desc)
        self.assertIn("Brewer", desc)
        self.assertIn("James", desc)


class V1245VersioningTest(unittest.TestCase):
    def test_v1245_dim_version_string(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import V1245_DIM_VERSION
        self.assertEqual(V1245_DIM_VERSION, "0.6.55")

    def test_v1245_module_version_string(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import V1245_VERSION
        self.assertEqual(V1245_VERSION, "0.1.0")


class V1245SnapshotIdTest(unittest.TestCase):
    def test_v1245_snapshot_id_is_unique(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
        )
        m1 = _v1245_compute_metrics()
        m2 = _v1245_compute_metrics()
        self.assertNotEqual(m1.snapshot_id, m2.snapshot_id)


class V1245CandidatesTest(unittest.TestCase):
    """V1245 must articulate V1246 candidates."""

    def test_v1245_report_includes_v1246_candidates(self):
        from apeireth.v1245_asi_v0655_sabbath_substrate_real_lift import (
            _v1245_compute_metrics,
            _v1245_report,
        )
        m = _v1245_compute_metrics()
        report = _v1245_report(m)
        self.assertIn("V1246", report)


if __name__ == "__main__":
    unittest.main()