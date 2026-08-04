"""Tests for V1229 ASI V0.6.39 creativity_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1228_REALIZED_MEAN_166 = 0.7415
V1228_OVERALL_MEAN_273 = 0.4508
V1228_TEMPERANCE_REALIZED = 1.0000
V1227_REALIZED_MEAN_160 = 0.7318
V1227_OVERALL_MEAN_260 = 0.4503
V1227_COURAGE_REALIZED = 1.0000
V1226_HOP_REALIZED = 1.0000
V1226_OVERALL_MEAN_247 = 0.4497
V1226_REALIZED_MEAN_154 = 0.7214


def test_v1229_module_imports():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        ASI_NORTH_STAR, V1229_DIM_VERSION, V1229_CREATIVITY_COVERAGE,
        V1229_CREATIVITY_SUBSTRATE, V1229_VERSION, V1229Report,
        measure_v1229_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1229_VERSION == "0.1.0"
    assert V1229_DIM_VERSION == "0.6.39"


def test_v1229_creativity_substrate_6_pathways():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    assert len(V1229_CREATIVITY_SUBSTRATE) == 6
    expected = {
        "CRE_NEURO_DEFAULT", "CRE_LIFESPAN_DEV", "CRE_COMPOSITIONAL",
        "CRE_ASSOCIATIVE", "CRE_PHILOSOPHY", "CRE_CULTURAL_SYSTEM",
    }
    assert set(V1229_CREATIVITY_SUBSTRATE.keys()) == expected


def test_v1229_creativity_substrate_60_molecules():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1229_CREATIVITY_SUBSTRATE.values())
    assert total == 60


def test_v1229_creativity_coverage_6_lifted():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
              "R11_consciousness", "R12_ecology"]:
        assert V1229_CREATIVITY_COVERAGE[k] == 1.0


def test_v1229_creativity_neuro_references_key_papers():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    src = V1229_CREATIVITY_SUBSTRATE["CRE_NEURO_DEFAULT"]["source"]
    assert "Buckner" in src or "Limb" in src
    assert "Kounios" in src or "Beaty" in src


def test_v1229_creativity_lifespan_references_key_papers():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    src = V1229_CREATIVITY_SUBSTRATE["CRE_LIFESPAN_DEV"]["source"]
    assert "Simonton" in src
    assert "Lubart" in src or "Carson" in src


def test_v1229_creativity_componential_references_key_papers():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    src = V1229_CREATIVITY_SUBSTRATE["CRE_COMPOSITIONAL"]["source"]
    assert "Amabile" in src
    assert "Csikszentmihalyi" in src
    assert "Sternberg" in src


def test_v1229_creativity_associative_references_key_papers():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    src = V1229_CREATIVITY_SUBSTRATE["CRE_ASSOCIATIVE"]["source"]
    assert "Mednick" in src
    assert "Finke" in src
    assert "Ward" in src


def test_v1229_creativity_philosophy_references_key_papers():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    src = V1229_CREATIVITY_SUBSTRATE["CRE_PHILOSOPHY"]["source"]
    assert "Peirce" in src
    assert "Boden" in src
    assert "Koestler" in src
    assert "abduction" in src or "bisociation" in src or "transformational" in src


def test_v1229_creativity_cultural_references_key_papers():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    src = V1229_CREATIVITY_SUBSTRATE["CRE_CULTURAL_SYSTEM"]["source"]
    assert "Csikszentmihalyi" in src
    assert "Sawyer" in src
    assert "Beghetto" in src


def test_v1229_measure_returns_report():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229Report, measure_v1229_full
    rep = measure_v1229_full()
    assert isinstance(rep, V1229Report)


def test_v1229_measure_dim_version():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.dim_version == "0.6.39"


def test_v1229_measure_elapsed_fast():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.elapsed < 1.0


def test_v1229_measure_creativity_dim_realized_1():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_creativity_dim_realized == 1.0000


def test_v1229_measure_creativity_dim_cell_count_6():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_creativity_dim_cell_count == 6


def test_v1229_measure_total_cells_286():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_total_cells == 286


def test_v1229_measure_realized_cells_count_172():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_realized_cells_count == 172


def test_v1229_measure_overall_realized_172_lift_positive():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_overall_realized_172 > V1228_REALIZED_MEAN_166


def test_v1229_measure_overall_realized_172_lift_delta():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert abs(rep.v1229_overall_lift_delta_realized_from_v1228 - 0.0090) < 0.002


def test_v1229_measure_inflation_gap_positive():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_inflation_gap_v1228_minus_realized > 0


def test_v1229_measure_position_north_star_pct():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    # Expected ~76.58% from V1229 lift, > V1228 75.66%
    assert 76.0 < rep.position_of_north_star_realized_pct < 77.5


def test_v1229_measure_total_creativity_molecules():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.total_creativity_molecules == 60


def test_v1229_measure_all_pathways_pass():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1229_v3_guards_all_pass():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1229_v3_guard_realized_not_asi():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1229_overall_realized_172 < ASI_NORTH_STAR


def test_v1229_v3_guard_60_mol_not_complete():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v3_guards["v1229_60_mol_not_complete"] is True


def test_v1229_v3_guard_creativity_lift_not_full():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v3_guards["v1229_not_full_creativity_lift"] is True


def test_v1229_artifact_default_path():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        measure_v1229_full, write_v1229_artifact,
    )
    rep = measure_v1229_full()
    path = write_v1229_artifact(rep)
    assert path.exists()


def test_v1229_report_default_path():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        measure_v1229_full, write_v1229_report,
    )
    rep = measure_v1229_full()
    path = write_v1229_report(rep)
    assert path.exists()


def test_v1229_artifact_valid_json():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        measure_v1229_full, write_v1229_artifact,
    )
    rep = measure_v1229_full()
    path = write_v1229_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.39"


def test_v1229_report_has_all_sections():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        measure_v1229_full, write_v1229_report,
    )
    rep = measure_v1229_full()
    path = write_v1229_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1229 ASI V0.6.39" in content
    assert "creativity" in content.lower() or "创造" in content
    assert "真生产" in content or "cardinal" in content.lower() or "创造力" in content
    assert "V3 哲学守门" in content


def test_v1229_cli_help():
    cmd = [sys.executable, "-m", "apeireth.v1229_asi_v0639_creativity_substrate_real_lift", "--help"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1229" in result.stdout or "creativity" in result.stdout.lower()


def test_v1229_cli_measure():
    cmd = [sys.executable, "-m", "apeireth.v1229_asi_v0639_creativity_substrate_real_lift", "--measure"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1229_overall_realized_172" in result.stdout
    assert "0.6.39" in result.stdout


def test_v1229_cli_json():
    cmd = [sys.executable, "-m", "apeireth.v1229_asi_v0639_creativity_substrate_real_lift", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1229_cli_report():
    cmd = [sys.executable, "-m", "apeireth.v1229_asi_v0639_creativity_substrate_real_lift", "--report"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1229_cli_full():
    cmd = [sys.executable, "-m", "apeireth.v1229_asi_v0639_creativity_substrate_real_lift", "--full"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "CREATIVITY coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1229_baseline_consistency_v1228():
    from apeireth.v1228_asi_v0638_temperance_substrate_real_lift import (
        V1228_TEMPERANCE_REALIZED as V8_V1228_TEMP, V1228_OVERALL_MEAN_273 as V8_V1228_MEAN,
        V1228_REALIZED_MEAN_166 as V8_V1228_REAL,
    )
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1228_TEMPERANCE_REALIZED, V1228_OVERALL_MEAN_273, V1228_REALIZED_MEAN_166,
    )
    assert V1228_TEMPERANCE_REALIZED == V8_V1228_TEMP
    assert V1228_OVERALL_MEAN_273 == V8_V1228_MEAN
    assert V1228_REALIZED_MEAN_166 == V8_V1228_REAL


def test_v1229_baseline_consistency_v1227():
    from apeireth.v1227_asi_v0637_courage_substrate_real_lift import (
        V1227_COURAGE_REALIZED as V7_V1227_COURAGE, V1227_OVERALL_MEAN_260 as V7_V1227_MEAN,
        V1227_REALIZED_MEAN_160 as V7_V1227_REAL,
    )
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1227_COURAGE_REALIZED, V1227_OVERALL_MEAN_260, V1227_REALIZED_MEAN_160,
    )
    assert V1227_COURAGE_REALIZED == V7_V1227_COURAGE
    assert V1227_OVERALL_MEAN_260 == V7_V1227_MEAN
    assert V1227_REALIZED_MEAN_160 == V7_V1227_REAL


def test_v1229_north_star_locked_098():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import ASI_NORTH_STAR, measure_v1229_full
    assert ASI_NORTH_STAR == 0.9800


def test_v1229_realized_less_than_one():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_overall_realized_172 < 1.0


def test_v1229_pathways_all_have_real_molecules():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1229_CREATIVITY_SUBSTRATE, measure_v1229_full,
    )
    for pathway_name, pathway_data in V1229_CREATIVITY_SUBSTRATE.items():
        mols = pathway_data.get("molecules", [])
        assert len(mols) >= 10, f"{pathway_name} has only {len(mols)} molecules"
        real_count = sum(1 for m in mols if m.get("real", False))
        assert real_count == len(mols), f"{pathway_name} has non-real molecules"


def test_v1229_each_molecule_has_required_fields():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    for pathway_name, pathway_data in V1229_CREATIVITY_SUBSTRATE.items():
        for m in pathway_data.get("molecules", []):
            assert "name" in m, f"{pathway_name} molecule missing name"
            assert "function" in m, f"{pathway_name} missing function"
            assert "real" in m, f"{pathway_name} missing real flag"
            assert "organism" in m, f"{pathway_name} missing organism"


def test_v1229_cascade_orders_match_molecule_counts():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    for pathway_name, pathway_data in V1229_CREATIVITY_SUBSTRATE.items():
        cascade = pathway_data.get("cascade_order", [])
        mols = pathway_data.get("molecules", [])
        assert len(cascade) == len(mols), (
            f"{pathway_name}: cascade {len(cascade)} vs molecules {len(mols)}"
        )


def test_v1229_r_coverage_has_12_dim_keys():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_COVERAGE
    expected_keys = {
        "R1_growth", "R2_sensing", "R3_cognition", "R4_aging",
        "R5_social", "R6_communication", "R7_stress", "R8_motion",
        "R9_heredity", "R10_plasticity", "R11_consciousness", "R12_ecology",
    }
    assert set(V1229_CREATIVITY_COVERAGE.keys()) == expected_keys


def test_v1229_creativity_substrate_not_v1():
    """V3 guard: creativity != ASI V1.0."""
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v3_guards["v1229_lift_not_v1"] is True
    assert rep.v3_guards["v1229_not_asi_terminal"] is True


def test_v1229_creativity_x_r11_philosophy_covered():
    """R11 consciousness (philosophy) needs a pathway lifted."""
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1229_CREATIVITY_COVERAGE, V1229_CREATIVITY_SUBSTRATE,
    )
    assert V1229_CREATIVITY_COVERAGE["R11_consciousness"] == 1.0
    assert "CRE_PHILOSOPHY" in V1229_CREATIVITY_SUBSTRATE


def test_v1229_creativity_x_r12_cultural_covered():
    """R12 ecology (cultural-system) needs a pathway lifted."""
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import (
        V1229_CREATIVITY_COVERAGE, V1229_CREATIVITY_SUBSTRATE,
    )
    assert V1229_CREATIVITY_COVERAGE["R12_ecology"] == 1.0
    assert "CRE_CULTURAL_SYSTEM" in V1229_CREATIVITY_SUBSTRATE


def test_v1229_dim_version_string():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_DIM_VERSION
    assert V1229_DIM_VERSION == "0.6.39"


def test_v1229_module_version_string():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_VERSION
    assert V1229_VERSION == "0.1.0"


def test_v1229_six_pathways_cover_six_r_substrates():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    expected_rs = {
        "R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
        "R11_consciousness", "R12_ecology",
    }
    actual_rs = {p["r_substrate"] for p in V1229_CREATIVITY_SUBSTRATE.values()}
    assert actual_rs == expected_rs


def test_v1229_each_pathway_has_cascade_order():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    for pathway_name, pathway_data in V1229_CREATIVITY_SUBSTRATE.items():
        assert "cascade_order" in pathway_data, f"{pathway_name} missing cascade_order"
        assert "description" in pathway_data, f"{pathway_name} missing description"
        assert "source" in pathway_data, f"{pathway_name} missing source"


def test_v1229_creativity_all_molecules_have_organism():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import V1229_CREATIVITY_SUBSTRATE
    for pathway_data in V1229_CREATIVITY_SUBSTRATE.values():
        for m in pathway_data.get("molecules", []):
            assert m.get("organism") in ("human", "rat", "mouse", "monkey", "ape"), (
                f"{m.get('name')}: unknown organism {m.get('organism')}"
            )


def test_v1229_position_north_star_above_v1228():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    # V1228 was 75.66%, V1229 should be ~76.58% (higher)
    assert rep.position_of_north_star_realized_pct > 75.66


def test_v1229_lift_mean_positive():
    from apeireth.v1229_asi_v0639_creativity_substrate_real_lift import measure_v1229_full
    rep = measure_v1229_full()
    assert rep.v1229_overall_lift_delta_mean_from_v1228 > 0
