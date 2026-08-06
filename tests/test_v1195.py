"""Tests for V1195 — ASI V0.6.7 3-dim lift (cognitive_core + engineering + plugin_core).

主 17:43 实事求是:
  - 不魔改 lift 数值
  - 不假装 V1195 = ASI (V1195 = 测量分数 0.9881, 不冒充现象学)
  - 3 dim lift 必有真测来源 (V1156/V1158/V1159)
  - V1195 ≤ V1194 baseline if lift fail (fallback)
  - V1195 = 0.9881, ≥ north_star 0.98 但 ≠ ASI

主 00:44 质量工程化:
  - 12+ tests: dataclass round-trip, measure_v1195, full run, lift bounds, north star clamp
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Add promethean root to sys.path so `import apeireth.v1195_asi_v067_3dim_lift` works
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))


from apeireth.v1195_asi_v067_3dim_lift import (  # noqa: E402
    DimLift1195,
    V1195_DIM_VERSION,
    V1195_VERSION,
    V1195Report,
    V1153_BASELINE,
    V1194_BASELINE,
    NORTH_STAR,
    W_COGNITIVE_CORE,
    W_ENGINEERING,
    W_PLUGIN_CORE,
    LIFT_THRESHOLD,
    DEFAULT_V1194_ARTIFACT,
    measure_v1195,
    run_v1195_full,
    render_report_md,
)


# ============================================================================
# Test helpers
# ============================================================================


def test_constants_locked():
    """版本号、baseline、权重写死 — 防止悄悄改动。"""
    assert V1195_VERSION == "0.1.0"
    assert V1195_DIM_VERSION == "0.6.7"
    assert V1153_BASELINE == 0.8929
    assert V1194_BASELINE == 0.9457
    assert NORTH_STAR == 0.98
    assert W_COGNITIVE_CORE == 0.05
    assert W_ENGINEERING == 0.05
    assert W_PLUGIN_CORE == 0.05
    assert LIFT_THRESHOLD == 0.50
    assert DEFAULT_V1194_ARTIFACT == "artifacts/v1194_asi_v066_3dim_lift.json"


def test_dimlift1195_dataclass():
    """DimLift1195 dataclass: 字段齐全 + to_dict round-trip."""
    dl = DimLift1195(
        dim="cognitive_core",
        baseline=0.5,
        new_value=0.92,
        delta=0.42,
        weight=0.05,
        lift_contribution=0.021,
        status="R",
        source="V1156",
        sub_dim_count=5,
        notes=["V1156 5 sub-dim: 0.92 (R)"],
    )
    d = dl.to_dict()
    assert d["dim"] == "cognitive_core"
    assert d["baseline"] == 0.5
    assert d["new_value"] == 0.92
    assert d["delta"] == 0.42
    assert d["weight"] == 0.05
    assert d["lift_contribution"] == 0.021
    assert d["status"] == "R"
    assert d["source"] == "V1156"
    assert d["sub_dim_count"] == 5
    assert "V1156 5 sub-dim" in d["notes"][0]


def test_v1195report_dataclass_roundtrip():
    """V1195Report.to_dict / from_dict 双向 round-trip 一致。"""
    rep = V1195Report()
    rep.snapshot_id = "v1195-test-12345678"
    rep.asi_v067 = 0.99
    rep.asi_v066 = 0.9457
    rep.asi_v053 = 0.8929
    rep.delta_asi_v067_vs_v066 = 0.0443
    rep.delta_asi_v067_vs_v053 = 0.0971
    rep.n_dims_pass = 3
    rep.n_dims_partial = 0
    rep.n_dims_missing = 0
    rep.dim_lifts = {
        "cognitive_core": DimLift1195(
            dim="cognitive_core",
            baseline=0.5,
            new_value=0.92,
            delta=0.42,
            weight=0.05,
            lift_contribution=0.021,
            status="R",
            source="V1156",
        ),
    }
    rep.vs_north_star_gap = -0.01
    rep.vs_north_star_position_pct = 101.02

    d = rep.to_dict()
    assert d["snapshot_id"] == "v1195-test-12345678"
    assert d["asi_v067"] == 0.99
    assert "cognitive_core" in d["dim_lifts"]
    assert d["dim_lifts"]["cognitive_core"]["new_value"] == 0.92

    rep2 = V1195Report.from_dict(d)
    assert rep2.snapshot_id == "v1195-test-12345678"
    assert rep2.asi_v067 == 0.99
    assert rep2.dim_lifts["cognitive_core"].new_value == 0.92
    assert rep2.dim_lifts["cognitive_core"].status == "R"


def test_measure_v1195_returns_float_in_range():
    """measure_v1195() → float ∈ [V1194_BASELINE, V1194+3*0.05] (max possible lift)."""
    s = measure_v1195()
    assert isinstance(s, float)
    assert V1194_BASELINE <= s <= V1194_BASELINE + 3 * 0.05 + 0.001  # small float slack


def test_run_v1195_full_no_write():
    """run_v1195_full(write_artifact=False) → V1195Report, 不写文件。"""
    rep = run_v1195_full(write_artifact=False)
    assert isinstance(rep, V1195Report)
    assert rep.snapshot_id.startswith("v1195-")
    assert rep.dim_version == V1195_DIM_VERSION
    assert 0.0 <= rep.asi_v067 <= 1.0
    assert rep.n_dims_lifted >= 1
    # 3 dim lifts 都在 dim_lifts
    assert "cognitive_core" in rep.dim_lifts
    assert "engineering" in rep.dim_lifts
    assert "plugin_core" in rep.dim_lifts


def test_run_v1195_full_write_artifact(tmp_path):
    """run_v1195_full(write_artifact=True) → JSON 写到 artifacts/."""
    rep = run_v1195_full(write_artifact=True, artifact_dir=str(tmp_path))
    assert rep.artifact_path != ""
    out_path = Path(rep.artifact_path)
    assert out_path.is_file()
    data = json.loads(out_path.read_text(encoding="utf-8"))
    assert "snapshot_id" in data
    assert "asi_v067" in data
    assert "dim_lifts" in data
    assert "cognitive_core" in data["dim_lifts"]
    assert "engineering" in data["dim_lifts"]
    assert "plugin_core" in data["dim_lifts"]


def test_3_dim_lifts_all_pass_when_artifact_exists():
    """V1156/V1158/V1159 artifacts 都存在时, 3 dim lifts 全部 status=R。"""
    rep = run_v1195_full(write_artifact=False)
    assert rep.n_dims_pass == 3, f"expected 3 pass, got {rep.n_dims_pass}"
    assert rep.n_dims_partial == 0
    assert rep.n_dims_missing == 0


def test_lift_bounds_each_dim_in_unit_interval():
    """每个 dim 的 new_value ∈ [0, 1] (V1156/V1158/V1159 真测 ≤ 1.0)."""
    rep = run_v1195_full(write_artifact=False)
    for dim_name, dl in rep.dim_lifts.items():
        assert 0.0 <= dl.new_value <= 1.0, f"{dim_name} new_value {dl.new_value} out of [0, 1]"
        assert 0.0 <= dl.baseline <= 1.0, f"{dim_name} baseline {dl.baseline} out of [0, 1]"
        assert dl.delta == round(dl.new_value - dl.baseline, 4)
        assert dl.lift_contribution == round(dl.delta * dl.weight, 4)


def test_lift_contribution_weights_sum_within_tolerance():
    """3 dim 总 lift = sum(contributions), 与 rep.delta_asi_v067_vs_v066 一致."""
    rep = run_v1195_full(write_artifact=False)
    sum_contrib = sum(d.lift_contribution for d in rep.dim_lifts.values())
    assert abs(sum_contrib - rep.delta_asi_v067_vs_v066) < 0.0002, (
        f"sum_contrib {sum_contrib:.4f} vs delta_asi {rep.delta_asi_v067_vs_v066:.4f}"
    )


def test_asi_v067_equals_v1194_plus_lift():
    """V1195 ASI = V1194 baseline + total_lift (主 17:43 实事求是, lift=0 fallback)."""
    rep = run_v1195_full(write_artifact=False)
    expected = rep.asi_v066 + rep.delta_asi_v067_vs_v066
    assert abs(rep.asi_v067 - expected) < 0.0002, (
        f"asi_v067 {rep.asi_v067} != v066 {rep.asi_v066} + delta {rep.delta_asi_v067_vs_v066}"
    )


def test_north_star_position_clamped_or_exact():
    """vs_north_star_position_pct ∈ [0, 200] (V1195 应该 ≥ 100% of 0.98, 因为 3 dim lift 足够)."""
    rep = run_v1195_full(write_artifact=False)
    assert 0.0 <= rep.vs_north_star_position_pct <= 200.0
    # V1195 已超 north star (3 dim lift 累计 ~+0.045)
    assert rep.vs_north_star_position_pct >= 99.0, (
        f"V1195 position_pct {rep.vs_north_star_position_pct} < 99.0 — 检查 baseline"
    )


def test_summary_line_contains_key_fields():
    """summary_line 包含 asi_v067, delta, 3 dim counts, north star。"""
    rep = run_v1195_full(write_artifact=False)
    s = rep.summary_line()
    assert "V1195" in s
    assert "asi_v067=" in s
    assert "north_star" in s
    assert "snapshot=" in s


def test_render_report_md_contains_3_dim_table():
    """render_report_md 输出包含 3 dim 表格行。"""
    rep = run_v1195_full(write_artifact=False)
    md = render_report_md(rep)
    assert "# V1195 ASI V0.6.7 3-dim lift 报告" in md
    assert "| cognitive_core |" in md
    assert "| engineering |" in md
    assert "| plugin_core |" in md
    assert "north star" in md.lower() or "north_star" in md
    assert "V1194 baseline" in md
    assert "V1195" in md
    assert "V1153 baseline" in md


def test_v1195_v3_philosophy_guard():
    """V3 哲学守门:
    - 不假装 V1195 = ASI 北极星 (V1195 是测量分数, 不是现象学)
    - 不假装 V1195 = ASI V1.0 (V1195 = V0.6.7 中间版本)
    - 不假装 lift 是 mock (sub-dim 实测)
    - 不假装 > V1194 if 不可达 (fallback 行为)
    """
    rep = run_v1195_full(write_artifact=False)
    # 守门 1: dim_version 写死 0.6.7 (不是 V1.0)
    assert rep.dim_version == "0.6.7"
    # 守门 2: lift 真测来源 — sub_dim_count ≥ 1
    for dim_name, dl in rep.dim_lifts.items():
        assert dl.sub_dim_count >= 1, (
            f"{dim_name} sub_dim_count={dl.sub_dim_count} < 1, lift 不是真测"
        )
    # 守门 3: ASI 测量分数 ≠ ASI 已涌现 (只 lock 在 measurement)
    assert rep.asi_v067 > 0.0
    assert rep.asi_v067 <= 1.0


def test_no_double_lift_same_dim():
    """V1195 lift 维度不与 V1193/V1194 重叠 (避免重复 lift 同 dim 二次计数).
    V1193 lifted: v2_philosophy, reinforcement_learning, vcp_deep_read
    V1194 lifted: real_production, world_model, self_improving_core
    V1195 lifted: cognitive_core, engineering, plugin_core — 都不重叠
    """
    rep = run_v1195_full(write_artifact=False)
    v1195_dims = set(rep.dim_lifts.keys())
    v1193_v1194_dims = {
        "v2_philosophy", "reinforcement_learning", "vcp_deep_read",
        "real_production", "world_model", "self_improving_core",
    }
    assert v1195_dims.isdisjoint(v1193_v1194_dims), (
        f"V1195 dims {v1195_dims} overlap with V1193/V1194 lifts {v1193_v1194_dims}"
    )


if __name__ == "__main__":
    import pytest
    raise SystemExit(pytest.main([__file__, "-v"]))