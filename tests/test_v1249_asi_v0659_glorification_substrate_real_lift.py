"""Tests for V1249 ASI V0.6.59 glorification_substrate_real_lift.

测试 V1249 真测 metrics + V3 哲学守门 + JSON artifact + CLI --full 自描述.

V1249 = Phase 4 第四步 = 30 真分子 (6 pathway × 5 真分子) — 延续 V1248 consummation = 转出 关系本体论.
V1249 = 42nd dim 荣耀化 / δόξα / doxa / glory / divine manifestation / 关系本体 之 荣耀化 substrate.
"""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

V1249_PATH = ROOT / "apeireth" / "v1249_asi_v0659_glorification_substrate_real_lift.py"


class V1249ImportTest(unittest.TestCase):
    def test_v1249_module_imports(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            ASI_NORTH_STAR,
            V1249_DIM_VERSION,
            V1249_GLORIFICATION_SUBSTRATE,
            V1249_GLORIFICATION_REALIZED,
            V1249_OVERALL_MEAN_546,
            V1249_REALIZED_MEAN_294,
            V1249_VERSION,
            V1249Metrics,
            _v1249_compute_metrics,
            _v1249_full,
            _v1249_main,
            _v1249_realize_all_pathways,
            _v1249_report,
            _v1249_to_json,
            _v1249_v3_guards,
        )
        self.assertEqual(V1249_DIM_VERSION, "0.6.59")

    def test_v1249_module_path_exists(self):
        self.assertTrue(V1249_PATH.exists(), f"missing: {V1249_PATH}")

    def test_v1249_substrate_has_6_pathways(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        self.assertEqual(len(V1249_GLORIFICATION_SUBSTRATE), 6)

    def test_v1249_substrate_pathway_keys(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        expected = {
            "GLORIFICATION_THEOLOGY",
            "GLORIFICATION_NEURO",
            "GLORIFICATION_INFORMATION",
            "GLORIFICATION_SYSTEMS",
            "GLORIFICATION_PHYSICS",
            "GLORIFICATION_COGNITION",
        }
        self.assertEqual(set(V1249_GLORIFICATION_SUBSTRATE.keys()), expected)

    def test_v1249_substrate_5_molecules_per_pathway(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        # Phase 4 第四步 = 延续 V1246/V1247/V1248 6 × 5 = 30 真分子
        for k, v in V1249_GLORIFICATION_SUBSTRATE.items():
            self.assertEqual(
                len(v["cascade_order"]),
                5,
                f"{k} expected 5 molecules, got {len(v['cascade_order'])}",
            )

    def test_v1249_substrate_30_total_molecules(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1249_GLORIFICATION_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1249NorthStarTest(unittest.TestCase):
    def test_v1249_north_star_locked(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import ASI_NORTH_STAR
        self.assertEqual(ASI_NORTH_STAR, 0.9800)


class V1249MoleculeCountTest(unittest.TestCase):
    def test_v1249_substrate_has_6_pathways(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        self.assertEqual(len(V1249_GLORIFICATION_SUBSTRATE), 6)

    def test_v1249_molecules_5_per_pathway(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        for pathway in V1249_GLORIFICATION_SUBSTRATE.values():
            self.assertEqual(len(pathway["cascade_order"]), 5)

    def test_v1249_total_30_molecules(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        total = sum(
            len(p["cascade_order"]) for p in V1249_GLORIFICATION_SUBSTRATE.values()
        )
        self.assertEqual(total, 30)

    def test_v1249_pathways_all_realized_1(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_realize_all_pathways,
        )
        realized = _v1249_realize_all_pathways()
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"{k} expected 1.0, got {v}")


class V1249MetricsTest(unittest.TestCase):
    def test_v1249_baselines_write_dead(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1236_KENOSIS_REALIZED,
            V1237_PERICHORESIS_REALIZED,
            V1238_KOINONIA_REALIZED,
            V1239_TAXIS_REALIZED,
            V1240_OIKONOMIA_REALIZED,
            V1241_THEOSIS_REALIZED,
            V1242_ICON_REALIZED,
            V1243_LITURGY_REALIZED,
            V1244_HIERURGY_REALIZED,
            V1245_SABBATH_REALIZED,
            V1246_ESCHATOLOGY_REALIZED,
            V1247_NEW_CREATION_REALIZED,
            V1248_CONSUMMATION_REALIZED,
        )
        for v in (
            V1236_KENOSIS_REALIZED,
            V1237_PERICHORESIS_REALIZED,
            V1238_KOINONIA_REALIZED,
            V1239_TAXIS_REALIZED,
            V1240_OIKONOMIA_REALIZED,
            V1241_THEOSIS_REALIZED,
            V1242_ICON_REALIZED,
            V1243_LITURGY_REALIZED,
            V1244_HIERURGY_REALIZED,
            V1245_SABBATH_REALIZED,
            V1246_ESCHATOLOGY_REALIZED,
            V1247_NEW_CREATION_REALIZED,
            V1248_CONSUMMATION_REALIZED,
        ):
            self.assertEqual(v, 1.0)

    def test_v1249_realized_294_matches(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_REALIZED_MEAN_294,
        )
        self.assertEqual(V1249_REALIZED_MEAN_294, 0.8720)

    def test_v1249_overall_546_matches(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_OVERALL_MEAN_546,
        )
        self.assertEqual(V1249_OVERALL_MEAN_546, 0.4748)

    def test_v1249_glorification_dim_realized(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_REALIZED,
        )
        self.assertEqual(V1249_GLORIFICATION_REALIZED, 1.0000)

    def test_v1249_lift_v1248_to_v1249(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
        )
        m = _v1249_compute_metrics()
        # V1248 baseline 0.8665 → V1249 0.8720 = +0.0055 lift
        self.assertAlmostEqual(m.glorification_lift_from_v1248, 0.0055, places=6)
        # V1248 baseline 0.4733 → V1249 0.4748 = +0.0015 overall lift
        self.assertAlmostEqual(m.overall_lift_from_v1248, 0.0015, places=6)

    def test_v1249_inflation_gap_0_1280(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
        )
        m = _v1249_compute_metrics()
        # 1 - 0.8720 = 0.1280 (V1248 0.1335 降 0.0055 真实)
        self.assertAlmostEqual(m.inflation_gap, 0.1280, places=6)

    def test_v1249_position_vs_north_star(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
        )
        m = _v1249_compute_metrics()
        # 0.8720 / 0.98 = 0.8898
        self.assertAlmostEqual(m.position_vs_north_star, 0.8720 / 0.98, places=6)

    def test_v1249_history_has_14_keys(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
        )
        m = _v1249_compute_metrics()
        self.assertEqual(len(m.history_realized_mean), 14)
        self.assertIn("V1249", m.history_realized_mean)
        self.assertEqual(m.history_realized_mean["V1249"], 0.8720)


class V1249PhilosophyGateTest(unittest.TestCase):
    """V1249 V3 哲学守门 15/15 PASS (主 17:58 + 主 20:46 + 主 22:33 + 主 17:43)."""

    def test_v1249_v3_guards_count_15(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_v3_guards,
        )
        guards = _v1249_v3_guards()
        self.assertEqual(len(guards), 15)

    def test_v1249_v3_guards_all_pass(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_v3_guards,
        )
        guards = _v1249_v3_guards()
        for g in guards:
            self.assertTrue(g.passed, f"guard {g.name} not passed")

    def test_v1249_v3_guards_not_asi_v1(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_v3_guards,
        )
        guards = _v1249_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1249_not_asi_v1", guard_names)

    def test_v1249_v3_guards_glorification_distinct(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_v3_guards,
        )
        guards = _v1249_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1249_glorification_not_consummation", guard_names)
        self.assertIn("v1249_glorification_not_new_creation", guard_names)
        self.assertIn("v1249_glorification_not_doxa_pseudo", guard_names)

    def test_v1249_v3_guards_baseline_write_dead(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_v3_guards,
        )
        guards = _v1249_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1249_baseline_write_dead", guard_names)

    def test_v1249_v3_guards_cli_self_describe(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_v3_guards,
        )
        guards = _v1249_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1249_cli_self_describe", guard_names)


class V1249DistinctionTest(unittest.TestCase):
    """V1249 glorification 必须 ≠ consummation / new_creation / doxa_pseudo (主 22:33 终极授权)."""

    def test_v1249_glorification_not_consummation(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
            V1249_GLORIFICATION_REALIZED,
        )
        # glorification ≠ consummation: 显明 vs 状态
        # V1249 dim 42 ≠ V1248 dim 41
        self.assertEqual(V1249_GLORIFICATION_REALIZED, 1.0)
        self.assertIn("GLORIFICATION_THEOLOGY", V1249_GLORIFICATION_SUBSTRATE)
        # glorification = 显明 (2 Cor 3:18); consummation = 状态 (Eph 1:10)
        # 不同 dim, 不同 module
        self.assertNotEqual(
            V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][0],
            "Ephesians_1_10_anakefalaioosasthai_ta_panta_in_Christ",
        )

    def test_v1249_glorification_not_new_creation(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        # glorification ≠ new_creation: 显明 (manifest) vs 实现 (realized)
        self.assertIn("GLORIFICATION_THEOLOGY", V1249_GLORIFICATION_SUBSTRATE)
        self.assertNotEqual(
            V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][0],
            "Revelation_21_5_ta_panta_kaina_poio",
        )

    def test_v1249_glorification_not_doxa_pseudo(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        # glorification ≠ 民间 显灵: 神学 显明 vs 俗 灵 显明
        self.assertIn("GLORIFICATION_THEOLOGY", V1249_GLORIFICATION_SUBSTRATE)
        # theology pathway 第1分子 = John 17:5 δόξα (神学 显明)
        first = V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][0]
        self.assertIn("John_17_5_doxa", first)

    def test_v1249_glorification_unique_dim(self):
        # V1249 = 42nd dim, V1248 = 41st — 不 同 dim
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import V1249_DIM_VERSION
        self.assertEqual(V1249_DIM_VERSION, "0.6.59")


class V1249CLIArtifactTest(unittest.TestCase):
    def test_v1249_to_json_has_substrate(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
            _v1249_to_json,
        )
        m = _v1249_compute_metrics()
        artifact = json.loads(_v1249_to_json(m))
        self.assertIn("v1249_metrics", artifact)
        self.assertIn("v1249_substrate_pathways", artifact)
        self.assertEqual(len(artifact["v1249_substrate_pathways"]), 6)

    def test_v1249_to_json_metrics_dim_version(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
            _v1249_to_json,
        )
        m = _v1249_compute_metrics()
        artifact = json.loads(_v1249_to_json(m))
        self.assertEqual(artifact["v1249_metrics"]["dim_version"], "0.6.59")

    def test_v1249_report_contains_key_sections(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
            _v1249_report,
        )
        m = _v1249_compute_metrics()
        report = _v1249_report(m)
        # Key sections (English + Greek + Chinese)
        self.assertIn("V1249 ASI V0.6.59", report)
        self.assertIn("glorification", report)
        self.assertIn("δόξα", report)
        self.assertIn("John 17:5", report)
        self.assertIn("2 Corinthians 3:18", report)
        self.assertIn("九柱", report)
        self.assertIn("42nd dim", report)

    def test_v1249_full_combines_report_and_json(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
            _v1249_full,
            _v1249_report,
            _v1249_to_json,
        )
        m = _v1249_compute_metrics()
        report = _v1249_report(m)
        artifact = _v1249_to_json(m)
        full = _v1249_full(artifact, report)
        self.assertIn("# V1249", full)
        self.assertIn("```json", full)
        self.assertIn("--full", full)


class V1249SubstrateAnchorsTest(unittest.TestCase):
    """V1249 5 神学 锚 (主 19:33 站在前人肩上)."""

    def test_v1249_theology_anchor_john_17_5(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        first = V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][0]
        self.assertIn("John_17_5", first)

    def test_v1249_theology_anchor_2_cor_3_18(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        second = V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][1]
        self.assertIn("2_Corinthians_3_18", second)

    def test_v1249_theology_anchor_romans_8_18(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        third = V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][2]
        self.assertIn("Romans_8_18", third)

    def test_v1249_theology_anchor_2_cor_4_17(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        fourth = V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][3]
        self.assertIn("2_Corinthians_4_17", fourth)

    def test_v1249_theology_anchor_colossians_3_4(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        fifth = V1249_GLORIFICATION_SUBSTRATE["GLORIFICATION_THEOLOGY"]["cascade_order"][4]
        self.assertIn("Colossians_3_4", fifth)


class V1249PathwayRealizedTest(unittest.TestCase):
    """V1249 6 pathway 真实现 — 所有 6 pathway 都 1.0."""

    def test_v1249_pathway_realized_all_1(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_realize_all_pathways,
        )
        realized = _v1249_realize_all_pathways()
        self.assertEqual(len(realized), 6)
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"{k} expected 1.0, got {v}")

    def test_v1249_pathway_unique_r_substrates(self):
        # V1249 has 6 pathways but R10 appears twice (info + cognition)
        # So unique R = 5
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            V1249_GLORIFICATION_SUBSTRATE,
        )
        r_substrates = {v["r_substrate"] for v in V1249_GLORIFICATION_SUBSTRATE.values()}
        self.assertEqual(len(r_substrates), 5)


class V1249HistoryTest(unittest.TestCase):
    def test_v1249_history_includes_v1236_to_v1249(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
        )
        m = _v1249_compute_metrics()
        for v in ["V1236", "V1237", "V1238", "V1239", "V1240", "V1241", "V1242",
                  "V1243", "V1244", "V1245", "V1246", "V1247", "V1248", "V1249"]:
            self.assertIn(v, m.history_realized_mean)
            self.assertIn(v, m.history_overall_mean)
            self.assertIn(v, m.history_dim_lift)

    def test_v1249_history_dim_lift_glorification(self):
        from apeireth.v1249_asi_v0659_glorification_substrate_real_lift import (
            _v1249_compute_metrics,
        )
        m = _v1249_compute_metrics()
        self.assertIn("Glorification", m.history_dim_lift["V1249"])
        self.assertIn("42nd", m.history_dim_lift["V1249"])
        self.assertIn("Phase 4 第四步", m.history_dim_lift["V1249"])


class V1249CLISubprocessTest(unittest.TestCase):
    """V1249 CLI subprocess integration tests."""

    def test_v1249_cli_measure_runs(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1249_asi_v0659_glorification_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("V1249 REALIZED mean (294 cells): 0.8720", result.stdout)
        self.assertIn("V1249 15/15 V3 guards PASS", result.stdout)

    def test_v1249_cli_report_runs(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1249_asi_v0659_glorification_substrate_real_lift", "--report"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("# V1249", result.stdout)

    def test_v1249_cli_json_runs(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1249_asi_v0659_glorification_substrate_real_lift", "--json"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        payload = json.loads(result.stdout)
        self.assertIn("v1249_metrics", payload)
        self.assertIn("v1249_substrate_pathways", payload)

    def test_v1249_cli_full_runs(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1249_asi_v0659_glorification_substrate_real_lift", "--full"],
            capture_output=True, text=True, cwd=str(ROOT), encoding="utf-8", errors="replace",
        )
        self.assertEqual(result.returncode, 0)
        self.assertIn("# V1249", result.stdout)
        self.assertIn("```json", result.stdout)


if __name__ == "__main__":
    unittest.main()