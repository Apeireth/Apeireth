"""Tests for V1144 — ASI V0.5 17 维度 真测补完版 (主 17:43 实事求是 + 主 22:33 ASI 北极星).

主 17:43 实事求是真测试:
  - 17 dims 都填 (dim_fill_rate = 1.0)
  - 14 dims 真测 (R) + 3 dims LOCKED (H)
  - v05_score > V1143 baseline 0.4511
  - vs ASI LOCKED 0.98 gap 缩到 < 0.2

主 19:33 走在前人经验上 — 复用 pytest + dataclass.
"""
from __future__ import annotations

import pytest

from apeireth.v1144_asi_v05_17dim_real_measure_complete import (
    ASI_V05_17DIMS,
    DIM_REGISTRY,
    DIM_STATUS_HARDCODED,
    DIM_STATUS_PARTIAL,
    DIM_STATUS_REAL,
    V1143_BASELINE_SCORE,
    V1144Snapshot,
    V1144_VERSION,
)


# ---------- structural tests ----------


def test_v1144_version_locked():
    assert V1144_VERSION == "0.1.0"


def test_17_dims_locked():
    assert len(ASI_V05_17DIMS) == 17
    # must include all V1143 dims + new ones
    must_have = {
        "phi_proxy", "capabilities", "cross_domain", "engineering",
        "vcp_4", "v2_philosophy", "rubric_open", "real_production",
        "cognitive_core", "self_organizing_core", "plugin_core",
        "self_improving_core", "neurosymbolic", "world_model",
        "reinforcement_learning", "scientific_method", "eternal_identity",
    }
    assert must_have.issubset(set(ASI_V05_17DIMS))


def test_dim_registry_complete():
    assert len(DIM_REGISTRY) == 17
    for dim in ASI_V05_17DIMS:
        assert dim in DIM_REGISTRY, f"dim {dim} missing from registry"
        fn, source, is_hardcoded_locked = DIM_REGISTRY[dim]
        assert callable(fn), f"dim {dim} has non-callable fn"
        assert isinstance(source, str) and len(source) > 0
        assert isinstance(is_hardcoded_locked, bool)


# ---------- snapshot tests ----------


def test_snapshot_empty_init():
    s = V1144Snapshot()
    assert s.n_dims == 0
    assert s.v05_score == 0.0
    assert s.n_real == 0
    assert s.n_hardcoded == 0
    assert s.n_partial == 0
    assert s.n_missing == 0


def test_snapshot_measure_all_runs():
    s = V1144Snapshot()
    s.measure_all()
    assert s.n_dims == 17, f"expected 17 dims, got {s.n_dims}"
    # dim_fill_rate should be 1.0 (all dims filled)
    assert s.dim_fill_rate == 1.0, f"dim_fill_rate = {s.dim_fill_rate}, expected 1.0"
    # n_real + n_hardcoded + n_partial + n_missing = 17
    assert s.n_real + s.n_hardcoded + s.n_partial + s.n_missing == 17


def test_snapshot_no_missing():
    """主 17:43 实事求是: 不允许任何 missing."""
    s = V1144Snapshot()
    s.measure_all()
    assert s.n_missing == 0, f"n_missing = {s.n_missing}, expected 0 (主 17:43 实事求是)"


def test_snapshot_locked_dims_hardcoded():
    """3 LOCKED dims 应标 H."""
    s = V1144Snapshot()
    s.measure_all()
    locked_dims = {"cross_domain", "vcp_4", "eternal_identity"}
    for dim in locked_dims:
        m = s.dim_values[dim]
        assert m.status == DIM_STATUS_HARDCODED, f"{dim} status = {m.status}, expected H"


def test_snapshot_locked_values():
    s = V1144Snapshot()
    s.measure_all()
    # LOCKED dims values must match
    assert abs(s.dim_values["cross_domain"].value - 1.0) < 0.001
    assert abs(s.dim_values["vcp_4"].value - 0.9588) < 0.001
    assert abs(s.dim_values["eternal_identity"].value - 0.8441) < 0.001


def test_snapshot_v05_score_above_baseline():
    """主 17:43 实事求是: V1144 真测补完必须 > V1143 baseline 0.4511."""
    s = V1144Snapshot()
    s.measure_all()
    assert s.v05_score > V1143_BASELINE_SCORE, (
        f"V1144 v05_score = {s.v05_score:.4f} <= V1143 baseline {V1143_BASELINE_SCORE}"
    )


def test_snapshot_v05_score_high():
    """V1144 真测补完应该 > 0.7 (vs LOCKED 0.98 gap < 0.3)."""
    s = V1144Snapshot()
    s.measure_all()
    assert s.v05_score > 0.7, f"V1144 v05_score = {s.v05_score:.4f}, expected > 0.7"


def test_snapshot_vs_locked_gap_small():
    """vs ASI LOCKED 0.98 gap 应 < 0.2 (V1144 真测补完 gap 缩窄)."""
    s = V1144Snapshot()
    s.measure_all()
    assert s.vs_locked_gap < 0.2, f"vs_locked_gap = {s.vs_locked_gap:.4f}, expected < 0.2"


def test_snapshot_delta_positive():
    s = V1144Snapshot()
    s.measure_all()
    assert s.vs_v1143_delta > 0.0, f"vs_v1143_delta = {s.vs_v1143_delta:+.4f}, expected > 0"


def test_snapshot_to_dict():
    s = V1144Snapshot()
    s.measure_all()
    d = s.to_dict()
    assert d["version"] == V1144_VERSION
    assert d["n_dims"] == 17
    assert "v05_score" in d
    assert "dim_fill_rate" in d
    assert "vs_v1143_baseline_delta" in d
    assert "vs_asi_locked_gap" in d
    assert d["philosophy_guard_ok"] is True
    assert "dim_breakdown" in d
    assert len(d["dim_breakdown"]) == 17


def test_snapshot_to_markdown():
    s = V1144Snapshot()
    s.measure_all()
    md = s.to_markdown()
    assert "# V1144 ASI V0.5 17 维度 真测补完快照报告" in md
    assert "V3 哲学守门" in md
    assert "V1144 vs V1143" in md
    for dim in ASI_V05_17DIMS:
        assert dim in md, f"dim {dim} missing from markdown report"


def test_snapshot_compare_v1143():
    s = V1144Snapshot()
    s.measure_all()
    cmp = s.compare_v1143()
    assert "V1144 vs V1143 真测补完对比" in cmp
    assert "V1143 baseline" in cmp
    for dim in ASI_V05_17DIMS:
        assert dim in cmp


# ---------- individual dim tests (主 17:43 实事求是: 每 dim 都能跑) ----------


def test_all_17_dims_have_value():
    s = V1144Snapshot()
    s.measure_all()
    for dim in ASI_V05_17DIMS:
        m = s.dim_values[dim]
        assert isinstance(m.value, float)
        assert 0.0 <= m.value <= 1.0, f"dim {dim} value {m.value} out of [0, 1]"
        assert m.status in (DIM_STATUS_REAL, DIM_STATUS_HARDCODED, DIM_STATUS_PARTIAL)
        assert len(m.source) > 0


# ---------- philosophy guard ----------


def test_philosophy_guard_ok():
    """不假装: 17 dim 真测不是 ASI 等级, dim_fill_rate 不是 ASI 等级."""
    s = V1144Snapshot()
    s.measure_all()
    d = s.to_dict()
    assert d["philosophy_guard_ok"] is True


# ---------- strict mode ----------


def test_strict_mode_passes():
    """--strict 要求 n_real >= 10. V1144 真测补完后应满足."""
    s = V1144Snapshot()
    s.measure_all()
    assert s.n_real >= 10, f"n_real = {s.n_real}, expected >= 10"


# ---------- main integration ----------


def test_v1144_main_default():
    """main() 默认输出应跑通."""
    import sys
    from apeireth.v1144_asi_v05_17dim_real_measure_complete import main
    rc = main([])
    assert rc == 0


def test_v1144_main_json():
    """--json 输出应有完整 snapshot."""
    import json
    import sys
    from io import StringIO

    from apeireth.v1144_asi_v05_17dim_real_measure_complete import main

    old_stdout = sys.stdout
    sys.stdout = captured = StringIO()
    try:
        rc = main(["--json"])
    finally:
        sys.stdout = old_stdout
    assert rc == 0
    d = json.loads(captured.getvalue())
    assert d["n_dims"] == 17
    assert d["v05_score"] > V1143_BASELINE_SCORE


def test_v1144_main_report():
    """--report 输出 Markdown."""
    import sys
    from io import StringIO

    from apeireth.v1144_asi_v05_17dim_real_measure_complete import main

    old_stdout = sys.stdout
    sys.stdout = captured = StringIO()
    try:
        rc = main(["--report"])
    finally:
        sys.stdout = old_stdout
    assert rc == 0
    md = captured.getvalue()
    assert "# V1144 ASI V0.5" in md
    assert "V3 哲学守门" in md


def test_v1144_main_compare():
    """--compare 输出 V1144 vs V1143 对比."""
    import sys
    from io import StringIO

    from apeireth.v1144_asi_v05_17dim_real_measure_complete import main

    old_stdout = sys.stdout
    sys.stdout = captured = StringIO()
    try:
        rc = main(["--compare"])
    finally:
        sys.stdout = old_stdout
    assert rc == 0
    cmp = captured.getvalue()
    assert "V1144 vs V1143" in cmp


def test_v1144_main_strict_passes():
    """--strict 要求 n_real >= 10. V1144 应该 PASS (n_real=14)."""
    import sys
    from io import StringIO

    from apeireth.v1144_asi_v05_17dim_real_measure_complete import main

    old_stdout = sys.stdout
    sys.stdout = captured = StringIO()
    old_stderr = sys.stderr
    sys.stderr = err = StringIO()
    try:
        rc = main(["--strict"])
    finally:
        sys.stdout = old_stdout
        sys.stderr = old_stderr
    assert rc == 0, f"--strict returned {rc}, stderr: {err.getvalue()}"