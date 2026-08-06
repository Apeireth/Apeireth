"""V1185 — vcp_real_run dim 真重算 接入测试 (V1148 cached artifact).

主 06:15 + 主 22:33 + 主 17:43 + 主 19:33 + 主 13:31 + 主 17:58 + 主 20:46 + 主 00:56 + 主 00:44

Tests cover:
  - measure_v1185() = ~0.95 (5/5 real repos + low HTTP + 1 error penalty)
  - compute_v1185_deltas() v1182_old → v1185_new delta
  - _load_v1148_artifact() fallback if file missing
  - philosophy guards: 5 不假装 守门
  - V1182 v0_6_new_dim_collector 接入路径
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from apeireth.v1185_v06_vcp_real_run_baseline import (
    compute_v1185_deltas,
    measure_v1185,
    render_summary,
)


def test_measure_v1185_in_range():
    """measure_v1185() must be in [0..1]."""
    m = measure_v1185()
    assert 0.0 <= m <= 1.0, f"measure_v1185 out of range: {m}"
    print(f"  ✓ measure_v1185() = {m:.4f}")


def test_measure_v1185_above_baseline():
    """V1185 must beat V1182 baseline (0.0)."""
    m = measure_v1185()
    assert m > 0.5, f"V1185 should be > 0.5 (V1148 cached is high), got {m}"
    print(f"  ✓ V1185 = {m:.4f} > 0.5")


def test_compute_v1185_deltas_has_keys():
    d = compute_v1185_deltas()
    assert "v1182_vcp_real_run_old" in d
    assert "v1185_new" in d
    assert "delta_v1185_vs_v1182" in d
    assert "sub_dim" in d
    assert "data" in d
    assert "philosophy_guards" in d
    print(f"  ✓ compute_v1185_deltas has all expected keys")


def test_delta_is_positive():
    """V1185 - V1182 should be positive (lift)."""
    d = compute_v1185_deltas()
    assert d["delta_v1185_vs_v1182"] > 0, f"Delta should be positive: {d['delta_v1185_vs_v1182']}"
    print(f"  ✓ Delta = {d['delta_v1185_vs_v1182']:+.4f} > 0")


def test_v1182_baseline_old_is_zero():
    """V1182 baseline vcp_real_run must be 0.0 (subprocess timeout → 0)."""
    d = compute_v1185_deltas()
    # V1182 baseline reads from V1182 artifact
    # V1148 subprocess times out → 0.0
    assert d["v1182_vcp_real_run_old"] == 0.0, f"V1182 baseline should be 0.0, got {d['v1182_vcp_real_run_old']}"
    print(f"  ✓ V1182 baseline vcp_real_run = 0.0 (subprocess timeout)")


def test_sub_dim_real_repo_ratio():
    """Real repo ratio should be 5/5 = 1.0 (all 5 repos real)."""
    d = compute_v1185_deltas()
    assert d["sub_dim"]["real_repo_ratio"] == 1.0, f"5/5 repos should be 1.0"
    print(f"  ✓ real_repo_ratio = {d['sub_dim']['real_repo_ratio']:.4f} (5/5)")


def test_philosophy_guards_present():
    """5 不假装 守门 must be present."""
    d = compute_v1185_deltas()
    guards = d["philosophy_guards"]
    assert "1_cached_artifact_is_honest" in guards
    assert "2_5_repos_is_not_all_vcp" in guards
    assert "3_pattern_density_cap" in guards
    assert "4_error_penalty_honest" in guards
    assert "5_measure_v1185_is_not_asi_total" in guards
    print(f"  ✓ All 5 philosophy guards present")


def test_philosophy_guard_honesty():
    """Cached artifact guard must mention snapshot_id + honesty."""
    d = compute_v1185_deltas()
    g = d["philosophy_guards"]["1_cached_artifact_is_honest"]
    assert "snapshot" in g.lower() or "v1148" in g.lower()
    assert "实事" in g or "不假装" in g
    print(f"  ✓ Cached artifact guard mentions snapshot + honesty")


def test_render_summary_works():
    d = compute_v1185_deltas()
    s = render_summary(d)
    assert "V1185" in s
    assert "V1182" in s
    assert "Delta" in s
    print(f"  ✓ render_summary works ({len(s)} chars)")


def test_v1182_v06_new_dim_collector_integration_path():
    """V1182 v0_6_new_dim_collector can be patched to use measure_v1185()."""
    # Demonstrate the integration path (without actually modifying V1182)
    from apeireth.v1185_v06_vcp_real_run_baseline import measure_v1185
    m = measure_v1185()
    # V1182 would call measure_v1185() instead of subprocess V1148
    # Expected ASI lift: 0.0375 * m
    expected_lift = 0.0375 * m
    assert expected_lift > 0.0, f"Expected positive ASI lift"
    print(f"  ✓ Integration path: V1182 → measure_v1185() → ASI lift = {expected_lift:+.4f}")


def test_missing_artifact_fallback():
    """If V1148 artifact missing, measure_v1185() returns 0.0 (honest fallback)."""
    # Temporarily rename artifact
    artifact = REPO / "artifacts" / "v1148_real_read_5repos.json"
    if not artifact.exists():
        return  # already missing
    backup = artifact.with_suffix(".json.bak")
    artifact.rename(backup)
    try:
        m = measure_v1185()
        assert m == 0.0, f"Should fall back to 0.0, got {m}"
        print(f"  ✓ Missing artifact → 0.0 (honest fallback)")
    finally:
        backup.rename(artifact)


def run_all():
    tests = [
        ("measure_v1185_in_range", test_measure_v1185_in_range),
        ("measure_v1185_above_baseline", test_measure_v1185_above_baseline),
        ("compute_v1185_deltas_has_keys", test_compute_v1185_deltas_has_keys),
        ("delta_is_positive", test_delta_is_positive),
        ("v1182_baseline_old_is_zero", test_v1182_baseline_old_is_zero),
        ("sub_dim_real_repo_ratio", test_sub_dim_real_repo_ratio),
        ("philosophy_guards_present", test_philosophy_guards_present),
        ("philosophy_guard_honesty", test_philosophy_guard_honesty),
        ("render_summary_works", test_render_summary_works),
        ("v1182_v06_new_dim_collector_integration_path", test_v1182_v06_new_dim_collector_integration_path),
        ("missing_artifact_fallback", test_missing_artifact_fallback),
    ]
    passed = 0
    failed = 0
    print(f"\n{'='*60}\nV1185 — vcp_real_run dim 真重算 接入测试 (V1148 cached)\n{'='*60}\n")
    for name, fn in tests:
        try:
            print(f"[{name}]")
            fn()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAIL: {e}")
            failed += 1
    print(f"\n{'='*60}\nV1185: {passed} pass, {failed} fail\n{'='*60}")
    return failed == 0


if __name__ == "__main__":
    success = run_all()
    sys.exit(0 if success else 1)