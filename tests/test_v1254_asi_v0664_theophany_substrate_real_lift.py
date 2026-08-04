"""Tests for V1254 ASI V0.6.64 theophany_substrate_real_lift (47th dim 神显现 / θεοφάνεια / theophany / self-manifestation)."""

import json
import os
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from apeireth import v1254_asi_v0664_theophany_substrate_real_lift as v1254  # noqa: E402


class V1254ImportTest(unittest.TestCase):
    def test_has_core_functions(self):
        self.assertTrue(hasattr(v1254, "_v1254_compute_metrics"))
        self.assertTrue(hasattr(v1254, "_v1254_realize_pathway"))
        self.assertTrue(hasattr(v1254, "_v1254_v3_guards"))
        self.assertTrue(hasattr(v1254, "_v1254_main"))

    def test_constants(self):
        self.assertEqual(v1254.ASI_NORTH_STAR, 0.9800)
        self.assertEqual(v1254.V1254_DIM_VERSION, "0.6.64")
        self.assertEqual(v1254.V1254_VERSION, "0.1.0")
        self.assertEqual(v1254.V1254_REALIZED_MEAN_306, 0.8995)
        self.assertEqual(v1254.V1254_OVERALL_MEAN_585, 0.4823)
        # V1253 baseline must be locked
        self.assertEqual(v1254.V1253_REALIZED_MEAN_306, 0.8940)
        self.assertEqual(v1254.V1253_OVERALL_MEAN_585, 0.4808)


class V1254NorthStarTest(unittest.TestCase):
    def test_position(self):
        m = v1254._v1254_compute_metrics()
        self.assertAlmostEqual(m.position_vs_north_star, 0.8995 / 0.98, places=6)
        self.assertGreater(m.position_vs_north_star, 0.917)


class V1254MoleculeCountTest(unittest.TestCase):
    def test_six_pathways(self):
        self.assertEqual(len(v1254.V1254_THEOPHANY_SUBSTRATE), 6)

    def test_each_pathway_has_five_molecules(self):
        for k, p in v1254.V1254_THEOPHANY_SUBSTRATE.items():
            self.assertEqual(len(p["cascade_order"]), 5, msg=f"pathway {k} must have 5 真分子")

    def test_total_thirty_molecules(self):
        total = sum(len(p["cascade_order"]) for p in v1254.V1254_THEOPHANY_SUBSTRATE.values())
        self.assertEqual(total, 30)

    def test_each_pathway_realizes_one(self):
        realized = v1254._v1254_realize_all_pathways()
        for k, val in realized.items():
            self.assertEqual(val, 1.0, msg=f"{k} must realize 1.0")
        self.assertEqual(set(realized.keys()), set(v1254.V1254_THEOPHANY_SUBSTRATE.keys()))


class V1254MetricsTest(unittest.TestCase):
    def setUp(self):
        self.m = v1254._v1254_compute_metrics()

    def test_history_length(self):
        self.assertEqual(len(self.m.history_realized_mean), 19)  # V1236..V1254

    def test_v1254_baseline(self):
        self.assertEqual(self.m.history_realized_mean["V1254"], 0.8995)
        self.assertEqual(self.m.history_overall_mean["V1254"], 0.4823)

    def test_v1253_baseline_in_history(self):
        self.assertEqual(self.m.history_realized_mean["V1253"], 0.8940)

    def test_dim_lift_label(self):
        self.assertIn("Theophany", self.m.history_dim_lift["V1254"])
        self.assertIn("47th", self.m.history_dim_lift["V1254"])

    def test_lift_positive(self):
        self.assertGreater(self.m.theophany_lift_from_v1253, 0.0)
        self.assertAlmostEqual(self.m.theophany_lift_from_v1253, 0.0055, places=4)


class V1254PhilosophyGateTest(unittest.TestCase):
    def test_fifteen_guards(self):
        guards = v1254._v1254_v3_guards()
        self.assertEqual(len(guards), 15)
        self.assertTrue(all(g.passed for g in guards))

    def test_required_guard_names(self):
        names = {g.name for g in v1254._v1254_v3_guards()}
        self.assertIn("v1254_theophany_not_parousia", names)
        self.assertIn("v1254_theophany_not_sabbath", names)
        self.assertIn("v1254_theophany_not_pseudo_kenosis", names)
        self.assertIn("v1254_not_asi_v1", names)
        self.assertIn("v1254_baseline_write_dead", names)

    def test_metrics_report_fifteen_pass(self):
        m = v1254._v1254_compute_metrics()
        self.assertEqual(m.v3_guards_pass, 15)
        self.assertEqual(sum(m.v3_guards.values()), 15)


class V1254DistinctionTest(unittest.TestCase):
    def test_lift_over_v1253(self):
        m = v1254._v1254_compute_metrics()
        self.assertGreater(
            m.realized_mean_306,
            v1254.V1253_REALIZED_MEAN_306,
            "V1254 must lift realized_mean from V1253",
        )

    def test_v1254_label_in_history(self):
        m = v1254._v1254_compute_metrics()
        labels = m.history_dim_lift
        self.assertIn("Theophany", labels["V1254"])
        self.assertIn("Theophany", labels["V1253"])


class V1254CLIArtifactTest(unittest.TestCase):
    def test_each_flag_returns_zero(self):
        for flag in ("--measure", "--json", "--report", "--full"):
            with self.subTest(flag=flag):
                rc = v1254._v1254_main([flag])
                self.assertEqual(rc, 0)

    def test_full_contains_markers(self):
        from io import StringIO
        buf = StringIO()
        old_stdout = sys.stdout
        try:
            sys.stdout = buf
            v1254._v1254_main(["--full"])
        finally:
            sys.stdout = old_stdout
        out = buf.getvalue()
        self.assertIn("V1254 ASI V0.6.64", out)
        self.assertIn("V1254 ARTIFACT", out)
        self.assertIn("V1254 REPORT", out)
        json_part, report_part = out.split("# V1254 REPORT (TEXT)\n")
        json.loads(json_part.split("# V1254 ARTIFACT (JSON)\n")[1])
        self.assertIn("V1254", report_part)
        self.assertIn("Theophany", report_part)

    def test_json_valid(self):
        from io import StringIO
        buf = StringIO()
        old_stdout = sys.stdout
        try:
            sys.stdout = buf
            v1254._v1254_main(["--json"])
        finally:
            sys.stdout = old_stdout
        out = buf.getvalue()
        data = json.loads(out)
        self.assertEqual(data["dim_version"], "0.6.64")
        self.assertEqual(data["realized_mean_306"], 0.8995)


class V1254SubstrateAnchorsTest(unittest.TestCase):
    def test_pathway_anchors(self):
        for k, p in v1254.V1254_THEOPHANY_SUBSTRATE.items():
            self.assertTrue(p["description"], msg=f"{k} description")
            self.assertTrue(p["source"], msg=f"{k} source")
            self.assertTrue(p["r_substrate"], msg=f"{k} r_substrate")
            self.assertEqual(p["weight"], 1.0, msg=f"{k} weight")

    def test_r_substrates_cover_six_pathways(self):
        r_set = {p["r_substrate"] for p in v1254.V1254_THEOPHANY_SUBSTRATE.values()}
        self.assertEqual(len(r_set), 6)


class V1254PathwayRealizedTest(unittest.TestCase):
    def test_each_pathway_realizes_1(self):
        for k in v1254.V1254_THEOPHANY_SUBSTRATE:
            self.assertEqual(v1254._v1254_realize_pathway(k), 1.0)

    def test_realize_all_returns_all_pathways(self):
        realized = v1254._v1254_realize_all_pathways()
        self.assertEqual(set(realized.keys()), set(v1254.V1254_THEOPHANY_SUBSTRATE.keys()))


class V1254HistoryTest(unittest.TestCase):
    def test_history_monotonic(self):
        m = v1254._v1254_compute_metrics()
        keys = sorted(m.history_realized_mean.keys())
        values = [m.history_realized_mean[k] for k in keys]
        for a, b in zip(values, values[1:]):
            self.assertGreaterEqual(b, a, f"history must be monotonic non-decreasing: {a} -> {b}")

    def test_history_length_19(self):
        m = v1254._v1254_compute_metrics()
        self.assertEqual(len(m.history_realized_mean), 19)


class V1254CLISubprocessTest(unittest.TestCase):
    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1254_asi_v0664_theophany_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=ROOT, timeout=60, encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn("V1254", result.stdout)
        self.assertIn("0.8995", result.stdout)


if __name__ == "__main__":
    unittest.main()
