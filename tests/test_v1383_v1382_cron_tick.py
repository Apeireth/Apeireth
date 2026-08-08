"""Tests for V1383 — V1382 cron-driven snapshot tick + dashboard.

Run from promethean/:
    python -m pytest tests/test_v1383_v1382_cron_tick.py -v
"""

from __future__ import annotations

import datetime as _dt
import io
import json
import os
import sys
import tempfile
from contextlib import redirect_stderr, redirect_stdout

import pytest

import apeireth.v1383_v1382_cron_tick as v1383


# ----------------------------------------------------------------------
# Constants + GUARDS
# ----------------------------------------------------------------------

def test_schema_version_constant():
    assert v1383.SCHEMA_VERSION == "v1383.tick/v1"


def test_dashboard_schema_version_constant():
    assert v1383.DASHBOARD_SCHEMA_VERSION == "v1383.dashboard/v1"


def test_script_name_constant():
    assert v1383.SCRIPT_NAME == "v1383_v1382_cron_tick"


def test_default_paths_constants():
    assert v1383.DEFAULT_LEDGER_PATH.endswith(".jsonl")
    assert v1383.DEFAULT_DASHBOARD_PATH.endswith(".md")


def test_guards_tuple_length():
    assert len(v1383.V1383_GUARDS) == 10


def test_guards_required_members():
    required = (
        "GUARD_CRON_SAFE",
        "GUARD_HISTORY_APPEND_ONLY",
        "GUARD_NO_CAP_CHANGE",
        "GUARD_DETERMINISTIC",
        "GUARD_ATOMIC_WRITE",
        "GUARD_NO_TOUCH_V1382",
        "GUARD_LOCAL_FILESYSTEM_ONLY",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_DASHBOARD_PURE",
        "GUARD_DRIFT_PURE",
    )
    for g in required:
        assert g in v1383.V1383_GUARDS, f"missing guard: {g}"


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def test_validate_safe_path_passes_safe():
    v1383._validate_safe_path("a/b/c.jsonl")
    v1383._validate_safe_path("V1383_TICKS.jsonl")
    v1383._validate_safe_path("a\\b\\c.jsonl")


def test_validate_safe_path_rejects_parent_traversal():
    with pytest.raises(ValueError):
        v1383._validate_safe_path("../escape/file.jsonl")
    with pytest.raises(ValueError):
        v1383._validate_safe_path("a/../../b/c.jsonl")


# ----------------------------------------------------------------------
# Tick id
# ----------------------------------------------------------------------

def test_make_tick_id_is_deterministic():
    ts = _dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc)
    id1 = v1383._make_tick_id(ts)
    id2 = v1383._make_tick_id(ts)
    assert id1 == id2


def test_make_tick_id_has_expected_format():
    ts = _dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc)
    tick_id = v1383._make_tick_id(ts)
    assert tick_id.startswith("tick-")
    assert tick_id.endswith("-" + tick_id.split("-")[-1])  # hex suffix
    parts = tick_id.split("-")
    # tick-YYYY-MM-DDTHH-MM-SSZ-XXXX
    assert len(parts[-1]) == 4
    assert parts[-1].isalnum()


def test_make_tick_id_differs_across_microseconds():
    ts1 = _dt.datetime(2026, 8, 9, 5, 20, 0, 0, tzinfo=_dt.timezone.utc)
    ts2 = _dt.datetime(2026, 8, 9, 5, 20, 0, 123456, tzinfo=_dt.timezone.utc)
    id1 = v1383._make_tick_id(ts1)
    id2 = v1383._make_tick_id(ts2)
    assert id1 != id2


# ----------------------------------------------------------------------
# Atomic JSONL append + read
# ----------------------------------------------------------------------

def test_atomic_append_jsonl_creates_file():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        rec = {"schema": v1383.SCHEMA_VERSION, "tick_id": "t1"}
        v1383._atomic_append_jsonl(p, rec)
        assert os.path.isfile(p)
        with open(p, "r", encoding="utf-8") as fh:
            content = fh.read()
        # One JSON object + newline
        lines = [l for l in content.split("\n") if l]
        assert len(lines) == 1
        parsed = json.loads(lines[0])
        assert parsed["tick_id"] == "t1"


def test_atomic_append_jsonl_appends_multiple():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        for i in range(5):
            v1383._atomic_append_jsonl(p, {"tick_id": f"t{i}"})
        ticks = v1383._read_ticks(p)
        assert len(ticks) == 5


def test_atomic_append_jsonl_rejects_traversal():
    with pytest.raises(ValueError):
        v1383._atomic_append_jsonl("../escape/ticks.jsonl", {"tick_id": "x"})


def test_read_ticks_empty_returns_list():
    with tempfile.TemporaryDirectory() as td:
        empty = os.path.join(td, "empty.jsonl")
        # nonexistent
        assert v1383._read_ticks(empty) == []
        # exists but empty
        open(empty, "w").close()
        assert v1383._read_ticks(empty) == []


def test_read_ticks_reverse_default():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        for i in range(3):
            v1383._atomic_append_jsonl(p, {"tick_id": f"t{i}"})
        ticks = v1383._read_ticks(p)  # default reverse=True
        assert [t["tick_id"] for t in ticks] == ["t2", "t1", "t0"]


def test_read_ticks_reverse_false():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        for i in range(3):
            v1383._atomic_append_jsonl(p, {"tick_id": f"t{i}"})
        ticks = v1383._read_ticks(p, reverse=False)
        assert [t["tick_id"] for t in ticks] == ["t0", "t1", "t2"]


def test_read_ticks_limit():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        for i in range(5):
            v1383._atomic_append_jsonl(p, {"tick_id": f"t{i}"})
        ticks = v1383._read_ticks(p, limit=2)
        assert len(ticks) == 2


def test_read_ticks_skips_malformed_lines():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "mixed.jsonl")
        with open(p, "w", encoding="utf-8") as fh:
            fh.write('{"tick_id":"good"}\n')
            fh.write("\n")
            fh.write("not json\n")
            fh.write('{"tick_id":"good2"}\n')
        ticks = v1383._read_ticks(p)
        assert len(ticks) == 2
        assert [t["tick_id"] for t in ticks] == ["good2", "good"]


def test_count_total():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "ticks.jsonl")
        assert v1383._count_total(p) == 0
        for i in range(3):
            v1383._atomic_append_jsonl(p, {"tick_id": f"t{i}"})
        assert v1383._count_total(p) == 3


# ----------------------------------------------------------------------
# Drift computation
# ----------------------------------------------------------------------

def _make_tick(archives, integ_ok, tiers, actions, tick_id="t", ts="2026-08-09T05:00:00Z"):
    return {
        "tick_id": tick_id,
        "ts": ts,
        "v1382_snapshot": {
            "totals": {"archives": archives},
            "integrity": {"ok": integ_ok},
            "tier_counts": tiers,
            "action_counts": actions,
        },
    }


def test_compute_drift_archives_delta():
    prev = _make_tick(3, True,
                      {"HOT": 1, "WARM": 2, "COLD": 0, "FROZEN": 0},
                      {"keep": 3, "compress": 0, "prune": 0},
                      tick_id="prev", ts="2026-08-09T05:00:00Z")
    curr = _make_tick(5, False,
                      {"HOT": 2, "WARM": 1, "COLD": 1, "FROZEN": 1},
                      {"keep": 2, "compress": 1, "prune": 1},
                      tick_id="curr", ts="2026-08-09T05:05:00Z")
    d = v1383._compute_drift(prev, curr)
    assert d["archives_delta"] == 2
    assert d["integrity_status_delta"] == "ok->broken"
    assert d["tier_distribution_delta"]["HOT"] == 1
    assert d["tier_distribution_delta"]["WARM"] == -1
    assert d["tier_distribution_delta"]["COLD"] == 1
    assert d["tier_distribution_delta"]["FROZEN"] == 1
    assert d["integrity_changed"] is True
    assert d["archives_changed"] is True


def test_compute_drift_no_change():
    prev = _make_tick(3, True,
                      {"HOT": 3, "WARM": 0, "COLD": 0, "FROZEN": 0},
                      {"keep": 3, "compress": 0, "prune": 0})
    curr = _make_tick(3, True,
                      {"HOT": 3, "WARM": 0, "COLD": 0, "FROZEN": 0},
                      {"keep": 3, "compress": 0, "prune": 0})
    d = v1383._compute_drift(prev, curr)
    assert d["archives_delta"] == 0
    assert d["integrity_status_delta"] == "ok->ok"
    assert d["integrity_changed"] is False
    assert d["archives_changed"] is False


def test_compute_drift_integrity_recovery():
    prev = _make_tick(3, False, {"HOT": 3, "WARM": 0, "COLD": 0, "FROZEN": 0},
                      {"keep": 3, "compress": 0, "prune": 0})
    curr = _make_tick(3, True, {"HOT": 3, "WARM": 0, "COLD": 0, "FROZEN": 0},
                      {"keep": 3, "compress": 0, "prune": 0})
    d = v1383._compute_drift(prev, curr)
    assert d["integrity_status_delta"] == "broken->ok"
    assert d["integrity_changed"] is True


# ----------------------------------------------------------------------
# Tick construction
# ----------------------------------------------------------------------

def test_compute_tick_first_tick():
    now = _dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc)
    tick = v1383._compute_tick(now=now, tag="test",
                               archive_dir="/nonexistent/_v1383_pytest_dir",
                               manifest_path="/nonexistent/_v1383_pytest_manifest.json")
    assert tick["schema"] == v1383.SCHEMA_VERSION
    assert tick["tick_id"].startswith("tick-")
    assert tick["ts"] == "2026-08-09T05:20:00Z"
    assert tick.get("first_tick") is True
    assert tick.get("drift_from_previous") is None
    assert tick.get("tag") == "test"
    assert tick["v1383_version"] == "0.1.0"
    assert tick["v1382_snapshot"]["schema"] == "v1382.overlay/v1"
    assert tick["v1382_snapshot"]["totals"]["archives"] == 0
    assert tick["v1382_snapshot"]["integrity"]["ok"] is False
    assert "GUARD_CRON_SAFE" in tick["guards"]
    assert isinstance(tick["known_unknowns"], list)
    assert len(tick["known_unknowns"]) >= 1


def test_compute_tick_with_prev_tick():
    now = _dt.datetime(2026, 8, 9, 5, 25, 0, tzinfo=_dt.timezone.utc)
    prev_tick = {
        "tick_id": "tick-prev",
        "ts": "2026-08-09T05:20:00Z",
        "v1382_snapshot": {
            "totals": {"archives": 1},
            "integrity": {"ok": True},
            "tier_counts": {"HOT": 1, "WARM": 0, "COLD": 0, "FROZEN": 0},
            "action_counts": {"keep": 1, "compress": 0, "prune": 0},
        },
    }
    tick = v1383._compute_tick(now=now, tag=None,
                               archive_dir="/nonexistent/_v1383_pytest_dir",
                               manifest_path="/nonexistent/_v1383_pytest_manifest.json",
                               prev_tick=prev_tick)
    assert tick.get("first_tick") is False
    assert tick.get("drift_from_previous") is not None
    assert tick["drift_from_previous"]["previous_tick_id"] == "tick-prev"
    assert tick["drift_from_previous"]["archives_delta"] == -1  # 0 - 1


def test_compute_tick_omit_tag_when_none():
    now = _dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc)
    tick = v1383._compute_tick(now=now, tag=None,
                               archive_dir="/nonexistent/_v1383_pytest_dir",
                               manifest_path="/nonexistent/_v1383_pytest_manifest.json")
    assert "tag" not in tick


# ----------------------------------------------------------------------
# Render helpers
# ----------------------------------------------------------------------

def test_render_tick_one_line_contains_key_fields():
    tick = {
        "ts": "2026-08-09T05:20:00Z",
        "tick_id": "tick-x",
        "tag": "my-tag",
        "v1382_snapshot": {
            "totals": {"archives": 5},
            "integrity": {"ok": True},
        },
    }
    line = v1383._render_tick_one_line(tick)
    assert "tick-x" in line
    assert "archives=5" in line
    assert "OK" in line
    assert "my-tag" in line


def test_render_dashboard_md_empty():
    md = v1383._render_dashboard_md([])
    assert "no ticks" in md.lower()


def test_render_dashboard_md_with_one_tick():
    sample = {
        "schema": v1383.SCHEMA_VERSION,
        "tick_id": "tick-2026-08-09T05-20-00Z-abcd",
        "ts": "2026-08-09T05:20:00Z",
        "v1383_version": "0.1.0",
        "first_tick": True,
        "drift_from_previous": None,
        "tag": "cron-5min",
        "guards": list(v1383.V1383_GUARDS),
        "known_unknowns": ["x"],
        "v1382_snapshot": {
            "schema": "v1382.overlay/v1",
            "totals": {"archives": 7, "indexed": 6, "manifested": 7},
            "tier_counts": {"HOT": 5, "WARM": 1, "COLD": 1, "FROZEN": 0},
            "action_counts": {"keep": 5, "compress": 1, "prune": 1},
            "integrity": {"ok": True, "missing_on_disk": [], "extra_on_disk": []},
            "rotation": {
                "policy_version": "v1381.rotation.policy/v1",
                "plan_path": "V1381_PLAN_AUTO.md",
                "actions_summary": {"keep": 5, "compress": 1, "prune": 1},
            },
        },
    }
    md = v1383._render_dashboard_md([sample])
    assert "v1383.dashboard/v1" in md
    assert "tick-2026-08-09T05-20-00Z-abcd" in md
    assert "archives on disk:** 7" in md
    assert "integrity:** `OK`" in md
    assert "first tick" in md.lower()
    assert "GUARD_CRON_SAFE" in md
    assert "| HOT | 5 |" in md
    assert "| keep | 5 |" in md


def test_render_dashboard_md_with_broken_integrity():
    sample = {
        "schema": v1383.SCHEMA_VERSION,
        "tick_id": "tick-broken",
        "ts": "2026-08-09T05:20:00Z",
        "v1383_version": "0.1.0",
        "first_tick": True,
        "drift_from_previous": None,
        "guards": list(v1383.V1383_GUARDS),
        "known_unknowns": [],
        "v1382_snapshot": {
            "totals": {"archives": 0, "indexed": 0, "manifested": 0},
            "tier_counts": {"HOT": 0, "WARM": 0, "COLD": 0, "FROZEN": 0},
            "action_counts": {"keep": 0, "compress": 0, "prune": 0},
            "integrity": {
                "manifest_present": False,
                "ok": False,
                "reason": "no manifest found",
                "missing_on_disk": [],
                "extra_on_disk": [],
            },
            "rotation": {
                "policy_version": "v1381.rotation.policy/v1",
                "plan_path": "V1381_PLAN_AUTO.md",
                "actions_summary": {"keep": 0, "compress": 0, "prune": 0},
            },
        },
    }
    md = v1383._render_dashboard_md([sample])
    assert "BROKEN" in md
    assert "no manifest found" in md


def test_render_dashboard_md_with_drift():
    sample = {
        "schema": v1383.SCHEMA_VERSION,
        "tick_id": "tick-d",
        "ts": "2026-08-09T05:25:00Z",
        "v1383_version": "0.1.0",
        "first_tick": False,
        "drift_from_previous": {
            "archives_delta": 3,
            "integrity_status_delta": "ok->broken",
            "tier_distribution_delta": {"HOT": 2, "WARM": -1, "COLD": 1, "FROZEN": 1},
            "action_counts_delta": {"keep": 1, "compress": 1, "prune": 1},
            "integrity_changed": True,
            "archives_changed": True,
            "previous_tick_id": "tick-p",
            "previous_ts": "2026-08-09T05:20:00Z",
        },
        "guards": list(v1383.V1383_GUARDS),
        "known_unknowns": [],
        "v1382_snapshot": {
            "totals": {"archives": 5, "indexed": 5, "manifested": 5},
            "tier_counts": {"HOT": 3, "WARM": 0, "COLD": 1, "FROZEN": 1},
            "action_counts": {"keep": 3, "compress": 1, "prune": 1},
            "integrity": {"ok": False, "missing_on_disk": [], "extra_on_disk": []},
            "rotation": {
                "policy_version": "v1381.rotation.policy/v1",
                "plan_path": "V1381_PLAN_AUTO.md",
                "actions_summary": {"keep": 3, "compress": 1, "prune": 1},
            },
        },
    }
    md = v1383._render_dashboard_md([sample])
    assert "archives_delta:** `3`" in md
    assert "ok->broken" in md


def test_render_dashboard_md_includes_last_n_table():
    ticks = []
    for i in range(12):
        ticks.append({
            "schema": v1383.SCHEMA_VERSION,
            "tick_id": f"tick-{i}",
            "ts": f"2026-08-09T05:{i:02d}:00Z",
            "v1383_version": "0.1.0",
            "first_tick": (i == 0),
            "drift_from_previous": None,
            "tag": f"tag-{i}",
            "guards": list(v1383.V1383_GUARDS),
            "known_unknowns": [],
            "v1382_snapshot": {
                "totals": {"archives": i, "indexed": i, "manifested": i},
                "tier_counts": {"HOT": i, "WARM": 0, "COLD": 0, "FROZEN": 0},
                "action_counts": {"keep": i, "compress": 0, "prune": 0},
                "integrity": {"ok": True, "missing_on_disk": [], "extra_on_disk": []},
                "rotation": {
                    "policy_version": "v1381.rotation.policy/v1",
                    "plan_path": "V1381_PLAN_AUTO.md",
                    "actions_summary": {"keep": i, "compress": 0, "prune": 0},
                },
            },
        })
    md = v1383._render_dashboard_md(ticks[:10])
    # Cap at 10 in dashboard
    assert "Last 10 ticks" in md
    md_all = v1383._render_dashboard_md(ticks)
    # Dashboard caps at 10 — 12 ticks still shows "Last 10 ticks"
    assert "Last 10 ticks" in md_all


# ----------------------------------------------------------------------
# tick_now end-to-end (uses V1382 against a temp dir)
# ----------------------------------------------------------------------

def test_tick_now_against_nonexistent_dir():
    """tick_now against a nonexistent archive dir should still produce a tick."""
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        tick = v1383.tick_now(
            tag="pytest",
            ledger_path=ledger,
            archive_dir=os.path.join(td, "nonexistent_dir"),
            manifest_path=os.path.join(td, "nonexistent_manifest.json"),
        )
        assert tick["schema"] == v1383.SCHEMA_VERSION
        assert tick.get("first_tick") is True
        # Verify ledger has the tick
        assert os.path.isfile(ledger)
        ticks = v1383._read_ticks(ledger)
        assert len(ticks) == 1
        assert ticks[0]["tick_id"] == tick["tick_id"]


def test_tick_now_idempotent_under_repeated_calls():
    """Repeated tick_now calls append to ledger without corruption."""
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        # First tick (no archive dir, returns empty snapshot)
        for i in range(3):
            v1383.tick_now(
                tag=f"iter-{i}",
                ledger_path=ledger,
                archive_dir=os.path.join(td, "no_archive"),
                manifest_path=os.path.join(td, "no_manifest.json"),
                now=_dt.datetime(2026, 8, 9, 5, 20 + i, 0, tzinfo=_dt.timezone.utc),
            )
        ticks = v1383._read_ticks(ledger)
        assert len(ticks) == 3
        # Latest tick (newest first) should be iter-2
        assert ticks[0]["tag"] == "iter-2"


def test_tick_now_drift_after_second_call():
    """Second tick should carry a non-null drift_from_previous."""
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        archive = os.path.join(td, "archive")
        manifest = os.path.join(td, "manifest.json")
        os.makedirs(archive)
        # Create one archive file in V1375 slug format
        with open(os.path.join(archive, "2026-08-09T05-00-00Z__v1374.md"), "w") as fh:
            fh.write("x")
        # Write a minimal manifest
        with open(manifest, "w", encoding="utf-8") as fh:
            json.dump({
                "schema": "v1379.integrity/v1",
                "records": [{"name": "2026-08-09T05-00-00Z__v1374.md", "sha256": "x" * 64}],
            }, fh)

        # First tick
        tick1 = v1383.tick_now(
            tag="a",
            ledger_path=ledger,
            archive_dir=archive,
            manifest_path=manifest,
            now=_dt.datetime(2026, 8, 9, 5, 20, 0, tzinfo=_dt.timezone.utc),
        )
        assert tick1.get("first_tick") is True
        assert tick1.get("drift_from_previous") is None

        # Second tick (same archive state)
        tick2 = v1383.tick_now(
            tag="b",
            ledger_path=ledger,
            archive_dir=archive,
            manifest_path=manifest,
            now=_dt.datetime(2026, 8, 9, 5, 25, 0, tzinfo=_dt.timezone.utc),
        )
        assert tick2.get("first_tick") is False
        assert tick2.get("drift_from_previous") is not None
        assert tick2["drift_from_previous"]["archives_delta"] == 0


# ----------------------------------------------------------------------
# Atomic text write
# ----------------------------------------------------------------------

def test_atomic_write_text_creates_file():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "out.md")
        v1383._atomic_write_text(p, "# hello\n\nbody\n")
        with open(p, "r", encoding="utf-8") as fh:
            assert fh.read() == "# hello\n\nbody\n"


def test_atomic_write_text_creates_parent_dir():
    with tempfile.TemporaryDirectory() as td:
        p = os.path.join(td, "subdir", "deep", "out.md")
        v1383._atomic_write_text(p, "x")
        assert os.path.isfile(p)


def test_atomic_write_text_rejects_traversal():
    with pytest.raises(ValueError):
        v1383._atomic_write_text("../escape.md", "x")


# ----------------------------------------------------------------------
# CLI dispatch
# ----------------------------------------------------------------------

def test_cli_version_exit_0():
    rc = v1383.run_cli(["version"])
    assert rc == 0


def test_cli_show_last_empty_exit_0():
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "empty.jsonl")
        rc = v1383.run_cli(["--ledger-path", ledger, "show-last", "--n", "3"])
        assert rc == 0


def test_cli_summary_empty_exit_0():
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "empty.jsonl")
        rc = v1383.run_cli(["--ledger-path", ledger, "summary"])
        assert rc == 0


def test_cli_drift_empty_exit_1():
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "empty.jsonl")
        rc = v1383.run_cli(["--ledger-path", ledger, "drift"])
        assert rc == 1


def test_cli_drift_with_two_ticks_exit_0():
    """Two ticks → drift succeeds (exit 0)."""
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        archive = os.path.join(td, "archive")
        manifest = os.path.join(td, "manifest.json")
        os.makedirs(archive)
        with open(os.path.join(archive, "2026-08-09T05-00-00Z__v1374.md"), "w") as fh:
            fh.write("x")
        with open(manifest, "w", encoding="utf-8") as fh:
            json.dump({
                "schema": "v1379.integrity/v1",
                "records": [{"name": "2026-08-09T05-00-00Z__v1374.md", "sha256": "x" * 64}],
            }, fh)
        v1383.tick_now(
            tag="x", ledger_path=ledger,
            archive_dir=archive, manifest_path=manifest,
            now=_dt.datetime(2026, 8, 9, 5, 0, 0, tzinfo=_dt.timezone.utc),
        )
        v1383.tick_now(
            tag="y", ledger_path=ledger,
            archive_dir=archive, manifest_path=manifest,
            now=_dt.datetime(2026, 8, 9, 5, 5, 0, tzinfo=_dt.timezone.utc),
        )
        rc = v1383.run_cli(["--ledger-path", ledger, "drift"])
        assert rc == 0


def test_cli_dashboard_to_stdout_exit_0():
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        archive = os.path.join(td, "archive")
        manifest = os.path.join(td, "manifest.json")
        os.makedirs(archive)
        with open(os.path.join(archive, "2026-08-09T05-00-00Z__v1374.md"), "w") as fh:
            fh.write("x")
        with open(manifest, "w", encoding="utf-8") as fh:
            json.dump({
                "schema": "v1379.integrity/v1",
                "records": [{"name": "2026-08-09T05-00-00Z__v1374.md", "sha256": "x" * 64}],
            }, fh)
        v1383.tick_now(
            tag="dash", ledger_path=ledger,
            archive_dir=archive, manifest_path=manifest,
            now=_dt.datetime(2026, 8, 9, 5, 0, 0, tzinfo=_dt.timezone.utc),
        )
        # Capture stdout
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = v1383.run_cli(["--ledger-path", ledger, "dashboard"])
        assert rc == 0
        out = buf.getvalue()
        assert "v1383.dashboard/v1" in out


def test_cli_dashboard_to_file_exit_0_empty_ledger():
    """Empty ledger dashboard should still produce a markdown file."""
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        out = os.path.join(td, "dash.md")
        rc = v1383.run_cli(["--ledger-path", ledger, "dashboard", "--out", out])
        assert rc == 0
        assert os.path.isfile(out)
        with open(out, "r", encoding="utf-8") as fh:
            content = fh.read()
        assert "V1383 archive-health dashboard" in content
        assert "no ticks" in content.lower()


def test_cli_tick_exits_0_and_appends():
    """tick CLI should append to the ledger and exit 0."""
    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        archive = os.path.join(td, "archive")
        manifest = os.path.join(td, "manifest.json")
        os.makedirs(archive)
        with open(os.path.join(archive, "2026-08-09T05-00-00Z__v1374.md"), "w") as fh:
            fh.write("x")
        with open(manifest, "w", encoding="utf-8") as fh:
            json.dump({
                "schema": "v1379.integrity/v1",
                "records": [{"name": "2026-08-09T05-00-00Z__v1374.md", "sha256": "x" * 64}],
            }, fh)
        rc = v1383.run_cli([
            "--ledger-path", ledger,
            "--archive-dir", archive,
            "--manifest-path", manifest,
            "tick", "--tag", "cli-test",
        ])
        assert rc == 0
        ticks = v1383._read_ticks(ledger)
        assert len(ticks) == 1
        assert ticks[0]["tag"] == "cli-test"


def test_cli_popper_does_not_recurse():
    """Direct popper CLI call must not recurse infinitely.

    The V1381/V1382 self-test bug was: popper → run_cli(["popper"]) → ...
    V1383 must avoid this by NOT calling run_cli(["popper"]) inside
    _popper_self_tests.
    """
    # This test would hang for >5s if recursion existed
    import time
    start = time.time()
    rc = v1383.run_cli(["popper"])
    elapsed = time.time() - start
    assert elapsed < 5.0, f"popper CLI took {elapsed:.2f}s — possible recursion"
    # popper returns 0 on full pass
    assert rc == 0


# ----------------------------------------------------------------------
# V1382 integration: real-data smoke against promethean/V1375_HISTORY
# ----------------------------------------------------------------------

def test_real_data_tick_smoke():
    """Real-data: tick against the live V1375_HISTORY/ + V1379 manifest.

    This test must be run from promethean/ where V1375_HISTORY/ lives.
    Skipped if the archive dir doesn't exist (e.g., from a partial clone).
    """
    archive_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                               "V1375_HISTORY")
    manifest_path = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                 "V1379_INTEGRITY_AUTO.json")
    if not os.path.isdir(archive_dir):
        pytest.skip(f"V1375_HISTORY not found at {archive_dir}")

    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        tick = v1383.tick_now(
            tag="real-data-smoke",
            ledger_path=ledger,
            archive_dir=archive_dir,
            manifest_path=manifest_path,
            now=_dt.datetime(2026, 8, 9, 5, 30, 0, tzinfo=_dt.timezone.utc),
        )
        # Should produce a non-trivial snapshot from real archive
        assert tick["v1382_snapshot"]["totals"]["archives"] >= 1
        # V1379 manifest is built when V1379 is run; integrity may be ok
        assert "integrity" in tick["v1382_snapshot"]
        # Tick record is well-formed
        assert tick["tick_id"].startswith("tick-")
        assert tick["first_tick"] is True  # first tick in temp ledger


def test_real_data_dashboard_smoke():
    """Real-data: dashboard from live V1375_HISTORY/ via CLI."""
    archive_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                               "V1375_HISTORY")
    manifest_path = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                                 "V1379_INTEGRITY_AUTO.json")
    if not os.path.isdir(archive_dir):
        pytest.skip(f"V1375_HISTORY not found at {archive_dir}")

    with tempfile.TemporaryDirectory() as td:
        ledger = os.path.join(td, "ticks.jsonl")
        out = os.path.join(td, "dash.md")
        # Run tick + dashboard
        rc = v1383.run_cli([
            "--ledger-path", ledger,
            "--archive-dir", archive_dir,
            "--manifest-path", manifest_path,
            "tick", "--tag", "real-dash",
        ])
        assert rc == 0
        rc = v1383.run_cli([
            "--ledger-path", ledger,
            "dashboard", "--out", out,
        ])
        assert rc == 0
        with open(out, "r", encoding="utf-8") as fh:
            content = fh.read()
        assert "v1383.dashboard/v1" in content
        assert "real-dash" in content


# ----------------------------------------------------------------------
# Popper self-test invocation
# ----------------------------------------------------------------------

def test_popper_self_tests_pass():
    passed, total, failures = v1383._popper_self_tests()
    assert passed == total, f"popper failed: {failures}"
    assert total == 52, f"expected 52 popper checks, got {total}"
