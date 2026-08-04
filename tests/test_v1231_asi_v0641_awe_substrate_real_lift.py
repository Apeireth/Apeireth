"""Tests for V1231 ASI V0.6.41 awe_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1230_REALIZED_MEAN_178 = 0.7590
V1230_OVERALL_MEAN_299 = 0.4517
V1230_CURIOSITY_REALIZED = 1.0000
V1229_REALIZED_MEAN_172 = 0.7505
V1229_OVERALL_MEAN_286 = 0.4513
V1229_CREATIVITY_REALIZED = 1.0000
V1228_REALIZED_MEAN_166 = 0.7415
V1228_OVERALL_MEAN_273 = 0.4508
V1228_TEMPERANCE_REALIZED = 1.0000
V1227_REALIZED_MEAN_160 = 0.7318
V1227_OVERALL_MEAN_260 = 0.4503
V1227_COURAGE_REALIZED = 1.0000


def test_v1231_module_imports():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        ASI_NORTH_STAR, V1231_DIM_VERSION, V1231_AWE_COVERAGE,
        V1231_AWE_SUBSTRATE, V1231_VERSION, V1231Report,
        measure_v1231_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1231_VERSION == "0.1.0"
    assert V1231_DIM_VERSION == "0.6.41"


def test_v1231_awe_substrate_6_pathways():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    assert len(V1231_AWE_SUBSTRATE) == 6
    expected = {
        "AWE_NEURO_DEFAULT", "AWE_LIFESPAN_DEV", "AWE_MOTIVATIONAL",
        "AWE_COGNITIVE", "AWE_PHILOSOPHY", "AWE_CULTURAL_SYSTEM",
    }
    assert set(V1231_AWE_SUBSTRATE.keys()) == expected


def test_v1231_awe_substrate_60_molecules():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1231_AWE_SUBSTRATE.values())
    assert total == 60


def test_v1231_awe_coverage_6_lifted():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
              "R11_consciousness", "R12_ecology"]:
        assert V1231_AWE_COVERAGE[k] == 1.0


def test_v1231_awe_neuro_references_key_papers():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    src = V1231_AWE_SUBSTRATE["AWE_NEURO_DEFAULT"]["source"]
    assert "Keltner" in src or "Piff" in src
    assert "Shiota" in src or "Yaden" in src


def test_v1231_awe_lifespan_references_key_papers():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    src = V1231_AWE_SUBSTRATE["AWE_LIFESPAN_DEV"]["source"]
    assert "Gopnik" in src
    assert "Vaillant" in src or "Cohen" in src


def test_v1231_awe_motivational_references_key_papers():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    src = V1231_AWE_SUBSTRATE["AWE_MOTIVATIONAL"]["source"]
    assert "Piff" in src
    assert "Norenzayan" in src or "Saroglou" in src


def test_v1231_awe_cognitive_references_key_papers():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    src = V1231_AWE_SUBSTRATE["AWE_COGNITIVE"]["source"]
    assert "Chirico" in src
    assert "Piff" in src or "Valdesolo" in src


def test_v1231_awe_philosophy_references_key_papers():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    src = V1231_AWE_SUBSTRATE["AWE_PHILOSOPHY"]["source"]
    assert "Kant" in src
    assert "Heidegger" in src
    assert "Otto" in src or "Wittgenstein" in src


def test_v1231_awe_cultural_references_key_papers():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    src = V1231_AWE_SUBSTRATE["AWE_CULTURAL_SYSTEM"]["source"]
    assert "Durkheim" in src
    assert "Geertz" in src or "Turner" in src


def test_v1231_measure_returns_report():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231Report, measure_v1231_full
    rep = measure_v1231_full()
    assert isinstance(rep, V1231Report)


def test_v1231_measure_dim_version():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.dim_version == "0.6.41"


def test_v1231_measure_elapsed_fast():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.elapsed < 1.0


def test_v1231_measure_awe_dim_realized_1():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_awe_dim_realized == 1.0000


def test_v1231_measure_awe_dim_cell_count_6():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_awe_dim_cell_count == 6


def test_v1231_measure_total_cells_299():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_total_cells == 299


def test_v1231_measure_realized_cells_count_184():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_realized_cells_count == 184


def test_v1231_measure_overall_realized_184_lift_positive():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_overall_realized_184 > V1230_REALIZED_MEAN_178


def test_v1231_measure_overall_realized_184_lift_delta():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    # +6/184 = +0.0326, minus V1230 baseline → ~+0.0079
    assert abs(rep.v1231_overall_lift_delta_realized_from_v1230 - 0.0079) < 0.002


def test_v1231_measure_inflation_gap_positive():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_inflation_gap_v1230_minus_realized > 0


def test_v1231_measure_position_north_star_pct():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    # Expected ~78.25% from V1231 lift, > V1230 77.44%
    assert 77.5 < rep.position_of_north_star_realized_pct < 79.0


def test_v1231_measure_total_awe_molecules():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.total_awe_molecules == 60


def test_v1231_measure_all_pathways_pass():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1231_v3_guards_all_pass():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1231_v3_guard_realized_not_asi():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1231_overall_realized_184 < ASI_NORTH_STAR


def test_v1231_v3_guard_60_mol_not_complete():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v3_guards["v1231_60_mol_not_complete"] is True


def test_v1231_v3_guard_awe_lift_not_full():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v3_guards["v1231_not_full_awe_lift"] is True


def test_v1231_artifact_default_path():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        measure_v1231_full, write_v1231_artifact,
    )
    rep = measure_v1231_full()
    path = write_v1231_artifact(rep)
    assert path.exists()


def test_v1231_report_default_path():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        measure_v1231_full, write_v1231_report,
    )
    rep = measure_v1231_full()
    path = write_v1231_report(rep)
    assert path.exists()


def test_v1231_artifact_valid_json():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        measure_v1231_full, write_v1231_artifact,
    )
    rep = measure_v1231_full()
    path = write_v1231_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.41"


def test_v1231_report_has_all_sections():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        measure_v1231_full, write_v1231_report,
    )
    rep = measure_v1231_full()
    path = write_v1231_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1231 ASI V0.6.41" in content
    assert "awe" in content.lower() or "敬畏" in content
    assert "闭环" in content or "闭环" in content
    assert "V3 哲学守门" in content


def test_v1231_cli_help():
    cmd = [sys.executable, "-m", "apeireth.v1231_asi_v0641_awe_substrate_real_lift", "--help"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1231" in result.stdout or "awe" in result.stdout.lower()


def test_v1231_cli_measure():
    cmd = [sys.executable, "-m", "apeireth.v1231_asi_v0641_awe_substrate_real_lift", "--measure"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1231_overall_realized_184" in result.stdout
    assert "0.6.41" in result.stdout


def test_v1231_cli_json():
    cmd = [sys.executable, "-m", "apeireth.v1231_asi_v0641_awe_substrate_real_lift", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1231_cli_report():
    cmd = [sys.executable, "-m", "apeireth.v1231_asi_v0641_awe_substrate_real_lift", "--report"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1231_cli_full():
    cmd = [sys.executable, "-m", "apeireth.v1231_asi_v0641_awe_substrate_real_lift", "--full"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "AWE coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1231_baseline_consistency_v1230():
    from apeireth.v1230_asi_v0640_curiosity_substrate_real_lift import (
        V1230_CURIOSITY_REALIZED as V0_V1230_CUR, V1230_OVERALL_MEAN_299 as V0_V1230_MEAN,
        V1230_REALIZED_MEAN_178 as V0_V1230_REAL,
    )
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        V1230_CURIOSITY_REALIZED, V1230_OVERALL_MEAN_299, V1230_REALIZED_MEAN_178,
    )
    assert V1230_CURIOSITY_REALIZED == V0_V1230_CUR
    assert V1230_OVERALL_MEAN_299 == V0_V1230_MEAN
    assert V1230_REALIZED_MEAN_178 == V0_V1230_REAL


def test_v1231_baseline_consistency_v1229():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1229_CREATIVITY_REALIZED as V9_V1229_CRE, V1229_OVERALL_MEAN_286 as V9_V1229_MEAN,
        V1229_REALIZED_MEAN_172 as V9_V1229_REAL,
    )
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        V1229_CREATIVITY_REALIZED, V1229_OVERALL_MEAN_286, V1229_REALIZED_MEAN_172,
    )
    assert V1229_CREATIVITY_REALIZED == V9_V1229_CRE
    assert V1229_OVERALL_MEAN_286 == V9_V1229_MEAN
    assert V1229_REALIZED_MEAN_172 == V9_V1229_REAL


def test_v1231_baseline_consistency_v1228():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        V1228_TEMPERANCE_REALIZED as V8_V1228_TEMP, V1228_OVERALL_MEAN_273 as V8_V1228_MEAN,
        V1228_REALIZED_MEAN_166 as V8_V1228_REAL,
    )
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        V1228_TEMPERANCE_REALIZED, V1228_OVERALL_MEAN_273, V1228_REALIZED_MEAN_166,
    )
    assert V1228_TEMPERANCE_REALIZED == V8_V1228_TEMP
    assert V1228_OVERALL_MEAN_273 == V8_V1228_MEAN
    assert V1228_REALIZED_MEAN_166 == V8_V1228_REAL


def test_v1231_north_star_locked_098():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import ASI_NORTH_STAR, measure_v1231_full
    assert ASI_NORTH_STAR == 0.9800


def test_v1231_realized_less_than_one():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_overall_realized_184 < 1.0


def test_v1231_pathways_all_have_real_molecules():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        V1231_AWE_SUBSTRATE, measure_v1231_full,
    )
    for pathway_name, pathway_data in V1231_AWE_SUBSTRATE.items():
        mols = pathway_data.get("molecules", [])
        assert len(mols) >= 10, f"{pathway_name} has only {len(mols)} molecules"
        real_count = sum(1 for m in mols if m.get("real", False))
        assert real_count == len(mols), f"{pathway_name} has non-real molecules"


def test_v1231_each_molecule_has_required_fields():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    for pathway_name, pathway_data in V1231_AWE_SUBSTRATE.items():
        for m in pathway_data.get("molecules", []):
            assert "name" in m, f"{pathway_name} molecule missing name"
            assert "function" in m, f"{pathway_name} missing function"
            assert "real" in m, f"{pathway_name} missing real flag"
            assert "organism" in m, f"{pathway_name} missing organism"


def test_v1231_cascade_orders_match_molecule_counts():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    for pathway_name, pathway_data in V1231_AWE_SUBSTRATE.items():
        cascade = pathway_data.get("cascade_order", [])
        mols = pathway_data.get("molecules", [])
        assert len(cascade) == len(mols), (
            f"{pathway_name}: cascade {len(cascade)} vs molecules {len(mols)}"
        )


def test_v1231_r_coverage_has_12_dim_keys():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_COVERAGE
    expected_keys = {
        "R1_growth", "R2_sensing", "R3_cognition", "R4_aging",
        "R5_social", "R6_communication", "R7_stress", "R8_motion",
        "R9_heredity", "R10_plasticity", "R11_consciousness", "R12_ecology",
    }
    assert set(V1231_AWE_COVERAGE.keys()) == expected_keys


def test_v1231_awe_substrate_not_v1():
    """V3 guard: awe != ASI V1.0."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v3_guards["v1231_lift_not_v1"] is True
    assert rep.v3_guards["v1231_not_asi_terminal"] is True


def test_v1231_awe_x_r11_philosophy_covered():
    """R11 consciousness (philosophy) needs a pathway lifted."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        V1231_AWE_COVERAGE, V1231_AWE_SUBSTRATE,
    )
    assert V1231_AWE_COVERAGE["R11_consciousness"] == 1.0
    assert "AWE_PHILOSOPHY" in V1231_AWE_SUBSTRATE


def test_v1231_awe_x_r12_cultural_covered():
    """R12 ecology (cultural-system) needs a pathway lifted."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import (
        V1231_AWE_COVERAGE, V1231_AWE_SUBSTRATE,
    )
    assert V1231_AWE_COVERAGE["R12_ecology"] == 1.0
    assert "AWE_CULTURAL_SYSTEM" in V1231_AWE_SUBSTRATE


def test_v1231_dim_version_string():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_DIM_VERSION
    assert V1231_DIM_VERSION == "0.6.41"


def test_v1231_module_version_string():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_VERSION
    assert V1231_VERSION == "0.1.0"


def test_v1231_six_pathways_cover_six_r_substrates():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    expected_rs = {
        "R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
        "R11_consciousness", "R12_ecology",
    }
    actual_rs = {p["r_substrate"] for p in V1231_AWE_SUBSTRATE.values()}
    assert actual_rs == expected_rs


def test_v1231_each_pathway_has_cascade_order():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    for pathway_name, pathway_data in V1231_AWE_SUBSTRATE.items():
        assert "cascade_order" in pathway_data, f"{pathway_name} missing cascade_order"
        assert "description" in pathway_data, f"{pathway_name} missing description"
        assert "source" in pathway_data, f"{pathway_name} missing source"


def test_v1231_awe_all_molecules_have_organism():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    for pathway_data in V1231_AWE_SUBSTRATE.values():
        for m in pathway_data.get("molecules", []):
            assert m.get("organism") in ("human", "rat", "mouse", "monkey", "ape"), (
                f"{m.get('name')}: unknown organism {m.get('organism')}"
            )


def test_v1231_position_north_star_above_v1230():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    # V1230 was 77.44%, V1231 should be ~78.25% (higher)
    assert rep.position_of_north_star_realized_pct > 77.44


def test_v1231_lift_mean_positive():
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_overall_lift_delta_mean_from_v1230 > 0


def test_v1231_lift_realized_value_check():
    """V1231 lift from V1230 should be approximately +0.0079."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    # (V1230 sum + 6) / 184 - V1230 mean = (0.7590*178 + 6)/184 - 0.7590 ≈ 0.7669 - 0.7590 = +0.0079
    expected = ((V1230_REALIZED_MEAN_178 * 178.0 + 6.0) / 184.0) - V1230_REALIZED_MEAN_178
    assert abs(rep.v1231_overall_lift_delta_realized_from_v1230 - expected) < 0.0002


def test_v1231_lift_mean_value_check():
    """V1231 mean lift from V1230 should be approximately +0.0201."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    # (V1230 mean sum + 6) / 299 - V1230 mean = (0.4517*299 + 6)/299 - 0.4517 ≈ 0.4718 - 0.4517 = +0.0201
    expected = ((V1230_OVERALL_MEAN_299 * 299.0 + 6.0) / 299.0) - V1230_OVERALL_MEAN_299
    assert abs(rep.v1231_overall_lift_delta_mean_from_v1230 - expected) < 0.0002


def test_v1231_inflation_gap_value_check():
    """Inflation gap = 1.0 - V1231 mean_299."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    expected = 1.0 - rep.v1231_overall_mean_299
    assert abs(rep.v1231_inflation_gap_v1230_minus_realized - expected) < 0.001


def test_v1231_position_value_check():
    """Position = (realized_184 / ASI_NORTH_STAR) * 100."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    expected = (rep.v1231_overall_realized_184 / ASI_NORTH_STAR) * 100.0
    assert abs(rep.position_of_north_star_realized_pct - expected) < 0.01


def test_v1231_neuro_molecule_keltner_present():
    """Keltner Haidt 2003 must be present in neuro pathway."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    mols = V1231_AWE_SUBSTRATE["AWE_NEURO_DEFAULT"]["molecules"]
    assert any("Keltner" in m["name"] for m in mols)


def test_v1231_philosophy_molecule_kant_present():
    """Kant 1790 must be present in philosophy pathway."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    mols = V1231_AWE_SUBSTRATE["AWE_PHILOSOPHY"]["molecules"]
    assert any("Kant" in m["name"] for m in mols)


def test_v1231_motivational_molecule_norenzayan_present():
    """Norenzayan big gods must be present in motivational pathway."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    mols = V1231_AWE_SUBSTRATE["AWE_MOTIVATIONAL"]["molecules"]
    assert any("Norenzayan" in m["name"] for m in mols)


def test_v1231_cognitive_molecule_chirico_present():
    """Chirico virtual reality must be present in cognitive pathway."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    mols = V1231_AWE_SUBSTRATE["AWE_COGNITIVE"]["molecules"]
    assert any("Chirico" in m["name"] for m in mols)


def test_v1231_lifespan_molecule_gopnik_present():
    """Gopnik child as scientist must be present in lifespan pathway."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    mols = V1231_AWE_SUBSTRATE["AWE_LIFESPAN_DEV"]["molecules"]
    assert any("Gopnik" in m["name"] for m in mols)


def test_v1231_cultural_molecule_durkheim_present():
    """Durkheim collective effervescence must be present in cultural pathway."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import V1231_AWE_SUBSTRATE
    mols = V1231_AWE_SUBSTRATE["AWE_CULTURAL_SYSTEM"]["molecules"]
    assert any("Durkheim" in m["name"] for m in mols)


def test_v1231_position_above_78():
    """V1231 must reach > 78% ASI North Star."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.position_of_north_star_realized_pct > 78.0


def test_v1231_23_dim_matrix_math():
    """Sanity: 23 * 13 = 299."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_total_cells == 23 * 13
    assert rep.v1231_total_cells == 299


def test_v1231_realized_count_equals_178_plus_6():
    """Sanity: V1230 had 178 realized, V1231 adds 6 awe cells."""
    from apeireth.v1231_asi_v0641_awe_substrate_real_lift import measure_v1231_full
    rep = measure_v1231_full()
    assert rep.v1231_realized_cells_count == 178 + rep.v1231_awe_dim_cell_count
