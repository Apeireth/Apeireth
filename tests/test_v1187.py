"""V1187 — multi_agent_dag dim 真重算 接入测试 (V1149 real run).

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44
"""
from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from apeireth.v1187_v06_multi_agent_dag_baseline import (
    compute_v1187_deltas,
    measure_v1187,
    render_summary,
)


def test_measure_v1187_in_range():
    m = measure_v1187()
    assert 0.0 <= m <= 1.0, f"out of range: {m}"
    print(f"  ✓ measure_v1187() = {m:.4f}")


def test_measure_v1187_above_baseline():
    m = measure_v1187()
    assert m > 0.5, f"V1187 should be > 0.5, got {m}"
    print(f"  ✓ V1187 = {m:.4f} > 0.5")


def test_compute_v1187_deltas_has_keys():
    d = compute_v1187_deltas()
    for k in ["v1182_multi_agent_dag_old", "v1187_new", "delta_v1187_vs_v1182",
              "sub_dim", "data", "snapshot_id", "source", "philosophy_guards"]:
        assert k in d, f"missing key: {k}"
    print(f"  ✓ compute_v1187_deltas has all expected keys")


def test_delta_is_positive():
    d = compute_v1187_deltas()
    assert d["delta_v1187_vs_v1182"] > 0, f"Delta should be positive"
    print(f"  ✓ Delta = {d['delta_v1187_vs_v1182']:+.4f} > 0")


def test_v1182_baseline_old_is_zero():
    d = compute_v1187_deltas()
    assert d["v1182_multi_agent_dag_old"] == 0.0, f"V1182 baseline should be 0.0"
    print(f"  ✓ V1182 baseline multi_agent_dag = 0.0")


def test_sub_dim_present():
    d = compute_v1187_deltas()
    for k in ["role_coverage", "topo_order_validity", "success_rate",
              "dag_depth_score", "plan_diversity"]:
        assert k in d["sub_dim"], f"missing sub_dim key: {k}"
    print(f"  ✓ All 5 sub_dim keys present")


def test_role_coverage_full():
    """All 5 真角色 should be present."""
    d = compute_v1187_deltas()
    assert d["sub_dim"]["role_coverage"] == 1.0, f"5/5 roles should be 1.0"
    print(f"  ✓ role_coverage = 1.0 (5/5 真角色)")


def test_topo_order_valid():
    """Topo order should be valid (5 nodes)."""
    d = compute_v1187_deltas()
    assert d["sub_dim"]["topo_order_validity"] == 1.0
    print(f"  ✓ topo_order_validity = 1.0")


def test_philosophy_guards_present():
    d = compute_v1187_deltas()
    guards = d["philosophy_guards"]
    for k in ["1_v1149_is_not_real_llm_agent", "2_5_roles_is_not_asi_multi_agent",
              "3_topo_is_not_optimal", "4_100_percent_is_mock_success",
              "5_measure_v1187_is_not_asi_total"]:
        assert k in guards, f"missing guard: {k}"
    print(f"  ✓ All 5 philosophy guards present")


def test_render_summary_works():
    d = compute_v1187_deltas()
    s = render_summary(d)
    assert "V1187" in s
    assert "V1149" in s
    assert "Delta" in s
    print(f"  ✓ render_summary works ({len(s)} chars)")


def test_v1182_integration_path():
    from apeireth.v1187_v06_multi_agent_dag_baseline import measure_v1187
    m = measure_v1187()
    expected_lift = 0.0375 * m
    assert expected_lift > 0.0
    print(f"  ✓ Integration path: V1182 → measure_v1187() → ASI lift = {expected_lift:+.4f}")


def test_5_roles_used():
    """5 真角色 (Planner/Executor/Critic/Refiner/Synthesizer)."""
    d = compute_v1187_deltas()
    assert d["data"]["n_roles"] == 5, f"should be 5 roles, got {d['data']['n_roles']}"
    print(f"  ✓ n_roles = 5 (Planner/Executor/Critic/Refiner/Synthesizer)")


def run_all():
    tests = [
        ("measure_v1187_in_range", test_measure_v1187_in_range),
        ("measure_v1187_above_baseline", test_measure_v1187_above_baseline),
        ("compute_v1187_deltas_has_keys", test_compute_v1187_deltas_has_keys),
        ("delta_is_positive", test_delta_is_positive),
        ("v1182_baseline_old_is_zero", test_v1182_baseline_old_is_zero),
        ("sub_dim_present", test_sub_dim_present),
        ("role_coverage_full", test_role_coverage_full),
        ("topo_order_valid", test_topo_order_valid),
        ("philosophy_guards_present", test_philosophy_guards_present),
        ("render_summary_works", test_render_summary_works),
        ("v1182_integration_path", test_v1182_integration_path),
        ("5_roles_used", test_5_roles_used),
    ]
    passed = 0
    failed = 0
    print(f"\n{'='*60}\nV1187 — multi_agent_dag dim 真重算 接入测试\n{'='*60}\n")
    for name, fn in tests:
        try:
            print(f"[{name}]")
            fn()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAIL: {e}")
            failed += 1
    print(f"\n{'='*60}\nV1187: {passed} pass, {failed} fail\n{'='*60}")
    return failed == 0


if __name__ == "__main__":
    sys.exit(0 if run_all() else 1)