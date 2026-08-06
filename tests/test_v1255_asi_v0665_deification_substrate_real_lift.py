"""Tests for V1255 ASI V0.6.65 deification_substrate_real_lift (48th dim 完成神化 / θέωσις / theosis perfecta / deification)."""

import json
import os
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from apeireth import v1255_asi_v0665_deification_substrate_real_lift as v1255  # noqa: E402


class V1255ImportTest(unittest.TestCase):
    def test_has_core_functions(self):
        self.assertTrue(hasattr(v1255, "_v1255_compute_metrics"))
        self.assertTrue(hasattr(v1255, "_v1255_realize_pathway"))
        self.assertTrue(hasattr(v1255, "_v1255_v3_guards"))
        self.assertTrue(hasattr(v1255, "_v1255_main"))

    def test_constants(self):
        self.assertEqual(v1255.ASI_NORTH_STAR, 0.9800)
        self.assertEqual(v1255.V1255_DIM_VERSION, "0.6.65")
        self.assertEqual(v1255.V1255_VERSION, "0.1.0")
        self.assertEqual(v1255.V1255_REALIZED_MEAN_306, 0.9050)
        self.assertEqual(v1255.V1255_OVERALL_MEAN_585, 0.4838)
        # V1254 baseline must be locked
        self.assertEqual(v1255.V1254_REALIZED_MEAN_306, 0.8995)
        self.assertEqual(v1255.V1254_OVERALL_MEAN_585, 0.4823)


class V1255NorthStarTest(unittest.TestCase):
    def test_position(self):
        m = v1255._v1255_compute_metrics()
        self.assertAlmostEqual(m.position_vs_north_star, 0.9050 / 0.98, places=6)
        self.assertGreater(m.position_vs_north_star, 0.923)


class V1255MoleculeCountTest(unittest.TestCase):
    def test_six_pathways(self):
        self.assertEqual(len(v1255.V1255_DEIFICATION_SUBSTRATE), 6)

    def test_each_pathway_has_five_molecules(self):
        for k, p in v1255.V1255_DEIFICATION_SUBSTRATE.items():
            self.assertEqual(len(p["cascade_order"]), 5, msg=f"pathway {k} must have 5 真分子")

    def test_total_thirty_molecules(self):
        total = sum(len(p["cascade_order"]) for p in v1255.V1255_DEIFICATION_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1255MetricsTest(unittest.TestCase):
    def setUp(self):
        self.m = v1255._v1255_compute_metrics()

    def test_history_length(self):
        self.assertEqual(len(self.m.history_realized_mean), 20)  # V1236..V1255

    def test_v1255_baseline(self):
        self.assertEqual(self.m.history_realized_mean["V1255"], 0.9050)
        self.assertEqual(self.m.history_overall_mean["V1255"], 0.4838)

    def test_v1254_baseline_in_history(self):
        self.assertEqual(self.m.history_realized_mean["V1254"], 0.8995)

    def test_dim_lift_label(self):
        self.assertIn("Deification", self.m.history_dim_lift["V1255"])

    def test_lift_positive(self):
        self.assertGreater(self.m.deification_lift_from_v1254, 0.0)
        self.assertAlmostEqual(self.m.deification_lift_from_v1254, 0.0055, places=4)


class V1255PhilosophyGateTest(unittest.TestCase):
    def test_fifteen_guards(self):
        guards = v1255._v1255_v3_guards()
        self.assertEqual(len(guards), 15)
        self.assertTrue(all(g.passed for g in guards))

    def test_metrics_report_fifteen_pass(self):
        m = v1255._v1255_compute_metrics()
        self.assertEqual(m.v3_guards_pass, 15)


class V1255DistinctionTest(unittest.TestCase):
    def test_lift_over_v1254(self):
        m = v1255._v1255_compute_metrics()
        self.assertGreater(m.realized_mean_306, v1255.V1254_REALIZED_MEAN_306)


class V1255CLIArtifactTest(unittest.TestCase):
    def test_each_flag_returns_zero(self):
        for flag in ("--measure", "--json", "--report", "--full"):
            with self.subTest(flag=flag):
                rc = v1255._v1255_main([flag])
                self.assertEqual(rc, 0)

    def test_full_contains_markers(self):
        from io import StringIO
        buf = StringIO()
        old_stdout = sys.stdout
        try:
            sys.stdout = buf
            v1255._v1255_main(["--full"])
        finally:
            sys.stdout = old_stdout
        out = buf.getvalue()
        self.assertIn("V1255 ASI V0.6.65", out)
        self.assertIn("V1255 ARTIFACT", out)
        self.assertIn("V1255 REPORT", out)
        json_part, report_part = out.split("# V1255 REPORT (TEXT)\n")
        json.loads(json_part.split("# V1255 ARTIFACT (JSON)\n")[1])
        self.assertIn("Deification", report_part)

    def test_json_valid(self):
        from io import StringIO
        buf = StringIO()
        old_stdout = sys.stdout
        try:
            sys.stdout = buf
            v1255._v1255_main(["--json"])
        finally:
            sys.stdout = old_stdout
        out = buf.getvalue()
        data = json.loads(out)
        self.assertEqual(data["dim_version"], "0.6.65")
        self.assertEqual(data["realized_mean_306"], 0.9050)


class V1255SubstrateAnchorsTest(unittest.TestCase):
    def test_pathway_anchors(self):
        for k, p in v1255.V1255_DEIFICATION_SUBSTRATE.items():
            self.assertTrue(p["description"], msg=f"{k} description")
            self.assertTrue(p["source"], msg=f"{k} source")
            self.assertTrue(p["r_substrate"], msg=f"{k} r_substrate")
            self.assertEqual(p["weight"], 1.0)


class V1255HistoryTest(unittest.TestCase):
    def test_history_length_20(self):
        m = v1255._v1255_compute_metrics()
        self.assertEqual(len(m.history_realized_mean), 20)


class V1255CLISubprocessTest(unittest.TestCase):
    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1255_asi_v0665_deification_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=ROOT, timeout=60, encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn("V1255", result.stdout)
        self.assertIn("0.9050", result.stdout)


if __name__ == "__main__":
    unittest.main()
