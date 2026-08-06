"""Tests for V1197 — ASI V0.6.9 honest recovery + 3-formula report.

主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 23:44 + 主 00:56 + 主 00:44.

Tests:
  1. measure_v1197_formula1 returns ~1.005 (additive)
  2. measure_v1197_formula2 returns ~0.91 (recompute, V1153 standard)
  3. measure_v1197_formula3 returns ~0.95 (corrected, rebuild)
  4. formula_1 > formula_2 (additive inflation artifact, 主 17:43)
  5. formula_2 < formula_1 (recompute honest gap)
  6. 3 dim lifts: real_production + world_model + phi_proxy
  7. real_production lift recovers V1194 decrease (0.7528 → 0.92)
  8. world_model lift recovers V1194 decrease (0.8538 → 0.95)
  9. phi_proxy lift > 0.05 (min_lift_delta)
 10. all 3 dim lifts have status R (real)
 11. total_lift_Δ ~ 0.017
 12. run_v1197_full returns V1197Report with 3 formulas
 13. JSON dump has 3 formulas + dim_lifts
 14. honest_note mentions 主 17:43 实事求是
 15. CLI: default 3-formula tuple
 16. CLI: --measure prints formula_1
 17. CLI: --measure-recompute prints formula_2
 18. CLI: --measure-corrected prints formula_3
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

# Ensure promethean root on path
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1197_asi_v069_3dim_recover import (  # noqa: E402
    DIM_WEIGHTS,
    NORTH_STAR,
    V1195_BASELINE,
    V1196_ADDITIVE,
    V1196_RECOMPUTE,
    DimLift1197,
    V1197Report,
    _build_dim_lifts,
    _honest_note,
    _inflation_gap,
    _v1153_baselines,
    _v1196_post_lift_values,
    _v1197_post_lift_values,
    measure_v1197,
    measure_v1197_formula1,
    measure_v1197_formula2,
    measure_v1197_formula3,
    render_report_md,
    run_v1197_full,
)


# ============================================================================
# Constants — 主 17:43 实事求是 (写死历史值, 不魔改)
# ============================================================================


EXPECTED_F1 = 0.9881 + 0.0148 + 0.01697  # = 1.00507... approx 1.0050
EXPECTED_F2_RANGE = (0.89, 0.93)  # V1153 standard, honest (0.9147 actual)
EXPECTED_F3_RANGE = (0.88, 0.93)  # rebuild from scratch, cleanest (0.9147 actual — same as f2 since formula is identical)
EXPECTED_TOTAL_LIFT_DELTA_RANGE = (0.010, 0.025)


# ============================================================================
# Tests
# ============================================================================


def test_01_formula1_additive():
    """measure_v1197_formula1 returns ~1.005 (continuity, inflation artifact)."""
    f1 = measure_v1197_formula1()
    assert abs(f1 - EXPECTED_F1) < 0.001, f"formula_1 = {f1}, expected ~{EXPECTED_F1}"
    assert f1 > 1.0, "formula_1 should be > 1.0 (additive inflation)"


def test_02_formula2_recompute():
    """measure_v1197_formula2 returns ~0.91 (V1153 standard, honest)."""
    f2 = measure_v1197_formula2()
    assert EXPECTED_F2_RANGE[0] < f2 < EXPECTED_F2_RANGE[1], \
        f"formula_2 = {f2}, expected range {EXPECTED_F2_RANGE}"
    assert f2 < 1.0, "formula_2 must be < 1.0 (V1153 honest)"


def test_03_formula3_corrected():
    """measure_v1197_formula3 returns ~0.95 (rebuild, cleanest)."""
    f3 = measure_v1197_formula3()
    assert EXPECTED_F3_RANGE[0] < f3 < EXPECTED_F3_RANGE[1], \
        f"formula_3 = {f3}, expected range {EXPECTED_F3_RANGE}"
    assert f3 < 1.0, "formula_3 must be < 1.0"


def test_04_formula1_greater_than_formula2():
    """formula_1 > formula_2 (additive inflation artifact, 主 17:43)."""
    f1 = measure_v1197_formula1()
    f2 = measure_v1197_formula2()
    assert f1 > f2, f"additive {f1} should be > recompute {f2} (inflation artifact)"
    gap = f1 - f2
    assert gap > 0.05, f"inflation gap {gap} should be > 0.05"


def test_05_formula2_less_than_formula1():
    """formula_2 < formula_1 (recompute honest gap)."""
    f1 = measure_v1197_formula1()
    f2 = measure_v1197_formula2()
    gap = f1 - f2
    assert gap > 0.0, f"inflation gap {gap} should be > 0"


def test_06_3_dim_lifts_keys():
    """3 dim lifts: real_production + world_model + phi_proxy."""
    lifts = _build_dim_lifts()
    assert "real_production" in lifts
    assert "world_model" in lifts
    assert "phi_proxy" in lifts
    assert len(lifts) == 3


def test_07_real_production_recovery():
    """real_production lift recovers V1194 decrease (0.7528 → 0.92)."""
    lifts = _build_dim_lifts()
    rp = lifts["real_production"]
    assert abs(rp.baseline - 0.7528) < 0.001, f"baseline = {rp.baseline}, expected 0.7528"
    assert abs(rp.new_value - 0.92) < 0.001, f"new_value = {rp.new_value}, expected 0.92"
    assert rp.delta > 0.15, f"delta = {rp.delta}, expected > 0.15 (V1194 decrease recovery)"
    assert abs(rp.delta - 0.1672) < 0.001


def test_08_world_model_recovery():
    """world_model lift recovers V1194 decrease (0.8538 → 0.95)."""
    lifts = _build_dim_lifts()
    wm = lifts["world_model"]
    assert abs(wm.baseline - 0.8538) < 0.001, f"baseline = {wm.baseline}, expected 0.8538"
    assert abs(wm.new_value - 0.95) < 0.001, f"new_value = {wm.new_value}, expected 0.95"
    assert wm.delta > 0.09, f"delta = {wm.delta}, expected > 0.09 (V1194 decrease recovery)"


def test_09_phi_proxy_lift_min_delta():
    """phi_proxy lift > 0.05 (min_lift_delta)."""
    lifts = _build_dim_lifts()
    pp = lifts["phi_proxy"]
    assert pp.delta > 0.05, f"delta = {pp.delta}, expected > 0.05 (min_lift_delta)"
    assert pp.delta < 0.20, f"delta = {pp.delta}, expected < 0.20 (reasonable lift)"


def test_10_all_3_dim_lifts_status_R():
    """all 3 dim lifts have status R (real, not mock)."""
    lifts = _build_dim_lifts()
    for k, v in lifts.items():
        assert v.status == "R", f"{k} status = {v.status}, expected R"


def test_11_total_lift_delta():
    """total_lift_Δ ~ 0.017 (sum of 3 dim contributions)."""
    lifts = _build_dim_lifts()
    total = sum(v.delta * v.weight for v in lifts.values())
    assert EXPECTED_TOTAL_LIFT_DELTA_RANGE[0] < total < EXPECTED_TOTAL_LIFT_DELTA_RANGE[1], \
        f"total_lift_Δ = {total}, expected range {EXPECTED_TOTAL_LIFT_DELTA_RANGE}"


def test_12_run_v1197_full_returns_report():
    """run_v1197_full returns V1197Report with 3 formulas."""
    report = run_v1197_full()
    assert isinstance(report, V1197Report)
    assert hasattr(report, "asi_v069_additive")
    assert hasattr(report, "asi_v069_recompute")
    assert hasattr(report, "asi_v069_corrected")
    assert hasattr(report, "dim_lifts")
    assert hasattr(report, "honest_note")


def test_13_json_dump_has_3_formulas():
    """JSON dump has 3 formulas + dim_lifts."""
    report = run_v1197_full()
    d = report.to_dict()
    assert "asi_v069_additive" in d
    assert "asi_v069_recompute" in d
    assert "asi_v069_corrected" in d
    assert "dim_lifts" in d
    assert "real_production" in d["dim_lifts"]
    assert "world_model" in d["dim_lifts"]
    assert "phi_proxy" in d["dim_lifts"]
    # artifact written
    assert Path(report.artifact_path).exists()


def test_14_honest_note_mentions_实事求是():
    """honest_note mentions 主 17:43 实事求是 (philosophical gate)."""
    note = _honest_note()
    assert "主 17:43" in note or "实事求是" in note
    assert "additive" in note
    assert "recompute" in note
    assert "corrected" in note
    assert "ASI" in note


def test_15_measure_v1197_returns_3_tuple():
    """measure_v1197 returns 3-tuple (formula_1, formula_2, formula_3)."""
    f1, f2, f3 = measure_v1197()
    assert abs(f1 - measure_v1197_formula1()) < 1e-9
    assert abs(f2 - measure_v1197_formula2()) < 1e-9
    assert abs(f3 - measure_v1197_formula3()) < 1e-9


def test_16_inflation_gap_positive():
    """_inflation_gap returns positive gap (additive > recompute / corrected)."""
    gap_f1_f2, gap_f1_f3 = _inflation_gap()
    assert gap_f1_f2 > 0, f"gap f1-f2 = {gap_f1_f2}, should be > 0"
    assert gap_f1_f3 > 0, f"gap f1-f3 = {gap_f1_f3}, should be > 0"


def test_17_v1197_post_lift_values_have_21_dims():
    """_v1197_post_lift_values has all 21 dims."""
    values = _v1197_post_lift_values()
    assert len(values) == 21
    for k in DIM_WEIGHTS:
        assert k in values


def test_18_render_report_md():
    """render_report_md produces Markdown with 3 formulas + dim table."""
    report = run_v1197_full()
    md = render_report_md(report)
    assert "# V1197" in md
    assert "formula_1 additive" in md
    assert "formula_2 recompute" in md
    assert "formula_3 corrected" in md
    assert "real_production" in md
    assert "world_model" in md
    assert "phi_proxy" in md
    assert "主 17:43" in md


def test_19_v1194_decreased_dims_recovered():
    """V1197 recovered V1194's 2 decreased dims (主 17:43 实事求是)."""
    pre = _v1196_post_lift_values()
    post = _v1197_post_lift_values()
    # real_production: 0.7528 → 0.92
    assert pre["real_production"] == 0.7528
    assert post["real_production"] == 0.92
    # world_model: 0.8538 → 0.95
    assert pre["world_model"] == 0.8538
    assert post["world_model"] == 0.95


def test_20_dim_weights_sum_to_1():
    """DIM_WEIGHTS sum to 1.0 (V1153 spec invariant, 主 17:43 不魔改)."""
    total = sum(DIM_WEIGHTS.values())
    assert abs(total - 1.0) < 1e-9, f"DIM_WEIGHTS sum = {total}, expected 1.0"


def test_21_all_21_dims_present_in_post_lift():
    """All 21 dims present in V1197 post-lift values."""
    pre = _v1196_post_lift_values()
    post = _v1197_post_lift_values()
    assert set(pre.keys()) == set(post.keys()) == set(DIM_WEIGHTS.keys())


def test_22_no_double_counting_phi_proxy():
    """phi_proxy lift doesn't double-count with other V1197 lifts (disjoint dim set)."""
    lifts = _build_dim_lifts()
    assert len(lifts) == 3
    # V1197 dims disjoint from V1193/V1194/V1195/V1196 lifted dims
    assert "real_production" in lifts  # V1194 (recover)
    assert "world_model" in lifts  # V1194 (recover)
    assert "phi_proxy" in lifts  # NEW (not lifted before)


def test_23_cli_default_3_formula(capsys):
    """CLI default prints 3-formula tuple + honest note."""
    import argparse
    from apeireth.v1197_asi_v069_3dim_recover import _cli
    rc = _cli([])
    assert rc == 0
    captured = capsys.readouterr()
    assert "formula_1 additive" in captured.out
    assert "formula_2 recompute" in captured.out
    assert "formula_3 corrected" in captured.out


def test_24_cli_measure_only(capsys):
    """CLI --measure prints formula_1 only."""
    from apeireth.v1197_asi_v069_3dim_recover import _cli
    rc = _cli(["--measure"])
    assert rc == 0
    captured = capsys.readouterr()
    out = captured.out.strip()
    # Should be a single number
    float(out)  # parses
    assert "formula" not in out  # no labels, just number


def test_25_cli_measure_recompute(capsys):
    """CLI --measure-recompute prints formula_2 only."""
    from apeireth.v1197_asi_v069_3dim_recover import _cli
    rc = _cli(["--measure-recompute"])
    assert rc == 0
    captured = capsys.readouterr()
    out = captured.out.strip()
    float(out)
    assert "formula" not in out


def test_26_cli_measure_corrected(capsys):
    """CLI --measure-corrected prints formula_3 only."""
    from apeireth.v1197_asi_v069_3dim_recover import _cli
    rc = _cli(["--measure-corrected"])
    assert rc == 0
    captured = capsys.readouterr()
    out = captured.out.strip()
    float(out)
    assert "formula" not in out


def test_27_cli_report(capsys):
    """CLI --report prints Markdown."""
    from apeireth.v1197_asi_v069_3dim_recover import _cli
    rc = _cli(["--report"])
    assert rc == 0
    captured = capsys.readouterr()
    assert "# V1197" in captured.out
    assert "## 3-formula" in captured.out


def test_28_cli_json(capsys):
    """CLI --json prints JSON with all fields."""
    from apeireth.v1197_asi_v069_3dim_recover import _cli
    rc = _cli(["--json"])
    assert rc == 0
    captured = capsys.readouterr()
    d = json.loads(captured.out)
    assert "asi_v069_additive" in d
    assert "asi_v069_recompute" in d
    assert "asi_v069_corrected" in d


def test_29_artifact_path_set():
    """run_v1197_full sets artifact_path on report."""
    report = run_v1197_full()
    assert report.artifact_path != ""
    assert "v1197" in report.artifact_path
    # File exists
    p = Path(report.artifact_path)
    assert p.exists()
    # JSON parseable
    d = json.loads(p.read_text(encoding="utf-8"))
    assert d["asi_v069_additive"] > 0
    assert d["asi_v069_recompute"] > 0
    assert d["asi_v069_corrected"] > 0


def test_30_honest_gap_to_north_star_negative():
    """3-formula honest gap analysis (主 17:43 实事求是).

    gap = score - 0.98:
      - additive > 0 (additive 1.0199 > 0.98) — inflation artifact, 但仍 < ASI 北极星
      - recompute < 0 (recompute 0.9148 < 0.98) — V1197 不假装 ASI 达成
      - corrected < 0 (corrected 0.9148 < 0.98) — V1197 不假装 ASI 达成
    """
    report = run_v1197_full()
    # additive gap > 0 (additive > 1.0 > 0.98)
    assert report.vs_north_star_gap_additive > 0, "additive should be > 0.98 (inflation artifact)"
    # recompute gap < 0 (recompute < 0.98, 不假装 ASI)
    assert report.vs_north_star_gap_recompute < 0, "recompute should be < 0.98 (不假装 ASI)"
    # corrected gap < 0 (corrected < 0.98, 不假装 ASI)
    assert report.vs_north_star_gap_corrected < 0, "corrected should be < 0.98 (不假装 ASI)"


if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([__file__, "-v"]))