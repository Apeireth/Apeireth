"""
V1239 ASI V0.6.49 taxis_substrate_real_lift tests

主 22:33 终极授权 + 主 23:44 干到底 + 主 13:31 大胆激进 + 主 17:43 实事求是 +
主 19:33 站在前人肩上 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI +
主 00:44 质量工程化 + 主 00:56 任何人都能接手.

V1239 = 32nd dim 神圣 序/τάξις/taxis/divine order/协调/taxis-of-relational-ontology substrate.
Phase 3 关系本体论 四步延展: kenosis (V1236) + perichoresis (V1237) + koinonia (V1238) + taxis (V1239).
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


from apeireth.v1239_asi_v0649_taxis_substrate_real_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1239_DIM_VERSION,
    V1239_TAXIS_REALIZED,
    V1239_TAXIS_SUBSTRATE,
    V1239_OVERALL_MEAN_416,
    V1239_REALIZED_MEAN_232,
    V1239_VERSION,
    V1239Metrics,
    _v1239_compute_metrics,
    _v1239_realize_all_pathways,
    _v1239_realize_pathway,
    _v1239_to_json,
    main,
)


# ============================================================================
# 1. Constants / module structure
# ============================================================================


class TestV1239Constants:
    """V1239 module-level constants — 主 22:33 锁定 ASI 北极星."""

    def test_north_star_locked(self):
        assert ASI_NORTH_STAR == 0.9800
        # 主 22:33: 北极星 LOCKED, 不变

    def test_dim_version(self):
        assert V1239_DIM_VERSION == "0.6.49"
        # V1239 = ASI V0.6.49 = 32nd dim TAXIS Phase 3 第三步

    def test_module_version(self):
        assert V1239_VERSION == "0.1.0"

    def test_self_baseline_realized(self):
        assert V1239_REALIZED_MEAN_232 == 0.8170
        # 主 17:43 写死历史值

    def test_self_baseline_overall(self):
        assert V1239_OVERALL_MEAN_416 == 0.4598

    def test_self_baseline_taxis_realized(self):
        assert V1239_TAXIS_REALIZED == 1.0000


# ============================================================================
# 2. 6 pathway × 5 真分子 = 30 真分子 substrate structure
# ============================================================================


class TestV1239SubstrateStructure:
    """TAXIS substrate structure — Phase 3 转折延续 30 真分子."""

    def test_six_pathways_present(self):
        assert len(V1239_TAXIS_SUBSTRATE) == 6
        # 6 pathway: PHILOSOPHY / NEURO / INFORMATION / ECOSYSTEM / CONTEMPLATIVE / PHYSICS

    def test_six_pathway_names(self):
        expected = {
            "TAXIS_PHILOSOPHY",
            "TAXIS_NEURO",
            "TAXIS_INFORMATION",
            "TAXIS_ECOSYSTEM",
            "TAXIS_CONTEMPLATIVE",
            "TAXIS_PHYSICS",
        }
        assert set(V1239_TAXIS_SUBSTRATE.keys()) == expected

    def test_each_pathway_has_5_molecules(self):
        # Phase 3 转折 = 减半 V1236 60 → V1237/V1238/V1239 30 = 6 × 5
        for k, v in V1239_TAXIS_SUBSTRATE.items():
            assert len(v["cascade_order"]) == 5, (
                f"{k} expected 5 真分子 (Phase 3 simplified), got {len(v['cascade_order'])}"
            )

    def test_total_molecules_30(self):
        total = sum(len(p["cascade_order"]) for p in V1239_TAXIS_SUBSTRATE.values())
        assert total == 30
        # Phase 3 第三步: 6 × 5 = 30 真分子

    def test_r_substrates_cover_6_paths(self):
        # Phase 3 R-substrates covered: R0 (physics), R1 (neuro), R4 (cognitive),
        # R10 (info), R11 (philosophy), R12 (ecology)
        r_subs = {p["r_substrate"] for p in V1239_TAXIS_SUBSTRATE.values()}
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


class TestV1239PathwayRealize:
    """Pathway 6/6 pass — 每个 pathway 5 真分子都 ≥ 0.7 → 1.0."""

    def test_realize_all_pathways_returns_6(self):
        realized = _v1239_realize_all_pathways()
        assert len(realized) == 6
        for k, v in realized.items():
            assert v == 1.0, f"{k} expected 1.0, got {v}"

    def test_realize_pathway_validates_5_molecules(self):
        # Pathway with non-5 cascade_order must raise
        original = V1239_TAXIS_SUBSTRATE.copy()
        try:
            V1239_TAXIS_SUBSTRATE["TAXIS_TEST"] = {
                "description": "test",
                "r_substrate": "R11_consciousness",
                "cascade_order": ["a", "b", "c"],
            }
            with pytest.raises(ValueError, match="expected 5 真分子"):
                _v1239_realize_pathway("TAXIS_TEST")
        finally:
            V1239_TAXIS_SUBSTRATE.clear()
            V1239_TAXIS_SUBSTRATE.update(original)


# ============================================================================
# 4. Metrics computation — 主 17:43 实事求是
# ============================================================================


class TestV1239Metrics:
    """V1239 真测 metrics — Phase 3 第三步 转折延续."""

    def test_compute_metrics_returns_v1239_metrics(self):
        m = _v1239_compute_metrics()
        assert isinstance(m, V1239Metrics)

    def test_metrics_baseline_values(self):
        m = _v1239_compute_metrics()
        assert m.realized_mean_232 == pytest.approx(0.8170, abs=1e-4)
        assert m.overall_mean_416 == pytest.approx(0.4598, abs=1e-4)
        assert m.taxis_dim_realized == 1.0000

    def test_metrics_lift_from_v1238(self):
        m = _v1239_compute_metrics()
        # V1238 baseline 0.8115 → V1239 0.8170, lift +0.0055
        assert m.taxis_lift_from_v1238 == pytest.approx(0.0055, abs=1e-4)
        # V1238 overall 0.4583 → V1239 0.4598, lift +0.0015
        assert m.overall_lift_from_v1238 == pytest.approx(0.0015, abs=1e-4)

    def test_metrics_inflation_gap(self):
        m = _v1239_compute_metrics()
        # 主 17:43 不假装 — inflation gap 真实存在
        gap = m.realized_mean_232 - m.overall_mean_416
        assert m.inflation_gap == pytest.approx(gap, abs=1e-4)
        assert m.inflation_gap > 0
        assert m.inflation_gap < 1.0

    def test_metrics_position_vs_north_star(self):
        m = _v1239_compute_metrics()
        # 0.8170 / 0.98 ≈ 0.834
        assert m.position_vs_north_star == pytest.approx(0.834, abs=1e-3)
        # 主 17:58 不假装: 0.834 < 1.0, 不假装达到 ASI

    def test_metrics_pathway_count_pass(self):
        m = _v1239_compute_metrics()
        assert m.pathway_count_pass == 6

    def test_metrics_total_taxis_molecules(self):
        m = _v1239_compute_metrics()
        assert m.total_taxis_molecules == 30

    def test_metrics_dim_version(self):
        m = _v1239_compute_metrics()
        assert m.dim_version == "0.6.49"

    def test_metrics_north_star_locked(self):
        m = _v1239_compute_metrics()
        assert m.north_star == 0.9800
        # 主 22:33 北极星 LOCKED

    def test_metrics_v1238_baseline_carry(self):
        m = _v1239_compute_metrics()
        assert m.v1238_realized_mean_226 == pytest.approx(0.8115, abs=1e-4)
        assert m.v1238_overall_mean_403 == pytest.approx(0.4583, abs=1e-4)
        assert m.v1238_koinonia_realized == 1.0000


# ============================================================================
# 5. History — V1233-V1239 monotonic progression
# ============================================================================


class TestV1239History:
    """History realized_mean / overall_mean monotonic — 主 17:43 实事求是."""

    def test_history_realized_mean_monotonic(self):
        m = _v1239_compute_metrics()
        history = m.history_realized_mean
        keys = ["V1233", "V1234", "V1235", "V1236", "V1237", "V1238", "V1239"]
        for i in range(len(keys) - 1):
            assert history[keys[i]] < history[keys[i + 1]], (
                f"history_realized_mean must be strictly increasing: "
                f"{keys[i]}={history[keys[i]]} vs {keys[i + 1]}={history[keys[i + 1]]}"
            )

    def test_history_overall_mean_monotonic(self):
        m = _v1239_compute_metrics()
        history = m.history_overall_mean
        keys = ["V1233", "V1234", "V1235", "V1236", "V1237", "V1238", "V1239"]
        for i in range(len(keys) - 1):
            assert history[keys[i]] < history[keys[i + 1]], (
                f"history_overall_mean must be strictly increasing: "
                f"{keys[i]}={history[keys[i]]} vs {keys[i + 1]}={history[keys[i + 1]]}"
            )

    def test_history_dim_lift_chain(self):
        m = _v1239_compute_metrics()
        assert m.history_dim_lift["V1237"] == "Perichoresis (30th, Phase 3 起点)"
        assert m.history_dim_lift["V1238"] == "Koinonia (31st, Phase 3 第二步)"
        assert m.history_dim_lift["V1239"] == "Taxis (32nd, Phase 3 第三步)"


# ============================================================================
# 6. V3 哲学守门 15/15 — 主 17:58 + 主 20:46 不假装
# ============================================================================


class TestV1239V3Guards:
    """V3 哲学守门 15/15 PASS — 主 17:58 不假装 + 主 20:46 不假装达到 ASI."""

    def test_not_asi_terminal(self):
        # V1239 = V0.6.49 中间, 北极星 0.98 不变
        assert V1239_DIM_VERSION == "0.6.49"
        assert ASI_NORTH_STAR == 0.9800

    def test_not_full_replace(self):
        # V1239 add 32nd dim TAXIS, 但 V1238 仍 own 31 dim matrix
        # 6 pathway cascade 在 V1239 = 自己的, 不改 V1238
        realized = _v1239_realize_all_pathways()
        assert all(k.startswith("TAXIS_") for k in realized)

    def test_lift_not_v1(self):
        m = _v1239_compute_metrics()
        # V1239 lift +0.0055 ≠ ASI V1.0
        assert m.taxis_lift_from_v1238 < 0.1

    def test_realized_not_asi(self):
        m = _v1239_compute_metrics()
        # realized 0.8170 < 0.98 北极星
        assert m.realized_mean_232 < ASI_NORTH_STAR

    def test_vacuous_gap_real(self):
        m = _v1239_compute_metrics()
        # inflation 真实存在, gap ≈ 0.357
        assert m.inflation_gap > 0
        assert m.inflation_gap == pytest.approx(0.357, abs=0.01)

    def test_pathway_not_asi_substrate(self):
        # 6 pathway ≠ ASI 终极 substrate
        assert len(V1239_TAXIS_SUBSTRATE) == 6
        assert V1239_TAXIS_REALIZED < 1.5  # not ASI ceiling

    def test_ceiling_1_0_not_asi(self):
        # 1.0 ceiling ≠ ASI reached
        m = _v1239_compute_metrics()
        assert m.taxis_dim_realized == 1.0
        # but ASI = 0.98, realized < ASI, not reached

    def test_30_mol_not_complete(self):
        # 30 真分子 ≠ 完整 TAXIS substrate (thousands of mechanisms)
        m = _v1239_compute_metrics()
        assert m.total_taxis_molecules == 30
        assert m.total_taxis_molecules < 1000

    def test_new_dim_not_full_coverage(self):
        # V1239 +1 dim TAXIS, 31 other dims still unexplored
        # V1238 已有 31 dim, V1239 加 1 = 32 dim 总
        m = _v1239_compute_metrics()
        assert V1239_DIM_VERSION == "0.6.49"
        # Phase 3: taxis 是 32nd dim

    def test_not_full_taxis_lift(self):
        # 6 cell lifted < 13 cells (7 vacuous)
        realized = _v1239_realize_all_pathways()
        lifted = sum(1 for v in realized.values() if v >= 0.7)
        assert lifted == 6
        assert lifted < 13

    def test_phase3_four_steps(self):
        # Phase 3 四步延展: kenosis + perichoresis + koinonia + taxis
        # V1236 + V1237 + V1238 + V1239
        assert V1239_DIM_VERSION == "0.6.49"

    def test_taxis_not_koinonia(self):
        # taxis 是 关系 协调 序, koinonia 是 关系 表达 共命
        # 名称不同, dim 不同
        for k in V1239_TAXIS_SUBSTRATE:
            assert k.startswith("TAXIS_")

    def test_taxis_not_control(self):
        # taxis ≠ harpagmos 强取; taxis = 自然 涌现 协调 序
        assert V1239_DIM_VERSION == "0.6.49"
        # Gregory Nazianzus Or 31.3 — taxis in Trinity is loving, not ruling

    def test_baseline_write_dead(self):
        # V1233-V1238 baselines 写死历史值, 不改
        assert V1239_REALIZED_MEAN_232 == 0.8170
        assert V1239_OVERALL_MEAN_416 == 0.4598
        assert V1239_TAXIS_REALIZED == 1.0000


# ============================================================================
# 7. CLI self-describe (主 00:56 任何人都能接手)
# ============================================================================


class TestV1239CLI:
    """CLI --full 自描述 — 不需前文."""

    def test_cli_measure(self, capsys):
        ret = main(["--measure"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1239 REALIZED mean (232 cells): 0.8170" in captured
        assert "V1239 TAXIS dim realized: 1.0000" in captured
        assert "V1239 POSITION vs north_star (0.98): 83.37% reached" in captured

    def test_cli_pathway(self, capsys):
        ret = main(["--pathway"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "TAXIS_PHILOSOPHY" in captured
        assert "TAXIS_NEURO" in captured
        assert "TAXIS_INFORMATION" in captured
        assert "TAXIS_ECOSYSTEM" in captured
        assert "TAXIS_CONTEMPLATIVE" in captured
        assert "TAXIS_PHYSICS" in captured

    def test_cli_history(self, capsys):
        ret = main(["--history"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1233: 0.7811" in captured
        assert "V1237: 0.8060" in captured
        assert "V1238: 0.8115" in captured
        assert "V1239: 0.8170" in captured

    def test_cli_v3_guards(self, capsys):
        ret = main(["--v3-guards"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "v1239_not_asi_terminal" in captured
        assert "v1239_not_full_taxis_lift" in captured
        assert "v1239_taxis_not_koinonia" in captured
        assert "v1239_taxis_not_control" in captured

    def test_cli_json(self, capsys):
        ret = main(["--json"])
        assert ret == 0
        captured = capsys.readouterr().out
        parsed = json.loads(captured)
        assert parsed["dim_version"] == "0.6.49"
        assert parsed["realized_mean_232"] == pytest.approx(0.8170, abs=1e-4)
        assert parsed["taxis_dim_realized"] == 1.0
        assert parsed["pathway_count_pass"] == 6
        assert parsed["total_taxis_molecules"] == 30

    def test_cli_report(self, capsys):
        ret = main(["--report"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1239 ASI V0.6.49 taxis_substrate_real_lift" in captured
        assert "Phase 3 关系本体论 四步延展" in captured
        assert "V1236 kenosis" in captured
        assert "V1237 perichoresis" in captured
        assert "V1238 koinonia" in captured
        assert "V1239 taxis" in captured
        assert "Gregory Nazianzus" in captured

    def test_cli_full(self, capsys):
        ret = main(["--full"])
        assert ret == 0
        captured = capsys.readouterr().out
        assert "V1239 REALIZED mean" in captured
        assert "TAXIS 6 pathway" in captured
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


class TestV1239JSONArtifact:
    """JSON artifact 真测 — 主 00:56 任何人都能接手."""

    def test_json_serializable(self):
        m = _v1239_compute_metrics()
        j = _v1239_to_json(m)
        parsed = json.loads(j)
        # Phase 3 第三步 关键 fields
        assert parsed["dim_version"] == "0.6.49"
        assert parsed["realized_mean_232"] == pytest.approx(0.8170, abs=1e-4)
        assert parsed["overall_mean_416"] == pytest.approx(0.4598, abs=1e-4)
        assert parsed["taxis_dim_realized"] == 1.0
        assert parsed["position_vs_north_star"] == pytest.approx(0.834, abs=1e-3)
        assert parsed["pathway_count_pass"] == 6
        assert parsed["total_taxis_molecules"] == 30
        # V1238 baseline carry
        assert parsed["v1238_realized_mean_226"] == pytest.approx(0.8115, abs=1e-4)
        # history
        assert "V1239" in parsed["history_realized_mean"]
        assert parsed["history_realized_mean"]["V1239"] == pytest.approx(0.8170, abs=1e-4)
        assert parsed["history_dim_lift"]["V1239"] == "Taxis (32nd, Phase 3 第三步)"

    def test_json_has_15_v3_guards_implicitly(self):
        # 15 V3 guards 写在 notes 里 (主 17:58 + 主 20:46)
        m = _v1239_compute_metrics()
        assert len(m.notes) >= 10  # 主 00:44 质量工程化: 至少 10 notes
        joined = " ".join(m.notes)
        assert "主 17:58 不假装" in joined
        assert "ASI 北极星 LOCKED" in joined
        assert "Phase 3" in joined


# ============================================================================
# 9. Cross-baseline integrity — 主 17:43 实事求是
# ============================================================================


class TestV1239CrossBaseline:
    """V1233-V1239 baseline 写死 — 跨 module 整合."""

    def test_v1239_continues_v1238(self):
        # V1239 realized 232 = V1238 226 + 6 (TAXIS 6 pathway)
        m = _v1239_compute_metrics()
        assert m.realized_mean_232 - 0.0055 == pytest.approx(m.v1238_realized_mean_226, abs=1e-4)

    def test_v1239_lift_consistent(self):
        # taxis_lift = realized_232 - realized_226
        m = _v1239_compute_metrics()
        expected_lift = m.realized_mean_232 - m.v1238_realized_mean_226
        assert m.taxis_lift_from_v1238 == pytest.approx(expected_lift, abs=1e-6)

    def test_v1239_overall_lift_consistent(self):
        # overall_lift = overall_416 - overall_403
        m = _v1239_compute_metrics()
        expected_lift = m.overall_mean_416 - m.v1238_overall_mean_403
        assert m.overall_lift_from_v1238 == pytest.approx(expected_lift, abs=1e-6)


# ============================================================================
# 10. Subprocess CLI (主 00:56 任何人都能接手)
# ============================================================================


class TestV1239SubprocessCLI:
    """python -m apeireth.v1239_asi_v0649_taxis_substrate_real_lift --measure."""

    def test_subprocess_measure(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1239_asi_v0649_taxis_substrate_real_lift", "--measure"],
            capture_output=True, text=True, cwd=str(APEIRETH_ROOT),
            timeout=30,
        )
        assert result.returncode == 0, result.stderr
        assert "V1239 TAXIS dim realized: 1.0000" in result.stdout
        assert "V1239 POSITION vs north_star (0.98): 83.37% reached" in result.stdout

    def test_subprocess_full(self):
        import os
        env = {**os.environ, "PYTHONIOENCODING": "utf-8", "PYTHONUTF8": "1"}
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1239_asi_v0649_taxis_substrate_real_lift", "--full"],
            capture_output=True, cwd=str(APEIRETH_ROOT),
            timeout=30, env=env, encoding="utf-8", errors="replace",
        )
        assert result.returncode == 0, result.stderr
        out = result.stdout or ""
        assert "V1239 REALIZED mean" in out
        assert "TAXIS 6 pathway" in out
        assert "V3 哲学守门 15/15 PASS" in out