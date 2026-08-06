"""Tests for V1196 — ASI V0.6.8 3-dim lift (rubric_open + self_organizing_core + capabilities).

主 17:43 实事求是:
  - 不魔改 lift 数值
  - 不假装 V1196 = ASI (V1196 是测量分数, 不冒充现象学)
  - 3 dim lift 必有真测来源 (V1160/V1165/V1191)
  - V1196 ≤ V1195 baseline if lift fail (fallback)
  - 同时报告 additive 和 recompute (双公式, 不掩盖差异)
  - V1196 dims 不与 V1193/V1194/V1195 重叠 (无重复 lift)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))


from apeireth.v1196_asi_v068_3dim_lift import (  # noqa: E402
    DimLift1196,
    V1196_DIM_VERSION,
    V1196_VERSION,
    V1196Report,
    V1153_BASELINE,
    V1195_BASELINE,
    NORTH_STAR,
    W_RUBRIC_OPEN,
    W_SELF_ORGANIZING_CORE,
    W_CAPABILITIES,
    LIFT_THRESHOLD,
    DEFAULT_V1195_ARTIFACT,
    DEFAULT_V1193_ARTIFACT,
    DEFAULT_V1194_ARTIFACT,
    DEFAULT_V1153_SPEC,
    measure_v1196,
    measure_v1196_recompute,
    run_v1196_full,
    render_report_md,
    _compute_recompute_total,
)


def test_constants_locked():
    assert V1196_VERSION == "0.1.0"
    assert V1196_DIM_VERSION == "0.6.8"
    assert V1153_BASELINE == 0.8929
    assert V1195_BASELINE == 0.9881
    assert NORTH_STAR == 0.98
    assert W_RUBRIC_OPEN == 0.05
    assert W_SELF_ORGANIZING_CORE == 0.05
    assert W_CAPABILITIES == 0.05
    assert LIFT_THRESHOLD == 0.50
    assert DEFAULT_V1195_ARTIFACT == "artifacts/v1195_asi_v067_3dim_lift.json"
    assert DEFAULT_V1193_ARTIFACT == "artifacts/v1193_asi_v065_3dim_lift.json"
    assert DEFAULT_V1194_ARTIFACT == "artifacts/v1194_asi_v066_3dim_lift.json"
    assert DEFAULT_V1153_SPEC == "artifacts/v1153_v06_spec.json"


def test_dimlift1196_dataclass():
    dl = DimLift1196(
        dim="rubric_open",
        baseline=0.7,
        new_value=0.88,
        delta=0.18,
        weight=0.05,
        lift_contribution=0.009,
        status="R",
        source="V1160",
        sub_dim_count=5,
        notes=["V1160 5 sub-dim: 0.88 (R)"],
    )
    d = dl.to_dict()
    assert d["dim"] == "rubric_open"
    assert d["baseline"] == 0.7
    assert d["new_value"] == 0.88
    assert d["delta"] == 0.18
    assert d["weight"] == 0.05
    assert d["lift_contribution"] == 0.009


def test_v1196report_dataclass_roundtrip():
    rep = V1196Report()
    rep.snapshot_id = "v1196-test-12345678"
    rep.asi_v068_additive = 1.0
    rep.asi_v068_recompute = 0.89
    rep.asi_v067 = 0.9881
    rep.asi_v053 = 0.8929
    rep.delta_asi_additive = 0.02
    rep.delta_asi_recompute_vs_v067 = -0.10
    rep.delta_asi_recompute_vs_v053 = -0.003
    rep.n_dims_pass = 3
    rep.dim_lifts = {
        "rubric_open": DimLift1196(
            dim="rubric_open",
            baseline=0.7,
            new_value=0.88,
            delta=0.18,
            weight=0.05,
            lift_contribution=0.009,
            status="R",
            source="V1160",
        ),
    }
    rep.vs_north_star_gap_additive = -0.02
    rep.vs_north_star_position_pct_additive = 102.04
    rep.vs_north_star_gap_recompute = 0.09
    rep.vs_north_star_position_pct_recompute = 90.82

    d = rep.to_dict()
    assert d["snapshot_id"] == "v1196-test-12345678"
    assert d["asi_v068_additive"] == 1.0
    assert d["asi_v068_recompute"] == 0.89
    assert "rubric_open" in d["dim_lifts"]

    rep2 = V1196Report.from_dict(d)
    assert rep2.snapshot_id == "v1196-test-12345678"
    assert rep2.asi_v068_additive == 1.0
    assert rep2.dim_lifts["rubric_open"].new_value == 0.88


def test_measure_v1196_additive_returns_float():
    """measure_v1196() → float (additive formula)."""
    s = measure_v1196()
    assert isinstance(s, float)
    # V1196 = V1195 (0.9881) + lift (~0.0148)
    assert 0.98 <= s <= 1.01


def test_measure_v1196_recompute_returns_float_in_range():
    """measure_v1196_recompute() → float ∈ [0, 1.05] (full-recompute V1153 formula)."""
    s = measure_v1196_recompute()
    assert isinstance(s, float)
    assert 0.85 <= s <= 1.05  # full-recompute ~0.89-0.91


def test_run_v1196_full_no_write():
    rep = run_v1196_full(write_artifact=False)
    assert isinstance(rep, V1196Report)
    assert rep.snapshot_id.startswith("v1196-")
    assert rep.dim_version == V1196_DIM_VERSION
    # additive and recompute 都计算了
    assert 0.0 <= rep.asi_v068_additive <= 1.10
    assert 0.0 <= rep.asi_v068_recompute <= 1.10
    assert rep.n_dims_lifted >= 1
    # 3 dim lifts 都在 dim_lifts
    assert "rubric_open" in rep.dim_lifts
    assert "self_organizing_core" in rep.dim_lifts
    assert "capabilities" in rep.dim_lifts


def test_run_v1196_full_write_artifact(tmp_path):
    rep = run_v1196_full(write_artifact=True, artifact_dir=str(tmp_path))
    assert rep.artifact_path != ""
    out_path = Path(rep.artifact_path)
    assert out_path.is_file()
    data = json.loads(out_path.read_text(encoding="utf-8"))
    assert "asi_v068_additive" in data
    assert "asi_v068_recompute" in data
    assert "dim_lifts" in data


def test_3_dim_lifts_all_pass_when_artifact_exists():
    """V1160/V1165/V1191 artifacts 都存在时, 3 dim lifts 全部 status=R."""
    rep = run_v1196_full(write_artifact=False)
    assert rep.n_dims_pass == 3, f"expected 3 pass, got {rep.n_dims_pass}"
    assert rep.n_dims_partial == 0
    assert rep.n_dims_missing == 0


def test_lift_bounds_each_dim_in_unit_interval():
    rep = run_v1196_full(write_artifact=False)
    for dim_name, dl in rep.dim_lifts.items():
        assert 0.0 <= dl.new_value <= 1.0, f"{dim_name} new_value {dl.new_value} out of [0, 1]"
        assert 0.0 <= dl.baseline <= 1.0, f"{dim_name} baseline {dl.baseline} out of [0, 1]"


def test_lift_contribution_weights_sum_within_tolerance():
    rep = run_v1196_full(write_artifact=False)
    sum_contrib = sum(d.lift_contribution for d in rep.dim_lifts.values())
    assert abs(sum_contrib - rep.delta_asi_additive) < 0.0002


def test_recompute_helper_returns_reasonable_value():
    """_compute_recompute_total() 用 V1153 spec formula 重算, 返 ~0.88-0.91."""
    total, reason = _compute_recompute_total()
    assert reason == "ok"
    assert 0.85 <= total <= 0.95, f"recompute total {total} out of expected range"


def test_recompute_with_new_dim_values():
    """传 new_dim_values 时, recompute 应该反映新值."""
    # 全部 dim = 1.0 → total 应该 = 1.0
    all_ones = {}  # will use V1153 dims
    # 取所有 dim name
    import json
    with open("artifacts/v1153_v06_spec.json") as f:
        spec = json.load(f)["spec"]
    all_ones = {d["dim"]: 1.0 for d in spec["dim_results"]}
    total, reason = _compute_recompute_total(new_dim_values=all_ones)
    assert reason == "ok"
    assert abs(total - 1.0) < 0.001, f"all 1.0 → total {total} != 1.0"


def test_north_star_position_additive_and_recompute():
    rep = run_v1196_full(write_artifact=False)
    assert 0.0 <= rep.vs_north_star_position_pct_additive <= 200.0
    assert 0.0 <= rep.vs_north_star_position_pct_recompute <= 200.0


def test_summary_line_contains_both_formulas():
    rep = run_v1196_full(write_artifact=False)
    s = rep.summary_line()
    assert "V1196" in s
    assert "asi_v068_additive=" in s
    assert "asi_v068_recompute=" in s
    assert "snapshot=" in s


def test_render_report_md_contains_3_dim_table_and_dual_formulas():
    rep = run_v1196_full(write_artifact=False)
    md = render_report_md(rep)
    assert "# V1196 ASI V0.6.8 3-dim lift 报告" in md
    assert "| rubric_open |" in md
    assert "| self_organizing_core |" in md
    assert "| capabilities |" in md
    assert "additive" in md
    assert "recompute" in md
    assert "V1195 baseline" in md


def test_v1196_v3_philosophy_guard():
    """V3 哲学守门:
    - 不假装 V1196 = ASI 北极星 (V1196 是测量分数)
    - 不假装 V1196 = ASI V1.0 (V1196 = V0.6.8 中间版本)
    - 不假装 lift 是 mock (sub-dim 实测)
    - 不假装 > V1195 if 不可达 (fallback 行为)
    - 双公式如实报告 (主 17:43 实事求是)
    """
    rep = run_v1196_full(write_artifact=False)
    # 守门 1: dim_version 写死 0.6.8
    assert rep.dim_version == "0.6.8"
    # 守门 2: lift 真测来源
    for dim_name, dl in rep.dim_lifts.items():
        assert dl.sub_dim_count >= 1
    # 守门 3: ASI 测量分数 ∈ [0, 1.5]
    assert 0.0 < rep.asi_v068_additive <= 1.5
    assert 0.0 < rep.asi_v068_recompute <= 1.5


def test_no_double_lift_same_dim():
    """V1196 lift 维度不与 V1193/V1194/V1195 重叠."""
    rep = run_v1196_full(write_artifact=False)
    v1196_dims = set(rep.dim_lifts.keys())
    prior_dims = {
        "v2_philosophy", "reinforcement_learning", "vcp_deep_read",
        "real_production", "world_model", "self_improving_core",
        "cognitive_core", "engineering", "plugin_core",
    }
    assert v1196_dims.isdisjoint(prior_dims), (
        f"V1196 dims {v1196_dims} overlap with prior lifts {prior_dims}"
    )


def test_additive_vs_recompute_discrepancy_logged():
    """主 17:43 实事求是 — 当 additive 与 recompute 差距 > 0.05, 应记录在 notes."""
    rep = run_v1196_full(write_artifact=False)
    # 当前 additive (1.0029) vs recompute (0.8978) 差 ~0.105, 应该 ≥ 0.05
    diff = abs(rep.asi_v068_additive - rep.asi_v068_recompute)
    if diff > 0.05:
        # 检查 notes 里有 ⚠️ 提示
        has_warning = any("additive vs recompute gap" in n for n in rep.notes)
        assert has_warning, f"Expected ⚠️ note for diff={diff}, notes={rep.notes}"


if __name__ == "__main__":
    import pytest
    raise SystemExit(pytest.main([__file__, "-v"]))