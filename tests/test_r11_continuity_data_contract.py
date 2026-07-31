"""R11 real SQLite contract tests for ContinuityTracker/dashboard payload."""
from __future__ import annotations

import json
import sqlite3
from pathlib import Path

import pytest

from apeireth.v1072_asi_central_ai_eternal_identity import ContinuityTracker
from apeireth.v1130_continuity_tracker_dashboard import (
    CONTINUITY_SCHEMA_VERSION,
    ContinuityDashboard,
    ContinuitySnapshotStore,
    DashboardConfig,
)
from apeireth.v1136_asi_v05_3dim_real_measurement import V1136Result


def test_migration_preserves_unrelated_legacy_rows(tmp_path: Path) -> None:
    db = tmp_path / "legacy.sqlite3"
    with sqlite3.connect(db) as conn:
        conn.execute("CREATE TABLE legacy_dashboard(id INTEGER PRIMARY KEY, payload TEXT NOT NULL)")
        conn.execute("INSERT INTO legacy_dashboard(payload) VALUES('keep-me')")
    store = ContinuitySnapshotStore(db)
    assert store.migrate() == CONTINUITY_SCHEMA_VERSION
    assert store.migrate() == CONTINUITY_SCHEMA_VERSION
    with sqlite3.connect(db) as conn:
        assert conn.execute("SELECT payload FROM legacy_dashboard").fetchone()[0] == "keep-me"
        assert conn.execute("SELECT value FROM continuity_schema_meta WHERE key='schema_version'").fetchone()[0] == "2"


def test_tracker_round_trip_is_idempotent(tmp_path: Path) -> None:
    store = ContinuitySnapshotStore(tmp_path / "contract.sqlite3")
    tracker = ContinuityTracker()
    sid = tracker.start_session()
    tracker.sessions[sid].n_entries_added = 7
    tracker.sessions[sid].n_importance_avg = 0.75
    tracker.end_session(sid)
    assert store.persist_tracker("identity-real", tracker) == 1
    tracker.sessions[sid].n_entries_added = 9
    assert store.persist_tracker("identity-real", tracker) == 1
    with sqlite3.connect(store.db_path) as conn:
        row = conn.execute(
            "SELECT n_entries_added,is_active,tracker_version FROM continuity_session"
        ).fetchone()
        assert row == (9, 0, "v1072")
        assert conn.execute("SELECT COUNT(*) FROM continuity_session").fetchone()[0] == 1


def _real_result(ts: float) -> V1136Result:
    details = {
        "sub_scores": {"real_sqlite_source": 0.8},
        "implemented": 1,
        "total": 1,
        "failed": 0,
    }
    return V1136Result(
        continuity=0.81, autonomy=0.82, transferability=0.83,
        v05_total_v1136=0.85, v05_total_v1125=0.85, v04_score=0.85,
        delta_v05_total=0.0, continuity_detail=details,
        autonomy_detail=details, transferability_detail=details,
        chaos_report=None, v3_guards_pass=True, elapsed_seconds=0.01, timestamp=ts,
    )


def test_v1136_snapshot_sources_and_versions_are_traceable(tmp_path: Path) -> None:
    store = ContinuitySnapshotStore(tmp_path / "contract.sqlite3")
    snapshot_id = store.persist_snapshot("identity-real", _real_result(1000.0))
    with sqlite3.connect(store.db_path) as conn:
        snap = conn.execute(
            "SELECT measurement_version,contract_version,continuity,autonomy,transferability "
            "FROM continuity_snapshot WHERE snapshot_id=?", (snapshot_id,),
        ).fetchone()
        assert snap == ("v1136-0.1.0", 2, 0.81, 0.82, 0.83)
        sources = conn.execute(
            "SELECT dimension,source_name,source_version,detail_json FROM continuity_snapshot_source "
            "WHERE snapshot_id=? ORDER BY dimension", (snapshot_id,),
        ).fetchall()
        assert {row[0] for row in sources} == {"continuity", "autonomy", "transferability"}
        assert all(row[1:3] == ("real_sqlite_source", "v1136-0.1.0") for row in sources)
        assert all(json.loads(row[3])["score"] == 0.8 for row in sources)


def test_snapshot_timeline_uses_identity_time_index(tmp_path: Path) -> None:
    store = ContinuitySnapshotStore(tmp_path / "contract.sqlite3")
    store.persist_snapshot("identity-real", _real_result(2000.0))
    store.persist_snapshot("identity-real", _real_result(1000.0))
    assert [row["measured_at"] for row in store.timeline("identity-real")] == [1000.0, 2000.0]
    with sqlite3.connect(store.db_path) as conn:
        plan = " ".join(str(x) for row in conn.execute(
            "EXPLAIN QUERY PLAN SELECT * FROM continuity_snapshot WHERE identity_id=? ORDER BY measured_at DESC",
            ("identity-real",),
        ) for x in row)
    assert "idx_continuity_snapshot_time" in plan


def test_dashboard_payload_matches_persisted_sessions(tmp_path: Path) -> None:
    config = DashboardConfig(
        out_dir=tmp_path / "out", db_dir=tmp_path / "db", identity_id="identity-dashboard",
        n_sessions=3, benchmark_scales=(1000,), enable_full_stress=False,
    )
    payload = ContinuityDashboard(config).build()
    summary = payload.persistence_summary
    assert summary["schema_version"] == CONTINUITY_SCHEMA_VERSION
    assert summary["persisted_sessions"] == summary["stored_sessions"] == 3
    with sqlite3.connect(summary["db_path"]) as conn:
        assert conn.execute(
            "SELECT COUNT(*) FROM continuity_session WHERE identity_id='identity-dashboard'"
        ).fetchone()[0] == payload.timeline_json["n_sessions"]
