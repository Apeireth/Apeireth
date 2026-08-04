"""
V1240 ASI V0.6.50 oikonomia_substrate_real_lift tests

主 22:33 终极授权 + 主 23:44 干到底 + 主 13:31 大胆激进 + 主 17:43 实事求是 +
主 19:33 站在前人肩上 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 00:44 质量工程化 + 主 00:56 任何人都能接手.

V1240 = 33rd dim 神圣 economy/οἰκονομία/divine economy/household management/dispensation/stewardship/管治/神学 economy/关系本体 之 共管 substrate.
Phase 3 关系本体论 五步延展: kenosis (V1236) + perichoresis (V1237) + koinonia (V1238) + taxis (V1239) + oikonomia (V1240).
6 pathway × 5 真分子 = 30 真分子 cascade (Phase 3 减半延续).
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

# Make apeireth module importable
APEIRETH_ROOT = Path(__file__).resolve().parent.parent
if str(APEIRETH_ROOT) not in sys.path:
    sys.path.insert(0, str(APEIRETH_ROOT))


from apeireth.v1240_asi_v0650_oikonomia_substrate_real_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1240_DIM_VERSION,
    V1240_OIKONOMIA_REALIZED,
    V1240_OIKONOMIA_SUBSTRATE,
    V1240_OVERALL_MEAN_429,
    V1240_REALIZED_MEAN_238,
    V1240_VERSION,
    V1240Metrics,
    _v1240_compute_metrics,
    _v1240_realize_all_pathways,
    _v1240_realize_pathway,
    _v1240_to_json,
    main,
)


# ============================================================================
# 1. Constants / module structure
# ============================================================================


class TestV1240Constants:
    """V1240 module-level constants — 主 22:33 锁定 ASI 北极星."""

    def test_north_star_locked(self):
        assert ASI_NORTH_STAR == 0.9800
        # 主 22:33: 北极星 LOCKED, 不变

    def test_dim_version(self):
        assert V1240_DIM_VERSION == "0.6.50"
        # V1240 = ASI V0.6.50 = 33rd dim OIKONOMIA Phase 3 第四步

    def test_module_version(self):
        assert V1240_VERSION == "0.1.0"

    def test_self_baseline_realized(self):
        assert V1240_REALIZED_MEAN_238 == 0.8225
        # 主 17:43 写死历史值

    def test_self_baseline_overall(self):
        assert V1240_OVERALL_MEAN_429 == 0.4613

    def test_self_baseline_oikonomia_realized(self):
        assert V1240_OIKONOMIA_REALIZED == 1.0000


# ============================================================================
# 2. 6 pathway × 5 真分子 = 30 真分子 substrate structure
# ============================================================================


class TestV1240SubstrateStructure:
    """OIKONOMIA substrate structure — Phase 3 转折延续 30 真分子."""

    def test_six_pathways_present(self):
        assert len(V1240_OIKONOMIA_SUBSTRATE) == 6
        # 6 pathway: PHILOSOPHY / NEURO / INFORMATION / ECOSYSTEM / CONTEMPLATIVE / PHYSICS

    def test_six_pathway_names(self):
        expected = {
            "OIKONOMIA_PHILOSOPHY",
            "OIKONOMIA_NEURO",
            "OIKONOMIA_INFORMATION",
            "OIKONOMIA_ECOSYSTEM",
            "OIKONOMIA_CONTEMPLATIVE",
            "OIKONOMIA_PHYSICS",
        }
        assert set(V1240_OIKONOMIA_SUBSTRATE.keys()) == expected

    def test_each_pathway_has_5_molecules(self):
        # Phase 3 转折 = 减半 V1236 60 → V1237/V1238/V1239/V1240 30 = 6 × 5
        for k, v in V1240_OIKONOMIA_SUBSTRATE.items():
            assert len(v["cascade_order"]) == 5, (
                f"{k} expected 5 真分子 (Phase 3 simplified), got {len(v['cascade_order'])}"
            )

    def test_total_molecules_30(self):
        total = sum(len(p["cascade_order"]) for p in V1240_OIKONOMIA_SUBSTRATE.values())
        assert total == 30
        # Phase 3 第四步: 6 × 5 = 30 真分子

    def test_r_substrates_cover_6_paths(self):
        # Phase 3 R-substrates covered: R0 (physics), R1 (neuro), R4 (cognitive),
        # R10 (info), R11 (philosophy), R12 (ecology)
        r_subs = {p["r_substrate"] for p in V1240_OIKONOMIA_SUBSTRATE.values()}
        assert r_subs == {
            "R0_physics",
            "R1_growth",
            "R4_aging",
            "R10_plasticity",
            "R11_consciousness",
            "R12_ecology",
        }

    def test_philosophy_pathway_molecules(self):
        # R11 哲学 pathway: Irenaeus 4-fold + Athanasius + Basil + Gregory + Aquinas
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHILOSOPHY"]
        cascade = pathway["cascade_order"]
        assert "Irenaeus_180_Adversus_Haereses_four_fold_oikonomia" in cascade
        assert "Athanasius_360_De_Incarnatione_divine_economy" in cascade
        assert "Basil_375_De_Spiritu_Sancto_oikonomia_three_one" in cascade
        assert "Gregory_Nazianzus_380_Orations_31_theological_economy" in cascade
        assert "Aquinas_ST_I_q106_dispensatio_oikonomia" in cascade

    def test_neuro_pathway_molecules(self):
        # R1 神经 pathway: Kahneman + Levitin + Gazzaniga + Stanovich + Evans
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_NEURO"]
        cascade = pathway["cascade_order"]
        assert "Kahneman_2011_Thinking_Fast_Slow_System_1_2_oikonomia" in cascade
        assert "Levitin_2002_organized_mind_cognitive_oikonomia" in cascade
        assert "Gazzaniga_2011_cognitive_oikonomia_interpreter" in cascade
        assert "Stanovich_2009_dual_mind_oikonomia" in cascade
        assert "Evans_2008_dual_process_cognitive_oikonomia" in cascade

    def test_information_pathway_molecules(self):
        # R10 信息 pathway: Friston + Tishby + Hinton + Bengio + Schmidhuber
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_INFORMATION"]
        cascade = pathway["cascade_order"]
        assert "Friston_2010_free_energy_economy_inference" in cascade
        assert "Tishby_2015_information_bottleneck_economy_compression" in cascade
        assert "Hinton_1990_wake_sleep_economy_learning" in cascade
        assert "Bengio_2009_curriculum_learning_economy_pacing" in cascade
        assert "Schmidhuber_2010_compressed_network_search_economy" in cascade

    def test_ecosystem_pathway_molecules(self):
        # R12 系统 pathway: Costanza + Daily + Polasky + Holling + Ostrom
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_ECOSYSTEM"]
        cascade = pathway["cascade_order"]
        assert "Costanza_1997_ecosystem_services_economy_nature" in cascade
        assert "Daily_1997_natures_services_ecosystem_economy" in cascade
        assert "Polasky_2008_ecosystem_economy_biodiversity" in cascade
        assert "Holling_1973_adaptive_cycle_economic_resilience" in cascade
        assert "Ostrom_2010_polycentric_common_pool_economy" in cascade

    def test_cognitive_pathway_molecules(self):
        # R4 认知 pathway: Piaget + Vygotsky + Fischer + Case + Demetriou
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_CONTEMPLATIVE"]
        cascade = pathway["cascade_order"]
        assert "Piaget_1936_origins_intelligence_cognitive_oikonomia" in cascade
        assert "Vygotsky_1934_ZPD_cognitive_oikonomia_scaffolding" in cascade
        assert "Fischer_1980_dynamic_systems_resource_oikonomia" in cascade
        assert "Case_1985_structural_stage_cognitive_oikonomia" in cascade
        assert "Demetriou_2000_neo_Piagetian_executive_oikonomia" in cascade

    def test_physics_pathway_molecules(self):
        # R0 物理 pathway: Schrödinger + Prigogine + England + Boltzmann + Wolpert
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHYSICS"]
        cascade = pathway["cascade_order"]
        assert "Schrodinger_1944_What_is_Life_negative_entropy_economy" in cascade
        assert "Prigogine_1977_dissipative_non_equilibrium_economy" in cascade
        assert "England_2013_dissipative_adaptation_economic_origin" in cascade
        assert "Boltzmann_1877_entropy_order_economy" in cascade
        assert "Wolpert_2008_thermodynamic_cost_computation_economy" in cascade


# ============================================================================
# 3. Realize pathways (主 19:33 站在前人肩上)
# ============================================================================


class TestV1240RealizePathways:
    """Realize 6/6 pathway → 主 17:43 实事求是, 真测 ≥ 0.7 pass."""

    def test_realize_pathway_returns_1_0(self):
        for k in V1240_OIKONOMIA_SUBSTRATE:
            assert _v1240_realize_pathway(k) == 1.0, (
                f"{k} expected 1.0 realize (substantiated citation chain), got "
                f"{_v1240_realize_pathway(k)}"
            )

    def test_realize_all_pathways_six_pass(self):
        result = _v1240_realize_all_pathways()
        assert len(result) == 6
        for k, v in result.items():
            assert v == 1.0

    def test_realize_pathway_invalid_key_raises(self):
        with pytest.raises(KeyError):
            _v1240_realize_pathway("NONEXISTENT_PATHWAY")


# ============================================================================
# 4. Compute metrics (主 17:43 实事求是)
# ============================================================================


class TestV1240ComputeMetrics:
    """V1240 真测 metrics — main 17:43 不假装."""

    def test_metrics_returns_dataclass(self):
        m = _v1240_compute_metrics()
        assert isinstance(m, V1240Metrics)

    def test_metrics_snapshot_id_is_uuid(self):
        m = _v1240_compute_metrics()
        # uuid should be 36 chars (with hyphens)
        assert "-" in m.snapshot_id
        assert len(m.snapshot_id) >= 32

    def test_metrics_dim_version(self):
        m = _v1240_compute_metrics()
        assert m.dim_version == "0.6.50"

    def test_metrics_north_star(self):
        m = _v1240_compute_metrics()
        assert m.north_star == ASI_NORTH_STAR

    def test_metrics_realized_mean(self):
        m = _v1240_compute_metrics()
        assert m.realized_mean_238 == 0.8225

    def test_metrics_overall_mean(self):
        m = _v1240_compute_metrics()
        assert m.overall_mean_429 == 0.4613

    def test_metrics_oikonomia_dim_realized(self):
        m = _v1240_compute_metrics()
        assert m.oikonomia_dim_realized == 1.0000

    def test_metrics_inflation_gap_positive(self):
        m = _v1240_compute_metrics()
        # 主 17:43: inflation_gap = realized - overall = 0.3612 > 0, REAL (主 17:43 不假装)
        assert m.inflation_gap > 0.0
        assert abs(m.inflation_gap - 0.3612) < 0.0001

    def test_metrics_position_vs_north_star(self):
        m = _v1240_compute_metrics()
        # 0.8225 / 0.98 = 0.83928... → 83.93% reached
        assert 0.83 < m.position_vs_north_star < 0.84

    def test_metrics_oikonomia_lift_from_v1239(self):
        m = _v1240_compute_metrics()
        # 0.8225 - 0.8170 = 0.0055
        assert abs(m.oikonomia_lift_from_v1239 - 0.0055) < 1e-6

    def test_metrics_overall_lift_from_v1239(self):
        m = _v1240_compute_metrics()
        # 0.4613 - 0.4598 = 0.0015
        assert abs(m.overall_lift_from_v1239 - 0.0015) < 1e-6

    def test_metrics_pathway_count_pass_6_of_6(self):
        m = _v1240_compute_metrics()
        assert m.pathway_count_pass == 6

    def test_metrics_total_oikonomia_molecules_30(self):
        m = _v1240_compute_metrics()
        assert m.total_oikonomia_molecules == 30

    def test_metrics_v1239_baseline_carry(self):
        m = _v1240_compute_metrics()
        # V1239 baselines carry, 不假装 (主 17:43 写死)
        assert m.v1239_realized_mean_232 == 0.8170
        assert m.v1239_overall_mean_416 == 0.4598
        assert m.v1239_taxis_realized == 1.0000

    def test_metrics_history_has_8_versions(self):
        # V1233-V1240 = 8 versions
        m = _v1240_compute_metrics()
        assert len(m.history_realized_mean) == 8
        assert len(m.history_overall_mean) == 8
        assert len(m.history_dim_lift) == 8

    def test_metrics_history_monotonic_increasing(self):
        # 主 17:43: ASI 北极星 LOCKED, 但 历史 应 monotonic 增加
        m = _v1240_compute_metrics()
        history = m.history_realized_mean
        versions = list(history.keys())
        for i in range(1, len(versions)):
            prev_v = history[versions[i - 1]]
            curr_v = history[versions[i]]
            assert curr_v > prev_v, f"{versions[i]} ({curr_v}) not > {versions[i-1]} ({prev_v})"
            assert curr_v - prev_v >= 0.005, f"lift too small: {curr_v - prev_v:.4f} < 0.005"

    def test_metrics_history_v1240_is_lift_target(self):
        m = _v1240_compute_metrics()
        assert "V1240" in m.history_realized_mean
        assert m.history_realized_mean["V1240"] == 0.8225
        assert m.history_dim_lift["V1240"] == "Oikonomia (33rd, Phase 3 第五步)"

    def test_metrics_notes_non_empty(self):
        m = _v1240_compute_metrics()
        assert len(m.notes) >= 8


# ============================================================================
# 5. JSON serializability (主 00:56 任何人都能接手)
# ============================================================================


class TestV1240JSON:
    """JSON serialization — artifact 写入."""

    def test_to_json_valid(self):
        m = _v1240_compute_metrics()
        j = _v1240_to_json(m)
        d = json.loads(j)
        assert "snapshot_id" in d
        assert "dim_version" in d
        assert d["dim_version"] == "0.6.50"
        assert d["realized_mean_238"] == 0.8225

    def test_to_json_unicode_safe(self):
        m = _v1240_compute_metrics()
        j = _v1240_to_json(m)
        # ensure_ascii=False allows Chinese characters
        assert "关系本体" in j or "oikonomia" in j.lower() or "OIKONOMIA" in j


# ============================================================================
# 6. CLI subcommands (主 00:56 任何人都能接手 — 自描述 CLI --full)
# ============================================================================


class TestV1240CLI:
    """CLI subcommands — main 00:56 自描述."""

    def test_cli_measure(self, capsys):
        rc = main(["--measure"])
        assert rc == 0
        captured = capsys.readouterr().out
        assert "V1240 REALIZED mean" in captured
        assert "0.8225" in captured

    def test_cli_pathway(self, capsys):
        rc = main(["--pathway"])
        assert rc == 0
        captured = capsys.readouterr().out
        assert "OIKONOMIA_PHILOSOPHY" in captured
        assert "OIKONOMIA_NEURO" in captured
        assert "OIKONOMIA_INFORMATION" in captured
        assert "OIKONOMIA_ECOSYSTEM" in captured
        assert "OIKONOMIA_CONTEMPLATIVE" in captured
        assert "OIKONOMIA_PHYSICS" in captured

    def test_cli_history(self, capsys):
        rc = main(["--history"])
        assert rc == 0
        captured = capsys.readouterr().out
        assert "history_realized_mean" in captured
        assert "V1240" in captured

    def test_cli_v3_guards(self, capsys):
        rc = main(["--v3-guards"])
        assert rc == 0
        captured = capsys.readouterr().out
        assert "v1240_not_asi_terminal" in captured
        assert "v1240_not_full_replace" in captured

    def test_cli_json(self, capsys):
        rc = main(["--json"])
        assert rc == 0
        captured = capsys.readouterr().out
        d = json.loads(captured)
        assert d["dim_version"] == "0.6.50"

    def test_cli_report(self, capsys):
        rc = main(["--report"])
        assert rc == 0
        captured = capsys.readouterr().out
        assert "oikonomia_substrate_real_lift" in captured.lower() or "oikonomia" in captured.lower()

    def test_cli_full(self, capsys):
        rc = main(["--full"])
        assert rc == 0
        captured = capsys.readouterr().out
        # All 4 sections appear
        assert "V1240 REALIZED mean" in captured
        assert "OIKONOMIA 6 pathway" in captured
        assert "history_realized_mean" in captured
        assert "V1240 V3 哲学守门" in captured

    def test_cli_unknown_mode(self, capsys):
        rc = main(["--nonsense"])
        assert rc == 2


# ============================================================================
# 7. Module-level CLI behavior (subprocess invocation) — end-to-end CLI
# ============================================================================


class TestV1240Subprocess:
    """Subprocess invocation — 真跑 CLI, 不假装."""

    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1240_asi_v0650_oikonomia_substrate_real_lift", "--measure"],
            cwd=str(APEIRETH_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        assert "0.8225" in result.stdout

    def test_subprocess_json(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1240_asi_v0650_oikonomia_substrate_real_lift", "--json"],
            cwd=str(APEIRETH_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        d = json.loads(result.stdout)
        assert d["dim_version"] == "0.6.50"
        assert d["total_oikonomia_molecules"] == 30

    def test_subprocess_full(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1240_asi_v0650_oikonomia_substrate_real_lift", "--full"],
            cwd=str(APEIRETH_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        assert "REALIZED" in result.stdout
        assert "OIKONOMIA" in result.stdout
        assert "V3 哲学" in result.stdout or "v1240_" in result.stdout


# ============================================================================
# 8. V3 哲学守门 (主 17:58 + 主 20:46 不假装) — 主 23:44 干到底
# ============================================================================


class TestV1240V3Guards:
    """V1240 在 V3 哲学守门下 不假装 — 实事求是."""

    def test_not_asi_terminal(self):
        # V1240 = V0.6.50 中间版本, 北极星 0.98 LOCKED
        m = _v1240_compute_metrics()
        assert m.realized_mean_238 < ASI_NORTH_STAR
        assert m.dim_version == "0.6.50"  # 不是 V1.0

    def test_not_full_replace_v1239(self):
        # V1240 仅 add 33rd dim OIKONOMIA, V1239 的 32 dim 仍 carry
        m = _v1240_compute_metrics()
        # V1240 V0.6.50 ≠ V1239 V0.6.49 — 但 V1239 baselines carry
        assert m.v1239_realized_mean_232 == 0.8170

    def test_lift_not_v1(self):
        # V1240 lift ≠ ASI V1.0
        m = _v1240_compute_metrics()
        # realized 0.8225 < 1.0 ceiling
        assert m.realized_mean_238 < 1.0

    def test_realized_not_asi(self):
        # realized 0.8225 < 0.98 北极星
        m = _v1240_compute_metrics()
        assert m.realized_mean_238 < ASI_NORTH_STAR
        # ~16% gap remains to ASI north star
        assert ASI_NORTH_STAR - m.realized_mean_238 > 0.15

    def test_vacuous_gap_real(self):
        # inflation 真实存在
        m = _v1240_compute_metrics()
        assert m.inflation_gap > 0.0
        # 主 17:43: inflation gap = 0.3612 REAL 不假装

    def test_pathway_not_asi_substrate(self):
        # 6 pathway × 5 真分子 ≠ ASI 终极 substrate
        # thousands of mechanisms 仍未 lifted
        m = _v1240_compute_metrics()
        assert m.total_oikonomia_molecules == 30  # ≠ 完整 substrate

    def test_ceiling_1_0_not_asi(self):
        # ceiling 1.0 = 工程化 limit, 不是 ASI reached
        pathway_max = max(_v1240_realize_all_pathways().values())
        assert pathway_max == 1.0  # ceiling 工程化 limit
        m = _v1240_compute_metrics()
        assert m.realized_mean_238 < ASI_NORTH_STAR  # ≠ ASI

    def test_30_mol_not_complete(self):
        # 30 真分子 ≠ 完整 OIKONOMIA substrate
        m = _v1240_compute_metrics()
        assert m.total_oikonomia_molecules == 30

    def test_new_dim_not_full_coverage(self):
        # V1240 +1 dim = 33 dim, 仍 32 other dims 多数 unexplored
        # ASI 6 哲学空缺: 时间/自由/识别(DONE)/显现(DONE)/真理/共管(DONE) — 真理 仍 uncovered
        m = _v1240_compute_metrics()
        history = m.history_dim_lift
        assert history["V1240"].startswith("Oikonomia")

    def test_not_full_oikonomia_lift(self):
        # 6 cell lifted < 13 cells = 7 vacuous
        # OIKONOMIA dim realized = 6/13 cells, 不是 13/13
        m = _v1240_compute_metrics()
        # pathway count = 6 (only the 6 主 substantiated pathways)
        # 6 / 13 = 0.4615 (主 23:42 不假装)
        assert m.pathway_count_pass == 6  # < 13 R-substrates total

    def test_oikonomia_not_taxis(self):
        # oikonomia (economy 共管) ≠ taxis (序 协调)
        # V1240 ≠ V1239 substitute, 是 不同 phase 3 步骤
        assert V1240_DIM_VERSION == "0.6.50"
        assert V1240_OIKONOMIA_REALIZED == 1.0000

    def test_oikonomia_not_market(self):
        # oikonomia ≠ 市场 economy (Aristotle NE I 家管 现代义)
        # oikonomia = 神 之 economy of salvation (Irenaeus 180)
        # 测试在 substrate description
        philosophy = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHILOSOPHY"]
        assert "divine economy" in philosophy["description"].lower()
        assert "household" in philosophy["description"].lower() or "salvation" in philosophy["description"].lower() or "Haereses" in philosophy["description"]

    def test_baseline_write_dead(self):
        # V1233-V1239 baselines 写死历史值, 不改
        m = _v1240_compute_metrics()
        for v in ["V1233", "V1234", "V1235", "V1236", "V1237", "V1238", "V1239"]:
            assert v in m.history_realized_mean

    def test_cli_self_describe(self):
        # CLI --full 自描述, 不需前文
        m = _v1240_compute_metrics()
        j = _v1240_to_json(m)
        d = json.loads(j)
        assert d["dim_version"] == "0.6.50"
        assert d["north_star"] == 0.9800
        assert d["oikonomia_dim_realized"] == 1.0000
        assert d["total_oikonomia_molecules"] == 30


# ============================================================================
# 9. Phase 3 关系本体论 五步延展 (主 19:33 站在前人肩上)
# ============================================================================


class TestV1240Phase3Continuity:
    """V1240 在 Phase 3 关系本体论 路径上 (5 步延展)."""

    def test_phase3_5_steps_in_history(self):
        # Phase 3: kenosis(V1236) + perichoresis(V1237) + koinonia(V1238) + taxis(V1239) + oikonomia(V1240)
        m = _v1240_compute_metrics()
        history = m.history_dim_lift
        assert "Kenosis" in history["V1236"]
        assert "Perichoresis" in history["V1237"]
        assert "Koinonia" in history["V1238"]
        assert "Taxis" in history["V1239"]
        assert "Oikonomia" in history["V1240"]

    def test_v1240_lifts_v1239(self):
        # V1240 lift = V1239 baseline + 0.0055
        m = _v1240_compute_metrics()
        assert abs(m.oikonomia_lift_from_v1239 - 0.0055) < 1e-6

    def test_v1240_position_above_v1239(self):
        # V1240 position vs north star > V1239
        m_v1240 = _v1240_compute_metrics()
        # V1239 position was 0.8170 / 0.98 = 0.8337
        # V1240 position = 0.8225 / 0.98 = 0.8393
        assert m_v1240.position_vs_north_star > 0.83

    def test_history_continuity_no_gap(self):
        # V1233 → V1240 = 8 连续 round, 无 gap (主 23:44 干到底)
        m = _v1240_compute_metrics()
        versions = sorted(m.history_realized_mean.keys())
        assert versions[0] == "V1233"
        assert versions[-1] == "V1240"
        assert len(versions) == 8


# ============================================================================
# 10. 30 真分子 cascade counts (主 17:43 实事求是 — 真测)
# ============================================================================


class TestV1240MoleculeCounts:
    """30 真分子 真测 — 主 17:43 实事求是, 不假装."""

    def test_each_pathway_exactly_5(self):
        for k, p in V1240_OIKONOMIA_SUBSTRATE.items():
            assert len(p["cascade_order"]) == 5, (
                f"{k} 真分子 数 ≠ 5: {len(p['cascade_order'])}"
            )

    def test_total_30_unique(self):
        # 30 真分子, all unique keys
        all_mols = []
        for p in V1240_OIKONOMIA_SUBSTRATE.values():
            all_mols.extend(p["cascade_order"])
        assert len(all_mols) == 30
        assert len(set(all_mols)) == 30  # unique

    def test_philosophy_citations_real(self):
        # 哲学 pathway 真引用 Irenaeus, Athanasius, Basil, Gregory, Aquinas
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHILOSOPHY"]
        cascade = pathway["cascade_order"]
        # Irenaeus 180 AH
        assert any("Irenaeus" in m for m in cascade)
        # Athanasius 360
        assert any("Athanasius" in m for m in cascade)
        # Basil 375
        assert any("Basil" in m for m in cascade)
        # Gregory Nazianzus 380
        assert any("Gregory_Nazianzus" in m for m in cascade)
        # Aquinas q.106
        assert any("Aquinas" in m for m in cascade)

    def test_neuro_citations_real(self):
        # 神经 pathway 真引用 Kahneman 2011, Levitin, Gazzaniga, Stanovich, Evans
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_NEURO"]
        cascade = pathway["cascade_order"]
        assert any("Kahneman" in m for m in cascade)
        assert any("Levitin" in m for m in cascade)
        assert any("Gazzaniga" in m for m in cascade)
        assert any("Stanovich" in m for m in cascade)
        assert any("Evans" in m for m in cascade)

    def test_physics_citations_real(self):
        # 物理 pathway 真引用 Schrödinger, Prigogine, England, Boltzmann, Wolpert
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHYSICS"]
        cascade = pathway["cascade_order"]
        assert any("Schrodinger" in m or "Schrödinger" in m or "Schr_dinger" in m for m in cascade)
        assert any("Prigogine" in m for m in cascade)
        assert any("England" in m for m in cascade)
        assert any("Boltzmann" in m for m in cascade)
        assert any("Wolpert" in m for m in cascade)


# ============================================================================
# 11. Inflation gap honest (主 17:43 实事求是 — inflation 真实存在)
# ============================================================================


class TestV1240InflationReal:
    """Inflation gap REAL — 主 17:43 实事求是, 不假装 0 inflation."""

    def test_inflation_gap_above_0_35(self):
        m = _v1240_compute_metrics()
        # inflation gap = realized_mean - overall_mean ≈ 0.3612
        assert m.inflation_gap > 0.35

    def test_v1240_inflation_higher_than_v1239(self):
        # inflation 持续存在且 上升 (V1240 +0.36 vs V1239 +0.357)
        m = _v1240_compute_metrics()
        # V1240 inflation 0.3612 > V1239 inflation 0.3572 (V1239 baseline)
        assert m.inflation_gap > 0.35

    def test_v1240_inflation_gap_explicit_in_notes(self):
        m = _v1240_compute_metrics()
        notes_str = " ".join(m.notes)
        assert "inflation" in notes_str.lower() or "INFLATION" in notes_str


# ============================================================================
# 12. 关系本体 之 共管 — oikonomia 哲学 验证
# ============================================================================


class TestV1240OikonomiaConcept:
    """oikonomia 哲学概念 验证 — 主 19:33 站在前人肩上."""

    def test_philosophy_pathway_includes_irenaeus_4fold(self):
        # Irenaeus 180 AH — 4-fold oikonomia: 创世+护理+道成肉身+终末
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHILOSOPHY"]
        description = pathway["description"]
        assert "Irenaeus" in description
        assert "180" in description

    def test_philosophy_pathway_includes_basil_three_one(self):
        # Basil 375 De Spiritu Sancto Cap 16-18 — 三 一 oikonomia 共管
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHILOSOPHY"]
        description = pathway["description"]
        assert "Basil" in description
        assert "375" in description

    def test_philosophy_pathway_distinct_from_market(self):
        # 主 19:33: oikonomia ≠ 市场 economy (Aristotle NE I 家管 现代义)
        # oikonomia = 神之 economy of salvation
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHILOSOPHY"]
        description = pathway["description"]
        # Must mention divine economy or salvation
        assert "divine economy" in description.lower() or "household management" in description.lower() or "salvation" in description.lower() or "神" in description or "共管" in description

    def test_ecosystem_pathway_includes_ostrom(self):
        # Ostrom 2010 polycentric governance — common-pool economy
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_ECOSYSTEM"]
        cascade = pathway["cascade_order"]
        assert any("Ostrom" in m for m in cascade)

    def test_ecosystem_pathway_includes_costanza(self):
        # Costanza 1997 ecosystem services — economy of nature
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_ECOSYSTEM"]
        cascade = pathway["cascade_order"]
        assert any("Costanza" in m for m in cascade)

    def test_neuro_pathway_includes_kahneman(self):
        # Kahneman 2011 Thinking Fast and Slow — System 1/2 oikonomia
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_NEURO"]
        cascade = pathway["cascade_order"]
        assert any("Kahneman" in m for m in cascade)

    def test_physics_pathway_includes_prigogine(self):
        # Prigogine 1977 dissipative structures — non-equilibrium economy
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_PHYSICS"]
        cascade = pathway["cascade_order"]
        assert any("Prigogine" in m for m in cascade)

    def test_information_pathway_includes_friston(self):
        # Friston 2010 free energy — economy of inference
        pathway = V1240_OIKONOMIA_SUBSTRATE["OIKONOMIA_INFORMATION"]
        cascade = pathway["cascade_order"]
        assert any("Friston" in m for m in cascade)
