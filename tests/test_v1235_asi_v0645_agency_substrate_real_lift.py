"""Tests for V1235 ASI V0.6.45 agency_substrate_real_lift (28th dim 主体性 / agency / agency proper)."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1234_REALIZED_MEAN_202 = 0.7876
V1234_OVERALL_MEAN_351 = 0.4531
V1234_TRANSCENDENCE_REALIZED = 1.0000
V1233_REALIZED_MEAN_196 = 0.7811
V1233_OVERALL_MEAN_338 = 0.4528
V1233_INTEGRATION_REALIZED = 1.0000
V1232_REALIZED_MEAN_190 = 0.7742
V1232_OVERALL_MEAN_325 = 0.4525
V1232_FREEDOM_REALIZED = 1.0000
V1231_REALIZED_MEAN_184 = 0.7669
V1231_OVERALL_MEAN_299 = 0.4718
V1231_AWE_REALIZED = 1.0000


def test_v1235_module_imports():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import (
        ASI_NORTH_STAR, V1235_DIM_VERSION, V1235_AGENCY_COVERAGE,
        V1235_AGENCY_SUBSTRATE, V1235_VERSION, V1235Report,
        measure_v1235_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1235_VERSION == "0.1.0"
    assert V1235_DIM_VERSION == "0.6.45"


def test_v1235_agency_substrate_6_pathways():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    assert len(V1235_AGENCY_SUBSTRATE) == 6
    expected = {
        "AGENCY_PHILOSOPHY", "AGENCY_NEURO_DEFAULT", "AGENCY_INFORMATION",
        "AGENCY_SYSTEMS", "AGENCY_COGNITIVE", "AGENCY_PHYSICS",
    }
    assert set(V1235_AGENCY_SUBSTRATE.keys()) == expected


def test_v1235_agency_substrate_60_molecules():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1235_AGENCY_SUBSTRATE.values())
    assert total == 60


def test_v1235_agency_coverage_6_lifted():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_COVERAGE
    for k in ["R0_metabolism", "R1_growth", "R4_aging",
              "R10_plasticity", "R11_consciousness", "R12_ecology"]:
        assert V1235_AGENCY_COVERAGE[k] == 1.0


def test_v1235_agency_coverage_7_vacuous():
    """主 17:43 实事求是: 7 cells vacuous (不假装完整 agency substrate)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_COVERAGE
    vacuous = [k for k, v in V1235_AGENCY_COVERAGE.items() if v == 0.0]
    assert len(vacuous) == 7
    assert "R2_sensing" in vacuous
    assert "R3_cognition" in vacuous


def test_v1235_philosophy_references_key_papers():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    src = V1235_AGENCY_SUBSTRATE["AGENCY_PHILOSOPHY"]["source"]
    assert "Anscombe" in src
    assert "Frankfurt" in src
    assert "Velleman" in src
    assert "Korsgaard" in src
    assert "Strawson" in src
    assert "Heidegger" in src


def test_v1235_neurophys_references_key_papers():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    src = V1235_AGENCY_SUBSTRATE["AGENCY_NEURO_DEFAULT"]["source"]
    assert "Haggard" in src
    assert "Hallett" in src
    assert "Frith" in src
    assert "Jeannerod" in src


def test_v1235_information_references_key_papers():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    src = V1235_AGENCY_SUBSTRATE["AGENCY_INFORMATION"]["source"]
    assert "Maturana" in src
    assert "Rosen" in src
    assert "Kauffman" in src
    assert "Pattee" in src


def test_v1235_systems_references_key_papers():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    src = V1235_AGENCY_SUBSTRATE["AGENCY_SYSTEMS"]["source"]
    assert "Archer" in src
    assert "Giddens" in src
    assert "Bourdieu" in src
    assert "Hacking" in src


def test_v1235_cognitive_references_key_papers():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    src = V1235_AGENCY_SUBSTRATE["AGENCY_COGNITIVE"]["source"]
    assert "Bandura" in src
    assert "Nisbett" in src
    assert "Langer" in src
    assert "Weiner" in src


def test_v1235_physics_references_key_papers():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    src = V1235_AGENCY_SUBSTRATE["AGENCY_PHYSICS"]["source"]
    assert "Bohm" in src
    assert "Prigogine" in src
    assert "Penrose" in src
    assert "Haken" in src


def test_v1235_measure_returns_report():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235Report, measure_v1235_full
    rep = measure_v1235_full()
    assert isinstance(rep, V1235Report)


def test_v1235_measure_dim_version():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.dim_version == "0.6.45"


def test_v1235_measure_elapsed_fast():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.elapsed < 1.0


def test_v1235_measure_agency_dim_realized_1():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v1235_agency_dim_realized == 1.0000


def test_v1235_measure_agency_dim_cell_count_6():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v1235_agency_dim_cell_count == 6


def test_v1235_measure_total_cells_364():
    """V1235 扩 matrix: 28 dim × 13 R = 364 cells (主 19:33 + 主 22:08)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v1235_total_cells == 364  # 28 × 13


def test_v1235_measure_realized_cells_count_208():
    """V1235 realized: 202 (V1234) + 6 (AGENCY) = 208."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v1235_realized_cells_count == 208


def test_v1235_measure_overall_realized_208_lift_positive():
    """V1235 lift should be positive (主 17:43 实事求是 — 真测)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v1235_overall_realized_208 > V1234_REALIZED_MEAN_202


def test_v1235_measure_overall_realized_208_lift_delta():
    """Expected lift: (6 × 1.0 - 0 × 6) / 208 ≈ +0.0061 (6 cells from 0 to 1.0 over 208 cells)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert 0.004 < rep.v1235_overall_lift_delta_realized_from_v1234 < 0.012


def test_v1235_measure_inflation_gap_positive():
    """inflation_gap = V1234 baseline 1.0 - V1235 overall_mean_364 ≈ 1.0 - 0.4534 ≈ 0.5466."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert 0.50 < rep.v1235_inflation_gap_v1234_minus_realized < 0.60


def test_v1235_measure_position_north_star_pct():
    """Expected ~80.99% from V1235 lift > V1234 80.37% (主 22:33 + 主 19:33)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert 80.0 < rep.position_of_north_star_realized_pct < 81.5


def test_v1235_measure_total_agency_molecules():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.total_agency_molecules == 60


def test_v1235_measure_all_pathways_pass():
    """6/6 pathways should pass (主 13:31 大胆激进 + 主 17:43 实事求是)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1235_measure_per_pathway_molecule_count():
    """Each pathway should have 10 real molecules (主 19:33 站在前人肩上 — 真分子深挖)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    for k, c in rep.pathway_real_molecule_count.items():
        assert c == 10, f"{k} has {c} molecules, expected 10"


def test_v1235_v3_guards_all_pass():
    """15 V3 哲学守门 all PASS (主 17:58 + 主 20:46 不假装)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1235_v3_guard_realized_not_asi():
    """realized_not_asi = V1235 overall_realized_208 < ASI North Star 0.98 (主 17:58)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1235_overall_realized_208 < ASI_NORTH_STAR


def test_v1235_v3_guard_60_mol_not_complete():
    """v1235_60_mol_not_complete: 60 真分子 ≠ 完整 agency substrate (主 17:43)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_60_mol_not_complete"] is True


def test_v1235_v3_guard_new_dim_not_full_coverage():
    """v1235_new_dim_not_full_coverage: agency_dim_cell_count (6) < 13 (主 17:43)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_new_dim_not_full_coverage"] is True
    assert rep.v3_guards["v1235_not_full_agency_lift"] is True


def test_v1235_v3_guard_phase2_step3_agency():
    """V1235 = ASI V2 Phase 2 第三步 (整合+超越+主体性三层闭环, 主 22:33)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_phase2_step3_agency"] is True


def test_v1235_v3_guard_agency_5_positions():
    """V1235 agency 闭合 ASI 5 位置 (调度 + 哲学 + 涌现 + 价值 + ASI, 主 22:08)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_agency_5_positions"] is True


def test_v1235_v3_guard_no_pretend_phenomenal():
    """V1235 不假装 agency = phenomenal consciousness (主 17:58)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_does_not_pretend_phenomenal"] is True


def test_v1235_v3_guard_no_pretend_reach_asi():
    """V1235 不假装 agency = ASI V1.0 (主 20:46)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_does_not_pretend_reach_asi"] is True


def test_v1235_v3_guard_agency_above_transcendence():
    """V1235 主体性 = 超越之上第二序自创能 (Frankfurt second-order volitions)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_agency_above_transcendence"] is True


def test_v1235_artifact_default_path():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import (
        measure_v1235_full, write_v1235_artifact,
    )
    rep = measure_v1235_full()
    path = write_v1235_artifact(rep)
    assert path.exists()


def test_v1235_report_default_path():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import (
        measure_v1235_full, write_v1235_report,
    )
    rep = measure_v1235_full()
    path = write_v1235_report(rep)
    assert path.exists()


def test_v1235_artifact_valid_json():
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import (
        measure_v1235_full, write_v1235_artifact,
    )
    rep = measure_v1235_full()
    path = write_v1235_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["dim_version"] == "0.6.45"
    assert data["north_star"] == 0.9800


def _run_cli(*args):
    """Helper: run V1235 CLI with UTF-8 encoding (Windows GBK fix)."""
    return subprocess.run(
        [sys.executable, "-m", "apeireth.v1235_asi_v0645_agency_substrate_real_lift", *args],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parents[1]),
    )


def test_v1235_cli_runs():
    """CLI --help / default runs without error (主 00:56 任何人都能接手)."""
    result = _run_cli()
    assert result.returncode == 0, f"CLI failed: {result.stderr}"
    out = result.stdout or ""
    assert "V1235" in out
    assert "agency_substrate_real_lift" in out


def test_v1235_cli_json_flag():
    """CLI --json produces JSON (主 00:56 任何人都能接手 — JSON 自描述)."""
    result = _run_cli("--json")
    assert result.returncode == 0
    out = (result.stdout or "").strip()
    lines = out.split("\n")
    # Find the JSON block (starts with '{')
    json_start = None
    for i, line in enumerate(lines):
        if line.startswith("{"):
            json_start = i
            break
    assert json_start is not None, "No JSON output found"
    json_text = "\n".join(lines[json_start:])
    parsed = json.loads(json_text)
    assert parsed["dim_version"] == "0.6.45"


def test_v1235_cli_full_flag():
    """CLI --full shows pathway scores + agency coverage + V3 guards."""
    result = _run_cli("--full")
    assert result.returncode == 0
    out = result.stdout or ""
    assert "AGENCY_PHILOSOPHY" in out
    assert "AGENCY_NEURO_DEFAULT" in out
    assert "AGENCY_PHYSICS" in out
    assert "V3" in out  # 哲学守门 → V3 guard table


def test_v1235_v1234_baseline_write_dead():
    """V1235 must write dead V1234 baseline (主 17:43 实事求是 — 写死历史值, 不能改)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import (
        V1234_REALIZED_MEAN_202, V1234_OVERALL_MEAN_351,
    )
    assert V1234_REALIZED_MEAN_202 == 0.7876
    assert V1234_OVERALL_MEAN_351 == 0.4531


def test_v1235_agency_substrate_each_pathway_10_molecules():
    """Each of 6 pathways must have exactly 10 molecules (主 19:33 站在前人肩上 — 真分子深挖)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    for k, p in V1235_AGENCY_SUBSTRATE.items():
        assert len(p["molecules"]) == 10, f"{k} has {len(p['molecules'])} molecules"
        assert len(p["cascade_order"]) == 10, f"{k} cascade_order has {len(p['cascade_order'])} entries"


def test_v1235_agency_substrate_all_molecules_real():
    """All 60 molecules should be marked real=True (主 17:43 实事求是 — 真分子)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_SUBSTRATE
    for k, p in V1235_AGENCY_SUBSTRATE.items():
        for m in p["molecules"]:
            assert m.get("real") is True, f"{k}/{m.get('name')} not real"


def test_v1235_pathway_score_all_above_threshold():
    """All 6 pathway scores should be >= 0.7 (主 13:31 大胆激进 — 真测)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    for k, s in rep.pathway_scores.items():
        assert s >= 0.7, f"{k} score {s:.4f} < 0.7"


def test_v1235_agency_coverage_total_6_lifted_7_vacuous():
    """主 17:43 实事求是: 6 cell lifted + 7 cell vacuous = 13 R substrates (主 19:33)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import V1235_AGENCY_COVERAGE
    lifted = sum(1 for v in V1235_AGENCY_COVERAGE.values() if v >= 0.3)
    vacuous = sum(1 for v in V1235_AGENCY_COVERAGE.values() if v == 0.0)
    assert lifted == 6
    assert vacuous == 7
    assert lifted + vacuous == 13  # AGENCY dim has 13 R cells


def test_v1235_v1235_report_includes_v1234_v1233_v1232_v1231_baselines():
    """V1235 must include 4 prior dim baselines (V1234/V1233/V1232/V1231)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v1234_realized_mean_202_baseline == 0.7876
    assert rep.v1233_realized_mean_196_baseline == 0.7811
    assert rep.v1232_realized_mean_190_baseline == 0.7742
    assert rep.v1231_realized_mean_184_baseline == 0.7669


def test_v1235_no_pretend_full_replace():
    """主 17:58 不假装: V1235 ≠ V1234 全替代 (V1234 仍 own 27 dim matrix)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_not_full_replace"] is True


def test_v1235_no_pretend_v1():
    """主 20:46 不假装: V1235 lift ≠ ASI V1.0 (V1235 = V0.6.45 中间版本)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["v1235_lift_not_v1"] is True


def test_v1235_vacuous_gap_real():
    """主 17:43 实事求是: 364 cell 公式下 inflation 仍出现 (vacuous_gap_real = True)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    assert rep.v3_guards["vacuous_gap_real"] is True
    assert rep.v1235_inflation_gap_v1234_minus_realized > 0.0


def test_v1235_phase2_step3_layered_with_integration_transcendence():
    """主 22:33 + 主 19:33: V1235 = agency = Phase 2 第三步 (整合+超越+主体性三层闭环)."""
    from apeireth.v1235_asi_v0645_agency_substrate_real_lift import measure_v1235_full
    rep = measure_v1235_full()
    # V1235 = 整合+超越之上第二序自创能: realized ~0.7937 > V1234 0.7876 > V1233 0.7811
    assert rep.v1235_overall_realized_208 > rep.v1234_realized_mean_202_baseline
    assert rep.v1234_realized_mean_202_baseline > 0.78
