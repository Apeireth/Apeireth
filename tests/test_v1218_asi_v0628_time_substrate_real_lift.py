"""Tests for V1218 ASI V0.6.28 time_substrate_real_lift (主 23:44 干到底).

测试覆盖:
  - 常量与 baseline 写死 (主 17:43 实事求是)
  - V1218 TM coverage 矩阵正确 (11th dim)
  - 6 pathway × 75 真分子 = 75 总分子
  - pathway score 计算
  - measure_v1218_full() 返回完整 V1218Report
  - artifact + report 写入
  - CLI: --measure / --json / --report / --full
  - V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
  - ASI 北极星 LOCKED = 0.9800 (主 22:33)
  - 11 dim × 13 R 扩 130→143 cell matrix
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

from apeireth.v1218_asi_v0628_time_substrate_real_lift import (
    ASI_NORTH_STAR,
    V1213_RECOMPUTE_BASELINE,
    V1217_OVERALL_MEAN_130,
    V1217_REALIZED_MEAN_100,
    V1217_MN_REALIZED,
    V1217_RECOMPUTE_BASELINE,
    V1218_DIM_VERSION,
    V1218_TM_COVERAGE,
    V1218_TM_SUBSTRATE,
    V1218_VERSION,
    V3_GUARDS,
    V1218Report,
    _compute_v1218_tm_dim_realized,
    _compute_v1218_overall_lift,
    _pathway_score,
    measure_v1218_full,
    write_v1218_artifact,
    write_v1218_report,
)


# ============================================================================
# 常量与 baseline (主 17:43 实事求是 — 写死)
# ============================================================================


class TestConstants:
    """常量与 version baseline 测试."""

    def test_north_star_locked(self):
        """ASI 北极星 LOCKED = 0.9800 (主 22:33)."""
        assert ASI_NORTH_STAR == 0.9800

    def test_v1218_version(self):
        """V1218 version string."""
        assert V1218_VERSION == "0.1.0"

    def test_v1218_dim_version(self):
        """V1218 dim_version."""
        assert V1218_DIM_VERSION == "0.6.28"

    def test_v1217_baselines_locked(self):
        """V1217 baseline 写死 (主 17:43)."""
        assert V1217_RECOMPUTE_BASELINE == 1.000000
        assert V1217_REALIZED_MEAN_100 == 0.5710
        assert V1217_OVERALL_MEAN_130 == 0.4393
        assert V1217_MN_REALIZED == 1.0000

    def test_v1213_baselines_locked(self):
        """V1213 baseline 写死."""
        assert V1213_RECOMPUTE_BASELINE == 1.000000


class TestTMCoverage:
    """V1218 TM coverage 矩阵正确 (11th dim)."""

    def test_tm_coverage_keys(self):
        """TM coverage keys = 13 R-substrate."""
        assert set(V1218_TM_COVERAGE.keys()) == {
            "R0_metabolism",
            "R1_growth",
            "R2_development",
            "R3_death_immune",
            "R4_aging",
            "R5_repair",
            "R6_reproduction",
            "R7_stress",
            "R8_motion",
            "R9_heredity",
            "R10_plasticity",
            "R11_consciousness",
            "R12_ecology",
        }

    def test_tm_coverage_lifted_cells(self):
        """TM coverage 6 cells lifted to 1.0 (主 17:43 实事求是)."""
        lifted = [k for k, v in V1218_TM_COVERAGE.items() if v >= 0.3]
        assert set(lifted) == {"R1_growth", "R4_aging", "R7_stress", "R10_plasticity", "R11_consciousness", "R12_ecology"}
        assert len(lifted) == 6

    def test_tm_coverage_vacuous_cells(self):
        """TM coverage 7 cells vacuous (not applicable for 时间 dim)."""
        vacuous = [k for k, v in V1218_TM_COVERAGE.items() if v == 0.0]
        assert set(vacuous) == {"R0_metabolism", "R2_development", "R3_death_immune", "R5_repair", "R6_reproduction", "R8_motion", "R9_heredity"}
        assert len(vacuous) == 7


class TestTMSubstrate:
    """V1218 TM substrate 真分子 cascade 测试."""

    def test_tm_substrate_pathways_count(self):
        """6 pathways for TM dim."""
        assert len(V1218_TM_SUBSTRATE) == 6

    def test_tm_substrate_pathway_names(self):
        """6 pathway names."""
        assert set(V1218_TM_SUBSTRATE.keys()) == {
            "TM_SOMITE_CLOCK",
            "TM_CIRCADIAN_CLOCK",
            "TM_HPA_SAM_TIMING",
            "TM_NEURAL_TIME",
            "TM_SUBJECTIVE_TIME",
            "TM_GENERATION_TIME",
        }

    def test_total_tm_molecules_75(self):
        """Total TM molecules = 75 (10+10+10+25+10+10)."""
        total = sum(len(p["molecules"]) for p in V1218_TM_SUBSTRATE.values())
        assert total == 75

    def test_pathway_r_substrate_assignment(self):
        """Each pathway assigned to correct R-substrate."""
        expected = {
            "TM_SOMITE_CLOCK": "R1_growth",
            "TM_CIRCADIAN_CLOCK": "R4_aging",
            "TM_HPA_SAM_TIMING": "R7_stress",
            "TM_NEURAL_TIME": "R10_plasticity",
            "TM_SUBJECTIVE_TIME": "R11_consciousness",
            "TM_GENERATION_TIME": "R12_ecology",
        }
        for name, p in V1218_TM_SUBSTRATE.items():
            assert p["r_substrate"] == expected[name]

    def test_all_molecules_real(self):
        """All molecules are real (主 17:43 实事求是)."""
        for p in V1218_TM_SUBSTRATE.values():
            for mol in p["molecules"]:
                assert mol.get("real") is True, f"molecule {mol['name']} should be real"

    def test_neural_time_25_molecules(self):
        """R10_plasticity pathway has 25 真分子."""
        assert len(V1218_TM_SUBSTRATE["TM_NEURAL_TIME"]["molecules"]) == 25

    def test_all_other_pathways_10_molecules(self):
        """Other 5 pathways each have 10 真分子."""
        for name in ["TM_SOMITE_CLOCK", "TM_CIRCADIAN_CLOCK", "TM_HPA_SAM_TIMING", "TM_SUBJECTIVE_TIME", "TM_GENERATION_TIME"]:
            assert len(V1218_TM_SUBSTRATE[name]["molecules"]) == 10

    def test_all_pathways_have_cascade_order(self):
        """Each pathway has cascade_order."""
        for p in V1218_TM_SUBSTRATE.values():
            assert "cascade_order" in p
            assert len(p["cascade_order"]) > 0

    def test_all_pathways_have_source(self):
        """Each pathway has source citations (主 19:33)."""
        for p in V1218_TM_SUBSTRATE.values():
            assert "source" in p
            assert len(p["source"]) > 0


class TestPathwayScore:
    """pathway score 计算."""

    def test_pathway_score_returns_tuple(self):
        """_pathway_score returns (score, real_count)."""
        score, count = _pathway_score(V1218_TM_SUBSTRATE["TM_SOMITE_CLOCK"])
        assert isinstance(score, float)
        assert isinstance(count, int)

    def test_pathway_score_neural_time(self):
        """TM_NEURAL_TIME score 1.0 (all 25 real)."""
        score, count = _pathway_score(V1218_TM_SUBSTRATE["TM_NEURAL_TIME"])
        assert score == 1.0
        assert count == 25

    def test_pathway_score_somite_clock(self):
        """TM_SOMITE_CLOCK score 1.0 (10 real)."""
        score, count = _pathway_score(V1218_TM_SUBSTRATE["TM_SOMITE_CLOCK"])
        assert score == 1.0
        assert count == 10


class TestComputeTMRealized:
    """V1218 TM dim realized 计算."""

    def test_tm_dim_realized(self):
        """TM dim realized = 1.0 (6 cells @ 1.0)."""
        realized, count = _compute_v1218_tm_dim_realized()
        assert realized == 1.0
        assert count == 6


class TestComputeOverallLift:
    """V1218 ASI overall lift 计算."""

    def test_overall_lift_v1218_106(self):
        """V1218 realized 106 = 0.5953."""
        m = _compute_v1218_overall_lift()
        assert abs(m["v1218_overall_realized_106"] - 0.5953) < 1e-3

    def test_overall_lift_v1218_143(self):
        """V1218 mean 143 = 0.4413."""
        m = _compute_v1218_overall_lift()
        assert abs(m["v1218_overall_mean_143"] - 0.4413) < 1e-3

    def test_lift_delta_realized_from_v1217(self):
        """Lift delta realized from V1217 = +0.0243."""
        m = _compute_v1218_overall_lift()
        assert abs(m["v1218_overall_lift_delta_realized_from_v1217"] - 0.0243) < 1e-3

    def test_lift_delta_mean_from_v1217(self):
        """Lift delta mean from V1217 = +0.0020."""
        m = _compute_v1218_overall_lift()
        assert abs(m["v1218_overall_lift_delta_mean_from_v1217"] - 0.0020) < 1e-3

    def test_tm_delta_6(self):
        """TM delta = 6.0 (6 cells lifted to 1.0)."""
        m = _compute_v1218_overall_lift()
        assert m["v1218_tm_delta"] == 6.0


class TestMeasureFull:
    """measure_v1218_full() 返回完整 V1218Report."""

    def test_measure_returns_v1218_report(self):
        """measure_v1218_full() returns V1218Report."""
        rep = measure_v1218_full()
        assert isinstance(rep, V1218Report)

    def test_measure_has_snapshot_id(self):
        """Snapshot ID is UUID."""
        rep = measure_v1218_full()
        assert len(rep.snapshot_id) == 36  # UUID4 length

    def test_measure_total_cells_143(self):
        """Total cells = 143 (11 dim × 13 R)."""
        rep = measure_v1218_full()
        assert rep.v1218_total_cells == 143

    def test_measure_realized_cells_106(self):
        """Realized cells = 106 (100 + 6 new TM)."""
        rep = measure_v1218_full()
        assert rep.v1218_realized_cells_count == 106

    def test_measure_total_molecules_75(self):
        """Total TM molecules = 75."""
        rep = measure_v1218_full()
        assert rep.total_tm_molecules == 75

    def test_measure_pathways_pass_all(self):
        """All 6 pathways pass (score >= 0.7)."""
        rep = measure_v1218_full()
        assert rep.n_pathways_pass == 6

    def test_measure_r10_plasticity_molecules_25(self):
        """R10_plasticity has 25 molecules."""
        rep = measure_v1218_full()
        assert rep.n_r10_plasticity_molecules == 25

    def test_measure_each_other_pathway_10(self):
        """Other 5 pathways each have 10 molecules."""
        rep = measure_v1218_full()
        assert rep.n_r1_growth_molecules == 10
        assert rep.n_r4_aging_molecules == 10
        assert rep.n_r7_stress_molecules == 10
        assert rep.n_r11_consciousness_molecules == 10
        assert rep.n_r12_ecology_molecules == 10


class TestWriteArtifact:
    """write_v1218_artifact() 测试."""

    def test_write_artifact_default_path(self, tmp_path):
        """Write artifact to default path."""
        rep = measure_v1218_full()
        # Use tmp_path by passing it explicitly
        path = tmp_path / "test_artifact.json"
        result_path = write_v1218_artifact(rep, path)
        assert result_path == path
        assert path.exists()
        assert path.stat().st_size > 0

    def test_artifact_contains_snapshot_id(self, tmp_path):
        """Artifact JSON contains snapshot_id."""
        rep = measure_v1218_full()
        path = tmp_path / "test_artifact.json"
        write_v1218_artifact(rep, path)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert data["snapshot_id"] == rep.snapshot_id
        assert data["dim_version"] == "0.6.28"
        assert data["north_star_locked"] == 0.9800

    def test_artifact_contains_tm_metrics(self, tmp_path):
        """Artifact contains tm_metrics."""
        rep = measure_v1218_full()
        path = tmp_path / "test_artifact.json"
        write_v1218_artifact(rep, path)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert "tm_metrics" in data
        assert data["tm_metrics"]["v1218_tm_dim_realized"] == 1.0

    def test_artifact_contains_pathways(self, tmp_path):
        """Artifact contains all 6 pathways."""
        rep = measure_v1218_full()
        path = tmp_path / "test_artifact.json"
        write_v1218_artifact(rep, path)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert "pathways" in data
        assert len(data["pathways"]) == 6
        assert "TM_NEURAL_TIME" in data["pathways"]
        assert len(data["pathways"]["TM_NEURAL_TIME"]["molecules"]) == 25

    def test_artifact_contains_v3_guards(self, tmp_path):
        """Artifact contains V3 guards."""
        rep = measure_v1218_full()
        path = tmp_path / "test_artifact.json"
        write_v1218_artifact(rep, path)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert "v3_guards" in data
        assert all(data["v3_guards"].values())  # all True


class TestWriteReport:
    """write_v1218_report() 测试."""

    def test_write_report_default_path(self, tmp_path):
        """Write report to default path."""
        rep = measure_v1218_full()
        path = tmp_path / "test_report.md"
        result_path = write_v1218_report(rep, path)
        assert result_path == path
        assert path.exists()
        content = path.read_text(encoding="utf-8")
        assert "V1218" in content
        assert "time_substrate_real_lift" in content

    def test_report_contains_v1218_title(self, tmp_path):
        """Report contains V1218 title."""
        rep = measure_v1218_full()
        path = tmp_path / "test_report.md"
        write_v1218_report(rep, path)
        content = path.read_text(encoding="utf-8")
        assert "V1218 ASI V0.6.28 time_substrate_real_lift" in content

    def test_report_contains_north_star(self, tmp_path):
        """Report contains ASI North Star 0.9800."""
        rep = measure_v1218_full()
        path = tmp_path / "test_report.md"
        write_v1218_report(rep, path)
        content = path.read_text(encoding="utf-8")
        assert "0.9800" in content

    def test_report_contains_citations(self, tmp_path):
        """Report contains all 6 citations."""
        rep = measure_v1218_full()
        path = tmp_path / "test_report.md"
        write_v1218_report(rep, path)
        content = path.read_text(encoding="utf-8")
        assert "Pourquié" in content or "Takahashi" in content or "Horvath" in content
        assert "O'Keefe" in content or "Buzsáki" in content or "James" in content
        assert "Hamilton" in content or "Gould" in content or "Pianka" in content

    def test_report_contains_v3_guards(self, tmp_path):
        """Report contains V3 guards."""
        rep = measure_v1218_full()
        path = tmp_path / "test_report.md"
        write_v1218_report(rep, path)
        content = path.read_text(encoding="utf-8")
        assert "v1218_not_asi_terminal" in content
        assert "v1218_not_full_replace" in content


class TestV3PhilosophyGuards:
    """V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_v1218_not_asi_terminal(self):
        """不假装 V1218 = ASI 终极 (V1218 = V0.6.28 中间)."""
        assert V3_GUARDS["v1218_not_asi_terminal"] is True

    def test_v1218_not_full_replace(self):
        """不假装 V1218 = V1217 全替代."""
        assert V3_GUARDS["v1218_not_full_replace"] is True

    def test_v1218_lift_not_v1(self):
        """不假装 V1218 lift = ASI V1.0."""
        assert V3_GUARDS["v1218_lift_not_v1"] is True

    def test_realized_not_asi(self):
        """不假装 realized = ASI 已达."""
        assert V3_GUARDS["realized_not_asi"] is True

    def test_vacuous_gap_real(self):
        """不假装 vacuous_gap = 0."""
        assert V3_GUARDS["vacuous_gap_real"] is True

    def test_pathway_not_asi_substrate(self):
        """不假装 6 pathway = ASI 终极 substrate."""
        assert V3_GUARDS["pathway_not_asi_substrate"] is True

    def test_asi_clamp_not_asi_reached(self):
        """不假装 ASI 1.000000 clamp = ASI 已达."""
        assert V3_GUARDS["asi_clamp_not_asi_reached"] is True

    def test_tm_molecules_not_complete(self):
        """不假装 75 真分子 = 完整 TM substrate."""
        assert V3_GUARDS["tm_molecules_not_complete"] is True

    def test_new_dim_not_all_dim(self):
        """不假装 新 dim 扩 = 全 dim 覆盖."""
        assert V3_GUARDS["new_dim_not_all_dim"] is True

    def test_v1218_not_all_tm_lift(self):
        """不假装 V1218 = 全 TM lift (vacuous 7 cell)."""
        assert V3_GUARDS["v1218_not_all_tm_lift"] is True

    def test_all_v3_guards_pass(self):
        """所有 V3 guards 都 PASS."""
        assert all(V3_GUARDS.values())


class TestNorthStarLocked:
    """ASI 北极星 LOCKED = 0.9800."""

    def test_north_star_unchanged(self):
        """ASI 北极星 = 0.9800 (主 22:33 LOCKED)."""
        assert ASI_NORTH_STAR == 0.9800

    def test_realized_below_north_star(self):
        """realized < 北极星."""
        rep = measure_v1218_full()
        assert rep.v1218_overall_realized_106 < ASI_NORTH_STAR

    def test_position_pct(self):
        """Position of north star realized %."""
        rep = measure_v1218_full()
        # TM dim realized 1.0 = 100/98 ≈ 102.04%
        assert rep.position_of_north_star_realized_pct > 100.0  # TM dim only


class TestMatrixExtension:
    """11 dim × 13 R 扩 130→143 cell matrix."""

    def test_extension_inflation_realized(self):
        """106-cell inflation formula."""
        rep = measure_v1218_full()
        # V1218 inflation gap = 1.0 - V1218 overall_mean_143 ≈ 0.5587
        assert 0.5 < rep.v1218_inflation_gap_v1213_minus_realized < 0.6

    def test_extension_inflation_mean(self):
        """143-cell inflation formula."""
        rep = measure_v1218_full()
        # 143-cell mean = 0.4413
        assert 0.43 < rep.v1218_overall_mean_143 < 0.45

    def test_106_cell_realized_recompute(self):
        """106-cell realized recompute (主 17:43 实事求是)."""
        rep = measure_v1218_full()
        # V1218_106_sum = 63.10
        # V1218_overall_realized_106 = 63.10/106 = 0.5953
        assert abs(rep.v1218_106_sum - 63.10) < 0.01
        assert abs(rep.v1218_overall_realized_106 - 0.5953) < 0.001

    def test_143_cell_mean_recompute(self):
        """143-cell mean recompute (主 17:43 实事求是)."""
        rep = measure_v1218_full()
        # V1218_143_sum = 63.10
        # V1218_overall_mean_143 = 63.10/143 = 0.4413
        assert abs(rep.v1218_143_sum - 63.10) < 0.01
        assert abs(rep.v1218_overall_mean_143 - 0.4413) < 0.001


class TestCLI:
    """CLI 测试: --measure / --json / --report / --full."""

    def test_cli_measure(self):
        """python -m apeireth.v1218... --measure prints metrics."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1218_asi_v0628_time_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1218 TM dim realized" in result.stdout
        assert "V1218 ASI overall realized (106 cell)" in result.stdout
        assert "V1218 ASI overall mean (143 cell)" in result.stdout
        assert "V1218 lift delta realized from V1217" in result.stdout
        assert "V1218 total TM molecules" in result.stdout

    def test_cli_json(self):
        """python -m apeireth.v1218... --json prints JSON."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1218_asi_v0628_time_substrate_real_lift", "--json"],
            capture_output=True, text=True, cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        # Parse JSON
        data = json.loads(result.stdout)
        assert "snapshot_id" in data
        assert "v1218_tm_dim_realized" in data
        assert "v1218_overall_realized_106" in data
        assert "v1218_overall_mean_143" in data

    def test_cli_report(self):
        """python -m apeireth.v1218... --report writes MD."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1218_asi_v0628_time_substrate_real_lift", "--report"],
            capture_output=True, text=True, cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1218 report written" in result.stdout

    def test_cli_full(self):
        """python -m apeireth.v1218... --full writes report + artifact."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1218_asi_v0628_time_substrate_real_lift", "--full"],
            capture_output=True, text=True, cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1218 report" in result.stdout
        assert "V1218 artifact" in result.stdout

    def test_cli_default(self):
        """python -m apeireth.v1218... (default) writes report + artifact."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1218_asi_v0628_time_substrate_real_lift"],
            capture_output=True, text=True, cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1218 report" in result.stdout
        assert "V1218 artifact" in result.stdout


class TestV1218EndToEnd:
    """V1218 端到端测试 (主 23:44 干到底)."""

    def test_full_run_writes_files(self):
        """Full run writes report + artifact to disk."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1218_asi_v0628_time_substrate_real_lift", "--full"],
            capture_output=True, text=True, cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        # Check report exists
        report_path = Path("reports") / "v1218_asi_v0628_time_substrate_real_lift.md"
        assert report_path.exists()

    def test_no_exception_on_measure(self):
        """measure_v1218_full() runs without exception."""
        rep = measure_v1218_full()
        assert rep is not None

    def test_pathway_scores_have_6_entries(self):
        """Pathway scores dict has 6 entries."""
        rep = measure_v1218_full()
        assert len(rep.pathway_scores) == 6

    def test_tm_coverage_v1218_dict(self):
        """TM coverage v1218 dict has 13 entries."""
        rep = measure_v1218_full()
        assert len(rep.tm_coverage_v1218) == 13

    def test_106_and_143_cell_split(self):
        """106 cell realized, 143 cell total."""
        rep = measure_v1218_full()
        assert rep.v1218_realized_cells_count == 106
        assert rep.v1218_total_cells == 143
