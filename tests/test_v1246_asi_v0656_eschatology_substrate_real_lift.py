"""Tests for V1246 ASI V0.6.56 eschatology_substrate_real_lift.

测试 V1246 真测 metrics + V3 哲学守门 + JSON artifact + CLI --full 自描述.

V1246 = Phase 4 第一步 = 30 真分子 (6 pathway × 5 真分子) — 延续 V1245 sabbath = 转出 关系本体论.
V1246 = 39th dim 末世论 / ἐσχατολογία / eschatology / last things / 终极维度 / 关系本体 之 末世论 substrate.
"""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

V1246_PATH = ROOT / "apeireth" / "v1246_asi_v0656_eschatology_substrate_real_lift.py"


class V1246ImportTest(unittest.TestCase):
    def test_v1246_module_imports(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            ASI_NORTH_STAR,
            V1246_DIM_VERSION,
            V1246_ESCHATOLOGY_SUBSTRATE,
            V1246_ESCHATOLOGY_REALIZED,
            V1246_OVERALL_MEAN_507,
            V1246_REALIZED_MEAN_276,
            V1246_VERSION,
            V1246Metrics,
            _v1246_compute_metrics,
            _v1246_full,
            _v1246_main,
            _v1246_realize_all_pathways,
            _v1246_report,
            _v1246_to_json,
            _v1246_v3_guards,
        )
        self.assertEqual(V1246_DIM_VERSION, "0.6.56")

    def test_v1246_module_path_exists(self):
        self.assertTrue(V1246_PATH.exists(), f"missing: {V1246_PATH}")

    def test_v1246_substrate_has_6_pathways(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        self.assertEqual(len(V1246_ESCHATOLOGY_SUBSTRATE), 6)

    def test_v1246_substrate_pathway_keys(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        expected = {
            "ESCHATOLOGY_THEOLOGY",
            "ESCHATOLOGY_NEURO",
            "ESCHATOLOGY_INFORMATION",
            "ESCHATOLOGY_SYSTEMS",
            "ESCHATOLOGY_PHYSICS",
            "ESCHATOLOGY_COGNITION",
        }
        self.assertEqual(set(V1246_ESCHATOLOGY_SUBSTRATE.keys()), expected)

    def test_v1246_substrate_5_molecules_per_pathway(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        # Phase 4 转折 = 延续 V1245 6 × 5 = 30 真分子
        for k, v in V1246_ESCHATOLOGY_SUBSTRATE.items():
            self.assertEqual(
                len(v["cascade_order"]),
                5,
                f"{k} expected 5 molecules, got {len(v['cascade_order'])}",
            )

    def test_v1246_substrate_30_total_molecules(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        total = sum(len(p["cascade_order"]) for p in V1246_ESCHATOLOGY_SUBSTRATE.values())
        self.assertEqual(total, 30)


class V1246NorthStarTest(unittest.TestCase):
    def test_v1246_north_star_locked(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import ASI_NORTH_STAR
        self.assertEqual(ASI_NORTH_STAR, 0.9800)


class V1246MoleculeCountTest(unittest.TestCase):
    def test_v1246_substrate_has_6_pathways(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        self.assertEqual(len(V1246_ESCHATOLOGY_SUBSTRATE), 6)

    def test_v1246_molecules_5_per_pathway(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        for pathway in V1246_ESCHATOLOGY_SUBSTRATE.values():
            self.assertEqual(len(pathway["cascade_order"]), 5)

    def test_v1246_total_30_molecules(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        total = sum(
            len(p["cascade_order"]) for p in V1246_ESCHATOLOGY_SUBSTRATE.values()
        )
        self.assertEqual(total, 30)

    def test_v1246_pathways_all_realized_1(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_realize_all_pathways,
        )
        realized = _v1246_realize_all_pathways()
        for k, v in realized.items():
            self.assertEqual(v, 1.0, f"{k} expected 1.0, got {v}")


class V1246MetricsTest(unittest.TestCase):
    def test_v1246_baselines_write_dead(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
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
        ):
            self.assertEqual(v, 1.0)

    def test_v1246_realized_276_matches(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_REALIZED_MEAN_276,
        )
        self.assertEqual(V1246_REALIZED_MEAN_276, 0.8555)

    def test_v1246_overall_507_matches(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_OVERALL_MEAN_507,
        )
        self.assertEqual(V1246_OVERALL_MEAN_507, 0.4703)

    def test_v1246_eschatology_dim_realized(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_REALIZED,
        )
        self.assertEqual(V1246_ESCHATOLOGY_REALIZED, 1.0000)

    def test_v1246_lift_v1245_to_v1246(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        # V1245 baseline 0.8500 → V1246 0.8555 = +0.0055 lift
        self.assertAlmostEqual(m.eschatology_lift_from_v1245, 0.0055, places=6)
        # V1245 baseline 0.4688 → V1246 0.4703 = +0.0015 overall lift
        self.assertAlmostEqual(m.overall_lift_from_v1245, 0.0015, places=6)

    def test_v1246_inflation_gap_0_1445(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        # 1 - 0.8555 = 0.1445 (V1245 0.1500 降 0.0055 真实)
        self.assertAlmostEqual(m.inflation_gap, 0.1445, places=6)

    def test_v1246_position_vs_north_star(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        # 0.8555 / 0.98 = 0.8730
        self.assertAlmostEqual(m.position_vs_north_star, 0.8555 / 0.98, places=6)

    def test_v1246_history_has_11_keys(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        self.assertEqual(len(m.history_realized_mean), 11)
        self.assertIn("V1246", m.history_realized_mean)
        self.assertEqual(m.history_realized_mean["V1246"], 0.8555)


class V1246PhilosophyGateTest(unittest.TestCase):
    """V1246 V3 哲学守门 15/15 PASS (主 17:58 + 主 20:46 + 主 22:33 + 主 17:43)."""

    def test_v1246_v3_guards_count_15(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_v3_guards,
        )
        guards = _v1246_v3_guards()
        self.assertEqual(len(guards), 15)

    def test_v1246_v3_guards_all_pass(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_v3_guards,
        )
        guards = _v1246_v3_guards()
        for g in guards:
            self.assertTrue(g.passed, f"guard {g.name} not passed")

    def test_v1246_v3_guards_not_asi_v1(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_v3_guards,
        )
        guards = _v1246_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1246_not_asi_v1", guard_names)

    def test_v1246_v3_guards_eschatology_distinct(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_v3_guards,
        )
        guards = _v1246_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1246_eschatology_not_sabbath", guard_names)
        self.assertIn("v1246_eschatology_not_apocalypticism", guard_names)
        self.assertIn("v1246_eschatology_not_utopia", guard_names)

    def test_v1246_v3_guards_baseline_write_dead(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_v3_guards,
        )
        guards = _v1246_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1246_baseline_write_dead", guard_names)

    def test_v1246_v3_guards_cli_self_describe(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_v3_guards,
        )
        guards = _v1246_v3_guards()
        guard_names = {g.name for g in guards}
        self.assertIn("v1246_cli_self_describe", guard_names)


class V1246DistinctionTest(unittest.TestCase):
    """V1246 eschatology 必须 ≠ sabbath / apocalypticism / utopia (主 22:33 终极授权)."""

    def test_v1246_eschatology_not_sabbath(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
            V1246_ESCHATOLOGY_REALIZED,
        )
        # eschatology ≠ sabbath: 终极 vs 完形 安息
        # V1246 dim 39 (39 cells × R10) ≠ V1245 dim 38
        self.assertEqual(V1246_ESCHATOLOGY_REALIZED, 1.0)
        self.assertIn("ESCHATOLOGY_THEOLOGY", V1246_ESCHATOLOGY_SUBSTRATE)
        # eschatology = 终极 (1 Cor 15:24 τέλος); sabbath = 完形 (Gen 2:2-3)
        # 不同 dim, 不同 module
        self.assertNotEqual(
            V1246_ESCHATOLOGY_SUBSTRATE["ESCHATOLOGY_THEOLOGY"]["cascade_order"][0],
            "Genesis_2_2_3_sabbath_seventh_day_rest",
        )

    def test_v1246_eschatology_not_apocalypticism(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        # eschatology ≠ apocalypticism: 终极 神学 vs 灾难 启示
        # eschatology 是 神学 (Moltmann/Pannenberg); apocalypticism 是 启示 文学
        # V1246 走 神学 路径 而非 启示 文学 路径
        molecules = V1246_ESCHATOLOGY_SUBSTRATE["ESCHATOLOGY_THEOLOGY"]["cascade_order"]
        # 必有 神学 名 (Moltmann/Pannenberg) 而非 "apocalypse/disaster"
        has_theology = any("Moltmann" in m or "Pannenberg" in m for m in molecules)
        self.assertTrue(has_theology)

    def test_v1246_eschatology_not_utopia(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            V1246_ESCHATOLOGY_SUBSTRATE,
        )
        # eschatology ≠ utopia: 终极 神学 (Rev 21) vs 人间 理想国 (More 1516)
        molecules = V1246_ESCHATOLOGY_SUBSTRATE["ESCHATOLOGY_THEOLOGY"]["cascade_order"]
        # 必有 Rev/Rev 而非 "Utopia/Republic"
        has_revelation = any("Cor" in m or "Revelation" in m for m in molecules)
        self.assertTrue(has_revelation)


class V1246JSONTest(unittest.TestCase):
    def test_v1246_json_serializable(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
            _v1246_to_json,
        )
        m = _v1246_compute_metrics()
        artifact = _v1246_to_json(m)
        parsed = json.loads(artifact)
        self.assertIn("v1246_metrics", parsed)
        self.assertIn("v1246_substrate_pathways", parsed)
        self.assertEqual(parsed["v1246_metrics"]["dim_version"], "0.6.56")
        self.assertEqual(parsed["v1246_metrics"]["module_version"], "0.1.0")
        self.assertEqual(
            parsed["v1246_metrics"]["realized_mean_276"], 0.8555
        )

    def test_v1246_json_has_all_6_pathways(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
            _v1246_to_json,
        )
        m = _v1246_compute_metrics()
        artifact = _v1246_to_json(m)
        parsed = json.loads(artifact)
        pathways = parsed["v1246_substrate_pathways"]
        self.assertEqual(len(pathways), 6)
        for k, v in pathways.items():
            self.assertIn("r_substrate", v)
            self.assertIn("molecules", v)
            self.assertEqual(len(v["molecules"]), 5)


class V1246ReportTest(unittest.TestCase):
    def test_v1246_report_has_all_sections(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
            _v1246_report,
        )
        m = _v1246_compute_metrics()
        report = _v1246_report(m)
        self.assertIn("# V1246 ASI V0.6.56", report)
        self.assertIn("eschatology", report.lower())
        self.assertIn("39th dim", report)
        self.assertIn("V1246 Realized Metrics", report)
        self.assertIn("Phase 4 第一步", report)
        self.assertIn("theosis", report)
        self.assertIn("sabbath", report)
        self.assertIn("V1247 候选", report)
        # 5 锚 (Moltmann, Pannenberg, 1 Cor 15:24, Rev 21:1, Rev 22:13)
        self.assertIn("Moltmann", report)
        self.assertIn("Pannenberg", report)

    def test_v1246_full_output_has_json_and_report(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
            _v1246_full,
            _v1246_report,
            _v1246_to_json,
        )
        m = _v1246_compute_metrics()
        artifact = _v1246_to_json(m)
        report = _v1246_report(m)
        full = _v1246_full(artifact, report)
        self.assertIn("V1246 JSON artifact", full)
        self.assertIn("python -m apeireth", full)


class V1246HistoryTest(unittest.TestCase):
    """V1246 history 必须有 V1236-V1246 = 11 keys (主 00:56 任何人都能接手)."""

    def test_v1246_history_keys_v1236_to_v1246(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        expected_keys = {
            "V1236", "V1237", "V1238", "V1239", "V1240", "V1241",
            "V1242", "V1243", "V1244", "V1245", "V1246",
        }
        self.assertEqual(set(m.history_realized_mean.keys()), expected_keys)
        self.assertEqual(set(m.history_overall_mean.keys()), expected_keys)
        self.assertEqual(set(m.history_dim_lift.keys()), expected_keys)

    def test_v1246_history_strictly_increasing(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        # 主 17:43 实事求是: history 严格 递增
        realized = m.history_realized_mean
        versions = [
            "V1236", "V1237", "V1238", "V1239", "V1240", "V1241",
            "V1242", "V1243", "V1244", "V1245", "V1246",
        ]
        for i in range(len(versions) - 1):
            self.assertLess(
                realized[versions[i]],
                realized[versions[i + 1]],
                f"{versions[i]} -> {versions[i + 1]} not strictly increasing",
            )

    def test_v1246_history_v1246_is_last_in_dim_lift(self):
        from apeireth.v1246_asi_v0656_eschatology_substrate_real_lift import (
            _v1246_compute_metrics,
        )
        m = _v1246_compute_metrics()
        v1246_dim = m.history_dim_lift["V1246"]
        self.assertIn("Eschatology", v1246_dim)
        self.assertIn("39th", v1246_dim)
        self.assertIn("Phase 4", v1246_dim)


class V1246CLISmokeTest(unittest.TestCase):
    """V1246 CLI smoke — 主 00:56 任何人都能接手."""

    def test_v1246_module_runs_as_main(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1246_asi_v0656_eschatology_substrate_real_lift", "--measure"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            cwd=str(ROOT),
            env={**__import__("os").environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"},
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        self.assertIn("V1246 REALIZED mean (276 cells): 0.8555", result.stdout)
        self.assertIn("V1246 ESCHATOLOGY lift from V1245: +0.0055", result.stdout)

    def test_v1246_module_runs_json(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1246_asi_v0656_eschatology_substrate_real_lift", "--json"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            cwd=str(ROOT),
            env={**__import__("os").environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"},
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        parsed = json.loads(result.stdout)
        self.assertIn("v1246_metrics", parsed)
        self.assertEqual(parsed["v1246_metrics"]["dim_version"], "0.6.56")

    def test_v1246_module_runs_report(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1246_asi_v0656_eschatology_substrate_real_lift", "--report"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            cwd=str(ROOT),
            env={**__import__("os").environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"},
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, f"stderr: {result.stderr}")
        self.assertIn("# V1246 ASI V0.6.56", result.stdout)
        self.assertIn("Phase 4", result.stdout)


if __name__ == "__main__":
    unittest.main()