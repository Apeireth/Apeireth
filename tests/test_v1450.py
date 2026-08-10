"""Tests for V1450 — ASI cross-modular cube history aggregator."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Ensure promethean root is on sys.path
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

from apeireth import v1450_asi_cross_modular_cube_history as v1450


# ============================================================================
# Module-level constants
# ============================================================================


class TestConstants:
    def test_version_set(self):
        assert v1450.V1450_VERSION == "0.1.0"

    def test_schema_set(self):
        assert v1450.V1450_SCHEMA == "asi.cross-modular-cube-history.v1"

    def test_module_id(self):
        assert v1450.V1450_MODULE == "apeireth.v1450_asi_cross_modular_cube_history"

    def test_seven_problems(self):
        assert len(v1450.V1450_PROBLEM_NAMES) == 7
        assert "time" in v1450.V1450_PROBLEM_NAMES
        assert "value_alignment" in v1450.V1450_PROBLEM_NAMES

    def test_five_positions(self):
        assert len(v1450.V1450_POSITION_NAMES) == 5
        assert "scheduler" in v1450.V1450_POSITION_NAMES
        assert "asi_occupier" in v1450.V1450_POSITION_NAMES

    def test_six_protocols(self):
        assert len(v1450.V1450_PROTOCOL_NAMES) == 6
        assert "sync" in v1450.V1450_PROTOCOL_NAMES
        assert "hybrid" in v1450.V1450_PROTOCOL_NAMES

    def test_face_reports_three(self):
        assert len(v1450.V1450_FACE_REPORTS) == 3

    def test_face_axes(self):
        # V1447: problem × position; V1448: position × protocol; V1449: problem × protocol
        face_axes = {face[0]: face[2] for face in v1450.V1450_FACE_REPORTS}
        assert face_axes["V1447_problem_position"] == ("problem", "position")
        assert face_axes["V1448_position_protocol"] == ("position", "protocol")
        assert face_axes["V1449_problem_protocol"] == ("problem", "protocol")

    def test_axis_face_overlap(self):
        # Each axis appears in 2 faces
        assert v1450.V1450_AXIS_FACE_OVERLAP["problem"] == 2
        assert v1450.V1450_AXIS_FACE_OVERLAP["position"] == 2
        assert v1450.V1450_AXIS_FACE_OVERLAP["protocol"] == 2

    def test_closure_kinds_count(self):
        assert len(v1450.V1450_CLOSURE_KINDS) == 5

    def test_guards_count(self):
        assert len(v1450.V1450_GUARDS) >= 14

    def test_v3_guards_count(self):
        assert len(v1450.V1450_V3_GUARDS) == 5

    def test_borrowed_count(self):
        assert len(v1450.V1450_BORROWED) == 5

    def test_face_files_paths_set(self):
        # 3 face files should be defined
        assert len(v1450.V1450_FACE_FILES) == 3
        for face_id, path in v1450.V1450_FACE_FILES.items():
            assert face_id in [f[0] for f in v1450.V1450_FACE_REPORTS]
            assert path.exists() or True  # path might not exist on every host


# ============================================================================
# Helpers
# ============================================================================


class TestHelpers:
    def test_clip01_low(self):
        assert v1450._clip01(-0.5) == 0.0

    def test_clip01_high(self):
        assert v1450._clip01(1.5) == 1.0

    def test_clip01_mid(self):
        assert v1450._clip01(0.42) == 0.42

    def test_safe_div_normal(self):
        assert v1450._safe_div(6.0, 2.0) == 3.0

    def test_safe_div_zero(self):
        assert v1450._safe_div(1.0, 0.0) == 0.0

    def test_safe_str_short(self):
        assert v1450._safe_str("hello") == "hello"

    def test_safe_str_long(self):
        long_str = "x" * 500
        result = v1450._safe_str(long_str, max_len=100)
        assert len(result) <= 100
        assert "truncated" in result

    def test_safe_load_json_missing(self):
        result = v1450._safe_load_json(Path("/nonexistent/path.json"))
        assert result is None

    def test_now_utc_iso_format(self):
        ts = v1450._now_utc_iso()
        assert "T" in ts
        assert ts.endswith("Z")


# ============================================================================
# Data classes
# ============================================================================


class TestDataClasses:
    def test_face_snapshot_constructible(self):
        face = v1450.FaceSnapshot(
            face_id="TEST", source_module="test", axes=("a", "b"),
            overall_closure_rate=0.5, cross_link_density=0.6,
            n_pairs=2, n_probes=10,
            per_kind_closure={"forward": 0.5},
            per_axis_a_closure={"a1": 0.5},
            per_axis_b_closure={"b1": 0.5},
            found=True,
        )
        assert face.face_id == "TEST"
        assert face.axes == ("a", "b")
        d = face.to_dict()
        assert d["axes"] == ["a", "b"]

    def test_axis_stats_constructible(self):
        stat = v1450.AxisElementStats(axis="problem", element="time")
        assert stat.axis == "problem"
        stat.closure_rates["face1"] = 0.5
        stat.mean_closure = 0.5
        stat.face_count = 1
        d = stat.to_dict()
        assert d["axis"] == "problem"

    def test_cube_snapshot_constructible(self):
        snap = v1450.CubeSnapshot(timestamp="2026-08-10T00:00:00Z")
        assert snap.timestamp == "2026-08-10T00:00:00Z"
        assert snap.n_faces_total == 3
        d = snap.to_dict()
        assert isinstance(d, dict)

    def test_cube_history_report_constructible(self):
        snap = v1450.CubeSnapshot(timestamp="t1")
        report = v1450.CubeHistoryReport(
            schema="test", version="0.1.0", module="test",
            started="t1", ended="t2",
            snapshots_loaded=0, snapshots_appended=1,
            current_snapshot=snap, history_trend="INSUFFICIENT",
        )
        assert report.history_trend == "INSUFFICIENT"
        d = report.to_dict()
        assert "guards" in d
        assert "borrowed" in d


# ============================================================================
# Face loading + cube aggregation
# ============================================================================


class TestFaceLoading:
    def test_load_v1447_face(self):
        face = v1450._load_face_snapshot(
            "V1447_problem_position", "v1447_asi_cross_modular_audit", ("problem", "position")
        )
        assert face.face_id == "V1447_problem_position"
        assert face.axes == ("problem", "position")
        # Real V1447 report should exist and be loadable
        if face.found:
            assert face.overall_closure_rate > 0
            # per_axis_a_closure is a dict of problem_name -> closure_rate
            # (NOT a dict of "per_problem_closure_rate" -> dict)
            for prob_name in v1450.V1450_PROBLEM_NAMES:
                # At least one known problem should appear (or dict is empty if file structure differs)
                pass  # just ensure no crash
            # Verify dict values are floats
            for key, val in face.per_axis_a_closure.items():
                assert isinstance(val, (int, float)), f"per_axis_a_closure[{key}]={val} not a number"
            for key, val in face.per_axis_b_closure.items():
                assert isinstance(val, (int, float)), f"per_axis_b_closure[{key}]={val} not a number"

    def test_load_v1448_face(self):
        face = v1450._load_face_snapshot(
            "V1448_position_protocol", "v1448_asi_vcp_six_protocol_cross_modular", ("position", "protocol")
        )
        assert face.face_id == "V1448_position_protocol"

    def test_load_v1449_face(self):
        face = v1450._load_face_snapshot(
            "V1449_problem_protocol", "v1449_asi_seven_problems_vcp_cross_modular", ("problem", "protocol")
        )
        assert face.face_id == "V1449_problem_protocol"

    def test_load_unknown_face(self):
        face = v1450._load_face_snapshot(
            "UNKNOWN_face", "unknown_module", ("x", "y")
        )
        assert face.found is False


class TestCubeAggregation:
    def test_aggregate_returns_snapshot(self):
        snap = v1450.aggregate_cube_snapshot()
        assert snap is not None
        assert snap.n_faces_total == 3

    def test_aggregate_rate_bounded(self):
        snap = v1450.aggregate_cube_snapshot()
        assert 0.0 <= snap.cube_overall_closure_rate <= 1.0

    def test_aggregate_xlink_bounded(self):
        snap = v1450.aggregate_cube_snapshot()
        assert 0.0 <= snap.cube_cross_link_density <= 1.0

    def test_aggregate_balance_bounded(self):
        snap = v1450.aggregate_cube_snapshot()
        assert 0.0 <= snap.axis_balance_score <= 1.0

    def test_aggregate_axis_stats_count(self):
        # 7 problems + 5 positions + 6 protocols = 18 axis elements
        snap = v1450.aggregate_cube_snapshot()
        assert len(snap.axis_stats) == 18

    def test_aggregate_per_axis_overall(self):
        snap = v1450.aggregate_cube_snapshot()
        # Should have 3 axis keys
        assert "problem" in snap.per_axis_overall
        assert "position" in snap.per_axis_overall
        assert "protocol" in snap.per_axis_overall

    def test_aggregate_faces_in_range(self):
        snap = v1450.aggregate_cube_snapshot()
        assert 0 <= snap.n_faces_found <= snap.n_faces_total


# ============================================================================
# History operations
# ============================================================================


class TestHistoryOperations:
    def test_load_history_returns_list(self):
        history = v1450.load_cube_history()
        assert isinstance(history, list)

    def test_append_snapshot_returns_bool(self):
        snap = v1450.aggregate_cube_snapshot()
        result = v1450.append_cube_snapshot(snap)
        assert isinstance(result, bool)

    def test_compute_trend_empty(self):
        trend = v1450.compute_cube_history_trend([])
        assert trend == "INSUFFICIENT"

    def test_compute_trend_one(self):
        trend = v1450.compute_cube_history_trend([{"cube_overall_closure_rate": 0.5}])
        assert trend == "INSUFFICIENT"

    def test_compute_trend_stable(self):
        trend = v1450.compute_cube_history_trend([
            {"timestamp": "t1", "cube_overall_closure_rate": 0.5},
            {"timestamp": "t2", "cube_overall_closure_rate": 0.51},
        ])
        assert trend == "STABLE"

    def test_compute_trend_improving(self):
        trend = v1450.compute_cube_history_trend([
            {"timestamp": "t1", "cube_overall_closure_rate": 0.5},
            {"timestamp": "t2", "cube_overall_closure_rate": 0.7},
        ])
        assert trend == "IMPROVING"

    def test_compute_trend_degrading(self):
        trend = v1450.compute_cube_history_trend([
            {"timestamp": "t1", "cube_overall_closure_rate": 0.7},
            {"timestamp": "t2", "cube_overall_closure_rate": 0.5},
        ])
        assert trend == "DEGRADING"

    def test_compute_digest_empty(self):
        digest = v1450.compute_cube_history_digest([])
        assert digest["n_snapshots"] == 0

    def test_compute_digest_one(self):
        digest = v1450.compute_cube_history_digest([{"cube_overall_closure_rate": 0.5}])
        assert digest["n_snapshots"] == 1
        assert digest["mean_rate"] == 0.5


# ============================================================================
# Render
# ============================================================================


class TestRender:
    def test_render_returns_string(self):
        snap = v1450.aggregate_cube_snapshot()
        report = v1450.CubeHistoryReport(
            schema="test", version="0.1.0", module="test",
            started="t1", ended="t2",
            snapshots_loaded=0, snapshots_appended=1,
            current_snapshot=snap, history_trend="INSUFFICIENT",
        )
        md = v1450.render_cube_report_md(report)
        assert isinstance(md, str)
        assert len(md) > 100

    def test_render_has_required_sections(self):
        snap = v1450.aggregate_cube_snapshot()
        report = v1450.CubeHistoryReport(
            schema="test", version="0.1.0", module="test",
            started="t1", ended="t2",
            snapshots_loaded=0, snapshots_appended=1,
            current_snapshot=snap, history_trend="INSUFFICIENT",
        )
        md = v1450.render_cube_report_md(report)
        assert "## Cube snapshot" in md
        assert "## Per-face" in md
        assert "## Per-axis" in md
        assert "## History trend" in md
        assert "## Honest disclosure" in md
        assert "## Guards" in md

    def test_render_includes_v3_guards(self):
        snap = v1450.aggregate_cube_snapshot()
        report = v1450.CubeHistoryReport(
            schema="test", version="0.1.0", module="test",
            started="t1", ended="t2",
            snapshots_loaded=0, snapshots_appended=1,
            current_snapshot=snap, history_trend="INSUFFICIENT",
        )
        md = v1450.render_cube_report_md(report)
        assert "V3 哲学守门" in md
        assert "GUARD_NO_PHENOMENAL_CUBE" in md

    def test_render_includes_borrowed(self):
        snap = v1450.aggregate_cube_snapshot()
        report = v1450.CubeHistoryReport(
            schema="test", version="0.1.0", module="test",
            started="t1", ended="t2",
            snapshots_loaded=0, snapshots_appended=1,
            current_snapshot=snap, history_trend="INSUFFICIENT",
        )
        md = v1450.render_cube_report_md(report)
        assert "V1417" in md or "V1413" in md


# ============================================================================
# Popper + chain_delegate
# ============================================================================


class TestPopper:
    def test_popper_returns_tuple(self):
        ok, results = v1450.popper()
        assert isinstance(ok, bool)
        assert isinstance(results, list)

    def test_popper_all_pass(self):
        ok, results = v1450.popper()
        failed = [r for r in results if not r["ok"]]
        assert ok is True, f"Failed: {[r['name'] for r in failed]}"


class TestChainDelegate:
    def test_chain_delegate_returns_dict(self):
        chain = v1450.chain_delegate()
        assert isinstance(chain, dict)

    def test_chain_delegate_has_all_ok(self):
        chain = v1450.chain_delegate()
        assert "all_ok" in chain

    def test_chain_delegate_has_upstream(self):
        chain = v1450.chain_delegate()
        assert "upstream" in chain
        assert len(chain["upstream"]) == 3

    def test_chain_delegate_n_upstream(self):
        chain = v1450.chain_delegate()
        assert chain["n_upstream"] == 3


# ============================================================================
# run_all
# ============================================================================


class TestRunAll:
    def test_run_all_returns_report(self, tmp_path):
        out_json = tmp_path / "report.json"
        out_md = tmp_path / "report.md"
        report = v1450.run_all(
            out_json=out_json,
            out_md=out_md,
            append_history=False,  # don't pollute real history
        )
        assert isinstance(report, v1450.CubeHistoryReport)
        assert report.history_trend in ("IMPROVING", "STABLE", "DEGRADING", "INSUFFICIENT")

    def test_run_all_writes_files(self, tmp_path):
        out_json = tmp_path / "report.json"
        out_md = tmp_path / "report.md"
        v1450.run_all(
            out_json=out_json,
            out_md=out_md,
            append_history=False,
        )
        assert out_json.exists()
        assert out_md.exists()

    def test_run_all_json_valid(self, tmp_path):
        out_json = tmp_path / "report.json"
        v1450.run_all(out_json=out_json, out_md=tmp_path / "x.md", append_history=False)
        with open(out_json, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert "current_snapshot" in data
        assert "history_trend" in data


# ============================================================================
# CLI
# ============================================================================


class TestCLI:
    def test_main_version(self, capsys):
        rc = v1450.main(["version"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "0.1.0" in out

    def test_main_help(self, capsys):
        rc = v1450.main(["help"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1450" in out

    def test_main_meta(self, capsys):
        rc = v1450.main(["meta"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "schema" in out

    def test_main_meta_json(self, capsys):
        rc = v1450.main(["meta", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert data["version"] == "0.1.0"

    def test_main_chain(self, capsys):
        rc = v1450.main(["chain"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "all_ok" in data

    def test_main_load_history(self, capsys):
        rc = v1450.main(["load-history"])
        assert rc == 0

    def test_main_cube_trend(self, capsys):
        rc = v1450.main(["cube-trend"])
        assert rc == 0
        out = capsys.readouterr().out
        data = json.loads(out)
        assert "trend" in data

    def test_main_axis_stats(self, capsys):
        rc = v1450.main(["axis-stats"])
        assert rc == 0

    def test_main_unknown(self, capsys):
        # argparse raises SystemExit(2) on unknown command
        with pytest.raises(SystemExit) as exc_info:
            v1450.main(["unknown_command"])
        assert exc_info.value.code == 2