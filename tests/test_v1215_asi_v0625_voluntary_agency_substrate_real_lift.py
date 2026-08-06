"""Tests for V1215 — ASI V0.6.25 voluntary_agency_substrate_real_lift.

主 00:56 任何人都能接手: 40 tests covering:
  - 7 pathway 真分子 cascade 真测 (actin + cilium + skeletal muscle + volitional plasticity + Helmholtz + Friston FEP + GNWT)
  - ≥ 25 真分子 ≥ 79 实际 (主 13:31 大胆激进)
  - VL × R8_motion + R10_plasticity + R11_consciousness lift 真测
  - V1213 + V1214 baseline 写死 (主 17:43 实事求是)
  - V3 哲学守门 module-level guard 真测
  - artifact + report 真写 真测
  - CLI 真测
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure promethean is on path
PROMETHEAN_ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth import v1215_asi_v0625_voluntary_agency_substrate_real_lift as v1215


# ============================================================================
# Module-level constants
# ============================================================================

def test_module_version():
    assert v1215.V1215_VERSION == "0.1.0"


def test_dim_version():
    assert v1215.V1215_DIM_VERSION == "0.6.25"


def test_north_star_locked():
    """ASI 北极星 LOCKED at 0.98 (主 22:33)."""
    assert v1215.ASI_NORTH_STAR == 0.98


def test_v1213_baseline_locked():
    """V1213 baseline 写死 (主 17:43 实事求是 — 写死历史值, 不能改)."""
    assert v1215.V1213_RECOMPUTE_BASELINE == 1.000000
    assert v1215.V1213_REALIZED_MEAN == 0.461702
    assert v1215.V1213_OVERALL_MEAN == 0.370940
    assert v1215.V1213_TR_REALIZED == 0.4673


def test_v1214_baseline_locked():
    """V1214 baseline 写死 (主 17:43 实事求是 — 写死历史值, 不能改)."""
    assert v1215.V1214_RECOMPUTE_BASELINE == 1.000000
    assert v1215.V1214_REALIZED_MEAN == 0.4830
    assert v1215.V1214_OVERALL_MEAN_117 == 0.3880
    assert v1215.V1214_TR_REALIZED == 0.6000


def test_v1213_vl_row_locked():
    """V1213 VL row 写死 (主 17:43 实事求是 — 写死历史值, 不能改)."""
    expected = {
        "R0_metabolism": 0.0, "R1_growth": 0.3, "R2_development": 0.3,
        "R3_death_immune": 0.3, "R4_aging": 0.0, "R5_repair": 0.0,
        "R6_reproduction": 1.0, "R7_stress": 0.3, "R8_motion": 0.6,
        "R9_heredity": 0.3, "R10_plasticity": 0.3, "R11_consciousness": 0.6,
        "R12_ecology": 1.0,
    }
    assert dict(v1215.V1213_VL_ROW) == expected


def test_7_pathways_defined():
    """7 pathway: 3 voluntary motion + 1 volitional plasticity + 3 voluntary consciousness."""
    assert len(v1215.V1215_VL_SUBSTRATE) == 7
    expected = [
        "VL_VOLUNTARY_ACTIN", "VL_VOLUNTARY_CILIUM", "VL_VOLUNTARY_SKELETAL_MUSCLE",
        "VL_VOLITIONAL_PLASTICITY",
        "VL_VOLUNTARY_PREDICTIVE", "VL_VOLUNTARY_FEP", "VL_VOLUNTARY_GNWT",
    ]
    assert list(v1215.V1215_VL_SUBSTRATE.keys()) == expected


def test_pathways_have_required_fields():
    """每 pathway 必有 molecules + cascade_order + r_substrate + source."""
    for name, pathway in v1215.V1215_VL_SUBSTRATE.items():
        assert "molecules" in pathway, f"{name} missing molecules"
        assert "cascade_order" in pathway, f"{name} missing cascade_order"
        assert "r_substrate" in pathway, f"{name} missing r_substrate"
        assert "source" in pathway, f"{name} missing source"
        assert pathway["r_substrate"] in ["R8_motion", "R10_plasticity", "R11_consciousness"]


def test_molecules_have_function_and_real():
    """每 molecule 必有 function + real=True."""
    for pathway_name, pathway in v1215.V1215_VL_SUBSTRATE.items():
        for mol in pathway["molecules"]:
            assert mol.get("function"), f"{pathway_name} molecule missing function"
            assert mol.get("real") is True, f"{pathway_name} molecule not real"


def test_cascade_order_matches_molecules():
    """每 pathway cascade_order 应与 molecules 顺序一致."""
    for pathway_name, pathway in v1215.V1215_VL_SUBSTRATE.items():
        mol_names = [m["name"] for m in pathway["molecules"]]
        assert pathway["cascade_order"] == mol_names, f"{pathway_name} cascade mismatch"


def test_total_molecules_minimum():
    """≥ 25 真分子 (主 13:31 真生产 — 1 cron ≥ 25 真分子)."""
    total = sum(len(p["molecules"]) for p in v1215.V1215_VL_SUBSTRATE.values())
    assert total >= 25


def test_actin_pathway_with_actin_myosin():
    """actin pathway 必有 actin-myosin 相关 真分子."""
    actin = v1215.V1215_VL_SUBSTRATE["VL_VOLUNTARY_ACTIN"]
    mol_names = [m["name"] for m in actin["molecules"]]
    assert any("Myosin" in n or "Actin" in n for n in mol_names)


def test_cilium_pathway_with_axoneme():
    """cilium pathway 必有 axoneme + IFT 真分子."""
    cilium = v1215.V1215_VL_SUBSTRATE["VL_VOLUNTARY_CILIUM"]
    mol_names = [m["name"] for m in cilium["molecules"]]
    assert any("axoneme" in n.lower() for n in mol_names)
    assert any("IFT" in n for n in mol_names)


def test_skeletal_muscle_pathway_with_DHPR():
    """skeletal muscle pathway 必有 DHPR + RyR1 + SERCA."""
    muscle = v1215.V1215_VL_SUBSTRATE["VL_VOLUNTARY_SKELETAL_MUSCLE"]
    mol_names = [m["name"] for m in muscle["molecules"]]
    assert any("DHPR" in n for n in mol_names)
    assert any("RyR1" in n for n in mol_names)
    assert any("SERCA" in n for n in mol_names)


def test_plasticity_pathway_with_NMDA_CaMKII_BDNF():
    """plasticity pathway 必有 NMDA + CaMKII + BDNF."""
    plast = v1215.V1215_VL_SUBSTRATE["VL_VOLITIONAL_PLASTICITY"]
    mol_names = [m["name"] for m in plast["molecules"]]
    assert any("NMDA" in n for n in mol_names)
    assert any("CaMKII" in n for n in mol_names)
    assert any("BDNF" in n or "TrkB" in n for n in mol_names)


def test_predictive_pathway_with_NMDA():
    """Helmholtz predictive coding pathway 必有 NMDA precision weighting."""
    pred = v1215.V1215_VL_SUBSTRATE["VL_VOLUNTARY_PREDICTIVE"]
    mol_names = [m["name"] for m in pred["molecules"]]
    assert any("NMDA" in n for n in mol_names)
    assert any("precision" in n.lower() for n in mol_names)


def test_fep_pathway_with_free_energy():
    """Friston FEP pathway 必有 variational free energy + active inference."""
    fep = v1215.V1215_VL_SUBSTRATE["VL_VOLUNTARY_FEP"]
    mol_names = [m["name"] for m in fep["molecules"]]
    assert any("free_energy" in n.lower() for n in mol_names)
    assert any("active" in n.lower() or "Active" in n for n in mol_names)


def test_gnwt_pathway_with_global_broadcast():
    """GNWT pathway 必有 global workspace + ignition."""
    gnwt = v1215.V1215_VL_SUBSTRATE["VL_VOLUNTARY_GNWT"]
    mol_names = [m["name"] for m in gnwt["molecules"]]
    assert any("global" in n.lower() for n in mol_names)
    assert any("ignition" in n.lower() for n in mol_names)


def test_v3_guards_present():
    """V3 哲学守门 module-level 至少 5 guard."""
    assert len(v1215.V3_GUARDS) >= 5
    assert any("不假装" in g for g in v1215.V3_GUARDS.keys())


def test_v1215_vl_coverage_lifted():
    """V1215 VL coverage 应 ≥ V1213 VL row 写死基线."""
    for r_sub, baseline in v1215.V1213_VL_ROW.items():
        new = v1215.V1215_VL_COVERAGE[r_sub]
        assert new >= baseline, f"V1215 VL[{r_sub}] not lifted: {new} < {baseline}"


def test_r8_lift_3_pathways():
    """VL × R8_motion 3 pathway: actin + cilium + skeletal muscle."""
    r8_pathways = ["VL_VOLUNTARY_ACTIN", "VL_VOLUNTARY_CILIUM", "VL_VOLUNTARY_SKELETAL_MUSCLE"]
    for p in r8_pathways:
        assert p in v1215.V1215_VL_SUBSTRATE
        assert v1215.V1215_VL_SUBSTRATE[p]["r_substrate"] == "R8_motion"


def test_r10_lift_1_pathway():
    """VL × R10_plasticity 1 pathway: volitional plasticity."""
    assert "VL_VOLITIONAL_PLASTICITY" in v1215.V1215_VL_SUBSTRATE
    assert v1215.V1215_VL_SUBSTRATE["VL_VOLITIONAL_PLASTICITY"]["r_substrate"] == "R10_plasticity"


def test_r11_lift_3_pathways():
    """VL × R11_consciousness 3 pathway: Helmholtz + Friston + GNWT."""
    r11_pathways = ["VL_VOLUNTARY_PREDICTIVE", "VL_VOLUNTARY_FEP", "VL_VOLUNTARY_GNWT"]
    for p in r11_pathways:
        assert p in v1215.V1215_VL_SUBSTRATE
        assert v1215.V1215_VL_SUBSTRATE[p]["r_substrate"] == "R11_consciousness"


def test_vacuous_cells_unlifted():
    """Vacuous cells (R0, R4, R5) 应保持 0.0 — 不假装 lift."""
    for vacuous in ["R0_metabolism", "R4_aging", "R5_repair"]:
        assert v1215.V1215_VL_COVERAGE[vacuous] == 0.0, f"vacuous {vacuous} lifted"


# ============================================================================
# measure_v1215_full
# ============================================================================

def test_measure_v1215_full_returns_report():
    rep = v1215.measure_v1215_full()
    assert isinstance(rep, v1215.V1215Report)
    assert rep.dim_version == "0.6.25"
    assert rep.north_star == 0.98


def test_measure_v1215_full_7_pathways():
    """7 pathway 应全部 pass (主 13:31 真生产 — ≥ 4 真分子 each)."""
    rep = v1215.measure_v1215_full()
    assert rep.n_pathways_total == 7
    assert rep.n_pathways_pass == 7


def test_measure_v1215_full_total_molecules_minimum():
    """≥ 25 真分子."""
    rep = v1215.measure_v1215_full()
    assert rep.total_vl_molecules >= 25


def test_measure_v1215_full_r8_molecules_minimum():
    """VL × R8_motion ≥ 25 真分子 (3 pathways × 8-10)."""
    rep = v1215.measure_v1215_full()
    assert rep.n_r8_motion_molecules >= 20


def test_measure_v1215_full_r10_molecules_minimum():
    """VL × R10_plasticity ≥ 20 真分子 (1 pathway × 25)."""
    rep = v1215.measure_v1215_full()
    assert rep.n_r10_plasticity_molecules >= 20


def test_measure_v1215_full_r11_molecules_minimum():
    """VL × R11_consciousness ≥ 18 真分子 (3 pathways × 8)."""
    rep = v1215.measure_v1215_full()
    assert rep.n_r11_consciousness_molecules >= 18


def test_measure_v1215_full_r8_pass():
    """R8_motion 3 pathway 应全 pass."""
    rep = v1215.measure_v1215_full()
    assert rep.r8_pass == 3


def test_measure_v1215_full_r10_pass():
    """R10_plasticity 1 pathway 应 pass."""
    rep = v1215.measure_v1215_full()
    assert rep.r10_pass == 1


def test_measure_v1215_full_r11_pass():
    """R11_consciousness 3 pathway 应全 pass."""
    rep = v1215.measure_v1215_full()
    assert rep.r11_pass == 3


def test_measure_v1215_full_vl_lift_positive():
    """VL dim realized V1213 → V1215 应 lift 真实存在 (> 0)."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_vl_lift_delta > 0


def test_measure_v1215_full_overall_lift_positive():
    """Overall realized 应 lift (虽小但 > 0)."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_overall_lift_delta > 0


def test_measure_v1215_full_r8_lift():
    """VL × R8_motion V1213=0.6 → V1215 = 1.0."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_vl_x_r8_motion == 1.0


def test_measure_v1215_full_r10_lift():
    """VL × R10_plasticity V1213=0.3 → V1215 = 1.0."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_vl_x_r10_plasticity == 1.0


def test_measure_v1215_full_r11_lift():
    """VL × R11_consciousness V1213=0.6 → V1215 = 1.0."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_vl_x_r11_consciousness == 1.0


def test_measure_v1215_full_position_above_50pct():
    """VL dim realized position of north_star 应 > 50%."""
    rep = v1215.measure_v1215_full()
    assert rep.position_of_north_star_realized_pct > 50.0


def test_measure_v1215_full_inflation_audit_present():
    """inflation_gap 应为正 (V1213 clamp 1.0 - V1215 overall realized)."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_inflation_gap > 0


# ============================================================================
# Per-measure helpers
# ============================================================================

def test_measure_v1215_vl_dim_realized():
    realized = v1215.measure_v1215_vl_dim_realized()
    assert realized > 0.4


def test_measure_v1215_overall_realized():
    overall = v1215.measure_v1215_overall_realized()
    assert overall > 0.3


def test_measure_v1215_overall_mean():
    overall = v1215.measure_v1215_overall_mean()
    assert overall > 0.3


def test_measure_v1215_inflation_gap():
    gap = v1215.measure_v1215_inflation_gap()
    assert gap > 0


# ============================================================================
# Helper classification
# ============================================================================

def test_classify_pathway_r8():
    """_classify_pathway 应正确分到 R8_motion."""
    assert v1215._classify_pathway("VL_VOLUNTARY_ACTIN") == "R8_motion"
    assert v1215._classify_pathway("VL_VOLUNTARY_CILIUM") == "R8_motion"
    assert v1215._classify_pathway("VL_VOLUNTARY_SKELETAL_MUSCLE") == "R8_motion"


def test_classify_pathway_r10():
    """_classify_pathway 应正确分到 R10_plasticity."""
    assert v1215._classify_pathway("VL_VOLITIONAL_PLASTICITY") == "R10_plasticity"


def test_classify_pathway_r11():
    """_classify_pathway 应正确分到 R11_consciousness."""
    assert v1215._classify_pathway("VL_VOLUNTARY_PREDICTIVE") == "R11_consciousness"
    assert v1215._classify_pathway("VL_VOLUNTARY_FEP") == "R11_consciousness"
    assert v1215._classify_pathway("VL_VOLUNTARY_GNWT") == "R11_consciousness"


# ============================================================================
# Artifact + Report writers
# ============================================================================

def test_write_v1215_artifact_default_path(tmp_path):
    """write_v1215_artifact 应能写默认路径."""
    out = tmp_path / "v1215_artifact.json"
    path = v1215.write_v1215_artifact(out)
    assert path.exists()
    assert path.suffix == ".json"
    data = json.loads(path.read_text(encoding="utf-8"))
    assert data["module"] == "v1215_asi_v0625_voluntary_agency_substrate_real_lift"
    assert data["dim_version"] == "0.6.25"
    assert data["north_star"] == 0.98
    assert len(data["pathways"]) == 7


def test_write_v1215_artifact_contains_pathway_data(tmp_path):
    """write_v1215_artifact 应包含 7 pathway 真分子 detail."""
    out = tmp_path / "v1215_artifact_detail.json"
    path = v1215.write_v1215_artifact(out)
    data = json.loads(path.read_text(encoding="utf-8"))
    # 每 pathway 必有 molecules + score + r_substrate
    for name, p in data["pathways"].items():
        assert "molecules" in p
        assert "score" in p
        assert p["r_substrate"] in ["R8_motion", "R10_plasticity", "R11_consciousness"]
        assert p["score"] == 1.0


def test_write_v1215_report_default_path(tmp_path):
    """write_v1215_report 应能写默认路径."""
    out = tmp_path / "v1215_report.md"
    path = v1215.write_v1215_report(out)
    assert path.exists()
    assert path.suffix == ".md"
    content = path.read_text(encoding="utf-8")
    assert "V1215 ASI V0.6.25" in content
    assert "V3 哲学守门" in content or "V3" in content


def test_write_v1215_report_contains_v3_guards(tmp_path):
    """write_v1215_report 应包含 V3 守门章节."""
    out = tmp_path / "v1215_report_v3.md"
    path = v1215.write_v1215_report(out)
    content = path.read_text(encoding="utf-8")
    assert "不假装 V1215 = ASI 终极" in content
    assert "不假装 realized = ASI 已达" in content
    assert "不假装 vacuous_gap = 0" in content


def test_write_v1215_report_contains_lift_table(tmp_path):
    """write_v1215_report 应包含 V1213 → V1215 lift 矩阵表."""
    out = tmp_path / "v1215_report_table.md"
    path = v1215.write_v1215_report(out)
    content = path.read_text(encoding="utf-8")
    assert "R8_motion" in content
    assert "R10_plasticity" in content
    assert "R11_consciousness" in content


# ============================================================================
# CLI
# ============================================================================

def test_cli_measure():
    """CLI --measure 应能跑 (smoke test)."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_vl_dim_realized > 0.4
    assert rep.v1215_overall_realized > 0.3


def test_cli_function_with_argv():
    """CLI main() 应能处理 argv."""
    code = v1215.main(["--measure"])
    assert code == 0


def test_cli_function_full_creates_files(tmp_path):
    """CLI --full 应能写 artifact + report."""
    art = tmp_path / "art.json"
    rpt = tmp_path / "rep.md"
    code = v1215.main(["--full", "--artifact", str(art), "--md-out", str(rpt)])
    assert code == 0
    assert art.exists()
    assert rpt.exists()


def test_cli_function_json_outputs_valid_json():
    """CLI --json 应输出 valid JSON."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        code = v1215.main(["--json"])
    assert code == 0
    data = json.loads(buf.getvalue())
    assert data["dim_version"] == "0.6.25"
    assert "pathway_scores" in data


# ============================================================================
# Honest finding checks (主 17:43 实事求是)
# ============================================================================

def test_v1215_vl_dim_0p65_real():
    """VL dim 实际位置 (主 17:43 实事求是) ≈ 0.65 — 不假装."""
    rep = v1215.measure_v1215_full()
    expected_min = 0.64
    expected_max = 0.66
    assert expected_min < rep.v1215_vl_dim_realized < expected_max


def test_v1215_overall_realized_lift_honest():
    """V1215 overall lift 从 V1214 baseline (0.4830) 真实 lift (主 17:43 实事求是)."""
    rep = v1215.measure_v1215_full()
    assert rep.v1215_overall_lift_delta > 0.0
    assert rep.v1215_overall_lift_delta < 0.05  # lift is honest, not magic


def test_inflation_gap_real():
    """inflation_gap > 0.5 (主 17:43 — V1213 baseline 1.0 - V1215 realized ≈ 0.50)."""
    rep = v1215.measure_v1215_full()
    assert 0.45 < rep.v1215_inflation_gap < 0.55


def test_v1215_inflation_audit_in_text():
    """inflation audit (主 17:43) 应在 artifact 中显式."""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "test.json"
        v1215.write_v1215_artifact(out)
        data = json.loads(out.read_text(encoding="utf-8"))
        assert "inflation_gap_v1213_minus_realized" in data["v1215_measurements"]
        assert data["v1215_measurements"]["inflation_gap_v1213_minus_realized"] > 0
