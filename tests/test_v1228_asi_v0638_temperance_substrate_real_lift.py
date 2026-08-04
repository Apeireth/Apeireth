"""Tests for V1228 ASI V0.6.38 temperance_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1227_REALIZED_MEAN_160 = 0.7318
V1227_OVERALL_MEAN_260 = 0.4503
V1227_COURAGE_REALIZED = 1.0000
V1226_HOP_REALIZED = 1.0000
V1226_OVERALL_MEAN_247 = 0.4497
V1226_REALIZED_MEAN_154 = 0.7214


def test_v1228_module_imports():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        ASI_NORTH_STAR, V1228_DIM_VERSION, V1228_TEMPERANCE_COVERAGE,
        V1228_TEMPERANCE_SUBSTRATE, V1228_VERSION, V1228Report,
        measure_v1228_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1228_VERSION == "0.1.0"
    assert V1228_DIM_VERSION == "0.6.38"


def test_v1228_temperance_substrate_6_pathways():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    assert len(V1228_TEMPERANCE_SUBSTRATE) == 6
    expected = {
        "TEMP_NEURO_INHIBIT", "TEMP_LIFESPAN_MODERATION", "TEMP_CRISIS_SOPHROSYNE",
        "TEMP_COGNITIVE_RESTRAINT", "TEMP_PHILOSOPHY", "TEMP_ECOLOGY",
    }
    assert set(V1228_TEMPERANCE_SUBSTRATE.keys()) == expected


def test_v1228_temperance_substrate_60_molecules():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1228_TEMPERANCE_SUBSTRATE.values())
    assert total == 60


def test_v1228_temperance_coverage_6_lifted():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
              "R11_consciousness", "R12_ecology"]:
        assert V1228_TEMPERANCE_COVERAGE[k] == 1.0


def test_v1228_temperance_neuro_references_key_papers():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    src = V1228_TEMPERANCE_SUBSTRATE["TEMP_NEURO_INHIBIT"]["source"]
    assert "Aron" in src or "Hare" in src
    assert "McClure" in src or "Bechara" in src


def test_v1228_temperance_lifespan_references_key_papers():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    src = V1228_TEMPERANCE_SUBSTRATE["TEMP_LIFESPAN_MODERATION"]["source"]
    assert "Erikson" in src
    assert "Baltes" in src or "Carstensen" in src


def test_v1228_temperance_crisis_references_key_papers():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    src = V1228_TEMPERANCE_SUBSTRATE["TEMP_CRISIS_SOPHROSYNE"]["source"]
    assert "Sapolsky" in src
    assert "McEwen" in src or "Hofmann" in src


def test_v1228_temperance_cognitive_references_key_papers():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    src = V1228_TEMPERANCE_SUBSTRATE["TEMP_COGNITIVE_RESTRAINT"]["source"]
    assert "Mischel" in src
    assert "Hofmann" in src or "Baumeister" in src


def test_v1228_temperance_philosophy_references_key_papers():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    src = V1228_TEMPERANCE_SUBSTRATE["TEMP_PHILOSOPHY"]["source"]
    assert "Aristotle" in src
    assert "Aquinas" in src
    assert "sophrosyne" in src or "temperantia" in src
    assert "Buddhist" in src or "Confucius" in src


def test_v1228_temperance_ecology_references_key_papers():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    src = V1228_TEMPERANCE_SUBSTRATE["TEMP_ECOLOGY"]["source"]
    assert "Raworth" in src or "Pianka" in src
    assert "Schwartz" in src or "Schumacher" in src


def test_v1228_measure_returns_report():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228Report, measure_v1228_full
    rep = measure_v1228_full()
    assert isinstance(rep, V1228Report)


def test_v1228_measure_dim_version():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.dim_version == "0.6.38"


def test_v1228_measure_elapsed_fast():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.elapsed < 1.0


def test_v1228_measure_temperance_dim_realized_1():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_temperance_dim_realized == 1.0000


def test_v1228_measure_temperance_dim_cell_count_6():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_temperance_dim_cell_count == 6


def test_v1228_measure_total_cells_273():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_total_cells == 273


def test_v1228_measure_realized_cells_count_166():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_realized_cells_count == 166


def test_v1228_measure_overall_realized_166_lift_positive():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_overall_realized_166 > V1227_REALIZED_MEAN_160


def test_v1228_measure_overall_realized_166_lift_delta():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert abs(rep.v1228_overall_lift_delta_realized_from_v1227 - 0.0097) < 0.001


def test_v1228_measure_inflation_gap_positive():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_inflation_gap_v1227_minus_realized > 0


def test_v1228_measure_position_north_star_pct():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert abs(rep.position_of_north_star_realized_pct - 75.66) < 0.5


def test_v1228_measure_total_temperance_molecules():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.total_temperance_molecules == 60


def test_v1228_measure_all_pathways_pass():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1228_v3_guards_all_pass():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1228_v3_guard_realized_not_asi():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1228_overall_realized_166 < ASI_NORTH_STAR


def test_v1228_v3_guard_60_mol_not_complete():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v3_guards["v1228_60_mol_not_complete"] is True


def test_v1228_v3_guard_temperance_lift_not_full():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v3_guards["v1228_not_full_temperance_lift"] is True


def test_v1228_artifact_default_path():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        measure_v1228_full, write_v1228_artifact,
    )
    rep = measure_v1228_full()
    path = write_v1228_artifact(rep)
    assert path.exists()


def test_v1228_report_default_path():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        measure_v1228_full, write_v1228_report,
    )
    rep = measure_v1228_full()
    path = write_v1228_report(rep)
    assert path.exists()


def test_v1228_artifact_valid_json():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        measure_v1228_full, write_v1228_artifact,
    )
    rep = measure_v1228_full()
    path = write_v1228_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.38"


def test_v1228_report_has_all_sections():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        measure_v1228_full, write_v1228_report,
    )
    rep = measure_v1228_full()
    path = write_v1228_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1228 ASI V0.6.38" in content
    assert "temperance" in content.lower() or "节制" in content
    assert "4 Cardinal Virtue" in content or "cardinal" in content.lower()
    assert "V3 哲学守门" in content


def test_v1228_cli_help():
    cmd = [sys.executable, "-m", "apeireth.v1228_asi_v0638_temperance_substrate_real_lift", "--help"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1228" in result.stdout or "temperance" in result.stdout.lower()


def test_v1228_cli_measure():
    cmd = [sys.executable, "-m", "apeireth.v1228_asi_v0638_temperance_substrate_real_lift", "--measure"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1228_overall_realized_166" in result.stdout
    assert "0.6.38" in result.stdout


def test_v1228_cli_json():
    cmd = [sys.executable, "-m", "apeireth.v1228_asi_v0638_temperance_substrate_real_lift", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1228_cli_report():
    cmd = [sys.executable, "-m", "apeireth.v1228_asi_v0638_temperance_substrate_real_lift", "--report"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1228_cli_full():
    cmd = [sys.executable, "-m", "apeireth.v1228_asi_v0638_temperance_substrate_real_lift", "--full"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "TEMPERANCE coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1228_baseline_consistency_v1227():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        V1227_COURAGE_REALIZED as V7_V1227_COURAGE, V1227_OVERALL_MEAN_260 as V7_V1227_MEAN,
        V1227_REALIZED_MEAN_160 as V7_V1227_REAL,
    )
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        V1227_COURAGE_REALIZED, V1227_OVERALL_MEAN_260, V1227_REALIZED_MEAN_160,
    )
    assert V1227_COURAGE_REALIZED == V7_V1227_COURAGE
    assert V1227_OVERALL_MEAN_260 == V7_V1227_MEAN
    assert V1227_REALIZED_MEAN_160 == V7_V1227_REAL


def test_v1228_north_star_locked_098():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import ASI_NORTH_STAR, measure_v1228_full
    assert ASI_NORTH_STAR == 0.9800
    rep = measure_v1228_full()
    assert rep.north_star == 0.9800


def test_v1228_overall_realized_166_value():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert abs(rep.v1228_overall_realized_166 - 0.7415) < 0.001


def test_v1228_overall_mean_273_value():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert abs(rep.v1228_overall_mean_273 - 0.4508) < 0.001


def test_v1228_166_sum_value():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert abs(rep.v1228_166_sum - 166.0 * 0.7415) < 0.5


def test_v1228_273_sum_value():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert abs(rep.v1228_273_sum - 273.0 * 0.4508) < 0.5


def test_v1228_realized_less_than_north_star():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_overall_realized_166 < ASI_NORTH_STAR


def test_v1228_mean_less_than_1():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.v1228_overall_mean_273 < 1.0


def test_v1228_temperance_6_lifted_cells_sum():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    s = (
        rep.v1228_temperance_x_r1_growth
        + rep.v1228_temperance_x_r4_aging
        + rep.v1228_temperance_x_r7_stress
        + rep.v1228_temperance_x_r10_plasticity
        + rep.v1228_temperance_x_r11_consciousness
        + rep.v1228_temperance_x_r12_ecology
    )
    assert s == 6.0


def test_v1228_all_temperance_pathways_have_cascade():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    for p_name, p_data in V1228_TEMPERANCE_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        mols = p_data.get("molecules", [])
        assert len(cascade) > 0, f"{p_name} missing cascade"
        assert len(cascade) == len(mols), f"{p_name} cascade ≠ molecules count"


def test_v1228_all_temperance_molecules_real():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import V1228_TEMPERANCE_SUBSTRATE
    for p_name, p_data in V1228_TEMPERANCE_SUBSTRATE.items():
        for mol in p_data.get("molecules", []):
            assert mol.get("real", False), f"{p_name}:{mol.get('name')} not real"


def test_v1228_pathway_score_threshold():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    for k, s in rep.pathway_scores.items():
        assert s >= 0.7, f"{k} score {s:.4f} below threshold"


def test_v1228_pathway_real_molecule_count_60():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    total = sum(rep.pathway_real_molecule_count.values())
    assert total == 60


def test_v1228_per_pathway_pass_count_by_r_substrate():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.n_r1_growth_pathways_pass == 1
    assert rep.n_r4_aging_pathways_pass == 1
    assert rep.n_r7_stress_pathways_pass == 1
    assert rep.n_r10_plasticity_pathways_pass == 1
    assert rep.n_r11_consciousness_pathways_pass == 1
    assert rep.n_r12_ecology_pathways_pass == 1


def test_v1228_per_pathway_molecule_count_by_r_substrate():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import measure_v1228_full
    rep = measure_v1228_full()
    assert rep.n_r1_growth_molecules == 10
    assert rep.n_r4_aging_molecules == 10
    assert rep.n_r7_stress_molecules == 10
    assert rep.n_r10_plasticity_molecules == 10
    assert rep.n_r11_consciousness_molecules == 10
    assert rep.n_r12_ecology_molecules == 10
