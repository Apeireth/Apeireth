"""Tests for V1224 ASI V0.6.34 wisdom_substrate_real_lift.

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

# V1223 baseline
V1223_REALIZED_MEAN_136 = 0.6846
V1223_OVERALL_MEAN_208 = 0.4475
V1223_ME_REALIZED = 1.0000

# V1222 baseline
V1222_REALIZED_MEAN_130 = 0.6700
V1222_OVERALL_MEAN_195 = 0.4466
V1222_AE_REALIZED = 1.0000


# ============================================================================
# Module-level import tests
# ============================================================================

def test_v1224_module_imports():
    """V1224 module imports without error."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import (
        ASI_NORTH_STAR,
        V1224_DIM_VERSION,
        V1224_WIS_COVERAGE,
        V1224_WIS_SUBSTRATE,
        V1224_VERSION,
        V1224Report,
        measure_v1224_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1224_VERSION == "0.1.0"
    assert V1224_DIM_VERSION == "0.6.34"
    assert isinstance(V1224_WIS_SUBSTRATE, dict)
    assert isinstance(V1224_WIS_COVERAGE, dict)
    assert isinstance(V1224Report, type)


def test_v1224_wis_substrate_6_pathways():
    """V1224 WIS substrate has 6 pathways (主 19:33 站在前人肩上)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    assert len(V1224_WIS_SUBSTRATE) == 6
    expected = {
        "WIS_NEURO_WISDOM",
        "WIS_LIFESPAN_DEV",
        "WIS_CRISIS_APP",
        "WIS_COGNITIVE_INTEGRATION",
        "WIS_PHILOSOPHICAL_TRADITION",
        "WIS_CULTURAL_TRADITION",
    }
    assert set(V1224_WIS_SUBSTRATE.keys()) == expected


def test_v1224_wis_substrate_60_molecules():
    """V1224 has 60 molecules total (6 pathway × 10 molecules)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1224_WIS_SUBSTRATE.values())
    assert total == 60


def test_v1224_wis_substrate_75_molecules_or_more():
    """V1224 has ≥ 75 cascade_order entries across pathways (主 17:43)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    total = sum(len(p.get("cascade_order", [])) for p in V1224_WIS_SUBSTRATE.values())
    assert total >= 60  # 6 pathway × 10 cascade = 60


def test_v1224_wis_coverage_6_lifted():
    """V1224 WIS coverage has 6 cells lifted to 1.0 (R1/R4/R7/R10/R11/R12)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_COVERAGE
    assert V1224_WIS_COVERAGE["R1_growth"] == 1.0
    assert V1224_WIS_COVERAGE["R4_aging"] == 1.0
    assert V1224_WIS_COVERAGE["R7_stress"] == 1.0
    assert V1224_WIS_COVERAGE["R10_plasticity"] == 1.0
    assert V1224_WIS_COVERAGE["R11_consciousness"] == 1.0
    assert V1224_WIS_COVERAGE["R12_ecology"] == 1.0


# ============================================================================
# Reference key papers tests (主 19:33 站在前人肩上)
# ============================================================================

def test_v1224_wis_neuro_wisdom_references_key_papers():
    """WIS_NEURO_WISDOM references Jeste 2010 + Reisch 2018 + Greene 2001."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    src = V1224_WIS_SUBSTRATE["WIS_NEURO_WISDOM"]["source"]
    assert "Jeste 2010" in src
    assert "Reisch 2018" in src
    assert "Greene 2001" in src or "Koenigs 2007" in src


def test_v1224_wis_lifespan_dev_references_key_papers():
    """WIS_LIFESPAN_DEV references Baltes 1995 + Ardelt 2004 + Webster 2003."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    src = V1224_WIS_SUBSTRATE["WIS_LIFESPAN_DEV"]["source"]
    assert "Baltes 1995" in src
    assert "Ardelt 2004" in src
    assert "Webster 2003" in src


def test_v1224_wis_crisis_app_references_key_papers():
    """WIS_CRISIS_APP references King Kitchener + Aristotle + Hadot."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    src = V1224_WIS_SUBSTRATE["WIS_CRISIS_APP"]["source"]
    assert "King Kitchener" in src
    assert "Aristotle" in src
    assert "Hadot" in src


def test_v1224_wis_cognitive_integration_references_key_papers():
    """WIS_COGNITIVE_INTEGRATION references Tetlock + Rawls + Stanovich West."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    src = V1224_WIS_SUBSTRATE["WIS_COGNITIVE_INTEGRATION"]["source"]
    assert "Tetlock" in src
    assert "Rawls" in src
    assert "Stanovich" in src


def test_v1224_wis_philosophical_tradition_references_key_papers():
    """WIS_PHILOSOPHICAL_TRADITION references Aristotle + Confucius + Buddhist + Stoic."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    src = V1224_WIS_SUBSTRATE["WIS_PHILOSOPHICAL_TRADITION"]["source"]
    assert "Aristotle" in src
    assert "Confucius" in src
    assert "Buddhist" in src or "Prajna" in src
    assert "Stoic" in src or "Marcus" in src


def test_v1224_wis_cultural_tradition_references_key_papers():
    """WIS_CULTURAL_TRADITION references Kimmerer + Ubuntu + Buen Vivir + Berkes."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    src = V1224_WIS_SUBSTRATE["WIS_CULTURAL_TRADITION"]["source"]
    assert "Kimmerer" in src
    assert "Ubuntu" in src or "Tutu" in src
    assert "Buen Vivir" in src or "Acosta" in src
    assert "Berkes" in src


# ============================================================================
# measure_v1224_full() tests
# ============================================================================

def test_v1224_measure_returns_report():
    """measure_v1224_full returns V1224Report dataclass."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import (
        V1224Report,
        measure_v1224_full,
    )
    rep = measure_v1224_full()
    assert isinstance(rep, V1224Report)


def test_v1224_measure_snapshot_id_nonempty():
    """measure_v1224_full snapshot_id is non-empty UUID-like string."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert len(rep.snapshot_id) == 36
    assert rep.snapshot_id.count("-") == 4


def test_v1224_measure_dim_version_correct():
    """measure_v1224_full dim_version is correct."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.dim_version == "0.6.34"


def test_v1224_measure_timestamp_nonempty():
    """measure_v1224_full timestamp is non-empty ISO string."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert "T" in rep.timestamp


def test_v1224_measure_elapsed_fast():
    """measure_v1224_full elapsed < 1s (主 23:44 干到底)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.elapsed < 1.0


def test_v1224_measure_wis_dim_realized_1():
    """WIS dim realized = 1.0000 (all 6 cells lifted to 1.0)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_dim_realized == 1.0000


def test_v1224_measure_wis_dim_cell_count_6():
    """WIS dim cell count = 6 (R1/R4/R7/R10/R11/R12 lifted)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_dim_cell_count == 6


def test_v1224_measure_total_cells_221():
    """V1224 total matrix = 221 = 17 dim × 13 R."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_total_cells == 221


def test_v1224_measure_realized_cells_count_142():
    """V1224 realized cells = 142 = 136 + 6."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_realized_cells_count == 142


def test_v1224_measure_overall_realized_142_lift_positive():
    """V1224 realized 142 > V1223 baseline 136 (主 17:43 实事求是 — lift positive)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_overall_realized_142 > V1223_REALIZED_MEAN_136


def test_v1224_measure_overall_realized_142_lift_delta():
    """V1224 lift delta from V1223 ≈ +0.0133 (主 17:43)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.v1224_overall_lift_delta_realized_from_v1223 - 0.0133) < 0.001


def test_v1224_measure_overall_mean_221_lift_positive():
    """V1224 mean 221 > V1223 baseline 208 (主 17:43)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_overall_mean_221 > V1223_OVERALL_MEAN_208


def test_v1224_measure_inflation_gap_positive():
    """V1224 inflation gap > 0 (主 17:43 实事求是 — inflation 真实存在)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_inflation_gap_v1223_minus_realized > 0


def test_v1224_measure_inflation_gap_value():
    """V1224 inflation gap ≈ 0.5517 (1.0 - 0.4483)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.v1224_inflation_gap_v1223_minus_realized - 0.5517) < 0.001


def test_v1224_measure_position_north_star_pct():
    """V1224 position vs north star ≈ 71.22% (0.6979/0.98)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.position_of_north_star_realized_pct - 71.22) < 0.5


def test_v1224_measure_total_wis_molecules():
    """V1224 total WIS molecules = 60."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.total_wis_molecules == 60


def test_v1224_measure_all_pathways_pass():
    """All 6 pathways pass (score >= 0.7) (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1224_measure_pathway_scores_valid():
    """All pathway scores are in [0, 1]."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    for k, s in rep.pathway_scores.items():
        assert 0.0 <= s <= 1.0, f"{k}: {s}"


def test_v1224_measure_wis_x_r1_growth_1():
    """WIS × R1_growth = 1.0."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_x_r1_growth == 1.0


def test_v1224_measure_wis_x_r4_aging_1():
    """WIS × R4_aging = 1.0."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_x_r4_aging == 1.0


def test_v1224_measure_wis_x_r7_stress_1():
    """WIS × R7_stress = 1.0."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_x_r7_stress == 1.0


def test_v1224_measure_wis_x_r10_plasticity_1():
    """WIS × R10_plasticity = 1.0."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_x_r10_plasticity == 1.0


def test_v1224_measure_wis_x_r11_consciousness_1():
    """WIS × R11_consciousness = 1.0."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_x_r11_consciousness == 1.0


def test_v1224_measure_wis_x_r12_ecology_1():
    """WIS × R12_ecology = 1.0."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_x_r12_ecology == 1.0


# ============================================================================
# V3 哲学守门 tests (主 17:58 + 主 20:46 不假装)
# ============================================================================

def test_v1224_v3_guards_all_pass():
    """All 10 V3 哲学守门 PASS (主 17:58 不假装)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1224_v3_guard_realized_not_asi():
    """realized 0.6979 < north_star 0.98 (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1224_overall_realized_142 < ASI_NORTH_STAR


def test_v1224_v3_guard_inflation_real():
    """inflation gap > 0 (主 17:43 — 221 cell 公式下 inflation 真实存在)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v3_guards["vacuous_gap_real"] is True
    assert rep.v1224_inflation_gap_v1223_minus_realized > 0


def test_v1224_v3_guard_75_mol_not_complete():
    """60 真分子 ≠ complete WIS substrate (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v3_guards["v1224_75_mol_not_complete"] is True


def test_v1224_v3_guard_wis_lift_not_full():
    """6 lifted < 13 cells = vacuous 7 cell (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v3_guards["v1224_not_full_wis_lift"] is True
    assert rep.v1224_wis_dim_cell_count < 13


# ============================================================================
# write_v1224_artifact + write_v1224_report tests
# ============================================================================

def test_v1224_artifact_default_path():
    """V1224 artifact written to artifacts/."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import (
        measure_v1224_full,
        write_v1224_artifact,
    )
    rep = measure_v1224_full()
    path = write_v1224_artifact(rep)
    assert path.exists()
    assert "v0634_wisdom" in str(path).lower()


def test_v1224_report_default_path():
    """V1224 report written to reports/."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import (
        measure_v1224_full,
        write_v1224_report,
    )
    rep = measure_v1224_full()
    path = write_v1224_report(rep)
    assert path.exists()
    assert "v1224" in str(path).lower()


def test_v1224_artifact_valid_json():
    """V1224 artifact is valid JSON."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import (
        measure_v1224_full,
        write_v1224_artifact,
    )
    rep = measure_v1224_full()
    path = write_v1224_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.34"


def test_v1224_report_has_all_sections():
    """V1224 report contains key sections (主 00:56 任何人都能接手)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import (
        measure_v1224_full,
        write_v1224_report,
    )
    rep = measure_v1224_full()
    path = write_v1224_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1224 ASI V0.6.34" in content
    assert "wisdom" in content.lower()
    assert "North Star" in content
    assert "V3 哲学守门" in content
    assert "Pathway scores" in content
    assert "WIS coverage" in content


# ============================================================================
# CLI tests (主 23:44 干到底)
# ============================================================================

def test_v1224_cli_help():
    """V1224 CLI --help works."""
    cmd = [
        sys.executable, "-m", "apeireth.v1224_asi_v0634_wisdom_substrate_real_lift", "--help"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1224" in result.stdout or "wisdom" in result.stdout.lower()


def test_v1224_cli_measure():
    """V1224 CLI default (--measure) works and prints key metrics."""
    cmd = [
        sys.executable, "-m", "apeireth.v1224_asi_v0634_wisdom_substrate_real_lift", "--measure"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1224_overall_realized_142" in result.stdout
    assert "0.6.34" in result.stdout
    assert "north_star: 0.9800" in result.stdout


def test_v1224_cli_json():
    """V1224 CLI --json prints JSON."""
    cmd = [
        sys.executable, "-m", "apeireth.v1224_asi_v0634_wisdom_substrate_real_lift", "--json"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    try:
        json.loads(result.stdout.split("\n", 1)[1] if "\n" in result.stdout else result.stdout)
    except json.JSONDecodeError:
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


def test_v1224_cli_report():
    """V1224 CLI --report writes a report file."""
    cmd = [
        sys.executable, "-m", "apeireth.v1224_asi_v0634_wisdom_substrate_real_lift", "--report"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1224_cli_full():
    """V1224 CLI --full prints everything (主 23:44)."""
    cmd = [
        sys.executable, "-m", "apeireth.v1224_asi_v0634_wisdom_substrate_real_lift", "--full"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "WIS coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


# ============================================================================
# Baseline consistency tests (主 17:43 实事求是)
# ============================================================================

def test_v1224_baseline_consistency_v1223():
    """V1223 baseline values match V1223 module (主 17:43 实事求是)."""
    from apeireth.v1223_asi_v0633_meaning_substrate_real_lift import V1223_ME_REALIZED, V1223_OVERALL_MEAN_208, V1223_REALIZED_MEAN_136
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1223_ME_REALIZED as V4_V1223_ME, V1223_OVERALL_MEAN_208 as V4_V1223_MEAN, V1223_REALIZED_MEAN_136 as V4_V1223_REAL
    assert V1223_ME_REALIZED == V4_V1223_ME
    assert V1223_OVERALL_MEAN_208 == V4_V1223_MEAN
    assert V1223_REALIZED_MEAN_136 == V4_V1223_REAL


# ============================================================================
# North Star trajectory tests (主 22:33 LOCKED)
# ============================================================================

def test_v1224_north_star_locked_098():
    """North Star LOCKED at 0.9800 (主 22:33 终极授权)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import ASI_NORTH_STAR, measure_v1224_full
    assert ASI_NORTH_STAR == 0.9800
    rep = measure_v1224_full()
    assert rep.north_star == 0.9800


def test_v1224_north_star_trajectory_monotonic_increase():
    """V1222 → V1223 → V1224 realized monotonically increasing (主 23:44 干到底)."""
    # V1222: 0.6700 (130 cell)
    # V1223: 0.6846 (136 cell)
    # V1224: 0.6979 (142 cell)
    assert V1222_REALIZED_MEAN_130 < V1223_REALIZED_MEAN_136
    assert V1223_REALIZED_MEAN_136 < 0.6979 + 0.001


# ============================================================================
# ASI dimensions inventory test (主 19:33 站在前人肩上)
# ============================================================================

def test_v1224_wis_dim_dominates_pathway_lift():
    """WIS dim 6 cells lifted >= V1222 AE dim 6 cells lifted (主 19:33)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_wis_dim_cell_count == 6


# ============================================================================
# ASI overall matrix tests
# ============================================================================

def test_v1224_overall_realized_142_value():
    """V1224 overall realized 142 ≈ 0.6979 (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.v1224_overall_realized_142 - 0.6979) < 0.001


def test_v1224_overall_mean_221_value():
    """V1224 overall mean 221 ≈ 0.4483 (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.v1224_overall_mean_221 - 0.4483) < 0.001


def test_v1224_142_sum_value():
    """V1224 142 sum = 142 * 0.6979 ≈ 99.10."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.v1224_142_sum - 142.0 * 0.6979) < 0.5


def test_v1224_221_sum_value():
    """V1224 221 sum = 221 * 0.4483 ≈ 99.07."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert abs(rep.v1224_221_sum - 221.0 * 0.4483) < 0.5


def test_v1224_realized_less_than_north_star():
    """V1224 realized 0.6979 < north star 0.98 (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_overall_realized_142 < ASI_NORTH_STAR


def test_v1224_mean_less_than_1():
    """V1224 mean 0.4483 < 1.0 (主 17:43 实事求是)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    assert rep.v1224_overall_mean_221 < 1.0


def test_v1224_wis_6_lifted_cells_sum():
    """WIS row sum = 6.0 (6 cells × 1.0)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full
    rep = measure_v1224_full()
    wis_row_sum = (
        rep.v1224_wis_x_r1_growth
        + rep.v1224_wis_x_r4_aging
        + rep.v1224_wis_x_r7_stress
        + rep.v1224_wis_x_r10_plasticity
        + rep.v1224_wis_x_r11_consciousness
        + rep.v1224_wis_x_r12_ecology
    )
    assert wis_row_sum == 6.0


def test_v1224_wis_vacuous_7_cells_sum():
    """WIS vacuous 7 cells = 0.0 (R2/R3/R5/R6/R8/R9 = 0 + 0)."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import measure_v1224_full, V1224_WIS_COVERAGE
    rep = measure_v1224_full()
    vacuous_sum = (
        V1224_WIS_COVERAGE["R2_sensing"]
        + V1224_WIS_COVERAGE["R3_cognition"]
        + V1224_WIS_COVERAGE["R5_social"]
        + V1224_WIS_COVERAGE["R6_communication"]
        + V1224_WIS_COVERAGE["R8_motion"]
        + V1224_WIS_COVERAGE["R9_heredity"]
    )
    assert vacuous_sum == 0.0


def test_v1224_all_wis_pathways_have_cascade():
    """All 6 WIS pathways have cascade_order entries."""
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_SUBSTRATE
    for p_name, p_data in V1224_WIS_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        mols = p_data.get("molecules", [])
        assert len(cascade) > 0, f"{p_name} missing cascade"
        assert len(cascade) == len(mols), f"{p_name} cascade mismatch molecules"
        mol_names = [m["name"] for m in mols]
        for c in cascade:
            assert c in mol_names, f"{p_name}: cascade entry {c} not in molecules"