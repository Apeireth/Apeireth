"""Tests for V1378 — V1375 × V1362 History Overlay.

Run from promethean/:
    python -m pytest tests/test_v1378_v1375_x_v1362_history_overlay.py -v
"""

from __future__ import annotations

import io
import json
import os
import re
import subprocess
import sys
import tempfile
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

import pytest

import apeireth.v1378_v1375_x_v1362_history_overlay as v1378


# ----------------------------------------------------------------------
# Synthetic fixtures
# ----------------------------------------------------------------------

def _make_ledger_entry(
    measured_at: str = "2026-08-09T04:00:00Z",
    *,
    tag: str | None = "test-tag",
    pole_star_total: float | None = 0.9,
    pole_star_cap: float | None = 0.9,
    pole_star_delta_vs_v01: float | None = 0.1,
    toolchain_present: int | None = 11,
    toolchain_total: int | None = 11,
    close_loop_pass: int | None = 7,
    close_loop_total: int | None = 7,
    v_modules: int | None = 1388,
    test_files: int | None = 426,
) -> dict:
    return {
        "measured_at": measured_at,
        "tag": tag,
        "pole_star_total": pole_star_total,
        "pole_star_cap": pole_star_cap,
        "pole_star_delta_vs_v01": pole_star_delta_vs_v01,
        "toolchain_present": toolchain_present,
        "toolchain_total": toolchain_total,
        "close_loop_pass": close_loop_pass,
        "close_loop_total": close_loop_total,
        "v_modules": v_modules,
        "test_files": test_files,
    }


def _make_archive(
    iso: str = "2026-08-09T04:00:00Z",
    *,
    schema: str = "v1374",
    filename: str | None = None,
    size: int = 2310,
) -> dict:
    if filename is None:
        # Convert ISO basic-style for filename; keep iso as filesystem-safe
        filename = iso.replace(":", "-") + "__" + schema + ".md"
    return {
        "iso": iso,
        "schema": schema,
        "filename": filename,
        "size": size,
        "path": filename,
    }


def _write_ledger(entries: list[dict]) -> str:
    fd, path = tempfile.mkstemp(suffix=".jsonl", prefix="v1378_test_ledger_")
    with os.fdopen(fd, "w", encoding="utf-8") as fh:
        for e in entries:
            fh.write(json.dumps(e) + "\n")
    return path


# ----------------------------------------------------------------------
# Constants + GUARDS
# ----------------------------------------------------------------------

def test_schema_version_constant():
    assert v1378.SCHEMA_VERSION == "v1378.overlay/v1"


def test_script_name_constant():
    assert v1378.SCRIPT_NAME == "v1378_v1375_x_v1362_history_overlay"


def test_default_archive_dir_constant():
    assert v1378.DEFAULT_ARCHIVE_DIR == "V1375_HISTORY"


def test_default_ledger_path_constant():
    assert v1378.DEFAULT_LEDGER_PATH == "pole_star_history.jsonl"


def test_guards_list_length():
    assert len(v1378.GUARDS) == 10


def test_guards_contains_required():
    expected = {
        "GUARD_INPUT_V1375_FAMILY",
        "GUARD_CHRONOLOGICAL_SORT",
        "GUARD_DETERMINISTIC",
        "GUARD_ATOMIC_WRITE",
        "GUARD_NO_LEDGER_WRITE",
        "GUARD_NO_SIDECAR_TOUCH",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_MARKDOWN_ONLY",
        "GUARD_NO_CAP_CHANGE",
        "GUARD_NO_LEDGER_MUTATION",
    }
    assert set(v1378.GUARDS) == expected


# ----------------------------------------------------------------------
# parse_iso_dt
# ----------------------------------------------------------------------

def test_parse_iso_dt_none():
    assert v1378.parse_iso_dt(None) is None


def test_parse_iso_dt_empty():
    assert v1378.parse_iso_dt("") is None
    assert v1378.parse_iso_dt("   ") is None


def test_parse_iso_dt_garbage():
    assert v1378.parse_iso_dt("not-a-date") is None


def test_parse_iso_dt_zulu():
    dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    assert dt is not None
    assert dt.tzinfo is not None


def test_parse_iso_dt_offset():
    dt = v1378.parse_iso_dt("2026-08-09T04:00:00+00:00")
    assert dt is not None
    assert dt.tzinfo is not None


def test_parse_iso_dt_naive_assumes_utc():
    dt = v1378.parse_iso_dt("2026-08-09T04:00:00")
    assert dt is not None
    assert dt.tzinfo is not None  # treated as UTC


def test_parse_iso_dt_iso_basic_v1375_format():
    """V1375 archive filenames use ISO basic format: 2026-08-08T20-06-51Z."""
    dt = v1378.parse_iso_dt("2026-08-08T20-06-51Z")
    assert dt is not None
    assert dt.year == 2026
    assert dt.month == 8
    assert dt.day == 8
    assert dt.hour == 20
    assert dt.minute == 6
    assert dt.second == 51
    assert dt.tzinfo is not None


def test_parse_iso_dt_whitespace_tolerated():
    dt = v1378.parse_iso_dt("  2026-08-09T04:00:00Z  ")
    assert dt is not None


# ----------------------------------------------------------------------
# Format helpers
# ----------------------------------------------------------------------

def test_format_gap_none():
    assert v1378._format_gap(None) == "—"


def test_format_gap_seconds():
    assert v1378._format_gap(45) == "45s"


def test_format_gap_minutes():
    assert v1378._format_gap(120) == "2m"


def test_format_gap_hours():
    assert v1378._format_gap(7200) == "2.0h"


def test_format_value_none():
    assert v1378._format_value(None) == "—"


def test_format_value_float():
    assert v1378._format_value(0.9) == "0.9000"


def test_format_value_int():
    assert v1378._format_value(42) == "42"


# ----------------------------------------------------------------------
# read_ledger_jsonl
# ----------------------------------------------------------------------

def test_read_ledger_missing_file():
    """Robust to nonexistent file."""
    assert v1378.read_ledger_jsonl("/nonexistent/path/ledger.jsonl") == []


def test_read_ledger_empty_entries():
    entries: list[dict] = []
    path = _write_ledger(entries)
    try:
        assert v1378.read_ledger_jsonl(path) == []
    finally:
        os.unlink(path)


def test_read_ledger_two_entries():
    entries = [
        _make_ledger_entry(measured_at="2026-08-09T04:00:00Z", tag="a"),
        _make_ledger_entry(measured_at="2026-08-09T05:00:00Z", tag="b"),
    ]
    path = _write_ledger(entries)
    try:
        result = v1378.read_ledger_jsonl(path)
        assert len(result) == 2
        assert result[0]["tag"] == "a"
        assert result[1]["tag"] == "b"
    finally:
        os.unlink(path)


def test_read_ledger_handles_blank_lines():
    entries = [
        _make_ledger_entry(measured_at="2026-08-09T04:00:00Z", tag="a"),
    ]
    fd, path = tempfile.mkstemp(suffix=".jsonl", prefix="v1378_test_")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write("\n")
            fh.write(json.dumps(entries[0]) + "\n")
            fh.write("\n")
            fh.write("\n")
        result = v1378.read_ledger_jsonl(path)
        assert len(result) == 1
    finally:
        os.unlink(path)


def test_read_ledger_handles_malformed_lines():
    fd, path = tempfile.mkstemp(suffix=".jsonl", prefix="v1378_test_")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write("garbage line\n")
            fh.write(json.dumps({"measured_at": "2026-08-09T04:00:00Z", "tag": "good"}) + "\n")
            fh.write("not json at all\n")
        result = v1378.read_ledger_jsonl(path)
        assert len(result) == 1
        assert result[0]["tag"] == "good"
    finally:
        os.unlink(path)


def test_read_ledger_robust_to_windows_root_path():
    """Windows quirk: /nonexistent resolves to current drive root, exists but unreadable."""
    # On Windows, /nonexistent resolves to C:\ which exists
    # On Linux, /nonexistent doesn't exist
    # read_ledger_jsonl should handle both
    result = v1378.read_ledger_jsonl("/nonexistent")
    assert result == []


# ----------------------------------------------------------------------
# find_nearest_ledger
# ----------------------------------------------------------------------

def test_find_nearest_ledger_basic():
    ledger = [
        _make_ledger_entry(measured_at="2026-08-09T03:00:00Z", tag="before"),
        _make_ledger_entry(measured_at="2026-08-09T05:00:00Z", tag="after"),
    ]
    archive_dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    entry, gap = v1378.find_nearest_ledger(archive_dt, ledger)
    assert entry is not None
    assert entry["tag"] in ("before", "after")
    assert gap is not None
    assert isinstance(gap, float)


def test_find_nearest_ledger_picks_closer():
    ledger = [
        _make_ledger_entry(measured_at="2026-08-09T03:00:00Z", tag="far-before"),
        _make_ledger_entry(measured_at="2026-08-09T03:55:00Z", tag="near-before"),
    ]
    archive_dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    entry, gap = v1378.find_nearest_ledger(archive_dt, ledger)
    assert entry["tag"] == "near-before"


def test_find_nearest_ledger_none_dt():
    ledger = [_make_ledger_entry()]
    assert v1378.find_nearest_ledger(None, ledger) == (None, None)


def test_find_nearest_ledger_empty():
    archive_dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    assert v1378.find_nearest_ledger(archive_dt, []) == (None, None)


def test_find_nearest_ledger_tie_break_first():
    """When two entries are equidistant, first occurrence wins."""
    ledger = [
        _make_ledger_entry(measured_at="2026-08-09T03:00:00Z", tag="first"),
        _make_ledger_entry(measured_at="2026-08-09T05:00:00Z", tag="second"),
    ]
    archive_dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    entry, _ = v1378.find_nearest_ledger(archive_dt, ledger)
    assert entry["tag"] == "first"


def test_find_nearest_ledger_skips_entries_without_dt():
    """Entries with no valid measured_at are skipped, not crash."""
    ledger = [
        {"tag": "no-dt"},
        _make_ledger_entry(measured_at="2026-08-09T04:00:00Z", tag="with-dt"),
    ]
    archive_dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    entry, _ = v1378.find_nearest_ledger(archive_dt, ledger)
    assert entry["tag"] == "with-dt"


def test_find_nearest_ledger_gap_signed():
    """Positive gap means ledger is after archive; negative means before."""
    ledger_after = [_make_ledger_entry(measured_at="2026-08-09T05:00:00Z", tag="after")]
    archive_dt = v1378.parse_iso_dt("2026-08-09T04:00:00Z")
    _, gap = v1378.find_nearest_ledger(archive_dt, ledger_after)
    assert gap > 0

    ledger_before = [_make_ledger_entry(measured_at="2026-08-09T03:00:00Z", tag="before")]
    _, gap = v1378.find_nearest_ledger(archive_dt, ledger_before)
    assert gap < 0


# ----------------------------------------------------------------------
# overlay_row
# ----------------------------------------------------------------------

def test_overlay_row_with_no_entry():
    arc = _make_archive()
    row = v1378.overlay_row(arc, None, None)
    assert row["pole_star_total"] is None
    assert row["ledger_tag"] is None
    assert row["time_gap_s"] is None


def test_overlay_row_with_entry():
    arc = _make_archive()
    entry = _make_ledger_entry(measured_at="2026-08-09T04:00:00Z", tag="x")
    row = v1378.overlay_row(arc, entry, 0.0)
    assert row["ledger_tag"] == "x"
    assert row["pole_star_total"] == 0.9


def test_overlay_row_with_none_tag():
    arc = _make_archive()
    entry = _make_ledger_entry(measured_at="2026-08-09T04:00:00Z", tag=None)
    row = v1378.overlay_row(arc, entry, 0.0)
    assert row["ledger_tag"] is None


def test_overlay_row_includes_archive_metadata():
    arc = _make_archive(iso="2026-08-09T04:00:00Z", size=1234)
    row = v1378.overlay_row(arc, None, None)
    assert row["archive_iso"] == "2026-08-09T04:00:00Z"
    assert row["archive_size"] == 1234


# ----------------------------------------------------------------------
# build_overlay
# ----------------------------------------------------------------------

def test_build_overlay_length():
    archives = [_make_archive() for _ in range(3)]
    ledger = [_make_ledger_entry()]
    rows = v1378.build_overlay(archives, ledger)
    assert len(rows) == 3


def test_build_overlay_preserves_input_order():
    """V1378 does NOT re-sort; caller pre-sorts."""
    archives = [
        _make_archive(iso="2026-08-09T04:00:00Z", filename="second.md"),
        _make_archive(iso="2026-08-09T03:00:00Z", filename="first.md"),
    ]
    ledger = [_make_ledger_entry(measured_at="2026-08-09T03:00:00Z")]
    rows = v1378.build_overlay(archives, ledger)
    assert rows[0]["archive_filename"] == "second.md"
    assert rows[1]["archive_filename"] == "first.md"


def test_build_overlay_handles_garbage_iso():
    """Archives with malformed iso get no ledger match."""
    archives = [_make_archive(iso="garbage")]
    ledger = [_make_ledger_entry()]
    rows = v1378.build_overlay(archives, ledger)
    assert rows[0]["ledger_iso"] is None


# ----------------------------------------------------------------------
# summarize_overlay
# ----------------------------------------------------------------------

def test_summarize_overlay_basic():
    archives = [_make_archive()]
    entries = [_make_ledger_entry()]
    rows = v1378.build_overlay(archives, entries)
    s = v1378.summarize_overlay(rows, archives, entries)
    assert s["n_archives"] == 1
    assert s["n_ledger"] == 1
    assert s["n_with_ledger"] == 1
    assert s["n_with_pole_star"] == 1


def test_summarize_overlay_empty():
    s = v1378.summarize_overlay([], [], [])
    assert s["n_archives"] == 0
    assert s["n_ledger"] == 0
    assert s["gap_min_s"] is None


def test_summarize_overlay_range_computation():
    archives = [_make_archive()]
    row = {
        "archive_iso": "2026-08-09T04:00:00Z",
        "archive_filename": "a.md",
        "archive_size": 1,
        "ledger_iso": "2026-08-09T04:00:00Z",
        "ledger_tag": "a",
        "time_gap_s": 0.0,
        "pole_star_total": 0.9,
        "pole_star_cap": 0.9,
        "pole_star_delta_vs_v01": 0.1,
        "toolchain_present": 11,
        "toolchain_total": 11,
        "close_loop_pass": 7,
        "close_loop_total": 7,
        "v_modules": 100,
        "test_files": 30,
    }
    s = v1378.summarize_overlay([row], archives, [_make_ledger_entry()])
    assert s["v_modules_range"] == (100, 100)
    assert s["gap_min_s"] == 0.0


# ----------------------------------------------------------------------
# render_overlay_md
# ----------------------------------------------------------------------

def test_render_contains_title():
    md = v1378.render_overlay_md(
        [], v1378.summarize_overlay([], [], []), [], []
    )
    assert "# V1378" in md
    assert v1378.SCHEMA_VERSION in md


def test_render_contains_honesty_disclosure():
    md = v1378.render_overlay_md(
        [], v1378.summarize_overlay([], [], []), [], []
    )
    assert "Honesty disclosure" in md or "Honest baseline" in md


def test_render_is_deterministic():
    """Same inputs in same order → same output bytes."""
    archives = [_make_archive()]
    entries = [_make_ledger_entry()]
    rows = v1378.build_overlay(archives, entries)
    s = v1378.summarize_overlay(rows, archives, entries)
    md1 = v1378.render_overlay_md(rows, s, archives, entries)
    md2 = v1378.render_overlay_md(rows, s, archives, entries)
    assert md1 == md2


def test_render_does_not_compose_metric():
    """V1378 must never compute/emit its own pole_star/cap."""
    md = v1378.render_overlay_md(
        [], v1378.summarize_overlay([], [], []), [], []
    )
    # No "v1378_pole_star" or "v1378_cap" key invention
    assert "v1378_pole_star" not in md
    assert "v1378_cap" not in md


# ----------------------------------------------------------------------
# write_overlay_md (atomic)
# ----------------------------------------------------------------------

def test_write_overlay_md_roundtrip():
    fd, path = tempfile.mkstemp(suffix=".md", prefix="v1378_test_")
    os.close(fd)
    try:
        content = "# test\n\n- hello: world\n"
        v1378.write_overlay_md(path, content)
        with open(path, "r", encoding="utf-8") as fh:
            assert fh.read() == content
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


def test_write_overlay_md_rejects_traversal():
    with pytest.raises(ValueError):
        v1378.write_overlay_md("../escape.md", "x")


def test_write_overlay_md_no_temp_leftovers():
    """Atomic write must not leave .tmp files behind."""
    parent = tempfile.mkdtemp(prefix="v1378_atomic_test_")
    try:
        path = os.path.join(parent, "out.md")
        v1378.write_overlay_md(path, "x")
        leftovers = [
            n for n in os.listdir(parent)
            if n.startswith(".v1378_") and n.endswith(".tmp")
        ]
        assert leftovers == []
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# run_overlay (top-level)
# ----------------------------------------------------------------------

def test_run_overlay_with_nonexistent_paths():
    """Both archive-dir and ledger path can be missing — should still succeed."""
    parent = tempfile.mkdtemp(prefix="v1378_run_test_")
    try:
        out_path = os.path.join(parent, "overlay.md")
        result = v1378.run_overlay(
            archive_dir=os.path.join(parent, "missing_dir"),
            ledger_path=os.path.join(parent, "missing_ledger.jsonl"),
            output_path=out_path,
        )
        assert result["summary"]["n_archives"] == 0
        assert result["summary"]["n_ledger"] == 0
        assert os.path.exists(out_path)
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_run_overlay_with_real_data():
    """Smoke test against actual V1375_HISTORY + pole_star_history.jsonl."""
    if not os.path.isdir("V1375_HISTORY"):
        pytest.skip("V1375_HISTORY dir not present (production smoke test)")
    parent = tempfile.mkdtemp(prefix="v1378_smoke_")
    try:
        out_path = os.path.join(parent, "overlay.md")
        result = v1378.run_overlay(
            archive_dir="V1375_HISTORY",
            ledger_path="pole_star_history.jsonl",
            output_path=out_path,
        )
        s = result["summary"]
        assert s["n_archives"] >= 1
        assert s["n_ledger"] >= 1
        assert s["n_with_ledger"] >= 0  # could be 0 if archive timestamp outside ledger range
        assert os.path.exists(out_path)
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def test_cli_version():
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = v1378.run_cli(["version"])
    assert rc == 0
    assert v1378.SCHEMA_VERSION in buf.getvalue()


def test_cli_overlay_nonexistent_paths():
    parent = tempfile.mkdtemp(prefix="v1378_cli_overlay_")
    try:
        out_path = os.path.join(parent, "overlay.md")
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1378.run_cli([
                "overlay",
                "--archive-dir", "/nonexistent",
                "--ledger", "/nonexistent",
                "--output", out_path,
            ])
        assert rc == 0
        assert os.path.exists(out_path)
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_cli_summary_json():
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1378.run_cli([
            "summary",
            "--archive-dir", "/nonexistent",
            "--ledger", "/nonexistent",
        ])
    assert rc == 0
    assert '"n_archives"' in buf.getvalue()


def test_cli_unknown_command():
    """Argparse rejects unknown subcommand with SystemExit(2)."""
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        with pytest.raises(SystemExit) as exc_info:
            v1378.run_cli(["bogus"])
    assert exc_info.value.code == 2


def test_cli_no_args_prints_version():
    """No cmd → version."""
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = v1378.run_cli([])
    assert rc == 0
    assert v1378.SCHEMA_VERSION in buf.getvalue()


# ----------------------------------------------------------------------
# Subprocess integration
# ----------------------------------------------------------------------

def test_subprocess_overlay_end_to_end(tmp_path):
    """Run overlay via subprocess and verify output file exists + has content."""
    out = tmp_path / "overlay.md"
    proc = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1378_v1375_x_v1362_history_overlay",
            "overlay",
            "--archive-dir", "/nonexistent",
            "--ledger", "/nonexistent",
            "--output", str(out),
        ],
        cwd=os.getcwd(),
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert proc.returncode == 0, f"stderr: {proc.stderr}"
    assert out.exists()
    content = out.read_text(encoding="utf-8")
    assert "# V1378" in content
    assert "Honesty disclosure" in content or "Honest baseline" in content


def test_subprocess_popper():
    """Run popper self-tests via subprocess; all must pass."""
    proc = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1378_v1375_x_v1362_history_overlay",
            "popper",
        ],
        cwd=os.getcwd(),
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert proc.returncode == 0
    assert "Popper self-tests:" in proc.stdout
    assert "FAIL" not in proc.stdout


# ----------------------------------------------------------------------
# Guards (semantic checks)
# ----------------------------------------------------------------------

def test_guard_no_ledger_write_real_run():
    """Running overlay against real ledger must not mutate it."""
    if not os.path.exists("pole_star_history.jsonl"):
        pytest.skip("real ledger not present")
    with open("pole_star_history.jsonl", "rb") as fh:
        before = fh.read()
    parent = tempfile.mkdtemp(prefix="v1378_no_ledger_write_")
    try:
        v1378.run_overlay(output_path=os.path.join(parent, "overlay.md"))
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)
    with open("pole_star_history.jsonl", "rb") as fh:
        after = fh.read()
    assert before == after


def test_guard_no_sidecar_touch():
    """V1378 must not import V1371."""
    src = Path(v1378.__file__).parent / "v1378_v1375_x_v1362_history_overlay.py"
    text = src.read_text(encoding="utf-8")
    # V1371 (calibrated cron hook) is referenced only in honesty docs, never imported
    assert "import v1371" not in text
    assert "from apeireth.v1371" not in text
    # V1370 cal reference is only in narrative prose ("see V1370 calibration")
    # but never imported as a module
    assert "import v1370_v1368" not in text
    assert "from apeireth.v1370" not in text


# ----------------------------------------------------------------------
# Popper self-tests integration
# ----------------------------------------------------------------------

def test_popper_self_tests_all_pass():
    passed, total, failures = v1378._popper_self_tests(verbose=False)
    assert passed == total, f"Popper failures: {failures}"
    assert total == 53, f"Expected 53 Popper self-tests, got {total}"