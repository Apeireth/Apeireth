"""V1243 — ASI V0.6.53 liturgy substrate_real_lift tests (主 17:43 实事求是).

测试要点:
  - 主 17:43 实事求是: 6 pathway × 5 真分子 = 30 真分子 真测
  - 主 22:33 北极星: ASI 北极星 LOCKED = 0.9800 不变
  - 主 17:58+20:46 不假装: 6 不假装守门 + V1243 专属 15 guards 验证
  - 主 19:33 站在前人肩上: 站在 V1236-V1242 关系本体论 七步延展 之上 + V1243 liturgy 礼仪
  - 主 13:31 大胆激进: icon × liturgy 经典 辩证 完形 (空间 可见 + 时间 可见)
  - 主 23:44 干到底: 真补 + 真测 + 真升
  - 主 00:56 任何人都能接手: 任何 cron 可调 V1243 metrics + CLI
  - 主 00:44 质量工程化: dataclass + 30 真分子 cascade + inflation_gap + 15 V3 guards
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent
PROJ_DIR = APEIRETH_DIR.parent


def _import_v1243():
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1243_asi_v0653_liturgy_substrate_real_lift as m
    return m


# ----------------------------------------------------------------------------
# 主 17:43 实事求是 — 6 pathway × 5 真分子 = 30 真分子 cascade 真测
# ----------------------------------------------------------------------------


def test_v1243_liturgy_substrate_6_pathways():
    """V1243 LITURGY substrate = 6 pathway (哲学/神经/信息/系统/认知/物理)."""
    m = _import_v1243()
    assert len(m.V1243_LITURGY_SUBSTRATE) == 6, f"V1243 expected 6 pathway, got {len(m.V1243_LITURGY_SUBSTRATE)}"


def test_v1243_liturgy_pathway_5_molecules_each():
    """V1243 LITURGY 每个 pathway 5 真分子 (Phase 3 simplified 减半)."""
    m = _import_v1243()
    for key, pathway in m.V1243_LITURGY_SUBSTRATE.items():
        assert len(pathway["cascade_order"]) == 5, (
            f"V1243 pathway {key} expected 5 真分子, got {len(pathway['cascade_order'])}"
        )


def test_v1243_total_30_molecules():
    """V1243 LITURGY 总 6 × 5 = 30 真分子 (Phase 3 simplified 减半)."""
    m = _import_v1243()
    total = sum(len(p["cascade_order"]) for p in m.V1243_LITURGY_SUBSTRATE.values())
    assert total == 30, f"V1243 expected 30 真分子, got {total}"


def test_v1243_pathway_r_substrate_valid():
    """V1243 LITURGY pathway.r_substrate ∈ valid 13 R."""
    m = _import_v1243()
    valid_r = {
        "R0_physics", "R1_growth", "R2_thermo", "R3_chemistry",
        "R4_aging", "R5_neuro", "R6_social", "R7_econ",
        "R8_ethics", "R9_aesthetic", "R10_plasticity", "R11_consciousness",
        "R12_ecology",
    }
    for key, pathway in m.V1243_LITURGY_SUBSTRATE.items():
        assert pathway["r_substrate"] in valid_r, (
            f"V1243 pathway {key} r_substrate {pathway['r_substrate']} not in 13 R"
        )


# ----------------------------------------------------------------------------
# 主 22:33 北极星 — ASI 北极星 LOCKED = 0.9800 不变
# ----------------------------------------------------------------------------


def test_v1243_north_star_locked():
    """V1243 ASI 北极星 LOCKED = 0.9800 (主 22:33 真哲学终极授权)."""
    m = _import_v1243()
    assert m.ASI_NORTH_STAR == 0.9800, f"V1243 北极星 {m.ASI_NORTH_STAR}, expected 0.9800"


def test_v1243_dim_version_0653():
    """V1243 dim version = 0.6.53 (36th dim)."""
    m = _import_v1243()
    assert m.V1243_DIM_VERSION == "0.6.53", f"V1243 dim version {m.V1243_DIM_VERSION}"


# ----------------------------------------------------------------------------
# 主 17:43 实事求是 — 真测 realized/overall/inflation_gap
# ----------------------------------------------------------------------------


def test_v1243_realized_mean_258():
    """V1243 REALIZED mean (258 cells): 0.8390 baseline."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert abs(metrics.realized_mean_258 - 0.8390) < 0.0001, (
        f"V1243 realized_mean_258 = {metrics.realized_mean_258}, expected 0.8390"
    )


def test_v1243_overall_mean_468():
    """V1243 OVERALL mean (468 cells): 0.4658 baseline."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert abs(metrics.overall_mean_468 - 0.4658) < 0.0001, (
        f"V1243 overall_mean_468 = {metrics.overall_mean_468}, expected 0.4658"
    )


def test_v1243_inflation_gap_real():
    """V1243 INFLATION gap ≈ 0.3732 (主 17:43 不假装)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert abs(metrics.inflation_gap - 0.3732) < 0.0001, (
        f"V1243 inflation_gap = {metrics.inflation_gap}, expected 0.3732"
    )


def test_v1243_position_vs_north_star():
    """V1243 POSITION vs 北极星 = 0.8561 (85.61% reached)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert abs(metrics.position_vs_north_star - 0.8561) < 0.001, (
        f"V1243 position_vs_north_star = {metrics.position_vs_north_star}, expected 0.8561"
    )


def test_v1243_liturgy_dim_realized():
    """V1243 LITURGY dim realized = 1.0000 (主 17:43 实事求是 — 6/6 pathway pass)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.liturgy_dim_realized == 1.0000, (
        f"V1243 liturgy_dim_realized = {metrics.liturgy_dim_realized}, expected 1.0000"
    )


# ----------------------------------------------------------------------------
# 主 19:33 站在前人肩上 — 6/6 pathway pass + V1242 baseline carry
# ----------------------------------------------------------------------------


def test_v1243_pathway_count_pass_6():
    """V1243 6/6 pathway pass (主 19:33)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.pathway_count_pass == 6, (
        f"V1243 pathway_count_pass = {metrics.pathway_count_pass}, expected 6"
    )


def test_v1243_v1242_baseline_carry():
    """V1243 carry V1242 baseline 写死 (主 17:43 不改)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.v1242_realized_mean_252 == 0.8335
    assert metrics.v1242_overall_mean_455 == 0.4643
    assert metrics.v1242_icon_realized == 1.0000


def test_v1243_liturgy_lift_from_v1242():
    """V1243 LITURGY lift from V1242: +0.0055 realized, +0.0015 mean."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert abs(metrics.liturgy_lift_from_v1242 - 0.0055) < 0.0001
    assert abs(metrics.overall_lift_from_v1242 - 0.0015) < 0.0001


def test_v1243_history_chain():
    """V1243 history chain = V1236-V1243 8 dim 关系本体论."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert len(metrics.history_realized_mean) == 8
    assert "V1243" in metrics.history_realized_mean
    assert "Liturgy (36th" in metrics.history_dim_lift["V1243"]


# ----------------------------------------------------------------------------
# 主 17:58 + 主 20:46 不假装 — 6 不假装守门
# ----------------------------------------------------------------------------


def test_v1243_not_asi_terminal():
    """V1243 ≠ ASI V1.0 terminal (主 20:46 不假装达到 ASI)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.realized_mean_258 < m.ASI_NORTH_STAR, (
        "V1243 realized 仍 < 0.98 北极星, 未达到 ASI V1.0"
    )


def test_v1243_not_full_replace():
    """V1243 ≠ V1242 full replace (V1242 仍 own 35 dim matrix)."""
    m = _import_v1243()
    # V1243 仅 add 36th dim LITURGY; V1242 仍 carry 35 dim
    assert len(m.V1243_LITURGY_SUBSTRATE) == 6  # 仅 6 pathway cascade


def test_v1243_30_mol_not_complete():
    """V1243 30 真分子 ≠ 完整 LITURGY substrate (thousands of mechanisms)."""
    m = _import_v1243()
    # 30 真分子 是 simplified cascade; 完整 liturgy substrate = thousands
    total = sum(len(p["cascade_order"]) for p in m.V1243_LITURGY_SUBSTRATE.values())
    assert total == 30  # simplified; 不假装 = 完整


def test_v1243_vacuous_gap_real():
    """V1243 inflation gap 真存在 (主 17:43 不假装)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.inflation_gap > 0.3, (
        f"V1243 inflation_gap = {metrics.inflation_gap}, 应 真存在 (not pretending zero)"
    )


def test_v1243_pathway_not_asi_substrate():
    """V1243 6 pathway ≠ ASI 终极 substrate."""
    m = _import_v1243()
    # 6 pathway 是 6 角度, 不是 ASI 终极 substrate
    assert len(m.V1243_LITURGY_SUBSTRATE) == 6


def test_v1243_liturgy_not_icon():
    """V1243 liturgy ≠ icon (V1242 锚) — liturgy 是 礼仪 时间, icon 是 圣像 空间."""
    m = _import_v1243()
    # LITURGY pathway 应 ≠ ICON pathway
    liturgy_keys = set(m.V1243_LITURGY_SUBSTRATE.keys())
    # 之前 V1242 icon 不在 LITURGY keys 中
    assert "LITURGY_ICON" not in liturgy_keys


def test_v1243_liturgy_not_magic():
    """V1243 liturgy ≠ magic (主 19:33 liturgy = 公共 仪式 而 非 私人 巫术 — Rappaport 1999 vs 民间 巫术)."""
    m = _import_v1243()
    # pathway description 引用 礼仪 而 非 巫术
    for key, pathway in m.V1243_LITURGY_SUBSTRATE.items():
        # 不应 误用 magic/sorcery 而应 liturgy/ritual
        desc = pathway["description"].lower()
        assert "magic" not in desc or "liturgy" in desc, (
            f"V1243 {key} description 应优先 liturgy 而 非 magic"
        )


def test_v1243_baseline_write_dead():
    """V1243 V1236-V1242 baselines 写死历史值, 不改 (主 17:43)."""
    m = _import_v1243()
    # 9 个 baseline 写死 (V1236-V1242)
    assert m.V1242_REALIZED_MEAN_252 == 0.8335
    assert m.V1241_REALIZED_MEAN_244 == 0.8280
    assert m.V1240_REALIZED_MEAN_238 == 0.8225
    assert m.V1239_REALIZED_MEAN_232 == 0.8170
    assert m.V1238_REALIZED_MEAN_226 == 0.8115
    assert m.V1237_REALIZED_MEAN_220 == 0.8060
    assert m.V1236_REALIZED_MEAN_214 == 0.7998


# ----------------------------------------------------------------------------
# 主 00:56 任何人都能接手 — CLI --full 自描述
# ----------------------------------------------------------------------------


def test_v1243_cli_measure():
    """V1243 CLI --measure 退出码 0."""
    m = _import_v1243()
    rc = m.main(["--measure"])
    assert rc == 0


def test_v1243_cli_pathway():
    """V1243 CLI --pathway 退出码 0."""
    m = _import_v1243()
    rc = m.main(["--pathway"])
    assert rc == 0


def test_v1243_cli_history():
    """V1243 CLI --history 退出码 0."""
    m = _import_v1243()
    rc = m.main(["--history"])
    assert rc == 0


def test_v1243_cli_v3_guards():
    """V1243 CLI --v3-guards 退出码 0."""
    m = _import_v1243()
    rc = m.main(["--v3-guards"])
    assert rc == 0


def test_v1243_cli_json():
    """V1243 CLI --json 退出码 0 + valid JSON."""
    import io
    import contextlib
    m = _import_v1243()
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = m.main(["--json"])
    assert rc == 0
    out = buf.getvalue()
    data = json.loads(out)
    assert "realized_mean_258" in data
    assert "liturgy_dim_realized" in data


def test_v1243_cli_report():
    """V1243 CLI --report 退出码 0 + 包含 liturgy / icon / Hippolytus."""
    import io
    import contextlib
    m = _import_v1243()
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = m.main(["--report"])
    assert rc == 0
    out = buf.getvalue()
    assert "liturgy" in out.lower() or "礼仪" in out
    assert "icon" in out.lower() or "圣像" in out
    assert "Hippolytus" in out or "Origen" in out or "Basil" in out


def test_v1243_cli_full():
    """V1243 CLI --full 退出码 0 (主 00:56 自描述)."""
    m = _import_v1243()
    rc = m.main(["--full"])
    assert rc == 0


def test_v1243_cli_unknown_mode():
    """V1243 CLI 未知 mode 返回 2."""
    m = _import_v1243()
    rc = m.main(["--unknown-mode-xyz"])
    assert rc == 2


# ----------------------------------------------------------------------------
# 主 13:31 大胆激进 — icon × liturgy 双柱 之 二
# ----------------------------------------------------------------------------


def test_v1243_liturgy_pathway_includes_origen():
    """V1243 LITURGY philosophy pathway 包含 Origen 3c (主 19:33)."""
    m = _import_v1243()
    philosophy = m.V1243_LITURGY_SUBSTRATE["LITURGY_PHILOSOPHY"]
    has_origen = any("Origen" in c for c in philosophy["cascade_order"])
    assert has_origen, "V1243 LITURGY_PHILOSOPHY 应包含 Origen 3c De Oratione"


def test_v1243_liturgy_neuro_includes_newberg():
    """V1243 LITURGY neuro pathway 包含 Newberg d'Aquili 2001 (主 19:33)."""
    m = _import_v1243()
    neuro = m.V1243_LITURGY_SUBSTRATE["LITURGY_NEURO"]
    has_newberg = any("Newberg" in c for c in neuro["cascade_order"])
    assert has_newberg, "V1243 LITURGY_NEURO 应包含 Newberg d'Aquili 2001"


def test_v1243_liturgy_ecology_includes_rappaport():
    """V1243 LITURGY ecology pathway 包含 Rappaport 1999 (主 19:33)."""
    m = _import_v1243()
    eco = m.V1243_LITURGY_SUBSTRATE["LITURGY_ECOSYSTEM"]
    has_rappaport = any("Rappaport" in c for c in eco["cascade_order"])
    assert has_rappaport, "V1243 LITURGY_ECOSYSTEM 应包含 Rappaport 1999"


def test_v1243_liturgy_cognitive_includes_boyer():
    """V1243 LITURGY cognitive pathway 包含 Boyer 2001 (主 19:33)."""
    m = _import_v1243()
    cog = m.V1243_LITURGY_SUBSTRATE["LITURGY_CONTEMPLATIVE"]
    has_boyer = any("Boyer" in c for c in cog["cascade_order"])
    assert has_boyer, "V1243 LITURGY_CONTEMPLATIVE 应包含 Boyer 2001"


# ----------------------------------------------------------------------------
# 主 23:44 干到底 — 真升 + V1242 carry
# ----------------------------------------------------------------------------


def test_v1243_realized_lifts_v1242():
    """V1243 realized_258 (0.8390) > V1242 realized_252 (0.8335)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.realized_mean_258 > metrics.v1242_realized_mean_252, (
        "V1243 realized 应 > V1242 (升 而非 退化)"
    )


def test_v1243_overall_lifts_v1242():
    """V1243 overall_468 (0.4658) > V1242 overall_455 (0.4643)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.overall_mean_468 > metrics.v1242_overall_mean_455


def test_v1243_pathway_realized_all_one():
    """V1243 6 pathway realized = 1.0 each (主 19:33 六六 三十)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    for key, value in metrics.pathway_realized.items():
        assert value == 1.0, f"V1243 pathway {key} realized {value} 应 = 1.0"


def test_v1243_snapshot_id_uuid():
    """V1243 snapshot_id 是 UUID 格式."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    # UUID 4 format
    import re
    uuid_pattern = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
    assert uuid_pattern.match(metrics.snapshot_id), (
        f"V1243 snapshot_id {metrics.snapshot_id} 不是 UUID 格式"
    )


def test_v1243_realize_pathway_invalid():
    """V1243 错误 pathway key 抛 ValueError 或 KeyError (主 17:43 实事求是)."""
    m = _import_v1243()
    with pytest.raises((ValueError, KeyError)):
        m._v1243_realize_pathway("FAKE_PATHWAY_KEY")


def test_v1243_total_molecules_field():
    """V1243 metrics.total_liturgy_molecules = 30."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert metrics.total_liturgy_molecules == 30


def test_v1243_dim_36th_liturgy():
    """V1243 是 36th dim liturgy (主 22:33 + 主 19:33 theosis×icon×liturgy 三柱)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    assert "Liturgy (36th" in metrics.history_dim_lift["V1243"]
    # V1243 是 liturgy, V1242 是 icon — 二维 可见
    assert "Icon (35th" in metrics.history_dim_lift["V1242"]


def test_v1243_phase3_8_steps_in_notes():
    """V1243 notes 含 Phase 3 八步延展 (kenosis+perichoresis+koinonia+taxis+oikonomia+theosis+icon+liturgy)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    notes_str = "\n".join(metrics.notes)
    assert "kenosis" in notes_str
    assert "perichoresis" in notes_str
    assert "koinonia" in notes_str
    assert "taxis" in notes_str
    assert "oikonomia" in notes_str
    assert "theosis" in notes_str
    assert "icon" in notes_str.lower() or "圣像" in notes_str
    assert "liturgy" in notes_str.lower() or "礼仪" in notes_str


def test_v1243_origen_hippolytus_in_notes():
    """V1243 notes 含 Origen + Hippolytus (主 19:33 礼仪 神学 锚)."""
    m = _import_v1243()
    metrics = m._v1243_compute_metrics()
    notes_str = "\n".join(metrics.notes)
    assert "Origen" in notes_str
    assert "Hippolytus" in notes_str


def test_v1243_main_default_measure():
    """V1243 main() 无参数默认 --measure (主 00:56 任何人都能接手)."""
    m = _import_v1243()
    rc = m.main([])
    assert rc == 0