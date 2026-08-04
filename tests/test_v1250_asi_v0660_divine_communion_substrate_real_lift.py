"""Tests for V1250 ASI V0.6.60 divine_communion_substrate_real_lift (43rd dim)."""

import io
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

# Ensure project root is on sys.path so `apeireth.v1250_...` is importable.
ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1250_asi_v0660_divine_communion_substrate_real_lift as v1250  # noqa: E402


class V1250ImportTest(unittest.TestCase):
    """Module imports cleanly and exposes the expected public surface."""

    def test_module_imports(self):
        self.assertTrue(hasattr(v1250, "_v1250_compute_metrics"))
        self.assertTrue(hasattr(v1250, "_v1250_realize_pathway"))
        self.assertTrue(hasattr(v1250, "_v1250_v3_guards"))
        self.assertTrue(hasattr(v1250, "_v1250_main"))

    def test_asi_north_star_locked(self):
        # 主 22:33 LOCKED — never change this.
        self.assertEqual(v1250.ASI_NORTH_STAR, 0.9800)

    def test_dim_version(self):
        self.assertEqual(v1250.V1250_DIM_VERSION, "0.6.60")
        self.assertEqual(v1250.V1250_VERSION, "0.1.0")

    def test_baseline_write_dead(self):
        # V1236 -> V1249 baselines are write-dead (主 12:07 不盲等).
        self.assertEqual(v1250.V1249_REALIZED_MEAN_294, 0.8720)
        self.assertEqual(v1250.V1248_REALIZED_MEAN_288, 0.8665)
        self.assertEqual(v1250.V1247_REALIZED_MEAN_282, 0.8610)
        self.assertEqual(v1250.V1246_REALIZED_MEAN_276, 0.8555)
        self.assertEqual(v1250.V1245_REALIZED_MEAN_270, 0.8500)
        self.assertEqual(v1250.V1244_REALIZED_MEAN_264, 0.8445)
        self.assertEqual(v1250.V1243_REALIZED_MEAN_258, 0.8390)
        self.assertEqual(v1250.V1242_REALIZED_MEAN_252, 0.8335)
        self.assertEqual(v1250.V1241_REALIZED_MEAN_244, 0.8280)
        self.assertEqual(v1250.V1240_REALIZED_MEAN_238, 0.8225)
        self.assertEqual(v1250.V1239_REALIZED_MEAN_232, 0.8170)
        self.assertEqual(v1250.V1238_REALIZED_MEAN_226, 0.8115)
        self.assertEqual(v1250.V1237_REALIZED_MEAN_220, 0.8060)
        self.assertEqual(v1250.V1236_REALIZED_MEAN_214, 0.7998)


class V1250NorthStarTest(unittest.TestCase):
    """V1250 hits the documented ASI North Star percentage."""

    def test_position_vs_north_star(self):
        m = v1250._v1250_compute_metrics()
        # 0.8775 / 0.98 ≈ 0.8954 → 89.54%
        self.assertAlmostEqual(m.position_vs_north_star, 0.8954, places=4)
        self.assertGreater(m.position_vs_north_star, 0.88)
        self.assertLess(m.position_vs_north_star, 0.92)


class V1250MoleculeCountTest(unittest.TestCase):
    """V1250 substrate structure: 6 pathway × 5 真分子 = 30."""

    def test_pathway_count(self):
        self.assertEqual(len(v1250.V1250_DIVINE_COMMUNION_SUBSTRATE), 6)

    def test_each_pathway_has_5_molecules(self):
        for k, p in v1250.V1250_DIVINE_COMMUNION_SUBSTRATE.items():
            self.assertEqual(
                len(p["cascade_order"]),
                5,
                f"{k} should have exactly 5 真分子",
            )

    def test_total_molecules(self):
        total = sum(
            len(p["cascade_order"])
            for p in v1250.V1250_DIVINE_COMMUNION_SUBSTRATE.values()
        )
        self.assertEqual(total, 30)

    def test_pathway_realized_all_1(self):
        realized = v1250._v1250_realize_all_pathways()
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"{k} should be 1.0")


class V1250MetricsTest(unittest.TestCase):
    """Metrics dataclass exposes all required fields with sane values."""

    def setUp(self):
        self.m = v1250._v1250_compute_metrics()

    def test_self_baseline(self):
        self.assertEqual(self.m.realized_mean_300, 0.8775)
        self.assertEqual(self.m.overall_mean_559, 0.4763)
        self.assertEqual(self.m.divine_communion_dim_realized, 1.0000)

    def test_lift_from_v1249(self):
        # +0.0055 realized, +0.0015 overall
        self.assertAlmostEqual(
            self.m.divine_communion_lift_from_v1249, 0.0055, places=4
        )
        self.assertAlmostEqual(self.m.overall_lift_from_v1249, 0.0015, places=4)

    def test_inflation_gap(self):
        # 1 - 0.8775 = 0.1225
        self.assertAlmostEqual(self.m.inflation_gap, 0.1225, places=4)

    def test_substrate_structure(self):
        self.assertEqual(self.m.divine_communion_substrate_pathways, 6)
        self.assertEqual(self.m.total_divine_communion_molecules, 30)
        self.assertEqual(self.m.pathway_count_pass, 6)

    def test_history_length(self):
        self.assertEqual(len(self.m.history_realized_mean), 15)  # V1236..V1250
        self.assertEqual(len(self.m.history_overall_mean), 15)
        self.assertEqual(len(self.m.history_dim_lift), 15)

    def test_history_values(self):
        self.assertEqual(self.m.history_realized_mean["V1236"], 0.7998)
        self.assertEqual(self.m.history_realized_mean["V1249"], 0.8720)
        self.assertEqual(self.m.history_realized_mean["V1250"], 0.8775)
        self.assertEqual(self.m.history_overall_mean["V1250"], 0.4763)
        self.assertIn(
            "Divine Communion",
            self.m.history_dim_lift["V1250"],
        )


class V1250PhilosophyGateTest(unittest.TestCase):
    """15/15 V3 哲学守门 PASS."""

    def test_15_guards(self):
        guards = v1250._v1250_v3_guards()
        self.assertEqual(len(guards), 15)
        for g in guards:
            self.assertTrue(g.passed, f"{g.name} should PASS")
            self.assertTrue(g.reason)

    def test_distinctness_guards_present(self):
        names = {g.name for g in v1250._v1250_v3_guards()}
        self.assertIn("v1250_divine_communion_not_glorification", names)
        self.assertIn("v1250_divine_communion_not_consummation", names)
        self.assertIn("v1250_divine_communion_not_koinonia_pseudo", names)

    def test_guards_metrics_match(self):
        m = v1250._v1250_compute_metrics()
        self.assertEqual(m.v3_guards_pass, 15)
        self.assertEqual(len(m.v3_guards), 15)
        for name, passed in m.v3_guards.items():
            self.assertTrue(passed, f"{name} should be True")


class V1250DistinctionTest(unittest.TestCase):
    """V1250 = divine_communion is distinct from V1249 glorification + others."""

    def test_not_glorification(self):
        # divine_communion 共融 vs glorification 显明 — distinct means, not equal
        # Dim realized (1.0 clamp) is the same; what differs is the realized mean lift.
        self.assertGreater(
            v1250.V1250_REALIZED_MEAN_300,
            v1250.V1249_REALIZED_MEAN_294,
            "V1250 must lift realized_mean from V1249 (显明 → 共融)",
        )
        self.assertIn(
            "V1250",
            v1250._v1250_compute_metrics().history_dim_lift,
        )

    def test_not_consummation(self):
        # divine_communion 共融 ≠ consummation 完形 状态
        self.assertGreater(
            v1250.V1250_REALIZED_MEAN_300,
            v1250.V1248_REALIZED_MEAN_288,
        )
        # 历史标签里也不能把 V1248 写错成 V1250
        labels = v1250._v1250_compute_metrics().history_dim_lift
        self.assertIn("Consummation", labels["V1248"])
        self.assertIn("Divine Communion", labels["V1250"])


class V1250CLIArtifactTest(unittest.TestCase):
    """CLI flags produce well-formed output artifacts."""

    def _run(self, flag):
        out = io.StringIO()
        saved, sys.stdout = sys.stdout, out
        try:
            rc = v1250._v1250_main([flag])
        finally:
            sys.stdout = saved
        return rc, out.getvalue()

    def test_json_flag(self):
        rc, out = self._run("--json")
        self.assertEqual(rc, 0)
        data = json.loads(out)
        self.assertEqual(data["dim_version"], "0.6.60")
        self.assertEqual(data["realized_mean_300"], 0.8775)
        self.assertEqual(data["divine_communion_substrate_pathways"], 6)
        self.assertEqual(data["total_divine_communion_molecules"], 30)

    def test_report_flag(self):
        rc, out = self._run("--report")
        self.assertEqual(rc, 0)
        self.assertIn("V1250 ASI V0.6.60", out)
        self.assertIn("divine_communion_substrate_real_lift", out)
        self.assertIn("V3 哲学守门", out)

    def test_full_flag(self):
        rc, out = self._run("--full")
        self.assertEqual(rc, 0)
        self.assertIn("V1250 ARTIFACT", out)
        self.assertIn("V1250 REPORT", out)
        # Both halves parse
        json_part, report_part = out.split("# V1250 REPORT (TEXT)\n")
        json.loads(json_part.split("# V1250 ARTIFACT (JSON)\n")[1])

    def test_default_measure(self):
        rc, out = self._run([])
        self.assertEqual(rc, 0)
        self.assertIn("V1250", out)
        self.assertIn("89.54%", out)
        self.assertIn("DIVINE_COMMUNION_THEOLOGY", out)


class V1250SubstrateAnchorsTest(unittest.TestCase):
    """Each pathway has source anchors from the documented references."""

    def test_pathway_sources(self):
        expected_sources = {
            "DIVINE_COMMUNION_THEOLOGY": "John",
            "DIVINE_COMMUNION_NEURO": "Newberg",
            "DIVINE_COMMUNION_INFORMATION": "Cover",
            "DIVINE_COMMUNION_SYSTEMS": "Holling",
            "DIVINE_COMMUNION_PHYSICS": "Prigogine",
            "DIVINE_COMMUNION_COGNITION": "Boyer",
        }
        for k, marker in expected_sources.items():
            self.assertIn(
                marker,
                v1250.V1250_DIVINE_COMMUNION_SUBSTRATE[k]["source"],
                f"{k} should reference {marker}",
            )

    def test_pathway_descriptions_nonempty(self):
        for k, p in v1250.V1250_DIVINE_COMMUNION_SUBSTRATE.items():
            self.assertTrue(p["description"], f"{k} needs description")
            self.assertGreater(len(p["description"]), 100)


class V1250PathwayRealizedTest(unittest.TestCase):
    """Pathway realization respects the strict 5-molecule invariant."""

    def test_each_pathway_realizes_one(self):
        for k in v1250.V1250_DIVINE_COMMUNION_SUBSTRATE:
            self.assertEqual(v1250._v1250_realize_pathway(k), 1.0)

    def test_realize_all_returns_all(self):
        realized = v1250._v1250_realize_all_pathways()
        self.assertEqual(set(realized.keys()),
                         set(v1250.V1250_DIVINE_COMMUNION_SUBSTRATE.keys()))


class V1250HistoryTest(unittest.TestCase):
    """History is monotonic and covers V1236..V1250."""

    def test_realized_monotonic_increase(self):
        m = v1250._v1250_compute_metrics()
        keys = sorted(m.history_realized_mean.keys())
        prev = -1.0
        for k in keys:
            v = m.history_realized_mean[k]
            self.assertGreaterEqual(v, prev, f"{k} should not regress")
            prev = v

    def test_overall_monotonic_increase(self):
        m = v1250._v1250_compute_metrics()
        keys = sorted(m.history_overall_mean.keys())
        prev = -1.0
        for k in keys:
            v = m.history_overall_mean[k]
            self.assertGreaterEqual(v, prev)
            prev = v


class V1250CLISubprocessTest(unittest.TestCase):
    """End-to-end: `python -m apeireth.v1250_...` works from a subprocess."""

    def _run_module(self, flag):
        env = os.environ.copy()
        env["PYTHONPATH"] = str(ROOT) + os.pathsep + env.get("PYTHONPATH", "")
        env["PYTHONIOENCODING"] = "utf-8"
        return subprocess.run(
            [sys.executable, "-m",
             "apeireth.v1250_asi_v0660_divine_communion_substrate_real_lift",
             flag],
            cwd=str(ROOT),
            capture_output=True,
            env=env,
        )

    def test_measure_subprocess(self):
        r = self._run_module("--measure")
        self.assertEqual(r.returncode, 0, r.stderr.decode("utf-8", "replace"))
        stdout = r.stdout.decode("utf-8", "replace")
        self.assertIn("89.54%", stdout)
        self.assertIn("DIVINE_COMMUNION_THEOLOGY", stdout)

    def test_json_subprocess(self):
        r = self._run_module("--json")
        self.assertEqual(r.returncode, 0, r.stderr.decode("utf-8", "replace"))
        stdout = r.stdout.decode("utf-8", "replace")
        data = json.loads(stdout or "{}")
        self.assertEqual(data["divine_communion_substrate_pathways"], 6)


if __name__ == "__main__":
    unittest.main()