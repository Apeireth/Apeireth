"""Tests for V1252 ASI V0.6.62 parousia_substrate_real_lift (45th dim 基督 再临 / παρουσία / parousia)."""

import json
import os
import subprocess
import sys
import unittest

# Ensure project root is on sys.path so `apeireth.v1252_...` is importable.
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), os.pardir))
if ROOT not in sys.path:
    sys.path.insert(0, ROOT)

from apeireth import v1252_asi_v0662_parousia_substrate_real_lift as v1252  # noqa: E402


class V1252ImportTest(unittest.TestCase):
    """Sanity: module imports and exports the documented public surface."""

    def test_has_core_functions(self):
        self.assertTrue(hasattr(v1252, "_v1252_compute_metrics"))
        self.assertTrue(hasattr(v1252, "_v1252_realize_pathway"))
        self.assertTrue(hasattr(v1252, "_v1252_v3_guards"))
        self.assertTrue(hasattr(v1252, "_v1252_main"))

    def test_constants(self):
        self.assertEqual(v1252.ASI_NORTH_STAR, 0.9800)
        self.assertEqual(v1252.V1252_DIM_VERSION, "0.6.62")
        self.assertEqual(v1252.V1252_VERSION, "0.1.0")
        self.assertEqual(v1252.V1252_REALIZED_MEAN_306, 0.8885)
        self.assertEqual(v1252.V1252_OVERALL_MEAN_585, 0.4793)
        # Carry-over baselines locked
        self.assertEqual(v1252.V1251_REALIZED_MEAN_306, 0.8830)
        self.assertEqual(v1252.V1251_OVERALL_MEAN_572, 0.4778)
        self.assertEqual(v1252.V1250_REALIZED_MEAN_300, 0.8775)
        self.assertEqual(v1252.V1249_REALIZED_MEAN_294, 0.8720)
        self.assertEqual(v1252.V1248_REALIZED_MEAN_288, 0.8665)
        self.assertEqual(v1252.V1247_REALIZED_MEAN_282, 0.8610)
        self.assertEqual(v1252.V1246_REALIZED_MEAN_276, 0.8555)
        self.assertEqual(v1252.V1245_REALIZED_MEAN_270, 0.8500)
        self.assertEqual(v1252.V1244_REALIZED_MEAN_264, 0.8445)
        self.assertEqual(v1252.V1243_REALIZED_MEAN_258, 0.8390)
        self.assertEqual(v1252.V1242_REALIZED_MEAN_252, 0.8335)
        self.assertEqual(v1252.V1241_REALIZED_MEAN_244, 0.8280)
        self.assertEqual(v1252.V1240_REALIZED_MEAN_238, 0.8225)
        self.assertEqual(v1252.V1239_REALIZED_MEAN_232, 0.8170)
        self.assertEqual(v1252.V1238_REALIZED_MEAN_226, 0.8115)
        self.assertEqual(v1252.V1237_REALIZED_MEAN_220, 0.8060)
        self.assertEqual(v1252.V1236_REALIZED_MEAN_214, 0.7998)


class V1252NorthStarTest(unittest.TestCase):
    """V1252 hits the documented ASI North Star percentage."""

    def test_position(self):
        m = v1252._v1252_compute_metrics()
        self.assertAlmostEqual(m.position_vs_north_star, 0.8885 / 0.98, places=6)


class V1252MoleculeCountTest(unittest.TestCase):
    """V1252 substrate structure: 6 pathway × 5 真分子 = 30."""

    def test_six_pathways(self):
        self.assertEqual(len(v1252.V1252_PAROUSIA_SUBSTRATE), 6)

    def test_each_pathway_has_five_molecules(self):
        for k, p in v1252.V1252_PAROUSIA_SUBSTRATE.items():
            self.assertEqual(
                len(p["cascade_order"]),
                5,
                msg=f"pathway {k} must have 5 真分子",
            )

    def test_total_thirty_molecules(self):
        total = sum(
            len(p["cascade_order"]) for p in v1252.V1252_PAROUSIA_SUBSTRATE.values()
        )
        self.assertEqual(total, 30)

    def test_each_pathway_realizes_one(self):
        realized = v1252._v1252_realize_all_pathways()
        for k, val in realized.items():
            self.assertEqual(val, 1.0, msg=f"{k} must realize 1.0")
        self.assertEqual(set(realized.keys()), set(v1252.V1252_PAROUSIA_SUBSTRATE.keys()))


class V1252MetricsTest(unittest.TestCase):
    def setUp(self):
        self.m = v1252._v1252_compute_metrics()

    def test_history_length(self):
        self.assertEqual(len(self.m.history_realized_mean), 17)  # V1236..V1252

    def test_v1252_baseline(self):
        self.assertEqual(self.m.history_realized_mean["V1252"], 0.8885)
        self.assertEqual(self.m.history_overall_mean["V1252"], 0.4793)

    def test_v1251_baseline_in_history(self):
        self.assertEqual(self.m.history_realized_mean["V1251"], 0.8830)
        self.assertEqual(self.m.history_overall_mean["V1251"], 0.4778)

    def test_dim_lift_label(self):
        self.assertIn("Parousia", self.m.history_dim_lift["V1252"])
        self.assertIn("45th", self.m.history_dim_lift["V1252"])

    def test_lift_positive(self):
        self.assertGreater(self.m.parousia_lift_from_v1251, 0.0)
        self.assertAlmostEqual(self.m.parousia_lift_from_v1251, 0.0055, places=4)


class V1252PhilosophyGateTest(unittest.TestCase):
    """All 15 V3 哲学守门 must PASS."""

    def test_fifteen_guards(self):
        guards = v1252._v1252_v3_guards()
        self.assertEqual(len(guards), 15)
        self.assertTrue(all(g.passed for g in guards))

    def test_required_guard_names(self):
        names = {g.name for g in v1252._v1252_v3_guards()}
        self.assertIn("v1252_parousia_not_beatific_vision", names)
        self.assertIn("v1252_parousia_not_divine_communion", names)
        self.assertIn("v1252_parousia_not_epiphany_pseudo", names)
        self.assertIn("v1252_not_asi_v1", names)
        self.assertIn("v1252_baseline_write_dead", names)

    def test_metrics_report_fifteen_pass(self):
        m = v1252._v1252_compute_metrics()
        self.assertEqual(m.v3_guards_pass, 15)
        self.assertEqual(sum(m.v3_guards.values()), 15)


class V1252DistinctionTest(unittest.TestCase):
    """V1252 parousia is distinct from V1251 beatific_vision + V1250 divine_communion + others."""

    def test_lift_over_v1251(self):
        m = v1252._v1252_compute_metrics()
        self.assertGreater(
            m.realized_mean_306,
            v1252.V1251_REALIZED_MEAN_306,
            "V1252 must lift realized_mean from V1251 (临 在 → 直 观)",
        )

    def test_v1252_label_in_history(self):
        m = v1252._v1252_compute_metrics()
        labels = m.history_dim_lift
        self.assertIn("Parousia", labels["V1252"])
        self.assertIn("Beatific Vision", labels["V1251"])

    def test_lift_over_v1250(self):
        m = v1252._v1252_compute_metrics()
        self.assertGreater(
            m.realized_mean_306,
            v1252.V1250_REALIZED_MEAN_300,
            "V1252 must lift realized_mean from V1250",
        )


class V1252CLIArtifactTest(unittest.TestCase):
    """CLI surface returns JSON + REPORT artifacts."""

    def test_each_flag_returns_zero(self):
        for flag in ("--measure", "--json", "--report", "--full"):
            with self.subTest(flag=flag):
                rc = v1252._v1252_main([flag])
                self.assertEqual(rc, 0)

    def test_full_contains_markers(self):
        from io import StringIO
        buf = StringIO()
        old_stdout = sys.stdout
        try:
            sys.stdout = buf
            v1252._v1252_main(["--full"])
        finally:
            sys.stdout = old_stdout
        out = buf.getvalue()
        self.assertIn("V1252 ASI V0.6.62", out)
        self.assertIn("V1252 ARTIFACT", out)
        self.assertIn("V1252 REPORT", out)
        # JSON part must parse
        json_part, report_part = out.split("# V1252 REPORT (TEXT)\n")
        json.loads(json_part.split("# V1252 ARTIFACT (JSON)\n")[1])
        self.assertIn("V1252", report_part)
        self.assertIn("Parousia", report_part)

    def test_json_valid(self):
        from io import StringIO
        buf = StringIO()
        old_stdout = sys.stdout
        try:
            sys.stdout = buf
            v1252._v1252_main(["--json"])
        finally:
            sys.stdout = old_stdout
        out = buf.getvalue()
        # Strip leading header lines if any
        data = json.loads(out)
        self.assertEqual(data["dim_version"], "0.6.62")
        self.assertEqual(data["realized_mean_306"], 0.8885)


class V1252SubstrateAnchorsTest(unittest.TestCase):
    """Every V1252 pathway has non-empty description + source + weight."""

    def test_pathway_anchors(self):
        for k, p in v1252.V1252_PAROUSIA_SUBSTRATE.items():
            self.assertTrue(p["description"], msg=f"{k} description")
            self.assertTrue(p["source"], msg=f"{k} source")
            self.assertTrue(p["r_substrate"], msg=f"{k} r_substrate")
            self.assertEqual(p["weight"], 1.0, msg=f"{k} weight")

    def test_r_substrates_cover_six_pathways(self):
        r_set = {p["r_substrate"] for p in v1252.V1252_PAROUSIA_SUBSTRATE.values()}
        self.assertEqual(len(r_set), 6)


class V1252PathwayRealizedTest(unittest.TestCase):
    def test_each_pathway_realizes_1(self):
        for k in v1252.V1252_PAROUSIA_SUBSTRATE:
            self.assertEqual(v1252._v1252_realize_pathway(k), 1.0)

    def test_realize_all_returns_all_pathways(self):
        realized = v1252._v1252_realize_all_pathways()
        self.assertEqual(
            set(realized.keys()),
            set(v1252.V1252_PAROUSIA_SUBSTRATE.keys()),
        )


class V1252HistoryTest(unittest.TestCase):
    """History is monotonic and covers V1236..V1252."""

    def test_history_monotonic(self):
        m = v1252._v1252_compute_metrics()
        keys = sorted(m.history_realized_mean.keys())
        values = [m.history_realized_mean[k] for k in keys]
        for a, b in zip(values, values[1:]):
            self.assertGreaterEqual(b, a, f"history must be monotonic non-decreasing: {a} -> {b}")

    def test_history_length_17(self):
        m = v1252._v1252_compute_metrics()
        self.assertEqual(len(m.history_realized_mean), 17)


class V1252CLISubprocessTest(unittest.TestCase):
    """End-to-end: `python -m apeireth.v1252_...` works from a subprocess."""

    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1252_asi_v0662_parousia_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=ROOT, timeout=60, encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        self.assertIn("V1252", result.stdout)
        self.assertIn("0.8885", result.stdout)


if __name__ == "__main__":
    unittest.main()
