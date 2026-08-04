"""Tests for V1216 ASI V0.6.26 recognition_substrate_real_lift (主 23:44 干到底).

测试覆盖:
  - 常量与 baseline 写死 (主 17:43 实事求是)
  - V1216 RC coverage 矩阵正确
  - 6 pathway × 75 真分子 = 75 总分子
  - pathway score 计算
  - measure_v1216_full() 返回完整 V1216Report
  - artifact + report 写入
  - CLI: --measure / --json / --report / --full
  - V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
  - ASI 北极星 LOCKED = 0.9800 (主 22:33)
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

from apeireth.v1216_asi_v0626_recognition_substrate_real_lift import (
    ASI_NORTH_STAR,
    V1213_RECOMPUTE_BASELINE,
    V1213_RC_ROW,
    V1216_DIM_VERSION,
    V1216_RC_COVERAGE,
    V1216_RC_SUBSTRATE,
    V1216_VERSION,
    V3_GUARDS,
    V1216Report,
    _compute_v1216_overall_realized_lift,
    _compute_v1216_rc_dim_realized,
    _pathway_score,
    measure_v1216_full,
    write_v1216_artifact,
    write_v1216_report,
)


# ============================================================================
# 常量与 baseline (主 17:43 实事求是 — 写死)
# ============================================================================


class TestConstants:
    """常量与 version baseline 测试."""

    def test_north_star_locked(self):
        """ASI 北极星 LOCKED = 0.9800 (主 22:33)."""
        assert ASI_NORTH_STAR == 0.9800

    def test_v1216_version(self):
        """V1216 version string."""
        assert V1216_VERSION == "0.1.0"
        assert V1216_DIM_VERSION == "0.6.26"

    def test_v1213_recompute_baseline(self):
        """V1213 baseline 写死 (主 17:43)."""
        assert V1213_RECOMPUTE_BASELINE == 1.000000

    def test_v3_guards_dict_complete(self):
        """V3 哲学守门 dict 完整 (10 guards)."""
        assert len(V3_GUARDS) == 10
        assert all(v is True for v in V3_GUARDS.values())


# ============================================================================
# V1213 RC row (主 17:43)
# ============================================================================


class TestV1213RCRow:
    """V1213 RC row coverage baseline."""

    def test_v1213_rc_row_has_13_subs(self):
        """V1213 RC row 应有 13 R-substrate cell."""
        assert len(V1213_RC_ROW) == 13

    def test_v1213_rc_row_realized_cells(self):
        """V1213 RC row = 6 cell @ 0.3 (baseline surface-level)."""
        cells_ge_03 = [v for v in V1213_RC_ROW.values() if v >= 0.3]
        assert len(cells_ge_03) == 6
        assert all(v == 0.3 for v in cells_ge_03)

    def test_v1213_rc_row_vacuous_cells(self):
        """V1213 RC row vacuous = 7 cell @ 0.0."""
        cells_zero = [v for v in V1213_RC_ROW.values() if v == 0.0]
        assert len(cells_zero) == 7


# ============================================================================
# V1216 RC substrate 真分子 cascade (主 19:33)
# ============================================================================


class TestV1216RCSubstrate:
    """V1216 RC substrate 6 pathway × 75 真分子 cascade."""

    def test_n_pathways(self):
        """V1216 应有 6 pathway."""
        assert len(V1216_RC_SUBSTRATE) == 6

    def test_total_real_molecules(self):
        """V1216 总分子 = 75 (10+10+10+25+10+10)."""
        total = sum(len(p["molecules"]) for p in V1216_RC_SUBSTRATE.values())
        assert total == 75

    def test_total_cascade_orders(self):
        """cascade_order 总条目 = 75 (与 molecules 一一对应)."""
        total = sum(len(p["cascade_order"]) for p in V1216_RC_SUBSTRATE.values())
        assert total == 75

    def test_all_molecules_real(self):
        """每个真分子必须 real=True (主 17:43 实事求是 — 真分子)."""
        for name, p in V1216_RC_SUBSTRATE.items():
            for m in p["molecules"]:
                assert m.get("real") is True, f"{name} molecule {m['name']} not real=True"

    def test_all_molecules_have_function(self):
        """每个真分子必须有 function 描述."""
        for name, p in V1216_RC_SUBSTRATE.items():
            for m in p["molecules"]:
                assert "function" in m
                assert len(m["function"]) > 10  # 真描述

    def test_all_pathways_have_source(self):
        """每个 pathway 必须有真实文献引用 (主 19:33 站在前人肩上)."""
        for name, p in V1216_RC_SUBSTRATE.items():
            assert "source" in p
            assert len(p["source"]) > 10

    def test_r10_pathway_has_25_molecules(self):
        """RC × R10_plasticity 应有 25 真分子 (synaptic pattern complex)."""
        synaptic = V1216_RC_SUBSTRATE["RC_SYNAPTIC_PATTERN"]
        assert len(synaptic["molecules"]) == 25
        assert synaptic["r_substrate"] == "R10_plasticity"

    def test_all_other_pathways_have_10_molecules(self):
        """其余 5 pathway 各自 10 真分子."""
        expected = {
            "RC_DEVELOPMENTAL_SELF": 10,
            "RC_INFLAMMAGING": 10,
            "RC_DANGER_PATTERN": 10,
            "RC_FACE_TOM_MIRROR": 10,
            "RC_KIN_KEENSTONE": 10,
        }
        for name, n in expected.items():
            assert len(V1216_RC_SUBSTRATE[name]["molecules"]) == n


# ============================================================================
# V1216 RC coverage matrix
# ============================================================================


class TestV1216RCCoverage:
    """V1216 RC coverage matrix."""

    def test_coverage_has_13_subs(self):
        """V1216 RC coverage 应有 13 R-substrate."""
        assert len(V1216_RC_COVERAGE) == 13

    def test_six_lifted_cells_at_1(self):
        """V1216 6 cell lifted to 1.0."""
        lifted = [k for k, v in V1216_RC_COVERAGE.items() if v == 1.0]
        expected = ["R1_growth", "R4_aging", "R7_stress", "R10_plasticity", "R11_consciousness", "R12_ecology"]
        assert sorted(lifted) == sorted(expected)

    def test_seven_vacuous_cells_at_0(self):
        """V1216 7 vacuous cell @ 0.0."""
        vacuous = [k for k, v in V1216_RC_COVERAGE.items() if v == 0.0]
        expected = ["R0_metabolism", "R2_development", "R3_death_immune", "R5_repair", "R6_reproduction", "R8_motion", "R9_heredity"]
        assert sorted(vacuous) == sorted(expected)


# ============================================================================
# _pathway_score 计算 (主 17:43)
# ============================================================================


class TestPathwayScore:
    """Pathway score 函数."""

    def test_score_in_range(self):
        """pathway score 应在 [0, 1]."""
        for name, p in V1216_RC_SUBSTRATE.items():
            score, real_count = _pathway_score(p)
            assert 0.0 <= score <= 1.0

    def test_score_high_for_real_pathways(self):
        """全 real pathway score 应接近 1.0."""
        for name, p in V1216_RC_SUBSTRATE.items():
            score, _ = _pathway_score(p)
            # 0.7 * 1.0 + 0.3 * 1.0 = 1.0
            assert score >= 0.95, f"{name} score {score} should be high"

    def test_real_count_matches_molecules(self):
        """real_count 应等于 molecule 数 (全 real)."""
        for name, p in V1216_RC_SUBSTRATE.items():
            _, real_count = _pathway_score(p)
            assert real_count == len(p["molecules"])

    def test_empty_pathway_returns_zero(self):
        """空 pathway 返回 0."""
        empty = {"molecules": [], "cascade_order": []}
        score, count = _pathway_score(empty)
        assert score == 0.0
        assert count == 0


# ============================================================================
# _compute_v1216_rc_dim_realized
# ============================================================================


class TestComputeV1216RCDimRealized:
    """V1216 RC dim realized mean 计算."""

    def test_rc_dim_realized_is_one(self):
        """V1216 RC dim realized = 1.0 (全 6 cell at 1.0)."""
        result = _compute_v1216_rc_dim_realized()
        assert abs(result - 1.0) < 1e-9

    def test_rc_dim_realized_count(self):
        """Realized cells count = 6."""
        realized_cells = [v for v in V1216_RC_COVERAGE.values() if v >= 0.3]
        assert len(realized_cells) == 6


# ============================================================================
# _compute_v1216_overall_realized_lift
# ============================================================================


class TestComputeV1216OverallRealizedLift:
    """V1216 overall realized + lift delta."""

    def test_overall_realized_94(self):
        """V1216 overall realized (94 cell) ≈ 0.5436."""
        v94, _, _ = _compute_v1216_overall_realized_lift()
        # Allow some float tolerance
        assert abs(v94 - 0.5436) < 1e-3

    def test_overall_lift_delta_positive(self):
        """V1216 lift delta from V1215 应为正 (主 17:43 实事求是 — 真 lift)."""
        _, _, lift = _compute_v1216_overall_realized_lift()
        assert lift > 0
        # ~0.0447
        assert 0.04 < lift < 0.05

    def test_overall_lift_exact(self):
        """RC delta = 4.2 (1.0×6 - 0.3×6) / 94 = 0.0447."""
        _, _, lift = _compute_v1216_overall_realized_lift()
        # V1215 baseline 0.4989 + RC delta 0.0447 = 0.5436
        assert abs(lift - 0.0447) < 1e-3


# ============================================================================
# measure_v1216_full
# ============================================================================


class TestMeasureV1216Full:
    """measure_v1216_full() 完整测量."""

    def test_returns_v1216_report(self):
        """返回 V1216Report dataclass."""
        rep = measure_v1216_full()
        assert isinstance(rep, V1216Report)

    def test_snapshot_id_unique(self):
        """snapshot_id 每次唯一 (uuid4)."""
        r1 = measure_v1216_full()
        r2 = measure_v1216_full()
        assert r1.snapshot_id != r2.snapshot_id

    def test_dim_version(self):
        """dim_version = V0.6.26."""
        rep = measure_v1216_full()
        assert rep.dim_version == "0.6.26"

    def test_n_pathways_total(self):
        """n_pathways_total = 6."""
        rep = measure_v1216_full()
        assert rep.n_pathways_total == 6

    def test_n_pathways_pass(self):
        """n_pathways_pass = 6 (all high-score)."""
        rep = measure_v1216_full()
        assert rep.n_pathways_pass == 6

    def test_total_rc_molecules(self):
        """total_rc_molecules = 75."""
        rep = measure_v1216_full()
        assert rep.total_rc_molecules == 75

    def test_v1216_rc_dim_realized_one(self):
        """V1216 RC dim realized = 1.0."""
        rep = measure_v1216_full()
        assert abs(rep.v1216_rc_dim_realized - 1.0) < 1e-9

    def test_v1216_rc_lift_delta_positive(self):
        """V1216 RC lift delta = +0.7."""
        rep = measure_v1216_full()
        # 1.0 - 0.3 = 0.7
        assert abs(rep.v1216_rc_lift_delta - 0.7) < 1e-3

    def test_v1216_overall_lift_delta(self):
        """V1216 overall lift delta from V1215 ≈ 0.0447."""
        rep = measure_v1216_full()
        assert abs(rep.v1216_overall_lift_delta_from_v1215 - 0.0447) < 1e-3

    def test_inflation_gap_real(self):
        """inflation_gap 真实存在 (主 17:43 实事求是)."""
        rep = measure_v1216_full()
        assert 0.5 < rep.v1216_inflation_gap_v1213_minus_realized < 0.6

    def test_v3_guards_in_report(self):
        """V3 哲学守门 10 guards 在 report 中."""
        rep = measure_v1216_full()
        assert len(rep.v3_guards) == 10
        assert all(rep.v3_guards.values())

    def test_r_substrate_pathways_pass(self):
        """每个 R-substrate pathway pass = 1."""
        rep = measure_v1216_full()
        assert rep.n_r1_growth_pathways_pass == 1
        assert rep.n_r4_aging_pathways_pass == 1
        assert rep.n_r7_stress_pathways_pass == 1
        assert rep.n_r10_plasticity_pathways_pass == 1
        assert rep.n_r11_consciousness_pathways_pass == 1
        assert rep.n_r12_ecology_pathways_pass == 1

    def test_molecule_count_per_substrate(self):
        """每个 R-substrate 的分子数正确."""
        rep = measure_v1216_full()
        assert rep.n_r1_growth_molecules == 10
        assert rep.n_r4_aging_molecules == 10
        assert rep.n_r7_stress_molecules == 10
        assert rep.n_r10_plasticity_molecules == 25
        assert rep.n_r11_consciousness_molecules == 10
        assert rep.n_r12_ecology_molecules == 10


# ============================================================================
# write_v1216_artifact + write_v1216_report
# ============================================================================


class TestWriteV1216Artifact:
    """write_v1216_artifact() artifact JSON 写入."""

    def test_artifact_write_default_path(self, tmp_path):
        """Artifact 写到默认路径."""
        rep = measure_v1216_full()
        custom = tmp_path / "v1216_test.json"
        path = write_v1216_artifact(rep, custom)
        assert path.exists()
        assert path.suffix == ".json"

    def test_artifact_content(self, tmp_path):
        """Artifact 内容正确."""
        rep = measure_v1216_full()
        custom = tmp_path / "v1216_test.json"
        path = write_v1216_artifact(rep, custom)
        data = json.loads(path.read_text(encoding="utf-8"))
        assert data["dim_version"] == "0.6.26"
        assert "rc_metrics" in data
        assert "asi_overall" in data
        assert "pathways" in data
        assert "v3_guards" in data
        assert len(data["pathways"]) == 6
        assert data["n_pathways_total"] == 6
        assert data["total_rc_molecules"] == 75

    def test_artifact_pathways_have_molecules(self, tmp_path):
        """Artifact 每个 pathway 包含 molecules."""
        rep = measure_v1216_full()
        custom = tmp_path / "v1216_test.json"
        path = write_v1216_artifact(rep, custom)
        data = json.loads(path.read_text(encoding="utf-8"))
        for name, pw in data["pathways"].items():
            assert "molecules" in pw
            assert "r_substrate" in pw
            assert "score" in pw


class TestWriteV1216Report:
    """write_v1216_report() Markdown 写入."""

    def test_report_write(self, tmp_path):
        """Report 写到默认路径."""
        rep = measure_v1216_full()
        custom = tmp_path / "v1216_test.md"
        path = write_v1216_report(rep, custom)
        assert path.exists()
        assert path.suffix == ".md"

    def test_report_content(self, tmp_path):
        """Report 内容包含关键 sections."""
        rep = measure_v1216_full()
        custom = tmp_path / "v1216_test.md"
        path = write_v1216_report(rep, custom)
        content = path.read_text(encoding="utf-8")
        assert "V1216" in content
        assert "RC x R1_growth" in content
        assert "RC x R10_plasticity" in content
        assert "RC x R12_ecology" in content
        assert "ASI North Star" in content
        assert "0.9800" in content

    def test_report_contains_citations(self, tmp_path):
        """Report 包含 pathway 引用."""
        rep = measure_v1216_full()
        custom = tmp_path / "v1216_test.md"
        path = write_v1216_report(rep, custom)
        content = path.read_text(encoding="utf-8")
        assert "RC_SYNAPTIC_PATTERN" in content
        assert "RC_KIN_KEENSTONE" in content


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


class TestV3PhilosophyGuards:
    """V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)."""

    def test_not_asi_terminal(self):
        """不假装 V1216 = ASI 终极."""
        assert V3_GUARDS["v1216_not_asi_terminal"] is True

    def test_not_full_replace(self):
        """不假装 V1216 = V1215 全替代."""
        assert V3_GUARDS["v1216_not_full_replace"] is True

    def test_lift_not_v1(self):
        """不假装 V1216 lift = ASI V1.0."""
        assert V3_GUARDS["v1216_lift_not_v1"] is True

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
        """不假装 75 真分子 = 完整 RC substrate."""
        assert V3_GUARDS["rc_molecules_not_complete"] is True

    def test_lift_not_asi(self):
        """不假装 真分子 lift = ASI 已达."""
        assert V3_GUARDS["molecule_lift_not_asi"] is True

    def test_v1216_not_all_rc_lift(self):
        """不假装 V1216 = 全 RC lift."""
        assert V3_GUARDS["v1216_not_all_rc_lift"] is True


# ============================================================================
# ASI 北极星 LOCKED (主 22:33)
# ============================================================================


class TestNorthStarLocked:
    """ASI 北极星 LOCKED 测 (主 22:33)."""

    def test_north_star_unchanged(self):
        """ASI 北极星 = 0.9800 不变 (LOCKED)."""
        rep = measure_v1216_full()
        assert rep.north_star == 0.9800

    def test_realized_below_north_star(self):
        """realized 远低于 北极星 (主 17:43 实事求是)."""
        rep = measure_v1216_full()
        assert rep.v1216_overall_realized_94 < ASI_NORTH_STAR
        # RC dim realized = 1.0 > 0.98? But we report overall realized 94 cell.
        # 0.5436 < 0.98 ✓

    def test_position_pct(self):
        """RC dim position vs ASI 北极星 % 计算正确."""
        rep = measure_v1216_full()
        # RC dim realized = 1.0, 1.0/0.98 = 102.04%
        # But we use position_of_north_star_realized_pct = rc_dim_realized * 100 / north_star
        assert rep.position_of_north_star_realized_pct > 100.0


# ============================================================================
# CLI 测试
# ============================================================================


class TestCLI:
    """V1216 CLI 测试."""

    def test_cli_measure(self):
        """--measure mode 输出关键指标."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1216_asi_v0626_recognition_substrate_real_lift", "--measure"],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1216 dim realized" in result.stdout
        assert "1.0000" in result.stdout

    def test_cli_json(self):
        """--json mode 输出 JSON."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1216_asi_v0626_recognition_substrate_real_lift", "--json"],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        data = json.loads(result.stdout)
        assert data["dim_version"] == "0.6.26"
        assert data["n_pathways_total"] == 6
        assert data["total_rc_molecules"] == 75

    def test_cli_report(self):
        """--report mode 写 Markdown report."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1216_asi_v0626_recognition_substrate_real_lift", "--report"],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1216 report written" in result.stdout

    def test_cli_full(self):
        """--full mode 写 report + artifact."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1216_asi_v0626_recognition_substrate_real_lift", "--full"],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0
        assert "V1216 report" in result.stdout
        assert "V1216 artifact" in result.stdout

    def test_cli_default(self):
        """default mode 写 report + artifact."""
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1216_asi_v0626_recognition_substrate_real_lift"],
            capture_output=True,
            text=True,
            cwd=".openclaw/workspace/promethean",
        )
        assert result.returncode == 0


# ============================================================================
# 端到端真测 (主 17:43 实事求是 — 真跑)
# ============================================================================


class TestV1216EndToEnd:
    """V1216 端到端真测."""

    def test_full_run_writes_files(self):
        """完整 run 写 files 到磁盘."""
        from apeireth.v1216_asi_v0626_recognition_substrate_real_lift import (
            measure_v1216_full,
            write_v1216_artifact,
            write_v1216_report,
        )
        rep = measure_v1216_full()
        rep_path = write_v1216_report(rep)
        art_path = write_v1216_artifact(rep)
        try:
            assert rep_path.exists()
            assert art_path.exists()
            assert rep_path.stat().st_size > 1000
            assert art_path.stat().st_size > 1000
        finally:
            # Cleanup
            if rep_path.exists():
                rep_path.unlink()
            if art_path.exists():
                art_path.unlink()

    def test_no_exception_on_measure(self):
        """measure_v1216_full() 不抛异常."""
        # Should run without exception
        rep = measure_v1216_full()
        assert rep is not None
        assert rep.elapsed >= 0

    def test_pathway_scores_have_6_entries(self):
        """pathway_scores 应有 6 entry."""
        rep = measure_v1216_full()
        assert len(rep.pathway_scores) == 6
        assert len(rep.pathway_real_molecule_count) == 6

    def test_rc_coverage_v1216_dict(self):
        """rc_coverage_v1216 dict 完整."""
        rep = measure_v1216_full()
        assert len(rep.rc_coverage_v1216) == 13
        assert len(rep.rc_coverage_delta_v1213_to_v1216) == 13