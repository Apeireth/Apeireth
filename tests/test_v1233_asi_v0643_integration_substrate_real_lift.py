"""Tests for V1233 ASI V0.6.43 integration_substrate_real_lift (26th dim 整体性 / integration / holism / synthesis / coherence / unity / synergy substrate)."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1232_REALIZED_MEAN_190 = 0.7742
V1232_OVERALL_MEAN_325 = 0.4525
V1232_FREEDOM_REALIZED = 1.0000
V1231_REALIZED_MEAN_184 = 0.7669
V1231_OVERALL_MEAN_299 = 0.4718
V1231_AWE_REALIZED = 1.0000
V1230_REALIZED_MEAN_178 = 0.7590
V1230_OVERALL_MEAN_299 = 0.4517
V1230_CURIOSITY_REALIZED = 1.0000
V1229_REALIZED_MEAN_172 = 0.7505
V1229_OVERALL_MEAN_286 = 0.4512
V1229_CREATIVITY_REALIZED = 1.0000


def test_v1233_module_imports():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import (
        ASI_NORTH_STAR, V1233_DIM_VERSION, V1233_INTEGRATION_COVERAGE,
        V1233_INTEGRATION_SUBSTRATE, V1233_VERSION, V1233Report,
        measure_v1233_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1233_VERSION == "0.1.0"
    assert V1233_DIM_VERSION == "0.6.43"


def test_v1233_integration_substrate_6_pathways():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    assert len(V1233_INTEGRATION_SUBSTRATE) == 6
    expected = {
        "INTEGRATION_PHILOSOPHY", "INTEGRATION_NEURO_DEFAULT", "INTEGRATION_INFORMATION",
        "INTEGRATION_SYSTEMS", "INTEGRATION_COGNITIVE", "INTEGRATION_PHYSICS",
    }
    assert set(V1233_INTEGRATION_SUBSTRATE.keys()) == expected


def test_v1233_integration_substrate_60_molecules():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1233_INTEGRATION_SUBSTRATE.values())
    assert total == 60


def test_v1233_integration_coverage_6_lifted():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_COVERAGE
    for k in ["R0_metabolism", "R1_growth", "R4_aging",
              "R10_plasticity", "R11_consciousness", "R12_ecology"]:
        assert V1233_INTEGRATION_COVERAGE[k] == 1.0


def test_v1233_integration_coverage_7_vacuous():
    """主 17:43 实事求是: 7 cells vacuous (不假装完整 integration substrate)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_COVERAGE
    vacuous = [k for k, v in V1233_INTEGRATION_COVERAGE.items() if v == 0.0]
    assert len(vacuous) == 7
    assert "R2_sensing" in vacuous
    assert "R3_cognition" in vacuous


def test_v1233_philosophy_references_key_papers():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    src = V1233_INTEGRATION_SUBSTRATE["INTEGRATION_PHILOSOPHY"]["source"]
    assert "Aristotle" in src
    assert "Kant" in src
    assert "Tononi" in src
    assert "Bertalanffy" in src


def test_v1233_neurophys_references_key_papers():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    src = V1233_INTEGRATION_SUBSTRATE["INTEGRATION_NEURO_DEFAULT"]["source"]
    assert "Llinas" in src
    assert "Baars" in src
    assert "Dehaene" in src
    assert "Freeman" in src


def test_v1233_information_references_key_papers():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    src = V1233_INTEGRATION_SUBSTRATE["INTEGRATION_INFORMATION"]["source"]
    assert "Shannon" in src
    assert "Tononi" in src
    assert "Mediano" in src


def test_v1233_systems_references_key_papers():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    src = V1233_INTEGRATION_SUBSTRATE["INTEGRATION_SYSTEMS"]["source"]
    assert "Bertalanffy" in src
    assert "Holling" in src
    assert "Folke" in src


def test_v1233_cognitive_references_key_papers():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    src = V1233_INTEGRATION_SUBSTRATE["INTEGRATION_COGNITIVE"]["source"]
    assert "Piaget" in src
    assert "Vygotsky" in src
    assert "Varela" in src


def test_v1233_physics_references_key_papers():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    src = V1233_INTEGRATION_SUBSTRATE["INTEGRATION_PHYSICS"]["source"]
    assert "Prigogine" in src
    assert "Anderson" in src
    assert "Haken" in src


def test_v1233_measure_returns_report():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233Report, measure_v1233_full
    rep = measure_v1233_full()
    assert isinstance(rep, V1233Report)


def test_v1233_measure_dim_version():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.dim_version == "0.6.43"


def test_v1233_measure_elapsed_fast():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.elapsed < 1.0


def test_v1233_measure_integration_dim_realized_1():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v1233_integration_dim_realized == 1.0000


def test_v1233_measure_integration_dim_cell_count_6():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v1233_integration_dim_cell_count == 6


def test_v1233_measure_total_cells_338():
    """V1233 扩 matrix: 26 dim × 13 R = 338 cells (主 19:33 + 主 22:08)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v1233_total_cells == 338  # 26 × 13


def test_v1233_measure_realized_cells_count_196():
    """V1233 realized: 190 (V1232) + 6 (INTEGRATION) = 196."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v1233_realized_cells_count == 196


def test_v1233_measure_overall_realized_196_lift_positive():
    """V1233 lift should be positive (主 17:43 实事求是 — 真测)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v1233_overall_realized_196 > V1232_REALIZED_MEAN_190


def test_v1233_measure_overall_realized_196_lift_delta():
    """Expected lift: (6 × 1.0 - 0 × 6) / 196 ≈ +0.0069 (6 cells from 0 to 1.0 over 196 cells)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert 0.004 < rep.v1233_overall_lift_delta_realized_from_v1232 < 0.012


def test_v1233_measure_inflation_gap_positive():
    """inflation_gap = V1232 baseline 1.0 - V1233 overall_mean_338 ≈ 1.0 - 0.4525 ≈ 0.5475."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert 0.50 < rep.v1233_inflation_gap_v1232_minus_realized < 0.60


def test_v1233_measure_position_north_star_pct():
    """Expected ~79.55% from V1233 lift > V1232 79.01% (主 22:33 + 主 19:33)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert 79.0 < rep.position_of_north_star_realized_pct < 80.5


def test_v1233_measure_total_integration_molecules():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.total_integration_molecules == 60


def test_v1233_measure_all_pathways_pass():
    """6/6 pathways should pass (主 13:31 大胆激进 + 主 17:43 实事求是)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1233_measure_per_pathway_molecule_count():
    """Each pathway should have 10 real molecules (主 19:33 站在前人肩上 — 真分子深挖)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    for k, c in rep.pathway_real_molecule_count.items():
        assert c == 10, f"{k} has {c} molecules, expected 10"


def test_v1233_v3_guards_all_pass():
    """14 V3 哲学守门 all PASS (主 17:58 + 主 20:46 不假装)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1233_v3_guard_realized_not_asi():
    """realized_not_asi = V1233 overall_realized_196 < ASI North Star 0.98 (主 17:58)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1233_overall_realized_196 < ASI_NORTH_STAR


def test_v1233_v3_guard_60_mol_not_complete():
    """v1233_60_mol_not_complete: 60 真分子 ≠ 完整 integration substrate (主 17:43)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_60_mol_not_complete"] is True


def test_v1233_v3_guard_new_dim_not_full_coverage():
    """v1233_new_dim_not_full_coverage: integration_dim_cell_count (6) < 13 (主 17:43)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_new_dim_not_full_coverage"] is True
    assert rep.v3_guards["v1233_not_full_integration_lift"] is True


def test_v1233_v3_guard_phase2_start_integration():
    """V1233 = ASI V2 Phase 2 起点 (闭环之上整合 dim, 主 22:33)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_phase2_start_integration"] is True


def test_v1233_v3_guard_integration_5_positions():
    """V1233 integration 闭合 ASI 5 位置 (调度 + 哲学 + 涌现 + 价值 + ASI, 主 22:08)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_integration_5_positions"] is True


def test_v1233_v3_guard_no_pretend_phenomenal():
    """V1233 不假装 integration = phenomenal consciousness (主 17:58)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_does_not_pretend_phenomenal"] is True


def test_v1233_v3_guard_no_pretend_reach_asi():
    """V1233 不假装 integration = ASI V1.0 (主 20:46)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_does_not_pretend_reach_asi"] is True


def test_v1233_artifact_default_path():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import (
        measure_v1233_full, write_v1233_artifact,
    )
    rep = measure_v1233_full()
    path = write_v1233_artifact(rep)
    assert path.exists()


def test_v1233_report_default_path():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import (
        measure_v1233_full, write_v1233_report,
    )
    rep = measure_v1233_full()
    path = write_v1233_report(rep)
    assert path.exists()


def test_v1233_artifact_valid_json():
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import (
        measure_v1233_full, write_v1233_artifact,
    )
    rep = measure_v1233_full()
    path = write_v1233_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["dim_version"] == "0.6.43"
    assert data["north_star"] == 0.9800


def _run_cli(*args):
    """Helper: run V1233 CLI with UTF-8 encoding (Windows GBK fix)."""
    return subprocess.run(
        [sys.executable, "-m", "apeireth.v1233_asi_v0643_integration_substrate_real_lift", *args],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parents[1]),
    )


def test_v1233_cli_runs():
    """CLI --help / default runs without error (主 00:56 任何人都能接手)."""
    result = _run_cli()
    assert result.returncode == 0, f"CLI failed: {result.stderr}"
    out = result.stdout or ""
    assert "V1233" in out
    assert "integration_substrate_real_lift" in out


def test_v1233_cli_json_flag():
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
    assert parsed["dim_version"] == "0.6.43"


def test_v1233_cli_full_flag():
    """CLI --full shows pathway scores + integration coverage + V3 guards."""
    result = _run_cli("--full")
    assert result.returncode == 0
    out = result.stdout or ""
    assert "INTEGRATION_PHILOSOPHY" in out
    assert "INTEGRATION_NEURO_DEFAULT" in out
    assert "INTEGRATION_PHYSICS" in out
    assert "V3" in out  # 哲学守门 → V3 guard table


def test_v1233_v1232_baseline_write_dead():
    """V1233 must write dead V1232 baseline (主 17:43 实事求是 — 写死历史值, 不能改)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import (
        V1232_REALIZED_MEAN_190, V1232_OVERALL_MEAN_325,
    )
    assert V1232_REALIZED_MEAN_190 == 0.7742
    assert V1232_OVERALL_MEAN_325 == 0.4525


def test_v1233_integration_substrate_each_pathway_10_molecules():
    """Each of 6 pathways must have exactly 10 molecules (主 19:33 站在前人肩上 — 真分子深挖)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    for k, p in V1233_INTEGRATION_SUBSTRATE.items():
        assert len(p["molecules"]) == 10, f"{k} has {len(p['molecules'])} molecules"
        assert len(p["cascade_order"]) == 10, f"{k} cascade_order has {len(p['cascade_order'])} entries"


def test_v1233_integration_substrate_all_molecules_real():
    """All 60 molecules should be marked real=True (主 17:43 实事求是 — 真分子)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_SUBSTRATE
    for k, p in V1233_INTEGRATION_SUBSTRATE.items():
        for m in p["molecules"]:
            assert m.get("real") is True, f"{k}/{m.get('name')} not real"


def test_v1233_pathway_score_all_above_threshold():
    """All 6 pathway scores should be >= 0.7 (主 13:31 大胆激进 — 真测)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    for k, s in rep.pathway_scores.items():
        assert s >= 0.7, f"{k} score {s:.4f} < 0.7"


def test_v1233_integration_coverage_total_6_lifted_7_vacuous():
    """主 17:43 实事求是: 6 cell lifted + 7 cell vacuous = 13 R substrates (主 19:33)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import V1233_INTEGRATION_COVERAGE
    lifted = sum(1 for v in V1233_INTEGRATION_COVERAGE.values() if v >= 0.3)
    vacuous = sum(1 for v in V1233_INTEGRATION_COVERAGE.values() if v == 0.0)
    assert lifted == 6
    assert vacuous == 7
    assert lifted + vacuous == 13  # INTEGRATION dim has 13 R cells


def test_v1233_v1233_report_includes_v1232_v1231_v1230_v1229_baselines():
    """V1233 must include 4 prior dim baselines (V1232/V1231/V1230/V1229)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v1232_realized_mean_190_baseline == 0.7742
    assert rep.v1231_realized_mean_184_baseline == 0.7669
    assert rep.v1230_realized_mean_178_baseline == 0.7590
    assert rep.v1229_realized_mean_172_baseline == 0.7505


def test_v1233_no_pretend_full_replace():
    """主 17:58 不假装: V1233 ≠ V1232 全替代 (V1232 仍 own 25 dim matrix)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_not_full_replace"] is True


def test_v1233_no_pretend_v1():
    """主 20:46 不假装: V1233 lift ≠ ASI V1.0 (V1233 = V0.6.43 中间版本)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["v1233_lift_not_v1"] is True


def test_v1233_vacuous_gap_real():
    """主 17:43 实事求是: 338 cell 公式下 inflation 仍出现 (vacuous_gap_real = True)."""
    from apeireth.v1233_asi_v0643_integration_substrate_real_lift import measure_v1233_full
    rep = measure_v1233_full()
    assert rep.v3_guards["vacuous_gap_real"] is True
    assert rep.v1233_inflation_gap_v1232_minus_realized > 0.0