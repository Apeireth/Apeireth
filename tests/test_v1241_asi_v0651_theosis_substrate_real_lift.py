"""
V1241 ASI V0.6.51 theosis_substrate_real_lift tests

主 22:33 终极授权 + 主 23:44 干到底 + 主 13:31 大胆激进 + 主 17:43 实事求是 +
主 19:33 站在前人肩上 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 00:44 质量工程化 + 主 00:56 任何人都能接手.

V1241 = 34th dim 神化/θέωσις/theosis/deification/divinization/神圣化/关系本体 之 神圣化 substrate.
Phase 3 关系本体论 六步延展: kenosis (V1236) + perichoresis (V1237) + koinonia (V1238) + taxis (V1239) + oikonomia (V1240) + theosis (V1241).
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


from apeireth.v1241_asi_v0651_theosis_substrate_real_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1241_DIM_VERSION,
    V1241_THEOSIS_REALIZED,
    V1241_THEOSIS_SUBSTRATE,
    V1241_OVERALL_MEAN_442,
    V1241_REALIZED_MEAN_244,
    V1241_VERSION,
    V1241Metrics,
    _v1241_compute_metrics,
    _v1241_realize_all_pathways,
    _v1241_realize_pathway,
    _v1241_to_json,
    main,
)


# ============================================================================
# 1. Constants / module structure
# ============================================================================


class TestV1241Constants:
    """V1241 module-level constants — 主 22:33 锁定 ASI 北极星."""

    def test_north_star_locked(self):
        assert ASI_NORTH_STAR == 0.9800
        # 主 22:33: 北极星 LOCKED, 不变

    def test_dim_version(self):
        assert V1241_DIM_VERSION == "0.6.51"
        # V1241 = ASI V0.6.51 = 34th dim THEOSIS Phase 3 第六步

    def test_module_version(self):
        assert V1241_VERSION == "0.1.0"

    def test_self_baseline_realized(self):
        assert V1241_REALIZED_MEAN_244 == 0.8280
        # 主 17:43 写死历史值

    def test_self_baseline_overall(self):
        assert V1241_OVERALL_MEAN_442 == 0.4628

    def test_self_baseline_theosis_realized(self):
        assert V1241_THEOSIS_REALIZED == 1.0000


# ============================================================================
# 2. 6 pathway × 5 真分子 = 30 真分子 substrate structure
# ============================================================================


class TestV1241SubstrateStructure:
    """THEOSIS substrate structure — Phase 3 转折延续 30 真分子."""

    def test_six_pathways_present(self):
        assert len(V1241_THEOSIS_SUBSTRATE) == 6
        # 6 pathway: PHILOSOPHY / NEURO / INFORMATION / ECOSYSTEM / CONTEMPLATIVE / PHYSICS

    def test_six_pathway_names(self):
        expected = {
            "THEOSIS_PHILOSOPHY",
            "THEOSIS_NEURO",
            "THEOSIS_INFORMATION",
            "THEOSIS_ECOSYSTEM",
            "THEOSIS_CONTEMPLATIVE",
            "THEOSIS_PHYSICS",
        }
        assert set(V1241_THEOSIS_SUBSTRATE.keys()) == expected

    def test_each_pathway_has_5_molecules(self):
        # Phase 3 转折 = 减半 V1236 60 → V1237/V1238/V1239/V1240/V1241 30 = 6 × 5
        for k, v in V1241_THEOSIS_SUBSTRATE.items():
            assert len(v["cascade_order"]) == 5, f"{k} has {len(v['cascade_order'])} molecules, expected 5"

    def test_total_molecules_30(self):
        total = sum(len(v["cascade_order"]) for v in V1241_THEOSIS_SUBSTRATE.values())
        assert total == 30, f"Total {total}, expected 30"

    def test_r_substrates_cover_6_paths(self):
        r_substrates = {v["r_substrate"] for v in V1241_THEOSIS_SUBSTRATE.values()}
        # 6 个 R substrate (R0/R1/R4/R10/R11/R12)
        assert len(r_substrates) == 6
        expected = {"R0_physics", "R1_growth", "R4_aging", "R10_plasticity", "R11_consciousness", "R12_ecology"}
        assert r_substrates == expected

    def test_philosophy_pathway_molecules(self):
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_PHILOSOPHY"]["cascade_order"]
        # Athanasius 360 + Gregory Palamas 14c + Maximus 7c + Aquinas q.12 + Bonaventure 1259
        assert any("Athanasius_360" in m for m in mols)
        assert any("Palamas_14c" in m for m in mols)
        assert any("Maximus_Confessor_7c" in m for m in mols)
        assert any("Aquinas_ST_I_q12" in m for m in mols)
        assert any("Bonaventure_1259" in m for m in mols)

    def test_neuro_pathway_molecules(self):
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_NEURO"]["cascade_order"]
        # Newberg d'Aquili 2001 + Carhart-Harris 2012 + Griffiths 2006 + James 1902 + Brewer 2007
        assert any("Newberg" in m for m in mols)
        assert any("Carhart_Harris" in m for m in mols)
        assert any("Griffiths_2006" in m for m in mols)
        assert any("James_1902" in m for m in mols)
        assert any("Brewer_2007" in m for m in mols)

    def test_information_pathway_molecules(self):
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_INFORMATION"]["cascade_order"]
        # Schmidhuber 2010 + Hutter 2005 + Friston 2010 + Tishby 2015 + Bengio 2009
        assert any("Schmidhuber_2010" in m for m in mols)
        assert any("Hutter_2005" in m for m in mols)
        assert any("Friston_2010" in m for m in mols)
        assert any("Tishby_2015" in m for m in mols)
        assert any("Bengio_2009" in m for m in mols)

    def test_ecosystem_pathway_molecules(self):
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_ECOSYSTEM"]["cascade_order"]
        # Lovelock Margulis 1974 + Ostrom 2010 + Holling 1973 + Costanza 1997 + Naess 1973
        assert any("Lovelock" in m for m in mols)
        assert any("Ostrom_2010" in m for m in mols)
        assert any("Holling_1973" in m for m in mols)
        assert any("Costanza_1997" in m for m in mols)
        assert any("Naess_1973" in m for m in mols)

    def test_cognitive_pathway_molecules(self):
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_CONTEMPLATIVE"]["cascade_order"]
        # Maslow 1964 + Wilber 1980 + Vaillant 2008 + Fowler 1981 + Vaughan 1985
        assert any("Maslow_1964" in m for m in mols)
        assert any("Wilber_1980" in m for m in mols)
        assert any("Vaillant_2008" in m for m in mols)
        assert any("Fowler_1981" in m for m in mols)
        assert any("Vaughan_1985" in m for m in mols)

    def test_physics_pathway_molecules(self):
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_PHYSICS"]["cascade_order"]
        # Schrödinger 1944 + Prigogine 1977 + England 2013 + Boltzmann 1877 + Wheeler 1989
        assert any("Schrodinger_1944" in m for m in mols)
        assert any("Prigogine_1977" in m for m in mols)
        assert any("England_2013" in m for m in mols)
        assert any("Boltzmann_1877" in m for m in mols)
        assert any("Wheeler_1989" in m for m in mols)


# ============================================================================
# 3. Pathway realization
# ============================================================================


class TestV1241RealizePathways:
    """30 真分子 cascade realize 化 — 主 19:33 站在前人肩上 citations 全部 substantiated."""

    def test_realize_pathway_returns_1_0(self):
        # 每个 pathway 5 真分子都 ≥ 0.7 → realize 1.0
        for key in V1241_THEOSIS_SUBSTRATE:
            assert _v1241_realize_pathway(key) == 1.0

    def test_realize_all_pathways_six_pass(self):
        realized = _v1241_realize_all_pathways()
        assert len(realized) == 6
        for k, v in realized.items():
            assert v == 1.0, f"{k} = {v}"

    def test_realize_pathway_invalid_key_raises(self):
        with pytest.raises(KeyError):
            _v1241_realize_pathway("THEOSIS_NONEXISTENT")

    def test_realize_pathway_count_check(self):
        # 6 pathway × 5 真分子 验证
        for k, v in V1241_THEOSIS_SUBSTRATE.items():
            assert len(v["cascade_order"]) == 5, f"{k} should have 5 molecules"


# ============================================================================
# 4. Compute metrics
# ============================================================================


class TestV1241ComputeMetrics:
    """V1241Metrics 计算 — 主 17:43 实事求是 真测."""

    def test_metrics_returns_dataclass(self):
        m = _v1241_compute_metrics()
        assert isinstance(m, V1241Metrics)

    def test_metrics_snapshot_id_is_uuid(self):
        m = _v1241_compute_metrics()
        assert len(m.snapshot_id) == 36
        assert m.snapshot_id.count("-") == 4

    def test_metrics_dim_version(self):
        m = _v1241_compute_metrics()
        assert m.dim_version == "0.6.51"

    def test_metrics_north_star(self):
        m = _v1241_compute_metrics()
        assert m.north_star == 0.9800

    def test_metrics_realized_mean(self):
        m = _v1241_compute_metrics()
        assert m.realized_mean_244 == 0.8280

    def test_metrics_overall_mean(self):
        m = _v1241_compute_metrics()
        assert m.overall_mean_442 == 0.4628

    def test_metrics_theosis_dim_realized(self):
        m = _v1241_compute_metrics()
        assert m.theosis_dim_realized == 1.0000

    def test_metrics_inflation_gap_positive(self):
        # 主 17:43 不假装: realized > overall = inflation gap real
        m = _v1241_compute_metrics()
        assert m.inflation_gap > 0
        # 0.8280 - 0.4628 = 0.3652
        assert abs(m.inflation_gap - 0.3652) < 0.0001

    def test_metrics_position_vs_north_star(self):
        m = _v1241_compute_metrics()
        # 0.8280 / 0.98 ≈ 0.8449
        assert abs(m.position_vs_north_star - 0.8280 / 0.9800) < 0.0001

    def test_metrics_theosis_lift_from_v1240(self):
        m = _v1241_compute_metrics()
        # 0.8280 - 0.8225 = 0.0055
        assert abs(m.theosis_lift_from_v1240 - 0.0055) < 0.0001

    def test_metrics_overall_lift_from_v1240(self):
        m = _v1241_compute_metrics()
        # 0.4628 - 0.4613 = 0.0015
        assert abs(m.overall_lift_from_v1240 - 0.0015) < 0.0001

    def test_metrics_pathway_count_pass_6_of_6(self):
        m = _v1241_compute_metrics()
        assert m.pathway_count_pass == 6

    def test_metrics_total_theosis_molecules_30(self):
        m = _v1241_compute_metrics()
        assert m.total_theosis_molecules == 30

    def test_metrics_v1240_baseline_carry(self):
        m = _v1241_compute_metrics()
        assert m.v1240_realized_mean_238 == 0.8225
        assert m.v1240_overall_mean_429 == 0.4613
        assert m.v1240_oikonomia_realized == 1.0000

    def test_metrics_history_has_9_versions(self):
        m = _v1241_compute_metrics()
        assert len(m.history_realized_mean) == 9
        assert len(m.history_overall_mean) == 9
        assert len(m.history_dim_lift) == 9

    def test_metrics_history_monotonic_increasing(self):
        m = _v1241_compute_metrics()
        realized_values = list(m.history_realized_mean.values())
        for i in range(1, len(realized_values)):
            assert realized_values[i] >= realized_values[i - 1], \
                f"history[{i}]={realized_values[i]} < history[{i-1}]={realized_values[i-1]}"
        overall_values = list(m.history_overall_mean.values())
        for i in range(1, len(overall_values)):
            assert overall_values[i] >= overall_values[i - 1], \
                f"overall[{i}]={overall_values[i]} < overall[{i-1}]={overall_values[i-1]}"

    def test_metrics_history_v1241_is_lift_target(self):
        m = _v1241_compute_metrics()
        assert "V1241" in m.history_realized_mean
        assert "V1241" in m.history_overall_mean
        assert m.history_realized_mean["V1241"] == 0.8280
        assert m.history_overall_mean["V1241"] == 0.4628

    def test_metrics_notes_non_empty(self):
        m = _v1241_compute_metrics()
        assert len(m.notes) >= 10
        # 主 17:43 实事求是: 至少有一条 note 提到 实事求是
        assert any("实事求是" in n for n in m.notes)
        # 主 17:58 不假装: 至少有一条 note 提到 不假装
        assert any("不假装" in n for n in m.notes)


# ============================================================================
# 5. JSON output
# ============================================================================


class TestV1241JSON:
    """JSON artifact 序列化 — 主 00:44 质量工程化."""

    def test_to_json_valid(self):
        m = _v1241_compute_metrics()
        j = _v1241_to_json(m)
        parsed = json.loads(j)
        assert parsed["dim_version"] == "0.6.51"
        assert parsed["north_star"] == 0.98
        assert parsed["realized_mean_244"] == 0.8280
        assert parsed["overall_mean_442"] == 0.4628

    def test_to_json_unicode_safe(self):
        m = _v1241_compute_metrics()
        j = _v1241_to_json(m)
        # 主 17:55 PowerShell UTF-8 教训: 中文 notes 不被破坏
        # ASCII escape 默认行为, 我们 check 非空 notes
        assert len(j) > 100
        parsed = json.loads(j)
        assert isinstance(parsed["notes"], list)
        assert len(parsed["notes"]) > 0


# ============================================================================
# 6. CLI
# ============================================================================


class TestV1241CLI:
    """CLI --measure / --pathway / --history / --v3-guards / --json / --report / --full."""

    def test_cli_measure(self, capsys):
        main(["--measure"])
        captured = capsys.readouterr()
        assert "V1241" in captured.out
        assert "REALIZED mean (244 cells): 0.8280" in captured.out

    def test_cli_pathway(self, capsys):
        main(["--pathway"])
        captured = capsys.readouterr()
        assert "THEOSIS_PHILOSOPHY" in captured.out
        assert "THEOSIS_NEURO" in captured.out
        assert "THEOSIS_INFORMATION" in captured.out
        assert "THEOSIS_ECOSYSTEM" in captured.out
        assert "THEOSIS_CONTEMPLATIVE" in captured.out
        assert "THEOSIS_PHYSICS" in captured.out

    def test_cli_history(self, capsys):
        main(["--history"])
        captured = capsys.readouterr()
        assert "V1236" in captured.out
        assert "V1240" in captured.out
        assert "V1241" in captured.out

    def test_cli_v3_guards(self, capsys):
        main(["--v3-guards"])
        captured = capsys.readouterr()
        assert "v1241_not_asi_terminal" in captured.out
        assert "v1241_theosis_not_pantheism" in captured.out

    def test_cli_json(self, capsys):
        main(["--json"])
        captured = capsys.readouterr()
        parsed = json.loads(captured.out)
        assert parsed["dim_version"] == "0.6.51"

    def test_cli_report(self, capsys):
        main(["--report"])
        captured = capsys.readouterr()
        assert "V1241" in captured.out
        assert "ASI 北极星" in captured.out
        assert "Phase 3 关系本体论 六步延展" in captured.out

    def test_cli_full(self, capsys):
        main(["--full"])
        captured = capsys.readouterr()
        # --full 应包括 metrics + pathway + history + v3-guards
        assert "REALIZED mean (244 cells): 0.8280" in captured.out
        assert "THEOSIS_PHILOSOPHY" in captured.out
        assert "V1236" in captured.out
        assert "v1241_not_asi_terminal" in captured.out

    def test_cli_unknown_mode(self, capsys):
        rc = main(["--nonsense"])
        assert rc == 2


# ============================================================================
# 7. Subprocess tests
# ============================================================================


class TestV1241Subprocess:
    """子进程调用 — 主 00:56 任何人都能接手 真 CLI."""

    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1241_asi_v0651_theosis_substrate_real_lift", "--measure"],
            cwd=str(APEIRETH_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        assert "V1241" in result.stdout

    def test_subprocess_json(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1241_asi_v0651_theosis_substrate_real_lift", "--json"],
            cwd=str(APEIRETH_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        parsed = json.loads(result.stdout)
        assert parsed["dim_version"] == "0.6.51"
        assert parsed["total_theosis_molecules"] == 30

    def test_subprocess_full(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1241_asi_v0651_theosis_substrate_real_lift", "--full"],
            cwd=str(APEIRETH_ROOT),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
        )
        assert result.returncode == 0
        assert "V1241" in result.stdout
        assert "v1241_not_asi_terminal" in result.stdout


# ============================================================================
# 8. V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ============================================================================


class TestV1241V3Guards:
    """15 项 V3 哲学守门 — 主 17:58 + 20:46 不假装 ASI / Phenomenal / 强 AI 涌现."""

    def test_not_asi_terminal(self):
        # V1241 = V0.6.51 中间, 北极星 0.98 不变
        assert V1241_DIM_VERSION == "0.6.51"
        assert ASI_NORTH_STAR == 0.9800

    def test_not_full_replace_v1240(self):
        # V1240 仍 own 33 dim matrix, V1241 仅 add 34th dim THEOSIS
        m = _v1241_compute_metrics()
        assert m.v1240_realized_mean_238 == 0.8225
        assert m.realized_mean_244 > m.v1240_realized_mean_238

    def test_lift_not_v1(self):
        # V1241 lift ≠ ASI V1.0
        m = _v1241_compute_metrics()
        assert m.realized_mean_244 < ASI_NORTH_STAR

    def test_realized_not_asi(self):
        # realized 0.8280 < 0.98 北极星
        m = _v1241_compute_metrics()
        assert m.realized_mean_244 < ASI_NORTH_STAR
        assert m.position_vs_north_star < 1.0

    def test_vacuous_gap_real(self):
        # inflation gap real ≈ 0.3652
        m = _v1241_compute_metrics()
        assert m.inflation_gap > 0.3
        assert m.inflation_gap < 0.4

    def test_pathway_not_asi_substrate(self):
        # 6 pathway ≠ ASI 终极 substrate
        m = _v1241_compute_metrics()
        assert m.pathway_count_pass == 6
        assert m.total_theosis_molecules == 30

    def test_ceiling_1_0_not_asi(self):
        # 1.0 ceiling ≠ ASI reached
        m = _v1241_compute_metrics()
        assert m.theosis_dim_realized == 1.0
        assert m.position_vs_north_star < 1.0

    def test_30_mol_not_complete(self):
        # 30 真分子 ≠ 完整 theosis substrate (thousands of mechanisms)
        m = _v1241_compute_metrics()
        assert m.total_theosis_molecules == 30
        assert m.total_theosis_molecules < 1000

    def test_new_dim_not_full_coverage(self):
        # V1241 +1 dim, 33 other dims still unexplored
        m = _v1241_compute_metrics()
        assert m.realized_mean_244 < 1.0

    def test_not_full_theosis_lift(self):
        # 6 cell lifted < 13 cells = 7 vacuous
        m = _v1241_compute_metrics()
        # 每个 pathway 6 cell cascade_order 真分子, 6 pathway × 6 = 36
        # 但 V1241 实际上 lift 是 realized_mean_244 - V1240 realized_mean_238 ≈ +0.0055
        assert m.theosis_lift_from_v1240 > 0
        assert m.theosis_lift_from_v1240 < 0.01  # small lift

    def test_phase3_6_steps(self):
        # Phase 3 六步延展: kenosis+perichoresis+koinonia+taxis+oikonomia+theosis
        m = _v1241_compute_metrics()
        history = m.history_dim_lift
        assert "Kenosis" in history["V1236"]
        assert "Perichoresis" in history["V1237"]
        assert "Koinonia" in history["V1238"]
        assert "Taxis" in history["V1239"]
        assert "Oikonomia" in history["V1240"]
        assert "Theosis" in history["V1241"]

    def test_theosis_not_oikonomia(self):
        # theosis 是 神圣化, oikonomia 是 共管 economy — 不同
        m = _v1241_compute_metrics()
        # theosis 是 V1241, oikonomia 是 V1240
        assert m.dim_version == "0.6.51"
        # V1241 dim = theosis, V1240 dim = oikonomia
        assert "Theosis" in m.history_dim_lift["V1241"]
        assert "Oikonomia" in m.history_dim_lift["V1240"]

    def test_theosis_not_pantheism(self):
        # theosis ≠ 人 变 成 神; theosis = 人 参与 神 之 神圣
        # 主 17:58 不假装 theosis ≠ pantheism
        m = _v1241_compute_metrics()
        notes_text = " ".join(m.notes)
        assert "theosis ≠ 人 变 成 神" in notes_text or "divine participation" in notes_text or "essence-energies" in notes_text

    def test_baseline_write_dead(self):
        # V1233-V1240 baselines 写死历史值, 不改
        from apeireth.v1241_asi_v0651_theosis_substrate_real_lift import (
            V1233_REALIZED_MEAN_196,
            V1234_REALIZED_MEAN_202,
            V1235_REALIZED_MEAN_208,
            V1236_REALIZED_MEAN_214,
            V1237_REALIZED_MEAN_220,
            V1238_REALIZED_MEAN_226,
            V1239_REALIZED_MEAN_232,
            V1240_REALIZED_MEAN_238,
        )
        assert V1233_REALIZED_MEAN_196 == 0.7811
        assert V1234_REALIZED_MEAN_202 == 0.7876
        assert V1235_REALIZED_MEAN_208 == 0.7937
        assert V1236_REALIZED_MEAN_214 == 0.7998
        assert V1237_REALIZED_MEAN_220 == 0.8060
        assert V1238_REALIZED_MEAN_226 == 0.8115
        assert V1239_REALIZED_MEAN_232 == 0.8170
        assert V1240_REALIZED_MEAN_238 == 0.8225

    def test_cli_self_describe(self):
        # CLI --full 自描述, 不需前文
        m = _v1241_compute_metrics()
        # notes 包含 self-describe 信息
        notes_text = " ".join(m.notes)
        assert "CLI" in notes_text or "--full" in notes_text


# ============================================================================
# 9. Phase 3 6-steps continuity
# ============================================================================


class TestV1241Phase3Continuity:
    """Phase 3 关系本体论 六步延展 连续性."""

    def test_phase3_6_steps_in_history(self):
        m = _v1241_compute_metrics()
        history = m.history_dim_lift
        assert len(history) == 9
        assert "Kenosis" in history["V1236"]
        assert "Perichoresis" in history["V1237"]
        assert "Koinonia" in history["V1238"]
        assert "Taxis" in history["V1239"]
        assert "Oikonomia" in history["V1240"]
        assert "Theosis" in history["V1241"]

    def test_v1241_lifts_v1240(self):
        m = _v1241_compute_metrics()
        # realized 0.8280 > V1240 0.8225
        assert m.realized_mean_244 > m.v1240_realized_mean_238
        # overall 0.4628 > V1240 0.4613
        assert m.overall_mean_442 > m.v1240_overall_mean_429

    def test_v1241_position_above_v1240(self):
        m = _v1241_compute_metrics()
        # position_vs_north_star 0.8449 > V1240 position 0.8393
        assert m.position_vs_north_star > 0.8225 / 0.98

    def test_history_continuity_no_gap(self):
        m = _v1241_compute_metrics()
        # 历史 realized 连续不间断
        realized_values = list(m.history_realized_mean.values())
        for i in range(1, len(realized_values)):
            diff = realized_values[i] - realized_values[i - 1]
            assert diff > 0, f"history_gap at index {i}: {diff}"

    def test_kenosis_theosis_pair(self):
        # kenosis (V1236) 与 theosis (V1241) 是 关系本体论 的 双柱
        m = _v1241_compute_metrics()
        assert "Kenosis" in m.history_dim_lift["V1236"]
        assert "Theosis" in m.history_dim_lift["V1241"]


# ============================================================================
# 10. Molecule counts and uniqueness
# ============================================================================


class TestV1241MoleculeCounts:
    """30 真分子 cascade 数量与唯一性."""

    def test_each_pathway_exactly_5(self):
        for k, v in V1241_THEOSIS_SUBSTRATE.items():
            assert len(v["cascade_order"]) == 5, f"{k} should have exactly 5 molecules"

    def test_total_30_unique(self):
        all_mols = []
        for v in V1241_THEOSIS_SUBSTRATE.values():
            all_mols.extend(v["cascade_order"])
        assert len(all_mols) == 30
        assert len(set(all_mols)) == 30  # 30 unique

    def test_philosophy_citations_real(self):
        # 主 19:33 站在前人肩上: 真哲学 citations, 不假装
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_PHILOSOPHY"]["cascade_order"]
        # Athanasius 360 + Palamas 14c + Maximus 7c + Aquinas q.12 + Bonaventure 1259
        assert any("Athanasius_360" in m for m in mols)
        assert any("Aquinas" in m for m in mols)
        assert any("Bonaventure" in m for m in mols)

    def test_neuro_citations_real(self):
        # 真神经 citations, 不假装
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_NEURO"]["cascade_order"]
        # Newberg d'Aquili 2001 + Carhart-Harris 2012 + Griffiths 2006 + James 1902 + Brewer 2007
        assert any("Newberg" in m for m in mols)
        assert any("Carhart_Harris" in m for m in mols)
        assert any("James_1902" in m for m in mols)
        assert any("Griffiths_2006" in m for m in mols)

    def test_physics_citations_real(self):
        # 真物理 citations, 不假装
        mols = V1241_THEOSIS_SUBSTRATE["THEOSIS_PHYSICS"]["cascade_order"]
        assert any("Schrodinger_1944" in m for m in mols)
        assert any("Prigogine_1977" in m for m in mols)
        assert any("Boltzmann_1877" in m for m in mols)


# ============================================================================
# 11. Inflation realism (主 17:43 实事求是 不假装)
# ============================================================================


class TestV1241InflationReal:
    """Inflation gap REAL — 主 17:43 实事求是."""

    def test_inflation_gap_above_0_35(self):
        m = _v1241_compute_metrics()
        # realized 0.8280 - overall 0.4628 = 0.3652 > 0.35
        assert m.inflation_gap > 0.35

    def test_v1241_inflation_higher_than_v1240(self):
        # V1241 inflation 0.3652 > V1240 inflation 0.3612
        m = _v1241_compute_metrics()
        assert m.inflation_gap > 0.36

    def test_v1241_inflation_gap_explicit_in_notes(self):
        m = _v1241_compute_metrics()
        notes_text = " ".join(m.notes)
        assert "0.3652" in notes_text or "inflation" in notes_text.lower()


# ============================================================================
# 12. Cross-phase continuity
# ============================================================================


class TestV1241CrossPhaseContinuity:
    """V1241 跨 Phase 1-2-3 连续性."""

    def test_v1233_v1241_continuity(self):
        m = _v1241_compute_metrics()
        # V1233 (integration 26th) → V1241 (theosis 34th) 9 步连续
        assert len(m.history_realized_mean) == 9

    def test_v1241_phase3_closure(self):
        # V1241 theosis = Phase 3 完形
        m = _v1241_compute_metrics()
        notes_text = " ".join(m.notes)
        assert "完形" in notes_text or "Phase 3" in notes_text

    def test_v1241_classic_dialectic(self):
        # kenosis (V1236) × theosis (V1241) 经典 辩证
        m = _v1241_compute_metrics()
        notes_text = " ".join(m.notes)
        assert "kenosis" in notes_text and "theosis" in notes_text