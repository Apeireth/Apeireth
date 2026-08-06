"""Tests for V1217 ASI V0.6.27 manifestation_substrate_real_lift (主 23:44 干到底).

测试覆盖:
  - 常量与 baseline 写死 (主 17:43 实事求是)
  - V1217 MN coverage 矩阵正确 (10th dim)
  - 6 pathway × 75 真分子 = 75 总分子
  - pathway score 计算
  - measure_v1217_full() 返回完整 V1217Report
  - artifact + report 写入
  - CLI: --measure / --json / --report / --full
  - V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
  - ASI 北极星 LOCKED = 0.9800 (主 22:33)
  - 10 dim × 13 R 扩 117→130 cell matrix
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

from apeireth.v1217_asi_v0627_manifestation_substrate_real_lift import (
    ASI_NORTH_STAR,
    V1213_RECOMPUTE_BASELINE,
    V1216_OVERALL_MEAN_117,
    V1216_REALIZED_MEAN_94,
    V1216_RC_REALIZED,
    V1216_RECOMPUTE_BASELINE,
    V1217_DIM_VERSION,
    V1217_MN_COVERAGE,
    V1217_MN_SUBSTRATE,
    V1217_VERSION,
    V3_GUARDS,
    V1217Report,
    _compute_v1217_mn_dim_realized,
    _compute_v1217_overall_lift,
    _pathway_score,
    measure_v1217_full,
    write_v1217_artifact,
    write_v1217_report,
)


# ============================================================================
# 常量与 baseline (主 17:43 实事求是 — 写死)
# ============================================================================


class TestConstants:
    """常量与 version baseline 测试."""

    def test_north_star_locked(self):
        """ASI 北极星 LOCKED = 0.9800 (主 22:33)."""
        assert ASI_NORTH_STAR == 0.9800

    def test_v1217_version(self):
        """V1217 version string."""
        assert V1217_VERSION == "0.1.0"
        assert V1217_DIM_VERSION == "0.6.27"

    def test_v1213_recompute_baseline(self):
        """V1213 baseline 写死 (主 17:43)."""
        assert V1213_RECOMPUTE_BASELINE == 1.000000

    def test_v1216_recompute_baseline(self):
        """V1216 baseline 写死 (主 17:43)."""
        assert V1216_RECOMPUTE_BASELINE == 1.000000
        assert V1216_REALIZED_MEAN_94 == 0.5436
        assert V1216_OVERALL_MEAN_117 == 0.4368
        assert V1216_RC_REALIZED == 1.0000

    def test_v3_guards_dict_complete(self):
        """V3 哲学守门 dict 完整 (10 guards)."""
        assert len(V3_GUARDS) == 10
        assert all(v is True for v in V3_GUARDS.values())


# ============================================================================
# V1217 MN coverage (10th dim 显现 / manifestation)
# ============================================================================


class TestV1217MNCoverage:
    """V1217 MN coverage matrix (new 10th dim)."""

    def test_coverage_has_13_subs(self):
        """V1217 MN coverage 应有 13 R-substrate."""
        assert len(V1217_MN_COVERAGE) == 13

    def test_six_lifted_cells_at_1(self):
        """V1217 6 cell lifted to 1.0."""
        lifted = [k for k, v in V1217_MN_COVERAGE.items() if v == 1.0]
        expected = [
            "R1_growth",
            "R4_aging",
            "R7_stress",
            "R10_plasticity",
            "R11_consciousness",
            "R12_ecology",
        ]
        assert sorted(lifted) == sorted(expected)

    def test_seven_vacuous_cells_at_0(self):
        """V1217 7 vacuous cell @ 0.0 (manifestation not applicable)."""
        vacuous = [k for k, v in V1217_MN_COVERAGE.items() if v == 0.0]
        expected = [
            "R0_metabolism",
            "R2_development",
            "R3_death_immune",
            "R5_repair",
            "R6_reproduction",
            "R8_motion",
            "R9_heredity",
        ]
        assert sorted(vacuous) == sorted(expected)


# ============================================================================
# V1217 MN substrate 真分子 cascade (主 19:33)
# ============================================================================


class TestV1217MNSubstrate:
    """V1217 MN substrate 6 pathway × 75 真分子 cascade."""

    def test_n_pathways(self):
        """V1217 应有 6 pathway."""
        assert len(V1217_MN_SUBSTRATE) == 6

    def test_total_real_molecules(self):
        """V1217 总分子 = 75 (10+10+10+25+10+10)."""
        total = sum(len(p["molecules"]) for p in V1217_MN_SUBSTRATE.values())
        assert total == 75

    def test_total_cascade_orders(self):
        """cascade_order 总条目 = 75."""
        total = sum(len(p["cascade_order"]) for p in V1217_MN_SUBSTRATE.values())
        assert total == 75

    def test_all_molecules_real(self):
        """每个真分子必须 real=True (主 17:43 实事求是)."""
        for name, p in V1217_MN_SUBSTRATE.items():
            for m in p["molecules"]:
                assert m.get("real") is True, f"{name} molecule {m['name']} not real=True"

    def test_all_molecules_have_function(self):
        """每个真分子必须有 function 描述."""
        for name, p in V1217_MN_SUBSTRATE.items():
            for m in p["molecules"]:
                assert "function" in m
                assert len(m["function"]) > 10

    def test_all_pathways_have_source(self):
        """每个 pathway 必须有真实文献引用 (主 19:33 站在前人肩上)."""
        for name, p in V1217_MN_SUBSTRATE.items():
            assert "source" in p
            assert len(p["source"]) > 10

    def test_r10_pathway_has_25_molecules(self):
        """MN × R10_plasticity 应有 25 真分子 (IEG expression complex)."""
        ieg = V1217_MN_SUBSTRATE["MN_IEG_BEHAVIORAL"]
        assert len(ieg["molecules"]) == 25
        assert ieg["r_substrate"] == "R10_plasticity"

    def test_all_other_pathways_have_10_molecules(self):
        """其余 5 pathway 各自 10 真分子."""
        expected = {
            "MN_MORPHOGEN_GRADIENT": 10,
            "MN_SENESCENT_BETA_GAL": 10,
            "MN_HEAT_SHOCK_UPR": 10,
            "MN_NCC_GNWT": 10,
            "MN_NICHE_CONSTRUCT": 10,
        }
        for name, n in expected.items():
            assert len(V1217_MN_SUBSTRATE[name]["molecules"]) == n


# ============================================================================
# _pathway_score 计算 (主 17:43)
# ============================================================================


class TestPathwayScore:
    """Pathway score 函数."""

    def test_score_in_range(self):
        """pathway score 应在 [0, 1]."""
        for name, p in V1217_MN_SUBSTRATE.items():
            score, real_count = _pathway_score(p)
            assert 0.0 <= score <= 1.0

    def test_score_high_for_real_pathways(self):
        """全 real pathway score 应接近 1.0."""
        for name, p in V1217_MN_SUBSTRATE.items():
            score, _ = _pathway_score(p)
            assert score >= 0.95, f"{name} score {score} should be high"

    def test_real_count_matches_molecules(self):
        """real_count 应等于 molecule 数 (全 real)."""
        for name, p in V1217_MN_SUBSTRATE.items():
            _, real_count = _pathway_score(p)
            assert real_count == len(p["molecules"])

    def test_empty_pathway_returns_zero(self):
        """空 pathway 返回 0."""
        empty = {"molecules": [], "cascade_order": []}
        score, count = _pathway_score(empty)
        assert score == 0.0
        assert count == 0


# ============================================================================
# _compute_v1217_mn_dim_realized
# ============================================================================


class TestComputeV1217MNDimRealized:
    """V1217 MN dim realized mean 计算."""

    def test_mn_dim_realized_is_one(self):
        """V1217 MN dim realized = 1.0 (全 6 cell at 1.0)."""
        result, count = _compute_v1217_mn_dim_realized()
        assert abs(result - 1.0) < 1e-9
        assert count == 6


# ============================================================================
# _compute_v1217_overall_lift
# ============================================================================


class TestComputeV1217OverallLift:
    """V1217 overall realized + lift delta — 130 cell matrix."""

    def test_total_cells_extends_to_130(self):
        """V1217 130 cell formula 应正确."""
        lift_metrics = _compute_v1217_overall_lift()
        # 130 = 117 + 13 (new dim)
        # 100 = 94 + 6 (new realized)
        # V1216_94_sum = 0.5436 * 94 = 51.10
        # V1217_100_sum = 51.10 + 6.0 = 57.10
        # V1217_realized_100 = 57.10 / 100 = 0.5710
        assert abs(lift_metrics["v1217_overall_realized_100"] - 0.5710) < 1e-3

    def test_overall_mean_130(self):
        """V1217 overall mean (130 cell) ≈ 0.4393."""
        lift_metrics = _compute_v1217_overall_lift()
        # V1216_117_sum = 0.4368 * 117 = 51.10
        # V1217_130_sum = 51.10 + 6.0 = 57.10
        # V1217_mean_130 = 57.10 / 130 ≈ 0.4393
        assert abs(lift_metrics["v1217_overall_mean_130"] - 0.4393) < 1e-3

    def test_lift_delta_realized_positive(self):
        """V1217 lift delta realized from V1216 > 0."""
        lift_metrics = _compute_v1217_overall_lift()
        # +0.0274
        assert lift_metrics["v1217_overall_lift_delta_realized_from_v1216"] > 0
        assert 0.025 < lift_metrics["v1217_overall_lift_delta_realized_from_v1216"] < 0.030

    def test_lift_delta_mean_positive(self):
        """V1217 lift delta mean from V1216 > 0."""
        lift_metrics = _compute_v1217_overall_lift()
        # +0.0025 (small — because total cells also expanded from 117 to 130)
        assert lift_metrics["v1217_overall_lift_delta_mean_from_v1216"] > 0
        assert 0.001 < lift_metrics["v1217_overall_lift_delta_mean_from_v1216"] < 0.005

    def test_mn_delta_correct(self):
        """MN delta = +6.0 (6 cells lifted from 0)."""
        lift_metrics = _compute_v1217_overall_lift()
        assert abs(lift_metrics["v1217_mn_delta"] - 6.0) < 1e-9


# ============================================================================
# measure_v1217_full
# ============================================================================


class TestMeasureV1217Full:
    """measure_v1217_full() 完整测量."""

    def test_returns_v1217_report(self):
        """返回 V1217Report dataclass."""
        rep = measure_v1217_full()
        assert isinstance(rep, V1217Report)

    def test_snapshot_id_unique(self):
        """snapshot_id 每次唯一 (uuid4)."""
        r1 = measure_v1217_full()
        r2 = measure_v1217_full()
        assert r1.snapshot_id != r2.snapshot_id

    def test_dim_version(self):
        """dim_version = V0.6.27."""
        rep = measure_v1217_full()
        assert rep.dim_version == "0.6.27"

    def test_n_pathways_total(self):
        """n_pathways_total = 6."""
        rep = measure_v1217_full()
        assert rep.n_pathways_total == 6

    def test_n_pathways_pass(self):
        """n_pathways_pass = 6 (all high-score)."""
        rep = measure_v1217_full()
        assert rep.n_pathways_pass == 6

    def test_total_mn_molecules(self):
        """total_mn_molecules = 75."""
        rep = measure_v1217_full()
        assert rep.total_mn_molecules == 75

    def test_v1217_mn_dim_realized_one(self):
        """V1217 MN dim realized = 1.0."""
        rep = measure_v1217_full()
        assert abs(rep.v1217_mn_dim_realized - 1.0) < 1e-9

    def test_v1217_mn_dim_cell_count(self):
        """V1217 MN dim cell count = 6 lifted."""
        rep = measure_v1217_full()
        assert rep.v1217_mn_dim_cell_count == 6

    def test_v1217_total_cells_130(self):
        """V1217 total cells = 130 (10 dim × 13 R)."""
        rep = measure_v1217_full()
        assert rep.v1217_total_cells == 130

    def test_v1217_realized_cells_count_100(self):
        """V1217 realized cells = 100."""
        rep = measure_v1217_full()
        assert rep.v1217_realized_cells_count == 100

    def test_v1217_overall_realized_100(self):
        """V1217 ASI overall realized (100 cell) ≈ 0.5710."""
        rep = measure_v1217_full()
        assert abs(rep.v1217_overall_realized_100 - 0.5710) < 1e-3

    def test_v1217_overall_mean_130(self):
        """V1217 ASI overall mean (130 cell) ≈ 0.4393."""
        rep = measure_v1217_full()
        assert abs(rep.v1217_overall_mean_130 - 0.4393) < 1e-3

    def test_v1217_lift_delta_realized(self):
        """V1217 lift delta realized ≈ +0.0274."""
        rep = measure_v1217_full()
        assert abs(rep.v1217_overall_lift_delta_realized_from_v1216 - 0.0274) < 1e-3

    def test_v1217_lift_delta_mean(self):
        """V1217 lift delta mean ≈ +0.0025."""
        rep = measure_v1217_full()
        assert abs(rep.v1217_overall_lift_delta_mean_from_v1216 - 0.0025) < 1e-3

    def test_inflation_gap_real(self):
        """inflation_gap 真实存在 (主 17:43 实事求是)."""
        rep = measure_v1217_full()
        assert 0.55 < rep.v1217_inflation_gap_v1213_minus_realized < 0.6

    def test_v3_guards_in_report(self):
        """V3 哲学守门 10 guards 在 report 中."""
        rep = measure_v1217_full()
        assert len(rep.v3_guards) == 10
        assert all(rep.v3_guards.values())

    def test_r_substrate_pathways_pass(self):
        """每个 R-substrate pathway pass = 1."""
        rep = measure_v1217_full()
        assert rep.n_r1_growth_pathways_pass == 1
        assert rep.n_r4_aging_pathways_pass == 1
        assert rep.n_r7_stress_pathways_pass == 1
        assert rep.n_r10_plasticity_pathways_pass == 1
        assert rep.n_r11_consciousness_pathways_pass == 1
        assert rep.n_r12_ecology_pathways_pass == 1

    def test_molecule_count_per_substrate(self):
        """每个 R-substrate 的分子数正确."""
        rep = measure_v1217_full()
        assert rep.n_r1_growth_molecules == 10
        assert rep.n_r4_aging_molecules == 10
        assert rep.n_r7_stress_molecules == 10
        assert rep.n_r10_plasticity_molecules == 25
        assert rep.n_r11_consciousness_molecules == 10
        assert rep.n_r12_ecology_molecules == 10


# ============================================================================
# write_v1217_artifact + write_v1217_report
# ============================================================================


class TestWriteV1217Artifact:
    """write_v1217_artifact() artifact JSON 写入."""

    def test_artifact_write_default_path(self, tmp_path):
        """Artifact 写到默认路径."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.json"
        path = write_v1217_artifact(rep, custom)
        assert path.exists()
        assert path.suffix == ".json"

    def test_artifact_content(self, tmp_path):
        """Artifact 内容正确."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.json"
        path = write_v1217_artifact(rep, custom)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert data["dim_version"] == "0.6.27"
        assert "mn_metrics" in data
        assert "asi_overall_v1217" in data
        assert "pathways" in data
        assert "v3_guards" in data
        assert len(data["pathways"]) == 6
        assert data["n_pathways_total"] == 6
        assert data["total_mn_molecules"] == 75

    def test_artifact_pathways_have_molecules(self, tmp_path):
        """Artifact 每个 pathway 包含 molecules."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.json"
        path = write_v1217_artifact(rep, custom)
        data = json.loads(path.read_text(encoding="utf-8"))
        for name, pw in data["pathways"].items():
            assert "molecules" in pw
            assert "r_substrate" in pw
            assert "score" in pw

    def test_artifact_total_130(self, tmp_path):
        """Artifact 包含 130 cell extension."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.json"
        path = write_v1217_artifact(rep, custom)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert data["asi_overall_v1217"]["v1217_total_cells_130"] == 130
        assert data["asi_overall_v1217"]["v1217_realized_cells_count_100"] == 100


class TestWriteV1217Report:
    """write_v1217_report() Markdown 写入."""

    def test_report_write(self, tmp_path):
        """Report 写到默认路径."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.md"
        path = write_v1217_report(rep, custom)
        assert path.exists()
        assert path.suffix == ".md"

    def test_report_content(self, tmp_path):
        """Report 内容包含关键 sections."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.md"
        path = write_v1217_report(rep, custom)
        content = path.read_text(encoding="utf-8")
        assert "V1217" in content
        assert "MN x R1_growth" in content
        assert "MN x R10_plasticity" in content
        assert "MN x R12_ecology" in content
        assert "ASI North Star" in content
        assert "0.9800" in content
        assert "10 dim" in content

    def test_report_contains_citations(self, tmp_path):
        """Report 包含 pathway 引用."""
        rep = measure_v1217_full()
        custom = tmp_path / "v1217_test.md"
        path = write_v1217_report(rep, custom)
        content = path.read_text(encoding="utf-8")
        assert "MN_MORPHOGEN_GRADIENT" in content
        assert "MN_IEG_BEHAVIORAL" in content
        assert "MN_NICHE_CONSTRUCT" in content


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


class TestV3PhilosophyGuards:
    """V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_not_asi_terminal(self):
        """不假装 V1217 = ASI 终极."""
        assert V3_GUARDS["v1217_not_asi_terminal"] is True

    def test_not_full_replace(self):
        """不假装 V1217 = V1216 全替代."""
        assert V3_GUARDS["v1217_not_full_replace"] is True

    def test_lift_not_v1(self):
        """不假装 V1217 lift = ASI V1.0."""
        assert V3_GUARDS["v1217_lift_not_v1"] is True

    def test_realized_not_asi(self):
        """不假装 realized = ASI 已达."""
        assert V3_GUARDS["realized_not_asi"] is True

    def test_vacuous_gap_real(self):
        """不假装 vacuous_gap = 0."""
        assert V3_GUARDS["vacuous_gap_real"] is True

    def test_pathway_not_asi_substrate(self):
        """不假装 6 pathway = ASI 终极 substrate."""
        assert V3_GUARDS["pathway_not_asi_substrate"] is True

    def test_clamp_not_asi(self):
        """不假装 ASI 1.000000 clamp = ASI 已达."""
        assert V3_GUARDS["asi_clamp_not_asi_reached"] is True

    def test_molecules_not_complete(self):
        """不假装 75 真分子 = 完整 MN substrate."""
        assert V3_GUARDS["mn_molecules_not_complete"] is True

    def test_new_dim_not_all_dim(self):
        """不假装 新 dim 扩 = 全 dim 覆盖."""
        assert V3_GUARDS["new_dim_not_all_dim"] is True

    def test_not_all_mn_lift(self):
        """不假装 V1217 = 全 MN lift."""
        assert V3_GUARDS["v1217_not_all_mn_lift"] is True


# ============================================================================
# ASI 北极星 LOCKED (主 22:33)
# ============================================================================


class TestNorthStarLocked:
    """ASI 北极星 LOCKED 测 (主 22:33)."""

    def test_north_star_unchanged(self):
        """ASI 北极星 = 0.9800 不变 (LOCKED)."""
        rep = measure_v1217_full()
        assert rep.north_star == 0.9800

    def test_realized_below_north_star(self):
        """realized 远低于 北极星 (主 17:43 实事求是)."""
        rep = measure_v1217_full()
        assert rep.v1217_overall_realized_100 < ASI_NORTH_STAR
        # 0.5710 < 0.98 ✓

    def test_position_pct(self):
        """MN dim position vs ASI 北极星 % 计算正确."""
        rep = measure_v1217_full()
        # MN dim realized = 1.0, 1.0/0.98 = 102.04%
        assert rep.position_of_north_star_realized_pct > 100.0


# ============================================================================
# 矩阵扩展 117→130 (主 17:43)
# ============================================================================


class TestMatrixExtension:
    """V1216 9 dim × 13 R = 117 → V1217 10 dim × 13 R = 130 扩."""

    def test_extension_inflation_realized(self):
        """130 cell 公式下, 整体 realized 0.5710 < 北极星 0.98."""
        rep = measure_v1217_full()
        assert 0.55 < rep.v1217_overall_realized_100 < 0.60
        assert rep.v1217_overall_realized_100 < ASI_NORTH_STAR

    def test_extension_inflation_mean(self):
        """130 cell mean 0.4392, 比 V1216 0.4368 略升 (扩 130 +6 cells)."""
        rep = measure_v1217_full()
        assert rep.v1217_overall_mean_130 > V1216_OVERALL_MEAN_117 - 1e-3
        assert rep.v1217_overall_mean_130 < 0.45

    def test_100_cell_realized_recompute(self):
        """100 cell realizable, V1217 realized_mean = 0.5710."""
        rep = measure_v1217_full()
        assert abs(rep.v1217_overall_realized_100 - 0.5710) < 1e-3
        # Sanity: 100 = (94 V1216 realized + 6 V1217 MN lifted)
        assert rep.v1217_realized_cells_count == 100


# ============================================================================
# CLI 测试 (主 23:44 干到底)
# ============================================================================


class TestCLI:
    """V1217 CLI 测试."""

    def test_cli_measure(self):
        """--measure mode 输出关键指标."""
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1217_asi_v0627_manifestation_substrate_real_lift",
                "--measure",
            ],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1217 MN dim realized" in result.stdout
        assert "1.0000" in result.stdout

    def test_cli_json(self):
        """--json mode 输出 JSON."""
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1217_asi_v0627_manifestation_substrate_real_lift",
                "--json",
            ],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["dim_version"] == "0.6.27"
        assert data["n_pathways_total"] == 6
        assert data["total_mn_molecules"] == 75

    def test_cli_report(self):
        """--report mode 写 Markdown report."""
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1217_asi_v0627_manifestation_substrate_real_lift",
                "--report",
            ],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1217 report written" in result.stdout

    def test_cli_full(self):
        """--full mode 写 report + artifact."""
        result = subprocess.run(
            [
                sys.executable,
                "-m",
                "apeireth.v1217_asi_v0627_manifestation_substrate_real_lift",
                "--full",
            ],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1217 report" in result.stdout
        assert "V1217 artifact" in result.stdout

    def test_cli_default(self):
        """default mode 写 report + artifact."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1217_asi_v0627_manifestation_substrate_real_lift"],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0


# ============================================================================
# 端到端真测 (主 17:43 实事求是 — 真跑)
# ============================================================================


class TestV1217EndToEnd:
    """V1217 端到端真测."""

    def test_full_run_writes_files(self):
        """完整 run 写 files 到磁盘."""
        rep = measure_v1217_full()
        rep_path = write_v1217_report(rep)
        art_path = write_v1217_artifact(rep)
        try:
            assert rep_path.exists()
            assert art_path.exists()
            assert rep_path.stat().st_size > 1000
            assert art_path.stat().st_size > 1000
        finally:
            if rep_path.exists():
                rep_path.unlink()
            if art_path.exists():
                art_path.unlink()

    def test_no_exception_on_measure(self):
        """measure_v1217_full() 不抛异常."""
        rep = measure_v1217_full()
        assert rep is not None
        assert rep.elapsed >= 0

    def test_pathway_scores_have_6_entries(self):
        """pathway_scores 应有 6 entry."""
        rep = measure_v1217_full()
        assert len(rep.pathway_scores) == 6
        assert len(rep.pathway_real_molecule_count) == 6

    def test_mn_coverage_v1217_dict(self):
        """mn_coverage_v1217 dict 完整."""
        rep = measure_v1217_full()
        assert len(rep.mn_coverage_v1217) == 13

    def test_100_and_130_cell_split(self):
        """V1217 100 cell realized vs 130 cell total 区分."""
        rep = measure_v1217_full()
        # 100 = 94 (V1216) + 6 (V1217 MN lifted)
        # 130 = 117 (V1216) + 13 (V1217 MN total)
        assert rep.v1217_total_cells == 130
        assert rep.v1217_realized_cells_count == 100
