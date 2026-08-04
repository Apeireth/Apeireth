"""Tests for V1237 ASI V0.6.47 perichoresis_substrate_real_lift.

主 23:44 干到底 + 主 00:44 质量工程化 + 主 17:43 实事求是 + 主 00:56 任何人都能接手.
测试 V1237 真测 metrics + V3 哲学守门 + JSON artifact + CLI --full 自描述.

V1237 = Phase 3 起点 = 30 真分子 (6 pathway × 5 真分子) — 减半 V1236 60 = 转折而非堆叠.
"""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

# Ensure repo root on sys.path so `import apeireth.v1237_...` works
_REPO = Path(__file__).resolve().parents[1]
if str(_REPO) not in sys.path:
    sys.path.insert(0, str(_REPO))

ROOT = Path(__file__).resolve().parents[1]
V1237_PATH = ROOT / "apeireth" / "v1237_asi_v0647_perichoresis_substrate_real_lift.py"


# ============================================================================
# 1. Imports & module basics (主 00:56 任何人都能接手)
# ============================================================================


class V1237ImportTest(unittest.TestCase):
    def test_v1237_module_imports(self):
        """Module 可正常 import."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            main,
            ASI_NORTH_STAR,
            V1237_DIM_VERSION,
            V1237_PERICHORESIS_SUBSTRATE,
            _v1237_realize_all_pathways,
            _v1237_compute_metrics,
            _v1237_v3_guards,
            _v1237_json_artifact,
            _v1237_report,
            _v1237_full,
        )
        self.assertTrue(callable(main))
        self.assertEqual(ASI_NORTH_STAR, 0.9800)
        self.assertEqual(V1237_DIM_VERSION, "0.6.47")

    def test_v1237_module_path_exists(self):
        """Module 文件存在."""
        self.assertTrue(V1237_PATH.exists(), f"missing: {V1237_PATH}")

    def test_v1237_substrate_has_6_pathways(self):
        """Substrate 必须 6 pathway (Phase 3 转折 = 6 pathway 保持, 但 真分子简化)."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            V1237_PERICHORESIS_SUBSTRATE,
        )
        self.assertEqual(len(V1237_PERICHORESIS_SUBSTRATE), 6)


# ============================================================================
# 2. ASI 北极星 LOCKED (主 22:33)
# ============================================================================


class V1237NorthStarTest(unittest.TestCase):
    def test_north_star_locked_098(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import ASI_NORTH_STAR
        self.assertEqual(ASI_NORTH_STAR, 0.9800)


# ============================================================================
# 3. Phase 3 = 30 真分子 (6 pathway × 5 真分子) — Phase 3 转折 而非堆叠
# ============================================================================


class V1237MoleculeCountTest(unittest.TestCase):
    def test_pathway_count_6(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            V1237_PERICHORESIS_SUBSTRATE,
        )
        self.assertEqual(len(V1237_PERICHORESIS_SUBSTRATE), 6)

    def test_per_pathway_molecule_count_5(self):
        """Phase 3 简化: 6 pathway × 5 真分子 = 30."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            V1237_PERICHORESIS_SUBSTRATE,
        )
        for key, pathway in V1237_PERICHORESIS_SUBSTRATE.items():
            with self.subTest(pathway=key):
                self.assertEqual(
                    len(pathway["cascade_order"]),
                    5,
                    f"PERICHORESIS pathway {key} expected 5 真分子 (Phase 3 simplified), got {len(pathway['cascade_order'])}",
                )

    def test_total_molecules_30(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            V1237_PERICHORESIS_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1237_PERICHORESIS_SUBSTRATE.values())
        self.assertEqual(total, 30)

    def test_pathway_realized_all_pass(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_realize_all_pathways,
        )
        realized = _v1237_realize_all_pathways()
        for k, v in realized.items():
            with self.subTest(pathway=k):
                self.assertGreaterEqual(
                    v,
                    0.7,
                    f"pathway {k} score {v} < 0.7 — 主 19:33 cascade all 30 真分子 应 substantiate",
                )


# ============================================================================
# 4. 真测 metrics (主 17:43 实事求是)
# ============================================================================


class V1237MetricsTest(unittest.TestCase):
    def test_compute_metrics_returns_dataclass(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
        )
        metrics = _v1237_compute_metrics()
        self.assertGreater(len(metrics.snapshot_id), 30)
        self.assertEqual(metrics.dim_version, "0.6.47")
        self.assertEqual(metrics.north_star, 0.9800)

    def test_v1237_baseline_locks(self):
        """V1237 baseline 写死 — 写死不变 (主 17:43 实事求是)."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            V1237_REALIZED_MEAN_220,
            V1237_OVERALL_MEAN_390,
            V1237_PERICHORESIS_REALIZED,
        )
        self.assertEqual(V1237_REALIZED_MEAN_220, 0.8060)
        self.assertEqual(V1237_OVERALL_MEAN_390, 0.4568)
        self.assertEqual(V1237_PERICHORESIS_REALIZED, 1.0000)

    def test_realized_less_than_north_star(self):
        """不假装达到 ASI (主 17:43 + 主 17:58)."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
            ASI_NORTH_STAR,
        )
        m = _v1237_compute_metrics()
        self.assertLess(
            m.realized_mean_220,
            ASI_NORTH_STAR,
            f"realized {m.realized_mean_220} >= north_star {ASI_NORTH_STAR} — 不假装达到 ASI",
        )

    def test_lift_positive_from_v1236(self):
        """V1237 lift 应 > 0 from V1236 baseline."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
        )
        m = _v1237_compute_metrics()
        self.assertGreater(
            m.perichoresis_lift_from_v1236,
            0.0,
            f"V1237 lift from V1236 {m.perichoresis_lift_from_v1236} ≤ 0 — 转折仍 lift 必然",
        )

    def test_inflation_gap_larger_than_zero(self):
        """inflation_gap 真存在 — 主 17:43 不假装."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
        )
        m = _v1237_compute_metrics()
        self.assertGreater(
            m.inflation_gap,
            0.0,
            f"inflation_gap {m.inflation_gap} ≤ 0 — 不假装 vacuous 不存在",
        )

    def test_position_against_north_star_reasonable(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
        )
        m = _v1237_compute_metrics()
        # V1237 should be around 0.82 (= ~82% of 0.98 lock)
        self.assertGreater(m.position_vs_north_star, 0.78)
        self.assertLess(m.position_vs_north_star, 0.86)


# ============================================================================
# 5. V3 哲学守门 — 不假装 (主 17:58 + 主 20:46) — 15/15 PASS
# ============================================================================


class V1237V3GuardsTest(unittest.TestCase):
    def test_guards_count_15(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_v3_guards,
        )
        guards = _v1237_v3_guards()
        self.assertEqual(len(guards), 15)

    def test_all_guards_pass(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_v3_guards,
        )
        guards = _v1237_v3_guards()
        for g in guards:
            with self.subTest(guard=g.name):
                self.assertTrue(
                    g.passed,
                    f"V3 guard {g.name} FAILED: {g.detail}",
                )

    def test_phase3_transition_guard(self):
        """Phase 3 起点 guard."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_v3_guards,
        )
        guards = _v1237_v3_guards()
        names = {g.name for g in guards}
        self.assertIn("v1237_phase3_transition", names)


# ============================================================================
# 6. JSON artifact (主 00:56 任何人都能接手)
# ============================================================================


class V1237JsonArtifactTest(unittest.TestCase):
    def test_json_artifact_serializable(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
            _v1237_v3_guards,
            _v1237_json_artifact,
        )
        m = _v1237_compute_metrics()
        guards = _v1237_v3_guards()
        artifact_str = _v1237_json_artifact(m, guards)
        artifact = json.loads(artifact_str)
        self.assertIn("snapshot_id", artifact)
        self.assertIn("north_star", artifact)
        self.assertIn("v1237_metrics", artifact)
        self.assertIn("v3_guards", artifact)
        self.assertIn("v1237_substrate_pathways", artifact)
        self.assertEqual(artifact["dim_version"], "0.6.47")
        self.assertEqual(artifact["north_star"], 0.9800)
        # 30 真分子 = 6 × 5
        for path_key, info in artifact["v1237_substrate_pathways"].items():
            self.assertEqual(info["true_molecule_count"], 5, f"{path_key} pathway expected 5 真分子")


# ============================================================================
# 7. Report (主 17:43 实事求是)
# ============================================================================


class V1237ReportTest(unittest.TestCase):
    def test_report_includes_key_metrics(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
            _v1237_v3_guards,
            _v1237_report,
        )
        m = _v1237_compute_metrics()
        guards = _v1237_v3_guards()
        report = _v1237_report(m, guards)
        self.assertIn("V1237", report)
        self.assertIn("perichoresis", report)
        self.assertIn("Phase 3", report)
        self.assertIn("V3", report)

    def test_report_includes_substrate_pathways(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
            _v1237_v3_guards,
            _v1237_report,
        )
        m = _v1237_compute_metrics()
        guards = _v1237_v3_guards()
        report = _v1237_report(m, guards)
        # 6 pathway names
        for key in [
            "PERICHORESIS_PHILOSOPHY",
            "PERICHORESIS_NEURO",
            "PERICHORESIS_INFORMATION",
            "PERICHORESIS_ECOSYSTEM",
            "PERICHORESIS_CONTEMPLATIVE",
            "PERICHORESIS_PHYSICS",
        ]:
            self.assertIn(key, report)


# ============================================================================
# 8. CLI 自描述 (主 00:56)
# ============================================================================


class V1237CLITest(unittest.TestCase):
    def test_cli_measure_exits_0(self):
        """CLI --measure 默认 应 exit 0."""
        r = subprocess.run(
            [sys.executable, "-m", "apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift", "--measure"],
            capture_output=True,
            text=True,
            cwd=str(ROOT),
            timeout=30,
        )
        self.assertEqual(r.returncode, 0, f"stderr: {r.stderr}")

    def test_cli_json_valid(self):
        env = {**__import__("os").environ, "PYTHONIOENCODING": "utf-8"}
        r = subprocess.run(
            [sys.executable, "-m", "apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift", "--json"],
            capture_output=True,
            text=True,
            cwd=str(ROOT),
            env=env,
            timeout=30,
            encoding="utf-8",
            errors="replace",
        )
        self.assertEqual(r.returncode, 0, f"stderr: {r.stderr}")
        artifact = json.loads(r.stdout)
        self.assertEqual(artifact["north_star"], 0.9800)


# ============================================================================
# 9. 历史可追 + Phase 2 carry-over 不丢 (主 17:43 + 主 19:33)
# ============================================================================


class V1237HistoryTest(unittest.TestCase):
    def test_v1232_v1236_baselines_locked(self):
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            V1232_REALIZED_MEAN_190,
            V1233_REALIZED_MEAN_196,
            V1234_REALIZED_MEAN_202,
            V1235_REALIZED_MEAN_208,
            V1236_REALIZED_MEAN_214,
        )
        # 写死的 baseline 锁住不变 (Phase 2 四层闭环 carry over)
        self.assertEqual(V1232_REALIZED_MEAN_190, 0.7742)
        self.assertEqual(V1233_REALIZED_MEAN_196, 0.7811)
        self.assertEqual(V1234_REALIZED_MEAN_202, 0.7876)
        self.assertEqual(V1235_REALIZED_MEAN_208, 0.7937)
        self.assertEqual(V1236_REALIZED_MEAN_214, 0.7998)

    def test_perichoresis_above_kenosis_history(self):
        """V1237 history dim_lift 包含 PERICHORESIS Phase 3 起点."""
        from apeireth.v1237_asi_v0647_perichoresis_substrate_real_lift import (
            _v1237_compute_metrics,
        )
        m = _v1237_compute_metrics()
        self.assertIn("V1237", m.history_dim_lift)
        self.assertIn("Perichoresis", m.history_dim_lift["V1237"])
        self.assertIn("Phase 3", m.history_dim_lift["V1237"])


if __name__ == "__main__":
    unittest.main()
