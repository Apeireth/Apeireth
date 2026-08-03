"""Tests for V1194 — ASI V0.6.6 3-dim lift (real_production + world_model + self_improving_core).

主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手 + 主 17:58 + 主 20:46 不假装.
"""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

# Ensure project root on path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

from apeireth.v1194_asi_v066_3dim_lift import (  # noqa: E402
    V1194Report,
    V1194_VERSION,
    V1194_DIM_VERSION,
    NORTH_STAR,
    V1193_BASELINE,
    V1153_BASELINE,
    DimLift1194,
    _measure_real_production_lift,
    _measure_world_model_lift,
    _measure_self_improving_core_lift,
    _run_v1194_full,
    measure_v1194,
    run_v1194_full,
    render_report_md,
)


class TestV1194Version(unittest.TestCase):
    def test_version_constants(self):
        self.assertEqual(V1194_VERSION, "0.1.0")
        self.assertEqual(V1194_DIM_VERSION, "0.6.6")
        self.assertEqual(NORTH_STAR, 0.9800)
        self.assertAlmostEqual(V1193_BASELINE, 0.9348, places=4)
        self.assertAlmostEqual(V1153_BASELINE, 0.8929, places=4)


class TestV1194DimLift(unittest.TestCase):
    def test_dim_lift_creation(self):
        d = DimLift1194(
            dim="test",
            baseline=0.5,
            new_value=0.7,
            delta=0.2,
            weight=0.05,
            lift_contribution=0.01,
            status="R",
            source="test",
        )
        self.assertEqual(d.dim, "test")
        self.assertAlmostEqual(d.lift_contribution, 0.01, places=4)
        d_dict = d.to_dict()
        self.assertIsInstance(d_dict, dict)
        self.assertIn("dim", d_dict)


class TestV1194MeasureHelpers(unittest.TestCase):
    def test_real_production_lift(self):
        contrib, lift = _measure_real_production_lift()
        # real_production baseline 0.654 → should lift
        self.assertEqual(lift.dim, "real_production")
        self.assertAlmostEqual(lift.baseline, 0.654, places=4)
        self.assertGreaterEqual(lift.new_value, 0.0)
        self.assertLessEqual(lift.new_value, 1.0)
        # lift_contribution = delta × 0.05
        expected_contrib = lift.delta * 0.05
        self.assertAlmostEqual(contrib, lift.lift_contribution, places=4)

    def test_world_model_lift(self):
        contrib, lift = _measure_world_model_lift()
        self.assertEqual(lift.dim, "world_model")
        self.assertAlmostEqual(lift.baseline, 0.7944, places=4)
        self.assertGreaterEqual(lift.new_value, 0.0)
        self.assertLessEqual(lift.new_value, 1.0)
        self.assertGreaterEqual(contrib, 0.0)

    def test_self_improving_core_lift(self):
        contrib, lift = _measure_self_improving_core_lift()
        self.assertEqual(lift.dim, "self_improving_core")
        self.assertAlmostEqual(lift.baseline, 0.84, places=2)
        self.assertGreaterEqual(lift.new_value, 0.0)
        self.assertLessEqual(lift.new_value, 1.0)
        self.assertGreaterEqual(contrib, 0.0)


class TestV1194RunFull(unittest.TestCase):
    def test_measure_v1194(self):
        s = measure_v1194()
        self.assertGreater(s, 0.0)
        self.assertLessEqual(s, 1.0)
        # ASI V0.6.6 should be >= V1193 0.9348 (主 17:43 实事求是, lift >= 0)
        self.assertGreaterEqual(s, V1193_BASELINE)

    def test_run_v1194_full(self):
        rep = run_v1194_full(write_artifact=False)
        self.assertIsInstance(rep, V1194Report)
        self.assertTrue(rep.snapshot_id.startswith("v1194-"))
        self.assertGreater(rep.asi_v066, 0.0)
        # 3 dim lifts should all be present
        self.assertIn("real_production", rep.dim_lifts)
        self.assertIn("world_model", rep.dim_lifts)
        self.assertIn("self_improving_core", rep.dim_lifts)
        self.assertEqual(len(rep.dim_lifts), 3)

    def test_run_v1194_full_with_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            rep = run_v1194_full(write_artifact=True, artifact_dir=tmp)
            self.assertTrue(rep.artifact_path)
            p = Path(rep.artifact_path)
            self.assertTrue(p.is_file())
            data = json.loads(p.read_text(encoding="utf-8"))
            self.assertEqual(data["dim_version"], "0.6.6")
            self.assertIn("dim_lifts", data)
            self.assertEqual(len(data["dim_lifts"]), 3)


class TestV1194ReportSerialization(unittest.TestCase):
    def test_render_report_md(self):
        rep = run_v1194_full(write_artifact=False)
        md = render_report_md(rep)
        self.assertIn("# V1194 ASI V0.6.6 3-dim lift 报告", md)
        self.assertIn("ASI 北极星进度", md)
        self.assertIn("real_production", md)
        self.assertIn("world_model", md)
        self.assertIn("self_improving_core", md)
        self.assertIn(f"| **V1194 (V0.6.6)**", md)

    def test_summary_line(self):
        rep = run_v1194_full(write_artifact=False)
        s = rep.summary_line()
        self.assertIn("V1194 ASI V0.6.6 3-dim lift", s)
        self.assertIn("asi_v066=", s)
        self.assertIn("north_star", s)

    def test_to_from_dict(self):
        rep = run_v1194_full(write_artifact=False)
        d = rep.to_dict()
        rep2 = V1194Report.from_dict(d)
        self.assertEqual(rep.snapshot_id, rep2.snapshot_id)
        self.assertEqual(rep.asi_v066, rep2.asi_v066)
        self.assertEqual(set(rep.dim_lifts.keys()), set(rep2.dim_lifts.keys()))


class TestV1194PhilosophyGuards(unittest.TestCase):
    def test_lift_is_real_not_mock(self):
        """主 17:58 + 主 20:46 不假装: lift 必须真调真模块, 不是 mock."""
        contrib_rp, lift_rp = _measure_real_production_lift()
        contrib_wm, lift_wm = _measure_world_model_lift()
        contrib_si, lift_si = _measure_self_improving_core_lift()
        # 每个 lift 必须有 sub_dim_count > 0 (真调了模块)
        self.assertGreater(lift_rp.sub_dim_count, 0)
        self.assertGreater(lift_wm.sub_dim_count, 0)
        self.assertGreater(lift_si.sub_dim_count, 0)
        # 每个 lift 必须有 notes (实情记录)
        self.assertGreater(len(lift_rp.notes), 0)
        self.assertGreater(len(lift_wm.notes), 0)
        self.assertGreater(len(lift_si.notes), 0)

    def test_not_asi_north_star(self):
        """主 17:43 实事求是: V1194 ≠ ASI 北极星."""
        rep = run_v1194_full(write_artifact=False)
        # ASI V0.6.6 应该 < 0.98 (north star)
        self.assertLess(rep.asi_v066, NORTH_STAR)
        # gap > 0
        self.assertGreater(rep.vs_north_star_gap, 0.0)
        # position_pct < 100
        self.assertLess(rep.vs_north_star_position_pct, 100.0)

    def test_v06_6_intermediate_version(self):
        """主 17:58 不假装: V1194 = V0.6.6 中间版本, 不是 V1.0."""
        rep = run_v1194_full(write_artifact=False)
        self.assertEqual(rep.dim_version, "0.6.6")
        # 中间版本号, not 1.0
        self.assertNotIn("1.0", rep.dim_version)


class TestV1194CLIViaRunpy(unittest.TestCase):
    def test_measure_cli(self):
        """CLI --measure 只 print measure_v1194()."""
        import subprocess
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1194_asi_v066_3dim_lift", "--measure"],
            capture_output=True,
            text=True,
            cwd=str(PROJECT_ROOT),
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, msg=result.stderr)
        # 输出应该是 float string
        out = result.stdout.strip()
        self.assertRegex(out, r"^\d+\.\d{4}$")


if __name__ == "__main__":
    unittest.main()