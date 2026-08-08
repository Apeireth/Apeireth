"""Tests for V1382 — V1375 × V1379 × V1381 archive-health overlay.

Run from promethean/:
    python -m pytest tests/test_v1382_v1375_x_v1381_overlay.py -v
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

import apeireth.v1382_v1375_x_v1381_overlay as v1382


# ----------------------------------------------------------------------
# Constants + GUARDS
# ----------------------------------------------------------------------

def test_schema_version_constant():
    assert v1382.SCHEMA_VERSION == "v1382.overlay/v1"


def test_script_name_constant():
    assert v1382.SCRIPT_NAME == "v1382_v1375_x_v1381_overlay"


def test_default_paths_constants():
    assert v1382.DEFAULT_ARCHIVE_DIR == "V1375_HISTORY"
    assert v1382.DEFAULT_INDEX_PATH == "V1375_HISTORY/INDEX.md"
    assert v1382.DEFAULT_MANIFEST_PATH.endswith(".json")
    assert v1382.DEFAULT_HISTORY_PATH.endswith(".jsonl")


# ----------------------------------------------------------------------
# Disk counting
# ----------------------------------------------------------------------

def test_list_archive_names_filters_index_and_non_md():
    with tempfile.TemporaryDirectory() as td:
        for n in [
            "2026-08-09T03-55-00Z__v1374.md",
            "2026-08-09T04-00-00Z__v1374.md",
            "INDEX.md",
            "README.txt",
            "notes.md",  # not a V1375 slug
        ]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("x")
        names = v1382._list_archive_names(td)
        assert "INDEX.md" not in names
        assert "README.txt" not in names
        assert "notes.md" not in names
        assert len(names) == 2


def test_list_archive_names_returns_empty_for_missing_dir():
    names = v1382._list_archive_names("/nonexistent/v1382/test")
    assert names == []


def test_count_archives_counts_only_valid_slugs():
    with tempfile.TemporaryDirectory() as td:
        for n in [
            "2026-08-09T03-55-00Z__v1374.md",
            "INDEX.md",
        ]:
            with open(os.path.join(td, n), "w") as fh:
                fh.write("x")
        assert v1382._count_archives(td) == 1


def test_count_indexed_counts_backtick_slugs():
    with tempfile.TemporaryDirectory() as td:
        idx = os.path.join(td, "INDEX.md")
        with open(idx, "w") as fh:
            fh.write("# Index\n\n| name |\n|---|\n"
                     + "| `2026-08-09T03-55-00Z__v1374.md` | HOT |\n"
                     + "| `2026-08-09T04-00-00Z__v1374.md` | HOT |\n")
        assert v1382._count_indexed(td) == 2


def test_count_indexed_returns_zero_for_missing_index():
    with tempfile.TemporaryDirectory() as td:
        assert v1382._count_indexed(td) == 0


# ----------------------------------------------------------------------
# Manifest reading
# ----------------------------------------------------------------------

def test_read_manifest_returns_none_for_missing():
    assert v1382._read_manifest("/nonexistent/v1382/m.json") is None


def test_read_manifest_returns_none_for_malformed():
    with tempfile.TemporaryDirectory() as td:
        bad = os.path.join(td, "bad.json")
        with open(bad, "w") as fh:
            fh.write("not json {{")
        assert v1382._read_manifest(bad) is None


def test_read_manifest_parses_valid_json():
    with tempfile.TemporaryDirectory() as td:
        good = os.path.join(td, "good.json")
        with open(good, "w") as fh:
            json.dump({"schema_version": "v1379.integrity/v1", "records": []}, fh)
        m = v1382._read_manifest(good)
        assert m is not None
        assert m["schema_version"] == "v1379.integrity/v1"


# ----------------------------------------------------------------------
# Integrity check
# ----------------------------------------------------------------------

def test_integrity_missing_manifest():
    integ = v1382._check_integrity("/nonexistent/v1382/test", None)
    assert integ["ok"] is False
    assert integ["manifest_present"] is False
    assert "reason" in integ


def test_integrity_ok_when_manifest_matches_disk():
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# x\n")
        manifest = {
            "schema_version": "v1379.integrity/v1",
            "records": [{"name": name, "sha256": "x" * 64}],
        }
        integ = v1382._check_integrity(td, manifest)
        assert integ["ok"] is True
        assert integ["missing_on_disk"] == []
        assert integ["extra_on_disk"] == []


def test_integrity_detects_missing_archive():
    with tempfile.TemporaryDirectory() as td:
        manifest = {
            "schema_version": "v1379.integrity/v1",
            "records": [{"name": "ghost.md", "sha256": "x" * 64}],
        }
        integ = v1382._check_integrity(td, manifest)
        assert "ghost.md" in integ["missing_on_disk"]
        assert integ["ok"] is False


def test_integrity_detects_extra_archive():
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# x\n")
        manifest = {"schema_version": "v1379.integrity/v1", "records": []}
        integ = v1382._check_integrity(td, manifest)
        assert name in integ["extra_on_disk"]
        assert integ["ok"] is False


# ----------------------------------------------------------------------
# Path safety
# ----------------------------------------------------------------------

def test_validate_safe_path_rejects_parent_traversal():
    with pytest.raises(ValueError):
        v1382._validate_safe_path("../etc/passwd")
    with pytest.raises(ValueError):
        v1382._validate_safe_path("/tmp/../etc/x")


def test_validate_safe_path_accepts_normal_paths():
    v1382._validate_safe_path("V1375_HISTORY")
    v1382._validate_safe_path("/tmp/test")
    v1382._validate_safe_path("C:\\Users\\test")


# ----------------------------------------------------------------------
# Atomic write + history append
# ----------------------------------------------------------------------

def test_atomic_write_creates_nested_dirs():
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "nested", "deeper", "snapshot.json")
        v1382._atomic_write(path, '{"hello": "world"}')
        assert os.path.exists(path)
        with open(path, "r", encoding="utf-8") as fh:
            assert fh.read() == '{"hello": "world"}'


def test_append_history_appends_jsonl_line():
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "history.jsonl")
        snap = {"schema": v1382.SCHEMA_VERSION, "totals": {"archives": 0}}
        v1382._append_history(snap, path=path, tag="t1")
        v1382._append_history(snap, path=path, tag="t2")
        with open(path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        assert len(lines) == 2
        assert json.loads(lines[0])["tag"] == "t1"
        assert json.loads(lines[1])["tag"] == "t2"


def test_append_history_preserves_existing_lines():
    with tempfile.TemporaryDirectory() as td:
        path = os.path.join(td, "history.jsonl")
        with open(path, "w") as fh:
            fh.write('{"preexisting": true}\n')
        v1382._append_history({"x": 1}, path=path)
        with open(path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        assert len(lines) == 2
        assert json.loads(lines[0]).get("preexisting") is True
        assert json.loads(lines[1]).get("snapshot") is not None


# ----------------------------------------------------------------------
# Snapshot builder
# ----------------------------------------------------------------------

def test_snapshot_returns_safe_dict_for_missing_dir():
    snap = v1382.snapshot_archive_health(
        archive_dir="/nonexistent/v1382/test",
        manifest_path="/nonexistent/v1382/manifest.json",
    )
    assert snap["schema"] == v1382.SCHEMA_VERSION
    assert snap["totals"]["archives"] == 0
    assert snap["totals"]["indexed"] == 0
    assert len(snap["guarded_observations"]) >= 5
    assert len(snap["known_unknowns"]) >= 2
    assert "generated" in snap


def test_snapshot_includes_tier_and_action_counts():
    snap = v1382.snapshot_archive_health(
        archive_dir="/nonexistent/v1382/test",
    )
    # tier_counts and action_counts must be present even when empty
    assert set(snap["tier_counts"].keys()) == {"HOT", "WARM", "COLD", "FROZEN"}
    assert set(snap["action_counts"].keys()) == {"keep", "compress", "prune"}


def test_snapshot_includes_integrity_block():
    snap = v1382.snapshot_archive_health(archive_dir="/nonexistent/v1382/test")
    assert "integrity" in snap
    assert "manifest_present" in snap["integrity"]
    assert "ok" in snap["integrity"]


def test_snapshot_includes_rotation_block():
    snap = v1382.snapshot_archive_health(archive_dir="/nonexistent/v1382/test")
    assert "rotation" in snap
    assert "policy_version" in snap["rotation"]
    assert "actions_summary" in snap["rotation"]


def test_snapshot_on_real_dir_counts_archives():
    with tempfile.TemporaryDirectory() as td:
        name = "2026-08-09T03-55-00Z__v1374.md"
        with open(os.path.join(td, name), "w") as fh:
            fh.write("# archive\n")
        idx = os.path.join(td, "INDEX.md")
        with open(idx, "w") as fh:
            fh.write(f"# Index\n\n| name |\n|---|\n| `{name}` |\n")
        snap = v1382.snapshot_archive_health(archive_dir=td)
        assert snap["totals"]["archives"] == 1
        assert snap["totals"]["indexed"] == 1
        assert snap["tier_counts"]["HOT"] >= 1


# ----------------------------------------------------------------------
# JSON rendering
# ----------------------------------------------------------------------

def test_render_snapshot_json_deterministic():
    snap = v1382.snapshot_archive_health(archive_dir="/nonexistent/v1382/test")
    j1 = v1382._render_snapshot_json(snap)
    j2 = v1382._render_snapshot_json(snap)
    assert j1 == j2


def test_render_snapshot_json_pretty_multiline():
    snap = v1382.snapshot_archive_health(archive_dir="/nonexistent/v1382/test")
    pretty = v1382._render_snapshot_json(snap, pretty=True)
    assert "\n" in pretty


def test_one_line_summary_contains_keys():
    snap = v1382.snapshot_archive_health(archive_dir="/nonexistent/v1382/test")
    line = v1382._one_line_summary(snap)
    assert "V1382" in line
    assert "archives=" in line
    assert "integrity=" in line
    assert "actions=[" in line


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def _capture_cli(args):
    buf_out = io.StringIO()
    buf_err = io.StringIO()
    with redirect_stdout(buf_out), redirect_stderr(buf_err):
        rc = v1382.run_cli(args)
    return rc, buf_out.getvalue(), buf_err.getvalue()


def test_cli_version_subcommand():
    rc, out, _ = _capture_cli(["version"])
    assert rc == 0
    assert v1382.SCHEMA_VERSION in out


def test_cli_summary_subcommand():
    rc, out, _ = _capture_cli([
        "--archive-dir", "/nonexistent/v1382/test", "summary",
    ])
    assert rc == 0
    assert "V1382" in out


def test_cli_snapshot_to_stdout():
    rc, out, _ = _capture_cli([
        "--archive-dir", "/nonexistent/v1382/test", "snapshot",
    ])
    assert rc == 0
    parsed = json.loads(out)
    assert parsed["schema"] == v1382.SCHEMA_VERSION


def test_cli_snapshot_pretty():
    rc, out, _ = _capture_cli([
        "--archive-dir", "/nonexistent/v1382/test", "snapshot", "--pretty",
    ])
    assert rc == 0
    assert "\n" in out


def test_cli_snapshot_out_writes_file():
    with tempfile.TemporaryDirectory() as td:
        out_path = os.path.join(td, "snap.json")
        rc, _, _ = _capture_cli([
            "--archive-dir", "/nonexistent/v1382/test",
            "snapshot", "--out", out_path,
        ])
        assert rc == 0
        assert os.path.exists(out_path)
        with open(out_path, "r", encoding="utf-8") as fh:
            parsed = json.loads(fh.read())
        assert parsed["schema"] == v1382.SCHEMA_VERSION


def test_cli_snapshot_record_appends_history():
    with tempfile.TemporaryDirectory() as td:
        history_path = os.path.join(td, "history.jsonl")
        rc, _, _ = _capture_cli([
            "--archive-dir", "/nonexistent/v1382/test",
            "--history-path", history_path,
            "snapshot", "--record", "--tag", "unit_test",
        ])
        assert rc == 0
        assert os.path.exists(history_path)
        with open(history_path, "r", encoding="utf-8") as fh:
            lines = fh.readlines()
        assert len(lines) == 1
        rec = json.loads(lines[0])
        assert rec.get("tag") == "unit_test"
        assert rec.get("snapshot", {}).get("schema") == v1382.SCHEMA_VERSION


def test_cli_snapshot_default_does_not_write_history():
    """Default CLI must NOT touch V1382_HISTORY.jsonl (GUARD_RECORD_IS_OPT_IN)."""
    with tempfile.TemporaryDirectory() as td:
        history_path = os.path.join(td, "history.jsonl")
        rc, _, _ = _capture_cli([
            "--archive-dir", "/nonexistent/v1382/test",
            "--history-path", history_path,
            "snapshot",
        ])
        assert rc == 0
        assert not os.path.exists(history_path), \
            "Default snapshot must not write history (--record is opt-in)"


# ----------------------------------------------------------------------
# Determinism + Popper integration
# ----------------------------------------------------------------------

def test_snapshot_deterministic_with_fixed_now():
    now = _dt.datetime(2026, 8, 9, 5, 0, 0, tzinfo=_dt.timezone.utc)
    snap_a = v1382.snapshot_archive_health(
        archive_dir="/nonexistent/v1382/test", now=now,
    )
    snap_b = v1382.snapshot_archive_health(
        archive_dir="/nonexistent/v1382/test", now=now,
    )
    # Replace `generated` (only timestamp differs) and compare the rest
    assert snap_a["generated"] == snap_b["generated"]
    assert snap_a["totals"] == snap_b["totals"]
    assert snap_a["integrity"] == snap_b["integrity"]


def test_popper_self_tests_all_pass():
    passed, total, failures = v1382._popper_self_tests()
    assert passed == total, f"Popper failures: {failures}"
    assert total >= 50, f"Expected ≥50 Popper self-tests, got {total}"


# ----------------------------------------------------------------------
# Real-data smoke (production data, optional)
# ----------------------------------------------------------------------

def test_real_data_snapshot_smoke():
    """Snapshot the actual V1375_HISTORY if it exists."""
    if not os.path.isdir("V1375_HISTORY"):
        pytest.skip("V1375_HISTORY dir not present (production smoke test)")
    rc, out, _ = _capture_cli(["snapshot", "--out", "V1382_SNAPSHOT_AUTO.json"])
    assert rc == 0
    assert os.path.exists("V1382_SNAPSHOT_AUTO.json")
    # Clean up
    try:
        os.unlink("V1382_SNAPSHOT_AUTO.json")
    except OSError:
        pass