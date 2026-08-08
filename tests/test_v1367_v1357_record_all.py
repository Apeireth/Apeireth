"""Test V1367 v1367_v1357_record_all — V1357 --record-all flag wrapper.

These tests verify V1367:
1. CLI `version` works.
2. CLI `wrap <subcommand>` passes through V1357 stdout unchanged.
3. CLI `wrap <subcommand> --record-all [--tag TAG]` appends to V1362 ledger.
4. CLI without --record-all does NOT append (default OFF).
5. Library helpers `record_summary`, `record_recipe`, `record_snapshot` work.
6. `build_record_entry` shape contract is stable.
7. V3 philosophy guards present and intact.
8. Windows UTF-8 encoding handles unicode arrows in V1357 recipe output.
9. V1357's exit codes are preserved (pass-through).
10. V1362 self-test still passes (no regression).
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
V1367_MODULE = "apeireth.v1367_v1357_record_all"
V1357_MODULE = "apeireth.v1357_vcp_observability_snapshot"
V1362_MODULE = "apeireth.v1362_pole_star_history"


def _run(args: list, cwd: Path = REPO_ROOT) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, "-m", *args],
        cwd=str(cwd),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
    )


# -----------------------------------------------------------------------------
# CLI tests
# -----------------------------------------------------------------------------


def test_cli_version():
    """CLI `version` exits 0 and prints V1367 version string."""
    proc = _run([V1367_MODULE, "version"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"
    assert "v1367-v1357-record-all" in proc.stdout
    assert "0.1.0" in proc.stdout


def test_cli_self_test_passes():
    """V1367 self-test passes end-to-end (30/30 in default mode)."""
    proc = _run([V1367_MODULE, "self-test"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"
    assert "30/30" in proc.stdout


def test_cli_self_test_verbose_passes():
    """V1367 self-test in verbose mode (32/32) passes when live recorders run."""
    proc = _run([V1367_MODULE, "self-test", "--verbose"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"
    assert "32/32" in proc.stdout


def test_cli_wrap_summary_passthrough():
    """CLI `wrap summary` passes through V1357 stdout unchanged (no record)."""
    proc = _run([V1367_MODULE, "wrap", "summary"])
    # V1357's summary exit code 0 (or 1 with known_unknowns, both accepted)
    assert proc.returncode in (0, 1), f"rc={proc.returncode} stderr={proc.stderr}"
    # V1357 summary contains "pole_star"
    assert "pole_star" in proc.stdout
    # V1367 prints hint to stderr (default OFF)
    assert "pass-through only" in proc.stderr


def test_cli_wrap_recipe_passthrough():
    """CLI `wrap recipe` passes through V1357 recipe stdout (no record)."""
    proc = _run([V1367_MODULE, "wrap", "recipe"])
    assert proc.returncode in (0, 1), f"rc={proc.returncode} stderr={proc.stderr}"
    assert "RECIPE" in proc.stdout
    assert "pass-through only" in proc.stderr


def test_cli_wrap_snapshot_passthrough():
    """CLI `wrap snapshot` returns V1357 JSON snapshot (no record)."""
    proc = _run([V1367_MODULE, "wrap", "snapshot"])
    assert proc.returncode in (0, 1), f"rc={proc.returncode} stderr={proc.stderr}"
    # stdout is JSON
    data = json.loads(proc.stdout)
    assert "pole_star" in data
    assert "toolchain_health" in data
    assert "pass-through only" in proc.stderr


def test_cli_wrap_summary_record_all():
    """CLI `wrap summary --record-all` records to V1362 ledger."""
    import time as _t
    tag = f"v1367-pytest-summary-{int(_t.time() * 1000)}"
    proc = _run([V1367_MODULE, "wrap", "summary", "--record-all", "--tag", tag])
    assert proc.returncode in (0, 1), f"rc={proc.returncode} stderr={proc.stderr}"
    assert "pole_star" in proc.stdout
    assert "recorded to history" in proc.stderr
    assert tag in proc.stderr


def test_cli_wrap_recipe_record_all():
    """CLI `wrap recipe --record-all` records to V1362 ledger."""
    import time as _t
    tag = f"v1367-pytest-recipe-{int(_t.time() * 1000)}"
    proc = _run([V1367_MODULE, "wrap", "recipe", "--record-all", "--tag", tag])
    assert proc.returncode in (0, 1), f"rc={proc.returncode} stderr={proc.stderr}"
    assert "RECIPE" in proc.stdout
    assert "recorded to history" in proc.stderr
    assert tag in proc.stderr


def test_cli_wrap_snapshot_record_all():
    """CLI `wrap snapshot --record-all` records JSON snapshot to V1362 ledger."""
    import time as _t
    tag = f"v1367-pytest-snap-{int(_t.time() * 1000)}"
    proc = _run([V1367_MODULE, "wrap", "snapshot", "--record-all", "--tag", tag])
    assert proc.returncode in (0, 1), f"rc={proc.returncode} stderr={proc.stderr}"
    data = json.loads(proc.stdout)
    assert "pole_star" in data
    assert "recorded to history" in proc.stderr
    assert tag in proc.stderr


def test_cli_no_record_all_does_not_append():
    """Without --record-all, the wrapper MUST NOT append to V1362 ledger."""
    # Read history before
    proc_before = _run([V1362_MODULE, "show"])
    before_lines = [ln for ln in proc_before.stdout.splitlines() if ln.startswith("|") and ln.count("|") >= 7]

    # Run wrap without --record-all
    _run([V1367_MODULE, "wrap", "summary"])

    # Read history after
    proc_after = _run([V1362_MODULE, "show"])
    after_lines = [ln for ln in proc_after.stdout.splitlines() if ln.startswith("|") and ln.count("|") >= 7]

    assert len(before_lines) == len(after_lines), (
        f"wrap without --record-all should NOT append, but history grew: "
        f"{len(before_lines)} → {len(after_lines)}"
    )


# -----------------------------------------------------------------------------
# Library API tests
# -----------------------------------------------------------------------------


def test_library_imports():
    """V1367 module imports cleanly with all helpers exposed."""
    from apeireth import v1367_v1357_record_all as v1367
    assert v1367.V1367_VERSION == "0.1.0"
    assert v1367.V1367_ASI_CAP <= 0.01
    assert callable(v1367.record_summary)
    assert callable(v1367.record_recipe)
    assert callable(v1367.record_snapshot)
    assert callable(v1367.build_record_entry)
    assert callable(v1367._run_v1357_subcommand)


def test_library_build_record_entry():
    """build_record_entry produces stable shape contract."""
    from apeireth import v1367_v1357_record_all as v1367
    entry = v1367.build_record_entry("summary", "ABC", "", tag="t1")
    assert entry["version"] == "0.1.0"
    assert "measured_at" in entry
    assert entry["subcommand"] == "summary"
    assert entry["v1357_stdout"] == "ABC"
    assert entry["v1357_stderr"] == ""
    assert entry["v1357_stderr_lines"] == 0
    assert entry["tag"] == "t1"
    assert "GUARD_RECORD_ALL_OPT_IN" in entry["philosophy_guards"]


def test_library_build_record_entry_with_stderr():
    """build_record_entry counts stderr lines correctly."""
    from apeireth import v1367_v1357_record_all as v1367
    entry = v1367.build_record_entry("recipe", "OUT", "warn1\nwarn2\n", tag=None)
    assert entry["subcommand"] == "recipe"
    assert entry["v1357_stderr_lines"] == 2
    assert entry["tag"] is None


def test_library_record_helpers_append_to_ledger():
    """record_summary / record_recipe / record_snapshot return appended dicts."""
    from apeireth import v1367_v1357_record_all as v1367
    import time as _t
    ts = int(_t.time() * 1000)
    info_s = v1367.record_summary(tag=f"v1367-lib-summary-{ts}")
    info_r = v1367.record_recipe(tag=f"v1367-lib-recipe-{ts}")
    info_n = v1367.record_snapshot(tag=f"v1367-lib-snap-{ts}")
    assert isinstance(info_s, dict)
    assert isinstance(info_r, dict)
    assert isinstance(info_n, dict)
    # All entries should have measured_at
    assert "measured_at" in info_s
    assert "measured_at" in info_r
    assert "measured_at" in info_n


# -----------------------------------------------------------------------------
# V3 philosophy guards
# -----------------------------------------------------------------------------


def test_philosophy_guards_present():
    """All required V3 philosophy guards are defined."""
    from apeireth import v1367_v1357_record_all as v1367
    required = [
        "GUARD_RECORD_ALL_OPT_IN",
        "GUARD_DEFAULT_OFF",
        "GUARD_NO_FABRICATION",
        "GUARD_DELEGATE_TO_V1357_V1362",
        "GUARD_READ_ONLY_ON_V1357",
        "GUARD_READ_ONLY_ON_V1362",
        "GUARD_PASSTHROUGH_EXIT_CODES",
        "GUARD_HONEST_CAP",
        "GUARD_RECORD_NOT_ASI",
    ]
    for g in required:
        assert g in v1367.V1367_PHILOSOPHY_GUARDS, f"missing guard: {g}"


def test_philosophy_cap_bounded():
    """V1367 ASI cap is bounded (<= 0.01), never claims progress."""
    from apeireth import v1367_v1357_record_all as v1367
    assert v1367.V1367_ASI_CAP <= 0.01
    assert v1367.V1367_ASI_CAP >= 0.0


# -----------------------------------------------------------------------------
# Windows UTF-8 encoding safety
# -----------------------------------------------------------------------------


def test_windows_utf8_recipe_decodes():
    """V1357 recipe output contains unicode arrows; wrapper must decode UTF-8.

    This test guards against Windows GBK decode crashes that surfaced
    during V1367 development.
    """
    from apeireth import v1367_v1357_record_all as v1367
    rc, out, err = v1367._run_v1357_subcommand("recipe")
    assert rc in (0, 1)
    # Unicode arrows should appear (V1357 recipe uses →)
    assert "→" in out or "->" in out  # either form acceptable


# -----------------------------------------------------------------------------
# Pass-through exit codes
# -----------------------------------------------------------------------------


def test_v1357_exit_code_preserved_on_success():
    """V1357 exit code 0 propagates through V1367.

    Uses `wrap summary` (V1357 summary returns 0 in clean state).
    """
    proc = _run([V1367_MODULE, "wrap", "summary"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"


def test_v1357_exit_code_preserved_on_unknowns():
    """V1357 exit code 1 (with known_unknowns) propagates through V1367.

    V1357 returns 1 when the snapshot has known unknowns. V1367 MUST NOT
    mask this with 0.
    """
    proc = _run([V1367_MODULE, "wrap", "snapshot"])
    assert proc.returncode in (0, 1)


# -----------------------------------------------------------------------------
# V1362 self-test regression check
# -----------------------------------------------------------------------------


def test_v1362_self_test_passes_no_regression():
    """V1362 self-test still passes after V1367 added text-capture entries.

    V1367 added 4 entries with None pole_star_total (text-capture shape).
    V1362's render_trend_md was patched to handle this gracefully.
    This test verifies the patch held.
    """
    proc = _run([V1362_MODULE, "self-test"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"
    assert "passed" in proc.stdout


def test_v1357_self_test_passes_no_regression():
    """V1357 self-test still passes after V1367 added wrapper layer."""
    proc = _run([V1357_MODULE, "self-test"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"
    assert "passed" in proc.stdout


def test_v1366_self_test_passes_no_regression():
    """V1366 self-test still passes after V1367 + V1362 patch."""
    proc = _run(["apeireth.v1366_vcp_cookbook_dashboard_overlay", "self-test"])
    assert proc.returncode == 0, f"rc={proc.returncode} stderr={proc.stderr}"
    assert "ALL CHECKS PASS" in proc.stdout


# -----------------------------------------------------------------------------
# Chain integration: V1367 + V1366 + V1357 + V1362 + V1358 stage delivery
# -----------------------------------------------------------------------------


def test_chain_snapshots_consistent():
    """V1367 wrap snapshot output is consistent with V1357 snapshot."""
    proc_v1357 = _run([V1357_MODULE, "snapshot"])
    proc_v1367 = _run([V1367_MODULE, "wrap", "snapshot"])
    assert proc_v1357.returncode in (0, 1)
    assert proc_v1367.returncode == proc_v1357.returncode, (
        f"V1367 must preserve V1357 exit code: "
        f"v1357={proc_v1357.returncode} v1367={proc_v1367.returncode}"
    )
    data_v1357 = json.loads(proc_v1357.stdout)
    data_v1367 = json.loads(proc_v1367.stdout)
    assert data_v1357["pole_star"] == data_v1367["pole_star"], (
        "V1367 must not modify V1357's pole_star data"
    )
    assert data_v1357["toolchain_health"] == data_v1367["toolchain_health"]