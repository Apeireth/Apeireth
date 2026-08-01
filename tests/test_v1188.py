"""V1188 — ASI V0.6.3 真 baseline 全 dim 重算 接入测试.

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from apeireth.v1188_v06_v3_baseline_lifted import (
    compute_v1188_asi_lift,
    compute_v1188_full_deltas,
    measure_v1188,
    render_summary,
)


def test_measure_v1188_in_range():
    m = measure_v1188()
    assert 0.0 <= m <= 1.0, f"out of range: {m}"
    print(f"  ✓ measure_v1188() = {m:.4f}")


def test_measure_v1188_above_v1182():
    """V1188 must beat V1182 baseline (0.7425)."""
    m = measure_v1188()
    assert m > 0.7425, f"V1188 should beat V1182 (0.7425), got {m}"
    print(f"  ✓ V1188 = {m:.4f} > V1182 (0.7425)")


def test_measure_v1188_below_north_star():
    """V1188 should be below ASI north star (0.98)."""
    m = measure_v1188()
    assert m < 0.98, f"V1188 should be below north star (0.98), got {m}"
    print(f"  ✓ V1188 = {m:.4f} < north_star (0.98)")


def test_asi_lift_positive():
    """Total ASI lift should be > 0.1."""
    lift = compute_v1188_asi_lift()
    assert lift.delta_asi > 0.1, f"Expected lift > 0.1, got {lift.delta_asi}"
    print(f"  ✓ ASI lift = {lift.delta_asi:+.4f} > 0.1")


def test_4_dims_lifted():
    """All 4 v0.6_new_dim should be lifted (>0)."""
    lift = compute_v1188_asi_lift()
    for name, val in [("V1184", lift.measure_v1184), ("V1185", lift.measure_v1185),
                      ("V1186", lift.measure_v1186), ("V1187", lift.measure_v1187)]:
        assert val > 0.5, f"{name} should be > 0.5, got {val}"
    print(f"  ✓ All 4 dims lifted > 0.5")


def test_compute_full_deltas_has_keys():
    d = compute_v1188_full_deltas()
    for k in ["snapshot_id", "version", "v1182_baseline", "v1188_new",
              "deltas", "vs_asi_locked", "philosophy_guards"]:
        assert k in d, f"missing key: {k}"
    print(f"  ✓ compute_v1188_full_deltas has all expected keys")


def test_v1182_baseline_zero_for_4_dims():
    """V1182 baseline 4 dims should all be 0.0."""
    d = compute_v1188_full_deltas()
    for k in ["vcp_deep_read", "vcp_real_run", "llm_bridge", "multi_agent_dag"]:
        assert d["v1182_baseline"]["dims"][k] == 0.0, f"V1182 baseline {k} should be 0.0"
    print(f"  ✓ V1182 baseline 4 dims all = 0.0")


def test_delta_asi_matches_weight_sum():
    """Total ASI delta should match sum of dim lifts."""
    lift = compute_v1188_asi_lift()
    expected = (lift.measure_v1184 + lift.measure_v1185 + lift.measure_v1186 + lift.measure_v1187) * lift.weight_per_dim
    assert abs(lift.delta_asi - expected) < 1e-6, f"Delta mismatch: {lift.delta_asi} vs {expected}"
    print(f"  ✓ Delta ASI math correct")


def test_philosophy_guards_present():
    d = compute_v1188_full_deltas()
    guards = d["philosophy_guards"]
    for k in ["1_4_dim_lift_is_not_asi_total", "2_v1188_is_not_north_star",
              "3_cached_is_honest", "4_v1155_is_older_baseline",
              "5_v0_6_series_is_intermediate"]:
        assert k in guards, f"missing guard: {k}"
    print(f"  ✓ All 5 philosophy guards present")


def test_philosophy_guard_honesty():
    d = compute_v1188_full_deltas()
    g = d["philosophy_guards"]["2_v1188_is_not_north_star"]
    assert "0.98" in g, f"Guard should mention 0.98 (north star value)"
    print(f"  ✓ North star guard mentions 0.98")


def test_render_summary_works():
    d = compute_v1188_full_deltas()
    s = render_summary(d)
    assert "V1188" in s
    assert "V1182" in s
    assert "Delta ASI" in s
    assert "north star" in s.lower()
    print(f"  ✓ render_summary works ({len(s)} chars)")


def test_v1188_position_vs_north_star():
    """V1188 should be 80-95% of north star."""
    d = compute_v1188_full_deltas()
    pos = d["vs_asi_locked"]["v1188_position"]
    # parse XX.XX%
    pct = float(pos.replace("% of north star", ""))
    assert 80 <= pct <= 95, f"V1188 position {pct}% out of expected range"
    print(f"  ✓ V1188 position = {pct:.2f}% of north star (in expected range)")


def run_all():
    tests = [
        ("measure_v1188_in_range", test_measure_v1188_in_range),
        ("measure_v1188_above_v1182", test_measure_v1188_above_v1182),
        ("measure_v1188_below_north_star", test_measure_v1188_below_north_star),
        ("asi_lift_positive", test_asi_lift_positive),
        ("4_dims_lifted", test_4_dims_lifted),
        ("compute_full_deltas_has_keys", test_compute_full_deltas_has_keys),
        ("v1182_baseline_zero_for_4_dims", test_v1182_baseline_zero_for_4_dims),
        ("delta_asi_matches_weight_sum", test_delta_asi_matches_weight_sum),
        ("philosophy_guards_present", test_philosophy_guards_present),
        ("philosophy_guard_honesty", test_philosophy_guard_honesty),
        ("render_summary_works", test_render_summary_works),
        ("v1188_position_vs_north_star", test_v1188_position_vs_north_star),
    ]
    passed = 0
    failed = 0
    print(f"\n{'='*60}\nV1188 — ASI V0.6.3 真 baseline 全 dim 重算 接入测试\n{'='*60}\n")
    for name, fn in tests:
        try:
            print(f"[{name}]")
            fn()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAIL: {e}")
            failed += 1
    print(f"\n{'='*60}\nV1188: {passed} pass, {failed} fail\n{'='*60}")
    return failed == 0


if __name__ == "__main__":
    sys.exit(0 if run_all() else 1)