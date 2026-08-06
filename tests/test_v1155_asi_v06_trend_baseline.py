"""
V1155 tests — ASI V0.6 真生产 trend baseline + 21-dim heatmap + next-ROI suggester.

(主 17:43 实事求是: 测试必真跑必检出失败, 不假装)
"""
from __future__ import annotations

import json
import shutil
import tempfile
from pathlib import Path

import pytest

from apeireth.v1155_asi_v06_trend_baseline import (
    BAR_CHAR,
    BAR_WIDTH,
    DEFAULT_SNAPSHOT_DIR,
    DEFAULT_TOP_K,
    EMPTY_CHAR,
    NextROITarget,
    V06DimSnapshot,
    V06Snapshot,
    V1155_VERSION,
    _bar,
    _git_commit,
    _git_dirty,
    diff_snapshots,
    render_heatmap_md,
    render_next_targets_md,
    run_v1155_acceptance,
    snapshot_v06,
    suggest_next_targets,
    write_baseline,
)


# ---------------------------------------------------------------------------
# 基础结构 (主 17:43 实事求是: snapshot 必须真跑, 不假装)
# ---------------------------------------------------------------------------


def test_version_is_string():
    assert isinstance(V1155_VERSION, str)
    assert len(V1155_VERSION) > 0


def test_snapshot_returns_v06snapshot_dataclass():
    snap = snapshot_v06()
    assert isinstance(snap, V06Snapshot)
    assert snap.snapshot_id.startswith("v1155-")
    assert len(snap.snapshot_id) == len("v1155-") + 8
    assert snap.taken_at > 0


def test_snapshot_has_21_dims():
    snap = snapshot_v06()
    assert snap.n_dims == 21
    assert len(snap.dims) == 21
    for d in snap.dims:
        assert isinstance(d, V06DimSnapshot)
        assert 0.0 <= d.value <= 1.0
        assert d.weight > 0
        assert d.status in ("R", "H", "P", "M")
        assert isinstance(d.source, str)


def test_snapshot_score_in_range():
    snap = snapshot_v06()
    assert 0.0 <= snap.score <= 1.0
    assert snap.gap == snap.score - snap.north_star


def test_snapshot_includes_git_context():
    snap = snapshot_v06()
    assert isinstance(snap.git_commit, str)
    assert isinstance(snap.git_dirty, bool)


def test_snapshot_status_counts_sum_to_21():
    snap = snapshot_v06()
    total = snap.n_real + snap.n_hardcoded + snap.n_partial + snap.n_missing
    assert total == 21


def test_bar_function():
    assert _bar(0.0) == EMPTY_CHAR * BAR_WIDTH
    assert _bar(1.0) == BAR_CHAR * BAR_WIDTH
    assert _bar(0.5, width=10) == BAR_CHAR * 5 + EMPTY_CHAR * 5
    assert _bar(-0.1) == EMPTY_CHAR * BAR_WIDTH  # clamp
    assert _bar(1.5) == BAR_CHAR * BAR_WIDTH  # clamp


# ---------------------------------------------------------------------------
# Diff (主 17:43 实事求是: 不假装 trend = 一定上升)
# ---------------------------------------------------------------------------


def test_diff_snapshots_has_required_fields():
    snap1 = snapshot_v06()
    snap2 = snapshot_v06()
    diff = diff_snapshots(snap1, snap2)
    assert "score_delta" in diff
    assert "gap_delta" in diff
    assert "dim_deltas" in diff
    assert "improved_dims" in diff
    assert "regressed_dims" in diff
    assert "unchanged_dims" in diff


def test_diff_snapshots_self_diff_is_zero():
    """跟自己对对比 = 0 delta (主 17:43 实事求是)."""
    snap = snapshot_v06()
    diff = diff_snapshots(snap, snap)
    assert diff["score_delta"] == 0.0
    assert diff["gap_delta"] == 0.0
    assert diff["improved_dims"] == []
    assert diff["regressed_dims"] == []
    assert len(diff["unchanged_dims"]) == 21


def test_diff_snapshots_detects_modified_dim():
    """手工改一个 dim 的 value, diff 应能检出."""
    snap1 = snapshot_v06()
    # 复制并修改 snap1 一个 dim
    modified_dims = []
    for d in snap1.dims:
        if d.dim == "cognitive_core":
            modified_dims.append(V06DimSnapshot(
                dim=d.dim, weight=d.weight, value=min(1.0, d.value + 0.1),
                status=d.status, source=d.source,
            ))
        else:
            modified_dims.append(d)
    snap2 = V06Snapshot(
        snapshot_id=snap1.snapshot_id + "-mod",
        taken_at=snap1.taken_at + 1.0,
        version=snap1.version,
        git_commit=snap1.git_commit,
        git_dirty=snap1.git_dirty,
        score=snap1.score + 0.1 * snap1.dims[0].weight,  # approx
        north_star=snap1.north_star,
        gap=snap1.gap,
        n_dims=snap1.n_dims,
        n_real=snap1.n_real,
        n_hardcoded=snap1.n_hardcoded,
        n_partial=snap1.n_partial,
        n_missing=snap1.n_missing,
        dims=modified_dims,
    )
    diff = diff_snapshots(snap1, snap2)
    assert "cognitive_core" in diff["dim_deltas"]
    assert diff["dim_deltas"]["cognitive_core"]["value_delta"] > 0


# ---------------------------------------------------------------------------
# Heatmap (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


def test_render_heatmap_md_contains_21_dims():
    snap = snapshot_v06()
    md = render_heatmap_md(snap)
    for d in snap.dims:
        assert d.dim in md, f"dim {d.dim} missing in heatmap md"


def test_render_heatmap_md_contains_score_and_gap():
    snap = snapshot_v06()
    md = render_heatmap_md(snap)
    assert f"{snap.score:.4f}" in md
    assert f"{snap.gap:+.4f}" in md
    assert f"{snap.north_star:.4f}" in md
    assert snap.snapshot_id in md


def test_render_heatmap_md_sorted_ascending():
    """21-dim 应按 value asc 排序 (主 00:56 接手一目了然: 最低在最上)."""
    snap = snapshot_v06()
    md = render_heatmap_md(snap)
    lines = md.split("\n")
    # 找到 markdown table rows
    table_rows = [l for l in lines if l.startswith("| `") and "|" in l]
    assert len(table_rows) == 21
    # 提取 value
    values = []
    for row in table_rows:
        parts = [p.strip() for p in row.split("|")]
        # parts[0]="" parts[1]="`dim`" parts[2]="value" ...
        try:
            v = float(parts[2])
            values.append(v)
        except (ValueError, IndexError):
            pass
    assert len(values) == 21
    assert values == sorted(values), f"not sorted asc: {values}"


# ---------------------------------------------------------------------------
# Next-ROI Suggester (主 13:31 大胆激进)
# ---------------------------------------------------------------------------


def test_suggest_next_targets_sorted_by_gain_desc():
    snap = snapshot_v06()
    targets = suggest_next_targets(snap, top_k=5)
    assert len(targets) == 5
    gains = [t.potential_gain for t in targets]
    assert gains == sorted(gains, reverse=True), f"not desc: {gains}"
    for t in targets:
        assert isinstance(t, NextROITarget)
        assert t.potential_gain == round(t.weight * (1.0 - t.value), 4)
        assert t.rank > 0


def test_suggest_next_targets_top_k_respected():
    snap = snapshot_v06()
    for k in [1, 3, 7, 21]:
        targets = suggest_next_targets(snap, top_k=k)
        assert len(targets) == k


def test_suggest_next_targets_rationale_not_empty():
    snap = snapshot_v06()
    targets = suggest_next_targets(snap, top_k=3)
    for t in targets:
        assert isinstance(t.rationale, str)
        assert len(t.rationale) > 10


def test_render_next_targets_md_contains_module_names():
    snap = snapshot_v06()
    targets = suggest_next_targets(snap, top_k=5)
    md = render_next_targets_md(targets, snap)
    for t in targets:
        assert t.dim in md
        assert f"V{1156 + t.rank - 1}" in md


# ---------------------------------------------------------------------------
# Write Baseline (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


def test_write_baseline_creates_3_files():
    snap = snapshot_v06()
    with tempfile.TemporaryDirectory() as tmpdir:
        jp, hp, np_ = write_baseline(snap, snapshot_dir=tmpdir, top_k=5)
        assert Path(jp).exists()
        assert Path(hp).exists()
        assert Path(np_).exists()
        # JSON 必含 21 dims
        with open(jp, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert len(data["dims"]) == 21
        # heatmap md 必含 snapshot_id
        hm_content = Path(hp).read_text(encoding="utf-8")
        assert snap.snapshot_id in hm_content
        # next_targets md 必含 V1156+
        nt_content = Path(np_).read_text(encoding="utf-8")
        assert "V1156" in nt_content


def test_write_baseline_json_is_loadable():
    snap = snapshot_v06()
    with tempfile.TemporaryDirectory() as tmpdir:
        jp, _, _ = write_baseline(snap, snapshot_dir=tmpdir, top_k=5)
        with open(jp, "r", encoding="utf-8") as f:
            data = json.load(f)
        # 必含字段
        for key in [
            "snapshot_id", "taken_at", "version", "git_commit",
            "score", "north_star", "gap", "n_dims", "dims",
        ]:
            assert key in data, f"missing key: {key}"


# ---------------------------------------------------------------------------
# Acceptance (主 17:43 实事求是: spec 必须可证伪)
# ---------------------------------------------------------------------------


def test_run_v1155_acceptance_all_pass():
    result = run_v1155_acceptance()
    assert result["n_fail"] == 0, f"acceptance failed: {result}"
    assert result["n_pass"] == result["n_tests"]
    assert result["n_tests"] >= 6


def test_acceptance_each_test_has_name():
    result = run_v1155_acceptance()
    for t in result["tests"]:
        assert "name" in t
        assert "passed" in t


# ---------------------------------------------------------------------------
# Git context (主 00:44 质量工程化)
# ---------------------------------------------------------------------------


def test_git_commit_is_string():
    commit = _git_commit()
    assert isinstance(commit, str)
    # "unknown" if git not available, otherwise short hash
    assert commit == "unknown" or len(commit) >= 4


def test_git_dirty_is_bool():
    dirty = _git_dirty()
    assert isinstance(dirty, bool)


# ---------------------------------------------------------------------------
# Snapshot 不可变 (主 00:44 质量工程化)
# ---------------------------------------------------------------------------


def test_snapshot_to_dict_is_json_serializable():
    snap = snapshot_v06()
    d = snap.to_dict()
    json_str = json.dumps(d, ensure_ascii=False)
    # round-trip
    d2 = json.loads(json_str)
    assert d2["snapshot_id"] == snap.snapshot_id
    assert d2["score"] == snap.score
    assert len(d2["dims"]) == 21


def test_two_snapshots_have_different_ids():
    snap1 = snapshot_v06()
    snap2 = snapshot_v06()
    assert snap1.snapshot_id != snap2.snapshot_id