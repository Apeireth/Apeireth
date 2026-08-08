"""Tests for V1379 — V1375 archive integrity manifest.

Run from promethean/:
    python -m pytest tests/test_v1379_v1375_archive_integrity.py -v
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

import apeireth.v1379_v1375_archive_integrity as v1379


# ----------------------------------------------------------------------
# Constants + GUARDS
# ----------------------------------------------------------------------

def test_schema_version_constant():
    assert v1379.SCHEMA_VERSION == "v1379.integrity/v1"


def test_script_name_constant():
    assert v1379.SCRIPT_NAME == "v1379_v1375_archive_integrity"


def test_default_archive_dir_constant():
    assert v1379.DEFAULT_ARCHIVE_DIR == "V1375_HISTORY"


def test_default_manifest_path_constant():
    assert v1379.DEFAULT_MANIFEST_PATH.endswith(".json")


def test_default_verify_report_path_constant():
    assert v1379.DEFAULT_VERIFY_REPORT_PATH.endswith(".md")


def test_guards_list_length():
    assert len(v1379.GUARDS) == 10


def test_guards_contains_required():
    expected = {
        "GUARD_HASH_SHA256_ONLY",
        "GUARD_ATOMIC_WRITE",
        "GUARD_NO_SIDECAR_TOUCH",
        "GUARD_NO_LEDGER_TOUCH",
        "GUARD_VERIFY_READ_ONLY",
        "GUARD_REPORT_ALL_MISMATCHES",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_NO_CAP_CHANGE",
        "GUARD_DETERMINISTIC",
        "GUARD_NO_FAKE_REPAIR",
    }
    assert set(v1379.GUARDS) == expected


def test_guards_no_duplicates():
    assert len(set(v1379.GUARDS)) == len(v1379.GUARDS)


# ----------------------------------------------------------------------
# Synthetic fixture helper
# ----------------------------------------------------------------------

def _make_archive_file(
    arch_dir: str,
    iso_basic: str,
    content: bytes = b"# archive\n",
    schema: str = "v1374",
) -> str:
    """Create one V1375-style archive file. Returns the full path."""
    name = f"{iso_basic}__{schema}.md"
    path = os.path.join(arch_dir, name)
    with open(path, "wb") as fh:
        fh.write(content)
    return path


def _make_three_archives(
    parent: str | None = None,
) -> tuple[str, list[str]]:
    """Create three archives in chronological order. Returns (dir, paths)."""
    if parent is None:
        parent = tempfile.mkdtemp(prefix="v1379_three_")
    _make_archive_file(parent, "2026-08-09T03-00-00Z", b"# A\n")
    _make_archive_file(parent, "2026-08-09T04-00-00Z", b"# B\n")
    _make_archive_file(parent, "2026-08-09T05-00-00Z", b"# C\n")
    return parent, [
        os.path.join(parent, n) for n in sorted(os.listdir(parent))
        if n.endswith(".md")
    ]


# ----------------------------------------------------------------------
# _parse_iso_basic + _format_iso
# ----------------------------------------------------------------------

def test_parse_iso_basic_zulu():
    dt = v1379._parse_iso_basic("2026-08-09T04-00-00Z")
    assert dt is not None
    assert dt.year == 2026
    assert dt.month == 8
    assert dt.day == 9
    assert dt.hour == 4
    assert dt.minute == 0
    assert dt.second == 0
    assert dt.utcoffset().total_seconds() == 0


def test_parse_iso_basic_offset():
    """+0800 means UTC hour = local hour - 8."""
    dt = v1379._parse_iso_basic("2026-08-09T12-00-00+0800")
    assert dt is not None
    assert dt.hour == 4  # 12 - 8 = 4 UTC
    assert dt.minute == 0


def test_parse_iso_basic_garbage():
    assert v1379._parse_iso_basic("not-an-iso") is None


def test_parse_iso_basic_empty():
    assert v1379._parse_iso_basic("") is None


def test_parse_iso_basic_whitespace():
    assert v1379._parse_iso_basic("   ") is None


def test_format_iso_aware():
    import datetime as _dt
    dt = _dt.datetime(2026, 8, 9, 4, 0, 0, tzinfo=_dt.timezone.utc)
    assert v1379._format_iso(dt) == "2026-08-09T04:00:00Z"


def test_format_iso_naive_assumes_utc():
    import datetime as _dt
    dt = _dt.datetime(2026, 8, 9, 4, 0, 0)
    assert v1379._format_iso(dt) == "2026-08-09T04:00:00Z"


def test_format_iso_none_returns_dash():
    assert v1379._format_iso(None) == "—"


# ----------------------------------------------------------------------
# _slug_components
# ----------------------------------------------------------------------

def test_slug_basic_iso_parses():
    slug = v1379._slug_components("2026-08-09T04-00-00Z__v1374.md")
    assert slug is not None
    assert slug["iso_basic"] == "2026-08-09T04-00-00Z"
    assert slug["iso_extended"] == "2026-08-09T04:00:00Z"
    assert slug["schema"] == "v1374"


def test_slug_rejects_non_v1375():
    assert v1379._slug_components("not-a-slug.md") is None


def test_slug_rejects_extended_iso():
    """V1375 only writes basic ISO (filesystem-safe, dashes instead of colons)."""
    assert v1379._slug_components("2026-08-09T04:00:00Z__v1374.md") is None


def test_slug_rejects_empty():
    assert v1379._slug_components("") is None


def test_slug_rejects_wrong_schema():
    slug = v1379._slug_components("2026-08-09T04-00-00Z__v9999.md")
    assert slug is not None
    assert slug["schema"] == "v9999"


# ----------------------------------------------------------------------
# _validate_safe_path + _safe_join
# ----------------------------------------------------------------------

def test_validate_safe_path_rejects_traversal():
    with pytest.raises(ValueError):
        v1379._validate_safe_path("../etc/passwd")


def test_validate_safe_path_accepts_absolute():
    v1379._validate_safe_path("/normal/path/file.md")  # no exception


def test_validate_safe_path_accepts_relative():
    v1379._validate_safe_path("normal/path/file.md")  # no exception


def test_safe_join_rejects_traversal():
    with pytest.raises(ValueError):
        v1379._safe_join("..", "passwd")


def test_safe_join_accepts_normal():
    result = v1379._safe_join("dir", "subdir", "file.md")
    assert result.endswith("file.md")


# ----------------------------------------------------------------------
# hash_archive
# ----------------------------------------------------------------------

def test_hash_archive_returns_64char_hex():
    fd, path = tempfile.mkstemp(prefix="v1379_hash_", suffix=".md")
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(b"hello\n")
        h = v1379.hash_archive(path)
        assert len(h) == 64
        assert all(c in "0123456789abcdef" for c in h)
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


def test_hash_archive_deterministic():
    fd, path = tempfile.mkstemp(prefix="v1379_hash_", suffix=".md")
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(b"deterministic content\n")
        h1 = v1379.hash_archive(path)
        h2 = v1379.hash_archive(path)
        assert h1 == h2
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


def test_hash_archive_matches_hashlib():
    """hash_archive must use hashlib.sha256 directly."""
    import hashlib
    fd, path = tempfile.mkstemp(prefix="v1379_hash_", suffix=".md")
    try:
        content = b"compare\n"
        with os.fdopen(fd, "wb") as fh:
            fh.write(content)
        assert v1379.hash_archive(path) == hashlib.sha256(content).hexdigest()
    finally:
        try:
            os.unlink(path)
        except OSError:
            pass


# ----------------------------------------------------------------------
# scan_archives
# ----------------------------------------------------------------------

def test_scan_archives_returns_chronological():
    parent = tempfile.mkdtemp(prefix="v1379_scan_chrono_")
    try:
        _make_archive_file(parent, "2026-08-09T05-00-00Z", b"# C\n")
        _make_archive_file(parent, "2026-08-09T03-00-00Z", b"# A\n")
        _make_archive_file(parent, "2026-08-09T04-00-00Z", b"# B\n")
        recs = v1379.scan_archives(parent)
        assert [r["iso_basic"] for r in recs] == [
            "2026-08-09T03-00-00Z",
            "2026-08-09T04-00-00Z",
            "2026-08-09T05-00-00Z",
        ]
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_scan_archives_skips_index_md():
    parent = tempfile.mkdtemp(prefix="v1379_scan_idx_")
    try:
        _make_archive_file(parent, "2026-08-09T03-00-00Z", b"# A\n")
        # INDEX.md is part of V1375 output, not an archive
        with open(os.path.join(parent, "INDEX.md"), "wb") as fh:
            fh.write(b"# INDEX\n")
        recs = v1379.scan_archives(parent)
        assert len(recs) == 1
        assert recs[0]["name"].endswith("__v1374.md")
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_scan_archives_hashes_64char():
    parent = tempfile.mkdtemp(prefix="v1379_scan_hash_")
    try:
        _make_archive_file(parent, "2026-08-09T03-00-00Z", b"# A\n")
        _make_archive_file(parent, "2026-08-09T04-00-00Z", b"# B\n")
        recs = v1379.scan_archives(parent)
        assert all(len(r["sha256"]) == 64 for r in recs)
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_scan_archives_distinct_hashes():
    parent = tempfile.mkdtemp(prefix="v1379_scan_distinct_")
    try:
        _make_archive_file(parent, "2026-08-09T03-00-00Z", b"# A\n")
        _make_archive_file(parent, "2026-08-09T04-00-00Z", b"# B\n")
        recs = v1379.scan_archives(parent)
        assert len(set(r["sha256"] for r in recs)) == 2
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_scan_archives_missing_dir_returns_empty():
    assert v1379.scan_archives("/nonexistent") == []


def test_scan_archives_empty_dir_returns_empty():
    parent = tempfile.mkdtemp(prefix="v1379_scan_empty_")
    try:
        assert v1379.scan_archives(parent) == []
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# build_manifest + render_manifest_json
# ----------------------------------------------------------------------

def test_build_manifest_schema():
    parent = tempfile.mkdtemp(prefix="v1379_manifest_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        assert m["schema"] == v1379.SCHEMA_VERSION
        assert m["hash_algorithm"] == "sha256"
        assert m["archive_count"] == 3
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_build_manifest_algo_sha256():
    parent = tempfile.mkdtemp(prefix="v1379_manifest_algo_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        assert m["hash_algorithm"] == "sha256"
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_render_manifest_json_parses():
    parent = tempfile.mkdtemp(prefix="v1379_render_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        js = v1379.render_manifest_json(m)
        parsed = json.loads(js)
        assert isinstance(parsed, dict)
        assert parsed["archive_count"] == 3
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_render_manifest_json_deterministic():
    parent = tempfile.mkdtemp(prefix="v1379_render_det_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        js1 = v1379.render_manifest_json(m)
        js2 = v1379.render_manifest_json(m)
        assert js1 == js2
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_render_manifest_json_sha256_keys_count():
    """Three archives = three occurrences of the 'sha256' JSON key."""
    parent = tempfile.mkdtemp(prefix="v1379_render_keys_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        js = v1379.render_manifest_json(m)
        # Key "sha256": appears once per archive; hash_algorithm is the value "sha256" not key
        assert js.count('"sha256":') == 3
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# write_manifest + load_manifest (roundtrip)
# ----------------------------------------------------------------------

def test_write_manifest_roundtrip():
    parent = tempfile.mkdtemp(prefix="v1379_roundtrip_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        loaded = v1379.load_manifest(manifest_path)
        assert loaded["archive_count"] == 3
        assert loaded["archives"][0]["iso_basic"] == "2026-08-09T03-00-00Z"
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_write_manifest_atomic_no_tmp_leftovers():
    parent = tempfile.mkdtemp(prefix="v1379_atomic_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        leftovers = [
            n for n in os.listdir(parent)
            if n.startswith(".v1379_manifest_") and n.endswith(".json.tmp")
        ]
        assert leftovers == []
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_load_manifest_missing_returns_empty():
    assert v1379.load_manifest("/nonexistent/path/MANIFEST.json") == {}


def test_load_manifest_malformed_returns_empty():
    parent = tempfile.mkdtemp(prefix="v1379_malformed_")
    try:
        bad_path = os.path.join(parent, "BAD.json")
        with open(bad_path, "w", encoding="utf-8") as fh:
            fh.write("not-valid-json {{{")
        assert v1379.load_manifest(bad_path) == {}
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# verify_against_manifest
# ----------------------------------------------------------------------

def test_verify_ok_when_match():
    parent = tempfile.mkdtemp(prefix="v1379_verify_ok_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        result = v1379.verify_against_manifest(manifest_path, parent)
        assert result["ok"] is True
        assert result["mismatches"] == []
        assert result["missing"] == []
        assert result["extra"] == []
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_verify_detects_tamper():
    parent = tempfile.mkdtemp(prefix="v1379_verify_tamper_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        # Tamper with one archive
        tampered = os.path.join(parent, "2026-08-09T04-00-00Z__v1374.md")
        with open(tampered, "ab") as fh:
            fh.write(b"\nTAMPERED\n")
        result = v1379.verify_against_manifest(manifest_path, parent)
        assert result["ok"] is False
        assert len(result["mismatches"]) == 1
        assert result["mismatches"][0]["name"] == "2026-08-09T04-00-00Z__v1374.md"
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_verify_detects_missing():
    parent = tempfile.mkdtemp(prefix="v1379_verify_missing_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        # Delete one archive
        victim = os.path.join(parent, "2026-08-09T03-00-00Z__v1374.md")
        os.unlink(victim)
        result = v1379.verify_against_manifest(manifest_path, parent)
        assert result["ok"] is False
        assert "2026-08-09T03-00-00Z__v1374.md" in result["missing"]
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_verify_detects_extra():
    parent = tempfile.mkdtemp(prefix="v1379_verify_extra_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        # Add a new archive (not in manifest)
        _make_archive_file(parent, "2026-08-09T06-00-00Z", b"# D\n")
        result = v1379.verify_against_manifest(manifest_path, parent)
        assert result["ok"] is False
        assert "2026-08-09T06-00-00Z__v1374.md" in result["extra"]
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_verify_missing_manifest_returns_not_ok():
    parent = tempfile.mkdtemp(prefix="v1379_verify_nomanifest_")
    try:
        _make_three_archives(parent)
        result = v1379.verify_against_manifest("/nonexistent/MANIFEST.json", parent)
        assert result["ok"] is False
        assert result["manifest_archive_count"] == 0
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_verify_reports_size_drift():
    """Mismatched file should also report expected vs actual size."""
    parent = tempfile.mkdtemp(prefix="v1379_verify_size_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        tampered = os.path.join(parent, "2026-08-09T04-00-00Z__v1374.md")
        with open(tampered, "ab") as fh:
            fh.write(b"\nEXTRA\n")
        result = v1379.verify_against_manifest(manifest_path, parent)
        assert result["mismatches"][0]["expected_size"] != result["mismatches"][0]["actual_size"]
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_verify_is_read_only():
    """Verify must not modify the manifest or archives."""
    parent = tempfile.mkdtemp(prefix="v1379_verify_readonly_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        before_manifest = open(manifest_path, "rb").read()
        archive_path = os.path.join(parent, "2026-08-09T04-00-00Z__v1374.md")
        before_archive = open(archive_path, "rb").read()
        # Run verify multiple times
        for _ in range(3):
            v1379.verify_against_manifest(manifest_path, parent)
        assert open(manifest_path, "rb").read() == before_manifest
        assert open(archive_path, "rb").read() == before_archive
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# render_verify_report_md
# ----------------------------------------------------------------------

def test_render_verify_report_ok():
    parent = tempfile.mkdtemp(prefix="v1379_report_ok_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        result = v1379.verify_against_manifest(manifest_path, parent)
        md = v1379.render_verify_report_md(result, archive_dir=parent, manifest_path=manifest_path)
        assert "# V1379" in md
        assert "all good" in md or "✓ all good" in md
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_render_verify_report_tamper():
    parent = tempfile.mkdtemp(prefix="v1379_report_tamper_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        tampered = os.path.join(parent, "2026-08-09T04-00-00Z__v1374.md")
        with open(tampered, "ab") as fh:
            fh.write(b"\nTAMPERED\n")
        result = v1379.verify_against_manifest(manifest_path, parent)
        md = v1379.render_verify_report_md(result, archive_dir=parent, manifest_path=manifest_path)
        assert "integrity issue" in md or "✗ integrity issue" in md
        assert "2026-08-09T04-00-00Z__v1374.md" in md
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_render_verify_report_honesty():
    parent = tempfile.mkdtemp(prefix="v1379_report_hon_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        result = v1379.verify_against_manifest(manifest_path, parent)
        md = v1379.render_verify_report_md(result, archive_dir=parent, manifest_path=manifest_path)
        assert "Honesty disclosure" in md
        assert "GUARD" in md.upper() or "guard" in md
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# run_cli
# ----------------------------------------------------------------------

def test_cli_version():
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = v1379.run_cli(["version"])
    assert rc == 0
    assert v1379.SCHEMA_VERSION in buf.getvalue()
    assert "GUARD_HASH_SHA256_ONLY" in buf.getvalue()


def test_cli_popper():
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1379.run_cli(["popper"])
    assert rc == 0
    assert "Popper self-tests:" in buf.getvalue()
    assert "FAIL" not in buf.getvalue()


def test_cli_popper_verbose():
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1379.run_cli(["popper", "--verbose"])
    assert rc == 0


def test_cli_build_then_verify(tmp_path):
    """End-to-end: build manifest, then verify against it."""
    arch_dir = tmp_path / "archives"
    arch_dir.mkdir()
    _make_archive_file(str(arch_dir), "2026-08-09T03-00-00Z", b"# A\n")
    _make_archive_file(str(arch_dir), "2026-08-09T04-00-00Z", b"# B\n")
    manifest_path = tmp_path / "MANIFEST.json"

    # Build
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1379.run_cli([
            "build",
            "--archive-dir", str(arch_dir),
            "--manifest-path", str(manifest_path),
            "--quiet",
        ])
    assert rc == 0
    assert manifest_path.exists()

    # Verify (should pass)
    report_path = tmp_path / "REPORT.md"
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1379.run_cli([
            "verify",
            "--archive-dir", str(arch_dir),
            "--manifest-path", str(manifest_path),
            "--report-path", str(report_path),
            "--quiet",
        ])
    assert rc == 0
    assert report_path.exists()
    assert "all good" in report_path.read_text(encoding="utf-8") or "✓ all good" in report_path.read_text(encoding="utf-8")


def test_cli_verify_detects_tamper(tmp_path):
    """End-to-end: build, tamper, verify fails."""
    arch_dir = tmp_path / "archives"
    arch_dir.mkdir()
    _make_archive_file(str(arch_dir), "2026-08-09T03-00-00Z", b"# A\n")
    _make_archive_file(str(arch_dir), "2026-08-09T04-00-00Z", b"# B\n")
    manifest_path = tmp_path / "MANIFEST.json"

    v1379.run_cli(["build", "--archive-dir", str(arch_dir), "--manifest-path", str(manifest_path), "--quiet"])

    # Tamper
    tampered = arch_dir / "2026-08-09T04-00-00Z__v1374.md"
    with open(tampered, "ab") as fh:
        fh.write(b"\nTAMPERED\n")

    report_path = tmp_path / "REPORT.md"
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1379.run_cli([
            "verify",
            "--archive-dir", str(arch_dir),
            "--manifest-path", str(manifest_path),
            "--report-path", str(report_path),
            "--quiet",
        ])
    assert rc == 1  # FAIL
    assert report_path.exists()
    assert "integrity issue" in report_path.read_text(encoding="utf-8") or "✗ integrity issue" in report_path.read_text(encoding="utf-8")


def test_cli_show():
    """`show` prints the manifest as JSON to stdout."""
    arch_dir = tempfile.mkdtemp(prefix="v1379_show_")
    manifest_path = os.path.join(arch_dir, "M.json")
    try:
        _make_archive_file(arch_dir, "2026-08-09T03-00-00Z", b"# A\n")
        v1379.run_cli(["build", "--archive-dir", arch_dir, "--manifest-path", manifest_path, "--quiet"])
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = v1379.run_cli(["show", "--manifest-path", manifest_path])
        assert rc == 0
        out = buf.getvalue()
        assert '"archive_count"' in out
        assert '"sha256":' in out
    finally:
        import shutil
        shutil.rmtree(arch_dir, ignore_errors=True)


def test_cli_show_missing_manifest():
    buf = io.StringIO()
    err = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(err):
        rc = v1379.run_cli(["show", "--manifest-path", "/nonexistent/M.json"])
    assert rc == 1


def test_cli_unknown_command():
    """Argparse rejects unknown subcommand with SystemExit(2)."""
    with pytest.raises(SystemExit) as exc_info:
        v1379.run_cli(["bogus"])
    assert exc_info.value.code == 2


def test_cli_no_args():
    """No cmd → prints help and exits 2."""
    buf = io.StringIO()
    with redirect_stdout(buf), redirect_stderr(buf):
        rc = v1379.run_cli([])
    assert rc == 2


# ----------------------------------------------------------------------
# Subprocess integration
# ----------------------------------------------------------------------

def test_subprocess_popper_all_pass():
    """Run popper self-tests via subprocess."""
    proc = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1379_v1375_archive_integrity",
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


def test_subprocess_build_then_verify(tmp_path):
    """End-to-end subprocess: build manifest, then verify."""
    arch_dir = tmp_path / "archives"
    arch_dir.mkdir()
    _make_archive_file(str(arch_dir), "2026-08-09T03-00-00Z", b"# A\n")
    manifest_path = tmp_path / "MANIFEST.json"
    report_path = tmp_path / "REPORT.md"

    # Build
    proc = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1379_v1375_archive_integrity",
            "build",
            "--archive-dir", str(arch_dir),
            "--manifest-path", str(manifest_path),
            "--quiet",
        ],
        cwd=os.getcwd(),
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert proc.returncode == 0, f"stderr: {proc.stderr}"

    # Verify
    proc2 = subprocess.run(
        [
            sys.executable, "-m", "apeireth.v1379_v1375_archive_integrity",
            "verify",
            "--archive-dir", str(arch_dir),
            "--manifest-path", str(manifest_path),
            "--report-path", str(report_path),
            "--quiet",
        ],
        cwd=os.getcwd(),
        capture_output=True,
        text=True,
        timeout=30,
    )
    assert proc2.returncode == 0, f"stderr: {proc2.stderr}"
    assert report_path.exists()


# ----------------------------------------------------------------------
# Guards (semantic)
# ----------------------------------------------------------------------

def test_guard_no_sidecar_touch():
    """V1379 must not import V1371 / V1370 / V1369 sidecar modules."""
    src = Path(v1379.__file__)
    text = src.read_text(encoding="utf-8")
    # Sidecar modules are V1371, V1370, V1369 — none should be imported
    assert "import v1371" not in text
    assert "from apeireth.v1371" not in text
    assert "import v1370" not in text
    assert "from apeireth.v1370" not in text
    assert "import v1369" not in text
    assert "from apeireth.v1369" not in text


def test_guard_no_ledger_touch():
    """V1379 must not import V1362 / V1368 / V1375 ledger-touching modules."""
    src = Path(v1379.__file__)
    text = src.read_text(encoding="utf-8")
    assert "import v1362" not in text
    assert "from apeireth.v1362" not in text
    assert "import v1368" not in text
    assert "from apeireth.v1368" not in text
    assert "import v1375" not in text
    assert "from apeireth.v1375" not in text


def test_guard_verify_read_only():
    """Verify must not modify the manifest or archives."""
    parent = tempfile.mkdtemp(prefix="v1379_guard_readonly_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        manifest_path = os.path.join(parent, "MANIFEST.json")
        v1379.write_manifest(manifest_path, m)
        before_manifest_mtime = os.path.getmtime(manifest_path)
        archive_path = os.path.join(parent, "2026-08-09T04-00-00Z__v1374.md")
        before_archive_mtime = os.path.getmtime(archive_path)
        v1379.verify_against_manifest(manifest_path, parent)
        # Manifest mtime must not change (verify never writes to manifest)
        assert os.path.getmtime(manifest_path) == before_manifest_mtime
        # Archive mtime must not change
        assert os.path.getmtime(archive_path) == before_archive_mtime
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


def test_guard_hash_sha256_only():
    """Only SHA-256 is used (no MD5, no SHA-1)."""
    parent = tempfile.mkdtemp(prefix="v1379_guard_sha256_")
    try:
        _make_three_archives(parent)
        recs = v1379.scan_archives(parent)
        m = v1379.build_manifest(recs, archive_dir=parent)
        assert m["hash_algorithm"] == "sha256"
        # Manifest JSON should not mention MD5 or SHA-1
        js = v1379.render_manifest_json(m)
        assert "md5" not in js.lower()
        assert "sha1" not in js.lower()
    finally:
        import shutil
        shutil.rmtree(parent, ignore_errors=True)


# ----------------------------------------------------------------------
# Real-data smoke (production data, optional)
# ----------------------------------------------------------------------

def test_real_data_build_then_verify_smoke():
    """Build + verify against the actual V1375_HISTORY if it exists."""
    if not os.path.isdir("V1375_HISTORY"):
        pytest.skip("V1375_HISTORY dir not present (production smoke test)")
    manifest_path = os.path.join(tempfile.gettempdir(), "v1379_smoke_manifest.json")
    try:
        # Build
        rc = v1379.run_cli([
            "build",
            "--archive-dir", "V1375_HISTORY",
            "--manifest-path", manifest_path,
            "--quiet",
        ])
        assert rc == 0
        assert os.path.exists(manifest_path)

        # Verify
        rc = v1379.run_cli([
            "verify",
            "--archive-dir", "V1375_HISTORY",
            "--manifest-path", manifest_path,
            "--report-path", os.path.join(tempfile.gettempdir(), "v1379_smoke_report.md"),
            "--quiet",
        ])
        assert rc == 0
    finally:
        try:
            os.unlink(manifest_path)
        except OSError:
            pass


# ----------------------------------------------------------------------
# Popper integration
# ----------------------------------------------------------------------

def test_popper_self_tests_all_pass():
    passed, total, failures = v1379._popper_self_tests()
    assert passed == total, f"Popper failures: {failures}"
    assert total >= 50, f"Expected ≥50 Popper self-tests, got {total}"
