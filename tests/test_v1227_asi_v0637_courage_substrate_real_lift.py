"""Tests for V1227 ASI V0.6.37 courage_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1226_REALIZED_MEAN_154 = 0.7214
V1226_OVERALL_MEAN_247 = 0.4497
V1226_HOP_REALIZED = 1.0000
V1225_LOV_REALIZED = 1.0000
V1225_OVERALL_MEAN_234 = 0.4490
V1225_REALIZED_MEAN_148 = 0.7101


def test_v1227_module_imports():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        ASI_NORTH_STAR, V1227_DIM_VERSION, V1227_COURAGE_COVERAGE,
        V1227_COURAGE_SUBSTRATE, V1227_VERSION, V1227Report,
        measure_v1227_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1227_VERSION == "0.1.0"
    assert V1227_DIM_VERSION == "0.6.37"


def test_v1227_courage_substrate_6_pathways():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    assert len(V1227_COURAGE_SUBSTRATE) == 6
    expected = {
        "COURAGE_NEURO_FEAR", "COURAGE_LIFESPAN_DEV", "COURAGE_CRISIS",
        "COURAGE_COGNITIVE", "COURAGE_PHILOSOPHY", "COURAGE_SOCIAL_ECOLOGY",
    }
    assert set(V1227_COURAGE_SUBSTRATE.keys()) == expected


def test_v1227_courage_substrate_60_molecules():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1227_COURAGE_SUBSTRATE.values())
    assert total == 60


def test_v1227_courage_coverage_6_lifted():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
              "R11_consciousness", "R12_ecology"]:
        assert V1227_COURAGE_COVERAGE[k] == 1.0


def test_v1227_courage_neuro_references_key_papers():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    src = V1227_COURAGE_SUBSTRATE["COURAGE_NEURO_FEAR"]["source"]
    assert "LeDoux 1996" in src or "Eisenberger 2011" in src
    assert "Panksepp" in src or "Mobbs" in src


def test_v1227_courage_lifespan_references_key_papers():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    src = V1227_COURAGE_SUBSTRATE["COURAGE_LIFESPAN_DEV"]["source"]
    assert "Erikson" in src
    assert "Putnam" in src or "Damon" in src


def test_v1227_courage_crisis_references_key_papers():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    src = V1227_COURAGE_SUBSTRATE["COURAGE_CRISIS"]["source"]
    assert "Yalom" in src
    assert "Tedeschi Calhoun" in src or "Masten" in src


def test_v1227_courage_cognitive_references_key_papers():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    src = V1227_COURAGE_SUBSTRATE["COURAGE_COGNITIVE"]["source"]
    assert "Duckworth" in src
    assert "Dweck" in src or "Bandura" in src


def test_v1227_courage_philosophy_references_key_papers():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    src = V1227_COURAGE_SUBSTRATE["COURAGE_PHILOSOPHY"]["source"]
    assert "Aristotle" in src
    assert "Tillich" in src or "Tillich 1952" in src
    assert "Camus" in src or "Heidegger" in src


def test_v1227_courage_social_references_key_papers():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    src = V1227_COURAGE_SUBSTRATE["COURAGE_SOCIAL_ECOLOGY"]["source"]
    assert "Freire" in src
    assert "Havel" in src or "Mandela" in src or "Tutu" in src


def test_v1227_measure_returns_report():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227Report, measure_v1227_full
    rep = measure_v1227_full()
    assert isinstance(rep, V1227Report)


def test_v1227_measure_dim_version():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.dim_version == "0.6.37"


def test_v1227_measure_elapsed_fast():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.elapsed < 1.0


def test_v1227_measure_courage_dim_realized_1():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_courage_dim_realized == 1.0000


def test_v1227_measure_courage_dim_cell_count_6():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_courage_dim_cell_count == 6


def test_v1227_measure_total_cells_260():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_total_cells == 260


def test_v1227_measure_realized_cells_count_160():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_realized_cells_count == 160


def test_v1227_measure_overall_realized_160_lift_positive():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_overall_realized_160 > V1226_REALIZED_MEAN_154


def test_v1227_measure_overall_realized_160_lift_delta():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.v1227_overall_lift_delta_realized_from_v1226 - 0.0104) < 0.001


def test_v1227_measure_inflation_gap_positive():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_inflation_gap_v1226_minus_realized > 0


def test_v1227_measure_position_north_star_pct():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.position_of_north_star_realized_pct - 74.68) < 0.5


def test_v1227_measure_total_courage_molecules():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.total_courage_molecules == 60


def test_v1227_measure_all_pathways_pass():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1227_v3_guards_all_pass():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1227_v3_guard_realized_not_asi():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1227_overall_realized_160 < ASI_NORTH_STAR


def test_v1227_v3_guard_60_mol_not_complete():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v3_guards["v1227_60_mol_not_complete"] is True


def test_v1227_v3_guard_courage_lift_not_full():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v3_guards["v1227_not_full_courage_lift"] is True


def test_v1227_artifact_default_path():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        measure_v1227_full, write_v1227_artifact,
    )
    rep = measure_v1227_full()
    path = write_v1227_artifact(rep)
    assert path.exists()


def test_v1227_report_default_path():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        measure_v1227_full, write_v1227_report,
    )
    rep = measure_v1227_full()
    path = write_v1227_report(rep)
    assert path.exists()


def test_v1227_artifact_valid_json():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        measure_v1227_full, write_v1227_artifact,
    )
    rep = measure_v1227_full()
    path = write_v1227_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.37"


def test_v1227_report_has_all_sections():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        measure_v1227_full, write_v1227_report,
    )
    rep = measure_v1227_full()
    path = write_v1227_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1227 ASI V0.6.37" in content
    assert "courage" in content.lower() or "勇气" in content
    assert "V3 哲学守门" in content


def test_v1227_cli_help():
    cmd = [sys.executable, "-m", "apeireth.v1227_asi_v0637_courage_substrate_real_lift", "--help"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1227" in result.stdout or "courage" in result.stdout.lower()


def test_v1227_cli_measure():
    cmd = [sys.executable, "-m", "apeireth.v1227_asi_v0637_courage_substrate_real_lift", "--measure"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1227_overall_realized_160" in result.stdout
    assert "0.6.37" in result.stdout


def test_v1227_cli_json():
    cmd = [sys.executable, "-m", "apeireth.v1227_asi_v0637_courage_substrate_real_lift", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1227_cli_report():
    cmd = [sys.executable, "-m", "apeireth.v1227_asi_v0637_courage_substrate_real_lift", "--report"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1227_cli_full():
    cmd = [sys.executable, "-m", "apeireth.v1227_asi_v0637_courage_substrate_real_lift", "--full"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "COURAGE coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1227_baseline_consistency_v1226():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import (
        V1226_HOP_REALIZED as V6_V1226_HOP, V1226_OVERALL_MEAN_247 as V6_V1226_MEAN,
        V1226_REALIZED_MEAN_154 as V6_V1226_REAL,
    )
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        V1226_HOP_REALIZED, V1226_OVERALL_MEAN_247, V1226_REALIZED_MEAN_154,
    )
    assert V1226_HOP_REALIZED == V6_V1226_HOP
    assert V1226_OVERALL_MEAN_247 == V6_V1226_MEAN
    assert V1226_REALIZED_MEAN_154 == V6_V1226_REAL


def test_v1227_north_star_locked_098():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import ASI_NORTH_STAR, measure_v1227_full
    assert ASI_NORTH_STAR == 0.9800
    rep = measure_v1227_full()
    assert rep.north_star == 0.9800


def test_v1227_overall_realized_160_value():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.v1227_overall_realized_160 - 0.7318) < 0.001


def test_v1227_overall_mean_260_value():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.v1227_overall_mean_260 - 0.4503) < 0.001


def test_v1227_160_sum_value():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.v1227_160_sum - 160.0 * 0.7318) < 0.5


def test_v1227_260_sum_value():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.v1227_260_sum - 260.0 * 0.4503) < 0.5


def test_v1227_realized_less_than_north_star():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_overall_realized_160 < ASI_NORTH_STAR


def test_v1227_mean_less_than_1():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v1227_overall_mean_260 < 1.0


def test_v1227_courage_6_lifted_cells_sum():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    s = (
        rep.v1227_courage_x_r1_growth
        + rep.v1227_courage_x_r4_aging
        + rep.v1227_courage_x_r7_stress
        + rep.v1227_courage_x_r10_plasticity
        + rep.v1227_courage_x_r11_consciousness
        + rep.v1227_courage_x_r12_ecology
    )
    assert s == 6.0


def test_v1227_all_courage_pathways_have_cascade():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    for p_name, p_data in V1227_COURAGE_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        mols = p_data.get("molecules", [])
        assert len(cascade) > 0, f"{p_name} missing cascade"
        assert len(cascade) == len(mols), f"{p_name} cascade ≠ molecules count"


def test_v1227_all_courage_molecules_real():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import V1227_COURAGE_SUBSTRATE
    for p_name, p_data in V1227_COURAGE_SUBSTRATE.items():
        for mol in p_data.get("molecules", []):
            assert mol.get("real", False), f"{p_name}:{mol.get('name')} not real"


def test_v1227_pathway_score_threshold():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    for k, s in rep.pathway_scores.items():
        assert s >= 0.7, f"{k} score {s:.4f} below threshold"


def test_v1227_pathway_real_molecule_count_60():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    total = sum(rep.pathway_real_molecule_count.values())
    assert total == 60


def test_v1227_per_pathway_pass_count_by_r_substrate():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.n_r1_growth_pathways_pass == 1
    assert rep.n_r4_aging_pathways_pass == 1
    assert rep.n_r7_stress_pathways_pass == 1
    assert rep.n_r10_plasticity_pathways_pass == 1
    assert rep.n_r11_consciousness_pathways_pass == 1
    assert rep.n_r12_ecology_pathways_pass == 1


def test_v1227_per_pathway_molecule_count_by_r_substrate():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.n_r1_growth_molecules == 10
    assert rep.n_r4_aging_molecules == 10
    assert rep.n_r7_stress_molecules == 10
    assert rep.n_r10_plasticity_molecules == 10
    assert rep.n_r11_consciousness_molecules == 10
    assert rep.n_r12_ecology_molecules == 10


def test_v1227_inflation_gap_value():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert abs(rep.v1227_inflation_gap_v1226_minus_realized - 0.5497) < 0.001


def test_v1227_v3_guard_not_full_replace():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v3_guards["v1227_not_full_replace"] is True


def test_v1227_v3_guard_new_dim_not_full_coverage():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import measure_v1227_full
    rep = measure_v1227_full()
    assert rep.v3_guards["v1227_new_dim_not_full_coverage"] is True
