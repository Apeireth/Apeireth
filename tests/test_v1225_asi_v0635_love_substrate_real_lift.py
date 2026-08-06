"""Tests for V1225 ASI V0.6.35 love_substrate_real_lift.

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

APEIRETH_ROOT = Path(__file__).resolve().parents[1] / "apeireth"
WORKSPACE_ROOT = Path(__file__).resolve().parents[1]


def _safe_div(a: float, b: float, default: float = 0.0) -> float:
    if b == 0.0:
        return default
    return a / b


# Static baselines (主 17:43 实事求是 — 写死)
ASI_NORTH_STAR = 0.9800

V1224_REALIZED_MEAN_142 = 0.6979
V1224_OVERALL_MEAN_221 = 0.4483
V1224_WIS_REALIZED = 1.0000

V1223_REALIZED_MEAN_136 = 0.6846
V1223_OVERALL_MEAN_208 = 0.4475
V1223_ME_REALIZED = 1.0000


def test_v1225_module_imports():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import (
        ASI_NORTH_STAR,
        V1225_DIM_VERSION,
        V1225_LOV_COVERAGE,
        V1225_LOV_SUBSTRATE,
        V1225_VERSION,
        V1225Report,
        measure_v1225_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1225_VERSION == "0.1.0"
    assert V1225_DIM_VERSION == "0.6.35"
    assert isinstance(V1225_LOV_SUBSTRATE, dict)
    assert isinstance(V1225_LOV_COVERAGE, dict)
    assert isinstance(V1225Report, type)


def test_v1225_lov_substrate_6_pathways():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    assert len(V1225_LOV_SUBSTRATE) == 6
    expected = {
        "LOV_NEURO_ATTACHMENT",
        "LOV_LIFESPAN_ATTACHMENT",
        "LOV_CARE_STRESS",
        "LOV_COGNITIVE_MENTALIZING",
        "LOV_PHILOSOPHICAL",
        "LOV_CULTURAL",
    }
    assert set(V1225_LOV_SUBSTRATE.keys()) == expected


def test_v1225_lov_substrate_60_molecules():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1225_LOV_SUBSTRATE.values())
    assert total == 60


def test_v1225_lov_coverage_6_lifted():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_COVERAGE
    assert V1225_LOV_COVERAGE["R1_growth"] == 1.0
    assert V1225_LOV_COVERAGE["R4_aging"] == 1.0
    assert V1225_LOV_COVERAGE["R7_stress"] == 1.0
    assert V1225_LOV_COVERAGE["R10_plasticity"] == 1.0
    assert V1225_LOV_COVERAGE["R11_consciousness"] == 1.0
    assert V1225_LOV_COVERAGE["R12_ecology"] == 1.0


def test_v1225_lov_neuro_attachment_references_key_papers():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    src = V1225_LOV_SUBSTRATE["LOV_NEURO_ATTACHMENT"]["source"]
    assert "Bowlby 1969" in src
    assert "Ainsworth 1978" in src
    assert "Panksepp 1998" in src


def test_v1225_lov_lifespan_attachment_references_key_papers():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    src = V1225_LOV_SUBSTRATE["LOV_LIFESPAN_ATTACHMENT"]["source"]
    assert "Hazan Shaver 1987" in src
    assert "Bartholomew 1990" in src
    assert "Fraley 2000" in src


def test_v1225_lov_care_stress_references_key_papers():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    src = V1225_LOV_SUBSTRATE["LOV_CARE_STRESS"]["source"]
    assert "Neff 2003" in src
    assert "Batson 1991" in src


def test_v1225_lov_cognitive_mentalizing_references_key_papers():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    src = V1225_LOV_SUBSTRATE["LOV_COGNITIVE_MENTALIZING"]["source"]
    assert "Fonagy" in src
    assert "Meins" in src or "Premack" in src


def test_v1225_lov_philosophical_references_key_papers():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    src = V1225_LOV_SUBSTRATE["LOV_PHILOSOPHICAL"]["source"]
    assert "Plato Symposium" in src
    assert "Aristotle" in src
    assert "Confucius" in src
    assert "Fromm" in src or "Levinas" in src


def test_v1225_lov_cultural_references_key_papers():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    src = V1225_LOV_SUBSTRATE["LOV_CULTURAL"]["source"]
    assert "Tutu" in src or "Ubuntu" in src
    assert "Kimmerer" in src or "Mignolo" in src


def test_v1225_measure_returns_report():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import (
        V1225Report,
        measure_v1225_full,
    )
    rep = measure_v1225_full()
    assert isinstance(rep, V1225Report)


def test_v1225_measure_snapshot_id_nonempty():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert len(rep.snapshot_id) == 36


def test_v1225_measure_dim_version_correct():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.dim_version == "0.6.35"


def test_v1225_measure_elapsed_fast():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.elapsed < 1.0


def test_v1225_measure_lov_dim_realized_1():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_lov_dim_realized == 1.0000


def test_v1225_measure_lov_dim_cell_count_6():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_lov_dim_cell_count == 6


def test_v1225_measure_total_cells_234():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_total_cells == 234


def test_v1225_measure_realized_cells_count_148():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_realized_cells_count == 148


def test_v1225_measure_overall_realized_148_lift_positive():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_overall_realized_148 > V1224_REALIZED_MEAN_142


def test_v1225_measure_overall_realized_148_lift_delta():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert abs(rep.v1225_overall_lift_delta_realized_from_v1224 - 0.0122) < 0.001


def test_v1225_measure_inflation_gap_positive():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_inflation_gap_v1224_minus_realized > 0


def test_v1225_measure_position_north_star_pct():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert abs(rep.position_of_north_star_realized_pct - 72.46) < 0.5


def test_v1225_measure_total_lov_molecules():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.total_lov_molecules == 60


def test_v1225_measure_all_pathways_pass():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1225_v3_guards_all_pass():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1225_v3_guard_realized_not_asi():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1225_overall_realized_148 < ASI_NORTH_STAR


def test_v1225_v3_guard_60_mol_not_complete():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v3_guards["v1225_60_mol_not_complete"] is True


def test_v1225_v3_guard_lov_lift_not_full():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v3_guards["v1225_not_full_lov_lift"] is True


def test_v1225_artifact_default_path():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import (
        measure_v1225_full,
        write_v1225_artifact,
    )
    rep = measure_v1225_full()
    path = write_v1225_artifact(rep)
    assert path.exists()


def test_v1225_report_default_path():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import (
        measure_v1225_full,
        write_v1225_report,
    )
    rep = measure_v1225_full()
    path = write_v1225_report(rep)
    assert path.exists()


def test_v1225_artifact_valid_json():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import (
        measure_v1225_full,
        write_v1225_artifact,
    )
    rep = measure_v1225_full()
    path = write_v1225_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["snapshot_id"] == rep.snapshot_id
    assert data["dim_version"] == "0.6.35"


def test_v1225_report_has_all_sections():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import (
        measure_v1225_full,
        write_v1225_report,
    )
    rep = measure_v1225_full()
    path = write_v1225_report(rep)
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    assert "V1225 ASI V0.6.35" in content
    assert "love" in content.lower()
    assert "North Star" in content
    assert "V3 哲学守门" in content


def test_v1225_cli_help():
    cmd = [
        sys.executable, "-m", "apeireth.v1225_asi_v0635_love_substrate_real_lift", "--help"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "V1225" in result.stdout or "love" in result.stdout.lower()


def test_v1225_cli_measure():
    cmd = [
        sys.executable, "-m", "apeireth.v1225_asi_v0635_love_substrate_real_lift", "--measure"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "v1225_overall_realized_148" in result.stdout
    assert "0.6.35" in result.stdout


def test_v1225_cli_json():
    cmd = [
        sys.executable, "-m", "apeireth.v1225_asi_v0635_love_substrate_real_lift", "--json"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0


def test_v1225_cli_report():
    cmd = [
        sys.executable, "-m", "apeireth.v1225_asi_v0635_love_substrate_real_lift", "--report"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "report:" in result.stdout


def test_v1225_cli_full():
    cmd = [
        sys.executable, "-m", "apeireth.v1225_asi_v0635_love_substrate_real_lift", "--full"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=20, encoding="utf-8", errors="replace")
    assert result.returncode == 0
    assert "Pathway scores" in result.stdout
    assert "LOV coverage" in result.stdout
    assert "V3 哲学守门" in result.stdout


def test_v1225_baseline_consistency_v1224():
    from apeireth.v1224_asi_v0634_wisdom_substrate_real_lift import V1224_WIS_REALIZED, V1224_OVERALL_MEAN_221, V1224_REALIZED_MEAN_142
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1224_WIS_REALIZED as V5_V1224_WIS, V1224_OVERALL_MEAN_221 as V5_V1224_MEAN, V1224_REALIZED_MEAN_142 as V5_V1224_REAL
    assert V1224_WIS_REALIZED == V5_V1224_WIS
    assert V1224_OVERALL_MEAN_221 == V5_V1224_MEAN
    assert V1224_REALIZED_MEAN_142 == V5_V1224_REAL


def test_v1225_north_star_locked_098():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import ASI_NORTH_STAR, measure_v1225_full
    assert ASI_NORTH_STAR == 0.9800
    rep = measure_v1225_full()
    assert rep.north_star == 0.9800


def test_v1225_overall_realized_148_value():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert abs(rep.v1225_overall_realized_148 - 0.7101) < 0.001


def test_v1225_overall_mean_234_value():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert abs(rep.v1225_overall_mean_234 - 0.4490) < 0.001


def test_v1225_148_sum_value():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert abs(rep.v1225_148_sum - 148.0 * 0.7101) < 0.5


def test_v1225_234_sum_value():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert abs(rep.v1225_234_sum - 234.0 * 0.4490) < 0.5


def test_v1225_realized_less_than_north_star():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_overall_realized_148 < ASI_NORTH_STAR


def test_v1225_mean_less_than_1():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    assert rep.v1225_overall_mean_234 < 1.0


def test_v1225_lov_6_lifted_cells_sum():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full
    rep = measure_v1225_full()
    lov_row_sum = (
        rep.v1225_lov_x_r1_growth
        + rep.v1225_lov_x_r4_aging
        + rep.v1225_lov_x_r7_stress
        + rep.v1225_lov_x_r10_plasticity
        + rep.v1225_lov_x_r11_consciousness
        + rep.v1225_lov_x_r12_ecology
    )
    assert lov_row_sum == 6.0


def test_v1225_lov_vacuous_7_cells_sum():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import measure_v1225_full, V1225_LOV_COVERAGE
    rep = measure_v1225_full()
    vacuous_sum = (
        V1225_LOV_COVERAGE["R2_sensing"]
        + V1225_LOV_COVERAGE["R3_cognition"]
        + V1225_LOV_COVERAGE["R5_social"]
        + V1225_LOV_COVERAGE["R6_communication"]
        + V1225_LOV_COVERAGE["R8_motion"]
        + V1225_LOV_COVERAGE["R9_heredity"]
    )
    assert vacuous_sum == 0.0


def test_v1225_all_lov_pathways_have_cascade():
    from apeireth.v1225_asi_v0635_love_substrate_real_lift import V1225_LOV_SUBSTRATE
    for p_name, p_data in V1225_LOV_SUBSTRATE.items():
        cascade = p_data.get("cascade_order", [])
        mols = p_data.get("molecules", [])
        assert len(cascade) > 0, f"{p_name} missing cascade"
        assert len(cascade) == len(mols), f"{p_name} cascade mismatch molecules"
        mol_names = [m["name"] for m in mols]
        for c in cascade:
            assert c in mol_names, f"{p_name}: cascade entry {c} not in molecules"