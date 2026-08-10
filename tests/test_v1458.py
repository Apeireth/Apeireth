"""V1458 — Tests for ASI North Star ceiling chain consistency audit.

Author: 楚零 (Chu Ling) | cron tick 2026-08-10 11:38 Asia/Shanghai
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

# Ensure the promethean package is importable
_PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(_PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(_PROMETHEAN_ROOT))

import apeireth.v1458_asi_north_star_ceiling_chain_audit as v1458  # noqa: E402


# ============================================================================
# Module-level smoke tests
# ============================================================================

def test_module_importable():
    """V1458 module imports without error."""
    assert v1458.V1458_VERSION == "0.1.0"
    assert v1458.V1458_MODULE == "v1458_asi_north_star_ceiling_chain_audit"
    assert len(v1458.V1458_GUARDS) >= 14
    assert len(v1458.V1458_V3_GUARDS) >= 5
    assert len(v1458.V1458_BORROWED) >= 8


def test_locked_constants():
    """Locked baseline values match the V1256 anchor."""
    assert v1458.LOCKED_ANCHOR_VALUE == 0.9105
    assert v1458.LOCKED_NORTH_STAR_CEILING == 0.98
    assert v1458.LOCKED_ABSOLUTE_CEILING_V1256 == 1.0
    assert v1458.LOCKED_GAP_TO_NORTH_STAR == 0.0695
    assert v1458.LOCKED_GAP_TO_CEILING == 0.0895
    assert v1458.V1411_ABSOLUTE_CEILING == 0.99
    assert v1458.V1411_GAP_TO_CEILING == 0.0795
    assert v1458.BOUNDED_TOLERANCE == 0.0001


def test_ceiling_chain_modules_count():
    """V1458 declares exactly 5 ceiling-chain modules."""
    assert len(v1458.CEILING_CHAIN_MODULES) == 5
    module_ids = {m_id for _, m_id in v1458.CEILING_CHAIN_MODULES}
    assert "V1256" in module_ids
    assert "V1256_evidence_audit" in module_ids
    assert "V1259" in module_ids
    assert "V1410" in module_ids
    assert "V1411" in module_ids


def test_deployment_cube_modules_count():
    """V1458 declares exactly 4 deployment-cube modules."""
    assert len(v1458.DEPLOYMENT_CUBE_MODULES) == 4
    module_ids = {m_id for _, m_id in v1458.DEPLOYMENT_CUBE_MODULES}
    assert "V1450" in module_ids
    assert "V1454" in module_ids
    assert "V1455" in module_ids
    assert "V1457" in module_ids


def test_approx_equal_helper():
    """Helper correctly handles approx equality within tolerance."""
    assert v1458._approx_equal(0.9105, 0.9105) is True
    # Within tolerance ±0.0001
    assert v1458._approx_equal(0.9106, 0.9105, tol=0.0001) is True
    assert v1458._approx_equal(0.9104, 0.9105, tol=0.0001) is True
    # Outside tolerance
    assert v1458._approx_equal(0.9100, 0.9105, tol=0.0001) is False
    assert v1458._approx_equal(0.92, 0.9105, tol=0.0001) is False
    # Edge cases
    assert v1458._approx_equal(None, 0.9105) is False
    # Type-safety
    assert v1458._approx_equal("foo", 0.9105) is False


# ============================================================================
# Per-module audit tests
# ============================================================================

def test_audit_v1256_ceiling_module():
    """V1256 reports anchor=0.9105 + north=0.98 + abs=1.0 + consistency=1.0."""
    r = v1458.audit_ceiling_chain_module(
        "v1256_asi_v0666_unio_mystica_substrate_real_lift", "V1256"
    )
    assert r.module_id == "V1256"
    assert r.importable is True
    assert r.anchor_value == 0.9105
    assert r.north_star_ceiling == 0.98
    assert r.absolute_ceiling == 1.0
    assert r.computed_gap_to_north_star == 0.0695
    assert r.computed_gap_to_ceiling == 0.0895
    assert r.check_anchor_locked is True
    assert r.check_north_star_locked is True
    assert r.check_absolute_ceiling_consistent is True
    assert r.check_gap_to_north_star is True
    assert r.check_gap_to_ceiling is True
    assert r.internal_consistency_score == 1.0


def test_audit_v1256_evidence_audit_ceiling_module():
    """V1256_evidence_audit reports anchor=0.9105 + abs=1.0."""
    r = v1458.audit_ceiling_chain_module("v1256_evidence_audit", "V1256_evidence_audit")
    assert r.module_id == "V1256_evidence_audit"
    assert r.importable is True
    assert r.anchor_value == 0.9105
    assert r.north_star_ceiling == 0.98
    assert r.absolute_ceiling == 1.0
    assert r.computed_gap_to_ceiling == 0.0895
    assert r.internal_consistency_score == 1.0


def test_audit_v1259_ceiling_module():
    """V1259 reports anchor=0.9105 + north=0.98 + abs=1.0."""
    r = v1458.audit_ceiling_chain_module("v1259_north_star_trajectory", "V1259")
    assert r.module_id == "V1259"
    assert r.importable is True
    assert r.anchor_value == 0.9105
    assert r.north_star_ceiling == 0.98
    assert r.absolute_ceiling == 1.0
    assert r.computed_gap_to_ceiling == 0.0895
    assert r.internal_consistency_score == 1.0


def test_audit_v1410_ceiling_module():
    """V1410 reports anchor=0.9105 + north=0.98 + abs=0.99 (V1410 convention)."""
    r = v1458.audit_ceiling_chain_module(
        "v1410_asi_five_position_framework", "V1410"
    )
    assert r.module_id == "V1410"
    assert r.importable is True
    assert r.anchor_value == 0.9105
    assert r.north_star_ceiling == 0.98
    assert r.absolute_ceiling == 0.99
    assert r.computed_gap_to_north_star == 0.0695
    assert r.computed_gap_to_ceiling == 0.0795
    # V1410 internal math is correct (V1410 convention)
    assert r.check_anchor_locked is True
    assert r.check_north_star_locked is True
    assert r.check_absolute_ceiling_consistent is True  # accepts 0.99
    assert r.check_gap_to_north_star is True
    assert r.check_gap_to_ceiling is True
    assert r.internal_consistency_score == 1.0


def test_audit_v1411_ceiling_module():
    """V1411 reports anchor=0.9105 + north=0.98 + abs=0.99 (V1410 convention)."""
    r = v1458.audit_ceiling_chain_module(
        "v1411_asi_overarching_framework", "V1411"
    )
    assert r.module_id == "V1411"
    assert r.importable is True
    assert r.anchor_value == 0.9105
    assert r.north_star_ceiling == 0.98
    assert r.absolute_ceiling == 0.99
    assert r.computed_gap_to_ceiling == 0.0795
    assert r.internal_consistency_score == 1.0


# ============================================================================
# Deployment-cube tests (no ceiling touch)
# ============================================================================

def test_audit_v1450_does_not_touch_ceiling():
    """V1450 cube history aggregator does NOT touch ceiling chain."""
    r = v1458.audit_deployment_cube_module(
        "v1450_asi_cross_modular_cube_history", "V1450"
    )
    assert r.module_id == "V1450"
    assert r.importable is True
    assert r.no_ceiling_touch is True
    assert r.check_no_ceiling_touch is True
    assert r.check_honest_disclosure is True


def test_audit_v1454_does_not_touch_ceiling():
    """V1454 hypercube 4-axis does NOT touch ceiling chain."""
    r = v1458.audit_deployment_cube_module(
        "v1454_asi_hypercube_four_axis_deployment", "V1454"
    )
    assert r.module_id == "V1454"
    assert r.no_ceiling_touch is True
    assert r.check_no_ceiling_touch is True


def test_audit_v1455_does_not_touch_ceiling():
    """V1455 hypercube full-source-content does NOT touch ceiling chain."""
    r = v1458.audit_deployment_cube_module(
        "v1455_asi_hypercube_full_source_content_audit_v5", "V1455"
    )
    assert r.module_id == "V1455"
    assert r.no_ceiling_touch is True
    assert r.check_no_ceiling_touch is True


def test_audit_v1457_does_not_touch_ceiling():
    """V1457 6-deployment operational runbook does NOT touch ceiling chain."""
    r = v1458.audit_deployment_cube_module(
        "v1457_asi_six_deployment_operational_runbook", "V1457"
    )
    assert r.module_id == "V1457"
    assert r.no_ceiling_touch is True
    assert r.check_no_ceiling_touch is True


# ============================================================================
# Aggregate audit tests
# ============================================================================

def test_run_full_audit_returns_report():
    """run_ceiling_chain_audit returns a valid report."""
    r = v1458.run_ceiling_chain_audit()
    assert r.module == v1458.V1458_MODULE
    assert r.version == v1458.V1458_VERSION
    assert r.n_ceiling_modules == 5
    assert r.n_deployment_modules == 4
    assert r.n_ceiling_modules_passed == 5  # all 5 internally consistent
    assert r.n_deployment_modules_passed == 4  # all 4 don't touch ceiling


def test_audit_aggregate_internal_consistency():
    """All 5 ceiling modules have internal consistency = 1.0 (math correct)."""
    r = v1458.run_ceiling_chain_audit()
    assert r.aggregate_internal_consistency == 1.0


def test_audit_aggregate_cross_consistency():
    """All 5 modules have anchor=0.9105 and north=0.98 locked."""
    r = v1458.run_ceiling_chain_audit()
    assert r.aggregate_cross_consistency == 1.0


def test_audit_aggregate_gap_preservation():
    """All 5 modules have correct internal gap math (gap = abs - anchor)."""
    r = v1458.run_ceiling_chain_audit()
    assert r.aggregate_gap_preservation == 1.0


def test_audit_ceiling_convention_uniformity_split():
    """3/5 modules use V1256 convention (1.0); 2/5 use V1410 (0.99)."""
    r = v1458.run_ceiling_chain_audit()
    assert r.n_using_v1256_convention == 3
    assert r.n_using_v1410_convention == 2
    assert abs(r.aggregate_ceiling_convention_uniformity - 0.6) < 0.001


def test_audit_no_inflation():
    """No module inflates anchor past 0.9105."""
    r = v1458.run_ceiling_chain_audit()
    assert r.any_inflation is False


def test_audit_no_lowered_north_star():
    """No module lowers north_star below 0.98."""
    r = v1458.run_ceiling_chain_audit()
    assert r.any_lowered_north_star is False


def test_audit_no_lowered_ceiling():
    """No module lowers absolute_ceiling below 0.99 (V1410 floor)."""
    r = v1458.run_ceiling_chain_audit()
    assert r.any_lowered_ceiling is False


def test_audit_inconsistencies_are_only_convention_split():
    """The only inconsistencies are ceiling_convention_split (V1410 + V1411)."""
    r = v1458.run_ceiling_chain_audit()
    types = {inc["type"] for inc in r.inconsistencies}
    assert types == {"ceiling_convention_split"}, (
        f"unexpected inconsistency types: {types}"
    )
    modules = {inc["module"] for inc in r.inconsistencies}
    assert modules == {"V1410", "V1411"}


# ============================================================================
# V3 philosophy guard tests
# ============================================================================

def test_v3_guards_declared():
    """V1458 declares 5 V3 philosophy guards (不假装 Phenomenal/ASI/human/absolute)."""
    expected = {
        "GUARD_CEILING_AUDIT_NOT_PHENOMENAL",
        "GUARD_CEILING_AUDIT_NOT_ASI",
        "GUARD_CEILING_AUDIT_NOT_HUMAN_LEVEL",
        "GUARD_CEILING_AUDIT_NOT_ABSOLUTE",
        "GUARD_CEILING_AUDIT_NOT_LOCK_CHANGE",
    }
    assert expected.issubset(set(v1458.V1458_V3_GUARDS))


def test_v1458_specific_guards_declared():
    """V1458 declares at least 14 module-specific guards."""
    assert len(v1458.V1458_GUARDS) >= 14
    expected = {
        "GUARD_CEILING_CHAIN_DECLARED",
        "GUARD_DEPLOYMENT_CUBE_DECLARED",
        "GUARD_INTERNAL_CONSISTENCY",
        "GUARD_CROSS_CONSISTENCY",
        "GUARD_GAP_PRESERVATION",
        "GUARD_NO_INFLATION",
        "GUARD_NO_LOWERED_NORTH",
        "GUARD_NO_LOWERED_CEILING",
        "GUARD_BOUNDED_TOLERANCE",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_DEPLOYMENT_NO_CEILING_TOUCH",
        "GUARD_CLI_RUNNABLE",
        "GUARD_POPPER_RUNS",
        "GUARD_BORROWED_LINEAGE",
    }
    assert expected.issubset(set(v1458.V1458_GUARDS))


# ============================================================================
# Popper self-test
# ============================================================================

def test_popper_self_test_passes():
    """Popper self-test returns 7/7 pass."""
    result = v1458.popper_self_test()
    assert result["all_pass"] is True
    assert result["pass_count"] == 7
    assert result["total_count"] == 7


# ============================================================================
# CLI tests
# ============================================================================

def test_cli_version():
    """CLI `version` command prints version."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["version"])
    assert rc == 0
    assert "0.1.0" in buf.getvalue()


def test_cli_meta_returns_dict():
    """CLI `meta` command reports module structure."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["meta", "--json"])
    assert rc == 0
    meta = json.loads(buf.getvalue())
    assert meta["module"] == v1458.V1458_MODULE
    assert meta["version"] == v1458.V1458_VERSION
    # ceiling_chain_modules holds module IDs (V1256, etc.) not module names
    assert "V1256" in meta["ceiling_chain_modules"]
    assert "V1411" in meta["ceiling_chain_modules"]


def test_cli_audit_text():
    """CLI `audit` command runs full audit and reports aggregates."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["audit"])
    assert rc == 0
    out = buf.getvalue()
    assert "V1458" in out
    assert "internal_consistency" in out
    assert "ceiling_convention_uniformity" in out


def test_cli_audit_json():
    """CLI `audit --json` returns valid JSON with expected keys."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["audit", "--json"])
    assert rc == 0
    data = json.loads(buf.getvalue())
    assert data["module"] == v1458.V1458_MODULE
    assert data["n_ceiling_modules"] == 5
    assert data["n_deployment_modules"] == 4
    assert data["aggregate_internal_consistency"] == 1.0
    assert data["aggregate_cross_consistency"] == 1.0
    assert data["aggregate_gap_preservation"] == 1.0
    assert data["n_using_v1256_convention"] == 3
    assert data["n_using_v1410_convention"] == 2
    assert data["any_inflation"] is False
    assert data["any_lowered_north_star"] is False


def test_cli_inconsistencies_command():
    """CLI `inconsistencies` command reports ceiling_convention_split."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["inconsistencies", "--json"])
    assert rc == 0
    incs = json.loads(buf.getvalue())
    types = {i["type"] for i in incs}
    assert "ceiling_convention_split" in types


def test_cli_ceiling_command():
    """CLI `ceiling` command reports 5 ceiling modules."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["ceiling", "--json"])
    assert rc == 0
    ceiling = json.loads(buf.getvalue())
    assert len(ceiling) == 5
    module_ids = {r["module_id"] for r in ceiling}
    assert module_ids == {"V1256", "V1256_evidence_audit", "V1259", "V1410", "V1411"}


def test_cli_deploy_command():
    """CLI `deploy` command reports 4 deployment-cube modules."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["deploy", "--json"])
    assert rc == 0
    deploy = json.loads(buf.getvalue())
    assert len(deploy) == 4
    for r in deploy:
        assert r["check_no_ceiling_touch"] is True
        assert r["check_honest_disclosure"] is True


# ============================================================================
# Subprocess CLI test (verifies __main__ entry point)
# ============================================================================

def test_subprocess_audit_json():
    """Subprocess `python -m apeireth.v1458... audit --json` works.

    On Windows, we set PYTHONIOENCODING=utf-8 to avoid the gbk codec
    UnicodeDecodeError that occurs when reading non-ASCII output.
    """
    env = {
        **__import__("os").environ,
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
    }
    result = subprocess.run(
        [sys.executable, "-m",
         "apeireth.v1458_asi_north_star_ceiling_chain_audit",
         "audit", "--json"],
        cwd=str(_PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=60,
    )
    assert result.returncode == 0, (
        f"stderr: {result.stderr}\nstdout: {(result.stdout or '')[:500]}"
    )
    assert result.stdout is not None
    data = json.loads(result.stdout)
    assert data["n_ceiling_modules"] == 5
    assert data["n_deployment_modules"] == 4
    assert data["aggregate_internal_consistency"] == 1.0
    assert data["any_inflation"] is False


def test_subprocess_meta():
    """Subprocess `python -m ... meta` works (Windows-encoding-safe)."""
    import os
    env = {
        **os.environ,
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
    }
    result = subprocess.run(
        [sys.executable, "-m",
         "apeireth.v1458_asi_north_star_ceiling_chain_audit", "meta"],
        cwd=str(_PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=30,
    )
    assert result.returncode == 0
    out = result.stdout or ""
    # The meta command prints module IDs (V1256, V1259, ...) not module names
    assert "V1259" in out
    assert "V1457" in out


def test_subprocess_help():
    """Subprocess `python -m ... --help` works (Windows-encoding-safe)."""
    import os
    env = {
        **os.environ,
        "PYTHONIOENCODING": "utf-8",
        "PYTHONUTF8": "1",
    }
    result = subprocess.run(
        [sys.executable, "-m",
         "apeireth.v1458_asi_north_star_ceiling_chain_audit", "--help"],
        cwd=str(_PROMETHEAN_ROOT),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
        timeout=10,
    )
    assert result.returncode == 0
    out = result.stdout or ""
    assert "V1458" in out


# ============================================================================
# Honest-disclosure test (V1458 reports V1410/V1411 convention split)
# ============================================================================

def test_honest_disclosure_in_output():
    """Audit output explicitly mentions V1410/V1411 convention split."""
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = v1458.run_cli(["audit"])
    assert rc == 0
    out = buf.getvalue()
    assert "V1410/V1411" in out
    assert "0.99" in out
    assert "ceiling chain" in out.lower() or "ceiling-chain" in out.lower()


def test_v1458_is_audit_only_not_lock_changer():
    """V1458 does not modify ceiling chain — it only audits it."""
    r = v1458.run_ceiling_chain_audit()
    # Run twice and verify the report content is stable (no state mutation)
    r2 = v1458.run_ceiling_chain_audit()
    assert r.aggregate_internal_consistency == r2.aggregate_internal_consistency
    assert r.aggregate_cross_consistency == r2.aggregate_cross_consistency
    assert r.any_inflation == r2.any_inflation
    # Note: generated_at will differ slightly, but aggregates stable
