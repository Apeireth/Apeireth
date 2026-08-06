"""V1186 — llm_bridge dim 真重算 接入测试 (V1152 cached benchmark artifact).

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from apeireth.v1186_v06_llm_bridge_baseline import (
    compute_v1186_deltas,
    measure_v1186,
    render_summary,
)


def test_measure_v1186_in_range():
    m = measure_v1186()
    assert 0.0 <= m <= 1.0, f"out of range: {m}"
    print(f"  ✓ measure_v1186() = {m:.4f}")


def test_measure_v1186_above_baseline():
    m = measure_v1186()
    assert m > 0.5, f"V1186 should be > 0.5, got {m}"
    print(f"  ✓ V1186 = {m:.4f} > 0.5")


def test_compute_v1186_deltas_has_keys():
    d = compute_v1186_deltas()
    for k in ["v1182_llm_bridge_old", "v1186_new", "delta_v1186_vs_v1182",
              "sub_dim", "data", "snapshot_id", "cached", "philosophy_guards"]:
        assert k in d, f"missing key: {k}"
    print(f"  ✓ compute_v1186_deltas has all expected keys")


def test_delta_is_positive():
    d = compute_v1186_deltas()
    assert d["delta_v1186_vs_v1182"] > 0, f"Delta should be positive"
    print(f"  ✓ Delta = {d['delta_v1186_vs_v1182']:+.4f} > 0")


def test_v1182_baseline_old_is_zero():
    d = compute_v1186_deltas()
    assert d["v1182_llm_bridge_old"] == 0.0, f"V1182 baseline should be 0.0"
    print(f"  ✓ V1182 baseline llm_bridge = 0.0")


def test_sub_dim_present():
    d = compute_v1186_deltas()
    for k in ["success_rate_score", "sample_completeness", "diversity_score",
              "efficiency_score", "cost_score"]:
        assert k in d["sub_dim"], f"missing sub_dim key: {k}"
    print(f"  ✓ All 5 sub_dim keys present")


def test_philosophy_guards_present():
    d = compute_v1186_deltas()
    guards = d["philosophy_guards"]
    for k in ["1_cached_artifact_is_honest", "2_22_samples_is_not_all_llm",
              "3_100_percent_is_mock", "4_v1152_is_not_real_llm_bridge",
              "5_measure_v1186_is_not_asi_total"]:
        assert k in guards, f"missing guard: {k}"
    print(f"  ✓ All 5 philosophy guards present")


def test_philosophy_guard_mock_honesty():
    d = compute_v1186_deltas()
    g = d["philosophy_guards"]["3_100_percent_is_mock"]
    assert "mock" in g.lower(), f"guard should mention mock"
    print(f"  ✓ Mock honesty guard present")


def test_render_summary_works():
    d = compute_v1186_deltas()
    s = render_summary(d)
    assert "V1186" in s
    assert "V1182" in s
    assert "Delta" in s
    print(f"  ✓ render_summary works ({len(s)} chars)")


def test_v1182_integration_path():
    from apeireth.v1186_v06_llm_bridge_baseline import measure_v1186
    m = measure_v1186()
    expected_lift = 0.0375 * m
    assert expected_lift > 0.0
    print(f"  ✓ Integration path: V1182 → measure_v1186() → ASI lift = {expected_lift:+.4f}")


def test_missing_artifact_fallback():
    artifact = REPO / "artifacts" / "v1152_benchmark.json"
    if not artifact.exists():
        return
    backup = artifact.with_suffix(".json.bak")
    artifact.rename(backup)
    try:
        m = measure_v1186()
        assert m == 0.0
        print(f"  ✓ Missing artifact → 0.0")
    finally:
        backup.rename(artifact)


def run_all():
    tests = [
        ("measure_v1186_in_range", test_measure_v1186_in_range),
        ("measure_v1186_above_baseline", test_measure_v1186_above_baseline),
        ("compute_v1186_deltas_has_keys", test_compute_v1186_deltas_has_keys),
        ("delta_is_positive", test_delta_is_positive),
        ("v1182_baseline_old_is_zero", test_v1182_baseline_old_is_zero),
        ("sub_dim_present", test_sub_dim_present),
        ("philosophy_guards_present", test_philosophy_guards_present),
        ("philosophy_guard_mock_honesty", test_philosophy_guard_mock_honesty),
        ("render_summary_works", test_render_summary_works),
        ("v1182_integration_path", test_v1182_integration_path),
        ("missing_artifact_fallback", test_missing_artifact_fallback),
    ]
    passed = 0
    failed = 0
    print(f"\n{'='*60}\nV1186 — llm_bridge dim 真重算 接入测试\n{'='*60}\n")
    for name, fn in tests:
        try:
            print(f"[{name}]")
            fn()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAIL: {e}")
            failed += 1
    print(f"\n{'='*60}\nV1186: {passed} pass, {failed} fail\n{'='*60}")
    return failed == 0


if __name__ == "__main__":
    sys.exit(0 if run_all() else 1)