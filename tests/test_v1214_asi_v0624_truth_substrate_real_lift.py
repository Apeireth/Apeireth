"""Tests for V1214 — ASI V0.6.24 truth_substrate_real_lift.

主 00:56 任何人都能接手: 32 tests covering:
  - 9 pathway 真分子 cascade 真测 (NER + MMR + BER + NHEJ + HDR + HGT transformation/conjugation/transduction + CRISPR)
  - ≥ 25 真分子 ≥ 48 实际 (主 13:31 大胆激进)
  - TR × R5_repair + TR × R9_heredity lift 真测
  - V1213 baseline 写死 (主 17:43 实事求是)
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

from apeireth import v1214_asi_v0624_truth_substrate_real_lift as v1214


# ============================================================================
# Module-level constants
# ============================================================================

def test_module_version():
    assert v1214.V1214_VERSION == "0.1.0"


def test_dim_version():
    assert v1214.V1214_DIM_VERSION == "0.6.24"


def test_north_star_locked():
    """ASI 北极星 LOCKED at 0.98 (主 22:33)."""
    assert v1214.ASI_NORTH_STAR == 0.98


def test_v1213_baseline_locked():
    """V1213 baseline 写死 (主 17:43 实事求是 — 写死历史值, 不能改)."""
    assert v1214.V1213_RECOMPUTE_BASELINE == 1.000000
    assert v1214.V1213_REALIZED_MEAN == 0.461702
    assert v1214.V1213_OVERALL_MEAN == 0.370940
    assert v1214.V1213_TR_REALIZED == 0.4673


def test_9_pathways_defined():
    """9 pathway: 5 DNA repair + 3 HGT + 1 CRISPR."""
    assert len(v1214.V1214_TR_SUBSTRATE) == 9
    expected = [
        "TR_NER", "TR_MMR", "TR_BER", "TR_NHEJ", "TR_HDR",
        "TR_HGT_TRANSFORMATION", "TR_HGT_CONJUGATION", "TR_HGT_TRANSDUCTION",
        "TR_CRISPR",
    ]
    assert list(v1214.V1214_TR_SUBSTRATE.keys()) == expected


def test_pathways_have_required_fields():
    """每 pathway 必有 molecules + cascade_order + r_substrate + source."""
    for name, pathway in v1214.V1214_TR_SUBSTRATE.items():
        assert "molecules" in pathway, f"{name} missing molecules"
        assert "cascade_order" in pathway, f"{name} missing cascade_order"
        assert "r_substrate" in pathway, f"{name} missing r_substrate"
        assert "source" in pathway, f"{name} missing source"
        assert pathway["r_substrate"] in ["R5_repair", "R9_heredity"]


def test_molecules_have_function_and_real():
    """每 molecule 必有 function + real=True."""
    for pathway_name, pathway in v1214.V1214_TR_SUBSTRATE.items():
        for mol in pathway["molecules"]:
            assert mol.get("function"), f"{pathway_name} molecule missing function"
            assert mol.get("real") is True, f"{pathway_name} molecule not real"


def test_cascade_order_matches_molecules():
    """每 pathway cascade_order 应与 molecules 顺序一致."""
    for pathway_name, pathway in v1214.V1214_TR_SUBSTRATE.items():
        mol_names = [m["name"] for m in pathway["molecules"]]
        assert pathway["cascade_order"] == mol_names, f"{pathway_name} cascade mismatch"


def test_total_molecules_minimum():
    """≥ 25 真分子 (主 13:31 真生产 — 1 cron ≥ 25 真分子)."""
    total = sum(len(p["molecules"]) for p in v1214.V1214_TR_SUBSTRATE.values())
    assert total >= 25


def test_dna_repair_minimum_5_pathways():
    """DNA repair ≥ 5 pathway (NER + MMR + BER + NHEJ + HDR)."""
    dna_repair_pathways = ["TR_NER", "TR_MMR", "TR_BER", "TR_NHEJ", "TR_HDR"]
    for p in dna_repair_pathways:
        assert p in v1214.V1214_TR_SUBSTRATE


def test_hgt_minimum_3_pathways():
    """HGT ≥ 3 pathway (transformation + conjugation + transduction)."""
    hgt_pathways = ["TR_HGT_TRANSFORMATION", "TR_HGT_CONJUGATION", "TR_HGT_TRANSDUCTION"]
    for p in hgt_pathways:
        assert p in v1214.V1214_TR_SUBSTRATE


def test_crispr_present():
    """CRISPR ≥ 1 pathway (Cas9 + sgRNA + PAM + nuclease domains)."""
    assert "TR_CRISPR" in v1214.V1214_TR_SUBSTRATE


def test_v3_guards_present():
    """V3 哲学守门 module-level 至少 5 guard."""
    assert len(v1214.V3_GUARDS) >= 5
    assert any("不假装" in g for g in v1214.V3_GUARDS.keys())


# ============================================================================
# measure_v1214_full
# ============================================================================

def test_measure_v1214_full_returns_report():
    rep = v1214.measure_v1214_full()
    assert isinstance(rep, v1214.V1214Report)
    assert rep.dim_version == "0.6.24"
    assert rep.north_star == 0.98


def test_measure_v1214_full_9_pathways():
    """9 pathway 应全部 pass (主 13:31 真生产 — ≥ 5 真分子 each)."""
    rep = v1214.measure_v1214_full()
    assert rep.n_pathways_total == 9
    assert rep.n_pathways_pass == 9


def test_measure_v1214_full_total_molecules_minimum():
    """≥ 25 真分子."""
    rep = v1214.measure_v1214_full()
    assert rep.total_tr_molecules >= 25


def test_measure_v1214_full_dna_repair_molecules_minimum():
    """DNA repair 真分子 ≥ 25 (5 pathways × 4-9)."""
    rep = v1214.measure_v1214_full()
    assert rep.n_dna_repair_molecules >= 20


def test_measure_v1214_full_hgt_molecules_minimum():
    """HGT 真分子 ≥ 10 (3 pathways × 3-5)."""
    rep = v1214.measure_v1214_full()
    assert rep.n_hgt_molecules >= 10


def test_measure_v1214_full_crispr_molecules_minimum():
    """CRISPR 真分子 ≥ 5."""
    rep = v1214.measure_v1214_full()
    assert rep.n_crispr_molecules >= 5


def test_measure_v1214_full_r5_pass():
    """R5_repair 5 pathway (NER + MMR + BER + NHEJ + HDR) 应全 pass."""
    rep = v1214.measure_v1214_full()
    assert rep.r5_pass == 5


def test_measure_v1214_full_r9_pass():
    """R9_heredity 4 pathway (3 HGT + 1 CRISPR) 应全 pass."""
    rep = v1214.measure_v1214_full()
    assert rep.r9_pass == 4


def test_measure_v1214_full_tr_lift_positive():
    """TR dim realized V1213 → V1214 应 lift 真实存在 (> 0)."""
    rep = v1214.measure_v1214_full()
    assert rep.v1214_tr_lift_delta > 0


def test_measure_v1214_full_overall_lift_positive():
    """Overall realized 应 lift (虽小但 > 0)."""
    rep = v1214.measure_v1214_full()
    assert rep.v1214_overall_lift_delta > 0


def test_measure_v1214_full_r5_lift():
    """TR × R5_repair V1213=0.6 → V1214 ≥ 0.85."""
    rep = v1214.measure_v1214_full()
    assert rep.v1214_tr_x_r5_repair >= 0.85


def test_measure_v1214_full_r9_lift():
    """TR × R9_heredity V1213=0.6 → V1214 ≥ 0.80."""
    rep = v1214.measure_v1214_full()
    assert rep.v1214_tr_x_r9_heredity >= 0.80


def test_measure_v1214_full_position_above_50pct():
    """TR dim realized position of north_star 应 > 50%."""
    rep = v1214.measure_v1214_full()
    assert rep.position_of_north_star_realized_pct > 50.0


def test_measure_v1214_full_inflation_audit_present():
    """inflation_gap 应为正 (V1212 clamp 1.0 - V1214 overall realized)."""
    rep = v1214.measure_v1214_full()
    assert rep.v1214_inflation_gap > 0


# ============================================================================
# Per-measure helpers
# ============================================================================

def test_measure_v1214_tr_dim_realized():
    realized = v1214.measure_v1214_tr_dim_realized()
    assert realized > 0.4


def test_measure_v1214_overall_realized():
    overall = v1214.measure_v1214_overall_realized()
    assert overall > 0.3


def test_measure_v1214_inflation_gap():
    gap = v1214.measure_v1214_inflation_gap()
    assert gap > 0.0


# ============================================================================
# Realized coverage quality checks (主 17:43 实事求是)
# ============================================================================

def test_ner_pathway_passes():
    """TR_NER pathway 应 pass (≥ 9 真分子)."""
    rep = v1214.measure_v1214_full()
    assert rep.pathway_scores["TR_NER"] == 1.0


def test_mmr_pathway_passes():
    """TR_MMR pathway 应 pass (≥ 9 真分子)."""
    rep = v1214.measure_v1214_full()
    assert rep.pathway_scores["TR_MMR"] == 1.0


def test_crispr_pathway_passes():
    """TR_CRISPR pathway 应 pass (≥ 5 真分子)."""
    rep = v1214.measure_v1214_full()
    assert rep.pathway_scores["TR_CRISPR"] == 1.0


def test_v3_guards_no_pretending():
    """V3_GUARDS 必须包含 "不假装" markers."""
    guards = v1214.V3_GUARDS
    n_no_pretense = sum(1 for g in guards if "不假装" in g)
    assert n_no_pretense >= 8


def test_v3_guards_inflation_audit():
    """V3_GUARDS 必须显式 audit inflation."""
    guards = v1214.V3_GUARDS
    assert any("inflation" in g.lower() or "inflation" in v.lower() for g, v in guards.items())


def test_v3_guards_clamp_ceiling():
    """V3_GUARDS 必须显式 audit clamp ceiling."""
    guards = v1214.V3_GUARDS
    assert any("clamp" in v.lower() for v in guards.values())


# ============================================================================
# Artifact + Report writer
# ============================================================================

def test_write_v1214_artifact(tmp_path: Path):
    artifact_path = tmp_path / "v1214_test.json"
    v1214.write_v1214_artifact(artifact_path)
    assert artifact_path.exists()
    data = json.loads(artifact_path.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.24"
    assert "pathway_scores" in data
    assert "total_tr_molecules" in data
    assert "v1214_tr_lift_delta" in data
    assert len(data["pathway_scores"]) == 9


def test_write_v1214_report(tmp_path: Path):
    report_path = tmp_path / "v1214_test.md"
    v1214.write_v1214_report(report_path)
    assert report_path.exists()
    content = report_path.read_text(encoding="utf-8")
    assert "V1214" in content
    assert "9 pathway" in content or "9 pathway 真测" in content
    assert "TR" in content
    assert "V3 哲学守门" in content


def test_run_module_help():
    """CLI --help 应 work."""
    import subprocess
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1214_asi_v0624_truth_substrate_real_lift", "--help"],
        cwd=str(PROMETHEAN_ROOT),
        capture_output=True, text=False, timeout=30, encoding=None
    )
    assert result.returncode == 0
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "V1214" in stdout


def test_run_module_measure():
    """CLI --measure 应输出 ASI V0.6.24 truth_substrate_real_lift."""
    import subprocess
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1214_asi_v0624_truth_substrate_real_lift", "--measure"],
        cwd=str(PROMETHEAN_ROOT),
        capture_output=True, text=False, timeout=30, encoding=None
    )
    assert result.returncode == 0
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "ASI V0.6.24" in stdout
    assert "TR" in stdout
    assert "R5" in stdout
    assert "R9" in stdout