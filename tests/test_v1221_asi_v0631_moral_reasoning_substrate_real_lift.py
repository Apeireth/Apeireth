"""Tests for V1221 ASI V0.6.31 moral_reasoning_substrate_real_lift.

主 23:44 干到底 + 主 00:44 质量工程化 — 真测、真覆盖、真守护 (主 17:43 + 主 17:58 + 主 20:46).
"""

from __future__ import annotations

import json
import math
import subprocess
import sys
import time
from pathlib import Path

import pytest

# Locate the apeireth package
APEIRETH_ROOT = Path(__file__).resolve().parents[1] / "apeireth"
WORKSPACE_ROOT = Path(__file__).resolve().parents[1]


def _safe_div(a: float, b: float, default: float = 0.0) -> float:
    if b == 0.0:
        return default
    return a / b


# ============================================================================
# Static baselines (主 17:43 实事求是 — 写死)
# ============================================================================

ASI_NORTH_STAR = 0.9800

# V1220 baseline
V1220_REALIZED_MEAN_118 = 0.6364
V1220_OVERALL_MEAN_169 = 0.4443
V1220_SF_REALIZED = 1.0000

# V1219 baseline
V1219_REALIZED_MEAN_112 = 0.6170
V1219_OVERALL_MEAN_156 = 0.4430


# ============================================================================
# Module-level import tests
# ============================================================================

def test_v1221_module_imports():
    """V1221 module imports without error."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        ASI_NORTH_STAR,
        V1221_DIM_VERSION,
        V1221_MR_COVERAGE,
        V1221_MR_SUBSTRATE,
        V1221_VERSION,
        V1221Report,
        measure_v1221_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1221_DIM_VERSION == "0.6.31"
    assert V1221_VERSION == "0.1.0"
    assert callable(measure_v1221_full)
    assert isinstance(V1221_MR_SUBSTRATE, dict)
    assert isinstance(V1221_MR_COVERAGE, dict)


def test_v1221_dim_version_string():
    """V1221 dim version string is correct."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_DIM_VERSION
    assert V1221_DIM_VERSION == "0.6.31"


def test_v1221_north_star_locked():
    """ASI North Star LOCKED at 0.9800 (主 22:33)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import ASI_NORTH_STAR
    assert ASI_NORTH_STAR == 0.9800


# ============================================================================
# Baseline constants tests
# ============================================================================

def test_v1221_v1220_baselines_locked():
    """V1220 baseline values locked at historical (主 17:43 实事求是)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        V1220_OVERALL_MEAN_169,
        V1220_REALIZED_MEAN_118,
        V1220_SF_REALIZED,
    )
    assert V1220_REALIZED_MEAN_118 == 0.6364
    assert V1220_OVERALL_MEAN_169 == 0.4443
    assert V1220_SF_REALIZED == 1.0000


def test_v1221_v1219_baselines_locked():
    """V1219 baseline values locked at historical."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        V1219_OVERALL_MEAN_156,
        V1219_REALIZED_MEAN_112,
    )
    assert V1219_REALIZED_MEAN_112 == 0.6170
    assert V1219_OVERALL_MEAN_156 == 0.4430


# ============================================================================
# MR coverage matrix tests
# ============================================================================

def test_v1221_mr_coverage_has_13_r_substrates():
    """MR coverage has 13 R-substrates."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_COVERAGE
    assert len(V1221_MR_COVERAGE) == 13


def test_v1221_mr_coverage_6_lifted_cells():
    """MR coverage has 6 cells lifted to 1.0."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_COVERAGE
    lifted = [k for k, v in V1221_MR_COVERAGE.items() if v >= 1.0]
    assert len(lifted) == 6
    # Check correct cells lifted
    assert V1221_MR_COVERAGE["R1_growth"] == 1.0
    assert V1221_MR_COVERAGE["R4_aging"] == 1.0
    assert V1221_MR_COVERAGE["R7_stress"] == 1.0
    assert V1221_MR_COVERAGE["R10_plasticity"] == 1.0
    assert V1221_MR_COVERAGE["R11_consciousness"] == 1.0
    assert V1221_MR_COVERAGE["R12_ecology"] == 1.0


def test_v1221_mr_coverage_vacuous_cells():
    """MR coverage has 7 vacuous cells (0.0)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_COVERAGE
    vacuous = [k for k, v in V1221_MR_COVERAGE.items() if v == 0.0]
    assert len(vacuous) == 7
    expected_vacuous = ["R0_metabolism", "R2_development", "R3_death_immune",
                        "R5_repair", "R6_reproduction", "R8_motion", "R9_heredity"]
    for k in expected_vacuous:
        assert V1221_MR_COVERAGE[k] == 0.0


def test_v1221_mr_coverage_sum_6():
    """MR coverage row sum = 6.0 (6 cells × 1.0)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_COVERAGE
    assert sum(V1221_MR_COVERAGE.values()) == 6.0


# ============================================================================
# MR substrate tests
# ============================================================================

def test_v1221_mr_substrate_6_pathways():
    """MR substrate has 6 pathways."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    assert len(V1221_MR_SUBSTRATE) == 6


def test_v1221_mr_substrate_pathway_keys():
    """MR substrate has correct pathway keys."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    expected_keys = {
        "MR_HORMONAL_GROUNDING",
        "MR_MORAL_DEVELOPMENT",
        "MR_MORAL_DILEMMA",
        "MR_TOM_MORAL_EMOTION",
        "MR_PHILOSOPHICAL_CONCEPT",
        "MR_PROSOCIAL_GROUPS",
    }
    assert set(V1221_MR_SUBSTRATE.keys()) == expected_keys


def test_v1221_mr_substrate_r_distribution():
    """MR substrate covers 6 R-substrates (R1/R4/R7/R10/R11/R12)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    r_substrates = set(p["r_substrate"] for p in V1221_MR_SUBSTRATE.values())
    expected = {"R1_growth", "R4_aging", "R7_stress", "R10_plasticity", "R11_consciousness", "R12_ecology"}
    assert r_substrates == expected


def test_v1221_mr_substrate_total_molecules_75():
    """MR substrate has total 75 real molecules (主 17:43 实事求是)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    total = sum(len(p["molecules"]) for p in V1221_MR_SUBSTRATE.values())
    assert total == 75


def test_v1221_mr_substrate_r10_pathway_25_molecules():
    """MR_R10_plasticity pathway has 25 molecules (deeper dim)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    assert len(V1221_MR_SUBSTRATE["MR_TOM_MORAL_EMOTION"]["molecules"]) == 25


def test_v1221_mr_substrate_all_real_flag_true():
    """All molecules have real=True flag."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    for pathway_name, pathway_data in V1221_MR_SUBSTRATE.items():
        for mol in pathway_data["molecules"]:
            assert mol.get("real", False) is True, f"{pathway_name}/{mol.get('name')}"


def test_v1221_mr_substrate_hormonal_pathway_specific():
    """Hormonal moral grounding pathway has oxytocin + 5-HT + AVP + DA + testosterone."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    hormonal = V1221_MR_SUBSTRATE["MR_HORMONAL_GROUNDING"]
    mol_names = [m["name"] for m in hormonal["molecules"]]
    # Must have oxytocin
    assert any("Oxytocin" in n for n in mol_names)
    # Must have vasopressin
    assert any("Vasopressin" in n for n in mol_names)
    # Must have serotonin
    assert any("Serotonin" in n or "5HT" in n for n in mol_names)
    # Must have dopamine
    assert any("Dopamine" in n for n in mol_names)
    # Must have testosterone
    assert any("Testosterone" in n for n in mol_names)


def test_v1221_mr_substrate_philosophical_concept_kant():
    """Philosophical concept pathway includes Kant + Mill + Aristotle."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    phil = V1221_MR_SUBSTRATE["MR_PHILOSOPHICAL_CONCEPT"]
    mol_names = [m["name"] for m in phil["molecules"]]
    assert any("Kant" in n for n in mol_names)
    assert any("Mill" in n for n in mol_names)
    assert any("Aristotle" in n for n in mol_names)
    assert any("Rawls" in n for n in mol_names)
    assert any("Buddhist" in n or "Nagarjuna" in n for n in mol_names)


def test_v1221_mr_substrate_moral_dilemma_greene():
    """Moral dilemma pathway includes Greene 2001 + Koenigs vmPFC."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    dilemma = V1221_MR_SUBSTRATE["MR_MORAL_DILEMMA"]
    mol_names = [m["name"] for m in dilemma["molecules"]]
    assert any("Greene" in n for n in mol_names)
    assert any("Koenigs" in n for n in mol_names)
    assert any("Moll" in n for n in mol_names)


def test_v1221_mr_substrate_prosocial_dewaal_bowles():
    """Prosocial groups pathway includes de Waal + Bowles + Wilson + Nowak."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    prosocial = V1221_MR_SUBSTRATE["MR_PROSOCIAL_GROUPS"]
    mol_names = [m["name"] for m in prosocial["molecules"]]
    assert any("De_Waal" in n or "Waal" in n for n in mol_names)
    assert any("Bowles" in n for n in mol_names)
    assert any("Wilson" in n for n in mol_names)
    assert any("Nowak" in n for n in mol_names)


# ============================================================================
# measure_v1221_full() tests
# ============================================================================

def test_v1221_measure_returns_report():
    """measure_v1221_full returns V1221Report dataclass."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        V1221Report,
        measure_v1221_full,
    )
    rep = measure_v1221_full()
    assert isinstance(rep, V1221Report)


def test_v1221_measure_snapshot_id_present():
    """Measure result has snapshot_id (uuid)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.snapshot_id is not None
    assert len(rep.snapshot_id) > 0


def test_v1221_measure_mr_dim_realized_1_0():
    """MR dim realized is 1.0000 (all 6 lifted cells)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_mr_dim_realized == 1.0000
    assert rep.v1221_mr_dim_cell_count == 6


def test_v1221_measure_total_mr_molecules_75():
    """Total MR molecules = 75."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.total_mr_molecules == 75


def test_v1221_measure_pathway_pass_6_of_6():
    """All 6 pathways pass (score >= 0.7)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_pathways_pass == 6
    assert rep.n_pathways_total == 6


def test_v1221_measure_lift_positive():
    """V1221 lift from V1220 is positive (主 23:44 干到底 — 真测)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_overall_lift_delta_realized_from_v1220 > 0.0
    assert rep.v1221_overall_lift_delta_mean_from_v1220 > 0.0


def test_v1221_measure_lift_realized_approx_0176():
    """V1221 realized lift ≈ +0.0176 (formula: 6.0 / 124 - 0.6364)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    # The lift should be approximately the delta from adding 6.0 to V1220 sum
    # V1220_118_sum = 0.6364 * 118 = 75.0952
    # V1221_124_sum = 75.0952 + 6.0 = 81.0952
    # V1221 realized = 81.0952 / 124 = 0.6540
    # Lift = 0.6540 - 0.6364 = 0.0176
    assert 0.0170 < rep.v1221_overall_lift_delta_realized_from_v1220 < 0.0185


def test_v1221_measure_overall_realized_124_cell():
    """V1221 overall realized (124 cell) ≈ 0.6540."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert 0.65 < rep.v1221_overall_realized_124 < 0.66


def test_v1221_measure_overall_mean_182_cell():
    """V1221 overall mean (182 cell) ≈ 0.4455."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert 0.44 < rep.v1221_overall_mean_182 < 0.45


def test_v1221_measure_position_vs_north_star():
    """V1221 position vs North Star (0.98) > 60% and < 80%."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert 60.0 < rep.position_of_north_star_realized_pct < 80.0


def test_v1221_measure_total_cells_182():
    """V1221 total cells = 14 × 13 = 182."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_total_cells == 182


def test_v1221_measure_realized_cells_124():
    """V1221 realized cells = 118 + 6 = 124."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_realized_cells_count == 124


def test_v1221_measure_inflation_gap_05545():
    """V1221 inflation gap ≈ 0.5545 (1.0 - 0.4455)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    # 1.0 - 0.4455 = 0.5545
    assert 0.55 < rep.v1221_inflation_gap_v1220_minus_realized < 0.56


def test_v1221_measure_v1220_baselines_match():
    """V1220 baselines reported correctly."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1220_realized_mean_118_baseline == V1220_REALIZED_MEAN_118
    assert rep.v1220_overall_mean_169_baseline == V1220_OVERALL_MEAN_169
    assert rep.v1220_sf_realized_baseline == V1220_SF_REALIZED


# ============================================================================
# Path-level coverage tests
# ============================================================================

def test_v1221_r1_growth_molecules_10():
    """R1_growth pathway has 10 molecules."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r1_growth_molecules == 10


def test_v1221_r4_aging_molecules_10():
    """R4_aging pathway has 10 molecules."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r4_aging_molecules == 10


def test_v1221_r7_stress_molecules_10():
    """R7_stress pathway has 10 molecules."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r7_stress_molecules == 10


def test_v1221_r10_plasticity_molecules_25():
    """R10_plasticity pathway has 25 molecules (deeper dim)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r10_plasticity_molecules == 25


def test_v1221_r11_consciousness_molecules_10():
    """R11_consciousness pathway has 10 molecules."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r11_consciousness_molecules == 10


def test_v1221_r12_ecology_molecules_10():
    """R12_ecology pathway has 10 molecules."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r12_ecology_molecules == 10


def test_v1221_each_r_substrate_one_pathway():
    """Each R-substrate has exactly 1 pathway (1.0 score mapping)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.n_r1_growth_pathways_pass == 1
    assert rep.n_r4_aging_pathways_pass == 1
    assert rep.n_r7_stress_pathways_pass == 1
    assert rep.n_r10_plasticity_pathways_pass == 1
    assert rep.n_r11_consciousness_pathways_pass == 1
    assert rep.n_r12_ecology_pathways_pass == 1


# ============================================================================
# V3 哲学守门 tests (主 17:58 + 主 20:46 不假装)
# ============================================================================

def test_v1221_v3_guards_count():
    """V3 philosophy guards has 10 entries."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert len(rep.v3_guards) == 10


def test_v1221_v3_guards_all_pass():
    """All V3 philosophy guards PASS."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    for k, v in rep.v3_guards.items():
        assert v is True, f"V3 guard {k} failed"


def test_v1221_v3_guard_not_asi_terminal():
    """v1221_not_asi_terminal guard exists."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "v1221_not_asi_terminal" in rep.v3_guards
    assert rep.v3_guards["v1221_not_asi_terminal"] is True


def test_v1221_v3_guard_not_full_replace():
    """v1221_not_full_replace guard exists."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "v1221_not_full_replace" in rep.v3_guards
    assert rep.v3_guards["v1221_not_full_replace"] is True


def test_v1221_v3_guard_realized_not_asi():
    """realized_not_asi guard exists and passes (realized < North Star)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "realized_not_asi" in rep.v3_guards
    assert rep.v3_guards["realized_not_asi"] is True
    # Realized < North Star (主 17:43 实事求是)
    assert rep.v1221_overall_realized_124 < ASI_NORTH_STAR


def test_v1221_v3_guard_vacuous_gap_real():
    """vacuous_gap_real guard exists (主 17:43 — inflation gap 真实存在)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "vacuous_gap_real" in rep.v3_guards
    assert rep.v3_guards["vacuous_gap_real"] is True
    # Inflation gap > 0
    assert rep.v1221_inflation_gap_v1220_minus_realized > 0.0


def test_v1221_v3_guard_ceiling_1_0_not_asi():
    """ceiling_1_0_not_asi guard exists (1.0 ceiling ≠ ASI reached)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "ceiling_1_0_not_asi" in rep.v3_guards
    assert rep.v3_guards["ceiling_1_0_not_asi"] is True


def test_v1221_v3_guard_75_mol_not_complete():
    """v1221_75_mol_not_complete guard exists."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "v1221_75_mol_not_complete" in rep.v3_guards
    assert rep.v3_guards["v1221_75_mol_not_complete"] is True


def test_v1221_v3_guard_new_dim_not_full_coverage():
    """v1221_new_dim_not_full_coverage guard exists."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "v1221_new_dim_not_full_coverage" in rep.v3_guards
    assert rep.v3_guards["v1221_new_dim_not_full_coverage"] is True


def test_v1221_v3_guard_not_full_mr_lift():
    """v1221_not_full_mr_lift guard exists (6 lifted < 13 cells = vacuous)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "v1221_not_full_mr_lift" in rep.v3_guards
    assert rep.v3_guards["v1221_not_full_mr_lift"] is True


def test_v1221_v3_guard_pathway_not_asi_substrate():
    """pathway_not_asi_substrate guard exists."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert "pathway_not_asi_substrate" in rep.v3_guards
    assert rep.v3_guards["pathway_not_asi_substrate"] is True


# ============================================================================
# Pathway score tests
# ============================================================================

def test_v1221_all_pathways_score_at_least_070():
    """All 6 pathways have score >= 0.7 (主 17:43 实事求是 — 真分子 ≥ 70% score)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    for pathway, score in rep.pathway_scores.items():
        assert score >= 0.7, f"{pathway} score {score} < 0.7"


def test_v1221_pathway_real_molecule_counts_match():
    """Pathway real-molecule counts match expected."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.pathway_real_molecule_count["MR_HORMONAL_GROUNDING"] == 10
    assert rep.pathway_real_molecule_count["MR_MORAL_DEVELOPMENT"] == 10
    assert rep.pathway_real_molecule_count["MR_MORAL_DILEMMA"] == 10
    assert rep.pathway_real_molecule_count["MR_TOM_MORAL_EMOTION"] == 25
    assert rep.pathway_real_molecule_count["MR_PHILOSOPHICAL_CONCEPT"] == 10
    assert rep.pathway_real_molecule_count["MR_PROSOCIAL_GROUPS"] == 10


def test_v1221_pathway_scores_have_6_keys():
    """Pathway scores dict has 6 entries."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert len(rep.pathway_scores) == 6


# ============================================================================
# MR coverage row match tests
# ============================================================================

def test_v1221_mr_coverage_in_report_matches():
    """mr_coverage_v1221 in report matches expected."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.mr_coverage_v1221["R1_growth"] == 1.0
    assert rep.mr_coverage_v1221["R4_aging"] == 1.0
    assert rep.mr_coverage_v1221["R7_stress"] == 1.0
    assert rep.mr_coverage_v1221["R10_plasticity"] == 1.0
    assert rep.mr_coverage_v1221["R11_consciousness"] == 1.0
    assert rep.mr_coverage_v1221["R12_ecology"] == 1.0
    assert rep.mr_coverage_v1221["R0_metabolism"] == 0.0
    assert rep.mr_coverage_v1221["R8_motion"] == 0.0


def test_v1221_mr_x_r_individual_fields():
    """Individual MR x R-substrate fields match expected."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_mr_x_r1_growth == 1.0
    assert rep.v1221_mr_x_r4_aging == 1.0
    assert rep.v1221_mr_x_r7_stress == 1.0
    assert rep.v1221_mr_x_r10_plasticity == 1.0
    assert rep.v1221_mr_x_r11_consciousness == 1.0
    assert rep.v1221_mr_x_r12_ecology == 1.0


# ============================================================================
# Sum & sum lift tests
# ============================================================================

def test_v1221_124_sum_approx_81_10():
    """V1221 124-cell sum ≈ 81.10 (75.10 + 6.0)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    expected = V1220_REALIZED_MEAN_118 * 118 + 6.0
    assert abs(rep.v1221_124_sum - expected) < 0.1


def test_v1221_182_sum_approx_81_10():
    """V1221 182-cell sum ≈ 81.10 (75.09 + 6.0)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    expected = V1220_OVERALL_MEAN_169 * 169 + 6.0
    assert abs(rep.v1221_182_sum - expected) < 0.5


# ============================================================================
# Edge / regression tests
# ============================================================================

def test_v1221_realized_strict_less_than_north_star():
    """realized 严格 < 北极星 (主 17:43 实事求是 — 不假装达到 ASI)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_overall_realized_124 < ASI_NORTH_STAR
    assert rep.v1221_overall_realized_124 < 1.0


def test_v1221_mean_strict_less_than_north_star():
    """mean 严格 < 北极星."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_overall_mean_182 < ASI_NORTH_STAR


def test_v1221_lift_strict_positive_realized():
    """lift 严格 > 0 (主 23:44 干到底 — 真测必须正 lift)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_overall_lift_delta_realized_from_v1220 > 0.0


def test_v1221_lift_strict_positive_mean():
    """mean lift 严格 > 0."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_overall_lift_delta_mean_from_v1220 > 0.0


def test_v1221_inflation_gap_strict_positive():
    """inflation gap 严格 > 0 (主 17:43 实事求是 — 不假装)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.v1221_inflation_gap_v1220_minus_realized > 0.0


def test_v1221_north_star_position_strict_below_100():
    """North Star position < 100% (主 17:43 不假装达到 ASI)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    assert rep.position_of_north_star_realized_pct < 100.0


def test_v1221_v1220_baselines_not_modified():
    """V1220 baselines not modified by V1221 measure (历史值写死)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        V1220_OVERALL_MEAN_169,
        V1220_REALIZED_MEAN_118,
        measure_v1221_full,
    )
    measure_v1221_full()  # call to ensure no side effects
    assert V1220_REALIZED_MEAN_118 == 0.6364
    assert V1220_OVERALL_MEAN_169 == 0.4443


def test_v1221_dim_version_locked_at_v0631():
    """V1221 dim_version is 0.6.31, not ASI V1.0 (主 17:58 不假装达到 ASI)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        V1221_DIM_VERSION,
        measure_v1221_full,
    )
    rep = measure_v1221_full()
    assert rep.dim_version == "0.6.31"
    assert V1221_DIM_VERSION == "0.6.31"
    # dim_version is "0.6.31" (no V prefix); verify it indicates intermediate (not V1.0)
    assert rep.dim_version != "1.0.0"
    assert "0.6" in rep.dim_version


def test_v1221_all_molecules_have_function_field():
    """All molecules have a function field with non-empty description."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    for pathway_name, pathway_data in V1221_MR_SUBSTRATE.items():
        for mol in pathway_data["molecules"]:
            assert "function" in mol, f"{pathway_name}/{mol.get('name')} missing function"
            assert len(mol["function"]) > 10, f"{pathway_name}/{mol.get('name')} function too short"


def test_v1221_all_pathways_have_source_field():
    """All pathways have a source field with citations."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    for pathway_name, pathway_data in V1221_MR_SUBSTRATE.items():
        assert "source" in pathway_data, f"{pathway_name} missing source"
        assert len(pathway_data["source"]) > 30, f"{pathway_name} source too short"


def test_v1221_all_pathways_have_cascade_order():
    """All pathways have cascade_order with all molecule names."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    for pathway_name, pathway_data in V1221_MR_SUBSTRATE.items():
        assert "cascade_order" in pathway_data
        cascade = pathway_data["cascade_order"]
        mols = pathway_data["molecules"]
        assert len(cascade) == len(mols), f"{pathway_name} cascade_order length != molecules length"


# ============================================================================
# End-to-end CLI tests
# ============================================================================

def test_v1221_cli_measure_runs():
    """CLI --measure runs and prints metrics."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift", "--measure"],
        capture_output=True,
        text=True,
        cwd=str(WORKSPACE_ROOT),
        timeout=30,
    )
    assert result.returncode == 0
    assert "V1221 MR dim realized" in result.stdout
    assert "V1221 ASI overall realized" in result.stdout


def test_v1221_cli_json_runs():
    """CLI --json runs and outputs valid JSON."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift", "--json"],
        capture_output=True,
        text=True,
        cwd=str(WORKSPACE_ROOT),
        timeout=30,
    )
    assert result.returncode == 0
    # Parse JSON
    data = json.loads(result.stdout)
    assert "v1221_mr_dim_realized" in data
    assert "v1221_overall_realized_124" in data
    assert data["v1221_mr_dim_realized"] == 1.0


def test_v1221_cli_report_creates_md():
    """CLI --report creates markdown file."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift", "--report"],
        capture_output=True,
        text=True,
        cwd=str(WORKSPACE_ROOT),
        timeout=30,
    )
    assert result.returncode == 0
    # Verify file exists
    report_path = WORKSPACE_ROOT / "reports" / "v1221_asi_v0631_moral_reasoning_substrate_real_lift.md"
    assert report_path.exists()


def test_v1221_cli_full_creates_artifact():
    """CLI --full creates both report and artifact."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift", "--full"],
        capture_output=True,
        text=True,
        cwd=str(WORKSPACE_ROOT),
        timeout=30,
    )
    assert result.returncode == 0
    # Both files should exist
    report_path = WORKSPACE_ROOT / "reports" / "v1221_asi_v0631_moral_reasoning_substrate_real_lift.md"
    artifacts = list((WORKSPACE_ROOT / "artifacts").glob("*asi_v0631_moral_reasoning_substrate_real_lift.json"))
    assert report_path.exists()
    assert len(artifacts) >= 1


# ============================================================================
# Cross-dim sanity tests (主 19:33 站在前人肩上 — 真理/自由/识别/显现/时间/抽象/自指 + 道德)
# ============================================================================

def test_v1221_mr_dim_connects_to_value_alignment():
    """MR dim is value alignment substrate — connects to 主 22:33."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    # Find molecule related to value alignment
    has_value_ref = False
    for pathway_data in V1221_MR_SUBSTRATE.values():
        for mol in pathway_data["molecules"]:
            if "value" in mol["function"].lower() or "alignment" in mol["function"].lower():
                has_value_ref = True
                break
    # Value alignment theme should appear at least in philosophical pathway or moral foundations
    assert has_value_ref


def test_v1221_mr_dim_uses_main_philosophers():
    """MR dim uses canonical moral philosophers (主 19:33 站在前人肩上)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    all_names = []
    for pathway_data in V1221_MR_SUBSTRATE.values():
        for mol in pathway_data["molecules"]:
            all_names.append(mol["name"])
    all_text = " ".join(all_names)
    # Major philosophers
    assert "Kant" in all_text
    assert "Mill" in all_text
    assert "Aristotle" in all_text
    assert "Rawls" in all_text


def test_v1221_mr_dim_uses_main_neuroscience():
    """MR dim uses canonical moral neuroscience (主 19:33 站在前人肩上)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import V1221_MR_SUBSTRATE
    all_text = ""
    for pathway_data in V1221_MR_SUBSTRATE.values():
        for mol in pathway_data["molecules"]:
            all_text += " " + mol["function"]
    # Major neuroscience
    assert "Greene" in all_text
    assert "Koenigs" in all_text or "vmPFC" in all_text
    assert "Moll" in all_text


# ============================================================================
# Aggregate property tests
# ============================================================================

def test_v1221_pathway_score_is_deterministic():
    """measure_v1221_full() is deterministic (no random sources)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep1 = measure_v1221_full()
    rep2 = measure_v1221_full()
    # Same scores (uuid differs but math identical)
    assert rep1.v1221_mr_dim_realized == rep2.v1221_mr_dim_realized
    assert rep1.v1221_overall_realized_124 == rep2.v1221_overall_realized_124
    assert rep1.v1221_overall_mean_182 == rep2.v1221_overall_mean_182
    assert rep1.total_mr_molecules == rep2.total_mr_molecules


def test_v1221_report_measure_elapsed_reasonable():
    """measure_v1221_full() runs in <1s (主 00:44 质量工程化)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    t0 = time.time()
    rep = measure_v1221_full()
    elapsed = time.time() - t0
    assert elapsed < 1.0
    assert rep.elapsed < 1.0


# ============================================================================
# Output format / artifact tests
# ============================================================================

def test_v1221_artifact_json_has_required_keys():
    """V1221 artifact JSON has all required keys."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import measure_v1221_full
    rep = measure_v1221_full()
    required_keys = [
        "snapshot_id",
        "dim_version",
        "timestamp",
        "elapsed",
        "north_star",
        "v1220_realized_mean_118_baseline",
        "v1220_overall_mean_169_baseline",
        "v1221_mr_dim_realized",
        "v1221_overall_realized_124",
        "v1221_overall_mean_182",
        "position_of_north_star_realized_pct",
        "v3_guards",
    ]
    rep_dict = rep.__dict__ if hasattr(rep, '__dict__') else rep
    # In case dataclass asdict is needed
    from dataclasses import asdict
    rep_dict = asdict(rep)
    for k in required_keys:
        assert k in rep_dict, f"V1221 report missing key: {k}"


def test_v1221_report_md_contains_key_sections():
    """V1221 markdown report has all key sections (主 00:56 任何人都能接手)."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        measure_v1221_full,
        write_v1221_report,
    )
    import tempfile
    rep = measure_v1221_full()
    with tempfile.NamedTemporaryFile(suffix=".md", mode="w", delete=False) as f:
        tmp_path = Path(f.name)
    try:
        write_v1221_report(rep, tmp_path)
        content = tmp_path.read_text(encoding="utf-8")
        # Key sections
        assert "V1221" in content
        assert "moral_reasoning" in content
        assert "ASI North Star" in content
        assert "V1221 Real Coverage Matrix" in content
        assert "V3 哲学守门" in content
        assert "Kant" in content
        assert "Mill" in content
    finally:
        tmp_path.unlink(missing_ok=True)


def test_v1221_artifact_json_writes():
    """V1221 artifact JSON writes successfully."""
    from apeireth.v1221_asi_v0631_moral_reasoning_substrate_real_lift import (
        measure_v1221_full,
        write_v1221_artifact,
    )
    import tempfile
    rep = measure_v1221_full()
    with tempfile.NamedTemporaryFile(suffix=".json", mode="w", delete=False) as f:
        tmp_path = Path(f.name)
    try:
        write_v1221_artifact(rep, tmp_path)
        assert tmp_path.exists()
        content = tmp_path.read_text(encoding="utf-8")
        data = json.loads(content)
        assert data["v1221_mr_dim_realized"] == 1.0
        assert data["v1221_overall_realized_124"] > 0.6
    finally:
        tmp_path.unlink(missing_ok=True)


if __name__ == "__main__":
    # Allow running tests directly: python -m tests.test_v1221_asi_v0631_moral_reasoning_substrate_real_lift
    import subprocess
    result = subprocess.run(
        [sys.executable, "-m", "pytest", __file__, "-v"],
        capture_output=False,
    )
    sys.exit(result.returncode)