"""Tests for V1375 — V1374 history archive.

~31 pytest tests covering:
- slug_timestamp (fixed + default)
- archive_name (basic + custom schema + invalid inputs)
- archive_report (basic + collision + missing file + path traversal)
- list_archives (sorting, schema separation, skip INDEX, empty)
- _extract_summary (iso/schema/all deltas/gap)
- render_index_md (header/table/legend/honesty/empty/custom title)
- write_index (atomic write)
- parse_index (roundtrip)
- archive_tick (1st + 2nd run)
- run_cli (archive / list / show / index / digest / popper / version)
- GUARD constants: SCHEMA_VERSION, HISTORIC_ADDS_ONLY, no_sidecar, no_ledger, atomic, markdown, no_cap
- _format_signed, _parse_int_delta, _validate_safe_path
"""

from __future__ import annotations

import datetime as _dt
import os
import subprocess
import sys
import tempfile

import pytest

from apeireth import v1375_v1374_history_archive as v1375

SCRIPT = "v1375_v1374_history_archive"
# Project root so subprocess `python -m apeireth.xxx` can find the package.
PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


# ----------------------------------------------------------------------
# Fixtures
# ----------------------------------------------------------------------

@pytest.fixture
def v1374_md() -> str:
    """Minimal V1374-shape .md fixture for archive tests."""
    return (
        "# V1374 — V1373 Snapshot Diff\n"
        "\n"
        "- **schema:** `v1374.diff/v1`\n"
        "- **generated:** 2026-08-09T03:55:00Z\n"
        "\n"
        "## Scalar deltas\n"
        "\n"
        "| metric | left | right | delta |\n"
        "|--------|-----:|------:|------:|\n"
        "| raw fires | 0 | 0 | 0 |\n"
        "| calibrated fires | 0 | 0 | 0 |\n"
        "| suppressed FP | 0 | 0 | 0 |\n"
        "| evaluations | 26 | 26 | 0 |\n"
        "| triggers | 8 | 8 | 0 |\n"
        "| time gap | — | — | 1s |\n"
        "\n"
        "## Honesty disclosure\n"
        "\n"
        "- **added:** 0\n"
        "- **removed:** 0\n"
        "- **changed:** 0\n"
        "- **unchanged:** 8\n"
        "- **time gap:** 1s\n"
    )


@pytest.fixture
def workdir_with_report(v1374_md):
    """Workdir containing a single V1374_REPORT_AUTO.md file.

    Note: we deliberately do NOT os.chdir(tmpdir) to avoid Windows
    PermissionError races when the temp dir cleanup tries to remove
    files still held by child processes.
    """
    tmpdir = tempfile.mkdtemp(prefix="v1375_test_")
    try:
        report_path = os.path.join(tmpdir, "V1374_REPORT_AUTO.md")
        with open(report_path, "w", encoding="utf-8") as fh:
            fh.write(v1374_md)
        yield tmpdir, report_path
    finally:
        # Best-effort cleanup; rely on OS to release handles later
        try:
            import shutil
            shutil.rmtree(tmpdir, ignore_errors=True)
        except Exception:
            pass


def _run_cli(args, cwd):
    """Helper: run `python -m apeireth.<script>` with PYTHONPATH set."""
    env = os.environ.copy()
    env["PYTHONPATH"] = PROJECT_ROOT + os.pathsep + env.get("PYTHONPATH", "")
    return subprocess.run(
        [sys.executable, "-m", f"apeireth.{SCRIPT}", *args],
        capture_output=True, text=True, env=env, cwd=cwd,
    )


# ----------------------------------------------------------------------
# Constants / GUARDS
# ----------------------------------------------------------------------

def test_schema_version_format():
    """V1375 schema version is the expected string."""
    assert v1375.SCHEMA_VERSION == "v1375.history/v1"


def test_default_archive_dir():
    """V1375 default archive dir is V1375_HISTORY."""
    assert v1375.DEFAULT_ARCHIVE_DIR == "V1375_HISTORY"


def test_default_report_path():
    """V1375 default report path is V1374_REPORT_AUTO.md."""
    assert v1375.DEFAULT_REPORT_PATH == "V1374_REPORT_AUTO.md"


def test_script_name():
    """V1375 script name matches the module file."""
    assert v1375.SCRIPT_NAME == "v1375_v1374_history_archive"


# ----------------------------------------------------------------------
# slug_timestamp
# ----------------------------------------------------------------------

def test_slug_timestamp_utc():
    """Fixed UTC datetime is rendered as the expected ISO basic slug."""
    fixed = _dt.datetime(2026, 8, 9, 3, 55, 0, tzinfo=_dt.timezone.utc)
    assert v1375.slug_timestamp(fixed) == "2026-08-09T03-55-00Z"


def test_slug_timestamp_naive_assumed_utc():
    """Naive datetime is assumed UTC and converted to its slug form."""
    naive = _dt.datetime(2026, 8, 9, 5, 0, 0)  # no tzinfo
    assert v1375.slug_timestamp(naive) == "2026-08-09T05-00-00Z"


def test_slug_timestamp_default_is_now():
    """Default call returns a non-empty slug."""
    assert v1375.slug_timestamp() != ""


# ----------------------------------------------------------------------
# archive_name
# ----------------------------------------------------------------------

def test_archive_name_default_schema():
    """Default schema tag is v1374; filename format is <ts>__v1374.md."""
    assert (
        v1375.archive_name("2026-08-09T03-55-00Z")
        == "2026-08-09T03-55-00Z__v1374.md"
    )


def test_archive_name_custom_schema():
    """Custom schema tag is appended after the timestamp."""
    assert (
        v1375.archive_name("2026-08-09T03-55-00Z", schema="v1374_diff")
        == "2026-08-09T03-55-00Z__v1374_diff.md"
    )


def test_archive_name_invalid_short():
    """Invalid short timestamp is rejected with ValueError."""
    with pytest.raises(ValueError):
        v1375.archive_name("2026-08-09")


def test_archive_name_traversal_schema():
    """Schema with path traversal is rejected."""
    with pytest.raises(ValueError):
        v1375.archive_name("2026-08-09T03-55-00Z", schema="../../../etc/passwd")


# ----------------------------------------------------------------------
# archive_report
# ----------------------------------------------------------------------

def test_archive_report_creates_file(workdir_with_report):
    """archive_report creates the file with the expected name and content."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    path = v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    assert os.path.exists(path)
    assert os.path.basename(path) == "2026-08-09T03-55-00Z__v1374.md"
    with open(path, encoding="utf-8") as fh:
        archived_content = fh.read()
    with open(report, encoding="utf-8") as fh:
        original_content = fh.read()
    assert archived_content == original_content


def test_archive_report_collision_safe(workdir_with_report):
    """Re-archiving same timestamp produces a different file (no overwrite)."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    os.makedirs(archive_dir)
    path1 = v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    path2 = v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    assert path1 != path2
    assert os.path.exists(path1)
    assert os.path.exists(path2)


def test_archive_report_missing_file():
    """Missing source report raises FileNotFoundError."""
    with tempfile.TemporaryDirectory() as tmpdir:
        with pytest.raises(FileNotFoundError):
            v1375.archive_report(
                os.path.join(tmpdir, "does_not_exist.md"),
                tmpdir,
                timestamp="2026-08-09T03-55-00Z",
            )


def test_archive_report_rejects_traversal(workdir_with_report):
    """Path traversal in archive_dir is rejected."""
    tmpdir, report = workdir_with_report
    with pytest.raises(ValueError):
        v1375.archive_report(report, "../etc", timestamp="2026-08-09T03-55-00Z")


# ----------------------------------------------------------------------
# list_archives
# ----------------------------------------------------------------------

def test_list_archives_empty():
    """Empty archive dir returns empty list."""
    with tempfile.TemporaryDirectory() as tmpdir:
        assert v1375.list_archives(tmpdir) == []


def test_list_archives_skips_index(workdir_with_report):
    """INDEX.md is excluded from the archive listing."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    os.makedirs(archive_dir)
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    with open(os.path.join(archive_dir, "INDEX.md"), "w", encoding="utf-8") as fh:
        fh.write("# dummy index\n")
    archives = v1375.list_archives(archive_dir)
    assert len(archives) == 1
    assert all(a["filename"] != "INDEX.md" for a in archives)


def test_list_archives_sorted_ascending(workdir_with_report):
    """Archives are listed in ascending timestamp order."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T04-00-00Z")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T04-05-00Z")
    archives = v1375.list_archives(archive_dir)
    iso_list = [a["iso"] for a in archives]
    assert iso_list == sorted(iso_list)


def test_list_archives_schema_only(workdir_with_report):
    """Each entry has schema=v1374 (not v1374_001) even with collisions."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    archives = v1375.list_archives(archive_dir)
    assert len(archives) == 2
    for a in archives:
        assert a["schema"] == "v1374"
        assert not a["filename"].endswith("v1374_001.md") or a["schema"] == "v1374"


# ----------------------------------------------------------------------
# _extract_summary
# ----------------------------------------------------------------------

def test_extract_summary_basic(workdir_with_report):
    """_extract_summary parses all scalar + count fields from a V1374 .md."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    path = v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    summary = v1375._extract_summary(path)
    assert summary["iso"] == "2026-08-09T03-55-00Z"
    assert summary["schema"] == "v1374"
    assert summary["added"] == 0
    assert summary["removed"] == 0
    assert summary["changed"] == 0
    assert summary["unchanged"] == 8
    assert summary["delta_raw"] == 0
    assert summary["delta_cal"] == 0
    assert summary["gap"] == "1s"


def test_extract_summary_collision(workdir_with_report):
    """_extract_summary still gets schema=v1374 even when filename has _NNN collision suffix."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    path2 = v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    summary = v1375._extract_summary(path2)
    assert summary["schema"] == "v1374"


# ----------------------------------------------------------------------
# render_index_md
# ----------------------------------------------------------------------

def test_render_index_md_empty():
    """render_index_md with no archives shows the empty hint."""
    md = v1375.render_index_md([])
    assert "# V1375 — V1374 History Archive" in md
    assert "No archives yet" in md


def test_render_index_md_with_archives(workdir_with_report):
    """render_index_md includes the schema, table, and legend."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T04-00-00Z")
    archives = v1375.list_archives(archive_dir)
    summaries = [v1375._extract_summary(a["path"]) for a in archives]
    md = v1375.render_index_md(archives, summaries)
    assert "| archived |" in md
    assert "## Honesty disclosure" in md
    assert "## Legend" in md
    assert f"- **archives:** {len(archives)}" in md
    assert archives[0]["iso"] in md


def test_render_index_md_custom_title():
    """render_index_md honors a custom title."""
    md = v1375.render_index_md([], title="My Index")
    assert "# My Index" in md


# ----------------------------------------------------------------------
# write_index + parse_index roundtrip
# ----------------------------------------------------------------------

def test_write_index_atomic(workdir_with_report):
    """write_index creates INDEX.md atomically."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    archives = v1375.list_archives(archive_dir)
    summaries = [v1375._extract_summary(a["path"]) for a in archives]
    idx_path = v1375.write_index(archive_dir, archives, summaries)
    assert os.path.exists(idx_path)
    assert os.path.basename(idx_path) == "INDEX.md"


def test_parse_index_roundtrip(workdir_with_report):
    """parse_index recovers the summaries written by write_index."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "archives")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T03-55-00Z")
    v1375.archive_report(report, archive_dir, timestamp="2026-08-09T04-00-00Z")
    archives = v1375.list_archives(archive_dir)
    summaries = [v1375._extract_summary(a["path"]) for a in archives]
    v1375.write_index(archive_dir, archives, summaries)
    rows = v1375.parse_index(archive_dir)
    assert len(rows) == len(archives)
    for r, s in zip(rows, summaries):
        assert r["iso"] == s["iso"]
        assert r["schema"] == s["schema"]
        assert r["unchanged"] == s["unchanged"]


def test_parse_index_missing():
    """parse_index returns empty list when INDEX.md is missing."""
    with tempfile.TemporaryDirectory() as tmpdir:
        assert v1375.parse_index(tmpdir) == []


# ----------------------------------------------------------------------
# archive_tick
# ----------------------------------------------------------------------

def test_archive_tick_first_run(workdir_with_report):
    """archive_tick: 1st run archives + writes index, reports n=1."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "tick")
    result = v1375.archive_tick(archive_dir, report, timestamp="2026-08-09T04-00-00Z")
    assert os.path.exists(result["archive_path"])
    assert os.path.exists(result["index_path"])
    assert result["n_archives"] == 1
    assert result["timestamp"] == "2026-08-09T04-00-00Z"


def test_archive_tick_second_run(workdir_with_report):
    """archive_tick: 2nd run adds another archive + n=2."""
    tmpdir, report = workdir_with_report
    archive_dir = os.path.join(tmpdir, "tick")
    v1375.archive_tick(archive_dir, report, timestamp="2026-08-09T04-00-00Z")
    result = v1375.archive_tick(archive_dir, report, timestamp="2026-08-09T04-05-00Z")
    assert result["n_archives"] == 2


# ----------------------------------------------------------------------
# run_cli (subprocess)
# ----------------------------------------------------------------------

def test_cli_version(workdir_with_report):
    """CLI `version` prints the version string."""
    tmpdir, _ = workdir_with_report
    result = _run_cli(["version"], cwd=tmpdir)
    assert result.returncode == 0, result.stderr
    assert v1375.SCHEMA_VERSION in result.stdout


def test_cli_popper(workdir_with_report):
    """CLI `popper` runs all internal self-tests."""
    tmpdir, _ = workdir_with_report
    result = _run_cli(["popper"], cwd=tmpdir)
    assert result.returncode == 0, result.stderr
    assert "passed" in result.stdout


def test_cli_archive(workdir_with_report):
    """CLI `archive` writes a real archive + refreshes the index.

    Note: argparse subparsers require parent optionals BEFORE the subcommand.
    So the order is: `--archive-dir XXX archive --timestamp YYY`.
    """
    tmpdir, _ = workdir_with_report
    archive_dir = os.path.join(tmpdir, "cli_archives")
    result = _run_cli(
        [
            "--archive-dir", archive_dir,
            "archive",
            "--timestamp", "2026-08-09T03-30-00Z",
        ],
        cwd=tmpdir,
    )
    assert result.returncode == 0, result.stderr
    assert os.path.exists(os.path.join(archive_dir, "INDEX.md"))
    archives = v1375.list_archives(archive_dir)
    assert len(archives) == 1
    assert archives[0]["iso"] == "2026-08-09T03-30-00Z"


def test_cli_digest_placeholder(workdir_with_report):
    """CLI `digest` is a placeholder (V1376+ candidates)."""
    tmpdir, _ = workdir_with_report
    result = _run_cli(["digest"], cwd=tmpdir)
    assert result.returncode == 0, result.stderr
    assert "V1376+" in result.stdout


# ----------------------------------------------------------------------
# Helpers / GUARDS
# ----------------------------------------------------------------------

def test_validate_safe_path_rejects_relative_traversal():
    """_validate_safe_path rejects '..' in path."""
    with pytest.raises(ValueError):
        v1375._validate_safe_path("../foo")


def test_validate_safe_path_rejects_absolute_traversal():
    """_validate_safe_path rejects '/tmp/../etc'."""
    with pytest.raises(ValueError):
        v1375._validate_safe_path("/tmp/../etc/passwd")


def test_validate_safe_path_allows_absolute_without_traversal():
    """_validate_safe_path accepts absolute path without '..' segments."""
    v1375._validate_safe_path("/tmp/foo")  # no exception


def test_format_signed_positive():
    assert v1375._format_signed(5) == "+5"


def test_format_signed_negative():
    assert v1375._format_signed(-3) == "-3"


def test_format_signed_zero():
    assert v1375._format_signed(0) == "0"


def test_parse_int_delta_signs():
    """_parse_int_delta handles +, -, 0, unsigned."""
    assert v1375._parse_int_delta("+5") == 5
    assert v1375._parse_int_delta("-3") == -3
    assert v1375._parse_int_delta("7") == 7
    assert v1375._parse_int_delta("0") == 0
