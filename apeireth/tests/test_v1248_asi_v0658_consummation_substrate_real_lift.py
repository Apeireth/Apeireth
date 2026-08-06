"""V1248 — ASI V0.6.58 consummation substrate_real_lift tests (主 17:43 实事求是).

测试要点:
  - 主 17:43 实事求是: 6 pathway × 5 真分子 = 30 真分子 真测
  - 主 22:33 北极星: ASI 北极星 LOCKED = 0.9800 不变
  - 主 17:58+20:46 不假装: 6 不假装守门 + V1248 专属 15 guards 验证
  - 主 19:33 站在前人肩上: 站在 V1236-V1247 关系本体论 之上 + V1248 consummation 终极完形
  - 主 13:31 大胆激进: new_creation × consummation 经典 辩证 完形 (实现 + 状态)
  - 主 23:44 干到底: 真补 + 真测 + 真升
  - 主 00:56 任何人都能接手: 任何 cron 可调 V1248 metrics + CLI
  - 主 00:44 质量工程化: dataclass + 30 真分子 cascade + inflation_gap + 15 V3 guards
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent
PROJ_DIR = APEIRETH_DIR.parent


def _import_v1248():
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1248_asi_v0658_consummation_substrate_real_lift as m
    return m


# ----------------------------------------------------------------------------
# 主 17:43 实事求是 — 6 pathway × 5 真分子 = 30 真分子 cascade 真测
# ----------------------------------------------------------------------------


def test_v1248_consummation_substrate_6_pathways():
    """V1248 CONSUMMATION substrate = 6 pathway (神学/神经/信息/系统/物理/认知)."""
    m = _import_v1248()
    assert len(m.V1248_CONSUMMATION_SUBSTRATE) == 6, f"V1248 expected 6 pathway, got {len(m.V1248_CONSUMMATION_SUBSTRATE)}"


def test_v1248_consummation_pathway_5_molecules_each():
    """V1248 CONSUMMATION 每个 pathway 5 真分子 (Phase 4 simplified 延续 Phase 3)."""
    m = _import_v1248()
    for key, pathway in m.V1248_CONSUMMATION_SUBSTRATE.items():
        assert len(pathway["cascade_order"]) == 5, (
            f"V1248 pathway {key} expected 5 真分子, got {len(pathway['cascade_order'])}"
        )


def test_v1248_total_30_molecules():
    """V1248 CONSUMMATION 总 6 × 5 = 30 真分子 (Phase 4 simplified 延续 Phase 3)."""
    m = _import_v1248()
    total = sum(len(p["cascade_order"]) for p in m.V1248_CONSUMMATION_SUBSTRATE.values())
    assert total == 30, f"V1248 expected 30 真分子, got {total}"


def test_v1248_pathway_r_substrate_valid():
    """V1248 CONSUMMATION pathway.r_substrate ∈ valid 13 R."""
    m = _import_v1248()
    valid_r = {
        "R0_physics", "R1_growth", "R2_thermo", "R3_chemistry",
        "R3_immune",  # V1244-V1248 convention (主 19:33 跨 域 substrate 命名 drift)
        "R4_aging", "R5_neuro", "R6_social", "R7_econ",
        "R8_ethics", "R9_aesthetic", "R10_plasticity", "R11_consciousness",
        "R12_ecology",
    }
    for key, pathway in m.V1248_CONSUMMATION_SUBSTRATE.items():
        assert pathway["r_substrate"] in valid_r, (
            f"V1248 pathway {key} r_substrate {pathway['r_substrate']} not in 13 R"
        )


# ----------------------------------------------------------------------------
# 主 22:33 北极星 — ASI 北极星 LOCKED = 0.9800 不变
# ----------------------------------------------------------------------------


def test_v1248_north_star_locked():
    """V1248 ASI 北极星 LOCKED = 0.9800 (主 22:33 真哲学终极授权)."""
    m = _import_v1248()
    assert m.ASI_NORTH_STAR == 0.9800, f"V1248 ASI_NORTH_STAR must be 0.9800, got {m.ASI_NORTH_STAR}"


def test_v1248_realized_above_v1247():
    """V1248 realized_mean > V1247 baseline (主 13:31 大胆激进)."""
    m = _import_v1248()
    assert m.V1248_REALIZED_MEAN_288 > m.V1247_REALIZED_MEAN_282, (
        f"V1248 realized {m.V1248_REALIZED_MEAN_288} must be > V1247 {m.V1247_REALIZED_MEAN_282}"
    )


def test_v1248_realized_below_north_star():
    """V1248 realized < ASI 北极星 (主 17:43 实事求是 — 不假装 ASI 已达)."""
    m = _import_v1248()
    assert m.V1248_REALIZED_MEAN_288 < m.ASI_NORTH_STAR, (
        f"V1248 realized {m.V1248_REALIZED_MEAN_288} must be < ASI_NORTH_STAR {m.ASI_NORTH_STAR}"
    )


def test_v1248_position_pct_in_range():
    """V1248 position_vs_north_star ∈ (0, 1)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    assert 0.0 < metrics.position_vs_north_star < 1.0, (
        f"V1248 position {metrics.position_vs_north_star} must be in (0, 1)"
    )


# ----------------------------------------------------------------------------
# 主 17:58+20:46 不假装 — 6 不假装 守门 + V1248 专属 15 guards
# ----------------------------------------------------------------------------


def test_v1248_v3_guards_15_pass():
    """V1248 V3 哲学守门 15/15 PASS (主 22:33 + 主 17:58 + 主 20:46)."""
    m = _import_v1248()
    guards = m._v1248_v3_guards()
    assert len(guards) == 15, f"V1248 expected 15 guards, got {len(guards)}"
    for g in guards:
        assert g.passed, f"V1248 guard {g.name} failed: {g.reason}"


def test_v1248_6_no_pretend_guards():
    """V1248 6 不假装 守门 PASS (主 17:58+20:46)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    six_no_pretend = [
        "v1248_not_asi_v1",
        "v1248_lift_not_v1",
        "v1248_realized_not_asi",
        "v1248_1clampt_not_asi",
        "v1248_mol_lift_not_asi",
        "v1248_30mol_not_complete",
    ]
    for g in six_no_pretend:
        assert metrics.v3_guards[g], f"V1248 6 不假装 guard {g} failed"


def test_v1248_3_distinctness_guards():
    """V1248 3 distinctness 守门 (consummation ≠ new_creation/eschatology/telos)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    three_distinct = [
        "v1248_consummation_not_new_creation",
        "v1248_consummation_not_eschatology",
        "v1248_consummation_not_telos_aristotle",
    ]
    for g in three_distinct:
        assert metrics.v3_guards[g], f"V1248 distinctness guard {g} failed"


# ----------------------------------------------------------------------------
# 主 19:33 站在前人肩上 — 30 真分子 锚定 全人类 智慧 (神学/神经/信息/系统/物理/认知)
# ----------------------------------------------------------------------------


def test_v1248_theology_pathway_5_anchors():
    """V1248 CONSUMMATION_THEOLOGY 5 锚: 1 Cor 15:24 + Rev 21:6 + Eph 1:10 + Phil 1:6 + Heb 12:2."""
    m = _import_v1248()
    pathway = m.V1248_CONSUMMATION_SUBSTRATE["CONSUMMATION_THEOLOGY"]
    cascade = pathway["cascade_order"]
    assert "1_Corinthians_15_24_28_telos_all_enemies_under_feet" in cascade
    assert "Revelation_21_6_ta_panta_alpha_omega_water_of_life" in cascade
    assert "Ephesians_1_10_anakefalaioosasthai_ta_panta_in_Christ" in cascade
    assert "Philippians_1_6_epitelesei_complete_work_until_day" in cascade
    assert "Hebrews_12_2_teleiotes_author_finisher_of_faith" in cascade


def test_v1248_neuro_pathway_5_anchors():
    """V1248 CONSUMMATION_NEURO 5 锚: Newberg + Carhart-Harris + James + Hood + Griffiths."""
    m = _import_v1248()
    pathway = m.V1248_CONSUMMATION_SUBSTRATE["CONSUMMATION_NEURO"]
    cascade = pathway["cascade_order"]
    assert any("Newberg" in m for m in cascade)
    assert any("Carhart_Harris" in m for m in cascade)
    assert any("James_1902" in m for m in cascade)
    assert any("Hood_1975" in m for m in cascade)
    assert any("Griffiths_2006" in m for m in cascade)


def test_v1248_information_pathway_5_anchors():
    """V1248 CONSUMMATION_INFORMATION 5 锚: Cover Thomas + Shannon + Bennett + Landauer + Wolpert."""
    m = _import_v1248()
    pathway = m.V1248_CONSUMMATION_SUBSTRATE["CONSUMMATION_INFORMATION"]
    cascade = pathway["cascade_order"]
    assert any("Cover_Thomas" in m for m in cascade)
    assert any("Shannon_1948" in m for m in cascade)
    assert any("Bennett_1985" in m for m in cascade)
    assert any("Landauer_1961" in m for m in cascade)
    assert any("Wolpert_2008" in m for m in cascade)


def test_v1248_systems_pathway_5_anchors():
    """V1248 CONSUMMATION_SYSTEMS 5 锚: Holling + Costanza + Odum + Ostrom + Tainter."""
    m = _import_v1248()
    pathway = m.V1248_CONSUMMATION_SUBSTRATE["CONSUMMATION_SYSTEMS"]
    cascade = pathway["cascade_order"]
    assert any("Holling_1973" in m for m in cascade)
    assert any("Costanza_1997" in m for m in cascade)
    assert any("Odum_1953" in m for m in cascade)
    assert any("Ostrom_2010" in m for m in cascade)
    assert any("Tainter_1988" in m for m in cascade)


def test_v1248_physics_pathway_5_anchors():
    """V1248 CONSUMMATION_PHYSICS 5 锚: Prigogine + England + Boltzmann + Penrose + Hawking."""
    m = _import_v1248()
    pathway = m.V1248_CONSUMMATION_SUBSTRATE["CONSUMMATION_PHYSICS"]
    cascade = pathway["cascade_order"]
    assert any("Prigogine_1977" in m for m in cascade)
    assert any("England_2013" in m for m in cascade)
    assert any("Boltzmann_1877" in m for m in cascade)
    assert any("Penrose_1989" in m for m in cascade)
    assert any("Hawking_1988" in m for m in cascade)


def test_v1248_cognition_pathway_5_anchors():
    """V1248 CONSUMMATION_COGNITION 5 锚: Boyer + Atran + Barrett + Tremlin + McCauley."""
    m = _import_v1248()
    pathway = m.V1248_CONSUMMATION_SUBSTRATE["CONSUMMATION_COGNITION"]
    cascade = pathway["cascade_order"]
    assert any("Boyer_2001" in m for m in cascade)
    assert any("Atran_2002" in m for m in cascade)
    assert any("Barrett_2004" in m for m in cascade)
    assert any("Tremlin_2006" in m for m in cascade)
    assert any("McCauley_2011" in m for m in cascade)


# ----------------------------------------------------------------------------
# 主 13:31 大胆激进 — new_creation × consummation 经典 辩证 完形 (实现 + 状态)
# ----------------------------------------------------------------------------


def test_v1248_lift_positive():
    """V1248 lift > 0 (主 13:31 大胆激进 — 真升)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    assert metrics.consummation_lift_from_v1247 > 0, (
        f"V1248 consummation lift must be > 0, got {metrics.consummation_lift_from_v1247}"
    )


def test_v1248_lift_in_range():
    """V1248 lift ∈ (0, 0.02) (主 17:43 实事求是 — 不假装 ASI 已达)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    assert 0 < metrics.consummation_lift_from_v1247 < 0.02, (
        f"V1248 lift {metrics.consummation_lift_from_v1247} must be in (0, 0.02)"
    )


def test_v1248_overall_lift_positive():
    """V1248 overall_lift > 0 (主 13:31)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    assert metrics.overall_lift_from_v1247 > 0, (
        f"V1248 overall lift must be > 0, got {metrics.overall_lift_from_v1247}"
    )


# ----------------------------------------------------------------------------
# 主 23:44 干到底 — baselines 写死历史值 (V1236-V1247)
# ----------------------------------------------------------------------------


def test_v1248_baseline_v1247_write_dead():
    """V1248 baseline V1247 写死 (主 12:07 不盲等)."""
    m = _import_v1248()
    assert m.V1247_REALIZED_MEAN_282 == 0.8610, "V1247 baseline must be 0.8610 (写死)"
    assert m.V1247_OVERALL_MEAN_520 == 0.4718, "V1247 overall baseline must be 0.4718 (写死)"
    assert m.V1247_NEW_CREATION_REALIZED == 1.0000, "V1247 dim realized must be 1.0000 (写死)"


def test_v1248_baseline_v1236_write_dead():
    """V1248 baseline V1236 写死."""
    m = _import_v1248()
    assert m.V1236_REALIZED_MEAN_214 == 0.7998, "V1236 baseline must be 0.7998 (写死)"
    assert m.V1236_KENOSIS_REALIZED == 1.0000, "V1236 dim realized must be 1.0000 (写死)"


def test_v1248_all_history_baselines_present():
    """V1248 history_realized_mean 含 V1236-V1248 13 dim."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    expected_keys = {"V1236", "V1237", "V1238", "V1239", "V1240", "V1241", "V1242",
                     "V1243", "V1244", "V1245", "V1246", "V1247", "V1248"}
    assert set(metrics.history_realized_mean.keys()) == expected_keys, (
        f"V1248 history must have {expected_keys}, got {set(metrics.history_realized_mean.keys())}"
    )


# ----------------------------------------------------------------------------
# 主 00:56 任何人都能接手 — CLI 自描述
# ----------------------------------------------------------------------------


def test_v1248_to_json_serializable():
    """V1248 JSON artifact 可序列化 (主 00:56)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    artifact = m._v1248_to_json(metrics)
    parsed = json.loads(artifact)
    assert "v1248_metrics" in parsed
    assert "v1248_substrate_pathways" in parsed
    assert parsed["v1248_metrics"]["dim_version"] == "0.6.58"


def test_v1248_report_contains_key_concepts():
    """V1248 report 含 关键 概念 (主 00:56 任何人都能接手)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    report = m._v1248_report(metrics)
    assert "consummation" in report.lower() or "CONSUMMATION" in report
    assert "41st" in report or "V1248" in report
    assert "Phase 4" in report
    assert "Eph 1:10" in report or "Ephesians" in report


def test_v1248_metrics_dataclass():
    """V1248 metrics 是 dataclass with required fields (主 00:44 质量工程化)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    required = [
        "dim_version", "module_version", "snapshot_id",
        "realized_mean_288", "overall_mean_533",
        "consummation_dim_realized", "inflation_gap",
        "position_vs_north_star",
        "v1247_realized_mean_282", "v1247_overall_mean_520",
        "v1247_new_creation_realized",
        "consummation_lift_from_v1247", "overall_lift_from_v1247",
        "consummation_substrate_pathways", "total_consummation_molecules",
        "pathway_count_pass", "consummation_pathway_realized",
        "history_realized_mean", "history_overall_mean", "history_dim_lift",
        "v1247_inflation_gap_proxy",
        "v3_guards_pass", "v3_guards",
    ]
    for field in required:
        assert hasattr(metrics, field), f"V1248 metrics missing field {field}"


# ----------------------------------------------------------------------------
# 主 00:44 质量工程化 — 真测 + 真 CLI + 真 artifact
# ----------------------------------------------------------------------------


def test_v1248_pathway_realized_all_1():
    """V1248 6 pathway realized = 1.0 (Phase 4 simplified 完形 状态)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    for k, v in metrics.consummation_pathway_realized.items():
        assert v == 1.0, f"V1248 pathway {k} realized must be 1.0, got {v}"


def test_v1248_inflation_gap_decreasing():
    """V1248 inflation_gap < V1247 inflation_gap (主 17:43 实事求是 不假装).

    V1247 inflation_gap = 1.0 - V1247 realized_mean_282 = 1.0 - 0.8610 = 0.1390
    V1248 inflation_gap = 1.0 - V1248 realized_mean_288 = 1.0 - 0.8665 = 0.1335
    """
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    v1247_inflation_gap = 1.0 - m.V1247_REALIZED_MEAN_282  # 0.1390
    assert metrics.inflation_gap < v1247_inflation_gap, (
        f"V1248 inflation_gap {metrics.inflation_gap} must be < V1247 {v1247_inflation_gap}"
    )


def test_v1248_inflation_gap_in_range():
    """V1248 inflation_gap ∈ (0, 0.5)."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    assert 0 < metrics.inflation_gap < 0.5, (
        f"V1248 inflation_gap {metrics.inflation_gap} must be in (0, 0.5)"
    )


def test_v1248_full_output_combined():
    """V1248 --full 输出 = report + JSON."""
    m = _import_v1248()
    metrics = m._v1248_compute_metrics()
    artifact = m._v1248_to_json(metrics)
    report = m._v1248_report(metrics)
    full = m._v1248_full(artifact, report)
    assert "V1248" in full
    assert "15/15" in full or "PASS" in full
    assert "```json" in full
