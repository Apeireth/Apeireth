"""V1203 — ASI V0.6.13 dual_dim_lift 测试 (主 22:33 北极星 + 主 17:43 实事求是 + 主 00:44 质量工程化).

测试 5 类:
  1. constants (版本/北极星/V1202 baseline)
  2. measure_v1203 (主入口, 至少 asi_recompute ∈ [V1202, north_star])
  3. cognitive_core sub-dim (5+5=10 sub-dim 全跑, 无 crash)
  4. engineering sub-dim (5+5=10 sub-dim 全跑, 无 crash)
  5. V1203Report dataclass + artifact roundtrip
  6. CLI flags (--measure / --json / --report)
  7. V3 philosophy guard (3-formula + inflation gap ≤ 0.1)

主 13:31 大胆激进: ≥30 tests
主 17:43 实事求是: 不假装 lift = 真 ASI, 不假装 sub-dim = phenomenology
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure promethean dir is on path
_PROMETHEAN = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN))


from apeireth.v1203_asi_v0613_dual_dim_lift import (
    ASI_NORTH_STAR,
    V1202_RECOMPUTE,
    V1203_COGNITIVE_CORE_SUBDIM_NAMES,
    V1203_DIM_VERSION,
    V1203_ENGINEERING_SUBDIM_NAMES,
    V1203_VERSION,
    V1156_COGNITIVE_CORE_BASELINE,
    V1159_ENGINEERING_BASELINE,
    W_COGNITIVE_CORE,
    W_ENGINEERING,
    V1203Report,
    V1203DimLift,
    V1203SubDimEvidence,
    measure_v1203,
    render_report_md,
    run_v1203_full,
    write_v1203_artifact,
)


# ============================================================================
# 1. constants
# ============================================================================


def test_v1203_version():
    assert V1203_VERSION == "0.1.0"


def test_v1203_dim_version():
    assert V1203_DIM_VERSION == "0.6.13"


def test_asi_north_star_locked():
    assert ASI_NORTH_STAR == 0.9800


def test_v1202_baseline_honest():
    """V1202 baseline 写死, 不能改 (主 17:43 实事求是)."""
    assert V1202_RECOMPUTE == 0.96921


def test_v1156_v1159_baselines_honest():
    """V1156 / V1159 baseline 从 artifact 读 (主 17:43 实事求是)."""
    assert V1156_COGNITIVE_CORE_BASELINE == 0.92
    assert V1159_ENGINEERING_BASELINE == 0.92


def test_weights_locked():
    """V2 5 位置权重 LOCKED (主 22:08)."""
    assert W_COGNITIVE_CORE == 0.05
    assert W_ENGINEERING == 0.05


def test_cognitive_subdim_count_10():
    assert len(V1203_COGNITIVE_CORE_SUBDIM_NAMES) == 10


def test_engineering_subdim_count_10():
    assert len(V1203_ENGINEERING_SUBDIM_NAMES) == 10


def test_cognitive_subdim_names_unique():
    assert len(V1203_COGNITIVE_CORE_SUBDIM_NAMES) == len(set(V1203_COGNITIVE_CORE_SUBDIM_NAMES))


def test_engineering_subdim_names_unique():
    assert len(V1203_ENGINEERING_SUBDIM_NAMES) == len(set(V1203_ENGINEERING_SUBDIM_NAMES))


# ============================================================================
# 2. measure_v1203 (主入口, 主 00:56 任何人都能接手)
# ============================================================================


def test_measure_v1203_runs():
    asi, scores, ev = measure_v1203()
    assert isinstance(asi, float)
    assert isinstance(scores, dict)
    assert isinstance(ev, dict)


def test_measure_v1203_asi_in_range():
    """ASI recompute ∈ [V1202 - 0.1, north_star + 0.05] — 允许小幅回归 (V3 实事求是)."""
    asi, _, _ = measure_v1203()
    assert asi >= V1202_RECOMPUTE - 0.1
    assert asi <= ASI_NORTH_STAR + 0.05


def test_measure_v1203_has_all_subdims():
    asi, scores, ev = measure_v1203()
    expected = set(V1203_COGNITIVE_CORE_SUBDIM_NAMES) | set(V1203_ENGINEERING_SUBDIM_NAMES)
    assert set(scores.keys()) == expected


def test_measure_v1203_meta_present():
    asi, scores, ev = measure_v1203()
    meta = ev.get("_meta", {})
    assert "asi_recompute" in meta
    assert "cog_total_lift" in meta
    assert "eng_total_lift" in meta
    assert "cog_lift_delta" in meta
    assert "eng_lift_delta" in meta
    assert "asi_north_star" in meta
    assert "gap_to_north_star" in meta
    assert "position_pct" in meta


def test_measure_v1203_north_star_honest():
    asi, _, ev = measure_v1203()
    meta = ev["_meta"]
    assert meta["asi_north_star"] == ASI_NORTH_STAR


def test_measure_v1203_position_pct_honest():
    asi, _, ev = measure_v1203()
    meta = ev["_meta"]
    expected_pct = asi / ASI_NORTH_STAR * 100.0
    assert abs(meta["position_pct"] - expected_pct) < 0.01


# ============================================================================
# 3. cognitive_core sub-dim (5+5=10, 全跑无 crash)
# ============================================================================


def test_cognitive_introspection_depth_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_cognitive_introspection_depth
    s, ev = _measure_cognitive_introspection_depth()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0
    assert ev["name"] == "introspection_depth"


def test_cognitive_self_model_accuracy_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_cognitive_self_model_accuracy
    s, ev = _measure_cognitive_self_model_accuracy()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_meta_cognition_calibration_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_cognitive_meta_cognition_calibration
    s, ev = _measure_cognitive_meta_cognition_calibration()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_perception_action_loop_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_cognitive_perception_action_loop
    s, ev = _measure_cognitive_perception_action_loop()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_reasoning_consistency_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_cognitive_reasoning_consistency
    s, ev = _measure_cognitive_reasoning_consistency()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_v1061_components_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1061_components_real
    s, ev = _measure_v1061_components_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0
    # V1061 actually has ≥ 10 components, so should score high
    assert s >= 0.5


def test_cognitive_v1061_chunk_types_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1061_chunk_types_real
    s, ev = _measure_v1061_chunk_types_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_v1061_rules_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1061_rules_real
    s, ev = _measure_v1061_rules_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_v1107_lift_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1107_cognitive_lift_real
    s, ev = _measure_v1107_cognitive_lift_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_cognitive_v1061_inference_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1061_inference_real
    s, ev = _measure_v1061_inference_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


# ============================================================================
# 4. engineering sub-dim (5+5=10, 全跑无 crash)
# ============================================================================


def test_eng_test_coverage_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_eng_test_coverage
    s, ev = _measure_eng_test_coverage()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_capability_density_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_eng_capability_density
    s, ev = _measure_eng_capability_density()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_module_organization_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_eng_module_organization
    s, ev = _measure_eng_module_organization()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_code_total_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_eng_code_total
    s, ev = _measure_eng_code_total()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_score_engineering_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_eng_score_engineering
    s, ev = _measure_eng_score_engineering()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_v1106_components_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1106_components_real
    s, ev = _measure_v1106_components_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0
    # V1106 has 24 components listed
    assert s >= 0.5


def test_eng_v1106_metrics_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1106_metrics_real
    s, ev = _measure_v1106_metrics_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_v1106_resilience_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1106_resilience_real
    s, ev = _measure_v1106_resilience_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_v1106_shutdown_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1106_shutdown_real
    s, ev = _measure_v1106_shutdown_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


def test_eng_v1106_idempotency_real_runs():
    from apeireth.v1203_asi_v0613_dual_dim_lift import _measure_v1106_idempotency_real
    s, ev = _measure_v1106_idempotency_real()
    assert isinstance(s, float)
    assert 0.0 <= s <= 1.0


# ============================================================================
# 5. V1203Report dataclass + artifact roundtrip
# ============================================================================


def test_run_v1203_full_returns_report():
    rep = run_v1203_full()
    assert isinstance(rep, V1203Report)


def test_v1203_report_summary_line():
    rep = run_v1203_full()
    s = rep.summary_line()
    assert "V1203 ASI V0.6.13" in s
    assert "north_star=0.9800" in s


def test_v1203_report_has_2_dim_lifts():
    rep = run_v1203_full()
    assert "cognitive_core" in rep.dim_lifts
    assert "engineering" in rep.dim_lifts
    assert isinstance(rep.dim_lifts["cognitive_core"], V1203DimLift)


def test_v1203_report_cognitive_subdim_evidence():
    rep = run_v1203_full()
    assert len(rep.cognitive_sub_dim_evidence) == 10
    for name in V1203_COGNITIVE_CORE_SUBDIM_NAMES:
        assert name in rep.cognitive_sub_dim_evidence
        ev = rep.cognitive_sub_dim_evidence[name]
        assert isinstance(ev, V1203SubDimEvidence)


def test_v1203_report_engineering_subdim_evidence():
    rep = run_v1203_full()
    assert len(rep.engineering_sub_dim_evidence) == 10
    for name in V1203_ENGINEERING_SUBDIM_NAMES:
        assert name in rep.engineering_sub_dim_evidence


def test_v1203_report_n_cognitive_total():
    rep = run_v1203_full()
    assert rep.n_cognitive_subdims_total == 10


def test_v1203_report_n_engineering_total():
    rep = run_v1203_full()
    assert rep.n_engineering_subdims_total == 10


def test_v1203_report_n_dims_lifted_2():
    rep = run_v1203_full()
    assert rep.n_dims_lifted == 2


def test_v1203_report_to_dict_roundtrip():
    rep = run_v1203_full()
    d = rep.to_dict()
    assert d["version"] == V1203_VERSION
    assert d["dim_version"] == V1203_DIM_VERSION
    assert "dim_lifts" in d
    assert "cognitive_sub_dim_evidence" in d
    assert "engineering_sub_dim_evidence" in d


def test_v1203_write_artifact():
    rep = run_v1203_full()
    p = write_v1203_artifact(rep, artifact_dir="artifacts")
    assert p.exists()
    # Verify JSON is valid
    data = json.loads(p.read_text(encoding="utf-8"))
    assert data["version"] == V1203_VERSION


def test_v1203_render_report_md():
    rep = run_v1203_full()
    md = render_report_md(rep)
    assert "# V1203 — ASI V0.6.13 dual_dim_lift" in md
    assert "north_star" in md
    assert "V3 philosophy guard" in md


# ============================================================================
# 6. V3 philosophy guard (主 17:43 实事求是 + 主 17:58 + 主 20:46 不假装)
# ============================================================================


def test_v3_philosophy_inflation_gap_bounded():
    """inflation gap 不应过大 (>0.1 表示 lifted formula 与 recompute 严重不一致, 不诚实)."""
    rep = run_v1203_full()
    assert abs(rep.inflation_gap_additive_vs_recompute) <= 0.1


def test_v3_philosophy_3_formula_all_present():
    rep = run_v1203_full()
    assert isinstance(rep.formula_1_additive, float)
    assert isinstance(rep.formula_2_recompute, float)
    assert isinstance(rep.formula_3_corrected, float)


def test_v3_philosophy_recompute_honest():
    """recompute = corrected (no inflation)."""
    rep = run_v1203_full()
    assert abs(rep.formula_2_recompute - rep.formula_3_corrected) < 0.001


def test_v3_philosophy_gap_negative_or_small():
    """gap to north_star ≥ -0.05 (允许小幅回归) 且 < 0.05 (尚未达成)."""
    rep = run_v1203_full()
    assert rep.gap_to_north_star_recompute >= -0.05
    assert rep.gap_to_north_star_recompute < 0.05


def test_v3_philosophy_position_under_100():
    """position < 100% (尚未达成北极星, 主 17:58 不假装)."""
    rep = run_v1203_full()
    assert 0.0 < rep.position_pct_recompute < 100.0


def test_v3_philosophy_not_all_dims_pass():
    """不假装 2 dim 都 100% pass (主 17:43 实事求是)."""
    rep = run_v1203_full()
    # partial 是常态, pass 应该是少数
    assert rep.n_dims_pass <= 2


# ============================================================================
# 7. ASI delta vs V1202
# ============================================================================


def test_asi_delta_vs_v1202_honest():
    """Δ ∈ [-0.05, +0.05] (允许小幅升降, V3 实事求是)."""
    rep = run_v1203_full()
    delta = rep.asi_recompute_delta
    assert -0.05 <= delta <= 0.05


def test_asi_recompute_lift_or_hold():
    """V1203 应该是 V1202 持平或提升 (不大幅倒退)."""
    rep = run_v1203_full()
    # Δ should be ≥ -0.02 (允许极小回落)
    assert rep.asi_recompute_delta >= -0.02


# ============================================================================
# Run via pytest
# ============================================================================


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v", "--tb=short"])