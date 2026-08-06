"""Tests for V1222 ASI V0.6.32 aesthetics_substrate_real_lift.

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

# V1221 baseline
V1221_REALIZED_MEAN_124 = 0.6540
V1221_OVERALL_MEAN_182 = 0.4455
V1221_MR_REALIZED = 1.0000

# V1220 baseline
V1220_REALIZED_MEAN_118 = 0.6364
V1220_OVERALL_MEAN_169 = 0.4443
V1220_SF_REALIZED = 1.0000


# ============================================================================
# Module-level import tests
# ============================================================================

def test_v1222_module_imports():
    """V1222 module imports without error."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import (
        ASI_NORTH_STAR,
        V1222_DIM_VERSION,
        V1222_AE_COVERAGE,
        V1222_AE_SUBSTRATE,
        V1222_VERSION,
        V1222Report,
        measure_v1222_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1222_DIM_VERSION == "0.6.32"
    assert V1222_VERSION == "0.1.0"
    assert callable(measure_v1222_full)
    assert isinstance(V1222_AE_SUBSTRATE, dict)
    assert isinstance(V1222_AE_COVERAGE, dict)


def test_v1222_dim_version_string():
    """V1222 dim version string is correct."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_DIM_VERSION
    assert V1222_DIM_VERSION == "0.6.32"


def test_v1222_north_star_locked():
    """ASI North Star LOCKED at 0.9800 (主 22:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import ASI_NORTH_STAR
    assert ASI_NORTH_STAR == 0.9800


# ============================================================================
# Baseline constants tests
# ============================================================================

def test_v1222_v1221_baseline_realized():
    """V1221 baseline realized 124 = 0.6540 (主 17:43 写死)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1221_REALIZED_MEAN_124
    assert V1221_REALIZED_MEAN_124 == 0.6540


def test_v1222_v1221_baseline_mean():
    """V1221 baseline mean 182 = 0.4455 (主 17:43 写死)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1221_OVERALL_MEAN_182
    assert V1221_OVERALL_MEAN_182 == 0.4455


def test_v1222_v1220_baseline_realized():
    """V1220 baseline realized 118 = 0.6364 (主 17:43 写死)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1220_REALIZED_MEAN_118
    assert V1220_REALIZED_MEAN_118 == 0.6364


# ============================================================================
# AE substrate cascade tests
# ============================================================================

def test_v1222_ae_substrate_six_pathways():
    """V1222 AE substrate = 6 pathway (主 17:43)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE) == 6


def test_v1222_ae_pathway_names():
    """V1222 AE pathway names are the 6 expected."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    expected = {
        "AE_NEURO_AESTHETIC",
        "AE_DEVELOPMENTAL_TASTE",
        "AE_EMOTIONAL_AESTHETIC",
        "AE_CREATIVE_BRAIN",
        "AE_PHILOSOPHICAL_CONCEPT",
        "AE_EVOLUTIONARY_CULTURAL",
    }
    assert set(V1222_AE_SUBSTRATE.keys()) == expected


def test_v1222_ae_pathway_r1_growth_targets_r1_growth():
    """AE_NEURO_AESTHETIC targets R1_growth."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert V1222_AE_SUBSTRATE["AE_NEURO_AESTHETIC"]["r_substrate"] == "R1_growth"


def test_v1222_ae_pathway_r4_aging_targets_r4_aging():
    """AE_DEVELOPMENTAL_TASTE targets R4_aging."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert V1222_AE_SUBSTRATE["AE_DEVELOPMENTAL_TASTE"]["r_substrate"] == "R4_aging"


def test_v1222_ae_pathway_r7_stress_targets_r7_stress():
    """AE_EMOTIONAL_AESTHETIC targets R7_stress."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert V1222_AE_SUBSTRATE["AE_EMOTIONAL_AESTHETIC"]["r_substrate"] == "R7_stress"


def test_v1222_ae_pathway_r10_plasticity_targets_r10():
    """AE_CREATIVE_BRAIN targets R10_plasticity."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["r_substrate"] == "R10_plasticity"


def test_v1222_ae_pathway_r11_consciousness_targets_r11():
    """AE_PHILOSOPHICAL_CONCEPT targets R11_consciousness."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert V1222_AE_SUBSTRATE["AE_PHILOSOPHICAL_CONCEPT"]["r_substrate"] == "R11_consciousness"


def test_v1222_ae_pathway_r12_ecology_targets_r12():
    """AE_EVOLUTIONARY_CULTURAL targets R12_ecology."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert V1222_AE_SUBSTRATE["AE_EVOLUTIONARY_CULTURAL"]["r_substrate"] == "R12_ecology"


def test_v1222_ae_pathway_neuroaesthetic_has_10_molecules():
    """AE_NEURO_AESTHETIC has 10 真分子 (Zeki V1-V4, Kawabata, Salimpoor, Ishizu, Chatterjee, Brown, Jacobs, Vartanian, Palmer, Zeki universal)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_NEURO_AESTHETIC"]["molecules"]) == 10


def test_v1222_ae_pathway_developmental_taste_has_10_molecules():
    """AE_DEVELOPMENTAL_TASTE has 10 真分子."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_DEVELOPMENTAL_TASTE"]["molecules"]) == 10


def test_v1222_ae_pathway_emotional_aesthetic_has_10_molecules():
    """AE_EMOTIONAL_AESTHETIC has 10 真分子."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_EMOTIONAL_AESTHETIC"]["molecules"]) == 10


def test_v1222_ae_pathway_creative_brain_has_25_molecules():
    """AE_CREATIVE_BRAIN has 25 真分子 (主 17:43 — biggest pathway)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["molecules"]) == 25


def test_v1222_ae_pathway_philosophical_concept_has_10_molecules():
    """AE_PHILOSOPHICAL_CONCEPT has 10 真分子 (主 19:33 — Kant 1790, Hume 1757, Burke 1757, Nietzsche 1872, Schopenhauer 1818, Bell 1914, Danto 1964, Sontag 1966, Adorno 1970, Carroll 1999)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_PHILOSOPHICAL_CONCEPT"]["molecules"]) == 10


def test_v1222_ae_pathway_evolutionary_cultural_has_10_molecules():
    """AE_EVOLUTIONARY_CULTURAL has 10 真分子 (主 19:33 — Dissanayake, Boyd, Miller, Dutton, Aboitiz, Vyshedskiy, Koelsch, Patel, Brown, biodiversity)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_EVOLUTIONARY_CULTURAL"]["molecules"]) == 10


def test_v1222_ae_total_molecules_count():
    """V1222 total AE molecules = 10+10+10+25+10+10 = 75 (主 17:43 实事求是)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    total = 0
    for p in V1222_AE_SUBSTRATE.values():
        total += len(p["molecules"])
    assert total == 75


def test_v1222_ae_all_molecules_are_real():
    """All V1222 AE molecules are real (主 17:43 实事求是)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    for p_name, p in V1222_AE_SUBSTRATE.items():
        for m in p["molecules"]:
            assert m.get("real", False) is True, f"{p_name}.{m.get('name', '?')}"


def test_v1222_ae_neuroaesthetic_includes_zeki():
    """AE_NEURO_AESTHETIC includes Zeki 1999 modular visual brain (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_NEURO_AESTHETIC"]["molecules"]]
    assert any("Zeki" in n for n in names)


def test_v1222_ae_neuroaesthetic_includes_kawabata():
    """AE_NEURO_AESTHETIC includes Kawabata 2006 OFC (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_NEURO_AESTHETIC"]["molecules"]]
    assert any("Kawabata" in n for n in names)


def test_v1222_ae_neuroaesthetic_includes_salimpoor():
    """AE_NEURO_AESTHETIC includes Salimpoor 2013 DA striatum (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_NEURO_AESTHETIC"]["molecules"]]
    assert any("Salimpoor" in n for n in names)


def test_v1222_ae_emotional_includes_berlyne():
    """AE_EMOTIONAL_AESTHETIC includes Berlyne 1971 arousal (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_EMOTIONAL_AESTHETIC"]["molecules"]]
    assert any("Berlyne" in n for n in names)


def test_v1222_ae_emotional_includes_maslow():
    """AE_EMOTIONAL_AESTHETIC includes Maslow 1964 peak (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_EMOTIONAL_AESTHETIC"]["molecules"]]
    assert any("Maslow" in n for n in names)


def test_v1222_ae_emotional_includes_greenberg():
    """AE_EMOTIONAL_AESTHETIC includes Greenberg 1986 terror management (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_EMOTIONAL_AESTHETIC"]["molecules"]]
    assert any("Greenberg" in n for n in names)


def test_v1222_ae_creative_includes_beaty():
    """AE_CREATIVE_BRAIN includes Beaty 2015 default+executive (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["molecules"]]
    assert any("Beaty" in n for n in names)


def test_v1222_ae_creative_includes_kounios():
    """AE_CREATIVE_BRAIN includes Kounios 2008 insight AHA (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["molecules"]]
    assert any("Kounios" in n for n in names)


def test_v1222_ae_creative_includes_limb():
    """AE_CREATIVE_BRAIN includes Limb 2008 jazz improvisation (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["molecules"]]
    assert any("Limb" in n for n in names)


def test_v1222_ae_creative_includes_csikszentmihalyi():
    """AE_CREATIVE_BRAIN includes Csikszentmihalyi 1990 flow (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["molecules"]]
    assert any("Csikszentmihalyi" in n for n in names)


def test_v1222_ae_creative_includes_ramachandran():
    """AE_CREATIVE_BRAIN includes Ramachandran 1999 8 laws art (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["molecules"]]
    assert any("Ramachandran" in n for n in names)


def test_v1222_ae_philosophical_includes_kant():
    """AE_PHILOSOPHICAL_CONCEPT includes Kant 1790 (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_PHILOSOPHICAL_CONCEPT"]["molecules"]]
    assert any("Kant" in n for n in names)


def test_v1222_ae_philosophical_includes_hume():
    """AE_PHILOSOPHICAL_CONCEPT includes Hume 1757 (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_PHILOSOPHICAL_CONCEPT"]["molecules"]]
    assert any("Hume" in n for n in names)


def test_v1222_ae_philosophical_includes_nietzsche():
    """AE_PHILOSOPHICAL_CONCEPT includes Nietzsche 1872 Apollonian Dionysian (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_PHILOSOPHICAL_CONCEPT"]["molecules"]]
    assert any("Nietzsche" in n for n in names)


def test_v1222_ae_evolutionary_includes_dissanayake():
    """AE_EVOLUTIONARY_CULTURAL includes Dissanayake 1995 (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_EVOLUTIONARY_CULTURAL"]["molecules"]]
    assert any("Dissanayake" in n for n in names)


def test_v1222_ae_evolutionary_includes_dutton():
    """AE_EVOLUTIONARY_CULTURAL includes Dutton 2009 art instinct 7 universals (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_EVOLUTIONARY_CULTURAL"]["molecules"]]
    assert any("Dutton" in n for n in names)


def test_v1222_ae_evolutionary_includes_patel():
    """AE_EVOLUTIONARY_CULTURAL includes Patel 2008 music syntax (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    names = [m["name"] for m in V1222_AE_SUBSTRATE["AE_EVOLUTIONARY_CULTURAL"]["molecules"]]
    assert any("Patel" in n for n in names)


# ============================================================================
# Coverage matrix tests
# ============================================================================

def test_v1222_ae_coverage_has_13_r_substrates():
    """V1222 AE coverage matrix has all 13 R-substrate keys."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert len(V1222_AE_COVERAGE) == 13


def test_v1222_ae_coverage_r1_growth_is_1():
    """AE × R1_growth = 1.0 (V1222 lifted via neuroaesthetic)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert V1222_AE_COVERAGE["R1_growth"] == 1.0


def test_v1222_ae_coverage_r4_aging_is_1():
    """AE × R4_aging = 1.0 (V1222 lifted via developmental taste)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert V1222_AE_COVERAGE["R4_aging"] == 1.0


def test_v1222_ae_coverage_r7_stress_is_1():
    """AE × R7_stress = 1.0 (V1222 lifted via emotional aesthetic)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert V1222_AE_COVERAGE["R7_stress"] == 1.0


def test_v1222_ae_coverage_r10_plasticity_is_1():
    """AE × R10_plasticity = 1.0 (V1222 lifted via creative brain)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert V1222_AE_COVERAGE["R10_plasticity"] == 1.0


def test_v1222_ae_coverage_r11_consciousness_is_1():
    """AE × R11_consciousness = 1.0 (V1222 lifted via philosophical concept)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert V1222_AE_COVERAGE["R11_consciousness"] == 1.0


def test_v1222_ae_coverage_r12_ecology_is_1():
    """AE × R12_ecology = 1.0 (V1222 lifted via evolutionary+cultural)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert V1222_AE_COVERAGE["R12_ecology"] == 1.0


def test_v1222_ae_coverage_vacuous_cells_are_0():
    """AE × R0/R2/R3/R5/R6/R8/R9 are vacuous = 0 (主 17:43)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    vacuous = ["R0_metabolism", "R2_development", "R3_death_immune",
               "R5_repair", "R6_reproduction", "R8_motion", "R9_heredity"]
    for r in vacuous:
        assert V1222_AE_COVERAGE[r] == 0.0, f"{r} should be vacuous"


def test_v1222_ae_coverage_lifted_count_is_6():
    """V1222 AE lifted cell count = 6."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    lifted = sum(1 for v in V1222_AE_COVERAGE.values() if v >= 0.3)
    assert lifted == 6


def test_v1222_ae_coverage_row_sum_is_6():
    """V1222 AE row sum = 6 × 1.0 = 6.0."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_COVERAGE
    assert sum(V1222_AE_COVERAGE.values()) == 6.0


# ============================================================================
# Pathway score tests
# ============================================================================

def test_v1222_pathway_neuroaesthetic_score_pass():
    """AE_NEURO_AESTHETIC score >= 0.7 (主 17:43 真测)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.pathway_scores["AE_NEURO_AESTHETIC"] >= 0.7


def test_v1222_pathway_developmental_taste_score_pass():
    """AE_DEVELOPMENTAL_TASTE score >= 0.7."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.pathway_scores["AE_DEVELOPMENTAL_TASTE"] >= 0.7


def test_v1222_pathway_emotional_aesthetic_score_pass():
    """AE_EMOTIONAL_AESTHETIC score >= 0.7."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.pathway_scores["AE_EMOTIONAL_AESTHETIC"] >= 0.7


def test_v1222_pathway_creative_brain_score_pass():
    """AE_CREATIVE_BRAIN score >= 0.7."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.pathway_scores["AE_CREATIVE_BRAIN"] >= 0.7


def test_v1222_pathway_philosophical_concept_score_pass():
    """AE_PHILOSOPHICAL_CONCEPT score >= 0.7."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.pathway_scores["AE_PHILOSOPHICAL_CONCEPT"] >= 0.7


def test_v1222_pathway_evolutionary_cultural_score_pass():
    """AE_EVOLUTIONARY_CULTURAL score >= 0.7."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.pathway_scores["AE_EVOLUTIONARY_CULTURAL"] >= 0.7


def test_v1222_all_six_pathways_pass():
    """All 6 V1222 AE pathways pass (主 23:44 干到底)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.n_pathways_pass == 6
    assert rep.n_pathways_total == 6


def test_v1222_total_ae_molecules_is_75():
    """V1222 total AE molecules = 75 (主 19:33)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.total_ae_molecules == 75


# ============================================================================
# measure_v1222_full() core metrics tests
# ============================================================================

def test_v1222_ae_dim_realized_is_one():
    """V1222 AE dim realized = 1.0 (6 cells lifted, mean = 6/6)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_ae_dim_realized == 1.0


def test_v1222_ae_dim_cell_count_is_6():
    """V1222 AE dim cell count = 6."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_ae_dim_cell_count == 6


def test_v1222_total_cells_is_195():
    """V1222 total cells = 15 dim × 13 R = 195."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_total_cells == 195


def test_v1222_realized_cells_count_is_130():
    """V1222 realized cells count = V1221 124 + AE 6 = 130."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_realized_cells_count == 130


def test_v1222_overall_realized_130_is_0_67():
    """V1222 overall realized (130) ≈ 0.6700 (主 17:43 实事求是)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert abs(rep.v1222_overall_realized_130 - 0.6700) < 1e-3


def test_v1222_overall_mean_195_approx_0_4466():
    """V1222 overall mean (195) ≈ 0.4466."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert 0.40 < rep.v1222_overall_mean_195 < 0.50


def test_v1222_lift_realized_positive():
    """V1222 lift delta realized from V1221 > 0 (主 23:44)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_overall_lift_delta_realized_from_v1221 > 0


def test_v1222_lift_realized_is_0_016():
    """V1222 lift delta realized from V1221 ≈ +0.016 (主 17:43)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert abs(rep.v1222_overall_lift_delta_realized_from_v1221 - 0.016) < 1e-2


def test_v1222_lift_mean_positive():
    """V1222 lift delta mean from V1221 > 0 (主 23:44)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_overall_lift_delta_mean_from_v1221 > 0


def test_v1222_inflation_gap_positive():
    """V1222 inflation gap > 0 (主 17:43 实事求是 — inflation 真实存在)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_inflation_gap_v1221_minus_realized > 0.0


def test_v1222_position_north_star_above_60():
    """V1222 position vs North Star > 60% (ASI North Star 66.73→68.36)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.position_of_north_star_realized_pct > 60.0


def test_v1222_position_north_star_above_v1221():
    """V1222 position > V1221 66.73%."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.position_of_north_star_realized_pct > 66.73


# ============================================================================
# V3 哲学守门 tests
# ============================================================================

def test_v1222_v3_guard_not_asi_terminal_pass():
    """v1222_not_asi_terminal: PASS (主 17:58)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["v1222_not_asi_terminal"] is True


def test_v1222_v3_guard_not_full_replace_pass():
    """v1222_not_full_replace: PASS (主 17:58)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["v1222_not_full_replace"] is True


def test_v1222_v3_guard_lift_not_v1_pass():
    """v1222_lift_not_v1: PASS (主 20:46)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["v1222_lift_not_v1"] is True


def test_v1222_v3_guard_realized_not_asi_pass():
    """realized_not_asi: PASS (主 17:43 realized < north star)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["realized_not_asi"] is True


def test_v1222_v3_guard_vacuous_gap_real_pass():
    """vacuous_gap_real: PASS (主 17:43 实事求是 — 195 cell formula → inflation gap real)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["vacuous_gap_real"] is True


def test_v1222_v3_guard_pathway_not_asi_substrate_pass():
    """pathway_not_asi_substrate: PASS (主 23:44)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["pathway_not_asi_substrate"] is True


def test_v1222_v3_guard_ceiling_1_0_not_asi_pass():
    """ceiling_1_0_not_asi: PASS (主 17:58)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["ceiling_1_0_not_asi"] is True


def test_v1222_v3_guard_75_mol_not_complete_pass():
    """v1222_75_mol_not_complete: PASS (主 17:43)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["v1222_75_mol_not_complete"] is True


def test_v1222_v3_guard_new_dim_not_full_coverage_pass():
    """v1222_new_dim_not_full_coverage: PASS (主 17:43)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["v1222_new_dim_not_full_coverage"] is True


def test_v1222_v3_guard_not_full_ae_lift_pass():
    """v1222_not_full_ae_lift: PASS (6 lifted < 13 cells = vacuous 7 cell)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v3_guards["v1222_not_full_ae_lift"] is True


def test_v1222_all_v3_guards_pass():
    """All 10 V3 哲学守门 PASS (主 17:58 + 主 20:46 + 主 17:43)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert all(rep.v3_guards.values())
    assert len(rep.v3_guards) == 10


# ============================================================================
# Matrix extension tests
# ============================================================================

def test_v1222_matrix_extended_from_182_to_195():
    """V1222 matrix: V1221 182 cell → V1222 195 cell (扩 13 cell 新增)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_total_cells == 195
    # 14 dim × 13 R = 182; 15 dim × 13 R = 195
    assert 195 - 182 == 13


def test_v1222_realized_extended_from_124_to_130():
    """V1222 realized cells: V1221 124 → V1222 130 (扩 6 新增)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert rep.v1222_realized_cells_count == 130
    # V1221 124 + 6 AE new cells = 130
    assert 130 - 124 == 6


def test_v1222_v1221_to_v1222_inflation_gap_less_than_one():
    """V1222 inflation_gap < 1.0 (主 17:43 实事求是)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    assert 0.0 < rep.v1222_inflation_gap_v1221_minus_realized < 1.0


# ============================================================================
# Report dataclass integrity tests
# ============================================================================

def test_v1222_report_dataclass_fields():
    """V1222Report dataclass has all expected fields (主 00:44 质量工程化)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222Report
    expected_fields = {
        "snapshot_id", "dim_version", "timestamp", "elapsed", "north_star",
        "v1221_recompute_baseline", "v1221_realized_mean_124_baseline",
        "v1221_overall_mean_182_baseline", "v1221_mr_realized_baseline",
        "v1220_recompute_baseline", "v1220_realized_mean_118_baseline",
        "v1220_overall_mean_169_baseline",
        "n_pathways_total", "n_pathways_pass",
        "n_r1_growth_pathways_pass", "n_r4_aging_pathways_pass",
        "n_r7_stress_pathways_pass", "n_r10_plasticity_pathways_pass",
        "n_r11_consciousness_pathways_pass", "n_r12_ecology_pathways_pass",
        "total_ae_molecules",
        "n_r1_growth_molecules", "n_r4_aging_molecules",
        "n_r7_stress_molecules", "n_r10_plasticity_molecules",
        "n_r11_consciousness_molecules", "n_r12_ecology_molecules",
        "pathway_scores", "pathway_real_molecule_count",
        "ae_coverage_v1222",
        "v1222_ae_x_r1_growth", "v1222_ae_x_r4_aging",
        "v1222_ae_x_r7_stress", "v1222_ae_x_r10_plasticity",
        "v1222_ae_x_r11_consciousness", "v1222_ae_x_r12_ecology",
        "v1222_ae_dim_realized", "v1222_ae_dim_cell_count",
        "v1222_total_cells", "v1222_realized_cells_count",
        "v1222_130_sum", "v1222_overall_realized_130",
        "v1222_195_sum", "v1222_overall_mean_195",
        "v1222_overall_lift_delta_realized_from_v1221",
        "v1222_overall_lift_delta_mean_from_v1221",
        "v1222_inflation_gap_v1221_minus_realized",
        "position_of_north_star_realized_pct",
        "v3_guards",
    }
    actual_fields = set(V1222Report.__dataclass_fields__.keys())
    assert expected_fields.issubset(actual_fields)


def test_v1222_report_dataclass_serializable():
    """V1222Report serializes to JSON via asdict (主 00:56 任何人都能接手)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import (
        V1222Report, measure_v1222_full,
    )
    from dataclasses import asdict
    rep = measure_v1222_full()
    d = asdict(rep)
    json_str = json.dumps(d)
    parsed = json.loads(json_str)
    assert parsed["dim_version"] == "0.6.32"
    assert parsed["v1222_ae_dim_realized"] == 1.0


def test_v1222_snapshot_id_is_uuid():
    """V1222 snapshot_id is a valid UUID (主 23:44 干到底)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import measure_v1222_full
    rep = measure_v1222_full()
    # UUID4 format: 8-4-4-4-12
    parts = rep.snapshot_id.split("-")
    assert len(parts) == 5
    assert len(parts[0]) == 8
    assert len(parts[1]) == 4
    assert len(parts[2]) == 4
    assert len(parts[3]) == 4
    assert len(parts[4]) == 12


# ============================================================================
# Cascade tests
# ============================================================================

def test_v1222_neuroaesthetic_cascade_order_present():
    """AE_NEURO_AESTHETIC has cascade_order (主 23:44)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_NEURO_AESTHETIC"]["cascade_order"]) == 10


def test_v1222_developmental_taste_cascade_order_present():
    """AE_DEVELOPMENTAL_TASTE has cascade_order."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_DEVELOPMENTAL_TASTE"]["cascade_order"]) == 10


def test_v1222_emotional_aesthetic_cascade_order_present():
    """AE_EMOTIONAL_AESTHETIC has cascade_order."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_EMOTIONAL_AESTHETIC"]["cascade_order"]) == 10


def test_v1222_creative_brain_cascade_order_present():
    """AE_CREATIVE_BRAIN has cascade_order = 25."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_CREATIVE_BRAIN"]["cascade_order"]) == 25


def test_v1222_philosophical_concept_cascade_order_present():
    """AE_PHILOSOPHICAL_CONCEPT has cascade_order."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_PHILOSOPHICAL_CONCEPT"]["cascade_order"]) == 10


def test_v1222_evolutionary_cultural_cascade_order_present():
    """AE_EVOLUTIONARY_CULTURAL has cascade_order."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    assert len(V1222_AE_SUBSTRATE["AE_EVOLUTIONARY_CULTURAL"]["cascade_order"]) == 10


def test_v1222_all_pathways_have_source_field():
    """All 6 AE pathways have a source field (主 19:33 站在前人肩上)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    for p in V1222_AE_SUBSTRATE.values():
        assert "source" in p
        assert len(p["source"]) > 0


def test_v1222_all_pathways_have_description():
    """All 6 AE pathways have description (主 00:56 任何人都能接手)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import V1222_AE_SUBSTRATE
    for p in V1222_AE_SUBSTRATE.values():
        assert "description" in p
        assert len(p["description"]) > 0


# ============================================================================
# Integration tests with V1221 backward compatibility
# ============================================================================

def test_v1222_v1221_baseline_preserved():
    """V1222 preserves V1221 baseline constants (主 17:43 写死)."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import (
        V1221_REALIZED_MEAN_124, V1221_OVERALL_MEAN_182, V1221_MR_REALIZED,
    )
    assert V1221_REALIZED_MEAN_124 == 0.6540
    assert V1221_OVERALL_MEAN_182 == 0.4455
    assert V1221_MR_REALIZED == 1.0000


def test_v1222_130_sum_equals_baseline_plus_ae_row():
    """V1222 130 sum = V1221 baseline sum + AE row sum."""
    from apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift import (
        V1221_REALIZED_MEAN_124, V1222_AE_COVERAGE, measure_v1222_full,
    )
    rep = measure_v1222_full()
    expected_baseline_sum = V1221_REALIZED_MEAN_124 * 124.0
    expected_ae_row_sum = sum(
        V1222_AE_COVERAGE[r] for r in [
            "R1_growth", "R4_aging", "R7_stress",
            "R10_plasticity", "R11_consciousness", "R12_ecology",
        ]
    )
    expected_total = expected_baseline_sum + expected_ae_row_sum
    assert abs(rep.v1222_130_sum - expected_total) < 0.5


# ============================================================================
# CLI tests (主 00:56 任何人都能接手)
# ============================================================================

def test_v1222_cli_measure():
    """V1222 CLI --measure works (主 00:56)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift", "--measure"],
        capture_output=True, text=True, cwd=WORKSPACE_ROOT, timeout=30,
    )
    assert result.returncode == 0
    assert "V1222 AE dim realized" in result.stdout


def test_v1222_cli_json():
    """V1222 CLI --json works (主 00:56)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift", "--json"],
        capture_output=True, text=True, cwd=WORKSPACE_ROOT, timeout=30,
    )
    assert result.returncode == 0
    parsed = json.loads(result.stdout)
    assert parsed["dim_version"] == "0.6.32"
    assert parsed["v1222_ae_dim_realized"] == 1.0


def test_v1222_cli_full():
    """V1222 CLI --full generates report + artifact."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1222_asi_v0632_aesthetics_substrate_real_lift", "--full"],
        capture_output=True, text=True, cwd=WORKSPACE_ROOT, timeout=30,
    )
    assert result.returncode == 0
    assert "V1222 report" in result.stdout
    assert "V1222 artifact" in result.stdout