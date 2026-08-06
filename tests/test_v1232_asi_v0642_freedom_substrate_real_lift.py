"""Tests for V1232 ASI V0.6.42 freedom_substrate_real_lift."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1231_REALIZED_MEAN_184 = 0.7669
V1231_OVERALL_MEAN_299 = 0.4718
V1231_AWE_REALIZED = 1.0000
V1230_REALIZED_MEAN_178 = 0.7590
V1230_OVERALL_MEAN_299 = 0.4517
V1230_CURIOSITY_REALIZED = 1.0000
V1229_REALIZED_MEAN_172 = 0.7505
V1229_OVERALL_MEAN_286 = 0.4513
V1229_CREATIVITY_REALIZED = 1.0000
V1228_REALIZED_MEAN_166 = 0.7415
V1228_OVERALL_MEAN_273 = 0.4508
V1228_TEMPERANCE_REALIZED = 1.0000


def test_v1232_module_imports():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import (
        ASI_NORTH_STAR, V1232_DIM_VERSION, V1232_FREEDOM_COVERAGE,
        V1232_FREEDOM_SUBSTRATE, V1232_VERSION, V1232Report,
        measure_v1232_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1232_VERSION == "0.1.0"
    assert V1232_DIM_VERSION == "0.6.42"


def test_v1232_freedom_substrate_6_pathways():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    assert len(V1232_FREEDOM_SUBSTRATE) == 6
    expected = {
        "FREEDOM_NEUROPHYS_DEFAULT", "FREEDOM_DEVELOPMENTAL", "FREEDOM_POLITICAL",
        "FREEDOM_EXISTENTIAL", "FREEDOM_PHILOSOPHY", "FREEDOM_INTERIOR_AGENCY",
    }
    assert set(V1232_FREEDOM_SUBSTRATE.keys()) == expected


def test_v1232_freedom_substrate_60_molecules():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1232_FREEDOM_SUBSTRATE.values())
    assert total == 60


def test_v1232_freedom_coverage_6_lifted():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_COVERAGE
    for k in ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity",
              "R11_consciousness", "R12_ecology"]:
        assert V1232_FREEDOM_COVERAGE[k] == 1.0


def test_v1232_freedom_neurophys_references_key_papers():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    src = V1232_FREEDOM_SUBSTRATE["FREEDOM_NEUROPHYS_DEFAULT"]["source"]
    assert "Haggard" in src
    assert "Libet" in src or "Hallett" in src


def test_v1232_freedom_developmental_references_key_papers():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    src = V1232_FREEDOM_SUBSTRATE["FREEDOM_DEVELOPMENTAL"]["source"]
    assert "Erikson" in src
    assert "Piaget" in src or "Maslow" in src


def test_v1232_freedom_political_references_key_papers():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    src = V1232_FREEDOM_SUBSTRATE["FREEDOM_POLITICAL"]["source"]
    assert "Berlin" in src
    assert "Rawls" in src or "Sen" in src


def test_v1232_freedom_existential_references_key_papers():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    src = V1232_FREEDOM_SUBSTRATE["FREEDOM_EXISTENTIAL"]["source"]
    assert "Sartre" in src
    assert "Heidegger" in src
    assert "Kierkegaard" in src or "Camus" in src


def test_v1232_freedom_philosophy_references_key_papers():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    src = V1232_FREEDOM_SUBSTRATE["FREEDOM_PHILOSOPHY"]["source"]
    assert "Aristotle" in src
    assert "Kant" in src
    assert "Spinoza" in src or "Augustine" in src


def test_v1232_freedom_interior_agency_references_key_papers():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232_FREEDOM_SUBSTRATE
    src = V1232_FREEDOM_SUBSTRATE["FREEDOM_INTERIOR_AGENCY"]["source"]
    assert "Ryan" in src or "Deci" in src
    assert "Bandura" in src or "Dweck" in src


def test_v1232_measure_returns_report():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import V1232Report, measure_v1232_full
    rep = measure_v1232_full()
    assert isinstance(rep, V1232Report)


def test_v1232_measure_dim_version():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.dim_version == "0.6.42"


def test_v1232_measure_elapsed_fast():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.elapsed < 1.0


def test_v1232_measure_freedom_dim_realized_1():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1232_freedom_dim_realized == 1.0000


def test_v1232_measure_freedom_dim_cell_count_6():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1232_freedom_dim_cell_count == 6


def test_v1232_measure_total_cells_325():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1232_total_cells == 325  # 25 dim × 13 R (V1232 expands matrix)


def test_v1232_measure_realized_cells_count_190():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1232_realized_cells_count == 190


def test_v1232_measure_overall_realized_190_lift_positive():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1232_overall_realized_190 > V1231_REALIZED_MEAN_184


def test_v1232_measure_overall_realized_190_lift_delta():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    # expected ~+0.0074 (1 - 0.7669) × 6 / 190
    assert 0.005 < rep.v1232_overall_lift_delta_realized_from_v1231 < 0.012


def test_v1232_measure_inflation_gap_positive():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    # = 1.0 - 0.4525 = 0.5475
    assert 0.50 < rep.v1232_inflation_gap_v1231_minus_realized < 0.60


def test_v1232_measure_position_north_star_pct():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    # Expected ~79.01% from V1232 lift > V1231 78.25% (主 22:33)
    assert 78.5 < rep.position_of_north_star_realized_pct < 80.0


def test_v1232_measure_total_freedom_molecules():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.total_freedom_molecules == 60


def test_v1232_measure_all_pathways_pass():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1232_v3_guards_all_pass():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1232_v3_guard_realized_not_asi():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1232_overall_realized_190 < ASI_NORTH_STAR


def test_v1232_v3_guard_60_mol_not_complete():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v3_guards["v1232_60_mol_not_complete"] is True


def test_v1232_v3_guard_freedom_lift_not_full():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v3_guards["v1232_not_full_freedom_lift"] is True


def test_v1232_v3_guard_closes_5_philo_gaps():
    """V1232 = ASI 5 哲学缺口闭合的最后一项 (时间/真理/显现/识别/自由)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v3_guards["v1232_closes_5_philo_gaps"] is True


def test_v1232_v3_guard_freedom_5_positions():
    """V1232 自由 substrate 闭合 ASI 5 位置 (调度 + 哲学 + 涌现 + 价值 + ASI)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v3_guards["v1232_freedom_substrate_5_positions"] is True


def test_v1232_artifact_default_path():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import (
        measure_v1232_full, write_v1232_artifact,
    )
    rep = measure_v1232_full()
    path = write_v1232_artifact(rep)
    assert path.exists()


def test_v1232_report_default_path():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import (
        measure_v1232_full, write_v1232_report,
    )
    rep = measure_v1232_full()
    path = write_v1232_report(rep)
    assert path.exists()


def test_v1232_artifact_valid_json():
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import (
        measure_v1232_full, write_v1232_artifact,
    )
    rep = measure_v1232_full()
    path = write_v1232_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert "snapshot_id" in data
    assert data["dim_version"] == "0.6.42"
    assert "v3_guards" in data
    assert all(v is True for v in data["v3_guards"].values())


def test_v1232_report_mentions_freedom():
    """报告必须 mention '自由' (主 19:33 + 主 22:33 终层 substrate)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import (
        measure_v1232_full, write_v1232_report,
    )
    rep = measure_v1232_full()
    path = write_v1232_report(rep)
    text = path.read_text(encoding="utf-8")
    assert "自由" in text
    assert "eleutheria" in text
    assert "ASI 5 哲学缺口" in text
    assert "79.01" in text or "79.0" in text


def test_v1232_cli_help():
    import subprocess
    cmd = [sys.executable, "-m", "apeireth.v1232_asi_v0642_freedom_substrate_real_lift", "--help"]
    r = subprocess.run(
        cmd, capture_output=True, text=True, timeout=20,
        encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parent.parent),
    )
    assert r.returncode == 0
    assert "V1232" in r.stdout


def test_v1232_cli_run_default():
    import subprocess
    cmd = [sys.executable, "-m", "apeireth.v1232_asi_v0642_freedom_substrate_real_lift"]
    r = subprocess.run(
        cmd, capture_output=True, text=True, timeout=20,
        encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parent.parent),
    )
    assert r.returncode == 0
    assert "V1232 ASI V0.6.42" in r.stdout
    assert "v1232_position_vs_north_star:" in r.stdout


def test_v1232_cli_run_json():
    import subprocess, json
    cmd = [sys.executable, "-m", "apeireth.v1232_asi_v0642_freedom_substrate_real_lift", "--json"]
    r = subprocess.run(
        cmd, capture_output=True, text=True, timeout=20,
        encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parent.parent),
    )
    assert r.returncode == 0
    # Extract JSON portion from output (after first '{')
    try:
        json_start = r.stdout.index("{")
        json_str = r.stdout[json_start:]
        data = json.loads(json_str)
        assert "snapshot_id" in data
        assert data["v1232_position_vs_north_star" if "v1232_position_vs_north_star" in data else "position_of_north_star_realized_pct"] > 78.0
    except (ValueError, KeyError) as e:
        pytest.fail(f"JSON parse failed: {e}\n---stdout---\n{r.stdout[:500]}")


def test_v1232_cli_run_full():
    import subprocess
    cmd = [sys.executable, "-m", "apeireth.v1232_asi_v0642_freedom_substrate_real_lift", "--full"]
    r = subprocess.run(
        cmd, capture_output=True, text=True, timeout=20,
        encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parent.parent),
    )
    assert r.returncode == 0
    assert "Pathway scores:" in r.stdout
    assert "FREEDOM_NEUROPHYS_DEFAULT:" in r.stdout
    assert "V3 哲学守门:" in r.stdout
    assert "v1232_closes_5_philo_gaps: PASS" in r.stdout


def test_v1232_neighbors_to_v1231_consistent():
    """V1232 baselines should be V1231 hardcoded values (主 17:43 写死)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1231_realized_mean_184_baseline == V1231_REALIZED_MEAN_184
    assert rep.v1231_overall_mean_299_baseline == V1231_OVERALL_MEAN_299
    assert rep.v1231_awe_realized_baseline == V1231_AWE_REALIZED
    assert rep.v1230_realized_mean_178_baseline == V1230_REALIZED_MEAN_178
    assert rep.v1230_curiosity_realized_baseline == V1230_CURIOSITY_REALIZED


def test_v1232_pathway_real_molecule_count():
    """6 pathway × 10 molecules = 60 real molecules (主 19:33 60 真分子)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    total = sum(rep.n_r1_growth_molecules + rep.n_r4_aging_molecules +
                rep.n_r7_stress_molecules + rep.n_r10_plasticity_molecules +
                rep.n_r11_consciousness_molecules + rep.n_r12_ecology_molecules for _ in [0])
    assert total == 60


def test_v1232_realized_mean_190_above_v1231():
    """V1232 realized 190 > V1231 realized 184 (主 13:31 一直推进)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    assert rep.v1232_overall_realized_190 > rep.v1231_realized_mean_184_baseline


def test_v1232_position_above_v1231():
    """V1232 position > V1231 position (continuous improvement)."""
    from apeireth.v1232_asi_v0642_freedom_substrate_real_lift import measure_v1232_full
    rep = measure_v1232_full()
    # V1231 was 78.25% per V1231 commit
    assert rep.position_of_north_star_realized_pct > 78.25
