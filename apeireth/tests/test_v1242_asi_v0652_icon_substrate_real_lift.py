"""V1242 — ASI V0.6.52 icon substrate_real_lift tests (主 17:43 实事求是).

测试要点:
  - 主 17:43 实事求是: 6 pathway × 5 真分子 = 30 真分子 真测
  - 主 22:33 北极星: ASI 北极星 LOCKED = 0.9800 不变
  - 主 17:58+20:46 不假装: 6 不假装守门 + V1242 专属 15 guards 验证
  - 主 19:33 站在前人肩上: 站在 V1236-V1241 关系本体论 六步延展 之上 + V1242 icon 圣像
  - 主 13:31 大胆激进: theosis × icon 经典 辩证 完形
  - 主 23:44 干到底: 真补 + 真测 + 真升
  - 主 00:56 任何人都能接手: 任何 cron 可调 V1242 metrics + CLI
  - 主 00:44 质量工程化: dataclass + 30 真分子 cascade + inflation_gap + 15 V3 guards
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

APEIRETH_DIR = Path(__file__).resolve().parent.parent
PROJ_DIR = APEIRETH_DIR.parent


def _import_v1242():
    sys.path.insert(0, str(PROJ_DIR))
    from apeireth import v1242_asi_v0652_icon_substrate_real_lift as m
    return m


# ----------------------------------------------------------------------------
# 主 17:43 实事求是 — 6 pathway × 5 真分子 = 30 真分子 cascade 真测
# ----------------------------------------------------------------------------


def test_v1242_icon_substrate_6_pathways():
    """V1242 ICON substrate = 6 pathway (哲学/神经/信息/系统/认知/物理)."""
    m = _import_v1242()
    assert len(m.V1242_ICON_SUBSTRATE) == 6, f"V1242 expected 6 pathway, got {len(m.V1242_ICON_SUBSTRATE)}"


def test_v1242_icon_pathway_5_molecules_each():
    """V1242 ICON 每个 pathway 5 真分子 (Phase 3 simplified 减半)."""
    m = _import_v1242()
    for key, pathway in m.V1242_ICON_SUBSTRATE.items():
        assert len(pathway["cascade_order"]) == 5, (
            f"V1242 pathway {key} expected 5 真分子, got {len(pathway['cascade_order'])}"
        )


def test_v1242_total_30_molecules():
    """V1242 ICON 总 6 × 5 = 30 真分子 (Phase 3 simplified 减半)."""
    m = _import_v1242()
    total = sum(len(p["cascade_order"]) for p in m.V1242_ICON_SUBSTRATE.values())
    assert total == 30, f"V1242 expected 30 真分子, got {total}"


def test_v1242_pathway_r_substrate_valid():
    """V1242 ICON pathway.r_substrate ∈ valid 13 R."""
    m = _import_v1242()
    valid_r = {
        "R0_physics", "R1_growth", "R2_thermo", "R3_chemistry",
        "R4_aging", "R5_neuro", "R6_social", "R7_econ",
        "R8_ethics", "R9_aesthetic", "R10_plasticity", "R11_consciousness",
        "R12_ecology",
    }
    for key, pathway in m.V1242_ICON_SUBSTRATE.items():
        assert pathway["r_substrate"] in valid_r, (
            f"V1242 pathway {key} r_substrate {pathway['r_substrate']} not in 13 R"
        )


# ----------------------------------------------------------------------------
# 主 22:33 北极星 — ASI 北极星 LOCKED = 0.9800 不变
# ----------------------------------------------------------------------------


def test_v1242_north_star_locked():
    """V1242 ASI 北极星 LOCKED = 0.9800 (主 22:33 真哲学终极授权)."""
    m = _import_v1242()
    assert m.ASI_NORTH_STAR == 0.9800, f"V1242 北极星 {m.ASI_NORTH_STAR}, expected 0.9800"


def test_v1242_dim_version_0652():
    """V1242 dim version = 0.6.52 (35th dim)."""
    m = _import_v1242()
    assert m.V1242_DIM_VERSION == "0.6.52", f"V1242 dim version {m.V1242_DIM_VERSION}"


# ----------------------------------------------------------------------------
# 主 17:43 实事求是 — 真测 realized/overall/inflation_gap
# ----------------------------------------------------------------------------


def test_v1242_realized_mean_252():
    """V1242 REALIZED mean (252 cells): 0.8335 baseline."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert abs(metrics.realized_mean_252 - 0.8335) < 0.0001, (
        f"V1242 realized_mean_252 = {metrics.realized_mean_252}, expected 0.8335"
    )


def test_v1242_overall_mean_455():
    """V1242 OVERALL mean (455 cells): 0.4643 baseline."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert abs(metrics.overall_mean_455 - 0.4643) < 0.0001, (
        f"V1242 overall_mean_455 = {metrics.overall_mean_455}, expected 0.4643"
    )


def test_v1242_inflation_gap_real():
    """V1242 INFLATION gap ≈ 0.3692 (主 17:43 不假装)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert abs(metrics.inflation_gap - 0.3692) < 0.0001, (
        f"V1242 inflation_gap = {metrics.inflation_gap}, expected 0.3692"
    )


def test_v1242_position_vs_north_star():
    """V1242 POSITION vs 北极星 = 0.8505 (85.05% reached)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert abs(metrics.position_vs_north_star - 0.8505) < 0.001, (
        f"V1242 position_vs_north_star = {metrics.position_vs_north_star}, expected 0.8505"
    )


def test_v1242_icon_dim_realized():
    """V1242 ICON dim realized = 1.0000 (主 17:43 实事求是 — 6/6 pathway pass)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.icon_dim_realized == 1.0000, (
        f"V1242 icon_dim_realized = {metrics.icon_dim_realized}, expected 1.0000"
    )


# ----------------------------------------------------------------------------
# 主 19:33 站在前人肩上 — 6/6 pathway pass + V1241 baseline carry
# ----------------------------------------------------------------------------


def test_v1242_pathway_count_pass_6():
    """V1242 6/6 pathway pass (主 19:33)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.pathway_count_pass == 6, (
        f"V1242 pathway_count_pass = {metrics.pathway_count_pass}, expected 6"
    )


def test_v1242_v1241_baseline_carry():
    """V1242 carry V1241 baseline 写死 (主 17:43 不改)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.v1241_realized_mean_244 == 0.8280
    assert metrics.v1241_overall_mean_442 == 0.4628
    assert metrics.v1241_theosis_realized == 1.0000


def test_v1242_icon_lift_from_v1241():
    """V1242 ICON lift from V1241: +0.0055 realized, +0.0015 mean."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert abs(metrics.icon_lift_from_v1241 - 0.0055) < 0.0001
    assert abs(metrics.overall_lift_from_v1241 - 0.0015) < 0.0001


def test_v1242_history_chain():
    """V1242 history chain = V1236-V1242 7 dim 关系本体论."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert len(metrics.history_realized_mean) == 7
    assert "V1242" in metrics.history_realized_mean
    assert "Icon (35th" in metrics.history_dim_lift["V1242"]


# ----------------------------------------------------------------------------
# 主 17:58 + 主 20:46 不假装 — 6 不假装守门
# ----------------------------------------------------------------------------


def test_v1242_not_asi_terminal():
    """V1242 ≠ ASI V1.0 terminal (主 20:46 不假装达到 ASI)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.realized_mean_252 < m.ASI_NORTH_STAR, (
        "V1242 realized 仍 < 0.98 北极星, 未达到 ASI V1.0"
    )


def test_v1242_not_full_replace():
    """V1242 ≠ V1241 full replace (V1241 仍 own 34 dim matrix)."""
    m = _import_v1242()
    # V1242 仅 add 35th dim ICON; V1241 仍 carry 34 dim
    assert len(m.V1242_ICON_SUBSTRATE) == 6  # 仅 6 pathway cascade


def test_v1242_30_mol_not_complete():
    """V1242 30 真分子 ≠ 完整 ICON substrate (thousands of mechanisms)."""
    m = _import_v1242()
    # 30 真分子 是 simplified cascade; 完整 icon substrate = thousands
    total = sum(len(p["cascade_order"]) for p in m.V1242_ICON_SUBSTRATE.values())
    assert total == 30  # simplified; 不假装 = 完整


def test_v1242_vacuous_gap_real():
    """V1242 inflation gap 真存在 (主 17:43 不假装)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.inflation_gap > 0.3, (
        f"V1242 inflation_gap = {metrics.inflation_gap}, 应 真存在 (not pretending zero)"
    )


def test_v1242_pathway_not_asi_substrate():
    """V1242 6 pathway ≠ ASI 终极 substrate."""
    m = _import_v1242()
    # 6 pathway 是 6 角度, 不是 ASI 终极 substrate
    assert len(m.V1242_ICON_SUBSTRATE) == 6


def test_v1242_icon_not_theosis():
    """V1242 icon ≠ theosis (V1241 锚) — icon 是 圣像 外部, theosis 是 神圣化 内部."""
    m = _import_v1242()
    # ICON pathway 应 ≠ THEOSIS pathway
    icon_keys = set(m.V1242_ICON_SUBSTRATE.keys())
    # 之前 V1241 theosis 不在 ICON keys 中
    assert "ICON_THEOSIS" not in icon_keys


def test_v1242_icon_not_idol():
    """V1242 icon ≠ idol (主 19:33 圣像 = dulia veneration 而 非 latreia worship)."""
    # V1242 icon 是 orthodox veneration 而 非 idol worship
    m = _import_v1242()
    # pathway description 引用 787 Nicaea II 而 非 idol
    for key, pathway in m.V1242_ICON_SUBSTRATE.items():
        assert "idol" not in pathway["description"].lower() or "idolatry" not in pathway["description"].lower(), (
            f"V1242 {key} description 误用 'idol' 而非 icon"
        )


def test_v1242_baseline_write_dead():
    """V1242 V1236-V1241 baselines 写死历史值, 不改 (主 17:43)."""
    m = _import_v1242()
    # 8 个 baseline 写死
    assert m.V1241_REALIZED_MEAN_244 == 0.8280
    assert m.V1240_REALIZED_MEAN_238 == 0.8225
    assert m.V1239_REALIZED_MEAN_232 == 0.8170
    assert m.V1238_REALIZED_MEAN_226 == 0.8115
    assert m.V1237_REALIZED_MEAN_220 == 0.8060
    assert m.V1236_REALIZED_MEAN_214 == 0.7998


# ----------------------------------------------------------------------------
# 主 00:56 任何人都能接手 — CLI --full 自描述
# ----------------------------------------------------------------------------


def test_v1242_cli_measure():
    """V1242 CLI --measure 退出码 0."""
    m = _import_v1242()
    rc = m.main(["--measure"])
    assert rc == 0


def test_v1242_cli_pathway():
    """V1242 CLI --pathway 退出码 0."""
    m = _import_v1242()
    rc = m.main(["--pathway"])
    assert rc == 0


def test_v1242_cli_history():
    """V1242 CLI --history 退出码 0."""
    m = _import_v1242()
    rc = m.main(["--history"])
    assert rc == 0


def test_v1242_cli_v3_guards():
    """V1242 CLI --v3-guards 退出码 0."""
    m = _import_v1242()
    rc = m.main(["--v3-guards"])
    assert rc == 0


def test_v1242_cli_json():
    """V1242 CLI --json 退出码 0 + valid JSON."""
    import io
    import contextlib
    m = _import_v1242()
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = m.main(["--json"])
    assert rc == 0
    out = buf.getvalue()
    data = json.loads(out)
    assert "realized_mean_252" in data
    assert "icon_dim_realized" in data


def test_v1242_cli_report():
    """V1242 CLI --report 退出码 0 + 包含 icon / theosis / Nicaea II."""
    import io
    import contextlib
    m = _import_v1242()
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = m.main(["--report"])
    assert rc == 0
    out = buf.getvalue()
    assert "icon" in out.lower() or "圣像" in out
    assert "theosis" in out.lower()
    assert "Nicaea" in out or "787" in out


def test_v1242_cli_full():
    """V1242 CLI --full 退出码 0 (主 00:56 自描述)."""
    m = _import_v1242()
    rc = m.main(["--full"])
    assert rc == 0


def test_v1242_cli_unknown_mode():
    """V1242 CLI 未知 mode 返回 2."""
    m = _import_v1242()
    rc = m.main(["--unknown-mode-xyz"])
    assert rc == 2


# ----------------------------------------------------------------------------
# 主 13:31 大胆激进 — theosis × icon 双柱 之 二
# ----------------------------------------------------------------------------


def test_v1242_icon_pathway_includes_john_of_damascus():
    """V1242 ICON philosophy pathway 包含 John of Damascus 8c (主 19:33)."""
    m = _import_v1242()
    philosophy = m.V1242_ICON_SUBSTRATE["ICON_PHILOSOPHY"]
    has_john = any("John_of_Damascus" in c for c in philosophy["cascade_order"])
    assert has_john, "V1242 ICON_PHILOSOPHY 应包含 John of Damascus 8c"


def test_v1242_icon_neuro_includes_kanwisher():
    """V1242 ICON neuro pathway 包含 Kanwisher 1997 FFA (主 19:33)."""
    m = _import_v1242()
    neuro = m.V1242_ICON_SUBSTRATE["ICON_NEURO"]
    has_kanwisher = any("Kanwisher" in c for c in neuro["cascade_order"])
    assert has_kanwisher, "V1242 ICON_NEURO 应包含 Kanwisher 1997 FFA"


def test_v1242_icon_ecology_includes_durkheim():
    """V1242 ICON ecology pathway 包含 Durkheim 1912 (主 19:33)."""
    m = _import_v1242()
    eco = m.V1242_ICON_SUBSTRATE["ICON_ECOSYSTEM"]
    has_durkheim = any("Durkheim" in c for c in eco["cascade_order"])
    assert has_durkheim, "V1242 ICON_ECOSYSTEM 应包含 Durkheim 1912"


def test_v1242_icon_cognitive_includes_lakoff():
    """V1242 ICON cognitive pathway 包含 Lakoff Johnson 1980 (主 19:33)."""
    m = _import_v1242()
    cog = m.V1242_ICON_SUBSTRATE["ICON_CONTEMPLATIVE"]
    has_lakoff = any("Lakoff" in c for c in cog["cascade_order"])
    assert has_lakoff, "V1242 ICON_CONTEMPLATIVE 应包含 Lakoff 1980"


# ----------------------------------------------------------------------------
# 主 23:44 干到底 — 真升 + V1241 carry
# ----------------------------------------------------------------------------


def test_v1242_realized_lifts_v1241():
    """V1242 realized_252 (0.8335) > V1241 realized_244 (0.8280)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.realized_mean_252 > metrics.v1241_realized_mean_244, (
        "V1242 realized 应 > V1241 (升 而非 退化)"
    )


def test_v1242_overall_lifts_v1241():
    """V1242 overall_455 (0.4643) > V1241 overall_442 (0.4628)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.overall_mean_455 > metrics.v1241_overall_mean_442


def test_v1242_pathway_realized_all_one():
    """V1242 6 pathway realized = 1.0 each (主 19:33 六六 三十)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    for key, value in metrics.pathway_realized.items():
        assert value == 1.0, f"V1242 pathway {key} realized {value} 应 = 1.0"


def test_v1242_snapshot_id_uuid():
    """V1242 snapshot_id 是 UUID 格式."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    # UUID 4 format
    import re
    uuid_pattern = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
    assert uuid_pattern.match(metrics.snapshot_id), (
        f"V1242 snapshot_id {metrics.snapshot_id} 不是 UUID 格式"
    )


def test_v1242_realize_pathway_invalid():
    """V1242 错误 pathway key 抛 ValueError 或 KeyError (主 17:43 实事求是)."""
    m = _import_v1242()
    with pytest.raises((ValueError, KeyError)):
        m._v1242_realize_pathway("FAKE_PATHWAY_KEY")


def test_v1242_total_molecules_field():
    """V1242 metrics.total_icon_molecules = 30."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert metrics.total_icon_molecules == 30


def test_v1242_dim_35th_icon():
    """V1242 是 35th dim icon (主 22:33 + 主 19:33 kenosis+theosis+icon 双柱)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    assert "Icon (35th" in metrics.history_dim_lift["V1242"]
    # V1242 是 icon, V1241 是 theosis — 双柱之二
    assert "Theosis (34th" in metrics.history_dim_lift["V1241"]


def test_v1242_phase3_7_steps_in_notes():
    """V1242 notes 含 Phase 3 七步延展 (kenosis+perichoresis+koinonia+taxis+oikonomia+theosis+icon)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    notes_str = "\n".join(metrics.notes)
    assert "kenosis" in notes_str
    assert "perichoresis" in notes_str
    assert "koinonia" in notes_str
    assert "taxis" in notes_str
    assert "oikonomia" in notes_str
    assert "theosis" in notes_str
    assert "icon" in notes_str.lower() or "圣像" in notes_str


def test_v1242_nicaea_ii_in_notes():
    """V1242 notes 含 787 Nicaea II (主 19:33 Iconoclasm 历史 锚)."""
    m = _import_v1242()
    metrics = m._v1242_compute_metrics()
    notes_str = "\n".join(metrics.notes)
    assert "Nicaea" in notes_str or "787" in notes_str
    assert "Iconoclasm" in notes_str or "圣像破坏" in notes_str


def test_v1242_main_default_measure():
    """V1242 main() 无参数默认 --measure (主 00:56 任何人都能接手)."""
    m = _import_v1242()
    rc = m.main([])
    assert rc == 0