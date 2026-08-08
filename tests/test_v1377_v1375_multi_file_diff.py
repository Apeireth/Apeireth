"""Tests for V1377 — V1375 Multi-File Diff.

Run from promethean/:
    python -m pytest tests/test_v1377_v1375_multi_file_diff.py -v
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

import apeireth.v1377_v1375_multi_file_diff as v1377


# ----------------------------------------------------------------------
# Synthetic V1374-family fixtures
# ----------------------------------------------------------------------

def _build_v1374_md(
    generated: str,
    *,
    scalars: list[tuple[str, str, str]] | None = None,
    per_trigger: list[tuple[str, str, str, int, int, int, str]] | None = None,
    honesty: dict[str, int] | None = None,
    left: str = "A.md",
    right: str = "B.md",
) -> str:
    """Build a V1374-family markdown content string.

    scalars: list of (metric, left, right, delta) — left/right are strings, delta is int formatted as signed
    per_trigger: list of (status, trigger, kind, raw, cal, sup, rate_pct)
    honesty: dict with triggers/added/removed/changed/unchanged
    """
    if scalars is None:
        scalars = [
            ("raw fires", "0", "0", "0"),
            ("calibrated fires", "0", "0", "0"),
            ("suppressed FP", "0", "0", "0"),
            ("evaluations", "10", "11", "+1"),
            ("triggers", "8", "8", "0"),
        ]
    if per_trigger is None:
        per_trigger = [
            ("=", "TRIG_A", "remeasure", 0, 0, 0, "0.00%"),
            ("~", "TRIG_B", "remeasure", 1, 0, 0, "1.00%"),
        ]
    if honesty is None:
        honesty = {"triggers": 8, "added": 0, "removed": 0, "changed": 1, "unchanged": 7}

    lines = [
        "# V1374 — V1373 Snapshot Diff",
        "",
        "- **schema:** `v1374.diff/v1`",
        f"- **generated:** {generated}",
        f"- **left:** `{left}`",
        f"- **right:** `{right}`",
        "- **left schema:** `v1373.markdown/v1`",
        "- **right schema:** `v1373.markdown/v1`",
        f"- **triggers compared:** {honesty['triggers']}",
        "",
        "## Scalar deltas",
        "",
        "| metric | left | right | delta |",
        "|--------|-----:|------:|------:|",
    ]
    for metric, lv, rv, dv in scalars:
        lines.append(f"| {metric} | {lv} | {rv} | {dv} |")
    lines.append("")
    lines.append("## Per-trigger deltas")
    lines.append("")
    lines.append("| status | trigger | kind | raw Δ | cal Δ | sup Δ | rate Δ |")
    lines.append("|:------:|---------|------|------:|------:|------:|-------:|")
    for status, trig, kind, raw, cal, sup, rate in per_trigger:
        lines.append(f"| {status} | `{trig}` | {kind} | {raw} | {cal} | {sup} | {rate} |")
    lines.append("")
    lines.append("## Honesty disclosure")
    lines.append("")
    lines.append(f"- **trigger-rows compared:** {honesty['triggers']}")
    lines.append(f"- **added:** {honesty['added']}")
    lines.append(f"- **removed:** {honesty['removed']}")
    lines.append(f"- **changed:** {honesty['changed']}")
    lines.append(f"- **unchanged:** {honesty['unchanged']}")
    lines.append("")
    return "\n".join(lines)


@pytest.fixture
def tmp_v1374_dir(tmp_path: Path) -> Path:
    """Create a temp dir with 3 V1374-family files at different timestamps."""
    a = _build_v1374_md(
        "2026-08-09T03:00:00Z",
        per_trigger=[
            ("=", "T1", "remeasure", 0, 0, 0, "0.00%"),
            ("=", "T2", "remeasure", 1, 0, 0, "1.00%"),
        ],
    )
    b = _build_v1374_md(
        "2026-08-09T04:00:00Z",
        per_trigger=[
            ("~", "T1", "remeasure", 1, 0, 0, "1.00%"),
            ("~", "T2", "remeasure", 2, 1, 0, "2.00%"),
            ("=", "T3", "v03_evolution", 0, 0, 0, "0.00%"),
        ],
    )
    c = _build_v1374_md(
        "2026-08-09T05:00:00Z",
        per_trigger=[
            ("=", "T1", "remeasure", 1, 0, 0, "1.00%"),
            ("=", "T2", "remeasure", 1, 0, 0, "1.00%"),
        ],
    )
    (tmp_path / "a.md").write_text(a, encoding="utf-8")
    (tmp_path / "b.md").write_text(b, encoding="utf-8")
    (tmp_path / "c.md").write_text(c, encoding="utf-8")
    return tmp_path


# ----------------------------------------------------------------------
# Constants & GUARDS
# ----------------------------------------------------------------------

class TestConstants:
    def test_schema_version(self):
        assert v1377.SCHEMA_VERSION == "v1377.multidiff/v1"

    def test_script_name(self):
        assert v1377.SCRIPT_NAME == "v1377_v1375_multi_file_diff"

    def test_v1374_schema(self):
        assert v1377.V1374_SCHEMA == "v1374.diff/v1"

    def test_default_output_path(self):
        assert v1377.DEFAULT_OUTPUT_PATH == "V1377_REPORT_AUTO.md"

    def test_guards_count(self):
        assert len(v1377.GUARDS) == 10

    def test_required_guards(self):
        required = [
            "GUARD_INPUT_V1374_FAMILY",
            "GUARD_CHRONOLOGICAL_SORT",
            "GUARD_DETERMINISTIC",
            "GUARD_ATOMIC_WRITE",
            "GUARD_NO_LEDGER_TOUCH",
            "GUARD_NO_SIDECAR_TOUCH",
            "GUARD_HONEST_DISCLOSURE",
            "GUARD_MARKDOWN_ONLY",
            "GUARD_NO_CAP_CHANGE",
            "GUARD_MIN_INPUT_2",
        ]
        for g in required:
            assert g in v1377.GUARDS


# ----------------------------------------------------------------------
# Helpers
# ----------------------------------------------------------------------

class TestHelpers:
    def test_parse_iso_z(self):
        dt = v1377._parse_iso("2026-08-09T04:00:00Z")
        assert dt is not None
        assert dt.year == 2026 and dt.hour == 4

    def test_parse_iso_offset(self):
        dt = v1377._parse_iso("2026-08-09T04:00:00+00:00")
        assert dt is not None

    def test_parse_iso_none(self):
        assert v1377._parse_iso(None) is None

    def test_parse_iso_bad(self):
        assert v1377._parse_iso("not-a-date") is None

    def test_int_or_none_dash(self):
        assert v1377._int_or_none("—") is None

    def test_int_or_none_int(self):
        assert v1377._int_or_none("42") == 42

    def test_int_or_none_neg(self):
        assert v1377._int_or_none("-3") == -3

    def test_int_or_none_empty(self):
        assert v1377._int_or_none("") is None

    def test_format_signed(self):
        assert v1377._format_signed(5) == "+5"
        assert v1377._format_signed(-5) == "-5"
        assert v1377._format_signed(0) == "0"

    def test_is_separator_header(self):
        assert v1377._is_separator_or_header("metric") is True
        assert v1377._is_separator_or_header("trigger") is True
        assert v1377._is_separator_or_header(":------:") is True
        assert v1377._is_separator_or_header("---") is True
        assert v1377._is_separator_or_header("TRIG_A") is False
        assert v1377._is_separator_or_header("raw fires") is False


# ----------------------------------------------------------------------
# parse_v1374_diff_md
# ----------------------------------------------------------------------

class TestParseV1374Diff:
    def test_parse_synthetic(self, tmp_path: Path):
        p = tmp_path / "x.md"
        p.write_text(_build_v1374_md("2026-08-09T04:00:00Z"), encoding="utf-8")
        parsed = v1377.parse_v1374_diff_md(str(p))
        assert parsed["schema"] == "v1374.diff/v1"
        assert parsed["generated"] == "2026-08-09T04:00:00Z"
        assert parsed["n_triggers"] == 8
        assert len(parsed["per_trigger"]) == 2
        assert parsed["per_trigger"][0]["trigger"] == "TRIG_A"
        assert len(parsed["scalars"]) == 5
        assert parsed["honesty"]["triggers"] == 8
        assert parsed["honesty"]["changed"] == 1

    def test_parse_real_v1374(self):
        """Parse the actual V1374_REPORT_AUTO.md if it exists."""
        path = "V1374_REPORT_AUTO.md"
        if not os.path.exists(path):
            pytest.skip("V1374_REPORT_AUTO.md not present")
        parsed = v1377.parse_v1374_diff_md(path)
        assert parsed["schema"] == "v1374.diff/v1"
        assert parsed["generated"] is not None
        assert parsed["n_triggers"] == 8
        assert len(parsed["per_trigger"]) == 8
        assert len(parsed["scalars"]) >= 5
        # First trigger
        assert parsed["per_trigger"][0]["trigger"] == "CAP_BECOMES_DISHONEST"

    def test_parse_wrong_schema_raises(self, tmp_path: Path):
        p = tmp_path / "bad.md"
        p.write_text("- **schema:** `v1373.markdown/v1`\n", encoding="utf-8")
        with pytest.raises(ValueError):
            v1377.parse_v1374_diff_md(str(p))

    def test_parse_missing_raises(self):
        with pytest.raises(FileNotFoundError):
            v1377.parse_v1374_diff_md("/nonexistent/path.md")

    def test_parse_skips_header_separator(self, tmp_path: Path):
        """Header rows and separator rows must not appear in parsed output."""
        p = tmp_path / "x.md"
        p.write_text(_build_v1374_md("2026-08-09T04:00:00Z"), encoding="utf-8")
        parsed = v1377.parse_v1374_diff_md(str(p))
        triggers = [r["trigger"] for r in parsed["per_trigger"]]
        assert "trigger" not in triggers
        assert "---------" not in triggers
        metrics = [r["metric"] for r in parsed["scalars"]]
        assert "metric" not in metrics


# ----------------------------------------------------------------------
# sort_by_generated
# ----------------------------------------------------------------------

class TestSortByGenerated:
    def test_sorts_ascending(self):
        r1 = {"path": "a", "generated_dt": v1377._parse_iso("2026-08-09T03:00:00Z")}
        r2 = {"path": "b", "generated_dt": v1377._parse_iso("2026-08-09T04:00:00Z")}
        r3 = {"path": "c", "generated_dt": v1377._parse_iso("2026-08-09T05:00:00Z")}
        result = v1377.sort_by_generated([r3, r1, r2])
        assert [r["path"] for r in result] == ["a", "b", "c"]

    def test_keeps_none_at_end(self):
        r1 = {"path": "a", "generated_dt": v1377._parse_iso("2026-08-09T03:00:00Z")}
        r2 = {"path": "b", "generated_dt": None}
        result = v1377.sort_by_generated([r2, r1])
        assert result[-1]["path"] == "b"

    def test_already_sorted(self):
        r1 = {"path": "a", "generated_dt": v1377._parse_iso("2026-08-09T03:00:00Z")}
        r2 = {"path": "b", "generated_dt": v1377._parse_iso("2026-08-09T04:00:00Z")}
        result = v1377.sort_by_generated([r1, r2])
        assert result[0]["path"] == "a"

    def test_empty(self):
        assert v1377.sort_by_generated([]) == []

    def test_returns_new_list(self):
        r1 = {"path": "a", "generated_dt": None}
        original = [r1]
        result = v1377.sort_by_generated(original)
        assert result is not original  # new list


# ----------------------------------------------------------------------
# diff_pairwise
# ----------------------------------------------------------------------

class TestDiffPairwise:
    def test_n_pairs(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        assert len(pw) == 2

    def test_time_gap(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        assert pw[0]["time_gap_seconds"] == 3600

    def test_per_trigger_status(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        # T3 added in second report (b.md)
        t3 = next(r for r in pw[0]["per_trigger"] if r["trigger"] == "T3")
        assert t3["status"] == "+"
        # T3 removed in third report (c.md)
        t3 = next(r for r in pw[1]["per_trigger"] if r["trigger"] == "T3")
        assert t3["status"] == "-"

    def test_per_trigger_deltas(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        # T1: 0 → 1 → 1
        t1_p0 = next(r for r in pw[0]["per_trigger"] if r["trigger"] == "T1")
        t1_p1 = next(r for r in pw[1]["per_trigger"] if r["trigger"] == "T1")
        assert t1_p0["delta_raw"] == 1
        assert t1_p1["delta_raw"] == 0

    def test_single_returns_empty(self):
        r = {"path": "a", "per_trigger": [], "scalars": [], "generated_dt": None}
        assert v1377.diff_pairwise([r]) == []

    def test_empty_input(self):
        assert v1377.diff_pairwise([]) == []


# ----------------------------------------------------------------------
# aggregate_per_trigger
# ----------------------------------------------------------------------

class TestAggregatePerTrigger:
    def test_aggregate_count(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        agg = v1377.aggregate_per_trigger(reports)
        assert len(agg) == 3  # T1, T2, T3

    def test_aggregate_net_zero(self, tmp_v1374_dir: Path):
        """T2: 1→2→1 → net=0, total_abs=2."""
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        agg = v1377.aggregate_per_trigger(reports)
        t2 = next(a for a in agg if a["trigger"] == "T2")
        assert t2["net_delta"] == 0
        assert t2["total_abs_movement"] == 2
        assert t2["monotonic"] is False

    def test_aggregate_monotonic_positive(self, tmp_v1374_dir: Path):
        """T1: 0→1→1 → net=+1, monotonic (one positive step, one zero)."""
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        agg = v1377.aggregate_per_trigger(reports)
        t1 = next(a for a in agg if a["trigger"] == "T1")
        assert t1["net_delta"] == 1
        assert t1["monotonic"] is True

    def test_aggregate_sort_by_movement(self, tmp_v1374_dir: Path):
        """Aggregate is sorted by total_abs_movement descending."""
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        agg = v1377.aggregate_per_trigger(reports)
        movements = [a["total_abs_movement"] for a in agg]
        assert movements == sorted(movements, reverse=True)

    def test_aggregate_empty(self):
        assert v1377.aggregate_per_trigger([]) == []


# ----------------------------------------------------------------------
# summarize_drift
# ----------------------------------------------------------------------

class TestSummarizeDrift:
    def test_summary_keys(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        assert s["n_reports"] == 3
        assert s["n_pairs"] == 2
        assert s["triggers_seen"] == 3
        assert s["max_movement_trigger"] is not None

    def test_window_seconds(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        # 03:00 to 05:00 = 7200s
        assert s["total_window_seconds"] == 7200


# ----------------------------------------------------------------------
# render_multi_diff_md
# ----------------------------------------------------------------------

class TestRenderMultiDiff:
    def test_render_has_title(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        md = v1377.render_multi_diff_md(reports, pw, agg, s)
        assert "# V1377" in md

    def test_render_has_schema(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        md = v1377.render_multi_diff_md(reports, pw, agg, s)
        assert "v1377.multidiff/v1" in md

    def test_render_has_pairwise(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        md = v1377.render_multi_diff_md(reports, pw, agg, s)
        assert "Pair 1" in md
        assert "Pair 2" in md

    def test_render_deterministic(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        md1 = v1377.render_multi_diff_md(reports, pw, agg, s)
        md2 = v1377.render_multi_diff_md(reports, pw, agg, s)
        assert md1 == md2

    def test_render_honesty(self, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        md = v1377.render_multi_diff_md(reports, pw, agg, s)
        assert "Honest baseline" in md


# ----------------------------------------------------------------------
# write_multi_diff_md (atomic)
# ----------------------------------------------------------------------

class TestWriteAtomic:
    def test_roundtrip(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        content = v1377.render_multi_diff_md(reports, pw, agg, s)
        out = tmp_path / "out.md"
        v1377.write_multi_diff_md(str(out), content)
        assert out.read_text(encoding="utf-8") == content

    def test_no_tmp_leftover(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        reports = [v1377.parse_v1374_diff_md(p) for p in paths]
        reports = v1377.sort_by_generated(reports)
        pw = v1377.diff_pairwise(reports)
        agg = v1377.aggregate_per_trigger(reports)
        s = v1377.summarize_drift(reports, pw, agg)
        content = v1377.render_multi_diff_md(reports, pw, agg, s)
        out = tmp_path / "out.md"
        v1377.write_multi_diff_md(str(out), content)
        leftovers = [n for n in os.listdir(tmp_path) if n.startswith(".v1377_") and n.endswith(".tmp")]
        assert leftovers == []


# ----------------------------------------------------------------------
# run_multi_diff (all-in-one)
# ----------------------------------------------------------------------

class TestRunMultiDiff:
    def test_sorted_output(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        # Pass in reverse order to verify sorting
        result = v1377.run_multi_diff(list(reversed(paths)))
        assert [r["path"] for r in result["reports"]] == paths

    def test_writes_output(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        out = tmp_path / "out.md"
        result = v1377.run_multi_diff(paths, output_path=str(out))
        assert out.exists()
        assert result["output_path"] == str(out)

    def test_no_write(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        out = tmp_path / "out.md"
        result = v1377.run_multi_diff(paths, output_path=str(out))
        # Now run with output_path=None (no write)
        result2 = v1377.run_multi_diff(paths)
        assert result2["output_path"] is None

    def test_min_input_2_raises(self):
        with pytest.raises(ValueError):
            v1377.run_multi_diff(["/nonexistent/only.md"])

    def test_no_inputs_raises(self):
        with pytest.raises(ValueError):
            v1377.run_multi_diff([])


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

class TestCLI:
    def test_version(self):
        buf = io.StringIO()
        with redirect_stdout(buf):
            rc = v1377.run_cli(["version"])
        assert rc == 0
        assert "v1377.multidiff/v1" in buf.getvalue()

    def test_no_inputs_rc_2(self):
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1377.run_cli(["diff"])
        assert rc == 2

    def test_summary_no_inputs_rc_2(self):
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1377.run_cli(["summary"])
        assert rc == 2

    def test_missing_files_rc_1(self):
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1377.run_cli(["diff", "/nonexistent/a.md", "/nonexistent/b.md"])
        assert rc == 1

    def test_diff_subcommand(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        out = tmp_path / "out.md"
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1377.run_cli(["diff", paths[0], paths[1], "-o", str(out)])
        assert rc == 0
        assert out.exists()

    def test_summary_subcommand(self, tmp_path: Path, tmp_v1374_dir: Path):
        paths = sorted(str(p) for p in tmp_v1374_dir.glob("*.md"))
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1377.run_cli(["summary", paths[0], paths[1]])
        assert rc == 0
        assert "reports:" in buf.getvalue()

    def test_archive_dir_expansion(self, tmp_path: Path, tmp_v1374_dir: Path):
        """--archive-dir should pick up V1374-family files."""
        out = tmp_path / "out.md"
        buf = io.StringIO()
        with redirect_stdout(buf), redirect_stderr(buf):
            rc = v1377.run_cli(["diff", "--archive-dir", str(tmp_v1374_dir), "-o", str(out)])
        assert rc == 0
        assert out.exists()

    def test_subprocess_version(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1377_v1375_multi_file_diff", "version"],
            capture_output=True, text=True, cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        )
        assert result.returncode == 0
        assert "v1377.multidiff/v1" in result.stdout

    def test_subprocess_popper(self):
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1377_v1375_multi_file_diff", "popper"],
            capture_output=True, text=True, cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        )
        assert result.returncode == 0
        assert "popper self-tests" in result.stdout
        # Should be 77/77
        assert "77/77" in result.stdout

    def test_subprocess_diff_via_archive(self, tmp_path: Path):
        """End-to-end: create V1374 files, run diff via subprocess with --archive-dir."""
        a = _build_v1374_md(
            "2026-08-09T03:00:00Z",
            per_trigger=[("=", "T1", "remeasure", 0, 0, 0, "0.00%")],
        )
        b = _build_v1374_md(
            "2026-08-09T04:00:00Z",
            per_trigger=[("~", "T1", "remeasure", 1, 0, 0, "1.00%")],
        )
        (tmp_path / "a.md").write_text(a, encoding="utf-8")
        (tmp_path / "b.md").write_text(b, encoding="utf-8")
        out = tmp_path / "out.md"
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1377_v1375_multi_file_diff",
             "diff", "--archive-dir", str(tmp_path), "-o", str(out)],
            capture_output=True, text=True, cwd=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        )
        assert result.returncode == 0, f"stderr: {result.stderr}"
        assert out.exists()


# ----------------------------------------------------------------------
# V1374 sidecar honesty — V1377 must not touch V1362/V1368 or V1371
# ----------------------------------------------------------------------

class TestGuardsUpheld:
    def test_no_ledger_import(self):
        """V1377 must not actually import V1362 (ledger) or V1368 (triggers).

        The docstring may mention these names, but the source must not import them.
        """
        src = Path(v1377.__file__).read_text(encoding="utf-8")
        # Look only at code lines (skip docstring/header)
        # Any non-comment line containing `apeireth.v1362` or `apeireth.v1368` would be an import.
        assert "import apeireth.v1362" not in src
        assert "from apeireth.v1362 import" not in src
        assert "import apeireth.v1368" not in src
        assert "from apeireth.v1368 import" not in src

    def test_no_sidecar_import(self):
        """V1377 must not actually import V1371 (calibrated cron hook sidecar)."""
        src = Path(v1377.__file__).read_text(encoding="utf-8")
        assert "import apeireth.v1371" not in src
        assert "from apeireth.v1371 import" not in src

    def test_no_metrics_or_caps(self):
        """V1377 has no metric, no cap, no score module-level constants."""
        src = Path(v1377.__file__).read_text(encoding="utf-8")
        forbidden_patterns = [
            r"^\s*METRIC\s*=",
            r"^\s*SCORE\s*=",
            r"^\s*CAP\s*=",
        ]
        for pat in forbidden_patterns:
            assert not re.search(pat, src, re.MULTILINE), f"forbidden pattern: {pat}"


# ----------------------------------------------------------------------
# Popper self-test sanity
# ----------------------------------------------------------------------

class TestPopperSelfTests:
    def test_popper_returns_full(self):
        passed, total, failures = v1377._popper_self_tests()
        assert failures == [], f"Popper failures: {failures}"
        assert total >= 70
        assert passed == total