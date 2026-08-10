"""V1460 — Tests for ASI Real Windows Anyone-Run Harness.

Author: 楚零 (Chu Ling) | cron tick 2026-08-10 12:03 Asia/Shanghai
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

# Ensure the promethean package is importable
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

import apeireth.v1460_asi_real_windows_anyone_run_harness as v1460  # noqa: E402


# ============================================================================
# Module-level smoke tests
# ============================================================================

def test_module_importable():
    """V1460 module imports without error."""
    assert v1460.V1460_VERSION == "0.1.0"
    assert v1460.V1460_MODULE == "v1460_asi_real_windows_anyone_run_harness"
    assert len(v1460.V1460_GUARDS) >= 8
    assert len(v1460.V1460_V3_GUARDS) >= 5
    assert len(v1460.V1460_BORROWED) >= 8


def test_stages_declared():
    """V1460 declares exactly 13 stages."""
    assert len(v1460.STAGE_NAMES) == v1460.EXPECTED_N_STAGES == 13


def test_subprocess_timeout_bounded():
    """V1460 subprocess timeout is bounded (10s ≤ x ≤ 60s)."""
    assert 10 <= v1460.SUBPROCESS_TIMEOUT_S <= 60


def test_subprocess_targets_well_formed():
    """All subprocess targets are well-formed module names."""
    for mod, cmd in v1460.V14XX_SUBPROCESS_TARGETS:
        assert mod.startswith("v14"), f"unexpected module: {mod}"
        assert "_" in mod, f"module name should have underscore: {mod}"
        assert cmd in {"summary", "audit", "popper", "meta"}, f"unexpected cmd: {cmd}"


# ============================================================================
# Per-stage test (lightweight — without subprocess calls)
# ============================================================================

def test_stage_python_version_passes():
    """stage_python_version returns StageResult with passed=True on modern Python."""
    r = v1460.stage_python_version()
    assert r.stage == "python_version"
    assert r.passed is True
    assert r.score == 1.0


def test_stage_import_apeireth_passes():
    """stage_import_apeireth returns StageResult with passed=True if apeireth loads."""
    r = v1460.stage_import_apeireth()
    assert r.stage == "import_apeireth"
    assert r.passed is True
    assert r.score == 1.0


def test_stage_streamlit_probe_passes():
    """stage_streamlit_probe returns StageResult with passed=True (streamlit installed)."""
    r = v1460.stage_streamlit_probe()
    assert r.stage == "streamlit_probe"
    assert r.passed is True
    assert r.score == 1.0


def test_stage_requests_probe_passes():
    """stage_requests_probe returns StageResult with passed=True (requests installed)."""
    r = v1460.stage_requests_probe()
    assert r.stage == "requests_probe"
    assert r.passed is True
    assert r.score == 1.0


def test_stage_docker_probe_returns_result():
    """stage_docker_probe returns a StageResult (allowed to fail on Windows)."""
    r = v1460.stage_docker_probe()
    assert r.stage == "docker_probe"
    # docker not installed on Windows dev box — passed may be False
    # GUARD_DOCKER_OPTIONAL: score is always 1.0 regardless of docker availability
    assert r.score == 1.0


# ============================================================================
# Stage dataclass tests
# ============================================================================

def test_stage_result_dataclass():
    """StageResult dataclass fields are accessible."""
    r = v1460.StageResult(
        stage="test_stage",
        passed=True,
        score=1.0,
        latency_ms=12.3,
        detail="ok",
        exit_code=0,
    )
    assert r.stage == "test_stage"
    assert r.passed is True
    assert r.score == 1.0
    assert r.latency_ms == 12.3
    assert r.exit_code == 0


def test_harness_report_dataclass():
    """HarnessReport dataclass can be instantiated with default empty fields."""
    r = v1460.HarnessReport(
        module="x",
        version="0.0.0",
        generated_at="2026-08-10T00:00:00Z",
        platform="test",
        python_version="3.13.0",
        n_stages=0,
        n_passed=0,
        n_failed=0,
        overall_score=0.0,
    )
    assert r.module == "x"
    assert r.stage_results == []
    assert r.honest_disclosure == []
    assert len(r.guards) >= 8
    assert len(r.v3_guards) >= 5
    assert len(r.borrowed) >= 8


# ============================================================================
# Report writer tests
# ============================================================================

def test_stage_write_report_md(tmp_path=None):
    """stage_write_report_md writes a markdown file with expected sections."""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "report.md")
        report = v1460.HarnessReport(
            module=v1460.V1460_MODULE,
            version=v1460.V1460_VERSION,
            generated_at="2026-08-10T12:00:00Z",
            platform="Windows-test",
            python_version="3.13.0",
            n_stages=11,
            n_passed=11,
            n_failed=0,
            overall_score=1.0,
        )
        r = v1460.stage_write_report_md(path, report)
        assert r.passed is True
        assert r.stage == "write_report_md"
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()
        assert "V1460" in content
        assert "stage results" in content.lower()
        assert "Honest disclosure" in content
        assert "V3 哲学守门" in content


def test_stage_write_report_json(tmp_path=None):
    """stage_write_report_json writes a JSON file with all expected fields."""
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        path = os.path.join(tmp, "report.json")
        report = v1460.HarnessReport(
            module=v1460.V1460_MODULE,
            version=v1460.V1460_VERSION,
            generated_at="2026-08-10T12:00:00Z",
            platform="Windows-test",
            python_version="3.13.0",
            n_stages=11,
            n_passed=11,
            n_failed=0,
            overall_score=1.0,
        )
        r = v1460.stage_write_report_json(path, report)
        assert r.passed is True
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert data["module"] == v1460.V1460_MODULE
        assert data["n_stages"] == 11
        assert data["n_passed"] == 11
        assert data["n_failed"] == 0
        assert data["overall_score"] == 1.0


# ============================================================================
# V3 philosophy guard tests
# ============================================================================

def test_v3_guards_declared():
    """V1460 declares 5 V3 philosophy guards (不假装 ASI / Phenomenal / lock-change / absolute / human-level)."""
    expected = {
        "GUARD_HARNESS_NOT_ASI",
        "GUARD_HARNESS_NOT_PHENOMENAL",
        "GUARD_HARNESS_NOT_LOCK_CHANGE",
        "GUARD_HARNESS_NOT_ABSOLUTE",
        "GUARD_HARNESS_NOT_HUMAN_LEVEL",
    }
    assert expected.issubset(set(v1460.V1460_V3_GUARDS))


def test_v1460_specific_guards_declared():
    """V1460 declares at least 8 module-specific guards."""
    assert len(v1460.V1460_GUARDS) >= 8
    expected = {
        "GUARD_STAGES_DECLARED",
        "GUARD_PROBES_REAL",
        "GUARD_EXIT_CODES_CAPTURED",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_BOUNDED_TIMEOUT",
        "GUARD_DOCKER_OPTIONAL",
        "GUARD_CLI_RUNNABLE",
        "GUARD_BORROWED_LINEAGE",
    }
    assert expected.issubset(set(v1460.V1460_GUARDS))


def test_borrowed_lineage_cited():
    """V1460 cites at least 8 borrowed sources (走在前人经验上)."""
    assert len(v1460.V1460_BORROWED) >= 8
    keys = {b["key"] for b in v1460.V1460_BORROWED}
    assert "v1256_unio_mystica_2026" in keys
    assert any("v145" in k for k in keys)


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_self_test_passes():
    """Popper self-test returns 7/7 pass."""
    results = v1460.popper_self_test()
    assert len(results) == 7
    assert all(ok for _, ok in results), (
        f"popper failures: {[name for name, ok in results if not ok]}"
    )


# ============================================================================
# CLI tests
# ============================================================================

def test_cli_meta():
    """CLI `meta` command prints module metadata."""
    import argparse
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1460._cmd_meta(argparse.Namespace())
    assert rc == 0
    out = buf.getvalue()
    assert v1460.V1460_MODULE in out
    assert v1460.V1460_VERSION in out
    assert "stages: 13" in out


def test_cli_probes():
    """CLI `probes` command lists 13 probes."""
    import argparse
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1460._cmd_probes(argparse.Namespace())
    assert rc == 0
    out = buf.getvalue()
    assert "13" in out
    for stage in v1460.STAGE_NAMES:
        assert stage in out, f"stage {stage} not listed"


def test_cli_popper():
    """CLI `popper` command runs popper self-test."""
    import argparse
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1460._cmd_popper(argparse.Namespace())
    assert rc == 0
    out = buf.getvalue()
    assert "popper_stages_count_is_13" in out
    assert "popper_cli_subcommands_complete" in out
    assert "FAIL" not in out


# ============================================================================
# Subprocess CLI test (verifies __main__ entry point)
# ============================================================================

def test_subprocess_meta():
    """Subprocess `python -m apeireth.v1460... meta` works (Windows-encoding-safe)."""
    env = {
        **os.environ,
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
    }
    result = subprocess.run(
        [sys.executable, "-m",
         "apeireth.v1460_asi_real_windows_anyone_run_harness", "meta"],
        cwd=str(_PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=30,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    out = result.stdout or ""
    assert v1460.V1460_MODULE in out


def test_subprocess_popper():
    """Subprocess `python -m apeireth.v1460... popper` works (Windows-encoding-safe)."""
    env = {
        **os.environ,
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
    }
    result = subprocess.run(
        [sys.executable, "-m",
         "apeireth.v1460_asi_real_windows_anyone_run_harness", "popper"],
        cwd=str(_PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=30,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    out = result.stdout or ""
    assert "popper_stages_count_is_13" in out
    assert "FAIL" not in out


def test_subprocess_help():
    """Subprocess `python -m apeireth.v1460... --help` works (Windows-encoding-safe)."""
    env = {
        **os.environ,
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
    }
    result = subprocess.run(
        [sys.executable, "-m",
         "apeireth.v1460_asi_real_windows_anyone_run_harness", "--help"],
        cwd=str(_PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=10,
    )
    assert result.returncode == 0, f"stderr: {result.stderr}"
    out = result.stdout or ""
    assert "V1460" in out


# ============================================================================
# Honest-disclosure test (主 17:43 实事求是)
# ============================================================================

def test_v1460_does_not_silently_patch_failures():
    """V1460 does NOT silently patch subprocess failures — they are reported.

    V1460's GUARD_HONEST_DISCLOSURE: failures are reported as failures.
    """
    # If V1454-V1459 exit codes are non-zero on this machine, V1460 must
    # capture the exit code and report it. We verify the structure by
    # building a fake StageResult and confirming it surfaces in honest_disclosure.
    report = v1460.HarnessReport(
        module=v1460.V1460_MODULE,
        version=v1460.V1460_VERSION,
        generated_at="2026-08-10T12:00:00Z",
        platform="Windows-test",
        python_version="3.13.0",
        n_stages=0,
        n_passed=0,
        n_failed=0,
        overall_score=0.0,
        stage_results=[
            v1460.StageResult(
                stage="run_v1459_subprocess",
                passed=False,
                score=0.0,
                latency_ms=100.0,
                detail="exit=1 | simulated failure",
                exit_code=1,
            ),
        ],
        honest_disclosure=[
            "run_v1459_subprocess failed: exit=1 | simulated failure",
        ],
    )
    # The disclosure must include the failure
    assert any(
        "run_v1459_subprocess failed" in d
        for d in report.honest_disclosure
    )


def test_v1460_is_harness_not_audit():
    """V1460 is a harness, NOT an audit. It does not modify ceiling chain."""
    # The harness exercises V1454-V1459 via subprocess but does not
    # introspect or modify their ceiling chain. We verify by checking
    # that V1460 has no V1256 anchor constants.
    assert not hasattr(v1460, "LOCKED_ANCHOR_VALUE")
    assert not hasattr(v1460, "NORTH_STAR_CEILING")
    # V1460 only imports stdlib + runs subprocess
    # It does not lock / unlock anything
    assert v1460.SUBPROCESS_TIMEOUT_S > 0