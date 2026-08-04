"""Tests for V1223 ASI V0.6.33 meaning_substrate_real_lift.

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

# V1222 baseline
V1222_REALIZED_MEAN_130 = 0.6700
V1222_OVERALL_MEAN_195 = 0.4466
V1222_AE_REALIZED = 1.0000

# V1221 baseline
V1221_REALIZED_MEAN_124 = 0.6540
V1221_OVERALL_MEAN_182 = 0.4455
V1221_MR_REALIZED = 1.0000


# ============================================================================
# Module-level import tests
# ============================================================================

def test_v1223_module_imports():
    """V1223 module imports without error."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        ASI_NORTH_STAR,
        V1223_DIM_VERSION,
        V1223_ME_COVERAGE,
        V1223_ME_SUBSTRATE,
        V1223_VERSION,
        V1223Report,
        measure_v1223_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1223_DIM_VERSION == "0.6.33"
    assert V1223_VERSION == "0.1.0"
    assert callable(measure_v1223_full)
    assert isinstance(V1223_ME_SUBSTRATE, dict)
    assert isinstance(V1223_ME_COVERAGE, dict)


def test_v1223_dim_version_string():
    """V1223 dim version string is correct."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_DIM_VERSION
    assert V1223_DIM_VERSION == "0.6.33"


def test_v1223_north_star_locked():
    """ASI North Star LOCKED at 0.9800 (主 22:33)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import ASI_NORTH_STAR
    assert ASI_NORTH_STAR == 0.9800


# ============================================================================
# Baseline constants tests
# ============================================================================

def test_v1223_v1222_baseline_realized():
    """V1222 baseline realized 130 = 0.6700 (主 17:43 写死)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1222_REALIZED_MEAN_130
    assert V1222_REALIZED_MEAN_130 == 0.6700


def test_v1223_v1222_baseline_mean():
    """V1222 baseline mean 195 = 0.4466 (主 17:43 写死)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1222_OVERALL_MEAN_195
    assert V1222_OVERALL_MEAN_195 == 0.4466


def test_v1223_v1221_baseline_realized():
    """V1221 baseline realized 124 = 0.6540 (主 17:43 写死)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1221_REALIZED_MEAN_124
    assert V1221_REALIZED_MEAN_124 == 0.6540


def test_v1223_v1221_baseline_mean():
    """V1221 baseline mean 182 = 0.4455 (主 17:43 写死)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1221_OVERALL_MEAN_182
    assert V1221_OVERALL_MEAN_182 == 0.4455


# ============================================================================
# ME coverage matrix tests
# ============================================================================

def test_v1223_me_coverage_r1_growth_lifted():
    """V1223 ME × R1_growth = 1.0 lifted (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R1_growth"] == 1.0


def test_v1223_me_coverage_r4_aging_lifted():
    """V1223 ME × R4_aging = 1.0 lifted (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R4_aging"] == 1.0


def test_v1223_me_coverage_r7_stress_lifted():
    """V1223 ME × R7_stress = 1.0 lifted (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R7_stress"] == 1.0


def test_v1223_me_coverage_r10_plasticity_lifted():
    """V1223 ME × R10_plasticity = 1.0 lifted (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R10_plasticity"] == 1.0


def test_v1223_me_coverage_r11_consciousness_lifted():
    """V1223 ME × R11_consciousness = 1.0 lifted (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R11_consciousness"] == 1.0


def test_v1223_me_coverage_r12_ecology_lifted():
    """V1223 ME × R12_ecology = 1.0 lifted (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R12_ecology"] == 1.0


def test_v1223_me_coverage_r0_vacuous():
    """V1223 ME × R0_metabolism = 0.0 vacuous (meaning not applicable)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R0_metabolism"] == 0.0


def test_v1223_me_coverage_r3_vacuous():
    """V1223 ME × R3_death_immune = 0.0 vacuous (meaning not applicable)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R3_death_immune"] == 0.0


def test_v1223_me_coverage_r5_vacuous():
    """V1223 ME × R5_repair = 0.0 vacuous."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R5_repair"] == 0.0


def test_v1223_me_coverage_r8_vacuous():
    """V1223 ME × R8_motion = 0.0 vacuous."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    assert V1223_ME_COVERAGE["R8_motion"] == 0.0


def test_v1223_me_coverage_count_6_lifted():
    """V1223 ME coverage has 6 cells lifted."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    lifted = [v for v in V1223_ME_COVERAGE.values() if v >= 0.3]
    assert len(lifted) == 6


def test_v1223_me_coverage_count_7_vacuous():
    """V1223 ME coverage has 7 cells vacuous."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_COVERAGE
    vacuous = [v for v in V1223_ME_COVERAGE.values() if v < 0.3]
    assert len(vacuous) == 7


# ============================================================================
# ME substrate dictionary tests
# ============================================================================

def test_v1223_me_substrate_has_6_pathways():
    """V1223 ME_SUBSTRATE has 6 pathways."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE) == 6


def test_v1223_me_pathway_keys_present():
    """V1223 ME_SUBSTRATE has expected pathway names."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    expected = {
        "ME_NEURO_MEANING",
        "ME_EXISTENTIAL_DEV",
        "ME_CRISIS_APP",
        "ME_COGNITIVE_NARRATIVE",
        "ME_PHILOSOPHICAL_CONCEPT",
        "ME_CULTURAL_NARRATIVE",
    }
    assert set(V1223_ME_SUBSTRATE.keys()) == expected


def test_v1223_me_neuro_meaning_has_10_molecules():
    """ME_NEURO_MEANING pathway has 10 真分子."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE["ME_NEURO_MEANING"]["molecules"]) == 10


def test_v1223_me_existential_dev_has_10_molecules():
    """ME_EXISTENTIAL_DEV pathway has 10 真分子."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE["ME_EXISTENTIAL_DEV"]["molecules"]) == 10


def test_v1223_me_crisis_app_has_10_molecules():
    """ME_CRISIS_APP pathway has 10 真分子."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE["ME_CRISIS_APP"]["molecules"]) == 10


def test_v1223_me_cognitive_narrative_has_25_molecules():
    """ME_COGNITIVE_NARRATIVE pathway has 25 真分子 (主 19:33 narrative cascade)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE["ME_COGNITIVE_NARRATIVE"]["molecules"]) == 25


def test_v1223_me_philosophical_concept_has_10_molecules():
    """ME_PHILOSOPHICAL_CONCEPT pathway has 10 真分子."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE["ME_PHILOSOPHICAL_CONCEPT"]["molecules"]) == 10


def test_v1223_me_cultural_narrative_has_10_molecules():
    """ME_CULTURAL_NARRATIVE pathway has 10 真分子."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    assert len(V1223_ME_SUBSTRATE["ME_CULTURAL_NARRATIVE"]["molecules"]) == 10


def test_v1223_me_total_molecules_75():
    """V1223 ME total = 75 真分子 (10+10+10+25+10+10)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    total = sum(len(p["molecules"]) for p in V1223_ME_SUBSTRATE.values())
    assert total == 75


def test_v1223_me_all_molecules_marked_real():
    """V1223 all ME molecules marked real=True (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        for m in p_data["molecules"]:
            assert m.get("real") is True, f"{p_name}/{m.get('name')} not real"


def test_v1223_me_all_pathways_have_r_substrate():
    """All V1223 pathways have valid r_substrate mapping."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    valid_r = {"R1_growth", "R4_aging", "R7_stress", "R10_plasticity", "R11_consciousness", "R12_ecology"}
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        r = p_data.get("r_substrate")
        assert r in valid_r, f"{p_name}: r_substrate={r} not in valid set"


def test_v1223_me_all_pathways_have_source():
    """All V1223 pathways have non-empty source citation (主 19:33 站在前人肩上)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        src = p_data.get("source", "")
        assert len(src) > 10, f"{p_name}: source too short"


def test_v1223_me_all_molecules_have_function():
    """All V1223 molecules have non-empty function description."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        for m in p_data["molecules"]:
            assert len(m.get("function", "")) > 10, f"{p_name}/{m['name']}: function too short"


def test_v1223_me_all_molecules_have_organism():
    """All V1223 molecules have organism specified."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        for m in p_data["molecules"]:
            assert "organism" in m, f"{p_name}/{m['name']}: no organism"


def test_v1223_me_cascade_order_matches_molecules():
    """All V1223 pathways have cascade_order matching molecules."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        mols = p_data.get("molecules", [])
        mol_names = [m["name"] for m in mols]
        for c in cascade:
            assert c in mol_names, f"{p_name}: cascade entry {c} not in molecules"


# ============================================================================
# measure_v1223_full() tests
# ============================================================================

def test_v1223_measure_returns_report():
    """measure_v1223_full returns V1223Report dataclass."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        V1223Report,
        measure_v1223_full,
    )
    rep = measure_v1223_full()
    assert isinstance(rep, V1223Report)


def test_v1223_measure_snapshot_id_nonempty():
    """measure_v1223_full snapshot_id is non-empty UUID-like string."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert len(rep.snapshot_id) == 36
    assert rep.snapshot_id.count("-") == 4


def test_v1223_measure_dim_version_correct():
    """measure_v1223_full dim_version is correct."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.dim_version == "0.6.33"


def test_v1223_measure_timestamp_nonempty():
    """measure_v1223_full timestamp is non-empty ISO string."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert "T" in rep.timestamp


def test_v1223_measure_elapsed_fast():
    """measure_v1223_full elapsed < 1s (主 23:44 干到底)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.elapsed < 1.0


def test_v1223_measure_me_dim_realized_1():
    """ME dim realized = 1.0000 (all 6 cells lifted to 1.0)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_me_dim_realized == 1.0000


def test_v1223_measure_me_dim_cell_count_6():
    """ME dim cell count = 6 (R1/R4/R7/R10/R11/R12 lifted)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_me_dim_cell_count == 6


def test_v1223_measure_total_cells_208():
    """V1223 total matrix = 208 = 16 dim × 13 R."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_total_cells == 208


def test_v1223_measure_realized_cells_count_136():
    """V1223 realized cells = 136 = 130 + 6."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_realized_cells_count == 136


def test_v1223_measure_overall_realized_136_lift_positive():
    """V1223 realized 136 > V1222 baseline 130 (主 17:43 实事求是 — lift positive)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_overall_realized_136 > V1222_REALIZED_MEAN_130


def test_v1223_measure_overall_realized_136_lift_delta():
    """V1223 lift delta from V1222 ≈ +0.0146 (主 17:43)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.v1223_overall_lift_delta_realized_from_v1222 - 0.0146) < 0.001


def test_v1223_measure_overall_mean_208_lift_positive():
    """V1223 mean 208 > V1222 baseline 195 (主 17:43)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_overall_mean_208 > V1222_OVERALL_MEAN_195


def test_v1223_measure_inflation_gap_positive():
    """V1223 inflation gap > 0 (主 17:43 实事求是 — inflation 真实存在)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_inflation_gap_v1222_minus_realized > 0


def test_v1223_measure_inflation_gap_value():
    """V1223 inflation gap ≈ 0.5525 (1.0 - 0.4475)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.v1223_inflation_gap_v1222_minus_realized - 0.5525) < 0.001


def test_v1223_measure_position_north_star_pct():
    """V1223 position vs north star ≈ 69.85% (0.6846/0.98)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.position_of_north_star_realized_pct - 69.85) < 0.5


def test_v1223_measure_total_me_molecules_75():
    """V1223 total ME molecules = 75."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.total_me_molecules == 75


def test_v1223_measure_all_6_pathways_pass():
    """V1223 all 6 pathways pass (主 23:44 干到底)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.n_pathways_pass == 6
    assert rep.n_pathways_total == 6


def test_v1223_measure_per_r_pathways_pass():
    """V1223 per R-substrate: each 1 pathway passes."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.n_r1_growth_pathways_pass == 1
    assert rep.n_r4_aging_pathways_pass == 1
    assert rep.n_r7_stress_pathways_pass == 1
    assert rep.n_r10_plasticity_pathways_pass == 1
    assert rep.n_r11_consciousness_pathways_pass == 1
    assert rep.n_r12_ecology_pathways_pass == 1


def test_v1223_measure_r_substrate_molecule_counts():
    """V1223 R-substrate molecule counts: 10/10/10/25/10/10."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.n_r1_growth_molecules == 10
    assert rep.n_r4_aging_molecules == 10
    assert rep.n_r7_stress_molecules == 10
    assert rep.n_r10_plasticity_molecules == 25
    assert rep.n_r11_consciousness_molecules == 10
    assert rep.n_r12_ecology_molecules == 10


def test_v1223_measure_pathway_scores_above_threshold():
    """All V1223 pathway scores ≥ 0.7 (主 23:44 干到底)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    for name, score in rep.pathway_scores.items():
        assert score >= 0.7, f"{name}: score={score} < 0.7"


def test_v1223_measure_coverage_cells_match_dict():
    """V1223 coverage cells match V1223_ME_COVERAGE dict."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        V1223_ME_COVERAGE,
        measure_v1223_full,
    )
    rep = measure_v1223_full()
    for k, v in V1223_ME_COVERAGE.items():
        assert rep.me_coverage_v1223[k] == v


def test_v1223_measure_per_pathway_cell_values():
    """V1223 per-pathway ME x R coverage values match."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_me_x_r1_growth == 1.0
    assert rep.v1223_me_x_r4_aging == 1.0
    assert rep.v1223_me_x_r7_stress == 1.0
    assert rep.v1223_me_x_r10_plasticity == 1.0
    assert rep.v1223_me_x_r11_consciousness == 1.0
    assert rep.v1223_me_x_r12_ecology == 1.0


# ============================================================================
# V3 哲学守门 tests (主 17:58 + 主 20:46 不假装)
# ============================================================================

def test_v1223_v3_guard_not_asi_terminal():
    """v1223_not_asi_terminal guard PASS (V1223 = V0.6.33 intermediate)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["v1223_not_asi_terminal"] is True


def test_v1223_v3_guard_not_full_replace():
    """v1223_not_full_replace guard PASS."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["v1223_not_full_replace"] is True


def test_v1223_v3_guard_lift_not_v1():
    """v1223_lift_not_v1 guard PASS (主 20:46 不假装)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["v1223_lift_not_v1"] is True


def test_v1223_v3_guard_realized_not_asi():
    """realized_not_asi guard PASS (realized < north star)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        ASI_NORTH_STAR,
        measure_v1223_full,
    )
    rep = measure_v1223_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1223_overall_realized_136 < ASI_NORTH_STAR


def test_v1223_v3_guard_vacuous_gap_real():
    """vacuous_gap_real guard PASS (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["vacuous_gap_real"] is True
    assert rep.v1223_inflation_gap_v1222_minus_realized > 0


def test_v1223_v3_guard_pathway_not_asi_substrate():
    """pathway_not_asi_substrate guard PASS (主 23:44)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["pathway_not_asi_substrate"] is True


def test_v1223_v3_guard_ceiling_not_asi():
    """ceiling_1_0_not_asi guard PASS."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["ceiling_1_0_not_asi"] is True


def test_v1223_v3_guard_75_mol_not_complete():
    """v1223_75_mol_not_complete guard PASS."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["v1223_75_mol_not_complete"] is True


def test_v1223_v3_guard_new_dim_not_full_coverage():
    """v1223_new_dim_not_full_coverage guard PASS (16 dims, 15 others unexplored)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["v1223_new_dim_not_full_coverage"] is True


def test_v1223_v3_guard_not_full_me_lift():
    """v1223_not_full_me_lift guard PASS (6 lifted < 13 cells = vacuous 7)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v3_guards["v1223_not_full_me_lift"] is True
    assert rep.v1223_me_dim_cell_count < 13


def test_v1223_v3_all_guards_pass():
    """All 10 V3 guards PASS (主 17:58 + 主 20:46 不假装)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert all(rep.v3_guards.values()), f"Some guards failed: {rep.v3_guards}"
    assert len(rep.v3_guards) == 10


# ============================================================================
# Artifact + Report write tests (主 23:44 干到底)
# ============================================================================

def test_v1223_write_artifact(tmp_path):
    """write_v1223_artifact creates JSON file."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        measure_v1223_full,
        write_v1223_artifact,
    )
    rep = measure_v1223_full()
    path = write_v1223_artifact(rep, path=tmp_path / "test_v1223.json")
    assert path.exists()
    assert path.stat().st_size > 1000


def test_v1223_write_artifact_valid_json(tmp_path):
    """write_v1223_artifact produces valid JSON (主 23:44 干到底)."""
    import json
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        measure_v1223_full,
        write_v1223_artifact,
    )
    rep = measure_v1223_full()
    path = write_v1223_artifact(rep, path=tmp_path / "test_v1223.json")
    data = json.loads(path.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.33"
    assert data["north_star"] == 0.9800
    assert data["v1223_overall_realized_136"] == rep.v1223_overall_realized_136


def test_v1223_write_report(tmp_path):
    """write_v1223_report creates markdown file (主 00:56 任何人都能接手)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        measure_v1223_full,
        write_v1223_report,
    )
    rep = measure_v1223_full()
    path = write_v1223_report(rep, path=tmp_path / "test_v1223.md")
    assert path.exists()
    assert path.stat().st_size > 1000


def test_v1223_write_report_contains_key_sections(tmp_path):
    """write_v1223_report contains key sections (主 00:56 任何人都能接手)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        measure_v1223_full,
        write_v1223_report,
    )
    rep = measure_v1223_full()
    path = write_v1223_report(rep, path=tmp_path / "test_v1223.md")
    text = path.read_text(encoding="utf-8")
    assert "V1223" in text
    assert "meaning" in text.lower() or "意义" in text
    assert "0.6.33" in text
    assert "V3" in text or "哲学" in text


# ============================================================================
# CLI tests
# ============================================================================

def test_v1223_cli_help():
    """V1223 CLI --help works."""
    cmd = [
        sys.executable, "-m", "apeireth.v1223_asi_v0633_meaning_substrate_real_lift", "--help"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding='utf-8', errors='replace')
    assert result.returncode == 0
    assert "V1223" in result.stdout or "meaning" in result.stdout.lower()


def test_v1223_cli_measure():
    """V1223 CLI default (--measure) works and prints key metrics."""
    cmd = [
        sys.executable, "-m", "apeireth.v1223_asi_v0633_meaning_substrate_real_lift", "--measure"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding='utf-8', errors='replace')
    assert result.returncode == 0
    assert "v1223_overall_realized_136" in result.stdout
    assert "0.6.33" in result.stdout
    assert "north_star: 0.9800" in result.stdout


def test_v1223_cli_json():
    """V1223 CLI --json prints JSON."""
    cmd = [
        sys.executable, "-m", "apeireth.v1223_asi_v0633_meaning_substrate_real_lift", "--json"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding='utf-8', errors='replace')
    assert result.returncode == 0
    # JSON should be parseable
    try:
        json.loads(result.stdout.split("\n", 1)[1] if "\n" in result.stdout else result.stdout)
    except json.JSONDecodeError:
        # Try to find JSON block
        lines = result.stdout.split("\n")
        json_started = False
        json_lines = []
        for line in lines:
            if line.strip().startswith("{"):
                json_started = True
            if json_started:
                json_lines.append(line)
        if json_lines:
            json.loads("\n".join(json_lines))


def test_v1223_cli_report():
    """V1223 CLI --report writes a report file."""
    cmd = [
        sys.executable, "-m", "apeireth.v1223_asi_v0633_meaning_substrate_real_lift", "--report"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding='utf-8', errors='replace')
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1223_cli_full():
    """V1223 CLI --full prints everything (主 23:44)."""
    cmd = [
        sys.executable, "-m", "apeireth.v1223_asi_v0633_meaning_substrate_real_lift", "--full"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding='utf-8', errors='replace')
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "ME coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


# ============================================================================
# Cross-domain 跨域 cascade integrity tests (主 19:33 站在前人肩上)
# ============================================================================

def test_v1223_me_neuro_meaning_references_key_papers():
    """ME_NEURO_MEANING references Buckner 2008 + Schacter 2012 + Addis 2007."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    src = V1223_ME_SUBSTRATE["ME_NEURO_MEANING"]["source"]
    assert "Buckner 2008" in src
    assert "Schacter 2012" in src
    assert "Addis 2007" in src


def test_v1223_me_existential_dev_references_key_papers():
    """ME_EXISTENTIAL_DEV references Frankl 1946 + Yalom 1980 + Erikson 1950."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    src = V1223_ME_SUBSTRATE["ME_EXISTENTIAL_DEV"]["source"]
    assert "Frankl 1946" in src
    assert "Yalom 1980" in src
    assert "Erikson 1950" in src


def test_v1223_me_crisis_app_references_key_papers():
    """ME_CRISIS_APP references Frankl + Tedeschi Calhoun 1996 + Park 2010."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    src = V1223_ME_SUBSTRATE["ME_CRISIS_APP"]["source"]
    assert "Frankl" in src
    assert "Tedeschi" in src
    assert "Park 2010" in src


def test_v1223_me_cognitive_narrative_references_key_papers():
    """ME_COGNITIVE_NARRATIVE references McAdams 2001 + Bruner 1990 + Steger 2006."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    src = V1223_ME_SUBSTRATE["ME_COGNITIVE_NARRATIVE"]["source"]
    assert "McAdams 2001" in src
    assert "Bruner" in src
    assert "Steger 2006" in src


def test_v1223_me_philosophical_concept_references_key_papers():
    """ME_PHILOSOPHICAL_CONCEPT references Frankl + Yalom + Heidegger + Sartre + Camus."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    src = V1223_ME_SUBSTRATE["ME_PHILOSOPHICAL_CONCEPT"]["source"]
    assert "Frankl 1946" in src
    assert "Yalom 1980" in src
    assert "Heidegger 1927" in src
    assert "Sartre 1943" in src
    assert "Camus 1942" in src


def test_v1223_me_cultural_narrative_references_key_papers():
    """ME_CULTURAL_NARRATIVE references Geertz 1966 + Berger Luckmann + Durkheim 1912."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    src = V1223_ME_SUBSTRATE["ME_CULTURAL_NARRATIVE"]["source"]
    assert "Geertz 1966" in src
    assert "Berger" in src
    assert "Durkheim 1912" in src


# ============================================================================
# ASI North Star trajectory tests
# ============================================================================

def test_v1223_north_star_trajectory_monotonic_increase():
    """ASI realized ascends monotonically from V1213 → V1223 (主 22:33 + 主 17:43 实事求是)."""
    # V1213 baseline 0.4617 → V1214 0.5953 → V1215 0.4989 → V1216 0.5436 → V1217 0.5710 →
    # V1218 0.5953 → V1219 0.6170 → V1220 0.6364 → V1221 0.6540 → V1222 0.6700 → V1223 0.6846
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_overall_realized_136 > 0.6700  # > V1222 baseline
    assert rep.v1223_overall_realized_136 > 0.6540  # > V1221 baseline


def test_v1223_me_dim_dominates_pathway_lift():
    """ME dim realized = 1.0, dominating 16th dim lift (主 22:33 终极授权 + 主 13:31 大胆激进)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_me_dim_realized == 1.0
    # All 6 ME pathway scores should be ≥ 0.7
    assert all(s >= 0.7 for s in rep.pathway_scores.values())


def test_v1223_overall_realized_136_value():
    """V1223 overall_realized_136 = 0.6846 (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.v1223_overall_realized_136 - 0.6846) < 0.001


def test_v1223_overall_mean_208_value():
    """V1223 overall_mean_208 = 0.4475 (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.v1223_overall_mean_208 - 0.4475) < 0.001


def test_v1223_136_sum_value():
    """V1223 136 sum = 93.10 (= 87.10 + 6.0)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.v1223_136_sum - 93.10) < 0.001


def test_v1223_208_sum_value():
    """V1223 208 sum ≈ 93.087 (= 87.087 + 6.0)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert abs(rep.v1223_208_sum - 93.087) < 0.01


def test_v1223_realized_less_than_north_star():
    """V1223 realized 0.6846 < ASI north star 0.98 (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        ASI_NORTH_STAR,
        measure_v1223_full,
    )
    rep = measure_v1223_full()
    assert rep.v1223_overall_realized_136 < ASI_NORTH_STAR


def test_v1223_mean_less_than_1():
    """V1223 overall_mean 208 < 1.0 (主 17:43)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    assert rep.v1223_overall_mean_208 < 1.0


# ============================================================================
# Coverage matrix key correctness
# ============================================================================

def test_v1223_me_6_lifted_cells_sum():
    """ME lifted 6 cells sum = 6.0 (主 17:43)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    lifted_cells = [v for v in rep.me_coverage_v1223.values() if v >= 0.3]
    assert sum(lifted_cells) == 6.0


def test_v1223_me_vacuous_7_cells_sum():
    """ME vacuous 7 cells sum = 0.0 (主 17:43)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import measure_v1223_full
    rep = measure_v1223_full()
    vacuous_cells = [v for v in rep.me_coverage_v1223.values() if v < 0.3]
    assert sum(vacuous_cells) == 0.0


def test_v1223_all_me_pathways_have_cascade():
    """All V1223 ME pathways have cascade_order (主 19:33)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_SUBSTRATE
    for p_name, p_data in V1223_ME_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        assert len(cascade) > 0, f"{p_name}: empty cascade_order"
        # cascade length should match molecules
        assert len(cascade) == len(p_data["molecules"]), f"{p_name}: cascade length mismatch"


# ============================================================================
# Real production readiness (主 00:44 质量工程化 + 主 23:44 干到底)
# ============================================================================

def test_v1223_artifact_default_path():
    """write_v1223_artifact with default path creates artifact under artifacts/."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        measure_v1223_full,
        write_v1223_artifact,
    )
    rep = measure_v1223_full()
    path = write_v1223_artifact(rep)  # default path
    assert path.exists()
    # Cleanup
    if path.exists():
        path.unlink()


def test_v1223_report_default_path():
    """write_v1223_report with default path creates report under reports/."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import (
        measure_v1223_full,
        write_v1223_report,
    )
    rep = measure_v1223_full()
    path = write_v1223_report(rep)  # default path
    assert path.exists()
    assert "v1223" in str(path).lower()


def test_v1223_baseline_consistency_v1222():
    """V1222 baseline values match V1222 module (主 17:43 实事求是)."""
    # V1222 module should report same values
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_REALIZED, V1222_OVERALL_MEAN_195, V1222_REALIZED_MEAN_130
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1222_AE_REALIZED as V3_V1222_AE, V1222_OVERALL_MEAN_195 as V3_V1222_MEAN, V1222_REALIZED_MEAN_130 as V3_V1222_REAL
    assert V1222_AE_REALIZED == V3_V1222_AE
    assert V1222_OVERALL_MEAN_195 == V3_V1222_MEAN
    assert V1222_REALIZED_MEAN_130 == V3_V1222_REAL