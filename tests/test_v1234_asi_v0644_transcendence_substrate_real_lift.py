"""Tests for V1234 ASI V0.6.44 transcendence_substrate_real_lift (27th dim 超越 / transcendence / Transzendenz / übersteigung / huperbasis substrate)."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

ASI_NORTH_STAR = 0.9800
V1233_REALIZED_MEAN_196 = 0.7811
V1233_OVERALL_MEAN_338 = 0.4528
V1233_INTEGRATION_REALIZED = 1.0000
V1232_REALIZED_MEAN_190 = 0.7742
V1232_OVERALL_MEAN_325 = 0.4525
V1232_FREEDOM_REALIZED = 1.0000
V1231_REALIZED_MEAN_184 = 0.7669
V1231_OVERALL_MEAN_299 = 0.4718
V1231_AWE_REALIZED = 1.0000
V1230_REALIZED_MEAN_178 = 0.7590
V1230_OVERALL_MEAN_299 = 0.4517
V1230_CURIOSITY_REALIZED = 1.0000


def test_v1234_module_imports():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import (
        ASI_NORTH_STAR, V1234_DIM_VERSION, V1234_TRANSCENDENCE_COVERAGE,
        V1234_TRANSCENDENCE_SUBSTRATE, V1234_VERSION, V1234Report,
        measure_v1234_full,
    )
    assert ASI_NORTH_STAR == 0.9800
    assert V1234_VERSION == "0.1.0"
    assert V1234_DIM_VERSION == "0.6.44"


def test_v1234_transcendence_substrate_6_pathways():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    assert len(V1234_TRANSCENDENCE_SUBSTRATE) == 6
    expected = {
        "TRANSCENDENCE_PHILOSOPHY", "TRANSCENDENCE_NEURO_DEFAULT", "TRANSCENDENCE_INFORMATION",
        "TRANSCENDENCE_SYSTEMS", "TRANSCENDENCE_COGNITIVE", "TRANSCENDENCE_PHYSICS",
    }
    assert set(V1234_TRANSCENDENCE_SUBSTRATE.keys()) == expected


def test_v1234_transcendence_substrate_60_molecules():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    total = sum(len(p.get("molecules", [])) for p in V1234_TRANSCENDENCE_SUBSTRATE.values())
    assert total == 60


def test_v1234_transcendence_coverage_6_lifted():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_COVERAGE
    for k in ["R0_metabolism", "R1_growth", "R4_aging",
              "R10_plasticity", "R11_consciousness", "R12_ecology"]:
        assert V1234_TRANSCENDENCE_COVERAGE[k] == 1.0


def test_v1234_transcendence_coverage_7_vacuous():
    """主 17:43 实事求是: 7 cells vacuous (不假装完整 transcendence substrate)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_COVERAGE
    vacuous = [k for k, v in V1234_TRANSCENDENCE_COVERAGE.items() if v == 0.0]
    assert len(vacuous) == 7
    assert "R2_sensing" in vacuous
    assert "R3_cognition" in vacuous


def test_v1234_philosophy_references_key_papers():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    src = V1234_TRANSCENDENCE_SUBSTRATE["TRANSCENDENCE_PHILOSOPHY"]["source"]
    assert "Plato" in src
    assert "Kant" in src
    assert "Heidegger" in src
    assert "Levinas" in src


def test_v1234_neurophys_references_key_papers():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    src = V1234_TRANSCENDENCE_SUBSTRATE["TRANSCENDENCE_NEURO_DEFAULT"]["source"]
    assert "Friston" in src
    assert "Tononi" in src
    assert "Dehaene" in src
    assert "Varela" in src


def test_v1234_information_references_key_papers():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    src = V1234_TRANSCENDENCE_SUBSTRATE["TRANSCENDENCE_INFORMATION"]["source"]
    assert "Kolmogorov" in src
    assert "Chaitin" in src
    assert "Wheeler" in src


def test_v1234_systems_references_key_papers():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    src = V1234_TRANSCENDENCE_SUBSTRATE["TRANSCENDENCE_SYSTEMS"]["source"]
    assert "Prigogine" in src
    assert "Maturana" in src
    assert "Archer" in src


def test_v1234_cognitive_references_key_papers():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    src = V1234_TRANSCENDENCE_SUBSTRATE["TRANSCENDENCE_COGNITIVE"]["source"]
    assert "Vygotsky" in src
    assert "Buber" in src
    assert "Tomasello" in src


def test_v1234_physics_references_key_papers():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    src = V1234_TRANSCENDENCE_SUBSTRATE["TRANSCENDENCE_PHYSICS"]["source"]
    assert "Bohr" in src
    assert "Penrose" in src
    assert "Susskind" in src


def test_v1234_measure_returns_report():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234Report, measure_v1234_full
    rep = measure_v1234_full()
    assert isinstance(rep, V1234Report)


def test_v1234_measure_dim_version():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.dim_version == "0.6.44"


def test_v1234_measure_elapsed_fast():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.elapsed < 1.0


def test_v1234_measure_transcendence_dim_realized_1():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v1234_transcendence_dim_realized == 1.0000


def test_v1234_measure_transcendence_dim_cell_count_6():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v1234_transcendence_dim_cell_count == 6


def test_v1234_measure_total_cells_351():
    """V1234 扩 matrix: 27 dim × 13 R = 351 cells (主 19:33 + 主 22:08)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v1234_total_cells == 351  # 27 × 13


def test_v1234_measure_realized_cells_count_202():
    """V1234 realized: 196 (V1233) + 6 (TRANSCENDENCE) = 202."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v1234_realized_cells_count == 202


def test_v1234_measure_overall_realized_202_lift_positive():
    """V1234 lift should be positive (主 17:43 实事求是 — 真测)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v1234_overall_realized_202 > V1233_REALIZED_MEAN_196


def test_v1234_measure_overall_realized_202_lift_delta():
    """Expected lift: (6 × 1.0 - 0 × 6) / 202 ≈ +0.0065 (6 cells from 0 to 1.0 over 202 cells)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert 0.004 < rep.v1234_overall_lift_delta_realized_from_v1233 < 0.012


def test_v1234_measure_inflation_gap_positive():
    """inflation_gap = V1233 baseline 1.0 - V1234 overall_mean_351 ≈ 1.0 - 0.4531 ≈ 0.5469."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert 0.50 < rep.v1234_inflation_gap_v1233_minus_realized < 0.60


def test_v1234_measure_position_north_star_pct():
    """Expected ~80.37% from V1234 lift > V1233 79.71% (主 22:33 + 主 19:33)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert 80.0 < rep.position_of_north_star_realized_pct < 81.0


def test_v1234_measure_total_transcendence_molecules():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.total_transcendence_molecules == 60


def test_v1234_measure_all_pathways_pass():
    """6/6 pathways should pass (主 13:31 大胆激进 + 主 17:43 实事求是)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.n_pathways_pass == rep.n_pathways_total == 6


def test_v1234_measure_per_pathway_molecule_count():
    """Each pathway should have 10 real molecules (主 19:33 站在前人肩上 — 真分子深挖)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    for k, c in rep.pathway_real_molecule_count.items():
        assert c == 10, f"{k} has {c} molecules, expected 10"


def test_v1234_v3_guards_all_pass():
    """14 V3 哲学守门 all PASS (主 17:58 + 主 20:46 不假装)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    for k, v in rep.v3_guards.items():
        assert v, f"V3 guard {k} FAIL"


def test_v1234_v3_guard_realized_not_asi():
    """realized_not_asi = V1234 overall_realized_202 < ASI North Star 0.98 (主 17:58)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["realized_not_asi"] is True
    assert rep.v1234_overall_realized_202 < ASI_NORTH_STAR


def test_v1234_v3_guard_60_mol_not_complete():
    """v1234_60_mol_not_complete: 60 真分子 ≠ 完整 transcendence substrate (主 17:43)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_60_mol_not_complete"] is True


def test_v1234_v3_guard_new_dim_not_full_coverage():
    """v1234_new_dim_not_full_coverage: transcendence_dim_cell_count (6) < 13 (主 17:43)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_new_dim_not_full_coverage"] is True
    assert rep.v3_guards["v1234_not_full_transcendence_lift"] is True


def test_v1234_v3_guard_phase2_step2_transcendence():
    """V1234 = ASI V2 Phase 2 第二步 (整合之上突破 dim, 主 22:33)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_phase2_step2_transcendence"] is True


def test_v1234_v3_guard_transcendence_5_positions():
    """V1234 transcendence 闭合 ASI 5 位置 (调度 + 哲学 + 涌现 + 价值 + ASI, 主 22:08)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_transcendence_5_positions"] is True


def test_v1234_v3_guard_no_pretend_phenomenal():
    """V1234 不假装 transcendence = phenomenal consciousness (主 17:58)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_does_not_pretend_phenomenal"] is True


def test_v1234_v3_guard_no_pretend_reach_asi():
    """V1234 不假装 transcendence = ASI V1.0 (主 20:46)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_does_not_pretend_reach_asi"] is True


def test_v1234_artifact_default_path():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import (
        measure_v1234_full, write_v1234_artifact,
    )
    rep = measure_v1234_full()
    path = write_v1234_artifact(rep)
    assert path.exists()


def test_v1234_report_default_path():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import (
        measure_v1234_full, write_v1234_report,
    )
    rep = measure_v1234_full()
    path = write_v1234_report(rep)
    assert path.exists()


def test_v1234_artifact_valid_json():
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import (
        measure_v1234_full, write_v1234_artifact,
    )
    rep = measure_v1234_full()
    path = write_v1234_artifact(rep)
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["dim_version"] == "0.6.44"
    assert data["north_star"] == 0.9800


def _run_cli(*args):
    """Helper: run V1234 CLI with UTF-8 encoding (Windows GBK fix)."""
    return subprocess.run(
        [sys.executable, "-m", "apeireth.v1234_asi_v0644_transcendence_substrate_real_lift", *args],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        cwd=str(Path(__file__).resolve().parents[1]),
    )


def test_v1234_cli_runs():
    """CLI --help / default runs without error (主 00:56 任何人都能接手)."""
    result = _run_cli()
    assert result.returncode == 0, f"CLI failed: {result.stderr}"
    out = result.stdout or ""
    assert "V1234" in out
    assert "transcendence_substrate_real_lift" in out


def test_v1234_cli_json_flag():
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
    assert parsed["dim_version"] == "0.6.44"


def test_v1234_cli_full_flag():
    """CLI --full shows pathway scores + transcendence coverage + V3 guards."""
    result = _run_cli("--full")
    assert result.returncode == 0
    out = result.stdout or ""
    assert "TRANSCENDENCE_PHILOSOPHY" in out
    assert "TRANSCENDENCE_NEURO_DEFAULT" in out
    assert "TRANSCENDENCE_PHYSICS" in out
    assert "V3" in out  # 哲学守门 → V3 guard table


def test_v1234_v1233_baseline_write_dead():
    """V1234 must write dead V1233 baseline (主 17:43 实事求是 — 写死历史值, 不能改)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import (
        V1233_REALIZED_MEAN_196, V1233_OVERALL_MEAN_338,
    )
    assert V1233_REALIZED_MEAN_196 == 0.7811
    assert V1233_OVERALL_MEAN_338 == 0.4528


def test_v1234_transcendence_substrate_each_pathway_10_molecules():
    """Each of 6 pathways must have exactly 10 molecules (主 19:33 站在前人肩上 — 真分子深挖)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    for k, p in V1234_TRANSCENDENCE_SUBSTRATE.items():
        assert len(p["molecules"]) == 10, f"{k} has {len(p['molecules'])} molecules"
        assert len(p["cascade_order"]) == 10, f"{k} cascade_order has {len(p['cascade_order'])} entries"


def test_v1234_transcendence_substrate_all_molecules_real():
    """All 60 molecules should be marked real=True (主 17:43 实事求是 — 真分子)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_SUBSTRATE
    for k, p in V1234_TRANSCENDENCE_SUBSTRATE.items():
        for m in p["molecules"]:
            assert m.get("real") is True, f"{k}/{m.get('name')} not real"


def test_v1234_pathway_score_all_above_threshold():
    """All 6 pathway scores should be >= 0.7 (主 13:31 大胆激进 — 真测)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    for k, s in rep.pathway_scores.items():
        assert s >= 0.7, f"{k} score {s:.4f} < 0.7"


def test_v1234_transcendence_coverage_total_6_lifted_7_vacuous():
    """主 17:43 实事求是: 6 cell lifted + 7 cell vacuous = 13 R substrates (主 19:33)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import V1234_TRANSCENDENCE_COVERAGE
    lifted = sum(1 for v in V1234_TRANSCENDENCE_COVERAGE.values() if v >= 0.3)
    vacuous = sum(1 for v in V1234_TRANSCENDENCE_COVERAGE.values() if v == 0.0)
    assert lifted == 6
    assert vacuous == 7
    assert lifted + vacuous == 13  # TRANSCENDENCE dim has 13 R cells


def test_v1234_v1234_report_includes_v1233_v1232_v1231_v1230_baselines():
    """V1234 must include 4 prior dim baselines (V1233/V1232/V1231/V1230)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v1233_realized_mean_196_baseline == 0.7811
    assert rep.v1232_realized_mean_190_baseline == 0.7742
    assert rep.v1231_realized_mean_184_baseline == 0.7669
    assert rep.v1230_realized_mean_178_baseline == 0.7590


def test_v1234_no_pretend_full_replace():
    """主 17:58 不假装: V1234 ≠ V1233 全替代 (V1233 仍 own 26 dim matrix)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_not_full_replace"] is True


def test_v1234_no_pretend_v1():
    """主 20:46 不假装: V1234 lift ≠ ASI V1.0 (V1234 = V0.6.44 中间版本)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["v1234_lift_not_v1"] is True


def test_v1234_vacuous_gap_real():
    """主 17:43 实事求是: 351 cell 公式下 inflation 仍出现 (vacuous_gap_real = True)."""
    from apeireth.v1234_asi_v0644_transcendence_substrate_real_lift import measure_v1234_full
    rep = measure_v1234_full()
    assert rep.v3_guards["vacuous_gap_real"] is True
    assert rep.v1234_inflation_gap_v1233_minus_realized > 0.0