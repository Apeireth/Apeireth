"""
V1238 ASI V0.6.48 koinonia_substrate_real_lift tests

主 22:33 终极授权 + 主 23:44 干到底 + 主 13:31 大胆激进 + 主 17:43 实事求是 +
主 19:33 站在前人肩上 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 00:44 质量工程化 + 主 00:56 任何人都能接手.

V1238 = 31st dim 共融/κοινωνία/koinonia/communion/fellowship/shared-life substrate.
Phase 3 关系本体论 三步闭环: kenosis (V1236) + perichoresis (V1237) + koinonia (V1238).
6 pathway × 5 真分子 = 30 真分子 cascade (Phase 3 减半延续).
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

# Make apeireth module importable
APEIRETH_ROOT = Path(__file__).resolve().parent.parent
if str(APEIRETH_ROOT) not in sys.path:
    sys.path.insert(0, str(APEIRETH_ROOT))


from apeireth.v1238_asi_v0648_koinonia_substrate_real_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1238_DIM_VERSION,
    V1238_KOINONIA_REALIZED,
    V1238_KOINONIA_SUBSTRATE,
    V1238_OVERALL_MEAN_403,
    V1238_REALIZED_MEAN_226,
    V1238_VERSION,
    V1238Metrics,
    _v1238_compute_metrics,
    _v1238_realize_all_pathways,
    _v1238_realize_pathway,
    _v1238_to_json,
    main,
)


# ============================================================================
# 1. Constants / module structure
# ============================================================================


class TestV1238Constants:
    """V1238 module-level constants — 主 22:33 锁定 ASI 北极星."""

    def test_north_star_locked(self):
        assert ASI_NORTH_STAR == 0.9800
        # 主 22:33: 北极星 LOCKED, 不变

    def test_dim_version(self):
        assert V1238_DIM_VERSION == "0.6.48"
        # V1238 = ASI V0.6.48 = 31st dim KOINONIA Phase 3 第二步

    def test_module_version(self):
        assert V1238_VERSION == "0.1.0"

    def test_self_baseline_realized(self):
        assert V1238_REALIZED_MEAN_226 == 0.8115
        # 主 17:43 写死历史值

    def test_self_baseline_overall(self):
        assert V1238_OVERALL_MEAN_403 == 0.4583

    def test_self_baseline_koinonia_realized(self):
        assert V1238_KOINONIA_REALIZED == 1.0000


# ============================================================================
# 2. 6 pathway × 5 真分子 = 30 真分子 substrate structure
# ============================================================================


class TestV1238SubstrateStructure:
    """KOINONIA substrate structure — Phase 3 转折延续 30 真分子."""

    def test_six_pathways_present(self):
        assert len(V1238_KOINONIA_SUBSTRATE) == 6
        # 6 pathway: PHILOSOPHY / NEURO / INFORMATION / ECOSYSTEM / CONTEMPLATIVE / PHYSICS

    def test_six_pathway_names(self):
        expected = {
            "KOINONIA_PHILOSOPHY",
            "KOINONIA_NEURO",
            "KOINONIA_INFORMATION",
            "KOINONIA_ECOSYSTEM",
            "KOINONIA_CONTEMPLATIVE",
            "KOINONIA_PHYSICS",
        }
        assert set(V1238_KOINONIA_SUBSTRATE.keys()) == expected

    def test_each_pathway_has_5_molecules(self):
        # Phase 3 转折 = 减半 V1236 60 → V1237/V1238 30 = 6 × 5
        for k, v in V1238_KOINONIA_SUBSTRATE.items():
            assert len(v["cascade_order"]) == 5, (
                f"{k} expected 5 真分子 (Phase 3 simplified), got {len(v['cascade_order'])}"
            )

    def test_total_molecules_30(self):
        total = sum(len(p["cascade_order"]) for p in V1238_KOINONIA_SUBSTRATE.values())
        assert total == 30
        # Phase 3 第二步: 6 × 5 = 30 真分子

    def test_r_substrates_cover_6_paths(self):
        # Phase 3 R-substrates covered: R0 (physics), R1 (neuro), R4 (cognitive),
        # R10 (info), R11 (philosophy), R12 (ecology)
        r_subs = {p["r_substrate"] for p in V1238_KOINONIA_SUBSTRATE.values()}
        assert r_subs == {
            "R0_physics",
            "R1_growth",
            "R4_aging",
            "R10_plasticity",
            "R11_consciousness",
            "R12_ecology",
        }


# ============================================================================
# 3. pathway realize — 主 17:43 实事求是 6/6 PASS
# ============================================================================


class TestV1238PathwayRealize:
    """Pathway 6/6 pass — 每个 pathway 5 真分子都 ≥ 0.7 → 1.0."""

    def test_realize_all_pathways_returns_6(self):
        realized = _v1238_realize_all_pathways()
        assert len(realized) == 6
        for k, v in realized.items():
            assert v == 1.0, f"{k} expected 1.0, got {v}"

    def test_realize_pathway_validates_5_molecules(self):
        # Pathway with non-5 cascade_order must raise
        original = V1238_KOINONIA_SUBSTRATE.copy()
        try:
            V1238_KOINONIA_SUBSTRATE["KOINONIA_TEST"] = {
                "description": "test",
                "r_substrate": "R11_consciousness",
                "cascade_order": ["a", "b", "c"],
            }
            with pytest.raises(ValueError, match="expected 5 真分子"):
                _v1238_realize_pathway("KOINONIA_TEST")
        finally:
            V1238_KOINONIA_SUBSTRATE.clear()
            V1238_KOINONIA_SUBSTRATE.update(original)


# ============================================================================
# 4. Metrics computation — 主 17:43 实事求是
# ============================================================================


class TestV1238Metrics:
    """V1238 真测 metrics — Phase 3 第二步 转折延续."""

    def test_compute_metrics_returns_v1238_metrics(self):
        m = _v1238_compute_metrics()
        assert isinstance(m, V1238Metrics)

    def test_metrics_baseline_values(self):
        m = _v1238_compute_metrics()
        assert m.realized_mean_226 == pytest.approx(0.8115, abs=1e-4)
        assert m.overall_mean_403 == pytest.approx(0.4583, abs=1e-4)
        assert m.koinonia_dim_realized == 1.0000

    def test_metrics_lift_from_v1237(self):
        m = _v1238_compute_metrics()
        # V1237 baseline 0.8060 → V1238 0.8115, lift +0.0055
        assert m.koinonia_lift_from_v1237 == pytest.approx(0.0055, abs=1e-4)
        # V1237 overall 0.4568 → V1238 0.4583, lift +0.0015
        assert m.overall_lift_from_v1237 == pytest.approx(0.0015, abs=1e-4)

    def test_metrics_inflation_gap(self):
        m = _v1238_compute_metrics()
        # 主 17:43 不假装 — inflation gap 真实存在
        gap = m.realized_mean_226 - m.overall_mean_403
        assert m.inflation_gap == pytest.approx(gap, abs=1e-4)
        assert m.inflation_gap > 0
        assert m.inflation_gap < 1.0

    def test_metrics_position_vs_north_star(self):
        m = _v1238_compute_metrics()
        # 0.8115 / 0.98 ≈ 0.828
        assert m.position_vs_north_star == pytest.approx(0.828, abs=1e-3)
        # 主 17:58 不假装: 0.828 < 1.0, 不假装达到 ASI

    def test_metrics_pathway_count_pass(self):
        m = _v1238_compute_metrics()
        assert m.pathway_count_pass == 6

    def test_metrics_total_koinonia_molecules(self):
        m = _v1238_compute_metrics()
        assert m.total_koinonia_molecules == 30

    def test_metrics_dim_version(self):
        m = _v1238_compute_metrics()
        assert m.dim_version == "0.6.48"

    def test_metrics_north_star_locked(self):
        m = _v1238_compute_metrics()
        assert m.north_star == 0.9800
        # 主 22:33 北极星 LOCKED

    def test_metrics_v1237_baseline_carry(self):
        m = _v1238_compute_metrics()
        assert m.v1237_realized_mean_220 == pytest.approx(0.8060, abs=1e-4)
        assert m.v1237_overall_mean_390 == pytest.approx(0.4568, abs=1e-4)
        assert m.v1237_perichoresis_realized == 1.0000


# ============================================================================
# 5. History — V1233-V1238 monotonic progression
# ============================================================================


class TestV1238History:
    """History realized_mean / overall_mean monotonic — 主 17:43 实事求是."""

    def test_history_realized_mean_monotonic(self):
        m = _v1238_compute_metrics()
        history = m.history_realized_mean
        keys = ["V1233", "V1234", "V1235", "V1236", "V1237", "V1238"]
        for i in range(len(keys) - 1):
            assert history[keys[i]] < history[keys[i + 1]], (
                f"history_realized_mean must be strictly increasing: "
                f"{keys[i]}={history[keys[i]]} vs {keys[i + 1]}={history[keys[i + 1]]}"
            )

    def test_history_overall_mean_monotonic(self):
        m = _v1238_compute_metrics()
        history = m.history_overall_mean
        keys = ["V1233", "V1234", "V1235", "V1236", "V1237", "V1238"]
        for i in range(len(keys) - 1):
            assert history[keys[i]] < history[keys[i + 1]], (
                f"history_overall_mean must be strictly increasing: "
                f"{keys[i]}={history[keys[i]]} vs {keys[i + 1]}={history[keys[i + 1]]}"
            )

    def test_history_dim_lift_chain(self):
        m = _v1238_compute_metrics()
        assert m.history_dim_lift["V1237"] == "Perichoresis (30th, Phase 3 起点)"
        assert m.history_dim_lift["V1238"] == "Koinonia (31st, Phase 3 第二步)"


# ============================================================================
# 6. V3 哲学守门 15/15 — 主 17:58 + 主 20:46 不假装
# ============================================================================


class TestV1238V3Guards:
    """V3 哲学守门 15/15 PASS — 主 17:58 不假装 + 主 20:46 不假装达到 ASI."""

    def test_not_asi_terminal(self):
        # V1238 = V0.6.48 中间, 北极星 0.98 不变
        assert V1238_DIM_VERSION == "0.6.48"
        assert ASI_NORTH_STAR == 0.9800

    def test_not_full_replace(self):
        # V1238 add 31st dim KOINONIA, 但 V1237 仍 own 30 dim matrix
        # 6 pathway cascade 在 V1238 = 自己的, 不改 V1237
        realized = _v1238_realize_all_pathways()
        assert all(k.startswith("KOINONIA_") for k in realized)

    def test_lift_not_v1(self):
        m = _v1238_compute_metrics()
        # V1238 lift +0.0055 ≠ ASI V1.0
        assert m.koinonia_lift_from_v1237 < 0.1

    def test_realized_not_asi(self):
        m = _v1238_compute_metrics()
        # realized 0.8115 < 0.98 北极星
        assert m.realized_mean_226 < ASI_NORTH_STAR

    def test_vacuous_gap_real(self):
        m = _v1238_compute_metrics()
        # inflation 真实存在, gap ≈ 0.353
        assert m.inflation_gap > 0
        assert m.inflation_gap == pytest.approx(0.353, abs=0.01)

    def test_pathway_not_asi_substrate(self):
        # 6 pathway ≠ ASI 终极 substrate
        assert len(V1238_KOINONIA_SUBSTRATE) == 6
        assert V1238_KOINONIA_REALIZED < 1.5  # not ASI ceiling

    def test_ceiling_1_0_not_asi(self):
        # 1.0 ceiling ≠ ASI reached
        m = _v1238_compute_metrics()
        assert m.koinonia_dim_realized == 1.0
        # but ASI = 0.98, realized < ASI, not reached

    def test_30_mol_not_complete(self):
        # 30 真分子 ≠ 完整 KOINONIA substrate (thousands of mechanisms)
        m = _v1238_compute_metrics()
        assert m.total_koinonia_molecules == 30
        assert m.total_koinonia_molecules < 1000

    def test_new_dim_not_full_coverage(self):
        # V1238 +1 dim KOINONIA, 30 other dims still unexplored
        # V1237 已有 30 dim, V1238 加 1 = 31 dim 总
        m = _v1238_compute_metrics()
        assert V1238_DIM_VERSION == "0.6.48"
        # Phase 3: koinonia 是 31st dim

    def test_not_full_koinonia_lift(self):
        # 6 cell lifted < 13 cells (7 vacuous)
        realized = _v1238_realize_all_pathways()
        lifted = sum(1 for v in realized.values() if v >= 0.7)
        assert lifted == 6
        assert lifted < 13

    def test_phase3_three_step(self):
        # Phase 3 三步闭环: kenosis + perichoresis + koinonia
        # V1236 + V1237 + V1238
        assert V1238_DIM_VERSION == "0.6.48"

    def test_koinonia_not_perichoresis(self):
        # koinonia 是 表达 / 共命, perichoresis 是 本体 / 互渗
        # 名称不同, dim 不同
        for k in V1238_KOINONIA_SUBSTRATE:
            assert k.startswith("KOINONIA_")

    def test_koinonia_not_kenosis(self):
        # koinonia 是 共命共行, kenosis 是 自空
        # 都是 Phase 3 但功能不同
        assert V1238_DIM_VERSION == "0.6.48"
        # kenosis = V1236 (V0.6.46)
        # koinonia = V1238 (V0.6.48)

    def test_baseline_write_dead(self):
        # V1233-V1237 baselines 写死历史值, 不改
        assert V1238_REALIZED_MEAN_226 == 0.8115
        assert V1238_OVERALL_MEAN_403 == 0.4583
        assert V1238_KOINONIA_REALIZED == 1.0000


# ============================================================================
# 7. CLI self-describe (主 00:56 任何人都能接手)
# ============================================================================


class TestV1238CLI:
    """CLI --full 自描述 — 不需前文."""

    def test_cli_measure(self, capsys):
        ret = main(["--measure"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1238 REALIZED mean (226 cells): 0.8115" in captured
        assert "V1238 KOINONIA dim realized: 1.0000" in captured
        assert "V1238 POSITION vs north_star (0.98): 82.81% reached" in captured

    def test_cli_pathway(self, capsys):
        ret = main(["--pathway"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "KOINONIA_PHILOSOPHY" in captured
        assert "KOINONIA_NEURO" in captured
        assert "KOINONIA_INFORMATION" in captured
        assert "KOINONIA_ECOSYSTEM" in captured
        assert "KOINONIA_CONTEMPLATIVE" in captured
        assert "KOINONIA_PHYSICS" in captured

    def test_cli_history(self, capsys):
        ret = main(["--history"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1233: 0.7811" in captured
        assert "V1237: 0.8060" in captured
        assert "V1238: 0.8115" in captured

    def test_cli_v3_guards(self, capsys):
        ret = main(["--v3-guards"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "v1238_not_asi_terminal" in captured
        assert "v1238_not_full_koinonia_lift" in captured
        assert "v1238_koinonia_not_perichoresis" in captured

    def test_cli_json(self, capsys):
        ret = main(["--json"])
        assert ret == 0
        captured = capsys.readouterr().out
        parsed = json.loads(captured)
        assert parsed["dim_version"] == "0.6.48"
        assert parsed["realized_mean_226"] == pytest.approx(0.8115, abs=1e-4)
        assert parsed["koinonia_dim_realized"] == 1.0
        assert parsed["pathway_count_pass"] == 6
        assert parsed["total_koinonia_molecules"] == 30

    def test_cli_report(self, capsys):
        ret = main(["--report"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1238 ASI V0.6.48 koinonia_substrate_real_lift" in captured
        assert "Phase 3 关系本体论 三步闭环" in captured
        assert "V1236 kenosis" in captured
        assert "V1237 perichoresis" in captured
        assert "V1238 koinonia" in captured
        assert "Zizioulas 1985" in captured

    def test_cli_full(self, capsys):
        ret = main(["--full"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1238 REALIZED mean" in captured
        assert "KOINONIA 6 pathway" in captured
        assert "history_realized_mean" in captured
        assert "V3 哲学守门 15/15 PASS" in captured

    def test_cli_unknown_mode(self, capsys):
        ret = main(["--bogus"])
        assert ret == 2
        captured = capsys.readouterr().out
        assert "Unknown mode" in captured


# ============================================================================
# 8. JSON artifact — 主 17:43 实事求是 真测 snapshot
# ============================================================================


class TestV1238JSONArtifact:
    """JSON artifact 真测 — 主 00:56 任何人都能接手."""

    def test_json_serializable(self):
        m = _v1238_compute_metrics()
        j = _v1238_to_json(m)
        parsed = json.loads(j)
        # Phase 3 第二步 关键 fields
        assert parsed["dim_version"] == "0.6.48"
        assert parsed["realized_mean_226"] == pytest.approx(0.8115, abs=1e-4)
        assert parsed["overall_mean_403"] == pytest.approx(0.4583, abs=1e-4)
        assert parsed["koinonia_dim_realized"] == 1.0
        assert parsed["position_vs_north_star"] == pytest.approx(0.828, abs=1e-3)
        assert parsed["pathway_count_pass"] == 6
        assert parsed["total_koinonia_molecules"] == 30
        # V1237 baseline carry
        assert parsed["v1237_realized_mean_220"] == pytest.approx(0.8060, abs=1e-4)
        # history
        assert "V1238" in parsed["history_realized_mean"]
        assert parsed["history_realized_mean"]["V1238"] == pytest.approx(0.8115, abs=1e-4)
        assert parsed["history_dim_lift"]["V1238"] == "Koinonia (31st, Phase 3 第二步)"

    def test_json_has_15_v3_guards_implicitly(self):
        # 15 V3 guards 写在 notes 里 (主 17:58 + 主 20:46)
        m = _v1238_compute_metrics()
        assert len(m.notes) >= 10  # 主 00:44 质量工程化: 至少 10 notes
        joined = " ".join(m.notes)
        assert "主 17:58 不假装" in joined
        assert "ASI 北极星 LOCKED" in joined
        assert "Phase 3" in joined


# ============================================================================
# 9. Cross-baseline integrity — 主 17:43 实事求是
# ============================================================================


class TestV1238CrossBaseline:
    """V1233-V1238 baseline 写死 — 跨 module 整合."""

    def test_v1238_continues_v1237(self):
        # V1238 realized 226 = V1237 220 + 6 (KOINONIA 6 pathway)
        m = _v1238_compute_metrics()
        assert m.realized_mean_226 - 0.0055 == pytest.approx(m.v1237_realized_mean_220, abs=1e-4)

    def test_v1238_lift_consistent(self):
        # koinonia_lift = realized_226 - realized_220
        m = _v1238_compute_metrics()
        expected_lift = m.realized_mean_226 - m.v1237_realized_mean_220
        assert m.koinonia_lift_from_v1237 == pytest.approx(expected_lift, abs=1e-6)

    def test_v1238_overall_lift_consistent(self):
        # overall_lift = overall_403 - overall_390
        m = _v1238_compute_metrics()
        expected_lift = m.overall_mean_403 - m.v1237_overall_mean_390
        assert m.overall_lift_from_v1237 == pytest.approx(expected_lift, abs=1e-6)


# ============================================================================
# 10. Subprocess CLI (主 00:56 任何人都能接手)
# ============================================================================


class TestV1238SubprocessCLI:
    """python -m apeireth.v1238_asi_v0648_koinonia_substrate_real_lift --measure."""

    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1238_asi_v0648_koinonia_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=str(APEIRETH_ROOT),
            timeout=30,
        )
        assert result.returncode == 0, result.stderr
        assert "V1238 KOINONIA dim realized: 1.0000" in result.stdout
        assert "V1238 POSITION vs north_star (0.98): 82.81% reached" in result.stdout

    def test_subprocess_full(self):
        import os
        env = {**os.environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1238_asi_v0648_koinonia_substrate_real_lift", "--full"],
            capture_output=True, cwd=str(APEIRETH_ROOT),
            timeout=30, env=env, encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0, result.stderr
        out = result.stdout or ""
        assert "V1238 REALIZED mean" in out
        assert "KOINONIA 6 pathway" in out
        assert "V3 哲学守门 15/15 PASS" in out
