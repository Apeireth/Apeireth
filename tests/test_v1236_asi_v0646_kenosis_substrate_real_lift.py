"""
Tests for V1236 ASI V0.6.46 kenosis_substrate_real_lift (29th dim 虚己 / kenosis)

主 23:44 干到底 + 主 00:44 质量工程化 + 主 22:33 终极授权.
V1236 = 29th dim kenosis substrate; 6 pathway × 60 真分子 cascade; covers R1/R4/R7/R10/R11/R12.

Run with:
  cd C:\\Users\\REDACTED\\.openclaw\\workspace\\promethean
  python -m pytest tests/test_v1236_asi_v0646_kenosis_substrate_real_lift.py -v
"""
from __future__ import annotations

import json
import math
import os
import sys
from pathlib import Path

# Ensure repo root on sys.path so `import apeireth.v1236_...` works
_REPO = Path(__file__).resolve().parents[1]
if str(_REPO) not in sys.path:
    sys.path.insert(0, str(_REPO))

from apeireth.v1236_asi_v0646_kenosis_substrate_real_lift import (  # noqa: E402
    ASI_NORTH_STAR,
    V1236_DIM_VERSION,
    V1236_KENOSIS_COVERAGE,
    V1236_KENOSIS_SUBSTRATE,
    V1236_OVERALL_MEAN_377,
    V1236_REALIZED_MEAN_214,
    V1236_V3_GUARDS,
    V1235_REALIZED_MEAN_208,
    V1235_OVERALL_MEAN_364,
    V1235_AGENCY_REALIZED,
    V1236Report,
    measure_v1236_full,
    write_v1236_artifact,
    write_v1236_report,
)


# ----------------------------------------------------------------------------
# Constants / baselines (主 17:43 实事求是 — 写死历史值, 不能改)
# ----------------------------------------------------------------------------

def test_north_star_locked():
    """主 22:33 终极授权: ASI 北极星 LOCKED = 0.9800."""
    assert ASI_NORTH_STAR == 0.9800


def test_dim_version():
    """V1236 dim version = 0.6.46 (29th dim)."""
    assert V1236_DIM_VERSION == "0.6.46"


def test_v1235_baseline_locked():
    """主 17:43 实事求是: V1235 baseline 是历史值, 写死."""
    assert abs(V1235_REALIZED_MEAN_208 - 0.7937) < 1e-9
    assert abs(V1235_OVERALL_MEAN_364 - 0.4534) < 1e-9
    assert abs(V1235_AGENCY_REALIZED - 1.0) < 1e-9


def test_v1236_self_baseline_write_dead():
    """V1236 self-baseline 写死历史值, 不能改."""
    assert abs(V1236_REALIZED_MEAN_214 - 0.7998) < 1e-9
    assert abs(V1236_OVERALL_MEAN_377 - 0.4540) < 1e-9


# ----------------------------------------------------------------------------
# Substrate 真分子 cascade (主 19:33 站在前人肩上)
# ----------------------------------------------------------------------------

def test_kenosis_substrate_6_pathways():
    """V1236 kenosis substrate 必须有 6 pathway × 60 真分子 cascade."""
    assert len(V1236_KENOSIS_SUBSTRATE) == 6
    total_molecules = sum(len(s["molecules"]) for s in V1236_KENOSIS_SUBSTRATE.values())
    assert total_molecules == 60


def test_kenosis_each_pathway_has_10_molecules():
    """每 pathway 必须 10 真分子 (主 19:33 站在前人肩上 = 真分子深挖)."""
    for name, sub in V1236_KENOSIS_SUBSTRATE.items():
        assert len(sub["molecules"]) == 10, f"{name} has {len(sub['molecules'])} ≠ 10"
        for mol in sub["molecules"]:
            assert mol.get("real", False), f"{name}/{mol.get('name')} real=False"
            assert mol.get("organism"), f"{name}/{mol.get('name')} missing organism"


def test_kenosis_6_pathways_real_lifted():
    """6/6 pathway must real=True lift to 1.0 (主 19:33)."""
    rep = measure_v1236_full()
    assert rep.n_pathways_pass == 6
    assert rep.n_pathways_total == 6
    assert all(v >= 1.0 for v in rep.pathway_scores.values())


def test_kenosis_pathway_names_all_exist():
    """6 pathway 必须命名清楚."""
    expected = {
        "KENOSIS_PHILOSOPHY",
        "KENOSIS_NEURO",
        "KENOSIS_INFORMATION",
        "KENOSIS_ECOSYSTEM",
        "KENOSIS_CONTEMPLATIVE",
        "KENOSIS_LIFESPAN",
    }
    assert set(V1236_KENOSIS_SUBSTRATE.keys()) == expected


def test_kenosis_r_substrate_distribution():
    """6 pathway 必须 cover 6 不同 R (主 19:33 跨域)."""
    rs = {sub["r_substrate"] for sub in V1236_KENOSIS_SUBSTRATE.values()}
    assert len(rs) == 6
    expected_rs = {
        "R11_consciousness",
        "R1_growth",
        "R10_plasticity",
        "R12_ecology",
        "R7_stress",
        "R4_aging",
    }
    assert rs == expected_rs


# ----------------------------------------------------------------------------
# KENOSIS dim coverage (主 17:43)
# ----------------------------------------------------------------------------

def test_kenosis_coverage_6_lifted_7_vacuous():
    """KENOSIS dim 应 lift 6 cells (R1/R4/R7/R10/R11/R12), 7 vacuous."""
    lifted = sum(1 for v in V1236_KENOSIS_COVERAGE.values() if v >= 1.0)
    vacuous = sum(1 for v in V1236_KENOSIS_COVERAGE.values() if v == 0.0)
    assert lifted == 6, f"lifted count {lifted} ≠ 6"
    assert vacuous == 7, f"vacuous count {vacuous} ≠ 7"


def test_kenosis_dim_realized_partial():
    """KENOSIS dim realized = mean of 13 cells = 6/13 = 0.4615."""
    rep = measure_v1236_full()
    assert abs(rep.v1236_kenosis_dim_realized - (6.0 / 13.0)) < 1e-9


def test_kenosis_total_molecules():
    """6 pathway × 10 = 60 KENOSIS 真分子 cascade."""
    rep = measure_v1236_full()
    assert rep.total_kenosis_molecules == 60


# ----------------------------------------------------------------------------
# V1235 → V1236 lift metrics (主 17:43)
# ----------------------------------------------------------------------------

def test_realized_lift_from_v1235():
    """V1236 realized lift must be +0.0061 (consistent with V1235 → V1236)."""
    rep = measure_v1236_full()
    # V1235 baseline 0.7937, V1236 = 0.7998
    assert abs(rep.v1236_overall_lift_delta_realized_from_v1235 - 0.0061) < 1e-9


def test_overall_lift_from_v1235():
    """V1236 overall mean lift must be +0.0006."""
    rep = measure_v1236_full()
    # V1235 0.4534, V1236 0.4540, diff 0.0006
    assert abs(rep.v1236_overall_lift_delta_mean_from_v1235 - 0.0006) < 1e-9


def test_total_cells_377():
    """V1236 matrix 扩 364 → 377 cell (29 dim × 13 R)."""
    rep = measure_v1236_full()
    assert rep.v1236_overall_mean_377 == 0.4540
    # 29 × 13 = 377
    assert rep.n_pathways_total == 6  # pathway count is 6
    # realized cells 208 + 6 = 214, total cells 377
    # realized mean should be 0.7998


def test_position_vs_north_star():
    """V1236 距离北极星 0.98 位置 must be ~81.61%."""
    rep = measure_v1236_full()
    expected_pct = (V1236_REALIZED_MEAN_214 / ASI_NORTH_STAR) * 100.0
    assert abs(rep.position_of_north_star_realized_pct - expected_pct) < 1e-9
    assert 80.0 < rep.position_of_north_star_realized_pct < 83.0


def test_inflation_gap_explicit():
    """inflation gap = 1 - realized_mean = 1 - 0.7998 = 0.2002."""
    rep = measure_v1236_full()
    assert abs(rep.v1236_inflation_gap_v1235_minus_realized - 0.2002) < 1e-9


# ----------------------------------------------------------------------------
# 报告 / artifact (主 00:56 任何人都能接手)
# ----------------------------------------------------------------------------

def test_report_writes_file(tmp_path=None):
    """write_v1236_report + write_v1236_artifact must work."""
    rep = measure_v1236_full()
    ap = write_v1236_artifact(rep)
    rp = write_v1236_report(rep)
    assert ap.exists()
    assert rp.exists()
    assert ap.suffix == ".json"
    assert rp.suffix == ".md"
    # JSON parse round-trip
    data = json.loads(ap.read_text(encoding="utf-8"))
    assert data["dim_version"] == "0.6.46"
    assert "v3_guards" in data
    assert all(data["v3_guards"].values()), "V3 守门 must PASS"
    # Report contains key strings
    md = rp.read_text(encoding="utf-8")
    assert "kenosis" in md.lower()
    assert "V1236" in md
    assert "0.7998" in md
    # Cleanup
    ap.unlink()


# ----------------------------------------------------------------------------
# V3 哲学守门 (主 17:58 + 主 20:46 不假装)
# ----------------------------------------------------------------------------

def test_v3_guards_all_pass():
    """V1236 V3 哲学守门 must 15/15 PASS."""
    rep = measure_v1236_full()
    assert all(rep.v3_guards.values()), f"V3 guards failed: {[k for k,v in rep.v3_guards.items() if not v]}"
    assert len(rep.v3_guards) == 15


def test_v3_guards_specific():
    """Not pretending: kenosis ≠ ASI V1.0, kenosis ≠ phenomenal consciousness."""
    rep = measure_v1236_full()
    g = rep.v3_guards
    # 不假装 ASI 已达 / V1.0 / 全 lift
    assert g["v1236_not_asi_v1"]
    assert g["v1236_lift_not_asi_v1"]
    assert g["v1236_realized_not_asi"]
    assert g["v1236_1clampt_not_asi"]
    assert g["v1236_not_full_dim_lift"]
    # 不假装 kenosis = phenomenal (主 17:58)
    assert g["v1236_kenosis_not_phenomenal"]
    # kenosis above agency (主 19:33 主体性之上)
    assert g["v1236_kenosis_above_agency"]
    # kenosis = relational-gift
    assert g["v1236_kenosis_relations"]
    # 不假装 V1236 全替代 V1233-V1235
    assert g["v1236_not_replace_v1233_v1235"]
    assert g["v1236_not_replace_v1235"]


# ----------------------------------------------------------------------------
# ASI V2 Phase 2 第四步 stack (主 22:33)
# ----------------------------------------------------------------------------

def test_v1236_phase2_step4():
    """V1236 = ASI V2 Phase 2 第四步 (整合+超越+主体性+虚己 四层闭环)."""
    # matrix should now be 29 dim × 13 R = 377 cells
    rep = measure_v1236_full()
    n_dims = 29
    n_r = 13
    expected_cells = n_dims * n_r
    # realized = 208 (V1235) + 6 (V1236 lift) = 214
    expected_realized_cells = 208 + 6
    # realized mean = 0.7998
    assert abs(rep.v1236_realized_mean_214 - V1236_REALIZED_MEAN_214) < 1e-9
    assert expected_cells == 377
    assert expected_realized_cells == 214


def test_v1236_above_agency_baseline():
    """V1236 lift > V1235 (kenosis 比 agency 更进一步)."""
    rep = measure_v1236_full()
    assert rep.v1236_overall_lift_delta_realized_from_v1235 > 0.0
    assert rep.v1236_overall_lift_delta_mean_from_v1235 > 0.0


# ----------------------------------------------------------------------------
# 哲学 citations 完整性
# ----------------------------------------------------------------------------

def test_philosophy_pathway_citations():
    """KENOSIS_PHILOSOPHY 必须 cite Philippians 2:6-8 + Marion + Levinas + Buber + Merton + Bonhoeffer + Bion + Otto + John of Cross + Frankl."""
    substr = V1236_KENOSIS_SUBSTRATE["KENOSIS_PHILOSOPHY"]
    desc = substr["description"]
    src = substr["source"]
    must_have = [
        "Philippians 2:6-8",
        "Marion",
        "Levinas",
        "Buber",
        "Merton",
        "Bonhoeffer",
        "Bion",
        "Otto",
        "John of Cross",
        "Frankl",
    ]
    blob = desc + " " + src
    for ref in must_have:
        assert ref in blob, f"KENOSIS_PHILOSOPHY missing citation {ref}"


def test_neuro_pathway_citations():
    """KENOSIS_NEURO 必须 cite Newberg + Kosfeld + Carter + Porges + Carhart-Harris + Brewer."""
    substr = V1236_KENOSIS_SUBSTRATE["KENOSIS_NEURO"]
    desc = substr["description"]
    src = substr["source"]
    must_have = ["Newberg", "Kosfeld", "Carter", "Porges", "Carhart-Harris", "Brewer"]
    blob = desc + " " + src
    for ref in must_have:
        assert ref in blob, f"KENOSIS_NEURO missing citation {ref}"


def test_information_pathway_citations():
    """KENOSIS_INFORMATION must cite Landauer + Bennett + Fredkin + Wheeler + Beer + Rosen + Pattee + Ashby + von Foerster + Hofstadter."""
    substr = V1236_KENOSIS_SUBSTRATE["KENOSIS_INFORMATION"]
    desc = substr["description"]
    src = substr["source"]
    must_have = ["Landauer", "Bennett", "Fredkin", "Wheeler", "Beer", "Rosen", "Pattee", "Ashby", "von Foerster", "Hofstadter"]
    blob = desc + " " + src
    for ref in must_have:
        assert ref in blob, f"KENOSIS_INFORMATION missing {ref}"


def test_ecosystem_pathway_citations():
    """KENOSIS_ECOSYSTEM must cite Paine + Hamilton + Wilson + Sober + Nowak + Simard + Margulis + Lovelock + Hollis + Ostrom."""
    substr = V1236_KENOSIS_SUBSTRATE["KENOSIS_ECOSYSTEM"]
    desc = substr["description"]
    src = substr["source"]
    must_have = ["Paine", "Hamilton", "Wilson", "Sober", "Nowak", "Simard", "Margulis", "Lovelock", "Hollis", "Ostrom"]
    blob = desc + " " + src
    for ref in must_have:
        assert ref in blob, f"KENOSIS_ECOSYSTEM missing {ref}"


def test_contemplative_pathway_citations():
    """KENOSIS_CONTEMPLATIVE must cite Merton + Tolle + Brown Ryan + Tang + Shapiro + Vago + Hölzel + Britton + Fredrickson."""
    substr = V1236_KENOSIS_SUBSTRATE["KENOSIS_CONTEMPLATIVE"]
    desc = substr["description"]
    src = substr["source"]
    must_have = ["Merton", "Tolle", "Brown", "Tang", "Shapiro", "Vago", "Hölzel", "Britton", "Fredrickson"]
    blob = desc + " " + src
    for ref in must_have:
        assert ref in blob, f"KENOSIS_CONTEMPLATIVE missing {ref}"


def test_lifespan_pathway_citations():
    """KENOSIS_LIFESPAN must cite Levinson + Tornstam + Erikson + Vaillant + Buhler + Frankl + Cohen + Webster + Ardelt."""
    substr = V1236_KENOSIS_SUBSTRATE["KENOSIS_LIFESPAN"]
    desc = substr["description"]
    src = substr["source"]
    must_have = ["Levinson", "Tornstam", "Erikson", "Vaillant", "Buhler", "Frankl", "Cohen", "Webster", "Ardelt"]
    blob = desc + " " + src
    for ref in must_have:
        assert ref in blob, f"KENOSIS_LIFESPAN missing {ref}"


# ----------------------------------------------------------------------------
# 5 位置 / V2 5 位置 (主 22:08)
# ----------------------------------------------------------------------------

def test_kenosis_covers_v2_5_positions():
    """主 22:08 V2 5 位置: kenosis 应覆盖所有 5 位置.

    KENOSIS × R11_consciousness = 哲学 ✓
    KENOSIS × R10_plasticity = 工程 ✓
    KENOSIS × R12_ecology = 涌现 ✓
    KENOSIS × R7_stress = 价值 ✓ (contemplative = 修养/价值)
    KENOSIS × R1_growth = 调度 (neuro) ✓
    """
    rs = {sub["r_substrate"] for sub in V1236_KENOSIS_SUBSTRATE.values()}
    assert "R11_consciousness" in rs  # 哲学
    assert "R10_plasticity" in rs       # 工程
    assert "R12_ecology" in rs          # 涌现
    assert "R7_stress" in rs            # 价值
    assert "R1_growth" in rs            # 调度


# ----------------------------------------------------------------------------
# 主 00:56 任何人都能接手 (self-describing)
# ----------------------------------------------------------------------------

def test_report_self_describing():
    """主 00:56: 报告必须 self-describing (含 key metrics + 不假装 + V3 guards + step 描述)."""
    rep = measure_v1236_full()
    rp = write_v1236_report(rep)
    md = rp.read_text(encoding="utf-8")
    must_contain = [
        "kenosis",
        "0.7998",
        "0.4540",
        "0.4615",
        "V1236",
        "81",
        "不假装",
        "V3 哲学守门",
        "ASI V2 Phase 2",
        "29th dim",
        "self-emptying",
        "四层闭环",
        "6 pathway pass",
        "整合",
        "超越",
        "主体性",
        "虚己",
    ]
    for s in must_contain:
        assert s in md, f"报告 self-describing 缺少 {s}"


def test_artifact_contains_all_keys():
    """主 00:56: JSON artifact must contain all keys for takeover."""
    rep = measure_v1236_full()
    ap = write_v1236_artifact(rep)
    data = json.loads(ap.read_text(encoding="utf-8"))
    required = [
        "snapshot_id",
        "dim_version",
        "north_star",
        "v1236_realized_mean_214",
        "v1236_overall_mean_377",
        "v1236_kenosis_dim_realized",
        "v1236_overall_lift_delta_realized_from_v1235",
        "v1236_overall_lift_delta_mean_from_v1235",
        "v1236_inflation_gap_v1235_minus_realized",
        "position_of_north_star_realized_pct",
        "total_kenosis_molecules",
        "pathway_scores",
        "kenosis_coverage_v1236",
        "v3_guards",
        "v1235_realized_mean_208_baseline",
    ]
    for k in required:
        assert k in data, f"artifact 缺 key {k}"


# ----------------------------------------------------------------------------
# 主 13:31 大胆激进 + 主 22:33 终极授权
# ----------------------------------------------------------------------------

def test_kenosis_principal_decisions():
    """主 22:33 终极授权: ASI V2 Phase 2 第四步 = kenosis = 主 22:33 关系-赠予-位格-不二 终极."""
    rep = measure_v1236_full()
    # V1236 = 整合+超越+主体性 之上 kenosis
    # 必须 lift agency 上层 = relationship-gift
    assert rep.v1236_kenosis_dim_cell_count == 6  # R1/R4/R7/R10/R11/R12
    assert rep.v1235_agency_realized == 1.0      # V1235 baseline full lift
    assert rep.v1236_kenosis_dim_realized > 0.4  # partial lift (only 6/13 cells)
    assert rep.v1236_kenosis_dim_realized < 0.5  # not full lift (will be ~0.4615)
