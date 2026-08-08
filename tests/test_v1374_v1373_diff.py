"""Tests for V1374 — V1373 Markdown diff mode.

24 pytest tests covering:
- parse_markdown on a valid V1373 .md
- parse_markdown on a missing file (raises FileNotFoundError)
- compute_diff: scalar deltas
- compute_diff: added/removed triggers
- compute_diff: antisymmetry
- render_diff_markdown: structure, no HTML, legend, honesty
- write_diff_markdown: atomic write
- diff_two_files: full pipeline
- summary_two_files: short summary
- run_cli: diff / summary / version / popper subcommands
- GUARD constants: SCHEMA_VERSION, MARKDOWN_ONLY, no_ledger, no_sidecar
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile

import pytest

from apeireth import v1374_v1373_diff as v1374

SCRIPT = "v1374_v1373_diff"


# ----------------------------------------------------------------------
# Fixtures
# ----------------------------------------------------------------------

@pytest.fixture
def md_a() -> str:
    """Synthetic V1373 markdown A: 2 triggers, 10 evals, 5 raw fires on T_FIRE."""
    return (
        "# V1373 — V1372 Markdown Export\n"
        "\n"
        "- **schema:** `v1373.markdown/v1`\n"
        "- **generated:** 2026-08-08T19:00:00Z\n"
        "- **source sidecar:** `sidecar_a.jsonl`\n"
        "- **triggers:** 2\n"
        "- **evaluations:** 10\n"
        "\n"
        "## Per-trigger timeline\n"
        "\n"
        "| trigger | kind | timeline | raw | cal | sup |\n"
        "|---------|------|----------|----:|----:|----:|\n"
        "| `T_FIRE` | remeasure | `·5●5` | 5 | 5 | 0 |\n"
        "| `T_QUIET` | remeasure | `·10` | 0 | 0 | 0 |\n"
        "\n"
        "## Summary\n"
        "\n"
        "| trigger | kind | fire_rate |\n"
        "|---------|------|----------:|\n"
        "| `T_FIRE` | remeasure | 50.00% |\n"
        "| `T_QUIET` | remeasure | 0.00% |\n"
        "\n"
        "## Legend\n"
        "\n"
        "| char | meaning |\n"
        "|------|---------|\n"
        "| `·` | no fire |\n"
        "| `●` | raw fire |\n"
        "\n"
        "## Honesty disclosure\n"
        "\n"
        "- **trigger-checks evaluated:** 20 (10 evaluations × 2 triggers)\n"
        "- **raw fires:** 5\n"
        "- **calibrated fires:** 5\n"
        "- **V1370-suppressed false positives:** 0\n"
        "\n"
    )


@pytest.fixture
def md_b() -> str:
    """Synthetic V1373 markdown B: 3 triggers, 12 evals, adds T_NEW."""
    return (
        "# V1373 — V1372 Markdown Export\n"
        "\n"
        "- **schema:** `v1373.markdown/v1`\n"
        "- **generated:** 2026-08-08T20:00:00Z\n"
        "- **source sidecar:** `sidecar_b.jsonl`\n"
        "- **triggers:** 3\n"
        "- **evaluations:** 12\n"
        "\n"
        "## Per-trigger timeline\n"
        "\n"
        "| trigger | kind | timeline | raw | cal | sup |\n"
        "|---------|------|----------|----:|----:|----:|\n"
        "| `T_FIRE` | remeasure | `·7●5` | 5 | 5 | 0 |\n"
        "| `T_QUIET` | remeasure | `·12` | 0 | 0 | 0 |\n"
        "| `T_NEW` | v03_evolution | `·12` | 0 | 0 | 0 |\n"
        "\n"
        "## Summary\n"
        "\n"
        "| trigger | kind | fire_rate |\n"
        "|---------|------|----------:|\n"
        "| `T_FIRE` | remeasure | 41.67% |\n"
        "| `T_QUIET` | remeasure | 0.00% |\n"
        "| `T_NEW` | v03_evolution | 0.00% |\n"
        "\n"
        "## Legend\n"
        "\n"
        "| char | meaning |\n"
        "|------|---------|\n"
        "| `·` | no fire |\n"
        "| `●` | raw fire |\n"
        "\n"
        "## Honesty disclosure\n"
        "\n"
        "- **trigger-checks evaluated:** 36 (12 evaluations × 3 triggers)\n"
        "- **raw fires:** 5\n"
        "- **calibrated fires:** 5\n"
        "- **V1370-suppressed false positives:** 0\n"
        "\n"
    )


@pytest.fixture
def tmp_md_pair(md_a, md_b):
    """Write two V1373 .md files into a temp dir and return paths."""
    with tempfile.TemporaryDirectory() as td:
        a_path = os.path.join(td, "a.md")
        b_path = os.path.join(td, "b.md")
        with open(a_path, "w", encoding="utf-8") as fh:
            fh.write(md_a)
        with open(b_path, "w", encoding="utf-8") as fh:
            fh.write(md_b)
        yield a_path, b_path, td


# ----------------------------------------------------------------------
# parse_markdown
# ----------------------------------------------------------------------

def test_parse_markdown_basic(tmp_md_pair):
    a_path, _, _ = tmp_md_pair
    p = v1374.parse_markdown(a_path)
    assert p["schema"] == "v1373.markdown/v1"
    assert p["generated"] == "2026-08-08T19:00:00Z"
    assert p["source"] == "sidecar_a.jsonl"
    assert p["n_triggers"] == 2
    assert p["n_evals"] == 10
    assert len(p["timeline"]) == 2
    assert p["honesty"]["raw"] == 5
    assert p["honesty"]["cal"] == 5
    assert p["honesty"]["trigger_checks"] == 20


def test_parse_markdown_missing_raises():
    with tempfile.TemporaryDirectory() as td:
        with pytest.raises(FileNotFoundError):
            v1374.parse_markdown(os.path.join(td, "nope.md"))


def test_parse_markdown_timeline_rows(tmp_md_pair):
    a_path, _, _ = tmp_md_pair
    p = v1374.parse_markdown(a_path)
    by_name = {t["name"]: t for t in p["timeline"]}
    assert by_name["T_FIRE"]["raw"] == 5
    assert by_name["T_FIRE"]["cal"] == 5
    assert by_name["T_QUIET"]["raw"] == 0


def test_parse_markdown_summary_rates(tmp_md_pair):
    a_path, _, _ = tmp_md_pair
    p = v1374.parse_markdown(a_path)
    by_name = {t["name"]: t for t in p["summary"]}
    assert by_name["T_FIRE"]["fire_rate"] == 50.0
    assert by_name["T_QUIET"]["fire_rate"] == 0.0


# ----------------------------------------------------------------------
# compute_diff
# ----------------------------------------------------------------------

def test_compute_diff_scalar_deltas(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d = v1374.compute_diff(a, b)
    assert d["delta_raw_total"] == 0
    assert d["delta_cal_total"] == 0
    assert d["delta_evals"] == 2
    assert d["delta_triggers"] == 1
    assert d["delta_time_seconds"] == 3600


def test_compute_diff_added_removed(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d = v1374.compute_diff(a, b)
    assert d["added"] == ["T_NEW"]
    assert d["removed"] == []


def test_compute_diff_antisymmetry(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d_ab = v1374.compute_diff(a, b)
    d_ba = v1374.compute_diff(b, a)
    assert d_ab["delta_time_seconds"] == -d_ba["delta_time_seconds"]
    assert d_ab["delta_evals"] == -d_ba["delta_evals"]
    assert d_ab["delta_triggers"] == -d_ba["delta_triggers"]
    assert d_ab["added"] == d_ba["removed"]
    assert d_ab["removed"] == d_ba["added"]


def test_compute_diff_per_trigger_changed(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d = v1374.compute_diff(a, b)
    by_name = {t["name"]: t for t in d["trigger_diffs"]}
    # T_FIRE: raw 5→5 (same), cal 5→5 (same), sup 0→0 (same) but rate 50→41.67
    # Rate delta != 0 (denominator shifted because evals went 10→12), so it
    # counts as SYM_CHANGED. This is intentional: rate signal is meaningful.
    assert by_name["T_FIRE"]["status"] == "~"
    # T_QUIET: unchanged (zero counts, zero rate)
    assert by_name["T_QUIET"]["status"] == "="
    # T_NEW: added
    assert by_name["T_NEW"]["status"] == "+"


def test_compute_diff_identical_files(tmp_md_pair):
    a_path, _, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    d = v1374.compute_diff(a, a)
    assert d["delta_raw_total"] == 0
    assert d["delta_cal_total"] == 0
    assert d["delta_evals"] == 0
    assert d["delta_triggers"] == 0
    assert d["delta_time_seconds"] == 0
    assert d["added"] == []
    assert d["removed"] == []


def test_compute_diff_removed_trigger():
    """Verify a trigger present in left but not right is marked SYM_REMOVED."""
    with tempfile.TemporaryDirectory() as td:
        a_path = os.path.join(td, "a.md")
        b_path = os.path.join(td, "b.md")
        with open(a_path, "w", encoding="utf-8") as fh:
            fh.write(
                "# V1373 — V1372 Markdown Export\n"
                "\n"
                "- **schema:** `v1373.markdown/v1`\n"
                "- **generated:** 2026-08-08T19:00:00Z\n"
                "- **source sidecar:** `sidecar.jsonl`\n"
                "- **triggers:** 1\n"
                "- **evaluations:** 5\n"
                "\n"
                "## Per-trigger timeline\n"
                "\n"
                "| trigger | kind | timeline | raw | cal | sup |\n"
                "|---------|------|----------|----:|----:|----:|\n"
                "| `T_OLD` | remeasure | `·5` | 0 | 0 | 0 |\n"
                "\n"
                "## Summary\n"
                "\n"
                "| trigger | kind | fire_rate |\n"
                "|---------|------|----------:|\n"
                "| `T_OLD` | remeasure | 0.00% |\n"
                "\n"
                "## Honesty disclosure\n"
                "\n"
                "- **trigger-checks evaluated:** 5 (5 evaluations × 1 triggers)\n"
                "- **raw fires:** 0\n"
                "- **calibrated fires:** 0\n"
                "- **V1370-suppressed false positives:** 0\n"
                "\n"
            )
        # Empty B (no triggers)
        with open(b_path, "w", encoding="utf-8") as fh:
            fh.write(
                "# V1373 — V1372 Markdown Export\n"
                "\n"
                "- **schema:** `v1373.markdown/v1`\n"
                "- **generated:** 2026-08-08T20:00:00Z\n"
                "- **source sidecar:** `sidecar.jsonl`\n"
                "- **triggers:** 0\n"
                "- **evaluations:** 0\n"
                "\n"
                "## Per-trigger timeline\n"
                "\n"
                "## Summary\n"
                "\n"
                "## Honesty disclosure\n"
                "\n"
                "- **trigger-checks evaluated:** 0 (0 evaluations × 0 triggers)\n"
                "- **raw fires:** 0\n"
                "- **calibrated fires:** 0\n"
                "- **V1370-suppressed false positives:** 0\n"
                "\n"
            )
        a = v1374.parse_markdown(a_path)
        b = v1374.parse_markdown(b_path)
        d = v1374.compute_diff(a, b)
        assert d["removed"] == ["T_OLD"]
        assert d["added"] == []
        by_name = {t["name"]: t for t in d["trigger_diffs"]}
        assert by_name["T_OLD"]["status"] == "-"


# ----------------------------------------------------------------------
# render_diff_markdown
# ----------------------------------------------------------------------

def test_render_diff_markdown_structure(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d = v1374.compute_diff(a, b)
    md = v1374.render_diff_markdown(d, left_path=a_path, right_path=b_path)
    assert md.startswith("# ")
    assert "v1374.diff/v1" in md
    assert "## Scalar deltas" in md
    assert "## Per-trigger deltas" in md
    assert "## Legend" in md
    assert "## Honesty disclosure" in md
    # No HTML
    assert "<" not in md and ">" not in md


def test_render_diff_markdown_honesty_block(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d = v1374.compute_diff(a, b)
    md = v1374.render_diff_markdown(d, left_path=a_path, right_path=b_path)
    assert "honest" in md.lower() or "Honest" in md
    assert "antisymmetry" in md.lower()


def test_render_format_delta():
    assert v1374._format_delta(0) == "0"
    assert v1374._format_delta(5) == "+5"
    assert v1374._format_delta(-3) == "-3"


def test_render_format_delta_pct():
    assert v1374._format_delta_pct(0.0) == "0.00%"
    assert v1374._format_delta_pct(1.5) == "+1.50%"
    assert v1374._format_delta_pct(-2.25) == "-2.25%"


def test_render_format_duration():
    assert v1374._format_duration(0) == "0s"
    assert v1374._format_duration(45) == "45s"
    assert v1374._format_duration(60) == "1m00s"
    assert v1374._format_duration(3600) == "1h00m"
    assert v1374._format_duration(-3600) == "-1h00m"
    assert v1374._format_duration(-60) == "-1m00s"


# ----------------------------------------------------------------------
# file I/O
# ----------------------------------------------------------------------

def test_write_diff_markdown_atomic(tmp_md_pair):
    a_path, b_path, td = tmp_md_pair
    a = v1374.parse_markdown(a_path)
    b = v1374.parse_markdown(b_path)
    d = v1374.compute_diff(a, b)
    md = v1374.render_diff_markdown(d, left_path=a_path, right_path=b_path)
    out_path = os.path.join(td, "diff.md")
    v1374.write_diff_markdown(out_path, md)
    assert os.path.exists(out_path)
    assert os.path.getsize(out_path) > 200
    with open(out_path, "r", encoding="utf-8") as fh:
        content = fh.read()
    assert content == md


def test_diff_two_files_returns_zero(tmp_md_pair):
    a_path, b_path, td = tmp_md_pair
    rc = v1374.diff_two_files(a_path, b_path, out_path=os.path.join(td, "diff.md"))
    assert rc == 0


def test_diff_two_files_missing_left(tmp_md_pair):
    _, b_path, td = tmp_md_pair
    rc = v1374.diff_two_files(os.path.join(td, "missing.md"), b_path)
    assert rc == 2


def test_diff_two_files_missing_right(tmp_md_pair):
    a_path, _, td = tmp_md_pair
    rc = v1374.diff_two_files(a_path, os.path.join(td, "missing.md"))
    assert rc == 2


def test_summary_two_files(tmp_md_pair, capsys):
    a_path, b_path, _ = tmp_md_pair
    rc = v1374.summary_two_files(a_path, b_path)
    assert rc == 0
    captured = capsys.readouterr()
    assert "V1374 diff summary" in captured.out
    assert "delta raw fires" in captured.out
    assert "added:" in captured.out


# ----------------------------------------------------------------------
# CLI
# ----------------------------------------------------------------------

def test_run_cli_version():
    rc = v1374.run_cli(["version"])
    assert rc == 0


def test_run_cli_popper():
    rc = v1374.run_cli(["popper"])
    # 32 popper self-tests; should pass
    assert rc == 0


def test_run_cli_popper_verbose():
    rc = v1374.run_cli(["popper", "-v"])
    assert rc == 0


def test_run_cli_diff(tmp_md_pair):
    a_path, b_path, td = tmp_md_pair
    out_path = os.path.join(td, "diff.md")
    rc = v1374.run_cli(["diff", "--left", a_path, "--right", b_path, "--out", out_path])
    assert rc == 0
    assert os.path.exists(out_path)


def test_run_cli_summary(tmp_md_pair):
    a_path, b_path, _ = tmp_md_pair
    rc = v1374.run_cli(["summary", "--left", a_path, "--right", b_path])
    assert rc == 0


def test_run_cli_default_subcommand(tmp_md_pair):
    """Without an explicit subcommand, default to 'diff'."""
    a_path, b_path, td = tmp_md_pair
    out_path = os.path.join(td, "diff.md")
    # pass --left/--right without 'diff' subcommand
    rc = v1374.run_cli(["--left", a_path, "--right", b_path, "--out", out_path])
    assert rc == 0


# ----------------------------------------------------------------------
# GUARDS
# ----------------------------------------------------------------------

def test_guard_schema_version():
    assert v1374.SCHEMA_VERSION == "v1374.diff/v1"


def test_guard_no_ledger_import():
    """V1374 must not import V1362 or V1368 (ledger / trigger-spec)."""
    src = open(v1374.__file__, "r", encoding="utf-8").read()
    assert "v1362" not in src
    assert "v1368" not in src


def test_guard_no_sidecar_import():
    """V1374 must not import V1371 sidecar or V1372 timeline."""
    src = open(v1374.__file__, "r", encoding="utf-8").read()
    assert "v1371" not in src
    assert "v1372" not in src


# ----------------------------------------------------------------------
# subprocess (end-to-end)
# ----------------------------------------------------------------------

def test_subprocess_version():
    result = subprocess.run(
        [sys.executable, "-m", f"apeireth.{SCRIPT}", "version"],
        capture_output=True, text=True,
    )
    assert result.returncode == 0
    assert "v1374.diff/v1" in result.stdout


def test_subprocess_popper():
    result = subprocess.run(
        [sys.executable, "-m", f"apeireth.{SCRIPT}", "popper"],
        capture_output=True, text=True,
    )
    assert result.returncode == 0
    assert "Popper self-tests:" in result.stdout
    assert "32/32 passed" in result.stdout
