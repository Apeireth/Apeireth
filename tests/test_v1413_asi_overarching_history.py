"""Tests for V1413 ASI 总框架 history.

V1413 = ASI 总框架 time-series log + trend + digest + baseline:
- append_snapshot(): 真 append to JSONL
- load_history(): 真 load JSONL
- compute_trend(): 4 trend (IMPROVING/DECLINING/STABLE/INSUFFICIENT)
- compute_digest(): aggregate statistics
- make_baseline(): single-snapshot baseline
- compare_to_baseline(): regression detection
- render_history_md(): markdown report (8 sections)
- 15 GUARDS + 6 V3 哲学守门
- popper self-test 16/16 pass
- CLI: version/snapshot/list/trend/digest/baseline/compare/render/popper/meta/demo/help

主 17:43 实事求是: V1413 module + 90+ pytest pass + read-only delegate V1412 + V1411.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# Make apeireth importable
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1411_asi_overarching_framework as v1411  # noqa: E402
import v1412_asi_overarching_dashboard as v1412  # noqa: E402
import v1413_asi_overarching_history as v1413  # noqa: E402


# ----------------------- TestV1413Constants -----------------------

class TestV1413Constants:
    """Constants: VERSION, MODULE, SCHEMA, GUARDS, V3_GUARDS, TRENDS, BORROWED."""

    def test_version_is_0_1_0(self):
        assert v1413.V1413_VERSION == "0.1.0"

    def test_module_name(self):
        assert v1413.V1413_MODULE == "v1413_asi_overarching_history"

    def test_schema(self):
        assert v1413.V1413_SCHEMA == "v1413.asi-overarching-history/v1"

    def test_guards_count_15(self):
        assert len(v1413.V1413_GUARDS) == 15

    def test_guards_invariants(self):
        for g in v1413.V1413_GUARDS:
            assert g.startswith("GUARD_")

    def test_v3_guards_count_6(self):
        assert len(v1413.V1413_V3_GUARDS) == 6

    def test_v3_guards_phenomenal(self):
        assert "GUARD_HISTORY_IS_NOT_PHENOMENAL" in v1413.V1413_V3_GUARDS

    def test_v3_guards_asi(self):
        assert "GUARD_HISTORY_IS_NOT_ASI" in v1413.V1413_V3_GUARDS

    def test_v3_guards_human_level(self):
        assert "GUARD_HISTORY_IS_NOT_HUMAN_LEVEL" in v1413.V1413_V3_GUARDS

    def test_v3_guards_absolute(self):
        assert "GUARD_HISTORY_IS_NOT_ABSOLUTE" in v1413.V1413_V3_GUARDS

    def test_v3_guards_no_v1412_replace(self):
        assert "GUARD_HISTORY_IS_NOT_V1412_REPLACE" in v1413.V1413_V3_GUARDS

    def test_v3_guards_no_v1411_replace(self):
        assert "GUARD_HISTORY_IS_NOT_V1411_REPLACE" in v1413.V1413_V3_GUARDS

    def test_trends_count_4(self):
        assert len(v1413.V1413_TRENDS) == 4

    def test_trends_values(self):
        expected = ("IMPROVING", "DECLINING", "STABLE", "INSUFFICIENT")
        assert v1413.V1413_TRENDS == expected

    def test_borrowed_count_4(self):
        assert len(v1413.V1413_BORROWED) == 4

    def test_borrowed_has_v1412(self):
        keys = [b[0] for b in v1413.V1413_BORROWED]
        assert any("V1412" in k for k in keys)

    def test_default_history_path(self):
        assert v1413.V1413_DEFAULT_HISTORY_PATH == ".v1413-asi-overarching-history.jsonl"

    def test_default_baseline_path(self):
        assert v1413.V1413_DEFAULT_BASELINE_PATH == ".v1413-asi-overarching-baseline.json"


# ----------------------- TestV1413Dataclasses -----------------------

class TestV1413Dataclasses:
    """Dataclasses: HistorySnapshot, HistoryTrend, HistoryDigest, HistoryBaseline."""

    def test_snapshot_defaults(self):
        s = v1413.HistorySnapshot()
        assert s.timestamp == ""
        assert s.verdict == "INCOMPLETE"
        assert s.framework_score == 0
        assert s.anchor_value == 0.9105
        assert s.gap_to_north_star == 0.0695

    def test_snapshot_roundtrip(self):
        s = v1413.HistorySnapshot(
            timestamp="2026-08-10T02:00:00Z",
            snapshot_id="snap_1",
            verdict="COMPLETE",
            framework_score=11, level_score=12, coherence_score=12,
            chain_ok=True, borrowed_count=7,
            anchor_value=0.9105, gap_to_north_star=0.0695,
            note="test",
        )
        d = s.to_dict()
        assert d["schema"] == v1413.V1413_SCHEMA
        assert d["verdict"] == "COMPLETE"
        assert d["framework_score"] == 11
        s_back = v1413.HistorySnapshot.from_dict(d)
        assert s_back.verdict == s.verdict
        assert s_back.note == s.note

    def test_snapshot_from_dict_missing_fields(self):
        s = v1413.HistorySnapshot.from_dict({})
        assert s.timestamp == ""
        assert s.verdict == "INCOMPLETE"

    def test_trend_defaults(self):
        t = v1413.HistoryTrend()
        assert t.direction == "INSUFFICIENT"
        assert t.n_snapshots == 0

    def test_trend_roundtrip(self):
        t = v1413.HistoryTrend(
            direction="IMPROVING", delta_framework=2,
            n_snapshots=5, first_verdict="GOOD", last_verdict="COMPLETE",
        )
        d = t.to_dict()
        assert d["direction"] == "IMPROVING"
        assert d["delta_framework"] == 2

    def test_digest_defaults(self):
        d = v1413.HistoryDigest()
        assert d.n_snapshots == 0
        assert d.avg_gap_to_north_star == 0.0695

    def test_digest_roundtrip(self):
        d = v1413.HistoryDigest(
            n_snapshots=10, n_complete=5, n_good=3,
            earliest_timestamp="2026-08-01T00:00:00Z",
            latest_timestamp="2026-08-10T00:00:00Z",
        )
        dd = d.to_dict()
        assert dd["n_snapshots"] == 10
        assert dd["n_complete"] == 5

    def test_baseline_defaults(self):
        b = v1413.HistoryBaseline()
        assert b.baseline_verdict == "INCOMPLETE"
        assert b.baseline_anchor == 0.9105

    def test_baseline_roundtrip(self):
        b = v1413.HistoryBaseline(
            baseline_timestamp="2026-08-10T02:00:00Z",
            baseline_verdict="COMPLETE",
            baseline_framework_score=11,
            baseline_level_score=12,
            baseline_coherence_score=12,
            baseline_chain_ok=True,
            baseline_borrowed_count=7,
            baseline_anchor=0.9105,
            baseline_gap=0.0695,
            note="baseline",
        )
        d = b.to_dict()
        assert d["schema"] == v1413.V1413_SCHEMA + ".baseline/v1"
        assert d["baseline_verdict"] == "COMPLETE"
        b_back = v1413.HistoryBaseline.from_dict(d)
        assert b_back.baseline_verdict == "COMPLETE"
        assert b_back.baseline_framework_score == 11


# ----------------------- TestV1413Builders -----------------------

class TestV1413Builders:
    """slug_timestamp, build_snapshot_id, build_snapshot_from_dashboard."""

    def test_slug_timestamp_format(self):
        s = v1413.slug_timestamp()
        assert s.endswith("Z")
        assert "T" in s
        # Should be 20 chars: YYYY-MM-DDTHH-MM-SSZ
        assert len(s) == 20

    def test_slug_timestamp_custom_dt(self):
        from datetime import datetime, timezone
        dt = datetime(2026, 8, 10, 2, 15, 0, tzinfo=timezone.utc)
        s = v1413.slug_timestamp(dt)
        assert s == "2026-08-10T02-15-00Z"

    def test_build_snapshot_id_deterministic(self):
        id1 = v1413.build_snapshot_id("2026-08-10T02-15-00Z")
        id2 = v1413.build_snapshot_id("2026-08-10T02-15-00Z")
        assert id1 == id2

    def test_build_snapshot_id_has_v1413(self):
        id1 = v1413.build_snapshot_id("2026-08-10T02-15-00Z")
        assert "_v1413_" in id1

    def test_build_snapshot_from_v1412_real(self):
        snap = v1413.build_snapshot_from_v1412()
        # V1412 dashboard verdict is COMPLETE
        assert snap.verdict == "COMPLETE"
        assert snap.framework_score == 11
        assert snap.level_score == 12
        assert snap.coherence_score == 12
        assert snap.chain_ok is True
        assert snap.borrowed_count == 7
        assert snap.anchor_value == 0.9105
        assert abs(snap.gap_to_north_star - 0.0695) < 1e-6

    def test_build_snapshot_from_dashboard_custom_note(self):
        dash = v1412.build_dashboard_report()
        snap = v1413.build_snapshot_from_dashboard(dash, note="custom note", timestamp="2026-08-10T02:00:00Z")
        assert snap.note == "custom note"
        assert snap.timestamp == "2026-08-10T02:00:00Z"
        assert snap.snapshot_id != ""

    def test_snapshot_id_changes_with_timestamp(self):
        id1 = v1413.build_snapshot_id("2026-08-10T02-15-00Z")
        id2 = v1413.build_snapshot_id("2026-08-10T02-16-00Z")
        assert id1 != id2

    def test_short_hash_returns_4_hex(self):
        h = v1413._short_hash("hello")
        assert len(h) == 4
        assert all(c in "0123456789abcdef" for c in h)


# ----------------------- TestV1413IO -----------------------

class TestV1413IO:
    """append_snapshot, load_history, write_baseline, load_baseline, _is_path_safe."""

    def test_load_nonexistent_returns_empty(self):
        snaps = v1413.load_history("___nonexistent_xyz___")
        assert snaps == []

    def test_append_and_load_roundtrip(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            s1 = v1413.HistorySnapshot(
                timestamp="2026-08-10T01:00:00Z", snapshot_id="t1",
                verdict="GOOD", framework_score=10, level_score=11,
                coherence_score=11, chain_ok=True, borrowed_count=5,
                gap_to_north_star=0.0800,
            )
            s2 = v1413.HistorySnapshot(
                timestamp="2026-08-10T02:00:00Z", snapshot_id="t2",
                verdict="COMPLETE", framework_score=11, level_score=12,
                coherence_score=12, chain_ok=True, borrowed_count=7,
                gap_to_north_star=0.0695,
            )
            assert v1413.append_snapshot(s1, tp) is True
            assert v1413.append_snapshot(s2, tp) is True
            loaded = v1413.load_history(tp)
            assert len(loaded) == 2
            assert loaded[0].verdict == "GOOD"
            assert loaded[1].verdict == "COMPLETE"
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_append_auto_populates_timestamp(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            s = v1413.HistorySnapshot()  # no timestamp
            v1413.append_snapshot(s, tp)
            loaded = v1413.load_history(tp)
            assert loaded[0].timestamp != ""
            assert loaded[0].snapshot_id != ""
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_append_skips_invalid_lines(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            # Write invalid JSON line first, then valid
            with open(tp, "w", encoding="utf-8") as wf:
                wf.write("this is not json\n")
                wf.write(json.dumps(v1413.HistorySnapshot(
                    timestamp="2026-08-10T02:00:00Z", snapshot_id="t1",
                    verdict="GOOD", framework_score=10, level_score=11,
                    coherence_score=11, chain_ok=True, borrowed_count=5,
                ).to_dict()) + "\n")
            loaded = v1413.load_history(tp)
            assert len(loaded) == 1
            assert loaded[0].verdict == "GOOD"
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_write_baseline_atomic(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False, encoding="utf-8") as f:
            bp = f.name
        try:
            base = v1413.HistoryBaseline(
                baseline_timestamp="2026-08-10T02:00:00Z",
                baseline_verdict="COMPLETE",
                baseline_framework_score=11,
                baseline_level_score=12,
                baseline_coherence_score=12,
                baseline_chain_ok=True,
                baseline_borrowed_count=7,
                baseline_anchor=0.9105,
                baseline_gap=0.0695,
            )
            v1413.write_baseline(base, bp)
            loaded = v1413.load_baseline(bp)
            assert loaded is not None
            assert loaded.baseline_verdict == "COMPLETE"
        finally:
            Path(bp).unlink(missing_ok=True)

    def test_load_baseline_nonexistent(self):
        loaded = v1413.load_baseline("___nonexistent_xyz___")
        assert loaded is None

    def test_is_path_safe_relative(self):
        assert v1413._is_path_safe(".v1413.jsonl")
        assert v1413._is_path_safe("subdir/v1413.jsonl")

    def test_is_path_safe_rejects_traversal(self):
        assert v1413._is_path_safe("../escaped.jsonl") is False

    def test_is_path_safe_rejects_empty(self):
        assert v1413._is_path_safe("") is False

    def test_append_unsafe_path_raises(self):
        s = v1413.HistorySnapshot(timestamp="2026-08-10T02:00:00Z", snapshot_id="t1")
        with pytest.raises(ValueError):
            v1413.append_snapshot(s, "../escaped.jsonl")


# ----------------------- TestV1413Trend -----------------------

class TestV1413Trend:
    """compute_trend: IMPROVING / DECLINING / STABLE / INSUFFICIENT."""

    def _make_snap(self, ts, verdict, fw, lvl=12, coh=12, chain=True, bor=7, gap=0.0695):
        return v1413.HistorySnapshot(
            timestamp=ts, snapshot_id=ts,
            verdict=verdict, framework_score=fw, level_score=lvl,
            coherence_score=coh, chain_ok=chain, borrowed_count=bor,
            gap_to_north_star=gap,
        )

    def test_trend_insufficient_empty(self):
        t = v1413.compute_trend([])
        assert t.direction == "INSUFFICIENT"
        assert t.n_snapshots == 0

    def test_trend_insufficient_single(self):
        s = self._make_snap("2026-08-10T02:00:00Z", "GOOD", 10)
        t = v1413.compute_trend([s])
        assert t.direction == "INSUFFICIENT"
        assert t.n_snapshots == 1

    def test_trend_improving_framework(self):
        s1 = self._make_snap("2026-08-10T01:00:00Z", "GOOD", 10)
        s2 = self._make_snap("2026-08-10T02:00:00Z", "COMPLETE", 11)
        t = v1413.compute_trend([s1, s2])
        assert t.direction == "IMPROVING"
        assert t.delta_framework == 1
        assert t.first_verdict == "GOOD"
        assert t.last_verdict == "COMPLETE"

    def test_trend_declining_verdict_regression(self):
        s1 = self._make_snap("2026-08-10T01:00:00Z", "COMPLETE", 11)
        s2 = self._make_snap("2026-08-10T02:00:00Z", "GOOD", 10)
        t = v1413.compute_trend([s1, s2])
        assert t.direction == "DECLINING"
        assert t.first_verdict == "COMPLETE"
        assert t.last_verdict == "GOOD"

    def test_trend_stable(self):
        s1 = self._make_snap("2026-08-10T01:00:00Z", "COMPLETE", 11)
        s2 = self._make_snap("2026-08-10T02:00:00Z", "COMPLETE", 11)
        t = v1413.compute_trend([s1, s2])
        assert t.direction == "STABLE"

    def test_trend_improving_gap_closing(self):
        # Same verdict, same scores, but gap closes
        s1 = self._make_snap("2026-08-10T01:00:00Z", "COMPLETE", 11, gap=0.1000)
        s2 = self._make_snap("2026-08-10T02:00:00Z", "COMPLETE", 11, gap=0.0695)
        t = v1413.compute_trend([s1, s2])
        assert t.direction == "IMPROVING"
        assert t.delta_gap < 0

    def test_trend_reason_populated(self):
        s1 = self._make_snap("2026-08-10T01:00:00Z", "GOOD", 10)
        s2 = self._make_snap("2026-08-10T02:00:00Z", "COMPLETE", 11)
        t = v1413.compute_trend([s1, s2])
        assert t.reason != ""

    def test_trend_three_snapshots(self):
        s1 = self._make_snap("2026-08-10T01:00:00Z", "PARTIAL", 8)
        s2 = self._make_snap("2026-08-10T02:00:00Z", "GOOD", 10)
        s3 = self._make_snap("2026-08-10T03:00:00Z", "COMPLETE", 11)
        t = v1413.compute_trend([s1, s2, s3])
        assert t.direction == "IMPROVING"
        assert t.n_snapshots == 3
        assert t.first_verdict == "PARTIAL"
        assert t.last_verdict == "COMPLETE"


# ----------------------- TestV1413Digest -----------------------

class TestV1413Digest:
    """compute_digest: verdict distribution + averages + gap min/max."""

    def _make_snap(self, ts, verdict, fw=11, lvl=12, coh=12, chain=True, bor=7, gap=0.0695):
        return v1413.HistorySnapshot(
            timestamp=ts, snapshot_id=ts,
            verdict=verdict, framework_score=fw, level_score=lvl,
            coherence_score=coh, chain_ok=chain, borrowed_count=bor,
            gap_to_north_star=gap,
        )

    def test_digest_empty(self):
        d = v1413.compute_digest([])
        assert d.n_snapshots == 0

    def test_digest_single(self):
        s = self._make_snap("2026-08-10T02:00:00Z", "COMPLETE")
        d = v1413.compute_digest([s])
        assert d.n_snapshots == 1
        assert d.n_complete == 1
        assert d.n_good == 0
        assert d.avg_framework_score == 11.0
        assert d.min_gap == 0.0695

    def test_digest_verdict_distribution(self):
        snaps = [
            self._make_snap("2026-08-10T01:00:00Z", "COMPLETE"),
            self._make_snap("2026-08-10T02:00:00Z", "GOOD"),
            self._make_snap("2026-08-10T03:00:00Z", "GOOD"),
            self._make_snap("2026-08-10T04:00:00Z", "PARTIAL"),
            self._make_snap("2026-08-10T05:00:00Z", "WEAK"),
            self._make_snap("2026-08-10T06:00:00Z", "INCOMPLETE"),
        ]
        d = v1413.compute_digest(snaps)
        assert d.n_complete == 1
        assert d.n_good == 2
        assert d.n_partial == 1
        assert d.n_weak == 1
        assert d.n_incomplete == 1

    def test_digest_gap_min_max(self):
        snaps = [
            self._make_snap("2026-08-10T01:00:00Z", "COMPLETE", gap=0.10),
            self._make_snap("2026-08-10T02:00:00Z", "COMPLETE", gap=0.05),
            self._make_snap("2026-08-10T03:00:00Z", "COMPLETE", gap=0.0695),
        ]
        d = v1413.compute_digest(snaps)
        assert d.min_gap == 0.05
        assert d.max_gap == 0.10
        assert abs(d.avg_gap_to_north_star - (0.10 + 0.05 + 0.0695) / 3) < 1e-6

    def test_digest_span_seconds(self):
        snaps = [
            self._make_snap("2026-08-10T01:00:00Z", "COMPLETE"),
            self._make_snap("2026-08-10T02:00:00Z", "COMPLETE"),
        ]
        d = v1413.compute_digest(snaps)
        assert d.span_seconds == 3600.0

    def test_digest_chain_ok_count(self):
        snaps = [
            self._make_snap("2026-08-10T01:00:00Z", "COMPLETE", chain=True),
            self._make_snap("2026-08-10T02:00:00Z", "GOOD", chain=False),
        ]
        d = v1413.compute_digest(snaps)
        assert d.n_chain_ok == 1


# ----------------------- TestV1413Baseline -----------------------

class TestV1413Baseline:
    """make_baseline + compare_to_baseline."""

    def test_make_baseline_from_snapshot(self):
        s = v1413.HistorySnapshot(
            timestamp="2026-08-10T02:00:00Z", snapshot_id="b1",
            verdict="COMPLETE", framework_score=11, level_score=12,
            coherence_score=12, chain_ok=True, borrowed_count=7,
            gap_to_north_star=0.0695,
        )
        b = v1413.make_baseline(s, note="test baseline")
        assert b.baseline_verdict == "COMPLETE"
        assert b.baseline_framework_score == 11
        assert b.note == "test baseline"

    def test_compare_same_snapshot(self):
        s = v1413.HistorySnapshot(
            timestamp="2026-08-10T02:00:00Z", snapshot_id="b1",
            verdict="COMPLETE", framework_score=11, level_score=12,
            coherence_score=12, chain_ok=True, borrowed_count=7,
            gap_to_north_star=0.0695,
        )
        b = v1413.make_baseline(s)
        c = v1413.compare_to_baseline(s, b)
        assert c["delta_framework"] == 0
        assert c["verdict_unchanged"] is True
        assert c["verdict_improved"] is False
        assert c["verdict_regressed"] is False

    def test_compare_improved_snapshot(self):
        s1 = v1413.HistorySnapshot(
            timestamp="2026-08-10T01:00:00Z", snapshot_id="b1",
            verdict="GOOD", framework_score=10, level_score=11,
            coherence_score=11, chain_ok=True, borrowed_count=5,
            gap_to_north_star=0.0800,
        )
        s2 = v1413.HistorySnapshot(
            timestamp="2026-08-10T02:00:00Z", snapshot_id="b2",
            verdict="COMPLETE", framework_score=11, level_score=12,
            coherence_score=12, chain_ok=True, borrowed_count=7,
            gap_to_north_star=0.0695,
        )
        b = v1413.make_baseline(s1)
        c = v1413.compare_to_baseline(s2, b)
        assert c["delta_framework"] == 1
        assert c["verdict_improved"] is True
        assert c["delta_gap"] < 0

    def test_compare_regressed_snapshot(self):
        s1 = v1413.HistorySnapshot(
            timestamp="2026-08-10T01:00:00Z", snapshot_id="b1",
            verdict="COMPLETE", framework_score=11, level_score=12,
            coherence_score=12, chain_ok=True, borrowed_count=7,
            gap_to_north_star=0.0695,
        )
        s2 = v1413.HistorySnapshot(
            timestamp="2026-08-10T02:00:00Z", snapshot_id="b2",
            verdict="GOOD", framework_score=10, level_score=11,
            coherence_score=11, chain_ok=True, borrowed_count=5,
            gap_to_north_star=0.0800,
        )
        b = v1413.make_baseline(s1)
        c = v1413.compare_to_baseline(s2, b)
        assert c["verdict_regressed"] is True
        assert c["delta_framework"] == -1
        assert c["delta_gap"] > 0


# ----------------------- TestV1413Render -----------------------

class TestV1413Render:
    """render_history_md: 8 sections."""

    def _make_snap(self, ts, verdict, fw=11, lvl=12, coh=12, chain=True, bor=7, gap=0.0695):
        return v1413.HistorySnapshot(
            timestamp=ts, snapshot_id=ts,
            verdict=verdict, framework_score=fw, level_score=lvl,
            coherence_score=coh, chain_ok=chain, borrowed_count=bor,
            gap_to_north_star=gap,
        )

    def test_render_sections_present(self):
        snaps = [self._make_snap("2026-08-10T01:00:00Z", "GOOD")]
        t = v1413.compute_trend(snaps)
        d = v1413.compute_digest(snaps)
        md = v1413.render_history_md(snaps, t, d)
        assert "## 1." in md
        assert "## 2." in md
        assert "## 3." in md
        assert "## 4." in md
        assert "## 5." in md
        assert "## 6." in md
        assert "## 7." in md
        assert "## 8." in md

    def test_render_honest_disclosure(self):
        snaps = [self._make_snap("2026-08-10T01:00:00Z", "COMPLETE")]
        t = v1413.compute_trend(snaps)
        d = v1413.compute_digest(snaps)
        md = v1413.render_history_md(snaps, t, d)
        assert "Honest disclosure" in md
        assert "Phenomenal" in md
        assert "ASI 达成" in md

    def test_render_borrowed_section(self):
        snaps = [self._make_snap("2026-08-10T01:00:00Z", "COMPLETE")]
        t = v1413.compute_trend(snaps)
        d = v1413.compute_digest(snaps)
        md = v1413.render_history_md(snaps, t, d)
        assert "V1412" in md
        assert "V1375" in md
        assert "V1394" in md

    def test_render_with_baseline(self):
        s = self._make_snap("2026-08-10T01:00:00Z", "COMPLETE")
        b = v1413.make_baseline(s)
        snaps = [s]
        t = v1413.compute_trend(snaps)
        d = v1413.compute_digest(snaps)
        md = v1413.render_history_md(snaps, t, d, baseline=b)
        assert "Baseline" in md
        assert b.baseline_timestamp in md

    def test_render_without_baseline(self):
        s = self._make_snap("2026-08-10T01:00:00Z", "COMPLETE")
        snaps = [s]
        t = v1413.compute_trend(snaps)
        d = v1413.compute_digest(snaps)
        md = v1413.render_history_md(snaps, t, d)
        assert "No baseline set" in md


# ----------------------- TestV1413Popper -----------------------

class TestV1413Popper:
    """popper_self_test: 16 self-tests."""

    def test_popper_passes(self):
        r = v1413.popper_self_test()
        assert r["passed"] is True
        assert r["failures"] == []
        assert r["n_tested"] == 16


# ----------------------- TestV1413CLI -----------------------

class TestV1413CLI:
    """CLI: version / snapshot / list / trend / digest / baseline / compare / render / popper / meta / demo."""

    def _run_cli(self, args):
        env = os.environ.copy()
        env["PYTHONIOENCODING"] = "utf-8"
        return subprocess.run(
            [sys.executable, "-m", "apeireth.v1413_asi_overarching_history"] + args,
            capture_output=True, text=True, encoding="utf-8",
            env=env, cwd=str(ROOT),
        )

    def test_cli_version(self):
        r = self._run_cli(["version"])
        assert r.returncode == 0
        assert "V1413 ASI 总框架 history v0.1.0" in r.stdout

    def test_cli_meta(self):
        r = self._run_cli(["meta"])
        assert r.returncode == 0
        assert "v1413_asi_overarching_history" in r.stdout
        assert "GUARD_HISTORY_REAL" in r.stdout

    def test_cli_meta_json(self):
        r = self._run_cli(["meta", "--json"])
        assert r.returncode == 0
        data = json.loads(r.stdout)
        assert data["module"] == "v1413_asi_overarching_history"
        assert data["version"] == "0.1.0"
        assert len(data["guards"]) == 15
        assert len(data["v3_guards"]) == 6

    def test_cli_snapshot_appends(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            r = self._run_cli(["snapshot", "--note", "cli test", "--history", tp])
            assert r.returncode == 0
            assert "appended:" in r.stdout
            assert "verdict=COMPLETE" in r.stdout
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_list(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            self._run_cli(["snapshot", "--note", "list test", "--history", tp])
            r = self._run_cli(["list", "--history", tp])
            assert r.returncode == 0
            assert "COMPLETE" in r.stdout
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_list_json(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            self._run_cli(["snapshot", "--history", tp])
            r = self._run_cli(["list", "--history", tp, "--json"])
            assert r.returncode == 0
            data = json.loads(r.stdout)
            assert len(data) >= 1
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_trend_insufficient(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            r = self._run_cli(["trend", "--history", tp])
            assert r.returncode == 0
            assert "INSUFFICIENT" in r.stdout
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_digest_empty(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            r = self._run_cli(["digest", "--history", tp])
            assert r.returncode == 0
            assert "no snapshots" in r.stdout
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_digest_json(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            self._run_cli(["snapshot", "--history", tp])
            r = self._run_cli(["digest", "--history", tp, "--json"])
            assert r.returncode == 0
            data = json.loads(r.stdout)
            assert data["n_snapshots"] >= 1
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_baseline_set_and_compare(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False, encoding="utf-8") as f:
            bp = f.name
        try:
            self._run_cli(["snapshot", "--history", tp])
            r1 = self._run_cli(["baseline", "--history", tp, "--baseline-path", bp, "--note", "cli test"])
            assert r1.returncode == 0
            assert "baseline set:" in r1.stdout
            r2 = self._run_cli(["compare", "--baseline-path", bp])
            assert r2.returncode == 0
            assert "compare:" in r2.stdout
        finally:
            Path(tp).unlink(missing_ok=True)
            Path(bp).unlink(missing_ok=True)

    def test_cli_baseline_empty_history_fails(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            r = self._run_cli(["baseline", "--history", tp])
            assert r.returncode != 0
            assert "no snapshots" in r.stderr
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_compare_no_baseline_fails(self):
        r = self._run_cli(["compare", "--baseline-path", "___nonexistent_baseline_xyz___"])
        assert r.returncode != 0

    def test_cli_render_to_stdout(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            self._run_cli(["snapshot", "--history", tp])
            r = self._run_cli(["render", "--history", tp])
            assert r.returncode == 0
            assert "## 1." in r.stdout
            assert "Honest disclosure" in r.stdout
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_render_to_file(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        with tempfile.NamedTemporaryFile(mode="w", suffix=".md", delete=False, encoding="utf-8") as f:
            op = f.name
        try:
            self._run_cli(["snapshot", "--history", tp])
            r = self._run_cli(["render", "--history", tp, "--out", op])
            assert r.returncode == 0
            assert "wrote" in r.stdout
            assert Path(op).exists()
            content = Path(op).read_text(encoding="utf-8")
            assert "V1413" in content
        finally:
            Path(tp).unlink(missing_ok=True)
            Path(op).unlink(missing_ok=True)

    def test_cli_render_json(self):
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        try:
            self._run_cli(["snapshot", "--history", tp])
            r = self._run_cli(["render", "--history", tp, "--format", "json"])
            assert r.returncode == 0
            data = json.loads(r.stdout)
            assert "trend" in data
            assert "digest" in data
            assert "snapshots" in data
        finally:
            Path(tp).unlink(missing_ok=True)

    def test_cli_popper(self):
        r = self._run_cli(["popper"])
        assert r.returncode == 0
        data = json.loads(r.stdout)
        assert data["passed"] is True
        assert data["n_tested"] == 16

    def test_cli_demo(self):
        r = self._run_cli(["demo"])
        assert r.returncode == 0
        assert "V1413 demo: 2 snapshots" in r.stdout
        assert "trend: IMPROVING" in r.stdout


# ----------------------- TestV1413PhilosophyGuard -----------------------

class TestV1413PhilosophyGuard:
    """V3 哲学守门 + honest cap preserved."""

    def test_v3_guards_complete(self):
        expected = {
            "GUARD_HISTORY_IS_NOT_PHENOMENAL",
            "GUARD_HISTORY_IS_NOT_ASI",
            "GUARD_HISTORY_IS_NOT_HUMAN_LEVEL",
            "GUARD_HISTORY_IS_NOT_ABSOLUTE",
            "GUARD_HISTORY_IS_NOT_V1412_REPLACE",
            "GUARD_HISTORY_IS_NOT_V1411_REPLACE",
        }
        assert expected == set(v1413.V1413_V3_GUARDS)

    def test_honest_cap_preserved(self):
        # V1413 source anchor = V1256 unio_mystica 0.9105 LOCKED
        snap = v1413.build_snapshot_from_v1412()
        assert snap.anchor_value == 0.9105
        assert abs(snap.gap_to_north_star - 0.0695) < 1e-6

    def test_no_v1412_write(self):
        # V1413 reads V1412 build_dashboard_report() but never modifies it
        import inspect
        src = inspect.getsource(v1413)
        # V1413 does not import V1412 module attributes for write
        assert "v1412.build_dashboard_report()" in src
        # No assignment to v1412 attributes
        assert "v1412." not in src.replace("v1412.build_dashboard_report", "")

    def test_no_v1411_write(self):
        import inspect
        src = inspect.getsource(v1413)
        # V1413 does not import or write to v1411
        assert "v1411" not in src

    def test_history_is_dashboard_log(self):
        # V1413 = time-series log of V1412 dashboard reports
        snap = v1413.build_snapshot_from_v1412()
        assert snap.source_module == "v1412_asi_overarching_dashboard"
        assert snap.source_version == "0.1.0"


# ----------------------- TestV1413Integration -----------------------

class TestV1413Integration:
    """End-to-end integration with V1411 + V1412."""

    def test_v1413_reads_v1412_dashboard(self):
        # V1412 build_dashboard_report() must work
        dash = v1412.build_dashboard_report()
        assert dash.verdict.verdict == "COMPLETE"

    def test_v1413_builds_snapshot_from_real_v1412(self):
        snap = v1413.build_snapshot_from_v1412()
        assert snap.verdict == "COMPLETE"
        assert snap.framework_score == 11
        assert snap.level_score == 12

    def test_v1413_does_not_mutate_v1412(self):
        # Snapshot from V1412 twice → same fields (read-only)
        s1 = v1413.build_snapshot_from_v1412()
        s2 = v1413.build_snapshot_from_v1412()
        # Same verdict / scores (timestamps differ)
        assert s1.verdict == s2.verdict
        assert s1.framework_score == s2.framework_score
        assert s1.anchor_value == s2.anchor_value

    def test_v1411_report_still_works(self):
        # V1411 chain closure v1 still works (V1413 doesn't break V1411)
        r = v1411.run_self_overarching()
        assert r.asi_overarching_complete is True
        assert len(r.frameworks) == 11

    def test_end_to_end_workflow(self):
        # Build snapshot → append → load → trend → digest → baseline → compare
        with tempfile.NamedTemporaryFile(mode="w", suffix=".jsonl", delete=False, encoding="utf-8") as f:
            tp = f.name
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False, encoding="utf-8") as f:
            bp = f.name
        try:
            # 1. Build + append snapshot
            snap1 = v1413.build_snapshot_from_v1412(note="tick 1", timestamp="2026-08-10T01:00:00Z")
            v1413.append_snapshot(snap1, tp)
            # 2. Load
            loaded = v1413.load_history(tp)
            assert len(loaded) == 1
            # 3. Trend (insufficient)
            t = v1413.compute_trend(loaded)
            assert t.direction == "INSUFFICIENT"
            # 4. Digest
            d = v1413.compute_digest(loaded)
            assert d.n_complete == 1
            # 5. Baseline
            base = v1413.make_baseline(loaded[0])
            v1413.write_baseline(base, bp)
            loaded_base = v1413.load_baseline(bp)
            assert loaded_base.baseline_verdict == "COMPLETE"
            # 6. Compare
            cmp = v1413.compare_to_baseline(loaded[0], loaded_base)
            assert cmp["verdict_unchanged"] is True
        finally:
            Path(tp).unlink(missing_ok=True)
            Path(bp).unlink(missing_ok=True)


# ----------------------- TestV1413ChainIntegration -----------------------

class TestV1413ChainIntegration:
    """V1413 + V1411 + V1412 chain: no regression."""

    def test_v1413_chain_no_regression(self):
        # V1413 chain: build snapshot from V1412 + load + compute trend + compute digest + render
        snap = v1413.build_snapshot_from_v1412()
        snaps = [snap]
        t = v1413.compute_trend(snaps)
        d = v1413.compute_digest(snaps)
        md = v1413.render_history_md(snaps, t, d)
        # V1412 chain: V1412 dashboard still works
        dash = v1412.build_dashboard_report()
        assert dash.verdict.verdict == "COMPLETE"
        # V1411 chain: V1411 report still works
        rep = v1411.run_self_overarching()
        assert rep.asi_overarching_complete is True
        # All three frozen together
        assert snap.verdict == dash.verdict.verdict
        assert snap.anchor_value == dash.source_anchor_value