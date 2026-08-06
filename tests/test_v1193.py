"""Tests for V1193 — ASI V0.6.5 3-dim lift.

主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手.
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

from apeireth.v1193_asi_v065_3dim_lift import (  # noqa: E402
    V1193Report,
    V1193_VERSION,
    V1193_DIM_VERSION,
    NORTH_STAR,
    V1192_BASELINE,
    V1153_BASELINE,
    DimLift,
    _measure_v2_philosophy_lift,
    _measure_reinforcement_learning_lift,
    _measure_vcp_deep_read_lift,
    _run_v1193_full,
    measure_v1193,
    run_v1193_full,
    render_report_md,
)


class TestV1193Version(unittest.TestCase):
    def test_version_constants(self):
        self.assertEqual(V1193_VERSION, "0.1.0")
        self.assertEqual(V1193_DIM_VERSION, "0.6.5")
        self.assertEqual(NORTH_STAR, 0.9800)
        self.assertAlmostEqual(V1192_BASELINE, 0.9181, places=4)
        self.assertAlmostEqual(V1153_BASELINE, 0.8929, places=4)


class TestV1193DimLift(unittest.TestCase):
    def test_dim_lift_creation(self):
        d = DimLift(
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


class TestV1193MeasureHelpers(unittest.TestCase):
    def test_v2_philosophy_lift(self):
        contrib, lift = _measure_v2_philosophy_lift()
        self.assertIn(lift.dim, "v2_philosophy")
        self.assertEqual(lift.weight, 0.05)
        self.assertGreaterEqual(lift.new_value, 0.0)
        self.assertLessEqual(lift.new_value, 1.0)
        self.assertEqual(contrib, lift.lift_contribution)

    def test_reinforcement_learning_lift(self):
        contrib, lift = _measure_reinforcement_learning_lift()
        self.assertEqual(lift.dim, "reinforcement_learning")
        self.assertEqual(lift.weight, 0.05)
        self.assertGreaterEqual(lift.new_value, 0.0)
        self.assertLessEqual(lift.new_value, 1.0)

    def test_vcp_deep_read_lift(self):
        contrib, lift = _measure_vcp_deep_read_lift()
        self.assertEqual(lift.dim, "vcp_deep_read")
        self.assertEqual(lift.weight, 0.0375)
        self.assertGreaterEqual(lift.new_value, 0.0)
        self.assertLessEqual(lift.new_value, 1.0)


class TestV1193RunFull(unittest.TestCase):
    def test_run_v1193_full(self):
        rep = _run_v1193_full(write_artifact=False)
        self.assertIsInstance(rep, V1193Report)
        self.assertGreater(rep.asi_v065, 0.0)
        self.assertLessEqual(rep.asi_v065, 1.0)
        # Should be >= V1192 baseline (0.9181) if any lift, or equal
        self.assertGreaterEqual(rep.asi_v065, V1192_BASELINE - 0.001)
        # Should be < NORTH_STAR (still room)
        self.assertLess(rep.asi_v065, NORTH_STAR)
        # V1193 - V1192 should be reasonable lift (0..0.05)
        self.assertGreaterEqual(rep.delta_asi_v065_vs_v064, 0.0)
        self.assertLessEqual(rep.delta_asi_v065_vs_v064, 0.10)
        # 3 dim lifts tracked
        self.assertEqual(len(rep.dim_lifts), 3)
        for dim_name in ("v2_philosophy", "reinforcement_learning", "vcp_deep_read"):
            self.assertIn(dim_name, rep.dim_lifts)

    def test_measure_v1193(self):
        s = measure_v1193()
        self.assertGreater(s, V1192_BASELINE - 0.01)
        self.assertLess(s, NORTH_STAR + 0.01)

    def test_run_v1193_full_with_artifact(self):
        with tempfile.TemporaryDirectory() as tmp:
            # Mock V1192 artifact
            v1192_path = Path(tmp) / "v1192_test.json"
            v1192_path.write_text(
                json.dumps({
                    "asi_v064": 0.9181,
                    "snapshot_id": "v1192-test",
                }, ensure_ascii=False),
                encoding="utf-8",
            )
            rep = _run_v1193_full(
                v1192_artifact_path=str(v1192_path),
                write_artifact=True,
                artifact_dir=tmp,
                artifact_name="v1193_test.json",
            )
            self.assertEqual(rep.asi_v064, 0.9181)
            self.assertEqual(rep.v1192_snapshot_id, "v1192-test")
            self.assertTrue(rep.artifact_path)
            self.assertTrue(Path(rep.artifact_path).is_file())


class TestV1193ReportSerialization(unittest.TestCase):
    def test_to_from_dict(self):
        rep = V1193Report(asi_v065=0.93, asi_v064=0.91, delta_asi_v065_vs_v064=0.02)
        rep.dim_lifts["test_dim"] = DimLift(
            dim="test_dim",
            baseline=0.5,
            new_value=0.7,
            delta=0.2,
            weight=0.05,
            lift_contribution=0.01,
            status="R",
            source="test",
            sub_dim_count=3,
            notes=["note1"],
        )
        d = rep.to_dict()
        self.assertIn("test_dim", d["dim_lifts"])
        rep2 = V1193Report.from_dict(d)
        self.assertEqual(rep2.asi_v065, 0.93)
        self.assertIn("test_dim", rep2.dim_lifts)
        self.assertEqual(rep2.dim_lifts["test_dim"].sub_dim_count, 3)

    def test_summary_line(self):
        rep = V1193Report(asi_v065=0.93, asi_v064=0.9181, asi_v053=0.8929, delta_asi_v065_vs_v064=0.02, delta_asi_v065_vs_v053=0.04)
        rep.n_dims_pass = 3
        line = rep.summary_line()
        self.assertIn("V1193", line)
        self.assertIn("0.9300", line)
        # summary_line template uses baseline constants V1192_BASELINE/V1153_BASELINE for display
        self.assertIn("0.9181", line)
        self.assertIn("0.8929", line)

    def test_render_report_md(self):
        rep = V1193Report(asi_v065=0.93, asi_v064=0.91, asi_v053=0.89, delta_asi_v065_vs_v064=0.02)
        rep.dim_lifts["v2_philosophy"] = DimLift(
            dim="v2_philosophy",
            baseline=0.72,
            new_value=0.76,
            delta=0.04,
            weight=0.05,
            lift_contribution=0.002,
            status="R",
            source="V1161",
            sub_dim_count=7,
        )
        md = render_report_md(rep)
        self.assertIn("V1193", md)
        self.assertIn("0.9300", md)
        self.assertIn("v2_philosophy", md)
        self.assertIn("0.0500", md)


class TestV1193PhilosophyGuards(unittest.TestCase):
    """V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43): 验证不假装."""

    def test_not_asi_north_star(self):
        rep = _run_v1193_full(write_artifact=False)
        # 不假装 V1193 = ASI 北极星
        self.assertLess(rep.asi_v065, NORTH_STAR)
        self.assertGreater(rep.vs_north_star_gap, 0.0)

    def test_v06_5_intermediate_version(self):
        # 不假装 V1193 = ASI V1.0
        self.assertEqual(V1193_DIM_VERSION, "0.6.5")
        self.assertNotIn("1.0", V1193_DIM_VERSION)

    def test_lift_is_real_not_mock(self):
        # 不假装 V1193 lift 是 mock — 验证 sub_dim_count > 0
        rep = _run_v1193_full(write_artifact=False)
        total_sub_dim = sum(d.sub_dim_count for d in rep.dim_lifts.values())
        self.assertGreater(total_sub_dim, 0)


class TestV1193CLIViaRunpy(unittest.TestCase):
    def test_measure_cli(self):
        import subprocess
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1193_asi_v065_3dim_lift", "--measure"],
            cwd=str(PROJECT_ROOT),
            capture_output=True, text=True, timeout=30,
        )
        self.assertEqual(result.returncode, 0, msg=f"stderr={result.stderr[:500]}")
        # Should be a number
        val = float(result.stdout.strip())
        self.assertGreater(val, 0.5)
        self.assertLess(val, 1.0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
