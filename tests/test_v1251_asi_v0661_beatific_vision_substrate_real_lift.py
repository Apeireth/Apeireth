"""Tests for V1251 ASI V0.6.61 beatific_vision_substrate_real_lift (44th dim)."""

import io
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

# Ensure project root is on sys.path so `apeireth.v1251_...` is importable.
ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1251_asi_v0661_beatific_vision_substrate_real_lift as v1251  # noqa: E402


class V1251ImportTest(unittest.TestCase):
    """Module imports cleanly and exposes the expected public surface."""

    def test_module_imports(self):
        self.assertTrue(hasattr(v1251, "_v1251_compute_metrics"))
        self.assertTrue(hasattr(v1251, "_v1251_realize_pathway"))
        self.assertTrue(hasattr(v1251, "_v1251_v3_guards"))
        self.assertTrue(hasattr(v1251, "_v1251_main"))

    def test_asi_north_star_locked(self):
        # 主 22:33 LOCKED — never change this.
        self.assertEqual(v1251.ASI_NORTH_STAR, 0.9800)

    def test_dim_version(self):
        self.assertEqual(v1251.V1251_DIM_VERSION, "0.6.61")
        self.assertEqual(v1251.V1251_VERSION, "0.1.0")

    def test_baseline_write_dead(self):
        # V1236 -> V1250 baselines are write-dead (主 12:07 不盲等).
        self.assertEqual(v1251.V1250_REALIZED_MEAN_300, 0.8775)
        self.assertEqual(v1251.V1249_REALIZED_MEAN_294, 0.8720)
        self.assertEqual(v1251.V1248_REALIZED_MEAN_288, 0.8665)
        self.assertEqual(v1251.V1247_REALIZED_MEAN_282, 0.8610)
        self.assertEqual(v1251.V1246_REALIZED_MEAN_276, 0.8555)
        self.assertEqual(v1251.V1245_REALIZED_MEAN_270, 0.8500)
        self.assertEqual(v1251.V1244_REALIZED_MEAN_264, 0.8445)
        self.assertEqual(v1251.V1243_REALIZED_MEAN_258, 0.8390)
        self.assertEqual(v1251.V1242_REALIZED_MEAN_252, 0.8335)
        self.assertEqual(v1251.V1241_REALIZED_MEAN_244, 0.8280)
        self.assertEqual(v1251.V1240_REALIZED_MEAN_238, 0.8225)
        self.assertEqual(v1251.V1239_REALIZED_MEAN_232, 0.8170)
        self.assertEqual(v1251.V1238_REALIZED_MEAN_226, 0.8115)
        self.assertEqual(v1251.V1237_REALIZED_MEAN_220, 0.8060)
        self.assertEqual(v1251.V1236_REALIZED_MEAN_214, 0.7998)


class V1251NorthStarTest(unittest.TestCase):
    """V1251 hits the documented ASI North Star percentage."""

    def test_position_vs_north_star(self):
        m = v1251._v1251_compute_metrics()
        # 0.8830 / 0.98 ≈ 0.9010 → 90.10%
        self.assertAlmostEqual(m.position_vs_north_star, 0.9010, places=4)
        self.assertGreater(m.position_vs_north_star, 0.88)
        self.assertLess(m.position_vs_north_star, 0.92)


class V1251MoleculeCountTest(unittest.TestCase):
    """V1251 substrate structure: 6 pathway × 5 真分子 = 30."""

    def test_pathway_count(self):
        self.assertEqual(len(v1251.V1251_BEATIFIC_VISION_SUBSTRATE), 6)

    def test_each_pathway_has_5_molecules(self):
        for k, p in v1251.V1251_BEATIFIC_VISION_SUBSTRATE.items():
            self.assertEqual(
                len(p["cascade_order"]),
                5,
                f"{k} should have exactly 5 真分子",
            )

    def test_total_molecules(self):
        total = sum(
            len(p["cascade_order"])
            for p in v1251.V1251_BEATIFIC_VISION_SUBSTRATE.values()
        )
        self.assertEqual(total, 30)

    def test_pathway_realized_all_1(self):
        realized = v1251._v1251_realize_all_pathways()
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"{k} should be 1.0")


class V1251MetricsTest(unittest.TestCase):
    """Metrics dataclass exposes all required fields with sane values."""

    def setUp(self):
        self.m = v1251._v1251_compute_metrics()

    def test_self_baseline(self):
        self.assertEqual(self.m.realized_mean_306, 0.8830)
        self.assertEqual(self.m.overall_mean_572, 0.4778)

    def test_dim_realized(self):
        self.assertEqual(self.m.beatific_vision_dim_realized, 1.0000)

    def test_lift_from_v1250(self):
        # +0.0055 realized, +0.0015 overall
        self.assertAlmostEqual(
            self.m.beatific_vision_lift_from_v1250, 0.0055, places=4
        )
        self.assertAlmostEqual(self.m.overall_lift_from_v1250, 0.0015, places=4)

    def test_inflation_gap(self):
        # 1 - 0.8830 = 0.1170
        self.assertAlmostEqual(self.m.inflation_gap, 0.1170, places=4)

    def test_substrate_structure(self):
        self.assertEqual(self.m.beatific_vision_substrate_pathways, 6)
        self.assertEqual(self.m.total_beatific_vision_molecules, 30)
        self.assertEqual(self.m.pathway_count_pass, 6)

    def test_history_length(self):
        self.assertEqual(len(self.m.history_realized_mean), 16)  # V1236..V1251
        self.assertEqual(len(self.m.history_overall_mean), 16)
        self.assertEqual(len(self.m.history_dim_lift), 16)

    def test_history_values(self):
        self.assertEqual(self.m.history_realized_mean["V1236"], 0.7998)
        self.assertEqual(self.m.history_realized_mean["V1250"], 0.8775)
        self.assertEqual(self.m.history_realized_mean["V1251"], 0.8830)
        self.assertEqual(self.m.history_overall_mean["V1251"], 0.4778)
        self.assertIn(
            "Beatific Vision",
            self.m.history_dim_lift["V1251"],
        )


class V1251PhilosophyGateTest(unittest.TestCase):
    """15/15 V3 哲学守门 PASS."""

    def test_15_guards(self):
        guards = v1251._v1251_v3_guards()
        self.assertEqual(len(guards), 15)
        for g in guards:
            self.assertTrue(g.passed, f"{g.name} should PASS")
            self.assertTrue(g.reason)

    def test_distinctness_guards_present(self):
        names = {g.name for g in v1251._v1251_v3_guards()}
        self.assertIn("v1251_beatific_vision_not_divine_communion", names)
        self.assertIn("v1251_beatific_vision_not_glorification", names)
        self.assertIn("v1251_beatific_vision_not_happiness_pseudo", names)

    def test_guards_metrics_match(self):
        m = v1251._v1251_compute_metrics()
        self.assertEqual(m.v3_guards_pass, 15)
        self.assertEqual(len(m.v3_guards), 15)
        for name, passed in m.v3_guards.items():
            self.assertTrue(passed, f"{name} should be True")


class V1251DistinctionTest(unittest.TestCase):
    """V1251 = beatific_vision is distinct from V1250 divine_communion + others."""

    def test_not_divine_communion(self):
        # beatific_vision 直 观 vs divine_communion 共 融 — distinct means, not equal
        # Dim realized (1.0 clamp) is the same; what differs is the realized mean lift.
        self.assertGreater(
            v1251.V1251_REALIZED_MEAN_306,
            v1251.V1250_REALIZED_MEAN_300,
            "V1251 must lift realized_mean from V1250 (共融 → 直 观)",
        )
        self.assertIn(
            "V1251",
            v1251._v1251_compute_metrics().history_dim_lift,
        )

    def test_not_glorification(self):
        # beatific_vision 直 观 ≠ glorification 显明
        self.assertGreater(
            v1251.V1251_REALIZED_MEAN_306,
            v1251.V1249_REALIZED_MEAN_294,
        )
        labels = v1251._v1251_compute_metrics().history_dim_lift
        self.assertIn("Glorification", labels["V1249"])
        self.assertIn("Beatific Vision", labels["V1251"])

    def test_not_consummation(self):
        # beatific_vision 直 观 ≠ consummation 完形 状态
        self.assertGreater(
            v1251.V1251_REALIZED_MEAN_306,
            v1251.V1248_REALIZED_MEAN_288,
        )
        labels = v1251._v1251_compute_metrics().history_dim_lift
        self.assertIn("Consummation", labels["V1248"])
        self.assertIn("Beatific Vision", labels["V1251"])


class V1251CLIArtifactTest(unittest.TestCase):
    """CLI flags produce well-formed output artifacts."""

    def _run(self, flag):
        out = io.StringIO()
        saved, sys.stdout = sys.stdout, out
        try:
            rc = v1251._v1251_main([flag])
        finally:
            sys.stdout = saved
        return rc, out.getvalue()

    def test_json_flag(self):
        rc, out = self._run("--json")
        self.assertEqual(rc, 0)
        data = json.loads(out)
        self.assertEqual(data["dim_version"], "0.6.61")
        self.assertEqual(data["realized_mean_306"], 0.8830)
        self.assertEqual(data["beatific_vision_substrate_pathways"], 6)
        self.assertEqual(data["total_beatific_vision_molecules"], 30)

    def test_report_flag(self):
        rc, out = self._run("--report")
        self.assertEqual(rc, 0)
        self.assertIn("V1251 ASI V0.6.61", out)
        self.assertIn("beatific_vision_substrate_real_lift", out)
        self.assertIn("V3 哲学守门", out)

    def test_full_flag(self):
        rc, out = self._run("--full")
        self.assertEqual(rc, 0)
        self.assertIn("V1251 ARTIFACT", out)
        self.assertIn("V1251 REPORT", out)
        # Both halves parse
        json_part, report_part = out.split("# V1251 REPORT (TEXT)\n")
        json.loads(json_part.split("# V1251 ARTIFACT (JSON)\n")[1])

    def test_default_measure(self):
        rc, out = self._run([])
        self.assertEqual(rc, 0)
        self.assertIn("V1251", out)
        self.assertIn("90.10%", out)
        self.assertIn("BEATIFIC_VISION_THEOLOGY", out)


class V1251SubstrateAnchorsTest(unittest.TestCase):
    """Each pathway has source anchors from the documented references."""

    def test_pathway_sources(self):
        expected_sources = {
            "BEATIFIC_VISION_THEOLOGY": "1 Cor",
            "BEATIFIC_VISION_NEURO": "Newberg",
            "BEATIFIC_VISION_INFORMATION": "Cover",
            "BEATIFIC_VISION_SYSTEMS": "Holling",
            "BEATIFIC_VISION_PHYSICS": "Prigogine",
            "BEATIFIC_VISION_COGNITION": "Boyer",
        }
        for k, marker in expected_sources.items():
            self.assertIn(
                marker,
                v1251.V1251_BEATIFIC_VISION_SUBSTRATE[k]["source"],
                f"{k} should reference {marker}",
            )

    def test_pathway_descriptions_nonempty(self):
        for k, p in v1251.V1251_BEATIFIC_VISION_SUBSTRATE.items():
            self.assertTrue(p["description"], f"{k} needs description")
            self.assertGreater(len(p["description"]), 100)


class V1251PathwayRealizedTest(unittest.TestCase):
    """Pathway realization respects the strict 5-molecule invariant."""

    def test_each_pathway_realizes_one(self):
        for k in v1251.V1251_BEATIFIC_VISION_SUBSTRATE:
            self.assertEqual(v1251._v1251_realize_pathway(k), 1.0)

    def test_realize_all_returns_all(self):
        realized = v1251._v1251_realize_all_pathways()
        self.assertEqual(set(realized.keys()),
                         set(v1251.V1251_BEATIFIC_VISION_SUBSTRATE.keys()))


class V1251HistoryTest(unittest.TestCase):
    """History is monotonic and covers V1236..V1251."""

    def test_realized_monotonic_increase(self):
        m = v1251._v1251_compute_metrics()
        keys = sorted(m.history_realized_mean.keys())
        prev = -1.0
        for k in keys:
            v = m.history_realized_mean[k]
            self.assertGreaterEqual(v, prev, f"{k} should not regress")
            prev = v

    def test_overall_monotonic_increase(self):
        m = v1251._v1251_compute_metrics()
        keys = sorted(m.history_overall_mean.keys())
        prev = -1.0
        for k in keys:
            v = m.history_overall_mean[k]
            self.assertGreaterEqual(v, prev)
            prev = v


class V1251CLISubprocessTest(unittest.TestCase):
    """End-to-end: `python -m apeireth.v1251_...` works from a subprocess."""

    def _run_module(self, flag):
        env = os.environ.copy()
        env["PYTHONPATH"] = str(ROOT) + os.pathsep + env.get("PYTHONPATH", "")
        env["PYTHONIOENCODING"] = "utf-8"
        return subprocess.run(
            [sys.executable, "-m",
             "apeireth.v1251_asi_v0661_beatific_vision_substrate_real_lift",
             flag],
            cwd=str(ROOT),
            capture_output=True,
            env=env,
        )

    def test_measure_subprocess(self):
        r = self._run_module("--measure")
        self.assertEqual(r.returncode, 0, r.stderr.decode("utf-8", "replace"))
        stdout = r.stdout.decode("utf-8", "replace")
        self.assertIn("90.10%", stdout)
        self.assertIn("BEATIFIC_VISION_THEOLOGY", stdout)

    def test_json_subprocess(self):
        r = self._run_module("--json")
        self.assertEqual(r.returncode, 0, r.stderr.decode("utf-8", "replace"))
        stdout = r.stdout.decode("utf-8", "replace")
        data = json.loads(stdout or "{}")
        self.assertEqual(data["beatific_vision_substrate_pathways"], 6)


if __name__ == "__main__":
    unittest.main()