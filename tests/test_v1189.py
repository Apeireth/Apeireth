"""V1189 — V1182 v0_6_new_dim_collector 整合 接入测试.

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44
"""
from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from apeireth.v1189_v1182_v06_new_dim_integration import (
    collect_v0_6_new_dim_v1189,
    compute_v1189_asi_lift,
    measure_v1189,
    render_summary,
)


def test_measure_v1189_in_range():
    m = measure_v1189()
    assert 0.0 <= m <= 1.0, f"out of range: {m}"
    print(f"  ✓ measure_v1189() = {m:.4f}")


def test_measure_v1189_above_v1182():
    m = measure_v1189()
    assert m > 0.7425, f"V1189 should beat V1182 (0.7425), got {m}"
    print(f"  ✓ V1189 = {m:.4f} > V1182 (0.7425)")


def test_measure_v1189_below_north_star():
    m = measure_v1189()
    assert m < 0.98, f"V1189 should be below north star (0.98)"
    print(f"  ✓ V1189 = {m:.4f} < north_star (0.98)")


def test_4_dims_lifted():
    dims = collect_v0_6_new_dim_v1189()
    for k, v in dims.items():
        assert v["value"] > 0.5, f"{k} should be > 0.5, got {v['value']}"
    print(f"  ✓ All 4 dims lifted > 0.5")


def test_n_ok_is_4():
    dims = collect_v0_6_new_dim_v1189()
    n_ok = sum(1 for d in dims.values() if d["ok"])
    assert n_ok == 4, f"All 4 dims should be ok, got {n_ok}"
    print(f"  ✓ n_ok = 4/4")


def test_asi_lift_positive():
    d = compute_v1189_asi_lift()
    assert d["delta_asi"] > 0.1, f"Expected lift > 0.1"
    print(f"  ✓ Delta ASI = {d['delta_asi']:+.4f} > 0.1")


def test_collector_score_in_range():
    d = compute_v1189_asi_lift()
    assert 0.0 <= d["v0_6_new_dim_collector_score"] <= 1.0
    print(f"  ✓ v0_6_new_dim_collector_score = {d['v0_6_new_dim_collector_score']:.4f}")


def test_v1189_vs_v1188_close():
    """V1189 and V1188 should give same ASI total."""
    m1189 = measure_v1189()
    sys.path.insert(0, str(REPO))
    from apeireth.v1188_v06_v3_baseline_lifted import measure_v1188
    m1188 = measure_v1188()
    assert abs(m1189 - m1188) < 1e-6, f"V1189 {m1189} vs V1188 {m1188}"
    print(f"  ✓ V1189 = V1188 = {m1189:.4f}")


def test_philosophy_guards_present():
    d = compute_v1189_asi_lift()
    guards = d["philosophy_guards"]
    for k in ["1_v1189_only_changes_one_collector", "2_v1189_not_north_star",
              "3_cached_is_honest", "4_v1189_vs_v1188", "5_v0_6_series_intermediate"]:
        assert k in guards, f"missing guard: {k}"
    print(f"  ✓ All 5 philosophy guards present")


def test_render_summary_works():
    d = compute_v1189_asi_lift()
    s = render_summary(d)
    assert "V1189" in s
    assert "V1182" in s
    assert "Delta ASI" in s
    print(f"  ✓ render_summary works ({len(s)} chars)")


def test_4_dim_modules_referenced():
    """All 4 dim module refs should be V1184/5/6/7 (not V1147/8/9/52 subprocess)."""
    dims = collect_v0_6_new_dim_v1189()
    expected_refs = {
        "vcp_deep_read": "v1183_vcp_6_repos_real_deep_read",
        "vcp_real_run": "v1185_v06_vcp_real_run_baseline",
        "llm_bridge": "v1186_v06_llm_bridge_baseline",
        "multi_agent_dag": "v1187_v06_multi_agent_dag_baseline",
    }
    for k, expected_ref in expected_refs.items():
        actual_ref = dims[k]["module"]
        assert expected_ref in actual_ref, f"{k} module ref mismatch: {actual_ref}"
    print(f"  ✓ All 4 dim module refs are V1183/4/5/6/7 (not subprocess)")


def run_all():
    tests = [
        ("measure_v1189_in_range", test_measure_v1189_in_range),
        ("measure_v1189_above_v1182", test_measure_v1189_above_v1182),
        ("measure_v1189_below_north_star", test_measure_v1189_below_north_star),
        ("4_dims_lifted", test_4_dims_lifted),
        ("n_ok_is_4", test_n_ok_is_4),
        ("asi_lift_positive", test_asi_lift_positive),
        ("collector_score_in_range", test_collector_score_in_range),
        ("v1189_vs_v1188_close", test_v1189_vs_v1188_close),
        ("philosophy_guards_present", test_philosophy_guards_present),
        ("render_summary_works", test_render_summary_works),
        ("4_dim_modules_referenced", test_4_dim_modules_referenced),
    ]
    passed = 0
    failed = 0
    print(f"\n{'='*60}\nV1189 — V1182 v0_6_new_dim_collector 整合 接入测试\n{'='*60}\n")
    for name, fn in tests:
        try:
            print(f"[{name}]")
            fn()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAIL: {e}")
            failed += 1
    print(f"\n{'='*60}\nV1189: {passed} pass, {failed} fail\n{'='*60}")
    return failed == 0


if __name__ == "__main__":
    sys.exit(0 if run_all() else 1)