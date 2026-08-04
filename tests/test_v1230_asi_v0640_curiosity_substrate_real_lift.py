"""Tests for V1230 ASI V0.6.40 curiosity_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1229_REALIZED_MEAN_172 = 0.7505
V1229_OVERALL_MEAN_286 = 0.4513
V1229_CREATIVITY_REALIZED = 1.0000
V1228_REALIZED_MEAN_166 = 0.7415
V1228_OVERALL_MEAN_273 = 0.4508
V1228_TEMPERANCE_REALIZED = 1.0000
V1227_REALIZED_MEAN_160 = 0.7318
V1227_OVERALL_MEAN_260 = 0.4503
V1227_COURAGE_REALIZED = 1.0000
V1226_HOP_REALIZED = 1.0000
V1226_OVERALL_MEAN_247 = 0.4497
V1226_REALIZED_MEAN_154 = 0.7214


def test_v1230_module_imports():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        ASI_NORTH_STAR, V1230_DIM_VERSION, V1230_CURIOSITY_COVERAGE,
        V1230_CURIOSITY_SUBSTRATE, V1230_VERSION, V1230Report,
        measure_v1230_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1230_VERSION == "0.1.0"
    assert V1230_DIM_VERSION == "0.6.40"


def test_v1230_curiosity_substrate_6_pathways():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    assert len(V1230_CURIOSITY_SUBSTRATE) == 6
    expected = {
        "CUR_NEURO_DEFAULT", "CUR_LIFESPAN_DEV", "CUR_MOTIVATIONAL",
        "CUR_COGNITIVE", "CUR_PHILOSOPHY", "CUR_CULTURAL_SYSTEM",
    }
    assert set(V1230_CURIOSITY_SUBSTRATE.keys()) == expected


def test_v1230_curiosity_substrate_60_molecules():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1230_CURIOSITY_SUBSTRATE.values())
    assert total == 60


def test_v1230_curiosity_coverage_6_lifted():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
              "R11_consciousness", "R12_ecology"]:
        assert V1230_CURIOSITY_COVERAGE[k] == 1.0


def test_v1230_curiosity_neuro_references_key_papers():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    src = V1230_CURIOSITY_SUBSTRATE["CUR_NEURO_DEFAULT"]["source"]
    assert "Berlyne" in src or "Panksepp" in src
    assert "Litman" in src or "Gruber" in src


def test_v1230_curiosity_lifespan_references_key_papers():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    src = V1230_CURIOSITY_SUBSTRATE["CUR_LIFESPAN_DEV"]["source"]
    assert "Piaget" in src or "Engel" in src
    assert "Ainley" in src or "Gross" in src


def test_v1230_curiosity_motivational_references_key_papers():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    src = V1230_CURIOSITY_SUBSTRATE["CUR_MOTIVATIONAL"]["source"]
    assert "Litman" in src
    assert "Kashdan" in src


def test_v1230_curiosity_cognitive_references_key_papers():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    src = V1230_CURIOSITY_SUBSTRATE["CUR_COGNITIVE"]["source"]
    assert "Loewenstein" in src
    assert "Gopnik" in src or "Schulz" in src


def test_v1230_curiosity_philosophy_references_key_papers():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    src = V1230_CURIOSITY_SUBSTRATE["CUR_PHILOSOPHY"]["source"]
    assert "Peirce" in src
    assert "Heidegger" in src
    assert "James" in src or "Dewey" in src


def test_v1230_curiosity_cultural_references_key_papers():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    src = V1230_CURIOSITY_SUBSTRATE["CUR_CULTURAL_SYSTEM"]["source"]
    assert "Kashdan" in src
    assert "Hofstede" in src or "Markus" in src


def test_v1230_measure_returns_report():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230Report, measure_v1230_full
    rep = measure_v1230_full()
    assert isinstance(rep, V1230Report)


def test_v1230_measure_dim_version():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.dim_version == "0.6.40"


def test_v1230_measure_elapsed_fast():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.elapsed < 1.0


def test_v1230_measure_curiosity_dim_realized_1():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_curiosity_dim_realized == 1.0000


def test_v1230_measure_curiosity_dim_cell_count_6():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_curiosity_dim_cell_count == 6


def test_v1230_measure_total_cells_299():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_total_cells == 299


def test_v1230_measure_realized_cells_count_178():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_realized_cells_count == 178


def test_v1230_measure_overall_realized_178_lift_positive():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_overall_realized_178 > V1229_REALIZED_MEAN_172


def test_v1230_measure_overall_realized_178_lift_delta():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    # +6/178 = +0.0337, minus lost precision = ~+0.0085
    assert abs(rep.v1230_overall_lift_delta_realized_from_v1229 - 0.0085) < 0.002


def test_v1230_measure_inflation_gap_positive():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_inflation_gap_v1229_minus_realized > 0


def test_v1230_measure_position_north_star_pct():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    # Expected ~77.45% from V1230 lift, > V1229 76.58%
    assert 77.0 < rep.position_of_north_star_realized_pct < 78.5


def test_v1230_measure_total_curiosity_molecules():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.total_curiosity_molecules == 60


def test_v1230_measure_all_pathways_pass():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1230_v3_guards_all_pass():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1230_v3_guard_realized_not_asi():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1230_overall_realized_178 < ASI_NORTH_STAR


def test_v1230_v3_guard_60_mol_not_complete():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v3_guards["v1230_60_mol_not_complete"] is True


def test_v1230_v3_guard_curiosity_lift_not_full():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v3_guards["v1230_not_full_curiosity_lift"] is True


def test_v1230_artifact_default_path():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        measure_v1230_full, write_v1230_artifact,
    )
    rep = measure_v1230_full()
    path = write_v1230_artifact(rep)
    assert path.exists()


def test_v1230_report_default_path():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        measure_v1230_full, write_v1230_report,
    )
    rep = measure_v1230_full()
    path = write_v1230_report(rep)
    assert path.exists()


def test_v1230_artifact_valid_json():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        measure_v1230_full, write_v1230_artifact,
    )
    rep = measure_v1230_full()
    path = write_v1230_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.40"


def test_v1230_report_has_all_sections():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        measure_v1230_full, write_v1230_report,
    )
    rep = measure_v1230_full()
    path = write_v1230_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1230 ASI V0.6.40" in content
    assert "curiosity" in content.lower() or "好奇" in content
    assert "探索" in content or "创造" in content or "cardinal" in content.lower()
    assert "V3 哲学守门" in content


def test_v1230_cli_help():
    cmd = [sys.executable, "-m", "apeireth.v1230_asi_v0640_curiosity_substrate_real_lift", "--help"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1230" in result.stdout or "curiosity" in result.stdout.lower()


def test_v1230_cli_measure():
    cmd = [sys.executable, "-m", "apeireth.v1230_asi_v0640_curiosity_substrate_real_lift", "--measure"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1230_overall_realized_178" in result.stdout
    assert "0.6.40" in result.stdout


def test_v1230_cli_json():
    cmd = [sys.executable, "-m", "apeireth.v1230_asi_v0640_curiosity_substrate_real_lift", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1230_cli_report():
    cmd = [sys.executable, "-m", "apeireth.v1230_asi_v0640_curiosity_substrate_real_lift", "--report"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1230_cli_full():
    cmd = [sys.executable, "-m", "apeireth.v1230_asi_v0640_curiosity_substrate_real_lift", "--full"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "CURIOSITY coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1230_baseline_consistency_v1229():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1229_CREATIVITY_REALIZED as V9_V1229_CRE, V1229_OVERALL_MEAN_286 as V9_V1229_MEAN,
        V1229_REALIZED_MEAN_172 as V9_V1229_REAL,
    )
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        V1229_CREATIVITY_REALIZED, V1229_OVERALL_MEAN_286, V1229_REALIZED_MEAN_172,
    )
    assert V1229_CREATIVITY_REALIZED == V9_V1229_CRE
    assert V1229_OVERALL_MEAN_286 == V9_V1229_MEAN
    assert V1229_REALIZED_MEAN_172 == V9_V1229_REAL


def test_v1230_baseline_consistency_v1228():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        V1228_TEMPERANCE_REALIZED as V8_V1228_TEMP, V1228_OVERALL_MEAN_273 as V8_V1228_MEAN,
        V1228_REALIZED_MEAN_166 as V8_V1228_REAL,
    )
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        V1228_TEMPERANCE_REALIZED, V1228_OVERALL_MEAN_273, V1228_REALIZED_MEAN_166,
    )
    assert V1228_TEMPERANCE_REALIZED == V8_V1228_TEMP
    assert V1228_OVERALL_MEAN_273 == V8_V1228_MEAN
    assert V1228_REALIZED_MEAN_166 == V8_V1228_REAL


def test_v1230_north_star_locked_098():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import ASI_NORTH_STAR, measure_v1230_full
    assert ASI_NORTH_STAR == 0.9800


def test_v1230_realized_less_than_one():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_overall_realized_178 < 1.0


def test_v1230_pathways_all_have_real_molecules():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        V1230_CURIOSITY_SUBSTRATE, measure_v1230_full,
    )
    for pathway_name, pathway_data in V1230_CURIOSITY_SUBSTRATE.items():
        mols = pathway_data.get("molecules", [])
        assert len(mols) >= 10, f"{pathway_name} has only {len(mols)} molecules"
        real_count = sum(1 for m in mols if m.get("real", False))
        assert real_count == len(mols), f"{pathway_name} has non-real molecules"


def test_v1230_each_molecule_has_required_fields():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    for pathway_name, pathway_data in V1230_CURIOSITY_SUBSTRATE.items():
        for m in pathway_data.get("molecules", []):
            assert "name" in m, f"{pathway_name} molecule missing name"
            assert "function" in m, f"{pathway_name} missing function"
            assert "real" in m, f"{pathway_name} missing real flag"
            assert "organism" in m, f"{pathway_name} missing organism"


def test_v1230_cascade_orders_match_molecule_counts():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    for pathway_name, pathway_data in V1230_CURIOSITY_SUBSTRATE.items():
        cascade = pathway_data.get("cascade_order", [])
        mols = pathway_data.get("molecules", [])
        assert len(cascade) == len(mols), (
            f"{pathway_name}: cascade {len(cascade)} vs molecules {len(mols)}"
        )


def test_v1230_r_coverage_has_12_dim_keys():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_COVERAGE
    expected_keys = {
        "R1_growth", "R2_sensing", "R3_cognition", "R4_aging",
        "R5_social", "R6_communication", "R7_stress", "R8_motion",
        "R9_heredity", "R10_plasticity", "R11_consciousness", "R12_ecology",
    }
    assert set(V1230_CURIOSITY_COVERAGE.keys()) == expected_keys


def test_v1230_curiosity_substrate_not_v1():
    """V3 guard: curiosity != ASI V1.0."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v3_guards["v1230_lift_not_v1"] is True
    assert rep.v3_guards["v1230_not_asi_terminal"] is True


def test_v1230_curiosity_x_r11_philosophy_covered():
    """R11 consciousness (philosophy) needs a pathway lifted."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        V1230_CURIOSITY_COVERAGE, V1230_CURIOSITY_SUBSTRATE,
    )
    assert V1230_CURIOSITY_COVERAGE["R11_consciousness"] == 1.0
    assert "CUR_PHILOSOPHY" in V1230_CURIOSITY_SUBSTRATE


def test_v1230_curiosity_x_r12_cultural_covered():
    """R12 ecology (cultural-system) needs a pathway lifted."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        V1230_CURIOSITY_COVERAGE, V1230_CURIOSITY_SUBSTRATE,
    )
    assert V1230_CURIOSITY_COVERAGE["R12_ecology"] == 1.0
    assert "CUR_CULTURAL_SYSTEM" in V1230_CURIOSITY_SUBSTRATE


def test_v1230_dim_version_string():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_DIM_VERSION
    assert V1230_DIM_VERSION == "0.6.40"


def test_v1230_module_version_string():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_VERSION
    assert V1230_VERSION == "0.1.0"


def test_v1230_six_pathways_cover_six_r_substrates():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    expected_rs = {
        "R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
        "R11_consciousness", "R12_ecology",
    }
    actual_rs = {p["r_substrate"] for p in V1230_CURIOSITY_SUBSTRATE.values()}
    assert actual_rs == expected_rs


def test_v1230_each_pathway_has_cascade_order():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    for pathway_name, pathway_data in V1230_CURIOSITY_SUBSTRATE.items():
        assert "cascade_order" in pathway_data, f"{pathway_name} missing cascade_order"
        assert "description" in pathway_data, f"{pathway_name} missing description"
        assert "source" in pathway_data, f"{pathway_name} missing source"


def test_v1230_curiosity_all_molecules_have_organism():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    for pathway_data in V1230_CURIOSITY_SUBSTRATE.values():
        for m in pathway_data.get("molecules", []):
            assert m.get("organism") in ("human", "rat", "mouse", "monkey", "ape"), (
                f"{m.get('name')}: unknown organism {m.get('organism')}"
            )


def test_v1230_position_north_star_above_v1229():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    # V1229 was 76.58%, V1230 should be ~77.45% (higher)
    assert rep.position_of_north_star_realized_pct > 76.58


def test_v1230_lift_mean_positive():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_overall_lift_delta_mean_from_v1229 > 0


def test_v1230_lift_realized_value_check():
    """V1230 lift from V1229 should be approximately +0.0085."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    # (V1229 sum + 6) / 178 - V1229 mean = (129.086 + 6)/178 - 0.7505 ≈ 0.7590 - 0.7505 = +0.0085
    expected = ((V1229_REALIZED_MEAN_172 * 172.0 + 6.0) / 178.0) - V1229_REALIZED_MEAN_172
    assert abs(rep.v1230_overall_lift_delta_realized_from_v1229 - expected) < 0.0002


def test_v1230_lift_mean_value_check():
    """V1230 mean lift from V1229 should be approximately +0.0004."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    # (V1229 mean sum + 6) / 299 - V1229 mean = (0.4513*286 + 6)/299 - 0.4513 ≈ 0.4517 - 0.4513 = +0.0004
    expected = ((V1229_OVERALL_MEAN_286 * 286.0 + 6.0) / 299.0) - V1229_OVERALL_MEAN_286
    assert abs(rep.v1230_overall_lift_delta_mean_from_v1229 - expected) < 0.0002


def test_v1230_inflation_gap_value_check():
    """Inflation gap = 1.0 - V1230 mean_299."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    expected = 1.0 - rep.v1230_overall_mean_299
    assert abs(rep.v1230_inflation_gap_v1229_minus_realized - expected) < 0.001


def test_v1230_position_value_check():
    """Position = (realized_178 / ASI_NORTH_STAR) * 100."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    expected = (rep.v1230_overall_realized_178 / ASI_NORTH_STAR) * 100.0
    assert abs(rep.position_of_north_star_realized_pct - expected) < 0.01


def test_v1230_neuro_molecule_berlyne_present():
    """Berlyne 1954 conflict theory must be present in neuro pathway."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    mols = V1230_CURIOSITY_SUBSTRATE["CUR_NEURO_DEFAULT"]["molecules"]
    assert any("Berlyne" in m["name"] for m in mols)


def test_v1230_philosophy_molecule_heidegger_present():
    """Heidegger must be present in philosophy pathway."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    mols = V1230_CURIOSITY_SUBSTRATE["CUR_PHILOSOPHY"]["molecules"]
    assert any("Heidegger" in m["name"] for m in mols)


def test_v1230_motivational_molecule_litman_present():
    """Litman I/D must be present in motivational pathway."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    mols = V1230_CURIOSITY_SUBSTRATE["CUR_MOTIVATIONAL"]["molecules"]
    assert any("Litman" in m["name"] for m in mols)


def test_v1230_cognitive_molecule_loewenstein_present():
    """Loewenstein information gap must be present in cognitive pathway."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    mols = V1230_CURIOSITY_SUBSTRATE["CUR_COGNITIVE"]["molecules"]
    assert any("Loewenstein" in m["name"] for m in mols)


def test_v1230_lifespan_molecule_piaget_present():
    """Piaget curiosity origin must be present in lifespan pathway."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    mols = V1230_CURIOSITY_SUBSTRATE["CUR_LIFESPAN_DEV"]["molecules"]
    assert any("Piaget" in m["name"] for m in mols)


def test_v1230_cultural_molecule_hofstede_present():
    """Hofstede cultural must be present in cultural pathway."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import V1230_CURIOSITY_SUBSTRATE
    mols = V1230_CURIOSITY_SUBSTRATE["CUR_CULTURAL_SYSTEM"]["molecules"]
    assert any("Hofstede" in m["name"] for m in mols)


def test_v1230_position_above_77():
    """V1230 must reach > 77% ASI North Star."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.position_of_north_star_realized_pct > 77.0


def test_v1230_23_dim_matrix_math():
    """Sanity: 23 * 13 = 299."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_total_cells == 23 * 13
    assert rep.v1230_total_cells == 299


def test_v1230_realized_count_equals_172_plus_6():
    """Sanity: V1229 had 172 realized, V1230 adds 6 curiosity cells."""
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import measure_v1230_full
    rep = measure_v1230_full()
    assert rep.v1230_realized_cells_count == 172 + rep.v1230_curiosity_dim_cell_count