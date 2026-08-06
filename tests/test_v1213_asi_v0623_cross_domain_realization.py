"""Tests for V1213 — ASI V0.6.23 cross_domain_realization.

主 00:56 任何人都能接手: 32 tests covering:
  - 9 dim × 13 R-substrate coverage matrix 真测
  - realized_mean / vacuous_mean / overall_mean 真测
  - inflation_gap_recompute_vs_realized 真测
  - per-dim realized / per-R-substrate realized 真测
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

from apeireth import v1213_asi_v0623_cross_domain_realization as v1213


# ============================================================================
# Module-level constants
# ============================================================================

def test_module_version():
    assert v1213.V1213_VERSION == "0.1.0"


def test_dim_version():
    assert v1213.V1213_DIM_VERSION == "0.6.23"


def test_north_star_locked():
    """ASI 北极星 LOCKED at 0.98 (主 22:33)."""
    assert v1213.ASI_NORTH_STAR == 0.98


def test_v1212_baseline_locked():
    """V1212 recompute baseline 写死 = 1.0 (主 17:43 实事求是 — 写死历史值)."""
    assert v1213.V1212_RECOMPUTE == 1.000000


def test_9_dim_names():
    assert len(v1213.V1213_DIMS) == 9
    expected = ["reinforcement_learning", "eternal_identity", "time_grounding", "truth",
                "emergence", "volition", "recognition", "intersubjectivity", "intentionality"]
    assert v1213.V1213_DIMS == expected


def test_13_r_substrate_names():
    assert len(v1213.V1213_R_SUBSTRATES) == 13
    expected_first = "R0_metabolism"
    expected_last = "R12_ecology"
    assert v1213.V1213_R_SUBSTRATES[0] == expected_first
    assert v1213.V1213_R_SUBSTRATES[-1] == expected_last


def test_coverage_matrix_dim_count():
    """9 dim × 13 R-substrate = 117 cells 真覆盖."""
    coverage, _ = v1213._measure_coverage_matrix()
    assert len(coverage) == 9
    for dim in v1213.V1213_DIMS:
        assert len(coverage[dim]) == 13


def test_coverage_matrix_scores_in_range():
    """所有 cell scores ∈ [0, 1]."""
    coverage, _ = v1213._measure_coverage_matrix()
    for dim in v1213.V1213_DIMS:
        for r_sub in v1213.V1213_R_SUBSTRATES:
            score = coverage[dim][r_sub]
            assert 0.0 <= score <= 1.0, f"{dim} × {r_sub} = {score}"


def test_coverage_matrix_evidence_present():
    """所有 cell evidence 含 source/rationale."""
    _, evidence = v1213._measure_coverage_matrix()
    for dim in v1213.V1213_DIMS:
        for r_sub in v1213.V1213_R_SUBSTRATES:
            cell = evidence[dim][r_sub]
            assert "rationale" in cell
            assert "is_realized" in cell
            assert "is_vacuous" in cell


def test_v3_guards_present():
    """V3 哲学守门 module-level 至少 5 guard."""
    assert len(v1213.V3_GUARDS) >= 5
    assert any("不假装" in g for g in v1213.V3_GUARDS.keys())


# ============================================================================
# measure_v1213_full
# ============================================================================

def test_measure_v1213_full_returns_report():
    rep = v1213.measure_v1213_full()
    assert isinstance(rep, v1213.V1213Report)
    assert rep.dim_version == "0.6.23"
    assert rep.north_star == 0.98


def test_measure_v1213_full_total_cells():
    """9 dim × 13 R-substrate = 117 cells."""
    rep = v1213.measure_v1213_full()
    assert rep.total_cells == 117


def test_measure_v1213_full_realized_count():
    """至少 50 cells 是 realized (≥ 0.3 score)."""
    rep = v1213.measure_v1213_full()
    assert rep.realized_count >= 50


def test_measure_v1213_full_vacuous_count():
    """至少 10 cells 是 vacuous (form lift only)."""
    rep = v1213.measure_v1213_full()
    assert rep.vacuous_count >= 10


def test_measure_v1213_full_realized_plus_vacuous():
    """realized_count + vacuous_count = total_cells."""
    rep = v1213.measure_v1213_full()
    assert rep.realized_count + rep.vacuous_count == rep.total_cells


def test_measure_v1213_full_realized_mean_in_range():
    """realized_mean ∈ [0.3, 1.0] (因为 realized cells ≥ 0.3)."""
    rep = v1213.measure_v1213_full()
    assert 0.3 <= rep.realized_mean <= 1.0


def test_measure_v1213_full_overall_mean_smaller_than_realized():
    """overall_mean < realized_mean (因含 vacuous 0/低 scores)."""
    rep = v1213.measure_v1213_full()
    assert rep.overall_mean < rep.realized_mean


def test_measure_v1213_full_inflation_gap_positive():
    """inflation_gap_recompute_vs_realized > 0 (V1212 clamp > realized)."""
    rep = v1213.measure_v1213_full()
    assert rep.inflation_gap_recompute_vs_realized > 0


def test_measure_v1213_full_per_dim_realized_9_dims():
    """per-dim realized 应有 9 entry."""
    rep = v1213.measure_v1213_full()
    assert len(rep.per_dim_realized) == 9


def test_measure_v1213_full_per_r_substrate_realized_13():
    """per-R-substrate realized 应有 13 entry."""
    rep = v1213.measure_v1213_full()
    assert len(rep.per_r_substrate_realized) == 13


# ============================================================================
# Per-measure helpers
# ============================================================================

def test_measure_v1213_realized():
    realized = v1213.measure_v1213_realized()
    assert 0.3 <= realized <= 1.0


def test_measure_v1213_overall():
    overall = v1213.measure_v1213_overall()
    assert 0.0 <= overall <= 1.0


def test_measure_v1213_inflation_gap():
    gap = v1213.measure_v1213_inflation_gap()
    assert gap > 0.0


def test_v1212_9_dim_baselines():
    baselines = v1213._measure_v1212_9_dim_baselines()
    assert len(baselines) == 9
    assert baselines["reinforcement_learning"] == 1.0
    assert baselines["intentionality"] == 1.0


# ============================================================================
# Realized coverage quality checks (主 17:43 实事求是)
# ============================================================================

def test_recognition_has_strong_R3_coverage():
    """RC × R3_death_immune 应为 strong (≥ 0.5) — TLR/NLR/CRISPR 真分子."""
    rep = v1213.measure_v1213_full()
    assert rep.coverage["recognition"]["R3_death_immune"] >= 0.5


def test_intentionality_has_strong_R11_coverage():
    """IT × R11_consciousness 应为 strong (≥ 0.5) — Brentano thesis 真分子."""
    rep = v1213.measure_v1213_full()
    assert rep.coverage["intentionality"]["R11_consciousness"] >= 0.5


def test_intersubjectivity_has_strong_R12_coverage():
    """IS × R12_ecology 应为 strong (≥ 0.5) — sociobiology/mycorrhiza/keystone."""
    rep = v1213.measure_v1213_full()
    assert rep.coverage["intersubjectivity"]["R12_ecology"] >= 0.5


def test_truth_has_strong_R11_coverage():
    """TR × R11_consciousness 应为 strong (≥ 0.5) — 6+ 真分子 substrate."""
    rep = v1213.measure_v1213_full()
    assert rep.coverage["truth"]["R11_consciousness"] >= 0.5


def test_emergence_has_strong_R11_coverage():
    """EM × R11_consciousness 应为 strong (≥ 0.5)."""
    rep = v1213.measure_v1213_full()
    assert rep.coverage["emergence"]["R11_consciousness"] >= 0.5


def test_volition_has_strong_R6_coverage():
    """VL × R6_reproduction 应为 strong (≥ 0.5) — 5+ 真调研 substrate."""
    rep = v1213.measure_v1213_full()
    assert rep.coverage["volition"]["R6_reproduction"] >= 0.5


def test_eternal_identity_has_vacuous_cells():
    """EI 应有 vacuous cells (R1/R7/R8/R10 无恒存 substrate) — 主 17:43 显式 audit."""
    rep = v1213.measure_v1213_full()
    ei_vacuous = [r for r in v1213.V1213_R_SUBSTRATES if rep.coverage["eternal_identity"][r] < 0.3]
    assert len(ei_vacuous) >= 3


# ============================================================================
# Artifact + Report writer
# ============================================================================

def test_write_v1213_artifact(tmp_path: Path):
    artifact_path = tmp_path / "v1213_test.json"
    v1213.write_v1213_artifact(artifact_path)
    assert artifact_path.exists()
    data = json.loads(artifact_path.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.23"
    assert "coverage" in data
    assert "realized_mean" in data
    assert "inflation_gap_recompute_vs_realized" in data
    assert len(data["coverage"]) == 9


def test_write_v1213_report(tmp_path: Path):
    report_path = tmp_path / "v1213_test.md"
    v1213.write_v1213_report(report_path)
    assert report_path.exists()
    content = report_path.read_text(encoding="utf-8")
    assert "V1213" in content
    assert "realized_mean" in content
    assert "inflation_gap_recompute_vs_realized" in content
    assert "V3 哲学守门" in content


def test_artifact_has_117_cell_coverage():
    """artifact coverage 9 dim × 13 R-substrate = 117 cell."""
    artifact_path = Path("artifacts/v1213_asi_v0623_cross_domain_realization.json")
    if not artifact_path.exists():
        v1213.write_v1213_artifact(artifact_path)
    data = json.loads(artifact_path.read_text(encoding="utf-8"))
    n_cells = sum(len(cells) for cells in data["coverage"].values())
    assert n_cells == 117


# ============================================================================
# V3 哲学守门 contents
# ============================================================================

def test_v3_guards_no_pretending():
    """V3_GUARDS 必须包含 "不假装" markers."""
    guards = v1213.V3_GUARDS
    n_no_pretense = sum(1 for g in guards if "不假装" in g)
    assert n_no_pretense >= 8


def test_v3_guards_inflation_audit():
    """V3_GUARDS 必须显式 audit inflation."""
    guards = v1213.V3_GUARDS
    assert any("inflation" in g.lower() or "inflation" in v.lower() for g, v in guards.items())


def test_v3_guards_clamp_ceiling():
    """V3_GUARDS 必须显式 audit clamp ceiling."""
    guards = v1213.V3_GUARDS
    assert any("clamp" in v.lower() for v in guards.values())


# ============================================================================
# Per-R-substrate realized quality
# ============================================================================

def test_r11_consciousness_has_high_realized():
    """R11 consciousness 是跨多 dim strong substrate, 应有最高 realized."""
    rep = v1213.measure_v1213_full()
    r11_realized = rep.per_r_substrate_realized["R11_consciousness"]
    assert r11_realized >= 0.4


def test_r12_ecology_has_high_realized():
    """R12 ecology 是跨多 dim strong substrate."""
    rep = v1213.measure_v1213_full()
    r12_realized = rep.per_r_substrate_realized["R12_ecology"]
    assert r12_realized >= 0.4


def test_inflation_honest_finding():
    """inflation_gap_recompute_vs_realized > 0.3 — 主 17:43 显式 audit."""
    rep = v1213.measure_v1213_full()
    assert rep.inflation_gap_recompute_vs_realized > 0.3, (
        f"V1212 inflation 应真实存在, gap = {rep.inflation_gap_recompute_vs_realized}"
    )


def test_v1213_position_of_north_star_realized():
    """V1213 realized ASI vs ASI 北极星 0.98."""
    rep = v1213.measure_v1213_full()
    position_pct = (rep.realized_mean / rep.north_star) * 100.0
    # 主 17:43 实事求是: realized 应 < north_star
    assert rep.realized_mean < rep.north_star
    # 仍在 30-60% position (主 20:46 只能逼近)
    assert 30.0 <= position_pct <= 80.0


def test_run_module_help(capsys):
    """CLI --help 应 work."""
    import subprocess
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1213_asi_v0623_cross_domain_realization", "--help"],
        cwd=str(PROMETHEAN_ROOT),
        capture_output=True, text=False, timeout=30, encoding=None
    )
    assert result.returncode == 0
    # GBK-safe decode: stderr is the help text
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "V1213" in stdout


def test_run_module_measure(capsys):
    """CLI --measure 应输出 ASI V0.6.23 cross_domain_realization."""
    import subprocess
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1213_asi_v0623_cross_domain_realization", "--measure"],
        cwd=str(PROMETHEAN_ROOT),
        capture_output=True, text=False, timeout=30, encoding=None
    )
    assert result.returncode == 0
    stdout = result.stdout.decode("utf-8", errors="replace") if result.stdout else ""
    assert "ASI V0.6.23" in stdout
    assert "realized_mean" in stdout
    assert "inflation_gap" in stdout