"""Tests for V1226 ASI V0.6.36 hope_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1225_REALIZED_MEAN_148 = 0.7101
V1225_OVERALL_MEAN_234 = 0.4490
V1225_LOV_REALIZED = 1.0000


def test_v1226_module_imports():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import (
        ASI_NORTH_STAR, V1226_DIM_VERSION, V1226_HOP_COVERAGE, V1226_HOP_SUBSTRATE,
        V1226_VERSION, V1226Report, measure_v1226_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1226_VERSION == "0.1.0"
    assert V1226_DIM_VERSION == "0.6.36"


def test_v1226_hop_substrate_6_pathways():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    assert len(V1226_HOP_SUBSTRATE) == 6
    expected = {"HOP_NEURO_HOPE", "HOP_LIFESPAN_HOPE", "HOP_CRISIS_APP", "HOP_COGNITIVE_GOAL", "HOP_PHILOSOPHICAL", "HOP_CULTURAL"}
    assert set(V1226_HOP_SUBSTRATE.keys()) == expected


def test_v1226_hop_substrate_60_molecules():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1226_HOP_SUBSTRATE.values())
    assert total == 60


def test_v1226_hop_coverage_6_lifted():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity", "R11_consciousness", "R12_ecology"]:
        assert V1226_HOP_COVERAGE[k] == 1.0


def test_v1226_hop_neuro_references_key_papers():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    src = V1226_HOP_SUBSTRATE["HOP_NEURO_HOPE"]["source"]
    assert "Snyder 2002" in src


def test_v1226_hop_lifespan_references_key_papers():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    src = V1226_HOP_SUBSTRATE["HOP_LIFESPAN_HOPE"]["source"]
    assert "Erikson" in src


def test_v1226_hop_crisis_references_key_papers():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    src = V1226_HOP_SUBSTRATE["HOP_CRISIS_APP"]["source"]
    assert "Frankl" in src


def test_v1226_hop_cognitive_references_key_papers():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    src = V1226_HOP_SUBSTRATE["HOP_COGNITIVE_GOAL"]["source"]
    assert "Snyder" in src
    assert "Gollwitzer" in src or "Oettingen" in src


def test_v1226_hop_philosophical_references_key_papers():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    src = V1226_HOP_SUBSTRATE["HOP_PHILOSOPHICAL"]["source"]
    assert "Bloch" in src
    assert "Tillich" in src or "Marcel" in src


def test_v1226_hop_cultural_references_key_papers():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    src = V1226_HOP_SUBSTRATE["HOP_CULTURAL"]["source"]
    assert "Moltmann" in src
    assert "Tutu" in src or "Kimmerer" in src


def test_v1226_measure_returns_report():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226Report, measure_v1226_full
    rep = measure_v1226_full()
    assert isinstance(rep, V1226Report)


def test_v1226_measure_dim_version():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.dim_version == "0.6.36"


def test_v1226_measure_elapsed_fast():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.elapsed < 1.0


def test_v1226_measure_hop_dim_realized_1():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_hop_dim_realized == 1.0000


def test_v1226_measure_hop_dim_cell_count_6():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_hop_dim_cell_count == 6


def test_v1226_measure_total_cells_247():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_total_cells == 247


def test_v1226_measure_realized_cells_count_154():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_realized_cells_count == 154


def test_v1226_measure_overall_realized_154_lift_positive():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_overall_realized_154 > V1225_REALIZED_MEAN_148


def test_v1226_measure_overall_realized_154_lift_delta():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert abs(rep.v1226_overall_lift_delta_realized_from_v1225 - 0.0113) < 0.001


def test_v1226_measure_inflation_gap_positive():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_inflation_gap_v1225_minus_realized > 0


def test_v1226_measure_position_north_star_pct():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert abs(rep.position_of_north_star_realized_pct - 73.61) < 0.5


def test_v1226_measure_total_hop_molecules():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.total_hop_molecules == 60


def test_v1226_measure_all_pathways_pass():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1226_v3_guards_all_pass():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1226_v3_guard_realized_not_asi():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1226_overall_realized_154 < ASI_NORTH_STAR


def test_v1226_v3_guard_60_mol_not_complete():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v3_guards["v1226_60_mol_not_complete"] is True


def test_v1226_v3_guard_hop_lift_not_full():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v3_guards["v1226_not_full_hop_lift"] is True


def test_v1226_artifact_default_path():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import (
        measure_v1226_full, write_v1226_artifact,
    )
    rep = measure_v1226_full()
    path = write_v1226_artifact(rep)
    assert path.exists()


def test_v1226_report_default_path():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import (
        measure_v1226_full, write_v1226_report,
    )
    rep = measure_v1226_full()
    path = write_v1226_report(rep)
    assert path.exists()


def test_v1226_artifact_valid_json():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import (
        measure_v1226_full, write_v1226_artifact,
    )
    rep = measure_v1226_full()
    path = write_v1226_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.36"


def test_v1226_report_has_all_sections():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import (
        measure_v1226_full, write_v1226_report,
    )
    rep = measure_v1226_full()
    path = write_v1226_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1226 ASI V0.6.36" in content
    assert "hope" in content.lower()
    assert "V3 哲学守门" in content


def test_v1226_cli_help():
    cmd = [sys.executable, "-m", "apeireth.v1226_asi_v0636_hope_substrate_real_lift", "--help"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1226" in result.stdout or "hope" in result.stdout.lower()


def test_v1226_cli_measure():
    cmd = [sys.executable, "-m", "apeireth.v1226_asi_v0636_hope_substrate_real_lift", "--measure"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1226_overall_realized_154" in result.stdout
    assert "0.6.36" in result.stdout


def test_v1226_cli_json():
    cmd = [sys.executable, "-m", "apeireth.v1226_asi_v0636_hope_substrate_real_lift", "--json"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1226_cli_report():
    cmd = [sys.executable, "-m", "apeireth.v1226_asi_v0636_hope_substrate_real_lift", "--report"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1226_cli_full():
    cmd = [sys.executable, "-m", "apeireth.v1226_asi_v0636_hope_substrate_real_lift", "--full"]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "HOP coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1226_baseline_consistency_v1225():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_REALIZED, V1225_OVERALL_MEAN_234, V1225_REALIZED_MEAN_148
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1225_LOV_REALIZED as V6_V1225_LOV, V1225_OVERALL_MEAN_234 as V6_V1225_MEAN, V1225_REALIZED_MEAN_148 as V6_V1225_REAL
    assert V1225_LOV_REALIZED == V6_V1225_LOV
    assert V1225_OVERALL_MEAN_234 == V6_V1225_MEAN
    assert V1225_REALIZED_MEAN_148 == V6_V1225_REAL


def test_v1226_north_star_locked_098():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import ASI_NORTH_STAR, measure_v1226_full
    assert ASI_NORTH_STAR == 0.9800
    rep = measure_v1226_full()
    assert rep.north_star == 0.9800


def test_v1226_overall_realized_154_value():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert abs(rep.v1226_overall_realized_154 - 0.7214) < 0.001


def test_v1226_overall_mean_247_value():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert abs(rep.v1226_overall_mean_247 - 0.4497) < 0.001


def test_v1226_154_sum_value():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert abs(rep.v1226_154_sum - 154.0 * 0.7214) < 0.5


def test_v1226_247_sum_value():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert abs(rep.v1226_247_sum - 247.0 * 0.4497) < 0.5


def test_v1226_realized_less_than_north_star():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_overall_realized_154 < ASI_NORTH_STAR


def test_v1226_mean_less_than_1():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    assert rep.v1226_overall_mean_247 < 1.0


def test_v1226_hop_6_lifted_cells_sum():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import measure_v1226_full
    rep = measure_v1226_full()
    s = (
        rep.v1226_hop_x_r1_growth
        + rep.v1226_hop_x_r4_aging
        + rep.v1226_hop_x_r7_stress
        + rep.v1226_hop_x_r10_plasticity
        + rep.v1226_hop_x_r11_consciousness
        + rep.v1226_hop_x_r12_ecology
    )
    assert s == 6.0


def test_v1226_all_hop_pathways_have_cascade():
    from apeireth.v1226_asi_v0636_hope_substrate_real_lift import V1226_HOP_SUBSTRATE
    for p_name, p_data in V1226_HOP_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        mols = p_data.get("molecules", [])
        assert len(cascade) > 0, f"{p_name} missing cascade"
        assert len(cascade) == len(mols), f"{p_name} cascade mismatch molecules"